import { describe, expect, test } from "bun:test";
import {
  app,
  parseColor,
  MAX_COLLECTION_JSON_BYTES,
  MAX_COMPONENT_VALUE_BYTES,
  MAX_OPTIONS_JSON_BYTES,
  MAX_KEYMAP_JSON_BYTES,
  MAX_MENU_JSON_BYTES,
  MAX_STYLE_DECLARATION_BYTES,
  MAX_TOOLTIP_TEXT_BYTES,
  NativeNodeTag,
  PropertyCode,
  QuickGuiEvent,
  Window,
} from "@quickgui/native";
import { createSignal, onCleanup } from "solid-js";
import * as solid from "./index.ts";
import {
  Accordion,
  AlertDialog,
  Autocomplete,
  Button,
  Calendar,
  Checkbox,
  Collapsible,
  Combobox,
  ContextMenu,
  DateField,
  Dialog,
  Image,
  Meter,
  Field,
  Fieldset,
  Input,
  Markdown,
  Menubar,
  NumberField,
  Popover,
  PopoverMenu,
  Progress,
  Radio,
  RadioGroup,
  Select,
  Switch,
  Shader,
  Slider,
  Splitter,
  Table,
  Tabs,
  TimeField,
  Toast,
  Toggle,
  ToggleGroup,
  Toolbar,
  Tree,
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
  actionFromEvent,
  capturedPointerFromEvent,
  dropEventFromEvent,
  encodeMenu,
  gestureEventFromEvent,
  keyEventFromEvent,
  insertNode,
  menuSelectionFromEvent,
  mouseEventFromEvent,
  wheelEventFromEvent,
  setProp,
  terminalStatusFromEvent,
  type MenuSelectDetails,
} from "./index.ts";

