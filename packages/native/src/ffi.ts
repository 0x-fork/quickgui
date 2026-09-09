import { CString, dlopen, JSCallback, toArrayBuffer, type Pointer } from "bun:ffi";
import { existsSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { PROTOCOL_VERSION } from "./protocol.generated.ts";
export interface NativeEvent {
  kind: string;
  window: number;
  target: number;
  value?: string;
  extra?: { error?: string; [key: string]: unknown };
  data?: Uint8Array;
}

export function resolveLibrary(): string {
  const name =
    process.platform === "darwin"
      ? "libquickgui_host.dylib"
      : process.platform === "win32"
        ? "quickgui_host.dll"
        : "libquickgui_host.so";
  if (process.env.QUICKGUI_LIBRARY) return resolve(process.env.QUICKGUI_LIBRARY);
  const executable = dirname(process.execPath);
  const bundled =
    process.platform === "darwin"
      ? join(executable, "..", "Frameworks", name)
      : join(executable, name);
  if (existsSync(bundled)) return bundled;
  return resolve(
    import.meta.dir,
    "..",
    "lib",
    `${process.platform === "win32" ? "windows" : process.platform}-${process.arch}`,
    name,
  );
}

export function loadLibrary() {
  const library = dlopen(resolveLibrary(), {
    quickgui_protocol_version: { args: [], returns: "u32" },
    quickgui_run_host: { args: [], returns: "i32" },
    quickgui_set_event_notifier: { args: ["function"], returns: "void" },
    quickgui_clear_event_notifier: { args: [], returns: "void" },
    quickgui_drain_events: { args: ["function", "ptr"], returns: "i32" },
    quickgui_create_app: { args: ["buffer", "usize"], returns: "u32" },
    quickgui_prepare_app: { args: ["u32", "u32"], returns: "i32" },
    quickgui_allocate_window: { args: [], returns: "u32" },
    quickgui_create_window: {
      args: ["u32", "u32", "buffer", "usize", "buffer", "usize"],
      returns: "i32",
    },
    quickgui_create_system_popover: {
      args: ["u32", "u32", "u32", "u32", "buffer", "usize", "buffer", "usize"],
      returns: "i32",
    },
    quickgui_create_embedded_view: {
      args: ["u32", "u32", "u32", "u8", "u8", "buffer", "usize", "buffer", "usize"],
      returns: "i32",
    },
    quickgui_show_dialog: { args: ["u32", "u32", "u32", "u32", "buffer", "usize"], returns: "i32" },
    quickgui_call: {
      args: ["buffer", "usize", "buffer", "usize", "function", "ptr"],
      returns: "i32",
    },
    quickgui_invoke: { args: ["u32", "buffer", "usize", "buffer", "usize"], returns: "i32" },
    quickgui_destroy_app: { args: ["u32"], returns: "i32" },
    quickgui_register_extension_versioned: {
      args: ["ptr", "buffer", "usize", "buffer", "usize"],
      returns: "i32",
    },
    quickgui_apply_batch: { args: ["u32", "u32", "buffer", "usize"], returns: "i32" },
    quickgui_close_window: { args: ["u32", "u32"], returns: "i32" },
    quickgui_focus_node: { args: ["u32", "u32", "u32"], returns: "i32" },
    quickgui_command: { args: ["u32", "u32", "buffer", "usize"], returns: "i32" },
    quickgui_abort: { args: ["buffer"], returns: "void" },
  });
  if (library.symbols.quickgui_protocol_version() !== PROTOCOL_VERSION)
    throw new Error(
      `QuickGUI protocol mismatch; rebuild or update @quickgui/native (expected ${PROTOCOL_VERSION})`,
    );
  return library;
}
export type Library = ReturnType<typeof loadLibrary>;
export function check(status: number): void {
  if (status !== 0) throw new Error("The QuickGUI native host rejected a command");
}
export const json = (value: unknown) =>
  new TextEncoder().encode(
    JSON.stringify(value, (_key, item) => {
      if (item instanceof Uint8Array) return [...item];
      if (item?.type === "Buffer" && Array.isArray(item.data)) return item.data;
      return item;
    }),
  );
export function abort(library: Library, error: unknown): void {
  const message = error instanceof Error ? (error.stack ?? error.message) : String(error);
  console.error(message);
  library.symbols.quickgui_abort(new TextEncoder().encode(message.replaceAll("\0", "") + "\0"));
}

/** Callbacks are retained for the process lifetime, including notifications already in flight. */
const callbacks: JSCallback[] = [];
export function listen(
  library: Library,
  dispatch: (event: NativeEvent) => void,
  fail: (error: unknown) => void,
): void {
  const copied: NativeEvent[] = [];
  const text = (pointer: Pointer, length: number | bigint) =>
    Number(length) ? String(new CString(pointer, 0, Number(length))) : "";
  const receive = new JSCallback(
    (
      kind,
      kindLength,
      window,
      target,
      flags,
      value,
      valueLength,
      extra,
      extraLength,
      data,
      dataLength,
    ) => {
      copied.push({
        kind: text(kind, kindLength),
        window,
        target,
        ...(flags & 1 ? { value: text(value, valueLength) } : {}),
        ...(flags & 2 ? { extra: JSON.parse(text(extra, extraLength)) } : {}),
        ...(flags & 4
          ? {
              data: Number(dataLength)
                ? new Uint8Array(toArrayBuffer(data, 0, Number(dataLength))).slice()
                : new Uint8Array(),
            }
          : {}),
      });
    },
    {
      args: [
        "ptr",
        "usize",
        "u32",
        "u32",
        "u32",
        "ptr",
        "usize",
        "ptr",
        "usize",
        "ptr",
        "usize",
        "ptr",
      ],
      returns: "void",
    },
  );
  const notify = new JSCallback(
    () => {
      try {
        let count: number;
        do {
          count = library.symbols.quickgui_drain_events(receive.ptr, null);
          if (count < 0)
            throw new Error("QuickGUI native event queue exceeded its bounded capacity");
          for (const event of copied.splice(0)) dispatch(event);
        } while (count === 256);
      } catch (error) {
        copied.length = 0;
        fail(error);
      }
    },
    { args: [], returns: "void", threadsafe: true },
  );
  callbacks.push(receive, notify);
  library.symbols.quickgui_set_event_notifier(notify.ptr);
}
