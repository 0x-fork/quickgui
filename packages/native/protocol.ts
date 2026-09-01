import { Buffer } from "node:buffer";

export const PROTOCOL_VERSION = 18;
export const ROOT_NODE_ID = 0;
export const NO_ANCHOR = 0xffff_ffff;

export const enum NativeNodeTag {
  View = 1,
  Button = 2,
  Text = 3,
  Sentinel = 4,
  Input = 5,
  Markdown = 6,
  VirtualList = 7,
  Terminal = 8,
  Svg = 9,
  SwiftUIHost = 10,
  SwiftUIButton = 11,
  SwiftUIQuickGUIHost = 12,
  SwiftUIPopover = 13,
  SwiftUIPopoverTrigger = 14,
  SwiftUIPopoverContent = 15,
}

export const enum PropertyCode {
  Display = 1,
  FlexDirection = 2,
  FlexWrap = 3,
  FlexGrow = 4,
  FlexShrink = 5,
  FlexBasis = 6,
  AlignItems = 7,
  AlignSelf = 8,
  JustifyContent = 9,
  AlignContent = 10,
  Gap = 11,
  ColumnGap = 12,
  RowGap = 13,
  Width = 14,
  Height = 15,
  MinWidth = 16,
  MinHeight = 17,
  MaxWidth = 18,
  MaxHeight = 19,
  Padding = 20,
  PaddingTop = 21,
  PaddingRight = 22,
  PaddingBottom = 23,
  PaddingLeft = 24,
  Margin = 25,
  MarginTop = 26,
  MarginRight = 27,
  MarginBottom = 28,
  MarginLeft = 29,
  BackgroundColor = 30,
  Color = 31,
  Opacity = 32,
  BorderWidth = 33,
  BorderColor = 34,
  BorderRadius = 35,
  FontSize = 36,
  FontWeight = 37,
  LineHeight = 38,
  TextAlign = 39,
  WhiteSpace = 40,
  TextOverflow = 41,
  LineClamp = 42,
  Overflow = 43,
  OverflowX = 44,
  OverflowY = 45,
  Cursor = 46,
  AppRegion = 47,
  Disabled = 48,
  AccessibilityLabel = 49,
  Role = 50,
  TabIndex = 51,
  Position = 52,
  Top = 53,
  Right = 54,
  Bottom = 55,
  Left = 56,
  UserSelect = 57,
  ClickListener = 58,
  HoverListener = 59,
  Visibility = 60,
  AspectRatio = 61,
  Value = 62,
  Placeholder = 63,
  Multiline = 64,
  InputListener = 65,
  SubmitListener = 66,
  Streaming = 67,
  MarkdownCodeBackground = 68,
  MarkdownBorderColor = 69,
  MarkdownMutedColor = 70,
  MarkdownLinkColor = 71,
  MarkdownCodeTextColor = 72,
  MarkdownBlockGap = 73,
  MarkdownCodeFontSize = 74,
  ScrollToEndRevision = 75,
  Password = 76,
  EstimatedItemHeight = 77,
  Overscan = 78,
  ListAlignment = 79,
  FollowMode = 80,
  AnchorTarget = 81,
  AnchorPlacement = 82,
  AnchorGap = 83,
  ViewportMargin = 84,
  DismissOnEscape = 85,
  DismissOnPointerOutside = 86,
  DismissListener = 87,
  TerminalProgram = 88,
  TerminalArguments = 89,
  TerminalWorkingDirectory = 90,
  TerminalEnvironment = 91,
  TerminalScrollback = 92,
  TerminalStatusListener = 93,
  HoverBackgroundColor = 94,
  HoverColor = 95,
  ActiveBackgroundColor = 96,
  ActiveColor = 97,
  Transition = 98,
  PointerListener = 99,
  FocusOnPointer = 100,
  FontFamily = 101,
  TerminalPalette = 102,
  TerminalCursorColor = 103,
  HitSlop = 104,
  HitSlopTop = 105,
  HitSlopRight = 106,
  HitSlopBottom = 107,
  HitSlopLeft = 108,
  Overlay = 109,
  FocusTrap = 110,
  RestorePreviousFocus = 111,
  AutoFocus = 112,
  AccessibilityModal = 113,
  TerminalPaddingColor = 114,
  TerminalFontThicken = 115,
  SwiftUISystemImage = 116,
  SwiftUIButtonStyle = 117,
  SwiftUIControlSize = 118,
  SwiftUIMatchContentsHorizontal = 119,
  SwiftUIMatchContentsVertical = 120,
  SwiftUITarget = 121,
  SwiftUITestId = 122,
  SwiftUIModifiers = 123,
  SwiftUIEmbeddedWindow = 124,
  SwiftUIIsPresented = 125,
  SwiftUIAttachmentAnchor = 126,
  SwiftUIArrowEdge = 127,
  SwiftUIPresentationListener = 128,
  BorderTopWidth = 129,
  BorderRightWidth = 130,
  BorderBottomWidth = 131,
  BorderLeftWidth = 132,
  BoxShadow = 133,
}

