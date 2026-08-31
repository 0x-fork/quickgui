use super::*;

pub(super) struct NativeView {
    pub(super) window: u32,
    pub(super) handles: Option<Rc<RefCell<HashMap<WindowHandle, u32>>>>,
    pub(super) tree: Rc<RefCell<NativeTree>>,
    pub(super) events: EventQueue,
    pub(super) markdown: Rc<RefCell<HashMap<u32, Markdown>>>,
    pub(super) svgs: Rc<RefCell<HashMap<u32, NativeSvgState>>>,
    pub(super) lists: Rc<RefCell<HashMap<u32, NativeListState>>>,
    pub(super) terminals: Rc<RefCell<HashMap<u32, NativeTerminalState>>>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct NativeMenuAction(pub(super) u32);

impl View for NativeView {
    fn render(&mut self, cx: &mut ViewContext<'_, Self>) -> impl IntoElement {
        let window = self.handles.as_ref().map_or(self.window, |handles| {
            handles
                .borrow()
                .get(&cx.window_handle())
                .copied()
                .expect("a native binding view must retain its core window handle")
        });
        let tree = self.tree.borrow();
        let mut markdown = self.markdown.borrow_mut();
        markdown.retain(|id, _| {
            tree.nodes
                .get(id)
                .is_some_and(|node| node.tag == NodeTag::Markdown)
        });
        let mut svgs = self.svgs.borrow_mut();
        svgs.retain(|id, _| {
            tree.nodes
                .get(id)
                .is_some_and(|node| node.tag == NodeTag::Svg)
        });
        let mut lists = self.lists.borrow_mut();
        lists.retain(|id, _| {
            tree.nodes
                .get(id)
                .is_some_and(|node| node.tag == NodeTag::VirtualList)
        });
        let mut terminals = self.terminals.borrow_mut();
        terminals.retain(|id, _| {
            tree.nodes
                .get(id)
                .is_some_and(|node| node.tag == NodeTag::Terminal)
        });
        let mut root = div()
            .id(ElementId::new(ROOT_ELEMENT_ID))
            .size_full()
            .min_w(0.0)
            .min_h(0.0);
        if let Some(node) = tree.nodes.get(&ROOT_NODE) {
            let mut states = NativeElementStates {
                markdown: &mut markdown,
                svgs: &mut svgs,
                lists: &mut lists,
                terminals: &mut terminals,
            };
            root = root.children(node.children.iter().filter_map(|id| {
                build_element(*id, window, &tree, &self.events, &mut states, cx, 0)
            }));
        }
        let events = Rc::clone(&self.events);
        let menu_action = cx.action_listener(
            ElementId::new(ROOT_ELEMENT_ID),
            move |_view, action: &NativeMenuAction, _cx| {
                enqueue_event(
                    &events,
                    QueuedEvent {
                        kind: "menu-action",
                        window,
                        target: action.0,
                        value: None,
                    },
                );
            },
        );
        root.on_action(menu_action)
    }
}

pub(super) struct NativeElementStates<'a> {
    pub(super) markdown: &'a mut HashMap<u32, Markdown>,
    pub(super) svgs: &'a mut HashMap<u32, NativeSvgState>,
    pub(super) lists: &'a mut HashMap<u32, NativeListState>,
    pub(super) terminals: &'a mut HashMap<u32, NativeTerminalState>,
}

