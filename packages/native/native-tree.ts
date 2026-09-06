/**
 * The application-side retained node tree.
 *
 * Nodes are thin: an id, a tag, a parent, children, and the listeners the application attached.
 * Property values are not retained here; the Rust host keeps the committed tree and every
 * reactive binding remembers its own last value. A subtree that is not mounted in a window
 * records its mutations into one detached buffer that is spliced into the window batch, in
 * order, the moment the subtree is inserted, so building a view never waits on the host.
 */

import {
  MutationBatch,
  NativeNodeTag,
  NO_ANCHOR,
  PropertyCode,
  type NativePropertyValue,
} from "./protocol.ts";
import { hostApplyBatch, hostFocusNode } from "./ffi.ts";

// Event types, numbered so listeners live in plain arrays.
export const EVENT_CLICK = 1;
export const EVENT_MOUSE_ENTER = 2;
export const EVENT_MOUSE_LEAVE = 3;
export const EVENT_INPUT = 4;
export const EVENT_SUBMIT = 5;
export const EVENT_DISMISS = 6;
export const EVENT_TERMINAL = 7;
export const EVENT_POINTER = 8;
export const EVENT_PRESENTATION_CHANGE = 9;
export const EVENT_MENU_SELECT = 10;
export const EVENT_KEY_DOWN = 11;
export const EVENT_KEY_UP = 12;
export const EVENT_MOUSE_DOWN = 13;
export const EVENT_MOUSE_UP = 14;
export const EVENT_MOUSE_MOVE = 15;
export const EVENT_DOUBLE_CLICK = 16;
export const EVENT_WHEEL = 17;
export const EVENT_CONTEXT_MENU = 18;
export const EVENT_PINCH = 19;
export const EVENT_ROTATE = 20;
export const EVENT_SMART_MAGNIFY = 21;
export const EVENT_PRESSURE = 22;
export const EVENT_FOCUS = 23;
export const EVENT_BLUR = 24;
export const EVENT_ACTION = 25;
export const EVENT_DRAG_START = 26;
export const EVENT_DRAG_END = 27;
export const EVENT_DROP = 28;
export const EVENT_FILES_DROPPED = 29;
export const EVENT_COMPONENT_CHANGE = 30;
export const EVENT_COMMIT = 31;

/** Map one native event kind onto its numeric type, or 0 for kinds nodes never receive. */
export function eventTypeFromKind(kind: string): number {
  switch (kind) {
    case "click":
      return EVENT_CLICK;
    case "mouseenter":
      return EVENT_MOUSE_ENTER;
    case "mouseleave":
      return EVENT_MOUSE_LEAVE;
    case "input":
      return EVENT_INPUT;
    case "submit":
      return EVENT_SUBMIT;
    case "dismiss":
      return EVENT_DISMISS;
    case "terminal":
      return EVENT_TERMINAL;
    case "pointer":
      return EVENT_POINTER;
    case "presentationchange":
      return EVENT_PRESENTATION_CHANGE;
    case "menuselect":
      return EVENT_MENU_SELECT;
    case "keydown":
      return EVENT_KEY_DOWN;
    case "keyup":
      return EVENT_KEY_UP;
    case "mousedown":
      return EVENT_MOUSE_DOWN;
    case "mouseup":
      return EVENT_MOUSE_UP;
    case "mousemove":
      return EVENT_MOUSE_MOVE;
    case "dblclick":
      return EVENT_DOUBLE_CLICK;
    case "wheel":
      return EVENT_WHEEL;
    case "contextmenu":
      return EVENT_CONTEXT_MENU;
    case "pinch":
      return EVENT_PINCH;
    case "rotate":
      return EVENT_ROTATE;
    case "smartmagnify":
      return EVENT_SMART_MAGNIFY;
    case "pressure":
      return EVENT_PRESSURE;
    case "focus":
      return EVENT_FOCUS;
    case "blur":
      return EVENT_BLUR;
    case "action":
      return EVENT_ACTION;
    case "dragstart":
      return EVENT_DRAG_START;
    case "dragend":
      return EVENT_DRAG_END;
    case "drop":
      return EVENT_DROP;
    case "filesdropped":
      return EVENT_FILES_DROPPED;
    case "componentchange":
      return EVENT_COMPONENT_CHANGE;
    case "commit":
      return EVENT_COMMIT;
    default:
      return 0;
  }
}

