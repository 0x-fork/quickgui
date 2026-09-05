import { Menu, Shell } from "@quickgui/native";
import { Text, View } from "@quickgui/solid";
import { Button as SwiftButton, Host } from "@quickgui/solid/swift-ui";
import { buttonStyle, disabled } from "@quickgui/solid/swift-ui/modifiers";
import { For, Show } from "solid-js";
import { basename, dirname } from "node:path";
import { homedir } from "node:os";

import type { Worktree } from "../git/worktree.ts";
import { Tag } from "./branches.tsx";
import { useApp } from "./context.tsx";
import { confirm } from "./dialogs.tsx";
import { Icon } from "./icons.tsx";
import { EmptyState, PushButton } from "./primitives.tsx";
import { RowButton } from "./shell.tsx";
import { copyText, openInTerminal, runInTerminal } from "./sidebar.tsx";

export function WorktreesView() {
  const app = useApp();
  const store = app.store;
  const home = homedir();
  const shorten = (path: string) => (path.startsWith(home) ? `~${path.slice(home.length)}` : path);

  async function remove(worktree: Worktree): Promise<void> {
    const ok = await confirm(app, {
      message: `Remove worktree ${basename(worktree.path)}?`,
      detail: `The directory ${shorten(worktree.path)} will be deleted. Uncommitted changes in it will be lost; the branch itself is kept.`,
      confirmLabel: "Remove",
      destructive: true,
    });
    if (ok) await store.removeWorktree(worktree.path, true);
  }

  function menu(worktree: Worktree): void {
    const agents = store.agents();
    void Menu.popup(
      [
        { label: "Open in Quick Git", enabled: store.repository()?.root !== worktree.path, click: () => void store.selectWorktree(worktree.path) },
        { label: "Reveal in Finder", click: () => void Shell.showItemInFolder(worktree.path) },
        { label: "Open in Terminal", click: () => void openInTerminal(worktree.path) },
        ...agents.map((agent) => ({ label: `Launch ${agent.label} Here`, click: () => void runInTerminal(worktree.path, agent.command) })),
        { type: "separator" },
        { label: "Copy Path", click: () => void copyText(worktree.path) },
        { type: "separator" },
        { label: "Remove Worktree…", enabled: !worktree.main, click: () => void remove(worktree) },
      ],
      { window: app.window },
    );
  }

  return (
    <View style={{ display: "flex", flex: 1, minWidth: 0, minHeight: 0, flexDirection: "column" }}>
      <View style={{ display: "flex", flexDirection: "row", alignItems: "center", gap: 8, height: 44, flexShrink: 0, paddingLeft: 16, paddingRight: 12, borderBottomWidth: 1, borderColor: app.theme().border }}>
        <Text style={{ fontSize: 14, fontWeight: 700, color: app.theme().text }}>Worktrees</Text>
        <Text style={{ fontSize: 12, color: app.theme().textTertiary }}>Parallel checkouts of this repository, one directory each.</Text>
        <View style={{ flex: 1 }} />
        <Host matchContents>
          <SwiftButton label="New Worktree…" systemImage="plus" modifiers={[buttonStyle("borderedProminent")]} onPress={() => app.openDialog({ kind: "new-worktree" })} />
        </Host>
      </View>
      <View style={{ display: "flex", flex: 1, minHeight: 0, flexDirection: "column", gap: 2, overflowY: "auto", padding: 8 }}>
        <Show when={store.worktrees().length > 0} fallback={<EmptyState ui={app} icon="worktree" title="No worktrees" />}>
          <For each={store.worktrees()}>
            {(worktree) => (
              <RowButton
                label={worktree.path}
                selected={store.repository()?.root === worktree.path}
                onClick={() => void store.selectWorktree(worktree.path)}
                onContextMenu={() => menu(worktree)}
                height={54}
              >
                <View
                  style={{
                    display: "flex",
                    width: 34,
                    height: 34,
                    flexShrink: 0,
                    alignItems: "center",
                    justifyContent: "center",
                    borderRadius: 9,
                    backgroundColor: worktree.main ? app.theme().accentWash : app.theme().contentAlt,
                    color: worktree.main ? app.theme().accent : app.theme().textSecondary,
                  }}
                >
                  <Icon name={worktree.main ? "folder" : "worktree"} size={17} />
                </View>
                <View style={{ display: "flex", flex: 1, minWidth: 0, flexDirection: "column", gap: 3 }}>
                  <View style={{ display: "flex", flexDirection: "row", alignItems: "center", gap: 6 }}>
                    <Text style={{ fontSize: 13, fontWeight: 700, color: app.theme().text, lineClamp: 1, textOverflow: "ellipsis", minWidth: 0 }}>{basename(worktree.path)}</Text>
                    <Show when={worktree.main}>
                      <Tag label="main" accent />
                    </Show>
                    <Show when={worktree.locked}>
                      <Tag label={worktree.lockReason ? `locked: ${worktree.lockReason}` : "locked"} warn />
                    </Show>
                    <Show when={worktree.prunable}>
                      <Tag label="prunable" warn />
                    </Show>
                  </View>
                  <View style={{ display: "flex", flexDirection: "row", alignItems: "center", gap: 6 }}>
                    <Icon name="branch" size={11} color={app.theme().textTertiary} />
                    <Text style={{ fontSize: 11.5, color: app.theme().textSecondary, fontWeight: 600 }}>
                      {worktree.branchName ?? (worktree.detached ? `detached at ${worktree.headSha?.slice(0, 7) ?? "?"}` : "bare")}
                    </Text>
                    <Text style={{ fontSize: 11.5, color: app.theme().textTertiary, lineClamp: 1, textOverflow: "ellipsis", minWidth: 0, fontFamily: "monospace" }}>{shorten(dirname(worktree.path))}/</Text>
                  </View>
                </View>
                <View style={{ display: "flex", flexDirection: "row", alignItems: "center", gap: 4, flexShrink: 0, opacity: 0, groupHover: { opacity: 1 } }}>
                  <PushButton ui={app} icon="terminal" label="Terminal" onClick={() => void openInTerminal(worktree.path)} style={{ height: 24, fontSize: 11.5 }} />
                  <For each={store.agents()}>
                    {(agent) => <PushButton ui={app} icon="sparkles" label={agent.label} onClick={() => void runInTerminal(worktree.path, agent.command)} style={{ height: 24, fontSize: 11.5 }} />}
                  </For>
                  <Show when={!worktree.main}>
                    <PushButton ui={app} icon="trash" kind="danger" label="Remove" onClick={() => void remove(worktree)} style={{ height: 24, fontSize: 11.5 }} />
                  </Show>
                </View>
              </RowButton>
            )}
          </For>
        </Show>
      </View>
    </View>
  );
}
