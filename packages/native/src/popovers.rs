//! Base UI-aligned popover and tooltip compounds bound to the Rust core.
//!
//! Both compounds follow the same contract as every other declared component: JavaScript declares
//! the controlled open value, the preferred side and alignment, the bounded offsets, and the exact
//! deadlines ahead of time; the Rust core owns identity, roles, focus containment, restoration,
//! dismissal, hover deadlines, collision handling, and which side the surface really ended up on;
//! and everything the core decided travels back as one asynchronous `componentchange` event.
//!
//! The resolved placement is the interesting half. A declared `side`/`align` is only a preference:
//! the retained tree flips the side and re-aligns the cross axis whenever the surface does not fit.
//! `Element::report_anchor_placement` publishes the real answer into an application-owned
//! [`AnchorPlacementHandle`] during the paint QuickGUI was already performing, so the binding reads
//! it on the next pass and reports it — the hosted runtime never measures anything itself.

use super::*;

use std::time::Duration;

/// Declared part name of a popover root.
pub(super) const POPOVER_PART: &str = "popover";
/// Declared part name of a popover trigger.
pub(super) const POPOVER_TRIGGER_PART: &str = "popover-trigger";
/// Declared part name of a popover portal boundary.
pub(super) const POPOVER_PORTAL_PART: &str = "popover-portal";
/// Declared part name of a popover positioner.
pub(super) const POPOVER_POSITIONER_PART: &str = "popover-positioner";
/// Declared part name of a popover popup.
pub(super) const POPOVER_POPUP_PART: &str = "popover-popup";
/// Declared part name of a popover arrow.
pub(super) const POPOVER_ARROW_PART: &str = "popover-arrow";
/// Declared part name of a popover's scrollable viewport.
pub(super) const POPOVER_VIEWPORT_PART: &str = "popover-viewport";
/// Declared part name of a popover backdrop.
pub(super) const POPOVER_BACKDROP_PART: &str = "popover-backdrop";
/// Declared part name of a popover title.
pub(super) const POPOVER_TITLE_PART: &str = "popover-title";
/// Declared part name of a popover description.
pub(super) const POPOVER_DESCRIPTION_PART: &str = "popover-description";
/// Declared part name of a popover close control.
pub(super) const POPOVER_CLOSE_PART: &str = "popover-close";

/// Declared part name of a tooltip provider.
pub(super) const TOOLTIP_PROVIDER_PART: &str = "tooltip-provider";
/// Declared part name of a tooltip root.
pub(super) const TOOLTIP_PART: &str = "tooltip";
/// Declared part name of a tooltip trigger.
pub(super) const TOOLTIP_TRIGGER_PART: &str = "tooltip-trigger";
/// Declared part name of a tooltip portal boundary.
pub(super) const TOOLTIP_PORTAL_PART: &str = "tooltip-portal";
/// Declared part name of a tooltip positioner.
pub(super) const TOOLTIP_POSITIONER_PART: &str = "tooltip-positioner";
/// Declared part name of a tooltip popup.
pub(super) const TOOLTIP_POPUP_PART: &str = "tooltip-popup";
/// Declared part name of a tooltip arrow.
pub(super) const TOOLTIP_ARROW_PART: &str = "tooltip-arrow";

/// Whether one declared part belongs to this module.
pub(super) fn owns_popover_part(part: &str) -> bool {
    matches!(
        part,
        POPOVER_PART
            | POPOVER_TRIGGER_PART
            | POPOVER_PORTAL_PART
            | POPOVER_POSITIONER_PART
            | POPOVER_POPUP_PART
            | POPOVER_ARROW_PART
            | POPOVER_VIEWPORT_PART
            | POPOVER_BACKDROP_PART
            | POPOVER_TITLE_PART
            | POPOVER_DESCRIPTION_PART
            | POPOVER_CLOSE_PART
            | TOOLTIP_PROVIDER_PART
            | TOOLTIP_PART
            | TOOLTIP_TRIGGER_PART
            | TOOLTIP_PORTAL_PART
            | TOOLTIP_POSITIONER_PART
            | TOOLTIP_POPUP_PART
            | TOOLTIP_ARROW_PART
    )
}

