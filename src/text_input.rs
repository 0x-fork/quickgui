use std::{collections::VecDeque, ops::Range, sync::Arc};

use unicode_segmentation::UnicodeSegmentation;

#[derive(Clone, Debug)]
pub(crate) struct TextInputState {
    text: Arc<str>,
    anchor: usize,
    caret: usize,
    marked: Option<Range<usize>>,
    composition_backup: Option<CompositionBackup>,
    scroll_x: f32,
    undo: VecDeque<EditSnapshot>,
    redo: VecDeque<EditSnapshot>,
    undo_bytes: usize,
    redo_bytes: usize,
}

#[derive(Clone, Debug)]
struct CompositionBackup {
    text: Arc<str>,
    anchor: usize,
    caret: usize,
}

#[derive(Clone, Debug)]
struct EditSnapshot {
    text: Arc<str>,
    anchor: usize,
    caret: usize,
}

const MAX_HISTORY_ENTRIES: usize = 100;
const MAX_HISTORY_BYTES_PER_STACK: usize = 512 * 1024;

impl TextInputState {
    pub fn new(value: &str) -> Self {
        Self {
            text: Arc::from(value),
            anchor: value.len(),
            caret: value.len(),
            marked: None,
            composition_backup: None,
            scroll_x: 0.0,
            undo: VecDeque::new(),
            redo: VecDeque::new(),
            undo_bytes: 0,
            redo_bytes: 0,
        }
    }

    pub fn text(&self) -> &str {
        &self.text
    }

