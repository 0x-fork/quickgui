use quickgui::{
    Application, Color, Element, GridTrack, TitleBarStyle, View, ViewContext, div, text,
};

fn main() -> Result<(), quickgui::AppError> {
    Application::new().run(|cx| {
        cx.open_window(
            quickgui::WindowOptions::new("QuickGUI — CSS Grid")
                .size(920.0, 680.0)
                .title_bar_style(TitleBarStyle::HiddenInset)
                .traffic_light_position(16.0, 13.0),
            GridLayoutDemo,
        );
    })
}

struct GridLayoutDemo;

impl GridLayoutDemo {
    fn panel(label: &'static str, detail: &'static str, color: Color) -> Element {
        div()
            .size_full()
            .min_w(0.0)
            .min_h(0.0)
            .flex_col()
            .items_center()
            .justify_center()
            .gap_1()
            .p_3()
            .rounded_xl()
            .border(1.0, Color::rgba8(255, 255, 255, 22))
            .bg(color)
            .child(text(label).font_semibold())
            .child(
                text(detail)
                    .text_sm()
                    .text_color(Color::rgba8(255, 255, 255, 166)),
            )
    }

    fn metric(label: &'static str, value: &'static str) -> Element {
        div()
            .min_w(0.0)
            .flex_col()
            .gap_1()
            .p_3()
            .rounded_lg()
            .bg(Color::rgba8(255, 255, 255, 14))
            .child(text(label).text_xs().text_color(Color::rgb8(148, 163, 184)))
            .child(text(value).text_lg().font_semibold())
    }

    fn content_panel() -> Element {
        div()
            .size_full()
            .min_w(0.0)
            .min_h(0.0)
            .flex_col()
            .gap_3()
            .p_4()
            .rounded_xl()
            .border(1.0, Color::rgb8(51, 65, 85))
            .bg(Color::rgb8(18, 24, 34))
            .child(text("Content").text_xl().font_bold())
            .child(
                text("The outer grid mixes fixed and flexible tracks. This nested grid uses the GPUI-compatible equal-column helper.")
                    .text_sm()
                    .text_color(Color::rgb8(148, 163, 184)),
            )
            .child(
                div()
                    .grid()
                    .grid_cols(3)
                    .gap_2()
                    .children([
                        Self::metric("Layout", "Taffy Grid"),
                        Self::metric("Paint", "Retained"),
                        Self::metric("Idle", "Sleeping"),
                    ]),
            )
            .child(
                div()
                    .flex_1()
                    .min_h(0.0)
                    .rounded_lg()
                    .bg(Color::rgba8(59, 130, 246, 18))
                    .items_center()
                    .justify_center()
                    .p_4()
                    .child(
                        text("Resize below 700 pt to switch to the stacked layout. No animation or idle redraw loop is used.")
                            .text_color(Color::rgb8(147, 197, 253)),
                    ),
            )
    }

    fn wide_layout() -> Element {
        div()
            .size_full()
            .grid()
            .grid_template_columns([
                GridTrack::px(176.0),
                GridTrack::minmax_px_fr(240.0, 1.0),
                GridTrack::fit_content_px(156.0),
            ])
            .grid_template_rows([GridTrack::px(62.0), GridTrack::fr(1.0), GridTrack::px(50.0)])
            .gap_3()
            .child(
                Self::panel(
                    "Header",
                    "col-span-full · explicit row 1",
                    Color::rgb8(30, 41, 59),
                )
                .col_span_full()
                .row_start(1),
            )
            .child(
                Self::panel("Navigation", "176 px fixed track", Color::rgb8(49, 46, 129))
                    .col_start(1)
                    .row_start(2),
            )
            .child(Self::content_panel().col_start(2).row_start(2))
            .child(
                Self::panel("Inspector", "fit-content(156 px)", Color::rgb8(88, 28, 135))
                    .col_start(3)
                    .row_start(2),
            )
            .child(
                Self::panel(
                    "Footer",
                    "negative grid lines resolve from the explicit edge",
                    Color::rgb8(15, 118, 110),
                )
                .col_start(1)
                .col_end(-1)
                .row_start(3),
            )
    }

    fn narrow_layout() -> Element {
        div()
            .w_full()
            .min_h(740.0)
            .flex_col()
            .gap_3()
            .child(
                Self::panel(
                    "Header",
                    "Responsive stacked layout",
                    Color::rgb8(30, 41, 59),
                )
                .h(62.0)
                .flex_none(),
            )
            .child(
                Self::panel(
                    "Navigation",
                    "The view rebuilds only on the resize event",
                    Color::rgb8(49, 46, 129),
                )
                .h(92.0)
                .flex_none(),
            )
            .child(Self::content_panel().h(380.0).flex_none())
            .child(
                Self::panel("Inspector", "No layout polling", Color::rgb8(88, 28, 135))
                    .h(92.0)
                    .flex_none(),
            )
            .child(
                Self::panel(
                    "Footer",
                    "Ordinary flex fallback",
                    Color::rgb8(15, 118, 110),
                )
                .h(62.0)
                .flex_none(),
            )
    }
}

impl View for GridLayoutDemo {
    fn render(&mut self, cx: &mut ViewContext<'_, Self>) -> impl quickgui::IntoElement {
        let narrow = cx.size().width < 700.0;
        let metrics = cx.metrics();

        div()
            .size_full()
            .flex_col()
            .bg(Color::rgb8(10, 14, 22))
            .text_color(Color::rgb8(226, 232, 240))
            .child(
                div()
                    .h(52.0)
                    .flex_none()
                    .items_center()
                    .justify_center()
                    .app_region_drag()
                    .child(text("CSS Grid · retained layout").font_semibold()),
            )
            .child(
                div()
                    .flex_1()
                    .min_h(0.0)
                    .overflow_y_scroll()
                    .p_4()
                    .child(if narrow {
                        Self::narrow_layout()
                    } else {
                        Self::wide_layout()
                    }),
            )
            .child(
                div().h(28.0).flex_none().px_4().items_center().child(
                    text(format!(
                        "Previous frame: {:.2} ms CPU · {} draw calls · {} reshaped text areas",
                        metrics.cpu_milliseconds(),
                        metrics.render.draw_calls,
                        metrics.render.reshaped_text_areas,
                    ))
                    .text_xs()
                    .text_color(Color::rgb8(100, 116, 139)),
                ),
            )
    }
}
