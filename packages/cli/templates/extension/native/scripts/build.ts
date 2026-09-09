import { mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { join, resolve } from "node:path";
import { compileExtension } from "./extension.ts";

export const root = resolve(import.meta.dir, "..");

export interface Manifest {
  name: string;
  version: string;
  package: string;
  library: string;
}

export async function buildExtension(): Promise<string> {
  const manifest = JSON.parse(readFileSync(join(root, "quickgui.extension.json"), "utf8"));
  if (
    manifest.schema !== 1 ||
    manifest.abi !== 1 ||
    typeof manifest.name !== "string" ||
    !/^[a-z][a-z0-9-]{0,63}$/.test(manifest.name) ||
    manifest.name === "host" ||
    manifest.name === "terminal" ||
    typeof manifest.version !== "string" ||
    manifest.version.length > 64 ||
    !/^\d+\.\d+\.\d+(?:-[0-9A-Za-z.-]+)?(?:\+[0-9A-Za-z.-]+)?$/.test(manifest.version) ||
    typeof manifest.package !== "string" ||
    manifest.package.length > 214 ||
    !/^(?:@[a-z0-9][a-z0-9._-]*\/)?[a-z0-9][a-z0-9._-]*$/.test(manifest.package) ||
    manifest.library !== `quickgui_${manifest.name.replaceAll("-", "_")}`
  ) {
    throw new Error("Invalid extension manifest identity, version, library, or ABI");
  }
  const platform = process.platform === "win32" ? "windows" : process.platform;
  if (
    !["darwin", "linux", "windows"].includes(platform) ||
    !["arm64", "x64"].includes(process.arch)
  )
    throw new Error(`Unsupported build host: ${platform}-${process.arch}`);
  const directory = join(root, "artifacts/lib", `${platform}-${process.arch}`);
  const filename =
    platform === "windows"
      ? `${manifest.library}.dll`
      : `lib${manifest.library}.${platform === "darwin" ? "dylib" : "so"}`;
  mkdirSync(directory, { recursive: true });
  await compileExtension(root, manifest, join(directory, filename));

  const packagePath = join(root, "artifacts/package.json");
  const artifact = JSON.parse(readFileSync(packagePath, "utf8"));
  writeFileSync(
    packagePath,
    JSON.stringify({ ...artifact, name: manifest.package, version: manifest.version }, null, 2) +
      "\n",
  );
  console.log(`Built ${join(directory, filename)}`);
  return directory;
}

export async function run(argv: string[], cwd: string, env = process.env): Promise<void> {
  const child = Bun.spawn(argv, {
    cwd,
    env,
    stdin: "inherit",
    stdout: "inherit",
    stderr: "inherit",
  });
  if ((await child.exited) !== 0) throw new Error(`Command failed: ${argv.join(" ")}`);
}

if (import.meta.main) await buildExtension();
