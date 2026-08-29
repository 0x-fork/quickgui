import * as binding from "./binding.js";
import {
  type AlertDialogOptions,
  type OpenDialogOptions,
  type OpenDialogResult,
  type SaveDialogOptions,
  type SaveDialogResult,
  normalizeAlertDialogOptions,
  normalizeOpenDialogOptions,
  normalizeSaveDialogOptions,
} from "./dialog.ts";
import {
  MutationBatch,
  NativeNodeTag,
  PropertyCode,
  PROTOCOL_VERSION,
  ROOT_NODE_ID,
  type NativePropertyValue,
} from "./protocol.ts";
import {
  NativeNode,
  QuickGuiEvent,
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
  type ColorValue,
  type NativeElementName,
  type NativeEventListener,
  type NativeEventType,
} from "./native-tree.ts";
import {
  configureSystemContext,
  dispatchSystemEvent,
  getNativeWindowState,
  onNativeWindowStateChange,
  performNativeWindowAction,
  rejectPendingSystemRequests,
  removeNativeWindowStateListeners,
} from "./system.ts";
import {
  type SecondInstanceEvent,
  urlsFromArguments,
} from "./single-instance.ts";

export { PropertyCode } from "./protocol.ts";
export {
  NativeNode,
  QuickGuiEvent,
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
} from "./native-tree.ts";
export type {
  ColorValue,
  NativeElementName,
  NativeEventListener,
  NativeEventType,
} from "./native-tree.ts";
export type {
  AlertDialogButton,
  AlertDialogButtonRole,
  AlertDialogLevel,
  AlertDialogOptions,
  FileDialogFilter,
  OpenDialogOptions,
  OpenDialogProperty,
  OpenDialogResult,
  SaveDialogOptions,
  SaveDialogResult,
} from "./dialog.ts";
export {
  Appearance,
  Clipboard,
  GlobalShortcut,
  Keyboard,
  Menu,
  Notifications,
  PowerMonitor,
  Screen,
  Shell,
  Tray,
  TrayIcon,
} from "./system.ts";
export { AutoStart, SecureStorage, Updater } from "./integrations.ts";
export { DeepLink } from "./single-instance.ts";
export type { SecondInstanceEvent } from "./single-instance.ts";
export type {
  AutoStartMode,
  AutoStartOptions,
  AvailableUpdate,
  ProtocolRegistrationOptions,
  UpdateClientOptions,
} from "./integrations.ts";
export type {
  AppearanceMode,
  AppearancePreference,
  ClipboardEntry,
  ClipboardFilesEntry,
  ClipboardImageEntry,
  ClipboardItem,
  ClipboardTextEntry,
  Display,
  GlobalShortcutListener,
  KeyboardLayout,
  MenuActionItem,
  MenuDefinition,
  MenuItem,
  MenuRole,
  MenuSeparatorItem,
  MenuServicesItem,
  MenuSubmenuItem,
  NotificationAction,
  NotificationOptions,
  NotificationResponse,
  PowerEvent,
  Rectangle,
  TrayEvent,
  TrayEventType,
  TrayIconOptions,
  TrayIconSource,
  TrayMenuActionItem,
  TrayMenuItem,
  TrayMenuSeparatorItem,
  TrayMenuSubmenuItem,
  WindowState,
} from "./system.ts";
import type {
  AppearancePreference,
  KeyboardLayout,
  NotificationResponse,
  WindowState,
} from "./system.ts";

export type WindowCloseListener = (window: Window) => void;
export type WindowRenderer = (window: Window) => () => void;
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
  renderer: WindowRenderer;
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

export interface AppEventMap {
  ready: undefined;
  quit: { exitCode: number };
  openUrls: readonly string[];
  reopen: { hasVisibleWindows: boolean };
  systemWake: undefined;
  keyboardLayoutChange: KeyboardLayout;
  notificationResponse: NotificationResponse;
  secondInstance: SecondInstanceEvent;
}

type DialogEventKind = "alert-dialog" | "open-dialog" | "save-dialog";

type PendingDialog = {
  kind: DialogEventKind;
  window: Window | undefined;
  complete: (event: binding.NativeEvent) => void;
  reject: (reason: Error) => void;
};

let activeApp: App | undefined;
let currentWindow: Window | undefined;
const hostedRuntime = process.env.QUICKGUI_APP_WORKER === "1";

