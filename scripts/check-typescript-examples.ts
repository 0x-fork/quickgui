#!/usr/bin/env bun
/** Real native lifecycle checks for the gallery and Quick Git; no benchmark measurement. */
import { mkdirSync, mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { join, resolve } from "node:path";
import { buildProject } from "../packages/cli/src/build.ts";
import { loadConfig } from "../packages/cli/src/config.ts";
import { hostTarget } from "../packages/cli/src/targets.ts";

if (process.platform !== "darwin")
  throw new Error("Native example acceptance currently requires macOS");
const root = resolve(import.meta.dir, "..");
const output = join(root, "target/typescript-example-checks");
mkdirSync(output, { recursive: true });
const repository = mkdtempSync(join(output, "repository-"));
const gitEnv = { ...process.env, GIT_CONFIG_NOSYSTEM: "1", GIT_CONFIG_GLOBAL: "/dev/null" };
function git(args: string[]) {
  const result = Bun.spawnSync(["git", ...args], {
    cwd: repository,
    env: gitEnv,
    stdout: "pipe",
    stderr: "pipe",
  });
  if (result.exitCode) throw new Error(result.stderr.toString());
}
try {
  git(["init", "-b", "main"]);
  git(["config", "user.name", "QuickGUI test"]);
  git(["config", "user.email", "test@example.invalid"]);
  writeFileSync(join(repository, "README.md"), "Native TypeScript validation\n");
  mkdirSync(join(repository, "files"));
  for (let index = 0; index < 128; index++)
    writeFileSync(
      join(repository, "files", `${String(index).padStart(3, "0")}.txt`),
      "Initial line\n",
    );
  git(["add", "."]);
  git([
    "-c",
    "commit.gpgsign=false",
    "-c",
    "core.hooksPath=/dev/null",
    "commit",
    "-m",
    "Initial fixture",
  ]);
  for (let index = 0; index < 128; index++)
    writeFileSync(
      join(repository, "files", `${String(index).padStart(3, "0")}.txt`),
      "Second line\n",
    );
  git([
    "-c",
    "commit.gpgsign=false",
    "-c",
    "core.hooksPath=/dev/null",
    "commit",
    "-am",
    "Update fixture files",
  ]);
  writeFileSync(join(repository, "README.md"), "Native TypeScript validation\nChanged line\n");
  for (const [name, flag, marker] of [
    ["components-typescript", "--check-components", "QUICKGUI_COMPONENTS_OK 44"],
    ["quick-git-typescript", "--check-quick-git", "QUICKGUI_QUICK_GIT_OK"],
  ] as const) {
    if (process.argv.includes("--quick-git") && name !== "quick-git-typescript") continue;
    const result = await buildProject(await loadConfig(join(root, "examples", name)), {
      mode: "development",
      target: hostTarget(),
      outDir: join(output, name),
    });
    const child = Bun.spawn([result.executablePath, flag], {
      cwd: repository,
      env: {
        ...gitEnv,
        QUICK_GIT_OPEN: repository,
        QUICKGUI_TEST_REPOSITORY: repository,
        QUICKGUI_CHECK_VISIBLE: process.argv.includes("--visible") ? "1" : "0",
      },
      stdout: "pipe",
      stderr: "pipe",
      stdin: "ignore",
    });
    const timeout = setTimeout(() => child.kill(), 60_000);
    const [code, stdout, stderr] = await Promise.all([
      child.exited,
      new Response(child.stdout).text(),
      new Response(child.stderr).text(),
    ]);
    clearTimeout(timeout);
    writeFileSync(join(output, name + ".log"), stdout + "\n" + stderr);
    if (code !== 0 || !stdout.includes(marker))
      throw new Error(`${name} failed (${code})\n${stdout}\n${stderr}`);
    console.log(`${name}: native lifecycle and content checks passed`);
  }
} finally {
  rmSync(repository, { recursive: true, force: true });
}
