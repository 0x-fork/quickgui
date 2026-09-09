import { join } from "node:path";

import {
  Appearance,
  Dialog,
  Menu,
  Shell,
  Window,
  PropertyCode,
  NativeNodeTag,
  app,
  type AppearanceMode,
} from "@quickgui/native";
import { createRenderer } from "@quickgui/solid";
import { createEffect, createRoot, createSignal, flush, runWithOwner } from "solid-js";

import { GitRunner } from "./git/process.ts";
import { canonicalPath } from "./model/paths.ts";
import { createPersistence, loadPersistedState } from "./model/persistence.ts";
import { createStore, type Store } from "./model/store.ts";
import { activateCommands, commands } from "./ui/commands.ts";
import { App } from "./ui/shell.tsx";

await app.whenReady();

const paths = await app.getPaths();
const checking = process.argv.includes("--check-quick-git");
const statePath =
  !checking && paths?.dataDir ? join(paths.dataDir, "quick-git-state.json") : undefined;
const persistence = createPersistence(statePath, await loadPersistedState(statePath));
const runner = new GitRunner({ concurrency: 4 });

const [appearance, setAppearance] = createSignal<AppearanceMode>("light");

// Every window shows one repository through its own store; the git runner and the persisted
// state are shared, so a repository opened anywhere lands in every window's recent list.
const sessions = new Map<Window, Store>();
const [activeWindow, setActiveWindow] = createSignal<Window | undefined>(undefined);

function activeStore(): Store | undefined {
  const window = activeWindow();
  return window && !window.closed ? sessions.get(window) : undefined;
}

// The application menu itself (About, Hide, Quit) is the platform's; QuickGUI appends these.
// Item callbacks run against the window that was focused last.
function installMenu(recent: readonly string[], hasRepository: boolean, hasWindow: boolean): void {
  Menu.setApplicationMenu([
    {
      label: "File",
      items: [
        {
          label: "Open Repository…",
          accelerator: "CmdOrCtrl+O",
          click: () => void openRepositoryDialog(activeWindow() ?? openWindow()),
        },
        {
          type: "submenu",
          label: "Open Recent",
          enabled: recent.length > 0,
          items: recent.map((path) => ({
            label: path,
            click: () => void openRepositoryPath(path, activeWindow()),
          })),
        },
        { type: "separator" },
        {
          label: "New Branch…",
          enabled: hasRepository,
          accelerator: "CmdOrCtrl+Shift+N",
          click: () => commands()?.openDialog({ kind: "new-branch" }),
        },
        {
          label: "New Worktree…",
          enabled: hasRepository,
          accelerator: "CmdOrCtrl+Alt+N",
          click: () => commands()?.openDialog({ kind: "new-worktree" }),
        },
        {
          label: "Stash Changes…",
          enabled: hasRepository,
          accelerator: "CmdOrCtrl+Shift+T",
          click: () => commands()?.openDialog({ kind: "stash" }),
        },
        { type: "separator" },
        {
          label: "Close Repository",
          enabled: hasRepository,
          accelerator: "CmdOrCtrl+Shift+W",
          click: () => activeStore()?.closeRepository(),
        },
        {
          type: "role",
          label: "Close Window",
          role: "close-window",
          enabled: hasWindow,
          accelerator: "CmdOrCtrl+W",
        },
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
        {
          label: "Changes",
          enabled: hasRepository,
          accelerator: "CmdOrCtrl+1",
          click: () => activeStore()?.setView("changes"),
        },
        {
          label: "History",
          enabled: hasRepository,
          accelerator: "CmdOrCtrl+2",
          click: () => activeStore()?.setView("history"),
        },
        {
          label: "Branches",
          enabled: hasRepository,
          accelerator: "CmdOrCtrl+3",
          click: () => activeStore()?.setView("branches"),
        },
        {
          label: "Worktrees",
          enabled: hasRepository,
          accelerator: "CmdOrCtrl+4",
          click: () => activeStore()?.setView("worktrees"),
        },
        {
          label: "Stashes",
          enabled: hasRepository,
          accelerator: "CmdOrCtrl+5",
          click: () => activeStore()?.setView("stashes"),
        },
        { type: "separator" },
        {
          label: "Refresh",
          enabled: hasRepository,
          accelerator: "CmdOrCtrl+R",
          click: () => void activeStore()?.refresh(),
        },
        { type: "separator" },
        {
          type: "role",
          label: "Toggle Full Screen",
          role: "toggle-fullscreen",
          enabled: hasWindow,
          accelerator: "Ctrl+Cmd+F",
        },
      ],
    },
    {
      label: "Repository",
      items: [
        {
          label: "Commit",
          enabled: hasRepository,
          accelerator: "CmdOrCtrl+Enter",
          click: () => void activeStore()?.commit(),
        },
        {
          label: "Generate Commit Message",
          enabled: hasRepository,
          accelerator: "CmdOrCtrl+Shift+G",
          click: () => void activeStore()?.generateMessage(),
        },
        { type: "separator" },
        {
          label: "Stage All",
          enabled: hasRepository,
          accelerator: "CmdOrCtrl+Shift+A",
          click: () => void activeStore()?.stageAll(),
        },
        {
          label: "Unstage All",
          enabled: hasRepository,
          accelerator: "CmdOrCtrl+Shift+U",
          click: () => void activeStore()?.unstageAll(),
        },
        { type: "separator" },
        {
          label: "Fetch",
          enabled: hasRepository,
          accelerator: "CmdOrCtrl+Shift+F",
          click: () => void activeStore()?.fetch(),
        },
        {
          label: "Pull",
          enabled: hasRepository,
          accelerator: "CmdOrCtrl+Shift+L",
          click: () => void activeStore()?.pull(),
        },
        {
          label: "Push",
          enabled: hasRepository,
          accelerator: "CmdOrCtrl+Shift+P",
          click: () => void activeStore()?.push(),
        },
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
        {
          type: "role",
          label: "Minimize",
          role: "minimize-window",
          enabled: hasWindow,
          accelerator: "CmdOrCtrl+M",
        },
        { type: "role", label: "Zoom", role: "zoom-window", enabled: hasWindow },
        { type: "separator" },
        {
          type: "role",
          label: "Bring All to Front",
          role: "bring-all-to-front",
          enabled: hasWindow,
        },
      ],
    },
    {
      label: "Help",
      items: [
        {
          label: "QuickGUI on GitHub",
          click: () => void Shell.openExternal("https://github.com/egoist/quickgui"),
        },
      ],
    },
  ]);
}

