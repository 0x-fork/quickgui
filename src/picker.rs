use std::{cmp::Ordering, fmt, ops::Range, sync::Arc};

use thiserror::Error;
use unicode_segmentation::UnicodeSegmentation;

use crate::{
    AccessibilityRole, Color, Element, ElementId, EventContext, FocusHandle, HighlightStyle,
    IntoElement, KeyBinding, StyledText, ViewContext, VirtualList, button, div, text, text_input,
};

/// Maximum number of source items retained by one picker.
pub const MAX_PICKER_ITEMS: usize = 65_536;
/// Maximum ranked results retained after one query.
pub const MAX_PICKER_RESULTS: usize = 2_048;
/// Maximum Unicode grapheme clusters accepted in a picker query.
pub const MAX_PICKER_QUERY_GRAPHEMES: usize = 128;
/// Maximum UTF-8 bytes retained for one picker query.
pub const MAX_PICKER_QUERY_BYTES: usize = 4 * 1024;
/// Maximum UTF-8 search metadata accepted for one source item.
pub const MAX_PICKER_ITEM_TEXT_BYTES: usize = 64 * 1024;
/// Maximum UTF-8 search metadata retained by one picker.
pub const MAX_PICKER_TEXT_BYTES: usize = 16 * 1024 * 1024;

const MAX_NORMALIZED_QUERY_CHARS: usize = MAX_PICKER_QUERY_GRAPHEMES * 4;
const MAX_VISIBLE_PICKER_ROWS: usize = 64;
const PICKER_KEY_CONTEXT: &str = "Picker";
const INPUT_ID_TAG: u64 = 0x8b8b_a761_9a8c_4f3d;
const ROW_ID_TAG: u64 = 0x53f7_b9c2_7651_0aa1;

/// Move the active picker selection to the previous selectable result.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct PickerPrevious;
/// Move the active picker selection to the next selectable result.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct PickerNext;
/// Move the active picker selection one visible page upward.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct PickerPageUp;
/// Move the active picker selection one visible page downward.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct PickerPageDown;
/// Move the active picker selection to the first selectable result.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct PickerFirst;
/// Move the active picker selection to the final selectable result.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct PickerLast;
/// Activate the selected picker result.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct PickerConfirm;

/// Default contextual bindings for [`PickerState::element`].
///
/// Applications install these once with [`crate::App::bind_keys`]. The `Picker` context exists
/// only on the focused picker path, so ordinary text fields retain their normal arrow behavior.
pub fn picker_key_bindings() -> [KeyBinding; 7] {
    [
        KeyBinding::new("up", PickerPrevious, Some(PICKER_KEY_CONTEXT)),
        KeyBinding::new("down", PickerNext, Some(PICKER_KEY_CONTEXT)),
        KeyBinding::new("pageup", PickerPageUp, Some(PICKER_KEY_CONTEXT)),
        KeyBinding::new("pagedown", PickerPageDown, Some(PICKER_KEY_CONTEXT)),
        KeyBinding::new("platform-up", PickerFirst, Some(PICKER_KEY_CONTEXT)),
        KeyBinding::new("platform-down", PickerLast, Some(PICKER_KEY_CONTEXT)),
        KeyBinding::new("enter", PickerConfirm, Some(PICKER_KEY_CONTEXT)),
    ]
}

/// A value and its searchable, presentational picker metadata.
#[derive(Clone, Debug)]
pub struct PickerItem<T> {
    label: Arc<str>,
    detail: Option<Arc<str>>,
    keywords: Arc<str>,
    shortcut: Option<Arc<str>>,
    value: T,
    disabled: bool,
}

impl<T> PickerItem<T> {
    pub fn new(label: impl Into<Arc<str>>, value: T) -> Self {
        Self {
            label: label.into(),
            detail: None,
            keywords: Arc::from(""),
            shortcut: None,
            value,
            disabled: false,
        }
    }

    pub fn detail(mut self, detail: impl Into<Arc<str>>) -> Self {
        let detail = detail.into();
        self.detail = (!detail.is_empty()).then_some(detail);
        self
    }

    /// Add aliases or namespaced terms used for matching but not rendered as the primary label.
    pub fn keywords(mut self, keywords: impl Into<Arc<str>>) -> Self {
        self.keywords = keywords.into();
        self
    }

