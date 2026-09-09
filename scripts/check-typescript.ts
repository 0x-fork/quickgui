#!/usr/bin/env bun
/** Exercise the actual compiled Bun host/worker executable against a staged native library. */
import { mkdirSync, mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { loadConfig } from "../packages/cli/src/config.ts";
import { compileNativeApplication } from "../packages/cli/src/native-build.ts";
import { hostTarget } from "../packages/cli/src/targets.ts";

const project = resolve(import.meta.dir, "../examples/counter-typescript");
const cache = join(project, ".quickgui", "typescript");
mkdirSync(cache, { recursive: true });
const directory = mkdtempSync(join(cache, "smoke-"));
const source = join(directory, "smoke.tsx");
writeFileSync(
  join(directory, "records.json"),
  JSON.stringify(
    Array.from({ length: 1000 }, (_, id) => ({
      id,
      title: `Record ${id}: ` + "retained data ".repeat(40),
    })),
  ),
);
writeFileSync(
  source,
  `
import { app, Window } from "@quickgui/native";
import { createRenderer, Text, View } from "@quickgui/solid";
import { createMemo, createSignal, flush, For, onCleanup } from "solid-js";
import records from "./records.json";
if (Bun.isMainThread) throw new Error("Application code must run in a Bun worker");
if (!process.argv.includes("--quickgui-ffi-smoke")) throw new Error("Application arguments did not reach the Bun worker");
await app.whenReady();
const [count, setCount] = createSignal(0);
let cleaned = 0;
const renderer = createRenderer(() => {
  onCleanup(() => cleaned++);
  const items = records.map(record => ({ ...record, value: createSignal(record.title)[0] }));
  const visible = createMemo(() => items.slice(0, 100));
  return <View><Text>{count()}</Text><For each={visible()}>{item => <Text>{item.value()}</Text>}</For></View>;
});
const first = new Window({ title: "FFI first", width: 640, height: 460, visible: false, focus: false, renderer });
const second = new Window({ title: "FFI second", width: 660, height: 480, visible: false, focus: false, renderer });
const [one, two] = await Promise.all([first.getState(), second.getState()]);
if (one.viewportSize.width !== 640 || two.viewportSize.width !== 660) throw new Error("Native replies routed incorrectly: " + JSON.stringify([one, two]));
await new Promise(resolve => setTimeout(resolve, 10));
setCount(1); flush();
const closed = new Promise(resolve => first.onClose(() => resolve(undefined)));
first.close();
await closed;
if (cleaned !== 1 || second.closed) throw new Error("Window roots are not independent");
setCount(2); flush();
await second.setTitle("FFI still responsive");
await second.getState();
app.on("quit", () => {
  if (cleaned !== 2) throw new Error("Shutdown did not clean up both roots");
  console.log("QUICKGUI_TYPESCRIPT_SMOKE_OK");
});
await app.exit();
`,
);
try {
  const base = await loadConfig(project);
  for (const mode of ["development", "production"] as const) {
    const executablePath = join(directory, mode, "Smoke.app", "Contents", "MacOS", "Smoke");
    mkdirSync(dirname(executablePath), { recursive: true });
    await compileNativeApplication({
      config: { ...base, entry: source },
      mode,
      target: hostTarget(),
      executablePath,
      fonts: [],
    });
    const child = Bun.spawn([executablePath, "--quickgui-ffi-smoke"], {
      cwd: directory,
      stdin: "ignore",
      stdout: "pipe",
      stderr: "pipe",
    });
    const timeout = setTimeout(() => child.kill(), 20000);
    const [status, stdout, stderr] = await Promise.all([
      child.exited,
      new Response(child.stdout).text(),
      new Response(child.stderr).text(),
    ]);
    clearTimeout(timeout);
    if (status !== 0 || !stdout.includes("QUICKGUI_TYPESCRIPT_SMOKE_OK"))
      throw new Error(`${mode} native smoke failed (${status})\n${stdout}\n${stderr}`);
    console.log(
      `TypeScript ${mode}: native readiness, requests, reactive updates, independent windows, and shutdown passed`,
    );
  }
} finally {
  rmSync(directory, { recursive: true, force: true });
}
