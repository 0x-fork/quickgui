/**
 * Shared machinery of the compound components: the props every part accepts, how a part applies
 * them to its host element, and the scope keys that tie the parts of one instance together.
 *
 * A part is one host element the Rust core recognizes by its `part` name. The core owns the
 * semantics, keyboard behavior, and derived identities; the component only declares the
 * controlled value and forwards the application's styling and listeners.
 */

import { NativeNodeTag, type NativeNode, type QuickGuiEvent } from "@quickgui/native";
import {
  EVENT_BLUR,
  EVENT_CLICK,
  EVENT_FOCUS,
  EVENT_INPUT,
  EVENT_KEY_DOWN,
  EVENT_KEY_UP,
  EVENT_MOUSE_DOWN,
  EVENT_MOUSE_ENTER,
  EVENT_MOUSE_LEAVE,
  EVENT_MOUSE_UP,
  EVENT_SUBMIT,
} from "@quickgui/native/native-tree";

import {
  CODE_ARIA_LABEL,
  CODE_DISABLED,
  CODE_FOCUS_ON_POINTER,
  CODE_GROUP,
  CODE_MULTILINE,
  CODE_PART,
  CODE_PART_VALUE,
  CODE_PLACEHOLDER,
  CODE_ROLE,
  CODE_SCOPE,
  CODE_TAB_INDEX,
  CODE_TYPE,
  CODE_VALUE,
  applyStyle,
  applyStyleList,
  mergeStyle,
  type Style,
} from "./generated.ts";
import { createContext, createRenderEffect, useContext, type Accessor, type Context } from "./reactive.ts";
import {
  element,
  insert,
  setComponentValue,
  setExplicitBool,
  setInputType,
  setHoverGroup,
  setListener,
  setNumber,
  setString,
} from "./runtime.ts";

/** A value a part reads once, or an accessor it follows. */
export type Dynamic<T> = T | Accessor<T>;

/** Children of a part, created after the part's own context exists. */
export type PartChildren = () => NativeNode;

/** Props every compound part accepts. */
export interface PartProps {
  ref?: ((node: NativeNode) => void) | undefined;
  /** One style record, or a list merged left to right, read once or followed reactively. */
  style?: Dynamic<Style | Style[]> | undefined;
  children?: PartChildren | undefined;
  disabled?: Dynamic<boolean> | undefined;
  role?: string | undefined;
  focusOnPointer?: Dynamic<boolean> | undefined;
  group?: Dynamic<boolean | string> | undefined;
  ariaLabel?: Dynamic<string> | undefined;
  tabIndex?: number | undefined;
  onClick?: ((event: QuickGuiEvent) => void) | undefined;
  onMouseEnter?: ((event: QuickGuiEvent) => void) | undefined;
  onMouseLeave?: ((event: QuickGuiEvent) => void) | undefined;
  onMouseDown?: ((event: QuickGuiEvent) => void) | undefined;
  onMouseUp?: ((event: QuickGuiEvent) => void) | undefined;
  onKeyDown?: ((event: QuickGuiEvent) => void) | undefined;
  onKeyUp?: ((event: QuickGuiEvent) => void) | undefined;
  onFocus?: ((event: QuickGuiEvent) => void) | undefined;
  onBlur?: ((event: QuickGuiEvent) => void) | undefined;
}

/** Props of a part that hosts a native text input. */
export interface InputPartProps extends PartProps {
  type?: "text" | "password" | undefined;
  value?: Dynamic<string> | undefined;
  placeholder?: string | undefined;
  multiline?: boolean | undefined;
  onInput?: ((event: QuickGuiEvent) => void) | undefined;
  onChange?: ((event: QuickGuiEvent) => void) | undefined;
  onSubmit?: ((event: QuickGuiEvent) => void) | undefined;
}

// --- dynamic setters -----------------------------------------------------------------------------

export function bindExplicitBool(node: NativeNode, code: number, value: Dynamic<boolean>): void {
  if (typeof value === "function") {
    createRenderEffect(() => {
      setExplicitBool(node, code, value());
    });
  } else setExplicitBool(node, code, value);
}

