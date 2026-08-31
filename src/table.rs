use std::{fmt, sync::Arc};

use crate::{
    AccessibilityRole, AccessibilitySortDirection, Element, ElementId, EventContext, FocusHandle,
    GridTrack, IntoElement, KeyBinding, ListState, MAX_GRID_TRACKS, ViewContext, div,
};

/// Maximum logical rows managed by one reusable table.
pub const MAX_TABLE_ROWS: usize = 1_000_000;
/// Maximum columns retained in one table declaration.
pub const MAX_TABLE_COLUMNS: usize = MAX_GRID_TRACKS as usize;

const TABLE_KEY_CONTEXT: &str = "Table";
const TABLE_ROW_ID_TAG: u64 = 0xa9e0_2f75_8315_a7d1;
const TABLE_CELL_ID_TAG: u64 = 0xd2e1_6bd2_435b_0f97;
const TABLE_HEADER_ID_TAG: u64 = 0x49a7_3c01_94d8_62ef;

/// Move the active table cell one row upward.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct TablePreviousRow;
/// Move the active table cell one row downward.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct TableNextRow;
/// Move the active table cell one column left.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct TablePreviousColumn;
/// Move the active table cell one column right.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct TableNextColumn;
/// Move the active table cell one visible page upward.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct TablePageUp;
/// Move the active table cell one visible page downward.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct TablePageDown;
/// Move the active table cell to the first row.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct TableFirstRow;
/// Move the active table cell to the final row.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct TableLastRow;
/// Activate the current table cell.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct TableActivate;

/// Contextual bindings used by [`TableState::element`].
pub fn table_key_bindings() -> [KeyBinding; 9] {
    [
        KeyBinding::new("up", TablePreviousRow, Some(TABLE_KEY_CONTEXT)),
        KeyBinding::new("down", TableNextRow, Some(TABLE_KEY_CONTEXT)),
        KeyBinding::new("left", TablePreviousColumn, Some(TABLE_KEY_CONTEXT)),
        KeyBinding::new("right", TableNextColumn, Some(TABLE_KEY_CONTEXT)),
        KeyBinding::new("pageup", TablePageUp, Some(TABLE_KEY_CONTEXT)),
        KeyBinding::new("pagedown", TablePageDown, Some(TABLE_KEY_CONTEXT)),
        KeyBinding::new("platform-up", TableFirstRow, Some(TABLE_KEY_CONTEXT)),
        KeyBinding::new("platform-down", TableLastRow, Some(TABLE_KEY_CONTEXT)),
        KeyBinding::new("enter", TableActivate, Some(TABLE_KEY_CONTEXT)),
    ]
}

/// Horizontal alignment for one table column.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum TableColumnAlign {
    #[default]
    Start,
    Center,
    End,
}

/// One stable table column declaration.
#[derive(Clone, Debug, PartialEq)]
pub struct TableColumn {
    id: ElementId,
    label: Arc<str>,
    track: GridTrack,
    align: TableColumnAlign,
    sortable: bool,
    row_header: bool,
}

impl TableColumn {
    pub fn new(id: impl Into<ElementId>, label: impl Into<Arc<str>>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            track: GridTrack::fr(1.0),
            align: TableColumnAlign::Start,
            sortable: false,
            row_header: false,
        }
    }

    pub fn track(mut self, track: GridTrack) -> Self {
        self.track = track;
        self
    }

    pub fn align(mut self, align: TableColumnAlign) -> Self {
        self.align = align;
        self
    }

    pub fn sortable(mut self, sortable: bool) -> Self {
        self.sortable = sortable;
        self
    }

    /// Mark data cells in this column as row headers instead of ordinary grid cells.
    pub fn row_header(mut self, row_header: bool) -> Self {
        self.row_header = row_header;
        self
    }

    pub const fn id(&self) -> ElementId {
        self.id
    }

    pub fn label(&self) -> &Arc<str> {
        &self.label
    }

    pub const fn grid_track(&self) -> GridTrack {
        self.track
    }

    pub const fn alignment(&self) -> TableColumnAlign {
        self.align
    }

    pub const fn is_sortable(&self) -> bool {
        self.sortable
    }

    pub const fn is_row_header(&self) -> bool {
        self.row_header
    }
}

