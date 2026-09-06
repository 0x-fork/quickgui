/**
 * Virtual collections: table and tree.
 *
 * The rows the application declares are the rows the Rust core last reported as visible, so a
 * million-row table declares only the window on screen. Selection, sort, widths, display order,
 * expansion, lazy children, and the inline-edit lifetime are all decided by the core and
 * reported asynchronously.
 */

import { NativePart, type NativeNode, type QuickGuiEvent } from "@quickgui/native";
import { EVENT_COMMIT, EVENT_COMPONENT_CHANGE } from "@quickgui/native/native-tree";

import {
  CODE_COLUMNS,
  CODE_COLUMN_INDEX,
  CODE_DISCLOSURE,
  CODE_EDITING,
  CODE_EXPANDED,
  CODE_HEADER_HEIGHT,
  CODE_LOADING_LABEL,
  CODE_NODES,
  CODE_ROW_COUNT,
  CODE_ROW_HEIGHT,
  CODE_ROW_INDEX,
  CODE_SELECTED_VALUE,
  CODE_SELECTION,
  CODE_SELECTION_MODE,
  CODE_SET_CHILDREN,
  CODE_SORT_COLUMN,
  CODE_SORT_DIRECTION,
} from "./generated.ts";
import {
  commitFromEvent,
  componentChangeFromEvent,
  type ColumnWidth,
  type TableCell,
  type TableEditEndDetails,
  type TableSortState,
  type VisibleRange,
} from "./index.ts";
import { createComponentScope, createViewPart, finishPart, setPart, type PartProps } from "./parts.ts";
import { createRenderEffect, signal, type Accessor } from "./reactive.ts";
import { setComponentValue, setJson, setListener, setNumber, setString } from "./runtime.ts";

// --- table ---------------------------------------------------------------------------------------

/** One declared table column. */
export interface TableColumnDeclaration {
  id: string;
  label?: string;
  /** Fixed starting width in logical pixels. A column with a width is user-resizable. */
  width?: number;
  minWidth?: number;
  align?: "start" | "center" | "end";
  sortable?: boolean;
  /** Project this column's cells as the row's accessible name. */
  rowHeader?: boolean;
  /** CSS grid track for a column that is not user-resizable, such as `1fr` or `auto`. */
  track?: string;
}

/** One inclusive `[start, end]` row range. */
export type TableRowRange = number[];

export interface TableRootProps extends PartProps {
  columns?: Accessor<TableColumnDeclaration[]>;
  /** Total logical row count, up to the core's own million-row bound. */
  rowCount?: Accessor<number>;
  rowHeight?: number;
  headerHeight?: number;
  selectionMode?: "single" | "multiple";
  /** Controlled selection as inclusive `[start, end]` row ranges. */
  selection?: Accessor<TableRowRange[]>;
  sort?: Accessor<TableSortState | undefined>;
  /** Controlled inline-edit position. Declaring it opens the editor over that cell. */
  editing?: Accessor<TableCell | undefined>;
  /** The range of rows the core is virtualizing. Declare exactly these `Row` children. */
  onVisibleRangeChange?: (range: VisibleRange, event: QuickGuiEvent) => void;
  onSelectionChange?: (ranges: TableRowRange[], event: QuickGuiEvent) => void;
  onSortChange?: (sort: TableSortState | undefined, event: QuickGuiEvent) => void;
  onActiveCellChange?: (cell: TableCell | undefined, event: QuickGuiEvent) => void;
  onColumnResize?: (widths: ColumnWidth[], event: QuickGuiEvent) => void;
  onColumnReorder?: (order: string[], event: QuickGuiEvent) => void;
  onEditEnd?: (details: TableEditEndDetails, event: QuickGuiEvent) => void;
  onActivate?: (cell: TableCell, event: QuickGuiEvent) => void;
}

export interface TableHeaderProps extends PartProps {
  /** Declared column identifier this header paints. */
  column: string;
}

export interface TableRowProps extends PartProps {
  /** Logical row index, inside the range the core reported as visible. */
  index: number;
}

export interface TableCellProps extends PartProps {
  /** Declared column identifier, or use `index` for a positional cell. */
  column?: string;
  index?: number;
}