export function bindString(node: NativeNode, code: number, value: Dynamic<string>): void {
  if (typeof value === "function") {
    createRenderEffect(() => {
      setString(node, code, value());
    });
  } else setString(node, code, value);
}

export function bindNumber(node: NativeNode, code: number, value: Dynamic<number>): void {
  if (typeof value === "function") {
    createRenderEffect(() => {
      setNumber(node, code, value());
    });
  } else setNumber(node, code, value);
}

/** Read a dynamic value once, inside whatever tracking scope the caller runs in. */
export function resolveBoolean(value: Dynamic<boolean> | undefined): boolean | undefined {
  if (value === undefined) return undefined;
  return typeof value === "function" ? value() : value;
}

export function resolveString(value: Dynamic<string> | undefined): string | undefined {
  if (value === undefined) return undefined;
  return typeof value === "function" ? value() : value;
}

export function resolveNumber(value: Dynamic<number> | undefined): number | undefined {
  if (value === undefined) return undefined;
  return typeof value === "function" ? value() : value;
}

export function resolveStyle(value: Dynamic<Style | Style[]> | undefined): Style | undefined {
  if (value === undefined) return undefined;
  const current = typeof value === "function" ? value() : value;
  if (!Array.isArray(current)) return current;
  const merged: Style = {};
  for (const style of current) mergeStyle(merged, style);
  return merged;
}

/** Keep a part's host element styled from one record or a list, static or reactive. */
export function bindPartStyle(node: NativeNode, style: Dynamic<Style | Style[]>): void {
  if (typeof style === "function") {
    let previous: Style | undefined = undefined;
    createRenderEffect(() => {
      const next = style();
      if (Array.isArray(next)) previous = applyStyleList(node, next, previous);
      else {
        applyStyle(node, next, previous);
        previous = next;
      }
    });
    return;
  }
  if (Array.isArray(style)) applyStyleList(node, style, undefined);
  else applyStyle(node, style, undefined);
}

// --- parts ---------------------------------------------------------------------------------------

/** Apply the application-facing props of a part to its host element. */
export function applyPart(node: NativeNode, props: PartProps): void {
  const style = props.style;
  if (style !== undefined) bindPartStyle(node, style);
  applyPartBehavior(node, props);
}

/** Apply every part prop except `style`, for parts that compose their own style list. */
export function applyPartBehavior(node: NativeNode, props: PartProps): void {
  const disabled = props.disabled;
  if (disabled !== undefined) bindExplicitBool(node, CODE_DISABLED, disabled);
  const focusOnPointer = props.focusOnPointer;
  if (focusOnPointer !== undefined) bindExplicitBool(node, CODE_FOCUS_ON_POINTER, focusOnPointer);
  const group = props.group;
  if (group !== undefined) {
    if (typeof group === "function") createRenderEffect(() => { setHoverGroup(node, CODE_GROUP, group()); });
    else setHoverGroup(node, CODE_GROUP, group);
  }
  if (props.role !== undefined) setString(node, CODE_ROLE, props.role);
  const ariaLabel = props.ariaLabel;
  if (ariaLabel !== undefined) bindString(node, CODE_ARIA_LABEL, ariaLabel);
  if (props.tabIndex !== undefined) setNumber(node, CODE_TAB_INDEX, props.tabIndex);
  // Optional function props are read into locals first: the static compiler narrows a local,
  // not a property access.
  const onClick = props.onClick;
  if (onClick !== undefined) setListener(node, EVENT_CLICK, onClick);
  const onMouseEnter = props.onMouseEnter;
  if (onMouseEnter !== undefined) setListener(node, EVENT_MOUSE_ENTER, onMouseEnter);
  const onMouseLeave = props.onMouseLeave;
  if (onMouseLeave !== undefined) setListener(node, EVENT_MOUSE_LEAVE, onMouseLeave);
  const onMouseDown = props.onMouseDown;
  if (onMouseDown !== undefined) setListener(node, EVENT_MOUSE_DOWN, onMouseDown);
  const onMouseUp = props.onMouseUp;
  if (onMouseUp !== undefined) setListener(node, EVENT_MOUSE_UP, onMouseUp);
  const onKeyDown = props.onKeyDown;
  if (onKeyDown !== undefined) setListener(node, EVENT_KEY_DOWN, onKeyDown);
  const onKeyUp = props.onKeyUp;
  if (onKeyUp !== undefined) setListener(node, EVENT_KEY_UP, onKeyUp);
  const onFocus = props.onFocus;
  if (onFocus !== undefined) setListener(node, EVENT_FOCUS, onFocus);
  const onBlur = props.onBlur;
  if (onBlur !== undefined) setListener(node, EVENT_BLUR, onBlur);
}

