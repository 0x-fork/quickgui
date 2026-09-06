import { dispatchFileWatch } from "./integrations.ts";
/**
 * Application, window, dialog, and event plumbing for natively compiled QuickGUI applications.
 *
 * The compiled program runs on its own thread while the Rust host owns the platform loop. Every
 * call here enqueues a bounded command and returns; outcomes arrive as host events on this thread
 * and settle Promises or dispatch to listeners. Nothing waits on the native main thread.
 */

import { statSync } from "node:fs";
import { basename, dirname, resolve, sep } from "node:path";
import {
  hostAllocateWindow,
  hostClearEventCallback,
  hostCloseWindow,
  hostCreateApp,
  hostCreateEmbeddedView,
  hostCreateSystemPopover,
  hostCreateWindow,
  hostDestroyApp,
  hostPrepareApp,
  hostProtocolVersion,
  hostSetEventCallback,
  hostShowDialog,
  type HostEventCallback,
} from "./ffi.ts";
import {
  NativeNode,
  NodeHost,
  createRootNode,
  dispatchNativeEvent,
  eventTypeFromKind,
  parseColor,
  type ColorValue,
} from "./native-tree.ts";
import { PROTOCOL_VERSION, ROOT_NODE_ID } from "./protocol.ts";
import {
  Listeners,
  VoidListeners,
  dispatchSystemEvent,
  rejectPendingSystemRequests,
  releaseMenuCallbacks,
  serializeMenuDefinitions,
} from "./system.ts";
import type { HostEventExtra, ImageSource, MenuDefinition, WindowRestoreState } from "./system.ts";
import { encodeBase64 } from "./base64.ts";

export { NativeNodeTag, NativePart, PropertyCode, PROTOCOL_VERSION, ROOT_NODE_ID, MutationBatch } from "./protocol.ts";
export type { NativePartName } from "./protocol.ts";
export * from "./native-tree.ts";
export * from "./system.ts";
export * from "./integrations.ts";
export * from "./router.ts";
export { encodeBase64, decodeBase64 } from "./base64.ts";

import {
  allocateRequest,
  assertAppReady,
  awaitReply,
  callService,
  isNullJson,
  jsonBoolean,
  READY_REQUEST,
  rejectAllReplies,
  reportProgress,
  sendCommand,
  sendInvoke,
  sendMutation,
  sendRequest,
  setAppContext,
  settleDataReply,
  settleReply,
} from "./requests.ts";

export { allocateRequest, callService, isNullJson, sendCommand, sendInvoke, sendMutation, sendRequest } from "./requests.ts";

// ---------------------------------------------------------------------------
// Application
// ---------------------------------------------------------------------------

export type QuitMode = "default" | "last-window-closed" | "explicit";

export interface AppPathOverrides {
  resourceDir?: string;
  configDir?: string;
  dataDir?: string;
  localDataDir?: string;
  cacheDir?: string;
  logDir?: string;
  runtimeDir?: string;
  tempDir?: string;
}

export interface AppOptions {
  name?: string;
  version?: string;
  identifier?: string;
  paths?: AppPathOverrides;
  quitMode?: QuitMode;
  /** OpenType font files registered by the Rust core before the first window is created. */
  fonts?: string[];
}

export interface RelaunchOptions {
  executable?: string;
  arguments?: string[];
  /** Relaunch without the current arguments. */
  clearArguments?: boolean;
  workingDirectory?: string;
}

export interface AppInfo {
  name: string;
  version: string;
  identifier: string;
}

export interface AppPaths {
  executable: string;
  executableDir: string;
  resourceDir: string;
  homeDir?: string;
  configDir?: string;
  dataDir?: string;
  localDataDir?: string;
  cacheDir?: string;
  logDir?: string;
  runtimeDir?: string;
  tempDir: string;
  audioDir?: string;
  desktopDir?: string;
  documentDir?: string;
  downloadDir?: string;
  pictureDir?: string;
  videoDir?: string;
}

export interface SystemInfo {
  operatingSystem: string;
  family: string;
  name: string;
  version?: string;
  edition?: string;
  codename?: string;
  architecture: string;
  bitness: string;
  hostname?: string;
  locale?: string;
  preferredLanguages: string[];
  languagesTruncated: boolean;
}

/** Why the operating system or application began an orderly shutdown. */
export type QuitReason = "explicit" | "relaunch" | "last-window-closed" | "operating-system";

export interface QuitEvent {
  exitCode: number;
}

export interface QuitPhaseEvent {
  reason: QuitReason;
}

export interface ReopenEvent {
  hasVisibleWindows: boolean;
}

export interface KeyboardLayoutEvent {
  id: string;
  name: string;
}

export interface NotificationResponseEvent {
  tag: string;
  actionId?: string;
  reply?: string;
}

export interface SecondInstanceEvent {
  argv: string[];
  cwd: string;
}

export interface OpenUrlsEvent {
  urls: string[];
}

interface NativeAppOptions {
  name?: string;
  version?: string;
  identifier?: string;
  resourceDir?: string;
  configDir?: string;
  dataDir?: string;
  localDataDir?: string;
  cacheDir?: string;
  logDir?: string;
  runtimeDir?: string;
  tempDir?: string;
  quitMode?: string;
  fonts?: string[];
}

function nativeAppOptions(options: AppOptions): NativeAppOptions {
  const native: NativeAppOptions = {};
  if (options.name !== undefined) native.name = options.name;
  if (options.version !== undefined) native.version = options.version;
  if (options.identifier !== undefined) native.identifier = options.identifier;
  if (options.quitMode !== undefined) native.quitMode = options.quitMode;
  if (options.fonts !== undefined) native.fonts = options.fonts;
  const paths = options.paths;
  if (paths !== undefined) {
    if (paths.resourceDir !== undefined) native.resourceDir = paths.resourceDir;
    if (paths.configDir !== undefined) native.configDir = paths.configDir;
    if (paths.dataDir !== undefined) native.dataDir = paths.dataDir;
    if (paths.localDataDir !== undefined) native.localDataDir = paths.localDataDir;
    if (paths.cacheDir !== undefined) native.cacheDir = paths.cacheDir;
    if (paths.logDir !== undefined) native.logDir = paths.logDir;
    if (paths.runtimeDir !== undefined) native.runtimeDir = paths.runtimeDir;
    if (paths.tempDir !== undefined) native.tempDir = paths.tempDir;
  }
  return native;
}

let embeddedAppOptions: AppOptions | undefined = undefined;

/** @internal The CLI embeds the packaged identity and fonts before the application entry runs. */
export function __embedApp(name: string, version: string, identifier: string, fonts: string[]): void {
  embeddedAppOptions = { name, version, identifier, fonts };
}

let activeApp: App | undefined = undefined;
let currentWindow: Window | undefined = undefined;

export function withCurrentWindow<T>(window: Window, callback: () => T): T {
  const previous = currentWindow;
  currentWindow = window;
  try {
    return callback();
  } finally {
    currentWindow = previous;
  }
}

function quitReason(value: string | undefined): QuitReason {
  if (value === "relaunch" || value === "last-window-closed" || value === "operating-system") return value;
  return "explicit";
}

/** The host event callback. One function value, registered once and released on exit. */
const onHostEvent: HostEventCallback = (
  kind: string,
  window: number,
  target: number,
  flags: number,
  value: string,
  extra: string,
  data: Uint8Array,
): void => {
  const application = activeApp;
  if (application === undefined) return;
  application._dispatchHostEvent(
    kind,
    window,
    target,
    (flags & 1) !== 0 ? value : undefined,
    (flags & 2) !== 0 ? (JSON.parse(extra) as HostEventExtra) : undefined,
    (flags & 4) !== 0 ? data : undefined,
  );
};

export class App {
  nativeId = 0;
  readonly windows = new Map<number, Window>();
  _ready = false;
  _readyPromise: Promise<void> | undefined = undefined;
  _resolveReady: (() => void) | undefined = undefined;
  _rejectReady: ((error: Error) => void) | undefined = undefined;
  _destroyed = false;
  _exited = false;
  _exitCode = 0;
  _requestedExitCode: number | undefined = undefined;
  _singleInstanceIdentifier: string | undefined = undefined;
  _quitIntercepting = false;
  _quitListeners = new Listeners<QuitEvent>();
  _quitResolvers: (() => void)[] = [];
  readonly _onReady = new VoidListeners();
  readonly _onBeforeQuit = new Listeners<QuitPhaseEvent>();
  readonly _onWillQuit = new Listeners<QuitPhaseEvent>();
  readonly _onOpenUrls = new Listeners<OpenUrlsEvent>();
  readonly _onReopen = new Listeners<ReopenEvent>();
  readonly _onActivate = new VoidListeners();
  readonly _onDeactivate = new VoidListeners();
  readonly _onSystemWake = new VoidListeners();
  readonly _onKeyboardLayoutChange = new Listeners<KeyboardLayoutEvent>();
  readonly _onNotificationResponse = new Listeners<NotificationResponseEvent>();
  readonly _onSecondInstance = new Listeners<SecondInstanceEvent>();
  readonly _pendingDialogs = new Map<number, PendingDialog>();

