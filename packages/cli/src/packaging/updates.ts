/**
 * Signed updater artifacts and `latest.json` generation.
 *
 * The shapes here mirror `crates/quickgui-system/src/update.rs` exactly: the manifest is the
 * `Manifest` struct that `UpdateClient::parse_manifest` deserializes, and the per-target artifact
 * is the layout `install_staged` accepts on each platform.
 */

import { existsSync, readFileSync, statSync } from "node:fs";
import { basename } from "node:path";

import { CliError } from "../error.ts";
import { targetInfo, type QuickGuiTarget } from "../targets.ts";

/** Largest release-notes body copied into a manifest. The core caps a manifest at 1 MiB. */
export const MAX_UPDATE_NOTES_BYTES = 64 * 1024;
/** Largest Minisign signature file the CLI reads. Matches `MAX_UPDATE_SIGNATURE_BYTES`. */
export const MAX_UPDATE_SIGNATURE_BYTES = 64 * 1024;

export interface ManifestPlatform {
  url: string;
  signature: string;
}

/** The exact JSON `UpdateClient::parse_manifest` accepts. */
export interface UpdateManifest {
  version: string;
  notes?: string;
  pub_date?: string;
  url?: string;
  signature?: string;
  platforms?: Record<string, ManifestPlatform>;
}

export interface ManifestInput {
  version: string;
  baseUrl: string;
  target: QuickGuiTarget;
  artifactName: string;
  signature: string;
  notes?: string;
  publishedAt?: Date;
  /** Emit the flat `url`/`signature` form instead of the `platforms` map. */
  flat?: boolean;
  /** Existing manifest whose other platform entries are preserved. */
  existing?: UpdateManifest;
}

/**
 * Map a QuickGUI CLI target to the updater target string produced by `default_update_target()`.
 *
 * The Rust helper renders `<system>-<arch>` with `macos` reported as `darwin` and `std::env::consts::ARCH`
 * as the architecture, so `aarch64` — not `arm64` — is the macOS/Linux/Windows ARM name.
 */
export function updateTarget(target: QuickGuiTarget): string {
  const info = targetInfo(target);
  const system = info.platform === "darwin" ? "darwin" : info.platform;
  const architecture = info.architecture === "arm64" ? "aarch64" : "x86_64";
  return `${system}-${architecture}`;
}

/**
 * Filename of the artifact the Rust updater installs for `target`.
 *
 * The Linux name matches the AppImage the packaging pipeline produces, which uses the same
 * `aarch64`/`x86_64` architecture spelling as `default_update_target()`.
 */
export function updateArtifactName(
  target: QuickGuiTarget,
  executableName: string,
  version: string,
): string {
  const info = targetInfo(target);
  if (info.platform === "darwin") return `${executableName}.app.tar.gz`;
  if (info.platform === "windows") return `${executableName}-${version}-setup.exe`;
  const architecture = info.architecture === "arm64" ? "aarch64" : "x86_64";
  return `${executableName}-${version}-${architecture}.AppImage.tar.gz`;
}

/** RFC 3339 UTC timestamp with whole-second precision, matching the core's report format. */
export function rfc3339(date: Date): string {
  if (Number.isNaN(date.getTime())) throw new CliError("Invalid update publication date");
  return `${date.toISOString().slice(0, 19)}Z`;
}

