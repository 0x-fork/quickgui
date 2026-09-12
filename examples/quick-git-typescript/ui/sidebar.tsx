import { Clipboard, Menu, Shell } from "@quickgui/native";
import { Button, Text, View } from "@quickgui/solid";
import { For, Show } from "solid-js";
import { basename } from "node:path";
import { relativeTime } from "../git/log.ts";
import type { ViewId } from "../model/store.ts";
import { useApp } from "./context.tsx";
import { Icon } from "./icons.tsx";
import { repositoryLabels } from "./labels.ts";

export function Sidebar() {
  const app = useApp();
  const store = app.store;
  const nav: { id: ViewId; label: string }[] = [
    { id: "changes", label: "Changes" },
    { id: "history", label: "History" },
  ];
  function repositoryMenu() {
    const current = store.mainRepository()?.root;
    const recent = store.recentRepositories();
    const labels = repositoryLabels(recent);
    void Menu.popup(
      [
        ...recent.map((path) => ({
          label: labels.get(path) ?? basename(path),
          checked: path === current,
          click: () => {
            if (path !== current) void app.openRepositoryPath(path);
          },
        })),
        ...(recent.length ? [{ type: "separator" as const }] : []),
        { label: "Open Repository…", click: () => void app.openRepository() },
        { type: "separator" },
        {
          label: "Reveal in Finder",
          click: () => void Shell.showItemInFolder(store.repository()!.root),
        },
        { label: "Copy Path", click: () => void copyText(store.repository()!.root) },
        { type: "separator" },
        { label: "Close Repository", click: () => store.closeRepository() },
      ],
      { window: app.window },
    );
  }
  return (
    <>
      <View style={app.styles().sidebarTitlebar} />
      <View style={app.styles().sidebarScroll}>
        <Button
          aria-label="Repository actions"
          onClick={repositoryMenu}
          style={{
            display: "flex",
            flexDirection: "row",
            alignItems: "center",
            gap: 9,
            height: 44,
            flexShrink: 0,
            paddingLeft: 8,
            paddingRight: 8,
            marginBottom: 6,
            borderRadius: 8,
            bg: "transparent",
            cursor: "default",
            hover: { bg: app.theme().hover },
          }}
        >
          <View
            style={{
              display: "flex",
              width: 28,
              height: 28,
              flexShrink: 0,
              alignItems: "center",
              justifyContent: "center",
              borderRadius: 7,
              bg: app.theme().accent,
            }}
          >
            <Icon name="branch" size={18} color={app.theme().textOnAccent} />
          </View>
          <View style={{ display: "flex", flex: 1, minWidth: 0, flexDirection: "column", gap: 1 }}>
            <Text style={{ fontSize: 13, fontWeight: 700, color: app.theme().text, lineClamp: 1 }}>
              {store.repositoryName()}
            </Text>
            <Text style={{ fontSize: 11, color: app.theme().textTertiary, lineClamp: 1 }}>
              {store.status()?.branch || (store.status()?.detached ? "Detached HEAD" : "")}
            </Text>
          </View>
          <Icon name="chevron-down" size={14} color={app.theme().textTertiary} />
        </Button>
        <For each={nav}>
          {(item) => (
            <NavRow
              label={item.label}
              selected={store.view() === item.id}
              detail={
                item.id === "changes" && store.changeCount() > 0 ? String(store.changeCount()) : ""
              }
              onClick={() => store.setView(item.id)}
            />
          )}
        </For>
        <SectionRow
          label="Branches"
          count={store.refs().local.length}
          selected={store.view() === "branches"}
          onClick={() => store.setView("branches")}
          onAdd={() => app.openDialog({ kind: "new-branch" })}
        />
        <For each={store.refs().local.slice(0, 8)} keyed={(branch) => branch.fullName}>
          {(branch) => (
            <NavRow
              label={branch().name}
              current={branch().current}
              onClick={() => {
                if (branch().current) return;
                const path = branch().worktreePath;
                if (path && path !== store.repository()?.root) void store.selectWorktree(path);
                else void store.switchBranch(branch().name);
              }}
            />
          )}
        </For>
        <SectionRow
          label="Worktrees"
          count={store.worktrees().length}
          selected={store.view() === "worktrees"}
          onClick={() => store.setView("worktrees")}
          onAdd={() => app.openDialog({ kind: "new-worktree" })}
        />
        <For each={store.worktrees()} keyed={(worktree) => worktree.path}>
          {(worktree) => (
            <NavRow
              label={
                worktree().branchName ||
                (worktree().detached && worktree().headSha
                  ? `${worktree().headSha!.slice(0, 7)} (detached)`
                  : basename(worktree().path))
              }
              current={store.repository()?.root === worktree().path}
              onClick={() => void store.selectWorktree(worktree().path)}
            />
          )}
        </For>
        <SectionRow
          label="Stashes"
          count={store.stashes().length}
          selected={store.view() === "stashes"}
          onClick={() => store.setView("stashes")}
          onAdd={() => app.openDialog({ kind: "stash" })}
        />
        <For each={store.stashes().slice(0, 5)} keyed={(stash) => stash.ref}>
          {(stash) => (
            <NavRow
              label={stash().summary}
              detail={relativeTime(stash().time)}
              onClick={() => store.setView("stashes")}
            />
          )}
        </For>
      </View>
    </>
  );
}

