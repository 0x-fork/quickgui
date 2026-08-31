use super::*;

impl TestAppContext {
    pub(super) fn move_visual_pointer(
        &mut self,
        window: WindowHandle,
        point: Point,
    ) -> Result<bool, TestAppError> {
        self.render_visual(window, false)?;
        self.window_mut(window)?.pointer = Some(point);
        let mut renderer = self.visual_renderer.take().ok_or_else(|| {
            TestAppError::Visual(crate::VisualTestError::Render(
                "visual renderer was not retained after layout".to_owned(),
            ))
        })?;
        let changed = self
            .window_mut(window)?
            .ui
            .pointer_moved(point, &mut renderer);
        self.visual_renderer = Some(renderer);
        let hover_changed = self.invoke_pending_mouse_hover(window)?;
        if changed || hover_changed {
            self.render_visual(window, false)?;
        }
        Ok(changed || hover_changed)
    }

    pub(super) fn invoke_pending_mouse_hover(
        &mut self,
        window: WindowHandle,
    ) -> Result<bool, TestAppError> {
        let mut changes = Vec::with_capacity(4);
        self.window_mut(window)?
            .ui
            .take_mouse_hover_changes(&mut changes);
        let changed = !changes.is_empty();
        for change in changes {
            let listener = self.window(window)?.listeners.mouse_listener(change.key);
            let Some(listener) = listener else {
                continue;
            };
            let mut cx = self.event_context(Some(window));
            listener(
                self.window_mut(window)?.view.as_any_mut(),
                &MouseListenerEvent::Hover(change.hovered),
                &mut cx,
            );
            self.apply_context(Some(window), cx)?;
        }
        self.run_until_idle()?;
        Ok(changed)
    }

    pub fn focused(&self, window: WindowHandle) -> Result<Option<ElementId>, TestAppError> {
        Ok(self.window(window)?.ui.focused())
    }

    pub fn focused_input_value(
        &self,
        window: WindowHandle,
    ) -> Result<Option<Arc<str>>, TestAppError> {
        Ok(self.window(window)?.ui.focused_text_input_value())
    }

    /// Current deterministic monotonic time used by foreground timers.
    pub fn now(&self) -> Instant {
        self.now.get()
    }

    pub fn focus(
        &mut self,
        window: WindowHandle,
        element: impl Into<ElementId>,
    ) -> Result<(), TestAppError> {
        let element = element.into();
        let previous = self.window(window)?.ui.focused();
        if !self.window(window)?.ui.is_focusable(element) {
            return Err(TestAppError::NotFocusable { window, element });
        }
        self.window_mut(window)?.ui.focus(element);
        self.focus_changed(window, previous)?;
        self.run_until_idle()
    }

    pub fn blur(&mut self, window: WindowHandle) -> Result<(), TestAppError> {
        let previous = self.window(window)?.ui.focused();
        self.window_mut(window)?.ui.blur();
        self.focus_changed(window, previous)?;
        self.run_until_idle()
    }

    pub fn click(
        &mut self,
        window: WindowHandle,
        element: impl Into<ElementId>,
    ) -> Result<(), TestAppError> {
        let element = element.into();
        self.require_element(window, element)?;
        if !self.window(window)?.ui.is_clickable(element) {
            return Err(TestAppError::NotClickable { window, element });
        }
        if self.window(window)?.ui.is_focusable(element) {
            let previous = self.window(window)?.ui.focused();
            self.window_mut(window)?.ui.focus(element);
            self.focus_changed(window, previous)?;
            self.run_until_idle()?;
            self.require_element(window, element)?;
            if !self.window(window)?.ui.is_clickable(element) {
                return Err(TestAppError::NotClickable { window, element });
            }
        }
        let activation_target = self.window(window)?.ui.activation_target(element);
        let form = self.window(window)?.ui.form_for_submitter(element);
        let listener = self.window(window)?.listeners.clicks.get(&element).cloned();
        let mut default_prevented = false;
        if let Some(listener) = listener {
            let mut cx = self.event_context(Some(window));
            listener(self.window_mut(window)?.view.as_any_mut(), &mut cx);
            default_prevented = cx.prevent_default;
            self.apply_context(Some(window), cx)?;
        }
        self.queue_dispatch(TestDispatch::Event(window, Event::Click(element)))?;
        if let Some(form) = form {
            self.queue_dispatch(TestDispatch::Form(window, form, Some(element)))?;
        }
        self.run_until_idle()?;
        let Some(target) = activation_target
            .filter(|target| *target != element)
            .filter(|_| !default_prevented)
        else {
            return Ok(());
        };
        let focusable = self.window(window)?.ui.is_focusable(target);
        let clickable = self.window(window)?.ui.is_clickable(target);
        if focusable {
            let previous = self.window(window)?.ui.focused();
            self.window_mut(window)?.ui.focus(target);
            self.focus_changed(window, previous)?;
            self.run_until_idle()?;
        }
        if clickable {
            self.click(window, target)?;
        }
        Ok(())
    }

