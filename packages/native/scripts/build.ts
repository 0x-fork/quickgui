import { existsSync, realpathSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join, resolve } from "node:path";

const packageRoot = resolve(import.meta.dir, "..");
const debug = process.argv.includes("--debug");
const forwardedArguments = process.argv.slice(2).filter((argument) => argument !== "--debug");
const environment = { ...process.env };

const zig = findGhosttyZig();
if (process.platform === "darwin") {
  environment.QUICKGUI_ZIG = zig;
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
const command =
  process.platform === "darwin"
    ? [resolve(packageRoot, "..", "..", "scripts", "with-macos-ghostty-zig.sh"), ...arguments_]
    : arguments_;
const build = Bun.spawn(command, {
  cwd: packageRoot,
  env: environment,
  stdin: "inherit",
  stdout: "inherit",
  stderr: "inherit",
});
process.exit(await build.exited);

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

function commandOutput(
  command: string[],
  cwd = packageRoot,
  env: Record<string, string | undefined> = process.env,
): string | undefined {
  const result = Bun.spawnSync(command, { cwd, env, stderr: "pipe", stdout: "pipe" });
  if (result.exitCode !== 0) return undefined;
  return result.stdout.toString().trim() || undefined;
}
