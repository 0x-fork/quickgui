/**
 * Application state and every action the UI can take.
 *
 * The store is created once, outside any component, so its signals can be written from git
 * completions, watcher callbacks, and menu handlers. Views read signals and call actions; they
 * never run git themselves.
 */

import { basename, join, resolve } from "node:path";

import { createEffect, createMemo, createRoot, createSignal, untrack } from "solid-js";

/** Solid 2 batches synchronous writes on its own; this only groups them for readability. */
const batch = <T,>(work: () => T): T => work();

import {
  AGENT_TIMEOUT_MS,
  AgentError,
  detectAgents,
  generateCommitMessage,
  type AgentId,
  type AvailableAgent,
  type GeneratedMessage,
} from "../agent/commit-message.ts";
import type { Diff, DiffRow, HunkSelection } from "../git/diff.ts";
import { layoutGraph, type Commit, type GraphRow } from "../git/log.ts";
import { GitError, GitRunner } from "../git/process.ts";
import { Repository, type CommitFile, type NumstatEntry } from "../git/repository.ts";
import type { RefCollections, StashEntry } from "../git/refs.ts";
import { stagedChanges, unstagedChanges, type ChangeItem, type RepositoryStatus } from "../git/status.ts";
import type { Worktree } from "../git/worktree.ts";
import { canonicalPath } from "./paths.ts";
import { DEFAULT_STATE, rememberRepository, type Persistence, type PersistedState } from "./persistence.ts";
import { watchRepository, type ChangeKind } from "./watcher.ts";

export type ViewId = "changes" | "history" | "branches" | "worktrees" | "stashes";
export type ListId = "unstaged" | "staged";
export type RowRanges = readonly (readonly number[])[];

export interface Notice {
  type: "info" | "success" | "error" | "warning";
  title: string;
  description?: string;
  timeout?: number;
}

export interface DiffTarget {
  key: string;
  kind: "unstaged" | "staged" | "commit";
  path: string;
  originalPath?: string;
  untracked: boolean;
  sha?: string;
}

export interface DiffState {
  target?: DiffTarget;
  /** The parsed diff, held by the native module; the table fetches rows as it paints them. */
  diff?: Diff;
  loading: boolean;
  error?: string;
}

export type { DiffRow } from "../git/diff.ts";

export interface HistoryState {
  commits: Commit[];
  graph: GraphRow[];
  loading: boolean;
  exhausted: boolean;
  allBranches: boolean;
}

export interface CommitDetailState {
  sha?: string;
  files: CommitFile[];
  loading: boolean;
  selectedPath?: string;
}

export interface BusyState {
  label: string;
  cancel?: () => void;
}

export interface GenerationState {
  agent: AgentId;
  cancel: () => void;
}

export interface StoreOptions {
  runner: GitRunner;
  /** Shared with every other window: the one writer of the state file. */
  persistence: Persistence;
  /** Sends an untracked file to the Trash when discarding it. */
  trash?: (absolutePath: string) => Promise<void>;
}

const HISTORY_PAGE = 300;
/** Parsed diffs kept for quick switching, bounded by count and by the bytes they came from. */
const MAX_DIFF_CACHE_ENTRIES = 64;
const MAX_DIFF_CACHE_BYTES = 64 * 1024 * 1024;

export function createStore(options: StoreOptions) {
  return createRoot((disposeRoot) => {
    const store = buildStore(options);
    return {
      ...store,
      /** Stop watching, cancel work, and release every signal; for a window that closed. */
      dispose: () => {
        store.dispose();
        disposeRoot();
      },
    };
  });
}

export type Store = ReturnType<typeof createStore>;

