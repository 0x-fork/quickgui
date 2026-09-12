import { expect, mock, test } from "bun:test";
import { createComponent, DEV, flush, OBSERVE } from "solid-js";
import type { NativeNode } from "@quickgui/native";
import { fakeBinding, setEventDispatcher } from "../../packages/native/test/fake-binding.ts";
import { loadSnippets } from "./snippets.ts";

mock.module("../../packages/native/src/binding.ts", () => fakeBinding);
const { app, Window, PropertyCode } = await import("@quickgui/native");
const { createRenderer } = await import("@quickgui/solid");
const { CodeMorphDemo } = await import("./demo.tsx");
setEventDispatcher(() => app.dispatchEvents());
await app.whenReady();
const snippets = await loadSnippets();

function text(node: NativeNode): string {
  return (node.text ?? "") + node.children.map(text).join("");
}

function mount(reduceMotion = false) {
  const host = new Window({
    renderer: createRenderer(
      () => createComponent(CodeMorphDemo, { snippets, reduceMotion }) as NativeNode,
    ),
  });
  flush();
  return host;
}

function click(host: InstanceType<typeof Window>, label: string) {
  const node = [...host.nodes.values()].find(
    (node) => node.properties.get(PropertyCode.AccessibilityLabel) === label,
  );
  expect(node).toBeDefined();
  host._dispatchEvent("click", node!.id);
  flush();
}

function tokens(host: InstanceType<typeof Window>) {
  function walk(node: NativeNode): NativeNode[] {
    return [node, ...node.children.flatMap(walk)];
  }
  return walk(host.root).filter((node) => node.properties.has(PropertyCode.Transform));
}

async function close(host: InstanceType<typeof Window>) {
  const closed = new Promise<void>((resolve) => host.onClose(() => resolve()));
  host.close();
  await closed;
}

test("component setup and language switches do not read reactive state outside tracking scopes", async () => {
  expect(DEV).toBeDefined();
  expect(OBSERVE).toBeDefined();
  const warnings: string[] = [];
  const unsubscribe = OBSERVE!.diagnostics.subscribe((event) => {
    if (event.code === "STRICT_READ_UNTRACKED") warnings.push(event.message);
  });
  let host: InstanceType<typeof Window> | undefined;
  try {
    host = mount(true);
    click(host, "Show TypeScript");
    click(host, "Show Rust");
    click(host, "Show Go");
    expect(warnings).toEqual([]);
  } finally {
    if (host) await close(host);
    unsubscribe();
  }
});

test("native tokens keep their IDs while moving and interrupted exits are eventually removed", async () => {
  const host = mount();
  try {
    const original = tokens(host).find((node) => text(node) === "FormatTemperature")!;
    expect(original).toBeDefined();
    const transform = original.properties.get(PropertyCode.Transform);
    click(host, "Show TypeScript");
    const ts = tokens(host).find((node) => text(node) === "formatTemperature")!;
    expect(ts).toBe(original);
    expect(ts.properties.get(PropertyCode.Transform)).not.toBe(transform);
    expect(ts.properties.get(PropertyCode.Left)).toBe(0);
    expect(ts.properties.get(PropertyCode.Top)).toBe(0);
    expect(ts.properties.get(PropertyCode.TransitionProperties)).toBe("all");
    expect(ts.properties.get(PropertyCode.TransitionDuration)).toBe(550);
    click(host, "Show Rust");
    expect(tokens(host).find((node) => text(node) === "format_temperature")).toBe(original);
    click(host, "Show Go");
    click(host, "Show TypeScript");
    await Bun.sleep(850);
    flush();
    expect(tokens(host).map(text)).toEqual(
      snippets.typescript.tokens
        .filter((token) => /\S/.test(token.content))
        .map((token) => token.content),
    );
    expect(tokens(host).every((node) => node.properties.get(PropertyCode.Opacity) === 1)).toBe(
      true,
    );
  } finally {
    await close(host);
  }
});

test("Reduce Motion swaps directly; closing a window cancels pending entry and exit", async () => {
  const instant = mount(true);
  try {
    click(instant, "Next language");
    expect(tokens(instant).length).toBe(
      snippets.typescript.tokens.filter((token) => /\S/.test(token.content)).length,
    );
    expect(tokens(instant).every((node) => node.properties.get(PropertyCode.Opacity) === 1)).toBe(
      true,
    );
    expect(
      tokens(instant).every((node) => node.properties.get(PropertyCode.TransitionDuration) === 0),
    ).toBe(true);
  } finally {
    await close(instant);
  }
  const animated = mount();
  click(animated, "Next language");
  await close(animated);
  const remaining = animated.nodes.size;
  await Bun.sleep(850);
  flush();
  expect(remaining).toBe(0);
  expect(animated.nodes.size).toBe(remaining);
});
