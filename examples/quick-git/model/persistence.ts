import { mkdir, readFile, rename, rm, writeFile } from "node:fs/promises";
import { dirname } from "node:path";

import type { AgentId } from "../agent/commit-message.ts";

export interface PersistedState {
  recentRepositories: string[];
  lastRepository?: string | undefined;
  sidebarWidth: number;
  changesSplit: number;
  historySplit: number;
  preferredAgent?: AgentId;
  historyAllBranches: boolean;
}

const STATE_VERSION = 1;
export const MAX_RECENT_REPOSITORIES = 20;

export const DEFAULT_STATE: PersistedState = {
  recentRepositories: [],
  sidebarWidth: 236,
  changesSplit: 320,
  historySplit: 420,
  historyAllBranches: false,
};

export function parsePersistedState(value: unknown): PersistedState {
  if (typeof value !== "object" || value === null || Array.isArray(value)) return { ...DEFAULT_STATE };
  const record = value as Record<string, unknown>;
  if (record.version !== STATE_VERSION) return { ...DEFAULT_STATE };
  const recent = Array.isArray(record.recentRepositories)
    ? record.recentRepositories.filter((entry): entry is string => typeof entry === "string" && entry.length > 0)
    : [];
  return {
    recentRepositories: [...new Set(recent)].slice(0, MAX_RECENT_REPOSITORIES),
    ...(typeof record.lastRepository === "string" ? { lastRepository: record.lastRepository } : {}),
    sidebarWidth: clampNumber(record.sidebarWidth, 180, 420, DEFAULT_STATE.sidebarWidth),
    changesSplit: clampNumber(record.changesSplit, 220, 800, DEFAULT_STATE.changesSplit),
    historySplit: clampNumber(record.historySplit, 260, 1000, DEFAULT_STATE.historySplit),
    ...(record.preferredAgent === "codex" || record.preferredAgent === "claude"
      ? { preferredAgent: record.preferredAgent }
      : {}),
    historyAllBranches: record.historyAllBranches === true,
  };
}

function clampNumber(value: unknown, min: number, max: number, fallback: number): number {
  return typeof value === "number" && Number.isFinite(value) ? Math.min(max, Math.max(min, value)) : fallback;
}

export async function loadPersistedState(path: string | undefined): Promise<PersistedState> {
  if (!path) return { ...DEFAULT_STATE };
  try {
    return parsePersistedState(JSON.parse(await readFile(path, "utf8")));
  } catch {
    return { ...DEFAULT_STATE };
  }
}

export async function savePersistedState(path: string, state: PersistedState): Promise<void> {
  await mkdir(dirname(path), { recursive: true });
  const temporary = `${path}.${process.pid}.tmp`;
  await writeFile(temporary, `${JSON.stringify({ version: STATE_VERSION, ...state }, null, 2)}\n`, "utf8");
  try {
    await rename(temporary, path);
  } catch (error) {
    await rm(temporary, { force: true });
    throw error;
  }
}

/** Move `path` to the front of the recent list, bounded. */
export function rememberRepository(recent: readonly string[], path: string): string[] {
  return [path, ...recent.filter((entry) => entry !== path)].slice(0, MAX_RECENT_REPOSITORIES);
}
