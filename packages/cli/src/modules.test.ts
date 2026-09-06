import { afterAll, describe, expect, test } from "bun:test";
import { cpSync, existsSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { join, resolve } from "node:path";

import { NATIVE_MODULE_ABI } from "@quickgui/native/modules";

import { buildProject } from "./build.ts";
import { resolveConfig } from "./config.ts";
import { shouldIgnoreChange } from "./dev.ts";
import {
  buildNativeModules,
  discoverNativeModules,
  extractZigFunctionSignatures,
  findZig,
  generateNativeModuleSource,
  hoistedName,
  manifestProbeSource,
  moduleSymbolPrefix,
  nativeModuleFfiFunctions,
  parseZigVersion,
  probeBuildArguments,
  zigBuildArguments,
  zigSourceFiles,
  zigTarget,
  zigVersionSupported,
  type NativeModuleManifest,
} from "./modules.ts";
import { hostTarget } from "./targets.ts";

const fixtureRoot = resolve(import.meta.dir, "..", "test-fixtures", "native-modules");
const temporaryRoots: string[] = [];

afterAll(() => {
  for (const root of temporaryRoots.splice(0)) rmSync(root, { recursive: true, force: true });
});

/** A copy of the fixture project inside the workspace, so `@quickgui/native` resolves from it. */
function fixtureCopy(): string {
  const root = mkdtempSync(join(resolve(fixtureRoot, ".."), ".tmp-native-modules-"));
  temporaryRoots.push(root);
  cpSync(fixtureRoot, root, { recursive: true });
  return root;
}

const sampleManifest: NativeModuleManifest = {
  abi: NATIVE_MODULE_ABI,
  zig: "0.15.2",
  functions: [
    {
      name: "parseHunk",
      params: [{ injected: "allocator" }, { wire: "string", type: { kind: "string" } }],
      result: {
        wire: "json",
        type: {
          kind: "struct",
          name: "main.Hunk",
          fields: [
            { name: "heading", type: { kind: "string" }, hasDefault: false },
            {
              name: "lines",
              type: {
                kind: "array",
                element: {
                  kind: "struct",
                  name: "main.Line",
                  fields: [
                    {
                      name: "kind",
                      type: { kind: "enum", name: "main.Kind", values: ["context", "added"] },
                      hasDefault: false,
                    },
                    { name: "number", type: { kind: "optional", inner: { kind: "number" } }, hasDefault: true },
                  ],
                },
              },
              hasDefault: false,
            },
            {
              name: "stats",
              type: {
                kind: "struct",
                name: "main.Hunk__struct_2241",
                fields: [{ name: "added", type: { kind: "number" }, hasDefault: false }],
              },
              hasDefault: false,
            },
          ],
        },
      },
    },
    {
      name: "depth",
      params: [
        {
          wire: "json",
          type: {
            kind: "struct",
            name: "main.Node",
            fields: [
              { name: "children", type: { kind: "array", element: { kind: "ref", name: "main.Node" } }, hasDefault: true },
            ],
          },
        },
      ],
      result: { wire: "number", type: { kind: "number" } },
    },
    {
      name: "area",
      params: [
        {
          wire: "json",
          type: {
            kind: "union",
            name: "main.Shape",
            variants: [
              { name: "circle", type: { kind: "number" } },
              { name: "none", type: { kind: "void" } },
            ],
          },
        },
      ],
      result: { wire: "number", type: { kind: "number" } },
    },
    {
      name: "reverse",
      params: [{ wire: "bytes", type: { kind: "bytes" } }],
      result: { wire: "bytes", type: { kind: "bytes" } },
    },
    {
      name: "nothing",
      params: [],
      result: { wire: "void", type: { kind: "void" } },
    },
  ],
};

const sampleZig = `const std = @import("std");

/// Parses one hunk.
///
/// Second paragraph.
pub fn parseHunk(allocator: std.mem.Allocator, text: []const u8) !Hunk {
    _ = allocator;
    _ = text;
}

pub fn depth(
    node: Node,
) u32 {
    return 1;
}
pub inline fn area(shape: Shape) f64 { return 0; }
pub fn reverse(comptime bytes: quickgui.Bytes) quickgui.Bytes { return bytes; }
pub fn nothing() void {}
pub fn pair(allocator: std.mem.Allocator, first: u8, second: []const u8) !struct { u8, []const u8 } {
    return .{ first, second };
}
`;

describe("Zig source scanning", () => {
  test("recovers parameter names, doc comments, and one-line signatures", () => {
    const signatures = extractZigFunctionSignatures(sampleZig);
    expect(signatures.get("parseHunk")).toEqual({
      name: "parseHunk",
      params: ["allocator", "text"],
      doc: ["Parses one hunk.", "", "Second paragraph."],
      text: "pub fn parseHunk(allocator: std.mem.Allocator, text: []const u8) !Hunk",
    });
    expect(signatures.get("depth")).toMatchObject({
      params: ["node"],
      doc: [],
      text: "pub fn depth( node: Node, ) u32",
    });
    expect(signatures.get("area")).toMatchObject({ params: ["shape"] });
    expect(signatures.get("reverse")).toMatchObject({ params: ["bytes"] });
    expect(signatures.get("nothing")).toMatchObject({ params: [] });
    expect(signatures.get("pair")).toMatchObject({
      params: ["allocator", "first", "second"],
      text: "pub fn pair(allocator: std.mem.Allocator, first: u8, second: []const u8) !struct { u8, []const u8 }",
    });
  });
});

describe("TypeScript generation", () => {
  const source = generateNativeModuleSource({
    name: "echo",
    manifest: sampleManifest,
    entryDisplayPath: "modules/echo/main.zig",
    zigSource: sampleZig,
  });

  test("declares the module's entry points and imports only the decoders it uses", () => {
    expect(source).toContain(
      [
        "import {",
        "  NativeArguments,",
        "  allocateNativeModuleRequest,",
        "  awaitNativeModuleResult,",
        "  decodeNativeBytes,",
        "  decodeNativeJson,",
        "  decodeNativeNumber,",
        "  decodeNativeVoid,",
        '} from "@quickgui/native/modules";',
      ].join("\n"),
    );
    expect(source).toContain(
      "declare function quickgui_module_echo_call(index: number, args: Uint8Array, reply: (result: Uint8Array) => void): void;",
    );
    expect(source).toContain(
      "declare function quickgui_module_echo_call_async(request: number, index: number, args: Uint8Array): void;",
    );
    expect(source).toContain("  quickgui_module_echo_call(index, args, (bytes: Uint8Array): void => {");
    expect(source).toContain("  quickgui_module_echo_call_async(request, index, args);");
    expect(source).not.toContain("decodeNativeString");
  });

  test("hoists named Zig types, inlines anonymous ones, and resolves recursive references", () => {
    expect(source).toContain(
      "export interface Hunk {\n  heading: string;\n  lines: Line[];\n  stats: { added: number; };\n}",
    );
    expect(source).toContain("export interface Line {\n  kind: Kind;\n  number?: number | null;\n}");
    expect(source).toContain('export type Kind = "context" | "added";');
    expect(source).toContain("export interface Node {\n  children?: Node[];\n}");
    expect(source).toContain("export type Shape = { circle?: number; none?: Record<string, never>; };");
    expect(source).not.toContain("__struct_");
  });

  test("emits typed synchronous and asynchronous wrappers with source names and docs", () => {
    expect(source).toContain(
      [
        "/**",
        " * Parses one hunk.",
        " *",
        " * Second paragraph.",
        " *",
        " * Zig: `pub fn parseHunk(allocator: std.mem.Allocator, text: []const u8) !Hunk`",
        " */",
        "export function parseHunk(text: string): Hunk {",
        "  const args = new NativeArguments(1);",
        "  args.string(text);",
        '  return JSON.parse(decodeNativeJson("echo", "parseHunk", call(0, args.finish()))) as Hunk;',
        "}",
      ].join("\n"),
    );
    expect(source).toContain(
      [
        "export async function parseHunkAsync(text: string): Promise<Hunk> {",
        "  const args = new NativeArguments(1);",
        "  args.string(text);",
        '  return JSON.parse(decodeNativeJson("echo", "parseHunk", await callAsync(0, args.finish()))) as Hunk;',
        "}",
      ].join("\n"),
    );
    expect(source).toContain(
      "export function depth(node: Node): number {\n  const args = new NativeArguments(1);\n  args.json(JSON.stringify(node));\n  return decodeNativeNumber(\"echo\", \"depth\", call(1, args.finish()));\n}",
    );
    expect(source).toContain("export function area(shape: Shape): number {");
    // `bytes` is an identifier the generated file uses itself, so the parameter is renamed.
    expect(source).toContain(
      "export function reverse(bytes_: Uint8Array): Uint8Array {\n  const args = new NativeArguments(1);\n  args.bytes(bytes_);\n  return decodeNativeBytes(\"echo\", \"reverse\", call(3, args.finish()));\n}",
    );
    expect(source).toContain(
      "export function nothing(): void {\n  const args = new NativeArguments(0);\n  decodeNativeVoid(\"echo\", \"nothing\", call(4, args.finish()));\n}",
    );
    expect(source).toContain(
      "export async function nothingAsync(): Promise<void> {\n  const args = new NativeArguments(0);\n  decodeNativeVoid(\"echo\", \"nothing\", await callAsync(4, args.finish()));\n}",
    );
  });

  test("falls back to positional names and rejects unusable export names", () => {
    const positional = generateNativeModuleSource({
      name: "echo",
      manifest: { ...sampleManifest, functions: [sampleManifest.functions[0]!] },
      entryDisplayPath: "modules/echo/main.zig",
    });
    expect(positional).toContain("export function parseHunk(arg1: string): Hunk {");
    const reserved = { ...sampleManifest.functions[4]!, name: "delete" };
    expect(() =>
      generateNativeModuleSource({
        name: "echo",
        manifest: { ...sampleManifest, functions: [reserved] },
        entryDisplayPath: "modules/echo/main.zig",
      }),
    ).toThrow("exports `delete`, which is not a usable JavaScript export name");
    const clash = { ...sampleManifest.functions[4]!, name: "nothingAsync" };
    expect(() =>
      generateNativeModuleSource({
        name: "echo",
        manifest: { ...sampleManifest, functions: [sampleManifest.functions[4]!, clash] },
        entryDisplayPath: "modules/echo/main.zig",
      }),
    ).toThrow("exports both `nothing` and `nothingAsync`");
  });

  test("derives TypeScript names only for named Zig types", () => {
    expect(hoistedName("main.DiffFile")).toBe("DiffFile");
    expect(hoistedName("main.git.Line")).toBe("Line");
    expect(hoistedName("main.parse__struct_123")).toBeUndefined();
    expect(hoistedName("struct { u8, u16 }")).toBeUndefined();
    expect(hoistedName("main.delete")).toBeUndefined();
  });
});

describe("toolchain and build inputs", () => {
  test("parses Zig versions and enforces the minimum", () => {
    expect(parseZigVersion("0.15.2\n")).toEqual({ major: 0, minor: 15, patch: 2 });
    expect(parseZigVersion("0.16.0-dev.123+abc")).toEqual({ major: 0, minor: 16, patch: 0 });
    expect(parseZigVersion("nope")).toBeUndefined();
    expect(zigVersionSupported("0.16.0")).toBe(true);
    expect(zigVersionSupported("0.17.0-dev.1+abc")).toBe(true);
    expect(zigVersionSupported("1.0.0")).toBe(true);
    expect(zigVersionSupported("0.15.2")).toBe(false);
  });

  test("maps targets to Zig triples and refuses Windows for now", () => {
    expect(zigTarget("darwin-arm64")).toBe("aarch64-macos");
    expect(zigTarget("darwin-x64")).toBe("x86_64-macos");
    expect(zigTarget("linux-arm64")).toBe("aarch64-linux-gnu");
    expect(zigTarget("linux-x64")).toBe("x86_64-linux-gnu");
    expect(() => zigTarget("windows-x64")).toThrow("not supported for windows-x64");
  });

  test("names entry points, FFI bindings, and the manifest probe after the module", () => {
    expect(moduleSymbolPrefix("git")).toBe("quickgui_module_git");
    expect(moduleSymbolPrefix("image-codec")).toBe("quickgui_module_image_codec");
    expect(nativeModuleFfiFunctions("quickgui_module_git")).toEqual([
      {
        name: "quickgui_module_git_call",
        symbol: "quickgui_module_git_call",
        params: [
          "u32",
          "bytes",
          { callback: { id: "reply", params: ["bytes", { context: "reply" }], returns: "void", lifetime: "call" } },
          { context: "reply" },
        ],
        returns: "void",
      },
      {
        name: "quickgui_module_git_call_async",
        symbol: "quickgui_module_git_call_async",
        params: ["u32", "u32", "bytes"],
        returns: "void",
      },
    ]);
    expect(manifestProbeSource("quickgui_module_git")).toContain("fputs(quickgui_module_git_manifest(), stdout);");
    expect(manifestProbeSource("quickgui_module_git")).toContain("void quickgui_module_complete(");
    expect(probeBuildArguments("/usr/bin/zig", "/o/probe.c", "/o/libgit.a", "/o/probe")).toEqual([
      "/usr/bin/zig", "cc", "-o", "/o/probe", "/o/probe.c", "/o/libgit.a",
    ]);
  });

  test("builds the zig build-lib invocation", () => {
    expect(
      zigBuildArguments({
        zig: "/usr/bin/zig",
        name: "git",
        optimize: "ReleaseFast",
        target: "darwin-arm64",
        root: "/p/.quickgui/modules/git/darwin-arm64/root.zig",
        entry: "/p/modules/git/main.zig",
        runtime: "/n/zig/quickgui.zig",
        output: "/p/.quickgui/modules/git/darwin-arm64/libgit.a",
        cacheDirectory: "/p/.quickgui/zig-cache",
      }),
    ).toEqual([
      "/usr/bin/zig",
      "build-lib",
      "-static",
      "-OReleaseFast",
      "-target",
      "aarch64-macos",
      "-lc",
      "--name",
      "git",
      "-femit-bin=/p/.quickgui/modules/git/darwin-arm64/libgit.a",
      "--cache-dir",
      "/p/.quickgui/zig-cache",
      "--dep",
      "quickgui",
      "--dep",
      "module",
      "-Mroot=/p/.quickgui/modules/git/darwin-arm64/root.zig",
      "--dep",
      "quickgui",
      "-Mmodule=/p/modules/git/main.zig",
      "-Mquickgui=/n/zig/quickgui.zig",
    ]);
  });

  test("discovers module directories that contain main.zig", () => {
    const root = fixtureCopy();
    expect(discoverNativeModules(join(root, "modules")).map((module) => module.name)).toEqual(["echo"]);
    expect(discoverNativeModules(join(root, "missing"))).toEqual([]);
    expect(zigSourceFiles(join(root, "modules", "echo"))).toEqual(["main.zig"]);
    writeFileSync(join(root, "modules", "bad name.zig"), "");
    expect(discoverNativeModules(join(root, "modules")).map((module) => module.name)).toEqual(["echo"]);
  });

  test("the watcher ignores generated module entry points and Zig caches", () => {
    const root = "/p";
    expect(shouldIgnoreChange(root, "/p/modules/git/index.ts", "/p/dist", "/p/modules")).toBe(true);
    expect(shouldIgnoreChange(root, "/p/modules/git/main.zig", "/p/dist", "/p/modules")).toBe(false);
    expect(shouldIgnoreChange(root, "/p/modules/index.ts", "/p/dist", "/p/modules")).toBe(false);
    expect(shouldIgnoreChange(root, "/p/src/.zig-cache/h/x", "/p/dist")).toBe(true);
    expect(shouldIgnoreChange(root, "/p/modules/git/index.ts", "/p/dist")).toBe(false);
  });

  test("resolves the modules configuration", () => {
    const config = resolveConfig({ name: "Demo", identifier: "com.example.demo" }, "/p");
    expect(config.modules).toEqual({ directory: "/p/modules" });
    expect(
      resolveConfig(
        { name: "Demo", identifier: "com.example.demo", modules: { directory: "native", optimize: "Debug" } },
        "/p",
      ).modules,
    ).toEqual({ directory: "/p/native", optimize: "Debug" });
    expect(() =>
      resolveConfig({ name: "Demo", identifier: "com.example.demo", modules: { optimize: "Fast" } }, "/p"),
    ).toThrow("`modules.optimize` must be one of");
  });
});

const zigAvailable = (() => {
  try {
    findZig();
    return true;
  } catch {
    return false;
  }
})();

describe.skipIf(!zigAvailable)("compiling native modules", () => {
  test("compiles the fixture, generates its entry point, and round-trips every value shape", async () => {
    const root = fixtureCopy();
    const config = resolveConfig({ name: "Fixture", identifier: "com.example.fixture" }, root);
    const target = hostTarget();
    const lines: string[] = [];
    const built = await buildNativeModules(config, { target, mode: "development", log: (line) => lines.push(line) });
    expect(built.map((module) => [module.name, module.rebuilt])).toEqual([["echo", true]]);
    expect(lines[0]).toMatch(/^Built native module echo with Zig \d+\.\d+/);
    const archivePath = join(root, ".quickgui", "modules", "echo", target, "libecho.a");
    expect(existsSync(archivePath)).toBe(true);
    expect(built[0]!.archivePath).toBe(archivePath);
    expect(built[0]!.symbolPrefix).toBe("quickgui_module_echo");
    expect(built[0]!.manifest.functions.map((spec) => spec.name)).toEqual([
      "add", "scale", "negate", "nothing", "repeat", "lengthZ", "upper", "reverseBytes", "parseHunk",
      "countLines", "depth", "area", "describe", "pair", "maybe", "fail", "tooBig", "sumAll", "echoAny",
      "slowSquare",
    ]);
    const indexPath = join(root, "modules", "echo", "index.ts");
    const generated = readFileSync(indexPath, "utf8");
    expect(generated).toContain("declare function quickgui_module_echo_call(");
    expect(generated).toContain("export interface Hunk {");
    expect(generated).toContain("export function parseHunk(text: string): Hunk {");
    expect(generated).toContain("export function pair(first: number, second: string): [number, string] {");
    expect(generated).toContain("export function echoAny(value: unknown): unknown {");

    // A second build reuses the library and leaves the entry point untouched.
    const again = await buildNativeModules(config, { target, mode: "development" });
    expect(again[0]!.rebuilt).toBe(false);
    expect(readFileSync(indexPath, "utf8")).toBe(generated);

    // Editing the Zig source invalidates the cached library.
    const entry = join(root, "modules", "echo", "main.zig");
    writeFileSync(entry, `${readFileSync(entry, "utf8")}\npub fn extra() u8 {\n    return 7;\n}\n`);
    const rebuilt = await buildNativeModules(config, { target, mode: "development" });
    expect(rebuilt[0]!.rebuilt).toBe(true);
    expect(readFileSync(indexPath, "utf8")).toContain("export function extra(): number {");
  }, 240_000);

  test("reports Zig compile errors", async () => {
    const root = fixtureCopy();
    writeFileSync(join(root, "modules", "echo", "main.zig"), "pub fn broken() u32 {\n    return \"text\";\n}\n");
    const config = resolveConfig({ name: "Fixture", identifier: "com.example.fixture" }, root);
    await expect(
      buildNativeModules(config, { target: hostTarget(), mode: "development" }),
    ).rejects.toThrow("Zig could not compile native module echo");
  }, 240_000);
});

const hostLibraryStaged = existsSync(
  resolve(import.meta.dir, "..", "..", "native", "lib", hostTarget(), "libquickgui_host.a"),
);

describe.skipIf(!zigAvailable || !hostLibraryStaged || Bun.which("node") === null || process.platform !== "darwin")(
  "native modules in a compiled application",
  () => {
    test("calls every value shape through the FFI, synchronously and asynchronously", async () => {
      const root = fixtureCopy();
      const config = resolveConfig(
        { name: "Fixture", identifier: "com.example.fixture", entry: "src/app.ts" },
        root,
      );
      const result = await buildProject(config, { mode: "development", target: hostTarget() });
      const child = Bun.spawn([result.executablePath], { cwd: root, stdin: "ignore", stdout: "pipe", stderr: "pipe" });
      const [status, stdout, stderr] = await Promise.all([
        child.exited,
        new Response(child.stdout).text(),
        new Response(child.stderr).text(),
      ]);
      expect({ status, stderr }).toEqual({ status: 0, stderr: "" });
      expect(stdout.trim().split("\n")).toEqual([
        "add=42",
        "scale=3",
        "negate=false",
        "repeat=ababab",
        "lengthZ=6",
        "upper=SHOUT",
        "reverseBytes=3,2,1",
        'parseHunk={"heading":"parsed","lines":[{"kind":"context","text":"keep","number":1},{"kind":"removed","text":"old","number":null},{"kind":"added","text":"new","number":2}],"stats":{"added":1,"removed":1}}',
        "countLines=1",
        "depth=3",
        "area=9",
        'describe={"none":{}}',
        'pair=[7,"seven"]',
        "maybeNull=null",
        "maybe=8",
        "sumAll=6.5",
        'echoAny={"a":[1,"x",null]}',
        "fail=echo.fail failed with NotFound",
        "tooBig=echo.tooBig failed with IntegerOutOfRange",
        "notInteger=echo.add failed with NotAnInteger",
        "slowSquareAsync=4999950000",
        "addAsync=3",
        "repeatAsync=xx",
        "failAsync=echo.fail failed with Invalid",
      ]);
    }, 600_000);
  },
);
