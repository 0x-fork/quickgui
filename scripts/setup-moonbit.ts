#!/usr/bin/env bun
/** Install official MoonBit tooling locally, without changing shell profiles or HOME. */
import { chmodSync, existsSync, mkdirSync, mkdtempSync, readdirSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";

const root = resolve(import.meta.dir, "..");
const destination = join(root, "target/moonbit-toolchain");
const platform =
  process.platform === "darwin" ? "darwin" : process.platform === "linux" ? "linux" : undefined;
const arch = process.arch === "arm64" ? "aarch64" : process.arch === "x64" ? "x86_64" : undefined;
if (!platform || !arch)
  throw new Error("Install MoonBit from https://www.moonbitlang.com/download/ on this platform");
const env = {
  ...process.env,
  MOON_HOME: destination,
  PATH: `${destination}/bin:${process.env.PATH ?? ""}`,
};
async function run(argv: string[]) {
  const child = Bun.spawn(argv, { env, stdin: "ignore", stdout: "inherit", stderr: "inherit" });
  if ((await child.exited) !== 0) throw new Error(`Failed: ${argv.join(" ")}`);
}
if (!existsSync(join(destination, "bin/moon"))) {
  const temporary = mkdtempSync(join(tmpdir(), "quickgui-moonbit-toolchain-"));
  const version = encodeURIComponent(process.env.QUICKGUI_MOONBIT_VERSION ?? "latest");
  try {
    mkdirSync(destination, { recursive: true });
    for (const [url, name, output] of [
      [
        `https://cli.moonbitlang.com/binaries/${version}/moonbit-${platform}-${arch}.tar.gz`,
        "toolchain.tar.gz",
        destination,
      ],
      [
        `https://cli.moonbitlang.com/cores/core-${version}.tar.gz`,
        "core.tar.gz",
        join(destination, "lib"),
      ],
    ]) {
      console.log(`[moonbit] Downloading ${url}`);
      const response = await fetch(url!, { signal: AbortSignal.timeout(120_000) });
      if (!response.ok) throw new Error(`MoonBit download failed: ${response.status} ${url}`);
      const archive = join(temporary, name!);
      await Bun.write(archive, response);
      mkdirSync(output!, { recursive: true });
      await run(["tar", "-xzf", archive, "-C", output!]);
    }
    for (const entry of readdirSync(join(destination, "bin"), { withFileTypes: true })) {
      if (entry.isFile()) chmodSync(join(destination, "bin", entry.name), 0o755);
    }
    chmodSync(join(destination, "bin/internal/tcc"), 0o755);
  } finally {
    rmSync(temporary, { recursive: true, force: true });
  }
}
await run([
  join(destination, "bin/moon"),
  "-C",
  join(destination, "lib/core"),
  "bundle",
  "--target",
  "native",
  "--warn-list",
  "-a",
  "--quiet",
]);
await run([join(destination, "bin/moon"), "version", "--all"]);
console.log(`Set MOON_HOME=${destination} and add ${destination}/bin to PATH.`);
