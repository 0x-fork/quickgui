# Virtual tables and trees

[Documentation index](README.md)

QuickGUI's reusable table and tree are controlled components above the existing retained
`ListState` path. The application owns records and business state; the component owns bounded
selection, expansion, sorting declarations, focus, and scroll state. Both expose one composite Tab
stop — plus the column-resize handles an application chooses to mount — mount only the viewport plus
overscan, and create no timer, polling task, or idle scheduler source.

Run the combined gallery with:

```console
cargo run --release --example data_collections
```

## Virtual tables

Declare stable columns once and retain `TableState` in the view:

```rust
use quickgui::{
    Color, GridTrack, TableCellPosition, TableColumn, TableState, div, table_key_bindings, text,
};

struct Files {
    table: TableState,
    // Application-owned records and ordering live here.
}

impl Files {
    fn table(view: &mut Self) -> &mut TableState {
        &mut view.table
    }
}

let columns = [
    TableColumn::new("name", "Name")
        .track(GridTrack::minmax_px_fr(180.0, 2.0))
        .sortable(true)
        .row_header(true),
    TableColumn::new("kind", "Kind")
        .track(GridTrack::fr(1.0))
        .sortable(true),
];

let colors = self.collection_colors();
let table = self.table.element(
    cx,
    "files",
    &columns,
    Self::table,
    |header| {
        div()
            .bg(colors.header)
            .child(text(header.column.label().clone()))
    },
    |cell| {
        div()
            .bg(if cell.selected {
                colors.active_cell
            } else if cell.row_selected {
                colors.selected_row
            } else {
                Color::TRANSPARENT
            })
            .child(render_file_cell(cell.position.row, cell.position.column))
    },
    |view, TableCellPosition { row, column }, cx| {
        view.activate_file_cell(row, column);
        cx.invalidate();
    },
)
.border(1.0, colors.border)
.bg(colors.surface);
```

`TableLayout` retains only fixed header and row heights. The header callback receives the stable
column plus resolved sort direction; the cell callback receives logical position and exact row/cell
selection state. Both return the complete visible element. QuickGUI adds grid tracks, IDs, sorting
and selection listeners, virtual positioning, and native collection semantics without colors,
padding, dividers, typography, radii, or focus paint.

Install `table_key_bindings()` on the application. Arrow keys move the active cell, Page Up and
Page Down move by the current viewport, Command-Up and Command-Down select the first and last row,
and Return activates the selected cell. Clicking a cell selects it and returns focus to the table's
composite focus target. Mounted column-resize handles are the only additional Tab stops, and they
exist only when the application chooses to place them.

### Row selection

`TableSelectionMode::Multiple` turns on Shift ranges, platform-modified toggles, and Select All;
the default `Single` keeps the selection on the active row. `TableSelection` retains merged
inclusive ranges rather than one entry per row, so Select All over a million rows costs one range.

```rust
let mut table = TableState::new(rows).with_selection_mode(TableSelectionMode::Multiple);
table.select_row(4);          // plain click or arrow move
table.select_row_range(9);    // Shift extends from the anchor
table.toggle_row_selection(6); // Command or Control toggles one row
assert_eq!(table.selection().ranges(), [(4, 5), (7, 9)]);
```

An unmodified press selects one row and anchors future ranges there; Shift replaces the selection
with the anchor-to-row range; Command or Control toggles exactly one row. From the keyboard, Space
toggles the active row, Shift-Up and Shift-Down extend from the anchor, and Command-A selects
every row. Rows project `selected` state and a multiple-selection grid projects
`multiselectable`, so assistive technology announces the difference.

After any interaction that actually changed the selection, the table dispatches
`TableSelectionChanged` through the focused path; bind an ordinary action listener on the table root
and read `TableState::selection()`. A gesture that changes nothing dispatches nothing and
invalidates nothing. `TableState::selection_version()` is the same signal as a counter.

### Column widths and order

`TableColumn::width(px)` makes one column resizable and lays it out as a fixed pixel track: without
a retained width the resize behavior would not exist. `minimum_width(px)` refuses to shrink past a
declared floor. The header renderer receives `TableHeaderState::resize_handle`, a
behavior-decorated, appearance-free element carrying the captured pointer drag, the keyboard resize
actions, the platform column-resize cursor, and splitter semantics with the column's live width as
its numeric value. The application decides where the handle sits, how wide its hit area is, and how
it is painted — exactly like the tree's disclosure element.

