/**
 * Core-first platform services: clipboard, shell, notifications, menus, global shortcuts,
 * displays, desktop integration, appearance, keyboard, power, preferences, permissions, and tray
 * icons. Every mutation is fire-and-forget; every query resolves from a host reply event.
 */

import { decodeBase64, encodeBase64 } from "./base64.ts";
import type { Window } from "./index.ts";
import {
  allocateRequest,
  callService,
  isNullJson,
  jsonBoolean,
  sendCommand,
  sendMutation,
  sendRequest,
  settleReply,
} from "./requests.ts";

// ---------------------------------------------------------------------------
// Shared types
// ---------------------------------------------------------------------------

export interface ImageBytes {
  /** Encoded image bytes, or RGBA8 when width and height are supplied. */
  data: Uint8Array;
  width?: number;
  height?: number;
}

export type ImageSource = string | ImageBytes;

export interface Rectangle {
  x: number;
  y: number;
  width: number;
  height: number;
}

export interface Display {
  id: string;
  uuid?: string;
  name: string;
  bounds: Rectangle;
  workArea: Rectangle;
  scaleFactor: number;
  refreshRate?: number;
  primary: boolean;
}

export interface DesktopIntegrationSupport {
  systemNotifications: boolean;
  scheduledNotifications: boolean;
  notificationReplies: boolean;
  nativeApplicationMenus: boolean;
  nativePopupMenus: boolean;
  trayIcons: boolean;
  programmableTrayPopup: boolean;
  globalShortcuts: boolean;
  singleInstance: boolean;
  dynamicProtocolRegistration: boolean;
  autostart: boolean;
  windowIcons: boolean;
  windowFocusability: boolean;
  windowOpacity: boolean;
  skipTaskbar: boolean;
  visibleOnAllWorkspaces: boolean;
  cursorControl: boolean;
  cursorScreenPosition: boolean;
  taskbarProgress: boolean;
  taskbarOverlayIcons: boolean;
  dockBadges: boolean;
  dockIcons: boolean;
  dockMenus: boolean;
  recentDocuments: boolean;
  fileIcons: boolean;
  nativeAboutPanel: boolean;
  userTasks: boolean;
}

export interface AboutPanelOptions {
  applicationName?: string;
  applicationVersion?: string;
  version?: string;
  copyright?: string;
  credits?: string;
  icon?: ImageSource;
}

export interface UserTask {
  title: string;
  arguments: string;
  program?: string;
  description?: string;
  workingDirectory?: string;
  iconPath?: string;
  iconIndex?: number;
}

export interface NativeImage {
  data: Uint8Array;
  width: number;
  height: number;
}

export interface KeyboardLayout {
  id: string;
  name: string;
}

export interface WindowRestoreState {
  x: number;
  y: number;
  width: number;
  height: number;
  maximized: boolean;
  fullscreen: boolean;
  displayId?: string;
  displayUuid?: string;
  scaleFactor: number;
}

/**
 * One representation of a clipboard item. `type` is `text`, `image`, `data`, `bookmark`, or
 * `files`; text entries carry `text`, image and data entries `mimeType` and `data`, bookmarks
 * `title` and `url`, and file entries `paths`.
 */
export interface ClipboardEntry {
  type: string;
  text?: string;
  /** Application-owned JSON or string metadata of a text entry. */
  metadata?: string;
  /** MIME type of an image or data entry. */
  mimeType?: string;
  data?: Uint8Array;
  paths?: string[];
  title?: string;
  url?: string;
}

export interface ClipboardItem {
  entries: ClipboardEntry[];
}

interface NativeClipboardEntry {
  kind: string;
  text?: string;
  metadata?: string;
  format?: string;
  data?: string;
  paths?: string[];
  url?: string;
}

interface NativeClipboardItem {
  entries: NativeClipboardEntry[];
}

export interface NotificationAction {
  id: string;
  label: string;
  /** `button` (default) or `text-input`. */
  type?: "button" | "text-input";
  placeholder?: string;
}

interface NativeNotificationAction {
  id: string;
  label: string;
  kind?: string;
  placeholder?: string;
}

export interface NotificationAttachment {
  id: string;
  path: string;
}

export interface NotificationOptions {
  tag: string;
  title: string;
  body: string;
  subtitle?: string;
  actions?: NotificationAction[];
  /** `default`, `silent`, or a platform-recognized named sound. */
  sound?: string;
  iconPath?: string;
  attachments?: NotificationAttachment[];
  /** Absolute Unix epoch milliseconds. */
  deliveryAtMs?: number;
}

export type NotificationPermissionStatus = "not-determined" | "granted" | "denied" | "unsupported";