/** Open a window with its own store, showing `initialRepository` when given. */
function openWindow(initialRepository?: string): Window {
  // Repository menu callbacks may belong to another window's Solid root.
  return runWithOwner(null, () => {
    const store = createStore({ runner, persistence, trash: (path) => Shell.trashItem(path) });
    const window = new Window({
      title: "Quick Git",
      visible: !checking || process.env.QUICKGUI_CHECK_VISIBLE === "1",
      focus: !checking,
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
    sessions.set(window, store);
    setActiveWindow(window);
    activateCommands(window);
    void Appearance.getCurrent(window)
      .then(setAppearance)
      .catch(() => {});
    window.on("appearanceChange", ({ appearance: next }) => setAppearance(next));
    window.on("focus", () => {
      setActiveWindow(window);
      activateCommands(window);
    });
    // The title and represented file follow this window's repository.
    const disposeEffects = createRoot((dispose) => {
      createEffect(
        () => ({
          name: store.repositoryName(),
          branch: store.status()?.branch,
          root: store.repository()?.root,
        }),
        ({ name, branch, root }) => {
          if (window.closed) return;
          window.setTitle(name ? `${name}${branch ? ` — ${branch}` : ""}` : "Quick Git");
          window.setRepresentedFile(root);
        },
      );
      return dispose;
    });
    window.on("closed", () => {
      disposeEffects();
      sessions.delete(window);
      store.dispose();
      if (activeWindow() === window) {
        const next = [...sessions.keys()].at(-1);
        setActiveWindow(next);
        if (next) activateCommands(next);
      }
    });
    if (initialRepository) void store.openRepository(initialRepository);
    return window;
  });
}

function windowShowing(root: string): Window | undefined {
  for (const [window, store] of sessions) {
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
  const fromStore = from && !from.closed ? sessions.get(from) : undefined;
  if (fromStore && !fromStore.repository() && !fromStore.opening()) {
    if (await fromStore.openRepository(target)) fromStore.setView("changes");
    from!.focus();
    return;
  }
  openWindow(target);
}

async function openRepositoryDialog(from: Window): Promise<void> {
  const result = await Dialog.showOpenDialog(from, {
    title: "Open Repository",
    buttonLabel: "Open",
    properties: ["openDirectory"],
  });
  const [path] = result.filePaths;
  if (!result.canceled && path) await openRepositoryPath(path, from);
}

// Module-level effects live in one root so they have an owner and dispose together.
createRoot(() => {
  createEffect(
    () => ({
      recent: persistence.state().recentRepositories,
      hasRepository: activeStore()?.repository() !== undefined,
      hasWindow: activeStore() !== undefined,
    }),
    ({ recent, hasRepository, hasWindow }) => installMenu(recent, hasRepository, hasWindow),
  );
});

app.on("reopen", ({ hasVisibleWindows }) => {
  if (!hasVisibleWindows) openWindow();
});

app.on("quit", () => persistence.flush());

// `QUICK_GIT_OPEN=/path` opens a repository at launch, for scripted runs and screenshots.
const initialRepository =
  process.env.QUICK_GIT_OPEN?.trim() || persistence.current().lastRepository;
if (checking) {
  if (!process.env.QUICK_GIT_OPEN)
    throw new Error("Quick Git validation requires a disposable repository");
  const window = openWindow();
  const store = sessions.get(window)!;
  if (!(await store.openRepository(process.env.QUICK_GIT_OPEN)))
    throw new Error("Could not open validation repository");
  await window.whenReady();
  async function settled(ready: () => boolean, description: string): Promise<void> {
    const deadline = Date.now() + 5000;
    while (Date.now() < deadline) {
      flush();
      window.flush();
      if (ready()) return;
      await new Promise((resolve) => setTimeout(resolve, 10));
    }
    throw new Error(`Quick Git validation did not settle: ${description}`);
  }
  for (const view of ["changes", "history", "branches", "worktrees", "stashes"] as const) {
    store.setView(view);
    await store.refresh();
    flush();
    window.flush();
    const state = await window.getState();
    if (!store.repository() || state.viewportSize.width !== 1240 || window.nodes.size < 10)
      throw new Error(`Quick Git did not render ${view}`);
    if ([...window.nodes.values()].some((node) => node.tag === NativeNodeTag.SwiftUIHost))
      throw new Error(`Quick Git still mounts a SwiftUI host in ${view}`);
    const splitters = [...window.nodes.values()].filter(
      (node) => node.properties.get(PropertyCode.Part) === "splitter",
    );
    if (splitters.length !== (view === "changes" || view === "history" ? 2 : 1))
      throw new Error(`Missing native splitter in ${view}`);
    console.log(`Quick Git view ready: ${view}`);
  }
  store.setView("history");
  await settled(
    () =>
      store.commitDetail().files.length === 128 &&
      store.diff().files.length > 0 &&
      !store.diff().loading,
    "history detail",
  );
  const fileTable = [...window.nodes.values()].find(
    (node) => node.properties.get(PropertyCode.AccessibilityLabel) === "Commit files",
  )!;
  if (process.env.QUICKGUI_CHECK_VISIBLE === "1")
    await settled(() => fileTable.children.length > 0, "visible commit rows");
  if (fileTable.children.length >= 128) throw new Error("Commit file list is not virtualized");
  const path = store.commitDetail().files.at(-1)!.path;
  store.selectCommitFile(path);
  await settled(
    () => store.diff().target?.path === path && !store.diff().loading,
    "selected commit file",
  );
  const selectedDiff = store.diff();
  await store.refresh();
  flush();
  if (store.diff() !== selectedDiff || !window.nodes.has(fileTable.id))
    throw new Error("Focus refresh replaced the selected diff or commit file table");
  // Exercise mutations only on the repository allocated by the validation script.
  if (process.env.QUICKGUI_TEST_REPOSITORY !== process.env.QUICK_GIT_OPEN)
    throw new Error("Quick Git checks require the disposable test repository");
  store.setView("changes");
  flush();
  await store.stageAll();
  flush();
  if (!store.staged().some((item) => item.path === "README.md"))
    throw new Error("Stage all did not update the index");
  await store.unstageAll();
  flush();
  if (!store.unstaged().some((item) => item.path === "README.md"))
    throw new Error("Unstage all did not update the change list");
  const second = openWindow();
  const secondStore = sessions.get(second)!;
  await secondStore.openRepository(process.env.QUICK_GIT_OPEN);
  await second.whenReady();
  const closed = new Promise<void>((resolve) => window.onClose(() => resolve()));
  window.close();
  await closed;
  secondStore.setView("history");
  await secondStore.refresh();
  flush();
  second.flush();
  if (second.closed || secondStore.history().commits.length !== 2 || second.nodes.size < 10)
    throw new Error("Closing the first repository window disposed the second");
  await second.getState();
  const secondClosed = new Promise<void>((resolve) => second.onClose(() => resolve()));
  second.close();
  await secondClosed;
  console.log("QUICKGUI_QUICK_GIT_OK");
  await app.exit();
} else {
  openWindow(initialRepository || undefined);
}
