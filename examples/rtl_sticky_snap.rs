//! Right-to-left layout, sticky positioning, and scroll snapping.
//!
//! The left column mirrors an entire subtree with `rtl()`, including its logical padding and
//! text alignment. The middle column pins section headers inside one scroll container. The right
//! column snaps a horizontal carousel to whole pages at the end of every scroll gesture.

use quickgui::{
    Application, Color, Direction, Element, IntoElement, SnapAlign, SnapStrictness, View,
    ViewContext, div, text,
};

fn ink() -> Color {
    Color::rgb8(241, 245, 249)
}

fn muted() -> Color {
    Color::rgb8(148, 163, 184)
}

fn panel_fill() -> Color {
    Color::rgb8(24, 28, 36)
}

fn border() -> Color {
    Color::rgb8(51, 65, 85)
}

fn accent() -> Color {
    Color::rgb8(56, 189, 248)
}

struct DirectionStickySnapExample;

fn panel(title: &'static str, body: Element) -> Element {
    div()
        .flex_col()
        .gap_3()
        .flex_1()
        .min_w(0.0)
        .p_4()
        .rounded_xl()
        .border(1.0, border())
        .bg(panel_fill())
        .child(text(title).text_sm().font_semibold().text_color(muted()))
        .child(body)
}

fn direction_column(direction: Direction, label: &'static str) -> Element {
    div()
        .direction(direction)
        .flex_col()
        .gap_2()
        .p_3()
        .ps(20.0)
        .pe(4.0)
        .rounded_lg()
        .border(1.0, border())
        .child(text(label).text_xs().text_color(muted()).text_start())
        .child(
            div()
                .flex_row()
                .gap_2()
                .child(div().size(28.0, 20.0).rounded_md().bg(accent()))
                .child(div().size(20.0, 20.0).rounded_md().bg(border()))
                .child(div().size(12.0, 20.0).rounded_md().bg(border())),
        )
        .child(text("مرحبا بالعالم — hello").text_sm().text_start())
}

fn sticky_section(title: &'static str, tint: Color) -> Element {
    let mut section = div().w_full().flex_col().flex_none();
    section = section.child(
        div()
            .w_full()
            .h(28.0)
            .flex_none()
            .sticky_top(0.0)
            .px_3()
            .justify_center()
            .bg(tint)
            .child(text(title).text_xs().font_semibold()),
    );
    for index in 0..6 {
        section = section.child(
            div()
                .w_full()
                .h(30.0)
                .flex_none()
                .px_3()
                .justify_center()
                .child(
                    text(format!("{title} row {index}"))
                        .text_xs()
                        .text_color(muted()),
                ),
        );
    }
    section
}

fn snap_page(index: usize, tint: Color) -> Element {
    div()
        .size(200.0, 120.0)
        .flex_none()
        .snap_align(SnapAlign::Start)
        .rounded_lg()
        .bg(tint)
        .items_center()
        .justify_center()
        .child(text(format!("page {index}")).font_semibold())
}

impl View for DirectionStickySnapExample {
    fn render(&mut self, _cx: &mut ViewContext<'_, Self>) -> impl IntoElement {
        let direction_panel = panel(
            "Layout direction",
            div()
                .flex_col()
                .gap_3()
                .child(direction_column(Direction::Ltr, "ltr()"))
                .child(direction_column(Direction::Rtl, "rtl()")),
        );

        let sticky_panel = panel(
            "Sticky headers",
            div()
                .w_full()
                .h(240.0)
                .overflow_y_scroll()
                .flex_col()
                .rounded_lg()
                .border(1.0, border())
                .child(sticky_section("Inbox", Color::rgb8(30, 64, 91)))
                .child(sticky_section("Archive", Color::rgb8(60, 40, 88)))
                .child(sticky_section("Trash", Color::rgb8(78, 42, 42))),
        );

        let snap_panel = panel(
            "Mandatory scroll snap",
            div()
                .w_full()
                .flex_col()
                .gap_2()
                .child(
                    div()
                        .w_full()
                        .overflow_x_scroll()
                        .scroll_snap_x(SnapStrictness::Mandatory)
                        .flex_row()
                        .gap_2()
                        .child(snap_page(0, Color::rgb8(30, 64, 91)))
                        .child(snap_page(1, Color::rgb8(60, 40, 88)))
                        .child(snap_page(2, Color::rgb8(78, 42, 42)).snap_stop_always())
                        .child(snap_page(3, Color::rgb8(28, 74, 60))),
                )
                .child(
                    text("Release a gesture between two pages: the carousel settles on one.")
                        .text_xs()
                        .text_color(muted()),
                ),
        );

        div()
            .size_full()
            .flex_col()
            .gap_4()
            .p_6()
            .bg(Color::rgb8(15, 18, 24))
            .text_color(ink())
            .child(
                text("Direction, sticky positioning, and scroll snapping")
                    .text_xl()
                    .font_semibold(),
            )
            .child(
                div()
                    .w_full()
                    .flex_row()
                    .gap_4()
                    .items_start()
                    .child(direction_panel)
                    .child(sticky_panel)
                    .child(snap_panel),
            )
    }
}

fn main() -> Result<(), quickgui::AppError> {
    Application::new().run(|cx| {
        cx.open_window(
            quickgui::WindowOptions::new("QuickGUI RTL, Sticky, and Scroll Snap")
                .size(1080.0, 700.0),
            DirectionStickySnapExample,
        );
    })
}