export type PermissionKind = "camera" | "microphone" | "screen-recording" | "accessibility";
export type PermissionStatus = "not-determined" | "granted" | "denied" | "restricted" | "unknown";

export interface PowerEvent {
  /** `suspend`, `resume`, `lock-screen`, `unlock-screen`, `shutdown-requested`, `power-source-changed`, `thermal-state-changed`, `low-power-mode-changed`, or `cpu-speed-limit-changed`. */
  type: string;
  source?: string;
  state?: string;
  enabled?: boolean;
  percent?: number;
}

export interface BatteryState {
  chargePercent?: number;
  status: string;
}

export interface PowerState {
  source: string;
  battery?: BatteryState;
  thermalState: string;
  lowPowerMode?: boolean;
  cpuSpeedLimitPercent?: number;
}

export interface SystemColor {
  red: number;
  green: number;
  blue: number;
  alpha: number;
}

export interface SystemPreferencesSnapshot {
  colorScheme: string;
  reduceMotion?: boolean;
  reduceTransparency?: boolean;
  increaseContrast?: boolean;
  differentiateWithoutColor?: boolean;
  invertColors?: boolean;
  forcedColors?: boolean;
  screenReader?: boolean;
  switchControl?: boolean;
  accentColor?: SystemColor;
  highlightColor?: SystemColor;
  highlightTextColor?: SystemColor;
  windowBackgroundColor?: SystemColor;
  windowTextColor?: SystemColor;
  controlBackgroundColor?: SystemColor;
  controlTextColor?: SystemColor;
  linkColor?: SystemColor;
}

// ---------------------------------------------------------------------------
// Menus
// ---------------------------------------------------------------------------

export interface MenuIcon {
  path?: string;
  data?: Uint8Array;
  width?: number;
  height?: number;
}

/**
 * One native menu entry. `type` defaults to `submenu` when `items` is present, `role` when only
 * a role is given, and `action` otherwise. `system` names a platform menu (`window`, `help`,
 * `services`, ...) through `menu`.
 */
export interface MenuItem {
  type?: "action" | "role" | "separator" | "submenu" | "system" | "services";
  label?: string;
  enabled?: boolean;
  checked?: boolean;
  mark?: "none" | "check" | "radio";
  role?: string;
  accelerator?: string;
  hidden?: boolean;
  icon?: MenuIcon;
  items?: MenuItem[];
  menu?: string;
  click?: () => void;
}

export interface MenuDefinition {
  label: string;
  enabled?: boolean;
  items: MenuItem[];
}

interface NativeMenuIcon {
  path?: string;
  dataBase64?: string;
  width?: number;
  height?: number;
}

interface NativeMenuItem {
  type: string;
  id?: number;
  label?: string;
  enabled?: boolean;
  checked?: boolean;
  mark?: string;
  role?: string;
  icon?: NativeMenuIcon;
  accelerator?: string;
  hidden?: boolean;
  items?: NativeMenuItem[];
  menu?: string;
}

interface NativeMenuDefinition {
  label: string;
  enabled: boolean;
  items: NativeMenuItem[];
}

class MenuCallback {
  readonly owner: number;
  readonly click: () => void;

  constructor(owner: number, click: () => void) {
    this.owner = owner;
    this.click = click;
  }
}

const menuCallbacks = new Map<number, MenuCallback>();
let nextMenuAction = 1;

export interface SerializedMenu {
  json: string;
  release: () => void;
}

function nativeMenuIcon(icon: MenuIcon): NativeMenuIcon {
  const native: NativeMenuIcon = {};
  if (icon.path !== undefined) native.path = icon.path;
  if (icon.data !== undefined) native.dataBase64 = encodeBase64(icon.data);
  if (icon.width !== undefined) native.width = icon.width;
  if (icon.height !== undefined) native.height = icon.height;
  return native;
}

