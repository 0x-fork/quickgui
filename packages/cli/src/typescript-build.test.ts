import { afterEach, expect, test } from "bun:test";
import { existsSync, mkdirSync, mkdtempSync, readFileSync, rmSync, symlinkSync } from "node:fs";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";
import { parseCliArgs } from "./args.ts";
import { resolveConfig, loadConfig } from "./config.ts";
import { initProject } from "./init.ts";
import { quickguiSolidPlugin } from "./typescript-compiler.ts";

const roots: string[] = [];
afterEach(() => {
  for (const root of roots.splice(0)) rmSync(root, { recursive: true, force: true });
});

test("TypeScript is explicit, defaults to app.tsx, and rejects incompatible native options", () => {
  expect(parseCliArgs(["init", "demo", "--frontend", "typescript", "--no-install"])).toMatchObject({
    frontend: "typescript",
    install: false,
  });
  const input = { name: "Demo", identifier: "com.example.demo", frontend: "typescript" as const };
  expect(resolveConfig(input, "/example").entry).toBe("/example/app.tsx");
  expect(() => resolveConfig({ ...input, native: { tags: ["test"] } }, "/example")).toThrow(
    "not supported",
  );
  expect(
    resolveConfig(
      { ...input, native: { extensions: ["terminal", "@acme/native-provider", "./provider"] } },
      "/example",
    ).native.extensions,
  ).toEqual(["terminal", "@acme/native-provider", "/example/provider"]);
  expect(resolveConfig({ name: "Demo", identifier: "com.example.demo" }, "/example").frontend).toBe(
    "go",
  );
});

test("TypeScript scaffold pins Solid 2 and configures JSX and native window ownership", async () => {
  const root = mkdtempSync(join(tmpdir(), "quickgui-typescript-"));
  roots.push(root);
  const project = join(root, "app");
  await initProject({
    directory: project,
    frontend: "typescript",
    install: false,
    name: 'Quoted "App"',
  });
  mkdirSync(join(project, "node_modules/@quickgui"), { recursive: true });
  symlinkSync(resolve(import.meta.dir, ".."), join(project, "node_modules/@quickgui/cli"), "dir");
  const config = await loadConfig(project);
  expect(config.frontend).toBe("typescript");
  expect(config.name).toBe('Quoted "App"');
  expect(existsSync(join(project, "go.mod"))).toBe(false);
  const manifest = JSON.parse(readFileSync(join(project, "package.json"), "utf8"));
  expect(manifest.dependencies["solid-js"]).toBe("2.0.0-rc.7");
  expect(manifest.dependencies["@quickgui/solid"]).toBeTruthy();
  const source = readFileSync(join(project, "app.tsx"), "utf8");
  expect(source).toContain("await app.whenReady()");
  expect(source).toContain('app.on("reopen"');
  expect(
    JSON.parse(readFileSync(join(project, "tsconfig.json"), "utf8")).compilerOptions
      .jsxImportSource,
  ).toBe("@quickgui/solid");
});

test("the packaged JSX compiler resolves the reactive Solid client build", async () => {
  const root = resolve(import.meta.dir, "../../solid");
  const result = await Bun.build({
    entrypoints: [join(root, "test/renderer.test.tsx")],
    target: "bun",
    plugins: [quickguiSolidPlugin({ projectRoot: root, development: false })],
  });
  expect(result.success).toBe(true);
  const code = await result.outputs[0]!.text();
  expect(code).not.toContain("dist/server.js");
  expect(code).toContain("dist/solid.js");
  expect(code).not.toContain("react/jsx-runtime");
});
