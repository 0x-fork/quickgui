use super::*;

pub(super) fn fixed_text_layout_size(
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

pub(super) fn absolute_length(dimension: Dimension) -> Option<f32> {
    (dimension.tag() == CompactLength::LENGTH_TAG).then(|| dimension.value().max(0.0))
}

pub(super) fn compute_detached_layout(
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
                    // For a leaf with padding or a border, Taffy passes its assigned border-box
                    // width in `known` while the definite available width has already had those
                    // insets removed. Text shaping is content-box work, so using `known.width`
                    // here makes layout count fewer lines than paint and lets the final line cross
                    // a grid-row border during resize.
                    let max_width = match available.width {
                        AvailableSpace::Definite(width) => Some(width.max(0.0)),
                        AvailableSpace::MinContent => Some(0.0),
                        AvailableSpace::MaxContent => None,
                    };
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
                        width: measured.width,
                        height: measured.height,
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
pub(super) fn compute_container_query_child_layouts(
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
pub(super) fn mark_layout_nodes_dirty(
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

pub(super) fn finite_layout_length(value: f32) -> f32 {
    if value.is_finite() {
        value.max(0.0)
    } else {
        0.0
    }
}

pub(super) fn validate_container_query_limits(root: &Element) -> Result<(), UiError> {
    pub(super) fn visit(
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

pub(super) fn container_queries_need_resolution(
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

pub(super) struct ContainerQueryResolveContext<'a, F>
where
    F: FnMut(&mut Element),
{
    pub(super) taffy: &'a TaffyTree<MeasureContext>,
    pub(super) prepare: &'a mut F,
    pub(super) animations: &'a mut HashMap<ElementId, DeclarativeAnimationPlayback>,
    pub(super) motion_ids: &'a mut HashSet<ElementId>,
    pub(super) time_animation_ids: &'a mut HashSet<ElementId>,
    pub(super) springs: &'a mut HashMap<ElementId, DeclarativeSpringPlayback>,
    pub(super) spring_ids: &'a mut HashSet<ElementId>,
    pub(super) request_frame: &'a mut bool,
    pub(super) deadline: &'a mut Option<Instant>,
    pub(super) now: Instant,
    pub(super) animation_epoch: Instant,
    pub(super) enabled: bool,
    pub(super) reduce_motion: bool,
    pub(super) sanitize_detached: bool,
}

#[derive(Clone, Copy, Debug, Default)]
pub(super) struct ContainerQueryResolution {
    pub(super) changed: bool,
    pub(super) replaced_existing_subtree: bool,
}

impl ContainerQueryResolution {
    pub(super) fn merge(&mut self, other: Self) {
        self.changed |= other.changed;
        self.replaced_existing_subtree |= other.replaced_existing_subtree;
    }
}

impl<F> ContainerQueryResolveContext<'_, F>
where
    F: FnMut(&mut Element),
{
    pub(super) fn resolve(
        &mut self,
        element: &mut Element,
    ) -> Result<ContainerQueryResolution, UiError> {
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

pub(super) fn drain_container_query_motion_ids(element: &mut Element, ids: &mut Vec<ElementId>) {
    if let ElementKind::ContainerQuery(query) = &mut element.kind {
        ids.append(&mut query.resolved_motion_ids);
    }
    for child in &mut element.children {
        drain_container_query_motion_ids(child, ids);
    }
}

pub(super) fn sanitize_detached_element(element: &mut Element, preserve_motion: bool) {
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
    element.focus_on_pointer = true;
    element.focus_trap = false;
    element.restore_previous_focus = false;
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

pub(super) fn text_input_max_scroll(
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

pub(super) fn scroll_to_reveal_caret(
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

pub(super) fn text_input_index_at(
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

pub(super) fn squared_distance_to_rect(point: Point, rect: Rect) -> f32 {
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

pub(super) fn nearest_selectable_text_region(
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

pub(super) fn selection_separator(previous: Option<Rect>, next: Option<Rect>) -> &'static str {
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

pub(super) fn push_bounded_text(output: &mut String, value: &str, limit: usize) {
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

pub(super) fn measure_image(
    width: Option<f32>,
    height: Option<f32>,
    intrinsic: Size,
) -> TaffySize<f32> {
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

pub(super) fn fit_path(bounds: Rect, path: &Path, fit: ObjectFit) -> Option<([f32; 2], Vector)> {
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

pub(super) fn build_pending_container_query_subtrees(
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

pub(super) fn build_layout_node(
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

pub(super) fn apply_scroll_end_revision(
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

pub(super) fn collect_layout_bounds(
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

/// Commit variable-list viewport and row measurements from a completed layout.
///
/// This deliberately runs before paint. Measuring while recursively painting a row is too late:
/// its parent has already resolved the retained scroll translation, which can present a stale
/// bottom anchor for one frame when wrapped content changes height during resize.
pub(super) fn report_variable_list_layout_measurements(
    element: &Element,
    taffy: &TaffyTree<MeasureContext>,
) -> Result<bool, UiError> {
    if element.is_display_none() || element.is_visibility_hidden() {
        return Ok(false);
    }
    let node = element
        .taffy_node
        .expect("layout nodes are assigned before list measurement");
    let layout = taffy.layout(node)?;
    let mut changed = false;
    if let Some(virtual_scroll) = &element.virtual_scroll {
        changed |= virtual_scroll
            .handle
            .report_viewport(Size::new(layout.size.width, layout.size.height));
    }
    if let Some(measurement) = &element.list_item_measurement {
        changed |= measurement.report_height(layout.size.height);
    }
    for child in &element.children {
        changed |= report_variable_list_layout_measurements(child, taffy)?;
    }
    Ok(changed)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum AnchorSide {
    Top,
    Bottom,
    Left,
    Right,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum AnchorAlign {
    Start,
    Center,
    End,
}

pub(super) fn place_anchored(
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

pub(super) fn anchor_placement_parts(placement: AnchorPlacement) -> (AnchorSide, AnchorAlign) {
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

pub(super) fn opposite_anchor_side(side: AnchorSide) -> AnchorSide {
    match side {
        AnchorSide::Top => AnchorSide::Bottom,
        AnchorSide::Bottom => AnchorSide::Top,
        AnchorSide::Left => AnchorSide::Right,
        AnchorSide::Right => AnchorSide::Left,
    }
}

pub(super) fn available_anchor_space(
    anchor: Rect,
    viewport: Rect,
    side: AnchorSide,
    gap: f32,
) -> f32 {
    match side {
        AnchorSide::Top => anchor.y - viewport.y - gap,
        AnchorSide::Bottom => viewport.bottom() - anchor.bottom() - gap,
        AnchorSide::Left => anchor.x - viewport.x - gap,
        AnchorSide::Right => viewport.right() - anchor.right() - gap,
    }
    .max(0.0)
}

pub(super) fn anchored_origin(
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

pub(super) fn cross_axis_overflow(
    origin: Point,
    size: Size,
    viewport: Rect,
    side: AnchorSide,
) -> f32 {
    match side {
        AnchorSide::Top | AnchorSide::Bottom => {
            (viewport.x - origin.x).max(0.0) + (origin.x + size.width - viewport.right()).max(0.0)
        }
        AnchorSide::Left | AnchorSide::Right => {
            (viewport.y - origin.y).max(0.0) + (origin.y + size.height - viewport.bottom()).max(0.0)
        }
    }
}

pub(super) fn clamp_surface_axis(origin: f32, size: f32, minimum: f32, maximum: f32) -> f32 {
    if size >= maximum - minimum {
        minimum
    } else {
        origin.clamp(minimum, maximum - size)
    }
}

#[allow(clippy::too_many_arguments)]
pub(super) fn push_element_shadows(
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

pub(super) fn own_text_clip(element: &Element, bounds: Rect, parent_clip: Rect) -> Rect {
    let clips_own_text = element.resolved_typography.line_clamp.is_some()
        || element.layout.overflow.x != Overflow::Visible
        || element.layout.overflow.y != Overflow::Visible;
    if clips_own_text {
        parent_clip.intersection(bounds).unwrap_or(Rect::ZERO)
    } else {
        parent_clip
    }
}

/// Return the box Cosmic Text must use for a text leaf.
///
/// Taffy measures leaf content before adding its resolved padding and border. Painting with the
/// border box would therefore give Cosmic Text a different width from layout: padded wrapped text
/// could alternate between the two line breaks on consecutive resize frames and spill into a
/// neighboring grid cell. Keep shaping, selection, decorations, and paint in the same content box.
pub(super) fn text_content_bounds(bounds: Rect, layout: &taffy::tree::Layout) -> Rect {
    let left = layout.border.left + layout.padding.left;
    let right = layout.border.right + layout.padding.right;
    let top = layout.border.top + layout.padding.top;
    let bottom = layout.border.bottom + layout.padding.bottom;
    Rect::new(
        bounds.x + left,
        bounds.y + top,
        (bounds.width - left - right).max(0.0),
        (bounds.height - top - bottom).max(0.0),
    )
}

pub(super) fn element_has_outset_shadow(element: &Element) -> bool {
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
