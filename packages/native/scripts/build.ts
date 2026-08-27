import { existsSync } from "node:fs";
import { resolve } from "node:path";

const packageRoot = resolve(import.meta.dir, "..");
const debug = process.argv.includes("--debug");
const environment = { ...process.env };

if (process.platform === "darwin" && environment.QUICKGUI_ALLOW_BETA_XCODE !== "1") {
  const selected = Bun.spawnSync(["xcode-select", "-p"], {
    stderr: "pipe",
    stdout: "pipe",
  }).stdout.toString().trim();
  const stableDeveloper = "/Applications/Xcode.app/Contents/Developer";
  const stableClang = resolve(
    stableDeveloper,
    "Toolchains/XcodeDefault.xctoolchain/usr/bin/clang",
  );

  if (selected.toLowerCase().includes("beta") && existsSync(stableClang)) {
    environment.DEVELOPER_DIR = stableDeveloper;
    environment.CARGO_TARGET_AARCH64_APPLE_DARWIN_LINKER = stableClang;
    environment.CARGO_TARGET_X86_64_APPLE_DARWIN_LINKER = stableClang;
    console.info(`[@quickgui/native] using stable Xcode linker at ${stableClang}`);
  }
}

const arguments_ = [
  "napi",
  "build",
  "--platform",
  ...(debug ? [] : ["--release"]),
  "--esm",
  "--js",
  "binding.js",
  "--dts",
  "binding.d.ts",
];
const build = Bun.spawn(arguments_, {
  cwd: packageRoot,
  env: environment,
  stdin: "inherit",
  stdout: "inherit",
  stderr: "inherit",
});

process.exit(await build.exited);
