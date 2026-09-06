/**
 * The run-time half of the UI package: typed property setters the compiled JSX calls, reactive
 * insertion helpers, regions, and the control-flow components.
 *
 * Every setter validates and bounds a declaration exactly as the host bounds it, then records one
 * protocol mutation. Setters take `undefined` to withdraw a property, so a reactive binding can
 * re-run with any value the props allow.
 */

import {
  NativeNode,
  createNativeElement,
  createNativeSentinel,
  createNativeText,
  insertNativeNode,
  parseColor,
  removeNativeNode,
  replaceNativeText,
  setNativeBoolean,
  setNativeColor,
  setNativeEventListener,
  setNativeNumber,
  setNativeString,
  clearNativeProperty,
  type QuickGuiEvent,
  type Window,
  type WindowRenderer,
} from "@quickgui/native";

import {
  Context,
  Owner,
  Signal,
  batch,
  createRenderEffect,
  createRoot,
  disposeOwner,
  getComputation,
  getOwner,
  runWithOwner,
  untrack,
  type Accessor,
} from "./reactive.ts";
import type {
  DragSource,
  DropKind,
  Extent,
  GradientDeclaration,
  GroupStateStyle,
  Keymap,
  StateStyle,
  StringMap,
  TerminalPalette,
  TextShadowDeclaration,
  TransformMatrix,
  TransitionDeclaration,
} from "./types.ts";

// Property codes the composite setters need beyond their own.
const CODE_FLEX_GROW = 4;
const CODE_FLEX_SHRINK = 5;
const CODE_FLEX_BASIS = 6;
const CODE_TRANSITION = 98;
const CODE_TRANSITION_PROPERTIES = 171;
const CODE_TRANSITION_DURATION = 172;
const CODE_TRANSITION_EASING = 173;
const CODE_TRANSITION_MAX_FPS = 174;
const CODE_GRID_COLUMN_START = 165;
const CODE_GRID_COLUMN_END = 166;
const CODE_GRID_COLUMN_SPAN = 167;
const CODE_GRID_ROW_START = 168;
const CODE_GRID_ROW_END = 169;
const CODE_GRID_ROW_SPAN = 170;
const CODE_OUTLINE_WIDTH = 265;
const CODE_OUTLINE_COLOR = 266;
const CODE_OUTLINE_STYLE = 268;
const CODE_BORDER_RADIUS = 35;
const CODE_PASSWORD = 76;

/** Give inline ref callbacks the native node's type before invoking them. */
export function setRef(node: NativeNode, ref: (node: NativeNode) => void): void {
  ref(node);
}

export const MAX_STYLE_DECLARATION_BYTES = 4096;
export const MAX_STATE_STYLE_JSON_BYTES = 16 * 1024;
export const MAX_COMPONENT_VALUE_BYTES = 256;
export const MAX_TOOLTIP_TEXT_BYTES = 1024;
export const MAX_MENU_LINK_BYTES = 8 * 1024;
export const MAX_KEYMAP_JSON_BYTES = 64 * 1024;
export const MAX_DRAG_JSON_BYTES = 64 * 1024;
export const MAX_HOVER_GROUP_NAME_BYTES = 256;
export const MAX_GROUP_STYLES_PER_ELEMENT = 8;
export const MAX_BOX_SHADOWS_PER_ELEMENT = 8;
export const MAX_SHADER_PARAMETER_FLOATS = 16;

function byteLength(text: string): number {
  return new TextEncoder().encode(text).length;
}

function bounded(text: string, limit: number, what: string): string {
  if (byteLength(text) > limit) throw new RangeError("QuickGUI " + what + " are bounded to " + String(limit) + " bytes");
  return text;
}

function isWhitespace(code: number): boolean {
  return code === 32 || code === 9 || code === 10 || code === 13 || code === 12;
}

/** Split on whitespace outside parentheses. */
export function splitCssTokens(value: string): string[] {
  const tokens: string[] = [];
  let start = 0;
  let depth = 0;
  for (let index = 0; index < value.length; index += 1) {
    const code = value.charCodeAt(index);
    if (code === 40) depth += 1;
    else if (code === 41) {
      depth -= 1;
      if (depth < 0) throw new TypeError("QuickGUI declaration has unbalanced parentheses");
    } else if (isWhitespace(code) && depth === 0) {
      const token = value.slice(start, index).trim();
      if (token.length > 0) tokens.push(token);
      start = index + 1;
    }
  }
  if (depth !== 0) throw new TypeError("QuickGUI declaration has unbalanced parentheses");
  const token = value.slice(start).trim();
  if (token.length > 0) tokens.push(token);
  return tokens;
}

/** Split on commas outside parentheses. */
export function splitCssList(value: string): string[] {
  const values: string[] = [];
  let start = 0;
  let depth = 0;
  for (let index = 0; index < value.length; index += 1) {
    const code = value.charCodeAt(index);
    if (code === 40) depth += 1;
    else if (code === 41) depth = Math.max(0, depth - 1);
    else if (code === 44 && depth === 0) {
      const entry = value.slice(start, index).trim();
      if (entry.length > 0) values.push(entry);
      start = index + 1;
    }
  }
  const last = value.slice(start).trim();
  if (last.length > 0) values.push(last);
  return values;
}

/** `12px`, `12`, or `0` become numbers; anything else stays text (`100%`, `auto`). */
export function normalizeLength(value: string): number | string {
  const trimmed = value.trim();
  if (trimmed.endsWith("px")) {
    const number = Number(trimmed.slice(0, trimmed.length - 2));
    return Number.isFinite(number) ? number : trimmed;
  }
  if (trimmed === "0") return 0;
  const number = Number(trimmed);
  return Number.isFinite(number) && trimmed.length > 0 ? number : trimmed;
}

function lengthNumber(value: string): number | undefined {
  const normalized = normalizeLength(value);
  return typeof normalized === "number" ? normalized : undefined;
}

// ---------------------------------------------------------------------------
// Typed setters
// ---------------------------------------------------------------------------

export function setLength(node: NativeNode, code: number, value: number | string | undefined): void {
  if (value === undefined) {
    clearNativeProperty(node, code);
  } else if (typeof value === "number") {
    setNativeNumber(node, code, value);
  } else {
    const normalized = normalizeLength(value);
    if (typeof normalized === "number") setNativeNumber(node, code, normalized);
    else if (normalized.length === 0) clearNativeProperty(node, code);
    else setNativeString(node, code, normalized);
  }
}

