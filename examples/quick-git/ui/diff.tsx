import { KeyedFor } from "@quickgui/ui";
import { Button, Text, View, type VisibleRange } from "@quickgui/ui";
import { Table } from "@quickgui/ui/collections";
import { Button as SwiftButton, Host } from "@quickgui/ui/swift-ui";
import { buttonStyle, controlSize } from "@quickgui/ui/swift-ui/modifiers";
import { For, Show, createMemo, createSignal } from "@quickgui/ui";
import { basename } from "node:path";

import type { DiffRow } from "../model/store.ts";
import { FileName } from "./changes.tsx";
import { useApp } from "./context.tsx";
import { confirm } from "./dialogs.tsx";
import { Icon } from "./icons.tsx";
import { EmptyState } from "./primitives.tsx";
import { DIFF_ROW_HEIGHT, MONO_FONT_SIZE } from "./theme.ts";

const MAX_LINE_CHARACTERS = 2000;

export function DiffPane() {
  const app = useApp();
  const store = app.store;
  const diff = () => store.diff();
  const target = () => diff().target;
  const mode = () => target()?.kind ?? "unstaged";
  const rowCount = () => store.diffRowCount();
  const [range, setRange] = createSignal<VisibleRange>({ start: 0, end: 0 });
  // Only the rows the table is about to paint leave the native module.
  const visible = createMemo(() => {
    const { start, end } = range();
    const rows = store.diffRowsIn(start, Math.min(end, rowCount()));
    return rows.map((row, offset): { index: number; row: DiffRow } => ({ index: start + offset, row }));
  });
  const primaryFile = () => diff().diff?.files.at(0);
  const lineCount = () => store.selectedDiffLineCount();
  const hasLineSelection = () => lineCount() > 0;
  const activeItem = () => store.activeItem();

  async function discardWholeFile(): Promise<void> {
    const item = activeItem();
    if (!item) return;
    const ok = await confirm(app, {
      message: `Discard changes to ${basename(item.path)}?`,
      detail: item.entry.kind === "untracked" ? "The file will be moved to the Trash." : "The changes cannot be recovered.",
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
    <View style={{ display: "flex", flex: 1, minWidth: 0, minHeight: 0, flexDirection: "column", backgroundColor: app.theme().content }}>
      <Show
        when={target()}
        fallback={
          <EmptyState
            ui={app}
            icon="file"
            title={store.view() === "history" ? "Select a commit" : "Select a file"}
            description={store.view() === "history" ? "Its changes appear here." : "Choose a change on the left to review its diff, then stage hunks or single lines."}
          />
        }
      >
        <View style={{ display: "flex", flexDirection: "row", alignItems: "center", gap: 8, height: 40, flexShrink: 0, paddingLeft: 14, paddingRight: 10, borderBottomWidth: 1, borderColor: app.theme().border }}>
          <Icon name="file" size={14} color={app.theme().textTertiary} />
          <View style={{ display: "flex", flexShrink: 1, minWidth: 0 }}>
            <FileName path={target()!.path} originalPath={target()!.originalPath} size={13} />
          </View>
          <Show when={primaryFile()}>
            {(() => { const file = () => (primaryFile())!; return (
              <View style={{ display: "flex", flexDirection: "row", alignItems: "center", gap: 6, flexShrink: 0 }}>
                <Show when={file().kind !== "modified"}>
                  <Text style={{ fontSize: 11, fontWeight: 700, color: app.theme().textTertiary, textTransform: "capitalize" }}>{file().kind}</Text>
                </Show>
                <Show when={file().oldMode && file().newMode && file().oldMode !== file().newMode}>
                  <Text style={{ fontSize: 11, color: app.theme().textTertiary, fontFamily: "monospace" }}>{`${file().oldMode} → ${file().newMode}`}</Text>
                </Show>
                <Show when={store.diffStats().added > 0}>
                  <Text style={{ fontSize: 11, fontWeight: 700, color: app.theme().success, fontFamily: "monospace" }}>{`+${store.diffStats().added}`}</Text>
                </Show>
                <Show when={store.diffStats().removed > 0}>
                  <Text style={{ fontSize: 11, fontWeight: 700, color: app.theme().danger, fontFamily: "monospace" }}>{`−${store.diffStats().removed}`}</Text>
                </Show>
              </View>
            ); })()}
          </Show>
          <View style={{ flex: 1 }} />
          <Show when={diff().loading}>
            <Text style={{ fontSize: 11, color: app.theme().textTertiary }}>Loading…</Text>
          </Show>
          <Show when={mode() === "unstaged"}>
            <Show
              when={hasLineSelection()}
              fallback={
                <>
                  <Host matchContents>
                    <SwiftButton label="Discard…" role="destructive" modifiers={[buttonStyle("bordered"), controlSize("small")]} onPress={() => void discardWholeFile()} />
                  </Host>
                  <Host matchContents>
                    <SwiftButton label="Stage File" modifiers={[buttonStyle("borderedProminent"), controlSize("small")]} onPress={() => { if (activeItem()) void store.stageItems([activeItem()!]); }} />
                  </Host>
                </>
              }
            >
              <Host matchContents>
                <SwiftButton label={`Discard ${lineCount()} ${lineCount() === 1 ? "Line" : "Lines"}…`} role="destructive" modifiers={[buttonStyle("bordered"), controlSize("small")]} onPress={() => void discardLines()} />
              </Host>
              <Host matchContents>
                <SwiftButton label={`Stage ${lineCount()} ${lineCount() === 1 ? "Line" : "Lines"}`} modifiers={[buttonStyle("borderedProminent"), controlSize("small")]} onPress={() => void store.applySelectedLines("stage")} />
              </Host>
            </Show>
          </Show>
          <Show when={mode() === "staged"}>
            <Show
              when={hasLineSelection()}
              fallback={
                <Host matchContents>
                  <SwiftButton label="Unstage File" modifiers={[buttonStyle("bordered"), controlSize("small")]} onPress={() => { if (activeItem()) void store.unstageItems([activeItem()!]); }} />
                </Host>
              }
            >
              <Host matchContents>
                <SwiftButton label={`Unstage ${lineCount()} ${lineCount() === 1 ? "Line" : "Lines"}`} modifiers={[buttonStyle("borderedProminent"), controlSize("small")]} onPress={() => void store.applySelectedLines("unstage")} />
              </Host>
            </Show>
          </Show>
        </View>
        <Show when={diff().error}>
          <View style={{ display: "flex", flexDirection: "row", alignItems: "center", gap: 8, padding: 12, backgroundColor: app.theme().dangerWash }}>
            <Icon name="alert" size={14} color={app.theme().danger} />
            <Text style={{ fontSize: 12, color: app.theme().danger }}>{diff().error}</Text>
          </View>
        </Show>
        <Show
          when={rowCount() > 0}
          fallback={
            <Show when={!diff().loading}>
              <EmptyState
                ui={app}
                icon="file"
                title={primaryFile()?.binary ? "Binary file" : primaryFile() && primaryFile()!.hunkCount === 0 ? "No textual changes" : "Empty file"}
                description={primaryFile()?.binary ? "Binary changes are staged and committed whole." : primaryFile() && primaryFile()!.oldMode !== primaryFile()!.newMode ? "Only the file mode changed." : undefined}
              />
            </Show>
          }
        >
          <Table.Root
            rowCount={rowCount()}
            rowHeight={DIFF_ROW_HEIGHT}
            headerHeight={0}
            selectionMode="multiple"
            selection={store.diffSelection().map((range) => [...range])}
            columns={[
              { id: "old", track: "46px", align: "end" },
              { id: "new", track: "46px", align: "end" },
              { id: "text", track: "1fr" },
            ]}
            onVisibleRangeChange={setRange}
            onSelectionChange={(ranges) => store.setDiffSelection(ranges)}
            style={{ flex: 1, minHeight: 0, overflowY: "scroll" }}
          >
            <KeyedFor each={visible()} key={(entry) => entry.index}>
              {(entry) => <DiffTableRow index={entry().index} row={entry().row} mode={mode()} />}
            </KeyedFor>
          </Table.Root>
        </Show>
      </Show>
    </View>
  );
}

function DiffTableRow(props: { index: () => number; row: () => DiffRow; mode: "unstaged" | "staged" | "commit" }) {
  const app = useApp();
  const store = app.store;
  const theme = app.theme;
  const row = props.row;

  const background = () => {
    const current = row();
    if (current.kind === "line") return current.lineKind === "added" ? theme().diffAdded : current.lineKind === "removed" ? theme().diffRemoved : "transparent";
    if (current.kind === "hunk") return theme().diffHunk;
    if (current.kind === "file") return theme().contentAlt;
    return "transparent";
  };
  const selectedBackground = () => {
    const current = row();
    if (current.kind === "line") return current.lineKind === "added" ? theme().diffSelectedAdded : current.lineKind === "removed" ? theme().diffSelectedRemoved : theme().diffSelectedContext;
    return theme().diffSelectedContext;
  };
  const gutter = () => {
    const current = row();
    if (current.kind !== "line") return background();
    return current.lineKind === "added" ? theme().diffAddedGutter : current.lineKind === "removed" ? theme().diffRemovedGutter : theme().contentAlt;
  };
  const textColor = () => {
    const current = row();
    if (current.kind === "hunk") return theme().diffHunkText;
    if (current.kind === "notice") return theme().textTertiary;
    return theme().text;
  };
  const marker = () => {
    const current = row();
    if (current.kind !== "line") return " ";
    return current.lineKind === "added" ? "+" : current.lineKind === "removed" ? "−" : " ";
  };
  const text = () => {
    const current = row();
    if (current.kind !== "line") return current.text;
    const content = current.text.replace(/\t/g, "    ");
    return content.length > MAX_LINE_CHARACTERS ? `${content.slice(0, MAX_LINE_CHARACTERS)}…` : content;
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
      index={props.index()}
      group
      style={{
        backgroundColor: background(),
        selected: { backgroundColor: selectedBackground() },
      }}
    >
      <Table.Cell column="old" style={{ backgroundColor: gutter() }}>
        <Show when={row().kind === "line" && row().oldLineNumber !== null}>
          <Text style={numberStyle()}>{String(row().oldLineNumber)}</Text>
        </Show>
      </Table.Cell>
      <Table.Cell column="new" style={{ backgroundColor: gutter() }}>
        <Show when={row().kind === "line" && row().newLineNumber !== null}>
          <Text style={numberStyle()}>{String(row().newLineNumber)}</Text>
        </Show>
      </Table.Cell>
      <Table.Cell column="text" style={{ paddingLeft: 8, paddingRight: 8, minWidth: 0, gap: 8 }}>
        <Text
          style={{
            width: 12,
            flexShrink: 0,
            fontFamily: "monospace",
            fontSize: MONO_FONT_SIZE,
            fontWeight: 700,
            color: marker() === "+" ? theme().diffAddedText : marker() === "−" ? theme().diffRemovedText : theme().textTertiary,
            userSelect: "none",
          }}
        >
          {marker()}
        </Text>
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
          <View style={{ display: "flex", flexDirection: "row", gap: 4, flexShrink: 0, opacity: 0, groupHover: { opacity: 1 } }}>
            <Show when={props.mode === "unstaged"}>
              <HunkButton
                label="Discard hunk"
                danger
                onClick={() => void discardHunk(app, row().fileIndex, row().hunkIndex)}
              />
              <HunkButton label="Stage hunk" onClick={() => void store.stageHunk(row().fileIndex, row().hunkIndex)} />
            </Show>
            <Show when={props.mode === "staged"}>
              <HunkButton label="Unstage hunk" onClick={() => void store.unstageHunk(row().fileIndex, row().hunkIndex)} />
            </Show>
          </View>
        </Show>
        <Show when={row().kind === "line" && row().noNewline}>
          <Text tooltip="No newline at end of file" style={{ fontSize: 10, color: theme().textTertiary, flexShrink: 0 }}>⏎̸</Text>
        </Show>
      </Table.Cell>
    </Table.Row>
  );
}

async function discardHunk(app: ReturnType<typeof useApp>, fileIndex: number, hunkIndex: number): Promise<void> {
  const ok = await confirm(app, {
    message: "Discard this hunk?",
    detail: "The changes cannot be recovered.",
    confirmLabel: "Discard",
    destructive: true,
  });
  if (ok) await app.store.discardHunk(fileIndex, hunkIndex);
}

function HunkButton(props: { label: () => (string); danger?: () => (boolean); onClick: () => void }) {
  const readdanger = () => { const source = props.danger; return source === undefined ? undefined : source(); };
  const app = useApp();
  return (
    <Button
      aria-label={props.label()}
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
        color: readdanger() ? app.theme().danger : app.theme().text,
        fontSize: 10.5,
        fontWeight: 600,
        cursor: "default",
        userSelect: "none",
        hover: { backgroundColor: readdanger() ? app.theme().dangerWash : app.theme().accentWash },
        active: { opacity: 0.8 },
      }}
    >
      <Text>{props.label()}</Text>
    </Button>
  );
}
