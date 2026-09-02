import { Buffer } from "node:buffer";
import { resolve as resolvePath } from "node:path";
import * as binding from "./binding.js";

export type AutoStartMode =
  | "native"
  | "macos-launch-agent"
  | "macos-apple-script"
  | "linux-systemd"
  | "windows-system";

export interface AutoStartOptions {
  /** Portable registration identifier: ASCII letters, digits, dots, underscores, and hyphens. */
  appName: string;
  /** Defaults to the current packaged executable. */
  executable?: string;
  arguments?: readonly string[];
  mode?: AutoStartMode;
  /** Required by some macOS autostart modes. */
  bundleIdentifier?: string;
}

export interface ProtocolRegistrationOptions {
  scheme: string;
  appName: string;
  /** Stable reverse-DNS identifier used for Linux desktop integration. */
  appId: string;
  /** Defaults to the current packaged executable. */
  executable?: string;
  arguments?: readonly string[];
}

export interface UpdateClientOptions {
  currentVersion: string;
  /** Minisign public key, either raw or in Minisign's public-key file format. */
  publicKey: string;
  /** Defaults to QuickGUI's current `system-architecture` target. */
  target?: string;
  /** Hard download limit. The core caps this at 2 GiB. */
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
  installerArguments?: readonly string[];
}

export interface InstalledUpdate {
  version: string;
  disposition: "applied" | "installer-launched";
  installedPath: string;
  backupPath?: string;
  installerProcessId?: number;
  requiresApplicationExit: boolean;
  relaunchRecommended: boolean;
}

export type CrashKind = "panic" | "signal" | "hang";

export type CrashBacktracePolicy = "disabled" | "environment" | "always";

export interface CrashReporterOptions {
  appName: string;
  appVersion: string;
  appIdentifier: string;
  /** Defaults to `<log directory>/crashes`. Must be an absolute path. */
  directory?: string;
  /** Retained reports, 1..=128. Defaults to 16. */
  maxReports?: number;
  /** Bytes per report, 1024..=1048576. Defaults to 65536. */
  maxReportBytes?: number;
  /** Bounded key/value pairs stored in every report. At most 32 entries. */
  parameters?: Readonly<Record<string, string>>;
  /** Remembered HTTPS endpoint for `uploadPending()`. */
  uploadEndpoint?: string;
  /** Panic backtrace policy. Defaults to `environment` (`RUST_BACKTRACE`). */
  backtrace?: CrashBacktracePolicy;
  /** Install the native fatal-signal handler. Defaults to `true`. */
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
  kind: CrashKind;
  /** RFC 3339 UTC timestamp. */
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
  parameters: Record<string, string>;
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
  /** macOS `phys_footprint`. Undefined on other platforms. */
  footprintBytes?: number;
  virtualBytes: number;
  /** Undefined where the platform has no inexpensive thread count (Windows). */
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
  /** Percent of one core since the previous sample; undefined for the first sample. */
  percent?: number;
  intervalSeconds: number;
  cpuSeconds: number;
  totalCpuSeconds: number;
}

export interface CpuSampler {
  sample(): Promise<CpuUsage>;
}

export type UpdateProgressPhase =
  | "download-started"
  | "downloaded"
  | "download-finished"
  | "verification-started"
  | "verification-finished"
  | "staged";

export interface UpdateProgress {
  phase: UpdateProgressPhase;
  chunkBytes?: number;
  downloadedBytes?: number;
  totalBytes?: number;
  path?: string;
}

export interface UpdateStageOptions {
  onProgress?: (progress: UpdateProgress) => void;
}

/**
 * Bounded crash reporting backed by the Rust core.
 *
 * `start()` installs the process-wide panic hook and, unless `captureSignals` is false, a native
 * fatal-fault handler that writes a minimal pre-rendered report from an async-signal-safe path.
 */
