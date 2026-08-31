use super::*;

impl Element {
    pub fn hover(mut self, style: impl FnOnce(ElementStateStyle) -> ElementStateStyle) -> Self {
        self.hover = style(ElementStateStyle::default());
        self
    }

    pub fn active(mut self, style: impl FnOnce(ElementStateStyle) -> ElementStateStyle) -> Self {
        self.active = style(ElementStateStyle::default());
        self
    }

    /// Smooth supported paint-only style changes under one retained element identity.
    ///
    /// A [`Duration`](std::time::Duration) converts to an all-property [`Transition`]. Layout
    /// values use [`crate::AnimationExt`] instead because they require a declaration rebuild and
    /// Taffy layout.
    pub fn transition(mut self, transition: impl Into<Transition>) -> Self {
        self.transition = Some(transition.into());
        self
    }

    /// Paint-only styling while this element owns keyboard focus.
    pub fn focus(mut self, style: impl FnOnce(ElementStateStyle) -> ElementStateStyle) -> Self {
        self.focus = style(ElementStateStyle::default());
        self
    }

    /// Paint-only styling while this element is the source of an active internal drag.
    pub fn dragging(mut self, style: impl FnOnce(ElementStateStyle) -> ElementStateStyle) -> Self {
        self.dragging = style(ElementStateStyle::default());
        self
    }

    /// Paint-only styling while a compatible typed payload is over this drop target.
    pub fn drag_over(mut self, style: impl FnOnce(ElementStateStyle) -> ElementStateStyle) -> Self {
        self.drag_over = style(ElementStateStyle::default());
        self
    }

    pub fn accessibility_role(mut self, role: AccessibilityRole) -> Self {
        self.accessibility.role = role;
        self
    }

    pub fn accessibility_label(mut self, label: impl Into<Arc<str>>) -> Self {
        self.accessibility.label = Some(label.into());
        self
    }

    pub fn accessibility_value(mut self, value: impl Into<Arc<str>>) -> Self {
        self.accessibility.value = Some(value.into());
        self
    }

    /// Set placeholder text for a [`text_input`] element.
    pub fn placeholder(mut self, placeholder: impl Into<Arc<str>>) -> Self {
        if let ElementKind::TextInput(input) = &mut self.kind {
            input.placeholder = placeholder.into();
        }
        self
    }

    /// Mask the visible value and expose native secure-text-field semantics.
    ///
    /// Password inputs retain their real controlled value for editing and submit listeners, but
    /// paint one bullet per Unicode grapheme and do not expose selections to clipboard actions.
    /// Calling this with `false` restores an ordinary single-line text input, which supports
    /// web-style reveal buttons without replacing the retained input state.
    pub fn password(mut self, password: bool) -> Self {
        let ElementKind::TextInput(input) = &mut self.kind else {
            panic!("password can only be applied to a text input");
        };
        assert!(
            !password || !input.multiline,
            "a text area cannot be a password input"
        );
        input.password = password;
        self.accessibility.role = if password {
            AccessibilityRole::PasswordInput
        } else {
            AccessibilityRole::TextInput
        };
        self
    }

    /// Limit user edits to at most this many Unicode grapheme clusters.
    ///
    /// Pasted and committed IME text is truncated at a grapheme boundary before the input filter
    /// runs. Controlled values supplied by the application remain authoritative and are not
    /// rewritten during rendering.
    pub fn max_length(mut self, length: usize) -> Self {
        let ElementKind::TextInput(input) = &mut self.kind else {
            panic!("max_length can only be applied to a text input or text area");
        };
        input.constraints.max_length = Some(length);
        self
    }

    /// Accept or reject a proposed complete value before retained text and history are mutated.
    ///
    /// The callback runs only for edit attempts—not during paint, layout, pointer movement, or
    /// controlled-value synchronization. Returning `false` rejects typing, paste, IME commit,
    /// accessibility value changes, and undo/redo consistently.
    pub fn input_filter(mut self, filter: impl Fn(&str) -> bool + 'static) -> Self {
        let ElementKind::TextInput(input) = &mut self.kind else {
            panic!("input_filter can only be applied to a text input or text area");
        };
        assert!(
            input.constraints.filter.is_none(),
            "input_filter was registered more than once on one element"
        );
        input.constraints.filter = Some(Arc::new(filter));
        self
    }

