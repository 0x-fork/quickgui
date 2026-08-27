import { describe, expect, test } from "bun:test";
import { QuickGuiEvent } from "@quickgui/native";
import { createSignal, onCleanup } from "solid-js";
import {
  App,
  Button,
  Text,
  View,
  Window,
  createComponent,
  createElement,
  createTextNode,
  insertNode,
  render,
  setProp,
} from "./index.ts";

describe("Solid universal host", () => {
  test("retains an unattached native tree without crossing N-API", () => {
    const parent = createElement("view");
    const child = createTextNode("hello");
    insertNode(parent, child);
    setProp(parent, "display", "flex");
    setProp(parent, "padding", 12);

    expect(parent.children).toEqual([child]);
    expect(child.parent).toBe(parent);
    expect(parent.properties.size).toBe(2);
  });

  test("flushes Solid 2 signal writes at the native event boundary", () => {
    const root = createElement("view");
    const dispose = render(() => {
      const [count, setCount] = createSignal(0);
      return createComponent(View, {
        get children() {
          return [
            createComponent(Text, {
              get children() {
                return `Count: ${count()}`;
              },
            }),
            createComponent(Button, {
              onClick: () => setCount((value) => value + 1),
              children: "Increment",
            }),
          ];
        },
      });
    }, root);

    const container = root.children[0]!;
    const count = container.children[0]!;
    const button = container.children[1]!;
    expect(count.children[0]?.text).toBe("Count: 0");

    button.listeners.get("click")!(new QuickGuiEvent("click", button));

    expect(count.children[0]?.text).toBe("Count: 1");
    dispose();
  });

  test("disposes a mounted Solid root when its Window closes", () => {
    const app = new App();
    const window = new Window({ title: "Dispose test" });
    let cleaned = false;
    const dispose = render(() => {
      onCleanup(() => {
        cleaned = true;
      });
      return createComponent(View, { children: "Mounted" });
    }, window);

    expect(window.root.children).toHaveLength(1);
    window.close();

    expect(window.closed).toBe(true);
    expect(cleaned).toBe(true);
    expect(window.root.children).toHaveLength(0);
    dispose();
    app.destroy();
  });
});
