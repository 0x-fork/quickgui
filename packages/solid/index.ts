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
  type ColorValue,
  type NativeElementName,
  type NativeEventListener,
  type NativePartName,
  type PopoverPlacement,
  MAX_COMPONENT_JSON_BYTES,
  MAX_COMPONENT_VALUE_BYTES,
  MAX_DRAG_JSON_BYTES,
  MAX_KEYMAP_JSON_BYTES,
  MAX_MENU_JSON_BYTES,
  MAX_TOOLTIP_TEXT_BYTES,
  NativeNode,
  NativePart,
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

  opacity: { code: PropertyCode.Opacity },
  borderWidth: { code: PropertyCode.BorderWidth },
  borderTopWidth: { code: PropertyCode.BorderTopWidth },
  borderRightWidth: { code: PropertyCode.BorderRightWidth },
  borderBottomWidth: { code: PropertyCode.BorderBottomWidth },
  borderLeftWidth: { code: PropertyCode.BorderLeftWidth },
  borderColor: { code: PropertyCode.BorderColor, color: true },
  borderRadius: { code: PropertyCode.BorderRadius },
  boxShadow: {
    code: PropertyCode.BoxShadow,
    normalize: normalizeBoxShadow,
  },
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
  part: { code: PropertyCode.Part },
  scope: { code: PropertyCode.Scope, normalize: normalizeComponentValue },
  partValue: { code: PropertyCode.PartValue, normalize: normalizeComponentValue },
  activeValue: { code: PropertyCode.ActiveValue, normalize: normalizeComponentValue },
  checked: { code: PropertyCode.Checked },
  indeterminate: { code: PropertyCode.Indeterminate },
  orientation: { code: PropertyCode.Orientation },
  activateOnFocus: { code: PropertyCode.ActivateOnFocus },
  loopFocus: { code: PropertyCode.LoopFocus },
  keepMounted: { code: PropertyCode.KeepMounted },
  open: { code: PropertyCode.Open },
  itemIndex: { code: PropertyCode.ItemIndex },
  headingLevel: { code: PropertyCode.HeadingLevel },
  required: { code: PropertyCode.Required },
  invalid: { code: PropertyCode.Invalid },
  validationMessage: { code: PropertyCode.ValidationMessage },
  touched: { code: PropertyCode.Touched },
  dirty: { code: PropertyCode.Dirty },
  filled: { code: PropertyCode.Filled },
  tooltip: { code: PropertyCode.Tooltip, normalize: normalizeTooltipText },
  tooltipPlacement: { code: PropertyCode.TooltipPlacement },
  tooltipDelay: { code: PropertyCode.TooltipDelay },
  tooltipGap: { code: PropertyCode.TooltipGap },
  tooltipViewportMargin: { code: PropertyCode.TooltipViewportMargin },
  variant: { code: PropertyCode.Variant },
  menu: { code: PropertyCode.Menu },
  gridTemplateColumns: {
    code: PropertyCode.GridTemplateColumns,
    normalize: normalizeGridTemplate,
  },
  gridTemplateRows: {
    code: PropertyCode.GridTemplateRows,
    normalize: normalizeGridTemplate,
  },
  gridAutoFlow: { code: PropertyCode.GridAutoFlow },
  gridColumnStart: { code: PropertyCode.GridColumnStart },
  gridColumnEnd: { code: PropertyCode.GridColumnEnd },
  gridRowStart: { code: PropertyCode.GridRowStart },
  gridRowEnd: { code: PropertyCode.GridRowEnd },
  transitionProperty: { code: PropertyCode.TransitionProperties },
  transitionDuration: {
    code: PropertyCode.TransitionDuration,
    normalize: normalizeMilliseconds,
  },
  transitionTimingFunction: { code: PropertyCode.TransitionEasing },
  transitionEasing: { code: PropertyCode.TransitionEasing },
  transitionMaxFps: { code: PropertyCode.TransitionMaxFps },
  min: { code: PropertyCode.Minimum },
  max: { code: PropertyCode.Maximum },
  low: { code: PropertyCode.Low },
  high: { code: PropertyCode.High },
  optimum: { code: PropertyCode.Optimum },
  valueText: { code: PropertyCode.ValueText },
  pressed: { code: PropertyCode.Pressed },
  values: { code: PropertyCode.Values, normalize: normalizeComponentJson },
  items: { code: PropertyCode.Items, normalize: normalizeComponentJson },
  step: { code: PropertyCode.Step },
  largeStep: { code: PropertyCode.LargeStep },
  objectFit: { code: PropertyCode.ObjectFit },
  fit: { code: PropertyCode.ObjectFit },
  shaderParameters: {
    code: PropertyCode.ShaderParameters,
    normalize: normalizeShaderParameters,
  },
};

/**
 * Property codes whose `false` is a declaration, not an absence.
 *
 * The Rust core defaults some of these to `true`, so the renderer must transmit the explicit
 * negative instead of clearing the property.
 */