/// Direction requested for an application-owned table ordering.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TableSortDirection {
    Ascending,
    Descending,
}

impl TableSortDirection {
    const fn accessibility(self) -> AccessibilitySortDirection {
        match self {
            Self::Ascending => AccessibilitySortDirection::Ascending,
            Self::Descending => AccessibilitySortDirection::Descending,
        }
    }
}

/// Current sort declaration. QuickGUI renders and exposes it; the owning view orders its data.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TableSort {
    pub column: ElementId,
    pub direction: TableSortDirection,
}

/// Zero-based logical position of one table cell.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct TableCellPosition {
    pub row: usize,
    pub column: usize,
}

/// State supplied to one caller-owned column-header renderer.
#[derive(Clone, Copy, Debug)]
pub struct TableHeaderState<'a> {
    pub column_index: usize,
    pub column: &'a TableColumn,
    pub sort_direction: Option<TableSortDirection>,
}

/// State supplied to one caller-owned table-cell renderer.
#[derive(Clone, Copy, Debug)]
pub struct TableCellState<'a> {
    pub position: TableCellPosition,
    pub column: &'a TableColumn,
    pub row_selected: bool,
    pub selected: bool,
}

/// Structural geometry retained by one virtualized table.
///
/// No color, typography, padding, border, radius, shadow, or interaction-state paint is retained.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TableLayout {
    pub header_height: f32,
    pub row_height: f32,
}

impl TableLayout {
    pub fn new(header_height: f32, row_height: f32) -> Self {
        Self {
            header_height: finite_clamped(header_height, 20.0, 256.0, 34.0),
            row_height: finite_clamped(row_height, 20.0, 256.0, 32.0),
        }
    }

    pub fn row_height(mut self, height: f32) -> Self {
        self.row_height = finite_clamped(height, 20.0, 256.0, 32.0);
        self
    }

    pub fn header_height(mut self, height: f32) -> Self {
        self.header_height = finite_clamped(height, 20.0, 256.0, 34.0);
        self
    }

    fn sanitized(mut self) -> Self {
        self.header_height = finite_clamped(self.header_height, 20.0, 256.0, 34.0);
        self.row_height = finite_clamped(self.row_height, 20.0, 256.0, 32.0);
        self
    }
}

impl Default for TableLayout {
    fn default() -> Self {
        Self::new(34.0, 32.0)
    }
}

/// Retained scroll, cell selection, and sort state for a virtualized table.
///
/// The application owns row data and applies [`Self::sort`] to its ordering. QuickGUI retains only
/// constant-size interaction state plus the existing sparse [`ListState`] metrics, mounts visible
/// rows, and owns no timer or idle scheduler source.
pub struct TableState {
    row_count: usize,
    selected: Option<TableCellPosition>,
    sort: Option<TableSort>,
    list: ListState,
    layout: TableLayout,
}

impl fmt::Debug for TableState {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("TableState")
            .field("row_count", &self.row_count)
            .field("selected", &self.selected)
            .field("sort", &self.sort)
            .field("list", &self.list)
            .field("layout", &self.layout)
            .finish()
    }
}

impl TableState {
    pub fn new(row_count: usize) -> Self {
        assert!(
            row_count <= MAX_TABLE_ROWS,
            "a table supports at most {MAX_TABLE_ROWS} rows"
        );
        let layout = TableLayout::default();
        Self {
            row_count,
            selected: None,
            sort: None,
            list: ListState::new(row_count, layout.row_height).with_overscan(2),
            layout,
        }
    }