function withCurrentWindow<T>(window: Window, callback: () => T): T {
  const previous = currentWindow;
  currentWindow = window;
  try {
    return callback();
  } finally {
    currentWindow = previous;
  }
}

configureSystemContext(
  () => {
    const app = activeApp;
    if (!app) throw new Error("create a QuickGUI App before using a native system API");
    app._assertReady();
    return { appId: app.nativeId, hosted: hostedRuntime };
  },
  (window) => {
    const app = window?.app ?? activeApp;
    if (!app) throw new Error("create a QuickGUI App before using a native window API");
    const resolved = window ?? app.windows.values().next().value;
    if (!resolved || resolved.closed || app.windows.get(resolved.nativeId) !== resolved) {
      throw new Error("a native window API requires an open QuickGUI Window");
    }
    app._assertReady();
    return {
      context: { appId: app.nativeId, hosted: hostedRuntime },
      window: resolved,
    };
  },
);

const nativeProtocolVersion = binding.protocolVersion();
if (nativeProtocolVersion !== PROTOCOL_VERSION) {
  throw new Error(
    `QuickGUI native protocol mismatch: JavaScript uses ${PROTOCOL_VERSION}, binding uses ${nativeProtocolVersion}. Reinstall or rebuild @quickgui/native.`,
  );
}

class App {
  readonly nativeId: number;
  readonly windows = new Map<number, Window>();
  #running = false;
  #destroyed = false;
  readonly #readyPromise: Promise<void>;
  #nextDialogRequest = 1;
  readonly #pendingDialogs = new Map<number, PendingDialog>();
  #singleInstanceIdentifier: string | undefined;
  readonly #appEventListeners = new Map<
    keyof AppEventMap,
    Set<(payload: unknown) => unknown>
  >();