  isReady(): boolean {
    return this._ready;
  }

  /** Create the native application on first use and resolve once its platform loop is ready. */
  whenReady(): Promise<void> {
    const existing = this._readyPromise;
    if (existing !== undefined) return existing;
    const promise = new Promise<void>((resolve, reject) => {
      this._resolveReady = resolve;
      this._rejectReady = reject;
    });
    this._readyPromise = promise;
    const nativeProtocolVersion = hostProtocolVersion();
    if (nativeProtocolVersion !== PROTOCOL_VERSION) {
      const rejectReady = this._rejectReady;
      if (rejectReady !== undefined) {
        rejectReady(
          new Error(
            "QuickGUI native protocol mismatch: the application uses " +
              String(PROTOCOL_VERSION) +
              ", the host uses " +
              String(nativeProtocolVersion),
          ),
        );
      }
      return promise;
    }
    const options = embeddedAppOptions === undefined ? {} : nativeAppOptions(embeddedAppOptions);
    hostSetEventCallback(onHostEvent);
    const id = hostCreateApp(JSON.stringify(options));
    if (id === 0) {
      const rejectReady = this._rejectReady;
      if (rejectReady !== undefined) rejectReady(new Error("the QuickGUI host refused to create the application"));
      return promise;
    }
    this.nativeId = id;
    setAppContext(id, false);
    hostPrepareApp(id, READY_REQUEST);
    return promise;
  }

  /** Patch core-owned identity, paths, and quit policy before the first readiness turn. */
  async configure(options: AppOptions): Promise<void> {
    this._assertAlive();
    await sendCommand(JSON.stringify({ method: "configure-app", options: nativeAppOptions(options) }));
  }

  async getInfo(): Promise<AppInfo | undefined> {
    const json = await sendCommand('{"method":"get-app-info"}');
    return isNullJson(json) ? undefined : (JSON.parse(json) as AppInfo);
  }

  async getPaths(): Promise<AppPaths | undefined> {
    const json = await sendCommand('{"method":"get-app-paths"}');
    return isNullJson(json) ? undefined : (JSON.parse(json) as AppPaths);
  }

  async getSystemInfo(): Promise<SystemInfo> {
    const json = await sendCommand('{"method":"get-system-info"}');
    return JSON.parse(json) as SystemInfo;
  }

  onReady(listener: () => void): () => void {
    if (this._ready) listener();
    return this._onReady.add(listener);
  }

  onQuit(listener: (event: QuitEvent) => void): () => void {
    return this._quitListeners.add(listener);
  }

  /**
   * Hold the first preventable quit phase.
   *
   * Registering a listener declares quit interception, so the native shutdown waits until the
   * application completes it with `app.quit(true)` or `app.exit(code)`.
   */
  onBeforeQuit(listener: (event: QuitPhaseEvent) => void): () => void {
    const remove = this._onBeforeQuit.add(listener);
    this._syncQuitInterception();
    return () => {
      remove();
      this._syncQuitInterception();
    };
  }

  onWillQuit(listener: (event: QuitPhaseEvent) => void): () => void {
    return this._onWillQuit.add(listener);
  }

  onOpenUrls(listener: (event: OpenUrlsEvent) => void): () => void {
    return this._onOpenUrls.add(listener);
  }

  onReopen(listener: (event: ReopenEvent) => void): () => void {
    return this._onReopen.add(listener);
  }

  onActivate(listener: () => void): () => void {
    return this._onActivate.add(listener);
  }

  onDeactivate(listener: () => void): () => void {
    return this._onDeactivate.add(listener);
  }

  onSystemWake(listener: () => void): () => void {
    return this._onSystemWake.add(listener);
  }

  onKeyboardLayoutChange(listener: (event: KeyboardLayoutEvent) => void): () => void {
    return this._onKeyboardLayoutChange.add(listener);
  }

  onNotificationResponse(listener: (event: NotificationResponseEvent) => void): () => void {
    return this._onNotificationResponse.add(listener);
  }

  onSecondInstance(listener: (event: SecondInstanceEvent) => void): () => void {
    return this._onSecondInstance.add(listener);
  }

  /** Resolve once the application has quit, with its exit code. */
  run(): Promise<number> {
    if (this._exited) return Promise.resolve(this._exitCode);
    return new Promise<number>((resolve) => {
      this._quitResolvers.push(() => resolve(this._exitCode));
    });
  }

  _syncQuitInterception(): void {
    if (this._destroyed || !this._ready) return;
    const intercepting = this._onBeforeQuit.size > 0;
    if (intercepting === this._quitIntercepting) return;
    this._quitIntercepting = intercepting;
    sendMutation(JSON.stringify({ method: "set-quit-interception", intercepting }));
  }

  async requestSingleInstanceLock(identifier: string): Promise<boolean> {
    this._assertAlive();
    const existing = this._singleInstanceIdentifier;
    if (existing !== undefined) {
      if (existing !== identifier) throw new Error("this QuickGUI app already owns a different single-instance lock");
      return true;
    }
    const json = await sendCommand(JSON.stringify({ method: "request-single-instance-lock", identifier }));
    const acquired = jsonBoolean(json);
    if (acquired) this._singleInstanceIdentifier = identifier;
    return acquired;
  }

  async releaseSingleInstanceLock(): Promise<boolean> {
    if (this._singleInstanceIdentifier === undefined || this._destroyed || this._exited) return false;
    const json = await sendCommand('{"method":"release-single-instance-lock"}');
    const released = jsonBoolean(json);
    if (released) this._singleInstanceIdentifier = undefined;
    return released;
  }

  /**
   * Request an orderly native shutdown. Resolves false after shutdown already began.
   *
   * The default runs the preventable `beforeQuit` and `willQuit` phases, so a registered
   * `onBeforeQuit` listener holds the application open. Pass `true` to complete a held quit,
   * bypassing every interception.
   */
  async quit(force = false): Promise<boolean> {
    this._assertAlive();
    const json = await sendCommand(force ? '{"method":"exit"}' : '{"method":"request-quit"}');
    return json === "true";
  }

  /** Complete a held quit and report `code` from `app.run()` and the quit event. */
  async exit(code = 0): Promise<boolean> {
    if (!Number.isInteger(code) || code < 0 || code > 255) {
      throw new RangeError("an application exit code must be an integer between 0 and 255");
    }
    this._assertAlive();
    this._requestedExitCode = code;
    const json = await sendCommand(JSON.stringify({ method: "exit-with-code", code }));
    return json === "true";
  }

  /** Whether this process is running from an installed application bundle. */
  get isPackaged(): boolean {
    return callService("is-application-packaged", "") === "true";
  }

  /** Change how the application appears in the Dock and application switcher (macOS). */
  async setActivationPolicy(policy: "regular" | "accessory" | "prohibited"): Promise<void> {
    await appService("set-activation-policy", policy);
  }

  /** Bring the application forward; `steal` takes focus from the frontmost application. */
  focus(steal = false): void {
    sendMutation(JSON.stringify({ method: "app-mutation", action: "activate", value: steal ? "true" : "false" }));
  }

  hide(): void {
    sendMutation('{"method":"app-mutation","action":"hide"}');
  }

  show(): void {
    sendMutation('{"method":"app-mutation","action":"unhide"}');
  }

  setSecureKeyboardEntryEnabled(enabled: boolean): void {
    sendMutation(
      JSON.stringify({ method: "app-mutation", action: "set-secure-keyboard-entry", value: enabled ? "true" : "false" }),
    );
  }

  async isInApplicationsFolder(): Promise<boolean> {
    const json = await sendCommand('{"method":"get-applications-folder-support"}');
    const support = JSON.parse(json) as { supported: boolean; alreadyInstalled: boolean };
    return support.alreadyInstalled;
  }

  async moveToApplicationsFolder(): Promise<boolean> {
    const value = await appService("move-to-applications-folder", undefined);
    return value === "true";
  }

  /** Schedule a replacement process after ordinary child-first native teardown. */
  async relaunch(options: RelaunchOptions = {}): Promise<boolean> {
    const json = await sendCommand(JSON.stringify({ method: "relaunch", options }));
    return json === "true";
  }

  /** Bounce the Dock tile and resolve with the identifier that cancels a critical bounce. */
  async dockBounce(critical = false): Promise<number> {
    const value = await appService("request-dock-attention", critical ? "critical" : "informational");
    return Number(value ?? "0");
  }

