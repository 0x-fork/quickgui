//! Browser-class text services: spell checking, autocorrect, substitutions, dictionary lookup,
//! find and replace, and an application-wide undo manager.
//!
//! Run with `cargo run --release --example text_services`. The example installs its own tiny
//! [`SpellCheckProvider`] so the demo is identical on every target; a real application either
//! keeps the macOS `NSSpellChecker` default or installs its own dictionary.

use std::{cell::RefCell, ops::Range, sync::Arc};

use quickgui::{
    Application, Color, Element, FindBar, FindOptions, FindState, IgnoreWord, KeyBinding,
    LearnWord, LookUpSelection, Misspelling, Point, PopoverMenuItem, Redo, ReplaceWord,
    SpellCheckProvider, SpellDocumentTag, SpellingMenuLabels, TextCheckingPolicy, Undo,
    UndoManager, UndoableChange, View, ViewContext, WindowOptions, button, div,
    find_bar_key_bindings, set_default_text_checking, set_spell_check_provider,
    show_definition_for, spell_check_provider, spelling_menu_items, text, text_area, text_input,
    undo_key_bindings, word_range_at,
};

/// A deliberately tiny dictionary so the example behaves the same on every platform.
#[derive(Default)]
struct DemoDictionary {
    learned: RefCell<Vec<String>>,
}

const KNOWN_WORDS: &[&str] = &[
    "a", "and", "damage", "driven", "gpu", "is", "quickgui", "renderer", "sleeps", "the", "this",
    "window",
];

const CORRECTIONS: &[(&str, &str)] = &[("teh", "the"), ("recieve", "receive")];

impl DemoDictionary {
    fn is_known(&self, word: &str) -> bool {
        let lowered = word.to_lowercase();
        KNOWN_WORDS.contains(&lowered.as_str())
            || self
                .learned
                .borrow()
                .iter()
                .any(|entry| entry.eq_ignore_ascii_case(word))
    }
}

impl SpellCheckProvider for DemoDictionary {
    fn check(
        &self,
        text: &str,
        range: Range<usize>,
        policy: TextCheckingPolicy,
    ) -> Vec<Misspelling> {
        if !policy.spellcheck {
            return Vec::new();
        }
        let Some(slice) = text.get(range.clone()) else {
            return Vec::new();
        };
        let mut flagged = Vec::new();
        let mut word_start: Option<usize> = None;
        let push = |start: usize, end: usize, flagged: &mut Vec<Misspelling>| {
            if !self.is_known(&slice[start..end]) {
                flagged.push(Misspelling::spelling(
                    range.start + start..range.start + end,
                ));
            }
        };
        for (index, character) in slice.char_indices() {
            if character.is_alphanumeric() {
                word_start.get_or_insert(index);
            } else if let Some(start) = word_start.take() {
                push(start, index, &mut flagged);
            }
        }
        if let Some(start) = word_start {
            push(start, slice.len(), &mut flagged);
        }
        flagged
    }

    fn guesses(&self, word: &str) -> Vec<String> {
        let lowered = word.to_lowercase();
        KNOWN_WORDS
            .iter()
            .filter(|known| known.starts_with(lowered.chars().next().unwrap_or('_')))
            .map(|known| (*known).to_owned())
            .collect()
    }

    fn correction(&self, text: &str, range: Range<usize>) -> Option<String> {
        let word = text.get(range)?;
        CORRECTIONS
            .iter()
            .find(|(from, _)| from.eq_ignore_ascii_case(word))
            .map(|(_, to)| (*to).to_owned())
    }

    fn learn(&self, word: &str) {
        self.learned.borrow_mut().push(word.to_owned());
    }

    fn ignore(&self, word: &str, _tag: SpellDocumentTag) {
        self.learned.borrow_mut().push(word.to_owned());
    }
}

