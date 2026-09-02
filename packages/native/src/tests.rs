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
        images: Rc::new(RefCell::new(HashMap::new())),
        shaders: Rc::new(RefCell::new(HashMap::new())),
        menus: Rc::new(RefCell::new(HashMap::new())),
        context_menu: ContextMenuState::new(),
        context_menu_owner: None,
        focused_node: None,
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
        images: Rc::new(RefCell::new(HashMap::new())),
        shaders: Rc::new(RefCell::new(HashMap::new())),
        menus: Rc::new(RefCell::new(HashMap::new())),
        context_menu: ContextMenuState::new(),
        context_menu_owner: None,
        focused_node: None,
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
        images: Rc::new(RefCell::new(HashMap::new())),
        shaders: Rc::new(RefCell::new(HashMap::new())),
        menus: Rc::new(RefCell::new(HashMap::new())),
        context_menu: ContextMenuState::new(),
        context_menu_owner: None,
        focused_node: None,
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
        images: Rc::new(RefCell::new(HashMap::new())),
        shaders: Rc::new(RefCell::new(HashMap::new())),
        menus: Rc::new(RefCell::new(HashMap::new())),
        context_menu: ContextMenuState::new(),
        context_menu_owner: None,
        focused_node: None,
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
        images: Rc::new(RefCell::new(HashMap::new())),
        shaders: Rc::new(RefCell::new(HashMap::new())),
        menus: Rc::new(RefCell::new(HashMap::new())),
        context_menu: ContextMenuState::new(),
        context_menu_owner: None,
        focused_node: None,
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
        images: Rc::new(RefCell::new(HashMap::new())),
        shaders: Rc::new(RefCell::new(HashMap::new())),
        menus: Rc::new(RefCell::new(HashMap::new())),
        context_menu: ContextMenuState::new(),
        context_menu_owner: None,
        focused_node: None,
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
        images: Rc::new(RefCell::new(HashMap::new())),
        shaders: Rc::new(RefCell::new(HashMap::new())),
        menus: Rc::new(RefCell::new(HashMap::new())),
        context_menu: ContextMenuState::new(),
        context_menu_owner: None,
        focused_node: None,
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
        images: Rc::new(RefCell::new(HashMap::new())),
        shaders: Rc::new(RefCell::new(HashMap::new())),
        menus: Rc::new(RefCell::new(HashMap::new())),
        context_menu: ContextMenuState::new(),
        context_menu_owner: None,
        focused_node: None,
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

/// Build one component-part node with the string, boolean, and numeric properties it declares.
fn component_part_node(
    tag: NodeTag,
    parent: u32,
    part: &str,
    strings: &[(u16, &str)],
    flags: &[(u16, bool)],
) -> NativeNode {
    let mut node = NativeNode::new(tag);
    node.parent = Some(parent);
    node.set_property(property::PART, Some(PropertyValue::String(Arc::from(part))));
    node.set_property(property::WIDTH, Some(PropertyValue::Number(120.0)));
    node.set_property(property::HEIGHT, Some(PropertyValue::Number(32.0)));
    for (key, value) in strings {
        node.set_property(*key, Some(PropertyValue::String(Arc::from(*value))));
    }
    for (key, value) in flags {
        node.set_property(*key, Some(PropertyValue::Bool(*value)));
    }
    node
}

fn component_part_view(window: u32, tree: NativeTree, events: EventQueue) -> NativeView {
    NativeView {
        window,
        handles: None,
        tree: Rc::new(RefCell::new(tree)),
        events,
        markdown: Rc::new(RefCell::new(HashMap::new())),
        svgs: Rc::new(RefCell::new(HashMap::new())),
        lists: Rc::new(RefCell::new(HashMap::new())),
        terminals: Rc::new(RefCell::new(HashMap::new())),
        images: Rc::new(RefCell::new(HashMap::new())),
        shaders: Rc::new(RefCell::new(HashMap::new())),
        menus: Rc::new(RefCell::new(HashMap::new())),
        context_menu: ContextMenuState::new(),
        context_menu_owner: None,
        focused_node: None,
        #[cfg(target_os = "macos")]
        swift_ui_hosts: Rc::new(RefCell::new(HashMap::new())),
        embedded_views: Rc::new(RefCell::new(HashMap::new())),
    }
}

