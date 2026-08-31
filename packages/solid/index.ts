import { createRenderer as createUniversalRenderer } from "@solidjs/universal";
import {
  createContext,
  createSignal,
  flush as flushSolid,
  getOwner,
  omit,
  onCleanup,
  runWithOwner,
  Show,
  type Element as SolidElement,
  useContext,
} from "solid-js";
import {
  type NativeElementName,
  type NativeEventListener,
  type PopoverPlacement,
  NativeNode,
  PropertyCode,
  QuickGuiEvent,
  Window,
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
  type WindowRenderer,
} from "@quickgui/native";

type PropertyInput = unknown;
type PropertyEntry = {
  code: PropertyCode;
  color?: boolean;
  normalize?: (value: PropertyInput) => boolean | number | string | null;
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
  hoverBackgroundColor: {
    code: PropertyCode.HoverBackgroundColor,
    color: true,
  },
  hoverColor: { code: PropertyCode.HoverColor, color: true },
  activeBackgroundColor: {
    code: PropertyCode.ActiveBackgroundColor,
    color: true,
  },
  activeColor: { code: PropertyCode.ActiveColor, color: true },
  transition: {
    code: PropertyCode.Transition,
    normalize: normalizeTransitionShorthand,
  },
  opacity: { code: PropertyCode.Opacity },
  borderWidth: { code: PropertyCode.BorderWidth },
  borderColor: { code: PropertyCode.BorderColor, color: true },
  borderRadius: { code: PropertyCode.BorderRadius },
  fontSize: { code: PropertyCode.FontSize },
  fontFamily: { code: PropertyCode.FontFamily },
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
  focusOnPointer: { code: PropertyCode.FocusOnPointer },
  hitSlop: { code: PropertyCode.HitSlop },
  hitSlopTop: { code: PropertyCode.HitSlopTop },
  hitSlopRight: { code: PropertyCode.HitSlopRight },
  hitSlopBottom: { code: PropertyCode.HitSlopBottom },
  hitSlopLeft: { code: PropertyCode.HitSlopLeft },
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
  markdownCodeBackground: {
    code: PropertyCode.MarkdownCodeBackground,
    color: true,
  },
  markdownBorderColor: { code: PropertyCode.MarkdownBorderColor, color: true },
  markdownMutedColor: { code: PropertyCode.MarkdownMutedColor, color: true },
  markdownLinkColor: { code: PropertyCode.MarkdownLinkColor, color: true },
  markdownCodeTextColor: {
    code: PropertyCode.MarkdownCodeTextColor,
    color: true,
  },
  markdownBlockGap: { code: PropertyCode.MarkdownBlockGap },
  markdownCodeFontSize: { code: PropertyCode.MarkdownCodeFontSize },
  scrollToEndRevision: { code: PropertyCode.ScrollToEndRevision },
  estimatedItemHeight: { code: PropertyCode.EstimatedItemHeight },
  overscan: { code: PropertyCode.Overscan },
  listAlignment: { code: PropertyCode.ListAlignment },
  followMode: { code: PropertyCode.FollowMode },
  anchorPlacement: { code: PropertyCode.AnchorPlacement },
  anchorGap: { code: PropertyCode.AnchorGap },
  viewportMargin: { code: PropertyCode.ViewportMargin },
  dismissOnEscape: { code: PropertyCode.DismissOnEscape },
  dismissOnPointerOutside: { code: PropertyCode.DismissOnPointerOutside },
  overlay: { code: PropertyCode.Overlay },
  focusTrap: { code: PropertyCode.FocusTrap },
  restorePreviousFocus: { code: PropertyCode.RestorePreviousFocus },
  autoFocus: { code: PropertyCode.AutoFocus },
  ariaModal: { code: PropertyCode.AccessibilityModal },
  program: { code: PropertyCode.TerminalProgram },
  command: { code: PropertyCode.TerminalProgram },
  workingDirectory: { code: PropertyCode.TerminalWorkingDirectory },
  cwd: { code: PropertyCode.TerminalWorkingDirectory },
  scrollback: { code: PropertyCode.TerminalScrollback },
  terminalCursorColor: {
    code: PropertyCode.TerminalCursorColor,
    color: true,
  },
  terminalPaddingColor: { code: PropertyCode.TerminalPaddingColor },
  fontThicken: { code: PropertyCode.TerminalFontThicken },
  label: { code: PropertyCode.Value },
  systemImage: { code: PropertyCode.SwiftUISystemImage },
  buttonStyle: { code: PropertyCode.SwiftUIButtonStyle },
  controlSize: { code: PropertyCode.SwiftUIControlSize },
  target: { code: PropertyCode.SwiftUITarget },
  testID: { code: PropertyCode.SwiftUITestId },
  embeddedWindow: { code: PropertyCode.SwiftUIEmbeddedWindow },
  isPresented: { code: PropertyCode.SwiftUIIsPresented },
  attachmentAnchor: { code: PropertyCode.SwiftUIAttachmentAnchor },
  arrowEdge: { code: PropertyCode.SwiftUIArrowEdge },
};

