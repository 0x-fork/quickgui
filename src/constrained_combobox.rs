use std::{fmt, ops::Range, sync::Arc};

use crate::{
    AutocompleteListState, AutocompleteOptionState, AutocompletePopupLayout,
    AutocompleteSelectionBehavior, AutocompleteState, Element, ElementId, EventContext,
    PickerError, PickerFilterMode, PickerItem, ViewContext, WindowHandle,
    autocomplete::AutocompleteAccess,
};

/// Maximum option rows mounted by one constrained combobox popup.
pub const MAX_COMBOBOX_VISIBLE_ROWS: usize = crate::MAX_AUTOCOMPLETE_VISIBLE_ROWS;

/// Structural geometry for the constrained combobox's separate native suggestion surface.
///
/// This is the same geometry contract used by free-form autocomplete. It contains no color,
/// typography, border, radius, shadow, icon, or animation tokens.
pub type ComboboxPopupLayout = AutocompletePopupLayout;

/// State supplied to the caller-owned suggestion-surface renderer.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ComboboxListState {
    pub result_count: usize,
    pub total_match_count: usize,
    pub active_index: Option<usize>,
    pub results_truncated: bool,
}

/// State supplied to one caller-owned constrained option renderer.
#[derive(Clone, Debug, PartialEq)]
pub struct ComboboxOptionState {
    pub result_index: usize,
    pub source_index: usize,
    pub active: bool,
    pub selected: bool,
    pub disabled: bool,
    pub label_ranges: Arc<[Range<usize>]>,
}

#[derive(Clone)]
struct CommittedSelection<T> {
    stable_id: Option<ElementId>,
    source_index: Option<usize>,
    label: Arc<str>,
    value: T,
}

#[derive(Clone, Copy)]
struct SelectionMarker {
    stable_id: Option<ElementId>,
    source_index: Option<usize>,
}

impl SelectionMarker {
    fn matches<T>(self, item: &PickerItem<T>, source_index: usize) -> bool {
        self.stable_id
            .map_or(self.source_index == Some(source_index), |id| {
                item.stable_id() == Some(id)
            })
    }
}

/// Controlled, editable, single-value combobox constrained to declared items.
///
/// The application owns the input, popup-root, and option elements. QuickGUI owns bounded
/// matching, keyboard navigation, a never-key overflow-capable native child, exact dismissal,
/// committed-value restoration, pointer selection, owner-tree accessibility proxies, and
/// visible-only mounting. Unlike [`crate::AutocompleteState`], arbitrary text is an editing query,
/// not a committable value. Unlike [`crate::SelectState`], the owner control is an editable input.
/// Closed state owns no native window, task, timer, observer, renderer, or scheduler source.
pub struct ComboboxState<T> {
    autocomplete: AutocompleteState<T>,
    selection: Option<CommittedSelection<T>>,
    query: Arc<str>,
}

impl<T> fmt::Debug for ComboboxState<T> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ComboboxState")
            .field("autocomplete", &self.autocomplete)
            .field("query", &self.query)
            .field(
                "selected_source",
                &self
                    .selection
                    .as_ref()
                    .and_then(|selection| selection.source_index),
            )
            .field(
                "selected_id",
                &self
                    .selection
                    .as_ref()
                    .and_then(|selection| selection.stable_id),
            )
            .field(
                "selected_label",
                &self.selection.as_ref().map(|selection| &selection.label),
            )
            .finish_non_exhaustive()
    }
}