export function setNumber(node: NativeNode, code: number, value: number | undefined): void {
  if (value === undefined || !Number.isFinite(value)) clearNativeProperty(node, code);
  else setNativeNumber(node, code, value);
}

export function setString(node: NativeNode, code: number, value: string | undefined): void {
  if (value === undefined) clearNativeProperty(node, code);
  else setNativeString(node, code, value);
}

/** A boolean whose `false` withdraws the declaration. */
export function setBool(node: NativeNode, code: number, value: boolean | undefined): void {
  if (value === undefined || !value) clearNativeProperty(node, code);
  else setNativeBoolean(node, code, true);
}

/** A boolean the core defaults to `true`, so `false` is transmitted. */
export function setExplicitBool(node: NativeNode, code: number, value: boolean | undefined): void {
  if (value === undefined) clearNativeProperty(node, code);
  else setNativeBoolean(node, code, value);
}

export function setColor(node: NativeNode, code: number, value: number | string | undefined): void {
  if (value === undefined) clearNativeProperty(node, code);
  else setNativeColor(node, code, parseColor(value));
}

function isGradientText(value: string): boolean {
  const text = value.trim();
  return text.includes("linear-gradient(") || text.includes("radial-gradient(") || text.includes("conic-gradient(");
}

/** A solid color or one declared gradient, clearing whichever form is not used. */
export function setBackground(
  node: NativeNode,
  colorCode: number,
  gradientCode: number,
  value: number | string | GradientDeclaration | undefined,
): void {
  if (value === undefined) {
    clearNativeProperty(node, colorCode);
    clearNativeProperty(node, gradientCode);
    return;
  }
  if (typeof value === "number") {
    clearNativeProperty(node, gradientCode);
    setNativeColor(node, colorCode, value >>> 0);
    return;
  }
  if (typeof value === "string") {
    if (isGradientText(value)) {
      clearNativeProperty(node, colorCode);
      setNativeString(node, gradientCode, bounded(value.trim(), MAX_STYLE_DECLARATION_BYTES, "style declarations"));
    } else {
      clearNativeProperty(node, gradientCode);
      setNativeColor(node, colorCode, parseColor(value));
    }
    return;
  }
  const gradient: GradientDeclaration = value;
  clearNativeProperty(node, colorCode);
  setNativeString(node, gradientCode, bounded(JSON.stringify(gradient), MAX_STYLE_DECLARATION_BYTES, "style declarations"));
}

interface EncodedBoxShadow {
  offsetX: number;
  offsetY: number;
  blurRadius: number;
  spreadRadius: number;
  color: number | null;
  inset: boolean;
}

function parseShadowLength(value: string): number | undefined {
  const text = value.endsWith("px") ? value.slice(0, value.length - 2) : value;
  if (text.length === 0) return undefined;
  const number = Number(text);
  return Number.isFinite(number) ? number : undefined;
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
      if (lengths.length === 4) throw new TypeError("QuickGUI boxShadow accepts two to four length values");
      lengths.push(length);
      continue;
    }
    if (hasColor) throw new TypeError("QuickGUI boxShadow accepts one color per shadow");
    hasColor = true;
    if (token.toLowerCase() !== "currentcolor") color = parseColor(token);
  }
  if (lengths.length < 2) throw new TypeError("QuickGUI boxShadow requires horizontal and vertical offsets");
  return {
    offsetX: lengths[0]!,
    offsetY: lengths[1]!,
    blurRadius: lengths.length > 2 ? lengths[2]! : 0,
    spreadRadius: lengths.length > 3 ? lengths[3]! : 0,
    color,
    inset,
  };
}

/** The declared shadow entries; `none` is an empty list. */
export function parseBoxShadowList(value: string): EncodedBoxShadow[] {
  const shorthand = value.trim();
  if (shorthand.length === 0 || shorthand.toLowerCase() === "none") return [];
  const declarations = splitCssList(shorthand);
  if (declarations.length > MAX_BOX_SHADOWS_PER_ELEMENT) {
    throw new TypeError("QuickGUI boxShadow supports at most " + String(MAX_BOX_SHADOWS_PER_ELEMENT) + " shadows");
  }
  const shadows: EncodedBoxShadow[] = [];
  for (const declaration of declarations) shadows.push(parseBoxShadowDeclaration(declaration));
  return shadows;
}

export function setBoxShadow(node: NativeNode, code: number, value: string | undefined): void {
  if (value === undefined) {
    clearNativeProperty(node, code);
    return;
  }
  const shadows = parseBoxShadowList(value);
  if (shadows.length === 0) clearNativeProperty(node, code);
  else setNativeString(node, code, JSON.stringify(shadows));
}

/** A CSS function list such as a filter chain; an array joins with spaces. */
export function setDeclaration(node: NativeNode, code: number, value: string | string[] | undefined): void {
  if (value === undefined) {
    clearNativeProperty(node, code);
    return;
  }
  const text = (typeof value === "string" ? value : value.join(" ")).trim();
  if (text.length === 0) clearNativeProperty(node, code);
  else setNativeString(node, code, bounded(text, MAX_STYLE_DECLARATION_BYTES, "style declarations"));
}

export function setTransform(node: NativeNode, code: number, value: string | string[] | TransformMatrix | undefined): void {
  if (value === undefined) {
    clearNativeProperty(node, code);
    return;
  }
  if (typeof value === "string") {
    setDeclaration(node, code, value);
    return;
  }
  if (Array.isArray(value)) {
    setDeclaration(node, code, value);
    return;
  }
  const matrix: TransformMatrix = value;
  setNativeString(node, code, JSON.stringify(matrix));
}

export function setTextShadow(node: NativeNode, code: number, value: string | TextShadowDeclaration | undefined): void {
  if (value === undefined) {
    clearNativeProperty(node, code);
    return;
  }
  if (typeof value === "string") {
    setDeclaration(node, code, value);
    return;
  }
  const shadow: TextShadowDeclaration = value;
  setNativeString(node, code, JSON.stringify(shadow));
}

