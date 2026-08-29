use std::{
    fmt,
    sync::Arc,
    time::{Duration, Instant},
};

use crate::{
    AccessibilityPopover, AccessibilityRole, AnchorPlacement, ComboboxConfirm, ComboboxFirst,
    ComboboxLast, ComboboxNext, ComboboxPageDown, ComboboxPageUp, ComboboxPrevious, Element,
    ElementId, EventContext, FocusHandle, Key, MAX_VALIDATION_MESSAGE_BYTES, Modifiers,
    PickerError, PickerItem, View, ViewContext, VirtualList, WindowHandle, div,
    picker::collect_picker_items,
};

/// Maximum option rows mounted by one standalone select popover before virtual scrolling takes over.
pub const MAX_SELECT_VISIBLE_ROWS: usize = 64;
/// Maximum UTF-8 bytes retained by one select popover's incremental typeahead buffer.
pub const MAX_SELECT_TYPEAHEAD_BYTES: usize = 256;
/// Select typeahead expires when the next key arrives after this interval; no timer is scheduled.
pub const SELECT_TYPEAHEAD_TIMEOUT: Duration = Duration::from_millis(500);

const SELECT_KEY_CONTEXT: &str = "Select";
const SELECT_SURFACE_ID_TAG: u64 = 0x99f6_4cc3_13aa_0c81;
const SELECT_OPTION_ID_TAG: u64 = 0x6fe3_f490_a3b8_b271;

/// Structural geometry for the separate native popover surface used by [`SelectState`].
///
/// It intentionally contains no color, typography, border, radius, shadow, or animation tokens.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SelectPopoverLayout {
    pub width: f32,
    pub row_height: f32,
    pub max_visible_rows: usize,
    pub placement: AnchorPlacement,
    pub anchor_gap: f32,
}

impl SelectPopoverLayout {
    pub fn new(width: f32, row_height: f32) -> Self {
        Self {
            width: finite_clamped(width, 1.0, 4_096.0, 240.0),
            row_height: finite_clamped(row_height, 20.0, 256.0, 36.0),
            max_visible_rows: 8,
            placement: AnchorPlacement::BottomStart,
            anchor_gap: 4.0,
        }
    }

    pub fn width(mut self, width: f32) -> Self {
        self.width = finite_clamped(width, 1.0, 4_096.0, 240.0);
        self
    }

    pub fn row_height(mut self, row_height: f32) -> Self {
        self.row_height = finite_clamped(row_height, 20.0, 256.0, 36.0);
        self
    }

    pub fn max_visible_rows(mut self, rows: usize) -> Self {
        self.max_visible_rows = rows.clamp(1, MAX_SELECT_VISIBLE_ROWS);
        self
    }

    pub const fn placement(mut self, placement: AnchorPlacement) -> Self {
        self.placement = placement;
        self
    }

    pub fn anchor_gap(mut self, gap: f32) -> Self {
        self.anchor_gap = finite_clamped(gap, 0.0, 512.0, 4.0);
        self
    }

    fn sanitized(mut self) -> Self {
        self.width = finite_clamped(self.width, 1.0, 4_096.0, 240.0);
        self.row_height = finite_clamped(self.row_height, 20.0, 256.0, 36.0);
        self.max_visible_rows = self.max_visible_rows.clamp(1, MAX_SELECT_VISIBLE_ROWS);
        self.anchor_gap = finite_clamped(self.anchor_gap, 0.0, 512.0, 4.0);
        self
    }

    fn popover_height(self, option_count: usize) -> f32 {
        option_count.min(self.max_visible_rows).max(1) as f32 * self.row_height
    }
}

impl Default for SelectPopoverLayout {
    fn default() -> Self {
        Self::new(240.0, 36.0)
    }
}

/// State supplied to the caller-owned popover-root renderer.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SelectListState {
    pub option_count: usize,
    pub active_index: Option<usize>,
    pub selected_index: Option<usize>,
}

/// State supplied to the caller-owned option renderer.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SelectOptionState {
    pub source_index: usize,
    pub active: bool,
    pub selected: bool,
    pub disabled: bool,
}

/// Bounded controlled state for an unstyled, non-editable single-value select.
///
/// The application owns this value and every visual element. QuickGUI owns keyboard navigation,
/// typeahead, native overflow placement, pointer selection, accessibility semantics, exact child
/// lifecycle synchronization, and visible-row mounting. Closed selects own no window, timer,
/// task, observer, renderer, or scheduler source.
pub struct SelectState<T> {
    items: Arc<[PickerItem<T>]>,
    selected_source: Option<usize>,
    popover: Option<WindowHandle>,
    source_revision: u64,
    disabled: bool,
    invalid: bool,
    validation_message: Option<Arc<str>>,
    validation_message_truncated: bool,
    layout: SelectPopoverLayout,
}

