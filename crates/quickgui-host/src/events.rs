//! Declared input listeners bound to the Rust core's event system.
//!
//! Every listener is declared ahead of the core's decision, exactly like the existing click, hover,
//! and dismissal paths: the property says whether a listener exists, and the core's own targeting,
//! capture, bubbling, and default policy decide when it runs. Payloads travel back as bounded
//! asynchronous JSON events, so the hosted boundary never answers a synchronous question.

use super::*;
use serde::Deserialize;

/// Longest bounded keymap declaration accepted from one `keymap` property.
pub(super) const MAX_KEYMAP_JSON_BYTES: usize = 64 * 1024;
/// Most accelerator bindings retained from one declared keymap.
pub(super) const MAX_KEYMAP_BINDINGS: usize = 256;
/// Longest bounded drag declaration accepted from one `draggable` property.
pub(super) const MAX_DRAG_JSON_BYTES: usize = 64 * 1024;

/// A process-local drag payload declared by one hosted node.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct NativeDragPayload {
    pub(super) source: u32,
    pub(super) id: Arc<str>,
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
struct NativeDragFile {
    path: String,
    #[serde(default)]
    directory: bool,
}

/// One declared drag source.
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
struct NativeDragDeclaration {
    /// Stable application-local identifier delivered to a matching drop target.
    id: Option<String>,
    /// Plain text promoted to other applications.
    text: Option<String>,
    /// Absolute URL promoted to other applications.
    url: Option<String>,
    /// Existing files or directories promoted to other applications.
    #[serde(default)]
    files: Vec<NativeDragFile>,
}

fn modifier_fields(modifiers: quickgui::Modifiers) -> Vec<(String, serde_json::Value)> {
    vec![
        (
            "shift".to_owned(),
            modifiers.contains(quickgui::Modifiers::SHIFT).into(),
        ),
        (
            "control".to_owned(),
            modifiers.contains(quickgui::Modifiers::CONTROL).into(),
        ),
        (
            "alt".to_owned(),
            modifiers.contains(quickgui::Modifiers::ALT).into(),
        ),
        (
            "meta".to_owned(),
            modifiers.contains(quickgui::Modifiers::SUPER).into(),
        ),
    ]
}

fn event_object(
    fields: impl IntoIterator<Item = (String, serde_json::Value)>,
    modifiers: quickgui::Modifiers,
) -> Arc<str> {
    let mut object = serde_json::Map::new();
    for (name, value) in fields {
        object.insert(name, value);
    }
    for (name, value) in modifier_fields(modifiers) {
        object.insert(name, value);
    }
    Arc::from(serde_json::Value::Object(object).to_string().as_str())
}

fn point_fields(point: quickgui::Point) -> Vec<(String, serde_json::Value)> {
    vec![
        ("x".to_owned(), f64::from(point.x).into()),
        ("y".to_owned(), f64::from(point.y).into()),
    ]
}

/// Project one normalized core key onto its DOM-shaped `KeyboardEvent.key` name.
pub(super) fn key_name(key: &quickgui::Key) -> String {
    match key {
        quickgui::Key::Character(value) => value.clone(),
        quickgui::Key::ArrowUp => "ArrowUp".to_owned(),
        quickgui::Key::ArrowDown => "ArrowDown".to_owned(),
        quickgui::Key::ArrowLeft => "ArrowLeft".to_owned(),
        quickgui::Key::ArrowRight => "ArrowRight".to_owned(),
        quickgui::Key::PageUp => "PageUp".to_owned(),
        quickgui::Key::PageDown => "PageDown".to_owned(),
        quickgui::Key::Home => "Home".to_owned(),
        quickgui::Key::End => "End".to_owned(),
        quickgui::Key::Enter => "Enter".to_owned(),
        quickgui::Key::Escape => "Escape".to_owned(),
        quickgui::Key::Space => " ".to_owned(),
        quickgui::Key::Tab => "Tab".to_owned(),
        quickgui::Key::Backspace => "Backspace".to_owned(),
        quickgui::Key::Delete => "Delete".to_owned(),
        quickgui::Key::Insert => "Insert".to_owned(),
        quickgui::Key::Function(index) => format!("F{index}"),
        quickgui::Key::Other => "Unidentified".to_owned(),
    }
}