function transitionPropertyName(name: string): string {
  switch (name.trim()) {
    case "all":
      return "all";
    case "background":
    case "background-color":
      return "background-color";
    case "border-color":
      return "border-color";
    case "border-width":
      return "border-width";
    case "border-radius":
      return "border-radius";
    case "color":
      return "color";
    case "box-shadow":
      return "box-shadow";
    case "opacity":
      return "opacity";
    default:
      throw new TypeError("QuickGUI cannot transition `" + name + "`");
  }
}

function transitionEasingName(name: string): string {
  const normalized = name.trim();
  switch (normalized) {
    case "linear":
    case "ease-in":
    case "ease-out":
    case "ease-in-out":
      return normalized;
    case "ease":
      return "ease-in-out";
    default:
      throw new TypeError("QuickGUI does not expose the `" + normalized + "` easing curve");
  }
}

/** Duration in milliseconds from a `120ms`, `0.2s`, or plain-number declaration. */
export function normalizeMilliseconds(value: number | string): number | undefined {
  if (typeof value === "number") return Number.isFinite(value) ? value : undefined;
  const text = value.trim();
  if (text.endsWith("ms")) {
    const number = Number(text.slice(0, text.length - 2));
    return Number.isFinite(number) ? number : undefined;
  }
  if (text.endsWith("s")) {
    const number = Number(text.slice(0, text.length - 1));
    return Number.isFinite(number) ? number * 1000 : undefined;
  }
  const number = Number(text);
  return Number.isFinite(number) && text.length > 0 ? number : undefined;
}

function clearTransition(node: NativeNode): void {
  clearNativeProperty(node, CODE_TRANSITION);
  clearNativeProperty(node, CODE_TRANSITION_PROPERTIES);
  clearNativeProperty(node, CODE_TRANSITION_DURATION);
  clearNativeProperty(node, CODE_TRANSITION_EASING);
  clearNativeProperty(node, CODE_TRANSITION_MAX_FPS);
}

/** A complete paint transition from a number, the CSS shorthand, or the declared object form. */
export function setTransition(node: NativeNode, _code: number, value: number | string | TransitionDeclaration | undefined): void {
  clearTransition(node);
  if (value === undefined) return;
  if (typeof value === "number") {
    setNativeNumber(node, CODE_TRANSITION, value);
    return;
  }
  if (typeof value === "string") {
    const shorthand = value.trim();
    if (shorthand.length === 0 || shorthand === "none") return;
    const names: string[] = [];
    let duration: number | undefined = undefined;
    let easing: string | undefined = undefined;
    for (const declaration of splitCssList(shorthand)) {
      let property: string | undefined = undefined;
      for (const token of splitCssTokens(declaration)) {
        const milliseconds = normalizeMilliseconds(token);
        if (milliseconds !== undefined && (token.endsWith("s") || duration === undefined) && property !== undefined) {
          duration = milliseconds;
          continue;
        }
        if (token === "linear" || token === "ease" || token === "ease-in" || token === "ease-out" || token === "ease-in-out") {
          easing = transitionEasingName(token);
          continue;
        }
        if (property === undefined) {
          property = transitionPropertyName(token);
          continue;
        }
        const late = normalizeMilliseconds(token);
        if (late !== undefined) duration = late;
        else throw new TypeError("QuickGUI cannot parse the transition `" + declaration + "`");
      }
      if (property === undefined) throw new TypeError("QuickGUI cannot transition `" + declaration + "`");
      names.push(property);
    }
    setNativeString(node, CODE_TRANSITION_PROPERTIES, names.join(","));
    if (duration !== undefined) setNativeNumber(node, CODE_TRANSITION_DURATION, duration);
    if (easing !== undefined) setNativeString(node, CODE_TRANSITION_EASING, easing);
    return;
  }
  const declaration: TransitionDeclaration = value;
  if (declaration.duration !== undefined) {
    const duration = normalizeMilliseconds(declaration.duration);
    if (duration !== undefined) setNativeNumber(node, CODE_TRANSITION_DURATION, duration);
  }
  const property = declaration.property;
  const properties = property === undefined ? declaration.properties : typeof property === "string" ? [property] : property;
  if (properties !== undefined) {
    const names: string[] = [];
    for (const name of properties) names.push(transitionPropertyName(name));
    setNativeString(node, CODE_TRANSITION_PROPERTIES, names.join(","));
  }
  const easing = declaration.easing ?? declaration.timingFunction;
  if (easing !== undefined) setNativeString(node, CODE_TRANSITION_EASING, transitionEasingName(easing));
  if (declaration.maxFps !== undefined) setNativeNumber(node, CODE_TRANSITION_MAX_FPS, declaration.maxFps);
}

interface EncodedStateStyle {
  group?: string;
  backgroundColor?: number;
  background?: string;
  color?: number;
  borderColor?: number;
  borderWidth?: number;
  borderRadius?: number;
  outline?: string;
  boxShadow?: EncodedBoxShadow[];
  opacity?: number;
  cursor?: string;
  transform?: string;
  transformOrigin?: string;
}

function stateLength(state: string, name: string, value: number | string): number {
  const length = typeof value === "number" ? value : lengthNumber(value);
  if (length === undefined || !Number.isFinite(length)) {
    throw new TypeError("QuickGUI `" + state + "` " + name + " must be a number of logical pixels");
  }
  return length;
}

function outlineShorthand(value: number | string): string {
  if (typeof value === "number") return String(value) + "px";
  const shorthand = value.trim();
  if (shorthand.length === 0 || shorthand.toLowerCase() === "none") return "none";
  for (const token of splitCssTokens(shorthand)) {
    if (token === "solid" || token === "dashed" || token === "dotted") continue;
    if (lengthNumber(token) !== undefined) continue;
    parseColor(token);
  }
  return bounded(shorthand, MAX_STYLE_DECLARATION_BYTES, "style declarations");
}