    /// Expose that a form control requires a value before submission.
    ///
    /// This projects the native accessibility state only. The application remains responsible for
    /// deriving [`Self::invalid`] and its validation message from the controlled value.
    pub fn required(mut self, required: bool) -> Self {
        self.accessibility.required = required;
        self
    }

    /// Expose web-style invalid state to paint and the native accessibility tree.
    pub fn invalid(mut self, invalid: bool) -> Self {
        self.accessibility.invalid = invalid;
        self
    }

    /// Describe why an invalid control cannot currently be submitted.
    ///
    /// Call [`Self::invalid`] separately so clearing or replacing a message never changes validity
    /// accidentally.
    pub fn validation_message(mut self, message: impl Into<Arc<str>>) -> Self {
        let (message, truncated) = bounded_validation_message(message.into());
        self.accessibility.validation_message = message;
        self.accessibility.validation_message_truncated = truncated;
        self
    }

    pub(crate) fn validation_message_retained(
        mut self,
        message: Arc<str>,
        truncated: bool,
    ) -> Self {
        debug_assert!(message.len() <= MAX_VALIDATION_MESSAGE_BYTES);
        self.accessibility.validation_message = (!message.is_empty()).then_some(message);
        self.accessibility.validation_message_truncated = truncated;
        self
    }

    /// Set a native accessibility description independently of visible text.
    pub fn accessibility_description(mut self, description: impl Into<Arc<str>>) -> Self {
        let description = description.into();
        self.accessibility.description = (!description.is_empty()).then_some(description);
        self
    }

    /// Paint-only styling while this element is marked invalid.
    pub fn invalid_style(
        mut self,
        style: impl FnOnce(ElementStateStyle) -> ElementStateStyle,
    ) -> Self {
        self.invalid_style = style(ElementStateStyle::default());
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.accessibility.disabled = disabled;
        self
    }

    /// Paint-only styling while this element is disabled.
    ///
    /// Disabled elements are already removed from pointer, keyboard, and accessibility actions;
    /// this method adds an optional visual treatment without changing layout.
    pub fn disabled_style(
        mut self,
        style: impl FnOnce(ElementStateStyle) -> ElementStateStyle,
    ) -> Self {
        self.disabled_style = style(ElementStateStyle::default());
        self
    }

    pub fn selected(mut self, selected: bool) -> Self {
        self.accessibility.selected = selected;
        self
    }

    /// Hide this element and its complete subtree from the native accessibility tree.
    ///
    /// Painting, layout, pointer input, and keyboard behavior are unchanged. This is useful for
    /// a visual surface whose semantics are projected into another native window, such as a
    /// never-key autocomplete panel controlled by an owner-window text input.
    pub fn accessibility_hidden(mut self, hidden: bool) -> Self {
        self.accessibility.hidden = hidden;
        self
    }

    /// Expose whether a disclosure, popover, or similar controlled surface is expanded.
    pub fn accessibility_expanded(mut self, expanded: bool) -> Self {
        self.accessibility.expanded = Some(expanded);
        self
    }

    /// Relate this control to one stable element that it controls.
    ///
    /// The relationship is projected only when the target is present in the mounted
    /// accessibility tree. This avoids dangling native node references for closed controlled
    /// popovers whose content is intentionally unmounted.
    pub fn accessibility_controls(mut self, target: impl Into<ElementId>) -> Self {
        self.accessibility.relations.set_controls(target.into());
        self
    }

    /// Identify the mounted active option, row, cell, or tree item for a composite control.
    ///
    /// Keep DOM-style keyboard focus on the composite root and update this relationship as its
    /// controlled selection moves. The native relationship is omitted while the target is not
    /// mounted, which is important for virtualized collections.
    pub fn accessibility_active_descendant(mut self, target: impl Into<ElementId>) -> Self {
        self.accessibility
            .relations
            .set_active_descendant(target.into());
        self
    }

