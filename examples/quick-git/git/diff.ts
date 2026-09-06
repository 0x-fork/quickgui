/**
 * Unified diff parsing, retained diffs, and patch construction.
 *
 * A diff is read in one pass that keeps the text as lines and records where each file and hunk
 * begins, what kind every body line is, and nothing else. `Diff.open` walks that scan in chunks,
 * yielding to the event loop between them, so a diff at the app's 32 MB cap never freezes the
 * window and can be abandoned when the selection moves on. Lines become objects only where they
 * are needed: the rows the table is about to paint, the changed lines inside a selection, and the
 * one file a patch is built from. `parseDiff` is the eager form, for tests and small inputs.
 *
 * `formatPatch` does the inverse for a chosen subset of a hunk's lines, producing exactly the
 * patch `git apply --cached` (stage), `git apply --cached --reverse` (unstage), or
 * `git apply --reverse` (discard) needs, with recomputed hunk counts. Nothing here runs git.
 */

import { Buffer } from "node:buffer";

// --- diffs ---------------------------------------------------------------------------------------

export type DiffLineKind = "context" | "added" | "removed";

export interface DiffLine {
  kind: DiffLineKind;
  /** Line content without the leading marker or trailing newline. */
  text: string;
  /** Line number in the old file, `null` for added lines. */
  oldLineNumber: number | null;
  /** Line number in the new file, `null` for removed lines. */
  newLineNumber: number | null;
  /** Whether git printed `\ No newline at end of file` after this line. */
  noNewline: boolean;
}

export interface DiffHunk {
  oldStart: number;
  oldLines: number;
  newStart: number;
  newLines: number;
  /** Text after the closing `@@`, typically the enclosing function. */
  heading: string;
  lines: DiffLine[];
}

export type DiffFileKind = "modified" | "added" | "deleted" | "renamed" | "copied";

export interface DiffFile {
  kind: DiffFileKind;
  /** Path on the old side, `null` for an added file. */
  oldPath: string | null;
  /** Path on the new side, `null` for a deleted file. */
  newPath: string | null;
  /** The path the app shows: the new path, or the old one for a deletion. */
  path: string;
  binary: boolean;
  oldMode?: string;
  newMode?: string;
  similarity?: number;
  hunks: DiffHunk[];
  /** Whether the diff was cut short by the output bound. */
  truncated: boolean;
}

/** The two sides of a `diff --git` header line. */
export interface HeaderPaths {
  oldPath: string;
  newPath: string;
}

export interface ParseDiffOptions {
  /** Whether the output hit the byte bound; marks the last file as truncated. */
  truncated?: boolean;
}

/** Parse the complete output of `git diff` (text or the raw bytes git wrote) into files. */
export function parseDiff(output: string | Uint8Array, options: ParseDiffOptions = {}): DiffFile[] {
  const scan = new DiffScan(options.truncated ?? false);
  scan.push(decodeDiff(output));
  const files = scan.finish();
  return files.map((entry) => materializeFile(scan.lines, entry));
}

// --- scanning ------------------------------------------------------------------------------------

/** One hunk as the scan records it: its header, and where its body lines live in `DiffScan.lines`. */
interface HunkEntry {
  oldStart: number;
  oldLines: number;
  newStart: number;
  newLines: number;
  heading: string;
  /** Index of the first body line. */
  start: number;
  /** Body lines, not counting `\ No newline at end of file` markers. */
  count: number;
}

/** One file as the scan records it: its headers parsed, its lines only counted. */
interface FileEntry {
  kind: DiffFileKind;
  oldPath: string | null;
  newPath: string | null;
  path: string;
  binary: boolean;
  oldMode?: string;
  newMode?: string;
  similarity?: number;
  hunks: HunkEntry[];
  lineCount: number;
  added: number;
  removed: number;
  truncated: boolean;
}

/**
 * Where the scan is: outside a file, in its extended headers, in a hunk body, or between hunks
 * after a line that belongs to none of them (a truncated tail, say).
 */
type ScanState = "seeking" | "headers" | "body" | "hunks";

const PLUS = 43;
const MINUS = 45;
const SPACE = 32;
const NEWLINE = 10;
const BACKSLASH = 92;
const AT = 64;
const LETTER_D = 100;

/**
 * Reads diff text one chunk at a time and records its shape.
 *
 * Every line is kept — the rows are cut from them later — but no line object is built here, so
 * the cost of a scan is one pass of string comparisons per line rather than an object, a
 * substring, and two line numbers.
 */
class DiffScan {
  readonly lines: string[] = [];
  readonly files: FileEntry[] = [];
  #state: ScanState = "seeking";
  #file: FileEntry | undefined = undefined;
  #hunk: HunkEntry | undefined = undefined;
  /** Text after the last newline of the previous chunk. */
  #carry = "";
  #truncated: boolean;

  constructor(truncated: boolean) {
    this.#truncated = truncated;
  }

