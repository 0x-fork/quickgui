use super::*;

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
fn retained_paint_hover_tracks_rebuilt_layout_under_a_stationary_pointer() {
    let mut tree = UiTree::new();
    let plus = ElementId::new(90);
    let inserted_tab = ElementId::new(91);
    let original_bounds = Rect::new(0.0, 0.0, 32.0, 32.0);
    let moved_bounds = Rect::new(80.0, 0.0, 32.0, 32.0);
    let region = |id, bounds, source| HitRegion {
        id,
        bounds,
        clip: bounds,
        clickable: true,
        pointer_listener: false,
        drag_source: false,
        drop_target: false,
        focusable: true,
        cursor_style: None,
        cursor_states: CursorStateStyles::default(),
        stateful: true,
        blocks_pointer: true,
        app_region: None,
        order: PaintOrder {
            layer: PaintLayerKey::default(),
            source,
        },
    };
    let pointer = Point::new(16.0, 16.0);

    tree.hit_regions.push(region(plus, original_bounds, 0));
    assert!(tree.refresh_retained_hover(Some(pointer)));
    assert!(tree.hovered.contains(&plus));
    assert!(!tree.refresh_retained_hover(Some(pointer)));

    tree.hit_regions.clear();
    tree.hit_regions.push(region(plus, moved_bounds, 0));
    tree.hit_regions
        .push(region(inserted_tab, original_bounds, 1));
    assert!(tree.refresh_retained_hover(Some(pointer)));
    assert!(!tree.hovered.contains(&plus));
    assert!(tree.hovered.contains(&inserted_tab));
}

#[test]
fn layout_hit_testing_clears_moved_hover_before_paint() {
    let plus = ElementId::new(92);
    let inserted_tab = ElementId::new(93);
    let hover_color = Color::rgb8(255, 0, 0);
    let declaration = |insert_tab| {
        let plus_button = button()
            .id(plus)
            .size(32.0, 32.0)
            .flex_none()
            .clickable()
            .hover(|style| style.bg(hover_color))
            .transition(Transition::colors(Duration::from_millis(70)));
        let mut children = Vec::with_capacity(2);
        if insert_tab {
            children.push(
                button()
                    .id(inserted_tab)
                    .size(48.0, 32.0)
                    .flex_none()
                    .clickable(),
            );
        }
        children.push(plus_button);
        div().size(112.0, 32.0).flex_row().children(children)
    };
    let viewport = Size::new(112.0, 32.0);
    let pointer = Point::new(16.0, 16.0);
    let started = Instant::now();
    let mut tree = UiTree::new();
    let mut renderer = TestTextLayout;
    let mut scene = Scene::new();

    tree.set_root(declaration(false), viewport, 1.0, &mut renderer)
        .unwrap();
    tree.paint_at(&mut scene, &mut renderer, started).unwrap();
    assert!(tree.pointer_moved(pointer, &mut renderer));
    scene.clear(Color::TRANSPARENT);
    tree.paint_at(&mut scene, &mut renderer, started).unwrap();
    scene.clear(Color::TRANSPARENT);
    tree.paint_at(
        &mut scene,
        &mut renderer,
        started + Duration::from_millis(70),
    )
    .unwrap();
    assert_eq!(scene.quads().len(), 1);
    assert_eq!(scene.quads()[0].fill, hover_color);

    tree.set_root(declaration(true), viewport, 1.0, &mut renderer)
        .unwrap();
    assert!(tree.refresh_hover_after_layout(Some(pointer)).unwrap());
    assert!(!tree.hovered.contains(&plus));
    assert!(!tree.style_transitions.contains_key(&plus));

    scene.clear(Color::TRANSPARENT);
    tree.paint_at(
        &mut scene,
        &mut renderer,
        started + Duration::from_millis(71),
    )
    .unwrap();
    assert!(scene.quads().is_empty());
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
fn variable_list_relayout_commits_tail_measurement_before_first_paint() {
    let list = crate::ListState::new(1, 50.0)
        .with_overscan(0)
        .with_follow_mode(crate::FollowMode::Tail);
    list.set_viewport_size(100.0, 40.0);
    let row = ElementId::named("width-dependent-tail-row");
    let rows = list.render_rows(list.visible_rows().range, |_| {
        div().id(row).w_full().aspect_ratio(2.0)
    });
    let root = div()
        .relative()
        .size_full()
        .overflow_hidden()
        .variable_virtual_scroll(&list)
        .child(rows);
    let mut tree = UiTree::new();
    let mut renderer = TestTextLayout;
    tree.set_root(root, Size::new(100.0, 40.0), 1.0, &mut renderer)
        .unwrap();

    let mut scene = Scene::new();
    tree.paint(&mut scene, &mut renderer).unwrap();
    assert_eq!(list.scroll_offset(), 10.0);
    assert_eq!(
        tree.element_bounds(row),
        Some(Rect::new(0.0, -10.0, 100.0, 50.0))
    );

    tree.relayout_with_prepare(Size::new(200.0, 40.0), 1.0, &mut renderer, |_| {})
        .unwrap();
    scene.clear(Color::TRANSPARENT);
    tree.paint(&mut scene, &mut renderer).unwrap();

    assert_eq!(list.scroll_offset(), 60.0);
    assert_eq!(
        tree.element_bounds(row),
        Some(Rect::new(0.0, -60.0, 200.0, 100.0)),
        "the first post-resize paint must use the newly measured tail offset"
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
