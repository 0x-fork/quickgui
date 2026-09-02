//! Declared range, ordering, and roving-focus components bound to the Rust core.
//!
//! These components retain interaction state — the active slider thumb, conserved splitter pane
//! sizes, a toolbar's single roving Tab stop, a toggle group's pressed values. One hosted
//! `NativeView` renders every declared instance in its window, so each one reaches its retained
//! state through a per-instance [`quickgui::StateAccessor`] instead of a non-capturing accessor
//! that could only ever name one field.
//!
//! The declaration is the source of truth: JavaScript declares bounds, values, step, ordered
//! items, and pressed values ahead of time; the core owns clamping, snapping, thumb ordering,
//! size conservation, wrapping navigation, and disabled skipping; and every result the core
//! decides travels back as one asynchronous `componentchange` event. Nothing here re-implements a
//! component — the binding only translates the declaration and reports the outcome.

use super::*;
use serde::Deserialize;

/// Declared part name of a slider root.
pub(super) const SLIDER_PART: &str = "slider";
/// Declared part name of a slider track.
pub(super) const SLIDER_TRACK_PART: &str = "slider-track";
/// Declared part name of a slider range fill.
pub(super) const SLIDER_RANGE_PART: &str = "slider-range";
/// Declared part name of one slider thumb.
pub(super) const SLIDER_THUMB_PART: &str = "slider-thumb";
/// Declared part name of a splitter root.
pub(super) const SPLITTER_PART: &str = "splitter";
/// Declared part name of one splitter pane.
pub(super) const SPLITTER_PANE_PART: &str = "splitter-pane";
/// Declared part name of one splitter handle.
pub(super) const SPLITTER_HANDLE_PART: &str = "splitter-handle";
/// Declared part name of a toolbar root.
pub(super) const TOOLBAR_PART: &str = "toolbar";
/// Declared part name of one toolbar item.
pub(super) const TOOLBAR_ITEM_PART: &str = "toolbar-item";
/// Declared part name of a toggle-group root.
pub(super) const TOGGLE_GROUP_PART: &str = "toggle-group";
/// Declared part name of one toggle-group item.
pub(super) const TOGGLE_GROUP_ITEM_PART: &str = "toggle-group-item";

/// Longest bounded declaration accepted from one `values` or `items` property.
pub(super) const MAX_COMPONENT_JSON_BYTES: usize = 64 * 1024;
/// Most component instances of one kind retained by a single window.
pub(super) const MAX_COMPONENT_INSTANCES: usize = 1_024;
/// Most declared numbers accepted from one `values` property.
pub(super) const MAX_COMPONENT_VALUES: usize = 64;
/// Most declared entries accepted from one `items` property.
pub(super) const MAX_COMPONENT_ITEMS: usize = 256;

const DEFAULT_SLIDER_MAXIMUM: f32 = 100.0;
const DEFAULT_SPLITTER_SIZE: f32 = 100.0;

/// One declared toolbar or toggle-group entry.
#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DeclaredItem {
    #[serde(default)]
    value: String,
    #[serde(default)]
    disabled: bool,
}

/// One declared splitter pane constraint.
#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DeclaredPane {
    #[serde(default)]
    min: Option<f32>,
    #[serde(default)]
    collapsible: bool,
}

pub(super) fn bounded_json(node: &NativeNode, key: u16) -> Option<&str> {
    node.string(key)
        .filter(|value| value.len() <= MAX_COMPONENT_JSON_BYTES)
}

/// Decode a bounded numeric declaration. A malformed declaration yields no numbers, never a panic.
pub(super) fn declared_numbers(node: &NativeNode, key: u16) -> Vec<f64> {
    let Some(source) = bounded_json(node, key) else {
        return Vec::new();
    };
    let Ok(values) = serde_json::from_str::<Vec<f64>>(source) else {
        return Vec::new();
    };
    values
        .into_iter()
        .filter(|value| value.is_finite())
        .take(MAX_COMPONENT_VALUES)
        .collect()
}

/// Decode a bounded string declaration. A malformed declaration yields no values, never a panic.
fn declared_strings(node: &NativeNode, key: u16) -> Vec<String> {
    let Some(source) = bounded_json(node, key) else {
        return Vec::new();
    };
    let Ok(values) = serde_json::from_str::<Vec<String>>(source) else {
        return Vec::new();
    };
    values
        .into_iter()
        .filter(|value| !value.is_empty() && value.len() <= MAX_COMPONENT_VALUE_BYTES)
        .take(MAX_COMPONENT_ITEMS)
        .collect()
}

/// Decode a bounded, de-duplicated ordered item declaration.
///
/// `Toolbar::new` and `ToggleGroup::new` reject duplicate values and overflow with a panic, so
/// the binding enforces both here: the first occurrence of a value wins and everything past
/// `limit` is dropped. A malformed declaration yields no items, never a panic.
fn declared_items(node: &NativeNode, limit: usize) -> Vec<DeclaredItem> {
    let Some(source) = bounded_json(node, property::ITEMS) else {
        return Vec::new();
    };
    let Ok(items) = serde_json::from_str::<Vec<DeclaredItem>>(source) else {
        return Vec::new();
    };
    let mut seen = HashSet::new();
    items
        .into_iter()
        .filter(|item| !item.value.is_empty() && item.value.len() <= MAX_COMPONENT_VALUE_BYTES)
        .filter(|item| seen.insert(item.value.clone()))
        .take(limit.min(MAX_COMPONENT_ITEMS))
        .collect()
}

fn declared_panes(node: &NativeNode) -> Vec<DeclaredPane> {
    let Some(source) = bounded_json(node, property::ITEMS) else {
        return Vec::new();
    };
    serde_json::from_str::<Vec<DeclaredPane>>(source)
        .unwrap_or_default()
        .into_iter()
        .take(MAX_COMPONENT_VALUES)
        .collect()
}

fn declared_active(node: &NativeNode) -> Option<String> {
    node.string(property::ACTIVE_VALUE)
        .filter(|value| !value.is_empty() && value.len() <= MAX_COMPONENT_VALUE_BYTES)
        .map(str::to_owned)
}

pub(super) fn declares_change(node: &NativeNode) -> bool {
    node.boolean(property::COMPONENT_CHANGE_LISTENER)
        .unwrap_or(false)
}

/// The scope key one declared component instance is retained under.
///
/// A component declares `scope` so its parts resolve to one identity without a JavaScript
/// registry; the retained state is keyed by exactly that identity, so a thumb, handle, or item
/// reaches the same instance as its root.
pub(super) fn component_key(id: u32, node: &NativeNode) -> u64 {
    native_part_scope(id, node).as_u64()
}

/// Recover the declared name behind a derived item identity.
///
/// `ElementId::named` is a hash, so the retained instance keeps the ordered declaration and maps
/// back through it. The list is bounded by [`MAX_COMPONENT_ITEMS`], so this stays a bounded scan.
fn name_of(names: &[String], id: ElementId) -> Option<String> {
    names
        .iter()
        .find(|name| ElementId::named(name.as_str()) == id)
        .cloned()
}

/// One retained slider instance.
#[derive(Default)]
pub(super) struct NativeSliderState {
    pub(super) state: SliderState,
    owner: u32,
    listens: bool,
    declared: Vec<f64>,
    reported: Vec<f64>,
}

/// One retained splitter instance.
pub(super) struct NativeSplitterState {
    pub(super) state: SplitterState,
    owner: u32,
    listens: bool,
    declared: Vec<f32>,
    reported: Vec<f32>,
}

/// One retained toolbar instance.
pub(super) struct NativeToolbarState {
    pub(super) state: ToolbarState,
    pub(super) items: Vec<ToolbarItem>,
    pub(super) names: Vec<String>,
    pub(super) orientation: ToolbarOrientation,
    pub(super) loop_focus: bool,
    owner: u32,
    listens: bool,
    declared: Option<String>,
    reported: Option<String>,
}

/// One retained toggle-group instance.
pub(super) struct NativeToggleGroupState {
    pub(super) state: ToggleGroupState,
    pub(super) items: Vec<ToggleGroupItem>,
    pub(super) names: Vec<String>,
    pub(super) orientation: AccessibilityOrientation,
    pub(super) loop_focus: bool,
    owner: u32,
    listens: bool,
    declared: Vec<String>,
    reported: Vec<String>,
}

impl NativeToggleGroupState {
    fn pressed_names(&self) -> Vec<String> {
        self.state
            .pressed_values()
            .iter()
            .filter_map(|value| name_of(&self.names, *value))
            .collect()
    }
}

