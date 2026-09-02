//! Declared select, constrained combobox, and free-form autocomplete bound to the Rust core.
//!
//! These three components own a native popover window of their own. The rows in that window are
//! rendered by the core from a bounded `appearance` declaration exactly the way
//! [`super::popover_menu`] renders declared menu rows: the row renderer the core retains is a
//! `'static` closure, so it can never reach back into the hosted JavaScript tree while the core is
//! deciding what a keystroke means.
//!
//! Everything else is a declaration too. JavaScript declares the option source, the controlled
//! value, the controlled input text, and the filter mode ahead of time; the core owns filtering,
//! highlight movement, typeahead, native surface placement and lifetime, accessibility semantics,
//! and commit policy; and every result the core decides travels back as one asynchronous
//! `componentchange` or `commit` event.

use super::*;
use serde::Deserialize;

/// Declared part name of a select trigger.
pub(super) const SELECT_PART: &str = "select";
/// Declared part name of a constrained combobox input.
pub(super) const COMBOBOX_PART: &str = "combobox";
/// Declared part name of a free-form autocomplete input.
pub(super) const AUTOCOMPLETE_PART: &str = "autocomplete";
/// Declared part name of one child option node.
pub(super) const OPTION_PART: &str = "option";

/// Longest bounded option declaration accepted from one `options` property.
pub(super) const MAX_OPTIONS_JSON_BYTES: usize = 512 * 1024;
/// Most declared options retained by one picker instance.
pub(super) const MAX_DECLARED_OPTIONS: usize = 4_096;

const DEFAULT_PICKER_WIDTH: f32 = 240.0;
const DEFAULT_PICKER_ROW_HEIGHT: f32 = 28.0;
const DEFAULT_PICKER_VISIBLE_ROWS: usize = 8;
const DEFAULT_PICKER_ANCHOR_GAP: f32 = 4.0;
const DEFAULT_PICKER_FONT_SIZE: f32 = 13.0;
const DEFAULT_PICKER_RADIUS: f32 = 6.0;
const DEFAULT_PICKER_PADDING: f32 = 10.0;
const DEFAULT_PICKER_VERTICAL_PADDING: f32 = 4.0;
const DISABLED_ROW_OPACITY: f32 = 0.4;

/// One declared option. Absent fields fall back to the core's own defaults.
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DeclaredOption {
    #[serde(default)]
    value: String,
    #[serde(default)]
    label: Option<String>,
    #[serde(default)]
    detail: Option<String>,
    #[serde(default)]
    group: Option<String>,
    #[serde(default)]
    keywords: Option<String>,
    #[serde(default)]
    disabled: bool,
}

/// Structural geometry and paint for the rows the core renders in its own window.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct NativePickerStyle {
    width: f32,
    row_height: f32,
    max_visible_rows: usize,
    anchor_gap: f32,
    font_size: f32,
    radius: f32,
    padding: f32,
    vertical_padding: f32,
    background: Option<Color>,
    color: Option<Color>,
    highlight_background: Option<Color>,
    highlight_color: Option<Color>,
    selected_background: Option<Color>,
    muted_color: Option<Color>,
}

impl Default for NativePickerStyle {
    fn default() -> Self {
        Self {
            width: DEFAULT_PICKER_WIDTH,
            row_height: DEFAULT_PICKER_ROW_HEIGHT,
            max_visible_rows: DEFAULT_PICKER_VISIBLE_ROWS,
            anchor_gap: DEFAULT_PICKER_ANCHOR_GAP,
            font_size: DEFAULT_PICKER_FONT_SIZE,
            radius: DEFAULT_PICKER_RADIUS,
            padding: DEFAULT_PICKER_PADDING,
            vertical_padding: DEFAULT_PICKER_VERTICAL_PADDING,
            background: None,
            color: None,
            highlight_background: None,
            highlight_color: None,
            selected_background: None,
            muted_color: None,
        }
    }
}

