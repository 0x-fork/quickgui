/**
 * A diff parsed once and kept in the native `git` module (`modules/git/main.zig`).
 *
 * `Diff.open` hands git's raw bytes to Zig on the native thread pool and gets back only a
 * summary: the files, their counts, and the number of table rows. Rows are fetched as the table
 * scrolls, the changed lines of a selection are computed natively, and a full `DiffFile` is
 * materialized only for the one file a patch is built from. A 32 MB diff therefore never becomes
 * hundreds of thousands of JavaScript objects; JavaScript holds a handle and a few kilobytes.
 *
 * Handles are released with `close()`. A `Diff` that is garbage-collected without being closed is
 * released by a finalizer, so a forgotten handle cannot leak the parsed diff for good.
 */

import { Buffer } from "node:buffer";

import {
  closeDiff,
  diffFile,
  diffRows,
  diffSelectedLines,
  openDiff,
  openDiffAsync,
  openDiffCount,
  renderDiff,
  renderDiffAsync,
  type DiffFile,
  type DiffFileSummary,
  type DiffLineGroup,
  type DiffRow,
  type DiffSummary,
} from "../modules/git/index.ts";

/** Most rows the diff table shows; the rest is replaced by a notice. */
export const MAX_DIFF_ROWS = 60_000;

export interface OpenDiffOptions {
  /** Whether git's output hit the byte bound; marks the last file as truncated. */
  truncated?: boolean;
  /** Row cap for the table. Defaults to {@link MAX_DIFF_ROWS}. */
  maxRows?: number;
}

/** Changed lines of one hunk inside a selection, as `formatPatch` wants them. */
export interface SelectedLines {
  fileIndex: number;
  hunkIndex: number;
  lines: Set<number>;
}

const finalizer = new FinalizationRegistry<number>((id) => {
  closeDiff(id);
});

export class Diff {
  readonly id: number;
  readonly files: readonly DiffFileSummary[];
  readonly rowCount: number;
  /** Added and removed lines across every file, whether or not they fit in the table. */
  readonly added: number;
  readonly removed: number;
  /** Size of the text the diff was parsed from, for cache accounting. */
  readonly bytes: number;
  #closed = false;
  #pending = 0;
  #closeWhenIdle = false;

  private constructor(summary: DiffSummary, bytes: number) {
    this.id = summary.id;
    this.files = summary.files;
    this.rowCount = summary.rowCount;
    this.added = summary.added;
    this.removed = summary.removed;
    this.bytes = bytes;
    finalizer.register(this, this.id, this);
  }

  /** Parse on the native thread pool; the event loop keeps running while a big diff is read. */
  static async open(output: string | Uint8Array, options: OpenDiffOptions = {}): Promise<Diff> {
    const summary = await openDiffAsync(output, options.truncated ?? false, options.maxRows ?? MAX_DIFF_ROWS);
    return new Diff(summary, byteLength(output));
  }

  /** Parse on the calling thread, for tests and tiny diffs. */
  static openSync(output: string | Uint8Array, options: OpenDiffOptions = {}): Diff {
    const summary = openDiff(output, options.truncated ?? false, options.maxRows ?? MAX_DIFF_ROWS);
    return new Diff(summary, byteLength(output));
  }

  /** How many diffs the native module currently holds, for tests. */
  static openCount(): number {
    return openDiffCount();
  }

  get closed(): boolean {
    return this.#closed;
  }

  /** The table rows in `[start, end)`; empty once the diff is closed. */
  rows(start: number, end: number): DiffRow[] {
    if (this.#closed) return [];
    return diffRows(this.id, Math.max(0, start), Math.max(0, end));
  }

  /** One file with every hunk and line, for `formatPatch`. */
  file(index: number): DiffFile | undefined {
    if (this.#closed || index < 0 || index >= this.files.length) return undefined;
    return diffFile(this.id, index);
  }

  /** The changed lines inside the table's selected row ranges, grouped by hunk. */
  selectedLines(ranges: readonly (readonly number[])[]): SelectedLines[] {
    if (this.#closed || ranges.length === 0) return [];
    const pairs = ranges
      .filter((range) => range.length >= 2)
      .map((range): number[] => [Math.max(0, range[0]!), Math.max(0, range[1]!)]);
    return diffSelectedLines(this.id, pairs).map((group: DiffLineGroup) => ({
      fileIndex: group.fileIndex,
      hunkIndex: group.hunkIndex,
      lines: new Set(group.lines),
    }));
  }

  /** The diff as prompt text: one header per file, hunk headers, and marked lines. */
  render(): string {
    if (this.#closed) return "";
    return renderDiff(this.id);
  }

  /** `render` on the native thread pool. The diff stays open until it resolves. */
  async renderAsync(): Promise<string> {
    if (this.#closed) return "";
    this.#pending += 1;
    try {
      return await renderDiffAsync(this.id);
    } finally {
      this.#pending -= 1;
      if (this.#closeWhenIdle && this.#pending === 0) this.close();
    }
  }

  /** Release the parsed diff. Waits for an in-flight `renderAsync` before freeing it. */
  close(): void {
    if (this.#closed) return;
    if (this.#pending > 0) {
      this.#closeWhenIdle = true;
      return;
    }
    this.#closed = true;
    finalizer.unregister(this);
    closeDiff(this.id);
  }
}

function byteLength(output: string | Uint8Array): number {
  return typeof output === "string" ? Buffer.byteLength(output, "utf8") : output.byteLength;
}
