import { afterEach, expect, test } from "bun:test";
import { createHash } from "node:crypto";
import { mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { resolveConfig } from "./config.ts";
import {
  discoverExtensions,
  extensionLibraryName,
  extensionManifests,
  parseGoPackages,
  readBounded,
  resolveExtension,
  verifyArchive,
  type ExtensionManifest,
} from "./extensions.ts";

const directories: string[] = [];
const originalDirectory = process.env.QUICKGUI_EXTENSION_DIR;
afterEach(() => {
  for (const directory of directories.splice(0))
    rmSync(directory, { recursive: true, force: true });
  if (originalDirectory === undefined) delete process.env.QUICKGUI_EXTENSION_DIR;
  else process.env.QUICKGUI_EXTENSION_DIR = originalDirectory;
});
function temporary(): string {
  const path = mkdtempSync(join(tmpdir(), "quickgui-extension-test-"));
  directories.push(path);
  return path;
}
const terminal: ExtensionManifest = {
  schema: 1,
  name: "terminal",
  abi: 1,
  package: "@quickgui/native-terminal",
  version: "0.1.3",
  library: "quickgui_terminal",
};
function manifest(directory: string, value: unknown = terminal): void {
  mkdirSync(directory, { recursive: true });
  writeFileSync(join(directory, "quickgui.extension.json"), JSON.stringify(value));
}

test("Go package stream supports braces and quotes inside nested strings", () => {
  const packages = [{ Dir: '/path/"{nested}', Module: { Path: "example" } }, { Dir: "/other" }];
  expect(parseGoPackages(packages.map((value) => JSON.stringify(value)).join("\n"))).toEqual(
    packages,
  );
  expect(() => parseGoPackages('{"Dir":"unfinished')).toThrow();
  expect(() => parseGoPackages("{}garbage")).toThrow();
});

test("only imported packages opt in, and equivalent transitive requirements deduplicate", () => {
  const root = temporary();
  const normal = join(root, "ui");
  const first = join(root, "terminal");
  const second = join(root, "transitive");
  mkdirSync(normal);
  manifest(first);
  manifest(second, { ...terminal, description: "Extra metadata does not change the dependency" });
  expect(extensionManifests([{ Dir: normal }])).toEqual([]);
  expect(extensionManifests([{ Dir: normal }, { Dir: first }, { Dir: second }])).toEqual([
    terminal,
  ]);
  manifest(second, { ...terminal, version: "0.2.0" });
  expect(() => extensionManifests([{ Dir: first }, { Dir: second }])).toThrow("Conflicting");
  manifest(second, { ...terminal, library: "../../arbitrary" });
  expect(() => extensionManifests([{ Dir: second }])).toThrow("Invalid");
});

test("Go dependency discovery respects build tags, target files, and transitive imports", async () => {
  if (!Bun.which("go"))
    throw new Error("Go must be on PATH for extension import integration tests");
  const root = temporary();
  writeFileSync(join(root, "go.mod"), "module example.test/extensions\n\ngo 1.23\n");
  writeFileSync(join(root, "main.go"), "package main\nfunc main() {}\n");
  writeFileSync(
    join(root, "terminal.go"),
    '//go:build terminal\n\npackage main\nimport _ "example.test/extensions/widget"\n',
  );
  const widget = join(root, "widget");
  mkdirSync(widget);
  writeFileSync(
    join(widget, "widget_linux.go"),
    'package widget\nimport _ "example.test/extensions/terminal"\n',
  );
  writeFileSync(join(widget, "widget_darwin.go"), "package widget\n");
  const extension = join(root, "terminal");
  manifest(extension);
  writeFileSync(join(extension, "terminal.go"), "package terminal\n");
  const config = resolveConfig({ name: "Example", identifier: "test.extensions" }, root);
  const env = { ...process.env, CGO_ENABLED: "0", GOOS: "linux", GOARCH: "arm64", GOWORK: "off" };
  expect(await discoverExtensions(config, ".", env)).toEqual([]);
  config.native.tags = ["terminal"];
  expect(await discoverExtensions(config, ".", env)).toEqual([terminal]);
  expect(await discoverExtensions(config, ".", { ...env, GOOS: "darwin" })).toEqual([]);
}, 30_000);

test("extension names and explicit offline directories resolve per target", async () => {
  const root = temporary();
  const filename = extensionLibraryName(terminal, "darwin-arm64");
  expect(filename).toBe("libquickgui_terminal.dylib");
  expect(extensionLibraryName(terminal, "windows-x64")).toBe("quickgui_terminal.dll");
  expect(extensionLibraryName(terminal, "linux-x64")).toBe("libquickgui_terminal.so");
  writeFileSync(join(root, filename), "test-native-library");
  process.env.QUICKGUI_EXTENSION_DIR = root;
  expect(readFileSync(await resolveExtension(terminal, "darwin-arm64", root), "utf8")).toBe(
    "test-native-library",
  );
  await expect(resolveExtension(terminal, "windows-x64", root)).rejects.toThrow("missing");
});

test("archive bytes must match their SHA512 integrity", () => {
  const archive = new TextEncoder().encode("native archive");
  const integrity = `sha512-${createHash("sha512").update(archive).digest("base64")}`;
  expect(() => verifyArchive(archive, integrity)).not.toThrow();
  expect(() => verifyArchive(new TextEncoder().encode("corrupt"), integrity)).toThrow("integrity");
  expect(() => verifyArchive(archive, "sha1-not-accepted")).toThrow("integrity");
});

test("stream limits apply while reading decompressed data, cancelling oversized streams", async () => {
  let cancelled = false;
  const stream = new ReadableStream<Uint8Array>({
    pull(controller) {
      controller.enqueue(new Uint8Array(8));
    },
    cancel() {
      cancelled = true;
    },
  });
  await expect(readBounded(stream, 12)).rejects.toThrow("size limit");
  expect(cancelled).toBe(true);
  expect(await readBounded(new Response("abc").body!, 3)).toEqual(Buffer.from("abc"));
});

test("exact-version downloads verify integrity and repair a corrupt cache", async () => {
  const { spyOn } = await import("bun:test");
  const root = temporary();
  const extension = {
    ...terminal,
    name: "cache-test",
    package: "@quickgui/native-cache-test",
    library: "quickgui_cache_test",
    resources: { darwin: ["Sparkle.framework.qgr"] },
  };
  const filename = extensionLibraryName(extension, "darwin-arm64");
  const payload = join(root, "package/lib/darwin-arm64");
  mkdirSync(payload, { recursive: true });
  writeFileSync(join(payload, filename), "verified extension payload");
  writeFileSync(join(payload, "Sparkle.framework.qgr"), "verified extension resource");
  const archive = join(root, "extension.tgz");
  const tar = Bun.spawnSync(["tar", "-czf", archive, "package"], { cwd: root, stderr: "pipe" });
  expect(tar.exitCode).toBe(0);
  const bytes = readFileSync(archive);
  const url =
    "https://registry.npmjs.org/@quickgui/native-cache-test/-/native-cache-test-0.1.3.tgz";
  let calls = 0;
  const fetchMock = spyOn(globalThis, "fetch").mockImplementation(
    Object.assign(
      async (input: string | Request | URL) => {
        calls++;
        return String(input) === url
          ? new Response(bytes)
          : Response.json({
              name: extension.package,
              version: extension.version,
              dist: {
                tarball: url,
                integrity: `sha512-${createHash("sha512").update(bytes).digest("base64")}`,
              },
            });
      },
      { preconnect: globalThis.fetch.preconnect },
    ),
  );
  const originalCache = process.env.QUICKGUI_CACHE_DIR;
  delete process.env.QUICKGUI_EXTENSION_DIR;
  process.env.QUICKGUI_CACHE_DIR = join(root, "cache");
  try {
    const path = await resolveExtension(extension, "darwin-arm64", root);
    expect(readFileSync(path, "utf8")).toBe("verified extension payload");
    expect(calls).toBe(2);
    expect(await resolveExtension(extension, "darwin-arm64", root)).toBe(path);
    expect(calls).toBe(2);
    writeFileSync(path, "corrupted");
    await resolveExtension(extension, "darwin-arm64", root);
    expect(readFileSync(path, "utf8")).toBe("verified extension payload");
    expect(calls).toBe(4);
    const resource = await resolveExtension(
      extension,
      "darwin-arm64",
      root,
      "Sparkle.framework.qgr",
    );
    expect(readFileSync(resource, "utf8")).toBe("verified extension resource");
    expect(calls).toBe(6);
    expect(await resolveExtension(extension, "darwin-arm64", root, "Sparkle.framework.qgr")).toBe(
      resource,
    );
    expect(calls).toBe(6);
    await expect(
      resolveExtension(extension, "darwin-arm64", root, "undeclared.qgr"),
    ).rejects.toThrow();
  } finally {
    fetchMock.mockRestore();
    if (originalCache === undefined) delete process.env.QUICKGUI_CACHE_DIR;
    else process.env.QUICKGUI_CACHE_DIR = originalCache;
  }
});
