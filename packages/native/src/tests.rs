use super::*;

#[test]
fn hosted_void_mutations_are_fire_and_forget_host_commands() {
    let host = HostCoordinator::new();
    host.enqueue(HostCommand::Mutation {
        app: 7,
        command: system::SystemCommand::WindowAction {
            window: 11,
            action: system::WindowAction::SetMacOsVibrancy(Some(MacOsVibrancy::Sidebar)),
        },
    })
    .expect("a window mutation fits the host queue");
    host.enqueue(HostCommand::Mutation {
        app: 7,
        command: system::SystemCommand::SetDockBadge(Some("3".to_owned())),
    })
    .expect("an application mutation fits the host queue");

    let mut commands = host.take_commands().expect("the host queue is readable");
    match commands
        .pop_front()
        .expect("the window mutation was queued")
    {
        HostCommand::Mutation {
            app,
            command:
                system::SystemCommand::WindowAction {
                    window,
                    action: system::WindowAction::SetMacOsVibrancy(vibrancy),
                },
        } => {
            assert_eq!(app, 7);
            assert_eq!(window, 11);
            assert_eq!(vibrancy, Some(MacOsVibrancy::Sidebar));
        }
        _ => panic!("the mutation must not be wrapped in a synchronous system command"),
    }
    match commands
        .pop_front()
        .expect("the application mutation was queued")
    {
        HostCommand::Mutation {
            app,
            command: system::SystemCommand::SetDockBadge(value),
        } => {
            assert_eq!(app, 7);
            assert_eq!(value.as_deref(), Some("3"));
        }
        _ => panic!("the mutation must not carry a synchronous reply"),
    }
    assert!(commands.is_empty());
}

#[test]
fn hosted_window_creation_uses_a_preallocated_handle_without_a_reply() {
    let host = HostCoordinator::new();
    let window = host
        .allocate_window()
        .expect("the worker can allocate a hosted window handle");
    host.enqueue(HostCommand::CreateWindow {
        app: 7,
        window,
        options: NativeWindowOptions::default(),
        initial_batch: Vec::new(),
    })
    .expect("window creation fits the host queue");

    let mut commands = host.take_commands().expect("the host queue is readable");
    match commands.pop_front().expect("window creation was queued") {
        HostCommand::CreateWindow {
            app,
            window: queued_window,
            initial_batch,
            ..
        } => {
            assert_eq!(app, 7);
            assert_eq!(queued_window, window);
            assert!(initial_batch.is_empty());
        }
        _ => panic!("hosted creation must use a no-reply command"),
    }
    assert!(commands.is_empty());
}

#[test]
fn app_configuration_updates_preserve_embedded_identity_and_paths() {
    let (info, paths, quit_mode) = native_app_configuration(NativeAppOptions {
        name: Some("QuickGUI Test".to_owned()),
        version: Some("1.2.3".to_owned()),
        identifier: Some("dev.quickgui.binding-test".to_owned()),
        ..NativeAppOptions::default()
    })
    .expect("valid initial application configuration");
    assert_eq!(quit_mode, QuitMode::Default);
    let initial_paths = paths.clone().expect("identity resolves application paths");

    let (info, paths, quit_mode) = update_native_app_configuration(
        info,
        paths,
        quit_mode,
        NativeAppOptions {
            quit_mode: Some("explicit".to_owned()),
            ..NativeAppOptions::default()
        },
    )
    .expect("quit-only configuration update");

    assert_eq!(
        info.as_ref().map(AppInfo::identifier),
        Some("dev.quickgui.binding-test")
    );
    assert_eq!(paths.as_ref(), Some(&initial_paths));
    assert_eq!(quit_mode, QuitMode::Explicit);

    let overridden = PathBuf::from("/tmp/quickgui-binding-test-config");
    let (info, paths, quit_mode) = update_native_app_configuration(
        info,
        paths,
        quit_mode,
        NativeAppOptions {
            config_dir: Some(overridden.to_string_lossy().into_owned()),
            ..NativeAppOptions::default()
        },
    )
    .expect("path-only configuration update");

    assert_eq!(
        info.as_ref().map(AppInfo::identifier),
        Some("dev.quickgui.binding-test")
    );
    assert_eq!(
        paths.as_ref().and_then(AppPaths::config_dir),
        Some(overridden.as_path())
    );
    assert_eq!(quit_mode, QuitMode::Explicit);
}

