//! Declared virtual tables and trees bound to the Rust core's `TableState` and `TreeState`.
//!
//! Both collections are on-demand: the core owns the virtual window, and the rows frontends
//! declare are the rows the core last reported as visible. That range travels back as one
//! asynchronous `componentchange` payload, exactly like every other result the core decides, so
//! the hosted boundary never answers a synchronous question while a scroll is in flight.
//!
//! Row and cell content is still ordinary frontend content: the binding builds those elements
//! from the retained tree before it hands the core its row renderer, so the renderer the core
//! retains never reaches back into the hosted tree. Column widths, display order, sort direction,
//! selection ranges, expansion, lazy children, and inline-edit lifetime all live in the core.

use super::*;
use serde::Deserialize;

/// Declared part name of a virtual table root.
pub(super) const TABLE_PART: &str = "table";
/// Declared part name of one declared column header.
pub(super) const TABLE_HEADER_PART: &str = "table-header";
/// Declared part name of one declared row.
pub(super) const TABLE_ROW_PART: &str = "table-row";
/// Declared part name of one declared cell.
pub(super) const TABLE_CELL_PART: &str = "table-cell";
/// Declared part name of a virtual tree root.
pub(super) const TREE_PART: &str = "tree";
/// Declared part name of one declared tree row.
pub(super) const TREE_ROW_PART: &str = "tree-row";

/// Longest bounded column, node, or selection declaration accepted from one property.
pub(super) const MAX_COLLECTION_JSON_BYTES: usize = 2 * 1024 * 1024;
/// Most declared tree nodes decoded from one `nodes` or `setChildren` declaration.
pub(super) const MAX_DECLARED_TREE_NODES: usize = 65_536;
/// Most declared selection ranges decoded from or reported to one table.
pub(super) const MAX_DECLARED_SELECTION_RANGES: usize = quickgui::MAX_TABLE_SELECTION_RANGES;

const DEFAULT_TABLE_ROW_HEIGHT: f32 = 28.0;
const DEFAULT_TABLE_HEADER_HEIGHT: f32 = 32.0;
const DEFAULT_TREE_ROW_HEIGHT: f32 = 24.0;

/// One declared table column.
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DeclaredColumn {
    #[serde(default)]
    id: String,
    #[serde(default)]
    label: Option<String>,
    #[serde(default)]
    width: Option<f32>,
    #[serde(default)]
    min_width: Option<f32>,
    #[serde(default)]
    align: Option<String>,
    #[serde(default)]
    sortable: bool,
    #[serde(default)]
    row_header: bool,
    /// A CSS grid track for a column that is not user-resizable, such as `1fr` or `auto`.
    #[serde(default)]
    track: Option<String>,
}

/// One declared tree node, which may declare its own children or be lazily pending.
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DeclaredTreeNode {
    #[serde(default)]
    id: String,
    #[serde(default)]
    label: Option<String>,
    #[serde(default)]
    disabled: bool,
    #[serde(default)]
    pending: bool,
    #[serde(default)]
    children: Vec<DeclaredTreeNode>,
}

/// One bounded lazy-children declaration.
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DeclaredChildren {
    #[serde(default)]
    id: String,
    #[serde(default)]
    children: Vec<DeclaredTreeNode>,
}

/// One declared inline-edit position.
#[derive(Clone, Copy, Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DeclaredCell {
    #[serde(default)]
    row: usize,
    #[serde(default)]
    column: usize,
}

fn bounded_collection_json(node: &NativeNode, key: u16) -> Option<&str> {
    node.string(key)
        .filter(|value| value.len() <= MAX_COLLECTION_JSON_BYTES)
}