  dockCancelBounce(id: number): void {
    sendMutation(JSON.stringify({ method: "app-mutation", action: "cancel-dock-attention", value: String(id) }));
  }

  async dockHide(): Promise<void> {
    await appService("set-dock-visible", "false");
  }

  async dockShow(): Promise<void> {
    await appService("set-dock-visible", "true");
  }

  destroy(): void {
    if (this._destroyed) return;
    this._destroyed = true;
    const error = new Error("the QuickGUI app was destroyed");
    this._rejectDialogs(error);
    rejectPendingSystemRequests(error);
    rejectAllReplies(error);
    if (this.nativeId !== 0) hostDestroyApp(this.nativeId);
    for (const window of this.windows.values()) window._didDestroy();
    this.windows.clear();
  }

  _assertAlive(): void {
    if (this._destroyed) throw new Error("the QuickGUI app was destroyed");
  }

  _assertReady(): void {
    this._assertAlive();
    if (!this._ready) throw new Error("await app.whenReady() before using the native QuickGUI application");
  }

  _registerWindow(window: Window): void {
    this._assertAlive();
    this.windows.set(window.nativeId, window);
  }

  _closeWindow(window: Window): void {
    if (this._destroyed || window.closed) return;
    hostCloseWindow(this.nativeId, window.nativeId);
  }

  _didCloseWindow(window: Window): void {
    this.windows.delete(window.nativeId);
    window._didClose();
  }

  _showDialog(window: Window | undefined, kind: number, options: string, complete: (event: DialogEvent) => void, reject: (error: Error) => void): void {
    this._assertReady();
    const request = allocateRequest();
    this._pendingDialogs.set(request, new PendingDialog(complete, reject));
    hostShowDialog(this.nativeId, window === undefined ? 0 : window.nativeId, request, kind, options);
  }

  _rejectDialogs(error: Error): void {
    for (const [request, pending] of this._pendingDialogs) {
      this._pendingDialogs.delete(request);
      pending.reject(error);
    }
  }

  _dispatchHostEvent(
    kind: string,
    window: number,
    target: number,
    value: string | undefined,
    extra: HostEventExtra | undefined,
    data: Uint8Array | undefined,
  ): void {
    const error = extra === undefined ? undefined : extra.error;
    switch (kind) {
      case "app-ready": {
        if (error !== undefined) {
          const rejectReady = this._rejectReady;
          if (rejectReady !== undefined) rejectReady(new Error(error));
          return;
        }
        this._ready = true;
        setAppContext(this.nativeId, true);
        this._syncQuitInterception();
        const resolveReady = this._resolveReady;
        if (resolveReady !== undefined) resolveReady();
        this._onReady.emit();
        return;
      }
      case "command":
      case "invoke":
        settleReply(target, value, error);
        return;
      case "module-result":
        settleDataReply(target, data, error);
        return;
      case "file-watch":
        dispatchFileWatch(target, value);
        return;
      case "update-progress":
        if (value !== undefined) reportProgress(target, value);
        return;
      case "exit":
        this._didExit(target);
        return;
      case "host-error":
        console.error("quickgui: " + (error ?? "the native host failed"));
        return;
      case "alert-dialog":
      case "open-dialog":
      case "save-dialog": {
        const pending = this._pendingDialogs.get(target);
        if (pending === undefined) return;
        this._pendingDialogs.delete(target);
        if (error !== undefined) pending.reject(new Error(error));
        else pending.complete(new DialogEvent(value, extra === undefined ? undefined : extra.paths));
        return;
      }
      default:
    }
    if (dispatchSystemEvent(kind, target, value, extra === undefined ? undefined : extra.error, data, extra)) return;
    if (this._dispatchAppEvent(kind, value)) return;
    const owner = this.windows.get(window);
    if (owner === undefined) return;
    if (kind === "close") {
      this._didCloseWindow(owner);
    } else if (kind === "close-requested") {
      owner._didRequestClose();
    } else if (kind.startsWith("window-")) {
      owner._didObserveLifecycle(kind, value);
    } else {
      const type = eventTypeFromKind(kind);
      if (type === 0) return;
      withCurrentWindow(owner, () => {
        dispatchNativeEvent(owner, type, target, value);
      });
      owner.flush();
    }
  }

  _dispatchAppEvent(kind: string, value: string | undefined): boolean {
    switch (kind) {
      case "open-urls": {
        const urls = value === undefined ? [] : (JSON.parse(value) as string[]);
        this._onOpenUrls.emit({ urls });
        return true;
      }
      case "reopen":
        this._onReopen.emit({ hasVisibleWindows: value === "true" });
        return true;
      case "system-wake":
        this._onSystemWake.emit();
        return true;
      case "app-activate":
        this._onActivate.emit();
        return true;
      case "app-deactivate":
        this._onDeactivate.emit();
        return true;
      case "before-quit":
        this._onBeforeQuit.emit({ reason: quitReason(value) });
        return true;
      case "will-quit":
        this._onWillQuit.emit({ reason: quitReason(value) });
        return true;
      case "keyboard-layout-change": {
        if (this._onKeyboardLayoutChange.size > 0) void this._emitKeyboardLayout();
        return true;
      }
      case "notification-response": {
        if (value === undefined) return true;
        this._onNotificationResponse.emit(JSON.parse(value) as NotificationResponseEvent);
        return true;
      }
      case "second-instance": {
        if (value === undefined) return true;
        const event = JSON.parse(value) as SecondInstanceEvent;
        this._onSecondInstance.emit(event);
        const urls: string[] = [];
        for (const argument of event.argv) {
          if (argument.includes("://") && !argument.startsWith("-")) urls.push(argument);
        }
        if (urls.length > 0) this._onOpenUrls.emit({ urls });
        return true;
      }
      default:
        return false;
    }
  }

  async _emitKeyboardLayout(): Promise<void> {
    try {
      const json = await sendCommand('{"method":"get-keyboard-layout"}');
      this._onKeyboardLayoutChange.emit(JSON.parse(json) as KeyboardLayoutEvent);
    } catch {
      // The application is shutting down; the layout no longer matters.
    }
  }

  _didExit(code: number): void {
    if (this._exited) return;
    this._exited = true;
    const requested = this._requestedExitCode;
    this._exitCode = requested !== undefined && code === 0 ? requested : code;
    for (const window of this.windows.values()) window._didClose();
    this.windows.clear();
    const error = new Error("the QuickGUI application has exited");
    this._rejectDialogs(error);
    rejectPendingSystemRequests(error);
    rejectAllReplies(error);
    this._quitListeners.emit({ exitCode: this._exitCode });
    const resolvers = [...this._quitResolvers];
    this._quitResolvers.splice(0, this._quitResolvers.length);
    for (const resolve of resolvers) resolve();
    releaseMenuCallbacks();
    // Releasing the callback lets the program's event loop drain and the process exit.
    hostClearEventCallback(onHostEvent);
  }
}

async function appService(action: string, value: string | undefined): Promise<string | undefined> {
  const request = allocateRequest();
  const json = value === undefined
    ? JSON.stringify({ method: "app-service", request, action })
    : JSON.stringify({ method: "app-service", request, action, value });
  // The command itself completes immediately; the outcome arrives as an `app-service` event.
  const result = await sendRequest(request, json);
  return result === "null" ? undefined : result;
}

// ---------------------------------------------------------------------------
// Windows
// ---------------------------------------------------------------------------

export type WindowRenderer = (window: Window) => () => void;
export type PopoverPlacement =
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
export type PerformanceProfile = "low-power" | "balanced" | "high-performance";
export type InitialWindowState = "normal" | "maximized" | "fullscreen";
export type WindowKind = "normal" | "popover" | "system-popover" | "floating" | "dialog";
export type WindowLevel =
  | "always-on-bottom"
  | "normal"
  | "always-on-top"
  | "floating"
  | "modal-panel"
  | "main-menu"
  | "status"
  | "pop-up-menu"
  | "screen-saver";
export type ElectronWindowLevel = "floating" | "modalPanel" | "mainMenu" | "status" | "popUpMenu" | "screenSaver";
export type CursorGrabMode = "none" | "confined" | "locked";
export type TaskbarProgressState = "none" | "normal" | "indeterminate" | "paused" | "error";
export type WindowBackgroundAppearance = "opaque" | "transparent" | "blurred";
export type AppearanceMode = "light" | "dark";
export type AppearancePreference = "light" | "dark" | "system";
export type MacOSVibrancy =
  | "appearance-based"
  | "titlebar"
  | "selection"
  | "menu"
  | "popover"
  | "sidebar"
  | "header"
  | "sheet"
  | "window"
  | "hud"
  | "fullscreen-ui"
  | "tooltip"
  | "content"
  | "under-window"
  | "under-page";
export type MacOSVisualEffectState = "followWindow" | "active" | "inactive";

export interface Size {
  width: number;
  height: number;
}

