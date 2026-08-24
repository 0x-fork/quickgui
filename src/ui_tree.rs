use std::collections::{HashMap, HashSet};
use std::sync::Arc;

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
    AccessibilityRole, Color, Element, ElementId, Insets, Point, Quad, Rect, Scene, Size, TextId,
    TextRun, TextStyle, Vector, element::ElementKind, renderer::GpuRenderer,
    text_input::TextInputState,
};

const ACCESSIBILITY_ROOT_ID: AccessibilityNodeId = AccessibilityNodeId(u64::MAX);

#[derive(Debug, Error)]
pub(crate) enum UiError {
    #[error("flexbox layout failed: {0}")]
    Layout(#[from] taffy::TaffyError),
    #[error("element id {0:?} appears more than once in the same view")]
    DuplicateId(ElementId),
    #[error("element id {0:?} is reserved by the accessibility root")]
    ReservedId(ElementId),
}

#[derive(Clone)]
struct MeasureContext {
    id: TextId,
    content: Arc<str>,
    style: TextStyle,
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
    hit_regions: Vec<HitRegion>,
    scroll_regions: Vec<ScrollRegion>,
    text_input_regions: Vec<TextInputRegion>,
    text_inputs: HashMap<ElementId, TextInputState>,
    accessibility_text_ids: HashMap<ElementId, AccessibilityNodeId>,
    next_accessibility_text_id: u64,
    selecting_input: Option<ElementId>,
    hovered: HashSet<ElementId>,
    pressed: Option<ElementId>,
    focused: Option<ElementId>,
    focusable_ids: HashSet<ElementId>,
    clickable_ids: HashSet<ElementId>,
    focus_order: Vec<ElementId>,
    focus_initialized: bool,
    viewport: Size,
    scale_factor: f32,
}

impl UiTree {
    pub fn new() -> Self {
        Self {
            root: None,
            taffy: TaffyTree::with_capacity(256),
            root_node: None,
            seen_ids: HashSet::with_capacity(256),
            scroll_offsets: HashMap::new(),
            hit_regions: Vec::with_capacity(128),
            scroll_regions: Vec::with_capacity(8),
            text_input_regions: Vec::with_capacity(8),
            text_inputs: HashMap::with_capacity(8),
            accessibility_text_ids: HashMap::with_capacity(8),
            next_accessibility_text_id: ACCESSIBILITY_ROOT_ID.0 - 1,
            selecting_input: None,
            hovered: HashSet::with_capacity(8),
            pressed: None,
            focused: None,
            focusable_ids: HashSet::with_capacity(32),
            clickable_ids: HashSet::with_capacity(32),
            focus_order: Vec::with_capacity(32),
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
                let max_width = known.width.or_else(|| match available.width {
                    AvailableSpace::Definite(width) => Some(width.max(0.0)),
                    AvailableSpace::MinContent => Some(0.0),
                    AvailableSpace::MaxContent => None,
                });
                let measured = renderer.measure_text(
                    context.id,
                    &context.content,
                    &context.style,
                    max_width,
                    scale_factor,
                );
                TaffySize {
                    width: known.width.unwrap_or(measured.width),
                    height: known.height.unwrap_or(measured.height),
                }
            },
        )?;
        Ok(())
    }

    pub fn paint(&mut self, scene: &mut Scene, renderer: &mut GpuRenderer) -> Result<(), UiError> {
        self.hit_regions.clear();
        self.scroll_regions.clear();
        self.text_input_regions.clear();
        let Some(root) = &self.root else {
            return Ok(());
        };
        let viewport = Rect::from_size(self.viewport);
        paint_element(
            root,
            &self.taffy,
            &mut self.scroll_offsets,
            &self.hovered,
            self.pressed,
            self.focused,
            self.scale_factor,
            scene,
            renderer,
            &mut self.text_inputs,
            &mut self.hit_regions,
            &mut self.scroll_regions,
            &mut self.text_input_regions,
            Point::ZERO,
            viewport,
            None,
        )?;
        Ok(())
    }

    /// Returns true when paint-only hover state changed.
    pub fn pointer_moved(&mut self, point: Point, renderer: &mut GpuRenderer) -> bool {
        let mut next = HashSet::with_capacity(self.hovered.capacity().max(4));
        for region in &self.hit_regions {
            if region.stateful && region.contains(point) {
                next.insert(region.id);
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
        let region = point.and_then(|point| {
            self.hit_regions
                .iter()
                .rev()
                .find(|region| (region.clickable || region.focusable) && region.contains(point))
                .copied()
        });
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
            }
        } else {
            self.selecting_input = None;
            let clicked = self
                .pressed
                .filter(|pressed_id| Some(*pressed_id) == target);
            let repaint = self.pressed.take().is_some();
            PointerResult { repaint, clicked }
        }
    }

