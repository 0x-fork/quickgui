import { describe, expect, test } from "bun:test";

import { layoutGraph, parseDecorations, parseLog, relativeTime, type Commit } from "./log.ts";

function record(fields: string[]): string {
  return `${fields.join("\x1f")}\0`;
}

describe("log parsing", () => {
  test("parses NUL-separated records with bodies and decorations", () => {
    const output =
      record([
        "a".repeat(40),
        "aaaaaaa",
        `${"b".repeat(40)} ${"c".repeat(40)}`,
        "Ada",
        "ada@example.com",
        "1700000000",
        "Ada",
        "ada@example.com",
        "1700000001",
        "HEAD -> main, origin/main, tag: v1.0",
        "Merge feature",
        "Body line 1\n\nBody line 2\n",
      ]) +
      record([
        "b".repeat(40),
        "bbbbbbb",
        "",
        "Bob",
        "bob@example.com",
        "1600000000",
        "Bob",
        "bob@example.com",
        "1600000000",
        "",
        "Initial",
        "",
      ]);
    const commits = parseLog(output);
    expect(commits).toHaveLength(2);
    expect(commits[0]).toMatchObject({
      shortSha: "aaaaaaa",
      parents: ["b".repeat(40), "c".repeat(40)],
      authorName: "Ada",
      authorTime: 1700000000,
      subject: "Merge feature",
      body: "Body line 1\n\nBody line 2",
    });
    expect(commits[0]!.refs).toEqual([
      { kind: "branch", name: "main", current: true },
      { kind: "remote", name: "origin/main", current: false },
      { kind: "tag", name: "v1.0", current: false },
    ]);
    expect(commits[1]!.parents).toEqual([]);
    expect(commits[1]!.refs).toEqual([]);
  });

  test("parses a detached HEAD decoration", () => {
    expect(parseDecorations("HEAD, tag: v2")).toEqual([
      { kind: "head", name: "HEAD", current: true },
      { kind: "tag", name: "v2", current: false },
    ]);
  });
});

function commit(sha: string, parents: string[]): Commit {
  return {
    sha,
    shortSha: sha.slice(0, 7),
    parents,
    authorName: "",
    authorEmail: "",
    authorTime: 0,
    committerName: "",
    committerEmail: "",
    committerTime: 0,
    refs: [],
    subject: sha,
    body: "",
  };
}

describe("graph layout", () => {
  test("keeps a linear history in one lane", () => {
    const rows = layoutGraph([commit("c", ["b"]), commit("b", ["a"]), commit("a", [])]);
    expect(rows.map((row) => row.lane)).toEqual([0, 0, 0]);
    expect(rows.map((row) => row.laneCount)).toEqual([1, 1, 1]);
    expect(rows[2]!.edges).toEqual([]);
  });

  test("opens a lane for a merge's second parent and joins it at the commit where the branches meet", () => {
    // m merges f into d; f branched from b.
    const rows = layoutGraph([
      commit("m", ["d", "f"]),
      commit("d", ["b"]),
      commit("f", ["b"]),
      commit("b", ["a"]),
      commit("a", []),
    ]);
    // A tip has no line coming in from above.
    expect(rows[0]).toMatchObject({
      lane: 0,
      merge: true,
      laneCount: 2,
      incoming: false,
      joins: [],
    });
    expect(rows[0]!.edges).toEqual([
      { fromLane: 0, toLane: 0, color: 0 },
      { fromLane: 0, toLane: 1, color: 1 },
    ]);
    expect(rows[1]).toMatchObject({ lane: 0, incoming: true });
    expect(rows[1]!.passing).toEqual([{ lane: 1, color: 1 }]);
    // f keeps lane 1 down to b even though lane 0 already awaits b.
    expect(rows[2]).toMatchObject({ lane: 1, color: 1, incoming: true, laneCount: 2 });
    expect(rows[2]!.passing).toEqual([{ lane: 0, color: 0 }]);
    expect(rows[2]!.edges).toEqual([{ fromLane: 1, toLane: 1, color: 1 }]);
    // Both lines meet in b's node: lane 1 joins from above and the row still spans two lanes.
    expect(rows[3]).toMatchObject({ lane: 0, color: 0, incoming: true, laneCount: 2 });
    expect(rows[3]!.joins).toEqual([{ fromLane: 1, color: 1 }]);
    expect(rows[3]!.passing).toEqual([]);
    expect(rows[3]!.edges).toEqual([{ fromLane: 0, toLane: 0, color: 0 }]);
    expect(rows[4]).toMatchObject({ lane: 0, incoming: true, laneCount: 1, edges: [] });
  });

  test("opens a lane per second parent and joins converging lines where the commits meet", () => {
    // m2 merges s2 and m1 merges s1; s2 follows s1 on one side branch off base.
    const rows = layoutGraph([
      commit("m2", ["m1", "s2"]),
      commit("m1", ["base", "s1"]),
      commit("s2", ["s1"]),
      commit("s1", ["base"]),
      commit("base", []),
    ]);
    expect(rows[0]!.edges).toEqual([
      { fromLane: 0, toLane: 0, color: 0 },
      { fromLane: 0, toLane: 1, color: 1 },
    ]);
    // Lane 1 still awaits s2, so m1's second parent s1 opens lane 2 rather than sharing it.
    expect(rows[1]!.edges).toEqual([
      { fromLane: 0, toLane: 0, color: 0 },
      { fromLane: 0, toLane: 2, color: 2 },
    ]);
    expect(rows[1]!.passing).toEqual([{ lane: 1, color: 1 }]);
    expect(rows[1]).toMatchObject({ laneCount: 3 });
    // s2 keeps lane 1 down to s1 while lanes 0 and 2 pass by.
    expect(rows[2]).toMatchObject({ lane: 1, color: 1, incoming: true });
    expect(rows[2]!.passing).toEqual([
      { lane: 0, color: 0 },
      { lane: 2, color: 2 },
    ]);
    // s1 is awaited by lanes 1 and 2: it takes lane 1 and lane 2 joins it from the right.
    expect(rows[3]).toMatchObject({ lane: 1, color: 1, incoming: true, laneCount: 3 });
    expect(rows[3]!.joins).toEqual([{ fromLane: 2, color: 2 }]);
    // base is awaited by lanes 0 and 1: lane 1 joins it from the right and the graph narrows.
    expect(rows[4]).toMatchObject({ lane: 0, incoming: true, laneCount: 2 });
    expect(rows[4]!.joins).toEqual([{ fromLane: 1, color: 1 }]);
  });

  test("describes relative times", () => {
    const now = 1_700_000_000_000;
    expect(relativeTime(now / 1000 - 10, now)).toBe("just now");
    expect(relativeTime(now / 1000 - 300, now)).toBe("5 min ago");
    expect(relativeTime(now / 1000 - 3600 * 5, now)).toBe("5 hours ago");
    expect(relativeTime(now / 1000 - 86400 * 400, now)).toBe("1 year ago");
  });
});