export interface Point {
  x: number;
  y: number;
}

export interface WindowBounds {
  x: number;
  y: number;
  width: number;
  height: number;
  state?: InitialWindowState;
}

export interface WindowResizePolicy {
  aspectRatio?: number;
  minimum?: Size;
  maximum?: Size;
  snap?: Size;
}

export interface WindowMovePolicy {
  keepOnScreen?: boolean;
}

export interface TaskbarProgress {
  state: TaskbarProgressState;
  progress: number;
}

export interface TaskbarOverlay {
  icon: ImageSource;
  description: string;
}

export interface WindowOptions {
  renderer: WindowRenderer;
  title?: string;
  width?: number;
  height?: number;
  position?: Point;
  initialState?: InitialWindowState;
  displayId?: string;
  /** Set `disableMinimumSize` to remove the core's default minimum window size. */
  minimumSize?: Size;
  disableMinimumSize?: boolean;
  minimumWidth?: number;
  minimumHeight?: number;
  maximumSize?: Size;
  maximumWidth?: number;
  maximumHeight?: number;
  representedFile?: string;
  documentEdited?: boolean;
  tabbingIdentifier?: string;
  background?: ColorValue;
  backgroundAppearance?: WindowBackgroundAppearance;
  vibrancy?: MacOSVibrancy;
  visualEffectState?: MacOSVisualEffectState;
  performanceProfile?: PerformanceProfile;
  appearance?: AppearancePreference;
  titleBarStyle?: "default" | "hidden" | "hiddenInset";
  kind?: WindowKind;
  focus?: boolean;
  focusable?: boolean;
  visible?: boolean;
  movable?: boolean;
  resizable?: boolean;
  minimizable?: boolean;
  maximizable?: boolean;
  closable?: boolean;
  decorated?: boolean;
  shadow?: boolean;
  contentProtected?: boolean;
  windowLevel?: WindowLevel | "automatic";
  skipTaskbar?: boolean;
  visibleOnAllWorkspaces?: boolean;
  opacity?: number;
  icon?: ImageSource;
  taskbarProgress?: TaskbarProgress;
  taskbarOverlay?: TaskbarOverlay;
  cursorVisible?: boolean;
  cursorGrab?: CursorGrabMode;
  cursorHitTest?: boolean;
  cursorPosition?: Point;
  menu?: MenuDefinition[];
  restoreState?: WindowRestoreState;
  lineScrollPixels?: number;
  keySequenceTimeoutMs?: number;
  reduceMotion?: boolean;
  trafficLightPosition?: Point;
  transparent?: boolean;
  blur?: boolean;
  /** Open this window as a system popover anchored to the mounted node. */
  anchor?: NativeNode;
  placement?: PopoverPlacement;
  gap?: number;
  offset?: Point;
  viewportMargin?: number;
  dismissOnEscape?: boolean;
  dismissOnPointerOutside?: boolean;
  grab?: boolean;
  acceptsKeyFocus?: boolean;
}

export interface NativeImageSource {
  data?: string;
  path?: string;
  width?: number;
  height?: number;
}

export function nativeImageSource(source: ImageSource): NativeImageSource {
  if (typeof source === "string") return { path: source };
  const native: NativeImageSource = { data: encodeBase64(source.data) };
  if (source.width !== undefined) native.width = source.width;
  if (source.height !== undefined) native.height = source.height;
  return native;
}

interface NativeWindowOptions {
  title?: string;
  width?: number;
  height?: number;
  x?: number;
  y?: number;
  initialState?: string;
  displayId?: string;
  minimumSizeEnabled?: boolean;
  minimumWidth?: number;
  minimumHeight?: number;
  maximumWidth?: number;
  maximumHeight?: number;
  representedFile?: string;
  documentEdited?: boolean;
  tabbingIdentifier?: string;
  background?: number;
  performanceProfile?: string;
  appearance?: string;
  vibrancy?: string;
  visualEffectState?: string;
  titleBarStyle?: string;
  kind?: string;
  focus?: boolean;
  focusable?: boolean;
  show?: boolean;
  movable?: boolean;
  resizable?: boolean;
  minimizable?: boolean;
  maximizable?: boolean;
  closable?: boolean;
  decorated?: boolean;
  shadow?: boolean;
  contentProtected?: boolean;
  windowLevel?: string;
  skipTaskbar?: boolean;
  visibleOnAllWorkspaces?: boolean;
  opacity?: number;
  icon?: NativeImageSource;
  taskbarProgressState?: string;
  taskbarProgress?: number;
  taskbarOverlayIcon?: NativeImageSource;
  taskbarOverlayDescription?: string;
  cursorVisible?: boolean;
  cursorGrab?: string;
  cursorHitTest?: boolean;
  cursorX?: number;
  cursorY?: number;
  menu?: string;
  restoreState?: WindowRestoreState;
  lineScrollPixels?: number;
  keySequenceTimeoutMs?: number;
  reduceMotion?: boolean;
  trafficLightX?: number;
  trafficLightY?: number;
  transparent?: boolean;
  blur?: boolean;
  popoverPlacement?: string;
  popoverGap?: number;
  popoverOffsetX?: number;
  popoverOffsetY?: number;
  popoverViewportMargin?: number;
  popoverDismissOnEscape?: boolean;
  popoverDismissOnPointerOutside?: boolean;
  popoverGrab?: boolean;
  popoverAcceptsKeyFocus?: boolean;
}

function nativeWindowOptions(options: WindowOptions, menu: string | undefined): NativeWindowOptions {
  const native: NativeWindowOptions = {};
  if (options.title !== undefined) native.title = options.title;
  if (options.width !== undefined) native.width = options.width;
  if (options.height !== undefined) native.height = options.height;
  if (options.disableMinimumSize === true) {
    native.minimumSizeEnabled = false;
  } else if (options.minimumSize !== undefined) {
    native.minimumWidth = options.minimumSize.width;
    native.minimumHeight = options.minimumSize.height;
  } else {
    if (options.minimumWidth !== undefined) native.minimumWidth = options.minimumWidth;
    if (options.minimumHeight !== undefined) native.minimumHeight = options.minimumHeight;
  }
  if (options.maximumSize !== undefined) {
    native.maximumWidth = options.maximumSize.width;
    native.maximumHeight = options.maximumSize.height;
  } else {
    if (options.maximumWidth !== undefined) native.maximumWidth = options.maximumWidth;
    if (options.maximumHeight !== undefined) native.maximumHeight = options.maximumHeight;
  }
  if (options.position !== undefined) {
    native.x = options.position.x;
    native.y = options.position.y;
  }
  if (options.initialState !== undefined) native.initialState = options.initialState;
  if (options.displayId !== undefined) native.displayId = options.displayId;
  if (options.representedFile !== undefined) native.representedFile = options.representedFile;
  if (options.documentEdited !== undefined) native.documentEdited = options.documentEdited;
  if (options.tabbingIdentifier !== undefined) native.tabbingIdentifier = options.tabbingIdentifier;
  if (options.background !== undefined) native.background = parseColor(options.background);
  if (options.performanceProfile !== undefined) native.performanceProfile = options.performanceProfile;
  if (options.appearance !== undefined) native.appearance = options.appearance;
  if (options.vibrancy !== undefined) native.vibrancy = options.vibrancy;
  if (options.visualEffectState !== undefined) native.visualEffectState = options.visualEffectState;
  if (options.titleBarStyle !== undefined) native.titleBarStyle = options.titleBarStyle;
  if (options.kind !== undefined) native.kind = options.kind;
  if (options.focus !== undefined) native.focus = options.focus;
  if (options.focusable !== undefined) native.focusable = options.focusable;
  if (options.visible !== undefined) native.show = options.visible;
  if (options.movable !== undefined) native.movable = options.movable;
  if (options.resizable !== undefined) native.resizable = options.resizable;
  if (options.minimizable !== undefined) native.minimizable = options.minimizable;
  if (options.maximizable !== undefined) native.maximizable = options.maximizable;
  if (options.closable !== undefined) native.closable = options.closable;
  if (options.decorated !== undefined) native.decorated = options.decorated;
  if (options.shadow !== undefined) native.shadow = options.shadow;
  if (options.contentProtected !== undefined) native.contentProtected = options.contentProtected;
  if (options.windowLevel !== undefined) native.windowLevel = options.windowLevel;
  if (options.skipTaskbar !== undefined) native.skipTaskbar = options.skipTaskbar;
  if (options.visibleOnAllWorkspaces !== undefined) native.visibleOnAllWorkspaces = options.visibleOnAllWorkspaces;
  if (options.opacity !== undefined) native.opacity = options.opacity;
  if (options.icon !== undefined) native.icon = nativeImageSource(options.icon);
  if (options.taskbarProgress !== undefined) {
    native.taskbarProgressState = options.taskbarProgress.state;
    native.taskbarProgress = options.taskbarProgress.progress;
  }
  if (options.taskbarOverlay !== undefined) {
    native.taskbarOverlayIcon = nativeImageSource(options.taskbarOverlay.icon);
    native.taskbarOverlayDescription = options.taskbarOverlay.description;
  }
  if (options.cursorVisible !== undefined) native.cursorVisible = options.cursorVisible;
  if (options.cursorGrab !== undefined) native.cursorGrab = options.cursorGrab;
  if (options.cursorHitTest !== undefined) native.cursorHitTest = options.cursorHitTest;
  if (options.cursorPosition !== undefined) {
    native.cursorX = options.cursorPosition.x;
    native.cursorY = options.cursorPosition.y;
  }
  if (menu !== undefined) native.menu = menu;
  if (options.restoreState !== undefined) native.restoreState = options.restoreState;
  if (options.lineScrollPixels !== undefined) native.lineScrollPixels = options.lineScrollPixels;
  if (options.keySequenceTimeoutMs !== undefined) native.keySequenceTimeoutMs = options.keySequenceTimeoutMs;
  if (options.reduceMotion !== undefined) native.reduceMotion = options.reduceMotion;
  if (options.trafficLightPosition !== undefined) {
    native.trafficLightX = options.trafficLightPosition.x;
    native.trafficLightY = options.trafficLightPosition.y;
  }
  if (options.backgroundAppearance !== undefined) {
    native.transparent = options.backgroundAppearance === "transparent";
    native.blur = options.backgroundAppearance === "blurred";
  } else {
    if (options.transparent !== undefined) native.transparent = options.transparent;
    if (options.blur !== undefined) native.blur = options.blur;
  }
  if (options.placement !== undefined) native.popoverPlacement = options.placement;
  if (options.gap !== undefined) native.popoverGap = options.gap;
  if (options.offset !== undefined) {
    native.popoverOffsetX = options.offset.x;
    native.popoverOffsetY = options.offset.y;
  }
  if (options.viewportMargin !== undefined) native.popoverViewportMargin = options.viewportMargin;
  if (options.dismissOnEscape !== undefined) native.popoverDismissOnEscape = options.dismissOnEscape;
  if (options.dismissOnPointerOutside !== undefined) native.popoverDismissOnPointerOutside = options.dismissOnPointerOutside;
  if (options.grab !== undefined) native.popoverGrab = options.grab;
  if (options.acceptsKeyFocus !== undefined) native.popoverAcceptsKeyFocus = options.acceptsKeyFocus;
  return native;
}

