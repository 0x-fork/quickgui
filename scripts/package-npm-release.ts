#!/usr/bin/env bun
import { createHash } from "node:crypto";
import { existsSync, mkdirSync, readFileSync, readdirSync, rmSync, writeFileSync } from "node:fs";
import { join, resolve } from "node:path";

const root = resolve(import.meta.dir, "..");
const output = resolve(root, process.argv[2] ?? "target/npm-release");
const version = JSON.parse(readFileSync(join(root, "package.json"), "utf8")).version as string;

function run(argv: string[], cwd = root): string {
  const child = Bun.spawnSync(argv, { cwd, stdin: "ignore", stdout: "pipe", stderr: "pipe" });
  if (child.exitCode !== 0) throw new Error(`${argv.join(" ")}: ${child.stderr.toString()}`);
  return child.stdout.toString();
}

run(["bun", "scripts/release-metadata.ts"]);
mkdirSync(output, { recursive: true });
for (const file of readdirSync(output)) {
  if (/^quickgui-.*\.tgz$/.test(file) || file === "NPM_SHA256SUMS") rmSync(join(output, file));
}

const packages = [
  { name: "native", library: "quickgui_host" },
  { name: "native-terminal", library: "quickgui_terminal" },
  { name: "cli", library: undefined },
];
const archives: Record<string, string> = {};
const checksums: string[] = [];
for (const pkg of packages) {
  const directory = join(root, "packages", pkg.name);
  const expected = pkg.library
    ? ["arm64", "x64"].map((arch) => `package/lib/darwin-${arch}/lib${pkg.library}.dylib`)
    : [];
  for (const entry of expected) {
    const binary = join(directory, entry.slice("package/".length));
    if (!existsSync(binary)) throw new Error(`Missing native binary: ${binary}`);
    if (process.platform === "darwin")
      run(["lipo", binary, "-verify_arch", entry.includes("arm64") ? "arm64" : "x86_64"]);
  }
  run(["bun", "pm", "pack", "--destination", output, "--quiet"], directory);
  const filename = `quickgui-${pkg.name}-${version}.tgz`;
  const archive = join(output, filename);
  const manifest = JSON.parse(run(["tar", "-xOf", archive, "package/package.json"]));
  if (
    manifest.name !== `@quickgui/${pkg.name}` ||
    manifest.version !== version ||
    JSON.stringify(manifest.os) !== '["darwin"]' ||
    manifest.publishConfig?.access !== "public"
  ) {
    throw new Error(`Incorrect release metadata in ${filename}`);
  }
  if (
    pkg.name === "cli" &&
    (manifest.dependencies?.["@quickgui/native"] !== version ||
      manifest.dependencies?.["@quickgui/native-terminal"] ||
      manifest.bin?.quickgui !== "src/cli.ts")
  ) {
    throw new Error("CLI must depend only on the core native package; extensions are optional");
  }
  const entries = run(["tar", "-tzf", archive]).trim().split("\n");
  const binaries = entries.filter((entry) => /\.(dylib|dll|so)$/.test(entry));
  if (binaries.length !== expected.length || expected.some((entry) => !binaries.includes(entry))) {
    throw new Error(`Incorrect native library set in ${filename}: ${binaries.join(", ")}`);
  }
  if (pkg.name === "cli" && !entries.includes("package/src/extensions.ts"))
    throw new Error("CLI archive is missing the extension resolver");
  checksums.push(
    `${createHash("sha256").update(readFileSync(archive)).digest("hex")}  ${filename}`,
  );
  archives[pkg.name] = filename;
}
writeFileSync(join(output, "NPM_SHA256SUMS"), checksums.join("\n") + "\n");
console.log(
  `QUICKGUI_NPM_PACKAGE_RESULT ${JSON.stringify({ version, ...archives, checksums: "NPM_SHA256SUMS", passed: true })}`,
);
