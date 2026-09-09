import { describe, expect, test } from "bun:test";

import { classifyGitPath, classifyWorktreePath } from "./watcher.ts";
import { parsePersistedState, rememberRepository } from "./persistence.ts";

describe("watcher classification", () => {
  test("separates working-tree edits from ref and index changes and ignores objects", () => {
    expect(classifyWorktreePath("src/app.ts", ".git")).toBe("worktree");
    expect(classifyWorktreePath(".git/index", ".git")).toBe("index");
    expect(classifyWorktreePath(".git/HEAD", ".git")).toBe("refs");
    expect(classifyWorktreePath(".git/refs/heads/main", ".git")).toBe("refs");
    expect(classifyWorktreePath(".git/objects/ab/cdef", ".git")).toBeUndefined();
    expect(classifyWorktreePath(".gitignore", ".git")).toBe("worktree");
    expect(classifyGitPath("worktrees/feature/HEAD")).toBe("refs");
    expect(classifyGitPath("COMMIT_EDITMSG")).toBeUndefined();
  });

  test("handles a linked worktree whose git directory lives elsewhere", () => {
    expect(classifyWorktreePath(".git", "../main/.git/worktrees/feature")).toBe("refs");
    expect(classifyWorktreePath("notes.md", "../main/.git/worktrees/feature")).toBe("worktree");
  });
});

describe("persisted state", () => {
  test("rejects unknown versions and clamps numbers", () => {
    expect(parsePersistedState({ version: 99 }).recentRepositories).toEqual([]);
    const state = parsePersistedState({
      version: 1,
      recentRepositories: ["/a", "/a", "", 3, "/b"],
      sidebarWidth: 5,
      changesSplit: 9999,
      preferredAgent: "claude",
      historyAllBranches: true,
    });
    expect(state.recentRepositories).toEqual(["/a", "/b"]);
    expect(state.sidebarWidth).toBe(180);
    expect(state.changesSplit).toBe(800);
    expect(state.preferredAgent).toBe("claude");
    expect(state.historyAllBranches).toBe(true);
  });

  test("moves a repository to the front of the recent list", () => {
    expect(rememberRepository(["/a", "/b"], "/b")).toEqual(["/b", "/a"]);
    expect(rememberRepository([], "/c")).toEqual(["/c"]);
  });
});
