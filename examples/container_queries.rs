use quickgui::{
    App, AppConfig, Color, Element, IntoElement, Size, View, ViewContext, container_query, div,
    text,
};

fn main() -> Result<(), quickgui::AppError> {
    App::new(ResponsiveDashboard)
        .config(
            AppConfig::new("QuickGUI — container queries")
                .size(980.0, 680.0)
                .minimum_size(360.0, 420.0)
                .background(Color::rgb8(12, 16, 24)),
        )
        .run()
}

struct ResponsiveDashboard;

impl View for ResponsiveDashboard {
    fn render(&mut self, _cx: &mut ViewContext<'_, Self>) -> impl IntoElement {
        container_query(responsive_layout).bg(Color::rgb8(12, 16, 24))
    }
}

fn responsive_layout(size: Size) -> Element {
    let compact = size.width < 620.0;
    let columns = if size.width >= 880.0 { 3 } else { 2 };
    let content = div()
        .flex_1()
        .min_w(0.0)
        .min_h(0.0)
        .overflow_y_scroll()
        .p_5()
        .flex_col()
        .gap_5()
        .child(
            div()
                .flex_row()
                .items_end()
                .justify_between()
                .gap_3()
                .child(
                    div()
                        .flex_col()
                        .gap_1()
                        .child(text("Responsive workspace").text_2xl().font_bold())
                        .child(
                            text("The view below is declared from its actual parent size.")
                                .wrap()
                                .text_sm()
                                .text_color(Color::rgb8(148, 163, 184)),
                        ),
                )
                .child(
                    text(format!("{:.0} × {:.0}", size.width, size.height))
                        .flex_none()
                        .text_xs()
                        .text_color(Color::rgb8(125, 211, 252)),
                ),
        )
        .child(
            div()
                .when(!compact, |grid| grid.grid().grid_cols(columns))
                .when(compact, |stack| stack.flex_col())
                .gap_3()
                .children([
                    metric_card("GPU frame", "1.2 ms", Color::rgb8(56, 189, 248)),
                    metric_card("Idle CPU", "0.0%", Color::rgb8(74, 222, 128)),
                    metric_card("Retained nodes", "1,248", Color::rgb8(192, 132, 252)),
                ]),
        )
        .child(
            div()
                .min_h(260.0)
                .p_5()
                .flex_col()
                .gap_4()
                .rounded_2xl()
                .border(1.0, Color::rgb8(45, 55, 72))
                .bg(Color::rgb8(20, 26, 38))
                .child(text("Recent work").text_lg().font_bold())
                .children([
                    activity_row("Virtualized list", "Measured 100,000 rows", "2 min"),
                    activity_row("Native composition", "Reconciled one NSView", "8 min"),
                    activity_row("Text atlas", "Uploaded 14 new glyphs", "12 min"),
                ]),
        );

    div()
        .size_full()
        .text_color(Color::rgb8(241, 245, 249))
        .when(compact, |layout| layout.flex_col())
        .when(!compact, |layout| layout.flex_row())
        .child(navigation(compact))
        .child(content)
}

fn navigation(compact: bool) -> Element {
    div()
        .flex_none()
        .when(compact, |nav| nav.w_full().h(72.0).flex_row())
        .when(!compact, |nav| nav.w(210.0).h_full().flex_col())
        .items_center()
        .gap_3()
        .padding(16.0, 18.0, 16.0, 18.0)
        .border(1.0, Color::rgb8(38, 47, 63))
        .bg(Color::rgb8(16, 21, 31))
        .child(
            div()
                .size(34.0, 34.0)
                .flex_none()
                .items_center()
                .justify_center()
                .rounded_lg()
                .bg(Color::rgb8(37, 99, 235))
                .child(text("Q").font_bold()),
        )
        .child(
            text(if compact {
                "Overview"
            } else {
                "QuickGUI\nOverview"
            })
            .wrap()
            .font_semibold(),
        )
}

fn metric_card(label: &'static str, value: &'static str, accent: Color) -> Element {
    div()
        .min_h(122.0)
        .p_4()
        .flex_col()
        .justify_between()
        .rounded_2xl()
        .border(1.0, Color::rgb8(45, 55, 72))
        .bg(Color::rgb8(20, 26, 38))
        .child(text(label).text_sm().text_color(Color::rgb8(148, 163, 184)))
        .child(text(value).text_2xl().font_bold().text_color(accent))
}

fn activity_row(title: &'static str, detail: &'static str, age: &'static str) -> Element {
    div()
        .w_full()
        .p_3()
        .flex_row()
        .items_center()
        .justify_between()
        .gap_3()
        .rounded_xl()
        .bg(Color::rgb8(25, 33, 47))
        .child(
            div()
                .min_w(0.0)
                .flex_1()
                .flex_col()
                .child(text(title).font_semibold())
                .child(
                    text(detail)
                        .wrap()
                        .text_xs()
                        .text_color(Color::rgb8(148, 163, 184)),
                ),
        )
        .child(
            text(age)
                .flex_none()
                .text_xs()
                .text_color(Color::rgb8(100, 116, 139)),
        )
}
