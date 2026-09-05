import { describe, expect, test } from "bun:test";

import { branchNameProblem, parseRefs, parseStashes } from "./refs.ts";
import { parseWorktrees, worktreeDirectoryName } from "./worktree.ts";

describe("refs", () => {
  test("parses branches, remotes, tags, tracking, and worktree paths", () => {
    const output = [
      ["refs/heads/main", "a".repeat(40), "aaaaaaa", "origin/main", "[ahead 1, behind 2]", "1700000000", "*", "Subject", "/repo"].join("\0"),
      ["refs/heads/feature", "b".repeat(40), "bbbbbbb", "origin/feature", "[gone]", "1600000000", " ", "Feature", "/repo-feature"].join("\0"),
      ["refs/remotes/origin/main", "a".repeat(40), "aaaaaaa", "", "", "1700000000", " ", "Subject", ""].join("\0"),
      ["refs/remotes/origin/HEAD", "a".repeat(40), "aaaaaaa", "", "", "1700000000", " ", "Subject", ""].join("\0"),
      ["refs/tags/v1", "c".repeat(40), "ccccccc", "", "", "1500000000", " ", "Release", ""].join("\0"),
      "",
    ].join("\n");
    const refs = parseRefs(output);
    expect(refs.local.map((branch) => [branch.name, branch.current, branch.ahead, branch.behind, branch.upstreamGone, branch.worktreePath])).toEqual([
      ["main", true, 1, 2, false, "/repo"],
      ["feature", false, 0, 0, true, "/repo-feature"],
    ]);
    expect(refs.remote.map((branch) => branch.name)).toEqual(["origin/main"]);
    expect(refs.tags).toEqual([{ name: "v1", sha: "c".repeat(40), shortSha: "ccccccc", committerTime: 1500000000, subject: "Release" }]);
  });

  test("parses stashes", () => {
    const output = `stash@{0}\x1f${"a".repeat(40)}\x1f1700000000\x1fWIP on main: abc1234 Fix things\0stash@{1}\x1f${"b".repeat(40)}\x1f1600000000\x1fOn feature: my message\0`;
    expect(parseStashes(output)).toEqual([
      { ref: "stash@{0}", index: 0, sha: "a".repeat(40), time: 1700000000, message: "WIP on main: abc1234 Fix things", branch: "main", summary: "abc1234 Fix things" },
      { ref: "stash@{1}", index: 1, sha: "b".repeat(40), time: 1600000000, message: "On feature: my message", branch: "feature", summary: "my message" },
    ]);
  });

  test("validates branch names", () => {
    expect(branchNameProblem("feature/login")).toBeUndefined();
    expect(branchNameProblem("")).toBeDefined();
    expect(branchNameProblem("bad name")).toBeDefined();
    expect(branchNameProblem("a..b")).toBeDefined();
    expect(branchNameProblem("-x")).toBeDefined();
    expect(branchNameProblem("x.lock")).toBeDefined();
  });
});

describe("worktrees", () => {
  test("parses porcelain -z entries", () => {
    const output =
      "worktree /repo\0HEAD aaaa\0branch refs/heads/main\0\0" +
      "worktree /repo-feature\0HEAD bbbb\0branch refs/heads/feature\0locked working\0\0" +
      "worktree /repo-detached\0HEAD cccc\0detached\0prunable gitdir file points to non-existent location\0\0";
    const worktrees = parseWorktrees(output);
    expect(worktrees.map((worktree) => [worktree.path, worktree.branchName, worktree.main, worktree.locked, worktree.detached, worktree.prunable])).toEqual([
      ["/repo", "main", true, false, false, false],
      ["/repo-feature", "feature", false, true, false, false],
      ["/repo-detached", undefined, false, false, true, true],
    ]);
    expect(worktrees[1]!.lockReason).toBe("working");
    expect(worktreeDirectoryName("feature/login v2")).toBe("feature-login-v2");
  });
});