/// Whether the core owns one part's own Escape and outside-press dismissal.
pub(super) fn popover_part_owns_dismiss(part: &str) -> bool {
    matches!(part, TOOLTIP_POPUP_PART)
}

// ---------------------------------------------------------------------------
// Derived identities
//
// Every identity below is derived from the instance's scope alone, so it resolves before any
// retained state is read and stays stable while the surface opens, closes, and moves.
// ---------------------------------------------------------------------------

fn derived(root: ElementId, rotate: u32, salt: u64) -> ElementId {
    ElementId::new(root.as_u64().rotate_left(rotate) ^ salt)
}

pub(super) fn popover_trigger_id(root: ElementId) -> ElementId {
    derived(root, 29, 0xa076_1d64_78bd_642f)
}

pub(super) fn popover_surface_id(root: ElementId) -> ElementId {
    derived(root, 41, 0xe703_7ed1_a0b4_28db)
}

pub(super) fn tooltip_trigger_id(root: ElementId) -> ElementId {
    derived(root, 11, 0x8ebc_6af0_9c88_c6e3)
}

pub(super) fn tooltip_popup_id(root: ElementId) -> ElementId {
    derived(root, 53, 0x5899_65cc_7537_4cc3)
}

// ---------------------------------------------------------------------------
// Declared property readers
// ---------------------------------------------------------------------------

/// Read one bounded declared millisecond deadline. A malformed value keeps `default`.
fn declared_deadline(node: &NativeNode, key: u16, default: Duration, limit: Duration) -> Duration {
    match node.number(key) {
        Some(value) if value.is_finite() && value >= 0.0 => {
            Duration::from_secs_f32(value / 1000.0).min(limit)
        }
        _ => default,
    }
}

/// Read one declared millisecond deadline that may be absent entirely.
fn optional_deadline(node: &NativeNode, key: u16, limit: Duration) -> Option<Duration> {
    node.number(key)
        .filter(|value| value.is_finite() && *value >= 0.0)
        .map(|value| Duration::from_secs_f32(value / 1000.0).min(limit))
}

/// Read one declared, finite, bounded length. The core clamps again; this only rejects nonsense.
fn declared_length(node: &NativeNode, key: u16, limit: f32) -> Option<f32> {
    node.number(key)
        .filter(|value| value.is_finite())
        .map(|value| value.clamp(-limit, limit))
}

pub(super) fn declared_side(node: &NativeNode) -> AnchorSide {
    match node.string(property::SIDE) {
        Some("top") => AnchorSide::Top,
        Some("left") => AnchorSide::Left,
        Some("right") => AnchorSide::Right,
        _ => AnchorSide::Bottom,
    }
}

pub(super) fn declared_align(node: &NativeNode) -> AnchorAlign {
    match node.string(property::ALIGN) {
        Some("center") => AnchorAlign::Center,
        Some("end") => AnchorAlign::End,
        _ => AnchorAlign::Start,
    }
}

fn side_name(side: AnchorSide) -> &'static str {
    match side {
        AnchorSide::Top => "top",
        AnchorSide::Bottom => "bottom",
        AnchorSide::Left => "left",
        AnchorSide::Right => "right",
    }
}

fn align_name(align: AnchorAlign) -> &'static str {
    match align {
        AnchorAlign::Start => "start",
        AnchorAlign::Center => "center",
        AnchorAlign::End => "end",
    }
}

/// Decode a declared `"x,y"` anchor point. Anything else declares no point at all.
fn declared_anchor_point(node: &NativeNode) -> Option<Point> {
    let value = node.string(property::ANCHOR_POINT)?;
    let (x, y) = value.split_once(',')?;
    let x = x.trim().parse::<f32>().ok()?;
    let y = y.trim().parse::<f32>().ok()?;
    if !x.is_finite() || !y.is_finite() {
        return None;
    }
    Some(Point::new(x, y))
}

