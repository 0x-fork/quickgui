use std::{
    borrow::Cow,
    collections::VecDeque,
    mem::size_of,
    ops::Range,
    sync::Arc,
    time::{Duration, Instant},
};

use unicode_segmentation::UnicodeSegmentation;

use crate::{
    FontFamily, TextHighlight, element::InputConstraints, styled_text::MAX_TEXT_HIGHLIGHTS,
};

#[derive(Clone, Debug)]
pub(crate) struct TextInputState {
    text: Arc<str>,
    highlights: Arc<[TextHighlight]>,
    multiline: bool,
    constraints: InputConstraints,
    anchor: usize,
    caret: usize,
    marked: Option<Range<usize>>,
    composition_backup: Option<CompositionBackup>,
    preferred_x: Option<f32>,
    undo: VecDeque<EditSnapshot>,
    redo: VecDeque<EditSnapshot>,
    undo_bytes: usize,
    redo_bytes: usize,
    edit_group: Option<EditGroup>,
}

#[derive(Clone, Debug)]
struct CompositionBackup {
    text: Arc<str>,
    highlights: Arc<[TextHighlight]>,
    anchor: usize,
    caret: usize,
}

#[derive(Clone, Debug)]
struct EditSnapshot {
    text: Arc<str>,
    highlights: Arc<[TextHighlight]>,
    anchor: usize,
    caret: usize,
}