  /** Feed the next piece of diff text. Chunks may split a line. */
  push(chunk: string): void {
    const parts = (this.#carry + chunk).split("\n");
    this.#carry = parts.pop() ?? "";
    for (const line of parts) this.#line(line);
  }

  /** Close the scan and return the files. A trailing newline does not make an empty last line. */
  finish(): FileEntry[] {
    if (this.#carry.length > 0) {
      this.#line(this.#carry);
      this.#carry = "";
    }
    this.#closeFile();
    if (this.#truncated && this.files.length > 0) this.files[this.files.length - 1]!.truncated = true;
    return this.files;
  }

  #line(line: string): void {
    const index = this.lines.length;
    this.lines.push(line);
    // Almost every line of a diff is a body line, and its first character says which kind it is.
    // Only the two characters that can begin a header are worth a string comparison.
    const marker = line.length > 0 ? line.charCodeAt(0) : SPACE;
    if (marker === LETTER_D && line.startsWith("diff --git ")) {
      this.#closeFile();
      this.#file = emptyEntry(parseGitHeaderPaths(line.slice("diff --git ".length)));
      this.#state = "headers";
      return;
    }
    const file = this.#file;
    if (file === undefined || this.#state === "seeking") return;
    if (marker === AT && line.startsWith("@@")) {
      const hunk = parseHunkHeader(line, index + 1);
      if (hunk === undefined) {
        // A header the parser cannot read ends the file, as a truncated tail would.
        this.#state = "seeking";
        return;
      }
      file.hunks.push(hunk);
      this.#hunk = hunk;
      this.#state = "body";
      return;
    }
    if (this.#state === "headers") {
      this.#header(file, line);
      return;
    }
    if (this.#state === "hunks") {
      this.#state = "seeking";
      return;
    }
    // `\ No newline at end of file` describes the line before it; it is not a line of its own.
    if (marker === BACKSLASH) return;
    const hunk = this.#hunk;
    if (hunk === undefined) return;
    if (marker === PLUS) {
      file.added += 1;
    } else if (marker === MINUS) {
      file.removed += 1;
    } else if (marker !== SPACE) {
      // Anything else ends the hunk (for example a truncated tail).
      this.#state = "hunks";
      return;
    }
    hunk.count += 1;
    file.lineCount += 1;
  }

  /** An extended header line, between `diff --git` and the first hunk. */
  #header(file: FileEntry, line: string): void {
    if (line.startsWith("old mode ")) file.oldMode = line.slice(9).trim();
    else if (line.startsWith("new mode ")) file.newMode = line.slice(9).trim();
    else if (line.startsWith("deleted file mode ")) {
      file.kind = "deleted";
      file.oldMode = line.slice(18).trim();
    } else if (line.startsWith("new file mode ")) {
      file.kind = "added";
      file.newMode = line.slice(14).trim();
    } else if (line.startsWith("similarity index ")) {
      const similarity = leadingInteger(line.slice(17));
      if (similarity !== undefined) file.similarity = similarity;
    } else if (line.startsWith("rename from ")) {
      file.kind = "renamed";
      file.oldPath = line.slice(12);
    } else if (line.startsWith("rename to ")) {
      file.kind = "renamed";
      file.newPath = line.slice(10);
    } else if (line.startsWith("copy from ")) {
      file.kind = "copied";
      file.oldPath = line.slice(10);
    } else if (line.startsWith("copy to ")) {
      file.kind = "copied";
      file.newPath = line.slice(8);
    } else if (line.startsWith("Binary files ")) {
      file.binary = true;
    } else if (line.startsWith("--- ")) {
      file.oldPath = stripPrefix(line.slice(4), "a/");
    } else if (line.startsWith("+++ ")) {
      file.newPath = stripPrefix(line.slice(4), "b/");
    }
  }

  #closeFile(): void {
    const file = this.#file;
    this.#file = undefined;
    this.#hunk = undefined;
    if (file === undefined) return;
    if (file.kind === "added") file.oldPath = null;
    if (file.kind === "deleted") file.newPath = null;
    file.path = file.newPath ?? file.oldPath ?? "";
    this.files.push(file);
  }
}

function emptyEntry(paths: HeaderPaths): FileEntry {
  return {
    kind: "modified",
    oldPath: paths.oldPath,
    newPath: paths.newPath,
    path: paths.newPath,
    binary: false,
    hunks: [],
    lineCount: 0,
    added: 0,
    removed: 0,
    truncated: false,
  };
}

/**
 * Split the `a/<old> b/<new>` pair of a `diff --git` line.
 *
 * Paths may contain spaces, so the split point is the ` b/` whose two halves agree when the file
 * kept its name; otherwise the first ` b/` after `a/` is used and later header lines correct it.
 */
