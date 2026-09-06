import { mkdir, readFile, writeFile } from "node:fs/promises";
import { dirname } from "node:path";

import type { SavedState, Space } from "../model.ts";
import { spaceName } from "../model.ts";

export function normalizeSpaces(
  saved: SavedState["spaces"],
  fallbackPath: string,
): Space[] {
  const paths = [
    ...(saved ?? [])
      .map((space) => space.path)
      .filter((path) => typeof path === "string"),
    fallbackPath,
  ];
  const seen = new Set<string>();
  return paths
    .filter((path): boolean => {
      if (path.length === 0 || seen.has(path)) return false;
      seen.add(path);
      return true;
    })
    .map((path) => ({ id: path, name: spaceName(path), path }));
}

export async function readSavedState(
  path: string | undefined,
): Promise<SavedState> {
  if (!path) return {};
  try {
    const candidate = JSON.parse(await readFile(path, "utf8")) as SavedState;
    return {
      ...(Array.isArray(candidate.spaces) ? { spaces: candidate.spaces } : {}),
      ...(typeof candidate.activeSpaceId === "string"
        ? { activeSpaceId: candidate.activeSpaceId }
        : {}),
      ...(typeof candidate.sidebarWidth === "number"
        ? { sidebarWidth: candidate.sidebarWidth }
        : {}),
      ...(typeof candidate.sidebarSectionRatio === "number"
        ? { sidebarSectionRatio: candidate.sidebarSectionRatio }
        : {}),
      ...(candidate.appearance === "system" ||
      candidate.appearance === "light" ||
      candidate.appearance === "dark"
        ? { appearance: candidate.appearance }
        : {}),
    };
  } catch {
    return {};
  }
}

export async function writeSavedState(
  path: string,
  state: SavedState,
): Promise<void> {
  try {
    await mkdir(dirname(path), { recursive: true });
    await writeFile(path, JSON.stringify(state, null, 2) + "\n", "utf8");
  } catch {
    // Persistence is optional; a write failure must not interrupt a PTY session.
  }
}
