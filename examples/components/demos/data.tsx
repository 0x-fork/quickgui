import type { NativeNode } from "@quickgui/native";
import { For, Text, createSignal } from "@quickgui/ui";
import { ScrollArea, useScrollAreaState, type ScrollAreaState } from "@quickgui/ui/base-ui";
import {
  Table,
  Tree,
  type TableColumnDeclaration,
  type TableRowRange,
  type TreeChildrenSplice,
  type TreeNodeDeclaration,
} from "@quickgui/ui/collections";
import type { ColumnWidth, TableSortState, VisibleRange } from "@quickgui/ui";

import { p } from "../theme.ts";
import { Note, Panel } from "../ui.tsx";

/* -------------------------------------------------------------------------------------------- *
 * Scroll area
 * -------------------------------------------------------------------------------------------- */

/**
 * The thumb, styled from the state the core reports for it.
 *
 * The native side owns the thumb's position and length, both computed from the measured
 * geometry, so the application declares only its width and its paint.
 */
function ScrollAreaThumbBody(): NativeNode {
  const state = useScrollAreaState();
  return <ScrollArea.Thumb orientation="vertical" style={{ width: 8, borderRadius: 4, backgroundColor: state().scrolling ? p().accent : p().border }} />;
}

const LOG_ROW_HEIGHT = 22;
const LOG_ROWS = 40;
const LOG_VIEWPORT_WIDTH = 320;
const LOG_VIEWPORT_HEIGHT = 160;

function logLines(): string[] {
  const rows: string[] = [];
  for (let index = 0; index < LOG_ROWS; index += 1) rows.push("log line " + String(index + 1));
  return rows;
}

export function ScrollAreaDemo(): NativeNode {
  const rows = logLines();
  const [state, setState] = createSignal<ScrollAreaState | undefined>(undefined);
  const offsetY = (): number => {
    const current = state();
    return current === undefined ? 0 : current.offset.y;
  };
  const flag = (read: (current: ScrollAreaState) => boolean): string => {
    const current = state();
    return current === undefined ? "false" : String(read(current));
  };
  return (
    <Panel
      title="Scroll area"
      hint="A declared model: the extents are declared here (the core also measures them from painted bounds), the core clamps the offset, derives every overflow flag, and positions and sizes the thumb itself. The offset is applied to the content as a paint-only transform."
    >
      <ScrollArea.Root
        viewportSize={() => ({ width: LOG_VIEWPORT_WIDTH, height: LOG_VIEWPORT_HEIGHT })}
        contentSize={() => ({ width: LOG_VIEWPORT_WIDTH, height: LOG_ROWS * LOG_ROW_HEIGHT })}
        overflowEdgeThreshold={2}
        onScrollStateChange={(next) => setState(next)}
        style={{ display: "flex", flexDirection: "row", gap: 4, height: LOG_VIEWPORT_HEIGHT }}
      >
        <ScrollArea.Viewport
          style={{
            width: LOG_VIEWPORT_WIDTH,
            height: LOG_VIEWPORT_HEIGHT,
            backgroundColor: p().panelAlt,
            borderRadius: 10,
            borderWidth: 1,
            borderColor: p().border,
            overflow: "hidden",
          }}
        >
          <ScrollArea.Content style={{ display: "flex", flexDirection: "column", paddingLeft: 10, transform: "translateY(" + String(-offsetY()) + "px)" }}>
            <For each={() => rows}>
              {(row) => <Text style={{ fontSize: 12, color: p().muted, height: LOG_ROW_HEIGHT, lineHeight: LOG_ROW_HEIGHT }}>{row}</Text>}
            </For>
          </ScrollArea.Content>
        </ScrollArea.Viewport>
        <ScrollArea.Scrollbar orientation="vertical" style={{ width: 8, height: LOG_VIEWPORT_HEIGHT, backgroundColor: p().track, borderRadius: 4 }}>
          <ScrollAreaThumbBody />
        </ScrollArea.Scrollbar>
      </ScrollArea.Root>
      <Note
        text={
          "offset " + String(Math.round(offsetY())) + " · scrolling " + flag((current) => current.scrolling) + " · hasOverflowY " + flag((current) => current.hasOverflowY) + " · overflowYStart " + flag((current) => current.overflowYStart) + " · overflowYEnd " + flag((current) => current.overflowYEnd)
        }
      />
    </Panel>
  );
}