    /// Use one mounted element as this element's accessible label.
    ///
    /// The relationship is projected only while the target is present and is omitted for a
    /// self-reference. Prefer this to copying visible group, field, or section text into a second
    /// accessibility-only string.
    pub fn accessibility_labelled_by(mut self, target: impl Into<ElementId>) -> Self {
        self.accessibility.relations.set_labelled_by(target.into());
        self
    }

    /// Use one mounted element as this element's accessible description.
    ///
    /// Dangling and self-referential relationships are omitted from the native tree, matching
    /// [`Self::accessibility_labelled_by`].
    pub fn accessibility_described_by(mut self, target: impl Into<ElementId>) -> Self {
        self.accessibility.relations.set_described_by(target.into());
        self
    }

    pub(crate) fn accessibility_described_by_pair(
        mut self,
        first: impl Into<ElementId>,
        second: impl Into<ElementId>,
    ) -> Self {
        self.accessibility
            .relations
            .set_described_by_pair(first.into(), second.into());
        self
    }

    /// Describe the kind of popover opened by this control.
    pub fn accessibility_has_popover(mut self, popover: AccessibilityPopover) -> Self {
        self.accessibility.has_popover = Some(popover);
        self
    }

    /// Describe how an editable control presents completion suggestions.
    pub fn accessibility_auto_complete(mut self, behavior: AccessibilityAutoComplete) -> Self {
        self.accessibility.auto_complete = Some(behavior);
        self
    }

    /// Mark a dialog or alert-dialog as explicitly modal for assistive technology.
    pub fn accessibility_modal(mut self, modal: bool) -> Self {
        self.accessibility.modal = modal;
        self
    }

    /// Expose the complete logical row count for a table or grid, including unmounted rows.
    pub fn accessibility_row_count(mut self, count: usize) -> Self {
        self.accessibility.collection.row_count = accessibility_collection_storage(count);
        self
    }

    /// Expose the complete logical column count for a table or grid.
    pub fn accessibility_column_count(mut self, count: usize) -> Self {
        self.accessibility.collection.column_count = accessibility_collection_storage(count);
        self
    }

    /// Set the zero-based logical row index for a row or cell.
    pub fn accessibility_row_index(mut self, index: usize) -> Self {
        self.accessibility.collection.row_index = accessibility_collection_storage(index);
        self
    }

    /// Set the zero-based logical column index for a header or cell.
    pub fn accessibility_column_index(mut self, index: usize) -> Self {
        self.accessibility.collection.column_index = accessibility_collection_storage(index);
        self
    }

    /// Set the zero-based nesting level for a hierarchical item.
    pub fn accessibility_level(mut self, level: usize) -> Self {
        self.accessibility.collection.level = accessibility_collection_storage(level);
        self
    }

    /// Expose the total logical sibling count for a virtualized collection item.
    pub fn accessibility_size_of_set(mut self, size: usize) -> Self {
        self.accessibility.collection.size_of_set = accessibility_collection_storage(size);
        self
    }

    /// Set the zero-based logical position among an item's siblings.
    pub fn accessibility_position_in_set(mut self, position: usize) -> Self {
        self.accessibility.collection.position_in_set = accessibility_collection_storage(position);
        self
    }

    /// Expose the active ordering of a sortable table or grid column.
    pub fn accessibility_sort_direction(mut self, direction: AccessibilitySortDirection) -> Self {
        self.accessibility.collection.sort_direction = Some(direction);
        self
    }

    /// Expose a controlled checked, unchecked, or mixed state to assistive technology.
    pub fn toggle_state(mut self, state: impl Into<ToggleState>) -> Self {
        self.accessibility.toggled = Some(state.into());
        self
    }

    /// Web-style boolean shorthand for [`Self::toggle_state`].
    pub fn checked(self, checked: bool) -> Self {
        self.toggle_state(checked)
    }

