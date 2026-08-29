import { Buffer } from "node:buffer";
import { resolve as resolvePath } from "node:path";
import * as binding from "./binding.js";
import type { Window } from "./index.ts";
import {
  configureTrayContext,
  dispatchTrayEvent,
  rejectPendingTrayRequests,
} from "./tray.ts";

export { Tray, TrayIcon } from "./tray.ts";
export type {
  TrayEvent,
  TrayEventType,
  TrayIconOptions,
  TrayIconSource,
  TrayMenuActionItem,
  TrayMenuItem,
  TrayMenuSeparatorItem,
  TrayMenuSubmenuItem,
} from "./tray.ts";

export interface Rectangle {
  x: number;
  y: number;
  width: number;
  height: number;
}

export interface Display {
  /** Process-stable display identifier. Persist `uuid`, when available, across launches. */
  id: string;
  uuid?: string;
  name: string;
  bounds: Rectangle;
  workArea: Rectangle;
  scaleFactor: number;
  refreshRate?: number;
  primary: boolean;
}

export interface KeyboardLayout {
  id: string;
  name: string;
}

export type AppearanceMode = "light" | "dark";
export type AppearancePreference = AppearanceMode | "system";

export interface WindowState {
  displayId?: string;
  bounds: Rectangle;
  scaleFactor: number;
  appearance: AppearanceMode;
  focused: boolean;
  visible: boolean;
  minimized: boolean;
  maximized: boolean;
  fullscreen: boolean;
  occluded: boolean;
  movable: boolean;
  resizable: boolean;
  minimizable: boolean;
  representedFile: boolean;
  documentEdited: boolean;
}

export interface ClipboardTextEntry {
  type: "text";
  text: string;
  /** Application-owned JSON or string metadata. */
  metadata?: string;
}

export interface ClipboardImageEntry {
  type: "image";
  mimeType:
    | "image/png"
    | "image/jpeg"
    | "image/webp"
    | "image/gif"
    | "image/svg+xml"
    | "image/bmp"
    | "image/tiff"
    | "image/x-icon"
    | "image/x-portable-anymap";
  data: Uint8Array;
}

export interface ClipboardFilesEntry {
  type: "files";
  paths: readonly string[];
}

export type ClipboardEntry = ClipboardTextEntry | ClipboardImageEntry | ClipboardFilesEntry;

export interface ClipboardItem {
  /** Representations are committed atomically by one native clipboard write. */
  entries: readonly ClipboardEntry[];
}

export interface NotificationAction {
  id: string;
  label: string;
}

export interface NotificationOptions {
  /** Stable identity used to replace or later dismiss this notification. */
  tag: string;
  title: string;
  body?: string;
  actions?: readonly NotificationAction[];
}

export interface NotificationResponse {
  tag: string;
  /** Omitted when the notification body, rather than an action button, was activated. */
  actionId?: string;
}

export type MenuRole = "cut" | "copy" | "paste" | "select-all" | "undo" | "redo";

export interface MenuActionItem {
  type?: "action";
  label: string;
  enabled?: boolean;
  checked?: boolean;
  role?: MenuRole;
  click?: () => void;
}

export interface MenuSeparatorItem {
  type: "separator";
}

export interface MenuSubmenuItem {
  type: "submenu";
  label: string;
  enabled?: boolean;
  items: readonly MenuItem[];
}

export interface MenuServicesItem {
  /** macOS Services. Other platforms omit this operating-system-owned submenu. */
  type: "services";
  label?: string;
}

export type MenuItem =
  | MenuActionItem
  | MenuSeparatorItem
  | MenuSubmenuItem
  | MenuServicesItem;

export interface MenuDefinition {
  label: string;
  enabled?: boolean;
  items: readonly MenuItem[];
}

export type GlobalShortcutListener = () => void;
export type PowerEvent = "suspend" | "resume" | "lock-screen" | "unlock-screen";

type AppContext = {
  appId: number;
  hosted: boolean;
};

type ContextResolver = () => AppContext;
type WindowResolver = (window?: Window) => { context: AppContext; window: Window };

let resolveContext: ContextResolver | undefined;
let resolveWindow: WindowResolver | undefined;