export const CrashReporter = Object.freeze({
  isStarted(): boolean {
    return binding.isCrashReporterStarted();
  },

  /** Install the reporter and resolve with the directory holding retained reports. */
  start(options: CrashReporterOptions): Promise<string> {
    return binding.startCrashReporter(nativeCrashReporterOptions(options));
  },

  async getLastCrashReport(): Promise<CrashReport | undefined> {
    const reports = await binding.getLastCrashReport();
    return reports.length > 0 ? normalizeCrashReport(reports[0]!) : undefined;
  },

  async getPendingReports(): Promise<CrashReport[]> {
    const reports = await binding.getPendingCrashReports();
    return reports.map(normalizeCrashReport);
  },

  async addExtraParameter(key: string, value: string): Promise<void> {
    await binding.addCrashExtraParameter(key, value);
  },

  removeExtraParameter(key: string): Promise<boolean> {
    return binding.removeCrashExtraParameter(key);
  },

  deleteReport(id: string): Promise<boolean> {
    return binding.deleteCrashReport(id);
  },

  /** POST every retained report to `endpoint`, deleting the ones the endpoint accepts. */
  uploadPending(endpoint?: string): Promise<CrashUploadSummary> {
    return binding.uploadPendingCrashReports(endpoint);
  },
});

/** Explicit, on-demand process and system readings. Nothing is sampled in the background. */
export const Metrics = Object.freeze({
  getProcessMetrics(): Promise<ProcessMetrics> {
    return binding.getProcessMetrics() as Promise<ProcessMetrics>;
  },

  getSystemMemory(): Promise<SystemMemory> {
    return binding.getSystemMemory() as Promise<SystemMemory>;
  },

  /** Create a stateful sampler; each `sample()` reports usage since that sampler's last call. */
  createCpuSampler(): CpuSampler {
    const sampler = new binding.NativeCpuUsageSampler();
    return Object.freeze({
      sample: () => sampler.sample() as Promise<CpuUsage>,
    });
  },
});

function nativeCrashReporterOptions(
  options: CrashReporterOptions,
): binding.NativeCrashReporterOptions {
  const native: binding.NativeCrashReporterOptions = {
    appName: options.appName,
    appVersion: options.appVersion,
    appIdentifier: options.appIdentifier,
  };
  if (options.directory !== undefined) native.directory = resolvePath(options.directory);
  if (options.maxReports !== undefined) native.maxReports = options.maxReports;
  if (options.maxReportBytes !== undefined) native.maxReportBytes = options.maxReportBytes;
  if (options.parameters !== undefined) {
    native.parameters = Object.entries(options.parameters).map(([key, value]) => ({ key, value }));
  }
  if (options.uploadEndpoint !== undefined) native.uploadEndpoint = options.uploadEndpoint;
  if (options.backtrace !== undefined) native.backtrace = options.backtrace;
  if (options.captureSignals !== undefined) native.captureSignals = options.captureSignals;
  return native;
}

export function normalizeCrashReport(report: binding.NativeCrashReport): CrashReport {
  const parameters: Record<string, string> = {};
  for (const parameter of report.parameters) parameters[parameter.key] = parameter.value;
  const normalized: CrashReport = {
    schemaVersion: report.schemaVersion,
    id: report.id,
    kind: report.kind as CrashKind,
    timestamp: report.timestamp,
    appName: report.appName,
    appVersion: report.appVersion,
    appIdentifier: report.appIdentifier,
    operatingSystem: report.operatingSystem,
    architecture: report.architecture,
    processId: report.processId,
    message: report.message,
    parameters,
  };
  if (report.operatingSystemVersion !== undefined) {
    normalized.operatingSystemVersion = report.operatingSystemVersion;
  }
  if (report.thread !== undefined) normalized.thread = report.thread;
  if (report.location !== undefined) normalized.location = { ...report.location };
  if (report.backtrace !== undefined) normalized.backtrace = report.backtrace;
  if (report.signal !== undefined) normalized.signal = report.signal;
  if (report.signalName !== undefined) normalized.signalName = report.signalName;
  if (report.faultAddress !== undefined) normalized.faultAddress = report.faultAddress;
  return normalized;
}