/// Decode the declared columns into validated core descriptors.
///
/// `TableState::element_with` panics on a duplicate identifier or an overlong list, so both are
/// enforced here: the first occurrence of an identifier wins and everything past the core's own
/// bound is dropped.
fn declared_columns(node: &NativeNode) -> (Vec<TableColumn>, Vec<Arc<str>>) {
    let Some(source) = bounded_collection_json(node, property::COLUMNS) else {
        return (Vec::new(), Vec::new());
    };
    let Ok(declared) = serde_json::from_str::<Vec<DeclaredColumn>>(source) else {
        return (Vec::new(), Vec::new());
    };
    let mut seen = HashSet::new();
    let mut columns = Vec::new();
    let mut names = Vec::new();
    for column in declared {
        if columns.len() >= MAX_TABLE_COLUMNS {
            break;
        }
        if column.id.is_empty()
            || column.id.len() > MAX_COMPONENT_VALUE_BYTES
            || !seen.insert(column.id.clone())
        {
            continue;
        }
        let label = column.label.unwrap_or_else(|| column.id.clone());
        let mut built = TableColumn::new(
            ElementId::named(column.id.as_str()),
            bounded_label(label.as_str()),
        )
        .align(match column.align.as_deref() {
            Some("center") => TableColumnAlign::Center,
            Some("end" | "right") => TableColumnAlign::End,
            _ => TableColumnAlign::Start,
        })
        .sortable(column.sortable)
        .row_header(column.row_header);
        if let Some(minimum) = column.min_width.filter(|value| value.is_finite()) {
            built = built.minimum_width(minimum);
        }
        match column
            .width
            .filter(|value| value.is_finite() && *value > 0.0)
        {
            Some(width) => built = built.width(width),
            None => {
                if let Some(track) = column.track.as_deref()
                    && let Some(first) = native_grid_tracks(track).first()
                {
                    built = built.track(*first);
                }
            }
        }
        names.push(Arc::from(column.id.as_str()));
        columns.push(built);
    }
    (columns, names)
}

fn bounded_label(value: &str) -> Arc<str> {
    if value.len() <= quickgui::MAX_TREE_LABEL_BYTES {
        return Arc::from(value);
    }
    let mut end = quickgui::MAX_TREE_LABEL_BYTES;
    while !value.is_char_boundary(end) {
        end -= 1;
    }
    Arc::from(&value[..end])
}

/// Recover the declared identifier behind one derived core identity.
fn name_for(names: &[Arc<str>], id: ElementId) -> Option<Arc<str>> {
    names
        .iter()
        .find(|name| ElementId::named(name.as_ref()) == id)
        .cloned()
}

/// One retained virtual-table instance.
pub(super) struct NativeTableState {
    pub(super) state: TableState,
    pub(super) columns: Vec<TableColumn>,
    pub(super) names: Vec<Arc<str>>,
    /// Whether the next render must publish a complete snapshot of what the core decided.
    pending_report: bool,
    owner: u32,
    listens: bool,
    declaration: Arc<str>,
    reported: TableReport,
}

/// Everything a table reports back, compared once per pass so a settled table reports nothing.
#[derive(Clone, Debug, Default, PartialEq)]
struct TableReport {
    visible: (usize, usize),
    selection: Vec<(usize, usize)>,
    sort: Option<(Arc<str>, bool)>,
    active: Option<(usize, usize)>,
    widths: Vec<(Arc<str>, f32)>,
    order: Vec<Arc<str>>,
}

impl Default for NativeTableState {
    fn default() -> Self {
        Self {
            state: TableState::new(0),
            columns: Vec::new(),
            names: Vec::new(),
            pending_report: false,
            owner: 0,
            listens: false,
            declaration: Arc::from(""),
            reported: TableReport::default(),
        }
    }
}

/// One retained virtual-tree instance.
pub(super) struct NativeTreeState {
    pub(super) state: TreeState<Arc<str>>,
    pub(super) names: Vec<Arc<str>>,
    pub(super) disclosure: TreeDisclosurePlacement,
    /// Whether the next render must publish a complete snapshot; see [`NativeTableState`].
    pending_report: bool,
    owner: u32,
    listens: bool,
    declaration: Arc<str>,
    children_declaration: Arc<str>,
    expansion_declaration: Arc<str>,
    reported: TreeReport,
}

/// Where the binding places the core's behavior-only disclosure control inside a declared row.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(super) enum TreeDisclosurePlacement {
    #[default]
    Leading,
    Trailing,
    None,
}

#[derive(Clone, Debug, Default, PartialEq)]
struct TreeReport {
    visible: (usize, usize),
    selected: Option<Arc<str>>,
    expanded: Vec<Arc<str>>,
}

impl Default for NativeTreeState {
    fn default() -> Self {
        Self {
            state: TreeState::new(Vec::new()).expect("an empty tree source is always valid"),
            names: Vec::new(),
            disclosure: TreeDisclosurePlacement::Leading,
            pending_report: false,
            owner: 0,
            listens: false,
            declaration: Arc::from(""),
            children_declaration: Arc::from(""),
            expansion_declaration: Arc::from(""),
            reported: TreeReport::default(),
        }
    }
}