/** Compound parts for a virtual table. */
export class Table {
  /** Virtual table root. */
  static Root(props: TableRootProps): NativeNode {
    const node = createViewPart(props);
    // The core's table element takes the root identity, so the root declares a scope of its own
    // rather than sharing the host wrapper's node identity.
    setPart(node, NativePart.Table, createComponentScope("qg-table"), undefined);
    const columns = props.columns;
    if (columns !== undefined) {
      createRenderEffect(() => {
        setJson(node, CODE_COLUMNS, 65536, columns());
      });
    }
    const rowCount = props.rowCount;
    if (rowCount !== undefined) {
      createRenderEffect(() => {
        setNumber(node, CODE_ROW_COUNT, rowCount());
      });
    }
    if (props.rowHeight !== undefined) setNumber(node, CODE_ROW_HEIGHT, props.rowHeight);
    if (props.headerHeight !== undefined) setNumber(node, CODE_HEADER_HEIGHT, props.headerHeight);
    if (props.selectionMode !== undefined) setString(node, CODE_SELECTION_MODE, props.selectionMode);
    const selection = props.selection;
    if (selection !== undefined) {
      createRenderEffect(() => {
        setJson(node, CODE_SELECTION, 65536, selection());
      });
    }
    const sort = props.sort;
    if (sort !== undefined) {
      createRenderEffect(() => {
        const current = sort();
        setString(node, CODE_SORT_COLUMN, current === undefined ? undefined : current.column);
        setString(node, CODE_SORT_DIRECTION, current === undefined ? undefined : current.direction);
      });
    }
    const editing = props.editing;
    if (editing !== undefined) {
      createRenderEffect(() => {
        setJson(node, CODE_EDITING, 65536, editing());
      });
    }
    setListener(node, EVENT_COMPONENT_CHANGE, (event: QuickGuiEvent): void => {
      const details = componentChangeFromEvent(event);
      if (details === undefined) return;
      const visibleRange = details.visibleRange;
      if (visibleRange !== undefined) {
        const onVisibleRangeChange = props.onVisibleRangeChange;
        if (onVisibleRangeChange !== undefined) onVisibleRangeChange(visibleRange, event);
      }
      const selectedRanges = details.selectedRanges;
      if (selectedRanges !== undefined) {
        const onSelectionChange = props.onSelectionChange;
        if (onSelectionChange !== undefined) onSelectionChange(selectedRanges, event);
      }
      const nextSort = details.sort;
      if (nextSort !== undefined) {
        const onSortChange = props.onSortChange;
        if (onSortChange !== undefined) onSortChange(nextSort === null ? undefined : nextSort, event);
      }
      const activeCell = details.activeCell;
      if (activeCell !== undefined) {
        const onActiveCellChange = props.onActiveCellChange;
        if (onActiveCellChange !== undefined) onActiveCellChange(activeCell === null ? undefined : activeCell, event);
      }
      const columnWidths = details.columnWidths;
      if (columnWidths !== undefined) {
        const onColumnResize = props.onColumnResize;
        if (onColumnResize !== undefined) onColumnResize(columnWidths, event);
      }
      const columnOrder = details.columnOrder;
      if (columnOrder !== undefined) {
        const onColumnReorder = props.onColumnReorder;
        if (onColumnReorder !== undefined) onColumnReorder(columnOrder, event);
      }
      const editEnded = details.editEnded;
      if (editEnded !== undefined) {
        const onEditEnd = props.onEditEnd;
        if (onEditEnd !== undefined) onEditEnd(editEnded, event);
      }
    });
    const onActivate = props.onActivate;
    if (onActivate !== undefined) {
      setListener(node, EVENT_COMMIT, (event: QuickGuiEvent): void => {
        const details = commitFromEvent(event);
        if (details === undefined) return;
        const row = details.row;
        const column = details.column;
        if (row !== undefined && column !== undefined) onActivate({ row, column }, event);
      });
    }
    return finishPart(node, props);
  }

  /**
   * One declared column header.
   *
   * The core assigns the header's grid identity and appends its own resize handle, so this node
   * declares content and paint only.
   */
  static Header(props: TableHeaderProps): NativeNode {
    const node = createViewPart(props);
    setPart(node, NativePart.TableHeader, undefined, props.column);
    return finishPart(node, props);
  }

  /** One declared row inside the range the core reported as visible. */
  static Row(props: TableRowProps): NativeNode {
    const node = createViewPart(props);
    setPart(node, NativePart.TableRow, undefined, undefined);
    setNumber(node, CODE_ROW_INDEX, props.index);
    return finishPart(node, props);
  }

  /** One declared cell. The core owns its identity, selection state, and editor key context. */
  static Cell(props: TableCellProps): NativeNode {
    const node = createViewPart(props);
    setPart(node, NativePart.TableCell, undefined, props.column);
    if (props.index !== undefined) setNumber(node, CODE_COLUMN_INDEX, props.index);
    return finishPart(node, props);
  }
}

// --- tree ----------------------------------------------------------------------------------------

/** One declared tree node, which may declare its own children or be lazily pending. */
export interface TreeNodeDeclaration {
  id: string;
  label?: string;
  disabled?: boolean;
  /** A branch whose children are fetched the first time it is expanded. */
  pending?: boolean;
  children?: TreeNodeDeclaration[];
}

/** One bounded, atomically validated lazy-children splice. */
export interface TreeChildrenSplice {
  id: string;
  children: TreeNodeDeclaration[];
}

