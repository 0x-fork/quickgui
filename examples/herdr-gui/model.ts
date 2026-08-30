import { basename } from "node:path";

import type {
  AppearanceMode,
  AppearancePreference,
  NativeNode,
  QuickGuiEvent,
} from "@quickgui/native";
import type { TerminalStatusEvent } from "@quickgui/solid";

import type { AppStyles, Theme } from "./theme.ts";

export type Getter<T> = () => T;
export type AgentLauncherId = "codex" | "claude" | "opencode";
export type AgentStatus = NonNullable<TerminalStatusEvent["agentStatus"]>;
export type SplitDirection = "horizontal" | "vertical";

export interface AgentLauncher {
  id: AgentLauncherId;
  label: string;
  mark: string;
  description: string;
  command: string;
  executable?: string;
  installed: boolean;
  argumentsFor(prompt: string): string[];
}

/** A Herdr space: one project directory containing tabs and terminal panes. */
export interface Space {
  id: string;
  name: string;
  path: string;
}

/** A tab owns a tiled set of live panes. */
export interface WorkspaceTab {
  id: number;
  spaceId: string;
  number: number;
  customName?: string;
  paneIds: number[];
  activePaneId: number;
  splitDirection: SplitDirection;
}

/** A pane owns one real core PTY. Agent identity is never declared here. */
export interface Pane {
  id: number;
  tabId: number;
  spaceId: string;
  label: string;
  program: string;
  arguments: string[];
  environment?: Readonly<Record<string, string>>;
  requestedAgent?: AgentLauncherId;
  initialPrompt: string;
  status: Getter<TerminalStatusEvent>;
  setStatus(status: TerminalStatusEvent): void;
}

export interface SavedState {
  spaces?: Array<{ path: string }>;
  activeSpaceId?: string;
  sidebarWidth?: number;
  sidebarSectionRatio?: number;
  appearance?: AppearancePreference;
}

export interface HerdrModel {
  homePath: string;
  appearance: Getter<AppearanceMode>;
  sidebarWidth: Getter<number>;
  sidebarSectionRatio: Getter<number>;
  spaces: Getter<Space[]>;
  tabs: Getter<WorkspaceTab[]>;
  panes: Getter<Pane[]>;
  activeSpaceId: Getter<string>;
  activeTabId: Getter<number | undefined>;
  activePaneId: Getter<number | undefined>;
  activePane: Getter<Pane | undefined>;
  activeTab: Getter<WorkspaceTab | undefined>;
  activeSpace: Getter<Space>;
  activeSpaceTabs: Getter<WorkspaceTab[]>;
  visibleAgents: Getter<Pane[]>;
  agentSheetOpen: Getter<boolean>;
  selectedLauncherId: Getter<AgentLauncherId>;
  selectedLauncher: Getter<AgentLauncher>;
  initialPrompt: Getter<string>;
  catalogLoading: Getter<boolean>;
  launchers: Getter<AgentLauncher[]>;
  theme: Getter<Theme>;
  styles: Getter<AppStyles>;
  selectSpace(space: Space): void;
  selectTab(tab: WorkspaceTab): void;
  selectPane(pane: Pane): void;
  removeSpace(space: Space): void;
  addSpace(): Promise<void>;
  newTerminal(space?: Space): void;
  splitTerminal(direction: SplitDirection): void;
  openAgentSheet(): void;
  closeAgentSheet(): void;
  selectLauncher(id: AgentLauncherId): void;
  setInitialPrompt(value: string): void;
  launchAgent(): void;
  closePane(id: number): void;
  closeTab(id: number): void;
  restartPane(pane: Pane): void;
  toggleTheme(): void;
  handleSidebarPointer(event: QuickGuiEvent): void;
  handleSidebarSectionPointer(event: QuickGuiEvent): void;
  handleTerminalStatus(pane: Pane, event: QuickGuiEvent): void;
  registerTerminal(id: number, node: NativeNode): void;
}

export type FocusedCloseTarget =
  | { kind: "pane"; id: number }
  | { kind: "tab"; id: number }
  | { kind: "window" };

export function focusedCloseTarget(
  tab: WorkspaceTab | undefined,
  paneId: number | undefined,
  tabCount: number,
): FocusedCloseTarget {
  if (
    tab &&
    paneId !== undefined &&
    tab.paneIds.length > 1 &&
    tab.paneIds.includes(paneId)
  ) {
    return { kind: "pane", id: paneId };
  }
  if (tab && tabCount > 1) return { kind: "tab", id: tab.id };
  return { kind: "window" };
}

export function paneTitle(pane: Pane): string {
  const status = pane.status();
  const detected = status.agent;
  const title = status.title.trim();
  if (
    title &&
    !genericTerminalTitle(title, status.workingDirectory, detected)
  ) {
    return title;
  }
  if (detected) return agentLabel(detected);
  if (pane.requestedAgent) return agentLabel(pane.requestedAgent);
  return pane.label;
}

export function tabTitle(
  tab: WorkspaceTab,
  panes: readonly Pane[],
): string {
  if (tab.customName) return tab.customName;
  const pane =
    panes.find((candidate) => candidate.id === tab.activePaneId) ??
    panes.find((candidate) => tab.paneIds.includes(candidate.id));
  return pane ? paneTitle(pane) : String(tab.number);
}

function genericTerminalTitle(
  title: string,
  workingDirectory: string | null,
  agent: string | undefined,
): boolean {
  const normalized = title.toLowerCase();
  if (
    [
      "zsh",
      "bash",
      "sh",
      "fish",
      "xterm",
      "terminal",
      agent?.toLowerCase(),
    ].includes(normalized)
  ) {
    return true;
  }
  if (!workingDirectory) return false;
  const directory = normalizeTerminalPath(workingDirectory);
  return (
    title === workingDirectory ||
    title === directory ||
    title === basename(directory)
  );
}

export function agentLabel(agent: string | undefined): string {
  switch (agent) {
    case "claude":
      return "Claude Code";
    case "codex":
      return "Codex";
    case "opencode":
      return "OpenCode";
    case "copilot":
      return "GitHub Copilot";
    case "gemini":
      return "Gemini";
    case "cursor":
      return "Cursor";
    case "agy":
      return "Antigravity";
    case "mastracode":
      return "Mastra Code";
    case "qodercli":
      return "Qoder CLI";
    case undefined:
      return "Agent";
    default:
      return agent.charAt(0).toUpperCase() + agent.slice(1);
  }
}

export function spaceForPane(pane: Pane, spaces: readonly Space[]): Space {
  return spaces.find((space) => space.id === pane.spaceId) ?? spaces[0]!;
}

export function spaceName(path: string): string {
  return basename(path) || path;
}

export function normalizeTerminalPath(path: string): string {
  if (!path.startsWith("file://")) return path;
  try {
    return decodeURIComponent(new URL(path).pathname);
  } catch {
    return path;
  }
}

export function shortPath(path: string, homePath: string): string {
  const normalized = normalizeTerminalPath(path);
  if (normalized === homePath) return "~";
  if (normalized.startsWith(homePath + "/")) {
    return "~" + normalized.slice(homePath.length);
  }
  return normalized;
}

export function clamp(value: number, minimum: number, maximum: number): number {
  return Math.min(Math.max(value, minimum), maximum);
}
