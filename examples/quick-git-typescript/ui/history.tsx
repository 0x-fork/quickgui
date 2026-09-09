import { Menu } from "@quickgui/native";
import { Svg, Table, Text, View, type VisibleRange } from "@quickgui/solid";
import { For, Show, createMemo, createSignal } from "solid-js";

import { absoluteTime, relativeTime, type Commit, type GraphRow } from "../git/log.ts";
import { useApp } from "./context.tsx";
import { ResizablePanel } from "./resize.tsx";
import { DiffPane } from "./diff.tsx";
import { StatusGlyph } from "./changes.tsx";
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
    if (
      end > commits.length - 60 &&
      !history().exhausted &&
      !history().loading &&
      commits.length > 0
    ) {
      queueMicrotask(() => void store.loadHistory(false));
    }
    return rows;
  });
  const maxLanes = createMemo(() =>
    Math.min(
      8,
      Math.max(
        1,
        ...history()
          .graph.slice(range().start, range().end)
          .map((row) => row.laneCount),
      ),
    ),
  );
  const graphWidth = () => 12 + maxLanes() * LANE_WIDTH;

  function contextMenu(commit: Commit): void {
    void Menu.popup(
      [
        { label: "Copy SHA", click: () => void copyText(commit.sha) },
        { label: "Copy Short SHA", click: () => void copyText(commit.shortSha) },
        { label: "Copy Subject", click: () => void copyText(commit.subject) },
        { type: "separator" },
        {
          label: "New Branch from Here…",
          click: () => app.openDialog({ kind: "new-branch", from: commit.sha }),
        },
        { label: "Checkout (Detached)", click: () => void store.checkoutCommit(commit.sha) },
      ],
      { window: app.window },
    );
  }

  return (
    <View style={{ display: "flex", flex: 1, minWidth: 0, minHeight: 0, flexDirection: "row" }}>
      <ResizablePanel
        label="Resize history"
        width={store.historySplit()}
        onResize={store.setHistorySplit}
        minimum={260}
        maximum={1000}
        style={{
          display: "flex",
          flexShrink: 0,
          minHeight: 0,
          flexDirection: "column",
        }}
      >
        <View
          style={{
            display: "flex",
            flexDirection: "row",
            alignItems: "center",
            height: 32,
            flexShrink: 0,
            paddingLeft: 12,
            paddingRight: 8,
            gap: 8,
            borderBottomWidth: 1,
            borderColor: app.theme().border,
          }}
        >
          <Text
            style={{
              fontSize: 11,
              fontWeight: 700,
              letterSpacing: 0.3,
              textTransform: "uppercase",
              color: app.theme().textTertiary,
            }}
          >
            {history().allBranches ? "All branches" : (store.status()?.branch ?? "History")}
          </Text>
          <Text
            style={{ fontSize: 11, color: app.theme().textTertiary }}
          >{`${history().commits.length}${history().exhausted ? "" : "+"} commits`}</Text>
          <View style={{ flex: 1 }} />
          <CheckRow
            label="All branches"
            checked={history().allBranches}
            onChange={(value) => store.setHistoryAllBranches(value)}
          />
        </View>
        <Show
          when={history().commits.length > 0}
          fallback={
            <EmptyState
              ui={app}
              title={history().loading ? "Loading history…" : "No commits yet"}
              description={
                history().loading ? undefined : "Commits appear here once you make the first one."
              }
            />
          }
        >
          <Table.Root
            scope="history"
            rowCount={history().commits.length}
            rowHeight={HISTORY_ROW_HEIGHT}
            headerHeight={0}
            selectionMode="single"
            selection={store.historySelection()}
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
            <For each={visible()} keyed={(row) => row.index}>
              {(row) => (
                <Table.Row
                  index={row().index}
                  style={{
                    hover: { backgroundColor: app.theme().hover },
                    selected: { backgroundColor: app.theme().selection },
                  }}
                >
                  <Table.Cell column="graph">
                    <Graph row={row().graph} width={graphWidth()} height={HISTORY_ROW_HEIGHT} />
                  </Table.Cell>
                  <Table.Cell
                    column="subject"
                    style={{ minWidth: 0, paddingRight: 8, overflow: "hidden" }}
                  >
                    <View
                      onContextMenu={() => contextMenu(row().commit)}
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
                      <View
                        style={{
                          display: "flex",
                          flexDirection: "row",
                          alignItems: "center",
                          minWidth: 0,
                          maxWidth: "48%",
                          flexShrink: 1,
                          overflow: "hidden",
                          gap: 4,
                        }}
                      >
                        <For
                          each={row()
                            .commit.refs.filter((ref) => ref.kind !== "head")
                            .slice(0, 3)}
                        >
                          {(ref) => (
                            <View
                              style={{
                                display: "flex",
                                height: 16,
                                minWidth: 0,
                                alignItems: "center",
                                paddingLeft: 5,
                                paddingRight: 5,
                                borderRadius: 4,
                                backgroundColor: ref.current
                                  ? app.theme().accent
                                  : ref.kind === "tag"
                                    ? app.theme().warningWash
                                    : app.theme().accentWash,
                                maxWidth: 140,
                              }}
                            >
                              <Text
                                style={{
                                  fontSize: 10.5,
                                  fontWeight: 700,
                                  color: ref.current
                                    ? app.theme().textOnAccent
                                    : ref.kind === "tag"
                                      ? app.theme().warning
                                      : app.theme().accent,
                                  lineClamp: 1,
                                  textOverflow: "ellipsis",
                                }}
                              >
                                {ref.name}
                              </Text>
                            </View>
                          )}
                        </For>
                      </View>
                      <Text
                        style={{
                          flex: 1,
                          minWidth: 0,
                          fontSize: 12.5,
                          color: app.theme().text,
                          lineClamp: 1,
                          textOverflow: "ellipsis",
                        }}
                      >
                        {row().commit.subject}
                      </Text>
                    </View>
                  </Table.Cell>
                  <Table.Cell
                    column="author"
                    style={{ minWidth: 0, paddingRight: 8, overflow: "hidden" }}
                  >
                    <Text
                      style={{
                        fontSize: 11,
                        color: app.theme().textTertiary,
                        lineClamp: 1,
                        textOverflow: "ellipsis",
                      }}
                    >
                      {row().commit.authorName}
                    </Text>
                  </Table.Cell>
                  <Table.Cell column="date" style={{ paddingRight: 10 }}>
                    <Text
                      tooltip={absoluteTime(row().commit.authorTime)}
                      style={{
                        fontSize: 11,
                        color: app.theme().textTertiary,
                        whiteSpace: "nowrap",
                      }}
                    >
                      {relativeTime(row().commit.authorTime)}
                    </Text>
                  </Table.Cell>
                </Table.Row>
              )}
            </For>
          </Table.Root>
        </Show>
      </ResizablePanel>
      <View
        style={{ display: "flex", flex: 1, minWidth: 0, minHeight: 0, flexDirection: "column" }}
      >
        <CommitDetail />
        <DiffPane />
      </View>
    </View>
  );
}

