#!/usr/bin/env bun
/** Build and load all extension templates from the actual npm archive, outside the checkout. */
import {
  appendFileSync,
  cpSync,
  existsSync,
  mkdirSync,
  mkdtempSync,
  readFileSync,
  rmSync,
  writeFileSync,
} from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { pathToFileURL } from "node:url";
import { resolveHostLibrary } from "../packages/cli/src/native-build.ts";
import { hostTarget } from "../packages/cli/src/targets.ts";

const root = resolve(import.meta.dir, "..");
const localMoon = join(root, "target/moonbit-toolchain");
const moonHome =
  process.env.MOON_HOME ?? (existsSync(join(localMoon, "bin/moon")) ? localMoon : undefined);
if (moonHome)
  Object.assign(process.env, {
    MOON_HOME: moonHome,
    PATH: `${join(moonHome, "bin")}${process.platform === "win32" ? ";" : ":"}${process.env.PATH ?? ""}`,
  });
const target = hostTarget();
const core = resolveHostLibrary(target, root);
const directory = mkdtempSync(join(tmpdir(), "quickgui-extension-templates-"));
const version = JSON.parse(readFileSync(join(root, "packages/cli/package.json"), "utf8")).version;

async function run(
  argv: string[],
  cwd: string,
  env: Record<string, string | undefined> = {},
): Promise<string> {
  const child = Bun.spawn(argv, {
    cwd,
    env: { ...process.env, CGO_ENABLED: "0", GOWORK: "off", ...env },
    stdin: "ignore",
    stdout: "pipe",
    stderr: "pipe",
  });
  const timer = setTimeout(() => child.kill(), 120_000);
  try {
    const [status, stdout, stderr] = await Promise.all([
      child.exited,
      new Response(child.stdout).text(),
      new Response(child.stderr).text(),
    ]);
    if (status !== 0) throw new Error(`${argv.join(" ")}\n${stdout}\n${stderr}`);
    return stdout;
  } finally {
    clearTimeout(timer);
  }
}