pub(super) fn build_element(
    id: u32,
    window: u32,
    tree: &NativeTree,
    events: &EventQueue,
    states: &mut NativeElementStates<'_>,
    cx: &mut ViewContext<'_, NativeView>,
    depth: usize,
) -> Option<Element> {
    if depth >= MAX_TREE_DEPTH {
        return None;
    }
    let node = tree.nodes.get(&id)?;
    let element_id = ElementId::new(id as u64);
    let mut element = match node.tag {
        NodeTag::Root => return None,
        NodeTag::View => div(),
        NodeTag::Button => button().cursor_default(),
        NodeTag::Text => text(node.text.clone()),
        NodeTag::Sentinel => div().hidden(),
        NodeTag::Input => {
            let multiline = node.boolean(property::MULTILINE).unwrap_or(false);
            let mut input = if multiline {
                text_area(node.string(property::VALUE).unwrap_or_default())
            } else {
                text_input(node.string(property::VALUE).unwrap_or_default())
            }
            .bg(Color::TRANSPARENT)
            .border(0.0, Color::TRANSPARENT)
            .rounded(0.0);
            if let Some(placeholder) = node.string(property::PLACEHOLDER) {
                input = input.placeholder(placeholder);
            }
            if !multiline && node.boolean(property::PASSWORD).unwrap_or(false) {
                input = input.password(true);
            }
            if node.boolean(property::INPUT_LISTENER).unwrap_or(false) {
                let events = Rc::clone(events);
                // The retained input state already schedules its paint. Rebuilding here would read
                // the previous JavaScript-controlled value before Bun drains this queued event,
                // resetting every keystroke before Solid can commit the matching mutation batch.
                let listener = cx.input_listener(element_id, move |_view, value, _cx| {
                    enqueue_event(
                        &events,
                        QueuedEvent {
                            kind: "input",
                            window,
                            target: id,
                            value: Some(Arc::from(value)),
                        },
                    );
                });
                input = input.on_input(listener);
            }
            if !multiline && node.boolean(property::SUBMIT_LISTENER).unwrap_or(false) {
                let events = Rc::clone(events);
                // Submit has the same controlled-state boundary as input: JavaScript must consume
                // the queued value before a render can safely read the controlled property again.
                let listener = cx.submit_listener(element_id, move |_view, value, _cx| {
                    enqueue_event(
                        &events,
                        QueuedEvent {
                            kind: "submit",
                            window,
                            target: id,
                            value: Some(Arc::from(value)),
                        },
                    );
                });
                input = input.on_submit(listener);
            }
            input
        }
        NodeTag::Markdown => {
            let mut markdown_style = MarkdownStyle::default();
            markdown_style.text_color = node.color(property::COLOR);
            markdown_style.font_size = node
                .number(property::FONT_SIZE)
                .unwrap_or(markdown_style.font_size);
            markdown_style.line_height = node
                .number(property::LINE_HEIGHT)
                .unwrap_or(markdown_style.line_height);
            markdown_style.code_background = node.color(property::MARKDOWN_CODE_BACKGROUND);
            markdown_style.border_color = node.color(property::MARKDOWN_BORDER_COLOR);
            markdown_style.muted_color = node.color(property::MARKDOWN_MUTED_COLOR);
            markdown_style.link_color = node.color(property::MARKDOWN_LINK_COLOR);
            markdown_style.code_text_color = node.color(property::MARKDOWN_CODE_TEXT_COLOR);
            markdown_style.block_gap = node
                .number(property::MARKDOWN_BLOCK_GAP)
                .unwrap_or(markdown_style.block_gap);
            markdown_style.code_font_size = node
                .number(property::MARKDOWN_CODE_FONT_SIZE)
                .unwrap_or(markdown_style.code_font_size);
            let state = states.markdown.entry(id).or_default();
            state.set_streaming(node.boolean(property::STREAMING).unwrap_or(false));
            state.set_style(markdown_style);
            state.set_text(node.string(property::VALUE).unwrap_or_default());
            state.element(element_id)
        }
        NodeTag::Svg => {
            let source = node.string(property::VALUE).unwrap_or_default();
            let state = states
                .svgs
                .entry(id)
                .or_insert_with(|| NativeSvgState::new(Arc::from(source)));
            state.sync(source);
            state.element()
        }
        NodeTag::VirtualList => div(),
        NodeTag::Terminal => {
            let state = states
                .terminals
                .entry(id)
                .or_insert_with(|| NativeTerminalState::new(node, cx));
            state.sync(node, cx);
            let terminal = state.terminal.clone();
            if let Some(terminal) = terminal {
                let snapshot = terminal.snapshot();
                if node
                    .boolean(property::TERMINAL_STATUS_LISTENER)
                    .unwrap_or(false)
                {
                    let value = terminal_event_json(&snapshot);
                    if state.last_event.as_deref() != Some(value.as_str()) {
                        state.last_event = Some(Arc::from(value.as_str()));
                        enqueue_event(
                            events,
                            QueuedEvent {
                                kind: "terminal",
                                window,
                                target: id,
                                value: Some(value.into()),
                            },
                        );
                    }
                } else {
                    state.last_event = None;
                }
                terminal.element(
                    element_id,
                    TerminalStyle {
                        font_family: node
                            .string(property::FONT_FAMILY)
                            .and_then(native_font_family)
                            .unwrap_or(quickgui::FontFamily::Monospace),
                        font_size: node.number(property::FONT_SIZE).unwrap_or(13.0),
                        line_height: node.number(property::LINE_HEIGHT).unwrap_or(18.0),
                        font_thicken: node
                            .boolean(property::TERMINAL_FONT_THICKEN)
                            .unwrap_or(false),
                        padding_top: terminal_padding(node, property::PADDING_TOP),
                        padding_right: terminal_padding(node, property::PADDING_RIGHT),
                        padding_bottom: terminal_padding(node, property::PADDING_BOTTOM),
                        padding_left: terminal_padding(node, property::PADDING_LEFT),
                        padding_color: match node.string(property::TERMINAL_PADDING_COLOR) {
                            Some("extend") => TerminalPaddingColor::Extend,
                            _ => TerminalPaddingColor::Background,
                        },
                        foreground: node.color(property::COLOR),
                        background: node.color(property::BACKGROUND_COLOR),
                        theme: native_terminal_theme(node),
                        ..TerminalStyle::default()
                    },
                    cx,
                )
            } else {
                div()
                    .size_full()
                    .bg(Color::rgb8(20, 20, 20))
                    .text_color(Color::rgb8(248, 113, 113))
                    .font_family(quickgui::FontFamily::Monospace)
                    .text_sm()
                    .p_4()
                    .child(text(format!(
                        "QuickGUI terminal error\n\n{}",
                        state.error().unwrap_or("unknown terminal error")
                    )))
            }
        }
    }
    .id(element_id);

    element = apply_properties(element, node);

    let anchor_id = node
        .string(property::ANCHOR_TARGET)
        .and_then(|value| value.parse::<u32>().ok())
        .filter(|anchor_id| *anchor_id != id && tree.nodes.contains_key(anchor_id));
    if let Some(anchor_id) = anchor_id {
        let dismiss_on_escape = node.boolean(property::DISMISS_ON_ESCAPE).unwrap_or(true);
        let dismiss_on_pointer_outside = node
            .boolean(property::DISMISS_ON_POINTER_OUTSIDE)
            .unwrap_or(true);
        let mut popover = Popover::new(ElementId::new(anchor_id as u64), element_id, true)
            .placement(
                node.string(property::ANCHOR_PLACEMENT)
                    .and_then(parse_anchor_placement)
                    .unwrap_or_default(),
            )
            .dismiss_on_escape(dismiss_on_escape)
            .dismiss_on_pointer_outside(dismiss_on_pointer_outside);
        if let Some(gap) = node.number(property::ANCHOR_GAP) {
            popover = popover.anchor_gap(gap);
        }
        if let Some(margin) = node.number(property::VIEWPORT_MARGIN) {
            popover = popover.viewport_margin(margin);
        }
        element = popover.surface_part(element);

        element = attach_dismiss_listener(
            element,
            node.boolean(property::DISMISS_LISTENER).unwrap_or(false)
                && (dismiss_on_escape || dismiss_on_pointer_outside),
            element_id,
            id,
            window,
            events,
            cx,
        );
    } else {
        let dismiss_on_escape = node.boolean(property::DISMISS_ON_ESCAPE).unwrap_or(false);
        let dismiss_on_pointer_outside = node
            .boolean(property::DISMISS_ON_POINTER_OUTSIDE)
            .unwrap_or(false);
        if dismiss_on_escape {
            element = element.dismiss_on_escape();
        }
        if dismiss_on_pointer_outside {
            element = element.dismiss_on_pointer_outside();
        }
        element = attach_dismiss_listener(
            element,
            node.boolean(property::DISMISS_LISTENER).unwrap_or(false)
                && (dismiss_on_escape || dismiss_on_pointer_outside),
            element_id,
            id,
            window,
            events,
            cx,
        );
    }

    if node.boolean(property::CLICK_LISTENER).unwrap_or(false) {
        let events = Rc::clone(events);
        let listener = cx.listener(element_id, move |_view, cx| {
            enqueue_event(
                &events,
                QueuedEvent {
                    kind: "click",
                    window,
                    target: id,
                    value: None,
                },
            );
            cx.invalidate();
        });
        element = element.on_click(listener);
    }
    if node.boolean(property::HOVER_LISTENER).unwrap_or(false) {
        let events = Rc::clone(events);
        let listener = cx.hover_listener(element_id, move |_view, hovered, cx| {
            enqueue_event(
                &events,
                QueuedEvent {
                    kind: if *hovered { "mouseenter" } else { "mouseleave" },
                    window,
                    target: id,
                    value: None,
                },
            );
            cx.invalidate();
        });
        element = element.on_hover(listener);
    }
    if node.boolean(property::POINTER_LISTENER).unwrap_or(false) {
        let events = Rc::clone(events);
        let listener = cx.pointer_listener(element_id, move |_view, event, cx| {
            enqueue_event(
                &events,
                QueuedEvent {
                    kind: "pointer",
                    window,
                    target: id,
                    value: Some(pointer_event_json(event).into()),
                },
            );
            cx.prevent_default();
            cx.stop_propagation();
            cx.invalidate();
        });
        element = element.on_pointer(listener);
    }

    match node.tag {
        NodeTag::VirtualList => {
            let state = states
                .lists
                .entry(id)
                .or_insert_with(|| NativeListState::new(node));
            state.sync(node);
            let list = state.list.clone();
            let children = state.children.clone();
            let visible = list.visible_rows().range;
            let gap = node
                .number(property::ROW_GAP)
                .or_else(|| node.number(property::GAP))
                .unwrap_or(0.0)
                .max(0.0);
            let alignment = node.string(property::ALIGN_ITEMS);
            let item_count = children.len();
            let rows = list.render_rows(visible, |index| {
                let child =
                    build_element(children[index], window, tree, events, states, cx, depth + 1)
                        .unwrap_or_else(|| div().hidden());
                let mut row = div().w_full().flex_none().flex_row().child(child);
                row = match alignment {
                    Some("center") => row.justify_center(),
                    Some("flex-end" | "end") => row.justify_end(),
                    _ => row.justify_start(),
                };
                if gap > 0.0 && index + 1 < item_count {
                    row = row.padding(0.0, 0.0, gap, 0.0);
                }
                row
            });
            element = element.child(rows).variable_virtual_scroll(&list);
        }
        NodeTag::Text
        | NodeTag::Sentinel
        | NodeTag::Input
        | NodeTag::Markdown
        | NodeTag::Svg
        | NodeTag::Terminal => {}
        NodeTag::Root | NodeTag::View | NodeTag::Button => {
            element = element.children(node.children.iter().filter_map(|child| {
                build_element(*child, window, tree, events, states, cx, depth + 1)
            }));
        }
    }
    Some(element)
}