/// Every declared component instance this window retains.
///
/// The maps live directly on the view rather than behind a shared cell so a per-instance
/// [`quickgui::StateAccessor`] can hand the core an exclusive borrow of one instance's state.
#[derive(Default)]
pub(super) struct NativeComponentStates {
    pub(super) sliders: HashMap<u64, NativeSliderState>,
    pub(super) splitters: HashMap<u64, NativeSplitterState>,
    pub(super) toolbars: HashMap<u64, NativeToolbarState>,
    pub(super) toggle_groups: HashMap<u64, NativeToggleGroupState>,
    pub(super) number_fields: HashMap<u64, NativeNumberFieldState>,
    pub(super) date_fields: HashMap<u64, NativeDateFieldState>,
    pub(super) time_fields: HashMap<u64, NativeTimeFieldState>,
    pub(super) calendars: HashMap<u64, NativeCalendarState>,
    pub(super) menubars: HashMap<u64, NativeMenubarState>,
    pub(super) toasts: HashMap<u64, NativeToastState>,
    pub(super) selects: HashMap<u64, NativeSelectState>,
    pub(super) comboboxes: HashMap<u64, NativeComboboxState>,
    pub(super) autocompletes: HashMap<u64, NativeAutocompleteState>,
    pub(super) tables: HashMap<u64, NativeTableState>,
    pub(super) trees: HashMap<u64, NativeTreeState>,
}

impl NativeComponentStates {
    /// Reseed every declared instance from this frame's declaration and report what the core did.
    ///
    /// Reseeding happens only when the declaration itself changed, so a value the core moved stays
    /// moved until JavaScript commits the matching prop. Anything the core changed since the last
    /// pass leaves as one asynchronous `componentchange` event, which is the only way a result
    /// crosses back to the hosted runtime.
    pub(super) fn sync(&mut self, tree: &NativeTree, window: u32, events: &EventQueue) {
        let mut sliders = HashSet::new();
        let mut splitters = HashSet::new();
        let mut toolbars = HashSet::new();
        let mut toggle_groups = HashSet::new();
        let mut live = NativeLiveComponentKeys::default();
        for (id, node) in &tree.nodes {
            let Some(part) = node.string(property::PART) else {
                continue;
            };
            let key = component_key(*id, node);
            match part {
                SLIDER_PART if sliders.len() < MAX_COMPONENT_INSTANCES => {
                    sliders.insert(key);
                    self.sync_slider(key, *id, node);
                }
                SPLITTER_PART if splitters.len() < MAX_COMPONENT_INSTANCES => {
                    splitters.insert(key);
                    self.sync_splitter(key, *id, node);
                }
                TOOLBAR_PART if toolbars.len() < MAX_COMPONENT_INSTANCES => {
                    toolbars.insert(key);
                    self.sync_toolbar(key, *id, node);
                }
                TOGGLE_GROUP_PART if toggle_groups.len() < MAX_COMPONENT_INSTANCES => {
                    toggle_groups.insert(key);
                    self.sync_toggle_group(key, *id, node);
                }
                NUMBER_FIELD_PART if live.number_fields.len() < MAX_COMPONENT_INSTANCES => {
                    live.number_fields.insert(key);
                    self.sync_number_field(key, *id, node);
                }
                DATE_FIELD_PART if live.date_fields.len() < MAX_COMPONENT_INSTANCES => {
                    live.date_fields.insert(key);
                    self.sync_date_field(key, *id, node);
                }
                TIME_FIELD_PART if live.time_fields.len() < MAX_COMPONENT_INSTANCES => {
                    live.time_fields.insert(key);
                    self.sync_time_field(key, *id, node);
                }
                CALENDAR_PART if live.calendars.len() < MAX_COMPONENT_INSTANCES => {
                    live.calendars.insert(key);
                    self.sync_calendar(key, *id, node);
                }
                MENUBAR_PART if live.menubars.len() < MAX_COMPONENT_INSTANCES => {
                    live.menubars.insert(key);
                    self.sync_menubar(key, *id, node);
                }
                TOAST_VIEWPORT_PART if live.toasts.len() < MAX_COMPONENT_INSTANCES => {
                    live.toasts.insert(key);
                    self.sync_toasts(key, *id, node);
                }
                SELECT_PART if live.selects.len() < MAX_COMPONENT_INSTANCES => {
                    live.selects.insert(key);
                    self.sync_select(key, *id, node, tree);
                }
                COMBOBOX_PART if live.comboboxes.len() < MAX_COMPONENT_INSTANCES => {
                    live.comboboxes.insert(key);
                    self.sync_combobox(key, *id, node, tree);
                }
                AUTOCOMPLETE_PART if live.autocompletes.len() < MAX_COMPONENT_INSTANCES => {
                    live.autocompletes.insert(key);
                    self.sync_autocomplete(key, *id, node, tree);
                }
                TABLE_PART if live.tables.len() < MAX_COMPONENT_INSTANCES => {
                    live.tables.insert(key);
                    self.sync_table(key, *id, node);
                }
                TREE_PART if live.trees.len() < MAX_COMPONENT_INSTANCES => {
                    live.trees.insert(key);
                    self.sync_tree(key, *id, node);
                }
                _ => {}
            }
        }
        self.sliders.retain(|key, _| sliders.contains(key));
        self.splitters.retain(|key, _| splitters.contains(key));
        self.toolbars.retain(|key, _| toolbars.contains(key));
        self.toggle_groups
            .retain(|key, _| toggle_groups.contains(key));
        self.number_fields
            .retain(|key, _| live.number_fields.contains(key));
        self.date_fields
            .retain(|key, _| live.date_fields.contains(key));
        self.time_fields
            .retain(|key, _| live.time_fields.contains(key));
        self.calendars.retain(|key, _| live.calendars.contains(key));
        self.menubars.retain(|key, _| live.menubars.contains(key));
        self.toasts.retain(|key, _| live.toasts.contains(key));
        self.selects.retain(|key, _| live.selects.contains(key));
        self.comboboxes
            .retain(|key, _| live.comboboxes.contains(key));
        self.autocompletes
            .retain(|key, _| live.autocompletes.contains(key));
        self.tables.retain(|key, _| live.tables.contains(key));
        self.trees.retain(|key, _| live.trees.contains(key));
        self.report(window, events);
        self.report_fields(window, events);
        self.report_pickers(window, events);
    }

    fn sync_slider(&mut self, key: u64, id: u32, node: &NativeNode) {
        let minimum = f64::from(node.number(property::MINIMUM).unwrap_or(0.0));
        let maximum = f64::from(
            node.number(property::MAXIMUM)
                .unwrap_or(DEFAULT_SLIDER_MAXIMUM),
        );
        let mut declared = declared_numbers(node, property::VALUES);
        if declared.is_empty() {
            declared.push(0.0);
        }
        declared.truncate(MAX_SLIDER_THUMBS);
        let build = |values: &[f64]| {
            let mut state = SliderState::range(minimum, maximum, values);
            if let Some(step) = node.number(property::STEP) {
                state = state.step(f64::from(step));
            }
            if let Some(large_step) = node.number(property::LARGE_STEP) {
                state = state.large_step(f64::from(large_step));
            }
            state
                .orientation(match node.string(property::ORIENTATION) {
                    Some("vertical") => SliderOrientation::Vertical,
                    _ => SliderOrientation::Horizontal,
                })
                .disabled(node.boolean(property::DISABLED).unwrap_or(false))
        };
        let state = build(&declared);
        let listens = declares_change(node);
        let normalized = state.values().to_vec();
        match self.sliders.get_mut(&key) {
            Some(retained) => {
                retained.owner = id;
                retained.listens = listens;
                if retained.declared != normalized {
                    retained.declared = normalized.clone();
                    retained.reported = normalized;
                    retained.state = state;
                } else {
                    // The declaration is unchanged, so the core keeps the values and active thumb
                    // it decided; everything else in the declaration is replayed onto them.
                    let values = retained.state.values().to_vec();
                    let active = retained.state.active_thumb();
                    retained.state = build(&values);
                    retained.state.set_active_thumb(active);
                }
            }
            None => {
                self.sliders.insert(
                    key,
                    NativeSliderState {
                        state,
                        owner: id,
                        listens,
                        declared: normalized.clone(),
                        reported: normalized,
                    },
                );
            }
        }
    }