/// One bounded appearance declaration shared by all three pickers.
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
struct NativePickerAppearance {
    width: Option<f32>,
    row_height: Option<f32>,
    max_visible_rows: Option<f32>,
    anchor_gap: Option<f32>,
    font_size: Option<f32>,
    radius: Option<f32>,
    padding: Option<f32>,
    vertical_padding: Option<f32>,
    background: Option<u32>,
    color: Option<u32>,
    highlight_background: Option<u32>,
    highlight_color: Option<u32>,
    selected_background: Option<u32>,
    muted_color: Option<u32>,
}

fn positive(value: Option<f32>, fallback: f32) -> f32 {
    match value {
        Some(value) if value.is_finite() && value > 0.0 => value,
        _ => fallback,
    }
}

fn non_negative(value: Option<f32>, fallback: f32) -> f32 {
    match value {
        Some(value) if value.is_finite() && value >= 0.0 => value,
        _ => fallback,
    }
}

/// Decode the declared appearance. Malformed or oversized JSON falls back to the defaults.
fn picker_style(node: &NativeNode) -> NativePickerStyle {
    let default = NativePickerStyle::default();
    let Some(source) = node
        .string(property::APPEARANCE)
        .filter(|value| value.len() <= MAX_COMPONENT_JSON_BYTES)
    else {
        return default;
    };
    let Ok(declared) = serde_json::from_str::<NativePickerAppearance>(source) else {
        return default;
    };
    NativePickerStyle {
        width: positive(declared.width, default.width),
        row_height: positive(declared.row_height, default.row_height),
        max_visible_rows: declared
            .max_visible_rows
            .filter(|rows| rows.is_finite() && *rows >= 1.0)
            .map_or(default.max_visible_rows, |rows| {
                (rows as usize).min(quickgui::MAX_SELECT_VISIBLE_ROWS)
            }),
        anchor_gap: non_negative(declared.anchor_gap, default.anchor_gap),
        font_size: positive(declared.font_size, default.font_size),
        radius: non_negative(declared.radius, default.radius),
        padding: non_negative(declared.padding, default.padding),
        vertical_padding: non_negative(declared.vertical_padding, default.vertical_padding),
        background: declared.background.map(unpack_color),
        color: declared.color.map(unpack_color),
        highlight_background: declared.highlight_background.map(unpack_color),
        highlight_color: declared.highlight_color.map(unpack_color),
        selected_background: declared.selected_background.map(unpack_color),
        muted_color: declared.muted_color.map(unpack_color),
    }
}

impl NativePickerStyle {
    fn select_layout(self) -> SelectPopoverLayout {
        SelectPopoverLayout::new(self.width, self.row_height)
            .max_visible_rows(self.max_visible_rows)
            .anchor_gap(self.anchor_gap)
    }

    fn suggestion_layout(self) -> AutocompletePopoverLayout {
        AutocompletePopoverLayout::new(self.width, self.row_height)
            .max_visible_rows(self.max_visible_rows)
            .anchor_gap(self.anchor_gap)
    }

    /// The appearance-only popover container the core mounts its rows in.
    fn surface(self) -> Element {
        let mut root = div().flex_col().py(self.vertical_padding);
        if let Some(background) = self.background {
            root = root.bg(background);
        }
        if let Some(color) = self.color {
            root = root.text_color(color);
        }
        root.rounded(self.radius)
    }

    /// One appearance-only option row.
    ///
    /// The core wraps whatever this returns with the exact option semantics, identity, and
    /// active-descendant relationship, so this function contributes no behavior at all.
    fn row(
        self,
        item: &PickerItem<Arc<str>>,
        active: bool,
        selected: bool,
        disabled: bool,
    ) -> Element {
        let mut row = div()
            .w_full()
            .flex_none()
            .flex_row()
            .items_center()
            .justify_between()
            .gap(12.0)
            .px(self.padding)
            .rounded(self.radius.min(self.row_height / 2.0));
        if let Some(color) = self.color {
            row = row.text_color(color);
        }
        if selected
            && !active
            && let Some(background) = self.selected_background
        {
            row = row.bg(background);
        }
        if active && !disabled {
            if let Some(background) = self.highlight_background {
                row = row.bg(background);
            }
            if let Some(color) = self.highlight_color {
                row = row.text_color(color);
            }
        }
        if disabled {
            row = row.opacity(DISABLED_ROW_OPACITY);
        }
        row = row.child(
            text(item.label().clone())
                .text_size(self.font_size)
                .truncate(),
        );
        if let Some(detail) = item.detail_text() {
            let mut hint = text(Arc::clone(detail))
                .text_size(self.font_size)
                .whitespace_nowrap();
            if let Some(color) = self.muted_color {
                hint = hint.text_color(color);
            }
            row = row.child(hint);
        }
        row
    }
}