A retained width survives every rebuild, including a declaration that adds or removes columns, so a
frame never discards a user's drag. `TableState::set_column_width` and `resize_column` apply the
same clamping programmatically.

Alt-Left and Alt-Right move the active column through the display order.
`TableState::column_order()` reports the declared indices in display order and
`move_column(index, delta)` applies a move programmatically. `TableCellPosition::column` always
indexes the caller's declared column slice, never the display order, so an application reading a
position never has to undo a user's reordering; `TableCellState::display_column` and
`TableHeaderState::display_index` carry the visual position, and that is what the accessibility
projection uses. QuickGUI deliberately does not implement pointer-drag reordering: it would need
header geometry the framework does not retain, and an application that wants it can drive
`move_column` from its own drag.

### Inline editing

`begin_edit(position, column_count)` opens an editor over one cell and makes it active. The cell
renderer receives `TableCellState::editing` and mounts the application's own editor; QuickGUI gives
that cell its own key context and nothing else, so the editor's value, validation, and appearance
stay application-owned. Focus the editor when the edit begins — `auto_focus()` on a freshly mounted
input, or an explicit focus request — because the key context follows focus.

Return commits and Escape abandons. Both close the editor, return focus to the table, and dispatch
`TableEditEnded { position, committed }` through the focused path. Those bindings live in the
editor's deeper key context, so Return and Escape keep their ordinary meaning everywhere else in the
table.

Sortable headers update `TableState::sort()`. QuickGUI deliberately does not retain or copy row
records: compare that declaration with the sort already applied by the view, reorder the
application's stable row-index array when it changes, then render through the new ordering. This
keeps filtering, secondary keys, locale rules, and asynchronous data ownership outside the
renderer. `set_row_count` clamps selection and retained scrolling when the data size changes.

The accessibility root is a grid with exact logical row and column counts. Headers, rows, row
headers, and cells expose their native collection roles and zero-based logical indices. The active
cell is projected as the grid's active descendant, and sortable headers expose ascending or
descending state.

## Virtual trees

`TreeNode<T>` carries a stable ID, accessible label, application value, children, and disabled
state. Construction validates and flattens the hierarchy once:

```rust
use quickgui::{TreeNode, TreeState, tree_key_bindings};

let nodes = [
    TreeNode::new("src", "src", Folder::Source)
        .child(TreeNode::new("lib", "lib.rs", Folder::File))
        .child(
            TreeNode::new("platform", "platform", Folder::Platform)
                .child(TreeNode::new("macos", "macos.rs", Folder::File)),
        ),
    TreeNode::new("target", "target", Folder::Generated).disabled(true),
];

let mut tree = TreeState::new(nodes)?;
tree.set_expanded("src", true);

// Later, inside View::render:
let tree = self.tree.element(
    cx,
    "project-tree",
    |view| &mut view.tree,
    |row, disclosure| {
        let disclosure = disclosure.map_or_else(
            || div().w(20.0),
            |disclosure| {
                disclosure
                    .w(20.0)
                    .child(if row.is_expanded() { "⌄" } else { "›" })
            },
        );
        div()
            .pl(6.0 + row.level() as f32 * 16.0)
            .bg(if row.is_selected() { colors.selected } else { Color::TRANSPARENT })
            .child(disclosure)
            .child(text(row.label().clone()))
    },
    |view, node_id, cx| view.activate_node(node_id, cx),
)
.border(1.0, colors.border)
.bg(colors.surface);
```

`TreeLayout` retains only the virtual row height. For branch rows, QuickGUI passes an optional
unstyled disclosure element carrying the exact expand/collapse listener and accessible label; the
caller places and paints it alongside application content. Indentation, icons, disclosure hit-box
size, selected/hover/disabled appearance, typography, and root presentation remain application
owned.

Install `tree_key_bindings()` on the application. Up and Down skip disabled items, Right expands a
branch or enters its first enabled child, Left collapses it or selects its parent, Space toggles a
branch, Page Up/Page Down and Command-Up/Command-Down navigate by viewport or edge, and Return
activates the selected stable node ID.

### Lazy children

`TreeNode::pending(true)` declares a branch whose children have not been loaded. It expands like any
other branch; the first expansion mounts exactly one placeholder row and dispatches
`TreeLoadChildren { node }` through the focused path. The tree owns no loader, task, or timer: it
asks once and waits.

```rust
let load = cx.action_listener("project-tree", |view: &mut Files, action: &TreeLoadChildren, cx| {
    view.tree.set_children(action.node, view.fetch(action.node))?;
    cx.invalidate();
});
```