#[test]
fn window_configuration_maps_every_electron_compatible_macos_vibrancy_type() {
    let materials = [
        ("appearance-based", MacOsVibrancy::AppearanceBased),
        ("titlebar", MacOsVibrancy::Titlebar),
        ("selection", MacOsVibrancy::Selection),
        ("menu", MacOsVibrancy::Menu),
        ("popover", MacOsVibrancy::Popover),
        ("sidebar", MacOsVibrancy::Sidebar),
        ("header", MacOsVibrancy::Header),
        ("sheet", MacOsVibrancy::Sheet),
        ("window", MacOsVibrancy::Window),
        ("hud", MacOsVibrancy::Hud),
        ("fullscreen-ui", MacOsVibrancy::FullscreenUi),
        ("tooltip", MacOsVibrancy::Tooltip),
        ("content", MacOsVibrancy::Content),
        ("under-window", MacOsVibrancy::UnderWindow),
        ("under-page", MacOsVibrancy::UnderPage),
    ];

    for (name, expected) in materials {
        let config = window_config(&NativeWindowOptions {
            vibrancy: Some(name.to_owned()),
            visual_effect_state: Some("inactive".to_owned()),
            ..NativeWindowOptions::default()
        })
        .expect("Electron-compatible vibrancy material maps into the Rust core");
        assert_eq!(config.macos_vibrancy, Some(expected));
        assert_eq!(
            config.macos_visual_effect_state,
            MacOsVisualEffectState::Inactive
        );
    }

    assert!(
        window_config(&NativeWindowOptions {
            vibrancy: Some("glass".to_owned()),
            ..NativeWindowOptions::default()
        })
        .unwrap_err()
        .contains("unknown macOS vibrancy type")
    );
}

struct BatchWriter {
    bytes: Vec<u8>,
    count: u32,
}

impl BatchWriter {
    fn new() -> Self {
        let mut bytes = PROTOCOL_MAGIC.to_vec();
        bytes.extend_from_slice(&PROTOCOL_VERSION.to_le_bytes());
        bytes.extend_from_slice(&0_u32.to_le_bytes());
        Self { bytes, count: 0 }
    }

    fn op(&mut self, opcode: u8) {
        self.count += 1;
        self.bytes.push(opcode);
    }

    fn u32(&mut self, value: u32) {
        self.bytes.extend_from_slice(&value.to_le_bytes());
    }

    fn string(&mut self, value: &str) {
        self.u32(value.len() as u32);
        self.bytes.extend_from_slice(value.as_bytes());
    }

    fn finish(mut self) -> Vec<u8> {
        self.bytes[6..10].copy_from_slice(&self.count.to_le_bytes());
        self.bytes
    }
}

#[test]
fn binary_batch_builds_and_removes_a_tree_atomically() {
    let mut writer = BatchWriter::new();
    writer.op(1);
    writer.u32(1);
    writer.bytes.push(1);
    writer.op(2);
    writer.u32(2);
    writer.string("Hello");
    writer.op(6);
    writer.u32(1);
    writer.u32(2);
    writer.u32(NO_ANCHOR);
    writer.op(6);
    writer.u32(ROOT_NODE);
    writer.u32(1);
    writer.u32(NO_ANCHOR);

    let mut tree = NativeTree::default();
    let revision = apply_mutations(&mut tree, decode_batch(&writer.finish()).unwrap()).unwrap();
    assert_eq!(revision, 1);
    assert_eq!(tree.nodes[&ROOT_NODE].children, [1]);
    assert_eq!(tree.nodes[&1].children, [2]);
    assert_eq!(tree.nodes[&2].text.as_ref(), "Hello");

    let mut remove = BatchWriter::new();
    remove.op(7);
    remove.u32(ROOT_NODE);
    remove.u32(1);
    apply_mutations(&mut tree, decode_batch(&remove.finish()).unwrap()).unwrap();
    assert_eq!(tree.nodes.len(), 1);
    assert!(tree.nodes[&ROOT_NODE].children.is_empty());
}

#[test]
fn initial_window_batch_is_committed_before_the_core_view_opens() {
    let mut writer = BatchWriter::new();
    writer.op(2);
    writer.u32(1);
    writer.string("Ready");
    writer.op(6);
    writer.u32(ROOT_NODE);
    writer.u32(1);
    writer.u32(NO_ANCHOR);

    let tree = native_tree_from_initial_batch(&writer.finish()).unwrap();
    assert_eq!(tree.revision, 1);
    assert_eq!(tree.nodes[&ROOT_NODE].children, [1]);
    assert_eq!(tree.nodes[&1].text.as_ref(), "Ready");
}

#[test]
fn failed_cycle_does_not_mutate_the_committed_tree() {
    let mut tree = NativeTree::default();
    let mut setup = BatchWriter::new();
    for id in [1, 2] {
        setup.op(1);
        setup.u32(id);
        setup.bytes.push(1);
    }
    setup.op(6);
    setup.u32(ROOT_NODE);
    setup.u32(1);
    setup.u32(NO_ANCHOR);
    setup.op(6);
    setup.u32(1);
    setup.u32(2);
    setup.u32(NO_ANCHOR);
    apply_mutations(&mut tree, decode_batch(&setup.finish()).unwrap()).unwrap();
    let before = tree.clone();

    let mut cycle = BatchWriter::new();
    cycle.op(6);
    cycle.u32(2);
    cycle.u32(1);
    cycle.u32(NO_ANCHOR);
    assert!(apply_mutations(&mut tree, decode_batch(&cycle.finish()).unwrap()).is_err());
    assert_eq!(tree.revision, before.revision);
    assert_eq!(
        tree.nodes[&ROOT_NODE].children,
        before.nodes[&ROOT_NODE].children
    );
    assert_eq!(tree.nodes[&1].children, before.nodes[&1].children);
}

#[test]
fn malformed_batch_is_rejected_before_tree_mutation() {
    let error = decode_batch(b"not a batch").unwrap_err();
    assert!(error.to_string().contains("magic"));
}

