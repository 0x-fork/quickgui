import { beforeEach, describe, expect, test } from "bun:test";

import { NativeNodeTag, NodeHost, PropertyCode, ROOT_NODE_ID, createRootNode, type NativeNode } from "@quickgui/native";

import { createRenderEffect, createRoot, createSignal, onCleanup } from "./reactive.ts";
import { For, Index, KeyedFor, Show, dynamic, dynamicText, element, fragment, insert, setColor, setLength, text } from "./runtime.ts";
import { dispatchNativeEvent, EVENT_COMPONENT_CHANGE, EVENT_INPUT } from "@quickgui/native/native-tree";
import { Calendar } from "./date-time.ts";
import { Slider as SwiftSlider } from "./swift-ui.ts";

interface Mutation {
  op: string;
  id?: number;
  tag?: number;
  text?: string;
  property?: number;
  value?: string | number | boolean | null;
  parent?: number;
  child?: number;
  before?: number;
  children?: number[];
}

/** Decode one batch exactly as the host does, so the encoding is checked end to end. */
function decode(bytes: Uint8Array): Mutation[] {
  const view = new DataView(bytes.buffer, bytes.byteOffset, bytes.byteLength);
  expect(String.fromCharCode(bytes[0]!, bytes[1]!, bytes[2]!, bytes[3]!)).toBe("QGMB");
  expect(view.getUint16(4, true)).toBe(31);
  const count = view.getUint32(6, true);
  let offset = 10;
  const decoder = new TextDecoder();
  const u8 = (): number => bytes[offset++]!;
  const u16 = (): number => {
    const value = view.getUint16(offset, true);
    offset += 2;
    return value;
  };
  const u32 = (): number => {
    const value = view.getUint32(offset, true);
    offset += 4;
    return value;
  };
  const f32 = (): number => {
    const value = view.getFloat32(offset, true);
    offset += 4;
    return value;
  };
  const str = (): string => {
    const length = u32();
    const value = decoder.decode(bytes.subarray(offset, offset + length));
    offset += length;
    return value;
  };
  const mutations: Mutation[] = [];
  for (let i = 0; i < count; i++) {
    const op = u8();
    switch (op) {
      case 1:
        mutations.push({ op: "create", id: u32(), tag: u8() });
        break;
      case 2:
        mutations.push({ op: "text", id: u32(), text: str() });
        break;
      case 3:
        mutations.push({ op: "sentinel", id: u32() });
        break;
      case 4: {
        const id = u32();
        const property = u16();
        const kind = u8();
        let value: string | number | boolean | null = null;
        if (kind === 1) value = u8() === 1;
        else if (kind === 2) value = f32();
        else if (kind === 3) value = u32();
        else if (kind === 4) value = str();
        mutations.push({ op: "set", id, property, value });
        break;
      }
      case 5:
        mutations.push({ op: "replace", id: u32(), text: str() });
        break;
      case 6:
        mutations.push({ op: "insert", parent: u32(), child: u32(), before: u32() });
        break;
      case 7:
        mutations.push({ op: "remove", parent: u32(), child: u32() });
        break;
      case 8: {
        const parent = u32();
        const length = u32();
        const children: number[] = [];
        for (let j = 0; j < length; j++) children.push(u32());
        mutations.push({ op: "cleanup", parent, children });
        break;
      }
      default:
        throw new Error(`unknown opcode ${op}`);
    }
  }
  expect(offset).toBe(bytes.byteLength);
  return mutations;
}

const sent: Mutation[][] = [];
(globalThis as Record<string, unknown>).quickguiApplyBatch = (_app: number, _window: number, batch: Uint8Array): number => {
  sent.push(decode(batch));
  return 0;
};
(globalThis as Record<string, unknown>).quickguiFocusNode = (): number => 0;

function makeHost(): { host: NodeHost; root: NativeNode } {
  const host = new NodeHost(1, 1);
  const root = createRootNode(host, ROOT_NODE_ID);
  host.nativeReady = true;
  return { host, root };
}

async function settle(): Promise<void> {
  await new Promise<void>((resolve) => setTimeout(resolve, 0));
}

beforeEach(() => {
  sent.length = 0;
});

