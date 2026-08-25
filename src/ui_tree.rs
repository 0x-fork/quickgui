use std::{
    any::{Any, TypeId},
    collections::{HashMap, HashSet},
    sync::Arc,
    time::{Duration, Instant},
};

use accesskit::{
    Action, Affine, Invalid as AccessibilityInvalid, Live, Node as AccessibilityNode,
    NodeId as AccessibilityNodeId, Rect as AccessibilityRect, Role, TextPosition, TextSelection,
    Tree, TreeId, TreeUpdate,
};
use taffy::{
    geometry::Size as TaffySize,
    prelude::{AvailableSpace, NodeId, TaffyTree},
    style::{CompactLength, Dimension, Overflow, Style as TaffyStyle},
};
use thiserror::Error;
use unicode_segmentation::UnicodeSegmentation;

use crate::{
    AccessibilityRole, AnchorPlacement, AnimatedImage, AppRegion, BoxShadow, Canvas, Color,
    CustomShaderPrimitive, Element, ElementId, ImagePrimitive, Insets, KeyContext,
    MAX_TOOLTIPS_PER_WINDOW, ObjectFit, Path, PathPrimitive, Point, Quad, Rect, Scene, Shadow,
    Size, SvgPrimitive, TextHighlight, TextId, TextRun, TextStyle, TextWrap, Tooltip, UserSelect,
    Vector,
    animated_image::AnimatedImageId,
    element::{
        AnchorTarget, DropPredicateCallback, ElementKind, ElementStateStyle, ImageResolution,
    },
    event::{
        FormField, FormSubmitEvent, MAX_FORM_FIELDS, MAX_VALIDATION_ISSUES,
        MAX_VALIDATION_MESSAGE_BYTES, ValidationIssue, ValidationReport,
    },
    image::fit_image,
    renderer::GpuRenderer,
    scene::PaintLayerKey,
    text_input::{
        TextInputState, accessibility_byte_index, accessibility_character_index,
        boundary_at_or_before, selectable_character_lengths,
    },
    virtual_list::VirtualScrollHandle,
};

#[cfg(target_os = "macos")]
use crate::{ScenePlane, native_view::NativeViewPlacement};