#[cfg(target_os = "macos")]
#[test]
fn swift_ui_button_modifiers_decode_in_declared_order() {
    let mut button = NativeNode::new(NodeTag::SwiftUiButton);
    button.set_property(
        property::SWIFT_UI_MODIFIERS,
        Some(PropertyValue::String(Arc::from(
            r##"[{"$type":"buttonStyle","style":"glass"},{"$type":"controlSize","size":"large"},{"$type":"buttonBorderShape","shape":"roundedRectangle","cornerRadius":14},{"$type":"labelStyle","style":"iconOnly"},{"$type":"tint","color":"#3366ffff"},{"$type":"disabled","disabled":false}]"##,
        ))),
    );

    assert_eq!(
        swift_ui_modifiers(&button).unwrap(),
        vec![
            SwiftUiModifier::ButtonStyle(SwiftUiButtonStyle::Glass),
            SwiftUiModifier::ControlSize(SwiftUiControlSize::Large),
            SwiftUiModifier::ButtonBorderShape {
                shape: SwiftUiButtonBorderShape::RoundedRectangle,
                corner_radius: Some(14.0),
            },
            SwiftUiModifier::LabelStyle(SwiftUiLabelStyle::IconOnly),
            SwiftUiModifier::Tint(Arc::from("#3366ffff")),
            SwiftUiModifier::Disabled(false),
        ]
    );
}