#[test]
fn component_part_properties_decode_core_identities_and_mount_policy() {
    let tabs = Tabs::new("tabset", "one");

    let inactive_panel = component_part_node(
        NodeTag::View,
        0,
        "tab-panel",
        &[
            (property::SCOPE, "tabset"),
            (property::ACTIVE_VALUE, "one"),
            (property::PART_VALUE, "two"),
        ],
        &[],
    );
    assert_eq!(
        native_part_element_id(5, &inactive_panel),
        Some(tabs.tab("two").panel_id())
    );
    assert!(apply_part(div(), 5, &inactive_panel).is_none());

    let mut retained_panel = inactive_panel.clone();
    retained_panel.set_property(property::KEEP_MOUNTED, Some(PropertyValue::Bool(true)));
    assert_eq!(
        native_part_element_id(5, &retained_panel),
        Some(tabs.tab("two").panel_id())
    );
    assert!(apply_part(div(), 5, &retained_panel).is_some());

    let active_panel = component_part_node(
        NodeTag::View,
        0,
        "tab-panel",
        &[
            (property::SCOPE, "tabset"),
            (property::ACTIVE_VALUE, "one"),
            (property::PART_VALUE, "one"),
        ],
        &[],
    );
    assert_eq!(
        native_part_element_id(6, &active_panel),
        Some(tabs.tab("one").panel_id())
    );
    assert!(apply_part(div(), 6, &active_panel).is_some());

    let closed_panel = component_part_node(
        NodeTag::View,
        0,
        "collapsible-panel",
        &[(property::SCOPE, "disclosure")],
        &[(property::OPEN, false)],
    );
    assert_eq!(
        native_part_element_id(7, &closed_panel),
        Some(Collapsible::new("disclosure", false).panel_id())
    );
    assert!(apply_part(div(), 7, &closed_panel).is_none());

    let field_error = component_part_node(
        NodeTag::View,
        0,
        "field-error",
        &[(property::SCOPE, "email")],
        &[(property::INVALID, true)],
    );
    assert_eq!(
        native_part_element_id(8, &field_error),
        Some(Field::new("email").error_id())
    );
    assert!(apply_part(div(), 8, &field_error).is_some());

    // An unknown part name keeps the ordinary node identity and decorates nothing.
    let unknown = component_part_node(NodeTag::View, 0, "not-a-part", &[], &[]);
    assert_eq!(native_part_element_id(9, &unknown), None);
    assert!(apply_part(div(), 9, &unknown).is_some());

    // Scopes and item values are bounded; an oversized key falls back to the node identity.
    let oversized = component_part_node(
        NodeTag::View,
        0,
        "fieldset",
        &[(property::SCOPE, &"s".repeat(MAX_COMPONENT_VALUE_BYTES + 1))],
        &[],
    );
    assert_eq!(
        native_part_element_id(10, &oversized),
        Some(ElementId::new(10))
    );

    // A tab without a bounded value cannot resolve a core identity and mounts undecorated.
    let valueless_tab =
        component_part_node(NodeTag::View, 0, "tab", &[(property::SCOPE, "tabset")], &[]);
    assert_eq!(native_part_element_id(11, &valueless_tab), None);
    assert!(apply_part(div(), 11, &valueless_tab).is_some());
}

#[test]
fn native_tabs_parts_mount_core_panels_and_roving_arrow_navigation() {
    let root_id = 100;
    let list_id = 101;
    let first_tab_id = 102;
    let second_tab_id = 103;
    let first_panel_id = 110;
    let second_panel_id = 111;
    let scope = [(property::SCOPE, "tabset"), (property::ACTIVE_VALUE, "one")];
    let mut tree = NativeTree::default();

    let mut root = component_part_node(NodeTag::View, ROOT_NODE, "tabs", &scope, &[]);
    root.children
        .extend([list_id, first_panel_id, second_panel_id]);
    tree.nodes.insert(root_id, root);

    let mut list = component_part_node(NodeTag::View, root_id, "tabs-list", &scope, &[]);
    list.children.extend([first_tab_id, second_tab_id]);
    tree.nodes.insert(list_id, list);

    for (id, value) in [(first_tab_id, "one"), (second_tab_id, "two")] {
        let mut strings = scope.to_vec();
        strings.push((property::PART_VALUE, value));
        tree.nodes.insert(
            id,
            component_part_node(NodeTag::Button, list_id, "tab", &strings, &[]),
        );
    }
    for (id, value) in [(first_panel_id, "one"), (second_panel_id, "two")] {
        let mut strings = scope.to_vec();
        strings.push((property::PART_VALUE, value));
        tree.nodes.insert(
            id,
            component_part_node(NodeTag::View, root_id, "tab-panel", &strings, &[]),
        );
    }
    tree.nodes
        .get_mut(&ROOT_NODE)
        .unwrap()
        .children
        .push(root_id);

    let view = component_part_view(6, tree, Rc::new(RefCell::new(VecDeque::new())));
    let (mut cx, view) = quickgui::TestAppContext::new(view).unwrap();
    let window = view.window_handle();

    let tabs = Tabs::new("tabset", "one");
    let first = tabs.tab("one");
    let second = tabs.tab("two");
    assert!(cx.contains_element(window, first.tab_id()).unwrap());
    assert!(cx.contains_element(window, second.tab_id()).unwrap());
    assert!(cx.contains_element(window, first.panel_id()).unwrap());
    assert!(!cx.contains_element(window, second.panel_id()).unwrap());
    // The list identity is the core's derived one, not either node id.
    assert!(cx.contains_element(window, tabs.list_id()).unwrap());
    assert!(
        !cx.contains_element(window, ElementId::new(list_id as u64))
            .unwrap()
    );

    cx.focus(window, first.tab_id()).unwrap();
    cx.simulate_keystrokes(window, "right").unwrap();
    assert_eq!(cx.focused(window).unwrap(), Some(second.tab_id()));
    cx.simulate_keystrokes(window, "right").unwrap();
    assert_eq!(cx.focused(window).unwrap(), Some(first.tab_id()));
}

#[test]
fn selection_and_field_parts_adopt_core_focus_activation_and_labelling() {
    let checkbox_id = 120;
    let plain_id = 121;
    let field_root_id = 130;
    let label_id = 131;
    let control_id = 132;
    let mut tree = NativeTree::default();

    tree.nodes.insert(
        checkbox_id,
        component_part_node(
            NodeTag::View,
            ROOT_NODE,
            "checkbox",
            &[],
            &[(property::CHECKED, true)],
        ),
    );
    let mut plain = NativeNode::new(NodeTag::View);
    plain.parent = Some(ROOT_NODE);
    plain.set_property(property::WIDTH, Some(PropertyValue::Number(120.0)));
    plain.set_property(property::HEIGHT, Some(PropertyValue::Number(32.0)));
    tree.nodes.insert(plain_id, plain);

    let field = [(property::SCOPE, "email")];
    let mut field_root = component_part_node(NodeTag::View, ROOT_NODE, "field", &field, &[]);
    field_root.children.extend([label_id, control_id]);
    tree.nodes.insert(field_root_id, field_root);
    tree.nodes.insert(
        label_id,
        component_part_node(NodeTag::View, field_root_id, "field-label", &field, &[]),
    );
    tree.nodes.insert(
        control_id,
        component_part_node(NodeTag::Input, field_root_id, "field-control", &field, &[]),
    );
    tree.nodes
        .get_mut(&ROOT_NODE)
        .unwrap()
        .children
        .extend([checkbox_id, plain_id, field_root_id]);

    let view = component_part_view(7, tree, Rc::new(RefCell::new(VecDeque::new())));
    let (mut cx, view) = quickgui::TestAppContext::new(view).unwrap();
    let window = view.window_handle();

    // The core checkbox part supplies focus and click behavior the plain view does not have.
    let checkbox_element = ElementId::new(checkbox_id as u64);
    cx.focus(window, checkbox_element).unwrap();
    assert_eq!(cx.focused(window).unwrap(), Some(checkbox_element));
    cx.click(window, checkbox_element).unwrap();
    assert!(cx.focus(window, ElementId::new(plain_id as u64)).is_err());

    // The field label activates the control it names through the core relationship.
    let field = Field::new("email");
    assert!(cx.contains_element(window, field.root_id()).unwrap());
    cx.click(window, field.label_id()).unwrap();
    assert_eq!(cx.focused(window).unwrap(), Some(field.control_id()));
}

