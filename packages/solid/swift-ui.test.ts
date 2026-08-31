import { describe, expect, test } from "bun:test";
import {
  app,
  NativeNodeTag,
  PropertyCode,
  QuickGuiEvent,
  Window,
  type NativeNode,
} from "@quickgui/native";
import { createComponent, createRenderer, Text, View } from "./index.ts";
import {
  Button,
  Host,
  Popover,
  PopoverContent,
  PopoverTrigger,
  QuickGUIHostView,
} from "./swift-ui.ts";
import {
  buttonBorderShape,
  buttonStyle,
  controlSize,
  disabled,
  labelStyle,
  tint,
} from "./swift-ui/modifiers.ts";

describe("SwiftUI Solid bridge", () => {
  test("builds a retained Host and native Button description", () => {
    let presses = 0;
    const button = createComponent(Button, {
      label: "Save changes",
      systemImage: "square.and.arrow.down",
      role: "default",
      target: "save",
      testID: "save-button",
      modifiers: [
        buttonStyle("glass"),
        controlSize("large"),
        buttonBorderShape("roundedRectangle", 14),
        labelStyle("titleAndIcon"),
        tint("#3366ffff"),
        disabled(false),
      ],
      onPress: () => presses++,
    });
    const host = createComponent(Host, {
      matchContents: true,
      children: button,
    });

    expect(host.tag).toBe(NativeNodeTag.SwiftUIHost);
    expect(
      host.properties.get(PropertyCode.SwiftUIMatchContentsHorizontal),
    ).toBe(true);
    expect(host.properties.get(PropertyCode.SwiftUIMatchContentsVertical)).toBe(
      true,
    );
    expect(host.children).toEqual([button]);
    expect(button.tag).toBe(NativeNodeTag.SwiftUIButton);
    expect(button.properties.get(PropertyCode.Value)).toBe("Save changes");
    expect(button.properties.get(PropertyCode.SwiftUISystemImage)).toBe(
      "square.and.arrow.down",
    );
    expect(button.properties.get(PropertyCode.SwiftUITarget)).toBe("save");
    expect(button.properties.get(PropertyCode.SwiftUITestId)).toBe(
      "save-button",
    );
    expect(
      JSON.parse(
        button.properties.get(PropertyCode.SwiftUIModifiers) as string,
      ),
    ).toEqual([
      { $type: "buttonStyle", style: "glass" },
      { $type: "controlSize", size: "large" },
      {
        $type: "buttonBorderShape",
        shape: "roundedRectangle",
        cornerRadius: 14,
      },
      { $type: "labelStyle", style: "titleAndIcon" },
      { $type: "tint", color: "#3366ffff" },
      { $type: "disabled", disabled: false },
    ]);
    expect(button.properties.get(PropertyCode.ClickListener)).toBe(true);

    button.listeners.get("click")!(new QuickGuiEvent("click", button));
    expect(presses).toBe(1);
  });

  test("supports per-axis intrinsic sizing", () => {
    const host = createComponent(Host, {
      matchContents: { vertical: true },
      children: createComponent(Button, { label: "Save" }),
    });

    expect(
      host.properties.has(PropertyCode.SwiftUIMatchContentsHorizontal),
    ).toBe(false);
    expect(host.properties.get(PropertyCode.SwiftUIMatchContentsVertical)).toBe(
      true,
    );
  });

  test("composes a rendered trigger with controlled native Popover behavior", () => {
    let isPresented = false;
    const changes: boolean[] = [];
    let renderedTrigger: NativeNode | undefined;
    const popover = createComponent(Popover, {
      get isPresented() {
        return isPresented;
      },
      attachmentAnchor: "bottom",
      arrowEdge: "top",
      onIsPresentedChange(presented) {
        isPresented = presented;
        changes.push(presented);
      },
      get children() {
        const button = createComponent(Button, { label: "Open" });
        return [
          createComponent(PopoverTrigger, {
            render: button,
            ref: (node) => {
              renderedTrigger = node;
            },
          }),
          createComponent(PopoverContent, {
            children: createComponent(Button, { label: "Inside" }),
          }),
        ];
      },
    });

    expect(popover.tag).toBe(NativeNodeTag.SwiftUIPopover);
    expect(popover.properties.has(PropertyCode.SwiftUIIsPresented)).toBe(false);
    expect(popover.properties.get(PropertyCode.SwiftUIAttachmentAnchor)).toBe(
      "bottom",
    );
    expect(popover.properties.get(PropertyCode.SwiftUIArrowEdge)).toBe("top");
    expect(
      popover.properties.get(PropertyCode.SwiftUIPresentationListener),
    ).toBe(true);
    expect(popover.children.map((child) => child.tag)).toEqual([
      NativeNodeTag.SwiftUIPopoverTrigger,
      NativeNodeTag.SwiftUIPopoverContent,
    ]);
    const trigger = popover.children[0]!;
    const button = trigger.children[0]!;
    expect(renderedTrigger).toBe(button);
    expect(button.tag).toBe(NativeNodeTag.SwiftUIButton);
    expect(trigger.properties.get(PropertyCode.ClickListener)).toBe(true);

    const press = new QuickGuiEvent("click", button);
    let current: NativeNode | undefined = button;
    while (current) {
      press.currentTarget = current;
      current.listeners.get("click")?.(press);
      if (press.propagationStopped) break;
      current = current.parent;
    }
    expect(changes).toEqual([true]);

    popover.listeners.get("presentationchange")!(
      new QuickGuiEvent("presentationchange", popover, "false"),
    );
    expect(changes).toEqual([true, false]);
  });

  test("lets the rendered trigger prevent Popover presentation", () => {
    const changes: boolean[] = [];
    const popover = createComponent(Popover, {
      isPresented: false,
      onIsPresentedChange: (presented) => changes.push(presented),
      get children() {
        return [
          createComponent(PopoverTrigger, {
            render: createComponent(Button, {
              label: "Open",
              onPress: (event) => event.preventDefault(),
            }),
          }),
          createComponent(PopoverContent, {
            children: createComponent(Button, { label: "Inside" }),
          }),
        ];
      },
    });
    const button = popover.children[0]!.children[0]!;
    const press = new QuickGuiEvent("click", button);
    let current: NativeNode | undefined = button;
    while (current) {
      press.currentTarget = current;
      current.listeners.get("click")?.(press);
      if (press.propagationStopped) break;
      current = current.parent;
    }

    expect(press.defaultPrevented).toBe(true);
    expect(changes).toEqual([]);
  });

  test("owns an ordinary QuickGUI renderer for QuickGUIHostView", async () => {
    await app.whenReady();
    let reverseHost: NativeNode | undefined;
    const owner = new Window({
      title: "SwiftUI reverse host test",
      width: 320,
      height: 180,
      renderer: createRenderer(() =>
        createComponent(Host, {
          matchContents: true,
          children: createComponent(QuickGUIHostView, {
            width: 220,
            height: 96,
            ref: (node) => {
              reverseHost = node;
            },
            children: createComponent(View, {
              children: createComponent(Text, {
                children: "Ordinary QuickGUI",
              }),
            }),
          }),
        }),
      ),
    });

    expect(reverseHost?.tag).toBe(NativeNodeTag.SwiftUIQuickGUIHost);
    const embeddedId = reverseHost?.properties.get(
      PropertyCode.SwiftUIEmbeddedWindow,
    );
    expect(typeof embeddedId).toBe("number");
    const embedded = app.windows.get(embeddedId as number);
    expect(embedded).toBeDefined();
    expect(
      embedded?.root.children[0]?.children[0]?.children[0]?.children[0]?.text,
    ).toBe("Ordinary QuickGUI");

    owner.close();
    expect(embedded?.closed).toBe(true);
  });
});
