import { expect, test } from "bun:test";
import { readFileSync, statSync } from "node:fs";
import { resolve } from "node:path";
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
  expect(await runGo(project, "test")).toBe(0);
  expect(await runGo(project, "test", true)).toBe(0);
  expect(readFileSync(file, "utf8")).toBe(source);
  expect(statSync(generated).mtimeMs).toBe(timestamp);
}, 30_000);