/// A per-instance accessor from the hosted view to one declared table's retained state.
fn table_accessor(key: u64) -> StateAccessor<NativeView, TableState> {
    StateAccessor::new(move |view: &mut NativeView| {
        &mut view.components.tables.entry(key).or_default().state
    })
}

/// A per-instance accessor from the hosted view to one declared tree's retained state.
fn tree_accessor(key: u64) -> StateAccessor<NativeView, TreeState<Arc<str>>> {
    StateAccessor::new(move |view: &mut NativeView| {
        &mut view.components.trees.entry(key).or_default().state
    })
}

/// Translate declared tree nodes into validated core nodes.
///
/// A node without an identifier, or one past the declared bound, is dropped rather than retained,
/// so `TreeState::new` never sees a source it would reject for a reason the caller cannot see.
fn tree_nodes(
    declared: &[DeclaredTreeNode],
    names: &mut Vec<Arc<str>>,
    seen: &mut HashSet<String>,
    depth: usize,
) -> Vec<TreeNode<Arc<str>>> {
    if depth >= quickgui::MAX_TREE_DEPTH {
        return Vec::new();
    }
    let mut nodes = Vec::new();
    for node in declared {
        if names.len() >= MAX_DECLARED_TREE_NODES {
            break;
        }
        if node.id.is_empty()
            || node.id.len() > MAX_COMPONENT_VALUE_BYTES
            || !seen.insert(node.id.clone())
        {
            continue;
        }
        let value: Arc<str> = Arc::from(node.id.as_str());
        names.push(Arc::clone(&value));
        let label = node.label.clone().unwrap_or_else(|| node.id.clone());
        let children = tree_nodes(&node.children, names, seen, depth + 1);
        let mut built = TreeNode::new(
            ElementId::named(node.id.as_str()),
            bounded_label(label.as_str()),
            value,
        )
        .disabled(node.disabled);
        if children.is_empty() {
            built = built.pending(node.pending);
        } else {
            built = built.children(children);
        }
        nodes.push(built);
    }
    nodes
}

fn declared_strings_list(node: &NativeNode, key: u16) -> Vec<Arc<str>> {
    let Some(source) = bounded_collection_json(node, key) else {
        return Vec::new();
    };
    serde_json::from_str::<Vec<String>>(source)
        .unwrap_or_default()
        .into_iter()
        .filter(|value| !value.is_empty() && value.len() <= MAX_COMPONENT_VALUE_BYTES)
        .take(MAX_DECLARED_TREE_NODES)
        .map(|value| Arc::from(value.as_str()))
        .collect()
}

impl NativeComponentStates {
    /// Reseed one declared virtual table from this frame's declaration.
    pub(super) fn sync_table(&mut self, key: u64, id: u32, node: &NativeNode) {
        let declaration = declaration_fingerprint(
            node,
            &[
                property::COLUMNS,
                property::ROW_COUNT,
                property::ROW_HEIGHT,
                property::HEADER_HEIGHT,
                property::SELECTION_MODE,
                property::SELECTION,
                property::SORT_COLUMN,
                property::SORT_DIRECTION,
                property::EDITING,
            ],
        );
        let listens = declares_change(node);
        let (columns, names) = declared_columns(node);
        let retained = self.tables.entry(key).or_default();
        retained.owner = id;
        retained.listens = listens;
        retained.columns = columns;
        retained.names = names;
        if retained.declaration == declaration {
            return;
        }
        retained.declaration = declaration;
        let row_count = node
            .number(property::ROW_COUNT)
            .filter(|count| *count >= 0.0)
            .map_or(0, |count| (count as usize).min(MAX_TABLE_ROWS));
        retained.state.set_row_count(row_count);
        retained.state.set_layout(TableLayout::new(
            node.number(property::HEADER_HEIGHT)
                .unwrap_or(DEFAULT_TABLE_HEADER_HEIGHT),
            node.number(property::ROW_HEIGHT)
                .unwrap_or(DEFAULT_TABLE_ROW_HEIGHT),
        ));
        retained
            .state
            .set_selection_mode(match node.string(property::SELECTION_MODE) {
                Some("multiple") => TableSelectionMode::Multiple,
                _ => TableSelectionMode::Single,
            });
        retained
            .state
            .set_sort(declared_sort(node, &retained.names));
        if let Some(selection) = declared_selection(node) {
            retained.state.set_selection(selection);
        }
        match declared_editing(node) {
            Some(cell) => {
                retained
                    .state
                    .begin_edit(cell, retained.columns.len().max(1));
            }
            None => {
                retained.state.end_edit();
            }
        }
        retained.reported = table_report(retained);
        retained.pending_report = true;
    }