fn mouse_button_name(button: quickgui::MouseButton) -> String {
    match button {
        quickgui::MouseButton::Left => "left".to_owned(),
        quickgui::MouseButton::Right => "right".to_owned(),
        quickgui::MouseButton::Middle => "middle".to_owned(),
        quickgui::MouseButton::Back => "back".to_owned(),
        quickgui::MouseButton::Forward => "forward".to_owned(),
        quickgui::MouseButton::Other(code) => format!("other-{code}"),
    }
}

fn gesture_phase_name(phase: quickgui::GesturePhase) -> &'static str {
    match phase {
        quickgui::GesturePhase::Started => "started",
        quickgui::GesturePhase::Moved => "moved",
        quickgui::GesturePhase::Ended => "ended",
        quickgui::GesturePhase::Cancelled => "cancelled",
    }
}

/// Bounded accelerator bindings declared for one element.
#[derive(Clone, Debug, Default)]
pub(super) struct NativeKeymap {
    bindings: Vec<(quickgui::Keystroke, Arc<str>)>,
}

impl NativeKeymap {
    /// Parse the declared accelerator table with the core's own Electron-shaped parser.
    ///
    /// An accelerator the core rejects is skipped so one typo cannot silence the whole table.
    pub(super) fn parse(declaration: &str) -> Option<Self> {
        if declaration.len() > MAX_KEYMAP_JSON_BYTES {
            return None;
        }
        let declared = serde_json::from_str::<BTreeMap<String, String>>(declaration).ok()?;
        let mut bindings = Vec::new();
        for (accelerator, id) in declared {
            if bindings.len() >= MAX_KEYMAP_BINDINGS {
                break;
            }
            if id.is_empty() || id.len() > MAX_COMPONENT_VALUE_BYTES {
                continue;
            }
            let Ok(keystroke) = quickgui::Accelerator::parse(&accelerator) else {
                continue;
            };
            bindings.push((keystroke, Arc::from(id.as_str())));
        }
        (!bindings.is_empty()).then_some(Self { bindings })
    }

    /// Resolve the declared binding identifier for one normalized key press.
    pub(super) fn binding_for(&self, event: &quickgui::KeyDownEvent) -> Option<Arc<str>> {
        let command = quickgui::Keystroke::from_key_event(&event.key, event.modifiers);
        let printable = event
            .key_char
            .as_ref()
            .map(|key| quickgui::Keystroke::from_key_event(key, event.modifiers));
        self.bindings
            .iter()
            .find(|(keystroke, _)| {
                *keystroke == command || printable.as_ref().is_some_and(|key| key == keystroke)
            })
            .map(|(_, id)| Arc::clone(id))
    }

    #[cfg(test)]
    pub(super) fn len(&self) -> usize {
        self.bindings.len()
    }
}

