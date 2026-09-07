use super::*;
use crate::ElementUpdate;

const VIEWPORT: Size = Size::new(640.0, 480.0);

struct Measurements;

impl Measurements {
    fn start() -> Self {
        RECORDED_TEXT_IDS.with(|ids| *ids.borrow_mut() = Some(Vec::new()));
        Self
    }

    fn take(&self) -> Vec<TextId> {
        RECORDED_TEXT_IDS.with(|ids| std::mem::take(ids.borrow_mut().as_mut().unwrap()))
    }
}

impl Drop for Measurements {
    fn drop(&mut self) {
        RECORDED_TEXT_IDS.with(|ids| *ids.borrow_mut() = None);
    }
}

fn set_root(tree: &mut UiTree, root: Element) {
    tree.set_root(root, VIEWPORT, 1.0, &mut TestTextLayout)
        .unwrap();
}

fn assert_matches_fresh_layout(tree: &UiTree, root: Element, viewport: Size, scale: f32) {
    let mut fresh = UiTree::new();
    fresh
        .set_root(root, viewport, scale, &mut TestTextLayout)
        .unwrap();
    assert_eq!(
        tree.layout_nodes.nodes.len(),
        fresh.layout_nodes.nodes.len()
    );
    assert_eq!(
        tree.taffy.total_node_count(),
        fresh.taffy.total_node_count()
    );
    for (id, node) in &tree.layout_nodes.nodes {
        assert_eq!(
            tree.taffy.layout(*node).unwrap(),
            fresh.taffy.layout(fresh.layout_nodes.nodes[id]).unwrap(),
            "layout differs for {id:?}",
        );
    }
}

fn panels(value: &'static str, color: Color) -> Element {
    div().size_full().flex_row().text_color(color).children([
        div()
            .id("changing-panel")
            .size(240.0, 480.0)
            .child(text(value).id("value")),
        div()
            .id("stable-panel")
            .size(320.0, 480.0)
            .flex_col()
            .children((0..100).map(|i| text("Stable sibling").id(ElementId::new(1000 + i)))),
    ])
}

#[test]
fn redeclaration_and_paint_only_changes_do_not_measure_text() {
    let mut tree = UiTree::new();
    let measurements = Measurements::start();
    set_root(&mut tree, panels("One", Color::BLACK));
    let identities = tree.layout_nodes.nodes.clone();
    assert!(measurements.take().len() >= 101);
    let passes = tree.layout_nodes.layout_passes;

    set_root(&mut tree, panels("One", Color::BLACK));
    assert!(
        measurements.take().is_empty(),
        "identical declarations must reuse layout"
    );
    set_root(&mut tree, panels("One", Color::WHITE).bg(Color::BLACK));
    assert!(
        measurements.take().is_empty(),
        "inherited color and background are paint-only"
    );
    assert_eq!(tree.layout_nodes.nodes, identities);
    assert_eq!(
        tree.layout_nodes.layout_passes, passes,
        "paint-only declarations skip the layout pass entirely"
    );
    assert_matches_fresh_layout(
        &tree,
        panels("One", Color::WHITE).bg(Color::BLACK),
        VIEWPORT,
        1.0,
    );
}

#[test]
fn changed_text_measures_only_its_branch() {
    let mut tree = UiTree::new();
    let measurements = Measurements::start();
    set_root(&mut tree, panels("One", Color::BLACK));
    measurements.take();

    set_root(&mut tree, panels("A much longer value", Color::BLACK));
    let measured = measurements.take();
    assert!(!measured.is_empty());
    assert!(
        measured
            .iter()
            .all(|id| *id == TextId::new(ElementId::from("value").value())),
        "unchanged sibling subtrees were measured: {measured:?}"
    );
    assert_matches_fresh_layout(
        &tree,
        panels("A much longer value", Color::BLACK),
        VIEWPORT,
        1.0,
    );
}

#[test]
fn inherited_font_and_scale_changes_invalidate_measurements() {
    let mut tree = UiTree::new();
    let measurements = Measurements::start();
    set_root(&mut tree, panels("One", Color::BLACK));
    measurements.take();
    set_root(&mut tree, panels("One", Color::BLACK).text_size(24.0));
    assert!(measurements.take().len() >= 101);

    tree.relayout_with_prepare(VIEWPORT, 2.0, &mut TestTextLayout, |_| {})
        .unwrap();
    assert!(
        measurements.take().len() >= 101,
        "scale is a measurement input even at the same logical size"
    );
    assert_matches_fresh_layout(
        &tree,
        panels("One", Color::BLACK).text_size(24.0),
        VIEWPORT,
        2.0,
    );
}