try {
  await run(
    [process.execPath, "pm", "pack", "--destination", directory, "--quiet"],
    join(root, "packages/cli"),
  );
  await run(["tar", "-xzf", join(directory, `quickgui-cli-${version}.tgz`), "-C", directory], root);
  const cli = join(directory, "package/src/cli.ts");
  // These imports also catch missing modules in package.json's files allowlist.
  const { compileNativeApplication } = await import(
    pathToFileURL(join(directory, "package/src/native-build.ts")).href
  );
  const { resolveConfig } = await import(
    pathToFileURL(join(directory, "package/src/config.ts")).href
  );

  for (const type of ["go", "rust", "zig", "moonbit"] as const) {
    const name = `template-${type}`;
    const module = `example.test/${name}`;
    const project = join(directory, `${type} project`);
    await run(
      [
        process.execPath,
        cli,
        "init-extension",
        project,
        "--type",
        type,
        "--name",
        name,
        "--module",
        module,
        "--no-install",
        ...(type === "go" ? [] : ["--npm-package", `@template/${name}`]),
      ],
      directory,
    );
    appendFileSync(
      join(project, "go.mod"),
      `\nreplace github.com/egoist/quickgui/go => ${JSON.stringify(join(root, "go"))}\n`,
    );
    await run(["go", "mod", "tidy"], project);
    await run(
      ["go", "run", "github.com/egoist/quickgui/go/cmd/quickguifmt", "-check", "."],
      project,
    );
    const formatted = await run(["gofmt", "-l", "extension.go", "cmd/demo"], project);
    if (formatted.trim()) throw new Error(`Unformatted ${type} template: ${formatted}`);

    if (type !== "go") {
      // Editing one manifest must update both the Go requirement and native/npm versions.
      const path = join(project, "quickgui.extension.json");
      const manifest = JSON.parse(readFileSync(path, "utf8"));
      manifest.version = "0.2.3";
      writeFileSync(path, JSON.stringify(manifest, null, 2) + "\n");
    }
    await run([process.execPath, "scripts/build.ts"], project);
    // Build the actual UI demo as well as the headless consumer below.
    await run(
      [
        "go",
        "build",
        "-o",
        join(project, process.platform === "win32" ? "demo.exe" : "demo"),
        "./cmd/demo",
      ],
      project,
    );

    if (type !== "go") {
      const { extensionLibraryName } = await import("../packages/cli/src/extensions.ts");
      const manifest = JSON.parse(readFileSync(join(project, "quickgui.extension.json"), "utf8"));
      const artifact = JSON.parse(readFileSync(join(project, "artifacts/package.json"), "utf8"));
      if (artifact.name !== manifest.package || artifact.version !== manifest.version)
        throw new Error(`${type} artifact metadata did not follow the manifest`);
      const provider = join(
        project,
        "artifacts/lib",
        target,
        extensionLibraryName(manifest, target),
      );
      await run(
        ["go", "test", "./internal/ffi", "-run", "TestIndependentServiceLibrarySmoke", "-count=1"],
        join(root, "go"),
        {
          QUICKGUI_TEST_CORE: core,
          QUICKGUI_TEST_SERVICE: provider,
          QUICKGUI_TEST_SERVICE_NAME: manifest.name,
          QUICKGUI_TEST_SERVICE_VERSION: manifest.version,
        },
      );
      cpSync(join(project, "artifacts"), join(project, "node_modules", manifest.package), {
        recursive: true,
      });
      if (type === "rust") await run(["cargo", "fmt", "--check"], join(project, "native"));
      else if (type === "moonbit") {
        await run(["moon", "fmt", "--check"], join(project, "native"));
        for (const profile of ["--debug", "--release"]) {
          await run(
            ["moon", "test", "--target", "native", profile, "--deny-warn"],
            join(project, "native"),
          );
        }
        if (process.platform === "darwin") {
          const symbols = await run(["nm", "-gU", provider], project);
          if (
            symbols
              .trim()
              .split("\n")
              .some((line) => !line.endsWith(" _quickgui_extension_v1"))
          )
            throw new Error(`MoonBit provider leaked runtime symbols:\n${symbols}`);
        }
      } else
        await run(
          [
            process.env.ZIG ?? "zig",
            "fmt",
            "--check",
            "native/extension.zig",
            "native/build_info.zig",
          ],
          project,
        );
    }

    const consumer = join(project, "cmd/check");
    mkdirSync(consumer, { recursive: true });
    writeFileSync(
      join(consumer, "main.go"),
      `package main
import (
  "github.com/egoist/quickgui/go/host"
  _ "${module}"
)
func main() { if err := host.Load(); err != nil { panic(err) } }
`,
    );
    const executablePath =
      process.platform === "darwin"
        ? join(project, "Check.app/Contents/MacOS/Check")
        : join(project, "bundle", process.platform === "win32" ? "Check.exe" : "Check");
    mkdirSync(dirname(executablePath), { recursive: true });
    const libraries: string[] = await compileNativeApplication({
      config: resolveConfig(
        {
          name: "Check",
          identifier: "dev.quickgui.template-check",
          entry: "cmd/check",
          native: { libraryPath: core },
        },
        project,
      ),
      mode: "development",
      target,
      executablePath,
      fonts: [],
    });
    if (libraries.length !== (type === "go" ? 1 : 2))
      throw new Error(`${type} bundled unexpected native libraries`);

    const moonConsumers: string[] = [];
    if (type === "moonbit") {
      // A MoonBit application has its own runtime/layout table. Loading another
      // MoonBit image must not interpose either runtime or corrupt live objects.
      const sdk = join(directory, "sdk");
      cpSync(join(root, "moonbit"), sdk, {
        recursive: true,
        filter: (path) =>
          !["_build", ".mooncakes", ".repos", ".git"].includes(path.split(/[\\/]/).at(-1)!),
      });
      const consumer = join(project, "moonbit-demo");
      await run(
        [process.execPath, cli, "init", consumer, "--frontend", "moonbit", "--no-install"],
        directory,
      );
      writeFileSync(
        join(directory, "moon.work"),
        `members = ["sdk", "moonbit project/moonbit-demo"]\n`,
      );
      writeFileSync(
        join(consumer, "main/moon.pkg"),
        'import { "egoist/quickgui/native", "moonbitlang/core/ref" }\npkgtype(kind: "executable")\n',
      );
      writeFileSync(
        join(consumer, "main/main.mbt"),
        `
fn main {
  let replies = @ref.Ref(0)
  // This nested object stays live across calls into the independent runtime.
  let expected : Json = { "text": "MoonBit → provider", "nested": [null, { "ok": true }] }
  match @native.run(() => {
    for _ in 0..<32 {
      @native.invoke("extension/${name}/echo", expected, result => {
        match result {
          Ok(value) => if value != expected { abort("Changed extension reply") }
          Err(error) => abort(error)
        }
        replies.val += 1
        if replies.val == 32 { @native.quit() }
      })
    }
  }, options={ "quitMode": "explicit" }) {
    Ok(_) => ()
    Err(error) => abort(error)
  }
  if replies.val != 32 { abort("Missing extension replies") }
}
`,
      );
      for (const mode of ["development", "production"] as const) {
        const app = join(project, `Moon-${mode}.app`);
        const executablePath =
          process.platform === "darwin"
            ? join(app, "Contents/MacOS/Check")
            : join(app, process.platform === "win32" ? "Check.exe" : "Check");
        mkdirSync(dirname(executablePath), { recursive: true });
        await compileNativeApplication({
          config: resolveConfig(
            {
              name: "MoonBit Extension Check",
              identifier: "dev.quickgui.moonbit-extension-check",
              frontend: "moonbit",
              entry: "main",
              native: { libraryPath: core, extensions: [project] },
            },
            consumer,
          ),
          mode,
          target,
          executablePath,
          fonts: [],
        });
        moonConsumers.push(executablePath);
      }
    }
    rmSync(join(project, "node_modules"), { recursive: true, force: true });
    rmSync(join(project, "artifacts"), { recursive: true, force: true });
    rmSync(join(project, "native"), { recursive: true, force: true });
    for (const executable of [executablePath, ...moonConsumers]) {
      await run([executable], directory, {
        QUICKGUI_LIBRARY: undefined,
        QUICKGUI_HOST_LIB: undefined,
        QUICKGUI_EXTENSION_DIR: undefined,
      });
    }
    console.log(
      `[templates] ${type}: packed CLI scaffold, formatted sources, compiled demo, isolated bundle loaded through purego${type === "moonbit" ? "; MoonBit debug/release consumers passed" : ""}`,
    );
  }
} finally {
  rmSync(directory, { recursive: true, force: true });
}
