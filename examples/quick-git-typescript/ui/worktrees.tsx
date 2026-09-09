import { Menu, Shell } from "@quickgui/native";
import { Text, View } from "@quickgui/solid";
import { For } from "solid-js";
import { basename } from "node:path";
import type { Worktree } from "../git/worktree.ts";
import { useApp } from "./context.tsx";
import { confirm } from "./dialogs.tsx";
import { PushButton } from "./primitives.tsx";
import { RowButton } from "./shell.tsx";
import { copyText, openInTerminal, runInTerminal } from "./sidebar.tsx";

export function WorktreesView() {
  const app = useApp();
  const store = app.store;
  async function remove(worktree: Worktree) {
    if (
      await confirm(app, {
        message: `Remove worktree ${basename(worktree.path)}?`,
        detail:
          "The checkout directory will be removed. The branch is kept; Git will refuse removal if there are uncommitted changes.",
        confirmLabel: "Remove",
        destructive: true,
      })
    )
      await store.removeWorktree(worktree.path, false);
  }
  function menu(worktree: Worktree) {
    void Menu.popup(
      [
        {
          label: "Open in Quick Git",
          enabled: store.repository()?.root !== worktree.path,
          click: () => void store.selectWorktree(worktree.path),
        },
        { label: "Reveal in Finder", click: () => void Shell.showItemInFolder(worktree.path) },
        { label: "Open in Terminal", click: () => void openInTerminal(worktree.path) },
        ...store
          .agents()
          .map((agent) => ({
            label: `Launch ${agent.label} Here`,
            click: () => void runInTerminal(worktree.path, agent.command),
          })),
        { label: "Copy Path", click: () => void copyText(worktree.path) },
        ...(!worktree.main
          ? [
              { type: "separator" as const },
              { label: "Remove Worktree…", click: () => void remove(worktree) },
            ]
          : []),
      ],
      { window: app.window },
    );
  }
  return (
    <View style={app.styles().listPanel}>
      <View style={app.styles().listHeader}>
        <Text style={app.styles().listTitle}>Worktrees</Text>
        <PushButton
          ui={app}
          kind="primary"
          label="New Worktree"
          onClick={() => app.openDialog({ kind: "new-worktree" })}
        />
      </View>
      <For each={store.worktrees()} keyed={(worktree) => worktree.path}>
        {(worktree) => (
          <RowButton
            label={worktree().path}
            selected={store.repository()?.root === worktree().path}
            onClick={() => void store.selectWorktree(worktree().path)}
            onContextMenu={() => menu(worktree())}
          >
            <Text style={{ flex: 1, minWidth: 0, lineClamp: 1 }}>
              {worktree().branchName || basename(worktree().path)}
            </Text>
            <Text style={{ fontSize: 11, color: app.theme().textTertiary }}>
              {basename(worktree().path)}
            </Text>
          </RowButton>
        )}
      </For>
    </View>
  );
}