impl<T> fmt::Debug for SelectState<T> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("SelectState")
            .field("items", &self.items.len())
            .field("selected_source", &self.selected_source)
            .field("popover", &self.popover)
            .field("source_revision", &self.source_revision)
            .field("disabled", &self.disabled)
            .field("invalid", &self.invalid)
            .field("validation_message", &self.validation_message)
            .field(
                "validation_message_truncated",
                &self.validation_message_truncated,
            )
            .field("layout", &self.layout)
            .finish_non_exhaustive()
    }
}

impl<T> SelectState<T> {
    pub fn new(items: impl IntoIterator<Item = PickerItem<T>>) -> Result<Self, PickerError> {
        Ok(Self {
            items: Arc::from(collect_picker_items(items)?),
            selected_source: None,
            popover: None,
            source_revision: 1,
            disabled: false,
            invalid: false,
            validation_message: None,
            validation_message_truncated: false,
            layout: SelectPopoverLayout::default(),
        })
    }

    pub fn with_layout(mut self, layout: SelectPopoverLayout) -> Self {
        self.layout = layout.sanitized();
        self
    }

    pub fn layout(&self) -> SelectPopoverLayout {
        self.layout
    }

    pub fn set_layout(&mut self, layout: SelectPopoverLayout) -> bool {
        let layout = layout.sanitized();
        if self.layout == layout {
            return false;
        }
        self.layout = layout;
        true
    }

    pub fn items(&self) -> &[PickerItem<T>] {
        &self.items
    }

    pub const fn selected_source_index(&self) -> Option<usize> {
        self.selected_source
    }

    pub fn selected_item(&self) -> Option<&PickerItem<T>> {
        self.selected_source.and_then(|index| self.items.get(index))
    }

    pub fn selected_value(&self) -> Option<&T> {
        self.selected_item().map(PickerItem::value)
    }

    pub const fn popover_window(&self) -> Option<WindowHandle> {
        self.popover
    }

    pub const fn is_open(&self) -> bool {
        self.popover.is_some()
    }

    pub const fn is_disabled(&self) -> bool {
        self.disabled
    }

    pub const fn is_invalid(&self) -> bool {
        self.invalid
    }

    /// Replace the complete bounded source atomically and close an obsolete open snapshot.
    pub fn set_items(
        &mut self,
        items: impl IntoIterator<Item = PickerItem<T>>,
        cx: &mut EventContext,
    ) -> Result<(), PickerError> {
        let items = Arc::<[PickerItem<T>]>::from(collect_picker_items(items)?);
        let previous_index = self.selected_source;
        let previous_id = self.selected_item().and_then(PickerItem::stable_id);
        self.items = items;
        self.selected_source = previous_id
            .and_then(|id| {
                self.items
                    .iter()
                    .position(|item| item.stable_id() == Some(id))
            })
            .or_else(|| {
                previous_id
                    .is_none()
                    .then_some(previous_index)
                    .flatten()
                    .filter(|index| *index < self.items.len())
            })
            .filter(|index| !self.items[*index].is_disabled());
        self.source_revision = self.source_revision.wrapping_add(1).max(1);
        self.close(cx);
        Ok(())
    }

    pub fn select_source(&mut self, source_index: usize) -> bool {
        if self
            .items
            .get(source_index)
            .is_none_or(PickerItem::is_disabled)
            || self.selected_source == Some(source_index)
        {
            return false;
        }
        self.selected_source = Some(source_index);
        true
    }

    pub fn select_id(&mut self, id: impl Into<ElementId>) -> bool {
        let id = id.into();
        self.items
            .iter()
            .position(|item| item.stable_id() == Some(id))
            .is_some_and(|index| self.select_source(index))
    }

    pub fn clear_selection(&mut self) -> bool {
        self.selected_source.take().is_some()
    }

    /// Change disabled state and synchronously request closure of an open native popover.
    pub fn set_disabled(&mut self, disabled: bool, cx: &mut EventContext) -> bool {
        if self.disabled == disabled {
            return false;
        }
        self.disabled = disabled;
        if disabled {
            self.close(cx);
        }
        true
    }

    pub fn set_invalid(&mut self, invalid: bool) -> bool {
        if self.invalid == invalid {
            return false;
        }
        self.invalid = invalid;
        true
    }

    pub fn validation_message(&self) -> Option<&Arc<str>> {
        self.validation_message.as_ref()
    }

    pub fn set_validation_message(&mut self, message: impl Into<Arc<str>>) -> bool {
        let (message, truncated) = bounded_validation_message(message.into());
        if self.validation_message == message && self.validation_message_truncated == truncated {
            return false;
        }
        self.validation_message = message;
        self.validation_message_truncated = truncated;
        true
    }