export function joinUrl(baseUrl: string, name: string): string {
  if (!/^https:\/\/[^\s"']+$/.test(baseUrl)) {
    throw new CliError(`\`updates.baseUrl\` must be an HTTPS URL, got \`${baseUrl}\``);
  }
  return `${baseUrl.replace(/\/+$/, "")}/${encodeURIComponent(name)}`;
}

/** Build the manifest object written to `latest.json`. */
export function buildUpdateManifest(input: ManifestInput): UpdateManifest {
  if (!/^\d+\.\d+\.\d+(?:[-+][0-9A-Za-z.-]+)*$/.test(input.version)) {
    throw new CliError(
      `An update manifest needs a semantic version, got \`${input.version}\`. Set \`version\` in quickgui.config.ts.`,
    );
  }
  if (input.notes !== undefined && input.notes.length > MAX_UPDATE_NOTES_BYTES) {
    throw new CliError(`Release notes exceed ${MAX_UPDATE_NOTES_BYTES} bytes`);
  }
  if (input.signature.length === 0 || input.signature.length > MAX_UPDATE_SIGNATURE_BYTES) {
    throw new CliError(
      `An update signature must be between 1 and ${MAX_UPDATE_SIGNATURE_BYTES} bytes`,
    );
  }
  const url = joinUrl(input.baseUrl, input.artifactName);
  const manifest: UpdateManifest = {
    version: input.version,
    pub_date: rfc3339(input.publishedAt ?? new Date()),
  };
  if (input.notes !== undefined && input.notes.length > 0) manifest.notes = input.notes;
  if (input.flat) {
    manifest.url = url;
    manifest.signature = input.signature;
    return manifest;
  }
  manifest.platforms = {
    ...(input.existing?.version === input.version ? input.existing.platforms : undefined),
    [updateTarget(input.target)]: { url, signature: input.signature },
  };
  return manifest;
}

export function serializeUpdateManifest(manifest: UpdateManifest): string {
  return `${JSON.stringify(manifest, null, 2)}\n`;
}

export type MinisignTool = "minisign" | "rsign";

export interface MinisignSigningInput {
  tool: MinisignTool;
  secretKey: string;
  artifact: string;
  signaturePath: string;
  comment?: string;
  trustedComment?: string;
}

/**
 * Command line for signing one artifact.
 *
 * `minisign -S` and `rsign sign` accept the same inputs under different flag names; QuickGUI
 * builds both here so the choice is testable without either tool installed. Neither signing
 * subcommand takes a password flag: an unencrypted key prompts for nothing, and an encrypted key
 * reads its password from standard input (QuickGUI supplies `QUICKGUI_MINISIGN_PASSWORD` there).
 */
export function minisignSignArguments(input: MinisignSigningInput): string[] {
  if (input.tool === "minisign") {
    return [
      "minisign",
      "-S",
      "-s",
      input.secretKey,
      "-m",
      input.artifact,
      "-x",
      input.signaturePath,
      ...(input.comment ? ["-c", input.comment] : []),
      ...(input.trustedComment ? ["-t", input.trustedComment] : []),
    ];
  }
  return [
    "rsign",
    "sign",
    "-s",
    input.secretKey,
    "-x",
    input.signaturePath,
    ...(input.comment ? ["-c", input.comment] : []),
    ...(input.trustedComment ? ["-t", input.trustedComment] : []),
    input.artifact,
  ];
}

/** Command line for creating a new Minisign key pair. */
export function minisignKeygenArguments(
  tool: MinisignTool,
  publicKeyPath: string,
  secretKeyPath: string,
  passwordless: boolean,
): string[] {
  if (tool === "minisign") {
    return [
      "minisign",
      "-G",
      "-p",
      publicKeyPath,
      "-s",
      secretKeyPath,
      ...(passwordless ? ["-W"] : []),
    ];
  }
  return [
    "rsign",
    "generate",
    "-p",
    publicKeyPath,
    "-s",
    secretKeyPath,
    "-f",
    ...(passwordless ? ["-W"] : []),
  ];
}

/** Read a `.minisig` file and return the single-line base64 form the manifest carries. */
export function readSignature(signaturePath: string): string {
  if (!existsSync(signaturePath) || !statSync(signaturePath).isFile()) {
    throw new CliError(`Minisign did not produce a signature at ${signaturePath}`);
  }
  const text = readFileSync(signaturePath, "utf8");
  if (text.length === 0 || text.length > MAX_UPDATE_SIGNATURE_BYTES) {
    throw new CliError(`Signature file is empty or larger than ${MAX_UPDATE_SIGNATURE_BYTES} bytes`);
  }
  return Buffer.from(text, "utf8").toString("base64");
}

/** Resolve the Minisign secret key from configuration or `QUICKGUI_MINISIGN_SECRET_KEY`. */
export function resolveSecretKey(configured: string | undefined): string {
  const key = process.env.QUICKGUI_MINISIGN_SECRET_KEY ?? configured;
  if (!key) {
    throw new CliError(
      "Signing an update requires a Minisign secret key. Set `updates.minisignSecretKey` or the " +
        "QUICKGUI_MINISIGN_SECRET_KEY environment variable, or create one with `quickgui keygen`.",
    );
  }
  if (!existsSync(key) || !statSync(key).isFile()) {
    throw new CliError(`Minisign secret key not found: ${key}`);
  }
  return key;
}

/** `tar` arguments that create the gzip archive the macOS/Linux updater expects. */
export function updateArchiveArguments(
  parentDirectory: string,
  memberName: string,
  archivePath: string,
): string[] {
  return ["tar", "-czf", archivePath, "-C", parentDirectory, basename(memberName)];
}