    pub fn with_layout(mut self, layout: TableLayout) -> Self {
        self.layout = layout.sanitized();
        self.list = ListState::new(self.row_count, self.layout.row_height).with_overscan(2);
        self
    }

    pub const fn layout(&self) -> TableLayout {
        self.layout
    }

    pub fn set_layout(&mut self, layout: TableLayout) -> bool {
        let layout = layout.sanitized();
        if self.layout == layout {
            return false;
        }
        if self.layout.row_height != layout.row_height {
            let viewport = self.list.viewport_size();
            let offset = self.list.logical_scroll_top();
            self.list = ListState::new(self.row_count, layout.row_height).with_overscan(2);
            self.list.set_viewport_size(viewport.width, viewport.height);
            self.list.scroll_to(offset);
        }
        self.layout = layout;
        true
    }

    pub const fn row_count(&self) -> usize {
        self.row_count
    }

    pub fn set_row_count(&mut self, row_count: usize) -> bool {
        assert!(
            row_count <= MAX_TABLE_ROWS,
            "a table supports at most {MAX_TABLE_ROWS} rows"
        );
        if self.row_count == row_count {
            return false;
        }
        self.row_count = row_count;
        self.list.set_item_count(row_count);
        if let Some(selected) = &mut self.selected
            && selected.row >= row_count
        {
            self.selected = row_count.checked_sub(1).map(|row| TableCellPosition {
                row,
                column: selected.column,
            });
        }
        true
    }

    pub const fn selected_cell(&self) -> Option<TableCellPosition> {
        self.selected
    }

    pub fn selected_row(&self) -> Option<usize> {
        self.selected.map(|cell| cell.row)
    }

    pub const fn sort(&self) -> Option<TableSort> {
        self.sort
    }

    pub fn set_sort(&mut self, sort: Option<TableSort>) -> bool {
        if self.sort == sort {
            return false;
        }
        self.sort = sort;
        true
    }

    pub fn clear_selection(&mut self) -> bool {
        self.selected.take().is_some()
    }

    pub fn select_cell(&mut self, position: TableCellPosition, column_count: usize) -> bool {
        if position.row >= self.row_count || position.column >= column_count {
            return false;
        }
        let changed = self.selected != Some(position);
        self.selected = Some(position);
        self.list.scroll_to_reveal_item(position.row) || changed
    }

    pub fn visible_rows(&self) -> std::ops::Range<usize> {
        self.list.visible_rows().range
    }

    pub fn list_state(&self) -> &ListState {
        &self.list
    }

    pub fn focus_handle(id: impl Into<ElementId>) -> FocusHandle {
        FocusHandle::new(id)
    }

    pub fn row_id(id: impl Into<ElementId>, row: usize) -> ElementId {
        derived_table_id(id.into(), TABLE_ROW_ID_TAG, row as u64, 0)
    }

    pub fn cell_id(id: impl Into<ElementId>, position: TableCellPosition) -> ElementId {
        derived_table_id(
            id.into(),
            TABLE_CELL_ID_TAG,
            position.row as u64,
            position.column as u64,
        )
    }

    pub fn column_header_id(id: impl Into<ElementId>, column: ElementId) -> ElementId {
        derived_table_id(id.into(), TABLE_HEADER_ID_TAG, column.as_u64(), 0)
    }

