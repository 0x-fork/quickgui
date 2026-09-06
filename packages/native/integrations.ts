/**
 * Background system integrations: autostart, protocol registration, secure storage, updates,
 * crash reporting, and process metrics. Each call runs on a host worker thread and resolves on
 * the application thread; capability checks answer synchronously.
 */

import { decodeBase64, encodeBase64 } from "./base64.ts";
import { allocateRequest, callService, isNullJson, sendInvoke } from "./requests.ts";

export type AutoStartMode = "native" | "macos-launch-agent" | "macos-apple-script" | "linux-systemd" | "windows-system";

export interface AutoStartOptions {
  appName: string;
  executable?: string;
  arguments?: string[];
  mode?: AutoStartMode;
  bundleIdentifier?: string;
}

export interface ProtocolRegistrationOptions {
  scheme: string;
  appName: string;
  appId: string;
  executable?: string;
  arguments?: string[];
}

export interface UpdateClientOptions {
  currentVersion: string;
  publicKey: string;
  target?: string;
  maximumDownloadBytes?: number;
}

export interface AvailableUpdate {
  version: string;
  currentVersion: string;
  target: string;
  url: string;
  signature: string;
  notes?: string;
  publishedAt?: string;
}

export interface UpdateInstallOptions {
  targetExecutable?: string;
  retainBackup?: boolean;
  windowsMode?: "basic-ui" | "quiet" | "passive";
  installerArguments?: string[];
}

export interface InstalledUpdate {
  version: string;
  disposition: string;
  installedPath: string;
  backupPath?: string;
  installerProcessId?: number;
  requiresApplicationExit: boolean;
  relaunchRecommended: boolean;
}

export interface UpdateProgress {
  phase: string;
  chunkBytes?: number;
  downloadedBytes?: number;
  totalBytes?: number;
  path?: string;
}

export interface CrashParameter {
  key: string;
  value: string;
}

export interface CrashReporterOptions {
  appName: string;
  appVersion: string;
  appIdentifier: string;
  directory?: string;
  maxReports?: number;
  maxReportBytes?: number;
  parameters?: CrashParameter[];
  uploadEndpoint?: string;
  backtrace?: "disabled" | "environment" | "always";
  captureSignals?: boolean;
}

export interface CrashLocation {
  file: string;
  line: number;
  column: number;
}

export interface CrashReport {
  schemaVersion: number;
  id: string;
  kind: string;
  timestamp: string;
  appName: string;
  appVersion: string;
  appIdentifier: string;
  operatingSystem: string;
  operatingSystemVersion?: string;
  architecture: string;
  processId: number;
  thread?: string;
  message: string;
  location?: CrashLocation;
  backtrace?: string;
  signal?: number;
  signalName?: string;
  faultAddress?: string;
  parameters: CrashParameter[];
}

export interface CrashUploadSummary {
  attempted: number;
  uploaded: number;
  failed: number;
}

export interface ProcessMetrics {
  cpuUserSeconds: number;
  cpuSystemSeconds: number;
  residentBytes: number;
  footprintBytes?: number;
  virtualBytes: number;
  threadCount?: number;
  uptimeSeconds: number;
}

export interface SystemMemory {
  totalBytes: number;
  availableBytes: number;
  freeBytes: number;
  usedBytes: number;
}

export interface CpuUsage {
  percent?: number;
  intervalSeconds: number;
  cpuSeconds: number;
  totalCpuSeconds: number;
}

async function invokeVoid(method: string, params: string): Promise<void> {
  await sendInvoke(method, params, undefined);
}

async function invokeBoolean(method: string, params: string): Promise<boolean> {
  return (await sendInvoke(method, params, undefined)) === "true";
}

export class AutoStart {
  static isSupported(): boolean {
    return callService("is-auto-start-supported", "") === "true";
  }
  static enable(options: AutoStartOptions): Promise<void> {
    return invokeVoid("enable-auto-start", JSON.stringify(options));
  }
  static disable(options: AutoStartOptions): Promise<void> {
    return invokeVoid("disable-auto-start", JSON.stringify(options));
  }
  static isEnabled(options: AutoStartOptions): Promise<boolean> {
    return invokeBoolean("is-auto-start-enabled", JSON.stringify(options));
  }
}

