import { createRenderer } from "@solidjs/universal";
import { flush as flushSolid, type Element as SolidElement } from "solid-js";
import {
  App,
  type NativeElementName,
  type NativeEventListener,
  NativeNode,
  Window,
  PropertyCode,
  cleanupNativeNodes,
  createNativeElement,
  createNativeSentinel,
  createNativeText,
  getNativeFirstChild,
  getNativeNextSibling,
  getNativeParent,
  insertNativeNode,
  isNativeText,
  parseColor,
  removeNativeNode,
  replaceNativeText,
  setNativeEventListener,
  setNativeProperty,
} from "@quickgui/native";

export { App, NativeNode, Window } from "@quickgui/native";
export type { PopupPlacement, QuickGuiEvent, RunOptions, WindowOptions } from "@quickgui/native";

type PropertyInput = unknown;
type PropertyEntry = {
  code: PropertyCode;
  color?: boolean;
};

const properties: Record<string, PropertyEntry> = {
  display: { code: PropertyCode.Display },
  flexDirection: { code: PropertyCode.FlexDirection },
  flexWrap: { code: PropertyCode.FlexWrap },
  flexGrow: { code: PropertyCode.FlexGrow },
  flexShrink: { code: PropertyCode.FlexShrink },
  flexBasis: { code: PropertyCode.FlexBasis },
  alignItems: { code: PropertyCode.AlignItems },
  alignSelf: { code: PropertyCode.AlignSelf },
  justifyContent: { code: PropertyCode.JustifyContent },
  alignContent: { code: PropertyCode.AlignContent },
  gap: { code: PropertyCode.Gap },
  columnGap: { code: PropertyCode.ColumnGap },
  rowGap: { code: PropertyCode.RowGap },
  width: { code: PropertyCode.Width },
  height: { code: PropertyCode.Height },
  minWidth: { code: PropertyCode.MinWidth },
  minHeight: { code: PropertyCode.MinHeight },
  maxWidth: { code: PropertyCode.MaxWidth },
  maxHeight: { code: PropertyCode.MaxHeight },
  padding: { code: PropertyCode.Padding },
  paddingTop: { code: PropertyCode.PaddingTop },
  paddingRight: { code: PropertyCode.PaddingRight },
  paddingBottom: { code: PropertyCode.PaddingBottom },
  paddingLeft: { code: PropertyCode.PaddingLeft },
  margin: { code: PropertyCode.Margin },
  marginTop: { code: PropertyCode.MarginTop },
  marginRight: { code: PropertyCode.MarginRight },
  marginBottom: { code: PropertyCode.MarginBottom },
  marginLeft: { code: PropertyCode.MarginLeft },
  background: { code: PropertyCode.BackgroundColor, color: true },
  backgroundColor: { code: PropertyCode.BackgroundColor, color: true },
  color: { code: PropertyCode.Color, color: true },
  opacity: { code: PropertyCode.Opacity },
  borderWidth: { code: PropertyCode.BorderWidth },
  borderColor: { code: PropertyCode.BorderColor, color: true },
  borderRadius: { code: PropertyCode.BorderRadius },
  fontSize: { code: PropertyCode.FontSize },
  fontWeight: { code: PropertyCode.FontWeight },
  lineHeight: { code: PropertyCode.LineHeight },
  textAlign: { code: PropertyCode.TextAlign },
  whiteSpace: { code: PropertyCode.WhiteSpace },
  textOverflow: { code: PropertyCode.TextOverflow },
  lineClamp: { code: PropertyCode.LineClamp },
  WebkitLineClamp: { code: PropertyCode.LineClamp },
  overflow: { code: PropertyCode.Overflow },
  overflowX: { code: PropertyCode.OverflowX },
  overflowY: { code: PropertyCode.OverflowY },
  cursor: { code: PropertyCode.Cursor },
  appRegion: { code: PropertyCode.AppRegion },
  disabled: { code: PropertyCode.Disabled },
  ariaLabel: { code: PropertyCode.AccessibilityLabel },
  role: { code: PropertyCode.Role },
  tabIndex: { code: PropertyCode.TabIndex },
  position: { code: PropertyCode.Position },
  top: { code: PropertyCode.Top },
  right: { code: PropertyCode.Right },
  bottom: { code: PropertyCode.Bottom },
  left: { code: PropertyCode.Left },
  userSelect: { code: PropertyCode.UserSelect },
  visibility: { code: PropertyCode.Visibility },
  aspectRatio: { code: PropertyCode.AspectRatio },
  value: { code: PropertyCode.Value },
  content: { code: PropertyCode.Value },
  source: { code: PropertyCode.Value },
  placeholder: { code: PropertyCode.Placeholder },
  multiline: { code: PropertyCode.Multiline },
  streaming: { code: PropertyCode.Streaming },
  markdownCodeBackground: { code: PropertyCode.MarkdownCodeBackground, color: true },
  markdownBorderColor: { code: PropertyCode.MarkdownBorderColor, color: true },
  markdownMutedColor: { code: PropertyCode.MarkdownMutedColor, color: true },
  markdownLinkColor: { code: PropertyCode.MarkdownLinkColor, color: true },
  markdownCodeTextColor: { code: PropertyCode.MarkdownCodeTextColor, color: true },
  markdownBlockGap: { code: PropertyCode.MarkdownBlockGap },
  markdownCodeFontSize: { code: PropertyCode.MarkdownCodeFontSize },
  scrollToEndRevision: { code: PropertyCode.ScrollToEndRevision },
};