/// Resolve a declared anchor node into the identity that node really mounts under.
///
/// The anchor is another node in the same retained tree, so its identity may itself be derived
/// from a declared part; a dangling target declares no anchor at all.
fn declared_anchor_element(node: &NativeNode, tree: &NativeTree) -> Option<ElementId> {
    let anchor_id = node
        .string(property::ANCHOR_TARGET)
        .and_then(|value| value.parse::<u32>().ok())?;
    let anchor = tree.nodes.get(&anchor_id)?;
    Some(native_part_element_id(anchor_id, anchor).unwrap_or(ElementId::new(u64::from(anchor_id))))
}

// ---------------------------------------------------------------------------
// Retained instances
// ---------------------------------------------------------------------------

/// One retained popover instance.
pub(super) struct NativePopoverState {
    pub(super) hover: PopoverHoverState,
    /// Where the retained tree really placed the surface, published during paint.
    pub(super) placement: AnchorPlacementHandle,
    pub(super) side: AnchorSide,
    pub(super) align: AnchorAlign,
    pub(super) side_offset: Option<f32>,
    pub(super) align_offset: f32,
    pub(super) collision_padding: Option<f32>,
    pub(super) sticky: bool,
    pub(super) modal: bool,
    pub(super) anchor_element: Option<ElementId>,
    pub(super) anchor_point: Option<Point>,
    pub(super) open_on_hover: bool,
    declared_open: bool,
    owner: u32,
    listens: bool,
    reported_open: bool,
    reported_placement: Option<serde_json::Value>,
}

impl Default for NativePopoverState {
    fn default() -> Self {
        Self {
            hover: PopoverHoverState::new(),
            placement: AnchorPlacementHandle::new(),
            side: AnchorSide::Bottom,
            align: AnchorAlign::Start,
            side_offset: None,
            align_offset: 0.0,
            collision_padding: None,
            sticky: true,
            modal: false,
            anchor_element: None,
            anchor_point: None,
            open_on_hover: false,
            declared_open: false,
            owner: 0,
            listens: false,
            reported_open: false,
            reported_placement: None,
        }
    }
}

impl NativePopoverState {
    /// Whether the surface is open right now.
    ///
    /// A hover-opening popover lets the core's exact deadline own the value; every other popover
    /// is exactly what JavaScript declared.
    pub(super) fn is_open(&self) -> bool {
        if self.open_on_hover {
            self.hover.is_open()
        } else {
            self.declared_open
        }
    }
}

/// One retained tooltip instance.
pub(super) struct NativeTooltipState {
    pub(super) state: TooltipState,
    declared_open: bool,
    owner: u32,
    listens: bool,
    reported_open: bool,
    reported_placement: Option<serde_json::Value>,
}

impl NativeTooltipState {
    fn new(root: ElementId) -> Self {
        Self {
            state: TooltipState::new(tooltip_trigger_id(root), tooltip_popup_id(root)),
            declared_open: false,
            owner: 0,
            listens: false,
            reported_open: false,
            reported_placement: None,
        }
    }
}

/// Every declared popover and tooltip instance this window retains.
#[derive(Default)]
pub(super) struct NativePopoverStates {
    pub(super) popovers: HashMap<u64, NativePopoverState>,
    pub(super) tooltips: HashMap<u64, NativeTooltipState>,
    /// Retained warm tooltip groups keyed by the provider's own declared scope.
    pub(super) providers: HashMap<u64, TooltipProvider>,
}

/// `openOnHover`, `delay`, and `closeDelay` declared on a `Popover.Trigger`.
type PopoverTriggerDeclaration = (bool, Option<Duration>, Option<Duration>);
/// `delay`, `closeDelay`, and `closeOnClick` declared on a `Tooltip.Trigger`.
type TooltipTriggerDeclaration = (Option<Duration>, Option<Duration>, Option<bool>);
/// The open delay, close delay, and warm-group timeout declared on a `Tooltip.Provider`.
type TooltipProviderDeclaration = (Duration, Duration, Duration);

