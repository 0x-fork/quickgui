import { afterEach, describe, expect, test } from "bun:test";
import { mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";

import { quickguiSolidPlugin } from "./compiler.ts";

const temporaryDirectories: string[] = [];
const reactiveFixture =
  `import { createMemo, createRoot, createSignal, flush } from "solid-js";\n` +
  `let update = (_value: string) => {};\n` +
  `let read = () => "";\n` +
  `createRoot(() => {\n` +
  `  const [value, setValue] = createSignal("before");\n` +
  `  const memo = createMemo(() => value());\n` +
  `  update = setValue;\n` +
  `  read = memo;\n` +
  `});\n` +
  `flush();\n` +
  `update("after");\n` +
  `flush();\n` +
  `export default read();\n`;

afterEach(() => {
  for (const directory of temporaryDirectories.splice(0)) {
    rmSync(directory, { recursive: true, force: true });
  }
});

describe("Solid compiler", () => {
  test("lowers JSX and erases preserved TypeScript syntax", async () => {
    const directory = mkdtempSync(join(tmpdir(), "quickgui-solid-compiler-"));
    temporaryDirectories.push(directory);
    const entrypoint = join(directory, "app.tsx");
    writeFileSync(
      entrypoint,
      `interface CardProps { label: string }\n` +
        `const Card = (props: CardProps) => <view>{props.label}</view>;\n` +
        `export default <Card label="typed" />;\n`,
    );

    const result = await Bun.build({
      entrypoints: [entrypoint],
      outdir: join(directory, "out"),
      target: "bun",
      external: ["@quickgui/solid"],
      plugins: [quickguiSolidPlugin({ development: false })],
    });

    expect(result.success).toBe(true);
    expect(result.logs).toHaveLength(0);
  });

  test("forces Solid's reactive runtime without requiring browser launch conditions", async () => {
    const directory = mkdtempSync(join(tmpdir(), "quickgui-solid-runtime-"));
    temporaryDirectories.push(directory);
    const entrypoint = join(directory, "runtime.ts");
    writeFileSync(entrypoint, reactiveFixture);

    const result = await Bun.build({
      entrypoints: [entrypoint],
      outdir: join(directory, "out"),
      target: "bun",
      format: "esm",
      conditions: ["node"],
      plugins: [quickguiSolidPlugin({ development: false, projectRoot: process.cwd() })],
    });

    expect(result.success).toBe(true);
    const module = await import(`${pathToFileURL(result.outputs[0]!.path).href}?test=${Date.now()}`);
    expect(module.default).toBe("after");
  });

  test("registers the reactive Solid module in Bun's runtime loader", async () => {
    const directory = mkdtempSync(join(tmpdir(), "quickgui-solid-runtime-plugin-"));
    temporaryDirectories.push(directory);
    const entrypoint = join(directory, "runtime.ts");
    const runner = join(directory, "runner.ts");
    const packageRoot = dirname(fileURLToPath(import.meta.url));
    writeFileSync(entrypoint, reactiveFixture);
    writeFileSync(
      runner,
      `import { plugin } from "bun";\n` +
        `import { quickguiSolidPlugin } from ${JSON.stringify(pathToFileURL(join(packageRoot, "compiler.ts")).href)};\n` +
        `plugin(quickguiSolidPlugin({ development: true, projectRoot: ${JSON.stringify(packageRoot)} }));\n` +
        `const module = await import(${JSON.stringify(pathToFileURL(entrypoint).href)});\n` +
        `console.log(module.default);\n`,
    );

    const child = Bun.spawn(["bun", runner], {
      cwd: process.cwd(),
      stdout: "pipe",
      stderr: "pipe",
    });
    const [status, stdout, stderr] = await Promise.all([
      child.exited,
      new Response(child.stdout).text(),
      new Response(child.stderr).text(),
    ]);

    expect(status).toBe(0);
    expect(stderr).toBe("");
    expect(stdout.trim()).toBe("after");
  });
});
