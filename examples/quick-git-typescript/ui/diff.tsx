import { Button, Table, Text, View, type VisibleRange } from "@quickgui/solid";
import { For, Show, createMemo, createSignal } from "solid-js";
import { basename } from "node:path";

import type { DiffRow } from "../model/store.ts";
import { useApp } from "./context.tsx";
import { confirm } from "./dialogs.tsx";
import { Icon } from "./icons.tsx";
import { EmptyState, PushButton } from "./primitives.tsx";
import { DIFF_ROW_HEIGHT, MONO_FONT_SIZE } from "./theme.ts";

const MAX_LINE_CHARACTERS = 2000;

export function DiffPane() {
  const app = useApp();
  const store = app.store;
  const diff = () => store.diff();
  const target = () => diff().target;
  const mode = () => target()?.kind ?? "unstaged";
  const rows = () => store.diffRows();
  const [range, setRange] = createSignal<VisibleRange>({ start: 0, end: 0 });
  const visible = createMemo(() => {
    const all = rows();
    const { start, end } = range();
    const out: { index: number; row: DiffRow }[] = [];
    for (let index = start; index < Math.min(end, all.length); index += 1)
      out.push({ index, row: all[index]! });
    return out;
  });
  const primaryFile = () => diff().files[0];
  const lineCount = () => store.selectedDiffLineCount();
  const hasLineSelection = () => lineCount() > 0;
  const activeItem = () => store.activeItem();

  async function discardWholeFile(): Promise<void> {
    const item = activeItem();
    if (!item) return;
    const ok = await confirm(app, {
      message: `Discard changes to ${basename(item.path)}?`,
      detail:
        item.entry.kind === "untracked"
          ? "The file will be moved to the Trash."
          : "The changes cannot be recovered.",
      confirmLabel: item.entry.kind === "untracked" ? "Move to Trash" : "Discard",
      destructive: true,
    });
    if (ok) await store.discardItems([item]);
  }

  async function discardLines(): Promise<void> {
    const ok = await confirm(app, {
      message: `Discard ${lineCount()} selected ${lineCount() === 1 ? "line" : "lines"}?`,
      detail: "The changes cannot be recovered.",
      confirmLabel: "Discard",
      destructive: true,
    });
    if (ok) await store.applySelectedLines("discard");
  }

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
      <Show
        when={target() !== undefined}
        fallback={
          <EmptyState
            ui={app}
            title={store.view() === "history" ? "Select a commit" : "Select a file"}
            description={
              store.view() === "history"
                ? "Its changes appear here."
                : "Choose a change on the left to review its diff, then stage hunks or single lines."
            }
          />
        }
      >
        <View
          style={{
            display: "flex",
            flexDirection: "row",
            alignItems: "center",
            gap: 8,
            height: 40,
            flexShrink: 0,
            paddingLeft: 14,
            paddingRight: 10,
            borderBottomWidth: 1,
            borderColor: app.theme().border,
          }}
        >
          <Text style={{ flex: 1, minWidth: 0, fontSize: 13, fontWeight: 600, lineClamp: 1 }}>
            {target()?.path}
          </Text>
          <Show when={store.diffStats().added > 0}>
            <Text
              style={{
                fontSize: 11,
                fontWeight: 700,
                color: app.theme().success,
                fontFamily: "monospace",
              }}
            >{`+${store.diffStats().added}`}</Text>
          </Show>
          <Show when={store.diffStats().removed > 0}>
            <Text
              style={{
                fontSize: 11,
                fontWeight: 700,
                color: app.theme().danger,
                fontFamily: "monospace",
              }}
            >{`-${store.diffStats().removed}`}</Text>
          </Show>
          <Show when={diff().loading && diff().files.length === 0}>
            <Text style={{ fontSize: 11, color: app.theme().textTertiary }}>Loading…</Text>
          </Show>
          <Show when={mode() === "unstaged"}>
            <Show
              when={hasLineSelection()}
              fallback={
                <>
                  <PushButton
                    ui={app}
                    label="Discard…"
                    kind="danger"

                    onClick={() => void discardWholeFile()}
                  />

                  <PushButton
                    ui={app}
                    label="Stage File"
                    kind="primary"
                    onClick={() => activeItem() && void store.stageItems([activeItem()!])}
                  />
                </>
              }
            >
              <PushButton
                ui={app}
                label={`Discard ${lineCount()} ${lineCount() === 1 ? "Line" : "Lines"}…`}
                kind="danger"

                onClick={() => void discardLines()}
              />

              <PushButton
                ui={app}
                label={`Stage ${lineCount()} ${lineCount() === 1 ? "Line" : "Lines"}`}
                kind="primary"
                onClick={() => void store.applySelectedLines("stage")}
              />
            </Show>
          </Show>
          <Show when={mode() === "staged"}>
            <Show
              when={hasLineSelection()}
              fallback={
                <PushButton
                  ui={app}
                  label="Unstage File"

                  onClick={() => activeItem() && void store.unstageItems([activeItem()!])}
                />
              }
            >
              <PushButton
                ui={app}
                label={`Unstage ${lineCount()} ${lineCount() === 1 ? "Line" : "Lines"}`}
                kind="primary"
                onClick={() => void store.applySelectedLines("unstage")}
              />
            </Show>
          </Show>
        </View>
        <Show when={diff().error}>
          <View
            style={{
              display: "flex",
              flexDirection: "row",
              alignItems: "center",
              gap: 8,
              padding: 12,
              backgroundColor: app.theme().dangerWash,
            }}
          >
            <Icon name="alert" size={14} color={app.theme().danger} />
            <Text style={{ fontSize: 12, color: app.theme().danger }}>{diff().error}</Text>
          </View>
        </Show>
        <Show
          when={rows().length > 0}
          fallback={
            <Show when={!diff().loading}>
              <EmptyState
                ui={app}
                title={
                  primaryFile()?.binary
                    ? "Binary file"
                    : primaryFile() && primaryFile()!.hunks.length === 0
                      ? "No textual changes"
                      : "Empty file"
                }
                description={
                  primaryFile()?.binary
                    ? "Binary changes are staged and committed whole."
                    : primaryFile() && primaryFile()!.oldMode !== primaryFile()!.newMode
                      ? "Only the file mode changed."
                      : undefined
                }
              />
            </Show>
          }
        >
          <Table.Root
            scope="diff"
            rowCount={rows().length}
            rowHeight={DIFF_ROW_HEIGHT}
            headerHeight={0}
            selectionMode="multiple"
            selection={store.diffSelection()}
            columns={[
              { id: "old", track: "46px", align: "end" },
              { id: "new", track: "46px", align: "end" },
              { id: "text", track: "1fr" },
            ]}
            onVisibleRangeChange={setRange}
            onSelectionChange={(ranges) => store.setDiffSelection(ranges)}
            style={{ flex: 1, minHeight: 0, overflowY: "scroll" }}
          >
            <For each={visible()} keyed={(entry) => entry.index}>
              {(entry) => <DiffTableRow index={entry().index} row={entry().row} mode={mode()} />}
            </For>
          </Table.Root>
        </Show>
      </Show>
    </View>
  );
}