#[test]
fn retained_layout_matches_fresh_after_reorder_reparent_and_unmount() {
    let leaf = || text("Movable").id("leaf");
    let first = || {
        div().id("root").flex_row().children([
            div()
                .id("a")
                .p(8.0)
                .child(div().id("b").p(4.0).child(leaf())),
            text("Sibling").id("sibling"),
        ])
    };
    let inverted = || {
        div().id("root").flex_col().children([
            text("Sibling").id("sibling"),
            div()
                .id("b")
                .p(4.0)
                .child(div().id("a").p(8.0).child(leaf())),
        ])
    };
    let mut tree = UiTree::new();
    set_root(&mut tree, first());
    let identities = tree.layout_nodes.nodes.clone();
    set_root(&mut tree, inverted());
    assert_eq!(identities, tree.layout_nodes.nodes);
    assert_matches_fresh_layout(&tree, inverted(), VIEWPORT, 1.0);
    set_root(&mut tree, first());
    assert_matches_fresh_layout(&tree, first(), VIEWPORT, 1.0);

    for index in 0..20 {
        let root = || {
            div()
                .id("root")
                .child(text("Replacement").id(ElementId::new(2000 + index)))
        };
        set_root(&mut tree, root());
        assert_matches_fresh_layout(&tree, root(), VIEWPORT, 1.0);
        assert_eq!(
            tree.taffy.total_node_count(),
            2,
            "unmounted nodes must leave the arena"
        );
    }
}

#[test]
fn container_query_layout_survives_redeclaration_and_resizing() {
    let root = || {
        div().size_full().child(
            container_query(|size| {
                div().id("query-content").child(
                    text(if size.width < 300.0 { "Narrow" } else { "Wide" }).id("query-text"),
                )
            })
            .id("query")
            .w_full()
            .h(100.0),
        )
    };
    let mut tree = UiTree::new();
    let measurements = Measurements::start();
    set_root(&mut tree, root());
    let identities = tree.layout_nodes.nodes.clone();
    assert!(!measurements.take().is_empty());
    set_root(&mut tree, root());
    assert!(measurements.take().is_empty());
    assert_eq!(tree.layout_nodes.nodes, identities);
    let small = Size::new(200.0, 480.0);
    tree.relayout_with_prepare(small, 1.0, &mut TestTextLayout, |_| {})
        .unwrap();
    assert!(!measurements.take().is_empty());
    assert_matches_fresh_layout(&tree, root(), small, 1.0);
}

#[test]
fn element_kind_and_input_value_changes_update_intrinsic_layout() {
    let mut tree = UiTree::new();
    for child in [
        text("Label").id("content"),
        text_input("").placeholder("Placeholder").id("content"),
        text_input("A longer edited value").id("content"),
        div().size(50.0, 80.0).id("content"),
        text("Label again").id("content"),
    ] {
        let root = div().flex_col().child(child);
        set_root(&mut tree, root.clone());
        assert_matches_fresh_layout(&tree, root, VIEWPORT, 1.0);
    }
}

#[test]
fn targeted_text_updates_skip_unchanged_branches_and_preserve_selection() {
    let mut tree = UiTree::new();
    let measurements = Measurements::start();
    set_root(&mut tree, panels("Long selected text", Color::BLACK));
    measurements.take();
    let id = ElementId::from("value");
    tree.static_text_selection = Some(StaticTextSelection {
        anchor: StaticTextPosition { id, offset: 0 },
        focus: StaticTextPosition { id, offset: 18 },
    });
    let nodes = tree.layout_nodes.nodes.clone();
    assert_eq!(
        tree.update_elements(&[ElementUpdate::Text {
            id,
            content: Arc::from("Hi 🌍"),
        }])
        .unwrap(),
        Some(ElementUpdateKind::Layout)
    );
    assert!(tree.take_retained_semantics_dirty());
    assert!(!tree.take_retained_semantics_dirty());
    assert_eq!(
        tree.static_text_selection.unwrap().focus.offset,
        "Hi 🌍".len()
    );
    let entry = &tree.selectable_texts[tree.selectable_text_indices[&id]];
    assert_eq!(entry.content.as_ref(), "Hi 🌍");
    assert_eq!(entry.character_lengths.as_ref(), &[1, 1, 1, 4]);
    tree.layout(VIEWPORT, 1.0, &mut TestTextLayout).unwrap();
    assert_eq!(nodes, tree.layout_nodes.nodes);
    let measured = measurements.take();
    assert!(!measured.is_empty());
    assert!(
        measured
            .iter()
            .all(|text_id| *text_id == TextId::new(id.value()))
    );
    assert_matches_fresh_layout(&tree, panels("Hi 🌍", Color::BLACK), VIEWPORT, 1.0);
}

