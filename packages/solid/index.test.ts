import { describe, expect, test } from "bun:test";
import {
  app,
  NativeNodeTag,
  PropertyCode,
  QuickGuiEvent,
  Window,
} from "@quickgui/native";
import { createSignal, onCleanup } from "solid-js";
import * as solid from "./index.ts";
import {
  Button,
  Input,
  Markdown,
  Popover,
  SystemPopover,
  Svg,
  Terminal,
  Text,
  TextArea,
  View,
  VirtualList,
  createComponent,
  createElement,
  createRenderer,
  createTextNode,
  capturedPointerFromEvent,
  insertNode,
  setProp,
  terminalStatusFromEvent,
} from "./index.ts";

describe("Solid universal host", () => {
  test("keeps native application APIs in @quickgui/native", () => {
    expect("app" in solid).toBe(false);
    expect("Window" in solid).toBe(false);
    expect("Dialog" in solid).toBe(false);
  });

  test("exports only the Content compound part", () => {
    expect(Popover.Content).toBe(solid.PopoverContent);
    expect(SystemPopover.Content).toBe(solid.SystemPopoverContent);
    expect(Object.keys(Popover)).toEqual(["Root", "Trigger", "Content"]);
    expect(Object.keys(SystemPopover)).toEqual(["Root", "Trigger", "Content"]);
  });

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

  test("projects retained hover and pressed styles into the native core", () => {
    const button = createComponent(Button, {
      style: {
        hoverBackgroundColor: "#222233",
        hoverColor: "#ffffff",
        activeBackgroundColor: "#111122",
        activeColor: "#ddddff",
        transition:
          "background-color 90ms, border-color 90ms, color 90ms",
      },
      children: "New agent",
    });

    expect(button.properties.get(PropertyCode.HoverBackgroundColor)).toBeTypeOf(
      "number",
    );
    expect(button.properties.get(PropertyCode.HoverColor)).toBeTypeOf("number");
    expect(
      button.properties.get(PropertyCode.ActiveBackgroundColor),
    ).toBeTypeOf("number");
    expect(button.properties.get(PropertyCode.ActiveColor)).toBeTypeOf(
      "number",
    );
    expect(button.properties.get(PropertyCode.Transition)).toBe(90);
  });

  test("projects modal input, focus, dismissal, and accessibility to the native core", () => {
    const prompt = createComponent(TextArea, {
      autoFocus: true,
      value: "",
    });
    const surface = createComponent(View, {
      overlay: true,
      focusTrap: true,
      restorePreviousFocus: true,
      "aria-modal": true,
      dismissOnEscape: true,
      dismissOnPointerOutside: true,
      onDismiss() {},
      children: prompt,
    });

    expect(surface.properties.get(PropertyCode.Overlay)).toBe(true);
    expect(surface.properties.get(PropertyCode.FocusTrap)).toBe(true);
    expect(surface.properties.get(PropertyCode.RestorePreviousFocus)).toBe(
      true,
    );
    expect(surface.properties.get(PropertyCode.AccessibilityModal)).toBe(true);
    expect(surface.properties.get(PropertyCode.DismissOnEscape)).toBe(true);
    expect(
      surface.properties.get(PropertyCode.DismissOnPointerOutside),
    ).toBe(true);
    expect(surface.properties.get(PropertyCode.DismissListener)).toBe(true);
    expect(prompt.properties.get(PropertyCode.AutoFocus)).toBe(true);
  });

  test("flushes Solid 2 signal writes at the native event boundary", async () => {
    await app.whenReady();
    let renderedWindow: Window | undefined;
    let eventWindow: Window | undefined;
    const window = new Window({
      renderer: createRenderer(() => {
        renderedWindow = Window.getCurrentWindow();
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
                onClick: () => {
                  eventWindow = Window.getCurrentWindow();
                  setCount((value) => value + 1);
                },
                children: "Increment",
              }),
            ];
          },
        });
      }),
    });

    const container = window.root.children[0]!;
    const count = container.children[0]!;
    const button = container.children[1]!;
    expect(renderedWindow).toBe(window);
    expect(count.children[0]?.text).toBe("Count: 0");

    window._dispatchEvent("click", button.id);

    expect(count.children[0]?.text).toBe("Count: 1");
    expect(eventWindow).toBe(window);
    expect(() => Window.getCurrentWindow()).toThrow(
      "while rendering or handling a window event",
    );
    window.close();
  });

  test("disposes a mounted Solid root when its Window closes", async () => {
    await app.whenReady();
    let cleaned = false;
    const window = new Window({
      title: "Dispose test",
      renderer: createRenderer(() => {
        onCleanup(() => {
          cleaned = true;
        });
        return createComponent(View, { children: "Mounted" });
      }),
    });

    expect(window.root.children).toHaveLength(1);
    window.close();

    expect(window.closed).toBe(true);
    expect(cleaned).toBe(true);
    expect(window.root.children).toHaveLength(0);
  });

  test("commits shared signal writes before a secondary window root closes", async () => {
    await app.whenReady();
    const [value, setValue] = createSignal("before");
    const mainWindow = new Window({
      title: "Main",
      renderer: createRenderer(() =>
        createComponent(Text, {
          get children() {
            return value();
          },
        }),
      ),
    });
    const secondaryWindow = new Window({
      title: "Secondary",
      renderer: createRenderer(() => {
        const window = Window.getCurrentWindow();
        return createComponent(Button, {
          onClick: () => {
            setValue("after");
            window.close();
          },
          children: "Save",
        });
      }),
    });

    const text = mainWindow.root.children[0]!;
    const button = secondaryWindow.root.children[0]!;
    expect(text.children[0]?.text).toBe("before");
    secondaryWindow._dispatchEvent("click", button.id);

    expect(secondaryWindow.closed).toBe(true);
    expect(text.children[0]?.text).toBe("after");
    mainWindow.close();
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
    input.listeners.get("input")!(
      new QuickGuiEvent("input", input, "hello world"),
    );
    expect(value).toBe("hello world");
    input.listeners.get("submit")!(
      new QuickGuiEvent("submit", input, "hello world"),
    );
    expect(submitted).toBe("hello world");
  });

  test("maps web-style password input types without replacing the controlled value", () => {
    const input = createComponent(Input, {
      type: "password",
      value: "sk-secret",
    });

    expect(input.properties.get(PropertyCode.Password)).toBe(true);
    expect(input.properties.get(PropertyCode.Value)).toBe("sk-secret");

    setProp(input, "type", "text", "password");
    expect(input.properties.get(PropertyCode.Password)).toBe(false);
    expect(input.properties.get(PropertyCode.Value)).toBe("sk-secret");
  });

  test("can preserve keyboard focus when a button is pressed with a pointer", () => {
    const button = createComponent(Button, { focusOnPointer: false });

    expect(button.properties.get(PropertyCode.FocusOnPointer)).toBe(false);
  });

  test("maps paint-free hit slop for thin native interaction targets", () => {
    const divider = createComponent(View, {
      style: {
        width: 1,
        hitSlop: 2,
        hitSlopTop: 3,
        hitSlopRight: 4,
        hitSlopBottom: 5,
        hitSlopLeft: 6,
      },
    });

    expect(divider.properties.get(PropertyCode.Width)).toBe(1);
    expect(divider.properties.get(PropertyCode.HitSlop)).toBe(2);
    expect(divider.properties.get(PropertyCode.HitSlopTop)).toBe(3);
    expect(divider.properties.get(PropertyCode.HitSlopRight)).toBe(4);
    expect(divider.properties.get(PropertyCode.HitSlopBottom)).toBe(5);
    expect(divider.properties.get(PropertyCode.HitSlopLeft)).toBe(6);
  });

  test("creates retained SVG nodes whose source is parsed by the Rust core", () => {
    const source = '<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" />';
    const icon = createComponent(Svg, {
      source,
      style: { width: 16, height: 16, color: "#ffffff" },
    });

    expect(icon.tag).toBe(NativeNodeTag.Svg);
    expect(icon.properties.get(PropertyCode.Value)).toBe(source);
    expect(icon.properties.get(PropertyCode.Width)).toBe(16);
    expect(icon.properties.get(PropertyCode.Height)).toBe(16);
    expect(icon.properties.get(PropertyCode.Color)).toBeTypeOf("number");
  });

  test("declaratively configures a core-owned PTY terminal and decodes status events", () => {
    let status = "";
    const terminal = createComponent(Terminal, {
      program: "/bin/zsh",
      args: ["-l"],
      cwd: "/tmp",
      env: { QUICKGUI_TERMINAL_TEST: "1" },
      scrollback: 20_000,
      terminalCursorColor: "#0969da",
      terminalPaddingColor: "extend",
      fontThicken: true,
      terminalPalette: [
        "#24292f",
        "#cf222e",
        "#116329",
        "#4d2d00",
        "#0969da",
        "#8250df",
        "#1b7c83",
        "#6e7781",
        "#57606a",
        "#a40e26",
        "#1a7f37",
        "#633c01",
        "#218bff",
        "#a475f9",
        "#3192aa",
        "#8c959f",
      ],
      style: { fontFamily: "JetBrainsMono Nerd Font Mono" },
      onStatus: (event) => {
        status = terminalStatusFromEvent(event).status;
      },
    });

    expect(terminal.tag).toBe(NativeNodeTag.Terminal);
    expect(terminal.properties.get(PropertyCode.TerminalProgram)).toBe(
      "/bin/zsh",
    );
    expect(terminal.properties.get(PropertyCode.TerminalArguments)).toBe(
      '["-l"]',
    );
    expect(terminal.properties.get(PropertyCode.TerminalWorkingDirectory)).toBe(
      "/tmp",
    );
    expect(terminal.properties.get(PropertyCode.TerminalEnvironment)).toBe(
      '{"QUICKGUI_TERMINAL_TEST":"1"}',
    );
    expect(terminal.properties.get(PropertyCode.TerminalScrollback)).toBe(
      20_000,
    );
    expect(terminal.properties.get(PropertyCode.TerminalStatusListener)).toBe(
      true,
    );
    expect(terminal.properties.get(PropertyCode.FontFamily)).toBe(
      "JetBrainsMono Nerd Font Mono",
    );
    expect(terminal.properties.get(PropertyCode.TerminalCursorColor)).toBeTypeOf(
      "number",
    );
    expect(terminal.properties.get(PropertyCode.TerminalPaddingColor)).toBe(
      "extend",
    );
    expect(terminal.properties.get(PropertyCode.TerminalFontThicken)).toBe(
      true,
    );
    expect(
      JSON.parse(
        terminal.properties.get(PropertyCode.TerminalPalette) as string,
      ),
    ).toHaveLength(16);

    terminal.listeners.get("terminal")!(
      new QuickGuiEvent(
        "terminal",
        terminal,
        '{"status":"running","title":"zsh","workingDirectory":"/tmp","processId":42}',
      ),
    );
    expect(status).toBe("running");
  });

  test("bridges the Rust-core captured pointer stream", () => {
    let delta = 0;
    const divider = createComponent(View, {
      onPointer: (event) => {
        delta =
          capturedPointerFromEvent(event).position.x -
          capturedPointerFromEvent(event).origin.x;
      },
    });

    expect(divider.properties.get(PropertyCode.PointerListener)).toBe(true);
    divider.listeners.get("pointer")!(
      new QuickGuiEvent(
        "pointer",
        divider,
        '{"phase":"move","position":{"x":310,"y":40},"origin":{"x":250,"y":40},"delta":{"x":4,"y":0},"button":"left"}',
      ),
    );
    expect(delta).toBe(60);
  });

  test("retains Markdown source and streaming presentation properties", () => {
    const markdown = createComponent(Markdown, {
      content: "# Hello",
      streaming: true,
      markdownLinkColor: "#60a5fa",
    });

    expect(markdown.properties.get(PropertyCode.Value)).toBe("# Hello");
    expect(markdown.properties.get(PropertyCode.Streaming)).toBe(true);
    expect(markdown.properties.get(PropertyCode.MarkdownLinkColor)).toBeTypeOf(
      "number",
    );
  });

  test("creates unstyled variable lists with native windowing properties", () => {
    const list = createComponent(VirtualList, {
      estimatedItemHeight: 180,
      overscan: 3,
      listAlignment: "bottom",
      followMode: "tail",
      children: createComponent(Text, { children: "Visible row" }),
    });

    expect(list.properties.get(PropertyCode.EstimatedItemHeight)).toBe(180);
    expect(list.properties.get(PropertyCode.Overscan)).toBe(3);
    expect(list.properties.get(PropertyCode.ListAlignment)).toBe("bottom");
    expect(list.properties.get(PropertyCode.FollowMode)).toBe("tail");
    expect(list.children).toHaveLength(1);
  });

  test("coordinates Popover compound parts without exposing the native anchor", async () => {
    await app.whenReady();
    const [open, setOpen] = createSignal(false);
    const changes: Array<{ open: boolean; reason: string }> = [];
    const window = new Window({
      title: "Popover compound parts",
      renderer: createRenderer(() =>
        createComponent(Popover.Root, {
          get open() {
            return open();
          },
          dismissOnEscape: false,
          onOpenChange(nextOpen, details) {
            changes.push({ open: nextOpen, reason: details.reason });
            setOpen(nextOpen);
          },
          get children() {
            return [
              createComponent(Popover.Trigger, { children: "Open" }),
              createComponent(Popover.Content, {
                width: 240,
                height: 120,
                placement: "bottom-end",
                gap: 8,
                viewportMargin: 12,
                children: createComponent(Text, { children: "Popover" }),
              }),
            ];
          },
        }),
      ),
    });

    const trigger = window.root.children[0]!;
    window._focusNode = () => {
      throw new Error("Popover focus restoration must stay in the Rust core");
    };
    expect(window.root.children).toEqual([trigger]);

    window._dispatchEvent("click", trigger.id);

    const popover = window.root.children[1]!;
    expect(open()).toBe(true);
    expect(popover.properties.get(PropertyCode.AnchorTarget)).toBe(
      String(trigger.id),
    );
    expect(popover.properties.get(PropertyCode.AnchorPlacement)).toBe(
      "bottom-end",
    );
    expect(popover.properties.get(PropertyCode.AnchorGap)).toBe(8);
    expect(popover.properties.get(PropertyCode.ViewportMargin)).toBe(12);
    expect(popover.properties.get(PropertyCode.DismissOnEscape)).toBe(false);
    expect(popover.properties.get(PropertyCode.DismissOnPointerOutside)).toBe(
      true,
    );
    expect(popover.properties.get(PropertyCode.DismissListener)).toBe(true);
    expect(popover.children[0]?.children[0]?.text).toBe("Popover");
    expect(changes).toEqual([{ open: true, reason: "trigger-press" }]);

    popover.listeners.get("dismiss")!(new QuickGuiEvent("dismiss", popover));
    await Promise.resolve();
    await Promise.resolve();
    expect(open()).toBe(false);
    expect(window.root.children).toEqual([trigger]);
    expect(changes).toEqual([
      { open: true, reason: "trigger-press" },
      { open: false, reason: "dismiss" },
    ]);

    window._dispatchEvent("click", trigger.id);
    expect(open()).toBe(true);
    setOpen(false);
    await Promise.resolve();
    await Promise.resolve();
    expect(window.root.children).toEqual([trigger]);
    window.close();
  });

  test("mounts SystemPopover.Content into a separately disposed renderer", async () => {
    await app.whenReady();
    const [open, setOpen] = createSignal(false);
    const changes: Array<{ open: boolean; reason: string }> = [];
    const owner = new Window({
      title: "System popover owner",
      renderer: createRenderer(() =>
        createComponent(SystemPopover.Root, {
          get open() {
            return open();
          },
          onOpenChange(nextOpen, details) {
            changes.push({ open: nextOpen, reason: details.reason });
            setOpen(nextOpen);
          },
          get children() {
            return [
              createComponent(SystemPopover.Trigger, { children: "Open" }),
              createComponent(SystemPopover.Content, {
                width: 260,
                height: 140,
                placement: "bottom-start",
                gap: 8,
                viewportMargin: 12,
                children: createComponent(Text, { children: "Separate root" }),
              }),
            ];
          },
        }),
      ),
    });

    const trigger = owner.root.children[0]!;
    owner._focusNode = () => {
      throw new Error(
        "SystemPopover focus restoration must stay in the Rust core",
      );
    };
    expect(trigger.materialized).toBe(true);
    expect(trigger.host).toBe(owner);
    expect(owner.nodes.has(trigger.id)).toBe(true);
    expect(owner.root.children).toEqual([trigger]);

    owner._dispatchEvent("click", trigger.id);
    await Promise.resolve();

    const systemWindow = [...app.windows.values()].find(
      (window) => window !== owner,
    );
    expect(open()).toBe(true);
    expect(systemWindow).toBeDefined();
    expect(systemWindow!.root.children).toHaveLength(1);
    const surface = systemWindow!.root.children[0]!;
    expect(surface.properties.get(PropertyCode.Width)).toBe(260);
    expect(surface.properties.get(PropertyCode.Height)).toBe(140);
    expect(surface.properties.has(PropertyCode.AnchorTarget)).toBe(false);
    expect(surface.children[0]?.children[0]?.text).toBe("Separate root");
    expect(changes).toEqual([{ open: true, reason: "trigger-press" }]);

    systemWindow!.close();
    await Promise.resolve();
    await Promise.resolve();
    expect(open()).toBe(false);
    expect(systemWindow!.closed).toBe(true);
    expect(systemWindow!.root.children).toHaveLength(0);
    expect(changes).toEqual([
      { open: true, reason: "trigger-press" },
      { open: false, reason: "dismiss" },
    ]);

    owner._dispatchEvent("click", trigger.id);
    await Promise.resolve();
    const reopened = [...app.windows.values()].find(
      (window) => window !== owner,
    );
    expect(open()).toBe(true);
    expect(reopened).toBeDefined();

    // AppKit consumes a press on the active SystemPopover anchor before JavaScript dispatch. A
    // native close still settles the controlled JSX lifecycle normally.
    reopened!.close();
    await Promise.resolve();
    await Promise.resolve();
    expect(open()).toBe(false);
    expect(changes.at(-1)).toEqual({ open: false, reason: "dismiss" });
    expect([...app.windows.values()]).toEqual([owner]);

    owner._dispatchEvent("click", trigger.id);
    await Promise.resolve();
    const ownedPopover = [...app.windows.values()].find(
      (window) => window !== owner,
    );
    expect(ownedPopover).toBeDefined();
    owner.close();
    expect(ownedPopover!.closed).toBe(true);
    expect([...app.windows.values()]).toEqual([]);
    await Promise.resolve();
    await Promise.resolve();
  });
});