export type WindowEventName =
  | "closed"
  | "closeRequested"
  | "minimize"
  | "restore"
  | "maximize"
  | "unmaximize"
  | "enterFullScreen"
  | "leaveFullScreen"
  | "readyToShow"
  | "occlusionChange"
  | "levelChange"
  | "willResize"
  | "willMove"
  | "resize"
  | "move"
  | "focus"
  | "blur"
  | "appearanceChange"
  | "stateChange";

/** One window lifecycle notification; only the fields the event carries are set. */
export interface WindowEvent {
  window: Window;
  type: WindowEventName;
  size?: Size;
  position?: Point;
  occluded?: boolean;
  level?: string;
  appearance?: string;
}

export type WindowEventListener = (event: WindowEvent) => void;

class WindowListener {
  readonly type: WindowEventName;
  readonly listener: WindowEventListener;

  constructor(type: WindowEventName, listener: WindowEventListener) {
    this.type = type;
    this.listener = listener;
  }
}

interface EmbeddedOwner {
  owner: Window;
  matchHorizontal: boolean;
  matchVertical: boolean;
}

export class Window extends NodeHost {
  readonly root: NativeNode;
  readonly app: App;
  readonly _listeners: WindowListener[] = [];
  readonly _mountDisposers: (() => void)[] = [];
  _closeIntercepting = false;
  _closeRequestListeners = 0;
  _menuRelease: (() => void) | undefined = undefined;

  /** Return the Window whose renderer or native event callback is currently executing. */
  static getCurrentWindow(): Window {
    const window = currentWindow;
    if (window === undefined) {
      throw new Error("Window.getCurrentWindow() must be called while rendering or handling a window event");
    }
    return window;
  }

  /** @internal Create one retained renderer whose native view is owned by a SwiftUI host. */
  static _createEmbedded(owner: Window, options: WindowOptions, matchHorizontal: boolean, matchVertical: boolean): Window {
    if (owner.closed) throw new Error("an embedded QuickGUI view requires an open owner Window");
    owner.flush();
    return new Window(options, { owner, matchHorizontal, matchVertical });
  }

  constructor(options: WindowOptions, embedded?: EmbeddedOwner) {
    const application = activeApp;
    if (application === undefined) throw new Error("the QuickGUI app is unavailable");
    if (!application.isReady()) throw new Error("await app.whenReady() before creating a QuickGUI Window");
    super(application.nativeId, hostAllocateWindow());
    this.app = application;
    this.root = createRootNode(this, ROOT_NODE_ID);
    const anchor = options.anchor;
    const parent = anchor === undefined ? undefined : anchor.host;
    if (anchor !== undefined && (parent === undefined || parent.closed)) {
      throw new Error("a system popover requires a mounted node in an open parent Window");
    }
    if (parent !== undefined) parent.flush();
    const menuDefinitions = options.menu;
    const menu = menuDefinitions === undefined ? undefined : serializeMenuDefinitions(menuDefinitions, this.nativeId);
    const native = nativeWindowOptions(options, menu === undefined ? undefined : menu.json);
    const json = JSON.stringify(native);
    let dispose: (() => void) | undefined = undefined;
    try {
      dispose = withCurrentWindow(this, () => options.renderer(this));
    } catch (error) {
      this._didClose();
      throw error;
    }
    this._mountDisposers.push(dispose);
    const initialBatch = this.takeBatch();
    if (embedded !== undefined) {
      hostCreateEmbeddedView(
        this.appId,
        this.nativeId,
        embedded.owner.nativeId,
        embedded.matchHorizontal,
        embedded.matchVertical,
        json,
        initialBatch,
      );
    } else if (anchor !== undefined && parent !== undefined) {
      hostCreateSystemPopover(this.appId, this.nativeId, parent.nativeId, anchor.id, json, initialBatch);
    } else {
      hostCreateWindow(this.appId, this.nativeId, json, initialBatch);
    }
    this.nativeReady = true;
    application._registerWindow(this);
    if (menu !== undefined) {
      this._menuRelease = menu.release;
    }
  }

  /** Subscribe to one window lifecycle event. */
  on(type: WindowEventName, listener: WindowEventListener): () => void {
    if (this.closed) return () => undefined;
    const entry = new WindowListener(type, listener);
    this._listeners.push(entry);
    if (type === "closeRequested") {
      this._closeRequestListeners += 1;
      this._syncCloseInterception();
    }
    return () => {
      const index = this._listeners.indexOf(entry);
      if (index >= 0) this._listeners.splice(index, 1);
      if (type === "closeRequested") {
        this._closeRequestListeners -= 1;
        this._syncCloseInterception();
      }
    };
  }

  onClose(listener: (window: Window) => void): () => void {
    if (this.closed) {
      listener(this);
      return () => undefined;
    }
    return this.on("closed", (event) => listener(event.window));
  }

  /** Hold native close requests for this window and decide in application code. */
  onCloseRequested(listener: (window: Window) => void): () => void {
    return this.on("closeRequested", (event) => listener(event.window));
  }

  _emit(type: WindowEventName, event: WindowEvent): void {
    const snapshot = [...this._listeners];
    withCurrentWindow(this, () => {
      for (const entry of snapshot) {
        if (entry.type === type) entry.listener(event);
      }
    });
  }

  /** Complete a held close, or begin an ordinary one. */
  close(): void {
    this.app._closeWindow(this);
  }

  /** Close the window immediately, bypassing every registered close-request listener. */
  destroy(): void {
    let index = this._listeners.length;
    while (index > 0) {
      index -= 1;
      if (this._listeners[index]!.type === "closeRequested") this._listeners.splice(index, 1);
    }
    this._closeRequestListeners = 0;
    this._syncCloseInterception();
    this.app._closeWindow(this);
  }

  _syncCloseInterception(): void {
    const intercepting = this._closeRequestListeners > 0;
    if (this.closed || intercepting === this._closeIntercepting) return;
    this._closeIntercepting = intercepting;
    this.action("set-close-interception", intercepting ? "true" : "false");
  }

  _didRequestClose(): void {
    if (this.closed) return;
    this._emit("closeRequested", { window: this, type: "closeRequested" });
  }