export function parseGitHeaderPaths(pair: string): HeaderPaths {
  const candidates: number[] = [];
  let search = 0;
  for (;;) {
    const at = pair.indexOf(" b/", search);
    if (at < 0) break;
    candidates.push(at);
    search = at + 1;
  }
  for (const at of candidates) {
    const oldPath = pair.slice(0, at);
    const newPath = pair.slice(at + 1);
    if (oldPath.startsWith("a/") && newPath.startsWith("b/") && oldPath.slice(2) === newPath.slice(2)) {
      return { oldPath: oldPath.slice(2), newPath: newPath.slice(2) };
    }
  }
  const first = candidates.length > 0 ? candidates[0]! : undefined;
  if (first !== undefined && pair.startsWith("a/")) {
    return { oldPath: pair.slice(2, first), newPath: pair.slice(first + 3) };
  }
  // Quoted or unusual header: fall back to the whole text on both sides.
  const unquoted = pair.replace(/^"|"$/g, "");
  return { oldPath: unquoted, newPath: unquoted };
}

function parseHunkHeader(line: string, start: number): HunkEntry | undefined {
  const match = /^@@ -(\d+)(?:,(\d+))? \+(\d+)(?:,(\d+))? @@ ?(.*)$/.exec(line);
  if (!match) return undefined;
  return {
    oldStart: Number(match[1]),
    oldLines: match[2] === undefined ? 1 : Number(match[2]),
    newStart: Number(match[3]),
    newLines: match[4] === undefined ? 1 : Number(match[4]),
    heading: match[5] ?? "",
    start,
    count: 0,
  };
}

function stripPrefix(path: string, prefix: string): string | null {
  if (path === "/dev/null") return null;
  // A tab may follow the path when it carries a timestamp; git never adds one, but be safe.
  const tab = path.indexOf("\t");
  const clean = tab >= 0 ? path.slice(0, tab) : path;
  return clean.startsWith(prefix) ? clean.slice(prefix.length) : clean;
}

/** `Number.parseInt` on the leading digits: skip whitespace, read digits, ignore the rest. */
function leadingInteger(text: string): number | undefined {
  const match = /^\s*(\d+)/.exec(text);
  return match ? Number(match[1]) : undefined;
}

/** Git output reaches the app as the raw bytes it wrote; invalid sequences become U+FFFD. */
function decodeDiff(output: string | Uint8Array): string {
  return typeof output === "string" ? output : new TextDecoder().decode(output);
}

/** Whether the line is a `\ No newline at end of file` marker rather than a line of the file. */
function startsBackslash(line: string): boolean {
  return line.length > 0 && line.charCodeAt(0) === BACKSLASH;
}

// --- reading lines back --------------------------------------------------------------------------

/**
 * A position inside one hunk's body, which walks forward one line at a time.
 *
 * The line numbers of a line follow from the lines before it, so reading line `n` means walking
 * to it. The table scrolls, so one cursor per diff serves a window in a step per row; a jump
 * backwards or into another hunk starts a new walk from the hunk's first line.
 */
class LineCursor {
  /** Index of the line the cursor sits on, `-1` before the first. */
  logical = -1;
  #lines: string[];
  #next: number;
  #oldNext: number;
  #newNext: number;
  #raw = "";
  #kind: DiffLineKind = "context";
  #oldLineNumber: number | null = null;
  #newLineNumber: number | null = null;

  constructor(lines: string[], hunk: HunkEntry) {
    this.#lines = lines;
    this.#next = hunk.start;
    this.#oldNext = hunk.oldStart;
    this.#newNext = hunk.newStart;
  }

  /** Move to the line at `logical`, which must not be behind the cursor. */
  advanceTo(logical: number): void {
    while (this.logical < logical) this.#advance();
  }

  /** The line with its marker, as git wrote it. */
  get raw(): string {
    return this.#raw;
  }

  get kind(): DiffLineKind {
    return this.#kind;
  }

  get text(): string {
    return this.#raw.slice(1);
  }

  get oldLineNumber(): number | null {
    return this.#oldLineNumber;
  }

  get newLineNumber(): number | null {
    return this.#newLineNumber;
  }

  /** Whether git printed `\ No newline at end of file` after this line. */
  get noNewline(): boolean {
    return this.#next < this.#lines.length && startsBackslash(this.#lines[this.#next]!);
  }

  get line(): DiffLine {
    return {
      kind: this.#kind,
      text: this.text,
      oldLineNumber: this.#oldLineNumber,
      newLineNumber: this.#newLineNumber,
      noNewline: this.noNewline,
    };
  }

  #advance(): void {
    const lines = this.#lines;
    let index = this.#next;
    while (index < lines.length && startsBackslash(lines[index]!)) index += 1;
    const raw = index < lines.length ? lines[index]! : "";
    this.#raw = raw;
    this.#next = index + 1;
    this.logical += 1;
    const marker = raw.length > 0 ? raw.charCodeAt(0) : SPACE;
    if (marker === PLUS) {
      this.#kind = "added";
      this.#oldLineNumber = null;
      this.#newLineNumber = this.#newNext;
      this.#newNext += 1;
    } else if (marker === MINUS) {
      this.#kind = "removed";
      this.#oldLineNumber = this.#oldNext;
      this.#oldNext += 1;
      this.#newLineNumber = null;
    } else {
      this.#kind = "context";
      this.#oldLineNumber = this.#oldNext;
      this.#newLineNumber = this.#newNext;
      this.#oldNext += 1;
      this.#newNext += 1;
    }
  }
}

