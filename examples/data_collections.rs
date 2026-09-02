use std::{cmp::Ordering, sync::Arc};

use quickgui::{
    Application, Color, ElementId, GridTrack, IntoElement, TableCellPosition, TableColumn,
    TableColumnAlign, TableEditEnded, TableLayout, TableSelectionChanged, TableSelectionMode,
    TableSort, TableSortDirection, TableState, TitleBarStyle, TreeLayout, TreeLoadChildren,
    TreeNode, TreeState, View, ViewContext, div, table_key_bindings, text, text_input,
    tree_key_bindings,
};

const TABLE_ROWS: usize = 20_000;

fn main() -> Result<(), quickgui::AppError> {
    Application::new()
        .bind_keys(table_key_bindings())
        .bind_keys(tree_key_bindings())
        .run(|cx| {
            cx.open_window(
                quickgui::WindowOptions::new("QuickGUI — Table and tree collections")
                    .size(1_120.0, 720.0)
                    .title_bar_style(TitleBarStyle::HiddenInset)
                    .traffic_light_position(16.0, 13.0),
                CollectionsGallery::new(),
            );
        })
}

#[derive(Clone, Debug)]
struct FileRecord {
    name: Arc<str>,
    kind: &'static str,
    bytes: u64,
    modified: u32,
}

#[derive(Clone, Copy)]
struct CollectionPalette {
    page: Color,
    panel: Color,
    foreground: Color,
    muted: Color,
    border: Color,
    row_hover: Color,
    row_selected: Color,
    cell_active: Color,
    focus_ring: Color,
    header_background: Color,
    header_text: Color,
}

impl CollectionPalette {
    fn new(dark: bool) -> Self {
        if dark {
            Self {
                page: Color::rgb8(12, 15, 20),
                panel: Color::rgb8(18, 21, 27),
                foreground: Color::rgb8(232, 234, 239),
                muted: Color::rgb8(143, 149, 162),
                border: Color::rgb8(61, 67, 79),
                row_hover: Color::rgba8(255, 255, 255, 9),
                row_selected: Color::rgb8(31, 62, 98),
                cell_active: Color::rgba8(80, 160, 255, 28),
                focus_ring: Color::rgb8(64, 156, 255),
                header_background: Color::rgb8(31, 35, 43),
                header_text: Color::rgb8(181, 187, 199),
            }
        } else {
            Self {
                page: Color::rgb8(239, 241, 245),
                panel: Color::WHITE,
                foreground: Color::rgb8(28, 31, 38),
                muted: Color::rgb8(102, 108, 120),
                border: Color::rgb8(211, 215, 222),
                row_hover: Color::rgba8(0, 0, 0, 6),
                row_selected: Color::rgb8(220, 235, 252),
                cell_active: Color::rgba8(0, 122, 255, 20),
                focus_ring: Color::rgb8(0, 122, 255),
                header_background: Color::rgb8(246, 247, 249),
                header_text: Color::rgb8(79, 85, 97),
            }
        }
    }
}

struct CollectionsGallery {
    tree: TreeState<&'static str>,
    table: TableState,
    records: Arc<Vec<FileRecord>>,
    order: Arc<[usize]>,
    applied_sort: Option<TableSort>,
    draft: Arc<str>,
    status: Arc<str>,
}

impl CollectionsGallery {
    fn new() -> Self {
        let mut tree = TreeState::new(project_nodes())
            .expect("gallery tree is valid")
            .with_layout(TreeLayout::new(30.0));
        tree.set_expanded("workspace", true);
        tree.set_expanded("src", true);

        let records = (0..TABLE_ROWS)
            .map(|index| FileRecord {
                name: Arc::from(format!("module_{index:05}.rs")),
                kind: if index % 11 == 0 {
                    "Generated"
                } else if index % 3 == 0 {
                    "Test"
                } else {
                    "Rust"
                },
                bytes: 512 + ((index as u64 * 7_919) % 2_000_000),
                modified: 1 + (index as u32 * 37) % 365,
            })
            .collect::<Vec<_>>();
        let order = (0..records.len()).collect::<Vec<_>>();

        Self {
            tree,
            table: TableState::new(records.len())
                .with_layout(TableLayout::new(34.0, 32.0))
                .with_selection_mode(TableSelectionMode::Multiple),
            records: Arc::new(records),
            order: Arc::from(order),
            applied_sort: None,
            draft: Arc::from(""),
            status: Arc::from("Use arrow keys after Tab focuses either collection"),
        }
    }