  _didObserveLifecycle(kind: string, value: string | undefined): void {
    if (this.closed) return;
    switch (kind) {
      case "window-minimize":
        this._emit(value === "true" ? "minimize" : "restore", { window: this, type: value === "true" ? "minimize" : "restore" });
        return;
      case "window-maximize":
        this._emit(value === "true" ? "maximize" : "unmaximize", { window: this, type: value === "true" ? "maximize" : "unmaximize" });
        return;
      case "window-fullscreen":
        this._emit(value === "true" ? "enterFullScreen" : "leaveFullScreen", {
          window: this,
          type: value === "true" ? "enterFullScreen" : "leaveFullScreen",
        });
        return;
      case "window-ready-to-show":
        this._emit("readyToShow", { window: this, type: "readyToShow" });
        return;
      case "window-occlusion":
        this._emit("occlusionChange", { window: this, type: "occlusionChange", occluded: value === "true" });
        return;
      case "window-level":
        this._emit("levelChange", { window: this, type: "levelChange", level: value ?? "normal" });
        return;
      case "window-focus":
        this._emit(value === "true" ? "focus" : "blur", { window: this, type: value === "true" ? "focus" : "blur" });
        return;
      case "window-appearance":
        this._emit("appearanceChange", { window: this, type: "appearanceChange", appearance: value === "dark" ? "dark" : "light" });
        return;
      case "window-state-change":
        this._emit("stateChange", { window: this, type: "stateChange" });
        return;
      case "window-will-resize":
      case "window-resize": {
        if (value === undefined) return;
        const size = JSON.parse(value) as Size;
        const type: WindowEventName = kind === "window-resize" ? "resize" : "willResize";
        this._emit(type, { window: this, type, size });
        return;
      }
      case "window-will-move":
      case "window-move": {
        if (value === undefined) return;
        const position = JSON.parse(value) as Point;
        const type: WindowEventName = kind === "window-move" ? "move" : "willMove";
        this._emit(type, { window: this, type, position });
        return;
      }
      default:
    }
  }

  _didClose(): void {
    if (this.closed) return;
    this.closed = true;
    this.flushScheduled = false;
    const disposers = [...this._mountDisposers];
    this._mountDisposers.splice(0, this._mountDisposers.length);
    for (const dispose of disposers) dispose();
    const release = this._menuRelease;
    if (release !== undefined) {
      this._menuRelease = undefined;
      release();
    }
    this._closeIntercepting = false;
    this._emit("closed", { window: this, type: "closed" });
    this._listeners.splice(0, this._listeners.length);
    this.nodes.clear();
  }

  _didDestroy(): void {
    this._didClose();
  }

  /** Register a disposer that runs when the window closes. */
  trackMount(dispose: () => void): () => void {
    if (this.closed) {
      dispose();
      return () => undefined;
    }
    this._mountDisposers.push(dispose);
    return () => {
      const index = this._mountDisposers.indexOf(dispose);
      if (index >= 0) this._mountDisposers.splice(index, 1);
    };
  }

  /** Queue one native window action. */
  action(action: string, value: string | undefined): void {
    if (this.closed) return;
    const json = value === undefined
      ? JSON.stringify({ method: "window-action", window: this.nativeId, action })
      : JSON.stringify({ method: "window-action", window: this.nativeId, action, value });
    sendMutation(json);
  }

  imageAction(action: string, image: ImageSource | undefined, description: string | undefined): void {
    if (this.closed) return;
    const request: { method: string; window: number; action: string; image?: NativeImageSource; description?: string } = {
      method: "window-image-action",
      window: this.nativeId,
      action,
    };
    if (image !== undefined) request.image = nativeImageSource(image);
    if (description !== undefined) request.description = description;
    sendMutation(JSON.stringify(request));
  }

  async getState(): Promise<WindowState> {
    const json = await sendCommand(JSON.stringify({ method: "get-window-state", window: this.nativeId }));
    return windowStateFromNative(JSON.parse(json) as NativeWindowState);
  }

  async getRestoreState(): Promise<WindowRestoreState> {
    const json = await sendCommand(JSON.stringify({ method: "get-window-restore-state", window: this.nativeId }));
    return JSON.parse(json) as WindowRestoreState;
  }

  setTitle(title: string): void {
    this.action("set-title", title);
  }

  setBounds(bounds: WindowBounds): void {
    this.action("set-bounds", JSON.stringify(bounds));
  }

  setPosition(position: Point): void {
    this.action("move", JSON.stringify(position));
  }

  setSize(size: Size): void {
    this.action("resize", JSON.stringify(size));
  }

  minimize(): void {
    this.action("minimize", undefined);
  }

  maximize(): void {
    this.action("maximize", undefined);
  }

  restore(): void {
    this.action("restore", undefined);
  }

  setFullscreen(fullscreen: boolean): void {
    this.action("set-fullscreen", fullscreen ? "true" : "false");
  }

  setResizable(resizable: boolean): void {
    this.action("set-resizable", resizable ? "true" : "false");
  }

  setMovable(movable: boolean): void {
    this.action("set-movable", movable ? "true" : "false");
  }

  setMinimumSize(size: Size | undefined): void {
    this.action("set-minimum-size", size === undefined ? undefined : JSON.stringify(size));
  }

  setMaximumSize(size: Size | undefined): void {
    this.action("set-maximum-size", size === undefined ? undefined : JSON.stringify(size));
  }

  setMinimizable(minimizable: boolean): void {
    this.action("set-minimizable", minimizable ? "true" : "false");
  }

  setMaximizable(maximizable: boolean): void {
    this.action("set-maximizable", maximizable ? "true" : "false");
  }

  setClosable(closable: boolean): void {
    this.action("set-closable", closable ? "true" : "false");
  }

  setDecorated(decorated: boolean): void {
    this.action("set-decorated", decorated ? "true" : "false");
  }

  setShadow(shadow: boolean): void {
    this.action("set-shadow", shadow ? "true" : "false");
  }

  setHasShadow(shadow: boolean): void {
    this.setShadow(shadow);
  }

  setContentProtected(contentProtected: boolean): void {
    this.action("set-content-protected", contentProtected ? "true" : "false");
  }

  setWindowLevel(level: WindowLevel | "automatic"): void {
    this.action("set-window-level", level);
  }

  setFocusable(focusable: boolean): void {
    this.action("set-focusable", focusable ? "true" : "false");
  }

  setSkipTaskbar(skip: boolean): void {
    this.action("set-skip-taskbar", skip ? "true" : "false");
  }

  setVisibleOnAllWorkspaces(visible: boolean): void {
    this.action("set-visible-on-all-workspaces", visible ? "true" : "false");
  }

  setOpacity(opacity: number): void {
    this.action("set-opacity", String(opacity));
  }

  setIcon(icon: ImageSource): void {
    this.imageAction("set-icon", icon, undefined);
  }

  clearIcon(): void {
    this.imageAction("clear-icon", undefined, undefined);
  }

  setTaskbarProgress(state: TaskbarProgressState, progress: number): void {
    this.action("set-taskbar-progress", JSON.stringify({ state, progress }));
  }

  setTaskbarOverlayIcon(icon: ImageSource, description: string): void {
    this.imageAction("set-taskbar-overlay-icon", icon, description);
  }

  clearTaskbarOverlayIcon(): void {
    this.action("clear-taskbar-overlay-icon", undefined);
  }

  setCursorVisible(visible: boolean): void {
    this.action("set-cursor-visible", visible ? "true" : "false");
  }

  setCursorGrab(mode: CursorGrabMode): void {
    this.action("set-cursor-grab", mode);
  }

  setCursorHitTest(hitTest: boolean): void {
    this.action("set-cursor-hit-test", hitTest ? "true" : "false");
  }

  setCursorPosition(position: Point): void {
    this.action("set-cursor-position", JSON.stringify(position));
  }

  show(): void {
    this.action("set-visible", "true");
  }

  hide(): void {
    this.action("set-visible", "false");
  }

  focus(): void {
    this.action("focus", undefined);
  }

  requestAttention(): void {
    this.action("request-attention", undefined);
  }

  setRepresentedFile(path: string | undefined): void {
    this.action("set-represented-file", path ?? "");
  }

  setDocumentEdited(edited: boolean): void {
    this.action("set-document-edited", edited ? "true" : "false");
  }

  setAppearance(appearance: AppearancePreference): void {
    this.action("set-appearance", appearance);
  }

  setBackgroundAppearance(appearance: WindowBackgroundAppearance): void {
    this.action("set-background-appearance", appearance);
  }

  setVibrancy(vibrancy: MacOSVibrancy | undefined): void {
    this.action("set-vibrancy", vibrancy ?? "");
  }

  setVisualEffectState(state: MacOSVisualEffectState): void {
    this.action("set-visual-effect-state", state);
  }

  showCharacterPalette(): void {
    this.action("show-character-palette", undefined);
  }

