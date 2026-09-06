import { KeyedFor } from "@quickgui/ui";
import { Menu, Shell, type QuickGuiEvent } from "@quickgui/native";
import { Input, Text, View, actionFromEvent, capturedPointerFromEvent, type Keymap, type VisibleRange } from "@quickgui/ui";
import { Checkbox } from "@quickgui/ui/controls";
import { Table } from "@quickgui/ui/collections";
import { Button as SwiftButton, Host, Picker, Toggle } from "@quickgui/ui/swift-ui";
import { buttonStyle, controlSize, disabled } from "@quickgui/ui/swift-ui/modifiers";
import { For, Show, createMemo, createSignal } from "@quickgui/ui";
import { basename, dirname, join } from "node:path";

import type { ChangeItem } from "../git/status.ts";
import { describeStatusCode } from "../git/status.ts";
import type { ListId } from "../model/store.ts";
import { useApp } from "./context.tsx";
import { DiffPane } from "./diff.tsx";
import { checkboxBox, confirm } from "./dialogs.tsx";
import { Icon } from "./icons.tsx";
import { EmptyState, IconButton, PushButton } from "./primitives.tsx";
import { copyText, openInTerminal } from "./sidebar.tsx";
import { LIST_FONT_SIZE, ROW_HEIGHT, statusColor } from "./theme.ts";

export function ChangesView() {
  const app = useApp();
  const store = app.store;

  let splitDragStart = 0;
  function handleSplitPointer(event: QuickGuiEvent): void {
    const pointer = capturedPointerFromEvent(event);
    if (pointer === undefined) return;
    if (pointer.button !== "left") return;
    if (pointer.phase === "down") {
      splitDragStart = store.changesSplit();
      return;
    }
    if (pointer.phase === "move") {
      store.setChangesSplit(Math.min(800, Math.max(240, splitDragStart + pointer.position.x - pointer.origin.x)));
    }
  }

  return (
    <View style={{ display: "flex", flex: 1, minWidth: 0, minHeight: 0, flexDirection: "row" }}>
      <View style={{ display: "flex", width: store.changesSplit(), flexShrink: 0, minHeight: 0, flexDirection: "column" }}>
        <Show
          when={store.changeCount() > 0}
          fallback={
            <EmptyState
              ui={app}
              icon="check"
              title="No local changes"
              description={
                store.status()?.branch
                  ? `${store.repositoryName()} is clean on ${store.status()!.branch}.`
                  : "The working tree matches the last commit."
              }
            />
          }
        >
          <FileList list="unstaged" />
          <FileList list="staged" />
        </Show>
        <CommitComposer />
      </View>
      <View
        aria-label="Resize file list"
        hitSlopLeft={4}
        hitSlopRight={4}
        onPointer={handleSplitPointer}
        style={[app.styles().vhairline, { cursor: "col-resize", appRegion: "no-drag" }]}
      />
      <DiffPane />
    </View>
  );
}