function nativeMenuItems(items: MenuItem[], owner: number, ids: number[]): NativeMenuItem[] {
  const native: NativeMenuItem[] = [];
  for (const item of items) {
    let type = item.type;
    if (type === undefined) {
      if (item.items !== undefined) type = "submenu";
      else if (item.role !== undefined && item.click === undefined) type = "role";
      else type = "action";
    }
    if (type === "separator") {
      native.push({ type: "separator" });
      continue;
    }
    const label = item.label ?? "";
    if (type === "submenu") {
      native.push({ type: "submenu", label, enabled: item.enabled ?? true, items: nativeMenuItems(item.items ?? [], owner, ids) });
      continue;
    }
    if (type === "system") {
      native.push({ type: "system-menu", label, menu: item.menu ?? "window" });
      continue;
    }
    if (type === "services") {
      native.push({ type: "services", label });
      continue;
    }
    const entry: NativeMenuItem = {
      type: type === "role" ? "role" : "action",
      label,
      enabled: item.enabled ?? true,
      checked: item.checked ?? false,
      hidden: item.hidden ?? false,
    };
    if (item.mark !== undefined) entry.mark = item.mark;
    if (item.role !== undefined) entry.role = item.role;
    if (item.accelerator !== undefined) entry.accelerator = item.accelerator;
    if (item.icon !== undefined) entry.icon = nativeMenuIcon(item.icon);
    if (type !== "role") {
      const id = nextMenuAction;
      nextMenuAction = id >= 0xffff_fff0 ? 1 : id + 1;
      const click = item.click;
      menuCallbacks.set(id, new MenuCallback(owner, click === undefined ? () => undefined : click));
      ids.push(id);
      entry.id = id;
    }
    native.push(entry);
  }
  return native;
}

/** Encode menus for the host and register their click callbacks until `release` runs. */
export function serializeMenuDefinitions(definitions: MenuDefinition[], owner: number): SerializedMenu {
  const ids: number[] = [];
  const native: NativeMenuDefinition[] = [];
  for (const definition of definitions) {
    native.push({ label: definition.label, enabled: definition.enabled ?? true, items: nativeMenuItems(definition.items, owner, ids) });
  }
  return {
    json: JSON.stringify(native),
    release: () => {
      for (const id of ids) menuCallbacks.delete(id);
    },
  };
}

/** @internal Drop every registered menu callback, when the application exits. */
export function releaseMenuCallbacks(): void {
  menuCallbacks.clear();
}

let applicationMenu: SerializedMenu | undefined = undefined;
let dockMenu: SerializedMenu | undefined = undefined;

export class Menu {
  /** Show a native context menu and retain its callbacks until native tracking ends. */
  static async popup(items: MenuItem[], options: { window: Window; x?: number; y?: number }): Promise<void> {
    const window = options.window;
    if (window.closed) throw new Error("a native popup menu requires an open Window");
    if ((options.x === undefined) !== (options.y === undefined)) throw new TypeError("popup coordinates require both x and y");
    if ((options.x !== undefined && !Number.isFinite(options.x)) || (options.y !== undefined && !Number.isFinite(options.y))) {
      throw new TypeError("popup coordinates must be finite");
    }
    window.flush();
    const menu = serializeMenuDefinitions([{ label: "Context", items }], window.nativeId);
    const request = allocateRequest();
    try {
      await sendRequest(request, JSON.stringify({ method: "window-popup-menu", request, window: window.nativeId, menu: menu.json, x: options.x, y: options.y }));
    } finally {
      menu.release();
    }
  }

  /** Replace the application-wide native menu declaration. */
  static setApplicationMenu(definitions: MenuDefinition[]): void {
    const previous = applicationMenu;
    const menu = serializeMenuDefinitions(definitions, 0);
    applicationMenu = menu;
    sendMutation(JSON.stringify({ method: "set-application-menu", menu: menu.json }));
    if (previous !== undefined) previous.release();
  }
}

// ---------------------------------------------------------------------------
// Event routing
// ---------------------------------------------------------------------------

export class Listeners<T> {
  readonly entries: ((payload: T) => void)[] = [];

  add(listener: (payload: T) => void): () => void {
    this.entries.push(listener);
    return () => {
      const index = this.entries.indexOf(listener);
      if (index >= 0) this.entries.splice(index, 1);
    };
  }

  emit(payload: T): void {
    const snapshot = [...this.entries];
    for (const listener of snapshot) listener(payload);
  }

  get size(): number {
    return this.entries.length;
  }

  clear(): void {
    this.entries.splice(0, this.entries.length);
  }
}

/** A listener list for events that carry no payload. */
export class VoidListeners {
  readonly entries: (() => void)[] = [];

  add(listener: () => void): () => void {
    this.entries.push(listener);
    return () => {
      const index = this.entries.indexOf(listener);
      if (index >= 0) this.entries.splice(index, 1);
    };
  }

  emit(): void {
    const snapshot = [...this.entries];
    for (const listener of snapshot) listener();
  }

  get size(): number {
    return this.entries.length;
  }

  clear(): void {
    this.entries.splice(0, this.entries.length);
  }
}

class ShortcutRegistration {
  readonly accelerator: string;
  readonly listener: () => void;

  constructor(accelerator: string, listener: () => void) {
    this.accelerator = accelerator;
    this.listener = listener;
  }
}

const shortcutRegistrations = new Map<number, ShortcutRegistration>();
let nextShortcut = 1;
const powerListeners = new Listeners<PowerEvent>();
const preferencesListeners = new Listeners<SystemPreferencesSnapshot>();
const screenListeners = new Listeners<Display[]>();
const notificationPermissionResolvers = new Map<number, number>();

