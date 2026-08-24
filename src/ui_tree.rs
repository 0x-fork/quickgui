use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use taffy::{
    geometry::Size as TaffySize,
    prelude::{AvailableSpace, NodeId, TaffyTree},
    style::Overflow,
};
use thiserror::Error;

use crate::{
    Color, Element, ElementId, Point, Quad, Rect, Scene, Size, TextId, TextRun, TextStyle, Vector,
    element::ElementKind, renderer::GpuRenderer,
};

#[derive(Debug, Error)]
pub(crate) enum UiError {
    #[error("flexbox layout failed: {0}")]
    Layout(#[from] taffy::TaffyError),
    #[error("element id {0:?} appears more than once in the same view")]
    DuplicateId(ElementId),
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
    cursor_pointer: bool,
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

pub(crate) struct UiTree {
    root: Option<Element>,
    taffy: TaffyTree<MeasureContext>,
    root_node: Option<NodeId>,
    seen_ids: HashSet<ElementId>,
    scroll_offsets: HashMap<ElementId, Vector>,
    hit_regions: Vec<HitRegion>,
    scroll_regions: Vec<ScrollRegion>,
    hovered: HashSet<ElementId>,
    pressed: Option<ElementId>,
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
            hovered: HashSet::with_capacity(8),
            pressed: None,
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
        self.scroll_offsets
            .retain(|id, _| self.seen_ids.contains(id));
        self.hovered.retain(|id| self.seen_ids.contains(id));
        if self.pressed.is_some_and(|id| !self.seen_ids.contains(&id)) {
            self.pressed = None;
        }
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

    pub fn paint(&mut self, scene: &mut Scene) -> Result<(), UiError> {
        self.hit_regions.clear();
        self.scroll_regions.clear();
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
            scene,
            &mut self.hit_regions,
            &mut self.scroll_regions,
            Point::ZERO,
            viewport,
            None,
        )?;
        Ok(())
    }

    /// Returns true when paint-only hover state changed.
    pub fn pointer_moved(&mut self, point: Point) -> bool {
        let mut next = HashSet::with_capacity(self.hovered.capacity().max(4));
        for region in &self.hit_regions {
            if region.stateful && region.contains(point) {
                next.insert(region.id);
            }
        }
        if next == self.hovered {
            false
        } else {
            self.hovered = next;
            true
        }
    }

    pub fn pointer_left(&mut self) -> bool {
        if self.hovered.is_empty() {
            false
        } else {
            self.hovered.clear();
            true
        }
    }

    pub fn pointer_button(&mut self, point: Option<Point>, pressed: bool) -> PointerResult {
        let target = point.and_then(|point| {
            self.hit_regions
                .iter()
                .rev()
                .find(|region| region.clickable && region.contains(point))
                .map(|region| region.id)
        });
        if pressed {
            let repaint = self.pressed != target;
            self.pressed = target;
            PointerResult {
                repaint,
                clicked: None,
            }
        } else {
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
    scene: &mut Scene,
    hit_regions: &mut Vec<HitRegion>,
    scroll_regions: &mut Vec<ScrollRegion>,
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

    let state = if pressed == Some(element.runtime_id) {
        element.active
    } else if hovered.contains(&element.runtime_id) {
        element.hover
    } else {
        Default::default()
    };
    let fill = state
        .background
        .or(element.visual.background)
        .unwrap_or(Color::TRANSPARENT);
    let border = state
        .border_color
        .or(element.visual.border_color)
        .unwrap_or(Color::TRANSPARENT);
    if fill.a > 0.0 || (border.a > 0.0 && element.visual.border_width > 0.0) {
        scene.push_quad(
            Quad::new(bounds, fill)
                .radius(element.visual.radius)
                .border(element.visual.border_width, border)
                .clip(parent_clip),
        );
    }

    if element.clickable || element.cursor_pointer || element.has_stateful_paint() {
        hit_regions.push(HitRegion {
            id: element.runtime_id,
            bounds,
            clip: parent_clip,
            clickable: element.clickable,
            cursor_pointer: element.cursor_pointer,
            stateful: element.has_stateful_paint(),
        });
    }

    let state_text_color = state.text_color.or(inherited_state_text_color);
    if let ElementKind::Text(content) = &element.kind {
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
            scene,
            hit_regions,
            scroll_regions,
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
        return Err(UiError::DuplicateId(id));
    }
    for child in &element.children {
        collect_explicit_ids(child, ids)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{div, text};

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
}
