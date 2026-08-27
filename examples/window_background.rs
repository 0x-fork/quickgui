use std::sync::Arc;

use quickgui::{
    App, AppConfig, Color, Element, IntoElement, TitleBarStyle, View, ViewContext,
    WindowBackgroundAppearance, button, div, text,
};

fn main() -> Result<(), quickgui::AppError> {
    App::new(BackgroundDemo {
        mode: WindowBackgroundAppearance::Blurred,
    })
    .config(
        AppConfig::new("QuickGUI — Window background")
            .size(760.0, 560.0)
            .minimum_size(620.0, 460.0)
            .title_bar_style(TitleBarStyle::HiddenInset)
            .traffic_light_position(16.0, 14.0)
            .window_background(WindowBackgroundAppearance::Blurred)
            .background(Color::rgba8(13, 17, 25, 198)),
    )
    .run()
}

struct BackgroundDemo {
    mode: WindowBackgroundAppearance,
}

impl View for BackgroundDemo {
    fn render(&mut self, cx: &mut ViewContext<'_, Self>) -> impl IntoElement {
        let state = cx.window_state();
        let metrics = cx.metrics();
        let opaque = cx.listener("background-opaque", |this, cx| {
            if this.mode != WindowBackgroundAppearance::Opaque {
                this.mode = WindowBackgroundAppearance::Opaque;
                cx.set_window_background_appearance(WindowBackgroundAppearance::Opaque)
                    .expect("listener belongs to a window");
                cx.invalidate();
            }
        });
        let transparent = cx.listener("background-transparent", |this, cx| {
            if this.mode != WindowBackgroundAppearance::Transparent {
                this.mode = WindowBackgroundAppearance::Transparent;
                cx.set_window_background_appearance(WindowBackgroundAppearance::Transparent)
                    .expect("listener belongs to a window");
                cx.invalidate();
            }
        });
        let blurred = cx.listener("background-blurred", |this, cx| {
            if this.mode != WindowBackgroundAppearance::Blurred {
                this.mode = WindowBackgroundAppearance::Blurred;
                cx.set_window_background_appearance(WindowBackgroundAppearance::Blurred)
                    .expect("listener belongs to a window");
                cx.invalidate();
            }
        });

        div()
            .size_full()
            .flex_col()
            .text_color(Color::rgb8(242, 245, 250))
            .child(
                div()
                    .h(62.0)
                    .w_full()
                    .flex_none()
                    .flex_row()
                    .items_center()
                    .justify_between()
                    .padding(0.0, 18.0, 0.0, 88.0)
                    .border(1.0, Color::rgba8(255, 255, 255, 34))
                    .bg(Color::rgba8(24, 29, 39, 180))
                    .app_region_drag()
                    .child(
                        div()
                            .flex_col()
                            .child(text("Compositor backgrounds").text_lg().font_bold())
                            .child(
                                text("Opaque, desktop transparency, and native macOS blur")
                                    .text_xs()
                                    .text_color(Color::rgb8(170, 181, 199)),
                            ),
                    )
                    .child(
                        text(format!("frame {}", metrics.frame_number))
                            .text_xs()
                            .text_color(Color::rgb8(164, 176, 197)),
                    ),
            )
            .child(
                div()
                    .flex_1()
                    .min_h(0.0)
                    .overflow_y_scroll()
                    .p_6()
                    .flex_col()
                    .gap_5()
                    .child(
                        div()
                            .p_5()
                            .flex_col()
                            .gap_4()
                            .rounded_2xl()
                            .border(1.0, Color::rgba8(255, 255, 255, 42))
                            .bg(Color::rgba8(28, 34, 46, 205))
                            .shadow_lg()
                            .child(text("Window background").text_xl().font_bold())
                            .child(
                                text("The retained scene keeps its alpha channel all the way through the Metal surface. Blur is supplied by the native compositor behind the window, not by rerendering the application.")
                                    .max_w(650.0)
                                    .wrap()
                                    .text_sm()
                                    .text_color(Color::rgb8(177, 187, 204)),
                            )
                            .child(
                                div()
                                    .flex_row()
                                    .flex_wrap()
                                    .gap_2()
                                    .child(
                                        mode_button(
                                            "Opaque",
                                            self.mode == WindowBackgroundAppearance::Opaque,
                                        )
                                        .on_click(opaque),
                                    )
                                    .child(
                                        mode_button(
                                            "Transparent",
                                            self.mode == WindowBackgroundAppearance::Transparent,
                                        )
                                        .on_click(transparent),
                                    )
                                    .child(
                                        mode_button(
                                            "Blurred",
                                            self.mode == WindowBackgroundAppearance::Blurred,
                                        )
                                        .on_click(blurred),
                                    ),
                            ),
                    )
                    .child(
                        div()
                            .flex_row()
                            .flex_wrap()
                            .gap_3()
                            .child(readout(
                                "Retained policy",
                                mode_label(state.background_appearance),
                            ))
                            .child(readout(
                                "WGPU surface",
                                if state.background_appearance.is_transparent() {
                                    "Alpha compositing"
                                } else {
                                    "Opaque fast path"
                                },
                            ))
                            .child(readout(
                                "Native effect",
                                if state.background_appearance.is_blurred() {
                                    "Background blur"
                                } else {
                                    "None"
                                },
                            )),
                    )
                    .child(
                        text("Idle invariant: switching modes mutates only the compositor state that changed. A stationary translucent or blurred window schedules no QuickGUI frame, timer, or polling work.")
                            .max_w(680.0)
                            .wrap()
                            .text_xs()
                            .text_color(Color::rgb8(158, 170, 190)),
                    ),
            )
    }
}

fn mode_label(mode: WindowBackgroundAppearance) -> &'static str {
    match mode {
        WindowBackgroundAppearance::Opaque => "Opaque",
        WindowBackgroundAppearance::Transparent => "Transparent",
        WindowBackgroundAppearance::Blurred => "Blurred",
    }
}

fn mode_button(label: impl Into<Arc<str>>, selected: bool) -> Element {
    button()
        .min_w(128.0)
        .min_h(40.0)
        .px_4()
        .py_2()
        .flex_row()
        .items_center()
        .justify_center()
        .rounded_lg()
        .border(
            if selected { 2.0 } else { 1.0 },
            if selected {
                Color::rgb8(125, 211, 252)
            } else {
                Color::rgba8(255, 255, 255, 46)
            },
        )
        .bg(if selected {
            Color::rgba8(14, 116, 144, 160)
        } else {
            Color::rgba8(42, 49, 63, 185)
        })
        .hover(|style| style.bg(Color::rgba8(55, 65, 82, 220)))
        .focus(|style| style.border(2.0, Color::rgb8(125, 211, 252)))
        .child(text(label).font_semibold())
}

fn readout(label: impl Into<Arc<str>>, value: impl Into<Arc<str>>) -> Element {
    div()
        .min_w(190.0)
        .flex_1()
        .p_4()
        .flex_col()
        .gap_1()
        .rounded_xl()
        .border(1.0, Color::rgba8(255, 255, 255, 38))
        .bg(Color::rgba8(25, 31, 42, 190))
        .child(text(label).text_xs().text_color(Color::rgb8(166, 178, 198)))
        .child(text(value).font_semibold())
}