function encodeStateEntry(state: string, value: StateStyle, group: string | undefined): EncodedStateStyle | undefined {
  const encoded: EncodedStateStyle = {};
  let any = false;
  if (group !== undefined) {
    encoded.group = normalizeHoverGroupName(group);
    any = true;
  }
  const background = value.background;
  if (background !== undefined) {
    if (typeof background === "number") {
      encoded.backgroundColor = background >>> 0;
    } else if (typeof background === "string") {
      if (isGradientText(background)) encoded.background = background.trim();
      else encoded.backgroundColor = parseColor(background);
    } else {
      const gradient: GradientDeclaration = background;
      encoded.background = JSON.stringify(gradient);
    }
    any = true;
  }
  if (value.backgroundColor !== undefined) {
    encoded.backgroundColor = parseColor(value.backgroundColor);
    any = true;
  }
  if (value.color !== undefined) {
    encoded.color = parseColor(value.color);
    any = true;
  }
  if (value.borderColor !== undefined) {
    encoded.borderColor = parseColor(value.borderColor);
    any = true;
  }
  if (value.borderWidth !== undefined) {
    encoded.borderWidth = stateLength(state, "borderWidth", value.borderWidth);
    any = true;
  }
  if (value.borderRadius !== undefined) {
    encoded.borderRadius = stateLength(state, "borderRadius", value.borderRadius);
    any = true;
  }
  if (value.outline !== undefined) {
    encoded.outline = outlineShorthand(value.outline);
    any = true;
  }
  if (value.boxShadow !== undefined) {
    encoded.boxShadow = parseBoxShadowList(value.boxShadow);
    any = true;
  }
  if (value.opacity !== undefined) {
    if (!Number.isFinite(value.opacity)) throw new TypeError("QuickGUI `" + state + "` opacity must be a finite number");
    encoded.opacity = value.opacity;
    any = true;
  }
  if (value.cursor !== undefined) {
    if (state === "groupHover" || state === "groupActive" || state === "focusWithin") {
      throw new TypeError("QuickGUI `" + state + "` styles cannot declare a cursor; the pointer rests on another element");
    }
    encoded.cursor = value.cursor;
    any = true;
  }
  const transform = value.transform;
  if (transform !== undefined) {
    if (typeof transform === "string") encoded.transform = transform.trim();
    else if (Array.isArray(transform)) encoded.transform = transform.join(" ");
    else {
      const matrix: TransformMatrix = transform;
      encoded.transform = JSON.stringify(matrix);
    }
    any = true;
  }
  if (value.transformOrigin !== undefined) {
    encoded.transformOrigin = value.transformOrigin.trim();
    any = true;
  }
  return any ? encoded : undefined;
}

/** One nested interaction state, encoded as the bounded JSON the host parses. */
export function setStateStyle(node: NativeNode, code: number, state: string, value: StateStyle | undefined): void {
  if (value === undefined) {
    clearNativeProperty(node, code);
    return;
  }
  const encoded = encodeStateEntry(state, value, undefined);
  if (encoded === undefined) clearNativeProperty(node, code);
  else setNativeString(node, code, bounded(JSON.stringify(encoded), MAX_STATE_STYLE_JSON_BYTES, "state style declarations"));
}

/** A group state: one entry or a list of entries, each following its own group. */
export function setGroupStateStyle(
  node: NativeNode,
  code: number,
  state: string,
  value: GroupStateStyle | GroupStateStyle[] | undefined,
): void {
  if (value === undefined) {
    clearNativeProperty(node, code);
    return;
  }
  const entries: GroupStateStyle[] = Array.isArray(value) ? value : [value];
  const encoded: EncodedStateStyle[] = [];
  for (const entry of entries) {
    const declaration = encodeStateEntry(state, entry, entry.group);
    if (declaration !== undefined) encoded.push(declaration);
  }
  if (encoded.length === 0) {
    clearNativeProperty(node, code);
    return;
  }
  if (encoded.length > MAX_GROUP_STYLES_PER_ELEMENT) {
    throw new TypeError("QuickGUI `" + state + "` follows at most " + String(MAX_GROUP_STYLES_PER_ELEMENT) + " groups");
  }
  const json = encoded.length === 1 ? JSON.stringify(encoded[0]!) : JSON.stringify(encoded);
  setNativeString(node, code, bounded(json, MAX_STATE_STYLE_JSON_BYTES, "state style declarations"));
}

export function setFlex(node: NativeNode, _code: number, value: number | string | undefined): void {
  if (value === undefined) {
    clearNativeProperty(node, CODE_FLEX_GROW);
    clearNativeProperty(node, CODE_FLEX_SHRINK);
    clearNativeProperty(node, CODE_FLEX_BASIS);
    return;
  }
  if (typeof value === "number") {
    setNativeNumber(node, CODE_FLEX_GROW, value);
    setNativeNumber(node, CODE_FLEX_SHRINK, 1);
    setNativeNumber(node, CODE_FLEX_BASIS, 0);
    return;
  }
  const parts = splitCssTokens(value.trim());
  if (parts.length === 1 && parts[0] === "none") {
    setNativeNumber(node, CODE_FLEX_GROW, 0);
    setNativeNumber(node, CODE_FLEX_SHRINK, 0);
    setNativeString(node, CODE_FLEX_BASIS, "auto");
    return;
  }
  if (parts.length >= 1) setNumber(node, CODE_FLEX_GROW, Number(parts[0]!));
  if (parts.length >= 2) setNumber(node, CODE_FLEX_SHRINK, Number(parts[1]!));
  if (parts.length >= 3) setLength(node, CODE_FLEX_BASIS, parts[2]!);
}

/** A uniform radius stays a number; a one-to-four value shorthand travels as its CSS text. */
export function setBorderRadius(node: NativeNode, _code: number, value: number | string | undefined): void {
  if (value === undefined) {
    clearNativeProperty(node, CODE_BORDER_RADIUS);
    return;
  }
  if (typeof value === "number") {
    setNativeNumber(node, CODE_BORDER_RADIUS, value);
    return;
  }
  const text = value.trim();
  if (text.length === 0) {
    clearNativeProperty(node, CODE_BORDER_RADIUS);
    return;
  }
  const tokens = splitCssTokens(text);
  if (tokens.length > 4) throw new TypeError("QuickGUI borderRadius accepts one to four corner radii");
  if (tokens.length > 1) {
    setNativeString(node, CODE_BORDER_RADIUS, text);
    return;
  }
  setLength(node, CODE_BORDER_RADIUS, text);
}

function parseSpan(text: string): number | undefined {
  if (!text.startsWith("span")) return undefined;
  const count = Number(text.slice(4).trim());
  return Number.isFinite(count) ? count : undefined;
}