fn main() -> Result<(), quickgui::AppError> {
    set_spell_check_provider(DemoDictionary::default());
    set_default_text_checking(TextCheckingPolicy {
        smart_quotes: true,
        smart_dashes: true,
        ..TextCheckingPolicy::NONE
    });

    let mut bindings: Vec<KeyBinding> = Vec::new();
    bindings.extend(find_bar_key_bindings());
    bindings.extend(undo_key_bindings());
    bindings.push(KeyBinding::new("ctrl-cmd-d", LookUpSelection, None));

    Application::new().bind_keys(bindings).run(|cx| {
        cx.open_window(
            WindowOptions::new("QuickGUI — Text services").size(860.0, 620.0),
            TextServicesDemo::new(),
        );
    })
}

struct TextServicesDemo {
    notes: Arc<str>,
    find: FindState,
    find_bar_open: bool,
    undo: UndoManager<TextServicesDemo>,
    status: Arc<str>,
}

impl TextServicesDemo {
    fn new() -> Self {
        Self {
            notes: Arc::from(
                "Teh renderer is damage driven and this window sleeps.\nType teh then a space to see autocorrect.",
            ),
            find: FindState::new(),
            find_bar_open: true,
            undo: UndoManager::new(),
            status: Arc::from(
                "Edit the notes; misspellings underline 300 ms after you stop typing.",
            ),
        }
    }

    /// Replace the whole controlled value and register it as one application undo entry.
    fn set_notes(&mut self, value: impl Into<Arc<str>>, name: &str) {
        let value = value.into();
        let before = self.notes.clone();
        let after = value.clone();
        self.notes = value;
        self.find.refresh(&self.notes);
        self.undo
            .register(UndoableChange::new(name, before, after).into_entry(
                |this: &mut TextServicesDemo, value: &Arc<str>| {
                    this.notes = value.clone();
                    this.find.refresh(&this.notes);
                },
            ));
    }

    /// The standard right-click spelling entries for the word under `offset`.
    fn spelling_menu(&self, offset: usize) -> Vec<PopoverMenuItem> {
        let Some(range) = word_range_at(&self.notes, offset) else {
            return Vec::new();
        };
        let word = &self.notes[range.clone()];
        let guesses: Vec<Arc<str>> = spell_check_provider()
            .guesses(word)
            .into_iter()
            .map(Arc::from)
            .collect();
        spelling_menu_items(range, word, &guesses, SpellingMenuLabels::default())
    }
}