#[test]
fn in_window_dialog_parts_mount_only_while_open_and_dismiss_through_the_core() {
    let trigger_id = 140;
    let portal_id = 141;
    let backdrop_id = 142;
    let popup_id = 143;
    let title_id = 144;
    let close_id = 145;
    let dialog = [(property::SCOPE, "confirm")];
    let mut tree = NativeTree::default();

    tree.nodes.insert(
        trigger_id,
        component_part_node(
            NodeTag::Button,
            ROOT_NODE,
            "dialog-trigger",
            &dialog,
            &[(property::OPEN, true)],
        ),
    );
    let mut portal = component_part_node(
        NodeTag::View,
        ROOT_NODE,
        "dialog",
        &dialog,
        &[(property::OPEN, true)],
    );
    portal.children.extend([backdrop_id, popup_id]);
    tree.nodes.insert(portal_id, portal);
    tree.nodes.insert(
        backdrop_id,
        component_part_node(
            NodeTag::View,
            portal_id,
            "dialog-backdrop",
            &dialog,
            &[(property::OPEN, true)],
        ),
    );
    let mut popup = component_part_node(
        NodeTag::View,
        portal_id,
        "dialog-popup",
        &dialog,
        &[
            (property::OPEN, true),
            (property::DISMISS_ON_ESCAPE, true),
            (property::DISMISS_LISTENER, true),
        ],
    );
    popup.children.extend([title_id, close_id]);
    tree.nodes.insert(popup_id, popup);
    tree.nodes.insert(
        title_id,
        component_part_node(
            NodeTag::View,
            popup_id,
            "dialog-title",
            &dialog,
            &[(property::OPEN, true)],
        ),
    );
    tree.nodes.insert(
        close_id,
        component_part_node(
            NodeTag::Button,
            popup_id,
            "dialog-close",
            &dialog,
            &[(property::OPEN, true)],
        ),
    );
    tree.nodes
        .get_mut(&ROOT_NODE)
        .unwrap()
        .children
        .extend([trigger_id, portal_id]);

    let events: EventQueue = Rc::new(RefCell::new(VecDeque::new()));
    let view = component_part_view(8, tree, Rc::clone(&events));
    let (mut cx, view) = quickgui::TestAppContext::new(view).unwrap();
    let window = view.window_handle();

    let core = quickgui::Dialog::new("confirm", true);
    assert!(cx.contains_element(window, core.root_id()).unwrap());
    assert!(cx.contains_element(window, core.popover_id()).unwrap());
    assert!(cx.contains_element(window, core.title_id()).unwrap());
    assert!(cx.contains_element(window, core.close_id()).unwrap());

    // Escape reaches the core surface and is reported back as one asynchronous dismiss event.
    cx.focus(window, core.popover_id()).unwrap();
    cx.simulate_keystrokes(window, "escape").unwrap();
    let event = events.borrow_mut().pop_front().unwrap();
    assert_eq!(event.kind, "dismiss");
    assert_eq!(event.target, popup_id);

    // A closed dialog contributes no portal, backdrop, popup, or title at all.
    cx.update(view, |view, cx| {
        let mut tree = view.tree.borrow_mut();
        for id in [
            trigger_id,
            portal_id,
            backdrop_id,
            popup_id,
            title_id,
            close_id,
        ] {
            tree.nodes
                .get_mut(&id)
                .unwrap()
                .set_property(property::OPEN, Some(PropertyValue::Bool(false)));
        }
        cx.invalidate();
    })
    .unwrap();

    assert!(!cx.contains_element(window, core.root_id()).unwrap());
    assert!(!cx.contains_element(window, core.popover_id()).unwrap());
    assert!(!cx.contains_element(window, core.backdrop_id()).unwrap());
    assert!(!cx.contains_element(window, core.title_id()).unwrap());
    assert!(
        cx.contains_element(window, ElementId::new(trigger_id as u64))
            .unwrap()
    );
}

const DECLARED_MENU: &str = r#"{
  "width": 200,
  "itemHeight": 30,
  "items": [
    { "type": "group", "label": "File" },
    { "id": "open", "label": "Open…", "shortcut": "⌘O" },
    { "type": "separator" },
    { "type": "checkbox", "id": "sidebar", "label": "Show sidebar", "checked": true },
    { "type": "radio", "id": "small", "group": "size", "label": "Small", "checked": true },
    { "id": "recent", "label": "Recent", "items": [{ "id": "one", "label": "One" }] },
    { "id": "quit", "label": "Quit", "disabled": true },
    { "label": "no identifier" }
  ]
}"#;

