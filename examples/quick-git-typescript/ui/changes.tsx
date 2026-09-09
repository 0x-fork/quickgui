import { Menu, Shell, type QuickGuiEvent } from "@quickgui/native";
import {
  Checkbox,
  Input,
  TextArea,
  Table,
  Text,
  View,
  actionFromEvent,
  type VisibleRange,
} from "@quickgui/solid";
import { For, Show, createMemo, createSignal } from "solid-js";
import { basename, dirname, join } from "node:path";

import type { ChangeItem } from "../git/status.ts";
import type { ListId } from "../model/store.ts";
import { useApp } from "./context.tsx";
import { ResizablePanel } from "./resize.tsx";
import { DiffPane } from "./diff.tsx";
import { CheckRow, checkboxBox, confirm } from "./dialogs.tsx";
import { Icon } from "./icons.tsx";
import { EmptyState, PushButton } from "./primitives.tsx";
import { copyText, openInTerminal } from "./sidebar.tsx";
import { ROW_HEIGHT, statusColor } from "./theme.ts";

export function ChangesView() {
  const app = useApp();
  const store = app.store;

  return (
    <View style={{ display: "flex", flex: 1, minWidth: 0, minHeight: 0, flexDirection: "row" }}>
      <ResizablePanel
        label="Resize file list"
        width={store.changesSplit()}
        onResize={store.setChangesSplit}
        minimum={220}
        maximum={800}
        style={{
          display: "flex",
          flexShrink: 0,
          minHeight: 0,
          flexDirection: "column",
        }}
      >
        <Show
          when={store.changeCount() > 0}
          fallback={
            <EmptyState
              ui={app}
              title="No local changes"
              description="The working tree matches the last commit."
            />
          }
        >
          <FileList list="unstaged" />
          <FileList list="staged" />
        </Show>
        <CommitComposer />
      </ResizablePanel>
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
  const stats = () =>
    props.list === "unstaged" ? store.numstat().unstaged : store.numstat().staged;
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
          ? {
              label: many ? `Stage ${targets.length} Files` : "Stage File",
              click: () => void store.stageItems(targets),
            }
          : {
              label: many ? `Unstage ${targets.length} Files` : "Unstage File",
              click: () => void store.unstageItems(targets),
            },
        ...(props.list === "unstaged"
          ? [
              {
                label: many ? `Discard ${targets.length} Files…` : "Discard Changes…",
                click: () => void discard(targets),
              },
            ]
          : []),
        { type: "separator" },
        {
          label: "Reveal in Finder",
          enabled: item.code !== "D",
          click: () => void Shell.showItemInFolder(absolute),
        },
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
      message:
        targets.length === 1
          ? `Discard changes to ${basename(targets[0]!.path)}?`
          : `Discard changes to ${targets.length} files?`,
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
        flex: 1,
        flexBasis: 0,
      }}
    >
      {/* The label starts where the rows' checkboxes start and the button ends where their counts end. */}
      <View
        style={{
          display: "flex",
          flexDirection: "row",
          alignItems: "center",
          height: 32,
          flexShrink: 0,
          paddingLeft: 12,
          paddingRight: 8,
          gap: 6,
        }}
      >
        <Text style={{ fontSize: 11, fontWeight: 600, color: app.theme().textTertiary }}>
          {label()}
        </Text>
        <Text style={{ fontSize: 11, color: app.theme().textTertiary }}>
          {String(items().length)}
        </Text>
        <View style={{ flex: 1 }} />
        <Show when={!empty()}>
          <PushButton
            ui={app}
            label={props.list === "unstaged" ? "Stage All" : "Unstage All"}
            disabled={store.busy() !== undefined}
            onClick={() => void (props.list === "unstaged" ? store.stageAll() : store.unstageAll())}
          />
        </Show>
      </View>
      <Show
        when={!empty()}
        fallback={
          <Text
            style={{
              paddingLeft: 12,
              paddingBottom: 10,
              fontSize: 12,
              color: app.theme().textTertiary,
            }}
          >
            {props.list === "unstaged" ? "No unstaged changes" : "Nothing staged yet"}
          </Text>
        }
      >
        <Table.Root
          scope={`changes-${props.list}`}
          rowCount={items().length}
          rowHeight={ROW_HEIGHT}
          headerHeight={0}
          selectionMode="multiple"
          selection={store.selection()[props.list]}
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
          onActiveCellChange={(cell) =>
            store.setActiveCell(cell ? { list: props.list, row: cell.row } : undefined)
          }
          onActivate={() => void store.toggleStaging(props.list)}
          onFocus={() => store.setFocusedList(props.list)}
          style={{ flex: 1, minHeight: 0, overflowY: "scroll" }}
        >
          <For each={visible()} keyed={(row) => row.index}>
            {(row) => (
              <Table.Row
                index={row().index}
                style={{
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
                    style={{
                      display: "flex",
                      flex: 1,
                      minWidth: 0,
                      flexDirection: "row",
                      alignItems: "center",
                      gap: 6,
                      height: "100%",
                    }}
                  >
                    <Text style={{ flex: 1, minWidth: 0, fontSize: 13, lineClamp: 1 }}>
                      {row().item.path}
                    </Text>
                    <Counts entry={stats().get(row().item.path)} />
                  </View>
                </Table.Cell>
              </Table.Row>
            )}
          </For>
        </Table.Root>
      </Show>
    </View>
  );
}