/** The declared-listener property the host reads for one event type, or 0 when it is implicit. */
function listenerPropertyFor(type: number): number {
  switch (type) {
    case EVENT_CLICK:
      return PropertyCode.ClickListener;
    case EVENT_MOUSE_ENTER:
    case EVENT_MOUSE_LEAVE:
      return PropertyCode.HoverListener;
    case EVENT_INPUT:
      return PropertyCode.InputListener;
    case EVENT_SUBMIT:
      return PropertyCode.SubmitListener;
    case EVENT_DISMISS:
      return PropertyCode.DismissListener;
    case EVENT_TERMINAL:
      return PropertyCode.TerminalStatusListener;
    case EVENT_POINTER:
      return PropertyCode.PointerListener;
    case EVENT_PRESENTATION_CHANGE:
      return PropertyCode.SwiftUIPresentationListener;
    case EVENT_MENU_SELECT:
      return PropertyCode.SelectListener;
    case EVENT_KEY_DOWN:
      return PropertyCode.KeyDownListener;
    case EVENT_KEY_UP:
      return PropertyCode.KeyUpListener;
    case EVENT_MOUSE_DOWN:
      return PropertyCode.MouseDownListener;
    case EVENT_MOUSE_UP:
      return PropertyCode.MouseUpListener;
    case EVENT_MOUSE_MOVE:
      return PropertyCode.MouseMoveListener;
    case EVENT_DOUBLE_CLICK:
      return PropertyCode.DoubleClickListener;
    case EVENT_WHEEL:
      return PropertyCode.ScrollListener;
    case EVENT_CONTEXT_MENU:
      return PropertyCode.ContextMenuListener;
    case EVENT_PINCH:
      return PropertyCode.PinchListener;
    case EVENT_ROTATE:
      return PropertyCode.RotationListener;
    case EVENT_SMART_MAGNIFY:
      return PropertyCode.SmartMagnifyListener;
    case EVENT_PRESSURE:
      return PropertyCode.PressureListener;
    case EVENT_FOCUS:
    case EVENT_BLUR:
      return PropertyCode.FocusListener;
    case EVENT_ACTION:
      return PropertyCode.ActionListener;
    case EVENT_DRAG_START:
    case EVENT_DRAG_END:
      return PropertyCode.DragListener;
    case EVENT_DROP:
    case EVENT_FILES_DROPPED:
      return PropertyCode.DropListener;
    case EVENT_COMPONENT_CHANGE:
      return PropertyCode.ComponentChangeListener;
    case EVENT_COMMIT:
      return PropertyCode.CommitListener;
    default:
      return 0;
  }
}

/** Event types that share one declared-listener property. */
function sharesListenerProperty(type: number, other: number): boolean {
  return listenerPropertyFor(type) === listenerPropertyFor(other);
}

export type NativeEventListener = (event: QuickGuiEvent) => void;

export class NativeListener {
  readonly type: number;
  readonly listener: NativeEventListener;

  constructor(type: number, listener: NativeEventListener) {
    this.type = type;
    this.listener = listener;
  }
}

export class QuickGuiEvent {
  readonly type: number;
  readonly target: NativeNode;
  currentTarget: NativeNode;
  defaultPrevented = false;
  propagationStopped = false;
  /** The bounded payload the host attached, usually JSON, or `undefined`. */
  readonly value: string | undefined;

  constructor(type: number, target: NativeNode, value: string | undefined) {
    this.type = type;
    this.target = target;
    this.currentTarget = target;
    this.value = value;
  }

