import type { Window, NativeNode } from "@quickgui/native";
import { createContext, useContext, type Accessor } from "@quickgui/ui";

import type { Store } from "../model/store.ts";
import type { Styles, Theme } from "./theme.ts";

export interface DialogRequest {
  kind: "new-branch" | "new-worktree" | "stash";
  from?: string;
  branch?: string;
}

export interface AppContext {
  store: Store;
  window: Window;
  theme: Accessor<Theme>;
  styles: Accessor<Styles>;
  dialog: Accessor<DialogRequest | undefined>;
  openDialog: (request: DialogRequest) => void;
  closeDialog: () => void;
  /** Ask for a repository folder and open it: in this window if it shows none, else in a new one. */
  openRepository: () => Promise<void>;
  /** Show `path`: focus the window that already has it, fill this window if empty, else open one. */
  openRepositoryPath: (path: string) => Promise<void>;
}

const Context = createContext<AppContext | undefined>(undefined);

export function AppProvider(props: { value: AppContext; children: () => NativeNode }): NativeNode {
  return Context.provide(props.value, props.children);
}

export function useApp(): AppContext {
  const value = useContext(Context);
  if (!value) throw new Error("useApp must run inside AppProvider");
  return value;
}
