# Virtual tables and trees

[Documentation index](README.md)

QuickGUI's reusable table and tree are controlled components above the existing retained
`ListState` path. The application owns records and business state; the component owns bounded
selection, expansion, sorting declarations, focus, and scroll state. Both expose one composite Tab
stop, mount only the viewport plus overscan, and create no timer, polling task, or idle scheduler
source.

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
single composite focus target.

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
- One tree accepts at most `MAX_TREE_NODES` (1,000,000), depth `MAX_TREE_DEPTH` (256),
  `MAX_TREE_LABEL_BYTES` (64 KiB) per label, and `MAX_TREE_TEXT_BYTES` (16 MiB) in total.
- Tree identity lookup is a compact sorted stable-ID index. Expansion state is a bit vector; visible
  and reverse-visible positions use 32-bit indices.
- Header, cell, and tree-row render callbacks run only for mounted content. Scrolling reuses `ListState`, its
  captured native-style scrollbar, and its existing mount and metric ceilings.
- `TableLayout` and `TreeLayout` retain only finite bounded virtualization geometry. All visible
  presentation is declared by the application, and settled collections render zero extra idle
  frames.

The framework tests a 100,000-node flat tree in a three-row viewport and requires no more than
seven mounted rows, including overscan, with no measured-height entries created before layout.
Sorting and full source replacement are intentionally application events rather than per-frame
work.