/// Decode the declared option source into validated core picker items.
///
/// A declaration may carry the options as one bounded JSON array or as child `option` nodes. An
/// empty value, an oversized declaration, or a duplicate value is dropped rather than retained, so
/// the core never sees an item list it would reject.
fn declared_options(node: &NativeNode, tree: &NativeTree) -> Vec<PickerItem<Arc<str>>> {
    let declared = match node
        .string(property::OPTIONS)
        .filter(|value| value.len() <= MAX_OPTIONS_JSON_BYTES)
    {
        Some(source) => serde_json::from_str::<Vec<DeclaredOption>>(source).unwrap_or_default(),
        None => child_options(node, tree),
    };
    let mut seen = HashSet::new();
    declared
        .into_iter()
        .filter(|option| {
            !option.value.is_empty() && option.value.len() <= MAX_COMPONENT_VALUE_BYTES
        })
        .filter(|option| seen.insert(option.value.clone()))
        .take(MAX_DECLARED_OPTIONS)
        .map(|option| {
            let label = option.label.unwrap_or_else(|| option.value.clone());
            let mut item = PickerItem::new(
                bounded_option_text(&label),
                Arc::<str>::from(option.value.as_str()),
            )
            .id(ElementId::named(option.value.as_str()))
            .disabled(option.disabled);
            if let Some(detail) = option.detail.as_deref().filter(|detail| !detail.is_empty()) {
                item = item.detail(bounded_option_text(detail));
            }
            // A declared group is searchable metadata, not a separate row: the core's picker model
            // has no group rows, so the group name joins the item's own search keywords.
            let keywords = match (option.keywords.as_deref(), option.group.as_deref()) {
                (Some(keywords), Some(group)) => Some(format!("{keywords} {group}")),
                (Some(keywords), None) => Some(keywords.to_owned()),
                (None, Some(group)) => Some(group.to_owned()),
                (None, None) => None,
            };
            if let Some(keywords) = keywords.as_deref().filter(|value| !value.is_empty()) {
                item = item.keywords(bounded_option_text(keywords));
            }
            item
        })
        .collect()
}

fn bounded_option_text(value: &str) -> Arc<str> {
    if value.len() <= quickgui::MAX_PICKER_ITEM_TEXT_BYTES {
        return Arc::from(value);
    }
    let mut end = quickgui::MAX_PICKER_ITEM_TEXT_BYTES;
    while !value.is_char_boundary(end) {
        end -= 1;
    }
    Arc::from(&value[..end])
}

/// Collect the declared child `option` nodes of one picker, in declaration order.
fn child_options(node: &NativeNode, tree: &NativeTree) -> Vec<DeclaredOption> {
    let mut options = Vec::new();
    let mut pending = node.children.iter().copied().rev().collect::<Vec<_>>();
    while let Some(id) = pending.pop() {
        if options.len() >= MAX_DECLARED_OPTIONS {
            break;
        }
        let Some(child) = tree.nodes.get(&id) else {
            continue;
        };
        if child.string(property::PART) == Some(OPTION_PART) {
            options.push(DeclaredOption {
                value: child.string(property::PART_VALUE).unwrap_or("").to_owned(),
                label: child.string(property::VALUE).map(str::to_owned),
                detail: child.string(property::VALUE_TEXT).map(str::to_owned),
                group: child.string(property::GROUP).map(str::to_owned),
                keywords: None,
                disabled: child.boolean(property::DISABLED).unwrap_or(false),
            });
            continue;
        }
        pending.extend(child.children.iter().copied().rev());
    }
    options
}

