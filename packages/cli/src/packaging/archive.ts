/**
 * Pure-TypeScript archive writers used by QuickGUI packaging.
 *
 * `ustar` and `ar` are both fixed-record formats with no compression of their own, so QuickGUI
 * builds them directly instead of depending on external tools. Every member is bounded: paths are
 * limited to the 100-byte ustar name field (with a 155-byte prefix), and each writer refuses a
 * member larger than {@link MAX_ARCHIVE_MEMBER_BYTES}.
 */

import { CliError } from "../error.ts";

/** Largest single file QuickGUI writes into a generated archive. */
export const MAX_ARCHIVE_MEMBER_BYTES = 512 * 1024 * 1024;
/** Largest number of members in one generated archive. */
export const MAX_ARCHIVE_MEMBERS = 65_536;

export interface TarEntry {
  /** Archive-relative path using forward slashes and no `.`/`..` segments. */
  path: string;
  /** File contents. Directories pass an empty buffer. */
  data?: Uint8Array;
  /** Unix permission bits. Defaults to 0o644 for files and 0o755 for directories. */
  mode?: number;
  /** Modification time in whole seconds since the Unix epoch. Defaults to 0 for reproducibility. */
  mtime?: number;
  type?: "file" | "directory";
}

const BLOCK = 512;

/** Build a deterministic ustar archive. Entries keep the given order. */
export function createTar(entries: readonly TarEntry[]): Uint8Array {
  if (entries.length > MAX_ARCHIVE_MEMBERS) {
    throw new CliError(`A generated archive may contain at most ${MAX_ARCHIVE_MEMBERS} entries`);
  }
  const blocks: Uint8Array[] = [];
  for (const entry of entries) {
    const directory = entry.type === "directory";
    const data = directory ? new Uint8Array(0) : (entry.data ?? new Uint8Array(0));
    if (data.byteLength > MAX_ARCHIVE_MEMBER_BYTES) {
      throw new CliError(`Archive member ${entry.path} exceeds ${MAX_ARCHIVE_MEMBER_BYTES} bytes`);
    }
    const path = normalizeArchivePath(entry.path) + (directory ? "/" : "");
    blocks.push(
      tarHeader({
        path,
        size: data.byteLength,
        mode: entry.mode ?? (directory ? 0o755 : 0o644),
        mtime: entry.mtime ?? 0,
        typeflag: directory ? "5" : "0",
      }),
    );
    if (data.byteLength > 0) {
      blocks.push(data);
      const padding = (BLOCK - (data.byteLength % BLOCK)) % BLOCK;
      if (padding > 0) blocks.push(new Uint8Array(padding));
    }
  }
  // ustar terminates with two zero blocks.
  blocks.push(new Uint8Array(BLOCK * 2));
  return concat(blocks);
}

/** Split an archive path into the ustar `prefix`/`name` fields, or throw when it cannot fit. */
export function splitUstarPath(path: string): { name: string; prefix: string } {
  const bytes = new TextEncoder().encode(path);
  if (bytes.byteLength <= 100) return { name: path, prefix: "" };
  const separator = path.lastIndexOf("/", path.length - 1);
  const prefix = separator === -1 ? "" : path.slice(0, separator);
  const name = separator === -1 ? path : path.slice(separator + 1);
  const prefixBytes = new TextEncoder().encode(prefix).byteLength;
  const nameBytes = new TextEncoder().encode(name).byteLength;
  if (prefixBytes > 155 || nameBytes > 100) {
    throw new CliError(`Archive path is too long for the ustar format: ${path}`);
  }
  return { name, prefix };
}

export function normalizeArchivePath(path: string): string {
  const normalized = path.replaceAll("\\", "/").replace(/^\.\//, "").replace(/\/+$/, "");
  if (
    normalized.length === 0 ||
    normalized.startsWith("/") ||
    normalized.includes("\0") ||
    normalized.split("/").some((segment) => segment === "." || segment === "..")
  ) {
    throw new CliError(`Invalid archive path: ${path}`);
  }
  return normalized;
}

function tarHeader(options: {
  path: string;
  size: number;
  mode: number;
  mtime: number;
  typeflag: string;
}): Uint8Array {
  const header = new Uint8Array(BLOCK);
  const encoder = new TextEncoder();
  const write = (offset: number, value: string, length: number): void => {
    const bytes = encoder.encode(value);
    if (bytes.byteLength > length) {
      throw new CliError(`Archive header field does not fit: ${value}`);
    }
    header.set(bytes, offset);
  };
  const { name, prefix } = splitUstarPath(options.path);
  write(0, name, 100);
  write(100, octal(options.mode & 0o7777, 7), 8);
  write(108, octal(0, 7), 8);
  write(116, octal(0, 7), 8);
  write(124, octal(options.size, 11), 12);
  write(136, octal(options.mtime, 11), 12);
  header.fill(0x20, 148, 156); // Checksum field is spaces while the checksum is computed.
  write(156, options.typeflag, 1);
  write(257, "ustar", 6);
  write(263, "00", 2);
  write(265, "root", 32);
  write(297, "root", 32);
  write(345, prefix, 155);
  let checksum = 0;
  for (const byte of header) checksum += byte;
  write(148, `${checksum.toString(8).padStart(6, "0")}\0`, 8);
  return header;
}

function octal(value: number, width: number): string {
  if (!Number.isInteger(value) || value < 0) {
    throw new CliError(`Archive header value must be a non-negative integer: ${value}`);
  }
  const text = value.toString(8);
  if (text.length > width) throw new CliError(`Archive header value does not fit: ${value}`);
  return `${text.padStart(width, "0")}\0`;
}

export interface ArMember {
  name: string;
  data: Uint8Array;
  mode?: number;
  mtime?: number;
}

/** Build a `!<arch>` archive using the common (BSD-compatible) short-name form Debian requires. */
export function createAr(members: readonly ArMember[]): Uint8Array {
  const encoder = new TextEncoder();
  const parts: Uint8Array[] = [encoder.encode("!<arch>\n")];
  for (const member of members) {
    if (member.name.length > 15 || !/^[A-Za-z0-9._-]+$/.test(member.name)) {
      throw new CliError(`Invalid ar member name: ${member.name}`);
    }
    if (member.data.byteLength > MAX_ARCHIVE_MEMBER_BYTES) {
      throw new CliError(`Archive member ${member.name} exceeds ${MAX_ARCHIVE_MEMBER_BYTES} bytes`);
    }
    const header =
      member.name.padEnd(16, " ") +
      String(member.mtime ?? 0).padEnd(12, " ") +
      "0".padEnd(6, " ") +
      "0".padEnd(6, " ") +
      (member.mode ?? 0o100644).toString(8).padEnd(8, " ") +
      String(member.data.byteLength).padEnd(10, " ") +
      "`\n";
    parts.push(encoder.encode(header));
    parts.push(member.data);
    if (member.data.byteLength % 2 === 1) parts.push(encoder.encode("\n"));
  }
  return concat(parts);
}

export function concat(parts: readonly Uint8Array[]): Uint8Array {
  let length = 0;
  for (const part of parts) length += part.byteLength;
  const output = new Uint8Array(length);
  let offset = 0;
  for (const part of parts) {
    output.set(part, offset);
    offset += part.byteLength;
  }
  return output;
}