    pub fn shared_text(&self) -> Arc<str> {
        self.text.clone()
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

    pub fn scroll_x(&self) -> f32 {
        self.scroll_x
    }

    pub fn set_scroll_x(&mut self, value: f32) {
        self.scroll_x = value.max(0.0);
    }

    pub fn sync_external(&mut self, value: &str) {
        if self.text.as_ref() == value {
            return;
        }
        if self
            .composition_backup
            .as_ref()
            .is_some_and(|backup| backup.text.as_ref() == value)
        {
            return;
        }
        self.text = Arc::from(value);
        self.anchor = value.len();
        self.caret = value.len();
        self.marked = None;
        self.composition_backup = None;
        self.scroll_x = 0.0;
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

    pub fn move_home(&mut self, extend: bool) -> bool {
        self.move_to(0, extend)
    }

    pub fn move_end(&mut self, extend: bool) -> bool {
        self.move_to(self.text.len(), extend)
    }

    pub fn move_to(&mut self, index: usize, extend: bool) -> bool {
        let index = boundary_at_or_before(&self.text, index.min(self.text.len()));
        let changed = self.caret != index || (!extend && self.anchor != index);
        self.caret = index;
        if !extend {
            self.anchor = index;
        }
        self.marked = None;
        self.composition_backup = None;
        changed
    }

    pub fn select_all(&mut self) -> bool {
        let changed = self.anchor != 0 || self.caret != self.text.len();
        self.anchor = 0;
        self.caret = self.text.len();
        self.marked = None;
        self.composition_backup = None;
        changed
    }

    pub fn selected_text(&self) -> Option<&str> {
        let selection = self.selection();
        (!selection.is_empty()).then(|| &self.text[selection])
    }

    pub fn set_selection(&mut self, anchor: usize, caret: usize) -> bool {
        let anchor = boundary_at_or_before(&self.text, anchor.min(self.text.len()));
        let caret = boundary_at_or_before(&self.text, caret.min(self.text.len()));
        let changed = self.anchor != anchor || self.caret != caret || self.marked.is_some();
        self.anchor = anchor;
        self.caret = caret;
        self.marked = None;
        self.composition_backup = None;
        changed
    }

    pub fn set_value(&mut self, value: &str) -> bool {
        let value = single_line(value);
        let changed = self.text.as_ref() != value
            || self.anchor != value.len()
            || self.caret != value.len()
            || self.marked.is_some();
        if changed {
            self.record_edit();
        }
        self.text = Arc::from(value);
        self.anchor = self.text.len();
        self.caret = self.text.len();
        self.marked = None;
        self.composition_backup = None;
        changed
    }

    pub fn accessibility_character_lengths(&self) -> Vec<u8> {
        selectable_character_lengths(&self.text)
    }

    pub fn accessibility_character_index(&self, byte_index: usize) -> usize {
        let byte_index = byte_index.min(self.text.len());
        let mut offset = 0_usize;
        self.accessibility_character_lengths()
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

    pub fn accessibility_byte_index(&self, character_index: usize) -> usize {
        self.accessibility_character_lengths()
            .into_iter()
            .take(character_index)
            .map(usize::from)
            .sum::<usize>()
            .min(self.text.len())
    }

    pub fn backspace(&mut self) -> bool {
        let selection = self.selection();
        if selection.is_empty() {
            let start = previous_boundary(&self.text, self.caret);
            if start == self.caret {
                return false;
            }
            self.replace_range(start..self.caret, "", true);
        } else {
            self.replace_range(selection, "", true);
        }
        true
    }

    pub fn delete(&mut self) -> bool {
        let selection = self.selection();
        if selection.is_empty() {
            let end = next_boundary(&self.text, self.caret);
            if end == self.caret {
                return false;
            }
            self.replace_range(self.caret..end, "", true);
        } else {
            self.replace_range(selection, "", true);
        }
        true
    }

    pub fn replace_selection(&mut self, value: &str) -> bool {
        let value = single_line(value);
        if value.is_empty() && self.selection().is_empty() {
            return false;
        }
        self.record_edit();
        let range = self.marked.take().unwrap_or_else(|| self.selection());
        self.replace_range(range, &value, false);
        true
    }

    pub fn set_preedit(&mut self, value: &str, cursor: Option<(usize, usize)>) -> bool {
        let value = single_line(value);
        let previous_text = self.text.clone();
        let previous_anchor = self.anchor;
        let previous_caret = self.caret;
        let previous_marked = self.marked.clone();
        let previous_backup = self.composition_backup.clone();

        if value.is_empty() {
            if let Some(backup) = self.composition_backup.take() {
                self.text = backup.text;
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
                anchor: self.anchor,
                caret: self.caret,
            });
        let range = self.marked.take().unwrap_or_else(|| self.selection());
        let start = range.start;
        self.replace_range(range, &value, false);
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
        !self.undo.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.redo.is_empty()
    }

    pub fn undo(&mut self) -> bool {
        let Some(snapshot) = self.undo.pop_back() else {
            return false;
        };
        self.undo_bytes = self.undo_bytes.saturating_sub(snapshot.text.len());
        let current = self.snapshot();
        push_bounded_history(&mut self.redo, &mut self.redo_bytes, current);
        self.restore(snapshot);
        true
    }

    pub fn redo(&mut self) -> bool {
        let Some(snapshot) = self.redo.pop_back() else {
            return false;
        };
        self.redo_bytes = self.redo_bytes.saturating_sub(snapshot.text.len());
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
                anchor: backup.anchor,
                caret: backup.caret,
            })
            .unwrap_or_else(|| EditSnapshot {
                text: self.text.clone(),
                anchor: self.anchor,
                caret: self.caret,
            })
    }

    fn record_edit(&mut self) {
        let snapshot = self.snapshot();
        push_bounded_history(&mut self.undo, &mut self.undo_bytes, snapshot);
        self.redo.clear();
        self.redo_bytes = 0;
    }

    fn clear_history(&mut self) {
        self.undo.clear();
        self.redo.clear();
        self.undo_bytes = 0;
        self.redo_bytes = 0;
    }

    fn restore(&mut self, snapshot: EditSnapshot) {
        self.text = snapshot.text;
        self.anchor = snapshot.anchor;
        self.caret = snapshot.caret;
        self.marked = None;
        self.composition_backup = None;
    }

    fn replace_range(&mut self, range: Range<usize>, value: &str, record: bool) {
        if record {
            self.record_edit();
        }
        let mut text = self.text.to_string();
        text.replace_range(range.clone(), value);
        self.text = Arc::from(text);
        let end = range.start + value.len();
        self.anchor = end;
        self.caret = end;
        self.marked = None;
        self.composition_backup = None;
    }
}

fn push_bounded_history(
    history: &mut VecDeque<EditSnapshot>,
    retained_bytes: &mut usize,
    snapshot: EditSnapshot,
) {
    let bytes = snapshot.text.len();
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
        *retained_bytes = retained_bytes.saturating_sub(evicted.text.len());
    }
    *retained_bytes += bytes;
    history.push_back(snapshot);
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

fn boundary_at_or_before(text: &str, offset: usize) -> usize {
    if offset >= text.len() {
        return text.len();
    }
    text.grapheme_indices(true)
        .map(|(index, _)| index)
        .take_while(|index| *index <= offset)
        .last()
        .unwrap_or(0)
}

fn single_line(value: &str) -> String {
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
    result
}

fn selectable_character_lengths(value: &str) -> Vec<u8> {
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

        input.sync_external("hello");
        assert_eq!(input.text(), "hellお");

        assert!(input.set_preedit("", None));
        assert_eq!(input.text(), "hello");
        assert_eq!(input.selected_text(), Some("o"));
        assert!(input.replace_selection("お"));
        assert_eq!(input.text(), "hellお");
    }

    #[test]
    fn multiline_paste_is_normalized() {
        let mut input = TextInputState::new("");
        input.replace_selection("one\r\ntwo\nthree");
        assert_eq!(input.text(), "one two three");
    }

    #[test]
    fn undo_and_redo_restore_text_and_selection() {
        let mut input = TextInputState::new("a");
        assert!(input.replace_selection("b"));
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
    fn edit_history_is_bounded_by_count_and_bytes() {
        let mut input = TextInputState::new("");
        for _ in 0..(MAX_HISTORY_ENTRIES + 50) {
            assert!(input.replace_selection("x"));
        }
        assert!(input.undo.len() <= MAX_HISTORY_ENTRIES);
        assert!(input.undo_bytes <= MAX_HISTORY_BYTES_PER_STACK);

        input.set_value(&"x".repeat(MAX_HISTORY_BYTES_PER_STACK + 1));
        assert!(input.replace_selection("y"));
        assert!(!input.can_undo());
        assert_eq!(input.undo_bytes, 0);
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
