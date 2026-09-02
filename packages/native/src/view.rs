use super::*;
use serde::Deserialize;

pub(super) struct NativeView {
    pub(super) window: u32,
    pub(super) handles: Option<Rc<RefCell<HashMap<WindowHandle, u32>>>>,
    pub(super) tree: Rc<RefCell<NativeTree>>,
    pub(super) events: EventQueue,
    pub(super) markdown: Rc<RefCell<HashMap<u32, Markdown>>>,
    pub(super) svgs: Rc<RefCell<HashMap<u32, NativeSvgState>>>,
    pub(super) lists: Rc<RefCell<HashMap<u32, NativeListState>>>,
    pub(super) terminals: Rc<RefCell<HashMap<u32, NativeTerminalState>>>,
    #[cfg(target_os = "macos")]
    pub(super) swift_ui_hosts: Rc<RefCell<HashMap<u32, NativeSwiftUiHostState>>>,
    #[cfg(target_os = "macos")]
    pub(super) embedded_views: Rc<RefCell<HashMap<u32, MacEmbeddedView>>>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct NativeMenuAction(pub(super) u32);

impl View for NativeView {
    fn event(&mut self, event: &Event, cx: &mut EventContext) {
        // Input events reach this callback on every frame; resolving the hosted window id costs a
        // map lookup, so the hot path leaves before it.
        if !crate::runtime::is_hosted_window_event(event) {
            return;
        }
        let window = cx.window_handle().map_or(self.window, |handle| {
            self.handles.as_ref().map_or(self.window, |handles| {
                handles
                    .borrow()
                    .get(&handle)
                    .copied()
                    .unwrap_or(self.window)
            })
        });
        // Window lifecycle notifications and the declared-ahead resize/move constraints live in
        // the hosted runtime module; everything else falls through to close interception.
        if crate::runtime::handle_window_lifecycle_event(window, event, cx, &self.events) {
            return;
        }
        if !matches!(event, Event::CloseRequested) {
            return;
        }
        // Interception was declared before the native decision, so the veto is answered here and
        // JavaScript completes the close later with an explicit command.
        if !crate::runtime::intercepts_close(window) {
            return;
        }
        cx.prevent_close();
        enqueue_event(
            &self.events,
            QueuedEvent {
                kind: "close-requested",
                window,
                target: ROOT_NODE,
                value: None,
            },
        );
    }

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
        #[cfg(target_os = "macos")]
        let mut swift_ui_hosts = self.swift_ui_hosts.borrow_mut();
        #[cfg(target_os = "macos")]
        let embedded_views = self.embedded_views.borrow();
        #[cfg(target_os = "macos")]
        swift_ui_hosts.retain(|id, _| {
            tree.nodes
                .get(id)
                .is_some_and(|node| node.tag == NodeTag::SwiftUiHost)
        });
        let mut root = div()
            .id(ElementId::new(ROOT_ELEMENT_ID))
            .size_full()
            .min_w(0.0)
            .min_h(0.0);
        if let Some(node) = tree.nodes.get(&ROOT_NODE) {
            let mut part_ids = HashSet::new();
            let mut states = NativeElementStates {
                markdown: &mut markdown,
                svgs: &mut svgs,
                lists: &mut lists,
                terminals: &mut terminals,
                part_ids: &mut part_ids,
                #[cfg(target_os = "macos")]
                swift_ui_hosts: &mut swift_ui_hosts,
                #[cfg(target_os = "macos")]
                embedded_views: &embedded_views,
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
    /// Derived component-part element identities already mounted in this render pass.
    ///
    /// Nodes without a `part` property keep their unique node-derived identity and never touch
    /// this set, so the untouched element path costs nothing. Two parts that declare the same
    /// scope and value would otherwise register one core listener identity twice, which the core
    /// rejects with a panic; the duplicate mounts without listeners instead.
    pub(super) part_ids: &'a mut HashSet<u64>,
    #[cfg(target_os = "macos")]
    pub(super) swift_ui_hosts: &'a mut HashMap<u32, NativeSwiftUiHostState>,
    #[cfg(target_os = "macos")]
    pub(super) embedded_views: &'a HashMap<u32, MacEmbeddedView>,
}

#[cfg(target_os = "macos")]
pub(super) struct NativeSwiftUiHostState {
    host: std::result::Result<MacSwiftUiHost, String>,
}

#[cfg(target_os = "macos")]
impl NativeSwiftUiHostState {
    fn new(window: u32, events: &EventQueue, invalidator: quickgui::WindowInvalidator) -> Self {
        let action_events = Rc::clone(events);
        let presentation_events = Rc::clone(events);
        let action_invalidator = invalidator.clone();
        let presentation_invalidator = invalidator;
        Self {
            host: MacSwiftUiHost::new_with_events(
                move |target| {
                    let Ok(target) = u32::try_from(target) else {
                        return;
                    };
                    enqueue_event(
                        &action_events,
                        QueuedEvent {
                            kind: "click",
                            window,
                            target,
                            value: None,
                        },
                    );
                    action_invalidator.invalidate();
                },
                move |target, presented| {
                    let Ok(target) = u32::try_from(target) else {
                        return;
                    };
                    enqueue_event(
                        &presentation_events,
                        QueuedEvent {
                            kind: "presentationchange",
                            window,
                            target,
                            value: Some(Arc::from(if presented { "true" } else { "false" })),
                        },
                    );
                    presentation_invalidator.invalidate();
                },
            ),
        }
    }

    fn element(
        &mut self,
        node: &NativeNode,
        tree: &NativeTree,
        embedded_views: &HashMap<u32, MacEmbeddedView>,
    ) -> std::result::Result<Element, String> {
        let host = self.host.as_mut().map_err(|error| error.clone())?;
        let elements = swift_ui_children(&node.children, tree, embedded_views)?;
        host.sync(&elements)?;
        let fitting = host.fitting_size()?;
        let mut element = native_view(host.view());
        if node
            .boolean(property::SWIFT_UI_MATCH_CONTENTS_HORIZONTAL)
            .unwrap_or(false)
        {
            element = element.w(fitting.width);
        }
        if node
            .boolean(property::SWIFT_UI_MATCH_CONTENTS_VERTICAL)
            .unwrap_or(false)
        {
            element = element.h(fitting.height);
        }
        Ok(element)
    }
}

#[cfg(target_os = "macos")]
fn swift_ui_button(
    id: u32,
    node: &NativeNode,
    tree: &NativeTree,
) -> std::result::Result<SwiftUiButton, String> {
    let label = node
        .string(property::VALUE)
        .map(Arc::<str>::from)
        .or_else(|| {
            let text = swift_ui_text_content(node, tree);
            (!text.is_empty()).then(|| Arc::from(text))
        });
    let mut button = SwiftUiButton::new(u64::from(id)).role(match node.string(property::ROLE) {
        Some("cancel") => SwiftUiButtonRole::Cancel,
        Some("destructive") => SwiftUiButtonRole::Destructive,
        _ => SwiftUiButtonRole::Default,
    });
    if let Some(label) = label {
        button = button.label(label);
    }
    if let Some(target) = node.string(property::SWIFT_UI_TARGET) {
        button = button.target(Arc::<str>::from(target));
    }
    if let Some(test_id) = node.string(property::SWIFT_UI_TEST_ID) {
        button = button.test_id(Arc::<str>::from(test_id));
    }
    if let Some(style) = node.string(property::SWIFT_UI_BUTTON_STYLE) {
        button = button.style(match style {
            "bordered" => SwiftUiButtonStyle::Bordered,
            "borderedProminent" => SwiftUiButtonStyle::BorderedProminent,
            "borderless" => SwiftUiButtonStyle::Borderless,
            "plain" => SwiftUiButtonStyle::Plain,
            "glass" => SwiftUiButtonStyle::Glass,
            "glassProminent" => SwiftUiButtonStyle::GlassProminent,
            _ => SwiftUiButtonStyle::Automatic,
        });
    }
    if let Some(size) = node.string(property::SWIFT_UI_CONTROL_SIZE) {
        button = button.control_size(match size {
            "mini" => SwiftUiControlSize::Mini,
            "small" => SwiftUiControlSize::Small,
            "large" => SwiftUiControlSize::Large,
            "extraLarge" => SwiftUiControlSize::ExtraLarge,
            _ => SwiftUiControlSize::Regular,
        });
    }
    if let Some(disabled) = node.boolean(property::DISABLED) {
        button = button.disabled(disabled);
    }
    button = button
        .modifiers(swift_ui_modifiers(node)?)
        .action(node.boolean(property::CLICK_LISTENER).unwrap_or(false));
    if let Some(system_image) = node
        .string(property::SWIFT_UI_SYSTEM_IMAGE)
        .filter(|value| !value.is_empty())
    {
        button = button.system_image(Arc::<str>::from(system_image));
    }
    Ok(button)
}

#[cfg(target_os = "macos")]
fn swift_ui_children(
    children: &[u32],
    tree: &NativeTree,
    embedded_views: &HashMap<u32, MacEmbeddedView>,
) -> std::result::Result<Vec<SwiftUiElement>, String> {
    children
        .iter()
        .filter_map(|id| {
            tree.nodes
                .get(id)
                .and_then(|node| swift_ui_element(*id, node, tree, embedded_views))
        })
        .collect()
}

#[cfg(target_os = "macos")]
fn swift_ui_element(
    id: u32,
    node: &NativeNode,
    tree: &NativeTree,
    embedded_views: &HashMap<u32, MacEmbeddedView>,
) -> Option<std::result::Result<SwiftUiElement, String>> {
    match node.tag {
        NodeTag::SwiftUiButton => Some(swift_ui_button(id, node, tree).map(SwiftUiElement::Button)),
        NodeTag::SwiftUiQuickGuiHost => Some((|| {
            let embedded_id = node
                .number(property::SWIFT_UI_EMBEDDED_WINDOW)
                .filter(|id| *id >= 1.0 && *id <= u32::MAX as f32)
                .map(|id| id as u32);
            let mut host = match embedded_id {
                Some(embedded_id) => {
                    let embedded = embedded_views.get(&embedded_id).cloned().ok_or_else(|| {
                        format!(
                            "SwiftUI QuickGUIHostView {id} references unknown embedded window {embedded_id}"
                        )
                    })?;
                    SwiftUiQuickGuiHost::new(u64::from(id), embedded)
                }
                None => SwiftUiQuickGuiHost::pending(u64::from(id)),
            };
            host.match_horizontal = node
                .boolean(property::SWIFT_UI_MATCH_CONTENTS_HORIZONTAL)
                .unwrap_or(false);
            host.match_vertical = node
                .boolean(property::SWIFT_UI_MATCH_CONTENTS_VERTICAL)
                .unwrap_or(false);
            host.width = node.number(property::WIDTH).filter(|value| *value > 0.0);
            host.height = node.number(property::HEIGHT).filter(|value| *value > 0.0);
            host.test_id = node
                .string(property::SWIFT_UI_TEST_ID)
                .filter(|value| !value.is_empty())
                .map(Arc::<str>::from);
            Ok(SwiftUiElement::QuickGuiHost(host))
        })()),
        NodeTag::SwiftUiPopover => Some((|| {
            let mut trigger = None;
            let mut content = None;
            for child in &node.children {
                let Some(slot) = tree.nodes.get(child) else {
                    continue;
                };
                match slot.tag {
                    NodeTag::SwiftUiPopoverTrigger if trigger.is_none() => {
                        trigger = Some(swift_ui_children(&slot.children, tree, embedded_views)?);
                    }
                    NodeTag::SwiftUiPopoverContent if content.is_none() => {
                        content = Some(swift_ui_children(&slot.children, tree, embedded_views)?);
                    }
                    _ => {}
                }
            }
            let mut popover = SwiftUiPopover::new(u64::from(id));
            popover.is_presented = node
                .boolean(property::SWIFT_UI_IS_PRESENTED)
                .unwrap_or(false);
            popover.attachment_anchor = match node.string(property::SWIFT_UI_ATTACHMENT_ANCHOR) {
                Some("top") => SwiftUiPopoverAttachmentAnchor::Top,
                Some("bottom") => SwiftUiPopoverAttachmentAnchor::Bottom,
                Some("leading") => SwiftUiPopoverAttachmentAnchor::Leading,
                Some("trailing") => SwiftUiPopoverAttachmentAnchor::Trailing,
                _ => SwiftUiPopoverAttachmentAnchor::Center,
            };
            popover.arrow_edge = match node.string(property::SWIFT_UI_ARROW_EDGE) {
                Some("top") => SwiftUiPopoverArrowEdge::Top,
                Some("leading") => SwiftUiPopoverArrowEdge::Leading,
                Some("trailing") => SwiftUiPopoverArrowEdge::Trailing,
                _ => SwiftUiPopoverArrowEdge::Bottom,
            };
            popover.trigger = trigger
                .ok_or_else(|| format!("SwiftUI Popover {id} requires one Popover.Trigger"))?;
            popover.content = content
                .ok_or_else(|| format!("SwiftUI Popover {id} requires one Popover.Content"))?;
            popover.test_id = node
                .string(property::SWIFT_UI_TEST_ID)
                .filter(|value| !value.is_empty())
                .map(Arc::<str>::from);
            Ok(SwiftUiElement::Popover(popover))
        })()),
        NodeTag::SwiftUiPopoverTrigger | NodeTag::SwiftUiPopoverContent => None,
        _ => Some(Err(format!(
            "node {id} is not a SwiftUI component and cannot be mounted directly in Host"
        ))),
    }
}

#[cfg(target_os = "macos")]
#[derive(serde::Deserialize)]
struct NativeSwiftUiModifier {
    #[serde(rename = "$type")]
    kind: String,
    style: Option<String>,
    size: Option<String>,
    shape: Option<String>,
    #[serde(rename = "cornerRadius")]
    corner_radius: Option<f32>,
    color: Option<String>,
    disabled: Option<bool>,
}

#[cfg(target_os = "macos")]
pub(super) fn swift_ui_modifiers(
    node: &NativeNode,
) -> std::result::Result<Vec<SwiftUiModifier>, String> {
    let Some(payload) = node.string(property::SWIFT_UI_MODIFIERS) else {
        return Ok(Vec::new());
    };
    let modifiers = serde_json::from_str::<Vec<NativeSwiftUiModifier>>(payload)
        .map_err(|error| format!("invalid SwiftUI modifiers: {error}"))?;
    modifiers
        .into_iter()
        .map(|modifier| match modifier.kind.as_str() {
            "buttonStyle" => Ok(SwiftUiModifier::ButtonStyle(
                match modifier.style.as_deref() {
                    Some("automatic") => SwiftUiButtonStyle::Automatic,
                    Some("bordered") => SwiftUiButtonStyle::Bordered,
                    Some("borderedProminent") => SwiftUiButtonStyle::BorderedProminent,
                    Some("borderless") => SwiftUiButtonStyle::Borderless,
                    Some("glass") => SwiftUiButtonStyle::Glass,
                    Some("glassProminent") => SwiftUiButtonStyle::GlassProminent,
                    Some("plain") => SwiftUiButtonStyle::Plain,
                    _ => return Err("buttonStyle modifier has an invalid style".to_owned()),
                },
            )),
            "buttonBorderShape" => {
                let shape = match modifier.shape.as_deref() {
                    Some("automatic") => SwiftUiButtonBorderShape::Automatic,
                    Some("capsule") => SwiftUiButtonBorderShape::Capsule,
                    Some("roundedRectangle") => SwiftUiButtonBorderShape::RoundedRectangle,
                    Some("circle") => SwiftUiButtonBorderShape::Circle,
                    _ => return Err("buttonBorderShape modifier has an invalid shape".to_owned()),
                };
                if modifier
                    .corner_radius
                    .is_some_and(|radius| !radius.is_finite() || radius < 0.0)
                {
                    return Err("buttonBorderShape modifier has an invalid cornerRadius".to_owned());
                }
                Ok(SwiftUiModifier::ButtonBorderShape {
                    shape,
                    corner_radius: modifier.corner_radius,
                })
            }
            "controlSize" => Ok(SwiftUiModifier::ControlSize(
                match modifier.size.as_deref() {
                    Some("mini") => SwiftUiControlSize::Mini,
                    Some("small") => SwiftUiControlSize::Small,
                    Some("regular") => SwiftUiControlSize::Regular,
                    Some("large") => SwiftUiControlSize::Large,
                    Some("extraLarge") => SwiftUiControlSize::ExtraLarge,
                    _ => return Err("controlSize modifier has an invalid size".to_owned()),
                },
            )),
            "labelStyle" => Ok(SwiftUiModifier::LabelStyle(
                match modifier.style.as_deref() {
                    Some("automatic") => SwiftUiLabelStyle::Automatic,
                    Some("iconOnly") => SwiftUiLabelStyle::IconOnly,
                    Some("titleAndIcon") => SwiftUiLabelStyle::TitleAndIcon,
                    Some("titleOnly") => SwiftUiLabelStyle::TitleOnly,
                    _ => return Err("labelStyle modifier has an invalid style".to_owned()),
                },
            )),
            "tint" => modifier
                .color
                .filter(|color| !color.is_empty())
                .map(|color| SwiftUiModifier::Tint(Arc::from(color)))
                .ok_or_else(|| "tint modifier has an invalid color".to_owned()),
            "disabled" => modifier
                .disabled
                .map(SwiftUiModifier::Disabled)
                .ok_or_else(|| "disabled modifier has no disabled value".to_owned()),
            kind => Err(format!("unsupported SwiftUI modifier `{kind}`")),
        })
        .collect()
}

#[cfg(target_os = "macos")]
fn swift_ui_text_content(node: &NativeNode, tree: &NativeTree) -> String {
    let mut text = String::new();
    let mut pending = node.children.iter().copied().rev().collect::<Vec<_>>();
    let mut visited = 0;
    while let Some(id) = pending.pop() {
        visited += 1;
        if visited > MAX_TREE_DEPTH {
            break;
        }
        let Some(child) = tree.nodes.get(&id) else {
            continue;
        };
        if child.tag == NodeTag::Text {
            text.push_str(&child.text);
        } else {
            pending.extend(child.children.iter().copied().rev());
        }
    }
    text
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
    // A component part adopts the Rust descriptor's derived identity so the core's own
    // `aria-controls`/`labelled-by` relationships and mount policy resolve without a JavaScript
    // registry. Ordinary nodes keep their unique node identity on the untouched fast path.
    let part_element_id = native_part_element_id(id, node);
    let listeners_enabled = match part_element_id {
        Some(part_element_id) => states.part_ids.insert(part_element_id.as_u64()),
        None => true,
    };
    let element_id = part_element_id.unwrap_or_else(|| ElementId::new(id as u64));
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
            if listeners_enabled && node.boolean(property::INPUT_LISTENER).unwrap_or(false) {
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
            if listeners_enabled
                && !multiline
                && node.boolean(property::SUBMIT_LISTENER).unwrap_or(false)
            {
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
        NodeTag::SwiftUiHost => {
            #[cfg(target_os = "macos")]
            {
                let state = states.swift_ui_hosts.entry(id).or_insert_with(|| {
                    NativeSwiftUiHostState::new(window, events, cx.window_invalidator())
                });
                match state.element(node, tree, states.embedded_views) {
                    Ok(element) => element,
                    Err(error) => div()
                        .size_full()
                        .bg(Color::rgb8(255, 255, 255))
                        .text_color(Color::rgb8(185, 28, 28))
                        .p_4()
                        .child(text(format!("SwiftUI host error: {error}"))),
                }
            }
            #[cfg(not(target_os = "macos"))]
            {
                div().hidden()
            }
        }
        NodeTag::SwiftUiButton
        | NodeTag::SwiftUiQuickGuiHost
        | NodeTag::SwiftUiPopover
        | NodeTag::SwiftUiPopoverTrigger
        | NodeTag::SwiftUiPopoverContent => return None,
    }
    .id(element_id);

    element = apply_properties(element, node);
    element = apply_tooltip(element, node);
    // The core part descriptor decides identity, semantics, and whether an inactive panel is
    // mounted at all. A part that is not mounted contributes no layout, paint, input, or
    // accessibility node, exactly as the Rust component guides describe.
    element = apply_part(element, id, node)?;

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
            listeners_enabled
                && node.boolean(property::DISMISS_LISTENER).unwrap_or(false)
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
            listeners_enabled
                && node.boolean(property::DISMISS_LISTENER).unwrap_or(false)
                && (dismiss_on_escape || dismiss_on_pointer_outside),
            element_id,
            id,
            window,
            events,
            cx,
        );
    }

    if listeners_enabled && node.boolean(property::CLICK_LISTENER).unwrap_or(false) {
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
    if listeners_enabled && node.boolean(property::HOVER_LISTENER).unwrap_or(false) {
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
    if listeners_enabled && node.boolean(property::POINTER_LISTENER).unwrap_or(false) {
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
        NodeTag::SwiftUiHost
        | NodeTag::SwiftUiButton
        | NodeTag::SwiftUiQuickGuiHost
        | NodeTag::SwiftUiPopover
        | NodeTag::SwiftUiPopoverTrigger
        | NodeTag::SwiftUiPopoverContent => {}
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

/// Bounded compound scope key shared by every part of one component instance.
///
/// The renderer allocates the key; the Rust binding hashes it into the same [`ElementId`] the
/// core component would have used, so every derived part identity matches without a registry.
fn native_part_scope(id: u32, node: &NativeNode) -> ElementId {
    match node
        .string(property::SCOPE)
        .filter(|scope| !scope.is_empty() && scope.len() <= MAX_COMPONENT_VALUE_BYTES)
    {
        Some(scope) => ElementId::named(scope),
        None => ElementId::new(id as u64),
    }
}

fn native_part_value(node: &NativeNode, key: u16) -> Option<ElementId> {
    node.string(key)
        .filter(|value| !value.is_empty() && value.len() <= MAX_COMPONENT_VALUE_BYTES)
        .map(ElementId::named)
}

fn native_toggle_state(node: &NativeNode) -> ToggleState {
    if node.boolean(property::INDETERMINATE) == Some(true) {
        ToggleState::Mixed
    } else {
        ToggleState::from(node.boolean(property::CHECKED).unwrap_or(false))
    }
}

fn native_tabs(id: u32, node: &NativeNode) -> Tabs {
    let scope = native_part_scope(id, node);
    let tabs = match native_part_value(node, property::ACTIVE_VALUE) {
        Some(active) => Tabs::new(scope, active),
        None => Tabs::without_selection(scope),
    };
    let tabs = if node.string(property::ORIENTATION) == Some("vertical") {
        tabs.vertical()
    } else {
        tabs
    };
    tabs.activate_on_focus(node.boolean(property::ACTIVATE_ON_FOCUS).unwrap_or(false))
        .loop_focus(node.boolean(property::LOOP_FOCUS).unwrap_or(true))
        .keep_mounted(node.boolean(property::KEEP_MOUNTED).unwrap_or(false))
}

fn native_tab(id: u32, node: &NativeNode) -> Option<Tab> {
    let value = native_part_value(node, property::PART_VALUE)?;
    Some(
        native_tabs(id, node)
            .tab(value)
            .disabled(node.boolean(property::DISABLED).unwrap_or(false)),
    )
}

fn native_collapsible(id: u32, node: &NativeNode) -> Collapsible {
    Collapsible::new(
        native_part_scope(id, node),
        node.boolean(property::OPEN).unwrap_or(false),
    )
    .disabled(node.boolean(property::DISABLED).unwrap_or(false))
    .keep_mounted(node.boolean(property::KEEP_MOUNTED).unwrap_or(false))
}

fn native_accordion_item(id: u32, node: &NativeNode) -> Option<AccordionItem> {
    let value = native_part_value(node, property::PART_VALUE)?;
    let index = node
        .number(property::ITEM_INDEX)
        .unwrap_or(0.0)
        .clamp(0.0, MAX_NODES as f32) as usize;
    let mut accordion = Accordion::new(native_part_scope(id, node))
        .keep_mounted(node.boolean(property::KEEP_MOUNTED).unwrap_or(false));
    if let Some(level) = node.number(property::HEADING_LEVEL) {
        accordion = accordion.heading_level(level.clamp(1.0, 6.0) as usize);
    }
    Some(
        accordion
            .item(value, index, node.boolean(property::OPEN).unwrap_or(false))
            .disabled(node.boolean(property::DISABLED).unwrap_or(false)),
    )
}

fn native_field(id: u32, node: &NativeNode) -> Field {
    let field = Field::new(native_part_scope(id, node))
        .disabled(node.boolean(property::DISABLED).unwrap_or(false))
        .invalid(node.boolean(property::INVALID).unwrap_or(false))
        .required(node.boolean(property::REQUIRED).unwrap_or(false))
        .touched(node.boolean(property::TOUCHED).unwrap_or(false))
        .dirty(node.boolean(property::DIRTY).unwrap_or(false))
        .filled(node.boolean(property::FILLED).unwrap_or(false));
    match node.string(property::VALIDATION_MESSAGE) {
        Some(message) => field.validation_message(message),
        None => field,
    }
}

fn native_dialog(id: u32, node: &NativeNode) -> CoreDialog {
    let kind = match node.string(property::VARIANT) {
        Some("alertdialog") => DialogKind::AlertDialog,
        _ => DialogKind::Dialog,
    };
    let mut dialog = CoreDialog::with_kind(
        native_part_scope(id, node),
        node.boolean(property::OPEN).unwrap_or(false),
        kind,
    );
    if let Some(dismiss) = node.boolean(property::DISMISS_ON_ESCAPE) {
        dialog = dialog.dismiss_on_escape(dismiss);
    }
    if let Some(dismiss) = node.boolean(property::DISMISS_ON_POINTER_OUTSIDE) {
        dialog = dialog.dismiss_on_backdrop(dismiss);
    }
    dialog
}

fn native_fieldset(id: u32, node: &NativeNode) -> Fieldset {
    Fieldset::new(native_part_scope(id, node))
        .disabled(node.boolean(property::DISABLED).unwrap_or(false))
}

/// Resolve the identity a component part must mount with, or `None` for an ordinary node.
pub(super) fn native_part_element_id(id: u32, node: &NativeNode) -> Option<ElementId> {
    let part = node.string(property::PART)?;
    Some(match part {
        "tabs" | "collapsible" | "accordion" | "fieldset" | "field-control" => {
            native_part_scope(id, node)
        }
        "tabs-list" => native_tabs(id, node).list_id(),
        "tab" => native_tab(id, node)?.tab_id(),
        "tab-indicator" => native_tab(id, node)?.indicator_id(),
        "tab-panel" => native_tab(id, node)?.panel_id(),
        "collapsible-trigger" => native_collapsible(id, node).trigger_id(),
        "collapsible-panel" => native_collapsible(id, node).panel_id(),
        "accordion-item" => native_accordion_item(id, node)?.root_id(),
        "accordion-header" => native_accordion_item(id, node)?.header_id(),
        "accordion-trigger" => native_accordion_item(id, node)?.trigger_id(),
        "accordion-panel" => native_accordion_item(id, node)?.panel_id(),
        "field" => native_field(id, node).root_id(),
        "field-label" | "field-passive-label" => native_field(id, node).label_id(),
        "field-description" => native_field(id, node).description_id(),
        "field-error" => native_field(id, node).error_id(),
        "fieldset-legend" => native_fieldset(id, node).legend_id(),
        "fieldset-description" => native_fieldset(id, node).description_id(),
        "dialog" => native_dialog(id, node).root_id(),
        "dialog-backdrop" => native_dialog(id, node).backdrop_id(),
        "dialog-popup" => native_dialog(id, node).popover_id(),
        "dialog-title" => native_dialog(id, node).title_id(),
        "dialog-description" => native_dialog(id, node).description_id(),
        "dialog-close" => native_dialog(id, node).close_id(),
        _ => return None,
    })
}

/// Apply the Rust core part descriptor named by the `part` property.
///
/// `None` means the core decided this part is not mounted, such as an inactive tab panel or a
/// closed collapsible panel without `keepMounted`.
pub(super) fn apply_part(element: Element, id: u32, node: &NativeNode) -> Option<Element> {
    let Some(part) = node.string(property::PART) else {
        return Some(element);
    };
    Some(match part {
        "checkbox" => Checkbox::new(native_toggle_state(node)).root_part(element),
        "checkbox-indicator" => Checkbox::new(ToggleState::Off).indicator_part(element),
        "radio" => Radio::new(node.boolean(property::CHECKED).unwrap_or(false)).root_part(element),
        "radio-indicator" => Radio::new(false).indicator_part(element),
        "radio-group" => RadioGroup::new().root_part(element),
        "switch" => {
            Switch::new(node.boolean(property::CHECKED).unwrap_or(false)).root_part(element)
        }
        "switch-thumb" => Switch::new(false).thumb_part(element),
        "tabs" => native_tabs(id, node).root_part(element),
        "tabs-list" => native_tabs(id, node).list_part(element),
        "tab" => match native_tab(id, node) {
            Some(tab) => tab.tab_part(element),
            None => element,
        },
        "tab-indicator" => native_tab(id, node)?.indicator_part(element)?,
        "tab-panel" => native_tab(id, node)?.panel_part(element)?,
        "collapsible" => native_collapsible(id, node).root_part(element),
        "collapsible-trigger" => native_collapsible(id, node).trigger_part(element),
        "collapsible-panel" => native_collapsible(id, node).panel_part(element)?,
        "accordion" => Accordion::new(native_part_scope(id, node)).root_part(element),
        "accordion-item" => match native_accordion_item(id, node) {
            Some(item) => item.root_part(element),
            None => element,
        },
        "accordion-header" => match native_accordion_item(id, node) {
            Some(item) => item.header_part(element),
            None => element,
        },
        "accordion-trigger" => match native_accordion_item(id, node) {
            Some(item) => item.trigger_part(element),
            None => element,
        },
        "accordion-panel" => native_accordion_item(id, node)?.panel_part(element)?,
        "field" => native_field(id, node).root_part(element),
        "field-label" => native_field(id, node).label_part(element),
        "field-passive-label" => native_field(id, node).passive_label_part(element),
        "field-control" => native_field(id, node).control_part(element),
        "field-description" => native_field(id, node).description_part(element),
        "field-error" => native_field(id, node).error_part(element),
        "fieldset" => native_fieldset(id, node).root_part(element),
        "fieldset-legend" => native_fieldset(id, node).legend_part(element),
        "fieldset-description" => native_fieldset(id, node).description_part(element),
        "fieldset-control" => native_fieldset(id, node).control_part(element),
        // The Rust guide requires the portal root to be mounted only while the dialog is open, so
        // a closed dialog contributes no overlay, focus trap, backdrop, or accessibility node.
        "dialog" => {
            let dialog = native_dialog(id, node);
            if !dialog.is_open() {
                return None;
            }
            dialog.root_part(element)
        }
        "dialog-trigger" => {
            native_dialog(id, node).trigger_part(ElementId::new(id as u64), element)
        }
        "dialog-backdrop" => native_dialog(id, node).backdrop_part(element),
        "dialog-popup" => native_dialog(id, node).popover_part(element),
        "dialog-title" => native_dialog(id, node).title_part(element),
        "dialog-description" => native_dialog(id, node).description_part(element),
        "dialog-close" => native_dialog(id, node).close_part(
            node.string(property::ACCESSIBILITY_LABEL)
                .unwrap_or("Close"),
            element,
        ),
        _ => element,
    })
}

/// Attach the core's delayed, pointer-passive tooltip declared by the `tooltip` property.
pub(super) fn apply_tooltip(element: Element, node: &NativeNode) -> Element {
    let Some(label) = node
        .string(property::TOOLTIP)
        .filter(|label| !label.is_empty())
    else {
        return element;
    };
    let mut tooltip = Tooltip::text(bounded_tooltip_text(label));
    if let Some(placement) = node
        .string(property::TOOLTIP_PLACEMENT)
        .and_then(parse_anchor_placement)
    {
        tooltip = tooltip.placement(placement);
    }
    if let Some(milliseconds) = node.number(property::TOOLTIP_DELAY) {
        tooltip = tooltip.delay(Duration::from_secs_f32(
            (milliseconds / 1_000.0).clamp(0.0, 10.0),
        ));
    }
    if let Some(gap) = node.number(property::TOOLTIP_GAP) {
        tooltip = tooltip.gap(gap);
    }
    if let Some(margin) = node.number(property::TOOLTIP_VIEWPORT_MARGIN) {
        tooltip = tooltip.viewport_margin(margin);
    }
    element.tooltip(tooltip)
}

fn bounded_tooltip_text(label: &str) -> Arc<str> {
    if label.len() <= MAX_TOOLTIP_TEXT_BYTES {
        return Arc::from(label);
    }
    let mut end = MAX_TOOLTIP_TEXT_BYTES;
    while end > 0 && !label.is_char_boundary(end) {
        end -= 1;
    }
    Arc::from(&label[..end])
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
    let border_widths = native_border_widths(node);
    if border_widths.top > 0.0
        || border_widths.right > 0.0
        || border_widths.bottom > 0.0
        || border_widths.left > 0.0
    {
        element = element.border_widths(border_widths).border_color(
            node.color(property::BORDER_COLOR)
                .unwrap_or(Color::TRANSPARENT),
        );
    }
    if let Some(value) = node.number(property::BORDER_RADIUS) {
        element = element.rounded(value);
    }
    if let Some(shadows) = native_box_shadows(node) {
        element = element.shadows(shadows);
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

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct NativeBoxShadow {
    offset_x: f32,
    offset_y: f32,
    blur_radius: f32,
    spread_radius: f32,
    color: Option<u32>,
    #[serde(default)]
    inset: bool,
}

fn native_border_widths(node: &NativeNode) -> Insets {
    let border_width = node.number(property::BORDER_WIDTH).unwrap_or(0.0);
    Insets {
        top: node
            .number(property::BORDER_TOP_WIDTH)
            .unwrap_or(border_width),
        right: node
            .number(property::BORDER_RIGHT_WIDTH)
            .unwrap_or(border_width),
        bottom: node
            .number(property::BORDER_BOTTOM_WIDTH)
            .unwrap_or(border_width),
        left: node
            .number(property::BORDER_LEFT_WIDTH)
            .unwrap_or(border_width),
    }
}

fn native_box_shadows(node: &NativeNode) -> Option<Vec<BoxShadow>> {
    let encoded = node.string(property::BOX_SHADOW)?;
    let shadows = serde_json::from_str::<Vec<NativeBoxShadow>>(encoded).ok()?;
    if shadows.len() > MAX_BOX_SHADOWS_PER_ELEMENT {
        return None;
    }
    let current_color = node.color(property::COLOR).unwrap_or(Color::BLACK);
    Some(
        shadows
            .into_iter()
            .map(|shadow| {
                BoxShadow::new(
                    shadow.offset_x,
                    shadow.offset_y,
                    shadow.color.map(unpack_color).unwrap_or(current_color),
                )
                .blur_radius(shadow.blur_radius)
                .spread_radius(shadow.spread_radius)
                .inset(shadow.inset)
            })
            .collect(),
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

#[cfg(test)]
mod paint_tests {
    use super::*;

    #[test]
    fn native_border_edges_override_the_uniform_width_independently() {
        let mut node = NativeNode::new(NodeTag::View);
        node.set_property(property::BORDER_WIDTH, Some(PropertyValue::Number(1.0)));
        node.set_property(property::BORDER_TOP_WIDTH, Some(PropertyValue::Number(0.0)));
        node.set_property(
            property::BORDER_LEFT_WIDTH,
            Some(PropertyValue::Number(4.0)),
        );

        assert_eq!(
            native_border_widths(&node),
            Insets {
                top: 0.0,
                right: 1.0,
                bottom: 1.0,
                left: 4.0,
            }
        );
    }

    #[test]
    fn native_box_shadow_json_uses_explicit_and_current_text_colors() {
        let mut node = NativeNode::new(NodeTag::View);
        node.set_property(property::COLOR, Some(PropertyValue::Color(0xff665544)));
        node.set_property(
            property::BOX_SHADOW,
            Some(PropertyValue::String(Arc::from(
                r#"[{"offsetX":2,"offsetY":3,"blurRadius":8,"spreadRadius":-1,"color":2150834689,"inset":false},{"offsetX":0,"offsetY":1,"blurRadius":0,"spreadRadius":0,"color":null,"inset":true}]"#,
            ))),
        );

        let shadows = native_box_shadows(&node).expect("valid box shadows");
        assert_eq!(shadows.len(), 2);
        assert_eq!(shadows[0].offset(), quickgui::Vector::new(2.0, 3.0));
        assert_eq!(shadows[0].blur(), 8.0);
        assert_eq!(shadows[0].spread(), -1.0);
        assert_eq!(shadows[0].color(), unpack_color(0x80332201));
        assert!(!shadows[0].is_inset());
        assert_eq!(shadows[1].color(), unpack_color(0xff665544));
        assert!(shadows[1].is_inset());
    }
}
