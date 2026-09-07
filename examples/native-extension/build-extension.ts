import { mkdirSync } from "node:fs";
import { join, resolve } from "node:path";

const root = import.meta.dir;
const platforms: Partial<Record<NodeJS.Platform, string>> = {
  darwin: "darwin",
  linux: "linux",
  win32: "windows",
};
const platform = platforms[process.platform];
if (!platform || !["arm64", "x64"].includes(process.arch))
  throw new Error("Unsupported build host");
export const target = `${platform}-${process.arch}`;
export const filename =
  platform === "darwin"
    ? "libquickgui_acme_echo.dylib"
    : platform === "windows"
      ? "quickgui_acme_echo.dll"
      : "libquickgui_acme_echo.so";

export async function buildExtension(
  destination = join(root, "backend/lib", target),
): Promise<string> {
  mkdirSync(destination, { recursive: true });
  const output = join(destination, filename);
  const compiler = process.env.CC ?? "clang";
  const child = Bun.spawn(
    [
      compiler,
      "-std=c11",
      "-Wall",
      "-Wextra",
      "-Werror",
      "-O2",
      ...(platform === "darwin" ? ["-dynamiclib"] : ["-shared"]),
      ...(platform === "windows" ? [] : ["-fPIC", "-fvisibility=hidden"]),
      "-I",
      resolve(root, "../../include"),
      join(root, "backend/echo.c"),
      "-o",
      output,
    ],
    { stdin: "ignore", stdout: "inherit", stderr: "inherit" },
  );
  if ((await child.exited) !== 0) throw new Error("Extension build failed");
  return output;
}

if (import.meta.main) {
  const output = await buildExtension();
  console.log(`Built ${output}`);
  const command = process.argv[2];
  if (command) {
    if (command !== "dev" && command !== "build") throw new Error("Expected dev or build");
    const child = Bun.spawn(
      [
        "bun",
        resolve(root, "../../packages/cli/src/cli.ts"),
        command,
        "--project",
        root,
        ...process.argv.slice(3),
      ],
      {
        env: { ...process.env, QUICKGUI_EXTENSION_DIR: join(root, "backend/lib", target) },
        stdin: "inherit",
        stdout: "inherit",
        stderr: "inherit",
      },
    );
    process.exit(await child.exited);
  }
}