/// The core offers exactly two filter modes: its own bounded fuzzy matcher, or none at all for a
/// source an application or service already filtered. An unknown declaration keeps the default.
fn declared_filter_mode(node: &NativeNode) -> PickerFilterMode {
    match node.string(property::FILTER_MODE) {
        Some("none" | "manual" | "server") => PickerFilterMode::None,
        _ => PickerFilterMode::Fuzzy,
    }
}

fn declared_label(node: &NativeNode) -> Arc<str> {
    Arc::from(
        node.string(property::ACCESSIBILITY_LABEL)
            .filter(|label| label.len() <= MAX_COMPONENT_VALUE_BYTES)
            .unwrap_or(""),
    )
}

fn declared_text(node: &NativeNode, key: u16) -> Option<Arc<str>> {
    node.string(key)
        .filter(|value| !value.is_empty() && value.len() <= MAX_COMPONENT_VALUE_BYTES)
        .map(Arc::from)
}

/// A fingerprint of everything a picker's retained core state is built from.
///
/// The core's replacement mutators all need an `EventContext` because they close a live native
/// popover, and a render pass has none. The binding therefore rebuilds a picker's state whenever
/// this fingerprint changes and the popover is closed, which is exactly when a rebuild is free.
fn picker_declaration(node: &NativeNode) -> Arc<str> {
    let mut fingerprint = String::new();
    for key in [
        property::OPTIONS,
        property::APPEARANCE,
        property::FILTER_MODE,
        property::ACTIVE_VALUE,
        property::INPUT_VALUE,
    ] {
        fingerprint.push('\u{1f}');
        fingerprint.push_str(node.string(key).unwrap_or(""));
    }
    Arc::from(fingerprint.as_str())
}

/// One retained select instance.
pub(super) struct NativeSelectState {
    pub(super) state: SelectState<Arc<str>>,
    owner: u32,
    listens: bool,
    declaration: Arc<str>,
    reported: Option<Arc<str>>,
    reported_open: bool,
}

impl Default for NativeSelectState {
    fn default() -> Self {
        Self {
            state: SelectState::new(Vec::new()).expect("an empty select source is always valid"),
            owner: 0,
            listens: false,
            declaration: Arc::from(""),
            reported: None,
            reported_open: false,
        }
    }
}

/// One retained constrained-combobox instance.
pub(super) struct NativeComboboxState {
    pub(super) state: ComboboxState<Arc<str>>,
    owner: u32,
    listens: bool,
    declaration: Arc<str>,
    reported: Option<Arc<str>>,
    reported_input: Arc<str>,
    reported_open: bool,
}

impl Default for NativeComboboxState {
    fn default() -> Self {
        Self {
            state: ComboboxState::new(Vec::new())
                .expect("an empty combobox source is always valid"),
            owner: 0,
            listens: false,
            declaration: Arc::from(""),
            reported: None,
            reported_input: Arc::from(""),
            reported_open: false,
        }
    }
}

/// One retained free-form autocomplete instance.
pub(super) struct NativeAutocompleteState {
    pub(super) state: AutocompleteState<Arc<str>>,
    owner: u32,
    listens: bool,
    declaration: Arc<str>,
    reported_input: Arc<str>,
    reported_open: bool,
}

impl Default for NativeAutocompleteState {
    fn default() -> Self {
        Self {
            state: AutocompleteState::new(Vec::new())
                .expect("an empty autocomplete source is always valid"),
            owner: 0,
            listens: false,
            declaration: Arc::from(""),
            reported_input: Arc::from(""),
            reported_open: false,
        }
    }
}

/// A per-instance accessor from the hosted view to one declared select's retained state.
fn select_accessor(key: u64) -> StateAccessor<NativeView, SelectState<Arc<str>>> {
    StateAccessor::new(move |view: &mut NativeView| {
        &mut view.components.selects.entry(key).or_default().state
    })
}