export interface HostEventExtra {
  paths?: string[];
  width?: number;
  height?: number;
  error?: string;
}

/**
 * @internal Route one host event that belongs to a platform service; returns whether it was
 * consumed. Request completions settle the reply registered for their request id.
 */
export function dispatchSystemEvent(
  kind: string,
  target: number,
  value: string | undefined,
  error: string | undefined,
  data: Uint8Array | undefined,
  extra: HostEventExtra | undefined,
): boolean {
  switch (kind) {
    case "shell":
    case "global-shortcut-operation":
    case "notification-permission":
    case "user-tasks":
    case "app-service":
    case "popup-menu":
    case "tray-operation":
      settleReply(target, value, error);
      return true;
    case "file-icon": {
      if (error !== undefined || data === undefined || extra === undefined) {
        settleReply(target, undefined, error ?? "the native file-icon response was invalid");
        return true;
      }
      settleReply(
        target,
        JSON.stringify({ width: extra.width ?? 0, height: extra.height ?? 0, data: encodeBase64(data) }),
        undefined,
      );
      return true;
    }
    case "global-shortcut": {
      const registration = shortcutRegistrations.get(target);
      if (registration !== undefined) registration.listener();
      return true;
    }
    case "power-event":
      if (value !== undefined) powerListeners.emit(JSON.parse(value) as PowerEvent);
      return true;
    case "system-preferences-change":
      if (preferencesListeners.size > 0) void emitPreferencesChange();
      return true;
    case "screen-change":
      if (screenListeners.size > 0) void emitScreenChange();
      return true;
    case "menu-action": {
      const callback = menuCallbacks.get(target);
      if (callback !== undefined) callback.click();
      return true;
    }
    case "tray-event":
      dispatchTrayEvent(target, value);
      return true;
    default:
      return false;
  }
}

async function emitPreferencesChange(): Promise<void> {
  try {
    preferencesListeners.emit(await SystemPreferences.getCurrent());
  } catch {
    // The application is shutting down.
  }
}

async function emitScreenChange(): Promise<void> {
  try {
    screenListeners.emit(await Screen.getAllDisplays());
  } catch {
    // The application is shutting down.
  }
}

/** @internal Forget service registrations when the application exits or is destroyed. */
export function rejectPendingSystemRequests(_error: Error): void {
  shortcutRegistrations.clear();
  notificationPermissionResolvers.clear();
  powerListeners.clear();
  preferencesListeners.clear();
  screenListeners.clear();
}

// ---------------------------------------------------------------------------
// Services
// ---------------------------------------------------------------------------

function clipboardEntriesToNative(item: ClipboardItem): NativeClipboardItem {
  const entries: NativeClipboardEntry[] = [];
  for (const entry of item.entries) {
    const native: NativeClipboardEntry = { kind: entry.type };
    if (entry.text !== undefined) native.text = entry.text;
    if (entry.title !== undefined) native.text = entry.title;
    if (entry.metadata !== undefined) native.metadata = entry.metadata;
    if (entry.mimeType !== undefined) native.format = entry.mimeType;
    if (entry.data !== undefined) native.data = encodeBase64(entry.data);
    if (entry.paths !== undefined) native.paths = entry.paths;
    if (entry.url !== undefined) native.url = entry.url;
    entries.push(native);
  }
  return { entries };
}

function clipboardEntriesFromNative(item: NativeClipboardItem): ClipboardItem {
  const entries: ClipboardEntry[] = [];
  for (const native of item.entries) {
    const entry: ClipboardEntry = { type: native.kind };
    if (native.text !== undefined) {
      if (native.kind === "bookmark") entry.title = native.text;
      else entry.text = native.text;
    }
    if (native.metadata !== undefined) entry.metadata = native.metadata;
    if (native.format !== undefined) entry.mimeType = native.format;
    if (native.data !== undefined) entry.data = decodeBase64(native.data);
    if (native.paths !== undefined) entry.paths = native.paths;
    if (native.url !== undefined) entry.url = native.url;
    entries.push(entry);
  }
  return { entries };
}

async function readClipboardCommand(method: string): Promise<ClipboardItem | undefined> {
  const json = await sendCommand(JSON.stringify({ method }));
  if (isNullJson(json)) return undefined;
  return clipboardEntriesFromNative(JSON.parse(json) as NativeClipboardItem);
}

