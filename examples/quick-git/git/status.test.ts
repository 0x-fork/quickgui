import { describe, expect, test } from "bun:test";

import { describeStatusCode, isClean, parseStatus, stagedChanges, unstagedChanges } from "./status.ts";

const NUL = "\0";

describe("porcelain v2 status", () => {
  test("parses branch headers, ordinary, renamed, unmerged, untracked, and ignored entries", () => {
    const output = [
      "# branch.oid 1156c997a6001274d2579d00a5dbb606add3f528",
      "# branch.head main",
      "# branch.upstream origin/main",
      "# branch.ab +2 -1",
      "# stash 3",
      "1 .M N... 100644 100644 100644 422c2b7ab3b3c668038da977e4e93a5fc623169c 422c2b7ab3b3c668038da977e4e93a5fc623169c src/with space.txt",
      "1 A. N... 000000 100644 100644 0000000000000000000000000000000000000000 6372083f6a5f1b6d3e4c8f2a3b4c5d6e7f8a9b0c added.txt",
      "2 R. N... 100644 100644 100644 422c2b7ab3b3c668038da977e4e93a5fc623169c 422c2b7ab3b3c668038da977e4e93a5fc623169c R100 new name.txt",
      "old name.txt",
      "u UU N... 100644 100644 100644 100644 aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb cccccccccccccccccccccccccccccccccccccccc conflict.txt",
      "1 .M S.M. 160000 160000 160000 dddddddddddddddddddddddddddddddddddddddd dddddddddddddddddddddddddddddddddddddddd vendor/lib",
      "? notes.md",
      "! build/out.o",
    ].join(NUL) + NUL;

    const status = parseStatus(output);
    expect(status.headSha).toBe("1156c997a6001274d2579d00a5dbb606add3f528");
    expect(status.branch).toBe("main");
    expect(status.detached).toBe(false);
    expect(status.upstream).toBe("origin/main");
    expect(status).toMatchObject({ ahead: 2, behind: 1, hasUpstreamCounts: true, stashCount: 3 });
    expect(status.entries.map((entry) => [entry.kind, entry.path, entry.index, entry.worktree])).toEqual([
      ["ordinary", "src/with space.txt", ".", "M"],
      ["ordinary", "added.txt", "A", "."],
      ["renamed", "new name.txt", "R", "."],
      ["unmerged", "conflict.txt", "U", "U"],
      ["ordinary", "vendor/lib", ".", "M"],
      ["untracked", "notes.md", "?", "?"],
      ["ignored", "build/out.o", "!", "!"],
    ]);
    const renamed = status.entries[2]!;
    expect(renamed.originalPath).toBe("old name.txt");
    expect(renamed.similarity).toBe(100);
    expect(status.entries[3]!.conflict?.stage3Sha).toBe("cccccccccccccccccccccccccccccccccccccccc");
    expect(status.entries[4]!.submodule).toBe(true);
    expect(isClean(status)).toBe(false);

    expect(unstagedChanges(status).map((item) => [item.path, item.code, item.conflicted])).toEqual([
      ["src/with space.txt", "M", false],
      ["conflict.txt", "U", true],
      ["vendor/lib", "M", false],
      ["notes.md", "?", false],
    ]);
    expect(stagedChanges(status).map((item) => [item.path, item.code, item.originalPath])).toEqual([
      ["added.txt", "A", undefined],
      ["new name.txt", "R", "old name.txt"],
    ]);
    expect(stagedChanges(status)[1]!.id).toBe("staged:new name.txt");
  });

  test("reads an unborn branch and a detached head", () => {
    const unborn = parseStatus(["# branch.oid (initial)", "# branch.head main", "? a"].join(NUL) + NUL);
    expect(unborn.headSha).toBeUndefined();
    expect(unborn.branch).toBe("main");
    expect(unborn.hasUpstreamCounts).toBe(false);
    const detached = parseStatus(["# branch.oid abc", "# branch.head (detached)"].join(NUL) + NUL);
    expect(detached.detached).toBe(true);
    expect(detached.branch).toBeUndefined();
    expect(isClean(detached)).toBe(true);
  });

  test("names every status letter", () => {
    expect(describeStatusCode("?")).toBe("Untracked");
    expect(describeStatusCode("R")).toBe("Renamed");
    expect(describeStatusCode(".")).toBe("Unchanged");
  });
});
