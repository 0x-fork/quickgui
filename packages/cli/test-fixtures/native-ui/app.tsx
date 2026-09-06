import { QuickGuiEvent, type NativeNode } from "@quickgui/native";
import { For, View, createRoot, createSignal, text } from "@quickgui/ui";

export type ImportedNode = import("@quickgui/native").NativeNode;
export async function loadNative() { return import("@quickgui/native"); }
export function styled(): NativeNode {
  return <View selected disabled style={{ selected: { color: "#ff0000" }, disabled: { opacity: 0.5 } }} ref={(node: import("@quickgui/native").NativeNode) => { void node; }} />;
}

function Capture(props: { onValueChange: (value: number, event: QuickGuiEvent) => void }): NativeNode {
  const node = text("callback");
  props.onValueChange(7, new QuickGuiEvent(4, node, "7"));
  return node;
}

export function exercise(): { keys: number; reused: boolean; value: number } {
  return createRoot((dispose) => {
    const [items, setItems] = createSignal([1, 2, 3]);
    const [value, setValue] = createSignal(0);
    let keys = 0;
    const list = <For each={items} key={(item) => { keys += 1; return item; }}>{(item) => text(String(item))}</For>;
    const first = list.group![0]!;
    setItems([3, 1, 2]);
    const reused = first === list.group![1];
    <Capture onValueChange={setValue} />;
    let referenced = 0;
    const view = <View ref={(node) => { referenced = node.id; }} />;
    if (referenced !== view.id) throw new Error("ref was not called with its element");
    const result = { keys, reused, value: value() };
    dispose();
    return result;
  });
}