export class Clipboard {
  static read(): Promise<ClipboardItem | undefined> {
    return readClipboardCommand("read-clipboard");
  }
  static async readText(): Promise<string | undefined> {
    const item = await readClipboardCommand("read-clipboard");
    if (item === undefined) return undefined;
    for (const entry of item.entries) {
      if (entry.type === "text" && entry.text !== undefined) return entry.text;
    }
    return undefined;
  }
  static async write(item: ClipboardItem): Promise<void> {
    await sendCommand(JSON.stringify({ method: "write-clipboard", item: clipboardEntriesToNative(item) }));
  }
  static writeText(text: string): Promise<void> {
    return Clipboard.write({ entries: [{ type: "text", text }] });
  }
  static writeImage(format: string, data: Uint8Array): Promise<void> {
    return Clipboard.write({ entries: [{ type: "image", mimeType: format, data }] });
  }
  static writeFiles(paths: string[]): Promise<void> {
    return Clipboard.write({ entries: [{ type: "files", paths }] });
  }
  /** The macOS Find pasteboard. */
  static readFind(): Promise<ClipboardItem | undefined> {
    return readClipboardCommand("read-find-clipboard");
  }
  static async writeFind(item: ClipboardItem): Promise<void> {
    await sendCommand(JSON.stringify({ method: "write-find-clipboard", item: clipboardEntriesToNative(item) }));
  }
}

async function shellRequest(action: string, value: string): Promise<void> {
  const request = allocateRequest();
  await sendRequest(request, JSON.stringify({ method: "shell-action", request, action, value }));
}

export class Shell {
  static openExternal(url: string): Promise<void> {
    return shellRequest("open-external", url);
  }
  static openPath(path: string): Promise<void> {
    return shellRequest("open-path", path);
  }
  static showItemInFolder(path: string): Promise<void> {
    return shellRequest("reveal-path", path);
  }
  static trashItem(path: string): Promise<void> {
    return shellRequest("trash-path", path);
  }
  static beep(): void {
    sendMutation('{"method":"app-mutation","action":"beep"}');
  }
}

export class SpellChecker {
  static learnWord(word: string): void {
    sendMutation(JSON.stringify({ method: "app-mutation", action: "learn-word", value: word }));
  }
  static ignoreWord(word: string): void {
    sendMutation(JSON.stringify({ method: "app-mutation", action: "ignore-word", value: word }));
  }
}

export class Notifications {
  static async show(options: NotificationOptions): Promise<void> {
    const actions: NativeNotificationAction[] = [];
    for (const action of options.actions ?? []) {
      const native: NativeNotificationAction = { id: action.id, label: action.label };
      if (action.type !== undefined) native.kind = action.type;
      if (action.placeholder !== undefined) native.placeholder = action.placeholder;
      actions.push(native);
    }
    await sendCommand(JSON.stringify({ method: "show-notification", options: { ...options, actions } }));
  }
  static async dismiss(tag: string): Promise<void> {
    await sendCommand(JSON.stringify({ method: "dismiss-notification", tag }));
  }
  static getPermissionStatus(): Promise<NotificationPermissionStatus> {
    return notificationPermission(false);
  }
  static requestPermission(): Promise<NotificationPermissionStatus> {
    return notificationPermission(true);
  }
}

async function notificationPermission(prompt: boolean): Promise<NotificationPermissionStatus> {
  const request = allocateRequest();
  const value = await sendRequest(request, JSON.stringify({ method: "notification-permission", request, prompt }));
  if (value === "not-determined" || value === "granted" || value === "denied" || value === "unsupported") return value;
  throw new Error("the native notification permission response was invalid");
}

export class GlobalShortcut {
  /** Register `accelerator`; resolves with a function that unregisters it again. */
  static async register(accelerator: string, listener: () => void): Promise<() => Promise<void>> {
    const registration = nextShortcut;
    nextShortcut = registration >= 0xffff_fff0 ? 1 : registration + 1;
    shortcutRegistrations.set(registration, new ShortcutRegistration(accelerator, listener));
    const request = allocateRequest();
    try {
      await sendRequest(
        request,
        JSON.stringify({ method: "global-shortcut", request, action: "register", registration, accelerator }),
      );
    } catch (error) {
      shortcutRegistrations.delete(registration);
      throw error;
    }
    return () => GlobalShortcut.unregister(registration);
  }
  static async unregister(registration: number): Promise<void> {
    shortcutRegistrations.delete(registration);
    const request = allocateRequest();
    await sendRequest(request, JSON.stringify({ method: "global-shortcut", request, action: "unregister", registration }));
  }
  static async unregisterAll(): Promise<void> {
    shortcutRegistrations.clear();
    const request = allocateRequest();
    await sendRequest(request, JSON.stringify({ method: "global-shortcut", request, action: "unregister-all" }));
  }
  static isRegistered(accelerator: string): boolean {
    for (const registration of shortcutRegistrations.values()) {
      if (registration.accelerator === accelerator) return true;
    }
    return false;
  }
}