    pub fn shortcut(mut self, shortcut: impl Into<Arc<str>>) -> Self {
        let shortcut = shortcut.into();
        self.shortcut = (!shortcut.is_empty()).then_some(shortcut);
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    pub fn label(&self) -> &Arc<str> {
        &self.label
    }

    pub fn detail_text(&self) -> Option<&Arc<str>> {
        self.detail.as_ref()
    }

    pub fn search_keywords(&self) -> &Arc<str> {
        &self.keywords
    }

    pub fn shortcut_text(&self) -> Option<&Arc<str>> {
        self.shortcut.as_ref()
    }

    pub fn value(&self) -> &T {
        &self.value
    }

    pub fn into_value(self) -> T {
        self.value
    }

    pub fn is_disabled(&self) -> bool {
        self.disabled
    }
}

/// A picker source exceeded a hard CPU or memory boundary.
#[derive(Clone, Debug, Eq, Error, PartialEq)]
pub enum PickerError {
    #[error("a picker supports at most {limit} source items")]
    TooManyItems { limit: usize },
    #[error("picker item {index} has {bytes} bytes of searchable text; the limit is {limit}")]
    ItemTextTooLong {
        index: usize,
        bytes: usize,
        limit: usize,
    },
    #[error("picker searchable text uses {bytes} bytes; the total limit is {limit}")]
    TextBudgetExceeded { bytes: usize, limit: usize },
}

/// Visual configuration for the reusable picker surface.
#[derive(Clone, Debug, PartialEq)]
pub struct PickerStyle {
    pub width: f32,
    pub row_height: f32,
    pub max_visible_rows: usize,
    pub placeholder: Arc<str>,
    pub no_matches_text: Arc<str>,
    pub background: Color,
    pub input_background: Color,
    pub border: Color,
    pub selection: Color,
    pub selection_hover: Color,
    pub row_hover: Color,
    pub text: Color,
    pub muted_text: Color,
    pub matched_text: Color,
}

impl Default for PickerStyle {
    fn default() -> Self {
        Self {
            width: 560.0,
            row_height: 48.0,
            max_visible_rows: 9,
            placeholder: Arc::from("Type a command…"),
            no_matches_text: Arc::from("No matching commands"),
            background: Color::rgb8(27, 30, 37),
            input_background: Color::rgb8(20, 23, 29),
            border: Color::rgb8(74, 81, 96),
            selection: Color::rgb8(39, 76, 119),
            selection_hover: Color::rgb8(45, 88, 137),
            row_hover: Color::rgb8(43, 48, 59),
            text: Color::rgb8(234, 236, 241),
            muted_text: Color::rgb8(145, 151, 164),
            matched_text: Color::rgb8(126, 231, 212),
        }
    }
}

impl PickerStyle {
    pub fn width(mut self, width: f32) -> Self {
        self.width = width;
        self.sanitized()
    }

    pub fn row_height(mut self, height: f32) -> Self {
        self.row_height = height;
        self.sanitized()
    }

    pub fn max_visible_rows(mut self, rows: usize) -> Self {
        self.max_visible_rows = rows;
        self.sanitized()
    }

    pub fn placeholder(mut self, placeholder: impl Into<Arc<str>>) -> Self {
        self.placeholder = placeholder.into();
        self
    }

    pub fn no_matches_text(mut self, text: impl Into<Arc<str>>) -> Self {
        self.no_matches_text = text.into();
        self
    }

    fn sanitized(mut self) -> Self {
        self.width = finite_at_least(self.width, 1.0, 560.0);
        self.row_height = finite_at_least(self.row_height, 24.0, 48.0);
        self.max_visible_rows = self.max_visible_rows.clamp(1, MAX_VISIBLE_PICKER_ROWS);
        self
    }
}

#[derive(Clone, Debug)]
struct MatchEntry {
    source_index: usize,
    score: i32,
    label_ranges: Box<[Range<usize>]>,
}

/// A borrowed ranked picker result.
#[derive(Clone, Debug)]
pub struct PickerMatch<'a, T> {
    result_index: usize,
    source_index: usize,
    score: i32,
    label_ranges: &'a [Range<usize>],
    item: &'a PickerItem<T>,
    selected: bool,
}

impl<'a, T> PickerMatch<'a, T> {
    pub fn result_index(&self) -> usize {
        self.result_index
    }

    pub fn source_index(&self) -> usize {
        self.source_index
    }

    pub fn score(&self) -> i32 {
        self.score
    }

    pub fn label_ranges(&self) -> &'a [Range<usize>] {
        self.label_ranges
    }

    pub fn item(&self) -> &'a PickerItem<T> {
        self.item
    }

    pub fn is_selected(&self) -> bool {
        self.selected
    }
}

/// Retained fuzzy matches, keyboard selection, and virtual-scroll state for a picker.
///
/// Matching runs only when source items or the controlled query change. Results and search text
/// are hard-bounded; scrolling and hover reuse the framework's retained list and paint paths.
pub struct PickerState<T> {
    items: Vec<PickerItem<T>>,
    query: Arc<str>,
    matches: Vec<MatchEntry>,
    total_match_count: usize,
    selected_result: Option<usize>,
    list: VirtualList,
    style: PickerStyle,
    score_scratch: Vec<(i32, usize)>,
}

impl<T> fmt::Debug for PickerState<T> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("PickerState")
            .field("items", &self.items.len())
            .field("query", &self.query)
            .field("matches", &self.matches.len())
            .field("total_match_count", &self.total_match_count)
            .field("selected_result", &self.selected_result)
            .field("list", &self.list)
            .field("style", &self.style)
            .finish_non_exhaustive()
    }
}