impl View for TextServicesDemo {
    fn render(&mut self, cx: &mut ViewContext<'_, Self>) -> impl quickgui::IntoElement {
        let edit_notes = cx.input_listener("notes", |this, value, cx| {
            this.notes = Arc::from(value);
            this.find.refresh(&this.notes);
            cx.invalidate();
        });
        let edit_query = cx.input_listener("find-query", |this, value, cx| {
            let notes = this.notes.clone();
            this.find.set_query(value, &notes);
            cx.invalidate();
        });
        let edit_replacement = cx.input_listener("find-replace", |this, value, cx| {
            this.find.set_replacement(value);
            cx.invalidate();
        });
        let next = cx.listener("find-next", |this, cx| {
            let notes = this.notes.clone();
            this.find.find_next(&notes);
            cx.invalidate();
        });
        let previous = cx.listener("find-previous", |this, cx| {
            let notes = this.notes.clone();
            this.find.find_previous(&notes);
            cx.invalidate();
        });
        let replace = cx.listener("find-replace-current", |this, cx| {
            let notes = this.notes.clone();
            if let Some((replaced, _)) = this.find.replace_current(&notes) {
                this.set_notes(replaced, "Replace");
            }
            cx.invalidate();
        });
        let replace_all = cx.listener("find-replace-all", |this, cx| {
            let notes = this.notes.clone();
            if let Some(replaced) = this.find.replace_all(&notes) {
                this.set_notes(replaced, "Replace All");
            }
            cx.invalidate();
        });
        let close = cx.listener("find-close", |this, cx| {
            this.find_bar_open = false;
            cx.invalidate();
        });
        let toggle_whole_word = cx.listener("find-whole-word", |this, cx| {
            let notes = this.notes.clone();
            let options = FindOptions {
                whole_word: !this.find.options().whole_word,
                ..this.find.options()
            };
            this.find.set_options(options, &notes);
            cx.invalidate();
        });

        let replace_word = cx.action_listener("notes", |this, action: &ReplaceWord, cx| {
            let mut updated = this.notes.to_string();
            if action.range.end <= updated.len() {
                updated.replace_range(action.range.clone(), &action.replacement);
                this.set_notes(updated, "Correct Spelling");
            }
            cx.invalidate();
        });
        let learn_word = cx.action_listener("notes", |this, action: &LearnWord, cx| {
            spell_check_provider().learn(&action.word);
            this.status = Arc::from(format!("Learned \"{}\".", action.word));
            cx.invalidate();
        });
        let ignore_word = cx.action_listener("notes", |this, action: &IgnoreWord, cx| {
            spell_check_provider().ignore(&action.word, SpellDocumentTag::NONE);
            this.status = Arc::from(format!("Ignoring \"{}\".", action.word));
            cx.invalidate();
        });
        let look_up = cx.action_listener("notes", |this, _: &LookUpSelection, cx| {
            this.status = match show_definition_for("renderer", Point::new(24.0, 220.0)) {
                Ok(()) => Arc::from("Showed the macOS definition popover."),
                Err(error) => Arc::from(format!("Dictionary lookup: {error}")),
            };
            cx.invalidate();
        });
        let undo = cx.action_listener("notes", |this, _: &Undo, cx| {
            let mut manager = std::mem::take(&mut this.undo);
            manager.undo(this);
            this.undo = manager;
            cx.invalidate();
        });
        let redo = cx.action_listener("notes", |this, _: &Redo, cx| {
            let mut manager = std::mem::take(&mut this.undo);
            manager.redo(this);
            this.undo = manager;
            cx.invalidate();
        });

        let bar = FindBar::new("editor-find").state(&self.find);
        let undo_title = self
            .undo
            .undo_action_name()
            .map_or_else(|| "Undo".to_owned(), |name| format!("Undo {name}"));

        div()
            .size_full()
            .flex_col()
            .gap_3()
            .p_4()
            .bg(Color::rgb8(18, 19, 24))
            .text_color(Color::rgb8(226, 232, 240))
            .on_action(replace_word)
            .on_action(learn_word)
            .on_action(ignore_word)
            .on_action(look_up)
            .on_action(undo)
            .on_action(redo)
            .child(text("Text services").text_xl())
            .child(text(self.status.clone()))
            .child(text(undo_title))
            .child(
                text_area(self.notes.clone())
                    .id("notes")
                    .on_input(edit_notes)
                    .spellcheck(true)
                    .autocorrect(true)
                    .smart_quotes(true)
                    .smart_dashes(true)
                    .text_replacement(true)
                    .lookup_on_force_click(true)
                    .accessibility_label("Notes")
                    .w_full()
                    .h(220.0),
            )
            .when(self.find_bar_open, |root| {
                root.child(
                    bar.root()
                        .flex_row()
                        .items_center()
                        .gap_2()
                        .w_full()
                        .child(
                            bar.query_input_with(
                                text_input(self.find.query().clone())
                                    .on_input(edit_query)
                                    .w(200.0),
                            ),
                        )
                        .child(
                            bar.replace_input_with(
                                text_input(self.find.replacement().clone())
                                    .on_input(edit_replacement)
                                    .w(200.0),
                            ),
                        )
                        .child(bar.count_with(text(bar.count_text().clone())))
                        .child(bar.previous_with(control("Previous").on_click(previous)))
                        .child(bar.next_with(control("Next").on_click(next)))
                        .child(bar.replace_with(control("Replace").on_click(replace)))
                        .child(bar.replace_all_with(control("All").on_click(replace_all)))
                        .child(control("Whole word").on_click(toggle_whole_word))
                        .child(bar.close_with(control("Close").on_click(close))),
                )
            })
            .child(text(format!(
                "{} spelling entries would appear at offset 0.",
                self.spelling_menu(0).len()
            )))
    }
}

fn control(label: &'static str) -> Element {
    button()
        .h(32.0)
        .px_3()
        .flex_row()
        .items_center()
        .justify_center()
        .rounded_md()
        .border(1.0, Color::rgb8(75, 80, 92))
        .child(text(label))
}
