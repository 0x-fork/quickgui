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

  downloadAndStage(
    update: AvailableUpdate,
    destinationDirectory: string,
    options: UpdateClientOptions,
  ): Promise<string> {
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
