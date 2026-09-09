import { PROTOCOL_VERSION } from "../src/protocol.ts";

/**
 * One shared fake native binding for the JavaScript host tests.
 *
 * `bun test` evaluates every test file in a single module registry, so `../src/index.ts` and its
 * `app` singleton are created once against whichever `mock.module("../src/binding.ts", …)` ran first.
 * Both host test files therefore install this one recorder and read their assertions from it.
 *
 * This module is test support only: it is deliberately absent from the package's published
 * `files` list.
 */
export type Call = { name: string; args: unknown[] };

export const calls: Call[] = [];

/** Native events waiting for the next `app.dispatchEvents()`. */
const pendingEvents: Record<string, unknown>[] = [];
const extensionListeners = new Map<number, (value: string) => void>();

export function emitExtensionEvent(session: number, value: unknown): void {
  extensionListeners.get(session)?.(JSON.stringify(value));
}

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
  startExtension: (name: string, options: unknown, changed: (value: string) => void) => {
    const session = 1000 + callsNamed("startExtension").length;
    record("startExtension", [name, options, session]);
    extensionListeners.set(session, changed);
    return { session, ready: Promise.resolve() };
  },
  invoke: async (method: string, value: unknown) => {
    record("invoke", [method, value]);
    return null;
  },
  stopExtension: async (name: string, session: number) => {
    record("stopExtension", [name, session]);
    extensionListeners.delete(session);
  },
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
  performWindowImageAction: (...args: unknown[]) => record("performWindowImageAction", args),
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

// The FFI facade only exposes the asynchronous host route. Keep one recorder so
// behavioral assertions still inspect the same native operations.
for (const [name, value] of Object.entries(fakeBinding)) {
  if (typeof value !== "function") continue;
  const hosted = name.replace(
    /^(create|prepare|configure|destroy|apply|close|focus|get|read|write|set|show|perform|request|release|relaunch|exit|add|clear|remove|dismiss)/,
    "$1Hosted",
  );
  if (hosted !== name && !(hosted in fakeBinding))
    fakeBinding[hosted] = (...args: unknown[]) => {
      const result = value(...args);
      return /^(createHosted|applyHosted|focusHosted|closeHosted|destroyHosted|performHostedWindow)/.test(
        hosted,
      )
        ? result
        : Promise.resolve(result);
    };
}
fakeBinding.requestHostedAppQuit = fakeBinding.requestAppQuit;
fakeBinding.windowReady = () => Promise.resolve();
fakeBinding.waitForHostedEvents = () => new Promise(() => {});
let dispatchEvents: () => void = () => {};
export function setEventDispatcher(dispatch: () => void) {
  dispatchEvents = dispatch;
}
const parents = new Map<number, number>();
for (const name of ["createHostedSystemPopover", "createHostedEmbeddedView"]) {
  const create = fakeBinding[name] as (...args: unknown[]) => number;
  fakeBinding[name] = (...args: unknown[]) => {
    const id = create(...args);
    parents.set(id, args[1] as number);
    return id;
  };
}
fakeBinding.closeHostedWindow = (app: number, id: number) => {
  (fakeBinding.closeWindow as (...args: unknown[]) => unknown)(app, id);
  const close = (window: number) => {
    for (const [child, parent] of parents)
      if (parent === window) {
        parents.delete(child);
        close(child);
      }
    queueEvents({ kind: "close", window, target: 0 });
  };
  close(id);
  queueMicrotask(() => dispatchEvents());
};