impl<T> ComboboxState<T>
where
    T: Clone,
{
    pub fn new(items: impl IntoIterator<Item = PickerItem<T>>) -> Result<Self, PickerError> {
        let mut autocomplete = AutocompleteState::new(items)?;
        autocomplete.set_selection_behavior(AutocompleteSelectionBehavior::DismissOnly);
        Ok(Self {
            autocomplete,
            selection: None,
            query: Arc::from(""),
        })
    }

    pub fn with_layout(mut self, layout: ComboboxPopupLayout) -> Self {
        self.autocomplete = self.autocomplete.with_layout(layout);
        self
    }

    pub fn with_filter_mode(mut self, mode: PickerFilterMode) -> Self {
        self.autocomplete = self.autocomplete.with_filter_mode(mode);
        self
    }

    /// Set an initial selection before a runtime context exists.
    pub fn with_selected_source(mut self, source_index: usize) -> Self {
        self.set_initial_selection(source_index);
        self
    }

    /// Set an initial stable-ID selection before a runtime context exists.
    pub fn with_selected_id(mut self, id: impl Into<ElementId>) -> Self {
        let id = id.into();
        if let Some(source_index) = self
            .autocomplete
            .items()
            .iter()
            .position(|item| item.stable_id() == Some(id))
        {
            self.set_initial_selection(source_index);
        }
        self
    }

    pub const fn layout(&self) -> ComboboxPopupLayout {
        self.autocomplete.layout()
    }

    /// Replace popup geometry and synchronously dismiss an obsolete fixed-size child.
    pub fn set_layout(&mut self, layout: ComboboxPopupLayout, cx: &mut EventContext) -> bool {
        if !self.autocomplete.set_layout(layout, cx) {
            return false;
        }
        self.restore_committed(cx);
        true
    }

    pub fn items(&self) -> &[PickerItem<T>] {
        self.autocomplete.items()
    }

    /// Atomically replace the bounded suggestion source.
    ///
    /// A committed item with a stable ID is rebound across filtering or reordering. If it is
    /// temporarily absent, its cloned committed value and label remain authoritative while its
    /// current source index becomes `None`.
    pub fn set_items(
        &mut self,
        items: impl IntoIterator<Item = PickerItem<T>>,
        cx: &mut EventContext,
    ) -> Result<(), PickerError> {
        self.autocomplete.set_items(items, cx)?;
        self.rebind_selection();
        if !self.is_open() {
            self.restore_committed(cx);
        }
        Ok(())
    }

    pub const fn filter_mode(&self) -> PickerFilterMode {
        self.autocomplete.filter_mode()
    }

    pub fn set_filter_mode(&mut self, mode: PickerFilterMode, cx: &mut EventContext) -> bool {
        self.autocomplete.set_filter_mode(mode, cx)
    }

    pub fn query(&self) -> &Arc<str> {
        &self.query
    }

    /// The visible input value: an edit query while open, otherwise the committed label.
    pub fn input_value(&self) -> &Arc<str> {
        self.autocomplete.value()
    }

    pub fn selected_source_index(&self) -> Option<usize> {
        self.selection
            .as_ref()
            .and_then(|selection| selection.source_index)
    }

    pub fn selected_item(&self) -> Option<&PickerItem<T>> {
        let selection = self.selection.as_ref()?;
        let source_index = selection.source_index?;
        let item = self.autocomplete.items().get(source_index)?;
        selection.stable_id.map_or_else(
            || Some(item),
            |id| (item.stable_id() == Some(id)).then_some(item),
        )
    }

    pub fn selected_value(&self) -> Option<&T> {
        self.selection.as_ref().map(|selection| &selection.value)
    }

    pub fn selected_label(&self) -> Option<&Arc<str>> {
        self.selection.as_ref().map(|selection| &selection.label)
    }

    /// Replace the committed selection and close any open suggestion child.
    pub fn set_selected_source(&mut self, source_index: usize, cx: &mut EventContext) -> bool {
        let Some(selection) = self.selection_from_source(source_index) else {
            return false;
        };
        if self.same_selection(&selection) {
            return false;
        }
        self.autocomplete.close(cx);
        self.selection = Some(selection);
        self.restore_committed(cx);
        true
    }

    pub fn set_selected_id(&mut self, id: impl Into<ElementId>, cx: &mut EventContext) -> bool {
        let id = id.into();
        self.autocomplete
            .items()
            .iter()
            .position(|item| item.stable_id() == Some(id))
            .is_some_and(|source_index| self.set_selected_source(source_index, cx))
    }

    pub fn clear_selection(&mut self, cx: &mut EventContext) -> bool {
        if self.selection.take().is_none() {
            return false;
        }
        self.autocomplete.close(cx);
        self.restore_committed(cx);
        true
    }

    pub const fn popup_window(&self) -> Option<WindowHandle> {
        self.autocomplete.popup_window()
    }

    pub const fn is_open(&self) -> bool {
        self.autocomplete.is_open()
    }

    pub const fn is_disabled(&self) -> bool {
        self.autocomplete.is_disabled()
    }

    pub fn set_disabled(&mut self, disabled: bool, cx: &mut EventContext) -> bool {
        if !self.autocomplete.set_disabled(disabled, cx) {
            return false;
        }
        if disabled {
            self.restore_committed(cx);
        }
        true
    }

    pub const fn is_invalid(&self) -> bool {
        self.autocomplete.is_invalid()
    }

    pub fn set_invalid(&mut self, invalid: bool) -> bool {
        self.autocomplete.set_invalid(invalid)
    }

    pub fn validation_message(&self) -> Option<&Arc<str>> {
        self.autocomplete.validation_message()
    }

    pub fn set_validation_message(&mut self, message: impl Into<Arc<str>>) -> bool {
        self.autocomplete.set_validation_message(message)
    }

    pub fn clear_validation_message(&mut self) -> bool {
        self.autocomplete.clear_validation_message()
    }

    pub fn result_count(&self) -> usize {
        self.autocomplete.result_count()
    }

    pub fn total_match_count(&self) -> usize {
        self.autocomplete.total_match_count()
    }

    pub fn active_result_index(&self) -> Option<usize> {
        self.autocomplete.active_result_index()
    }

    pub fn active_source_index(&self) -> Option<usize> {
        self.autocomplete.active_source_index()
    }

    pub fn close(&mut self, cx: &mut EventContext) -> bool {
        let closed = self.autocomplete.close(cx);
        let (restored, _) = self.restore_committed(cx);
        closed || restored
    }

    pub fn surface_id(id: impl Into<ElementId>) -> ElementId {
        AutocompleteState::<T>::surface_id(id)
    }

    pub fn option_id_for_source(
        &self,
        id: impl Into<ElementId>,
        source_index: usize,
    ) -> Option<ElementId> {
        self.autocomplete.option_id_for_source(id, source_index)
    }

    /// Build the complete unstyled constrained interaction from caller-owned parts.
    #[allow(clippy::too_many_arguments)]
    pub fn element<V, PopupRoot, RenderOption, QueryChanged, Change>(
        &mut self,
        cx: &mut ViewContext<'_, V>,
        id: impl Into<ElementId>,
        label: impl Into<Arc<str>>,
        access: fn(&mut V) -> &mut ComboboxState<T>,
        input: Element,
        popup_root: PopupRoot,
        render_option: RenderOption,
        query_changed: QueryChanged,
        change: Change,
    ) -> Element
    where
        V: 'static,
        T: 'static,
        PopupRoot: Fn(ComboboxListState) -> Element + Clone + 'static,
        RenderOption: Fn(&PickerItem<T>, ComboboxOptionState) -> Element + Clone + 'static,
        QueryChanged: Fn(&mut V, Arc<str>, &mut EventContext) + Clone + 'static,
        Change: Fn(&mut V, T, &mut EventContext) + Clone + 'static,
    {
        let id = id.into();
        let selected_source = self.selected_source_index();
        let selection_marker = self.selection.as_ref().map(|selection| SelectionMarker {
            stable_id: selection.stable_id,
            source_index: selection.source_index,
        });

        let list_root = move |state: AutocompleteListState| {
            popup_root(ComboboxListState {
                result_count: state.result_count,
                total_match_count: state.total_match_count,
                active_index: state.active_index,
                results_truncated: state.results_truncated,
            })
        };
        let option = move |item: &PickerItem<T>, state: AutocompleteOptionState| {
            render_option(
                item,
                ComboboxOptionState {
                    result_index: state.result_index,
                    source_index: state.source_index,
                    active: state.active,
                    selected: selection_marker
                        .is_some_and(|marker| marker.matches(item, state.source_index)),
                    disabled: state.disabled,
                    label_ranges: state.label_ranges,
                },
            )
        };
        let inner_access = AutocompleteAccess::new(access, combobox_autocomplete::<T>);

        let edit_query_changed = query_changed.clone();
        let edited = move |view: &mut V, _value: Arc<str>, cx: &mut EventContext| {
            let query = {
                let state = access(view);
                let query = state.autocomplete.picker_query().clone();
                if state.query == query {
                    return;
                }
                state.query = query.clone();
                query
            };
            edit_query_changed(view, query, cx);
        };

        let commit_query_changed = query_changed.clone();
        let committed =
            move |view: &mut V, source_index: usize, value: T, cx: &mut EventContext| {
                let query_changed = {
                    let state = access(view);
                    let Some(item) = state.autocomplete.items().get(source_index) else {
                        return;
                    };
                    state.selection = Some(CommittedSelection {
                        stable_id: item.stable_id(),
                        source_index: Some(source_index),
                        label: item.label().clone(),
                        value: value.clone(),
                    });
                    state.restore_committed(cx).1
                };
                if query_changed {
                    commit_query_changed(view, Arc::from(""), cx);
                }
                change(view, value, cx);
            };

        let dismiss_query_changed = query_changed;
        let dismissed = move |view: &mut V, cx: &mut EventContext| {
            let query_changed = access(view).restore_committed(cx).1;
            if query_changed {
                dismiss_query_changed(view, Arc::from(""), cx);
            }
        };

        self.autocomplete.element_with_source(
            cx,
            id,
            label,
            inner_access,
            input,
            list_root,
            option,
            edited,
            committed,
            dismissed,
            selected_source,
        )
    }

    fn set_initial_selection(&mut self, source_index: usize) -> bool {
        let Some(selection) = self.selection_from_source(source_index) else {
            return false;
        };
        if self.same_selection(&selection) {
            return false;
        }
        self.autocomplete.set_display_value(selection.label.clone());
        self.selection = Some(selection);
        true
    }

    fn selection_from_source(&self, source_index: usize) -> Option<CommittedSelection<T>> {
        let item = self
            .autocomplete
            .items()
            .get(source_index)
            .filter(|item| !item.is_disabled())?;
        Some(CommittedSelection {
            stable_id: item.stable_id(),
            source_index: Some(source_index),
            label: item.label().clone(),
            value: item.value().clone(),
        })
    }

    fn same_selection(&self, next: &CommittedSelection<T>) -> bool {
        self.selection.as_ref().is_some_and(|current| {
            if let Some(id) = next.stable_id {
                current.stable_id == Some(id)
            } else {
                current.stable_id.is_none() && current.source_index == next.source_index
            }
        })
    }

    fn rebind_selection(&mut self) {
        let Some(selection) = &mut self.selection else {
            return;
        };
        let source_index = selection.stable_id.and_then(|id| {
            self.autocomplete
                .items()
                .iter()
                .position(|item| item.stable_id() == Some(id))
        });
        let source_index = source_index.or_else(|| {
            selection
                .stable_id
                .is_none()
                .then_some(selection.source_index)
                .flatten()
                .filter(|index| *index < self.autocomplete.items().len())
        });
        selection.source_index = source_index;
        if let Some(item) = source_index.and_then(|index| self.autocomplete.items().get(index)) {
            selection.stable_id = item.stable_id();
            selection.label = item.label().clone();
            selection.value = item.value().clone();
        }
    }

    /// Restore the committed label and reset the edit query. Returns `(changed, query_changed)`.
    fn restore_committed(&mut self, cx: &mut EventContext) -> (bool, bool) {
        let query_changed = !self.query.is_empty();
        if query_changed {
            self.query = Arc::from("");
        }
        let value = self
            .selection
            .as_ref()
            .map(|selection| selection.label.clone())
            .unwrap_or_else(|| Arc::from(""));
        let value_changed = self.autocomplete.set_display_value(value);
        let filter_changed = self.autocomplete.set_query_only("", cx);
        (
            query_changed || value_changed || filter_changed,
            query_changed,
        )
    }
}

