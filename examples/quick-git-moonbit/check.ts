import { existsSync, mkdtempSync, mkdirSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir, devNull } from "node:os";
import { dirname, join, resolve } from "node:path";
import { buildExtension } from "./build-extension.ts";
import { resolveHostLibrary } from "../../packages/cli/src/native-build.ts";
import { moonbitBuildEnvironment, runMoonbit } from "../../packages/cli/src/moonbit-build.ts";
import { hostTarget } from "../../packages/cli/src/targets.ts";

const root = import.meta.dir;
const repository = resolve(root, "../..");
const local = join(repository, "target/moonbit-toolchain");
const moonHome = process.env.MOON_HOME ?? (existsSync(join(local, "bin/moon")) ? local : undefined);
const env = {
  ...moonbitBuildEnvironment("development", hostTarget()),
  GIT_CONFIG_NOSYSTEM: "1",
  GIT_CONFIG_GLOBAL: devNull,
  ...(moonHome ? { MOON_HOME: moonHome, PATH: `${moonHome}/bin:${process.env.PATH ?? ""}` } : {}),
};
async function run(argv: string[], cwd = root, extra: Record<string, string> = {}) {
  const child = Bun.spawn(argv, {
    cwd,
    env: { ...env, ...extra },
    stdin: "ignore",
    stdout: "inherit",
    stderr: "inherit",
  });
  const timeout = setTimeout(() => child.kill(), 120_000);
  try {
    if ((await child.exited) !== 0) throw new Error(`Failed: ${argv.join(" ")}`);
  } finally {
    clearTimeout(timeout);
  }
}
for (const flag of process.argv.slice(2))
  if (flag !== "--native") throw new Error(`Unknown argument: ${flag}`);
if (await runMoonbit(root, ["test", "--deny-warn"], "development", env, 120_000))
  throw new Error("Quick Git tests failed");
await run(["cargo", "test", "--locked", "--manifest-path", "native/Cargo.toml"]);
if (process.argv.includes("--native")) {
  const library = await buildExtension();
  const temporary = mkdtempSync(join(tmpdir(), "quick-git-moonbit-test-"));
  try {
    for (const profile of ["--debug", "--release"]) {
      // The smoke stages files, creates commits, and changes branches/worktrees.
      const fixture = join(temporary, profile.slice(2));
      mkdirSync(fixture);
      await run(["git", "init", "-q", "-b", "main"], fixture);
      await run(["git", "config", "user.name", "QuickGUI Test"], fixture);
      await run(["git", "config", "user.email", "quickgui@example.invalid"], fixture);
      await run(["git", "config", "commit.gpgsign", "false"], fixture);
      writeFileSync(join(fixture, "one.txt"), "one\n");
      writeFileSync(join(fixture, "two.txt"), "two\n");
      await run(["git", "add", "."], fixture);
      await run(["git", "commit", "-qm", "Initial fixture"], fixture);
      writeFileSync(join(fixture, "one.txt"), "one\nsecond\n");
      writeFileSync(join(fixture, "two.txt"), "two\nsecond\n");
      await run(["git", "add", "."], fixture);
      await run(["git", "commit", "-qm", "Change both files"], fixture);
      writeFileSync(join(fixture, "one.txt"), "one edited\nsecond\n");
      writeFileSync(join(fixture, "untracked[1].txt"), "untracked\n");
      const status = await runMoonbit(
        root,
        ["run", "tests/smoke"],
        profile === "--debug" ? "development" : "production",
        {
          ...env,
          QUICKGUI_LIBRARY: resolveHostLibrary(hostTarget(), repository),
          QUICKGUI_EXTENSION_DIR: dirname(library),
          QUICK_GIT_OPEN: fixture,
        },
        120_000,
      );
      if (status) throw new Error(`Quick Git native smoke failed (${profile})`);
    }
  } finally {
    rmSync(temporary, { recursive: true, force: true });
  }
}
console.log("Quick Git MoonBit checks passed");