    /// Reseed one declared virtual tree from this frame's declaration.
    pub(super) fn sync_tree(&mut self, key: u64, id: u32, node: &NativeNode) {
        let declaration =
            declaration_fingerprint(node, &[property::NODES, property::LOADING_LABEL]);
        let children_declaration: Arc<str> =
            Arc::from(node.string(property::SET_CHILDREN).unwrap_or(""));
        let expansion_declaration =
            declaration_fingerprint(node, &[property::EXPANDED, property::SELECTED_VALUE]);
        let listens = declares_change(node);
        let retained = self.trees.entry(key).or_default();
        let mut reseeded = false;
        retained.owner = id;
        retained.listens = listens;
        retained.disclosure = match node.string(property::DISCLOSURE) {
            Some("trailing") => TreeDisclosurePlacement::Trailing,
            Some("none") => TreeDisclosurePlacement::None,
            _ => TreeDisclosurePlacement::Leading,
        };
        if retained.declaration != declaration {
            retained.declaration = declaration;
            let declared = bounded_collection_json(node, property::NODES)
                .and_then(|source| serde_json::from_str::<Vec<DeclaredTreeNode>>(source).ok())
                .unwrap_or_default();
            let mut names = Vec::new();
            let mut seen = HashSet::new();
            let nodes = tree_nodes(&declared, &mut names, &mut seen, 0);
            let mut state = TreeState::new(Vec::new()).expect("an empty tree source is valid");
            if let Some(label) = node
                .string(property::LOADING_LABEL)
                .filter(|label| !label.is_empty() && label.len() <= MAX_COMPONENT_VALUE_BYTES)
            {
                state = state.with_loading_label(label);
            }
            if state.set_nodes(nodes).is_ok() {
                retained.names = names;
            }
            retained.state = state;
            // A replaced source is a new arena, so the controlled expansion and selection are
            // applied again below instead of surviving as whatever the previous arena held.
            retained.children_declaration = Arc::from("");
            retained.expansion_declaration = Arc::from("");
            reseeded = true;
        }
        retained.state.set_layout(TreeLayout::new(
            node.number(property::ROW_HEIGHT)
                .unwrap_or(DEFAULT_TREE_ROW_HEIGHT),
        ));
        if retained.children_declaration != children_declaration {
            retained.children_declaration = Arc::clone(&children_declaration);
            reseeded = true;
            // Supplying lazy children is a declaration too: the core validates and splices the
            // replacement atomically, so an over-deep or duplicate payload changes nothing.
            if let Some(declared) = bounded_collection_json(node, property::SET_CHILDREN)
                .and_then(|source| serde_json::from_str::<DeclaredChildren>(source).ok())
                .filter(|declared| !declared.id.is_empty())
            {
                let mut names = std::mem::take(&mut retained.names);
                let mut seen = names.iter().map(|name| name.to_string()).collect();
                let children = tree_nodes(&declared.children, &mut names, &mut seen, 0);
                // A rejected splice leaves the arena untouched, but the identifiers it decoded
                // stay retained so a later successful splice can still name them.
                let _ = retained
                    .state
                    .set_children(ElementId::named(declared.id.as_str()), children);
                retained.names = names;
            }
        }
        if retained.expansion_declaration != expansion_declaration {
            retained.expansion_declaration = expansion_declaration;
            reseeded = true;
            let expanded = declared_strings_list(node, property::EXPANDED);
            for name in &retained.names {
                retained.state.set_expanded(
                    ElementId::named(name.as_ref()),
                    expanded.iter().any(|value| value == name),
                );
            }
            if let Some(selected) = node
                .string(property::SELECTED_VALUE)
                .filter(|value| !value.is_empty() && value.len() <= MAX_COMPONENT_VALUE_BYTES)
            {
                retained.state.select(ElementId::named(selected));
            }
        }
        // The baseline only moves when the declaration itself moved; anything the core decided
        // since the last pass stays a difference the report below publishes.
        if reseeded {
            retained.reported = tree_report(retained);
            retained.pending_report = true;
        }
    }
}