/// Attach every declared input listener to one caller-owned element.
///
/// Listeners are additive and independent; an element that declares none keeps the untouched fast
/// path with no core listener registration at all. Forwarding only wakes the host event queue;
/// Go's eventual mutation batch determines whether the retained view needs layout or paint.
#[allow(clippy::too_many_lines)]
#[allow(clippy::too_many_arguments)]
pub(super) fn attach_input_listeners(
    mut element: Element,
    element_id: ElementId,
    id: u32,
    window: u32,
    events: &EventQueue,
    node: &NativeNode,
    cx: &mut ViewContext<'_, NativeView>,
    core_owns_scroll_wheel: bool,
) -> Element {
    let declared = |key: u16| node.boolean(key).unwrap_or(false);
    let keymap = node
        .string(property::KEYMAP)
        .filter(|_| declared(property::ACTION_LISTENER))
        .and_then(NativeKeymap::parse);

    if declared(property::KEY_DOWN_LISTENER) || keymap.is_some() {
        let queue = Rc::clone(events);
        let wants_key_down = declared(property::KEY_DOWN_LISTENER);
        let listener = cx.key_down_listener(element_id, move |_view, event, cx| {
            // The declared keymap resolves first, so an accelerator becomes one typed action
            // instead of a raw key JavaScript would have to interpret itself.
            if let Some(binding) = keymap.as_ref().and_then(|keymap| keymap.binding_for(event)) {
                enqueue_event(
                    &queue,
                    QueuedEvent {
                        kind: "action",
                        window,
                        target: id,
                        value: Some(binding),
                    },
                );
                cx.prevent_default();
                cx.stop_propagation();
                return;
            }
            if !wants_key_down {
                return;
            }
            let mut fields = vec![
                ("key".to_owned(), key_name(&event.key).into()),
                ("repeat".to_owned(), event.repeat.into()),
            ];
            if let Some(text) = &event.text {
                fields.push(("text".to_owned(), text.clone().into()));
            }
            enqueue_event(
                &queue,
                QueuedEvent {
                    kind: "keydown",
                    window,
                    target: id,
                    value: Some(event_object(fields, event.modifiers)),
                },
            );
        });
        element = element.on_key_down(listener);
    }

    if declared(property::KEY_UP_LISTENER) {
        let queue = Rc::clone(events);
        let listener = cx.key_up_listener(element_id, move |_view, event, _cx| {
            enqueue_event(
                &queue,
                QueuedEvent {
                    kind: "keyup",
                    window,
                    target: id,
                    value: Some(event_object(
                        vec![("key".to_owned(), key_name(&event.key).into())],
                        event.modifiers,
                    )),
                },
            );
        });
        element = element.on_key_up(listener);
    }

    let wants_mouse_down = declared(property::MOUSE_DOWN_LISTENER);
    let wants_double_click = declared(property::DOUBLE_CLICK_LISTENER);
    if wants_mouse_down || wants_double_click {
        let queue = Rc::clone(events);
        let listener = cx.mouse_down_listener(element_id, move |_view, event, _cx| {
            let payload = || {
                let mut fields = point_fields(event.position);
                fields.push(("button".to_owned(), mouse_button_name(event.button).into()));
                fields.push(("clickCount".to_owned(), event.click_count.into()));
                fields.push(("firstMouse".to_owned(), event.first_mouse.into()));
                event_object(fields, event.modifiers)
            };
            if wants_mouse_down {
                enqueue_event(
                    &queue,
                    QueuedEvent {
                        kind: "mousedown",
                        window,
                        target: id,
                        value: Some(payload()),
                    },
                );
            }
            // The exact native multi-click count is the core's, so a double click is the second
            // press of one native sequence rather than a JavaScript timer.
            if wants_double_click && event.click_count == 2 {
                enqueue_event(
                    &queue,
                    QueuedEvent {
                        kind: "dblclick",
                        window,
                        target: id,
                        value: Some(payload()),
                    },
                );
            }
        });
        element = element.on_any_mouse_down(listener);
    }

    if declared(property::MOUSE_UP_LISTENER) {
        let queue = Rc::clone(events);
        let listener = cx.mouse_up_listener(element_id, move |_view, event, _cx| {
            let mut fields = point_fields(event.position);
            fields.push(("button".to_owned(), mouse_button_name(event.button).into()));
            fields.push(("clickCount".to_owned(), event.click_count.into()));
            enqueue_event(
                &queue,
                QueuedEvent {
                    kind: "mouseup",
                    window,
                    target: id,
                    value: Some(event_object(fields, event.modifiers)),
                },
            );
        });
        element = element.on_any_mouse_up(listener);
    }

    if declared(property::MOUSE_MOVE_LISTENER) {
        let queue = Rc::clone(events);
        let listener = cx.mouse_move_listener(element_id, move |_view, event, _cx| {
            let mut fields = point_fields(event.position);
            fields.push((
                "pressedButton".to_owned(),
                match event.pressed_button {
                    Some(button) => mouse_button_name(button).into(),
                    None => serde_json::Value::Null,
                },
            ));
            enqueue_event(
                &queue,
                QueuedEvent {
                    kind: "mousemove",
                    window,
                    target: id,
                    value: Some(event_object(fields, event.modifiers)),
                },
            );
        });
        element = element.on_mouse_move(listener);
    }

    if declared(property::SCROLL_LISTENER) && !core_owns_scroll_wheel {
        let queue = Rc::clone(events);
        let listener = cx.scroll_wheel_listener(element_id, move |_view, event, _cx| {
            let pixels = event.delta.pixel_delta(DEFAULT_SCROLL_LINE_HEIGHT);
            let mut fields = point_fields(event.position);
            fields.push(("deltaX".to_owned(), f64::from(pixels.x).into()));
            fields.push(("deltaY".to_owned(), f64::from(pixels.y).into()));
            fields.push(("precise".to_owned(), event.delta.precise().into()));
            fields.push(("phase".to_owned(), gesture_phase_name(event.phase).into()));
            enqueue_event(
                &queue,
                QueuedEvent {
                    kind: "wheel",
                    window,
                    target: id,
                    value: Some(event_object(fields, event.modifiers)),
                },
            );
        });
        element = element.on_scroll_wheel(listener);
    }

    // A declared context-menu target already owns the core's single secondary-click listener for
    // this identity, so the generic event never registers a second one.
    if declared(property::CONTEXT_MENU_LISTENER)
        && node.string(property::PART) != Some(CONTEXT_MENU_TRIGGER_PART)
    {
        let queue = Rc::clone(events);
        let listener = cx.context_menu_listener(element_id, move |_view, event, _cx| {
            enqueue_event(
                &queue,
                QueuedEvent {
                    kind: "contextmenu",
                    window,
                    target: id,
                    value: Some(event_object(point_fields(event.position), event.modifiers)),
                },
            );
        });
        element = element.on_context_menu(listener);
    }

    if declared(property::PINCH_LISTENER) {
        let queue = Rc::clone(events);
        let listener = cx.pinch_listener(element_id, move |_view, event, _cx| {
            enqueue_event(
                &queue,
                QueuedEvent {
                    kind: "pinch",
                    window,
                    target: id,
                    value: Some(gesture_value(
                        event.position,
                        event.delta,
                        event.phase,
                        event.modifiers,
                    )),
                },
            );
        });
        element = element.on_pinch(listener);
    }

    if declared(property::ROTATION_LISTENER) {
        let queue = Rc::clone(events);
        let listener = cx.rotation_listener(element_id, move |_view, event, _cx| {
            enqueue_event(
                &queue,
                QueuedEvent {
                    kind: "rotate",
                    window,
                    target: id,
                    value: Some(gesture_value(
                        event.position,
                        event.delta,
                        event.phase,
                        event.modifiers,
                    )),
                },
            );
        });
        element = element.on_rotation(listener);
    }

    if declared(property::SMART_MAGNIFY_LISTENER) {
        let queue = Rc::clone(events);
        let listener = cx.smart_magnify_listener(element_id, move |_view, event, _cx| {
            enqueue_event(
                &queue,
                QueuedEvent {
                    kind: "smartmagnify",
                    window,
                    target: id,
                    value: Some(event_object(point_fields(event.position), event.modifiers)),
                },
            );
        });
        element = element.on_smart_magnify(listener);
    }

    if declared(property::PRESSURE_LISTENER) {
        let queue = Rc::clone(events);
        let listener = cx.mouse_pressure_listener(element_id, move |_view, event, _cx| {
            let mut fields = point_fields(event.position);
            fields.push(("pressure".to_owned(), f64::from(event.pressure).into()));
            fields.push((
                "stage".to_owned(),
                match event.stage {
                    quickgui::PressureStage::Zero => "zero".to_owned(),
                    quickgui::PressureStage::Normal => "normal".to_owned(),
                    quickgui::PressureStage::Force => "force".to_owned(),
                    quickgui::PressureStage::Other(stage) => format!("other-{stage}"),
                }
                .into(),
            ));
            enqueue_event(
                &queue,
                QueuedEvent {
                    kind: "pressure",
                    window,
                    target: id,
                    value: Some(event_object(fields, event.modifiers)),
                },
            );
        });
        element = element.on_mouse_pressure(listener);
    }

    attach_drag_and_drop(element, element_id, id, window, events, node, cx)
}