    /// Deliver one deterministic web-style context-menu request to a declared secondary-click
    /// target without creating native or GPU resources for the owner window.
    pub fn simulate_context_menu(
        &mut self,
        window: WindowHandle,
        element: impl Into<ElementId>,
        position: Point,
        modifiers: Modifiers,
    ) -> Result<(), TestAppError> {
        let element = element.into();
        self.require_element(window, element)?;
        let listener = self
            .window(window)?
            .listeners
            .context_menus
            .get(&element)
            .cloned()
            .ok_or(TestAppError::NotListening {
                window,
                element,
                kind: "context menu",
            })?;
        let event = ContextMenuEvent {
            target: element,
            position,
            modifiers,
        };
        let mut cx = self.event_context(Some(window));
        listener(self.window_mut(window)?.view.as_any_mut(), &event, &mut cx);
        self.apply_context(Some(window), cx)?;
        self.queue_dispatch(TestDispatch::Event(window, Event::ContextMenu(event)))?;
        self.run_until_idle()
    }

    /// Deliver a targeted desktop mouse press through outside capture, capture, and bubble.
    ///
    /// `element` is the deterministic hit target. The returned value reports whether any callback
    /// prevented the framework default; propagation and default prevention remain independent.
    pub fn simulate_mouse_down(
        &mut self,
        window: WindowHandle,
        element: impl Into<ElementId>,
        event: MouseDownEvent,
    ) -> Result<bool, TestAppError> {
        self.simulate_mouse_event(
            window,
            element.into(),
            MouseListenerKind::Down,
            Some(event.button),
            MouseListenerEvent::Down(event),
        )
    }

    /// Deliver a targeted desktop mouse release through outside capture, capture, and bubble.
    pub fn simulate_mouse_up(
        &mut self,
        window: WindowHandle,
        element: impl Into<ElementId>,
        event: MouseUpEvent,
    ) -> Result<bool, TestAppError> {
        self.simulate_mouse_event(
            window,
            element.into(),
            MouseListenerKind::Up,
            Some(event.button),
            MouseListenerEvent::Up(event),
        )
    }

    /// Deliver targeted desktop mouse motion through capture and bubble.
    pub fn simulate_mouse_move(
        &mut self,
        window: WindowHandle,
        element: impl Into<ElementId>,
        event: MouseMoveEvent,
    ) -> Result<(), TestAppError> {
        self.window_mut(window)?.pointer = Some(event.position);
        self.simulate_mouse_event(
            window,
            element.into(),
            MouseListenerKind::Move,
            None,
            MouseListenerEvent::Move(event),
        )?;
        Ok(())
    }

    /// Deliver a native-window mouse-exit event along the preceding deterministic target path.
    pub fn simulate_mouse_exit(
        &mut self,
        window: WindowHandle,
        element: impl Into<ElementId>,
        event: MouseExitEvent,
    ) -> Result<(), TestAppError> {
        self.window_mut(window)?.pointer = Some(event.position);
        let result = self.simulate_mouse_event(
            window,
            element.into(),
            MouseListenerKind::Exit,
            None,
            MouseListenerEvent::Exit(event),
        );
        if let Ok(state) = self.window_mut(window) {
            state.pointer = None;
        }
        result.map(|_| ())
    }

    pub(super) fn simulate_mouse_event(
        &mut self,
        window: WindowHandle,
        element: ElementId,
        kind: MouseListenerKind,
        button: Option<MouseButton>,
        event: MouseListenerEvent,
    ) -> Result<bool, TestAppError> {
        self.require_element(window, element)?;
        let mut path = Vec::with_capacity(8);
        if !self
            .window(window)?
            .ui
            .mouse_event_path_for_target(element, &mut path)
        {
            return Err(TestAppError::MouseDispatchPathLimit);
        }
        let mut dispatch = Vec::with_capacity(8);
        self.window(window)?
            .ui
            .collect_mouse_dispatch(&path, kind, button, &mut dispatch);
        let mut default_prevented = false;
        for key in dispatch {
            let listener = self.window(window)?.listeners.mouse_listener(key);
            let Some(listener) = listener else {
                continue;
            };
            let mut cx = self.event_context(Some(window));
            listener(self.window_mut(window)?.view.as_any_mut(), &event, &mut cx);
            let stop_propagation = cx.stop_event_propagation;
            default_prevented |= cx.prevent_default;
            self.apply_context(Some(window), cx)?;
            if stop_propagation {
                break;
            }
        }
        self.run_until_idle()?;
        Ok(default_prevented)
    }

