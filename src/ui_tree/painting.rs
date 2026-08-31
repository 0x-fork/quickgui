use super::*;

#[allow(clippy::too_many_arguments)]
pub(super) fn paint_selectable_text(
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
pub(super) fn collect_inspector_nodes(
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

pub(super) fn element_hit_region(
    element: &Element,
    bounds: Rect,
    clip: Rect,
    order: PaintOrder,
    selectable_text: bool,
) -> Option<HitRegion> {
    if !(element.clickable
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
        || selectable_text
        || element.focusable
        || element.blocks_pointer
        || element.app_region.is_some()
        || element.has_stateful_paint()
        || element.has_stateful_cursor())
    {
        return None;
    }

    Some(HitRegion {
        id: element.runtime_id,
        bounds: expand_hit_bounds(bounds, element.hit_slop),
        clip,
        clickable: element.clickable && !element.accessibility.disabled,
        pointer_listener: element.pointer_listener && !element.accessibility.disabled,
        drag_source: element.drag_source && !element.accessibility.disabled,
        drop_target: element.drop_target && !element.accessibility.disabled,
        focusable: element.focusable && element.focus_on_pointer && !element.accessibility.disabled,
        cursor_style: effective_cursor_style(element, selectable_text),
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
    })
}

#[allow(clippy::too_many_arguments)]
pub(super) fn collect_layout_hit_regions(
    element: &Element,
    taffy: &TaffyTree<MeasureContext>,
    natural_bounds: &HashMap<ElementId, Rect>,
    scroll_offsets: &mut HashMap<ElementId, Vector>,
    selectable_text_indices: &HashMap<ElementId, usize>,
    hit_regions: &mut Vec<HitRegion>,
    parent_origin: Point,
    parent_clip: Rect,
    viewport: Rect,
    parent_layer: PaintLayerKey,
    source_order: &mut usize,
) -> Result<(), UiError> {
    if element.is_display_none() || element.is_visibility_hidden() {
        return Ok(());
    }
    let node = element
        .taffy_node
        .expect("layout nodes are assigned before hit testing");
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
    let hit_bounds = expand_hit_bounds(bounds, element.hit_slop);
    let effective_parent_clip = if element.portal {
        viewport
    } else {
        parent_clip
    };
    if element.children.is_empty()
        && effective_parent_clip.intersection(bounds).is_none()
        && effective_parent_clip.intersection(hit_bounds).is_none()
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
    let parent_clip = effective_parent_clip;
    if let Some(region) = element_hit_region(
        element,
        bounds,
        parent_clip,
        order,
        selectable_text_indices.contains_key(&element.runtime_id),
    ) {
        hit_regions.push(region);
    }

    let clips_children = element.layout.overflow.x != Overflow::Visible
        || element.layout.overflow.y != Overflow::Visible;
    let child_clip = if clips_children {
        let Some(clip) = parent_clip.intersection(bounds) else {
            return Ok(());
        };
        clip
    } else {
        parent_clip
    };

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
        collect_layout_hit_regions(
            child,
            taffy,
            natural_bounds,
            scroll_offsets,
            selectable_text_indices,
            hit_regions,
            child_origin,
            child_clip,
            viewport,
            layer,
            source_order,
        )?;
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub(super) fn paint_element(
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
    let hit_bounds = expand_hit_bounds(bounds, element.hit_slop);

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
        && effective_parent_clip.intersection(hit_bounds).is_none()
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
    if let Some(region) = element_hit_region(
        element,
        bounds,
        parent_clip,
        order,
        selectable_document_index.is_some(),
    ) {
        hit_regions.push(region);
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
            let text_bounds = text_content_bounds(bounds, layout);
            let text_clip = own_text_clip(element, bounds, parent_clip);
            let decorations = if style.has_decorations() && !text_bounds.is_empty() {
                text_clip.intersection(text_bounds).map(|clip| {
                    let visible_y = (clip.y - text_bounds.y).max(0.0)
                        ..(clip.bottom() - text_bounds.y).min(text_bounds.height);
                    renderer
                        .text_geometry(
                            TextId::new(element.runtime_id.value()),
                            content,
                            &style,
                            None,
                            text_bounds.width,
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
                text_bounds,
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
                    text_bounds,
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
                        Point::new(text_bounds.x, text_bounds.y),
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
            let text_bounds = text_content_bounds(bounds, layout);
            let text_clip = own_text_clip(element, bounds, parent_clip);
            if !text_bounds.is_empty()
                && let Some(clip) = text_clip.intersection(text_bounds)
            {
                let visible_y = (clip.y - text_bounds.y).max(0.0)
                    ..(clip.bottom() - text_bounds.y).min(text_bounds.height);
                let geometry = renderer.text_geometry(
                    text_id,
                    styled.content(),
                    &style,
                    Some(&highlights),
                    text_bounds.width,
                    scale_factor,
                    visible_y,
                );
                for background in geometry.backgrounds {
                    scene.push_quad_in(
                        layer,
                        Quad::new(
                            Rect::new(
                                text_bounds.x + background.rect.x,
                                text_bounds.y + background.rect.y,
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
                    text_bounds,
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
                    TextRun::new(text_id, styled.content().clone(), text_bounds, style)
                        .with_highlights(highlights)
                        .clip(text_clip),
                );
                for decoration in geometry.decorations {
                    push_text_paint_rect(
                        scene,
                        layer,
                        decoration,
                        Point::new(text_bounds.x, text_bounds.y),
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

pub(super) fn push_text_paint_rect(
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

pub(super) fn effective_cursor_style(
    element: &Element,
    selectable_text: bool,
) -> Option<CursorStyle> {
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

pub(super) fn paint_vertical_scrollbar(
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

pub(super) fn vertical_scrollbar_geometry(
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
