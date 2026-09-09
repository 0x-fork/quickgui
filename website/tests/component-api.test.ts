import { getDemoSource } from "../src/lib/demo-source.server";
import { rustFunction } from "../scripts/build-demo-sources";
import { expect, test } from "bun:test";
import { existsSync, readFileSync } from "node:fs";
import { resolve } from "node:path";
import { ALL_COMPONENT_DOCS } from "../src/lib/component-docs";
import { getComponentApi } from "../src/lib/component-api.server";
import { DEMO_COMPONENTS } from "../src/lib/component-demos";

const root = resolve(import.meta.dir, "../..");
test("each frontend reference uses real declarations, examples, and source targets", () => {
  const sourceLengths = new Map<string, number>();
  for (const component of ALL_COMPONENT_DOCS)
    for (const frontend of ["go", "moonbit", "typescript"] as const) {
      const api = getComponentApi(frontend, component.kind, component.slug);
      expect(api.example.length).toBeGreaterThan(0);
      expect(api.sections.length).toBeGreaterThan(0);
      for (const section of api.sections) {
        expect(section.signature).not.toContain("undefined");
        for (const entry of [section, ...section.entries]) {
          const [file, line] = entry.source.split("#L");
          if (!sourceLengths.has(file)) {
            expect(existsSync(resolve(root, file))).toBe(true);
            sourceLengths.set(file, readFileSync(resolve(root, file), "utf8").split("\n").length);
          }
          expect(Number(line)).toBeGreaterThan(0);
          expect(sourceLengths.get(file)!).toBeGreaterThanOrEqual(Number(line));
        }
        expect(new Set(section.entries.map((entry) => entry.name)).size).toBe(
          section.entries.length,
        );
      }
    }
});

test("compound props preserve callback types, inherited fields, and namespaces", () => {
  const slider = getComponentApi("go", "ui", "slider").sections.find(
    (section) => section.name === "Slider.Root",
  )!;
  expect(slider.signature).toStartWith("ui.Slider.Root(");
  expect(slider.entries.find((entry) => entry.name === "OnValueChange")?.type).toBe(
    "func([]float64, *native.Event)",
  );
  expect(slider.entries.some((entry) => entry.name === "Disabled")).toBe(true);
  const terminal = getComponentApi("go", "ui", "terminal").sections[0];
  expect(terminal.signature).toStartWith("terminal.View(");
  expect(terminal.entries.some((entry) => entry.name === "Program")).toBe(true);
});

test("MoonBit keeps its own fluent types and declaration defaults", () => {
  const checkbox = getComponentApi("moonbit", "ui", "checkbox").sections[0];
  const checked = checkbox.entries.find((entry) => entry.name === "checked")!;
  expect(checked.type).toContain("Bool");
  expect(checked.default).toBe("false");
  expect(checked.binding).toContain("() -> Bool");
  expect(checkbox.entries.some((entry) => entry.name === "default_checked")).toBe(false);
});

test("the browser catalog matches the Rust demo dispatcher", () => {
  const source = readFileSync(resolve(root, "crates/quickgui-docs-demo/src/lib.rs"), "utf8");
  const declared = [
    ...source.match(/pub const COMPONENTS:[\s\S]*?= &\[([\s\S]*?)\];/)![1].matchAll(/"([a-z-]+)"/g),
  ].map((match) => match[1]);
  expect(DEMO_COMPONENTS).toEqual(declared);
  expect(new Set(declared).size).toBe(declared.length);
  for (const name of declared)
    expect(
      ALL_COMPONENT_DOCS.some((component) => component.kind === "ui" && component.slug === name),
    ).toBe(true);
  expect(declared).not.toContain("terminal");
  expect(declared).not.toContain("system-popover");
});

test("preview code is extracted from the functions compiled for that demo", () => {
  for (const component of DEMO_COMPONENTS) {
    const demo = getDemoSource(component)!;
    expect(demo).toBeDefined();
    const source = readFileSync(resolve(root, demo.path), "utf8");
    for (const fn of demo.functions) expect(demo.code).toContain(rustFunction(source, fn));
    expect(demo.html).toContain('class="shiki ');
    expect(demo.html).toContain("--shiki-light:");
  }
  const button = getDemoSource("button")!;
  expect(button.code).toContain("view.count += 1");
  expect(button.code).toContain("fn action(");
});

test("the MoonBit container uses the same View name as the Go API", () => {
  expect(getComponentApi("moonbit", "ui", "view").sections[0].signature).toStartWith("@ui.view(");
});