export class Protocol {
  static supportsDynamicRegistration(): boolean {
    return callService("supports-dynamic-protocol-registration", "") === "true";
  }
  static register(options: ProtocolRegistrationOptions): Promise<boolean> {
    return invokeBoolean("register-protocol", JSON.stringify(options));
  }
  static unregister(options: ProtocolRegistrationOptions): Promise<boolean> {
    return invokeBoolean("unregister-protocol", JSON.stringify(options));
  }
  static isRegistered(options: ProtocolRegistrationOptions): Promise<boolean> {
    return invokeBoolean("is-protocol-registered", JSON.stringify(options));
  }
}

/** Custom URL schemes: the URLs this launch was asked to open, and dynamic registration. */
export class DeepLink {
  /** Deep links passed on the command line of this launch. */
  static getLaunchUrls(): string[] {
    return urlsFromArguments(process.argv.slice(1));
  }
  static supportsDynamicRegistration(): boolean {
    return Protocol.supportsDynamicRegistration();
  }
  static register(options: ProtocolRegistrationOptions): Promise<boolean> {
    return Protocol.register(options);
  }
  static unregister(options: ProtocolRegistrationOptions): Promise<boolean> {
    return Protocol.unregister(options);
  }
  static isRegistered(options: ProtocolRegistrationOptions): Promise<boolean> {
    return Protocol.isRegistered(options);
  }
}

/** The custom-scheme URLs among a launch's arguments; `http(s)` links and ordinary paths are skipped. */
export function urlsFromArguments(argv: string[]): string[] {
  const urls: string[] = [];
  for (const argument of argv) {
    let href: string | undefined = undefined;
    try {
      const url = new URL(argument);
      if (url.protocol !== "http:" && url.protocol !== "https:") href = url.href;
    } catch {
      // Ordinary CLI flags and filesystem paths are not deep links.
    }
    if (href !== undefined) urls.push(href);
  }
  return urls;
}

export class SecureStorage {
  static isSupported(): boolean {
    return callService("is-secure-storage-supported", "") === "true";
  }
  static set(service: string, account: string, value: Uint8Array): Promise<boolean> {
    return invokeBoolean("set-secure-storage", JSON.stringify({ service, account, value: encodeBase64(value) }));
  }
  static setText(service: string, account: string, value: string): Promise<boolean> {
    return SecureStorage.set(service, account, new TextEncoder().encode(value));
  }
  static async get(service: string, account: string): Promise<Uint8Array | undefined> {
    const json = await sendInvoke("get-secure-storage", JSON.stringify({ service, account }), undefined);
    if (isNullJson(json)) return undefined;
    return decodeBase64(JSON.parse(json) as string);
  }
  static async getText(service: string, account: string): Promise<string | undefined> {
    const bytes = await SecureStorage.get(service, account);
    return bytes === undefined ? undefined : new TextDecoder().decode(bytes);
  }
  static delete(service: string, account: string): Promise<boolean> {
    return invokeBoolean("delete-secure-storage", JSON.stringify({ service, account }));
  }
}

export class Updater {
  static defaultTarget(): string {
    return JSON.parse(callService("default-update-target", "")) as string;
  }
  static async check(endpoint: string, options: UpdateClientOptions): Promise<AvailableUpdate | undefined> {
    const json = await sendInvoke("check-for-update", JSON.stringify({ endpoint, options }), undefined);
    return isNullJson(json) ? undefined : (JSON.parse(json) as AvailableUpdate);
  }
  static async stage(
    update: AvailableUpdate,
    destinationDirectory: string,
    options: UpdateClientOptions,
    onProgress?: (progress: UpdateProgress) => void,
  ): Promise<string> {
    const listener = onProgress === undefined
      ? undefined
      : (json: string): void => {
          onProgress(JSON.parse(json) as UpdateProgress);
        };
    const json = await sendInvoke(
      "stage-update",
      JSON.stringify({ update, destinationDirectory, options, progress: onProgress !== undefined }),
      listener,
    );
    return JSON.parse(json) as string;
  }
  static verify(path: string, signature: string, options: UpdateClientOptions): Promise<void> {
    return invokeVoid("verify-update", JSON.stringify({ path, signature, options }));
  }
  static async install(
    update: AvailableUpdate,
    artifact: string,
    options: UpdateClientOptions,
    installOptions: UpdateInstallOptions = {},
  ): Promise<InstalledUpdate> {
    const json = await sendInvoke("install-update", JSON.stringify({ update, artifact, installOptions, options }), undefined);
    return JSON.parse(json) as InstalledUpdate;
  }
}