export function normalizeUpdateProgress(
  progress: binding.NativeUpdateProgress,
): UpdateProgress {
  const normalized: UpdateProgress = { phase: progress.phase as UpdateProgressPhase };
  if (progress.chunkBytes !== undefined) normalized.chunkBytes = progress.chunkBytes;
  if (progress.downloadedBytes !== undefined) {
    normalized.downloadedBytes = progress.downloadedBytes;
  }
  if (progress.totalBytes !== undefined) normalized.totalBytes = progress.totalBytes;
  if (progress.path !== undefined) normalized.path = progress.path;
  return normalized;
}
/** Native login-launch registration. Operations run outside the JavaScript event loop. */
export const AutoStart = Object.freeze({
  isSupported(): boolean {
    return binding.isAutoStartSupported();
  },

  enable(options: AutoStartOptions): Promise<void> {
    return binding.enableAutoStart(nativeAutoStartOptions(options));
  },

  disable(options: AutoStartOptions): Promise<void> {
    return binding.disableAutoStart(nativeAutoStartOptions(options));
  },

  isEnabled(options: AutoStartOptions): Promise<boolean> {
    return binding.isAutoStartEnabled(nativeAutoStartOptions(options));
  },
});

/** Native URL-scheme registration. macOS schemes are declared in the signed app bundle. */
export const Protocol = Object.freeze({
  supportsDynamicRegistration(): boolean {
    return binding.supportsDynamicProtocolRegistration();
  },

  async register(options: ProtocolRegistrationOptions): Promise<void> {
    await binding.registerProtocol(nativeProtocolOptions(options));
  },

  unregister(options: ProtocolRegistrationOptions): Promise<boolean> {
    return binding.unregisterProtocol(nativeProtocolOptions(options));
  },

  isRegistered(options: ProtocolRegistrationOptions): Promise<boolean> {
    return binding.isProtocolRegistered(nativeProtocolOptions(options));
  },
});

/** Keychain Services, Windows Credential Manager, or Secret Service. */
export const SecureStorage = Object.freeze({
  isSupported(): boolean {
    return binding.isSecureStorageSupported();
  },

  async set(service: string, account: string, secret: Uint8Array): Promise<void> {
    await binding.setSecureStorage(service, account, Buffer.from(secret));
  },

  async get(service: string, account: string): Promise<Uint8Array | undefined> {
    const secret = await binding.getSecureStorage(service, account);
    return secret === undefined ? undefined : new Uint8Array(secret);
  },

  async setText(service: string, account: string, value: string): Promise<void> {
    await binding.setSecureStorage(service, account, Buffer.from(value, "utf8"));
  },

  async getText(service: string, account: string): Promise<string | undefined> {
    const secret = await binding.getSecureStorage(service, account);
    return secret?.toString("utf8");
  },

  delete(service: string, account: string): Promise<boolean> {
    return binding.deleteSecureStorage(service, account);
  },
});