/// Declarations Base UI places on a part rather than on the component root.
#[derive(Default)]
struct PopoverDeclarations {
    popover_triggers: HashMap<u64, PopoverTriggerDeclaration>,
    tooltip_triggers: HashMap<u64, TooltipTriggerDeclaration>,
    providers: HashMap<u64, TooltipProviderDeclaration>,
}

fn gather_popover_declarations(tree: &NativeTree) -> PopoverDeclarations {
    let mut declarations = PopoverDeclarations::default();
    for (id, node) in &tree.nodes {
        let Some(part) = node.string(property::PART) else {
            continue;
        };
        let key = component_key(*id, node);
        match part {
            POPOVER_TRIGGER_PART
                if declarations.popover_triggers.len() < MAX_COMPONENT_INSTANCES =>
            {
                declarations.popover_triggers.insert(
                    key,
                    (
                        node.boolean(property::OPEN_ON_HOVER).unwrap_or(false),
                        optional_deadline(node, property::DELAY, MAX_POPOVER_HOVER_DELAY),
                        optional_deadline(node, property::CLOSE_DELAY, MAX_POPOVER_HOVER_DELAY),
                    ),
                );
            }
            TOOLTIP_TRIGGER_PART
                if declarations.tooltip_triggers.len() < MAX_COMPONENT_INSTANCES =>
            {
                declarations.tooltip_triggers.insert(
                    key,
                    (
                        optional_deadline(node, property::DELAY, MAX_TOOLTIP_DELAY),
                        optional_deadline(node, property::CLOSE_DELAY, MAX_TOOLTIP_DELAY),
                        node.boolean(property::CLOSE_ON_CLICK),
                    ),
                );
            }
            TOOLTIP_PROVIDER_PART if declarations.providers.len() < MAX_COMPONENT_INSTANCES => {
                declarations.providers.insert(
                    key,
                    (
                        declared_deadline(
                            node,
                            property::DELAY,
                            quickgui::DEFAULT_TOOLTIP_HOVER_DELAY,
                            MAX_TOOLTIP_DELAY,
                        ),
                        declared_deadline(
                            node,
                            property::CLOSE_DELAY,
                            Duration::ZERO,
                            MAX_TOOLTIP_DELAY,
                        ),
                        declared_deadline(
                            node,
                            property::TIMEOUT,
                            quickgui::DEFAULT_TOOLTIP_GROUP_TIMEOUT,
                            MAX_TOOLTIP_GROUP_TIMEOUT,
                        ),
                    ),
                );
            }
            _ => {}
        }
    }
    declarations
}

impl NativePopoverStates {
    /// Reseed every declared popover and tooltip and report what the core did.
    pub(super) fn sync(&mut self, tree: &NativeTree, window: u32, events: &EventQueue) {
        let declarations = gather_popover_declarations(tree);
        // A provider is retained across frames so its warm window survives a re-render; a provider
        // the declaration dropped is forgotten with it.
        self.providers
            .retain(|key, _| declarations.providers.contains_key(key));
        for (key, (delay, close_delay, timeout)) in &declarations.providers {
            // `TooltipProvider::default()` is an all-zero group; a retained provider is always
            // built with `new()` so an undeclared deadline keeps the core's own default.
            let provider = match self.providers.get(key) {
                Some(provider) => provider.clone(),
                None => {
                    let provider = TooltipProvider::new();
                    self.providers.insert(*key, provider.clone());
                    provider
                }
            };
            // Every builder writes through the shared cell, so the retained provider is updated.
            let _ = provider
                .delay(*delay)
                .close_delay(*close_delay)
                .timeout(*timeout);
        }

        let mut popovers = HashSet::new();
        let mut tooltips = HashSet::new();
        for (id, node) in &tree.nodes {
            let Some(part) = node.string(property::PART) else {
                continue;
            };
            let key = component_key(*id, node);
            match part {
                POPOVER_TRIGGER_PART if popovers.len() < MAX_COMPONENT_INSTANCES => {
                    popovers.insert(key);
                    self.sync_popover(key, *id, node, tree);
                }
                TOOLTIP_TRIGGER_PART if tooltips.len() < MAX_COMPONENT_INSTANCES => {
                    tooltips.insert(key);
                    self.sync_tooltip(key, *id, node);
                }
                _ => {}
            }
        }
        self.popovers.retain(|key, _| popovers.contains(key));
        self.tooltips.retain(|key, _| tooltips.contains(key));
        self.report(window, events);
    }