const colorProperties = new Set([
  PropertyCode.BackgroundColor,
  PropertyCode.Color,
  PropertyCode.HoverBackgroundColor,
  PropertyCode.HoverColor,
  PropertyCode.ActiveBackgroundColor,
  PropertyCode.ActiveColor,
  PropertyCode.BorderColor,
  PropertyCode.MarkdownCodeBackground,
  PropertyCode.MarkdownBorderColor,
  PropertyCode.MarkdownMutedColor,
  PropertyCode.MarkdownLinkColor,
  PropertyCode.MarkdownCodeTextColor,
  PropertyCode.TerminalCursorColor,
]);

function setProperty(
  node: NativeNode,
  name: string,
  value: PropertyInput,
  previous?: PropertyInput,
) {
  if (name === "children" || name === "ref" || name === "key") return;
  if (name === "style") {
    setStyle(node, value, previous);
    return;
  }
  if (name === "anchor") {
    if (value === null || value === undefined || value === false) {
      setNativeProperty(node, PropertyCode.AnchorTarget, null);
    } else if (value instanceof NativeNode) {
      setNativeProperty(node, PropertyCode.AnchorTarget, String(value.id));
    } else {
      throw new TypeError("QuickGUI popover anchor must be a NativeNode");
    }
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
  if (name === "aria-modal") name = "ariaModal";
  if (name === "arguments" || name === "args") {
    setNativeProperty(
      node,
      PropertyCode.TerminalArguments,
      value === null || value === undefined
        ? null
        : encodeTerminalArguments(value),
    );
    return;
  }
  if (name === "environment" || name === "env") {
    setNativeProperty(
      node,
      PropertyCode.TerminalEnvironment,
      value === null || value === undefined
        ? null
        : encodeTerminalEnvironment(value),
    );
    return;
  }
  if (name === "terminalPalette") {
    setNativeProperty(
      node,
      PropertyCode.TerminalPalette,
      value === null || value === undefined
        ? null
        : encodeTerminalPalette(value),
    );
    return;
  }
  if (name === "type") {
    setNativeProperty(node, PropertyCode.Password, value === "password");
    return;
  }
  if (name === "flex") {
    setFlex(node, value);
    return;
  }
  if (name === "matchContents") {
    setMatchContents(node, value);
    return;
  }
  if (name === "modifiers") {
    setNativeProperty(
      node,
      PropertyCode.SwiftUIModifiers,
      encodeSwiftUiModifiers(value),
    );
    return;
  }
  const entry = properties[name];
  if (!entry) return;
  const normalized = entry.normalize
    ? entry.normalize(value)
    : normalizeValue(value, entry.code);
  setNativeProperty(node, entry.code, normalized, { color: !!entry.color });
}

function setMatchContents(node: NativeNode, value: PropertyInput): void {
  let horizontal = false;
  let vertical = false;
  if (value === true) {
    horizontal = true;
    vertical = true;
  } else if (isRecord(value)) {
    horizontal = value.horizontal === true;
    vertical = value.vertical === true;
  }
  setNativeProperty(
    node,
    PropertyCode.SwiftUIMatchContentsHorizontal,
    horizontal || null,
  );
  setNativeProperty(
    node,
    PropertyCode.SwiftUIMatchContentsVertical,
    vertical || null,
  );
}

function setStyle(
  node: NativeNode,
  value: PropertyInput,
  previous: PropertyInput,
): void {
  const next = isRecord(value) ? value : {};
  const old = isRecord(previous) ? previous : {};
  for (const name of Object.keys(old)) {
    if (!(name in next)) setProperty(node, name, null, old[name]);
  }
  for (const [name, nextValue] of Object.entries(next)) {
    if (!Object.is(nextValue, old[name]))
      setProperty(node, name, nextValue, old[name]);
  }
}

function setFlex(node: NativeNode, value: PropertyInput): void {
  if (value === null || value === undefined || value === false) {
    for (const code of [
      PropertyCode.FlexGrow,
      PropertyCode.FlexShrink,
      PropertyCode.FlexBasis,
    ]) {
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
  if (parts.length >= 1)
    setNativeProperty(node, PropertyCode.FlexGrow, Number(parts[0]));
  if (parts.length >= 2)
    setNativeProperty(node, PropertyCode.FlexShrink, Number(parts[1]));
  if (parts.length >= 3) {
    setNativeProperty(node, PropertyCode.FlexBasis, normalizeLength(parts[2]));
  }
}

function normalizeValue(
  value: PropertyInput,
  code: PropertyCode,
): boolean | number | string | null {
  if (value === null || value === undefined) return null;
  if (value === false) {
    return code === PropertyCode.Disabled ||
      code === PropertyCode.DismissOnEscape ||
      code === PropertyCode.DismissOnPointerOutside ||
      code === PropertyCode.FocusOnPointer
      ? false
      : null;
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

function normalizeTransitionShorthand(value: PropertyInput): number | null {
  if (value === null || value === undefined || value === false) return null;
  if (typeof value !== "string") {
    throw new TypeError(
      "QuickGUI transition must use the CSS transition shorthand",
    );
  }
  const shorthand = value.trim();
  if (shorthand === "" || shorthand === "none") return null;

  const declarations = splitCssList(shorthand);
  const supported = new Set(["background-color", "border-color", "color"]);
  const declared = new Set<string>();
  let sharedDuration: number | undefined;
  for (const declaration of declarations) {
    const property = declaration
      .split(/\s+/)
      .find((token) => supported.has(token));
    if (!property) {
      throw new TypeError(
        "QuickGUI transition currently supports background-color, border-color, and color",
      );
    }
    declared.add(property);
    const times = Array.from(
      declaration.matchAll(/(?:^|\s)(\d*\.?\d+)(ms|s)(?=\s|$)/g),
      (match) => Number(match[1]) * (match[2] === "s" ? 1_000 : 1),
    );
    const duration = times[0] ?? 0;
    const delay = times[1] ?? 0;
    if (delay !== 0) {
      throw new TypeError(
        "QuickGUI transition does not support a non-zero delay",
      );
    }
    if (sharedDuration !== undefined && sharedDuration !== duration) {
      throw new TypeError(
        "QuickGUI color transition properties must share one duration",
      );
    }
    sharedDuration = duration;
  }
  if (
    declared.size !== supported.size ||
    Array.from(supported).some((property) => !declared.has(property))
  ) {
    throw new TypeError(
      "QuickGUI color transition must declare background-color, border-color, and color",
    );
  }
  return sharedDuration ?? 0;
}

function splitCssList(value: string): string[] {
  const values: string[] = [];
  let start = 0;
  let depth = 0;
  for (let index = 0; index < value.length; index += 1) {
    const character = value[index];
    if (character === "(") depth += 1;
    else if (character === ")") depth = Math.max(0, depth - 1);
    else if (character === "," && depth === 0) {
      values.push(value.slice(start, index).trim());
      start = index + 1;
    }
  }
  values.push(value.slice(start).trim());
  return values.filter(Boolean);
}

function isLengthProperty(code: PropertyCode): boolean {
  return (
    (code >= PropertyCode.Gap && code <= PropertyCode.MarginLeft) ||
    code === PropertyCode.BorderWidth ||
    code === PropertyCode.BorderRadius ||
    code === PropertyCode.FontSize ||
    code === PropertyCode.LineHeight ||
    (code >= PropertyCode.Top && code <= PropertyCode.Left) ||
    code === PropertyCode.AnchorGap ||
    code === PropertyCode.ViewportMargin ||
    (code >= PropertyCode.HitSlop && code <= PropertyCode.HitSlopLeft)
  );
}

function eventName(
  name: string,
):
  | "click"
  | "mouseenter"
  | "mouseleave"
  | "input"
  | "submit"
  | "dismiss"
  | "terminal"
  | "pointer"
  | "presentationchange"
  | undefined {
  switch (name.toLowerCase()) {
    case "onclick":
    case "on:click":
    case "onpress":
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
    case "ondismiss":
    case "on:dismiss":
      return "dismiss";
    case "onstatus":
    case "onterminal":
    case "on:terminal":
      return "terminal";
    case "onpointer":
    case "on:pointer":
      return "pointer";
    case "onispresentedchange":
    case "onpresentationchange":
    case "on:presentationchange":
      return "presentationchange";
    default:
      return undefined;
  }
}

function encodeTerminalArguments(value: unknown): string {
  if (
    !Array.isArray(value) ||
    value.some((argument) => typeof argument !== "string")
  ) {
    throw new TypeError(
      "QuickGUI terminal arguments must be an array of strings",
    );
  }
  return JSON.stringify(value);
}

function encodeTerminalEnvironment(value: unknown): string {
  if (
    !isRecord(value) ||
    Object.entries(value).some(
      ([key, item]) => key.length === 0 || typeof item !== "string",
    )
  ) {
    throw new TypeError(
      "QuickGUI terminal environment must contain string keys and values",
    );
  }
  return JSON.stringify(value);
}

function encodeTerminalPalette(value: unknown): string {
  if (
    !Array.isArray(value) ||
    value.length !== 16 ||
    value.some(
      (color) => typeof color !== "string" && typeof color !== "number",
    )
  ) {
    throw new TypeError(
      "QuickGUI terminalPalette must contain exactly 16 colors",
    );
  }
  return JSON.stringify(value.map((color) => parseColor(color)));
}

function encodeSwiftUiModifiers(value: unknown): string | null {
  if (value === null || value === undefined) return null;
  if (!Array.isArray(value)) {
    throw new TypeError("QuickGUI SwiftUI modifiers must be an array");
  }
  const modifiers = value.map((modifier, index) => {
    if (!isRecord(modifier) || typeof modifier.$type !== "string") {
      throw new TypeError(
        `QuickGUI SwiftUI modifier ${index} must contain a $type`,
      );
    }
    switch (modifier.$type) {
      case "buttonStyle":
        return {
          $type: modifier.$type,
          style: swiftUiEnum(
            modifier.style,
            [
              "automatic",
              "bordered",
              "borderedProminent",
              "borderless",
              "glass",
              "glassProminent",
              "plain",
            ],
            modifier.$type,
          ),
        };
      case "buttonBorderShape": {
        const shape = swiftUiEnum(
          modifier.shape,
          ["automatic", "capsule", "roundedRectangle", "circle"],
          modifier.$type,
        );
        const cornerRadius = modifier.cornerRadius;
        if (
          cornerRadius !== undefined &&
          (typeof cornerRadius !== "number" ||
            !Number.isFinite(cornerRadius) ||
            cornerRadius < 0)
        ) {
          throw new TypeError(
            "QuickGUI buttonBorderShape cornerRadius must be a non-negative finite number",
          );
        }
        return {
          $type: modifier.$type,
          shape,
          ...(cornerRadius === undefined ? {} : { cornerRadius }),
        };
      }
      case "controlSize":
        return {
          $type: modifier.$type,
          size: swiftUiEnum(
            modifier.size,
            ["mini", "small", "regular", "large", "extraLarge"],
            modifier.$type,
          ),
        };
      case "labelStyle":
        return {
          $type: modifier.$type,
          style: swiftUiEnum(
            modifier.style,
            ["automatic", "iconOnly", "titleAndIcon", "titleOnly"],
            modifier.$type,
          ),
        };
      case "tint":
        if (typeof modifier.color !== "string" || modifier.color.length === 0) {
          throw new TypeError("QuickGUI tint color must be a non-empty string");
        }
        return { $type: modifier.$type, color: modifier.color };
      case "disabled":
        if (typeof modifier.disabled !== "boolean") {
          throw new TypeError(
            "QuickGUI disabled modifier must contain a boolean",
          );
        }
        return { $type: modifier.$type, disabled: modifier.disabled };
      default:
        throw new TypeError(
          `unsupported QuickGUI SwiftUI modifier \`${modifier.$type}\``,
        );
    }
  });
  return JSON.stringify(modifiers);
}

function swiftUiEnum(
  value: unknown,
  allowed: readonly string[],
  modifier: string,
): string {
  if (typeof value === "string" && allowed.includes(value)) return value;
  throw new TypeError(
    `QuickGUI ${modifier} must be one of ${allowed.join(", ")}`,
  );
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

const universal = createUniversalRenderer<NativeNode>({
  createElement(tag, staticProps) {
    const name = tag as NativeElementName;
    if (
      ![
        "view",
        "div",
        "text",
        "button",
        "input",
        "textarea",
        "markdown",
        "virtual-list",
        "terminal",
        "svg",
        "swift-ui-host",
        "swift-ui-button",
        "swift-ui-quickgui-host",
        "swift-ui-popover",
        "swift-ui-popover-trigger",
        "swift-ui-popover-content",
      ].includes(name)
    ) {
      throw new TypeError(`unknown QuickGUI element <${tag}>`);
    }
    const node = createNativeElement(name);
    if (staticProps) {
      for (const [name, value] of Object.entries(staticProps))
        setProperty(node, name, value);
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

/** Unstyled variable-height list; only visible child blocks are mounted by QuickGUI core. */
export function VirtualList(props: JSX.VirtualListProps): NativeNode {
  const node = universal.createElement("virtual-list");
  universal.spread(node, props);
  return node;
}

/** Real PTY terminal rendered by QuickGUI core through libghostty-vt. */
export function Terminal(props: JSX.TerminalProps): NativeNode {
  const node = universal.createElement("terminal");
  universal.spread(node, props);
  return node;
}

/** Parsed-once retained SVG mask tinted by the inherited `color` style. */
export function Svg(props: JSX.SvgProps): NativeNode {
  const node = universal.createElement("svg");
  universal.spread(node, props);
  return node;
}

export type TerminalStatusKind = "starting" | "running" | "exited" | "failed";

export interface TerminalStatusEvent {
  status: TerminalStatusKind;
  title: string;
  workingDirectory: string | null;
  processId?: number;
  exitCode?: number | null;
  signal?: string | null;
  message?: string;
  agent?: string;
  agentStatus?: "idle" | "working" | "blocked";
  agentProcessId?: number;
}

export type TerminalPalette = readonly [
  number | string,
  number | string,
  number | string,
  number | string,
  number | string,
  number | string,
  number | string,
  number | string,
  number | string,
  number | string,
  number | string,
  number | string,
  number | string,
  number | string,
  number | string,
  number | string,
];

/** Decode the structured payload delivered to a terminal's `onStatus` listener. */
export function terminalStatusFromEvent(
  event: QuickGuiEvent,
): TerminalStatusEvent {
  if (!event.value)
    throw new TypeError("QuickGUI terminal status event has no payload");
  return JSON.parse(event.value) as TerminalStatusEvent;
}

export type PointerPhase = "down" | "move" | "up" | "cancel";

export interface CapturedPointerEvent {
  phase: PointerPhase;
  position: { x: number; y: number };
  origin: { x: number; y: number };
  localPosition: { x: number; y: number };
  localOrigin: { x: number; y: number };
  delta: { x: number; y: number };
  button: "left" | "right" | "middle" | "back" | "forward" | "other";
}

/** Decode a Rust-core captured pointer payload. */
export function capturedPointerFromEvent(
  event: QuickGuiEvent,
): CapturedPointerEvent {
  if (!event.value)
    throw new TypeError("QuickGUI pointer event has no payload");
  return JSON.parse(event.value) as CapturedPointerEvent;
}

export type PopoverOpenChangeReason = "trigger-press" | "dismiss";

export interface PopoverOpenChangeDetails {
  reason: PopoverOpenChangeReason;
  event: QuickGuiEvent;
}

type PopoverSurface = "popover" | "system-popover";

interface PopoverContextValue {
  surface: PopoverSurface;
  open: () => boolean;
  anchor: () => NativeNode | undefined;
  dismissOnEscape: () => boolean;
  dismissOnPointerOutside: () => boolean;
  registerTrigger: (node: NativeNode) => void;
  unregisterTrigger: (node: NativeNode) => void;
  toggleFromTrigger: (node: NativeNode, event: QuickGuiEvent) => void;
  dismiss: (event: QuickGuiEvent) => void;
}

const PopoverContext = createContext<PopoverContextValue>();

function createPopoverRoot(
  surface: PopoverSurface,
  props: JSX.PopoverRootProps,
): NativeNode {
  const [uncontrolledOpen, setUncontrolledOpen] = createSignal(
    props.defaultOpen ?? false,
  );
  const [anchor, setAnchor] = createSignal<NativeNode>();
  const triggers = new Set<NativeNode>();
  const open = () => props.open ?? uncontrolledOpen();

  const changeOpen = (
    nextOpen: boolean,
    reason: PopoverOpenChangeReason,
    event: QuickGuiEvent,
  ) => {
    if (props.open === undefined) setUncontrolledOpen(nextOpen);
    props.onOpenChange?.(nextOpen, { reason, event });
  };

  const context: PopoverContextValue = {
    surface,
    open,
    anchor,
    dismissOnEscape: () => props.dismissOnEscape ?? true,
    dismissOnPointerOutside: () => props.dismissOnPointerOutside ?? true,
    registerTrigger(node) {
      triggers.add(node);
      if (!anchor()) setAnchor(node);
    },
    unregisterTrigger(node) {
      triggers.delete(node);
      if (anchor() === node) setAnchor(triggers.values().next().value);
    },
    toggleFromTrigger(node, event) {
      setAnchor(node);
      changeOpen(!open(), "trigger-press", event);
    },
    dismiss(event) {
      changeOpen(false, "dismiss", event);
    },
  };

  return PopoverContext({
    value: context,
    get children() {
      return props.children as SolidElement;
    },
  }) as unknown as NativeNode;
}

/** Logical root for an in-window popover. It does not create a native element. */
export function PopoverRoot(props: JSX.PopoverRootProps): NativeNode {
  return createPopoverRoot("popover", props);
}

/** Logical root for a native-window popover. It does not create a native window by itself. */
export function SystemPopoverRoot(props: JSX.PopoverRootProps): NativeNode {
  return createPopoverRoot("system-popover", props);
}

/** Trigger button shared by in-window and system popover roots. */
export function PopoverTrigger(props: JSX.PopoverTriggerProps): NativeNode {
  const context = useContext(PopoverContext);
  let trigger: NativeNode | undefined;
  const forwarded = universal.mergeProps(props, {
    ref: [
      (node: NativeNode) => {
        trigger = node;
        context.registerTrigger(node);
      },
      props.ref,
    ].filter(
      (value): value is (node: NativeNode) => void =>
        typeof value === "function",
    ),
    onClick(event: QuickGuiEvent) {
      props.onClick?.(event);
      if (!event.defaultPrevented && trigger)
        context.toggleFromTrigger(trigger, event);
    },
  }) as JSX.PopoverTriggerProps;
  const node = universal.createElement("button");
  universal.spread(node, forwarded);
  onCleanup(() => {
    if (trigger) context.unregisterTrigger(trigger);
  });
  return node;
}

function requirePopoverSurface(
  expected: PopoverSurface,
  component: string,
): PopoverContextValue {
  const context = useContext(PopoverContext);
  if (context.surface !== expected) {
    const root = expected === "popover" ? "Popover.Root" : "SystemPopover.Root";
    throw new TypeError(`${component} must be used inside <${root}>`);
  }
  return context;
}

function createInWindowPopoverContent(
  props: JSX.PopoverContentProps,
  context: PopoverContextValue,
  anchor: NativeNode,
): NativeNode {
  const surface = omit(
    props,
    "placement",
    "gap",
    "viewportMargin",
  ) as JSX.NativeProps;
  const node = universal.createElement("view");
  const forwarded = universal.mergeProps(surface, {
    anchor,
    get anchorPlacement() {
      return props.placement ?? "bottom-start";
    },
    get anchorGap() {
      return props.gap ?? 6;
    },
    get viewportMargin() {
      return props.viewportMargin ?? 8;
    },
    get dismissOnEscape() {
      return context.dismissOnEscape();
    },
    get dismissOnPointerOutside() {
      return context.dismissOnPointerOutside();
    },
    onDismiss(event: QuickGuiEvent) {
      context.dismiss(event);
    },
  }) as object;
  universal.spread(node, forwarded);
  return node;
}

/** Popover content rendered in the current window's retained overlay plane. */
export function PopoverContent(props: JSX.PopoverContentProps): NativeNode {
  const context = requirePopoverSurface("popover", "Popover.Content");
  return Show({
    keyed: true,
    get when() {
      return context.open() ? context.anchor() : undefined;
    },
    children: (anchor) => createInWindowPopoverContent(props, context, anchor),
  }) as unknown as NativeNode;
}

function createSystemPopoverContent(
  props: JSX.PopoverContentProps,
  context: PopoverContextValue,
  anchor: NativeNode,
): NativeNode {
  const owner = getOwner();
  const placeholder = createNativeSentinel();
  const surface = omit(
    props,
    "placement",
    "gap",
    "viewportMargin",
  ) as JSX.NativeProps;
  let systemWindow: Window | undefined;
  let disposing = false;

  // Initial JSX is rendered before its owner Window has a native handle. The microtask also makes
  // later mounts use the same lifecycle path instead of special-casing initial render.
  queueMicrotask(() => {
    if (disposing) return;
    systemWindow = new Window({
      title: "QuickGUI System Popover",
      anchor,
      width: props.width,
      height: props.height,
      placement: props.placement ?? "bottom-start",
      gap: props.gap ?? 6,
      viewportMargin: props.viewportMargin ?? 8,
      dismissOnEscape: context.dismissOnEscape(),
      dismissOnPointerOutside: context.dismissOnPointerOutside(),
      renderer: (window) =>
        runWithOwner(owner, () =>
          createRenderer(() => {
            const node = universal.createElement("view");
            universal.spread(node, surface);
            return node;
          })(window),
        ),
    });
    systemWindow.onClose(() => {
      systemWindow = undefined;
      if (disposing) return;
      // Leave the native close/disposal stack before controlled state unmounts this portal.
      queueMicrotask(() => {
        if (disposing) return;
        try {
          context.dismiss(new QuickGuiEvent("dismiss", placeholder));
        } finally {
          flushSolid();
        }
      });
    });
  });

  onCleanup(() => {
    disposing = true;
    systemWindow?.close();
    systemWindow = undefined;
  });

  return placeholder;
}

/** Popover content rendered through a separate Solid renderer in a native child window. */
export function SystemPopoverContent(
  props: JSX.PopoverContentProps,
): NativeNode {
  const context = requirePopoverSurface(
    "system-popover",
    "SystemPopover.Content",
  );
  return Show({
    keyed: true,
    get when() {
      return context.open() ? context.anchor() : undefined;
    },
    children: (anchor) => createSystemPopoverContent(props, context, anchor),
  }) as unknown as NativeNode;
}

/** Base-UI-shaped compound parts for an in-window retained popover. */
export const Popover = Object.assign(PopoverRoot, {
  Root: PopoverRoot,
  Trigger: PopoverTrigger,
  Content: PopoverContent,
});

/** Compound popover parts whose content uses a native child window. */
export const SystemPopover = Object.assign(SystemPopoverRoot, {
  Root: SystemPopoverRoot,
  Trigger: PopoverTrigger,
  Content: SystemPopoverContent,
});

export function createRenderer(code: () => JSX.Element): WindowRenderer {
  return (window) => {
    const nativeDispose = nativeRender(() => code() as NativeNode, window.root);
    let disposed = false;
    window.flush();
    return () => {
      if (disposed) return;
      disposed = true;
      nativeDispose();
      window.flush();
    };
  };
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
    alignItems?:
      | "start"
      | "flex-start"
      | "center"
      | "end"
      | "flex-end"
      | "baseline"
      | "stretch";
    alignSelf?: Style["alignItems"];
    justifyContent?:
      | "start"
      | "flex-start"
      | "center"
      | "end"
      | "flex-end"
      | "space-between"
      | "space-around"
      | "space-evenly";
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
    hoverBackgroundColor?: number | string;
    hoverColor?: number | string;
    activeBackgroundColor?: number | string;
    activeColor?: number | string;
    transition?: string;
    opacity?: number;
    borderWidth?: number | string;
    borderColor?: number | string;
    borderRadius?: number | string;
    fontSize?: number | string;
    fontFamily?: string;
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
    /** Keep keyboard focus where it is when this element is activated with a pointer. */
    focusOnPointer?: boolean;
    /** Paint this subtree in the viewport overlay plane above embedded native views. */
    overlay?: boolean;
    /** Contain keyboard focus within this subtree while it is the topmost trap. */
    focusTrap?: boolean;
    /** Restore the previously focused mounted control when this surface unmounts. */
    restorePreviousFocus?: boolean;
    /** Prefer this control when its containing focus trap takes focus. */
    autoFocus?: boolean;
    /** Expose modal semantics to assistive technology. */
    "aria-modal"?: boolean;
    ariaModal?: boolean;
    dismissOnEscape?: boolean;
    dismissOnPointerOutside?: boolean;
    hitSlop?: number | string;
    hitSlopTop?: number | string;
    hitSlopRight?: number | string;
    hitSlopBottom?: number | string;
    hitSlopLeft?: number | string;
    "aria-label"?: string;
    ariaLabel?: string;
    ref?: ((node: NativeNode) => void) | NativeNode;
    onClick?: EventHandler;
    onMouseEnter?: EventHandler;
    onMouseLeave?: EventHandler;
    onPointerEnter?: EventHandler;
    onPointerLeave?: EventHandler;
    /** Captured pointer stream from press through release/cancel, including outside the element. */
    onPointer?: EventHandler;
    onInput?: EventHandler;
    onChange?: EventHandler;
    onSubmit?: EventHandler;
    onDismiss?: EventHandler;
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

  export interface VirtualListProps extends NativeProps {
    estimatedItemHeight?: number;
    overscan?: number;
    listAlignment?: "top" | "bottom";
    followMode?: "normal" | "tail";
  }

  export interface TerminalProps extends NativeProps {
    /** Executable to launch. Omit to use the user's default shell. */
    program?: string;
    command?: string;
    arguments?: readonly string[];
    args?: readonly string[];
    workingDirectory?: string;
    cwd?: string;
    environment?: Readonly<Record<string, string>>;
    env?: Readonly<Record<string, string>>;
    scrollback?: number;
    /** Standard black-through-white colors followed by their eight bright variants. */
    terminalPalette?: TerminalPalette;
    terminalCursorColor?: number | string;
    /** Paint grid padding with the default background or extend edge-cell backgrounds into it. */
    terminalPaddingColor?: "background" | "extend";
    /** Optically thicken terminal glyph stems without selecting another font weight. */
    fontThicken?: boolean;
    onStatus?: EventHandler;
    onTerminal?: EventHandler;
  }

  export interface SvgProps extends NativeProps {
    /** Complete inline SVG document. External resources are ignored by the Rust core. */
    source: string;
  }

  export interface PopoverRootProps {
    children?: unknown;
    open?: boolean;
    defaultOpen?: boolean;
    onOpenChange?: (open: boolean, details: PopoverOpenChangeDetails) => void;
    dismissOnEscape?: boolean;
    dismissOnPointerOutside?: boolean;
  }

  export interface PopoverTriggerProps extends NativeProps {}

  export interface PopoverContentProps extends NativeProps {
    width: number;
    height: number;
    placement?: PopoverPlacement;
    gap?: number;
    viewportMargin?: number;
  }

  export interface IntrinsicElements {
    view: NativeProps;
    div: NativeProps;
    text: NativeProps;
    button: NativeProps;
    input: InputProps;
    textarea: InputProps;
    markdown: MarkdownProps;
    "virtual-list": VirtualListProps;
    terminal: TerminalProps;
    svg: SvgProps;
  }
}