function materializeHunk(lines: string[], entry: HunkEntry): DiffHunk {
  const cursor = new LineCursor(lines, entry);
  const hunkLines: DiffLine[] = [];
  for (let index = 0; index < entry.count; index += 1) {
    cursor.advanceTo(index);
    hunkLines.push(cursor.line);
  }
  return {
    oldStart: entry.oldStart,
    oldLines: entry.oldLines,
    newStart: entry.newStart,
    newLines: entry.newLines,
    heading: entry.heading,
    lines: hunkLines,
  };
}

function materializeFile(lines: string[], entry: FileEntry): DiffFile {
  const file: DiffFile = {
    kind: entry.kind,
    oldPath: entry.oldPath,
    newPath: entry.newPath,
    path: entry.path,
    binary: entry.binary,
    hunks: entry.hunks.map((hunk) => materializeHunk(lines, hunk)),
    truncated: entry.truncated,
  };
  if (entry.oldMode !== undefined) file.oldMode = entry.oldMode;
  if (entry.newMode !== undefined) file.newMode = entry.newMode;
  if (entry.similarity !== undefined) file.similarity = entry.similarity;
  return file;
}

// --- retained diffs ------------------------------------------------------------------------------

/** What the app knows about a file without walking its hunks. */
export interface DiffFileSummary {
  kind: DiffFileKind;
  oldPath: string | null;
  newPath: string | null;
  path: string;
  binary: boolean;
  oldMode?: string;
  newMode?: string;
  similarity?: number;
  hunkCount: number;
  lineCount: number;
  added: number;
  removed: number;
  truncated: boolean;
}

export type DiffRowKind = "file" | "hunk" | "line" | "notice";

/** One row of the diff table, ready to paint. */
export interface DiffRow {
  kind: DiffRowKind;
  fileIndex: number;
  hunkIndex: number;
  lineIndex: number;
  /** `context` for rows that are not lines. */
  lineKind: DiffLineKind;
  /** The line text, the hunk header, the file path, or the notice. */
  text: string;
  oldLineNumber: number | null;
  newLineNumber: number | null;
  noNewline: boolean;
}

/** Changed lines of one hunk inside a selection, as `formatPatch` wants them. */
export interface SelectedLines {
  fileIndex: number;
  hunkIndex: number;
  lines: Set<number>;
}

export interface OpenDiffOptions {
  /** Whether git's output hit the byte bound; marks the last file as truncated. */
  truncated?: boolean;
  /** Row cap for the table. Defaults to {@link MAX_DIFF_ROWS}. */
  maxRows?: number;
  /** Abandons the scan between chunks; `Diff.open` then throws. */
  signal?: AbortSignal | undefined;
}

/** Most rows the diff table shows; the rest is replaced by a notice. */
export const MAX_DIFF_ROWS = 60_000;

/** Most characters of one line handed to the table; the view clips long lines itself. */
const MAX_ROW_TEXT = 4096;

/**
 * Diff text read between two yields to the event loop.
 *
 * A megabyte is a few tens of milliseconds of scanning, so the window keeps painting and answering
 * clicks while a diff at the app's 32 MB cap is read, and a diff nobody waits for any more is
 * dropped a chunk later. Yielding costs a few milliseconds of its own, so smaller chunks buy
 * smoothness the table cannot use.
 */
const SCAN_CHUNK_CHARS = 1_000_000;
const SCAN_CHUNK_BYTES = 1_000_000;

/** Notice rows carry their text by index here, so a row reference stays four numbers. */
const NOTICES = ["Binary file", "Diff truncated: the file is too large to show in full.", "Diff truncated: too many lines to show."];
const NOTICE_BINARY = 0;
const NOTICE_TRUNCATED_FILE = 1;
const NOTICE_TOO_MANY_ROWS = 2;

/** Where a table row comes from: the notice index for notices, the line for lines. */
interface RowRef {
  kind: DiffRowKind;
  fileIndex: number;
  hunkIndex: number;
  lineIndex: number;
}

let nextDiffId = 1;
let openDiffs = 0;

/**
 * A diff scanned once and held until `close()`.
 *
 * What the diff keeps is its lines and, per file and hunk, where they start and how many there
 * are. Rows are cut from that as the table paints them, so scrolling a 32 MB diff never builds a
 * line object twice and closing the handle drops the whole scan at once. Handles are released
 * explicitly: the store owns them and closes them on cache eviction, repository changes, and
 * window disposal; temporary scans use try/finally.
 */