fn combobox_autocomplete<T>(state: &mut ComboboxState<T>) -> &mut AutocompleteState<T> {
    &mut state.autocomplete
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        App, Color, MouseDownEvent, View, button, combobox_key_bindings, div, text, text_input,
    };
    use std::{cell::Cell, rc::Rc};

    fn options() -> [PickerItem<&'static str>; 4] {
        [
            PickerItem::new("Apple", "apple").id("apple"),
            PickerItem::new("Disabled", "disabled")
                .id("disabled")
                .disabled(true),
            PickerItem::new("Apricot", "apricot").id("apricot"),
            PickerItem::new("Banana", "banana").id("banana"),
        ]
    }

    fn popup_root(state: ComboboxListState) -> Element {
        div()
            .bg(Color::BLACK)
            .child(text(format!("{} results", state.result_count)))
    }

    fn option_row(item: &PickerItem<&'static str>, state: ComboboxOptionState) -> Element {
        div()
            .opacity(if state.active { 1.0 } else { 0.8 })
            .child(text(item.label().clone()))
    }

    struct Owner {
        combobox: ComboboxState<&'static str>,
        queries: Vec<Arc<str>>,
        changes: Vec<&'static str>,
    }

    impl Owner {
        fn new() -> Self {
            Self {
                combobox: ComboboxState::new(options())
                    .unwrap()
                    .with_layout(ComboboxPopupLayout::new(220.0, 32.0).max_visible_rows(2))
                    .with_selected_id("banana"),
                queries: Vec::new(),
                changes: Vec::new(),
            }
        }

        fn combobox(view: &mut Self) -> &mut ComboboxState<&'static str> {
            &mut view.combobox
        }
    }

    impl View for Owner {
        fn render(&mut self, cx: &mut ViewContext<'_, Self>) -> impl crate::IntoElement {
            let input = text_input(self.combobox.input_value().clone())
                .w(220.0)
                .h(36.0);
            let combobox = self.combobox.element(
                cx,
                "fruit",
                "Fruit",
                Self::combobox,
                input,
                popup_root,
                option_row,
                |view, query, _cx| view.queries.push(query),
                |view, value, _cx| view.changes.push(value),
            );
            div().children([combobox, button().id("after-combobox").child("After")])
        }
    }

    #[test]
    fn arbitrary_edits_never_commit_and_every_dismissal_restores_the_committed_label() {
        let (mut cx, owner) = App::new(Owner::new())
            .bind_keys(combobox_key_bindings())
            .into_test_context()
            .unwrap();
        let window = owner.window_handle();
        assert_eq!(
            cx.read(owner, |view| view.combobox.input_value().clone())
                .unwrap(),
            Arc::from("Banana")
        );

        cx.focus(window, "fruit").unwrap();
        cx.simulate_keystrokes(window, "platform-a").unwrap();
        cx.simulate_input(window, "ap").unwrap();
        assert!(cx.read(owner, |view| view.combobox.is_open()).unwrap());
        assert_eq!(
            cx.read(owner, |view| view.combobox.query().clone())
                .unwrap(),
            Arc::from("ap")
        );
        cx.simulate_keystrokes(window, "escape").unwrap();
        assert!(!cx.read(owner, |view| view.combobox.is_open()).unwrap());
        assert_eq!(
            cx.read(owner, |view| view.combobox.input_value().clone())
                .unwrap(),
            Arc::from("Banana")
        );
        assert_eq!(
            cx.read(owner, |view| view.combobox.query().clone())
                .unwrap(),
            Arc::from("")
        );
        assert_eq!(
            cx.read(owner, |view| view.combobox.selected_value().copied())
                .unwrap(),
            Some("banana")
        );
        assert!(cx.read(owner, |view| view.changes.is_empty()).unwrap());

        cx.simulate_keystrokes(window, "platform-a").unwrap();
        cx.simulate_input(window, "no such option").unwrap();
        assert!(!cx.dispatch_action(window, crate::ComboboxConfirm).unwrap());
        assert!(cx.read(owner, |view| view.changes.is_empty()).unwrap());
        cx.simulate_keystrokes(window, "escape").unwrap();

        cx.simulate_keystrokes(window, "platform-a").unwrap();
        cx.simulate_input(window, "ap").unwrap();
        cx.simulate_keystrokes(window, "enter").unwrap();
        assert_eq!(
            cx.read(owner, |view| view.combobox.selected_value().copied())
                .unwrap(),
            Some("apple")
        );
        assert_eq!(
            cx.read(owner, |view| view.combobox.input_value().clone())
                .unwrap(),
            Arc::from("Apple")
        );
        assert_eq!(
            cx.read(owner, |view| view.changes.clone()).unwrap(),
            vec!["apple"]
        );
        assert_eq!(
            cx.read(owner, |view| view.queries.last().cloned()).unwrap(),
            Some(Arc::from(""))
        );
    }

    #[test]
    fn tab_and_owner_outside_press_restore_without_stealing_normal_focus_movement() {
        let (mut cx, owner) = App::new(Owner::new())
            .bind_keys(combobox_key_bindings())
            .into_test_context()
            .unwrap();
        let window = owner.window_handle();
        cx.focus(window, "fruit").unwrap();
        cx.simulate_keystrokes(window, "platform-a").unwrap();
        cx.simulate_input(window, "ap").unwrap();
        cx.simulate_keystrokes(window, "tab").unwrap();
        assert_eq!(cx.focused(window).unwrap(), Some("after-combobox".into()));
        assert_eq!(
            cx.read(owner, |view| view.combobox.input_value().clone())
                .unwrap(),
            Arc::from("Banana")
        );

        cx.focus(window, "fruit").unwrap();
        cx.simulate_keystrokes(window, "platform-a").unwrap();
        cx.simulate_input(window, "ap").unwrap();
        let popup = cx
            .read(owner, |view| view.combobox.popup_window().unwrap())
            .unwrap();
        assert!(
            !cx.simulate_mouse_down(window, "after-combobox", MouseDownEvent::default(),)
                .unwrap()
        );
        assert!(!cx.is_window_open(popup));
        assert_eq!(
            cx.read(owner, |view| view.combobox.input_value().clone())
                .unwrap(),
            Arc::from("Banana")
        );

        cx.focus(window, "fruit").unwrap();
        cx.simulate_keystrokes(window, "platform-a").unwrap();
        cx.simulate_input(window, "ap").unwrap();
        let popup = cx
            .read(owner, |view| view.combobox.popup_window().unwrap())
            .unwrap();
        cx.update(owner, |_view, cx| cx.close_window_handle(popup))
            .unwrap();
        assert_eq!(
            cx.read(owner, |view| view.combobox.input_value().clone())
                .unwrap(),
            Arc::from("Banana")
        );
        assert_eq!(
            cx.read(owner, |view| view.combobox.query().clone())
                .unwrap(),
            Arc::from("")
        );
    }

    #[test]
    fn never_key_child_click_commits_a_declared_value_and_keeps_owner_input_focus() {
        let (mut cx, owner) = App::new(Owner::new())
            .bind_keys(combobox_key_bindings())
            .into_test_context()
            .unwrap();
        let window = owner.window_handle();
        cx.focus(window, "fruit").unwrap();
        cx.click(window, "fruit").unwrap();
        let (popup, row) = cx
            .read(owner, |view| {
                (
                    view.combobox.popup_window().unwrap(),
                    view.combobox.option_id_for_source("fruit", 2).unwrap(),
                )
            })
            .unwrap();
        assert_eq!(
            cx.window_state(popup).unwrap().kind,
            crate::WindowKind::AnchoredPopup
        );
        assert_eq!(cx.focused(window).unwrap(), Some("fruit".into()));
        cx.click(popup, row).unwrap();
        assert!(!cx.is_window_open(popup));
        assert_eq!(cx.focused(window).unwrap(), Some("fruit".into()));
        assert_eq!(
            cx.read(owner, |view| view.combobox.selected_value().copied())
                .unwrap(),
            Some("apricot")
        );
        assert_eq!(
            cx.read(owner, |view| view.combobox.input_value().clone())
                .unwrap(),
            Arc::from("Apricot")
        );
    }

    #[test]
    fn open_source_replacement_reuses_child_and_stable_selection_survives_absence() {
        let (mut cx, owner) = App::new(Owner::new())
            .bind_keys(combobox_key_bindings())
            .into_test_context()
            .unwrap();
        let window = owner.window_handle();
        cx.focus(window, "fruit").unwrap();
        cx.click(window, "fruit").unwrap();
        let popup = cx
            .read(owner, |view| view.combobox.popup_window().unwrap())
            .unwrap();

        cx.update(owner, |view, cx| {
            view.combobox
                .set_items(
                    [
                        PickerItem::new("Cherry", "cherry").id("cherry"),
                        PickerItem::new("Banana renamed", "banana-v2").id("banana"),
                    ],
                    cx,
                )
                .unwrap();
            cx.invalidate();
        })
        .unwrap();
        assert_eq!(
            cx.read(owner, |view| view.combobox.popup_window()).unwrap(),
            Some(popup)
        );
        assert_eq!(
            cx.read(owner, |view| view.combobox.selected_source_index())
                .unwrap(),
            Some(1)
        );
        assert_eq!(
            cx.read(owner, |view| view.combobox.selected_value().copied())
                .unwrap(),
            Some("banana-v2")
        );

        cx.update(owner, |view, cx| {
            view.combobox
                .set_items([PickerItem::new("Cherry", "cherry").id("cherry")], cx)
                .unwrap();
            cx.invalidate();
        })
        .unwrap();
        assert_eq!(
            cx.read(owner, |view| view.combobox.popup_window()).unwrap(),
            Some(popup)
        );
        assert_eq!(
            cx.read(owner, |view| view.combobox.selected_source_index())
                .unwrap(),
            None
        );
        assert_eq!(
            cx.read(owner, |view| view.combobox.selected_value().copied())
                .unwrap(),
            Some("banana-v2")
        );
        cx.simulate_keystrokes(window, "escape").unwrap();
        assert_eq!(
            cx.read(owner, |view| view.combobox.input_value().clone())
                .unwrap(),
            Arc::from("Banana renamed")
        );

        let error = cx
            .update(owner, |view, cx| {
                view.combobox.set_items(
                    [
                        PickerItem::new("One", "one").id("duplicate"),
                        PickerItem::new("Two", "two").id("duplicate"),
                    ],
                    cx,
                )
            })
            .unwrap()
            .unwrap_err();
        assert_eq!(
            error,
            PickerError::DuplicateId {
                id: "duplicate".into()
            }
        );
        assert_eq!(
            cx.read(owner, |view| view.combobox.items().len()).unwrap(),
            1
        );
        assert_eq!(
            cx.read(owner, |view| view.combobox.selected_value().copied())
                .unwrap(),
            Some("banana-v2")
        );
    }

    #[derive(Debug)]
    struct CloneProbe(Rc<Cell<usize>>);

    impl Clone for CloneProbe {
        fn clone(&self) -> Self {
            self.0.set(self.0.get() + 1);
            Self(Rc::clone(&self.0))
        }
    }

    struct LargeOwner {
        combobox: ComboboxState<CloneProbe>,
        clones: Rc<Cell<usize>>,
    }

    impl LargeOwner {
        fn new() -> Self {
            let clones = Rc::new(Cell::new(0));
            let combobox = ComboboxState::new((0..20_000).map(|index| {
                PickerItem::new(format!("Item {index}"), CloneProbe(Rc::clone(&clones)))
                    .id(index as u64 + 1)
            }))
            .unwrap()
            .with_layout(ComboboxPopupLayout::new(240.0, 30.0).max_visible_rows(5));
            Self { combobox, clones }
        }

        fn combobox(view: &mut Self) -> &mut ComboboxState<CloneProbe> {
            &mut view.combobox
        }
    }

    impl View for LargeOwner {
        fn render(&mut self, cx: &mut ViewContext<'_, Self>) -> impl crate::IntoElement {
            self.combobox.element(
                cx,
                "large-combobox",
                "Large combobox",
                Self::combobox,
                text_input(self.combobox.input_value().clone()).size(240.0, 36.0),
                |_state| div(),
                |_item, _state| div(),
                |_view, _query, _cx| {},
                |_view, _value, _cx| {},
            )
        }
    }

    #[test]
    fn large_sources_mount_visible_rows_without_value_clones_and_settled_windows_sleep() {
        let (mut cx, owner) = App::new(LargeOwner::new())
            .bind_keys(combobox_key_bindings())
            .into_test_context()
            .unwrap();
        let window = owner.window_handle();
        cx.focus(window, "large-combobox").unwrap();
        cx.click(window, "large-combobox").unwrap();
        let popup = cx
            .read(owner, |view| view.combobox.popup_window().unwrap())
            .unwrap();
        assert_eq!(cx.read(owner, |view| view.clones.get()).unwrap(), 0);

        let mounted = (0..32)
            .filter(|source_index| {
                let id = cx
                    .read(owner, |view| {
                        view.combobox
                            .option_id_for_source("large-combobox", *source_index)
                            .unwrap()
                    })
                    .unwrap();
                cx.element_bounds(popup, id).is_ok()
            })
            .count();
        assert!((1..=7).contains(&mounted));

        let owner_renders = cx.render_count(window).unwrap();
        let child_renders = cx.render_count(popup).unwrap();
        cx.run_until_idle().unwrap();
        assert_eq!(cx.render_count(window).unwrap(), owner_renders);
        assert_eq!(cx.render_count(popup).unwrap(), child_renders);
        assert_eq!(cx.read(owner, |view| view.clones.get()).unwrap(), 0);
    }

    #[test]
    fn programmatic_selection_rejects_disabled_items_and_closed_state_has_no_popup() {
        let mut state = ComboboxState::new(options()).unwrap();
        let mut cx = EventContext::default();
        assert!(!state.set_selected_source(1, &mut cx));
        assert!(state.set_selected_id("banana", &mut cx));
        assert_eq!(state.selected_value(), Some(&"banana"));
        assert_eq!(state.input_value().as_ref(), "Banana");
        assert!(state.clear_selection(&mut cx));
        assert_eq!(state.selected_value(), None);
        assert_eq!(state.input_value().as_ref(), "");
        assert_eq!(state.popup_window(), None);
    }

    #[test]
    fn owner_accessibility_proxy_keeps_committed_selection_distinct_from_active_preview() {
        let (mut cx, owner) = App::new(Owner::new())
            .bind_keys(combobox_key_bindings())
            .into_test_context()
            .unwrap();
        let window = owner.window_handle();
        cx.focus(window, "fruit").unwrap();
        cx.click(window, "fruit").unwrap();
        let update = cx.accessibility_update(window).unwrap();
        let node = |role: accesskit::Role, label: &str| {
            update
                .nodes
                .iter()
                .find(|(_node_id, node)| node.role() == role && node.label() == Some(label))
                .unwrap_or_else(|| {
                    panic!(
                        "missing accessibility node {role:?} {label:?}; available={:?}",
                        update
                            .nodes
                            .iter()
                            .map(|(node_id, node)| (*node_id, node.role(), node.label()))
                            .collect::<Vec<_>>()
                    )
                })
        };
        let (input_id, input) = node(accesskit::Role::EditableComboBox, "Fruit");
        let (list_id, _list) = node(accesskit::Role::ListBox, "Fruit");
        let (apple_id, apple) = node(accesskit::Role::ListBoxOption, "Apple");
        let (_banana_id, banana) = node(accesskit::Role::ListBoxOption, "Banana");
        assert_ne!(input_id, list_id);
        assert_eq!(input.controls(), &[*list_id]);
        assert_eq!(input.active_descendant(), Some(*apple_id));
        assert_eq!(apple.is_selected(), None);
        assert_eq!(banana.is_selected(), Some(true));
    }
}
