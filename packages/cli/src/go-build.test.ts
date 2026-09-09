import { expect, test } from "bun:test";
import { mkdtempSync, readFileSync, rmSync, statSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";
import { prepareGoWorkspace, runGo } from "./go-build.ts";

test("Go component overlays preserve authored files, reuse unchanged output, and retain runtime bindings", async () => {
  const project = resolve(import.meta.dir, "../test-fixtures/go-views");
  const file = resolve(project, "cart.go");
  const source = readFileSync(file, "utf8");
  const overlay = (await prepareGoWorkspace(project, ["./..."], { tests: true }))!;
  const manifest = JSON.parse(readFileSync(overlay, "utf8")) as { Replace: Record<string, string> };
  expect(manifest.Replace[file]).toBeDefined();
  const generated = manifest.Replace[file]!;
  const timestamp = statSync(generated).mtimeMs;
  const development = (await prepareGoWorkspace(project, ["."], { tests: false }))!;
  expect(development).not.toBe(overlay);
  for (const path of Object.values(manifest.Replace)) expect(statSync(path).isFile()).toBe(true);
  expect(await runGo(project, "test")).toBe(0);
  expect(await runGo(project, "test", true)).toBe(0);
  expect(readFileSync(file, "utf8")).toBe(source);
  expect(statSync(generated).mtimeMs).toBe(timestamp);
}, 30_000);

test("Go application compilation rejects implicit construction at the authored location", async () => {
  const project = mkdtempSync(join(tmpdir(), "quickgui-explicit-nodes-"));
  try {
    const fixture = resolve(import.meta.dir, "../test-fixtures/go-views");
    const module = readFileSync(join(fixture, "go.mod"), "utf8")
      .replace("example.test/quickgui-views", "example.test/explicit-nodes")
      .replace("../../../../go", resolve(import.meta.dir, "../../../go"));
    writeFileSync(join(project, "go.mod"), module);
    writeFileSync(join(project, "go.sum"), readFileSync(join(fixture, "go.sum")));
    const path = join(project, "view.go");
    writeFileSync(
      path,
      `package example\nimport "github.com/egoist/quickgui/go/ui"\nfunc View() *ui.Element {\n  return ui.Button("returned")\n}\n`,
    );
    await prepareGoWorkspace(project, ["."]);
    for (const body of [
      `ui.Button("discarded")`,
      `ui.View(func() { ui.Text("implicit") })`,
      `ui.Show(false, func() {})`,
      `ui.For[int](func() []int { return []int{1} }, func(int, func() int) {}, nil, nil)`,
    ]) {
      const source = `package example\nimport "github.com/egoist/quickgui/go/ui"\nfunc View() {\n  ${body}\n}\n`;
      writeFileSync(path, source);
      await expect(prepareGoWorkspace(project, ["."])).rejects.toThrow(`${path}:4:`);
      expect(readFileSync(path, "utf8")).toBe(source);
    }
    writeFileSync(
      path,
      `package example\nimport "github.com/egoist/quickgui/go/native"\nfunc Open() {\n  native.NewWindow(native.WindowOptions{Component: func() {}})\n}\n`,
    );
    await expect(prepareGoWorkspace(project, ["."])).rejects.toThrow(`${path}:4:`);
  } finally {
    rmSync(project, { recursive: true, force: true });
  }
}, 30_000);