    fn sync_splitter(&mut self, key: u64, id: u32, node: &NativeNode) {
        let orientation = match node.string(property::ORIENTATION) {
            Some("vertical") => SplitterOrientation::Vertical,
            _ => SplitterOrientation::Horizontal,
        };
        let mut declared = declared_numbers(node, property::VALUES)
            .into_iter()
            .map(|value| value as f32)
            .collect::<Vec<_>>();
        if declared.len() < 2 {
            declared = vec![DEFAULT_SPLITTER_SIZE, DEFAULT_SPLITTER_SIZE];
        }
        declared.truncate(MAX_SPLITTER_PANES);
        let panes = declared_panes(node);
        let build = |sizes: &[f32]| {
            let mut state = SplitterState::new(orientation, sizes);
            for (index, pane) in panes.iter().enumerate() {
                if let Some(minimum) = pane.min {
                    state = state.min_size(index, minimum);
                }
                if pane.collapsible {
                    state = state.collapsible(index, true);
                }
            }
            match node.number(property::STEP) {
                Some(step) => state.keyboard_step(step),
                None => state,
            }
        };
        let state = build(&declared);
        let listens = declares_change(node);
        let normalized = state.sizes().to_vec();
        match self.splitters.get_mut(&key) {
            Some(retained) => {
                retained.owner = id;
                retained.listens = listens;
                if retained.declared != normalized {
                    retained.declared = normalized.clone();
                    retained.reported = normalized;
                    retained.state = state;
                } else {
                    let sizes = retained.state.sizes().to_vec();
                    retained.state = build(&sizes);
                }
            }
            None => {
                self.splitters.insert(
                    key,
                    NativeSplitterState {
                        state,
                        owner: id,
                        listens,
                        declared: normalized.clone(),
                        reported: normalized,
                    },
                );
            }
        }
    }

    fn sync_toolbar(&mut self, key: u64, id: u32, node: &NativeNode) {
        let declared_items = declared_items(node, MAX_TOOLBAR_ITEMS);
        let names = declared_items
            .iter()
            .map(|item| item.value.clone())
            .collect::<Vec<_>>();
        let items = declared_items
            .iter()
            .map(|item| {
                ToolbarItem::new(ElementId::named(item.value.as_str())).disabled(item.disabled)
            })
            .collect::<Vec<_>>();
        let orientation = match node.string(property::ORIENTATION) {
            Some("vertical") => ToolbarOrientation::Vertical,
            _ => ToolbarOrientation::Horizontal,
        };
        let loop_focus = node.boolean(property::LOOP_FOCUS).unwrap_or(false);
        let declared = declared_active(node);
        let listens = declares_change(node);
        let state = match &declared {
            Some(value) => ToolbarState::new(ElementId::named(value.as_str())),
            None => ToolbarState::empty(),
        };
        match self.toolbars.get_mut(&key) {
            Some(retained) => {
                retained.owner = id;
                retained.listens = listens;
                retained.items = items;
                retained.names = names;
                retained.orientation = orientation;
                retained.loop_focus = loop_focus;
                if retained.declared != declared {
                    retained.declared = declared.clone();
                    retained.reported = declared;
                    retained.state = state;
                }
            }
            None => {
                self.toolbars.insert(
                    key,
                    NativeToolbarState {
                        state,
                        items,
                        names,
                        orientation,
                        loop_focus,
                        owner: id,
                        listens,
                        declared: declared.clone(),
                        reported: declared,
                    },
                );
            }
        }
    }

    fn sync_toggle_group(&mut self, key: u64, id: u32, node: &NativeNode) {
        let declared_items = declared_items(node, MAX_TOGGLE_GROUP_ITEMS);
        let names = declared_items
            .iter()
            .map(|item| item.value.clone())
            .collect::<Vec<_>>();
        let items = declared_items
            .iter()
            .map(|item| {
                ToggleGroupItem::new(ElementId::named(item.value.as_str())).disabled(item.disabled)
            })
            .collect::<Vec<_>>();
        let orientation = match node.string(property::ORIENTATION) {
            Some("vertical") => AccessibilityOrientation::Vertical,
            _ => AccessibilityOrientation::Horizontal,
        };
        let loop_focus = node.boolean(property::LOOP_FOCUS).unwrap_or(false);
        let multiple = node.string(property::VARIANT) == Some("multiple");
        let mut declared = declared_strings(node, property::VALUES);
        declared.truncate(MAX_TOGGLE_GROUP_ITEMS);
        let listens = declares_change(node);
        let mut state = if multiple {
            ToggleGroupState::multiple()
        } else {
            ToggleGroupState::single()
        };
        for value in &declared {
            state.press(ElementId::named(value.as_str()));
        }
        if let Some(active) = declared_active(node) {
            state.set_active(Some(ElementId::named(active.as_str())));
        }
        match self.toggle_groups.get_mut(&key) {
            Some(retained) => {
                retained.owner = id;
                retained.listens = listens;
                retained.items = items;
                retained.names = names;
                retained.orientation = orientation;
                retained.loop_focus = loop_focus;
                if retained.declared != declared {
                    retained.declared = declared.clone();
                    retained.reported = declared;
                    retained.state = state;
                }
            }
            None => {
                self.toggle_groups.insert(
                    key,
                    NativeToggleGroupState {
                        state,
                        items,
                        names,
                        orientation,
                        loop_focus,
                        owner: id,
                        listens,
                        declared: declared.clone(),
                        reported: declared,
                    },
                );
            }
        }
    }

    /// Enqueue one asynchronous change event per instance the core moved since the last pass.
    fn report(&mut self, window: u32, events: &EventQueue) {
        for retained in self.sliders.values_mut() {
            let values = retained.state.values().to_vec();
            if values == retained.reported {
                continue;
            }
            retained.reported = values.clone();
            if retained.listens {
                enqueue_component_change(
                    events,
                    window,
                    retained.owner,
                    serde_json::json!({ "values": values }),
                );
            }
        }
        for retained in self.splitters.values_mut() {
            let sizes = retained.state.sizes().to_vec();
            if sizes == retained.reported {
                continue;
            }
            retained.reported = sizes.clone();
            if retained.listens {
                enqueue_component_change(
                    events,
                    window,
                    retained.owner,
                    serde_json::json!({ "sizes": sizes }),
                );
            }
        }
        for retained in self.toolbars.values_mut() {
            let active = retained
                .state
                .active()
                .and_then(|active| name_of(&retained.names, active));
            if active == retained.reported {
                continue;
            }
            retained.reported = active.clone();
            if retained.listens {
                enqueue_component_change(
                    events,
                    window,
                    retained.owner,
                    serde_json::json!({ "active": active }),
                );
            }
        }
        for retained in self.toggle_groups.values_mut() {
            let pressed = retained.pressed_names();
            if pressed == retained.reported {
                continue;
            }
            retained.reported = pressed.clone();
            if retained.listens {
                enqueue_component_change(
                    events,
                    window,
                    retained.owner,
                    serde_json::json!({ "pressed": pressed }),
                );
            }
        }
    }
}

pub(super) fn enqueue_component_change(
    events: &EventQueue,
    window: u32,
    target: u32,
    value: serde_json::Value,
) {
    enqueue_event(
        events,
        QueuedEvent {
            kind: "componentchange",
            window,
            target,
            value: Some(Arc::from(value.to_string().as_str())),
        },
    );
}

impl Default for NativeSplitterState {
    fn default() -> Self {
        Self {
            state: SplitterState::new(SplitterOrientation::Horizontal, &[1.0, 1.0]),
            owner: 0,
            listens: false,
            declared: Vec::new(),
            reported: Vec::new(),
        }
    }
}

impl Default for NativeToolbarState {
    fn default() -> Self {
        Self {
            state: ToolbarState::empty(),
            items: Vec::new(),
            names: Vec::new(),
            orientation: ToolbarOrientation::Horizontal,
            loop_focus: false,
            owner: 0,
            listens: false,
            declared: None,
            reported: None,
        }
    }
}

impl Default for NativeToggleGroupState {
    fn default() -> Self {
        Self {
            state: ToggleGroupState::single(),
            items: Vec::new(),
            names: Vec::new(),
            orientation: AccessibilityOrientation::Horizontal,
            loop_focus: false,
            owner: 0,
            listens: false,
            declared: Vec::new(),
            reported: Vec::new(),
        }
    }
}

/// A per-instance accessor from the hosted view to one declared slider's retained state.
fn slider_accessor(key: u64) -> StateAccessor<NativeView, SliderState> {
    StateAccessor::new(move |view: &mut NativeView| {
        &mut view.components.sliders.entry(key).or_default().state
    })
}

/// A per-instance accessor from the hosted view to one declared splitter's retained state.
fn splitter_accessor(key: u64) -> StateAccessor<NativeView, SplitterState> {
    StateAccessor::new(move |view: &mut NativeView| {
        &mut view.components.splitters.entry(key).or_default().state
    })
}

/// A per-instance accessor from the hosted view to one declared toolbar's retained state.
fn toolbar_accessor(key: u64) -> StateAccessor<NativeView, ToolbarState> {
    StateAccessor::new(move |view: &mut NativeView| {
        &mut view.components.toolbars.entry(key).or_default().state
    })
}