/** Signed update discovery, staging, mandatory re-verification, and native installation. */
export const Updater = Object.freeze({
  defaultTarget(): string {
    return binding.defaultUpdateTarget();
  },

  async check(
    endpoint: string,
    options: UpdateClientOptions,
  ): Promise<AvailableUpdate | undefined> {
    const update = await binding.checkForUpdate(endpoint, nativeUpdateOptions(options));
    return update === undefined ? undefined : normalizeAvailableUpdate(update);
  },

  /**
   * Download, stream-verify, and atomically stage an update artifact.
   *
   * Passing `onProgress` routes bounded native progress events onto the JavaScript thread through
   * a threadsafe function; the download itself still runs off the main thread.
   */
  downloadAndStage(
    update: AvailableUpdate,
    destinationDirectory: string,
    options: UpdateClientOptions,
    stageOptions?: UpdateStageOptions,
  ): Promise<string> {
    const onProgress = stageOptions?.onProgress;
    if (onProgress) {
      return binding.stageUpdateWithProgress(
        nativeAvailableUpdate(update),
        destinationDirectory,
        nativeUpdateOptions(options),
        (progress) => {
          onProgress(normalizeUpdateProgress(progress));
        },
      );
    }
    return binding.stageUpdate(
      nativeAvailableUpdate(update),
      destinationDirectory,
      nativeUpdateOptions(options),
    );
  },

  verify(
    path: string,
    signature: string,
    options: UpdateClientOptions,
  ): Promise<void> {
    return binding.verifyUpdate(path, signature, nativeUpdateOptions(options));
  },

  async install(
    update: AvailableUpdate,
    artifact: string,
    installOptions: UpdateInstallOptions,
    options: UpdateClientOptions,
  ): Promise<InstalledUpdate> {
    const native: binding.NativeUpdateInstallOptions = {};
    if (installOptions.targetExecutable !== undefined) {
      native.targetExecutable = resolvePath(installOptions.targetExecutable);
    }
    if (installOptions.retainBackup !== undefined) {
      native.retainBackup = installOptions.retainBackup;
    }
    if (installOptions.windowsMode !== undefined) native.windowsMode = installOptions.windowsMode;
    if (installOptions.installerArguments !== undefined) {
      native.installerArguments = [...installOptions.installerArguments];
    }
    const installed = await binding.installUpdate(
      nativeAvailableUpdate(update),
      resolvePath(artifact),
      native,
      nativeUpdateOptions(options),
    );
    return { ...installed } as InstalledUpdate;
  },
});

function nativeAutoStartOptions(options: AutoStartOptions): binding.NativeAutoStartOptions {
  const native: binding.NativeAutoStartOptions = { appName: options.appName };
  if (options.executable !== undefined) native.executable = options.executable;
  if (options.arguments !== undefined) native.arguments = [...options.arguments];
  if (options.mode !== undefined) native.mode = options.mode;
  if (options.bundleIdentifier !== undefined) {
    native.bundleIdentifier = options.bundleIdentifier;
  }
  return native;
}

function nativeProtocolOptions(
  options: ProtocolRegistrationOptions,
): binding.NativeProtocolRegistrationOptions {
  const native: binding.NativeProtocolRegistrationOptions = {
    scheme: options.scheme,
    appName: options.appName,
    appId: options.appId,
  };
  if (options.executable !== undefined) native.executable = options.executable;
  if (options.arguments !== undefined) native.arguments = [...options.arguments];
  return native;
}

function nativeUpdateOptions(options: UpdateClientOptions): binding.NativeUpdateClientOptions {
  const native: binding.NativeUpdateClientOptions = {
    currentVersion: options.currentVersion,
    publicKey: options.publicKey,
  };
  if (options.target !== undefined) native.target = options.target;
  if (options.maximumDownloadBytes !== undefined) {
    native.maximumDownloadBytes = options.maximumDownloadBytes;
  }
  return native;
}

function nativeAvailableUpdate(update: AvailableUpdate): binding.NativeAvailableUpdate {
  const native: binding.NativeAvailableUpdate = {
    version: update.version,
    currentVersion: update.currentVersion,
    target: update.target,
    url: update.url,
    signature: update.signature,
  };
  if (update.notes !== undefined) native.notes = update.notes;
  if (update.publishedAt !== undefined) native.publishedAt = update.publishedAt;
  return native;
}

function normalizeAvailableUpdate(update: binding.NativeAvailableUpdate): AvailableUpdate {
  const normalized: AvailableUpdate = {
    version: update.version,
    currentVersion: update.currentVersion,
    target: update.target,
    url: update.url,
    signature: update.signature,
  };
  if (update.notes !== undefined) normalized.notes = update.notes;
  if (update.publishedAt !== undefined) normalized.publishedAt = update.publishedAt;
  return normalized;
}