    pub fn clear_validation_message(&mut self) -> bool {
        if self.validation_message.take().is_none() && !self.validation_message_truncated {
            return false;
        }
        self.validation_message_truncated = false;
        true
    }

    pub fn close(&mut self, cx: &mut EventContext) -> bool {
        let Some(popover) = self.popover.take() else {
            return false;
        };
        cx.close_window_handle(popover);
        true
    }

    pub fn surface_id(id: impl Into<ElementId>) -> ElementId {
        derived_select_id(id.into(), SELECT_SURFACE_ID_TAG, 0)
    }

    pub fn option_id_for_source(
        &self,
        id: impl Into<ElementId>,
        source_index: usize,
    ) -> Option<ElementId> {
        let item = self.items.get(source_index)?;
        Some(select_option_id(id.into(), item, source_index))
    }

    /// Decorate an application-owned trigger without adding appearance.
    pub fn trigger_part(
        &self,
        id: impl Into<ElementId>,
        label: impl Into<Arc<str>>,
        trigger: Element,
    ) -> Element {
        let id = id.into();
        let mut trigger = trigger
            .id(id)
            .focusable()
            .accessibility_role(AccessibilityRole::ComboBox)
            .accessibility_label(label.into())
            .accessibility_has_popover(AccessibilityPopover::ListBox)
            .accessibility_expanded(self.is_open())
            .disabled(self.disabled)
            .invalid(self.invalid)
            .app_region_no_drag()
            .user_select_none()
            .cursor_default();
        if let Some(item) = self.selected_item() {
            trigger = trigger.accessibility_value(item.label().clone());
        }
        if let Some(message) = self.validation_message.clone() {
            trigger =
                trigger.validation_message_retained(message, self.validation_message_truncated);
        }
        trigger
    }

    /// Build the complete unstyled select interaction from caller-owned trigger, popover, and rows.
    #[allow(clippy::too_many_arguments)]
    pub fn element<V, PopoverRoot, RenderOption, Change>(
        &self,
        cx: &mut ViewContext<'_, V>,
        id: impl Into<ElementId>,
        label: impl Into<Arc<str>>,
        access: fn(&mut V) -> &mut SelectState<T>,
        trigger: Element,
        popover_root: PopoverRoot,
        render_option: RenderOption,
        change: Change,
    ) -> Element
    where
        V: 'static,
        T: Clone + 'static,
        PopoverRoot: Fn(SelectListState) -> Element + Clone + 'static,
        RenderOption: Fn(&PickerItem<T>, SelectOptionState) -> Element + Clone + 'static,
        Change: Fn(&mut V, T, &mut EventContext) + Clone + 'static,
    {
        let id = id.into();
        let label = label.into();
        let renderers = SelectRenderers {
            popover_root,
            render_option,
        };

        cx.on_any_child_window_closed(move |view, closed, cx| {
            let state = access(view);
            if state.popover == Some(closed) {
                state.popover = None;
                cx.focus(FocusHandle::new(id));
                cx.invalidate();
            }
        });

        let commit_change = change.clone();
        let commit = cx.action_listener(id, move |view, action: &SelectCommit, cx| {
            if action.control != id {
                cx.propagate();
                return;
            }
            let value = {
                let state = access(view);
                if state.popover != Some(action.popover)
                    || state.source_revision != action.source_revision
                {
                    return;
                }
                let value = state
                    .items
                    .get(action.source_index)
                    .filter(|item| !item.is_disabled())
                    .map(|item| item.value().clone());
                if value.is_some() {
                    state.selected_source = Some(action.source_index);
                    state.popover = None;
                }
                value
            };
            if let Some(value) = value {
                commit_change(view, value, cx);
                cx.focus(FocusHandle::new(id));
                cx.invalidate();
            }
        });

        let click_renderers = renderers.clone();
        let click_label = label.clone();
        let click = cx.listener(id, move |view, cx| {
            open_select_popover(
                view,
                cx,
                id,
                click_label.clone(),
                access,
                click_renderers.clone(),
            );
        });

        let previous_renderers = renderers.clone();
        let previous_label = label.clone();
        let previous = cx.action_listener(id, move |view, _: &ComboboxPrevious, cx| {
            open_select_popover(
                view,
                cx,
                id,
                previous_label.clone(),
                access,
                previous_renderers.clone(),
            );
        });
        let next_renderers = renderers.clone();
        let next_label = label.clone();
        let next = cx.action_listener(id, move |view, _: &ComboboxNext, cx| {
            open_select_popover(
                view,
                cx,
                id,
                next_label.clone(),
                access,
                next_renderers.clone(),
            );
        });
        let page_up_renderers = renderers.clone();
        let page_up_label = label.clone();
        let page_up = cx.action_listener(id, move |view, _: &ComboboxPageUp, cx| {
            open_select_popover(
                view,
                cx,
                id,
                page_up_label.clone(),
                access,
                page_up_renderers.clone(),
            );
        });
        let page_down_renderers = renderers.clone();
        let page_down_label = label.clone();
        let page_down = cx.action_listener(id, move |view, _: &ComboboxPageDown, cx| {
            open_select_popover(
                view,
                cx,
                id,
                page_down_label.clone(),
                access,
                page_down_renderers.clone(),
            );
        });
        let first_renderers = renderers.clone();
        let first_label = label.clone();
        let first = cx.action_listener(id, move |view, _: &ComboboxFirst, cx| {
            open_select_popover(
                view,
                cx,
                id,
                first_label.clone(),
                access,
                first_renderers.clone(),
            );
        });
        let last_renderers = renderers.clone();
        let last_label = label.clone();
        let last = cx.action_listener(id, move |view, _: &ComboboxLast, cx| {
            open_select_popover(
                view,
                cx,
                id,
                last_label.clone(),
                access,
                last_renderers.clone(),
            );
        });
        let confirm_renderers = renderers;
        let confirm_label = label.clone();
        let confirm = cx.action_listener(id, move |view, _: &ComboboxConfirm, cx| {
            open_select_popover(
                view,
                cx,
                id,
                confirm_label.clone(),
                access,
                confirm_renderers.clone(),
            );
        });

        self.trigger_part(id, label, trigger)
            .on_click(click)
            .key_context(SELECT_KEY_CONTEXT)
            .on_action(commit)
            .on_action(previous)
            .on_action(next)
            .on_action(page_up)
            .on_action(page_down)
            .on_action(first)
            .on_action(last)
            .on_action(confirm)
    }
}

