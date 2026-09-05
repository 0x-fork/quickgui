/**
 * Commit history parsing and graph lane layout.
 *
 * `git log` records travel as NUL-separated records with `\x1f` between fields, so subjects,
 * bodies, and names can contain anything but those two bytes. `layoutGraph` assigns each commit a
 * lane and the edges to its parents so a row can be painted from one small record.
 */

import { splitNul } from "./process.ts";

export interface Commit {
  sha: string;
  shortSha: string;
  parents: string[];
  authorName: string;
  authorEmail: string;
  /** Unix seconds. */
  authorTime: number;
  committerName: string;
  committerEmail: string;
  committerTime: number;
  /** Decorations such as `HEAD -> main`, `origin/main`, or `tag: v1.0`. */
  refs: CommitRef[];
  subject: string;
  body: string;
}

export type CommitRefKind = "head" | "branch" | "remote" | "tag" | "other";

export interface CommitRef {
  kind: CommitRefKind;
  name: string;
  /** Whether HEAD points at this ref (`HEAD -> main`). */
  current: boolean;
}

export const LOG_FORMAT = ["%H", "%h", "%P", "%an", "%ae", "%at", "%cn", "%ce", "%ct", "%D", "%s", "%b"].join(
  "%x1f",
);

export function parseLog(output: string): Commit[] {
  const commits: Commit[] = [];
  for (const record of splitNul(output)) {
    const parts = record.split("\x1f");
    if (parts.length < 12) continue;
    const [sha, shortSha, parents, authorName, authorEmail, authorTime, committerName, committerEmail, committerTime, decorations, subject, ...rest] =
      parts;
    commits.push({
      sha: sha!.trim(),
      shortSha: shortSha!,
      parents: parents!.split(" ").filter((parent) => parent.length > 0),
      authorName: authorName!,
      authorEmail: authorEmail!,
      authorTime: Number(authorTime),
      committerName: committerName!,
      committerEmail: committerEmail!,
      committerTime: Number(committerTime),
      refs: parseDecorations(decorations!),
      subject: subject!,
      body: rest.join("\x1f").replace(/\n+$/, ""),
    });
  }
  return commits;
}