impl<T> PickerState<T> {
    pub fn new(items: impl IntoIterator<Item = PickerItem<T>>) -> Result<Self, PickerError> {
        let items = collect_picker_items(items)?;
        let style = PickerStyle::default().sanitized();
        let mut state = Self {
            items,
            query: Arc::from(""),
            matches: Vec::with_capacity(MAX_PICKER_RESULTS.min(256)),
            total_match_count: 0,
            selected_result: None,
            list: VirtualList::new(0, style.row_height).with_overscan(1),
            style,
            score_scratch: Vec::new(),
        };
        state.rebuild_matches();
        Ok(state)
    }

    pub fn with_style(mut self, style: PickerStyle) -> Self {
        self.set_style(style);
        self
    }

    pub fn style(&self) -> &PickerStyle {
        &self.style
    }

    pub fn set_style(&mut self, style: PickerStyle) -> bool {
        let style = style.sanitized();
        if self.style == style {
            return false;
        }
        let row_height_changed = self.style.row_height != style.row_height;
        self.style = style;
        if row_height_changed {
            self.list =
                VirtualList::new(self.matches.len(), self.style.row_height).with_overscan(1);
        }
        true
    }

    pub fn set_items(
        &mut self,
        items: impl IntoIterator<Item = PickerItem<T>>,
    ) -> Result<(), PickerError> {
        self.items = collect_picker_items(items)?;
        self.rebuild_matches();
        Ok(())
    }

    pub fn items(&self) -> &[PickerItem<T>] {
        &self.items
    }

    pub fn query(&self) -> &Arc<str> {
        &self.query
    }

    /// Replace the controlled query and recompute the bounded ranked result set.
    pub fn set_query(&mut self, query: &str) -> bool {
        let query = bounded_query(query);
        if self.query.as_ref() == query {
            return false;
        }
        self.query = Arc::from(query);
        self.rebuild_matches();
        true
    }

    /// Clear the query, restore source-order matches, select the first enabled item, and scroll
    /// back to the beginning. Command palettes normally call this each time they open.
    pub fn reset(&mut self) -> bool {
        let changed = !self.query.is_empty()
            || self.selected_result
                != (0..self.matches.len()).find(|index| self.result_is_selectable(*index))
            || self.list.scroll_offset() != 0.0;
        self.query = Arc::from("");
        self.rebuild_matches();
        changed
    }

    pub fn result_count(&self) -> usize {
        self.matches.len()
    }

    pub fn total_match_count(&self) -> usize {
        self.total_match_count
    }

    pub fn results_truncated(&self) -> bool {
        self.total_match_count > self.matches.len()
    }

    pub fn selected_result_index(&self) -> Option<usize> {
        self.selected_result
    }