#[derive(Clone)]
struct SelectRenderers<PopoverRoot, RenderOption> {
    popover_root: PopoverRoot,
    render_option: RenderOption,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct SelectCommit {
    control: ElementId,
    popover: WindowHandle,
    source_revision: u64,
    source_index: usize,
}

#[allow(clippy::too_many_arguments)]
fn open_select_popover<V, T, PopoverRoot, RenderOption>(
    view: &mut V,
    cx: &mut EventContext,
    id: ElementId,
    label: Arc<str>,
    access: fn(&mut V) -> &mut SelectState<T>,
    renderers: SelectRenderers<PopoverRoot, RenderOption>,
) where
    V: 'static,
    T: Clone + 'static,
    PopoverRoot: Fn(SelectListState) -> Element + Clone + 'static,
    RenderOption: Fn(&PickerItem<T>, SelectOptionState) -> Element + Clone + 'static,
{
    let (items, selected_source, source_revision, layout) = {
        let state = access(view);
        if state.disabled || state.popover.is_some() {
            return;
        }
        (
            Arc::clone(&state.items),
            state.selected_source,
            state.source_revision,
            state.layout,
        )
    };
    let popover = SelectPopoverView::new(
        id,
        label,
        items,
        selected_source,
        source_revision,
        layout,
        renderers,
    );
    let option_count = popover.items.len();
    let result = crate::SystemPopover::new(layout.width, layout.popover_height(option_count))
        .placement(layout.placement)
        .gap(layout.anchor_gap)
        .open(cx, id, "Select", popover);
    if let Ok(handle) = result {
        access(view).popover = Some(handle);
        cx.invalidate();
    }
}

struct SelectPopoverView<T, PopoverRoot, RenderOption> {
    control: ElementId,
    label: Arc<str>,
    items: Arc<[PickerItem<T>]>,
    selected_source: Option<usize>,
    source_revision: u64,
    active: Option<usize>,
    list: VirtualList,
    layout: SelectPopoverLayout,
    renderers: SelectRenderers<PopoverRoot, RenderOption>,
    typeahead: String,
    typeahead_at: Option<Instant>,
}

impl<T, PopoverRoot, RenderOption> SelectPopoverView<T, PopoverRoot, RenderOption> {
    fn new(
        control: ElementId,
        label: Arc<str>,
        items: Arc<[PickerItem<T>]>,
        selected_source: Option<usize>,
        source_revision: u64,
        layout: SelectPopoverLayout,
        renderers: SelectRenderers<PopoverRoot, RenderOption>,
    ) -> Self {
        let active = selected_source
            .filter(|index| items.get(*index).is_some_and(|item| !item.is_disabled()))
            .or_else(|| items.iter().position(|item| !item.is_disabled()));
        let mut list = VirtualList::new(items.len(), layout.row_height).with_overscan(1);
        list.set_viewport_height(layout.popover_height(items.len()));
        if let Some(active) = active {
            list.scroll_to_reveal(active);
        }
        Self {
            control,
            label,
            items,
            selected_source,
            source_revision,
            active,
            list,
            layout,
            renderers,
            typeahead: String::new(),
            typeahead_at: None,
        }
    }

