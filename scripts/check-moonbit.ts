#!/usr/bin/env bun
import { cpSync, existsSync, mkdirSync, mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { basename, dirname, join, resolve } from "node:path";
import { initProject } from "../packages/cli/src/init.ts";
import {
  compileMoonbitApplication,
  moonbitBuildEnvironment,
} from "../packages/cli/src/moonbit-build.ts";
import { loadConfig } from "../packages/cli/src/config.ts";
import { buildProject } from "../packages/cli/src/build.ts";
import { resolveHostLibrary } from "../packages/cli/src/native-build.ts";
import { hostTarget } from "../packages/cli/src/targets.ts";

const root = resolve(import.meta.dir, "..");
const local = join(root, "target/moonbit-toolchain");
const moonHome = process.env.MOON_HOME ?? (existsSync(join(local, "bin/moon")) ? local : undefined);
const env = {
  ...moonbitBuildEnvironment("development", hostTarget()),
  ...(moonHome ? { MOON_HOME: moonHome, PATH: `${moonHome}/bin:${process.env.PATH ?? ""}` } : {}),
};
Object.assign(process.env, env);
if (!Bun.which("moon", { PATH: process.env.PATH ?? "" }))
  throw new Error("Install MoonBit or run bun scripts/setup-moonbit.ts");
for (const arg of process.argv.slice(2))
  if (arg !== "--native") throw new Error(`Unknown argument ${arg}`);
async function run(
  argv: string[],
  cwd = root,
  timeout = 120_000,
  environment: NodeJS.ProcessEnv = env,
) {
  const child = Bun.spawn(argv, {
    cwd,
    env: environment,
    stdin: "ignore",
    stdout: "inherit",
    stderr: "inherit",
  });
  const timer = setTimeout(() => child.kill(), timeout);
  try {
    if ((await child.exited) !== 0) throw new Error(`Failed: ${argv.join(" ")}`);
  } finally {
    clearTimeout(timer);
  }
}
await run(["bun", "scripts/generate-moonbit.ts", "--check"]);
await run(["bun", "scripts/generate-style-helpers.ts", "--check"]);
await run(["bun", "scripts/generate-moonbit-components.ts", "--check"]);
await run(["bun", "scripts/generate-moonbit-view-api.ts", "--check"]);
await run(["bun", "scripts/check-moonbit-views.ts"]);
await run(["bun", "scripts/check-moonbit-docs.ts"]);
await run(["bun", "scripts/check-moonbit-extension.ts"]);
await run(["moon", "fmt", "--check"]);
await run(["bun", "examples/components-moonbit/check.ts", ...process.argv.slice(2)]);
const sdkPackages = [...new Bun.Glob("**/moon.pkg{,.json}").scanSync(join(root, "moonbit"))]
  .filter(
    (file) =>
      !file.split(/[\\/]/).some((part) => ["_build", ".quickgui", ".mooncakes"].includes(part)),
  )
  .map((file) => dirname(join(root, "moonbit", file)));
for (const profile of ["--debug", "--release"]) {
  // Without explicit paths, Moon selects every member of the parent workspace,
  // including application tests that require the QuickGUI view transform.
  await run(["moon", "test", ...sdkPackages, "--target", "native", profile, "--deny-warn"]);
}

// Build the distributed template away from repository workspace/package resolution.
const temporary = mkdtempSync(join(tmpdir(), "quickgui-moonbit-scaffold-"));
try {
  cpSync(join(root, "moonbit"), join(temporary, "sdk"), {
    recursive: true,
    filter: (path) => !["_build", ".mooncakes", ".repos", ".git"].includes(basename(path)),
  });
  const project = join(temporary, "app");
  await initProject({ directory: project, frontend: "moonbit", install: false });
  writeFileSync(join(temporary, "moon.work"), 'members = ["sdk", "app"]\n');
  const config = await loadConfig(project);
  const executablePath =
    process.platform === "darwin"
      ? join(temporary, "Test.app/Contents/MacOS/Test")
      : join(temporary, "app-bin");
  mkdirSync(dirname(executablePath), { recursive: true });
  for (const mode of ["development", "production"] as const) {
    const metadata = await compileMoonbitApplication(
      { config, mode, target: hostTarget(), executablePath, fonts: [] },
      [],
    );
    if (!existsSync(metadata)) throw new Error(`MoonBit ${mode} build metadata is missing`);
  }
  if (process.argv.includes("--native")) {
    // Exercise the actual packaged executable and library lookup, not only `moon run`.
    for (const file of ["main.mbt", "moon.pkg"]) {
      cpSync(join(root, "moonbit/tests/window-smoke", file), join(project, "main", file));
    }
    const build = await buildProject(
      {
        ...config,
        native: { ...config.native, libraryPath: resolveHostLibrary(hostTarget(), root) },
      },
      { mode: "development", target: hostTarget() },
    );
    await run([build.executablePath], project, 60_000, {
      ...env,
      QUICKGUI_LIBRARY: undefined,
      QUICKGUI_HOST_LIB: undefined,
      QUICKGUI_EXTENSION_DIR: undefined,
    });
  }
} finally {
  rmSync(temporary, { recursive: true, force: true });
}

if (process.argv.includes("--native")) {
  process.env.QUICKGUI_LIBRARY = resolveHostLibrary(hostTarget(), root);
  Object.assign(env, { QUICKGUI_LIBRARY: process.env.QUICKGUI_LIBRARY });
  for (const profile of ["--debug", "--release"]) {
    for (const smoke of [
      "abi-smoke",
      "window-smoke",
      ...(process.platform === "darwin" ? ["surfaces-smoke"] : []),
    ]) {
      await run(
        ["moon", "-C", "moonbit", "run", `tests/${smoke}`, "--target", "native", profile],
        root,
        60_000,
      );
    }
  }
  const { buildExtension } = await import("../examples/native-extension/build-extension.ts");
  const extension = await buildExtension();
  Object.assign(env, { QUICKGUI_EXTENSION_DIR: dirname(extension) });
  for (const profile of ["--debug", "--release"]) {
    await run(
      ["moon", "-C", "moonbit", "run", "tests/extension-smoke", "--target", "native", profile],
      root,
      60_000,
    );
  }
}
console.log("MoonBit checks passed");
