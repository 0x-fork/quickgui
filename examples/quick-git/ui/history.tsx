import { KeyedFor } from "@quickgui/ui";
import { Menu, type QuickGuiEvent } from "@quickgui/native";
import { Button, Svg, Text, View, capturedPointerFromEvent, type VisibleRange } from "@quickgui/ui";
import { Table } from "@quickgui/ui/collections";
import { For, Show, createMemo, createSignal } from "@quickgui/ui";

import { absoluteTime, relativeTime, type Commit, type GraphRow } from "../git/log.ts";
import { useApp } from "./context.tsx";
import { DiffPane } from "./diff.tsx";
import { FileName, StatusGlyph } from "./changes.tsx";
import { Icon } from "./icons.tsx";
import { LANE_WIDTH, paintGraphRow } from "./graph.ts";
import { EmptyState } from "./primitives.tsx";
import { copyText } from "./sidebar.tsx";
import { CheckRow } from "./dialogs.tsx";

const HISTORY_ROW_HEIGHT = 26;

export function HistoryView() {
  const app = useApp();
  const store = app.store;
  const history = () => store.history();
  const [range, setRange] = createSignal<VisibleRange>({ start: 0, end: 0 });
  const visible = createMemo(() => {
    const commits = history().commits;
    const graph = history().graph;
    const { start, end } = range();
    const rows: { index: number; commit: Commit; graph: GraphRow | undefined }[] = [];
    for (let index = start; index < Math.min(end, commits.length); index += 1) {
      rows.push({ index, commit: commits[index]!, graph: graph[index] });
    }
    // Paging: ask for more once the viewport nears the end of what is loaded.
    if (end > commits.length - 60 && !history().exhausted && !history().loading && commits.length > 0) {
      queueMicrotask(() => void store.loadHistory(false));
    }
    return rows;
  });
  const maxLanes = createMemo(() => Math.min(8, Math.max(1, Math.max(...history().graph.slice(range().start, range().end).map((row) => row.laneCount)))));
  const graphWidth = () => 12 + maxLanes() * LANE_WIDTH;

  let splitDragStart = 0;
  function handleSplitPointer(event: QuickGuiEvent): void {
    const pointer = capturedPointerFromEvent(event);
    if (pointer === undefined) return;
    if (pointer.button !== "left") return;
    if (pointer.phase === "down") {
      splitDragStart = store.historySplit();
      return;
    }
    if (pointer.phase === "move") store.setHistorySplit(Math.min(1000, Math.max(260, splitDragStart + pointer.position.x - pointer.origin.x)));
  }

  function contextMenu(commit: Commit): void {
    void Menu.popup(
      [
        { label: "Copy SHA", click: () => void copyText(commit.sha) },
        { label: "Copy Short SHA", click: () => void copyText(commit.shortSha) },
        { label: "Copy Subject", click: () => void copyText(commit.subject) },
        { type: "separator" },
        { label: "New Branch from Here…", click: () => app.openDialog({ kind: "new-branch", from: commit.sha }) },
        { label: "Checkout (Detached)", click: () => void store.checkoutCommit(commit.sha) },
      ],
      { window: app.window },
    );
  }

  return (
    <View style={{ display: "flex", flex: 1, minWidth: 0, minHeight: 0, flexDirection: "row" }}>
      <View style={{ display: "flex", width: store.historySplit(), flexShrink: 0, minHeight: 0, flexDirection: "column" }}>
        <View style={{ display: "flex", flexDirection: "row", alignItems: "center", height: 32, flexShrink: 0, paddingLeft: 12, paddingRight: 8, gap: 8, borderBottomWidth: 1, borderColor: app.theme().border }}>
          <Text style={{ fontSize: 11, fontWeight: 700, letterSpacing: 0.3, textTransform: "uppercase", color: app.theme().textTertiary }}>
            {history().allBranches ? "All branches" : store.status()?.branch ?? "History"}
          </Text>
          <Text style={{ fontSize: 11, color: app.theme().textTertiary }}>{`${history().commits.length}${history().exhausted ? "" : "+"} commits`}</Text>
          <View style={{ flex: 1 }} />
          <CheckRow label="All branches" checked={history().allBranches} onChange={(value) => store.setHistoryAllBranches(value)} />
        </View>
        <Show
          when={history().commits.length > 0}
          fallback={
            <EmptyState ui={app} icon="history" title={history().loading ? "Loading history…" : "No commits yet"} description={history().loading ? undefined : "Commits appear here once you make the first one."} />
          }
        >
          <Table.Root
            rowCount={history().commits.length}
            rowHeight={HISTORY_ROW_HEIGHT}
            headerHeight={0}
            selectionMode="single"
            selection={store.historySelection().map((range) => [...range])}
            columns={[
              { id: "graph", track: `${graphWidth()}px` },
              { id: "subject", track: "1fr", rowHeader: true },
              { id: "author", track: "110px" },
              { id: "date", track: "84px", align: "end" },
            ]}
            onVisibleRangeChange={setRange}
            onSelectionChange={(ranges) => store.setHistorySelection(ranges)}
            style={{ flex: 1, minHeight: 0, overflowY: "scroll" }}
          >
            <KeyedFor each={visible()} key={(row) => row.index}>
              {(row) => (
                <Table.Row
                  index={row().index}
                  style={{ transition: "background-color 60ms", hover: { backgroundColor: app.theme().hover }, selected: { backgroundColor: app.theme().selection } }}
                >
                  <Table.Cell column="graph">
                    <Graph row={row().graph} width={graphWidth()} height={HISTORY_ROW_HEIGHT} />
                  </Table.Cell>
                  <Table.Cell column="subject" style={{ minWidth: 0, gap: 6, paddingRight: 8 }}>
                    <View onContextMenu={() => contextMenu(row().commit)} style={{ display: "flex", flex: 1, minWidth: 0, flexDirection: "row", alignItems: "center", gap: 6, height: "100%" }}>
                      <For each={row().commit.refs.filter((ref) => ref.kind !== "head").slice(0, 3)}>
                        {(ref) => (
                          <View
                            style={{
                              display: "flex",
                              height: 16,
                              flexShrink: 0,
                              alignItems: "center",
                              paddingLeft: 5,
                              paddingRight: 5,
                              borderRadius: 4,
                              backgroundColor: ref.current ? app.theme().accent : ref.kind === "tag" ? app.theme().warningWash : app.theme().accentWash,
                              maxWidth: 140,
                            }}
                          >
                            <Text style={{ fontSize: 10.5, fontWeight: 700, color: ref.current ? app.theme().textOnAccent : ref.kind === "tag" ? app.theme().warning : app.theme().accent, lineClamp: 1, textOverflow: "ellipsis" }}>
                              {ref.name}
                            </Text>
                          </View>
                        )}
                      </For>
                      <Text style={{ flex: 1, minWidth: 0, fontSize: 12.5, color: app.theme().text, lineClamp: 1, textOverflow: "ellipsis" }}>{row().commit.subject}</Text>
                    </View>
                  </Table.Cell>
                  <Table.Cell column="author" style={{ minWidth: 0, paddingRight: 8 }}>
                    <Text style={{ fontSize: 11.5, color: app.theme().textSecondary, lineClamp: 1, textOverflow: "ellipsis" }}>{row().commit.authorName}</Text>
                  </Table.Cell>
                  <Table.Cell column="date" style={{ paddingRight: 10 }}>
                    <Text tooltip={absoluteTime(row().commit.authorTime)} style={{ fontSize: 11, color: app.theme().textTertiary, whiteSpace: "nowrap" }}>
                      {relativeTime(row().commit.authorTime)}
                    </Text>
                  </Table.Cell>
                </Table.Row>
              )}
            </KeyedFor>
          </Table.Root>
        </Show>
      </View>
      <View aria-label="Resize history" hitSlopLeft={4} hitSlopRight={4} onPointer={handleSplitPointer} style={[app.styles().vhairline, { cursor: "col-resize", appRegion: "no-drag" }]} />
      <CommitDetail />
    </View>
  );
}