#[test]
fn targeted_paint_updates_respect_inheritance_and_do_not_dirty_layout() {
    let red = Color::rgb8(200, 0, 0);
    let root = || {
        div().id("root").text_color(Color::BLACK).children([
            div().id("inherited").child(text("Inherit").id("label")),
            div()
                .id("boundary")
                .text_color(red)
                .child(text("Explicit").id("red-label")),
        ])
    };
    let mut tree = UiTree::new();
    set_root(&mut tree, root());
    let measurements = Measurements::start();
    let updates = [
        ElementUpdate::TextColor {
            id: "root".into(),
            color: Color::WHITE,
        },
        ElementUpdate::BackgroundColor {
            id: "inherited".into(),
            color: Color::BLACK,
        },
        ElementUpdate::Opacity {
            id: "boundary".into(),
            opacity: 0.5,
        },
    ];
    assert_eq!(
        tree.update_elements(&updates).unwrap(),
        Some(ElementUpdateKind::Paint)
    );
    assert_eq!(
        tree.update_elements(&updates).unwrap(),
        Some(ElementUpdateKind::None)
    );
    assert!(!tree.take_retained_semantics_dirty());
    let root = tree.root.as_ref().unwrap();
    assert_eq!(
        root.children[0].children[0].resolved_typography.color,
        Color::WHITE
    );
    assert_eq!(root.children[1].children[0].resolved_typography.color, red);
    for node in tree.layout_nodes.nodes.values() {
        assert!(!tree.taffy.dirty(*node).unwrap());
    }
    let mut scene = Scene::new();
    tree.paint(&mut scene, &mut TestTextLayout).unwrap();
    assert_eq!(scene.text_runs()[0].style.color, Color::WHITE);
    assert_eq!(scene.text_runs()[1].style.color, red);
    assert!(measurements.take().is_empty());
}

#[test]
fn unsupported_target_rejects_a_whole_retained_batch() {
    let mut tree = UiTree::new();
    set_root(
        &mut tree,
        div().id("root").children([
            text("Original").id("label"),
            container_query(|_| text("Callback").id("callback-text")).size(100.0, 50.0),
        ]),
    );
    for id in ["missing", "root", "callback-text"] {
        assert_eq!(
            tree.update_elements(&[
                ElementUpdate::Text {
                    id: "label".into(),
                    content: Arc::from("Should not apply")
                },
                ElementUpdate::Text {
                    id: id.into(),
                    content: Arc::from("Unsupported")
                },
            ])
            .unwrap(),
            None
        );
        assert!(matches!(&tree.root.as_ref().unwrap().children[0].kind,
            ElementKind::Text(content) if content.as_ref() == "Original"));
    }
}

#[test]
fn resolved_motion_callbacks_keep_ownership_of_their_subtrees() {
    let mut tree = UiTree::new();
    set_root(
        &mut tree,
        div().id("root").children([
            text("Independent").id("independent"),
            div().id("animated").with_animation(
                "clock",
                Animation::new(Duration::from_secs(1)),
                |element, _| element.child(text("Callback value").id("animated-text")),
            ),
        ]),
    );
    for id in ["animated", "animated-text"] {
        assert_eq!(
            tree.update_elements(&[ElementUpdate::TextColor {
                id: id.into(),
                color: Color::WHITE
            }])
            .unwrap(),
            None
        );
    }
    assert_eq!(
        tree.update_elements(&[ElementUpdate::TextColor {
            id: "independent".into(),
            color: Color::WHITE
        }])
        .unwrap(),
        Some(ElementUpdateKind::Paint)
    );
}

#[test]
fn targeted_background_updates_keep_transition_playback() {
    let now = Instant::now();
    let mut tree = UiTree::new_at(now);
    set_root(
        &mut tree,
        div()
            .id("box")
            .size(40.0, 40.0)
            .bg(Color::BLACK)
            .transition(Transition::colors(Duration::from_millis(100)).with_easing(crate::linear)),
    );
    let mut scene = Scene::new();
    tree.paint_at(&mut scene, &mut TestTextLayout, now).unwrap();
    assert_eq!(
        tree.update_elements(&[ElementUpdate::BackgroundColor {
            id: "box".into(),
            color: Color::WHITE,
        }])
        .unwrap(),
        Some(ElementUpdateKind::Paint)
    );
    scene.clear(Color::TRANSPARENT);
    tree.paint_at(&mut scene, &mut TestTextLayout, now).unwrap();
    assert_eq!(scene.edge_quads()[0].fill, Color::BLACK);
    scene.clear(Color::TRANSPARENT);
    tree.paint_at(
        &mut scene,
        &mut TestTextLayout,
        now + Duration::from_millis(50),
    )
    .unwrap();
    let middle = scene.edge_quads()[0].fill;
    assert_ne!(middle, Color::BLACK);
    assert_ne!(middle, Color::WHITE);
    assert!(tree.style_transition_frame_requested());
    scene.clear(Color::TRANSPARENT);
    tree.paint_at(
        &mut scene,
        &mut TestTextLayout,
        now + Duration::from_millis(100),
    )
    .unwrap();
    assert_eq!(scene.edge_quads()[0].fill, Color::WHITE);
    assert!(!tree.style_transition_frame_requested());
}
