/**
 * Unified diff parsing and patch construction.
 *
 * `parseDiff` turns `git diff` output into files, hunks, and lines with both line numbers.
 * `formatPatch` does the inverse for a chosen subset of a hunk's lines, producing exactly the
 * patch `git apply --cached` (stage), `git apply --cached --reverse` (unstage), or
 * `git apply --reverse` (discard) needs, with recomputed hunk counts. Nothing here runs git.
 */

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

/** Parse the complete output of `git diff` into files. */
export function parseDiff(output: string, options: { truncated?: boolean } = {}): DiffFile[] {
  const lines = output.split("\n");
  if (lines.at(-1) === "") lines.pop();
  const files: DiffFile[] = [];
  let index = 0;
  while (index < lines.length) {
    const line = lines[index]!;
    if (!line.startsWith("diff --git ")) {
      index += 1;
      continue;
    }
    const file = emptyFile(parseGitHeaderPaths(line.slice("diff --git ".length)));
    index += 1;
    // Extended header lines up to the first hunk, `---`, or next file.
    while (index < lines.length) {
      const header = lines[index]!;
      if (header.startsWith("diff --git ") || header.startsWith("@@")) break;
      index += 1;
      if (header.startsWith("old mode ")) file.oldMode = header.slice(9).trim();
      else if (header.startsWith("new mode ")) file.newMode = header.slice(9).trim();
      else if (header.startsWith("deleted file mode ")) {
        file.kind = "deleted";
        file.oldMode = header.slice(18).trim();
      } else if (header.startsWith("new file mode ")) {
        file.kind = "added";
        file.newMode = header.slice(14).trim();
      } else if (header.startsWith("similarity index ")) {
        file.similarity = Number.parseInt(header.slice(17), 10);
      } else if (header.startsWith("rename from ")) {
        file.kind = "renamed";
        file.oldPath = header.slice(12);
      } else if (header.startsWith("rename to ")) {
        file.kind = "renamed";
        file.newPath = header.slice(10);
      } else if (header.startsWith("copy from ")) {
        file.kind = "copied";
        file.oldPath = header.slice(10);
      } else if (header.startsWith("copy to ")) {
        file.kind = "copied";
        file.newPath = header.slice(8);
      } else if (header.startsWith("Binary files ")) {
        file.binary = true;
      } else if (header.startsWith("--- ")) {
        const path = stripPrefix(header.slice(4), "a/");
        file.oldPath = path;
      } else if (header.startsWith("+++ ")) {
        const path = stripPrefix(header.slice(4), "b/");
        file.newPath = path;
      }
    }
    if (file.kind === "added") file.oldPath = null;
    if (file.kind === "deleted") file.newPath = null;
    // Hunks.
    while (index < lines.length && lines[index]!.startsWith("@@")) {
      const hunk = parseHunkHeader(lines[index]!);
      index += 1;
      if (!hunk) break;
      let oldLine = hunk.oldStart;
      let newLine = hunk.newStart;
      while (index < lines.length) {
        const body = lines[index]!;
        if (body.startsWith("diff --git ") || body.startsWith("@@")) break;
        index += 1;
        if (body.startsWith("\\")) {
          const last = hunk.lines.at(-1);
          if (last) last.noNewline = true;
          continue;
        }
        const marker = body[0];
        const text = body.slice(1);
        if (marker === "+") {
          hunk.lines.push({ kind: "added", text, oldLineNumber: null, newLineNumber: newLine, noNewline: false });
          newLine += 1;
        } else if (marker === "-") {
          hunk.lines.push({ kind: "removed", text, oldLineNumber: oldLine, newLineNumber: null, noNewline: false });
          oldLine += 1;
        } else if (marker === " " || body.length === 0) {
          hunk.lines.push({ kind: "context", text, oldLineNumber: oldLine, newLineNumber: newLine, noNewline: false });
          oldLine += 1;
          newLine += 1;
        } else {
          // Anything else ends the hunk (for example a truncated tail).
          break;
        }
      }
      file.hunks.push(hunk);
    }
    file.path = file.newPath ?? file.oldPath ?? "";
    files.push(file);
  }
  if (options.truncated && files.length > 0) files[files.length - 1]!.truncated = true;
  return files;
}

function emptyFile(paths: { oldPath: string; newPath: string }): DiffFile {
  return {
    kind: "modified",
    oldPath: paths.oldPath,
    newPath: paths.newPath,
    path: paths.newPath,
    binary: false,
    hunks: [],
    truncated: false,
  };
}

function stripPrefix(path: string, prefix: string): string | null {
  if (path === "/dev/null") return null;
  // A tab may follow the path when it carries a timestamp; git never adds one, but be safe.
  const tab = path.indexOf("\t");
  const clean = tab >= 0 ? path.slice(0, tab) : path;
  return clean.startsWith(prefix) ? clean.slice(prefix.length) : clean;
}

/**
 * Split the `a/<old> b/<new>` pair of a `diff --git` line.
 *
 * Paths may contain spaces, so the split point is the ` b/` whose two halves agree when the file
 * kept its name; otherwise the first ` b/` after `a/` is used and later header lines correct it.
 */
export function parseGitHeaderPaths(pair: string): { oldPath: string; newPath: string } {
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
  const first = candidates[0];
  if (first !== undefined && pair.startsWith("a/")) {
    return { oldPath: pair.slice(2, first), newPath: pair.slice(first + 3) };
  }
  // Quoted or unusual header: fall back to the whole text on both sides.
  const unquoted = pair.replace(/^"|"$/g, "");
  return { oldPath: unquoted, newPath: unquoted };
}

function parseHunkHeader(line: string): DiffHunk | undefined {
  const match = /^@@ -(\d+)(?:,(\d+))? \+(\d+)(?:,(\d+))? @@ ?(.*)$/.exec(line);
  if (!match) return undefined;
  return {
    oldStart: Number(match[1]),
    oldLines: match[2] === undefined ? 1 : Number(match[2]),
    newStart: Number(match[3]),
    newLines: match[4] === undefined ? 1 : Number(match[4]),
    heading: match[5] ?? "",
    lines: [],
  };
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
  lines?: ReadonlySet<number>;
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
    const hunk = file.hunks[selection.hunkIndex];
    if (!hunk) continue;
    const lines: string[] = [];
    let oldCount = 0;
    let newCount = 0;
    let changed = false;
    hunk.lines.forEach((line, lineIndex) => {
      const selected = selection.lines === undefined || selection.lines.has(lineIndex);
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
      if (file.similarity !== undefined) header.push(`similarity index ${file.similarity}%`);
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
export function selectsWholeHunk(hunk: DiffHunk, lines: ReadonlySet<number>): boolean {
  return hunk.lines.every((line, index) => line.kind === "context" || lines.has(index));
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