/**
 * One row of the commit graph: one tinted SVG mask per lane color, so lines stay continuous
 * across rows and elbows are real quarter circles rather than stacked rectangles.
 */
function Graph(props: { row: () => (GraphRow | undefined); width: () => (number); height: () => (number) }) {
  const app = useApp();
  const palette = () => app.theme().graph;
  const layers = createMemo(() => (props.row() ? paintGraphRow(props.row()!, props.width(), props.height()) : []));
  return (
    <View style={{ position: "relative", width: "100%", height: props.height() }}>
      <For each={layers()}>
        {(layer) => (
          <Svg
            source={layer.source}
            style={{ position: "absolute", left: 0, top: 0, width: props.width(), height: props.height(), color: palette()[layer.color % palette().length]! }}
          />
        )}
      </For>
    </View>
  );
}

function CommitDetail() {
  const app = useApp();
  const store = app.store;
  const commit = () => store.selectedCommit();
  const detail = () => store.commitDetail();
  const selectedPath = () => detail().selectedPath ?? detail().files.at(0)?.path;

  return (
    <View style={{ display: "flex", flex: 1, minWidth: 0, minHeight: 0, flexDirection: "column" }}>
      <Show when={commit()} fallback={<EmptyState ui={app} icon="commit" title="Select a commit" description="Its message and changed files appear here." />}>
        {(() => { const current = () => (commit())!; return (
          <>
            <View style={{ display: "flex", flexShrink: 0, flexDirection: "column", gap: 8, padding: 14, borderBottomWidth: 1, borderColor: app.theme().border }}>
              <Text style={{ fontSize: 14, fontWeight: 700, color: app.theme().text, lineHeight: 19 }}>{current().subject}</Text>
              <Show when={current().body}>
                <Text style={{ fontSize: 12.5, lineHeight: 18, color: app.theme().textSecondary, fontFamily: "monospace", userSelect: "text" }}>{current().body}</Text>
              </Show>
              <View style={{ display: "flex", flexDirection: "row", alignItems: "center", gap: 10, flexWrap: "wrap" }}>
                <Text style={{ fontSize: 12, color: app.theme().text, fontWeight: 600 }}>{current().authorName}</Text>
                <Text style={{ fontSize: 12, color: app.theme().textTertiary }}>{absoluteTime(current().authorTime)}</Text>
                <Button
                  aria-label="Copy SHA"
                  tooltip="Copy full SHA"
                  focusOnPointer={false}
                  onClick={() => void copyText(current().sha)}
                  style={[app.styles().tinyButton, { fontFamily: "monospace", gap: 5 }]}
                >
                  <Text>{current().shortSha}</Text>
                  <Icon name="copy" size={11} />
                </Button>
                <Show when={current().parents.length > 1}>
                  <Text style={{ fontSize: 11, color: app.theme().textTertiary }}>{`Merge of ${current().parents.length} parents`}</Text>
                </Show>
              </View>
            </View>
            <View style={{ display: "flex", flexShrink: 0, flexDirection: "column", maxHeight: 220, overflowY: "auto", paddingTop: 4, paddingBottom: 4 }}>
              <Show when={detail().loading}>
                <Text style={{ paddingLeft: 14, paddingTop: 6, fontSize: 12, color: app.theme().textTertiary }}>Loading files…</Text>
              </Show>
              <For each={detail().files}>
                {(file) => (
                  <Button
                    aria-label={file.path}
                    focusOnPointer={false}
                    selected={selectedPath() === file.path}
                    onClick={() => store.selectCommitFile(file.path)}
                    style={{
                      display: "flex",
                      flexDirection: "row",
                      alignItems: "center",
                      gap: 8,
                      height: 26,
                      flexShrink: 0,
                      paddingLeft: 14,
                      paddingRight: 12,
                      backgroundColor: "transparent",
                      cursor: "default",
                      userSelect: "none",
                      hover: { backgroundColor: app.theme().hover },
                      selected: { backgroundColor: app.theme().selection },
                      focus: { outline: `2px solid ${app.theme().focusRing}` },
                    }}
                  >
                    <StatusGlyph code={file.status} />
                    <FileName path={file.path} originalPath={file.originalPath} />
                  </Button>
                )}
              </For>
            </View>
            <View style={app.styles().hairline} />
            <DiffPane />
          </>
        ); })()}
      </Show>
    </View>
  );
}