    /// Deliver one deterministic scroll-wheel event through an element's listening ancestors.
    ///
    /// The returned value is true when a callback called [`EventContext::prevent_default`]. This
    /// semantic helper exercises listener ordering without native or GPU resources; production
    /// retained scrolling continues to use the geometry-driven runtime path.
    pub fn simulate_scroll_wheel(
        &mut self,
        window: WindowHandle,
        element: impl Into<ElementId>,
        event: ScrollWheelEvent,
    ) -> Result<bool, TestAppError> {
        let element = element.into();
        self.require_element(window, element)?;
        let mut current = self
            .window(window)?
            .ui
            .scroll_wheel_listener_for_target(element)
            .ok_or(TestAppError::NotListening {
                window,
                element,
                kind: "scroll wheel",
            })?;
        let event = event.bounded();
        let mut default_prevented = false;
        loop {
            let listener = self
                .window(window)?
                .listeners
                .scroll_wheels
                .get(&current)
                .cloned();
            if let Some(listener) = listener {
                let mut cx = self.event_context(Some(window));
                listener(self.window_mut(window)?.view.as_any_mut(), &event, &mut cx);
                let stop_propagation = cx.stop_event_propagation;
                default_prevented |= cx.prevent_default;
                self.apply_context(Some(window), cx)?;
                if stop_propagation {
                    break;
                }
            }
            let Some(parent) = self
                .window(window)?
                .ui
                .parent_scroll_wheel_listener(current)
            else {
                break;
            };
            current = parent;
        }
        self.run_until_idle()?;
        Ok(default_prevented)
    }

    /// Apply one production retained-scroll delta at the center of an element.
    ///
    /// Unlike [`Self::simulate_scroll_wheel`], this exercises the geometry-driven default scroll
    /// path. Ordinary overflow scrolling updates retained paint state without rebuilding a clean
    /// view declaration; virtual scrolling rebuilds when its mounted slice must follow the shared
    /// offset. The returned value reports whether retained state changed.
    pub fn simulate_retained_scroll(
        &mut self,
        window: WindowHandle,
        element: impl Into<ElementId>,
        delta: Vector,
    ) -> Result<bool, TestAppError> {
        let element = element.into();
        self.require_element(window, element)?;
        self.prepare_retained_geometry(window)?;
        let bounds = self
            .window(window)?
            .ui
            .element_bounds(element)
            .ok_or(TestAppError::UnknownElement { window, element })?;
        let point = Point::new(
            bounds.x + bounds.width * 0.5,
            bounds.y + bounds.height * 0.5,
        );
        let now = self.now();
        let result = self
            .window_mut(window)?
            .ui
            .scroll_at(Some(point), delta, now);
        if result.view_dirty {
            let state = self.window_mut(window)?;
            state.dirty = true;
            state.retained_geometry_ready = false;
        }
        self.run_until_idle()?;
        Ok(result.changed)
    }

    /// Read one retained scroll container's current logical offset.
    pub fn retained_scroll_offset(
        &mut self,
        window: WindowHandle,
        element: impl Into<ElementId>,
    ) -> Result<Vector, TestAppError> {
        let element = element.into();
        self.require_element(window, element)?;
        self.prepare_retained_geometry(window)?;
        self.window(window)?
            .ui
            .scroll_offset(element)
            .ok_or(TestAppError::NotScrollable { window, element })
    }

