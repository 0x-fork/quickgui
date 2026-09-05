/**
 * `git status --porcelain=v2 -z` parsing.
 *
 * The v2 format is the only status output whose paths, renames, and conflict stages are
 * unambiguous; `-z` keeps every path byte-exact. Everything the app shows about the working tree
 * derives from one `RepositoryStatus` snapshot.
 */

import { splitNul } from "./process.ts";

/** One character of the porcelain `XY` pair. */
export type StatusCode = "." | "M" | "T" | "A" | "D" | "R" | "C" | "U" | "?" | "!";

export type StatusEntryKind =
  | "ordinary"
  | "renamed"
  | "copied"
  | "unmerged"
  | "untracked"
  | "ignored";

export interface StatusEntry {
  kind: StatusEntryKind;
  /** Path relative to the repository root, using `/` separators. */
  path: string;
  /** Source path of a rename or copy. */
  originalPath?: string;
  /** Index (staged) status. `.` when unchanged. `?` for untracked, `!` for ignored. */
  index: StatusCode;
  /** Working-tree status. `.` when unchanged. */
  worktree: StatusCode;
  /** Whether this path is a submodule. */
  submodule: boolean;
  /** Rename or copy similarity, 0–100. */
  similarity?: number;
  /** Octal modes as git prints them; `000000` when absent. */
  headMode: string;
  indexMode: string;
  worktreeMode: string;
  headSha: string;
  indexSha: string;
  /** Present for unmerged entries: the three stage modes and object names. */
  conflict?: {
    stage1Mode: string;
    stage2Mode: string;
    stage3Mode: string;
    stage1Sha: string;
    stage2Sha: string;
    stage3Sha: string;
  };
}

export interface RepositoryStatus {
  /** `undefined` on an unborn branch. */
  headSha?: string;
  /** `undefined` when HEAD is detached. */
  branch?: string;
  detached: boolean;
  upstream?: string;
  ahead: number;
  behind: number;
  /** Whether the branch has an upstream whose ahead/behind git could compute. */
  hasUpstreamCounts: boolean;
  stashCount: number;
  entries: StatusEntry[];
}

export function parseStatus(output: string): RepositoryStatus {
  const status: RepositoryStatus = {
    detached: false,
    ahead: 0,
    behind: 0,
    hasUpstreamCounts: false,
    stashCount: 0,
    entries: [],
  };
  const records = splitNul(output);
  for (let index = 0; index < records.length; index += 1) {
    const record = records[index]!;
    if (record.startsWith("# ")) {
      parseHeader(record.slice(2), status);
      continue;
    }
    const type = record[0];
    switch (type) {
      case "1": {
        const entry = parseOrdinary(record);
        if (entry) status.entries.push(entry);
        break;
      }
      case "2": {
        // A rename or copy record carries its source path as the next NUL-separated token.
        index += 1;
        const original = records[index] ?? "";
        const entry = parseRenamed(record, original);
        if (entry) status.entries.push(entry);
        break;
      }
      case "u": {
        const entry = parseUnmerged(record);
        if (entry) status.entries.push(entry);
        break;
      }
      case "?":
        status.entries.push(untrackedEntry(record.slice(2), "untracked", "?"));
        break;
      case "!":
        status.entries.push(untrackedEntry(record.slice(2), "ignored", "!"));
        break;
      default:
        break;
    }
  }
  return status;
}

function parseHeader(line: string, status: RepositoryStatus): void {
  const space = line.indexOf(" ");
  const key = space < 0 ? line : line.slice(0, space);
  const value = space < 0 ? "" : line.slice(space + 1);
  switch (key) {
    case "branch.oid":
      if (value !== "(initial)") status.headSha = value;
      break;
    case "branch.head":
      if (value === "(detached)") status.detached = true;
      else status.branch = value;
      break;
    case "branch.upstream":
      status.upstream = value;
      break;
    case "branch.ab": {
      const match = /^\+(\d+) -(\d+)$/.exec(value);
      if (match) {
        status.ahead = Number(match[1]);
        status.behind = Number(match[2]);
        status.hasUpstreamCounts = true;
      }
      break;
    }
    case "stash": {
      const count = Number(value);
      if (Number.isSafeInteger(count) && count >= 0) status.stashCount = count;
      break;
    }
    default:
      break;
  }
}

/** Take `count` space-separated fields from the front of `line`; the remainder is the last. */
function fields(line: string, count: number): string[] | undefined {
  const parts: string[] = [];
  let rest = line;
  for (let index = 0; index < count; index += 1) {
    const space = rest.indexOf(" ");
    if (space < 0) return undefined;
    parts.push(rest.slice(0, space));
    rest = rest.slice(space + 1);
  }
  parts.push(rest);
  return parts;
}

function code(value: string | undefined): StatusCode {
  switch (value) {
    case ".":
    case "M":
    case "T":
    case "A":
    case "D":
    case "R":
    case "C":
    case "U":
    case "?":
    case "!":
      return value;
    default:
      return ".";
  }
}