function DiffTableRow(props: {
  index: number;
  row: DiffRow;
  mode: "unstaged" | "staged" | "commit";
}) {
  const app = useApp();
  const store = app.store;
  const theme = app.theme;
  const row = () => props.row;

  const background = () => {
    const current = row();
    if (current.kind === "line")
      return current.line.kind === "added"
        ? theme().diffAdded
        : current.line.kind === "removed"
          ? theme().diffRemoved
          : "transparent";
    if (current.kind === "hunk") return theme().diffHunk;
    if (current.kind === "file") return theme().contentAlt;
    return "transparent";
  };
  const selectedBackground = () => {
    const current = row();
    if (current.kind === "line")
      return current.line.kind === "added"
        ? theme().diffSelectedAdded
        : current.line.kind === "removed"
          ? theme().diffSelectedRemoved
          : theme().diffSelectedContext;
    return theme().diffSelectedContext;
  };
  const gutter = () => {
    const current = row();
    if (current.kind !== "line") return background();
    return current.line.kind === "added"
      ? theme().diffAddedGutter
      : current.line.kind === "removed"
        ? theme().diffRemovedGutter
        : theme().contentAlt;
  };
  const textColor = () => {
    const current = row();
    if (current.kind === "hunk") return theme().diffHunkText;
    if (current.kind === "notice") return theme().textTertiary;
    if (current.kind === "line" && current.line.kind === "added") return theme().diffAddedText;
    if (current.kind === "line" && current.line.kind === "removed") return theme().diffRemovedText;
    return theme().text;
  };
  const marker = () => {
    const current = row();
    if (current.kind !== "line") return " ";
    return current.line.kind === "added" ? "+" : current.line.kind === "removed" ? "−" : " ";
  };
  const text = () => {
    const current = row();
    switch (current.kind) {
      case "line": {
        const content = current.line.text.replace(/\t/g, "    ");
        return content.length > MAX_LINE_CHARACTERS
          ? `${content.slice(0, MAX_LINE_CHARACTERS)}…`
          : content;
      }
      case "hunk":
        return `@@ -${current.hunk.oldStart},${current.hunk.oldLines} +${current.hunk.newStart},${current.hunk.newLines} @@${current.hunk.heading ? ` ${current.hunk.heading}` : ""}`;
      case "file":
        return current.file.path;
      case "notice":
        return current.text;
    }
  };
  const numberStyle = () => ({
    fontFamily: "monospace" as const,
    fontSize: MONO_FONT_SIZE - 1,
    color: theme().diffLineNumber,
    textAlign: "end" as const,
    paddingRight: 6,
    userSelect: "none" as const,
  });

  return (
    <Table.Row
      index={props.index}
      group
      style={{
        backgroundColor: background(),
        selected: { backgroundColor: selectedBackground() },
      }}
    >
      <Table.Cell column="old" style={{ backgroundColor: gutter() }}>
        <Show
          when={
            row().kind === "line" &&
            (row() as Extract<DiffRow, { kind: "line" }>).line.oldLineNumber !== null
          }
        >
          <Text style={numberStyle()}>
            {String((row() as Extract<DiffRow, { kind: "line" }>).line.oldLineNumber)}
          </Text>
        </Show>
      </Table.Cell>
      <Table.Cell column="new" style={{ backgroundColor: gutter() }}>
        <Show
          when={
            row().kind === "line" &&
            (row() as Extract<DiffRow, { kind: "line" }>).line.newLineNumber !== null
          }
        >
          <Text style={numberStyle()}>
            {String((row() as Extract<DiffRow, { kind: "line" }>).line.newLineNumber)}
          </Text>
        </Show>
      </Table.Cell>
      <Table.Cell column="text" style={{ paddingLeft: 8, paddingRight: 8, minWidth: 0, gap: 8 }}>
        <View style={{ width: 12, flexShrink: 0 }}>
          <Show when={marker() === "+" || marker() === "−"}>
            <Icon
              name={marker() === "+" ? "plus" : "minus"}
              size={12}
              color={marker() === "+" ? theme().diffAddedText : theme().diffRemovedText}
            />
          </Show>
        </View>
        <Text
          style={{
            flex: 1,
            minWidth: 0,
            fontFamily: "monospace",
            fontSize: MONO_FONT_SIZE,
            color: textColor(),
            whiteSpace: "nowrap",
            textOverflow: "clip",
            ...(row().kind === "file" ? { fontWeight: 700 } : {}),
          }}
        >
          {text()}
        </Text>
        <Show when={row().kind === "hunk" && props.mode !== "commit"}>
          <View
            style={{
              display: "flex",
              flexDirection: "row",
              gap: 4,
              flexShrink: 0,
            }}
          >
            <Show when={props.mode === "unstaged"}>
              <HunkButton
                label="Discard hunk"
                danger
                onClick={() =>
                  void discardHunk(
                    app,
                    (row() as Extract<DiffRow, { kind: "hunk" }>).fileIndex,
                    (row() as Extract<DiffRow, { kind: "hunk" }>).hunkIndex,
                  )
                }
              />
              <HunkButton
                label="Stage hunk"
                onClick={() =>
                  void store.stageHunk(
                    (row() as Extract<DiffRow, { kind: "hunk" }>).fileIndex,
                    (row() as Extract<DiffRow, { kind: "hunk" }>).hunkIndex,
                  )
                }
              />
            </Show>
            <Show when={props.mode === "staged"}>
              <HunkButton
                label="Unstage hunk"
                onClick={() =>
                  void store.unstageHunk(
                    (row() as Extract<DiffRow, { kind: "hunk" }>).fileIndex,
                    (row() as Extract<DiffRow, { kind: "hunk" }>).hunkIndex,
                  )
                }
              />
            </Show>
          </View>
        </Show>
        <Show
          when={
            row().kind === "line" && (row() as Extract<DiffRow, { kind: "line" }>).line.noNewline
          }
        >
          <Text
            tooltip="No newline at end of file"
            style={{ fontSize: 10, color: theme().textTertiary, flexShrink: 0 }}
          >
            ⏎̸
          </Text>
        </Show>
      </Table.Cell>
    </Table.Row>
  );
}

