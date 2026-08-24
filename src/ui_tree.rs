use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
    time::{Duration, Instant},
};

use accesskit::{
    Action, Affine, Node as AccessibilityNode, NodeId as AccessibilityNodeId,
    Rect as AccessibilityRect, Role, TextPosition, TextSelection, Tree, TreeId, TreeUpdate,
};
use taffy::{
    geometry::Size as TaffySize,
    prelude::{AvailableSpace, NodeId, TaffyTree},
    style::Overflow,
};
use thiserror::Error;

use crate::{
    AccessibilityRole, AnchorPlacement, AnimatedImage, BoxShadow, Canvas, Color, Element,
    ElementId, ImagePrimitive, Insets, KeyContext, ObjectFit, Path, PathPrimitive, Point, Quad,
    Rect, Scene, Shadow, Size, SvgPrimitive, TextId, TextRun, TextStyle, Vector,
    animated_image::AnimatedImageId,
    element::{ElementKind, ElementStateStyle, ImageResolution},
    image::fit_image,
    renderer::GpuRenderer,
    scene::PaintLayerKey,
    text_input::TextInputState,
};

#[cfg(target_os = "macos")]
use crate::{ScenePlane, native_view::NativeViewPlacement};

const ACCESSIBILITY_ROOT_ID: AccessibilityNodeId = AccessibilityNodeId(u64::MAX);

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
    focusable: bool,
    cursor_pointer: bool,
    cursor_text: bool,
    stateful: bool,
    blocks_pointer: bool,
    order: PaintOrder,
}

impl HitRegion {
    fn contains(self, point: Point) -> bool {
        self.bounds.contains(point) && self.clip.contains(point)
    }
}