  preventDefault(): void {
    this.defaultPrevented = true;
  }

  stopPropagation(): void {
    this.propagationStopped = true;
  }
}

let nextNodeId = 1;

function allocateNodeId(): number {
  if (nextNodeId >= 0xffff_ffff) throw new Error("QuickGUI native node id space exhausted");
  const id = nextNodeId;
  nextNodeId += 1;
  return id;
}

/**
 * One window's mutation sink. `Window` extends this so nodes never depend on window lifecycle.
 */
export class NodeHost {
  readonly appId: number;
  readonly nativeId: number;
  readonly nodes = new Map<number, NativeNode>();
  batch = new MutationBatch();
  flushScheduled = false;
  closed = false;
  /** Whether the native window exists; earlier mutations accumulate into the initial batch. */
  nativeReady = false;

  constructor(appId: number, nativeId: number) {
    this.appId = appId;
    this.nativeId = nativeId;
  }

  scheduleFlush(): void {
    if (this.flushScheduled || this.closed || !this.nativeReady) return;
    this.flushScheduled = true;
    queueMicrotask(() => {
      this.flushScheduled = false;
      if (!this.closed) this.flush();
    });
  }

  /** Commit every recorded mutation as one bounded batch. */
  flush(): void {
    if (this.closed || !this.nativeReady) return;
    this.flushScheduled = false;
    if (this.batch.empty) return;
    const bytes = this.batch.finish();
    this.batch = new MutationBatch();
    hostApplyBatch(this.appId, this.nativeId, bytes);
  }

  /** Take the recorded mutations without sending them, for a window's initial batch. */
  takeBatch(): Uint8Array {
    this.flushScheduled = false;
    const batch = this.batch;
    this.batch = new MutationBatch();
    return batch.finish();
  }

  focusNode(node: NativeNode): boolean {
    if (this.closed || node.host !== this) return false;
    this.flush();
    hostFocusNode(this.appId, this.nativeId, node.id);
    return true;
  }
}

export class NativeNode {
  readonly id: number;
  readonly tag: number;
  text: string;
  parent: NativeNode | undefined = undefined;
  readonly children: NativeNode[] = [];
  /** The window this node is mounted in, when mounted. */
  host: NodeHost | undefined = undefined;
  /** Mutations recorded while this node roots a detached subtree. */
  pending: MutationBatch | undefined = undefined;
  listeners: NativeListener[] | undefined = undefined;
  /**
   * Nodes a sentinel keeps immediately before itself: a fragment's members or a reactive
   * region's current content. They move, mount, and unmount together with the sentinel.
   */
  group: NativeNode[] | undefined = undefined;
  /** Set once the host removed this node; a removed node cannot be inserted again. */
  removed = false;

  constructor(tag: number, text: string, id: number) {
    this.id = id;
    this.tag = tag;
    this.text = text;
  }

  /** Focus this mounted node, matching the web `HTMLElement.focus()` shape. */
  focus(): boolean {
    const host = this.host;
    return host === undefined ? false : host.focusNode(this);
  }
}

/** The batch a mutation on `node` belongs to: its window's, or the detached root's buffer. */
function batchFor(node: NativeNode): MutationBatch {
  let current: NativeNode | undefined = node;
  while (current !== undefined) {
    const host = current.host;
    if (host !== undefined) {
      host.scheduleFlush();
      return host.batch;
    }
    const pending = current.pending;
    if (pending !== undefined) return pending;
    current = current.parent;
  }
  throw new Error("a removed QuickGUI node cannot be mutated");
}

export function createNativeElement(tag: number): NativeNode {
  const node = new NativeNode(tag, "", allocateNodeId());
  const pending = new MutationBatch();
  pending.createElement(node.id, tag);
  node.pending = pending;
  return node;
}

export function createNativeText(value: string): NativeNode {
  const node = new NativeNode(NativeNodeTag.Text, value, allocateNodeId());
  const pending = new MutationBatch();
  pending.createText(node.id, value);
  node.pending = pending;
  return node;
}

