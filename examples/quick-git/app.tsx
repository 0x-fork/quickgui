import { untrack as nativeUntrack, onCleanup as nativeOnCleanup } from "@quickgui/ui";
import { join } from "node:path";

import { Appearance, Dialog, Menu, Shell, Window, app, type AppearanceMode } from "@quickgui/native";
import { createRenderer } from "@quickgui/ui";
import { createEffect, createRoot, createSignal } from "@quickgui/ui";

import { GitRunner } from "./git/process.ts";
import { canonicalPath } from "./model/paths.ts";
import { createPersistence, loadPersistedState } from "./model/persistence.ts";
import { createStore, type Store } from "./model/store.ts";
import { activateCommands, commands } from "./ui/commands.ts";
import { App } from "./ui/shell.tsx";

console.log("[startup probe] waiting for app");
await app.whenReady();
console.log("[startup probe] app ready");

const paths = await app.getPaths();
console.log("[startup probe] paths ready");
const statePath = paths?.dataDir ? join(paths.dataDir, "quick-git-state.json") : undefined;
const persistence = createPersistence(statePath, await loadPersistedState(statePath));
console.log("[startup probe] persistence ready");
const runner = new GitRunner({ concurrency: 4 });

const [appearance, setAppearance] = createSignal<AppearanceMode>("light");

// Every window shows one repository through its own store; the git runner and the persisted
// state are shared, so a repository opened anywhere lands in every window's recent list.
const sessions = new Map<number, { window: Window; store: Store }>();
const [activeWindow, setActiveWindow] = createSignal<Window | undefined>(undefined);

function activeStore(): Store | undefined {
  const window = activeWindow();
  return window && !window.closed ? sessions.get(window.nativeId)?.store : undefined;
}

// The application menu itself (About, Hide, Quit) is the platform's; QuickGUI appends these.
// Item callbacks run against the window that was focused last.
function installMenu(recent: readonly string[], hasRepository: boolean): void {
  Menu.setApplicationMenu([
    {
      label: "File",
      items: [
        { label: "Open Repository…", accelerator: "CmdOrCtrl+O", click: () => { commands()?.openRepository(); } },
        {
          type: "submenu",
          label: "Open Recent",
          enabled: recent.length > 0,
          items: recent.map((path) => ({ label: path, click: () => void openRepositoryPath(path, activeWindow()) })),
        },
        { type: "separator" },
        { label: "New Branch…", accelerator: "CmdOrCtrl+Shift+N", click: () => { commands()?.openDialog({ kind: "new-branch" }); } },
        { label: "New Worktree…", accelerator: "CmdOrCtrl+Alt+N", click: () => { commands()?.openDialog({ kind: "new-worktree" }); } },
        { label: "Stash Changes…", accelerator: "CmdOrCtrl+Shift+T", click: () => { commands()?.openDialog({ kind: "stash" }); } },
        { type: "separator" },
        { label: "Close Repository", accelerator: "CmdOrCtrl+Shift+W", click: () => { activeStore()?.closeRepository(); } },
        { type: "role", label: "Close Window", role: "close-window", accelerator: "CmdOrCtrl+W" },
      ],
    },
    {
      label: "Edit",
      items: [
        { type: "role", label: "Undo", role: "undo", accelerator: "CmdOrCtrl+Z" },
        { type: "role", label: "Redo", role: "redo", accelerator: "CmdOrCtrl+Shift+Z" },
        { type: "separator" },
        { type: "role", label: "Cut", role: "cut", accelerator: "CmdOrCtrl+X" },
        { type: "role", label: "Copy", role: "copy", accelerator: "CmdOrCtrl+C" },
        { type: "role", label: "Paste", role: "paste", accelerator: "CmdOrCtrl+V" },
        { type: "role", label: "Select All", role: "select-all", accelerator: "CmdOrCtrl+A" },
      ],
    },
    {
      label: "View",
      items: [
        { label: "Changes", accelerator: "CmdOrCtrl+1", click: () => { activeStore()?.setView("changes"); } },
        { label: "History", accelerator: "CmdOrCtrl+2", click: () => { activeStore()?.setView("history"); } },
        { label: "Branches", accelerator: "CmdOrCtrl+3", click: () => { activeStore()?.setView("branches"); } },
        { label: "Worktrees", accelerator: "CmdOrCtrl+4", click: () => { activeStore()?.setView("worktrees"); } },
        { label: "Stashes", accelerator: "CmdOrCtrl+5", click: () => { activeStore()?.setView("stashes"); } },
        { type: "separator" },
        { label: "Refresh", accelerator: "CmdOrCtrl+R", click: () => void activeStore()?.refresh() },
        { type: "separator" },
        { type: "role", label: "Toggle Full Screen", role: "toggle-fullscreen", accelerator: "Ctrl+Cmd+F" },
      ],
    },
    {
      label: "Repository",
      items: [
        { label: "Commit", accelerator: "CmdOrCtrl+Enter", click: () => void activeStore()?.commit() },
        { label: "Generate Commit Message", accelerator: "CmdOrCtrl+Shift+G", click: () => void activeStore()?.generateMessage(undefined) },
        { type: "separator" },
        { label: "Stage All", accelerator: "CmdOrCtrl+Shift+A", click: () => void activeStore()?.stageAll() },
        { label: "Unstage All", accelerator: "CmdOrCtrl+Shift+U", click: () => void activeStore()?.unstageAll() },
        { type: "separator" },
        { label: "Fetch", accelerator: "CmdOrCtrl+Shift+F", click: () => void activeStore()?.fetch() },
        { label: "Pull", accelerator: "CmdOrCtrl+Shift+L", click: () => void activeStore()?.pull() },
        { label: "Push", accelerator: "CmdOrCtrl+Shift+P", click: () => void activeStore()?.push() },
        { type: "separator" },
        {
          label: "Reveal in Finder",
          enabled: hasRepository,
          click: () => {
            const root = activeStore()?.repository()?.root;
            if (root) void Shell.showItemInFolder(root);
          },
        },
      ],
    },
    {
      label: "Window",
      items: [
        { type: "role", label: "Minimize", role: "minimize-window", accelerator: "CmdOrCtrl+M" },
        { type: "role", label: "Zoom", role: "zoom-window" },
        { type: "separator" },
        { type: "role", label: "Bring All to Front", role: "bring-all-to-front" },
      ],
    },
    {
      label: "Help",
      items: [{ label: "QuickGUI on GitHub", click: () => void Shell.openExternal("https://github.com/egoist/quickgui") }],
    },
  ]);
}