export class Diff {
  readonly id: number;
  readonly files: readonly DiffFileSummary[];
  readonly rowCount: number;
  /** Added and removed lines across every file, whether or not they fit in the table. */
  readonly added: number;
  readonly removed: number;
  /** Size of the text the diff was read from, for cache accounting. */
  readonly bytes: number;
  #lines: string[];
  #entries: FileEntry[];
  #rows: RowRef[];
  /** The cursor of the last line read, and the hunk it walks. */
  #cursor: LineCursor | undefined = undefined;
  #cursorFile = -1;
  #cursorHunk = -1;
  #closed = false;

  private constructor(scan: DiffScan, entries: FileEntry[], bytes: number, maxRows: number) {
    const rows: RowRef[] = [];
    const summaries: DiffFileSummary[] = [];
    let added = 0;
    let removed = 0;
    for (let fileIndex = 0; fileIndex < entries.length; fileIndex += 1) {
      const file = entries[fileIndex]!;
      if (entries.length > 1) rows.push({ kind: "file", fileIndex, hunkIndex: 0, lineIndex: 0 });
      if (file.binary) {
        rows.push({ kind: "notice", fileIndex, hunkIndex: 0, lineIndex: NOTICE_BINARY });
      } else {
        for (let hunkIndex = 0; hunkIndex < file.hunks.length; hunkIndex += 1) {
          rows.push({ kind: "hunk", fileIndex, hunkIndex, lineIndex: 0 });
          const count = file.hunks[hunkIndex]!.count;
          // Lines past the cap are still counted in the summaries; only their rows are dropped.
          for (let lineIndex = 0; lineIndex < count && rows.length < maxRows; lineIndex += 1) {
            rows.push({ kind: "line", fileIndex, hunkIndex, lineIndex });
          }
        }
        if (file.truncated) {
          rows.push({ kind: "notice", fileIndex, hunkIndex: 0, lineIndex: NOTICE_TRUNCATED_FILE });
        }
      }
      added += file.added;
      removed += file.removed;
      const summary: DiffFileSummary = {
        kind: file.kind,
        oldPath: file.oldPath,
        newPath: file.newPath,
        path: file.path,
        binary: file.binary,
        hunkCount: file.hunks.length,
        lineCount: file.lineCount,
        added: file.added,
        removed: file.removed,
        truncated: file.truncated,
      };
      if (file.oldMode !== undefined) summary.oldMode = file.oldMode;
      if (file.newMode !== undefined) summary.newMode = file.newMode;
      if (file.similarity !== undefined) summary.similarity = file.similarity;
      summaries.push(summary);
    }
    if (rows.length >= maxRows) {
      rows.push({ kind: "notice", fileIndex: 0, hunkIndex: 0, lineIndex: NOTICE_TOO_MANY_ROWS });
    }

    this.id = nextDiffId;
    nextDiffId += 1;
    openDiffs += 1;
    this.files = summaries;
    this.rowCount = rows.length;
    this.added = added;
    this.removed = removed;
    this.bytes = bytes;
    this.#lines = scan.lines;
    this.#entries = entries;
    this.#rows = rows;
  }

  /**
   * Read `output` in chunks, yielding between them, and hold the result until `close()`.
   *
   * The event loop keeps turning while a big diff is read, so the window paints and answers
   * clicks throughout. An aborted signal ends the scan at the next chunk with an error.
   */
  static async open(output: string | Uint8Array, options: OpenDiffOptions = {}): Promise<Diff> {
    const scan = new DiffScan(options.truncated ?? false);
    if (typeof output === "string") {
      for (let offset = 0; offset < output.length; offset += SCAN_CHUNK_CHARS) {
        if (offset > 0) await pause(options);
        scan.push(output.slice(offset, Math.min(offset + SCAN_CHUNK_CHARS, output.length)));
      }
    } else {
      // Decoding is chunked too, so the first megabytes do not arrive as one blocking call.
      // A chunk ends on a newline byte, which UTF-8 never puts inside a character.
      const decoder = new TextDecoder();
      let offset = 0;
      while (offset < output.byteLength) {
        if (offset > 0) await pause(options);
        const end = lineEnd(output, offset, SCAN_CHUNK_BYTES);
        scan.push(decoder.decode(output.subarray(offset, end)));
        offset = end;
      }
    }
    return new Diff(scan, scan.finish(), byteLength(output), options.maxRows ?? MAX_DIFF_ROWS);
  }

  /** Read `output` in one go, for tests and diffs already known to be small. */
  static openSync(output: string | Uint8Array, options: OpenDiffOptions = {}): Diff {
    const scan = new DiffScan(options.truncated ?? false);
    scan.push(decodeDiff(output));
    return new Diff(scan, scan.finish(), byteLength(output), options.maxRows ?? MAX_DIFF_ROWS);
  }

  /** How many diffs are currently held, for tests. */
  static openCount(): number {
    return openDiffs;
  }

  get closed(): boolean {
    return this.#closed;
  }