export function createNativeSentinel(): NativeNode {
  const node = new NativeNode(NativeNodeTag.Sentinel, "", allocateNodeId());
  const pending = new MutationBatch();
  pending.createSentinel(node.id);
  node.pending = pending;
  return node;
}

/** The window root node: mounted from the start and never recorded as a creation. */
export function createRootNode(host: NodeHost, id: number): NativeNode {
  const node = new NativeNode(NativeNodeTag.View, "", id);
  node.host = host;
  host.nodes.set(id, node);
  return node;
}

export function replaceNativeText(node: NativeNode, value: string): void {
  if (node.tag !== NativeNodeTag.Text) throw new TypeError("replaceText expects a text node");
  if (node.text === value) return;
  node.text = value;
  batchFor(node).replaceText(node.id, value);
}

export function setNativeProperty(node: NativeNode, property: number, value: NativePropertyValue, color: boolean): void {
  batchFor(node).setProperty(node.id, property, value, color);
}

export function setNativeString(node: NativeNode, property: number, value: string): void {
  batchFor(node).setString(node.id, property, value);
}

export function setNativeNumber(node: NativeNode, property: number, value: number): void {
  batchFor(node).setNumber(node.id, property, value);
}

export function setNativeBoolean(node: NativeNode, property: number, value: boolean): void {
  batchFor(node).setBoolean(node.id, property, value);
}

export function setNativeColor(node: NativeNode, property: number, value: number): void {
  batchFor(node).setColor(node.id, property, value);
}

export function clearNativeProperty(node: NativeNode, property: number): void {
  batchFor(node).clearProperty(node.id, property);
}

function hasListenerSharing(node: NativeNode, type: number): boolean {
  const listeners = node.listeners;
  if (listeners === undefined) return false;
  for (const entry of listeners) {
    if (sharesListenerProperty(entry.type, type)) return true;
  }
  return false;
}

/** Attach, replace, or remove the listener for one event type and declare it to the host. */
export function setNativeEventListener(node: NativeNode, type: number, listener: NativeEventListener | undefined): void {
  let listeners = node.listeners;
  if (listeners === undefined) {
    if (listener === undefined) return;
    listeners = [];
    node.listeners = listeners;
  }
  let index = -1;
  for (let i = 0; i < listeners.length; i++) {
    if (listeners[i]!.type === type) {
      index = i;
      break;
    }
  }
  if (listener === undefined) {
    if (index >= 0) listeners.splice(index, 1);
  } else if (index >= 0) {
    listeners[index] = new NativeListener(type, listener);
  } else {
    listeners.push(new NativeListener(type, listener));
  }
  const property = listenerPropertyFor(type);
  if (property === 0) return;
  setNativeBoolean(node, property, hasListenerSharing(node, type));
}

function materialize(node: NativeNode, host: NodeHost): void {
  node.host = host;
  host.nodes.set(node.id, node);
  for (const child of node.children) materialize(child, host);
  const group = node.group;
  if (group !== undefined) {
    // Group members are siblings that were inserted alongside the sentinel already.
  }
}

function dematerialize(node: NativeNode): void {
  const host = node.host;
  if (host !== undefined) host.nodes.delete(node.id);
  node.host = undefined;
  for (const child of node.children) dematerialize(child);
}

/** Mark a subtree removed: it can no longer be mounted, and its links are released. */
function retire(node: NativeNode): void {
  node.removed = true;
  node.pending = undefined;
  node.listeners = undefined;
  for (const child of node.children) {
    child.parent = undefined;
    retire(child);
  }
  node.children.splice(0, node.children.length);
  node.group = undefined;
}

/** Insert `item` at `index`, shifting later entries right. */
function insertAt(array: NativeNode[], index: number, item: NativeNode): void {
  array.push(item);
  let position = array.length - 1;
  while (position > index) {
    array[position] = array[position - 1]!;
    position -= 1;
  }
  array[index] = item;
}