    pub fn match_at(&self, result_index: usize) -> Option<PickerMatch<'_, T>> {
        let entry = self.matches.get(result_index)?;
        Some(PickerMatch {
            result_index,
            source_index: entry.source_index,
            score: entry.score,
            label_ranges: &entry.label_ranges,
            item: &self.items[entry.source_index],
            selected: self.selected_result == Some(result_index),
        })
    }

    pub fn selected_match(&self) -> Option<PickerMatch<'_, T>> {
        self.selected_result.and_then(|index| self.match_at(index))
    }

    pub fn selected_value(&self) -> Option<&T> {
        self.selected_match().map(|matched| matched.item.value())
    }

    pub fn visible_matches(&self) -> impl Iterator<Item = PickerMatch<'_, T>> + '_ {
        self.list
            .visible_rows()
            .range
            .filter_map(move |index| self.match_at(index))
    }

    pub fn virtual_list(&self) -> &VirtualList {
        &self.list
    }

    pub fn select_result(&mut self, result_index: usize) -> bool {
        if !self.result_is_selectable(result_index) {
            return false;
        }
        let changed = self.selected_result != Some(result_index);
        self.selected_result = Some(result_index);
        self.list.scroll_to_reveal(result_index) || changed
    }

    pub fn select_source(&mut self, source_index: usize) -> bool {
        self.matches
            .iter()
            .position(|entry| entry.source_index == source_index)
            .is_some_and(|result_index| self.select_result(result_index))
    }

    pub fn select_next(&mut self) -> bool {
        let Some(next) = self.next_selectable(true) else {
            return false;
        };
        self.select_result(next)
    }

    pub fn select_previous(&mut self) -> bool {
        let Some(previous) = self.next_selectable(false) else {
            return false;
        };
        self.select_result(previous)
    }

    pub fn select_first(&mut self) -> bool {
        let Some(first) = (0..self.matches.len()).find(|index| self.result_is_selectable(*index))
        else {
            return false;
        };
        self.select_result(first)
    }

    pub fn select_last(&mut self) -> bool {
        let Some(last) = (0..self.matches.len())
            .rev()
            .find(|index| self.result_is_selectable(*index))
        else {
            return false;
        };
        self.select_result(last)
    }

    pub fn select_page_down(&mut self) -> bool {
        self.select_page(true)
    }

    pub fn select_page_up(&mut self) -> bool {
        self.select_page(false)
    }

    /// Stable focus handle for the search input derived from a picker surface identity.
    ///
    /// This may be passed to [`EventContext::focus`] in the same event that mounts the picker;
    /// QuickGUI carries the request through that one retained-tree rebuild.
    pub fn input_focus_handle(id: impl Into<ElementId>) -> FocusHandle {
        FocusHandle::new(derived_picker_id(id.into(), INPUT_ID_TAG, 0))
    }

    /// Build a complete uniform-row picker surface using view-local typed listeners.
    ///
    /// `access` identifies this state inside the owning view when an event arrives. `activate`
    /// receives a clone of the selected value, so the picker can close and restore focus before
    /// dispatching an application command without retaining a borrow into view state.
    pub fn element<V, Activate>(
        &mut self,
        cx: &mut ViewContext<'_, V>,
        id: impl Into<ElementId>,
        access: fn(&mut V) -> &mut PickerState<T>,
        activate: Activate,
    ) -> Element
    where
        V: 'static,
        T: Clone + 'static,
        Activate: Fn(&mut V, T, &mut EventContext) + Clone + 'static,
    {
        let id = id.into();
        let input_focus = Self::input_focus_handle(id);
        let input_id = input_focus.id();
        let style = self.style.clone();
        let visible_row_count = self.matches.len().min(style.max_visible_rows);
        let results_height = visible_row_count as f32 * style.row_height;
        self.list.set_viewport_height(results_height);
        if let Some(selected) = self.selected_result {
            self.list.scroll_to_reveal(selected);
        }

        let query = cx.input_listener(input_id, move |view, value, cx| {
            if access(view).set_query(value) {
                cx.invalidate();
            }
        });
        let previous = cx.action_listener(id, move |view, _: &PickerPrevious, cx| {
            if access(view).select_previous() {
                cx.invalidate();
            }
        });
        let next = cx.action_listener(id, move |view, _: &PickerNext, cx| {
            if access(view).select_next() {
                cx.invalidate();
            }
        });
        let page_up = cx.action_listener(id, move |view, _: &PickerPageUp, cx| {
            if access(view).select_page_up() {
                cx.invalidate();
            }
        });
        let page_down = cx.action_listener(id, move |view, _: &PickerPageDown, cx| {
            if access(view).select_page_down() {
                cx.invalidate();
            }
        });
        let first = cx.action_listener(id, move |view, _: &PickerFirst, cx| {
            if access(view).select_first() {
                cx.invalidate();
            }
        });
        let last = cx.action_listener(id, move |view, _: &PickerLast, cx| {
            if access(view).select_last() {
                cx.invalidate();
            }
        });
        let confirm_activate = activate.clone();
        let confirm = cx.action_listener(id, move |view, _: &PickerConfirm, cx| {
            let value = access(view).selected_value().cloned();
            if let Some(value) = value {
                confirm_activate(view, value, cx);
            }
        });

        let mut rows = Vec::with_capacity(self.list.visible_rows().len());
        for result_index in self.list.visible_rows().range {
            let entry = &self.matches[result_index];
            let item = &self.items[entry.source_index];
            let source_index = entry.source_index;
            let row_id = derived_picker_id(id, ROW_ID_TAG, source_index as u64);
            let selected = self.selected_result == Some(result_index);
            let disabled = item.disabled;
            let label = if entry.label_ranges.is_empty() {
                text(item.label.clone())
            } else {
                StyledText::new(item.label.clone())
                    .with_highlights(entry.label_ranges.iter().cloned().map(|range| {
                        (
                            range,
                            HighlightStyle::default()
                                .color(style.matched_text)
                                .font_semibold(),
                        )
                    }))
                    .into_element()
            }
            .text_sm()
            .font_medium()
            .no_wrap()
            .text_color(if disabled {
                style.muted_text
            } else {
                style.text
            });

            let mut label_stack = div().min_w(0.0).flex_1().flex_col().child(label);
            if let Some(detail) = &item.detail {
                label_stack = label_stack.child(
                    text(detail.clone())
                        .text_xs()
                        .no_wrap()
                        .text_color(style.muted_text),
                );
            }

            let mut row = button()
                .id(row_id)
                .disabled(disabled)
                .selected(selected)
                .accessibility_role(AccessibilityRole::MenuItem)
                .accessibility_label(item.label.clone())
                .absolute()
                .top(result_index as f32 * style.row_height - self.list.scroll_offset())
                .left(0.0)
                .w_full()
                .h(style.row_height)
                .flex_row()
                .flex_none()
                .items_center()
                .gap_3()
                .px_3()
                .bg(if selected {
                    style.selection
                } else {
                    Color::TRANSPARENT
                })
                .hover(|hover| {
                    hover.bg(if selected {
                        style.selection_hover
                    } else {
                        style.row_hover
                    })
                })
                .child(label_stack);
            if let Some(shortcut) = &item.shortcut {
                row = row.child(
                    text(shortcut.clone())
                        .text_xs()
                        .no_wrap()
                        .text_color(style.muted_text),
                );
            }
            if !disabled {
                let clicked_value = item.value.clone();
                let clicked_activate = activate.clone();
                let clicked = cx.listener(row_id, move |view, cx| {
                    access(view).select_source(source_index);
                    clicked_activate(view, clicked_value.clone(), cx);
                });
                row = row.on_click(clicked);
            }
            rows.push(row);
        }

        let results = if self.matches.is_empty() {
            div()
                .h(style.row_height)
                .w_full()
                .flex_row()
                .items_center()
                .px_3()
                .accessibility_role(AccessibilityRole::List)
                .child(
                    text(style.no_matches_text.clone())
                        .text_sm()
                        .text_color(style.muted_text),
                )
        } else {
            div()
                .relative()
                .h(results_height)
                .w_full()
                .overflow_hidden()
                .virtual_scroll(&self.list)
                .accessibility_role(AccessibilityRole::List)
                .accessibility_label("Picker results")
                .children(rows)
        };

        let status = if self.results_truncated() {
            format!(
                "Showing {} of {} matches",
                self.matches.len(),
                self.total_match_count
            )
        } else {
            format!(
                "{} match{}",
                self.total_match_count,
                if self.total_match_count == 1 {
                    ""
                } else {
                    "es"
                }
            )
        };

        div()
            .focus_scope(FocusHandle::new(id))
            .key_context(PICKER_KEY_CONTEXT)
            .on_action(previous)
            .on_action(next)
            .on_action(page_up)
            .on_action(page_down)
            .on_action(first)
            .on_action(last)
            .on_action(confirm)
            .accessibility_role(AccessibilityRole::Dialog)
            .accessibility_label("Command palette")
            .w(style.width)
            .flex_col()
            .overflow_hidden()
            .rounded_xl()
            .border(1.0, style.border)
            .shadow_xl()
            .bg(style.background)
            .text_color(style.text)
            .child(
                text_input(self.query.clone())
                    .track_focus(input_focus)
                    .auto_focus()
                    .on_input(query)
                    .max_length(MAX_PICKER_QUERY_GRAPHEMES)
                    .placeholder(style.placeholder.clone())
                    .h(48.0)
                    .w_full()
                    .px_3()
                    .border(1.0, style.border)
                    .focus(|focus| focus.border(2.0, style.matched_text))
                    .bg(style.input_background),
            )
            .child(results)
            .child(
                div()
                    .h(24.0)
                    .w_full()
                    .flex_row()
                    .items_center()
                    .px_3()
                    .child(
                        text(status)
                            .text_xs()
                            .no_wrap()
                            .text_color(style.muted_text),
                    ),
            )
    }

    fn rebuild_matches(&mut self) {
        let normalized_query = normalized_query(&self.query);
        self.score_scratch.clear();
        if normalized_query.is_empty() {
            self.total_match_count = self.items.len();
            self.matches.clear();
            self.matches
                .extend(
                    (0..self.items.len().min(MAX_PICKER_RESULTS)).map(|source_index| MatchEntry {
                        source_index,
                        score: 0,
                        label_ranges: Box::new([]),
                    }),
                );
        } else {
            for (source_index, item) in self.items.iter().enumerate() {
                if let Some(score) = fuzzy_match(item, &normalized_query, None) {
                    self.score_scratch.push((score, source_index));
                }
            }
            self.total_match_count = self.score_scratch.len();
            let result_limit = self.score_scratch.len().min(MAX_PICKER_RESULTS);
            if self.score_scratch.len() > result_limit {
                let items = &self.items;
                self.score_scratch
                    .select_nth_unstable_by(result_limit, |left, right| {
                        compare_ranked(items, left, right)
                    });
                self.score_scratch.truncate(result_limit);
            }
            let items = &self.items;
            self.score_scratch
                .sort_unstable_by(|left, right| compare_ranked(items, left, right));
            self.matches.clear();
            self.matches.reserve(
                self.score_scratch
                    .len()
                    .min(MAX_PICKER_RESULTS)
                    .saturating_sub(self.matches.capacity()),
            );
            for &(score, source_index) in &self.score_scratch {
                let mut ranges = Vec::with_capacity(normalized_query.len().min(16));
                let recomputed = fuzzy_match(
                    &self.items[source_index],
                    &normalized_query,
                    Some(&mut ranges),
                );
                debug_assert_eq!(recomputed, Some(score));
                self.matches.push(MatchEntry {
                    source_index,
                    score,
                    label_ranges: ranges.into_boxed_slice(),
                });
            }
        }
        self.list.set_len(self.matches.len());
        self.list.scroll_to(0.0);
        self.selected_result =
            (0..self.matches.len()).find(|index| self.result_is_selectable(*index));
    }

    fn result_is_selectable(&self, result_index: usize) -> bool {
        self.matches
            .get(result_index)
            .is_some_and(|entry| !self.items[entry.source_index].disabled)
    }

    fn next_selectable(&self, forward: bool) -> Option<usize> {
        if self.matches.is_empty() {
            return None;
        }
        let len = self.matches.len();
        let start = match (self.selected_result, forward) {
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
            .find(|index| self.result_is_selectable(*index))
    }

    fn select_page(&mut self, forward: bool) -> bool {
        if self.matches.is_empty() {
            return false;
        }
        let page = (self.list.viewport_height() / self.style.row_height)
            .floor()
            .max(1.0) as usize;
        let current =
            self.selected_result
                .unwrap_or(if forward { 0 } else { self.matches.len() - 1 });
        let target = if forward {
            current.saturating_add(page).min(self.matches.len() - 1)
        } else {
            current.saturating_sub(page)
        };
        let candidates: Box<dyn Iterator<Item = usize>> = if forward {
            Box::new((target..self.matches.len()).chain(0..target))
        } else {
            Box::new(
                (0..=target)
                    .rev()
                    .chain((target + 1..self.matches.len()).rev()),
            )
        };
        let Some(result) = candidates
            .into_iter()
            .find(|index| self.result_is_selectable(*index))
        else {
            return false;
        };
        self.select_result(result)
    }
}

