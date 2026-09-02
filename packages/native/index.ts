import * as binding from "./binding.js";
import { Buffer } from "node:buffer";
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
  nativeImageSource,
  onNativeWindowStateChange,
  performNativeWindowAction,
  performNativeWindowImageAction,
  rejectPendingSystemRequests,
  removeNativeWindowStateListeners,
  serializeNativeMenu,
} from "./system.ts";
import {
  type SecondInstanceEvent,
  urlsFromArguments,
} from "./single-instance.ts";

export { NativeNodeTag, PropertyCode } from "./protocol.ts";
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
  Desktop,
  GlobalShortcut,
  Keyboard,
  Menu,
  Notifications,
  Permissions,
  PowerMonitor,
  PowerAssertion,
  Screen,
  SystemPreferences,
  Shell,
  Tray,
  TrayIcon,
} from "./system.ts";
export { AutoStart, SecureStorage, Updater } from "./integrations.ts";
export { CrashReporter, Metrics } from "./integrations.ts";
export type { CrashBacktracePolicy, CrashKind, CrashLocation, CrashReport, CrashReporterOptions, CrashUploadSummary, CpuSampler, CpuUsage, ProcessMetrics, SystemMemory, UpdateProgress, UpdateProgressPhase, UpdateStageOptions } from "./integrations.ts";
export { DeepLink } from "./single-instance.ts";
export type { SecondInstanceEvent } from "./single-instance.ts";
export type {
  AutoStartMode,
  AutoStartOptions,
  AvailableUpdate,
  InstalledUpdate,
  ProtocolRegistrationOptions,
  UpdateClientOptions,
  UpdateInstallOptions,
} from "./integrations.ts";
export type {
  AppearanceMode,
  AppearancePreference,
  ClipboardEntry,
  ClipboardBookmarkEntry,
  ClipboardDataEntry,
  ClipboardFilesEntry,
  ClipboardImageEntry,
  ClipboardItem,
  ClipboardTextEntry,
  Display,
  DesktopIntegrationSupport,
  AboutPanelOptions,
  UserTask,
  NativeImage,
  GlobalShortcutListener,
  KeyboardLayout,
  MenuActionItem,
  MenuDefinition,
  MenuItem,
  MenuRole,
  MenuItemMark,
  MenuRoleItem,
  MenuSystemItem,
  MenuSeparatorItem,
  MenuServicesItem,
  MenuSubmenuItem,
  NotificationAction,
  NotificationOptions,
  NotificationAttachment,
  NotificationPermissionStatus,
  NotificationResponse,
  PermissionKind,
  PermissionStatus,
  PowerEvent,
  PowerEventType,
  PowerAssertionKind,
  PowerSource,
  PowerState,
  BatteryStatus,
  ThermalState,
  SessionState,
  IdleState,
  SystemColor,
  SystemPreferencesSnapshot,
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
  WindowBackgroundAppearance,
  MacOSVibrancy,
  MacOSVisualEffectState,
  WindowKind,
  WindowLevel,
  CursorGrabMode,
  TaskbarProgressState,
  ImageSource,
} from "./system.ts";
import type {
  AppearancePreference,
  CursorGrabMode,
  ImageSource,
  KeyboardLayout,
  MenuDefinition,
  NotificationResponse,
  TaskbarProgressState,
  WindowBackgroundAppearance,
  MacOSVibrancy,
  MacOSVisualEffectState,
  WindowKind,
  WindowLevel,
  WindowState,
} from "./system.ts";

export type WindowCloseListener = (window: Window) => void;
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
export interface Size {
  width: number;
  height: number;
}
export interface Point {
  x: number;
  y: number;
}
export interface WindowBounds extends Point, Size {
  state?: InitialWindowState;
}

export interface WindowOptions {
  renderer: WindowRenderer;
  title?: string;
  width?: number;
  height?: number;
  position?: Point;
  initialState?: InitialWindowState;
  displayId?: string;
  /** Pass `null` to remove the core's default minimum window size. */
  minimumSize?: Size | null;
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
  /** Electron-compatible macOS `NSVisualEffectView` semantic material. */
  vibrancy?: MacOSVibrancy;
  /** Defaults to `followWindow`. Used when `vibrancy` is enabled. */
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
  taskbarProgress?: { state: TaskbarProgressState; progress: number };
  taskbarOverlay?: { icon: ImageSource; description: string };
  cursorVisible?: boolean;
  cursorGrab?: CursorGrabMode;
  cursorHitTest?: boolean;
  cursorPosition?: Point;
  menu?: readonly MenuDefinition[];
  lineScrollPixels?: number;
  keySequenceTimeoutMs?: number;
  reduceMotion?: boolean;
  trafficLightPosition?: { x: number; y: number };
  transparent?: boolean;
  blur?: boolean;
  /** Open this window as a system popover anchored to the mounted node. */
  anchor?: NativeNode;
  placement?: PopoverPlacement;
  gap?: number;
  offset?: { x: number; y: number };
  viewportMargin?: number;
  dismissOnEscape?: boolean;
  dismissOnPointerOutside?: boolean;
  grab?: boolean;
  acceptsKeyFocus?: boolean;
}