pub(super) fn attach_dismiss_listener(
    mut element: Element,
    enabled: bool,
    element_id: ElementId,
    node_id: u32,
    window: u32,
    events: &EventQueue,
    cx: &mut ViewContext<'_, NativeView>,
) -> Element {
    if !enabled {
        return element;
    }
    let events = Rc::clone(events);
    let listener = cx.dismiss_listener(element_id, move |_view, cx| {
        enqueue_event(
            &events,
            QueuedEvent {
                kind: "dismiss",
                window,
                target: node_id,
                value: None,
            },
        );
        cx.invalidate();
    });
    element = element.on_dismiss(listener);
    element
}

pub(super) fn enqueue_event(events: &EventQueue, event: QueuedEvent) {
    let mut events = events.borrow_mut();
    if events.len() < MAX_QUEUED_EVENTS {
        events.push_back(event);
    }
}

pub(super) fn terminal_event_json(snapshot: &quickgui::TerminalSnapshot) -> String {
    let mut event = serde_json::json!({
        "status": snapshot.status.kind(),
        "title": snapshot.title.as_ref(),
        "workingDirectory": snapshot.working_directory.as_ref(),
    });
    let object = event
        .as_object_mut()
        .expect("terminal event JSON starts as an object");
    match &snapshot.status {
        TerminalStatus::Starting => {}
        TerminalStatus::Running { process_id } => {
            object.insert("processId".to_owned(), serde_json::json!(process_id));
        }
        TerminalStatus::Exited { exit_code, signal } => {
            object.insert("exitCode".to_owned(), serde_json::json!(exit_code));
            object.insert("signal".to_owned(), serde_json::json!(signal.as_deref()));
        }
        TerminalStatus::Failed { message } => {
            object.insert("message".to_owned(), serde_json::json!(message.as_ref()));
        }
    }
    if let Some(agent) = &snapshot.agent {
        object.insert("agent".to_owned(), serde_json::json!(agent.kind.as_ref()));
        object.insert(
            "agentStatus".to_owned(),
            serde_json::json!(agent.status.kind()),
        );
        object.insert(
            "agentProcessId".to_owned(),
            serde_json::json!(agent.process_id),
        );
    }
    event.to_string()
}

