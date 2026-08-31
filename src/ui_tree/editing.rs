use super::*;

impl UiTree {
    pub fn focused_text_input(&self) -> Option<ElementId> {
        self.focused
            .filter(|focused| self.text_inputs.contains_key(focused))
    }

    pub(crate) fn focused_text_input_is_multiline(&self) -> bool {
        self.focused_text_input()
            .and_then(|id| self.text_inputs.get(&id))
            .is_some_and(TextInputState::is_multiline)
    }

    pub(crate) fn focused_text_input_is_invalid(&self) -> bool {
        self.focused_text_input()
            .is_some_and(|id| self.invalid_ids.contains(&id))
    }

    pub(crate) fn focused_text_input_value(&self) -> Option<Arc<str>> {
        self.focused_text_input()
            .and_then(|id| self.text_inputs.get(&id))
            .map(TextInputState::committed_shared_text)
    }

    /// Find the nearest semantic form containing a control.
    pub(crate) fn form_for_control(&self, mut id: ElementId) -> Option<ElementId> {
        loop {
            if self.form_ids.contains(&id) {
                return Some(id);
            }
            id = self.parents.get(&id).copied()?;
        }
    }

    /// Return the nearest form only when this element opted into submit-button behavior.
    pub(crate) fn form_for_submitter(&self, id: ElementId) -> Option<ElementId> {
        self.form_submitter_ids
            .contains(&id)
            .then(|| self.form_for_control(id))
            .flatten()
    }

    pub(crate) fn activation_target(&self, id: ElementId) -> Option<ElementId> {
        self.activation_targets.get(&id).copied()
    }

    /// Validate one mounted form, retaining only bounded shared values and issue metadata.
    ///
    /// Invalid attempts synchronously focus the first focusable invalid control in document order
    /// and replace the one retained AccessKit live announcement. No timer or idle frame is added.
    pub(crate) fn attempt_form_submission(
        &mut self,
        form: ElementId,
        trigger: Option<ElementId>,
    ) -> Option<FormAttempt> {
        if !self.form_ids.contains(&form) {
            return None;
        }
        let root = self.root.as_ref()?;
        let form_element = find_element(root, form)?;
        let mut collection = FormCollection::default();
        for child in &form_element.children {
            collect_form_controls(
                child,
                &self.text_inputs,
                &self.focusable_ids,
                &mut collection,
            );
        }

        if collection.issues.is_empty() {
            self.validation_announcement = None;
            return Some(FormAttempt::Valid(FormSubmitEvent::new(
                form,
                trigger,
                collection.fields,
                collection.fields_truncated,
            )));
        }

        if let Some(first_focusable) = collection.first_focusable_invalid {
            self.focus(first_focusable);
        }
        let report = ValidationReport::new(
            form,
            trigger,
            collection.issues,
            collection.issues_truncated,
        );
        self.set_validation_announcement(&report);
        Some(FormAttempt::Invalid(report))
    }

    pub(super) fn set_validation_announcement(&mut self, report: &ValidationReport) {
        let message = validation_announcement_message(report);
        let node = self.next_auxiliary_accessibility_id();
        self.validation_announcement = Some(ValidationAnnouncement {
            form: report.form(),
            node,
            message,
        });
    }

    pub(super) fn next_auxiliary_accessibility_id(&mut self) -> AccessibilityNodeId {
        loop {
            let candidate = AccessibilityNodeId(self.next_accessibility_text_id);
            self.next_accessibility_text_id = self.next_accessibility_text_id.wrapping_sub(1);
            if candidate != ACCESSIBILITY_ROOT_ID
                && !self.seen_ids.contains(&ElementId::new(candidate.0))
                && !self
                    .accessibility_text_ids
                    .values()
                    .any(|existing| *existing == candidate)
                && self
                    .validation_announcement
                    .as_ref()
                    .is_none_or(|announcement| announcement.node != candidate)
            {
                return candidate;
            }
        }
    }

    pub(crate) fn selected_input_text(&self) -> Option<Arc<str>> {
        let id = self.focused_text_input()?;
        if self
            .root
            .as_ref()
            .and_then(|root| find_element(root, id))
            .is_some_and(|element| {
                matches!(
                    &element.kind,
                    ElementKind::TextInput(input) if input.password
                )
            })
        {
            return None;
        }
        self.text_inputs
            .get(&id)
            .and_then(TextInputState::selected_text)
            .map(Arc::from)
    }

    pub fn selected_text(&self) -> Option<Arc<str>> {
        self.selected_input_text()
            .or_else(|| self.selected_static_text())
    }

    pub(crate) fn has_selectable_text(&self) -> bool {
        !self.selectable_texts.is_empty()
    }

    pub(crate) fn select_all_static_text(&mut self) -> bool {
        let (Some(first), Some(last)) =
            (self.selectable_texts.first(), self.selectable_texts.last())
        else {
            return false;
        };
        let next = StaticTextSelection {
            anchor: StaticTextPosition {
                id: first.id,
                offset: 0,
            },
            focus: StaticTextPosition {
                id: last.id,
                offset: last.content.len(),
            },
        };
        let changed = self.static_text_selection != Some(next);
        self.static_text_selection = Some(next);
        self.static_text_gesture = None;
        changed
    }