impl EditSnapshot {
    fn retained_bytes(&self) -> usize {
        self.text
            .len()
            .saturating_add(
                self.highlights
                    .len()
                    .saturating_mul(size_of::<TextHighlight>()),
            )
            .saturating_add(self.highlights.iter().fold(0_usize, |bytes, highlight| {
                let family_bytes = match highlight.style.family.as_ref() {
                    Some(FontFamily::Named(name)) => name.len(),
                    Some(FontFamily::SansSerif | FontFamily::Serif | FontFamily::Monospace)
                    | None => 0,
                };
                bytes.saturating_add(family_bytes)
            }))
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum EditKind {
    Typing,
    Backspace,
    Delete,
}

#[derive(Clone, Copy, Debug)]
struct EditGroup {
    kind: EditKind,
    caret_after: usize,
    last_edit_at: Instant,
}

const MAX_HISTORY_ENTRIES: usize = 100;
const MAX_HISTORY_BYTES_PER_STACK: usize = 512 * 1024;
const EDIT_COALESCE_WINDOW: Duration = Duration::from_secs(1);

impl TextInputState {
    #[cfg(test)]
    pub fn new(value: &str) -> Self {
        Self::with_mode(value, false)
    }

    #[cfg(test)]
    pub fn with_mode(value: &str, multiline: bool) -> Self {
        Self::with_constraints(value, multiline, InputConstraints::default())
    }

    #[cfg(test)]
    pub(crate) fn with_constraints(
        value: &str,
        multiline: bool,
        constraints: InputConstraints,
    ) -> Self {
        Self::with_styling(value, multiline, constraints, Arc::from([]))
    }

    pub(crate) fn with_styling(
        value: &str,
        multiline: bool,
        constraints: InputConstraints,
        highlights: Arc<[TextHighlight]>,
    ) -> Self {
        let normalized = normalize_text(value, multiline);
        let highlights = if matches!(&normalized, Cow::Borrowed(_)) {
            highlights
        } else {
            normalize_highlights(value, &highlights)
        };
        Self {
            text: Arc::from(normalized.as_ref()),
            highlights,
            multiline,
            constraints,
            anchor: normalized.len(),
            caret: normalized.len(),
            marked: None,
            composition_backup: None,
            preferred_x: None,
            undo: VecDeque::new(),
            redo: VecDeque::new(),
            undo_bytes: 0,
            redo_bytes: 0,
            edit_group: None,
        }
    }

    pub fn text(&self) -> &str {
        &self.text
    }

    pub fn is_multiline(&self) -> bool {
        self.multiline
    }

    pub fn shared_text(&self) -> Arc<str> {
        self.text.clone()
    }

    pub fn shared_highlights(&self) -> Arc<[TextHighlight]> {
        self.highlights.clone()
    }

    /// The application-visible value, excluding an uncommitted IME preedit.
    pub fn committed_shared_text(&self) -> Arc<str> {
        self.composition_backup
            .as_ref()
            .map(|backup| backup.text.clone())
            .unwrap_or_else(|| self.text.clone())
    }

    pub fn selection(&self) -> Range<usize> {
        self.anchor.min(self.caret)..self.anchor.max(self.caret)
    }

    pub fn caret(&self) -> usize {
        self.caret
    }

    pub fn anchor(&self) -> usize {
        self.anchor
    }

    pub fn marked(&self) -> Option<Range<usize>> {
        self.marked.clone()
    }

    pub fn preferred_x(&self) -> Option<f32> {
        self.preferred_x
    }

    pub fn set_preferred_x(&mut self, value: Option<f32>) {
        self.preferred_x = value.filter(|value| value.is_finite());
    }

    #[cfg(test)]
    pub fn sync_external(&mut self, value: &str, multiline: bool, constraints: &InputConstraints) {
        self.sync_external_styled(value, multiline, constraints, &Arc::from([]));
    }

    pub fn sync_external_styled(
        &mut self,
        value: &str,
        multiline: bool,
        constraints: &InputConstraints,
        highlights: &Arc<[TextHighlight]>,
    ) {
        self.constraints = constraints.clone();
        let normalized = normalize_text(value, multiline);
        let highlights = if matches!(&normalized, Cow::Borrowed(_)) {
            highlights.clone()
        } else {
            normalize_highlights(value, highlights)
        };
        if self.multiline == multiline && self.text.as_ref() == normalized {
            self.highlights = highlights;
            return;
        }
        if self.multiline == multiline
            && self
                .composition_backup
                .as_ref()
                .is_some_and(|backup| backup.text.as_ref() == normalized)
        {
            let visible_highlights = self
                .composition_backup
                .as_ref()
                .map(|backup| {
                    let replaced = backup.anchor.min(backup.caret)..backup.anchor.max(backup.caret);
                    let inserted_len = self.marked.as_ref().map_or(0, Range::len);
                    replace_highlights(&highlights, replaced, inserted_len, self.text.len())
                })
                .unwrap_or_else(|| highlights.clone());
            self.highlights = visible_highlights;
            if let Some(backup) = &mut self.composition_backup {
                backup.highlights = highlights;
            }
            return;
        }
        self.text = Arc::from(normalized.as_ref());
        self.highlights = highlights;
        self.multiline = multiline;
        self.anchor = normalized.len();
        self.caret = normalized.len();
        self.marked = None;
        self.composition_backup = None;
        self.preferred_x = None;
        self.clear_history();
    }

    pub fn move_left(&mut self, extend: bool) -> bool {
        let selection = self.selection();
        let next = if !extend && !selection.is_empty() {
            selection.start
        } else {
            previous_boundary(&self.text, self.caret)
        };
        self.move_to(next, extend)
    }

    pub fn move_right(&mut self, extend: bool) -> bool {
        let selection = self.selection();
        let next = if !extend && !selection.is_empty() {
            selection.end
        } else {
            next_boundary(&self.text, self.caret)
        };
        self.move_to(next, extend)
    }

    pub fn move_word_left(&mut self, extend: bool) -> bool {
        let selection = self.selection();
        let next = if !extend && !selection.is_empty() {
            selection.start
        } else {
            previous_word_boundary(&self.text, self.caret)
        };
        self.move_to(next, extend)
    }

    pub fn move_word_right(&mut self, extend: bool) -> bool {
        let selection = self.selection();
        let next = if !extend && !selection.is_empty() {
            selection.end
        } else {
            next_word_boundary(&self.text, self.caret)
        };
        self.move_to(next, extend)
    }

    pub fn move_home(&mut self, extend: bool) -> bool {
        self.move_to(0, extend)
    }

    pub fn move_end(&mut self, extend: bool) -> bool {
        self.move_to(self.text.len(), extend)
    }

    pub fn move_line_start(&mut self, extend: bool) -> bool {
        self.move_to(line_start(&self.text, self.caret), extend)
    }

    pub fn move_line_end(&mut self, extend: bool) -> bool {
        self.move_to(line_end(&self.text, self.caret), extend)
    }

    pub fn move_to(&mut self, index: usize, extend: bool) -> bool {
        self.edit_group = None;
        let index = boundary_at_or_before(&self.text, index.min(self.text.len()));
        let changed = self.caret != index || (!extend && self.anchor != index);
        self.caret = index;
        if !extend {
            self.anchor = index;
        }
        self.marked = None;
        self.composition_backup = None;
        self.preferred_x = None;
        changed
    }

    pub fn move_to_with_preferred_x(
        &mut self,
        index: usize,
        extend: bool,
        preferred_x: f32,
    ) -> bool {
        let changed = self.move_to(index, extend);
        self.set_preferred_x(Some(preferred_x));
        changed
    }

    pub fn select_all(&mut self) -> bool {
        self.edit_group = None;
        let changed = self.anchor != 0 || self.caret != self.text.len();
        self.anchor = 0;
        self.caret = self.text.len();
        self.marked = None;
        self.composition_backup = None;
        self.preferred_x = None;
        changed
    }

    pub fn selected_text(&self) -> Option<&str> {
        let selection = self.selection();
        (!selection.is_empty()).then(|| &self.text[selection])
    }

    pub fn set_selection(&mut self, anchor: usize, caret: usize) -> bool {
        self.edit_group = None;
        let anchor = boundary_at_or_before(&self.text, anchor.min(self.text.len()));
        let caret = boundary_at_or_before(&self.text, caret.min(self.text.len()));
        let changed = self.anchor != anchor || self.caret != caret || self.marked.is_some();
        self.anchor = anchor;
        self.caret = caret;
        self.marked = None;
        self.composition_backup = None;
        self.preferred_x = None;
        changed
    }

    pub fn set_value(&mut self, value: &str) -> bool {
        let value = normalize_text(value, self.multiline);
        self.replace_range(0..self.text.len(), &value, None)
    }

    pub fn accessibility_character_lengths(&self) -> Vec<u8> {
        selectable_character_lengths(&self.text)
    }

    pub fn accessibility_character_index(&self, byte_index: usize) -> usize {
        accessibility_character_index(&self.text, byte_index)
    }

    pub fn accessibility_byte_index(&self, character_index: usize) -> usize {
        accessibility_byte_index(&self.text, character_index)
    }

    pub fn backspace(&mut self) -> bool {
        let selection = self.selection();
        if selection.is_empty() {
            let start = previous_boundary(&self.text, self.caret);
            if start == self.caret {
                return false;
            }
            self.replace_range(start..self.caret, "", Some(EditKind::Backspace))
        } else {
            self.replace_range(selection, "", None)
        }
    }

    pub fn delete(&mut self) -> bool {
        let selection = self.selection();
        if selection.is_empty() {
            let end = next_boundary(&self.text, self.caret);
            if end == self.caret {
                return false;
            }
            self.replace_range(self.caret..end, "", Some(EditKind::Delete))
        } else {
            self.replace_range(selection, "", None)
        }
    }

    pub fn delete_word_backward(&mut self) -> bool {
        let selection = self.selection();
        let range = if selection.is_empty() {
            previous_word_boundary(&self.text, self.caret)..self.caret
        } else {
            selection
        };
        if range.is_empty() {
            return false;
        }
        self.replace_range(range, "", None)
    }

    pub fn delete_word_forward(&mut self) -> bool {
        let selection = self.selection();
        let range = if selection.is_empty() {
            self.caret..next_word_boundary(&self.text, self.caret)
        } else {
            selection
        };
        if range.is_empty() {
            return false;
        }
        self.replace_range(range, "", None)
    }

    pub fn delete_to_line_start(&mut self) -> bool {
        let selection = self.selection();
        let range = if selection.is_empty() {
            let mut start = line_start(&self.text, self.caret);
            if start == self.caret {
                start = previous_boundary(&self.text, self.caret);
            }
            start..self.caret
        } else {
            selection
        };
        if range.is_empty() {
            return false;
        }
        self.replace_range(range, "", None)
    }

    pub fn delete_to_line_end(&mut self) -> bool {
        let selection = self.selection();
        let range = if selection.is_empty() {
            let mut end = line_end(&self.text, self.caret);
            if end == self.caret {
                end = next_boundary(&self.text, self.caret);
            }
            self.caret..end
        } else {
            selection
        };
        if range.is_empty() {
            return false;
        }
        self.replace_range(range, "", None)
    }

    pub fn replace_selection(&mut self, value: &str) -> bool {
        let value = normalize_text(value, self.multiline);
        if value.is_empty() && self.selection().is_empty() && self.marked.is_none() {
            return false;
        }
        let range = self.marked.clone().unwrap_or_else(|| self.selection());
        let kind = (self.marked.is_none() && value.graphemes(true).count() == 1)
            .then_some(EditKind::Typing);
        self.replace_range(range, &value, kind)
    }

    pub fn insert_newline(&mut self) -> bool {
        self.multiline && self.replace_selection("\n")
    }

    pub fn set_preedit(&mut self, value: &str, cursor: Option<(usize, usize)>) -> bool {
        self.edit_group = None;
        let value = normalize_text(value, self.multiline);
        let previous_text = self.text.clone();
        let previous_anchor = self.anchor;
        let previous_caret = self.caret;
        let previous_marked = self.marked.clone();
        let previous_backup = self.composition_backup.clone();

        if value.is_empty() {
            if let Some(backup) = self.composition_backup.take() {
                self.text = backup.text;
                self.highlights = backup.highlights;
                self.anchor = backup.anchor;
                self.caret = backup.caret;
            }
            self.marked = None;
            return self.text != previous_text
                || self.anchor != previous_anchor
                || self.caret != previous_caret
                || self.marked != previous_marked;
        }

        let backup = self
            .composition_backup
            .take()
            .unwrap_or_else(|| CompositionBackup {
                text: self.text.clone(),
                highlights: self.highlights.clone(),
                anchor: self.anchor,
                caret: self.caret,
            });
        let range = self.marked.take().unwrap_or_else(|| self.selection());
        let start = range.start;
        self.replace_range_unchecked(range, &value);
        self.composition_backup = Some(backup);

        let end = start + value.len();
        self.marked = Some(start..end);
        if let Some((cursor_start, cursor_end)) = cursor {
            let local_start = boundary_at_or_before(&value, cursor_start.min(value.len()));
            let local_end = boundary_at_or_before(&value, cursor_end.min(value.len()));
            self.anchor = start + local_start;
            self.caret = start + local_end;
        } else {
            self.anchor = end;
            self.caret = end;
        }
        self.text != previous_text
            || self.anchor != previous_anchor
            || self.caret != previous_caret
            || self.marked != previous_marked
            || self.composition_backup.is_some() != previous_backup.is_some()
    }

    pub fn can_undo(&self) -> bool {
        self.undo
            .back()
            .is_some_and(|snapshot| self.accepts_existing(&snapshot.text))
    }

    pub fn can_redo(&self) -> bool {
        self.redo
            .back()
            .is_some_and(|snapshot| self.accepts_existing(&snapshot.text))
    }

    pub fn undo(&mut self) -> bool {
        self.edit_group = None;
        if !self.can_undo() {
            return false;
        }
        let Some(snapshot) = self.undo.pop_back() else {
            return false;
        };
        self.undo_bytes = self.undo_bytes.saturating_sub(snapshot.retained_bytes());
        let current = self.snapshot();
        push_bounded_history(&mut self.redo, &mut self.redo_bytes, current);
        self.restore(snapshot);
        true
    }

    pub fn redo(&mut self) -> bool {
        self.edit_group = None;
        if !self.can_redo() {
            return false;
        }
        let Some(snapshot) = self.redo.pop_back() else {
            return false;
        };
        self.redo_bytes = self.redo_bytes.saturating_sub(snapshot.retained_bytes());
        let current = self.snapshot();
        push_bounded_history(&mut self.undo, &mut self.undo_bytes, current);
        self.restore(snapshot);
        true
    }

    fn snapshot(&self) -> EditSnapshot {
        self.composition_backup
            .as_ref()
            .map(|backup| EditSnapshot {
                text: backup.text.clone(),
                highlights: backup.highlights.clone(),
                anchor: backup.anchor,
                caret: backup.caret,
            })
            .unwrap_or_else(|| EditSnapshot {
                text: self.text.clone(),
                highlights: self.highlights.clone(),
                anchor: self.anchor,
                caret: self.caret,
            })
    }

    fn record_edit(&mut self, kind: Option<EditKind>, caret_after: usize) {
        let now = Instant::now();
        let coalesced = kind.is_some_and(|kind| {
            self.edit_group.is_some_and(|group| {
                group.kind == kind
                    && group.caret_after == self.caret
                    && now.saturating_duration_since(group.last_edit_at) <= EDIT_COALESCE_WINDOW
            })
        });
        if !coalesced {
            let snapshot = self.snapshot();
            push_bounded_history(&mut self.undo, &mut self.undo_bytes, snapshot);
            self.redo.clear();
            self.redo_bytes = 0;
        }
        self.edit_group = kind.map(|kind| EditGroup {
            kind,
            caret_after,
            last_edit_at: now,
        });
    }

    fn clear_history(&mut self) {
        self.undo.clear();
        self.redo.clear();
        self.undo_bytes = 0;
        self.redo_bytes = 0;
        self.edit_group = None;
    }

    fn restore(&mut self, snapshot: EditSnapshot) {
        self.text = snapshot.text;
        self.highlights = snapshot.highlights;
        self.anchor = snapshot.anchor;
        self.caret = snapshot.caret;
        self.marked = None;
        self.composition_backup = None;
        self.preferred_x = None;
        self.edit_group = None;
    }

    fn replace_range(
        &mut self,
        range: Range<usize>,
        value: &str,
        edit_kind: Option<EditKind>,
    ) -> bool {
        let Some((text, end)) = self.constrained_replacement(range.clone(), value) else {
            self.edit_group = None;
            return self.cancel_composition();
        };
        let committed_before = self.snapshot().text;
        let changed = self.text.as_ref() != text
            || self.anchor != end
            || self.caret != end
            || self.marked.is_some()
            || self.composition_backup.is_some();
        if !changed {
            return false;
        }
        if committed_before.as_ref() != text {
            self.record_edit(edit_kind, end);
        }
        self.highlights = replace_highlights(
            &self.highlights,
            range.clone(),
            end.saturating_sub(range.start),
            text.len(),
        );
        self.text = Arc::from(text);
        self.anchor = end;
        self.caret = end;
        self.marked = None;
        self.composition_backup = None;
        self.preferred_x = None;
        true
    }

    fn constrained_replacement(&self, range: Range<usize>, value: &str) -> Option<(String, usize)> {
        debug_assert!(range.start <= range.end && range.end <= self.text.len());
        debug_assert!(self.text.is_char_boundary(range.start));
        debug_assert!(self.text.is_char_boundary(range.end));

        let value = if let Some(max_length) = self.constraints.max_length {
            let retained = self.text[..range.start].graphemes(true).count()
                + self.text[range.end..].graphemes(true).count();
            truncate_graphemes(value, max_length.saturating_sub(retained))
        } else {
            value
        };
        let mut text = self.text.to_string();
        text.replace_range(range.clone(), value);
        if self
            .constraints
            .filter
            .as_ref()
            .is_some_and(|filter| !filter(&text))
        {
            return None;
        }
        let end = range.start + value.len();
        Some((text, end))
    }

    fn replace_range_unchecked(&mut self, range: Range<usize>, value: &str) {
        let mut text = self.text.to_string();
        text.replace_range(range.clone(), value);
        self.highlights =
            replace_highlights(&self.highlights, range.clone(), value.len(), text.len());
        self.text = Arc::from(text);
        let end = range.start + value.len();
        self.anchor = end;
        self.caret = end;
        self.marked = None;
        self.composition_backup = None;
        self.preferred_x = None;
    }

    fn cancel_composition(&mut self) -> bool {
        let Some(backup) = self.composition_backup.take() else {
            return false;
        };
        let changed = self.text != backup.text
            || self.anchor != backup.anchor
            || self.caret != backup.caret
            || self.marked.is_some();
        self.text = backup.text;
        self.highlights = backup.highlights;
        self.anchor = backup.anchor;
        self.caret = backup.caret;
        self.marked = None;
        self.preferred_x = None;
        changed
    }

    fn accepts_existing(&self, value: &str) -> bool {
        self.constraints
            .max_length
            .is_none_or(|max_length| value.graphemes(true).count() <= max_length)
            && self
                .constraints
                .filter
                .as_ref()
                .is_none_or(|filter| filter(value))
    }
}

fn truncate_graphemes(value: &str, max_length: usize) -> &str {
    value
        .grapheme_indices(true)
        .nth(max_length)
        .map_or(value, |(end, _)| &value[..end])
}

fn push_bounded_history(
    history: &mut VecDeque<EditSnapshot>,
    retained_bytes: &mut usize,
    snapshot: EditSnapshot,
) {
    let bytes = snapshot.retained_bytes();
    if bytes > MAX_HISTORY_BYTES_PER_STACK {
        history.clear();
        *retained_bytes = 0;
        return;
    }
    while history.len() >= MAX_HISTORY_ENTRIES
        || retained_bytes.saturating_add(bytes) > MAX_HISTORY_BYTES_PER_STACK
    {
        let Some(evicted) = history.pop_front() else {
            break;
        };
        *retained_bytes = retained_bytes.saturating_sub(evicted.retained_bytes());
    }
    *retained_bytes += bytes;
    history.push_back(snapshot);
}

/// Shift, split, and merge attributed ranges after one accepted UTF-8 replacement.
///
/// Inserted text inherits a style only when it is inserted inside a run, or replaces text whose
/// first byte was styled. At run boundaries it remains unstyled until the controlled highlighter
/// supplies the next table. Runtime splits are truncated to the same hard cap as public input.
fn replace_highlights(
    highlights: &Arc<[TextHighlight]>,
    replaced: Range<usize>,
    inserted_len: usize,
    new_text_len: usize,
) -> Arc<[TextHighlight]> {
    if highlights.is_empty() {
        return highlights.clone();
    }

    let removed_len = replaced.end.saturating_sub(replaced.start);
    let shift = inserted_len as isize - removed_len as isize;
    let inherited = highlights
        .iter()
        .find(|highlight| {
            highlight.range.start <= replaced.start
                && replaced.start < highlight.range.end
                && (!replaced.is_empty() || highlight.range.start < replaced.start)
        })
        .map(|highlight| highlight.style.clone());
    let mut transformed = Vec::with_capacity(highlights.len().saturating_add(1));

    for highlight in highlights.iter() {
        if highlight.range.end <= replaced.start {
            transformed.push(highlight.clone());
            continue;
        }
        if highlight.range.start >= replaced.end {
            transformed.push(TextHighlight {
                range: shifted_offset(highlight.range.start, shift)
                    ..shifted_offset(highlight.range.end, shift),
                style: highlight.style.clone(),
            });
            continue;
        }
        if highlight.range.start < replaced.start {
            transformed.push(TextHighlight {
                range: highlight.range.start..replaced.start,
                style: highlight.style.clone(),
            });
        }
        if highlight.range.end > replaced.end {
            transformed.push(TextHighlight {
                range: replaced.start + inserted_len..shifted_offset(highlight.range.end, shift),
                style: highlight.style.clone(),
            });
        }
    }

    if inserted_len > 0
        && let Some(style) = inherited
    {
        transformed.push(TextHighlight {
            range: replaced.start..replaced.start + inserted_len,
            style,
        });
    }
    transformed.sort_by_key(|highlight| (highlight.range.start, highlight.range.end));

    let mut normalized: Vec<TextHighlight> = Vec::with_capacity(transformed.len());
    for mut highlight in transformed {
        highlight.range.end = highlight.range.end.min(new_text_len);
        if highlight.range.start >= highlight.range.end {
            continue;
        }
        if let Some(previous) = normalized.last_mut()
            && previous.range.end == highlight.range.start
            && previous.style == highlight.style
        {
            previous.range.end = highlight.range.end;
            continue;
        }
        if normalized.len() == MAX_TEXT_HIGHLIGHTS {
            break;
        }
        normalized.push(highlight);
    }
    normalized.into()
}

fn shifted_offset(offset: usize, shift: isize) -> usize {
    if shift >= 0 {
        offset.saturating_add(shift as usize)
    } else {
        offset.saturating_sub(shift.unsigned_abs())
    }
}

fn previous_boundary(text: &str, offset: usize) -> usize {
    text.grapheme_indices(true)
        .rev()
        .find_map(|(index, _)| (index < offset).then_some(index))
        .unwrap_or(0)
}

fn next_boundary(text: &str, offset: usize) -> usize {
    text.grapheme_indices(true)
        .find_map(|(index, _)| (index > offset).then_some(index))
        .unwrap_or(text.len())
}

pub(crate) fn boundary_at_or_before(text: &str, offset: usize) -> usize {
    if offset >= text.len() {
        return text.len();
    }
    text.grapheme_indices(true)
        .map(|(index, _)| index)
        .take_while(|index| *index <= offset)
        .last()
        .unwrap_or(0)
}

fn line_start(text: &str, offset: usize) -> usize {
    text[..offset.min(text.len())]
        .rfind('\n')
        .map_or(0, |index| index + 1)
}

fn line_end(text: &str, offset: usize) -> usize {
    let offset = offset.min(text.len());
    text[offset..]
        .find('\n')
        .map_or(text.len(), |index| offset + index)
}

fn previous_word_boundary(text: &str, offset: usize) -> usize {
    let offset = boundary_at_or_before(text, offset.min(text.len()));
    let mut previous = 0;
    let mut segments = text.split_word_bound_indices().peekable();
    while let Some((start, segment)) = segments.next() {
        if start >= offset {
            break;
        }
        if !segment_is_word(segment) {
            continue;
        }
        let mut end = start + segment.len();
        while let Some((next_start, next_segment)) = segments.peek() {
            if *next_start != end || !segment_is_word(next_segment) {
                break;
            }
            end = *next_start + next_segment.len();
            segments.next();
        }
        if offset <= end {
            return start;
        }
        previous = start;
    }
    previous
}

fn next_word_boundary(text: &str, offset: usize) -> usize {
    let offset = boundary_at_or_before(text, offset.min(text.len()));
    let mut segments = text.split_word_bound_indices().peekable();
    while let Some((start, segment)) = segments.next() {
        if !segment_is_word(segment) {
            continue;
        }
        let mut end = start + segment.len();
        while let Some((next_start, next_segment)) = segments.peek() {
            if *next_start != end || !segment_is_word(next_segment) {
                break;
            }
            end = *next_start + next_segment.len();
            segments.next();
        }
        if end > offset {
            return end;
        }
    }
    text.len()
}

fn segment_is_word(segment: &str) -> bool {
    segment
        .chars()
        .any(|character| character.is_alphanumeric() || character == '_')
}

fn normalize_text(value: &str, multiline: bool) -> Cow<'_, str> {
    if multiline {
        if !value.contains('\r') {
            return Cow::Borrowed(value);
        }
        let mut result = String::with_capacity(value.len());
        let mut characters = value.chars().peekable();
        while let Some(character) = characters.next() {
            if character == '\r' {
                if characters.peek() == Some(&'\n') {
                    characters.next();
                }
                result.push('\n');
            } else {
                result.push(character);
            }
        }
        return Cow::Owned(result);
    }

    if !value.contains('\r') && !value.contains('\n') {
        return Cow::Borrowed(value);
    }
    let mut result = String::with_capacity(value.len());
    let mut previous_was_cr = false;
    for ch in value.chars() {
        match ch {
            '\r' => {
                result.push(' ');
                previous_was_cr = true;
            }
            '\n' if previous_was_cr => {
                previous_was_cr = false;
            }
            '\n' => {
                result.push(' ');
                previous_was_cr = false;
            }
            _ => {
                result.push(ch);
                previous_was_cr = false;
            }
        }
    }
    Cow::Owned(result)
}

/// Remap validated run boundaries when CR/CRLF normalization changes UTF-8 offsets.
///
/// At most two endpoints per bounded run are retained while the source string is scanned once;
/// this avoids an offset table proportional to the complete document size.
fn normalize_highlights(value: &str, highlights: &Arc<[TextHighlight]>) -> Arc<[TextHighlight]> {
    if highlights.is_empty() {
        return highlights.clone();
    }

    let mut endpoints = Vec::with_capacity(highlights.len().saturating_mul(2));
    for highlight in highlights.iter() {
        endpoints.push(highlight.range.start);
        endpoints.push(highlight.range.end);
    }
    endpoints.sort_unstable();
    endpoints.dedup();

    let mut mapped = Vec::with_capacity(endpoints.len());
    let mut endpoint_index = 0;
    let mut source_offset = 0;
    let mut normalized_offset = 0;
    let bytes = value.as_bytes();
    let record = |source_offset: usize,
                  normalized_offset: usize,
                  endpoint_index: &mut usize,
                  mapped: &mut Vec<usize>| {
        while endpoints.get(*endpoint_index).copied() == Some(source_offset) {
            mapped.push(normalized_offset);
            *endpoint_index += 1;
        }
    };
    record(
        source_offset,
        normalized_offset,
        &mut endpoint_index,
        &mut mapped,
    );

    while source_offset < value.len() {
        if bytes[source_offset] == b'\r' {
            source_offset += 1;
            normalized_offset += 1;
            record(
                source_offset,
                normalized_offset,
                &mut endpoint_index,
                &mut mapped,
            );
            if bytes.get(source_offset) == Some(&b'\n') {
                source_offset += 1;
                record(
                    source_offset,
                    normalized_offset,
                    &mut endpoint_index,
                    &mut mapped,
                );
            }
            continue;
        }
        let length = value[source_offset..]
            .chars()
            .next()
            .map_or(1, char::len_utf8);
        source_offset += length;
        normalized_offset += length;
        record(
            source_offset,
            normalized_offset,
            &mut endpoint_index,
            &mut mapped,
        );
    }
    debug_assert_eq!(mapped.len(), endpoints.len());

    let mapped_offset = |offset: usize| {
        endpoints
            .binary_search(&offset)
            .ok()
            .and_then(|index| mapped.get(index).copied())
            .unwrap_or(normalized_offset)
    };
    let mut normalized: Vec<TextHighlight> = Vec::with_capacity(highlights.len());
    for highlight in highlights.iter() {
        let range = mapped_offset(highlight.range.start)..mapped_offset(highlight.range.end);
        if range.is_empty() {
            continue;
        }
        if let Some(previous) = normalized.last_mut()
            && previous.range.end == range.start
            && previous.style == highlight.style
        {
            previous.range.end = range.end;
        } else {
            normalized.push(TextHighlight {
                range,
                style: highlight.style.clone(),
            });
        }
    }
    normalized.into()
}

pub(crate) fn selectable_character_lengths(value: &str) -> Vec<u8> {
    let mut lengths = Vec::with_capacity(value.graphemes(true).count());
    for grapheme in value.graphemes(true) {
        if let Ok(length) = u8::try_from(grapheme.len()) {
            lengths.push(length);
        } else {
            // Pathological combining sequences can exceed AccessKit's per-character u8 length.
            // Splitting those into scalar values preserves the required total UTF-8 byte count.
            lengths.extend(grapheme.chars().map(|ch| ch.len_utf8() as u8));
        }
    }
    lengths
}

pub(crate) fn accessibility_character_index(value: &str, byte_index: usize) -> usize {
    let byte_index = byte_index.min(value.len());
    let mut offset = 0_usize;
    selectable_character_lengths(value)
        .into_iter()
        .take_while(|length| {
            let next = offset + usize::from(*length);
            if next <= byte_index {
                offset = next;
                true
            } else {
                false
            }
        })
        .count()
}

pub(crate) fn accessibility_byte_index(value: &str, character_index: usize) -> usize {
    selectable_character_lengths(value)
        .into_iter()
        .take(character_index)
        .map(usize::from)
        .sum::<usize>()
        .min(value.len())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cursor_movement_uses_grapheme_boundaries() {
        let family = "a👨‍👩‍👧‍👦b";
        let mut input = TextInputState::new(family);
        assert!(input.move_left(false));
        assert_eq!(&family[input.caret()..], "b");
        assert!(input.move_left(false));
        assert_eq!(input.caret(), 1);
        assert!(input.backspace());
        assert_eq!(input.text(), "👨‍👩‍👧‍👦b");
    }

    #[test]
    fn replacement_and_selection_remain_utf8_safe() {
        let mut input = TextInputState::new("café");
        input.move_left(false);
        input.move_left(true);
        assert_eq!(input.selected_text(), Some("f"));
        assert!(input.replace_selection("🙂"));
        assert_eq!(input.text(), "ca🙂é");
    }

    #[test]
    fn preedit_replaces_previous_marked_text() {
        let mut input = TextInputState::new("hello ");
        assert!(input.set_preedit("n", Some((1, 1))));
        assert_eq!(input.text(), "hello n");
        assert_eq!(input.marked(), Some(6..7));
        assert!(input.set_preedit("你", Some((3, 3))));
        assert_eq!(input.text(), "hello 你");
        assert_eq!(input.marked(), Some(6..9));
        assert!(input.set_preedit("", None));
        assert_eq!(input.text(), "hello ");
        assert!(input.replace_selection("你"));
        assert_eq!(input.text(), "hello 你");
    }

    #[test]
    fn controlled_sync_preserves_and_cancel_restores_active_preedit() {
        let mut input = TextInputState::new("hello");
        input.move_left(true);
        assert_eq!(input.selected_text(), Some("o"));
        assert!(input.set_preedit("お", Some((3, 3))));
        assert_eq!(input.text(), "hellお");

        input.sync_external("hello", false, &InputConstraints::default());
        assert_eq!(input.text(), "hellお");

        assert!(input.set_preedit("", None));
        assert_eq!(input.text(), "hello");
        assert_eq!(input.selected_text(), Some("o"));
        assert!(input.replace_selection("お"));
        assert_eq!(input.text(), "hellお");
    }

    #[test]
    fn controlled_style_changes_remap_across_active_preedit() {
        let cool = crate::styled_text("token").with_highlights([(
            0..5,
            crate::HighlightStyle::default().color(crate::Color::rgb8(56, 189, 248)),
        )]);
        let (content, highlights) = cool.into_parts();
        let mut input =
            TextInputState::with_styling(&content, false, InputConstraints::default(), highlights);
        assert!(input.set_selection(2, 2));
        assert!(input.set_preedit("你", Some((3, 3))));

        let warm = crate::styled_text("token").with_highlights([(
            0..5,
            crate::HighlightStyle::default().color(crate::Color::rgb8(244, 114, 182)),
        )]);
        let (_, warm_highlights) = warm.into_parts();
        input.sync_external_styled(
            "token",
            false,
            &InputConstraints::default(),
            &warm_highlights,
        );

        assert_eq!(input.shared_highlights()[0].range(), 0..8);
        assert_eq!(
            input.shared_highlights()[0].style().color,
            Some(crate::Color::rgb8(244, 114, 182))
        );
        assert!(input.set_preedit("", None));
        assert_eq!(input.shared_highlights().as_ref(), warm_highlights.as_ref());
    }

    #[test]
    fn multiline_paste_is_normalized() {
        let mut input = TextInputState::new("");
        input.replace_selection("one\r\ntwo\nthree");
        assert_eq!(input.text(), "one two three");
    }

    #[test]
    fn multiline_editing_preserves_lines_and_normalizes_crlf() {
        let mut input = TextInputState::with_mode("one\r\ntwo\rthree", true);
        assert_eq!(input.text(), "one\ntwo\nthree");
        assert!(input.move_line_start(false));
        assert_eq!(input.caret(), 8);
        assert!(input.move_line_end(false));
        assert_eq!(input.caret(), input.text().len());
        assert!(input.insert_newline());
        assert_eq!(input.text(), "one\ntwo\nthree\n");
    }

    #[test]
    fn newline_normalization_remaps_attributed_byte_ranges_in_one_pass() {
        let styled = crate::styled_text("a\r\nb🙂\rc").with_highlights([
            (0..4, crate::HighlightStyle::default().font_bold()),
            (
                4..9,
                crate::HighlightStyle::default().color(crate::Color::rgb8(96, 165, 250)),
            ),
        ]);
        let (content, highlights) = styled.into_parts();
        let input =
            TextInputState::with_styling(&content, true, InputConstraints::default(), highlights);

        assert_eq!(input.text(), "a\nb🙂\nc");
        assert_eq!(input.shared_highlights()[0].range(), 0..3);
        assert_eq!(input.shared_highlights()[1].range(), 3..8);
    }

    #[test]
    fn line_deletion_joins_adjacent_lines_at_the_boundary() {
        let mut backward = TextInputState::with_mode("one\ntwo\nthree", true);
        assert!(backward.move_line_start(false));
        assert_eq!(&backward.text()[backward.caret()..], "three");
        assert!(backward.delete_to_line_start());
        assert_eq!(backward.text(), "one\ntwothree");

        let mut forward = TextInputState::with_mode("one\ntwo", true);
        assert!(forward.move_home(false));
        assert!(forward.move_line_end(false));
        assert_eq!(forward.caret(), 3);
        assert!(forward.delete_to_line_end());
        assert_eq!(forward.text(), "onetwo");
    }

    #[test]
    fn single_line_inputs_reject_newlines_but_normalize_multiline_replacement() {
        let mut single_line = TextInputState::new("value");
        assert!(!single_line.insert_newline());
        assert_eq!(single_line.text(), "value");

        let mut multiline = TextInputState::with_mode("", true);
        assert!(multiline.replace_selection("one\r\ntwo\rthree"));
        assert_eq!(multiline.text(), "one\ntwo\nthree");
    }

    #[test]
    fn word_navigation_and_deletion_respect_unicode_boundaries() {
        let mut input = TextInputState::with_mode("alpha café_beta 世界", true);
        assert!(input.move_word_left(false));
        assert_eq!(&input.text()[input.caret()..], "世界");
        assert!(input.move_word_left(false));
        assert_eq!(&input.text()[input.caret()..], "café_beta 世界");
        assert!(input.delete_word_forward());
        assert_eq!(input.text(), "alpha  世界");
        assert!(input.delete_word_backward());
        assert_eq!(input.text(), " 世界");
    }

    #[test]
    fn undo_and_redo_restore_text_and_selection() {
        let mut input = TextInputState::new("a");
        assert!(input.replace_selection("b"));
        input.move_to(input.caret(), false);
        assert!(input.replace_selection("c"));
        assert_eq!(input.text(), "abc");

        assert!(input.undo());
        assert_eq!(input.text(), "ab");
        assert_eq!(input.selection(), 2..2);
        assert!(input.undo());
        assert_eq!(input.text(), "a");
        assert!(input.redo());
        assert_eq!(input.text(), "ab");

        assert!(input.replace_selection("!"));
        assert_eq!(input.text(), "ab!");
        assert!(!input.can_redo());
    }

    #[test]
    fn adjacent_native_typing_and_deletion_coalesce_without_a_timer() {
        let mut input = TextInputState::new("replace me");
        assert!(input.select_all());
        for character in "GPU hot".chars() {
            assert!(input.replace_selection(&character.to_string()));
        }
        assert_eq!(input.text(), "GPU hot");
        assert_eq!(input.undo.len(), 1);
        assert!(input.undo());
        assert_eq!(input.text(), "replace me");
        assert!(input.redo());
        assert_eq!(input.text(), "GPU hot");

        let undo_before_backspace = input.undo.len();
        for _ in 0..3 {
            assert!(input.backspace());
        }
        assert_eq!(input.text(), "GPU ");
        assert_eq!(input.undo.len(), undo_before_backspace + 1);
        assert!(input.undo());
        assert_eq!(input.text(), "GPU hot");
    }

    #[test]
    fn typing_coalescing_expires_without_scheduling_work() {
        let mut input = TextInputState::new("");
        assert!(input.replace_selection("a"));
        input.edit_group.as_mut().unwrap().last_edit_at = Instant::now()
            .checked_sub(EDIT_COALESCE_WINDOW + Duration::from_millis(1))
            .unwrap();
        assert!(input.replace_selection("b"));

        assert_eq!(input.undo.len(), 2);
        assert!(input.undo());
        assert_eq!(input.text(), "a");
    }

    #[test]
    fn attributed_edits_shift_split_merge_and_restore_bounded_runs() {
        let styled = crate::styled_text("abcdef").with_highlights([(
            1..5,
            crate::HighlightStyle::default()
                .font_bold()
                .color(crate::Color::rgb8(94, 234, 212)),
        )]);
        let (content, highlights) = styled.into_parts();
        let mut input =
            TextInputState::with_styling(&content, false, InputConstraints::default(), highlights);

        assert!(input.set_selection(3, 3));
        assert!(input.replace_selection("X"));
        assert_eq!(input.text(), "abcXdef");
        assert_eq!(input.shared_highlights()[0].range(), 1..6);

        assert!(input.set_selection(2, 5));
        assert!(input.replace_selection("🙂"));
        assert_eq!(input.text(), "ab🙂ef");
        assert_eq!(input.shared_highlights()[0].range(), 1..7);
        assert!(input.undo());
        assert_eq!(input.text(), "abcXdef");
        assert_eq!(input.shared_highlights()[0].range(), 1..6);
    }

    #[test]
    fn attributed_insertions_at_run_boundaries_do_not_bleed_styles() {
        let styled = crate::styled_text("abcd")
            .with_highlights([(1..3, crate::HighlightStyle::default().underline())]);
        let (content, highlights) = styled.into_parts();
        let mut input =
            TextInputState::with_styling(&content, false, InputConstraints::default(), highlights);

        assert!(input.set_selection(1, 1));
        assert!(input.replace_selection("X"));
        assert_eq!(input.shared_highlights()[0].range(), 2..4);
        assert!(input.set_selection(4, 4));
        assert!(input.replace_selection("Y"));
        assert_eq!(input.shared_highlights()[0].range(), 2..4);
    }

    #[test]
    fn cancelling_preedit_restores_attributed_content_exactly() {
        let styled = crate::styled_text("token")
            .with_highlights([(0..5, crate::HighlightStyle::default().font_semibold())]);
        let (content, highlights) = styled.into_parts();
        let original = highlights.clone();
        let mut input =
            TextInputState::with_styling(&content, false, InputConstraints::default(), highlights);

        assert!(input.set_selection(2, 2));
        assert!(input.set_preedit("你", Some((3, 3))));
        assert_eq!(input.shared_highlights()[0].range(), 0..8);
        assert!(input.set_preedit("", None));
        assert_eq!(input.text(), "token");
        assert_eq!(input.shared_highlights().as_ref(), original.as_ref());
    }

    #[test]
    fn composition_commit_creates_one_undo_step() {
        let mut input = TextInputState::new("hello ");
        assert!(input.set_preedit("n", Some((1, 1))));
        assert!(input.set_preedit("ni", Some((2, 2))));
        assert!(input.replace_selection("你"));
        assert_eq!(input.text(), "hello 你");
        assert_eq!(input.undo.len(), 1);
        assert!(input.undo());
        assert_eq!(input.text(), "hello ");
    }

    #[test]
    fn max_length_truncates_at_unicode_grapheme_boundaries() {
        let constraints = InputConstraints {
            max_length: Some(3),
            filter: None,
        };
        let mut input = TextInputState::with_constraints("a", false, constraints);

        assert!(input.replace_selection("👨‍👩‍👧‍👦bc"));
        assert_eq!(input.text(), "a👨‍👩‍👧‍👦b");
        assert_eq!(input.selection(), input.text().len()..input.text().len());
        assert!(input.undo());
        assert_eq!(input.text(), "a");

        assert!(input.set_value("é🙂xy"));
        assert_eq!(input.text(), "é🙂x");
    }

    #[test]
    fn rejected_edits_do_not_mutate_text_selection_or_history() {
        let constraints = InputConstraints {
            max_length: None,
            filter: Some(Arc::new(|value| {
                value.chars().all(|character| character.is_ascii_digit())
            })),
        };
        let mut input = TextInputState::with_constraints("12", false, constraints);
        let selection = input.selection();

        assert!(!input.replace_selection("a"));
        assert_eq!(input.text(), "12");
        assert_eq!(input.selection(), selection);
        assert!(!input.can_undo());
    }

    #[test]
    fn rejected_ime_commit_restores_the_preedit_backup_without_history() {
        let constraints = InputConstraints {
            max_length: None,
            filter: Some(Arc::new(|value| value.is_ascii())),
        };
        let mut input = TextInputState::with_constraints("hello", false, constraints);

        assert!(input.set_preedit("你", Some((3, 3))));
        assert_eq!(input.text(), "hello你");
        assert_eq!(input.committed_shared_text().as_ref(), "hello");
        assert!(input.replace_selection("你"));
        assert_eq!(input.text(), "hello");
        assert_eq!(input.selection(), 5..5);
        assert!(!input.can_undo());
    }

    #[test]
    fn changed_constraints_can_disable_an_incompatible_undo_without_popping_it() {
        let mut input = TextInputState::new("ab");
        assert!(input.replace_selection("c"));
        assert!(input.can_undo());
        input.sync_external(
            "abc",
            false,
            &InputConstraints {
                max_length: Some(1),
                filter: None,
            },
        );

        assert!(!input.can_undo());
        assert!(!input.undo());
        assert_eq!(input.text(), "abc");

        input.sync_external("abc", false, &InputConstraints::default());
        assert!(input.can_undo());
        assert!(input.undo());
        assert_eq!(input.text(), "ab");
    }

    #[test]
    fn edit_history_is_bounded_by_count_and_bytes() {
        let mut input = TextInputState::new("");
        for _ in 0..(MAX_HISTORY_ENTRIES + 50) {
            assert!(input.replace_selection("x"));
            input.move_to(input.caret(), false);
        }
        assert!(input.undo.len() <= MAX_HISTORY_ENTRIES);
        assert!(input.undo_bytes <= MAX_HISTORY_BYTES_PER_STACK);

        input.set_value(&"x".repeat(MAX_HISTORY_BYTES_PER_STACK + 1));
        assert!(input.replace_selection("y"));
        assert!(!input.can_undo());
        assert_eq!(input.undo_bytes, 0);
    }

    #[test]
    fn attributed_history_counts_run_and_named_family_storage() {
        let content: Arc<str> = Arc::from("x".repeat(1_200));
        let highlights = (0..600).map(|index| {
            (
                index * 2..index * 2 + 1,
                crate::HighlightStyle::default().font_family(FontFamily::Named(Arc::from(
                    format!("family-{index}-{}", "x".repeat(900)),
                ))),
            )
        });
        let styled = crate::styled_text(content).with_highlights(highlights);
        let (content, highlights) = styled.into_parts();
        let mut input =
            TextInputState::with_styling(&content, false, InputConstraints::default(), highlights);

        assert!(input.replace_selection("!"));
        assert_eq!(input.undo_bytes, 0);
        assert!(!input.can_undo());
    }

    #[test]
    fn accessibility_offsets_use_editor_graphemes() {
        let input = TextInputState::new("a👨‍👩‍👧‍👦é");
        assert_eq!(input.accessibility_character_lengths(), vec![1, 25, 2]);
        assert_eq!(input.accessibility_character_index(1), 1);
        assert_eq!(input.accessibility_character_index(26), 2);
        assert_eq!(input.accessibility_byte_index(2), 26);
        assert_eq!(input.accessibility_byte_index(99), input.text().len());
    }
}