#[test]
fn declared_menu_entries_adopt_core_kinds_bounds_and_shortcuts() {
    let declaration =
        NativeMenuDeclaration::parse(DECLARED_MENU).expect("the declaration is bounded JSON");
    let style = declaration.style();
    assert_eq!(style.width, 200.0);
    assert_eq!(style.item_height, 30.0);
    // Unspecified measurements keep the core's own defaults instead of a JavaScript guess.
    assert_eq!(
        style.separator_height,
        NativeMenuStyle::default().separator_height
    );

    let items = menu_items(7, declaration.entries(), 0);
    // The entry without a stable identifier cannot form a core command and is skipped.
    assert_eq!(items.len(), 7);
    let kinds = items.iter().map(PopoverMenuItem::kind).collect::<Vec<_>>();
    assert_eq!(
        kinds,
        vec![
            PopoverMenuItemKind::GroupLabel,
            PopoverMenuItemKind::Action,
            PopoverMenuItemKind::Separator,
            PopoverMenuItemKind::Checkbox,
            PopoverMenuItemKind::Radio,
            PopoverMenuItemKind::Submenu,
            PopoverMenuItemKind::Action,
        ]
    );
    assert_eq!(
        items[1].shortcut_text().map(|value| &**value),
        Some("\u{2318}O")
    );
    assert_eq!(items[3].checked(), Some(true));
    assert_eq!(items[4].checked(), Some(true));
    assert!(items[6].is_disabled());
    assert_eq!(
        items[5]
            .submenu_menu()
            .expect("a declared submenu carries a validated core model")
            .items()
            .len(),
        1
    );

    // The whole declaration validates through the core before any of it is retained.
    let menu = PopoverMenu::new(items).expect("the declared tree passes core validation");
    assert_eq!(menu.items().len(), 7);
}

#[test]
fn malformed_and_oversized_menu_declarations_are_rejected_without_panicking() {
    assert!(NativeMenuDeclaration::parse("not json").is_none());
    assert!(NativeMenuDeclaration::parse(&"a".repeat(MAX_MENU_JSON_BYTES + 1)).is_none());

    // A bounded declaration with an oversized identifier keeps the rest of the menu.
    let oversized = format!(
        r#"{{"items":[{{"id":"{}","label":"Too long"}},{{"id":"ok","label":"Fine"}}]}}"#,
        "x".repeat(MAX_MENU_ITEM_ID_BYTES + 1)
    );
    let declaration = NativeMenuDeclaration::parse(&oversized).expect("the JSON itself is bounded");
    let items = menu_items(3, declaration.entries(), 0);
    assert_eq!(items.len(), 1);
    assert_eq!(&**items[0].label(), "Fine");

    // An empty declaration still forms a valid, inert model.
    let empty = NativeMenuDeclaration::parse("{}").expect("an empty object is a valid declaration");
    assert!(menu_items(3, empty.entries(), 0).is_empty());
}

fn menu_tree(popup_id: u32, trigger_id: u32, open: bool) -> NativeTree {
    let mut tree = NativeTree::default();
    let mut trigger = component_part_node(
        NodeTag::Button,
        ROOT_NODE,
        POPOVER_MENU_TRIGGER_PART,
        &[(property::CONTROLS, &popup_id.to_string())],
        &[(property::OPEN, open)],
    );
    trigger.children.clear();
    tree.nodes.insert(trigger_id, trigger);

    let popup = component_part_node(
        NodeTag::View,
        ROOT_NODE,
        POPOVER_MENU_POPUP_PART,
        &[
            (property::MENU, DECLARED_MENU),
            (property::ANCHOR_TARGET, &trigger_id.to_string()),
        ],
        &[
            (property::SELECT_LISTENER, true),
            (property::DISMISS_LISTENER, true),
        ],
    );
    tree.nodes.insert(popup_id, popup);

    let root = tree.nodes.get_mut(&ROOT_NODE).unwrap();
    root.children.push(trigger_id);
    if open {
        root.children.push(popup_id);
    }
    tree
}

#[test]
fn declared_popover_menu_rows_navigate_and_activate_through_the_core_model() {
    let trigger_id = 300;
    let popup_id = 301;
    let events: EventQueue = Rc::new(RefCell::new(VecDeque::new()));
    let view = component_part_view(9, menu_tree(popup_id, trigger_id, true), Rc::clone(&events));
    let (mut cx, view) = quickgui::TestAppContext::from_application(
        quickgui::Application::new().bind_keys(quickgui::popover_menu_key_bindings()),
        quickgui::WindowOptions::default(),
        view,
    )
    .unwrap();
    let window = view.window_handle();
    let surface = ElementId::new(popup_id as u64);

    let declaration = NativeMenuDeclaration::parse(DECLARED_MENU).unwrap();
    let menu = PopoverMenu::new(menu_items(popup_id, declaration.entries(), 0)).unwrap();
    // Rows mount with the core's derived identities, not the declaring node identity.
    let open_row = menu
        .item_element_id(surface, 1)
        .expect("an interactive row has a derived identity");
    assert!(cx.contains_element(window, open_row).unwrap());
    assert!(
        cx.contains_element(window, menu.item_element_id(surface, 0).unwrap())
            .unwrap()
    );

    cx.focus(window, surface).unwrap();
    // The first enabled entry is highlighted by the core, so one Down lands on the checkbox.
    cx.simulate_keystrokes(window, "down").unwrap();
    cx.simulate_keystrokes(window, "enter").unwrap();

    let selected = events
        .borrow()
        .iter()
        .find(|event| event.kind == "menuselect")
        .cloned()
        .expect("activating a row queues one selection event");
    assert_eq!(selected.target, popup_id);
    assert_eq!(selected.window, 9);
    let payload = selected.value.expect("a selection carries its declared id");
    assert!(
        payload.contains("\"id\":\"sidebar\""),
        "payload was {payload}"
    );
    // A checkbox reports the value the core computed, never a JavaScript-side guess.
    assert!(
        payload.contains("\"checked\":false"),
        "payload was {payload}"
    );
}