export function configureSystemContext(
  context: ContextResolver,
  window: WindowResolver,
): void {
  resolveContext = context;
  resolveWindow = window;
  configureTrayContext(context);
}

function context(): AppContext {
  if (!resolveContext) throw new Error("QuickGUI's native system context is not configured");
  return resolveContext();
}

function windowContext(window?: Window): { context: AppContext; window: Window } {
  if (!resolveWindow) throw new Error("QuickGUI's native window context is not configured");
  return resolveWindow(window);
}

function nativeDisplays(): binding.NativeDisplay[] {
  const current = context();
  return current.hosted
    ? binding.getHostedDisplays(current.appId)
    : binding.getDisplays(current.appId);
}

function normalizeDisplay(display: binding.NativeDisplay): Display {
  const normalized: Display = {
    id: display.id,
    name: display.name,
    bounds: { ...display.bounds },
    workArea: { ...display.workArea },
    scaleFactor: display.scaleFactor,
    primary: display.primary,
  };
  if (display.uuid !== undefined) normalized.uuid = display.uuid;
  if (display.refreshRate !== undefined) normalized.refreshRate = display.refreshRate;
  return normalized;
}

function nativeKeyboardLayout(): KeyboardLayout {
  const current = context();
  const layout = current.hosted
    ? binding.getHostedKeyboardLayout(current.appId)
    : binding.getKeyboardLayout(current.appId);
  return { id: layout.id, name: layout.name };
}

function normalizeWindowState(state: binding.NativeWindowState): WindowState {
  const normalized: WindowState = {
    bounds: { x: state.x, y: state.y, width: state.width, height: state.height },
    scaleFactor: state.scaleFactor,
    appearance: state.appearance === "dark" ? "dark" : "light",
    focused: state.focused,
    visible: state.visible,
    minimized: state.minimized,
    maximized: state.maximized,
    fullscreen: state.fullscreen,
    occluded: state.occluded,
    movable: state.movable,
    resizable: state.resizable,
    minimizable: state.minimizable,
    representedFile: state.representedFile,
    documentEdited: state.documentEdited,
  };
  if (state.displayId !== undefined) normalized.displayId = state.displayId;
  return normalized;
}

export function getNativeWindowState(window: Window): WindowState {
  const { context: current, window: resolved } = windowContext(window);
  const state = current.hosted
    ? binding.getHostedWindowState(current.appId, resolved.nativeId)
    : binding.getWindowState(current.appId, resolved.nativeId);
  return normalizeWindowState(state);
}

export function performNativeWindowAction(
  window: Window,
  action: string,
  value?: string,
): void {
  const { context: current, window: resolved } = windowContext(window);
  if (current.hosted) {
    binding.performHostedWindowAction(current.appId, resolved.nativeId, action, value);
  } else {
    binding.performWindowAction(current.appId, resolved.nativeId, action, value);
  }
}

function nativeClipboardItem(item: ClipboardItem): binding.NativeClipboardItem {
  return {
    entries: item.entries.map((entry): binding.NativeClipboardEntry => {
      if (entry.type === "text") {
        const native: binding.NativeClipboardEntry = { kind: "text", text: entry.text };
        if (entry.metadata !== undefined) native.metadata = entry.metadata;
        return native;
      }
      if (entry.type === "image") {
        return {
          kind: "image",
          format: entry.mimeType,
          data: Buffer.from(entry.data.buffer, entry.data.byteOffset, entry.data.byteLength),
        };
      }
      return { kind: "files", paths: [...entry.paths] };
    }),
  };
}

function clipboardItem(item: binding.NativeClipboardItem): ClipboardItem {
  return {
    entries: item.entries.map((entry): ClipboardEntry => {
      if (entry.kind === "text" && entry.text !== undefined) {
        const text: ClipboardTextEntry = { type: "text", text: entry.text };
        if (entry.metadata !== undefined) text.metadata = entry.metadata;
        return text;
      }
      if (entry.kind === "image" && entry.format && entry.data) {
        return {
          type: "image",
          mimeType: entry.format as ClipboardImageEntry["mimeType"],
          data: Uint8Array.from(entry.data),
        };
      }
      if (entry.kind === "files" && entry.paths) {
        return { type: "files", paths: entry.paths };
      }
      throw new Error(`the native clipboard returned an invalid ${entry.kind} entry`);
    }),
  };
}