function buildStore(options: StoreOptions) {
  const runner = options.runner;
  let notifier: (notice: Notice) => void = (notice) => console.log(`[${notice.type}] ${notice.title}`);
  // A snapshot seeds this window's own signals; the recent list stays live through the shared state.
  const persisted = options.persistence.current();
  let stopWatching: (() => void) | undefined;

  // --- repository ------------------------------------------------------------------------------
  const [repository, setRepository] = createSignal<Repository>();
  const [mainRepository, setMainRepository] = createSignal<Repository>();
  const [opening, setOpening] = createSignal<string>();
  const [worktrees, setWorktrees] = createSignal<Worktree[]>([]);
  const [status, setStatus] = createSignal<RepositoryStatus>();
  const [numstat, setNumstat] = createSignal<{ unstaged: Map<string, NumstatEntry>; staged: Map<string, NumstatEntry> }>({
    unstaged: new Map(),
    staged: new Map(),
  });
  const [generation, setGeneration] = createSignal(0);
  const [refs, setRefs] = createSignal<RefCollections>({ local: [], remote: [], tags: [] });
  const [stashes, setStashes] = createSignal<StashEntry[]>([]);
  const [remotes, setRemotes] = createSignal<string[]>([]);
  const recentRepositories = () => options.persistence.state().recentRepositories;
  const [busy, setBusy] = createSignal<BusyState>();
  const [view, setViewSignal] = createSignal<ViewId>("changes");
  const [sidebarWidth, setSidebarWidthSignal] = createSignal(persisted.sidebarWidth);
  const [changesSplit, setChangesSplitSignal] = createSignal(persisted.changesSplit);
  const [historySplit, setHistorySplitSignal] = createSignal(persisted.historySplit);
  const [hasHead, setHasHead] = createSignal(true);
  const [identity, setIdentity] = createSignal<{ name?: string; email?: string }>({});

  const repositoryName = createMemo(() => {
    const current = repository();
    return current ? basename(current.root) : "";
  });
  const unstaged = createMemo(() => (status() ? unstagedChanges(status()!) : []));
  const staged = createMemo(() => (status() ? stagedChanges(status()!) : []));
  const conflicts = createMemo(() => unstaged().filter((item) => item.conflicted).length);
  const changeCount = createMemo(() => unstaged().length + staged().length);

  // --- selection in the change lists -----------------------------------------------------------
  const [selection, setSelection] = createSignal<{ unstaged: RowRanges; staged: RowRanges }>({
    unstaged: [],
    staged: [],
  });
  const [activeCell, setActiveCell] = createSignal<{ list: ListId; row: number }>();
  const [focusedList, setFocusedList] = createSignal<ListId>("unstaged");

  const selectedItems = (list: ListId): ChangeItem[] => {
    const items = list === "unstaged" ? unstaged() : staged();
    const rows = new Set<number>();
    for (const [start, end] of selection()[list]) {
      for (let row = start!; row <= end!; row += 1) rows.add(row);
    }
    return items.filter((_, index) => rows.has(index));
  };

  const activeItem = createMemo<ChangeItem | undefined>(() => {
    // The focused list's selection wins; the active cell refines it to the row the user last
    // moved to. A table reports an active cell as soon as it mounts, so on its own it means
    // nothing until a selection exists.
    const lists: ListId[] = focusedList() === "staged" ? ["staged", "unstaged"] : ["unstaged", "staged"];
    for (const list of lists) {
      const selected = selectedItems(list);
      if (selected.length === 0) continue;
      const cell = activeCell();
      const items = list === "unstaged" ? unstaged() : staged();
      if (cell && cell.list === list && items[cell.row] && selected.includes(items[cell.row]!)) return items[cell.row];
      return selected[0];
    }
    // Nothing chosen yet: show the first change so the diff pane is never empty needlessly.
    return unstaged()[0] ?? staged()[0];
  });

  // --- diff ------------------------------------------------------------------------------------
  const [diff, setDiff] = createSignal<DiffState>({ loading: false });
  const [diffSelection, setDiffSelection] = createSignal<RowRanges>([]);
  const diffCache = new Map<string, Diff>();
  let diffCacheBytes = 0;
  let diffController: AbortController | undefined;

  /** Remember a diff as the most recent one, releasing the oldest while the cache is over budget. */
  function cacheDiff(key: string, parsed: Diff): void {
    const previous = diffCache.get(key);
    if (previous) {
      diffCache.delete(key);
      diffCacheBytes -= previous.bytes;
      if (previous !== parsed) previous.close();
    }
    diffCache.set(key, parsed);
    diffCacheBytes += parsed.bytes;
    for (const [oldKey, old] of diffCache) {
      if (diffCache.size <= MAX_DIFF_CACHE_ENTRIES && diffCacheBytes <= MAX_DIFF_CACHE_BYTES) break;
      if (oldKey === key) break;
      diffCache.delete(oldKey);
      diffCacheBytes -= old.bytes;
      old.close();
    }
  }

  function clearDiffCache(): void {
    for (const parsed of diffCache.values()) parsed.close();
    diffCache.clear();
    diffCacheBytes = 0;
  }

  const diffTarget = createMemo<DiffTarget | undefined>(() => {
    const item = activeItem();
    if (!item) return undefined;
    const kind: DiffTarget["kind"] = item.staged ? "staged" : "unstaged";
    return {
      key: `${kind}:${item.path}:${generation()}`,
      kind,
      path: item.path,
      ...(item.originalPath ? { originalPath: item.originalPath } : {}),
      untracked: item.entry.kind === "untracked",
    };
  });

  const diffRowCount = () => diff().diff?.rowCount ?? 0;
  /** The table rows in `[start, end)`, fetched from the native module for the visible range. */
  function diffRowsIn(start: number, end: number): DiffRow[] {
    return diff().diff?.rows(start, end) ?? [];
  }

  const diffStats = createMemo(() => {
    const parsed = diff().diff;
    return { added: parsed?.added ?? 0, removed: parsed?.removed ?? 0 };
  });

  /** The changed lines inside the diff table's selected rows, grouped by hunk. */
  const selectedDiffLines = createMemo(() => diff().diff?.selectedLines(diffSelection()) ?? []);
  const selectedDiffLineCount = createMemo(() => selectedDiffLines().reduce((total, entry) => total + entry.lines.size, 0));

  async function loadDiff(target: DiffTarget | undefined): Promise<void> {
    diffController?.abort();
    diffController = undefined;
    if (!target) {
      setDiff({ loading: false });
      return;
    }
    const cached = diffCache.get(target.key);
    if (cached) {
      cacheDiff(target.key, cached);
      setDiff({ target, diff: cached, loading: false });
      return;
    }
    const repo = repository();
    if (!repo) return;
    const controller = new AbortController();
    diffController = controller;
    setDiff((current) => ({
      target,
      ...(current.target?.path === target.path && current.diff ? { diff: current.diff } : {}),
      loading: true,
    }));
    try {
      let parsed: Diff;
      if (target.kind === "commit") parsed = await repo.diffCommit(target.sha!, target.path, { signal: controller.signal });
      else if (target.untracked) parsed = await repo.diffUntracked(target.path, { signal: controller.signal });
      else if (target.kind === "staged") parsed = await repo.diffIndex(target.path, target.originalPath, { signal: controller.signal });
      else parsed = await repo.diffWorkingTree(target.path, { signal: controller.signal });
      if (controller.signal.aborted) {
        parsed.close();
        return;
      }
      cacheDiff(target.key, parsed);
      setDiff({ target, diff: parsed, loading: false });
      setDiffSelection([]);
    } catch (error) {
      if (controller.signal.aborted) return;
      setDiff({ target, loading: false, error: describeError(error) });
    }
  }

  // --- commit composer -------------------------------------------------------------------------
  const [subject, setSubject] = createSignal("");
  const [body, setBody] = createSignal("");
  const [amend, setAmendSignal] = createSignal(false);
  const [committing, setCommitting] = createSignal(false);
  const [headMessage, setHeadMessage] = createSignal<{ subject: string; body: string }>();
  const [agents, setAgents] = createSignal<AvailableAgent[]>([]);
  const [preferredAgent, setPreferredAgentSignal] = createSignal<AgentId | undefined>(persisted.preferredAgent);
  const [generating, setGenerating] = createSignal<GenerationState>();
  const [lastGenerated, setLastGenerated] = createSignal<GeneratedMessage>();

  const canCommit = createMemo(() => {
    if (committing() || !repository()) return false;
    if (subject().trim().length === 0) return false;
    if (conflicts() > 0 && staged().length === 0) return false;
    return staged().length > 0 || amend();
  });

  void detectAgents().then((found) => {
    setAgents(found);
    if (!found.some((agent) => agent.id === untrack(preferredAgent)) && found[0]) {
      setPreferredAgentSignal(found[0].id);
    }
  });

  // --- history ---------------------------------------------------------------------------------
  const [history, setHistory] = createSignal<HistoryState>({
    commits: [],
    graph: [],
    loading: false,
    exhausted: false,
    allBranches: persisted.historyAllBranches,
  });
  const [historySelection, setHistorySelection] = createSignal<RowRanges>([]);
  const [commitDetail, setCommitDetail] = createSignal<CommitDetailState>({ files: [], loading: false });
  let historyController: AbortController | undefined;
  let detailController: AbortController | undefined;

  const selectedCommit = createMemo<Commit | undefined>(() => {
    const [range] = historySelection();
    const row = range?.[0];
    return row === undefined ? undefined : history().commits[row];
  });

  const commitDiffTarget = createMemo<DiffTarget | undefined>(() => {
    const detail = commitDetail();
    const commit = selectedCommit();
    if (!commit || detail.sha !== commit.sha) return undefined;
    const path = detail.selectedPath ?? detail.files[0]?.path;
    if (!path) return undefined;
    const file = detail.files.find((entry) => entry.path === path);
    return {
      key: `commit:${commit.sha}:${path}`,
      kind: "commit",
      path,
      ...(file?.originalPath ? { originalPath: file.originalPath } : {}),
      untracked: false,
      sha: commit.sha,
    };
  });

  createEffect(
    () => selectedCommit()?.sha,
    (sha) => {
      queueMicrotask(() => void loadCommitDetail(sha));
    },
  );

  createEffect(
    () => (view() === "changes" ? diffTarget() : commitDiffTarget()),
    (target) => {
      queueMicrotask(() => void loadDiff(target));
    },
  );

  async function loadCommitDetail(sha: string | undefined): Promise<void> {
    detailController?.abort();
    detailController = undefined;
    const repo = repository();
    if (!sha || !repo) {
      setCommitDetail({ files: [], loading: false });
      return;
    }
    const controller = new AbortController();
    detailController = controller;
    setCommitDetail({ sha, files: [], loading: true });
    try {
      const files = await repo.commitFiles(sha, { signal: controller.signal });
      if (controller.signal.aborted) return;
      setCommitDetail({ sha, files, loading: false });
    } catch (error) {
      if (controller.signal.aborted) return;
      setCommitDetail({ sha, files: [], loading: false });
      notify({ type: "error", title: "Unable to read commit", description: describeError(error) });
    }
  }

  async function loadHistory(reset: boolean): Promise<void> {
    const repo = repository();
    if (!repo) return;
    const current = untrack(history);
    if (!reset && (current.loading || current.exhausted)) return;
    historyController?.abort();
    const controller = new AbortController();
    historyController = controller;
    setHistory((state) => ({ ...state, loading: true, ...(reset ? { exhausted: false } : {}) }));
    try {
      const skip = reset ? 0 : current.commits.length;
      const page = await repo.log({
        limit: HISTORY_PAGE,
        skip,
        all: current.allBranches,
        signal: controller.signal,
      });
      if (controller.signal.aborted) return;
      const commits = reset ? page : [...current.commits, ...page];
      setHistory((state) => ({
        ...state,
        commits,
        graph: layoutGraph(commits),
        loading: false,
        exhausted: page.length < HISTORY_PAGE,
      }));
      if (reset) {
        const previous = untrack(selectedCommit)?.sha;
        const index = previous ? commits.findIndex((commit) => commit.sha === previous) : -1;
        setHistorySelection(index >= 0 ? [[index, index]] : commits.length > 0 ? [[0, 0]] : []);
      }
    } catch (error) {
      if (controller.signal.aborted) return;
      setHistory((state) => ({ ...state, loading: false }));
      notify({ type: "error", title: "Unable to load history", description: describeError(error) });
    }
  }

  // --- refresh ---------------------------------------------------------------------------------
  let refreshing = false;
  let refreshPending: Set<ChangeKind> | undefined;

  async function refresh(kinds: ReadonlySet<ChangeKind> = new Set(["worktree", "index", "refs"])): Promise<void> {
    if (refreshing) {
      refreshPending = new Set([...(refreshPending ?? []), ...kinds]);
      return;
    }
    refreshing = true;
    try {
      const repo = repository();
      if (!repo) return;
      const wantsRefs = kinds.has("refs");
      const [nextStatus, unstagedStats, stagedStats, nextRefs, nextStashes, nextWorktrees, nextRemotes, head] =
        await Promise.all([
          repo.status(),
          repo.numstat(false),
          repo.numstat(true),
          wantsRefs ? repo.refs() : undefined,
          wantsRefs ? repo.stashes() : undefined,
          wantsRefs ? repo.worktrees() : undefined,
          wantsRefs ? repo.remotes() : undefined,
          repo.hasHead(),
        ]);
      if (repository() !== repo) return;
      batch(() => {
        setStatus(nextStatus);
        setNumstat({
          unstaged: new Map(unstagedStats.map((entry) => [entry.path, entry])),
          staged: new Map(stagedStats.map((entry) => [entry.path, entry])),
        });
        setHasHead(head);
        if (nextRefs) setRefs(nextRefs);
        if (nextStashes) setStashes(nextStashes);
        if (nextWorktrees) setWorktrees(nextWorktrees);
        if (nextRemotes) setRemotes(nextRemotes);
        clearDiffCache();
        setGeneration((value) => value + 1);
        reconcileSelection(nextStatus);
      });
      if (wantsRefs) {
        void loadHistory(true);
        void loadHeadMessage(repo);
      }
    } catch (error) {
      if (!(error instanceof GitError && error.aborted)) {
        notify({ type: "error", title: "Unable to refresh", description: describeError(error) });
      }
    } finally {
      refreshing = false;
      if (refreshPending) {
        const pending = refreshPending;
        refreshPending = undefined;
        void refresh(pending);
      }
    }
  }

  /** Keep selection on the same paths after the lists change under it. */
  function reconcileSelection(next: RepositoryStatus): void {
    const previousUnstaged = selectedItems("unstaged").map((item) => item.path);
    const previousStaged = selectedItems("staged").map((item) => item.path);
    const nextUnstaged = unstagedChanges(next);
    const nextStaged = stagedChanges(next);
    setSelection({
      unstaged: rangesFor(nextUnstaged, previousUnstaged),
      staged: rangesFor(nextStaged, previousStaged),
    });
    const cell = activeCell();
    if (cell) {
      const items = cell.list === "unstaged" ? nextUnstaged : nextStaged;
      if (!items[cell.row]) setActiveCell(items.length > 0 ? { list: cell.list, row: Math.min(cell.row, items.length - 1) } : undefined);
    }
  }

  async function loadHeadMessage(repo: Repository): Promise<void> {
    try {
      const [head] = await repo.log({ limit: 1 });
      setHeadMessage(head ? { subject: head.subject, body: head.body } : undefined);
    } catch {
      setHeadMessage(undefined);
    }
  }

  // --- opening ---------------------------------------------------------------------------------
  async function openRepository(path: string): Promise<boolean> {
    const target = canonicalPath(path);
    setOpening(target);
    try {
      const repo = await Repository.open(runner, target, options.trash ? { trash: options.trash } : {});
      await activate(repo, repo);
      persist({
        lastRepository: repo.root,
        recentRepositories: rememberRepository(options.persistence.current().recentRepositories, repo.root),
      });
      return true;
    } catch (error) {
      notify({ type: "error", title: `Unable to open ${basename(target)}`, description: describeError(error) });
      persist({ recentRepositories: options.persistence.current().recentRepositories.filter((entry) => entry !== target) });
      return false;
    } finally {
      setOpening(undefined);
    }
  }

  async function activate(repo: Repository, main: Repository): Promise<void> {
    stopWatching?.();
    diffController?.abort();
    historyController?.abort();
    batch(() => {
      setRepository(repo);
      setMainRepository(main);
      setStatus(undefined);
      setSelection({ unstaged: [], staged: [] });
      setActiveCell(undefined);
      setDiff({ loading: false });
      setDiffSelection([]);
      setHistory((state) => ({ ...state, commits: [], graph: [], loading: false, exhausted: false }));
      setHistorySelection([]);
      setCommitDetail({ files: [], loading: false });
      setSubject("");
      setBody("");
      setAmendSignal(false);
    });
    clearDiffCache();
    stopWatching = watchRepository({
      root: repo.root,
      gitDir: repo.info.gitDir,
      commonDir: repo.info.commonDir,
      onChange: (kinds) => void refresh(kinds),
    });
    void Promise.all([repo.config("user.name"), repo.config("user.email")]).then(([name, email]) =>
      setIdentity({ ...(name ? { name } : {}), ...(email ? { email } : {}) }),
    );
    await refresh();
  }

  function closeRepository(): void {
    stopWatching?.();
    stopWatching = undefined;
    diffController?.abort();
    historyController?.abort();
    batch(() => {
      setRepository(undefined);
      setMainRepository(undefined);
      setStatus(undefined);
      setWorktrees([]);
      setRefs({ local: [], remote: [], tags: [] });
      setStashes([]);
      setDiff({ loading: false });
      setHistory((state) => ({ ...state, commits: [], graph: [] }));
      setHistorySelection([]);
      setView("changes");
    });
    persist({ lastRepository: undefined });
  }

  async function selectWorktree(path: string): Promise<void> {
    const main = mainRepository();
    if (!main) return;
    if (repository()?.root === path) return;
    try {
      const repo = path === main.root ? main : await main.worktree(path);
      await activate(repo, main);
      setView("changes");
    } catch (error) {
      notify({ type: "error", title: "Unable to open worktree", description: describeError(error) });
    }
  }

  // --- operations ------------------------------------------------------------------------------
  async function operation<T>(
    label: string,
    run: (repo: Repository, signal: AbortSignal) => Promise<T>,
    after: { refs?: boolean; success?: Notice | ((result: T) => Notice | undefined) } = {},
  ): Promise<T | undefined> {
    const repo = repository();
    if (!repo) return undefined;
    const controller = new AbortController();
    setBusy({ label, cancel: () => controller.abort() });
    try {
      const result = await run(repo, controller.signal);
      const success = typeof after.success === "function" ? after.success(result) : after.success;
      if (success) notify(success);
      return result;
    } catch (error) {
      if (error instanceof GitError && error.aborted) {
        notify({ type: "info", title: `${label} cancelled` });
      } else {
        notify({ type: "error", title: `${label} failed`, description: describeError(error), timeout: 9000 });
      }
      return undefined;
    } finally {
      setBusy(undefined);
      void refresh(new Set<ChangeKind>(after.refs === false ? ["worktree", "index"] : ["worktree", "index", "refs"]));
    }
  }

  async function stageItems(items: readonly ChangeItem[]): Promise<void> {
    if (items.length === 0) return;
    await operation("Stage", (repo, signal) => repo.stage(items.map((item) => item.path), { signal }), { refs: false });
  }

  async function unstageItems(items: readonly ChangeItem[]): Promise<void> {
    if (items.length === 0) return;
    const paths = items.flatMap((item) => (item.originalPath ? [item.originalPath, item.path] : [item.path]));
    await operation("Unstage", (repo, signal) => repo.unstage(paths, { signal }), { refs: false });
  }

  async function discardItems(items: readonly ChangeItem[]): Promise<void> {
    if (items.length === 0) return;
    const tracked = items.filter((item) => item.entry.kind !== "untracked").map((item) => item.path);
    const untracked = items.filter((item) => item.entry.kind === "untracked").map((item) => item.path);
    await operation("Discard", (repo, signal) => repo.discard(tracked, untracked, { signal }), {
      refs: false,
      success: { type: "info", title: items.length === 1 ? `Discarded ${basename(items[0]!.path)}` : `Discarded ${items.length} files` },
    });
  }

  function toggleStaging(list: ListId): Promise<void> {
    const items = selectedItems(list);
    const cell = activeCell();
    const fallback = cell && cell.list === list ? (list === "unstaged" ? unstaged() : staged())[cell.row] : undefined;
    const targets = items.length > 0 ? items : fallback ? [fallback] : [];
    return list === "unstaged" ? stageItems(targets) : unstageItems(targets);
  }

  async function applyHunkSelection(action: "stage" | "unstage" | "discard", selections: readonly HunkSelection[], fileIndex: number): Promise<void> {
    const file = diff().diff?.file(fileIndex);
    if (!file || selections.length === 0) return;
    const label = action === "stage" ? "Stage lines" : action === "unstage" ? "Unstage lines" : "Discard lines";
    await operation(
      label,
      (repo, signal) =>
        action === "stage"
          ? repo.stagePatch(file, selections, { signal })
          : action === "unstage"
            ? repo.unstagePatch(file, selections, { signal })
            : repo.discardPatch(file, selections, { signal }),
      { refs: false },
    );
  }

  function stageHunk(fileIndex: number, hunkIndex: number): Promise<void> {
    return applyHunkSelection("stage", [{ hunkIndex }], fileIndex);
  }
  function unstageHunk(fileIndex: number, hunkIndex: number): Promise<void> {
    return applyHunkSelection("unstage", [{ hunkIndex }], fileIndex);
  }
  function discardHunk(fileIndex: number, hunkIndex: number): Promise<void> {
    return applyHunkSelection("discard", [{ hunkIndex }], fileIndex);
  }

  async function applySelectedLines(action: "stage" | "unstage" | "discard"): Promise<void> {
    const groups = selectedDiffLines();
    if (groups.length === 0) return;
    // One patch per file; the diff pane shows one file at a time except for renames.
    const byFile = new Map<number, HunkSelection[]>();
    for (const group of groups) {
      const list = byFile.get(group.fileIndex) ?? [];
      list.push({ hunkIndex: group.hunkIndex, lines: group.lines });
      byFile.set(group.fileIndex, list);
    }
    for (const [fileIndex, selections] of byFile) {
      await applyHunkSelection(action, selections.sort((left, right) => left.hunkIndex - right.hunkIndex), fileIndex);
    }
    setDiffSelection([]);
  }

  async function commit(): Promise<void> {
    if (!canCommit()) return;
    const message = `${subject().trim()}\n\n${body().trim()}`.trim();
    const amending = amend();
    setCommitting(true);
    try {
      const done = await operation(
        amending ? "Amend" : "Commit",
        (repo, signal) => repo.commit(message, { amend: amending, signal }),
        { success: { type: "success", title: amending ? "Commit amended" : `Committed “${truncate(subject().trim(), 60)}”` } },
      );
      if (done !== undefined) {
        batch(() => {
          setSubject("");
          setBody("");
          setAmendSignal(false);
        });
      }
    } finally {
      setCommitting(false);
    }
  }

  function setAmend(value: boolean): void {
    setAmendSignal(value);
    const head = headMessage();
    if (value && head && subject().trim() === "" && body().trim() === "") {
      setSubject(head.subject);
      setBody(head.body);
    }
  }

  async function generateMessage(agentId?: AgentId): Promise<void> {
    const repo = repository();
    if (!repo) return;
    const agent = agents().find((candidate) => candidate.id === (agentId ?? preferredAgent())) ?? agents()[0];
    if (!agent) {
      notify({
        type: "warning",
        title: "No agent CLI found",
        description: "Install Codex (codex) or Claude Code (claude) to generate commit messages.",
      });
      return;
    }
    generating()?.cancel();
    const controller = new AbortController();
    setGenerating({ agent: agent.id, cancel: () => controller.abort() });
    setPreferredAgent(agent.id);
    try {
      const useStaged = staged().length > 0;
      const items = useStaged ? staged() : unstaged();
      const diffs = await Promise.all(
        items.slice(0, 200).map(async (item) => {
          const parsed =
            item.entry.kind === "untracked"
              ? await repo.diffUntracked(item.path, { signal: controller.signal })
              : useStaged
                ? await repo.diffIndex(item.path, item.originalPath, { signal: controller.signal })
                : await repo.diffWorkingTree(item.path, { signal: controller.signal });
          try {
            // The native module renders the prompt text; the parsed diff never reaches JavaScript.
            return await parsed.renderAsync();
          } finally {
            parsed.close();
          }
        }),
      );
      const subjects = await repo.recentSubjects(15, { signal: controller.signal });
      const timeout = setTimeout(() => controller.abort(), AGENT_TIMEOUT_MS);
      let message: GeneratedMessage;
      try {
        message = await generateCommitMessage(
          agent,
          {
            repositoryName: repositoryName(),
            ...(status()?.branch ? { branch: status()!.branch } : {}),
            diff: diffs.join("\n"),
            files: items.map((item) => `${item.code} ${item.path}`),
            recentSubjects: subjects,
            ...(amend() && headMessage() ? { amending: `${headMessage()!.subject}\n\n${headMessage()!.body}`.trim() } : {}),
          },
          repo.root,
          { signal: controller.signal },
        );
      } finally {
        clearTimeout(timeout);
      }
      if (controller.signal.aborted) return;
      batch(() => {
        setSubject(message.subject);
        setBody(message.body);
        setLastGenerated(message);
      });
      notify({
        type: "success",
        title: `${agent.label} wrote a commit message`,
        description: `${(message.durationMs / 1000).toFixed(1)}s${useStaged ? "" : " · from unstaged changes"}`,
        timeout: 3000,
      });
    } catch (error) {
      if (controller.signal.aborted || (error instanceof AgentError && error.aborted)) {
        notify({ type: "info", title: "Generation cancelled", timeout: 2000 });
        return;
      }
      notify({ type: "error", title: "Unable to generate a message", description: describeError(error), timeout: 9000 });
    } finally {
      if (generating()?.cancel === (() => controller.abort())) setGenerating(undefined);
      setGenerating((current) => (current?.agent === agent.id ? undefined : current));
    }
  }

  function cancelGeneration(): void {
    generating()?.cancel();
    setGenerating(undefined);
  }

  function setPreferredAgent(agent: AgentId): void {
    setPreferredAgentSignal(agent);
    persist({ preferredAgent: agent });
  }

  // --- remotes, branches, stashes, worktrees -------------------------------------------------
  const fetch = () =>
    operation("Fetch", (repo, signal) => repo.fetch({ signal }), { success: { type: "success", title: "Fetched", timeout: 2500 } });
  const pull = () =>
    operation("Pull", (repo, signal) => repo.pull({ signal }), { success: { type: "success", title: "Pulled", timeout: 2500 } });
  const push = async () => {
    const current = status();
    const branch = current?.branch;
    const setUpstream = branch && !current.upstream ? { remote: remotes()[0] ?? "origin", branch } : undefined;
    await operation("Push", (repo, signal) => repo.push(setUpstream ? { setUpstream } : {}, { signal }), {
      success: { type: "success", title: setUpstream ? `Pushed and set upstream to ${setUpstream.remote}/${branch}` : "Pushed", timeout: 2500 },
    });
  };

  const switchBranch = (name: string) =>
    operation("Switch branch", (repo, signal) => repo.switchBranch(name, { signal }), {
      success: { type: "success", title: `Switched to ${name}`, timeout: 2500 },
    });
  const createBranch = (name: string, details: { from?: string; checkout?: boolean }) =>
    operation("Create branch", (repo, signal) => repo.createBranch(name, details, { signal }), {
      success: { type: "success", title: `Created ${name}`, timeout: 2500 },
    });
  const deleteBranch = (name: string, force: boolean) =>
    operation("Delete branch", (repo, signal) => repo.deleteBranch(name, force, { signal }), {
      success: { type: "info", title: `Deleted ${name}`, timeout: 2500 },
    });
  const checkoutCommit = (sha: string) =>
    operation("Checkout", (repo, signal) => repo.checkoutCommit(sha, { signal }), {
      success: { type: "success", title: `Checked out ${sha.slice(0, 7)} (detached)`, timeout: 2500 },
    });

  const stashPush = (details: { message?: string; includeUntracked?: boolean }) =>
    operation("Stash", (repo, signal) => repo.stashPush(details, { signal }), {
      success: { type: "success", title: "Changes stashed", timeout: 2500 },
    });
  const stashApply = (ref: string) => operation("Apply stash", (repo, signal) => repo.stashApply(ref, { signal }));
  const stashPop = (ref: string) => operation("Pop stash", (repo, signal) => repo.stashPop(ref, { signal }));
  const stashDrop = (ref: string) => operation("Drop stash", (repo, signal) => repo.stashDrop(ref, { signal }));

  const addWorktree = async (details: { path: string; newBranch?: string; branch?: string; base?: string }) => {
    const main = mainRepository();
    if (!main) return undefined;
    const controller = new AbortController();
    setBusy({ label: "Add worktree", cancel: () => controller.abort() });
    try {
      await main.addWorktree(details, { signal: controller.signal });
      notify({ type: "success", title: `Added worktree at ${basename(details.path)}`, timeout: 3000 });
      return true;
    } catch (error) {
      notify({ type: "error", title: "Unable to add worktree", description: describeError(error), timeout: 9000 });
      return undefined;
    } finally {
      setBusy(undefined);
      void refresh();
    }
  };
  const removeWorktree = async (path: string, force: boolean) => {
    const main = mainRepository();
    if (!main) return;
    if (repository()?.root === path) await selectWorktree(main.root);
    const controller = new AbortController();
    setBusy({ label: "Remove worktree", cancel: () => controller.abort() });
    try {
      await main.removeWorktree(path, force, { signal: controller.signal });
      notify({ type: "info", title: `Removed worktree ${basename(path)}`, timeout: 3000 });
    } catch (error) {
      notify({ type: "error", title: "Unable to remove worktree", description: describeError(error), timeout: 9000 });
    } finally {
      setBusy(undefined);
      void refresh();
    }
  };

  /** Default location for a new worktree: a sibling directory named after the branch. */
  function suggestWorktreePath(branch: string): string {
    const main = mainRepository();
    const root = main?.root ?? repository()?.root ?? process.cwd();
    const name = branch.replace(/[\/\\:\s]+/g, "-").replace(/^-+|-+$/g, "") || "worktree";
    return join(root, "..", `${basename(root)}-${name}`);
  }

  // --- view state ------------------------------------------------------------------------------
  function setView(next: ViewId): void {
    setViewSignal(next);
    if (next === "history" && untrack(history).commits.length === 0) void loadHistory(true);
  }

  function setHistoryAllBranches(all: boolean): void {
    setHistory((state) => ({ ...state, allBranches: all }));
    persist({ historyAllBranches: all });
    void loadHistory(true);
  }

  function selectCommitFile(path: string): void {
    setCommitDetail((state) => ({ ...state, selectedPath: path }));
  }

  function setSidebarWidth(width: number): void {
    setSidebarWidthSignal(width);
    persist({ sidebarWidth: width });
  }
  function setChangesSplit(width: number): void {
    setChangesSplitSignal(width);
    persist({ changesSplit: width });
  }
  function setHistorySplit(width: number): void {
    setHistorySplitSignal(width);
    persist({ historySplit: width });
  }

  function persist(update: Partial<PersistedState>): void {
    options.persistence.update(update);
  }

  function notify(notice: Notice): void {
    notifier(notice);
  }

  function setNotifier(next: (notice: Notice) => void): void {
    notifier = next;
  }

  function flushPersistence(): Promise<void> {
    return options.persistence.flush();
  }

  function dispose(): void {
    stopWatching?.();
    stopWatching = undefined;
    generating()?.cancel();
    busy()?.cancel?.();
  }

  return {
    // state
    repository,
    mainRepository,
    repositoryName,
    opening,
    worktrees,
    status,
    numstat,
    generation,
    refs,
    stashes,
    remotes,
    recentRepositories,
    busy,
    view,
    sidebarWidth,
    changesSplit,
    historySplit,
    hasHead,
    identity,
    unstaged,
    staged,
    conflicts,
    changeCount,
    selection,
    activeCell,
    focusedList,
    activeItem,
    selectedItems,
    diff,
    diffRowCount,
    diffRowsIn,
    diffStats,
    diffSelection,
    selectedDiffLines,
    selectedDiffLineCount,
    subject,
    body,
    amend,
    committing,
    canCommit,
    headMessage,
    agents,
    preferredAgent,
    generating,
    lastGenerated,
    history,
    historySelection,
    selectedCommit,
    commitDetail,
    // actions
    openRepository,
    closeRepository,
    selectWorktree,
    refresh: () => refresh(),
    setSelection: (list: ListId, ranges: RowRanges) => setSelection((current) => ({ ...current, [list]: ranges })),
    setActiveCell,
    setFocusedList,
    setDiffSelection,
    stageItems,
    unstageItems,
    discardItems,
    stageAll: () => operation("Stage all", (repo, signal) => repo.stageAll({ signal }), { refs: false }),
    unstageAll: () => operation("Unstage all", (repo, signal) => repo.unstageAll({ signal }), { refs: false }),
    toggleStaging,
    stageHunk,
    unstageHunk,
    discardHunk,
    applySelectedLines,
    setSubject,
    setBody,
    setAmend,
    commit,
    generateMessage,
    cancelGeneration,
    setPreferredAgent,
    fetch,
    pull,
    push,
    switchBranch,
    createBranch,
    deleteBranch,
    checkoutCommit,
    stashPush,
    stashApply,
    stashPop,
    stashDrop,
    addWorktree,
    removeWorktree,
    suggestWorktreePath,
    setView,
    loadHistory,
    setHistoryAllBranches,
    setHistorySelection,
    selectCommitFile,
    setSidebarWidth,
    setChangesSplit,
    setHistorySplit,
    persistedState: () => options.persistence.current(),
    setNotifier,
    flushPersistence,
    dispose,
    notify,
    cancelBusy: () => busy()?.cancel?.(),
  };
}

function rangesFor(items: readonly ChangeItem[], paths: readonly string[]): RowRanges {
  if (paths.length === 0) return [];
  const wanted = new Set(paths);
  const ranges: number[][] = [];
  items.forEach((item, index) => {
    if (!wanted.has(item.path)) return;
    const last = ranges.at(-1);
    if (last && last[1] === index - 1) last[1] = index;
    else ranges.push([index, index]);
  });
  return ranges;
}

export function describeError(error: unknown): string {
  if (error instanceof GitError) return error.summary;
  if (error instanceof Error) return error.message;
  return String(error);
}

function truncate(text: string, length: number): string {
  return text.length <= length ? text : `${text.slice(0, length - 1)}…`;
}

export { DEFAULT_STATE };
