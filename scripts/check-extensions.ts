#!/usr/bin/env bun
/** Headless integration proof against the real staged core and optional backend images. */
import {
  existsSync,
  mkdirSync,
  mkdtempSync,
  readFileSync,
  readdirSync,
  rmSync,
  writeFileSync,
} from "node:fs";
import { tmpdir } from "node:os";
import { basename, dirname, join, resolve } from "node:path";
import { compileNativeApplication, resolveHostLibrary } from "../packages/cli/src/native-build.ts";
import { resolveConfig } from "../packages/cli/src/config.ts";
import { hostTarget } from "../packages/cli/src/targets.ts";

const root = resolve(import.meta.dir, "..");
const target = hostTarget();
const version = JSON.parse(readFileSync(join(root, "package.json"), "utf8")).version as string;
async function run(
  argv: string[],
  cwd = root,
  extra: Record<string, string | undefined> = {},
): Promise<string> {
  const child = Bun.spawn(argv, {
    cwd,
    env: { ...process.env, CGO_ENABLED: "0", ...extra },
    stdin: "ignore",
    stdout: "pipe",
    stderr: "pipe",
  });
  const [status, stdout, stderr] = await Promise.all([
    child.exited,
    new Response(child.stdout).text(),
    new Response(child.stderr).text(),
  ]);
  if (status !== 0) throw new Error(`${argv.join(" ")}: ${stderr}\n${stdout}`);
  return stdout;
}
const coreGraph = await run([
  "cargo",
  "tree",
  "-p",
  "quickgui-host",
  "--edges",
  "normal",
  "--prefix",
  "none",
]);
if (
  /^(libghostty|portable-pty|quickgui-terminal|quickgui-updater|ed25519-dalek|minisign-verify)\b/m.test(
    coreGraph,
  )
)
  throw new Error("Core library pulled in an optional backend dependency");
const extensionGraph = await run([
  "cargo",
  "tree",
  "-p",
  "quickgui-terminal",
  "--edges",
  "normal",
  "--prefix",
  "none",
]);
if (/^(quickgui |quickgui-host|wgpu|taffy)\b/m.test(extensionGraph))
  throw new Error("Terminal backend pulled in the renderer or runtime");

const directory = mkdtempSync(join(tmpdir(), "quickgui-native-extensions-"));
try {
  const core = resolveHostLibrary(target, root);
  const backendManifest = JSON.parse(
    readFileSync(join(root, "go/terminal/quickgui.extension.json"), "utf8"),
  );
  const { extensionLibraryName } = await import("../packages/cli/src/extensions.ts");
  const backend = join(
    root,
    "packages/native-terminal/lib",
    target,
    extensionLibraryName(backendManifest, target),
  );
  if (!existsSync(backend))
    throw new Error(
      "Build the terminal extension first: bun packages/native/build.ts --extension terminal",
    );
  await run(
    ["go", "test", "./internal/ffi", "-run", "TestExtensionLibrarySmoke", "-count=1"],
    join(root, "go"),
    { QUICKGUI_TEST_CORE: core, QUICKGUI_TEST_TERMINAL: backend },
  );
  for (const extensions of [[], ["terminal"], ["updater"], ["terminal", "updater"]]) {
    const project = join(directory, extensions.join("-") || "core");
    mkdirSync(project);
    writeFileSync(
      join(project, "go.mod"),
      `module example.test/extensions\n\ngo 1.23\n\nrequire github.com/egoist/quickgui/go v${version}\nreplace github.com/egoist/quickgui/go => ${JSON.stringify(join(root, "go"))}\n`,
    );
    writeFileSync(
      join(project, "main.go"),
      `package main
import (
  "github.com/egoist/quickgui/go/host"
  _ "github.com/egoist/quickgui/go/ui"
  ${extensions.map((name) => '_ "github.com/egoist/quickgui/go/' + name + '"').join("\n")}
)
func main() { if err := host.Load(); err != nil { panic(err) } }
`,
    );
    await run(["go", "mod", "tidy"], project, { GOWORK: "off" });
    const executablePath =
      process.platform === "darwin"
        ? join(project, "Test.app/Contents/MacOS/Test")
        : join(project, process.platform === "win32" ? "Test.exe" : "Test");
    mkdirSync(dirname(executablePath), { recursive: true });
    const libraries = await compileNativeApplication({
      config: resolveConfig(
        { name: "Test", identifier: "dev.quickgui.extension-test", native: { libraryPath: core } },
        project,
      ),
      mode: "development",
      target,
      executablePath,
      fonts: [],
    });
    const expectedImages = 1 + extensions.length;
    const expectedResources = extensions.includes("updater") ? 1 : 0;
    if (libraries.length !== expectedImages + expectedResources)
      throw new Error("Incorrect selected extension set");
    const actual = readdirSync(dirname(libraries[0]!)).filter((name) =>
      /\.(dylib|so|dll)$/.test(name),
    );
    if (
      actual.length !== expectedImages ||
      actual.some((name) => !libraries.some((path) => basename(path) === name))
    )
      throw new Error("Bundle contains unexpected libraries");
    await run([executablePath], directory, {
      QUICKGUI_LIBRARY: undefined,
      QUICKGUI_HOST_LIB: undefined,
      QUICKGUI_EXTENSION_DIR: undefined,
    });
    console.log(
      `[extensions] ${extensions.join(" + ") || "Core-only"}: ${actual.join(", ")} — loaded through purego`,
    );
  }
} finally {
  rmSync(directory, { recursive: true, force: true });
}
const updaterGraph = await run([
  "cargo",
  "tree",
  "-p",
  "quickgui-updater",
  "--edges",
  "normal",
  "--prefix",
  "none",
]);
if (/^(quickgui |quickgui-host|wgpu|taffy)\b/m.test(updaterGraph))
  throw new Error("Updater links another renderer/runtime");
console.log("Native extension dependency and bundle checks passed");