fn compare_ranked<T>(
    items: &[PickerItem<T>],
    left: &(i32, usize),
    right: &(i32, usize),
) -> Ordering {
    right
        .0
        .cmp(&left.0)
        .then_with(|| items[left.1].label.len().cmp(&items[right.1].label.len()))
        .then_with(|| left.1.cmp(&right.1))
}

fn collect_picker_items<T>(
    items: impl IntoIterator<Item = PickerItem<T>>,
) -> Result<Vec<PickerItem<T>>, PickerError> {
    let iterator = items.into_iter();
    let mut collected = Vec::with_capacity(iterator.size_hint().0.min(MAX_PICKER_ITEMS));
    let mut total_bytes = 0usize;
    for (index, item) in iterator.enumerate() {
        if index == MAX_PICKER_ITEMS {
            return Err(PickerError::TooManyItems {
                limit: MAX_PICKER_ITEMS,
            });
        }
        let bytes = picker_item_text_bytes(&item);
        if bytes > MAX_PICKER_ITEM_TEXT_BYTES {
            return Err(PickerError::ItemTextTooLong {
                index,
                bytes,
                limit: MAX_PICKER_ITEM_TEXT_BYTES,
            });
        }
        total_bytes = total_bytes.saturating_add(bytes);
        if total_bytes > MAX_PICKER_TEXT_BYTES {
            return Err(PickerError::TextBudgetExceeded {
                bytes: total_bytes,
                limit: MAX_PICKER_TEXT_BYTES,
            });
        }
        collected.push(item);
    }
    Ok(collected)
}

