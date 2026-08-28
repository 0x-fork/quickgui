import * as binding from "./binding.js";
import {
  MutationBatch,
  NativeNodeTag,
  PropertyCode,
  PROTOCOL_VERSION,
  ROOT_NODE_ID,
  type NativePropertyValue,
} from "./protocol.ts";

export { PropertyCode } from "./protocol.ts";

export type ColorValue = number | string;
export type NativeElementName =
  | "view"
  | "div"
  | "text"
  | "button"
  | "input"
  | "textarea"
  | "markdown";
export type NativeEventType =
  | "click"
  | "mouseenter"
  | "mouseleave"
  | "input"
  | "submit";
export type NativeEventListener = (event: QuickGuiEvent) => void;
export type WindowCloseListener = (window: Window) => void;
export type PopupPlacement =
  | "top-start"
  | "top"
  | "top-end"
  | "bottom-start"
  | "bottom"
  | "bottom-end"
  | "left-start"
  | "left"
  | "left-end"
  | "right-start"
  | "right"
  | "right-end";

export interface WindowOptions {
  title?: string;
  width?: number;
  height?: number;
  minimumWidth?: number;
  minimumHeight?: number;
  background?: ColorValue;
  titleBarStyle?: "default" | "hidden" | "hiddenInset";
  trafficLightPosition?: { x: number; y: number };
  transparent?: boolean;
  blur?: boolean;
  /** Open this window as a native child popup anchored to the mounted node. */
  anchor?: NativeNode;
  placement?: PopupPlacement;
  gap?: number;
  offset?: { x: number; y: number };
  grab?: boolean;
  acceptsKeyFocus?: boolean;
}

export interface RunOptions {
  /** Pump interval used only when running a source file outside the QuickGUI CLI host. */
  sliceMs?: number;
}

let nextNodeId = 1;
let activeApp: App | undefined;
const hostedRuntime = process.env.QUICKGUI_APP_WORKER === "1";

const nativeProtocolVersion = binding.protocolVersion();
if (nativeProtocolVersion !== PROTOCOL_VERSION) {
  throw new Error(
    `QuickGUI native protocol mismatch: JavaScript uses ${PROTOCOL_VERSION}, binding uses ${nativeProtocolVersion}. Reinstall or rebuild @quickgui/native.`,
  );
}

export class NativeNode {
  readonly id: number;
  readonly tag: NativeNodeTag;
  text: string;
  parent: NativeNode | undefined;
  readonly children: NativeNode[] = [];
  readonly properties = new Map<PropertyCode, NativePropertyValue>();
  readonly colorProperties = new Set<PropertyCode>();
  readonly listeners = new Map<NativeEventType, NativeEventListener>();
  host: Window | undefined;
  materialized = false;

  constructor(tag: NativeNodeTag, text = "", id = allocateNodeId()) {
    this.id = id;
    this.tag = tag;
    this.text = text;
  }

  /** Focus this mounted node, matching the web `HTMLElement.focus()` shape. */
  focus(): boolean {
    const host = this.host;
    if (!host || host.closed) return false;
    host.flush();
    return hostedRuntime
      ? binding.focusHostedNode(host.app.nativeId, host.nativeId, this.id)
      : binding.focusNode(host.app.nativeId, host.nativeId, this.id);
  }
}

export class QuickGuiEvent {
  readonly type: NativeEventType;
  readonly target: NativeNode;
  currentTarget: NativeNode;
  defaultPrevented = false;
  propagationStopped = false;
  readonly value: string | undefined;