function indexOfChild(parent: NativeNode, node: NativeNode): number {
  const children = parent.children;
  for (let i = 0; i < children.length; i++) {
    if (children[i] === node) return i;
  }
  return -1;
}

/**
 * Insert `node` into `parent` before `anchor`, or at the end.
 *
 * A node already in the tree moves; a detached subtree's recorded mutations are spliced in
 * first, so the host sees creation, properties, and insertion in order.
 */
export function insertNativeNode(parent: NativeNode, node: NativeNode, anchor: NativeNode | undefined): void {
  if (node.removed) throw new Error("a removed QuickGUI node cannot be inserted again");
  if (node === parent) throw new Error("a native node cannot contain itself");
  if (anchor !== undefined && anchor.parent !== parent) throw new Error("anchor is not a child of parent");
  if (anchor === node && node.parent === parent) return;
  const group = node.group;
  if (group !== undefined) {
    for (const member of group) insertNativeNode(parent, member, anchor);
  }
  const previousParent = node.parent;
  if (previousParent !== undefined) {
    const previousIndex = indexOfChild(previousParent, node);
    if (previousIndex >= 0) previousParent.children.splice(previousIndex, 1);
  }
  const index = anchor === undefined ? parent.children.length : indexOfChild(parent, anchor);
  insertAt(parent.children, index < 0 ? parent.children.length : index, node);
  node.parent = parent;
  const batch = batchFor(parent);
  const pending = node.pending;
  if (pending !== undefined) {
    batch.append(pending);
    node.pending = undefined;
  }
  batch.insert(parent.id, node.id, anchor === undefined ? NO_ANCHOR : anchor.id);
  const host = parent.host;
  if (host !== undefined && node.host !== host) {
    if (node.host !== undefined) throw new Error("a native node cannot move between QuickGUI windows");
    materialize(node, host);
  }
}

/** Remove one child subtree; the host releases it and the node cannot be reused. */
export function removeNativeNode(parent: NativeNode, node: NativeNode): void {
  if (node.parent !== parent) return;
  const group = node.group;
  if (group !== undefined) {
    for (const member of group) removeNativeNode(parent, member);
  }
  const index = indexOfChild(parent, node);
  if (index >= 0) parent.children.splice(index, 1);
  node.parent = undefined;
  batchFor(parent).remove(parent.id, node.id);
  dematerialize(node);
  retire(node);
}

/** Remove every listed child in one mutation. */
export function cleanupNativeNodes(parent: NativeNode, nodes: readonly NativeNode[]): void {
  const attached: NativeNode[] = [];
  for (const node of nodes) {
    if (node.parent === parent) attached.push(node);
  }
  if (attached.length === 0) return;
  const ids: number[] = [];
  for (const node of attached) {
    const group = node.group;
    if (group !== undefined) {
      for (const member of group) removeNativeNode(parent, member);
    }
    const index = indexOfChild(parent, node);
    if (index >= 0) parent.children.splice(index, 1);
    node.parent = undefined;
    ids.push(node.id);
  }
  batchFor(parent).cleanup(parent.id, ids);
  for (const node of attached) {
    dematerialize(node);
    retire(node);
  }
}

export function getNativeParent(node: NativeNode): NativeNode | undefined {
  return node.parent;
}

export function getNativeFirstChild(node: NativeNode): NativeNode | undefined {
  return node.children.length > 0 ? node.children[0] : undefined;
}

export function getNativeNextSibling(node: NativeNode): NativeNode | undefined {
  const parent = node.parent;
  if (parent === undefined) return undefined;
  const index = indexOfChild(parent, node);
  if (index < 0 || index + 1 >= parent.children.length) return undefined;
  return parent.children[index + 1];
}

export function isNativeText(node: NativeNode): boolean {
  return node.tag === NativeNodeTag.Text;
}