export class Screen {
  static async getAllDisplays(): Promise<Display[]> {
    const json = await sendCommand('{"method":"get-displays"}');
    return JSON.parse(json) as Display[];
  }
  static async getPrimaryDisplay(): Promise<Display | undefined> {
    const displays = await Screen.getAllDisplays();
    for (const display of displays) {
      if (display.primary) return display;
    }
    return displays.length > 0 ? displays[0] : undefined;
  }
  static async getCursorScreenPoint(): Promise<{ x: number; y: number }> {
    const json = await sendCommand('{"method":"get-cursor-screen-position"}');
    return JSON.parse(json) as { x: number; y: number };
  }
  static onChange(listener: (displays: Display[]) => void): () => void {
    return screenListeners.add(listener);
  }
}

export interface NativeImageSourceJson {
  data?: string;
  path?: string;
  width?: number;
  height?: number;
}

function imageSourceJson(source: ImageSource): NativeImageSourceJson {
  if (typeof source === "string") return { path: source };
  const native: NativeImageSourceJson = { data: encodeBase64(source.data) };
  if (source.width !== undefined) native.width = source.width;
  if (source.height !== undefined) native.height = source.height;
  return native;
}

/** What the current platform's desktop integration can do. */
export interface DesktopIntegrationSupport {
  systemNotifications: boolean;
  scheduledNotifications: boolean;
  notificationReplies: boolean;
  nativeApplicationMenus: boolean;
  nativePopupMenus: boolean;
  trayIcons: boolean;
  programmableTrayPopup: boolean;
  globalShortcuts: boolean;
  singleInstance: boolean;
  dynamicProtocolRegistration: boolean;
  autostart: boolean;
  windowIcons: boolean;
  windowFocusability: boolean;
  windowOpacity: boolean;
  skipTaskbar: boolean;
  visibleOnAllWorkspaces: boolean;
  cursorControl: boolean;
  cursorScreenPosition: boolean;
  taskbarProgress: boolean;
  taskbarOverlayIcons: boolean;
  dockBadges: boolean;
  dockIcons: boolean;
  dockMenus: boolean;
  recentDocuments: boolean;
  fileIcons: boolean;
  nativeAboutPanel: boolean;
  userTasks: boolean;
}

export class Desktop {
  static async getSupport(): Promise<DesktopIntegrationSupport> {
    const json = await sendCommand('{"method":"get-desktop-integration-support"}');
    return JSON.parse(json) as DesktopIntegrationSupport;
  }
  static setDockBadge(value?: string): void {
    sendMutation(value === undefined ? '{"method":"set-dock-badge"}' : JSON.stringify({ method: "set-dock-badge", value }));
  }
  static setDockIcon(icon: ImageSource | undefined): void {
    sendMutation(
      icon === undefined ? '{"method":"set-dock-icon"}' : JSON.stringify({ method: "set-dock-icon", icon: imageSourceJson(icon) }),
    );
  }
  static setDockMenu(definition: MenuDefinition | undefined): void {
    const previous = dockMenu;
    if (definition === undefined) {
      dockMenu = undefined;
      sendMutation('{"method":"set-dock-menu"}');
    } else {
      const menu = serializeMenuDefinitions([definition], 0);
      dockMenu = menu;
      sendMutation(JSON.stringify({ method: "set-dock-menu", menu: menu.json }));
    }
    if (previous !== undefined) previous.release();
  }
  static addRecentDocument(path: string): void {
    sendMutation(JSON.stringify({ method: "add-recent-document", path }));
  }
  static clearRecentDocuments(): void {
    sendMutation('{"method":"clear-recent-documents"}');
  }
  static showAboutPanel(options: AboutPanelOptions = {}): void {
    const native: { applicationName?: string; applicationVersion?: string; version?: string; copyright?: string; credits?: string; icon?: NativeImageSourceJson } = {};
    if (options.applicationName !== undefined) native.applicationName = options.applicationName;
    if (options.applicationVersion !== undefined) native.applicationVersion = options.applicationVersion;
    if (options.version !== undefined) native.version = options.version;
    if (options.copyright !== undefined) native.copyright = options.copyright;
    if (options.credits !== undefined) native.credits = options.credits;
    if (options.icon !== undefined) native.icon = imageSourceJson(options.icon);
    sendMutation(JSON.stringify({ method: "show-about-panel", options: native }));
  }
  static async getFileIcon(path: string, size: "small" | "normal" | "large" = "normal"): Promise<NativeImage> {
    const request = allocateRequest();
    const json = await sendRequest(request, JSON.stringify({ method: "file-icon", request, path, size }));
    const icon = JSON.parse(json) as { width: number; height: number; data: string };
    return { width: icon.width, height: icon.height, data: decodeBase64(icon.data) };
  }
  static async setUserTasks(tasks: UserTask[]): Promise<void> {
    const request = allocateRequest();
    await sendRequest(request, JSON.stringify({ method: "set-user-tasks", request, tasks }));
  }
  static async getIntegrationSupport(): Promise<DesktopIntegrationSupport> {
    const json = await sendCommand('{"method":"get-desktop-integration-support"}');
    return JSON.parse(json) as DesktopIntegrationSupport;
  }
  static async getWindowRegistry(): Promise<{ windows: number[]; activeWindow?: number; truncated: boolean }> {
    const json = await sendCommand('{"method":"get-window-registry"}');
    return JSON.parse(json) as { windows: number[]; activeWindow?: number; truncated: boolean };
  }
}