    /// Reseed one declared popover from its trigger.
    ///
    /// The trigger is the one part that is mounted whether the surface is open or closed, so it
    /// carries the whole declaration and owns the retained instance. Every other part finds it
    /// through the shared scope.
    fn sync_popover(&mut self, key: u64, id: u32, node: &NativeNode, tree: &NativeTree) {
        let declared_open = node.boolean(property::OPEN).unwrap_or(false);
        let listens = declares_change(node);
        let open_on_hover = node.boolean(property::OPEN_ON_HOVER).unwrap_or(false);
        let delay = optional_deadline(node, property::DELAY, MAX_POPOVER_HOVER_DELAY);
        let close_delay = optional_deadline(node, property::CLOSE_DELAY, MAX_POPOVER_HOVER_DELAY);

        let retained = self.popovers.entry(key).or_default();
        retained.owner = id;
        retained.listens = listens;
        retained.side = declared_side(node);
        retained.align = declared_align(node);
        retained.side_offset =
            declared_length(node, property::SIDE_OFFSET, MAX_POPOVER_SIDE_OFFSET)
                .or_else(|| declared_length(node, property::ANCHOR_GAP, MAX_POPOVER_SIDE_OFFSET));
        retained.align_offset =
            declared_length(node, property::ALIGN_OFFSET, MAX_POPOVER_ALIGN_OFFSET).unwrap_or(0.0);
        retained.collision_padding = declared_length(
            node,
            property::COLLISION_PADDING,
            MAX_POPOVER_COLLISION_PADDING,
        )
        .or_else(|| {
            declared_length(
                node,
                property::VIEWPORT_MARGIN,
                MAX_POPOVER_COLLISION_PADDING,
            )
        });
        retained.sticky = node.boolean(property::STICKY).unwrap_or(true);
        retained.modal = node.boolean(property::MODAL).unwrap_or(false);
        retained.anchor_element = declared_anchor_element(node, tree);
        retained.anchor_point = declared_anchor_point(node);
        retained.open_on_hover = open_on_hover;

        let mut hover = std::mem::replace(&mut retained.hover, PopoverHoverState::new());
        if let Some(delay) = delay {
            hover = hover.delay(delay);
        }
        if let Some(close_delay) = close_delay {
            hover = hover.close_delay(close_delay);
        }
        hover = hover.hoverable_popup(node.boolean(property::HOVERABLE).unwrap_or(true));
        // A controlled `open` the application committed wins over the hover deadline; the core
        // cancels any outstanding task so a click and a hover can never fight.
        if retained.declared_open != declared_open {
            retained.declared_open = declared_open;
            retained.reported_open = declared_open;
            if declared_open {
                hover.open_now();
            } else {
                hover.close_now();
            }
        }
        retained.hover = hover;
    }