  constructor(type: NativeEventType, target: NativeNode, value?: string) {
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

export class App {
  readonly nativeId: number;
  readonly windows = new Map<number, Window>();
  #started = false;
  #running = false;
  #destroyed = false;

  constructor() {
    if (activeApp) {
      throw new Error("a QuickGUI App is already active in this JavaScript isolate");
    }
    this.nativeId = hostedRuntime ? binding.createHostedApp() : binding.createApp();
    activeApp = this;
  }

  flush(): void {
    this.#assertAlive();
    for (const window of this.windows.values()) window.flush();
  }

  start(): void {
    this.#assertAlive();
    if (this.#started) return;
    this.flush();
    if (hostedRuntime) binding.startHostedApp(this.nativeId);
    else binding.startApp(this.nativeId);
    this.#started = true;
  }

  pump(sliceMs = 16): number {
    this.start();
    return this.#finishPump(binding.pumpApp(this.nativeId, sliceMs));
  }

  #finishPump(exitCode: number): number {
    this.dispatchEvents();
    this.flush();
    return exitCode;
  }

  dispatchEvents(): void {
    this.#dispatchNativeEvents(binding.takeEvents(this.nativeId));
  }

  #dispatchNativeEvents(events: binding.NativeEvent[]): void {
    for (const event of events) {
      const window = this.windows.get(event.window);
      if (!window) continue;
      if (event.kind === "close") {
        this._didCloseWindow(window);
      } else {
        window._dispatchEvent(event.kind as NativeEventType, event.target, event.value);
      }
    }
  }

  async run(options: RunOptions = {}): Promise<number> {
    if (this.#running) throw new Error("this QuickGUI app is already running");
    this.#running = true;
    try {
      this.start();
      if (hostedRuntime) {
        for (;;) {
          const update = await binding.waitForHostedEvents(this.nativeId);
          this.#dispatchNativeEvents(update.events);
          this.flush();
          if (update.exitCode !== undefined && update.exitCode !== null) {
            return update.exitCode;
          }
        }
      }
      const sliceMs = Math.max(0, Math.min(options.sliceMs ?? 16, 1_000));
      for (;;) {
        const exitCode = this.pump(sliceMs);
        if (exitCode >= 0) return exitCode;
        await Bun.sleep(0);
      }
    } finally {
      this.#running = false;
    }
  }

  destroy(): void {
    if (this.#destroyed) return;
    if (hostedRuntime) binding.destroyHostedApp(this.nativeId);
    else binding.destroyApp(this.nativeId);
    this.#destroyed = true;
    if (activeApp === this) activeApp = undefined;
    for (const window of this.windows.values()) window._didDestroy();
    this.windows.clear();
  }

  _registerWindow(window: Window): void {
    this.#assertAlive();
    this.windows.set(window.nativeId, window);
  }

  _closeWindow(window: Window): void {
    if (this.#destroyed || window.closed) return;
    const closed = hostedRuntime
      ? binding.closeHostedWindow(this.nativeId, window.nativeId)
      : binding.closeWindow(this.nativeId, window.nativeId);
    if (closed) this._didCloseWindow(window);
  }

  _didCloseWindow(window: Window): void {
    if (this.windows.get(window.nativeId) !== window) return;
    this.windows.delete(window.nativeId);
    window._didClose();
  }

  #assertAlive(): void {
    if (this.#destroyed) throw new Error("this QuickGUI app has been destroyed");
  }
}

export class Window {
  readonly root: NativeNode;
  readonly nativeId: number;
  readonly app: App;
  readonly nodes = new Map<number, NativeNode>();
  #batch = new MutationBatch();
  #flushScheduled = false;
  #closed = false;
  readonly #closeListeners = new Set<WindowCloseListener>();
  readonly #mountDisposers = new Set<() => void>();

  constructor(options: WindowOptions = {}) {
    const app = activeApp;
    if (!app) throw new Error("create a QuickGUI App before creating a Window");
    const nativeOptions: binding.NativeWindowOptions = {};
    if (options.title !== undefined) nativeOptions.title = options.title;
    if (options.width !== undefined) nativeOptions.width = options.width;
    if (options.height !== undefined) nativeOptions.height = options.height;
    if (options.minimumWidth !== undefined) nativeOptions.minimumWidth = options.minimumWidth;
    if (options.minimumHeight !== undefined) nativeOptions.minimumHeight = options.minimumHeight;
    if (options.background !== undefined) nativeOptions.background = parseColor(options.background);
    if (options.titleBarStyle !== undefined) nativeOptions.titleBarStyle = options.titleBarStyle;
    if (options.trafficLightPosition !== undefined) {
      nativeOptions.trafficLightX = options.trafficLightPosition.x;
      nativeOptions.trafficLightY = options.trafficLightPosition.y;
    }
    if (options.transparent !== undefined) nativeOptions.transparent = options.transparent;
    if (options.blur !== undefined) nativeOptions.blur = options.blur;
    if (options.placement !== undefined) nativeOptions.popupPlacement = options.placement;
    if (options.gap !== undefined) nativeOptions.popupGap = options.gap;
    if (options.offset !== undefined) {
      nativeOptions.popupOffsetX = options.offset.x;
      nativeOptions.popupOffsetY = options.offset.y;
    }
    if (options.grab !== undefined) nativeOptions.popupGrab = options.grab;
    if (options.acceptsKeyFocus !== undefined) {
      nativeOptions.popupAcceptsKeyFocus = options.acceptsKeyFocus;
    }
    this.app = app;
    if (options.anchor) {
      const parent = options.anchor.host;
      if (!parent || parent.closed || !options.anchor.materialized) {
        throw new Error("an anchored Window requires a mounted node in an open parent Window");
      }
      parent.flush();
      this.nativeId = hostedRuntime
        ? binding.createHostedAnchoredWindow(
            app.nativeId,
            parent.nativeId,
            options.anchor.id,
            nativeOptions,
          )
        : binding.createAnchoredWindow(
            app.nativeId,
            parent.nativeId,
            options.anchor.id,
            nativeOptions,
          );
    } else {
      this.nativeId = hostedRuntime
        ? binding.createHostedWindow(app.nativeId, nativeOptions)
        : binding.createWindow(app.nativeId, nativeOptions);
    }
    this.root = new NativeNode(NativeNodeTag.View, "", ROOT_NODE_ID);
    this.root.host = this;
    this.root.materialized = true;
    this.nodes.set(ROOT_NODE_ID, this.root);
    app._registerWindow(this);
  }

  get closed(): boolean {
    return this.#closed;
  }

  onClose(listener: WindowCloseListener): () => void {
    if (this.#closed) {
      listener(this);
      return () => {};
    }
    this.#closeListeners.add(listener);
    return () => this.#closeListeners.delete(listener);
  }

  close(): void {
    this.app._closeWindow(this);
  }

  flush(): number | undefined {
    if (this.#closed) return undefined;
    this.#flushScheduled = false;
    if (this.#batch.empty) return undefined;
    const batch = this.#batch;
    this.#batch = new MutationBatch();
    const bytes = batch.finish();
    return hostedRuntime
      ? binding.applyHostedBatch(this.app.nativeId, this.nativeId, bytes)
      : binding.applyBatch(this.app.nativeId, this.nativeId, bytes);
  }

  _dispatchEvent(type: NativeEventType, targetId: number, value?: string): void {
    const target = this.nodes.get(targetId);
    if (!target) return;
    const quickGuiEvent = new QuickGuiEvent(type, target, value);
    if (type === "mouseenter" || type === "mouseleave") {
      target.listeners.get(type)?.(quickGuiEvent);
      return;
    }
    let current: NativeNode | undefined = target;
    while (current) {
      quickGuiEvent.currentTarget = current;
      current.listeners.get(type)?.(quickGuiEvent);
      if (quickGuiEvent.propagationStopped) break;
      current = current.parent;
    }
  }

  _didClose(): void {
    if (this.#closed) return;
    this.#closed = true;
    this.#flushScheduled = false;
    const disposers = [...this.#mountDisposers];
    this.#mountDisposers.clear();
    for (const dispose of disposers) dispose();
    this.#batch = new MutationBatch();
    for (const listener of this.#closeListeners) listener(this);
    this.#closeListeners.clear();
    this.nodes.clear();
  }

  _didDestroy(): void {
    this._didClose();
  }

  _trackMount(dispose: () => void): () => void {
    if (this.#closed) {
      dispose();
      return () => {};
    }
    this.#mountDisposers.add(dispose);
    return () => this.#mountDisposers.delete(dispose);
  }

  _enqueueCreate(node: NativeNode): void {
    switch (node.tag) {
      case NativeNodeTag.Text:
        this.#batch.createText(node.id, node.text);
        break;
      case NativeNodeTag.Sentinel:
        this.#batch.createSentinel(node.id);
        break;
      default:
        this.#batch.createElement(node.id, node.tag);
    }
  }

  _enqueueProperty(
    node: NativeNode,
    property: PropertyCode,
    value: NativePropertyValue,
    color: boolean,
  ): void {
    this.#batch.setProperty(node.id, property, value, color);
    this._scheduleFlush();
  }

  _enqueueText(node: NativeNode): void {
    this.#batch.replaceText(node.id, node.text);
    this._scheduleFlush();
  }

  _enqueueInsert(parent: NativeNode, child: NativeNode, before?: NativeNode): void {
    this.#batch.insert(parent.id, child.id, before?.id);
    this._scheduleFlush();
  }

  _enqueueRemove(parent: NativeNode, child: NativeNode): void {
    this.#batch.remove(parent.id, child.id);
    this._scheduleFlush();
  }

  _enqueueCleanup(parent: NativeNode, children: readonly NativeNode[]): void {
    this.#batch.cleanup(
      parent.id,
      children.map((child) => child.id),
    );
    this._scheduleFlush();
  }

  _scheduleFlush(): void {
    if (this.#flushScheduled || this.#closed) return;
    this.#flushScheduled = true;
    queueMicrotask(() => {
      if (!this.#closed) this.flush();
    });
  }
}

export function createNativeElement(name: NativeElementName): NativeNode {
  const tag =
    name === "button"
      ? NativeNodeTag.Button
      : name === "input" || name === "textarea"
        ? NativeNodeTag.Input
        : name === "markdown"
          ? NativeNodeTag.Markdown
          : NativeNodeTag.View;
  const node = new NativeNode(tag);
  if (name === "textarea") setNativeProperty(node, PropertyCode.Multiline, true);
  return node;
}

export function createNativeText(value: string): NativeNode {
  return new NativeNode(NativeNodeTag.Text, value);
}

export function createNativeSentinel(): NativeNode {
  return new NativeNode(NativeNodeTag.Sentinel);
}

export function replaceNativeText(node: NativeNode, value: string): void {
  if (node.tag !== NativeNodeTag.Text) throw new TypeError("replaceText expects a text node");
  if (node.text === value) return;
  node.text = value;
  if (node.materialized) node.host?._enqueueText(node);
}

export function setNativeProperty(
  node: NativeNode,
  property: PropertyCode,
  value: NativePropertyValue,
  options: { color?: boolean } = {},
): void {
  const normalized = value ?? null;
  if (normalized === null) {
    if (!node.properties.delete(property)) return;
    node.colorProperties.delete(property);
  } else {
    const previous = node.properties.get(property);
    if (Object.is(previous, normalized) && node.colorProperties.has(property) === !!options.color) {
      return;
    }
    node.properties.set(property, normalized);
    if (options.color) node.colorProperties.add(property);
    else node.colorProperties.delete(property);
  }
  if (node.materialized) {
    node.host?._enqueueProperty(node, property, normalized, !!options.color);
  }
}

export function setNativeEventListener(
  node: NativeNode,
  type: NativeEventType,
  listener: NativeEventListener | undefined,
): void {
  if (listener) node.listeners.set(type, listener);
  else node.listeners.delete(type);
  if (type === "click") {
    setNativeProperty(node, PropertyCode.ClickListener, node.listeners.has("click"));
  } else if (type === "input") {
    setNativeProperty(node, PropertyCode.InputListener, node.listeners.has("input"));
  } else if (type === "submit") {
    setNativeProperty(node, PropertyCode.SubmitListener, node.listeners.has("submit"));
  } else {
    const listensForHover = node.listeners.has("mouseenter") || node.listeners.has("mouseleave");
    setNativeProperty(node, PropertyCode.HoverListener, listensForHover);
  }
}

export function insertNativeNode(parent: NativeNode, node: NativeNode, anchor?: NativeNode): void {
  if (anchor && anchor.parent !== parent) throw new Error("anchor is not a child of parent");
  if (node === parent) throw new Error("a native node cannot contain itself");
  if (anchor === node && node.parent === parent) return;

  if (node.parent) {
    const previousIndex = node.parent.children.indexOf(node);
    if (previousIndex >= 0) node.parent.children.splice(previousIndex, 1);
  }
  const index = anchor ? parent.children.indexOf(anchor) : parent.children.length;
  parent.children.splice(index, 0, node);
  node.parent = parent;

  if (parent.host) {
    materialize(node, parent.host);
    parent.host._enqueueInsert(parent, node, anchor);
  }
}

export function removeNativeNode(parent: NativeNode, node: NativeNode): void {
  if (node.parent !== parent) return;
  const index = parent.children.indexOf(node);
  if (index >= 0) parent.children.splice(index, 1);
  node.parent = undefined;
  if (node.materialized && parent.host) parent.host._enqueueRemove(parent, node);
  dematerialize(node);
}

export function cleanupNativeNodes(parent: NativeNode, nodes: readonly NativeNode[]): void {
  const attached = nodes.filter((node) => node.parent === parent);
  if (attached.length === 0) return;
  for (const node of attached) {
    const index = parent.children.indexOf(node);
    if (index >= 0) parent.children.splice(index, 1);
    node.parent = undefined;
  }
  if (parent.host) parent.host._enqueueCleanup(parent, attached);
  for (const node of attached) dematerialize(node);
}

export function getNativeParent(node: NativeNode): NativeNode | undefined {
  return node.parent;
}

export function getNativeFirstChild(node: NativeNode): NativeNode | undefined {
  return node.children[0];
}

export function getNativeNextSibling(node: NativeNode): NativeNode | undefined {
  if (!node.parent) return undefined;
  const index = node.parent.children.indexOf(node);
  return index < 0 ? undefined : node.parent.children[index + 1];
}

export function isNativeText(node: NativeNode): boolean {
  return node.tag === NativeNodeTag.Text;
}

export function parseColor(value: ColorValue): number {
  if (typeof value === "number") return value >>> 0;
  const color = value.trim().toLowerCase();
  if (color === "transparent") return 0;
  if (color === "black") return packColor(0, 0, 0, 255);
  if (color === "white") return packColor(255, 255, 255, 255);
  if (color.startsWith("#")) {
    const hex = color.slice(1);
    if (hex.length === 3 || hex.length === 4) {
      const [r = "0", g = "0", b = "0", a = "f"] = hex;
      return packColor(
        Number.parseInt(r + r, 16),
        Number.parseInt(g + g, 16),
        Number.parseInt(b + b, 16),
        Number.parseInt(a + a, 16),
      );
    }
    if (hex.length === 6 || hex.length === 8) {
      return packColor(
        Number.parseInt(hex.slice(0, 2), 16),
        Number.parseInt(hex.slice(2, 4), 16),
        Number.parseInt(hex.slice(4, 6), 16),
        hex.length === 8 ? Number.parseInt(hex.slice(6, 8), 16) : 255,
      );
    }
  }
  const rgb = color.match(/^rgba?\(([^)]+)\)$/);
  if (rgb) {
    const parts = rgb[1]?.split(",").map((part) => part.trim()) ?? [];
    if (parts.length === 3 || parts.length === 4) {
      return packColor(
        Number(parts[0]),
        Number(parts[1]),
        Number(parts[2]),
        parts[3] === undefined ? 255 : Math.round(Number(parts[3]) * 255),
      );
    }
  }
  throw new TypeError(`unsupported QuickGUI color \`${value}\``);
}

function packColor(r: number, g: number, b: number, a: number): number {
  const component = (value: number) => Math.max(0, Math.min(255, Math.round(value)));
  return (
    component(r) |
    (component(g) << 8) |
    (component(b) << 16) |
    (component(a) << 24)
  ) >>> 0;
}

function allocateNodeId(): number {
  if (nextNodeId >= 0xffff_ffff) throw new Error("QuickGUI native node id space exhausted");
  return nextNodeId++;
}

function materialize(node: NativeNode, host: Window): void {
  if (node.materialized) {
    if (node.host !== host) throw new Error("a native node cannot move between QuickGUI windows");
    return;
  }
  node.host = host;
  node.materialized = true;
  host.nodes.set(node.id, node);
  host._enqueueCreate(node);
  for (const [property, value] of node.properties) {
    host._enqueueProperty(node, property, value, node.colorProperties.has(property));
  }
  for (const child of node.children) {
    materialize(child, host);
    host._enqueueInsert(node, child);
  }
}

function dematerialize(node: NativeNode): void {
  const host = node.host;
  if (host) host.nodes.delete(node.id);
  node.materialized = false;
  node.host = undefined;
  for (const child of node.children) dematerialize(child);
}