    /// Deliver one deterministic raw touch sample through production capture semantics.
    ///
    /// `element` chooses the hit target only for [`TouchPhase::Started`]. Later samples with the
    /// same [`TouchId`] use the retained capture even if a different element is supplied.
    pub fn simulate_touch(
        &mut self,
        window: WindowHandle,
        element: impl Into<ElementId>,
        event: TouchEvent,
    ) -> Result<(), TestAppError> {
        let element = element.into();
        self.require_element(window, element)?;
        let event = event.bounded();
        let target = match event.phase {
            TouchPhase::Started => {
                let state = self.window_mut(window)?;
                state.touch_captures.remove(&event.id);
                let target = state.ui.touch_listener_for_target(element);
                if let Some(target) = target
                    && state.touch_captures.len() < MAX_ACTIVE_TOUCHES_PER_WINDOW
                {
                    state.touch_captures.insert(
                        event.id,
                        TouchCapture {
                            target,
                            last_event: event,
                        },
                    );
                    Some(target)
                } else {
                    None
                }
            }
            TouchPhase::Moved => self
                .window_mut(window)?
                .touch_captures
                .get_mut(&event.id)
                .map(|capture| {
                    capture.last_event = event;
                    capture.target
                }),
            TouchPhase::Ended | TouchPhase::Cancelled => self
                .window_mut(window)?
                .touch_captures
                .remove(&event.id)
                .map(|capture| capture.target),
        };

        let mut current = target;
        while let Some(id) = current {
            let listener = self.window(window)?.listeners.touches.get(&id).cloned();
            if let Some(listener) = listener {
                let mut cx = self.event_context(Some(window));
                listener(self.window_mut(window)?.view.as_any_mut(), &event, &mut cx);
                let stop_propagation = cx.stop_event_propagation;
                self.apply_context(Some(window), cx)?;
                if stop_propagation {
                    break;
                }
            }
            current = self.window(window)?.ui.parent_touch_listener(id);
        }
        self.queue_dispatch(TestDispatch::Event(window, Event::Touch(event)))?;
        self.run_until_idle()
    }

    /// Deliver one deterministic Force Touch event to an attached element listener.
    pub fn simulate_mouse_pressure(
        &mut self,
        window: WindowHandle,
        element: impl Into<ElementId>,
        event: MousePressureEvent,
    ) -> Result<(), TestAppError> {
        let element = element.into();
        self.require_element(window, element)?;
        let listener = self
            .window(window)?
            .listeners
            .mouse_pressures
            .get(&element)
            .cloned()
            .ok_or(TestAppError::NotListening {
                window,
                element,
                kind: "mouse pressure",
            })?;
        let mut cx = self.event_context(Some(window));
        listener(self.window_mut(window)?.view.as_any_mut(), &event, &mut cx);
        self.apply_context(Some(window), cx)?;
        self.queue_dispatch(TestDispatch::Event(window, Event::MousePressure(event)))?;
        self.run_until_idle()
    }

    /// Deliver one deterministic pinch event to an attached element listener.
    pub fn simulate_pinch(
        &mut self,
        window: WindowHandle,
        element: impl Into<ElementId>,
        event: PinchEvent,
    ) -> Result<(), TestAppError> {
        let element = element.into();
        self.require_element(window, element)?;
        let listener = self
            .window(window)?
            .listeners
            .pinches
            .get(&element)
            .cloned()
            .ok_or(TestAppError::NotListening {
                window,
                element,
                kind: "pinch",
            })?;
        let mut cx = self.event_context(Some(window));
        listener(self.window_mut(window)?.view.as_any_mut(), &event, &mut cx);
        self.apply_context(Some(window), cx)?;
        self.queue_dispatch(TestDispatch::Event(window, Event::Pinch(event)))?;
        self.run_until_idle()
    }

    /// Deliver one deterministic rotation event to an attached element listener.
    pub fn simulate_rotation(
        &mut self,
        window: WindowHandle,
        element: impl Into<ElementId>,
        event: RotationEvent,
    ) -> Result<(), TestAppError> {
        let element = element.into();
        self.require_element(window, element)?;
        let listener = self
            .window(window)?
            .listeners
            .rotations
            .get(&element)
            .cloned()
            .ok_or(TestAppError::NotListening {
                window,
                element,
                kind: "rotation",
            })?;
        let mut cx = self.event_context(Some(window));
        listener(self.window_mut(window)?.view.as_any_mut(), &event, &mut cx);
        self.apply_context(Some(window), cx)?;
        self.queue_dispatch(TestDispatch::Event(window, Event::Rotation(event)))?;
        self.run_until_idle()
    }

    /// Deliver one deterministic smart-magnify event to an attached element listener.
    pub fn simulate_smart_magnify(
        &mut self,
        window: WindowHandle,
        element: impl Into<ElementId>,
        event: SmartMagnifyEvent,
    ) -> Result<(), TestAppError> {
        let element = element.into();
        self.require_element(window, element)?;
        let listener = self
            .window(window)?
            .listeners
            .smart_magnifies
            .get(&element)
            .cloned()
            .ok_or(TestAppError::NotListening {
                window,
                element,
                kind: "smart magnify",
            })?;
        let mut cx = self.event_context(Some(window));
        listener(self.window_mut(window)?.view.as_any_mut(), &event, &mut cx);
        self.apply_context(Some(window), cx)?;
        self.queue_dispatch(TestDispatch::Event(window, Event::SmartMagnify(event)))?;
        self.run_until_idle()
    }

