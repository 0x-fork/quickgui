import { Button, Text, View, dropEventFromEvent } from "@quickgui/solid";
import { For, Show } from "solid-js";
import { basename, dirname } from "node:path";
import { homedir } from "node:os";

import { useApp } from "./context.tsx";
import { Icon } from "./icons.tsx";
import { PushButton } from "./primitives.tsx";

export function WelcomeView(props: { openRepository: () => Promise<void> }) {
  const app = useApp();
  const store = app.store;
  const home = homedir();
  const shorten = (path: string) => (path.startsWith(home) ? `~${path.slice(home.length)}` : path);

  return (
    <View
      style={{
        display: "flex",
        flex: 1,
        minWidth: 0,
        minHeight: 0,
        flexDirection: "column",
        backgroundColor: app.theme().content,
      }}
    >
      <View style={{ height: 52, flexShrink: 0, appRegion: "drag" }} />
      <View
        dropKinds="files"
        onFilesDropped={(event) => {
          const [path] = dropEventFromEvent(event)?.paths ?? [];
          if (path) void app.openRepositoryPath(path);
        }}
        style={{
          display: "flex",
          flex: 1,
          minHeight: 0,
          flexDirection: "column",
          alignItems: "center",
          justifyContent: "center",
          gap: 20,
          padding: 40,
          dragOver: { backgroundColor: app.theme().accentWash },
        }}
      >
        <View
          style={{
            display: "flex",
            width: 64,
            height: 64,
            alignItems: "center",
            justifyContent: "center",
            borderRadius: 18,
            backgroundColor: app.theme().accent,
            color: app.theme().textOnAccent,
            boxShadow: "0 12px 30px -10px #2f6bff88",
          }}
        >
          <Icon name="branch" size={32} />
        </View>
        <View style={{ display: "flex", flexDirection: "column", alignItems: "center", gap: 6 }}>
          <Text style={{ fontSize: 22, fontWeight: 800, color: app.theme().text }}>Quick Git</Text>
          <Text style={{ fontSize: 13, color: app.theme().textSecondary, textAlign: "center", lineHeight: 19 }}>
            Open a repository, or drop a folder here.
          </Text>
        </View>
        <PushButton ui={app} kind="primary" icon="folder-open" label="Open Repository…" onClick={() => void props.openRepository()} />
        <Show when={store.recentRepositories().length > 0}>
          <View style={{ display: "flex", flexDirection: "column", width: 420, maxWidth: "100%", gap: 2, marginTop: 8 }}>
            <Text style={{ fontSize: 11, fontWeight: 700, letterSpacing: 0.4, textTransform: "uppercase", color: app.theme().textTertiary, paddingLeft: 10, marginBottom: 4 }}>
              Recent
            </Text>
            <For each={store.recentRepositories().slice(0, 8)}>
              {(path) => (
                <Button
                  aria-label={`Open ${basename(path)}`}
                  focusOnPointer={false}
                  disabled={store.opening() !== undefined}
                  onClick={() => void app.openRepositoryPath(path)}
                  style={{
                    display: "flex",
                    flexDirection: "row",
                    alignItems: "center",
                    gap: 10,
                    height: 40,
                    paddingLeft: 10,
                    paddingRight: 10,
                    borderRadius: 8,
                    backgroundColor: "transparent",
                    cursor: "default",
                    userSelect: "none",
                    hover: { backgroundColor: app.theme().hover },
                    active: { backgroundColor: app.theme().active },
                    focus: { outline: `2px solid ${app.theme().focusRing}` },
                    disabled: { opacity: 0.6 },
                  }}
                >
                  <Icon name="folder" size={16} color={app.theme().textSecondary} />
                  <View style={{ display: "flex", flex: 1, minWidth: 0, flexDirection: "column" }}>
                    <Text style={{ fontSize: 13, fontWeight: 600, color: app.theme().text, lineClamp: 1, textOverflow: "ellipsis" }}>{basename(path)}</Text>
                    <Text style={{ fontSize: 11, color: app.theme().textTertiary, lineClamp: 1, textOverflow: "ellipsis" }}>{shorten(dirname(path))}</Text>
                  </View>
                  <Show when={store.opening() === path}>
                    <Text style={{ fontSize: 11, color: app.theme().textTertiary }}>Opening…</Text>
                  </Show>
                </Button>
              )}
            </For>
          </View>
        </Show>
      </View>
    </View>
  );
}