/// A per-instance accessor from the hosted view to one declared combobox's retained state.
fn combobox_accessor(key: u64) -> StateAccessor<NativeView, ComboboxState<Arc<str>>> {
    StateAccessor::new(move |view: &mut NativeView| {
        &mut view.components.comboboxes.entry(key).or_default().state
    })
}

/// A per-instance accessor from the hosted view to one declared autocomplete's retained state.
fn autocomplete_accessor(key: u64) -> StateAccessor<NativeView, AutocompleteState<Arc<str>>> {
    StateAccessor::new(move |view: &mut NativeView| {
        &mut view.components.autocompletes.entry(key).or_default().state
    })
}

impl NativeComponentStates {
    /// Reseed one declared select from this frame's declaration.
    pub(super) fn sync_select(&mut self, key: u64, id: u32, node: &NativeNode, tree: &NativeTree) {
        let declaration = picker_declaration(node);
        let listens = declares_change(node);
        let retained = self.selects.entry(key).or_default();
        retained.owner = id;
        retained.listens = listens;
        // Rebuilding while a native popover is open would orphan that window, so an in-flight
        // declaration waits for the core to close it; the close itself schedules another frame.
        if retained.declaration == declaration || retained.state.is_open() {
            return;
        }
        let style = picker_style(node);
        let mut state = SelectState::new(declared_options(node, tree))
            .unwrap_or_else(|_| {
                SelectState::new(Vec::new()).expect("an empty select source is always valid")
            })
            .with_layout(style.select_layout());
        let value = declared_text(node, property::ACTIVE_VALUE);
        if let Some(value) = &value {
            state.select_id(ElementId::named(value.as_ref()));
        }
        retained.state = state;
        retained.declaration = declaration;
        retained.reported = retained.state.selected_value().cloned();
        retained.reported_open = false;
        let _ = value;
    }

    /// Reseed one declared constrained combobox from this frame's declaration.
    pub(super) fn sync_combobox(
        &mut self,
        key: u64,
        id: u32,
        node: &NativeNode,
        tree: &NativeTree,
    ) {
        let declaration = picker_declaration(node);
        let listens = declares_change(node);
        let retained = self.comboboxes.entry(key).or_default();
        retained.owner = id;
        retained.listens = listens;
        if retained.declaration == declaration || retained.state.is_open() {
            return;
        }
        let style = picker_style(node);
        let mut state = ComboboxState::new(declared_options(node, tree))
            .unwrap_or_else(|_| {
                ComboboxState::new(Vec::new()).expect("an empty combobox source is always valid")
            })
            .with_layout(style.suggestion_layout())
            .with_filter_mode(declared_filter_mode(node));
        if let Some(value) = declared_text(node, property::ACTIVE_VALUE) {
            state = state.with_selected_id(ElementId::named(value.as_ref()));
        }
        retained.state = state;
        retained.declaration = declaration;
        retained.reported = retained.state.selected_value().cloned();
        retained.reported_input = retained.state.input_value().clone();
        retained.reported_open = false;
    }

    /// Reseed one declared free-form autocomplete from this frame's declaration.
    pub(super) fn sync_autocomplete(
        &mut self,
        key: u64,
        id: u32,
        node: &NativeNode,
        tree: &NativeTree,
    ) {
        let declaration = picker_declaration(node);
        let listens = declares_change(node);
        let retained = self.autocompletes.entry(key).or_default();
        retained.owner = id;
        retained.listens = listens;
        if retained.declaration == declaration || retained.state.is_open() {
            return;
        }
        let style = picker_style(node);
        let state = AutocompleteState::new(declared_options(node, tree))
            .unwrap_or_else(|_| {
                AutocompleteState::new(Vec::new())
                    .expect("an empty autocomplete source is always valid")
            })
            .with_layout(style.suggestion_layout())
            .with_filter_mode(declared_filter_mode(node))
            .with_value(node.string(property::INPUT_VALUE).unwrap_or(""));
        retained.state = state;
        retained.declaration = declaration;
        retained.reported_input = retained.state.value().clone();
        retained.reported_open = false;
    }

