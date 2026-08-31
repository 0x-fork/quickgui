use std::sync::Arc;

use quickgui::{
    Application, Color, Element, Event, IntoElement, TitleBarStyle, View, ViewContext,
    WindowAppearance, WindowOptions, button, div, text,
};

fn main() -> Result<(), quickgui::AppError> {
    Application::new().run(|cx| {
        cx.open_window(
            WindowOptions::new("QuickGUI — Native appearance")
                .size(760.0, 560.0)
                .minimum_size(620.0, 460.0)
                .title_bar_style(TitleBarStyle::HiddenInset)
                .traffic_light_position(16.0, 14.0)
                .follow_system_appearance()
                .background(Color::rgb8(20, 23, 29)),
            AppearanceDemo::default(),
        );
    })
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
enum AppearanceMode {
    #[default]
    System,
    Light,
    Dark,
}

impl AppearanceMode {
    const fn label(self) -> &'static str {
        match self {
            Self::System => "Following system",
            Self::Light => "Forced light",
            Self::Dark => "Forced dark",
        }
    }
}

#[derive(Default)]
struct AppearanceDemo {
    mode: AppearanceMode,
    native_changes: u64,
}

impl View for AppearanceDemo {
    fn event(&mut self, event: &Event, _cx: &mut quickgui::EventContext) {
        if matches!(event, Event::AppearanceChanged(_)) {
            self.native_changes = self.native_changes.saturating_add(1);
            // `render` observes `cx.appearance()`, so the runtime already invalidates exactly once.
        }
    }

    fn render(&mut self, cx: &mut ViewContext<'_, Self>) -> impl IntoElement {
        let appearance = cx.appearance();
        let palette = Palette::for_appearance(appearance);
        let metrics = cx.metrics();
        let use_system = cx.listener("appearance-system", |this, cx| {
            this.mode = AppearanceMode::System;
            cx.follow_system_window_appearance()
                .expect("listener belongs to a window");
            cx.invalidate();
        });
        let use_light = cx.listener("appearance-light", |this, cx| {
            this.mode = AppearanceMode::Light;
            cx.set_window_appearance(WindowAppearance::Light)
                .expect("listener belongs to a window");
            cx.invalidate();
        });
        let use_dark = cx.listener("appearance-dark", |this, cx| {
            this.mode = AppearanceMode::Dark;
            cx.set_window_appearance(WindowAppearance::Dark)
                .expect("listener belongs to a window");
            cx.invalidate();
        });

        div()
            .size_full()
            .flex_col()
            .bg(palette.page)
            .text_color(palette.text)
            .child(
                div()
                    .h(62.0)
                    .w_full()
                    .flex_none()
                    .flex_row()
                    .items_center()
                    .justify_between()
                    .padding(0.0, 18.0, 0.0, 88.0)
                    .border(1.0, palette.border)
                    .bg(palette.chrome)
                    .app_region_drag()
                    .child(
                        div()
                            .flex_col()
                            .child(text("Native appearance").text_lg().font_bold())
                            .child(
                                text("System-following or explicit per-window light/dark chrome")
                                    .text_xs()
                                    .text_color(palette.muted),
                            ),
                    )
                    .child(
                        text(format!("frame {}", metrics.frame_number))
                            .text_xs()
                            .text_color(palette.muted),
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
                            .border(1.0, palette.border)
                            .bg(palette.card)
                            .shadow_lg()
                            .child(text("Window preference").text_xl().font_bold())
                            .child(
                                text("Choose a native appearance override, or hand control back to macOS. The same effective value drives the GPU palette through cx.appearance().")
                                    .max_w(650.0)
                                    .wrap()
                                    .text_sm()
                                    .text_color(palette.muted),
                            )
                            .child(
                                div()
                                    .flex_row()
                                    .flex_wrap()
                                    .gap_2()
                                    .child(
                                        appearance_button(
                                            "System",
                                            self.mode == AppearanceMode::System,
                                            palette,
                                        )
                                        .on_click(use_system),
                                    )
                                    .child(
                                        appearance_button(
                                            "Light",
                                            self.mode == AppearanceMode::Light,
                                            palette,
                                        )
                                        .on_click(use_light),
                                    )
                                    .child(
                                        appearance_button(
                                            "Dark",
                                            self.mode == AppearanceMode::Dark,
                                            palette,
                                        )
                                        .on_click(use_dark),
                                    ),
                            ),
                    )
                    .child(
                        div()
                            .flex_row()
                            .flex_wrap()
                            .gap_3()
                            .child(readout(
                                "Effective",
                                match appearance {
                                    WindowAppearance::Light => "Light",
                                    WindowAppearance::Dark => "Dark",
                                },
                                palette,
                            ))
                            .child(readout("Preference", self.mode.label(), palette))
                            .child(readout(
                                "Native changes",
                                self.native_changes.to_string(),
                                palette,
                            )),
                    )
                    .child(
                        text("Idle invariant: appearance is delivered by the native event loop. No preference polling, timer, or continuous redraw is installed; the frame counter remains still while clean.")
                            .max_w(680.0)
                            .wrap()
                            .text_xs()
                            .text_color(palette.muted),
                    ),
            )
    }
}

#[derive(Clone, Copy)]
struct Palette {
    page: Color,
    chrome: Color,
    card: Color,
    button: Color,
    hover: Color,
    selected: Color,
    text: Color,
    muted: Color,
    border: Color,
    focus: Color,
}

impl Palette {
    fn for_appearance(appearance: WindowAppearance) -> Self {
        match appearance {
            WindowAppearance::Light => Self {
                page: Color::rgb8(242, 244, 248),
                chrome: Color::rgb8(251, 252, 254),
                card: Color::rgb8(255, 255, 255),
                button: Color::rgb8(246, 248, 251),
                hover: Color::rgb8(235, 239, 245),
                selected: Color::rgb8(219, 232, 255),
                text: Color::rgb8(28, 33, 43),
                muted: Color::rgb8(94, 105, 124),
                border: Color::rgb8(210, 216, 226),
                focus: Color::rgb8(37, 99, 235),
            },
            WindowAppearance::Dark => Self {
                page: Color::rgb8(17, 19, 24),
                chrome: Color::rgb8(24, 27, 34),
                card: Color::rgb8(27, 31, 39),
                button: Color::rgb8(35, 40, 50),
                hover: Color::rgb8(46, 52, 65),
                selected: Color::rgb8(35, 56, 91),
                text: Color::rgb8(237, 240, 246),
                muted: Color::rgb8(154, 165, 184),
                border: Color::rgb8(58, 65, 79),
                focus: Color::rgb8(125, 211, 252),
            },
        }
    }
}

fn appearance_button(label: impl Into<Arc<str>>, selected: bool, palette: Palette) -> Element {
    button()
        .min_w(112.0)
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
                palette.focus
            } else {
                palette.border
            },
        )
        .bg(if selected {
            palette.selected
        } else {
            palette.button
        })
        .hover(move |style| style.bg(palette.hover))
        .focus(move |style| style.border(2.0, palette.focus))
        .child(text(label).font_semibold())
}

fn readout(label: impl Into<Arc<str>>, value: impl Into<Arc<str>>, palette: Palette) -> Element {
    div()
        .min_w(190.0)
        .flex_1()
        .p_4()
        .flex_col()
        .gap_1()
        .rounded_xl()
        .border(1.0, palette.border)
        .bg(palette.card)
        .child(text(label).text_xs().text_color(palette.muted))
        .child(text(value).font_semibold())
}