  /** The table rows in `[start, end)`, clipped to the diff; empty once it is closed. */
  rows(start: number, end: number): DiffRow[] {
    if (this.#closed) return [];
    const first = Math.max(0, start);
    const last = Math.min(Math.max(0, end), this.#rows.length);
    const rows: DiffRow[] = [];
    for (let index = first; index < last; index += 1) rows.push(this.#row(this.#rows[index]!));
    return rows;
  }

  /** One file with every hunk and line, for `formatPatch`. */
  file(index: number): DiffFile | undefined {
    if (this.#closed || index < 0 || index >= this.#entries.length) return undefined;
    return materializeFile(this.#lines, this.#entries[index]!);
  }

  /**
   * The changed lines inside the table's selected row ranges (inclusive), grouped by hunk in the
   * order they were first seen.
   */
  selectedLines(ranges: readonly (readonly number[])[]): SelectedLines[] {
    if (this.#closed || ranges.length === 0) return [];
    const groups = new Map<string, SelectedLines>();
    const found: SelectedLines[] = [];
    for (const range of ranges) {
      if (range.length < 2) continue;
      const last = Math.min(range[1]!, this.#rows.length - 1);
      for (let index = Math.max(0, range[0]!); index <= last; index += 1) {
        const ref = this.#rows[index]!;
        if (ref.kind !== "line") continue;
        const cursor = this.#seek(ref);
        if (cursor.kind === "context") continue;
        const key = `${ref.fileIndex}:${ref.hunkIndex}`;
        let group = groups.get(key);
        if (!group) {
          group = { fileIndex: ref.fileIndex, hunkIndex: ref.hunkIndex, lines: new Set<number>() };
          groups.set(key, group);
          found.push(group);
        }
        group.lines.add(ref.lineIndex);
      }
    }
    return found;
  }

  /** The diff as text for a prompt: one header per file, hunk headers, and marked lines. */
  render(): string {
    if (this.#closed) return "";
    const out: string[] = [];
    for (const file of this.#entries) {
      out.push(`diff --git a/${file.oldPath ?? file.path} b/${file.newPath ?? file.path}`);
      if (file.kind === "added") {
        out.push(`new file mode ${file.newMode ?? "100644"}`, "--- /dev/null", `+++ b/${file.path}`);
      }
      for (const hunk of file.hunks) {
        out.push(hunkHeader(hunk));
        const cursor = new LineCursor(this.#lines, hunk);
        for (let lineIndex = 0; lineIndex < hunk.count; lineIndex += 1) {
          cursor.advanceTo(lineIndex);
          out.push(cursor.raw);
        }
      }
    }
    return out.join("\n");
  }

  /** Release the scan. */
  close(): void {
    if (this.#closed) return;
    this.#closed = true;
    this.#lines = [];
    this.#entries = [];
    this.#rows = [];
    this.#cursor = undefined;
    openDiffs -= 1;
  }

  /** The cursor placed on the line a row refers to, walking on from where it already stands. */
  #seek(ref: RowRef): LineCursor {
    let cursor = this.#cursor;
    if (
      cursor === undefined ||
      this.#cursorFile !== ref.fileIndex ||
      this.#cursorHunk !== ref.hunkIndex ||
      cursor.logical > ref.lineIndex
    ) {
      const hunk = this.#entries[ref.fileIndex]!.hunks[ref.hunkIndex]!;
      cursor = new LineCursor(this.#lines, hunk);
      this.#cursor = cursor;
      this.#cursorFile = ref.fileIndex;
      this.#cursorHunk = ref.hunkIndex;
    }
    cursor.advanceTo(ref.lineIndex);
    return cursor;
  }

  #row(ref: RowRef): DiffRow {
    const file = this.#entries[ref.fileIndex]!;
    if (ref.kind === "line") {
      const cursor = this.#seek(ref);
      return {
        kind: "line",
        fileIndex: ref.fileIndex,
        hunkIndex: ref.hunkIndex,
        lineIndex: ref.lineIndex,
        lineKind: cursor.kind,
        text: clip(cursor.text),
        oldLineNumber: cursor.oldLineNumber,
        newLineNumber: cursor.newLineNumber,
        noNewline: cursor.noNewline,
      };
    }
    const text =
      ref.kind === "file" ? file.path : ref.kind === "hunk" ? hunkHeader(file.hunks[ref.hunkIndex]!) : NOTICES[ref.lineIndex]!;
    return {
      kind: ref.kind,
      fileIndex: ref.fileIndex,
      hunkIndex: ref.kind === "hunk" ? ref.hunkIndex : 0,
      lineIndex: 0,
      lineKind: "context",
      text,
      oldLineNumber: null,
      newLineNumber: null,
      noNewline: false,
    };
  }
}

function hunkHeader(hunk: HunkEntry | DiffHunk): string {
  const range = `@@ -${hunk.oldStart},${hunk.oldLines} +${hunk.newStart},${hunk.newLines} @@`;
  return hunk.heading ? `${range} ${hunk.heading}` : range;
}

/** Hand the thread back so the host's events and the app's timers run before the next chunk. */
function yieldToEventLoop(): Promise<void> {
  return new Promise<void>((resolve) => {
    setTimeout(() => resolve(), 0);
  });
}

/** Hand the thread back between chunks, and give up on a diff nobody is waiting for. */
async function pause(options: OpenDiffOptions): Promise<void> {
  await yieldToEventLoop();
  if (options.signal?.aborted ?? false) throw new Error("The diff was abandoned");
}

/**
 * The end of the last whole line within `limit` bytes of `from`, so a chunk never splits a
 * character: a newline byte is never part of a multi-byte sequence in UTF-8. A line longer than a
 * chunk is taken whole.
 */
function lineEnd(bytes: Uint8Array, from: number, limit: number): number {
  const hard = Math.min(from + limit, bytes.byteLength);
  if (hard >= bytes.byteLength) return bytes.byteLength;
  for (let index = hard; index > from; index -= 1) if (bytes[index - 1]! === NEWLINE) return index;
  for (let index = hard; index < bytes.byteLength; index += 1) if (bytes[index]! === NEWLINE) return index + 1;
  return bytes.byteLength;
}

/** The first at most `MAX_ROW_TEXT` characters, never splitting a surrogate pair. */
function clip(text: string): string {
  if (text.length <= MAX_ROW_TEXT) return text;
  const code = text.charCodeAt(MAX_ROW_TEXT - 1);
  return text.slice(0, code >= 0xd800 && code <= 0xdbff ? MAX_ROW_TEXT - 1 : MAX_ROW_TEXT);
}

function byteLength(output: string | Uint8Array): number {
  return typeof output === "string" ? Buffer.byteLength(output, "utf8") : output.byteLength;
}

// --- patches -------------------------------------------------------------------------------------

/**
 * Render an untracked file as the diff text of an addition, ready for `Diff.open`.
 *
 * Parsing it yields what `syntheticAddedFile` builds: one hunk born at `-0,0`, a
 * `noNewline` marker when the content lacks a final newline, and no hunk for empty or binary
 * content.
 */
export function syntheticDiffText(path: string, content: string, options: { binary?: boolean } = {}): string {
  const header = `diff --git a/${path} b/${path}\nnew file mode 100644\n`;
  if (options.binary) return `${header}Binary files /dev/null and b/${path} differ\n`;
  const lines = content.split("\n");
  const endsWithNewline = lines.at(-1) === "";
  if (endsWithNewline) lines.pop();
  if (lines.length === 0) return header;
  const body = lines.map((line) => `+${line}`).join("\n");
  return `${header}@@ -0,0 +1,${lines.length} @@\n${body}\n${endsWithNewline ? "" : "\\ No newline at end of file\n"}`;
}

/** Additions and deletions across every hunk. */
export function countChanges(file: DiffFile): { added: number; removed: number } {
  let added = 0;
  let removed = 0;
  for (const hunk of file.hunks) {
    for (const line of hunk.lines) {
      if (line.kind === "added") added += 1;
      else if (line.kind === "removed") removed += 1;
    }
  }
  return { added, removed };
}

/** Synthesize the diff of an untracked file so it renders like an addition. */
export function syntheticAddedFile(path: string, content: string, options: { binary?: boolean } = {}): DiffFile {
  const lines = content.split("\n");
  const endsWithNewline = lines.at(-1) === "";
  if (endsWithNewline) lines.pop();
  const diffLines: DiffLine[] = lines.map((text, index) => ({
    kind: "added",
    text,
    oldLineNumber: null,
    newLineNumber: index + 1,
    noNewline: false,
  }));
  if (diffLines.length > 0 && !endsWithNewline) diffLines[diffLines.length - 1]!.noNewline = true;
  return {
    kind: "added",
    oldPath: null,
    newPath: path,
    path,
    binary: options.binary ?? false,
    newMode: "100644",
    hunks:
      diffLines.length === 0
        ? []
        : [{ oldStart: 0, oldLines: 0, newStart: 1, newLines: diffLines.length, heading: "", lines: diffLines }],
    truncated: false,
  };
}

export interface HunkSelection {
  /** Index of the hunk inside `file.hunks`. */
  hunkIndex: number;
  /** Indices of lines inside the hunk to include; omit to include every changed line. */
  lines?: readonly number[];
}

export interface FormatPatchOptions {
  /**
   * Build the patch for `git apply --reverse`: unselected additions stay as context because the
   * target already contains them, and unselected removals are dropped because it does not.
   */
  reverse?: boolean;
}

/**
 * Build a patch containing the chosen hunks, or the chosen lines of them.
 *
 * Forward patches apply to a target that matches the old side (the index when staging); reverse
 * patches apply to a target that matches the new side (the index when unstaging, the working tree
 * when discarding). Hunks that end up with no change are left out, and a patch with no hunks is
 * the empty string.
 */
export function formatPatch(
  file: DiffFile,
  selections: readonly HunkSelection[],
  options: FormatPatchOptions = {},
): string {
  const reverse = options.reverse ?? false;
  const body: string[] = [];
  let delta = 0;
  let includedAllChanges = true;
  let anyChange = false;
  for (const selection of selections) {
    if (selection.hunkIndex < 0 || selection.hunkIndex >= file.hunks.length) continue;
    const hunk = file.hunks[selection.hunkIndex]!;
    const lines: string[] = [];
    let oldCount = 0;
    let newCount = 0;
    let changed = false;
    hunk.lines.forEach((line, lineIndex) => {
      const selected = selection.lines === undefined || selection.lines.includes(lineIndex);
      let marker: " " | "+" | "-" | null;
      if (line.kind === "context") marker = " ";
      else if (selected) marker = line.kind === "added" ? "+" : "-";
      else if (line.kind === "added") marker = reverse ? " " : null;
      else marker = reverse ? null : " ";
      if (marker === null) {
        includedAllChanges = false;
        return;
      }
      if (marker !== " ") changed = true;
      else if (line.kind !== "context") includedAllChanges = false;
      if (marker !== "+") oldCount += 1;
      if (marker !== "-") newCount += 1;
      lines.push(`${marker}${line.text}`);
      if (line.noNewline) lines.push("\\ No newline at end of file");
    });
    if (!changed) continue;
    anyChange = true;
    // A forward patch locates each hunk by its old-side position, which is unchanged whether or
    // not earlier hunks were included; the new-side start is informational and follows the lines
    // this patch really adds. A reverse patch locates hunks by the new side, which the target
    // holds in full, so the original new start is exact.
    // An empty side starts at 0, as git prints it; a non-empty side never starts before line 1,
    // which matters for a partially reversed addition whose hunk was born at `-0,0`.
    const oldStart = oldCount === 0 ? 0 : Math.max(hunk.oldStart, 1);
    const newStart = newCount === 0 ? 0 : Math.max(reverse ? hunk.newStart : hunk.oldStart + delta, 1);
    body.push(
      `@@ -${oldStart}${countSuffix(oldCount)} +${newStart}${countSuffix(newCount)} @@${hunk.heading ? ` ${hunk.heading}` : ""}`,
    );
    body.push(...lines);
    delta += newCount - oldCount;
  }
  if (!anyChange) return "";

  const header: string[] = [];
  const oldName = file.oldPath ?? file.newPath ?? file.path;
  const newName = file.newPath ?? file.oldPath ?? file.path;
  header.push(`diff --git a/${oldName} b/${newName}`);
  // A partially staged addition is still a new file; a partially unstaged one must keep the file
  // in the index, so it is written as an ordinary modification. Deletions mirror that.
  const wholeFile = includedAllChanges;
  if (file.kind === "added" && (!reverse || wholeFile)) {
    header.push(`new file mode ${file.newMode ?? "100644"}`);
    header.push(`--- /dev/null`);
    header.push(`+++ b/${newName}`);
  } else if (file.kind === "deleted" && wholeFile) {
    header.push(`deleted file mode ${file.oldMode ?? "100644"}`);
    header.push(`--- a/${oldName}`);
    header.push(`+++ /dev/null`);
  } else {
    if (file.kind === "renamed" || file.kind === "copied") {
      if (file.similarity != null) header.push(`similarity index ${file.similarity}%`);
      header.push(`${file.kind === "renamed" ? "rename" : "copy"} from ${oldName}`);
      header.push(`${file.kind === "renamed" ? "rename" : "copy"} to ${newName}`);
    } else if (file.oldMode && file.newMode && file.oldMode !== file.newMode) {
      header.push(`old mode ${file.oldMode}`);
      header.push(`new mode ${file.newMode}`);
    }
    header.push(`--- a/${oldName}`);
    header.push(`+++ b/${newName}`);
  }
  return `${[...header, ...body].join("\n")}\n`;
}

function countSuffix(count: number): string {
  return count === 1 ? "" : `,${count}`;
}

/** Whether a line index set covers every changed line of the hunk. */
export function selectsWholeHunk(hunk: DiffHunk, lines: readonly number[]): boolean {
  return hunk.lines.every((line, index) => line.kind === "context" || lines.includes(index));
}

/** Split a big diff into files whose total line count stays under `limit`, for rendering. */
export function totalLines(files: readonly DiffFile[]): number {
  let total = 0;
  for (const file of files) for (const hunk of file.hunks) total += hunk.lines.length;
  return total;
}

/** Detect binary content the way git does: a NUL byte in the first 8000 bytes. */
export function looksBinary(bytes: Uint8Array): boolean {
  const end = Math.min(bytes.byteLength, 8000);
  for (let index = 0; index < end; index += 1) if (bytes[index] === 0) return true;
  return false;
}