/** Split a `grid-column` / `grid-row` shorthand into the core's line and span placement. */
export function setGridPlacement(node: NativeNode, column: boolean, value: number | string | undefined): void {
  const startCode = column ? CODE_GRID_COLUMN_START : CODE_GRID_ROW_START;
  const endCode = column ? CODE_GRID_COLUMN_END : CODE_GRID_ROW_END;
  const spanCode = column ? CODE_GRID_COLUMN_SPAN : CODE_GRID_ROW_SPAN;
  clearNativeProperty(node, startCode);
  clearNativeProperty(node, endCode);
  clearNativeProperty(node, spanCode);
  if (value === undefined) return;
  if (typeof value === "number") {
    setNativeNumber(node, startCode, value);
    return;
  }
  const parts = value.split("/");
  const start = (parts.length > 0 ? parts[0]! : "").trim();
  const end = (parts.length > 1 ? parts[1]! : "").trim();
  const startSpan = parseSpan(start);
  if (startSpan !== undefined) {
    setNativeNumber(node, spanCode, startSpan);
    return;
  }
  const startLine = start === "" || start === "auto" ? undefined : Number(start);
  if (startLine !== undefined && Number.isFinite(startLine)) setNativeNumber(node, startCode, startLine);
  const endSpan = parseSpan(end);
  if (endSpan !== undefined) {
    if (startLine !== undefined && Number.isFinite(startLine)) setNativeNumber(node, endCode, startLine + endSpan);
    else setNativeNumber(node, spanCode, endSpan);
    return;
  }
  const endLine = end === "" || end === "auto" ? undefined : Number(end);
  if (endLine !== undefined && Number.isFinite(endLine)) setNativeNumber(node, endCode, endLine);
}

/** Split the CSS `outline` shorthand into the width, style, and color the core declares separately. */
export function setOutline(node: NativeNode, _code: number, value: number | string | undefined): void {
  clearNativeProperty(node, CODE_OUTLINE_WIDTH);
  clearNativeProperty(node, CODE_OUTLINE_COLOR);
  clearNativeProperty(node, CODE_OUTLINE_STYLE);
  if (value === undefined) return;
  if (typeof value === "number") {
    setNativeNumber(node, CODE_OUTLINE_WIDTH, value);
    return;
  }
  const shorthand = value.trim();
  if (shorthand.length === 0 || shorthand === "none") {
    setNativeString(node, CODE_OUTLINE_STYLE, "none");
    return;
  }
  for (const token of splitCssTokens(shorthand)) {
    if (token === "solid" || token === "dashed" || token === "dotted") {
      setNativeString(node, CODE_OUTLINE_STYLE, token);
      continue;
    }
    const width = lengthNumber(token);
    if (width !== undefined) {
      setNativeNumber(node, CODE_OUTLINE_WIDTH, width);
      continue;
    }
    setNativeColor(node, CODE_OUTLINE_COLOR, parseColor(token));
  }
}

/** A CSS grid track list, an array of tracks, or a count of equal `1fr` tracks. */
export function setGridTemplate(node: NativeNode, code: number, value: number | string | (number | string)[] | undefined): void {
  if (value === undefined) {
    clearNativeProperty(node, code);
    return;
  }
  if (typeof value === "number") {
    setNumber(node, code, value);
    return;
  }
  let text: string;
  if (typeof value === "string") {
    text = value;
  } else {
    const parts: string[] = [];
    for (const track of value) parts.push(typeof track === "number" ? String(track) : track);
    text = parts.join(" ");
  }
  const normalized = splitCssTokens(text.trim()).join(" ");
  if (normalized.length === 0 || normalized === "none") clearNativeProperty(node, code);
  else setNativeString(node, code, normalized);
}

export function setMilliseconds(node: NativeNode, code: number, value: number | string | undefined): void {
  if (value === undefined) {
    clearNativeProperty(node, code);
    return;
  }
  const milliseconds = normalizeMilliseconds(value);
  if (milliseconds === undefined) clearNativeProperty(node, code);
  else setNativeNumber(node, code, milliseconds);
}

/** A bounded component scope, value, or label. */
export function setComponentValue(node: NativeNode, code: number, value: string | undefined): void {
  if (value === undefined || value.length === 0) {
    clearNativeProperty(node, code);
    return;
  }
  setNativeString(node, code, bounded(value, MAX_COMPONENT_VALUE_BYTES, "component scopes and values"));
}

/** One bounded JSON declaration (option sources, columns, values, toasts, ...). */
export function setJson<T>(node: NativeNode, code: number, limit: number, value: T | undefined): void {
  if (value === undefined) {
    clearNativeProperty(node, code);
    return;
  }
  setNativeString(node, code, bounded(JSON.stringify(value), limit, "component declarations"));
}

export function setExtent(node: NativeNode, code: number, value: Extent | undefined): void {
  if (value === undefined) {
    clearNativeProperty(node, code);
    return;
  }
  if (!Number.isFinite(value.width) || !Number.isFinite(value.height)) throw new TypeError("QuickGUI extents must be finite numbers");
  setNativeString(node, code, "[" + String(value.width) + "," + String(value.height) + "]");
}

export function setShaderParameters(node: NativeNode, code: number, value: number[] | undefined): void {
  if (value === undefined) {
    clearNativeProperty(node, code);
    return;
  }
  if (value.length > MAX_SHADER_PARAMETER_FLOATS) {
    throw new RangeError("QuickGUI exposes " + String(MAX_SHADER_PARAMETER_FLOATS) + " shader parameter floats");
  }
  for (const entry of value) {
    if (!Number.isFinite(entry)) throw new TypeError("QuickGUI shader parameters must be finite numbers");
  }
  setNativeString(node, code, JSON.stringify(value));
}

export function setTooltipText(node: NativeNode, code: number, value: string | undefined): void {
  if (value === undefined || value.length === 0) clearNativeProperty(node, code);
  else setNativeString(node, code, bounded(value, MAX_TOOLTIP_TEXT_BYTES, "tooltip texts"));
}

export function setMenuLink(node: NativeNode, code: number, value: string | undefined): void {
  if (value === undefined || value.length === 0) clearNativeProperty(node, code);
  else setNativeString(node, code, bounded(value, MAX_MENU_LINK_BYTES, "menu links"));
}

export function normalizeHoverGroupName(value: string): string {
  const name = value.trim();
  if (name.length === 0) throw new TypeError("QuickGUI hover group names cannot be empty");
  return bounded(name, MAX_HOVER_GROUP_NAME_BYTES, "hover group names");
}

