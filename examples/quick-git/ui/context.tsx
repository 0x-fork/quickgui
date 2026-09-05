import type { Window } from "@quickgui/native";
import { createContext, useContext, type Accessor } from "solid-js";

import type { Store } from "../model/store.ts";
import type { Styles, Theme } from "./theme.ts";

export type DialogRequest =
  | { kind: "new-branch"; from?: string }
  | { kind: "new-worktree"; branch?: string }
  | { kind: "stash" };

export interface AppContext {
  store: Store;
  window: Window;
  theme: Accessor<Theme>;
  styles: Accessor<Styles>;
  dialog: Accessor<DialogRequest | undefined>;
  openDialog: (request: DialogRequest) => void;
  closeDialog: () => void;
}

const Context = createContext<AppContext>();

export const AppProvider = Context;

export function useApp(): AppContext {
  const value = useContext(Context);
  if (!value) throw new Error("useApp must run inside AppProvider");
  return value;
}