function StageToggle(props: { item: ChangeItem; list: ListId }) {
  const app = useApp();
  const store = app.store;
  const checked = () => props.list === "staged";
  return (
    <Checkbox.Root
      checked={checked()}
      aria-label={checked() ? `Unstage ${props.item.path}` : `Stage ${props.item.path}`}
      focusOnPointer={false}
      onCheckedChange={(next) =>
        void (next ? store.stageItems([props.item]) : store.unstageItems([props.item]))
      }
      style={{
        display: "flex",
        alignItems: "center",
        justifyContent: "center",
        width: 22,
        height: 22,
        borderRadius: 4,
        cursor: "default",
      }}
    >
      <Checkbox.Indicator style={checkboxBox(app, checked())}>
        <Show when={checked()}>
          <Icon name="check" size={11} color={app.theme().textOnAccent} />
        </Show>
      </Checkbox.Indicator>
    </Checkbox.Root>
  );
}

export function StatusGlyph(props: { code: string }) {
  const app = useApp();
  return (
    <Text style={{ width: 16, fontWeight: 700, color: statusColor(app.theme(), props.code) }}>
      {props.code}
    </Text>
  );
}

function Counts(props: { entry: { added: number | null; removed: number | null } | undefined }) {
  const app = useApp();
  const counts = () =>
    [
      props.entry?.added != null ? `+${props.entry.added}` : "",
      props.entry?.removed != null ? `-${props.entry.removed}` : "",
    ]
      .filter(Boolean)
      .join(" ");
  return (
    <Show when={counts()}>
      <Text style={{ fontSize: 11, fontFamily: "monospace", color: app.theme().textTertiary }}>
        {counts()}
      </Text>
    </Show>
  );
}

function CommitComposer() {
  const app = useApp();
  const store = app.store;
  const keymap = { "CmdOrCtrl+Enter": "commit", "CmdOrCtrl+Shift+G": "generate" } as const;
  const onAction = (event: QuickGuiEvent) => {
    const action = actionFromEvent(event);
    if (action === "commit") void store.commit();
    if (action === "generate") void store.generateMessage();
  };
  return (
    <View
      style={{
        display: "flex",
        flexDirection: "column",
        flexShrink: 0,
        gap: 8,
        padding: 12,
        borderTopWidth: 1,
        borderColor: app.theme().border,
      }}
    >
      <Input
        placeholder="Commit summary"
        value={store.subject()}
        onInput={(event) => store.setSubject(event.value ?? "")}
        onSubmit={() => void store.commit()}
        keymap={keymap}
        onAction={onAction}
        style={app.styles().input}
      />
      <TextArea
        placeholder="Description"
        value={store.body()}
        onInput={(event) => store.setBody(event.value ?? "")}
        keymap={keymap}
        onAction={onAction}
        style={app.styles().textArea}
      />
      <View style={{ display: "flex", flexDirection: "row", alignItems: "center", gap: 8 }}>
        <CheckRow label="Amend" checked={store.amend()} onChange={store.setAmend} />
        <View style={{ flex: 1 }} />
        <Show when={store.agents().length > 0}>
          <PushButton
            ui={app}
            label={store.generating() ? "Cancel" : "Generate"}
            onClick={() =>
              store.generating() ? store.cancelGeneration() : void store.generateMessage()
            }
          />
        </Show>
        <PushButton
          ui={app}
          kind="primary"
          label="Commit"
          disabled={!store.canCommit()}
          onClick={() => void store.commit()}
        />
      </View>
    </View>
  );
}
