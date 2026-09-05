import { Menu } from "@quickgui/native";
import { Text, View } from "@quickgui/solid";
import { Button as SwiftButton, Host } from "@quickgui/solid/swift-ui";
import { buttonStyle, disabled } from "@quickgui/solid/swift-ui/modifiers";
import { For, Show } from "solid-js";

import { relativeTime } from "../git/log.ts";
import type { StashEntry } from "../git/refs.ts";
import { useApp } from "./context.tsx";
import { confirm } from "./dialogs.tsx";
import { Icon } from "./icons.tsx";
import { EmptyState, PushButton } from "./primitives.tsx";
import { RowButton } from "./shell.tsx";

export function StashesView() {
  const app = useApp();
  const store = app.store;

  async function drop(stash: StashEntry): Promise<void> {
    const ok = await confirm(app, { message: `Drop ${stash.ref}?`, detail: stash.summary, confirmLabel: "Drop", destructive: true });
    if (ok) await store.stashDrop(stash.ref);
  }

  function menu(stash: StashEntry): void {
    void Menu.popup(
      [
        { label: "Apply", click: () => void store.stashApply(stash.ref) },
        { label: "Pop (Apply and Drop)", click: () => void store.stashPop(stash.ref) },
        { type: "separator" },
        { label: "Drop…", click: () => void drop(stash) },
      ],
      { window: app.window },
    );
  }

  return (
    <View style={{ display: "flex", flex: 1, minWidth: 0, minHeight: 0, flexDirection: "column" }}>
      <View style={{ display: "flex", flexDirection: "row", alignItems: "center", gap: 8, height: 44, flexShrink: 0, paddingLeft: 16, paddingRight: 12, borderBottomWidth: 1, borderColor: app.theme().border }}>
        <Text style={{ fontSize: 14, fontWeight: 700, color: app.theme().text }}>Stashes</Text>
        <Text style={{ fontSize: 12, color: app.theme().textTertiary }}>{`${store.stashes().length} stashed`}</Text>
        <View style={{ flex: 1 }} />
        <Host matchContents>
          <SwiftButton label="Stash Changes…" systemImage="archivebox" modifiers={[buttonStyle("borderedProminent"), disabled(store.changeCount() === 0)]} onPress={() => app.openDialog({ kind: "stash" })} />
        </Host>
      </View>
      <View style={{ display: "flex", flex: 1, minHeight: 0, flexDirection: "column", gap: 2, overflowY: "auto", padding: 8 }}>
        <Show when={store.stashes().length > 0} fallback={<EmptyState ui={app} icon="archive" title="No stashes" description="Stash the working tree to set changes aside without committing them." />}>
          <For each={store.stashes()}>
            {(stash) => (
              <RowButton label={stash.summary} onClick={() => {}} onContextMenu={() => menu(stash)} onDoubleClick={() => void store.stashPop(stash.ref)} height={44}>
                <Icon name="archive" size={15} color={app.theme().textSecondary} />
                <View style={{ display: "flex", flex: 1, minWidth: 0, flexDirection: "column", gap: 2 }}>
                  <Text style={{ fontSize: 13, fontWeight: 600, color: app.theme().text, lineClamp: 1, textOverflow: "ellipsis" }}>{stash.summary}</Text>
                  <Text style={{ fontSize: 11.5, color: app.theme().textTertiary }}>{`${stash.ref}${stash.branch ? ` · on ${stash.branch}` : ""} · ${relativeTime(stash.time)}`}</Text>
                </View>
                <View style={{ display: "flex", flexDirection: "row", gap: 4, flexShrink: 0, opacity: 0, groupHover: { opacity: 1 } }}>
                  <PushButton ui={app} label="Apply" onClick={() => void store.stashApply(stash.ref)} style={{ height: 24, fontSize: 11.5 }} />
                  <PushButton ui={app} label="Pop" kind="primary" onClick={() => void store.stashPop(stash.ref)} style={{ height: 24, fontSize: 11.5 }} />
                  <PushButton ui={app} label="Drop" kind="danger" onClick={() => void drop(stash)} style={{ height: 24, fontSize: 11.5 }} />
                </View>
              </RowButton>
            )}
          </For>
        </Show>
      </View>
    </View>
  );
}