/// A per-instance accessor from the hosted view to one declared toggle group's retained state.
fn toggle_group_accessor(key: u64) -> StateAccessor<NativeView, ToggleGroupState> {
    StateAccessor::new(move |view: &mut NativeView| {
        &mut view.components.toggle_groups.entry(key).or_default().state
    })
}

pub(super) fn declared_index(node: &NativeNode) -> usize {
    node.number(property::ITEM_INDEX)
        .filter(|index| *index >= 0.0 && *index < MAX_COMPONENT_VALUES as f32)
        .map_or(0, |index| index as usize)
}

fn declared_item_value(node: &NativeNode) -> Option<ElementId> {
    node.string(property::PART_VALUE)
        .filter(|value| !value.is_empty() && value.len() <= MAX_COMPONENT_VALUE_BYTES)
        .map(ElementId::named)
}

/// Apply one declared range, ordering, or roving-focus part.
///
/// A part whose instance is not retained — a stale node, or a value the declaration no longer
/// lists — mounts as the caller declared it and registers nothing, so a malformed declaration
/// costs a plain element instead of a panic.
pub(super) fn apply_component_part(
    element: Element,
    id: u32,
    node: &NativeNode,
    components: &NativeComponentStates,
    cx: &mut ViewContext<'_, NativeView>,
    listeners_enabled: bool,
) -> Element {
    let Some(part) = node.string(property::PART) else {
        return element;
    };
    let key = component_key(id, node);
    let root = ElementId::new(key);
    match part {
        SLIDER_PART => {
            let Some(retained) = components.sliders.get(&key) else {
                return element;
            };
            let slider = Slider::new(root, &retained.state);
            let element = slider.root_part(element);
            // A single-thumb slider answers arrows on its root; a range slider answers them on
            // each thumb so the focused thumb is always the one that moves.
            if listeners_enabled && retained.state.thumb_count() == 1 {
                slider.key_part_with(cx, element, slider_accessor(key))
            } else {
                element
            }
        }
        SLIDER_TRACK_PART => {
            let Some(retained) = components.sliders.get(&key) else {
                return element;
            };
            let slider = Slider::new(root, &retained.state);
            let element = slider.track_part(element);
            if !listeners_enabled {
                return element;
            }
            let access = slider_accessor(key);
            // The core delivers the captured track's own laid-out size with the event, so the
            // binding never re-derives geometry the layout already decided.
            let drag = cx.pointer_listener(slider.track_id(), move |view, event, cx| {
                if access.get(view).apply_pointer(event, event.size) {
                    cx.invalidate();
                }
            });
            element.on_pointer(drag)
        }
        SLIDER_RANGE_PART => match components.sliders.get(&key) {
            Some(retained) => Slider::new(root, &retained.state).range_part(element),
            None => element,
        },
        SLIDER_THUMB_PART => {
            let Some(retained) = components.sliders.get(&key) else {
                return element;
            };
            let slider = Slider::new(root, &retained.state);
            let Some(thumb) = slider.thumb(declared_index(node)) else {
                return element;
            };
            let element = thumb.thumb_part(element);
            if listeners_enabled && retained.state.thumb_count() > 1 {
                thumb.key_part_with(cx, element, slider_accessor(key))
            } else {
                element
            }
        }
        SPLITTER_PART => match components.splitters.get(&key) {
            Some(retained) => Splitter::new(root, &retained.state).root_part(element),
            None => element,
        },
        SPLITTER_PANE_PART => {
            let Some(retained) = components.splitters.get(&key) else {
                return element;
            };
            match Splitter::new(root, &retained.state).pane(declared_index(node)) {
                Some(pane) => pane.pane_part(element),
                None => element,
            }
        }
        SPLITTER_HANDLE_PART => {
            let Some(retained) = components.splitters.get(&key) else {
                return element;
            };
            let index = declared_index(node);
            let Some(handle) = Splitter::new(root, &retained.state).handle(index) else {
                return element;
            };
            let element = handle.handle_part(element);
            if !listeners_enabled {
                return element;
            }
            let access = splitter_accessor(key);
            let drag = cx.pointer_listener(handle.handle_id(), move |view, event, cx| {
                if access.get(view).apply_pointer(index, event) {
                    cx.invalidate();
                }
            });
            handle
                .key_part_with(cx, element, splitter_accessor(key))
                .on_pointer(drag)
        }
        TOOLBAR_PART => match components.toolbars.get(&key) {
            Some(retained) => Toolbar::new(root, &retained.state, &retained.items)
                .orientation(retained.orientation)
                .loop_focus(retained.loop_focus)
                .root_part(element),
            None => element,
        },
        TOOLBAR_ITEM_PART => {
            let Some(retained) = components.toolbars.get(&key) else {
                return element;
            };
            let Some(value) = declared_item_value(node) else {
                return element;
            };
            let toolbar = Toolbar::new(root, &retained.state, &retained.items)
                .orientation(retained.orientation)
                .loop_focus(retained.loop_focus);
            let Some(entry) = toolbar.item(value) else {
                return element;
            };
            let element = entry.item_part(element);
            if !listeners_enabled {
                return element;
            }
            let access = toolbar_accessor(key);
            // Pointer activation moves the single Tab stop through the core's own contract, so a
            // clicked item and an arrowed item report the same roving value.
            let focus = cx.listener(entry.item_id(), move |view, cx| {
                if access.get(view).focus(value) {
                    cx.invalidate();
                }
            });
            entry
                .key_part_with(cx, element, toolbar_accessor(key))
                .on_click(focus)
        }
        TOGGLE_GROUP_PART => match components.toggle_groups.get(&key) {
            Some(retained) => {
                let group = ToggleGroup::new(root, &retained.state, &retained.items)
                    .loop_focus(retained.loop_focus);
                let group = match retained.orientation {
                    AccessibilityOrientation::Vertical => group.vertical(),
                    AccessibilityOrientation::Horizontal => group,
                };
                group.root_part(element)
            }
            None => element,
        },
        TOGGLE_GROUP_ITEM_PART => {
            let Some(retained) = components.toggle_groups.get(&key) else {
                return element;
            };
            let Some(value) = declared_item_value(node) else {
                return element;
            };
            let group = ToggleGroup::new(root, &retained.state, &retained.items)
                .loop_focus(retained.loop_focus);
            let group = match retained.orientation {
                AccessibilityOrientation::Vertical => group.vertical(),
                AccessibilityOrientation::Horizontal => group,
            };
            let Some(entry) = group.item(value) else {
                return element;
            };
            let element = entry.item_part(element);
            if !listeners_enabled {
                return element;
            }
            let access = toggle_group_accessor(key);
            // The core owns single/multiple policy and the roving stop; the binding only asks it
            // to toggle the value the user pressed.
            let toggle = cx.listener(entry.item_id(), move |view, cx| {
                let state = access.get(view);
                let changed = state.toggle(value);
                let focused = state.focus(value);
                if changed || focused {
                    cx.invalidate();
                }
            });
            entry
                .key_part_with(cx, element, toggle_group_accessor(key))
                .on_click(toggle)
        }
        _ => element,
    }
}

/// The core-derived identity one declared range, ordering, or roving-focus part mounts under.
///
/// Every identity here is derived from the instance's scope alone, so it resolves before the
/// retained state is read and stays stable while values change.
pub(super) fn component_part_element_id(id: u32, node: &NativeNode) -> Option<ElementId> {
    let part = node.string(property::PART)?;
    let root = ElementId::new(component_key(id, node));
    let slider = || Slider::new(root, &SliderState::default());
    let splitter = || {
        Splitter::new(
            root,
            &SplitterState::new(SplitterOrientation::Horizontal, &[1.0, 1.0]),
        )
    };
    Some(match part {
        SLIDER_PART | SPLITTER_PART | TOOLBAR_PART | TOGGLE_GROUP_PART => root,
        SLIDER_TRACK_PART => slider().track_id(),
        SLIDER_RANGE_PART => slider().range_id(),
        SLIDER_THUMB_PART => slider().thumb_id(declared_index(node)),
        SPLITTER_PANE_PART => splitter().pane_id(declared_index(node)),
        SPLITTER_HANDLE_PART => splitter().handle_id(declared_index(node)),
        TOOLBAR_ITEM_PART => {
            Toolbar::new(root, &ToolbarState::empty(), &[]).item_id(declared_item_value(node)?)
        }
        TOGGLE_GROUP_ITEM_PART => {
            ToggleGroup::new(root, &TOGGLE_GROUP_IDENTITY, &[]).item_id(declared_item_value(node)?)
        }
        _ => return None,
    })
}

/// A pressed-value-free group used only to derive item identities, which ignore pressed state.
static TOGGLE_GROUP_IDENTITY: ToggleGroupState = ToggleGroupState::single();