/** `true` opens an unnamed hover group; a string names one for `groupHover: { group }`. */
export function setHoverGroup(node: NativeNode, code: number, value: boolean | string | undefined): void {
  if (value === undefined) {
    clearNativeProperty(node, code);
  } else if (typeof value === "boolean") {
    if (value) setNativeBoolean(node, code, true);
    else clearNativeProperty(node, code);
  } else {
    setNativeString(node, code, normalizeHoverGroupName(value));
  }
}

export function setStringList(node: NativeNode, code: number, value: string[] | undefined): void {
  if (value === undefined) clearNativeProperty(node, code);
  else setNativeString(node, code, JSON.stringify(value));
}

/** Ordered name/value pairs encoded as the JSON object the host expects. */
export function setStringMap(node: NativeNode, code: number, value: StringMap | undefined): void {
  if (value === undefined) {
    clearNativeProperty(node, code);
    return;
  }
  const parts: string[] = [];
  for (const entry of value) {
    if (entry.name.length === 0) throw new TypeError("QuickGUI string map keys cannot be empty");
    parts.push(JSON.stringify(entry.name) + ":" + JSON.stringify(entry.value));
  }
  setNativeString(node, code, "{" + parts.join(",") + "}");
}

export function setTerminalPalette(node: NativeNode, code: number, value: TerminalPalette | undefined): void {
  if (value === undefined) {
    clearNativeProperty(node, code);
    return;
  }
  if (value.length !== 16) throw new TypeError("QuickGUI terminalPalette must contain exactly 16 colors");
  const packed: number[] = [];
  for (const color of value) packed.push(parseColor(color));
  setNativeString(node, code, JSON.stringify(packed));
}

export function setInputType(node: NativeNode, _code: number, value: "text" | "password" | undefined): void {
  setNativeBoolean(node, CODE_PASSWORD, value === "password");
}

export function setKeymap(node: NativeNode, code: number, value: Keymap | undefined): void {
  if (value === undefined) {
    clearNativeProperty(node, code);
    return;
  }
  const parts: string[] = [];
  for (const binding of value) {
    if (binding.action.length === 0) throw new TypeError("QuickGUI keymap binding ids must be nonempty");
    parts.push(JSON.stringify(binding.keys) + ":" + JSON.stringify(binding.action));
  }
  setNativeString(node, code, bounded("{" + parts.join(",") + "}", MAX_KEYMAP_JSON_BYTES, "keymaps"));
}

export function setDragSource(node: NativeNode, code: number, value: boolean | DragSource | undefined): void {
  if (value === undefined) {
    clearNativeProperty(node, code);
    return;
  }
  if (typeof value === "boolean") {
    if (value) setNativeString(node, code, "{}");
    else clearNativeProperty(node, code);
    return;
  }
  const source: DragSource = value;
  setNativeString(node, code, bounded(JSON.stringify(source), MAX_DRAG_JSON_BYTES, "drag declarations"));
}

export function setDropKinds(node: NativeNode, code: number, value: DropKind | DropKind[] | undefined): void {
  if (value === undefined) {
    clearNativeProperty(node, code);
    return;
  }
  const kinds: DropKind[] = typeof value === "string" ? [value] : value;
  setNativeString(node, code, JSON.stringify(kinds));
}

/** Attach one event listener; writes inside it are batched into a single update. */
export function setListener(node: NativeNode, type: number, handler: ((event: QuickGuiEvent) => void) | undefined): void {
  if (handler === undefined) {
    setNativeEventListener(node, type, undefined);
    return;
  }
  setNativeEventListener(node, type, (event: QuickGuiEvent) => {
    batch(() => {
      handler(event);
    });
  });
}

// ---------------------------------------------------------------------------
// Nodes, children, and reactive regions
// ---------------------------------------------------------------------------

export type Child = string | number | boolean | NativeNode | undefined | null | Child[];
export type Children = Child | Child[];

export function element(tag: number): NativeNode {
  return createNativeElement(tag);
}

export function text(value: string): NativeNode {
  return createNativeText(value);
}

/** Insert `child` at the end of `parent`. */
export function insert(parent: NativeNode, child: NativeNode): void {
  insertNativeNode(parent, child, undefined);
}

function insertChild(parent: NativeNode, child: Child): void {
  if (child === undefined || child === null) return;
  if (typeof child === "boolean") return;
  if (Array.isArray(child)) {
    for (const nested of child) insertChild(parent, nested);
    return;
  }
  if (typeof child === "string") {
    insertNativeNode(parent, createNativeText(child), undefined);
    return;
  }
  if (typeof child === "number") {
    insertNativeNode(parent, createNativeText(String(child)), undefined);
    return;
  }
  const node: NativeNode = child;
  insertNativeNode(parent, node, undefined);
}

/** Insert plain children: strings and numbers become text nodes; booleans and holes are skipped. */
export function insertChildren(parent: NativeNode, children: Children): void {
  if (Array.isArray(children)) {
    for (const child of children) insertChild(parent, child);
    return;
  }
  insertChild(parent, children);
}

/** A text node that follows a reactive string. */
export function dynamicText(value: Accessor<string>): NativeNode {
  const node = createNativeText("");
  let last = "";
  createRenderEffect(() => {
    const next = value();
    if (next !== last) {
      last = next;
      replaceNativeText(node, next);
    }
  });
  return node;
}

/** A fragment built from plain children: strings and numbers become text nodes; booleans and holes are skipped. */
export function childrenFragment(children: Children): NativeNode {
  const nodes: NativeNode[] = [];
  collectChildren(children, nodes);
  return fragment(nodes);
}

function collectChildren(children: Children, nodes: NativeNode[]): void {
  if (Array.isArray(children)) {
    for (const child of children) collectChildren(child, nodes);
    return;
  }
  const child = children;
  if (child === undefined || child === null || typeof child === "boolean") return;
  if (typeof child === "string") {
    nodes.push(createNativeText(child));
    return;
  }
  if (typeof child === "number") {
    nodes.push(createNativeText(String(child)));
    return;
  }
  nodes.push(child);
}

/** A sentinel that keeps `nodes` immediately before itself wherever it is inserted. */
export function fragment(nodes: NativeNode[]): NativeNode {
  const sentinel = createNativeSentinel();
  sentinel.group = nodes;
  return sentinel;
}