    /// Set or clear the web-style indeterminate checkbox state.
    ///
    /// Clearing indeterminate preserves an existing on/off state and maps a previously mixed
    /// state to off.
    pub fn indeterminate(mut self, indeterminate: bool) -> Self {
        self.accessibility.toggled = Some(if indeterminate {
            ToggleState::Mixed
        } else {
            match self.accessibility.toggled {
                Some(ToggleState::On) => ToggleState::On,
                _ => ToggleState::Off,
            }
        });
        self
    }

    /// Include this element in the window's focus path and Tab traversal.
    pub fn focusable(mut self) -> Self {
        self.focusable = true;
        self
    }

    /// Control whether a pointer press moves keyboard focus to this focusable element.
    ///
    /// Disabling pointer focus does not remove the element from Tab traversal or accessibility
    /// focus. This is useful for native-style sidebars and toolbars whose controls should activate
    /// without taking keyboard ownership from an editor or terminal.
    pub fn focus_on_pointer(mut self, focus: bool) -> Self {
        self.focus_on_pointer = focus;
        self
    }

    /// Expand this element's pointer hit region without changing its layout or paint bounds.
    ///
    /// The expanded region remains clipped by the element's parent. This is useful for thin native
    /// affordances such as split-view dividers and resize handles that need a forgiving target
    /// without a visible gutter.
    pub fn hit_slop(mut self, insets: Insets) -> Self {
        self.hit_slop = Insets {
            top: finite_nonnegative(insets.top),
            right: finite_nonnegative(insets.right),
            bottom: finite_nonnegative(insets.bottom),
            left: finite_nonnegative(insets.left),
        };
        self
    }

    /// Assign a stable focus identity to this element.
    pub fn track_focus(mut self, handle: FocusHandle) -> Self {
        self.bind_listener_id(handle.id());
        self.focusable = true;
        self
    }

    /// Give a non-focusable ancestor a stable identity for scoped action dispatch.
    ///
    /// Descendant focus is still tracked through this element, but the scope itself is not added to
    /// Tab traversal.
    pub fn focus_scope(mut self, handle: FocusHandle) -> Self {
        self.bind_listener_id(handle.id());
        self
    }

    /// Contain keyboard focus within this subtree while it is mounted and topmost.
    ///
    /// Nested traps are resolved by overlay plane, `z_index`, and declaration order. Only the
    /// topmost trap participates in Tab traversal or programmatic framework focus; mounting one
    /// moves focus to its first enabled Tab stop (or the trap root when it is focusable), and
    /// removing a focused child keeps focus inside the remaining trap. The marker retains no
    /// observer, timer, task, or idle scheduler source.
    pub fn focus_trap(mut self) -> Self {
        self.focus_trap = true;
        self
    }

    /// Restore the element that owned focus before this surface mounted when it unmounts.
    ///
    /// The retained UI tree captures the target once per stable surface identity, so rebuilds do
    /// not replace it with a descendant focus target. Nested surfaces restore in stack order.
    pub fn restore_previous_focus(mut self) -> Self {
        self.restore_previous_focus = true;
        self
    }

    /// Attach contextual keymap properties to this node in the focused ancestor path.
    pub fn key_context(mut self, context: impl Into<KeyContext>) -> Self {
        self.key_context = Some(context.into());
        self
    }

    /// Set keyboard traversal order. Negative values remove the element from Tab traversal.
    pub fn tab_index(mut self, index: i16) -> Self {
        self.tab_index = index;
        self
    }

    /// Prefer this element for initial window focus or when a newly mounted focus trap takes focus.
    pub fn auto_focus(mut self) -> Self {
        self.auto_focus = true;
        self.focusable = true;
        self
    }

    /// Include this node in click hit testing. Clicks arrive as [`crate::Event::Click`].
    pub fn clickable(mut self) -> Self {
        self.clickable = true;
        self.set_implicit_cursor(CursorStyle::PointingHand);
        self.focusable = true;
        if self.accessibility.role == AccessibilityRole::GenericContainer {
            self.accessibility.role = AccessibilityRole::Button;
        }
        self
    }
}