#[test]
fn a_declared_context_menu_target_opens_the_core_cursor_point_surface() {
    let target_id = 320;
    let mut tree = NativeTree::default();
    tree.nodes.insert(
        target_id,
        component_part_node(
            NodeTag::View,
            ROOT_NODE,
            CONTEXT_MENU_TRIGGER_PART,
            &[(property::MENU, DECLARED_MENU)],
            &[(property::SELECT_LISTENER, true)],
        ),
    );
    tree.nodes
        .get_mut(&ROOT_NODE)
        .unwrap()
        .children
        .push(target_id);

    let events: EventQueue = Rc::new(RefCell::new(VecDeque::new()));
    let view = component_part_view(11, tree, Rc::clone(&events));
    let (mut cx, view) = quickgui::TestAppContext::from_application(
        quickgui::Application::new().bind_keys(quickgui::popover_menu_key_bindings()),
        quickgui::WindowOptions::default(),
        view,
    )
    .unwrap();
    let window = view.window_handle();
    let target = ElementId::new(target_id as u64);
    assert!(cx.contains_element(window, target).unwrap());
    assert_eq!(cx.windows().len(), 1);

    cx.simulate_context_menu(
        window,
        target,
        quickgui::Point::new(40.0, 24.0),
        quickgui::Modifiers::empty(),
    )
    .unwrap();
    // The core owns the separate cursor-point surface; JavaScript never creates or measures it.
    assert_eq!(
        cx.windows().len(),
        2,
        "a declared context menu opens exactly one core-owned surface"
    );
}

#[test]
fn declared_css_grid_tracks_and_placement_lay_out_through_taffy() {
    let grid_id = 400;
    let first_id = 401;
    let second_id = 402;
    let mut tree = NativeTree::default();

    let mut grid = NativeNode::new(NodeTag::View);
    grid.parent = Some(ROOT_NODE);
    grid.set_property(
        property::DISPLAY,
        Some(PropertyValue::String(Arc::from("grid"))),
    );
    grid.set_property(
        property::GRID_TEMPLATE_COLUMNS,
        Some(PropertyValue::String(Arc::from("100px 1fr"))),
    );
    grid.set_property(
        property::GRID_TEMPLATE_ROWS,
        Some(PropertyValue::String(Arc::from("repeat(1, 40px)"))),
    );
    grid.set_property(property::WIDTH, Some(PropertyValue::Number(300.0)));
    grid.set_property(property::HEIGHT, Some(PropertyValue::Number(40.0)));
    grid.children.extend([first_id, second_id]);
    tree.nodes.insert(grid_id, grid);

    for id in [first_id, second_id] {
        let mut cell = NativeNode::new(NodeTag::View);
        cell.parent = Some(grid_id);
        tree.nodes.insert(id, cell);
    }
    tree.nodes
        .get_mut(&ROOT_NODE)
        .unwrap()
        .children
        .push(grid_id);

    let view = component_part_view(12, tree, Rc::new(RefCell::new(VecDeque::new())));
    let (mut cx, view) = quickgui::TestAppContext::new(view).unwrap();
    let window = view.window_handle();

    let first = cx
        .element_bounds(window, ElementId::new(first_id as u64))
        .unwrap();
    let second = cx
        .element_bounds(window, ElementId::new(second_id as u64))
        .unwrap();
    assert_eq!(first.width, 100.0);
    assert_eq!(second.x - first.x, 100.0);
    assert_eq!(second.width, 200.0);
    assert_eq!(first.height, 40.0);
}

#[test]
fn declared_transitions_map_css_names_onto_core_paint_flags_and_easing() {
    let mut node = NativeNode::new(NodeTag::View);
    // The shorthand-only declaration keeps the previous colors-only behavior.
    node.set_property(property::TRANSITION, Some(PropertyValue::Number(120.0)));
    let transition = native_transition(&node).expect("a duration declares a transition");
    assert!((transition.duration.as_secs_f32() - 0.120).abs() < 0.001);
    assert_eq!(
        transition.properties,
        quickgui::TransitionProperties::COLORS
    );

    node.set_property(
        property::TRANSITION_PROPERTIES,
        Some(PropertyValue::String(Arc::from(
            "opacity,box-shadow,border-radius",
        ))),
    );
    node.set_property(
        property::TRANSITION_DURATION,
        Some(PropertyValue::Number(240.0)),
    );
    node.set_property(
        property::TRANSITION_EASING,
        Some(PropertyValue::String(Arc::from("linear"))),
    );
    node.set_property(
        property::TRANSITION_MAX_FPS,
        Some(PropertyValue::Number(30.0)),
    );
    let transition = native_transition(&node).expect("an explicit duration declares a transition");
    assert!((transition.duration.as_secs_f32() - 0.240).abs() < 0.001);
    assert_eq!(
        transition.properties,
        quickgui::TransitionProperties::OPACITY
            | quickgui::TransitionProperties::BOX_SHADOW
            | quickgui::TransitionProperties::BORDER_RADIUS
    );
    assert_eq!(transition.max_fps, Some(30.0));
    // Easing comes from the core's own curves, so linear is the identity.
    assert_eq!((transition.easing)(0.25), 0.25);

    // An unknown property name falls back to the core's color set instead of an empty transition.
    node.set_property(
        property::TRANSITION_PROPERTIES,
        Some(PropertyValue::String(Arc::from("left,width"))),
    );
    assert_eq!(
        native_transition(&node).unwrap().properties,
        quickgui::TransitionProperties::COLORS
    );

    let mut plain = NativeNode::new(NodeTag::View);
    plain.set_property(property::COLOR, Some(PropertyValue::Color(0xff00_00ff)));
    assert!(native_transition(&plain).is_none());
}

