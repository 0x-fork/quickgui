import { afterAll, describe, expect, test } from "bun:test";
import { cpSync, existsSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { join, resolve } from "node:path";

import { NATIVE_MODULE_ABI, type NativeModuleManifest } from "@quickgui/native/modules";

import { resolveConfig } from "./config.ts";
import { shouldIgnoreChange } from "./dev.ts";
import {
  buildNativeModules,
  discoverNativeModules,
  extractZigFunctionSignatures,
  findZig,
  generateNativeModuleSource,
  hoistedName,
  parseZigVersion,
  zigBuildArguments,
  zigSourceFiles,
  zigTarget,
  zigVersionSupported,
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
    addonImportPath: "../../.quickgui/modules/echo/darwin-arm64/echo.node",
    entryDisplayPath: "modules/echo/main.zig",
    zigSource: sampleZig,
  });

  test("hoists named Zig types, inlines anonymous ones, and resolves recursive references", () => {
    expect(source).toContain('import { bindNativeModule, type NativeModuleBinding } from "@quickgui/native/modules";');
    expect(source).toContain(
      'const binding: NativeModuleBinding = {"abi":1,"functions":[{"name":"parseHunk","params":["allocator","string"],"result":"json"},{"name":"depth","params":["json"],"result":"number"},{"name":"area","params":["json"],"result":"number"},{"name":"reverse","params":["bytes"],"result":"bytes"},{"name":"nothing","params":[],"result":"void"}]};',
    );
    expect(source).toContain('return require("../../.quickgui/modules/echo/darwin-arm64/echo.node");');
    expect(source).toContain(
      "export interface Hunk {\n  heading: string;\n  lines: Line[];\n  stats: { added: number; };\n}",
    );
    expect(source).toContain("export interface Line {\n  kind: Kind;\n  number?: number | null;\n}");
    expect(source).toContain('export type Kind = "context" | "added";');
    expect(source).toContain("export interface Node {\n  children?: Node[];\n}");
    expect(source).toContain("export type Shape = { circle: number } | { none: Record<string, never> };");
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
        "export function parseHunk(text: string | Uint8Array): Hunk {",
        "  return native.call(0, [text]) as Hunk;",
        "}",
      ].join("\n"),
    );
    expect(source).toContain(
      "export function parseHunkAsync(text: string | Uint8Array): Promise<Hunk> {\n  return native.callAsync(0, [text]) as Promise<Hunk>;\n}",
    );
    expect(source).toContain("export function depth(node: Node): number {");
    expect(source).toContain("export function area(shape: Shape): number {");
    expect(source).toContain("export function reverse(bytes: Uint8Array): Uint8Array {");
    expect(source).toContain("export function nothing(): void {\n  return native.call(4, []) as void;\n}");
  });

  test("falls back to positional names and rejects unusable export names", () => {
    const positional = generateNativeModuleSource({
      name: "echo",
      manifest: { ...sampleManifest, functions: [sampleManifest.functions[0]!] },
      addonImportPath: "./echo.node",
      entryDisplayPath: "modules/echo/main.zig",
    });
    expect(positional).toContain("export function parseHunk(arg1: string | Uint8Array): Hunk {");
    const reserved = { ...sampleManifest.functions[4]!, name: "delete" };
    expect(() =>
      generateNativeModuleSource({
        name: "echo",
        manifest: { ...sampleManifest, functions: [reserved] },
        addonImportPath: "./echo.node",
        entryDisplayPath: "modules/echo/main.zig",
      }),
    ).toThrow("exports `delete`, which is not a usable JavaScript export name");
    const clash = { ...sampleManifest.functions[4]!, name: "nothingAsync" };
    expect(() =>
      generateNativeModuleSource({
        name: "echo",
        manifest: { ...sampleManifest, functions: [sampleManifest.functions[4]!, clash] },
        addonImportPath: "./echo.node",
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
        output: "/p/.quickgui/modules/git/darwin-arm64/git.node",
        cacheDirectory: "/p/.quickgui/zig-cache",
      }),
    ).toEqual([
      "/usr/bin/zig",
      "build-lib",
      "-dynamic",
      "-OReleaseFast",
      "-target",
      "aarch64-macos",
      "-fallow-shlib-undefined",
      "--name",
      "git",
      "-femit-bin=/p/.quickgui/modules/git/darwin-arm64/git.node",
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
    const addonPath = join(root, ".quickgui", "modules", "echo", target, "echo.node");
    expect(existsSync(addonPath)).toBe(true);
    const indexPath = join(root, "modules", "echo", "index.ts");
    const generated = readFileSync(indexPath, "utf8");
    expect(generated).toContain(`return require("../../.quickgui/modules/echo/${target}/echo.node");`);
    expect(generated).toContain("export interface Hunk {");
    expect(generated).toContain("export function parseHunk(text: string | Uint8Array): Hunk {");

    // A second build reuses the addon and leaves the entry point untouched.
    const again = await buildNativeModules(config, { target, mode: "development" });
    expect(again[0]!.rebuilt).toBe(false);
    expect(readFileSync(indexPath, "utf8")).toBe(generated);

    const echo = (await import(indexPath)) as Record<string, (...values: unknown[]) => unknown>;
    expect(echo.add!(2, 40)).toBe(42);
    expect(echo.scale!(1.5, 2)).toBe(3);
    expect(echo.negate!(true)).toBe(false);
    expect(echo.nothing!()).toBeUndefined();
    expect(echo.repeat!("ab", 3)).toBe("ababab");
    expect(echo.repeat!(new TextEncoder().encode("é"), 2)).toBe("éé");
    expect(echo.lengthZ!("héllo")).toBe(6);
    expect(echo.upper!("shout")).toBe("SHOUT");
    expect(Array.from(echo.reverseBytes!(new Uint8Array([1, 2, 3])) as Uint8Array)).toEqual([3, 2, 1]);
    expect(echo.parseHunk!(" keep\n-old\n+new")).toEqual({
      heading: "parsed",
      lines: [
        { kind: "context", text: "keep", number: 1 },
        { kind: "removed", text: "old", number: null },
        { kind: "added", text: "new", number: 2 },
      ],
      stats: { added: 1, removed: 1 },
    });
    expect(echo.countLines!({ heading: "h", lines: [{ kind: "added", text: "x" }], stats: { added: 1, removed: 0 } })).toBe(1);
    expect(echo.depth!({ name: "root", children: [{ name: "a", children: [{ name: "b" }] }] })).toBe(3);
    expect(echo.area!({ square: { side: 3 } })).toBe(9);
    expect(echo.describe!({ none: {} })).toEqual({ none: {} });
    expect(echo.pair!(7, "seven")).toEqual([7, "seven"]);
    expect(echo.maybe!(null)).toBeNull();
    expect(echo.maybe!(4)).toBe(8);
    expect(echo.sumAll!([1, 2, 3.5])).toBe(6.5);
    expect(echo.echoAny!({ a: [1, "x", null] })).toEqual({ a: [1, "x", null] });
    expect(() => echo.fail!(1)).toThrow("echo.fail failed with NotFound");
    expect(() => echo.tooBig!()).toThrow("IntegerOutOfRange");
    expect(() => echo.add!(1.5, 1)).toThrow("NotAnInteger");
    expect(() => echo.add!("1", 1)).toThrow("echo.add: argument 1 must be a number");
    await expect(echo.slowSquareAsync!(100_000)).resolves.toBe(4_999_950_000);
    await expect(Promise.all([echo.addAsync!(1, 2), echo.repeatAsync!("x", 2)])).resolves.toEqual([3, "xx"]);
    await expect(echo.failAsync!(2)).rejects.toThrow("echo.fail failed with Invalid");

    // Editing the Zig source invalidates the cached addon.
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