  setAlwaysOnTop(flag: boolean, level?: WindowLevel | ElectronWindowLevel): void {
    this.action("set-always-on-top", level === undefined ? JSON.stringify({ flag }) : JSON.stringify({ flag, level }));
  }

  moveTop(): void {
    this.action("move-top", undefined);
  }

  moveAbove(other: Window): void {
    if (other === this) throw new RangeError("a window cannot be ordered above itself");
    this.action("move-above", String(other.nativeId));
  }

  setIgnoreMouseEvents(ignore: boolean, forward = false): void {
    this.action("set-ignore-mouse-events", JSON.stringify({ ignore, forward }));
  }

  setEnabled(enabled: boolean): void {
    this.action("set-enabled", enabled ? "true" : "false");
  }

  setAspectRatio(ratio: Size | undefined): void {
    this.action("set-aspect-ratio", ratio === undefined ? undefined : JSON.stringify(ratio));
  }

  setWindowButtonVisibility(visible: boolean): void {
    this.action("set-window-button-visibility", visible ? "true" : "false");
  }

  setResizePolicy(policy: WindowResizePolicy | undefined): void {
    this.action("set-resize-policy", policy === undefined ? undefined : JSON.stringify(policy));
  }

  setMovePolicy(policy: WindowMovePolicy | undefined): void {
    this.action("set-move-policy", policy === undefined ? undefined : JSON.stringify(policy));
  }

  /** Replace this window's native menu declaration, or pass `undefined` to inherit the app menu. */
  setMenu(definitions: MenuDefinition[] | undefined): void {
    const release = this._menuRelease;
    if (release !== undefined) {
      this._menuRelease = undefined;
      release();
    }
    if (definitions === undefined) {
      this.action("set-menu", undefined);
      return;
    }
    const menu = serializeMenuDefinitions(definitions, this.nativeId);
    this._menuRelease = menu.release;
    this.action("set-menu", menu.json);
  }

  setTabbingIdentifier(identifier: string | undefined): void {
    this.action("set-tabbing-identifier", identifier ?? "");
  }

  selectNextTab(): void {
    this.action("select-next-tab", undefined);
  }

  selectPreviousTab(): void {
    this.action("select-previous-tab", undefined);
  }

  selectTab(index: number): void {
    if (!Number.isInteger(index) || index < 0) throw new RangeError("a native tab index must be a non-negative integer");
    this.action("select-tab", String(index));
  }

  mergeAllWindows(): void {
    this.action("merge-all-windows", undefined);
  }

  moveTabToNewWindow(): void {
    this.action("move-tab-to-new-window", undefined);
  }

  toggleTabBar(): void {
    this.action("toggle-tab-bar", undefined);
  }

  toggleTabOverview(): void {
    this.action("toggle-tab-overview", undefined);
  }

  /** Present a native popup menu at `position`, or at the cursor; resolves once it closes. */
  popupMenu(items: MenuDefinition, position: Point | undefined): Promise<void> {
    return popupWindowMenu(this, items, position);
  }
}

interface NativeWindowState {
  displayId?: string;
  kind: string;
  x: number;
  y: number;
  width: number;
  height: number;
  viewportWidth: number;
  viewportHeight: number;
  minimumWidth?: number;
  minimumHeight?: number;
  maximumWidth?: number;
  maximumHeight?: number;
  scaleFactor: number;
  appearance: string;
  backgroundAppearance: string;
  vibrancy?: string;
  visualEffectState: string;
  focused: boolean;
  focusable: boolean;
  visible: boolean;
  minimized: boolean;
  maximized: boolean;
  fullscreen: boolean;
  occluded: boolean;
  movable: boolean;
  resizable: boolean;
  minimizable: boolean;
  maximizable: boolean;
  closable: boolean;
  decorated: boolean;
  shadow: boolean;
  contentProtected: boolean;
  windowLevel: string;
  skipTaskbar: boolean;
  visibleOnAllWorkspaces: boolean;
  opacity: number;
  hasIcon: boolean;
  taskbarProgressState: string;
  taskbarProgress: number;
  hasTaskbarOverlayIcon: boolean;
  cursorVisible: boolean;
  cursorGrab: string;
  cursorHitTest: boolean;
  cursorX?: number;
  cursorY?: number;
  representedFile: boolean;
  documentEdited: boolean;
  nativeTabbing: boolean;
  nativeTabCount: number;
  nativeSelectedTab?: number;
  nativeTabBarVisible: boolean;
  nativeTabOverviewVisible: boolean;
  nativeTabsTruncated: boolean;
}

export interface Rectangle {
  x: number;
  y: number;
  width: number;
  height: number;
}

export interface WindowNativeTabs {
  count: number;
  selectedIndex?: number;
  tabBarVisible: boolean;
  overviewVisible: boolean;
  truncated: boolean;
}

/** A complete snapshot of one window, as the host sees it. */
export interface WindowState {
  displayId?: string;
  kind: string;
  bounds: Rectangle;
  viewportSize: Size;
  minimumSize?: Size;
  maximumSize?: Size;
  scaleFactor: number;
  appearance: string;
  backgroundAppearance: string;
  vibrancy?: string;
  visualEffectState: string;
  focused: boolean;
  focusable: boolean;
  visible: boolean;
  minimized: boolean;
  maximized: boolean;
  fullscreen: boolean;
  occluded: boolean;
  movable: boolean;
  resizable: boolean;
  minimizable: boolean;
  maximizable: boolean;
  closable: boolean;
  decorated: boolean;
  shadow: boolean;
  contentProtected: boolean;
  windowLevel: string;
  skipTaskbar: boolean;
  visibleOnAllWorkspaces: boolean;
  opacity: number;
  hasIcon: boolean;
  taskbarProgressState: string;
  taskbarProgress: number;
  hasTaskbarOverlayIcon: boolean;
  cursorVisible: boolean;
  cursorGrab: string;
  cursorHitTest: boolean;
  cursorPosition?: Point;
  representedFile: boolean;
  documentEdited: boolean;
  nativeTabbing: boolean;
  nativeTabs: WindowNativeTabs;
}

function windowStateFromNative(native: NativeWindowState): WindowState {
  const state: WindowState = {
    kind: native.kind,
    bounds: { x: native.x, y: native.y, width: native.width, height: native.height },
    viewportSize: { width: native.viewportWidth, height: native.viewportHeight },
    scaleFactor: native.scaleFactor,
    appearance: native.appearance,
    backgroundAppearance: native.backgroundAppearance,
    visualEffectState: native.visualEffectState,
    focused: native.focused,
    focusable: native.focusable,
    visible: native.visible,
    minimized: native.minimized,
    maximized: native.maximized,
    fullscreen: native.fullscreen,
    occluded: native.occluded,
    movable: native.movable,
    resizable: native.resizable,
    minimizable: native.minimizable,
    maximizable: native.maximizable,
    closable: native.closable,
    decorated: native.decorated,
    shadow: native.shadow,
    contentProtected: native.contentProtected,
    windowLevel: native.windowLevel,
    skipTaskbar: native.skipTaskbar,
    visibleOnAllWorkspaces: native.visibleOnAllWorkspaces,
    opacity: native.opacity,
    hasIcon: native.hasIcon,
    taskbarProgressState: native.taskbarProgressState,
    taskbarProgress: native.taskbarProgress,
    hasTaskbarOverlayIcon: native.hasTaskbarOverlayIcon,
    cursorVisible: native.cursorVisible,
    cursorGrab: native.cursorGrab,
    cursorHitTest: native.cursorHitTest,
    representedFile: native.representedFile,
    documentEdited: native.documentEdited,
    nativeTabbing: native.nativeTabbing,
    nativeTabs: {
      count: native.nativeTabCount,
      tabBarVisible: native.nativeTabBarVisible,
      overviewVisible: native.nativeTabOverviewVisible,
      truncated: native.nativeTabsTruncated,
    },
  };
  if (native.displayId !== undefined) state.displayId = native.displayId;
  if (native.minimumWidth !== undefined && native.minimumHeight !== undefined) {
    state.minimumSize = { width: native.minimumWidth, height: native.minimumHeight };
  }
  if (native.maximumWidth !== undefined && native.maximumHeight !== undefined) {
    state.maximumSize = { width: native.maximumWidth, height: native.maximumHeight };
  }
  if (native.vibrancy !== undefined) state.vibrancy = native.vibrancy;
  if (native.cursorX !== undefined && native.cursorY !== undefined) {
    state.cursorPosition = { x: native.cursorX, y: native.cursorY };
  }
  if (native.nativeSelectedTab !== undefined) state.nativeTabs.selectedIndex = native.nativeSelectedTab;
  return state;
}