#[derive(Clone, Copy, Debug)]
struct ScrollRegion {
    id: ElementId,
    bounds: Rect,
    max_offset: Vector,
    order: PaintOrder,
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
    style: TextStyle,
    scroll_x: f32,
    caret_bounds: Rect,
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

pub(crate) struct UiTree {
    root: Option<Element>,
    taffy: TaffyTree<MeasureContext>,
    root_node: Option<NodeId>,
    seen_ids: HashSet<ElementId>,
    scroll_offsets: HashMap<ElementId, Vector>,
    natural_bounds: HashMap<ElementId, Rect>,
    element_bounds: HashMap<ElementId, Rect>,
    hit_regions: Vec<HitRegion>,
    scroll_regions: Vec<ScrollRegion>,
    dismiss_regions: Vec<DismissRegion>,
    #[cfg(target_os = "macos")]
    native_views: Vec<NativeViewPlacement>,
    text_input_regions: Vec<TextInputRegion>,
    text_inputs: HashMap<ElementId, TextInputState>,
    accessibility_text_ids: HashMap<ElementId, AccessibilityNodeId>,
    next_accessibility_text_id: u64,
    animations: HashMap<ElementId, AnimationPlayback>,
    animation_ids: HashSet<ElementId>,
    animations_enabled: bool,
    selecting_input: Option<ElementId>,
    hovered: HashSet<ElementId>,
    pressed: Option<ElementId>,
    focused: Option<ElementId>,
    focusable_ids: HashSet<ElementId>,
    clickable_ids: HashSet<ElementId>,
    focus_order: Vec<ElementId>,
    parents: HashMap<ElementId, ElementId>,
    key_contexts: HashMap<ElementId, KeyContext>,
    focus_initialized: bool,
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

impl UiTree {
    pub fn new() -> Self {
        Self {
            root: None,
            taffy: TaffyTree::with_capacity(256),
            root_node: None,
            seen_ids: HashSet::with_capacity(256),
            scroll_offsets: HashMap::new(),
            natural_bounds: HashMap::with_capacity(256),
            element_bounds: HashMap::with_capacity(256),
            hit_regions: Vec::with_capacity(128),
            scroll_regions: Vec::with_capacity(8),
            dismiss_regions: Vec::with_capacity(4),
            #[cfg(target_os = "macos")]
            native_views: Vec::with_capacity(4),
            text_input_regions: Vec::with_capacity(8),
            text_inputs: HashMap::with_capacity(8),
            accessibility_text_ids: HashMap::with_capacity(8),
            next_accessibility_text_id: ACCESSIBILITY_ROOT_ID.0 - 1,
            animations: HashMap::with_capacity(8),
            animation_ids: HashSet::with_capacity(8),
            animations_enabled: true,
            selecting_input: None,
            hovered: HashSet::with_capacity(8),
            pressed: None,
            focused: None,
            focusable_ids: HashSet::with_capacity(32),
            clickable_ids: HashSet::with_capacity(32),
            focus_order: Vec::with_capacity(32),
            parents: HashMap::with_capacity(256),
            key_contexts: HashMap::with_capacity(32),
            focus_initialized: false,
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
            let mut input_ids = HashSet::new();
            sync_text_inputs(root, &mut self.text_inputs, &mut input_ids);
            self.text_inputs.retain(|id, _| input_ids.contains(id));
            sync_accessibility_text_ids(
                &input_ids,
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
        self.scroll_offsets
            .retain(|id, _| self.seen_ids.contains(id));
        self.hovered.retain(|id| self.seen_ids.contains(id));
        if self.pressed.is_some_and(|id| !self.seen_ids.contains(&id)) {
            self.pressed = None;
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
        changed
    }

    pub fn next_animation_deadline(&self) -> Option<Instant> {
        self.animations_enabled
            .then(|| {
                self.animations
                    .values()
                    .filter_map(AnimationPlayback::deadline)
                    .min()
            })
            .flatten()
    }

    pub fn animation_counts(&self) -> (usize, usize) {
        (
            self.animations.len(),
            self.animations
                .values()
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
        self.taffy.compute_layout_with_measure(
            root,
            TaffySize {
                width: AvailableSpace::Definite(viewport.width),
                height: AvailableSpace::Definite(viewport.height),
            },
            |known, available, _node, context, _style| {
                let Some(context) = context else {
                    return TaffySize::ZERO;
                };
                if let (Some(width), Some(height)) = (known.width, known.height) {
                    return TaffySize { width, height };
                }
                match context {
                    MeasureContext::Text { id, content, style } => {
                        let max_width = known.width.or_else(|| match available.width {
                            AvailableSpace::Definite(width) => Some(width.max(0.0)),
                            AvailableSpace::MinContent => Some(0.0),
                            AvailableSpace::MaxContent => None,
                        });
                        let measured =
                            renderer.measure_text(*id, content, style, max_width, scale_factor);
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

    pub fn paint(&mut self, scene: &mut Scene, renderer: &mut GpuRenderer) -> Result<(), UiError> {
        self.natural_bounds.clear();
        self.element_bounds.clear();
        self.hit_regions.clear();
        self.scroll_regions.clear();
        self.dismiss_regions.clear();
        #[cfg(target_os = "macos")]
        self.native_views.clear();
        self.text_input_regions.clear();
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
            &mut self.dismiss_regions,
            #[cfg(target_os = "macos")]
            &mut self.native_views,
            &mut self.text_input_regions,
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
        self.hit_regions.sort_by_key(|region| region.order);
        self.scroll_regions.sort_by_key(|region| region.order);
        self.dismiss_regions.sort_by_key(|region| region.order);
        #[cfg(target_os = "macos")]
        self.native_views
            .sort_by_key(|region| (region.z_index, region.source_order));
        scene.finish();
        Ok(())
    }

    /// Returns true when paint-only hover state changed.
    pub fn pointer_moved(&mut self, point: Point, renderer: &mut GpuRenderer) -> bool {
        let mut next = HashSet::with_capacity(self.hovered.capacity().max(4));
        for region in self.hit_regions.iter().rev() {
            if region.stateful && region.contains(point) {
                next.insert(region.id);
            }
            if region.blocks_pointer && region.contains(point) {
                break;
            }
        }
        let hover_changed = next != self.hovered;
        if hover_changed {
            self.hovered = next;
        }

        let selection_changed = self
            .selecting_input
            .and_then(|id| {
                self.text_input_regions
                    .iter()
                    .rev()
                    .find(|region| region.id == id)
                    .cloned()
            })
            .and_then(|region| {
                let index = text_input_index_at(&region, point, self.scale_factor, renderer);
                self.text_inputs
                    .get_mut(&region.id)
                    .map(|state| state.move_to(index, true))
            })
            .unwrap_or(false);

        hover_changed || selection_changed
    }

    pub fn pointer_left(&mut self) -> bool {
        if self.hovered.is_empty() {
            false
        } else {
            self.hovered.clear();
            true
        }
    }

    pub fn pointer_button(
        &mut self,
        point: Option<Point>,
        pressed: bool,
        extend_selection: bool,
        renderer: &mut GpuRenderer,
    ) -> PointerResult {
        if pressed
            && let Some(dismiss) = self.dismiss_regions.last().copied()
            && point.is_none_or(|point| !dismiss.contains(point))
        {
            self.selecting_input = None;
            let repaint = self.pressed.take().is_some();
            return PointerResult {
                repaint,
                clicked: None,
                dismissed: Some(DismissRequest {
                    id: dismiss.id,
                    restore_focus: dismiss.restore_focus,
                }),
            };
        }

        let region = point.and_then(|point| self.interactive_region_at(point));
        let target = region
            .filter(|region| region.clickable)
            .map(|region| region.id);
        if pressed {
            let focus_changed = region
                .filter(|region| region.focusable)
                .is_some_and(|region| self.focus(region.id));
            let mut selection_changed = false;
            self.selecting_input = None;
            if let (Some(point), Some(region)) = (point, region)
                && self.text_inputs.contains_key(&region.id)
                && let Some(input_region) = self
                    .text_input_regions
                    .iter()
                    .rev()
                    .find(|input| input.id == region.id)
                    .cloned()
            {
                let index = text_input_index_at(&input_region, point, self.scale_factor, renderer);
                selection_changed = self
                    .text_inputs
                    .get_mut(&region.id)
                    .is_some_and(|state| state.move_to(index, extend_selection));
                self.selecting_input = Some(region.id);
            }
            let repaint = self.pressed != target || focus_changed || selection_changed;
            self.pressed = target;
            PointerResult {
                repaint,
                clicked: None,
                dismissed: None,
            }
        } else {
            self.selecting_input = None;
            let clicked = self
                .pressed
                .filter(|pressed_id| Some(*pressed_id) == target);
            let repaint = self.pressed.take().is_some();
            PointerResult {
                repaint,
                clicked,
                dismissed: None,
            }
        }
    }

    /// Apply platform content-motion deltas to the deepest scrollable region under the pointer.
    pub fn scroll_at(&mut self, point: Option<Point>, delta: Vector) -> bool {
        let Some(point) = point else {
            return false;
        };
        let blocker = self
            .hit_regions
            .iter()
            .rev()
            .find(|region| region.blocks_pointer && region.contains(point))
            .map(|region| region.order);
        for region in self.scroll_regions.iter().rev() {
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
                return true;
            }
        }
        false
    }

    pub fn wants_pointer_cursor(&self, point: Point) -> bool {
        self.cursor_at(point, |region| region.cursor_pointer)
    }

    pub fn wants_text_cursor(&self, point: Point) -> bool {
        self.cursor_at(point, |region| region.cursor_text)
    }

    pub fn dismiss_topmost(&self) -> Option<DismissRequest> {
        self.dismiss_regions.last().map(|region| DismissRequest {
            id: region.id,
            restore_focus: region.restore_focus,
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
            if region.blocks_pointer {
                return None;
            }
        }
        None
    }

    fn cursor_at(&self, point: Point, cursor: impl Fn(HitRegion) -> bool) -> bool {
        for region in self.hit_regions.iter().rev() {
            if !region.contains(point) {
                continue;
            }
            if cursor(*region) {
                return true;
            }
            if region.blocks_pointer {
                return false;
            }
        }
        false
    }

    pub fn focused_text_input(&self) -> Option<ElementId> {
        self.focused
            .filter(|focused| self.text_inputs.contains_key(focused))
    }

    pub fn selected_text(&self) -> Option<Arc<str>> {
        self.focused_text_input()
            .and_then(|id| self.text_inputs.get(&id))
            .and_then(TextInputState::selected_text)
            .map(Arc::from)
    }

    pub fn input_move_left(&mut self, extend: bool) -> InputResult {
        self.edit_focused_input(|state| state.move_left(extend))
    }

    pub fn input_move_right(&mut self, extend: bool) -> InputResult {
        self.edit_focused_input(|state| state.move_right(extend))
    }

    pub fn input_move_home(&mut self, extend: bool) -> InputResult {
        self.edit_focused_input(|state| state.move_home(extend))
    }

    pub fn input_move_end(&mut self, extend: bool) -> InputResult {
        self.edit_focused_input(|state| state.move_end(extend))
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

    pub fn input_replace(&mut self, value: &str) -> InputResult {
        self.edit_focused_input(|state| state.replace_selection(value))
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

    pub fn input_set_accessibility_selection(
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
        self.edit_input(id, |state| {
            let anchor = state.accessibility_byte_index(selection.anchor.character_index);
            let caret = state.accessibility_byte_index(selection.focus.character_index);
            state.set_selection(anchor, caret)
        })
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
        let previous = state.shared_text();
        let repaint = edit(state);
        let change = (previous.as_ref() != state.text()).then(|| InputChange {
            id,
            value: state.shared_text(),
        });
        InputResult { repaint, change }
    }

    pub fn focused(&self) -> Option<ElementId> {
        self.focused
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
        if !self.focusable_ids.contains(&id) || self.focused == Some(id) {
            return false;
        }
        self.focused = Some(id);
        true
    }

    pub fn blur(&mut self) -> bool {
        self.focused.take().is_some()
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
        if self.focused == Some(next) {
            false
        } else {
            self.focused = Some(next);
            true
        }
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
        let mut nodes = Vec::with_capacity(self.seen_ids.len());
        let mut root = AccessibilityNode::new(Role::Window);
        root.set_bounds(accessibility_rect(Rect::from_size(self.viewport)));
        root.set_transform(Affine::scale(self.scale_factor as f64));
        root.set_label(window_title);
        if let Some(element) = &self.root {
            root.set_children([accessibility_id(element.runtime_id)]);
            build_accessibility_nodes(
                element,
                &self.element_bounds,
                &self.text_inputs,
                &self.accessibility_text_ids,
                &mut nodes,
            );
        }
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
        let Some(root) = &self.root else {
            return;
        };
        collect_dispatch_metadata(root, None, &mut self.parents, &mut self.key_contexts);
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
    renderer.text_index_for_x(
        TextId::new(region.id.value()),
        &region.content,
        &region.style,
        region.bounds.width,
        scale_factor,
        point.x - region.bounds.x + region.scroll_x,
    )
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

    let mut child_nodes = Vec::with_capacity(element.children.len());
    for (index, child) in element.children.iter_mut().enumerate() {
        child_nodes.push(build_layout_node(
            taffy,
            seen_ids,
            child,
            id,
            index,
            &element.resolved_typography,
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
        ElementKind::Canvas(_) => taffy.new_leaf(element.layout.clone())?,
        ElementKind::TextInput(input) => {
            let content = if input.value.is_empty() {
                input.placeholder.clone()
            } else {
                input.value.clone()
            };
            taffy.new_leaf_with_context(
                element.layout.clone(),
                MeasureContext::Text {
                    id: TextId::new(id.value()),
                    content,
                    style: element.resolved_typography.clone(),
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

    let is_scrollable = element.layout.overflow.x == Overflow::Scroll
        || element.layout.overflow.y == Overflow::Scroll;
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
fn paint_element(
    element: &Element,
    taffy: &TaffyTree<MeasureContext>,
    natural_bounds: &HashMap<ElementId, Rect>,
    element_bounds: &mut HashMap<ElementId, Rect>,
    scroll_offsets: &mut HashMap<ElementId, Vector>,
    hovered: &HashSet<ElementId>,
    pressed: Option<ElementId>,
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
    dismiss_regions: &mut Vec<DismissRegion>,
    #[cfg(target_os = "macos")] native_views: &mut Vec<NativeViewPlacement>,
    text_input_regions: &mut Vec<TextInputRegion>,
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
        let anchor_bounds =
            natural_bounds
                .get(&anchor.target)
                .copied()
                .ok_or(UiError::MissingAnchor {
                    element: element.runtime_id,
                    anchor: anchor.target,
                })?;
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
    let interaction_state = if pressed == Some(element.runtime_id) {
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
    let fill = interaction_state
        .background
        .or(focus_state.background)
        .or(element.visual.background)
        .unwrap_or(Color::TRANSPARENT);
    let border = interaction_state
        .border_color
        .or(focus_state.border_color)
        .or(element.visual.border_color)
        .unwrap_or(Color::TRANSPARENT);
    let border_width = interaction_state
        .border_width
        .or(focus_state.border_width)
        .unwrap_or(element.visual.border_width);
    let shadows = interaction_state
        .shadows
        .as_deref()
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

    if element.clickable
        || element.cursor_pointer
        || element.cursor_text
        || element.focusable
        || element.blocks_pointer
        || element.has_stateful_paint()
    {
        hit_regions.push(HitRegion {
            id: element.runtime_id,
            bounds,
            clip: parent_clip,
            clickable: element.clickable && !element.accessibility.disabled,
            focusable: element.focusable && !element.accessibility.disabled,
            cursor_pointer: element.cursor_pointer && !element.accessibility.disabled,
            cursor_text: element.cursor_text && !element.accessibility.disabled,
            stateful: element.has_stateful_paint(),
            blocks_pointer: element.blocks_pointer,
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
        .or(focus_state.text_color)
        .or(inherited_state_text_color);
    match &element.kind {
        ElementKind::Text(content) => {
            let mut style = element.resolved_typography.clone();
            if let Some(color) = state_text_color {
                style.color = color;
            }
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
        ElementKind::TextInput(input) => {
            let input_state = text_inputs
                .entry(element.runtime_id)
                .or_insert_with(|| TextInputState::new(&input.value));
            let mut style = element.resolved_typography.clone();
            if let Some(color) = state_text_color {
                style.color = color;
            }
            let vertical_inset = ((bounds.height - style.line_height) * 0.5).max(0.0);
            let text_viewport = bounds.inset(Insets {
                top: vertical_inset,
                right: 12.0,
                bottom: vertical_inset,
                left: 12.0,
            });
            if let Some(text_clip) = parent_clip.intersection(text_viewport) {
                let content = input_state.shared_text();
                let text_id = TextId::new(element.runtime_id.value());
                let is_focused = focused == Some(element.runtime_id);
                let caret_x = if content.is_empty() {
                    0.0
                } else {
                    renderer.text_caret_x(
                        text_id,
                        &content,
                        &style,
                        text_viewport.width,
                        scale_factor,
                        input_state.caret(),
                    )
                };
                let mut scroll_x = input_state.scroll_x();
                if is_focused {
                    let right_edge = (text_viewport.width - 2.0).max(0.0);
                    if caret_x - scroll_x > right_edge {
                        scroll_x = (caret_x - right_edge).max(0.0);
                    } else if caret_x < scroll_x {
                        scroll_x = caret_x.max(0.0);
                    }
                    input_state.set_scroll_x(scroll_x);
                }
                let caret_bounds = Rect::new(
                    text_viewport.x + caret_x - scroll_x,
                    text_viewport.y + 2.0,
                    1.5,
                    (text_viewport.height - 4.0).max(1.0),
                );

                let selection = input_state.selection();
                if is_focused && !selection.is_empty() && !content.is_empty() {
                    for (x, width) in renderer.text_selection_spans(
                        text_id,
                        &content,
                        &style,
                        text_viewport.width,
                        scale_factor,
                        selection.start,
                        selection.end,
                    ) {
                        scene.push_quad_in(
                            layer,
                            Quad::new(
                                Rect::new(
                                    text_viewport.x + x - scroll_x,
                                    text_viewport.y,
                                    width,
                                    text_viewport.height,
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
                    for (x, width) in renderer.text_selection_spans(
                        text_id,
                        &content,
                        &style,
                        text_viewport.width,
                        scale_factor,
                        marked.start,
                        marked.end,
                    ) {
                        scene.push_quad_in(
                            layer,
                            Quad::new(
                                Rect::new(
                                    text_viewport.x + x - scroll_x,
                                    text_viewport.bottom() - 1.5,
                                    width,
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
                    text_viewport.x - scroll_x,
                    text_viewport.y,
                    text_viewport.width + scroll_x,
                    text_viewport.height,
                );
                scene.push_text_in(
                    layer,
                    TextRun::new(text_id, display_text, text_bounds, display_style).clip(text_clip),
                );
                text_input_regions.push(TextInputRegion {
                    id: element.runtime_id,
                    bounds: text_viewport,
                    clip: text_clip,
                    content,
                    style,
                    scroll_x,
                    caret_bounds,
                });
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

    let is_scrollable = element.layout.overflow.x == Overflow::Scroll
        || element.layout.overflow.y == Overflow::Scroll;
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
        scroll_regions.push(ScrollRegion {
            id: element.runtime_id,
            bounds,
            max_offset,
            order,
        });
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
            dismiss_regions,
            #[cfg(target_os = "macos")]
            native_views,
            text_input_regions,
            child_origin,
            child_clip,
            viewport,
            layer,
            source_order,
            state_text_color,
        )?;
    }

    if is_scrollable && (scroll.y > 0.0 || layout.content_size.height > layout.size.height) {
        let content_height = layout.content_size.height.max(layout.size.height);
        if content_height > layout.size.height && layout.size.height > 0.0 {
            let thumb_height = (layout.size.height * layout.size.height / content_height)
                .max(24.0)
                .min(layout.size.height);
            let travel = layout.size.height - thumb_height;
            let max_scroll = (content_height - layout.size.height).max(1.0);
            let thumb_y = bounds.y + travel * (scroll.y / max_scroll);
            scene.push_quad_in(
                layer,
                Quad::new(
                    Rect::new(
                        bounds.right() - 6.0,
                        thumb_y + 2.0,
                        4.0,
                        (thumb_height - 4.0).max(4.0),
                    ),
                    Color::rgba8(142, 147, 160, 150),
                )
                .radius(2.0)
                .clip(child_clip),
            );
        }
    }
    Ok(())
}

#[derive(Clone, Copy)]
struct FocusCandidate {
    id: ElementId,
    tab_index: i16,
    order: usize,
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

fn collect_dispatch_metadata(
    element: &Element,
    parent: Option<ElementId>,
    parents: &mut HashMap<ElementId, ElementId>,
    key_contexts: &mut HashMap<ElementId, KeyContext>,
) {
    if let Some(parent) = parent {
        parents.insert(element.runtime_id, parent);
    }
    if let Some(context) = &element.key_context {
        key_contexts.insert(element.runtime_id, context.clone());
    }
    for child in &element.children {
        collect_dispatch_metadata(child, Some(element.runtime_id), parents, key_contexts);
    }
}

fn find_auto_focus(element: &Element) -> Option<ElementId> {
    if element.auto_focus && element.focusable && !element.accessibility.disabled {
        return Some(element.runtime_id);
    }
    element.children.iter().find_map(find_auto_focus)
}

fn build_accessibility_nodes(
    element: &Element,
    element_bounds: &HashMap<ElementId, Rect>,
    text_inputs: &HashMap<ElementId, TextInputState>,
    accessibility_text_ids: &HashMap<ElementId, AccessibilityNodeId>,
    nodes: &mut Vec<(AccessibilityNodeId, AccessibilityNode)>,
) {
    let Some(bounds) = element_bounds.get(&element.runtime_id).copied() else {
        return;
    };
    let mut node = AccessibilityNode::new(accessibility_role(element.accessibility.role));
    node.set_bounds(accessibility_rect(bounds));
    let mut children = element
        .children
        .iter()
        .map(|child| accessibility_id(child.runtime_id))
        .collect::<Vec<_>>();
    let text_input = text_inputs
        .get(&element.runtime_id)
        .zip(accessibility_text_ids.get(&element.runtime_id).copied());
    if let Some((_, text_id)) = text_input {
        children.push(text_id);
    }
    node.set_children(children);
    if let Some(label) = accessibility_label(element) {
        node.set_label(label);
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
    } else if let Some(value) = &element.accessibility.value {
        node.set_value(value.to_string());
    }
    if element.accessibility.disabled {
        node.set_disabled();
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
        build_accessibility_nodes(
            child,
            element_bounds,
            text_inputs,
            accessibility_text_ids,
            nodes,
        );
    }
}

fn accessibility_label(element: &Element) -> Option<String> {
    if let Some(label) = &element.accessibility.label {
        return Some(label.to_string());
    }
    if let ElementKind::Text(content) = &element.kind {
        return Some(content.to_string());
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
        AccessibilityRole::Dialog => Role::Dialog,
        AccessibilityRole::Menu => Role::Menu,
        AccessibilityRole::MenuItem => Role::MenuItem,
        AccessibilityRole::Tooltip => Role::Tooltip,
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
            .and_modify(|state| state.sync_external(&input.value))
            .or_insert_with(|| TextInputState::new(&input.value));
    }
    for child in &element.children {
        sync_text_inputs(child, inputs, ids);
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
    use crate::{
        AnimatedImageFrame, AnimationRepeat, FocusHandle, Image, PathBuilder, button, div, text,
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
    fn overlay_pointer_blockers_hide_lower_layer_cursors_and_targets() {
        let mut tree = UiTree::new();
        let bounds = Rect::new(0.0, 0.0, 100.0, 100.0);
        tree.hit_regions.push(HitRegion {
            id: ElementId::new(1),
            bounds,
            clip: bounds,
            clickable: true,
            focusable: true,
            cursor_pointer: true,
            cursor_text: false,
            stateful: false,
            blocks_pointer: false,
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
            focusable: false,
            cursor_pointer: false,
            cursor_text: false,
            stateful: false,
            blocks_pointer: true,
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

    fn assign_runtime_ids(element: &mut Element) {
        if let Some(id) = element.explicit_id {
            element.runtime_id = id;
        }
        for child in &mut element.children {
            assign_runtime_ids(child);
        }
    }
}
