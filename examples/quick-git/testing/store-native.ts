// Run by store-native.test.ts as a compiled app: Bun does not enforce scriptc's array bounds.
import { app } from "@quickgui/native";

import { layoutGraph, type Commit } from "../git/log.ts";
import { GitRunner } from "../git/process.ts";
import { createPersistence, DEFAULT_STATE, type PersistedState } from "../model/persistence.ts";
import { createStore } from "../model/store.ts";

await app.whenReady();

// A patch leaves most fields out; under scriptc a `{ ...current, ...patch }` merge threw for them.
const initial: PersistedState = { ...DEFAULT_STATE, recentRepositories: ["/a"], lastRepository: "/a", preferredAgent: "codex" };
const persistence = createPersistence(undefined, initial);
persistence.update({ sidebarWidth: 300 });
const patched = persistence.current();
if (patched.sidebarWidth !== 300 || patched.lastRepository !== "/a" || patched.preferredAgent !== "codex" || patched.recentRepositories.length !== 1) {
  throw new Error(`A patch must keep the fields it leaves out: ${JSON.stringify(patched)}`);
}
persistence.update({ lastRepository: null });
if (persistence.current().lastRepository !== undefined || persistence.current().sidebarWidth !== 300) {
  throw new Error(`lastRepository: null must forget only the last repository: ${JSON.stringify(persistence.current())}`);
}
console.log("persistence=ok");

const store = createStore({
  runner: new GitRunner({ concurrency: 4 }),
  persistence,
});
store.setSidebarWidth(320);
if (persistence.current().sidebarWidth !== 320 || persistence.current().recentRepositories.length !== 1) {
  throw new Error("The store's persist must merge into the shared state");
}
if (store.activeItem() !== undefined || store.selectedCommit() !== undefined || store.changeCount() !== 0) {
  throw new Error("An empty store must have no selected change or commit");
}
store.setHistorySelection([[0, 0]]);
if (store.selectedCommit() !== undefined) throw new Error("A stale history selection must be empty");
store.setHistorySelection([]);

const rootCommit: Commit = {
  sha: "root", shortSha: "root", parents: [], authorName: "Test", authorEmail: "test@example.com",
  authorTime: 0, committerName: "Test", committerEmail: "test@example.com", committerTime: 0,
  refs: [], subject: "Initial commit", body: "",
};
const graph = layoutGraph([rootCommit]);
if (graph.length !== 1 || graph[0]!.edges.length !== 0) throw new Error("A root commit must have no parent edges");

console.log("empty-store=ok");
store.dispose();
await app.exit(0);