const ACCESSIBILITY_ROOT_ID: AccessibilityNodeId = AccessibilityNodeId(u64::MAX);
const SCROLLBAR_AUTO_HIDE_DELAY: Duration = Duration::from_millis(900);
const STATIC_TEXT_MULTI_CLICK_INTERVAL: Duration = Duration::from_millis(500);
const STATIC_TEXT_MULTI_CLICK_DISTANCE: f32 = 4.0;

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
    #[error("anchored element {element:?} refers to missing element {anchor:?}")]
    MissingAnchor {
        element: ElementId,
        anchor: ElementId,
    },
    #[error("a window cannot retain more than {MAX_TOOLTIPS_PER_WINDOW} tooltips")]
    TooManyTooltips,
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
struct HitRegion {
    id: ElementId,
    bounds: Rect,
    clip: Rect,
    clickable: bool,
    pointer_listener: bool,
    drag_source: bool,
    drop_target: bool,
    focusable: bool,
    cursor_pointer: bool,
    cursor_text: bool,
    stateful: bool,
    blocks_pointer: bool,
    app_region: Option<AppRegion>,
    order: PaintOrder,
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
struct PaintOrder {
    layer: PaintLayerKey,
    source: usize,
}

#[derive(Clone, Copy, Debug)]
struct DismissRegion {
    id: ElementId,
    bounds: Rect,
    clip: Rect,
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
    highlights: Option<Arc<[TextHighlight]>>,
    style: TextStyle,
    scroll: Vector,
    max_scroll: Vector,
    caret_bounds: Rect,
}

#[derive(Clone, Debug)]
struct SelectableTextEntry {
    id: ElementId,
    content: Arc<str>,
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

pub(crate) struct UiTree {
    root: Option<Element>,
    taffy: TaffyTree<MeasureContext>,
    root_node: Option<NodeId>,
    seen_ids: HashSet<ElementId>,
    scroll_offsets: HashMap<ElementId, Vector>,
    virtual_scroll_handles: HashMap<ElementId, VirtualScrollHandle>,
    natural_bounds: HashMap<ElementId, Rect>,
    element_bounds: HashMap<ElementId, Rect>,
    hit_regions: Vec<HitRegion>,
    drop_predicates: HashMap<(ElementId, TypeId), DropPredicateCallback>,
    context_menu_ids: HashSet<ElementId>,
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
    animations_enabled: bool,
    selecting_input: Option<ElementId>,
    hovered: HashSet<ElementId>,
    pressed: Option<ElementId>,
    dragging: Option<ElementId>,
    external_drag_active: bool,
    drag_over: Option<ElementId>,
    drag_preview: Option<DragPreview>,
    focused: Option<ElementId>,
    focusable_ids: HashSet<ElementId>,
    clickable_ids: HashSet<ElementId>,
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

struct DragPreview {
    root: Element,
    taffy: TaffyTree<MeasureContext>,
    root_node: NodeId,
    cursor_offset: Point,
    position: Point,
    scroll_offsets: HashMap<ElementId, Vector>,
    natural_bounds: HashMap<ElementId, Rect>,
    text_inputs: HashMap<ElementId, TextInputState>,
    animations: HashMap<ElementId, AnimationPlayback>,
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
    root: Element,
    taffy: TaffyTree<MeasureContext>,
    root_node: NodeId,
    scroll_offsets: HashMap<ElementId, Vector>,
    natural_bounds: HashMap<ElementId, Rect>,
    text_inputs: HashMap<ElementId, TextInputState>,
    animations: HashMap<ElementId, AnimationPlayback>,
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

impl DragPreview {
    #[allow(clippy::too_many_arguments)]
    fn paint(
        &mut self,
        scene: &mut Scene,
        renderer: &mut GpuRenderer,
        scale_factor: f32,
        animations_enabled: bool,
        paint_time: Instant,
        viewport: Rect,
        source_order: &mut usize,
    ) -> Result<(), UiError> {
        self.natural_bounds.clear();
        for playback in self.animations.values_mut() {
            playback.seen = false;
        }
        let origin = Point::new(
            self.position.x - self.cursor_offset.x,
            self.position.y - self.cursor_offset.y,
        );
        collect_layout_bounds(
            &self.root,
            &self.taffy,
            &mut self.scroll_offsets,
            &mut self.natural_bounds,
            origin,
        )?;

        let mut element_bounds = HashMap::new();
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
        let result = paint_element(
            &self.root,
            &self.taffy,
            &self.natural_bounds,
            &mut element_bounds,
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
            PaintLayerKey {
                plane: crate::ScenePlane::Overlay,
                z_index: i16::MAX,
            },
            source_order,
            None,
        );
        for playback in self.animations.values_mut() {
            playback.finish_visibility();
        }
        result
    }
}

impl TooltipOverlay {
    fn new(
        target: ElementId,
        tooltip: &Tooltip,
        viewport: Size,
        scale_factor: f32,
        renderer: &mut GpuRenderer,
    ) -> Result<Self, UiError> {
        let mut root = tooltip.content.as_ref().clone();
        sanitize_drag_preview(&mut root);
        let mut taffy = TaffyTree::with_capacity(32);
        let mut seen_ids = HashSet::with_capacity(32);
        seen_ids.insert(ElementId::new(ACCESSIBILITY_ROOT_ID.0));
        let inherited = TextStyle::new(14.0, Color::WHITE);
        let root_node = build_layout_node(
            &mut taffy,
            &mut seen_ids,
            &mut root,
            ElementId::new(0x7474_6970_6f6f_6c00),
            0,
            &inherited,
            false,
        )?;
        compute_detached_layout(&mut taffy, root_node, viewport, scale_factor, renderer)?;

        let mut text_inputs = HashMap::new();
        let mut input_ids = HashSet::new();
        sync_text_inputs(&root, &mut text_inputs, &mut input_ids);
        let mut animations = HashMap::new();
        let mut animation_ids = HashSet::new();
        sync_animations(&root, &mut animations, &mut animation_ids, Instant::now());
        Ok(Self {
            target,
            content_identity: tooltip.content_identity(),
            placement: tooltip.placement,
            gap: tooltip.gap,
            viewport_margin: tooltip.viewport_margin,
            root,
            taffy,
            root_node,
            scroll_offsets: HashMap::new(),
            natural_bounds: HashMap::new(),
            text_inputs,
            animations,
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
        renderer: &mut GpuRenderer,
        scale_factor: f32,
        animations_enabled: bool,
        paint_time: Instant,
        viewport: Rect,
        source_order: &mut usize,
    ) -> Result<(), UiError> {
        self.natural_bounds.clear();
        for playback in self.animations.values_mut() {
            playback.seen = false;
        }
        let layout = self.taffy.layout(self.root_node)?;
        let placed = place_anchored(
            anchor,
            Size::new(layout.size.width, layout.size.height),
            viewport,
            self.placement,
            self.gap,
            self.viewport_margin,
        );
        let origin = Point::new(placed.x - layout.location.x, placed.y - layout.location.y);
        collect_layout_bounds(
            &self.root,
            &self.taffy,
            &mut self.scroll_offsets,
            &mut self.natural_bounds,
            origin,
        )?;

        let mut element_bounds = HashMap::new();
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
        let result = paint_element(
            &self.root,
            &self.taffy,
            &self.natural_bounds,
            &mut element_bounds,
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
            PaintLayerKey {
                plane: crate::ScenePlane::Overlay,
                z_index: i16::MAX - 1,
            },
            source_order,
            None,
        );
        for playback in self.animations.values_mut() {
            playback.finish_visibility();
        }
        result
    }
}

impl UiTree {
    pub fn new() -> Self {
        Self {
            root: None,
            taffy: TaffyTree::with_capacity(256),
            root_node: None,
            seen_ids: HashSet::with_capacity(256),
            scroll_offsets: HashMap::new(),
            virtual_scroll_handles: HashMap::with_capacity(8),
            natural_bounds: HashMap::with_capacity(256),
            element_bounds: HashMap::with_capacity(256),
            hit_regions: Vec::with_capacity(128),
            drop_predicates: HashMap::with_capacity(8),
            context_menu_ids: HashSet::with_capacity(8),
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
            animations_enabled: true,
            selecting_input: None,
            hovered: HashSet::with_capacity(8),
            pressed: None,
            dragging: None,
            external_drag_active: false,
            drag_over: None,
            drag_preview: None,
            focused: None,
            focusable_ids: HashSet::with_capacity(32),
            clickable_ids: HashSet::with_capacity(32),
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

    pub fn set_root(
        &mut self,
        mut root: Element,
        viewport: Size,
        scale_factor: f32,
        renderer: &mut GpuRenderer,
    ) -> Result<(), UiError> {
        self.taffy.clear();
        self.seen_ids.clear();
        self.seen_ids
            .insert(ElementId::new(ACCESSIBILITY_ROOT_ID.0));
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
        self.animation_ids.clear();
        sync_animations(
            &root,
            &mut self.animations,
            &mut self.animation_ids,
            Instant::now(),
        );
        self.animations
            .retain(|id, _| self.animation_ids.contains(id));
        self.root = Some(root);
        self.root_node = Some(root_node);
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
                Instant::now(),
            );
            let mut input_ids = HashSet::new();
            sync_text_inputs(root, &mut self.text_inputs, &mut input_ids);
            self.text_inputs.retain(|id, _| input_ids.contains(id));

            self.selectable_texts.clear();
            self.selectable_text_indices.clear();
            collect_selectable_texts(
                root,
                &mut self.selectable_texts,
                &mut self.selectable_text_indices,
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
            .retain(|id, _| self.seen_ids.contains(id));
        self.scrollbar_states
            .retain(|id, _| self.seen_ids.contains(id));
        if self
            .hovered_scrollbar
            .is_some_and(|id| !self.seen_ids.contains(&id))
        {
            self.hovered_scrollbar = None;
        }
        self.hovered.retain(|id| self.seen_ids.contains(id));
        if self.pressed.is_some_and(|id| !self.seen_ids.contains(&id)) {
            self.pressed = None;
        }
        if self.dragging.is_some_and(|id| !self.seen_ids.contains(&id)) {
            self.dragging = None;
            self.drag_over = None;
        }
        if self
            .drag_over
            .is_some_and(|id| !self.seen_ids.contains(&id))
        {
            self.drag_over = None;
        }
        if self
            .focused
            .is_some_and(|id| !self.focusable_ids.contains(&id))
        {
            self.focused = None;
        }
        if self.focused.is_none() && !self.focus_initialized {
            self.focused = self
                .root
                .as_ref()
                .and_then(find_auto_focus)
                .filter(|id| self.focusable_ids.contains(id));
        }
        self.focus_initialized = true;
        if self.pointer_tooltip.is_none() {
            self.reconcile_tooltip(Instant::now());
        }
        self.layout(viewport, scale_factor, renderer)
    }

    pub fn set_animations_enabled(&mut self, enabled: bool, now: Instant) {
        if self.animations_enabled == enabled {
            return;
        }
        if !enabled {
            for playback in self.animations.values_mut() {
                playback.advance(now);
                playback.active = false;
            }
            if let Some(preview) = &mut self.drag_preview {
                for playback in preview.animations.values_mut() {
                    playback.advance(now);
                    playback.active = false;
                }
            }
            if let Some(tooltip) = &mut self.tooltip_overlay {
                for playback in tooltip.animations.values_mut() {
                    playback.advance(now);
                    playback.active = false;
                }
            }
        }
        self.animations_enabled = enabled;
    }

    pub fn advance_animations(&mut self, now: Instant) -> bool {
        if !self.animations_enabled {
            return false;
        }
        let mut changed = false;
        for playback in self.animations.values_mut() {
            changed |= playback.advance(now);
        }
        if let Some(preview) = &mut self.drag_preview {
            for playback in preview.animations.values_mut() {
                changed |= playback.advance(now);
            }
        }
        if let Some(tooltip) = &mut self.tooltip_overlay {
            for playback in tooltip.animations.values_mut() {
                changed |= playback.advance(now);
            }
        }
        changed
    }

    pub fn next_animation_deadline(&self) -> Option<Instant> {
        if !self.animations_enabled {
            return None;
        }
        self.animations
            .values()
            .chain(
                self.drag_preview
                    .iter()
                    .flat_map(|preview| preview.animations.values()),
            )
            .chain(
                self.tooltip_overlay
                    .iter()
                    .flat_map(|tooltip| tooltip.animations.values()),
            )
            .filter_map(AnimationPlayback::deadline)
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
            .iter()
            .flat_map(|preview| preview.animations.values());
        let tooltip = self
            .tooltip_overlay
            .iter()
            .flat_map(|tooltip| tooltip.animations.values());
        (
            self.animations.len() + preview.clone().count() + tooltip.clone().count(),
            self.animations
                .values()
                .chain(preview)
                .chain(tooltip)
                .filter(|state| state.active)
                .count(),
        )
    }

    pub fn layout(
        &mut self,
        viewport: Size,
        scale_factor: f32,
        renderer: &mut GpuRenderer,
    ) -> Result<(), UiError> {
        self.viewport = viewport;
        self.scale_factor = scale_factor;
        let Some(root) = self.root_node else {
            return Ok(());
        };
        compute_detached_layout(&mut self.taffy, root, viewport, scale_factor, renderer)?;
        if let Some(preview) = &mut self.drag_preview {
            compute_detached_layout(
                &mut preview.taffy,
                preview.root_node,
                viewport,
                scale_factor,
                renderer,
            )?;
        }
        if let Some(tooltip) = &mut self.tooltip_overlay {
            compute_detached_layout(
                &mut tooltip.taffy,
                tooltip.root_node,
                viewport,
                scale_factor,
                renderer,
            )?;
        }
        Ok(())
    }

    pub fn paint(&mut self, scene: &mut Scene, renderer: &mut GpuRenderer) -> Result<(), UiError> {
        self.natural_bounds.clear();
        self.element_bounds.clear();
        self.hit_regions.clear();
        self.scroll_regions.clear();
        self.dismiss_regions.clear();
        #[cfg(target_os = "macos")]
        self.native_views.clear();
        self.text_input_regions.clear();
        self.selectable_text_regions.clear();
        for playback in self.animations.values_mut() {
            playback.seen = false;
        }
        let Some(root) = &self.root else {
            return Ok(());
        };
        let paint_time = Instant::now();
        let viewport = Rect::from_size(self.viewport);
        collect_layout_bounds(
            root,
            &self.taffy,
            &mut self.scroll_offsets,
            &mut self.natural_bounds,
            Point::ZERO,
        )?;
        let mut source_order = 0;
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
                )?);
            }
            if let Some(overlay) = &mut self.tooltip_overlay {
                overlay.paint(
                    anchor,
                    scene,
                    renderer,
                    self.scale_factor,
                    self.animations_enabled,
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
    pub fn pointer_moved(&mut self, point: Point, renderer: &mut GpuRenderer) -> bool {
        let now = Instant::now();
        let mut next = HashSet::with_capacity(self.hovered.capacity().max(4));
        if self.dragging.is_none() && !self.external_drag_active {
            for region in self.hit_regions.iter().rev() {
                if region.stateful && region.contains(point) {
                    next.insert(region.id);
                }
                if (region.blocks_pointer || region.pointer_listener) && region.contains(point) {
                    break;
                }
            }
        }
        let hover_changed = next != self.hovered;
        if hover_changed {
            self.hovered = next;
        }

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
        hover_changed | self.update_tooltip_hover(None, Instant::now())
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

    pub(crate) fn set_drag_preview(
        &mut self,
        preview: Option<Element>,
        source: ElementId,
        origin: Point,
        position: Point,
        cursor_offset: Option<Point>,
        renderer: &mut GpuRenderer,
    ) -> Result<bool, UiError> {
        let Some(mut root) = preview else {
            return Ok(self.drag_preview.take().is_some());
        };
        sanitize_drag_preview(&mut root);
        if let Some(source_bounds) = self.element_bounds(source) {
            if absolute_length(root.layout.size.width).is_none() {
                root.layout.size.width = Dimension::length(source_bounds.width);
            }
            if absolute_length(root.layout.size.height).is_none() {
                root.layout.size.height = Dimension::length(source_bounds.height);
            }
        }

        let mut taffy = TaffyTree::with_capacity(32);
        let mut seen_ids = HashSet::with_capacity(32);
        seen_ids.insert(ElementId::new(ACCESSIBILITY_ROOT_ID.0));
        let inherited = TextStyle::new(14.0, Color::WHITE);
        let root_node = build_layout_node(
            &mut taffy,
            &mut seen_ids,
            &mut root,
            ElementId::new(0xd4a6_31f8_6c92_7b05),
            0,
            &inherited,
            false,
        )?;
        compute_detached_layout(
            &mut taffy,
            root_node,
            self.viewport,
            self.scale_factor,
            renderer,
        )?;

        let mut text_inputs = HashMap::new();
        let mut input_ids = HashSet::new();
        sync_text_inputs(&root, &mut text_inputs, &mut input_ids);
        let mut animations = HashMap::new();
        let mut animation_ids = HashSet::new();
        sync_animations(&root, &mut animations, &mut animation_ids, Instant::now());
        let default_offset = self
            .element_bounds(source)
            .map(|bounds| Point::new(origin.x - bounds.x, origin.y - bounds.y))
            .unwrap_or(Point::ZERO);
        self.drag_preview = Some(DragPreview {
            root,
            taffy,
            root_node,
            cursor_offset: cursor_offset.unwrap_or(default_offset),
            position,
            scroll_offsets: HashMap::new(),
            natural_bounds: HashMap::new(),
            text_inputs,
            animations,
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
        let view_dirty = region.virtual_scroll && *offset != previous;
        if view_dirty && let Some(handle) = self.virtual_scroll_handles.get(&region.id) {
            handle.set_offset(offset.y);
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
            if region.virtual_scroll
                && let Some(handle) = self.virtual_scroll_handles.get(&region.id)
            {
                handle.set_offset(next);
            }
            ScrollResult {
                changed: true,
                view_dirty: region.virtual_scroll,
            }
        }
    }

    pub(crate) fn end_scrollbar_drag(&mut self, now: Instant) -> bool {
        let Some(drag) = self.scrollbar_drag.take() else {
            return false;
        };
        let state = self.scrollbar_states.entry(drag.id).or_default();
        state.dragging = false;
        if !state.hovered {
            state.visible_until = now.checked_add(SCROLLBAR_AUTO_HIDE_DELAY);
        }
        true
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
        renderer: &mut GpuRenderer,
    ) -> PointerResult {
        let tooltip_repaint = pressed && self.clear_tooltip();
        if pressed
            && let Some(dismiss) = self.dismiss_regions.last().copied()
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
                if region.virtual_scroll
                    && let Some(handle) = self.virtual_scroll_handles.get(&region.id)
                {
                    handle.set_offset(next.y);
                }
                return ScrollResult {
                    changed: true,
                    view_dirty: region.virtual_scroll,
                };
            }
        }
        ScrollResult {
            changed: tooltip_changed,
            view_dirty: false,
        }
    }

    pub fn wants_pointer_cursor(&self, point: Point) -> bool {
        self.cursor_at(point, |region| region.cursor_pointer)
    }

    pub fn wants_text_cursor(&self, point: Point) -> bool {
        self.cursor_at(point, |region| region.cursor_text)
    }

    fn selectable_text_position_at(
        &self,
        point: Point,
        allow_nearest: bool,
        renderer: &mut GpuRenderer,
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
        self.dismiss_regions.last().map(|region| DismissRequest {
            id: region.id,
            restore_focus: region.restore_focus,
        })
    }

    pub(crate) fn dismiss_request_for_pointer(
        &self,
        point: Option<Point>,
    ) -> Option<DismissRequest> {
        let dismiss = self.dismiss_regions.last().copied()?;
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

    fn cursor_at(&self, point: Point, cursor: impl Fn(HitRegion) -> bool) -> bool {
        for region in self.hit_regions.iter().rev() {
            if !region.contains(point) {
                continue;
            }
            if cursor(*region) {
                return true;
            }
            if region.blocks_pointer || region.pointer_listener {
                return false;
            }
        }
        false
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
        self.focused_text_input()
            .and_then(|id| self.text_inputs.get(&id))
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
        renderer: &mut GpuRenderer,
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
        renderer: &mut GpuRenderer,
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
        let next_index = match self
            .focused
            .and_then(|focused| self.focus_order.iter().position(|id| *id == focused))
        {
            Some(index) if reverse => index.checked_sub(1).unwrap_or(self.focus_order.len() - 1),
            Some(index) => (index + 1) % self.focus_order.len(),
            None if reverse => self.focus_order.len() - 1,
            None => 0,
        };
        let next = self.focus_order[next_index];
        self.focus(next)
    }

    pub fn activate_focused(&self) -> Option<ElementId> {
        self.focused
            .filter(|focused| self.clickable_ids.contains(focused))
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
            return Some(*input);
        }
        let id = ElementId::new(id.0);
        self.seen_ids.contains(&id).then_some(id)
    }

    pub fn accessibility_update(&self, window_title: &str) -> TreeUpdate {
        let mut nodes = Vec::with_capacity(
            self.seen_ids.len() + usize::from(self.validation_announcement.is_some()),
        );
        let mut root = AccessibilityNode::new(Role::Window);
        root.set_bounds(accessibility_rect(Rect::from_size(self.viewport)));
        root.set_transform(Affine::scale(self.scale_factor as f64));
        root.set_label(window_title);
        let mut root_children = Vec::with_capacity(2);
        if let Some(element) = &self.root {
            root_children.push(accessibility_id(element.runtime_id));
            let context = AccessibilityBuildContext {
                element_bounds: &self.element_bounds,
                text_inputs: &self.text_inputs,
                selectable_texts: &self.selectable_texts,
                selectable_text_indices: &self.selectable_text_indices,
                static_text_selection: self.static_text_selection,
                accessibility_text_ids: &self.accessibility_text_ids,
            };
            build_accessibility_nodes(element, &context, &mut nodes);
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
                .map(accessibility_id)
                .unwrap_or(ACCESSIBILITY_ROOT_ID),
        }
    }

    fn rebuild_focus_index(&mut self) {
        self.focusable_ids.clear();
        self.clickable_ids.clear();
        self.focus_order.clear();
        let Some(root) = &self.root else {
            return;
        };
        let mut candidates = Vec::new();
        collect_focus_candidates(
            root,
            &mut self.focusable_ids,
            &mut self.clickable_ids,
            &mut candidates,
        );
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
    }

    fn rebuild_dispatch_index(&mut self) {
        self.parents.clear();
        self.key_contexts.clear();
        self.invalid_ids.clear();
        self.form_ids.clear();
        self.form_submitter_ids.clear();
        self.context_menu_ids.clear();
        let Some(root) = &self.root else {
            return;
        };
        collect_dispatch_metadata(
            root,
            None,
            &mut DispatchMetadata {
                parents: &mut self.parents,
                key_contexts: &mut self.key_contexts,
                invalid_ids: &mut self.invalid_ids,
                form_ids: &mut self.form_ids,
                form_submitter_ids: &mut self.form_submitter_ids,
                context_menu_ids: &mut self.context_menu_ids,
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
    renderer: &mut GpuRenderer,
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

fn sanitize_drag_preview(element: &mut Element) {
    element.explicit_id = None;
    element.runtime_id = ElementId::new(0);
    element.taffy_node = None;
    element.clickable = false;
    element.pointer_listener = false;
    element.context_menu_listener = false;
    element.drag_source = false;
    element.drop_target = false;
    element.drop_predicates.clear();
    element.cursor_pointer = false;
    element.cursor_text = false;
    element.user_select = UserSelect::None;
    element.resolved_user_select = false;
    element.focusable = false;
    element.key_context = None;
    element.auto_focus = false;
    element.plane = None;
    element.anchor = None;
    element.tooltip = None;
    element.app_region = None;
    element.virtual_scroll = None;
    element.blocks_pointer = false;
    element.dismissible = false;
    element.restore_focus = None;
    for child in &mut element.children {
        sanitize_drag_preview(child);
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
    renderer: &mut GpuRenderer,
) -> usize {
    if region.content.is_empty() {
        return 0;
    }
    renderer.text_index_for_point_with_highlights(
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
    )
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
    let automatic_user_select = inherited_user_select
        && !element.clickable
        && !element.pointer_listener
        && !element.drag_source
        && !matches!(
            element.accessibility.role,
            AccessibilityRole::Button | AccessibilityRole::CheckBox | AccessibilityRole::MenuItem
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
            } else {
                input.value.clone()
            };
            let highlights = (!input.value.is_empty() && !input.highlights.is_empty())
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
    element.taffy_node = Some(node);
    Ok(node)
}

fn collect_layout_bounds(
    element: &Element,
    taffy: &TaffyTree<MeasureContext>,
    scroll_offsets: &mut HashMap<ElementId, Vector>,
    bounds: &mut HashMap<ElementId, Rect>,
    parent_origin: Point,
) -> Result<(), UiError> {
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
        offset.x = offset.x.clamp(0.0, max_offset.x);
        offset.y = offset.y.clamp(0.0, max_offset.y);
        scroll = *offset;
    }

    let child_origin = Point::new(element_bounds.x - scroll.x, element_bounds.y - scroll.y);
    for child in &element.children {
        collect_layout_bounds(child, taffy, scroll_offsets, bounds, child_origin)?;
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
    renderer: &mut GpuRenderer,
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
    renderer: &mut GpuRenderer,
    text_inputs: &mut HashMap<ElementId, TextInputState>,
    animations: &mut HashMap<ElementId, AnimationPlayback>,
    animations_enabled: bool,
    paint_time: Instant,
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
    let invalid_state = if element.accessibility.invalid {
        &element.invalid_style
    } else {
        &empty_state
    };
    let fill = interaction_state
        .background
        .or(invalid_state.background)
        .or(focus_state.background)
        .or(element.visual.background)
        .unwrap_or(Color::TRANSPARENT);
    let border = interaction_state
        .border_color
        .or(invalid_state.border_color)
        .or(focus_state.border_color)
        .or(element.visual.border_color)
        .unwrap_or(Color::TRANSPARENT);
    let border_width = interaction_state
        .border_width
        .or(invalid_state.border_width)
        .or(focus_state.border_width)
        .unwrap_or(element.visual.border_width);
    let shadows = interaction_state
        .shadows
        .as_deref()
        .or(invalid_state.shadows.as_deref())
        .or(focus_state.shadows.as_deref())
        .or(element.visual.shadows.as_deref())
        .unwrap_or_default();
    push_element_shadows(
        scene,
        layer,
        bounds,
        element.visual.radius,
        parent_clip,
        shadows,
        false,
    );
    if fill.a > 0.0 || (border.a > 0.0 && border_width > 0.0) {
        scene.push_quad_in(
            layer,
            Quad::new(bounds, fill)
                .radius(element.visual.radius)
                .border(border_width, border)
                .clip(parent_clip),
        );
    }
    push_element_shadows(
        scene,
        layer,
        bounds,
        element.visual.radius,
        parent_clip,
        shadows,
        true,
    );

    let selectable_document_index = selectable_text_indices.get(&element.runtime_id).copied();
    if element.clickable
        || element.pointer_listener
        || element.context_menu_listener
        || element.tooltip.is_some()
        || element.drag_source
        || element.drop_target
        || element.cursor_pointer
        || element.cursor_text
        || selectable_document_index.is_some()
        || element.focusable
        || element.blocks_pointer
        || element.app_region.is_some()
        || element.has_stateful_paint()
    {
        hit_regions.push(HitRegion {
            id: element.runtime_id,
            bounds,
            clip: parent_clip,
            clickable: element.clickable && !element.accessibility.disabled,
            pointer_listener: element.pointer_listener && !element.accessibility.disabled,
            drag_source: element.drag_source && !element.accessibility.disabled,
            drop_target: element.drop_target && !element.accessibility.disabled,
            focusable: element.focusable && !element.accessibility.disabled,
            cursor_pointer: element.cursor_pointer && !element.accessibility.disabled,
            cursor_text: (element.cursor_text || selectable_document_index.is_some())
                && !element.accessibility.disabled,
            stateful: element.has_stateful_paint(),
            blocks_pointer: element.blocks_pointer,
            app_region: element.app_region,
            order,
        });
    }
    if element.dismissible {
        dismiss_regions.push(DismissRegion {
            id: element.runtime_id,
            bounds,
            clip: parent_clip,
            restore_focus: element.restore_focus.map(|handle| handle.id()),
            order,
        });
    }

    let state_text_color = interaction_state
        .text_color
        .or(invalid_state.text_color)
        .or(focus_state.text_color)
        .or(inherited_state_text_color);
    let mut text_input_scroll = None;
    match &element.kind {
        ElementKind::Text(content) => {
            let mut style = element.resolved_typography.clone();
            if let Some(color) = state_text_color {
                style.color = color;
            }
            paint_selectable_text(
                element,
                selectable_document_index,
                content,
                &style,
                None,
                bounds,
                parent_clip,
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
                .clip(parent_clip),
            );
        }
        ElementKind::StyledText(styled) => {
            let mut style = element.resolved_typography.clone();
            if let Some(color) = state_text_color {
                style.color = color;
            }
            let text_id = TextId::new(element.runtime_id.value());
            let highlights = styled.shared_highlights().clone();
            if !bounds.is_empty()
                && let Some(clip) = parent_clip.intersection(bounds)
            {
                let visible_y =
                    (clip.y - bounds.y).max(0.0)..(clip.bottom() - bounds.y).min(bounds.height);
                let geometry = renderer.styled_text_geometry(
                    text_id,
                    styled.content(),
                    &style,
                    &highlights,
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
                    parent_clip,
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
                        .clip(parent_clip),
                );
                for decoration in geometry.decorations {
                    scene.push_quad_in(
                        layer,
                        Quad::new(
                            Rect::new(
                                bounds.x + decoration.rect.x,
                                bounds.y + decoration.rect.y,
                                decoration.rect.width,
                                decoration.rect.height,
                            ),
                            decoration.color,
                        )
                        .clip(clip),
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
                let content = input_state.shared_text();
                let highlights = (!content.is_empty())
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
                        input_state.caret(),
                    )
                };
                let max_scroll = Vector::new(
                    (content_size.width - text_viewport.width).max(0.0),
                    (content_size.height.max(style.line_height) - text_viewport.height).max(0.0),
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
                if let Some(highlights) = &highlights {
                    let geometry = renderer.styled_text_geometry(
                        text_id,
                        &content,
                        &style,
                        highlights,
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
                        selection.start,
                        selection.end,
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
                        marked.start,
                        marked.end,
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
                    scene.push_quad_in(
                        layer,
                        Quad::new(
                            Rect::new(
                                text_viewport.x + decoration.rect.x - scroll.x,
                                text_viewport.y + decoration.rect.y - scroll.y,
                                decoration.rect.width,
                                decoration.rect.height,
                            ),
                            decoration.color,
                        )
                        .clip(text_clip),
                    );
                }
                text_input_regions.push(TextInputRegion {
                    id: element.runtime_id,
                    bounds: text_viewport,
                    clip: text_clip,
                    content,
                    highlights,
                    style,
                    scroll,
                    max_scroll,
                    caret_bounds,
                });
                if max_scroll.x > 0.0 || max_scroll.y > 0.0 {
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
                    z_index: layer.z_index,
                    source_order: order.source,
                });
            }
        }
        ElementKind::Container => {}
    }

    let clips_children = element.layout.overflow.x != Overflow::Visible
        || element.layout.overflow.y != Overflow::Visible;
    let child_clip = if clips_children {
        match parent_clip.intersection(bounds) {
            Some(clip) => clip,
            None => return Ok(()),
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
        let max_offset = Vector::new(0.0, virtual_scroll.max_offset_y.max(0.0));
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
    Ok(())
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

fn collect_focus_candidates(
    element: &Element,
    focusable_ids: &mut HashSet<ElementId>,
    clickable_ids: &mut HashSet<ElementId>,
    candidates: &mut Vec<FocusCandidate>,
) {
    if element.focusable && !element.accessibility.disabled {
        focusable_ids.insert(element.runtime_id);
        if element.tab_index >= 0 {
            candidates.push(FocusCandidate {
                id: element.runtime_id,
                tab_index: element.tab_index,
                order: candidates.len(),
            });
        }
    }
    if element.clickable && !element.accessibility.disabled {
        clickable_ids.insert(element.runtime_id);
    }
    for child in &element.children {
        collect_focus_candidates(child, focusable_ids, clickable_ids, candidates);
    }
}

struct DispatchMetadata<'a> {
    parents: &'a mut HashMap<ElementId, ElementId>,
    key_contexts: &'a mut HashMap<ElementId, KeyContext>,
    invalid_ids: &'a mut HashSet<ElementId>,
    form_ids: &'a mut HashSet<ElementId>,
    form_submitter_ids: &'a mut HashSet<ElementId>,
    context_menu_ids: &'a mut HashSet<ElementId>,
}

fn collect_dispatch_metadata(
    element: &Element,
    parent: Option<ElementId>,
    metadata: &mut DispatchMetadata<'_>,
) {
    if let Some(parent) = parent {
        metadata.parents.insert(element.runtime_id, parent);
    }
    if let Some(context) = &element.key_context {
        metadata
            .key_contexts
            .insert(element.runtime_id, context.clone());
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
    for child in &element.children {
        collect_dispatch_metadata(child, Some(element.runtime_id), metadata);
    }
}

fn collect_drop_predicates(
    element: &Element,
    predicates: &mut HashMap<(ElementId, TypeId), DropPredicateCallback>,
) {
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
    if element.auto_focus && element.focusable && !element.accessibility.disabled {
        return Some(element.runtime_id);
    }
    element.children.iter().find_map(find_auto_focus)
}

struct AccessibilityBuildContext<'a> {
    element_bounds: &'a HashMap<ElementId, Rect>,
    text_inputs: &'a HashMap<ElementId, TextInputState>,
    selectable_texts: &'a [SelectableTextEntry],
    selectable_text_indices: &'a HashMap<ElementId, usize>,
    static_text_selection: Option<StaticTextSelection>,
    accessibility_text_ids: &'a HashMap<ElementId, AccessibilityNodeId>,
}

fn build_accessibility_nodes(
    element: &Element,
    context: &AccessibilityBuildContext<'_>,
    nodes: &mut Vec<(AccessibilityNodeId, AccessibilityNode)>,
) {
    let Some(bounds) = context.element_bounds.get(&element.runtime_id).copied() else {
        return;
    };
    let mut node = AccessibilityNode::new(accessibility_role(element.accessibility.role));
    node.set_bounds(accessibility_rect(bounds));
    let mut children = element
        .children
        .iter()
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
        node.set_value(state.text());
        if let ElementKind::TextInput(input) = &element.kind
            && !input.placeholder.is_empty()
        {
            node.set_placeholder(input.placeholder.to_string());
        }
        let anchor = TextPosition {
            node: text_id,
            character_index: state.accessibility_character_index(state.anchor()),
        };
        let focus = TextPosition {
            node: text_id,
            character_index: state.accessibility_character_index(state.caret()),
        };
        node.set_text_selection(TextSelection { anchor, focus });
        if !element.accessibility.disabled {
            node.add_action(Action::SetValue);
            node.add_action(Action::SetTextSelection);
        }

        let mut text_node = AccessibilityNode::new(Role::TextRun);
        text_node.set_bounds(accessibility_rect(bounds));
        text_node.set_value(state.text());
        text_node.set_character_lengths(state.accessibility_character_lengths());
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
                character_index: accessibility_character_index(&entry.content, anchor_offset),
            },
            focus: TextPosition {
                node: text_id,
                character_index: accessibility_character_index(&entry.content, focus_offset),
            },
        });
        if !element.accessibility.disabled {
            node.add_action(Action::SetTextSelection);
        }

        let mut text_node = AccessibilityNode::new(Role::TextRun);
        text_node.set_bounds(accessibility_rect(bounds));
        text_node.set_value(entry.content.to_string());
        text_node.set_character_lengths(selectable_character_lengths(&entry.content));
        nodes.push((text_id, text_node));
    } else if let Some(value) = &element.accessibility.value {
        node.set_value(value.to_string());
    }
    if element.accessibility.disabled {
        node.set_disabled();
    }
    if element.accessibility.invalid {
        node.set_invalid(AccessibilityInvalid::True);
        if let Some(message) = &element.accessibility.validation_message {
            node.set_description(message.to_string());
        }
    }
    if element.accessibility.selected {
        node.set_selected(true);
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
        build_accessibility_nodes(child, context, nodes);
    }
}

fn accessibility_label(element: &Element) -> Option<String> {
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
        AccessibilityRole::TextInput => Role::TextInput,
        AccessibilityRole::MultilineTextInput => Role::MultilineTextInput,
        AccessibilityRole::Dialog => Role::Dialog,
        AccessibilityRole::Menu => Role::Menu,
        AccessibilityRole::MenuItem => Role::MenuItem,
        AccessibilityRole::Tooltip => Role::Tooltip,
        AccessibilityRole::Form => Role::Form,
    }
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

fn sync_text_inputs(
    element: &Element,
    inputs: &mut HashMap<ElementId, TextInputState>,
    ids: &mut HashSet<ElementId>,
) {
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
) {
    if let Some(content) = selectable_text_content(element) {
        let document_index = entries.len();
        entries.push(SelectableTextEntry {
            id: element.runtime_id,
            content: content.clone(),
        });
        indices.insert(element.runtime_id, document_index);
    }
    for child in &element.children {
        collect_selectable_texts(child, entries, indices);
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
    handles: &mut HashMap<ElementId, VirtualScrollHandle>,
    offsets: &mut HashMap<ElementId, Vector>,
    scrollbar_states: &mut HashMap<ElementId, ScrollbarState>,
    now: Instant,
) {
    if let Some(virtual_scroll) = &element.virtual_scroll {
        let max_offset = virtual_scroll.max_offset_y.max(0.0);
        let next = virtual_scroll.handle.offset().clamp(0.0, max_offset);
        virtual_scroll.handle.set_offset(next);
        let previous = offsets.insert(element.runtime_id, Vector::new(0.0, next));
        if previous.is_some_and(|previous| previous.y != next) {
            let state = scrollbar_states.entry(element.runtime_id).or_default();
            if !state.hovered && !state.dragging {
                state.visible_until = now.checked_add(SCROLLBAR_AUTO_HIDE_DELAY);
            }
        }
        handles.insert(element.runtime_id, virtual_scroll.handle.clone());
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
        AnimatedImageFrame, AnimationRepeat, FocusHandle, Image, PathBuilder, button, div, form,
        text,
    };

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
        assert_eq!(node.description(), Some("A value is required"));
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
            cursor_pointer: false,
            cursor_text: false,
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
    fn nested_context_menu_targets_bubble_but_overlays_do_not_click_through() {
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
            cursor_pointer: false,
            cursor_text: false,
            stateful: false,
            blocks_pointer,
            app_region: None,
            order: PaintOrder {
                layer: PaintLayerKey::default(),
                source,
            },
        };
        tree.context_menu_ids.insert(parent);
        tree.parents.insert(child, parent);
        tree.hit_regions.push(region(parent, 0, false));
        tree.hit_regions.push(region(child, 1, false));

        let point = Point::new(10.0, 10.0);
        assert_eq!(tree.context_menu_listener_at(point), Some(parent));

        tree.hit_regions.push(region(blocker, 2, true));
        assert_eq!(tree.context_menu_listener_at(point), None);
    }

    #[test]
    fn virtual_scroll_uses_the_shared_reveal_hover_and_drag_path() {
        let mut tree = UiTree::new();
        let mut list = crate::VirtualList::new(100, 10.0);
        list.set_viewport_height(100.0);
        let id = ElementId::new(71);
        let bounds = Rect::new(0.0, 0.0, 100.0, 100.0);
        tree.virtual_scroll_handles.insert(id, list.scroll_handle());
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
        assert!(tree.end_scrollbar_drag(now));
        assert!(tree.update_scrollbar_hover(None, now));
        assert_eq!(
            tree.next_scrollbar_deadline()
                .expect("leaving the virtual scrollbar schedules one hide"),
            now + SCROLLBAR_AUTO_HIDE_DELAY,
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
        tree.virtual_scroll_handles
            .insert(virtual_id, list.scroll_handle());
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
        assert!(tree.end_scrollbar_drag(now));
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
            cursor_pointer: false,
            cursor_text: false,
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
            cursor_pointer: true,
            cursor_text: false,
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
            cursor_pointer: false,
            cursor_text: false,
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

        assert!(!tree.wants_pointer_cursor(Point::new(10.0, 10.0)));
        assert!(tree.interactive_region_at(Point::new(10.0, 10.0)).is_none());
        #[cfg(target_os = "macos")]
        assert!(tree.overlay_input_active());
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
            cursor_pointer: true,
            cursor_text: false,
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
            cursor_pointer: true,
            cursor_text: false,
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
        assert!(tree.wants_pointer_cursor(Point::new(10.0, 10.0)));
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
            cursor_pointer: false,
            cursor_text: false,
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
            cursor_pointer: false,
            cursor_text: false,
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
            cursor_pointer: false,
            cursor_text: false,
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
            cursor_pointer: false,
            cursor_text: false,
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
            cursor_pointer: false,
            cursor_text: false,
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
                cursor_pointer: false,
                cursor_text: false,
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
            },
            SelectableTextEntry {
                id: ids[1],
                content: Arc::from("beta"),
            },
            SelectableTextEntry {
                id: ids[2],
                content: Arc::from("gamma"),
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
