import { basename, dirname, resolve } from "node:path";
import type { Application } from "./index.ts";
import { abort, loadLibrary } from "./ffi.ts";

/** The main Bun thread owns AppKit; the worker owns Solid and application I/O. */
export async function runHost(entrypoint: string): Promise<void> {
  if (!Bun.isMainThread)
    throw new Error("QuickGUI's native loop requires Bun's process main thread");
  const library = loadLibrary();
  const worker = new Worker(entrypoint, { name: "quickgui-app", argv: process.argv.slice(2) });
  await new Promise<void>((resolve, reject) => {
    const timeout = setTimeout(
      () => failed(new Error("QuickGUI application worker did not start")),
      10000,
    );
    const failed = (error: Error) => {
      clearTimeout(timeout);
      worker.terminate();
      reject(error);
    };
    const onError = (event: ErrorEvent) => failed(new Error(event.message));
    const onMessage = (event: MessageEvent) => {
      if (event.data === "ready") {
        clearTimeout(timeout);
        worker.removeEventListener("error", onError);
        worker.removeEventListener("message", onMessage);
        resolve();
      } else if (event.data?.type === "startup-error") failed(new Error(event.data.message));
    };
    worker.addEventListener("error", onError);
    worker.addEventListener("message", onMessage);
  });
  worker.addEventListener("error", (event) => abort(library, event.message));
  const stopped = new Promise<number>((resolve) => {
    worker.addEventListener("message", (event) => {
      if (event.data?.type === "stopped") resolve(event.data.code);
    });
  });
  const code = library.symbols.quickgui_run_host();
  worker.postMessage({ type: "host-stopped", code });
  const timeout = setTimeout(() => {
    console.error("QuickGUI worker did not finish shutdown");
    process.exit(1);
  }, 5000);
  const workerCode = await stopped;
  clearTimeout(timeout);
  worker.terminate();
  process.exit(code || workerCode);
}

export async function runApplication(
  load: () => Promise<unknown>,
  options: {
    name?: string;
    version?: string;
    identifier?: string;
    fonts?: string[];
    extensions?: { name: string; version: string; library: string }[];
    updater?: object;
  },
): Promise<void> {
  const native = loadLibrary();
  let failureCode = 0;
  const fail = (error: unknown) => {
    failureCode = 1;
    abort(native, error);
  };
  process.on("uncaughtException", fail);
  process.on("unhandledRejection", fail);
  const binding = await import("./binding.ts");
  let app: Application | undefined;
  let running: Promise<number> | undefined;
  const scope = globalThis as unknown as {
    onmessage: (event: MessageEvent) => void;
    postMessage(value: unknown): void;
  };
  scope.onmessage = async (event) => {
    if (event.data?.type === "host-stopped") {
      try {
        binding.finish(event.data.code);
        if (running) failureCode ||= await running;
        app?.destroy();
      } catch (error) {
        fail(error);
      } finally {
        scope.postMessage({ type: "stopped", code: failureCode });
      }
    }
  };
  try {
    (
      globalThis as typeof globalThis & { __QUICKGUI_UPDATER_OPTIONS__?: object }
    ).__QUICKGUI_UPDATER_OPTIONS__ = options.updater ?? {};
    const directory = dirname(process.execPath);
    binding.initialize(
      {
        ...options,
        resourceDir:
          basename(directory) === "MacOS" ? resolve(directory, "..", "Resources") : directory,
      },
      fail,
      options.extensions,
    );
    app = (await import("./index.ts")).app;
    // Start event dispatch before evaluating application modules with top-level await.
    running = app.run().catch((error) => {
      fail(error);
      return 1;
    });
    scope.postMessage("ready");
    await load();
  } catch (error) {
    scope.postMessage({ type: "startup-error", message: String(error) });
    fail(error);
  }
}