    /// Enqueue one asynchronous change event per picker the core moved since the last pass.
    pub(super) fn report_pickers(&mut self, window: u32, events: &EventQueue) {
        for retained in self.selects.values_mut() {
            let value = retained.state.selected_value().cloned();
            let open = retained.state.is_open();
            if value == retained.reported && open == retained.reported_open {
                continue;
            }
            let mut payload = serde_json::Map::new();
            if value != retained.reported {
                payload.insert("value".to_owned(), optional_text(value.as_deref()));
            }
            payload.insert("open".to_owned(), serde_json::Value::Bool(open));
            retained.reported.clone_from(&value);
            retained.reported_open = open;
            if retained.listens {
                enqueue_component_change(
                    events,
                    window,
                    retained.owner,
                    serde_json::Value::Object(payload),
                );
            }
        }
        for retained in self.comboboxes.values_mut() {
            let value = retained.state.selected_value().cloned();
            let input = retained.state.input_value().clone();
            let open = retained.state.is_open();
            if value == retained.reported
                && input == retained.reported_input
                && open == retained.reported_open
            {
                continue;
            }
            let mut payload = serde_json::Map::new();
            if value != retained.reported {
                payload.insert("value".to_owned(), optional_text(value.as_deref()));
            }
            if input != retained.reported_input {
                payload.insert(
                    "inputValue".to_owned(),
                    serde_json::Value::String(input.to_string()),
                );
            }
            payload.insert("open".to_owned(), serde_json::Value::Bool(open));
            retained.reported.clone_from(&value);
            retained.reported_input = input;
            retained.reported_open = open;
            if retained.listens {
                enqueue_component_change(
                    events,
                    window,
                    retained.owner,
                    serde_json::Value::Object(payload),
                );
            }
        }
        for retained in self.autocompletes.values_mut() {
            let input = retained.state.value().clone();
            let open = retained.state.is_open();
            if input == retained.reported_input && open == retained.reported_open {
                continue;
            }
            let mut payload = serde_json::Map::new();
            if input != retained.reported_input {
                payload.insert(
                    "inputValue".to_owned(),
                    serde_json::Value::String(input.to_string()),
                );
            }
            payload.insert("open".to_owned(), serde_json::Value::Bool(open));
            retained.reported_input = input;
            retained.reported_open = open;
            if retained.listens {
                enqueue_component_change(
                    events,
                    window,
                    retained.owner,
                    serde_json::Value::Object(payload),
                );
            }
        }
    }
}

fn optional_text(value: Option<&str>) -> serde_json::Value {
    match value {
        Some(value) => serde_json::Value::String(value.to_owned()),
        None => serde_json::Value::Null,
    }
}

/// Publish one committed picker value as its own asynchronous `commit` event.
///
/// Commit is an edge, not a value: committing the same option twice reports twice while the
/// controlled value never moves, so it cannot travel on the `componentchange` value channel.
fn enqueue_commit(events: &EventQueue, window: u32, target: u32, value: serde_json::Value) {
    enqueue_event(
        events,
        QueuedEvent {
            kind: "commit",
            window,
            target,
            value: Some(Arc::from(value.to_string().as_str())),
        },
    );
}