pub(super) fn pointer_event_json(event: &quickgui::PointerEvent) -> String {
    let phase = match event.phase {
        PointerPhase::Down => "down",
        PointerPhase::Move => "move",
        PointerPhase::Up => "up",
        PointerPhase::Cancel => "cancel",
    };
    let button = match event.button {
        quickgui::MouseButton::Left => "left",
        quickgui::MouseButton::Right => "right",
        quickgui::MouseButton::Middle => "middle",
        quickgui::MouseButton::Back => "back",
        quickgui::MouseButton::Forward => "forward",
        quickgui::MouseButton::Other(_) => "other",
    };
    serde_json::json!({
        "phase": phase,
        "position": { "x": event.position.x, "y": event.position.y },
        "origin": { "x": event.origin.x, "y": event.origin.y },
        "localPosition": { "x": event.local_position.x, "y": event.local_position.y },
        "localOrigin": { "x": event.local_origin.x, "y": event.local_origin.y },
        "delta": { "x": event.delta.x, "y": event.delta.y },
        "button": button,
    })
    .to_string()
}

pub(super) fn apply_properties(mut element: Element, node: &NativeNode) -> Element {
    if let Some(display) = node.string(property::DISPLAY) {
        element = match display {
            "none" => element.hidden(),
            "flex" => match node.string(property::FLEX_DIRECTION) {
                Some("row") => element.flex_row(),
                Some("row-reverse") => element.flex_row_reverse(),
                Some("column") => element.flex_col(),
                Some("column-reverse") => element.flex_col_reverse(),
                _ => element.flex(),
            },
            "grid" => element.grid(),
            _ => element.block(),
        };
    }
    if let Some(wrap) = node.string(property::FLEX_WRAP) {
        element = match wrap {
            "wrap" => element.flex_wrap(),
            "wrap-reverse" => element.flex_wrap_reverse(),
            _ => element.flex_nowrap(),
        };
    }
    if let Some(value) = node.number(property::FLEX_GROW) {
        element = element.flex_grow(value);
    }
    if let Some(value) = node.number(property::FLEX_SHRINK) {
        element = element.flex_shrink(value);
    }
    if let Some(value) = node.number(property::FLEX_BASIS) {
        element = element.flex_basis(value);
    } else if node.string(property::FLEX_BASIS) == Some("auto") {
        element = element.flex_basis_auto();
    }
    if let Some(value) = node.string(property::ALIGN_ITEMS) {
        element = match value {
            "center" => element.items_center(),
            "flex-end" | "end" => element.items_end(),
            "baseline" => element.items_baseline(),
            "stretch" => element.items_stretch(),
            _ => element.items_start(),
        };
    }
    if let Some(value) = node.string(property::ALIGN_SELF) {
        element = match value {
            "center" => element.self_center(),
            "flex-end" => element.self_flex_end(),
            "end" => element.self_end(),
            "baseline" => element.self_baseline(),
            "stretch" => element.self_stretch(),
            "start" => element.self_start(),
            _ => element.self_flex_start(),
        };
    }
    if let Some(value) = node.string(property::JUSTIFY_CONTENT) {
        element = match value {
            "center" => element.justify_center(),
            "flex-end" | "end" => element.justify_end(),
            "space-between" => element.justify_between(),
            "space-around" => element.justify_around(),
            "space-evenly" => element.justify_evenly(),
            _ => element.justify_start(),
        };
    }
    if let Some(value) = node.string(property::ALIGN_CONTENT) {
        element = match value {
            "center" => element.content_center(),
            "flex-end" | "end" => element.content_end(),
            "space-between" => element.content_between(),
            "space-around" => element.content_around(),
            "space-evenly" => element.content_evenly(),
            "stretch" => element.content_stretch(),
            "normal" => element.content_normal(),
            _ => element.content_start(),
        };
    }
    if let Some(value) = node.number(property::GAP) {
        element = element.gap(value);
    }
    if let Some(value) = node.number(property::COLUMN_GAP) {
        element = element.gap_x(value);
    }
    if let Some(value) = node.number(property::ROW_GAP) {
        element = element.gap_y(value);
    }

    element = apply_dimension(element, node, property::WIDTH, DimensionKind::Width);
    element = apply_dimension(element, node, property::HEIGHT, DimensionKind::Height);
    if let Some(value) = node.number(property::MIN_WIDTH) {
        element = element.min_w(value);
    }
    if let Some(value) = node.number(property::MIN_HEIGHT) {
        element = element.min_h(value);
    }
    if let Some(value) = node.number(property::MAX_WIDTH) {
        element = element.max_w(value);
    }
    if let Some(value) = node.number(property::MAX_HEIGHT) {
        element = element.max_h(value);
    }

    let padding = node.number(property::PADDING).unwrap_or(0.0);
    let padding_top = node.number(property::PADDING_TOP).unwrap_or(padding);
    let padding_right = node.number(property::PADDING_RIGHT).unwrap_or(padding);
    let padding_bottom = node.number(property::PADDING_BOTTOM).unwrap_or(padding);
    let padding_left = node.number(property::PADDING_LEFT).unwrap_or(padding);
    if node.tag != NodeTag::Terminal
        && [padding_top, padding_right, padding_bottom, padding_left]
            .iter()
            .any(|value| *value != 0.0)
    {
        element = element.padding(padding_top, padding_right, padding_bottom, padding_left);
    }
    let margin = node.number(property::MARGIN).unwrap_or(0.0);
    let margin_top = node.number(property::MARGIN_TOP).unwrap_or(margin);
    let margin_right = node.number(property::MARGIN_RIGHT).unwrap_or(margin);
    let margin_bottom = node.number(property::MARGIN_BOTTOM).unwrap_or(margin);
    let margin_left = node.number(property::MARGIN_LEFT).unwrap_or(margin);
    if [margin_top, margin_right, margin_bottom, margin_left]
        .iter()
        .any(|value| *value != 0.0)
    {
        element = element.margin(margin_top, margin_right, margin_bottom, margin_left);
    }

    if let Some(color) = node.color(property::BACKGROUND_COLOR) {
        element = element.bg(color);
    }
    if let Some(color) = node.color(property::COLOR) {
        element = element.text_color(color);
    }
    let hover_background = node.color(property::HOVER_BACKGROUND_COLOR);
    let hover_color = node.color(property::HOVER_COLOR);
    if hover_background.is_some() || hover_color.is_some() {
        element = element.hover(move |mut style| {
            if let Some(color) = hover_background {
                style = style.bg(color);
            }
            if let Some(color) = hover_color {
                style = style.text_color(color);
            }
            style
        });
    }
    let active_background = node.color(property::ACTIVE_BACKGROUND_COLOR);
    let active_color = node.color(property::ACTIVE_COLOR);
    if active_background.is_some() || active_color.is_some() {
        element = element.active(move |mut style| {
            if let Some(color) = active_background {
                style = style.bg(color);
            }
            if let Some(color) = active_color {
                style = style.text_color(color);
            }
            style
        });
    }
    if let Some(milliseconds) = node.number(property::TRANSITION) {
        element = element.transition(Transition::colors(Duration::from_secs_f32(
            (milliseconds / 1_000.0).clamp(0.0, 10.0),
        )));
    }
    if let Some(value) = node.number(property::OPACITY) {
        element = element.opacity(value);
    }
    let border_width = node.number(property::BORDER_WIDTH).unwrap_or(0.0);
    if border_width > 0.0 {
        element = element.border(
            border_width,
            node.color(property::BORDER_COLOR)
                .unwrap_or(Color::TRANSPARENT),
        );
    }
    if let Some(value) = node.number(property::BORDER_RADIUS) {
        element = element.rounded(value);
    }
    if let Some(value) = node.number(property::FONT_SIZE) {
        element = element.text_size(value);
    }
    if let Some(family) = node
        .string(property::FONT_FAMILY)
        .and_then(native_font_family)
    {
        element = element.font_family(family);
    }
    if let Some(value) = node.number(property::LINE_HEIGHT) {
        element = element.line_height(value);
    }
    if let Some(weight) = font_weight(node.property(property::FONT_WEIGHT)) {
        element = element.font_weight(weight);
    }
    if let Some(value) = node.string(property::TEXT_ALIGN) {
        element = element.text_align(match value {
            "center" => TextAlign::Center,
            "right" | "end" => TextAlign::Right,
            "justify" => TextAlign::Justify,
            _ => TextAlign::Left,
        });
    }
    if let Some(value) = node.string(property::WHITE_SPACE) {
        element = match value {
            "nowrap" => element.whitespace_nowrap(),
            _ => element.whitespace_normal(),
        };
    }
    if node.string(property::TEXT_OVERFLOW) == Some("ellipsis") {
        element = element.text_ellipsis();
    }
    if let Some(value) = node.number(property::LINE_CLAMP) {
        element = element.line_clamp(value.max(1.0) as usize);
    }
    if [
        node.string(property::OVERFLOW),
        node.string(property::OVERFLOW_X),
        node.string(property::OVERFLOW_Y),
    ]
    .into_iter()
    .flatten()
    .any(|value| value == "hidden")
    {
        element = element.overflow_hidden();
    }
    if node
        .string(property::OVERFLOW_Y)
        .is_some_and(|value| matches!(value, "auto" | "scroll"))
        || node
            .string(property::OVERFLOW)
            .is_some_and(|value| matches!(value, "auto" | "scroll"))
    {
        element = element.overflow_y_scroll();
    }
    if let Some(value) = node.number(property::SCROLL_TO_END_REVISION) {
        element = element.scroll_to_end(value.max(0.0) as u64);
    }
    if let Some(value) = node.string(property::CURSOR) {
        element = element.cursor(cursor(value));
    }
    if let Some(value) = node.string(property::APP_REGION) {
        element = element.app_region(if value == "drag" {
            AppRegion::Drag
        } else {
            AppRegion::NoDrag
        });
    }
    if let Some(value) = node.boolean(property::DISABLED) {
        element = element.disabled(value);
    }
    if let Some(value) = node.string(property::ACCESSIBILITY_LABEL) {
        element = element.accessibility_label(value.to_owned());
    }
    if let Some(value) = node.string(property::ROLE).and_then(accessibility_role) {
        element = element.accessibility_role(value);
    }
    if let Some(value) = node.number(property::TAB_INDEX) {
        element = element.tab_index(value.clamp(i16::MIN as f32, i16::MAX as f32) as i16);
    }
    if let Some(value) = node.boolean(property::FOCUS_ON_POINTER) {
        element = element.focus_on_pointer(value);
    }
    if node.boolean(property::OVERLAY) == Some(true) {
        element = element.overlay();
    }
    if node.boolean(property::FOCUS_TRAP) == Some(true) {
        element = element.focus_trap();
    }
    if node.boolean(property::RESTORE_PREVIOUS_FOCUS) == Some(true) {
        element = element.restore_previous_focus();
    }
    if node.boolean(property::AUTO_FOCUS) == Some(true) {
        element = element.auto_focus();
    }
    if let Some(value) = node.boolean(property::ACCESSIBILITY_MODAL) {
        element = element.accessibility_modal(value);
    }
    let hit_slop = node.number(property::HIT_SLOP).unwrap_or(0.0);
    let hit_slop = quickgui::Insets {
        top: node.number(property::HIT_SLOP_TOP).unwrap_or(hit_slop),
        right: node.number(property::HIT_SLOP_RIGHT).unwrap_or(hit_slop),
        bottom: node.number(property::HIT_SLOP_BOTTOM).unwrap_or(hit_slop),
        left: node.number(property::HIT_SLOP_LEFT).unwrap_or(hit_slop),
    };
    if hit_slop != quickgui::Insets::default() {
        element = element.hit_slop(hit_slop);
    }
    if let Some(value) = node.string(property::POSITION) {
        element = if value == "absolute" {
            element.absolute()
        } else {
            element.relative()
        };
    }
    if let Some(value) = node.number(property::TOP) {
        element = element.top(value);
    }
    if let Some(value) = node.number(property::RIGHT) {
        element = element.right(value);
    }
    if let Some(value) = node.number(property::BOTTOM) {
        element = element.bottom(value);
    }
    if let Some(value) = node.number(property::LEFT) {
        element = element.left(value);
    }
    if let Some(value) = node.string(property::USER_SELECT) {
        element = match value {
            "none" => element.user_select_none(),
            "text" => element.user_select_text(),
            _ => element,
        };
    }
    if let Some(value) = node.string(property::VISIBILITY) {
        element = if value == "hidden" {
            element.invisible()
        } else {
            element.visible()
        };
    }
    if let Some(value) = node.number(property::ASPECT_RATIO) {
        element = element.aspect_ratio(value);
    }
    element
}

