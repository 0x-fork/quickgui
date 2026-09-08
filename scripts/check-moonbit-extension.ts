#!/usr/bin/env bun
/** Native provider checks without Go, the frontend SDK, or a staged core library. */
import { existsSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";
import { initExtension } from "../packages/cli/src/init-extension.ts";
import { extensionLibraryName } from "../packages/cli/src/extensions.ts";
import { hostTarget } from "../packages/cli/src/targets.ts";

const root = resolve(import.meta.dir, "..");
const local = join(root, "target/moonbit-toolchain");
const moonHome = process.env.MOON_HOME ?? (existsSync(join(local, "bin/moon")) ? local : undefined);
const env = {
  ...process.env,
  ...(moonHome
    ? {
        MOON_HOME: moonHome,
        PATH: `${join(moonHome, "bin")}${process.platform === "win32" ? ";" : ":"}${process.env.PATH ?? ""}`,
      }
    : {}),
};
async function run(argv: string[], cwd: string) {
  const child = Bun.spawn(argv, { cwd, env, stdin: "ignore", stdout: "pipe", stderr: "pipe" });
  const timer = setTimeout(() => child.kill(), 60_000);
  try {
    const [status, out, err] = await Promise.all([
      child.exited,
      new Response(child.stdout).text(),
      new Response(child.stderr).text(),
    ]);
    if (status !== 0) throw new Error(`${argv.join(" ")}\n${out}\n${err}`);
    return out;
  } finally {
    clearTimeout(timer);
  }
}

const temporary = mkdtempSync(join(tmpdir(), "quickgui-moonbit-providers-"));
try {
  const libraries: string[] = [];
  for (const name of ["moon-smoke-a", "moon-smoke-b"]) {
    const project = join(temporary, name);
    await initExtension({ directory: project, type: "moonbit", name, install: false });
    // Exercise both state and replies allocated by MoonBit, not just echo aliases.
    writeFileSync(
      join(project, "native/extension.mbt"),
      `
let calls : Array[Int] = [0]
fn handle(request : UInt, operation : Bytes, params : Bytes) -> Result[Bytes, String] {
  if operation == b"echo" { Ok(params) }
  else if operation == b"request-id" { Ok(@utf8.encode(request.to_string())) }
  else if operation == b"large-reply" { Ok(Bytes::make(65537, 0)) }
  else if operation == b"counter" {
    calls[0] = calls[0] + 1
    Ok(@utf8.encode(calls[0].to_string()))
  } else { Err("Unknown method") }
}
fn shutdown() -> Unit { calls[0] = 0 }
`,
    );
    await run([process.execPath, "scripts/build.ts"], project);
    const manifest = JSON.parse(readFileSync(join(project, "quickgui.extension.json"), "utf8"));
    libraries.push(
      join(project, "artifacts/lib", hostTarget(), extensionLibraryName(manifest, hostTarget())),
    );
  }
  const executable = join(temporary, process.platform === "win32" ? "smoke.exe" : "smoke");
  const source = join(root, "packages/cli/test-fixtures/moonbit-extension-smoke.c");
  if (process.platform === "win32") {
    await run(
      [
        process.env.CC ?? Bun.which("cl") ?? "clang-cl",
        "/nologo",
        "/std:c11",
        "/utf-8",
        `/I${join(root, "include")}`,
        source,
        `/Fe${executable}`,
      ],
      temporary,
    );
  } else {
    await run(
      [
        process.env.CC ?? "cc",
        "-std=c11",
        "-Wall",
        "-Wextra",
        "-Werror",
        "-pthread",
        "-I",
        join(root, "include"),
        source,
        "-o",
        executable,
        ...(process.platform === "linux" ? ["-ldl"] : []),
      ],
      temporary,
    );
  }
  console.log((await run([executable, ...libraries], temporary)).trim());
} finally {
  rmSync(temporary, { recursive: true, force: true });
}