function FileList(props: { list: ListId }) {
  const app = useApp();
  const store = app.store;
  const items = createMemo(() => (props.list === "unstaged" ? store.unstaged() : store.staged()));
  const [range, setRange] = createSignal<VisibleRange>({ start: 0, end: 0 });
  const visible = createMemo(() =>
    items()
      .slice(range().start, range().end)
      .map((item, offset) => ({ index: range().start + offset, item })),
  );
  const stats = () => (props.list === "unstaged" ? store.numstat().unstaged : store.numstat().staged);
  const label = () => (props.list === "unstaged" ? "Unstaged" : "Staged");
  const empty = () => items().length === 0;

  async function contextMenu(item: ChangeItem, event: QuickGuiEvent): Promise<void> {
    event.stopPropagation();
    // Right-clicking a row outside the selection selects it, as every native list does.
    const selected = store.selectedItems(props.list);
    const targets = selected.some((candidate) => candidate.id === item.id) ? selected : [item];
    if (targets.length === 1 && targets[0] === item) {
      const index = items().indexOf(item);
      store.setSelection(props.list, [[index, index]]);
      store.setActiveCell({ list: props.list, row: index });
    }
    const absolute = join(store.repository()!.root, item.path);
    const many = targets.length > 1;
    await Menu.popup(
      [
        props.list === "unstaged"
          ? { label: many ? `Stage ${targets.length} Files` : "Stage File", click: () => void store.stageItems(targets) }
          : { label: many ? `Unstage ${targets.length} Files` : "Unstage File", click: () => void store.unstageItems(targets) },
        ...(props.list === "unstaged"
          ? [{ label: many ? `Discard ${targets.length} Files…` : "Discard Changes…", click: () => void discard(targets) }]
          : []),
        { type: "separator" },
        { label: "Reveal in Finder", enabled: item.code !== "D", click: () => void Shell.showItemInFolder(absolute) },
        { label: "Open", enabled: item.code !== "D", click: () => void Shell.openPath(absolute) },
        { label: "Open in Terminal", click: () => void openInTerminal(dirname(absolute)) },
        { type: "separator" },
        { label: "Copy Path", click: () => void copyText(item.path) },
        { label: "Copy Absolute Path", click: () => void copyText(absolute) },
      ],
      { window: app.window },
    );
  }

  async function discard(targets: readonly ChangeItem[]): Promise<void> {
    const untracked = targets.filter((item) => item.entry.kind === "untracked").length;
    const ok = await confirm(app, {
      message: targets.length === 1 ? `Discard changes to ${basename(targets[0]!.path)}?` : `Discard changes to ${targets.length} files?`,
      detail:
        untracked > 0
          ? `${untracked === targets.length ? "The files" : `${untracked} untracked ${untracked === 1 ? "file" : "files"}`} will be moved to the Trash. Tracked changes cannot be recovered.`
          : "The changes cannot be recovered.",
      confirmLabel: untracked === targets.length ? "Move to Trash" : "Discard",
      destructive: true,
    });
    if (ok) await store.discardItems(targets);
  }

  return (
    <View
      style={{
        display: "flex",
        flexDirection: "column",
        minHeight: 0,
        ...(empty() ? { flexShrink: 0 } : { flex: 1, flexBasis: 0 }),
      }}
    >
      {/* The label starts where the rows' checkboxes start and the button ends where their counts end. */}
      <View style={{ display: "flex", flexDirection: "row", alignItems: "center", height: 32, flexShrink: 0, paddingLeft: 12, paddingRight: 8, gap: 6, marginTop: props.list === "staged" ? 6 : 0 }}>
        <Text style={{ fontSize: 11, fontWeight: 600, color: app.theme().textTertiary }}>{label()}</Text>
        <Text style={{ fontSize: 11, color: app.theme().textTertiary }}>{String(items().length)}</Text>
        <View style={{ flex: 1 }} />
        <Show when={!empty()}>
          <Host matchContents>
            <SwiftButton
              label={props.list === "unstaged" ? "Stage All" : "Unstage All"}
              modifiers={[buttonStyle("bordered"), controlSize("small"), disabled(store.busy() !== undefined)]}
              onPress={() => void (props.list === "unstaged" ? store.stageAll() : store.unstageAll())}
            />
          </Host>
        </Show>
      </View>
      <Show when={!empty()} fallback={<Text style={{ paddingLeft: 12, paddingBottom: 10, fontSize: 12, color: app.theme().textTertiary }}>{props.list === "unstaged" ? "No unstaged changes" : "Nothing staged yet"}</Text>}>
        <Table.Root
          rowCount={items().length}
          rowHeight={ROW_HEIGHT}
          headerHeight={0}
          selectionMode="multiple"
          selection={store.selection()[props.list].map((range) => [...range])}
          columns={[
            { id: "toggle", track: "40px", align: "center" },
            { id: "status", track: "24px", align: "center" },
            { id: "name", track: "1fr", rowHeader: true },
          ]}
          onVisibleRangeChange={setRange}
          onSelectionChange={(ranges) => {
            store.setSelection(props.list, ranges);
            store.setFocusedList(props.list);
          }}
          onActiveCellChange={(cell) => store.setActiveCell(cell ? { list: props.list, row: cell.row } : undefined)}
          onActivate={() => void store.toggleStaging(props.list)}
          onFocus={() => store.setFocusedList(props.list)}
          style={{ flex: 1, minHeight: 0, overflowY: "scroll" }}
        >
          <KeyedFor each={visible()} key={(row) => row.index}>
            {(row) => (
              <Table.Row
                index={row().index}
                style={{
                  transition: "background-color 60ms",
                  hover: { backgroundColor: app.theme().hover },
                  selected: { backgroundColor: app.theme().selection },
                }}
              >
                <Table.Cell column="toggle">
                  <StageToggle item={row().item} list={props.list} />
                </Table.Cell>
                <Table.Cell column="status">
                  <StatusGlyph code={row().item.code} />
                </Table.Cell>
                <Table.Cell column="name" style={{ paddingRight: 8, minWidth: 0 }}>
                  <View
                    onContextMenu={(event) => void contextMenu(row().item, event)}
                    onDoubleClick={() => void store.toggleStaging(props.list)}
                    style={{ display: "flex", flex: 1, minWidth: 0, flexDirection: "row", alignItems: "center", gap: 6, height: "100%" }}
                  >
                    <FileName path={row().item.path} originalPath={row().item.originalPath} />
                    <Counts entry={stats().get(row().item.path)} />
                  </View>
                </Table.Cell>
              </Table.Row>
            )}
          </KeyedFor>
        </Table.Root>
      </Show>
    </View>
  );
}