pub(super) fn terminal_padding(node: &NativeNode, side: u16) -> f32 {
    node.number(side)
        .or_else(|| node.number(property::PADDING))
        .unwrap_or(0.0)
}

pub(super) enum DimensionKind {
    Width,
    Height,
}

pub(super) fn apply_dimension(
    element: Element,
    node: &NativeNode,
    property: u16,
    kind: DimensionKind,
) -> Element {
    match node.property(property) {
        Some(PropertyValue::Number(value)) => match kind {
            DimensionKind::Width => element.w(*value),
            DimensionKind::Height => element.h(*value),
        },
        Some(PropertyValue::String(value)) if value.as_ref() == "100%" => match kind {
            DimensionKind::Width => element.w_full(),
            DimensionKind::Height => element.h_full(),
        },
        _ => element,
    }
}

pub(super) fn unpack_color(value: u32) -> Color {
    Color::rgba8(
        value as u8,
        (value >> 8) as u8,
        (value >> 16) as u8,
        (value >> 24) as u8,
    )
}

pub(super) fn native_font_family(value: &str) -> Option<quickgui::FontFamily> {
    match value.trim() {
        "sans-serif" | "system-ui" => Some(quickgui::FontFamily::SansSerif),
        "serif" => Some(quickgui::FontFamily::Serif),
        "monospace" => Some(quickgui::FontFamily::Monospace),
        name if !name.is_empty() && name.len() <= quickgui::MAX_FONT_FAMILY_BYTES => {
            Some(quickgui::FontFamily::named(Arc::<str>::from(name)))
        }
        _ => None,
    }
}

