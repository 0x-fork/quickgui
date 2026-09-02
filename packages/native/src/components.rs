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

fn bounded_json(node: &NativeNode, key: u16) -> Option<&str> {
    node.string(key)
        .filter(|value| value.len() <= MAX_COMPONENT_JSON_BYTES)
}

/// Decode a bounded numeric declaration. A malformed declaration yields no numbers, never a panic.
fn declared_numbers(node: &NativeNode, key: u16) -> Vec<f64> {
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

fn declares_change(node: &NativeNode) -> bool {
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
                _ => {}
            }
        }
        self.sliders.retain(|key, _| sliders.contains(key));
        self.splitters.retain(|key, _| splitters.contains(key));
        self.toolbars.retain(|key, _| toolbars.contains(key));
        self.toggle_groups
            .retain(|key, _| toggle_groups.contains(key));
        self.report(window, events);
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

fn enqueue_component_change(
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

fn declared_index(node: &NativeNode) -> usize {
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