const colorProperties = new Set([
  PropertyCode.BackgroundColor,
  PropertyCode.Color,
  PropertyCode.BorderColor,
  PropertyCode.MarkdownCodeBackground,
  PropertyCode.MarkdownBorderColor,
  PropertyCode.MarkdownMutedColor,
  PropertyCode.MarkdownLinkColor,
  PropertyCode.MarkdownCodeTextColor,
]);

function setProperty(node: NativeNode, name: string, value: PropertyInput, previous?: PropertyInput) {
  if (name === "children" || name === "ref" || name === "key") return;
  if (name === "style") {
    setStyle(node, value, previous);
    return;
  }
  const event = eventName(name);
  if (event) {
    setNativeEventListener(
      node,
      event,
      typeof value === "function"
        ? (nativeEvent) => {
            try {
              (value as NativeEventListener)(nativeEvent);
            } finally {
              // Solid 2 batches external writes until the host marks the event boundary.
              flushSolid();
            }
          }
        : undefined,
    );
    return;
  }
  if (name === "class" || name === "className") return;
  if (name === "aria-label") name = "ariaLabel";
  if (name === "type") {
    setNativeProperty(node, PropertyCode.Password, value === "password");
    return;
  }
  if (name === "flex") {
    setFlex(node, value);
    return;
  }
  const entry = properties[name];
  if (!entry) return;
  const normalized = normalizeValue(value, entry.code);
  setNativeProperty(node, entry.code, normalized, { color: !!entry.color });
}

function setStyle(node: NativeNode, value: PropertyInput, previous: PropertyInput): void {
  const next = isRecord(value) ? value : {};
  const old = isRecord(previous) ? previous : {};
  for (const name of Object.keys(old)) {
    if (!(name in next)) setProperty(node, name, null, old[name]);
  }
  for (const [name, nextValue] of Object.entries(next)) {
    if (!Object.is(nextValue, old[name])) setProperty(node, name, nextValue, old[name]);
  }
}

function setFlex(node: NativeNode, value: PropertyInput): void {
  if (value === null || value === undefined || value === false) {
    for (const code of [PropertyCode.FlexGrow, PropertyCode.FlexShrink, PropertyCode.FlexBasis]) {
      setNativeProperty(node, code, null);
    }
    return;
  }
  if (typeof value === "number") {
    setNativeProperty(node, PropertyCode.FlexGrow, value);
    setNativeProperty(node, PropertyCode.FlexShrink, 1);
    setNativeProperty(node, PropertyCode.FlexBasis, 0);
    return;
  }
  const parts = String(value).trim().split(/\s+/);
  if (parts.length === 1 && parts[0] === "none") {
    setNativeProperty(node, PropertyCode.FlexGrow, 0);
    setNativeProperty(node, PropertyCode.FlexShrink, 0);
    setNativeProperty(node, PropertyCode.FlexBasis, "auto");
    return;
  }
  if (parts.length >= 1) setNativeProperty(node, PropertyCode.FlexGrow, Number(parts[0]));
  if (parts.length >= 2) setNativeProperty(node, PropertyCode.FlexShrink, Number(parts[1]));
  if (parts.length >= 3) {
    setNativeProperty(node, PropertyCode.FlexBasis, normalizeLength(parts[2]));
  }
}

