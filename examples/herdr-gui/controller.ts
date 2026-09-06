import { untrack as nativeUntrack, onCleanup as nativeOnCleanup } from "@quickgui/ui";
import {
  Dialog,
  Menu,
  type Window,
  type AppearanceMode,
  type AppearancePreference,
  type NativeNode,
  type QuickGuiEvent,
} from "@quickgui/native";
import { capturedPointerFromEvent, terminalStatusFromEvent, type TerminalStatusEvent } from "@quickgui/ui";
import { createMemo, createSignal, onCleanup } from "@quickgui/ui";

import {
  clamp,
  focusedCloseTarget,
  spaceName,
  type AgentLauncher,
  type AgentLauncherId,
  type HerdrModel,
  type Pane,
  type SavedState,
  type Space,
  type SplitDirection,
  type WorkspaceTab,
} from "./model.ts";
import {
  captureShellEnvironment,
  launcherDefinitions,
  loginShell,
  resolveLaunchers,
} from "./services/launchers.ts";
import {
  writeSavedState,
} from "./services/persistence.ts";
import { createStyles, themeFor } from "./theme.ts";

interface PaneOptions {
  label: string;
  program: string;
  arguments: string[];
  environment?: Readonly<Record<string, string>>;
  requestedAgent?: AgentLauncherId;
  initialPrompt: string;
}

export interface CreateHerdrModelOptions {
  window: Window;
  homePath: string;
  stateFile: string | undefined;
  savedState: SavedState;
  restoredSpaces: Space[];
  initialAppearance: AppearancePreference;
}