/// Logical-pixel height used to project discrete wheel lines for JavaScript listeners.
const DEFAULT_SCROLL_LINE_HEIGHT: f32 = 16.0;

fn gesture_value(
    position: quickgui::Point,
    delta: f32,
    phase: quickgui::GesturePhase,
    modifiers: quickgui::Modifiers,
) -> Arc<str> {
    let mut fields = point_fields(position);
    fields.push(("delta".to_owned(), f64::from(delta).into()));
    fields.push(("phase".to_owned(), gesture_phase_name(phase).into()));
    event_object(fields, modifiers)
}

fn drag_origin_name(origin: quickgui::DragOrigin) -> &'static str {
    match origin {
        quickgui::DragOrigin::Internal(_) => "internal",
        quickgui::DragOrigin::CrossWindow { .. } => "cross-window",
        quickgui::DragOrigin::External => "external",
    }
}

/// Attach the declared drag source and drop targets.
fn attach_drag_and_drop(
    mut element: Element,
    element_id: ElementId,
    id: u32,
    window: u32,
    events: &EventQueue,
    node: &NativeNode,
    cx: &mut ViewContext<'_, NativeView>,
) -> Element {
    // The core rejects one element owning both a captured pointer stream and a typed drag source,
    // so a conflicting declaration is ignored instead of panicking the render. Drop targets are
    // unaffected and stay declared.
    let captures_pointer = node.boolean(property::POINTER_LISTENER).unwrap_or(false);
    if let Some(declaration) = node
        .string(property::DRAGGABLE)
        .filter(|_| !captures_pointer)
        .filter(|value| value.len() <= MAX_DRAG_JSON_BYTES)
        .and_then(|value| serde_json::from_str::<NativeDragDeclaration>(value).ok())
    {
        let payload = NativeDragPayload {
            source: id,
            id: Arc::from(declaration.id.as_deref().unwrap_or("")),
        };
        let external = external_drag_payload(&declaration);
        let queue = Rc::clone(events);
        let wants_start = node.boolean(property::DRAG_LISTENER).unwrap_or(false);
        let listener = cx.drag_listener(element_id, move |_view, event, _cx| {
            if wants_start {
                enqueue_event(
                    &queue,
                    QueuedEvent {
                        kind: "dragstart",
                        window,
                        target: id,
                        value: Some(event_object(point_fields(event.position), event.modifiers)),
                    },
                );
            }
            let drag = quickgui::Drag::new(payload.clone());
            match external.clone() {
                Some(external) => drag.external_payload(external),
                None => drag,
            }
        });
        element = element.on_drag(listener);
    }

    let kinds = node
        .string(property::DROP_KINDS)
        .and_then(|value| serde_json::from_str::<Vec<String>>(value).ok())
        .unwrap_or_default();
    if node.boolean(property::DROP_LISTENER).unwrap_or(false) {
        // Accepted payload kinds are declared ahead of the native drag so the core can decide
        // compatibility while the pointer is still moving.
        if kinds
            .iter()
            .any(|kind| kind == "local" || kind == "app-local")
        {
            let queue = Rc::clone(events);
            let listener = cx.drop_listener::<NativeDragPayload>(
                element_id,
                move |_view, payload, event, _cx| {
                    let mut fields = point_fields(event.position);
                    fields.push(("id".to_owned(), payload.id.to_string().into()));
                    fields.push(("source".to_owned(), payload.source.into()));
                    fields.push(("origin".to_owned(), drag_origin_name(event.origin).into()));
                    enqueue_event(
                        &queue,
                        QueuedEvent {
                            kind: "drop",
                            window,
                            target: id,
                            value: Some(event_object(fields, event.modifiers)),
                        },
                    );
                },
            );
            element = element.on_drop(listener);
        }
        if kinds.iter().any(|kind| kind == "files") {
            let queue = Rc::clone(events);
            let listener = cx.drop_listener::<quickgui::DroppedFiles>(
                element_id,
                move |_view, files, event, _cx| {
                    let mut fields = point_fields(event.position);
                    fields.push((
                        "paths".to_owned(),
                        serde_json::Value::Array(
                            files
                                .paths()
                                .iter()
                                .map(|path| {
                                    serde_json::Value::String(path.to_string_lossy().into_owned())
                                })
                                .collect(),
                        ),
                    ));
                    fields.push(("origin".to_owned(), drag_origin_name(event.origin).into()));
                    enqueue_event(
                        &queue,
                        QueuedEvent {
                            kind: "filesdropped",
                            window,
                            target: id,
                            value: Some(event_object(fields, event.modifiers)),
                        },
                    );
                },
            );
            element = element.on_drop(listener);
        }
    }
    element
}

fn external_drag_payload(
    declaration: &NativeDragDeclaration,
) -> Option<quickgui::ExternalDragPayload> {
    if !declaration.files.is_empty() {
        return Some(quickgui::ExternalDragPayload::Files(
            quickgui::FileDragPaths::new(
                declaration
                    .files
                    .iter()
                    .map(|file| (PathBuf::from(&file.path), file.directory)),
            ),
        ));
    }
    if let Some(url) = declaration.url.as_deref()
        && let Ok(url) = quickgui::ExternalDragUrl::new(url)
    {
        return Some(quickgui::ExternalDragPayload::Url(url));
    }
    declaration
        .text
        .as_deref()
        .map(|text| quickgui::ExternalDragPayload::Text(quickgui::ExternalDragText::new(text)))
}