export interface RunOptions {
  /** Pump interval used only when running a source file outside the QuickGUI CLI host. */
  sliceMs?: number;
}

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
  fonts?: readonly Uint8Array[];
}

export interface RelaunchOptions {
  executable?: string;
  /** Omit to preserve current arguments; pass `null` to relaunch without arguments. */
  arguments?: readonly string[] | null;
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
  operatingSystem:
    | "macos"
    | "windows"
    | "linux"
    | "freebsd"
    | "dragonfly"
    | "netbsd"
    | "openbsd"
    | "android"
    | "ios"
    | "wasm"
    | "other";
  family: "unix" | "windows" | "wasm" | "other";
  name: string;
  version?: string;
  edition?: string;
  codename?: string;
  architecture: string;
  bitness: "32" | "64" | "unknown";
  hostname?: string;
  locale?: string;
  preferredLanguages: readonly string[];
  languagesTruncated: boolean;
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

function nativeAppOptions(options: AppOptions): binding.NativeAppOptions {
  const native: binding.NativeAppOptions = {};
  if (options.name !== undefined) native.name = options.name;
  if (options.version !== undefined) native.version = options.version;
  if (options.identifier !== undefined) native.identifier = options.identifier;
  if (options.quitMode !== undefined) native.quitMode = options.quitMode;
  if (options.fonts !== undefined) {
    native.fontData = options.fonts.map((font) => Buffer.from(font));
  }
  const paths = options.paths;
  if (paths?.resourceDir !== undefined) native.resourceDir = paths.resourceDir;
  if (paths?.configDir !== undefined) native.configDir = paths.configDir;
  if (paths?.dataDir !== undefined) native.dataDir = paths.dataDir;
  if (paths?.localDataDir !== undefined)
    native.localDataDir = paths.localDataDir;
  if (paths?.cacheDir !== undefined) native.cacheDir = paths.cacheDir;
  if (paths?.logDir !== undefined) native.logDir = paths.logDir;
  if (paths?.runtimeDir !== undefined) native.runtimeDir = paths.runtimeDir;
  if (paths?.tempDir !== undefined) native.tempDir = paths.tempDir;
  return native;
}

const embeddedAppOptions = (
  globalThis as typeof globalThis & { __QUICKGUI_APP_OPTIONS__?: AppOptions }
).__QUICKGUI_APP_OPTIONS__;

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
    if (!app)
      throw new Error("create a QuickGUI App before using a native system API");
    app._assertReady();
    return { appId: app.nativeId, hosted: hostedRuntime };
  },
  (window) => {
    const app = window?.app ?? activeApp;
    if (!app)
      throw new Error("create a QuickGUI App before using a native window API");
    const resolved = window ?? app.windows.values().next().value;
    if (
      !resolved ||
      resolved.closed ||
      app.windows.get(resolved.nativeId) !== resolved
    ) {
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
  #ready = false;
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
      throw new Error(
        "a QuickGUI App is already active in this JavaScript isolate",
      );
    }
    const initialOptions = embeddedAppOptions
      ? nativeAppOptions(embeddedAppOptions)
      : undefined;
    this.nativeId = hostedRuntime
      ? binding.createHostedApp(initialOptions)
      : binding.createApp(initialOptions);
    activeApp = this;
    this.#readyPromise = Promise.resolve().then(async () => {
      this.#assertAlive();
      if (hostedRuntime) await binding.prepareHostedApp(this.nativeId);
      else binding.prepareApp(this.nativeId);
      if (!hostedRuntime && !binding.isAppReady(this.nativeId)) {
        throw new Error("the native QuickGUI application did not become ready");
      }
      this.#ready = true;
      this.#emitAppEvent("ready", undefined);
    });
  }

  isReady(): boolean {
    this.#assertAlive();
    return this.#ready;
  }

  whenReady(): Promise<void> {
    return this.#readyPromise;
  }

  /** Patch core-owned identity, paths, and quit policy before the first readiness turn. */
  async configure(options: AppOptions): Promise<void> {
    this.#assertAlive();
    const native = nativeAppOptions(options);
    if (hostedRuntime) await binding.configureHostedApp(this.nativeId, native);
    else binding.configureApp(this.nativeId, native);
  }

  async getInfo(): Promise<AppInfo | undefined> {
    this._assertReady();
    const info = hostedRuntime
      ? await binding.getHostedAppInfo(this.nativeId)
      : binding.getAppInfo(this.nativeId);
    return info ? { ...info } : undefined;
  }

  async getPaths(): Promise<AppPaths | undefined> {
    this._assertReady();
    const paths = hostedRuntime
      ? await binding.getHostedAppPaths(this.nativeId)
      : binding.getAppPaths(this.nativeId);
    if (!paths) return undefined;
    const result: AppPaths = {
      executable: paths.executable,
      executableDir: paths.executableDir,
      resourceDir: paths.resourceDir,
      tempDir: paths.tempDir,
    };
    if (paths.homeDir !== undefined) result.homeDir = paths.homeDir;
    if (paths.configDir !== undefined) result.configDir = paths.configDir;
    if (paths.dataDir !== undefined) result.dataDir = paths.dataDir;
    if (paths.localDataDir !== undefined)
      result.localDataDir = paths.localDataDir;
    if (paths.cacheDir !== undefined) result.cacheDir = paths.cacheDir;
    if (paths.logDir !== undefined) result.logDir = paths.logDir;
    if (paths.runtimeDir !== undefined) result.runtimeDir = paths.runtimeDir;
    if (paths.audioDir !== undefined) result.audioDir = paths.audioDir;
    if (paths.desktopDir !== undefined) result.desktopDir = paths.desktopDir;
    if (paths.documentDir !== undefined) result.documentDir = paths.documentDir;
    if (paths.downloadDir !== undefined) result.downloadDir = paths.downloadDir;
    if (paths.pictureDir !== undefined) result.pictureDir = paths.pictureDir;
    if (paths.videoDir !== undefined) result.videoDir = paths.videoDir;
    return result;
  }

  async getSystemInfo(): Promise<SystemInfo> {
    this._assertReady();
    const info = hostedRuntime
      ? await binding.getHostedSystemInfo(this.nativeId)
      : binding.getSystemInfo(this.nativeId);
    const result: SystemInfo = {
      operatingSystem: info.operatingSystem as SystemInfo["operatingSystem"],
      family: info.family as SystemInfo["family"],
      name: info.name,
      architecture: info.architecture,
      bitness: info.bitness as SystemInfo["bitness"],
      preferredLanguages: [...info.preferredLanguages],
      languagesTruncated: info.languagesTruncated,
    };
    if (info.version !== undefined) result.version = info.version;
    if (info.edition !== undefined) result.edition = info.edition;
    if (info.codename !== undefined) result.codename = info.codename;
    if (info.hostname !== undefined) result.hostname = info.hostname;
    if (info.locale !== undefined) result.locale = info.locale;
    return result;
  }

  getWindows(): readonly Window[] {
    this._assertReady();
    return [...this.windows.values()];
  }

  async getActiveWindow(): Promise<Window | undefined> {
    this._assertReady();
    const registry = hostedRuntime
      ? await binding.getHostedWindowRegistry(this.nativeId)
      : binding.getWindowRegistry(this.nativeId);
    return registry.activeWindow === undefined
      ? undefined
      : this.windows.get(registry.activeWindow);
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
      const systemEvent = dispatchSystemEvent(event, (id) =>
        this.windows.get(id),
      );
      const appEvent = this.#dispatchAppEvent(event);
      if (systemEvent || appEvent) continue;
      const window = this.windows.get(event.window);
      if (!window) continue;
      if (event.kind === "close") {
        this._didCloseWindow(window);
      } else {
        window._dispatchEvent(
          event.kind as NativeEventType,
          event.target,
          event.value,
        );
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
      if (exitCode === undefined)
        throw new Error("the QuickGUI app exited without a status code");
      await this.#emitAppEventAndWait("quit", { exitCode });
      return exitCode;
    } finally {
      this.#running = false;
      await this.releaseSingleInstanceLock();
    }
  }

  async requestSingleInstanceLock(identifier: string): Promise<boolean> {
    this.#assertAlive();
    if (this.#singleInstanceIdentifier) {
      if (this.#singleInstanceIdentifier !== identifier) {
        throw new Error(
          "this QuickGUI app already owns a different single-instance lock",
        );
      }
      return true;
    }
    this._assertReady();
    const acquired = hostedRuntime
      ? await binding.requestHostedSingleInstanceLock(this.nativeId, identifier)
      : binding.requestSingleInstanceLock(this.nativeId, identifier);
    if (acquired) this.#singleInstanceIdentifier = identifier;
    return acquired;
  }

  async releaseSingleInstanceLock(): Promise<boolean> {
    if (!this.#singleInstanceIdentifier || this.#destroyed) return false;
    const released = hostedRuntime
      ? await binding.releaseHostedSingleInstanceLock(this.nativeId)
      : binding.releaseSingleInstanceLock(this.nativeId);
    if (released) this.#singleInstanceIdentifier = undefined;
    return released;
  }

  /** Request an orderly native shutdown. Returns false after shutdown already began. */
  async quit(): Promise<boolean> {
    this.#assertAlive();
    this._assertReady();
    return hostedRuntime
      ? await binding.exitHostedApp(this.nativeId)
      : binding.exitApp(this.nativeId);
  }

  /** Schedule a replacement process after ordinary child-first native teardown. */
  async relaunch(options: RelaunchOptions = {}): Promise<boolean> {
    this.#assertAlive();
    this._assertReady();
    const native: binding.NativeRelaunchOptions = {};
    if (options.executable !== undefined)
      native.executable = options.executable;
    if (options.arguments === null) native.clearArguments = true;
    else if (options.arguments !== undefined)
      native.arguments = [...options.arguments];
    if (options.workingDirectory !== undefined) {
      native.workingDirectory = options.workingDirectory;
    }
    return hostedRuntime
      ? await binding.relaunchHostedApp(this.nativeId, native)
      : binding.relaunchApp(this.nativeId, native);
  }

  destroy(): void {
    if (this.#destroyed) return;
    const error = new Error("the QuickGUI app was destroyed");
    this.#rejectDialogs(undefined, error);
    rejectPendingSystemRequests(error);
    void this.releaseSingleInstanceLock().catch(() => {});
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
        payload =
          Array.isArray(parsed) &&
          parsed.every((url) => typeof url === "string")
            ? parsed
            : [];
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
      const layout = hostedRuntime
        ? binding.getHostedKeyboardLayout(this.nativeId)
        : Promise.resolve(binding.getKeyboardLayout(this.nativeId));
      void layout
        .then((value) => {
          if (this.#destroyed) return;
          this.#emitAppEvent("keyboardLayoutChange", {
            id: value.id,
            name: value.name,
          });
        })
        .catch(() => {});
      return true;
    } else if (event.kind === "notification-response") {
      type = "notificationResponse";
      try {
        const parsed = JSON.parse(event.value ?? "{}") as {
          tag?: unknown;
          actionId?: unknown;
          reply?: unknown;
        };
        if (typeof parsed.tag !== "string") return true;
        const response: NotificationResponse = { tag: parsed.tag };
        if (typeof parsed.actionId === "string")
          response.actionId = parsed.actionId;
        if (typeof parsed.reply === "string") response.reply = parsed.reply;
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

  #emitAppEvent<K extends keyof AppEventMap>(
    type: K,
    payload: AppEventMap[K],
  ): void {
    for (const listener of this.#appEventListeners.get(type) ?? [])
      listener(payload);
  }

  async #emitAppEventAndWait<K extends keyof AppEventMap>(
    type: K,
    payload: AppEventMap[K],
  ): Promise<void> {
    await Promise.all(
      [...(this.#appEventListeners.get(type) ?? [])].map((listener) =>
        listener(payload),
      ),
    );
  }

  _registerWindow(window: Window): void {
    this.#assertAlive();
    this.windows.set(window.nativeId, window);
  }

  _assertReady(): void {
    this.#assertAlive();
    if (!this.isReady()) {
      throw new Error(
        "await app.whenReady() before using the native QuickGUI application",
      );
    }
  }

  _closeWindow(window: Window): void {
    if (this.#destroyed || window.closed) return;
    if (hostedRuntime) {
      binding.closeHostedWindow(this.nativeId, window.nativeId);
    } else if (binding.closeWindow(this.nativeId, window.nativeId)) {
      this._didCloseWindow(window);
    }
  }

  _didCloseWindow(window: Window): void {
    if (this.windows.get(window.nativeId) !== window) return;
    this.windows.delete(window.nativeId);
    this.#rejectDialogs(
      window,
      new Error("the native dialog's owner window closed"),
    );
    window._didClose();
  }

  _showAlertDialog(
    window: Window | undefined,
    options: AlertDialogOptions,
  ): Promise<number> {
    try {
      const nativeOptions = normalizeAlertDialogOptions(options);
      return this.#requestDialog(
        window,
        "alert-dialog",
        (request) => {
          if (hostedRuntime) {
            binding.showHostedAlertDialog(
              this.nativeId,
              window?.nativeId,
              request,
              nativeOptions,
            );
          } else {
            binding.showAlertDialog(
              this.nativeId,
              window?.nativeId,
              request,
              nativeOptions,
            );
          }
        },
        (event) => {
          const response = Number(event.value);
          if (!Number.isSafeInteger(response) || response < 0) {
            throw new Error(
              "the native dialog returned an invalid button index",
            );
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
            binding.showHostedOpenDialog(
              this.nativeId,
              window?.nativeId,
              request,
              nativeOptions,
            );
          } else {
            binding.showOpenDialog(
              this.nativeId,
              window?.nativeId,
              request,
              nativeOptions,
            );
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
            binding.showHostedSaveDialog(
              this.nativeId,
              window?.nativeId,
              request,
              nativeOptions,
            );
          } else {
            binding.showSaveDialog(
              this.nativeId,
              window?.nativeId,
              request,
              nativeOptions,
            );
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
    invoke: (request: number) => void | Promise<void>,
    result: (event: binding.NativeEvent) => T,
  ): Promise<T> {
    this.#assertAlive();
    if (
      window &&
      (window.closed || this.windows.get(window.nativeId) !== window)
    ) {
      return Promise.reject(
        new Error("the native dialog parent must be an open window"),
      );
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
        void Promise.resolve(invoke(request)).catch((error) => {
          if (!this.#pendingDialogs.delete(request)) return;
          reject(asError(error));
        });
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
      pending.reject(
        new Error("the native dialog response had the wrong owner window"),
      );
      return;
    }
    if (pending.kind !== event.kind) {
      pending.reject(
        new Error("the native dialog response had the wrong response type"),
      );
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
    if (this.#destroyed)
      throw new Error("this QuickGUI app has been destroyed");
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
  readonly #nativeReadyCallbacks = new Set<() => void>();
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

  /** @internal Create one retained renderer whose native view is owned by a SwiftUI host. */
  static _createEmbedded(
    owner: Window,
    options: WindowOptions,
    matchContents: { horizontal: boolean; vertical: boolean },
  ): Window {
    if (owner.closed) {
      throw new Error(
        "an embedded QuickGUI view requires an open owner Window",
      );
    }
    owner.flush();
    return new Window(options, { owner, matchContents });
  }

  constructor(
    options: WindowOptions,
    embedded?: {
      owner: Window;
      matchContents: { horizontal: boolean; vertical: boolean };
    },
  ) {
    const app = activeApp;
    if (!app) throw new Error("the QuickGUI app is unavailable");
    if (!app.isReady()) {
      throw new Error(
        "await app.whenReady() before creating a QuickGUI Window",
      );
    }
    const nativeOptions: binding.NativeWindowOptions = {};
    const serializedMenu = options.menu
      ? serializeNativeMenu(options.menu)
      : undefined;
    if (options.title !== undefined) nativeOptions.title = options.title;
    if (options.width !== undefined) nativeOptions.width = options.width;
    if (options.height !== undefined) nativeOptions.height = options.height;
    if (
      options.minimumSize !== undefined &&
      (options.minimumWidth !== undefined ||
        options.minimumHeight !== undefined)
    ) {
      throw new TypeError(
        "minimumSize cannot be combined with minimumWidth or minimumHeight",
      );
    }
    if (options.minimumSize === null) {
      nativeOptions.minimumSizeEnabled = false;
    } else if (options.minimumSize !== undefined) {
      nativeOptions.minimumWidth = options.minimumSize.width;
      nativeOptions.minimumHeight = options.minimumSize.height;
    } else {
      if (options.minimumWidth !== undefined)
        nativeOptions.minimumWidth = options.minimumWidth;
      if (options.minimumHeight !== undefined)
        nativeOptions.minimumHeight = options.minimumHeight;
    }
    if (
      options.maximumSize !== undefined &&
      (options.maximumWidth !== undefined ||
        options.maximumHeight !== undefined)
    ) {
      throw new TypeError(
        "maximumSize cannot be combined with maximumWidth or maximumHeight",
      );
    }
    if (options.maximumSize !== undefined) {
      nativeOptions.maximumWidth = options.maximumSize.width;
      nativeOptions.maximumHeight = options.maximumSize.height;
    } else {
      if (options.maximumWidth !== undefined)
        nativeOptions.maximumWidth = options.maximumWidth;
      if (options.maximumHeight !== undefined)
        nativeOptions.maximumHeight = options.maximumHeight;
    }
    if (options.position !== undefined) {
      nativeOptions.x = options.position.x;
      nativeOptions.y = options.position.y;
    }
    if (options.initialState !== undefined)
      nativeOptions.initialState = options.initialState;
    if (options.displayId !== undefined)
      nativeOptions.displayId = options.displayId;
    if (options.representedFile !== undefined)
      nativeOptions.representedFile = options.representedFile;
    if (options.documentEdited !== undefined)
      nativeOptions.documentEdited = options.documentEdited;
    if (options.tabbingIdentifier !== undefined) {
      nativeOptions.tabbingIdentifier = options.tabbingIdentifier;
    }
    if (options.background !== undefined)
      nativeOptions.background = parseColor(options.background);
    if (options.performanceProfile !== undefined) {
      nativeOptions.performanceProfile = options.performanceProfile;
    }
    if (options.appearance !== undefined)
      nativeOptions.appearance = options.appearance;
    if (options.vibrancy !== undefined)
      nativeOptions.vibrancy = options.vibrancy;
    if (options.visualEffectState !== undefined)
      nativeOptions.visualEffectState = options.visualEffectState;
    if (options.titleBarStyle !== undefined)
      nativeOptions.titleBarStyle = options.titleBarStyle;
    if (options.kind !== undefined) nativeOptions.kind = options.kind;
    if (options.focus !== undefined) nativeOptions.focus = options.focus;
    if (options.focusable !== undefined)
      nativeOptions.focusable = options.focusable;
    if (options.visible !== undefined) nativeOptions.show = options.visible;
    if (options.movable !== undefined) nativeOptions.movable = options.movable;
    if (options.resizable !== undefined)
      nativeOptions.resizable = options.resizable;
    if (options.minimizable !== undefined)
      nativeOptions.minimizable = options.minimizable;
    if (options.maximizable !== undefined)
      nativeOptions.maximizable = options.maximizable;
    if (options.closable !== undefined)
      nativeOptions.closable = options.closable;
    if (options.decorated !== undefined)
      nativeOptions.decorated = options.decorated;
    if (options.shadow !== undefined) nativeOptions.shadow = options.shadow;
    if (options.contentProtected !== undefined) {
      nativeOptions.contentProtected = options.contentProtected;
    }
    if (options.windowLevel !== undefined)
      nativeOptions.windowLevel = options.windowLevel;
    if (options.skipTaskbar !== undefined)
      nativeOptions.skipTaskbar = options.skipTaskbar;
    if (options.visibleOnAllWorkspaces !== undefined) {
      nativeOptions.visibleOnAllWorkspaces = options.visibleOnAllWorkspaces;
    }
    if (options.opacity !== undefined) nativeOptions.opacity = options.opacity;
    if (options.icon !== undefined)
      nativeOptions.icon = nativeImageSource(options.icon);
    if (options.taskbarProgress !== undefined) {
      nativeOptions.taskbarProgressState = options.taskbarProgress.state;
      nativeOptions.taskbarProgress = options.taskbarProgress.progress;
    }
    if (options.taskbarOverlay !== undefined) {
      nativeOptions.taskbarOverlayIcon = nativeImageSource(
        options.taskbarOverlay.icon,
      );
      nativeOptions.taskbarOverlayDescription =
        options.taskbarOverlay.description;
    }
    if (options.cursorVisible !== undefined)
      nativeOptions.cursorVisible = options.cursorVisible;
    if (options.cursorGrab !== undefined)
      nativeOptions.cursorGrab = options.cursorGrab;
    if (options.cursorHitTest !== undefined)
      nativeOptions.cursorHitTest = options.cursorHitTest;
    if (options.cursorPosition !== undefined) {
      nativeOptions.cursorX = options.cursorPosition.x;
      nativeOptions.cursorY = options.cursorPosition.y;
    }
    if (serializedMenu !== undefined) nativeOptions.menu = serializedMenu.json;
    if (options.lineScrollPixels !== undefined) {
      nativeOptions.lineScrollPixels = options.lineScrollPixels;
    }
    if (options.keySequenceTimeoutMs !== undefined) {
      nativeOptions.keySequenceTimeoutMs = options.keySequenceTimeoutMs;
    }
    if (options.reduceMotion !== undefined)
      nativeOptions.reduceMotion = options.reduceMotion;
    if (options.trafficLightPosition !== undefined) {
      nativeOptions.trafficLightX = options.trafficLightPosition.x;
      nativeOptions.trafficLightY = options.trafficLightPosition.y;
    }
    if (options.backgroundAppearance !== undefined) {
      nativeOptions.transparent =
        options.backgroundAppearance === "transparent";
      nativeOptions.blur = options.backgroundAppearance === "blurred";
    } else {
      if (options.transparent !== undefined)
        nativeOptions.transparent = options.transparent;
      if (options.blur !== undefined) nativeOptions.blur = options.blur;
    }
    if (options.placement !== undefined)
      nativeOptions.popoverPlacement = options.placement;
    if (options.gap !== undefined) nativeOptions.popoverGap = options.gap;
    if (options.offset !== undefined) {
      nativeOptions.popoverOffsetX = options.offset.x;
      nativeOptions.popoverOffsetY = options.offset.y;
    }
    if (options.viewportMargin !== undefined) {
      nativeOptions.popoverViewportMargin = options.viewportMargin;
    }
    if (options.dismissOnEscape !== undefined) {
      nativeOptions.popoverDismissOnEscape = options.dismissOnEscape;
    }
    if (options.dismissOnPointerOutside !== undefined) {
      nativeOptions.popoverDismissOnPointerOutside =
        options.dismissOnPointerOutside;
    }
    if (options.grab !== undefined) nativeOptions.popoverGrab = options.grab;
    if (options.acceptsKeyFocus !== undefined) {
      nativeOptions.popoverAcceptsKeyFocus = options.acceptsKeyFocus;
    }
    const parent = options.anchor?.host;
    if (
      options.anchor &&
      (!parent || parent.closed || !options.anchor.materialized)
    ) {
      throw new Error(
        "a system popover requires a mounted node in an open parent Window",
      );
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
        throw new TypeError(
          "a QuickGUI Window renderer must return a dispose function",
        );
      }
      this._trackMount(dispose);
      const initialBatch = this.#takePendingBatch();
      if (embedded) {
        if (process.platform !== "darwin") {
          throw new Error("embedded SwiftUI QuickGUI views require macOS");
        }
        this.nativeId = hostedRuntime
          ? binding.createHostedEmbeddedView(
              app.nativeId,
              embedded.owner.nativeId,
              embedded.matchContents.horizontal,
              embedded.matchContents.vertical,
              nativeOptions,
              initialBatch,
            )
          : binding.createEmbeddedView(
              app.nativeId,
              embedded.owner.nativeId,
              embedded.matchContents.horizontal,
              embedded.matchContents.vertical,
              nativeOptions,
              initialBatch,
            );
      } else if (options.anchor) {
        this.nativeId = hostedRuntime
          ? binding.createHostedSystemPopover(
              app.nativeId,
              parent!.nativeId,
              options.anchor.id,
              nativeOptions,
              initialBatch,
            )
          : binding.createSystemPopover(
              app.nativeId,
              parent!.nativeId,
              options.anchor.id,
              nativeOptions,
              initialBatch,
            );
      } else {
        this.nativeId = hostedRuntime
          ? binding.createHostedWindow(
              app.nativeId,
              nativeOptions,
              initialBatch,
            )
          : binding.createWindow(app.nativeId, nativeOptions, initialBatch);
      }
      this.#nativeReady = true;
      app._registerWindow(this);
      for (const callback of [...this.#nativeReadyCallbacks]) {
        this.#nativeReadyCallbacks.delete(callback);
        callback();
      }
      if (serializedMenu !== undefined)
        this._trackMount(serializedMenu.install());
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

  /** @internal Run after this window has a native id, including during its initial construction. */
  _afterNativeReady(callback: () => void): () => void {
    if (this.#closed) return () => {};
    if (this.#nativeReady) {
      callback();
      return () => {};
    }
    this.#nativeReadyCallbacks.add(callback);
    return () => this.#nativeReadyCallbacks.delete(callback);
  }

  close(): void {
    this.app._closeWindow(this);
  }

  getState(): Promise<WindowState> {
    return getNativeWindowState(this);
  }

  onStateChange(listener: (state: WindowState) => void): () => void {
    if (this.#closed) return () => {};
    return onNativeWindowStateChange(this, listener);
  }

  setTitle(title: string): void {
    performNativeWindowAction(this, "set-title", title);
  }

  setBounds(bounds: WindowBounds): void {
    performNativeWindowAction(this, "set-bounds", JSON.stringify(bounds));
  }

  setPosition(position: Point): void {
    performNativeWindowAction(this, "move", JSON.stringify(position));
  }

  setSize(size: Size): void {
    performNativeWindowAction(this, "resize", JSON.stringify(size));
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

  setResizable(resizable: boolean): void {
    performNativeWindowAction(this, "set-resizable", String(resizable));
  }

  setMovable(movable: boolean): void {
    performNativeWindowAction(this, "set-movable", String(movable));
  }

  setMinimumSize(size?: Size): void {
    performNativeWindowAction(
      this,
      "set-minimum-size",
      size === undefined ? undefined : JSON.stringify(size),
    );
  }

  setMaximumSize(size?: Size): void {
    performNativeWindowAction(
      this,
      "set-maximum-size",
      size === undefined ? undefined : JSON.stringify(size),
    );
  }

  setMinimizable(minimizable: boolean): void {
    performNativeWindowAction(this, "set-minimizable", String(minimizable));
  }

  setMaximizable(maximizable: boolean): void {
    performNativeWindowAction(this, "set-maximizable", String(maximizable));
  }

  setClosable(closable: boolean): void {
    performNativeWindowAction(this, "set-closable", String(closable));
  }

  setDecorated(decorated: boolean): void {
    performNativeWindowAction(this, "set-decorated", String(decorated));
  }

  setShadow(shadow: boolean): void {
    performNativeWindowAction(this, "set-shadow", String(shadow));
  }

  setContentProtected(protected_: boolean): void {
    performNativeWindowAction(
      this,
      "set-content-protected",
      String(protected_),
    );
  }

  setWindowLevel(level: WindowLevel | "automatic"): void {
    performNativeWindowAction(this, "set-window-level", level);
  }

  setFocusable(focusable: boolean): void {
    performNativeWindowAction(this, "set-focusable", String(focusable));
  }

  setSkipTaskbar(skip: boolean): void {
    performNativeWindowAction(this, "set-skip-taskbar", String(skip));
  }

  setVisibleOnAllWorkspaces(visible: boolean): void {
    performNativeWindowAction(
      this,
      "set-visible-on-all-workspaces",
      String(visible),
    );
  }

  setOpacity(opacity: number): void {
    performNativeWindowAction(this, "set-opacity", String(opacity));
  }

  setIcon(icon: ImageSource): void {
    performNativeWindowImageAction(this, "set-icon", icon);
  }

  clearIcon(): void {
    performNativeWindowImageAction(this, "clear-icon");
  }

  setTaskbarProgress(state: TaskbarProgressState, progress: number): void {
    performNativeWindowAction(
      this,
      "set-taskbar-progress",
      JSON.stringify({ state, progress }),
    );
  }

  setTaskbarOverlayIcon(icon: ImageSource, description: string): void {
    performNativeWindowImageAction(
      this,
      "set-taskbar-overlay-icon",
      icon,
      description,
    );
  }

  clearTaskbarOverlayIcon(): void {
    performNativeWindowAction(this, "clear-taskbar-overlay-icon");
  }

  setCursorVisible(visible: boolean): void {
    performNativeWindowAction(this, "set-cursor-visible", String(visible));
  }

  setCursorGrab(mode: CursorGrabMode): void {
    performNativeWindowAction(this, "set-cursor-grab", mode);
  }

  setCursorHitTest(hitTest: boolean): void {
    performNativeWindowAction(this, "set-cursor-hit-test", String(hitTest));
  }

  setCursorPosition(position: Point): void {
    performNativeWindowAction(
      this,
      "set-cursor-position",
      JSON.stringify(position),
    );
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

  setBackgroundAppearance(appearance: WindowBackgroundAppearance): void {
    performNativeWindowAction(this, "set-background-appearance", appearance);
  }

  setVibrancy(vibrancy?: MacOSVibrancy): void {
    performNativeWindowAction(this, "set-vibrancy", vibrancy);
  }

  setVisualEffectState(state: MacOSVisualEffectState): void {
    performNativeWindowAction(this, "set-visual-effect-state", state);
  }

  _focusNode(node: NativeNode): boolean {
    if (this.#closed || node.host !== this) return false;
    this.flush();
    if (hostedRuntime) {
      binding.focusHostedNode(this.app.nativeId, this.nativeId, node.id);
      return true;
    }
    return binding.focusNode(this.app.nativeId, this.nativeId, node.id);
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

  _dispatchEvent(
    type: NativeEventType,
    targetId: number,
    value?: string,
  ): void {
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
    this.#nativeReadyCallbacks.clear();
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

  _enqueueInsert(
    parent: NativeNode,
    child: NativeNode,
    before?: NativeNode,
  ): void {
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
function showAlertDialog(
  window: Window,
  options: AlertDialogOptions,
): Promise<number>;
function showAlertDialog(
  windowOrOptions: Window | AlertDialogOptions,
  maybeOptions?: AlertDialogOptions,
): Promise<number> {
  const hasWindow = windowOrOptions instanceof Window;
  const window = hasWindow ? windowOrOptions : undefined;
  const options = hasWindow
    ? maybeOptions
    : (windowOrOptions as AlertDialogOptions);
  if (!options)
    return Promise.reject(new TypeError("showAlertDialog requires options"));
  const app = window?.app ?? activeApp;
  if (!app)
    return Promise.reject(
      new Error("create a QuickGUI App before showing a dialog"),
    );
  return app._showAlertDialog(window, options);
}

function showOpenDialog(options?: OpenDialogOptions): Promise<OpenDialogResult>;
function showOpenDialog(
  window: Window,
  options?: OpenDialogOptions,
): Promise<OpenDialogResult>;
function showOpenDialog(
  windowOrOptions: Window | OpenDialogOptions = {},
  maybeOptions: OpenDialogOptions = {},
): Promise<OpenDialogResult> {
  const hasWindow = windowOrOptions instanceof Window;
  const window = hasWindow ? windowOrOptions : undefined;
  const options = hasWindow
    ? maybeOptions
    : (windowOrOptions as OpenDialogOptions);
  const app = window?.app ?? activeApp;
  if (!app)
    return Promise.reject(
      new Error("create a QuickGUI App before showing a dialog"),
    );
  return app._showOpenDialog(window, options);
}

function showSaveDialog(options?: SaveDialogOptions): Promise<SaveDialogResult>;
function showSaveDialog(
  window: Window,
  options?: SaveDialogOptions,
): Promise<SaveDialogResult>;
function showSaveDialog(
  windowOrOptions: Window | SaveDialogOptions = {},
  maybeOptions: SaveDialogOptions = {},
): Promise<SaveDialogResult> {
  const hasWindow = windowOrOptions instanceof Window;
  const window = hasWindow ? windowOrOptions : undefined;
  const options = hasWindow
    ? maybeOptions
    : (windowOrOptions as SaveDialogOptions);
  const app = window?.app ?? activeApp;
  if (!app)
    return Promise.reject(
      new Error("create a QuickGUI App before showing a dialog"),
    );
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
