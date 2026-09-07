/** Sparkle-compatible appcasts and raw Ed25519 signatures, shared by all updater backends. */
import { createPrivateKey, createPublicKey, generateKeyPairSync, sign } from "node:crypto";
import {
  chmodSync,
  copyFileSync,
  existsSync,
  mkdirSync,
  readFileSync,
  renameSync,
  statSync,
  writeFileSync,
} from "node:fs";
import { basename, join, resolve } from "node:path";
import type { ResolvedQuickGuiConfig } from "../config.ts";
import { CliError } from "../error.ts";
import type { QuickGuiTarget } from "../targets.ts";
import { targetInfo } from "../targets.ts";

const privatePrefix = Buffer.from("302e020100300506032b657004220420", "hex");
export function sparklePrivateKey(secret: string) {
  const bytes = Buffer.from(secret.trim(), "base64");
  if (bytes.length !== 32 || bytes.toString("base64") !== secret.trim())
    throw new CliError("Sparkle private key must be a base64 32-byte Ed25519 seed");
  const key = createPrivateKey({
    key: Buffer.concat([privatePrefix, bytes]),
    format: "der",
    type: "pkcs8",
  });
  return key;
}
export function sparklePublicKey(key: ReturnType<typeof createPrivateKey>): string {
  const jwk = createPublicKey(key).export({ format: "jwk" });
  if (!jwk.x) throw new CliError("Missing Ed25519 public key");
  return Buffer.from(jwk.x, "base64url").toString("base64");
}
export function generateUpdaterKeys(
  directory: string,
  force = false,
): { publicKeyPath: string; secretKeyPath: string } {
  const publicKeyPath = resolve(directory, "quickgui-update.pub"),
    secretKeyPath = resolve(directory, "quickgui-update.key");
  if (!force && [publicKeyPath, secretKeyPath].some(existsSync))
    throw new CliError("An updater key already exists; use --force to replace it");
  const { privateKey } = generateKeyPairSync("ed25519");
  const jwk = privateKey.export({ format: "jwk" });
  const publicKey = sparklePublicKey(privateKey);
  const secret = Buffer.from(jwk.d!, "base64url");
  mkdirSync(directory, { recursive: true });
  writeFileSync(secretKeyPath, secret.toString("base64") + "\n", {
    mode: 0o600,
    flag: force ? "w" : "wx",
  });
  chmodSync(secretKeyPath, 0o600);
  writeFileSync(publicKeyPath, publicKey + "\n", { flag: force ? "w" : "wx" });
  return { publicKeyPath, secretKeyPath };
}
export function updaterMetadata(
  config: ResolvedQuickGuiConfig,
  target: QuickGuiTarget,
  mode: "development" | "production",
) {
  return {
    feedUrl: (
      config.updates?.feedUrl ?? `${config.updates?.baseUrl ?? ""}/appcast-${target}.xml`
    ).replaceAll("{target}", target),
    publicKey: config.updates?.publicKey ?? "",
    currentVersion: config.version,
    identifier: mode === "development" ? `${config.identifier}.dev` : config.identifier,
    automaticChecks: config.updates?.automaticChecks ?? true,
    development: mode === "development",
  };
}
const xml = (value: string): string =>
  value
    .replaceAll("&", "&amp;")
    .replaceAll("<", "&lt;")
    .replaceAll(">", "&gt;")
    .replaceAll('"', "&quot;")
    .replaceAll("'", "&apos;");