/** Deliver one host event to a mounted node, bubbling through its ancestors. */
export function dispatchNativeEvent(host: NodeHost, type: number, targetId: number, value: string | undefined): void {
  const target = host.nodes.get(targetId);
  if (target === undefined) return;
  const event = new QuickGuiEvent(type, target, value);
  if (type === EVENT_MOUSE_ENTER || type === EVENT_MOUSE_LEAVE) {
    invokeListeners(target, event);
    return;
  }
  let current: NativeNode | undefined = target;
  while (current !== undefined) {
    event.currentTarget = current;
    invokeListeners(current, event);
    if (event.propagationStopped) break;
    current = current.parent;
  }
}

function invokeListeners(node: NativeNode, event: QuickGuiEvent): void {
  const listeners = node.listeners;
  if (listeners === undefined) return;
  for (const entry of listeners) {
    if (entry.type === event.type) {
      entry.listener(event);
      return;
    }
  }
}

// ---------------------------------------------------------------------------
// Colors
// ---------------------------------------------------------------------------

export type ColorValue = number | string;

function hexDigit(code: number): number {
  if (code >= 48 && code <= 57) return code - 48;
  if (code >= 97 && code <= 102) return code - 87;
  if (code >= 65 && code <= 70) return code - 55;
  return -1;
}

function hexByte(text: string, index: number, doubled: boolean): number {
  const high = hexDigit(text.charCodeAt(index));
  const low = doubled ? high : hexDigit(text.charCodeAt(index + 1));
  if (high < 0 || low < 0) return -1;
  return high * 16 + low;
}

export function packColor(r: number, g: number, b: number, a: number): number {
  const component = (value: number): number => Math.max(0, Math.min(255, Math.round(value)));
  return (component(r) | (component(g) << 8) | (component(b) << 16) | (component(a) << 24)) >>> 0;
}

/** Parse a packed RGBA integer, `#rgb[a]`, `#rrggbb[aa]`, `rgb()`, `rgba()`, or a keyword. */
export function parseColor(value: ColorValue): number {
  if (typeof value === "number") return value >>> 0;
  const color = value.trim().toLowerCase();
  if (color === "transparent") return 0;
  if (color === "black") return packColor(0, 0, 0, 255);
  if (color === "white") return packColor(255, 255, 255, 255);
  if (color.startsWith("#")) {
    const hex = color.slice(1);
    if (hex.length === 3 || hex.length === 4) {
      const r = hexByte(hex, 0, true);
      const g = hexByte(hex, 1, true);
      const b = hexByte(hex, 2, true);
      const a = hex.length === 4 ? hexByte(hex, 3, true) : 255;
      if (r >= 0 && g >= 0 && b >= 0 && a >= 0) return packColor(r, g, b, a);
    } else if (hex.length === 6 || hex.length === 8) {
      const r = hexByte(hex, 0, false);
      const g = hexByte(hex, 2, false);
      const b = hexByte(hex, 4, false);
      const a = hex.length === 8 ? hexByte(hex, 6, false) : 255;
      if (r >= 0 && g >= 0 && b >= 0 && a >= 0) return packColor(r, g, b, a);
    }
  } else if ((color.startsWith("rgba(") || color.startsWith("rgb(")) && color.endsWith(")")) {
    const inner = color.slice(color.indexOf("(") + 1, color.length - 1);
    const parts = inner.split(",");
    if (parts.length === 3 || parts.length === 4) {
      const r = Number(parts[0]!.trim());
      const g = Number(parts[1]!.trim());
      const b = Number(parts[2]!.trim());
      const a = parts.length === 4 ? Math.round(Number(parts[3]!.trim()) * 255) : 255;
      if (Number.isFinite(r) && Number.isFinite(g) && Number.isFinite(b) && Number.isFinite(a)) {
        return packColor(r, g, b, a);
      }
    }
  }
  throw new TypeError("unsupported QuickGUI color `" + value + "`");
}