// ---------------------------------------------------------------------------
// Number fields, date and time fields, month grids, menubars, and toasts
//
// Every component below retains interaction state and exposes a `StateAccessor` entry point, so
// one hosted view addresses many declared instances. The declaration stays the source of truth:
// JavaScript declares bounds, civil values, segment order, menu count, and the queued toasts;
// the core owns parsing, clamping, formatting, segment arithmetic, month arithmetic, roving
// focus, live-region politeness, and auto-dismiss deadlines.
// ---------------------------------------------------------------------------

/// Declared part name of a number-field root.
pub(super) const NUMBER_FIELD_PART: &str = "number-field";
/// Declared part name of a number-field input.
pub(super) const NUMBER_FIELD_INPUT_PART: &str = "number-field-input";
/// Declared part name of a number-field increment stepper.
pub(super) const NUMBER_FIELD_INCREMENT_PART: &str = "number-field-increment";
/// Declared part name of a number-field decrement stepper.
pub(super) const NUMBER_FIELD_DECREMENT_PART: &str = "number-field-decrement";
/// Declared part name of a date-field root.
pub(super) const DATE_FIELD_PART: &str = "date-field";
/// Declared part name of one date-field segment.
pub(super) const DATE_FIELD_SEGMENT_PART: &str = "date-field-segment";
/// Declared part name of a time-field root.
pub(super) const TIME_FIELD_PART: &str = "time-field";
/// Declared part name of one time-field segment.
pub(super) const TIME_FIELD_SEGMENT_PART: &str = "time-field-segment";
/// Declared part name of a month grid.
pub(super) const CALENDAR_PART: &str = "calendar";
/// Declared part name of one calendar week row.
pub(super) const CALENDAR_WEEK_PART: &str = "calendar-week";
/// Declared part name of one calendar day cell.
pub(super) const CALENDAR_DAY_PART: &str = "calendar-day";
/// Declared part name of an in-window menubar root.
pub(super) const MENUBAR_PART: &str = "menubar";
/// Declared part name of one menubar trigger.
pub(super) const MENUBAR_ITEM_PART: &str = "menubar-item";
/// Declared part name of a toast viewport.
pub(super) const TOAST_VIEWPORT_PART: &str = "toast-viewport";
/// Declared part name of one queued toast root.
pub(super) const TOAST_PART: &str = "toast";
/// Declared part name of one toast title.
pub(super) const TOAST_TITLE_PART: &str = "toast-title";
/// Declared part name of one toast description.
pub(super) const TOAST_DESCRIPTION_PART: &str = "toast-description";
/// Declared part name of one toast action control.
pub(super) const TOAST_ACTION_PART: &str = "toast-action";
/// Declared part name of one toast close control.
pub(super) const TOAST_CLOSE_PART: &str = "toast-close";

/// Every component key seen in one declaration pass, so stale instances are dropped.
#[derive(Default)]
pub(super) struct NativeLiveComponentKeys {
    pub(super) number_fields: HashSet<u64>,
    pub(super) date_fields: HashSet<u64>,
    pub(super) time_fields: HashSet<u64>,
    pub(super) calendars: HashSet<u64>,
    pub(super) menubars: HashSet<u64>,
    pub(super) toasts: HashSet<u64>,
    pub(super) selects: HashSet<u64>,
    pub(super) comboboxes: HashSet<u64>,
    pub(super) autocompletes: HashSet<u64>,
    pub(super) tables: HashSet<u64>,
    pub(super) trees: HashSet<u64>,
}

/// Concatenate the declared properties one retained state is rebuilt from.
///
/// Several core states are rebuilt rather than mutated because their replacement mutators need an
/// `EventContext` a render pass does not have. Comparing this fingerprint keeps a rebuild to the
/// frames where the declaration itself actually changed.
pub(super) fn declaration_fingerprint(node: &NativeNode, keys: &[u16]) -> Arc<str> {
    let mut fingerprint = String::new();
    for key in keys {
        fingerprint.push('\u{1f}');
        match node.property(*key) {
            Some(PropertyValue::String(value)) => fingerprint.push_str(value),
            Some(PropertyValue::Number(value)) => fingerprint.push_str(&value.to_string()),
            Some(PropertyValue::Bool(value)) => {
                fingerprint.push_str(if *value { "1" } else { "0" })
            }
            Some(PropertyValue::Color(value)) => fingerprint.push_str(&value.to_string()),
            None => {}
        }
    }
    Arc::from(fingerprint.as_str())
}

/// One retained number-field instance.
pub(super) struct NativeNumberFieldState {
    pub(super) state: NumberFieldState,
    pub(super) repeat_deadline: Option<Instant>,
    owner: u32,
    listens: bool,
    declaration: Arc<str>,
    reported: Arc<str>,
}

impl Default for NativeNumberFieldState {
    fn default() -> Self {
        Self {
            state: NumberFieldState::empty(),
            repeat_deadline: None,
            owner: 0,
            listens: false,
            declaration: Arc::from(""),
            reported: Arc::from(""),
        }
    }
}

/// One retained date-field instance.
pub(super) struct NativeDateFieldState {
    pub(super) state: DateFieldState,
    owner: u32,
    listens: bool,
    declaration: Arc<str>,
    reported: Option<CivilDate>,
}

impl Default for NativeDateFieldState {
    fn default() -> Self {
        Self {
            state: DateFieldState::new(),
            owner: 0,
            listens: false,
            declaration: Arc::from(""),
            reported: None,
        }
    }
}

/// One retained time-field instance.
pub(super) struct NativeTimeFieldState {
    pub(super) state: TimeFieldState,
    owner: u32,
    listens: bool,
    declaration: Arc<str>,
    reported: Option<CivilTime>,
}

impl Default for NativeTimeFieldState {
    fn default() -> Self {
        Self {
            state: TimeFieldState::new(),
            owner: 0,
            listens: false,
            declaration: Arc::from(""),
            reported: None,
        }
    }
}

/// One retained month-grid instance.
pub(super) struct NativeCalendarState {
    pub(super) state: CalendarState,
    owner: u32,
    listens: bool,
    declaration: Arc<str>,
    reported: (Option<CivilDate>, CivilDate),
}

/// The civil date a month grid falls back to when nothing at all is declared.
fn calendar_epoch() -> CivilDate {
    CivilDate::new(1970, 1, 1).expect("1970-01-01 is a real civil date")
}

impl Default for NativeCalendarState {
    fn default() -> Self {
        let epoch = calendar_epoch();
        Self {
            state: CalendarState::new(epoch),
            owner: 0,
            listens: false,
            declaration: Arc::from(""),
            reported: (None, epoch),
        }
    }
}

/// One retained menubar instance.
#[derive(Default)]
pub(super) struct NativeMenubarState {
    pub(super) state: MenubarState,
    owner: u32,
    listens: bool,
    declaration: Arc<str>,
    reported: (Option<usize>, usize),
}

/// One retained toast viewport.
#[derive(Default)]
pub(super) struct NativeToastState {
    pub(super) manager: ToastManager,
    /// Declared identifier per queued toast, aligned with the core's own opaque ids.
    pub(super) ids: Vec<(Arc<str>, ToastId)>,
    pub(super) deadline: Option<Instant>,
    owner: u32,
    listens: bool,
    declaration: Arc<str>,
}

impl NativeToastState {
    /// The retained core entry declared under one identifier.
    pub(super) fn entry(&self, declared: &str) -> Option<&ToastEntry> {
        let id = self
            .ids
            .iter()
            .find(|(value, _)| value.as_ref() == declared)
            .map(|(_, id)| *id)?;
        self.manager.entry(id)
    }
}

/// One declared toast entry.
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DeclaredToast {
    #[serde(default)]
    id: String,
    #[serde(default)]
    title: String,
    #[serde(default)]
    description: Option<String>,
    #[serde(default)]
    action: Option<String>,
    #[serde(default)]
    kind: Option<String>,
    /// Auto-dismiss duration in milliseconds. A missing or non-positive value stays persistent.
    #[serde(default)]
    duration: Option<f64>,
}

fn declared_civil_date(node: &NativeNode, key: u16) -> Option<CivilDate> {
    parse_civil_date(node.string(key)?)
}

/// Parse an ISO `YYYY-MM-DD` civil date. Anything else declares no date at all.
pub(super) fn parse_civil_date(value: &str) -> Option<CivilDate> {
    let mut parts = value.split('-');
    let year = parts.next()?.parse::<i32>().ok()?;
    let month = parts.next()?.parse::<u8>().ok()?;
    let day = parts.next()?.parse::<u8>().ok()?;
    if parts.next().is_some() {
        return None;
    }
    CivilDate::new(year, month, day)
}

fn format_civil_date(value: CivilDate) -> String {
    format!("{:04}-{:02}-{:02}", value.year, value.month, value.day)
}