#[test]
fn range_and_feedback_parts_adopt_core_value_ranges() {
    let mut progress = component_part_node(NodeTag::View, ROOT_NODE, "progress", &[], &[]);
    progress.set_property(property::VALUE, Some(PropertyValue::Number(3.0)));
    progress.set_property(property::MAXIMUM, Some(PropertyValue::Number(12.0)));
    progress.set_property(
        property::VALUE_TEXT,
        Some(PropertyValue::String(Arc::from("3 of 12 files"))),
    );
    let declared = native_progress(&progress);
    assert_eq!(declared.value(), Some(3.0));
    assert_eq!(declared.maximum(), 12.0);
    assert_eq!(declared.completion(), Some(0.25));
    assert!(apply_part(div(), 41, &progress).is_some());

    progress.set_property(property::INDETERMINATE, Some(PropertyValue::Bool(true)));
    assert!(native_progress(&progress).is_indeterminate());

    let mut meter = component_part_node(NodeTag::View, ROOT_NODE, "meter", &[], &[]);
    meter.set_property(property::VALUE, Some(PropertyValue::Number(20.0)));
    meter.set_property(property::MINIMUM, Some(PropertyValue::Number(0.0)));
    meter.set_property(property::MAXIMUM, Some(PropertyValue::Number(100.0)));
    meter.set_property(property::LOW, Some(PropertyValue::Number(25.0)));
    meter.set_property(property::HIGH, Some(PropertyValue::Number(75.0)));
    let declared = native_meter(&meter);
    assert_eq!(declared.completion(), 0.2);
    assert!(declared.is_low());
    assert!(!declared.is_high());
    assert!(apply_part(div(), 42, &meter).is_some());

    let pressed = component_part_node(
        NodeTag::Button,
        ROOT_NODE,
        "toggle",
        &[],
        &[(property::PRESSED, true)],
    );
    assert!(apply_part(div(), 43, &pressed).is_some());
    assert!(
        apply_part(
            div(),
            44,
            &component_part_node(NodeTag::View, ROOT_NODE, "toggle-indicator", &[], &[])
        )
        .is_some()
    );
}

/// A one-pixel opaque red PNG.
const RED_PIXEL_PNG: &str = "data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mP8z8BQDwAEhQGAhKmMIQAAAABJRU5ErkJggg==";

#[test]
fn declared_image_sources_decode_once_and_degrade_without_panicking() {
    let mut node = NativeNode::new(NodeTag::Image);
    node.set_property(
        property::VALUE,
        Some(PropertyValue::String(Arc::from(RED_PIXEL_PNG))),
    );
    node.set_property(
        property::OBJECT_FIT,
        Some(PropertyValue::String(Arc::from("cover"))),
    );
    let mut state = NativeImageState::new(Arc::from(RED_PIXEL_PNG));
    assert!(state.parsed.is_ok(), "a base64 data URL decodes once");
    // Re-declaring the same source keeps the decoded image.
    let before = state.source.clone();
    state.sync(RED_PIXEL_PNG);
    assert!(Arc::ptr_eq(&before, &state.source));

    state.sync("/tmp/quickgui-does-not-exist.png");
    // A path stays a lazy core resource; the core's worker pool owns the decode.
    assert!(state.parsed.is_ok());

    let broken = NativeImageState::new(Arc::from("data:image/png;base64,not-base64!!"));
    assert!(broken.parsed.is_err());
    assert!(NativeImageState::new(Arc::from("")).parsed.is_err());
}

#[test]
fn declared_shaders_validate_wgsl_and_bound_their_parameter_vectors() {
    let valid = "fn quickgui_fragment(input: QuickGuiShaderInput) -> vec4<f32> {\n\
         return vec4<f32>(input.uv, 0.0, 1.0);\n\
     }";
    let state = NativeShaderState::new(Arc::from(valid));
    assert!(state.parsed.is_ok(), "{:?}", state.parsed.as_ref().err());
    assert!(
        NativeShaderState::new(Arc::from("not wgsl"))
            .parsed
            .is_err()
    );
    assert!(NativeShaderState::new(Arc::from("")).parsed.is_err());

    let mut node = NativeNode::new(NodeTag::Shader);
    node.set_property(
        property::SHADER_PARAMETERS,
        Some(PropertyValue::String(Arc::from("[0.5,0.25,0,1]"))),
    );
    let parameters = native_shader_parameters(&node);
    assert_eq!(parameters.vectors()[0], [0.5, 0.25, 0.0, 1.0]);

    // Extra declared floats are ignored instead of growing the core's fixed uniform.
    let many = (0..64).map(|_| "1").collect::<Vec<_>>().join(",");
    node.set_property(
        property::SHADER_PARAMETERS,
        Some(PropertyValue::String(Arc::from(format!("[{many}]")))),
    );
    let parameters = native_shader_parameters(&node);
    assert_eq!(parameters.vectors().len(), 4);

    node.set_property(
        property::SHADER_PARAMETERS,
        Some(PropertyValue::String(Arc::from("not json"))),
    );
    assert_eq!(native_shader_parameters(&node).vectors()[0], [0.0; 4]);
}

fn input_node(parent: u32, flags: &[(u16, bool)], strings: &[(u16, &str)]) -> NativeNode {
    let mut node = NativeNode::new(NodeTag::View);
    node.parent = Some(parent);
    node.set_property(property::WIDTH, Some(PropertyValue::Number(200.0)));
    node.set_property(property::HEIGHT, Some(PropertyValue::Number(80.0)));
    node.set_property(property::TAB_INDEX, Some(PropertyValue::Number(0.0)));
    for (key, value) in flags {
        node.set_property(*key, Some(PropertyValue::Bool(*value)));
    }
    for (key, value) in strings {
        node.set_property(*key, Some(PropertyValue::String(Arc::from(*value))));
    }
    node
}