    /// Reseed one declared tooltip from its trigger, which carries the whole declaration.
    fn sync_tooltip(&mut self, key: u64, id: u32, node: &NativeNode) {
        let declared_open = node.boolean(property::OPEN).unwrap_or(false);
        let listens = declares_change(node);
        let provider = node
            .string(property::PROVIDER)
            .filter(|value| !value.is_empty() && value.len() <= MAX_COMPONENT_VALUE_BYTES)
            .map(|value| ElementId::named(value).as_u64())
            .and_then(|key| self.providers.get(&key))
            .cloned();
        let root = ElementId::new(key);
        let retained = self
            .tooltips
            .entry(key)
            .or_insert_with(|| NativeTooltipState::new(root));
        retained.owner = id;
        retained.listens = listens;

        let mut state = std::mem::replace(
            &mut retained.state,
            TooltipState::new(tooltip_trigger_id(root), tooltip_popup_id(root)),
        );
        if let Some(provider) = provider.as_ref() {
            state = state.provider(provider);
        }
        if let Some(delay) = optional_deadline(node, property::DELAY, MAX_TOOLTIP_DELAY) {
            state = state.delay(delay);
        }
        if let Some(close_delay) = optional_deadline(node, property::CLOSE_DELAY, MAX_TOOLTIP_DELAY)
        {
            state = state.close_delay(close_delay);
        }
        state = state
            .hoverable(node.boolean(property::HOVERABLE).unwrap_or(false))
            .close_on_click(node.boolean(property::CLOSE_ON_CLICK).unwrap_or(true))
            .track_cursor_axis(match node.string(property::TRACK_CURSOR_AXIS) {
                Some("x") => TooltipCursorAxis::X,
                Some("y") => TooltipCursorAxis::Y,
                Some("both") => TooltipCursorAxis::Both,
                _ => TooltipCursorAxis::None,
            })
            .placement(anchor_placement(declared_side(node), declared_align(node)));
        if let Some(offset) = declared_length(node, property::SIDE_OFFSET, MAX_TOOLTIP_SIDE_OFFSET)
        {
            state = state.side_offset(offset);
        }
        if let Some(padding) = declared_length(
            node,
            property::COLLISION_PADDING,
            MAX_TOOLTIP_COLLISION_PADDING,
        ) {
            state = state.collision_padding(padding);
        }
        // `disabled` cancels any pending deadline and closes, which is exactly Base UI's contract.
        state = state.disabled(node.boolean(property::DISABLED).unwrap_or(false));
        if retained.declared_open != declared_open {
            retained.declared_open = declared_open;
            retained.reported_open = declared_open;
            if declared_open {
                state.open_now();
            } else {
                state.close_now();
            }
        }
        retained.state = state;
    }

    /// Enqueue one asynchronous change event per instance the core moved since the last pass.
    fn report(&mut self, window: u32, events: &EventQueue) {
        for retained in self.popovers.values_mut() {
            let open = retained.is_open();
            let placement = popover_descriptor_placement(retained);
            let open_changed = open != retained.reported_open;
            let placement_changed = retained.reported_placement.as_ref() != Some(&placement);
            if !open_changed && !placement_changed {
                continue;
            }
            retained.reported_open = open;
            retained.reported_placement = Some(placement.clone());
            if retained.listens {
                let _ = open_changed;
                enqueue_component_change(
                    events,
                    window,
                    retained.owner,
                    serde_json::json!({ "open": open, "placement": placement }),
                );
            }
        }
        for retained in self.tooltips.values_mut() {
            let open = retained.state.is_open();
            let placement = serde_json::json!({
                "side": side_name(retained.state.resolved_side()),
                "align": align_name(retained.state.resolved_align()),
            });
            let open_changed = open != retained.reported_open;
            let placement_changed = retained.reported_placement.as_ref() != Some(&placement);
            if !open_changed && !placement_changed {
                continue;
            }
            retained.reported_open = open;
            retained.reported_placement = Some(placement.clone());
            if retained.listens {
                let _ = open_changed;
                enqueue_component_change(
                    events,
                    window,
                    retained.owner,
                    serde_json::json!({ "open": open, "placement": placement }),
                );
            }
        }
    }
}

/// The placement snapshot one popover reports, taken straight from the core's own part state.
fn popover_descriptor_placement(retained: &NativePopoverState) -> serde_json::Value {
    let state = popover_descriptor(ElementId::new(0), retained).state();
    serde_json::json!({
        "side": side_name(state.side),
        "align": align_name(state.align),
        "anchorHidden": state.anchor_hidden,
        "anchorWidth": state.anchor_width,
        "anchorHeight": state.anchor_height,
        "availableWidth": state.available_width,
        "availableHeight": state.available_height,
    })
}

