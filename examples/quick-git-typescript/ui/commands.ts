/**
 * Commands the native menu bar and keyboard accelerators dispatch into the mounted UI.
 *
 * Menu callbacks run outside any component, and every window mounts its own shell, so each shell
 * registers its handlers for its window and the menu routes to the window that was focused last.
 */

import type { Window } from "@quickgui/native";

import type { DialogRequest } from "./context.tsx";

export interface Commands {
  openRepository(): void;
  openDialog(request: DialogRequest): void;
  focusCommitMessage(): void;
}

const registry = new Map<Window, Commands>();
let active: Window | undefined;

export function registerCommands(window: Window, commands: Commands): () => void {
  registry.set(window, commands);
  if (!active || active.closed) active = window;
  return () => {
    if (registry.get(window) === commands) registry.delete(window);
    if (active === window) active = undefined;
  };
}

/** Route menu commands to `window` from now on; call when it gains focus. */
export function activateCommands(window: Window): void {
  active = window;
}

export function commands(): Commands | undefined {
  if (active && !active.closed) {
    const current = registry.get(active);
    if (current) return current;
  }
  return [...registry.values()].at(-1);
}