export function NavRow(props: {
  label: string;
  detail?: string;
  selected?: boolean;
  current?: boolean;
  onClick: () => void;
}) {
  const app = useApp();
  return (
    <Button
      aria-label={props.label}
      selected={props.selected ?? false}
      onClick={props.onClick}
      style={{
        display: "flex",
        flexDirection: "row",
        width: "100%",
        minWidth: 0,
        height: 28,
        flexShrink: 0,
        alignItems: "center",
        gap: 8,
        paddingLeft: 10,
        paddingRight: 8,
        borderRadius: 6,
        bg: props.selected ? app.theme().selection : "transparent",
        color: app.theme().text,
        cursor: "default",
        userSelect: "none",
        hover: { bg: props.selected ? app.theme().selection : app.theme().hover },
        active: { bg: app.theme().active },
      }}
    >
      <Text style={{ flex: 1, minWidth: 0, fontSize: 13, lineClamp: 1 }}>{props.label}</Text>
      <Show when={props.detail}>
        <Text style={{ fontSize: 11, color: app.theme().textTertiary }}>{props.detail}</Text>
      </Show>
      <Show when={props.current}>
        <Icon name="check" size={14} color={app.theme().textSecondary} />
      </Show>
    </Button>
  );
}

function SectionRow(props: {
  label: string;
  count: number;
  selected: boolean;
  onClick: () => void;
  onAdd: () => void;
}) {
  const app = useApp();
  return (
    <View
      group
      style={{
        display: "flex",
        flexDirection: "row",
        alignItems: "center",
        gap: 4,
        marginTop: 14,
        paddingRight: 2,
      }}
    >
      <Button
        onClick={props.onClick}
        style={{
          display: "flex",
          flex: 1,
          minWidth: 0,
          flexDirection: "row",
          alignItems: "center",
          gap: 6,
          height: 22,
          paddingLeft: 9,
          paddingRight: 6,
          borderRadius: 6,
          bg: "transparent",
          cursor: "default",
          hover: { bg: app.theme().hover },
        }}
      >
        <Text
          style={{
            flex: 1,
            minWidth: 0,
            fontSize: 11,
            fontWeight: 600,
            color: props.selected ? app.theme().accent : app.theme().textTertiary,
          }}
        >
          {props.label}
        </Text>
        <Show when={props.count > 0}>
          <Text style={{ fontSize: 11, fontWeight: 600, color: app.theme().textTertiary }}>
            {props.count}
          </Text>
        </Show>
      </Button>
      <Button
        aria-label={`Add ${props.label.toLowerCase()}`}
        onClick={props.onAdd}
        style={app.styles().iconButton(22)}
      >
        <Icon name="plus" size={14} color={app.theme().textSecondary} />
      </Button>
    </View>
  );
}

export async function copyText(text: string): Promise<void> {
  await Clipboard.write({ entries: [{ type: "text", text }] });
}

export async function openInTerminal(path: string): Promise<void> {
  const child = Bun.spawn(["open", "-a", "Terminal", path], { stdout: "ignore", stderr: "ignore" });
  await child.exited;
}

export async function runInTerminal(path: string, command: string): Promise<void> {
  const quote = (value: string) => "'" + value.replaceAll("'", "'\\''") + "'";
  const shellCommand = `cd ${quote(path)} && ${quote(command)}`;
  const literal = '"' + shellCommand.replaceAll("\\", "\\\\").replaceAll('"', '\\"') + '"';
  const child = Bun.spawn(
    ["osascript", "-e", `tell application "Terminal"\nactivate\ndo script ${literal}\nend tell`],
    { stdout: "ignore", stderr: "ignore" },
  );
  await child.exited;
}