export function createHerdrModel(options: CreateHerdrModelOptions): HerdrModel {
  const {
    window,
    homePath,
    stateFile,
    savedState,
    restoredSpaces,
    initialAppearance,
  } = options;
  const [appearance, setAppearance] = createSignal<AppearanceMode>(
    initialAppearance === "light" ? "light" : "dark",
  );
  const [appearancePreference, setAppearancePreference] =
    createSignal<AppearancePreference>(initialAppearance);
  const initialSidebarWidth = clamp(savedState.sidebarWidth ?? 248, 196, 380);
  const initialSidebarSectionRatio = clamp(
    savedState.sidebarSectionRatio ?? 0.5,
    0.25,
    0.75,
  );
  const [sidebarWidth, setSidebarWidth] = createSignal(initialSidebarWidth);
  const [sidebarSectionRatio, setSidebarSectionRatio] = createSignal(
    initialSidebarSectionRatio,
  );
  const [spaces, setSpaces] = createSignal<Space[]>(restoredSpaces);
  const restoredActiveSpace =
    restoredSpaces.find((space) => space.id === savedState.activeSpaceId) ??
    restoredSpaces[0]!;
  const initialTabId = 1;
  const initialPaneId = 1;
  const [initialPaneStatus, setInitialPaneStatus] =
    createSignal<TerminalStatusEvent>({
      status: "starting",
      title: "",
      workingDirectory: restoredActiveSpace.path,
    });
  const initialPane: Pane = {
    id: initialPaneId,
    tabId: initialTabId,
    spaceId: restoredActiveSpace.id,
    label: "Terminal 1",
    program: loginShell(),
    arguments: ["-l"],
    initialPrompt: "",
    status: initialPaneStatus,
    setStatus: setInitialPaneStatus,
  };
  const initialTab: WorkspaceTab = {
    id: initialTabId,
    spaceId: restoredActiveSpace.id,
    number: 1,
    paneIds: [initialPaneId],
    activePaneId: initialPaneId,
    splitDirection: "horizontal",
  };
  const [activeSpaceId, setActiveSpaceId] = createSignal(
    restoredActiveSpace.id,
  );
  const [tabs, setTabs] = createSignal<WorkspaceTab[]>([initialTab]);
  const [panes, setPanes] = createSignal<Pane[]>([initialPane]);
  const [activeTabId, setActiveTabId] = createSignal<number | undefined>(
    initialTabId,
  );
  const [activePaneId, setActivePaneId] = createSignal<number | undefined>(
    initialPaneId,
  );
  const [agentSheetOpen, setAgentSheetOpen] = createSignal(false);
  const [selectedLauncherId, setSelectedLauncherId] =
    createSignal<AgentLauncherId>("codex");
  const [initialPrompt, setInitialPrompt] = createSignal("");
  const [catalogLoading, setCatalogLoading] = createSignal(true);
  const [launchEnvironment, setLaunchEnvironment] =
    createSignal<Readonly<Record<string, string>> | undefined>(undefined);
  const [launchers, setLaunchers] = createSignal<AgentLauncher[]>(
    launcherDefinitions.map((launcher) => ({ ...launcher, installed: false })),
  );
  const terminalRefs = new Map<number, NativeNode>();
  const lastActiveTabBySpace = new Map<string, number>([
    [restoredActiveSpace.id, initialTabId],
  ]);
  let nextPaneId = initialPaneId + 1;
  let nextTabId = initialTabId + 1;
  let addingSpace = false;
  let sidebarDragStart = initialSidebarWidth;
  let sectionDragStart = initialSidebarSectionRatio;
  let sectionDragHeight = 320;
  let persistenceTimer: ReturnType<typeof setTimeout> | undefined;
  let disposed = false;

  const theme = createMemo(() => themeFor(appearance()));
  const styles = createMemo(() => createStyles(theme()));
  const activeSpace = createMemo(
    () =>
      spaces().find((space) => space.id === activeSpaceId()) ?? spaces()[0]!,
  );
  const activeSpaceTabs = createMemo(() =>
    tabs().filter((tab) => tab.spaceId === activeSpace().id),
  );
  const activeTab = createMemo(() =>
    tabs().find((tab) => tab.id === activeTabId()),
  );
  const activePane = createMemo(() =>
    panes().find((pane) => pane.id === activePaneId()),
  );
  const visibleAgents = createMemo(() => {
    const spaceOrder = new Map(
      spaces().map((space, index) => [space.id, index] as const),
    );
    const tabOrder = new Map(tabs().map((tab, index) => [tab.id, index] as const));
    return panes()
      .filter((pane) => pane.status().agent !== undefined)
      .slice()
      .sort(
        (left, right) =>
          (spaceOrder.get(left.spaceId) ?? 0) -
            (spaceOrder.get(right.spaceId) ?? 0) ||
          (tabOrder.get(left.tabId) ?? 0) -
            (tabOrder.get(right.tabId) ?? 0),
      );
  });
  const selectedLauncher = createMemo(
    () =>
      launchers().find((launcher) => launcher.id === selectedLauncherId()) ??
      launchers()[0]!,
  );

  const stopAppearance = window.on("appearanceChange", (event) => {
    if (event.appearance === "dark" || event.appearance === "light") setAppearance(event.appearance);
  });
  const updateWindowMetrics = (state: Awaited<ReturnType<Window["getState"]>>) => {
    sectionDragHeight = Math.max(state.viewportSize.height - 40, 320);
  };
  const refreshWindowState = async (): Promise<void> => {
    try {
      const state = await window.getState();
      if (!disposed && !window.closed) {
        updateWindowMetrics(state);
        setAppearance(state.appearance === "dark" ? "dark" : "light");
      }
    } catch {}
  };
  const stopWindowState = window.on("stateChange", () => { void refreshWindowState(); });
  const stopReady = window.on("readyToShow", () => { void refreshWindowState(); });
  onCleanup(() => {
    disposed = true;
    stopAppearance();
    stopWindowState();
    stopReady();
    if (persistenceTimer) clearTimeout(persistenceTimer);
  });


  void captureShellEnvironment().then(async (environment) => {
    const resolved = await resolveLaunchers(environment, homePath);
    if (disposed) return;
    setLaunchEnvironment(environment);
    setLaunchers(resolved);
    const first = resolved.find((launcher) => launcher.installed);
    if (first) setSelectedLauncherId(first.id);
    setCatalogLoading(false);
  });

  function createPane(
    space: Space,
    tabId: number,
    options: PaneOptions,
    existingId: number | undefined,
  ): Pane {
    const id = existingId ?? nextPaneId++;
    const [status, setStatus] = createSignal<TerminalStatusEvent>({
      status: "starting",
      title: "",
      workingDirectory: space.path,
    });
    return {
      id,
      tabId,
      spaceId: space.id,
      label: options.label,
      program: options.program,
      arguments: options.arguments,
      ...(options.environment ? { environment: options.environment } : {}),
      ...(options.requestedAgent
        ? { requestedAgent: options.requestedAgent }
        : {}),
      initialPrompt: options.initialPrompt,
      status,
      setStatus,
    };
  }

  function createTabWithPane(space: Space, options: PaneOptions) {
    const id = nextTabId++;
    const number =
      tabs()
        .filter((tab) => tab.spaceId === space.id)
        .reduce((maximum, tab) => Math.max(maximum, tab.number), 0) + 1;
    const pane = createPane(space, id, options, undefined);
    const tab: WorkspaceTab = {
      id,
      spaceId: space.id,
      number,
      paneIds: [pane.id],
      activePaneId: pane.id,
      splitDirection: "horizontal",
    };
    return { tab, pane };
  }

  function activateTab(tab: WorkspaceTab) {
    setActiveSpaceId(tab.spaceId);
    setActiveTabId(tab.id);
    setActivePaneId(tab.activePaneId);
    lastActiveTabBySpace.set(tab.spaceId, tab.id);
    focusPane(tab.activePaneId);
  }

  function addTab(space: Space, options: PaneOptions) {
    const created = createTabWithPane(space, options);
    setPanes(((current: ReturnType<typeof panes>) => [...current, created.pane])(nativeUntrack(panes)));
    setTabs(((current: ReturnType<typeof tabs>) => [...current, created.tab])(nativeUntrack(tabs)));
    activateTab(created.tab);
    schedulePersist();
  }

  function newTerminal(candidate: Space | undefined) {
    const space = candidate ?? activeSpace();
    const count = panes().filter((pane) => pane.spaceId === space.id).length + 1;
    const environment = launchEnvironment();
    addTab(space, {
      label: "Terminal " + count,
      program: loginShell(),
      arguments: ["-l"],
      ...(environment ? { environment } : {}),
      initialPrompt: "",
    });
  }

  function splitTerminal(direction: SplitDirection) {
    const tab = activeTab();
    if (!tab) {
      newTerminal(undefined);
      return;
    }
    const space = activeSpace();
    const count = panes().filter((pane) => pane.spaceId === space.id).length + 1;
    const environment = launchEnvironment();
    const pane = createPane(space, tab.id, {
      label: "Terminal " + count,
      program: loginShell(),
      arguments: ["-l"],
      ...(environment ? { environment } : {}),
      initialPrompt: "",
    }, undefined);
    setPanes(((current: ReturnType<typeof panes>) => [...current, pane])(nativeUntrack(panes)));
    setTabs(((current: ReturnType<typeof tabs>) =>
      current.map((candidate) => {
        if (candidate.id !== tab.id) return candidate;
        const activeIndex = candidate.paneIds.indexOf(candidate.activePaneId);
        const offset = activeIndex < 0 ? candidate.paneIds.length : activeIndex + 1;
        const paneIds = [...candidate.paneIds.slice(0, offset), pane.id, ...candidate.paneIds.slice(offset)];
        return {
          ...candidate,
          paneIds,
          activePaneId: pane.id,
          splitDirection: direction,
        };
      }))(nativeUntrack(tabs)),
    );
    setActivePaneId(pane.id);
    focusPane(pane.id);
  }

  function openAgentSheet() {
    const firstInstalled = launchers().find((launcher) => launcher.installed);
    if (firstInstalled) setSelectedLauncherId(firstInstalled.id);
    setInitialPrompt("");
    setAgentSheetOpen(true);
  }

  function launchAgent() {
    const launcher = selectedLauncher();
    if (!launcher.installed || !launcher.executable) return;
    const prompt = initialPrompt().trim();
    const environment = launchEnvironment();
    addTab(activeSpace(), {
      label: launcher.label,
      program: launcher.executable,
      arguments: launcher.argumentsFor(prompt),
      ...(environment ? { environment } : {}),
      requestedAgent: launcher.id,
      initialPrompt: prompt,
    });
    setAgentSheetOpen(false);
  }

  function selectSpace(space: Space) {
    setActiveSpaceId(space.id);
    const candidateId = lastActiveTabBySpace.get(space.id);
    const tab =
      tabs().find((entry) => entry.id === candidateId) ??
      tabs()
        .filter((entry) => entry.spaceId === space.id)
        .at(-1);
    if (tab) activateTab(tab);
    else newTerminal(space);
    schedulePersist();
  }

  function selectTab(tab: WorkspaceTab) {
    activateTab(tab);
    schedulePersist();
  }

  function selectPane(pane: Pane) {
    const tab = tabs().find((candidate) => candidate.id === pane.tabId);
    if (!tab) return;
    setTabs(((current: ReturnType<typeof tabs>) =>
      current.map((candidate) =>
        candidate.id === tab.id
          ? { ...candidate, activePaneId: pane.id }
          : candidate,
      ))(nativeUntrack(tabs)),
    );
    setActiveSpaceId(pane.spaceId);
    setActiveTabId(tab.id);
    setActivePaneId(pane.id);
    lastActiveTabBySpace.set(pane.spaceId, tab.id);
    focusPane(pane.id);
  }

  function focusPane(id: number) {
    queueMicrotask(() => terminalRefs.get(id)?.focus());
  }

  function closePane(id: number) {
    const closing = panes().find((pane) => pane.id === id);
    if (!closing) return;
    const tab = tabs().find((candidate) => candidate.id === closing.tabId);
    if (!tab) return;
    if (tab.paneIds.length === 1) {
      closeTab(tab.id);
      return;
    }
    const paneIds = tab.paneIds.filter((paneId) => paneId !== id);
    const closingIndex = tab.paneIds.indexOf(id);
    const nextPaneId = paneIds[Math.min(Math.max(closingIndex, 0), paneIds.length - 1)]!;
    setPanes(((current: ReturnType<typeof panes>) => current.filter((pane) => pane.id !== id))(nativeUntrack(panes)));
    setTabs(((current: ReturnType<typeof tabs>) =>
      current.map((candidate) =>
        candidate.id === tab.id
          ? { ...candidate, paneIds, activePaneId: nextPaneId }
          : candidate,
      ))(nativeUntrack(tabs)),
    );
    terminalRefs.delete(id);
    if (activePaneId() === id) {
      setActivePaneId(nextPaneId);
      focusPane(nextPaneId);
    }
  }

  function closeTab(id: number) {
    const closing = tabs().find((tab) => tab.id === id);
    if (!closing) return;
    const closingPaneIds = new Set(closing.paneIds);
    const remainingTabs = tabs().filter((tab) => tab.id !== id);
    setTabs(remainingTabs);
    setPanes(((current: ReturnType<typeof panes>) =>
      current.filter((pane) => !closingPaneIds.has(pane.id)))(nativeUntrack(panes)),
    );
    for (const paneId of closingPaneIds) terminalRefs.delete(paneId);
    if (activeTabId() !== id) return;
    const next =
      remainingTabs.filter((tab) => tab.spaceId === closing.spaceId).at(-1) ??
      remainingTabs.at(-1);
    if (next) {
      activateTab(next);
      return;
    }
    setActiveTabId(undefined);
    setActivePaneId(undefined);
    const space = spaces().find((candidate) => candidate.id === closing.spaceId);
    if (space) newTerminal(space);
  }

  function closeFocusedItem() {
    const target = focusedCloseTarget(activeTab(), activePaneId(), tabs().length);
    switch (target.kind) {
      case "pane":
        closePane(target.id);
        break;
      case "tab":
        closeTab(target.id);
        break;
      case "window":
        window.close();
        break;
    }
  }

  function restartPane(pane: Pane) {
    const space = spaces().find((candidate) => candidate.id === pane.spaceId);
    if (!space) return;
    const replacement = createPane(
      space,
      pane.tabId,
      {
        label: pane.label,
        program: pane.program,
        arguments: pane.arguments,
        ...(pane.environment ? { environment: pane.environment } : {}),
        ...(pane.requestedAgent ? { requestedAgent: pane.requestedAgent } : {}),
        initialPrompt: pane.initialPrompt,
      },
      pane.id,
    );
    terminalRefs.delete(pane.id);
    setPanes(((current: ReturnType<typeof panes>) =>
      current.map((candidate) =>
        candidate.id === pane.id ? replacement : candidate,
      ))(nativeUntrack(panes)),
    );
    focusPane(replacement.id);
  }

  function handleTerminalStatus(pane: Pane, event: QuickGuiEvent) {
    try {
      const status = terminalStatusFromEvent(event);
      if (status !== undefined) pane.setStatus(status);
    } catch (error) {
      pane.setStatus({
        status: "failed",
        title: "Terminal event error",
        workingDirectory:
          spaces().find((space) => space.id === pane.spaceId)?.path ?? "",
        message: error instanceof Error ? error.message : String(error),
      });
    }
  }

  async function addSpace() {
    if (addingSpace) return;
    addingSpace = true;
    try {
      const result = await Dialog.showOpenDialog({
        title: "Add a space",
        defaultPath: activeSpace().path,
        properties: ["openDirectory"],
      }, window);
      const path = result.filePaths[0];
      if (result.canceled || !path) return;
      const existing = spaces().find((space) => space.path === path);
      if (existing) {
        selectSpace(existing);
        return;
      }
      const space = { id: path, name: spaceName(path), path };
      setSpaces(((current: ReturnType<typeof spaces>) => [...current, space])(nativeUntrack(spaces)));
      newTerminal(space);
      schedulePersist();
    } finally {
      addingSpace = false;
    }
  }

  function removeSpace(space: Space) {
    if (spaces().length === 1) return;
    const remainingSpaces = spaces().filter(
      (candidate) => candidate.id !== space.id,
    );
    const closingTabIds = new Set(
      tabs()
        .filter((tab) => tab.spaceId === space.id)
        .map((tab) => tab.id),
    );
    const closingPaneIds = new Set(
      panes()
        .filter((pane) => pane.spaceId === space.id)
        .map((pane) => pane.id),
    );
    setSpaces(remainingSpaces);
    setTabs(((current: ReturnType<typeof tabs>) => current.filter((tab) => !closingTabIds.has(tab.id)))(nativeUntrack(tabs)));
    setPanes(((current: ReturnType<typeof panes>) =>
      current.filter((pane) => !closingPaneIds.has(pane.id)))(nativeUntrack(panes)),
    );
    for (const id of closingPaneIds) terminalRefs.delete(id);
    lastActiveTabBySpace.delete(space.id);
    if (activeSpaceId() === space.id) selectSpace(remainingSpaces[0]!);
    schedulePersist();
  }

  function setThemePreference(preference: AppearancePreference) {
    setAppearancePreference(preference);
    window.setAppearance(preference);
    if (preference !== "system") setAppearance(preference);
    installMenu(preference);
    schedulePersist();
  }

  function toggleTheme() {
    setThemePreference(appearance() === "dark" ? "light" : "dark");
  }

  function handleSidebarPointer(event: QuickGuiEvent) {
    const pointer = capturedPointerFromEvent(event);
    if (pointer === undefined) return;
    if (pointer.button !== "left") return;
    if (pointer.phase === "down") {
      sidebarDragStart = sidebarWidth();
      return;
    }
    if (pointer.phase === "move") {
      setSidebarWidth(
        clamp(
          sidebarDragStart + pointer.position.x - pointer.origin.x,
          196,
          380,
        ),
      );
      return;
    }
    schedulePersist();
  }

  function handleSidebarSectionPointer(event: QuickGuiEvent) {
    const pointer = capturedPointerFromEvent(event);
    if (pointer === undefined) return;
    if (pointer.button !== "left") return;
    if (pointer.phase === "down") {
      sectionDragStart = sidebarSectionRatio();
      return;
    }
    if (pointer.phase === "move") {
      setSidebarSectionRatio(
        clamp(
          sectionDragStart +
            (pointer.position.y - pointer.origin.y) / sectionDragHeight,
          0.25,
          0.75,
        ),
      );
      return;
    }
    schedulePersist();
  }

  function schedulePersist() {
    if (!stateFile) return;
    if (persistenceTimer) clearTimeout(persistenceTimer);
    persistenceTimer = setTimeout(() => {
      persistenceTimer = undefined;
      const state: SavedState = {
        spaces: spaces().map((space) => ({ path: space.path })),
        activeSpaceId: activeSpaceId(),
        sidebarWidth: sidebarWidth(),
        sidebarSectionRatio: sidebarSectionRatio(),
        appearance: appearancePreference(),
      };
      void writeSavedState(stateFile, state);
    }, 180);
  }

  function installMenu(preference: AppearancePreference) {
    Menu.setApplicationMenu([
      {
        label: "Herdr GUI",
        items: [
          { label: "About Herdr GUI", enabled: false },
          { type: "separator" },
          { type: "role", label: "Quit Herdr GUI", role: "quit" },
        ],
      },
      {
        label: "File",
        items: [
          { label: "New Agent…", click: openAgentSheet },
          { label: "New Tab", click: () => newTerminal(undefined) },
          { label: "Add Space…", click: () => void addSpace() },
          { type: "separator" },
          { label: "Split Right", click: () => splitTerminal("horizontal") },
          { label: "Split Down", click: () => splitTerminal("vertical") },
          { type: "separator" },
          {
            label: "Close",
            role: "close-window",
            click: closeFocusedItem,
          },
        ],
      },
      {
        label: "Edit",
        items: [
          { type: "role", label: "Copy", role: "copy" },
          { type: "role", label: "Paste", role: "paste" },
          { type: "role", label: "Select All", role: "select-all" },
        ],
      },
      {
        label: "View",
        items: [
          {
            label: "System Appearance",
            checked: preference === "system",
            mark: "radio",
            click: () => setThemePreference("system"),
          },
          {
            label: "Light Appearance",
            checked: preference === "light",
            mark: "radio",
            click: () => setThemePreference("light"),
          },
          {
            label: "Dark Appearance",
            checked: preference === "dark",
            mark: "radio",
            click: () => setThemePreference("dark"),
          },
        ],
      },
      {
        label: "Window",
        items: [
          { type: "role", label: "Minimize", role: "minimize-window" },
          { type: "role", label: "Zoom", role: "zoom-window" },
          {
            type: "role",
            label: "Enter Full Screen",
            role: "toggle-fullscreen",
          },
        ],
      },
    ]);
  }

  function registerTerminal(id: number, node: NativeNode) {
    terminalRefs.set(id, node);
    queueMicrotask(() => {
      if (activePaneId() === id) node.focus();
    });
  }

  installMenu(initialAppearance);

  const model: HerdrModel = {
    homePath,
    appearance,
    sidebarWidth,
    sidebarSectionRatio,
    spaces,
    tabs,
    panes,
    activeSpaceId,
    activeTabId,
    activePaneId,
    activePane,
    activeTab,
    activeSpace,
    activeSpaceTabs,
    visibleAgents,
    agentSheetOpen,
    selectedLauncherId,
    selectedLauncher,
    initialPrompt,
    catalogLoading,
    launchers,
    theme,
    styles,
    selectSpace,
    selectTab,
    selectPane,
    removeSpace,
    addSpace,
    newTerminal,
    splitTerminal,
    openAgentSheet,
    closeAgentSheet: () => setAgentSheetOpen(false),
    selectLauncher: setSelectedLauncherId,
    setInitialPrompt,
    launchAgent,
    closePane,
    closeTab,
    restartPane,
    toggleTheme,
    handleSidebarPointer,
    handleSidebarSectionPointer,
    handleTerminalStatus,
    registerTerminal,
  };

  return model;
}
