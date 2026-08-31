use quickgui::{
    Application, Color, Element, FontWeight, IntoElement, TitleBarStyle, View, ViewContext, div,
    text,
};

fn main() -> Result<(), quickgui::AppError> {
    Application::new().run(|cx| {
        cx.open_window(
            quickgui::WindowOptions::new("QuickGUI — Web Flex Layout")
                .size(940.0, 660.0)
                .title_bar_style(TitleBarStyle::HiddenInset)
                .traffic_light_position(16.0, 13.0),
            FlexLayoutDemo,
        );
    })
}

struct FlexLayoutDemo;

impl FlexLayoutDemo {
    fn nav_item(label: &'static str, selected: bool) -> Element {
        div()
            .w_full()
            .px_3()
            .py_2()
            .rounded_lg()
            .bg(if selected {
                Color::rgb8(37, 99, 235)
            } else {
                Color::TRANSPARENT
            })
            .text_color(if selected {
                Color::WHITE
            } else {
                Color::rgb8(148, 163, 184)
            })
            .child(label)
    }

    fn card(index: usize, color: Color) -> Element {
        div()
            .w(156.0)
            .aspect_ratio(4.0 / 3.0)
            .flex_col()
            .flex_none()
            .justify_between()
            .p_3()
            .rounded_xl()
            .border(1.0, Color::rgba8(255, 255, 255, 22))
            .bg(color)
            .child(
                div()
                    .h(32.0)
                    .aspect_square()
                    .flex_none()
                    .items_center()
                    .justify_center()
                    .rounded_lg()
                    .bg(Color::rgba8(255, 255, 255, 28))
                    .child(text(format!("{:02}", index + 1)).font_semibold()),
            )
            .child(
                text("Retained Flexbox")
                    .text_sm()
                    .font_weight(FontWeight::SEMIBOLD),
            )
    }
}

impl View for FlexLayoutDemo {
    fn render(&mut self, _cx: &mut ViewContext<'_, Self>) -> impl IntoElement {
        let colors = [
            Color::rgb8(30, 64, 175),
            Color::rgb8(91, 33, 182),
            Color::rgb8(15, 118, 110),
            Color::rgb8(159, 18, 57),
            Color::rgb8(154, 52, 18),
            Color::rgb8(55, 48, 163),
        ];

        div()
            .size_full()
            .flex_col()
            .bg(Color::rgb8(9, 13, 22))
            .text_color(Color::rgb8(226, 232, 240))
            .child(
                div()
                    .h(52.0)
                    .flex_none()
                    .items_center()
                    .justify_center()
                    .app_region_drag()
                    .child(text("Web-shaped Flexbox · zero idle work").font_semibold()),
            )
            .child(
                div()
                    .flex_1()
                    .min_h(0.0)
                    .flex_row()
                    .child(
                        div()
                            .w(190.0)
                            .flex_none()
                            .flex_col()
                            .gap_2()
                            .p_4()
                            .border(1.0, Color::rgb8(30, 41, 59))
                            .child(text("Workspace").text_xs().font_semibold().mb_2())
                            .child(Self::nav_item("Overview", true))
                            .child(Self::nav_item("Documents", false))
                            .child(Self::nav_item("Settings", false))
                            .child(
                                text("flex: none · fixed sidebar")
                                    .text_xs()
                                    .text_color(Color::rgb8(100, 116, 139))
                                    .mt_auto(),
                            ),
                    )
                    .child(
                        div()
                            .flex_1()
                            .min_w(0.0)
                            .min_h(0.0)
                            .overflow_y_scroll()
                            .p_5()
                            .child(
                                div()
                                    .max_w(700.0)
                                    .mx_auto()
                                    .flex_col()
                                    .gap_4()
                                    .child(
                                        div()
                                            .flex_row_reverse()
                                            .items_center()
                                            .justify_between()
                                            .child(
                                                div()
                                                    .px_3()
                                                    .py_1()
                                                    .rounded_lg()
                                                    .bg(Color::rgb8(22, 101, 52))
                                                    .child(text("Sleeping").text_xs()),
                                            )
                                            .child(
                                                div()
                                                    .min_w(0.0)
                                                    .flex_col()
                                                    .gap_1()
                                                    .child(text("Flexible application shell").text_3xl())
                                                    .child(
                                                        text("Reverse flow, auto margins, independent gaps, aspect ratios, and complete flex sizing are ordinary retained layout state.")
                                                            .text_sm()
                                                            .text_color(Color::rgb8(148, 163, 184)),
                                                    ),
                                            ),
                                    )
                                    .child(
                                        div()
                                            .flex()
                                            .flex_wrap()
                                            .content_start()
                                            .justify_around()
                                            .gap_x_3()
                                            .gap_y_4()
                                            .children(
                                                colors
                                                    .into_iter()
                                                    .enumerate()
                                                    .map(|(index, color)| Self::card(index, color)),
                                            ),
                                    ),
                            ),
                    ),
            )
    }
}
