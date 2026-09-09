import { afterEach, expect, test } from "bun:test";
import { mkdtemp, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { createEffect, createRoot, flush } from "solid-js";
import { GitRunner } from "../git/process.ts";
import { createPersistence, DEFAULT_STATE } from "./persistence.ts";
import { createStore, type Store } from "./store.ts";

const cleanup: (() => void | Promise<void>)[] = [];
afterEach(async () => {
  for (const dispose of cleanup.splice(0).reverse()) await dispose();
});
async function until(ready: () => boolean) {
  const deadline = Date.now() + 5000;
  while (Date.now() < deadline) {
    flush();
    if (ready()) return;
    await Bun.sleep(5);
  }
  throw new Error("Model did not settle");
}
async function fixture() {
  const root = await mkdtemp(join(tmpdir(), "quick-git-store-"));
  cleanup.push(() => rm(root, { recursive: true, force: true }));
  const runner = new GitRunner({
    concurrency: 2,
    env: { GIT_CONFIG_GLOBAL: "/dev/null", GIT_CONFIG_NOSYSTEM: "1" },
  });
  const git = (...args: string[]) => runner.text(args, { cwd: root }).then((value) => value.trim());
  const write = (path: string, value: string) => writeFile(join(root, path), value);
  await git("init", "-q", "-b", "main");
  await git("config", "user.name", "Quick Git Tests");
  await git("config", "user.email", "test@example.invalid");
  await git("config", "commit.gpgsign", "false");
  await git("config", "core.hooksPath", "/dev/null");
  await write("a.txt", "first\n");
  await write("z.txt", "first\n");
  await git("add", ".");
  await git("commit", "-qm", "Initial");
  await write("a.txt", "second\n");
  await write("z.txt", "second\n");
  await git("commit", "-qam", "Second");
  const makeStore = () => {
    const store = createStore({
      runner,
      persistence: createPersistence(undefined, DEFAULT_STATE),
      watch: () => () => {},
    });
    cleanup.push(() => store.dispose());
    return store;
  };
  const store = makeStore();
  expect(await store.openRepository(root)).toBe(true);
  flush();
  return { store, runner, git, write, root, makeStore };
}
function diffReady(store: Store) {
  return store.diff().files.length > 0 && !store.diff().loading;
}
function diffText(store: Store) {
  return store
    .diff()
    .files.flatMap((file) => file.hunks.flatMap((hunk) => hunk.lines.map((line) => line.text)))
    .join("\n");
}

test("focus refresh retains the commit, file, diff identity, and selected lines", async () => {
  const { store, git, write } = await fixture();
  store.setView("history");
  await until(() => store.history().commits.length === 2 && diffReady(store));
  store.selectCommitFile("z.txt");
  await until(() => store.diff().target?.path === "z.txt" && diffReady(store));
  store.setDiffSelection([[2, 2]]);
  flush();
  const selectedSHA = store.selectedCommit()!.sha;
  const detail = store.commitDetail();
  const diff = store.diff();
  let detailChanges = 0;
  let diffChanges = 0;
  cleanup.push(
    createRoot((dispose) => {
      createEffect(store.commitDetail, () => {
        detailChanges++;
      });
      createEffect(store.diff, () => {
        diffChanges++;
      });
      return dispose;
    }),
  );
  flush();
  async function refresh() {
    detailChanges = 0;
    diffChanges = 0;
    await store.refresh();
    await until(() => !store.history().loading && diffReady(store));
    expect(store.selectedCommit()!.sha).toBe(selectedSHA);
    expect(store.commitDetail()).toBe(detail);
    expect(store.diff()).toBe(diff);
    expect(store.diffSelection()).toEqual([[2, 2]]);
    expect({ detailChanges, diffChanges }).toEqual({ detailChanges: 0, diffChanges: 0 });
  }
  await refresh();
  await refresh();
  await write("a.txt", "third\n");
  await git("commit", "-qam", "Third");
  await refresh();
  expect(store.history().commits).toHaveLength(3);
  expect(store.historySelection()).toEqual([[1, 1]]);
  store.setHistorySelection([[0, 0]]);
  await until(
    () => store.diff().target?.sha === store.history().commits[0]!.sha && diffReady(store),
  );
  expect(diffText(store)).toContain("third");
});

test("working-tree refresh replaces stale content and preserves paths across reordered rows", async () => {
  const { store, write } = await fixture();
  await write("z.txt", "working tree\n");
  await store.refresh();
  await until(() => store.diff().target?.path === "z.txt" && diffReady(store));
  store.setSelection("unstaged", [[0, 0]]);
  flush();
  const previous = store.diff().files;
  await write("a.txt", "another change\n");
  await write("z.txt", "changed while unfocused\n");
  await store.refresh();
  await until(() => store.diff().files !== previous && !store.diff().loading);
  expect(store.selectedItems("unstaged").map((item) => item.path)).toEqual(["z.txt"]);
  expect(store.diff().target?.path).toBe("z.txt");
  expect(diffText(store)).toContain("changed while unfocused");
});

test("all-branches changes apply to the immediate history request", async () => {
  const { store, git, write } = await fixture();
  await git("switch", "-qc", "feature");
  await write("feature.txt", "feature\n");
  await git("add", ".");
  await git("commit", "-qm", "Feature only");
  await git("switch", "-q", "main");
  await store.refresh();
  store.setHistoryAllBranches(true);
  await until(() => !store.history().loading && store.history().commits.length === 3);
  expect(store.history().commits.some((commit) => commit.subject === "Feature only")).toBe(true);
  store.setHistoryAllBranches(false);
  await until(() => !store.history().loading && store.history().commits.length === 2);
});

test("a successful commit clears the composer and refreshes the repository", async () => {
  const { store, write } = await fixture();
  await write("a.txt", "third\n");
  await store.refresh();
  await store.stageAll();
  store.setSubject("Third");
  store.setBody("New body");
  flush();
  expect(store.canCommit()).toBe(true);
  await store.commit();
  flush();
  expect(store.subject()).toBe("");
  expect(store.body()).toBe("");
  expect(store.staged()).toHaveLength(0);
  expect(store.history().commits[0]!.subject).toBe("Third");
});

test("closing during repository loading cancels jobs and cannot repopulate a closed store", async () => {
  const { store, runner, root } = await fixture();
  const opened = store.openRepository(root);
  store.closeRepository();
  expect(await opened).toBe(false);
  await until(() => runner.pending === 0);
  expect(store.repository()).toBeUndefined();
  expect(store.commitDetail().files).toEqual([]);
  const pending = store.openRepository(root);
  store.dispose();
  expect(await pending).toBe(false);
  await until(() => runner.pending === 0);
});

test("repository stores opened from another root have independent lifetimes", async () => {
  const { root, makeStore } = await fixture();
  let other!: Store;
  const disposeSource = createRoot((dispose) => {
    other = makeStore();
    return dispose;
  });
  expect(await other.openRepository(root)).toBe(true);
  disposeSource();
  other.setView("history");
  await until(() => diffReady(other));
  expect(other.history().commits).toHaveLength(2);
});