export interface TreeRootProps extends PartProps {
  nodes?: Accessor<TreeNodeDeclaration[]>;
  /** Controlled expanded node identifiers. */
  expanded?: Accessor<string[]>;
  defaultExpanded?: string[];
  /** Controlled selected node identifier. */
  value?: Accessor<string | undefined>;
  rowHeight?: number;
  /** Row text the core paints while a pending branch is loading. */
  loadingLabel?: string;
  /** Where the core's behavior-only disclosure control is mounted inside a declared row. */
  disclosure?: "leading" | "trailing" | "none";
  /** One bounded, atomically validated lazy-children splice. */
  setChildren?: Accessor<TreeChildrenSplice | undefined>;
  onExpandedChange?: (expanded: string[], event: QuickGuiEvent) => void;
  onValueChange?: (value: string | undefined, event: QuickGuiEvent) => void;
  /** A pending branch asks for its children exactly once, when it is first expanded. */
  onLoadChildren?: (id: string, event: QuickGuiEvent) => void;
  onVisibleRangeChange?: (range: VisibleRange, event: QuickGuiEvent) => void;
  onActivate?: (id: string, event: QuickGuiEvent) => void;
}

export interface TreeRowProps extends PartProps {
  /** Declared node identifier this row paints. */
  nodeId: string;
}

/** Compound parts for a virtual tree. */
export class Tree {
  /**
   * Virtual tree root.
   *
   * Expansion and selection are controlled declarations; the core owns arrow navigation, the
   * virtual window, and the lazy-children request a pending branch makes exactly once.
   */
  static Root(props: TreeRootProps): NativeNode {
    const uncontrolled = signal<string[]>(props.defaultExpanded ?? []);
    const expanded = (): string[] => {
      const controlled = props.expanded;
      return controlled !== undefined ? controlled() : uncontrolled.read();
    };
    const node = createViewPart(props);
    setPart(node, NativePart.Tree, createComponentScope("qg-tree"), undefined);
    const nodes = props.nodes;
    if (nodes !== undefined) {
      createRenderEffect(() => {
        setJson(node, CODE_NODES, 1048576, nodes());
      });
    }
    createRenderEffect(() => {
      setJson(node, CODE_EXPANDED, 65536, expanded());
    });
    const value = props.value;
    if (value !== undefined) {
      createRenderEffect(() => {
        setComponentValue(node, CODE_SELECTED_VALUE, value());
      });
    }
    if (props.rowHeight !== undefined) setNumber(node, CODE_ROW_HEIGHT, props.rowHeight);
    if (props.loadingLabel !== undefined) setString(node, CODE_LOADING_LABEL, props.loadingLabel);
    if (props.disclosure !== undefined) setString(node, CODE_DISCLOSURE, props.disclosure);
    const setChildren = props.setChildren;
    if (setChildren !== undefined) {
      createRenderEffect(() => {
        setJson(node, CODE_SET_CHILDREN, 1048576, setChildren());
      });
    }
    setListener(node, EVENT_COMPONENT_CHANGE, (event: QuickGuiEvent): void => {
      const details = componentChangeFromEvent(event);
      if (details === undefined) return;
      const visibleRange = details.visibleRange;
      if (visibleRange !== undefined) {
        const onVisibleRangeChange = props.onVisibleRangeChange;
        if (onVisibleRangeChange !== undefined) onVisibleRangeChange(visibleRange, event);
      }
      const nextExpanded = details.expanded;
      if (nextExpanded !== undefined) {
        if (props.expanded === undefined) uncontrolled.write(nextExpanded);
        const onExpandedChange = props.onExpandedChange;
        if (onExpandedChange !== undefined) onExpandedChange(nextExpanded, event);
      }
      if (details.value !== undefined) {
        const onValueChange = props.onValueChange;
        if (onValueChange !== undefined) onValueChange(details.value === null ? undefined : details.value, event);
      }
      const loadChildren = details.loadChildren;
      if (loadChildren !== undefined) {
        const onLoadChildren = props.onLoadChildren;
        if (onLoadChildren !== undefined) onLoadChildren(loadChildren, event);
      }
    });
    const onActivate = props.onActivate;
    if (onActivate !== undefined) {
      setListener(node, EVENT_COMMIT, (event: QuickGuiEvent): void => {
        const details = commitFromEvent(event);
        if (details === undefined) return;
        const id = details.value;
        if (id !== undefined) onActivate(id, event);
      });
    }
    return finishPart(node, props);
  }

  /**
   * One declared tree row inside the range the core reported as visible.
   *
   * The core hands the binding a behavior-only disclosure control for a branch; `disclosure` on
   * the root decides whether it is mounted before or after this content.
   */
  static Row(props: TreeRowProps): NativeNode {
    const node = createViewPart(props);
    setPart(node, NativePart.TreeRow, undefined, props.nodeId);
    return finishPart(node, props);
  }
}
