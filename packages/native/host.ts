import * as binding from "./binding.js";

const WORKER_ENV = "QUICKGUI_APP_WORKER";
const WORKER_READY = "quickgui:worker-ready";

/**
 * Start the application Bun isolate, then permanently hand the process main thread to AppKit.
 * Application code stays on Bun's supported Worker event loop; the native host sleeps until either
 * Winit or the worker posts bounded work.
 */
export async function runApplicationWorker(entrypoint: string): Promise<number> {
  if (!Bun.isMainThread) {
    throw new Error("the QuickGUI native host must run on Bun's process main thread");
  }

  const worker = new Worker(entrypoint, {
    name: "quickgui-app",
    env: { ...process.env, [WORKER_ENV]: "1" },
  });
  await waitForWorkerReady(worker);
  const onReady = developmentReadyCallback();
  return binding.runAppHost(onReady);
}

/** Report an application-worker startup failure while the main thread is blocked in AppKit. */
export function reportWorkerFailure(error: unknown): void {
  const message =
    error instanceof Error ? (error.stack ?? error.message) : `QuickGUI worker failed: ${String(error)}`;
  binding.abortAppHost(message);
}

function waitForWorkerReady(worker: Worker): Promise<void> {
  return new Promise((resolve, reject) => {
    const ready = (event: MessageEvent): void => {
      if (event.data !== WORKER_READY) return;
      worker.removeEventListener("error", failed);
      worker.removeEventListener("message", ready);
      resolve();
    };
    const failed = (event: ErrorEvent): void => {
      worker.removeEventListener("message", ready);
      reject(new Error(event.message || "QuickGUI application worker failed to start"));
    };
    worker.addEventListener("message", ready);
    worker.addEventListener("error", failed, { once: true });
  });
}

function developmentReadyCallback(): (() => void) | undefined {
  if (process.env.QUICKGUI_DEV !== "1" || typeof process.send !== "function") return undefined;
  return () => {
    try {
      process.send?.({ type: "quickgui-ready" });
    } catch {
      // The CLI may have exited while the native window was being presented.
    }
  };
}