pub(super) fn native_terminal_theme(node: &NativeNode) -> Option<TerminalTheme> {
    let encoded = node.string(property::TERMINAL_PALETTE)?;
    let packed = serde_json::from_str::<Vec<u32>>(encoded).ok()?;
    let packed: [u32; TERMINAL_ANSI_COLOR_COUNT] = packed.try_into().ok()?;
    let ansi = packed.map(unpack_color);
    let foreground = node.color(property::COLOR)?;
    let background = node.color(property::BACKGROUND_COLOR)?;
    Some(
        TerminalTheme::new(foreground, background, ansi).cursor(
            node.color(property::TERMINAL_CURSOR_COLOR)
                .unwrap_or(foreground),
        ),
    )
}

pub(super) fn font_weight(value: Option<&PropertyValue>) -> Option<FontWeight> {
    let weight = match value {
        Some(PropertyValue::Number(value)) => *value,
        Some(PropertyValue::String(value)) => match value.as_ref() {
            "thin" => 100.0,
            "extralight" | "extra-light" => 200.0,
            "light" => 300.0,
            "medium" => 500.0,
            "semibold" | "semi-bold" => 600.0,
            "bold" => 700.0,
            "extrabold" | "extra-bold" => 800.0,
            "black" => 900.0,
            _ => 400.0,
        },
        _ => return None,
    };
    Some(if weight < 150.0 {
        FontWeight::THIN
    } else if weight < 250.0 {
        FontWeight::EXTRA_LIGHT
    } else if weight < 350.0 {
        FontWeight::LIGHT
    } else if weight < 450.0 {
        FontWeight::NORMAL
    } else if weight < 550.0 {
        FontWeight::MEDIUM
    } else if weight < 650.0 {
        FontWeight::SEMIBOLD
    } else if weight < 750.0 {
        FontWeight::BOLD
    } else if weight < 850.0 {
        FontWeight::EXTRA_BOLD
    } else {
        FontWeight::BLACK
    })
}