    fn select_previous(&mut self) -> bool {
        self.move_active(false)
    }

    fn select_next(&mut self) -> bool {
        self.move_active(true)
    }

    fn select_first(&mut self) -> bool {
        self.select_boundary(false)
    }

    fn select_last(&mut self) -> bool {
        self.select_boundary(true)
    }

    fn select_page(&mut self, forward: bool) -> bool {
        if self.items.is_empty() {
            return false;
        }
        let page = self.layout.max_visible_rows.max(1);
        let current = self
            .active
            .unwrap_or(if forward { 0 } else { self.items.len() - 1 });
        let target = if forward {
            current.saturating_add(page).min(self.items.len() - 1)
        } else {
            current.saturating_sub(page)
        };
        let candidate = if forward {
            (target..self.items.len())
                .chain(0..target)
                .find(|index| !self.items[*index].is_disabled())
        } else {
            (0..=target)
                .rev()
                .chain((target + 1..self.items.len()).rev())
                .find(|index| !self.items[*index].is_disabled())
        };
        candidate.is_some_and(|index| self.set_active(index))
    }

    fn select_boundary(&mut self, last: bool) -> bool {
        let candidate = if last {
            self.items.iter().rposition(|item| !item.is_disabled())
        } else {
            self.items.iter().position(|item| !item.is_disabled())
        };
        candidate.is_some_and(|index| self.set_active(index))
    }

    fn move_active(&mut self, forward: bool) -> bool {
        let len = self.items.len();
        if len == 0 {
            return false;
        }
        let start = match (self.active, forward) {
            (Some(index), true) => (index + 1) % len,
            (Some(index), false) => (index + len - 1) % len,
            (None, true) => 0,
            (None, false) => len - 1,
        };
        (0..len)
            .map(|offset| {
                if forward {
                    (start + offset) % len
                } else {
                    (start + len - offset) % len
                }
            })
            .find(|index| !self.items[*index].is_disabled())
            .is_some_and(|index| self.set_active(index))
    }

    fn set_active(&mut self, index: usize) -> bool {
        if self.items.get(index).is_none_or(PickerItem::is_disabled) || self.active == Some(index) {
            return false;
        }
        self.active = Some(index);
        self.list.scroll_to_reveal(index);
        self.typeahead.clear();
        self.typeahead_at = None;
        true
    }

    fn typeahead(&mut self, value: &str, now: Instant) -> bool {
        let input = normalized_typeahead_input(value);
        if input.is_empty() {
            return false;
        }
        if self.typeahead_at.is_none_or(|previous| {
            now.saturating_duration_since(previous) > SELECT_TYPEAHEAD_TIMEOUT
        }) {
            self.typeahead.clear();
        }
        self.typeahead_at = Some(now);
        push_bounded(&mut self.typeahead, &input, MAX_SELECT_TYPEAHEAD_BYTES);
        let first = self.typeahead.chars().next();
        let repeated = first.is_some()
            && self
                .typeahead
                .chars()
                .all(|character| Some(character) == first);
        let repeated_prefix;
        let prefix = if repeated && self.typeahead.chars().count() > 1 {
            repeated_prefix = first.expect("non-empty typeahead").to_string();
            repeated_prefix.as_str()
        } else {
            self.typeahead.as_str()
        };
        let len = self.items.len();
        let start = self.active.map_or(0, |active| (active + 1) % len.max(1));
        for offset in 0..len {
            let index = (start + offset) % len;
            let item = &self.items[index];
            if !item.is_disabled() && label_starts_with(item.label(), prefix) {
                let changed = self.active != Some(index);
                self.active = Some(index);
                self.list.scroll_to_reveal(index);
                return changed;
            }
        }
        false
    }

    fn commit(&self, cx: &mut EventContext) -> bool {
        let Some(source_index) = self.active else {
            return false;
        };
        let Some(popover) = cx.window_handle() else {
            return false;
        };
        if !cx.dispatch_action_to_popover_owner(SelectCommit {
            control: self.control,
            popover,
            source_revision: self.source_revision,
            source_index,
        }) {
            return false;
        }
        cx.close_popover_chain()
    }

    fn surface_id(&self) -> ElementId {
        SelectState::<T>::surface_id(self.control)
    }