async function discardHunk(
  app: ReturnType<typeof useApp>,
  fileIndex: number,
  hunkIndex: number,
): Promise<void> {
  const ok = await confirm(app, {
    message: "Discard this hunk?",
    detail: "The changes cannot be recovered.",
    confirmLabel: "Discard",
    destructive: true,
  });
  if (ok) await app.store.discardHunk(fileIndex, hunkIndex);
}

function HunkButton(props: { label: string; danger?: boolean; onClick: () => void }) {
  const app = useApp();
  return (
    <Button
      aria-label={props.label}
      focusOnPointer={false}
      onClick={props.onClick}
      style={{
        display: "flex",
        height: 16,
        alignItems: "center",
        justifyContent: "center",
        paddingLeft: 6,
        paddingRight: 6,
        borderRadius: 4,
        backgroundColor: app.theme().raised,
        borderWidth: 1,
        borderColor: app.theme().borderStrong,
        color: props.danger ? app.theme().danger : app.theme().text,
        fontSize: 10.5,
        fontWeight: 600,
        cursor: "default",
        userSelect: "none",
        hover: { backgroundColor: props.danger ? app.theme().dangerWash : app.theme().accentWash },
        active: { opacity: 0.8 },
      }}
    >
      <Text>{props.label}</Text>
    </Button>
  );
}