fn queued(events: &EventQueue, kind: &str) -> Option<QueuedEvent> {
    events
        .borrow()
        .iter()
        .find(|event| event.kind == kind)
        .cloned()
}

#[test]
fn declared_input_listeners_report_bounded_core_payloads() {
    let target_id = 500;
    let mut tree = NativeTree::default();
    tree.nodes.insert(
        target_id,
        input_node(
            ROOT_NODE,
            &[
                (property::KEY_DOWN_LISTENER, true),
                (property::KEY_UP_LISTENER, true),
                (property::MOUSE_DOWN_LISTENER, true),
                (property::MOUSE_UP_LISTENER, true),
                (property::MOUSE_MOVE_LISTENER, true),
                (property::DOUBLE_CLICK_LISTENER, true),
                (property::SCROLL_LISTENER, true),
                (property::CONTEXT_MENU_LISTENER, true),
                (property::PINCH_LISTENER, true),
                (property::ROTATION_LISTENER, true),
                (property::SMART_MAGNIFY_LISTENER, true),
                (property::PRESSURE_LISTENER, true),
                (property::FOCUS_LISTENER, true),
            ],
            &[],
        ),
    );
    tree.nodes
        .get_mut(&ROOT_NODE)
        .unwrap()
        .children
        .push(target_id);

    let events: EventQueue = Rc::new(RefCell::new(VecDeque::new()));
    let view = component_part_view(21, tree, Rc::clone(&events));
    let (mut cx, view) = quickgui::TestAppContext::new(view).unwrap();
    let window = view.window_handle();
    let target = ElementId::new(target_id as u64);

    cx.focus(window, target).unwrap();
    let focus = queued(&events, "focus").expect("focus is reported once");
    assert_eq!(focus.target, target_id);
    assert_eq!(focus.window, 21);

    cx.simulate_keystrokes(window, "cmd-shift-k").unwrap();
    let key = queued(&events, "keydown").expect("a declared key listener reports the press");
    let payload = key.value.expect("a key press carries its payload");
    assert!(payload.contains("\"key\":\"k\""), "payload was {payload}");
    assert!(payload.contains("\"meta\":true"), "payload was {payload}");
    assert!(payload.contains("\"shift\":true"), "payload was {payload}");
    assert!(
        payload.contains("\"repeat\":false"),
        "payload was {payload}"
    );

    cx.simulate_key_up(
        window,
        quickgui::Keystroke::parse("k").expect("a bounded keystroke"),
    )
    .unwrap();
    assert!(queued(&events, "keyup").is_some());

    cx.simulate_mouse_down(
        window,
        target,
        quickgui::MouseDownEvent {
            button: quickgui::MouseButton::Left,
            position: quickgui::Point::new(12.0, 8.0),
            modifiers: quickgui::Modifiers::empty(),
            click_count: 2,
            first_mouse: false,
        },
    )
    .unwrap();
    let down = queued(&events, "mousedown").expect("a mouse press is reported");
    let payload = down.value.expect("a mouse press carries its payload");
    assert!(
        payload.contains("\"button\":\"left\""),
        "payload was {payload}"
    );
    assert!(
        payload.contains("\"clickCount\":2"),
        "payload was {payload}"
    );
    // The exact native multi-click count comes from the core, not a JavaScript timer.
    assert!(queued(&events, "dblclick").is_some());

    cx.simulate_mouse_up(
        window,
        target,
        quickgui::MouseUpEvent {
            button: quickgui::MouseButton::Left,
            position: quickgui::Point::new(12.0, 8.0),
            modifiers: quickgui::Modifiers::empty(),
            click_count: 2,
        },
    )
    .unwrap();
    assert!(queued(&events, "mouseup").is_some());

    cx.simulate_mouse_move(
        window,
        target,
        quickgui::MouseMoveEvent {
            position: quickgui::Point::new(20.0, 10.0),
            pressed_button: None,
            modifiers: quickgui::Modifiers::empty(),
        },
    )
    .unwrap();
    assert!(queued(&events, "mousemove").is_some());

    cx.simulate_scroll_wheel(
        window,
        target,
        quickgui::ScrollWheelEvent {
            position: quickgui::Point::new(20.0, 10.0),
            delta: quickgui::ScrollDelta::Pixels(quickgui::Vector::new(0.0, -24.0)),
            phase: quickgui::GesturePhase::Moved,
            modifiers: quickgui::Modifiers::empty(),
        },
    )
    .unwrap();
    let wheel = queued(&events, "wheel").expect("a wheel event is reported");
    let payload = wheel.value.expect("a wheel event carries its payload");
    assert!(
        payload.contains("\"deltaY\":-24.0"),
        "payload was {payload}"
    );
    assert!(
        payload.contains("\"precise\":true"),
        "payload was {payload}"
    );

    cx.simulate_context_menu(
        window,
        target,
        quickgui::Point::new(30.0, 12.0),
        quickgui::Modifiers::CONTROL,
    )
    .unwrap();
    let menu = queued(&events, "contextmenu").expect("a secondary click is reported");
    assert!(
        menu.value
            .expect("a context-menu event carries its payload")
            .contains("\"control\":true")
    );

    cx.simulate_pinch(
        window,
        target,
        quickgui::PinchEvent {
            position: quickgui::Point::new(4.0, 4.0),
            delta: 0.1,
            phase: quickgui::GesturePhase::Started,
            modifiers: quickgui::Modifiers::empty(),
        },
    )
    .unwrap();
    let pinch = queued(&events, "pinch").expect("a pinch gesture is reported");
    assert!(
        pinch
            .value
            .expect("a pinch carries its payload")
            .contains("\"phase\":\"started\"")
    );

    cx.simulate_rotation(
        window,
        target,
        quickgui::RotationEvent {
            position: quickgui::Point::new(4.0, 4.0),
            delta: -12.0,
            phase: quickgui::GesturePhase::Moved,
            modifiers: quickgui::Modifiers::empty(),
        },
    )
    .unwrap();
    assert!(queued(&events, "rotate").is_some());

    cx.simulate_smart_magnify(
        window,
        target,
        quickgui::SmartMagnifyEvent {
            position: quickgui::Point::new(4.0, 4.0),
            modifiers: quickgui::Modifiers::empty(),
        },
    )
    .unwrap();
    assert!(queued(&events, "smartmagnify").is_some());

    cx.simulate_mouse_pressure(
        window,
        target,
        quickgui::MousePressureEvent {
            position: quickgui::Point::new(4.0, 4.0),
            pressure: 0.5,
            stage: quickgui::PressureStage::Force,
            modifiers: quickgui::Modifiers::empty(),
        },
    )
    .unwrap();
    let pressure = queued(&events, "pressure").expect("a pressure stage is reported");
    assert!(
        pressure
            .value
            .expect("a pressure event carries its payload")
            .contains("\"stage\":\"force\"")
    );
}