    fn tree(view: &mut Self) -> &mut TreeState<&'static str> {
        &mut view.tree
    }

    fn table(view: &mut Self) -> &mut TableState {
        &mut view.table
    }

    fn apply_table_sort(&mut self) {
        let requested = self.table.sort();
        if self.applied_sort == requested {
            return;
        }
        let mut order = (0..self.records.len()).collect::<Vec<_>>();
        if let Some(sort) = requested {
            let records = &self.records;
            order.sort_unstable_by(|left, right| {
                let left = &records[*left];
                let right = &records[*right];
                let ordering = if sort.column == ElementId::named("name") {
                    left.name.cmp(&right.name)
                } else if sort.column == ElementId::named("kind") {
                    left.kind.cmp(right.kind)
                } else if sort.column == ElementId::named("size") {
                    left.bytes.cmp(&right.bytes)
                } else if sort.column == ElementId::named("modified") {
                    left.modified.cmp(&right.modified)
                } else {
                    Ordering::Equal
                };
                match sort.direction {
                    TableSortDirection::Ascending => ordering,
                    TableSortDirection::Descending => ordering.reverse(),
                }
                .then_with(|| left.name.cmp(&right.name))
            });
        }
        self.order = Arc::from(order);
        self.applied_sort = requested;
    }

    fn columns() -> [TableColumn; 4] {
        [
            TableColumn::new("name", "Name")
                .track(GridTrack::minmax_px_fr(180.0, 2.2))
                .sortable(true)
                .row_header(true),
            TableColumn::new("kind", "Kind")
                .width(120.0)
                .minimum_width(70.0)
                .sortable(true),
            TableColumn::new("size", "Size")
                .track(GridTrack::px(100.0))
                .align(TableColumnAlign::End)
                .sortable(true),
            TableColumn::new("modified", "Modified")
                .track(GridTrack::px(116.0))
                .align(TableColumnAlign::End)
                .sortable(true),
        ]
    }
}

