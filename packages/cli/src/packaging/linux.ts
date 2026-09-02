/**
 * Linux desktop integration: `.desktop` entries, AppDir/AppImage layout, and `.deb` packages.
 *
 * The `.deb` writer is pure TypeScript — an `ar` container holding two ustar+gzip tarballs — so it
 * runs on any host without `dpkg-deb`. AppImage creation still needs `appimagetool`; when that is
 * missing QuickGUI leaves the finished AppDir and prints the one command that completes it.
 */

import { CliError } from "../error.ts";
import { createAr, createTar, type TarEntry } from "./archive.ts";
import { linuxMimeTypes, type ResolvedDocumentType } from "./documents.ts";
import { LINUX_ICON_SIZES } from "./icons.ts";

/** Debian architecture names for the Linux targets QuickGUI builds. */
export const DEBIAN_ARCHITECTURES: Readonly<Record<"arm64" | "x64", string>> = Object.freeze({
  arm64: "arm64",
  x64: "amd64",
});

export interface DesktopEntryOptions {
  name: string;
  executableName: string;
  identifier: string;
  comment?: string;
  categories: readonly string[];
  protocols: readonly string[];
  documentTypes: readonly ResolvedDocumentType[];
  /** `Exec=` command. Defaults to the executable name plus `%U` when the app handles URLs/files. */
  execPath?: string;
}

/** Freedesktop desktop entry text. Values are single-line and never contain a newline. */
export function desktopEntry(options: DesktopEntryOptions): string {
  const mimeTypes = [
    ...options.protocols.map((protocol) => `x-scheme-handler/${protocol}`),
    ...linuxMimeTypes(options.documentTypes),
  ];
  const exec = options.execPath ?? options.executableName;
  const acceptsArguments = mimeTypes.length > 0;
  const lines = [
    "[Desktop Entry]",
    "Type=Application",
    `Name=${desktopValue(options.name)}`,
    ...(options.comment ? [`Comment=${desktopValue(options.comment)}`] : []),
    `Exec=${desktopValue(exec)}${acceptsArguments ? " %U" : ""}`,
    `Icon=${desktopValue(options.executableName)}`,
    "Terminal=false",
    `Categories=${options.categories.map(desktopValue).join(";")};`,
    ...(mimeTypes.length > 0 ? [`MimeType=${mimeTypes.map(desktopValue).join(";")};`] : []),
    `StartupWMClass=${desktopValue(options.executableName)}`,
    `X-QuickGUI-Identifier=${desktopValue(options.identifier)}`,
  ];
  return `${lines.join("\n")}\n`;
}

function desktopValue(value: string): string {
  const single = value.replaceAll(/[\r\n]+/g, " ").trim();
  if (single.length === 0) throw new CliError("A desktop entry value cannot be empty");
  return single;
}

/** The `AppRun` script an AppDir needs when the payload is a plain executable. */
export function appRunScript(executableName: string): string {
  return `#!/bin/sh
HERE="$(dirname "$(readlink -f "$0")")"
exec "$HERE/usr/bin/${executableName}" "$@"
`;
}

/** `appimagetool` command line. */
export function appImageArguments(appDir: string, output: string): string[] {
  return ["appimagetool", "--no-appstream", appDir, output];
}

export interface DebianControlOptions {
  packageName: string;
  version: string;
  architecture: string;
  maintainer: string;
  description: string;
  section: string;
  depends: readonly string[];
  installedSizeKilobytes: number;
  homepage?: string;
}

/** Debian `control` file. The description body is indented per Debian policy. */
export function debianControl(options: DebianControlOptions): string {
  if (!/^[a-z0-9][a-z0-9+.-]+$/.test(options.packageName)) {
    throw new CliError(
      `Invalid Debian package name \`${options.packageName}\`: use lowercase letters, digits, +, -, and .`,
    );
  }
  if (!options.maintainer.includes("<") || !options.maintainer.includes(">")) {
    throw new CliError(
      "`linux.maintainer` must look like `Name <email@example.com>` to build a .deb",
    );
  }
  const [summary, ...rest] = options.description.split("\n");
  const body = rest
    .map((line) => (line.trim().length === 0 ? " ." : ` ${line.trim()}`))
    .join("\n");
  const lines = [
    `Package: ${options.packageName}`,
    `Version: ${options.version}`,
    `Architecture: ${options.architecture}`,
    `Maintainer: ${options.maintainer}`,
    `Installed-Size: ${Math.max(1, Math.round(options.installedSizeKilobytes))}`,
    `Section: ${options.section}`,
    "Priority: optional",
    ...(options.depends.length > 0 ? [`Depends: ${options.depends.join(", ")}`] : []),
    ...(options.homepage ? [`Homepage: ${options.homepage}`] : []),
    `Description: ${(summary ?? options.packageName).trim()}`,
    ...(body.length > 0 ? [body] : []),
  ];
  return `${lines.join("\n")}\n`;
}

/** `md5sums` control file body for the package payload. */
export function debianMd5Sums(files: ReadonlyArray<{ path: string; md5: string }>): string {
  return files.map((file) => `${file.md5}  ${file.path.replace(/^\/+/, "")}\n`).join("");
}

export interface DebianPackageInput {
  control: string;
  md5sums: string;
  /** Payload entries with paths relative to the filesystem root, e.g. `usr/bin/app`. */
  data: readonly TarEntry[];
  postinst?: string;
  gzip: (data: Uint8Array) => Uint8Array;
}

/** Assemble a `.deb` from already-built control text and payload entries. */
export function createDebianPackage(input: DebianPackageInput): Uint8Array {
  const encoder = new TextEncoder();
  const controlEntries: TarEntry[] = [
    { path: "./control", data: encoder.encode(input.control), mode: 0o644 },
    { path: "./md5sums", data: encoder.encode(input.md5sums), mode: 0o644 },
    ...(input.postinst
      ? [{ path: "./postinst", data: encoder.encode(input.postinst), mode: 0o755 }]
      : []),
  ].map((entry) => ({ ...entry, path: entry.path.replace(/^\.\//, "") }));
  return createAr([
    { name: "debian-binary", data: encoder.encode("2.0\n") },
    { name: "control.tar.gz", data: input.gzip(createTar(controlEntries)) },
    { name: "data.tar.gz", data: input.gzip(createTar(input.data)) },
  ]);
}

/** Standard payload paths for a QuickGUI Linux install. */
export function debianPayloadPaths(executableName: string): {
  executable: string;
  desktopEntry: string;
  mimePackage: string;
  icon: (size: number) => string;
} {
  return {
    executable: `usr/bin/${executableName}`,
    desktopEntry: `usr/share/applications/${executableName}.desktop`,
    mimePackage: `usr/share/mime/packages/${executableName}.xml`,
    icon: (size: number) => `usr/share/icons/hicolor/${size}x${size}/apps/${executableName}.png`,
  };
}

export { LINUX_ICON_SIZES };