/// Publish what one table's core state decided during the render that just built it.
///
/// Widths, display order, and the virtual window are all decided inside the core's own element
/// builder, so the report runs after it rather than at the start of the next pass.
fn publish_table(retained: &mut NativeTableState, window: u32, events: &EventQueue) {
    {
        {
            let report = table_report(retained);
            if report == retained.reported && !retained.pending_report {
                return;
            }
            let forced = std::mem::take(&mut retained.pending_report);
            let previous = std::mem::replace(&mut retained.reported, report.clone());
            let previous = if forced {
                TableReport::default()
            } else {
                previous
            };
            if !retained.listens {
                return;
            }
            let mut payload = serde_json::Map::new();
            if report.visible != previous.visible {
                payload.insert(
                    "visibleRange".to_owned(),
                    serde_json::json!({ "start": report.visible.0, "end": report.visible.1 }),
                );
            }
            if report.selection != previous.selection {
                payload.insert(
                    "selectedRanges".to_owned(),
                    serde_json::json!(
                        report
                            .selection
                            .iter()
                            .map(|(start, end)| vec![*start, *end])
                            .collect::<Vec<_>>()
                    ),
                );
            }
            if report.sort != previous.sort {
                payload.insert(
                    "sort".to_owned(),
                    match &report.sort {
                        Some((column, ascending)) => serde_json::json!({
                            "column": column.as_ref(),
                            "direction": if *ascending { "ascending" } else { "descending" },
                        }),
                        None => serde_json::Value::Null,
                    },
                );
            }
            if report.active != previous.active {
                payload.insert(
                    "activeCell".to_owned(),
                    match report.active {
                        Some((row, column)) => serde_json::json!({ "row": row, "column": column }),
                        None => serde_json::Value::Null,
                    },
                );
            }
            if report.widths != previous.widths {
                payload.insert(
                    "columnWidths".to_owned(),
                    serde_json::Value::Array(
                        report
                            .widths
                            .iter()
                            .map(|(column, width)| {
                                serde_json::json!({ "id": column.to_string(), "width": width })
                            })
                            .collect(),
                    ),
                );
            }
            if report.order != previous.order {
                payload.insert(
                    "columnOrder".to_owned(),
                    serde_json::json!(
                        report
                            .order
                            .iter()
                            .map(|column| column.to_string())
                            .collect::<Vec<_>>()
                    ),
                );
            }
            if payload.is_empty() {
                return;
            }
            enqueue_component_change(
                events,
                window,
                retained.owner,
                serde_json::Value::Object(payload),
            );
        }
    }
}

/// Publish what one tree's core state decided during the render that just built it.
fn publish_tree(retained: &mut NativeTreeState, window: u32, events: &EventQueue) {
    {
        {
            let report = tree_report(retained);
            if report == retained.reported && !retained.pending_report {
                return;
            }
            let forced = std::mem::take(&mut retained.pending_report);
            let previous = std::mem::replace(&mut retained.reported, report.clone());
            let previous = if forced {
                TreeReport::default()
            } else {
                previous
            };
            if !retained.listens {
                return;
            }
            let mut payload = serde_json::Map::new();
            if report.visible != previous.visible {
                payload.insert(
                    "visibleRange".to_owned(),
                    serde_json::json!({ "start": report.visible.0, "end": report.visible.1 }),
                );
            }
            if report.selected != previous.selected {
                payload.insert(
                    "value".to_owned(),
                    match &report.selected {
                        Some(value) => serde_json::Value::String(value.to_string()),
                        None => serde_json::Value::Null,
                    },
                );
            }
            if report.expanded != previous.expanded {
                payload.insert(
                    "expanded".to_owned(),
                    serde_json::json!(
                        report
                            .expanded
                            .iter()
                            .map(|value| value.to_string())
                            .collect::<Vec<_>>()
                    ),
                );
            }
            if payload.is_empty() {
                return;
            }
            enqueue_component_change(
                events,
                window,
                retained.owner,
                serde_json::Value::Object(payload),
            );
        }
    }
}