pub(super) fn cursor(value: &str) -> CursorStyle {
    match value {
        "text" => CursorStyle::IBeam,
        "pointer" => CursorStyle::PointingHand,
        "crosshair" => CursorStyle::Crosshair,
        "grab" => CursorStyle::OpenHand,
        "grabbing" => CursorStyle::ClosedHand,
        "not-allowed" | "no-drop" => CursorStyle::OperationNotAllowed,
        "copy" => CursorStyle::DragCopy,
        "alias" => CursorStyle::DragLink,
        "context-menu" => CursorStyle::ContextualMenu,
        "ew-resize" => CursorStyle::ResizeLeftRight,
        "ns-resize" => CursorStyle::ResizeUpDown,
        "nwse-resize" => CursorStyle::ResizeUpLeftDownRight,
        "nesw-resize" => CursorStyle::ResizeUpRightDownLeft,
        "col-resize" => CursorStyle::ResizeColumn,
        "row-resize" => CursorStyle::ResizeRow,
        "n-resize" => CursorStyle::ResizeUp,
        "e-resize" => CursorStyle::ResizeRight,
        "s-resize" => CursorStyle::ResizeDown,
        "w-resize" => CursorStyle::ResizeLeft,
        "vertical-text" => CursorStyle::IBeamCursorForVerticalLayout,
        _ => CursorStyle::Arrow,
    }
}

