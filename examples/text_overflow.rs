use quickgui::{Application, Color, IntoElement, View, ViewContext, div, text};

const LONG_PATH: &str =
    "/Users/egoist/dev/quickgui/examples/a-very-long-directory/important-file.rs";
const PARAGRAPH: &str = "QuickGUI keeps Unicode 🙂, bidirectional shaping مرحبا, selection, and styled ranges mapped to the original text while only the visible clamped projection reaches the GPU.";

struct TextOverflowExample;

impl TextOverflowExample {
    fn row(label: &'static str, content: impl IntoElement) -> quickgui::Element {
        div()
            .grid()
            .grid_template_columns([quickgui::GridTrack::px(116.0), quickgui::GridTrack::fr(1.0)])
            .gap_3()
            .items_start()
            .child(
                text(label)
                    .text_xs()
                    .font_semibold()
                    .text_color(Color::rgb8(148, 163, 184)),
            )
            .child(content)
    }
}

impl View for TextOverflowExample {
    fn render(&mut self, _cx: &mut ViewContext<'_, Self>) -> impl IntoElement {
        let panel = div()
            .w_full()
            .max_w(720.0)
            .flex_col()
            .gap_4()
            .p_5()
            .rounded_xl()
            .border(1.0, Color::rgb8(51, 65, 85))
            .bg(Color::rgb8(24, 28, 36))
            .child(
                text("Web-like text overflow")
                    .w_full()
                    .text_center()
                    .text_xl()
                    .font_semibold(),
            )
            .child(Self::row(
                "Full wrapping",
                text(PARAGRAPH).w_full().text_sm(),
            ))
            .child(Self::row(
                "truncate()",
                text(PARAGRAPH).w_full().min_w(0.0).truncate().text_sm(),
            ))
            .child(Self::row(
                "Start ellipsis",
                text(LONG_PATH)
                    .w_full()
                    .min_w(0.0)
                    .whitespace_nowrap()
                    .text_ellipsis_start()
                    .overflow_hidden()
                    .text_sm(),
            ))
            .child(Self::row(
                "Middle ellipsis",
                text(LONG_PATH)
                    .w_full()
                    .min_w(0.0)
                    .whitespace_nowrap()
                    .text_ellipsis_middle()
                    .overflow_hidden()
                    .text_sm(),
            ))
            .child(Self::row(
                "Two-line clamp",
                text(PARAGRAPH)
                    .w_full()
                    .min_w(0.0)
                    .line_clamp(2)
                    .text_ellipsis()
                    .text_sm(),
            ));

        div()
            .size_full()
            .flex_col()
            .items_center()
            .justify_center()
            .p_6()
            .bg(Color::rgb8(15, 18, 24))
            .text_color(Color::rgb8(241, 245, 249))
            .child(panel)
    }
}

fn main() -> Result<(), quickgui::AppError> {
    Application::new().run(|cx| {
        cx.open_window(
            quickgui::WindowOptions::new("QuickGUI Text Overflow").size(840.0, 600.0),
            TextOverflowExample,
        );
    })
}