/** Open a window with its own store, showing `initialRepository` when given. */
function openWindow(initialRepository?: string): Window {
  const store = createStore({ runner, persistence, trash: (path) => Shell.trashItem(path) });
  const window = new Window({
    title: "Quick Git",
    width: 1240,
    height: 800,
    minimumWidth: 900,
    minimumHeight: 560,
    background: "transparent",
    vibrancy: "sidebar",
    visualEffectState: "followWindow",
    appearance: "system",
    titleBarStyle: "hiddenInset",
    trafficLightPosition: { x: 14, y: 19 },
    renderer: createRenderer(() => (
      <App
        store={store}
        appearance={appearance}
        openRepository={() => openRepositoryDialog(window)}
        openRepositoryPath={(path) => openRepositoryPath(path, window)}
      />
    )),
  });
  sessions.set(window.nativeId, { window, store });
  setActiveWindow(window);
  activateCommands(window);
  window.on("readyToShow", () => { void adoptAppearance(); });
  async function adoptAppearance(): Promise<void> {
    try {
      const state = await window.getState();
      if (!window.closed) setAppearance(state.appearance === "dark" ? "dark" : "light");
    } catch {}
  }
  window.on("appearanceChange", ({ appearance: next }) => { if (next === "dark" || next === "light") setAppearance(next); });
  window.on("focus", () => {
    setActiveWindow(window);
    activateCommands(window);
  });
  // The title and represented file follow this window's repository.
  const disposeEffects = createRoot((dispose) => {
    createEffect(
      () => { const value = (() => ({ name: store.repositoryName(), branch: store.status()?.branch, root: store.repository()?.root }))(); nativeUntrack(() => (({ name, branch, root }) => {
        if (window.closed) return;
        window.setTitle(name ? `${name}${branch ? ` — ${branch}` : ""}` : "Quick Git");
        window.setRepresentedFile(root);
      })(value)); },
    );
    return dispose;
  });
  window.on("closed", () => {
    disposeEffects();
    sessions.delete(window.nativeId);
    store.dispose();
    if (activeWindow() === window) setActiveWindow([...sessions.values()].at(-1)?.window);
  });
  if (initialRepository) void store.openRepository(initialRepository);
  return window;
}

function windowShowing(root: string): Window | undefined {
  for (const { window, store } of sessions.values()) {
    if (window.closed) continue;
    if ((store.mainRepository()?.root ?? store.repository()?.root) === root) return window;
  }
  return undefined;
}

/**
 * Show the repository at `path`: focus the window that already has it, fill `from` when it shows
 * nothing, and otherwise open a new window for it.
 */
async function openRepositoryPath(path: string, from?: Window): Promise<void> {
  const target = canonicalPath(path);
  const existing = windowShowing(target);
  if (existing) {
    existing.focus();
    return;
  }
  const fromStore = from && !from.closed ? sessions.get(from.nativeId)?.store : undefined;
  if (fromStore && !fromStore.repository() && !fromStore.opening()) {
    if (await fromStore.openRepository(target)) fromStore.setView("changes");
    return;
  }
  openWindow(target);
}

async function openRepositoryDialog(from: Window): Promise<void> {
  const result = await Dialog.showOpenDialog({
    title: "Open Repository",
    buttonLabel: "Open",
    properties: ["openDirectory"],
  }, from);
  const path = result.filePaths.at(0);
  if (!result.canceled && path) await openRepositoryPath(path, from);
}

// Module-level effects live in one root so they have an owner and dispose together.
createRoot(() => {
  createEffect(
    () => { const value = (() => ({ recent: persistence.state().recentRepositories, hasRepository: activeStore()?.repository() !== undefined }))(); nativeUntrack(() => (({ recent, hasRepository }) => installMenu(recent, hasRepository))(value)); },
  );
});

app.onReopen( ({ hasVisibleWindows }) => {
  if (!hasVisibleWindows) openWindow();
});

app.onBeforeQuit(() => { void flushAndQuit(); });
async function flushAndQuit(): Promise<void> {
  await persistence.flush();
  await app.quit(true);
}

// `QUICK_GIT_OPEN=/path` opens a repository at launch, for scripted runs and screenshots.
const initialRepository = process.env.QUICK_GIT_OPEN?.trim() || persistence.current().lastRepository;
console.log("[startup probe] opening window");
openWindow(initialRepository || undefined);
console.log("[startup probe] window created");