  constructor() {
    if (activeApp) {
      throw new Error("a QuickGUI App is already active in this JavaScript isolate");
    }
    this.nativeId = hostedRuntime ? binding.createHostedApp() : binding.createApp();
    activeApp = this;
    this.#readyPromise = Promise.resolve().then(() => {
      this.#assertAlive();
      if (hostedRuntime) binding.prepareHostedApp(this.nativeId);
      else binding.prepareApp(this.nativeId);
      if (!this.isReady()) {
        throw new Error("the native QuickGUI application did not become ready");
      }
      this.#emitAppEvent("ready", undefined);
    });
  }

  isReady(): boolean {
    this.#assertAlive();
    return hostedRuntime
      ? binding.isHostedAppReady(this.nativeId)
      : binding.isAppReady(this.nativeId);
  }

  whenReady(): Promise<void> {
    return this.#readyPromise;
  }

  flush(): void {
    this.#assertAlive();
    for (const window of this.windows.values()) window.flush();
  }

  pump(sliceMs = 16): number {
    this._assertReady();
    this.flush();
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
      if (
        event.kind === "alert-dialog" ||
        event.kind === "open-dialog" ||
        event.kind === "save-dialog"
      ) {
        this.#dispatchDialog(event);
        continue;
      }
      const systemEvent = dispatchSystemEvent(event, (id) => this.windows.get(id));
      const appEvent = this.#dispatchAppEvent(event);
      if (systemEvent || appEvent) continue;
      const window = this.windows.get(event.window);
      if (!window) continue;
      if (event.kind === "close") {
        this._didCloseWindow(window);
      } else {
        window._dispatchEvent(event.kind as NativeEventType, event.target, event.value);
      }
    }
  }

  on<K extends keyof AppEventMap>(
    type: K,
    listener: (payload: AppEventMap[K]) => void,
  ): () => void {
    this.#assertAlive();
    const listeners = this.#appEventListeners.get(type) ?? new Set();
    const wrapped = (payload: unknown) => listener(payload as AppEventMap[K]);
    listeners.add(wrapped);
    this.#appEventListeners.set(type, listeners);
    return () => {
      listeners.delete(wrapped);
      if (listeners.size === 0) this.#appEventListeners.delete(type);
    };
  }

  async run(options: RunOptions = {}): Promise<number> {
    if (this.#running) throw new Error("this QuickGUI app is already running");
    this.#running = true;
    let exitCode: number | undefined;
    try {
      await this.whenReady();
      if (hostedRuntime) {
        for (;;) {
          const update = await binding.waitForHostedEvents(this.nativeId);
          this.#dispatchNativeEvents(update.events);
          this.flush();
          if (update.exitCode !== undefined && update.exitCode !== null) {
            exitCode = update.exitCode;
            break;
          }
        }
      } else {
        const sliceMs = Math.max(0, Math.min(options.sliceMs ?? 16, 1_000));
        for (;;) {
          const nextExitCode = this.pump(sliceMs);
          if (nextExitCode >= 0) {
            exitCode = nextExitCode;
            break;
          }
          await Bun.sleep(0);
        }
      }
      if (exitCode === undefined) throw new Error("the QuickGUI app exited without a status code");
      await this.#emitAppEventAndWait("quit", { exitCode });
      return exitCode;
    } finally {
      this.#running = false;
      this.releaseSingleInstanceLock();
    }
  }

  async requestSingleInstanceLock(identifier: string): Promise<boolean> {
    this.#assertAlive();
    if (this.#singleInstanceIdentifier) {
      if (this.#singleInstanceIdentifier !== identifier) {
        throw new Error("this QuickGUI app already owns a different single-instance lock");
      }
      return true;
    }
    this._assertReady();
    const acquired = hostedRuntime
      ? binding.requestHostedSingleInstanceLock(this.nativeId, identifier)
      : binding.requestSingleInstanceLock(this.nativeId, identifier);
    if (acquired) this.#singleInstanceIdentifier = identifier;
    return acquired;
  }

  releaseSingleInstanceLock(): void {
    if (!this.#singleInstanceIdentifier || this.#destroyed) return;
    if (hostedRuntime) binding.releaseHostedSingleInstanceLock(this.nativeId);
    else binding.releaseSingleInstanceLock(this.nativeId);
    this.#singleInstanceIdentifier = undefined;
  }

  /** Request an orderly native shutdown. Returns false after shutdown already began. */
  quit(): boolean {
    this.#assertAlive();
    this._assertReady();
    return hostedRuntime
      ? binding.exitHostedApp(this.nativeId)
      : binding.exitApp(this.nativeId);
  }

  destroy(): void {
    if (this.#destroyed) return;
    const error = new Error("the QuickGUI app was destroyed");
    this.#rejectDialogs(undefined, error);
    rejectPendingSystemRequests(error);
    this.releaseSingleInstanceLock();
    if (hostedRuntime) binding.destroyHostedApp(this.nativeId);
    else binding.destroyApp(this.nativeId);
    this.#destroyed = true;
    this.#appEventListeners.clear();
    if (activeApp === this) activeApp = undefined;
    for (const window of this.windows.values()) window._didDestroy();
    this.windows.clear();
  }

  #dispatchAppEvent(event: binding.NativeEvent): boolean {
    let type: keyof AppEventMap;
    let payload: AppEventMap[keyof AppEventMap];
    if (event.kind === "open-urls") {
      type = "openUrls";
      try {
        const parsed: unknown = JSON.parse(event.value ?? "[]");
        payload = Array.isArray(parsed) && parsed.every((url) => typeof url === "string") ? parsed : [];
      } catch {
        payload = [];
      }
    } else if (event.kind === "reopen") {
      type = "reopen";
      payload = { hasVisibleWindows: event.value === "true" };
    } else if (event.kind === "system-wake") {
      type = "systemWake";
      payload = undefined;
    } else if (event.kind === "keyboard-layout-change") {
      type = "keyboardLayoutChange";
      const layout = hostedRuntime
        ? binding.getHostedKeyboardLayout(this.nativeId)
        : binding.getKeyboardLayout(this.nativeId);
      payload = { id: layout.id, name: layout.name };
    } else if (event.kind === "notification-response") {
      type = "notificationResponse";
      try {
        const parsed = JSON.parse(event.value ?? "{}") as {
          tag?: unknown;
          actionId?: unknown;
        };
        if (typeof parsed.tag !== "string") return true;
        const response: NotificationResponse = { tag: parsed.tag };
        if (typeof parsed.actionId === "string") response.actionId = parsed.actionId;
        payload = response;
      } catch {
        return true;
      }
    } else if (event.kind === "second-instance") {
      type = "secondInstance";
      try {
        const parsed = JSON.parse(event.value ?? "{}") as {
          argv?: unknown;
          cwd?: unknown;
        };
        if (
          !Array.isArray(parsed.argv) ||
          !parsed.argv.every((argument) => typeof argument === "string") ||
          typeof parsed.cwd !== "string"
        ) {
          return true;
        }
        payload = { argv: parsed.argv, cwd: parsed.cwd };
        const urls = urlsFromArguments(parsed.argv);
        if (urls.length > 0) this.#emitAppEvent("openUrls", urls);
      } catch {
        return true;
      }
    } else {
      return false;
    }
    this.#emitAppEvent(type, payload);
    return true;
  }

  #emitAppEvent<K extends keyof AppEventMap>(type: K, payload: AppEventMap[K]): void {
    for (const listener of this.#appEventListeners.get(type) ?? []) listener(payload);
  }

  async #emitAppEventAndWait<K extends keyof AppEventMap>(
    type: K,
    payload: AppEventMap[K],
  ): Promise<void> {
    await Promise.all(
      [...(this.#appEventListeners.get(type) ?? [])].map((listener) => listener(payload)),
    );
  }

  _registerWindow(window: Window): void {
    this.#assertAlive();
    this.windows.set(window.nativeId, window);
  }

  _assertReady(): void {
    this.#assertAlive();
    if (!this.isReady()) {
      throw new Error("await app.whenReady() before using the native QuickGUI application");
    }
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
    this.#rejectDialogs(window, new Error("the native dialog's owner window closed"));
    window._didClose();
  }

  _showAlertDialog(window: Window | undefined, options: AlertDialogOptions): Promise<number> {
    try {
      const nativeOptions = normalizeAlertDialogOptions(options);
      return this.#requestDialog(
        window,
        "alert-dialog",
        (request) => {
          if (hostedRuntime) {
            binding.showHostedAlertDialog(this.nativeId, window?.nativeId, request, nativeOptions);
          } else {
            binding.showAlertDialog(this.nativeId, window?.nativeId, request, nativeOptions);
          }
        },
        (event) => {
          const response = Number(event.value);
          if (!Number.isSafeInteger(response) || response < 0) {
            throw new Error("the native dialog returned an invalid button index");
          }
          return response;
        },
      );
    } catch (error) {
      return Promise.reject(asError(error));
    }
  }

  _showOpenDialog(
    window: Window | undefined,
    options: OpenDialogOptions,
  ): Promise<OpenDialogResult> {
    try {
      const nativeOptions = normalizeOpenDialogOptions(options);
      return this.#requestDialog(
        window,
        "open-dialog",
        (request) => {
          if (hostedRuntime) {
            binding.showHostedOpenDialog(this.nativeId, window?.nativeId, request, nativeOptions);
          } else {
            binding.showOpenDialog(this.nativeId, window?.nativeId, request, nativeOptions);
          }
        },
        (event) => ({
          canceled: event.paths === undefined,
          filePaths: event.paths ?? [],
        }),
      );
    } catch (error) {
      return Promise.reject(asError(error));
    }
  }

  _showSaveDialog(
    window: Window | undefined,
    options: SaveDialogOptions,
  ): Promise<SaveDialogResult> {
    try {
      const nativeOptions = normalizeSaveDialogOptions(options);
      return this.#requestDialog(
        window,
        "save-dialog",
        (request) => {
          if (hostedRuntime) {
            binding.showHostedSaveDialog(this.nativeId, window?.nativeId, request, nativeOptions);
          } else {
            binding.showSaveDialog(this.nativeId, window?.nativeId, request, nativeOptions);
          }
        },
        (event) =>
          event.value === undefined
            ? { canceled: true }
            : { canceled: false, filePath: event.value },
      );
    } catch (error) {
      return Promise.reject(asError(error));
    }
  }

  #requestDialog<T>(
    window: Window | undefined,
    kind: DialogEventKind,
    invoke: (request: number) => void,
    result: (event: binding.NativeEvent) => T,
  ): Promise<T> {
    this.#assertAlive();
    if (window && (window.closed || this.windows.get(window.nativeId) !== window)) {
      return Promise.reject(new Error("the native dialog parent must be an open window"));
    }
    const request = this.#allocateDialogRequest();
    return new Promise<T>((resolve, reject) => {
      this.#pendingDialogs.set(request, {
        kind,
        window,
        complete: (event) => resolve(result(event)),
        reject,
      });
      try {
        invoke(request);
      } catch (error) {
        this.#pendingDialogs.delete(request);
        reject(asError(error));
      }
    });
  }

  #dispatchDialog(event: binding.NativeEvent): void {
    const pending = this.#pendingDialogs.get(event.target);
    if (!pending) return;
    this.#pendingDialogs.delete(event.target);
    if ((pending.window?.nativeId ?? 0) !== event.window) {
      pending.reject(new Error("the native dialog response had the wrong owner window"));
      return;
    }
    if (pending.kind !== event.kind) {
      pending.reject(new Error("the native dialog response had the wrong response type"));
      return;
    }
    if (event.error !== undefined) {
      pending.reject(new Error(event.error));
      return;
    }
    try {
      pending.complete(event);
    } catch (error) {
      pending.reject(asError(error));
    }
  }

  #allocateDialogRequest(): number {
    for (let attempt = 0; attempt <= this.#pendingDialogs.size; attempt += 1) {
      const request = this.#nextDialogRequest;
      this.#nextDialogRequest = request >= 0xffff_ffff ? 1 : request + 1;
      if (!this.#pendingDialogs.has(request)) return request;
    }
    throw new Error("the native dialog request id space is exhausted");
  }

  #rejectDialogs(window: Window | undefined, error: Error): void {
    for (const [request, pending] of this.#pendingDialogs) {
      if (window && pending.window !== window) continue;
      this.#pendingDialogs.delete(request);
      pending.reject(error);
    }
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
  #nativeReady = false;
  #closed = false;
  readonly #closeListeners = new Set<WindowCloseListener>();
  readonly #mountDisposers = new Set<() => void>();

  /** Return the Window whose renderer or native event callback is currently executing. */
  static getCurrentWindow(): Window {
    if (!currentWindow) {
      throw new Error(
        "Window.getCurrentWindow() must be called while rendering or handling a window event",
      );
    }
    return currentWindow;
  }

  constructor(options: WindowOptions) {
    const app = activeApp;
    if (!app) throw new Error("the QuickGUI app is unavailable");
    if (!app.isReady()) {
      throw new Error("await app.whenReady() before creating a QuickGUI Window");
    }
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
    const parent = options.anchor?.host;
    if (
      options.anchor &&
      (!parent || parent.closed || !options.anchor.materialized)
    ) {
      throw new Error("an anchored Window requires a mounted node in an open parent Window");
    }
    parent?.flush();
    this.app = app;
    this.root = new NativeNode(NativeNodeTag.View, "", ROOT_NODE_ID);
    this.root.host = this;
    this.root.materialized = true;
    this.nodes.set(ROOT_NODE_ID, this.root);
    try {
      const dispose = withCurrentWindow(this, () => options.renderer(this));
      if (typeof dispose !== "function") {
        throw new TypeError("a QuickGUI Window renderer must return a dispose function");
      }
      this._trackMount(dispose);
      const initialBatch = this.#takePendingBatch();
      if (options.anchor) {
        this.nativeId = hostedRuntime
          ? binding.createHostedAnchoredWindow(
              app.nativeId,
              parent!.nativeId,
              options.anchor.id,
              nativeOptions,
              initialBatch,
            )
          : binding.createAnchoredWindow(
              app.nativeId,
              parent!.nativeId,
              options.anchor.id,
              nativeOptions,
              initialBatch,
            );
      } else {
        this.nativeId = hostedRuntime
          ? binding.createHostedWindow(app.nativeId, nativeOptions, initialBatch)
          : binding.createWindow(app.nativeId, nativeOptions, initialBatch);
      }
      this.#nativeReady = true;
      app._registerWindow(this);
    } catch (error) {
      this._didClose();
      throw error;
    }
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

  getState(): WindowState {
    return getNativeWindowState(this);
  }

  onStateChange(listener: (state: WindowState) => void): () => void {
    if (this.#closed) return () => {};
    return onNativeWindowStateChange(this, listener);
  }

  setTitle(title: string): void {
    performNativeWindowAction(this, "set-title", title);
  }

  minimize(): void {
    performNativeWindowAction(this, "minimize");
  }

  maximize(): void {
    performNativeWindowAction(this, "maximize");
  }

  restore(): void {
    performNativeWindowAction(this, "restore");
  }

  setFullscreen(fullscreen: boolean): void {
    performNativeWindowAction(this, "set-fullscreen", String(fullscreen));
  }

  show(): void {
    performNativeWindowAction(this, "set-visible", "true");
  }

  hide(): void {
    performNativeWindowAction(this, "set-visible", "false");
  }

  focus(): void {
    performNativeWindowAction(this, "focus");
  }

  requestAttention(): void {
    performNativeWindowAction(this, "request-attention");
  }

  setRepresentedFile(path?: string): void {
    performNativeWindowAction(this, "set-represented-file", path ?? "");
  }

  setDocumentEdited(edited: boolean): void {
    performNativeWindowAction(this, "set-document-edited", String(edited));
  }

  setAppearance(appearance: AppearancePreference): void {
    performNativeWindowAction(this, "set-appearance", appearance);
  }

  _focusNode(node: NativeNode): boolean {
    if (this.#closed || node.host !== this) return false;
    this.flush();
    return hostedRuntime
      ? binding.focusHostedNode(this.app.nativeId, this.nativeId, node.id)
      : binding.focusNode(this.app.nativeId, this.nativeId, node.id);
  }

  flush(): number | undefined {
    if (this.#closed || !this.#nativeReady) return undefined;
    this.#flushScheduled = false;
    if (this.#batch.empty) return undefined;
    const batch = this.#batch;
    this.#batch = new MutationBatch();
    const bytes = batch.finish();
    return hostedRuntime
      ? binding.applyHostedBatch(this.app.nativeId, this.nativeId, bytes)
      : binding.applyBatch(this.app.nativeId, this.nativeId, bytes);
  }

  #takePendingBatch() {
    this.#flushScheduled = false;
    const batch = this.#batch;
    this.#batch = new MutationBatch();
    return batch.finish();
  }

  _dispatchEvent(type: NativeEventType, targetId: number, value?: string): void {
    withCurrentWindow(this, () => {
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
    });
  }

  _didClose(): void {
    if (this.#closed) return;
    this.#closed = true;
    this.#flushScheduled = false;
    const disposers = [...this.#mountDisposers];
    this.#mountDisposers.clear();
    for (const dispose of disposers) dispose();
    this.#batch = new MutationBatch();
    removeNativeWindowStateListeners(this);
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

function showAlertDialog(options: AlertDialogOptions): Promise<number>;
function showAlertDialog(window: Window, options: AlertDialogOptions): Promise<number>;
function showAlertDialog(
  windowOrOptions: Window | AlertDialogOptions,
  maybeOptions?: AlertDialogOptions,
): Promise<number> {
  const hasWindow = windowOrOptions instanceof Window;
  const window = hasWindow ? windowOrOptions : undefined;
  const options = hasWindow ? maybeOptions : (windowOrOptions as AlertDialogOptions);
  if (!options) return Promise.reject(new TypeError("showAlertDialog requires options"));
  const app = window?.app ?? activeApp;
  if (!app) return Promise.reject(new Error("create a QuickGUI App before showing a dialog"));
  return app._showAlertDialog(window, options);
}

function showOpenDialog(options?: OpenDialogOptions): Promise<OpenDialogResult>;
function showOpenDialog(window: Window, options?: OpenDialogOptions): Promise<OpenDialogResult>;
function showOpenDialog(
  windowOrOptions: Window | OpenDialogOptions = {},
  maybeOptions: OpenDialogOptions = {},
): Promise<OpenDialogResult> {
  const hasWindow = windowOrOptions instanceof Window;
  const window = hasWindow ? windowOrOptions : undefined;
  const options = hasWindow ? maybeOptions : (windowOrOptions as OpenDialogOptions);
  const app = window?.app ?? activeApp;
  if (!app) return Promise.reject(new Error("create a QuickGUI App before showing a dialog"));
  return app._showOpenDialog(window, options);
}

function showSaveDialog(options?: SaveDialogOptions): Promise<SaveDialogResult>;
function showSaveDialog(window: Window, options?: SaveDialogOptions): Promise<SaveDialogResult>;
function showSaveDialog(
  windowOrOptions: Window | SaveDialogOptions = {},
  maybeOptions: SaveDialogOptions = {},
): Promise<SaveDialogResult> {
  const hasWindow = windowOrOptions instanceof Window;
  const window = hasWindow ? windowOrOptions : undefined;
  const options = hasWindow ? maybeOptions : (windowOrOptions as SaveDialogOptions);
  const app = window?.app ?? activeApp;
  if (!app) return Promise.reject(new Error("create a QuickGUI App before showing a dialog"));
  return app._showSaveDialog(window, options);
}

/** Platform-native dialogs. Pass a Window first to attach the dialog; omit it for app-modal UI. */
export const Dialog = Object.freeze({
  showAlertDialog,
  showOpenDialog,
  showSaveDialog,
});

function asError(error: unknown): Error {
  return error instanceof Error ? error : new Error(String(error));
}

export type Application = App;
export const app = new App();