fn declared_civil_time(node: &NativeNode, key: u16) -> Option<CivilTime> {
    let value = node.string(key)?;
    let mut parts = value.split(':');
    let hour = parts.next()?.parse::<u8>().ok()?;
    let minute = parts.next()?.parse::<u8>().ok()?;
    let second = match parts.next() {
        Some(second) => second.parse::<u8>().ok()?,
        None => 0,
    };
    if parts.next().is_some() {
        return None;
    }
    CivilTime::new(hour, minute, second)
}

fn format_civil_time(value: CivilTime) -> String {
    format!("{:02}:{:02}:{:02}", value.hour, value.minute, value.second)
}

fn declared_date_segment(node: &NativeNode) -> Option<DateSegment> {
    match node.string(property::SEGMENT)? {
        "year" => Some(DateSegment::Year),
        "month" => Some(DateSegment::Month),
        "day" => Some(DateSegment::Day),
        _ => None,
    }
}

fn declared_time_segment(node: &NativeNode) -> Option<TimeSegment> {
    match node.string(property::SEGMENT)? {
        "hour" => Some(TimeSegment::Hour),
        "minute" => Some(TimeSegment::Minute),
        "second" => Some(TimeSegment::Second),
        "period" | "dayPeriod" => Some(TimeSegment::Period),
        _ => None,
    }
}

impl NativeComponentStates {
    fn sync_number_field(&mut self, key: u64, id: u32, node: &NativeNode) {
        let declaration = declaration_fingerprint(
            node,
            &[
                property::VALUES,
                property::MINIMUM,
                property::MAXIMUM,
                property::STEP,
                property::PRECISION,
                property::DISABLED,
            ],
        );
        let listens = declares_change(node);
        let retained = self.number_fields.entry(key).or_default();
        retained.owner = id;
        retained.listens = listens;
        if retained.declaration != declaration {
            let declared = declared_numbers(node, property::VALUES);
            let mut state = match declared.first() {
                Some(value) => NumberFieldState::new(*value),
                None => NumberFieldState::empty(),
            };
            let minimum = node.number(property::MINIMUM).map(f64::from);
            let maximum = node.number(property::MAXIMUM).map(f64::from);
            if minimum.is_some() || maximum.is_some() {
                state = state.range(
                    minimum.unwrap_or(f64::NEG_INFINITY),
                    maximum.unwrap_or(f64::INFINITY),
                );
            }
            if let Some(step) = node.number(property::STEP) {
                state = state.step(f64::from(step));
            }
            if let Some(precision) = node
                .number(property::PRECISION)
                .filter(|precision| *precision >= 0.0)
            {
                let precision = (precision as u32).min(u32::from(MAX_NUMBER_FIELD_PRECISION));
                state = state.precision(precision as u8);
            }
            retained.declaration = declaration;
            retained.reported = state.text().clone();
            retained.state = state;
            retained.repeat_deadline = None;
            return;
        }
        // A held stepper repeats on the core's own exact deadlines: the binding applies every step
        // that came due and asks the window for one repaint at the next one.
        if retained.state.repeat_deadline().is_some() {
            retained.state.repeat(Instant::now());
        }
        retained.repeat_deadline = retained.state.repeat_deadline();
    }

    fn sync_date_field(&mut self, key: u64, id: u32, node: &NativeNode) {
        let declaration = declaration_fingerprint(
            node,
            &[
                property::CIVIL_VALUE,
                property::CIVIL_MINIMUM,
                property::CIVIL_MAXIMUM,
                property::SEGMENT_ORDER,
                property::DISABLED,
            ],
        );
        let listens = declares_change(node);
        let retained = self.date_fields.entry(key).or_default();
        retained.owner = id;
        retained.listens = listens;
        if retained.declaration == declaration {
            return;
        }
        let value = declared_civil_date(node, property::CIVIL_VALUE);
        let mut state = match value {
            Some(value) => DateFieldState::from_date(value),
            None => DateFieldState::new(),
        }
        .order(match node.string(property::SEGMENT_ORDER) {
            Some("dmy" | "day-month-year") => DateFieldOrder::DayMonthYear,
            Some("mdy" | "month-day-year") => DateFieldOrder::MonthDayYear,
            _ => DateFieldOrder::YearMonthDay,
        })
        .disabled(node.boolean(property::DISABLED).unwrap_or(false));
        if let Some(minimum) = declared_civil_date(node, property::CIVIL_MINIMUM) {
            state = state.minimum(minimum);
        }
        if let Some(maximum) = declared_civil_date(node, property::CIVIL_MAXIMUM) {
            state = state.maximum(maximum);
        }
        retained.declaration = declaration;
        retained.reported = state.value();
        retained.state = state;
    }

    fn sync_time_field(&mut self, key: u64, id: u32, node: &NativeNode) {
        let declaration = declaration_fingerprint(
            node,
            &[
                property::CIVIL_VALUE,
                property::CIVIL_MINIMUM,
                property::CIVIL_MAXIMUM,
                property::SEGMENT_ORDER,
                property::VARIANT,
                property::DISABLED,
            ],
        );
        let listens = declares_change(node);
        let retained = self.time_fields.entry(key).or_default();
        retained.owner = id;
        retained.listens = listens;
        if retained.declaration == declaration {
            return;
        }
        let value = declared_civil_time(node, property::CIVIL_VALUE);
        let mut state = match value {
            Some(value) => TimeFieldState::from_time(value),
            None => TimeFieldState::new(),
        }
        .hour12(node.string(property::SEGMENT_ORDER) == Some("h12"))
        .seconds(node.string(property::VARIANT) == Some("seconds"))
        .disabled(node.boolean(property::DISABLED).unwrap_or(false));
        if let Some(minimum) = declared_civil_time(node, property::CIVIL_MINIMUM) {
            state = state.minimum(minimum);
        }
        if let Some(maximum) = declared_civil_time(node, property::CIVIL_MAXIMUM) {
            state = state.maximum(maximum);
        }
        retained.declaration = declaration;
        retained.reported = state.value();
        retained.state = state;
    }

    fn sync_calendar(&mut self, key: u64, id: u32, node: &NativeNode) {
        let declaration = declaration_fingerprint(
            node,
            &[
                property::CIVIL_VALUE,
                property::CIVIL_MINIMUM,
                property::CIVIL_MAXIMUM,
                property::FIRST_WEEKDAY,
                property::DISABLED,
            ],
        );
        let listens = declares_change(node);
        let retained = self.calendars.entry(key).or_default();
        retained.owner = id;
        retained.listens = listens;
        if retained.declaration == declaration {
            return;
        }
        let selected = declared_civil_date(node, property::CIVIL_VALUE);
        let mut state = match selected {
            Some(selected) => CalendarState::selected(selected),
            None => CalendarState::new(calendar_epoch()),
        }
        .first_weekday(CalendarWeekday::from_index(
            node.number(property::FIRST_WEEKDAY)
                .filter(|index| (0.0..7.0).contains(index))
                .map_or(0, |index| index as u8),
        ))
        .disabled(node.boolean(property::DISABLED).unwrap_or(false));
        if let Some(minimum) = declared_civil_date(node, property::CIVIL_MINIMUM) {
            state = state.minimum(minimum);
        }
        if let Some(maximum) = declared_civil_date(node, property::CIVIL_MAXIMUM) {
            state = state.maximum(maximum);
        }
        retained.declaration = declaration;
        retained.reported = (state.selected_day(), state.focused_day());
        retained.state = state;
    }

    fn sync_menubar(&mut self, key: u64, id: u32, node: &NativeNode) {
        let declaration = declaration_fingerprint(
            node,
            &[
                property::MENU_COUNT,
                property::ITEM_INDEX,
                property::OPEN,
                property::DISABLED,
            ],
        );
        let listens = declares_change(node);
        let count = node
            .number(property::MENU_COUNT)
            .filter(|count| *count >= 0.0)
            .map_or(0, |count| (count as usize).min(MAX_MENUBAR_MENUS));
        let retained = self.menubars.entry(key).or_default();
        retained.owner = id;
        retained.listens = listens;
        retained.state.set_menu_count(count);
        if retained.declaration == declaration {
            return;
        }
        let mut state =
            MenubarState::new(count).disabled(node.boolean(property::DISABLED).unwrap_or(false));
        let open = node.boolean(property::OPEN).unwrap_or(false);
        let index = node
            .number(property::ITEM_INDEX)
            .filter(|index| *index >= 0.0 && (*index as usize) < count)
            .map(|index| index as usize);
        match (index, open) {
            (Some(index), true) => {
                state.open_menu_at(index);
            }
            (Some(index), false) => {
                state.focus_menu(index);
            }
            (None, true) => {
                state.open_focused();
            }
            (None, false) => {}
        }
        retained.declaration = declaration;
        retained.reported = (state.open_menu(), state.focused_menu());
        retained.state = state;
    }

