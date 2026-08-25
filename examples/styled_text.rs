use std::{ops::Range, sync::Arc};

use quickgui::{
    App, Color, HighlightStyle, KeyBinding, Menu, MenuItem, OsAction, StyledText, TitleBarStyle,
    View, ViewContext, button, div, styled_text, styled_text_area, text, text_area,
};

quickgui::actions!(edit, [Cut, Copy, Paste, SelectAll]);

fn edit_menu() -> Menu {
    Menu::new("Edit").items([
        MenuItem::os_action("Cut", Cut, OsAction::Cut),
        MenuItem::os_action("Copy", Copy, OsAction::Copy),
        MenuItem::os_action("Paste", Paste, OsAction::Paste),
        MenuItem::os_action("Select All", SelectAll, OsAction::SelectAll),
    ])
}

const SAMPLE: &str = r#"fn render_status(cache_hits: usize) -> &'static str {
    let state = "GPU cached";
    // Fallback scripts stay shaped in one buffer: 東京, مرحبا, and 🙂.
    deprecated_api();
    state
}"#;

fn main() -> Result<(), quickgui::AppError> {
    App::new(StyledTextDemo {
        warm: false,
        editable: Arc::from(SAMPLE),
        pasted: Arc::from(""),
    })
    .title("QuickGUI — Styled text")
    .size(820.0, 620.0)
    .title_bar_style(TitleBarStyle::HiddenInset)
    .traffic_light_position(16.0, 13.0)
    .bind_keys([
        KeyBinding::new("platform-x", Cut, None),
        KeyBinding::new("platform-c", Copy, None),
        KeyBinding::new("platform-v", Paste, None),
        KeyBinding::new("platform-a", SelectAll, None),
    ])
    .menu(edit_menu())
    .run()
}

struct StyledTextDemo {
    warm: bool,
    editable: Arc<str>,
    pasted: Arc<str>,
}

impl StyledTextDemo {
    fn code(&self) -> StyledText {
        self.highlighted_code(Arc::from(SAMPLE))
    }

    fn editable_code(&self) -> StyledText {
        self.highlighted_code(self.editable.clone())
    }

    fn highlighted_code(&self, content: Arc<str>) -> StyledText {
        let accent = if self.warm {
            Color::rgba8(190, 24, 93, 72)
        } else {
            Color::rgba8(14, 116, 144, 72)
        };
        let mut highlights = Vec::new();
        for keyword in ["fn", "let", "usize", "str"] {
            push_all(
                &content,
                keyword,
                HighlightStyle::default()
                    .color(Color::rgb8(196, 181, 253))
                    .font_semibold(),
                &mut highlights,
            );
        }
        push_one_if_present(
            &content,
            "\"GPU cached\"",
            HighlightStyle::default()
                .color(Color::rgb8(103, 232, 249))
                .background(accent),
            &mut highlights,
        );
        push_one_if_present(
            &content,
            "// Fallback scripts stay shaped in one buffer: 東京, مرحبا, and 🙂.",
            HighlightStyle::default()
                .color(Color::rgb8(148, 163, 184))
                .italic(),
            &mut highlights,
        );
        push_one_if_present(
            &content,
            "deprecated_api",
            HighlightStyle::default()
                .color(Color::rgb8(248, 113, 113))
                .strikethrough(),
            &mut highlights,
        );
        styled_text(content).with_highlights(non_overlapping(highlights))
    }

    fn prose(&self) -> StyledText {
        let content: Arc<str> = Arc::from(
            "QuickGUI shapes this entire paragraph once. Byte-range runs can change foreground, weight, family, italic style, background, underline, or strikethrough while wrapping and bidirectional text continue to share one retained Cosmic Text layout.",
        );
        let mut highlights = Vec::new();
        push_one_if_present(
            &content,
            "Byte-range runs",
            HighlightStyle::default()
                .font_bold()
                .color(Color::rgb8(94, 234, 212)),
            &mut highlights,
        );
        push_one_if_present(
            &content,
            "underline",
            HighlightStyle::default()
                .color(Color::rgb8(96, 165, 250))
                .double_underline(),
            &mut highlights,
        );
        push_one_if_present(
            &content,
            "one retained Cosmic Text layout",
            HighlightStyle::default()
                .font_semibold()
                .underline_color(Color::rgb8(45, 212, 191)),
            &mut highlights,
        );
        highlights.sort_by_key(|(range, _)| range.start);
        styled_text(content).with_highlights(highlights)
    }
}