fn declared_sort(node: &NativeNode, names: &[Arc<str>]) -> Option<TableSort> {
    let column = node
        .string(property::SORT_COLUMN)
        .filter(|value| !value.is_empty() && value.len() <= MAX_COMPONENT_VALUE_BYTES)?;
    if !names.iter().any(|name| name.as_ref() == column) {
        return None;
    }
    Some(TableSort {
        column: ElementId::named(column),
        direction: match node.string(property::SORT_DIRECTION) {
            Some("descending" | "desc") => TableSortDirection::Descending,
            _ => TableSortDirection::Ascending,
        },
    })
}

fn declared_selection(node: &NativeNode) -> Option<TableSelection> {
    let source = bounded_collection_json(node, property::SELECTION)?;
    let ranges = serde_json::from_str::<Vec<Vec<usize>>>(source).ok()?;
    let mut selection = TableSelection::new();
    for range in ranges.into_iter().take(MAX_DECLARED_SELECTION_RANGES) {
        match range.as_slice() {
            [row] => selection.insert(*row),
            [start, end] => selection.insert_range(*start, *end),
            _ => false,
        };
    }
    Some(selection)
}

fn declared_editing(node: &NativeNode) -> Option<TableCellPosition> {
    let source = bounded_collection_json(node, property::EDITING)?;
    let cell = serde_json::from_str::<DeclaredCell>(source).ok()?;
    Some(TableCellPosition {
        row: cell.row,
        column: cell.column,
    })
}

fn table_report(retained: &NativeTableState) -> TableReport {
    let visible = retained.state.visible_rows();
    TableReport {
        visible: (visible.start, visible.end),
        selection: retained
            .state
            .selection()
            .ranges()
            .iter()
            .take(MAX_DECLARED_SELECTION_RANGES)
            .copied()
            .collect(),
        sort: retained.state.sort().and_then(|sort| {
            name_for(&retained.names, sort.column).map(|column| {
                (
                    column,
                    matches!(sort.direction, TableSortDirection::Ascending),
                )
            })
        }),
        active: retained
            .state
            .selected_cell()
            .map(|cell| (cell.row, cell.column)),
        widths: retained
            .columns
            .iter()
            .filter_map(|column| {
                let width = retained.state.column_width(column.id())?;
                name_for(&retained.names, column.id()).map(|name| (name, width))
            })
            .collect(),
        order: retained
            .state
            .column_order()
            .iter()
            .filter_map(|index| retained.names.get(usize::from(*index)).cloned())
            .collect(),
    }
}

fn tree_report(retained: &NativeTreeState) -> TreeReport {
    let visible = retained.state.visible_rows();
    TreeReport {
        visible: (visible.start, visible.end),
        selected: retained.state.selected_value().cloned(),
        expanded: retained
            .names
            .iter()
            .filter(|name| retained.state.is_expanded(ElementId::named(name.as_ref())))
            .cloned()
            .collect(),
    }
}

/// Retain a contiguous supplied window nearest the requested destination. A partial declaration
/// must not fill gaps with empty cells or drop every old row during an asynchronous range change.
fn supplied_table_range(
    rows: &HashMap<usize, Element>,
    requested: usize,
) -> std::ops::Range<usize> {
    let mut indices = rows.keys().copied().collect::<Vec<_>>();
    indices.sort_unstable();
    let mut best = 0..0;
    let mut distance = usize::MAX;
    let mut cursor = 0;
    while cursor < indices.len() {
        let start = indices[cursor];
        let mut end = start + 1;
        cursor += 1;
        while cursor < indices.len() && indices[cursor] == end {
            end += 1;
            cursor += 1;
        }
        let candidate_distance = if requested < start {
            start - requested
        } else {
            requested.saturating_sub(end - 1)
        };
        if candidate_distance < distance {
            best = start..end;
            distance = candidate_distance;
        }
    }
    best
}

