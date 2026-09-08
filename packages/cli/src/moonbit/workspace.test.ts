import { expect, test } from "bun:test";
import {
  lstatSync,
  mkdirSync,
  mkdtempSync,
  readFileSync,
  rmSync,
  statSync,
  writeFileSync,
} from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import {
  prepareMoonbitWorkspace,
  remapMoonbitDiagnostics,
  renderMoonbitOutput,
} from "./workspace.ts";

test("staging preserves source files, caches unchanged outputs, and removes stale files", async () => {
  const root = mkdtempSync(join(tmpdir(), "quickgui-views-stage-"));
  try {
    writeFileSync(join(root, "moon.mod"), 'name = "test/app"\n');
    writeFileSync(join(root, "moon.pkg"), 'import { "egoist/quickgui/ui" }');
    const original = 'fn view() { @ui.text("Count: \\{count.get()}") }';
    const filename = join(root, "view.mbt");
    writeFileSync(filename, original);
    const first = await prepareMoonbitWorkspace(root);
    const output = first.sources.find((source) => source.original === filename)!.generated;
    expect(first.bindings).toBe(1);
    expect(first.changedFiles).toBe(1);
    expect(readFileSync(filename, "utf8")).toBe(original);
    const mtime = statSync(output).mtimeMs;
    expect((await prepareMoonbitWorkspace(root)).changedFiles).toBe(0);
    expect(statSync(output).mtimeMs).toBe(mtime);
    writeFileSync(filename, original.replace("Count:", "Value:"));
    expect((await prepareMoonbitWorkspace(root)).changedFiles).toBe(1);
    rmSync(filename);
    expect(
      (await prepareMoonbitWorkspace(root)).sources.some((source) => source.original === filename),
    ).toBe(false);
    expect(() => statSync(output)).toThrow();
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
});

test("adding a UI import never writes transformed code through an original-source symlink", async () => {
  const root = mkdtempSync(join(tmpdir(), "quickgui-views-import-"));
  try {
    writeFileSync(join(root, "moon.mod"), 'name = "test/app"\n');
    writeFileSync(join(root, "moon.pkg"), 'import { "other/ui" }');
    const source = "fn view() { @ui.text(value.get()) }";
    writeFileSync(join(root, "view.mbt"), source);
    const first = await prepareMoonbitWorkspace(root);
    expect(lstatSync(join(first.project, "view.mbt")).isSymbolicLink()).toBe(true);
    writeFileSync(join(root, "moon.pkg"), 'import { "egoist/quickgui/ui" }');
    const second = await prepareMoonbitWorkspace(root);
    expect(lstatSync(join(second.project, "view.mbt")).isSymbolicLink()).toBe(false);
    expect(readFileSync(join(root, "view.mbt"), "utf8")).toBe(source);
    expect(second.bindings).toBe(1);
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
});

test("compiler and runtime locations map back to the original file", async () => {
  const root = mkdtempSync(join(tmpdir(), "quickgui-views-locations-"));
  try {
    mkdirSync(join(root, "main"));
    writeFileSync(join(root, "moon.mod"), 'name = "test/app"\n');
    writeFileSync(join(root, "main/moon.pkg"), 'import { "egoist/quickgui/ui" }');
    const filename = join(root, "main/view.mbt");
    writeFileSync(filename, 'fn view() {\n @ui.text("Hello")\n   .width(bad_width())\n}\n');
    const workspace = await prepareMoonbitWorkspace(root);
    const record = workspace.sources.find((source) => source.original === filename)!;
    const code = readFileSync(record.generated, "utf8");
    const prefix = code.slice(0, code.indexOf("bad_width")).split("\n");
    const line = prefix.length,
      column = prefix.at(-1)!.length + 1;
    expect(
      remapMoonbitDiagnostics(
        `Error: ${record.generated}:${line}:${column}`,
        workspace.sources,
        workspace.project,
      ),
    ).toBe(`Error: ${filename}:3:11`);
    expect(
      remapMoonbitDiagnostics(
        `at main/view.mbt:${line}:${column}`,
        workspace.sources,
        workspace.project,
      ),
    ).toBe(`at ${filename}:3:11`);
    const diagnostic = renderMoonbitOutput(
      JSON.stringify({
        $message_type: "diagnostic",
        path: record.generated,
        loc: `${line}:${column}-${line}:${column + 9}`,
        level: "error",
        message: "Unknown function",
        context: "Generated code should not leak",
      }),
      workspace.sources,
      workspace.project,
    );
    expect(diagnostic).toContain(`${filename}:3:11-3:20`);
    expect(diagnostic).toContain("   .width(bad_width())");
    expect(diagnostic).not.toContain("Generated code");
    expect(
      remapMoonbitDiagnostics(
        `${workspace.project}/main/moon.pkg:1:1`,
        workspace.sources,
        workspace.project,
      ),
    ).toBe(`${root}/main/moon.pkg:1:1`);
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
});

test("fluent bindings resolve component return types across files and invalidate that metadata", async () => {
  const root = mkdtempSync(join(tmpdir(), "quickgui-views-functions-"));
  try {
    writeFileSync(join(root, "moon.mod"), 'name = "test/app"\n');
    writeFileSync(join(root, "moon.pkg"), 'import { "egoist/quickgui/ui" }');
    writeFileSync(join(root, "helper.mbt"), "fn card() -> @ui.Element { @ui.div([]) }");
    writeFileSync(join(root, "view.mbt"), "fn view() { card().width(width.get()) }");
    const first = await prepareMoonbitWorkspace(root);
    expect(first.bindings).toBe(1);
    writeFileSync(join(root, "helper.mbt"), "fn card() -> Other { other() }");
    expect((await prepareMoonbitWorkspace(root)).bindings).toBe(0);
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
});

test("unrelated workspace applications do not enter the build or its transform cache", async () => {
  const root = mkdtempSync(join(tmpdir(), "quickgui-views-members-"));
  try {
    for (const name of ["app", "helper", "unrelated"]) {
      mkdirSync(join(root, name));
      writeFileSync(
        join(root, name, "moon.mod"),
        `name = "test/${name}"\n${name === "app" ? 'import { "test/helper@0.1.0" }' : ""}`,
      );
      writeFileSync(join(root, name, "moon.pkg"), 'import { "egoist/quickgui/ui" }');
      writeFileSync(
        join(root, name, "view.mbt"),
        name === "unrelated" ? "fn unfinished() { @ui.text(" : "fn view() { @ui.text(value()) }",
      );
    }
    writeFileSync(join(root, "moon.work"), 'members = ["app", "helper", "unrelated"]');
    const workspace = await prepareMoonbitWorkspace(join(root, "app"));
    expect(workspace.sources.some((source) => source.original.includes("/helper/"))).toBe(true);
    expect(workspace.sources.some((source) => source.original.includes("/unrelated/"))).toBe(false);
    expect(workspace.bindings).toBe(2);
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
});