/* -------------------------------------------------------------------------------------------- *
 * Table
 * -------------------------------------------------------------------------------------------- */

interface Asset {
  name: string;
  size: number;
}

const ASSET_COUNT = 5000;

function assets(): Asset[] {
  const list: Asset[] = [];
  for (let index = 0; index < ASSET_COUNT; index += 1) {
    list.push({ name: "asset-" + String(index).padStart(4, "0") + ".png", size: (index % 900) + 12 });
  }
  return list;
}

const assetColumns: TableColumnDeclaration[] = [
  { id: "name", label: "Name", width: 240, minWidth: 120, sortable: true, rowHeader: true },
  { id: "size", label: "Size", track: "1fr", align: "end", sortable: true },
];

function sortMarker(sort: TableSortState | undefined, column: string): string {
  if (sort === undefined || sort.column !== column) return "";
  return sort.direction === "ascending" ? " ▲" : " ▼";
}

interface VisibleAsset {
  row: number;
  asset: Asset;
}

export function TableDemo(): NativeNode {
  const base = assets();
  const [sort, setSort] = createSignal<TableSortState | undefined>(undefined);
  const [range, setRange] = createSignal<VisibleRange>({ start: 0, end: 0 });
  const [selection, setSelection] = createSignal<TableRowRange[]>([]);
  const [activated, setActivated] = createSignal("nothing yet");
  const [widths, setWidths] = createSignal("declared");

  const ordered = (): Asset[] => {
    const order = sort();
    if (order === undefined) return base;
    const sorted = [...base];
    sorted.sort((left, right) => {
      if (order.column === "size") return left.size - right.size;
      return left.name < right.name ? -1 : left.name > right.name ? 1 : 0;
    });
    if (order.direction === "descending") sorted.reverse();
    return sorted;
  };
  const visible = (): VisibleAsset[] => {
    const list = ordered();
    const window = range();
    const rows: VisibleAsset[] = [];
    for (let row = window.start; row < window.end && row < list.length; row += 1) rows.push({ row, asset: list[row]! });
    return rows;
  };
  const selected = (): number => {
    let total = 0;
    for (const span of selection()) total += (span[1] ?? 0) - (span[0] ?? 0) + 1;
    return total;
  };
  const describeWidths = (next: ColumnWidth[]): string => {
    const parts: string[] = [];
    for (const entry of next) parts.push(entry.id + " " + String(Math.round(entry.width)));
    return parts.join(", ");
  };

  return (
    <Panel
      title="Table"
      hint="Five thousand rows, of which only the range the core last reported as visible is declared. Selection, sort, and column widths are all reported asynchronously."
    >
      <Table.Root
        rowCount={() => base.length}
        rowHeight={26}
        headerHeight={28}
        selectionMode="multiple"
        selection={selection}
        sort={sort}
        columns={() => assetColumns}
        onVisibleRangeChange={(next) => setRange(next)}
        onSelectionChange={(next) => setSelection(next)}
        onSortChange={(next) => setSort(next)}
        onColumnResize={(next) => setWidths(describeWidths(next))}
        onActivate={(cell) => setActivated("row " + String(cell.row) + ", column " + String(cell.column))}
        style={{ height: 260, borderRadius: 10, borderWidth: 1, borderColor: p().border, backgroundColor: p().panelAlt, overflowY: "scroll" }}
      >
        <Table.Header column="name" style={{ paddingLeft: 10, justifyContent: "center" }}>
          <Text style={{ fontSize: 11, fontWeight: 700, color: p().faint }}>{"NAME" + sortMarker(sort(), "name")}</Text>
        </Table.Header>
        <Table.Header column="size" style={{ paddingRight: 10, justifyContent: "center" }}>
          <Text style={{ fontSize: 11, fontWeight: 700, color: p().faint }}>{"SIZE" + sortMarker(sort(), "size")}</Text>
        </Table.Header>
        <For each={visible} key={(entry) => entry.row}>
          {(entry) => (
            <Table.Row index={entry.row}>
              <Table.Cell column="name" style={{ paddingLeft: 10 }}>
                <Text style={{ fontSize: 12, color: p().ink }}>{entry.asset.name}</Text>
              </Table.Cell>
              <Table.Cell column="size" style={{ paddingRight: 10, justifyContent: "flex-end" }}>
                <Text style={{ fontSize: 12, color: p().muted, textAlign: "end" }}>{String(entry.asset.size) + " KB"}</Text>
              </Table.Cell>
            </Table.Row>
          )}
        </For>
      </Table.Root>
      <Note
        text={
          "rows " + String(range().start) + "–" + String(range().end) + " of " + String(base.length) + " · " + String(selected()) + " selected · sort " + (sort() === undefined ? "none" : (sort()?.column ?? "") + " " + (sort()?.direction ?? "")) + " · widths " + widths() + " · activated " + activated()
        }
      />
    </Panel>
  );
}

