import { describe, expect, test } from "bun:test";
import {
  MutationBatch,
  NativeNodeTag,
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

  test("encodes native controls, SwiftUI reverse hosts, overlays, terminals, SVGs, paint, and pointer capture under protocol v18", () => {
    expect(PROTOCOL_VERSION).toBe(18);
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
});
