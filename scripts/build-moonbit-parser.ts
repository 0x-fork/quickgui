#!/usr/bin/env bun
/** Rebuild the CLI's pinned MoonBit grammar. End users only load its bundled Wasm. */
import { cpSync, mkdirSync, mkdtempSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";

const revision = "5435c307c6cf2ef0d508a99047b06f35a4308444";
const destination = resolve(import.meta.dir, "../packages/cli/src/moonbit/parser");
const temporary = mkdtempSync(join(tmpdir(), "quickgui-moonbit-parser-"));
async function run(argv: string[]) {
  const child = Bun.spawn(argv, { cwd: temporary, stdout: "inherit", stderr: "inherit" });
  if (await child.exited) throw new Error(`Failed: ${argv.join(" ")}`);
}
try {
  await run(["git", "clone", "https://github.com/moonbitlang/tree-sitter-moonbit.git", "grammar"]);
  await run(["git", "-C", "grammar", "checkout", "--detach", revision]);
  mkdirSync(destination, { recursive: true });
  await run([
    "bun",
    "x",
    "tree-sitter-cli@0.26.7",
    "build",
    "--wasm",
    "grammar",
    "-o",
    join(destination, "moonbit.wasm"),
  ]);
  cpSync(join(temporary, "grammar/LICENSE"), join(destination, "LICENSE"));
} finally {
  rmSync(temporary, { recursive: true, force: true });
}
