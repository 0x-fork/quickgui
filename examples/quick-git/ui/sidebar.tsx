import { Menu, Shell } from "@quickgui/native";
import { Button, Text, View } from "@quickgui/solid";
import { For, Show, createMemo } from "solid-js";
import { basename } from "node:path";

import { relativeTime } from "../git/log.ts";
import type { ViewId } from "../model/store.ts";
import { useApp } from "./context.tsx";
import { Icon, type IconName } from "./icons.tsx";
import { repositoryLabels } from "./labels.ts";
import { IconButton } from "./primitives.tsx";
import { SIDEBAR_TOP_INSET, TITLEBAR_HEIGHT } from "./theme.ts";

/** Left inset of the sidebar's content, where the repository menu drops down from. */
const SIDEBAR_INSET = 10;
const REPOSITORY_HEADER_HEIGHT = 44;

export function Sidebar() {
  const app = useApp();
  const store = app.store;

  const navItems: { id: ViewId; label: string; icon: IconName; count?: () => number }[] = [
    { id: "changes", label: "Changes", icon: "commit", count: () => store.changeCount() },
    { id: "history", label: "History", icon: "history" },
  ];

  const localBranches = createMemo(() => store.refs().local);
  const currentBranch = () => store.status()?.branch;
  const activeRoot = () => store.repository()?.root;

  // The header is a repository switcher: a menu dropping down from it lists every known
  // repository with the current one checked, and a chosen one opens in its own window.
  function repositoryMenu(): void {
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
        ...(recent.length > 0 ? [{ type: "separator" as const }] : []),
        { label: "Open Repository…", click: () => void app.openRepository() },
        { type: "separator" as const },
        { label: "Reveal in Finder", click: () => void Shell.showItemInFolder(store.repository()!.root) },
        { label: "Open in Terminal", click: () => void openInTerminal(store.repository()!.root) },
        { label: "Copy Path", click: () => void copyText(store.repository()!.root) },
        { type: "separator" as const },
        { label: "Close Repository", click: () => store.closeRepository() },
      ],
      { window: app.window, x: SIDEBAR_INSET, y: TITLEBAR_HEIGHT + SIDEBAR_TOP_INSET + REPOSITORY_HEADER_HEIGHT },
    );
  }

  return (
    <>
      <View style={app.styles().sidebarTitlebar}>
        <View style={{ flex: 1 }} />
      </View>
      <View style={app.styles().sidebarScroll}>
        <Button
          aria-label="Repository actions"
          focusOnPointer={false}
          onClick={repositoryMenu}
          style={{
            display: "flex",
            flexDirection: "row",
            alignItems: "center",
            gap: 9,
            height: REPOSITORY_HEADER_HEIGHT,
            flexShrink: 0,
            paddingLeft: 8,
            paddingRight: 8,
            marginBottom: 6,
            borderRadius: 8,
            backgroundColor: "transparent",
            cursor: "default",
            userSelect: "none",
            hover: { backgroundColor: app.theme().hover },
            active: { backgroundColor: app.theme().active },
            focus: { outline: `2px solid ${app.theme().focusRing}` },
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
              backgroundColor: app.theme().accent,
              color: app.theme().textOnAccent,
            }}
          >
            <Icon name="branch" size={15} />
          </View>
          <View style={{ display: "flex", flex: 1, minWidth: 0, flexDirection: "column", gap: 1 }}>
            <Text style={{ fontSize: 13, fontWeight: 700, color: app.theme().text, lineClamp: 1, textOverflow: "ellipsis" }}>
              {store.repositoryName()}
            </Text>
            <Text style={{ fontSize: 11, color: app.theme().textTertiary, lineClamp: 1, textOverflow: "ellipsis" }}>
              {currentBranch() ?? (store.status()?.detached ? "Detached HEAD" : "")}
            </Text>
          </View>
          <Icon name="chevron-down" size={12} color={app.theme().textTertiary} />
        </Button>

        <For each={navItems}>
          {(item) => (
            <NavRow
              icon={item.icon}
              label={item.label}
              selected={store.view() === item.id}
              count={item.count?.()}
              onClick={() => store.setView(item.id)}
            />
          )}
        </For>

        <SectionRow
          label="Branches"
          count={localBranches().length}
          selected={store.view() === "branches"}
          onClick={() => store.setView("branches")}
          action={{ icon: "plus", label: "New branch", onClick: () => app.openDialog({ kind: "new-branch" }) }}
        />
        <For each={localBranches().slice(0, 8)}>
          {(branch) => (
            <NavRow
              icon="branch"
              label={branch.name}
              muted={!branch.current}
              small
              trailing={
                branch.worktreePath && branch.worktreePath !== store.mainRepository()?.root && !branch.current
                  ? "worktree"
                  : branch.current
                    ? "check"
                    : undefined
              }
              selected={false}
              onClick={() => {
                if (branch.current) return;
                if (branch.worktreePath && branch.worktreePath !== store.repository()?.root) void store.selectWorktree(branch.worktreePath);
                else void store.switchBranch(branch.name);
              }}
            />
          )}
        </For>
        <Show when={localBranches().length > 8}>
          <NavRow icon="more" label={`${localBranches().length - 8} more…`} muted small selected={false} onClick={() => store.setView("branches")} />
        </Show>

        <SectionRow
          label="Worktrees"
          count={store.worktrees().length}
          selected={store.view() === "worktrees"}
          onClick={() => store.setView("worktrees")}
          action={{ icon: "plus", label: "New worktree", onClick: () => app.openDialog({ kind: "new-worktree" }) }}
        />
        <For each={store.worktrees()}>
          {(worktree) => (
            <NavRow
              icon={worktree.main ? "folder" : "worktree"}
              label={worktree.branchName ?? (worktree.detached ? `${worktree.headSha?.slice(0, 7) ?? ""} (detached)` : basename(worktree.path))}
              detail={basename(worktree.path)}
              small
              muted={activeRoot() !== worktree.path}
              trailing={activeRoot() === worktree.path ? "check" : undefined}
              selected={false}
              onClick={() => void store.selectWorktree(worktree.path)}
            />
          )}
        </For>

        <SectionRow
          label="Stashes"
          count={store.stashes().length}
          selected={store.view() === "stashes"}
          onClick={() => store.setView("stashes")}
          action={{
            icon: "plus",
            label: "Stash changes",
            disabled: store.changeCount() === 0,
            onClick: () => app.openDialog({ kind: "stash" }),
          }}
        />
        <For each={store.stashes().slice(0, 5)}>
          {(stash) => (
            <NavRow
              icon="archive"
              label={stash.summary}
              detail={relativeTime(stash.time)}
              small
              muted
              selected={false}
              onClick={() => store.setView("stashes")}
            />
          )}
        </For>
      </View>
    </>
  );
}

