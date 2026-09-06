/**
 * Unified diff parsing and patch construction.
 *
 * Parsing lives in the native `git` module (`modules/git/main.zig`). The app opens a diff with
 * `Diff.open`, which parses git's raw bytes off the JavaScript thread and keeps the result native;
 * `parseDiff` is the eager form for tests and small inputs. `formatPatch` does the inverse for a
 * chosen subset of a hunk's lines, producing exactly the patch `git apply --cached` (stage),
 * `git apply --cached --reverse` (unstage), or `git apply --reverse` (discard) needs, with
 * recomputed hunk counts. Nothing here runs git.
 */

import {
  parseDiff as parseDiffNative,
  parseDiffAsync as parseDiffNativeAsync,
  parseGitHeaderPaths,
  type DiffFile,
  type DiffHunk,
  type DiffLine,
} from "../modules/git/index.ts";

export type {
  DiffFile,
  DiffFileKind,
  DiffFileSummary,
  DiffHunk,
  DiffLine,
  DiffLineKind,
  DiffRow,
  DiffRowKind,
} from "../modules/git/index.ts";
export { parseGitHeaderPaths };
export { Diff, MAX_DIFF_ROWS, type OpenDiffOptions, type SelectedLines } from "./diff-handle.ts";

export interface ParseDiffOptions {
  /** Whether the output hit the byte bound; marks the last file as truncated. */
  truncated?: boolean;
}

/** Parse the complete output of `git diff` (text or the raw bytes git wrote) into files. */
export function parseDiff(output: string | Uint8Array, options: ParseDiffOptions = {}): DiffFile[] {
  return parseDiffNative(typeof output === "string" ? output : new TextDecoder().decode(output), options.truncated ?? false);
}

/** `parseDiff` on the native thread pool. */
export function parseDiffAsync(output: string | Uint8Array, options: ParseDiffOptions = {}): Promise<DiffFile[]> {
  return parseDiffNativeAsync(typeof output === "string" ? output : new TextDecoder().decode(output), options.truncated ?? false);
}

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
    const hunk = file.hunks[selection.hunkIndex];
    if (!hunk) continue;
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