export type NativePropertyValue = boolean | number | string | null;

const encoder = new TextEncoder();

export class MutationBatch {
  #bytes = new Uint8Array(512);
  #view = new DataView(this.#bytes.buffer);
  #length = 10;
  #count = 0;

  get empty(): boolean {
    return this.#count === 0;
  }

  get mutationCount(): number {
    return this.#count;
  }

  createElement(id: number, tag: NativeNodeTag): void {
    this.#op(1);
    this.#u32(id);
    this.#u8(tag);
  }

  createText(id: number, value: string): void {
    this.#op(2);
    this.#u32(id);
    this.#string(value);
  }

  createSentinel(id: number): void {
    this.#op(3);
    this.#u32(id);
  }

  setProperty(
    id: number,
    property: PropertyCode,
    value: NativePropertyValue,
    color = false,
  ): void {
    this.#op(4);
    this.#u32(id);
    this.#u16(property);
    if (value === null) {
      this.#u8(0);
    } else if (typeof value === "boolean") {
      this.#u8(1);
      this.#u8(value ? 1 : 0);
    } else if (typeof value === "number") {
      if (!Number.isFinite(value)) {
        throw new TypeError(`QuickGUI property ${property} must be finite`);
      }
      if (color) {
        this.#u8(3);
        this.#u32(value);
      } else {
        this.#u8(2);
        this.#f32(value);
      }
    } else {
      this.#u8(4);
      this.#string(value);
    }
  }

  replaceText(id: number, value: string): void {
    this.#op(5);
    this.#u32(id);
    this.#string(value);
  }

  insert(parent: number, child: number, before?: number): void {
    this.#op(6);
    this.#u32(parent);
    this.#u32(child);
    this.#u32(before ?? NO_ANCHOR);
  }

  remove(parent: number, child: number): void {
    this.#op(7);
    this.#u32(parent);
    this.#u32(child);
  }

  cleanup(parent: number, children: readonly number[]): void {
    this.#op(8);
    this.#u32(parent);
    this.#u32(children.length);
    for (const child of children) this.#u32(child);
  }

  finish(): Buffer {
    if (this.#count === 0) return Buffer.alloc(0);
    this.#bytes.set([0x51, 0x47, 0x4d, 0x42], 0);
    this.#view.setUint16(4, PROTOCOL_VERSION, true);
    this.#view.setUint32(6, this.#count, true);
    return Buffer.from(this.#bytes.buffer, 0, this.#length);
  }

  #op(opcode: number): void {
    this.#count++;
    this.#u8(opcode);
  }

  #ensure(additional: number): void {
    const required = this.#length + additional;
    if (required <= this.#bytes.length) return;
    let capacity = this.#bytes.length;
    while (capacity < required) capacity *= 2;
    const next = new Uint8Array(capacity);
    next.set(this.#bytes.subarray(0, this.#length));
    this.#bytes = next;
    this.#view = new DataView(next.buffer);
  }

  #u8(value: number): void {
    this.#ensure(1);
    this.#view.setUint8(this.#length, value);
    this.#length += 1;
  }

  #u16(value: number): void {
    this.#ensure(2);
    this.#view.setUint16(this.#length, value, true);
    this.#length += 2;
  }

  #u32(value: number): void {
    if (!Number.isInteger(value) || value < 0 || value > 0xffff_ffff) {
      throw new RangeError(`QuickGUI protocol value ${value} is not a u32`);
    }
    this.#ensure(4);
    this.#view.setUint32(this.#length, value, true);
    this.#length += 4;
  }

  #f32(value: number): void {
    this.#ensure(4);
    this.#view.setFloat32(this.#length, value, true);
    this.#length += 4;
  }

  #string(value: string): void {
    const bytes = encoder.encode(value);
    this.#u32(bytes.length);
    this.#ensure(bytes.length);
    this.#bytes.set(bytes, this.#length);
    this.#length += bytes.length;
  }
}