function NavRow(props: {
  icon: IconName;
  label: string;
  detail?: string | undefined;
  count?: number | undefined;
  trailing?: IconName | undefined;
  selected: boolean;
  muted?: boolean | undefined;
  small?: boolean | undefined;
  onClick: () => void;
}) {
  const app = useApp();
  return (
    <Button
      aria-label={props.label}
      focusOnPointer={false}
      selected={props.selected}
      onClick={props.onClick}
      style={{
        display: "flex",
        flexDirection: "row",
        alignItems: "center",
        gap: 8,
        height: 28,
        flexShrink: 0,
        paddingLeft: 8,
        paddingRight: 8,
        borderRadius: 6,
        backgroundColor: "transparent",
        color: props.muted ? app.theme().textSecondary : app.theme().text,
        cursor: "default",
        userSelect: "none",
        transition: "background-color 80ms",
        hover: { backgroundColor: app.theme().hover },
        active: { backgroundColor: app.theme().active },
        selected: { backgroundColor: app.theme().selection, color: app.theme().text },
        focus: { outline: `2px solid ${app.theme().focusRing}` },
      }}
    >
      <Icon name={props.icon} size={16} color={props.selected ? app.theme().accent : app.theme().textSecondary} />
      <Text
        style={{
          flex: 1,
          minWidth: 0,
          fontSize: 13,
          fontWeight: 400,
          lineClamp: 1,
          textOverflow: "ellipsis",
        }}
      >
        {props.label}
      </Text>
      <Show when={props.detail}>
        <Text style={{ fontSize: 11, color: app.theme().textTertiary, lineClamp: 1, textOverflow: "ellipsis", maxWidth: 90 }}>
          {props.detail}
        </Text>
      </Show>
      <Show when={props.count !== undefined && props.count > 0}>
        <Text style={{ fontSize: 12, color: app.theme().textTertiary }}>{String(props.count)}</Text>
      </Show>
      <Show when={props.trailing}>{(name) => <Icon name={name()} size={12} color={app.theme().textTertiary} />}</Show>
    </Button>
  );
}

function SectionRow(props: {
  label: string;
  count: number;
  selected: boolean;
  onClick: () => void;
  action: { icon: IconName; label: string; onClick: () => void; disabled?: boolean };
}) {
  const app = useApp();
  return (
    <View group style={{ display: "flex", flexDirection: "row", alignItems: "center", gap: 4, marginTop: 14, paddingRight: 2 }}>
      <Button
        aria-label={props.label}
        focusOnPointer={false}
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
          backgroundColor: "transparent",
          cursor: "default",
          userSelect: "none",
          hover: { backgroundColor: app.theme().hover },
          focus: { outline: `2px solid ${app.theme().focusRing}` },
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
          <Text style={{ fontSize: 11, fontWeight: 600, color: app.theme().textTertiary }}>{String(props.count)}</Text>
        </Show>
      </Button>
      <IconButton
        ui={app}
        icon={props.action.icon}
        label={props.action.label}
        size={20}
        iconSize={12}
        disabled={props.action.disabled ?? false}
        onClick={props.action.onClick}
        style={{ focus: { outline: `2px solid ${app.theme().focusRing}` } }}
      />
    </View>
  );
}

export async function openInTerminal(path: string): Promise<void> {
  const child = Bun.spawn(["open", "-a", "Terminal", path], { stdout: "ignore", stderr: "ignore" });
  await child.exited;
}

/** Open Terminal at `path` and run `command` there, for launching an agent in a worktree. */
export async function runInTerminal(path: string, command: string): Promise<void> {
  const escaped = `cd ${shellQuote(path)} && ${command}`;
  const script = `tell application "Terminal"\nactivate\ndo script ${appleScriptString(escaped)}\nend tell`;
  const child = Bun.spawn(["osascript", "-e", script], { stdout: "ignore", stderr: "ignore" });
  await child.exited;
}

function shellQuote(value: string): string {
  return `'${value.replace(/'/g, `'\\''`)}'`;
}

function appleScriptString(value: string): string {
  return `"${value.replace(/\\/g, "\\\\").replace(/"/g, '\\"')}"`;
}

export async function copyText(text: string): Promise<void> {
  const { Clipboard } = await import("@quickgui/native");
  await Clipboard.write({ entries: [{ type: "text", text }] });
}
