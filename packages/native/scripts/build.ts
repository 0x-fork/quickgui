import {
  chmodSync,
  existsSync,
  mkdirSync,
  mkdtempSync,
  realpathSync,
  rmSync,
  symlinkSync,
  writeFileSync,
} from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join, resolve } from "node:path";

const packageRoot = resolve(import.meta.dir, "..");
const debug = process.argv.includes("--debug");
const forwardedArguments = process.argv.slice(2).filter((argument) => argument !== "--debug");
const environment = { ...process.env };
const cleanup: string[] = [];

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

const zig = findGhosttyZig();
if (process.platform === "darwin") {
  prepareMacOSGhosttyBuild(zig);
} else {
  environment.PATH = `${dirname(zig)}:${environment.PATH ?? ""}`;
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
  ...forwardedArguments,
];
let exitCode = 1;
try {
  const build = Bun.spawn(arguments_, {
    cwd: packageRoot,
    env: environment,
    stdin: "inherit",
    stdout: "inherit",
    stderr: "inherit",
  });
  exitCode = await build.exited;
} finally {
  for (const path of cleanup) rmSync(path, { force: true, recursive: true });
}

process.exit(exitCode);

function findGhosttyZig(): string {
  const candidates: string[] = [];
  if (environment.QUICKGUI_ZIG) candidates.push(environment.QUICKGUI_ZIG);

  const current = commandOutput(["sh", "-c", "command -v zig"]);
  if (current) candidates.push(current);

  const miseRoot = commandOutput(["mise", "where", "zig@0.15.2"], tmpdir());
  if (miseRoot) {
    candidates.push(join(miseRoot, "zig"));
    candidates.push(join(miseRoot, "bin", "zig"));
  }

  for (const candidate of candidates) {
    if (!existsSync(candidate)) continue;
    const path = realpathSync(candidate);
    if (commandOutput([path, "version"]) === "0.15.2") return path;
  }
  throw new Error(
    "@quickgui/native requires Zig 0.15.2 to build libghostty-vt. " +
      "Install it with `mise install zig@0.15.2` or set QUICKGUI_ZIG.",
  );
}

function prepareMacOSGhosttyBuild(zig: string): void {
  environment.MACOSX_DEPLOYMENT_TARGET ??= "13.0";
  const sdk = commandOutput(
    ["xcrun", "--sdk", "macosx", "--show-sdk-path"],
    packageRoot,
    environment,
  );
  if (!sdk) throw new Error("could not locate the macOS SDK for libghostty-vt");

  const zigRoot = dirname(zig);
  const bundledLibSystem = [
    join(zigRoot, "lib", "libc", "darwin", "libSystem.tbd"),
    join(dirname(zigRoot), "lib", "libc", "darwin", "libSystem.tbd"),
  ].find(existsSync);
  if (!bundledLibSystem) {
    throw new Error(`could not locate Zig's bundled macOS libSystem.tbd next to ${zig}`);
  }

  // Zig 0.15.2 cannot parse the newer libSystem.tbd shipped by current Xcode SDKs. Keep the real
  // headers/frameworks, but let only the Zig subprocess see its compatible bundled libSystem.
  const scratch = mkdtempSync(join(tmpdir(), "quickgui-ghostty-"));
  cleanup.push(scratch);
  const overlay = join(scratch, "MacOSX.sdk");
  mkdirSync(join(overlay, "usr", "lib"), { recursive: true });
  mkdirSync(join(overlay, "System", "Library"), { recursive: true });
  symlinkSync(join(sdk, "usr", "include"), join(overlay, "usr", "include"));
  symlinkSync(bundledLibSystem, join(overlay, "usr", "lib", "libSystem.tbd"));
  symlinkSync(
    join(sdk, "System", "Library", "Frameworks"),
    join(overlay, "System", "Library", "Frameworks"),
  );
  for (const settings of ["SDKSettings.json", "SDKSettings.plist"]) {
    const source = join(sdk, settings);
    if (existsSync(source)) symlinkSync(source, join(overlay, settings));
  }

  const xcrunDirectory = join(scratch, "xcrun-bin");
  const zigDirectory = join(scratch, "zig-bin");
  mkdirSync(xcrunDirectory);
  mkdirSync(zigDirectory);
  const xcrun = join(xcrunDirectory, "xcrun");
  writeFileSync(
    xcrun,
    `#!/bin/sh\nif [ "$1" = "--sdk" ] && [ "$3" = "--show-sdk-path" ]; then\n  echo ${shellQuote(overlay)}\n  exit 0\nfi\nexec /usr/bin/xcrun "$@"\n`,
  );
  chmodSync(xcrun, 0o755);
  const wrapper = join(zigDirectory, "zig");
  writeFileSync(
    wrapper,
    `#!/bin/sh\nPATH=${shellQuote(xcrunDirectory)}:"$PATH" exec ${shellQuote(zig)} "$@"\n`,
  );
  chmodSync(wrapper, 0o755);

  environment.PATH = `${zigDirectory}:${environment.PATH ?? ""}`;
  environment.ZIG_GLOBAL_CACHE_DIR ??= resolve(
    packageRoot,
    "..",
    "..",
    "target",
    "zig-global-cache-0.15.2",
  );
  console.info(`[@quickgui/native] using Zig 0.15.2 at ${zig}`);
}

function commandOutput(
  command: string[],
  cwd = packageRoot,
  env: Record<string, string | undefined> = process.env,
): string | undefined {
  const result = Bun.spawnSync(command, { cwd, env, stderr: "pipe", stdout: "pipe" });
  if (result.exitCode !== 0) return undefined;
  return result.stdout.toString().trim() || undefined;
}

function shellQuote(value: string): string {
  return `'${value.replaceAll("'", `'"'"'`)}'`;
}
