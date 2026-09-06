import { expect, test } from "bun:test";
import { cpSync, existsSync, mkdirSync, mkdtempSync, rmSync } from "node:fs";
import { join, resolve } from "node:path";

import { buildProject } from "../../../packages/cli/src/build.ts";
import { resolveConfig } from "../../../packages/cli/src/config.ts";
import { hostTarget } from "../../../packages/cli/src/targets.ts";

const exampleRoot = resolve(import.meta.dir, "..");
const hostStaged = existsSync(resolve(exampleRoot, "../../packages/native/lib", hostTarget(), "libquickgui_host.a"));

test.skipIf(process.platform !== "darwin" || !hostStaged || Bun.which("node") === null)(
  "the compiled Quick Git store starts empty and tolerates stale selections and root commits",
  async () => {
    const testDir = join(exampleRoot, ".quickgui", "tests");
    mkdirSync(testDir, { recursive: true });
    const root = mkdtempSync(join(testDir, "store-native-"));
    try {
      for (const directory of ["agent", "git", "model", "modules"]) {
        cpSync(join(exampleRoot, directory), join(root, directory), { recursive: true });
      }
      cpSync(join(exampleRoot, "process.ts"), join(root, "process.ts"));
      mkdirSync(join(root, "testing"));
      cpSync(join(import.meta.dir, "store-native.ts"), join(root, "testing", "store-native.ts"));
      const config = resolveConfig({
        name: "Quick Git Store Test", identifier: "dev.quickgui.quick-git.store-test", entry: "testing/store-native.ts",
      }, root);
      const build = await buildProject(config, { mode: "development", target: hostTarget() });
      const child = Bun.spawn([build.executablePath], { cwd: root, stdin: "ignore", stdout: "pipe", stderr: "pipe" });
      const timeout = setTimeout(() => child.kill(), 20_000);
      try {
        const [status, stdout, stderr] = await Promise.all([
          child.exited, new Response(child.stdout).text(), new Response(child.stderr).text(),
        ]);
        expect({ status, stderr }).toEqual({ status: 0, stderr: "" });
        expect(stdout.trim()).toBe("persistence=ok\nempty-store=ok");
      } finally {
        clearTimeout(timeout);
      }
    } finally {
      rmSync(root, { recursive: true, force: true });
    }
  }, 240_000,
);