function normalizeValue(value: PropertyInput, code: PropertyCode): boolean | number | string | null {
  if (value === null || value === undefined || value === false) {
    return code === PropertyCode.Disabled ? false : null;
  }
  if (colorProperties.has(code)) return parseColor(value as number | string);
  if (typeof value === "number" || typeof value === "boolean") return value;
  if (isLengthProperty(code)) return normalizeLength(String(value));
  return String(value);
}

function normalizeLength(value: string | undefined): number | string | null {
  if (value === undefined) return null;
  const trimmed = value.trim();
  if (trimmed.endsWith("px")) {
    const number = Number(trimmed.slice(0, -2));
    return Number.isFinite(number) ? number : null;
  }
  if (trimmed === "0") return 0;
  const number = Number(trimmed);
  return Number.isFinite(number) ? number : trimmed;
}

function isLengthProperty(code: PropertyCode): boolean {
  return (
    (code >= PropertyCode.Gap && code <= PropertyCode.MarginLeft) ||
    code === PropertyCode.BorderWidth ||
    code === PropertyCode.BorderRadius ||
    code === PropertyCode.FontSize ||
    code === PropertyCode.LineHeight ||
    (code >= PropertyCode.Top && code <= PropertyCode.Left)
  );
}

function eventName(name: string):
  | "click"
  | "mouseenter"
  | "mouseleave"
  | "input"
  | "submit"
  | undefined {
  switch (name.toLowerCase()) {
    case "onclick":
    case "on:click":
      return "click";
    case "onmouseenter":
    case "onpointerenter":
      return "mouseenter";
    case "onmouseleave":
    case "onpointerleave":
      return "mouseleave";
    case "oninput":
    case "onchange":
      return "input";
    case "onsubmit":
      return "submit";
    default:
      return undefined;
  }
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

const universal = createRenderer<NativeNode>({
  createElement(tag, staticProps) {
    const name = tag as NativeElementName;
    if (!["view", "div", "text", "button", "input", "textarea", "markdown"].includes(name)) {
      throw new TypeError(`unknown QuickGUI element <${tag}>`);
    }
    const node = createNativeElement(name);
    if (staticProps) {
      for (const [name, value] of Object.entries(staticProps)) setProperty(node, name, value);
    }
    return node;
  },
  createTextNode: createNativeText,
  createSentinel: createNativeSentinel,
  replaceText: replaceNativeText,
  isTextNode: isNativeText,
  setProperty,
  insertNode: insertNativeNode,
  removeNode: removeNativeNode,
  cleanupNodes: cleanupNativeNodes,
  getParentNode: getNativeParent,
  getFirstChild: getNativeFirstChild,
  getNextSibling: getNativeNextSibling,
});

const nativeRender = universal.render;

/** Unstyled block/flex/grid container. */
export function View(props: JSX.NativeProps): NativeNode {
  const node = universal.createElement("view");
  universal.spread(node, props);
  return node;
}

/** Unstyled text-semantic container whose string children remain individually reactive. */
export function Text(props: JSX.NativeProps): NativeNode {
  const node = universal.createElement("text");
  universal.spread(node, props);
  return node;
}

/** Unstyled, focusable native button with web-style arrow-cursor behavior by default. */
export function Button(props: JSX.NativeProps): NativeNode {
  const node = universal.createElement("button");
  universal.spread(node, props);
  return node;
}

/** Controlled, unstyled single-line native text input. */
export function Input(props: JSX.InputProps): NativeNode {
  const node = universal.createElement("input");
  universal.spread(node, props);
  return node;
}

/** Controlled, unstyled multiline native text area. */
export function TextArea(props: JSX.InputProps): NativeNode {
  const node = universal.createElement("textarea");
  universal.spread(node, props);
  return node;
}

/** Retained, incremental native Markdown document. */
export function Markdown(props: JSX.MarkdownProps): NativeNode {
  const node = universal.createElement("markdown");
  universal.spread(node, props);
  return node;
}

export function render(code: () => JSX.Element, target: Window | NativeNode): () => void {
  const root = target instanceof Window ? target.root : target;
  const nativeDispose = nativeRender(code as () => NativeNode, root);
  const window = target instanceof Window ? target : root.host;
  let disposed = false;
  let untrack = () => {};
  const dispose = () => {
    if (disposed) return;
    disposed = true;
    untrack();
    nativeDispose();
    root.host?.flush();
  };
  if (window) untrack = window._trackMount(dispose);
  root.host?.flush();
  return dispose;
}

export const effect = universal.effect;
export const memo = universal.memo;
export const createComponent = universal.createComponent;
export const createElement = universal.createElement;
export const createTextNode = universal.createTextNode;
export const insertNode = universal.insertNode;
export const insert = universal.insert;
export const spread = universal.spread;
export const setProp = universal.setProp;
export const mergeProps = universal.mergeProps;
export const applyRef = universal.applyRef;
export const ref = universal.ref;

export namespace JSX {
  export type Element = SolidElement;
  export type Child = SolidElement;
  export type EventHandler = NativeEventListener;

  export interface ElementChildrenAttribute {
    children: {};
  }

  export interface IntrinsicAttributes {
    key?: string | number;
  }

  export interface Style {
    display?: "none" | "block" | "flex" | "grid";
    flex?: number | string;
    flexDirection?: "row" | "row-reverse" | "column" | "column-reverse";
    flexWrap?: "nowrap" | "wrap" | "wrap-reverse";
    flexGrow?: number;
    flexShrink?: number;
    flexBasis?: number | string;
    alignItems?: "start" | "flex-start" | "center" | "end" | "flex-end" | "baseline" | "stretch";
    alignSelf?: Style["alignItems"];
    justifyContent?: "start" | "flex-start" | "center" | "end" | "flex-end" | "space-between" | "space-around" | "space-evenly";
    alignContent?: Style["justifyContent"] | "normal" | "stretch";
    gap?: number | string;
    columnGap?: number | string;
    rowGap?: number | string;
    width?: number | string;
    height?: number | string;
    minWidth?: number | string;
    minHeight?: number | string;
    maxWidth?: number | string;
    maxHeight?: number | string;
    padding?: number | string;
    paddingTop?: number | string;
    paddingRight?: number | string;
    paddingBottom?: number | string;
    paddingLeft?: number | string;
    margin?: number | string;
    marginTop?: number | string;
    marginRight?: number | string;
    marginBottom?: number | string;
    marginLeft?: number | string;
    background?: number | string;
    backgroundColor?: number | string;
    color?: number | string;
    opacity?: number;
    borderWidth?: number | string;
    borderColor?: number | string;
    borderRadius?: number | string;
    fontSize?: number | string;
    fontWeight?: number | string;
    lineHeight?: number | string;
    textAlign?: "left" | "center" | "right" | "justify" | "start" | "end";
    whiteSpace?: "normal" | "nowrap";
    textOverflow?: "clip" | "ellipsis";
    lineClamp?: number;
    overflow?: "visible" | "hidden" | "auto" | "scroll";
    overflowX?: Style["overflow"];
    overflowY?: Style["overflow"];
    cursor?: string;
    appRegion?: "drag" | "no-drag";
    position?: "relative" | "absolute";
    top?: number | string;
    right?: number | string;
    bottom?: number | string;
    left?: number | string;
    userSelect?: "auto" | "text" | "none";
    visibility?: "visible" | "hidden";
    aspectRatio?: number;
    markdownCodeBackground?: number | string;
    markdownBorderColor?: number | string;
    markdownMutedColor?: number | string;
    markdownLinkColor?: number | string;
    markdownCodeTextColor?: number | string;
    markdownBlockGap?: number;
    markdownCodeFontSize?: number;
    scrollToEndRevision?: number;
  }

  export interface NativeProps extends Style {
    children?: unknown;
    style?: Style;
    class?: string;
    className?: string;
    disabled?: boolean;
    role?: string;
    tabIndex?: number;
    "aria-label"?: string;
    ariaLabel?: string;
    ref?: ((node: NativeNode) => void) | NativeNode;
    onClick?: EventHandler;
    onMouseEnter?: EventHandler;
    onMouseLeave?: EventHandler;
    onPointerEnter?: EventHandler;
    onPointerLeave?: EventHandler;
    onInput?: EventHandler;
    onChange?: EventHandler;
    onSubmit?: EventHandler;
  }

  export interface InputProps extends NativeProps {
    type?: "text" | "password";
    value?: string;
    placeholder?: string;
    multiline?: boolean;
  }

  export interface MarkdownProps extends NativeProps {
    content?: string;
    source?: string;
    streaming?: boolean;
  }

  export interface IntrinsicElements {
    view: NativeProps;
    div: NativeProps;
    text: NativeProps;
    button: NativeProps;
    input: InputProps;
    textarea: InputProps;
    markdown: MarkdownProps;
  }
}