/// Build one declared virtual table, mounting the core's own scroll container inside the
/// caller-owned wrapper.
///
/// Row and cell elements are built from the retained tree before the core's renderers are handed
/// over, so the renderer the core keeps holds only finished elements.
#[allow(clippy::too_many_arguments)]
pub(super) fn build_table(
    wrapper: Element,
    id: u32,
    window: u32,
    tree: &NativeTree,
    events: &EventQueue,
    states: &mut NativeElementStates<'_>,
    cx: &mut ViewContext<'_, NativeView>,
    depth: usize,
) -> Element {
    let Some(node) = tree.nodes.get(&id) else {
        return wrapper;
    };
    let key = component_key(id, node);
    let root = ElementId::new(key);
    let mut headers: HashMap<u64, Element> = HashMap::new();
    let mut cells: HashMap<(usize, usize), Element> = HashMap::new();
    // A declared row is the container its cells are laid out in: its paint — background,
    // divider, hover, and `selected` state — reaches the core row, while its children are the
    // cells the core positions itself.
    let mut row_shells: HashMap<usize, Element> = HashMap::new();
    let column_index =
        |names: &[Arc<str>], value: &str| names.iter().position(|name| name.as_ref() == value);
    let names = match states.components.tables.get(&key) {
        Some(retained) => retained.names.clone(),
        None => return wrapper,
    };
    // Walk only the declared collection subtree, so an ordinary child of a table row still builds
    // through the normal path.
    let mut pending = node.children.clone();
    let mut rows = Vec::new();
    while let Some(child_id) = pending.pop() {
        let Some(child) = tree.nodes.get(&child_id) else {
            continue;
        };
        match child.string(property::PART) {
            Some(TABLE_HEADER_PART) => {
                let Some(value) = child.string(property::PART_VALUE) else {
                    continue;
                };
                if let Some(element) =
                    build_element(child_id, window, tree, events, states, cx, depth + 1)
                {
                    headers.insert(ElementId::named(value).as_u64(), element);
                }
            }
            Some(TABLE_ROW_PART) => rows.push(child_id),
            _ => pending.extend(child.children.iter().copied()),
        }
    }
    for row_id in rows {
        let Some(row) = tree.nodes.get(&row_id) else {
            continue;
        };
        let Some(row_index) = row
            .number(property::ROW_INDEX)
            .filter(|index| *index >= 0.0 && *index < MAX_TABLE_ROWS as f32)
            .map(|index| index as usize)
        else {
            continue;
        };
        if let Some(shell) =
            build_element_shell(row_id, window, tree, events, states, cx, depth + 1)
        {
            row_shells.insert(row_index, shell);
        }
        let mut pending = row.children.clone();
        while let Some(cell_id) = pending.pop() {
            let Some(cell) = tree.nodes.get(&cell_id) else {
                continue;
            };
            if cell.string(property::PART) != Some(TABLE_CELL_PART) {
                pending.extend(cell.children.iter().copied());
                continue;
            }
            // A cell names its column by the declared identifier, or by index when a caller
            // renders positional cells.
            let Some(column) = cell
                .string(property::PART_VALUE)
                .and_then(|value| column_index(&names, value))
                .or_else(|| {
                    cell.number(property::COLUMN_INDEX)
                        .filter(|index| *index >= 0.0 && (*index as usize) < names.len())
                        .map(|index| index as usize)
                })
            else {
                continue;
            };
            if let Some(element) =
                build_element(cell_id, window, tree, events, states, cx, depth + 1)
            {
                cells.insert((row_index, column), element);
            }
        }
    }

    let activates = node.boolean(property::COMMIT_LISTENER).unwrap_or(false);
    let listens = declares_change(node);
    let activate_events = Rc::clone(events);
    let edit_events = Rc::clone(events);
    let Some(retained) = states.components.tables.get_mut(&key) else {
        return wrapper;
    };
    let supplied = supplied_table_range(
        &row_shells,
        retained.state.list_state().logical_scroll_top().item_ix,
    );
    retained
        .state
        .list_state()
        .set_available_range(Some(supplied));
    let table = retained.state.element_with_rows(
        cx,
        root,
        &retained.columns,
        table_accessor(key),
        move |row: TableRowState| row_shells.remove(&row.row).unwrap_or_else(div),
        move |state: TableHeaderState<'_>| {
            let mut header = headers
                .remove(&state.column.id().as_u64())
                .unwrap_or_else(|| div().flex_row().items_center());
            if let Some(handle) = state.resize_handle {
                header = header.child(handle);
            }
            header
        },
        move |state: TableCellState<'_>| {
            cells
                .remove(&(state.position.row, state.position.column))
                .unwrap_or_else(|| div().flex_row().items_center())
        },
        move |_view: &mut NativeView, position: TableCellPosition, cx: &mut EventContext| {
            if activates {
                enqueue_event(
                    &activate_events,
                    QueuedEvent {
                        kind: "commit",
                        window,
                        target: id,
                        value: Some(Arc::from(
                            serde_json::json!({ "row": position.row, "column": position.column })
                                .to_string()
                                .as_str(),
                        )),
                    },
                );
            }
            cx.invalidate();
        },
    );
    // Ending an inline edit is an edge, not a value: the core reports the exact cell and whether
    // the editor committed, which no retained value can express.
    let edit_ended = cx.action_listener(root, move |_view, action: &TableEditEnded, cx| {
        if listens {
            enqueue_component_change(
                &edit_events,
                window,
                id,
                serde_json::json!({
                    "editEnded": {
                        "row": action.position.row,
                        "column": action.position.column,
                        "committed": action.committed,
                    }
                }),
            );
        }
        cx.invalidate();
    });
    // Request the destination plus bounded prefetch while the core continues presenting supplied
    // rows. The frontend's next mutation batch replaces the window without an empty-cell frame.
    if let Some(retained) = states.components.tables.get_mut(&key) {
        publish_table(retained, window, events);
    }
    wrapper.child(table.on_action(edit_ended))
}