function StageToggle(props: { item: () => (ChangeItem); list: ListId }) {
  const app = useApp();
  const store = app.store;
  const checked = () => props.list === "staged";
  return (
    <Checkbox.Root
      checked={checked()}
      ariaLabel={checked() ? `Unstage ${props.item().path}` : `Stage ${props.item().path}`}
      focusOnPointer={false}
      onCheckedChange={(next) => void (next ? store.stageItems([props.item()]) : store.unstageItems([props.item()]))}
      style={{ display: "flex", alignItems: "center", justifyContent: "center", width: 22, height: 22, borderRadius: 4, cursor: "default" }}
    >
      <Checkbox.Indicator style={checkboxBox(app, checked())}>
        <Show when={checked()}>
          <Icon name="check" size={11} color={app.theme().textOnAccent} />
        </Show>
      </Checkbox.Indicator>
    </Checkbox.Root>
  );
}

export function StatusGlyph(props: { code: () => (string) }) {
  const app = useApp();
  return (
    <View
      tooltip={describeStatusCode(props.code() as never)}
      style={{
        display: "flex",
        width: 16,
        height: 16,
        alignItems: "center",
        justifyContent: "center",
        borderRadius: 4,
        backgroundColor: `${statusColor(app.theme(), props.code())}22`,
      }}
    >
      <Text style={{ fontSize: 10, fontWeight: 800, color: statusColor(app.theme(), props.code()), fontFamily: "monospace" }}>
        {props.code() === "?" ? "U" : props.code()}
      </Text>
    </View>
  );
}

export function FileName(props: { path: () => (string); originalPath?: () => (string | undefined); size?: () => (number | undefined) }) {
  const readoriginalPath = () => { const source = props.originalPath; return source === undefined ? undefined : source(); };
  const readsize = () => { const source = props.size; return source === undefined ? undefined : source(); };
  const app = useApp();
  const directory = () => {
    const parent = dirname(props.path());
    return parent === "." ? "" : `${parent}/`;
  };
  return (
    <View style={{ display: "flex", flex: 1, minWidth: 0, flexDirection: "row", alignItems: "baseline", gap: 5 }}>
      <Show when={readoriginalPath()}>
        <Text style={{ fontSize: readsize() ?? LIST_FONT_SIZE, color: app.theme().textTertiary, lineClamp: 1, textOverflow: "ellipsis", flexShrink: 1 }}>
          {`${basename(readoriginalPath()!)} →`}
        </Text>
      </Show>
      <Text style={{ fontSize: readsize() ?? LIST_FONT_SIZE, fontWeight: 600, color: app.theme().text, flexShrink: 0 }}>{basename(props.path())}</Text>
      <Show when={directory()}>
        <Text style={{ fontSize: (readsize() ?? LIST_FONT_SIZE) - 1, color: app.theme().textTertiary, lineClamp: 1, textOverflow: "ellipsis", minWidth: 0, flexShrink: 1 }}>
          {directory()}
        </Text>
      </Show>
    </View>
  );
}

function Counts(props: { entry: () => ({ added: number | null; removed: number | null } | undefined) }) {
  const app = useApp();
  return (
    <Show when={props.entry() !== undefined && ((props.entry()!.added ?? 0) > 0 || (props.entry()!.removed ?? 0) > 0)}>
      <View style={{ display: "flex", flexDirection: "row", gap: 4, flexShrink: 0 }}>
        <Show when={(props.entry()!.added ?? 0) > 0}>
          <Text style={{ fontSize: 10.5, fontWeight: 700, color: app.theme().success, fontFamily: "monospace" }}>{`+${props.entry()!.added}`}</Text>
        </Show>
        <Show when={(props.entry()!.removed ?? 0) > 0}>
          <Text style={{ fontSize: 10.5, fontWeight: 700, color: app.theme().danger, fontFamily: "monospace" }}>{`−${props.entry()!.removed}`}</Text>
        </Show>
      </View>
    </Show>
  );
}

