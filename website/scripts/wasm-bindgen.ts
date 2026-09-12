import { chmod, mkdir, mkdtemp, rename, rm } from "node:fs/promises";
import { resolve } from "node:path";

function matchesVersion(binary: string, version: string): boolean {
  try {
    const result = Bun.spawnSync([binary, "--version"], {
      stdout: "pipe",
      stderr: "ignore",
    });
    return result.exitCode === 0 && result.stdout.toString().trim() === `wasm-bindgen ${version}`;
  } catch {
    return false;
  }
}

export async function ensureWasmBindgen(root: string): Promise<string> {
  const manifestPath = resolve(root, "crates/quickgui-docs-demo/Cargo.toml");
  const manifest = Bun.TOML.parse(await Bun.file(manifestPath).text()) as {
    target: Record<string, { dependencies: Record<string, string> }>;
  };
  const requirement = manifest.target['cfg(target_arch = "wasm32")']?.dependencies["wasm-bindgen"];
  const version = requirement?.match(/^=(\d+\.\d+\.\d+)$/)?.[1];
  if (!version) {
    throw new Error(`Expected an exact wasm-bindgen version in ${manifestPath}`);
  }

  const installed = Bun.which("wasm-bindgen", { PATH: process.env.PATH ?? "" });
  if (installed && matchesVersion(installed, version)) return installed;

  const targets: Record<string, string> = {
    "darwin-arm64": "aarch64-apple-darwin",
    "darwin-x64": "x86_64-apple-darwin",
    "linux-arm64": "aarch64-unknown-linux-musl",
    "linux-x64": "x86_64-unknown-linux-musl",
    "win32-x64": "x86_64-pc-windows-msvc",
  };
  const platform = `${process.platform}-${process.arch}`;
  const target = targets[platform];
  if (!target) {
    throw new Error(
      `No wasm-bindgen release binary for ${platform}. Install it with cargo install wasm-bindgen-cli --version ${version} --locked`,
    );
  }

  const release = `wasm-bindgen-${version}-${target}`;
  const cache = resolve(root, "target/tools");
  const directory = resolve(cache, release);
  const executable = process.platform === "win32" ? "wasm-bindgen.exe" : "wasm-bindgen";
  const binary = resolve(directory, executable);
  if (matchesVersion(binary, version)) return binary;

  const url = `https://github.com/wasm-bindgen/wasm-bindgen/releases/download/${version}/${release}.tar.gz`;
  console.log(`Downloading wasm-bindgen ${version} for ${target}...`);
  await mkdir(cache, { recursive: true });
  const temporary = await mkdtemp(resolve(cache, `${release}-`));
  try {
    const response = await fetch(url);
    if (!response.ok) {
      throw new Error(`Failed to download wasm-bindgen: ${response.status} ${response.statusText} (${url})`);
    }
    const archive = resolve(temporary, "release.tar.gz");
    await Bun.write(archive, response);
    const extraction = Bun.spawn(
      ["tar", "-xzf", archive, "--strip-components=1", "-C", temporary],
      { stdout: "inherit", stderr: "inherit" },
    );
    if (await extraction.exited) throw new Error(`Failed to extract ${url}`);

    const downloaded = resolve(temporary, executable);
    if (process.platform !== "win32") await chmod(downloaded, 0o755);
    if (!matchesVersion(downloaded, version)) {
      throw new Error(`Downloaded wasm-bindgen does not report the expected version ${version}`);
    }
    await mkdir(directory, { recursive: true });
    await rename(downloaded, binary);
    return binary;
  } finally {
    await rm(temporary, { recursive: true, force: true });
  }
}