fn picker_item_text_bytes<T>(item: &PickerItem<T>) -> usize {
    item.label
        .len()
        .saturating_add(item.detail.as_ref().map_or(0, |detail| detail.len()))
        .saturating_add(item.keywords.len())
        .saturating_add(item.shortcut.as_ref().map_or(0, |shortcut| shortcut.len()))
}

fn bounded_query(query: &str) -> &str {
    let mut byte_end = query.len().min(MAX_PICKER_QUERY_BYTES);
    while !query.is_char_boundary(byte_end) {
        byte_end -= 1;
    }
    let query = &query[..byte_end];
    query
        .grapheme_indices(true)
        .nth(MAX_PICKER_QUERY_GRAPHEMES)
        .map_or(query, |(offset, _)| &query[..offset])
}

fn normalized_query(query: &str) -> Vec<char> {
    let mut normalized = Vec::with_capacity(query.len().min(MAX_NORMALIZED_QUERY_CHARS));
    let mut previous_space = true;
    for character in query.trim().chars() {
        for lower in character.to_lowercase() {
            let lower = if lower.is_whitespace() { ' ' } else { lower };
            if lower == ' ' && previous_space {
                continue;
            }
            normalized.push(lower);
            previous_space = lower == ' ';
            if normalized.len() == MAX_NORMALIZED_QUERY_CHARS {
                return normalized;
            }
        }
    }
    if normalized.last() == Some(&' ') {
        normalized.pop();
    }
    normalized
}