function CommitComposer() {
  const app = useApp();
  const store = app.store;
  const branch = () => store.status()?.branch;
  const identityMissing = () => !store.identity().name || !store.identity().email;
  const agentOptions = () => store.agents().map((agent) => ({ value: agent.id, label: agent.label }));
  const keymap: Keymap = [{ keys: "CmdOrCtrl+Enter", action: "commit" }, { keys: "CmdOrCtrl+Shift+G", action: "generate" }];
  const onAction = (event: QuickGuiEvent) => {
    const action = actionFromEvent(event);
    if (action === "commit") void store.commit();
    if (action === "generate") void store.generateMessage(undefined);
  };

  return (
    <View style={{ display: "flex", flexShrink: 0, flexDirection: "column", gap: 8, padding: 12, backgroundColor: app.theme().contentAlt, borderTopWidth: 1, borderColor: app.theme().border }}>
      <Input
        value={store.subject()}
        placeholder={store.amend() ? "Amend summary" : "Summary"}
        onInput={(event) => store.setSubject(event.value ?? "")}
        keymap={keymap}
        onAction={onAction}
        style={[app.styles().input, { width: "100%", minWidth: 0 }]}
      />
      <Input
        multiline
        value={store.body()}
        placeholder="Description"
        onInput={(event) => store.setBody(event.value ?? "")}
        keymap={keymap}
        onAction={onAction}
        style={[app.styles().textArea, { width: "100%", minWidth: 0, height: 72 }]}
      />
      <View style={{ display: "flex", flexDirection: "row", alignItems: "center", gap: 8 }}>
        <Host matchContents>
          <Toggle label="Amend" isOn={store.amend()} onIsOnChange={(value) => store.setAmend(value)} modifiers={[disabled(!store.hasHead())]} />
        </Host>
        <View style={{ flex: 1 }} />
        <Show when={agentOptions().length > 0}>
          <Host matchContents>
            <Picker
              style="menu"
              selection={store.preferredAgent() ?? agentOptions()[0]!.value}
              options={agentOptions()}
              modifiers={[controlSize("regular")]}
              onSelectionChange={(value) => store.setPreferredAgent(value as "codex" | "claude")}
            />
          </Host>
          <Host matchContents>
            <Show
              when={!store.generating()}
              fallback={<SwiftButton label="Cancel" systemImage="stop.fill" modifiers={[buttonStyle("bordered")]} onPress={() => store.cancelGeneration()} />}
            >
              <SwiftButton
                label="Generate"
                systemImage="sparkles"
                modifiers={[buttonStyle("bordered"), disabled(store.changeCount() === 0)]}
                onPress={() => void store.generateMessage(undefined)}
              />
            </Show>
          </Host>
        </Show>
      </View>
      <View style={{ display: "flex", flexDirection: "row", alignItems: "center", justifyContent: "flex-end", gap: 8 }}>
        <Show when={identityMissing()}>
          <Text style={{ flex: 1, minWidth: 0, fontSize: 11.5, color: app.theme().warning, lineClamp: 2 }}>Set user.name and user.email in git config before committing.</Text>
        </Show>
        <Show when={store.conflicts() > 0}>
          <Text style={{ flex: 1, minWidth: 0, fontSize: 11.5, color: app.theme().warning, lineClamp: 2 }}>Resolve the conflicted files and stage them to continue.</Text>
        </Show>
        <Host matchContents>
          <SwiftButton
            label={store.committing() ? "Committing…" : store.amend() ? "Amend" : `Commit${branch() ? ` to ${branch()}` : ""}`}
            modifiers={[buttonStyle("borderedProminent"), disabled(!store.canCommit())]}
            onPress={() => void store.commit()}
          />
        </Host>
      </View>

      <Show when={store.generating()}>
        <Text style={{ fontSize: 11.5, color: app.theme().textTertiary }}>{`${store.generating()!.agent === "codex" ? "Codex" : "Claude"} is reading the diff…`}</Text>
      </Show>
      <Show when={!store.generating() && store.lastGenerated() !== undefined && store.subject() === store.lastGenerated()!.subject}>
        <Text style={{ fontSize: 11.5, color: app.theme().textTertiary }}>{`Drafted by ${store.lastGenerated()!.agent === "codex" ? "Codex" : "Claude"}. Edit freely before committing.`}</Text>
      </Show>
    </View>
  );
}