export function parseDecorations(decorations: string): CommitRef[] {
  const refs: CommitRef[] = [];
  const trimmed = decorations.trim();
  if (!trimmed) return refs;
  for (const raw of trimmed.split(", ")) {
    let name = raw.trim();
    if (!name) continue;
    if (name === "HEAD") {
      refs.push({ kind: "head", name: "HEAD", current: true });
      continue;
    }
    let current = false;
    const arrow = name.indexOf(" -> ");
    if (arrow >= 0) {
      current = true;
      name = name.slice(arrow + 4);
    }
    if (name.startsWith("tag: ")) {
      refs.push({ kind: "tag", name: name.slice(5), current });
    } else if (name.startsWith("refs/remotes/") || /^[^/\s]+\/.+/.test(name) && !name.startsWith("refs/")) {
      refs.push({ kind: "remote", name: name.replace(/^refs\/remotes\//, ""), current });
    } else if (name.startsWith("refs/")) {
      refs.push({ kind: "other", name, current });
    } else {
      refs.push({ kind: "branch", name, current });
    }
  }
  return refs;
}

/** One painted edge leaving a commit's node toward a parent, from its lane into a lane below. */
export interface GraphEdge {
  fromLane: number;
  toLane: number;
  /** Lane color index, stable per branch line. */
  color: number;
}

/** A lane from the row above whose line ends in this commit's node. */
export interface GraphJoin {
  fromLane: number;
  color: number;
}

export interface GraphRow {
  /** Lane of this commit's node. */
  lane: number;
  color: number;
  /** Whether the node's own lane continues from the row above, because a child awaited it. */
  incoming: boolean;
  /** Other lanes from the row above that end in this commit: children on other branches. */
  joins: GraphJoin[];
  /** Lanes that pass straight through this row without touching the commit. */
  passing: { lane: number; color: number }[];
  /** Edges leaving this commit toward its parents, drawn into the next row. */
  edges: GraphEdge[];
  /** Number of lanes in use across this row (the width to paint). */
  laneCount: number;
  merge: boolean;
}

/**
 * Assign lanes to commits in the order `git log` returned them.
 *
 * Each open lane waits for one commit (the next parent to appear). A commit takes the lane that
 * waits for it, keeps it for its first parent, and opens a lane per extra parent. A branch line
 * stays in its own lane until its parent's row, where it joins the parent's node, so lines
 * converge where the history does rather than at the child. Colors follow lanes, so one branch
 * keeps one color until it ends.
 */
export function layoutGraph(commits: readonly Commit[]): GraphRow[] {
  const rows: GraphRow[] = [];
  const lanes: ({ sha: string; color: number } | null)[] = [];
  let nextColor = 0;
  const claimLane = (sha: string, color: number): number => {
    const free = lanes.indexOf(null);
    if (free >= 0) {
      lanes[free] = { sha, color };
      return free;
    }
    lanes.push({ sha, color });
    return lanes.length - 1;
  };
  for (const commit of commits) {
    let lane = lanes.findIndex((entry) => entry?.sha === commit.sha);
    const incoming = lane >= 0;
    let color: number;
    if (lane < 0) {
      color = nextColor;
      nextColor += 1;
      lane = claimLane(commit.sha, color);
    } else {
      color = lanes[lane]!.color;
    }
    // Other lanes waiting for this same commit end in its node.
    const joins: GraphJoin[] = [];
    lanes.forEach((entry, index) => {
      if (index !== lane && entry?.sha === commit.sha) {
        joins.push({ fromLane: index, color: entry.color });
        lanes[index] = null;
      }
    });
    const passing = lanes
      .map((entry, index) => (entry && index !== lane ? { lane: index, color: entry.color } : null))
      .filter((entry): entry is { lane: number; color: number } => entry !== null);
    const laneCountBefore = lanes.length;
    const edges: GraphEdge[] = [];
    const [first, ...others] = commit.parents;
    if (first === undefined) {
      lanes[lane] = null;
    } else {
      // The line keeps its lane down to the first parent even when another lane already awaits
      // that parent; both end in the parent's node as joins there.
      lanes[lane] = { sha: first, color };
      edges.push({ fromLane: lane, toLane: lane, color });
    }
    for (const parent of others) {
      const existing = lanes.findIndex((entry) => entry?.sha === parent);
      if (existing >= 0) {
        edges.push({ fromLane: lane, toLane: existing, color: lanes[existing]!.color });
      } else {
        const parentColor = nextColor;
        nextColor += 1;
        const parentLane = claimLane(parent, parentColor);
        edges.push({ fromLane: lane, toLane: parentLane, color: parentColor });
      }
    }
    while (lanes.length > 0 && lanes[lanes.length - 1] === null) lanes.pop();
    rows.push({
      lane,
      color,
      incoming,
      joins,
      passing,
      edges,
      laneCount: Math.max(laneCountBefore, lanes.length, lane + 1),
      merge: commit.parents.length > 1,
    });
  }
  return rows;
}

/** Relative time such as `3 minutes ago` or `2 years ago`, for list rows. */
export function relativeTime(unixSeconds: number, now = Date.now()): string {
  const seconds = Math.max(0, Math.round(now / 1000 - unixSeconds));
  if (seconds < 45) return "just now";
  const minutes = Math.round(seconds / 60);
  if (minutes < 60) return `${minutes} min ago`;
  const hours = Math.round(minutes / 60);
  if (hours < 24) return `${hours} ${hours === 1 ? "hour" : "hours"} ago`;
  const days = Math.round(hours / 24);
  if (days < 30) return `${days} ${days === 1 ? "day" : "days"} ago`;
  const months = Math.round(days / 30);
  if (months < 12) return `${months} ${months === 1 ? "month" : "months"} ago`;
  const years = Math.round(days / 365);
  return `${years} ${years === 1 ? "year" : "years"} ago`;
}

/** Absolute local time such as `Sep 5, 2026, 14:03`. */
export function absoluteTime(unixSeconds: number): string {
  const date = new Date(unixSeconds * 1000);
  return date.toLocaleString(undefined, {
    year: "numeric",
    month: "short",
    day: "numeric",
    hour: "2-digit",
    minute: "2-digit",
  });
}