    fn sync_toasts(&mut self, key: u64, id: u32, node: &NativeNode) {
        let declaration: Arc<str> = Arc::from(node.string(property::TOASTS).unwrap_or(""));
        let listens = declares_change(node);
        let declared = bounded_json(node, property::TOASTS)
            .and_then(|source| serde_json::from_str::<Vec<DeclaredToast>>(source).ok())
            .unwrap_or_default();
        let retained = self.toasts.entry(key).or_default();
        retained.owner = id;
        retained.listens = listens;
        if retained.declaration != declaration {
            retained.declaration = declaration;
            let now = Instant::now();
            // Pushing a toast is adding one to the declared list: an identifier the manager has
            // never seen is pushed, and an identifier the declaration dropped is dismissed.
            let mut next: Vec<(Arc<str>, ToastId)> = Vec::new();
            for entry in declared.into_iter().take(MAX_TOASTS) {
                if entry.id.is_empty() || entry.id.len() > MAX_COMPONENT_VALUE_BYTES {
                    continue;
                }
                let declared_id: Arc<str> = Arc::from(entry.id.as_str());
                if let Some(existing) = retained
                    .ids
                    .iter()
                    .find(|(value, _)| *value == declared_id)
                    .map(|(_, id)| *id)
                    .filter(|id| retained.manager.entry(*id).is_some())
                {
                    next.push((declared_id, existing));
                    continue;
                }
                let mut toast =
                    CoreToast::new(entry.title.as_str()).kind(match entry.kind.as_deref() {
                        Some("success") => ToastKind::Success,
                        Some("warning") => ToastKind::Warning,
                        Some("error") => ToastKind::Error,
                        _ => ToastKind::Info,
                    });
                if let Some(description) = entry.description.as_deref().filter(|v| !v.is_empty()) {
                    toast = toast.description(description);
                }
                if let Some(action) = entry.action.as_deref().filter(|v| !v.is_empty()) {
                    toast = toast.action(action);
                }
                toast = match entry.duration.filter(|ms| ms.is_finite() && *ms > 0.0) {
                    Some(ms) => toast.duration(Duration::from_millis(ms as u64)),
                    None => toast.persistent(),
                };
                next.push((declared_id, retained.manager.push(toast, now)));
            }
            let kept = next.iter().map(|(_, id)| *id).collect::<Vec<_>>();
            for (_, existing) in &retained.ids {
                if !kept.contains(existing) {
                    retained.manager.dismiss(*existing);
                }
            }
            retained.ids = next;
        }
        retained.manager.expire(Instant::now());
        retained.deadline = retained.manager.next_deadline();
    }

    /// Enqueue one asynchronous change event per field instance the core moved.
    pub(super) fn report_fields(&mut self, window: u32, events: &EventQueue) {
        for retained in self.number_fields.values_mut() {
            let text = retained.state.text().clone();
            if text == retained.reported {
                continue;
            }
            retained.reported = text.clone();
            if retained.listens {
                enqueue_component_change(
                    events,
                    window,
                    retained.owner,
                    serde_json::json!({
                        "value": retained.state.value(),
                        "text": text.as_ref(),
                        "valid": retained.state.is_valid(),
                    }),
                );
            }
        }
        for retained in self.date_fields.values_mut() {
            let value = retained.state.value();
            if value == retained.reported {
                continue;
            }
            retained.reported = value;
            if retained.listens {
                enqueue_component_change(
                    events,
                    window,
                    retained.owner,
                    serde_json::json!({ "value": value.map(format_civil_date) }),
                );
            }
        }
        for retained in self.time_fields.values_mut() {
            let value = retained.state.value();
            if value == retained.reported {
                continue;
            }
            retained.reported = value;
            if retained.listens {
                enqueue_component_change(
                    events,
                    window,
                    retained.owner,
                    serde_json::json!({ "value": value.map(format_civil_time) }),
                );
            }
        }
        for retained in self.calendars.values_mut() {
            let current = (retained.state.selected_day(), retained.state.focused_day());
            if current == retained.reported {
                continue;
            }
            retained.reported = current;
            if retained.listens {
                let (year, month) = retained.state.displayed_month();
                enqueue_component_change(
                    events,
                    window,
                    retained.owner,
                    serde_json::json!({
                        "value": current.0.map(format_civil_date),
                        "focused": format_civil_date(current.1),
                        "month": format!("{year:04}-{month:02}"),
                    }),
                );
            }
        }
        for retained in self.menubars.values_mut() {
            let current = (retained.state.open_menu(), retained.state.focused_menu());
            if current == retained.reported {
                continue;
            }
            retained.reported = current;
            if retained.listens {
                enqueue_component_change(
                    events,
                    window,
                    retained.owner,
                    serde_json::json!({ "open": current.0, "focused": current.1 }),
                );
            }
        }
        for retained in self.toasts.values_mut() {
            let dismissed = retained
                .ids
                .iter()
                .filter(|(_, id)| retained.manager.entry(*id).is_none())
                .map(|(declared, _)| declared.to_string())
                .collect::<Vec<_>>();
            if dismissed.is_empty() {
                continue;
            }
            retained
                .ids
                .retain(|(_, id)| retained.manager.entry(*id).is_some());
            if retained.listens {
                enqueue_component_change(
                    events,
                    window,
                    retained.owner,
                    serde_json::json!({ "dismissed": dismissed }),
                );
            }
        }
    }
}

/// A per-instance accessor from the hosted view to one declared number field.
fn number_field_accessor(key: u64) -> StateAccessor<NativeView, NumberFieldState> {
    StateAccessor::new(move |view: &mut NativeView| {
        &mut view.components.number_fields.entry(key).or_default().state
    })
}

/// A per-instance accessor from the hosted view to one declared date field.
fn date_field_accessor(key: u64) -> StateAccessor<NativeView, DateFieldState> {
    StateAccessor::new(move |view: &mut NativeView| {
        &mut view.components.date_fields.entry(key).or_default().state
    })
}

/// A per-instance accessor from the hosted view to one declared time field.
fn time_field_accessor(key: u64) -> StateAccessor<NativeView, TimeFieldState> {
    StateAccessor::new(move |view: &mut NativeView| {
        &mut view.components.time_fields.entry(key).or_default().state
    })
}

/// A per-instance accessor from the hosted view to one declared month grid.
fn calendar_accessor(key: u64) -> StateAccessor<NativeView, CalendarState> {
    StateAccessor::new(move |view: &mut NativeView| {
        &mut view.components.calendars.entry(key).or_default().state
    })
}

/// A per-instance accessor from the hosted view to one declared menubar.
fn menubar_accessor(key: u64) -> StateAccessor<NativeView, MenubarState> {
    StateAccessor::new(move |view: &mut NativeView| {
        &mut view.components.menubars.entry(key).or_default().state
    })
}

/// A per-instance accessor from the hosted view to one declared toast viewport.
fn toast_accessor(key: u64) -> StateAccessor<NativeView, ToastManager> {
    StateAccessor::new(move |view: &mut NativeView| {
        &mut view.components.toasts.entry(key).or_default().manager
    })
}