describe("detached subtrees", () => {
  test("a subtree built before insertion is committed in creation order", async () => {
    const { host, root } = makeHost();
    const view = element(NativeNodeTag.View);
    setLength(view, PropertyCode.Width, "100%");
    setColor(view, PropertyCode.BackgroundColor, "#102030");
    const label = text("hello");
    insert(view, label);
    expect(sent).toHaveLength(0);
    insert(root, view);
    host.flush();
    expect(sent).toHaveLength(1);
    const ops = sent[0]!.map((mutation) => mutation.op);
    expect(ops).toEqual(["create", "set", "set", "text", "insert", "insert"]);
    expect(sent[0]![1]).toEqual({ op: "set", id: view.id, property: PropertyCode.Width, value: "100%" });
    expect(sent[0]![2]).toMatchObject({ op: "set", id: view.id, property: PropertyCode.BackgroundColor, value: 0xff302010 });
    expect(sent[0]![4]).toEqual({ op: "insert", parent: view.id, child: label.id, before: 0xffff_ffff });
    expect(sent[0]![5]).toEqual({ op: "insert", parent: ROOT_NODE_ID, child: view.id, before: 0xffff_ffff });
    expect(host.nodes.get(label.id)).toBe(label);
    await settle();
  });

  test("mutations on a mounted node schedule one flush per turn", async () => {
    const { root } = makeHost();
    const view = element(NativeNodeTag.View);
    insert(root, view);
    setLength(view, PropertyCode.Width, 10);
    setLength(view, PropertyCode.Height, 20);
    expect(sent).toHaveLength(0);
    await settle();
    expect(sent).toHaveLength(1);
    expect(sent[0]!.map((mutation) => mutation.op)).toEqual(["create", "insert", "set", "set"]);
  });
});

describe("reactive regions", () => {
  test("retained conditional content stays reactive when a truthy condition changes", () => {
    createRoot((dispose) => {
      const { host, root } = makeHost();
      const [condition, setCondition] = createSignal(1);
      const [label, setLabel] = createSignal("first");
      let cleanups = 0;
      insert(root, Show({ when: condition, children: () => {
        onCleanup(() => { cleanups += 1; });
        return dynamicText(label);
      } }));
      const row = root.children[0]!;
      setCondition(2);
      setLabel("second");
      host.flush();
      expect(root.children[0]).toBe(row);
      expect(row.text).toBe("second");
      expect(cleanups).toBe(0);
      setCondition(0);
      expect(cleanups).toBe(1);
      dispose();
      expect(cleanups).toBe(1);
    });
  });

  test("Show removes its content before effects inside it see a falsy condition", () => {
    createRoot((dispose) => {
      const { host, root } = makeHost();
      const [busy, setBusy] = createSignal<{ label: string } | undefined>(undefined);
      // As in a toolbar: the content asserts the condition, and effects created after the Show
      // also read it, which reorders the signal's observers when they re-run.
      insert(root, Show({ when: busy, children: () => dynamicText(() => busy()!.label + "…") }));
      for (let index = 0; index < 4; index++) createRenderEffect(() => { void busy(); });
      setBusy({ label: "Fetch" });
      host.flush();
      expect(root.children[0]!.text).toBe("Fetch…");
      expect(() => setBusy(undefined)).not.toThrow();
      host.flush();
      expect(root.children).toHaveLength(1);
      dispose();
    });
  });

  test("positional rows continue reacting after multiple item replacements", () => {
    createRoot((dispose) => {
      const { host, root } = makeHost();
      const [items, setItems] = createSignal(["one", "two"]);
      insert(root, Index({ each: items, children: (item) => dynamicText(item) }));
      const first = root.children[0]!;
      setItems(["three", "four"]);
      setItems(["five", "six"]);
      host.flush();
      expect(root.children[0]).toBe(first);
      expect(first.text).toBe("five");
      dispose();
    });
  });
  test("dynamic text replaces only the text node", async () => {
    const { host, root } = makeHost();
    createRoot(() => {
      const [count, setCount] = createSignal(1);
      const view = element(NativeNodeTag.View);
      insert(view, dynamicText(() => "Count: " + String(count())));
      insert(root, view);
      host.flush();
      sent.length = 0;
      setCount(2);
      host.flush();
      expect(sent).toHaveLength(1);
      expect(sent[0]).toEqual([{ op: "replace", id: view.children[0]!.id, text: "Count: 2" }]);
    });
  });

  test("Show mounts content before its sentinel and removes it when hidden", () => {
    const { host, root } = makeHost();
    createRoot(() => {
      const [open, setOpen] = createSignal(false);
      const region = Show({ when: open, children: () => text("open") });
      insert(root, region);
      host.flush();
      expect(sent[0]!.map((mutation) => mutation.op)).toEqual(["sentinel", "insert"]);
      sent.length = 0;
      setOpen(true);
      host.flush();
      const ops = sent[0]!;
      expect(ops.map((mutation) => mutation.op)).toEqual(["text", "insert"]);
      expect(ops[1]).toEqual({ op: "insert", parent: ROOT_NODE_ID, child: ops[0]!.id!, before: region.id });
      sent.length = 0;
      setOpen(false);
      host.flush();
      expect(sent[0]).toEqual([{ op: "remove", parent: ROOT_NODE_ID, child: ops[0]!.id! }]);
    });
  });

  test("fragments keep their members before the sentinel", () => {
    const { host, root } = makeHost();
    const a = text("a");
    const b = text("b");
    const frag = fragment([a, b]);
    insert(root, frag);
    host.flush();
    expect(root.children.map((node) => node.id)).toEqual([a.id, b.id, frag.id]);
  });

  test("keyed For reuses rows and moves them into order", () => {
    const { host, root } = makeHost();
    createRoot(() => {
      const [items, setItems] = createSignal([1, 2, 3]);
      let created = 0;
      const list = For({
        each: items,
        key: (item) => item,
        children: (item) => {
          created += 1;
          return text(String(item));
        },
      });
      insert(root, list);
      host.flush();
      expect(created).toBe(3);
      const ids = root.children.slice(0, 3).map((node) => node.text);
      expect(ids).toEqual(["1", "2", "3"]);
      sent.length = 0;
      setItems([3, 1, 2, 4]);
      host.flush();
      expect(created).toBe(4);
      expect(root.children.map((node) => node.text)).toEqual(["3", "1", "2", "4", ""]);
      const ops = sent[0]!.map((mutation) => mutation.op);
      expect(ops.filter((op) => op === "text")).toHaveLength(1);
      expect(ops.filter((op) => op === "remove")).toHaveLength(0);
      sent.length = 0;
      setItems([4]);
      host.flush();
      expect(root.children.map((node) => node.text)).toEqual(["4", ""]);
      expect(sent[0]!.filter((mutation) => mutation.op === "remove")).toHaveLength(3);
    });
  });

  test("dynamic swaps its node and disposes the previous owner", () => {
    const { host, root } = makeHost();
    createRoot(() => {
      const [flag, setFlag] = createSignal(true);
      const region = dynamic(() => (flag() ? text("yes") : text("no")));
      insert(root, region);
      host.flush();
      expect(root.children[0]!.text).toBe("yes");
      setFlag(false);
      host.flush();
      expect(root.children[0]!.text).toBe("no");
      expect(root.children).toHaveLength(2);
    });
  });

  test("keyed item accessors update immutable data without recreating a row", () => {
    const { host, root } = makeHost();
    createRoot(() => {
      const [items, setItems] = createSignal([{ id: 1, title: "first" }]);
      let mounts = 0;
      const list = KeyedFor({ each: items, key: (item) => item.id, children: (item) => {
        mounts += 1;
        return dynamicText(() => item().title);
      } });
      insert(root, list);
      host.flush();
      const row = root.children[0]!;
      sent.length = 0;
      setItems([{ id: 1, title: "updated" }]);
      host.flush();
      expect(mounts).toBe(1);
      expect(root.children[0]).toBe(row);
      expect(sent.flat()).toEqual([{ op: "replace", id: row.id, text: "updated" }]);
    });
  });
});

