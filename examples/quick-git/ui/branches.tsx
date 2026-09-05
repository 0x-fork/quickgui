import { Menu } from "@quickgui/native";
import { Text, View } from "@quickgui/solid";
import { Button as SwiftButton, Host } from "@quickgui/solid/swift-ui";
import { buttonStyle, disabled } from "@quickgui/solid/swift-ui/modifiers";
import { For, Show, createMemo, createSignal } from "solid-js";
import { basename } from "node:path";

import { relativeTime } from "../git/log.ts";
import type { BranchRef } from "../git/refs.ts";
import { useApp } from "./context.tsx";
import { confirm } from "./dialogs.tsx";
import { Icon } from "./icons.tsx";
import { EmptyState, PushButton } from "./primitives.tsx";
import { ListHeading, RowButton } from "./shell.tsx";
import { copyText } from "./sidebar.tsx";

export function BranchesView() {
  const app = useApp();
  const store = app.store;
  const [selected, setSelected] = createSignal<string>();
  const [showRemotes, setShowRemotes] = createSignal(false);
  const local = createMemo(() => store.refs().local);
  const remote = createMemo(() => store.refs().remote);
  const tags = createMemo(() => store.refs().tags);

  async function deleteBranch(branch: BranchRef): Promise<void> {
    const merged = branch.ahead === 0 && Boolean(branch.upstream) && !branch.upstreamGone;
    const ok = await confirm(app, {
      message: `Delete branch ${branch.name}?`,
      detail: merged ? "The branch can be recreated from its upstream." : "Unmerged commits on this branch will be lost.",
      confirmLabel: "Delete",
      destructive: true,
    });
    if (ok) await store.deleteBranch(branch.name, true);
  }

  function menu(branch: BranchRef): void {
    const checkedOutElsewhere = branch.worktreePath && branch.worktreePath !== store.repository()?.root;
    void Menu.popup(
      [
        checkedOutElsewhere
          ? { label: `Open Worktree (${basename(branch.worktreePath!)})`, click: () => void store.selectWorktree(branch.worktreePath!) }
          : { label: "Switch to Branch", enabled: !branch.current && !branch.remote, click: () => void store.switchBranch(branch.name) },
        { label: "New Branch from Here…", click: () => app.openDialog({ kind: "new-branch", from: branch.name }) },
        { label: "New Worktree for Branch…", enabled: !branch.remote && !branch.worktreePath, click: () => app.openDialog({ kind: "new-worktree", branch: branch.name }) },
        { type: "separator" },
        { label: "Copy Branch Name", click: () => void copyText(branch.name) },
        { type: "separator" },
        { label: "Delete Branch…", enabled: !branch.current && !branch.remote && !branch.worktreePath, click: () => void deleteBranch(branch) },
      ],
      { window: app.window },
    );
  }

  const Row = (props: { branch: BranchRef }) => {
    const branch = () => props.branch;
    const elsewhere = () => branch().worktreePath && branch().worktreePath !== store.repository()?.root && !branch().current;
    return (
      <RowButton
        label={branch().name}
        selected={selected() === branch().fullName}
        onClick={() => setSelected(branch().fullName)}
        onDoubleClick={() => {
          if (branch().remote || branch().current) return;
          if (elsewhere()) void store.selectWorktree(branch().worktreePath!);
          else void store.switchBranch(branch().name);
        }}
        onContextMenu={() => menu(branch())}
        height={32}
      >
        <Icon name={branch().remote ? "cloud-down" : "branch"} size={14} color={branch().current ? app.theme().accent : app.theme().textTertiary} />
        <Text style={{ fontSize: 13, fontWeight: branch().current ? 700 : 500, color: app.theme().text, lineClamp: 1, textOverflow: "ellipsis", minWidth: 0, flexShrink: 1 }}>
          {branch().name}
        </Text>
        <Show when={branch().current}>
          <Tag label="current" accent />
        </Show>
        <Show when={elsewhere()}>
          <Tag label={basename(branch().worktreePath!)} />
        </Show>
        <Show when={branch().upstreamGone}>
          <Tag label="upstream gone" warn />
        </Show>
        <Show when={branch().ahead > 0 || branch().behind > 0}>
          <Text style={{ fontSize: 11, color: app.theme().textTertiary, flexShrink: 0 }}>
            {`${branch().ahead > 0 ? `↑${branch().ahead}` : ""}${branch().ahead > 0 && branch().behind > 0 ? " " : ""}${branch().behind > 0 ? `↓${branch().behind}` : ""}`}
          </Text>
        </Show>
        <View style={{ flex: 1 }} />
        <Text style={{ fontSize: 11.5, color: app.theme().textTertiary, lineClamp: 1, textOverflow: "ellipsis", maxWidth: 260, minWidth: 0 }}>{branch().subject}</Text>
        <Text style={{ fontSize: 11, color: app.theme().textTertiary, flexShrink: 0, width: 80, textAlign: "end" }}>{relativeTime(branch().committerTime)}</Text>
      </RowButton>
    );
  };

  return (
    <View style={{ display: "flex", flex: 1, minWidth: 0, minHeight: 0, flexDirection: "column" }}>
      <View style={{ display: "flex", flexDirection: "row", alignItems: "center", gap: 8, height: 44, flexShrink: 0, paddingLeft: 16, paddingRight: 12, borderBottomWidth: 1, borderColor: app.theme().border }}>
        <Text style={{ fontSize: 14, fontWeight: 700, color: app.theme().text }}>Branches</Text>
        <Text style={{ fontSize: 12, color: app.theme().textTertiary }}>{`${local().length} local · ${remote().length} remote · ${tags().length} tags`}</Text>
        <View style={{ flex: 1 }} />
        <Host matchContents>
          <SwiftButton label="New Branch…" systemImage="plus" modifiers={[buttonStyle("borderedProminent")]} onPress={() => app.openDialog({ kind: "new-branch" })} />
        </Host>
      </View>
      <View style={{ display: "flex", flex: 1, minHeight: 0, flexDirection: "column", overflowY: "auto", paddingLeft: 6, paddingRight: 6, paddingBottom: 12 }}>
        <ListHeading label="Local" />
        <Show when={local().length > 0} fallback={<EmptyState ui={app} icon="branch" title="No branches yet" />}>
          <For each={local()}>{(branch) => <Row branch={branch} />}</For>
        </Show>
        <Show when={remote().length > 0}>
          <ListHeading
            label={`Remote${showRemotes() ? "" : ` (${remote().length})`}`}
            trailing={
              <PushButton ui={app} label={showRemotes() ? "Hide" : "Show"} onClick={() => setShowRemotes(!showRemotes())} style={{ height: 22, fontSize: 11, paddingLeft: 8, paddingRight: 8 }} />
            }
          />
          <Show when={showRemotes()}>
            <For each={remote()}>{(branch) => <Row branch={branch} />}</For>
          </Show>
        </Show>
        <Show when={tags().length > 0}>
          <ListHeading label="Tags" />
          <For each={tags().slice(0, 100)}>
            {(tag) => (
              <RowButton
                label={tag.name}
                onClick={() => setSelected(`refs/tags/${tag.name}`)}
                selected={selected() === `refs/tags/${tag.name}`}
                onContextMenu={() =>
                  void Menu.popup(
                    [
                      { label: "New Branch from Tag…", click: () => app.openDialog({ kind: "new-branch", from: tag.name }) },
                      { label: "Copy Tag Name", click: () => void copyText(tag.name) },
                    ],
                    { window: app.window },
                  )
                }
              >
                <Icon name="tag" size={13} color={app.theme().warning} />
                <Text style={{ fontSize: 12.5, fontWeight: 600, color: app.theme().text }}>{tag.name}</Text>
                <View style={{ flex: 1 }} />
                <Text style={{ fontSize: 11.5, color: app.theme().textTertiary, lineClamp: 1, textOverflow: "ellipsis", maxWidth: 260 }}>{tag.subject}</Text>
                <Text style={{ fontSize: 11, color: app.theme().textTertiary, width: 80, textAlign: "end" }}>{relativeTime(tag.committerTime)}</Text>
              </RowButton>
            )}
          </For>
        </Show>
      </View>
    </View>
  );
}

export function Tag(props: { label: string; accent?: boolean; warn?: boolean }) {
  const app = useApp();
  return (
    <View
      style={{
        display: "flex",
        height: 16,
        flexShrink: 0,
        alignItems: "center",
        paddingLeft: 5,
        paddingRight: 5,
        borderRadius: 3,
        backgroundColor: props.accent ? app.theme().accentWash : props.warn ? app.theme().warningWash : app.theme().hover,
      }}
    >
      <Text style={{ fontSize: 10.5, fontWeight: 500, color: props.accent ? app.theme().accent : props.warn ? app.theme().warning : app.theme().textSecondary }}>{props.label}</Text>
    </View>
  );
}