    fn option_id(&self, source_index: usize) -> ElementId {
        select_option_id(self.control, &self.items[source_index], source_index)
    }
}

impl<T, PopoverRoot, RenderOption> View for SelectPopoverView<T, PopoverRoot, RenderOption>
where
    T: Clone + 'static,
    PopoverRoot: Fn(SelectListState) -> Element + Clone + 'static,
    RenderOption: Fn(&PickerItem<T>, SelectOptionState) -> Element + Clone + 'static,
{
    fn render(&mut self, cx: &mut ViewContext<'_, Self>) -> impl crate::IntoElement {
        let surface_id = self.surface_id();
        let previous = cx.action_listener(surface_id, |view, _: &ComboboxPrevious, cx| {
            if view.select_previous() {
                cx.invalidate();
            }
        });
        let next = cx.action_listener(surface_id, |view, _: &ComboboxNext, cx| {
            if view.select_next() {
                cx.invalidate();
            }
        });
        let page_up = cx.action_listener(surface_id, |view, _: &ComboboxPageUp, cx| {
            if view.select_page(false) {
                cx.invalidate();
            }
        });
        let page_down = cx.action_listener(surface_id, |view, _: &ComboboxPageDown, cx| {
            if view.select_page(true) {
                cx.invalidate();
            }
        });
        let first = cx.action_listener(surface_id, |view, _: &ComboboxFirst, cx| {
            if view.select_first() {
                cx.invalidate();
            }
        });
        let last = cx.action_listener(surface_id, |view, _: &ComboboxLast, cx| {
            if view.select_last() {
                cx.invalidate();
            }
        });
        let confirm = cx.action_listener(surface_id, |view, _: &ComboboxConfirm, cx| {
            view.commit(cx);
        });
        let key_down = cx.key_down_listener(surface_id, |view, event, cx| {
            if event.key == Key::Escape {
                cx.close_popover_chain();
                cx.prevent_default();
                cx.stop_propagation();
                return;
            }
            if event
                .modifiers
                .intersects(Modifiers::CONTROL | Modifiers::ALT | Modifiers::SUPER)
            {
                return;
            }
            let Key::Character(value) = event.key_char.as_ref().unwrap_or(&event.key) else {
                return;
            };
            if view.typeahead(value, Instant::now()) {
                cx.invalidate();
            }
            cx.prevent_default();
            cx.stop_propagation();
        });

        self.list
            .set_viewport_height(self.layout.popover_height(self.items.len()));
        if let Some(active) = self.active {
            self.list.scroll_to_reveal(active);
        }
        let mut rows = Vec::with_capacity(self.list.visible_rows().len());
        for source_index in self.list.visible_rows().range {
            let item = &self.items[source_index];
            let option_state = SelectOptionState {
                source_index,
                active: self.active == Some(source_index),
                selected: self.selected_source == Some(source_index),
                disabled: item.is_disabled(),
            };
            let row_id = self.option_id(source_index);
            let mut row = (self.renderers.render_option)(item, option_state)
                .id(row_id)
                .clickable()
                .tab_index(-1)
                .disabled(option_state.disabled)
                .selected(option_state.selected)
                .accessibility_role(AccessibilityRole::ListBoxOption)
                .accessibility_label(item.label().clone())
                .accessibility_position_in_set(source_index)
                .absolute()
                .top(source_index as f32 * self.layout.row_height - self.list.scroll_offset())
                .left(0.0)
                .w_full()
                .h(self.layout.row_height)
                .app_region_no_drag()
                .user_select_none()
                .cursor_default();
            if !option_state.disabled {
                let hover = cx.hover_listener(row_id, move |view, hovered, cx| {
                    if *hovered && view.set_active(source_index) {
                        cx.invalidate();
                    }
                });
                let click = cx.listener(row_id, move |view, cx| {
                    if view.set_active(source_index) {
                        cx.invalidate();
                    }
                    view.commit(cx);
                });
                row = row.on_hover(hover).on_click(click);
            }
            rows.push(row);
        }

        let list_state = SelectListState {
            option_count: self.items.len(),
            active_index: self.active,
            selected_index: self.selected_source,
        };
        let options = div()
            .relative()
            .size_full()
            .overflow_hidden()
            .virtual_scroll(&self.list)
            .children(rows);
        let mut root = (self.renderers.popover_root)(list_state)
            .id(surface_id)
            .track_focus(FocusHandle::new(surface_id))
            .auto_focus()
            .tab_index(-1)
            .key_context(SELECT_KEY_CONTEXT)
            .accessibility_role(AccessibilityRole::ListBox)
            .accessibility_label(self.label.clone())
            .accessibility_size_of_set(self.items.len())
            .size_full()
            .overflow_hidden()
            .app_region_no_drag()
            .user_select_none()
            .cursor_default()
            .on_action(previous)
            .on_action(next)
            .on_action(page_up)
            .on_action(page_down)
            .on_action(first)
            .on_action(last)
            .on_action(confirm)
            .on_key_down(key_down)
            .child(options);
        if let Some(active) = self.active {
            root = root.accessibility_active_descendant(self.option_id(active));
        }
        root
    }
}

fn select_option_id<T>(control: ElementId, item: &PickerItem<T>, source_index: usize) -> ElementId {
    derived_select_id(
        control,
        SELECT_OPTION_ID_TAG,
        item.stable_id()
            .map_or(source_index as u64, ElementId::as_u64),
    )
}

fn derived_select_id(parent: ElementId, tag: u64, value: u64) -> ElementId {
    let mut hash = parent.as_u64() ^ tag ^ value.wrapping_mul(0x9e37_79b9_7f4a_7c15);
    hash ^= hash >> 30;
    hash = hash.wrapping_mul(0xbf58_476d_1ce4_e5b9);
    hash ^= hash >> 27;
    hash = hash.wrapping_mul(0x94d0_49bb_1331_11eb);
    hash ^= hash >> 31;
    if hash == 0 || hash == parent.as_u64() || hash == u64::MAX {
        hash ^= tag.rotate_left(17);
    }
    ElementId::new(hash)
}

fn normalized_typeahead_input(value: &str) -> String {
    value
        .chars()
        .filter(|character| !character.is_control())
        .flat_map(char::to_lowercase)
        .collect()
}

fn label_starts_with(label: &str, prefix: &str) -> bool {
    let mut label = label.chars().flat_map(char::to_lowercase);
    prefix
        .chars()
        .all(|expected| label.next() == Some(expected))
}

fn push_bounded(target: &mut String, value: &str, maximum: usize) {
    if target.len() >= maximum {
        return;
    }
    let remaining = maximum - target.len();
    if value.len() <= remaining {
        target.push_str(value);
        return;
    }
    let mut end = remaining;
    while end > 0 && !value.is_char_boundary(end) {
        end -= 1;
    }
    target.push_str(&value[..end]);
}

fn finite_clamped(value: f32, minimum: f32, maximum: f32, fallback: f32) -> f32 {
    if value.is_finite() {
        value.clamp(minimum, maximum)
    } else {
        fallback
    }
}

fn bounded_validation_message(message: Arc<str>) -> (Option<Arc<str>>, bool) {
    if message.is_empty() {
        return (None, false);
    }
    if message.len() <= MAX_VALIDATION_MESSAGE_BYTES {
        return (Some(message), false);
    }
    let mut end = MAX_VALIDATION_MESSAGE_BYTES;
    while !message.is_char_boundary(end) {
        end -= 1;
    }
    (Some(Arc::from(&message[..end])), true)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{App, Color, select_key_bindings, text};

    fn options() -> [PickerItem<&'static str>; 4] {
        [
            PickerItem::new("Alpha", "alpha").id("alpha"),
            PickerItem::new("Disabled", "disabled")
                .id("disabled")
                .disabled(true),
            PickerItem::new("Beta", "beta").id("beta"),
            PickerItem::new("Bravo", "bravo").id("bravo"),
        ]
    }

    fn popover_root(_state: SelectListState) -> Element {
        div().bg(Color::BLACK)
    }

    fn option_row(item: &PickerItem<&'static str>, state: SelectOptionState) -> Element {
        div()
            .child(text(item.label().clone()))
            .opacity(if state.active { 1.0 } else { 0.8 })
    }

    #[test]
    fn state_replacement_is_atomic_and_preserves_stable_selection() {
        let mut state = SelectState::new(options()).unwrap();
        assert!(state.select_id("beta"));
        let stable_option = state.option_id_for_source("select", 2).unwrap();
        let mut cx = EventContext::default();
        state
            .set_items(
                [
                    PickerItem::new("Beta renamed", "new beta").id("beta"),
                    PickerItem::new("Alpha", "new alpha").id("alpha"),
                ],
                &mut cx,
            )
            .unwrap();
        assert_eq!(state.selected_source_index(), Some(0));
        assert_eq!(state.selected_value(), Some(&"new beta"));
        assert_eq!(state.option_id_for_source("select", 0), Some(stable_option));

        let error = state
            .set_items(
                [
                    PickerItem::new("One", "one").id("same"),
                    PickerItem::new("Two", "two").id("same"),
                ],
                &mut cx,
            )
            .unwrap_err();
        assert_eq!(error, PickerError::DuplicateId { id: "same".into() });
        assert_eq!(state.selected_value(), Some(&"new beta"));
    }

    #[test]
    fn trigger_part_adds_behavior_without_appearance() {
        let state = SelectState::new(options()).unwrap();
        let trigger = state.trigger_part("select", "Theme", div());
        assert_eq!(trigger.accessibility.role, AccessibilityRole::ComboBox);
        assert_eq!(
            trigger.accessibility.has_popover,
            Some(AccessibilityPopover::ListBox)
        );
        assert!(trigger.focusable);
        assert_eq!(trigger.visual.background, None);
        assert_eq!(trigger.visual.border_color, None);
    }

    #[test]
    fn validation_message_bound_preserves_utf8() {
        let mut state = SelectState::new(options()).unwrap();
        let message = "é".repeat(MAX_VALIDATION_MESSAGE_BYTES);
        assert!(state.set_validation_message(message));
        let retained = state.validation_message().unwrap();
        assert!(retained.len() <= MAX_VALIDATION_MESSAGE_BYTES);
        assert!(retained.is_char_boundary(retained.len()));
        assert!(state.validation_message_truncated);
        assert!(state.clear_validation_message());
        assert_eq!(state.validation_message(), None);
        assert!(!state.validation_message_truncated);
    }

    struct SelectOwner {
        select: SelectState<&'static str>,
        value: Option<&'static str>,
    }

    impl Default for SelectOwner {
        fn default() -> Self {
            Self {
                select: SelectState::new(options())
                    .unwrap()
                    .with_layout(SelectPopoverLayout::new(220.0, 32.0).max_visible_rows(2)),
                value: None,
            }
        }
    }

    impl SelectOwner {
        fn select(view: &mut Self) -> &mut SelectState<&'static str> {
            &mut view.select
        }
    }