    pub fn dispatch_action<A: Action>(
        &mut self,
        window: WindowHandle,
        action: A,
    ) -> Result<bool, TestAppError> {
        self.window(window)?;
        let consumed = self.invoke_action(window, &AnyAction::new(action))?;
        self.run_until_idle()?;
        Ok(consumed)
    }

    pub fn simulate_input(&mut self, window: WindowHandle, text: &str) -> Result<(), TestAppError> {
        if self.window(window)?.ui.focused_text_input().is_none() {
            return Err(TestAppError::NoFocusedTextInput(window));
        }
        let result = self.window_mut(window)?.ui.input_replace(text);
        let changed = result.change.is_some();
        self.apply_input_result(window, result)?;
        if changed {
            self.queue_dispatch(TestDispatch::Event(
                window,
                Event::TextInput(text.to_owned()),
            ))?;
        }
        self.run_until_idle()
    }

    /// Simulate one or more whitespace-separated normalized keystrokes.
    ///
    /// Multi-stroke bindings are resolved without a wall-clock timeout: an exact shorter binding
    /// is selected when this supplied sequence ends, while a prefix with no exact binding fails.
    pub fn simulate_keystrokes(
        &mut self,
        window: WindowHandle,
        sequence: &str,
    ) -> Result<(), TestAppError> {
        let strokes = sequence
            .split_whitespace()
            .map(Keystroke::parse)
            .collect::<Result<Vec<_>, _>>()
            .map_err(|error| TestAppError::View(error.to_string()))?;
        self.simulate_keystroke_values(window, strokes)
    }

    /// Simulate one already-normalized platform keystroke, including optional `key_char`
    /// metadata.
    pub fn simulate_keystroke(
        &mut self,
        window: WindowHandle,
        stroke: Keystroke,
    ) -> Result<(), TestAppError> {
        self.simulate_keystroke_values(window, vec![stroke])
    }

    /// Simulate one normalized key release through focused capture and bubble listeners.
    pub fn simulate_key_up(
        &mut self,
        window: WindowHandle,
        stroke: Keystroke,
    ) -> Result<(), TestAppError> {
        self.window(window)?;
        self.invoke_key_event(
            window,
            KeyListenerEvent::Up(KeyUpEvent {
                key: stroke.key.clone(),
                key_char: stroke.key_char.clone(),
                modifiers: stroke.modifiers,
            }),
        )?;
        self.queue_dispatch(TestDispatch::Event(
            window,
            Event::KeyUp {
                key: stroke.key,
                key_char: stroke.key_char,
                modifiers: stroke.modifiers,
            },
        ))?;
        self.run_until_idle()
    }

    pub(super) fn simulate_keystroke_values(
        &mut self,
        window: WindowHandle,
        strokes: Vec<Keystroke>,
    ) -> Result<(), TestAppError> {
        self.window(window)?;
        let mut prefix = Vec::new();
        for stroke in strokes {
            prefix.push(stroke);
            loop {
                let contexts = self.window(window)?.ui.key_context_stack();
                let matched = self.keymap.bindings_for_input(&prefix, &contexts);
                if matched.pending {
                    break;
                }
                if !matched.bindings.is_empty() {
                    let fallback = prefix.last().cloned();
                    let consumed = self.dispatch_bindings(window, &matched.bindings)?;
                    if !consumed && let Some(fallback) = fallback {
                        self.handle_keystroke_fallback(window, fallback)?;
                    }
                    self.run_until_idle()?;
                    prefix.clear();
                    break;
                }
                let replay = prefix.remove(0);
                self.handle_keystroke_fallback(window, replay)?;
                self.run_until_idle()?;
                if prefix.is_empty() {
                    break;
                }
            }
        }
        if !prefix.is_empty() {
            let contexts = self.window(window)?.ui.key_context_stack();
            let matched = self.keymap.bindings_for_input(&prefix, &contexts);
            if matched.bindings.is_empty() {
                return Err(TestAppError::IncompleteKeystrokeSequence);
            }
            let fallback = prefix.last().cloned();
            let consumed = self.dispatch_bindings(window, &matched.bindings)?;
            if !consumed && let Some(fallback) = fallback {
                self.handle_keystroke_fallback(window, fallback)?;
            }
            self.run_until_idle()?;
        }
        self.run_until_idle()
    }
}