impl View for CollectionsGallery {
    fn render(&mut self, cx: &mut ViewContext<'_, Self>) -> impl IntoElement {
        self.apply_table_sort();
        let appearance = cx.appearance();
        let palette = CollectionPalette::new(appearance.is_dark());

        let load_children = cx.action_listener(
            "project-tree",
            |view: &mut Self, action: &TreeLoadChildren, cx| {
                let loaded = view.tree.set_children(
                    action.node,
                    [
                        TreeNode::new("network-a", "origin/main", "branch"),
                        TreeNode::new("network-b", "origin/release", "branch"),
                    ],
                );
                view.status = match loaded {
                    Ok(true) => Arc::from("Loaded remote branches"),
                    _ => Arc::from("Remote branches were already loaded"),
                };
                cx.invalidate();
            },
        );

        let tree = self
            .tree
            .element(
                cx,
                "project-tree",
                Self::tree,
                move |row, disclosure| {
                    let row_selected = row.is_selected();
                    let disclosure = disclosure.map_or_else(
                        || div().w(20.0).h(30.0).flex_none(),
                        |disclosure| {
                            disclosure
                                .w(20.0)
                                .h(30.0)
                                .flex_none()
                                .flex_row()
                                .items_center()
                                .justify_center()
                                .text_color(palette.muted)
                                .child(text(if row.is_expanded() { "⌄" } else { "›" }).text_sm())
                        },
                    );
                    div()
                        .padding(0.0, 6.0, 0.0, 6.0 + row.level() as f32 * 16.0)
                        .min_w(0.0)
                        .flex_row()
                        .items_center()
                        .gap_2()
                        .bg(if row_selected {
                            palette.row_selected
                        } else {
                            Color::TRANSPARENT
                        })
                        .hover(move |hover| {
                            hover.bg(if row_selected {
                                palette.row_selected
                            } else {
                                palette.row_hover
                            })
                        })
                        .disabled_style(|disabled| disabled.opacity(0.45))
                        .child(disclosure)
                        .child(
                            text(if row.has_children() { "◇" } else { "·" })
                                .text_xs()
                                .text_color(Color::rgb8(94, 234, 212)),
                        )
                        .child(
                            text(row.label().clone())
                                .text_sm()
                                .no_wrap()
                                .text_ellipsis(),
                        )
                },
                |view, id, cx| {
                    view.status = Arc::from(format!("Activated tree node {}", id.as_u64()));
                    cx.invalidate();
                },
            )
            .rounded(8.0)
            .border(1.0, palette.border)
            .bg(palette.panel)
            .text_color(palette.foreground)
            .focus(move |focus| focus.border(2.0, palette.focus_ring))
            .on_action(load_children);

        let selection_changed = cx.action_listener(
            "file-table",
            |view: &mut Self, _: &TableSelectionChanged, cx| {
                let rows = view.table.selection().len();
                view.status = Arc::from(format!("{rows} selected row(s)"));
                cx.invalidate();
            },
        );
        let edit_ended = cx.action_listener(
            "file-table",
            |view: &mut Self, action: &TableEditEnded, cx| {
                if action.committed && !view.draft.is_empty() {
                    let record = view.order[action.position.row];
                    Arc::make_mut(&mut view.records)[record].name = Arc::clone(&view.draft);
                    view.status = Arc::from(format!("Renamed row {}", action.position.row));
                } else {
                    view.status = Arc::from("Edit cancelled");
                }
                cx.invalidate();
            },
        );
        let edit_input = cx.input_listener("cell-editor", |view: &mut Self, value, cx| {
            view.draft = Arc::from(value);
            cx.invalidate();
        });

        let records = Arc::clone(&self.records);
        let order = Arc::clone(&self.order);
        let draft = Arc::clone(&self.draft);
        let table = self
            .table
            .element(
                cx,
                "file-table",
                &Self::columns(),
                Self::table,
                move |header| {
                    let mut element = div()
                        .px(10.0)
                        .bg(palette.header_background)
                        .text_color(palette.header_text)
                        .child(
                            text(header.column.label().clone())
                                .text_xs()
                                .font_semibold()
                                .no_wrap()
                                .text_ellipsis(),
                        );
                    if let Some(direction) = header.sort_direction {
                        element = element.child(
                            text(match direction {
                                TableSortDirection::Ascending => "↑",
                                TableSortDirection::Descending => "↓",
                            })
                            .ml(4.0)
                            .text_xs()
                            .accessibility_hidden(true),
                        );
                    }
                    if let Some(handle) = header.resize_handle {
                        element = element.justify_between().child(
                            handle
                                .w(7.0)
                                .h_full()
                                .flex_none()
                                .bg(Color::TRANSPARENT)
                                .hover(move |hover| hover.bg(palette.focus_ring))
                                .focus(move |focus| focus.bg(palette.focus_ring)),
                        );
                    }
                    element
                },
                move |cell| {
                    let row = cell.position.row;
                    let column = cell.position.column;
                    if cell.editing {
                        return div().px(6.0).child(
                            text_input(draft.clone())
                                .id("cell-editor")
                                .auto_focus()
                                .on_input(edit_input)
                                .w_full()
                                .text_sm(),
                        );
                    }
                    let record = &records[order[row]];
                    let content = match column {
                        0 => text(record.name.clone())
                            .text_sm()
                            .no_wrap()
                            .text_ellipsis(),
                        1 => text(record.kind).text_sm().no_wrap().text_ellipsis(),
                        2 => text(format_bytes(record.bytes)).text_sm().no_wrap(),
                        _ => text(format!("{} days ago", record.modified))
                            .text_sm()
                            .no_wrap(),
                    };
                    let background = if cell.selected {
                        palette.cell_active
                    } else if cell.row_selected {
                        palette.row_selected
                    } else {
                        Color::TRANSPARENT
                    };
                    let selected = cell.selected;
                    let row_selected = cell.row_selected;
                    div()
                        .px(10.0)
                        .border(1.0, palette.border)
                        .bg(background)
                        .hover(move |hover| {
                            hover.bg(if selected {
                                palette.cell_active
                            } else if row_selected {
                                palette.row_selected
                            } else {
                                palette.row_hover
                            })
                        })
                        .child(content)
                },
                |view, position @ TableCellPosition { row, column }, cx| {
                    if column == 0 {
                        view.draft = view.records[view.order[row]].name.clone();
                        view.table.begin_edit(position, 4);
                        view.status = Arc::from(format!("Editing row {row}; Return commits"));
                    } else {
                        view.status =
                            Arc::from(format!("Activated table row {row}, column {column}"));
                    }
                    cx.invalidate();
                },
            )
            .rounded(8.0)
            .border(1.0, palette.border)
            .bg(palette.panel)
            .text_color(palette.foreground)
            .focus(move |focus| focus.border(2.0, palette.focus_ring))
            .on_action(selection_changed)
            .on_action(edit_ended);

        let metrics = cx.metrics();
        div()
            .size_full()
            .flex_col()
            .bg(palette.page)
            .child(
                div()
                    .h(52.0)
                    .flex_none()
                    .flex_row()
                    .items_center()
                    .justify_between()
                    .px(20.0)
                    .app_region_drag()
                    .child(text("Virtual collections").font_semibold())
                    .child(
                        text("20,000 table rows · bounded tree arena")
                            .text_xs()
                            .text_color(palette.muted),
                    ),
            )
            .child(
                div()
                    .flex_1()
                    .min_h(0.0)
                    .flex_row()
                    .gap_3()
                    .p_3()
                    .app_region_no_drag()
                    .child(
                        div()
                            .w(280.0)
                            .h_full()
                            .flex_none()
                            .flex_col()
                            .gap_2()
                            .p_3()
                            .rounded_xl()
                            .bg(palette.panel)
                            .child(text("Project tree").text_sm().font_semibold())
                            .child(
                                text("Left/right expands and collapses; `remotes` loads its children on first expansion.")
                                    .text_xs()
                                    .text_color(palette.muted),
                            )
                            .child(div().flex_1().min_h(0.0).child(tree)),
                    )
                    .child(
                        div()
                            .flex_1()
                            .min_w(0.0)
                            .h_full()
                            .flex_col()
                            .gap_2()
                            .p_3()
                            .rounded_xl()
                            .bg(palette.panel)
                            .child(text("Sortable data table").text_sm().font_semibold())
                            .child(
                                text("Shift and Command extend the selection, Command-A selects all, Alt-Left/Right reorders columns, the Kind divider resizes, and Return edits a name.")
                                    .text_xs()
                                    .text_color(palette.muted),
                            )
                            .child(div().flex_1().min_h(0.0).child(table)),
                    ),
            )
            .child(
                div()
                    .h(30.0)
                    .flex_none()
                    .flex_row()
                    .items_center()
                    .justify_between()
                    .px_4()
                    .text_color(palette.muted)
                    .child(text(self.status.clone()).text_xs().no_wrap().text_ellipsis())
                    .child(
                        text(format!(
                            "{:.2} ms CPU · {} draw calls · {} shaped",
                            metrics.cpu_milliseconds(),
                            metrics.render.draw_calls,
                            metrics.render.reshaped_text_areas,
                        ))
                        .text_xs()
                        .no_wrap(),
                    ),
            )
    }
}

fn project_nodes() -> Vec<TreeNode<&'static str>> {
    vec![
        TreeNode::new("workspace", "quickgui", "workspace")
            .child(TreeNode::new("src", "src", "directory").children([
                TreeNode::new("runtime", "runtime.rs", "rust"),
                TreeNode::new("renderer", "renderer.rs", "rust"),
                TreeNode::new("tree-source", "tree.rs", "rust"),
                TreeNode::new("table-source", "table.rs", "rust"),
            ]))
            .child(
                TreeNode::new("examples", "examples", "directory").child(TreeNode::new(
                    "gallery",
                    "data_collections.rs",
                    "rust",
                )),
            )
            .child(TreeNode::new("target", "target", "directory").disabled(true)),
        TreeNode::new("network", "remotes", "directory").pending(true),
        TreeNode::new("readme", "README.md", "markdown"),
        TreeNode::new("manifest", "Cargo.toml", "toml"),
    ]
}

fn format_bytes(bytes: u64) -> String {
    if bytes >= 1_000_000 {
        format!("{:.1} MB", bytes as f64 / 1_000_000.0)
    } else if bytes >= 1_000 {
        format!("{:.1} KB", bytes as f64 / 1_000.0)
    } else {
        format!("{bytes} B")
    }
}