fn fuzzy_match<T>(
    item: &PickerItem<T>,
    query: &[char],
    mut label_ranges: Option<&mut Vec<Range<usize>>>,
) -> Option<i32> {
    if query.is_empty() {
        return Some(0);
    }

    let segments = [
        (item.label.as_ref(), true),
        (item.detail.as_deref().unwrap_or(""), false),
        (item.keywords.as_ref(), false),
        (item.shortcut.as_deref().unwrap_or(""), false),
    ];
    let mut query_index = 0usize;
    let mut ordinal = 0i32;
    let mut previous_match = None;
    let mut score = 0i32;
    let mut first_match_at_label_start = false;
    let mut every_match_in_label = true;
    let mut every_match_consecutive = true;

    for (segment_index, (segment, is_label)) in segments.into_iter().enumerate() {
        if segment_index > 0 {
            if query[query_index] == ' ' {
                apply_fuzzy_match(
                    &mut score,
                    &mut previous_match,
                    &mut every_match_consecutive,
                    ordinal,
                    true,
                    false,
                );
                query_index += 1;
                if query_index == query.len() {
                    break;
                }
            }
            ordinal += 1;
        }
        let mut previous_was_alphanumeric = false;
        let mut previous_was_space = true;
        for (byte_index, character) in segment.char_indices() {
            let boundary = !previous_was_alphanumeric;
            let original_range = byte_index..byte_index + character.len_utf8();
            let mut expansion_index = 0usize;
            for lower in character.to_lowercase() {
                let lower = if lower.is_whitespace() { ' ' } else { lower };
                if lower == ' ' && previous_was_space {
                    continue;
                }
                previous_was_space = lower == ' ';
                if query[query_index] == lower {
                    if query_index == 0 {
                        first_match_at_label_start = is_label && byte_index == 0;
                    }
                    every_match_in_label &= is_label;
                    apply_fuzzy_match(
                        &mut score,
                        &mut previous_match,
                        &mut every_match_consecutive,
                        ordinal,
                        boundary && expansion_index == 0,
                        is_label,
                    );
                    if is_label && let Some(ranges) = label_ranges.as_deref_mut() {
                        push_coalesced_range(ranges, original_range.clone());
                    }
                    query_index += 1;
                    if query_index == query.len() {
                        break;
                    }
                }
                ordinal += 1;
                expansion_index += 1;
            }
            previous_was_alphanumeric = character.is_alphanumeric();
            if query_index == query.len() {
                break;
            }
        }
        if query_index == query.len() {
            break;
        }
    }

    if query_index != query.len() {
        return None;
    }
    if first_match_at_label_start {
        score += 420;
    }
    if every_match_in_label {
        score += 180;
    }
    if every_match_consecutive {
        score += 220;
    }
    score -= i32::try_from(item.label.chars().count().min(255)).unwrap_or(255);
    Some(score)
}

fn apply_fuzzy_match(
    score: &mut i32,
    previous_match: &mut Option<i32>,
    every_match_consecutive: &mut bool,
    ordinal: i32,
    boundary: bool,
    in_label: bool,
) {
    *score += 1_000;
    if boundary {
        *score += 110;
    }
    if in_label {
        *score += 55;
    } else {
        *score -= 35;
    }
    if let Some(previous) = *previous_match {
        let gap = ordinal.saturating_sub(previous + 1);
        if gap == 0 {
            *score += 150;
        } else {
            *score -= gap.min(64) * 7;
            *every_match_consecutive = false;
        }
    } else {
        *score -= ordinal.min(128) * 3;
    }
    *previous_match = Some(ordinal);
}

fn push_coalesced_range(ranges: &mut Vec<Range<usize>>, range: Range<usize>) {
    if let Some(previous) = ranges.last_mut()
        && range.start <= previous.end
    {
        previous.end = previous.end.max(range.end);
        return;
    }
    ranges.push(range);
}

fn derived_picker_id(parent: ElementId, tag: u64, value: u64) -> ElementId {
    let mut hash = parent.as_u64() ^ tag ^ value.wrapping_mul(0x9e37_79b9_7f4a_7c15);
    hash ^= hash >> 30;
    hash = hash.wrapping_mul(0xbf58_476d_1ce4_e5b9);
    hash ^= hash >> 27;
    hash = hash.wrapping_mul(0x94d0_49bb_1331_11eb);
    hash ^= hash >> 31;
    if hash == parent.as_u64() {
        hash ^= tag;
    }
    ElementId::new(hash)
}