The placeholder is reported through `TreeRow::is_loading()`, carries the tree's bounded loading
label, sits one level under its branch, and is never selectable by pointer or keyboard.
`TreeState::with_loading_label` replaces the default accessible name.

`set_children` validates the payload — depth, node budget, label bytes, duplicate IDs across the
whole tree — before any live state changes, then splices it in one step. A rejected payload leaves
the tree exactly as it was; a successful one clears the pending flag, removes the placeholder, and
preserves selection, expansion, and the logical scroll anchor. Loading an empty list turns the
branch into an ordinary leaf. A branch that already has children refuses a second splice.

`TreeState::set_nodes` validates a replacement before changing live state. On success it preserves
selection, expansion, and the logical top scroll anchor by stable ID when those nodes still exist;
on validation failure the old tree is unchanged. Expansion rebuilds only compact visible-index
vectors. Ordinary rendering visits mounted rows rather than walking the hierarchy.

The accessibility root is a tree with the logical root count and one active descendant. Mounted
tree items expose exact zero-based level, position-in-set, set size, selected, disabled, and
expanded state.

## Bounds and resource ownership

- One table accepts at most `MAX_TABLE_ROWS` (1,000,000) logical rows and
  `MAX_TABLE_COLUMNS` grid columns. It retains no row records.
- One selection retains at most `MAX_TABLE_SELECTION_RANGES` (1,024) disjoint ranges. A toggle that
  would fragment it further is refused and leaves the selection unchanged.
- A resizable column stays between its declared minimum, at least `MIN_TABLE_COLUMN_WIDTH` (24 px),
  and `MAX_TABLE_COLUMN_WIDTH` (4,096 px); one keyboard step is `TABLE_COLUMN_RESIZE_STEP` (8 px).
- A table retains one display-order entry and one width entry per column, one optional edited cell,
  and one selection-version counter. It creates no timer or idle source for any of them.
- One tree accepts at most `MAX_TREE_NODES` (1,000,000), depth `MAX_TREE_DEPTH` (256),
  `MAX_TREE_LABEL_BYTES` (64 KiB) per label, and `MAX_TREE_TEXT_BYTES` (16 MiB) in total.
- Tree identity lookup is a compact sorted stable-ID index. Expansion state is a bit vector; visible
  and reverse-visible positions use 32-bit indices. A loading placeholder is encoded in the free top
  bit of one visible index, so lazy loading adds no vector and at most one mounted row per branch.
- Header, cell, and tree-row render callbacks run only for mounted content. Scrolling reuses `ListState`, its
  captured native-style scrollbar, and its existing mount and metric ceilings.
- `TableLayout` and `TreeLayout` retain only finite bounded virtualization geometry. All visible
  presentation is declared by the application, and settled collections render zero extra idle
  frames.

The framework tests a 100,000-node flat tree in a three-row viewport and requires no more than
seven mounted rows, including overscan, with no measured-height entries created before layout.
Sorting and full source replacement are intentionally application events rather than per-frame
work.

## JavaScript bindings

`Table.Root` / `Header` / `Row` / `Cell` and `Tree.Root` / `Row` declare the column list, the row
count, the node source, the controlled selection, sort, expansion, and inline-edit position. The
hosted view reaches each declared instance's retained `TableState` or `TreeState` through a
per-instance [`StateAccessor`](view-api.md), so several collections in one window stay independent.

Both are on-demand. The core owns the virtual window and reports the range it mounted through
`onVisibleRangeChange`; JavaScript declares exactly those `Row` children, so a million-row table
declares only the window on screen. Column resizing and reordering, keyboard navigation, selection
policy, expansion, the lazy-children request, and Return activation stay in the core, and every
result travels back as one asynchronous payload keyed by the caller's own declared identifiers
rather than by a positional index a source replacement could invalidate.

A declared header, row, or cell carries content only: an element can hold exactly one stable id and
the core assigns the grid, tree-item, and active-descendant identities itself, so these nodes mount
without one and register no listener of their own. An interactive control belongs inside a cell as
an ordinary child node. Supplying a pending branch's children is a declaration too, spliced
atomically by the core — an over-deep, oversized, or duplicate payload leaves the tree exactly as it
was. Declarations are bounded before they reach the core: 2 MiB per column, node, selection, or
toast source, 512 columns, 65,536 nodes, and a duplicate identifier keeps its first occurrence. See
the [Solid 2 renderer](solid.md#virtual-tables-and-trees).