    pub(crate) fn clear_static_text_selection(&mut self) -> bool {
        self.static_text_gesture = None;
        self.static_text_selection.take().is_some()
    }

    pub(super) fn selected_static_text(&self) -> Option<Arc<str>> {
        let selection = self.static_text_selection?;
        let (start, end) = normalized_static_selection(selection, &self.selectable_text_indices)?;
        if start == end {
            return None;
        }
        let mut output = String::new();
        for document_index in start.0..=end.0 {
            let entry = self.selectable_texts.get(document_index)?;
            let range = if start.0 == end.0 {
                start.1.min(entry.content.len())..end.1.min(entry.content.len())
            } else if document_index == start.0 {
                start.1.min(entry.content.len())..entry.content.len()
            } else if document_index == end.0 {
                0..end.1.min(entry.content.len())
            } else {
                0..entry.content.len()
            };
            if document_index > start.0 {
                let previous = &self.selectable_texts[document_index - 1];
                let separator = selection_separator(
                    self.element_bounds.get(&previous.id).copied(),
                    self.element_bounds.get(&entry.id).copied(),
                );
                push_bounded_text(&mut output, separator, MAX_STATIC_TEXT_COPY_BYTES);
            }
            push_bounded_text(
                &mut output,
                &entry.content[range],
                MAX_STATIC_TEXT_COPY_BYTES,
            );
            if output.len() == MAX_STATIC_TEXT_COPY_BYTES {
                break;
            }
        }
        (!output.is_empty()).then(|| Arc::from(output))
    }

    pub fn input_move_left(&mut self, extend: bool) -> InputResult {
        self.edit_focused_input(|state| state.move_left(extend))
    }

    pub fn input_move_right(&mut self, extend: bool) -> InputResult {
        self.edit_focused_input(|state| state.move_right(extend))
    }

    pub fn input_move_word_left(&mut self, extend: bool) -> InputResult {
        self.edit_focused_input(|state| state.move_word_left(extend))
    }

    pub fn input_move_word_right(&mut self, extend: bool) -> InputResult {
        self.edit_focused_input(|state| state.move_word_right(extend))
    }

    pub fn input_move_home(&mut self, extend: bool) -> InputResult {
        self.edit_focused_input(|state| state.move_home(extend))
    }

    pub fn input_move_end(&mut self, extend: bool) -> InputResult {
        self.edit_focused_input(|state| state.move_end(extend))
    }

    pub fn input_move_line_start(&mut self, extend: bool) -> InputResult {
        self.edit_focused_input(|state| state.move_line_start(extend))
    }

    pub fn input_move_line_end(&mut self, extend: bool) -> InputResult {
        self.edit_focused_input(|state| state.move_line_end(extend))
    }

    pub fn input_move_vertical(
        &mut self,
        lines: isize,
        extend: bool,
        renderer: &mut impl TextLayoutEngine,
    ) -> InputResult {
        if lines == 0 {
            return InputResult::default();
        }
        let Some(id) = self.focused_text_input() else {
            return InputResult::default();
        };
        let Some(region) = self
            .text_input_regions
            .iter()
            .rev()
            .find(|region| region.id == id)
            .cloned()
        else {
            return InputResult::default();
        };
        let Some(state) = self.text_inputs.get_mut(&id) else {
            return InputResult::default();
        };
        if !state.is_multiline() {
            return InputResult::default();
        }

        let mut repaint = false;
        let selection = state.selection();
        if !extend && !selection.is_empty() {
            let collapse = if lines < 0 {
                selection.start
            } else {
                selection.end
            };
            repaint |= state.move_to(collapse, false);
        }

        let content = state.shared_text();
        let text_id = TextId::new(id.value());
        let caret = renderer.text_caret_position_with_highlights(
            text_id,
            &content,
            &region.style,
            region.highlights.as_ref(),
            region.bounds.width,
            self.scale_factor,
            state.caret(),
        );
        let preferred_x = state.preferred_x().unwrap_or(caret.x);
        let target = Point::new(
            preferred_x,
            caret.y + lines as f32 * region.style.line_height + region.style.line_height * 0.5,
        );
        let index = renderer.text_index_for_point_with_highlights(
            text_id,
            &content,
            &region.style,
            region.highlights.as_ref(),
            region.bounds.width,
            self.scale_factor,
            target,
        );
        repaint |= state.move_to_with_preferred_x(index, extend, preferred_x);
        InputResult {
            repaint,
            change: None,
        }
    }

    pub fn input_move_page(
        &mut self,
        direction: isize,
        extend: bool,
        renderer: &mut impl TextLayoutEngine,
    ) -> InputResult {
        let Some(id) = self.focused_text_input() else {
            return InputResult::default();
        };
        let Some(region) = self
            .text_input_regions
            .iter()
            .rev()
            .find(|region| region.id == id)
        else {
            return InputResult::default();
        };
        let lines = (region.bounds.height / region.style.line_height)
            .floor()
            .max(1.0) as isize;
        self.input_move_vertical(direction.signum() * lines, extend, renderer)
    }