    impl View for SelectOwner {
        fn render(&mut self, cx: &mut ViewContext<'_, Self>) -> impl crate::IntoElement {
            self.select.element(
                cx,
                "select",
                "Theme",
                Self::select,
                div().child("Choose"),
                popover_root,
                option_row,
                |view, value, _cx| view.value = Some(value),
            )
        }
    }

    #[test]
    fn native_popover_commits_through_owner_and_close_lifecycle_clears_state() {
        let (mut cx, owner) = App::new(SelectOwner::default())
            .bind_keys(select_key_bindings())
            .into_test_context()
            .unwrap();
        cx.click(owner.window_handle(), "select").unwrap();
        let popover = cx
            .read(owner, |view| view.select.popover_window().unwrap())
            .unwrap();
        assert_eq!(
            cx.window_state(popover).unwrap().kind,
            crate::WindowKind::SystemPopover
        );

        cx.simulate_keystrokes(popover, "down enter").unwrap();
        assert!(!cx.is_window_open(popover));
        assert_eq!(cx.read(owner, |view| view.value).unwrap(), Some("beta"));
        assert_eq!(
            cx.read(owner, |view| view.select.popover_window()).unwrap(),
            None
        );

        cx.click(owner.window_handle(), "select").unwrap();
        let popover = cx
            .read(owner, |view| view.select.popover_window().unwrap())
            .unwrap();
        cx.update(owner, |_view, cx| cx.close_window_handle(popover))
            .unwrap();
        assert_eq!(
            cx.read(owner, |view| view.select.popover_window()).unwrap(),
            None
        );
        let renders = cx.render_count(owner.window_handle()).unwrap();
        cx.run_until_idle().unwrap();
        assert_eq!(cx.render_count(owner.window_handle()).unwrap(), renders);
    }