/* -------------------------------------------------------------------------------------------- *
 * Tree
 * -------------------------------------------------------------------------------------------- */

const srcChildren: TreeNodeDeclaration[] = [
  { id: "src/lib.rs", label: "lib.rs" },
  { id: "src/main.rs", label: "main.rs" },
  { id: "src/element", label: "element", pending: true },
];
const docsChildren: TreeNodeDeclaration[] = [
  { id: "docs/native.md", label: "native.md" },
  { id: "docs/tabs.md", label: "tabs.md" },
];
const treeNodes: TreeNodeDeclaration[] = [
  { id: "src", label: "src", children: srcChildren },
  { id: "docs", label: "docs", children: docsChildren },
];

/** The rows the core can mount, in tree order, each with the depth that indents it. */
interface FlatRow {
  node: TreeNodeDeclaration;
  depth: number;
}

function flatten(list: TreeNodeDeclaration[], depth: number, expanded: string[], splice: TreeChildrenSplice | undefined, into: FlatRow[]): void {
  for (const node of list) {
    into.push({ node, depth });
    if (!expanded.includes(node.id)) continue;
    const children = splice !== undefined && splice.id === node.id ? splice.children : (node.children ?? []);
    flatten(children, depth + 1, expanded, splice, into);
  }
}

export function TreeDemo(): NativeNode {
  const [expanded, setExpanded] = createSignal<string[]>(["src"]);
  const [selected, setSelected] = createSignal("—");
  const [activated, setActivated] = createSignal("nothing yet");
  const [range, setRange] = createSignal<VisibleRange>({ start: 0, end: 0 });
  const [loaded, setLoaded] = createSignal<TreeChildrenSplice | undefined>(undefined);
  const [asked, setAsked] = createSignal(0);

  const visible = (): FlatRow[] => {
    const rows: FlatRow[] = [];
    flatten(treeNodes, 0, expanded(), loaded(), rows);
    const window = range();
    return rows.slice(window.start, window.end);
  };

  return (
    <Panel
      title="Tree"
      hint="A lazy tree. The one pending branch asks for its children exactly once through onLoadChildren, answered with a single validated setChildren splice."
    >
      <Tree.Root
        nodes={() => treeNodes}
        rowHeight={26}
        loadingLabel="Loading…"
        disclosure="leading"
        expanded={expanded}
        setChildren={loaded}
        onExpandedChange={(next) => setExpanded(next)}
        onValueChange={(next) => setSelected(next ?? "—")}
        onVisibleRangeChange={(next) => setRange(next)}
        onActivate={(id) => setActivated(id)}
        onLoadChildren={(id) => {
          setAsked(asked() + 1);
          const children: TreeNodeDeclaration[] = [
            { id: id + "/style.rs", label: "style.rs" },
            { id: id + "/state.rs", label: "state.rs" },
          ];
          setLoaded({ id, children });
        }}
        style={{ height: 220, borderRadius: 10, borderWidth: 1, borderColor: p().border, backgroundColor: p().panelAlt, overflowY: "scroll" }}
      >
        <For each={visible} key={(row) => row.node.id}>
          {(row) => (
            <Tree.Row nodeId={row.node.id} style={{ display: "flex", flexDirection: "row", alignItems: "center", height: 26, paddingLeft: 10 + row.depth * 16 }}>
              <Text style={{ fontSize: 12, color: p().ink }}>{row.node.label ?? row.node.id}</Text>
            </Tree.Row>
          )}
        </For>
      </Tree.Root>
      <Note text={"expanded [" + expanded().join(", ") + "] · selected " + selected() + " · activated " + activated() + " · onLoadChildren asked " + String(asked()) + "×"} />
    </Panel>
  );
}
