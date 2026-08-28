use std::{
    any::{Any, TypeId},
    collections::{HashMap, HashSet},
    ops::Range,
    sync::Arc,
    time::{Duration, Instant},
};

use accesskit::{
    Action, Affine, AutoComplete as NativeAccessibilityAutoComplete,
    HasPopup as AccessibilityHasPopup, Invalid as AccessibilityInvalid, Live,
    Node as AccessibilityNode, NodeId as AccessibilityNodeId,
    Orientation as NativeAccessibilityOrientation, Rect as AccessibilityRect, Role,
    SortDirection as NativeAccessibilitySortDirection, TextPosition, TextSelection,
    Toggled as AccessibilityToggled, Tree, TreeId, TreeUpdate, Vec2 as AccessibilityVector,
};
use taffy::{
    geometry::Size as TaffySize,
    prelude::{AvailableSpace, NodeId, TaffyTree},
    style::{CompactLength, Dimension, Overflow, Style as TaffyStyle},
};
use thiserror::Error;
use unicode_segmentation::UnicodeSegmentation;

use crate::{
    AccessibilityAutoComplete, AccessibilityPopup, AccessibilityRole, AccessibilitySortDirection,
    AnchorPlacement, AnimatedImage, AppRegion, BoxShadow, Canvas, Color, CursorStyle,
    CustomShaderPrimitive, DispatchPhase, Element, ElementId, ImagePrimitive, Insets, Interpolate,
    KeyContext, MAX_BOX_SHADOWS_PER_ELEMENT, MAX_CONTAINER_QUERIES_PER_WINDOW,
    MAX_CONTAINER_QUERY_DEPTH, MAX_DECLARATIVE_ANIMATIONS_PER_WINDOW,
    MAX_STYLE_TRANSITIONS_PER_WINDOW, MAX_TOOLTIPS_PER_WINDOW, MouseButton, ObjectFit, Path,
    PathPrimitive, Point, Quad, Rect, Scene, Shadow, Size, SvgPrimitive, TextHighlight, TextId,
    TextRun, TextStyle, TextWrap, ToggleState, Tooltip, Transition, TransitionProperties,
    UserSelect, Vector,
    action::ActionListenerBinding,
    animated_image::AnimatedImageId,
    animation::{Animation, ElementAnimation},
    element::{
        AccessibilityOrientation, AnchorTarget, DismissPolicy, DropPredicateCallback, ElementKind,
        ElementStateStyle, ImageResolution, KeyListenerBinding, KeyListenerKind,
        MouseListenerBinding, MouseListenerKey, MouseListenerKind,
    },
    event::{
        FormField, FormSubmitEvent, MAX_FORM_FIELDS, MAX_VALIDATION_ISSUES,
        MAX_VALIDATION_MESSAGE_BYTES, ValidationIssue, ValidationReport,
    },
    image::fit_image,
    renderer::{TextLayoutEngine, TextPaintKind, TextPaintRect},
    scene::{PaintLayerKey, WavyUnderline},
    spring::{ElementSpring, SpringConfig, SpringPlayback, SpringState},
    text_input::{
        TextInputState, accessibility_byte_index, accessibility_character_index,
        accessibility_character_index_from_lengths, boundary_at_or_before,
        selectable_character_lengths,
    },
    virtual_list::{VirtualScrollHandle, VirtualScrollMount},
};

#[cfg(target_os = "macos")]
use crate::{ScenePlane, native_view::NativeViewPlacement};

const ACCESSIBILITY_ROOT_ID: AccessibilityNodeId = AccessibilityNodeId(u64::MAX);
const SCROLLBAR_AUTO_HIDE_DELAY: Duration = Duration::from_millis(900);
const STATIC_TEXT_MULTI_CLICK_INTERVAL: Duration = Duration::from_millis(500);
const STATIC_TEXT_MULTI_CLICK_DISTANCE: f32 = 4.0;

/// Maximum retained ancestor depth traversed by one targeted desktop mouse event.
pub const MAX_MOUSE_EVENT_PATH: usize = 256;

/// Maximum retained ancestor depth traversed by one focused action or key event.
pub const MAX_FOCUSED_EVENT_PATH: usize = 256;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct MouseHoverChange {
    pub(crate) key: MouseListenerKey,
    pub(crate) hovered: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct TabNavigationTarget {
    pub(crate) id: ElementId,
    pub(crate) activate: bool,
}

/// Maximum retained hit regions consulted synchronously by one native macOS drag.
///
/// Regions are copied topmost-first, so reaching the bound conservatively rejects only targets
/// below the retained stack instead of allowing a drag through an omitted pointer blocker.
#[cfg(target_os = "macos")]
pub(crate) const MAX_EXTERNAL_DROP_HIT_REGIONS: usize = 4_096;

/// Maximum exact-type listener acceptances copied into one native macOS drag snapshot.
#[cfg(target_os = "macos")]
pub(crate) const MAX_EXTERNAL_DROP_ACCEPTANCES: usize = 8_192;

/// Maximum UTF-8 payload materialized by one immutable-text copy operation.
///
/// The mounted strings stay shared; this limit only bounds the temporary joined clipboard value.
pub const MAX_STATIC_TEXT_COPY_BYTES: usize = 8 * 1024 * 1024;

pub(crate) enum FormAttempt {
    Valid(FormSubmitEvent),
    Invalid(ValidationReport),
}

struct ValidationAnnouncement {
    form: ElementId,
    node: AccessibilityNodeId,
    message: Arc<str>,
}

fn static_selection_color() -> Color {
    Color::rgba8(48, 120, 196, 105)
}

#[derive(Debug, Error)]
pub(crate) enum UiError {
    #[error("flexbox layout failed: {0}")]
    Layout(#[from] taffy::TaffyError),
    #[error("element id {0:?} appears more than once in the same view")]
    DuplicateId(ElementId),
    #[error("element id {0:?} is reserved by the accessibility root")]
    ReservedId(ElementId),
    #[error("animation id {0:?} appears more than once in the same view")]
    DuplicateAnimationId(ElementId),
    #[error(
        "a window cannot retain more than {MAX_DECLARATIVE_ANIMATIONS_PER_WINDOW} declarative animations"
    )]
    TooManyDeclarativeAnimations,
    #[error(
        "a window cannot retain more than {MAX_STYLE_TRANSITIONS_PER_WINDOW} style transitions"
    )]
    TooManyStyleTransitions,
    #[error("anchored element {element:?} refers to missing element {anchor:?}")]
    MissingAnchor {
        element: ElementId,
        anchor: ElementId,
    },
    #[error("a window cannot retain more than {MAX_TOOLTIPS_PER_WINDOW} tooltips")]
    TooManyTooltips,
    #[error(
        "a window cannot retain more than {MAX_CONTAINER_QUERIES_PER_WINDOW} container queries"
    )]
    TooManyContainerQueries,
    #[error("container queries cannot nest deeper than {MAX_CONTAINER_QUERY_DEPTH} levels")]
    ContainerQueryDepthExceeded,
    #[error("container query layout did not converge within the bounded nesting limit")]
    ContainerQueryDidNotConverge,
    #[cfg(target_os = "macos")]
    #[error("native AppKit view {0:?} must stay in the base composition plane")]
    NativeViewInOverlay(ElementId),
}

#[derive(Clone)]
enum MeasureContext {
    Text {
        id: TextId,
        content: Arc<str>,
        style: TextStyle,
        highlights: Option<Arc<[TextHighlight]>>,
    },
    Image {
        intrinsic: Size,
    },
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct HitRegion {
    pub(crate) id: ElementId,
    pub(crate) bounds: Rect,
    pub(crate) clip: Rect,
    pub(crate) clickable: bool,
    pub(crate) pointer_listener: bool,
    pub(crate) drag_source: bool,
    pub(crate) drop_target: bool,
    pub(crate) focusable: bool,
    pub(crate) cursor_style: Option<CursorStyle>,
    cursor_states: CursorStateStyles,
    pub(crate) stateful: bool,
    pub(crate) blocks_pointer: bool,
    pub(crate) app_region: Option<AppRegion>,
    pub(crate) order: PaintOrder,
}

#[derive(Clone, Copy, Debug, Default)]
struct CursorStateStyles {
    hover: Option<CursorStyle>,
    active: Option<CursorStyle>,
    focus: Option<CursorStyle>,
    invalid: Option<CursorStyle>,
    dragging: Option<CursorStyle>,
    drag_over: Option<CursorStyle>,
}

impl HitRegion {
    fn contains(self, point: Point) -> bool {
        self.bounds.contains(point) && self.clip.contains(point)
    }
}

#[cfg(target_os = "macos")]
#[derive(Clone)]
enum ExternalDropAcceptance {
    Always,
    Predicate(DropPredicateCallback),
}

#[cfg(target_os = "macos")]
impl ExternalDropAcceptance {
    fn accepts(&self, value: &dyn Any) -> bool {
        match self {
            Self::Always => true,
            // Application predicates must never unwind through an Objective-C drag callback.
            Self::Predicate(predicate) => {
                std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| predicate(value)))
                    .unwrap_or(false)
            }
        }
    }
}

#[cfg(target_os = "macos")]
#[derive(Clone)]
struct ExternalDropHitRegion {
    id: ElementId,
    bounds: Rect,
    clip: Rect,
    drop_target: bool,
    blocks_pointer: bool,
    pointer_listener: bool,
}

#[cfg(target_os = "macos")]
impl ExternalDropHitRegion {
    fn contains(&self, point: Point) -> bool {
        self.bounds.contains(point) && self.clip.contains(point)
    }
}

/// Reusable, bounded copy of the retained drop topology used by AppKit's synchronous callbacks.
#[cfg(target_os = "macos")]
#[derive(Default)]
pub(crate) struct ExternalDropSnapshot {
    regions: Vec<ExternalDropHitRegion>,
    acceptances: HashMap<(ElementId, TypeId), ExternalDropAcceptance>,
    target_ids: HashSet<ElementId>,
    regions_truncated: bool,
    acceptances_truncated: bool,
}

#[cfg(target_os = "macos")]
impl ExternalDropSnapshot {
    pub(crate) fn new() -> Self {
        Self {
            regions: Vec::with_capacity(64),
            acceptances: HashMap::with_capacity(64),
            target_ids: HashSet::with_capacity(32),
            regions_truncated: false,
            acceptances_truncated: false,
        }
    }

    pub(crate) fn clear(&mut self) {
        self.regions.clear();
        self.acceptances.clear();
        self.target_ids.clear();
        self.regions_truncated = false;
        self.acceptances_truncated = false;
    }

    pub(crate) fn offer_target_at<'a, I>(
        &self,
        point: Point,
        offers: I,
    ) -> Option<(ElementId, usize)>
    where
        I: Iterator<Item = (TypeId, &'a dyn Any)> + Clone,
    {
        // An omitted acceptance could belong to a higher target than one retained below it.
        // Reject the complete native offer rather than allowing it to reach through that target.
        if self.acceptances_truncated {
            return None;
        }
        for region in &self.regions {
            if !region.contains(point) {
                continue;
            }
            if region.drop_target {
                for (index, (value_type, value)) in offers.clone().enumerate() {
                    if self
                        .acceptances
                        .get(&(region.id, value_type))
                        .is_some_and(|acceptance| acceptance.accepts(value))
                    {
                        return Some((region.id, index));
                    }
                }
            }
            if region.blocks_pointer || region.pointer_listener {
                return None;
            }
        }
        None
    }

    #[cfg(test)]
    pub(crate) fn target_at(
        &self,
        point: Point,
        value_type: TypeId,
        value: &dyn Any,
    ) -> Option<ElementId> {
        self.offer_target_at(point, std::iter::once((value_type, value)))
            .map(|(id, _)| id)
    }

    #[cfg(test)]
    fn is_truncated(&self) -> bool {
        self.regions_truncated || self.acceptances_truncated
    }
}

#[derive(Clone, Copy, Debug)]
struct ScrollRegion {
    id: ElementId,
    /// Pointer/wheel hit bounds for this scrolling container.
    bounds: Rect,
    /// Geometry used by the overlay scrollbar; normally equal to `bounds`.
    scrollbar_bounds: Rect,
    clip: Rect,
    max_offset: Vector,
    /// Whether changing this offset must update a bound virtual list and rebuild the view.
    virtual_scroll: bool,
    order: PaintOrder,
    scrollbar_order: PaintOrder,
}

#[derive(Clone, Debug)]
struct RetainedVirtualScroll {
    handle: VirtualScrollHandle,
    measurement_revision: u64,
    mount: VirtualScrollMount,
}

impl RetainedVirtualScroll {
    fn update_from_input(&self, offset_y: f32, viewport_height: f32) -> bool {
        self.handle.set_offset_from_input(offset_y);
        !self.mount.retains_viewport(offset_y, viewport_height)
    }
}

#[derive(Clone, Copy, Debug)]
struct ScrollbarDrag {
    id: ElementId,
    pointer_origin_y: f32,
    scroll_origin_y: f32,
}

#[derive(Clone, Copy, Debug, Default)]
struct ScrollbarState {
    hovered: bool,
    dragging: bool,
    visible_until: Option<Instant>,
}

impl ScrollbarState {
    fn visible(self, now: Instant) -> bool {
        self.hovered || self.dragging || self.visible_until.is_some_and(|deadline| deadline > now)
    }
}

#[derive(Clone, Copy, Debug)]
struct VerticalScrollbarGeometry {
    track: Rect,
    thumb: Rect,
    travel: f32,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(crate) struct PaintOrder {
    pub(crate) layer: PaintLayerKey,
    pub(crate) source: usize,
}

#[derive(Clone, Copy, Debug)]
struct DismissRegion {
    id: ElementId,
    bounds: Rect,
    clip: Rect,
    policy: DismissPolicy,
    restore_focus: Option<ElementId>,
    order: PaintOrder,
}

impl DismissRegion {
    fn contains(self, point: Point) -> bool {
        self.bounds.contains(point) && self.clip.contains(point)
    }
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct DismissRequest {
    pub id: ElementId,
    pub restore_focus: Option<ElementId>,
}

#[derive(Clone)]
struct TextInputRegion {
    id: ElementId,
    bounds: Rect,
    clip: Rect,
    content: Arc<str>,
    password: Option<PasswordDisplay>,
    highlights: Option<Arc<[TextHighlight]>>,
    style: TextStyle,
    scroll: Vector,
    max_scroll: Vector,
    caret_bounds: Rect,
}

const PASSWORD_MASK: &str = "•";

/// A display-only password projection with exact source/display boundary translation.
///
/// The retained editor always owns the real string. Rendering one mask glyph per grapheme keeps
/// Unicode cursor and selection behavior web-like without placing the secret in a scene command.
#[derive(Clone)]
struct PasswordDisplay {
    content: Arc<str>,
    source_boundaries: Arc<[usize]>,
}

impl PasswordDisplay {
    fn new(source: &str) -> Self {
        let graphemes = source.grapheme_indices(true).collect::<Vec<_>>();
        let mut masked = String::with_capacity(graphemes.len() * PASSWORD_MASK.len());
        let mut source_boundaries = Vec::with_capacity(graphemes.len() + 1);
        source_boundaries.push(0);
        for (start, grapheme) in graphemes {
            masked.push_str(PASSWORD_MASK);
            source_boundaries.push(start + grapheme.len());
        }
        Self {
            content: Arc::from(masked),
            source_boundaries: Arc::from(source_boundaries),
        }
    }

    fn display_index(&self, source_index: usize) -> usize {
        self.source_boundaries
            .partition_point(|boundary| *boundary <= source_index)
            .saturating_sub(1)
            .min(self.source_boundaries.len().saturating_sub(1))
            * PASSWORD_MASK.len()
    }

    fn source_index(&self, display_index: usize) -> usize {
        let grapheme = (display_index / PASSWORD_MASK.len())
            .min(self.source_boundaries.len().saturating_sub(1));
        self.source_boundaries[grapheme]
    }
}

#[derive(Clone, Debug)]
struct SelectableTextEntry {
    id: ElementId,
    content: Arc<str>,
    character_lengths: Arc<[u8]>,
}

#[derive(Clone)]
struct SelectableTextRegion {
    document_index: usize,
    bounds: Rect,
    clip: Rect,
    style: TextStyle,
    highlights: Option<Arc<[TextHighlight]>>,
    order: PaintOrder,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct StaticTextPosition {
    id: ElementId,
    offset: usize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct StaticTextSelection {
    anchor: StaticTextPosition,
    focus: StaticTextPosition,
}

#[derive(Clone, Copy, Debug)]
struct StaticTextGesture {
    origin: Point,
    moved: bool,
    unit: StaticTextSelectionUnit,
    base_start: StaticTextPosition,
    base_end: StaticTextPosition,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum StaticTextSelectionUnit {
    Character,
    Word,
    Line,
}

#[derive(Clone, Copy, Debug)]
struct StaticTextClick {
    position: Point,
    id: ElementId,
    at: Instant,
    count: u8,
}

#[derive(Clone, Debug)]
pub(crate) struct InputChange {
    pub id: ElementId,
    pub value: Arc<str>,
}

#[derive(Clone, Debug, Default)]
pub(crate) struct InputResult {
    pub repaint: bool,
    pub change: Option<InputChange>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct ScrollResult {
    pub changed: bool,
    pub view_dirty: bool,
}

#[derive(Clone, Copy, Debug)]
struct ScrollEndState {
    revision: u64,
    previous_max_y: f32,
}

pub(crate) struct UiTree {
    root: Option<Element>,
    taffy: TaffyTree<MeasureContext>,
    root_node: Option<NodeId>,
    mounted_state_dirty: bool,
    seen_ids: HashSet<ElementId>,
    /// IDs whose complete ancestor chain participates in layout (`display != none`).
    displayed_ids: HashSet<ElementId>,
    /// Displayed IDs whose complete ancestor chain is painted (`visibility != hidden`).
    visible_ids: HashSet<ElementId>,
    scroll_offsets: HashMap<ElementId, Vector>,
    scroll_end_states: HashMap<ElementId, ScrollEndState>,
    virtual_scroll_handles: HashMap<ElementId, RetainedVirtualScroll>,
    natural_bounds: HashMap<ElementId, Rect>,
    element_bounds: HashMap<ElementId, Rect>,
    hit_regions: Vec<HitRegion>,
    drop_predicates: HashMap<(ElementId, TypeId), DropPredicateCallback>,
    context_menu_ids: HashSet<ElementId>,
    mouse_listener_bindings: Vec<MouseListenerBinding>,
    mouse_listener_ranges: HashMap<ElementId, Range<usize>>,
    mouse_listener_elements: Vec<ElementId>,
    key_listener_bindings: Vec<KeyListenerBinding>,
    key_listener_ranges: HashMap<ElementId, Range<usize>>,
    action_listener_bindings: Vec<ActionListenerBinding>,
    action_listener_ranges: HashMap<ElementId, Range<usize>>,
    scroll_wheel_ids: HashSet<ElementId>,
    touch_ids: HashSet<ElementId>,
    mouse_pressure_ids: HashSet<ElementId>,
    pinch_ids: HashSet<ElementId>,
    rotation_ids: HashSet<ElementId>,
    smart_magnify_ids: HashSet<ElementId>,
    tooltips: HashMap<ElementId, Tooltip>,
    pointer_tooltip: Option<ElementId>,
    hovered_tooltip: Option<ElementId>,
    pending_tooltip: Option<PendingTooltip>,
    visible_tooltip: Option<ElementId>,
    tooltip_overlay: Option<TooltipOverlay>,
    scroll_regions: Vec<ScrollRegion>,
    scrollbar_drag: Option<ScrollbarDrag>,
    scrollbar_states: HashMap<ElementId, ScrollbarState>,
    hovered_scrollbar: Option<ElementId>,
    dismiss_regions: Vec<DismissRegion>,
    #[cfg(target_os = "macos")]
    native_views: Vec<NativeViewPlacement>,
    text_input_regions: Vec<TextInputRegion>,
    text_inputs: HashMap<ElementId, TextInputState>,
    selectable_texts: Vec<SelectableTextEntry>,
    selectable_text_indices: HashMap<ElementId, usize>,
    selectable_text_regions: Vec<SelectableTextRegion>,
    static_text_selection: Option<StaticTextSelection>,
    static_text_gesture: Option<StaticTextGesture>,
    last_static_text_click: Option<StaticTextClick>,
    accessibility_text_ids: HashMap<ElementId, AccessibilityNodeId>,
    next_accessibility_text_id: u64,
    animations: HashMap<ElementId, AnimationPlayback>,
    animation_ids: HashSet<ElementId>,
    declarative_animations: HashMap<ElementId, DeclarativeAnimationPlayback>,
    declarative_time_animation_ids: HashSet<ElementId>,
    declarative_springs: HashMap<ElementId, DeclarativeSpringPlayback>,
    declarative_spring_ids: HashSet<ElementId>,
    /// Shared namespace across time animations and springs for one declaration.
    declarative_animation_ids: HashSet<ElementId>,
    declarative_animation_frame_requested: bool,
    declarative_animation_deadline: Option<Instant>,
    style_transitions: HashMap<ElementId, StyleTransitionPlayback>,
    style_transition_ids: HashSet<ElementId>,
    style_transition_frame_requested: bool,
    animation_epoch: Instant,
    animations_enabled: bool,
    reduce_motion: bool,
    selecting_input: Option<ElementId>,
    hovered: HashSet<ElementId>,
    hover_scratch: HashSet<ElementId>,
    mouse_hover_path: Vec<ElementId>,
    mouse_hover_path_scratch: Vec<ElementId>,
    pending_mouse_hover_changes: Vec<MouseHoverChange>,
    pressed: Option<ElementId>,
    dragging: Option<ElementId>,
    external_drag_active: bool,
    drag_over: Option<ElementId>,
    drag_preview: Option<DragPreview>,
    focused: Option<ElementId>,
    active_focus_trap: Option<ElementId>,
    focusable_ids: HashSet<ElementId>,
    clickable_ids: HashSet<ElementId>,
    activation_targets: HashMap<ElementId, ElementId>,
    invalid_ids: HashSet<ElementId>,
    form_ids: HashSet<ElementId>,
    form_submitter_ids: HashSet<ElementId>,
    focus_order: Vec<ElementId>,
    parents: HashMap<ElementId, ElementId>,
    key_contexts: HashMap<ElementId, KeyContext>,
    focus_initialized: bool,
    validation_announcement: Option<ValidationAnnouncement>,
    viewport: Size,
    scale_factor: f32,
}

struct AnimationPlayback {
    asset: AnimatedImage,
    asset_id: AnimatedImageId,
    frame_index: usize,
    elapsed: Duration,
    last_advanced_at: Instant,
    active: bool,
    seen: bool,
    completed: bool,
}

#[derive(Clone, Copy, Debug)]
struct DeclarativeAnimationSample {
    animation_ix: usize,
    value: f32,
    request_frame: bool,
    deadline: Option<Instant>,
}

#[derive(Clone, Debug)]
struct DeclarativeAnimationPlayback {
    animation_ix: usize,
    elapsed: Duration,
    last_advanced_at: Instant,
    next_frame_at: Option<Instant>,
    scheduled_stage: usize,
    last_value: f32,
    completed: bool,
    active: bool,
}

#[derive(Clone, Copy, Debug)]
struct DeclarativeSpringSample {
    value: f32,
    request_frame: bool,
}

#[derive(Clone, Debug)]
struct DeclarativeSpringPlayback {
    state: SpringState,
    target: f32,
    config: SpringConfig,
    initial: f32,
    playback: SpringPlayback,
    updated_at: Instant,
    active: bool,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct TransitionShadowList {
    values: [BoxShadow; MAX_BOX_SHADOWS_PER_ELEMENT],
    len: u8,
}

impl TransitionShadowList {
    fn empty() -> Self {
        Self {
            values: [neutral_transition_shadow(false); MAX_BOX_SHADOWS_PER_ELEMENT],
            len: 0,
        }
    }

    fn from_slice(shadows: &[BoxShadow]) -> Self {
        debug_assert!(shadows.len() <= MAX_BOX_SHADOWS_PER_ELEMENT);
        let mut list = Self::empty();
        for (index, shadow) in shadows
            .iter()
            .copied()
            .take(MAX_BOX_SHADOWS_PER_ELEMENT)
            .enumerate()
        {
            list.values[index] = sane_transition_shadow(shadow);
            list.len += 1;
        }
        list
    }

    fn as_slice(&self) -> &[BoxShadow] {
        &self.values[..usize::from(self.len)]
    }

    fn interpolate(from: Self, to: Self, phase: f32) -> Self {
        if phase == 0.0 {
            return from;
        }
        if phase == 1.0 {
            return to;
        }
        let len = usize::from(from.len.max(to.len));
        let mut result = Self::empty();
        result.len = len as u8;
        for index in 0..len {
            let from_shadow = if index < usize::from(from.len) {
                from.values[index]
            } else {
                neutral_transition_shadow(to.values[index].is_inset())
            };
            let to_shadow = if index < usize::from(to.len) {
                to.values[index]
            } else {
                neutral_transition_shadow(from.values[index].is_inset())
            };
            result.values[index] = interpolate_transition_shadow(from_shadow, to_shadow, phase);
        }
        result
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct TransitionPaintStyle {
    background: Color,
    border_color: Color,
    border_width: f32,
    radius: f32,
    opacity: f32,
    text_color: Option<Color>,
    text_fallback: Color,
    shadows: TransitionShadowList,
}

impl TransitionPaintStyle {
    fn interpolate(from: Self, to: Self, phase: f32, properties: TransitionProperties) -> Self {
        let selected = |property| properties.contains(property);
        Self {
            background: if selected(TransitionProperties::BACKGROUND) {
                Color::interpolate(from.background, to.background, phase)
            } else {
                to.background
            },
            border_color: if selected(TransitionProperties::BORDER_COLOR) {
                Color::interpolate(from.border_color, to.border_color, phase)
            } else {
                to.border_color
            },
            border_width: if selected(TransitionProperties::BORDER_WIDTH) {
                f32::interpolate(from.border_width, to.border_width, phase).max(0.0)
            } else {
                to.border_width
            },
            radius: if selected(TransitionProperties::BORDER_RADIUS) {
                f32::interpolate(from.radius, to.radius, phase).max(0.0)
            } else {
                to.radius
            },
            opacity: if selected(TransitionProperties::OPACITY) {
                f32::interpolate(from.opacity, to.opacity, phase).clamp(0.0, 1.0)
            } else {
                to.opacity
            },
            text_color: if selected(TransitionProperties::TEXT_COLOR) {
                if phase == 0.0 {
                    from.text_color
                } else if phase == 1.0 {
                    to.text_color
                } else {
                    Some(Color::interpolate(
                        from.text_color.unwrap_or(from.text_fallback),
                        to.text_color.unwrap_or(to.text_fallback),
                        phase,
                    ))
                }
            } else {
                to.text_color
            },
            text_fallback: if selected(TransitionProperties::TEXT_COLOR) {
                Color::interpolate(from.text_fallback, to.text_fallback, phase)
            } else {
                to.text_fallback
            },
            shadows: if selected(TransitionProperties::BOX_SHADOW) {
                TransitionShadowList::interpolate(from.shadows, to.shadows, phase)
            } else {
                to.shadows
            },
        }
    }

    fn differs_for(self, other: Self, properties: TransitionProperties) -> bool {
        (properties.contains(TransitionProperties::BACKGROUND)
            && self.background != other.background)
            || (properties.contains(TransitionProperties::BORDER_COLOR)
                && self.border_color != other.border_color)
            || (properties.contains(TransitionProperties::BORDER_WIDTH)
                && self.border_width != other.border_width)
            || (properties.contains(TransitionProperties::BORDER_RADIUS)
                && self.radius != other.radius)
            || (properties.contains(TransitionProperties::OPACITY) && self.opacity != other.opacity)
            || (properties.contains(TransitionProperties::TEXT_COLOR)
                && (self.text_color.unwrap_or(self.text_fallback)
                    != other.text_color.unwrap_or(other.text_fallback)))
            || (properties.contains(TransitionProperties::BOX_SHADOW)
                && self.shadows != other.shadows)
    }
}

#[derive(Clone, Debug)]
struct StyleTransitionPlayback {
    from: TransitionPaintStyle,
    current: TransitionPaintStyle,
    target: TransitionPaintStyle,
    config: Transition,
    elapsed: Duration,
    last_advanced_at: Instant,
    next_frame_at: Option<Instant>,
    in_progress: bool,
    active: bool,
}

struct StyleTransitionPaintContext<'a> {
    playbacks: Option<&'a mut HashMap<ElementId, StyleTransitionPlayback>>,
    request_frame: &'a mut bool,
    enabled: bool,
    reduce_motion: bool,
    now: Instant,
}

/// A detached declaration owns its template and motion clock independently from the application
/// view. Dropping the enclosing tooltip or drag preview therefore drops every playback and exact
/// deadline at the same time.
struct DetachedMotionState {
    template: Element,
    animations: HashMap<ElementId, DeclarativeAnimationPlayback>,
    time_animation_ids: HashSet<ElementId>,
    springs: HashMap<ElementId, DeclarativeSpringPlayback>,
    spring_ids: HashSet<ElementId>,
    motion_ids: HashSet<ElementId>,
    frame_requested: bool,
    deadline: Option<Instant>,
    needs_resolve: bool,
    animation_epoch: Instant,
}

/// Retained layout and image state for a pointer-passive tree painted outside the main view.
struct DetachedTree {
    motion: DetachedMotionState,
    root: Element,
    taffy: TaffyTree<MeasureContext>,
    root_node: Option<NodeId>,
    root_seed: ElementId,
    seen_ids: HashSet<ElementId>,
    input_ids: HashSet<ElementId>,
    animation_ids: HashSet<ElementId>,
    scroll_offsets: HashMap<ElementId, Vector>,
    scroll_end_states: HashMap<ElementId, ScrollEndState>,
    natural_bounds: HashMap<ElementId, Rect>,
    paint_bounds: HashMap<ElementId, Rect>,
    text_inputs: HashMap<ElementId, TextInputState>,
    animations: HashMap<ElementId, AnimationPlayback>,
}

struct DragPreview {
    tree: DetachedTree,
    cursor_offset: Point,
    position: Point,
}

#[derive(Clone, Copy, Debug)]
struct PendingTooltip {
    target: ElementId,
    show_at: Instant,
}

struct TooltipOverlay {
    target: ElementId,
    content_identity: *const Element,
    placement: AnchorPlacement,
    gap: f32,
    viewport_margin: f32,
    tree: DetachedTree,
}

impl AnimationPlayback {
    fn new(asset: AnimatedImage, now: Instant) -> Self {
        Self {
            asset_id: asset.id(),
            asset,
            frame_index: 0,
            elapsed: Duration::ZERO,
            last_advanced_at: now,
            active: false,
            seen: false,
            completed: false,
        }
    }

    fn activate(&mut self, now: Instant, enabled: bool) {
        self.seen = true;
        if enabled && !self.completed && self.asset.frame_count() > 1 && !self.active {
            self.active = true;
            self.last_advanced_at = now;
        }
    }

    fn advance(&mut self, now: Instant) -> bool {
        if !self.active || self.completed {
            return false;
        }
        self.elapsed = self
            .elapsed
            .saturating_add(now.saturating_duration_since(self.last_advanced_at));
        self.last_advanced_at = now;
        let previous = self.frame_index;
        (self.frame_index, self.completed) = self.asset.frame_index_at(self.elapsed);
        if self.completed {
            self.active = false;
        }
        self.frame_index != previous
    }

    fn finish_visibility(&mut self) {
        if !self.seen {
            self.active = false;
        }
    }

    fn deadline(&self) -> Option<Instant> {
        (self.active && !self.completed && self.asset.frame_count() > 1)
            .then(|| {
                self.last_advanced_at.checked_add(
                    self.asset
                        .remaining_in_frame(self.elapsed, self.frame_index),
                )
            })
            .flatten()
    }
}

impl DeclarativeAnimationPlayback {
    fn new(now: Instant) -> Self {
        Self {
            animation_ix: 0,
            elapsed: Duration::ZERO,
            last_advanced_at: now,
            next_frame_at: None,
            scheduled_stage: 0,
            last_value: 0.0,
            completed: false,
            active: false,
        }
    }

    fn pause(&mut self, now: Instant) {
        if self.active && !self.completed {
            self.elapsed = self
                .elapsed
                .saturating_add(now.saturating_duration_since(self.last_advanced_at));
        }
        self.last_advanced_at = now;
        self.next_frame_at = None;
        self.active = false;
    }

    fn resume(&mut self, now: Instant) {
        self.last_advanced_at = now;
        self.next_frame_at = None;
    }

    fn sample(
        &mut self,
        stages: &[Animation],
        now: Instant,
        animation_epoch: Instant,
        enabled: bool,
        reduce_motion: bool,
    ) -> DeclarativeAnimationSample {
        debug_assert!(!stages.is_empty());
        if self.animation_ix >= stages.len() {
            self.animation_ix = stages.len() - 1;
            self.elapsed = Duration::ZERO;
            self.completed = false;
            self.next_frame_at = None;
        }

        if reduce_motion {
            let animation_ix = stages.len() - 1;
            let phase = if stages[animation_ix].oneshot {
                1.0
            } else {
                0.0
            };
            self.active = false;
            self.next_frame_at = None;
            self.last_value = stages[animation_ix].eased(phase);
            return DeclarativeAnimationSample {
                animation_ix,
                value: self.last_value,
                request_frame: false,
                deadline: None,
            };
        }

        if !enabled {
            return DeclarativeAnimationSample {
                animation_ix: self.animation_ix,
                value: self.last_value,
                request_frame: false,
                deadline: None,
            };
        }

        if !self.completed {
            self.elapsed = self
                .elapsed
                .saturating_add(now.saturating_duration_since(self.last_advanced_at));
        }
        self.last_advanced_at = now;

        let (animation_ix, phase, active) = self.resolve_phase(stages, now, animation_epoch);
        let stage = &stages[animation_ix];
        self.last_value = stage.eased(phase);
        self.active = active;

        let (request_frame, deadline) = if active {
            if let Some(interval) = stage.frame_interval() {
                if self.scheduled_stage != animation_ix
                    || self.next_frame_at.is_none_or(|deadline| deadline <= now)
                {
                    let delay = if stage.oneshot {
                        interval.min(stage.duration.saturating_sub(self.elapsed))
                    } else {
                        interval
                    };
                    self.next_frame_at = now.checked_add(delay);
                    self.scheduled_stage = animation_ix;
                }
                (false, self.next_frame_at)
            } else {
                self.next_frame_at = None;
                (true, None)
            }
        } else {
            self.next_frame_at = None;
            (false, None)
        };

        DeclarativeAnimationSample {
            animation_ix,
            value: self.last_value,
            request_frame,
            deadline,
        }
    }

    fn resolve_phase(
        &mut self,
        stages: &[Animation],
        now: Instant,
        animation_epoch: Instant,
    ) -> (usize, f32, bool) {
        if self.completed {
            return (self.animation_ix, 1.0, false);
        }

        loop {
            let stage = &stages[self.animation_ix];
            if stage.duration.is_zero() {
                if stage.oneshot && self.animation_ix + 1 < stages.len() {
                    self.animation_ix += 1;
                    self.elapsed = Duration::ZERO;
                    self.next_frame_at = None;
                    continue;
                }
                self.completed = stage.oneshot;
                return (
                    self.animation_ix,
                    if stage.oneshot { 1.0 } else { 0.0 },
                    false,
                );
            }

            if !stage.oneshot && stage.synced {
                let elapsed = now.saturating_duration_since(animation_epoch);
                let remainder = elapsed.as_nanos() % stage.duration.as_nanos();
                let phase = remainder as f64 / stage.duration.as_nanos() as f64;
                return (self.animation_ix, phase as f32, true);
            }

            if stage.oneshot {
                if self.elapsed >= stage.duration {
                    if self.animation_ix + 1 < stages.len() {
                        self.elapsed = self.elapsed.saturating_sub(stage.duration);
                        self.animation_ix += 1;
                        self.next_frame_at = None;
                        continue;
                    }
                    self.elapsed = stage.duration;
                    self.completed = true;
                    return (self.animation_ix, 1.0, false);
                }
                return (
                    self.animation_ix,
                    self.elapsed.as_secs_f32() / stage.duration.as_secs_f32(),
                    true,
                );
            }

            let remainder = self.elapsed.as_nanos() % stage.duration.as_nanos();
            let phase = remainder as f64 / stage.duration.as_nanos() as f64;
            return (self.animation_ix, phase as f32, true);
        }
    }
}

impl DeclarativeSpringPlayback {
    fn new(declaration: &ElementSpring, now: Instant) -> Self {
        let target = finite_spring_value(declaration.target, 0.0);
        let initial = declaration
            .initial
            .filter(|value| value.is_finite())
            .unwrap_or(target);
        Self {
            state: SpringState {
                position: initial,
                velocity: 0.0,
            },
            target,
            config: declaration.config,
            initial,
            playback: declaration.playback,
            updated_at: now,
            active: false,
        }
    }

    fn pause(&mut self, now: Instant) {
        self.advance_previous_target(now);
        self.updated_at = now;
        self.active = false;
    }

    fn resume(&mut self, now: Instant) {
        self.updated_at = now;
    }

    fn sample(
        &mut self,
        declaration: &ElementSpring,
        now: Instant,
        enabled: bool,
        reduce_motion: bool,
    ) -> DeclarativeSpringSample {
        self.advance_previous_target(now);
        self.updated_at = now;
        self.config = declaration.config;
        self.target = finite_spring_value(declaration.target, self.target);
        self.playback = declaration.playback;

        if reduce_motion {
            self.state = SpringState {
                position: self.target,
                velocity: 0.0,
            };
            self.active = false;
            return DeclarativeSpringSample {
                value: self.state.position,
                request_frame: false,
            };
        }

        if !enabled {
            self.active = false;
            return DeclarativeSpringSample {
                value: self.state.position,
                request_frame: false,
            };
        }

        let epsilon = sane_spring_epsilon(declaration.epsilon);
        self.active = match self.playback {
            SpringPlayback::Running if self.config.is_valid() => {
                let settled = self.config.is_settled(self.state, self.target, epsilon);
                if settled {
                    self.state = SpringState {
                        position: self.target,
                        velocity: 0.0,
                    };
                }
                !settled
            }
            SpringPlayback::Running => {
                self.state = SpringState {
                    position: self.target,
                    velocity: 0.0,
                };
                false
            }
            SpringPlayback::Paused => false,
            SpringPlayback::Stopped => {
                self.state.velocity = 0.0;
                false
            }
            SpringPlayback::Completed => {
                self.state = SpringState {
                    position: self.target,
                    velocity: 0.0,
                };
                false
            }
            SpringPlayback::Cancelled => {
                self.state = SpringState {
                    position: self.initial,
                    velocity: 0.0,
                };
                false
            }
        };

        DeclarativeSpringSample {
            value: self.state.position,
            request_frame: self.active,
        }
    }

    fn advance_previous_target(&mut self, now: Instant) {
        if !self.active || self.playback != SpringPlayback::Running || !self.config.is_valid() {
            return;
        }
        let elapsed = now.saturating_duration_since(self.updated_at);
        let seconds = elapsed.as_secs_f64().min(60.0) as f32;
        self.state = self.config.step(self.state, self.target, seconds);
    }
}

impl StyleTransitionPlayback {
    fn new(target: TransitionPaintStyle, config: &Transition, now: Instant) -> Self {
        Self {
            from: target,
            current: target,
            target,
            config: config.clone(),
            elapsed: Duration::ZERO,
            last_advanced_at: now,
            next_frame_at: None,
            in_progress: false,
            active: false,
        }
    }

    fn sample(
        &mut self,
        target: TransitionPaintStyle,
        config: &Transition,
        now: Instant,
        enabled: bool,
        reduce_motion: bool,
    ) -> TransitionPaintStyle {
        self.advance_to(now);
        let target_changed = self.target != target;
        let properties_changed = self.config.properties != config.properties;
        let cadence_changed =
            self.config.max_fps.map(f32::to_bits) != config.max_fps.map(f32::to_bits);
        let configuration_changed = !self.config.same_configuration(config);
        if target_changed || properties_changed {
            self.from = self.current;
            self.target = target;
            self.elapsed = Duration::ZERO;
            self.next_frame_at = None;
        }
        if configuration_changed {
            self.config = config.clone();
        }
        if cadence_changed {
            self.next_frame_at = None;
        }

        if reduce_motion || self.config.duration.is_zero() || self.config.properties.is_empty() {
            self.finish();
            return self.current;
        }

        if target_changed || properties_changed {
            self.in_progress = self.from.differs_for(self.target, self.config.properties);
            if !self.in_progress {
                self.finish();
                return self.current;
            }
        }

        if !enabled {
            self.active = false;
            self.next_frame_at = None;
            self.last_advanced_at = now;
            return self.current;
        }

        if self.in_progress {
            self.active = true;
            self.resolve_current();
        }
        if self.active {
            if let Some(interval) = self.config.frame_interval() {
                if self.next_frame_at.is_none_or(|deadline| deadline <= now) {
                    let remaining = self.config.duration.saturating_sub(self.elapsed);
                    self.next_frame_at = now.checked_add(interval.min(remaining));
                }
            } else {
                self.next_frame_at = None;
            }
        }
        self.current
    }

    fn advance_to(&mut self, now: Instant) {
        if self.active && self.in_progress {
            self.elapsed = self
                .elapsed
                .saturating_add(now.saturating_duration_since(self.last_advanced_at));
            self.resolve_current();
        }
        self.last_advanced_at = now;
    }

    fn resolve_current(&mut self) {
        if self.config.duration.is_zero() || self.elapsed >= self.config.duration {
            self.finish();
            return;
        }
        let phase = self.elapsed.as_secs_f32() / self.config.duration.as_secs_f32();
        self.current = TransitionPaintStyle::interpolate(
            self.from,
            self.target,
            self.config.eased(phase),
            self.config.properties,
        );
    }

    fn finish(&mut self) {
        self.current = self.target;
        self.elapsed = self.config.duration;
        self.next_frame_at = None;
        self.in_progress = false;
        self.active = false;
    }

    fn pause(&mut self, now: Instant) {
        self.advance_to(now);
        self.active = false;
        self.next_frame_at = None;
    }

    fn resume(&mut self, now: Instant) {
        self.last_advanced_at = now;
        self.next_frame_at = None;
        self.active = self.in_progress;
    }

    fn deadline_due(&mut self, now: Instant) -> bool {
        if self.active && self.next_frame_at.is_some_and(|deadline| deadline <= now) {
            self.next_frame_at = None;
            true
        } else {
            false
        }
    }

    fn deadline(&self) -> Option<Instant> {
        (self.active && self.in_progress)
            .then_some(self.next_frame_at)
            .flatten()
    }

    fn requests_frame(&self) -> bool {
        self.active && self.in_progress && self.config.frame_interval().is_none()
    }
}

impl StyleTransitionPaintContext<'_> {
    fn sample(
        &mut self,
        id: ElementId,
        target: TransitionPaintStyle,
        config: &Transition,
    ) -> TransitionPaintStyle {
        let Some(playbacks) = self.playbacks.as_deref_mut() else {
            return target;
        };
        let playback = playbacks
            .entry(id)
            .or_insert_with(|| StyleTransitionPlayback::new(target, config, self.now));
        let value = playback.sample(target, config, self.now, self.enabled, self.reduce_motion);
        *self.request_frame |= playback.requests_frame();
        value
    }
}

fn sane_transition_color(color: Color, fallback: Color) -> Color {
    if [color.r, color.g, color.b, color.a]
        .into_iter()
        .all(f32::is_finite)
    {
        color
    } else {
        fallback
    }
}

fn sane_transition_shadow(shadow: BoxShadow) -> BoxShadow {
    BoxShadow::new(
        shadow.offset().x,
        shadow.offset().y,
        sane_transition_color(shadow.color(), Color::TRANSPARENT),
    )
    .blur_radius(shadow.blur())
    .spread_radius(shadow.spread())
    .inset(shadow.is_inset())
}

fn neutral_transition_shadow(inset: bool) -> BoxShadow {
    BoxShadow::new(0.0, 0.0, Color::TRANSPARENT).inset(inset)
}

fn interpolate_transition_shadow(from: BoxShadow, to: BoxShadow, phase: f32) -> BoxShadow {
    if from.is_inset() != to.is_inset() {
        return if phase < 0.5 { from } else { to };
    }
    BoxShadow::new(
        f32::interpolate(from.offset().x, to.offset().x, phase),
        f32::interpolate(from.offset().y, to.offset().y, phase),
        Color::interpolate(from.color(), to.color(), phase),
    )
    .blur_radius(f32::interpolate(from.blur(), to.blur(), phase))
    .spread_radius(f32::interpolate(from.spread(), to.spread(), phase))
    .inset(from.is_inset())
}

fn sane_spring_epsilon(epsilon: f32) -> f32 {
    if epsilon.is_finite() && epsilon >= 0.0 {
        epsilon
    } else {
        0.001
    }
}

fn finite_spring_value(value: f32, fallback: f32) -> f32 {
    if value.is_finite() { value } else { fallback }
}

impl DetachedMotionState {
    fn new(mut template: Element, animation_epoch: Instant) -> Self {
        sanitize_detached_element(&mut template, true);
        Self {
            template,
            animations: HashMap::with_capacity(4),
            time_animation_ids: HashSet::with_capacity(4),
            springs: HashMap::with_capacity(4),
            spring_ids: HashSet::with_capacity(4),
            motion_ids: HashSet::with_capacity(4),
            frame_requested: false,
            deadline: None,
            needs_resolve: true,
            animation_epoch,
        }
    }

    #[cfg(test)]
    fn resolve(
        &mut self,
        now: Instant,
        enabled: bool,
        reduce_motion: bool,
    ) -> Result<Element, UiError> {
        let root = self.begin_resolution(now, enabled, reduce_motion)?;
        self.finish_resolution();
        Ok(root)
    }

    fn begin_resolution(
        &mut self,
        now: Instant,
        enabled: bool,
        reduce_motion: bool,
    ) -> Result<Element, UiError> {
        let mut root = self.template.clone();
        self.motion_ids.clear();
        self.time_animation_ids.clear();
        self.spring_ids.clear();
        self.frame_requested = false;
        self.deadline = None;
        let mut resolved_motion_ids = Vec::new();
        resolve_declarative_animations(
            &mut root,
            &mut self.animations,
            &mut self.motion_ids,
            &mut self.time_animation_ids,
            &mut self.springs,
            &mut self.spring_ids,
            &mut self.frame_requested,
            &mut self.deadline,
            now,
            self.animation_epoch,
            enabled,
            reduce_motion,
            &mut resolved_motion_ids,
        )?;
        // Animators may construct fresh interactive descendants. Detached surfaces stay strictly
        // pointer-passive even when their resolved shape changes from frame to frame.
        sanitize_detached_element(&mut root, false);
        self.needs_resolve = false;
        Ok(root)
    }

    fn finish_resolution(&mut self) {
        self.animations
            .retain(|id, _| self.time_animation_ids.contains(id));
        self.springs.retain(|id, _| self.spring_ids.contains(id));
    }

    fn pause(&mut self, now: Instant) {
        for playback in self.animations.values_mut() {
            playback.pause(now);
        }
        for playback in self.springs.values_mut() {
            playback.pause(now);
        }
        self.frame_requested = false;
        self.deadline = None;
    }

    fn resume(&mut self, now: Instant) {
        for playback in self.animations.values_mut() {
            playback.resume(now);
        }
        for playback in self.springs.values_mut() {
            playback.resume(now);
        }
        self.frame_requested = false;
        self.deadline = None;
        self.needs_resolve = !self.animations.is_empty() || !self.springs.is_empty();
    }

    fn mark_needs_resolve(&mut self) {
        self.needs_resolve |= !self.animations.is_empty() || !self.springs.is_empty();
    }

    fn deadline_due(&mut self, now: Instant) -> bool {
        if self.deadline.is_some_and(|deadline| deadline <= now) {
            self.deadline = None;
            self.needs_resolve = true;
            true
        } else {
            false
        }
    }

    fn active_count(&self) -> usize {
        self.animations
            .values()
            .filter(|playback| playback.active)
            .count()
            + self
                .springs
                .values()
                .filter(|playback| playback.active)
                .count()
    }
}

impl DetachedTree {
    #[allow(clippy::too_many_arguments)]
    fn new(
        template: Element,
        root_seed: ElementId,
        viewport: Size,
        scale_factor: f32,
        renderer: &mut impl TextLayoutEngine,
        now: Instant,
        animation_epoch: Instant,
        animations_enabled: bool,
        reduce_motion: bool,
    ) -> Result<Self, UiError> {
        let mut tree = Self {
            motion: DetachedMotionState::new(template, animation_epoch),
            root: crate::div(),
            taffy: TaffyTree::with_capacity(32),
            root_node: None,
            root_seed,
            seen_ids: HashSet::with_capacity(32),
            input_ids: HashSet::with_capacity(4),
            animation_ids: HashSet::with_capacity(4),
            scroll_offsets: HashMap::new(),
            scroll_end_states: HashMap::new(),
            natural_bounds: HashMap::new(),
            paint_bounds: HashMap::with_capacity(32),
            text_inputs: HashMap::new(),
            animations: HashMap::new(),
        };
        tree.refresh(
            viewport,
            scale_factor,
            renderer,
            now,
            animations_enabled,
            reduce_motion,
        )?;
        Ok(tree)
    }

    #[allow(clippy::too_many_arguments)]
    fn refresh(
        &mut self,
        viewport: Size,
        scale_factor: f32,
        renderer: &mut impl TextLayoutEngine,
        now: Instant,
        animations_enabled: bool,
        reduce_motion: bool,
    ) -> Result<bool, UiError> {
        if !self.motion.needs_resolve {
            return Ok(false);
        }
        let mut root = self
            .motion
            .begin_resolution(now, animations_enabled, reduce_motion)?;
        let mut final_root_node = None;
        let mut converged = false;
        for _ in 0..=MAX_CONTAINER_QUERY_DEPTH {
            self.taffy.clear();
            self.seen_ids.clear();
            self.seen_ids
                .insert(ElementId::new(ACCESSIBILITY_ROOT_ID.0));
            validate_container_query_limits(&root)?;
            let inherited = TextStyle::new(14.0, Color::WHITE);
            let root_node = build_layout_node(
                &mut self.taffy,
                &mut self.seen_ids,
                &mut root,
                self.root_seed,
                0,
                &inherited,
                false,
            )?;
            compute_detached_layout(&mut self.taffy, root_node, viewport, scale_factor, renderer)?;
            compute_container_query_child_layouts(&root, &mut self.taffy, scale_factor, renderer)?;

            let resolution = {
                let mut prepare = |_: &mut Element| {};
                let motion = &mut self.motion;
                let mut context = ContainerQueryResolveContext {
                    taffy: &self.taffy,
                    prepare: &mut prepare,
                    animations: &mut motion.animations,
                    motion_ids: &mut motion.motion_ids,
                    time_animation_ids: &mut motion.time_animation_ids,
                    springs: &mut motion.springs,
                    spring_ids: &mut motion.spring_ids,
                    request_frame: &mut motion.frame_requested,
                    deadline: &mut motion.deadline,
                    now,
                    animation_epoch: motion.animation_epoch,
                    enabled: animations_enabled,
                    reduce_motion,
                    sanitize_detached: true,
                };
                context.resolve(&mut root)?
            };
            if !resolution.changed {
                final_root_node = Some(root_node);
                converged = true;
                break;
            }
        }
        if !converged {
            return Err(UiError::ContainerQueryDidNotConverge);
        }
        self.motion.finish_resolution();
        let root_node = final_root_node.expect("a converged detached declaration has a root node");

        self.input_ids.clear();
        sync_text_inputs(&root, &mut self.text_inputs, &mut self.input_ids);
        self.text_inputs.retain(|id, _| self.input_ids.contains(id));
        self.animation_ids.clear();
        sync_animations(&root, &mut self.animations, &mut self.animation_ids, now);
        self.animations
            .retain(|id, _| self.animation_ids.contains(id));
        let mut displayed_ids = HashSet::with_capacity(self.seen_ids.len());
        collect_displayed_ids(&root, &mut displayed_ids);
        self.scroll_offsets
            .retain(|id, _| displayed_ids.contains(id));
        self.scroll_end_states
            .retain(|id, _| displayed_ids.contains(id));
        self.natural_bounds.clear();
        self.root = root;
        self.root_node = Some(root_node);
        Ok(true)
    }

    fn layout(
        &mut self,
        viewport: Size,
        scale_factor: f32,
        renderer: &mut impl TextLayoutEngine,
    ) -> Result<(), UiError> {
        if self.motion.needs_resolve {
            return Ok(());
        }
        if let Some(root_node) = self.root_node {
            compute_detached_layout(&mut self.taffy, root_node, viewport, scale_factor, renderer)?;
            compute_container_query_child_layouts(
                &self.root,
                &mut self.taffy,
                scale_factor,
                renderer,
            )?;
            if container_queries_need_resolution(&self.root, &self.taffy)? {
                self.motion.needs_resolve = true;
            }
        }
        Ok(())
    }

    fn root_layout(&self) -> Result<&taffy::tree::Layout, UiError> {
        let root = self
            .root_node
            .expect("a detached tree is laid out before it is painted");
        Ok(self.taffy.layout(root)?)
    }

    #[allow(clippy::too_many_arguments)]
    fn paint(
        &mut self,
        origin: Point,
        layer: PaintLayerKey,
        scene: &mut Scene,
        renderer: &mut impl TextLayoutEngine,
        scale_factor: f32,
        animations_enabled: bool,
        reduce_motion: bool,
        paint_time: Instant,
        viewport: Rect,
        source_order: &mut usize,
    ) -> Result<(), UiError> {
        self.natural_bounds.clear();
        for playback in self.animations.values_mut() {
            playback.seen = false;
        }
        collect_layout_bounds(
            &self.root,
            &self.taffy,
            &mut self.scroll_offsets,
            &mut self.scroll_end_states,
            &mut self.natural_bounds,
            origin,
        )?;

        self.paint_bounds.clear();
        let hovered = HashSet::new();
        let mut hit_regions = Vec::new();
        let mut scroll_regions = Vec::new();
        let mut scrollbar_states = HashMap::new();
        let mut dismiss_regions = Vec::new();
        #[cfg(target_os = "macos")]
        let mut native_views = Vec::new();
        let mut text_input_regions = Vec::new();
        let selectable_text_indices = HashMap::new();
        let mut selectable_text_regions = Vec::new();
        let mut transition_frame_requested = false;
        let mut transition_context = StyleTransitionPaintContext {
            playbacks: None,
            request_frame: &mut transition_frame_requested,
            enabled: animations_enabled,
            reduce_motion,
            now: paint_time,
        };
        let result = paint_element(
            &self.root,
            &self.taffy,
            &self.natural_bounds,
            &mut self.paint_bounds,
            &mut self.scroll_offsets,
            &hovered,
            None,
            None,
            None,
            None,
            scale_factor,
            scene,
            renderer,
            &mut self.text_inputs,
            &mut self.animations,
            animations_enabled,
            paint_time,
            &mut transition_context,
            &mut hit_regions,
            &mut scroll_regions,
            &mut scrollbar_states,
            &mut dismiss_regions,
            #[cfg(target_os = "macos")]
            &mut native_views,
            &mut text_input_regions,
            &selectable_text_indices,
            &mut selectable_text_regions,
            None,
            origin,
            viewport,
            viewport,
            layer,
            source_order,
            None,
        );
        for playback in self.animations.values_mut() {
            playback.finish_visibility();
        }
        // An unthrottled declaration samples once per presented paint. Marking the next sample
        // here avoids resolving a newly created tooltip twice inside its first paint.
        self.motion.needs_resolve |= self.motion.frame_requested;
        result
    }

    fn pause(&mut self, now: Instant) {
        self.motion.pause(now);
        for playback in self.animations.values_mut() {
            playback.advance(now);
            playback.active = false;
        }
    }

    fn resume(&mut self, now: Instant) {
        self.motion.resume(now);
    }

    fn advance_animations(&mut self, now: Instant) -> bool {
        let mut changed = self.motion.deadline_due(now);
        for playback in self.animations.values_mut() {
            changed |= playback.advance(now);
        }
        changed
    }

    fn next_animation_deadline(&self) -> Option<Instant> {
        self.animations
            .values()
            .filter_map(AnimationPlayback::deadline)
            .chain(self.motion.deadline)
            .min()
    }

    fn animation_counts(&self) -> (usize, usize) {
        (
            self.animations.len(),
            self.animations
                .values()
                .filter(|playback| playback.active)
                .count()
                + self.motion.active_count(),
        )
    }
}

impl DragPreview {
    #[allow(clippy::too_many_arguments)]
    fn paint(
        &mut self,
        scene: &mut Scene,
        renderer: &mut impl TextLayoutEngine,
        scale_factor: f32,
        animations_enabled: bool,
        reduce_motion: bool,
        paint_time: Instant,
        viewport: Rect,
        source_order: &mut usize,
    ) -> Result<(), UiError> {
        self.tree.refresh(
            Size::new(viewport.width, viewport.height),
            scale_factor,
            renderer,
            paint_time,
            animations_enabled,
            reduce_motion,
        )?;
        let origin = Point::new(
            self.position.x - self.cursor_offset.x,
            self.position.y - self.cursor_offset.y,
        );
        self.tree.paint(
            origin,
            PaintLayerKey {
                plane: crate::ScenePlane::Overlay,
                z_index: i16::MAX,
            },
            scene,
            renderer,
            scale_factor,
            animations_enabled,
            reduce_motion,
            paint_time,
            viewport,
            source_order,
        )
    }
}

impl TooltipOverlay {
    #[allow(clippy::too_many_arguments)]
    fn new(
        target: ElementId,
        tooltip: &Tooltip,
        viewport: Size,
        scale_factor: f32,
        renderer: &mut impl TextLayoutEngine,
        now: Instant,
        animation_epoch: Instant,
        animations_enabled: bool,
        reduce_motion: bool,
    ) -> Result<Self, UiError> {
        let tree = DetachedTree::new(
            tooltip.content.as_ref().clone(),
            ElementId::new(0x7474_6970_6f6f_6c00),
            viewport,
            scale_factor,
            renderer,
            now,
            animation_epoch,
            animations_enabled,
            reduce_motion,
        )?;
        Ok(Self {
            target,
            content_identity: tooltip.content_identity(),
            placement: tooltip.placement,
            gap: tooltip.gap,
            viewport_margin: tooltip.viewport_margin,
            tree,
        })
    }

    fn matches(&self, target: ElementId, tooltip: &Tooltip) -> bool {
        self.target == target && self.content_identity == tooltip.content_identity()
    }

    #[allow(clippy::too_many_arguments)]
    fn paint(
        &mut self,
        anchor: Rect,
        scene: &mut Scene,
        renderer: &mut impl TextLayoutEngine,
        scale_factor: f32,
        animations_enabled: bool,
        reduce_motion: bool,
        paint_time: Instant,
        viewport: Rect,
        source_order: &mut usize,
    ) -> Result<(), UiError> {
        self.tree.refresh(
            Size::new(viewport.width, viewport.height),
            scale_factor,
            renderer,
            paint_time,
            animations_enabled,
            reduce_motion,
        )?;
        let layout = *self.tree.root_layout()?;
        let placed = place_anchored(
            anchor,
            Size::new(layout.size.width, layout.size.height),
            viewport,
            self.placement,
            self.gap,
            self.viewport_margin,
        );
        let origin = Point::new(placed.x - layout.location.x, placed.y - layout.location.y);
        self.tree.paint(
            origin,
            PaintLayerKey {
                plane: crate::ScenePlane::Overlay,
                z_index: i16::MAX - 1,
            },
            scene,
            renderer,
            scale_factor,
            animations_enabled,
            reduce_motion,
            paint_time,
            viewport,
            source_order,
        )
    }
}

impl UiTree {
    pub fn new() -> Self {
        Self::new_at(Instant::now())
    }

    pub(crate) fn new_at(animation_epoch: Instant) -> Self {
        Self {
            root: None,
            taffy: TaffyTree::with_capacity(256),
            root_node: None,
            mounted_state_dirty: false,
            seen_ids: HashSet::with_capacity(256),
            displayed_ids: HashSet::with_capacity(256),
            visible_ids: HashSet::with_capacity(256),
            scroll_offsets: HashMap::new(),
            scroll_end_states: HashMap::with_capacity(8),
            virtual_scroll_handles: HashMap::with_capacity(8),
            natural_bounds: HashMap::with_capacity(256),
            element_bounds: HashMap::with_capacity(256),
            hit_regions: Vec::with_capacity(128),
            drop_predicates: HashMap::with_capacity(8),
            context_menu_ids: HashSet::with_capacity(8),
            mouse_listener_bindings: Vec::with_capacity(16),
            mouse_listener_ranges: HashMap::with_capacity(8),
            mouse_listener_elements: Vec::with_capacity(8),
            key_listener_bindings: Vec::with_capacity(16),
            key_listener_ranges: HashMap::with_capacity(8),
            action_listener_bindings: Vec::with_capacity(16),
            action_listener_ranges: HashMap::with_capacity(8),
            scroll_wheel_ids: HashSet::with_capacity(8),
            touch_ids: HashSet::with_capacity(8),
            mouse_pressure_ids: HashSet::with_capacity(8),
            pinch_ids: HashSet::with_capacity(8),
            rotation_ids: HashSet::with_capacity(8),
            smart_magnify_ids: HashSet::with_capacity(8),
            tooltips: HashMap::with_capacity(8),
            pointer_tooltip: None,
            hovered_tooltip: None,
            pending_tooltip: None,
            visible_tooltip: None,
            tooltip_overlay: None,
            scroll_regions: Vec::with_capacity(8),
            scrollbar_drag: None,
            scrollbar_states: HashMap::with_capacity(8),
            hovered_scrollbar: None,
            dismiss_regions: Vec::with_capacity(4),
            #[cfg(target_os = "macos")]
            native_views: Vec::with_capacity(4),
            text_input_regions: Vec::with_capacity(8),
            text_inputs: HashMap::with_capacity(8),
            selectable_texts: Vec::with_capacity(64),
            selectable_text_indices: HashMap::with_capacity(64),
            selectable_text_regions: Vec::with_capacity(64),
            static_text_selection: None,
            static_text_gesture: None,
            last_static_text_click: None,
            accessibility_text_ids: HashMap::with_capacity(8),
            next_accessibility_text_id: ACCESSIBILITY_ROOT_ID.0 - 1,
            animations: HashMap::with_capacity(8),
            animation_ids: HashSet::with_capacity(8),
            declarative_animations: HashMap::with_capacity(8),
            declarative_time_animation_ids: HashSet::with_capacity(8),
            declarative_springs: HashMap::with_capacity(8),
            declarative_spring_ids: HashSet::with_capacity(8),
            declarative_animation_ids: HashSet::with_capacity(8),
            declarative_animation_frame_requested: false,
            declarative_animation_deadline: None,
            style_transitions: HashMap::with_capacity(8),
            style_transition_ids: HashSet::with_capacity(8),
            style_transition_frame_requested: false,
            animation_epoch,
            animations_enabled: true,
            reduce_motion: false,
            selecting_input: None,
            hovered: HashSet::with_capacity(8),
            hover_scratch: HashSet::with_capacity(8),
            mouse_hover_path: Vec::with_capacity(8),
            mouse_hover_path_scratch: Vec::with_capacity(8),
            pending_mouse_hover_changes: Vec::with_capacity(8),
            pressed: None,
            dragging: None,
            external_drag_active: false,
            drag_over: None,
            drag_preview: None,
            focused: None,
            active_focus_trap: None,
            focusable_ids: HashSet::with_capacity(32),
            clickable_ids: HashSet::with_capacity(32),
            activation_targets: HashMap::with_capacity(8),
            invalid_ids: HashSet::with_capacity(8),
            form_ids: HashSet::with_capacity(4),
            form_submitter_ids: HashSet::with_capacity(4),
            focus_order: Vec::with_capacity(32),
            parents: HashMap::with_capacity(256),
            key_contexts: HashMap::with_capacity(32),
            focus_initialized: false,
            validation_announcement: None,
            viewport: Size::ZERO,
            scale_factor: 1.0,
        }
    }

    /// Copy the current painted application tree into one reusable, bounded inspector snapshot.
    ///
    /// This method exists only in explicit `inspector` builds. Ordinary builds contain neither
    /// the traversal nor any snapshot storage or paint-path branch.
    #[cfg(feature = "inspector")]
    pub(crate) fn inspector_snapshot_into(
        &self,
        snapshot: &mut crate::inspector::InspectorSnapshot,
        hit_lookup: &mut HashMap<ElementId, HitRegion>,
    ) {
        snapshot.nodes.clear();
        snapshot.focus_path.clear();
        snapshot.nodes_truncated = false;
        snapshot.hit_regions_truncated =
            self.hit_regions.len() > crate::inspector::MAX_INSPECTOR_NODES;

        hit_lookup.clear();
        hit_lookup.extend(
            self.hit_regions
                .iter()
                .copied()
                .take(crate::inspector::MAX_INSPECTOR_NODES)
                .map(|region| (region.id, region)),
        );

        snapshot.focus_path.extend(self.focus_path());
        let Some(root) = self.root.as_ref() else {
            return;
        };
        let viewport = Rect::from_size(self.viewport);
        let mut source_order = 0;
        collect_inspector_nodes(
            root,
            None,
            0,
            PaintLayerKey::default(),
            viewport,
            viewport,
            &self.element_bounds,
            hit_lookup,
            self.focused,
            &mut source_order,
            snapshot,
        );
    }

    #[cfg(test)]
    fn set_root(
        &mut self,
        root: Element,
        viewport: Size,
        scale_factor: f32,
        renderer: &mut impl TextLayoutEngine,
    ) -> Result<(), UiError> {
        self.set_root_with_prepare(root, viewport, scale_factor, renderer, |_| {})
    }

    /// Mount a declaration while preparing each size-dependent subtree before its first layout.
    ///
    /// The production runtime uses this hook to resolve image resources returned by a query
    /// callback in the same cache frame as the surrounding view. Ordinary callers use
    /// [`UiTree::set_root`], whose preparation hook is a no-op.
    pub(crate) fn set_root_with_prepare(
        &mut self,
        root: Element,
        viewport: Size,
        scale_factor: f32,
        renderer: &mut impl TextLayoutEngine,
        mut prepare: impl FnMut(&mut Element),
    ) -> Result<(), UiError> {
        let now = Instant::now();
        // Retain the previous playback registry until query callbacks have declared their current
        // descendants. This preserves an animation keyed inside a stable query across view
        // rebuilds instead of restarting it during the intermediate empty-shell layout.
        self.set_root_unlaid(root, viewport, scale_factor, now, false)?;
        self.layout_with_prepare_at(viewport, scale_factor, renderer, now, &mut prepare)
    }

    fn set_root_unlaid(
        &mut self,
        mut root: Element,
        viewport: Size,
        scale_factor: f32,
        now: Instant,
        retain_motion_registry: bool,
    ) -> Result<(), UiError> {
        self.declarative_animation_ids.clear();
        self.declarative_time_animation_ids.clear();
        self.declarative_spring_ids.clear();
        self.declarative_animation_frame_requested = false;
        self.declarative_animation_deadline = None;
        let mut resolved_motion_ids = Vec::new();
        resolve_declarative_animations(
            &mut root,
            &mut self.declarative_animations,
            &mut self.declarative_animation_ids,
            &mut self.declarative_time_animation_ids,
            &mut self.declarative_springs,
            &mut self.declarative_spring_ids,
            &mut self.declarative_animation_frame_requested,
            &mut self.declarative_animation_deadline,
            now,
            self.animation_epoch,
            self.animations_enabled,
            self.reduce_motion,
            &mut resolved_motion_ids,
        )?;
        if retain_motion_registry {
            self.finalize_declarative_motion_registry();
        }
        self.build_resolved_root(root, viewport, scale_factor)?;
        if retain_motion_registry {
            self.sync_mounted_root(now)?;
        }
        Ok(())
    }

    fn build_resolved_root(
        &mut self,
        mut root: Element,
        viewport: Size,
        scale_factor: f32,
    ) -> Result<(), UiError> {
        self.viewport = viewport;
        self.scale_factor = scale_factor;
        self.taffy.clear();
        self.seen_ids.clear();
        self.seen_ids
            .insert(ElementId::new(ACCESSIBILITY_ROOT_ID.0));
        let mut style_transition_count = 0;
        validate_style_transition_count(&root, &mut style_transition_count)?;
        validate_container_query_limits(&root)?;
        collect_explicit_ids(&root, &mut self.seen_ids)?;
        let inherited = TextStyle::new(14.0, Color::WHITE);
        let root_node = build_layout_node(
            &mut self.taffy,
            &mut self.seen_ids,
            &mut root,
            ElementId::new(0x9e37_79b9_7f4a_7c15),
            0,
            &inherited,
            true,
        )?;
        self.root = Some(root);
        self.root_node = Some(root_node);
        self.mounted_state_dirty = true;
        Ok(())
    }

    /// Reconcile retained interaction and playback state exactly once against a fully expanded
    /// declaration. Intermediate container-query shells deliberately skip this phase so they
    /// cannot transiently unmount stable text input, scroll, transition, or image state.
    fn sync_mounted_root(&mut self, now: Instant) -> Result<(), UiError> {
        let root = self
            .root
            .as_ref()
            .expect("mounted state is synchronized after a resolved root is built");
        self.displayed_ids.clear();
        collect_displayed_ids(root, &mut self.displayed_ids);
        self.visible_ids.clear();
        collect_visible_ids(root, &mut self.visible_ids);
        if let Some(drag) = self.scrollbar_drag
            && !self.visible_ids.contains(&drag.id)
        {
            if let Some(binding) = self.virtual_scroll_handles.get(&drag.id) {
                binding.handle.scrollbar_drag_ended();
            }
            self.scrollbar_drag = None;
        }
        self.style_transition_ids.clear();
        collect_style_transition_ids(root, &mut self.style_transition_ids);
        self.style_transitions
            .retain(|id, _| self.style_transition_ids.contains(id));
        self.animation_ids.clear();
        sync_animations(root, &mut self.animations, &mut self.animation_ids, now);
        self.animations
            .retain(|id, _| self.animation_ids.contains(id));
        if let Some(root) = &self.root {
            let mut tooltips = HashMap::with_capacity(self.tooltips.capacity().max(8));
            collect_tooltips(root, &mut tooltips)?;
            self.tooltips = tooltips;
            self.tooltip_overlay = None;
            if self
                .hovered_tooltip
                .is_some_and(|target| !self.tooltips.contains_key(&target))
            {
                self.hovered_tooltip = None;
                self.pointer_tooltip = None;
                self.pending_tooltip = None;
                self.visible_tooltip = None;
            }
            self.virtual_scroll_handles.clear();
            sync_virtual_scrolls(
                root,
                &mut self.virtual_scroll_handles,
                &mut self.scroll_offsets,
                &mut self.scrollbar_states,
                now,
            );
            let mut input_ids = HashSet::new();
            sync_text_inputs(root, &mut self.text_inputs, &mut input_ids);
            self.text_inputs.retain(|id, _| input_ids.contains(id));

            let previous_selectable_texts = std::mem::take(&mut self.selectable_texts);
            self.selectable_texts = Vec::with_capacity(previous_selectable_texts.len());
            let mut previous_selectable_texts = previous_selectable_texts
                .into_iter()
                .map(|entry| (entry.id, entry))
                .collect::<HashMap<_, _>>();
            self.selectable_text_indices.clear();
            collect_selectable_texts(
                root,
                &mut self.selectable_texts,
                &mut self.selectable_text_indices,
                &mut previous_selectable_texts,
            );
            sync_static_text_selection(
                &mut self.static_text_selection,
                &self.selectable_texts,
                &self.selectable_text_indices,
            );
            if self.static_text_gesture.is_some() && self.static_text_selection.is_none() {
                self.static_text_gesture = None;
            }
            if self
                .last_static_text_click
                .is_some_and(|click| !self.selectable_text_indices.contains_key(&click.id))
            {
                self.last_static_text_click = None;
            }
            let mut accessible_text_ids = input_ids.clone();
            accessible_text_ids.extend(self.selectable_text_indices.keys().copied());
            sync_accessibility_text_ids(
                &accessible_text_ids,
                &self.seen_ids,
                &mut self.accessibility_text_ids,
                &mut self.next_accessibility_text_id,
            );
            if self
                .selecting_input
                .is_some_and(|id| !input_ids.contains(&id))
            {
                self.selecting_input = None;
            }
        }
        self.rebuild_focus_index();
        self.rebuild_dispatch_index();
        self.rebuild_drop_predicates();
        self.scroll_offsets
            .retain(|id, _| self.displayed_ids.contains(id));
        self.scroll_end_states
            .retain(|id, _| self.displayed_ids.contains(id));
        self.scrollbar_states
            .retain(|id, _| self.displayed_ids.contains(id));
        for (id, state) in &mut self.scrollbar_states {
            if !self.visible_ids.contains(id) {
                state.hovered = false;
                state.dragging = false;
                state.visible_until = None;
            }
        }
        if self
            .hovered_scrollbar
            .is_some_and(|id| !self.visible_ids.contains(&id))
        {
            self.hovered_scrollbar = None;
        }
        self.hovered.retain(|id| self.visible_ids.contains(id));
        if self
            .pressed
            .is_some_and(|id| !self.visible_ids.contains(&id))
        {
            self.pressed = None;
        }
        if self
            .dragging
            .is_some_and(|id| !self.visible_ids.contains(&id))
        {
            self.dragging = None;
            self.drag_over = None;
        }
        if self
            .drag_over
            .is_some_and(|id| !self.visible_ids.contains(&id))
        {
            self.drag_over = None;
        }
        if self
            .focused
            .is_some_and(|id| !self.focusable_ids.contains(&id))
        {
            self.focused = None;
        }
        self.initialize_focus_if_needed();
        if self.pointer_tooltip.is_none() {
            self.reconcile_tooltip(now);
        }
        self.mounted_state_dirty = false;
        Ok(())
    }

    fn finalize_declarative_motion_registry(&mut self) {
        self.declarative_animations
            .retain(|id, _| self.declarative_time_animation_ids.contains(id));
        self.declarative_springs
            .retain(|id, _| self.declarative_spring_ids.contains(id));
        self.recompute_declarative_motion_schedule();
    }

    fn initialize_focus_if_needed(&mut self) {
        if self.focused.is_none() && !self.focus_initialized {
            self.focused = self
                .root
                .as_ref()
                .and_then(find_auto_focus)
                .filter(|id| self.focusable_ids.contains(id));
        }
        self.focus_initialized = true;
    }

    fn recompute_declarative_motion_schedule(&mut self) {
        self.declarative_animation_frame_requested = self
            .declarative_time_animation_ids
            .iter()
            .filter_map(|id| self.declarative_animations.get(id))
            .any(|playback| playback.active && playback.next_frame_at.is_none())
            || self
                .declarative_spring_ids
                .iter()
                .filter_map(|id| self.declarative_springs.get(id))
                .any(|playback| playback.active);
        self.declarative_animation_deadline = self
            .declarative_time_animation_ids
            .iter()
            .filter_map(|id| self.declarative_animations.get(id))
            .filter(|playback| playback.active)
            .filter_map(|playback| playback.next_frame_at)
            .min();
    }

    #[cfg(any(test, feature = "test-support"))]
    pub(crate) fn set_root_for_test(
        &mut self,
        root: Element,
        viewport: Size,
        scale_factor: f32,
        now: Instant,
    ) -> Result<(), UiError> {
        self.set_root_unlaid(root, viewport, scale_factor, now, false)
    }

    #[cfg(any(test, feature = "test-support"))]
    pub(crate) fn contains_element(&self, id: ElementId) -> bool {
        self.seen_ids.contains(&id)
    }

    pub(crate) fn is_clickable(&self, id: ElementId) -> bool {
        self.clickable_ids.contains(&id)
    }

    pub fn set_animations_enabled(&mut self, enabled: bool, now: Instant) {
        if self.animations_enabled == enabled {
            return;
        }
        if !enabled {
            for playback in self.declarative_animations.values_mut() {
                playback.pause(now);
            }
            for playback in self.declarative_springs.values_mut() {
                playback.pause(now);
            }
            for playback in self.style_transitions.values_mut() {
                playback.pause(now);
            }
            for playback in self.animations.values_mut() {
                playback.advance(now);
                playback.active = false;
            }
            if let Some(preview) = &mut self.drag_preview {
                preview.tree.pause(now);
            }
            if let Some(tooltip) = &mut self.tooltip_overlay {
                tooltip.tree.pause(now);
            }
        } else {
            for playback in self.declarative_animations.values_mut() {
                playback.resume(now);
            }
            for playback in self.declarative_springs.values_mut() {
                playback.resume(now);
            }
            for playback in self.style_transitions.values_mut() {
                playback.resume(now);
            }
            if let Some(preview) = &mut self.drag_preview {
                preview.tree.resume(now);
            }
            if let Some(tooltip) = &mut self.tooltip_overlay {
                tooltip.tree.resume(now);
            }
        }
        self.animations_enabled = enabled;
    }

    pub(crate) fn set_reduce_motion(&mut self, reduce_motion: bool) {
        if self.reduce_motion == reduce_motion {
            return;
        }
        self.reduce_motion = reduce_motion;
        if let Some(preview) = &mut self.drag_preview {
            preview.tree.motion.mark_needs_resolve();
        }
        if let Some(tooltip) = &mut self.tooltip_overlay {
            tooltip.tree.motion.mark_needs_resolve();
        }
    }

    pub(crate) fn declarative_animation_frame_requested(&self) -> bool {
        self.animations_enabled && !self.reduce_motion && self.declarative_animation_frame_requested
    }

    pub(crate) fn style_transition_frame_requested(&self) -> bool {
        self.animations_enabled && !self.reduce_motion && self.style_transition_frame_requested
    }

    pub(crate) fn detached_animation_frame_requested(&self) -> bool {
        self.animations_enabled
            && !self.reduce_motion
            && (self
                .drag_preview
                .as_ref()
                .is_some_and(|preview| preview.tree.motion.frame_requested)
                || self
                    .tooltip_overlay
                    .as_ref()
                    .is_some_and(|tooltip| tooltip.tree.motion.frame_requested))
    }

    pub(crate) fn has_declarative_animations(&self) -> bool {
        !self.declarative_animations.is_empty() || !self.declarative_springs.is_empty()
    }

    pub(crate) fn declarative_animation_due(&mut self, now: Instant) -> bool {
        if !self.animations_enabled || self.reduce_motion {
            return false;
        }
        if self
            .declarative_animation_deadline
            .is_some_and(|deadline| deadline <= now)
        {
            self.declarative_animation_deadline = None;
            true
        } else {
            false
        }
    }

    pub fn advance_animations(&mut self, now: Instant) -> bool {
        if !self.animations_enabled {
            return false;
        }
        let mut changed = false;
        for playback in self.animations.values_mut() {
            changed |= playback.advance(now);
        }
        for playback in self.style_transitions.values_mut() {
            changed |= playback.deadline_due(now);
        }
        if let Some(preview) = &mut self.drag_preview {
            changed |= preview.tree.advance_animations(now);
        }
        if let Some(tooltip) = &mut self.tooltip_overlay {
            changed |= tooltip.tree.advance_animations(now);
        }
        changed
    }

    pub fn next_animation_deadline(&self) -> Option<Instant> {
        if !self.animations_enabled {
            return None;
        }
        self.animations
            .values()
            .filter_map(AnimationPlayback::deadline)
            .chain(self.declarative_animation_deadline)
            .chain(
                self.style_transitions
                    .values()
                    .filter_map(StyleTransitionPlayback::deadline),
            )
            .chain(
                self.drag_preview
                    .iter()
                    .filter_map(|preview| preview.tree.next_animation_deadline()),
            )
            .chain(
                self.tooltip_overlay
                    .iter()
                    .filter_map(|tooltip| tooltip.tree.next_animation_deadline()),
            )
            .min()
    }

    /// Advance the one-shot native scrollbar visibility deadline.
    ///
    /// This never drives an animation loop: it returns true once, when a visible scrollbar needs
    /// its final hide repaint.
    pub(crate) fn advance_scrollbars(&mut self, now: Instant) -> bool {
        let mut changed = false;
        for state in self.scrollbar_states.values_mut() {
            if !state.hovered
                && !state.dragging
                && state.visible_until.is_some_and(|deadline| deadline <= now)
            {
                state.visible_until = None;
                changed = true;
            }
        }
        changed
    }

    pub(crate) fn next_scrollbar_deadline(&self) -> Option<Instant> {
        self.scrollbar_states
            .values()
            .filter(|state| !state.hovered && !state.dragging)
            .filter_map(|state| state.visible_until)
            .min()
    }

    pub fn animation_counts(&self) -> (usize, usize) {
        let preview = self
            .drag_preview
            .as_ref()
            .map_or((0, 0), |preview| preview.tree.animation_counts());
        let tooltip = self
            .tooltip_overlay
            .as_ref()
            .map_or((0, 0), |tooltip| tooltip.tree.animation_counts());
        (
            self.animations.len() + preview.0 + tooltip.0,
            self.animations
                .values()
                .filter(|state| state.active)
                .count()
                + self
                    .declarative_animations
                    .values()
                    .filter(|state| state.active)
                    .count()
                + self
                    .declarative_springs
                    .values()
                    .filter(|state| state.active)
                    .count()
                + self
                    .style_transitions
                    .values()
                    .filter(|state| state.active)
                    .count()
                + preview.1
                + tooltip.1,
        )
    }

    /// Whether a mounted variable-height list learned geometry after this declaration was built.
    ///
    /// The runtime uses this edge-triggered revision comparison to request one correcting frame.
    /// A view rebuild is needed only when the measured mounted slice no longer covers the viewport;
    /// unchanged retained lists create no redraw source.
    pub(crate) fn take_variable_list_measurement_update(&mut self) -> ScrollResult {
        let mut result = ScrollResult::default();
        for binding in self.virtual_scroll_handles.values_mut() {
            let revision = binding.handle.measurement_revision();
            if revision == binding.measurement_revision {
                continue;
            }
            binding.measurement_revision = revision;
            result.changed = true;
            result.view_dirty |= !binding
                .handle
                .refresh_mount_after_measurement(&mut binding.mount);
        }
        result
    }

    #[cfg(test)]
    fn layout(
        &mut self,
        viewport: Size,
        scale_factor: f32,
        renderer: &mut impl TextLayoutEngine,
    ) -> Result<(), UiError> {
        self.layout_with_prepare_at(
            viewport,
            scale_factor,
            renderer,
            Instant::now(),
            &mut |_| {},
        )
    }

    #[cfg(any(test, feature = "test-support"))]
    pub(crate) fn layout_for_test(
        &mut self,
        viewport: Size,
        scale_factor: f32,
        renderer: &mut impl TextLayoutEngine,
        now: Instant,
    ) -> Result<(), UiError> {
        self.layout_with_prepare_at(viewport, scale_factor, renderer, now, &mut |_| {})
    }

    /// Recompute layout for the mounted declaration without rebuilding the application view.
    ///
    /// Resize-only frames use this retained path. Size-dependent container-query callbacks still
    /// receive their preparation hook, while stable element, text-input, scroll, and accessibility
    /// state remains mounted.
    pub(crate) fn relayout_with_prepare(
        &mut self,
        viewport: Size,
        scale_factor: f32,
        renderer: &mut impl TextLayoutEngine,
        mut prepare: impl FnMut(&mut Element),
    ) -> Result<(), UiError> {
        self.layout_with_prepare_at(
            viewport,
            scale_factor,
            renderer,
            Instant::now(),
            &mut prepare,
        )
    }

    /// Discard CPU-only semantic measurement caches before an exact offscreen visual layout.
    #[cfg(any(test, feature = "test-support"))]
    pub(crate) fn invalidate_layout_measurements_for_test(&mut self) -> Result<(), UiError> {
        if let Some(root) = &self.root {
            mark_layout_nodes_dirty(root, &mut self.taffy)?;
        }
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    fn layout_with_prepare_at(
        &mut self,
        viewport: Size,
        scale_factor: f32,
        renderer: &mut impl TextLayoutEngine,
        now: Instant,
        prepare: &mut impl FnMut(&mut Element),
    ) -> Result<(), UiError> {
        self.viewport = viewport;
        self.scale_factor = scale_factor;
        let mut converged = self.root_node.is_none();
        let mut needs_outer_layout = true;
        for _ in 0..=MAX_CONTAINER_QUERY_DEPTH {
            let Some(root_node) = self.root_node else {
                converged = true;
                break;
            };
            if needs_outer_layout {
                compute_detached_layout(
                    &mut self.taffy,
                    root_node,
                    viewport,
                    scale_factor,
                    renderer,
                )?;
            }
            if let Some(root) = &self.root {
                compute_container_query_child_layouts(
                    root,
                    &mut self.taffy,
                    scale_factor,
                    renderer,
                )?;
            }
            let resolution = self.resolve_container_queries(now, prepare)?;
            if !resolution.changed {
                converged = true;
                break;
            }
            self.mounted_state_dirty = true;

            let requires_full_rebuild = resolution.replaced_existing_subtree
                || self.build_pending_container_query_subtrees()?;
            if requires_full_rebuild {
                let root = self
                    .root
                    .take()
                    .expect("a laid out query declaration retains its element root");
                self.build_resolved_root(root, viewport, scale_factor)?;
                needs_outer_layout = true;
            } else {
                // The query boxes already have final parent geometry. Newly declared child roots
                // can be laid out directly inside those boxes without recomputing the outer tree.
                needs_outer_layout = false;
            }
        }
        if !converged {
            return Err(UiError::ContainerQueryDidNotConverge);
        }
        self.finalize_declarative_motion_registry();
        if self.mounted_state_dirty {
            self.sync_mounted_root(now)?;
        }
        if let Some(preview) = &mut self.drag_preview {
            preview.tree.layout(viewport, scale_factor, renderer)?;
        }
        if let Some(tooltip) = &mut self.tooltip_overlay {
            tooltip.tree.layout(viewport, scale_factor, renderer)?;
        }
        Ok(())
    }

    fn resolve_container_queries(
        &mut self,
        now: Instant,
        prepare: &mut impl FnMut(&mut Element),
    ) -> Result<ContainerQueryResolution, UiError> {
        let Some(root) = &mut self.root else {
            return Ok(ContainerQueryResolution::default());
        };
        let mut context = ContainerQueryResolveContext {
            taffy: &self.taffy,
            prepare,
            animations: &mut self.declarative_animations,
            motion_ids: &mut self.declarative_animation_ids,
            time_animation_ids: &mut self.declarative_time_animation_ids,
            springs: &mut self.declarative_springs,
            spring_ids: &mut self.declarative_spring_ids,
            request_frame: &mut self.declarative_animation_frame_requested,
            deadline: &mut self.declarative_animation_deadline,
            now,
            animation_epoch: self.animation_epoch,
            enabled: self.animations_enabled,
            reduce_motion: self.reduce_motion,
            sanitize_detached: false,
        };
        context.resolve(root)
    }

    /// Build callback roots into the existing Taffy arena when every changed query was previously
    /// empty. Returns `true` when an explicit ID collides with an already assigned generated ID;
    /// a full rebuild then preserves the normal explicit-ID-first assignment contract.
    fn build_pending_container_query_subtrees(&mut self) -> Result<bool, UiError> {
        let Some(root) = &mut self.root else {
            return Ok(false);
        };
        let mut style_transition_count = 0;
        validate_style_transition_count(root, &mut style_transition_count)?;
        validate_container_query_limits(root)?;
        build_pending_container_query_subtrees(root, &mut self.taffy, &mut self.seen_ids)
    }

    pub fn paint(
        &mut self,
        scene: &mut Scene,
        renderer: &mut impl TextLayoutEngine,
    ) -> Result<(), UiError> {
        self.paint_at(scene, renderer, Instant::now())
    }

    pub(crate) fn paint_at(
        &mut self,
        scene: &mut Scene,
        renderer: &mut impl TextLayoutEngine,
        paint_time: Instant,
    ) -> Result<(), UiError> {
        self.natural_bounds.clear();
        self.element_bounds.clear();
        self.hit_regions.clear();
        self.scroll_regions.clear();
        self.dismiss_regions.clear();
        #[cfg(target_os = "macos")]
        self.native_views.clear();
        self.text_input_regions.clear();
        self.selectable_text_regions.clear();
        self.style_transition_frame_requested = false;
        for playback in self.animations.values_mut() {
            playback.seen = false;
        }
        let Some(root) = &self.root else {
            return Ok(());
        };
        let viewport = Rect::from_size(self.viewport);
        collect_layout_bounds(
            root,
            &self.taffy,
            &mut self.scroll_offsets,
            &mut self.scroll_end_states,
            &mut self.natural_bounds,
            Point::ZERO,
        )?;
        let mut source_order = 0;
        let mut transition_context = StyleTransitionPaintContext {
            playbacks: Some(&mut self.style_transitions),
            request_frame: &mut self.style_transition_frame_requested,
            enabled: self.animations_enabled,
            reduce_motion: self.reduce_motion,
            now: paint_time,
        };
        let result = paint_element(
            root,
            &self.taffy,
            &self.natural_bounds,
            &mut self.element_bounds,
            &mut self.scroll_offsets,
            &self.hovered,
            self.pressed,
            self.dragging,
            self.drag_over,
            self.focused,
            self.scale_factor,
            scene,
            renderer,
            &mut self.text_inputs,
            &mut self.animations,
            self.animations_enabled,
            paint_time,
            &mut transition_context,
            &mut self.hit_regions,
            &mut self.scroll_regions,
            &mut self.scrollbar_states,
            &mut self.dismiss_regions,
            #[cfg(target_os = "macos")]
            &mut self.native_views,
            &mut self.text_input_regions,
            &self.selectable_text_indices,
            &mut self.selectable_text_regions,
            self.static_text_selection,
            Point::ZERO,
            viewport,
            viewport,
            PaintLayerKey::default(),
            &mut source_order,
            None,
        );
        for playback in self.animations.values_mut() {
            playback.finish_visibility();
        }
        result?;
        if let Some(target) = self.visible_tooltip
            && let Some(tooltip) = self.tooltips.get(&target).cloned()
            && let Some(anchor) = self.element_bounds.get(&target).copied()
        {
            let rebuild = self
                .tooltip_overlay
                .as_ref()
                .is_none_or(|overlay| !overlay.matches(target, &tooltip));
            if rebuild {
                self.tooltip_overlay = Some(TooltipOverlay::new(
                    target,
                    &tooltip,
                    self.viewport,
                    self.scale_factor,
                    renderer,
                    paint_time,
                    self.animation_epoch,
                    self.animations_enabled,
                    self.reduce_motion,
                )?);
            }
            if let Some(overlay) = &mut self.tooltip_overlay {
                overlay.paint(
                    anchor,
                    scene,
                    renderer,
                    self.scale_factor,
                    self.animations_enabled,
                    self.reduce_motion,
                    paint_time,
                    viewport,
                    &mut source_order,
                )?;
            }
        }
        if let Some(preview) = &mut self.drag_preview {
            preview.paint(
                scene,
                renderer,
                self.scale_factor,
                self.animations_enabled,
                self.reduce_motion,
                paint_time,
                viewport,
                &mut source_order,
            )?;
        }
        self.hit_regions.sort_by_key(|region| region.order);
        self.scroll_regions.sort_by_key(|region| region.order);
        if let Some(drag) = self.scrollbar_drag
            && !self
                .scroll_regions
                .iter()
                .any(|region| region.id == drag.id)
        {
            self.scrollbar_drag = None;
            if let Some(state) = self.scrollbar_states.get_mut(&drag.id) {
                state.dragging = false;
                state.hovered = false;
                state.visible_until = None;
            }
            if self.hovered_scrollbar == Some(drag.id) {
                self.hovered_scrollbar = None;
            }
        }
        self.dismiss_regions.sort_by_key(|region| region.order);
        #[cfg(target_os = "macos")]
        self.native_views
            .sort_by_key(|region| (region.z_index, region.source_order));
        scene.finish();
        Ok(())
    }

    /// Returns true when paint-only hover state changed.
    pub fn pointer_moved(&mut self, point: Point, renderer: &mut impl TextLayoutEngine) -> bool {
        let now = Instant::now();
        self.hover_scratch.clear();
        if self.dragging.is_none() && !self.external_drag_active {
            for region in self.hit_regions.iter().rev() {
                if region.stateful && region.contains(point) {
                    self.hover_scratch.insert(region.id);
                }
                if (region.blocks_pointer || region.pointer_listener) && region.contains(point) {
                    break;
                }
            }
        }
        let hover_changed = self.hover_scratch != self.hovered;
        if hover_changed {
            std::mem::swap(&mut self.hovered, &mut self.hover_scratch);
        }
        self.hover_scratch.clear();
        self.refresh_mouse_hover(Some(point));

        let mut selection_changed = false;
        if let Some(id) = self.selecting_input
            && let Some(mut region) = self
                .text_input_regions
                .iter()
                .rev()
                .find(|region| region.id == id)
                .cloned()
        {
            let offset = self.scroll_offsets.entry(id).or_default();
            let previous = *offset;
            if point.x < region.bounds.x {
                offset.x -= region.bounds.x - point.x;
            } else if point.x > region.bounds.right() {
                offset.x += point.x - region.bounds.right();
            }
            if point.y < region.bounds.y {
                offset.y -= region.bounds.y - point.y;
            } else if point.y > region.bounds.bottom() {
                offset.y += point.y - region.bounds.bottom();
            }
            offset.x = offset.x.clamp(0.0, region.max_scroll.x);
            offset.y = offset.y.clamp(0.0, region.max_scroll.y);
            if previous != *offset {
                let state = self.scrollbar_states.entry(id).or_default();
                if !state.hovered && !state.dragging {
                    state.visible_until = now.checked_add(SCROLLBAR_AUTO_HIDE_DELAY);
                }
            }
            region.scroll = *offset;
            let index = text_input_index_at(&region, point, self.scale_factor, renderer);
            let moved = self
                .text_inputs
                .get_mut(&id)
                .is_some_and(|state| state.move_to(index, true));
            selection_changed = previous != *offset || moved;
        }

        if let Some(gesture) = self.static_text_gesture {
            let next = self.selectable_text_position_at(point, true, renderer);
            if let Some(next) = next {
                let (target_start, target_end) = static_selection_unit_range(
                    next,
                    gesture.unit,
                    &self.selectable_texts,
                    &self.selectable_text_indices,
                )
                .unwrap_or((next, next));
                let extends_backward =
                    static_position_key(target_start, &self.selectable_text_indices)
                        < static_position_key(gesture.base_start, &self.selectable_text_indices);
                let next_selection = if extends_backward {
                    StaticTextSelection {
                        anchor: gesture.base_end,
                        focus: target_start,
                    }
                } else {
                    StaticTextSelection {
                        anchor: gesture.base_start,
                        focus: target_end,
                    }
                };
                if self.static_text_selection != Some(next_selection) {
                    self.static_text_selection = Some(next_selection);
                    selection_changed = true;
                }
                let delta = point - gesture.origin;
                if !gesture.moved
                    && delta.x * delta.x + delta.y * delta.y >= 4.0
                    && let Some(gesture) = self.static_text_gesture.as_mut()
                {
                    gesture.moved = true;
                }
            }
        }

        let tooltip_changed = self.update_tooltip_hover(Some(point), now);
        hover_changed || selection_changed || tooltip_changed
    }

    pub fn pointer_left(&mut self) -> bool {
        let hover_changed = if self.hovered.is_empty() {
            false
        } else {
            self.hovered.clear();
            true
        };
        self.refresh_mouse_hover(None);
        hover_changed | self.update_tooltip_hover(None, Instant::now())
    }

    /// Recompute listener hover state without changing selection, scrolling, or tooltip timers.
    ///
    /// Redraw calls this after layout so moving an element beneath a stationary pointer produces
    /// the same entry/exit transitions as web hover.
    pub(crate) fn refresh_mouse_hover(&mut self, point: Option<Point>) {
        self.mouse_hover_path_scratch.clear();
        if self.dragging.is_none()
            && !self.external_drag_active
            && let Some(point) = point
            && let Some(mut current) = self
                .hit_regions
                .iter()
                .rev()
                .find(|region| region.contains(point))
                .map(|region| region.id)
        {
            let mut depth = 0;
            loop {
                if depth == MAX_MOUSE_EVENT_PATH {
                    self.mouse_hover_path_scratch.clear();
                    break;
                }
                depth += 1;
                if self
                    .mouse_listener_ranges
                    .get(&current)
                    .is_some_and(|range| {
                        self.mouse_listener_bindings[range.clone()]
                            .iter()
                            .any(|binding| binding.kind == MouseListenerKind::Hover)
                    })
                {
                    self.mouse_hover_path_scratch.push(current);
                }
                let Some(parent) = self.parents.get(&current).copied() else {
                    break;
                };
                current = parent;
            }
        }

        for index in 0..self.mouse_hover_path.len() {
            let id = self.mouse_hover_path[index];
            let exited = !self.mouse_hover_path_scratch.contains(&id);
            if exited {
                self.push_mouse_hover_changes(id, false);
            }
        }
        for index in (0..self.mouse_hover_path_scratch.len()).rev() {
            let id = self.mouse_hover_path_scratch[index];
            let entered = !self.mouse_hover_path.contains(&id);
            if entered {
                self.push_mouse_hover_changes(id, true);
            }
        }
        std::mem::swap(
            &mut self.mouse_hover_path,
            &mut self.mouse_hover_path_scratch,
        );
        self.mouse_hover_path_scratch.clear();
    }

    fn push_mouse_hover_changes(&mut self, id: ElementId, hovered: bool) {
        let Some(range) = self.mouse_listener_ranges.get(&id).cloned() else {
            return;
        };
        for binding in self.mouse_listener_bindings[range].iter().copied() {
            if binding.kind == MouseListenerKind::Hover
                && self.pending_mouse_hover_changes.len() < crate::MAX_MOUSE_LISTENERS_PER_WINDOW
            {
                self.pending_mouse_hover_changes.push(MouseHoverChange {
                    key: binding.key,
                    hovered,
                });
            }
        }
    }

    pub(crate) fn take_mouse_hover_changes(&mut self, output: &mut Vec<MouseHoverChange>) {
        output.append(&mut self.pending_mouse_hover_changes);
    }

    /// Advance exactly one pending tooltip deadline.
    pub(crate) fn advance_tooltips(&mut self, now: Instant) -> bool {
        let Some(pending) = self.pending_tooltip else {
            return false;
        };
        if pending.show_at > now {
            return false;
        }
        self.pending_tooltip = None;
        if self.hovered_tooltip == Some(pending.target)
            && self.tooltips.contains_key(&pending.target)
        {
            self.visible_tooltip = Some(pending.target);
            self.tooltip_overlay = None;
            true
        } else {
            false
        }
    }

    pub(crate) fn next_tooltip_deadline(&self) -> Option<Instant> {
        self.pending_tooltip.map(|pending| pending.show_at)
    }

    /// Hide any pending or visible tooltip after presses, scrolling, or drag initiation.
    pub(crate) fn clear_tooltip(&mut self) -> bool {
        self.pointer_tooltip = None;
        self.hovered_tooltip = None;
        self.pending_tooltip = None;
        let changed = self.visible_tooltip.take().is_some();
        self.tooltip_overlay = None;
        changed
    }

    fn update_tooltip_hover(&mut self, point: Option<Point>, now: Instant) -> bool {
        self.pointer_tooltip = point.and_then(|point| self.tooltip_target_at(point));
        self.reconcile_tooltip(now)
    }

    fn reconcile_tooltip(&mut self, now: Instant) -> bool {
        let next = self.pointer_tooltip.or_else(|| {
            self.focused
                .and_then(|focused| self.tooltip_ancestor(focused))
        });
        if next == self.hovered_tooltip {
            return false;
        }
        let was_visible = self.visible_tooltip.take().is_some();
        self.tooltip_overlay = None;
        self.pending_tooltip = None;
        self.hovered_tooltip = next;
        let mut immediately_visible = false;
        if let Some(target) = next
            && let Some(tooltip) = self.tooltips.get(&target)
        {
            if tooltip.delay.is_zero() {
                self.visible_tooltip = Some(target);
                immediately_visible = true;
            } else if let Some(show_at) = now.checked_add(tooltip.delay) {
                self.pending_tooltip = Some(PendingTooltip { target, show_at });
            }
        }
        was_visible || immediately_visible
    }

    fn tooltip_ancestor(&self, mut current: ElementId) -> Option<ElementId> {
        loop {
            if self.tooltips.contains_key(&current) {
                return Some(current);
            }
            current = self.parents.get(&current).copied()?;
        }
    }

    /// Clear press/selection state when a platform pointer sequence is cancelled.
    pub(crate) fn cancel_pointer_interaction(&mut self) -> bool {
        self.selecting_input = None;
        let static_gesture_changed = self.static_text_gesture.take().is_some();
        let scrollbar_changed = self.scrollbar_drag.take().is_some();
        if scrollbar_changed {
            for state in self.scrollbar_states.values_mut() {
                state.dragging = false;
                state.hovered = false;
                state.visible_until = None;
            }
            self.hovered_scrollbar = None;
        }
        self.pressed.take().is_some()
            | scrollbar_changed
            | static_gesture_changed
            | self.clear_tooltip()
    }

    /// Mark an internal drag active and suppress the source click/ordinary hover states.
    pub(crate) fn begin_drag(&mut self, source: ElementId) -> bool {
        let changed = (self.dragging != Some(source))
            | self.drag_over.take().is_some()
            | self.pressed.take().is_some()
            | !self.hovered.is_empty()
            | self.clear_tooltip();
        self.dragging = Some(source);
        self.external_drag_active = false;
        self.hovered.clear();
        self.selecting_input = None;
        self.static_text_gesture = None;
        changed
    }

    /// Update only when the compatible target changes, avoiding redundant repaints while moving
    /// within one drop zone.
    pub(crate) fn set_drag_over(&mut self, target: Option<ElementId>) -> bool {
        if self.drag_over == target {
            false
        } else {
            self.drag_over = target;
            true
        }
    }

    pub(crate) fn begin_external_drag(&mut self) -> bool {
        let changed = !self.external_drag_active
            | self.dragging.take().is_some()
            | self.pressed.take().is_some()
            | !self.hovered.is_empty()
            | self.clear_tooltip();
        self.external_drag_active = true;
        self.hovered.clear();
        self.selecting_input = None;
        self.static_text_gesture = None;
        changed
    }

    pub(crate) fn end_drag(&mut self) -> bool {
        self.dragging.take().is_some()
            | std::mem::take(&mut self.external_drag_active)
            | self.drag_over.take().is_some()
    }

    pub(crate) fn element_bounds(&self, id: ElementId) -> Option<Rect> {
        self.element_bounds.get(&id).copied()
    }

    #[cfg(any(test, feature = "test-support"))]
    pub(crate) fn scroll_offset(&self, id: ElementId) -> Option<Vector> {
        self.scroll_offsets.get(&id).copied()
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn set_drag_preview(
        &mut self,
        preview: Option<Element>,
        source: ElementId,
        origin: Point,
        position: Point,
        cursor_offset: Option<Point>,
        renderer: &mut impl TextLayoutEngine,
        now: Instant,
    ) -> Result<bool, UiError> {
        let Some(mut root) = preview else {
            return Ok(self.drag_preview.take().is_some());
        };
        if let Some(source_bounds) = self.element_bounds(source) {
            if absolute_length(root.layout.size.width).is_none() {
                root.layout.size.width = Dimension::length(source_bounds.width);
            }
            if absolute_length(root.layout.size.height).is_none() {
                root.layout.size.height = Dimension::length(source_bounds.height);
            }
        }
        let mut tree = DetachedTree::new(
            root,
            ElementId::new(0xd4a6_31f8_6c92_7b05),
            self.viewport,
            self.scale_factor,
            renderer,
            now,
            self.animation_epoch,
            self.animations_enabled,
            self.reduce_motion,
        )?;
        // Drag creation happens between frames, unlike tooltip creation during paint. Sample an
        // unthrottled entrance again at the first actual presentation time.
        tree.motion.needs_resolve |= tree.motion.frame_requested;
        let default_offset = self
            .element_bounds(source)
            .map(|bounds| Point::new(origin.x - bounds.x, origin.y - bounds.y))
            .unwrap_or(Point::ZERO);
        self.drag_preview = Some(DragPreview {
            tree,
            cursor_offset: cursor_offset.unwrap_or(default_offset),
            position,
        });
        Ok(true)
    }

    pub(crate) fn move_drag_preview(&mut self, position: Point) -> bool {
        let Some(preview) = &mut self.drag_preview else {
            return false;
        };
        if preview.position == position {
            false
        } else {
            preview.position = position;
            true
        }
    }

    pub(crate) fn clear_drag_preview(&mut self) -> bool {
        self.drag_preview.take().is_some()
    }

    /// Start a captured drag on the topmost built-in vertical scrollbar under `point`.
    ///
    /// The full 12-point track is interactive even though the painted thumb stays visually slim.
    /// Pressing the track first centers the thumb at the pointer, then continues as a drag.
    pub(crate) fn begin_scrollbar_drag(&mut self, point: Option<Point>) -> Option<bool> {
        let point = point?;
        let (region, geometry) = self.scrollbar_at(point)?;
        let offset = self.scroll_offsets.entry(region.id).or_default();
        let previous = *offset;
        if !geometry.thumb.contains(point) && geometry.travel > 0.0 {
            let thumb_top = (point.y - region.scrollbar_bounds.y - geometry.thumb.height * 0.5)
                .clamp(0.0, geometry.travel);
            offset.y = thumb_top / geometry.travel * region.max_offset.y;
        }
        let view_dirty = region.virtual_scroll
            && *offset != previous
            && self
                .virtual_scroll_handles
                .get(&region.id)
                .is_none_or(|binding| binding.update_from_input(offset.y, region.bounds.height));
        if let Some(binding) = self.virtual_scroll_handles.get(&region.id) {
            binding.handle.scrollbar_drag_started();
        }
        self.scrollbar_drag = Some(ScrollbarDrag {
            id: region.id,
            pointer_origin_y: point.y,
            scroll_origin_y: offset.y,
        });
        let state = self.scrollbar_states.entry(region.id).or_default();
        state.hovered = true;
        state.dragging = true;
        state.visible_until = None;
        self.hovered_scrollbar = Some(region.id);
        self.selecting_input = None;
        self.static_text_gesture = None;
        self.pressed = None;
        Some(view_dirty)
    }

    /// Update an active scrollbar drag. Returns whether the retained scroll offset changed.
    pub(crate) fn drag_scrollbar(&mut self, point: Point) -> ScrollResult {
        let Some(drag) = self.scrollbar_drag else {
            return ScrollResult::default();
        };
        let Some(region) = self
            .scroll_regions
            .iter()
            .find(|region| region.id == drag.id)
            .copied()
        else {
            self.scrollbar_drag = None;
            return ScrollResult::default();
        };
        let Some(geometry) = vertical_scrollbar_geometry(region, drag.scroll_origin_y) else {
            self.scrollbar_drag = None;
            return ScrollResult::default();
        };
        if geometry.travel <= 0.0 {
            return ScrollResult::default();
        }
        let next = (drag.scroll_origin_y
            + (point.y - drag.pointer_origin_y) * region.max_offset.y / geometry.travel)
            .clamp(0.0, region.max_offset.y);
        let offset = self.scroll_offsets.entry(region.id).or_default();
        let state = self.scrollbar_states.entry(region.id).or_default();
        state.dragging = true;
        state.visible_until = None;
        if offset.y == next {
            ScrollResult::default()
        } else {
            offset.y = next;
            let view_dirty = region.virtual_scroll
                && self
                    .virtual_scroll_handles
                    .get(&region.id)
                    .is_none_or(|binding| binding.update_from_input(next, region.bounds.height));
            ScrollResult {
                changed: true,
                view_dirty,
            }
        }
    }

    pub(crate) fn end_scrollbar_drag(&mut self, now: Instant) -> ScrollResult {
        let Some(drag) = self.scrollbar_drag.take() else {
            return ScrollResult::default();
        };
        let state = self.scrollbar_states.entry(drag.id).or_default();
        state.dragging = false;
        if !state.hovered {
            state.visible_until = now.checked_add(SCROLLBAR_AUTO_HIDE_DELAY);
        }
        let view_dirty = self
            .virtual_scroll_handles
            .get(&drag.id)
            .is_some_and(|binding| binding.handle.scrollbar_drag_ended());
        ScrollResult {
            changed: true,
            view_dirty,
        }
    }

    pub(crate) fn scrollbar_drag_active(&self) -> bool {
        self.scrollbar_drag.is_some()
    }

    pub(crate) fn is_over_scrollbar(&self, point: Point) -> bool {
        self.scrollbar_at(point).is_some()
    }

    /// Resolve the topmost explicit web-style app-region declaration at `point`.
    ///
    /// The built-in overlay scrollbar always wins over an ancestor drag region.
    pub(crate) fn is_app_region_drag(&self, point: Point) -> bool {
        if self.is_over_scrollbar(point) {
            return false;
        }
        self.hit_regions
            .iter()
            .rev()
            .filter(|region| region.contains(point))
            .find_map(|region| region.app_region)
            == Some(AppRegion::Drag)
    }

    /// Update the topmost scrollbar edge under the pointer.
    ///
    /// The 12-point hit track remains available while the thumb itself is hidden. Only state
    /// transitions repaint, so stationary pointer events do not create extra frames.
    pub(crate) fn update_scrollbar_hover(&mut self, point: Option<Point>, now: Instant) -> bool {
        let next = point
            .and_then(|point| self.scrollbar_at(point))
            .map(|(region, _)| region.id);
        if next == self.hovered_scrollbar {
            return false;
        }

        if let Some(previous) = self.hovered_scrollbar.take() {
            let state = self.scrollbar_states.entry(previous).or_default();
            state.hovered = false;
            if !state.dragging {
                state.visible_until = now.checked_add(SCROLLBAR_AUTO_HIDE_DELAY);
            }
        }
        if let Some(next) = next {
            let state = self.scrollbar_states.entry(next).or_default();
            state.hovered = true;
            state.visible_until = None;
            self.hovered_scrollbar = Some(next);
        }
        true
    }

    fn scrollbar_at(&self, point: Point) -> Option<(ScrollRegion, VerticalScrollbarGeometry)> {
        let blocker = self
            .hit_regions
            .iter()
            .rev()
            .find(|region| region.blocks_pointer && region.contains(point))
            .map(|region| region.order);
        self.scroll_regions
            .iter()
            .filter_map(|region| {
                let offset = self
                    .scroll_offsets
                    .get(&region.id)
                    .copied()
                    .unwrap_or_default();
                let geometry = vertical_scrollbar_geometry(*region, offset.y)?;
                (region.clip.contains(point)
                    && geometry.track.contains(point)
                    && blocker.is_none_or(|blocker| blocker < region.scrollbar_order))
                .then_some((*region, geometry))
            })
            .max_by_key(|(region, _)| region.scrollbar_order)
    }

    pub fn pointer_button(
        &mut self,
        point: Option<Point>,
        pressed: bool,
        extend_selection: bool,
        now: Instant,
        renderer: &mut impl TextLayoutEngine,
    ) -> PointerResult {
        let tooltip_repaint = pressed && self.clear_tooltip();
        if pressed
            && let Some(dismiss) = self.dismiss_regions.last().copied()
            && dismiss.policy.on_pointer_outside()
            && point.is_none_or(|point| !dismiss.contains(point))
        {
            self.selecting_input = None;
            self.static_text_gesture = None;
            let repaint = self.pressed.take().is_some() | tooltip_repaint;
            return PointerResult {
                repaint,
                clicked: None,
                dismissed: Some(DismissRequest {
                    id: dismiss.id,
                    restore_focus: dismiss.restore_focus,
                }),
                pointer_listener: None,
                drag_source: None,
            };
        }

        let static_position = if pressed {
            point.and_then(|point| self.selectable_text_position_at(point, false, renderer))
        } else {
            None
        };
        let pointer_listener = static_position
            .is_none()
            .then(|| point.and_then(|point| self.pointer_listener_at(point)))
            .flatten();
        let drag_source = pointer_listener
            .is_none()
            .then(|| {
                static_position
                    .is_none()
                    .then(|| point.and_then(|point| self.drag_source_at(point)))
                    .flatten()
            })
            .flatten();
        let region = point.and_then(|point| self.interactive_region_at(point));
        let target = region
            .filter(|region| region.clickable)
            .map(|region| region.id);
        if pressed {
            let focus_changed = if static_position.is_some() {
                self.blur()
            } else {
                region
                    .filter(|region| region.focusable)
                    .is_some_and(|region| self.focus_from_pointer(region.id))
            };
            let selection_changed;
            self.selecting_input = None;
            if let (Some(point), Some(position)) = (point, static_position) {
                let unit = self.static_text_click_unit(point, position, now, extend_selection);
                let existing_anchor = if extend_selection {
                    self.static_text_selection
                        .map(|selection| selection.anchor)
                        .unwrap_or(position)
                } else {
                    position
                };
                let (base_start, base_end) = if extend_selection {
                    (existing_anchor, existing_anchor)
                } else {
                    static_selection_unit_range(
                        position,
                        unit,
                        &self.selectable_texts,
                        &self.selectable_text_indices,
                    )
                    .unwrap_or((position, position))
                };
                let next = if extend_selection {
                    StaticTextSelection {
                        anchor: existing_anchor,
                        focus: position,
                    }
                } else {
                    StaticTextSelection {
                        anchor: base_start,
                        focus: base_end,
                    }
                };
                selection_changed = self.static_text_selection != Some(next);
                self.static_text_selection = Some(next);
                self.static_text_gesture = Some(StaticTextGesture {
                    origin: point,
                    moved: unit != StaticTextSelectionUnit::Character,
                    unit,
                    base_start,
                    base_end,
                });
            } else if let (Some(point), Some(region)) = (point, region)
                && self.text_inputs.contains_key(&region.id)
                && let Some(input_region) = self
                    .text_input_regions
                    .iter()
                    .rev()
                    .find(|input| input.id == region.id)
                    .cloned()
            {
                let index = text_input_index_at(&input_region, point, self.scale_factor, renderer);
                let input_selection_changed = self
                    .text_inputs
                    .get_mut(&region.id)
                    .is_some_and(|state| state.move_to(index, extend_selection));
                self.selecting_input = Some(region.id);
                selection_changed =
                    input_selection_changed | self.static_text_selection.take().is_some();
                self.static_text_gesture = None;
            } else {
                selection_changed = self.static_text_selection.take().is_some();
                self.static_text_gesture = None;
            }
            let repaint =
                self.pressed != target || focus_changed || selection_changed || tooltip_repaint;
            self.pressed = target;
            PointerResult {
                repaint,
                clicked: None,
                dismissed: None,
                pointer_listener,
                drag_source,
            }
        } else {
            self.selecting_input = None;
            let suppress_click = self
                .static_text_gesture
                .take()
                .is_some_and(|gesture| gesture.moved);
            let clicked = self
                .pressed
                .filter(|pressed_id| !suppress_click && Some(*pressed_id) == target);
            let repaint = self.pressed.take().is_some();
            PointerResult {
                repaint,
                clicked,
                dismissed: None,
                pointer_listener: None,
                drag_source: None,
            }
        }
    }

    /// Apply platform content-motion deltas to the deepest scrollable region under the pointer.
    pub fn scroll_at(&mut self, point: Option<Point>, delta: Vector, now: Instant) -> ScrollResult {
        let Some(point) = point else {
            return ScrollResult::default();
        };
        let tooltip_changed = self.clear_tooltip();
        let blocker = self
            .hit_regions
            .iter()
            .rev()
            .find(|region| region.blocks_pointer && region.contains(point))
            .map(|region| region.order);
        for region in self.scroll_regions.iter().rev().copied() {
            if !region.bounds.contains(point) {
                continue;
            }
            if blocker.is_some_and(|blocker| region.order < blocker) {
                continue;
            }
            let offset = self.scroll_offsets.entry(region.id).or_default();
            let next = Vector::new(
                (offset.x - delta.x).clamp(0.0, region.max_offset.x),
                (offset.y - delta.y).clamp(0.0, region.max_offset.y),
            );
            if next != *offset {
                *offset = next;
                let state = self.scrollbar_states.entry(region.id).or_default();
                if !state.hovered && !state.dragging {
                    state.visible_until = now.checked_add(SCROLLBAR_AUTO_HIDE_DELAY);
                }
                let view_dirty = region.virtual_scroll
                    && self
                        .virtual_scroll_handles
                        .get(&region.id)
                        .is_none_or(|binding| {
                            binding.update_from_input(next.y, region.bounds.height)
                        });
                return ScrollResult {
                    changed: true,
                    view_dirty,
                };
            }
        }
        ScrollResult {
            changed: tooltip_changed,
            view_dirty: false,
        }
    }

    /// Return the topmost declared or inferred cursor at `point`.
    ///
    /// An explicit arrow is retained as `Some(Arrow)`, allowing a foreground element to reset a
    /// cursor inherited from a lower hit region without becoming a pointer blocker.
    pub(crate) fn cursor_style_at(&self, point: Point) -> Option<CursorStyle> {
        let inert_background_below = self
            .dismiss_regions
            .last()
            .filter(|region| region.order.layer.plane == crate::ScenePlane::Overlay)
            .map(|region| region.order);
        for region in self.hit_regions.iter().rev() {
            if inert_background_below.is_some_and(|order| region.order < order) {
                // Outside a dismissible overlay, the first press dismisses instead of activating
                // anything painted beneath its surface. Keep the platform arrow there instead of
                // advertising a stale text, link, or resize cursor from inert background content.
                return Some(CursorStyle::Arrow);
            }
            if !region.contains(point) {
                continue;
            }
            let interaction_cursor = if self.drag_over == Some(region.id) {
                region.cursor_states.drag_over
            } else if self.dragging == Some(region.id) {
                region.cursor_states.dragging
            } else if self.pressed == Some(region.id) {
                region.cursor_states.active
            } else if self.dragging.is_none() && !self.external_drag_active {
                region.cursor_states.hover
            } else {
                None
            };
            let cursor = interaction_cursor
                .or(region.cursor_states.invalid)
                .or_else(|| {
                    (self.focused == Some(region.id))
                        .then_some(region.cursor_states.focus)
                        .flatten()
                })
                .or(region.cursor_style);
            if let Some(cursor) = cursor {
                return Some(cursor);
            }
            if region.blocks_pointer || region.pointer_listener {
                return None;
            }
        }
        inert_background_below.map(|_| CursorStyle::Arrow)
    }

    fn selectable_text_position_at(
        &self,
        point: Point,
        allow_nearest: bool,
        renderer: &mut impl TextLayoutEngine,
    ) -> Option<StaticTextPosition> {
        let blocker = self
            .hit_regions
            .iter()
            .rev()
            .find(|region| {
                (region.blocks_pointer || region.pointer_listener) && region.contains(point)
            })
            .map(|region| region.order);
        let exact = self
            .selectable_text_regions
            .iter()
            .filter(|region| {
                region.clip.contains(point)
                    && region.bounds.contains(point)
                    && blocker.is_none_or(|blocker| blocker < region.order)
            })
            .max_by_key(|region| region.order);
        let region = exact.or_else(|| {
            allow_nearest
                .then(|| nearest_selectable_text_region(&self.selectable_text_regions, point))?
        })?;
        let entry = self.selectable_texts.get(region.document_index)?;
        let local = Point::new(
            (point.x - region.bounds.x).clamp(0.0, region.bounds.width),
            (point.y - region.bounds.y).clamp(0.0, region.bounds.height),
        );
        let offset = renderer.text_index_for_point_with_highlights(
            TextId::new(entry.id.value()),
            &entry.content,
            &region.style,
            region.highlights.as_ref(),
            region.bounds.width,
            self.scale_factor,
            local,
        );
        Some(StaticTextPosition {
            id: entry.id,
            offset: boundary_at_or_before(&entry.content, offset),
        })
    }

    fn static_text_click_unit(
        &mut self,
        point: Point,
        position: StaticTextPosition,
        now: Instant,
        extend: bool,
    ) -> StaticTextSelectionUnit {
        if extend {
            self.last_static_text_click = None;
            return StaticTextSelectionUnit::Character;
        }
        let count = self
            .last_static_text_click
            .filter(|click| {
                click.id == position.id
                    && now.saturating_duration_since(click.at) <= STATIC_TEXT_MULTI_CLICK_INTERVAL
                    && {
                        let delta = point - click.position;
                        delta.x * delta.x + delta.y * delta.y
                            <= STATIC_TEXT_MULTI_CLICK_DISTANCE * STATIC_TEXT_MULTI_CLICK_DISTANCE
                    }
            })
            .map_or(
                1,
                |click| if click.count >= 3 { 1 } else { click.count + 1 },
            );
        self.last_static_text_click = Some(StaticTextClick {
            position: point,
            id: position.id,
            at: now,
            count,
        });
        match count {
            2 => StaticTextSelectionUnit::Word,
            3 => StaticTextSelectionUnit::Line,
            _ => StaticTextSelectionUnit::Character,
        }
    }

    pub fn dismiss_topmost(&self) -> Option<DismissRequest> {
        self.dismiss_regions
            .last()
            .filter(|region| region.policy.on_escape())
            .map(|region| DismissRequest {
                id: region.id,
                restore_focus: region.restore_focus,
            })
    }

    pub(crate) fn dismiss_request_for_pointer(
        &self,
        point: Option<Point>,
    ) -> Option<DismissRequest> {
        let dismiss = self.dismiss_regions.last().copied()?;
        if !dismiss.policy.on_pointer_outside() {
            return None;
        }
        point
            .is_none_or(|point| !dismiss.contains(point))
            .then_some(DismissRequest {
                id: dismiss.id,
                restore_focus: dismiss.restore_focus,
            })
    }

    #[cfg(target_os = "macos")]
    pub fn native_views(&self) -> &[NativeViewPlacement] {
        &self.native_views
    }

    #[cfg(target_os = "macos")]
    pub fn overlay_input_active(&self) -> bool {
        self.hit_regions.iter().any(|region| {
            region.order.layer.plane == crate::ScenePlane::Overlay && region.blocks_pointer
        })
    }

    fn interactive_region_at(&self, point: Point) -> Option<HitRegion> {
        for region in self.hit_regions.iter().rev() {
            if !region.contains(point) {
                continue;
            }
            if region.clickable || region.focusable {
                return Some(*region);
            }
            if region.blocks_pointer || region.pointer_listener {
                return None;
            }
        }
        None
    }

    pub(crate) fn pointer_listener_at(&self, point: Point) -> Option<ElementId> {
        for region in self.hit_regions.iter().rev() {
            if !region.contains(point) {
                continue;
            }
            if region.pointer_listener {
                return Some(region.id);
            }
            if region.blocks_pointer {
                return None;
            }
        }
        None
    }

    pub(crate) fn context_menu_listener_at(&self, point: Point) -> Option<ElementId> {
        self.ancestor_at(point, |id| self.context_menu_ids.contains(&id))
    }

    pub(crate) fn scroll_wheel_listener_at(&self, point: Point) -> Option<ElementId> {
        if self.scroll_wheel_ids.is_empty() {
            return None;
        }
        let target = self
            .hit_regions
            .iter()
            .rev()
            .find(|region| region.contains(point))
            .map(|region| region.id)?;
        self.scroll_wheel_listener_for_target(target)
    }

    pub(crate) fn scroll_wheel_listener_for_target(&self, mut id: ElementId) -> Option<ElementId> {
        loop {
            if self.scroll_wheel_ids.contains(&id) {
                return Some(id);
            }
            id = self.parents.get(&id).copied()?;
        }
    }

    pub(crate) fn parent_scroll_wheel_listener(&self, id: ElementId) -> Option<ElementId> {
        let mut current = self.parents.get(&id).copied();
        while let Some(id) = current {
            if self.scroll_wheel_ids.contains(&id) {
                return Some(id);
            }
            current = self.parents.get(&id).copied();
        }
        None
    }

    pub(crate) fn touch_listener_at(&self, point: Point) -> Option<ElementId> {
        if self.touch_ids.is_empty() {
            return None;
        }
        self.ancestor_at(point, |id| self.touch_ids.contains(&id))
    }

    #[cfg(any(test, feature = "test-support"))]
    pub(crate) fn touch_listener_for_target(&self, mut id: ElementId) -> Option<ElementId> {
        loop {
            if self.touch_ids.contains(&id) {
                return Some(id);
            }
            id = self.parents.get(&id).copied()?;
        }
    }

    pub(crate) fn parent_touch_listener(&self, id: ElementId) -> Option<ElementId> {
        let mut current = self.parents.get(&id).copied();
        while let Some(id) = current {
            if self.touch_ids.contains(&id) {
                return Some(id);
            }
            current = self.parents.get(&id).copied();
        }
        None
    }

    pub(crate) fn mouse_pressure_listener_at(&self, point: Point) -> Option<ElementId> {
        self.ancestor_at(point, |id| self.mouse_pressure_ids.contains(&id))
    }

    pub(crate) fn pinch_listener_at(&self, point: Point) -> Option<ElementId> {
        self.ancestor_at(point, |id| self.pinch_ids.contains(&id))
    }

    pub(crate) fn rotation_listener_at(&self, point: Point) -> Option<ElementId> {
        self.ancestor_at(point, |id| self.rotation_ids.contains(&id))
    }

    pub(crate) fn smart_magnify_listener_at(&self, point: Point) -> Option<ElementId> {
        self.ancestor_at(point, |id| self.smart_magnify_ids.contains(&id))
    }

    fn tooltip_target_at(&self, point: Point) -> Option<ElementId> {
        self.ancestor_at(point, |id| self.tooltips.contains_key(&id))
    }

    fn ancestor_at(
        &self,
        point: Point,
        mut predicate: impl FnMut(ElementId) -> bool,
    ) -> Option<ElementId> {
        let mut current = self
            .hit_regions
            .iter()
            .rev()
            .find(|region| region.contains(point))
            .map(|region| region.id)?;
        loop {
            if predicate(current) {
                return Some(current);
            }
            current = self.parents.get(&current).copied()?;
        }
    }

    /// Fill a bounded target-to-root path for the topmost retained hit region at `point`.
    ///
    /// A tree deeper than the public dispatch bound fails closed and produces no partial event.
    pub(crate) fn mouse_event_path_at(&self, point: Point, output: &mut Vec<ElementId>) -> bool {
        output.clear();
        let Some(mut current) = self
            .hit_regions
            .iter()
            .rev()
            .find(|region| region.contains(point))
            .map(|region| region.id)
        else {
            return true;
        };
        loop {
            if output.len() == MAX_MOUSE_EVENT_PATH {
                output.clear();
                return false;
            }
            output.push(current);
            let Some(parent) = self.parents.get(&current).copied() else {
                return true;
            };
            current = parent;
        }
    }

    #[cfg(any(test, feature = "test-support"))]
    pub(crate) fn mouse_event_path_for_target(
        &self,
        mut target: ElementId,
        output: &mut Vec<ElementId>,
    ) -> bool {
        output.clear();
        loop {
            if output.len() == MAX_MOUSE_EVENT_PATH {
                output.clear();
                return false;
            }
            output.push(target);
            let Some(parent) = self.parents.get(&target).copied() else {
                return true;
            };
            target = parent;
        }
    }

    /// Collect outside capture handlers, then root-to-target capture and target-to-root bubble.
    pub(crate) fn collect_mouse_dispatch(
        &self,
        path: &[ElementId],
        kind: MouseListenerKind,
        button: Option<MouseButton>,
        output: &mut Vec<MouseListenerKey>,
    ) {
        output.clear();

        // Outside listeners behave like document capture: topmost/deepest declarations get the
        // first chance to close transient UI before the in-path target sees the press.
        if matches!(kind, MouseListenerKind::Down | MouseListenerKind::Up) {
            if self.hit_regions.is_empty() {
                for id in self.mouse_listener_elements.iter().rev().copied() {
                    if path.contains(&id) {
                        continue;
                    }
                    self.extend_mouse_dispatch_for(
                        id,
                        kind,
                        DispatchPhase::Capture,
                        button,
                        true,
                        output,
                    );
                }
            } else {
                for id in self.hit_regions.iter().rev().map(|region| region.id) {
                    if path.contains(&id) {
                        continue;
                    }
                    self.extend_mouse_dispatch_for(
                        id,
                        kind,
                        DispatchPhase::Capture,
                        button,
                        true,
                        output,
                    );
                }
            }
        }

        for id in path.iter().rev().copied() {
            self.extend_mouse_dispatch_for(id, kind, DispatchPhase::Capture, button, false, output);
        }
        for id in path.iter().copied() {
            self.extend_mouse_dispatch_for(id, kind, DispatchPhase::Bubble, button, false, output);
        }
    }

    fn extend_mouse_dispatch_for(
        &self,
        id: ElementId,
        kind: MouseListenerKind,
        phase: DispatchPhase,
        button: Option<MouseButton>,
        outside: bool,
        output: &mut Vec<MouseListenerKey>,
    ) {
        let Some(range) = self.mouse_listener_ranges.get(&id).cloned() else {
            return;
        };
        for binding in self.mouse_listener_bindings[range].iter().copied() {
            if binding.kind == kind
                && binding.phase == phase
                && binding.outside == outside
                && binding
                    .button
                    .is_none_or(|expected| button == Some(expected))
            {
                output.push(binding.key);
            }
        }
    }

    pub(crate) fn drag_source_at(&self, point: Point) -> Option<ElementId> {
        for region in self.hit_regions.iter().rev() {
            if !region.contains(point) {
                continue;
            }
            if region.drag_source {
                return Some(region.id);
            }
            if region.blocks_pointer || region.pointer_listener {
                return None;
            }
        }
        None
    }

    pub(crate) fn drop_target_at(
        &self,
        point: Point,
        accepts: impl Fn(ElementId) -> bool,
    ) -> Option<ElementId> {
        for region in self.hit_regions.iter().rev() {
            if !region.contains(point) {
                continue;
            }
            if region.drop_target && accepts(region.id) {
                return Some(region.id);
            }
            if region.blocks_pointer || region.pointer_listener {
                return None;
            }
        }
        None
    }

    pub(crate) fn drop_offer_target_at<'a, I>(
        &self,
        point: Point,
        offers: I,
        accepts: impl Fn(ElementId, TypeId, &dyn Any) -> bool,
    ) -> Option<(ElementId, usize)>
    where
        I: Iterator<Item = (TypeId, &'a dyn Any)> + Clone,
    {
        for region in self.hit_regions.iter().rev() {
            if !region.contains(point) {
                continue;
            }
            if region.drop_target {
                for (index, (value_type, value)) in offers.clone().enumerate() {
                    if accepts(region.id, value_type, value) {
                        return Some((region.id, index));
                    }
                }
            }
            if region.blocks_pointer || region.pointer_listener {
                return None;
            }
        }
        None
    }

    pub(crate) fn can_drop(&self, target: ElementId, value_type: TypeId, value: &dyn Any) -> bool {
        self.drop_predicates
            .get(&(target, value_type))
            .is_none_or(|predicate| predicate(value))
    }

    /// Refresh the platform-facing typed drop topology without retaining the full UI tree.
    ///
    /// The snapshot follows the same topmost-first blocker semantics as [`Self::drop_target_at`]
    /// and reuses its allocations across damage frames. Listener order is supplied by the view's
    /// stable callback registry so reaching a bound stays deterministic.
    #[cfg(target_os = "macos")]
    pub(crate) fn update_external_drop_snapshot(
        &self,
        snapshot: &mut ExternalDropSnapshot,
        listeners: &[(ElementId, TypeId)],
    ) {
        snapshot.clear();
        snapshot.acceptances_truncated = listeners.len() > MAX_EXTERNAL_DROP_ACCEPTANCES;
        for &(id, value_type) in listeners.iter().take(MAX_EXTERNAL_DROP_ACCEPTANCES) {
            snapshot.target_ids.insert(id);
            snapshot.acceptances.insert(
                (id, value_type),
                drop_acceptance(&self.drop_predicates, id, value_type),
            );
        }
        for region in self.hit_regions.iter().rev() {
            let relevant = (region.drop_target && snapshot.target_ids.contains(&region.id))
                || region.blocks_pointer
                || region.pointer_listener;
            if !relevant {
                continue;
            }
            if snapshot.regions.len() == MAX_EXTERNAL_DROP_HIT_REGIONS {
                snapshot.regions_truncated = true;
                break;
            }
            snapshot.regions.push(ExternalDropHitRegion {
                id: region.id,
                bounds: region.bounds,
                clip: region.clip,
                drop_target: region.drop_target,
                blocks_pointer: region.blocks_pointer,
                pointer_listener: region.pointer_listener,
            });
        }
    }

    pub fn focused_text_input(&self) -> Option<ElementId> {
        self.focused
            .filter(|focused| self.text_inputs.contains_key(focused))
    }

    pub(crate) fn focused_text_input_is_multiline(&self) -> bool {
        self.focused_text_input()
            .and_then(|id| self.text_inputs.get(&id))
            .is_some_and(TextInputState::is_multiline)
    }

    pub(crate) fn focused_text_input_is_invalid(&self) -> bool {
        self.focused_text_input()
            .is_some_and(|id| self.invalid_ids.contains(&id))
    }

    pub(crate) fn focused_text_input_value(&self) -> Option<Arc<str>> {
        self.focused_text_input()
            .and_then(|id| self.text_inputs.get(&id))
            .map(TextInputState::committed_shared_text)
    }

    /// Find the nearest semantic form containing a control.
    pub(crate) fn form_for_control(&self, mut id: ElementId) -> Option<ElementId> {
        loop {
            if self.form_ids.contains(&id) {
                return Some(id);
            }
            id = self.parents.get(&id).copied()?;
        }
    }

    /// Return the nearest form only when this element opted into submit-button behavior.
    pub(crate) fn form_for_submitter(&self, id: ElementId) -> Option<ElementId> {
        self.form_submitter_ids
            .contains(&id)
            .then(|| self.form_for_control(id))
            .flatten()
    }

    pub(crate) fn activation_target(&self, id: ElementId) -> Option<ElementId> {
        self.activation_targets.get(&id).copied()
    }

    /// Validate one mounted form, retaining only bounded shared values and issue metadata.
    ///
    /// Invalid attempts synchronously focus the first focusable invalid control in document order
    /// and replace the one retained AccessKit live announcement. No timer or idle frame is added.
    pub(crate) fn attempt_form_submission(
        &mut self,
        form: ElementId,
        trigger: Option<ElementId>,
    ) -> Option<FormAttempt> {
        if !self.form_ids.contains(&form) {
            return None;
        }
        let root = self.root.as_ref()?;
        let form_element = find_element(root, form)?;
        let mut collection = FormCollection::default();
        for child in &form_element.children {
            collect_form_controls(
                child,
                &self.text_inputs,
                &self.focusable_ids,
                &mut collection,
            );
        }

        if collection.issues.is_empty() {
            self.validation_announcement = None;
            return Some(FormAttempt::Valid(FormSubmitEvent::new(
                form,
                trigger,
                collection.fields,
                collection.fields_truncated,
            )));
        }

        if let Some(first_focusable) = collection.first_focusable_invalid {
            self.focus(first_focusable);
        }
        let report = ValidationReport::new(
            form,
            trigger,
            collection.issues,
            collection.issues_truncated,
        );
        self.set_validation_announcement(&report);
        Some(FormAttempt::Invalid(report))
    }

    fn set_validation_announcement(&mut self, report: &ValidationReport) {
        let message = validation_announcement_message(report);
        let node = self.next_auxiliary_accessibility_id();
        self.validation_announcement = Some(ValidationAnnouncement {
            form: report.form(),
            node,
            message,
        });
    }

    fn next_auxiliary_accessibility_id(&mut self) -> AccessibilityNodeId {
        loop {
            let candidate = AccessibilityNodeId(self.next_accessibility_text_id);
            self.next_accessibility_text_id = self.next_accessibility_text_id.wrapping_sub(1);
            if candidate != ACCESSIBILITY_ROOT_ID
                && !self.seen_ids.contains(&ElementId::new(candidate.0))
                && !self
                    .accessibility_text_ids
                    .values()
                    .any(|existing| *existing == candidate)
                && self
                    .validation_announcement
                    .as_ref()
                    .is_none_or(|announcement| announcement.node != candidate)
            {
                return candidate;
            }
        }
    }

    pub(crate) fn selected_input_text(&self) -> Option<Arc<str>> {
        let id = self.focused_text_input()?;
        if self
            .root
            .as_ref()
            .and_then(|root| find_element(root, id))
            .is_some_and(|element| {
                matches!(
                    &element.kind,
                    ElementKind::TextInput(input) if input.password
                )
            })
        {
            return None;
        }
        self.text_inputs
            .get(&id)
            .and_then(TextInputState::selected_text)
            .map(Arc::from)
    }

    pub fn selected_text(&self) -> Option<Arc<str>> {
        self.selected_input_text()
            .or_else(|| self.selected_static_text())
    }

    pub(crate) fn has_selectable_text(&self) -> bool {
        !self.selectable_texts.is_empty()
    }

    pub(crate) fn select_all_static_text(&mut self) -> bool {
        let (Some(first), Some(last)) =
            (self.selectable_texts.first(), self.selectable_texts.last())
        else {
            return false;
        };
        let next = StaticTextSelection {
            anchor: StaticTextPosition {
                id: first.id,
                offset: 0,
            },
            focus: StaticTextPosition {
                id: last.id,
                offset: last.content.len(),
            },
        };
        let changed = self.static_text_selection != Some(next);
        self.static_text_selection = Some(next);
        self.static_text_gesture = None;
        changed
    }

    pub(crate) fn clear_static_text_selection(&mut self) -> bool {
        self.static_text_gesture = None;
        self.static_text_selection.take().is_some()
    }

    fn selected_static_text(&self) -> Option<Arc<str>> {
        let selection = self.static_text_selection?;
        let (start, end) = normalized_static_selection(selection, &self.selectable_text_indices)?;
        if start == end {
            return None;
        }
        let mut output = String::new();
        for document_index in start.0..=end.0 {
            let entry = self.selectable_texts.get(document_index)?;
            let range = if start.0 == end.0 {
                start.1.min(entry.content.len())..end.1.min(entry.content.len())
            } else if document_index == start.0 {
                start.1.min(entry.content.len())..entry.content.len()
            } else if document_index == end.0 {
                0..end.1.min(entry.content.len())
            } else {
                0..entry.content.len()
            };
            if document_index > start.0 {
                let previous = &self.selectable_texts[document_index - 1];
                let separator = selection_separator(
                    self.element_bounds.get(&previous.id).copied(),
                    self.element_bounds.get(&entry.id).copied(),
                );
                push_bounded_text(&mut output, separator, MAX_STATIC_TEXT_COPY_BYTES);
            }
            push_bounded_text(
                &mut output,
                &entry.content[range],
                MAX_STATIC_TEXT_COPY_BYTES,
            );
            if output.len() == MAX_STATIC_TEXT_COPY_BYTES {
                break;
            }
        }
        (!output.is_empty()).then(|| Arc::from(output))
    }

    pub fn input_move_left(&mut self, extend: bool) -> InputResult {
        self.edit_focused_input(|state| state.move_left(extend))
    }

    pub fn input_move_right(&mut self, extend: bool) -> InputResult {
        self.edit_focused_input(|state| state.move_right(extend))
    }

    pub fn input_move_word_left(&mut self, extend: bool) -> InputResult {
        self.edit_focused_input(|state| state.move_word_left(extend))
    }

    pub fn input_move_word_right(&mut self, extend: bool) -> InputResult {
        self.edit_focused_input(|state| state.move_word_right(extend))
    }

    pub fn input_move_home(&mut self, extend: bool) -> InputResult {
        self.edit_focused_input(|state| state.move_home(extend))
    }

    pub fn input_move_end(&mut self, extend: bool) -> InputResult {
        self.edit_focused_input(|state| state.move_end(extend))
    }

    pub fn input_move_line_start(&mut self, extend: bool) -> InputResult {
        self.edit_focused_input(|state| state.move_line_start(extend))
    }

    pub fn input_move_line_end(&mut self, extend: bool) -> InputResult {
        self.edit_focused_input(|state| state.move_line_end(extend))
    }

    pub fn input_move_vertical(
        &mut self,
        lines: isize,
        extend: bool,
        renderer: &mut impl TextLayoutEngine,
    ) -> InputResult {
        if lines == 0 {
            return InputResult::default();
        }
        let Some(id) = self.focused_text_input() else {
            return InputResult::default();
        };
        let Some(region) = self
            .text_input_regions
            .iter()
            .rev()
            .find(|region| region.id == id)
            .cloned()
        else {
            return InputResult::default();
        };
        let Some(state) = self.text_inputs.get_mut(&id) else {
            return InputResult::default();
        };
        if !state.is_multiline() {
            return InputResult::default();
        }

        let mut repaint = false;
        let selection = state.selection();
        if !extend && !selection.is_empty() {
            let collapse = if lines < 0 {
                selection.start
            } else {
                selection.end
            };
            repaint |= state.move_to(collapse, false);
        }

        let content = state.shared_text();
        let text_id = TextId::new(id.value());
        let caret = renderer.text_caret_position_with_highlights(
            text_id,
            &content,
            &region.style,
            region.highlights.as_ref(),
            region.bounds.width,
            self.scale_factor,
            state.caret(),
        );
        let preferred_x = state.preferred_x().unwrap_or(caret.x);
        let target = Point::new(
            preferred_x,
            caret.y + lines as f32 * region.style.line_height + region.style.line_height * 0.5,
        );
        let index = renderer.text_index_for_point_with_highlights(
            text_id,
            &content,
            &region.style,
            region.highlights.as_ref(),
            region.bounds.width,
            self.scale_factor,
            target,
        );
        repaint |= state.move_to_with_preferred_x(index, extend, preferred_x);
        InputResult {
            repaint,
            change: None,
        }
    }

    pub fn input_move_page(
        &mut self,
        direction: isize,
        extend: bool,
        renderer: &mut impl TextLayoutEngine,
    ) -> InputResult {
        let Some(id) = self.focused_text_input() else {
            return InputResult::default();
        };
        let Some(region) = self
            .text_input_regions
            .iter()
            .rev()
            .find(|region| region.id == id)
        else {
            return InputResult::default();
        };
        let lines = (region.bounds.height / region.style.line_height)
            .floor()
            .max(1.0) as isize;
        self.input_move_vertical(direction.signum() * lines, extend, renderer)
    }

    pub fn input_select_all(&mut self) -> InputResult {
        self.edit_focused_input(TextInputState::select_all)
    }

    pub fn input_backspace(&mut self) -> InputResult {
        self.edit_focused_input(TextInputState::backspace)
    }

    pub fn input_delete(&mut self) -> InputResult {
        self.edit_focused_input(TextInputState::delete)
    }

    pub fn input_delete_word_backward(&mut self) -> InputResult {
        self.edit_focused_input(TextInputState::delete_word_backward)
    }

    pub fn input_delete_word_forward(&mut self) -> InputResult {
        self.edit_focused_input(TextInputState::delete_word_forward)
    }

    pub fn input_delete_to_line_start(&mut self) -> InputResult {
        self.edit_focused_input(TextInputState::delete_to_line_start)
    }

    pub fn input_delete_to_line_end(&mut self) -> InputResult {
        self.edit_focused_input(TextInputState::delete_to_line_end)
    }

    pub fn input_replace(&mut self, value: &str) -> InputResult {
        self.edit_focused_input(|state| state.replace_selection(value))
    }

    pub fn input_insert_newline(&mut self) -> InputResult {
        self.edit_focused_input(TextInputState::insert_newline)
    }

    pub fn input_can_undo(&self) -> bool {
        self.focused_text_input()
            .and_then(|id| self.text_inputs.get(&id))
            .is_some_and(TextInputState::can_undo)
    }

    pub fn input_can_redo(&self) -> bool {
        self.focused_text_input()
            .and_then(|id| self.text_inputs.get(&id))
            .is_some_and(TextInputState::can_redo)
    }

    pub fn input_undo(&mut self) -> InputResult {
        self.edit_focused_input(TextInputState::undo)
    }

    pub fn input_redo(&mut self) -> InputResult {
        self.edit_focused_input(TextInputState::redo)
    }

    pub fn input_preedit(&mut self, value: &str, cursor: Option<(usize, usize)>) -> InputResult {
        self.edit_focused_input(|state| state.set_preedit(value, cursor))
    }

    pub fn input_cancel_preedit(&mut self, id: ElementId) -> InputResult {
        self.edit_input(id, |state| state.set_preedit("", None))
    }

    pub fn input_set_value(&mut self, id: ElementId, value: &str) -> InputResult {
        self.edit_input(id, |state| state.set_value(value))
    }

    pub fn set_accessibility_text_selection(
        &mut self,
        id: ElementId,
        selection: &TextSelection,
    ) -> InputResult {
        let Some(text_id) = self.accessibility_text_ids.get(&id).copied() else {
            return InputResult::default();
        };
        if selection.anchor.node != text_id || selection.focus.node != text_id {
            return InputResult::default();
        }
        if self.text_inputs.contains_key(&id) {
            return self.edit_input(id, |state| {
                let anchor = state.accessibility_byte_index(selection.anchor.character_index);
                let caret = state.accessibility_byte_index(selection.focus.character_index);
                state.set_selection(anchor, caret)
            });
        }
        let Some(document_index) = self.selectable_text_indices.get(&id).copied() else {
            return InputResult::default();
        };
        let content = &self.selectable_texts[document_index].content;
        let next = StaticTextSelection {
            anchor: StaticTextPosition {
                id,
                offset: accessibility_byte_index(content, selection.anchor.character_index),
            },
            focus: StaticTextPosition {
                id,
                offset: accessibility_byte_index(content, selection.focus.character_index),
            },
        };
        let repaint = self.static_text_selection != Some(next);
        self.static_text_selection = Some(next);
        self.static_text_gesture = None;
        InputResult {
            repaint,
            change: None,
        }
    }

    pub fn ime_cursor_area(&self) -> Option<Rect> {
        let focused = self.focused_text_input()?;
        self.text_input_regions
            .iter()
            .rev()
            .find(|region| region.id == focused)
            .and_then(|region| region.caret_bounds.intersection(region.clip))
    }

    fn edit_focused_input(
        &mut self,
        edit: impl FnOnce(&mut TextInputState) -> bool,
    ) -> InputResult {
        let Some(id) = self.focused_text_input() else {
            return InputResult::default();
        };
        self.edit_input(id, edit)
    }

    fn edit_input(
        &mut self,
        id: ElementId,
        edit: impl FnOnce(&mut TextInputState) -> bool,
    ) -> InputResult {
        let Some(state) = self.text_inputs.get_mut(&id) else {
            return InputResult::default();
        };
        let previous = state.committed_shared_text();
        let repaint = edit(state);
        let committed = state.committed_shared_text();
        let change = (previous != committed).then_some(InputChange {
            id,
            value: committed,
        });
        InputResult { repaint, change }
    }

    pub fn focused(&self) -> Option<ElementId> {
        self.focused
    }

    pub(crate) fn is_focusable(&self, id: ElementId) -> bool {
        self.focusable_ids.contains(&id)
    }

    /// Return the retained root-to-focus element path used for scoped command dispatch.
    pub fn focus_path(&self) -> Vec<ElementId> {
        let Some(root) = self.root.as_ref().map(|root| root.runtime_id) else {
            return Vec::new();
        };
        let Some(mut current) = self.focused else {
            return vec![root];
        };
        let mut path = Vec::with_capacity(8);
        path.push(current);
        while let Some(parent) = self.parents.get(&current).copied() {
            if path.len() == MAX_FOCUSED_EVENT_PATH {
                return vec![root];
            }
            path.push(parent);
            current = parent;
        }
        path.reverse();
        if path.first().copied() != Some(root) {
            vec![root]
        } else {
            path
        }
    }

    /// Collect root-to-focus capture and focus-to-root bubble key listeners.
    pub(crate) fn collect_key_dispatch(
        &self,
        path: &[ElementId],
        kind: KeyListenerKind,
        output: &mut Vec<KeyListenerBinding>,
    ) {
        output.clear();
        for id in path.iter().copied() {
            self.extend_key_dispatch_for(id, kind, DispatchPhase::Capture, output);
        }
        for id in path.iter().rev().copied() {
            self.extend_key_dispatch_for(id, kind, DispatchPhase::Bubble, output);
        }
    }

    fn extend_key_dispatch_for(
        &self,
        id: ElementId,
        kind: KeyListenerKind,
        phase: DispatchPhase,
        output: &mut Vec<KeyListenerBinding>,
    ) {
        let Some(range) = self.key_listener_ranges.get(&id).cloned() else {
            return;
        };
        output.extend(
            self.key_listener_bindings[range]
                .iter()
                .copied()
                .filter(|binding| binding.kind == kind && binding.phase == phase),
        );
    }

    /// Collect typed action capture listeners before bubble listeners for the focused path.
    pub(crate) fn collect_action_dispatch(
        &self,
        path: &[ElementId],
        action_type: TypeId,
        output: &mut Vec<ActionListenerBinding>,
    ) {
        output.clear();
        for id in path.iter().copied() {
            self.extend_action_dispatch_for(id, action_type, DispatchPhase::Capture, output);
        }
        for id in path.iter().rev().copied() {
            self.extend_action_dispatch_for(id, action_type, DispatchPhase::Bubble, output);
        }
    }

    fn extend_action_dispatch_for(
        &self,
        id: ElementId,
        action_type: TypeId,
        phase: DispatchPhase,
        output: &mut Vec<ActionListenerBinding>,
    ) {
        let Some(range) = self.action_listener_ranges.get(&id).cloned() else {
            return;
        };
        output.extend(
            self.action_listener_bindings[range]
                .iter()
                .copied()
                .filter(|binding| binding.action_type == action_type && binding.phase == phase),
        );
    }

    pub(crate) fn action_available(&self, path: &[ElementId], action_type: TypeId) -> bool {
        path.iter().copied().any(|id| {
            self.action_listener_ranges.get(&id).is_some_and(|range| {
                self.action_listener_bindings[range.clone()]
                    .iter()
                    .any(|binding| binding.action_type == action_type)
            })
        })
    }

    pub fn key_context_stack(&self) -> Vec<KeyContext> {
        self.focus_path()
            .into_iter()
            .filter_map(|id| self.key_contexts.get(&id).cloned())
            .collect()
    }

    pub fn focus(&mut self, id: ElementId) -> bool {
        self.set_focus(id, true)
    }

    fn focus_from_pointer(&mut self, id: ElementId) -> bool {
        self.set_focus(id, false)
    }

    fn set_focus(&mut self, id: ElementId, show_tooltip: bool) -> bool {
        if !self.focusable_ids.contains(&id) {
            return false;
        }
        let mut changed = self.focused != Some(id);
        self.focused = Some(id);
        if self.text_inputs.contains_key(&id) {
            changed |= self.clear_static_text_selection();
        }
        if changed && show_tooltip {
            changed |= self.reconcile_tooltip(Instant::now());
        }
        changed
    }

    pub fn blur(&mut self) -> bool {
        let changed = self.focused.take().is_some();
        if changed {
            changed | self.reconcile_tooltip(Instant::now())
        } else {
            false
        }
    }

    pub fn focus_next(&mut self, reverse: bool) -> bool {
        if self.focus_order.is_empty() {
            return false;
        }
        let current_index = self
            .focused
            .and_then(|focused| self.focus_order.iter().position(|id| *id == focused))
            .or_else(|| {
                let root = self.root.as_ref()?;
                let tab_stop = tab_stop_for_focused_tab(root, self.focused?)?;
                self.focus_order.iter().position(|id| *id == tab_stop)
            });
        let next_index = match current_index {
            Some(index) if reverse => index.checked_sub(1).unwrap_or(self.focus_order.len() - 1),
            Some(index) => (index + 1) % self.focus_order.len(),
            None if reverse => self.focus_order.len() - 1,
            None => 0,
        };
        let next = self.focus_order[next_index];
        self.focus(next)
    }

    pub fn activate_focused(&self) -> Option<ElementId> {
        self.focused.filter(|focused| {
            self.clickable_ids.contains(focused) && !self.text_inputs.contains_key(focused)
        })
    }

    /// Find the previous or next enabled radio in the focused radio group, wrapping at its ends.
    ///
    /// The tree is scanned only for an explicit keyboard arrow action. No group registry,
    /// observer, allocation, or idle work is retained between events.
    pub(crate) fn adjacent_radio(&self, reverse: bool) -> Option<ElementId> {
        let focused = self.focused?;
        let root = self.root.as_ref()?;
        let group = radio_group_for_focused(root, focused)?;
        let mut neighbors = RadioNeighbors::default();
        collect_radio_neighbors(group, group.runtime_id, focused, &mut neighbors);
        if !neighbors.found_focus {
            return None;
        }
        if reverse {
            neighbors.previous.or(neighbors.last)
        } else {
            neighbors.next.or(neighbors.first)
        }
    }

    /// Find the previous or next enabled tab in the focused tab list.
    ///
    /// The mounted tree is scanned only for the explicit arrow-key event. No component registry,
    /// observer, allocation, or idle source is retained.
    pub(crate) fn adjacent_tab(
        &self,
        vertical_axis: bool,
        reverse: bool,
    ) -> Option<TabNavigationTarget> {
        let focused = self.focused?;
        let root = self.root.as_ref()?;
        let list = tab_list_for_focused(root, focused)?;
        let behavior = list.tab_list_behavior?;
        if behavior.vertical != vertical_axis {
            return None;
        }
        let mut neighbors = TabNeighbors::default();
        collect_tab_neighbors(list, list.runtime_id, focused, &mut neighbors);
        if !neighbors.found_focus {
            return None;
        }
        let id = if reverse {
            neighbors
                .previous
                .or_else(|| behavior.loop_focus.then_some(neighbors.last).flatten())
        } else {
            neighbors
                .next
                .or_else(|| behavior.loop_focus.then_some(neighbors.first).flatten())
        }?;
        Some(TabNavigationTarget {
            id,
            activate: behavior.activate_on_focus,
        })
    }

    /// Find the first or last enabled tab in the focused tab list for Home/End.
    pub(crate) fn edge_tab(&self, last: bool) -> Option<TabNavigationTarget> {
        let focused = self.focused?;
        let root = self.root.as_ref()?;
        let list = tab_list_for_focused(root, focused)?;
        let behavior = list.tab_list_behavior?;
        let mut neighbors = TabNeighbors::default();
        collect_tab_neighbors(list, list.runtime_id, focused, &mut neighbors);
        if !neighbors.found_focus {
            return None;
        }
        Some(TabNavigationTarget {
            id: if last {
                neighbors.last?
            } else {
                neighbors.first?
            },
            activate: behavior.activate_on_focus,
        })
    }

    pub fn accessibility_element(&self, id: AccessibilityNodeId) -> Option<ElementId> {
        if id == ACCESSIBILITY_ROOT_ID {
            return None;
        }
        if let Some((input, _)) = self
            .accessibility_text_ids
            .iter()
            .find(|(_, text_id)| **text_id == id)
        {
            return self
                .root
                .as_ref()
                .is_some_and(|root| accessibility_tree_contains(root, *input))
                .then_some(*input);
        }
        let id = ElementId::new(id.0);
        self.root
            .as_ref()
            .is_some_and(|root| accessibility_tree_contains(root, id))
            .then_some(id)
    }

    pub fn accessibility_update(&self, window_title: &str) -> TreeUpdate {
        let mut accessible_ids = HashSet::with_capacity(self.visible_ids.len());
        if let Some(element) = &self.root {
            collect_accessible_ids(element, &mut accessible_ids);
        }
        let mut nodes = Vec::with_capacity(
            accessible_ids.len() + usize::from(self.validation_announcement.is_some()),
        );
        let mut root = AccessibilityNode::new(Role::Window);
        root.set_bounds(accessibility_rect(Rect::from_size(self.viewport)));
        root.set_transform(Affine::scale(self.scale_factor as f64));
        root.set_label(window_title);
        let mut root_children = Vec::with_capacity(2);
        if let Some(element) = &self.root
            && accessible_ids.contains(&element.runtime_id)
        {
            root_children.push(accessibility_id(element.runtime_id));
            let context = AccessibilityBuildContext {
                element_bounds: &self.element_bounds,
                scroll_offsets: &self.scroll_offsets,
                text_inputs: &self.text_inputs,
                selectable_texts: &self.selectable_texts,
                selectable_text_indices: &self.selectable_text_indices,
                static_text_selection: self.static_text_selection,
                accessibility_text_ids: &self.accessibility_text_ids,
                accessible_ids: &accessible_ids,
            };
            build_accessibility_nodes(
                element,
                &context,
                &mut nodes,
                Vector::ZERO,
                AccessibilityBuildMode::Full,
            );
        }
        if let Some(announcement) = &self.validation_announcement {
            root_children.push(announcement.node);
            let mut alert = AccessibilityNode::new(Role::Alert);
            alert.set_value(announcement.message.to_string());
            alert.set_live(Live::Assertive);
            alert.set_live_atomic();
            nodes.push((announcement.node, alert));
        }
        root.set_children(root_children);
        nodes.insert(0, (ACCESSIBILITY_ROOT_ID, root));
        TreeUpdate {
            nodes,
            tree: Some(Tree::new(ACCESSIBILITY_ROOT_ID)),
            tree_id: TreeId::ROOT,
            focus: self
                .focused
                .filter(|id| accessible_ids.contains(id))
                .map(accessibility_id)
                .unwrap_or(ACCESSIBILITY_ROOT_ID),
        }
    }

    /// Update only scrolling containers after a retained scroll. Full accessibility trees encode
    /// descendants in stable, unscrolled coordinates and put the live translation on the scroll
    /// container, so AccessKit does not need to diff every text node while the viewport moves.
    pub fn accessibility_scroll_update(&self) -> TreeUpdate {
        let mut accessible_ids = HashSet::with_capacity(self.visible_ids.len());
        if let Some(element) = &self.root {
            collect_accessible_ids(element, &mut accessible_ids);
        }
        let mut nodes = Vec::with_capacity(self.scroll_offsets.len());
        if let Some(element) = &self.root
            && accessible_ids.contains(&element.runtime_id)
        {
            let context = AccessibilityBuildContext {
                element_bounds: &self.element_bounds,
                scroll_offsets: &self.scroll_offsets,
                text_inputs: &self.text_inputs,
                selectable_texts: &self.selectable_texts,
                selectable_text_indices: &self.selectable_text_indices,
                static_text_selection: self.static_text_selection,
                accessibility_text_ids: &self.accessibility_text_ids,
                accessible_ids: &accessible_ids,
            };
            build_accessibility_nodes(
                element,
                &context,
                &mut nodes,
                Vector::ZERO,
                AccessibilityBuildMode::ScrollContainers,
            );
        }
        TreeUpdate {
            nodes,
            tree: None,
            tree_id: TreeId::ROOT,
            focus: self
                .focused
                .filter(|id| accessible_ids.contains(id))
                .map(accessibility_id)
                .unwrap_or(ACCESSIBILITY_ROOT_ID),
        }
    }

    fn rebuild_focus_index(&mut self) {
        self.focusable_ids.clear();
        self.clickable_ids.clear();
        self.focus_order.clear();
        let Some(root) = &self.root else {
            self.active_focus_trap = None;
            return;
        };
        let active_focus_trap = topmost_focus_trap(root);
        self.active_focus_trap = active_focus_trap;
        let mut candidates = Vec::new();
        let mut collection = FocusCollection {
            focusable_ids: &mut self.focusable_ids,
            clickable_ids: &mut self.clickable_ids,
            candidates: &mut candidates,
            active_trap: active_focus_trap,
        };
        collect_focus_candidates(root, &mut collection, false, None, false, None, false);
        candidates.sort_by_key(|candidate| {
            let group = if candidate.tab_index > 0 { 0 } else { 1 };
            let tab_index = if candidate.tab_index > 0 {
                candidate.tab_index
            } else {
                0
            };
            (group, tab_index, candidate.order)
        });
        self.focus_order
            .extend(candidates.into_iter().map(|candidate| candidate.id));
        if let Some(trap) = active_focus_trap
            && self
                .focused
                .is_none_or(|focused| !self.focusable_ids.contains(&focused))
        {
            self.focused = self
                .focus_order
                .first()
                .copied()
                .or_else(|| self.focusable_ids.contains(&trap).then_some(trap));
        }
    }

    fn rebuild_dispatch_index(&mut self) {
        self.parents.clear();
        self.key_contexts.clear();
        self.activation_targets.clear();
        self.invalid_ids.clear();
        self.form_ids.clear();
        self.form_submitter_ids.clear();
        self.context_menu_ids.clear();
        self.mouse_listener_bindings.clear();
        self.mouse_listener_ranges.clear();
        self.mouse_listener_elements.clear();
        self.key_listener_bindings.clear();
        self.key_listener_ranges.clear();
        self.action_listener_bindings.clear();
        self.action_listener_ranges.clear();
        self.scroll_wheel_ids.clear();
        self.touch_ids.clear();
        self.mouse_pressure_ids.clear();
        self.pinch_ids.clear();
        self.rotation_ids.clear();
        self.smart_magnify_ids.clear();
        let Some(root) = &self.root else {
            return;
        };
        collect_dispatch_metadata(
            root,
            None,
            &mut DispatchMetadata {
                parents: &mut self.parents,
                key_contexts: &mut self.key_contexts,
                activation_targets: &mut self.activation_targets,
                invalid_ids: &mut self.invalid_ids,
                form_ids: &mut self.form_ids,
                form_submitter_ids: &mut self.form_submitter_ids,
                context_menu_ids: &mut self.context_menu_ids,
                mouse_listener_bindings: &mut self.mouse_listener_bindings,
                mouse_listener_ranges: &mut self.mouse_listener_ranges,
                mouse_listener_elements: &mut self.mouse_listener_elements,
                key_listener_bindings: &mut self.key_listener_bindings,
                key_listener_ranges: &mut self.key_listener_ranges,
                action_listener_bindings: &mut self.action_listener_bindings,
                action_listener_ranges: &mut self.action_listener_ranges,
                scroll_wheel_ids: &mut self.scroll_wheel_ids,
                touch_ids: &mut self.touch_ids,
                mouse_pressure_ids: &mut self.mouse_pressure_ids,
                pinch_ids: &mut self.pinch_ids,
                rotation_ids: &mut self.rotation_ids,
                smart_magnify_ids: &mut self.smart_magnify_ids,
            },
        );
        if self
            .validation_announcement
            .as_ref()
            .is_some_and(|announcement| !self.form_ids.contains(&announcement.form))
        {
            self.validation_announcement = None;
        }
    }

    fn rebuild_drop_predicates(&mut self) {
        self.drop_predicates.clear();
        let Some(root) = &self.root else {
            return;
        };
        collect_drop_predicates(root, &mut self.drop_predicates);
    }
}

fn fixed_text_layout_size(
    known: TaffySize<Option<f32>>,
    available: TaffySize<AvailableSpace>,
    layout: &TaffyStyle,
    text: &TextStyle,
) -> Option<TaffySize<f32>> {
    let width = known.width.or_else(|| absolute_length(layout.size.width));
    let height = known.height.or_else(|| absolute_length(layout.size.height));
    if let (Some(width), Some(height)) = (width, height) {
        return Some(TaffySize { width, height });
    }

    // Taffy invokes leaf measurement again during PerformLayout to calculate content metadata,
    // even when flexbox has already assigned the leaf a definite main-axis size. A web-style
    // `flex: 1 1 0; min-width: 0` no-wrap label explicitly opts out of intrinsic width sizing, so
    // shaping it here cannot affect layout. Its fixed line box also makes the height definite.
    let fills_available_width = text.wrap == TextWrap::None
        && absolute_length(layout.flex_basis) == Some(0.0)
        && absolute_length(layout.min_size.width) == Some(0.0);
    if !fills_available_width {
        return None;
    }

    let width = width.unwrap_or_else(|| match available.width {
        AvailableSpace::Definite(width) => width.max(0.0),
        // This element has explicitly disabled intrinsic width participation. Taffy may still
        // request content metadata during PerformLayout; reporting zero here cannot influence
        // the final flex width, which was established from the zero basis and flex growth.
        AvailableSpace::MinContent | AvailableSpace::MaxContent => 0.0,
    });
    height.map(|height| TaffySize { width, height })
}

fn absolute_length(dimension: Dimension) -> Option<f32> {
    (dimension.tag() == CompactLength::LENGTH_TAG).then(|| dimension.value().max(0.0))
}

fn compute_detached_layout(
    taffy: &mut TaffyTree<MeasureContext>,
    root: NodeId,
    viewport: Size,
    scale_factor: f32,
    renderer: &mut impl TextLayoutEngine,
) -> Result<(), UiError> {
    taffy.compute_layout_with_measure(
        root,
        TaffySize {
            width: AvailableSpace::Definite(viewport.width),
            height: AvailableSpace::Definite(viewport.height),
        },
        |known, available, _node, context, style| {
            let Some(context) = context else {
                return TaffySize::ZERO;
            };
            if let (Some(width), Some(height)) = (known.width, known.height) {
                return TaffySize { width, height };
            }
            match context {
                MeasureContext::Text {
                    id,
                    content,
                    style: text_style,
                    highlights,
                } => {
                    if let Some(size) = fixed_text_layout_size(known, available, style, text_style)
                    {
                        return size;
                    }
                    let max_width = known.width.or_else(|| match available.width {
                        AvailableSpace::Definite(width) => Some(width.max(0.0)),
                        AvailableSpace::MinContent => Some(0.0),
                        AvailableSpace::MaxContent => None,
                    });
                    let measured = if let Some(highlights) = highlights {
                        renderer.measure_styled_text(
                            *id,
                            content,
                            text_style,
                            highlights,
                            max_width,
                            scale_factor,
                        )
                    } else {
                        renderer.measure_text(*id, content, text_style, max_width, scale_factor)
                    };
                    TaffySize {
                        width: known.width.unwrap_or(measured.width),
                        height: known.height.unwrap_or(measured.height),
                    }
                }
                MeasureContext::Image { intrinsic } => {
                    measure_image(known.width, known.height, *intrinsic)
                }
            }
        },
    )?;
    Ok(())
}

/// Lay out every callback subtree as an independent root within its query's assigned box.
///
/// Query child nodes intentionally exist in the same Taffy arena but are not connected to the
/// query leaf. Keeping them in one arena preserves QuickGUI's compact node IDs and lets the normal
/// paint, hit-test, accessibility, and focus walks keep traversing the ordinary `Element` tree.
fn compute_container_query_child_layouts(
    element: &Element,
    taffy: &mut TaffyTree<MeasureContext>,
    scale_factor: f32,
    renderer: &mut impl TextLayoutEngine,
) -> Result<(), UiError> {
    if element.is_display_none() {
        return Ok(());
    }
    if matches!(&element.kind, ElementKind::ContainerQuery(_)) {
        debug_assert!(element.children.len() <= 1);
        let Some(child) = element.children.first() else {
            return Ok(());
        };
        let node = element
            .taffy_node
            .expect("container query nodes are assigned before isolated layout");
        let layout = taffy.layout(node)?;
        let available = Size::new(
            finite_layout_length(layout.size.width),
            finite_layout_length(layout.size.height),
        );
        let child_node = child
            .taffy_node
            .expect("container query children are assigned before isolated layout");
        compute_detached_layout(taffy, child_node, available, scale_factor, renderer)?;
        compute_container_query_child_layouts(child, taffy, scale_factor, renderer)?;
        return Ok(());
    }

    for child in &element.children {
        compute_container_query_child_layouts(child, taffy, scale_factor, renderer)?;
    }
    Ok(())
}

#[cfg(any(test, feature = "test-support"))]
fn mark_layout_nodes_dirty(
    element: &Element,
    taffy: &mut TaffyTree<MeasureContext>,
) -> Result<(), UiError> {
    for child in &element.children {
        mark_layout_nodes_dirty(child, taffy)?;
    }
    let node = element
        .taffy_node
        .expect("mounted elements have layout nodes before visual measurement invalidation");
    taffy.mark_dirty(node)?;
    Ok(())
}

fn finite_layout_length(value: f32) -> f32 {
    if value.is_finite() {
        value.max(0.0)
    } else {
        0.0
    }
}

fn validate_container_query_limits(root: &Element) -> Result<(), UiError> {
    fn visit(
        element: &Element,
        query_depth: usize,
        query_count: &mut usize,
    ) -> Result<(), UiError> {
        let query_depth = if matches!(&element.kind, ElementKind::ContainerQuery(_)) {
            *query_count += 1;
            if *query_count > MAX_CONTAINER_QUERIES_PER_WINDOW {
                return Err(UiError::TooManyContainerQueries);
            }
            let depth = query_depth + 1;
            if depth > MAX_CONTAINER_QUERY_DEPTH {
                return Err(UiError::ContainerQueryDepthExceeded);
            }
            depth
        } else {
            query_depth
        };

        for child in &element.children {
            visit(child, query_depth, query_count)?;
        }
        Ok(())
    }

    visit(root, 0, &mut 0)
}

fn container_queries_need_resolution(
    element: &Element,
    taffy: &TaffyTree<MeasureContext>,
) -> Result<bool, UiError> {
    if element.is_display_none() {
        return Ok(false);
    }
    if let ElementKind::ContainerQuery(query) = &element.kind {
        let node = element
            .taffy_node
            .expect("container query nodes are assigned before size reconciliation");
        let layout = taffy.layout(node)?;
        let assigned_size = Size::new(
            finite_layout_length(layout.size.width),
            finite_layout_length(layout.size.height),
        );
        if query.resolved_size != Some(assigned_size) || element.children.len() != 1 {
            return Ok(true);
        }
    }
    for child in &element.children {
        if container_queries_need_resolution(child, taffy)? {
            return Ok(true);
        }
    }
    Ok(false)
}

struct ContainerQueryResolveContext<'a, F>
where
    F: FnMut(&mut Element),
{
    taffy: &'a TaffyTree<MeasureContext>,
    prepare: &'a mut F,
    animations: &'a mut HashMap<ElementId, DeclarativeAnimationPlayback>,
    motion_ids: &'a mut HashSet<ElementId>,
    time_animation_ids: &'a mut HashSet<ElementId>,
    springs: &'a mut HashMap<ElementId, DeclarativeSpringPlayback>,
    spring_ids: &'a mut HashSet<ElementId>,
    request_frame: &'a mut bool,
    deadline: &'a mut Option<Instant>,
    now: Instant,
    animation_epoch: Instant,
    enabled: bool,
    reduce_motion: bool,
    sanitize_detached: bool,
}

#[derive(Clone, Copy, Debug, Default)]
struct ContainerQueryResolution {
    changed: bool,
    replaced_existing_subtree: bool,
}

impl ContainerQueryResolution {
    fn merge(&mut self, other: Self) {
        self.changed |= other.changed;
        self.replaced_existing_subtree |= other.replaced_existing_subtree;
    }
}

impl<F> ContainerQueryResolveContext<'_, F>
where
    F: FnMut(&mut Element),
{
    fn resolve(&mut self, element: &mut Element) -> Result<ContainerQueryResolution, UiError> {
        if element.is_display_none() {
            return Ok(ContainerQueryResolution::default());
        }
        if matches!(&element.kind, ElementKind::ContainerQuery(_)) {
            let node = element
                .taffy_node
                .expect("container query nodes are assigned before callback resolution");
            let layout = self.taffy.layout(node)?;
            let assigned_size = Size::new(
                finite_layout_length(layout.size.width),
                finite_layout_length(layout.size.height),
            );
            let needs_resolution = match &element.kind {
                ElementKind::ContainerQuery(query) => {
                    query.resolved_size != Some(assigned_size) || element.children.len() != 1
                }
                _ => unreachable!(),
            };

            if needs_resolution {
                let replaced_existing_subtree = !element.children.is_empty();
                let mut removed_motion_ids = Vec::new();
                if let ElementKind::ContainerQuery(query) = &mut element.kind {
                    removed_motion_ids.append(&mut query.resolved_motion_ids);
                }
                for child in &mut element.children {
                    drain_container_query_motion_ids(child, &mut removed_motion_ids);
                }
                for id in removed_motion_ids {
                    self.motion_ids.remove(&id);
                    self.time_animation_ids.remove(&id);
                    self.spring_ids.remove(&id);
                }

                let mut child = match &element.kind {
                    ElementKind::ContainerQuery(query) => query.render(assigned_size),
                    _ => unreachable!(),
                };
                (self.prepare)(&mut child);
                if self.sanitize_detached {
                    sanitize_detached_element(&mut child, true);
                }
                let mut resolved_motion_ids = Vec::new();
                resolve_declarative_animations(
                    &mut child,
                    self.animations,
                    self.motion_ids,
                    self.time_animation_ids,
                    self.springs,
                    self.spring_ids,
                    self.request_frame,
                    self.deadline,
                    self.now,
                    self.animation_epoch,
                    self.enabled,
                    self.reduce_motion,
                    &mut resolved_motion_ids,
                )?;
                if self.sanitize_detached {
                    sanitize_detached_element(&mut child, false);
                }
                element.children.clear();
                element.children.push(child);
                if let ElementKind::ContainerQuery(query) = &mut element.kind {
                    query.resolved_size = Some(assigned_size);
                    query.resolved_motion_ids = resolved_motion_ids;
                    query.layout_pending = true;
                }
                return Ok(ContainerQueryResolution {
                    changed: true,
                    replaced_existing_subtree,
                });
            }
        }

        let mut resolution = ContainerQueryResolution::default();
        for child in &mut element.children {
            resolution.merge(self.resolve(child)?);
        }
        Ok(resolution)
    }
}

fn drain_container_query_motion_ids(element: &mut Element, ids: &mut Vec<ElementId>) {
    if let ElementKind::ContainerQuery(query) = &mut element.kind {
        ids.append(&mut query.resolved_motion_ids);
    }
    for child in &mut element.children {
        drain_container_query_motion_ids(child, ids);
    }
}

fn sanitize_detached_element(element: &mut Element, preserve_motion: bool) {
    element.explicit_id = None;
    element.runtime_id = ElementId::new(0);
    element.taffy_node = None;
    element.clickable = false;
    element.pointer_listener = false;
    element.scroll_wheel_listener = false;
    element.touch_listener = false;
    element.context_menu_listener = false;
    element.mouse_pressure_listener = false;
    element.pinch_listener = false;
    element.rotation_listener = false;
    element.smart_magnify_listener = false;
    element.drag_source = false;
    element.drop_target = false;
    element.drop_predicates.clear();
    element.cursor_style = None;
    element.cursor_style_explicit = false;
    element.hover.cursor_style = None;
    element.active.cursor_style = None;
    element.focus.cursor_style = None;
    element.disabled_style.cursor_style = None;
    element.invalid_style.cursor_style = None;
    element.dragging.cursor_style = None;
    element.drag_over.cursor_style = None;
    element.user_select = UserSelect::None;
    element.resolved_user_select = false;
    element.focusable = false;
    element.focus_trap = false;
    element.key_context = None;
    element.auto_focus = false;
    element.activation_target = None;
    element.plane = None;
    element.anchor = None;
    element.tooltip = None;
    element.app_region = None;
    element.virtual_scroll = None;
    element.scroll_to_end_revision = None;
    element.list_item_measurement = None;
    if !preserve_motion {
        element.animation = None;
        element.spring = None;
    }
    element.transition = None;
    element.blocks_pointer = false;
    element.dismiss_policy = DismissPolicy::default();
    element.restore_focus = None;
    for child in &mut element.children {
        sanitize_detached_element(child, preserve_motion);
    }
}

impl Default for UiTree {
    fn default() -> Self {
        Self::new()
    }
}

pub(crate) struct PointerResult {
    pub repaint: bool,
    pub clicked: Option<ElementId>,
    pub dismissed: Option<DismissRequest>,
    pub pointer_listener: Option<ElementId>,
    pub drag_source: Option<ElementId>,
}

fn text_input_max_scroll(
    content_size: Size,
    viewport: Size,
    line_height: f32,
    multiline: bool,
) -> Vector {
    Vector::new(
        (content_size.width - viewport.width).max(0.0),
        if multiline {
            (content_size.height.max(line_height) - viewport.height).max(0.0)
        } else {
            0.0
        },
    )
}

fn scroll_to_reveal_caret(
    viewport: Size,
    caret: Point,
    line_height: f32,
    mut scroll: Vector,
    max_scroll: Vector,
) -> Vector {
    let horizontal_margin = 16.0_f32.min(viewport.width * 0.5);
    let left_edge = horizontal_margin;
    let right_edge = (viewport.width - horizontal_margin).max(left_edge);
    if caret.x - scroll.x > right_edge {
        scroll.x = caret.x - right_edge;
    } else if caret.x - scroll.x < left_edge {
        scroll.x = (caret.x - left_edge).max(0.0);
    }

    let bottom_edge = (viewport.height - 2.0).max(0.0);
    if caret.y + line_height - scroll.y > bottom_edge {
        scroll.y = caret.y + line_height - bottom_edge;
    } else if caret.y < scroll.y {
        scroll.y = caret.y;
    }
    Vector::new(
        scroll.x.clamp(0.0, max_scroll.x),
        scroll.y.clamp(0.0, max_scroll.y),
    )
}

fn text_input_index_at(
    region: &TextInputRegion,
    point: Point,
    scale_factor: f32,
    renderer: &mut impl TextLayoutEngine,
) -> usize {
    if region.content.is_empty() {
        return 0;
    }
    let display_index = renderer.text_index_for_point_with_highlights(
        TextId::new(region.id.value()),
        &region.content,
        &region.style,
        region.highlights.as_ref(),
        region.bounds.width,
        scale_factor,
        Point::new(
            point.x - region.bounds.x + region.scroll.x,
            point.y - region.bounds.y + region.scroll.y,
        ),
    );
    region.password.as_ref().map_or(display_index, |password| {
        password.source_index(display_index)
    })
}

fn squared_distance_to_rect(point: Point, rect: Rect) -> f32 {
    let dx = if point.x < rect.x {
        rect.x - point.x
    } else if point.x > rect.right() {
        point.x - rect.right()
    } else {
        0.0
    };
    let dy = if point.y < rect.y {
        rect.y - point.y
    } else if point.y > rect.bottom() {
        point.y - rect.bottom()
    } else {
        0.0
    };
    dx * dx + dy * dy
}

fn nearest_selectable_text_region(
    regions: &[SelectableTextRegion],
    point: Point,
) -> Option<&SelectableTextRegion> {
    regions
        .iter()
        .filter_map(|region| {
            region
                .bounds
                .intersection(region.clip)
                .map(|visible| (region, visible))
        })
        .min_by(|(left, left_visible), (right, right_visible)| {
            let left_distance = squared_distance_to_rect(point, *left_visible);
            let right_distance = squared_distance_to_rect(point, *right_visible);
            left_distance
                .total_cmp(&right_distance)
                .then_with(|| right.order.cmp(&left.order))
        })
        .map(|(region, _)| region)
}

fn selection_separator(previous: Option<Rect>, next: Option<Rect>) -> &'static str {
    let (Some(previous), Some(next)) = (previous, next) else {
        return "\n";
    };
    let overlap = previous.bottom().min(next.bottom()) - previous.y.max(next.y);
    if overlap > previous.height.min(next.height) * 0.4 {
        " "
    } else {
        "\n"
    }
}

fn push_bounded_text(output: &mut String, value: &str, limit: usize) {
    let remaining = limit.saturating_sub(output.len());
    if remaining == 0 {
        return;
    }
    let mut end = value.len().min(remaining);
    while end > 0 && !value.is_char_boundary(end) {
        end -= 1;
    }
    output.push_str(&value[..end]);
}

fn measure_image(width: Option<f32>, height: Option<f32>, intrinsic: Size) -> TaffySize<f32> {
    match (width, height) {
        (Some(width), Some(height)) => TaffySize { width, height },
        (Some(width), None) => TaffySize {
            width,
            height: width * intrinsic.height / intrinsic.width,
        },
        (None, Some(height)) => TaffySize {
            width: height * intrinsic.width / intrinsic.height,
            height,
        },
        (None, None) => TaffySize {
            width: intrinsic.width,
            height: intrinsic.height,
        },
    }
}

fn fit_path(bounds: Rect, path: &Path, fit: ObjectFit) -> Option<([f32; 2], Vector)> {
    let intrinsic = path.size();
    if bounds.is_empty() || intrinsic.is_empty() {
        return None;
    }
    let fitted = fit_image(bounds, intrinsic, fit);
    let source_width = intrinsic.width * fitted.source_uv.width;
    let source_height = intrinsic.height * fitted.source_uv.height;
    if source_width <= 0.0 || source_height <= 0.0 {
        return None;
    }
    let scale = [
        fitted.destination.width / source_width,
        fitted.destination.height / source_height,
    ];
    let source_bounds = path.bounds();
    let translation = Vector::new(
        fitted.destination.x - (source_bounds.x + intrinsic.width * fitted.source_uv.x) * scale[0],
        fitted.destination.y - (source_bounds.y + intrinsic.height * fitted.source_uv.y) * scale[1],
    );
    Some((scale, translation))
}

fn build_pending_container_query_subtrees(
    element: &mut Element,
    taffy: &mut TaffyTree<MeasureContext>,
    seen_ids: &mut HashSet<ElementId>,
) -> Result<bool, UiError> {
    if element.is_display_none() {
        return Ok(false);
    }
    let layout_pending = matches!(
        &element.kind,
        ElementKind::ContainerQuery(query) if query.layout_pending
    );
    if layout_pending {
        debug_assert_eq!(element.children.len(), 1);
        let parent_id = element.runtime_id;
        let inherited_typography = element.resolved_typography.clone();
        let inherited_user_select = element.resolved_user_select;
        let child = element
            .children
            .first_mut()
            .expect("a pending query layout has callback contents");
        match collect_explicit_ids(child, seen_ids) {
            Ok(()) => {}
            Err(UiError::DuplicateId(_)) => return Ok(true),
            Err(error) => return Err(error),
        }
        build_layout_node(
            taffy,
            seen_ids,
            child,
            parent_id,
            0,
            &inherited_typography,
            inherited_user_select,
        )?;
        if let ElementKind::ContainerQuery(query) = &mut element.kind {
            query.layout_pending = false;
        }
        return Ok(false);
    }

    for child in &mut element.children {
        if build_pending_container_query_subtrees(child, taffy, seen_ids)? {
            return Ok(true);
        }
    }
    Ok(false)
}

fn build_layout_node(
    taffy: &mut TaffyTree<MeasureContext>,
    seen_ids: &mut HashSet<ElementId>,
    element: &mut Element,
    parent_id: ElementId,
    child_index: usize,
    inherited_typography: &TextStyle,
    inherited_user_select: bool,
) -> Result<NodeId, UiError> {
    let id = if let Some(id) = element.explicit_id {
        id
    } else {
        let mut generated = ElementId::new(mix_id(parent_id.value(), child_index as u64));
        let mut collision_index = 0_u64;
        while !seen_ids.insert(generated) {
            collision_index = collision_index.wrapping_add(1);
            generated = ElementId::new(mix_id(generated.value(), collision_index));
        }
        generated
    };
    element.runtime_id = id;
    element.resolved_typography = element.typography.resolve(inherited_typography);
    // Editable controls must retain a one-to-one mapping between their controlled value and the
    // shaped buffer. Their own viewport already provides web-style clipping and caret scrolling;
    // inherited display-only truncation would otherwise hide editable bytes from IME and hit
    // testing.
    if matches!(&element.kind, ElementKind::TextInput(_)) {
        element.resolved_typography.text_overflow = None;
        element.resolved_typography.line_clamp = None;
    }
    let automatic_user_select = inherited_user_select
        && !element.clickable
        && !element.pointer_listener
        && !element.drag_source
        && !matches!(
            element.accessibility.role,
            AccessibilityRole::Button
                | AccessibilityRole::CheckBox
                | AccessibilityRole::MenuItem
                | AccessibilityRole::MenuItemCheckBox
                | AccessibilityRole::MenuItemRadio
        )
        && !matches!(&element.kind, ElementKind::TextInput(_));
    element.resolved_user_select = match element.user_select {
        UserSelect::Auto => automatic_user_select,
        UserSelect::Text => true,
        UserSelect::None => false,
    };

    let mut child_nodes = Vec::with_capacity(element.children.len());
    for (index, child) in element.children.iter_mut().enumerate() {
        child_nodes.push(build_layout_node(
            taffy,
            seen_ids,
            child,
            id,
            index,
            &element.resolved_typography,
            element.resolved_user_select,
        )?);
    }

    let node = match &element.kind {
        ElementKind::Container => taffy.new_with_children(element.layout.clone(), &child_nodes)?,
        // Query contents are deliberately disconnected from this leaf. Their layout is computed
        // later as a separate root using this node's assigned size, so callback contents cannot
        // feed intrinsic size back into the query box.
        ElementKind::ContainerQuery(_) => taffy.new_leaf(element.layout.clone())?,
        ElementKind::Text(content) => taffy.new_leaf_with_context(
            element.layout.clone(),
            MeasureContext::Text {
                id: TextId::new(id.value()),
                content: content.clone(),
                style: element.resolved_typography.clone(),
                highlights: None,
            },
        )?,
        ElementKind::StyledText(styled) => taffy.new_leaf_with_context(
            element.layout.clone(),
            MeasureContext::Text {
                id: TextId::new(id.value()),
                content: styled.content().clone(),
                style: element.resolved_typography.clone(),
                highlights: Some(styled.shared_highlights().clone()),
            },
        )?,
        ElementKind::Image(image) => match &image.resolved {
            ImageResolution::Ready(source) => taffy.new_leaf_with_context(
                element.layout.clone(),
                MeasureContext::Image {
                    intrinsic: source.size(),
                },
            )?,
            ImageResolution::Animated(animation) => taffy.new_leaf_with_context(
                element.layout.clone(),
                MeasureContext::Image {
                    intrinsic: animation.size(),
                },
            )?,
            ImageResolution::Loading | ImageResolution::Failed => {
                if child_nodes.is_empty() {
                    taffy.new_leaf(element.layout.clone())?
                } else {
                    taffy.new_with_children(element.layout.clone(), &child_nodes)?
                }
            }
        },
        ElementKind::Svg(svg) => taffy.new_leaf_with_context(
            element.layout.clone(),
            MeasureContext::Image {
                intrinsic: svg.svg.size(),
            },
        )?,
        ElementKind::Path(path) => {
            let intrinsic = path.path.size();
            if intrinsic.is_empty() {
                taffy.new_leaf(element.layout.clone())?
            } else {
                taffy.new_leaf_with_context(
                    element.layout.clone(),
                    MeasureContext::Image { intrinsic },
                )?
            }
        }
        ElementKind::Canvas(_) | ElementKind::CustomShader(_) => {
            taffy.new_leaf(element.layout.clone())?
        }
        ElementKind::TextInput(input) => {
            let content = if input.value.is_empty() {
                input.placeholder.clone()
            } else if input.password {
                PasswordDisplay::new(&input.value).content
            } else {
                input.value.clone()
            };
            let highlights =
                (!input.password && !input.value.is_empty() && !input.highlights.is_empty())
                    .then(|| input.highlights.clone());
            taffy.new_leaf_with_context(
                element.layout.clone(),
                MeasureContext::Text {
                    id: TextId::new(id.value()),
                    content,
                    style: element.resolved_typography.clone(),
                    highlights,
                },
            )?
        }
        #[cfg(target_os = "macos")]
        ElementKind::NativeView(_) => taffy.new_leaf(element.layout.clone())?,
    };
    if let ElementKind::ContainerQuery(query) = &mut element.kind {
        query.layout_pending = false;
    }
    element.taffy_node = Some(node);
    Ok(node)
}

fn apply_scroll_end_revision(
    id: ElementId,
    revision: Option<u64>,
    max_y: f32,
    offset: &mut Vector,
    states: &mut HashMap<ElementId, ScrollEndState>,
) {
    let Some(revision) = revision else {
        states.remove(&id);
        return;
    };
    let should_follow = states.get(&id).is_none_or(|state| {
        let declaration_changed =
            state.revision != revision || (state.previous_max_y - max_y).abs() > f32::EPSILON;
        declaration_changed && offset.y >= state.previous_max_y - 1.0
    });
    if should_follow {
        offset.y = max_y;
    }
    states.insert(
        id,
        ScrollEndState {
            revision,
            previous_max_y: max_y,
        },
    );
}

fn collect_layout_bounds(
    element: &Element,
    taffy: &TaffyTree<MeasureContext>,
    scroll_offsets: &mut HashMap<ElementId, Vector>,
    scroll_end_states: &mut HashMap<ElementId, ScrollEndState>,
    bounds: &mut HashMap<ElementId, Rect>,
    parent_origin: Point,
) -> Result<(), UiError> {
    if element.is_display_none() {
        return Ok(());
    }
    let node = element
        .taffy_node
        .expect("layout nodes are assigned before bounds collection");
    let layout = taffy.layout(node)?;
    let element_bounds = Rect::new(
        parent_origin.x + layout.location.x,
        parent_origin.y + layout.location.y,
        layout.size.width,
        layout.size.height,
    );
    bounds.insert(element.runtime_id, element_bounds);

    let is_scrollable = !matches!(&element.kind, ElementKind::TextInput(_))
        && (element.layout.overflow.x == Overflow::Scroll
            || element.layout.overflow.y == Overflow::Scroll);
    let mut scroll = Vector::ZERO;
    if is_scrollable {
        let max_offset = Vector::new(
            (layout.content_size.width - layout.size.width).max(0.0),
            (layout.content_size.height - layout.size.height).max(0.0),
        );
        let offset = scroll_offsets.entry(element.runtime_id).or_default();
        apply_scroll_end_revision(
            element.runtime_id,
            element.scroll_to_end_revision,
            max_offset.y,
            offset,
            scroll_end_states,
        );
        offset.x = offset.x.clamp(0.0, max_offset.x);
        offset.y = offset.y.clamp(0.0, max_offset.y);
        scroll = *offset;
    } else if let Some(virtual_scroll) = &element.virtual_scroll {
        let max_offset = virtual_scroll
            .handle
            .max_offset(virtual_scroll.max_offset_y)
            .max(0.0);
        let offset = scroll_offsets.entry(element.runtime_id).or_default();
        offset.x = 0.0;
        offset.y = virtual_scroll.handle.offset().clamp(0.0, max_offset);
        scroll.y = offset.y - virtual_scroll.mount.layout_offset_y;
        scroll_end_states.remove(&element.runtime_id);
    } else {
        scroll_end_states.remove(&element.runtime_id);
    }

    let child_origin = Point::new(element_bounds.x - scroll.x, element_bounds.y - scroll.y);
    for child in &element.children {
        collect_layout_bounds(
            child,
            taffy,
            scroll_offsets,
            scroll_end_states,
            bounds,
            child_origin,
        )?;
    }
    Ok(())
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum AnchorSide {
    Top,
    Bottom,
    Left,
    Right,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum AnchorAlign {
    Start,
    Center,
    End,
}

fn place_anchored(
    anchor: Rect,
    size: Size,
    viewport: Rect,
    placement: AnchorPlacement,
    gap: f32,
    margin: f32,
) -> Rect {
    let (preferred_side, preferred_align) = anchor_placement_parts(placement);
    let inner = viewport.inset(Insets::all(margin.max(0.0)));
    let gap = gap.max(0.0);
    let preferred_space = available_anchor_space(anchor, inner, preferred_side, gap);
    let opposite_side = opposite_anchor_side(preferred_side);
    let opposite_space = available_anchor_space(anchor, inner, opposite_side, gap);
    let primary_size = match preferred_side {
        AnchorSide::Top | AnchorSide::Bottom => size.height,
        AnchorSide::Left | AnchorSide::Right => size.width,
    };
    let side = if primary_size > preferred_space && opposite_space > preferred_space {
        opposite_side
    } else {
        preferred_side
    };

    let alignments = match preferred_align {
        AnchorAlign::Start => [AnchorAlign::Start, AnchorAlign::End, AnchorAlign::Center],
        AnchorAlign::Center => [AnchorAlign::Center, AnchorAlign::Start, AnchorAlign::End],
        AnchorAlign::End => [AnchorAlign::End, AnchorAlign::Start, AnchorAlign::Center],
    };
    let align = alignments
        .into_iter()
        .min_by(|left, right| {
            let left = anchored_origin(anchor, size, side, *left, gap);
            let right = anchored_origin(anchor, size, side, *right, gap);
            cross_axis_overflow(left, size, inner, side)
                .total_cmp(&cross_axis_overflow(right, size, inner, side))
        })
        .unwrap_or(preferred_align);
    let mut origin = anchored_origin(anchor, size, side, align, gap);
    origin.x = clamp_surface_axis(origin.x, size.width, inner.x, inner.right());
    origin.y = clamp_surface_axis(origin.y, size.height, inner.y, inner.bottom());
    Rect::new(origin.x, origin.y, size.width, size.height)
}

fn anchor_placement_parts(placement: AnchorPlacement) -> (AnchorSide, AnchorAlign) {
    match placement {
        AnchorPlacement::TopStart => (AnchorSide::Top, AnchorAlign::Start),
        AnchorPlacement::Top => (AnchorSide::Top, AnchorAlign::Center),
        AnchorPlacement::TopEnd => (AnchorSide::Top, AnchorAlign::End),
        AnchorPlacement::BottomStart => (AnchorSide::Bottom, AnchorAlign::Start),
        AnchorPlacement::Bottom => (AnchorSide::Bottom, AnchorAlign::Center),
        AnchorPlacement::BottomEnd => (AnchorSide::Bottom, AnchorAlign::End),
        AnchorPlacement::LeftStart => (AnchorSide::Left, AnchorAlign::Start),
        AnchorPlacement::Left => (AnchorSide::Left, AnchorAlign::Center),
        AnchorPlacement::LeftEnd => (AnchorSide::Left, AnchorAlign::End),
        AnchorPlacement::RightStart => (AnchorSide::Right, AnchorAlign::Start),
        AnchorPlacement::Right => (AnchorSide::Right, AnchorAlign::Center),
        AnchorPlacement::RightEnd => (AnchorSide::Right, AnchorAlign::End),
    }
}

fn opposite_anchor_side(side: AnchorSide) -> AnchorSide {
    match side {
        AnchorSide::Top => AnchorSide::Bottom,
        AnchorSide::Bottom => AnchorSide::Top,
        AnchorSide::Left => AnchorSide::Right,
        AnchorSide::Right => AnchorSide::Left,
    }
}

fn available_anchor_space(anchor: Rect, viewport: Rect, side: AnchorSide, gap: f32) -> f32 {
    match side {
        AnchorSide::Top => anchor.y - viewport.y - gap,
        AnchorSide::Bottom => viewport.bottom() - anchor.bottom() - gap,
        AnchorSide::Left => anchor.x - viewport.x - gap,
        AnchorSide::Right => viewport.right() - anchor.right() - gap,
    }
    .max(0.0)
}

fn anchored_origin(
    anchor: Rect,
    size: Size,
    side: AnchorSide,
    align: AnchorAlign,
    gap: f32,
) -> Point {
    let cross_x = match align {
        AnchorAlign::Start => anchor.x,
        AnchorAlign::Center => anchor.x + (anchor.width - size.width) * 0.5,
        AnchorAlign::End => anchor.right() - size.width,
    };
    let cross_y = match align {
        AnchorAlign::Start => anchor.y,
        AnchorAlign::Center => anchor.y + (anchor.height - size.height) * 0.5,
        AnchorAlign::End => anchor.bottom() - size.height,
    };
    match side {
        AnchorSide::Top => Point::new(cross_x, anchor.y - gap - size.height),
        AnchorSide::Bottom => Point::new(cross_x, anchor.bottom() + gap),
        AnchorSide::Left => Point::new(anchor.x - gap - size.width, cross_y),
        AnchorSide::Right => Point::new(anchor.right() + gap, cross_y),
    }
}

fn cross_axis_overflow(origin: Point, size: Size, viewport: Rect, side: AnchorSide) -> f32 {
    match side {
        AnchorSide::Top | AnchorSide::Bottom => {
            (viewport.x - origin.x).max(0.0) + (origin.x + size.width - viewport.right()).max(0.0)
        }
        AnchorSide::Left | AnchorSide::Right => {
            (viewport.y - origin.y).max(0.0) + (origin.y + size.height - viewport.bottom()).max(0.0)
        }
    }
}

fn clamp_surface_axis(origin: f32, size: f32, minimum: f32, maximum: f32) -> f32 {
    if size >= maximum - minimum {
        minimum
    } else {
        origin.clamp(minimum, maximum - size)
    }
}

#[allow(clippy::too_many_arguments)]
fn push_element_shadows(
    scene: &mut Scene,
    layer: PaintLayerKey,
    bounds: Rect,
    radius: f32,
    clip: Rect,
    shadows: &[BoxShadow],
    inset: bool,
) {
    // CSS paints the first declared shadow on top, so display-list insertion is reversed.
    for shadow in shadows
        .iter()
        .rev()
        .filter(|shadow| shadow.is_inset() == inset)
    {
        scene.push_shadow_in(
            layer,
            Shadow::new(bounds, *shadow).radius(radius).clip(clip),
        );
    }
}

fn own_text_clip(element: &Element, bounds: Rect, parent_clip: Rect) -> Rect {
    let clips_own_text = element.resolved_typography.line_clamp.is_some()
        || element.layout.overflow.x != Overflow::Visible
        || element.layout.overflow.y != Overflow::Visible;
    if clips_own_text {
        parent_clip.intersection(bounds).unwrap_or(Rect::ZERO)
    } else {
        parent_clip
    }
}

fn element_has_outset_shadow(element: &Element) -> bool {
    [
        element.visual.shadows.as_deref(),
        element.hover.shadows.as_deref(),
        element.active.shadows.as_deref(),
        element.focus.shadows.as_deref(),
        element.invalid_style.shadows.as_deref(),
        element.disabled_style.shadows.as_deref(),
        element.dragging.shadows.as_deref(),
        element.drag_over.shadows.as_deref(),
    ]
    .into_iter()
    .flatten()
    .flatten()
    .any(|shadow| !shadow.is_inset())
}

#[allow(clippy::too_many_arguments)]
fn paint_selectable_text(
    element: &Element,
    document_index: Option<usize>,
    content: &Arc<str>,
    style: &TextStyle,
    highlights: Option<&Arc<[TextHighlight]>>,
    bounds: Rect,
    parent_clip: Rect,
    layer: PaintLayerKey,
    order: PaintOrder,
    scale_factor: f32,
    selection: Option<StaticTextSelection>,
    indices: &HashMap<ElementId, usize>,
    regions: &mut Vec<SelectableTextRegion>,
    scene: &mut Scene,
    renderer: &mut impl TextLayoutEngine,
) {
    let Some(document_index) = document_index else {
        return;
    };
    let Some(clip) = parent_clip.intersection(bounds) else {
        return;
    };
    regions.push(SelectableTextRegion {
        document_index,
        bounds,
        clip,
        style: style.clone(),
        highlights: highlights.cloned(),
        order,
    });
    let Some(range) =
        static_selection_range_for_entry(selection, indices, document_index, content.len())
    else {
        return;
    };
    let visible_y = (clip.y - bounds.y).max(0.0)..(clip.bottom() - bounds.y).min(bounds.height);
    for rect in renderer.text_selection_rects_with_highlights(
        TextId::new(element.runtime_id.value()),
        content,
        style,
        highlights,
        bounds.width,
        scale_factor,
        visible_y,
        range.start,
        range.end,
    ) {
        scene.push_quad_in(
            layer,
            Quad::new(
                Rect::new(
                    bounds.x + rect.x,
                    bounds.y + rect.y,
                    rect.width,
                    rect.height,
                ),
                static_selection_color(),
            )
            .clip(clip),
        );
    }
}

#[cfg(feature = "inspector")]
#[allow(clippy::too_many_arguments)]
fn collect_inspector_nodes(
    element: &Element,
    parent: Option<ElementId>,
    depth: usize,
    parent_layer: PaintLayerKey,
    parent_clip: Rect,
    viewport: Rect,
    element_bounds: &HashMap<ElementId, Rect>,
    hit_lookup: &HashMap<ElementId, HitRegion>,
    focused: Option<ElementId>,
    source_order: &mut usize,
    snapshot: &mut crate::inspector::InspectorSnapshot,
) {
    if element.is_display_none() || element.is_visibility_hidden() {
        return;
    }
    let Some(bounds) = element_bounds.get(&element.runtime_id).copied() else {
        return;
    };
    if snapshot.nodes.len() == crate::inspector::MAX_INSPECTOR_NODES {
        snapshot.nodes_truncated = true;
        return;
    }

    let plane = element.plane.unwrap_or(parent_layer.plane);
    let z_index = if plane == parent_layer.plane {
        parent_layer
            .z_index
            .saturating_add(element.z_index.unwrap_or(0))
    } else {
        element.z_index.unwrap_or(0)
    };
    let layer = PaintLayerKey { plane, z_index };
    let source = *source_order;
    *source_order = source.saturating_add(1);
    let clip = if element.portal {
        viewport
    } else {
        parent_clip
    };
    let accessibility = crate::inspector::inspector_accessibility(element);
    let on_focus_path = snapshot.focus_path.contains(&element.runtime_id);
    snapshot.nodes.push(crate::inspector::InspectorNode {
        id: element.runtime_id,
        parent,
        depth,
        explicit_id: element.explicit_id.is_some(),
        kind: crate::inspector::InspectorElementKind::from_element(&element.kind),
        bounds,
        clip,
        plane,
        z_index,
        source_order: source,
        portal: element.portal,
        focused: focused == Some(element.runtime_id),
        on_focus_path,
        hit_region: hit_lookup
            .get(&element.runtime_id)
            .copied()
            .map(crate::inspector::InspectorHitRegion::from_region),
        accessibility,
    });

    let Some(child_clip) = crate::inspector::child_clip(element, clip, bounds) else {
        return;
    };
    for child in &element.children {
        collect_inspector_nodes(
            child,
            Some(element.runtime_id),
            depth.saturating_add(1),
            layer,
            child_clip,
            viewport,
            element_bounds,
            hit_lookup,
            focused,
            source_order,
            snapshot,
        );
        if snapshot.nodes_truncated {
            return;
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn paint_element(
    element: &Element,
    taffy: &TaffyTree<MeasureContext>,
    natural_bounds: &HashMap<ElementId, Rect>,
    element_bounds: &mut HashMap<ElementId, Rect>,
    scroll_offsets: &mut HashMap<ElementId, Vector>,
    hovered: &HashSet<ElementId>,
    pressed: Option<ElementId>,
    dragging: Option<ElementId>,
    drag_over: Option<ElementId>,
    focused: Option<ElementId>,
    scale_factor: f32,
    scene: &mut Scene,
    renderer: &mut impl TextLayoutEngine,
    text_inputs: &mut HashMap<ElementId, TextInputState>,
    animations: &mut HashMap<ElementId, AnimationPlayback>,
    animations_enabled: bool,
    paint_time: Instant,
    transition_context: &mut StyleTransitionPaintContext<'_>,
    hit_regions: &mut Vec<HitRegion>,
    scroll_regions: &mut Vec<ScrollRegion>,
    scrollbar_states: &mut HashMap<ElementId, ScrollbarState>,
    dismiss_regions: &mut Vec<DismissRegion>,
    #[cfg(target_os = "macos")] native_views: &mut Vec<NativeViewPlacement>,
    text_input_regions: &mut Vec<TextInputRegion>,
    selectable_text_indices: &HashMap<ElementId, usize>,
    selectable_text_regions: &mut Vec<SelectableTextRegion>,
    static_text_selection: Option<StaticTextSelection>,
    parent_origin: Point,
    parent_clip: Rect,
    viewport: Rect,
    parent_layer: PaintLayerKey,
    source_order: &mut usize,
    inherited_state_text_color: Option<Color>,
) -> Result<(), UiError> {
    if element.is_display_none() || element.is_visibility_hidden() {
        return Ok(());
    }
    let node = element
        .taffy_node
        .expect("layout nodes are assigned before paint");
    let layout = taffy.layout(node)?;
    let natural = Rect::new(
        parent_origin.x + layout.location.x,
        parent_origin.y + layout.location.y,
        layout.size.width,
        layout.size.height,
    );
    let bounds = if let Some(anchor) = element.anchor {
        let anchor_bounds = match anchor.target {
            AnchorTarget::Element(target) => {
                natural_bounds
                    .get(&target)
                    .copied()
                    .ok_or(UiError::MissingAnchor {
                        element: element.runtime_id,
                        anchor: target,
                    })?
            }
            AnchorTarget::Point(point) => Rect::new(point.x, point.y, 0.0, 0.0),
        };
        place_anchored(
            anchor_bounds,
            Size::new(layout.size.width, layout.size.height),
            viewport,
            anchor.placement,
            anchor.gap,
            anchor.viewport_margin,
        )
    } else {
        natural
    };
    element_bounds.insert(element.runtime_id, bounds);
    if let Some(virtual_scroll) = &element.virtual_scroll {
        virtual_scroll
            .handle
            .report_viewport(Size::new(bounds.width, bounds.height));
    }
    if let Some(measurement) = &element.list_item_measurement {
        measurement.report_height(bounds.height);
    }

    // A clipped leaf cannot contribute pixels or interaction regions. Avoid emitting offscreen
    // text/image primitives for long documents while retaining its measured and accessibility
    // bounds above. Outset shadows are the one leaf effect allowed to cross its own bounds.
    let effective_parent_clip = if element.portal {
        viewport
    } else {
        parent_clip
    };
    if element.children.is_empty()
        && effective_parent_clip.intersection(bounds).is_none()
        && !element_has_outset_shadow(element)
    {
        return Ok(());
    }

    let plane = element.plane.unwrap_or(parent_layer.plane);
    let z_index = if plane == parent_layer.plane {
        parent_layer
            .z_index
            .saturating_add(element.z_index.unwrap_or(0))
    } else {
        element.z_index.unwrap_or(0)
    };
    let layer = PaintLayerKey { plane, z_index };
    let order = PaintOrder {
        layer,
        source: *source_order,
    };
    *source_order = (*source_order).saturating_add(1);
    let parent_clip = if element.portal {
        viewport
    } else {
        parent_clip
    };

    let empty_state = ElementStateStyle::default();
    let interaction_state = if drag_over == Some(element.runtime_id) {
        &element.drag_over
    } else if dragging == Some(element.runtime_id) {
        &element.dragging
    } else if pressed == Some(element.runtime_id) {
        &element.active
    } else if hovered.contains(&element.runtime_id) {
        &element.hover
    } else {
        &empty_state
    };
    let focus_state = if focused == Some(element.runtime_id) {
        &element.focus
    } else {
        &empty_state
    };
    let disabled_state = if element.accessibility.disabled {
        &element.disabled_style
    } else {
        &empty_state
    };
    let invalid_state = if element.accessibility.invalid {
        &element.invalid_style
    } else {
        &empty_state
    };
    let target_fill = disabled_state
        .background
        .or(interaction_state.background)
        .or(invalid_state.background)
        .or(focus_state.background)
        .or(element.visual.background)
        .unwrap_or(Color::TRANSPARENT);
    let target_border = disabled_state
        .border_color
        .or(interaction_state.border_color)
        .or(invalid_state.border_color)
        .or(focus_state.border_color)
        .or(element.visual.border_color)
        .unwrap_or(Color::TRANSPARENT);
    let target_border_width = disabled_state
        .border_width
        .or(interaction_state.border_width)
        .or(invalid_state.border_width)
        .or(focus_state.border_width)
        .unwrap_or(element.visual.border_width);
    let target_radius = disabled_state
        .radius
        .or(interaction_state.radius)
        .or(invalid_state.radius)
        .or(focus_state.radius)
        .unwrap_or(element.visual.radius);
    let target_shadows = disabled_state
        .shadows
        .as_deref()
        .or(interaction_state.shadows.as_deref())
        .or(invalid_state.shadows.as_deref())
        .or(focus_state.shadows.as_deref())
        .or(element.visual.shadows.as_deref())
        .unwrap_or_default();
    let target_opacity = disabled_state
        .opacity
        .or(interaction_state.opacity)
        .or(invalid_state.opacity)
        .or(focus_state.opacity)
        .unwrap_or(element.visual.opacity)
        .clamp(0.0, 1.0);
    let target_state_text_color = disabled_state
        .text_color
        .or(interaction_state.text_color)
        .or(invalid_state.text_color)
        .or(focus_state.text_color)
        .or(inherited_state_text_color);
    let sampled_transition = element.transition.as_ref().map(|config| {
        let text_fallback = sane_transition_color(element.resolved_typography.color, Color::WHITE);
        transition_context.sample(
            element.runtime_id,
            TransitionPaintStyle {
                background: sane_transition_color(target_fill, Color::TRANSPARENT),
                border_color: sane_transition_color(target_border, Color::TRANSPARENT),
                border_width: target_border_width.max(0.0),
                radius: target_radius.max(0.0),
                opacity: target_opacity,
                text_color: target_state_text_color
                    .map(|color| sane_transition_color(color, text_fallback)),
                text_fallback,
                shadows: TransitionShadowList::from_slice(target_shadows),
            },
            config,
        )
    });
    let fill = sampled_transition
        .as_ref()
        .map_or(target_fill, |style| style.background);
    let border = sampled_transition
        .as_ref()
        .map_or(target_border, |style| style.border_color);
    let border_width = sampled_transition
        .as_ref()
        .map_or(target_border_width, |style| style.border_width);
    let radius = sampled_transition
        .as_ref()
        .map_or(target_radius, |style| style.radius);
    let shadows = sampled_transition
        .as_ref()
        .map_or(target_shadows, |style| style.shadows.as_slice());
    let opacity = sampled_transition
        .as_ref()
        .map_or(target_opacity, |style| style.opacity);
    let state_text_color = sampled_transition
        .as_ref()
        .map_or(target_state_text_color, |style| style.text_color);
    let previous_opacity = scene.multiply_opacity(opacity);
    push_element_shadows(scene, layer, bounds, radius, parent_clip, shadows, false);
    if fill.a > 0.0 || (border.a > 0.0 && border_width > 0.0) {
        scene.push_quad_in(
            layer,
            Quad::new(bounds, fill)
                .radius(radius)
                .border(border_width, border)
                .clip(parent_clip),
        );
    }
    push_element_shadows(scene, layer, bounds, radius, parent_clip, shadows, true);

    let selectable_document_index = selectable_text_indices.get(&element.runtime_id).copied();
    if element.clickable
        || element.pointer_listener
        || (element.mouse_listeners.is_some() && !element.accessibility.disabled)
        || element.scroll_wheel_listener
        || element.touch_listener
        || element.context_menu_listener
        || element.mouse_pressure_listener
        || element.pinch_listener
        || element.rotation_listener
        || element.smart_magnify_listener
        || element.tooltip.is_some()
        || element.drag_source
        || element.drop_target
        || element.cursor_style.is_some()
        || selectable_document_index.is_some()
        || element.focusable
        || element.blocks_pointer
        || element.app_region.is_some()
        || element.has_stateful_paint()
        || element.has_stateful_cursor()
    {
        let cursor_style = effective_cursor_style(element, selectable_document_index.is_some());
        hit_regions.push(HitRegion {
            id: element.runtime_id,
            bounds,
            clip: parent_clip,
            clickable: element.clickable && !element.accessibility.disabled,
            pointer_listener: element.pointer_listener && !element.accessibility.disabled,
            drag_source: element.drag_source && !element.accessibility.disabled,
            drop_target: element.drop_target && !element.accessibility.disabled,
            focusable: element.focusable && !element.accessibility.disabled,
            cursor_style,
            cursor_states: CursorStateStyles {
                hover: element.hover.cursor_style,
                active: element.active.cursor_style,
                focus: element.focus.cursor_style,
                invalid: element
                    .accessibility
                    .invalid
                    .then_some(element.invalid_style.cursor_style)
                    .flatten(),
                dragging: element.dragging.cursor_style,
                drag_over: element.drag_over.cursor_style,
            },
            stateful: element.has_stateful_paint(),
            blocks_pointer: element.blocks_pointer,
            app_region: element.app_region,
            order,
        });
    }
    if !element.dismiss_policy.is_empty() {
        dismiss_regions.push(DismissRegion {
            id: element.runtime_id,
            bounds,
            clip: parent_clip,
            policy: element.dismiss_policy,
            restore_focus: element.restore_focus.map(|handle| handle.id()),
            order,
        });
    }

    let mut text_input_scroll = None;
    match &element.kind {
        ElementKind::Text(content) => {
            let mut style = element.resolved_typography.clone();
            if let Some(color) = state_text_color {
                style.color = color;
            }
            let text_clip = own_text_clip(element, bounds, parent_clip);
            let decorations = if style.has_decorations() && !bounds.is_empty() {
                text_clip.intersection(bounds).map(|clip| {
                    let visible_y =
                        (clip.y - bounds.y).max(0.0)..(clip.bottom() - bounds.y).min(bounds.height);
                    renderer
                        .text_geometry(
                            TextId::new(element.runtime_id.value()),
                            content,
                            &style,
                            None,
                            bounds.width,
                            scale_factor,
                            visible_y,
                        )
                        .decorations
                })
            } else {
                None
            };
            paint_selectable_text(
                element,
                selectable_document_index,
                content,
                &style,
                None,
                bounds,
                text_clip,
                layer,
                order,
                scale_factor,
                static_text_selection,
                selectable_text_indices,
                selectable_text_regions,
                scene,
                renderer,
            );
            scene.push_text_in(
                layer,
                TextRun::new(
                    TextId::new(element.runtime_id.value()),
                    content.clone(),
                    bounds,
                    style,
                )
                .clip(text_clip),
            );
            if let Some(decorations) = decorations {
                for decoration in decorations {
                    push_text_paint_rect(
                        scene,
                        layer,
                        decoration,
                        Point::new(bounds.x, bounds.y),
                        text_clip,
                    );
                }
            }
        }
        ElementKind::StyledText(styled) => {
            let mut style = element.resolved_typography.clone();
            if let Some(color) = state_text_color {
                style.color = color;
            }
            let text_id = TextId::new(element.runtime_id.value());
            let highlights = styled.shared_highlights().clone();
            let text_clip = own_text_clip(element, bounds, parent_clip);
            if !bounds.is_empty()
                && let Some(clip) = text_clip.intersection(bounds)
            {
                let visible_y =
                    (clip.y - bounds.y).max(0.0)..(clip.bottom() - bounds.y).min(bounds.height);
                let geometry = renderer.text_geometry(
                    text_id,
                    styled.content(),
                    &style,
                    Some(&highlights),
                    bounds.width,
                    scale_factor,
                    visible_y,
                );
                for background in geometry.backgrounds {
                    scene.push_quad_in(
                        layer,
                        Quad::new(
                            Rect::new(
                                bounds.x + background.rect.x,
                                bounds.y + background.rect.y,
                                background.rect.width,
                                background.rect.height,
                            ),
                            background.color,
                        )
                        .clip(clip),
                    );
                }
                paint_selectable_text(
                    element,
                    selectable_document_index,
                    styled.content(),
                    &style,
                    Some(&highlights),
                    bounds,
                    text_clip,
                    layer,
                    order,
                    scale_factor,
                    static_text_selection,
                    selectable_text_indices,
                    selectable_text_regions,
                    scene,
                    renderer,
                );
                scene.push_text_in(
                    layer,
                    TextRun::new(text_id, styled.content().clone(), bounds, style)
                        .with_highlights(highlights)
                        .clip(text_clip),
                );
                for decoration in geometry.decorations {
                    push_text_paint_rect(
                        scene,
                        layer,
                        decoration,
                        Point::new(bounds.x, bounds.y),
                        clip,
                    );
                }
            }
        }
        ElementKind::Image(image) => {
            if !bounds.is_empty()
                && let Some(clip) = parent_clip.intersection(bounds)
                && let Some(source) = match &image.resolved {
                    ImageResolution::Ready(source) => Some(source),
                    ImageResolution::Animated(animation) => {
                        let frame_index = animations
                            .get_mut(&element.runtime_id)
                            .map(|playback| {
                                playback.activate(paint_time, animations_enabled);
                                playback.frame_index
                            })
                            .unwrap_or(0);
                        animation.frame(frame_index).map(|frame| frame.image())
                    }
                    ImageResolution::Loading | ImageResolution::Failed => None,
                }
            {
                let fitted = fit_image(bounds, source.size(), image.object_fit);
                scene.push_image_in(
                    layer,
                    ImagePrimitive::new(source.clone(), fitted.destination)
                        .source_uv(fitted.source_uv)
                        .mask(bounds)
                        .radius(element.visual.radius)
                        .grayscale(image.grayscale)
                        .clip(clip),
                );
            }
        }
        ElementKind::Svg(svg) => {
            if !bounds.is_empty()
                && let Some(clip) = parent_clip.intersection(bounds)
            {
                let fitted = fit_image(bounds, svg.svg.size(), svg.object_fit);
                let color = state_text_color.unwrap_or(element.resolved_typography.color);
                scene.push_svg_in(
                    layer,
                    SvgPrimitive::new(svg.svg.clone(), fitted.destination, color)
                        .source_uv(fitted.source_uv)
                        .mask(bounds)
                        .radius(element.visual.radius)
                        .transform(svg.transform)
                        .clip(clip),
                );
            }
        }
        ElementKind::Path(path) => {
            if let Some(clip) = parent_clip.intersection(bounds)
                && let Some((scale, translation)) = fit_path(bounds, &path.path, path.object_fit)
            {
                let background = path.background.unwrap_or_else(|| {
                    state_text_color
                        .unwrap_or(element.resolved_typography.color)
                        .into()
                });
                scene.push_path_in(
                    layer,
                    PathPrimitive::new(&path.path, background)
                        .scale_xy(scale[0], scale[1])
                        .translate(translation.x, translation.y)
                        .clip(clip),
                );
            }
        }
        ElementKind::Canvas(canvas) => {
            if !bounds.is_empty()
                && let Some(clip) = parent_clip.intersection(bounds)
            {
                let local_bounds = Rect::new(0.0, 0.0, bounds.width, bounds.height);
                let mut context = Canvas::new(
                    scene,
                    layer,
                    Point::new(bounds.x, bounds.y),
                    Size::new(bounds.width, bounds.height),
                    clip,
                );
                (canvas.painter.as_ref())(local_bounds, &mut context);
            }
        }
        ElementKind::CustomShader(shader) => {
            if !bounds.is_empty()
                && let Some(clip) = parent_clip.intersection(bounds)
            {
                scene.push_custom_shader_in(
                    layer,
                    CustomShaderPrimitive::new(shader.shader.clone(), bounds)
                        .parameters(shader.parameters)
                        .clip(clip),
                );
            }
        }
        ElementKind::TextInput(input) => {
            let input_state = text_inputs.entry(element.runtime_id).or_insert_with(|| {
                TextInputState::with_styling(
                    &input.value,
                    input.multiline,
                    input.constraints.clone(),
                    input.highlights.clone(),
                )
            });
            let mut style = element.resolved_typography.clone();
            if let Some(color) = state_text_color {
                style.color = color;
            }
            let vertical_inset = if input.multiline {
                10.0_f32.min(bounds.height * 0.5)
            } else {
                ((bounds.height - style.line_height) * 0.5).max(0.0)
            };
            let text_viewport = bounds.inset(Insets {
                top: vertical_inset,
                right: 12.0,
                bottom: vertical_inset,
                left: 12.0,
            });
            if text_viewport.width > 0.0
                && text_viewport.height > 0.0
                && let Some(text_clip) = parent_clip.intersection(text_viewport)
            {
                let source_content = input_state.shared_text();
                let password = input
                    .password
                    .then(|| PasswordDisplay::new(&source_content));
                let content = password
                    .as_ref()
                    .map_or_else(|| source_content.clone(), |display| display.content.clone());
                let display_index = |source_index| {
                    password
                        .as_ref()
                        .map_or(source_index, |display| display.display_index(source_index))
                };
                let highlights = (!input.password && !content.is_empty())
                    .then(|| input_state.shared_highlights())
                    .filter(|highlights| !highlights.is_empty());
                let text_id = TextId::new(element.runtime_id.value());
                let is_focused = focused == Some(element.runtime_id);
                let content_size = if content.is_empty() {
                    Size::new(0.0, style.line_height)
                } else if let Some(highlights) = &highlights {
                    renderer.measure_styled_text(
                        text_id,
                        &content,
                        &style,
                        highlights,
                        Some(text_viewport.width),
                        scale_factor,
                    )
                } else {
                    renderer.measure_text(
                        text_id,
                        &content,
                        &style,
                        Some(text_viewport.width),
                        scale_factor,
                    )
                };
                let caret = if content.is_empty() {
                    Point::ZERO
                } else {
                    renderer.text_caret_position_with_highlights(
                        text_id,
                        &content,
                        &style,
                        highlights.as_ref(),
                        text_viewport.width,
                        scale_factor,
                        display_index(input_state.caret()),
                    )
                };
                let max_scroll = text_input_max_scroll(
                    content_size,
                    Size::new(text_viewport.width, text_viewport.height),
                    style.line_height,
                    input.multiline,
                );
                let mut scroll = scroll_offsets
                    .get(&element.runtime_id)
                    .copied()
                    .unwrap_or_default();
                scroll.x = scroll.x.clamp(0.0, max_scroll.x);
                scroll.y = scroll.y.clamp(0.0, max_scroll.y);
                let scroll_before_caret = scroll;
                if is_focused {
                    scroll = scroll_to_reveal_caret(
                        Size::new(text_viewport.width, text_viewport.height),
                        caret,
                        style.line_height,
                        scroll,
                        max_scroll,
                    );
                }
                scroll_offsets.insert(element.runtime_id, scroll);
                if scroll != scroll_before_caret {
                    let scrollbar_state = scrollbar_states.entry(element.runtime_id).or_default();
                    if !scrollbar_state.hovered && !scrollbar_state.dragging {
                        scrollbar_state.visible_until =
                            paint_time.checked_add(SCROLLBAR_AUTO_HIDE_DELAY);
                    }
                }
                let caret_inset = if input.multiline { 1.0 } else { 2.0 };
                let caret_bounds = Rect::new(
                    text_viewport.x + caret.x - scroll.x,
                    text_viewport.y + caret.y - scroll.y + caret_inset,
                    1.5,
                    (style.line_height - caret_inset * 2.0).max(1.0),
                );

                let mut decorations = Vec::new();
                if !content.is_empty() && (highlights.is_some() || style.has_decorations()) {
                    let geometry = renderer.text_geometry(
                        text_id,
                        &content,
                        &style,
                        highlights.as_ref(),
                        text_viewport.width,
                        scale_factor,
                        scroll.y..scroll.y + text_viewport.height,
                    );
                    for background in geometry.backgrounds {
                        scene.push_quad_in(
                            layer,
                            Quad::new(
                                Rect::new(
                                    text_viewport.x + background.rect.x - scroll.x,
                                    text_viewport.y + background.rect.y - scroll.y,
                                    background.rect.width,
                                    background.rect.height,
                                ),
                                background.color,
                            )
                            .clip(text_clip),
                        );
                    }
                    decorations = geometry.decorations;
                }

                let selection = input_state.selection();
                if is_focused && !selection.is_empty() && !content.is_empty() {
                    for rect in renderer.text_selection_rects_with_highlights(
                        text_id,
                        &content,
                        &style,
                        highlights.as_ref(),
                        text_viewport.width,
                        scale_factor,
                        scroll.y..scroll.y + text_viewport.height,
                        display_index(selection.start),
                        display_index(selection.end),
                    ) {
                        scene.push_quad_in(
                            layer,
                            Quad::new(
                                Rect::new(
                                    text_viewport.x + rect.x - scroll.x,
                                    text_viewport.y + rect.y - scroll.y,
                                    rect.width,
                                    rect.height,
                                ),
                                Color::rgba8(48, 120, 196, 105),
                            )
                            .clip(text_clip),
                        );
                    }
                }
                if is_focused && selection.is_empty() {
                    scene.push_quad_in(
                        layer,
                        Quad::new(caret_bounds, Color::rgb8(226, 232, 240)).clip(text_clip),
                    );
                }
                if let Some(marked) = input_state.marked()
                    && !marked.is_empty()
                    && !content.is_empty()
                {
                    for rect in renderer.text_selection_rects_with_highlights(
                        text_id,
                        &content,
                        &style,
                        highlights.as_ref(),
                        text_viewport.width,
                        scale_factor,
                        scroll.y..scroll.y + text_viewport.height,
                        display_index(marked.start),
                        display_index(marked.end),
                    ) {
                        scene.push_quad_in(
                            layer,
                            Quad::new(
                                Rect::new(
                                    text_viewport.x + rect.x - scroll.x,
                                    text_viewport.y + rect.bottom() - scroll.y - 1.5,
                                    rect.width,
                                    1.0,
                                ),
                                style.color,
                            )
                            .clip(text_clip),
                        );
                    }
                }

                let (display_text, display_style) =
                    if content.is_empty() && !input.placeholder.is_empty() {
                        let mut placeholder_style = style.clone();
                        placeholder_style.color = Color::rgb8(132, 137, 148);
                        (input.placeholder.clone(), placeholder_style)
                    } else {
                        (content.clone(), style.clone())
                    };
                let text_bounds = Rect::new(
                    text_viewport.x - scroll.x,
                    text_viewport.y - scroll.y,
                    text_viewport.width,
                    content_size.height.max(text_viewport.height),
                );
                let mut text_run =
                    TextRun::new(text_id, display_text, text_bounds, display_style).clip(text_clip);
                if let Some(highlights) = &highlights {
                    text_run = text_run.with_highlights(highlights.clone());
                }
                scene.push_text_in(layer, text_run);
                for decoration in decorations {
                    push_text_paint_rect(
                        scene,
                        layer,
                        decoration,
                        Point::new(text_viewport.x - scroll.x, text_viewport.y - scroll.y),
                        text_clip,
                    );
                }
                text_input_regions.push(TextInputRegion {
                    id: element.runtime_id,
                    bounds: text_viewport,
                    clip: text_clip,
                    content,
                    password,
                    highlights,
                    style,
                    scroll,
                    max_scroll,
                    caret_bounds,
                });
                if input.multiline && max_scroll.y > 0.0 {
                    let scrollbar_bounds = Rect::new(
                        bounds.x,
                        text_viewport.y,
                        bounds.width,
                        text_viewport.height,
                    );
                    text_input_scroll = Some((
                        bounds,
                        scrollbar_bounds,
                        parent_clip.intersection(bounds).unwrap_or(text_clip),
                        scroll,
                        max_scroll,
                    ));
                }
            }
        }
        #[cfg(target_os = "macos")]
        ElementKind::NativeView(view) => {
            if layer.plane != ScenePlane::Base {
                return Err(UiError::NativeViewInOverlay(element.runtime_id));
            }
            if let Some(clip) = parent_clip.intersection(bounds) {
                native_views.push(NativeViewPlacement {
                    id: element.runtime_id,
                    view: view.clone(),
                    bounds,
                    clip,
                    corner_radius: element.visual.radius,
                    opacity: scene.current_opacity(),
                    z_index: layer.z_index,
                    source_order: order.source,
                });
            }
        }
        ElementKind::Container | ElementKind::ContainerQuery(_) => {}
    }

    let clips_children = element.layout.overflow.x != Overflow::Visible
        || element.layout.overflow.y != Overflow::Visible;
    let child_clip = if clips_children {
        match parent_clip.intersection(bounds) {
            Some(clip) => clip,
            None => {
                scene.restore_opacity(previous_opacity);
                return Ok(());
            }
        }
    } else {
        parent_clip
    };

    let is_scrollable = !matches!(&element.kind, ElementKind::TextInput(_))
        && (element.layout.overflow.x == Overflow::Scroll
            || element.layout.overflow.y == Overflow::Scroll);
    let mut scroll = Vector::ZERO;
    let mut scroll_max_offset = None;
    if is_scrollable {
        let max_offset = Vector::new(
            (layout.content_size.width - layout.size.width).max(0.0),
            (layout.content_size.height - layout.size.height).max(0.0),
        );
        let offset = scroll_offsets.entry(element.runtime_id).or_default();
        offset.x = offset.x.clamp(0.0, max_offset.x);
        offset.y = offset.y.clamp(0.0, max_offset.y);
        scroll = *offset;
        scroll_max_offset = Some(max_offset);
    } else if let Some(virtual_scroll) = &element.virtual_scroll {
        let max_offset_y = virtual_scroll
            .handle
            .max_offset(virtual_scroll.max_offset_y)
            .max(0.0);
        let offset = scroll_offsets.entry(element.runtime_id).or_default();
        offset.x = 0.0;
        offset.y = virtual_scroll.handle.offset().clamp(0.0, max_offset_y);
        scroll.y = offset.y - virtual_scroll.mount.layout_offset_y;
    }

    let child_origin = Point::new(bounds.x - scroll.x, bounds.y - scroll.y);
    for child in &element.children {
        paint_element(
            child,
            taffy,
            natural_bounds,
            element_bounds,
            scroll_offsets,
            hovered,
            pressed,
            dragging,
            drag_over,
            focused,
            scale_factor,
            scene,
            renderer,
            text_inputs,
            animations,
            animations_enabled,
            paint_time,
            transition_context,
            hit_regions,
            scroll_regions,
            scrollbar_states,
            dismiss_regions,
            #[cfg(target_os = "macos")]
            native_views,
            text_input_regions,
            selectable_text_indices,
            selectable_text_regions,
            static_text_selection,
            child_origin,
            child_clip,
            viewport,
            layer,
            source_order,
            state_text_color,
        )?;
    }

    if let Some((text_bounds, scrollbar_bounds, text_clip, text_scroll, max_offset)) =
        text_input_scroll
    {
        let scrollbar_order = PaintOrder {
            layer,
            source: *source_order,
        };
        *source_order = (*source_order).saturating_add(1);
        let region = ScrollRegion {
            id: element.runtime_id,
            bounds: text_bounds,
            scrollbar_bounds,
            clip: text_clip,
            max_offset,
            virtual_scroll: false,
            order,
            scrollbar_order,
        };
        scroll_regions.push(region);
        paint_vertical_scrollbar(
            scene,
            layer,
            region,
            text_scroll.y,
            scrollbar_states
                .get(&region.id)
                .copied()
                .unwrap_or_default(),
            paint_time,
        );
    }

    let vertical_scroll = if let Some(max_offset) = scroll_max_offset {
        Some((max_offset, scroll.y, false))
    } else if let Some(virtual_scroll) = &element.virtual_scroll {
        let max_offset = Vector::new(
            0.0,
            virtual_scroll
                .handle
                .max_offset(virtual_scroll.max_offset_y)
                .max(0.0),
        );
        let offset = scroll_offsets.entry(element.runtime_id).or_default();
        offset.x = 0.0;
        offset.y = offset.y.clamp(0.0, max_offset.y);
        Some((max_offset, offset.y, true))
    } else {
        None
    };

    // Ordinary overflow containers and retained virtual lists intentionally converge here. This
    // keeps their hit track, hover expansion, captured drag, paint style, and autohide behavior
    // identical; virtual lists differ only in how an offset change is propagated back to the view.
    if let Some((max_offset, scroll_offset_y, virtual_scroll)) = vertical_scroll {
        let scrollbar_order = PaintOrder {
            layer,
            source: *source_order,
        };
        *source_order = (*source_order).saturating_add(1);
        let region = ScrollRegion {
            id: element.runtime_id,
            bounds,
            scrollbar_bounds: bounds,
            clip: child_clip,
            max_offset,
            virtual_scroll,
            order,
            scrollbar_order,
        };
        scroll_regions.push(region);
        paint_vertical_scrollbar(
            scene,
            layer,
            region,
            scroll_offset_y,
            scrollbar_states
                .get(&region.id)
                .copied()
                .unwrap_or_default(),
            paint_time,
        );
    }
    scene.restore_opacity(previous_opacity);
    Ok(())
}

fn push_text_paint_rect(
    scene: &mut Scene,
    layer: PaintLayerKey,
    paint: TextPaintRect,
    origin: Point,
    clip: Rect,
) {
    let rect = Rect::new(
        origin.x + paint.rect.x,
        origin.y + paint.rect.y,
        paint.rect.width,
        paint.rect.height,
    );
    match paint.kind {
        TextPaintKind::Solid => {
            scene.push_quad_in(layer, Quad::new(rect, paint.color).clip(clip));
        }
        TextPaintKind::WavyUnderline {
            baseline,
            amplitude,
            thickness,
            wavelength,
        } => {
            scene.push_wavy_underline_in(
                layer,
                WavyUnderline::new(
                    rect,
                    origin.y + baseline,
                    amplitude,
                    thickness,
                    wavelength,
                    paint.color,
                )
                .clip(clip),
            );
        }
    }
}

fn effective_cursor_style(element: &Element, selectable_text: bool) -> Option<CursorStyle> {
    if element.accessibility.disabled {
        return element.disabled_style.cursor_style.or_else(|| {
            element
                .cursor_style_explicit
                .then_some(element.cursor_style)
                .flatten()
        });
    }
    if element.cursor_style_explicit {
        return element.cursor_style;
    }
    if element.drag_source {
        return Some(CursorStyle::OpenHand);
    }
    element
        .cursor_style
        .or_else(|| selectable_text.then_some(CursorStyle::IBeam))
}

fn paint_vertical_scrollbar(
    scene: &mut Scene,
    layer: PaintLayerKey,
    region: ScrollRegion,
    scroll_offset_y: f32,
    state: ScrollbarState,
    now: Instant,
) {
    if !state.visible(now) {
        return;
    }
    let Some(scrollbar) = vertical_scrollbar_geometry(region, scroll_offset_y) else {
        return;
    };
    let expanded = state.hovered || state.dragging;
    let width = if expanded { 8.0 } else { 4.0 };
    let alpha = if expanded { 210 } else { 150 };
    scene.push_quad_in(
        layer,
        Quad::new(
            Rect::new(
                region.scrollbar_bounds.right() - 2.0 - width,
                scrollbar.thumb.y + 2.0,
                width,
                (scrollbar.thumb.height - 4.0).max(4.0),
            ),
            Color::rgba8(142, 147, 160, alpha),
        )
        .radius(width * 0.5)
        .clip(region.clip),
    );
}

fn vertical_scrollbar_geometry(
    region: ScrollRegion,
    scroll_offset_y: f32,
) -> Option<VerticalScrollbarGeometry> {
    let bounds = region.scrollbar_bounds;
    let viewport_height = bounds.height;
    if region.max_offset.y <= 0.0 || viewport_height <= 0.0 {
        return None;
    }
    let content_height = viewport_height + region.max_offset.y;
    let thumb_height = (viewport_height * viewport_height / content_height)
        .max(24.0)
        .min(viewport_height);
    let travel = viewport_height - thumb_height;
    let thumb_y =
        bounds.y + travel * (scroll_offset_y.clamp(0.0, region.max_offset.y) / region.max_offset.y);
    Some(VerticalScrollbarGeometry {
        track: Rect::new(
            (bounds.right() - 12.0).max(bounds.x),
            bounds.y,
            bounds.width.min(12.0),
            viewport_height,
        ),
        thumb: Rect::new(
            (bounds.right() - 12.0).max(bounds.x),
            thumb_y,
            bounds.width.min(12.0),
            thumb_height,
        ),
        travel,
    })
}

#[derive(Clone, Copy)]
struct FocusCandidate {
    id: ElementId,
    tab_index: i16,
    order: usize,
}

struct FocusCollection<'a> {
    focusable_ids: &'a mut HashSet<ElementId>,
    clickable_ids: &'a mut HashSet<ElementId>,
    candidates: &'a mut Vec<FocusCandidate>,
    active_trap: Option<ElementId>,
}

#[derive(Default)]
struct FormCollection {
    fields: Vec<FormField>,
    fields_truncated: bool,
    issues: Vec<ValidationIssue>,
    issues_truncated: bool,
    first_focusable_invalid: Option<ElementId>,
}

fn find_element(element: &Element, id: ElementId) -> Option<&Element> {
    if element.runtime_id == id {
        return Some(element);
    }
    element
        .children
        .iter()
        .find_map(|child| find_element(child, id))
}

fn collect_form_controls(
    element: &Element,
    text_inputs: &HashMap<ElementId, TextInputState>,
    focusable_ids: &HashSet<ElementId>,
    collection: &mut FormCollection,
) {
    if element.is_display_none() || element.is_visibility_hidden() {
        return;
    }
    // Forms do not inherit controls from nested forms. This also gives malformed declarative
    // nesting deterministic browser-like ownership without retaining a second membership index.
    if element.form {
        return;
    }
    if !element.accessibility.disabled {
        if let Some(input) = text_inputs.get(&element.runtime_id) {
            if collection.fields.len() < MAX_FORM_FIELDS {
                collection.fields.push(FormField::new(
                    element.runtime_id,
                    input.committed_shared_text(),
                ));
            } else {
                collection.fields_truncated = true;
            }
        }
        if element.accessibility.invalid {
            if collection.first_focusable_invalid.is_none()
                && focusable_ids.contains(&element.runtime_id)
            {
                collection.first_focusable_invalid = Some(element.runtime_id);
            }
            if collection.issues.len() < MAX_VALIDATION_ISSUES {
                collection.issues.push(ValidationIssue::new(
                    element.runtime_id,
                    element.accessibility.validation_message.clone(),
                    element.accessibility.validation_message_truncated,
                ));
            } else {
                collection.issues_truncated = true;
            }
        }
    }
    for child in &element.children {
        collect_form_controls(child, text_inputs, focusable_ids, collection);
    }
}

fn validation_announcement_message(report: &ValidationReport) -> Arc<str> {
    let first = report
        .first()
        .and_then(ValidationIssue::message)
        .unwrap_or("This field is invalid.");
    let mut message = if report.issues().len() == 1 && !report.is_truncated() {
        first.to_owned()
    } else {
        let qualifier = if report.is_truncated() {
            "at least "
        } else {
            ""
        };
        format!(
            "{qualifier}{} fields need attention. {first}",
            report.issues().len()
        )
    };
    if message.len() > MAX_VALIDATION_MESSAGE_BYTES {
        let mut end = MAX_VALIDATION_MESSAGE_BYTES;
        while !message.is_char_boundary(end) {
            end -= 1;
        }
        message.truncate(end);
    }
    Arc::from(message)
}

fn topmost_focus_trap(root: &Element) -> Option<ElementId> {
    fn visit(
        element: &Element,
        parent_layer: PaintLayerKey,
        source: &mut usize,
        topmost: &mut Option<(PaintOrder, ElementId)>,
    ) {
        if element.is_display_none() || element.is_visibility_hidden() {
            return;
        }
        let plane = element.plane.unwrap_or(parent_layer.plane);
        let z_index = if plane == parent_layer.plane {
            parent_layer
                .z_index
                .saturating_add(element.z_index.unwrap_or(0))
        } else {
            element.z_index.unwrap_or(0)
        };
        let layer = PaintLayerKey { plane, z_index };
        let order = PaintOrder {
            layer,
            source: *source,
        };
        *source = source.saturating_add(1);
        if element.focus_trap
            && topmost
                .as_ref()
                .is_none_or(|(previous, _)| order > *previous)
        {
            *topmost = Some((order, element.runtime_id));
        }
        for child in &element.children {
            visit(child, layer, source, topmost);
        }
    }

    let mut source = 0;
    let mut topmost = None;
    visit(root, PaintLayerKey::default(), &mut source, &mut topmost);
    topmost.map(|(_, id)| id)
}

fn collect_focus_candidates(
    element: &Element,
    collection: &mut FocusCollection<'_>,
    inside_radio_group: bool,
    radio_tab_stop: Option<ElementId>,
    inside_tab_list: bool,
    tab_stop: Option<ElementId>,
    inside_active_focus_trap: bool,
) {
    if element.is_display_none() || element.is_visibility_hidden() {
        return;
    }
    let inside_active_focus_trap = inside_active_focus_trap
        || collection.active_trap.is_none()
        || collection.active_trap == Some(element.runtime_id);
    if inside_active_focus_trap && element.focusable && !element.accessibility.disabled {
        collection.focusable_ids.insert(element.runtime_id);
        let radio_is_tab_stop = element.accessibility.role != AccessibilityRole::RadioButton
            || !inside_radio_group
            || radio_tab_stop == Some(element.runtime_id);
        let tab_is_tab_stop = element.accessibility.role != AccessibilityRole::Tab
            || !inside_tab_list
            || tab_stop == Some(element.runtime_id);
        if element.tab_index >= 0 && radio_is_tab_stop && tab_is_tab_stop {
            collection.candidates.push(FocusCandidate {
                id: element.runtime_id,
                tab_index: element.tab_index,
                order: collection.candidates.len(),
            });
        }
    }
    if element.clickable && !element.accessibility.disabled {
        collection.clickable_ids.insert(element.runtime_id);
    }
    let (inside_radio_group, radio_tab_stop) =
        if element.accessibility.role == AccessibilityRole::RadioGroup {
            (true, radio_group_tab_stop(element))
        } else {
            (inside_radio_group, radio_tab_stop)
        };
    let (inside_tab_list, tab_stop) = if element.accessibility.role == AccessibilityRole::TabList {
        (true, tab_list_tab_stop(element))
    } else {
        (inside_tab_list, tab_stop)
    };
    for child in &element.children {
        collect_focus_candidates(
            child,
            collection,
            inside_radio_group,
            radio_tab_stop,
            inside_tab_list,
            tab_stop,
            inside_active_focus_trap,
        );
    }
}

fn tab_list_tab_stop(list: &Element) -> Option<ElementId> {
    fn visit(
        element: &Element,
        list_id: ElementId,
        first: &mut Option<ElementId>,
        selected: &mut Option<ElementId>,
    ) {
        if element.is_display_none()
            || element.is_visibility_hidden()
            || (element.runtime_id != list_id
                && element.accessibility.role == AccessibilityRole::TabList)
        {
            return;
        }
        if element.accessibility.role == AccessibilityRole::Tab
            && element.focusable
            && !element.accessibility.disabled
            && element.tab_index >= 0
        {
            first.get_or_insert(element.runtime_id);
            if selected.is_none() && element.accessibility.selected {
                *selected = Some(element.runtime_id);
            }
        }
        for child in &element.children {
            visit(child, list_id, first, selected);
        }
    }

    let mut first = None;
    let mut selected = None;
    visit(list, list.runtime_id, &mut first, &mut selected);
    selected.or(first)
}

fn tab_list_for_focused(element: &Element, focused: ElementId) -> Option<&Element> {
    fn visit(element: &Element, focused: ElementId) -> (bool, Option<&Element>) {
        if element.is_display_none() || element.is_visibility_hidden() {
            return (false, None);
        }
        let mut contains = element.runtime_id == focused;
        for child in &element.children {
            let (child_contains, child_list) = visit(child, focused);
            if let Some(list) = child_list {
                return (true, Some(list));
            }
            contains |= child_contains;
        }
        if contains && element.accessibility.role == AccessibilityRole::TabList {
            (true, Some(element))
        } else {
            (contains, None)
        }
    }

    visit(element, focused).1
}

fn tab_stop_for_focused_tab(root: &Element, focused: ElementId) -> Option<ElementId> {
    let focused_element = find_element(root, focused)?;
    if focused_element.accessibility.role != AccessibilityRole::Tab {
        return None;
    }
    tab_list_tab_stop(tab_list_for_focused(root, focused)?)
}

#[derive(Default)]
struct TabNeighbors {
    first: Option<ElementId>,
    previous: Option<ElementId>,
    next: Option<ElementId>,
    last: Option<ElementId>,
    found_focus: bool,
}

fn collect_tab_neighbors(
    element: &Element,
    list_id: ElementId,
    focused: ElementId,
    neighbors: &mut TabNeighbors,
) {
    if element.is_display_none()
        || element.is_visibility_hidden()
        || (element.runtime_id != list_id
            && element.accessibility.role == AccessibilityRole::TabList)
    {
        return;
    }
    if element.accessibility.role == AccessibilityRole::Tab
        && element.focusable
        && element.clickable
        && !element.accessibility.disabled
    {
        let id = element.runtime_id;
        neighbors.first.get_or_insert(id);
        if neighbors.found_focus && neighbors.next.is_none() {
            neighbors.next = Some(id);
        } else if id == focused {
            neighbors.found_focus = true;
        } else if !neighbors.found_focus {
            neighbors.previous = Some(id);
        }
        neighbors.last = Some(id);
    }
    for child in &element.children {
        collect_tab_neighbors(child, list_id, focused, neighbors);
    }
}

fn radio_group_tab_stop(group: &Element) -> Option<ElementId> {
    fn visit(
        element: &Element,
        group_id: ElementId,
        first: &mut Option<ElementId>,
        selected: &mut Option<ElementId>,
    ) {
        if element.is_display_none()
            || element.is_visibility_hidden()
            || (element.runtime_id != group_id
                && element.accessibility.role == AccessibilityRole::RadioGroup)
        {
            return;
        }
        if element.accessibility.role == AccessibilityRole::RadioButton
            && element.focusable
            && !element.accessibility.disabled
            && element.tab_index >= 0
        {
            first.get_or_insert(element.runtime_id);
            if selected.is_none() && element.accessibility.toggled == Some(ToggleState::On) {
                *selected = Some(element.runtime_id);
            }
        }
        for child in &element.children {
            visit(child, group_id, first, selected);
        }
    }

    let mut first = None;
    let mut selected = None;
    visit(group, group.runtime_id, &mut first, &mut selected);
    selected.or(first)
}

fn radio_group_for_focused(element: &Element, focused: ElementId) -> Option<&Element> {
    fn visit(element: &Element, focused: ElementId) -> (bool, Option<&Element>) {
        if element.is_display_none() || element.is_visibility_hidden() {
            return (false, None);
        }
        let mut contains = element.runtime_id == focused;
        for child in &element.children {
            let (child_contains, child_group) = visit(child, focused);
            if let Some(group) = child_group {
                return (true, Some(group));
            }
            contains |= child_contains;
        }
        if contains && element.accessibility.role == AccessibilityRole::RadioGroup {
            (true, Some(element))
        } else {
            (contains, None)
        }
    }

    visit(element, focused).1
}

#[derive(Default)]
struct RadioNeighbors {
    first: Option<ElementId>,
    previous: Option<ElementId>,
    next: Option<ElementId>,
    last: Option<ElementId>,
    found_focus: bool,
}

fn collect_radio_neighbors(
    element: &Element,
    group_id: ElementId,
    focused: ElementId,
    neighbors: &mut RadioNeighbors,
) {
    if element.is_display_none()
        || element.is_visibility_hidden()
        || (element.runtime_id != group_id
            && element.accessibility.role == AccessibilityRole::RadioGroup)
    {
        return;
    }
    if element.accessibility.role == AccessibilityRole::RadioButton
        && element.focusable
        && element.clickable
        && !element.accessibility.disabled
    {
        let id = element.runtime_id;
        neighbors.first.get_or_insert(id);
        if neighbors.found_focus && neighbors.next.is_none() {
            neighbors.next = Some(id);
        } else if id == focused {
            neighbors.found_focus = true;
        } else if !neighbors.found_focus {
            neighbors.previous = Some(id);
        }
        neighbors.last = Some(id);
    }
    for child in &element.children {
        collect_radio_neighbors(child, group_id, focused, neighbors);
    }
}

struct DispatchMetadata<'a> {
    parents: &'a mut HashMap<ElementId, ElementId>,
    key_contexts: &'a mut HashMap<ElementId, KeyContext>,
    activation_targets: &'a mut HashMap<ElementId, ElementId>,
    invalid_ids: &'a mut HashSet<ElementId>,
    form_ids: &'a mut HashSet<ElementId>,
    form_submitter_ids: &'a mut HashSet<ElementId>,
    context_menu_ids: &'a mut HashSet<ElementId>,
    mouse_listener_bindings: &'a mut Vec<MouseListenerBinding>,
    mouse_listener_ranges: &'a mut HashMap<ElementId, Range<usize>>,
    mouse_listener_elements: &'a mut Vec<ElementId>,
    key_listener_bindings: &'a mut Vec<KeyListenerBinding>,
    key_listener_ranges: &'a mut HashMap<ElementId, Range<usize>>,
    action_listener_bindings: &'a mut Vec<ActionListenerBinding>,
    action_listener_ranges: &'a mut HashMap<ElementId, Range<usize>>,
    scroll_wheel_ids: &'a mut HashSet<ElementId>,
    touch_ids: &'a mut HashSet<ElementId>,
    mouse_pressure_ids: &'a mut HashSet<ElementId>,
    pinch_ids: &'a mut HashSet<ElementId>,
    rotation_ids: &'a mut HashSet<ElementId>,
    smart_magnify_ids: &'a mut HashSet<ElementId>,
}

fn collect_dispatch_metadata(
    element: &Element,
    parent: Option<ElementId>,
    metadata: &mut DispatchMetadata<'_>,
) {
    if element.is_display_none() || element.is_visibility_hidden() {
        return;
    }
    if let Some(parent) = parent {
        metadata.parents.insert(element.runtime_id, parent);
    }
    if let Some(context) = &element.key_context {
        metadata
            .key_contexts
            .insert(element.runtime_id, context.clone());
    }
    if let Some(target) = element.activation_target
        && !element.accessibility.disabled
        && target != element.runtime_id
    {
        metadata
            .activation_targets
            .insert(element.runtime_id, target);
    }
    if element.accessibility.invalid {
        metadata.invalid_ids.insert(element.runtime_id);
    }
    if element.form {
        metadata.form_ids.insert(element.runtime_id);
    }
    if element.form_submitter && !element.accessibility.disabled {
        metadata.form_submitter_ids.insert(element.runtime_id);
    }
    if element.context_menu_listener && !element.accessibility.disabled {
        metadata.context_menu_ids.insert(element.runtime_id);
    }
    if let Some(listeners) = &element.mouse_listeners
        && !element.accessibility.disabled
    {
        let start = metadata.mouse_listener_bindings.len();
        metadata
            .mouse_listener_bindings
            .extend(listeners.iter().copied());
        let end = metadata.mouse_listener_bindings.len();
        metadata
            .mouse_listener_ranges
            .insert(element.runtime_id, start..end);
        metadata.mouse_listener_elements.push(element.runtime_id);
    }
    if let Some(listeners) = &element.key_listeners
        && !element.accessibility.disabled
    {
        let start = metadata.key_listener_bindings.len();
        metadata
            .key_listener_bindings
            .extend(listeners.iter().copied());
        let end = metadata.key_listener_bindings.len();
        metadata
            .key_listener_ranges
            .insert(element.runtime_id, start..end);
    }
    if let Some(listeners) = &element.action_listeners
        && !element.accessibility.disabled
    {
        let start = metadata.action_listener_bindings.len();
        metadata
            .action_listener_bindings
            .extend(listeners.iter().copied());
        let end = metadata.action_listener_bindings.len();
        metadata
            .action_listener_ranges
            .insert(element.runtime_id, start..end);
    }
    if element.scroll_wheel_listener && !element.accessibility.disabled {
        metadata.scroll_wheel_ids.insert(element.runtime_id);
    }
    if element.touch_listener && !element.accessibility.disabled {
        metadata.touch_ids.insert(element.runtime_id);
    }
    if element.mouse_pressure_listener && !element.accessibility.disabled {
        metadata.mouse_pressure_ids.insert(element.runtime_id);
    }
    if element.pinch_listener && !element.accessibility.disabled {
        metadata.pinch_ids.insert(element.runtime_id);
    }
    if element.rotation_listener && !element.accessibility.disabled {
        metadata.rotation_ids.insert(element.runtime_id);
    }
    if element.smart_magnify_listener && !element.accessibility.disabled {
        metadata.smart_magnify_ids.insert(element.runtime_id);
    }
    for child in &element.children {
        collect_dispatch_metadata(child, Some(element.runtime_id), metadata);
    }
}

fn collect_drop_predicates(
    element: &Element,
    predicates: &mut HashMap<(ElementId, TypeId), DropPredicateCallback>,
) {
    if element.is_display_none() || element.is_visibility_hidden() {
        return;
    }
    for predicate in &element.drop_predicates {
        predicates.insert(
            (element.runtime_id, predicate.type_id),
            Arc::clone(&predicate.callback),
        );
    }
    for child in &element.children {
        collect_drop_predicates(child, predicates);
    }
}

#[cfg(target_os = "macos")]
fn drop_acceptance(
    predicates: &HashMap<(ElementId, TypeId), DropPredicateCallback>,
    id: ElementId,
    value_type: TypeId,
) -> ExternalDropAcceptance {
    predicates
        .get(&(id, value_type))
        .map(|predicate| ExternalDropAcceptance::Predicate(Arc::clone(predicate)))
        .unwrap_or(ExternalDropAcceptance::Always)
}

fn collect_tooltips(
    element: &Element,
    tooltips: &mut HashMap<ElementId, Tooltip>,
) -> Result<(), UiError> {
    if element.is_display_none() || element.is_visibility_hidden() {
        return Ok(());
    }
    if let Some(tooltip) = &element.tooltip
        && !element.accessibility.disabled
    {
        if tooltips.len() >= MAX_TOOLTIPS_PER_WINDOW {
            return Err(UiError::TooManyTooltips);
        }
        tooltips.insert(element.runtime_id, tooltip.clone());
    }
    for child in &element.children {
        collect_tooltips(child, tooltips)?;
    }
    Ok(())
}

fn find_auto_focus(element: &Element) -> Option<ElementId> {
    if element.is_display_none() || element.is_visibility_hidden() {
        return None;
    }
    if element.auto_focus && element.focusable && !element.accessibility.disabled {
        return Some(element.runtime_id);
    }
    element.children.iter().find_map(find_auto_focus)
}

struct AccessibilityBuildContext<'a> {
    element_bounds: &'a HashMap<ElementId, Rect>,
    scroll_offsets: &'a HashMap<ElementId, Vector>,
    text_inputs: &'a HashMap<ElementId, TextInputState>,
    selectable_texts: &'a [SelectableTextEntry],
    selectable_text_indices: &'a HashMap<ElementId, usize>,
    static_text_selection: Option<StaticTextSelection>,
    accessibility_text_ids: &'a HashMap<ElementId, AccessibilityNodeId>,
    accessible_ids: &'a HashSet<ElementId>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum AccessibilityBuildMode {
    Full,
    ScrollContainers,
}

fn accessibility_scroll_translation(
    element: &Element,
    scroll_offsets: &HashMap<ElementId, Vector>,
) -> Option<Vector> {
    let overflow_scroll = !matches!(&element.kind, ElementKind::TextInput(_))
        && (element.layout.overflow.x == Overflow::Scroll
            || element.layout.overflow.y == Overflow::Scroll);
    if overflow_scroll {
        return Some(
            scroll_offsets
                .get(&element.runtime_id)
                .copied()
                .unwrap_or_default(),
        );
    }
    element.virtual_scroll.as_ref().map(|virtual_scroll| {
        let offset_y = scroll_offsets
            .get(&element.runtime_id)
            .map_or_else(|| virtual_scroll.handle.offset(), |offset| offset.y);
        Vector::new(0.0, offset_y - virtual_scroll.mount.layout_offset_y)
    })
}

fn accessibility_unscrolled_bounds(bounds: Rect, translation: Vector) -> Rect {
    Rect::new(
        bounds.x + translation.x,
        bounds.y + translation.y,
        bounds.width,
        bounds.height,
    )
}

fn build_accessibility_nodes(
    element: &Element,
    context: &AccessibilityBuildContext<'_>,
    nodes: &mut Vec<(AccessibilityNodeId, AccessibilityNode)>,
    inherited_scroll_translation: Vector,
    mode: AccessibilityBuildMode,
) {
    if !context.accessible_ids.contains(&element.runtime_id) {
        return;
    }
    let Some(viewport_bounds) = context.element_bounds.get(&element.runtime_id).copied() else {
        return;
    };
    let own_scroll_translation = accessibility_scroll_translation(element, context.scroll_offsets);
    let content_scroll_translation =
        inherited_scroll_translation + own_scroll_translation.unwrap_or_default();
    if mode == AccessibilityBuildMode::ScrollContainers && own_scroll_translation.is_none() {
        for child in &element.children {
            build_accessibility_nodes(child, context, nodes, content_scroll_translation, mode);
        }
        return;
    }
    let bounds = accessibility_unscrolled_bounds(viewport_bounds, content_scroll_translation);
    let mut node = AccessibilityNode::new(accessibility_role(element.accessibility.role));
    node.set_bounds(accessibility_rect(bounds));
    if let Some(scroll) = own_scroll_translation {
        node.set_transform(Affine::translate(AccessibilityVector::new(
            -(scroll.x as f64),
            -(scroll.y as f64),
        )));
        node.set_clips_children();
        node.set_scroll_x(scroll.x as f64);
        node.set_scroll_y(scroll.y as f64);
    }
    let mut children = element
        .children
        .iter()
        .filter(|child| context.accessible_ids.contains(&child.runtime_id))
        .map(|child| accessibility_id(child.runtime_id))
        .collect::<Vec<_>>();
    let text_input = context.text_inputs.get(&element.runtime_id).zip(
        context
            .accessibility_text_ids
            .get(&element.runtime_id)
            .copied(),
    );
    let selectable_text = context
        .selectable_text_indices
        .get(&element.runtime_id)
        .and_then(|index| {
            context
                .selectable_texts
                .get(*index)
                .map(|entry| (*index, entry))
        })
        .zip(
            context
                .accessibility_text_ids
                .get(&element.runtime_id)
                .copied(),
        );
    if let Some(text_id) = text_input
        .map(|(_, text_id)| text_id)
        .or_else(|| selectable_text.map(|(_, text_id)| text_id))
    {
        children.push(text_id);
    }
    node.set_children(children);
    if let Some(label) = accessibility_label(element) {
        node.set_label(label);
    }
    if let Some(description) = &element.accessibility.description {
        node.set_description(description.to_string());
    }
    if let Some((state, text_id)) = text_input {
        let password = matches!(
            &element.kind,
            ElementKind::TextInput(input) if input.password
        )
        .then(|| PasswordDisplay::new(state.text()));
        let accessible_text = password
            .as_ref()
            .map_or_else(|| state.shared_text(), |display| display.content.clone());
        let accessible_character_index = |source_index| {
            password.as_ref().map_or_else(
                || state.accessibility_character_index(source_index),
                |display| {
                    accessibility_character_index(
                        &display.content,
                        display.display_index(source_index),
                    )
                },
            )
        };
        let accessible_character_lengths = password.as_ref().map_or_else(
            || state.accessibility_character_lengths(),
            |display| selectable_character_lengths(&display.content),
        );
        node.set_value(accessible_text.to_string());
        if let ElementKind::TextInput(input) = &element.kind
            && !input.placeholder.is_empty()
        {
            node.set_placeholder(input.placeholder.to_string());
        }
        let anchor = TextPosition {
            node: text_id,
            character_index: accessible_character_index(state.anchor()),
        };
        let focus = TextPosition {
            node: text_id,
            character_index: accessible_character_index(state.caret()),
        };
        node.set_text_selection(TextSelection { anchor, focus });
        if !element.accessibility.disabled {
            node.add_action(Action::SetValue);
            node.add_action(Action::SetTextSelection);
        }

        let mut text_node = AccessibilityNode::new(Role::TextRun);
        text_node.set_bounds(accessibility_rect(bounds));
        text_node.set_value(accessible_text.to_string());
        text_node.set_character_lengths(accessible_character_lengths);
        nodes.push((text_id, text_node));
    } else if let Some(((document_index, entry), text_id)) = selectable_text {
        node.set_value(entry.content.to_string());
        let range = static_selection_range_for_entry(
            context.static_text_selection,
            context.selectable_text_indices,
            document_index,
            entry.content.len(),
        )
        .unwrap_or(0..0);
        let reversed = context
            .static_text_selection
            .and_then(|selection| {
                let anchor = (
                    *context.selectable_text_indices.get(&selection.anchor.id)?,
                    selection.anchor.offset,
                );
                let focus = (
                    *context.selectable_text_indices.get(&selection.focus.id)?,
                    selection.focus.offset,
                );
                Some(anchor > focus)
            })
            .unwrap_or(false);
        let (anchor_offset, focus_offset) = if reversed {
            (range.end, range.start)
        } else {
            (range.start, range.end)
        };
        node.set_text_selection(TextSelection {
            anchor: TextPosition {
                node: text_id,
                character_index: accessibility_character_index_from_lengths(
                    &entry.character_lengths,
                    anchor_offset,
                ),
            },
            focus: TextPosition {
                node: text_id,
                character_index: accessibility_character_index_from_lengths(
                    &entry.character_lengths,
                    focus_offset,
                ),
            },
        });
        if !element.accessibility.disabled {
            node.add_action(Action::SetTextSelection);
        }

        let mut text_node = AccessibilityNode::new(Role::TextRun);
        text_node.set_bounds(accessibility_rect(bounds));
        text_node.set_value(entry.content.to_string());
        text_node.set_character_lengths(entry.character_lengths.to_vec());
        nodes.push((text_id, text_node));
    } else if let Some(value) = &element.accessibility.value {
        node.set_value(value.to_string());
    }
    if element.accessibility.disabled {
        node.set_disabled();
    }
    if element.accessibility.required {
        node.set_required();
    }
    if element.accessibility.invalid {
        node.set_invalid(AccessibilityInvalid::True);
        if let Some(message) = &element.accessibility.validation_message {
            node.set_description(message.to_string());
        }
    }
    if element.accessibility.role == AccessibilityRole::Tab {
        node.set_selected(element.accessibility.selected);
    } else if element.accessibility.selected {
        node.set_selected(true);
    }
    if let Some(toggled) = element.accessibility.toggled {
        node.set_toggled(match toggled {
            ToggleState::Off => AccessibilityToggled::False,
            ToggleState::On => AccessibilityToggled::True,
            ToggleState::Mixed => AccessibilityToggled::Mixed,
        });
    }
    if let Some(expanded) = element.accessibility.expanded {
        node.set_expanded(expanded);
    }
    if let Some(target) = element.accessibility.relations.controls()
        && context.accessible_ids.contains(&target)
    {
        node.push_controlled(accessibility_id(target));
    }
    if let Some(target) = element.accessibility.relations.active_descendant()
        && context.accessible_ids.contains(&target)
    {
        node.set_active_descendant(accessibility_id(target));
    }
    if let Some(target) = element.accessibility.relations.labelled_by()
        && target != element.runtime_id
        && context.accessible_ids.contains(&target)
    {
        node.push_labelled_by(accessibility_id(target));
    }
    if let Some(target) = element.accessibility.relations.described_by()
        && target != element.runtime_id
        && context.accessible_ids.contains(&target)
    {
        node.push_described_by(accessibility_id(target));
    }
    if let Some(target) = element.accessibility.relations.described_by_secondary()
        && target != element.runtime_id
        && context.accessible_ids.contains(&target)
    {
        node.push_described_by(accessibility_id(target));
    }
    if let Some(popup) = element.accessibility.has_popup {
        node.set_has_popup(match popup {
            AccessibilityPopup::Menu => AccessibilityHasPopup::Menu,
            AccessibilityPopup::ListBox => AccessibilityHasPopup::Listbox,
            AccessibilityPopup::Tree => AccessibilityHasPopup::Tree,
            AccessibilityPopup::Grid => AccessibilityHasPopup::Grid,
            AccessibilityPopup::Dialog => AccessibilityHasPopup::Dialog,
        });
    }
    if let Some(behavior) = element.accessibility.auto_complete {
        node.set_auto_complete(match behavior {
            AccessibilityAutoComplete::Inline => NativeAccessibilityAutoComplete::Inline,
            AccessibilityAutoComplete::List => NativeAccessibilityAutoComplete::List,
            AccessibilityAutoComplete::Both => NativeAccessibilityAutoComplete::Both,
        });
    }
    if let Some(orientation) = element.accessibility.orientation {
        node.set_orientation(match orientation {
            AccessibilityOrientation::Horizontal => NativeAccessibilityOrientation::Horizontal,
            AccessibilityOrientation::Vertical => NativeAccessibilityOrientation::Vertical,
        });
    }
    if element.accessibility.modal {
        node.set_modal();
    }
    let collection = element.accessibility.collection;
    if let Some(value) = collection.row_count() {
        node.set_row_count(value);
    }
    if let Some(value) = collection.column_count() {
        node.set_column_count(value);
    }
    if let Some(value) = collection.row_index() {
        node.set_row_index(value);
    }
    if let Some(value) = collection.column_index() {
        node.set_column_index(value);
    }
    if let Some(value) = collection.level() {
        node.set_level(value);
    }
    if let Some(value) = collection.size_of_set() {
        node.set_size_of_set(value);
    }
    if let Some(value) = collection.position_in_set() {
        node.set_position_in_set(value);
    }
    if let Some(direction) = collection.sort_direction {
        node.set_sort_direction(match direction {
            AccessibilitySortDirection::Ascending => NativeAccessibilitySortDirection::Ascending,
            AccessibilitySortDirection::Descending => NativeAccessibilitySortDirection::Descending,
            AccessibilitySortDirection::Other => NativeAccessibilitySortDirection::Other,
        });
    }
    if element.focusable && !element.accessibility.disabled {
        node.add_action(Action::Focus);
        node.add_action(Action::Blur);
    }
    if element.clickable && !element.accessibility.disabled {
        node.add_action(Action::Click);
    }
    nodes.push((accessibility_id(element.runtime_id), node));

    for child in &element.children {
        build_accessibility_nodes(child, context, nodes, content_scroll_translation, mode);
    }
}

fn accessibility_label(element: &Element) -> Option<String> {
    if element.is_display_none() || element.is_visibility_hidden() || element.accessibility.hidden {
        return None;
    }
    if let Some(label) = &element.accessibility.label {
        return Some(label.to_string());
    }
    if let ElementKind::Text(content) = &element.kind {
        return Some(content.to_string());
    }
    if let ElementKind::StyledText(content) = &element.kind {
        return Some(content.content().to_string());
    }
    if element.accessibility.role == AccessibilityRole::GenericContainer {
        return None;
    }
    let mut label = String::new();
    collect_text(element, &mut label);
    (!label.is_empty()).then_some(label)
}

fn collect_text(element: &Element, output: &mut String) {
    if element.is_display_none() || element.is_visibility_hidden() || element.accessibility.hidden {
        return;
    }
    if let ElementKind::Text(content) = &element.kind {
        if !output.is_empty() {
            output.push(' ');
        }
        output.push_str(content);
    }
    if let ElementKind::StyledText(content) = &element.kind {
        if !output.is_empty() {
            output.push(' ');
        }
        output.push_str(content.content());
    }
    for child in &element.children {
        collect_text(child, output);
    }
}

fn accessibility_id(id: ElementId) -> AccessibilityNodeId {
    AccessibilityNodeId(id.value())
}

fn accessibility_rect(rect: Rect) -> AccessibilityRect {
    AccessibilityRect {
        x0: rect.x as f64,
        y0: rect.y as f64,
        x1: rect.right() as f64,
        y1: rect.bottom() as f64,
    }
}

fn accessibility_role(role: AccessibilityRole) -> Role {
    match role {
        AccessibilityRole::GenericContainer => Role::GenericContainer,
        AccessibilityRole::Label => Role::Label,
        AccessibilityRole::Button => Role::Button,
        AccessibilityRole::Link => Role::Link,
        AccessibilityRole::Image => Role::Image,
        AccessibilityRole::List => Role::List,
        AccessibilityRole::ListItem => Role::ListItem,
        AccessibilityRole::Heading => Role::Heading,
        AccessibilityRole::CheckBox => Role::CheckBox,
        AccessibilityRole::RadioButton => Role::RadioButton,
        AccessibilityRole::RadioGroup => Role::RadioGroup,
        AccessibilityRole::Switch => Role::Switch,
        AccessibilityRole::TextInput => Role::TextInput,
        AccessibilityRole::PasswordInput => Role::PasswordInput,
        AccessibilityRole::MultilineTextInput => Role::MultilineTextInput,
        AccessibilityRole::Dialog => Role::Dialog,
        AccessibilityRole::AlertDialog => Role::AlertDialog,
        AccessibilityRole::Menu => Role::Menu,
        AccessibilityRole::MenuItem => Role::MenuItem,
        AccessibilityRole::MenuItemCheckBox => Role::MenuItemCheckBox,
        AccessibilityRole::MenuItemRadio => Role::MenuItemRadio,
        AccessibilityRole::Separator => Role::Splitter,
        AccessibilityRole::Group => Role::Group,
        AccessibilityRole::Region => Role::Region,
        AccessibilityRole::ListBox => Role::ListBox,
        AccessibilityRole::ListBoxOption => Role::ListBoxOption,
        AccessibilityRole::ComboBox => Role::ComboBox,
        AccessibilityRole::EditableComboBox => Role::EditableComboBox,
        AccessibilityRole::Table => Role::Table,
        AccessibilityRole::Tree => Role::Tree,
        AccessibilityRole::Grid => Role::Grid,
        AccessibilityRole::Row => Role::Row,
        AccessibilityRole::ColumnHeader => Role::ColumnHeader,
        AccessibilityRole::RowHeader => Role::RowHeader,
        AccessibilityRole::GridCell => Role::GridCell,
        AccessibilityRole::TreeItem => Role::TreeItem,
        AccessibilityRole::Tab => Role::Tab,
        AccessibilityRole::TabList => Role::TabList,
        AccessibilityRole::TabPanel => Role::TabPanel,
        AccessibilityRole::Tooltip => Role::Tooltip,
        AccessibilityRole::Form => Role::Form,
    }
}

fn collect_displayed_ids(element: &Element, ids: &mut HashSet<ElementId>) {
    if element.is_display_none() {
        return;
    }
    ids.insert(element.runtime_id);
    for child in &element.children {
        collect_displayed_ids(child, ids);
    }
}

fn collect_visible_ids(element: &Element, ids: &mut HashSet<ElementId>) {
    if element.is_display_none() || element.is_visibility_hidden() {
        return;
    }
    ids.insert(element.runtime_id);
    for child in &element.children {
        collect_visible_ids(child, ids);
    }
}

fn collect_accessible_ids(element: &Element, ids: &mut HashSet<ElementId>) {
    if element.is_display_none() || element.is_visibility_hidden() || element.accessibility.hidden {
        return;
    }
    ids.insert(element.runtime_id);
    for child in &element.children {
        collect_accessible_ids(child, ids);
    }
}

fn accessibility_tree_contains(element: &Element, target: ElementId) -> bool {
    if element.is_display_none() || element.is_visibility_hidden() || element.accessibility.hidden {
        return false;
    }
    element.runtime_id == target
        || element
            .children
            .iter()
            .any(|child| accessibility_tree_contains(child, target))
}

fn mix_id(parent: u64, child: u64) -> u64 {
    let mut value = parent ^ child.wrapping_add(0x9e37_79b9_7f4a_7c15);
    value = (value ^ (value >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    value = (value ^ (value >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    value ^ (value >> 31)
}

fn collect_explicit_ids(element: &Element, ids: &mut HashSet<ElementId>) -> Result<(), UiError> {
    if let Some(id) = element.explicit_id
        && !ids.insert(id)
    {
        if id.value() == ACCESSIBILITY_ROOT_ID.0 {
            return Err(UiError::ReservedId(id));
        }
        return Err(UiError::DuplicateId(id));
    }
    for child in &element.children {
        collect_explicit_ids(child, ids)?;
    }
    Ok(())
}

fn validate_style_transition_count(element: &Element, count: &mut usize) -> Result<(), UiError> {
    if element.is_display_none() || element.is_visibility_hidden() {
        return Ok(());
    }
    if element.transition.is_some() {
        if *count == MAX_STYLE_TRANSITIONS_PER_WINDOW {
            return Err(UiError::TooManyStyleTransitions);
        }
        *count += 1;
    }
    for child in &element.children {
        validate_style_transition_count(child, count)?;
    }
    Ok(())
}

fn collect_style_transition_ids(element: &Element, ids: &mut HashSet<ElementId>) {
    if element.is_display_none() || element.is_visibility_hidden() {
        return;
    }
    if element.transition.is_some() {
        ids.insert(element.runtime_id);
    }
    for child in &element.children {
        collect_style_transition_ids(child, ids);
    }
}

fn sync_text_inputs(
    element: &Element,
    inputs: &mut HashMap<ElementId, TextInputState>,
    ids: &mut HashSet<ElementId>,
) {
    if element.is_display_none() {
        return;
    }
    if let ElementKind::TextInput(input) = &element.kind {
        ids.insert(element.runtime_id);
        inputs
            .entry(element.runtime_id)
            .and_modify(|state| {
                state.sync_external_styled(
                    &input.value,
                    input.multiline,
                    &input.constraints,
                    &input.highlights,
                )
            })
            .or_insert_with(|| {
                TextInputState::with_styling(
                    &input.value,
                    input.multiline,
                    input.constraints.clone(),
                    input.highlights.clone(),
                )
            });
    }
    for child in &element.children {
        sync_text_inputs(child, inputs, ids);
    }
}

fn selectable_text_content(element: &Element) -> Option<&Arc<str>> {
    if !element.resolved_user_select {
        return None;
    }
    match &element.kind {
        ElementKind::Text(content) => Some(content),
        ElementKind::StyledText(styled) => Some(styled.content()),
        _ => None,
    }
}

fn collect_selectable_texts(
    element: &Element,
    entries: &mut Vec<SelectableTextEntry>,
    indices: &mut HashMap<ElementId, usize>,
    previous: &mut HashMap<ElementId, SelectableTextEntry>,
) {
    if element.is_display_none() || element.is_visibility_hidden() {
        return;
    }
    if let Some(content) = selectable_text_content(element) {
        let document_index = entries.len();
        let character_lengths = previous
            .remove(&element.runtime_id)
            .filter(|entry| entry.content == *content)
            .map(|entry| entry.character_lengths)
            .unwrap_or_else(|| selectable_character_lengths(content).into());
        entries.push(SelectableTextEntry {
            id: element.runtime_id,
            content: content.clone(),
            character_lengths,
        });
        indices.insert(element.runtime_id, document_index);
    }
    for child in &element.children {
        collect_selectable_texts(child, entries, indices, previous);
    }
}

fn sync_static_text_selection(
    selection: &mut Option<StaticTextSelection>,
    entries: &[SelectableTextEntry],
    indices: &HashMap<ElementId, usize>,
) {
    let Some(current) = selection.as_mut() else {
        return;
    };
    let Some(anchor_index) = indices.get(&current.anchor.id).copied() else {
        *selection = None;
        return;
    };
    let Some(focus_index) = indices.get(&current.focus.id).copied() else {
        *selection = None;
        return;
    };
    current.anchor.offset = boundary_at_or_before(
        &entries[anchor_index].content,
        current
            .anchor
            .offset
            .min(entries[anchor_index].content.len()),
    );
    current.focus.offset = boundary_at_or_before(
        &entries[focus_index].content,
        current.focus.offset.min(entries[focus_index].content.len()),
    );
}

fn normalized_static_selection(
    selection: StaticTextSelection,
    indices: &HashMap<ElementId, usize>,
) -> Option<((usize, usize), (usize, usize))> {
    let anchor = (*indices.get(&selection.anchor.id)?, selection.anchor.offset);
    let focus = (*indices.get(&selection.focus.id)?, selection.focus.offset);
    Some(if anchor <= focus {
        (anchor, focus)
    } else {
        (focus, anchor)
    })
}

fn static_position_key(
    position: StaticTextPosition,
    indices: &HashMap<ElementId, usize>,
) -> Option<(usize, usize)> {
    Some((*indices.get(&position.id)?, position.offset))
}

fn static_selection_unit_range(
    position: StaticTextPosition,
    unit: StaticTextSelectionUnit,
    entries: &[SelectableTextEntry],
    indices: &HashMap<ElementId, usize>,
) -> Option<(StaticTextPosition, StaticTextPosition)> {
    let document_index = *indices.get(&position.id)?;
    let content = &entries.get(document_index)?.content;
    let range = match unit {
        StaticTextSelectionUnit::Character => position.offset..position.offset,
        StaticTextSelectionUnit::Word => word_range_at(content, position.offset),
        StaticTextSelectionUnit::Line => line_range_at(content, position.offset),
    };
    Some((
        StaticTextPosition {
            id: position.id,
            offset: range.start,
        },
        StaticTextPosition {
            id: position.id,
            offset: range.end,
        },
    ))
}

fn word_range_at(content: &str, offset: usize) -> std::ops::Range<usize> {
    if content.is_empty() {
        return 0..0;
    }
    let offset = boundary_at_or_before(content, offset.min(content.len()));
    let segments = content.split_word_bound_indices().collect::<Vec<_>>();
    let target = segments
        .iter()
        .position(|(start, segment)| {
            let end = *start + segment.len();
            offset < end || (offset == content.len() && end == content.len())
        })
        .unwrap_or(segments.len() - 1);
    let (mut start, segment) = segments[target];
    let mut end = start + segment.len();
    if segment_is_selectable_word(segment) {
        let mut index = target;
        while index > 0 {
            let (previous_start, previous) = segments[index - 1];
            if previous_start + previous.len() != start || !segment_is_selectable_word(previous) {
                break;
            }
            start = previous_start;
            index -= 1;
        }
        let mut index = target + 1;
        while let Some((next_start, next)) = segments.get(index).copied() {
            if next_start != end || !segment_is_selectable_word(next) {
                break;
            }
            end = next_start + next.len();
            index += 1;
        }
    }
    start..end
}

fn segment_is_selectable_word(segment: &str) -> bool {
    segment
        .chars()
        .any(|character| character.is_alphanumeric() || character == '_')
}

fn line_range_at(content: &str, offset: usize) -> std::ops::Range<usize> {
    let offset = boundary_at_or_before(content, offset.min(content.len()));
    let start = content[..offset].rfind('\n').map_or(0, |index| index + 1);
    let end = content[offset..]
        .find('\n')
        .map_or(content.len(), |index| offset + index + 1);
    start..end
}

fn static_selection_range_for_entry(
    selection: Option<StaticTextSelection>,
    indices: &HashMap<ElementId, usize>,
    document_index: usize,
    content_len: usize,
) -> Option<std::ops::Range<usize>> {
    let (start, end) = normalized_static_selection(selection?, indices)?;
    if document_index < start.0 || document_index > end.0 {
        return None;
    }
    let range = if start.0 == end.0 {
        start.1.min(content_len)..end.1.min(content_len)
    } else if document_index == start.0 {
        start.1.min(content_len)..content_len
    } else if document_index == end.0 {
        0..end.1.min(content_len)
    } else {
        0..content_len
    };
    (!range.is_empty()).then_some(range)
}

fn sync_virtual_scrolls(
    element: &Element,
    handles: &mut HashMap<ElementId, RetainedVirtualScroll>,
    offsets: &mut HashMap<ElementId, Vector>,
    scrollbar_states: &mut HashMap<ElementId, ScrollbarState>,
    now: Instant,
) {
    if element.is_display_none() {
        return;
    }
    if let Some(virtual_scroll) = &element.virtual_scroll {
        let max_offset = virtual_scroll
            .handle
            .max_offset(virtual_scroll.max_offset_y)
            .max(0.0);
        let next = virtual_scroll.handle.offset().clamp(0.0, max_offset);
        virtual_scroll.handle.set_offset_silent(next);
        let previous = offsets.insert(element.runtime_id, Vector::new(0.0, next));
        if previous.is_some_and(|previous| previous.y != next) {
            let state = scrollbar_states.entry(element.runtime_id).or_default();
            if !state.hovered && !state.dragging {
                state.visible_until = now.checked_add(SCROLLBAR_AUTO_HIDE_DELAY);
            }
        }
        handles.insert(
            element.runtime_id,
            RetainedVirtualScroll {
                handle: virtual_scroll.handle.clone(),
                measurement_revision: virtual_scroll.measurement_revision,
                mount: virtual_scroll.mount.clone(),
            },
        );
    }
    for child in &element.children {
        sync_virtual_scrolls(child, handles, offsets, scrollbar_states, now);
    }
}

fn sync_animations(
    element: &Element,
    animations: &mut HashMap<ElementId, AnimationPlayback>,
    ids: &mut HashSet<ElementId>,
    now: Instant,
) {
    if element.is_display_none() || element.is_visibility_hidden() {
        return;
    }
    if let ElementKind::Image(image) = &element.kind
        && let ImageResolution::Animated(asset) = &image.resolved
    {
        ids.insert(element.runtime_id);
        let replace = animations
            .get(&element.runtime_id)
            .is_none_or(|playback| playback.asset_id != asset.id());
        if replace {
            animations.insert(
                element.runtime_id,
                AnimationPlayback::new(asset.clone(), now),
            );
        }
    }
    for child in &element.children {
        sync_animations(child, animations, ids, now);
    }
}

#[allow(clippy::too_many_arguments)]
fn resolve_declarative_animations(
    element: &mut Element,
    playbacks: &mut HashMap<ElementId, DeclarativeAnimationPlayback>,
    motion_ids: &mut HashSet<ElementId>,
    time_animation_ids: &mut HashSet<ElementId>,
    springs: &mut HashMap<ElementId, DeclarativeSpringPlayback>,
    spring_ids: &mut HashSet<ElementId>,
    request_frame: &mut bool,
    deadline: &mut Option<Instant>,
    now: Instant,
    animation_epoch: Instant,
    enabled: bool,
    reduce_motion: bool,
    resolved_motion_ids: &mut Vec<ElementId>,
) -> Result<(), UiError> {
    if element.is_display_none() || element.is_visibility_hidden() {
        return Ok(());
    }
    if let Some(ElementAnimation {
        id,
        stages,
        animator,
    }) = element.animation.take()
    {
        if motion_ids.contains(&id) {
            return Err(UiError::DuplicateAnimationId(id));
        }
        if motion_ids.len() == MAX_DECLARATIVE_ANIMATIONS_PER_WINDOW {
            return Err(UiError::TooManyDeclarativeAnimations);
        }
        motion_ids.insert(id);
        time_animation_ids.insert(id);
        resolved_motion_ids.push(id);
        let sample = playbacks
            .entry(id)
            .or_insert_with(|| DeclarativeAnimationPlayback::new(now))
            .sample(&stages, now, animation_epoch, enabled, reduce_motion);
        *request_frame |= sample.request_frame;
        if let Some(next) = sample.deadline {
            *deadline = Some(deadline.map_or(next, |current| current.min(next)));
        }

        let base = std::mem::replace(element, crate::div());
        let mut resolved = animator(base, sample.animation_ix, sample.value);
        // The wrapper is declaration metadata, not part of the retained paint tree. An animator
        // may still create independently animated descendants, which are resolved below.
        resolved.animation = None;
        *element = resolved;
    }

    if let Some(spring) = element.spring.take() {
        let id = spring.id;
        if motion_ids.contains(&id) {
            return Err(UiError::DuplicateAnimationId(id));
        }
        if motion_ids.len() == MAX_DECLARATIVE_ANIMATIONS_PER_WINDOW {
            return Err(UiError::TooManyDeclarativeAnimations);
        }
        motion_ids.insert(id);
        spring_ids.insert(id);
        resolved_motion_ids.push(id);
        let sample = springs
            .entry(id)
            .or_insert_with(|| DeclarativeSpringPlayback::new(&spring, now))
            .sample(&spring, now, enabled, reduce_motion);
        *request_frame |= sample.request_frame;

        let base = std::mem::replace(element, crate::div());
        let mut resolved = (spring.animator)(base, sample.value);
        resolved.spring = None;
        *element = resolved;
    }

    for child in &mut element.children {
        resolve_declarative_animations(
            child,
            playbacks,
            motion_ids,
            time_animation_ids,
            springs,
            spring_ids,
            request_frame,
            deadline,
            now,
            animation_epoch,
            enabled,
            reduce_motion,
            resolved_motion_ids,
        )?;
    }
    Ok(())
}

fn sync_accessibility_text_ids(
    input_ids: &HashSet<ElementId>,
    element_ids: &HashSet<ElementId>,
    text_ids: &mut HashMap<ElementId, AccessibilityNodeId>,
    next_id: &mut u64,
) {
    text_ids.retain(|input, text_id| {
        input_ids.contains(input) && !element_ids.contains(&ElementId::new(text_id.0))
    });
    for input_id in input_ids {
        if text_ids.contains_key(input_id) {
            continue;
        }
        loop {
            let candidate = AccessibilityNodeId(*next_id);
            *next_id = next_id.wrapping_sub(1);
            if candidate != ACCESSIBILITY_ROOT_ID
                && !element_ids.contains(&ElementId::new(candidate.0))
                && !text_ids.values().any(|existing| *existing == candidate)
            {
                text_ids.insert(*input_id, candidate);
                break;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::element::InputConstraints;
    use crate::{
        AnimatedImageFrame, Animation, AnimationExt, AnimationRepeat, FocusHandle, Image,
        PathBuilder, SpringAnimation, Transition, button, container_query, div, form, img, text,
        text_input,
    };

    struct TestTextLayout;

    impl TextLayoutEngine for TestTextLayout {
        fn measure_text(
            &mut self,
            _id: TextId,
            content: &Arc<str>,
            style: &TextStyle,
            max_width: Option<f32>,
            _scale_factor: f32,
        ) -> Size {
            let natural_width = content.chars().count() as f32 * style.font_size * 0.5;
            Size::new(
                max_width.map_or(natural_width, |width| natural_width.min(width)),
                style.line_height,
            )
        }

        fn measure_styled_text(
            &mut self,
            id: TextId,
            content: &Arc<str>,
            style: &TextStyle,
            _highlights: &Arc<[TextHighlight]>,
            max_width: Option<f32>,
            scale_factor: f32,
        ) -> Size {
            self.measure_text(id, content, style, max_width, scale_factor)
        }

        fn text_geometry(
            &mut self,
            _id: TextId,
            _content: &Arc<str>,
            _style: &TextStyle,
            _highlights: Option<&Arc<[TextHighlight]>>,
            _width: f32,
            _scale_factor: f32,
            _visible_y: std::ops::Range<f32>,
        ) -> crate::renderer::StyledTextGeometry {
            crate::renderer::StyledTextGeometry {
                backgrounds: Vec::new(),
                decorations: Vec::new(),
            }
        }

        fn text_caret_position_with_highlights(
            &mut self,
            _id: TextId,
            _content: &Arc<str>,
            _style: &TextStyle,
            _highlights: Option<&Arc<[TextHighlight]>>,
            _width: f32,
            _scale_factor: f32,
            _index: usize,
        ) -> Point {
            Point::ZERO
        }

        fn text_index_for_point_with_highlights(
            &mut self,
            _id: TextId,
            _content: &Arc<str>,
            _style: &TextStyle,
            _highlights: Option<&Arc<[TextHighlight]>>,
            _width: f32,
            _scale_factor: f32,
            _point: Point,
        ) -> usize {
            0
        }

        fn text_selection_rects_with_highlights(
            &mut self,
            _id: TextId,
            _content: &Arc<str>,
            _style: &TextStyle,
            _highlights: Option<&Arc<[TextHighlight]>>,
            _width: f32,
            _scale_factor: f32,
            _visible_y: std::ops::Range<f32>,
            _start: usize,
            _end: usize,
        ) -> Vec<Rect> {
            Vec::new()
        }
    }

    fn playback_animation(repeat: AnimationRepeat) -> AnimatedImage {
        AnimatedImage::with_repeat(
            [
                AnimatedImageFrame::new(
                    Image::from_rgba(1, 1, vec![1, 0, 0, 255]).unwrap(),
                    Duration::from_millis(40),
                ),
                AnimatedImageFrame::new(
                    Image::from_rgba(1, 1, vec![2, 0, 0, 255]).unwrap(),
                    Duration::from_millis(60),
                ),
            ],
            repeat,
        )
        .unwrap()
    }

    #[test]
    fn generated_ids_are_path_stable() {
        assert_eq!(mix_id(42, 3), mix_id(42, 3));
        assert_ne!(mix_id(42, 3), mix_id(42, 4));
    }

    #[test]
    fn container_query_receives_its_fixed_box_and_isolates_child_layout() {
        let observed = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
        let callback_observed = observed.clone();
        let root = div().size(500.0, 300.0).child(
            container_query(move |size| {
                callback_observed.borrow_mut().push(size);
                div().id("oversized-query-child").size(900.0, 700.0)
            })
            .size(240.0, 120.0)
            .flex_none(),
        );
        let mut tree = UiTree::new();
        let mut renderer = TestTextLayout;

        tree.set_root(root, Size::new(500.0, 300.0), 1.0, &mut renderer)
            .unwrap();

        assert_eq!(observed.borrow().as_slice(), &[Size::new(240.0, 120.0)]);
        let query = &tree.root.as_ref().unwrap().children[0];
        let query_layout = tree.taffy.layout(query.taffy_node.unwrap()).unwrap();
        let child = &query.children[0];
        let child_layout = tree.taffy.layout(child.taffy_node.unwrap()).unwrap();
        assert_eq!(query_layout.size.width, 240.0);
        assert_eq!(query_layout.size.height, 120.0);
        assert_eq!(child_layout.size.width, 900.0);
        assert_eq!(child_layout.size.height, 700.0);

        let mut scene = Scene::new();
        tree.paint_at(&mut scene, &mut renderer, Instant::now())
            .unwrap();
        assert_eq!(
            tree.element_bounds("oversized-query-child".into()),
            Some(Rect::new(0.0, 0.0, 900.0, 700.0))
        );
    }

    #[test]
    fn container_query_redeclares_only_when_its_assigned_size_changes() {
        let calls = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
        let callback_calls = calls.clone();
        let root = container_query(move |size| {
            callback_calls.borrow_mut().push(size);
            if size.width < 300.0 {
                div().id("narrow-query-result")
            } else {
                div().id("wide-query-result")
            }
        });
        let mut tree = UiTree::new();
        let mut renderer = TestTextLayout;

        tree.set_root(root, Size::new(240.0, 160.0), 1.0, &mut renderer)
            .unwrap();
        assert!(tree.contains_element("narrow-query-result".into()));
        assert!(!tree.contains_element("wide-query-result".into()));

        tree.layout(Size::new(520.0, 160.0), 1.0, &mut renderer)
            .unwrap();
        assert!(!tree.contains_element("narrow-query-result".into()));
        assert!(tree.contains_element("wide-query-result".into()));

        tree.layout(Size::new(520.0, 160.0), 1.0, &mut renderer)
            .unwrap();
        assert_eq!(
            calls.borrow().as_slice(),
            &[Size::new(240.0, 160.0), Size::new(520.0, 160.0)]
        );
    }

    #[test]
    fn container_query_prepares_callback_subtrees_before_layout() {
        let prepared = std::rc::Rc::new(std::cell::Cell::new(0));
        let mut tree = UiTree::new();
        let mut renderer = TestTextLayout;
        let prepared_for_hook = prepared.clone();

        tree.set_root_with_prepare(
            container_query(|_| div().id("prepared-query-result")),
            Size::new(320.0, 200.0),
            1.0,
            &mut renderer,
            move |subtree| {
                assert_eq!(subtree.explicit_id, Some("prepared-query-result".into()));
                prepared_for_hook.set(prepared_for_hook.get() + 1);
            },
        )
        .unwrap();

        assert_eq!(prepared.get(), 1);
    }

    #[test]
    fn container_query_descendants_participate_in_initial_focus_resolution() {
        let mut tree = UiTree::new();
        let mut renderer = TestTextLayout;

        tree.set_root(
            container_query(|_| {
                crate::text_input("")
                    .id("query-autofocus-input")
                    .auto_focus()
            }),
            Size::new(320.0, 200.0),
            1.0,
            &mut renderer,
        )
        .unwrap();

        assert_eq!(tree.focused(), Some("query-autofocus-input".into()));
    }

    #[test]
    fn container_query_shells_do_not_unmount_stable_retained_state() {
        let started = Instant::now();
        let animated = playback_animation(AnimationRepeat::Infinite);
        let declaration = || {
            let animated = animated.clone();
            container_query(move |_| {
                div().children([
                    div()
                        .id("query-scroll-state")
                        .size(100.0, 100.0)
                        .overflow_y_scroll(),
                    img(animated.clone()).id("query-animated-image"),
                    div()
                        .id("query-transition-state")
                        .transition(Transition::colors(Duration::from_millis(100))),
                ])
            })
        };
        let mut tree = UiTree::new_at(started);
        let mut renderer = TestTextLayout;
        tree.set_root(declaration(), Size::new(360.0, 240.0), 1.0, &mut renderer)
            .unwrap();

        let scroll_id: ElementId = "query-scroll-state".into();
        let image_id: ElementId = "query-animated-image".into();
        let transition_id: ElementId = "query-transition-state".into();
        tree.scroll_offsets
            .insert(scroll_id, Vector::new(0.0, 37.0));
        tree.animations.get_mut(&image_id).unwrap().elapsed = Duration::from_millis(23);
        let transition = Transition::colors(Duration::from_millis(100));
        tree.style_transitions.insert(
            transition_id,
            StyleTransitionPlayback::new(
                transition_test_style(Color::BLACK, 0.0),
                &transition,
                started,
            ),
        );

        tree.set_root(declaration(), Size::new(360.0, 240.0), 1.0, &mut renderer)
            .unwrap();

        assert_eq!(tree.scroll_offsets[&scroll_id].y, 37.0);
        assert_eq!(
            tree.animations[&image_id].elapsed,
            Duration::from_millis(23)
        );
        assert!(tree.style_transitions.contains_key(&transition_id));
    }

    #[test]
    fn keyed_query_motion_survives_a_size_redeclaration_and_unmounts_cleanly() {
        let start = Instant::now();
        let root = container_query(|size| {
            if size.width < 500.0 {
                div().with_animation(
                    "query-motion",
                    Animation::new(Duration::from_millis(100)),
                    |element, phase| element.w(100.0 * phase),
                )
            } else {
                div()
            }
        });
        let mut tree = UiTree::new_at(start);
        let mut renderer = TestTextLayout;
        tree.set_root_unlaid(root, Size::new(240.0, 160.0), 1.0, start, false)
            .unwrap();
        tree.layout_with_prepare_at(
            Size::new(240.0, 160.0),
            1.0,
            &mut renderer,
            start,
            &mut |_| {},
        )
        .unwrap();

        let motion_id: ElementId = "query-motion".into();
        assert_eq!(tree.declarative_animations[&motion_id].last_value, 0.0);
        tree.layout_with_prepare_at(
            Size::new(360.0, 160.0),
            1.0,
            &mut renderer,
            start + Duration::from_millis(50),
            &mut |_| {},
        )
        .unwrap();
        assert!(
            (tree.declarative_animations[&motion_id].last_value - 0.5).abs() < 0.001,
            "the stable animation ID should retain elapsed time across callback replacement"
        );

        tree.layout_with_prepare_at(
            Size::new(600.0, 160.0),
            1.0,
            &mut renderer,
            start + Duration::from_millis(75),
            &mut |_| {},
        )
        .unwrap();
        assert!(!tree.declarative_animations.contains_key(&motion_id));
        assert!(!tree.declarative_animation_ids.contains(&motion_id));
    }

    #[test]
    fn detached_container_queries_are_pointer_passive_and_resize_on_demand() {
        let start = Instant::now();
        let observed = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
        let callback_observed = observed.clone();
        let template = container_query(move |size| {
            callback_observed.borrow_mut().push(size);
            div().clickable().cursor_pointer().with_animation(
                "detached-query-motion",
                Animation::new(Duration::from_millis(100)),
                |element, phase| element.w(10.0 + phase),
            )
        });
        let mut renderer = TestTextLayout;
        let mut tree = DetachedTree::new(
            template,
            ElementId::new(0xfeed),
            Size::new(240.0, 160.0),
            1.0,
            &mut renderer,
            start,
            start,
            true,
            false,
        )
        .unwrap();

        let child = &tree.root.children[0];
        assert!(!child.clickable);
        assert_eq!(child.cursor_style, None);
        assert!(child.animation.is_none());
        assert!(
            tree.motion
                .animations
                .contains_key(&ElementId::from("detached-query-motion"))
        );

        tree.layout(Size::new(420.0, 160.0), 1.0, &mut renderer)
            .unwrap();
        assert!(tree.motion.needs_resolve);
        assert!(
            tree.refresh(
                Size::new(420.0, 160.0),
                1.0,
                &mut renderer,
                start + Duration::from_millis(10),
                true,
                false,
            )
            .unwrap()
        );
        assert_eq!(
            observed.borrow().as_slice(),
            &[Size::new(240.0, 160.0), Size::new(420.0, 160.0)]
        );
    }

    #[test]
    fn container_query_depth_is_hard_bounded_before_nested_layout_allocation() {
        fn nested_query(depth: usize) -> Element {
            if depth == 0 {
                div()
            } else {
                container_query(move |_| nested_query(depth - 1))
            }
        }

        let mut tree = UiTree::new();
        let mut renderer = TestTextLayout;
        let result = tree.set_root(
            nested_query(MAX_CONTAINER_QUERY_DEPTH + 1),
            Size::new(320.0, 200.0),
            1.0,
            &mut renderer,
        );

        assert!(matches!(result, Err(UiError::ContainerQueryDepthExceeded)));
    }

    #[test]
    fn container_query_count_is_hard_bounded_before_layout() {
        let root = div()
            .children((0..=MAX_CONTAINER_QUERIES_PER_WINDOW).map(|_| container_query(|_| div())));
        let mut tree = UiTree::new();
        let result = tree.set_root_for_test(root, Size::new(640.0, 480.0), 1.0, Instant::now());

        assert!(matches!(result, Err(UiError::TooManyContainerQueries)));
        assert!(tree.root_node.is_none());
    }

    #[test]
    fn declarative_animation_chains_carry_elapsed_time_across_stages() {
        let start = Instant::now();
        let stages = [
            Animation::new(Duration::from_millis(100)),
            Animation::new(Duration::from_millis(200)),
        ];
        let mut playback = DeclarativeAnimationPlayback::new(start);

        let first = playback.sample(&stages, start, start, true, false);
        assert_eq!(first.animation_ix, 0);
        assert_eq!(first.value, 0.0);

        let second = playback.sample(
            &stages,
            start + Duration::from_millis(150),
            start,
            true,
            false,
        );
        assert_eq!(second.animation_ix, 1);
        assert!((second.value - 0.25).abs() < 0.001);

        let final_sample = playback.sample(
            &stages,
            start + Duration::from_millis(300),
            start,
            true,
            false,
        );
        assert_eq!(final_sample.animation_ix, 1);
        assert_eq!(final_sample.value, 1.0);
        assert!(!final_sample.request_frame);
        assert_eq!(final_sample.deadline, None);
    }

    #[test]
    fn duplicate_declarative_motion_ids_fail_before_layout() {
        let animation = || Animation::new(Duration::from_millis(100));
        let root = div().children([
            div().with_animation("shared-motion", animation(), |element, _| element),
            div().with_animation("shared-motion", animation(), |element, _| element),
        ]);
        let mut tree = UiTree::new();
        let result = tree.set_root_for_test(root, Size::new(640.0, 480.0), 1.0, Instant::now());

        assert!(matches!(result, Err(UiError::DuplicateAnimationId(_))));
    }

    #[test]
    fn style_transition_registry_is_hard_bounded_before_layout() {
        let root = div().children(
            (0..=MAX_STYLE_TRANSITIONS_PER_WINDOW)
                .map(|_| div().transition(Transition::colors(Duration::from_millis(100)))),
        );
        let mut tree = UiTree::new();
        let result = tree.set_root_for_test(root, Size::new(640.0, 480.0), 1.0, Instant::now());

        assert!(matches!(result, Err(UiError::TooManyStyleTransitions)));
        assert!(tree.root_node.is_none());
        assert!(tree.style_transitions.is_empty());
    }

    fn transition_test_style(background: Color, radius: f32) -> TransitionPaintStyle {
        TransitionPaintStyle {
            background,
            border_color: Color::TRANSPARENT,
            border_width: 0.0,
            radius,
            opacity: 1.0,
            text_color: None,
            text_fallback: Color::WHITE,
            shadows: TransitionShadowList::empty(),
        }
    }

    #[test]
    fn style_transitions_retarget_continuously_and_apply_unselected_properties_immediately() {
        let start = Instant::now();
        let from = transition_test_style(Color::BLACK, 0.0);
        let to = transition_test_style(Color::WHITE, 20.0);
        let config = Transition::colors(Duration::from_millis(100));
        let mut playback = StyleTransitionPlayback::new(from, &config, start);

        let initial = playback.sample(to, &config, start, true, false);
        assert_eq!(initial.background, Color::BLACK);
        assert_eq!(initial.radius, 20.0);
        assert!(playback.requests_frame());

        let midpoint = playback.sample(to, &config, start + Duration::from_millis(50), true, false);
        assert!((midpoint.background.r - 0.5).abs() < 0.001);
        assert_eq!(midpoint.radius, 20.0);

        let reversed = playback.sample(
            from,
            &config,
            start + Duration::from_millis(50),
            true,
            false,
        );
        assert_eq!(reversed.background, midpoint.background);
        assert_eq!(reversed.radius, 0.0);

        let reduced = playback.sample(to, &config, start + Duration::from_millis(60), true, true);
        assert_eq!(reduced, to);
        assert!(!playback.active);
        assert_eq!(playback.deadline(), None);
    }

    #[test]
    fn style_transitions_interpolate_opacity_without_including_it_in_colors() {
        let start = Instant::now();
        let mut from = transition_test_style(Color::BLACK, 0.0);
        from.opacity = 0.0;
        let mut to = transition_test_style(Color::BLACK, 0.0);
        to.opacity = 1.0;

        let all = Transition::new(Duration::from_millis(100));
        let mut playback = StyleTransitionPlayback::new(from, &all, start);
        playback.sample(to, &all, start, true, false);
        let midpoint = playback.sample(to, &all, start + Duration::from_millis(50), true, false);
        assert!((midpoint.opacity - 0.5).abs() < 0.001);

        let colors = Transition::colors(Duration::from_millis(100));
        let mut playback = StyleTransitionPlayback::new(from, &colors, start);
        assert_eq!(
            playback.sample(to, &colors, start, true, false).opacity,
            1.0
        );
        assert!(!playback.requests_frame());
    }

    #[test]
    fn throttled_style_transitions_wake_at_completion_even_below_one_fps() {
        let start = Instant::now();
        let from = transition_test_style(Color::BLACK, 0.0);
        let to = transition_test_style(Color::WHITE, 0.0);
        let config = Transition::colors(Duration::from_millis(100)).with_max_fps(0.5);
        let mut playback = StyleTransitionPlayback::new(from, &config, start);

        playback.sample(to, &config, start, true, false);
        assert_eq!(
            playback.deadline(),
            Some(start + Duration::from_millis(100))
        );
        assert!(!playback.requests_frame());
        assert!(!playback.deadline_due(start + Duration::from_millis(99)));
        assert!(playback.deadline_due(start + Duration::from_millis(100)));

        let final_style =
            playback.sample(to, &config, start + Duration::from_millis(100), true, false);
        assert_eq!(final_style, to);
        assert!(!playback.active);
        assert_eq!(playback.deadline(), None);
    }

    #[test]
    fn paused_style_transitions_preserve_time_and_shadow_interpolation_is_bounded() {
        let start = Instant::now();
        let mut from = transition_test_style(Color::BLACK, 0.0);
        let mut to = transition_test_style(Color::WHITE, 0.0);
        to.shadows = TransitionShadowList::from_slice(&[BoxShadow::new(10.0, 20.0, Color::WHITE)
            .blur_radius(30.0)
            .spread_radius(4.0)]);
        from.shadows = TransitionShadowList::empty();
        let config = Transition::new(Duration::from_millis(100));
        let mut playback = StyleTransitionPlayback::new(from, &config, start);
        playback.sample(to, &config, start, true, false);

        playback.pause(start + Duration::from_millis(25));
        let paused = playback.current;
        assert_eq!(paused.shadows.as_slice().len(), 1);
        assert!((paused.shadows.as_slice()[0].offset().x - 1.25).abs() < 0.001);
        playback.resume(start + Duration::from_secs(10));
        let resumed = playback.sample(to, &config, start + Duration::from_secs(10), true, false);
        assert_eq!(resumed, paused);

        let later = playback.sample(
            to,
            &config,
            start + Duration::from_secs(10) + Duration::from_millis(25),
            true,
            false,
        );
        assert!((later.background.r - 0.5).abs() < 0.001);
        assert!((later.shadows.as_slice()[0].offset().x - 5.0).abs() < 0.001);
    }

    #[test]
    fn capped_declarative_oneshots_always_schedule_their_terminal_frame() {
        let start = Instant::now();
        let stages = [Animation::new(Duration::from_millis(100)).with_max_fps(0.5)];
        let mut playback = DeclarativeAnimationPlayback::new(start);
        let sample = playback.sample(&stages, start, start, true, false);

        assert_eq!(sample.deadline, Some(start + Duration::from_millis(100)));
    }

    #[test]
    fn detached_motion_retains_exact_time_and_sanitizes_every_resolved_frame() {
        let start = Instant::now();
        let template = div()
            .clickable()
            .cursor_pointer()
            .hover(|style| style.cursor_crosshair())
            .with_animation(
                "detached-motion",
                Animation::new(Duration::from_millis(100)).with_max_fps(20.0),
                |element, phase| {
                    element
                        .w(10.0 + 90.0 * phase)
                        .clickable()
                        .cursor_copy()
                        .active(|style| style.cursor_grabbing())
                        .tooltip("animator-added interaction")
                },
            );
        let mut motion = DetachedMotionState::new(template, start);

        let initial = motion.resolve(start, true, false).unwrap();
        assert_eq!(absolute_length(initial.layout.size.width), Some(10.0));
        assert!(!initial.clickable);
        assert_eq!(initial.cursor_style, None);
        assert_eq!(initial.hover.cursor_style, None);
        assert_eq!(initial.active.cursor_style, None);
        assert!(initial.tooltip.is_none());
        assert!(initial.animation.is_none());
        assert!(!motion.frame_requested);
        assert_eq!(motion.deadline, Some(start + Duration::from_millis(50)));

        let midpoint = start + Duration::from_millis(50);
        assert!(!motion.deadline_due(midpoint - Duration::from_millis(1)));
        assert!(motion.deadline_due(midpoint));
        let midpoint_root = motion.resolve(midpoint, true, false).unwrap();
        assert_eq!(absolute_length(midpoint_root.layout.size.width), Some(55.0));
        assert!(!midpoint_root.clickable);
        assert_eq!(midpoint_root.cursor_style, None);
        assert_eq!(midpoint_root.hover.cursor_style, None);
        assert_eq!(midpoint_root.active.cursor_style, None);
        assert!(midpoint_root.tooltip.is_none());

        motion.pause(midpoint);
        let resumed_at = start + Duration::from_secs(10);
        motion.resume(resumed_at);
        let resumed = motion.resolve(resumed_at, true, false).unwrap();
        assert_eq!(absolute_length(resumed.layout.size.width), Some(55.0));
        assert_eq!(
            motion.deadline,
            Some(resumed_at + Duration::from_millis(50))
        );

        let completed_at = resumed_at + Duration::from_millis(50);
        assert!(motion.deadline_due(completed_at));
        let completed = motion.resolve(completed_at, true, false).unwrap();
        assert_eq!(absolute_length(completed.layout.size.width), Some(100.0));
        assert!(!motion.frame_requested);
        assert_eq!(motion.deadline, None);
        assert_eq!(motion.active_count(), 0);
    }

    #[test]
    fn detached_motion_uses_a_local_duplicate_id_namespace() {
        let start = Instant::now();
        let animation = || Animation::new(Duration::from_millis(100));
        let template = div().children([
            div().with_animation("duplicate-detached", animation(), |element, _| element),
            div().with_animation("duplicate-detached", animation(), |element, _| element),
        ]);
        let mut motion = DetachedMotionState::new(template, start);

        assert!(matches!(
            motion.resolve(start, true, false),
            Err(UiError::DuplicateAnimationId(_))
        ));
    }

    #[test]
    fn detached_springs_keep_velocity_and_snap_under_reduce_motion() {
        let start = Instant::now();
        let template = div().with_spring(
            "detached-spring",
            SpringAnimation::new(SpringConfig::new(170.0, 14.0, 1.0))
                .to(100.0)
                .from(0.0),
            |element, value| element.w(value),
        );
        let mut motion = DetachedMotionState::new(template, start);

        let initial = motion.resolve(start, true, false).unwrap();
        assert_eq!(absolute_length(initial.layout.size.width), Some(0.0));
        assert!(motion.frame_requested);
        assert_eq!(motion.active_count(), 1);

        let advanced = motion
            .resolve(start + Duration::from_millis(50), true, false)
            .unwrap();
        let width = absolute_length(advanced.layout.size.width).unwrap();
        assert!(width > 0.0 && width < 100.0);
        let velocity = motion
            .springs
            .values()
            .next()
            .expect("detached spring playback")
            .state
            .velocity;
        assert!(velocity > 0.0);

        motion.mark_needs_resolve();
        motion.pause(start + Duration::from_millis(50));
        assert!(motion.needs_resolve);
        let reduced = motion
            .resolve(start + Duration::from_millis(50), false, true)
            .unwrap();
        assert_eq!(absolute_length(reduced.layout.size.width), Some(100.0));
        assert!(!motion.frame_requested);
        assert_eq!(motion.active_count(), 0);
    }

    #[test]
    fn box_shadows_keep_css_declaration_order_around_the_element_quad() {
        let first_drop = Color::rgb8(220, 38, 38);
        let first_inset = Color::rgb8(22, 163, 74);
        let second_drop = Color::rgb8(37, 99, 235);
        let second_inset = Color::rgb8(147, 51, 234);
        let shadows = [
            BoxShadow::new(0.0, 2.0, first_drop),
            BoxShadow::new(0.0, 1.0, first_inset).inset(true),
            BoxShadow::new(0.0, 4.0, second_drop),
            BoxShadow::new(0.0, 2.0, second_inset).inset(true),
        ];
        let bounds = Rect::new(10.0, 10.0, 80.0, 40.0);
        let clip = Rect::new(0.0, 0.0, 100.0, 100.0);
        let mut scene = Scene::new();
        push_element_shadows(
            &mut scene,
            PaintLayerKey::default(),
            bounds,
            8.0,
            clip,
            &shadows,
            false,
        );
        scene.push_quad_in(PaintLayerKey::default(), Quad::new(bounds, Color::WHITE));
        push_element_shadows(
            &mut scene,
            PaintLayerKey::default(),
            bounds,
            8.0,
            clip,
            &shadows,
            true,
        );

        let layer = &scene.paint_layers()[0];
        assert_eq!(
            layer.shapes(),
            &[
                crate::scene::ShapeRef::Shadow(0),
                crate::scene::ShapeRef::Shadow(1),
                crate::scene::ShapeRef::Quad(0),
                crate::scene::ShapeRef::Shadow(2),
                crate::scene::ShapeRef::Shadow(3),
            ]
        );
        let colors: Vec<_> = layer
            .shadows()
            .iter()
            .map(|shadow| shadow.style.color())
            .collect();
        assert_eq!(
            colors,
            vec![second_drop, first_drop, second_inset, first_inset]
        );
    }

    #[test]
    fn images_preserve_intrinsic_ratio_when_one_axis_is_known() {
        assert_eq!(
            measure_image(Some(200.0), None, Size::new(400.0, 100.0)),
            TaffySize {
                width: 200.0,
                height: 50.0,
            }
        );
        assert_eq!(
            measure_image(None, Some(75.0), Size::new(400.0, 100.0)),
            TaffySize {
                width: 300.0,
                height: 75.0,
            }
        );
    }

    #[test]
    fn path_fitting_accounts_for_nonzero_source_bounds_and_cover_cropping() {
        let mut builder = PathBuilder::fill();
        builder.move_to(Point::new(10.0, 20.0));
        builder.line_to(Point::new(110.0, 20.0));
        builder.line_to(Point::new(110.0, 70.0));
        builder.line_to(Point::new(10.0, 70.0));
        builder.close();
        let path = builder.build().unwrap();
        let bounds = Rect::new(0.0, 0.0, 100.0, 100.0);

        let (contain_scale, contain_translation) =
            fit_path(bounds, &path, ObjectFit::Contain).unwrap();
        assert_eq!(contain_scale, [1.0, 1.0]);
        assert_eq!(contain_translation, Vector::new(-10.0, 5.0));

        let (cover_scale, cover_translation) = fit_path(bounds, &path, ObjectFit::Cover).unwrap();
        assert_eq!(cover_scale, [2.0, 2.0]);
        assert_eq!(cover_translation, Vector::new(-70.0, -40.0));
    }

    #[test]
    fn nested_opacity_multiplies_subtrees_and_restores_for_following_siblings() {
        let root = div().size(40.0, 20.0).flex_row().children([
            div()
                .size(20.0, 20.0)
                .opacity(0.5)
                .child(div().size_full().opacity(0.5).bg(Color::WHITE)),
            div().size(20.0, 20.0).bg(Color::WHITE),
        ]);
        let mut tree = UiTree::new();
        let mut renderer = TestTextLayout;
        tree.set_root(root, Size::new(40.0, 20.0), 1.0, &mut renderer)
            .unwrap();

        let mut scene = Scene::new();
        tree.paint(&mut scene, &mut renderer).unwrap();
        assert_eq!(scene.quads().len(), 2);
        assert_eq!(scene.quads()[0].fill.a, 0.25);
        assert_eq!(scene.quads()[1].fill.a, 1.0);
        assert_eq!(scene.current_opacity(), 1.0);
    }

    #[test]
    fn zero_opacity_keeps_pointer_and_accessibility_semantics() {
        let control = ElementId::named("transparent-control");
        let root = button()
            .id(control)
            .size(80.0, 24.0)
            .opacity(0.0)
            .clickable()
            .child("Still interactive");
        let mut tree = UiTree::new();
        let mut renderer = TestTextLayout;
        tree.set_root(root, Size::new(100.0, 40.0), 1.0, &mut renderer)
            .unwrap();

        let mut scene = Scene::new();
        tree.paint(&mut scene, &mut renderer).unwrap();
        assert!(scene.text_runs().is_empty());
        assert!(tree.hit_regions.iter().any(|region| region.id == control));
        assert!(
            tree.accessibility_update("Opacity test")
                .nodes
                .iter()
                .any(|(id, _)| *id == accessibility_id(control))
        );
    }

    #[test]
    fn retained_scroll_accessibility_updates_only_the_container() {
        let scroll_id = ElementId::named("accessibility-scroll");
        let child_id = ElementId::named("accessibility-scroll-child");
        let root = div()
            .id(scroll_id)
            .size(100.0, 100.0)
            .overflow_y_scroll()
            .child(
                div()
                    .h(300.0)
                    .flex_none()
                    .child(button().id(child_id).size(80.0, 24.0).child("Open")),
            );
        let mut tree = UiTree::new();
        let mut renderer = TestTextLayout;
        tree.set_root(root, Size::new(100.0, 100.0), 1.0, &mut renderer)
            .unwrap();

        let mut scene = Scene::new();
        tree.paint(&mut scene, &mut renderer).unwrap();
        let initial_child_bounds = tree
            .accessibility_update("Scroll test")
            .nodes
            .into_iter()
            .find_map(|(id, node)| (id == accessibility_id(child_id)).then(|| node.bounds()))
            .flatten()
            .expect("accessible child bounds");

        tree.scroll_offsets
            .insert(scroll_id, Vector::new(0.0, 80.0));
        scene.clear(Color::TRANSPARENT);
        tree.paint(&mut scene, &mut renderer).unwrap();

        let update = tree.accessibility_scroll_update();
        assert_eq!(update.nodes.len(), 1);
        let (node_id, node) = &update.nodes[0];
        assert_eq!(*node_id, accessibility_id(scroll_id));
        assert_eq!(node.bounds().expect("scroll bounds").y0, 80.0);
        assert_eq!(
            node.transform(),
            Some(&Affine::translate(AccessibilityVector::new(0.0, -80.0)))
        );
        assert!(node.clips_children());

        let scrolled_child_bounds = tree
            .accessibility_update("Scroll test")
            .nodes
            .into_iter()
            .find_map(|(id, node)| (id == accessibility_id(child_id)).then(|| node.bounds()))
            .flatten()
            .expect("accessible child bounds after scrolling");
        assert_eq!(scrolled_child_bounds, initial_child_bounds);
    }

    #[test]
    fn clipped_offscreen_text_keeps_bounds_without_emitting_a_text_run() {
        let text_id = ElementId::named("offscreen-text");
        let root = div().relative().size(100.0, 100.0).overflow_hidden().child(
            text("outside")
                .id(text_id)
                .absolute()
                .top(160.0)
                .left(0.0)
                .size(100.0, 20.0),
        );
        let mut tree = UiTree::new();
        let mut renderer = TestTextLayout;
        tree.set_root(root, Size::new(100.0, 100.0), 1.0, &mut renderer)
            .unwrap();

        let mut scene = Scene::new();
        tree.paint(&mut scene, &mut renderer).unwrap();

        assert!(scene.text_runs().is_empty());
        assert_eq!(
            tree.element_bounds(text_id),
            Some(Rect::new(0.0, 160.0, 100.0, 20.0))
        );
        assert!(
            tree.accessibility_update("Cull test")
                .nodes
                .iter()
                .any(|(id, _)| *id == accessibility_id(text_id))
        );
    }

    #[test]
    fn animation_playback_uses_exact_deadlines_and_pauses_without_catching_up() {
        let started = Instant::now();
        let mut playback =
            AnimationPlayback::new(playback_animation(AnimationRepeat::Infinite), started);
        playback.activate(started, true);
        assert_eq!(
            playback.deadline(),
            Some(started + Duration::from_millis(40))
        );

        assert!(!playback.advance(started + Duration::from_millis(39)));
        assert_eq!(playback.frame_index, 0);
        assert!(playback.advance(started + Duration::from_millis(40)));
        assert_eq!(playback.frame_index, 1);
        assert_eq!(
            playback.deadline(),
            Some(started + Duration::from_millis(100))
        );

        playback.active = false;
        playback.activate(started + Duration::from_secs(10), true);
        assert_eq!(
            playback.deadline(),
            Some(started + Duration::from_secs(10) + Duration::from_millis(60))
        );

        playback.active = false;
        playback.activate(started + Duration::from_secs(20), false);
        assert!(!playback.active);
        assert_eq!(playback.deadline(), None);

        playback.activate(started + Duration::from_secs(30), true);
        playback.seen = false;
        playback.finish_visibility();
        assert!(!playback.active);
        assert_eq!(playback.deadline(), None);
    }

    #[test]
    fn finite_animation_playback_stops_scheduling_on_the_last_frame() {
        let started = Instant::now();
        let repeat = AnimationRepeat::Finite(1);
        let mut playback = AnimationPlayback::new(playback_animation(repeat), started);
        playback.activate(started, true);
        assert!(playback.advance(started + Duration::from_millis(100)));
        assert_eq!(playback.frame_index, 1);
        assert!(playback.completed);
        assert_eq!(playback.deadline(), None);
    }

    #[test]
    fn duplicate_explicit_ids_are_rejected_before_layout() {
        let mut seen = HashSet::new();
        let root = div()
            .child(text("one").id("same"))
            .child(text("two").id("same"));
        let error = collect_explicit_ids(&root, &mut seen).unwrap_err();
        assert!(matches!(error, UiError::DuplicateId(_)));
    }

    #[test]
    fn accessibility_root_id_is_reserved() {
        let mut seen = HashSet::from([ElementId::new(ACCESSIBILITY_ROOT_ID.0)]);
        let root = div().id(ElementId::new(ACCESSIBILITY_ROOT_ID.0));
        let error = collect_explicit_ids(&root, &mut seen).unwrap_err();
        assert!(matches!(error, UiError::ReservedId(_)));
    }

    #[test]
    fn display_none_deactivates_the_complete_mounted_subtree() {
        let animated = playback_animation(AnimationRepeat::Infinite);
        let declaration = |hidden: bool| {
            let subtree = div()
                .id("hidden-subtree")
                .w(120.0)
                .children([
                    button()
                        .id("hidden-button")
                        .clickable()
                        .tooltip("Hidden tooltip")
                        .auto_focus(),
                    text_input("").id("hidden-input"),
                    text("hidden selectable")
                        .id("hidden-text")
                        .user_select_text(),
                    crate::img(animated.clone()).id("hidden-image"),
                    div().id("hidden-motion").with_animation(
                        "hidden-motion-clock",
                        Animation::new(Duration::from_millis(100)).repeat(),
                        |element, value| element.w(value * 10.0),
                    ),
                    div()
                        .id("hidden-transition")
                        .transition(Transition::colors(Duration::from_millis(100))),
                ])
                .when(hidden, Element::hidden);
            div()
                .size(300.0, 100.0)
                .flex_row()
                .children([subtree, div().id("visible-sibling").size(24.0, 24.0)])
        };

        let mut tree = UiTree::new();
        let mut renderer = TestTextLayout;
        tree.set_root(
            declaration(false),
            Size::new(300.0, 100.0),
            1.0,
            &mut renderer,
        )
        .unwrap();

        let button = ElementId::named("hidden-button");
        let subtree = ElementId::named("hidden-subtree");
        assert_eq!(tree.focused(), Some(button));
        assert!(tree.clickable_ids.contains(&button));
        assert!(tree.text_inputs.contains_key(&"hidden-input".into()));
        assert!(
            tree.selectable_text_indices
                .contains_key(&"hidden-text".into())
        );
        assert!(tree.animations.contains_key(&"hidden-image".into()));
        assert!(
            tree.declarative_animation_ids
                .contains(&"hidden-motion-clock".into())
        );
        assert!(
            tree.style_transition_ids
                .contains(&"hidden-transition".into())
        );
        assert!(tree.tooltips.contains_key(&button));

        tree.hovered.insert(button);
        tree.pressed = Some(button);
        tree.dragging = Some(button);
        tree.drag_over = Some(button);
        tree.scroll_offsets.insert(subtree, Vector::new(0.0, 12.0));
        tree.scrollbar_states
            .insert(subtree, ScrollbarState::default());
        tree.hovered_scrollbar = Some(subtree);
        tree.scrollbar_drag = Some(ScrollbarDrag {
            id: subtree,
            pointer_origin_y: 0.0,
            scroll_origin_y: 0.0,
        });

        tree.set_root(
            declaration(true),
            Size::new(300.0, 100.0),
            1.0,
            &mut renderer,
        )
        .unwrap();

        // The declarations and their stable IDs still exist, but no hidden descendant remains
        // mounted in an interactive, semantic, resource, or animation registry.
        assert!(tree.contains_element(button));
        assert!(!tree.displayed_ids.contains(&subtree));
        assert!(!tree.displayed_ids.contains(&button));
        assert!(!tree.focusable_ids.contains(&button));
        assert!(!tree.clickable_ids.contains(&button));
        assert_eq!(tree.focused(), None);
        assert!(!tree.text_inputs.contains_key(&"hidden-input".into()));
        assert!(
            !tree
                .selectable_text_indices
                .contains_key(&"hidden-text".into())
        );
        assert!(!tree.animations.contains_key(&"hidden-image".into()));
        assert!(tree.declarative_animation_ids.is_empty());
        assert!(tree.declarative_animations.is_empty());
        assert!(tree.style_transition_ids.is_empty());
        assert!(tree.style_transitions.is_empty());
        assert!(!tree.tooltips.contains_key(&button));
        assert!(!tree.hovered.contains(&button));
        assert_eq!(tree.pressed, None);
        assert_eq!(tree.dragging, None);
        assert_eq!(tree.drag_over, None);
        assert!(!tree.scroll_offsets.contains_key(&subtree));
        assert!(!tree.scrollbar_states.contains_key(&subtree));
        assert_eq!(tree.hovered_scrollbar, None);
        assert!(tree.scrollbar_drag.is_none());

        let mut scene = Scene::new();
        tree.paint(&mut scene, &mut renderer).unwrap();
        assert_eq!(tree.element_bounds(subtree), None);
        assert_eq!(
            tree.element_bounds("visible-sibling".into()),
            Some(Rect::new(0.0, 0.0, 24.0, 24.0))
        );
        let update = tree.accessibility_update("Display none test");
        assert!(
            !update
                .nodes
                .iter()
                .any(|(id, _)| *id == accessibility_id(subtree))
        );
        assert!(
            !update
                .nodes
                .iter()
                .any(|(id, _)| *id == accessibility_id(button))
        );
        assert_eq!(tree.accessibility_element(accessibility_id(button)), None);
    }

    #[test]
    fn display_none_does_not_resolve_descendant_container_queries() {
        let calls = std::rc::Rc::new(std::cell::Cell::new(0));
        let callback_calls = calls.clone();
        let root = div().hidden().child(container_query(move |_| {
            callback_calls.set(callback_calls.get() + 1);
            div().id("hidden-query-result")
        }));
        let mut tree = UiTree::new();
        let mut renderer = TestTextLayout;

        tree.set_root(root, Size::new(320.0, 200.0), 1.0, &mut renderer)
            .unwrap();

        assert_eq!(calls.get(), 0);
        assert!(!tree.contains_element("hidden-query-result".into()));
        assert!(!tree.declarative_animation_frame_requested());
    }

    #[test]
    fn visibility_hidden_retains_layout_and_controlled_state_but_not_runtime_activity() {
        let calls = std::rc::Rc::new(std::cell::Cell::new(0));
        let callback_calls = calls.clone();
        let invisible = div()
            .id("invisible-subtree")
            .w(120.0)
            .invisible()
            .children([
                button()
                    .id("invisible-button")
                    .clickable()
                    .auto_focus()
                    .tooltip("Invisible tooltip"),
                text_input("controlled value").id("invisible-input"),
                container_query(move |_| {
                    callback_calls.set(callback_calls.get() + 1);
                    div().id("invisible-query-result")
                }),
                div().id("invisible-motion").with_animation(
                    "invisible-motion-clock",
                    Animation::new(Duration::from_millis(100)).repeat(),
                    |element, value| element.w(value * 10.0),
                ),
            ]);
        let root = div()
            .size(300.0, 100.0)
            .flex_row()
            .children([invisible, div().id("visibility-sibling").size(24.0, 24.0)]);
        let mut tree = UiTree::new();
        let mut renderer = TestTextLayout;

        tree.set_root(root, Size::new(300.0, 100.0), 1.0, &mut renderer)
            .unwrap();

        let subtree = ElementId::named("invisible-subtree");
        let button = ElementId::named("invisible-button");
        assert_eq!(
            calls.get(),
            1,
            "visibility must not suppress layout callbacks"
        );
        assert!(tree.contains_element("invisible-query-result".into()));
        assert!(tree.displayed_ids.contains(&subtree));
        assert!(!tree.visible_ids.contains(&subtree));
        assert!(!tree.visible_ids.contains(&button));
        assert!(!tree.focusable_ids.contains(&button));
        assert!(!tree.clickable_ids.contains(&button));
        assert_eq!(tree.focused(), None);
        assert!(
            tree.text_inputs.contains_key(&"invisible-input".into()),
            "layout-preserving visibility keeps controlled input state warm"
        );
        assert!(tree.tooltips.is_empty());
        assert!(tree.declarative_animation_ids.is_empty());
        assert!(tree.declarative_animations.is_empty());
        assert!(!tree.declarative_animation_frame_requested());

        let mut scene = Scene::new();
        tree.paint(&mut scene, &mut renderer).unwrap();
        assert_eq!(tree.element_bounds(subtree), None);
        assert_eq!(
            tree.element_bounds("visibility-sibling".into()),
            Some(Rect::new(120.0, 0.0, 24.0, 24.0))
        );
        let update = tree.accessibility_update("Visibility test");
        assert!(
            !update
                .nodes
                .iter()
                .any(|(id, _)| *id == accessibility_id(subtree))
        );
        assert_eq!(tree.accessibility_element(accessibility_id(button)), None);
    }

    #[test]
    fn tab_order_prioritizes_positive_indices_and_skips_disabled_controls() {
        let mut root = div()
            .child(button().id(10_u64).clickable().tab_index(2))
            .child(button().id(20_u64).clickable())
            .child(button().id(30_u64).clickable().tab_index(1))
            .child(button().id(40_u64).clickable().disabled(true));
        assign_runtime_ids(&mut root);

        let mut tree = UiTree::new();
        tree.root = Some(root);
        tree.rebuild_focus_index();
        assert_eq!(
            tree.focus_order,
            vec![ElementId::new(30), ElementId::new(10), ElementId::new(20)]
        );
        assert!(tree.focus_next(false));
        assert_eq!(tree.focused(), Some(ElementId::new(30)));
        assert!(tree.focus_next(true));
        assert_eq!(tree.focused(), Some(ElementId::new(20)));
        assert!(!tree.focusable_ids.contains(&ElementId::new(40)));
    }

    #[test]
    fn topmost_focus_trap_contains_focus_and_falls_back_to_the_remaining_layer() {
        let mut root = div()
            .child(button().id(1_u64))
            .child(
                div()
                    .id(10_u64)
                    .overlay()
                    .z_index(1)
                    .focus_trap()
                    .child(button().id(11_u64)),
            )
            .child(
                div()
                    .id(20_u64)
                    .overlay()
                    .z_index(2)
                    .focus_trap()
                    .children([button().id(21_u64), button().id(22_u64)]),
            );
        assign_runtime_ids(&mut root);

        let mut tree = UiTree::new();
        tree.root = Some(root);
        tree.rebuild_focus_index();
        assert_eq!(tree.active_focus_trap, Some(ElementId::new(20)));
        assert_eq!(
            tree.focus_order,
            vec![ElementId::new(21), ElementId::new(22)]
        );
        assert_eq!(tree.focused(), Some(ElementId::new(21)));
        assert!(!tree.focus(ElementId::new(1)));
        assert!(!tree.focus(ElementId::new(11)));
        assert!(tree.focus_next(false));
        assert_eq!(tree.focused(), Some(ElementId::new(22)));
        assert!(tree.focus_next(false));
        assert_eq!(tree.focused(), Some(ElementId::new(21)));

        let mut root = div().child(button().id(1_u64)).child(
            div()
                .id(10_u64)
                .overlay()
                .z_index(1)
                .focus_trap()
                .child(button().id(11_u64)),
        );
        assign_runtime_ids(&mut root);
        tree.root = Some(root);
        tree.rebuild_focus_index();
        assert_eq!(tree.active_focus_trap, Some(ElementId::new(10)));
        assert_eq!(tree.focus_order, vec![ElementId::new(11)]);
        assert_eq!(tree.focused(), Some(ElementId::new(11)));
    }

    #[test]
    fn focused_path_retains_scopes_and_contexts_without_making_them_tab_stops() {
        let mut root = div()
            .focus_scope(FocusHandle::new(10_u64))
            .key_context("Workspace")
            .child(
                div()
                    .focus_scope(FocusHandle::new(20_u64))
                    .key_context("Pane active=true")
                    .child(
                        div()
                            .track_focus(FocusHandle::new(30_u64))
                            .key_context("Editor mode=insert"),
                    ),
            );
        assign_runtime_ids(&mut root);

        let mut tree = UiTree::new();
        tree.root = Some(root);
        tree.rebuild_focus_index();
        tree.rebuild_dispatch_index();
        assert!(tree.focus(ElementId::new(30)));
        assert_eq!(
            tree.focus_path(),
            vec![ElementId::new(10), ElementId::new(20), ElementId::new(30)]
        );
        assert!(tree.focus_path().contains(&ElementId::new(10)));
        assert!(!tree.focusable_ids.contains(&ElementId::new(10)));
        let contexts = tree.key_context_stack();
        assert_eq!(contexts.len(), 3);
        assert_eq!(contexts[1].value("active"), Some("true"));
        assert_eq!(contexts[2].value("mode"), Some("insert"));
    }

    #[test]
    fn controls_derive_accessible_names_from_text_children() {
        let control = button().child("Save changes");
        assert_eq!(
            accessibility_label(&control).as_deref(),
            Some("Save changes")
        );
    }

    #[test]
    fn controls_derive_accessible_names_from_styled_text_children() {
        let control = button().child(
            crate::styled_text("Save changes")
                .with_highlights([(0..4, crate::HighlightStyle::default().font_bold())]),
        );
        assert_eq!(
            accessibility_label(&control).as_deref(),
            Some("Save changes")
        );
    }

    #[test]
    fn accessibility_hidden_suppresses_only_the_native_semantic_subtree() {
        let input_id = ElementId::new(31);
        let visual_list_id = ElementId::new(32);
        let visual_option_id = ElementId::new(33);
        let proxy_list_id = ElementId::new(34);
        let proxy_option_id = ElementId::new(35);
        let dangling_control_id = ElementId::new(36);
        let root = div().children([
            text_input("ap")
                .id(input_id)
                .accessibility_role(AccessibilityRole::EditableComboBox)
                .accessibility_controls(proxy_list_id)
                .accessibility_active_descendant(proxy_option_id),
            div()
                .id(visual_list_id)
                .size(120.0, 48.0)
                .accessibility_hidden(true)
                .child(
                    button()
                        .id(visual_option_id)
                        .size(120.0, 24.0)
                        .clickable()
                        .child("Visual Apple"),
                ),
            div()
                .id(proxy_list_id)
                .size(0.0, 0.0)
                .accessibility_role(AccessibilityRole::ListBox)
                .child(
                    div()
                        .id(proxy_option_id)
                        .accessibility_role(AccessibilityRole::ListBoxOption)
                        .child("Apple"),
                ),
            button()
                .id(dangling_control_id)
                .accessibility_controls(visual_option_id)
                .child("Hidden relation target"),
        ]);
        let mut tree = UiTree::new();
        let mut renderer = TestTextLayout;
        tree.set_root(root, Size::new(320.0, 200.0), 1.0, &mut renderer)
            .unwrap();
        let mut scene = Scene::new();
        tree.paint(&mut scene, &mut renderer).unwrap();

        assert!(tree.clickable_ids.contains(&visual_option_id));
        assert!(tree.element_bounds(visual_option_id).is_some());

        let update = tree.accessibility_update("Accessibility portal");
        let node = |id| {
            update
                .nodes
                .iter()
                .find_map(|(node_id, node)| (*node_id == accessibility_id(id)).then_some(node))
        };
        assert!(node(visual_list_id).is_none());
        assert!(node(visual_option_id).is_none());
        assert_eq!(
            tree.accessibility_element(accessibility_id(visual_option_id)),
            None
        );
        assert_eq!(
            node(input_id).expect("owner input").controls(),
            &[accessibility_id(proxy_list_id)]
        );
        assert_eq!(
            node(input_id).expect("owner input").active_descendant(),
            Some(accessibility_id(proxy_option_id))
        );
        assert!(node(proxy_list_id).is_some());
        assert!(node(proxy_option_id).is_some());
        assert!(
            node(dangling_control_id)
                .expect("visible control")
                .controls()
                .is_empty()
        );
    }

    #[test]
    fn accessibility_hidden_text_does_not_leak_into_a_parent_name() {
        let control = button()
            .child("Visible label")
            .child(div().accessibility_hidden(true).child("Private duplicate"));
        assert_eq!(
            accessibility_label(&control).as_deref(),
            Some("Visible label")
        );
    }

    #[test]
    fn selection_controls_project_exact_roles_toggle_states_and_actions() {
        let checkbox_id = ElementId::new(41);
        let radio_id = ElementId::new(42);
        let switch_id = ElementId::new(43);
        let disabled_id = ElementId::new(44);
        let checkbox = crate::Checkbox::new(ToggleState::Mixed);
        let radio = crate::Radio::new(true);
        let switch = crate::Switch::new(false);
        let root = div().children([
            checkbox
                .root_part(
                    div()
                        .child(checkbox.indicator_part(div().child("decorative mixed mark")))
                        .child("Partial selection"),
                )
                .id(checkbox_id)
                .accessibility_description("Some files are selected"),
            radio
                .root_part(
                    div()
                        .child(radio.indicator_part(div().child("decorative radio dot")))
                        .child("Selected radio"),
                )
                .id(radio_id),
            switch
                .root_part(
                    div()
                        .child(switch.thumb_part(div().child("decorative switch thumb")))
                        .child("Inactive switch"),
                )
                .id(switch_id),
            crate::checkbox(false)
                .id(disabled_id)
                .disabled(true)
                .child("Unavailable"),
        ]);
        let mut tree = UiTree::new();
        let mut renderer = TestTextLayout;
        tree.set_root(root, Size::new(320.0, 200.0), 1.0, &mut renderer)
            .unwrap();
        let mut scene = Scene::new();
        tree.paint(&mut scene, &mut renderer).unwrap();
        let update = tree.accessibility_update("Selection controls");
        let node = |id| {
            update
                .nodes
                .iter()
                .find_map(|(node_id, node)| (*node_id == accessibility_id(id)).then_some(node))
                .expect("selection accessibility node")
        };

        assert_eq!(node(checkbox_id).role(), Role::CheckBox);
        assert_eq!(
            node(checkbox_id).toggled(),
            Some(AccessibilityToggled::Mixed)
        );
        assert_eq!(node(checkbox_id).label(), Some("Partial selection"));
        assert_eq!(
            node(checkbox_id).description(),
            Some("Some files are selected")
        );
        assert!(node(checkbox_id).supports_action(Action::Click));
        assert_eq!(node(radio_id).role(), Role::RadioButton);
        assert_eq!(node(radio_id).toggled(), Some(AccessibilityToggled::True));
        assert_eq!(node(radio_id).label(), Some("Selected radio"));
        assert_eq!(node(switch_id).role(), Role::Switch);
        assert_eq!(node(switch_id).toggled(), Some(AccessibilityToggled::False));
        assert_eq!(node(switch_id).label(), Some("Inactive switch"));
        assert!(node(disabled_id).is_disabled());
        assert!(!node(disabled_id).supports_action(Action::Click));
        assert!(!node(disabled_id).supports_action(Action::Focus));
    }

    #[test]
    fn separators_and_mounted_group_labels_project_exact_native_semantics() {
        let group_id = ElementId::new(45);
        let label_id = ElementId::new(46);
        let separator_id = ElementId::new(47);
        let self_label_id = ElementId::new(48);
        let root = div().children([
            div()
                .id(group_id)
                .accessibility_role(AccessibilityRole::Group)
                .accessibility_labelled_by(label_id)
                .children([
                    div()
                        .id(label_id)
                        .accessibility_role(AccessibilityRole::Label)
                        .accessibility_label("File"),
                    div().child("Open"),
                ]),
            div()
                .id(separator_id)
                .accessibility_role(AccessibilityRole::Separator),
            div()
                .id(self_label_id)
                .accessibility_role(AccessibilityRole::Group)
                .accessibility_labelled_by(self_label_id),
        ]);
        let mut tree = UiTree::new();
        let mut renderer = TestTextLayout;
        tree.set_root(root, Size::new(320.0, 200.0), 1.0, &mut renderer)
            .unwrap();
        let mut scene = Scene::new();
        tree.paint(&mut scene, &mut renderer).unwrap();
        let update = tree.accessibility_update("Menu structure");
        let node = |id| {
            update
                .nodes
                .iter()
                .find_map(|(node_id, node)| (*node_id == accessibility_id(id)).then_some(node))
                .expect("menu structure accessibility node")
        };

        assert_eq!(node(label_id).role(), Role::Label);
        assert_eq!(node(group_id).labelled_by(), &[accessibility_id(label_id)]);
        assert_eq!(node(separator_id).role(), Role::Splitter);
        assert!(!node(separator_id).supports_action(Action::Click));
        assert!(!node(separator_id).supports_action(Action::Focus));
        assert!(node(self_label_id).labelled_by().is_empty());
    }

    #[test]
    fn popover_trigger_projects_expansion_popup_kind_and_mounted_control_relation() {
        let trigger_id = ElementId::new(51);
        let surface_id = ElementId::new(52);
        let popover =
            crate::Popover::new(trigger_id, surface_id, true).kind(crate::PopoverKind::Dialog);
        let root =
            div()
                .child(popover.trigger().child("Open details"))
                .child(
                    popover.positioner_part(div().child(popover.popup_part(div()).children([
                        popover.title_part(text("Details")),
                        popover.description_part(text("More information about this item.")),
                    ]))),
                );
        let mut tree = UiTree::new();
        let mut renderer = TestTextLayout;
        tree.set_root(root, Size::new(420.0, 260.0), 1.0, &mut renderer)
            .unwrap();
        let mut scene = Scene::new();
        tree.paint(&mut scene, &mut renderer).unwrap();
        let update = tree.accessibility_update("Popover");
        let node = |id| {
            update
                .nodes
                .iter()
                .find_map(|(node_id, node)| (*node_id == accessibility_id(id)).then_some(node))
                .expect("popover accessibility node")
        };

        let trigger = node(trigger_id);
        assert_eq!(trigger.role(), Role::Button);
        assert_eq!(trigger.is_expanded(), Some(true));
        assert_eq!(trigger.has_popup(), Some(AccessibilityHasPopup::Dialog));
        assert_eq!(trigger.controls(), &[accessibility_id(surface_id)]);
        let popup = node(surface_id);
        assert_eq!(popup.role(), Role::Dialog);
        assert!(popup.supports_action(Action::Focus));
        assert_eq!(popup.labelled_by(), &[accessibility_id(popover.title_id())]);
        assert_eq!(
            popup.described_by(),
            &[accessibility_id(popover.description_id())]
        );
    }

    #[test]
    fn alert_dialog_projects_modal_role_and_mounted_title_description_relations() {
        let dialog = crate::Dialog::alert("delete-dialog", true);
        let root = dialog.root_part(div()).children([
            dialog.backdrop_part(div()),
            dialog.popup_part(div()).children([
                dialog.title_part(text("Delete file?")),
                dialog.description_part(text("This cannot be undone.")),
            ]),
        ]);
        let mut tree = UiTree::new();
        let mut renderer = TestTextLayout;
        tree.set_root(root, Size::new(420.0, 260.0), 1.0, &mut renderer)
            .unwrap();
        let mut scene = Scene::new();
        tree.paint(&mut scene, &mut renderer).unwrap();
        let update = tree.accessibility_update("Alert dialog");
        let node = |id| {
            update
                .nodes
                .iter()
                .find_map(|(node_id, node)| (*node_id == accessibility_id(id)).then_some(node))
                .expect("dialog accessibility node")
        };

        let popup = node(dialog.popup_id());
        assert_eq!(popup.role(), Role::AlertDialog);
        assert!(popup.is_modal());
        assert_eq!(popup.labelled_by(), &[accessibility_id(dialog.title_id())]);
        assert_eq!(
            popup.described_by(),
            &[accessibility_id(dialog.description_id())]
        );
        assert_eq!(node(dialog.title_id()).label(), Some("Delete file?"));
        assert_eq!(
            node(dialog.description_id()).label(),
            Some("This cannot be undone.")
        );
    }

    #[test]
    fn comboboxes_project_editability_completion_options_and_active_descendants() {
        let editable_id = ElementId::new(81);
        let listbox_id = ElementId::new(82);
        let option_id = ElementId::new(83);
        let select_id = ElementId::new(84);
        let root = div().children([
            text_input("ap")
                .track_focus(FocusHandle::new(editable_id))
                .accessibility_role(AccessibilityRole::EditableComboBox)
                .accessibility_label("Fruit")
                .accessibility_expanded(true)
                .accessibility_has_popup(AccessibilityPopup::ListBox)
                .accessibility_auto_complete(AccessibilityAutoComplete::List)
                .accessibility_controls(listbox_id)
                .accessibility_active_descendant(option_id),
            div()
                .id(listbox_id)
                .accessibility_role(AccessibilityRole::ListBox)
                .accessibility_size_of_set(2)
                .child(
                    button()
                        .id(option_id)
                        .tab_index(-1)
                        .accessibility_role(AccessibilityRole::ListBoxOption)
                        .accessibility_position_in_set(0)
                        .selected(true)
                        .child("Apple"),
                ),
            button()
                .id(select_id)
                .accessibility_role(AccessibilityRole::ComboBox)
                .accessibility_label("Letter")
                .accessibility_value("Gamma")
                .accessibility_expanded(false)
                .accessibility_has_popup(AccessibilityPopup::ListBox)
                .child("Gamma"),
        ]);
        let mut tree = UiTree::new();
        let mut renderer = TestTextLayout;
        tree.set_root(root, Size::new(640.0, 320.0), 1.0, &mut renderer)
            .unwrap();
        let mut scene = Scene::new();
        tree.paint(&mut scene, &mut renderer).unwrap();
        let update = tree.accessibility_update("Comboboxes");
        let node = |id| {
            update
                .nodes
                .iter()
                .find_map(|(node_id, node)| (*node_id == accessibility_id(id)).then_some(node))
                .expect("combobox accessibility node")
        };

        let editable = node(editable_id);
        assert_eq!(editable.role(), Role::EditableComboBox);
        assert_eq!(
            editable.auto_complete(),
            Some(NativeAccessibilityAutoComplete::List)
        );
        assert_eq!(editable.is_expanded(), Some(true));
        assert_eq!(editable.has_popup(), Some(AccessibilityHasPopup::Listbox));
        assert_eq!(editable.controls(), &[accessibility_id(listbox_id)]);
        assert_eq!(
            editable.active_descendant(),
            Some(accessibility_id(option_id))
        );
        assert_eq!(node(listbox_id).role(), Role::ListBox);
        assert_eq!(node(listbox_id).size_of_set(), Some(2));
        assert_eq!(node(option_id).role(), Role::ListBoxOption);
        assert_eq!(node(option_id).position_in_set(), Some(0));
        assert_eq!(node(option_id).is_selected(), Some(true));
        assert_eq!(node(select_id).role(), Role::ComboBox);
        assert_eq!(node(select_id).value(), Some("Gamma"));
        assert_eq!(node(select_id).is_expanded(), Some(false));
    }

    #[test]
    fn virtual_collections_project_active_items_counts_indices_levels_and_sorting() {
        let grid_id = ElementId::new(61);
        let header_id = ElementId::new(62);
        let row_id = ElementId::new(63);
        let cell_id = ElementId::new(64);
        let tree_id = ElementId::new(71);
        let tree_item_id = ElementId::new(72);
        let root = div().children([
            div()
                .id(grid_id)
                .track_focus(FocusHandle::new(grid_id))
                .accessibility_role(AccessibilityRole::Grid)
                .accessibility_row_count(100_001)
                .accessibility_column_count(3)
                .accessibility_active_descendant(cell_id)
                .child(
                    div()
                        .id(header_id)
                        .accessibility_role(AccessibilityRole::ColumnHeader)
                        .accessibility_column_index(1)
                        .accessibility_sort_direction(AccessibilitySortDirection::Ascending)
                        .child("Status"),
                )
                .child(
                    div()
                        .id(row_id)
                        .accessibility_role(AccessibilityRole::Row)
                        .accessibility_row_index(50_001)
                        .selected(true)
                        .child(
                            div()
                                .id(cell_id)
                                .accessibility_role(AccessibilityRole::GridCell)
                                .accessibility_row_index(50_001)
                                .accessibility_column_index(1)
                                .selected(true)
                                .child("Running"),
                        ),
                ),
            div()
                .id(tree_id)
                .track_focus(FocusHandle::new(tree_id))
                .accessibility_role(AccessibilityRole::Tree)
                .accessibility_size_of_set(2)
                .accessibility_active_descendant(tree_item_id)
                .child(
                    div()
                        .id(tree_item_id)
                        .accessibility_role(AccessibilityRole::TreeItem)
                        .accessibility_level(2)
                        .accessibility_position_in_set(3)
                        .accessibility_size_of_set(5)
                        .accessibility_expanded(true)
                        .selected(true)
                        .child("Sources"),
                ),
        ]);
        let mut tree = UiTree::new();
        let mut renderer = TestTextLayout;
        tree.set_root(root, Size::new(640.0, 320.0), 1.0, &mut renderer)
            .unwrap();
        let mut scene = Scene::new();
        tree.paint(&mut scene, &mut renderer).unwrap();
        let update = tree.accessibility_update("Collections");
        let node = |id| {
            update
                .nodes
                .iter()
                .find_map(|(node_id, node)| (*node_id == accessibility_id(id)).then_some(node))
                .expect("collection accessibility node")
        };

        assert_eq!(node(grid_id).role(), Role::Grid);
        assert_eq!(node(grid_id).row_count(), Some(100_001));
        assert_eq!(node(grid_id).column_count(), Some(3));
        assert_eq!(
            node(grid_id).active_descendant(),
            Some(accessibility_id(cell_id))
        );
        assert_eq!(node(header_id).role(), Role::ColumnHeader);
        assert_eq!(node(header_id).column_index(), Some(1));
        assert_eq!(
            node(header_id).sort_direction(),
            Some(NativeAccessibilitySortDirection::Ascending)
        );
        assert_eq!(node(row_id).role(), Role::Row);
        assert_eq!(node(row_id).row_index(), Some(50_001));
        assert_eq!(node(cell_id).role(), Role::GridCell);
        assert_eq!(node(cell_id).column_index(), Some(1));
        assert_eq!(node(tree_id).role(), Role::Tree);
        assert_eq!(node(tree_id).size_of_set(), Some(2));
        assert_eq!(
            node(tree_id).active_descendant(),
            Some(accessibility_id(tree_item_id))
        );
        assert_eq!(node(tree_item_id).role(), Role::TreeItem);
        assert_eq!(node(tree_item_id).level(), Some(2));
        assert_eq!(node(tree_item_id).position_in_set(), Some(3));
        assert_eq!(node(tree_item_id).size_of_set(), Some(5));
        assert_eq!(node(tree_item_id).is_expanded(), Some(true));
    }

    #[test]
    fn ime_notifications_compare_commits_against_the_preedit_backup() {
        let id = ElementId::new(44);
        let mut tree = UiTree::new();
        tree.focused = Some(id);
        tree.text_inputs.insert(
            id,
            TextInputState::with_constraints(
                "hello ",
                false,
                crate::element::InputConstraints::default(),
            ),
        );

        let preedit = tree.input_preedit("你", Some((3, 3)));
        assert!(preedit.repaint);
        assert!(preedit.change.is_none());

        let commit = tree.input_replace("你");
        assert!(commit.repaint);
        assert_eq!(
            commit.change.as_ref().map(|change| change.value.as_ref()),
            Some("hello 你")
        );
    }

    #[test]
    fn invalid_controls_expose_native_state_and_validation_description() {
        let id = ElementId::new(45);
        let mut root = crate::text_input("")
            .id(id)
            .required(true)
            .invalid(true)
            .validation_message("A value is required");
        assign_runtime_ids(&mut root);

        let mut tree = UiTree::new();
        tree.viewport = Size::new(320.0, 100.0);
        tree.element_bounds
            .insert(id, Rect::new(10.0, 10.0, 240.0, 40.0));
        tree.text_inputs.insert(
            id,
            TextInputState::with_constraints(
                "",
                false,
                crate::element::InputConstraints::default(),
            ),
        );
        tree.root = Some(root);
        tree.rebuild_focus_index();
        tree.rebuild_dispatch_index();
        assert!(tree.focus(id));
        assert!(tree.focused_text_input_is_invalid());

        let update = tree.accessibility_update("Input test");
        let node = update
            .nodes
            .iter()
            .find_map(|(node_id, node)| (*node_id == accessibility_id(id)).then_some(node))
            .expect("input accessibility node");
        assert_eq!(node.invalid(), Some(AccessibilityInvalid::True));
        assert!(node.is_required());
        assert_eq!(node.description(), Some("A value is required"));
    }

    #[test]
    fn password_inputs_mask_scene_and_accessibility_values_and_disable_copy() {
        let id = ElementId::new(46);
        let secret = "sk-é👨‍👩‍👧‍👦";
        let masked = PASSWORD_MASK.repeat(secret.graphemes(true).count());
        let mut tree = UiTree::new();
        let mut renderer = TestTextLayout;
        tree.set_root(
            crate::text_input(secret).id(id).password(true),
            Size::new(320.0, 100.0),
            1.0,
            &mut renderer,
        )
        .unwrap();
        assert!(tree.focus(id));
        assert!(tree.input_select_all().repaint);

        let mut scene = Scene::new();
        tree.paint(&mut scene, &mut renderer).unwrap();
        let input_text = scene
            .text_runs()
            .iter()
            .find(|run| run.id == TextId::new(id.value()))
            .expect("password text run");
        assert_eq!(input_text.content.as_ref(), masked);
        assert!(!input_text.content.contains(secret));
        assert!(tree.selected_input_text().is_none());

        let update = tree.accessibility_update("Password test");
        let node = update
            .nodes
            .iter()
            .find_map(|(node_id, node)| (*node_id == accessibility_id(id)).then_some(node))
            .expect("password accessibility node");
        assert_eq!(node.role(), Role::PasswordInput);
        assert_eq!(node.value(), Some(masked.as_str()));

        let display = PasswordDisplay::new(secret);
        for boundary in secret
            .grapheme_indices(true)
            .map(|(index, _)| index)
            .chain([secret.len()])
        {
            assert_eq!(
                display.source_index(display.display_index(boundary)),
                boundary
            );
        }
    }

    #[test]
    fn invalid_form_reports_in_document_order_focuses_and_announces_once() {
        let form_id = ElementId::new(100);
        let first_id = ElementId::new(101);
        let disabled_id = ElementId::new(102);
        let nested_form_id = ElementId::new(103);
        let nested_id = ElementId::new(104);
        let mut root = form().id(form_id).children([
            crate::text_input("")
                .id(first_id)
                .invalid(true)
                .validation_message("Name is required"),
            crate::text_input("")
                .id(disabled_id)
                .invalid(true)
                .validation_message("Disabled does not participate")
                .disabled(true),
            form().id(nested_form_id).child(
                crate::text_input("")
                    .id(nested_id)
                    .invalid(true)
                    .validation_message("Nested form owns this field"),
            ),
        ]);
        assign_runtime_ids(&mut root);

        let mut tree = UiTree::new();
        tree.root = Some(root);
        for id in [first_id, disabled_id, nested_id] {
            tree.text_inputs.insert(
                id,
                TextInputState::with_constraints("", false, InputConstraints::default()),
            );
        }
        tree.rebuild_focus_index();
        tree.rebuild_dispatch_index();

        let Some(FormAttempt::Invalid(report)) =
            tree.attempt_form_submission(form_id, Some(first_id))
        else {
            panic!("outer form should be invalid");
        };
        assert_eq!(report.form(), form_id);
        assert_eq!(report.trigger(), Some(first_id));
        assert_eq!(report.issues().len(), 1);
        assert_eq!(report.issues()[0].id(), first_id);
        assert_eq!(report.issues()[0].message(), Some("Name is required"));
        assert_eq!(tree.focused(), Some(first_id));

        let first_alert = tree
            .validation_announcement
            .as_ref()
            .expect("validation announcement")
            .node;
        let update = tree.accessibility_update("Form test");
        let alert = update
            .nodes
            .iter()
            .find_map(|(id, node)| (*id == first_alert).then_some(node))
            .expect("assertive validation alert");
        assert_eq!(alert.role(), Role::Alert);
        assert_eq!(alert.live(), Some(Live::Assertive));
        assert_eq!(alert.value(), Some("Name is required"));

        let Some(FormAttempt::Invalid(_)) = tree.attempt_form_submission(form_id, Some(first_id))
        else {
            panic!("repeated attempt remains invalid");
        };
        assert_ne!(
            tree.validation_announcement.as_ref().unwrap().node,
            first_alert,
            "a repeated identical message must still create an accessibility change"
        );
    }

    #[test]
    fn valid_form_shares_bounded_controlled_values_and_skips_nested_forms() {
        let form_id = ElementId::new(110);
        let name_id = ElementId::new(111);
        let notes_id = ElementId::new(112);
        let disabled_id = ElementId::new(113);
        let nested_form_id = ElementId::new(114);
        let nested_id = ElementId::new(115);
        let mut root = form().id(form_id).children([
            crate::text_input("Ada").id(name_id),
            crate::text_area("Notes").id(notes_id),
            crate::text_input("Ignored").id(disabled_id).disabled(true),
            form()
                .id(nested_form_id)
                .child(crate::text_input("Nested").id(nested_id)),
        ]);
        assign_runtime_ids(&mut root);

        let mut tree = UiTree::new();
        tree.root = Some(root);
        for (id, value, multiline) in [
            (name_id, "Ada", false),
            (notes_id, "Notes", true),
            (disabled_id, "Ignored", false),
            (nested_id, "Nested", false),
        ] {
            tree.text_inputs.insert(
                id,
                TextInputState::with_constraints(value, multiline, InputConstraints::default()),
            );
        }
        tree.rebuild_focus_index();
        tree.rebuild_dispatch_index();

        let Some(FormAttempt::Valid(event)) = tree.attempt_form_submission(form_id, Some(name_id))
        else {
            panic!("outer form should submit");
        };
        assert_eq!(event.form(), form_id);
        assert_eq!(event.trigger(), Some(name_id));
        assert_eq!(event.fields().len(), 2);
        assert_eq!(event.value(name_id), Some("Ada"));
        assert_eq!(event.value(notes_id), Some("Notes"));
        assert_eq!(event.value(disabled_id), None);
        assert_eq!(event.value(nested_id), None);
        assert!(!event.is_truncated());
    }

    #[test]
    fn no_wrap_flex_text_skips_intrinsic_shaping_during_layout() {
        let layout = TaffyStyle {
            flex_basis: Dimension::length(0.0),
            min_size: TaffySize {
                width: Dimension::length(0.0),
                height: Dimension::auto(),
            },
            size: TaffySize {
                width: Dimension::auto(),
                height: Dimension::length(20.0),
            },
            ..TaffyStyle::default()
        };
        let known = TaffySize {
            width: None,
            height: None,
        };
        let max_content = TaffySize {
            width: AvailableSpace::MaxContent,
            height: AvailableSpace::MaxContent,
        };
        let text = TextStyle::new(14.0, Color::WHITE).wrap(TextWrap::None);

        assert_eq!(
            fixed_text_layout_size(known, max_content, &layout, &text),
            Some(TaffySize {
                width: 0.0,
                height: 20.0,
            })
        );
        assert_eq!(
            fixed_text_layout_size(
                known,
                TaffySize {
                    width: AvailableSpace::Definite(640.0),
                    height: AvailableSpace::MaxContent,
                },
                &layout,
                &text,
            ),
            Some(TaffySize {
                width: 640.0,
                height: 20.0,
            })
        );
    }

    #[test]
    fn wrapped_flex_text_keeps_intrinsic_measurement() {
        let layout = TaffyStyle {
            flex_basis: Dimension::length(0.0),
            min_size: TaffySize {
                width: Dimension::length(0.0),
                height: Dimension::auto(),
            },
            size: TaffySize {
                width: Dimension::auto(),
                height: Dimension::length(20.0),
            },
            ..TaffyStyle::default()
        };
        let text = TextStyle::new(14.0, Color::WHITE).wrap(TextWrap::Word);

        assert_eq!(
            fixed_text_layout_size(
                TaffySize {
                    width: None,
                    height: None,
                },
                TaffySize {
                    width: AvailableSpace::MaxContent,
                    height: AvailableSpace::MaxContent,
                },
                &layout,
                &text,
            ),
            None
        );
    }

    #[test]
    fn editable_text_ignores_inherited_display_only_overflow() {
        let root = div()
            .line_clamp(2)
            .text_ellipsis_middle()
            .child(text_input("complete editable value"));
        let mut tree = UiTree::new();
        let mut renderer = TestTextLayout;
        tree.set_root(root, Size::new(320.0, 180.0), 1.0, &mut renderer)
            .unwrap();

        let input = &tree.root.as_ref().unwrap().children[0];
        assert_eq!(input.resolved_typography.wrap, TextWrap::None);
        assert!(input.resolved_typography.text_overflow.is_none());
        assert!(input.resolved_typography.line_clamp.is_none());
    }

    #[test]
    fn anchored_placement_keeps_the_preferred_side_when_it_fits() {
        let placed = place_anchored(
            Rect::new(50.0, 50.0, 30.0, 20.0),
            Size::new(80.0, 40.0),
            Rect::new(0.0, 0.0, 240.0, 180.0),
            AnchorPlacement::BottomStart,
            8.0,
            8.0,
        );
        assert_eq!(placed, Rect::new(50.0, 78.0, 80.0, 40.0));
    }

    #[test]
    fn anchored_placement_flips_before_it_shifts() {
        let placed = place_anchored(
            Rect::new(70.0, 150.0, 30.0, 20.0),
            Size::new(80.0, 48.0),
            Rect::new(0.0, 0.0, 240.0, 180.0),
            AnchorPlacement::BottomStart,
            8.0,
            8.0,
        );
        assert_eq!(placed, Rect::new(70.0, 94.0, 80.0, 48.0));
    }

    #[test]
    fn anchored_placement_tries_opposite_alignment_near_an_edge() {
        let placed = place_anchored(
            Rect::new(210.0, 40.0, 20.0, 24.0),
            Size::new(96.0, 40.0),
            Rect::new(0.0, 0.0, 240.0, 180.0),
            AnchorPlacement::BottomStart,
            8.0,
            8.0,
        );
        assert_eq!(placed, Rect::new(134.0, 72.0, 96.0, 40.0));
    }

    #[test]
    fn oversized_anchored_surfaces_pin_to_the_viewport_margin() {
        let placed = place_anchored(
            Rect::new(80.0, 70.0, 20.0, 20.0),
            Size::new(300.0, 220.0),
            Rect::new(0.0, 0.0, 240.0, 180.0),
            AnchorPlacement::Right,
            8.0,
            8.0,
        );
        assert_eq!(placed, Rect::new(8.0, 8.0, 300.0, 220.0));
    }

    #[test]
    fn scrollbars_reveal_on_scroll_then_hide_with_one_deadline() {
        let mut tree = UiTree::new();
        let id = ElementId::new(7);
        let bounds = Rect::new(0.0, 0.0, 100.0, 100.0);
        tree.scroll_regions.push(ScrollRegion {
            id,
            bounds,
            scrollbar_bounds: bounds,
            clip: bounds,
            max_offset: Vector::new(0.0, 900.0),
            virtual_scroll: false,
            order: PaintOrder {
                layer: PaintLayerKey::default(),
                source: 0,
            },
            scrollbar_order: PaintOrder {
                layer: PaintLayerKey::default(),
                source: 1,
            },
        });

        let now = Instant::now();
        assert!(
            tree.scroll_at(Some(Point::new(50.0, 50.0)), Vector::new(0.0, -120.0), now,)
                .changed
        );
        assert_eq!(tree.scroll_offsets[&id].y, 120.0);
        let deadline = tree
            .next_scrollbar_deadline()
            .expect("scroll reveals thumb");
        assert_eq!(deadline.duration_since(now), SCROLLBAR_AUTO_HIDE_DELAY);
        assert!(!tree.advance_scrollbars(deadline - Duration::from_millis(1)));
        assert!(tree.advance_scrollbars(deadline));
        assert!(!tree.advance_scrollbars(deadline));
        assert!(tree.next_scrollbar_deadline().is_none());
    }

    #[test]
    fn scroll_end_following_pauses_when_the_user_scrolls_away() {
        let id = ElementId::new(91);
        let mut states = HashMap::new();
        let mut offset = Vector::ZERO;

        apply_scroll_end_revision(id, Some(1), 100.0, &mut offset, &mut states);
        assert_eq!(offset.y, 100.0);
        apply_scroll_end_revision(id, Some(2), 160.0, &mut offset, &mut states);
        assert_eq!(offset.y, 160.0);

        offset.y = 40.0;
        apply_scroll_end_revision(id, Some(3), 220.0, &mut offset, &mut states);
        assert_eq!(offset.y, 40.0);

        offset.y = 220.0;
        apply_scroll_end_revision(id, Some(4), 260.0, &mut offset, &mut states);
        assert_eq!(offset.y, 260.0);
        apply_scroll_end_revision(id, None, 260.0, &mut offset, &mut states);
        assert!(!states.contains_key(&id));
    }

    #[test]
    fn tooltips_use_one_exact_show_deadline_and_hide_without_a_loop() {
        let mut tree = UiTree::new();
        let id = ElementId::new(88);
        let bounds = Rect::new(0.0, 0.0, 100.0, 40.0);
        tree.tooltips.insert(
            id,
            Tooltip::text("Delayed").delay(Duration::from_millis(500)),
        );
        tree.hit_regions.push(HitRegion {
            id,
            bounds,
            clip: bounds,
            clickable: false,
            pointer_listener: false,
            drag_source: false,
            drop_target: false,
            focusable: false,
            cursor_style: None,
            cursor_states: CursorStateStyles::default(),
            stateful: false,
            blocks_pointer: false,
            app_region: None,
            order: PaintOrder {
                layer: PaintLayerKey::default(),
                source: 0,
            },
        });

        let now = Instant::now();
        assert!(!tree.update_tooltip_hover(Some(Point::new(10.0, 10.0)), now));
        let deadline = tree.next_tooltip_deadline().expect("show deadline");
        assert_eq!(deadline, now + Duration::from_millis(500));
        assert!(!tree.advance_tooltips(deadline - Duration::from_millis(1)));
        assert!(tree.advance_tooltips(deadline));
        assert_eq!(tree.visible_tooltip, Some(id));
        assert!(tree.next_tooltip_deadline().is_none());
        assert!(!tree.update_tooltip_hover(Some(Point::new(20.0, 20.0)), deadline));
        assert!(tree.update_tooltip_hover(None, deadline));
        assert!(tree.visible_tooltip.is_none());
        assert!(tree.next_tooltip_deadline().is_none());
    }

    #[test]
    fn keyboard_focus_schedules_tooltips_but_pointer_focus_does_not_reopen_them() {
        let mut tree = UiTree::new();
        let id = ElementId::new(89);
        tree.tooltips.insert(id, Tooltip::text("Keyboard help"));
        tree.focusable_ids.insert(id);

        assert!(tree.focus(id));
        assert!(tree.next_tooltip_deadline().is_some());
        assert!(!tree.clear_tooltip());
        tree.focused = None;

        assert!(tree.focus_from_pointer(id));
        assert!(tree.next_tooltip_deadline().is_none());
    }

    #[test]
    fn nested_targeted_listeners_bubble_but_overlays_do_not_click_through() {
        let mut tree = UiTree::new();
        let parent = ElementId::new(90);
        let child = ElementId::new(91);
        let blocker = ElementId::new(92);
        let bounds = Rect::new(0.0, 0.0, 100.0, 100.0);
        let region = |id, source, blocks_pointer| HitRegion {
            id,
            bounds,
            clip: bounds,
            clickable: false,
            pointer_listener: false,
            drag_source: false,
            drop_target: false,
            focusable: false,
            cursor_style: None,
            cursor_states: CursorStateStyles::default(),
            stateful: false,
            blocks_pointer,
            app_region: None,
            order: PaintOrder {
                layer: PaintLayerKey::default(),
                source,
            },
        };
        tree.context_menu_ids.insert(parent);
        tree.scroll_wheel_ids.insert(parent);
        tree.touch_ids.insert(parent);
        tree.mouse_pressure_ids.insert(parent);
        tree.pinch_ids.insert(parent);
        tree.rotation_ids.insert(parent);
        tree.smart_magnify_ids.insert(parent);
        tree.parents.insert(child, parent);
        tree.hit_regions.push(region(parent, 0, false));
        tree.hit_regions.push(region(child, 1, false));

        let point = Point::new(10.0, 10.0);
        assert_eq!(tree.context_menu_listener_at(point), Some(parent));
        assert_eq!(tree.scroll_wheel_listener_at(point), Some(parent));
        assert_eq!(tree.touch_listener_at(point), Some(parent));
        assert_eq!(tree.mouse_pressure_listener_at(point), Some(parent));
        assert_eq!(tree.pinch_listener_at(point), Some(parent));
        assert_eq!(tree.rotation_listener_at(point), Some(parent));
        assert_eq!(tree.smart_magnify_listener_at(point), Some(parent));

        tree.scroll_wheel_ids.insert(child);
        tree.touch_ids.insert(child);
        assert_eq!(tree.scroll_wheel_listener_at(point), Some(child));
        assert_eq!(tree.parent_scroll_wheel_listener(child), Some(parent));
        assert_eq!(tree.parent_scroll_wheel_listener(parent), None);
        assert_eq!(tree.touch_listener_at(point), Some(child));
        assert_eq!(tree.parent_touch_listener(child), Some(parent));
        assert_eq!(tree.parent_touch_listener(parent), None);

        tree.hit_regions.push(region(blocker, 2, true));
        assert_eq!(tree.context_menu_listener_at(point), None);
        assert_eq!(tree.scroll_wheel_listener_at(point), None);
        assert_eq!(tree.touch_listener_at(point), None);
        assert_eq!(tree.mouse_pressure_listener_at(point), None);
        assert_eq!(tree.pinch_listener_at(point), None);
        assert_eq!(tree.rotation_listener_at(point), None);
        assert_eq!(tree.smart_magnify_listener_at(point), None);
    }

    #[test]
    fn desktop_mouse_dispatch_is_bounded_ordered_and_hover_tracks_layout() {
        let mut tree = UiTree::new();
        let parent = ElementId::new(100);
        let child = ElementId::new(101);
        let outside = ElementId::new(102);
        let bounds = Rect::new(0.0, 0.0, 100.0, 100.0);
        let region = |id, source| HitRegion {
            id,
            bounds,
            clip: bounds,
            clickable: false,
            pointer_listener: false,
            drag_source: false,
            drop_target: false,
            focusable: false,
            cursor_style: None,
            cursor_states: CursorStateStyles::default(),
            stateful: false,
            blocks_pointer: false,
            app_region: None,
            order: PaintOrder {
                layer: PaintLayerKey::default(),
                source,
            },
        };
        let binding = |key, kind, phase, button, outside| MouseListenerBinding {
            key: MouseListenerKey(key),
            kind,
            phase,
            button,
            outside,
        };
        tree.mouse_listener_bindings.extend([
            binding(
                0,
                MouseListenerKind::Down,
                DispatchPhase::Capture,
                None,
                false,
            ),
            binding(
                1,
                MouseListenerKind::Down,
                DispatchPhase::Bubble,
                Some(MouseButton::Left),
                false,
            ),
            binding(
                2,
                MouseListenerKind::Hover,
                DispatchPhase::Bubble,
                None,
                false,
            ),
            binding(
                3,
                MouseListenerKind::Down,
                DispatchPhase::Bubble,
                None,
                false,
            ),
            binding(
                4,
                MouseListenerKind::Hover,
                DispatchPhase::Bubble,
                None,
                false,
            ),
            binding(
                5,
                MouseListenerKind::Down,
                DispatchPhase::Capture,
                None,
                true,
            ),
        ]);
        tree.mouse_listener_ranges.insert(parent, 0..3);
        tree.mouse_listener_ranges.insert(child, 3..5);
        tree.mouse_listener_ranges.insert(outside, 5..6);
        tree.parents.insert(child, parent);
        tree.hit_regions.push(region(parent, 0));
        tree.hit_regions.push(region(child, 1));
        let outside_bounds = Rect::new(200.0, 200.0, 40.0, 40.0);
        tree.hit_regions.push(HitRegion {
            bounds: outside_bounds,
            clip: outside_bounds,
            ..region(outside, 2)
        });

        let point = Point::new(10.0, 10.0);
        let mut path = Vec::new();
        assert!(tree.mouse_event_path_at(point, &mut path));
        assert_eq!(path, vec![child, parent]);
        let mut dispatch = Vec::new();
        tree.collect_mouse_dispatch(
            &path,
            MouseListenerKind::Down,
            Some(MouseButton::Left),
            &mut dispatch,
        );
        assert_eq!(
            dispatch,
            vec![
                MouseListenerKey(5),
                MouseListenerKey(0),
                MouseListenerKey(3),
                MouseListenerKey(1),
            ]
        );

        tree.refresh_mouse_hover(Some(point));
        let mut changes = Vec::new();
        tree.take_mouse_hover_changes(&mut changes);
        assert_eq!(
            changes,
            vec![
                MouseHoverChange {
                    key: MouseListenerKey(2),
                    hovered: true,
                },
                MouseHoverChange {
                    key: MouseListenerKey(4),
                    hovered: true,
                },
            ]
        );
        changes.clear();
        tree.hit_regions.clear();
        tree.refresh_mouse_hover(Some(point));
        tree.take_mouse_hover_changes(&mut changes);
        assert_eq!(
            changes,
            vec![
                MouseHoverChange {
                    key: MouseListenerKey(4),
                    hovered: false,
                },
                MouseHoverChange {
                    key: MouseListenerKey(2),
                    hovered: false,
                },
            ]
        );

        let blocker = ElementId::new(103);
        tree.hit_regions.push(HitRegion {
            bounds: outside_bounds,
            clip: outside_bounds,
            ..region(outside, 2)
        });
        tree.hit_regions.push(HitRegion {
            blocks_pointer: true,
            ..region(blocker, 3)
        });
        assert!(tree.mouse_event_path_at(point, &mut path));
        assert_eq!(path, vec![blocker]);
        tree.collect_mouse_dispatch(
            &path,
            MouseListenerKind::Down,
            Some(MouseButton::Left),
            &mut dispatch,
        );
        assert_eq!(dispatch, vec![MouseListenerKey(5)]);
    }

    #[test]
    fn virtual_scroll_uses_the_shared_reveal_hover_and_drag_path() {
        let mut tree = UiTree::new();
        let mut list = crate::VirtualList::new(100, 10.0);
        list.set_viewport_height(100.0);
        let id = ElementId::new(71);
        let bounds = Rect::new(0.0, 0.0, 100.0, 100.0);
        let handle = list.scroll_handle();
        tree.virtual_scroll_handles.insert(
            id,
            RetainedVirtualScroll {
                measurement_revision: handle.measurement_revision(),
                handle,
                mount: list.scroll_mount(),
            },
        );
        tree.scroll_regions.push(ScrollRegion {
            id,
            bounds,
            scrollbar_bounds: bounds,
            clip: bounds,
            max_offset: Vector::new(0.0, list.max_scroll_offset()),
            virtual_scroll: true,
            order: PaintOrder {
                layer: PaintLayerKey::default(),
                source: 0,
            },
            scrollbar_order: PaintOrder {
                layer: PaintLayerKey::default(),
                source: 1,
            },
        });

        let now = Instant::now();
        let result = tree.scroll_at(Some(Point::new(50.0, 50.0)), Vector::new(0.0, -120.0), now);

        assert_eq!(
            result,
            ScrollResult {
                changed: true,
                view_dirty: true,
            }
        );
        assert_eq!(list.scroll_offset(), 120.0);
        assert_eq!(
            tree.next_scrollbar_deadline()
                .expect("virtual scrolling reveals the same overlay thumb"),
            now + SCROLLBAR_AUTO_HIDE_DELAY,
        );

        let track = Point::new(95.0, 50.0);
        assert!(tree.update_scrollbar_hover(Some(track), now));
        assert!(tree.scrollbar_states[&id].hovered);
        assert_eq!(tree.begin_scrollbar_drag(Some(track)), Some(true));
        assert_eq!(list.scroll_offset(), tree.scroll_offsets[&id].y);

        let dragged = tree.drag_scrollbar(Point::new(95.0, 99.0));
        assert_eq!(
            dragged,
            ScrollResult {
                changed: true,
                view_dirty: true,
            }
        );
        assert_eq!(list.scroll_offset(), list.max_scroll_offset());
        assert!(tree.end_scrollbar_drag(now).changed);
        assert!(tree.update_scrollbar_hover(None, now));
        assert_eq!(
            tree.next_scrollbar_deadline()
                .expect("leaving the virtual scrollbar schedules one hide"),
            now + SCROLLBAR_AUTO_HIDE_DELAY,
        );
    }

    #[test]
    fn virtual_scroll_translates_the_mounted_overscan_before_rebuilding() {
        let mut list = crate::VirtualList::new(100, 10.0).with_overscan(2);
        list.set_viewport_height(100.0);
        let row = ElementId::named("retained-virtual-row");
        let root = div()
            .id("retained-virtual-viewport")
            .relative()
            .size(100.0, 100.0)
            .overflow_hidden()
            .virtual_scroll(&list)
            .child(
                div()
                    .id(row)
                    .absolute()
                    .top(0.0)
                    .left(0.0)
                    .size(100.0, 10.0),
            );
        let mut tree = UiTree::new();
        let mut renderer = TestTextLayout;
        tree.set_root(root, Size::new(100.0, 100.0), 1.0, &mut renderer)
            .unwrap();
        let mut scene = Scene::new();
        tree.paint(&mut scene, &mut renderer).unwrap();
        assert_eq!(
            tree.element_bounds(row),
            Some(Rect::new(0.0, 0.0, 100.0, 10.0))
        );

        let now = Instant::now();
        assert_eq!(
            tree.scroll_at(Some(Point::new(50.0, 50.0)), Vector::new(0.0, -10.0), now,),
            ScrollResult {
                changed: true,
                view_dirty: false,
            }
        );
        scene.clear(Color::TRANSPARENT);
        tree.paint(&mut scene, &mut renderer).unwrap();
        assert_eq!(
            tree.element_bounds(row),
            Some(Rect::new(0.0, -10.0, 100.0, 10.0))
        );

        assert_eq!(
            tree.scroll_at(Some(Point::new(50.0, 50.0)), Vector::new(0.0, -11.0), now,),
            ScrollResult {
                changed: true,
                view_dirty: true,
            }
        );
    }

    #[test]
    fn variable_list_measurements_inside_the_mounted_slice_do_not_rebuild_the_view() {
        let list = crate::ListState::new(10, 20.0).with_overscan(0);
        list.set_viewport_size(100.0, 100.0);
        let rows = list.render_rows(list.visible_rows().range, |_| div().h(200.0));
        let root = div()
            .relative()
            .size(100.0, 100.0)
            .overflow_hidden()
            .variable_virtual_scroll(&list)
            .child(rows);
        let mut tree = UiTree::new();
        let mut renderer = TestTextLayout;
        tree.set_root(root, Size::new(100.0, 100.0), 1.0, &mut renderer)
            .unwrap();

        let mut scene = Scene::new();
        tree.paint(&mut scene, &mut renderer).unwrap();

        assert_eq!(
            tree.take_variable_list_measurement_update(),
            ScrollResult {
                changed: true,
                view_dirty: false,
            }
        );
        assert_eq!(
            tree.take_variable_list_measurement_update(),
            ScrollResult::default()
        );
    }

    #[test]
    fn overflow_and_virtual_sibling_scrollbars_keep_independent_native_state() {
        let mut tree = UiTree::new();
        let overflow_id = ElementId::new(72);
        let virtual_id = ElementId::new(73);
        let left = Rect::new(0.0, 0.0, 100.0, 100.0);
        let right = Rect::new(100.0, 0.0, 100.0, 100.0);
        let mut list = crate::VirtualList::new(100, 10.0);
        list.set_viewport_height(100.0);
        let handle = list.scroll_handle();
        tree.virtual_scroll_handles.insert(
            virtual_id,
            RetainedVirtualScroll {
                measurement_revision: handle.measurement_revision(),
                handle,
                mount: list.scroll_mount(),
            },
        );
        let region = |id, bounds, virtual_scroll, source| ScrollRegion {
            id,
            bounds,
            scrollbar_bounds: bounds,
            clip: bounds,
            max_offset: Vector::new(0.0, 900.0),
            virtual_scroll,
            order: PaintOrder {
                layer: PaintLayerKey::default(),
                source,
            },
            scrollbar_order: PaintOrder {
                layer: PaintLayerKey::default(),
                source: source + 1,
            },
        };
        tree.scroll_regions
            .push(region(overflow_id, left, false, 0));
        tree.scroll_regions.push(region(virtual_id, right, true, 2));

        let now = Instant::now();
        assert_eq!(
            tree.scroll_at(Some(Point::new(50.0, 50.0)), Vector::new(0.0, -120.0), now,),
            ScrollResult {
                changed: true,
                view_dirty: false,
            }
        );
        assert_eq!(
            tree.scroll_at(Some(Point::new(150.0, 50.0)), Vector::new(0.0, -120.0), now,),
            ScrollResult {
                changed: true,
                view_dirty: true,
            }
        );
        assert_eq!(tree.scroll_offsets[&overflow_id].y, 120.0);
        assert_eq!(tree.scroll_offsets[&virtual_id].y, 120.0);
        assert_eq!(list.scroll_offset(), 120.0);
        assert_eq!(
            tree.scrollbar_states[&overflow_id].visible_until,
            Some(now + SCROLLBAR_AUTO_HIDE_DELAY)
        );
        assert_eq!(
            tree.scrollbar_states[&virtual_id].visible_until,
            Some(now + SCROLLBAR_AUTO_HIDE_DELAY)
        );

        assert!(tree.update_scrollbar_hover(Some(Point::new(195.0, 50.0)), now));
        assert_eq!(tree.hovered_scrollbar, Some(virtual_id));
        assert!(tree.scrollbar_states[&virtual_id].hovered);
        assert!(!tree.scrollbar_states[&overflow_id].hovered);
        assert_eq!(
            tree.begin_scrollbar_drag(Some(Point::new(195.0, 50.0))),
            Some(true)
        );
        assert!(tree.scrollbar_states[&virtual_id].dragging);
        assert!(!tree.scrollbar_states[&overflow_id].dragging);
    }

    #[test]
    fn multiline_caret_visibility_scrolls_both_axes_and_clamps() {
        let viewport = Size::new(100.0, 80.0);
        let max_scroll = Vector::new(100.0, 200.0);
        assert_eq!(
            scroll_to_reveal_caret(
                viewport,
                Point::new(140.0, 220.0),
                20.0,
                Vector::ZERO,
                max_scroll,
            ),
            Vector::new(56.0, 162.0),
        );
        assert_eq!(
            scroll_to_reveal_caret(
                viewport,
                Point::new(400.0, 400.0),
                20.0,
                Vector::ZERO,
                max_scroll,
            ),
            max_scroll,
        );
        assert_eq!(
            scroll_to_reveal_caret(
                viewport,
                Point::new(10.0, 20.0),
                20.0,
                Vector::new(42.0, 162.0),
                max_scroll,
            ),
            Vector::new(0.0, 20.0),
        );

        assert_eq!(
            scroll_to_reveal_caret(
                viewport,
                Point::new(8.0, 20.0),
                20.0,
                Vector::new(900.0, 0.0),
                Vector::new(1_000.0, 0.0),
            ),
            Vector::ZERO,
        );
    }

    #[test]
    fn single_line_input_never_exposes_vertical_scroll() {
        let content = Size::new(600.0, 28.0);
        let viewport = Size::new(200.0, 20.0);

        assert_eq!(
            text_input_max_scroll(content, viewport, 20.0, false),
            Vector::new(400.0, 0.0)
        );
        assert_eq!(
            text_input_max_scroll(content, viewport, 20.0, true),
            Vector::new(400.0, 8.0)
        );
    }

    #[test]
    fn scrollbar_track_captures_drag_and_hover_without_click_through() {
        let mut tree = UiTree::new();
        let id = ElementId::new(8);
        let bounds = Rect::new(0.0, 0.0, 100.0, 100.0);
        tree.scroll_regions.push(ScrollRegion {
            id,
            bounds,
            scrollbar_bounds: bounds,
            clip: bounds,
            max_offset: Vector::new(0.0, 900.0),
            virtual_scroll: false,
            order: PaintOrder {
                layer: PaintLayerKey::default(),
                source: 0,
            },
            scrollbar_order: PaintOrder {
                layer: PaintLayerKey::default(),
                source: 1,
            },
        });

        let now = Instant::now();
        let point = Point::new(95.0, 50.0);
        assert!(tree.update_scrollbar_hover(Some(point), now));
        assert!(!tree.update_scrollbar_hover(Some(point), now));
        assert!(tree.is_over_scrollbar(point));
        assert_eq!(tree.begin_scrollbar_drag(Some(point)), Some(false));
        assert!(tree.scrollbar_drag_active());
        assert_eq!(tree.scroll_offsets[&id].y, 450.0);
        assert!(tree.drag_scrollbar(Point::new(95.0, 88.0)).changed);
        assert_eq!(tree.scroll_offsets[&id].y, 900.0);
        assert!(tree.end_scrollbar_drag(now).changed);
        assert!(tree.update_scrollbar_hover(None, now));
        assert_eq!(
            tree.next_scrollbar_deadline()
                .expect("leaving the track schedules one hide"),
            now + SCROLLBAR_AUTO_HIDE_DELAY,
        );
    }

    #[test]
    fn topmost_no_drag_and_overlay_scrollbar_override_drag_region() {
        let mut tree = UiTree::new();
        let bounds = Rect::new(0.0, 0.0, 100.0, 100.0);
        let order = |source| PaintOrder {
            layer: PaintLayerKey::default(),
            source,
        };
        let hit_region = |id, app_region, source| HitRegion {
            id: ElementId::new(id),
            bounds,
            clip: bounds,
            clickable: false,
            pointer_listener: false,
            drag_source: false,
            drop_target: false,
            focusable: false,
            cursor_style: None,
            cursor_states: CursorStateStyles::default(),
            stateful: false,
            blocks_pointer: false,
            app_region: Some(app_region),
            order: order(source),
        };
        tree.hit_regions.push(hit_region(1, AppRegion::Drag, 0));
        tree.hit_regions.push(hit_region(2, AppRegion::NoDrag, 1));

        assert!(!tree.is_app_region_drag(Point::new(50.0, 50.0)));
        tree.hit_regions.pop();
        assert!(tree.is_app_region_drag(Point::new(50.0, 50.0)));

        tree.scroll_regions.push(ScrollRegion {
            id: ElementId::new(3),
            bounds,
            scrollbar_bounds: bounds,
            clip: bounds,
            max_offset: Vector::new(0.0, 900.0),
            virtual_scroll: false,
            order: order(1),
            scrollbar_order: order(2),
        });
        assert!(!tree.is_app_region_drag(Point::new(95.0, 50.0)));
    }

    #[test]
    fn overlay_pointer_blockers_hide_lower_layer_cursors_and_targets() {
        let mut tree = UiTree::new();
        let bounds = Rect::new(0.0, 0.0, 100.0, 100.0);
        tree.hit_regions.push(HitRegion {
            id: ElementId::new(1),
            bounds,
            clip: bounds,
            clickable: true,
            pointer_listener: false,
            drag_source: false,
            drop_target: false,
            focusable: true,
            cursor_style: Some(CursorStyle::PointingHand),
            cursor_states: CursorStateStyles::default(),
            stateful: false,
            blocks_pointer: false,
            app_region: None,
            order: PaintOrder {
                layer: PaintLayerKey::default(),
                source: 0,
            },
        });
        tree.hit_regions.push(HitRegion {
            id: ElementId::new(2),
            bounds,
            clip: bounds,
            clickable: false,
            pointer_listener: false,
            drag_source: false,
            drop_target: false,
            focusable: false,
            cursor_style: None,
            cursor_states: CursorStateStyles::default(),
            stateful: false,
            blocks_pointer: true,
            app_region: None,
            order: PaintOrder {
                layer: PaintLayerKey {
                    plane: crate::ScenePlane::Overlay,
                    z_index: 0,
                },
                source: 1,
            },
        });

        assert_eq!(tree.cursor_style_at(Point::new(10.0, 10.0)), None);
        assert!(tree.interactive_region_at(Point::new(10.0, 10.0)).is_none());
        #[cfg(target_os = "macos")]
        assert!(tree.overlay_input_active());
    }

    #[test]
    fn dismissible_overlay_resets_the_cursor_over_inert_background_content() {
        let mut tree = UiTree::new();
        let viewport = Rect::new(0.0, 0.0, 100.0, 100.0);
        tree.hit_regions.push(HitRegion {
            id: ElementId::new(1),
            bounds: viewport,
            clip: viewport,
            clickable: true,
            pointer_listener: false,
            drag_source: false,
            drop_target: false,
            focusable: true,
            cursor_style: Some(CursorStyle::IBeam),
            cursor_states: CursorStateStyles::default(),
            stateful: false,
            blocks_pointer: false,
            app_region: None,
            order: PaintOrder {
                layer: PaintLayerKey::default(),
                source: 0,
            },
        });
        tree.hit_regions.push(HitRegion {
            id: ElementId::new(2),
            bounds: Rect::new(0.0, 0.0, 20.0, 20.0),
            clip: viewport,
            clickable: false,
            pointer_listener: false,
            drag_source: false,
            drop_target: false,
            focusable: false,
            cursor_style: Some(CursorStyle::Arrow),
            cursor_states: CursorStateStyles::default(),
            stateful: false,
            blocks_pointer: true,
            app_region: None,
            order: PaintOrder {
                layer: PaintLayerKey {
                    plane: crate::ScenePlane::Overlay,
                    z_index: 0,
                },
                source: 1,
            },
        });
        tree.dismiss_regions.push(DismissRegion {
            id: ElementId::new(2),
            bounds: Rect::new(0.0, 0.0, 20.0, 20.0),
            clip: viewport,
            policy: DismissPolicy::BOTH,
            restore_focus: None,
            order: PaintOrder {
                layer: PaintLayerKey {
                    plane: crate::ScenePlane::Overlay,
                    z_index: 0,
                },
                source: 1,
            },
        });

        let exposed_background = Point::new(50.0, 50.0);
        assert_eq!(
            tree.cursor_style_at(exposed_background),
            Some(CursorStyle::Arrow)
        );
        tree.dismiss_regions.pop();
        assert_eq!(
            tree.cursor_style_at(exposed_background),
            Some(CursorStyle::IBeam)
        );
        tree.hit_regions.pop();
        assert_eq!(
            tree.cursor_style_at(exposed_background),
            Some(CursorStyle::IBeam)
        );
    }

    #[test]
    fn topmost_dismiss_policy_keeps_escape_and_outside_pointer_independent() {
        let mut tree = UiTree::new();
        let bounds = Rect::new(20.0, 20.0, 40.0, 40.0);
        let region = |policy| DismissRegion {
            id: ElementId::new(9),
            bounds,
            clip: Rect::new(0.0, 0.0, 100.0, 100.0),
            policy,
            restore_focus: Some(ElementId::new(1)),
            order: PaintOrder {
                layer: PaintLayerKey::default(),
                source: 0,
            },
        };

        tree.dismiss_regions
            .push(region(DismissPolicy::default().with_escape()));
        assert_eq!(tree.dismiss_topmost().unwrap().id, ElementId::new(9));
        assert!(
            tree.dismiss_request_for_pointer(Some(Point::new(5.0, 5.0)))
                .is_none()
        );

        tree.dismiss_regions.clear();
        tree.dismiss_regions
            .push(region(DismissPolicy::default().with_pointer_outside()));
        assert!(tree.dismiss_topmost().is_none());
        assert_eq!(
            tree.dismiss_request_for_pointer(Some(Point::new(5.0, 5.0)))
                .unwrap()
                .id,
            ElementId::new(9)
        );
        assert!(
            tree.dismiss_request_for_pointer(Some(Point::new(30.0, 30.0)))
                .is_none()
        );
    }

    #[test]
    fn explicit_arrow_resets_a_lower_cursor_without_blocking_pointer_hits() {
        let mut tree = UiTree::new();
        let bounds = Rect::new(0.0, 0.0, 100.0, 100.0);
        let region = |id, cursor_style, source| HitRegion {
            id: ElementId::new(id),
            bounds,
            clip: bounds,
            clickable: false,
            pointer_listener: false,
            drag_source: false,
            drop_target: false,
            focusable: false,
            cursor_style,
            cursor_states: CursorStateStyles::default(),
            stateful: false,
            blocks_pointer: false,
            app_region: None,
            order: PaintOrder {
                layer: PaintLayerKey::default(),
                source,
            },
        };
        tree.hit_regions
            .push(region(1, Some(CursorStyle::PointingHand), 0));
        tree.hit_regions
            .push(region(2, Some(CursorStyle::Arrow), 1));

        let point = Point::new(10.0, 10.0);
        assert_eq!(tree.cursor_style_at(point), Some(CursorStyle::Arrow));

        tree.hit_regions[1].cursor_style = None;
        assert_eq!(tree.cursor_style_at(point), Some(CursorStyle::PointingHand));
    }

    #[test]
    fn cursor_state_overrides_resolve_live_without_repainting_the_hit_region() {
        let mut tree = UiTree::new();
        let id = ElementId::new(1);
        let bounds = Rect::new(0.0, 0.0, 100.0, 100.0);
        tree.hit_regions.push(HitRegion {
            id,
            bounds,
            clip: bounds,
            clickable: true,
            pointer_listener: false,
            drag_source: false,
            drop_target: false,
            focusable: true,
            cursor_style: Some(CursorStyle::Arrow),
            cursor_states: CursorStateStyles {
                hover: Some(CursorStyle::Crosshair),
                active: Some(CursorStyle::ClosedHand),
                focus: Some(CursorStyle::IBeam),
                invalid: None,
                dragging: Some(CursorStyle::DragCopy),
                drag_over: Some(CursorStyle::DragLink),
            },
            stateful: false,
            blocks_pointer: false,
            app_region: None,
            order: PaintOrder {
                layer: PaintLayerKey::default(),
                source: 0,
            },
        });
        let point = Point::new(10.0, 10.0);

        assert_eq!(tree.cursor_style_at(point), Some(CursorStyle::Crosshair));
        tree.pressed = Some(id);
        assert_eq!(tree.cursor_style_at(point), Some(CursorStyle::ClosedHand));
        tree.pressed = None;
        tree.dragging = Some(id);
        assert_eq!(tree.cursor_style_at(point), Some(CursorStyle::DragCopy));
        tree.drag_over = Some(id);
        assert_eq!(tree.cursor_style_at(point), Some(CursorStyle::DragLink));
        tree.drag_over = None;
        tree.dragging = None;
        tree.external_drag_active = true;
        tree.focused = Some(id);
        assert_eq!(tree.cursor_style_at(point), Some(CursorStyle::IBeam));
        tree.hit_regions[0].cursor_states.invalid = Some(CursorStyle::OperationNotAllowed);
        assert_eq!(
            tree.cursor_style_at(point),
            Some(CursorStyle::OperationNotAllowed)
        );
    }

    #[test]
    fn inferred_cursors_disable_cleanly_while_explicit_not_allowed_survives() {
        let disabled_button = div().clickable().disabled(true);
        let explicit_disabled = div().clickable().disabled(true).cursor_not_allowed();
        let mut drag_source = div().clickable();
        drag_source.drag_source = true;
        let explicit_drag_source = drag_source.clone().cursor_copy();

        assert_eq!(effective_cursor_style(&disabled_button, false), None);
        assert_eq!(
            effective_cursor_style(&explicit_disabled, false),
            Some(CursorStyle::OperationNotAllowed)
        );
        assert_eq!(
            effective_cursor_style(&drag_source, false),
            Some(CursorStyle::OpenHand)
        );
        assert_eq!(
            effective_cursor_style(&explicit_drag_source, false),
            Some(CursorStyle::DragCopy)
        );
        assert_eq!(
            effective_cursor_style(&div(), true),
            Some(CursorStyle::IBeam)
        );
    }

    #[test]
    fn pointer_listeners_capture_the_top_hit_without_clicking_through() {
        let mut tree = UiTree::new();
        let bounds = Rect::new(0.0, 0.0, 100.0, 100.0);
        tree.hit_regions.push(HitRegion {
            id: ElementId::new(1),
            bounds,
            clip: bounds,
            clickable: true,
            pointer_listener: false,
            drag_source: false,
            drop_target: false,
            focusable: true,
            cursor_style: Some(CursorStyle::PointingHand),
            cursor_states: CursorStateStyles::default(),
            stateful: false,
            blocks_pointer: false,
            app_region: None,
            order: PaintOrder {
                layer: PaintLayerKey::default(),
                source: 0,
            },
        });
        tree.hit_regions.push(HitRegion {
            id: ElementId::new(2),
            bounds,
            clip: bounds,
            clickable: false,
            pointer_listener: true,
            drag_source: false,
            drop_target: false,
            focusable: false,
            cursor_style: Some(CursorStyle::PointingHand),
            cursor_states: CursorStateStyles::default(),
            stateful: false,
            blocks_pointer: false,
            app_region: None,
            order: PaintOrder {
                layer: PaintLayerKey::default(),
                source: 1,
            },
        });

        assert_eq!(
            tree.pointer_listener_at(Point::new(10.0, 10.0)),
            Some(ElementId::new(2))
        );
        assert!(tree.interactive_region_at(Point::new(10.0, 10.0)).is_none());
        assert_eq!(
            tree.cursor_style_at(Point::new(10.0, 10.0)),
            Some(CursorStyle::PointingHand)
        );
    }

    #[test]
    fn typed_drag_hits_skip_incompatible_targets_but_respect_pointer_blockers() {
        let mut tree = UiTree::new();
        let bounds = Rect::new(0.0, 0.0, 100.0, 100.0);
        let region = |id, source, target, blocks, order| HitRegion {
            id: ElementId::new(id),
            bounds,
            clip: bounds,
            clickable: false,
            pointer_listener: false,
            drag_source: source,
            drop_target: target,
            focusable: false,
            cursor_style: None,
            cursor_states: CursorStateStyles::default(),
            stateful: source || target,
            blocks_pointer: blocks,
            app_region: None,
            order: PaintOrder {
                layer: PaintLayerKey::default(),
                source: order,
            },
        };
        tree.hit_regions.push(region(1, false, true, false, 0));
        tree.hit_regions.push(region(2, true, true, false, 1));

        let point = Point::new(10.0, 10.0);
        assert_eq!(tree.drag_source_at(point), Some(ElementId::new(2)));
        assert_eq!(
            tree.drop_target_at(point, |id| id == ElementId::new(1)),
            Some(ElementId::new(1))
        );
        assert!(tree.begin_drag(ElementId::new(2)));
        assert!(tree.set_drag_over(Some(ElementId::new(1))));
        assert!(!tree.set_drag_over(Some(ElementId::new(1))));
        assert!(tree.end_drag());

        tree.hit_regions[1].blocks_pointer = true;
        assert_eq!(
            tree.drop_target_at(point, |id| id == ElementId::new(1)),
            None
        );
    }

    #[test]
    fn typed_drop_predicates_control_highlight_and_delivery_acceptance() {
        let mut root = div().id(7_u64).can_drop::<u32>(|value| *value >= 10);
        assign_runtime_ids(&mut root);
        let mut tree = UiTree::new();
        tree.root = Some(root);
        tree.rebuild_drop_predicates();

        assert!(!tree.can_drop(ElementId::new(7), TypeId::of::<u32>(), &9_u32));
        assert!(tree.can_drop(ElementId::new(7), TypeId::of::<u32>(), &10_u32));
        assert!(tree.can_drop(ElementId::new(7), TypeId::of::<u64>(), &0_u64));
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn native_drop_snapshot_matches_typed_targets_predicates_and_blockers() {
        use crate::{ExternalDragText, ExternalDragUrl};

        let mut tree = UiTree::new();
        let bounds = Rect::new(0.0, 0.0, 100.0, 100.0);
        let region = |id, pointer_listener, blocks_pointer, order| HitRegion {
            id: ElementId::new(id),
            bounds,
            clip: bounds,
            clickable: false,
            pointer_listener,
            drag_source: false,
            drop_target: true,
            focusable: false,
            cursor_style: None,
            cursor_states: CursorStateStyles::default(),
            stateful: true,
            blocks_pointer,
            app_region: None,
            order: PaintOrder {
                layer: PaintLayerKey::default(),
                source: order,
            },
        };
        tree.hit_regions.push(region(1, false, false, 0));
        tree.hit_regions.push(region(2, false, false, 1));
        tree.drop_predicates.insert(
            (ElementId::new(1), TypeId::of::<ExternalDragText>()),
            Arc::new(|value| {
                value
                    .downcast_ref::<ExternalDragText>()
                    .is_some_and(|text| text.as_str() == "accepted")
            }),
        );

        let mut snapshot = ExternalDropSnapshot::new();
        tree.update_external_drop_snapshot(
            &mut snapshot,
            &[
                (ElementId::new(1), TypeId::of::<ExternalDragText>()),
                (ElementId::new(2), TypeId::of::<ExternalDragUrl>()),
            ],
        );
        let point = Point::new(20.0, 20.0);
        let text = ExternalDragText::new("accepted");
        assert_eq!(
            snapshot.target_at(point, TypeId::of::<ExternalDragText>(), &text),
            Some(ElementId::new(1))
        );
        let rejected = ExternalDragText::new("rejected");
        assert_eq!(
            snapshot.target_at(point, TypeId::of::<ExternalDragText>(), &rejected),
            None
        );
        let url = ExternalDragUrl::new("https://quickgui.dev").unwrap();
        assert_eq!(
            snapshot.target_at(point, TypeId::of::<ExternalDragUrl>(), &url),
            Some(ElementId::new(2))
        );

        tree.hit_regions[1].pointer_listener = true;
        tree.update_external_drop_snapshot(
            &mut snapshot,
            &[(ElementId::new(1), TypeId::of::<ExternalDragText>())],
        );
        assert_eq!(
            snapshot.target_at(point, TypeId::of::<ExternalDragText>(), &text),
            None
        );
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn native_drop_snapshot_selects_the_topmost_target_before_offer_order() {
        use crate::ExternalDragText;

        let mut tree = UiTree::new();
        let bounds = Rect::new(0.0, 0.0, 100.0, 100.0);
        let region = |id, source| HitRegion {
            id: ElementId::new(id),
            bounds,
            clip: bounds,
            clickable: false,
            pointer_listener: false,
            drag_source: false,
            drop_target: true,
            focusable: false,
            cursor_style: None,
            cursor_states: CursorStateStyles::default(),
            stateful: true,
            blocks_pointer: false,
            app_region: None,
            order: PaintOrder {
                layer: PaintLayerKey::default(),
                source,
            },
        };
        // The retained hit stack is bottom-to-top. Element 2 visually covers element 1.
        tree.hit_regions.push(region(1, 0));
        tree.hit_regions.push(region(2, 1));

        let mut snapshot = ExternalDropSnapshot::new();
        tree.update_external_drop_snapshot(
            &mut snapshot,
            &[
                (ElementId::new(1), TypeId::of::<u32>()),
                (ElementId::new(2), TypeId::of::<ExternalDragText>()),
            ],
        );
        let number = 42_u32;
        let text = ExternalDragText::new("topmost");
        let offers = [
            (TypeId::of::<u32>(), &number as &dyn Any),
            (TypeId::of::<ExternalDragText>(), &text as &dyn Any),
        ];
        assert_eq!(
            snapshot.offer_target_at(Point::new(20.0, 20.0), offers.into_iter()),
            Some((ElementId::new(2), 1))
        );

        // Within one target the native offer order is stable: process-local typed data precedes
        // its public representation, so no serialization is needed between QuickGUI windows.
        tree.update_external_drop_snapshot(
            &mut snapshot,
            &[
                (ElementId::new(2), TypeId::of::<u32>()),
                (ElementId::new(2), TypeId::of::<ExternalDragText>()),
            ],
        );
        assert_eq!(
            snapshot.offer_target_at(Point::new(20.0, 20.0), offers.into_iter()),
            Some((ElementId::new(2), 0))
        );
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn native_drop_snapshot_supports_arbitrary_typed_predicates() {
        let mut tree = UiTree::new();
        let bounds = Rect::new(0.0, 0.0, 40.0, 40.0);
        tree.hit_regions.push(HitRegion {
            id: ElementId::new(9),
            bounds,
            clip: bounds,
            clickable: false,
            pointer_listener: false,
            drag_source: false,
            drop_target: true,
            focusable: false,
            cursor_style: None,
            cursor_states: CursorStateStyles::default(),
            stateful: true,
            blocks_pointer: false,
            app_region: None,
            order: PaintOrder {
                layer: PaintLayerKey::default(),
                source: 0,
            },
        });
        tree.drop_predicates.insert(
            (ElementId::new(9), TypeId::of::<u32>()),
            Arc::new(|value| {
                value
                    .downcast_ref::<u32>()
                    .is_some_and(|value| *value >= 10)
            }),
        );

        let mut snapshot = ExternalDropSnapshot::new();
        tree.update_external_drop_snapshot(
            &mut snapshot,
            &[(ElementId::new(9), TypeId::of::<u32>())],
        );
        let point = Point::new(10.0, 10.0);
        assert_eq!(snapshot.target_at(point, TypeId::of::<u32>(), &9_u32), None);
        assert_eq!(
            snapshot.target_at(point, TypeId::of::<u32>(), &10_u32),
            Some(ElementId::new(9))
        );

        tree.drop_predicates.insert(
            (ElementId::new(9), TypeId::of::<u32>()),
            Arc::new(|_| panic!("application predicate panic")),
        );
        tree.update_external_drop_snapshot(
            &mut snapshot,
            &[(ElementId::new(9), TypeId::of::<u32>())],
        );
        assert_eq!(
            snapshot.target_at(point, TypeId::of::<u32>(), &10_u32),
            None
        );
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn native_drop_acceptance_cap_fails_closed_in_listener_order() {
        let mut tree = UiTree::new();
        let bounds = Rect::new(0.0, 0.0, 40.0, 40.0);
        let target = ElementId::new((MAX_EXTERNAL_DROP_ACCEPTANCES + 2) as u64);
        let region = |id, source| HitRegion {
            id,
            bounds,
            clip: bounds,
            clickable: false,
            pointer_listener: false,
            drag_source: false,
            drop_target: true,
            focusable: false,
            cursor_style: None,
            cursor_states: CursorStateStyles::default(),
            stateful: true,
            blocks_pointer: false,
            app_region: None,
            order: PaintOrder {
                layer: PaintLayerKey::default(),
                source,
            },
        };
        // The lower target is within the acceptance cap. The omitted top target must not let the
        // offer reach through to it, so a truncated acceptance table rejects the complete offer.
        tree.hit_regions.push(region(ElementId::new(1), 0));
        tree.hit_regions.push(region(target, 1));
        let mut listeners = (0..MAX_EXTERNAL_DROP_ACCEPTANCES)
            .map(|index| (ElementId::new(index as u64 + 1), TypeId::of::<u32>()))
            .collect::<Vec<_>>();
        listeners.push((target, TypeId::of::<u32>()));

        let mut snapshot = ExternalDropSnapshot::new();
        tree.update_external_drop_snapshot(&mut snapshot, &listeners);
        assert!(snapshot.is_truncated());
        assert_eq!(
            snapshot.target_at(Point::new(10.0, 10.0), TypeId::of::<u32>(), &42_u32,),
            None
        );
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn native_drop_snapshot_retains_only_the_topmost_bounded_stack() {
        use crate::ExternalDragText;

        let mut tree = UiTree::new();
        let bounds = Rect::new(0.0, 0.0, 20.0, 20.0);
        for source in 0..=MAX_EXTERNAL_DROP_HIT_REGIONS {
            tree.hit_regions.push(HitRegion {
                id: ElementId::new(source as u64 + 1),
                bounds,
                clip: bounds,
                clickable: false,
                pointer_listener: false,
                drag_source: false,
                drop_target: true,
                focusable: false,
                cursor_style: None,
                cursor_states: CursorStateStyles::default(),
                stateful: true,
                blocks_pointer: false,
                app_region: None,
                order: PaintOrder {
                    layer: PaintLayerKey::default(),
                    source,
                },
            });
        }
        let mut snapshot = ExternalDropSnapshot::new();
        let listeners = (0..=MAX_EXTERNAL_DROP_HIT_REGIONS)
            .map(|source| {
                (
                    ElementId::new(source as u64 + 1),
                    TypeId::of::<ExternalDragText>(),
                )
            })
            .collect::<Vec<_>>();
        tree.update_external_drop_snapshot(&mut snapshot, &listeners);
        assert!(snapshot.is_truncated());
        assert_eq!(snapshot.regions.len(), MAX_EXTERNAL_DROP_HIT_REGIONS);
        assert_eq!(
            snapshot.target_at(
                Point::new(10.0, 10.0),
                TypeId::of::<ExternalDragText>(),
                &ExternalDragText::new("payload"),
            ),
            Some(ElementId::new(MAX_EXTERNAL_DROP_HIT_REGIONS as u64 + 1))
        );
    }

    #[test]
    fn immutable_text_selection_copies_across_visual_nodes_in_document_order() {
        let ids = [
            ElementId::new(101),
            ElementId::new(102),
            ElementId::new(103),
        ];
        let mut tree = UiTree::new();
        tree.selectable_texts = vec![
            SelectableTextEntry {
                id: ids[0],
                content: Arc::from("alpha"),
                character_lengths: selectable_character_lengths("alpha").into(),
            },
            SelectableTextEntry {
                id: ids[1],
                content: Arc::from("beta"),
                character_lengths: selectable_character_lengths("beta").into(),
            },
            SelectableTextEntry {
                id: ids[2],
                content: Arc::from("gamma"),
                character_lengths: selectable_character_lengths("gamma").into(),
            },
        ];
        tree.selectable_text_indices = ids
            .into_iter()
            .enumerate()
            .map(|(index, id)| (id, index))
            .collect();
        tree.element_bounds
            .insert(ids[0], Rect::new(0.0, 0.0, 50.0, 20.0));
        tree.element_bounds
            .insert(ids[1], Rect::new(58.0, 0.0, 40.0, 20.0));
        tree.element_bounds
            .insert(ids[2], Rect::new(0.0, 28.0, 60.0, 20.0));
        tree.static_text_selection = Some(StaticTextSelection {
            anchor: StaticTextPosition {
                id: ids[0],
                offset: 2,
            },
            focus: StaticTextPosition {
                id: ids[2],
                offset: 2,
            },
        });

        assert_eq!(tree.selected_static_text().as_deref(), Some("pha beta\nga"));

        tree.static_text_selection =
            tree.static_text_selection
                .map(|selection| StaticTextSelection {
                    anchor: selection.focus,
                    focus: selection.anchor,
                });
        assert_eq!(tree.selected_static_text().as_deref(), Some("pha beta\nga"));
    }

    #[test]
    fn immutable_text_selection_clips_ranges_per_leaf() {
        let ids = [
            ElementId::new(201),
            ElementId::new(202),
            ElementId::new(203),
        ];
        let indices = ids
            .into_iter()
            .enumerate()
            .map(|(index, id)| (id, index))
            .collect::<HashMap<_, _>>();
        let selection = Some(StaticTextSelection {
            anchor: StaticTextPosition {
                id: ids[0],
                offset: 2,
            },
            focus: StaticTextPosition {
                id: ids[2],
                offset: 3,
            },
        });

        assert_eq!(
            static_selection_range_for_entry(selection, &indices, 0, 5),
            Some(2..5)
        );
        assert_eq!(
            static_selection_range_for_entry(selection, &indices, 1, 4),
            Some(0..4)
        );
        assert_eq!(
            static_selection_range_for_entry(selection, &indices, 2, 5),
            Some(0..3)
        );
    }

    #[test]
    fn immutable_text_copy_limit_keeps_utf8_boundaries() {
        let mut output = String::new();
        push_bounded_text(&mut output, "a🙂b", 4);
        assert_eq!(output, "a");
        assert!(output.is_char_boundary(output.len()));
    }

    #[test]
    fn immutable_text_drag_uses_visible_bounds_in_shared_clips() {
        let clip = Rect::new(0.0, 0.0, 400.0, 400.0);
        let region = |document_index, bounds, source| SelectableTextRegion {
            document_index,
            bounds,
            clip,
            style: TextStyle::new(14.0, Color::WHITE),
            highlights: None,
            order: PaintOrder {
                layer: PaintLayerKey::default(),
                source,
            },
        };
        let regions = [
            region(0, Rect::new(20.0, 20.0, 200.0, 20.0), 0),
            region(1, Rect::new(20.0, 120.0, 200.0, 20.0), 1),
        ];

        assert_eq!(
            nearest_selectable_text_region(&regions, Point::new(40.0, 55.0))
                .map(|region| region.document_index),
            Some(0)
        );
        assert_eq!(
            nearest_selectable_text_region(&regions, Point::new(40.0, 105.0))
                .map(|region| region.document_index),
            Some(1)
        );
    }

    #[test]
    fn focusing_an_input_clears_static_selection_and_keeps_clipboards_separate() {
        let text_id = ElementId::new(401);
        let input_id = ElementId::new(402);
        let mut tree = UiTree::new();
        tree.selectable_texts.push(SelectableTextEntry {
            id: text_id,
            content: Arc::from("document"),
            character_lengths: selectable_character_lengths("document").into(),
        });
        tree.selectable_text_indices.insert(text_id, 0);
        tree.static_text_selection = Some(StaticTextSelection {
            anchor: StaticTextPosition {
                id: text_id,
                offset: 0,
            },
            focus: StaticTextPosition {
                id: text_id,
                offset: "document".len(),
            },
        });
        tree.focus_order.push(input_id);
        tree.focusable_ids.insert(input_id);
        let mut input = TextInputState::new("input value");
        input.set_selection(0, "input".len());
        tree.text_inputs.insert(input_id, input);

        assert!(tree.focus_next(false));
        assert!(tree.static_text_selection.is_none());
        assert_eq!(tree.selected_input_text().as_deref(), Some("input"));
        assert_eq!(tree.selected_text().as_deref(), Some("input"));
    }

    #[test]
    fn immutable_text_multi_clicks_select_unicode_words_and_complete_lines() {
        let content = "alpha 世界!\nsecond line\n";
        let world = content.find('世').unwrap();
        assert_eq!(word_range_at(content, world), world..world + "世界".len());
        assert_eq!(
            line_range_at(content, world),
            0..content.find('\n').unwrap() + 1
        );

        let mut tree = UiTree::new();
        let id = ElementId::new(301);
        let position = StaticTextPosition { id, offset: world };
        let point = Point::new(20.0, 20.0);
        let now = Instant::now();
        assert_eq!(
            tree.static_text_click_unit(point, position, now, false),
            StaticTextSelectionUnit::Character
        );
        assert_eq!(
            tree.static_text_click_unit(point, position, now + Duration::from_millis(100), false,),
            StaticTextSelectionUnit::Word
        );
        assert_eq!(
            tree.static_text_click_unit(point, position, now + Duration::from_millis(200), false,),
            StaticTextSelectionUnit::Line
        );
    }

    #[test]
    fn automatic_user_selection_matches_web_control_boundaries() {
        let mut root = div()
            .child(text("plain"))
            .child(button().child(text("button")))
            .child(div().user_select_none().child(text("disabled")))
            .child(button().user_select_text().child(text("forced")));
        let mut taffy = TaffyTree::new();
        let mut seen = HashSet::new();
        let inherited = TextStyle::new(14.0, Color::WHITE);
        build_layout_node(
            &mut taffy,
            &mut seen,
            &mut root,
            ElementId::new(999),
            0,
            &inherited,
            true,
        )
        .unwrap();

        assert!(root.children[0].resolved_user_select);
        assert!(!root.children[1].resolved_user_select);
        assert!(!root.children[1].children[0].resolved_user_select);
        assert!(!root.children[2].children[0].resolved_user_select);
        assert!(root.children[3].resolved_user_select);
        assert!(root.children[3].children[0].resolved_user_select);
    }

    fn assign_runtime_ids(element: &mut Element) {
        if let Some(id) = element.explicit_id {
            element.runtime_id = id;
        }
        for child in &mut element.children {
            assign_runtime_ids(child);
        }
    }
}