    #[test]
    fn popover_typeahead_is_bounded_cycles_and_skips_disabled_options() {
        let renderers = SelectRenderers {
            popover_root,
            render_option: option_row,
        };
        let mut popover = SelectPopoverView::new(
            "select".into(),
            Arc::from("Theme"),
            Arc::from(options()),
            None,
            1,
            SelectPopoverLayout::default(),
            renderers,
        );
        let now = Instant::now();
        assert!(popover.typeahead("b", now));
        assert_eq!(popover.active, Some(2));
        assert!(popover.typeahead("b", now + Duration::from_millis(10)));
        assert_eq!(popover.active, Some(3));
        popover.typeahead(&"z".repeat(MAX_SELECT_TYPEAHEAD_BYTES * 2), now);
        assert!(popover.typeahead.len() <= MAX_SELECT_TYPEAHEAD_BYTES);
    }

    #[test]
    fn child_popover_options_remain_visible_only_for_large_sources() {
        let items = Arc::from(
            collect_picker_items(
                (0..20_000).map(|index| PickerItem::new(format!("Option {index}"), index)),
            )
            .unwrap(),
        );
        let popover = SelectPopoverView::new(
            "large-select".into(),
            Arc::from("Large"),
            items,
            Some(19_999),
            1,
            SelectPopoverLayout::new(240.0, 32.0).max_visible_rows(5),
            SelectRenderers {
                popover_root,
                render_option: |_item: &PickerItem<usize>, _state: SelectOptionState| div(),
            },
        );
        assert!(popover.list.visible_rows().len() <= 7);
        assert_eq!(popover.active, Some(19_999));
    }

    #[test]
    fn child_window_config_is_a_real_overflow_capable_popover() {
        let mut state = SelectState::new(options()).unwrap();
        let mut cx = EventContext::default();
        state.popover = Some(WindowHandle::next());
        assert!(state.close(&mut cx));
        assert_eq!(cx.close_windows.len(), 1);
        let options = crate::SystemPopover::new(240.0, 120.0).window_options("Select");
        assert_eq!(options.kind, crate::WindowKind::SystemPopover);
    }
}
