import { afterAll, expect, test } from "bun:test";
import { mkdtempSync, readFileSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";
import { closeTypeScriptSessions, lowerProject } from "./native-compiler.ts";

afterAll(closeTypeScriptSessions);

test("JSX preserves keyed list identity and adapts a named signal setter to native callback arity", async () => {
  const directory = mkdtempSync(join(tmpdir(), "quickgui-jsx-"));
  const projectRoot = resolve(import.meta.dir, "../test-fixtures/native-ui");
  try {
    await lowerProject({
      projectRoot,
      entry: join(projectRoot, "app.tsx"),
      outDir: join(directory, "lowered"),
      name: "UI fixture",
      version: "1.0.0",
      identifier: "dev.quickgui.ui-fixture",
      fonts: [],
      typeCheck: true,
    });
    const lowered = join(directory, "lowered/src/app/app.ts");
    const source = readFileSync(lowered, "utf8");
    expect(source).toMatch(/onValueChange: \(_p\$0, _p\$1\) => \{ \(setValue\)\(_p\$0\); \}/);
    expect(source).not.toContain('import("@quickgui/native")');
    expect(source).toContain('import("../pkg/native/index.ts")');
    expect(source).toMatch(/__qg_setBool\([^;]+true\)/);
    expect(source).toMatch(/__qg_setExplicitBool\([^;]+true\)/);
    expect(source).not.toMatch(/__qg_set(?:Explicit)?Bool\([^;]+\{ (?:color|opacity):/);
    const fixture = await import(lowered);
    expect(fixture.exercise()).toEqual({ keys: 6, reused: true, value: 7 });
    expect(fixture.styled().tag).toBe(1);
  } finally {
    rmSync(directory, { recursive: true, force: true });
  }
}, 30_000);