/** Present one native popup menu owned by a window; resolves when it closes. */
export async function popupWindowMenu(window: Window, items: MenuDefinition, position: Point | undefined): Promise<void> {
  const menu = serializeMenuDefinitions([items], window.nativeId);
  const request = allocateRequest();
  const json = position === undefined
    ? JSON.stringify({ method: "window-popup-menu", request, window: window.nativeId, menu: menu.json })
    : JSON.stringify({ method: "window-popup-menu", request, window: window.nativeId, menu: menu.json, x: position.x, y: position.y });
  try {
    await sendRequest(request, json);
  } finally {
    menu.release();
  }
}

// ---------------------------------------------------------------------------
// Dialogs
// ---------------------------------------------------------------------------

class DialogEvent {
  readonly value: string | undefined;
  readonly paths: string[] | undefined;

  constructor(value: string | undefined, paths: string[] | undefined) {
    this.value = value;
    this.paths = paths;
  }
}

class PendingDialog {
  readonly complete: (event: DialogEvent) => void;
  readonly reject: (error: Error) => void;

  constructor(complete: (event: DialogEvent) => void, reject: (error: Error) => void) {
    this.complete = complete;
    this.reject = reject;
  }
}

export type AlertDialogLevel = "info" | "warning" | "critical";
export type AlertDialogButtonRole = "default" | "cancel" | "other";

export interface AlertDialogButton {
  label: string;
  role?: AlertDialogButtonRole;
}

export interface AlertDialogOptions {
  level?: AlertDialogLevel;
  message: string;
  detail?: string;
  /**
   * Defaults to one system-styled OK button. macOS accepts up to 16 buttons; the portable
   * Windows/Linux/BSD backend accepts up to three uniquely labelled buttons.
   */
  buttons?: (string | AlertDialogButton)[];
}

export interface FileDialogFilter {
  name: string;
  /** File extensions without a leading dot. Use `*` to match every file. */
  extensions: string[];
}

export type OpenDialogProperty = "openFile" | "openDirectory" | "multiSelections" | "showHiddenFiles";

export interface OpenDialogOptions {
  title?: string;
  /** Initial file or directory. */
  defaultPath?: string;
  filters?: FileDialogFilter[];
  /** Custom open-button text. Currently supported by the macOS backend. */
  buttonLabel?: string;
  /**
   * Defaults to `["openFile"]`. `showHiddenFiles` can currently be forced only on macOS;
   * Windows/Linux otherwise follow the user's file-picker preference.
   */
  properties?: OpenDialogProperty[];
}

export interface OpenDialogResult {
  canceled: boolean;
  filePaths: string[];
}

export interface SaveDialogOptions {
  title?: string;
  /** Initial directory or complete suggested file path. */
  defaultPath?: string;
  filters?: FileDialogFilter[];
  /** Custom save-button text. Currently supported by the macOS backend. */
  buttonLabel?: string;
  /** Currently supported by the macOS backend. */
  showHiddenFiles?: boolean;
}

export interface SaveDialogResult {
  canceled: boolean;
  filePath?: string;
}

interface NativeDialogOptions {
  level?: string;
  message: string;
  detail?: string;
  buttons: AlertDialogButton[];
}

interface NativeOpenDialogOptions {
  files: boolean;
  directories: boolean;
  multiple: boolean;
  title?: string;
  prompt?: string;
  directory?: string;
  suggestedName?: string;
  filters: FileDialogFilter[];
  showsHiddenFiles: boolean;
}

interface NativeSaveDialogOptions {
  directory: string;
  title?: string;
  suggestedName?: string;
  prompt?: string;
  filters: FileDialogFilter[];
  showsHiddenFiles: boolean;
}

function validateFilters(filters: FileDialogFilter[] | undefined): FileDialogFilter[] {
  if (filters === undefined) return [];
  for (const filter of filters) {
    if (filter.name.length === 0 || filter.extensions.length === 0) {
      throw new TypeError("file dialog filters require a name and at least one extension");
    }
    for (const extension of filter.extensions) {
      if (extension.length === 0 || extension.startsWith(".") || extension.includes("/") || extension.includes("\\")) {
        throw new TypeError("file dialog filter extensions must be nonempty and omit dots and path separators");
      }
    }
  }
  return filters;
}

function requireApp(): App {
  const application = activeApp;
  if (application === undefined) throw new Error("create a QuickGUI App before showing a dialog");
  return application;
}

/** Present a native alert; resolves with the zero-based index of the chosen button. */
export function showAlertDialog(options: AlertDialogOptions, window?: Window): Promise<number> {
  const buttons: AlertDialogButton[] = [];
  if (options.buttons === undefined) buttons.push({ label: "OK", role: "default" });
  else {
    for (const button of options.buttons) {
      if (typeof button === "string") buttons.push({ label: button });
      else buttons.push(button);
    }
  }
  const native: NativeDialogOptions = { message: options.message, buttons };
  if (options.level !== undefined) native.level = options.level;
  if (options.detail !== undefined) native.detail = options.detail;
  const json = JSON.stringify(native);
  return new Promise<number>((resolve, reject) => {
    requireApp()._showDialog(
      window,
      0,
      json,
      (event) => {
        resolve(Number(event.value ?? "0"));
      },
      reject,
    );
  });
}

/** Present a native open panel with Electron-shaped results. */
/** Split an initial path into the directory to open and, for a file path, the name to suggest. */
function applyDefaultPath(defaultPath: string, apply: (directory: string, suggestedName: string | undefined) => void): void {
  const resolved = resolve(defaultPath);
  if (defaultPath.endsWith(sep) || isExistingDirectory(resolved)) apply(resolved, undefined);
  else apply(dirname(resolved), basename(resolved));
}

function isExistingDirectory(path: string): boolean {
  try {
    return statSync(path).isDirectory();
  } catch {
    return false;
  }
}

export function showOpenDialog(options: OpenDialogOptions, window?: Window): Promise<OpenDialogResult> {
  const properties = options.properties ?? ["openFile"];
  const files = properties.includes("openFile");
  const directories = properties.includes("openDirectory");
  if (!files && !directories) throw new TypeError("showOpenDialog properties must include openFile or openDirectory");
  if (files && directories && process.platform !== "darwin") {
    throw new TypeError("showOpenDialog cannot combine openFile and openDirectory on this platform");
  }
  const native: NativeOpenDialogOptions = {
    files,
    directories,
    multiple: properties.includes("multiSelections"),
    filters: validateFilters(options.filters),
    showsHiddenFiles: properties.includes("showHiddenFiles"),
  };
  if (options.title !== undefined) native.title = options.title;
  if (options.buttonLabel !== undefined) native.prompt = options.buttonLabel;
  if (options.defaultPath !== undefined) {
    applyDefaultPath(options.defaultPath, (directory, suggestedName) => {
      native.directory = directory;
      if (suggestedName !== undefined) native.suggestedName = suggestedName;
    });
  }
  const json = JSON.stringify(native);
  return new Promise<OpenDialogResult>((resolve, reject) => {
    requireApp()._showDialog(
      window,
      1,
      json,
      (event) => {
        const paths = event.paths;
        resolve(paths === undefined ? { canceled: true, filePaths: [] } : { canceled: false, filePaths: paths });
      },
      reject,
    );
  });
}

/** Present a native save panel; choosing a destination never writes the file. */
export function showSaveDialog(options: SaveDialogOptions, window?: Window): Promise<SaveDialogResult> {
  const native: NativeSaveDialogOptions = {
    directory: process.cwd(),
    filters: validateFilters(options.filters),
    showsHiddenFiles: options.showHiddenFiles ?? false,
  };
  if (options.title !== undefined) native.title = options.title;
  if (options.buttonLabel !== undefined) native.prompt = options.buttonLabel;
  if (options.defaultPath !== undefined) {
    applyDefaultPath(options.defaultPath, (directory, suggestedName) => {
      native.directory = directory;
      if (suggestedName !== undefined) native.suggestedName = suggestedName;
    });
  }
  const json = JSON.stringify(native);
  return new Promise<SaveDialogResult>((resolve, reject) => {
    requireApp()._showDialog(
      window,
      2,
      json,
      (event) => {
        const path = event.value;
        resolve(path === undefined ? { canceled: true } : { canceled: false, filePath: path });
      },
      reject,
    );
  });
}

/** Platform-native dialogs. Pass a Window second to attach the dialog; omit it for app-modal UI. */
export class Dialog {
  static showAlertDialog(options: AlertDialogOptions, window?: Window): Promise<number> {
    return showAlertDialog(options, window);
  }

  static showOpenDialog(options?: OpenDialogOptions, window?: Window): Promise<OpenDialogResult> {
    return showOpenDialog(options ?? {}, window);
  }

  static showSaveDialog(options?: SaveDialogOptions, window?: Window): Promise<SaveDialogResult> {
    return showSaveDialog(options ?? {}, window);
  }
}

export const app = new App();
activeApp = app;