fn finite_at_least(value: f32, minimum: f32, fallback: f32) -> f32 {
    if value.is_finite() {
        value.max(minimum)
    } else {
        fallback
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn item(label: &str) -> PickerItem<usize> {
        PickerItem::new(label, 0)
    }

    #[test]
    fn fuzzy_matching_ranks_prefixes_and_retains_unicode_safe_label_ranges() {
        let mut picker = PickerState::new([
            item("Close File"),
            item("Open File"),
            item("Öffnen").keywords("open file"),
        ])
        .unwrap();

        assert!(picker.set_query("of"));
        let first = picker.match_at(0).unwrap();
        assert_eq!(first.item().label().as_ref(), "Open File");
        assert_eq!(first.label_ranges(), [0..1, 5..6]);

        picker.set_query("öff");
        let first = picker.match_at(0).unwrap();
        assert_eq!(first.item().label().as_ref(), "Öffnen");
        assert_eq!(first.label_ranges().len(), 1);
        assert_eq!(first.label_ranges()[0], 0..4);
        assert!(
            first.label_ranges().iter().all(|range| first
                .item()
                .label()
                .is_char_boundary(range.start)
                && first.item().label().is_char_boundary(range.end))
        );
    }

    #[test]
    fn metadata_matches_without_manufacturing_label_highlights() {
        let mut picker = PickerState::new([
            item("Preferences").keywords("settings xyzalias"),
            item("Settings Sync"),
        ])
        .unwrap();
        picker.set_query("xyzalias");

        assert_eq!(picker.result_count(), 1);
        assert_eq!(
            picker.match_at(0).unwrap().item().label().as_ref(),
            "Preferences"
        );
        assert!(picker.match_at(0).unwrap().label_ranges().is_empty());
    }

    #[test]
    fn navigation_skips_disabled_results_wraps_and_reveals_selection() {
        let items = (0..20).map(|index| {
            PickerItem::new(format!("Command {index:02}"), index).disabled(index == 1)
        });
        let mut picker = PickerState::new(items).unwrap();
        picker
            .list
            .set_viewport_height(picker.style.row_height * 3.0);

        assert_eq!(picker.selected_result_index(), Some(0));
        assert!(picker.select_next());
        assert_eq!(picker.selected_result_index(), Some(2));
        assert!(picker.select_last());
        assert_eq!(picker.selected_value(), Some(&19));
        assert!(picker.list.scroll_offset() > 0.0);
        assert!(picker.select_next());
        assert_eq!(picker.selected_result_index(), Some(0));
        assert_eq!(picker.list.scroll_offset(), 0.0);
    }

    #[test]
    fn reset_restores_empty_query_source_order_and_initial_selection() {
        let mut picker = PickerState::new([
            PickerItem::new("Alpha", 1),
            PickerItem::new("Beta", 2),
            PickerItem::new("Gamma", 3),
        ])
        .unwrap();
        picker.list.set_viewport_height(picker.style.row_height);
        picker.set_query("gamma");
        assert_eq!(picker.selected_value(), Some(&3));

        assert!(picker.reset());
        assert!(picker.query().is_empty());
        assert_eq!(picker.match_at(0).unwrap().item().label().as_ref(), "Alpha");
        assert_eq!(picker.selected_value(), Some(&1));
        assert_eq!(picker.list.scroll_offset(), 0.0);
    }

    #[test]
    fn queries_and_ranked_results_are_hard_bounded() {
        let items = (0..MAX_PICKER_RESULTS + 10)
            .map(|index| PickerItem::new(format!("command {index}"), index));
        let mut picker = PickerState::new(items).unwrap();
        assert_eq!(picker.result_count(), MAX_PICKER_RESULTS);
        assert_eq!(picker.total_match_count(), MAX_PICKER_RESULTS + 10);
        assert!(picker.results_truncated());

        let query = "🦀".repeat(MAX_PICKER_QUERY_GRAPHEMES + 10);
        picker.set_query(&query);
        assert_eq!(
            picker.query().graphemes(true).count(),
            MAX_PICKER_QUERY_GRAPHEMES
        );

        let pathological_grapheme = format!("a{}", "\u{301}".repeat(MAX_PICKER_QUERY_BYTES));
        picker.set_query(&pathological_grapheme);
        assert!(picker.query().len() <= MAX_PICKER_QUERY_BYTES);
        assert!(pathological_grapheme.is_char_boundary(picker.query().len()));
    }

    #[test]
    fn source_item_and_text_budgets_fail_before_unbounded_retention() {
        let too_many = (0..=MAX_PICKER_ITEMS).map(|index| PickerItem::new("x", index));
        assert_eq!(
            PickerState::new(too_many).unwrap_err(),
            PickerError::TooManyItems {
                limit: MAX_PICKER_ITEMS
            }
        );

        let oversized = "x".repeat(MAX_PICKER_ITEM_TEXT_BYTES + 1);
        assert_eq!(
            PickerState::new([PickerItem::new(oversized, ())]).unwrap_err(),
            PickerError::ItemTextTooLong {
                index: 0,
                bytes: MAX_PICKER_ITEM_TEXT_BYTES + 1,
                limit: MAX_PICKER_ITEM_TEXT_BYTES,
            }
        );
    }

    #[test]
    fn picker_bindings_are_scoped_and_cover_navigation_and_confirmation() {
        let bindings = picker_key_bindings();
        assert_eq!(bindings.len(), 7);
        assert!(
            bindings.iter().all(
                |binding| binding.context_predicate().is_some_and(|context| context
                    .depth_of(&[crate::KeyContext::parse("Picker").unwrap()])
                    .is_some())
            )
        );
        assert!(
            bindings
                .iter()
                .any(|binding| binding.action().downcast_ref::<PickerConfirm>().is_some())
        );
    }

    #[test]
    fn style_dimensions_are_sanitized_without_unbounded_visible_mounts() {
        let style = PickerStyle::default()
            .width(f32::NAN)
            .row_height(-20.0)
            .max_visible_rows(usize::MAX);
        assert_eq!(style.width, 560.0);
        assert_eq!(style.row_height, 24.0);
        assert_eq!(style.max_visible_rows, MAX_VISIBLE_PICKER_ROWS);
    }
}
