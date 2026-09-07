use super::*;

fn fixture() -> (
    quickgui::TestAppContext,
    quickgui::TestWindowHandle<NativeView>,
    EventQueue,
) {
    let mut tree = NativeTree::default();
    let mut tx = TreeTransaction::new(&tree);
    for (id, tag, parent) in [
        (1, NodeTag::View, ROOT_NODE),
        (2, NodeTag::View, 1),
        (3, NodeTag::View, 1),
        (4, NodeTag::Button, 3),
        (5, NodeTag::Input, 3),
        (6, NodeTag::Text, 4),
        (7, NodeTag::Button, 1),
        (8, NodeTag::Text, 7),
    ] {
        tx.create(
            id,
            tag,
            Arc::from(if tag == NodeTag::Text { "Original" } else { "" }),
        )
        .unwrap();
        tx.insert(parent, id, None).unwrap();
    }
    for id in [4, 7] {
        tx.set_property(
            id,
            property::CLICK_LISTENER,
            Some(PropertyValue::Bool(true)),
        )
        .unwrap();
        tx.set_property(id, property::WIDTH, Some(PropertyValue::Number(120.0)))
            .unwrap();
        tx.set_property(id, property::HEIGHT, Some(PropertyValue::Number(32.0)))
            .unwrap();
    }
    tx.set_property(
        5,
        property::VALUE,
        Some(PropertyValue::String(Arc::from("Kept"))),
    )
    .unwrap();
    let overlay = tx.finish().unwrap();
    commit_overlay(&mut tree, overlay);
    let events: EventQueue = Rc::new(RefCell::new(VecDeque::new()));
    let (cx, view) = mounted_component_view(tree, events.clone());
    (cx, view, events)
}

fn apply(
    cx: &mut quickgui::TestAppContext,
    view: quickgui::TestWindowHandle<NativeView>,
    mutations: Vec<Mutation>,
) {
    let tree = cx.read(view, |view| view.tree.clone()).unwrap();
    let targets =
        retained_scope_updates(&tree.borrow(), &mutations).expect("ordinary mutation scopes");
    apply_mutations(&mut tree.borrow_mut(), mutations).unwrap();
    cx.invalidate_elements(view.window_handle(), &targets)
        .unwrap();
}

#[test]
fn host_scope_mutations_preserve_input_focus_and_replace_listener_flags() {
    let (mut cx, view, events) = fixture();
    let window = view.window_handle();
    cx.focus(window, ElementId::new(5)).unwrap();
    let renders = cx.render_count(window).unwrap();
    apply(
        &mut cx,
        view,
        vec![
            Mutation::SetProperty {
                id: 4,
                key: property::WIDTH,
                value: Some(PropertyValue::Number(200.0)),
            },
            Mutation::ReplaceText {
                id: 6,
                text: Arc::from("Changed label"),
            },
            Mutation::SetProperty {
                id: 4,
                key: property::CLICK_LISTENER,
                value: Some(PropertyValue::Bool(false)),
            },
        ],
    );
    assert_eq!(cx.render_count(window).unwrap(), renders);
    assert_eq!(
        cx.focused_input_value(window).unwrap().as_deref(),
        Some("Kept")
    );
    assert_eq!(cx.focused(window).unwrap(), Some(ElementId::new(5)));
    assert_eq!(
        cx.element_bounds(window, ElementId::new(4)).unwrap().width,
        200.0
    );
    events.borrow_mut().clear();
    assert!(matches!(
        cx.click(window, ElementId::new(4)),
        Err(quickgui::TestAppError::NotClickable { .. })
    ));
    assert!(
        !events
            .borrow()
            .iter()
            .any(|event| event.kind == "click" && event.target == 4)
    );
    apply(
        &mut cx,
        view,
        vec![Mutation::SetProperty {
            id: 4,
            key: property::CLICK_LISTENER,
            value: Some(PropertyValue::Bool(true)),
        }],
    );
    cx.click(window, ElementId::new(4)).unwrap();
    assert!(
        events
            .borrow()
            .iter()
            .any(|event| event.kind == "click" && event.target == 4)
    );
    cx.click(window, ElementId::new(7)).unwrap();
    assert!(
        events
            .borrow()
            .iter()
            .any(|event| event.kind == "click" && event.target == 7)
    );
}

#[test]
fn host_scope_batches_move_keyed_children_before_their_old_parent_is_rebuilt() {
    let (mut cx, view, events) = fixture();
    let window = view.window_handle();
    let renders = cx.render_count(window).unwrap();
    apply(
        &mut cx,
        view,
        vec![
            Mutation::Insert {
                parent: 2,
                child: 4,
                before: None,
            },
            Mutation::ReplaceText {
                id: 6,
                text: Arc::from("Moved"),
            },
            Mutation::Create {
                id: 9,
                tag: NodeTag::Text,
                text: Arc::from("Inserted"),
            },
            Mutation::SetProperty {
                id: 9,
                key: property::FONT_SIZE,
                value: Some(PropertyValue::Number(24.0)),
            },
            Mutation::Insert {
                parent: 2,
                child: 9,
                before: Some(4),
            },
        ],
    );
    assert_eq!(cx.render_count(window).unwrap(), renders);
    assert!(cx.contains_element(window, ElementId::new(9)).unwrap());
    cx.click(window, ElementId::new(4)).unwrap();
    assert!(
        events
            .borrow()
            .iter()
            .any(|event| event.kind == "click" && event.target == 4)
    );
    let renders = cx.render_count(window).unwrap();
    apply(
        &mut cx,
        view,
        vec![Mutation::Cleanup {
            parent: 2,
            children: vec![4, 9],
        }],
    );
    assert!(!cx.contains_element(window, ElementId::new(4)).unwrap());
    assert!(!cx.contains_element(window, ElementId::new(6)).unwrap());
    assert!(!cx.contains_element(window, ElementId::new(9)).unwrap());
    // Removing focused content can trigger the existing focus-observer declaration path.
    assert!(cx.render_count(window).unwrap() <= renders + 1);
    cx.click(window, ElementId::new(7)).unwrap();
    assert!(
        events
            .borrow()
            .iter()
            .any(|event| event.kind == "click" && event.target == 7)
    );
}

#[test]
fn detached_mutations_do_not_request_a_window_declaration() {
    let tree = NativeTree::default();
    assert!(
        retained_scope_updates(
            &tree,
            &[
                Mutation::Create {
                    id: 1,
                    tag: NodeTag::View,
                    text: Arc::from("")
                },
                Mutation::SetProperty {
                    id: 1,
                    key: property::WIDTH,
                    value: Some(PropertyValue::Number(100.0))
                },
            ]
        )
        .unwrap()
        .is_empty()
    );
}
