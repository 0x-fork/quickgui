#!/usr/bin/env bun
import { existsSync } from "node:fs";
import { join, resolve } from "node:path";
import { moonbitBuildEnvironment, runMoonbit } from "../../packages/cli/src/moonbit-build.ts";
import { resolveHostLibrary } from "../../packages/cli/src/native-build.ts";
import { hostTarget } from "../../packages/cli/src/targets.ts";

const project = import.meta.dir;
const repository = resolve(project, "../..");
const local = join(repository, "target/moonbit-toolchain");
const moonHome = process.env.MOON_HOME ?? (existsSync(join(local, "bin/moon")) ? local : undefined);
for (const arg of process.argv.slice(2)) {
  if (arg !== "--native") throw new Error(`Unknown argument: ${arg}`);
}
for (const mode of ["development", "production"] as const) {
  const env = {
    ...moonbitBuildEnvironment(mode, hostTarget()),
    ...(moonHome ? { MOON_HOME: moonHome, PATH: `${moonHome}/bin:${process.env.PATH ?? ""}` } : {}),
    ...(process.argv.includes("--native")
      ? { QUICKGUI_LIBRARY: resolveHostLibrary(hostTarget(), repository) }
      : {}),
  };
  for (const args of [
    ["test", "--deny-warn"],
    ...(process.argv.includes("--native") ? [["run", "tests/smoke"]] : []),
  ]) {
    if (await runMoonbit(project, args, mode, env, 120_000))
      throw new Error(`Failed: quickgui ${args.join(" ")}`);
  }
}
console.log("MoonBit components gallery checks passed");