/** Apply the text-input props of an input part. */
export function applyInputPart(node: NativeNode, props: InputPartProps): void {
  if (props.type !== undefined) setInputType(node, CODE_TYPE, props.type);
  const value = props.value;
  if (value !== undefined) bindString(node, CODE_VALUE, value);
  if (props.placeholder !== undefined) setString(node, CODE_PLACEHOLDER, props.placeholder);
  if (props.multiline !== undefined) setExplicitBool(node, CODE_MULTILINE, props.multiline);
  const onInput = props.onInput;
  if (onInput !== undefined) setListener(node, EVENT_INPUT, onInput);
  const onChange = props.onChange;
  if (onChange !== undefined) setListener(node, EVENT_INPUT, onChange);
  const onSubmit = props.onSubmit;
  if (onSubmit !== undefined) setListener(node, EVENT_SUBMIT, onSubmit);
}

/** Create a part's host element with its application props applied; children are not mounted yet. */
export function createPart(tag: number, props: PartProps): NativeNode {
  const node = element(tag);
  applyPart(node, props);
  return node;
}

/** A part hosted by a plain view. */
export function createViewPart(props: PartProps): NativeNode {
  return createPart(NativeNodeTag.View, props);
}

/** A part hosted by a focusable native button. */
export function createButtonPart(props: PartProps): NativeNode {
  return createPart(NativeNodeTag.Button, props);
}

/** Declare which core part descriptor a host element is, inside an instance scope. */
export function setPart(node: NativeNode, part: string, scope: string | undefined, partValue: string | undefined): void {
  setString(node, CODE_PART, part);
  if (scope !== undefined) setComponentValue(node, CODE_SCOPE, scope);
  if (partValue !== undefined) setComponentValue(node, CODE_PART_VALUE, partValue);
}

/** Mount a part's lazily created children, then hand the node to its `ref`. */
export function finishPart(node: NativeNode, props: PartProps): NativeNode {
  const children = props.children;
  if (children !== undefined) insert(node, children());
  const ref = props.ref;
  if (ref !== undefined) ref(node);
  return node;
}

/** Chain an application click handler ahead of the part's own activation. */
export function forwardClick(
  handler: ((event: QuickGuiEvent) => void) | undefined,
  activate: (event: QuickGuiEvent) => void,
): (event: QuickGuiEvent) => void {
  return (event: QuickGuiEvent): void => {
    if (handler !== undefined) handler(event);
    if (!event.defaultPrevented) activate(event);
  };
}

// --- scopes and contexts -------------------------------------------------------------------------

let nextComponentScope = 1;

/**
 * Allocate one bounded scope key shared by every part of a compound component instance.
 *
 * The Rust binding hashes it into the same `ElementId` the core component would have used, so
 * derived part identities and accessibility relationships resolve with no registry and no
 * synchronous question asked of the application.
 */
export function createComponentScope(prefix: string): string {
  const scope = prefix + "-" + String(nextComponentScope);
  nextComponentScope += 1;
  return scope;
}

/** A compound context whose value is absent outside its root. */
export function createPartContext<T>(): Context<T | undefined> {
  return createContext<T | undefined>(undefined);
}

/** The nearest value of a compound context, or an error naming the missing root. */
export function requireContext<T>(context: Context<T | undefined>, part: string, root: string): T {
  const value = useContext(context);
  if (value === undefined) throw new TypeError(part + " must be used inside <" + root + ">");
  return value;
}