/// Apply one declared number-field, date-field, time-field, calendar, menubar, or toast part.
#[allow(clippy::too_many_arguments)]
pub(super) fn apply_field_part(
    element: Element,
    part: &str,
    id: u32,
    window: u32,
    node: &NativeNode,
    events: &EventQueue,
    components: &NativeComponentStates,
    cx: &mut ViewContext<'_, NativeView>,
    listeners_enabled: bool,
) -> Option<Element> {
    let key = component_key(id, node);
    let root = ElementId::new(key);
    match part {
        NUMBER_FIELD_PART => {
            let Some(retained) = components.number_fields.get(&key) else {
                return Some(element);
            };
            if let Some(deadline) = retained.repeat_deadline {
                cx.request_repaint_at(deadline);
            }
            Some(NumberField::new(root).root_part(element))
        }
        NUMBER_FIELD_INPUT_PART => {
            let Some(retained) = components.number_fields.get(&key) else {
                return Some(element);
            };
            let field = NumberField::new(root);
            let element = field.input_part(&retained.state, element);
            if !listeners_enabled {
                return Some(element);
            }
            let access = number_field_accessor(key);
            // The core owns parsing, clamping, and formatting; the binding only hands it the text
            // the user typed and asks for the frame that publishes the outcome.
            let edit = cx.input_listener(field.input_id(), move |view, value, cx| {
                if access.get(view).set_text(value) {
                    cx.invalidate();
                }
            });
            let access = number_field_accessor(key);
            let commits = node.boolean(property::COMMIT_LISTENER).unwrap_or(false);
            let commit_events = Rc::clone(events);
            let commit = cx.submit_listener(field.input_id(), move |view, _value, cx| {
                let state = access.get(view);
                let changed = state.commit();
                let value = state.value();
                if commits {
                    enqueue_event(
                        &commit_events,
                        QueuedEvent {
                            kind: "commit",
                            window,
                            target: id,
                            value: Some(Arc::from(
                                serde_json::json!({ "value": value }).to_string().as_str(),
                            )),
                        },
                    );
                }
                if changed {
                    cx.invalidate();
                }
            });
            Some(element.on_input(edit).on_submit(commit))
        }
        NUMBER_FIELD_INCREMENT_PART | NUMBER_FIELD_DECREMENT_PART => {
            let Some(retained) = components.number_fields.get(&key) else {
                return Some(element);
            };
            let field = NumberField::new(root);
            let forward = part == NUMBER_FIELD_INCREMENT_PART;
            let element = if forward {
                field.increment_part(&retained.state, element)
            } else {
                field.decrement_part(&retained.state, element)
            };
            if !listeners_enabled {
                return Some(element);
            }
            let stepper_id = if forward {
                field.increment_id()
            } else {
                field.decrement_id()
            };
            // Press and hold is the core's own bounded repeat: the binding arms it on press,
            // releases it on lift, and never owns a timer of its own.
            let access = number_field_accessor(key);
            let press = cx.mouse_down_listener(stepper_id, move |view, _event, cx| {
                if access.get(view).press_step(forward, Instant::now()) {
                    cx.invalidate();
                }
            });
            let access = number_field_accessor(key);
            let release = cx.mouse_up_listener(stepper_id, move |view, _event, cx| {
                if access.get(view).release_step() {
                    cx.invalidate();
                }
            });
            Some(
                element
                    .on_mouse_down(quickgui::MouseButton::Left, press)
                    .on_mouse_up(quickgui::MouseButton::Left, release),
            )
        }
        DATE_FIELD_PART => match components.date_fields.get(&key) {
            Some(retained) => Some(DateField::new(root).root_part(&retained.state, element)),
            None => Some(element),
        },
        DATE_FIELD_SEGMENT_PART => {
            let Some(retained) = components.date_fields.get(&key) else {
                return Some(element);
            };
            let Some(segment) = declared_date_segment(node) else {
                return Some(element);
            };
            let descriptor = DateField::new(root).segment(segment);
            let element = descriptor.segment_part(&retained.state, element);
            if !listeners_enabled {
                return Some(element);
            }
            Some(descriptor.key_part_with(cx, element, date_field_accessor(key)))
        }
        TIME_FIELD_PART => match components.time_fields.get(&key) {
            Some(retained) => Some(TimeField::new(root).root_part(&retained.state, element)),
            None => Some(element),
        },
        TIME_FIELD_SEGMENT_PART => {
            let Some(retained) = components.time_fields.get(&key) else {
                return Some(element);
            };
            let Some(segment) = declared_time_segment(node) else {
                return Some(element);
            };
            // A twelve-hour field has no seconds segment unless one is declared, so a segment the
            // core does not own contributes no layout, paint, or accessibility node at all.
            if !retained.state.has_segment(segment) {
                return None;
            }
            let descriptor = TimeField::new(root).segment(segment);
            let element = descriptor.segment_part(&retained.state, element);
            if !listeners_enabled {
                return Some(element);
            }
            Some(descriptor.key_part_with(cx, element, time_field_accessor(key)))
        }
        CALENDAR_PART => match components.calendars.get(&key) {
            Some(retained) => Some(Calendar::new(root).grid_part(retained.state, element)),
            None => Some(element),
        },
        CALENDAR_WEEK_PART => {
            let Some(retained) = components.calendars.get(&key) else {
                return Some(element);
            };
            let index = declared_index(node);
            if index >= retained.state.week_count() {
                return None;
            }
            Some(Calendar::new(root).week_part(index, element))
        }
        CALENDAR_DAY_PART => {
            let Some(retained) = components.calendars.get(&key) else {
                return Some(element);
            };
            let day = node
                .string(property::CIVIL_VALUE)
                .and_then(parse_civil_date)?;
            let calendar = Calendar::new(root);
            let element = calendar.day_part(retained.state, day, element);
            if !listeners_enabled {
                return Some(element);
            }
            Some(calendar.key_part_with(cx, day, element, calendar_accessor(key)))
        }
        MENUBAR_PART => match components.menubars.get(&key) {
            Some(retained) => Some(Menubar::new(root).root_part(retained.state, element)),
            None => Some(element),
        },
        MENUBAR_ITEM_PART => {
            let Some(retained) = components.menubars.get(&key) else {
                return Some(element);
            };
            let index = declared_index(node);
            let Some(item) = Menubar::new(root).item(retained.state, index) else {
                return Some(element);
            };
            let element = item.item_part(element);
            if !listeners_enabled {
                return Some(element);
            }
            Some(item.key_part_with(cx, element, menubar_accessor(key)))
        }
        TOAST_VIEWPORT_PART => {
            let Some(retained) = components.toasts.get(&key) else {
                return Some(element);
            };
            // The core reports one exact deadline for the whole queue; the window sleeps until
            // then and owns no timer of its own.
            if let Some(deadline) = retained.deadline {
                cx.request_repaint_at(deadline);
            }
            Some(ToastViewport::new(root).viewport_part(element))
        }
        TOAST_PART
        | TOAST_TITLE_PART
        | TOAST_DESCRIPTION_PART
        | TOAST_ACTION_PART
        | TOAST_CLOSE_PART => {
            let retained = components.toasts.get(&key)?;
            let declared = node.string(property::PART_VALUE)?;
            let entry = retained.entry(declared)?;
            let parts = ToastViewport::new(root).toast(entry);
            let toast_id = parts.id();
            match part {
                TOAST_PART => {
                    let element = parts.root_part(element);
                    if !listeners_enabled {
                        return Some(element);
                    }
                    Some(parts.key_part_with(cx, element, toast_accessor(key)))
                }
                TOAST_TITLE_PART => Some(parts.title_part(element)),
                TOAST_DESCRIPTION_PART => Some(parts.description_part(element)),
                TOAST_ACTION_PART => Some(parts.action_part(element)),
                _ => {
                    let element = parts.close_part(element);
                    if !listeners_enabled {
                        return Some(element);
                    }
                    let access = toast_accessor(key);
                    let dismiss = cx.listener(parts.close_id(), move |view, cx| {
                        if access.get(view).dismiss(toast_id) {
                            cx.invalidate();
                        }
                    });
                    Some(element.on_click(dismiss))
                }
            }
        }
        _ => Some(element),
    }
}

/// The core-derived identity one declared field, calendar, menubar, or toast part mounts under.
pub(super) fn field_part_element_id(
    part: &str,
    id: u32,
    node: &NativeNode,
    components: &NativeComponentStates,
) -> Option<ElementId> {
    let key = component_key(id, node);
    let root = ElementId::new(key);
    Some(match part {
        NUMBER_FIELD_PART | DATE_FIELD_PART | TIME_FIELD_PART | CALENDAR_PART | MENUBAR_PART
        | TOAST_VIEWPORT_PART => root,
        NUMBER_FIELD_INPUT_PART => NumberField::new(root).input_id(),
        NUMBER_FIELD_INCREMENT_PART => NumberField::new(root).increment_id(),
        NUMBER_FIELD_DECREMENT_PART => NumberField::new(root).decrement_id(),
        DATE_FIELD_SEGMENT_PART => DateField::new(root).segment_id(declared_date_segment(node)?),
        TIME_FIELD_SEGMENT_PART => TimeField::new(root).segment_id(declared_time_segment(node)?),
        CALENDAR_WEEK_PART => Calendar::new(root).week_id(declared_index(node)),
        CALENDAR_DAY_PART => Calendar::new(root).day_id(
            node.string(property::CIVIL_VALUE)
                .and_then(parse_civil_date)?,
        ),
        MENUBAR_ITEM_PART => Menubar::new(root).item_id(declared_index(node)),
        TOAST_PART
        | TOAST_TITLE_PART
        | TOAST_DESCRIPTION_PART
        | TOAST_ACTION_PART
        | TOAST_CLOSE_PART => {
            let retained = components.toasts.get(&key)?;
            let entry = retained.entry(node.string(property::PART_VALUE)?)?;
            let parts = ToastViewport::new(root).toast(entry);
            match part {
                TOAST_PART => parts.root_id(),
                TOAST_TITLE_PART => parts.title_id(),
                TOAST_DESCRIPTION_PART => parts.description_id(),
                TOAST_ACTION_PART => parts.action_id(),
                _ => parts.close_id(),
            }
        }
        _ => return None,
    })
}