export class SystemPreferences {
  static async getCurrent(): Promise<SystemPreferencesSnapshot> {
    const json = await sendCommand('{"method":"get-system-preferences"}');
    return JSON.parse(json) as SystemPreferencesSnapshot;
  }
  static onChange(listener: (preferences: SystemPreferencesSnapshot) => void): () => void {
    return preferencesListeners.add(listener);
  }
}

export class Appearance {
  /** `light`, `dark`, or `unknown`. */
  static async getColorScheme(): Promise<string> {
    const preferences = await SystemPreferences.getCurrent();
    return preferences.colorScheme;
  }
}

export class Keyboard {
  static async getLayout(): Promise<KeyboardLayout> {
    const json = await sendCommand('{"method":"get-keyboard-layout"}');
    return JSON.parse(json) as KeyboardLayout;
  }
}

export class PowerMonitor {
  static getState(): PowerState {
    return JSON.parse(callService("get-power-state", "")) as PowerState;
  }
  static getSystemIdleTime(): number {
    return Number(callService("get-system-idle-time", ""));
  }
  /** `active`, `idle`, `locked`, or `unknown`. */
  static getSystemIdleState(thresholdSeconds: number): string {
    return JSON.parse(callService("get-system-idle-state", JSON.stringify({ thresholdSeconds }))) as string;
  }
  /** `active`, `inactive`, `locked`, or `unknown`. */
  static getSessionState(): string {
    return JSON.parse(callService("get-session-state", "")) as string;
  }
  static onEvent(listener: (event: PowerEvent) => void): () => void {
    return powerListeners.add(listener);
  }
}

export class Permissions {
  static status(kind: PermissionKind): PermissionStatus {
    return JSON.parse(callService("get-permission-status", JSON.stringify({ kind }))) as PermissionStatus;
  }
}

/** One acquired power assertion; release it to let the system sleep again. */
export class PowerAssertion {
  readonly id: number;
  readonly kind: string;
  readonly reason: string;
  #released = false;

  constructor(kind: "prevent-application-suspension" | "prevent-display-sleep", reason: string) {
    this.id = Number(callService("power-assertion-acquire", JSON.stringify({ kind, reason })));
    this.kind = kind;
    this.reason = reason;
  }