/**
 * A region: one sentinel whose content is replaced reactively. The content's owner is disposed
 * before the next content is created, and content nodes are removed from the host on swap.
 */
export class Region {
  readonly sentinel: NativeNode;
  readonly parentOwner: Owner | undefined;
  owner: Owner | undefined = undefined;

  constructor() {
    this.sentinel = createNativeSentinel();
    this.parentOwner = getOwner();
  }

  /** Remove and dispose the current content. */
  clear(): void {
    const owner = this.owner;
    if (owner !== undefined) {
      this.owner = undefined;
      disposeOwner(owner);
    }
    const group = this.sentinel.group;
    if (group !== undefined) {
      const parent = this.sentinel.parent;
      if (parent !== undefined) {
        for (const node of group) removeNativeNode(parent, node);
      }
      this.sentinel.group = undefined;
    }
  }

  /** Replace the content with the nodes `render` produces under a fresh owner. */
  replace(render: () => NativeNode[]): void {
    this.clear();
    const owner = new Owner(this.parentOwner);
    // The effect replacing the content runs before anything inside it when both are stale.
    owner.controller = getComputation();
    this.owner = owner;
    const nodes = runWithOwner(owner, render);
    this.sentinel.group = nodes;
    const parent = this.sentinel.parent;
    if (parent !== undefined) {
      for (const node of nodes) insertNativeNode(parent, node, this.sentinel);
    }
  }
}

/** A region whose single node is re-created whenever the accessor's dependencies change. */
export function dynamic(render: Accessor<NativeNode>): NativeNode {
  const region = new Region();
  createRenderEffect(() => {
    const node = render();
    region.replace(() => [node]);
  });
  return region.sentinel;
}

/** A region showing one node or nothing. */
export function dynamicMaybe(render: Accessor<NativeNode | undefined>): NativeNode {
  const region = new Region();
  createRenderEffect(() => {
    const node = render();
    if (node === undefined) region.clear();
    else region.replace(() => [node]);
  });
  return region.sentinel;
}

/** A region that replaces its whole list when the accessor's dependencies change. */
export function dynamicList(render: Accessor<NativeNode[]>): NativeNode {
  const region = new Region();
  createRenderEffect(() => {
    const nodes = render();
    region.replace(() => nodes);
  });
  return region.sentinel;
}

// ---------------------------------------------------------------------------
// Control flow
// ---------------------------------------------------------------------------

export interface ShowProps<T> {
  when: Accessor<T>;
  children: () => NativeNode;
  fallback?: () => NativeNode;
  /** Re-create the children when the condition's value changes, not only its truthiness. */
  keyed?: boolean;
}

/** Render `children` while `when` is truthy, else `fallback`. Children are created lazily. */
export function Show<T>(props: ShowProps<T>): NativeNode {
  const region = new Region();
  let shown = 0;
  // The last truthy condition, in a one-element array: the native compiler cannot hold a generic union.
  let previous: T[] = [];
  createRenderEffect(() => {
    const condition = props.when();
    const truthy = condition ? true : false;
    if (truthy) {
      if (shown === 1 && !(props.keyed === true && previous.length === 1 && condition !== previous[0])) return;
      previous = [condition];
      shown = 1;
      region.replace(() => [untrack(props.children)]);
      return;
    }
    if (shown === 2) return;
    shown = 2;
    const fallback = props.fallback;
    if (fallback === undefined) region.clear();
    else region.replace(() => [untrack(fallback)]);
  });
  return region.sentinel;
}

class ForRow<T> {
  readonly item: Signal<T>;
  key: string;
  readonly node: NativeNode;
  readonly owner: Owner;
  readonly index: Signal<number>;

  constructor(item: Signal<T>, key: string, node: NativeNode, owner: Owner, index: Signal<number>) {
    this.item = item;
    this.key = key;
    this.node = node;
    this.owner = owner;
    this.index = index;
  }
}

export interface ForProps<T> {
  each: Accessor<T[]>;
  children: (item: T, index: Accessor<number>) => NativeNode;
  /** Stable identity for reordering; without it rows are keyed by position and re-created when their item changes. */
  key?: (item: T) => string | number;
  fallback?: () => NativeNode;
}

function rowKey<T>(key: ((item: T) => string | number) | undefined, item: T, index: number): string {
  if (key === undefined) return String(index);
  const value = key(item);
  return typeof value === "number" ? "n" + String(value) : "s" + value;
}

/** Render one row per item, reusing rows whose key survives and moving them into order. */
export function For<T>(props: ForProps<T>): NativeNode {
  return renderList(props.each, props.key, (item: Accessor<T>, index: Accessor<number>): NativeNode => props.children(item(), index), props.fallback, false);
}

export interface KeyedForProps<T> {
  each: Accessor<T[]>;
  key: (item: T) => string | number;
  children: (item: Accessor<T>, index: Accessor<number>) => NativeNode;
  fallback?: () => NativeNode;
}

/** Keep each row mounted while its keyed data changes; item accessors update only their consumers. */
export function KeyedFor<T>(props: KeyedForProps<T>): NativeNode {
  return renderList(props.each, props.key, props.children, props.fallback, true);
}