/// Apply one declared picker part.
///
/// The trigger or input element the caller declared is handed straight to the core, which returns
/// it decorated with the complete interaction. Nothing here filters, highlights, or commits.
#[allow(clippy::too_many_arguments)]
pub(super) fn apply_picker_part(
    element: Element,
    part: &str,
    id: u32,
    window: u32,
    node: &NativeNode,
    events: &EventQueue,
    components: &mut NativeComponentStates,
    cx: &mut ViewContext<'_, NativeView>,
    listeners_enabled: bool,
) -> Option<Element> {
    // Every other declared part reaches this function too, so the untouched path leaves before it
    // hashes a scope, decodes an appearance, or allocates a label.
    if !matches!(
        part,
        SELECT_PART | COMBOBOX_PART | AUTOCOMPLETE_PART | OPTION_PART
    ) {
        return Some(element);
    }
    let key = component_key(id, node);
    let root = ElementId::new(key);
    let style = picker_style(node);
    let label = declared_label(node);
    let commits = node.boolean(property::COMMIT_LISTENER).unwrap_or(false);
    match part {
        SELECT_PART => {
            let Some(retained) = components.selects.get(&key) else {
                return Some(element);
            };
            if !listeners_enabled {
                return Some(retained.state.trigger_part(root, label, element));
            }
            let surface = move |_list: SelectListState| style.surface();
            let option = move |item: &PickerItem<Arc<str>>, state: SelectOptionState| {
                style
                    .row(item, state.active, state.selected, state.disabled)
                    .h(style.row_height)
            };
            let commit_events = Rc::clone(events);
            Some(retained.state.element_with(
                cx,
                root,
                label,
                select_accessor(key),
                element,
                surface,
                option,
                // The core has already committed the value into the retained state; the binding
                // only publishes the edge and asks for one more frame so the report pass runs.
                move |_view: &mut NativeView, value: Arc<str>, cx: &mut EventContext| {
                    if commits {
                        enqueue_commit(
                            &commit_events,
                            window,
                            id,
                            serde_json::json!({ "value": value.as_ref() }),
                        );
                    }
                    cx.invalidate();
                },
            ))
        }
        COMBOBOX_PART => {
            let Some(retained) = components.comboboxes.get_mut(&key) else {
                return Some(element);
            };
            if !listeners_enabled {
                return Some(element.id(root));
            }
            let surface = move |_list: ComboboxListState| style.surface();
            let option = move |item: &PickerItem<Arc<str>>, state: ComboboxOptionState| {
                style
                    .row(item, state.active, state.selected, state.disabled)
                    .h(style.row_height)
            };
            let commit_events = Rc::clone(events);
            Some(retained.state.element_with(
                cx,
                root,
                label,
                combobox_accessor(key),
                element,
                surface,
                option,
                move |_view: &mut NativeView, _query: Arc<str>, cx: &mut EventContext| {
                    cx.invalidate();
                },
                move |_view: &mut NativeView, value: Arc<str>, cx: &mut EventContext| {
                    if commits {
                        enqueue_commit(
                            &commit_events,
                            window,
                            id,
                            serde_json::json!({ "value": value.as_ref() }),
                        );
                    }
                    cx.invalidate();
                },
            ))
        }
        AUTOCOMPLETE_PART => {
            let Some(retained) = components.autocompletes.get_mut(&key) else {
                return Some(element);
            };
            if !listeners_enabled {
                return Some(element.id(root));
            }
            let surface = move |_list: AutocompleteListState| style.surface();
            let option = move |item: &PickerItem<Arc<str>>, state: AutocompleteOptionState| {
                style
                    .row(item, state.active, false, state.disabled)
                    .h(style.row_height)
            };
            let commit_events = Rc::clone(events);
            Some(retained.state.element_with(
                cx,
                root,
                label,
                autocomplete_accessor(key),
                element,
                surface,
                option,
                move |_view: &mut NativeView, _value: Arc<str>, cx: &mut EventContext| {
                    cx.invalidate();
                },
                move |_view: &mut NativeView, value: Arc<str>, cx: &mut EventContext| {
                    if commits {
                        enqueue_commit(
                            &commit_events,
                            window,
                            id,
                            serde_json::json!({ "value": value.as_ref() }),
                        );
                    }
                    cx.invalidate();
                },
            ))
        }
        // An option node is a declaration, not a rendered element: the core paints every row in
        // its own window from the declared appearance.
        OPTION_PART => None,
        _ => Some(element),
    }
}

/// The core-derived identity one declared picker part mounts under.
pub(super) fn picker_part_element_id(part: &str, id: u32, node: &NativeNode) -> Option<ElementId> {
    match part {
        SELECT_PART | COMBOBOX_PART | AUTOCOMPLETE_PART => {
            Some(ElementId::new(component_key(id, node)))
        }
        _ => None,
    }
}
