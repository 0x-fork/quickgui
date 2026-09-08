import { mkdirSync, copyFileSync } from "node:fs";
import { join, resolve } from "node:path";

const root = import.meta.dir;
const platform = process.platform === "win32" ? "windows" : process.platform;
export const target = `${platform}-${process.arch}`;
export const directory = join(root, "native/lib", target);

export async function buildExtension(): Promise<string> {
  const rustc = Bun.spawn(["rustc", "-vV"], { stdout: "pipe", stderr: "inherit" });
  const version = await new Response(rustc.stdout).text();
  if ((await rustc.exited) !== 0)
    throw new Error("Rust is needed to build Quick Git's I/O extension once");
  const triple = /^host: (.+)$/m.exec(version)?.[1];
  if (!triple) throw new Error("Missing Rust host target");
  const proc = Bun.spawn(["cargo", "build", "--release", "--locked", "--target", triple], {
    cwd: join(root, "native"),
    stdout: "inherit",
    stderr: "inherit",
    env: { ...process.env, CARGO_TARGET_DIR: join(root, "native/target") },
  });
  if ((await proc.exited) !== 0) throw new Error("Quick Git I/O extension build failed");
  const file =
    platform === "darwin"
      ? "libquickgui_quick_git_io.dylib"
      : platform === "windows"
        ? "quickgui_quick_git_io.dll"
        : "libquickgui_quick_git_io.so";
  mkdirSync(directory, { recursive: true });
  const dest = join(directory, file);
  copyFileSync(join(root, "native/target", triple, "release", file), dest);
  return dest;
}

if (import.meta.main) {
  console.log(`I/O extension: ${await buildExtension()}`);
  const action = process.argv[2];
  if (action) {
    if (action !== "dev" && action !== "build") throw new Error("Expected dev or build");
    const child = Bun.spawn(
      [
        "bun",
        resolve(root, "../../packages/cli/src/cli.ts"),
        action,
        "--project",
        root,
        ...process.argv.slice(3),
      ],
      {
        env: { ...process.env, QUICKGUI_EXTENSION_DIR: directory },
        stdin: "inherit",
        stdout: "inherit",
        stderr: "inherit",
      },
    );
    process.exit(await child.exited);
  }
}