#[test]
fn a_declared_keymap_parses_accelerators_through_the_core_and_dispatches_ids() {
    let declaration =
        r#"{"CmdOrCtrl+S":"save","CmdOrCtrl+Shift+P":"palette","Nonsense++":"skipped","Alt+X":""}"#;
    let keymap = NativeKeymap::parse(declaration).expect("valid accelerators form a keymap");
    // The unparsable accelerator and the empty identifier are skipped, not fatal.
    assert_eq!(keymap.len(), 2);
    assert!(NativeKeymap::parse("not json").is_none());
    assert!(NativeKeymap::parse(&"x".repeat(MAX_KEYMAP_JSON_BYTES + 1)).is_none());

    let target_id = 520;
    let mut tree = NativeTree::default();
    tree.nodes.insert(
        target_id,
        input_node(
            ROOT_NODE,
            &[(property::ACTION_LISTENER, true)],
            &[(property::KEYMAP, declaration)],
        ),
    );
    tree.nodes
        .get_mut(&ROOT_NODE)
        .unwrap()
        .children
        .push(target_id);

    let events: EventQueue = Rc::new(RefCell::new(VecDeque::new()));
    let view = component_part_view(22, tree, Rc::clone(&events));
    let (mut cx, view) = quickgui::TestAppContext::new(view).unwrap();
    let window = view.window_handle();
    let target = ElementId::new(target_id as u64);

    cx.focus(window, target).unwrap();
    cx.simulate_keystrokes(window, "cmd-s").unwrap();
    let action = queued(&events, "action").expect("a declared accelerator dispatches its id");
    assert_eq!(action.target, target_id);
    assert_eq!(action.value.as_deref(), Some("save"));

    // An unbound keystroke without a declared key listener queues nothing.
    events.borrow_mut().clear();
    cx.simulate_keystrokes(window, "cmd-q").unwrap();
    assert!(queued(&events, "action").is_none());
    assert!(queued(&events, "keydown").is_none());
}

#[test]
fn declared_drag_sources_and_drop_targets_route_typed_core_payloads() {
    let source_id = 540;
    let target_id = 541;
    let mut tree = NativeTree::default();
    tree.nodes.insert(
        source_id,
        input_node(
            ROOT_NODE,
            &[(property::DRAG_LISTENER, true)],
            &[(
                property::DRAGGABLE,
                r#"{"id":"row-7","text":"quickgui","files":[{"path":"/tmp/a.txt"}]}"#,
            )],
        ),
    );
    tree.nodes.insert(
        target_id,
        input_node(
            ROOT_NODE,
            &[(property::DROP_LISTENER, true)],
            &[(property::DROP_KINDS, r#"["local","files"]"#)],
        ),
    );
    let root = tree.nodes.get_mut(&ROOT_NODE).unwrap();
    root.children.push(source_id);
    root.children.push(target_id);

    let events: EventQueue = Rc::new(RefCell::new(VecDeque::new()));
    let view = component_part_view(23, tree, Rc::clone(&events));
    let (cx, view) = quickgui::TestAppContext::new(view).unwrap();
    let window = view.window_handle();
    // Both the declared drag source and both declared drop kinds mount on the core element tree.
    assert!(
        cx.contains_element(window, ElementId::new(source_id as u64))
            .unwrap()
    );
    assert!(
        cx.contains_element(window, ElementId::new(target_id as u64))
            .unwrap()
    );
    assert!(events.borrow().is_empty());
}

#[test]
fn a_drag_source_that_also_captures_the_pointer_is_ignored_instead_of_panicking() {
    let node_id = 560;
    let mut tree = NativeTree::default();
    let mut conflicting = input_node(
        ROOT_NODE,
        &[
            (property::POINTER_LISTENER, true),
            (property::DRAG_LISTENER, true),
            (property::DROP_LISTENER, true),
        ],
        &[
            (property::DRAGGABLE, r#"{"id":"row-1"}"#),
            (property::DROP_KINDS, r#"["local"]"#),
        ],
    );
    conflicting.parent = Some(ROOT_NODE);
    tree.nodes.insert(node_id, conflicting);
    tree.nodes
        .get_mut(&ROOT_NODE)
        .unwrap()
        .children
        .push(node_id);

    let events: EventQueue = Rc::new(RefCell::new(VecDeque::new()));
    let view = component_part_view(24, tree, Rc::clone(&events));
    // The core rejects one element owning both, so the drag source is dropped and the element and
    // its declared drop target still mount.
    let (cx, view) = quickgui::TestAppContext::new(view).unwrap();
    assert!(
        cx.contains_element(view.window_handle(), ElementId::new(node_id as u64))
            .unwrap()
    );
}
