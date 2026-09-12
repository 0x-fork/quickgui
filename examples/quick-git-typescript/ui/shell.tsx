import { Window, type AppearanceMode, type QuickGuiEvent } from "@quickgui/native";
import { Button, Text, Toast, View, useToastManager } from "@quickgui/solid";
import { For, Show, Switch, Match, createEffect, createMemo, createSignal } from "solid-js";

import type { Store } from "../model/store.ts";
import { BranchesView } from "./branches.tsx";
import { ChangesView } from "./changes.tsx";
import { activateCommands, registerCommands } from "./commands.ts";
import { AppProvider, useApp, type DialogRequest } from "./context.tsx";
import { Dialogs } from "./dialogs.tsx";
import { HistoryView } from "./history.tsx";
import { Icon } from "./icons.tsx";
import { ResizablePanel } from "./resize.tsx";
import { Sidebar } from "./sidebar.tsx";
import { StashesView } from "./stashes.tsx";
import { Toolbar } from "./toolbar.tsx";
import { createStyles, themeFor } from "./theme.ts";
import { WelcomeView } from "./welcome.tsx";
import { WorktreesView } from "./worktrees.tsx";

export function App(props: {
  store: Store;
  appearance: () => AppearanceMode;
  openRepository: () => Promise<void>;
  openRepositoryPath: (path: string) => Promise<void>;
}) {
  const window = Window.getCurrentWindow();
  const theme = createMemo(() => themeFor(props.appearance()));
  const styles = createMemo(() => createStyles(theme()));
  const [dialog, setDialog] = createSignal<DialogRequest | undefined>(undefined, {
    ownedWrite: true,
  });

  return (
    <AppProvider
      value={{
        store: props.store,
        window,
        theme,
        styles,
        dialog,
        openDialog: setDialog,
        closeDialog: () => setDialog(undefined),
        openRepository: () => props.openRepository(),
        openRepositoryPath: (path) => props.openRepositoryPath(path),
      }}
    >
      <Toast.Provider timeout={4500} limit={3} pitch={6} swipeDirection="right">
        <Shell />
      </Toast.Provider>
    </AppProvider>
  );
}

function Shell() {
  const app = useApp();
  const store = app.store;
  const toasts = useToastManager();

  // Runs once after the shell mounts: the notifier, menu commands, and focus refresh.
  createEffect(
    () => 0,
    () => {
      store.setNotifier((notice) => {
        toasts.add({
          title: notice.title,
          ...(notice.description ? { description: notice.description } : {}),
          type: notice.type,
          ...(notice.timeout ? { duration: notice.timeout } : {}),
        });
      });
      const unregister = registerCommands(app.window, {
        openRepository: () => void app.openRepository(),
        openDialog: app.openDialog,
        focusCommitMessage: () => store.setView("changes"),
      });
      const unfocus = app.window.on("focus", () => {
        activateCommands(app.window);
        if (store.repository()) void store.refresh();
      });
      return () => {
        unregister();
        unfocus();
      };
    },
  );

  return (
    <View style={app.styles().app}>
      <Show
        when={store.repository()}
        fallback={<WelcomeView openRepository={app.openRepository} />}
      >
        <ResizablePanel
          label="Resize sidebar"
          width={store.sidebarWidth()}
          onResize={store.setSidebarWidth}
          minimum={180}
          maximum={420}
          style={app.styles().sidebar}
        >
          <Sidebar />
        </ResizablePanel>
        <View style={app.styles().main}>
          <Toolbar />
          <Switch>
            <Match when={store.view() === "changes"}>
              <ChangesView />
            </Match>
            <Match when={store.view() === "history"}>
              <HistoryView />
            </Match>
            <Match when={store.view() === "branches"}>
              <BranchesView />
            </Match>
            <Match when={store.view() === "worktrees"}>
              <WorktreesView />
            </Match>
            <Match when={store.view() === "stashes"}>
              <StashesView />
            </Match>
          </Switch>
        </View>
      </Show>
      <Dialogs />
      <Notices />
    </View>
  );
}

function Notices() {
  const app = useApp();
  const toasts = useToastManager();
  const color = (type: string) =>
    type === "error"
      ? app.theme().danger
      : type === "success"
        ? app.theme().success
        : type === "warning"
          ? app.theme().warning
          : app.theme().accent;
  return (
    <Toast.Portal
      style={{ position: "absolute", right: 16, bottom: 16, width: 340, display: "flex" }}
    >
      <Toast.Viewport style={{ display: "flex", flexDirection: "column", gap: 8, width: "100%" }}>
        <For each={toasts.stack()}>
          {(entry) => {
            const toast = () => toasts.toasts().find((candidate) => candidate.id === entry.id);
            return (
              <Toast.Positioner toastId={entry.id}>
                <Toast.Root
                  toastId={entry.id}
                  style={[
                    app.styles().popup,
                    {
                      flexDirection: "row",
                      alignItems: "flex-start",
                      gap: 10,
                      paddingLeft: 12,
                      paddingRight: 8,
                      paddingTop: 10,
                      paddingBottom: 10,
                      opacity: entry.limited ? 0.6 : 1,
                      transform: `translateX(${entry.swipeMovement}px)`,
                    },
                  ]}
                >
                  <View
                    style={{
                      width: 3,
                      alignSelf: "stretch",
                      borderRadius: 2,
                      bg: color(entry.type),
                    }}
                  />
                  <Toast.Content
                    toastId={entry.id}
                    style={{
                      display: "flex",
                      flex: 1,
                      minWidth: 0,
                      flexDirection: "column",
                      gap: 2,
                    }}
                  >
                    <Toast.Title toastId={entry.id}>
                      <Text
                        style={{
                          fontSize: 12.5,
                          fontWeight: 700,
                          color: app.theme().text,
                          lineClamp: 2,
                        }}
                      >
                        {toast()?.title ?? ""}
                      </Text>
                    </Toast.Title>
                    <Show when={toast()?.description}>
                      <Toast.Description toastId={entry.id}>
                        <Text
                          style={{
                            fontSize: 12,
                            lineHeight: 16,
                            color: app.theme().textSecondary,
                            lineClamp: 4,
                          }}
                        >
                          {toast()?.description ?? ""}
                        </Text>
                      </Toast.Description>
                    </Show>
                  </Toast.Content>
                  <Toast.Close toastId={entry.id} style={app.styles().iconButton(22)}>
                    <Icon name="x" size={12} />
                  </Toast.Close>
                </Toast.Root>
              </Toast.Positioner>
            );
          }}
        </For>
      </Toast.Viewport>
    </Toast.Portal>
  );
}

export function RowButton(props: {
  selected?: boolean;
  onClick: () => void;
  onDoubleClick?: () => void;
  onContextMenu?: (event: QuickGuiEvent) => void;
  label: string;
  children: unknown;
  height?: number;
}) {
  const app = useApp();
  return (
    <Button
      aria-label={props.label}
      focusOnPointer={false}
      group
      selected={props.selected ?? false}
      onClick={props.onClick}
      {...(props.onDoubleClick ? { onDoubleClick: props.onDoubleClick } : {})}
      {...(props.onContextMenu ? { onContextMenu: props.onContextMenu } : {})}
      style={{
        display: "flex",
        flexDirection: "row",
        width: "100%",
        minWidth: 0,
        height: props.height ?? 28,
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
        hover: { bg: app.theme().hover },
        active: { bg: app.theme().active },
        selected: { bg: app.theme().selection },
        focus: { outline: `2px solid ${app.theme().focusRing}` },
      }}
    >
      {props.children as never}
    </Button>
  );
}
