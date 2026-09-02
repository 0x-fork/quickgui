import { PROTOCOL_VERSION } from "./protocol.ts";

/**
 * One shared fake native binding for the JavaScript host tests.
 *
 * `bun test` evaluates every test file in a single module registry, so `./index.ts` and its
 * `app` singleton are created once against whichever `mock.module("./binding.js", …)` ran first.
 * Both host test files therefore install this one recorder and read their assertions from it.
 *
 * This module is test support only: it is deliberately absent from the package's published
 * `files` list.
 */
export type Call = { name: string; args: unknown[] };

export const calls: Call[] = [];

/** Native events waiting for the next `app.dispatchEvents()`. */
const pendingEvents: Record<string, unknown>[] = [];

let clipboardItem: { entries: Record<string, unknown>[] } | null = null;
let findClipboardItem: { entries: Record<string, unknown>[] } | null = null;

function record(name: string, args: unknown[]): void {
  calls.push({ name, args });
}

export function callsNamed(name: string): Call[] {
  return calls.filter((call) => call.name === name);
}

export function lastCall(name: string): Call {
  const call = callsNamed(name).at(-1);
  if (!call) throw new Error(`no recorded ${name} call`);
  return call;
}

/** Queue native events for the next host dispatch. */
export function queueEvents(...events: Record<string, unknown>[]): void {
  pendingEvents.push(...events);
}

export const fakeBinding: Record<string, unknown> = {
  protocolVersion: () => PROTOCOL_VERSION,
  createApp: (...args: unknown[]) => {
    record("createApp", args);
    return 1;
  },
  prepareApp: (...args: unknown[]) => record("prepareApp", args),
  isAppReady: () => true,
  destroyApp: (...args: unknown[]) => {
    record("destroyApp", args);
    return true;
  },
  configureApp: (...args: unknown[]) => record("configureApp", args),
  createWindow: (...args: unknown[]) => {
    record("createWindow", args);
    return 10 + callsNamed("createWindow").length;
  },
  createSystemPopover: (...args: unknown[]) => {
    record("createSystemPopover", args);
    return 100 + callsNamed("createSystemPopover").length;
  },
  createEmbeddedView: (...args: unknown[]) => {
    record("createEmbeddedView", args);
    return 200 + callsNamed("createEmbeddedView").length;
  },
  applyBatch: (...args: unknown[]) => {
    record("applyBatch", args);
    return 0;
  },
  focusNode: () => true,
  closeWindow: (...args: unknown[]) => {
    record("closeWindow", args);
    return true;
  },
  takeEvents: () => pendingEvents.splice(0),
  pumpApp: () => -1,
  performWindowAction: (...args: unknown[]) => record("performWindowAction", args),
  performWindowImageAction: (...args: unknown[]) =>
    record("performWindowImageAction", args),
  performAppService: (...args: unknown[]) => record("performAppService", args),
  performAppMutation: (...args: unknown[]) => record("performAppMutation", args),
  showWindowPopupMenu: (...args: unknown[]) => record("showWindowPopupMenu", args),
  setApplicationMenu: (...args: unknown[]) => record("setApplicationMenu", args),
  setQuitInterception: (...args: unknown[]) => record("setQuitInterception", args),
  requestAppQuit: (...args: unknown[]) => {
    record("requestAppQuit", args);
    return true;
  },
  exitApp: (...args: unknown[]) => {
    record("exitApp", args);
    return true;
  },
  exitAppWithCode: (...args: unknown[]) => {
    record("exitAppWithCode", args);
    return true;
  },
  isApplicationPackaged: () => {
    record("isApplicationPackaged", []);
    return true;
  },
  getApplicationsFolderSupport: (...args: unknown[]) => {
    record("getApplicationsFolderSupport", args);
    return { supported: true, alreadyInstalled: false };
  },
  getWindowRestoreState: (...args: unknown[]) => {
    record("getWindowRestoreState", args);
    return {
      x: 32,
      y: 64,
      width: 900,
      height: 600,
      maximized: false,
      fullscreen: true,
      displayId: "3",
      displayUuid: "00112233-4455-6677-8899-aabbccddeeff",
      scaleFactor: 2,
    };
  },
  releaseSingleInstanceLock: () => true,
  readClipboard: () => clipboardItem,
  writeClipboard: (_app: number, item: { entries: Record<string, unknown>[] }) => {
    record("writeClipboard", [item]);
    clipboardItem = item;
  },
  readFindClipboard: (...args: unknown[]) => {
    record("readFindClipboard", args);
    return findClipboardItem;
  },
  writeFindClipboard: (_app: number, item: { entries: Record<string, unknown>[] }) => {
    record("writeFindClipboard", [item]);
    findClipboardItem = item;
  },
};
