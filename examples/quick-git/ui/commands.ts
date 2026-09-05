/**
 * Commands the native menu bar and keyboard accelerators dispatch into the mounted UI.
 *
 * Menu callbacks run outside any component, so the shell registers its handlers here once it is
 * mounted and withdraws them when it unmounts.
 */

import type { DialogRequest } from "./context.tsx";

export interface Commands {
  openRepository(): void;
  openDialog(request: DialogRequest): void;
  focusCommitMessage(): void;
}

let current: Commands | undefined;

export function registerCommands(commands: Commands): () => void {
  current = commands;
  return () => {
    if (current === commands) current = undefined;
  };
}

export function commands(): Commands | undefined {
  return current;
}