const screenListeners = new Set<(displays: readonly Display[]) => void>();
const appearanceListeners = new Set<(appearance: AppearanceMode, window: Window) => void>();
const keyboardListeners = new Set<(layout: KeyboardLayout) => void>();
const powerListeners = new Map<PowerEvent, Set<() => void>>();
const notificationListeners = new Set<(response: NotificationResponse) => void>();
let menuCallbacks = new Map<number, () => void>();
let nextMenuAction = 1;
const windowStateListeners = new Map<number, Set<(state: WindowState) => void>>();
const pendingShellRequests = new Map<
  number,
  { resolve: () => void; reject: (error: Error) => void }
>();
let nextShellRequest = 1;
const globalShortcutRegistrations = new Map<
  number,
  { accelerator: string; listener: GlobalShortcutListener }
>();
const globalShortcutIds = new Map<string, number>();
const pendingGlobalShortcutRequests = new Map<
  number,
  { resolve: () => void; reject: (error: Error) => void }
>();
let nextGlobalShortcutRequest = 1;
let nextGlobalShortcutRegistration = 1;

export const Clipboard = Object.freeze({
  async read(): Promise<ClipboardItem | undefined> {
    const current = context();
    const item = current.hosted
      ? binding.readHostedClipboard(current.appId)
      : binding.readClipboard(current.appId);
    return item ? clipboardItem(item) : undefined;
  },

  async write(item: ClipboardItem): Promise<void> {
    const current = context();
    const native = nativeClipboardItem(item);
    if (current.hosted) binding.writeHostedClipboard(current.appId, native);
    else binding.writeClipboard(current.appId, native);
  },

  async readText(): Promise<string> {
    const item = await this.read();
    if (!item) return "";
    const text = item.entries
      .filter((entry): entry is ClipboardTextEntry => entry.type === "text")
      .map((entry) => entry.text)
      .join("");
    if (text) return text;
    return item.entries
      .filter((entry): entry is ClipboardFilesEntry => entry.type === "files")
      .flatMap((entry) => entry.paths)
      .join("\n");
  },

  async writeText(text: string): Promise<void> {
    await this.write({ entries: [{ type: "text", text }] });
  },

  async clear(): Promise<void> {
    await this.write({ entries: [] });
  },
});

function allocateShellRequest(): number {
  for (let attempt = 0; attempt <= pendingShellRequests.size; attempt += 1) {
    const request = nextShellRequest;
    nextShellRequest = request >= 0xffff_ffff ? 1 : request + 1;
    if (!pendingShellRequests.has(request)) return request;
  }
  throw new Error("the native shell request id space is exhausted");
}

function shellRequest(action: string, value: string): Promise<void> {
  try {
    const current = context();
    const request = allocateShellRequest();
    return new Promise<void>((resolve, reject) => {
      pendingShellRequests.set(request, {
        resolve,
        reject: (error) => reject(error),
      });
      try {
        if (current.hosted) {
          binding.performHostedShellAction(current.appId, request, action, value);
        } else {
          binding.performShellAction(current.appId, request, action, value);
        }
      } catch (error) {
        pendingShellRequests.delete(request);
        reject(error instanceof Error ? error : new Error(String(error)));
      }
    });
  } catch (error) {
    return Promise.reject(error instanceof Error ? error : new Error(String(error)));
  }
}

function externalUrl(value: string | URL): string {
  const url = value instanceof URL ? value : new URL(value);
  if (url.protocol === "file:") {
    throw new TypeError("Shell.openExternal does not accept file URLs; use Shell.openPath");
  }
  return url.href;
}

/** Cross-platform operations delegated to the user's registered system applications. */
export const Shell = Object.freeze({
  openExternal(url: string | URL): Promise<void> {
    try {
      return shellRequest("open-external", externalUrl(url));
    } catch (error) {
      return Promise.reject(error instanceof Error ? error : new Error(String(error)));
    }
  },

  openPath(path: string): Promise<void> {
    return shellRequest("open-path", resolvePath(path));
  },

  showItemInFolder(path: string): Promise<void> {
    return shellRequest("reveal-path", resolvePath(path));
  },

  trashItem(path: string): Promise<void> {
    return shellRequest("trash-path", resolvePath(path));
  },
});

