import { Menu } from "@quickgui/native";
import { Text, View } from "@quickgui/solid";
import { For, Show } from "solid-js";
import { basename } from "node:path";
import type { BranchRef } from "../git/refs.ts";
import { useApp } from "./context.tsx";
import { confirm } from "./dialogs.tsx";
import { PushButton } from "./primitives.tsx";
import { RowButton } from "./shell.tsx";
import { copyText } from "./sidebar.tsx";

export function BranchesView() {
  const app = useApp();
  const store = app.store;
  function switchBranch(branch: BranchRef) {
    if (branch.current || branch.remote) return;
    if (branch.worktreePath && branch.worktreePath !== store.repository()?.root)
      void store.selectWorktree(branch.worktreePath);
    else void store.switchBranch(branch.name);
  }
  async function deleteBranch(branch: BranchRef) {
    if (
      await confirm(app, {
        message: `Delete branch ${branch.name}?`,
        detail: "Unmerged commits on this branch will be lost.",
        confirmLabel: "Delete",
        destructive: true,
      })
    )
      await store.deleteBranch(branch.name, true);
  }
  function menu(branch: BranchRef) {
    const elsewhere = branch.worktreePath && branch.worktreePath !== store.repository()?.root;
    void Menu.popup(
      [
        {
          label: elsewhere
            ? `Open Worktree (${basename(branch.worktreePath!)})`
            : "Switch to Branch",
          enabled: !branch.current && !branch.remote,
          click: () => switchBranch(branch),
        },
        {
          label: "New Branch from Here…",
          click: () => app.openDialog({ kind: "new-branch", from: branch.name }),
        },
        {
          label: "New Worktree for Branch…",
          enabled: !branch.remote && !branch.worktreePath,
          click: () => app.openDialog({ kind: "new-worktree", branch: branch.name }),
        },
        { label: "Copy Branch Name", click: () => void copyText(branch.name) },
        { type: "separator" },
        {
          label: "Delete Branch…",
          enabled: !branch.current && !branch.remote && !branch.worktreePath,
          click: () => void deleteBranch(branch),
        },
      ],
      { window: app.window },
    );
  }
  const BranchRow = (props: { branch: BranchRef }) => (
    <RowButton
      label={props.branch.name}
      selected={props.branch.current}
      onClick={() => {}}
      onDoubleClick={() => switchBranch(props.branch)}
      onContextMenu={() => menu(props.branch)}
    >
      <Text
        style={{
          flex: 1,
          minWidth: 0,
          fontWeight: props.branch.current ? 700 : 500,
          lineClamp: 1,
          color: props.branch.remote ? app.theme().textSecondary : app.theme().text,
        }}
      >
        {props.branch.name}
      </Text>
      <Show when={props.branch.current}>
        <Text style={{ fontSize: 11, color: app.theme().accent }}>current</Text>
      </Show>
      <Text style={{ fontSize: 11, color: app.theme().textTertiary }}>{props.branch.shortSha}</Text>
    </RowButton>
  );
  return (
    <View style={app.styles().listPanel}>
      <View style={app.styles().listHeader}>
        <Text style={app.styles().listTitle}>Local branches</Text>
        <PushButton
          ui={app}
          kind="primary"
          label="New Branch"
          onClick={() => app.openDialog({ kind: "new-branch" })}
        />
      </View>
      <For each={store.refs().local} keyed={(branch) => branch.fullName}>
        {(branch) => <BranchRow branch={branch()} />}
      </For>
      <Show when={store.refs().remote.length > 0}>
        <Text style={[app.styles().listTitle, { flex: "none", marginTop: 16 }]}>
          Remote branches
        </Text>
        <For each={store.refs().remote} keyed={(branch) => branch.fullName}>
          {(branch) => <BranchRow branch={branch()} />}
        </For>
      </Show>
      <Show when={store.refs().tags.length > 0}>
        <Text style={[app.styles().listTitle, { flex: "none", marginTop: 16 }]}>Tags</Text>
        <For each={store.refs().tags} keyed={(tag) => tag.name}>
          {(tag) => (
            <RowButton
              label={tag().name}
              onClick={() => {}}
              onContextMenu={() =>
                void Menu.popup(
                  [
                    {
                      label: "New Branch from Tag…",
                      click: () => app.openDialog({ kind: "new-branch", from: tag().name }),
                    },
                    { label: "Copy Tag Name", click: () => void copyText(tag().name) },
                  ],
                  { window: app.window },
                )
              }
            >
              <Text style={{ flex: 1, minWidth: 0, lineClamp: 1 }}>{tag().name}</Text>
              <Text style={{ fontSize: 11, color: app.theme().textTertiary }}>
                {tag().sha.slice(0, 7)}
              </Text>
            </RowButton>
          )}
        </For>
      </Show>
    </View>
  );
}
