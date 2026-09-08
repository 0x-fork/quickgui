import { basename } from "node:path";

export const idlePolicy = {
  minimumWaitMs: 30_000,
  stableWindowMs: 15_000,
  maximumWaitMs: 180_000,
  intervalMs: 1_000,
  samplesPerRun: 10,
  maximumCpuPercent: 1,
  maximumMemorySpreadBytes: 1_000_000,
  maximumMemorySpreadRatio: 0.01,
};

export type ProcessRole = "app" | "framework-helper" | "os-service";

export interface MemoryProcess {
  pid: number;
  name: string;
  path: string;
  role: ProcessRole;
  footprintBytes: number;
  rssBytes: number;
  cpuTimeNs: number;
}

export interface MemorySample {
  elapsedMs: number;
  footprintBytes: number;
  mainProcessBytes: number;
  frameworkHelperBytes: number;
  excludedOsServiceBytes: number;
  rssBytes: number;
  cpuPercent: number | null;
  processes: MemoryProcess[];
}

export function processRole(
  path: string,
  pid: number,
  mainPid: number,
  appPath: string,
): ProcessRole {
  if (pid === mainPid) return "app";
  if (path.startsWith(appPath + "/")) return "framework-helper";
  // WebKit runs out of OS XPC bundles, but its renderer, GPU, and networking processes
  // are part of the app's runtime. Coalition membership is checked by the caller.
  if (/^com\.apple\.WebKit\.(WebContent|GPU|Networking)$/.test(basename(path)))
    return "framework-helper";
  // AutoFill/SafariPlatformSupport, SetStoreUpdateService, and other macOS services
  // are recorded for audit, but never enter the app's memory or idle CPU totals.
  return "os-service";
}

export function median(values: number[]): number {
  if (!values.length || values.some((value) => !Number.isFinite(value) || value < 0))
    throw new Error("Cannot summarize missing or invalid measurements");
  const sorted = [...values].sort((a, b) => a - b);
  const middle = Math.floor(sorted.length / 2);
  return sorted.length % 2 ? sorted[middle]! : (sorted[middle - 1]! + sorted[middle]!) / 2;
}

function appProcesses(sample: Pick<MemorySample, "processes">) {
  return sample.processes.filter((item) => item.role !== "os-service");
}

export function memorySample(
  elapsedMs: number,
  processes: MemoryProcess[],
  previous?: MemorySample,
): MemorySample {
  const included = appProcesses({ processes });
  const previousProcesses = new Map(
    appProcesses(previous ?? { processes: [] }).map((p) => [p.pid, p]),
  );
  const sameProcesses =
    previousProcesses.size === included.length &&
    included.every((p) => previousProcesses.has(p.pid));
  const elapsedNs = previous ? (elapsedMs - previous.elapsedMs) * 1_000_000 : 0;
  const cpuDelta = included.reduce(
    (sum, p) => sum + p.cpuTimeNs - (previousProcesses.get(p.pid)?.cpuTimeNs ?? 0),
    0,
  );
  const sum = (role: ProcessRole) =>
    processes.filter((p) => p.role === role).reduce((sum, p) => sum + p.footprintBytes, 0);
  return {
    elapsedMs,
    footprintBytes: sum("app") + sum("framework-helper"),
    mainProcessBytes: sum("app"),
    frameworkHelperBytes: sum("framework-helper"),
    excludedOsServiceBytes: sum("os-service"),
    rssBytes: included.reduce((sum, p) => sum + p.rssBytes, 0),
    cpuPercent:
      sameProcesses && elapsedNs > 0 && cpuDelta >= 0 ? (cpuDelta / elapsedNs) * 100 : null,
    processes,
  };
}

export function isSettled(history: MemorySample[]): boolean {
  const last = history.at(-1);
  if (!last || last.elapsedMs < idlePolicy.minimumWaitMs) return false;
  const start = history.findLastIndex(
    (sample) => sample.elapsedMs <= last.elapsedMs - idlePolicy.stableWindowMs,
  );
  if (start < 0) return false;
  const window = history.slice(start);
  // CPU at each sample describes the interval ending at that sample.
  if (
    window
      .slice(1)
      .some(
        (sample) => sample.cpuPercent === null || sample.cpuPercent > idlePolicy.maximumCpuPercent,
      )
  )
    return false;
  for (const field of ["mainProcessBytes", "frameworkHelperBytes"] as const) {
    const values = window.map((sample) => sample[field]);
    const allowed = Math.max(
      idlePolicy.maximumMemorySpreadBytes,
      median(values) * idlePolicy.maximumMemorySpreadRatio,
    );
    if (Math.max(...values) - Math.min(...values) > allowed) return false;
  }
  return true;
}

/** Accept ten consecutive idle samples after the initial stable interval. A late
 * cleanup or CPU burst restarts the observation; it never lowers the thresholds. */
export function idleMeasurement(history: MemorySample[]) {
  const start = history.length - idlePolicy.samplesPerRun;
  if (start < 1) return null;
  for (let end = start; end <= history.length; end++) {
    if (!isSettled(history.slice(0, end))) return null;
  }
  return {
    idleDetectedAtMs: history[start - 1]!.elapsedMs,
    startupSamples: history.slice(0, start),
    samples: history.slice(start),
  };
}