pub(super) fn accessibility_role(value: &str) -> Option<AccessibilityRole> {
    Some(match value {
        "button" => AccessibilityRole::Button,
        "link" => AccessibilityRole::Link,
        "img" | "image" => AccessibilityRole::Image,
        "list" => AccessibilityRole::List,
        "listitem" => AccessibilityRole::ListItem,
        "heading" => AccessibilityRole::Heading,
        "checkbox" => AccessibilityRole::CheckBox,
        "radio" => AccessibilityRole::RadioButton,
        "radiogroup" => AccessibilityRole::RadioGroup,
        "switch" => AccessibilityRole::Switch,
        "dialog" => AccessibilityRole::Dialog,
        "alertdialog" => AccessibilityRole::AlertDialog,
        "menu" => AccessibilityRole::Menu,
        "menuitem" => AccessibilityRole::MenuItem,
        "separator" => AccessibilityRole::Separator,
        "group" => AccessibilityRole::Group,
        "region" => AccessibilityRole::Region,
        "listbox" => AccessibilityRole::ListBox,
        "option" => AccessibilityRole::ListBoxOption,
        "combobox" => AccessibilityRole::ComboBox,
        "table" => AccessibilityRole::Table,
        "tree" => AccessibilityRole::Tree,
        "grid" => AccessibilityRole::Grid,
        "row" => AccessibilityRole::Row,
        "columnheader" => AccessibilityRole::ColumnHeader,
        "rowheader" => AccessibilityRole::RowHeader,
        "gridcell" => AccessibilityRole::GridCell,
        "treeitem" => AccessibilityRole::TreeItem,
        "tab" => AccessibilityRole::Tab,
        "tablist" => AccessibilityRole::TabList,
        "tabpanel" => AccessibilityRole::TabPanel,
        "tooltip" => AccessibilityRole::Tooltip,
        "form" => AccessibilityRole::Form,
        "label" => AccessibilityRole::Label,
        _ => return None,
    })
}