  get active(): boolean {
    if (this.#released) return false;
    const info = JSON.parse(callService("power-assertion-info", JSON.stringify({ id: this.id }))) as { active: boolean };
    return info.active;
  }

  release(): boolean {
    if (this.#released) return false;
    this.#released = true;
    return callService("power-assertion-release", JSON.stringify({ id: this.id })) === "true";
  }
}

// ---------------------------------------------------------------------------
// Tray icons
// ---------------------------------------------------------------------------

export interface TrayMenuItem {
  type?: "action" | "separator" | "submenu";
  label?: string;
  enabled?: boolean;
  checked?: boolean;
  items?: TrayMenuItem[];
  click?: () => void;
}

export interface TrayIconOptions {
  icon: ImageSource;
  tooltip?: string;
  title?: string;
  iconIsTemplate?: boolean;
  menuOnLeftClick?: boolean;
  visible?: boolean;
  menu?: TrayMenuItem[];
}

export interface TrayEvent {
  /** `click`, `double-click`, `enter`, `move`, `leave`, `menu-item`, or `scroll`. */
  kind: string;
  menuItemId?: number;
  button?: string;
  position?: { x: number; y: number };
  pressed?: boolean;
  scrollDelta?: number;
  horizontal?: boolean;
}

interface NativeTrayMenuItem {
  type: string;
  id?: number;
  label?: string;
  enabled?: boolean;
  checked?: boolean;
  items?: NativeTrayMenuItem[];
}

interface NativeTrayIconOptions {
  id: number;
  iconData?: string;
  iconPath?: string;
  width?: number;
  height?: number;
  tooltip?: string;
  title?: string;
  iconIsTemplate?: boolean;
  menuOnLeftClick?: boolean;
  visible?: boolean;
  menu: string;
}

const trayIcons = new Map<number, TrayIcon>();
let nextTrayIcon = 1;

function dispatchTrayEvent(target: number, value: string | undefined): void {
  const icon = trayIcons.get(target);
  if (icon === undefined || value === undefined) return;
  const event = JSON.parse(value) as TrayEvent;
  icon._dispatch(event);
}

export class TrayIcon {
  readonly id: number;
  readonly #listeners = new Listeners<TrayEvent>();
  #menuCallbacks = new Map<number, MenuCallback>();
  #removed = false;

  constructor(id: number) {
    this.id = id;
  }

  /** Create and show a tray icon. */
  static async create(options: TrayIconOptions): Promise<TrayIcon> {
    const id = nextTrayIcon;
    nextTrayIcon = id >= 0xffff_fff0 ? 1 : id + 1;
    const icon = new TrayIcon(id);
    trayIcons.set(id, icon);
    try {
      await icon.update(options);
    } catch (error) {
      trayIcons.delete(id);
      throw error;
    }
    return icon;
  }

  onEvent(listener: (event: TrayEvent) => void): () => void {
    return this.#listeners.add(listener);
  }

  /** Replace the icon, tooltip, title, visibility, and menu. */
  async update(options: TrayIconOptions): Promise<void> {
    if (this.#removed) throw new Error("the tray icon was removed");
    const callbacks = new Map<number, MenuCallback>();
    let nextAction = 1;
    const encodeItems = (items: TrayMenuItem[]): NativeTrayMenuItem[] => {
      const native: NativeTrayMenuItem[] = [];
      for (const item of items) {
        const type = item.type ?? (item.items === undefined ? "action" : "submenu");
        if (type === "separator") {
          native.push({ type: "separator" });
        } else if (type === "submenu") {
          native.push({ type: "submenu", label: item.label ?? "", enabled: item.enabled ?? true, items: encodeItems(item.items ?? []) });
        } else {
          const actionId = nextAction;
          nextAction += 1;
          const click = item.click;
          callbacks.set(actionId, new MenuCallback(this.id, click === undefined ? () => undefined : click));
          native.push({ type: "action", id: actionId, label: item.label ?? "", enabled: item.enabled ?? true, checked: item.checked ?? false });
        }
      }
      return native;
    };
    const native: NativeTrayIconOptions = { id: this.id, menu: JSON.stringify(encodeItems(options.menu ?? [])) };
    const icon = options.icon;
    if (typeof icon === "string") {
      native.iconPath = icon;
    } else {
      native.iconData = encodeBase64(icon.data);
      if (icon.width !== undefined) native.width = icon.width;
      if (icon.height !== undefined) native.height = icon.height;
    }
    if (options.tooltip !== undefined) native.tooltip = options.tooltip;
    if (options.title !== undefined) native.title = options.title;
    native.iconIsTemplate = options.iconIsTemplate ?? false;
    native.menuOnLeftClick = options.menuOnLeftClick ?? true;
    native.visible = options.visible ?? true;
    const request = allocateRequest();
    await sendRequest(request, JSON.stringify({ method: "set-tray-icon", request, options: native }));
    this.#menuCallbacks = callbacks;
  }

  async showMenu(): Promise<void> {
    if (this.#removed) throw new Error("the tray icon was removed");
    const request = allocateRequest();
    await sendRequest(request, JSON.stringify({ method: "show-tray-menu", request, id: this.id }));
  }

  /** Remove the icon from the tray; the instance cannot be used afterwards. */
  async destroy(): Promise<void> {
    if (this.#removed) return;
    this.#removed = true;
    trayIcons.delete(this.id);
    this.#menuCallbacks.clear();
    const request = allocateRequest();
    await sendRequest(request, JSON.stringify({ method: "remove-tray-icon", request, id: this.id }));
  }

  _dispatch(event: TrayEvent): void {
    if (event.kind === "menu-item" && event.menuItemId !== undefined) {
      const callback = this.#menuCallbacks.get(event.menuItemId);
      if (callback !== undefined) callback.click();
    }
    this.#listeners.emit(event);
  }
}

export class Tray {
  static create(options: TrayIconOptions): Promise<TrayIcon> {
    return TrayIcon.create(options);
  }
}

/** @internal */
export function jsonTrue(json: string): boolean {
  return jsonBoolean(json);
}