/** Operating-system notifications delivered outside QuickGUI windows. */
export const Notifications = Object.freeze({
  isSupported(): boolean {
    return process.platform === "darwin" || process.platform === "win32" || process.platform === "linux";
  },

  async show(options: NotificationOptions): Promise<void> {
    const current = context();
    const native: binding.NativeNotificationOptions = {
      tag: options.tag,
      title: options.title,
      body: options.body ?? "",
      actions: (options.actions ?? []).map((action) => ({ ...action })),
    };
    if (current.hosted) binding.showHostedNotification(current.appId, native);
    else binding.showNotification(current.appId, native);
  },

  async dismiss(tag: string): Promise<void> {
    const current = context();
    if (current.hosted) binding.dismissHostedNotification(current.appId, tag);
    else binding.dismissNotification(current.appId, tag);
  },

  onResponse(listener: (response: NotificationResponse) => void): () => void {
    notificationListeners.add(listener);
    return () => notificationListeners.delete(listener);
  },
});

function allocateMenuAction(callbacks: Map<number, () => void>): number {
  for (let attempt = 0; attempt <= callbacks.size; attempt += 1) {
    const id = nextMenuAction;
    nextMenuAction = id >= 0xffff_ffff ? 1 : id + 1;
    if (!callbacks.has(id)) return id;
  }
  throw new Error("the native menu action id space is exhausted");
}

function nativeMenuItems(
  items: readonly MenuItem[],
  callbacks: Map<number, () => void>,
): unknown[] {
  return items.map((item) => {
    if (item.type === "separator") return { type: "separator" };
    if (item.type === "services") {
      return { type: "services", label: item.label ?? "Services" };
    }
    if (item.type === "submenu") {
      return {
        type: "submenu",
        label: item.label,
        enabled: item.enabled ?? true,
        items: nativeMenuItems(item.items, callbacks),
      };
    }
    const id = allocateMenuAction(callbacks);
    callbacks.set(id, item.click ?? (() => {}));
    return {
      type: "action",
      id,
      label: item.label,
      enabled: item.enabled ?? true,
      checked: item.checked ?? false,
      ...(item.role ? { role: item.role } : {}),
    };
  });
}

/** Declarative platform-native application menus. */
export const Menu = Object.freeze({
  setApplicationMenu(definitions: readonly MenuDefinition[] | null): void {
    const current = context();
    const callbacks = new Map<number, () => void>();
    const native = (definitions ?? []).map((menu) => ({
      label: menu.label,
      enabled: menu.enabled ?? true,
      items: nativeMenuItems(menu.items, callbacks),
    }));
    const json = JSON.stringify(native);
    if (current.hosted) binding.setHostedApplicationMenu(current.appId, json);
    else binding.setApplicationMenu(current.appId, json);
    menuCallbacks = callbacks;
  },
});

function allocateBoundedId(next: number, used: ReadonlyMap<number, unknown>): number {
  let candidate = next;
  for (let attempt = 0; attempt <= used.size; attempt += 1) {
    if (!used.has(candidate)) return candidate;
    candidate = candidate >= 0xffff_ffff ? 1 : candidate + 1;
  }
  throw new Error("the native system id space is exhausted");
}

function globalShortcutOperation(
  action: "register" | "unregister" | "unregister-all",
  registration?: number,
  accelerator?: string,
): Promise<void> {
  try {
    const current = context();
    const request = allocateBoundedId(nextGlobalShortcutRequest, pendingGlobalShortcutRequests);
    nextGlobalShortcutRequest = request >= 0xffff_ffff ? 1 : request + 1;
    return new Promise<void>((resolve, reject) => {
      pendingGlobalShortcutRequests.set(request, { resolve, reject });
      try {
        if (current.hosted) {
          binding.performHostedGlobalShortcutAction(
            current.appId,
            request,
            action,
            registration,
            accelerator,
          );
        } else {
          binding.performGlobalShortcutAction(
            current.appId,
            request,
            action,
            registration,
            accelerator,
          );
        }
      } catch (error) {
        pendingGlobalShortcutRequests.delete(request);
        reject(error instanceof Error ? error : new Error(String(error)));
      }
    });
  } catch (error) {
    return Promise.reject(error instanceof Error ? error : new Error(String(error)));
  }
}