/// Build one declared virtual tree, mounting the core's own scroll container inside the
/// caller-owned wrapper.
#[allow(clippy::too_many_arguments)]
pub(super) fn build_tree(
    wrapper: Element,
    id: u32,
    window: u32,
    tree: &NativeTree,
    events: &EventQueue,
    states: &mut NativeElementStates<'_>,
    cx: &mut ViewContext<'_, NativeView>,
    depth: usize,
) -> Element {
    let Some(node) = tree.nodes.get(&id) else {
        return wrapper;
    };
    let key = component_key(id, node);
    let root = ElementId::new(key);
    let mut declared_rows: HashMap<u64, Element> = HashMap::new();
    let mut pending = node.children.clone();
    while let Some(child_id) = pending.pop() {
        let Some(child) = tree.nodes.get(&child_id) else {
            continue;
        };
        if child.string(property::PART) != Some(TREE_ROW_PART) {
            pending.extend(child.children.iter().copied());
            continue;
        }
        let Some(value) = child.string(property::PART_VALUE) else {
            continue;
        };
        if let Some(element) = build_element(child_id, window, tree, events, states, cx, depth + 1)
        {
            declared_rows.insert(ElementId::named(value).as_u64(), element);
        }
    }

    let activates = node.boolean(property::COMMIT_LISTENER).unwrap_or(false);
    let listens = declares_change(node);
    let activate_events = Rc::clone(events);
    let load_events = Rc::clone(events);
    let Some(retained) = states.components.trees.get_mut(&key) else {
        return wrapper;
    };
    let placement = retained.disclosure;
    let names = retained.names.clone();
    let activate_names = names.clone();
    let element = retained.state.element_with(
        cx,
        root,
        tree_accessor(key),
        move |row: TreeRow<'_, Arc<str>>, disclosure: Option<Element>| {
            let declared = declared_rows
                .remove(&row.id().as_u64())
                .unwrap_or_else(|| div().flex_row().items_center());
            match (disclosure, placement) {
                (Some(disclosure), TreeDisclosurePlacement::Leading) => div()
                    .flex_row()
                    .items_center()
                    .w_full()
                    .min_w(0.0)
                    .child(disclosure)
                    .child(declared),
                (Some(disclosure), TreeDisclosurePlacement::Trailing) => div()
                    .flex_row()
                    .items_center()
                    .w_full()
                    .min_w(0.0)
                    .child(declared)
                    .child(disclosure),
                _ => declared,
            }
        },
        move |_view: &mut NativeView, node_id: ElementId, cx: &mut EventContext| {
            if activates && let Some(value) = name_for(&activate_names, node_id) {
                enqueue_event(
                    &activate_events,
                    QueuedEvent {
                        kind: "commit",
                        window,
                        target: id,
                        value: Some(Arc::from(
                            serde_json::json!({ "value": value.as_ref() })
                                .to_string()
                                .as_str(),
                        )),
                    },
                );
            }
            cx.invalidate();
        },
    );
    // A pending branch asks for its children exactly once, through the core's own typed action.
    let load = cx.action_listener(root, move |_view, action: &TreeLoadChildren, cx| {
        if listens && let Some(value) = name_for(&names, action.node) {
            enqueue_component_change(
                &load_events,
                window,
                id,
                serde_json::json!({ "loadChildren": value.as_ref() }),
            );
        }
        cx.invalidate();
    });
    if let Some(retained) = states.components.trees.get_mut(&key) {
        publish_tree(retained, window, events);
    }
    wrapper.child(element.on_action(load))
}
