import { describe, expect, test } from "bun:test";
import { PropertyCode, QuickGuiEvent } from "@quickgui/native";
import { createSignal, onCleanup } from "solid-js";
import {
  App,
  Button,
  Input,
  Markdown,
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

  test("commits shared signal writes before a secondary window root closes", () => {
    const app = new App();
    const mainWindow = new Window({ title: "Main" });
    const secondaryWindow = new Window({ title: "Secondary" });
    const [value, setValue] = createSignal("before");
    const disposeMain = render(
      () =>
        createComponent(Text, {
          get children() {
            return value();
          },
        }),
      mainWindow,
    );
    const disposeSecondary = render(
      () =>
        createComponent(Button, {
          onClick: () => {
            setValue("after");
            secondaryWindow.close();
          },
          children: "Save",
        }),
      secondaryWindow,
    );

    const text = mainWindow.root.children[0]!;
    const button = secondaryWindow.root.children[0]!;
    expect(text.children[0]?.text).toBe("before");
    button.listeners.get("click")!(new QuickGuiEvent("click", button));

    expect(secondaryWindow.closed).toBe(true);
    expect(text.children[0]?.text).toBe("after");
    disposeSecondary();
    disposeMain();
    app.destroy();
  });

  test("bridges controlled input values and exact native input and submit payloads", () => {
    let value = "";
    let submitted = "";
    const input = createComponent(Input, {
      value: "hello",
      onInput: (event: QuickGuiEvent) => {
        value = event.value ?? "";
      },
      onSubmit: (event: QuickGuiEvent) => {
        submitted = event.value ?? "";
      },
    });

    expect(input.properties.get(PropertyCode.Value)).toBe("hello");
    expect(input.properties.get(PropertyCode.InputListener)).toBe(true);
    expect(input.properties.get(PropertyCode.SubmitListener)).toBe(true);
    input.listeners.get("input")!(new QuickGuiEvent("input", input, "hello world"));
    expect(value).toBe("hello world");
    input.listeners.get("submit")!(new QuickGuiEvent("submit", input, "hello world"));
    expect(submitted).toBe("hello world");
  });

  test("maps web-style password input types without replacing the controlled value", () => {
    const input = createComponent(Input, { type: "password", value: "sk-secret" });

    expect(input.properties.get(PropertyCode.Password)).toBe(true);
    expect(input.properties.get(PropertyCode.Value)).toBe("sk-secret");

    setProp(input, "type", "text", "password");
    expect(input.properties.get(PropertyCode.Password)).toBe(false);
    expect(input.properties.get(PropertyCode.Value)).toBe("sk-secret");
  });

  test("retains Markdown source and streaming presentation properties", () => {
    const markdown = createComponent(Markdown, {
      content: "# Hello",
      streaming: true,
      markdownLinkColor: "#60a5fa",
    });

    expect(markdown.properties.get(PropertyCode.Value)).toBe("# Hello");
    expect(markdown.properties.get(PropertyCode.Streaming)).toBe(true);
    expect(markdown.properties.get(PropertyCode.MarkdownLinkColor)).toBeTypeOf("number");
  });
});
