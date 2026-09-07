/** Build the Rust shared library once and stage it for purego application builds. */

import { cpSync, existsSync, mkdirSync, realpathSync } from "node:fs";
import { join, resolve } from "node:path";

const packageRoot = resolve(import.meta.dir);
const repoRoot = resolve(packageRoot, "..", "..");
const debug = process.argv.includes("--debug");

const architecture = process.arch === "arm64" ? "arm64" : process.arch === "x64" ? "x64" : undefined;
if (architecture === undefined) throw new Error(`Unsupported host architecture: ${process.arch}`);
const platform = process.platform === "darwin" ? "darwin" : process.platform === "linux" ? "linux" : process.platform === "win32" ? "windows" : undefined;
if (platform === undefined) throw new Error(`Unsupported host platform: ${process.platform}`);
const requestedIndex = process.argv.indexOf("--target");
const requested = requestedIndex < 0 ? undefined : process.argv[requestedIndex + 1];
const triples: Record<string, { triple: string; stage: string }> = {
  "aarch64-apple-darwin": { triple: "aarch64-apple-darwin", stage: "darwin-arm64" },
  "x86_64-apple-darwin": { triple: "x86_64-apple-darwin", stage: "darwin-x64" },
  "darwin-arm64": { triple: "aarch64-apple-darwin", stage: "darwin-arm64" },
  "darwin-x64": { triple: "x86_64-apple-darwin", stage: "darwin-x64" },
};
const selected = requested === undefined ? undefined : triples[requested];
if (requestedIndex >= 0 && selected === undefined) throw new Error(`Unsupported Rust host target: ${requested ?? "(missing)"}`);
const target = selected?.stage ?? `${platform}-${architecture}`;

const profile = debug ? "debug" : "release";
const cargo = ["cargo", "build", "-p", "quickgui-host", "--lib", ...(selected === undefined ? [] : ["--target", selected.triple]), ...(debug ? [] : ["--release"])];
const command = platform === "darwin" ? [join(repoRoot, "scripts", "with-macos-ghostty-zig.sh"), ...cargo] : cargo;
console.log(`[native] ${command.join(" ")}`);
const build = Bun.spawnSync(command, { cwd: repoRoot, stdin: "inherit", stdout: "inherit", stderr: "pipe", env: process.env });
const stderr = build.stderr.toString();
process.stderr.write(stderr);
if (build.exitCode !== 0) process.exit(build.exitCode);

const targetDir = process.env.CARGO_TARGET_DIR ? resolve(process.env.CARGO_TARGET_DIR) : join(repoRoot, "target");
const name = platform === "darwin" ? "libquickgui_host.dylib" : platform === "windows" ? "quickgui_host.dll" : "libquickgui_host.so";
const library = join(targetDir, ...(selected === undefined ? [] : [selected.triple]), profile, name);
if (!existsSync(library)) throw new Error(`Expected the shared library at ${library}`);
const stage = join(packageRoot, "lib", target);
mkdirSync(stage, { recursive: true });
cpSync(library, join(stage, name));
console.log(`[native] Staged ${realpathSync(join(stage, name))}`);