describe("Solid universal host", () => {
  test("keeps native application APIs in @quickgui/native", () => {
    expect("app" in solid).toBe(false);
    expect("Window" in solid).toBe(false);
    // `Dialog` in this package is the caller-styled in-window composition, never the native
    // alert/file dialog namespace that stays in @quickgui/native.
    expect("showAlertDialog" in solid.Dialog).toBe(false);
    expect("showOpenDialog" in solid.Dialog).toBe(false);
    expect(solid.Dialog.Popup).toBe(solid.DialogPopup);
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

  test("projects independent border edges and CSS-like box shadows", () => {
    const panel = createComponent(View, {
      style: {
        color: "#445566",
        borderWidth: 1,
        borderTopWidth: 0,
        borderRightWidth: "2px",
        borderBottomWidth: 3,
        borderLeftWidth: "4px",
        borderColor: "#11223380",
        boxShadow:
          "0 8px 24px -8px rgba(15, 23, 42, 0.35), inset 0 1px 0 currentColor",
      },
    });

    expect(panel.properties.get(PropertyCode.BorderWidth)).toBe(1);
    expect(panel.properties.get(PropertyCode.BorderTopWidth)).toBe(0);
    expect(panel.properties.get(PropertyCode.BorderRightWidth)).toBe(2);
    expect(panel.properties.get(PropertyCode.BorderBottomWidth)).toBe(3);
    expect(panel.properties.get(PropertyCode.BorderLeftWidth)).toBe(4);
    expect(panel.properties.get(PropertyCode.BorderColor)).toBe(0x80332211);
    expect(
      JSON.parse(String(panel.properties.get(PropertyCode.BoxShadow))),
    ).toEqual([
      {
        offsetX: 0,
        offsetY: 8,
        blurRadius: 24,
        spreadRadius: -8,
        color: 0x592a170f,
        inset: false,
      },
      {
        offsetX: 0,
        offsetY: 1,
        blurRadius: 0,
        spreadRadius: 0,
        color: null,
        inset: true,
      },
    ]);
  });

  test("rejects invalid CSS-like box shadows", () => {
    const negativeBlur = createElement("view");
    expect(() => setProp(negativeBlur, "boxShadow", "0 2px -1px black")).toThrow(
      "blur radius cannot be negative",
    );
    const tooMany = createElement("view");
    expect(() =>
      setProp(
        tooMany,
        "boxShadow",
        Array(9).fill("0 1px black").join(", "),
      ),
    ).toThrow("at most 8 shadows");
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
        '{"phase":"move","position":{"x":310,"y":40},"origin":{"x":250,"y":40},"localPosition":{"x":60,"y":20},"localOrigin":{"x":0,"y":20},"delta":{"x":4,"y":0},"button":"left"}',
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

  test("exports Base UI-shaped compound parts for every bound core component", () => {
    expect(Object.keys(Checkbox)).toEqual(["Root", "Indicator"]);
    expect(Object.keys(Radio)).toEqual(["Root", "Indicator"]);
    expect(Object.keys(Switch)).toEqual(["Root", "Thumb"]);
    expect(Object.keys(Tabs)).toEqual([
      "Root",
      "List",
      "Tab",
      "Indicator",
      "Panel",
    ]);
    expect(Object.keys(Collapsible)).toEqual(["Root", "Trigger", "Panel"]);
    expect(Object.keys(Accordion)).toEqual([
      "Root",
      "Item",
      "Header",
      "Trigger",
      "Panel",
    ]);
    expect(Object.keys(Field)).toEqual([
      "Root",
      "Label",
      "Control",
      "Description",
      "Error",
    ]);
    expect(Object.keys(Fieldset)).toEqual([
      "Root",
      "Legend",
      "Description",
      "Control",
    ]);
    expect(Object.keys(Dialog)).toEqual([
      "Root",
      "Trigger",
      "Portal",
      "Backdrop",
      "Popup",
      "Title",
      "Description",
      "Close",
    ]);
    expect(Object.keys(AlertDialog)).toEqual(Object.keys(Dialog));
  });

  test("declares controlled checkbox and switch toggle state ahead of the core click", () => {
    let checked: boolean | undefined;
    const checkbox = createComponent(Checkbox.Root, {
      checked: "indeterminate",
      onCheckedChange: (next: boolean) => {
        checked = next;
      },
      children: createComponent(Checkbox.Indicator, { children: "–" }),
    });

    expect(checkbox.properties.get(PropertyCode.Part)).toBe("checkbox");
    expect(checkbox.properties.get(PropertyCode.Checked)).toBe(false);
    expect(checkbox.properties.get(PropertyCode.Indeterminate)).toBe(true);
    expect(checkbox.properties.get(PropertyCode.ClickListener)).toBe(true);
    checkbox.listeners.get("click")!(new QuickGuiEvent("click", checkbox));
    expect(checked).toBe(true);

    const toggled = createComponent(Switch.Root, { defaultChecked: true });
    expect(toggled.properties.get(PropertyCode.Part)).toBe("switch");
    expect(toggled.properties.get(PropertyCode.Checked)).toBe(true);
    toggled.listeners.get("click")!(new QuickGuiEvent("click", toggled));
    expect(toggled.properties.get(PropertyCode.Checked)).toBe(false);

    const thumb = createComponent(Switch.Thumb, {});
    expect(thumb.properties.get(PropertyCode.Part)).toBe("switch-thumb");
  });

  test("bounds compound scopes and tooltip text before they cross N-API", () => {
    const hinted = createComponent(View, {
      tooltip: "Rename this workspace",
      tooltipPlacement: "top",
      tooltipDelay: 250,
      tooltipGap: 7,
      tooltipViewportMargin: 8,
    });

    expect(hinted.properties.get(PropertyCode.Tooltip)).toBe(
      "Rename this workspace",
    );
    expect(hinted.properties.get(PropertyCode.TooltipPlacement)).toBe("top");
    expect(hinted.properties.get(PropertyCode.TooltipDelay)).toBe(250);

    const overlong = createElement("view");
    expect(() =>
      setProp(overlong, "tooltip", "x".repeat(MAX_TOOLTIP_TEXT_BYTES + 1)),
    ).toThrow("tooltip text is limited");
    expect(() =>
      setProp(overlong, "scope", "s".repeat(MAX_COMPONENT_VALUE_BYTES + 1)),
    ).toThrow("scopes and values are limited");
  });

  test("shares one scope across controlled tab parts and reports activation", async () => {
    await app.whenReady();
    const [value, setValue] = createSignal("overview");
    let changed: string | undefined;
    const window = new Window({
      title: "Tabs",
      renderer: createRenderer(() =>
        createComponent(Tabs.Root, {
          get value() {
            return value();
          },
          onValueChange: (next: string) => {
            changed = next;
            setValue(next);
          },
          orientation: "vertical",
          activation: "automatic",
          loop: false,
          keepMounted: true,
          get children() {
            return [
              createComponent(Tabs.List, {
                get children() {
                  return [
                    createComponent(Tabs.Tab, {
                      value: "overview",
                      children: "Overview",
                    }),
                    createComponent(Tabs.Tab, {
                      value: "usage",
                      children: "Usage",
                    }),
                  ];
                },
              }),
              createComponent(Tabs.Panel, {
                value: "overview",
                children: "Overview panel",
              }),
              createComponent(Tabs.Panel, {
                value: "usage",
                children: "Usage panel",
              }),
            ];
          },
        }),
      ),
    });

    const root = window.root.children[0]!;
    const list = root.children[0]!;
    const first = list.children[0]!;
    const second = list.children[1]!;
    const usagePanel = root.children[2]!;
    const scope = String(root.properties.get(PropertyCode.Scope));

    expect(root.properties.get(PropertyCode.Part)).toBe("tabs");
    expect(scope.startsWith("qg-tabs-")).toBe(true);
    expect(list.properties.get(PropertyCode.Part)).toBe("tabs-list");
    expect(list.properties.get(PropertyCode.Scope)).toBe(scope);
    expect(list.properties.get(PropertyCode.Orientation)).toBe("vertical");
    expect(list.properties.get(PropertyCode.ActivateOnFocus)).toBe(true);
    expect(list.properties.get(PropertyCode.LoopFocus)).toBe(false);
    expect(first.properties.get(PropertyCode.Part)).toBe("tab");
    expect(first.properties.get(PropertyCode.PartValue)).toBe("overview");
    expect(first.properties.get(PropertyCode.ActiveValue)).toBe("overview");
    expect(usagePanel.properties.get(PropertyCode.Part)).toBe("tab-panel");
    expect(usagePanel.properties.get(PropertyCode.PartValue)).toBe("usage");
    expect(usagePanel.properties.get(PropertyCode.KeepMounted)).toBe(true);

    window._dispatchEvent("click", second.id);
    expect(changed).toBe("usage");
    expect(first.properties.get(PropertyCode.ActiveValue)).toBe("usage");
    expect(usagePanel.properties.get(PropertyCode.ActiveValue)).toBe("usage");
    window.close();
  });

  test("controls collapsible and accordion disclosure state through core part properties", async () => {
    await app.whenReady();
    const window = new Window({
      title: "Disclosures",
      renderer: createRenderer(() => [
        createComponent(Collapsible.Root, {
          defaultOpen: false,
          keepMounted: true,
          get children() {
            return [
              createComponent(Collapsible.Trigger, { children: "Details" }),
              createComponent(Collapsible.Panel, { children: "Body" }),
            ];
          },
        }),
        createComponent(Accordion.Root, {
          defaultValue: "first",
          headingLevel: 4,
          get children() {
            return createComponent(Accordion.Item, {
              value: "first",
              index: 0,
              get children() {
                return [
                  createComponent(Accordion.Header, {
                    get children() {
                      return createComponent(Accordion.Trigger, {
                        children: "First",
                      });
                    },
                  }),
                  createComponent(Accordion.Panel, { children: "Body" }),
                ];
              },
            });
          },
        }),
      ]),
    });

    const collapsible = window.root.children[0]!;
    const trigger = collapsible.children[0]!;
    const panel = collapsible.children[1]!;
    expect(collapsible.properties.get(PropertyCode.Part)).toBe("collapsible");
    expect(trigger.properties.get(PropertyCode.Part)).toBe(
      "collapsible-trigger",
    );
    expect(panel.properties.get(PropertyCode.Open)).toBe(false);
    expect(panel.properties.get(PropertyCode.KeepMounted)).toBe(true);
    window._dispatchEvent("click", trigger.id);
    expect(panel.properties.get(PropertyCode.Open)).toBe(true);

    const accordion = window.root.children[1]!;
    const item = accordion.children[0]!;
    const header = item.children[0]!;
    const accordionTrigger = header.children[0]!;
    const accordionPanel = item.children[1]!;
    expect(item.properties.get(PropertyCode.Part)).toBe("accordion-item");
    expect(item.properties.get(PropertyCode.PartValue)).toBe("first");
    expect(item.properties.get(PropertyCode.ItemIndex)).toBe(0);
    expect(header.properties.get(PropertyCode.HeadingLevel)).toBe(4);
    expect(accordionPanel.properties.get(PropertyCode.Open)).toBe(true);
    window._dispatchEvent("click", accordionTrigger.id);
    expect(accordionPanel.properties.get(PropertyCode.Open)).toBe(false);
    window.close();
  });

  test("projects field and fieldset relationships and inherited disabled state", async () => {
    await app.whenReady();
    const window = new Window({
      title: "Field",
      renderer: createRenderer(() =>
        createComponent(Fieldset.Root, {
          disabled: true,
          get children() {
            return [
              createComponent(Fieldset.Legend, { children: "Account" }),
              createComponent(Field.Root, {
                invalid: true,
                required: true,
                validationMessage: "Enter an address",
                get children() {
                  return [
                    createComponent(Field.Label, { children: "Email" }),
                    createComponent(Field.Label, {
                      passive: true,
                      children: "Email",
                    }),
                    createComponent(Field.Control, {
                      value: "",
                      placeholder: "you@example.com",
                    }),
                    createComponent(Field.Description, {
                      children: "We never share it",
                    }),
                    createComponent(Field.Error, {
                      children: "Enter an address",
                    }),
                  ];
                },
              }),
            ];
          },
        }),
      ),
    });

    const fieldset = window.root.children[0]!;
    const field = fieldset.children[1]!;
    const label = field.children[0]!;
    const passiveLabel = field.children[1]!;
    const control = field.children[2]!;
    const description = field.children[3]!;
    const error = field.children[4]!;

    expect(fieldset.properties.get(PropertyCode.Part)).toBe("fieldset");
    expect(field.properties.get(PropertyCode.Part)).toBe("field");
    expect(field.properties.get(PropertyCode.Disabled)).toBe(true);
    expect(label.properties.get(PropertyCode.Part)).toBe("field-label");
    expect(passiveLabel.properties.get(PropertyCode.Part)).toBe(
      "field-passive-label",
    );
    expect(control.tag).toBe(NativeNodeTag.Input);
    expect(control.properties.get(PropertyCode.Part)).toBe("field-control");
    expect(control.properties.get(PropertyCode.Required)).toBe(true);
    expect(control.properties.get(PropertyCode.Invalid)).toBe(true);
    expect(control.properties.get(PropertyCode.ValidationMessage)).toBe(
      "Enter an address",
    );
    expect(description.properties.get(PropertyCode.Part)).toBe(
      "field-description",
    );
    expect(error.properties.get(PropertyCode.Part)).toBe("field-error");
    expect(
      new Set(
        [field, label, control, description, error].map((node) =>
          node.properties.get(PropertyCode.Scope),
        ),
      ).size,
    ).toBe(1);
    window.close();
  });

  test("selects one radio through its group without a native round trip", async () => {
    await app.whenReady();
    const selected: string[] = [];
    const window = new Window({
      title: "Radios",
      renderer: createRenderer(() =>
        createComponent(RadioGroup.Root, {
          defaultValue: "light",
          onValueChange: (next: string) => selected.push(next),
          get children() {
            return [
              createComponent(Radio.Root, { value: "light", children: "Light" }),
              createComponent(Radio.Root, { value: "dark", children: "Dark" }),
            ];
          },
        }),
      ),
    });

    const group = window.root.children[0]!;
    const light = group.children[0]!;
    const dark = group.children[1]!;
    expect(group.properties.get(PropertyCode.Part)).toBe("radio-group");
    expect(light.properties.get(PropertyCode.Part)).toBe("radio");
    expect(light.properties.get(PropertyCode.Checked)).toBe(true);
    expect(dark.properties.get(PropertyCode.Checked)).toBe(false);

    window._dispatchEvent("click", dark.id);
    expect(selected).toEqual(["dark"]);
    expect(light.properties.get(PropertyCode.Checked)).toBe(false);
    expect(dark.properties.get(PropertyCode.Checked)).toBe(true);
    window.close();
  });

  test("declares in-window dialog dismissal policy ahead of every core decision", async () => {
    await app.whenReady();
    const changes: Array<{ open: boolean; reason: string }> = [];
    const window = new Window({
      title: "Dialogs",
      renderer: createRenderer(() =>
        createComponent(AlertDialog.Root, {
          defaultOpen: false,
          onOpenChange: (open: boolean, details: { reason: string }) =>
            changes.push({ open, reason: details.reason }),
          get children() {
            return [
              createComponent(AlertDialog.Trigger, { children: "Delete" }),
              createComponent(AlertDialog.Portal, {
                get children() {
                  return [
                    createComponent(AlertDialog.Backdrop, {}),
                    createComponent(AlertDialog.Popup, {
                      get children() {
                        return [
                          createComponent(AlertDialog.Title, {
                            children: "Delete project?",
                          }),
                          createComponent(AlertDialog.Description, {
                            children: "This cannot be undone.",
                          }),
                          createComponent(AlertDialog.Close, {
                            "aria-label": "Cancel",
                            children: "Cancel",
                          }),
                        ];
                      },
                    }),
                  ];
                },
              }),
            ];
          },
        }),
      ),
    });

    const trigger = window.root.children[0]!;
    const portal = window.root.children[1]!;
    const popup = portal.children[1]!;
    const close = popup.children[2]!;

    expect(trigger.properties.get(PropertyCode.Part)).toBe("dialog-trigger");
    expect(trigger.properties.get(PropertyCode.Variant)).toBe("alertdialog");
    expect(portal.properties.get(PropertyCode.Part)).toBe("dialog");
    expect(portal.properties.get(PropertyCode.Open)).toBe(false);
    expect(popup.properties.get(PropertyCode.Part)).toBe("dialog-popup");
    // An alert dialog keeps Escape and blocks backdrop dismissal by default.
    expect(popup.properties.get(PropertyCode.DismissOnEscape)).toBe(true);
    expect(popup.properties.get(PropertyCode.DismissOnPointerOutside)).toBe(
      false,
    );
    expect(popup.properties.get(PropertyCode.DismissListener)).toBe(true);
    expect(close.properties.get(PropertyCode.AccessibilityLabel)).toBe(
      "Cancel",
    );

    window._dispatchEvent("click", trigger.id);
    expect(portal.properties.get(PropertyCode.Open)).toBe(true);
    window._dispatchEvent("dismiss", popup.id);
    expect(portal.properties.get(PropertyCode.Open)).toBe(false);
    window._dispatchEvent("click", trigger.id);
    window._dispatchEvent("click", close.id);
    expect(changes).toEqual([
      { open: true, reason: "trigger-press" },
      { open: false, reason: "dismiss" },
      { open: true, reason: "trigger-press" },
      { open: false, reason: "close-press" },
    ]);
    window.close();
  });

  test("declares a bounded popover-menu model and adopts core selection", async () => {
    await app.whenReady();
    const [open, setOpen] = createSignal(false);
    const selections: MenuSelectDetails[] = [];
    const items = [
      { type: "group" as const, label: "File" },
      { id: "open", label: "Open…", shortcut: "⌘O" },
      { type: "separator" as const },
      { type: "checkbox" as const, id: "sidebar", label: "Sidebar", checked: true },
      { id: "recent", label: "Recent", items: [{ id: "one", label: "One" }] },
    ];
    const window = new Window({
      title: "Popover menu",
      renderer: createRenderer(() =>
        createComponent(PopoverMenu.Root, {
          items,
          appearance: { width: 240, background: "#101014", highlightColor: "#ffffff" },
          get open() {
            return open();
          },
          onOpenChange: (next: boolean) => setOpen(next),
          onSelect: (details: MenuSelectDetails) => selections.push(details),
          placement: "bottom-end",
          gap: 6,
          get children() {
            return [
              createComponent(PopoverMenu.Trigger, { children: "Actions" }),
              createComponent(PopoverMenu.Popup, {}),
            ];
          },
        }),
      ),
    });

    const trigger = window.root.children[0]!;
    expect(trigger.properties.get(PropertyCode.Part)).toBe(
      "popover-menu-trigger",
    );
    expect(trigger.properties.get(PropertyCode.Open)).toBe(false);
    expect(window.root.children.length).toBe(1);

    window._dispatchEvent("click", trigger.id);
    expect(open()).toBe(true);

    const popup = window.root.children[1]!;
    expect(popup.properties.get(PropertyCode.Part)).toBe("popover-menu-popup");
    expect(popup.properties.get(PropertyCode.AnchorTarget)).toBe(
      String(trigger.id),
    );
    expect(popup.properties.get(PropertyCode.AnchorPlacement)).toBe(
      "bottom-end",
    );
    expect(popup.properties.get(PropertyCode.AnchorGap)).toBe(6);
    expect(popup.properties.get(PropertyCode.SelectListener)).toBe(true);
    expect(popup.properties.get(PropertyCode.DismissListener)).toBe(true);
    // The trigger relates to the mounted surface through the validated controls property.
    expect(trigger.properties.get(PropertyCode.Controls)).toBe(String(popup.id));
    expect(trigger.properties.get(PropertyCode.Open)).toBe(true);

    const declaration = JSON.parse(String(popup.properties.get(PropertyCode.Menu)));
    expect(declaration.width).toBe(240);
    expect(declaration.background).toBeTypeOf("number");
    expect(declaration.items.length).toBe(5);
    expect(declaration.items[4].items[0].id).toBe("one");

    // The core decides what an activation means; JavaScript only receives the declared id.
    window._dispatchEvent(
      "menuselect",
      popup.id,
      JSON.stringify({ id: "sidebar", checked: false }),
    );
    await Promise.resolve();
    await Promise.resolve();
    expect(selections).toEqual([{ id: "sidebar", checked: false }]);
    expect(open()).toBe(false);
    expect(window.root.children.length).toBe(1);
    window.close();
  });

  test("declares a bounded context-menu model on its secondary-click target", async () => {
    await app.whenReady();
    const selections: MenuSelectDetails[] = [];
    const window = new Window({
      title: "Context menu",
      renderer: createRenderer(() =>
        createComponent(ContextMenu.Root, {
          items: [
            { id: "cut", label: "Cut", shortcut: "⌘X" },
            { type: "separator" as const },
            { type: "radio" as const, id: "list", group: "view", label: "List", checked: true },
          ],
          appearance: { itemHeight: 26, mutedColor: "#8a8a8a", loop: false },
          onSelect: (details: MenuSelectDetails) => selections.push(details),
          get children() {
            return createComponent(ContextMenu.Trigger, { children: "Canvas" });
          },
        }),
      ),
    });

    const target = window.root.children[0]!;
    expect(target.properties.get(PropertyCode.Part)).toBe("context-menu-trigger");
    expect(target.properties.get(PropertyCode.SelectListener)).toBe(true);
    const declaration = JSON.parse(String(target.properties.get(PropertyCode.Menu)));
    expect(declaration.itemHeight).toBe(26);
    expect(declaration.loopFocus).toBe(false);
    expect(declaration.mutedColor).toBeTypeOf("number");
    expect(declaration.items[2]).toEqual({
      type: "radio",
      id: "list",
      group: "view",
      label: "List",
      checked: true,
    });

    window._dispatchEvent(
      "menuselect",
      target.id,
      JSON.stringify({ id: "cut" }),
    );
    expect(selections).toEqual([{ id: "cut" }]);
    window.close();
  });

  test("bounds declared menu models before they cross N-API", () => {
    expect(() =>
      encodeMenu([
        { id: "big", label: "x".repeat(MAX_MENU_JSON_BYTES + 1) },
      ]),
    ).toThrow("bounded");
    expect(JSON.parse(encodeMenu(undefined)).items).toEqual([]);
    expect(menuSelectionFromEvent(new QuickGuiEvent("menuselect", createElement("view")))).toBe(
      undefined,
    );
    expect(
      menuSelectionFromEvent(
        new QuickGuiEvent("menuselect", createElement("view"), "not json"),
      ),
    ).toBe(undefined);
    expect(Object.keys(PopoverMenu)).toEqual(["Root", "Trigger", "Popup"]);
    expect(Object.keys(ContextMenu)).toEqual(["Root", "Trigger"]);
  });

  test("projects CSS grid templates, flow, and item placement", () => {
    const grid = createComponent(View, {
      style: {
        display: "grid",
        gridTemplateColumns: ["200px", "1fr", "minmax(120px, 2fr)"],
        gridTemplateRows: 3,
        gridAutoFlow: "column dense",
      },
    });
    expect(grid.properties.get(PropertyCode.GridTemplateColumns)).toBe(
      "200px 1fr minmax(120px, 2fr)",
    );
    expect(grid.properties.get(PropertyCode.GridTemplateRows)).toBe(3);
    expect(grid.properties.get(PropertyCode.GridAutoFlow)).toBe("column dense");

    const spanned = createComponent(View, {
      style: { gridColumn: "2 / span 3", gridRow: "span 2" },
    });
    // `2 / span 3` is lines 2 through 5, which is the placement the core exposes.
    expect(spanned.properties.get(PropertyCode.GridColumnStart)).toBe(2);
    expect(spanned.properties.get(PropertyCode.GridColumnEnd)).toBe(5);
    expect(spanned.properties.get(PropertyCode.GridColumnSpan)).toBe(undefined);
    expect(spanned.properties.get(PropertyCode.GridRowSpan)).toBe(2);

    const lines = createComponent(View, { style: { gridColumn: "1 / 4" } });
    expect(lines.properties.get(PropertyCode.GridColumnStart)).toBe(1);
    expect(lines.properties.get(PropertyCode.GridColumnEnd)).toBe(4);
  });

  test("declares the complete paint transition the Rust core supports", () => {
    const shorthand = createComponent(View, {
      style: { transition: "opacity 180ms ease-out, box-shadow 180ms ease-out" },
    });
    expect(shorthand.properties.get(PropertyCode.Transition)).toBe(180);
    expect(shorthand.properties.get(PropertyCode.TransitionProperties)).toBe(
      "opacity,box-shadow",
    );
    expect(shorthand.properties.get(PropertyCode.TransitionEasing)).toBe(
      "ease-out",
    );

    const declared = createComponent(View, {
      style: {
        transition: {
          properties: ["background-color", "color"],
          duration: "0.2s",
          easing: "ease",
          maxFps: 30,
        },
      },
    });
    expect(declared.properties.get(PropertyCode.TransitionDuration)).toBe(200);
    expect(declared.properties.get(PropertyCode.TransitionProperties)).toBe(
      "background-color,color",
    );
    // `ease` is the core's own ease-in-out curve.
    expect(declared.properties.get(PropertyCode.TransitionEasing)).toBe(
      "ease-in-out",
    );
    expect(declared.properties.get(PropertyCode.TransitionMaxFps)).toBe(30);

    const rejected = createElement("view");
    expect(() => setProp(rejected, "transition", "left 100ms")).toThrow(
      "cannot transition",
    );
    expect(() => setProp(rejected, "transition", "opacity 100ms 40ms")).toThrow(
      "delay",
    );
  });

  test("creates retained image and shader nodes with bounded declarations", () => {
    const image = createComponent(Image, {
      source: "/assets/logo.png",
      fit: "cover",
      style: { width: 64, height: 64 },
    });
    expect(image.tag).toBe(NativeNodeTag.Image);
    expect(image.properties.get(PropertyCode.Value)).toBe("/assets/logo.png");
    expect(image.properties.get(PropertyCode.ObjectFit)).toBe("cover");

    const shader = createComponent(Shader, {
      source: "fn quickgui_fragment(input: QuickGuiShaderInput) -> vec4<f32> { return vec4<f32>(1.0); }",
      shaderParameters: [
        [0.5, 0.25, 0, 1],
        [1, 0, 0, 1],
      ],
    });
    expect(shader.tag).toBe(NativeNodeTag.Shader);
    expect(
      JSON.parse(String(shader.properties.get(PropertyCode.ShaderParameters))),
    ).toEqual([0.5, 0.25, 0, 1, 1, 0, 0, 1]);

    const bounded = createElement("shader");
    expect(() =>
      setProp(bounded, "shaderParameters", new Array(20).fill(0)),
    ).toThrow("16 shader parameter floats");
  });

  test("declares progress, meter, and toggle state ahead of the core", () => {
    const progress = createComponent(Progress.Root, {
      value: 3,
      max: 12,
      valueText: "3 of 12 files",
      children: createComponent(Progress.Indicator, {}),
    });
    expect(progress.properties.get(PropertyCode.Part)).toBe("progress");
    expect(progress.properties.get(PropertyCode.Value)).toBe(3);
    expect(progress.properties.get(PropertyCode.Maximum)).toBe(12);
    expect(progress.properties.get(PropertyCode.ValueText)).toBe(
      "3 of 12 files",
    );

    const meter = createComponent(Meter.Root, {
      value: 20,
      min: 0,
      max: 100,
      low: 25,
      high: 75,
      optimum: 90,
    });
    expect(meter.properties.get(PropertyCode.Part)).toBe("meter");
    expect(meter.properties.get(PropertyCode.Low)).toBe(25);
    expect(meter.properties.get(PropertyCode.High)).toBe(75);
    expect(meter.properties.get(PropertyCode.Optimum)).toBe(90);

    let pressedChanges = 0;
    const toggle = createComponent(Toggle.Root, {
      defaultPressed: true,
      onPressedChange: () => {
        pressedChanges += 1;
      },
    });
    expect(toggle.properties.get(PropertyCode.Part)).toBe("toggle");
    expect(toggle.properties.get(PropertyCode.Pressed)).toBe(true);
    toggle.listeners.get("click")!(new QuickGuiEvent("click", toggle));
    expect(pressedChanges).toBe(1);
    expect(toggle.properties.get(PropertyCode.Pressed)).toBe(false);
    expect(Object.keys(Toggle)).toEqual(["Root", "Indicator"]);
  });

  test("declares every input listener ahead of the core decision", () => {
    const seen: string[] = [];
    const record = (name: string) => (event: QuickGuiEvent) => {
      seen.push(`${name}:${event.value ?? ""}`);
    };
    const node = createComponent(View, {
      tabIndex: 0,
      onKeyDown: record("keydown"),
      onKeyUp: record("keyup"),
      onMouseDown: record("mousedown"),
      onMouseUp: record("mouseup"),
      onMouseMove: record("mousemove"),
      onDoubleClick: record("dblclick"),
      onWheel: record("wheel"),
      onContextMenu: record("contextmenu"),
      onPinch: record("pinch"),
      onRotate: record("rotate"),
      onSmartMagnify: record("smartmagnify"),
      onPressure: record("pressure"),
      onFocus: record("focus"),
      onBlur: record("blur"),
    });

    for (const code of [
      PropertyCode.KeyDownListener,
      PropertyCode.KeyUpListener,
      PropertyCode.MouseDownListener,
      PropertyCode.MouseUpListener,
      PropertyCode.MouseMoveListener,
      PropertyCode.DoubleClickListener,
      PropertyCode.ScrollListener,
      PropertyCode.ContextMenuListener,
      PropertyCode.PinchListener,
      PropertyCode.RotationListener,
      PropertyCode.SmartMagnifyListener,
      PropertyCode.PressureListener,
      PropertyCode.FocusListener,
    ]) {
      expect(node.properties.get(code)).toBe(true);
    }

    const keyPayload = JSON.stringify({
      key: "s",
      repeat: false,
      shift: false,
      control: false,
      alt: false,
      meta: true,
    });
    node.listeners.get("keydown")!(
      new QuickGuiEvent("keydown", node, keyPayload),
    );
    expect(keyEventFromEvent(new QuickGuiEvent("keydown", node, keyPayload))).toEqual(
      {
        key: "s",
        repeat: false,
        shift: false,
        control: false,
        alt: false,
        meta: true,
      },
    );
    expect(seen).toEqual([`keydown:${keyPayload}`]);

    const wheel = new QuickGuiEvent(
      "wheel",
      node,
      JSON.stringify({
        x: 4,
        y: 8,
        deltaX: 0,
        deltaY: -24,
        precise: true,
        phase: "moved",
        shift: false,
        control: false,
        alt: false,
        meta: false,
      }),
    );
    expect(wheelEventFromEvent(wheel)?.deltaY).toBe(-24);
    expect(wheelEventFromEvent(wheel)?.precise).toBe(true);
    expect(
      mouseEventFromEvent(new QuickGuiEvent("mousedown", node, "not json")),
    ).toBe(undefined);
    expect(
      gestureEventFromEvent(
        new QuickGuiEvent("pressure", node, JSON.stringify({ stage: "force" })),
      )?.stage,
    ).toBe("force");
  });

  test("declares bounded accelerator keymaps and typed action ids", () => {
    let dispatched: string | undefined;
    const node = createComponent(View, {
      tabIndex: 0,
      keymap: { "CmdOrCtrl+S": "save", "CmdOrCtrl+Shift+P": "palette" },
      onAction: (event: QuickGuiEvent) => {
        dispatched = actionFromEvent(event);
      },
    });
    expect(node.properties.get(PropertyCode.ActionListener)).toBe(true);
    expect(JSON.parse(String(node.properties.get(PropertyCode.Keymap)))).toEqual({
      "CmdOrCtrl+S": "save",
      "CmdOrCtrl+Shift+P": "palette",
    });
    node.listeners.get("action")!(new QuickGuiEvent("action", node, "save"));
    expect(dispatched).toBe("save");

    const invalid = createElement("view");
    expect(() => setProp(invalid, "keymap", { "Cmd+S": 3 })).toThrow(
      "binding ids must be strings",
    );
    expect(() =>
      setProp(invalid, "keymap", { "Cmd+S": "x".repeat(MAX_KEYMAP_JSON_BYTES) }),
    ).toThrow("bounded");
  });

  test("declares drag payloads and accepted drop kinds ahead of the native drag", () => {
    const source = createComponent(View, {
      draggable: { id: "row-7", text: "quickgui", files: [{ path: "/tmp/a.txt" }] },
      onDragStart: () => {},
      onDragEnd: () => {},
    });
    expect(source.properties.get(PropertyCode.DragListener)).toBe(true);
    expect(
      JSON.parse(String(source.properties.get(PropertyCode.Draggable))),
    ).toEqual({
      id: "row-7",
      text: "quickgui",
      files: [{ path: "/tmp/a.txt" }],
    });

    const target = createComponent(View, {
      dropKinds: ["local", "files"],
      onDrop: () => {},
      onFilesDropped: () => {},
    });
    expect(target.properties.get(PropertyCode.DropListener)).toBe(true);
    expect(
      JSON.parse(String(target.properties.get(PropertyCode.DropKinds))),
    ).toEqual(["local", "files"]);
    expect(
      dropEventFromEvent(
        new QuickGuiEvent(
          "filesdropped",
          target,
          JSON.stringify({ x: 0, y: 0, paths: ["/tmp/a.txt"], origin: "external" }),
        ),
      )?.paths,
    ).toEqual(["/tmp/a.txt"]);

    const invalid = createElement("view");
    expect(() => setProp(invalid, "dropKinds", ["text"])).toThrow(
      "drop kinds",
    );
    expect(() => setProp(invalid, "draggable", 7)).toThrow("declare its payload");
  });
});

describe("declared range, ordering, and roving-focus components", () => {
  test("declares slider bounds and adopts the value the core reports", async () => {
    await app.whenReady();
    let reported: readonly number[] | undefined;
    const window = new Window({
      title: "Slider",
      renderer: createRenderer(() => [
        createComponent(Slider.Root, {
          scope: "volume",
          defaultValue: [10, 60],
          min: 0,
          max: 100,
          step: 5,
          largeStep: 25,
          orientation: "horizontal",
          onValueChange: (values: readonly number[]) => {
            reported = values;
          },
          get children() {
            return [
              createComponent(Slider.Track, {
                scope: "volume",
                get children() {
                  return createComponent(Slider.Range, { scope: "volume" });
                },
              }),
              createComponent(Slider.Thumb, { scope: "volume", itemIndex: 0 }),
              createComponent(Slider.Thumb, { scope: "volume", itemIndex: 1 }),
            ];
          },
        }),
      ]),
    });

    const root = window.root.children[0]!;
    const track = root.children[0]!;
    const upper = root.children[2]!;
    expect(root.properties.get(PropertyCode.Part)).toBe("slider");
    expect(root.properties.get(PropertyCode.Scope)).toBe("volume");
    expect(root.properties.get(PropertyCode.Values)).toBe("[10,60]");
    expect(root.properties.get(PropertyCode.Minimum)).toBe(0);
    expect(root.properties.get(PropertyCode.Maximum)).toBe(100);
    expect(root.properties.get(PropertyCode.Step)).toBe(5);
    expect(root.properties.get(PropertyCode.LargeStep)).toBe(25);
    expect(root.properties.get(PropertyCode.ComponentChangeListener)).toBe(true);
    expect(track.properties.get(PropertyCode.Part)).toBe("slider-track");
    expect(track.children[0]!.properties.get(PropertyCode.Part)).toBe("slider-range");
    expect(upper.properties.get(PropertyCode.Part)).toBe("slider-thumb");
    expect(upper.properties.get(PropertyCode.ItemIndex)).toBe(1);

    // The core decides the snapped, ordered result; the renderer only adopts it.
    window._dispatchEvent(
      "componentchange",
      root.id,
      JSON.stringify({ values: [10, 90] }),
    );
    expect(reported).toEqual([10, 90]);
    expect(root.properties.get(PropertyCode.Values)).toBe("[10,90]");
    window.close();
  });

  test("declares splitter panes and toolbar and toggle-group navigation models", async () => {
    await app.whenReady();
    let sizes: readonly number[] | undefined;
    let active: string | undefined;
    let pressed: readonly string[] | undefined;
    const window = new Window({
      title: "Components",
      renderer: createRenderer(() => [
        createComponent(Splitter.Root, {
          scope: "panes",
          defaultValue: [200, 200],
          panes: [{ min: 80 }, { min: 80, collapsible: true }],
          step: 16,
          onSizesChange: (next: readonly number[]) => {
            sizes = next;
          },
          get children() {
            return [
              createComponent(Splitter.Pane, { scope: "panes", itemIndex: 0 }),
              createComponent(Splitter.Handle, { scope: "panes", itemIndex: 0 }),
              createComponent(Splitter.Pane, { scope: "panes", itemIndex: 1 }),
            ];
          },
        }),
        createComponent(Toolbar.Root, {
          scope: "actions",
          items: [{ value: "cut" }, { value: "copy", disabled: true }],
          defaultActive: "cut",
          loopFocus: true,
          onActiveChange: (next: string | undefined) => {
            active = next;
          },
          get children() {
            return [
              createComponent(Toolbar.Item, { scope: "actions", partValue: "cut" }),
              createComponent(Toolbar.Item, { scope: "actions", partValue: "copy" }),
            ];
          },
        }),
        createComponent(ToggleGroup.Root, {
          scope: "align",
          items: [{ value: "left" }, { value: "right" }],
          defaultValue: ["left"],
          variant: "multiple",
          onValueChange: (next: readonly string[]) => {
            pressed = next;
          },
          get children() {
            return [
              createComponent(ToggleGroup.Item, { scope: "align", partValue: "left" }),
              createComponent(ToggleGroup.Item, { scope: "align", partValue: "right" }),
            ];
          },
        }),
      ]),
    });

    const splitter = window.root.children[0]!;
    const toolbar = window.root.children[1]!;
    const group = window.root.children[2]!;
    expect(splitter.properties.get(PropertyCode.Part)).toBe("splitter");
    expect(splitter.properties.get(PropertyCode.Values)).toBe("[200,200]");
    expect(splitter.properties.get(PropertyCode.Items)).toBe(
      '[{"min":80},{"min":80,"collapsible":true}]',
    );
    expect(splitter.children[1]!.properties.get(PropertyCode.Part)).toBe("splitter-handle");
    expect(toolbar.properties.get(PropertyCode.Part)).toBe("toolbar");
    expect(toolbar.properties.get(PropertyCode.Items)).toBe(
      '[{"value":"cut"},{"value":"copy","disabled":true}]',
    );
    expect(toolbar.properties.get(PropertyCode.ActiveValue)).toBe("cut");
    expect(toolbar.properties.get(PropertyCode.LoopFocus)).toBe(true);
    expect(toolbar.children[0]!.properties.get(PropertyCode.PartValue)).toBe("cut");
    expect(group.properties.get(PropertyCode.Part)).toBe("toggle-group");
    expect(group.properties.get(PropertyCode.Variant)).toBe("multiple");
    expect(group.properties.get(PropertyCode.Values)).toBe('["left"]');

    window._dispatchEvent(
      "componentchange",
      splitter.id,
      JSON.stringify({ sizes: [216, 184] }),
    );
    window._dispatchEvent(
      "componentchange",
      toolbar.id,
      JSON.stringify({ active: "copy" }),
    );
    window._dispatchEvent(
      "componentchange",
      group.id,
      JSON.stringify({ pressed: ["left", "right"] }),
    );
    expect(sizes).toEqual([216, 184]);
    expect(active).toBe("copy");
    expect(pressed).toEqual(["left", "right"]);
    expect(splitter.properties.get(PropertyCode.Values)).toBe("[216,184]");
    expect(toolbar.properties.get(PropertyCode.ActiveValue)).toBe("copy");
    expect(group.properties.get(PropertyCode.Values)).toBe('["left","right"]');
    window.close();
  });

  test("keeps a controlled component declaration authoritative and bounds the payload", async () => {
    await app.whenReady();
    const [values, setValues] = createSignal<readonly number[]>([25]);
    let reported = 0;
    const window = new Window({
      title: "Controlled slider",
      renderer: createRenderer(() => [
        createComponent(Slider.Root, {
          scope: "controlled",
          get value() {
            return values();
          },
          onValueChange: () => {
            reported += 1;
          },
        }),
      ]),
    });

    const root = window.root.children[0]!;
    expect(root.properties.get(PropertyCode.Values)).toBe("[25]");
    // A reported value never overwrites a controlled declaration on its own.
    window._dispatchEvent(
      "componentchange",
      root.id,
      JSON.stringify({ values: [30] }),
    );
    expect(reported).toBe(1);
    expect(root.properties.get(PropertyCode.Values)).toBe("[25]");
    setValues([30]);
    await Promise.resolve();
    expect(root.properties.get(PropertyCode.Values)).toBe("[30]");

    // A malformed payload is ignored rather than throwing into the renderer.
    window._dispatchEvent("componentchange", root.id, "{not json");
    expect(reported).toBe(1);

    expect(() =>
      solid.componentChangeFromEvent(
        new QuickGuiEvent("componentchange", root, "{oops"),
      ),
    ).not.toThrow();
    expect(
      solid.componentChangeFromEvent(
        new QuickGuiEvent("componentchange", root, '{"values":[1]}'),
      ),
    ).toEqual({ values: [1] });
    window.close();
  });
});

describe("declared option sources, virtual collections, and stateful fields", () => {
  test("declares a bounded select source and adopts the value the core commits", async () => {
    await app.whenReady();
    let value: string | undefined;
    let open: boolean | undefined;
    let committed: solid.CommitDetails | undefined;
    const window = new Window({
      title: "Select",
      renderer: createRenderer(() => [
        createComponent(Select.Root, {
          scope: "theme",
          ariaLabel: "Theme",
          defaultValue: "light",
          filterMode: "fuzzy",
          items: [
            { value: "light", label: "Light" },
            { value: "dark", label: "Dark", detail: "⌘D" },
          ],
          appearance: { width: 240, rowHeight: 28, background: "#101014" },
          onValueChange: (next: string | undefined) => {
            value = next;
          },
          onOpenChange: (next: boolean) => {
            open = next;
          },
          onCommit: (details: solid.CommitDetails) => {
            committed = details;
          },
        }),
      ]),
    });

    const root = window.root.children[0]!;
    expect(root.tag).toBe(NativeNodeTag.Button);
    expect(root.properties.get(PropertyCode.Part)).toBe("select");
    expect(root.properties.get(PropertyCode.Scope)).toBe("theme");
    expect(root.properties.get(PropertyCode.ActiveValue)).toBe("light");
    expect(root.properties.get(PropertyCode.FilterMode)).toBe("fuzzy");
    expect(root.properties.get(PropertyCode.Options)).toBe(
      JSON.stringify([
        { value: "light", label: "Light" },
        { value: "dark", label: "Dark", detail: "⌘D" },
      ]),
    );
    // Declared colors travel packed, exactly like every other declared paint value.
    expect(root.properties.get(PropertyCode.Appearance)).toBe(
      JSON.stringify({
        width: 240,
        rowHeight: 28,
        background: parseColor("#101014"),
      }),
    );
    expect(root.properties.get(PropertyCode.ComponentChangeListener)).toBe(true);
    expect(root.properties.get(PropertyCode.CommitListener)).toBe(true);

    window._dispatchEvent(
      "componentchange",
      root.id,
      JSON.stringify({ value: "dark", open: false }),
    );
    expect(value).toBe("dark");
    expect(open).toBe(false);
    expect(root.properties.get(PropertyCode.ActiveValue)).toBe("dark");

    window._dispatchEvent("commit", root.id, JSON.stringify({ value: "dark" }));
    expect(committed).toEqual({ value: "dark" });
    window.close();
  });

  test("declares combobox and autocomplete inputs with child option nodes", async () => {
    await app.whenReady();
    let inputValue: string | undefined;
    const window = new Window({
      title: "Pickers",
      renderer: createRenderer(() => [
        createComponent(Combobox.Root, {
          scope: "fruit",
          inputValue: "ap",
          placeholder: "Fruit",
          get children() {
            return [
              createComponent(Combobox.Option, {
                partValue: "apple",
                label: "Apple",
                group: "recent",
              }),
              createComponent(Combobox.Option, {
                partValue: "banana",
                label: "Banana",
                disabled: true,
              }),
            ];
          },
        }),
        createComponent(Autocomplete.Root, {
          scope: "search",
          inputValue: "al",
          filterMode: "none",
          items: [{ value: "alpha" }],
          onInputValueChange: (next: string) => {
            inputValue = next;
          },
        }),
      ]),
    });

    const combobox = window.root.children[0]!;
    const autocomplete = window.root.children[1]!;
    expect(combobox.tag).toBe(NativeNodeTag.Input);
    expect(combobox.properties.get(PropertyCode.Part)).toBe("combobox");
    expect(combobox.properties.get(PropertyCode.InputValue)).toBe("ap");
    expect(combobox.properties.get(PropertyCode.Options)).toBeUndefined();
    expect(combobox.children).toHaveLength(2);
    expect(combobox.children[0]!.properties.get(PropertyCode.Part)).toBe("option");
    expect(combobox.children[0]!.properties.get(PropertyCode.PartValue)).toBe("apple");
    expect(combobox.children[0]!.properties.get(PropertyCode.Value)).toBe("Apple");
    expect(combobox.children[0]!.properties.get(PropertyCode.Group)).toBe("recent");
    expect(combobox.children[1]!.properties.get(PropertyCode.Disabled)).toBe(true);
    expect(autocomplete.properties.get(PropertyCode.Part)).toBe("autocomplete");
    expect(autocomplete.properties.get(PropertyCode.FilterMode)).toBe("none");

    window._dispatchEvent(
      "componentchange",
      autocomplete.id,
      JSON.stringify({ inputValue: "alp", open: true }),
    );
    expect(inputValue).toBe("alp");
    window.close();
  });

  test("declares a virtual table and adopts everything the core decides", async () => {
    await app.whenReady();
    let range: solid.VisibleRange | undefined;
    let selection: readonly (readonly number[])[] | undefined;
    let sort: solid.TableSortState | undefined;
    let widths: Readonly<Record<string, number>> | undefined;
    let order: readonly string[] | undefined;
    let edit: solid.TableEditEndDetails | undefined;
    let activated: solid.TableCell | undefined;
    const window = new Window({
      title: "Table",
      renderer: createRenderer(() => [
        createComponent(Table.Root, {
          scope: "files",
          rowCount: 1000,
          rowHeight: 24,
          headerHeight: 32,
          selectionMode: "multiple",
          selection: [[0, 2]],
          sort: { column: "name", direction: "descending" },
          editing: { row: 1, column: 0 },
          columns: [
            { id: "name", label: "Name", width: 160, sortable: true },
            { id: "size", label: "Size", track: "1fr", align: "end" },
          ],
          onVisibleRangeChange: (next: solid.VisibleRange) => {
            range = next;
          },
          onSelectionChange: (next: readonly (readonly number[])[]) => {
            selection = next;
          },
          onSortChange: (next: solid.TableSortState | undefined) => {
            sort = next;
          },
          onColumnResize: (next: Readonly<Record<string, number>>) => {
            widths = next;
          },
          onColumnReorder: (next: readonly string[]) => {
            order = next;
          },
          onEditEnd: (next: solid.TableEditEndDetails) => {
            edit = next;
          },
          onActivate: (cell: solid.TableCell) => {
            activated = cell;
          },
          get children() {
            return [
              createComponent(Table.Header, { column: "name" }),
              createComponent(Table.Row, {
                index: 0,
                get children() {
                  return createComponent(Table.Cell, { column: "name" });
                },
              }),
            ];
          },
        }),
      ]),
    });

    const root = window.root.children[0]!;
    expect(root.properties.get(PropertyCode.Part)).toBe("table");
    expect(root.properties.get(PropertyCode.RowCount)).toBe(1000);
    expect(root.properties.get(PropertyCode.RowHeight)).toBe(24);
    expect(root.properties.get(PropertyCode.HeaderHeight)).toBe(32);
    expect(root.properties.get(PropertyCode.SelectionMode)).toBe("multiple");
    expect(root.properties.get(PropertyCode.Selection)).toBe("[[0,2]]");
    expect(root.properties.get(PropertyCode.SortColumn)).toBe("name");
    expect(root.properties.get(PropertyCode.SortDirection)).toBe("descending");
    expect(root.properties.get(PropertyCode.Editing)).toBe('{"row":1,"column":0}');
    expect(root.properties.get(PropertyCode.Columns)).toBe(
      JSON.stringify([
        { id: "name", label: "Name", width: 160, sortable: true },
        { id: "size", label: "Size", track: "1fr", align: "end" },
      ]),
    );
    expect(root.children[0]!.properties.get(PropertyCode.Part)).toBe("table-header");
    expect(root.children[0]!.properties.get(PropertyCode.PartValue)).toBe("name");
    expect(root.children[1]!.properties.get(PropertyCode.Part)).toBe("table-row");
    expect(root.children[1]!.properties.get(PropertyCode.RowIndex)).toBe(0);
    expect(
      root.children[1]!.children[0]!.properties.get(PropertyCode.Part),
    ).toBe("table-cell");

    window._dispatchEvent(
      "componentchange",
      root.id,
      JSON.stringify({
        visibleRange: { start: 12, end: 40 },
        selectedRanges: [[3, 5]],
        sort: { column: "size", direction: "ascending" },
        columnWidths: { name: 168 },
        columnOrder: ["size", "name"],
        editEnded: { row: 1, column: 0, committed: true },
      }),
    );
    expect(range).toEqual({ start: 12, end: 40 });
    expect(selection).toEqual([[3, 5]]);
    expect(sort).toEqual({ column: "size", direction: "ascending" });
    expect(widths).toEqual({ name: 168 });
    expect(order).toEqual(["size", "name"]);
    expect(edit).toEqual({ row: 1, column: 0, committed: true });

    window._dispatchEvent("commit", root.id, JSON.stringify({ row: 4, column: 1 }));
    expect(activated).toEqual({ row: 4, column: 1 });
    window.close();
  });

  test("declares a lazy virtual tree and adopts the core's expansion and requests", async () => {
    await app.whenReady();
    let expanded: readonly string[] | undefined;
    let requested: string | undefined;
    let activated: string | undefined;
    const window = new Window({
      title: "Tree",
      renderer: createRenderer(() => [
        createComponent(Tree.Root, {
          scope: "explorer",
          rowHeight: 22,
          loadingLabel: "Fetching…",
          disclosure: "trailing",
          nodes: [
            { id: "src", label: "src", pending: true },
            { id: "readme", label: "README" },
          ],
          defaultExpanded: [],
          value: "readme",
          setChildren: { id: "src", children: [{ id: "main", label: "main.rs" }] },
          onExpandedChange: (next: readonly string[]) => {
            expanded = next;
          },
          onLoadChildren: (id: string) => {
            requested = id;
          },
          onActivate: (id: string) => {
            activated = id;
          },
          get children() {
            return createComponent(Tree.Row, { nodeId: "src" });
          },
        }),
      ]),
    });

    const root = window.root.children[0]!;
    expect(root.properties.get(PropertyCode.Part)).toBe("tree");
    expect(root.properties.get(PropertyCode.RowHeight)).toBe(22);
    expect(root.properties.get(PropertyCode.LoadingLabel)).toBe("Fetching…");
    expect(root.properties.get(PropertyCode.Disclosure)).toBe("trailing");
    expect(root.properties.get(PropertyCode.SelectedValue)).toBe("readme");
    expect(root.properties.get(PropertyCode.Expanded)).toBe("[]");
    expect(root.properties.get(PropertyCode.SetChildren)).toBe(
      '{"id":"src","children":[{"id":"main","label":"main.rs"}]}',
    );
    expect(root.children[0]!.properties.get(PropertyCode.Part)).toBe("tree-row");
    expect(root.children[0]!.properties.get(PropertyCode.PartValue)).toBe("src");

    window._dispatchEvent(
      "componentchange",
      root.id,
      JSON.stringify({ expanded: ["src"], loadChildren: "src" }),
    );
    expect(expanded).toEqual(["src"]);
    expect(requested).toBe("src");
    expect(root.properties.get(PropertyCode.Expanded)).toBe('["src"]');

    window._dispatchEvent("commit", root.id, JSON.stringify({ value: "readme" }));
    expect(activated).toBe("readme");
    window.close();
  });

  test("declares number, date, time, calendar, menubar, and toast declarations", async () => {
    await app.whenReady();
    let numeric: number | undefined;
    let valid = true;
    let day: string | undefined;
    let dismissed: readonly string[] | undefined;
    let openMenu: number | undefined;
    const window = new Window({
      title: "Fields",
      renderer: createRenderer(() => [
        createComponent(NumberField.Root, {
          scope: "quantity",
          defaultValue: 2,
          min: 0,
          max: 10,
          step: 2,
          precision: 1,
          onValueChange: (next: number | undefined, isValid: boolean) => {
            numeric = next;
            valid = isValid;
          },
          get children() {
            return [
              createComponent(NumberField.Input, { scope: "quantity" }),
              createComponent(NumberField.Increment, { scope: "quantity" }),
              createComponent(NumberField.Decrement, { scope: "quantity" }),
            ];
          },
        }),
        createComponent(DateField.Root, {
          scope: "due",
          defaultValue: "2026-09-03",
          min: "2000-01-01",
          format: "mdy",
          get children() {
            return createComponent(DateField.Segment, {
              scope: "due",
              segment: "year",
            });
          },
        }),
        createComponent(TimeField.Root, {
          scope: "alarm",
          defaultValue: "07:30",
          hour12: true,
          showSeconds: true,
          get children() {
            return createComponent(TimeField.Segment, {
              scope: "alarm",
              segment: "period",
            });
          },
        }),
        createComponent(Calendar.Root, {
          scope: "month",
          defaultValue: "2026-09-03",
          firstWeekday: 0,
          onFocusChange: (next: string) => {
            day = next;
          },
          get children() {
            return createComponent(Calendar.Week, {
              scope: "month",
              itemIndex: 0,
              get children() {
                return createComponent(Calendar.Day, {
                  scope: "month",
                  day: "2026-09-03",
                });
              },
            });
          },
        }),
        createComponent(Menubar.Root, {
          scope: "bar",
          count: 2,
          onOpenChange: (next: number | undefined) => {
            openMenu = next;
          },
          get children() {
            return createComponent(Menubar.Item, { scope: "bar", itemIndex: 0 });
          },
        }),
        createComponent(Toast.Viewport, {
          scope: "toasts",
          toasts: [{ id: "saved", title: "Saved", kind: "success", duration: 4000 }],
          onDismiss: (ids: readonly string[]) => {
            dismissed = ids;
          },
          get children() {
            return createComponent(Toast.Root, {
              scope: "toasts",
              toastId: "saved",
              get children() {
                return [
                  createComponent(Toast.Title, { scope: "toasts", toastId: "saved" }),
                  createComponent(Toast.Close, { scope: "toasts", toastId: "saved" }),
                ];
              },
            });
          },
        }),
      ]),
    });

    const [field, date, time, calendar, menubar, toasts] = window.root.children;
    expect(field!.properties.get(PropertyCode.Part)).toBe("number-field");
    expect(field!.properties.get(PropertyCode.Values)).toBe("[2]");
    expect(field!.properties.get(PropertyCode.Precision)).toBe(1);
    expect(field!.children[0]!.properties.get(PropertyCode.Part)).toBe(
      "number-field-input",
    );
    expect(field!.children[1]!.properties.get(PropertyCode.Part)).toBe(
      "number-field-increment",
    );

    expect(date!.properties.get(PropertyCode.CivilValue)).toBe("2026-09-03");
    expect(date!.properties.get(PropertyCode.CivilMinimum)).toBe("2000-01-01");
    expect(date!.properties.get(PropertyCode.SegmentOrder)).toBe("mdy");
    expect(date!.children[0]!.properties.get(PropertyCode.Segment)).toBe("year");

    expect(time!.properties.get(PropertyCode.CivilValue)).toBe("07:30");
    expect(time!.properties.get(PropertyCode.SegmentOrder)).toBe("h12");
    expect(time!.properties.get(PropertyCode.Variant)).toBe("seconds");
    expect(time!.children[0]!.properties.get(PropertyCode.Segment)).toBe("period");

    expect(calendar!.properties.get(PropertyCode.CivilValue)).toBe("2026-09-03");
    expect(calendar!.properties.get(PropertyCode.FirstWeekday)).toBe(0);
    expect(calendar!.children[0]!.properties.get(PropertyCode.Part)).toBe(
      "calendar-week",
    );
    expect(
      calendar!.children[0]!.children[0]!.properties.get(PropertyCode.CivilValue),
    ).toBe("2026-09-03");

    expect(menubar!.properties.get(PropertyCode.MenuCount)).toBe(2);
    expect(menubar!.properties.get(PropertyCode.Open)).toBe(false);
    expect(menubar!.children[0]!.properties.get(PropertyCode.Part)).toBe(
      "menubar-item",
    );

    expect(toasts!.properties.get(PropertyCode.Toasts)).toBe(
      JSON.stringify([
        { id: "saved", title: "Saved", kind: "success", duration: 4000 },
      ]),
    );
    expect(toasts!.children[0]!.properties.get(PropertyCode.PartValue)).toBe("saved");
    expect(
      toasts!.children[0]!.children[1]!.properties.get(PropertyCode.Part),
    ).toBe("toast-close");

    window._dispatchEvent(
      "componentchange",
      field!.id,
      JSON.stringify({ value: 42, text: "42", valid: false }),
    );
    expect(numeric).toBe(42);
    expect(valid).toBe(false);

    window._dispatchEvent(
      "componentchange",
      calendar!.id,
      JSON.stringify({ focused: "2026-09-04", month: "2026-09" }),
    );
    expect(day).toBe("2026-09-04");

    window._dispatchEvent(
      "componentchange",
      menubar!.id,
      JSON.stringify({ open: 1, focused: 1 }),
    );
    expect(openMenu).toBe(1);
    expect(menubar!.properties.get(PropertyCode.ItemIndex)).toBe(1);
    expect(menubar!.properties.get(PropertyCode.Open)).toBe(true);

    window._dispatchEvent(
      "componentchange",
      toasts!.id,
      JSON.stringify({ dismissed: ["saved"] }),
    );
    expect(dismissed).toEqual(["saved"]);
    window.close();
  });

  test("refuses an option or collection declaration past the boundary's own bounds", async () => {
    await app.whenReady();
    const window = new Window({ title: "Bounds", renderer: createRenderer(() => []) });
    const node = createElement("view");
    insertNode(window.root, node);
    expect(() =>
      setProp(node, "options", [
        { value: "a", label: "x".repeat(MAX_OPTIONS_JSON_BYTES) },
      ]),
    ).toThrow(/option source/);
    expect(() =>
      setProp(node, "columns", [
        { id: "a", label: "x".repeat(MAX_COLLECTION_JSON_BYTES) },
      ]),
    ).toThrow(/collection/);
    window.close();
  });

  test("projects extended text styling onto the core's own typography declarations", () => {
    const label = createComponent(Text, {
      style: {
        textAlign: "center",
        letterSpacing: "0.4px",
        wordSpacing: 2,
        textTransform: "uppercase",
        textShadow: "0 2px 4px #00000080",
        textDecoration: "underline line-through",
        textDecorationColor: "#2266dd",
        textDecorationStyle: "wavy",
        textDecorationThickness: "2px",
        wordBreak: "break-all",
        overflowWrap: "anywhere",
        hyphens: "manual",
        textDirection: "rtl",
      },
      children: "Extended text",
    });

    expect(label.properties.get(PropertyCode.TextAlign)).toBe("center");
    expect(label.properties.get(PropertyCode.LetterSpacing)).toBe(0.4);
    expect(label.properties.get(PropertyCode.WordSpacing)).toBe(2);
    expect(label.properties.get(PropertyCode.TextTransform)).toBe("uppercase");
    expect(label.properties.get(PropertyCode.TextShadow)).toBe(
      "0 2px 4px #00000080",
    );
    expect(label.properties.get(PropertyCode.TextDecorationLine)).toBe(
      "underline line-through",
    );
    expect(label.properties.get(PropertyCode.TextDecorationColor)).toBe(
      parseColor("#2266dd"),
    );
    expect(label.properties.get(PropertyCode.TextDecorationStyle)).toBe("wavy");
    expect(label.properties.get(PropertyCode.TextDecorationThickness)).toBe(2);
    expect(label.properties.get(PropertyCode.WordBreak)).toBe("break-all");
    expect(label.properties.get(PropertyCode.OverflowWrap)).toBe("anywhere");
    expect(label.properties.get(PropertyCode.Hyphens)).toBe("manual");
    expect(label.properties.get(PropertyCode.TextDirection)).toBe("rtl");

    // `start` and `end` stay logical so an RTL subtree can resolve them in the core.
    const logical = createComponent(Text, {
      style: { textAlign: "end" },
      children: "logical",
    });
    expect(logical.properties.get(PropertyCode.TextAlign)).toBe("end");

    // A text shadow may also be declared as the object form the Rust binding deserializes.
    const declared = createComponent(Text, {
      style: { textShadow: { offsetX: 0, offsetY: 1, blur: 3 } },
      children: "object",
    });
    expect(
      JSON.parse(String(declared.properties.get(PropertyCode.TextShadow))),
    ).toEqual({ offsetX: 0, offsetY: 1, blur: 3 });
  });

  test("projects direction-relative layout and logical spacing", () => {
    const panel = createComponent(View, {
      style: {
        direction: "rtl",
        paddingStart: 12,
        paddingEnd: "4px",
        marginStart: 6,
        marginEnd: 2,
        borderStartWidth: "3px",
        borderEndWidth: 1,
      },
    });

    expect(panel.properties.get(PropertyCode.Direction)).toBe("rtl");
    expect(panel.properties.get(PropertyCode.PaddingStart)).toBe(12);
    expect(panel.properties.get(PropertyCode.PaddingEnd)).toBe(4);
    expect(panel.properties.get(PropertyCode.MarginStart)).toBe(6);
    expect(panel.properties.get(PropertyCode.MarginEnd)).toBe(2);
    expect(panel.properties.get(PropertyCode.BorderStartWidth)).toBe(3);
    expect(panel.properties.get(PropertyCode.BorderEndWidth)).toBe(1);
  });

  test("routes one background declaration to either a color or a bounded gradient", () => {
    const solid = createComponent(View, { style: { background: "#101828" } });
    expect(solid.properties.get(PropertyCode.BackgroundColor)).toBe(
      parseColor("#101828"),
    );
    expect(solid.properties.has(PropertyCode.BackgroundGradient)).toBe(false);

    const gradient = createComponent(View, {
      style: { background: "linear-gradient(135deg, #0f172a, #38bdf8)" },
    });
    expect(gradient.properties.get(PropertyCode.BackgroundGradient)).toBe(
      "linear-gradient(135deg, #0f172a, #38bdf8)",
    );
    expect(gradient.properties.has(PropertyCode.BackgroundColor)).toBe(false);

    const declared = createComponent(View, {
      style: {
        background: {
          type: "radial",
          shape: "circle",
          center: { x: 0.3, y: 0.2 },
          stops: [{ color: "#1d4ed8", position: 0 }, "#0f172a"],
        },
      },
    });
    expect(
      JSON.parse(
        String(declared.properties.get(PropertyCode.BackgroundGradient)),
      ),
    ).toEqual({
      type: "radial",
      shape: "circle",
      center: { x: 0.3, y: 0.2 },
      stops: [{ color: "#1d4ed8", position: 0 }, "#0f172a"],
    });

    // Every state that the core's `ElementStateStyle` can swap a gradient in follows the same rule.
    const stateful = createComponent(Button, {
      style: {
        hoverBackground: "linear-gradient(90deg, #111827, #334155)",
        activeBackground: "#0b1220",
        focusBackground: "conic-gradient(from 90deg, #1d4ed8, #0f172a)",
      },
      children: "Run",
    });
    expect(stateful.properties.get(PropertyCode.HoverBackgroundGradient)).toBe(
      "linear-gradient(90deg, #111827, #334155)",
    );
    expect(stateful.properties.get(PropertyCode.ActiveBackgroundColor)).toBe(
      parseColor("#0b1220"),
    );
    expect(stateful.properties.get(PropertyCode.FocusBackgroundGradient)).toBe(
      "conic-gradient(from 90deg, #1d4ed8, #0f172a)",
    );
  });

  test("projects per-corner radii, border and outline rings, filters, transforms, and blending", () => {
    const card = createComponent(View, {
      style: {
        borderRadius: "16px 4px 16px 4px",
        borderTopLeftRadius: 20,
        borderStyle: "dashed",
        outline: "2px dotted #38bdf8",
        outlineOffset: 3,
        backgroundImage: "/assets/paper.png",
        backgroundSize: "cover",
        backgroundRepeat: "no-repeat",
        backgroundPosition: "center",
        filter: ["saturate(1.4)", "blur(2px)"],
        backdropFilter: "blur(18px) brightness(1.1)",
        transform: "rotate(3deg) scale(1.02)",
        transformOrigin: "left top",
        mixBlendMode: "multiply",
      },
    });

    expect(card.properties.get(PropertyCode.BorderRadius)).toBe(
      "16px 4px 16px 4px",
    );
    expect(card.properties.get(PropertyCode.BorderTopLeftRadius)).toBe(20);
    expect(card.properties.get(PropertyCode.BorderStyle)).toBe("dashed");
    expect(card.properties.get(PropertyCode.OutlineWidth)).toBe(2);
    expect(card.properties.get(PropertyCode.OutlineStyle)).toBe("dotted");
    expect(card.properties.get(PropertyCode.OutlineColor)).toBe(
      parseColor("#38bdf8"),
    );
    expect(card.properties.get(PropertyCode.OutlineOffset)).toBe(3);
    expect(card.properties.get(PropertyCode.BackgroundImage)).toBe(
      "/assets/paper.png",
    );
    expect(card.properties.get(PropertyCode.BackgroundSize)).toBe("cover");
    expect(card.properties.get(PropertyCode.BackgroundRepeat)).toBe(
      "no-repeat",
    );
    expect(card.properties.get(PropertyCode.BackgroundPosition)).toBe("center");
    expect(card.properties.get(PropertyCode.Filter)).toBe(
      "saturate(1.4) blur(2px)",
    );
    expect(card.properties.get(PropertyCode.BackdropFilter)).toBe(
      "blur(18px) brightness(1.1)",
    );
    expect(card.properties.get(PropertyCode.Transform)).toBe(
      "rotate(3deg) scale(1.02)",
    );
    expect(card.properties.get(PropertyCode.TransformOrigin)).toBe("left top");
    expect(card.properties.get(PropertyCode.MixBlendMode)).toBe("multiply");

    const uniform = createComponent(View, { style: { borderRadius: "12px" } });
    expect(uniform.properties.get(PropertyCode.BorderRadius)).toBe(12);

    const cleared = createComponent(View, { style: { outline: "none" } });
    expect(cleared.properties.get(PropertyCode.OutlineStyle)).toBe("none");
  });

  test("projects hover, active, and focus state styling the core supports", () => {
    const tile = createComponent(Button, {
      style: {
        hoverTransform: "translate(0, -2px)",
        hoverOutline: "2px solid #f8fafc",
        activeTransform: { a: 0.98, b: 0, c: 0, d: 0.98, tx: 0, ty: 0 },
        activeOutline: "1px solid #94a3b8",
        focusBackgroundColor: "#1f2937",
        focusColor: "#f8fafc",
        focusOutline: "2px solid #60a5fa",
        focusTransform: "scale(1.01)",
      },
      children: "Apply",
    });

    expect(tile.properties.get(PropertyCode.HoverTransform)).toBe(
      "translate(0, -2px)",
    );
    expect(tile.properties.get(PropertyCode.HoverOutline)).toBe(
      "2px solid #f8fafc",
    );
    expect(
      JSON.parse(String(tile.properties.get(PropertyCode.ActiveTransform))),
    ).toEqual({ a: 0.98, b: 0, c: 0, d: 0.98, tx: 0, ty: 0 });
    expect(tile.properties.get(PropertyCode.ActiveOutline)).toBe(
      "1px solid #94a3b8",
    );
    expect(tile.properties.get(PropertyCode.FocusBackgroundColor)).toBe(
      parseColor("#1f2937"),
    );
    expect(tile.properties.get(PropertyCode.FocusColor)).toBe(
      parseColor("#f8fafc"),
    );
    expect(tile.properties.get(PropertyCode.FocusOutline)).toBe(
      "2px solid #60a5fa",
    );
    expect(tile.properties.get(PropertyCode.FocusTransform)).toBe("scale(1.01)");
  });

  test("projects sticky positioning, horizontal scrolling, and scroll snapping", () => {
    const header = createComponent(View, {
      style: { position: "sticky", top: 0 },
    });
    expect(header.properties.get(PropertyCode.Position)).toBe("sticky");
    expect(header.properties.get(PropertyCode.Top)).toBe(0);

    const rail = createComponent(View, {
      style: {
        overflowX: "scroll",
        scrollSnapType: "x mandatory",
      },
    });
    expect(rail.properties.get(PropertyCode.OverflowX)).toBe("scroll");
    expect(rail.properties.get(PropertyCode.ScrollSnapType)).toBe(
      "x mandatory",
    );

    const page = createComponent(View, {
      style: { scrollSnapAlign: "start", scrollSnapStop: "always" },
    });
    expect(page.properties.get(PropertyCode.ScrollSnapAlign)).toBe("start");
    expect(page.properties.get(PropertyCode.ScrollSnapStop)).toBe("always");
  });

  test("bounds every declared styling string at the JavaScript boundary", () => {
    const overlong = createElement("view");
    for (const name of [
      "filter",
      "backdropFilter",
      "transform",
      "textShadow",
      "hoverOutline",
      "focusTransform",
    ]) {
      expect(() =>
        setProp(overlong, name, `a${"b".repeat(MAX_STYLE_DECLARATION_BYTES)}`),
      ).toThrow("style declarations");
    }
    expect(() =>
      setProp(
        overlong,
        "background",
        `linear-gradient(90deg, ${"#000000, ".repeat(MAX_STYLE_DECLARATION_BYTES)}#ffffff)`,
      ),
    ).toThrow("style declarations");
    expect(() => setProp(overlong, "borderRadius", "1 2 3 4 5")).toThrow(
      "one to four corner radii",
    );
    expect(() => setProp(overlong, "outline", "2px solid chartreuse")).toThrow(
      "unsupported QuickGUI color",
    );
  });

  test("clears extended styling when a declaration is withdrawn", () => {
    const node = createElement("view");
    setProp(node, "background", "linear-gradient(90deg, #000000, #ffffff)");
    expect(node.properties.get(PropertyCode.BackgroundGradient)).toBeTypeOf(
      "string",
    );
    setProp(node, "background", "#101828");
    expect(node.properties.has(PropertyCode.BackgroundGradient)).toBe(false);
    setProp(node, "background", null);
    expect(node.properties.has(PropertyCode.BackgroundColor)).toBe(false);

    setProp(node, "outline", "2px solid #38bdf8");
    expect(node.properties.get(PropertyCode.OutlineWidth)).toBe(2);
    setProp(node, "outline", null);
    expect(node.properties.has(PropertyCode.OutlineWidth)).toBe(false);
    expect(node.properties.has(PropertyCode.OutlineColor)).toBe(false);
  });
});