describe("native component bindings", () => {
  test("calendar days share only their own root's identity and publish selected dates", () => {
    const { host, root } = makeHost();
    createRoot(() => {
      const selected: string[] = [];
      const first = Calendar.Root({
        defaultValue: "2026-09-01",
        onValueChange: (value) => { if (value !== undefined) selected.push(value); },
        children: () => Calendar.Week({ children: () => Calendar.Day({ day: "2026-09-06" }) }),
      });
      const second = Calendar.Root({
        defaultValue: "2026-10-01",
        children: () => Calendar.Day({ day: "2026-10-06" }),
      });
      insert(root, first);
      insert(root, second);
      host.flush();
      const scope = (id: number) => sent.flat().find((entry) => entry.id === id && entry.property === PropertyCode.Scope)?.value;
      expect(scope(first.id)).toBeTypeOf("string");
      expect(scope(first.children[0]!.id)).toBe(scope(first.id));
      expect(scope(first.children[0]!.children[0]!.id)).toBe(scope(first.id));
      expect(scope(second.children[0]!.id)).toBe(scope(second.id));
      expect(scope(second.id)).not.toBe(scope(first.id));
      sent.length = 0;
      dispatchNativeEvent(host, EVENT_COMPONENT_CHANGE, first.id, '{"value":"2026-09-06"}');
      host.flush();
      expect(selected).toEqual(["2026-09-06"]);
      expect(sent.flat()).toContainEqual({ op: "set", id: first.id, property: PropertyCode.CivilValue, value: "2026-09-06" });
      expect(sent.flat().some((entry) => entry.id === second.id)).toBe(false);
    });
  });

  test("SwiftUI input events update the controlled value without rebuilding its host", () => {
    const { host, root } = makeHost();
    createRoot((dispose) => {
      const [value, setValue] = createSignal(0.25);
      const slider = SwiftSlider({ value, min: 0, max: 1, label: () => "Volume " + value(), onValueChange: setValue });
      insert(root, slider);
      host.flush();
      sent.length = 0;
      dispatchNativeEvent(host, EVENT_INPUT, slider.id, "0.75");
      host.flush();
      expect(value()).toBe(0.75);
      expect(sent.flat()).toContainEqual({ op: "set", id: slider.id, property: PropertyCode.Value, value: 0.75 });
      expect(sent.flat().some((entry) => entry.op === "create")).toBe(false);
      dispatchNativeEvent(host, EVENT_INPUT, slider.id, undefined);
      expect(value()).toBe(0.75);
      dispose();
      sent.length = 0;
      setValue(1);
      host.flush();
      expect(sent).toHaveLength(0);
    });
  });
});
