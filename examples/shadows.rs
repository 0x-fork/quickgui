use quickgui::{App, BoxShadow, Color, Element, View, ViewContext, button, div, text};

fn main() -> Result<(), quickgui::AppError> {
    App::new(ShadowDemo)
        .title("QuickGUI — analytic GPU shadows")
        .size(1120.0, 720.0)
        .run()
}

struct ShadowDemo;

impl ShadowDemo {
    fn elevation_card(label: &'static str, detail: &'static str, card: Element) -> Element {
        div()
            .flex_1()
            .min_w(0.0)
            .flex_col()
            .gap_3()
            .child(card)
            .child(
                div()
                    .flex_col()
                    .gap_1()
                    .child(text(label).font_semibold())
                    .child(
                        text(detail)
                            .text_sm()
                            .text_color(Color::rgb8(100, 116, 139)),
                    ),
            )
    }

    fn blank_card() -> Element {
        div()
            .h(126.0)
            .rounded_xl()
            .border(1.0, Color::rgba8(148, 163, 184, 80))
            .bg(Color::WHITE)
    }
}

impl View for ShadowDemo {
    fn render(&mut self, _cx: &mut ViewContext<'_, Self>) -> impl quickgui::IntoElement {
        let colored = Self::blank_card()
            .shadows([
                BoxShadow::new(0.0, 16.0, Color::rgba8(37, 99, 235, 65))
                    .blur_radius(32.0)
                    .spread_radius(-8.0),
                BoxShadow::new(-16.0, -8.0, Color::rgba8(168, 85, 247, 50))
                    .blur_radius(28.0)
                    .spread_radius(-10.0),
            ])
            .child(
                div()
                    .size_full()
                    .items_center()
                    .justify_center()
                    .child(text("Layered color").font_semibold()),
            );

        let inset = div()
            .h(52.0)
            .px_4()
            .items_center()
            .rounded_lg()
            .border(1.0, Color::rgb8(203, 213, 225))
            .bg(Color::WHITE)
            .shadow(
                BoxShadow::new(0.0, 2.0, Color::rgba8(15, 23, 42, 34))
                    .blur_radius(5.0)
                    .spread_radius(0.5)
                    .inset(true),
            )
            .child(
                text("Inset shadows stay inside the rounded box")
                    .text_sm()
                    .text_color(Color::rgb8(71, 85, 105)),
            );

        let interactive = button()
            .h(52.0)
            .px_4()
            .items_center()
            .justify_center()
            .rounded_lg()
            .bg(Color::rgb8(37, 99, 235))
            .text_color(Color::WHITE)
            .font_semibold()
            .shadow_md()
            .hover(|style| {
                style.bg(Color::rgb8(59, 130, 246)).shadow(
                    BoxShadow::new(0.0, 10.0, Color::rgba8(37, 99, 235, 90))
                        .blur_radius(22.0)
                        .spread_radius(-6.0),
                )
            })
            .active(|style| {
                style.bg(Color::rgb8(29, 78, 216)).shadow(
                    BoxShadow::new(0.0, 2.0, Color::rgba8(15, 23, 42, 60))
                        .blur_radius(5.0)
                        .spread_radius(-1.0),
                )
            })
            .child("Hover and press me");

        div()
            .size_full()
            .flex_col()
            .gap(24.0)
            .p(24.0)
            .bg(Color::rgb8(241, 245, 249))
            .text_color(Color::rgb8(15, 23, 42))
            .child(
                div()
                    .flex_col()
                    .gap_1()
                    .child(text("Box shadows").text_size(30.0).font_bold())
                    .child(
                        text("Drop and inset shadows are analytic GPU shapes—no blur textures or CPU rasterization.")
                            .text_color(Color::rgb8(71, 85, 105)),
                    ),
            )
            .child(
                div()
                    .flex_row()
                    .gap(20.0)
                    .child(Self::elevation_card(
                        "shadow-sm",
                        "A quiet one-pixel lift.",
                        Self::blank_card().shadow_sm(),
                    ))
                    .child(Self::elevation_card(
                        "shadow-md",
                        "Two ordered shadow layers.",
                        Self::blank_card().shadow_md(),
                    ))
                    .child(Self::elevation_card(
                        "shadow-lg",
                        "A larger floating surface.",
                        Self::blank_card().shadow_lg(),
                    ))
                    .child(Self::elevation_card(
                        "shadow-xl",
                        "Wide blur with bounded overdraw.",
                        Self::blank_card().shadow_xl(),
                    )),
            )
            .child(
                div()
                    .flex_row()
                    .gap(24.0)
                    .child(
                        div()
                            .flex_1()
                            .min_w(0.0)
                            .flex_col()
                            .gap_3()
                            .child(colored)
                            .child(
                                text("Custom declaration-ordered shadows")
                                    .text_sm()
                                    .font_semibold(),
                            ),
                    )
                    .child(
                        div()
                            .flex_1()
                            .min_w(0.0)
                            .flex_col()
                            .gap_4()
                            .child(inset)
                            .child(interactive),
                    ),
            )
    }
}
