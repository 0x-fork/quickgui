import { Menu } from "@quickgui/native";
import { Text, View } from "@quickgui/solid";
import { For, Show } from "solid-js";
import { relativeTime } from "../git/log.ts";
import type { StashEntry } from "../git/refs.ts";
import { useApp } from "./context.tsx";
import { confirm } from "./dialogs.tsx";
import { EmptyState, PushButton } from "./primitives.tsx";
import { RowButton } from "./shell.tsx";

export function StashesView() {
  const app = useApp();
  const store = app.store;
  async function drop(stash: StashEntry) {
    if (
      await confirm(app, {
        message: `Drop ${stash.ref}?`,
        detail: "The saved changes cannot be recovered.",
        confirmLabel: "Drop",
        destructive: true,
      })
    )
      await store.stashDrop(stash.ref);
  }
  function menu(stash: StashEntry) {
    void Menu.popup(
      [
        { label: "Apply", click: () => void store.stashApply(stash.ref) },
        { label: "Pop", click: () => void store.stashPop(stash.ref) },
        { label: "Drop", click: () => void drop(stash) },
      ],
      { window: app.window },
    );
  }
  return (
    <View style={app.styles().listPanel}>
      <View style={app.styles().listHeader}>
        <Text style={app.styles().listTitle}>Stashes</Text>
        <PushButton
          ui={app}
          kind="primary"
          label="Stash Changes"
          disabled={store.changeCount() === 0}
          onClick={() => app.openDialog({ kind: "stash" })}
        />
      </View>
      <Show
        when={store.stashes().length > 0}
        fallback={
          <EmptyState
            ui={app}
            title="No stashes"
            description="Stash local changes to save them for later."
          />
        }
      >
        <For each={store.stashes()} keyed={(stash) => stash.ref}>
          {(stash) => (
            <RowButton
              label={stash().summary}
              onClick={() => {}}
              onContextMenu={() => menu(stash())}
            >
              <View style={{ display: "flex", flex: 1, minWidth: 0, flexDirection: "column" }}>
                <Text style={{ lineClamp: 1 }}>{stash().summary}</Text>
                <Text
                  style={{ fontSize: 11, color: app.theme().textTertiary }}
                >{`${stash().ref} · ${relativeTime(stash().time)}`}</Text>
              </View>
              <PushButton
                ui={app}
                label="Apply"
                onClick={() => void store.stashApply(stash().ref)}
              />
              <PushButton ui={app} label="Pop" onClick={() => void store.stashPop(stash().ref)} />
            </RowButton>
          )}
        </For>
      </Show>
    </View>
  );
}