    /// Build a complete unstyled virtualized, sortable, keyboard-navigable table.
    ///
    /// `render_header` owns every visible header declaration, including label and sort indicator.
    /// `render_cell` owns every visible cell declaration and runs only for mounted rows. QuickGUI
    /// decorates those elements with fixed grid geometry, identities, pointer/keyboard behavior,
    /// and accessibility semantics but adds no paint. `activate` receives the current logical cell
    /// when Enter is dispatched; selection and sort state remain directly readable from `access`.
    #[allow(clippy::too_many_arguments)]
    pub fn element<V, H, E, RenderHeader, RenderCell, Activate>(
        &mut self,
        cx: &mut ViewContext<'_, V>,
        id: impl Into<ElementId>,
        columns: &[TableColumn],
        access: fn(&mut V) -> &mut TableState,
        mut render_header: RenderHeader,
        mut render_cell: RenderCell,
        activate: Activate,
    ) -> Element
    where
        V: 'static,
        H: IntoElement,
        E: IntoElement,
        RenderHeader: FnMut(TableHeaderState<'_>) -> H,
        RenderCell: FnMut(TableCellState<'_>) -> E,
        Activate: Fn(&mut V, TableCellPosition, &mut EventContext) + Clone + 'static,
    {
        assert_table_columns(columns);
        let id = id.into();
        let column_count = columns.len();
        self.normalize_selection(column_count);
        let layout = self.layout;
        let root_focus = Self::focus_handle(id);

        let previous_row = cx.action_listener(id, move |view, _: &TablePreviousRow, cx| {
            if access(view).move_row(false, column_count) {
                cx.invalidate();
            }
        });
        let next_row = cx.action_listener(id, move |view, _: &TableNextRow, cx| {
            if access(view).move_row(true, column_count) {
                cx.invalidate();
            }
        });
        let previous_column = cx.action_listener(id, move |view, _: &TablePreviousColumn, cx| {
            if access(view).move_column(false, column_count) {
                cx.invalidate();
            }
        });
        let next_column = cx.action_listener(id, move |view, _: &TableNextColumn, cx| {
            if access(view).move_column(true, column_count) {
                cx.invalidate();
            }
        });
        let page_up = cx.action_listener(id, move |view, _: &TablePageUp, cx| {
            if access(view).move_page(false, column_count) {
                cx.invalidate();
            }
        });
        let page_down = cx.action_listener(id, move |view, _: &TablePageDown, cx| {
            if access(view).move_page(true, column_count) {
                cx.invalidate();
            }
        });
        let first = cx.action_listener(id, move |view, _: &TableFirstRow, cx| {
            if access(view).select_edge(false, column_count) {
                cx.invalidate();
            }
        });
        let last = cx.action_listener(id, move |view, _: &TableLastRow, cx| {
            if access(view).select_edge(true, column_count) {
                cx.invalidate();
            }
        });
        let confirm_activate = activate.clone();
        let confirm = cx.action_listener(id, move |view, _: &TableActivate, cx| {
            if let Some(selected) = access(view).selected_cell() {
                confirm_activate(view, selected, cx);
            }
        });

        let tracks = columns
            .iter()
            .map(TableColumn::grid_track)
            .collect::<Vec<_>>();
        let mut headers = Vec::with_capacity(column_count);
        for (column_index, column) in columns.iter().enumerate() {
            let header_id = Self::column_header_id(id, column.id);
            let sort_direction = self
                .sort
                .filter(|sort| sort.column == column.id)
                .map(|sort| sort.direction);
            let mut header = render_header(TableHeaderState {
                column_index,
                column,
                sort_direction,
            })
            .into_element()
            .id(header_id)
            .accessibility_role(AccessibilityRole::ColumnHeader)
            .accessibility_label(column.label.clone())
            .accessibility_column_index(column_index)
            .h(layout.header_height)
            .min_w(0.0)
            .flex_row()
            .items_center()
            .overflow_hidden()
            .cursor_default()
            .app_region_no_drag()
            .user_select_none();
            header = align_cell(header, column.align);
            if let Some(direction) = sort_direction {
                header = header.accessibility_sort_direction(direction.accessibility());
            }
            if column.sortable {
                let column_id = column.id;
                let clicked = cx.listener(header_id, move |view, cx| {
                    if access(view).toggle_sort(column_id) {
                        cx.focus(root_focus);
                        cx.invalidate();
                    }
                });
                header = header.on_click(clicked).tab_index(-1);
            }
            headers.push(header);
        }

        let header = div()
            .accessibility_role(AccessibilityRole::Row)
            .accessibility_row_index(0)
            .grid()
            .grid_template_columns(tracks.clone())
            .h(layout.header_height)
            .flex_none()
            .app_region_no_drag()
            .children(headers);

        let selected = self.selected;
        let list = self.list.clone();
        let visible = list.visible_rows().range;
        let rows = list.render_rows(visible, |row_index| {
            let row_selected = selected.is_some_and(|cell| cell.row == row_index);
            let row_id = Self::row_id(id, row_index);
            let mut cells = Vec::with_capacity(column_count);
            for (column_index, column) in columns.iter().enumerate() {
                let position = TableCellPosition {
                    row: row_index,
                    column: column_index,
                };
                let cell_id = Self::cell_id(id, position);
                let cell_selected = selected == Some(position);
                let clicked = cx.listener(cell_id, move |view, cx| {
                    if access(view).select_cell(position, column_count) {
                        cx.invalidate();
                    }
                    cx.focus(root_focus);
                });
                let role = if column.row_header {
                    AccessibilityRole::RowHeader
                } else {
                    AccessibilityRole::GridCell
                };
                let mut cell = render_cell(TableCellState {
                    position,
                    column,
                    row_selected,
                    selected: cell_selected,
                })
                .into_element()
                .id(cell_id)
                .on_click(clicked)
                .tab_index(-1)
                .accessibility_role(role)
                .accessibility_row_index(row_index + 1)
                .accessibility_column_index(column_index)
                .selected(cell_selected)
                .h(layout.row_height)
                .min_w(0.0)
                .flex_row()
                .items_center()
                .overflow_hidden()
                .cursor_default()
                .app_region_no_drag();
                cell = align_cell(cell, column.align);
                cells.push(cell);
            }
            div()
                .id(row_id)
                .accessibility_role(AccessibilityRole::Row)
                .accessibility_row_index(row_index + 1)
                .selected(row_selected)
                .grid()
                .grid_template_columns(tracks.clone())
                .h(layout.row_height)
                .app_region_no_drag()
                .children(cells)
        });

        let body = div()
            .relative()
            .flex_1()
            .min_h(0.0)
            .w_full()
            .overflow_hidden()
            .variable_virtual_scroll(&list)
            .app_region_no_drag()
            .child(rows);

        let mut root = div()
            .id(id)
            .track_focus(root_focus)
            .key_context(TABLE_KEY_CONTEXT)
            .on_action(previous_row)
            .on_action(next_row)
            .on_action(previous_column)
            .on_action(next_column)
            .on_action(page_up)
            .on_action(page_down)
            .on_action(first)
            .on_action(last)
            .on_action(confirm)
            .accessibility_role(AccessibilityRole::Grid)
            .accessibility_row_count(self.row_count.saturating_add(1))
            .accessibility_column_count(column_count)
            .size_full()
            .min_w(0.0)
            .min_h(0.0)
            .flex_col()
            .overflow_hidden()
            .app_region_no_drag()
            .child(header)
            .child(body);
        if let Some(selected) = self.selected {
            root = root.accessibility_active_descendant(Self::cell_id(id, selected));
        }
        root
    }

    fn normalize_selection(&mut self, column_count: usize) {
        if self.row_count == 0 || column_count == 0 {
            self.selected = None;
            return;
        }
        let position = self.selected.unwrap_or_default();
        self.selected = Some(TableCellPosition {
            row: position.row.min(self.row_count - 1),
            column: position.column.min(column_count - 1),
        });
        if let Some(selected) = self.selected {
            self.list.scroll_to_reveal_item(selected.row);
        }
    }

    fn move_row(&mut self, forward: bool, column_count: usize) -> bool {
        self.normalize_selection(column_count);
        let Some(mut selected) = self.selected else {
            return false;
        };
        selected.row = if forward {
            selected.row.saturating_add(1).min(self.row_count - 1)
        } else {
            selected.row.saturating_sub(1)
        };
        self.select_cell(selected, column_count)
    }

    fn move_column(&mut self, forward: bool, column_count: usize) -> bool {
        self.normalize_selection(column_count);
        let Some(mut selected) = self.selected else {
            return false;
        };
        selected.column = if forward {
            selected.column.saturating_add(1).min(column_count - 1)
        } else {
            selected.column.saturating_sub(1)
        };
        self.select_cell(selected, column_count)
    }

    fn move_page(&mut self, forward: bool, column_count: usize) -> bool {
        self.normalize_selection(column_count);
        let Some(mut selected) = self.selected else {
            return false;
        };
        let page = (self.list.viewport_size().height / self.layout.row_height)
            .floor()
            .max(1.0) as usize;
        selected.row = if forward {
            selected.row.saturating_add(page).min(self.row_count - 1)
        } else {
            selected.row.saturating_sub(page)
        };
        self.select_cell(selected, column_count)
    }

    fn select_edge(&mut self, end: bool, column_count: usize) -> bool {
        self.normalize_selection(column_count);
        let Some(mut selected) = self.selected else {
            return false;
        };
        selected.row = if end { self.row_count - 1 } else { 0 };
        self.select_cell(selected, column_count)
    }

    fn toggle_sort(&mut self, column: ElementId) -> bool {
        self.sort = Some(match self.sort {
            Some(TableSort {
                column: current,
                direction: TableSortDirection::Ascending,
            }) if current == column => TableSort {
                column,
                direction: TableSortDirection::Descending,
            },
            _ => TableSort {
                column,
                direction: TableSortDirection::Ascending,
            },
        });
        true
    }
}

fn assert_table_columns(columns: &[TableColumn]) {
    assert!(
        columns.len() <= MAX_TABLE_COLUMNS,
        "a table supports at most {MAX_TABLE_COLUMNS} columns"
    );
    for (index, column) in columns.iter().enumerate() {
        assert!(
            columns[..index].iter().all(|other| other.id != column.id),
            "table column IDs must be unique"
        );
    }
}

fn align_cell(element: Element, alignment: TableColumnAlign) -> Element {
    match alignment {
        TableColumnAlign::Start => element.justify_start(),
        TableColumnAlign::Center => element.justify_center(),
        TableColumnAlign::End => element.justify_end(),
    }
}

fn derived_table_id(parent: ElementId, tag: u64, first: u64, second: u64) -> ElementId {
    let mut hash = parent.as_u64()
        ^ tag
        ^ first.wrapping_mul(0x9e37_79b9_7f4a_7c15)
        ^ second.wrapping_mul(0xd6e8_feb8_6659_fd93);
    hash ^= hash >> 30;
    hash = hash.wrapping_mul(0xbf58_476d_1ce4_e5b9);
    hash ^= hash >> 27;
    hash = hash.wrapping_mul(0x94d0_49bb_1331_11eb);
    hash ^= hash >> 31;
    if hash == parent.as_u64() || hash == u64::MAX {
        hash ^= tag.rotate_left(13);
    }
    ElementId::new(hash)
}

fn finite_clamped(value: f32, minimum: f32, maximum: f32, fallback: f32) -> f32 {
    if value.is_finite() {
        value.clamp(minimum, maximum)
    } else {
        fallback.clamp(minimum, maximum)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Application, Color, View, WindowOptions, text};

    #[test]
    fn layout_and_state_stay_bounded_and_preserve_logical_scroll_on_geometry_changes() {
        let mut state = TableState::new(100);
        state.list.set_viewport_size(400.0, 96.0);
        assert!(state.select_cell(TableCellPosition { row: 50, column: 1 }, 3));
        let before = state.list.logical_scroll_top();
        assert!(
            state.set_layout(
                TableLayout::default()
                    .row_height(f32::NAN)
                    .header_height(-10.0)
            )
        );
        assert_eq!(state.layout.row_height, 32.0);
        assert_eq!(state.layout.header_height, 20.0);
        assert_eq!(state.list.logical_scroll_top().item_ix, before.item_ix);
        assert!(state.visible_rows().len() < state.row_count());
    }

    #[derive(Debug)]
    struct TableView {
        table: TableState,
        activated: Option<TableCellPosition>,
    }

    impl Default for TableView {
        fn default() -> Self {
            Self {
                table: TableState::new(100),
                activated: None,
            }
        }
    }

    impl TableView {
        fn table(view: &mut Self) -> &mut TableState {
            &mut view.table
        }
    }

    impl View for TableView {
        fn render(&mut self, cx: &mut ViewContext<'_, Self>) -> impl IntoElement {
            let columns = [
                TableColumn::new("name", "Name")
                    .track(GridTrack::fr(2.0))
                    .sortable(true)
                    .row_header(true),
                TableColumn::new("status", "Status").sortable(true),
                TableColumn::new("cpu", "CPU").align(TableColumnAlign::End),
            ];
            self.table
                .element(
                    cx,
                    "table",
                    &columns,
                    Self::table,
                    |header| {
                        let mut element =
                            div().child(text(header.column.label().clone()).no_wrap());
                        if let Some(direction) = header.sort_direction {
                            element = element.child(text(match direction {
                                TableSortDirection::Ascending => "↑",
                                TableSortDirection::Descending => "↓",
                            }));
                        }
                        element
                    },
                    |cell| {
                        div()
                            .bg(if cell.selected {
                                Color::rgb8(10, 20, 30)
                            } else {
                                Color::TRANSPARENT
                            })
                            .child(
                                text(format!("{}:{}", cell.position.row, cell.position.column))
                                    .no_wrap(),
                            )
                    },
                    |view, position, cx| {
                        view.activated = Some(position);
                        cx.invalidate();
                    },
                )
                .bg(Color::rgb8(1, 2, 3))
        }
    }

    #[test]
    fn table_uses_composite_focus_keyboard_sort_click_and_idle_paths() {
        let (mut cx, view) = Application::new()
            .bind_keys(table_key_bindings())
            .into_test_context(WindowOptions::default(), TableView::default())
            .unwrap();
        let window = view.window_handle();

        cx.simulate_keystrokes(window, "tab down right enter")
            .unwrap();
        assert_eq!(cx.focused(window).unwrap(), Some("table".into()));
        assert_eq!(
            cx.read(view, |view| view.table.selected_cell()).unwrap(),
            Some(TableCellPosition { row: 1, column: 1 })
        );
        assert_eq!(
            cx.read(view, |view| view.activated).unwrap(),
            Some(TableCellPosition { row: 1, column: 1 })
        );

        let header = TableState::column_header_id("table", "status".into());
        cx.click(window, header).unwrap();
        assert_eq!(
            cx.read(view, |view| view.table.sort()).unwrap(),
            Some(TableSort {
                column: "status".into(),
                direction: TableSortDirection::Ascending,
            })
        );
        cx.click(window, header).unwrap();
        assert_eq!(
            cx.read(view, |view| view.table.sort()).unwrap(),
            Some(TableSort {
                column: "status".into(),
                direction: TableSortDirection::Descending,
            })
        );

        let renders = cx.render_count(window).unwrap();
        cx.run_until_idle().unwrap();
        assert_eq!(cx.render_count(window).unwrap(), renders);
    }

    #[test]
    #[should_panic(expected = "table column IDs must be unique")]
    fn duplicate_column_ids_fail_before_building_ambiguous_cells() {
        assert_table_columns(&[TableColumn::new("same", "A"), TableColumn::new("same", "B")]);
    }

    #[test]
    fn table_bindings_are_contextual_and_complete() {
        let bindings = table_key_bindings();
        assert_eq!(bindings.len(), 9);
        assert!(
            bindings.iter().all(
                |binding| binding.context_predicate().is_some_and(|context| context
                    .depth_of(&[crate::KeyContext::parse(TABLE_KEY_CONTEXT).unwrap()])
                    .is_some())
            )
        );
    }
}