/// The popover descriptor one declared scope resolves to.
pub(super) fn popover_descriptor(root: ElementId, retained: &NativePopoverState) -> Popover {
    let mut popover = Popover::new(
        popover_trigger_id(root),
        popover_surface_id(root),
        retained.is_open(),
    )
    .placement(anchor_placement(retained.side, retained.align))
    .align_offset(retained.align_offset)
    .sticky(retained.sticky)
    .modal(retained.modal)
    .track_placement(&retained.placement);
    if let Some(offset) = retained.side_offset {
        popover = popover.side_offset(offset);
    }
    if let Some(padding) = retained.collision_padding {
        popover = popover.collision_padding(padding);
    }
    if let Some(anchor) = retained.anchor_element {
        popover = popover.anchor_element(anchor);
    } else if let Some(point) = retained.anchor_point {
        popover = popover.anchor_point(point);
    }
    popover
}

/// A per-instance accessor from the hosted view to one declared popover's hover state.
fn popover_hover_accessor(key: u64) -> StateAccessor<NativeView, PopoverHoverState> {
    StateAccessor::new(move |view: &mut NativeView| {
        &mut view
            .components
            .popovers
            .popovers
            .entry(key)
            .or_default()
            .hover
    })
}

/// A per-instance accessor from the hosted view to one declared tooltip's retained state.
fn tooltip_accessor(key: u64) -> StateAccessor<NativeView, TooltipState> {
    StateAccessor::new(move |view: &mut NativeView| {
        &mut view
            .components
            .popovers
            .tooltips
            .entry(key)
            .or_insert_with(|| NativeTooltipState::new(ElementId::new(key)))
            .state
    })
}

/// The core-derived identity one declared popover or tooltip part mounts under.
pub(super) fn popover_part_element_id(
    part: &str,
    id: u32,
    node: &NativeNode,
    components: &NativeComponentStates,
) -> Option<ElementId> {
    let key = component_key(id, node);
    let root = ElementId::new(key);
    // A stale part whose instance is gone still resolves its identity from the scope alone.
    let popover = || {
        components.popovers.popovers.get(&key).map_or_else(
            || Popover::new(popover_trigger_id(root), popover_surface_id(root), false),
            |retained| popover_descriptor(root, retained),
        )
    };
    Some(match part {
        POPOVER_PART => root,
        POPOVER_TRIGGER_PART => popover_trigger_id(root),
        POPOVER_PORTAL_PART | POPOVER_POSITIONER_PART => popover().positioner_id(),
        POPOVER_POPUP_PART => popover_surface_id(root),
        POPOVER_ARROW_PART => popover().arrow_id(),
        POPOVER_VIEWPORT_PART => popover().viewport_id(),
        POPOVER_BACKDROP_PART => popover().backdrop_id(),
        POPOVER_TITLE_PART => popover().title_id(),
        POPOVER_DESCRIPTION_PART => popover().description_id(),
        POPOVER_CLOSE_PART => popover().close_id(),
        TOOLTIP_PROVIDER_PART | TOOLTIP_PART => root,
        TOOLTIP_TRIGGER_PART => tooltip_trigger_id(root),
        TOOLTIP_PORTAL_PART | TOOLTIP_POSITIONER_PART => components
            .popovers
            .tooltips
            .get(&key)
            .map_or(root, |retained| retained.state.positioner_id()),
        TOOLTIP_POPUP_PART => tooltip_popup_id(root),
        TOOLTIP_ARROW_PART => components
            .popovers
            .tooltips
            .get(&key)
            .map_or(root, |retained| retained.state.arrow_id()),
        _ => return None,
    })
}