/** System-wide keyboard shortcuts. Linux support requires an X11 session. */
export const GlobalShortcut = Object.freeze({
  isSupported(): boolean {
    return process.platform === "darwin" || process.platform === "win32" || process.platform === "linux";
  },

  async register(
    accelerator: string,
    listener: GlobalShortcutListener,
  ): Promise<() => Promise<void>> {
    if (globalShortcutIds.has(accelerator)) {
      throw new Error(`the global shortcut \`${accelerator}\` is already registered`);
    }
    const registration = allocateBoundedId(
      nextGlobalShortcutRegistration,
      globalShortcutRegistrations,
    );
    nextGlobalShortcutRegistration = registration >= 0xffff_ffff ? 1 : registration + 1;
    globalShortcutIds.set(accelerator, registration);
    globalShortcutRegistrations.set(registration, { accelerator, listener });
    try {
      await globalShortcutOperation("register", registration, accelerator);
    } catch (error) {
      globalShortcutIds.delete(accelerator);
      globalShortcutRegistrations.delete(registration);
      throw error;
    }
    let disposed = false;
    return async () => {
      if (disposed) return;
      disposed = true;
      if (globalShortcutRegistrations.get(registration)?.accelerator !== accelerator) return;
      await globalShortcutOperation("unregister", registration);
      globalShortcutRegistrations.delete(registration);
      globalShortcutIds.delete(accelerator);
    };
  },

  async unregister(accelerator: string): Promise<void> {
    const registration = globalShortcutIds.get(accelerator);
    if (registration === undefined) return;
    await globalShortcutOperation("unregister", registration);
    globalShortcutRegistrations.delete(registration);
    globalShortcutIds.delete(accelerator);
  },

  async unregisterAll(): Promise<void> {
    await globalShortcutOperation("unregister-all");
    globalShortcutRegistrations.clear();
    globalShortcutIds.clear();
  },

  isRegistered(accelerator: string): boolean {
    return globalShortcutIds.has(accelerator);
  },
});

export const Screen = Object.freeze({
  getAllDisplays(): Display[] {
    return nativeDisplays().map(normalizeDisplay);
  },

  getPrimaryDisplay(): Display | undefined {
    return this.getAllDisplays().find((display) => display.primary);
  },

  getDisplayNearestPoint(point: { x: number; y: number }): Display | undefined {
    let nearest: Display | undefined;
    let nearestDistance = Number.POSITIVE_INFINITY;
    for (const display of this.getAllDisplays()) {
      const right = display.bounds.x + display.bounds.width;
      const bottom = display.bounds.y + display.bounds.height;
      const dx = point.x < display.bounds.x ? display.bounds.x - point.x : Math.max(0, point.x - right);
      const dy = point.y < display.bounds.y ? display.bounds.y - point.y : Math.max(0, point.y - bottom);
      const distance = dx * dx + dy * dy;
      if (distance < nearestDistance) {
        nearest = display;
        nearestDistance = distance;
      }
    }
    return nearest;
  },

  onChange(listener: (displays: readonly Display[]) => void): () => void {
    screenListeners.add(listener);
    return () => screenListeners.delete(listener);
  },
});

export const Appearance = Object.freeze({
  getCurrent(window?: Window): AppearanceMode {
    return getNativeWindowState(windowContext(window).window).appearance;
  },

  onChange(listener: (appearance: AppearanceMode, window: Window) => void): () => void {
    appearanceListeners.add(listener);
    return () => appearanceListeners.delete(listener);
  },
});

export const Keyboard = Object.freeze({
  getLayout(): KeyboardLayout {
    return nativeKeyboardLayout();
  },

  onLayoutChange(listener: (layout: KeyboardLayout) => void): () => void {
    keyboardListeners.add(listener);
    return () => keyboardListeners.delete(listener);
  },
});

