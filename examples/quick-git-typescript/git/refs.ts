/**
 * Branch, tag, remote, and stash parsing from `git for-each-ref` and `git stash list`.
 */

import { splitNul } from "./process.ts";

export interface BranchRef {
  /** Short name, `main` or `origin/main`. */
  name: string;
  /** Full ref, `refs/heads/main`. */
  fullName: string;
  sha: string;
  shortSha: string;
  remote: boolean;
  /** Upstream short name for a local branch. */
  upstream?: string;
  ahead: number;
  behind: number;
  /** Whether the upstream is gone. */
  upstreamGone: boolean;
  /** Unix seconds of the tip commit. */
  committerTime: number;
  current: boolean;
  subject: string;
  /** Working tree where this branch is checked out, when git reports one. */
  worktreePath?: string;
}

export interface TagRef {
  name: string;
  sha: string;
  shortSha: string;
  committerTime: number;
  subject: string;
}

export const FOR_EACH_REF_FORMAT = [
  "%(refname)",
  "%(objectname)",
  "%(objectname:short)",
  "%(upstream:short)",
  "%(upstream:track)",
  "%(creatordate:unix)",
  "%(HEAD)",
  "%(subject)",
  "%(worktreepath)",
].join("%00");

export interface RefCollections {
  local: BranchRef[];
  remote: BranchRef[];
  tags: TagRef[];
}

export function parseRefs(output: string): RefCollections {
  const local: BranchRef[] = [];
  const remote: BranchRef[] = [];
  const tags: TagRef[] = [];
  for (const line of output.split("\n")) {
    if (!line) continue;
    const parts = line.split("\0");
    if (parts.length < 9) continue;
    const [fullName, sha, shortSha, upstream, track, time, head, subject, worktreePath] = parts;
    const committerTime = Number(time);
    if (fullName!.startsWith("refs/tags/")) {
      tags.push({
        name: fullName!.slice(10),
        sha: sha!,
        shortSha: shortSha!,
        committerTime,
        subject: subject!,
      });
      continue;
    }
    const isRemote = fullName!.startsWith("refs/remotes/");
    const name = isRemote ? fullName!.slice(13) : fullName!.slice(11);
    if (isRemote && name.endsWith("/HEAD")) continue;
    const counts = parseTrack(track!);
    const branch: BranchRef = {
      name,
      fullName: fullName!,
      sha: sha!,
      shortSha: shortSha!,
      remote: isRemote,
      ...(upstream ? { upstream } : {}),
      ahead: counts.ahead,
      behind: counts.behind,
      upstreamGone: counts.gone,
      committerTime,
      current: head === "*",
      subject: subject!,
      ...(worktreePath ? { worktreePath } : {}),
    };
    (isRemote ? remote : local).push(branch);
  }
  return { local, remote, tags };
}

function parseTrack(track: string): { ahead: number; behind: number; gone: boolean } {
  const result = { ahead: 0, behind: 0, gone: false };
  if (!track) return result;
  if (track.includes("gone")) result.gone = true;
  const ahead = /ahead (\d+)/.exec(track);
  const behind = /behind (\d+)/.exec(track);
  if (ahead) result.ahead = Number(ahead[1]);
  if (behind) result.behind = Number(behind[1]);
  return result;
}

export interface StashEntry {
  /** `stash@{0}` */
  ref: string;
  index: number;
  sha: string;
  /** Unix seconds. */
  time: number;
  /** The full reflog subject, `WIP on main: abc1234 subject` or `On main: message`. */
  message: string;
  /** The branch the stash was made on, when the message names it. */
  branch?: string;
  /** The user's own message, or the WIP subject. */
  summary: string;
}

export const STASH_FORMAT = ["%gd", "%H", "%at", "%gs"].join("%x1f");

export function parseStashes(output: string): StashEntry[] {
  const stashes: StashEntry[] = [];
  for (const record of splitNul(output)) {
    const [ref, sha, time, message] = record.split("\x1f");
    if (!ref || !sha) continue;
    const index = Number(/\{(\d+)\}/.exec(ref)?.[1] ?? stashes.length);
    const named = /^(?:WIP on|On) ([^:]+): (.*)$/s.exec(message ?? "");
    stashes.push({
      ref,
      index,
      sha,
      time: Number(time),
      message: message ?? "",
      ...(named?.[1] ? { branch: named[1] } : {}),
      summary: named?.[2] ?? message ?? "",
    });
  }
  return stashes;
}

/** Validate a branch name the way `git check-ref-format --branch` would, for instant feedback. */
export function branchNameProblem(name: string): string | undefined {
  if (!name) return "Enter a branch name.";
  if (name.startsWith("-")) return "A branch name cannot start with a dash.";
  if (name === "HEAD") return "HEAD is not a valid branch name.";
  if (/[\s~^:?*[\\\x00-\x1f\x7f]/.test(name))
    return "Branch names cannot contain spaces or ~ ^ : ? * [ \\.";
  if (name.includes("..") || name.includes("@{")) return "Branch names cannot contain .. or @{.";
  if (name.startsWith("/") || name.endsWith("/") || name.includes("//"))
    return "Branch names cannot start or end with a slash.";
  if (
    name.endsWith(".") ||
    name.endsWith(".lock") ||
    name.split("/").some((part) => part.startsWith("."))
  ) {
    return "Branch name components cannot start with a dot or end with .lock.";
  }
  return undefined;
}