/**
 * One row of the commit graph: one tinted SVG mask per lane color, so lines stay continuous
 * across rows and elbows are real quarter circles rather than stacked rectangles.
 */
function Graph(props: { row: GraphRow | undefined; width: number; height: number }) {
  const app = useApp();
  const palette = () => app.theme().graph;
  const layers = createMemo(() =>
    props.row ? paintGraphRow(props.row, props.width, props.height) : [],
  );
  return (
    <View style={{ position: "relative", width: "100%", height: props.height }}>
      <For each={layers()}>
        {(layer) => (
          <Svg
            source={layer.source}
            style={{
              position: "absolute",
              left: 0,
              top: 0,
              width: props.width,
              height: props.height,
              color: palette()[layer.color % palette().length]!,
            }}
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
  return (
    <View
      style={{
        display: "flex",
        flexDirection: "column",
        flexShrink: 0,
        maxHeight: "45%",
        minHeight: 0,
        overflowY: "auto",
        borderBottomWidth: 1,
        borderColor: app.theme().border,
      }}
    >
      <Show
        when={commit() !== undefined}
        fallback={
          <EmptyState ui={app} title="Select a commit" description="Its files appear here." />
        }
      >
        <View style={{ padding: 12, display: "flex", flexDirection: "column", gap: 4 }}>
          <Text style={{ fontSize: 14, fontWeight: 700 }}>{commit()?.subject}</Text>
          <Show when={!!commit()?.body}>
            <Text
              style={{
                fontSize: 12.5,
                lineHeight: 18,
                userSelect: "text",
                color: app.theme().textSecondary,
              }}
            >
              {commit()?.body}
            </Text>
          </Show>
          <Text style={{ fontSize: 12, color: app.theme().textSecondary }}>
            {commit()
              ? `${commit()!.authorName} · ${absoluteTime(commit()!.authorTime)} · ${commit()!.shortSha}`
              : ""}
          </Text>
        </View>
        <Show when={store.commitDetail().loading}>
          <Text style={{ paddingLeft: 12, fontSize: 12 }}>Loading files…</Text>
        </Show>
        <CommitFileTable />
      </Show>
    </View>
  );
}

function CommitFileTable() {
  const app = useApp();
  const store = app.store;
  const files = () => store.commitDetail().files;
  const [range, setRange] = createSignal<VisibleRange>({ start: 0, end: 0 });
  const visible = createMemo(() =>
    files()
      .slice(range().start, range().end)
      .map((file, offset) => ({ file, index: range().start + offset })),
  );
  const selection = createMemo(() => {
    const path = store.commitDetail().selectedPath ?? files()[0]?.path;
    const index = files().findIndex((file) => file.path === path);
    return index < 0 ? [] : [[index, index]];
  });
  return (
    <Table.Root
      scope="commit-files"
      aria-label="Commit files"
      rowCount={files().length}
      rowHeight={24}
      headerHeight={0}
      selectionMode="single"
      selection={selection()}
      columns={[
        { id: "status", track: "36px", align: "start" },
        { id: "name", track: "1fr", rowHeader: true },
      ]}
      onVisibleRangeChange={setRange}
      onSelectionChange={(ranges) => {
        const index = ranges[0]?.[0];
        const file = index === undefined ? undefined : files()[index];
        if (file) store.selectCommitFile(file.path);
      }}
      style={{
        height: Math.min(8, files().length) * 24,
        flexShrink: 0,
        minHeight: 0,
        overflowY: "scroll",
      }}
    >
      <For each={visible()} keyed={(row) => row.file.path}>
        {(row) => (
          <Table.Row
            index={row().index}
            style={{
              hover: { backgroundColor: app.theme().hover },
              selected: { backgroundColor: app.theme().selection },
            }}
          >
            <Table.Cell column="status" style={{ paddingLeft: 12 }}>
              <StatusGlyph code={row().file.status} />
            </Table.Cell>
            <Table.Cell column="name">
              <Text style={{ flex: 1, minWidth: 0, lineClamp: 1 }}>{row().file.path}</Text>
            </Table.Cell>
          </Table.Row>
        )}
      </For>
    </Table.Root>
  );
}