const explicitFalseProperties = new Set<PropertyCode>([
  PropertyCode.Disabled,
  PropertyCode.DismissOnEscape,
  PropertyCode.DismissOnPointerOutside,
  PropertyCode.FocusOnPointer,
  PropertyCode.Checked,
  PropertyCode.Indeterminate,
  PropertyCode.Pressed,
  PropertyCode.ActivateOnFocus,
  PropertyCode.LoopFocus,
  PropertyCode.KeepMounted,
  PropertyCode.Open,
  PropertyCode.Required,
  PropertyCode.Invalid,
  PropertyCode.Touched,
  PropertyCode.Dirty,
  PropertyCode.Filled,
]);

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
  if (name === "controls") {
    if (value === null || value === undefined || value === false) {
      setNativeProperty(node, PropertyCode.Controls, null);
    } else if (value instanceof NativeNode) {
      setNativeProperty(node, PropertyCode.Controls, String(value.id));
    } else {
      throw new TypeError("QuickGUI controls target must be a NativeNode");
    }
    return;
  }
  if (name === "transition") {
    setTransition(node, value);
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
  if (name === "keymap") {
    setNativeProperty(node, PropertyCode.Keymap, encodeKeymap(value));
    return;
  }
  if (name === "draggable") {
    setNativeProperty(node, PropertyCode.Draggable, encodeDragSource(value));
    return;
  }
  if (name === "dropKinds") {
    setNativeProperty(node, PropertyCode.DropKinds, encodeDropKinds(value));
    return;
  }
  if (name === "gridColumn" || name === "gridRow") {
    setGridPlacement(node, name === "gridColumn", value);
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
  if (value === false) return explicitFalseProperties.has(code) ? false : null;
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

/** Paint properties the Rust core can transition, keyed by their CSS-shaped names. */
const transitionProperties = new Map<string, string>([
  ["all", "all"],
  ["background", "background-color"],
  ["background-color", "background-color"],
  ["border-color", "border-color"],
  ["border-width", "border-width"],
  ["border-radius", "border-radius"],
  ["color", "color"],
  ["box-shadow", "box-shadow"],
  ["opacity", "opacity"],
]);

const transitionEasings = new Set([
  "linear",
  "ease",
  "ease-in",
  "ease-out",
  "ease-in-out",
]);

/** Duration in milliseconds from a `120ms`, `0.2s`, or plain-number declaration. */
function normalizeMilliseconds(value: PropertyInput): number | null {
  if (value === null || value === undefined || value === false) return null;
  if (typeof value === "number") {
    return Number.isFinite(value) ? value : null;
  }
  const text = String(value).trim();
  const match = text.match(/^(\d*\.?\d+)(ms|s)?$/);
  if (!match) return null;
  return Number(match[1]) * (match[2] === "s" ? 1_000 : 1);
}

/**
 * Declare a complete paint transition.
 *
 * The Rust core owns interpolation, cadence, and which paint properties can transition; this only
 * translates the CSS-shaped declaration into the property, duration, and easing the core reads.
 */
function setTransition(node: NativeNode, value: PropertyInput): void {
  const clear = () => {
    for (const code of [
      PropertyCode.Transition,
      PropertyCode.TransitionProperties,
      PropertyCode.TransitionDuration,
      PropertyCode.TransitionEasing,
      PropertyCode.TransitionMaxFps,
    ]) {
      setNativeProperty(node, code, null);
    }
  };
  if (value === null || value === undefined || value === false) {
    clear();
    return;
  }
  if (typeof value === "number") {
    clear();
    setNativeProperty(node, PropertyCode.Transition, value);
    return;
  }
  if (isRecord(value)) {
    clear();
    const duration = normalizeMilliseconds(value.duration as PropertyInput);
    if (duration !== null) {
      setNativeProperty(node, PropertyCode.TransitionDuration, duration);
    }
    const declared = value.property ?? value.properties;
    if (declared !== undefined && declared !== null) {
      const names = (Array.isArray(declared) ? declared : [declared])
        .map((name) => normalizeTransitionProperty(String(name)))
        .join(",");
      setNativeProperty(node, PropertyCode.TransitionProperties, names);
    }
    if (typeof value.easing === "string" || typeof value.timingFunction === "string") {
      setNativeProperty(
        node,
        PropertyCode.TransitionEasing,
        normalizeTransitionEasing(String(value.easing ?? value.timingFunction)),
      );
    }
    if (typeof value.maxFps === "number") {
      setNativeProperty(node, PropertyCode.TransitionMaxFps, value.maxFps);
    }
    if (value.delay !== undefined && Number(value.delay) !== 0) {
      throw new TypeError(
        "QuickGUI transitions do not support a non-zero delay",
      );
    }
    return;
  }
  if (typeof value !== "string") {
    throw new TypeError(
      "QuickGUI transition must use the CSS transition shorthand",
    );
  }
  const shorthand = value.trim();
  clear();
  if (shorthand === "" || shorthand === "none") return;

  const declared = new Set<string>();
  let sharedDuration: number | undefined;
  let easing: string | undefined;
  for (const declaration of splitCssList(shorthand)) {
    const tokens = declaration.split(/\s+/).filter(Boolean);
    const property = tokens.find((token) => transitionProperties.has(token));
    if (!property) {
      throw new TypeError(
        `QuickGUI cannot transition \`${declaration}\`; the core transitions background-color, border-color, border-width, border-radius, color, box-shadow, and opacity`,
      );
    }
    declared.add(transitionProperties.get(property)!);
    const times = Array.from(
      declaration.matchAll(/(?:^|\s)(\d*\.?\d+)(ms|s)(?=\s|$)/g),
      (match) => Number(match[1]) * (match[2] === "s" ? 1_000 : 1),
    );
    const duration = times[0] ?? 0;
    if ((times[1] ?? 0) !== 0) {
      throw new TypeError(
        "QuickGUI transitions do not support a non-zero delay",
      );
    }
    if (sharedDuration !== undefined && sharedDuration !== duration) {
      throw new TypeError(
        "QuickGUI transition properties must share one duration",
      );
    }
    sharedDuration = duration;
    const declaredEasing = tokens.find((token) => transitionEasings.has(token));
    if (declaredEasing) easing = declaredEasing;
  }
  setNativeProperty(node, PropertyCode.Transition, sharedDuration ?? 0);
  setNativeProperty(
    node,
    PropertyCode.TransitionProperties,
    Array.from(declared).join(","),
  );
  if (easing) {
    setNativeProperty(
      node,
      PropertyCode.TransitionEasing,
      normalizeTransitionEasing(easing),
    );
  }
}

function normalizeTransitionProperty(name: string): string {
  const normalized = transitionProperties.get(name.trim());
  if (!normalized) {
    throw new TypeError(`QuickGUI cannot transition \`${name}\``);
  }
  return normalized;
}

function normalizeTransitionEasing(name: string): string {
  const normalized = name.trim();
  if (!transitionEasings.has(normalized)) {
    throw new TypeError(
      `QuickGUI does not expose the \`${normalized}\` easing curve`,
    );
  }
  return normalized === "ease" ? "ease-in-out" : normalized;
}

/** Normalize a CSS grid track list. A plain number declares that many equal `1fr` tracks. */
function normalizeGridTemplate(value: PropertyInput): number | string | null {
  if (value === null || value === undefined || value === false) return null;
  if (typeof value === "number") {
    return Number.isFinite(value) ? value : null;
  }
  const tracks = Array.isArray(value) ? value.join(" ") : String(value);
  const normalized = tracks.trim().replace(/\s+/g, " ");
  return normalized === "" || normalized === "none" ? null : normalized;
}

/**
 * Split a `grid-column` / `grid-row` shorthand into the core's line and span placement.
 *
 * `2 / span 3` is the same placement as lines `2` through `5`, so a declared start plus span
 * becomes an explicit end and the core never needs a combined form it does not expose.
 */
function setGridPlacement(
  node: NativeNode,
  column: boolean,
  value: PropertyInput,
): void {
  const startCode = column
    ? PropertyCode.GridColumnStart
    : PropertyCode.GridRowStart;
  const endCode = column ? PropertyCode.GridColumnEnd : PropertyCode.GridRowEnd;
  const spanCode = column
    ? PropertyCode.GridColumnSpan
    : PropertyCode.GridRowSpan;
  setNativeProperty(node, startCode, null);
  setNativeProperty(node, endCode, null);
  setNativeProperty(node, spanCode, null);
  if (value === null || value === undefined || value === false) return;
  if (typeof value === "number") {
    setNativeProperty(node, startCode, value);
    return;
  }
  const [rawStart = "", rawEnd = ""] = String(value).split("/");
  const start = rawStart.trim();
  const end = rawEnd.trim();
  const startSpan = start.match(/^span\s+(\d+)$/);
  const endSpan = end.match(/^span\s+(\d+)$/);
  const startLine = start === "" || start === "auto" ? null : Number(start);
  if (startSpan) {
    setNativeProperty(node, spanCode, Number(startSpan[1]));
    return;
  }
  if (startLine !== null && Number.isFinite(startLine)) {
    setNativeProperty(node, startCode, startLine);
  }
  if (endSpan) {
    const span = Number(endSpan[1]);
    if (startLine !== null && Number.isFinite(startLine)) {
      setNativeProperty(node, endCode, startLine + span);
    } else {
      setNativeProperty(node, spanCode, span);
    }
    return;
  }
  const endLine = end === "" || end === "auto" ? null : Number(end);
  if (endLine !== null && Number.isFinite(endLine)) {
    setNativeProperty(node, endCode, endLine);
  }
}

/** Bounded shader parameter floats, packed into the core's four fixed vectors. */
function normalizeShaderParameters(value: PropertyInput): string | null {
  if (value === null || value === undefined || value === false) return null;
  if (!Array.isArray(value)) {
    throw new TypeError("QuickGUI shader parameters must be an array of numbers");
  }
  const floats = value.flat(2).map((entry) => {
    const number = Number(entry);
    if (!Number.isFinite(number)) {
      throw new TypeError("QuickGUI shader parameters must be finite numbers");
    }
    return number;
  });
  if (floats.length > MAX_SHADER_PARAMETER_FLOATS) {
    throw new RangeError(
      `QuickGUI exposes ${MAX_SHADER_PARAMETER_FLOATS} shader parameter floats`,
    );
  }
  return JSON.stringify(floats);
}

/** Four four-component vectors, matching the Rust core's fixed shader uniform. */
export const MAX_SHADER_PARAMETER_FLOATS = 16;

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

const MAX_BOX_SHADOWS_PER_ELEMENT = 8;

type EncodedBoxShadow = {
  offsetX: number;
  offsetY: number;
  blurRadius: number;
  spreadRadius: number;
  color: number | null;
  inset: boolean;
};

function normalizeBoxShadow(value: PropertyInput): string | null {
  if (value === null || value === undefined || value === false) return null;
  if (typeof value !== "string") {
    throw new TypeError("QuickGUI boxShadow must use the CSS box-shadow shorthand");
  }
  const shorthand = value.trim();
  if (shorthand === "" || shorthand.toLowerCase() === "none") return null;

  const declarations = splitCssList(shorthand);
  if (declarations.length > MAX_BOX_SHADOWS_PER_ELEMENT) {
    throw new TypeError(
      `QuickGUI boxShadow supports at most ${MAX_BOX_SHADOWS_PER_ELEMENT} shadows`,
    );
  }
  return JSON.stringify(declarations.map(parseBoxShadowDeclaration));
}

function parseBoxShadowDeclaration(declaration: string): EncodedBoxShadow {
  const lengths: number[] = [];
  let color: number | null = null;
  let hasColor = false;
  let inset = false;

  for (const token of splitCssTokens(declaration)) {
    if (token.toLowerCase() === "inset") {
      if (inset) throw new TypeError("QuickGUI boxShadow repeats `inset`");
      inset = true;
      continue;
    }
    const length = parseShadowLength(token);
    if (length !== undefined) {
      if (lengths.length === 4) {
        throw new TypeError(
          "QuickGUI boxShadow accepts two to four length values",
        );
      }
      lengths.push(length);
      continue;
    }
    if (hasColor) {
      throw new TypeError("QuickGUI boxShadow accepts one color per shadow");
    }
    hasColor = true;
    if (token.toLowerCase() !== "currentcolor") {
      color = parseColor(token);
    }
  }

  if (lengths.length < 2) {
    throw new TypeError(
      "QuickGUI boxShadow requires horizontal and vertical offsets",
    );
  }
  const blurRadius = lengths[2] ?? 0;
  if (blurRadius < 0) {
    throw new TypeError("QuickGUI boxShadow blur radius cannot be negative");
  }
  return {
    offsetX: lengths[0]!,
    offsetY: lengths[1]!,
    blurRadius,
    spreadRadius: lengths[3] ?? 0,
    color,
    inset,
  };
}

function splitCssTokens(value: string): string[] {
  const tokens: string[] = [];
  let start = 0;
  let depth = 0;
  for (let index = 0; index < value.length; index += 1) {
    const character = value[index]!;
    if (character === "(") depth += 1;
    else if (character === ")") {
      depth -= 1;
      if (depth < 0) {
        throw new TypeError("QuickGUI boxShadow has unbalanced parentheses");
      }
    } else if (/\s/.test(character) && depth === 0) {
      const token = value.slice(start, index).trim();
      if (token) tokens.push(token);
      start = index + 1;
    }
  }
  if (depth !== 0) {
    throw new TypeError("QuickGUI boxShadow has unbalanced parentheses");
  }
  const token = value.slice(start).trim();
  if (token) tokens.push(token);
  return tokens;
}

function parseShadowLength(value: string): number | undefined {
  const match = value.match(
    /^([+-]?(?:\d+(?:\.\d*)?|\.\d+))(?:px)?$/i,
  );
  if (!match) return undefined;
  const length = Number(match[1]);
  return Number.isFinite(length) ? length : undefined;
}

const textEncoder = new TextEncoder();

function normalizeComponentValue(value: PropertyInput): string | null {
  if (value === null || value === undefined || value === false) return null;
  const text = String(value);
  if (text.length === 0) return null;
  if (textEncoder.encode(text).length > MAX_COMPONENT_VALUE_BYTES) {
    throw new TypeError(
      `QuickGUI component scopes and values are limited to ${MAX_COMPONENT_VALUE_BYTES} bytes`,
    );
  }
  return text;
}

/**
 * Serialize one declared component list into the bounded JSON the Rust binding decodes.
 *
 * Slider thumb values, splitter pane sizes, toggle-group pressed values, and ordered toolbar or
 * toggle-group items all travel as one declaration so the core can answer a keypress without ever
 * asking JavaScript a synchronous question.
 */
function normalizeComponentJson(value: PropertyInput): string | null {
  if (value === null || value === undefined || value === false) return null;
  const encoded = Array.isArray(value) ? JSON.stringify(value) : String(value);
  if (encoded.length === 0 || encoded === "[]") return encoded === "[]" ? encoded : null;
  if (textEncoder.encode(encoded).length > MAX_COMPONENT_JSON_BYTES) {
    throw new TypeError(
      `QuickGUI component declarations are limited to ${MAX_COMPONENT_JSON_BYTES} bytes`,
    );
  }
  return encoded;
}

function normalizeTooltipText(value: PropertyInput): string | null {
  if (value === null || value === undefined || value === false) return null;
  const text = String(value);
  if (text.length === 0) return null;
  if (textEncoder.encode(text).length > MAX_TOOLTIP_TEXT_BYTES) {
    throw new TypeError(
      `QuickGUI tooltip text is limited to ${MAX_TOOLTIP_TEXT_BYTES} bytes`,
    );
  }
  return text;
}

function isLengthProperty(code: PropertyCode): boolean {
  return (
    (code >= PropertyCode.Gap && code <= PropertyCode.MarginLeft) ||
    code === PropertyCode.BorderWidth ||
    (code >= PropertyCode.BorderTopWidth &&
      code <= PropertyCode.BorderLeftWidth) ||
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
  | "menuselect"
  | "keydown"
  | "keyup"
  | "mousedown"
  | "mouseup"
  | "mousemove"
  | "dblclick"
  | "wheel"
  | "contextmenu"
  | "pinch"
  | "rotate"
  | "smartmagnify"
  | "pressure"
  | "focus"
  | "blur"
  | "action"
  | "dragstart"
  | "dragend"
  | "drop"
  | "filesdropped"
  | "componentchange"
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
    case "onselect":
    case "on:select":
    case "onmenuselect":
      return "menuselect";
    case "oncomponentchange":
    case "on:componentchange":
      return "componentchange";
    case "onkeydown":
    case "on:keydown":
      return "keydown";
    case "onkeyup":
    case "on:keyup":
      return "keyup";
    case "onmousedown":
    case "onpointerdown":
      return "mousedown";
    case "onmouseup":
    case "onpointerup":
      return "mouseup";
    case "onmousemove":
    case "onpointermove":
      return "mousemove";
    case "ondoubleclick":
    case "ondblclick":
      return "dblclick";
    case "onwheel":
    case "onscrollwheel":
      return "wheel";
    case "oncontextmenu":
    case "on:contextmenu":
      return "contextmenu";
    case "onpinch":
      return "pinch";
    case "onrotate":
    case "onrotation":
      return "rotate";
    case "onsmartmagnify":
      return "smartmagnify";
    case "onpressure":
      return "pressure";
    case "onfocus":
      return "focus";
    case "onblur":
      return "blur";
    case "onaction":
    case "on:action":
      return "action";
    case "ondragstart":
      return "dragstart";
    case "ondragend":
      return "dragend";
    case "ondrop":
      return "drop";
    case "onfilesdropped":
      return "filesdropped";
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
        "image",
        "shader",
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

// ---------------------------------------------------------------------------
// Compound component parts
//
// Every part below is one ordinary native node that declares which Rust core part descriptor the
// binding must rebuild. The core owns identity, semantics, keyboard behavior, and whether an
// inactive panel is mounted at all; Solid owns only the controlled value, the compound context
// that saves applications from repeating it, and the unstyled element tree.
// ---------------------------------------------------------------------------

let nextComponentScope = 1;

/**
 * Allocate one bounded scope key shared by every part of a compound component instance.
 *
 * The Rust binding hashes it into the same `ElementId` the core component would have used, so
 * derived part identities and accessibility relationships resolve with no registry and no
 * synchronous question asked of JavaScript.
 */
function createComponentScope(prefix: string): string {
  return `${prefix}-${nextComponentScope++}`;
}

function createHostNode(
  element: NativeElementName,
  props: unknown,
): NativeNode {
  const node = universal.createElement(element);
  universal.spread(node, props as object);
  return node;
}

function createPartNode(
  element: NativeElementName,
  props: unknown,
  part: unknown,
): NativeNode {
  const node = universal.createElement(element);
  universal.spread(
    node,
    universal.mergeProps(props as object, part as object) as object,
  );
  return node;
}

function forwardClick(
  handler: ((event: QuickGuiEvent) => void) | undefined,
  activate: (event: QuickGuiEvent) => void,
): (event: QuickGuiEvent) => void {
  return (event) => {
    handler?.(event);
    if (!event.defaultPrevented) activate(event);
  };
}

export type CheckedState = boolean | "indeterminate";

/** Controlled, unstyled checkbox root carrying the core's exact on/off/mixed toggle state. */
export function CheckboxRoot(props: JSX.CheckboxProps): NativeNode {
  const [uncontrolled, setUncontrolled] = createSignal<CheckedState>(
    props.defaultChecked ?? false,
  );
  const checked = () => props.checked ?? uncontrolled();
  return createPartNode(
    "button",
    omit(props, "checked", "defaultChecked", "onCheckedChange"),
    {
      part: NativePart.Checkbox,
      get checked() {
        return checked() === true;
      },
      get indeterminate() {
        return checked() === "indeterminate";
      },
      onClick: forwardClick(props.onClick, (event) => {
        const next = checked() !== true;
        if (props.checked === undefined) setUncontrolled(next);
        props.onCheckedChange?.(next, event);
      }),
    },
  );
}

/** Application-owned checkbox mark, hidden from the control's accessible name by the core. */
export function CheckboxIndicator(props: JSX.NativeProps): NativeNode {
  return createPartNode("view", props, { part: NativePart.CheckboxIndicator });
}

/** Base-UI-shaped compound parts for a controlled checkbox. */
export const Checkbox = Object.assign(CheckboxRoot, {
  Root: CheckboxRoot,
  Indicator: CheckboxIndicator,
});

interface RadioGroupContextValue {
  value: () => string | undefined;
  select: (value: string, event: QuickGuiEvent) => void;
}

const RadioGroupContext = createContext<RadioGroupContextValue | null>(null);

/** Semantic radio-group root. The core supplies roving Tab and arrow behavior from the tree. */
export function RadioGroupRoot(props: JSX.RadioGroupProps): NativeNode {
  const [uncontrolled, setUncontrolled] = createSignal(props.defaultValue);
  const value = () => props.value ?? uncontrolled();
  const context: RadioGroupContextValue = {
    value,
    select(next, event) {
      if (props.value === undefined) setUncontrolled(next);
      props.onValueChange?.(next, event);
    },
  };
  return createPartNode(
    "view",
    omit(props, "value", "defaultValue", "onValueChange", "children"),
    {
      part: NativePart.RadioGroup,
      get children() {
        return RadioGroupContext({
          value: context,
          get children() {
            return props.children as SolidElement;
          },
        });
      },
    },
  );
}

/** Controlled radio root. Inside a `RadioGroup` its selection comes from the group value. */
export function RadioRoot(props: JSX.RadioProps): NativeNode {
  const group = useContext(RadioGroupContext);
  const [uncontrolled, setUncontrolled] = createSignal(
    props.defaultChecked ?? false,
  );
  const checked = () =>
    group ? group.value() === props.value : (props.checked ?? uncontrolled());
  return createPartNode(
    "button",
    omit(props, "value", "checked", "defaultChecked", "onCheckedChange"),
    {
      part: NativePart.Radio,
      get checked() {
        return checked();
      },
      get partValue() {
        return props.value;
      },
      onClick: forwardClick(props.onClick, (event) => {
        if (group) {
          if (props.value !== undefined) group.select(props.value, event);
          return;
        }
        if (props.checked === undefined) setUncontrolled(true);
        props.onCheckedChange?.(true, event);
      }),
    },
  );
}

/** Application-owned radio dot, hidden from the control's accessible name by the core. */
export function RadioIndicator(props: JSX.NativeProps): NativeNode {
  return createPartNode("view", props, { part: NativePart.RadioIndicator });
}

/** Base-UI-shaped compound parts for a controlled radio button. */
export const Radio = Object.assign(RadioRoot, {
  Root: RadioRoot,
  Indicator: RadioIndicator,
});

/** Semantic group for related radio roots. */
export const RadioGroup = Object.assign(RadioGroupRoot, {
  Root: RadioGroupRoot,
});

/** Controlled, unstyled switch root/track carrying the core's switch role. */
export function SwitchRoot(props: JSX.SwitchProps): NativeNode {
  const [uncontrolled, setUncontrolled] = createSignal(
    props.defaultChecked ?? false,
  );
  const checked = () => props.checked ?? uncontrolled();
  return createPartNode(
    "button",
    omit(props, "checked", "defaultChecked", "onCheckedChange"),
    {
      part: NativePart.Switch,
      get checked() {
        return checked();
      },
      onClick: forwardClick(props.onClick, (event) => {
        const next = !checked();
        if (props.checked === undefined) setUncontrolled(next);
        props.onCheckedChange?.(next, event);
      }),
    },
  );
}

/** Application-owned switch thumb, hidden from the control's accessible name by the core. */
export function SwitchThumb(props: JSX.NativeProps): NativeNode {
  return createPartNode("view", props, { part: NativePart.SwitchThumb });
}

/** Base-UI-shaped compound parts for a controlled switch. */
export const Switch = Object.assign(SwitchRoot, {
  Root: SwitchRoot,
  Thumb: SwitchThumb,
});

interface TabsContextValue {
  scope: string;
  value: () => string | undefined;
  orientation: () => "horizontal" | "vertical";
  activation: () => "manual" | "automatic";
  loop: () => boolean;
  keepMounted: () => boolean;
  select: (value: string, event: QuickGuiEvent) => void;
}

const TabsContext = createContext<TabsContextValue | null>(null);
const TabValueContext = createContext<(() => string) | null>(null);

function requireTabs(component: string): TabsContextValue {
  const context = useContext(TabsContext);
  if (!context) {
    throw new TypeError(`${component} must be used inside <Tabs.Root>`);
  }
  return context;
}

/** Shared declaration every tab part repeats so the Rust binding decodes it without a registry. */
function tabsPartProps(context: TabsContextValue): object {
  return {
    get scope() {
      return context.scope;
    },
    get activeValue() {
      return context.value();
    },
    get orientation() {
      return context.orientation();
    },
    get activateOnFocus() {
      return context.activation() === "automatic";
    },
    get loopFocus() {
      return context.loop();
    },
    get keepMounted() {
      return context.keepMounted();
    },
  };
}

/** Controlled, unstyled tab set. The core owns roving focus, arrow keys, and panel mounting. */
export function TabsRoot(props: JSX.TabsRootProps): NativeNode {
  const scope = createComponentScope("qg-tabs");
  const [uncontrolled, setUncontrolled] = createSignal(props.defaultValue);
  const value = () => props.value ?? uncontrolled();
  const context: TabsContextValue = {
    scope,
    value,
    orientation: () => props.orientation ?? "horizontal",
    activation: () => props.activation ?? "manual",
    loop: () => props.loop ?? true,
    keepMounted: () => props.keepMounted === true,
    select(next, event) {
      if (props.value === undefined) setUncontrolled(next);
      props.onValueChange?.(next, event);
    },
  };
  return createPartNode(
    "view",
    omit(
      props,
      "value",
      "defaultValue",
      "onValueChange",
      "orientation",
      "activation",
      "loop",
      "keepMounted",
      "children",
    ),
    universal.mergeProps(tabsPartProps(context), {
      part: NativePart.Tabs,
      get children() {
        return TabsContext({
          value: context,
          get children() {
            return props.children as SolidElement;
          },
        });
      },
    }),
  );
}

/** Tab-list root. The core attaches its exact arrow/Home/End navigation behavior here. */
export function TabsList(props: JSX.NativeProps): NativeNode {
  const context = requireTabs("Tabs.List");
  return createPartNode(
    "view",
    props,
    universal.mergeProps(tabsPartProps(context), { part: NativePart.TabsList }),
  );
}

/** One controlled tab. Activation, roles, and the panel relationship come from the core. */
export function TabsTab(props: JSX.TabsTabProps): NativeNode {
  const context = requireTabs("Tabs.Tab");
  const value = () => props.value;
  return createPartNode(
    "button",
    omit(props, "value", "children"),
    universal.mergeProps(tabsPartProps(context), {
      part: NativePart.Tab,
      get partValue() {
        return props.value;
      },
      onClick: forwardClick(props.onClick, (event) =>
        context.select(props.value, event),
      ),
      get children() {
        return TabValueContext({
          value,
          get children() {
            return props.children as SolidElement;
          },
        });
      },
    }),
  );
}

/** Decorative indicator mounted by the core only while its tab is active. */
export function TabsIndicator(props: JSX.TabsIndicatorProps): NativeNode {
  const context = requireTabs("Tabs.Indicator");
  const inherited = useContext(TabValueContext);
  return createPartNode(
    "view",
    omit(props, "value"),
    universal.mergeProps(tabsPartProps(context), {
      part: NativePart.TabIndicator,
      get partValue() {
        return props.value ?? inherited?.() ?? context.value();
      },
    }),
  );
}

/** One tab panel. The core omits it, or retains it hidden with `keepMounted`, when inactive. */
export function TabsPanel(props: JSX.TabsPanelProps): NativeNode {
  const context = requireTabs("Tabs.Panel");
  return createPartNode(
    "view",
    omit(props, "value"),
    universal.mergeProps(tabsPartProps(context), {
      part: NativePart.TabPanel,
      get partValue() {
        return props.value;
      },
    }),
  );
}

/** Base-UI-shaped compound parts for a controlled tab set. */
export const Tabs = Object.assign(TabsRoot, {
  Root: TabsRoot,
  List: TabsList,
  Tab: TabsTab,
  Indicator: TabsIndicator,
  Panel: TabsPanel,
});

interface CollapsibleContextValue {
  scope: string;
  open: () => boolean;
  disabled: () => boolean;
  keepMounted: () => boolean;
  toggle: (event: QuickGuiEvent) => void;
}

const CollapsibleContext = createContext<CollapsibleContextValue | null>(null);

function requireCollapsible(component: string): CollapsibleContextValue {
  const context = useContext(CollapsibleContext);
  if (!context) {
    throw new TypeError(`${component} must be used inside <Collapsible.Root>`);
  }
  return context;
}

function collapsiblePartProps(context: CollapsibleContextValue): object {
  return {
    get scope() {
      return context.scope;
    },
    get open() {
      return context.open();
    },
    get disabled() {
      return context.disabled();
    },
    get keepMounted() {
      return context.keepMounted();
    },
  };
}

/** Controlled, unstyled disclosure root. */
export function CollapsibleRoot(props: JSX.CollapsibleRootProps): NativeNode {
  const scope = createComponentScope("qg-collapsible");
  const [uncontrolled, setUncontrolled] = createSignal(
    props.defaultOpen ?? false,
  );
  const open = () => props.open ?? uncontrolled();
  const context: CollapsibleContextValue = {
    scope,
    open,
    disabled: () => props.disabled === true,
    keepMounted: () => props.keepMounted === true,
    toggle(event) {
      const next = !open();
      if (props.open === undefined) setUncontrolled(next);
      props.onOpenChange?.(next, event);
    },
  };
  return createPartNode(
    "view",
    omit(props, "open", "defaultOpen", "onOpenChange", "keepMounted", "children"),
    universal.mergeProps(collapsiblePartProps(context), {
      part: NativePart.Collapsible,
      get children() {
        return CollapsibleContext({
          value: context,
          get children() {
            return props.children as SolidElement;
          },
        });
      },
    }),
  );
}

/** Disclosure button. Expanded state and the panel relationship come from the core. */
export function CollapsibleTrigger(props: JSX.NativeProps): NativeNode {
  const context = requireCollapsible("Collapsible.Trigger");
  return createPartNode(
    "button",
    props,
    universal.mergeProps(collapsiblePartProps(context), {
      part: NativePart.CollapsibleTrigger,
      onClick: forwardClick(props.onClick, (event) => context.toggle(event)),
    }),
  );
}

/** Disclosure panel. The core omits it, or retains it hidden with `keepMounted`, when closed. */
export function CollapsiblePanel(props: JSX.NativeProps): NativeNode {
  const context = requireCollapsible("Collapsible.Panel");
  return createPartNode(
    "view",
    props,
    universal.mergeProps(collapsiblePartProps(context), {
      part: NativePart.CollapsiblePanel,
    }),
  );
}

/** Base-UI-shaped compound parts for a controlled disclosure. */
export const Collapsible = Object.assign(CollapsibleRoot, {
  Root: CollapsibleRoot,
  Trigger: CollapsibleTrigger,
  Panel: CollapsiblePanel,
});

interface AccordionContextValue {
  scope: string;
  isOpen: (value: string) => boolean;
  toggle: (value: string, event: QuickGuiEvent) => void;
  disabled: () => boolean;
  keepMounted: () => boolean;
  headingLevel: () => number;
}

interface AccordionItemContextValue {
  value: () => string;
  index: () => number;
  open: () => boolean;
  disabled: () => boolean;
}

const AccordionContext = createContext<AccordionContextValue | null>(null);
const AccordionItemContext = createContext<AccordionItemContextValue | null>(null);

function requireAccordion(component: string): AccordionContextValue {
  const context = useContext(AccordionContext);
  if (!context) {
    throw new TypeError(`${component} must be used inside <Accordion.Root>`);
  }
  return context;
}

function requireAccordionItem(component: string): AccordionItemContextValue {
  const context = useContext(AccordionItemContext);
  if (!context) {
    throw new TypeError(`${component} must be used inside <Accordion.Item>`);
  }
  return context;
}

function accordionItemPartProps(
  accordion: AccordionContextValue,
  item: AccordionItemContextValue,
): object {
  return {
    get scope() {
      return accordion.scope;
    },
    get partValue() {
      return item.value();
    },
    get itemIndex() {
      return item.index();
    },
    get open() {
      return item.open();
    },
    get disabled() {
      return item.disabled();
    },
    get keepMounted() {
      return accordion.keepMounted();
    },
    get headingLevel() {
      return accordion.headingLevel();
    },
  };
}

function accordionOpenValues(value: string | readonly string[] | null | undefined): string[] {
  if (value === null || value === undefined) return [];
  return typeof value === "string" ? [value] : [...value];
}

/** Controlled, unstyled accordion root supporting single or multiple open items. */
export function AccordionRoot(props: JSX.AccordionRootProps): NativeNode {
  const scope = createComponentScope("qg-accordion");
  const [uncontrolled, setUncontrolled] = createSignal<string[]>(
    accordionOpenValues(props.defaultValue),
  );
  const open = () =>
    props.value === undefined ? uncontrolled() : accordionOpenValues(props.value);
  const context: AccordionContextValue = {
    scope,
    isOpen: (value) => open().includes(value),
    disabled: () => props.disabled === true,
    keepMounted: () => props.keepMounted === true,
    headingLevel: () => props.headingLevel ?? 3,
    toggle(value, event) {
      const current = open();
      const next = current.includes(value)
        ? current.filter((candidate) => candidate !== value)
        : props.multiple
          ? [...current, value]
          : [value];
      if (props.value === undefined) setUncontrolled(next);
      props.onValueChange?.(props.multiple ? next : (next[0] ?? null), event);
    },
  };
  return createPartNode(
    "view",
    omit(
      props,
      "value",
      "defaultValue",
      "onValueChange",
      "multiple",
      "keepMounted",
      "headingLevel",
      "children",
    ),
    {
      part: NativePart.Accordion,
      scope,
      get children() {
        return AccordionContext({
          value: context,
          get children() {
            return props.children as SolidElement;
          },
        });
      },
    },
  );
}

/** One accordion item. Its trigger, header, and panel identities derive from this value. */
export function AccordionItem(props: JSX.AccordionItemProps): NativeNode {
  const accordion = requireAccordion("Accordion.Item");
  const item: AccordionItemContextValue = {
    value: () => props.value,
    index: () => props.index ?? 0,
    open: () => accordion.isOpen(props.value),
    disabled: () => props.disabled === true || accordion.disabled(),
  };
  return createPartNode(
    "view",
    omit(props, "value", "index", "children"),
    universal.mergeProps(accordionItemPartProps(accordion, item), {
      part: NativePart.AccordionItem,
      get children() {
        return AccordionItemContext({
          value: item,
          get children() {
            return props.children as SolidElement;
          },
        });
      },
    }),
  );
}

/** Accordion heading that contains only this item's trigger. */
export function AccordionHeader(props: JSX.NativeProps): NativeNode {
  const accordion = requireAccordion("Accordion.Header");
  const item = requireAccordionItem("Accordion.Header");
  return createPartNode(
    "view",
    props,
    universal.mergeProps(accordionItemPartProps(accordion, item), {
      part: NativePart.AccordionHeader,
    }),
  );
}

/** Accordion disclosure button for one item. */
export function AccordionTrigger(props: JSX.NativeProps): NativeNode {
  const accordion = requireAccordion("Accordion.Trigger");
  const item = requireAccordionItem("Accordion.Trigger");
  return createPartNode(
    "button",
    props,
    universal.mergeProps(accordionItemPartProps(accordion, item), {
      part: NativePart.AccordionTrigger,
      onClick: forwardClick(props.onClick, (event) =>
        accordion.toggle(item.value(), event),
      ),
    }),
  );
}

/** Accordion panel mounted as a named region by the core while its item is open. */
export function AccordionPanel(props: JSX.NativeProps): NativeNode {
  const accordion = requireAccordion("Accordion.Panel");
  const item = requireAccordionItem("Accordion.Panel");
  return createPartNode(
    "view",
    props,
    universal.mergeProps(accordionItemPartProps(accordion, item), {
      part: NativePart.AccordionPanel,
    }),
  );
}

/** Base-UI-shaped compound parts for a controlled accordion. */
export const Accordion = Object.assign(AccordionRoot, {
  Root: AccordionRoot,
  Item: AccordionItem,
  Header: AccordionHeader,
  Trigger: AccordionTrigger,
  Panel: AccordionPanel,
});

interface FieldContextValue {
  scope: string;
  disabled: () => boolean;
  invalid: () => boolean;
  required: () => boolean;
  touched: () => boolean;
  dirty: () => boolean;
  filled: () => boolean;
  validationMessage: () => string | undefined;
}

interface FieldsetContextValue {
  disabled: () => boolean;
}

const FieldContext = createContext<FieldContextValue | null>(null);
const FieldsetContext = createContext<FieldsetContextValue | null>(null);

function requireField(component: string): FieldContextValue {
  const context = useContext(FieldContext);
  if (!context) {
    throw new TypeError(`${component} must be used inside <Field.Root>`);
  }
  return context;
}

function fieldPartProps(context: FieldContextValue): object {
  return {
    get scope() {
      return context.scope;
    },
    get disabled() {
      return context.disabled();
    },
    get invalid() {
      return context.invalid();
    },
    get required() {
      return context.required();
    },
    get touched() {
      return context.touched();
    },
    get dirty() {
      return context.dirty();
    },
    get filled() {
      return context.filled();
    },
    get validationMessage() {
      return context.validationMessage();
    },
  };
}

/** Controlled, unstyled labelling and validation composition for one form control. */
export function FieldRoot(props: JSX.FieldRootProps): NativeNode {
  const scope = createComponentScope("qg-field");
  const fieldset = useContext(FieldsetContext);
  const context: FieldContextValue = {
    scope,
    disabled: () => props.disabled === true || fieldset?.disabled() === true,
    invalid: () => props.invalid === true,
    required: () => props.required === true,
    touched: () => props.touched === true,
    dirty: () => props.dirty === true,
    filled: () => props.filled === true,
    validationMessage: () => props.validationMessage,
  };
  return createPartNode(
    "view",
    omit(props, "children"),
    universal.mergeProps(fieldPartProps(context), {
      part: NativePart.Field,
      get children() {
        return FieldContext({
          value: context,
          get children() {
            return props.children as SolidElement;
          },
        });
      },
    }),
  );
}

/** Visible label. The core forwards its clicks to the control unless `passive` is declared. */
export function FieldLabel(props: JSX.FieldLabelProps): NativeNode {
  const context = requireField("Field.Label");
  return createPartNode(
    "view",
    omit(props, "passive"),
    universal.mergeProps(fieldPartProps(context), {
      get part() {
        return props.passive
          ? NativePart.FieldPassiveLabel
          : NativePart.FieldLabel;
      },
    }),
  );
}

/**
 * The labelled control itself.
 *
 * The core part sets this element's identity, so the control must be the part rather than a
 * wrapper around one. `element` selects which native element the control renders.
 */
export function FieldControl(props: JSX.FieldControlProps): NativeNode {
  const context = requireField("Field.Control");
  return createPartNode(
    props.element ?? "input",
    omit(props, "element"),
    universal.mergeProps(fieldPartProps(context), {
      part: NativePart.FieldControl,
    }),
  );
}

/** Supplementary help described to assistive technology by the core. */
export function FieldDescription(props: JSX.NativeProps): NativeNode {
  const context = requireField("Field.Description");
  return createPartNode(
    "view",
    props,
    universal.mergeProps(fieldPartProps(context), {
      part: NativePart.FieldDescription,
    }),
  );
}

/** Visible error. The core removes it from layout while the controlled field is valid. */
export function FieldError(props: JSX.NativeProps): NativeNode {
  const context = requireField("Field.Error");
  return createPartNode(
    "view",
    props,
    universal.mergeProps(fieldPartProps(context), {
      part: NativePart.FieldError,
    }),
  );
}

/** Base-UI-shaped compound parts for one labelled, validated control. */
export const Field = Object.assign(FieldRoot, {
  Root: FieldRoot,
  Label: FieldLabel,
  Control: FieldControl,
  Description: FieldDescription,
  Error: FieldError,
});

/** Controlled group semantics for related fields. */
export function FieldsetRoot(props: JSX.FieldsetRootProps): NativeNode {
  const scope = createComponentScope("qg-fieldset");
  const context: FieldsetContextValue = {
    disabled: () => props.disabled === true,
  };
  return createPartNode("view", omit(props, "children"), {
    part: NativePart.Fieldset,
    scope,
    get disabled() {
      return context.disabled();
    },
    get children() {
      return FieldsetContext({
        value: context,
        get children() {
          return props.children as SolidElement;
        },
      });
    },
  });
}

/** Group legend named to assistive technology by the core. */
export function FieldsetLegend(props: JSX.NativeProps): NativeNode {
  return createPartNode("view", props, { part: NativePart.FieldsetLegend });
}

/** Group description named to assistive technology by the core. */
export function FieldsetDescription(props: JSX.NativeProps): NativeNode {
  return createPartNode("view", props, {
    part: NativePart.FieldsetDescription,
  });
}

/** Direct group control that inherits the fieldset's disabled state. */
export function FieldsetControl(props: JSX.FieldControlProps): NativeNode {
  const context = useContext(FieldsetContext);
  return createPartNode(props.element ?? "input", omit(props, "element"), {
    part: NativePart.FieldsetControl,
    get disabled() {
      return props.disabled === true || context?.disabled() === true;
    },
  });
}

export type DialogOpenChangeReason = "trigger-press" | "close-press" | "dismiss";

export interface DialogOpenChangeDetails {
  reason: DialogOpenChangeReason;
  event: QuickGuiEvent;
}

interface DialogContextValue {
  scope: string;
  variant: "dialog" | "alertdialog";
  open: () => boolean;
  dismissOnEscape: () => boolean;
  dismissOnBackdrop: () => boolean;
  change: (
    open: boolean,
    reason: DialogOpenChangeReason,
    event: QuickGuiEvent,
  ) => void;
}

const DialogContext = createContext<DialogContextValue | null>(null);

function requireDialog(component: string): DialogContextValue {
  const context = useContext(DialogContext);
  if (!context) {
    throw new TypeError(
      `${component} must be used inside <Dialog.Root> or <AlertDialog.Root>`,
    );
  }
  return context;
}

function dialogPartProps(context: DialogContextValue): object {
  return {
    get scope() {
      return context.scope;
    },
    get variant() {
      return context.variant;
    },
    get open() {
      return context.open();
    },
  };
}

function createDialogRoot(
  variant: "dialog" | "alertdialog",
  props: JSX.DialogRootProps,
): NativeNode {
  const scope = createComponentScope(
    variant === "alertdialog" ? "qg-alert-dialog" : "qg-dialog",
  );
  const [uncontrolled, setUncontrolled] = createSignal(
    props.defaultOpen ?? false,
  );
  const open = () => props.open ?? uncontrolled();
  const context: DialogContextValue = {
    scope,
    variant,
    open,
    dismissOnEscape: () => props.dismissOnEscape ?? true,
    dismissOnBackdrop: () =>
      props.dismissOnBackdrop ?? variant !== "alertdialog",
    change(next, reason, event) {
      if (props.open === undefined) setUncontrolled(next);
      props.onOpenChange?.(next, { reason, event });
    },
  };
  return DialogContext({
    value: context,
    get children() {
      return props.children as SolidElement;
    },
  }) as unknown as NativeNode;
}

/** Logical root for a controlled in-window modal dialog. It creates no native element. */
export function DialogRoot(props: JSX.DialogRootProps): NativeNode {
  return createDialogRoot("dialog", props);
}

/** Logical root for a consequential alert dialog whose backdrop does not dismiss by default. */
export function AlertDialogRoot(props: JSX.DialogRootProps): NativeNode {
  return createDialogRoot("alertdialog", props);
}

/** Trigger button carrying the core's dialog popover and expanded accessibility state. */
export function DialogTrigger(props: JSX.NativeProps): NativeNode {
  const context = requireDialog("Dialog.Trigger");
  return createPartNode(
    "button",
    props,
    universal.mergeProps(dialogPartProps(context), {
      part: NativePart.DialogTrigger,
      onClick: forwardClick(props.onClick, (event) =>
        context.change(true, "trigger-press", event),
      ),
    }),
  );
}

/**
 * Viewport portal, focus trap, and focus-restoration boundary for the dialog.
 *
 * The Rust core mounts it only while the dialog is open, so a closed dialog contributes no
 * overlay, layout, paint, input, or accessibility node.
 */
export function DialogPortal(props: JSX.NativeProps): NativeNode {
  const context = requireDialog("Dialog.Portal");
  return createPartNode(
    "view",
    props,
    universal.mergeProps(dialogPartProps(context), { part: NativePart.Dialog }),
  );
}

/** Application-owned backdrop filling the portal. */
export function DialogBackdrop(props: JSX.NativeProps): NativeNode {
  const context = requireDialog("Dialog.Backdrop");
  return createPartNode(
    "view",
    props,
    universal.mergeProps(dialogPartProps(context), {
      part: NativePart.DialogBackdrop,
    }),
  );
}

/** Modal surface. Escape and backdrop dismissal are decided ahead of time by the core. */
export function DialogPopup(props: JSX.NativeProps): NativeNode {
  const context = requireDialog("Dialog.Popup");
  return createPartNode(
    "view",
    props,
    universal.mergeProps(dialogPartProps(context), {
      part: NativePart.DialogPopup,
      get dismissOnEscape() {
        return context.dismissOnEscape();
      },
      get dismissOnPointerOutside() {
        return context.dismissOnBackdrop();
      },
      onDismiss(event: QuickGuiEvent) {
        (props as JSX.NativeProps).onDismiss?.(event);
        if (!event.defaultPrevented) context.change(false, "dismiss", event);
      },
    }),
  );
}

/** Visible dialog title used as the popup's accessible name. */
export function DialogTitle(props: JSX.NativeProps): NativeNode {
  const context = requireDialog("Dialog.Title");
  return createPartNode(
    "view",
    props,
    universal.mergeProps(dialogPartProps(context), {
      part: NativePart.DialogTitle,
    }),
  );
}

/** Visible dialog description used as the popup's accessible description. */
export function DialogDescription(props: JSX.NativeProps): NativeNode {
  const context = requireDialog("Dialog.Description");
  return createPartNode(
    "view",
    props,
    universal.mergeProps(dialogPartProps(context), {
      part: NativePart.DialogDescription,
    }),
  );
}

/** Close control. Its accessible name comes from `aria-label`, defaulting to `Close`. */
export function DialogClose(props: JSX.NativeProps): NativeNode {
  const context = requireDialog("Dialog.Close");
  return createPartNode(
    "button",
    props,
    universal.mergeProps(dialogPartProps(context), {
      part: NativePart.DialogClose,
      onClick: forwardClick(props.onClick, (event) =>
        context.change(false, "close-press", event),
      ),
    }),
  );
}

/** Base-UI-shaped compound parts for a controlled in-window modal dialog. */
export const Dialog = Object.assign(DialogRoot, {
  Root: DialogRoot,
  Trigger: DialogTrigger,
  Portal: DialogPortal,
  Backdrop: DialogBackdrop,
  Popup: DialogPopup,
  Title: DialogTitle,
  Description: DialogDescription,
  Close: DialogClose,
});

/** Compound parts for a consequential alert dialog. */
export const AlertDialog = Object.assign(AlertDialogRoot, {
  Root: AlertDialogRoot,
  Trigger: DialogTrigger,
  Portal: DialogPortal,
  Backdrop: DialogBackdrop,
  Popup: DialogPopup,
  Title: DialogTitle,
  Description: DialogDescription,
  Close: DialogClose,
});

/** Base-UI-shaped compound parts for a semantic field group. */
export const Fieldset = Object.assign(FieldsetRoot, {
  Root: FieldsetRoot,
  Legend: FieldsetLegend,
  Description: FieldsetDescription,
  Control: FieldsetControl,
});


/** Retained image node. A path decodes on the core's bounded worker pool. */
export function Image(props: JSX.ImageProps): NativeNode {
  return createHostNode("image", props);
}

/** Retained application shader surface painted by validated WGSL. */
export function Shader(props: JSX.ShaderProps): NativeNode {
  return createHostNode("shader", props);
}

// ---------------------------------------------------------------------------
// Range and feedback parts
// ---------------------------------------------------------------------------

/** Determinate or indeterminate progress root carrying the core's exact value range. */
export function ProgressRoot(props: JSX.ProgressProps): NativeNode {
  return createPartNode("view", props, { part: NativePart.Progress });
}

/** Application-owned progress fill, hidden from the accessible name by the core. */
export function ProgressIndicator(props: JSX.NativeProps): NativeNode {
  return createPartNode("view", props, { part: NativePart.ProgressIndicator });
}

/** Base-UI-shaped compound parts for a progress indicator. */
export const Progress = Object.assign(ProgressRoot, {
  Root: ProgressRoot,
  Indicator: ProgressIndicator,
});

/** Static measurement gauge with optional low, high, and optimum markers. */
export function MeterRoot(props: JSX.MeterProps): NativeNode {
  return createPartNode("view", props, { part: NativePart.Meter });
}

/** Application-owned meter fill, hidden from the accessible name by the core. */
export function MeterIndicator(props: JSX.NativeProps): NativeNode {
  return createPartNode("view", props, { part: NativePart.MeterIndicator });
}

/** Base-UI-shaped compound parts for a meter. */
export const Meter = Object.assign(MeterRoot, {
  Root: MeterRoot,
  Indicator: MeterIndicator,
});

/** Controlled toggle button. A toggle is a button that stays pressed, not a checkbox. */
export function ToggleRoot(props: JSX.ToggleProps): NativeNode {
  const [uncontrolled, setUncontrolled] = createSignal(
    props.defaultPressed ?? false,
  );
  const pressed = () => props.pressed ?? uncontrolled();
  return createPartNode(
    "button",
    omit(props, "pressed", "defaultPressed", "onPressedChange"),
    {
      part: NativePart.Toggle,
      get pressed() {
        return pressed();
      },
      onClick: forwardClick(props.onClick, (event) => {
        const next = !pressed();
        if (props.pressed === undefined) setUncontrolled(next);
        props.onPressedChange?.(next, event);
      }),
    },
  );
}

/** Application-owned toggle indicator, hidden from the accessible name by the core. */
export function ToggleIndicator(props: JSX.NativeProps): NativeNode {
  return createPartNode("view", props, { part: NativePart.ToggleIndicator });
}

/** Base-UI-shaped compound parts for a toggle button. */
export const Toggle = Object.assign(ToggleRoot, {
  Root: ToggleRoot,
  Indicator: ToggleIndicator,
});



// ---------------------------------------------------------------------------
// Declared range, ordering, and roving-focus components
//
// Every value below is declared ahead of the core's decision. The Rust core owns clamping, step
// snapping, thumb ordering, splitter size conservation, wrapping arrow navigation, disabled-item
// skipping, and the single roving Tab stop; JavaScript declares the state and receives whatever
// the core decided as one asynchronous `componentchange` payload.
// ---------------------------------------------------------------------------

/** The payload of a native `componentchange` event. */
export interface ComponentChangeDetails {
  /** Slider thumb values, in ascending thumb order. */
  values?: readonly number[];
  /** Splitter pane sizes in logical pixels, conserved across the whole splitter. */
  sizes?: readonly number[];
  /** The toolbar item that now owns the single roving Tab stop. */
  active?: string | null;
  /** The pressed toggle-group values, in declared item order. */
  pressed?: readonly string[];
}

/** Decode the payload of a native `componentchange` event. */
export function componentChangeFromEvent(
  event: QuickGuiEvent,
): ComponentChangeDetails | undefined {
  if (!event.value) return undefined;
  try {
    const parsed = JSON.parse(event.value) as ComponentChangeDetails;
    return typeof parsed === "object" && parsed !== null ? parsed : undefined;
  } catch {
    return undefined;
  }
}

function componentChangeListener<T>(
  read: (details: ComponentChangeDetails) => T | undefined,
  apply: (next: T, event: QuickGuiEvent) => void,
): (event: QuickGuiEvent) => void {
  return (event) => {
    const details = componentChangeFromEvent(event);
    if (!details) return;
    const next = read(details);
    if (next === undefined) return;
    apply(next, event);
  };
}

/**
 * Controlled slider root.
 *
 * `values` carries one entry per thumb, so a single-thumb slider and a range slider are the same
 * component. The core answers arrows, Page keys, Home, End, and captured pointer drags; it reports
 * the snapped, ordered, clamped result through `onValueChange`.
 */
export function SliderRoot(props: JSX.SliderProps): NativeNode {
  const [uncontrolled, setUncontrolled] = createSignal<readonly number[]>(
    props.defaultValue ?? [0],
  );
  const values = () => props.value ?? uncontrolled();
  return createPartNode(
    "view",
    omit(props, "value", "defaultValue", "onValueChange"),
    {
      part: NativePart.Slider,
      get values() {
        return values().slice();
      },
      onComponentChange: componentChangeListener(
        (details) => details.values,
        (next, event) => {
          if (props.value === undefined) setUncontrolled(next);
          props.onValueChange?.(next, event);
        },
      ),
    },
  );
}

/** Application-owned slider track. The core attaches this slider's captured pointer arithmetic. */
export function SliderTrack(props: JSX.NativeScopedProps): NativeNode {
  return createPartNode("view", props, { part: NativePart.SliderTrack });
}

/** Application-owned slider fill, hidden from the accessible name by the core. */
export function SliderRange(props: JSX.NativeScopedProps): NativeNode {
  return createPartNode("view", props, { part: NativePart.SliderRange });
}

/** Application-owned slider thumb. A range slider gives each thumb its own keyboard focus. */
export function SliderThumb(props: JSX.SliderThumbProps): NativeNode {
  return createPartNode("view", props, { part: NativePart.SliderThumb });
}

/** Base-UI-shaped compound parts for a slider. */
export const Slider = Object.assign(SliderRoot, {
  Root: SliderRoot,
  Track: SliderTrack,
  Range: SliderRange,
  Thumb: SliderThumb,
});

/**
 * Controlled splitter root.
 *
 * `value` carries one size per pane. The core conserves the total across captured drags and typed
 * keyboard resizing and reports every pane size together through `onSizesChange`.
 */
export function SplitterRoot(props: JSX.SplitterProps): NativeNode {
  const [uncontrolled, setUncontrolled] = createSignal<readonly number[]>(
    props.defaultValue ?? [],
  );
  const sizes = () => props.value ?? uncontrolled();
  return createPartNode(
    "view",
    omit(props, "value", "defaultValue", "onSizesChange", "panes"),
    {
      part: NativePart.Splitter,
      get values() {
        return sizes().slice();
      },
      get items() {
        return props.panes ? props.panes.slice() : undefined;
      },
      onComponentChange: componentChangeListener(
        (details) => details.sizes,
        (next, event) => {
          if (props.value === undefined) setUncontrolled(next);
          props.onSizesChange?.(next, event);
        },
      ),
    },
  );
}

/** Application-owned splitter pane. */
export function SplitterPane(props: JSX.SplitterPaneProps): NativeNode {
  return createPartNode("view", props, { part: NativePart.SplitterPane });
}

/** Application-owned splitter handle carrying the core's numeric resize semantics. */
export function SplitterHandle(props: JSX.SplitterPaneProps): NativeNode {
  return createPartNode("view", props, { part: NativePart.SplitterHandle });
}

/** Base-UI-shaped compound parts for an adjustable splitter. */
export const Splitter = Object.assign(SplitterRoot, {
  Root: SplitterRoot,
  Pane: SplitterPane,
  Handle: SplitterHandle,
});

/**
 * Toolbar root with a single roving Tab stop.
 *
 * `items` declares the ordered navigation model. The core answers arrows, Home, and End on the
 * focused item, skips disabled items, and reports the moved Tab stop through `onActiveChange`.
 */
export function ToolbarRoot(props: JSX.ToolbarProps): NativeNode {
  const [uncontrolled, setUncontrolled] = createSignal<string | undefined>(
    props.defaultActive,
  );
  const active = () => props.active ?? uncontrolled();
  return createPartNode(
    "view",
    omit(props, "active", "defaultActive", "onActiveChange"),
    {
      part: NativePart.Toolbar,
      get activeValue() {
        return active();
      },
      onComponentChange: componentChangeListener(
        (details) => details.active ?? undefined,
        (next, event) => {
          if (props.active === undefined) setUncontrolled(next);
          props.onActiveChange?.(next, event);
        },
      ),
    },
  );
}

/** Application-owned toolbar item. Exactly one enabled item stays in the Tab sequence. */
export function ToolbarItem(props: JSX.ComponentItemProps): NativeNode {
  return createPartNode("button", props, { part: NativePart.ToolbarItem });
}

/** Base-UI-shaped compound parts for a toolbar. */
export const Toolbar = Object.assign(ToolbarRoot, {
  Root: ToolbarRoot,
  Item: ToolbarItem,
});

/**
 * Toggle-group root with single or multiple selection.
 *
 * `items` declares the ordered navigation model and `value` the pressed values. The core owns the
 * selection policy, the roving Tab stop, and disabled-item skipping.
 */
export function ToggleGroupRoot(props: JSX.ToggleGroupProps): NativeNode {
  const [uncontrolled, setUncontrolled] = createSignal<readonly string[]>(
    props.defaultValue ?? [],
  );
  const pressed = () => props.value ?? uncontrolled();
  return createPartNode(
    "view",
    omit(props, "value", "defaultValue", "onValueChange"),
    {
      part: NativePart.ToggleGroup,
      get values() {
        return pressed().slice();
      },
      onComponentChange: componentChangeListener(
        (details) => details.pressed,
        (next, event) => {
          if (props.value === undefined) setUncontrolled(next);
          props.onValueChange?.(next, event);
        },
      ),
    },
  );
}

/** Application-owned toggle-group item carrying pressed-button semantics from the core. */
export function ToggleGroupItem(props: JSX.ComponentItemProps): NativeNode {
  return createPartNode("button", props, { part: NativePart.ToggleGroupItem });
}

/** Base-UI-shaped compound parts for a toggle group. */
export const ToggleGroup = Object.assign(ToggleGroupRoot, {
  Root: ToggleGroupRoot,
  Item: ToggleGroupItem,
});


// ---------------------------------------------------------------------------
// Declared input
//
// Every listener below is declared ahead of the core's decision. The Rust core owns targeting,
// capture, bubbling, multi-click counting, accelerator parsing, and drag promotion; JavaScript
// receives the outcome as a bounded asynchronous payload.
// ---------------------------------------------------------------------------

const inputTextEncoder = new TextEncoder();

/** Modifier flags shared by every declared input payload. */
export interface InputModifiers {
  shift: boolean;
  control: boolean;
  alt: boolean;
  meta: boolean;
}

export interface KeyEventDetails extends InputModifiers {
  /** DOM-shaped normalized key name, such as `"a"`, `"ArrowUp"`, or `"Escape"`. */
  key: string;
  /** Composed text the platform reported for this press, when it produced any. */
  text?: string;
  repeat?: boolean;
}

export interface MouseEventDetails extends InputModifiers {
  x: number;
  y: number;
  button?: string;
  pressedButton?: string | null;
  /** Exact native multi-click count. The first press is `1`. */
  clickCount?: number;
  firstMouse?: boolean;
}

export interface WheelEventDetails extends InputModifiers {
  x: number;
  y: number;
  deltaX: number;
  deltaY: number;
  /** `true` for a trackpad or precise wheel. */
  precise: boolean;
  phase: "started" | "moved" | "ended" | "cancelled";
}

export interface GestureEventDetails extends InputModifiers {
  x: number;
  y: number;
  delta?: number;
  phase?: "started" | "moved" | "ended" | "cancelled";
  pressure?: number;
  stage?: string;
}

export interface DropEventDetails extends InputModifiers {
  x: number;
  y: number;
  /** Declared identifier of an application-local payload. */
  id?: string;
  /** Node id that started the drag. */
  source?: number;
  /** Absolute paths of a native file drop. */
  paths?: string[];
  origin?: "internal" | "cross-window" | "external";
}

/** Accelerator-to-binding-id pairs the Rust core resolves for a focused element. */
export type Keymap = Readonly<Record<string, string>>;

export interface DragSource {
  /** Stable identifier delivered to an application-local drop target. */
  id?: string;
  /** Plain text promoted to other applications. */
  text?: string;
  /** Absolute URL promoted to other applications. */
  url?: string;
  /** Existing files or directories promoted to other applications. */
  files?: readonly { path: string; directory?: boolean }[];
}

export type DropKind = "local" | "files";

function bounded(value: string, limit: number, what: string): string {
  if (inputTextEncoder.encode(value).length > limit) {
    throw new RangeError(`QuickGUI ${what} are bounded to ${limit} bytes`);
  }
  return value;
}

function encodeKeymap(value: unknown): string | null {
  if (value === null || value === undefined || value === false) return null;
  if (!isRecord(value)) {
    throw new TypeError("QuickGUI keymap must map accelerators to binding ids");
  }
  for (const entry of Object.values(value)) {
    if (typeof entry !== "string") {
      throw new TypeError("QuickGUI keymap binding ids must be strings");
    }
  }
  return bounded(JSON.stringify(value), MAX_KEYMAP_JSON_BYTES, "keymaps");
}

function encodeDragSource(value: unknown): string | null {
  if (value === null || value === undefined || value === false) return null;
  const declaration = value === true ? {} : value;
  if (!isRecord(declaration)) {
    throw new TypeError("QuickGUI draggable must declare its payload");
  }
  return bounded(
    JSON.stringify(declaration),
    MAX_DRAG_JSON_BYTES,
    "drag declarations",
  );
}

function encodeDropKinds(value: unknown): string | null {
  if (value === null || value === undefined || value === false) return null;
  const kinds = Array.isArray(value) ? value : [value];
  for (const kind of kinds) {
    if (kind !== "local" && kind !== "files") {
      throw new TypeError(
        `QuickGUI accepts the drop kinds \`local\` and \`files\`, not \`${String(kind)}\``,
      );
    }
  }
  return JSON.stringify(kinds);
}

function parseInputPayload<T>(event: QuickGuiEvent): T | undefined {
  if (!event.value) return undefined;
  try {
    return JSON.parse(event.value) as T;
  } catch {
    return undefined;
  }
}

/** Decode a `keydown` or `keyup` payload. */
export function keyEventFromEvent(
  event: QuickGuiEvent,
): KeyEventDetails | undefined {
  return parseInputPayload<KeyEventDetails>(event);
}

/** Decode a `mousedown`, `mouseup`, `mousemove`, `dblclick`, or `contextmenu` payload. */
export function mouseEventFromEvent(
  event: QuickGuiEvent,
): MouseEventDetails | undefined {
  return parseInputPayload<MouseEventDetails>(event);
}

/** Decode a `wheel` payload. */
export function wheelEventFromEvent(
  event: QuickGuiEvent,
): WheelEventDetails | undefined {
  return parseInputPayload<WheelEventDetails>(event);
}

/** Decode a `pinch`, `rotate`, `smartmagnify`, or `pressure` payload. */
export function gestureEventFromEvent(
  event: QuickGuiEvent,
): GestureEventDetails | undefined {
  return parseInputPayload<GestureEventDetails>(event);
}

/** Decode a `drop` or `filesdropped` payload. */
export function dropEventFromEvent(
  event: QuickGuiEvent,
): DropEventDetails | undefined {
  return parseInputPayload<DropEventDetails>(event);
}

/** The binding id an `action` event carries, or `undefined` when the payload is missing. */
export function actionFromEvent(event: QuickGuiEvent): string | undefined {
  return event.value || undefined;
}

// ---------------------------------------------------------------------------
// Declared menus
//
// A menu is declared ahead of time as one bounded JSON model. The Rust core owns validation,
// highlighting, typeahead, checkbox/radio policy, submenu models, accessibility semantics, and the
// cursor-point native surface; JavaScript never answers a synchronous question while a menu is
// open and never reimplements menu behavior.
// ---------------------------------------------------------------------------

export type MenuItemKind =
  | "action"
  | "checkbox"
  | "radio"
  | "submenu"
  | "separator"
  | "group";

export interface MenuItem {
  /** Defaults to `submenu` when `items` is present, otherwise `action`. */
  type?: MenuItemKind;
  /** Stable identifier reported by `onSelect`. Required for every interactive entry. */
  id?: string;
  label?: string;
  /** Display-only accelerator hint, such as `"⌘O"`. */
  shortcut?: string;
  /** Radio group key. Defaults to the item's own id. */
  group?: string;
  checked?: boolean;
  disabled?: boolean;
  /** Override the core's default close policy for this entry. */
  closeOnSelect?: boolean;
  /** Alphanumeric navigation label when it differs from the visible one. */
  typeaheadLabel?: string;
  items?: readonly MenuItem[];
}

/** Structural geometry and appearance for a declared menu surface. */
export interface MenuAppearance {
  width?: number;
  itemHeight?: number;
  separatorHeight?: number;
  groupLabelHeight?: number;
  verticalPadding?: number;
  fontSize?: number;
  radius?: number;
  padding?: number;
  background?: ColorValue;
  color?: ColorValue;
  highlightBackground?: ColorValue;
  highlightColor?: ColorValue;
  mutedColor?: ColorValue;
  /** Wrap arrow navigation at the ends of a menu level. Defaults to `true`. */
  loop?: boolean;
}

export interface MenuSelectDetails {
  /** Stable id of the activated entry. */
  id: string;
  /** New checkbox value, or `true` for a radio item. */
  checked?: boolean;
  /** `true` when the entry opens a submenu instead of dispatching a command. */
  submenu?: boolean;
}

const menuAppearanceColors: readonly string[] = [
  "background",
  "color",
  "highlightBackground",
  "highlightColor",
  "mutedColor",
];

const menuTextEncoder = new TextEncoder();

function encodeMenuItems(items: readonly MenuItem[]): unknown[] {
  return items.map((item) => {
    const encoded: Record<string, unknown> = {};
    for (const [name, value] of Object.entries(item)) {
      if (value === undefined || name === "items") continue;
      encoded[name] = value;
    }
    if (item.items && item.items.length > 0) {
      encoded.items = encodeMenuItems(item.items);
    }
    return encoded;
  });
}

/**
 * Encode one bounded menu declaration for the Rust binding.
 *
 * Colors are packed here so the core never parses CSS, and the byte bound is enforced before the
 * declaration can cross N-API.
 */
export function encodeMenu(
  items: readonly MenuItem[] | undefined,
  appearance: MenuAppearance = {},
): string {
  const declaration: Record<string, unknown> = {
    items: encodeMenuItems(items ?? []),
  };
  for (const [name, value] of Object.entries(appearance)) {
    if (value === undefined || value === null) continue;
    if (name === "loop") {
      declaration.loopFocus = value === true;
      continue;
    }
    declaration[name] = menuAppearanceColors.includes(name)
      ? parseColor(value as ColorValue)
      : value;
  }
  const encoded = JSON.stringify(declaration);
  if (menuTextEncoder.encode(encoded).length > MAX_MENU_JSON_BYTES) {
    throw new RangeError(
      `QuickGUI menu declarations are bounded to ${MAX_MENU_JSON_BYTES} bytes`,
    );
  }
  return encoded;
}

/** Decode the payload of a native `menuselect` event. */
export function menuSelectionFromEvent(
  event: QuickGuiEvent,
): MenuSelectDetails | undefined {
  if (!event.value) return undefined;
  try {
    const parsed = JSON.parse(event.value) as MenuSelectDetails;
    return typeof parsed?.id === "string" ? parsed : undefined;
  } catch {
    return undefined;
  }
}

interface MenuContextValue {
  items: () => readonly MenuItem[];
  appearance: () => MenuAppearance;
  select: (details: MenuSelectDetails, event: QuickGuiEvent) => void;
}

interface PopoverMenuContextValue extends MenuContextValue {
  open: () => boolean;
  setOpen: (
    open: boolean,
    reason: PopoverOpenChangeReason,
    event: QuickGuiEvent,
  ) => void;
  trigger: () => NativeNode | undefined;
  registerTrigger: (node: NativeNode) => void;
  popup: () => NativeNode | undefined;
  registerPopup: (node: NativeNode | undefined) => void;
  placement: () => PopoverPlacement;
  gap: () => number;
  viewportMargin: () => number;
  dismissOnEscape: () => boolean;
  dismissOnPointerOutside: () => boolean;
}

const PopoverMenuContext = createContext<PopoverMenuContextValue | null>(null);
const ContextMenuContext = createContext<MenuContextValue | null>(null);

function requirePopoverMenu(component: string): PopoverMenuContextValue {
  const context = useContext(PopoverMenuContext);
  if (!context) {
    throw new TypeError(`${component} must be used inside <PopoverMenu.Root>`);
  }
  return context;
}

function requireContextMenu(component: string): MenuContextValue {
  const context = useContext(ContextMenuContext);
  if (!context) {
    throw new TypeError(`${component} must be used inside <ContextMenu.Root>`);
  }
  return context;
}

function menuSelectListener(
  context: MenuContextValue,
  handler: ((event: QuickGuiEvent) => void) | undefined,
): (event: QuickGuiEvent) => void {
  return (event) => {
    handler?.(event);
    const details = menuSelectionFromEvent(event);
    if (details) context.select(details, event);
  };
}

/** Logical root of a declared popover menu. It creates no native element. */
export function PopoverMenuRoot(props: JSX.PopoverMenuRootProps): NativeNode {
  const [uncontrolledOpen, setUncontrolledOpen] = createSignal(
    props.defaultOpen ?? false,
  );
  const [trigger, setTrigger] = createSignal<NativeNode>();
  const [popup, setPopup] = createSignal<NativeNode>();
  const open = () => props.open ?? uncontrolledOpen();
  const context: PopoverMenuContextValue = {
    open,
    setOpen(nextOpen, reason, event) {
      if (props.open === undefined) setUncontrolledOpen(nextOpen);
      props.onOpenChange?.(nextOpen, { reason, event });
    },
    items: () => props.items ?? [],
    appearance: () => props.appearance ?? {},
    select(details, event) {
      props.onSelect?.(details, event);
      if (details.submenu) return;
      if (props.open === undefined) setUncontrolledOpen(false);
      props.onOpenChange?.(false, { reason: "dismiss", event });
    },
    trigger,
    registerTrigger: (node) => setTrigger(() => node),
    popup,
    registerPopup: (node) => setPopup(() => node),
    placement: () => props.placement ?? "bottom-start",
    gap: () => props.gap ?? 4,
    viewportMargin: () => props.viewportMargin ?? 8,
    dismissOnEscape: () => props.dismissOnEscape ?? true,
    dismissOnPointerOutside: () => props.dismissOnPointerOutside ?? true,
  };
  return PopoverMenuContext({
    value: context,
    get children() {
      return props.children as SolidElement;
    },
  }) as unknown as NativeNode;
}

/** Menu trigger. The core supplies its `has-popup`, expansion, and controls relationship. */
export function PopoverMenuTrigger(props: JSX.NativeProps): NativeNode {
  const context = requirePopoverMenu("PopoverMenu.Trigger");
  let trigger: NativeNode | undefined;
  return createPartNode("button", omit(props, "onClick", "ref"), {
    part: NativePart.PopoverMenuTrigger,
    get open() {
      return context.open();
    },
    get controls() {
      // The relationship exists only while the surface is mounted, so a closed menu declares no
      // dangling target and no signal is written while the surface disposes.
      return context.open() ? context.popup() : undefined;
    },
    ref: (node: NativeNode) => {
      trigger = node;
      context.registerTrigger(node);
      if (typeof props.ref === "function") props.ref(node);
    },
    onClick: forwardClick(props.onClick, (event) => {
      if (trigger) context.registerTrigger(trigger);
      context.setOpen(!context.open(), "trigger-press", event);
    }),
  });
}

/**
 * Menu surface anchored to the trigger.
 *
 * Rows come from the declared model, so the surface owns only its own paint. It is mounted only
 * while the menu is open and while a trigger exists to anchor it.
 */
export function PopoverMenuPopup(props: JSX.PopoverMenuPopupProps): NativeNode {
  const context = requirePopoverMenu("PopoverMenu.Popup");
  return Show({
    keyed: true,
    get when() {
      return context.open() ? context.trigger() : undefined;
    },
    children: (anchor: NativeNode) => {
      const node = createPartNode("view", omit(props, "onDismiss", "onSelect"), {
        part: NativePart.PopoverMenuPopup,
        anchor,
        get menu() {
          return encodeMenu(context.items(), context.appearance());
        },
        get width() {
          return props.width ?? context.appearance().width ?? 224;
        },
        get anchorPlacement() {
          return context.placement();
        },
        get anchorGap() {
          return context.gap();
        },
        get viewportMargin() {
          return context.viewportMargin();
        },
        get dismissOnEscape() {
          return context.dismissOnEscape();
        },
        get dismissOnPointerOutside() {
          return context.dismissOnPointerOutside();
        },
        onSelect: menuSelectListener(context, props.onSelect),
        onDismiss(event: QuickGuiEvent) {
          props.onDismiss?.(event);
          if (!event.defaultPrevented) context.setOpen(false, "dismiss", event);
        },
      });
      context.registerPopup(node);
      return node;
    },
  }) as unknown as NativeNode;
}

/** Base-UI-shaped compound parts for a declared popover menu. */
export const PopoverMenu = Object.assign(PopoverMenuRoot, {
  Root: PopoverMenuRoot,
  Trigger: PopoverMenuTrigger,
  Popup: PopoverMenuPopup,
});

/** Logical root of a declared cursor-point context menu. It creates no native element. */
export function ContextMenuRoot(props: JSX.ContextMenuRootProps): NativeNode {
  const context: MenuContextValue = {
    items: () => props.items ?? [],
    appearance: () => props.appearance ?? {},
    select(details, event) {
      props.onSelect?.(details, event);
    },
  };
  return ContextMenuContext({
    value: context,
    get children() {
      return props.children as SolidElement;
    },
  }) as unknown as NativeNode;
}

/**
 * Secondary-click target for a declared context menu.
 *
 * The core opens its own cursor-point surface, keeps the menu inside the work area, owns submenu
 * hover intent and safe corridors, and tears the chain down child-first.
 */
export function ContextMenuTrigger(
  props: JSX.ContextMenuTriggerProps,
): NativeNode {
  const context = requireContextMenu("ContextMenu.Trigger");
  return createPartNode("view", omit(props, "onSelect"), {
    part: NativePart.ContextMenuTrigger,
    get menu() {
      return encodeMenu(context.items(), context.appearance());
    },
    onSelect: menuSelectListener(context, props.onSelect),
  });
}

/** Base-UI-shaped compound parts for a declared context menu. */
export const ContextMenu = Object.assign(ContextMenuRoot, {
  Root: ContextMenuRoot,
  Trigger: ContextMenuTrigger,
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

/** Easing curves the Rust core exposes. `ease` is an alias for `ease-in-out`. */
export type TransitionEasing =
  | "linear"
  | "ease"
  | "ease-in"
  | "ease-out"
  | "ease-in-out";

/** Transition properties the Rust core can interpolate without a layout pass. */
export type TransitionPropertyName =
  | "all"
  | "background"
  | "background-color"
  | "border-color"
  | "border-width"
  | "border-radius"
  | "color"
  | "box-shadow"
  | "opacity";

export interface TransitionDeclaration {
  property?: TransitionPropertyName | readonly TransitionPropertyName[];
  properties?: TransitionDeclaration["property"];
  duration?: number | string;
  easing?: TransitionEasing;
  timingFunction?: TransitionEasing;
  maxFps?: number;
}

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
    transition?: number | string | TransitionDeclaration;
    opacity?: number;
    borderWidth?: number | string;
    borderTopWidth?: number | string;
    borderRightWidth?: number | string;
    borderBottomWidth?: number | string;
    borderLeftWidth?: number | string;
    borderColor?: number | string;
    borderRadius?: number | string;
    boxShadow?: string;
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
    /** CSS grid track list, an array of tracks, or a count of equal `1fr` tracks. */
    gridTemplateColumns?: number | string | readonly (number | string)[];
    gridTemplateRows?: Style["gridTemplateColumns"];
    gridAutoFlow?: "row" | "column" | "row dense" | "column dense";
    /** CSS `grid-column` shorthand such as `2`, `2 / 4`, or `span 3`. */
    gridColumn?: number | string;
    gridRow?: number | string;
    gridColumnStart?: number;
    gridColumnEnd?: number;
    gridRowStart?: number;
    gridRowEnd?: number;
    transitionProperty?: string;
    transitionDuration?: number | string;
    transitionTimingFunction?: TransitionEasing;
    transitionEasing?: TransitionEasing;
    /** Repaint cadence ceiling while the transition runs. */
    transitionMaxFps?: number;
    objectFit?: "fill" | "contain" | "cover" | "scale-down" | "none";
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
    /** Delayed, pointer-passive native tooltip text shown while this element is hovered. */
    tooltip?: string;
    tooltipPlacement?: PopoverPlacement;
    /** Hover delay in milliseconds, clamped by the Rust core to at most ten seconds. */
    tooltipDelay?: number;
    tooltipGap?: number;
    tooltipViewportMargin?: number;
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
    /** Focused key press. Declare a `tabIndex` to make an ordinary container focusable. */
    onKeyDown?: EventHandler;
    onKeyUp?: EventHandler;
    onMouseDown?: EventHandler;
    onMouseUp?: EventHandler;
    onMouseMove?: EventHandler;
    /** Second press of one exact native multi-click sequence. */
    onDoubleClick?: EventHandler;
    onWheel?: EventHandler;
    /** Secondary-button press. Use `ContextMenu` for a declared native menu. */
    onContextMenu?: EventHandler;
    onPinch?: EventHandler;
    onRotate?: EventHandler;
    onSmartMagnify?: EventHandler;
    onPressure?: EventHandler;
    onFocus?: EventHandler;
    onBlur?: EventHandler;
    /** Bounded accelerator table resolved by the core while this element is focused. */
    keymap?: Keymap;
    /** Typed binding id dispatched by `keymap`. */
    onAction?: EventHandler;
    /** Declared drag payload promoted when this element starts a drag. */
    draggable?: boolean | DragSource;
    onDragStart?: EventHandler;
    onDragEnd?: EventHandler;
    /** Payload kinds this element accepts, declared ahead of the native drag. */
    dropKinds?: DropKind | readonly DropKind[];
    onDrop?: EventHandler;
    onFilesDropped?: EventHandler;
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

  export interface CheckboxProps extends NativeProps {
    /** Controlled `true`, `false`, or `"indeterminate"` toggle state. */
    checked?: CheckedState;
    defaultChecked?: CheckedState;
    onCheckedChange?: (checked: boolean, event: QuickGuiEvent) => void;
  }

  export interface RadioGroupProps extends NativeProps {
    value?: string;
    defaultValue?: string;
    onValueChange?: (value: string, event: QuickGuiEvent) => void;
  }

  export interface RadioProps extends NativeProps {
    /** Value this radio selects in its `RadioGroup`. */
    value?: string;
    /** Controlled selection for a radio used without a `RadioGroup`. */
    checked?: boolean;
    defaultChecked?: boolean;
    onCheckedChange?: (checked: boolean, event: QuickGuiEvent) => void;
  }

  export interface SwitchProps extends NativeProps {
    checked?: boolean;
    defaultChecked?: boolean;
    onCheckedChange?: (checked: boolean, event: QuickGuiEvent) => void;
  }

  export interface TabsRootProps extends NativeProps {
    value?: string;
    defaultValue?: string;
    onValueChange?: (value: string, event: QuickGuiEvent) => void;
    orientation?: "horizontal" | "vertical";
    /** `"manual"` activates on Enter or Space; `"automatic"` activates on arrow focus. */
    activation?: "manual" | "automatic";
    /** Wrap arrow navigation at the ends of the tab list. Defaults to `true`. */
    loop?: boolean;
    /** Retain inactive panels as `display: none` instead of omitting them. */
    keepMounted?: boolean;
  }

  export interface TabsTabProps extends NativeProps {
    value: string;
  }

  export interface TabsIndicatorProps extends NativeProps {
    /** Tab this indicator belongs to. Defaults to the enclosing tab, then the active tab. */
    value?: string;
  }

  export interface TabsPanelProps extends NativeProps {
    value: string;
  }

  export interface CollapsibleRootProps extends NativeProps {
    open?: boolean;
    defaultOpen?: boolean;
    onOpenChange?: (open: boolean, event: QuickGuiEvent) => void;
    keepMounted?: boolean;
  }

  export interface AccordionRootProps extends NativeProps {
    /** Open item value, or values when `multiple` is declared. */
    value?: string | readonly string[] | null;
    defaultValue?: string | readonly string[] | null;
    onValueChange?: (
      value: string | string[] | null,
      event: QuickGuiEvent,
    ) => void;
    multiple?: boolean;
    keepMounted?: boolean;
    /** Heading level for each item header, clamped by the core to 1 through 6. */
    headingLevel?: number;
  }

  export interface AccordionItemProps extends NativeProps {
    value: string;
    /** Caller-visible position projected across this item's parts. */
    index?: number;
  }

  export interface FieldRootProps extends NativeProps {
    invalid?: boolean;
    required?: boolean;
    touched?: boolean;
    dirty?: boolean;
    filled?: boolean;
    /** Bounded message retained for form reports and native accessibility. */
    validationMessage?: string;
  }

  export interface FieldLabelProps extends NativeProps {
    /** Name the control without forwarding pointer activation to it. */
    passive?: boolean;
  }

  export interface FieldControlProps extends InputProps {
    /** Native element this control renders. Defaults to `input`. */
    element?: "input" | "textarea" | "button" | "view" | "text";
  }

  export interface FieldsetRootProps extends NativeProps {}

  export interface ImageProps extends NativeProps {
    /** Filesystem path, `file://` URL, or a base64 `data:` URL. */
    source: string;
    /** `fill`, `contain` (default), `cover`, `scale-down`, or `none`. */
    fit?: "fill" | "contain" | "cover" | "scale-down" | "none";
    objectFit?: NonNullable<ImageProps["fit"]>;
  }

  export interface ShaderProps extends NativeProps {
    /** Complete WGSL bounded and validated by the Rust core. */
    source: string;
    /** Up to sixteen floats packed into the core's four fixed parameter vectors. */
    shaderParameters?: readonly number[] | readonly (readonly number[])[];
  }

  export interface ProgressProps extends NativeProps {
    /** Completed amount. Omit, or declare `indeterminate`, for unknown progress. */
    value?: number;
    /** Completion maximum. Defaults to `1`. */
    max?: number;
    indeterminate?: boolean;
    /** Human-readable value such as `"3 of 12 files"`, preferred by assistive technology. */
    valueText?: string;
  }

  export interface MeterProps extends NativeProps {
    value?: number;
    min?: number;
    max?: number;
    low?: number;
    high?: number;
    optimum?: number;
  }


  /** A declared component part that names the instance it belongs to. */
  export interface NativeScopedProps extends NativeProps {
    /** Stable key shared by every part of one component instance. */
    scope?: string;
  }

  /** One entry in a declared toolbar or toggle-group navigation model. */
  export interface ComponentItemDeclaration {
    value: string;
    disabled?: boolean;
  }

  /** One pane constraint in a declared splitter. */
  export interface SplitterPaneDeclaration {
    min?: number;
    collapsible?: boolean;
  }

  export interface SliderProps extends NativeScopedProps {
    /** Controlled thumb values, one entry per thumb. */
    value?: readonly number[];
    defaultValue?: readonly number[];
    min?: number;
    max?: number;
    step?: number;
    largeStep?: number;
    orientation?: "horizontal" | "vertical";
    onValueChange?: (values: readonly number[], event: QuickGuiEvent) => void;
  }

  export interface SliderThumbProps extends NativeScopedProps {
    /** Which thumb this part paints, matching the index in `value`. */
    itemIndex?: number;
  }

  export interface SplitterProps extends NativeScopedProps {
    /** Controlled pane sizes in logical pixels. */
    value?: readonly number[];
    defaultValue?: readonly number[];
    panes?: readonly SplitterPaneDeclaration[];
    step?: number;
    orientation?: "horizontal" | "vertical";
    onSizesChange?: (sizes: readonly number[], event: QuickGuiEvent) => void;
  }

  export interface SplitterPaneProps extends NativeScopedProps {
    /** Which pane or handle this part paints. */
    itemIndex?: number;
  }

  export interface ToolbarProps extends NativeScopedProps {
    items?: readonly ComponentItemDeclaration[];
    active?: string;
    defaultActive?: string;
    orientation?: "horizontal" | "vertical";
    loopFocus?: boolean;
    onActiveChange?: (active: string | undefined, event: QuickGuiEvent) => void;
  }

  export interface ToggleGroupProps extends NativeScopedProps {
    items?: readonly ComponentItemDeclaration[];
    /** Controlled pressed values. */
    value?: readonly string[];
    defaultValue?: readonly string[];
    /** `"single"` presses at most one item; `"multiple"` presses any number. */
    variant?: "single" | "multiple";
    active?: string;
    orientation?: "horizontal" | "vertical";
    loopFocus?: boolean;
    onValueChange?: (values: readonly string[], event: QuickGuiEvent) => void;
  }

  /** One declared toolbar or toggle-group item part. */
  export interface ComponentItemProps extends NativeScopedProps {
    /** The item's stable value, matching an entry in the group's `items`. */
    partValue?: string;
  }

  export interface ToggleProps extends NativeProps {
    pressed?: boolean;
    defaultPressed?: boolean;
    onPressedChange?: (pressed: boolean, event: QuickGuiEvent) => void;
  }

  export interface PopoverMenuRootProps {
    children?: unknown;
    /** Bounded menu model rebuilt by the Rust core on every declaration change. */
    items?: readonly MenuItem[];
    /** Structural geometry and paint for the declared rows. */
    appearance?: MenuAppearance;
    open?: boolean;
    defaultOpen?: boolean;
    onOpenChange?: (open: boolean, details: PopoverOpenChangeDetails) => void;
    onSelect?: (details: MenuSelectDetails, event: QuickGuiEvent) => void;
    placement?: PopoverPlacement;
    gap?: number;
    viewportMargin?: number;
    dismissOnEscape?: boolean;
    dismissOnPointerOutside?: boolean;
  }

  export interface PopoverMenuPopupProps extends NativeProps {
    /** Surface width. Defaults to the declared appearance width. */
    width?: number;
    onSelect?: EventHandler;
  }

  export interface ContextMenuRootProps {
    children?: unknown;
    items?: readonly MenuItem[];
    appearance?: MenuAppearance;
    onSelect?: (details: MenuSelectDetails, event: QuickGuiEvent) => void;
  }

  export interface ContextMenuTriggerProps extends NativeProps {
    onSelect?: EventHandler;
  }

  export interface DialogRootProps {
    children?: unknown;
    open?: boolean;
    defaultOpen?: boolean;
    onOpenChange?: (open: boolean, details: DialogOpenChangeDetails) => void;
    /** Dismiss on Escape. Defaults to `true` for both dialog kinds. */
    dismissOnEscape?: boolean;
    /** Dismiss on a backdrop press. Defaults to `true`, or `false` for an alert dialog. */
    dismissOnBackdrop?: boolean;
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
    image: ImageProps;
    shader: ShaderProps;
  }
}