impl View for StyledTextDemo {
    fn render(&mut self, cx: &mut ViewContext<'_, Self>) -> impl quickgui::IntoElement {
        let toggle = cx.listener("toggle-accent", |this, cx| {
            this.warm = !this.warm;
            cx.invalidate();
        });
        let edit_paste = cx.input_listener("paste-target", |this, value, cx| {
            this.pasted = Arc::from(value);
            cx.invalidate();
        });
        let edit_code = cx.input_listener("styled-editor", |this, value, cx| {
            this.editable = Arc::from(value);
            cx.invalidate();
        });
        let metrics = cx.metrics();

        div()
            .size_full()
            .flex_col()
            .bg(Color::rgb8(15, 17, 21))
            .text_color(Color::rgb8(226, 232, 240))
            .child(
                div()
                    .h(52.0)
                    .flex_none()
                    .flex_row()
                    .items_center()
                    .justify_center()
                    .app_region_drag()
                    .child(text("Retained styled text").font_semibold()),
            )
            .child(
                div()
                    .flex_1()
                    .min_h(0.0)
                    .overflow_y_scroll()
                    .p_6()
                    .gap_5()
                    .flex_col()
                    .child(
                        text("One layout, static and editable visual runs")
                            .text_2xl()
                            .font_bold(),
                    )
                    .child(
                        div()
                            .w_full()
                            .max_w(700.0)
                            .text_lg()
                            .line_height(28.0)
                            .child(self.prose()),
                    )
                    .child(
                        div()
                            .w_full()
                            .max_w(700.0)
                            .p_4()
                            .rounded_xl()
                            .border(1.0, Color::rgb8(51, 65, 85))
                            .bg(Color::rgb8(22, 27, 34))
                            .font_family(quickgui::FontFamily::Monospace)
                            .text_sm()
                            .line_height(22.0)
                            .child(self.code()),
                    )
                    .child(
                        button()
                            .app_region_no_drag()
                            .on_click(toggle)
                            .px_4()
                            .py_2()
                            .rounded_lg()
                            .bg(Color::rgb8(30, 64, 175))
                            .hover(|style| style.bg(Color::rgb8(37, 99, 235)))
                            .active(|style| style.bg(Color::rgb8(29, 78, 216)))
                            .child("Toggle paint-only background color"),
                    )
                    .child(
                        text("Controlled attributed editor")
                            .text_lg()
                            .font_semibold(),
                    )
                    .child(
                        text("Edit keywords, the string, comment, or deprecated call. The application recomputes a bounded run table only after committed value changes; caret geometry, selection, IME, and paint share that retained shaped buffer.")
                            .text_sm()
                            .text_color(Color::rgb8(148, 163, 184)),
                    )
                    .child(
                        styled_text_area(self.editable_code())
                            .on_input(edit_code)
                            .placeholder("Write highlighted Rust-like text…")
                            .accessibility_label("Attributed code editor")
                            .font_family(quickgui::FontFamily::Monospace)
                            .text_sm()
                            .line_height(22.0)
                            .no_wrap()
                            .w_full()
                            .max_w(700.0)
                            .h(176.0)
                            .flex_none(),
                    )
                    .child(
                        text_area(self.pasted.clone())
                            .on_input(edit_paste)
                            .placeholder("Paste the copied immutable selection here…")
                            .w_full()
                            .max_w(700.0)
                            .h(92.0)
                            .flex_none(),
                    )
                    .child(
                        text(format!(
                            "Previous frame: {} text areas · {} cached · {} reshaped · {:.2} ms CPU",
                            metrics.render.text_areas,
                            metrics.render.cached_text_areas,
                            metrics.render.reshaped_text_areas,
                            metrics.cpu_milliseconds(),
                        ))
                        .text_xs()
                        .text_color(Color::rgb8(148, 163, 184)),
                    )
                    .child(
                        text("Drag across prose and code to select continuously. Cmd-C copies, Cmd-A selects all immutable text, Escape clears, and Cmd-Q quits. Selection and decoration changes schedule no idle frames.")
                            .text_xs()
                            .text_color(Color::rgb8(100, 116, 139)),
                    ),
            )
    }
}

fn push_one_if_present(
    content: &str,
    needle: &str,
    style: HighlightStyle,
    highlights: &mut Vec<(Range<usize>, HighlightStyle)>,
) {
    if let Some(start) = content.find(needle) {
        highlights.push((start..start + needle.len(), style));
    }
}

fn push_all(
    content: &str,
    needle: &str,
    style: HighlightStyle,
    highlights: &mut Vec<(Range<usize>, HighlightStyle)>,
) {
    highlights.extend(
        content
            .match_indices(needle)
            .map(|(start, value)| (start..start + value.len(), style.clone())),
    );
}

fn non_overlapping(
    mut highlights: Vec<(Range<usize>, HighlightStyle)>,
) -> Vec<(Range<usize>, HighlightStyle)> {
    highlights.sort_by_key(|(range, _)| (range.start, range.end));
    let mut previous_end = 0;
    highlights.retain(|(range, _)| {
        if range.start < previous_end {
            return false;
        }
        previous_end = range.end;
        true
    });
    highlights
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn editable_highlighter_stays_sorted_bounded_and_unicode_safe() {
        let demo = StyledTextDemo {
            warm: false,
            editable: Arc::from("let value = \"GPU cached\"; // str 🙂\nstr"),
            pasted: Arc::from(""),
        };
        let highlighted = demo.editable_code();

        assert!(highlighted.highlights().len() <= quickgui::MAX_TEXT_HIGHLIGHTS);
        assert!(
            highlighted
                .highlights()
                .windows(2)
                .all(|ranges| { ranges[0].range().end <= ranges[1].range().start })
        );
        assert!(highlighted.highlights().iter().all(|highlight| {
            highlighted
                .content()
                .is_char_boundary(highlight.range().start)
                && highlighted
                    .content()
                    .is_char_boundary(highlight.range().end)
        }));
    }
}