    pub fn input_select_all(&mut self) -> InputResult {
        self.edit_focused_input(TextInputState::select_all)
    }

    pub fn input_backspace(&mut self) -> InputResult {
        self.edit_focused_input(TextInputState::backspace)
    }

    pub fn input_delete(&mut self) -> InputResult {
        self.edit_focused_input(TextInputState::delete)
    }

    pub fn input_delete_word_backward(&mut self) -> InputResult {
        self.edit_focused_input(TextInputState::delete_word_backward)
    }

    pub fn input_delete_word_forward(&mut self) -> InputResult {
        self.edit_focused_input(TextInputState::delete_word_forward)
    }

    pub fn input_delete_to_line_start(&mut self) -> InputResult {
        self.edit_focused_input(TextInputState::delete_to_line_start)
    }

    pub fn input_delete_to_line_end(&mut self) -> InputResult {
        self.edit_focused_input(TextInputState::delete_to_line_end)
    }

    pub fn input_replace(&mut self, value: &str) -> InputResult {
        self.edit_focused_input(|state| state.replace_selection(value))
    }

    pub fn input_insert_newline(&mut self) -> InputResult {
        self.edit_focused_input(TextInputState::insert_newline)
    }

    pub fn input_can_undo(&self) -> bool {
        self.focused_text_input()
            .and_then(|id| self.text_inputs.get(&id))
            .is_some_and(TextInputState::can_undo)
    }

    pub fn input_can_redo(&self) -> bool {
        self.focused_text_input()
            .and_then(|id| self.text_inputs.get(&id))
            .is_some_and(TextInputState::can_redo)
    }

    pub fn input_undo(&mut self) -> InputResult {
        self.edit_focused_input(TextInputState::undo)
    }

    pub fn input_redo(&mut self) -> InputResult {
        self.edit_focused_input(TextInputState::redo)
    }

    pub fn input_preedit(&mut self, value: &str, cursor: Option<(usize, usize)>) -> InputResult {
        self.edit_focused_input(|state| state.set_preedit(value, cursor))
    }

    pub fn input_cancel_preedit(&mut self, id: ElementId) -> InputResult {
        self.edit_input(id, |state| state.set_preedit("", None))
    }

    pub fn input_set_value(&mut self, id: ElementId, value: &str) -> InputResult {
        self.edit_input(id, |state| state.set_value(value))
    }

    pub fn set_accessibility_text_selection(
        &mut self,
        id: ElementId,
        selection: &TextSelection,
    ) -> InputResult {
        let Some(text_id) = self.accessibility_text_ids.get(&id).copied() else {
            return InputResult::default();
        };
        if selection.anchor.node != text_id || selection.focus.node != text_id {
            return InputResult::default();
        }
        if self.text_inputs.contains_key(&id) {
            return self.edit_input(id, |state| {
                let anchor = state.accessibility_byte_index(selection.anchor.character_index);
                let caret = state.accessibility_byte_index(selection.focus.character_index);
                state.set_selection(anchor, caret)
            });
        }
        let Some(document_index) = self.selectable_text_indices.get(&id).copied() else {
            return InputResult::default();
        };
        let content = &self.selectable_texts[document_index].content;
        let next = StaticTextSelection {
            anchor: StaticTextPosition {
                id,
                offset: accessibility_byte_index(content, selection.anchor.character_index),
            },
            focus: StaticTextPosition {
                id,
                offset: accessibility_byte_index(content, selection.focus.character_index),
            },
        };
        let repaint = self.static_text_selection != Some(next);
        self.static_text_selection = Some(next);
        self.static_text_gesture = None;
        InputResult {
            repaint,
            change: None,
        }
    }

    pub fn ime_cursor_area(&self) -> Option<Rect> {
        let focused = self.focused_text_input()?;
        self.text_input_regions
            .iter()
            .rev()
            .find(|region| region.id == focused)
            .and_then(|region| region.caret_bounds.intersection(region.clip))
    }

    pub(super) fn edit_focused_input(
        &mut self,
        edit: impl FnOnce(&mut TextInputState) -> bool,
    ) -> InputResult {
        let Some(id) = self.focused_text_input() else {
            return InputResult::default();
        };
        self.edit_input(id, edit)
    }

    pub(super) fn edit_input(
        &mut self,
        id: ElementId,
        edit: impl FnOnce(&mut TextInputState) -> bool,
    ) -> InputResult {
        let Some(state) = self.text_inputs.get_mut(&id) else {
            return InputResult::default();
        };
        let previous = state.committed_shared_text();
        let repaint = edit(state);
        let committed = state.committed_shared_text();
        let change = (previous != committed).then_some(InputChange {
            id,
            value: committed,
        });
        InputResult { repaint, change }
    }
}