/** Native operating-system sleep and user-session transitions. */
export const PowerMonitor = Object.freeze({
  on(event: PowerEvent, listener: () => void): () => void {
    const listeners = powerListeners.get(event) ?? new Set();
    listeners.add(listener);
    powerListeners.set(event, listeners);
    return () => {
      listeners.delete(listener);
      if (listeners.size === 0) powerListeners.delete(event);
    };
  },
});

export function onNativeWindowStateChange(
  window: Window,
  listener: (state: WindowState) => void,
): () => void {
  const listeners = windowStateListeners.get(window.nativeId) ?? new Set();
  listeners.add(listener);
  windowStateListeners.set(window.nativeId, listeners);
  return () => {
    listeners.delete(listener);
    if (listeners.size === 0) windowStateListeners.delete(window.nativeId);
  };
}

export function removeNativeWindowStateListeners(window: Window): void {
  windowStateListeners.delete(window.nativeId);
}

export function rejectPendingSystemRequests(error: Error): void {
  for (const [request, pending] of pendingShellRequests) {
    pendingShellRequests.delete(request);
    pending.reject(error);
  }
  for (const [request, pending] of pendingGlobalShortcutRequests) {
    pendingGlobalShortcutRequests.delete(request);
    pending.reject(error);
  }
  globalShortcutRegistrations.clear();
  globalShortcutIds.clear();
  rejectPendingTrayRequests(error);
  screenListeners.clear();
  appearanceListeners.clear();
  keyboardListeners.clear();
  powerListeners.clear();
  notificationListeners.clear();
  menuCallbacks.clear();
  windowStateListeners.clear();
}

export function dispatchSystemEvent(
  event: binding.NativeEvent,
  findWindow: (id: number) => Window | undefined,
): boolean {
  if (dispatchTrayEvent(event)) return true;
  if (event.kind === "shell") {
    const pending = pendingShellRequests.get(event.target);
    if (!pending) return true;
    pendingShellRequests.delete(event.target);
    if (event.error !== undefined) pending.reject(new Error(event.error));
    else pending.resolve();
    return true;
  }
  if (event.kind === "global-shortcut-operation") {
    const pending = pendingGlobalShortcutRequests.get(event.target);
    if (!pending) return true;
    pendingGlobalShortcutRequests.delete(event.target);
    if (event.error !== undefined) pending.reject(new Error(event.error));
    else pending.resolve();
    return true;
  }
  if (event.kind === "global-shortcut") {
    globalShortcutRegistrations.get(event.target)?.listener();
    return true;
  }
  if (event.kind === "power-event") {
    const value = event.value as PowerEvent | undefined;
    if (
      value !== "suspend" &&
      value !== "resume" &&
      value !== "lock-screen" &&
      value !== "unlock-screen"
    ) {
      return true;
    }
    for (const listener of powerListeners.get(value) ?? []) listener();
    return true;
  }
  if (event.kind === "notification-response") {
    try {
      const value = JSON.parse(event.value ?? "{}") as {
        tag?: unknown;
        actionId?: unknown;
      };
      if (typeof value.tag !== "string") return true;
      const response: NotificationResponse = { tag: value.tag };
      if (typeof value.actionId === "string") response.actionId = value.actionId;
      for (const listener of notificationListeners) listener(response);
    } catch {
      // The app-level dispatcher follows the same fail-closed parsing contract.
    }
    return notificationListeners.size > 0;
  }
  if (event.kind === "menu-action") {
    menuCallbacks.get(event.target)?.();
    return true;
  }
  if (event.kind === "screen-change") {
    const displays = Screen.getAllDisplays();
    for (const listener of screenListeners) listener(displays);
    return true;
  }
  if (event.kind === "keyboard-layout-change") {
    const layout = Keyboard.getLayout();
    for (const listener of keyboardListeners) listener(layout);
    return true;
  }
  if (event.kind === "appearance-change") {
    const window = findWindow(event.window);
    if (!window) return true;
    const appearance = event.value === "dark" ? "dark" : "light";
    for (const listener of appearanceListeners) listener(appearance, window);
    return true;
  }
  if (event.kind === "window-state-change") {
    const window = findWindow(event.window);
    if (!window) return true;
    const listeners = windowStateListeners.get(event.window);
    if (!listeners?.size) return true;
    const state = getNativeWindowState(window);
    for (const listener of listeners) listener(state);
    return true;
  }
  return false;
}
