import { describe, expect, test } from "bun:test";
import { mkdtemp, readFile, realpath, rm, symlink } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";

import { DEFAULT_STATE, createPersistence, parsePersistedState, rememberRepository } from "./persistence.ts";

describe("persistence", () => {
  test("merges updates into one shared state and writes it once after a pause", async () => {
    const dir = await mkdtemp(join(tmpdir(), "quick-git-state-"));
    const path = join(dir, "state.json");
    try {
      const persistence = createPersistence(path, DEFAULT_STATE, 10);
      persistence.update({ sidebarWidth: 300 });
      persistence.update({ recentRepositories: ["/a"] });
      expect(persistence.current()).toMatchObject({ sidebarWidth: 300, recentRepositories: ["/a"] });
      await new Promise((resolve) => setTimeout(resolve, 80));
      expect(persistence.state()).toMatchObject({ sidebarWidth: 300, recentRepositories: ["/a"] });
      expect(parsePersistedState(JSON.parse(await readFile(path, "utf8")))).toMatchObject({
        sidebarWidth: 300,
        recentRepositories: ["/a"],
      });
      persistence.update({ historySplit: 500 });
      await persistence.flush();
      expect(parsePersistedState(JSON.parse(await readFile(path, "utf8")))).toMatchObject({ historySplit: 500 });
    } finally {
      await rm(dir, { recursive: true, force: true });
    }
  });

  test("keeps the fields a patch leaves out and forgets the last repository on null", async () => {
    const persistence = createPersistence(undefined, { ...DEFAULT_STATE, lastRepository: "/a", preferredAgent: "codex" });
    persistence.update({ sidebarWidth: 300 });
    expect(persistence.current()).toEqual({ ...DEFAULT_STATE, lastRepository: "/a", preferredAgent: "codex", sidebarWidth: 300 });
    persistence.update({ lastRepository: null });
    expect(persistence.current().lastRepository).toBeUndefined();
    expect(JSON.stringify(persistence.current())).not.toContain("lastRepository");
    persistence.update({ lastRepository: "/b" });
    expect(persistence.state()).toMatchObject({ lastRepository: "/b", sidebarWidth: 300 });
  });

  test("keeps state in memory without a file", async () => {
    const persistence = createPersistence(undefined, DEFAULT_STATE);
    persistence.update({ sidebarWidth: 250 });
    await persistence.flush();
    expect(persistence.current().sidebarWidth).toBe(250);
  });

  test("counts two spellings of one folder once when loading", async () => {
    const dir = await mkdtemp(join(tmpdir(), "quick-git-recent-"));
    try {
      const link = join(dir, "link");
      await symlink(dir, link);
      const state = parsePersistedState({ version: 1, recentRepositories: [link, dir, "/elsewhere"] });
      expect(state.recentRepositories).toEqual([await realpath(dir), "/elsewhere"]);
    } finally {
      await rm(dir, { recursive: true, force: true });
    }
  });

  test("moves a reopened repository to the front of the recent list", () => {
    expect(rememberRepository(["/a", "/b"], "/b")).toEqual(["/b", "/a"]);
    expect(rememberRepository([], "/c")).toEqual(["/c"]);
  });
});
