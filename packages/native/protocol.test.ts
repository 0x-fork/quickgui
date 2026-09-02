import { describe, expect, test } from "bun:test";
import {
  MAX_COMPONENT_ITEMS,
  MAX_COMPONENT_JSON_BYTES,
  MAX_COMPONENT_VALUE_BYTES,
  MAX_COMPONENT_VALUES,
  MAX_DRAG_JSON_BYTES,
  MAX_KEYMAP_JSON_BYTES,
  MAX_MENU_JSON_BYTES,
  MAX_TOOLTIP_TEXT_BYTES,
  MutationBatch,
  NativeNodeTag,
  NativePart,
  NO_ANCHOR,
  PropertyCode,
  PROTOCOL_VERSION,
} from "./protocol.ts";

describe("binary mutation protocol", () => {
  test("writes one bounded little-endian batch", () => {
    const batch = new MutationBatch();
    batch.createElement(1, NativeNodeTag.View);
    batch.createText(2, "héllo");
    batch.setProperty(1, PropertyCode.Width, 320);
    batch.insert(1, 2);
    const bytes = batch.finish();
    const view = new DataView(bytes.buffer, bytes.byteOffset, bytes.byteLength);

    expect(bytes.subarray(0, 4).toString("utf8")).toBe("QGMB");
    expect(view.getUint16(4, true)).toBe(PROTOCOL_VERSION);
    expect(view.getUint32(6, true)).toBe(4);
    expect(view.getUint32(bytes.byteLength - 4, true)).toBe(NO_ANCHOR);
  });

  test("rejects non-finite numeric properties", () => {
    const batch = new MutationBatch();
    expect(() => batch.setProperty(1, PropertyCode.Width, Number.NaN)).toThrow(
      "finite",
    );
  });

  test("encodes native controls, SwiftUI reverse hosts, overlays, terminals, SVGs, paint, and pointer capture under protocol v21", () => {
    expect(PROTOCOL_VERSION).toBe(21);
    const batch = new MutationBatch();
    batch.createElement(1, NativeNodeTag.Input);
    batch.setProperty(1, PropertyCode.Value, "hello");
    batch.setProperty(1, PropertyCode.Password, true);
    batch.createElement(2, NativeNodeTag.Markdown);
    batch.setProperty(2, PropertyCode.Streaming, true);
    batch.setProperty(2, PropertyCode.ScrollToEndRevision, 3);
    batch.createElement(3, NativeNodeTag.VirtualList);
    batch.setProperty(3, PropertyCode.EstimatedItemHeight, 180);
    batch.setProperty(3, PropertyCode.FollowMode, "tail");
    batch.createElement(4, NativeNodeTag.View);
    batch.setProperty(4, PropertyCode.AnchorTarget, "1");
    batch.setProperty(4, PropertyCode.AnchorPlacement, "bottom-start");
    batch.setProperty(4, PropertyCode.AnchorGap, 8);
    batch.setProperty(4, PropertyCode.ViewportMargin, 12);
    batch.setProperty(4, PropertyCode.DismissOnEscape, false);
    batch.setProperty(4, PropertyCode.DismissOnPointerOutside, true);
    batch.setProperty(4, PropertyCode.DismissListener, true);
    batch.setProperty(4, PropertyCode.BorderTopWidth, 1);
    batch.setProperty(4, PropertyCode.BorderRightWidth, 2);
    batch.setProperty(4, PropertyCode.BorderBottomWidth, 3);
    batch.setProperty(4, PropertyCode.BorderLeftWidth, 4);
    batch.setProperty(
      4,
      PropertyCode.BoxShadow,
      '[{"offsetX":0,"offsetY":8,"blurRadius":24,"spreadRadius":-8,"color":4278190080,"inset":false}]',
    );
    batch.createElement(5, NativeNodeTag.Terminal);
    batch.setProperty(5, PropertyCode.TerminalProgram, "/bin/zsh");
    batch.setProperty(
      5,
      PropertyCode.TerminalArguments,
      JSON.stringify(["-l"]),
    );
    batch.setProperty(5, PropertyCode.TerminalWorkingDirectory, "/tmp");
    batch.setProperty(
      5,
      PropertyCode.TerminalEnvironment,
      JSON.stringify({ TERM: "xterm-256color" }),
    );
    batch.setProperty(5, PropertyCode.TerminalScrollback, 20_000);
    batch.setProperty(5, PropertyCode.TerminalStatusListener, true);
    batch.setProperty(
      5,
      PropertyCode.FontFamily,
      "JetBrainsMono Nerd Font Mono",
    );
    batch.setProperty(5, PropertyCode.TerminalPalette, "[1,2,3]");
    batch.setProperty(5, PropertyCode.TerminalCursorColor, 0xffda6909, true);
    batch.setProperty(5, PropertyCode.TerminalPaddingColor, "extend");
    batch.setProperty(5, PropertyCode.TerminalFontThicken, true);
    batch.setProperty(1, PropertyCode.HoverBackgroundColor, 0xff332211, true);
    batch.setProperty(1, PropertyCode.HoverColor, 0xffeeeeee, true);
    batch.setProperty(1, PropertyCode.ActiveBackgroundColor, 0xff221100, true);
    batch.setProperty(1, PropertyCode.ActiveColor, 0xffffffff, true);
    batch.setProperty(1, PropertyCode.Transition, 90);
    batch.setProperty(1, PropertyCode.PointerListener, true);
    batch.setProperty(1, PropertyCode.FocusOnPointer, false);
    batch.setProperty(1, PropertyCode.HitSlop, 2);
    batch.setProperty(1, PropertyCode.HitSlopTop, 3);
    batch.setProperty(1, PropertyCode.HitSlopRight, 4);
    batch.setProperty(1, PropertyCode.HitSlopBottom, 5);
    batch.setProperty(1, PropertyCode.HitSlopLeft, 6);
    batch.setProperty(4, PropertyCode.Overlay, true);
    batch.setProperty(4, PropertyCode.FocusTrap, true);
    batch.setProperty(4, PropertyCode.RestorePreviousFocus, true);
    batch.setProperty(1, PropertyCode.AutoFocus, true);
    batch.setProperty(4, PropertyCode.AccessibilityModal, true);
    batch.createElement(6, NativeNodeTag.Svg);
    batch.setProperty(6, PropertyCode.Value, "<svg/>");
    batch.createElement(7, NativeNodeTag.SwiftUIHost);
    batch.setProperty(7, PropertyCode.SwiftUIMatchContentsHorizontal, true);
    batch.setProperty(7, PropertyCode.SwiftUIMatchContentsVertical, true);
    batch.createElement(8, NativeNodeTag.SwiftUIButton);
    batch.setProperty(8, PropertyCode.SwiftUIButtonStyle, "glass");
    batch.setProperty(8, PropertyCode.SwiftUIControlSize, "large");
    batch.setProperty(
      8,
      PropertyCode.SwiftUISystemImage,
      "square.and.arrow.down",
    );
    batch.setProperty(8, PropertyCode.SwiftUITarget, "save");
    batch.setProperty(8, PropertyCode.SwiftUITestId, "save-button");
    batch.setProperty(
      8,
      PropertyCode.SwiftUIModifiers,
      JSON.stringify([{ $type: "buttonStyle", style: "glass" }]),
    );
    batch.createElement(9, NativeNodeTag.SwiftUIQuickGUIHost);
    batch.setProperty(9, PropertyCode.SwiftUIEmbeddedWindow, 2);
    batch.createElement(10, NativeNodeTag.SwiftUIPopover);
    batch.setProperty(10, PropertyCode.SwiftUIIsPresented, true);
    batch.setProperty(10, PropertyCode.SwiftUIAttachmentAnchor, "bottom");
    batch.setProperty(10, PropertyCode.SwiftUIArrowEdge, "top");
    batch.setProperty(10, PropertyCode.SwiftUIPresentationListener, true);
    batch.createElement(11, NativeNodeTag.SwiftUIPopoverTrigger);
    batch.createElement(12, NativeNodeTag.SwiftUIPopoverContent);
    expect(batch.mutationCount).toBe(72);
    expect(batch.finish().byteLength).toBeGreaterThan(10);
  });

  test("encodes selection, tab, disclosure, field, and tooltip component parts", () => {
    const batch = new MutationBatch();
    batch.createElement(1, NativeNodeTag.Button);
    batch.setProperty(1, PropertyCode.Part, NativePart.Checkbox);
    batch.setProperty(1, PropertyCode.Checked, false);
    batch.setProperty(1, PropertyCode.Indeterminate, true);
    batch.createElement(2, NativeNodeTag.Button);
    batch.setProperty(2, PropertyCode.Part, NativePart.Tab);
    batch.setProperty(2, PropertyCode.Scope, "qg-tabs-1");
    batch.setProperty(2, PropertyCode.PartValue, "overview");
    batch.setProperty(2, PropertyCode.ActiveValue, "overview");
    batch.setProperty(2, PropertyCode.Orientation, "vertical");
    batch.setProperty(2, PropertyCode.ActivateOnFocus, true);
    batch.setProperty(2, PropertyCode.LoopFocus, false);
    batch.setProperty(2, PropertyCode.KeepMounted, true);
    batch.createElement(3, NativeNodeTag.View);
    batch.setProperty(3, PropertyCode.Part, NativePart.AccordionItem);
    batch.setProperty(3, PropertyCode.Open, true);
    batch.setProperty(3, PropertyCode.ItemIndex, 2);
    batch.setProperty(3, PropertyCode.HeadingLevel, 4);
    batch.createElement(4, NativeNodeTag.Input);
    batch.setProperty(4, PropertyCode.Part, NativePart.FieldControl);
    batch.setProperty(4, PropertyCode.Required, true);
    batch.setProperty(4, PropertyCode.Invalid, true);
    batch.setProperty(4, PropertyCode.Touched, true);
    batch.setProperty(4, PropertyCode.Dirty, true);
    batch.setProperty(4, PropertyCode.Filled, false);
    batch.setProperty(4, PropertyCode.ValidationMessage, "Enter an address");
    batch.setProperty(4, PropertyCode.Tooltip, "We never share it");
    batch.setProperty(4, PropertyCode.TooltipPlacement, "top");
    batch.setProperty(4, PropertyCode.TooltipDelay, 250);
    batch.setProperty(4, PropertyCode.TooltipGap, 7);
    batch.setProperty(4, PropertyCode.TooltipViewportMargin, 8);
    batch.createElement(5, NativeNodeTag.View);
    batch.setProperty(5, PropertyCode.Part, NativePart.DialogPopup);
    batch.setProperty(5, PropertyCode.Variant, "alertdialog");
    batch.setProperty(5, PropertyCode.Open, true);
    batch.setProperty(5, PropertyCode.DismissOnEscape, true);
    batch.setProperty(5, PropertyCode.DismissOnPointerOutside, false);

    expect(batch.mutationCount).toBe(37);
    expect(batch.finish().byteLength).toBeGreaterThan(10);
  });

  test("encodes declared popover and context menus", () => {
    const batch = new MutationBatch();
    batch.createElement(1, NativeNodeTag.Button);
    batch.setProperty(1, PropertyCode.Part, NativePart.PopoverMenuTrigger);
    batch.setProperty(1, PropertyCode.Open, true);
    batch.setProperty(1, PropertyCode.Controls, "2");
    batch.createElement(2, NativeNodeTag.View);
    batch.setProperty(2, PropertyCode.Part, NativePart.PopoverMenuPopup);
    batch.setProperty(
      2,
      PropertyCode.Menu,
      JSON.stringify({ items: [{ id: "open", label: "Open" }] }),
    );
    batch.setProperty(2, PropertyCode.SelectListener, true);
    batch.createElement(3, NativeNodeTag.View);
    batch.setProperty(3, PropertyCode.Part, NativePart.ContextMenuTrigger);
    batch.setProperty(3, PropertyCode.Menu, JSON.stringify({ items: [] }));
    batch.setProperty(3, PropertyCode.SelectListener, true);

    expect(batch.mutationCount).toBe(12);
    expect(batch.finish().byteLength).toBeGreaterThan(10);
  });

  test("encodes CSS grid, full transitions, images, shaders, and range parts", () => {
    const batch = new MutationBatch();
    batch.createElement(1, NativeNodeTag.View);
    batch.setProperty(1, PropertyCode.Display, "grid");
    batch.setProperty(1, PropertyCode.GridTemplateColumns, "200px 1fr");
    batch.setProperty(1, PropertyCode.GridTemplateRows, 3);
    batch.setProperty(1, PropertyCode.GridAutoFlow, "column dense");
    batch.setProperty(1, PropertyCode.GridColumnStart, 2);
    batch.setProperty(1, PropertyCode.GridColumnEnd, 5);
    batch.setProperty(1, PropertyCode.GridRowSpan, 2);
    batch.setProperty(1, PropertyCode.TransitionProperties, "opacity,box-shadow");
    batch.setProperty(1, PropertyCode.TransitionDuration, 180);
    batch.setProperty(1, PropertyCode.TransitionEasing, "ease-out");
    batch.setProperty(1, PropertyCode.TransitionMaxFps, 30);
    batch.createElement(2, NativeNodeTag.Image);
    batch.setProperty(2, PropertyCode.Value, "/assets/logo.png");
    batch.setProperty(2, PropertyCode.ObjectFit, "cover");
    batch.createElement(3, NativeNodeTag.Shader);
    batch.setProperty(3, PropertyCode.ShaderParameters, "[0.5,0,0,1]");
    batch.createElement(4, NativeNodeTag.View);
    batch.setProperty(4, PropertyCode.Part, NativePart.Progress);
    batch.setProperty(4, PropertyCode.Maximum, 12);
    batch.setProperty(4, PropertyCode.ValueText, "3 of 12 files");
    batch.createElement(5, NativeNodeTag.View);
    batch.setProperty(5, PropertyCode.Part, NativePart.Meter);
    batch.setProperty(5, PropertyCode.Minimum, 0);
    batch.setProperty(5, PropertyCode.Low, 25);
    batch.setProperty(5, PropertyCode.High, 75);
    batch.setProperty(5, PropertyCode.Optimum, 90);
    batch.createElement(6, NativeNodeTag.Button);
    batch.setProperty(6, PropertyCode.Part, NativePart.Toggle);
    batch.setProperty(6, PropertyCode.Pressed, true);

    expect(batch.mutationCount).toBe(30);
    expect(batch.finish().byteLength).toBeGreaterThan(10);
  });

  test("encodes declared key, mouse, gesture, keymap, and drag listeners", () => {
    const batch = new MutationBatch();
    batch.createElement(1, NativeNodeTag.View);
    batch.setProperty(1, PropertyCode.TabIndex, 0);
    batch.setProperty(1, PropertyCode.KeyDownListener, true);
    batch.setProperty(1, PropertyCode.KeyUpListener, true);
    batch.setProperty(1, PropertyCode.MouseDownListener, true);
    batch.setProperty(1, PropertyCode.MouseUpListener, true);
    batch.setProperty(1, PropertyCode.MouseMoveListener, true);
    batch.setProperty(1, PropertyCode.DoubleClickListener, true);
    batch.setProperty(1, PropertyCode.ScrollListener, true);
    batch.setProperty(1, PropertyCode.ContextMenuListener, true);
    batch.setProperty(1, PropertyCode.PinchListener, true);
    batch.setProperty(1, PropertyCode.RotationListener, true);
    batch.setProperty(1, PropertyCode.SmartMagnifyListener, true);
    batch.setProperty(1, PropertyCode.PressureListener, true);
    batch.setProperty(1, PropertyCode.FocusListener, true);
    batch.setProperty(1, PropertyCode.Keymap, JSON.stringify({ "CmdOrCtrl+S": "save" }));
    batch.setProperty(1, PropertyCode.ActionListener, true);
    batch.createElement(2, NativeNodeTag.View);
    batch.setProperty(2, PropertyCode.Draggable, JSON.stringify({ id: "row-7" }));
    batch.setProperty(2, PropertyCode.DragListener, true);
    batch.createElement(3, NativeNodeTag.View);
    batch.setProperty(3, PropertyCode.DropKinds, JSON.stringify(["local", "files"]));
    batch.setProperty(3, PropertyCode.DropListener, true);

    expect(batch.mutationCount).toBe(23);
    expect(batch.finish().byteLength).toBeGreaterThan(10);
  });

  test("keeps every component part name and bound stable", () => {
    expect(NativePart.Checkbox).toBe("checkbox");
    expect(NativePart.TabPanel).toBe("tab-panel");
    expect(NativePart.CollapsiblePanel).toBe("collapsible-panel");
    expect(NativePart.AccordionTrigger).toBe("accordion-trigger");
    expect(NativePart.FieldPassiveLabel).toBe("field-passive-label");
    expect(NativePart.FieldsetLegend).toBe("fieldset-legend");
    expect(NativePart.Dialog).toBe("dialog");
    expect(NativePart.DialogPopup).toBe("dialog-popup");
    expect(MAX_COMPONENT_VALUE_BYTES).toBe(256);
    expect(MAX_TOOLTIP_TEXT_BYTES).toBe(1024);
    expect(NativePart.PopoverMenuTrigger).toBe("popover-menu-trigger");
    expect(NativePart.PopoverMenuPopup).toBe("popover-menu-popup");
    expect(NativePart.ContextMenuTrigger).toBe("context-menu-trigger");
    expect(MAX_MENU_JSON_BYTES).toBe(512 * 1024);
    expect(NativePart.Progress).toBe("progress");
    expect(NativePart.MeterIndicator).toBe("meter-indicator");
    expect(NativePart.Toggle).toBe("toggle");
    expect(NativeNodeTag.Image).toBe(16);
    expect(NativeNodeTag.Shader).toBe(17);
    expect(MAX_KEYMAP_JSON_BYTES).toBe(64 * 1024);
    expect(MAX_DRAG_JSON_BYTES).toBe(64 * 1024);
    expect(NativePart.Slider).toBe("slider");
    expect(NativePart.SliderTrack).toBe("slider-track");
    expect(NativePart.SliderRange).toBe("slider-range");
    expect(NativePart.SliderThumb).toBe("slider-thumb");
    expect(NativePart.Splitter).toBe("splitter");
    expect(NativePart.SplitterPane).toBe("splitter-pane");
    expect(NativePart.SplitterHandle).toBe("splitter-handle");
    expect(NativePart.Toolbar).toBe("toolbar");
    expect(NativePart.ToolbarItem).toBe("toolbar-item");
    expect(NativePart.ToggleGroup).toBe("toggle-group");
    expect(NativePart.ToggleGroupItem).toBe("toggle-group-item");
    expect(MAX_COMPONENT_JSON_BYTES).toBe(64 * 1024);
    expect(MAX_COMPONENT_VALUES).toBe(64);
    expect(MAX_COMPONENT_ITEMS).toBe(256);
  });

  test("encodes declared range, ordering, and roving-focus component declarations", () => {
    const batch = new MutationBatch();
    batch.createElement(1, NativeNodeTag.View);
    batch.setProperty(1, PropertyCode.Part, NativePart.Slider);
    batch.setProperty(1, PropertyCode.Scope, "volume");
    batch.setProperty(1, PropertyCode.Values, "[10,60]");
    batch.setProperty(1, PropertyCode.Minimum, 0);
    batch.setProperty(1, PropertyCode.Maximum, 100);
    batch.setProperty(1, PropertyCode.Step, 5);
    batch.setProperty(1, PropertyCode.LargeStep, 25);
    batch.setProperty(1, PropertyCode.ComponentChangeListener, true);
    batch.createElement(2, NativeNodeTag.View);
    batch.setProperty(2, PropertyCode.Part, NativePart.SliderThumb);
    batch.setProperty(2, PropertyCode.Scope, "volume");
    batch.setProperty(2, PropertyCode.ItemIndex, 1);
    batch.createElement(3, NativeNodeTag.View);
    batch.setProperty(3, PropertyCode.Part, NativePart.Toolbar);
    batch.setProperty(3, PropertyCode.Items, JSON.stringify([{ value: "cut" }]));

    expect(batch.mutationCount).toBe(16);
    expect(batch.finish().byteLength).toBeGreaterThan(10);
  });
});
