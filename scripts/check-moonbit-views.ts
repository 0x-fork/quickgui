#!/usr/bin/env bun
import { cpSync, existsSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";
import { runMoonbit } from "../packages/cli/src/moonbit-build.ts";

const root = resolve(import.meta.dir, "..");
const temporary = mkdtempSync(join(tmpdir(), "quickgui-compiled-views-"));
const project = join(temporary, "app");
const moonHome = process.env.MOON_HOME ?? join(root, "target/moonbit-toolchain");
const env = {
  ...process.env,
  MOON_HOME: moonHome,
  PATH: `${moonHome}/bin:${process.env.PATH ?? ""}`,
};
try {
  cpSync(join(root, "packages/cli/test-fixtures/moonbit-views"), project, { recursive: true });
  writeFileSync(
    join(temporary, "moon.work"),
    `members = ${JSON.stringify([join(root, "moonbit"), project])}\n`,
  );
  for (const mode of ["development", "production"] as const) {
    if (await runMoonbit(project, ["test", "--deny-warn"], mode, env))
      throw new Error(`Compiled MoonBit view tests failed (${mode})`);
  }
  const input = join(project, "views_test.mbt");
  const original = readFileSync(input, "utf8");
  if (
    original !==
    readFileSync(join(root, "packages/cli/test-fixtures/moonbit-views/views_test.mbt"), "utf8")
  )
    throw new Error("The compiler modified original sources");
  const cache = join(project, ".quickgui/moonbit-sources/sources.json");
  if (!existsSync(cache)) throw new Error("Missing persisted source maps");
  const diagnosticsFile = join(project, "diagnostics_test.mbt");
  const cli = join(root, "packages/cli/src/cli.ts");
  const failedCommand = async (command: string) => {
    const child = Bun.spawn(["bun", cli, command, "--project", project], {
      env: { ...env, NO_COLOR: "1" },
      stdout: "pipe",
      stderr: "pipe",
    });
    const [status, stdout, stderr] = await Promise.all([
      child.exited,
      new Response(child.stdout).text(),
      new Response(child.stderr).text(),
    ]);
    if (status === 0) throw new Error(`Expected ${command} to fail`);
    const output = stdout + stderr;
    if (output.includes("moonbit-sources/project/diagnostics_test.mbt"))
      throw new Error(`Unmapped ${command} diagnostic:\n${output}`);
    return output;
  };
  writeFileSync(
    diagnosticsFile,
    'test "compiler location" {\n  @native.test_tree(fn(root) raise {\n    root.append(@ui.text("你好").width(unknown_width()).native_node())\n  })\n}\n',
  );
  const diagnostic = await failedCommand("check");
  if (
    !diagnostic.includes(`${diagnosticsFile}:3:38`) ||
    !diagnostic.includes('root.append(@ui.text("你好").width(unknown_width()).native_node())')
  )
    throw new Error(`Incorrect compiler location/source excerpt:\n${diagnostic}`);
  writeFileSync(
    diagnosticsFile,
    'test "assertion location" {\n  @native.test_tree(fn(root) raise {\n    let count = @reactive.signal(1)\n    let text = @ui.text("Value: \\{count.get()}").native_node()\n    root.append(text)\n    assert_eq(text.children()[0].text_content(), "intentionally wrong")\n  })\n}\n',
  );
  const failure = await failedCommand("test");
  if (!failure.includes(`${diagnosticsFile}:6`))
    throw new Error(`Incorrect assertion location:\n${failure}`);
  if (process.platform === "darwin") {
    writeFileSync(
      diagnosticsFile,
      'fn crashing_label() -> String {\n  abort("intentional view compiler trace probe")\n}\n\ntest "native panic location" {\n  @native.test_tree(fn(root) raise {\n    root.append(@ui.text(crashing_label()).native_node())\n  })\n}\n',
    );
    const panic = await failedCommand("test");
    if (!panic.includes(`${diagnosticsFile}:2`) || !panic.includes(`${diagnosticsFile}:7`))
      throw new Error(`Incorrect native panic location:\n${panic}`);
  }
  rmSync(diagnosticsFile);
  console.log("Compiled MoonBit view reactivity, ownership, and native debug/release tests passed");
} finally {
  rmSync(temporary, { recursive: true, force: true });
}