export interface AppcastInput {
  name: string;
  version: string;
  buildVersion?: string;
  target: QuickGuiTarget;
  url: string;
  bytes: Uint8Array;
  privateKey: string;
  publicKey: string;
  notes?: string;
  minimumSystemVersion?: string;
}
export function renderAppcast(input: AppcastInput): string {
  if (!/^\d+\.\d+\.\d+(?:-[0-9A-Za-z.-]+)?(?:\+[0-9A-Za-z.-]+)?$/.test(input.version))
    throw new CliError("Appcast versions must be semantic versions");
  const url = new URL(input.url);
  if (url.protocol !== "https:" || url.username || url.password || url.hash)
    throw new CliError("Appcast enclosure must use HTTPS without credentials or fragments");
  if (!input.bytes.byteLength || input.bytes.byteLength > 512 * 1024 * 1024)
    throw new CliError("Update artifacts must be between 1 byte and 512 MiB");
  if (Buffer.byteLength(input.notes ?? "") > 16 * 1024)
    throw new CliError("Appcast notes must be at most 16 KiB");
  const key = sparklePrivateKey(input.privateKey);
  if (sparklePublicKey(key) !== input.publicKey)
    throw new CliError("Update signing key does not match updates.publicKey");
  const signature = sign(null, input.bytes, key).toString("base64");
  const platform = targetInfo(input.target).platform;
  return `<?xml version="1.0" encoding="utf-8"?>
<rss version="2.0" xmlns:sparkle="http://www.andymatuschak.org/xml-namespaces/sparkle">
  <channel>
    <title>${xml(input.name)} (${input.target})</title>
    <item>
      <title>${xml(input.version)}</title>
      <sparkle:version>${xml(platform === "darwin" ? (input.buildVersion ?? input.version) : input.version)}</sparkle:version>
      <sparkle:shortVersionString>${xml(input.version)}</sparkle:shortVersionString>
      <pubDate>${new Date().toUTCString()}</pubDate>
      <description>${xml(input.notes ?? "")}</description>
${platform === "darwin" && input.minimumSystemVersion ? `      <sparkle:minimumSystemVersion>${xml(input.minimumSystemVersion)}</sparkle:minimumSystemVersion>\n` : ""}      <enclosure url="${xml(input.url)}" length="${input.bytes.byteLength}" type="application/octet-stream" sparkle:edSignature="${signature}" sparkle:os="${platform === "darwin" ? "macos" : platform}" />
    </item>
  </channel>
</rss>
`;
}
export async function writeAppcast(input: {
  config: ResolvedQuickGuiConfig;
  target: QuickGuiTarget;
  outputDirectory: string;
  source: string;
  baseUrl: string;
  run: (argv: string[], cwd: string) => Promise<void>;
}) {
  const { config, target } = input;
  let artifactPath = input.source;
  if (targetInfo(target).platform === "darwin") {
    artifactPath = join(
      input.outputDirectory,
      `${config.executableName}-${config.version}-${target}.zip`,
    );
    await input.run(
      ["ditto", "-c", "-k", "--sequesterRsrc", "--keepParent", input.source, artifactPath],
      config.projectRoot,
    );
  } else if (targetInfo(target).platform === "linux" && !artifactPath.endsWith(".AppImage"))
    throw new CliError("The updater extension requires an AppImage on Linux");
  else if (targetInfo(target).platform === "windows" && !artifactPath.endsWith(".exe"))
    throw new CliError("The updater extension requires a QuickGUI NSIS installer on Windows");
  if (targetInfo(target).platform !== "darwin") {
    // Different architectures may share a release directory and base URL.
    const extension = targetInfo(target).platform === "windows" ? "exe" : "AppImage";
    const publishedPath = join(
      input.outputDirectory,
      `${config.executableName}-${config.version}-${target}.${extension}`,
    );
    if (resolve(artifactPath) !== resolve(publishedPath)) copyFileSync(artifactPath, publishedPath);
    artifactPath = publishedPath;
  }
  const secretPath = config.updates?.ed25519SecretKey;
  if (secretPath && statSync(secretPath).size > 1024)
    throw new CliError("Update signing key file is too large");
  const secret =
    process.env.QUICKGUI_UPDATER_PRIVATE_KEY ??
    process.env.SPARKLE_PRIVATE_KEY ??
    (secretPath ? readFileSync(secretPath, "utf8") : undefined);
  if (!secret || !config.updates?.publicKey)
    throw new CliError(
      "Set updates.publicKey and QUICKGUI_UPDATER_PRIVATE_KEY (or updates.ed25519SecretKey) to sign the appcast",
    );
  if (statSync(artifactPath).size > 512 * 1024 * 1024)
    throw new CliError("Update artifact exceeds 512 MiB");
  const url = `${input.baseUrl.replace(/\/+$/, "")}/${encodeURIComponent(basename(artifactPath))}`;
  if (config.updates.notesFile && statSync(config.updates.notesFile).size > 16 * 1024)
    throw new CliError("Appcast notes must be at most 16 KiB");
  const manifest = renderAppcast({
    name: config.name,
    version: config.version,
    buildVersion: config.buildVersion,
    target,
    url,
    bytes: readFileSync(artifactPath),
    privateKey: secret,
    publicKey: config.updates.publicKey,
    ...(config.updates.notesFile ? { notes: readFileSync(config.updates.notesFile, "utf8") } : {}),
    minimumSystemVersion: config.macos.minimumSystemVersion,
  });
  const manifestPath = join(input.outputDirectory, `appcast-${target}.xml`);
  const temporary = `${manifestPath}.tmp-${process.pid}`;
  writeFileSync(temporary, manifest);
  renameSync(temporary, manifestPath);
  return { artifactPath, manifestPath, url };
}