function parseOrdinary(record: string): StatusEntry | undefined {
  // 1 <XY> <sub> <mH> <mI> <mW> <hH> <hI> <path>
  const parts = fields(record, 8);
  if (!parts) return undefined;
  const [, xy, sub, headMode, indexMode, worktreeMode, headSha, indexSha, path] = parts;
  return {
    kind: "ordinary",
    path: path!,
    index: code(xy![0]),
    worktree: code(xy![1]),
    submodule: sub !== "N...",
    headMode: headMode!,
    indexMode: indexMode!,
    worktreeMode: worktreeMode!,
    headSha: headSha!,
    indexSha: indexSha!,
  };
}

function parseRenamed(record: string, originalPath: string): StatusEntry | undefined {
  // 2 <XY> <sub> <mH> <mI> <mW> <hH> <hI> <X><score> <path>
  const parts = fields(record, 9);
  if (!parts) return undefined;
  const [, xy, sub, headMode, indexMode, worktreeMode, headSha, indexSha, score, path] = parts;
  const similarity = Number(score!.slice(1));
  return {
    kind: score!.startsWith("C") ? "copied" : "renamed",
    path: path!,
    originalPath,
    index: code(xy![0]),
    worktree: code(xy![1]),
    submodule: sub !== "N...",
    ...(Number.isFinite(similarity) ? { similarity } : {}),
    headMode: headMode!,
    indexMode: indexMode!,
    worktreeMode: worktreeMode!,
    headSha: headSha!,
    indexSha: indexSha!,
  };
}

function parseUnmerged(record: string): StatusEntry | undefined {
  // u <XY> <sub> <m1> <m2> <m3> <mW> <h1> <h2> <h3> <path>
  const parts = fields(record, 10);
  if (!parts) return undefined;
  const [, xy, sub, stage1Mode, stage2Mode, stage3Mode, worktreeMode, stage1Sha, stage2Sha, stage3Sha, path] =
    parts;
  return {
    kind: "unmerged",
    path: path!,
    index: code(xy![0]),
    worktree: code(xy![1]),
    submodule: sub !== "N...",
    headMode: stage2Mode!,
    indexMode: stage2Mode!,
    worktreeMode: worktreeMode!,
    headSha: stage2Sha!,
    indexSha: stage2Sha!,
    conflict: {
      stage1Mode: stage1Mode!,
      stage2Mode: stage2Mode!,
      stage3Mode: stage3Mode!,
      stage1Sha: stage1Sha!,
      stage2Sha: stage2Sha!,
      stage3Sha: stage3Sha!,
    },
  };
}

function untrackedEntry(path: string, kind: "untracked" | "ignored", mark: "?" | "!"): StatusEntry {
  return {
    kind,
    path,
    index: mark,
    worktree: mark,
    submodule: false,
    headMode: "000000",
    indexMode: "000000",
    worktreeMode: "000000",
    headSha: "0".repeat(40),
    indexSha: "0".repeat(40),
  };
}

/** A file as one of the two change lists shows it. */
export interface ChangeItem {
  /** Stable identity across refreshes: `staged:` or `unstaged:` plus the path. */
  id: string;
  entry: StatusEntry;
  path: string;
  originalPath?: string;
  /** The status letter this list shows for the file. */
  code: StatusCode;
  staged: boolean;
  conflicted: boolean;
}

/** Working-tree changes: modified, deleted, untracked, and conflicted files. */
export function unstagedChanges(status: RepositoryStatus): ChangeItem[] {
  const items: ChangeItem[] = [];
  for (const entry of status.entries) {
    if (entry.kind === "ignored") continue;
    if (entry.kind === "unmerged") {
      items.push(changeItem(entry, "U", false, true));
      continue;
    }
    if (entry.kind === "untracked") {
      items.push(changeItem(entry, "?", false, false));
      continue;
    }
    if (entry.worktree !== ".") items.push(changeItem(entry, entry.worktree, false, false));
  }
  return items;
}

/** Index changes: everything that would go into the next commit. */
export function stagedChanges(status: RepositoryStatus): ChangeItem[] {
  const items: ChangeItem[] = [];
  for (const entry of status.entries) {
    if (entry.kind === "untracked" || entry.kind === "ignored" || entry.kind === "unmerged") continue;
    if (entry.index !== ".") items.push(changeItem(entry, entry.index, true, false));
  }
  return items;
}

function changeItem(
  entry: StatusEntry,
  code: StatusCode,
  staged: boolean,
  conflicted: boolean,
): ChangeItem {
  return {
    id: `${staged ? "staged" : "unstaged"}:${entry.path}`,
    entry,
    path: entry.path,
    ...(entry.originalPath !== undefined ? { originalPath: entry.originalPath } : {}),
    code,
    staged,
    conflicted,
  };
}

/** Human label for a status letter. */
export function describeStatusCode(code: StatusCode): string {
  switch (code) {
    case "M":
      return "Modified";
    case "T":
      return "Type changed";
    case "A":
      return "Added";
    case "D":
      return "Deleted";
    case "R":
      return "Renamed";
    case "C":
      return "Copied";
    case "U":
      return "Conflicted";
    case "?":
      return "Untracked";
    case "!":
      return "Ignored";
    default:
      return "Unchanged";
  }
}

/** Whether the status describes a clean tree with nothing to commit. */
export function isClean(status: RepositoryStatus): boolean {
  return status.entries.every((entry) => entry.kind === "ignored");
}