    /// Apply platform content-motion deltas to the deepest scrollable region under the pointer.
    pub fn scroll_at(&mut self, point: Option<Point>, delta: Vector) -> bool {
        let Some(point) = point else {
            return false;
        };
        for region in self.scroll_regions.iter().rev() {
            if !region.bounds.contains(point) {
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
        self.hit_regions
            .iter()
            .rev()
            .any(|region| region.cursor_pointer && region.contains(point))
    }

    pub fn wants_text_cursor(&self, point: Point) -> bool {
        self.hit_regions
            .iter()
            .rev()
            .any(|region| region.cursor_text && region.contains(point))
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
                &self.taffy,
                &self.scroll_offsets,
                &self.text_inputs,
                &self.accessibility_text_ids,
                &mut nodes,
                Point::ZERO,
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
}

impl Default for UiTree {
    fn default() -> Self {
        Self::new()
    }
}

pub(crate) struct PointerResult {
    pub repaint: bool,
    pub clicked: Option<ElementId>,
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
            MeasureContext {
                id: TextId::new(id.value()),
                content: content.clone(),
                style: element.resolved_typography.clone(),
            },
        )?,
        ElementKind::TextInput(input) => {
            let content = if input.value.is_empty() {
                input.placeholder.clone()
            } else {
                input.value.clone()
            };
            taffy.new_leaf_with_context(
                element.layout.clone(),
                MeasureContext {
                    id: TextId::new(id.value()),
                    content,
                    style: element.resolved_typography.clone(),
                },
            )?
        }
    };
    element.taffy_node = Some(node);
    Ok(node)
}

#[allow(clippy::too_many_arguments)]
fn paint_element(
    element: &Element,
    taffy: &TaffyTree<MeasureContext>,
    scroll_offsets: &mut HashMap<ElementId, Vector>,
    hovered: &HashSet<ElementId>,
    pressed: Option<ElementId>,
    focused: Option<ElementId>,
    scale_factor: f32,
    scene: &mut Scene,
    renderer: &mut GpuRenderer,
    text_inputs: &mut HashMap<ElementId, TextInputState>,
    hit_regions: &mut Vec<HitRegion>,
    scroll_regions: &mut Vec<ScrollRegion>,
    text_input_regions: &mut Vec<TextInputRegion>,
    parent_origin: Point,
    parent_clip: Rect,
    inherited_state_text_color: Option<Color>,
) -> Result<(), UiError> {
    let node = element
        .taffy_node
        .expect("layout nodes are assigned before paint");
    let layout = taffy.layout(node)?;
    let bounds = Rect::new(
        parent_origin.x + layout.location.x,
        parent_origin.y + layout.location.y,
        layout.size.width,
        layout.size.height,
    );

    let interaction_state = if pressed == Some(element.runtime_id) {
        element.active
    } else if hovered.contains(&element.runtime_id) {
        element.hover
    } else {
        Default::default()
    };
    let focus_state = if focused == Some(element.runtime_id) {
        element.focus
    } else {
        Default::default()
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
    if fill.a > 0.0 || (border.a > 0.0 && border_width > 0.0) {
        scene.push_quad(
            Quad::new(bounds, fill)
                .radius(element.visual.radius)
                .border(border_width, border)
                .clip(parent_clip),
        );
    }

    if element.clickable
        || element.cursor_pointer
        || element.focusable
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
            scene.push_text(
                TextRun::new(
                    TextId::new(element.runtime_id.value()),
                    content.clone(),
                    bounds,
                    style,
                )
                .clip(parent_clip),
            );
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
                        scene.push_quad(
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
                    scene.push_quad(
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
                        scene.push_quad(
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
                scene.push_text(
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
        });
    }

    let child_origin = Point::new(bounds.x - scroll.x, bounds.y - scroll.y);
    for child in &element.children {
        paint_element(
            child,
            taffy,
            scroll_offsets,
            hovered,
            pressed,
            focused,
            scale_factor,
            scene,
            renderer,
            text_inputs,
            hit_regions,
            scroll_regions,
            text_input_regions,
            child_origin,
            child_clip,
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
            scene.push_quad(
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

fn find_auto_focus(element: &Element) -> Option<ElementId> {
    if element.auto_focus && element.focusable && !element.accessibility.disabled {
        return Some(element.runtime_id);
    }
    element.children.iter().find_map(find_auto_focus)
}

fn build_accessibility_nodes(
    element: &Element,
    taffy: &TaffyTree<MeasureContext>,
    scroll_offsets: &HashMap<ElementId, Vector>,
    text_inputs: &HashMap<ElementId, TextInputState>,
    accessibility_text_ids: &HashMap<ElementId, AccessibilityNodeId>,
    nodes: &mut Vec<(AccessibilityNodeId, AccessibilityNode)>,
    parent_origin: Point,
) {
    let Some(node_id) = element.taffy_node else {
        return;
    };
    let Ok(layout) = taffy.layout(node_id) else {
        return;
    };
    let bounds = Rect::new(
        parent_origin.x + layout.location.x,
        parent_origin.y + layout.location.y,
        layout.size.width,
        layout.size.height,
    );
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

    let scroll = scroll_offsets
        .get(&element.runtime_id)
        .copied()
        .unwrap_or_default();
    let child_origin = Point::new(bounds.x - scroll.x, bounds.y - scroll.y);
    for child in &element.children {
        build_accessibility_nodes(
            child,
            taffy,
            scroll_offsets,
            text_inputs,
            accessibility_text_ids,
            nodes,
            child_origin,
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
    use crate::{button, div, text};

    #[test]
    fn generated_ids_are_path_stable() {
        assert_eq!(mix_id(42, 3), mix_id(42, 3));
        assert_ne!(mix_id(42, 3), mix_id(42, 4));
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
    fn controls_derive_accessible_names_from_text_children() {
        let control = button().child("Save changes");
        assert_eq!(
            accessibility_label(&control).as_deref(),
            Some("Save changes")
        );
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