function renderList<T>(
  each: Accessor<T[]>,
  keyFor: ((item: T) => string | number) | undefined,
  render: (item: Accessor<T>, index: Accessor<number>) => NativeNode,
  fallback: (() => NativeNode) | undefined,
  reactiveItems: boolean,
): NativeNode {
  const region = new Region();
  let rows: ForRow<T>[] = [];
  let showingFallback = false;
  const disposeRow = (row: ForRow<T>): void => {
    disposeOwner(row.owner);
    const parent = region.sentinel.parent;
    if (parent !== undefined && row.node.parent === parent) removeNativeNode(parent, row.node);
  };
  createRenderEffect(() => {
    const items = each();
    untrack(() => {
      if (items.length === 0) {
        for (const row of rows) disposeRow(row);
        rows = [];
        region.sentinel.group = undefined;
        if (fallback !== undefined && !showingFallback) {
          showingFallback = true;
          region.replace(() => [fallback()]);
        }
        return;
      }
      if (showingFallback) {
        showingFallback = false;
        region.clear();
      }
      const existing = new Map<string, ForRow<T>>();
      for (const row of rows) existing.set(row.key, row);
      const next: ForRow<T>[] = [];
      for (let index = 0; index < items.length; index++) {
        const item = items[index]!;
        const key = rowKey(keyFor, item, index);
        let row = existing.get(key);
        if (row !== undefined && (reactiveItems || row.item.peek() === item)) {
          existing.delete(key);
          row.item.write(item);
          if (row.index.peek() !== index) row.index.write(index);
          next.push(row);
          continue;
        }
        if (row !== undefined) {
          // Same position, different item: the row is rebuilt.
          existing.delete(key);
          disposeRow(row);
        }
        const owner = new Owner(region.parentOwner);
        owner.controller = getComputation();
        const indexSignal = new Signal<number>(index, true);
        const itemSignal = new Signal<T>(item, true);
        const node = runWithOwner(owner, () => render(() => itemSignal.read(), () => indexSignal.read()));
        next.push(new ForRow<T>(itemSignal, key, node, owner, indexSignal));
      }
      for (const row of existing.values()) disposeRow(row);
      rows = next;
      const nodes: NativeNode[] = [];
      for (const row of next) nodes.push(row.node);
      region.sentinel.group = nodes;
      const parent = region.sentinel.parent;
      if (parent !== undefined) {
        // Place rows in order from the end, moving only the ones that are out of place.
        let anchor: NativeNode = region.sentinel;
        let index = nodes.length;
        while (index > 0) {
          index -= 1;
          const node = nodes[index]!;
          const siblings = parent.children;
          const position = siblings.indexOf(node);
          const anchorPosition = siblings.indexOf(anchor);
          if (position < 0 || position + 1 !== anchorPosition) insertNativeNode(parent, node, anchor);
          anchor = node;
        }
      }
    });
  });
  return region.sentinel;
}

class IndexRow<T> {
  readonly node: NativeNode;
  readonly owner: Owner;
  readonly item: Signal<T>;

  constructor(node: NativeNode, owner: Owner, item: Signal<T>) {
    this.node = node;
    this.owner = owner;
    this.item = item;
  }
}

export interface IndexProps<T> {
  each: Accessor<T[]>;
  children: (item: Accessor<T>, index: number) => NativeNode;
  fallback?: () => NativeNode;
}

/** Render one row per position; the row's item accessor updates in place when the list changes. */
export function Index<T>(props: IndexProps<T>): NativeNode {
  const region = new Region();
  let rows: IndexRow<T>[] = [];
  let showingFallback = false;
  createRenderEffect(() => {
    const items = props.each();
    untrack(() => {
      if (items.length === 0 && props.fallback !== undefined) {
        for (const row of rows) disposeOwner(row.owner);
        rows = [];
        if (!showingFallback) {
          showingFallback = true;
          const fallback = props.fallback;
          region.replace(() => [fallback()]);
        }
        return;
      }
      if (showingFallback) {
        showingFallback = false;
        region.clear();
      }
      const parent = region.sentinel.parent;
      while (rows.length > items.length) {
        const row = rows.pop()!;
        disposeOwner(row.owner);
        if (parent !== undefined && row.node.parent === parent) removeNativeNode(parent, row.node);
      }
      for (let index = 0; index < rows.length; index++) {
        const row = rows[index]!;
        const item = items[index]!;
        if (row.item.peek() !== item) row.item.write(item);
      }
      const added: NativeNode[] = [];
      for (let index = rows.length; index < items.length; index++) {
        const owner = new Owner(region.parentOwner);
        owner.controller = getComputation();
        const item = new Signal<T>(items[index]!, true);
        const node = runWithOwner(owner, () => props.children(() => item.read(), index));
        rows.push(new IndexRow<T>(node, owner, item));
        added.push(node);
      }
      const nodes: NativeNode[] = [];
      for (const row of rows) nodes.push(row.node);
      region.sentinel.group = nodes;
      if (parent !== undefined) {
        for (const node of added) insertNativeNode(parent, node, region.sentinel);
      }
    });
  });
  return region.sentinel;
}

class SwitchCase {
  readonly when: () => boolean;
  readonly region: Region;
  readonly render: () => NativeNode;

  constructor(when: () => boolean, region: Region, render: () => NativeNode) {
    this.when = when;
    this.region = region;
    this.render = render;
  }
}

class SwitchState {
  readonly cases: SwitchCase[] = [];
}

const switchContext = new Context<SwitchState | undefined>(undefined);

export interface MatchProps<T> {
  when: Accessor<T>;
  children: () => NativeNode;
}

/** One `Switch` branch: rendered while it is the first branch whose condition holds. */
export function Match<T>(props: MatchProps<T>): NativeNode {
  const state = switchContext.use();
  if (state === undefined) throw new Error("<Match> must be a direct child of <Switch>");
  const region = new Region();
  state.cases.push(new SwitchCase(() => (props.when() ? true : false), region, props.children));
  return region.sentinel;
}

export interface SwitchProps {
  children: () => NativeNode;
  fallback?: () => NativeNode;
}

/** Render the first `Match` whose condition holds, else the fallback. */
export function Switch(props: SwitchProps): NativeNode {
  const state = new SwitchState();
  const fallback = new Region();
  const content = switchContext.provide(state, props.children);
  let selected = -2;
  createRenderEffect(() => {
    let index = -1;
    for (let i = 0; i < state.cases.length; i++) {
      if (state.cases[i]!.when()) {
        index = i;
        break;
      }
    }
    if (index === selected) return;
    const previous = selected;
    selected = index;
    if (previous >= 0) state.cases[previous]!.region.clear();
    if (previous === -1) fallback.clear();
    if (index >= 0) {
      const branch = state.cases[index]!;
      branch.region.replace(() => [untrack(branch.render)]);
    } else {
      const render = props.fallback;
      if (render !== undefined) fallback.replace(() => [untrack(render)]);
    }
  });
  return fragment([content, fallback.sentinel]);
}

// ---------------------------------------------------------------------------
// Renderer
// ---------------------------------------------------------------------------

/** Mount `render` into a window; the returned adapter is what `new Window({ renderer })` takes. */
export function createRenderer(render: () => NativeNode): WindowRenderer {
  return (window: Window) => {
    let disposed = false;
    const dispose = createRoot((disposeRoot) => {
      const node = render();
      insertNativeNode(window.root, node, undefined);
      return disposeRoot;
    });
    window.flush();
    return () => {
      if (disposed) return;
      disposed = true;
      dispose();
      window.flush();
    };
  };
}
