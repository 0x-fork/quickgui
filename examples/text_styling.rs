//! Extended text styling: shadows, tracking, case mapping, overlines, and break control.

use quickgui::{
    Color, Element, Hyphens, IntoElement, OverflowWrap, TextTransform, View, ViewContext,
    WordBreak, div, text,
};

const PARAGRAPH: &str = "QuickGUI shapes retained text once per canonical style key.";
const LONG_WORD: &str = "Kraftfahrzeughaftpflichtversicherungsvertragsbedingungen";
/// The same word with author-placed soft hyphens (`U+00AD`) at its component boundaries.
const HYPHENATED: &str = "Kraft\u{00ad}fahrzeug\u{00ad}haftpflicht\u{00ad}versicherung";

fn ink() -> Color {
    Color::rgb8(241, 245, 249)
}

fn muted() -> Color {
    Color::rgb8(148, 163, 184)
}

fn accent() -> Color {
    Color::rgb8(56, 189, 248)
}

fn panel_fill() -> Color {
    Color::rgb8(24, 28, 36)
}

fn border() -> Color {
    Color::rgb8(51, 65, 85)
}

struct TextStylingExample;

fn row(label: &'static str, content: Element) -> Element {
    div()
        .w_full()
        .flex_col()
        .gap_1()
        .child(text(label).text_xs().font_semibold().text_color(muted()))
        .child(content)
}

impl View for TextStylingExample {
    fn render(&mut self, _cx: &mut ViewContext<'_, Self>) -> impl IntoElement {
        let panel = div()
            .w_full()
            .max_w(760.0)
            .flex_col()
            .gap_4()
            .p_5()
            .rounded_xl()
            .border(1.0, border())
            .bg(panel_fill())
            .child(text("Extended text styling").text_xl().font_semibold())
            .child(row(
                "text_shadow(2, 2, 6, ...)",
                text("Soft drop shadow")
                    .text_2xl()
                    .font_semibold()
                    .text_shadow(2.0, 2.0, 6.0, Color::rgba8(0, 0, 0, 200)),
            ))
            .child(row(
                "letter_spacing(3.0)",
                text("W I D E   T R A C K I N G")
                    .text_sm()
                    .letter_spacing(3.0),
            ))
            .child(row(
                "word_spacing(10.0)",
                text("extra space between words")
                    .text_sm()
                    .word_spacing(10.0),
            ))
            .child(row(
                "uppercase() / capitalize()",
                div()
                    .flex_row()
                    .gap_4()
                    .child(text("shouting quietly").text_sm().uppercase())
                    .child(
                        text("title case example")
                            .text_sm()
                            .text_transform(TextTransform::Capitalize),
                    ),
            ))
            .child(row(
                "overline()",
                text("Above the ascent").text_sm().overline_color(accent()),
            ))
            .child(row(
                "word_break(BreakAll)",
                div()
                    .w(220.0)
                    .child(text(LONG_WORD).text_sm().word_break(WordBreak::BreakAll)),
            ))
            .child(row(
                "overflow_wrap(BreakWord)",
                div().w(220.0).child(
                    text(LONG_WORD)
                        .text_sm()
                        .overflow_wrap(OverflowWrap::BreakWord),
                ),
            ))
            .child(row(
                "hyphens(Manual) with U+00AD",
                div()
                    .w(180.0)
                    .child(text(HYPHENATED).text_sm().hyphens(Hyphens::Manual)),
            ))
            .child(row(
                "hyphens(None) removes them",
                div()
                    .w(180.0)
                    .child(text(HYPHENATED).text_sm().hyphens(Hyphens::None)),
            ))
            .child(row(
                "inherited through a subtree",
                div()
                    .letter_spacing(1.0)
                    .text_shadow(0.0, 1.0, 0.0, Color::rgba8(0, 0, 0, 180))
                    .child(text(PARAGRAPH).text_sm()),
            ));

        div()
            .size_full()
            .flex_col()
            .items_center()
            .justify_center()
            .p_6()
            .bg(Color::rgb8(15, 18, 24))
            .text_color(ink())
            .child(panel)
    }
}

fn main() -> Result<(), quickgui::AppError> {
    quickgui::Application::new().run(|cx| {
        cx.open_window(
            quickgui::WindowOptions::new("QuickGUI Text Styling").size(900.0, 760.0),
            TextStylingExample,
        );
    })
}
