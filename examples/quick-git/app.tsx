import { join } from "node:path";

import { Appearance, Menu, Shell, Window, app, type AppearanceMode } from "@quickgui/native";
import { createRenderer } from "@quickgui/solid";
import { createEffect, createRoot, createSignal } from "solid-js";

import { GitRunner } from "./git/process.ts";
import { loadPersistedState } from "./model/persistence.ts";
import { createStore } from "./model/store.ts";
import { commands } from "./ui/commands.ts";
import { App } from "./ui/shell.tsx";

await app.whenReady();

const paths = await app.getPaths();
const statePath = paths?.dataDir ? join(paths.dataDir, "quick-git-state.json") : undefined;
const persisted = await loadPersistedState(statePath);
const runner = new GitRunner({ concurrency: 4 });
const store = createStore({
  runner,
  persisted,
  ...(statePath ? { statePath } : {}),
  trash: (path) => Shell.trashItem(path),
});

const [appearance, setAppearance] = createSignal<AppearanceMode>("light");

// The application menu itself (About, Hide, Quit) is the platform's; QuickGUI appends these.
function installMenu(recent: readonly string[], hasRepository: boolean): void {
  Menu.setApplicationMenu([
    {
      label: "File",
      items: [
        { label: "Open Repository…", accelerator: "CmdOrCtrl+O", click: () => commands()?.openRepository() },
        {
          type: "submenu",
          label: "Open Recent",
          enabled: recent.length > 0,
          items: recent.map((path) => ({ label: path, click: () => void store.openRepository(path) })),
        },
        { type: "separator" },
        { label: "New Branch…", accelerator: "CmdOrCtrl+Shift+N", click: () => commands()?.openDialog({ kind: "new-branch" }) },
        { label: "New Worktree…", accelerator: "CmdOrCtrl+Alt+N", click: () => commands()?.openDialog({ kind: "new-worktree" }) },
        { label: "Stash Changes…", accelerator: "CmdOrCtrl+Shift+T", click: () => commands()?.openDialog({ kind: "stash" }) },
        { type: "separator" },
        { label: "Close Repository", accelerator: "CmdOrCtrl+Shift+W", click: () => store.closeRepository() },
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
        { label: "Changes", accelerator: "CmdOrCtrl+1", click: () => store.setView("changes") },
        { label: "History", accelerator: "CmdOrCtrl+2", click: () => store.setView("history") },
        { label: "Branches", accelerator: "CmdOrCtrl+3", click: () => store.setView("branches") },
        { label: "Worktrees", accelerator: "CmdOrCtrl+4", click: () => store.setView("worktrees") },
        { label: "Stashes", accelerator: "CmdOrCtrl+5", click: () => store.setView("stashes") },
        { type: "separator" },
        { label: "Refresh", accelerator: "CmdOrCtrl+R", click: () => void store.refresh() },
        { type: "separator" },
        { type: "role", label: "Toggle Full Screen", role: "toggle-fullscreen", accelerator: "Ctrl+Cmd+F" },
      ],
    },
    {
      label: "Repository",
      items: [
        { label: "Commit", accelerator: "CmdOrCtrl+Enter", click: () => void store.commit() },
        { label: "Generate Commit Message", accelerator: "CmdOrCtrl+Shift+G", click: () => void store.generateMessage() },
        { type: "separator" },
        { label: "Stage All", accelerator: "CmdOrCtrl+Shift+A", click: () => void store.stageAll() },
        { label: "Unstage All", accelerator: "CmdOrCtrl+Shift+U", click: () => void store.unstageAll() },
        { type: "separator" },
        { label: "Fetch", accelerator: "CmdOrCtrl+Shift+F", click: () => void store.fetch() },
        { label: "Pull", accelerator: "CmdOrCtrl+Shift+L", click: () => void store.pull() },
        { label: "Push", accelerator: "CmdOrCtrl+Shift+P", click: () => void store.push() },
        { type: "separator" },
        { label: "Reveal in Finder", enabled: hasRepository, click: () => void Shell.showItemInFolder(store.repository()!.root) },
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


function openMainWindow(): Window {
  const restore = store.persistedState();
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
    renderer: createRenderer(() => <App store={store} appearance={appearance} />),
  });
  void Appearance.getCurrent(window).then(setAppearance).catch(() => {});
  window.on("appearanceChange", ({ appearance: next }) => setAppearance(next));
  void restore;
  return window;
}

let mainWindow = openMainWindow();

// Module-level effects live in one root so they have an owner and dispose together.
createRoot(() => {
  createEffect(
    () => ({ recent: store.recentRepositories(), hasRepository: store.repository() !== undefined }),
    ({ recent, hasRepository }) => installMenu(recent, hasRepository),
  );
  createEffect(
    () => ({ name: store.repositoryName(), branch: store.status()?.branch, root: store.repository()?.root }),
    ({ name, branch, root }) => {
      if (mainWindow.closed) return;
      mainWindow.setTitle(name ? `${name}${branch ? ` — ${branch}` : ""}` : "Quick Git");
      mainWindow.setRepresentedFile(root);
    },
  );
});

app.on("reopen", ({ hasVisibleWindows }) => {
  if (!hasVisibleWindows) mainWindow = openMainWindow();
});

app.on("willQuit", () => {
  void store.flushPersistence();
});

// `QUICK_GIT_OPEN=/path` opens a repository at launch, for scripted runs and screenshots.
const initialRepository = process.env.QUICK_GIT_OPEN?.trim() || persisted.lastRepository;
if (initialRepository) {
  void store.openRepository(initialRepository);
}