export class CrashReporter {
  static isStarted(): boolean {
    return callService("is-crash-reporter-started", "") === "true";
  }
  static async start(options: CrashReporterOptions): Promise<string> {
    const json = await sendInvoke("start-crash-reporter", JSON.stringify(options), undefined);
    return JSON.parse(json) as string;
  }
  static async getLastCrashReport(): Promise<CrashReport | undefined> {
    const json = await sendInvoke("get-last-crash-report", "{}", undefined);
    const reports = JSON.parse(json) as CrashReport[];
    return reports.length > 0 ? reports[0] : undefined;
  }
  static async getPendingReports(): Promise<CrashReport[]> {
    const json = await sendInvoke("get-pending-crash-reports", "{}", undefined);
    return JSON.parse(json) as CrashReport[];
  }
  static addExtraParameter(key: string, value: string): Promise<boolean> {
    return invokeBoolean("add-crash-extra-parameter", JSON.stringify({ key, value }));
  }
  static removeExtraParameter(key: string): Promise<boolean> {
    return invokeBoolean("remove-crash-extra-parameter", JSON.stringify({ key }));
  }
  static deleteReport(id: string): Promise<boolean> {
    return invokeBoolean("delete-crash-report", JSON.stringify({ id }));
  }
  static async uploadPending(endpoint?: string): Promise<CrashUploadSummary> {
    const params = endpoint === undefined ? "{}" : JSON.stringify({ endpoint });
    const json = await sendInvoke("upload-pending-crash-reports", params, undefined);
    return JSON.parse(json) as CrashUploadSummary;
  }
}

/** Stateful CPU sampler. Each `sample()` reports usage since the previous call on this instance. */
export class CpuSampler {
  readonly id: number;

  constructor() {
    this.id = Number(callService("cpu-sampler-create", ""));
  }

  sample(): CpuUsage {
    return JSON.parse(callService("cpu-sampler-sample", JSON.stringify({ id: this.id }))) as CpuUsage;
  }

  release(): void {
    callService("cpu-sampler-release", JSON.stringify({ id: this.id }));
  }
}

export class Metrics {
  static async getProcessMetrics(): Promise<ProcessMetrics> {
    const json = await sendInvoke("get-process-metrics", "{}", undefined);
    return JSON.parse(json) as ProcessMetrics;
  }
  static async getSystemMemory(): Promise<SystemMemory> {
    const json = await sendInvoke("get-system-memory", "{}", undefined);
    return JSON.parse(json) as SystemMemory;
  }
}

export interface FileWatchEvent {
  paths: string[];
  rescan: boolean;
  error?: string | null;
}

const fileWatchers = new Map<number, FileWatcher>();

/** Native recursive file notifications; registration and close complete asynchronously. */
export class FileWatcher {
  readonly id: number;
  #listener: (event: FileWatchEvent) => void;
  #closed = false;
  private constructor(id: number, listener: (event: FileWatchEvent) => void) { this.id = id; this.#listener = listener; }

  static async start(paths: string[], listener: (event: FileWatchEvent) => void): Promise<FileWatcher> {
    const watcher = new FileWatcher(allocateRequest(), listener);
    fileWatchers.set(watcher.id, watcher);
    try {
      await sendInvoke("watch-files", JSON.stringify({ id: watcher.id, paths }), undefined);
      return watcher;
    } catch (error) {
      fileWatchers.delete(watcher.id);
      throw error;
    }
  }

  async close(): Promise<void> {
    if (this.#closed) return;
    this.#closed = true;
    fileWatchers.delete(this.id);
    await sendInvoke("unwatch-files", JSON.stringify({ id: this.id }), undefined);
  }

  /** @internal */
  _dispatch(event: FileWatchEvent): void { if (!this.#closed) this.#listener(event); }
}

/** @internal */
export function dispatchFileWatch(id: number, json: string | undefined): void {
  const watcher = fileWatchers.get(id);
  if (watcher === undefined || json === undefined) return;
  const event: FileWatchEvent = JSON.parse(json);
  watcher._dispatch(event);
}