#[cfg(target_os = "macos")]
#[test]
fn swift_ui_button_rejects_unknown_modifiers() {
    let mut button = NativeNode::new(NodeTag::SwiftUiButton);
    button.set_property(
        property::SWIFT_UI_MODIFIERS,
        Some(PropertyValue::String(Arc::from(r#"[{"$type":"unknown"}]"#))),
    );

    assert!(
        swift_ui_modifiers(&button)
            .unwrap_err()
            .contains("unsupported SwiftUI modifier")
    );
}
#[test]
fn queued_input_and_submit_survive_until_javascript_commits_the_controlled_value() {
    let input_id = 7;
    let mut tree = NativeTree::default();
    let mut input = NativeNode::new(NodeTag::Input);
    input.parent = Some(ROOT_NODE);
    input.set_property(property::INPUT_LISTENER, Some(PropertyValue::Bool(true)));
    input.set_property(property::SUBMIT_LISTENER, Some(PropertyValue::Bool(true)));
    tree.nodes.insert(input_id, input);
    tree.nodes
        .get_mut(&ROOT_NODE)
        .unwrap()
        .children
        .push(input_id);

    let events = Rc::new(RefCell::new(VecDeque::new()));
    let view = NativeView {
        window: 3,
        handles: None,
        tree: Rc::new(RefCell::new(tree)),
        events: Rc::clone(&events),
        markdown: Rc::new(RefCell::new(HashMap::new())),
        svgs: Rc::new(RefCell::new(HashMap::new())),
        lists: Rc::new(RefCell::new(HashMap::new())),
        terminals: Rc::new(RefCell::new(HashMap::new())),
        #[cfg(target_os = "macos")]
        swift_ui_hosts: Rc::new(RefCell::new(HashMap::new())),
        embedded_views: Rc::new(RefCell::new(HashMap::new())),
    };
    let (mut cx, view) = quickgui::TestAppContext::new(view).unwrap();
    let window = view.window_handle();

    cx.focus(window, ElementId::new(input_id as u64)).unwrap();
    cx.simulate_input(window, "hello").unwrap();

    assert_eq!(
        cx.focused_input_value(window).unwrap().as_deref(),
        Some("hello")
    );
    let event = events.borrow_mut().pop_front().unwrap();
    assert_eq!(event.kind, "input");
    assert_eq!(event.window, 3);
    assert_eq!(event.target, input_id);
    assert_eq!(event.value.as_deref(), Some("hello"));

    cx.simulate_keystrokes(window, "enter").unwrap();

    assert_eq!(
        cx.focused(window).unwrap(),
        Some(ElementId::new(input_id as u64))
    );
    assert_eq!(
        cx.focused_input_value(window).unwrap().as_deref(),
        Some("hello")
    );
    let event = events.borrow_mut().pop_front().unwrap();
    assert_eq!(event.kind, "submit");
    assert_eq!(event.window, 3);
    assert_eq!(event.target, input_id);
    assert_eq!(event.value.as_deref(), Some("hello"));
}

#[test]
fn flex_without_direction_uses_css_row_default() {
    let implicit_row_id = 10;
    let row_first_id = 11;
    let row_second_id = 12;
    let explicit_column_id = 20;
    let column_first_id = 21;
    let column_second_id = 22;
    let mut tree = NativeTree::default();

    let mut implicit_row = NativeNode::new(NodeTag::Button);
    implicit_row.parent = Some(ROOT_NODE);
    implicit_row.children.extend([row_first_id, row_second_id]);
    implicit_row.set_property(
        property::DISPLAY,
        Some(PropertyValue::String(Arc::from("flex"))),
    );
    implicit_row.set_property(property::WIDTH, Some(PropertyValue::Number(200.0)));
    implicit_row.set_property(property::HEIGHT, Some(PropertyValue::Number(60.0)));
    tree.nodes.insert(implicit_row_id, implicit_row);

    let mut explicit_column = NativeNode::new(NodeTag::Button);
    explicit_column.parent = Some(ROOT_NODE);
    explicit_column
        .children
        .extend([column_first_id, column_second_id]);
    explicit_column.set_property(
        property::DISPLAY,
        Some(PropertyValue::String(Arc::from("flex"))),
    );
    explicit_column.set_property(
        property::FLEX_DIRECTION,
        Some(PropertyValue::String(Arc::from("column"))),
    );
    explicit_column.set_property(property::WIDTH, Some(PropertyValue::Number(200.0)));
    explicit_column.set_property(property::HEIGHT, Some(PropertyValue::Number(60.0)));
    tree.nodes.insert(explicit_column_id, explicit_column);

    for (id, parent, value) in [
        (row_first_id, implicit_row_id, "Count:"),
        (row_second_id, implicit_row_id, "0"),
        (column_first_id, explicit_column_id, "Count:"),
        (column_second_id, explicit_column_id, "0"),
    ] {
        let mut text = NativeNode::new(NodeTag::Text);
        text.parent = Some(parent);
        text.text = Arc::from(value);
        tree.nodes.insert(id, text);
    }
    tree.nodes
        .get_mut(&ROOT_NODE)
        .unwrap()
        .children
        .extend([implicit_row_id, explicit_column_id]);

    let view = NativeView {
        window: 4,
        handles: None,
        tree: Rc::new(RefCell::new(tree)),
        events: Rc::new(RefCell::new(VecDeque::new())),
        markdown: Rc::new(RefCell::new(HashMap::new())),
        svgs: Rc::new(RefCell::new(HashMap::new())),
        lists: Rc::new(RefCell::new(HashMap::new())),
        terminals: Rc::new(RefCell::new(HashMap::new())),
        #[cfg(target_os = "macos")]
        swift_ui_hosts: Rc::new(RefCell::new(HashMap::new())),
        embedded_views: Rc::new(RefCell::new(HashMap::new())),
    };
    let (mut cx, view) = quickgui::TestAppContext::new(view).unwrap();
    let window = view.window_handle();

    let row_first = cx
        .element_bounds(window, ElementId::new(row_first_id as u64))
        .unwrap();
    let row_second = cx
        .element_bounds(window, ElementId::new(row_second_id as u64))
        .unwrap();
    assert_eq!(row_second.y, row_first.y);
    assert!(row_second.x >= row_first.right());

    let column_first = cx
        .element_bounds(window, ElementId::new(column_first_id as u64))
        .unwrap();
    let column_second = cx
        .element_bounds(window, ElementId::new(column_second_id as u64))
        .unwrap();
    assert_eq!(column_second.x, column_first.x);
    assert!(column_second.y >= column_first.bottom());
}

#[test]
fn retained_popover_uses_core_placement_dismissal_and_focus_restoration() {
    let trigger_id = 20;
    let popover_id = 21;
    let mut tree = NativeTree::default();

    let mut trigger = NativeNode::new(NodeTag::Button);
    trigger.parent = Some(ROOT_NODE);
    trigger.set_property(property::WIDTH, Some(PropertyValue::Number(120.0)));
    trigger.set_property(property::HEIGHT, Some(PropertyValue::Number(40.0)));
    tree.nodes.insert(trigger_id, trigger);

    let mut popover = NativeNode::new(NodeTag::View);
    popover.parent = Some(ROOT_NODE);
    popover.set_property(property::WIDTH, Some(PropertyValue::Number(200.0)));
    popover.set_property(property::HEIGHT, Some(PropertyValue::Number(100.0)));
    popover.set_property(
        property::ANCHOR_TARGET,
        Some(PropertyValue::String(Arc::from(trigger_id.to_string()))),
    );
    popover.set_property(
        property::ANCHOR_PLACEMENT,
        Some(PropertyValue::String(Arc::from("bottom-start"))),
    );
    popover.set_property(property::ANCHOR_GAP, Some(PropertyValue::Number(6.0)));
    popover.set_property(property::VIEWPORT_MARGIN, Some(PropertyValue::Number(12.0)));
    popover.set_property(property::DISMISS_LISTENER, Some(PropertyValue::Bool(true)));
    tree.nodes.insert(popover_id, popover);
    tree.nodes
        .get_mut(&ROOT_NODE)
        .unwrap()
        .children
        .extend([trigger_id, popover_id]);

    let events = Rc::new(RefCell::new(VecDeque::new()));
    let view = NativeView {
        window: 5,
        handles: None,
        tree: Rc::new(RefCell::new(tree)),
        events: Rc::clone(&events),
        markdown: Rc::new(RefCell::new(HashMap::new())),
        svgs: Rc::new(RefCell::new(HashMap::new())),
        lists: Rc::new(RefCell::new(HashMap::new())),
        terminals: Rc::new(RefCell::new(HashMap::new())),
        #[cfg(target_os = "macos")]
        swift_ui_hosts: Rc::new(RefCell::new(HashMap::new())),
        embedded_views: Rc::new(RefCell::new(HashMap::new())),
    };
    let (mut cx, view) = quickgui::TestAppContext::new(view).unwrap();
    let window = view.window_handle();
    let trigger_element = ElementId::new(trigger_id as u64);
    let popover_element = ElementId::new(popover_id as u64);

    let trigger_bounds = cx.element_bounds(window, trigger_element).unwrap();
    let popover_bounds = cx.element_bounds(window, popover_element).unwrap();
    assert_eq!(popover_bounds.x, 12.0);
    assert_eq!(popover_bounds.y, trigger_bounds.bottom() + 6.0);
    assert_eq!(popover_bounds.width, 200.0);
    assert_eq!(popover_bounds.height, 100.0);

    cx.focus(window, popover_element).unwrap();
    cx.simulate_keystrokes(window, "escape").unwrap();

    assert_eq!(cx.focused(window).unwrap(), Some(trigger_element));
    let event = events.borrow_mut().pop_front().unwrap();
    assert_eq!(event.kind, "dismiss");
    assert_eq!(event.window, 5);
    assert_eq!(event.target, popover_id);

    cx.focus(window, popover_element).unwrap();
    cx.update(view, |view, cx| {
        let mut tree = view.tree.borrow_mut();
        tree.nodes
            .get_mut(&ROOT_NODE)
            .unwrap()
            .children
            .retain(|child| *child != popover_id);
        tree.nodes.remove(&popover_id);
        cx.invalidate();
    })
    .unwrap();

    assert!(!cx.contains_element(window, popover_element).unwrap());
    assert_eq!(cx.focused(window).unwrap(), Some(trigger_element));
}

#[test]
fn unanchored_overlay_traps_autofocus_dismisses_and_restores_previous_focus() {
    let outside_id = 40;
    let overlay_id = 41;
    let surface_id = 42;
    let prompt_id = 43;
    let mut tree = NativeTree::default();

    let mut outside = NativeNode::new(NodeTag::Button);
    outside.parent = Some(ROOT_NODE);
    tree.nodes.insert(outside_id, outside);
    tree.nodes
        .get_mut(&ROOT_NODE)
        .unwrap()
        .children
        .push(outside_id);

    let events = Rc::new(RefCell::new(VecDeque::new()));
    let view = NativeView {
        window: 6,
        handles: None,
        tree: Rc::new(RefCell::new(tree)),
        events: Rc::clone(&events),
        markdown: Rc::new(RefCell::new(HashMap::new())),
        svgs: Rc::new(RefCell::new(HashMap::new())),
        lists: Rc::new(RefCell::new(HashMap::new())),
        terminals: Rc::new(RefCell::new(HashMap::new())),
        #[cfg(target_os = "macos")]
        swift_ui_hosts: Rc::new(RefCell::new(HashMap::new())),
        embedded_views: Rc::new(RefCell::new(HashMap::new())),
    };
    let (mut cx, view) = quickgui::TestAppContext::new(view).unwrap();
    let window = view.window_handle();
    let outside_element = ElementId::new(outside_id as u64);
    let prompt_element = ElementId::new(prompt_id as u64);

    cx.focus(window, outside_element).unwrap();
    cx.update(view, |view, cx| {
        let mut tree = view.tree.borrow_mut();

        let mut overlay = NativeNode::new(NodeTag::View);
        overlay.parent = Some(ROOT_NODE);
        overlay.children.push(surface_id);
        overlay.set_property(property::OVERLAY, Some(PropertyValue::Bool(true)));
        overlay.set_property(property::FOCUS_TRAP, Some(PropertyValue::Bool(true)));
        overlay.set_property(
            property::RESTORE_PREVIOUS_FOCUS,
            Some(PropertyValue::Bool(true)),
        );

        let mut surface = NativeNode::new(NodeTag::View);
        surface.parent = Some(overlay_id);
        surface.children.push(prompt_id);
        surface.set_property(property::DISMISS_ON_ESCAPE, Some(PropertyValue::Bool(true)));
        surface.set_property(
            property::DISMISS_ON_POINTER_OUTSIDE,
            Some(PropertyValue::Bool(true)),
        );
        surface.set_property(property::DISMISS_LISTENER, Some(PropertyValue::Bool(true)));
        surface.set_property(
            property::ACCESSIBILITY_MODAL,
            Some(PropertyValue::Bool(true)),
        );

        let mut prompt = NativeNode::new(NodeTag::Input);
        prompt.parent = Some(surface_id);
        prompt.set_property(property::MULTILINE, Some(PropertyValue::Bool(true)));
        prompt.set_property(property::AUTO_FOCUS, Some(PropertyValue::Bool(true)));

        tree.nodes.insert(overlay_id, overlay);
        tree.nodes.insert(surface_id, surface);
        tree.nodes.insert(prompt_id, prompt);
        tree.nodes
            .get_mut(&ROOT_NODE)
            .unwrap()
            .children
            .push(overlay_id);
        cx.invalidate();
    })
    .unwrap();

    assert_eq!(cx.focused(window).unwrap(), Some(prompt_element));
    cx.simulate_keystrokes(window, "escape").unwrap();
    let event = events.borrow_mut().pop_front().unwrap();
    assert_eq!(event.kind, "dismiss");
    assert_eq!(event.target, surface_id);

    cx.update(view, |view, cx| {
        let mut tree = view.tree.borrow_mut();
        tree.nodes
            .get_mut(&ROOT_NODE)
            .unwrap()
            .children
            .retain(|child| *child != overlay_id);
        tree.nodes.remove(&prompt_id);
        tree.nodes.remove(&surface_id);
        tree.nodes.remove(&overlay_id);
        cx.invalidate();
    })
    .unwrap();

    assert_eq!(cx.focused(window).unwrap(), Some(outside_element));
}

#[test]
fn native_svg_is_parsed_once_until_its_source_changes() {
    let svg_id = 30;
    let first_source = r#"<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24"><path d="M2 2h20v20H2z"/></svg>"#;
    let second_source = r#"<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24"><circle cx="12" cy="12" r="10"/></svg>"#;
    let mut tree = NativeTree::default();
    let mut icon = NativeNode::new(NodeTag::Svg);
    icon.parent = Some(ROOT_NODE);
    icon.set_property(
        property::VALUE,
        Some(PropertyValue::String(Arc::from(first_source))),
    );
    icon.set_property(property::WIDTH, Some(PropertyValue::Number(16.0)));
    icon.set_property(property::HEIGHT, Some(PropertyValue::Number(16.0)));
    tree.nodes.insert(svg_id, icon);
    tree.nodes
        .get_mut(&ROOT_NODE)
        .unwrap()
        .children
        .push(svg_id);

    let svgs = Rc::new(RefCell::new(HashMap::new()));
    let view = NativeView {
        window: 7,
        handles: None,
        tree: Rc::new(RefCell::new(tree)),
        events: Rc::new(RefCell::new(VecDeque::new())),
        markdown: Rc::new(RefCell::new(HashMap::new())),
        svgs: Rc::clone(&svgs),
        lists: Rc::new(RefCell::new(HashMap::new())),
        terminals: Rc::new(RefCell::new(HashMap::new())),
        #[cfg(target_os = "macos")]
        swift_ui_hosts: Rc::new(RefCell::new(HashMap::new())),
        embedded_views: Rc::new(RefCell::new(HashMap::new())),
    };
    let (mut cx, view) = quickgui::TestAppContext::new(view).unwrap();
    let window = view.window_handle();
    let bounds = cx
        .element_bounds(window, ElementId::new(svg_id as u64))
        .unwrap();
    assert_eq!((bounds.width, bounds.height), (16.0, 16.0));

    let first = svgs.borrow()[&svg_id].parsed.as_ref().unwrap().clone();
    cx.update(view, |_view, cx| cx.invalidate()).unwrap();
    let unchanged = svgs.borrow()[&svg_id].parsed.as_ref().unwrap().clone();
    assert_eq!(first, unchanged);

    cx.update(view, |view, cx| {
        view.tree
            .borrow_mut()
            .nodes
            .get_mut(&svg_id)
            .unwrap()
            .set_property(
                property::VALUE,
                Some(PropertyValue::String(Arc::from(second_source))),
            );
        cx.invalidate();
    })
    .unwrap();
    let changed = svgs.borrow()[&svg_id].parsed.as_ref().unwrap().clone();
    assert_ne!(first, changed);
}

#[test]
fn native_virtual_list_mounts_only_the_initial_window_and_overscan() {
    let list_id = 40;
    let first_item_id = 1_000;
    let item_count = 100;
    let mut tree = NativeTree::default();
    let mut list = NativeNode::new(NodeTag::VirtualList);
    list.parent = Some(ROOT_NODE);
    list.set_property(
        property::ESTIMATED_ITEM_HEIGHT,
        Some(PropertyValue::Number(24.0)),
    );
    list.set_property(property::OVERSCAN, Some(PropertyValue::Number(2.0)));
    for index in 0..item_count {
        let id = first_item_id + index;
        let mut item = NativeNode::new(NodeTag::Text);
        item.parent = Some(list_id);
        item.text = Arc::from(format!("Item {index}"));
        tree.nodes.insert(id, item);
        list.children.push(id);
    }
    tree.nodes.insert(list_id, list);
    tree.nodes
        .get_mut(&ROOT_NODE)
        .unwrap()
        .children
        .push(list_id);

    let lists = Rc::new(RefCell::new(HashMap::new()));
    let view = NativeView {
        window: 4,
        handles: None,
        tree: Rc::new(RefCell::new(tree)),
        events: Rc::new(RefCell::new(VecDeque::new())),
        markdown: Rc::new(RefCell::new(HashMap::new())),
        svgs: Rc::new(RefCell::new(HashMap::new())),
        lists: Rc::clone(&lists),
        terminals: Rc::new(RefCell::new(HashMap::new())),
        #[cfg(target_os = "macos")]
        swift_ui_hosts: Rc::new(RefCell::new(HashMap::new())),
        embedded_views: Rc::new(RefCell::new(HashMap::new())),
    };
    let (cx, view) = quickgui::TestAppContext::new(view).unwrap();
    let window = view.window_handle();

    assert!(
        cx.contains_element(window, ElementId::new(first_item_id as u64))
            .unwrap()
    );
    assert!(
        !cx.contains_element(
            window,
            ElementId::new((first_item_id + item_count - 1) as u64),
        )
        .unwrap()
    );
    let mounted = lists.borrow()[&list_id].list.visible_rows().len();
    assert!(mounted < item_count as usize);
}

#[cfg(unix)]
#[test]
fn native_terminal_runs_a_real_pty_and_rerenders_ghostty_output() {
    let terminal_id = 50;
    let mut tree = NativeTree::default();
    let mut terminal = NativeNode::new(NodeTag::Terminal);
    terminal.parent = Some(ROOT_NODE);
    terminal.set_property(
        property::TERMINAL_PROGRAM,
        Some(PropertyValue::String(Arc::from("/bin/sh"))),
    );
    terminal.set_property(
        property::TERMINAL_ARGUMENTS,
        Some(PropertyValue::String(Arc::from(
            serde_json::json!(["-c", "printf 'quickgui-pty-ok\\n'"]).to_string(),
        ))),
    );
    terminal.set_property(
        property::TERMINAL_STATUS_LISTENER,
        Some(PropertyValue::Bool(true)),
    );
    terminal.set_property(
        property::POSITION,
        Some(PropertyValue::String(Arc::from("absolute"))),
    );
    terminal.set_property(property::TOP, Some(PropertyValue::Number(8.0)));
    terminal.set_property(property::RIGHT, Some(PropertyValue::Number(9.0)));
    terminal.set_property(property::BOTTOM, Some(PropertyValue::Number(8.0)));
    terminal.set_property(property::LEFT, Some(PropertyValue::Number(9.0)));
    terminal.set_property(property::PADDING_TOP, Some(PropertyValue::Number(8.0)));
    terminal.set_property(property::PADDING_RIGHT, Some(PropertyValue::Number(9.0)));
    terminal.set_property(property::PADDING_BOTTOM, Some(PropertyValue::Number(8.0)));
    terminal.set_property(property::PADDING_LEFT, Some(PropertyValue::Number(9.0)));
    tree.nodes.insert(terminal_id, terminal);
    tree.nodes
        .get_mut(&ROOT_NODE)
        .unwrap()
        .children
        .push(terminal_id);

    let terminals = Rc::new(RefCell::new(HashMap::new()));
    let events = Rc::new(RefCell::new(VecDeque::new()));
    let view = NativeView {
        window: 6,
        handles: None,
        tree: Rc::new(RefCell::new(tree)),
        events: Rc::clone(&events),
        markdown: Rc::new(RefCell::new(HashMap::new())),
        svgs: Rc::new(RefCell::new(HashMap::new())),
        lists: Rc::new(RefCell::new(HashMap::new())),
        terminals: Rc::clone(&terminals),
        #[cfg(target_os = "macos")]
        swift_ui_hosts: Rc::new(RefCell::new(HashMap::new())),
        embedded_views: Rc::new(RefCell::new(HashMap::new())),
    };
    let (mut cx, view) = quickgui::TestAppContext::new(view).unwrap();
    let window = view.window_handle();
    for _ in 0..200 {
        let finished_with_output = terminals
            .borrow()
            .get(&terminal_id)
            .and_then(|state| state.terminal.as_ref())
            .is_some_and(|terminal| {
                let snapshot = terminal.snapshot();
                matches!(snapshot.status, TerminalStatus::Exited { .. })
                    && snapshot.content.contains("quickgui-pty-ok")
            });
        if finished_with_output {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    cx.update(view, |_view, cx| cx.invalidate()).unwrap();

    let snapshot = terminals.borrow()[&terminal_id]
        .terminal
        .as_ref()
        .unwrap()
        .snapshot();
    assert!(
        snapshot.content.contains("quickgui-pty-ok"),
        "snapshot: {snapshot:#?}"
    );
    assert!(matches!(snapshot.status, TerminalStatus::Exited { .. }));
    let root_bounds = cx
        .element_bounds(window, ElementId::new(ROOT_ELEMENT_ID))
        .unwrap();
    let terminal_bounds = cx
        .element_bounds(window, ElementId::new(terminal_id as u64))
        .unwrap();
    assert_eq!(terminal_bounds.x, root_bounds.x + 9.0);
    assert_eq!(terminal_bounds.y, root_bounds.y + 8.0);
    assert_eq!(terminal_bounds.right(), root_bounds.right() - 9.0);
    assert_eq!(terminal_bounds.bottom(), root_bounds.bottom() - 8.0);
    assert!(
        events
            .borrow()
            .iter()
            .any(|event| event.kind == "terminal" && event.target == terminal_id)
    );
}

#[test]
fn close_interception_is_declared_ahead_of_the_native_decision() {
    crate::runtime::reset_interception_state();
    assert!(!crate::runtime::intercepts_close(4));

    let action = crate::system::parse_window_action("set-close-interception", Some("true".into()))
        .expect("the hosted window action parses");
    assert!(matches!(
        action,
        system::WindowAction::SetCloseInterception(true)
    ));

    crate::runtime::set_close_interception(4, true);
    assert!(crate::runtime::intercepts_close(4));
    assert!(!crate::runtime::intercepts_close(5));

    // Withdrawing the last listener lets native closes proceed again.
    crate::runtime::set_close_interception(4, false);
    assert!(!crate::runtime::intercepts_close(4));
    crate::runtime::reset_interception_state();
}

#[test]
fn hosted_close_interception_travels_as_a_fire_and_forget_mutation() {
    let host = HostCoordinator::new();
    host.enqueue(HostCommand::Mutation {
        app: 3,
        command: system::SystemCommand::WindowAction {
            window: 9,
            action: system::WindowAction::SetCloseInterception(true),
        },
    })
    .expect("a close-interception declaration fits the host queue");

    let mut commands = host.take_commands().expect("the host queue is readable");
    match commands.pop_front().expect("the declaration was queued") {
        HostCommand::Mutation {
            app,
            command:
                system::SystemCommand::WindowAction {
                    window,
                    action: system::WindowAction::SetCloseInterception(intercepting),
                },
        } => {
            assert_eq!(app, 3);
            assert_eq!(window, 9);
            assert!(intercepting);
        }
        _ => panic!("close interception must never wait on a synchronous reply"),
    }
    assert!(commands.is_empty());
}

#[test]
fn quit_interception_is_declared_before_the_before_quit_phase() {
    crate::runtime::reset_interception_state();
    assert!(!crate::runtime::intercepts_quit());
    crate::runtime::set_quit_interception(true);
    assert!(crate::runtime::intercepts_quit());
    crate::runtime::reset_interception_state();
    assert!(!crate::runtime::intercepts_quit());
}

#[test]
fn quit_reasons_reach_javascript_with_stable_names() {
    for (reason, name) in [
        (quickgui::QuitReason::Explicit, "explicit"),
        (quickgui::QuitReason::Relaunch, "relaunch"),
        (quickgui::QuitReason::LastWindowClosed, "last-window-closed"),
        (quickgui::QuitReason::OperatingSystem, "operating-system"),
    ] {
        assert_eq!(crate::runtime::quit_reason_name(reason), name);
    }
}

#[test]
fn window_tab_and_character_palette_actions_parse_from_the_hosted_boundary() {
    for (action, expected) in [
        (
            "show-character-palette",
            system::WindowAction::ShowCharacterPalette,
        ),
        ("select-next-tab", system::WindowAction::SelectNextTab),
        (
            "select-previous-tab",
            system::WindowAction::SelectPreviousTab,
        ),
        ("merge-all-windows", system::WindowAction::MergeAllWindows),
        (
            "move-tab-to-new-window",
            system::WindowAction::MoveTabToNewWindow,
        ),
        ("toggle-tab-bar", system::WindowAction::ToggleTabBar),
        (
            "toggle-tab-overview",
            system::WindowAction::ToggleTabOverview,
        ),
    ] {
        let parsed = crate::system::parse_window_action(action, None)
            .unwrap_or_else(|error| panic!("{action} parses: {error}"));
        assert_eq!(
            std::mem::discriminant(&parsed),
            std::mem::discriminant(&expected),
            "action {action}"
        );
    }

    assert!(matches!(
        crate::system::parse_window_action("select-tab", Some("3".into())).unwrap(),
        system::WindowAction::SelectTab(3)
    ));
    assert!(crate::system::parse_window_action("select-tab", Some("-1".into())).is_err());
    assert!(crate::system::parse_window_action("select-tab", None).is_err());

    assert!(matches!(
        crate::system::parse_window_action("set-tabbing-identifier", Some("docs".into())).unwrap(),
        system::WindowAction::SetTabbingIdentifier(Some(identifier)) if identifier == "docs"
    ));
    assert!(matches!(
        crate::system::parse_window_action("set-tabbing-identifier", Some(String::new())).unwrap(),
        system::WindowAction::SetTabbingIdentifier(None)
    ));
}

#[test]
fn a_declared_close_interception_holds_the_window_and_reports_it_to_javascript() {
    crate::runtime::reset_interception_state();
    let events: EventQueue = Rc::new(RefCell::new(VecDeque::new()));
    let view = NativeView {
        window: 21,
        handles: None,
        tree: Rc::new(RefCell::new(NativeTree::default())),
        events: Rc::clone(&events),
        markdown: Rc::new(RefCell::new(HashMap::new())),
        svgs: Rc::new(RefCell::new(HashMap::new())),
        lists: Rc::new(RefCell::new(HashMap::new())),
        terminals: Rc::new(RefCell::new(HashMap::new())),
        #[cfg(target_os = "macos")]
        swift_ui_hosts: Rc::new(RefCell::new(HashMap::new())),
        embedded_views: Rc::new(RefCell::new(HashMap::new())),
    };
    let (mut cx, view) = quickgui::TestAppContext::new(view).unwrap();
    let window = view.window_handle();

    // While interception is declared the native close is held and reported to JavaScript.
    crate::runtime::set_close_interception(21, true);
    assert!(!cx.simulate_close_requested(window).unwrap());
    assert_eq!(
        events
            .borrow()
            .iter()
            .filter(|event| event.kind == "close-requested" && event.window == 21)
            .count(),
        1
    );

    // Withdrawing the last listener lets the next native close proceed with no extra event.
    crate::runtime::set_close_interception(21, false);
    assert!(cx.simulate_close_requested(window).unwrap());
    assert_eq!(
        events
            .borrow()
            .iter()
            .filter(|event| event.kind == "close-requested")
            .count(),
        1
    );
    crate::runtime::reset_interception_state();
}