/// Apply one declared popover or tooltip part.
///
/// `None` means the core decided this part is not mounted at all — every surface part of a closed
/// popover or tooltip.
pub(super) fn apply_popover_part(
    element: Element,
    part: &str,
    id: u32,
    node: &NativeNode,
    components: &NativeComponentStates,
    cx: &mut ViewContext<'_, NativeView>,
    listeners_enabled: bool,
) -> Option<Element> {
    let key = component_key(id, node);
    let root = ElementId::new(key);
    match part {
        // -------------------------------------------------------------------
        // Popover
        // -------------------------------------------------------------------
        POPOVER_PART => Some(element),
        POPOVER_TRIGGER_PART => {
            let Some(retained) = components.popovers.popovers.get(&key) else {
                return Some(element);
            };
            let popover = popover_descriptor(root, retained);
            if !listeners_enabled || !retained.open_on_hover {
                return Some(popover.trigger_part(element));
            }
            Some(retained.hover.trigger_part_with(
                cx,
                popover,
                popover_hover_accessor(key),
                element,
            ))
        }
        POPOVER_PORTAL_PART | POPOVER_POSITIONER_PART => {
            let retained = components.popovers.popovers.get(&key)?;
            if !retained.is_open() {
                return None;
            }
            Some(
                popover_descriptor(root, retained)
                    .tracked_positioner_part(element, &retained.placement),
            )
        }
        POPOVER_POPUP_PART => {
            let retained = components.popovers.popovers.get(&key)?;
            if !retained.is_open() {
                return None;
            }
            let popover = popover_descriptor(root, retained);
            if !listeners_enabled || !retained.open_on_hover {
                return Some(popover.popover_part(element));
            }
            Some(
                retained
                    .hover
                    .popup_part_with(cx, popover, popover_hover_accessor(key), element),
            )
        }
        POPOVER_ARROW_PART => {
            let retained = components.popovers.popovers.get(&key)?;
            if !retained.is_open() {
                return None;
            }
            Some(popover_descriptor(root, retained).arrow_part(element))
        }
        POPOVER_VIEWPORT_PART => {
            let retained = components.popovers.popovers.get(&key)?;
            if !retained.is_open() {
                return None;
            }
            Some(popover_descriptor(root, retained).viewport_part(element))
        }
        POPOVER_BACKDROP_PART => {
            let retained = components.popovers.popovers.get(&key)?;
            if !retained.is_open() {
                return None;
            }
            Some(popover_descriptor(root, retained).backdrop_part(element))
        }
        POPOVER_TITLE_PART => {
            let retained = components.popovers.popovers.get(&key)?;
            Some(popover_descriptor(root, retained).title_part(element))
        }
        POPOVER_DESCRIPTION_PART => {
            let retained = components.popovers.popovers.get(&key)?;
            Some(popover_descriptor(root, retained).description_part(element))
        }
        POPOVER_CLOSE_PART => {
            let retained = components.popovers.popovers.get(&key)?;
            Some(
                popover_descriptor(root, retained).close_part(
                    node.string(property::ACCESSIBILITY_LABEL)
                        .unwrap_or("Close"),
                    element,
                ),
            )
        }

        // -------------------------------------------------------------------
        // Tooltip
        // -------------------------------------------------------------------
        TOOLTIP_PROVIDER_PART | TOOLTIP_PART => Some(element),
        TOOLTIP_TRIGGER_PART => {
            let Some(retained) = components.popovers.tooltips.get(&key) else {
                return Some(element);
            };
            if !listeners_enabled {
                return Some(element.id(retained.state.trigger_id()));
            }
            Some(
                retained
                    .state
                    .trigger_part_with(cx, tooltip_accessor(key), element),
            )
        }
        TOOLTIP_PORTAL_PART | TOOLTIP_POSITIONER_PART => {
            let retained = components.popovers.tooltips.get(&key)?;
            if !retained.state.is_open() {
                return None;
            }
            Some(retained.state.positioner_part(element))
        }
        TOOLTIP_POPUP_PART => {
            let retained = components.popovers.tooltips.get(&key)?;
            if !retained.state.is_open() {
                return None;
            }
            if !listeners_enabled {
                return Some(element.id(retained.state.popup_id()));
            }
            Some(
                retained
                    .state
                    .popup_part_with(cx, tooltip_accessor(key), element),
            )
        }
        TOOLTIP_ARROW_PART => {
            let retained = components.popovers.tooltips.get(&key)?;
            if !retained.state.is_open() {
                return None;
            }
            Some(retained.state.arrow_part(element))
        }
        _ => Some(element),
    }
}
