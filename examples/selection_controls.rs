use std::{sync::Arc, time::Duration};

use quickgui::{
    App, Checkbox, Color, Element, Event, EventContext, Radio, RadioGroup, Switch, ToggleState,
    View, ViewContext, button, div, text,
};

const CONTROL_TRANSITION: Duration = Duration::from_millis(100);

fn main() -> Result<(), quickgui::AppError> {
    App::new(SelectionControlsDemo::default())
        .title("QuickGUI — Selection controls")
        .size(720.0, 640.0)
        .run()
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
enum Density {
    Compact,
    #[default]
    Comfortable,
    Spacious,
}

#[derive(Clone, Copy)]
struct GalleryColors {
    background: Color,
    surface: Color,
    border: Color,
    foreground: Color,
    muted: Color,
    accent: Color,
    control_background: Color,
    control_border: Color,
    mark: Color,
    hover: Color,
    active: Color,
    focus_ring: Color,
}

impl GalleryColors {
    fn new(light: bool) -> Self {
        if light {
            Self {
                background: Color::rgb8(241, 242, 246),
                surface: Color::rgb8(255, 255, 255),
                border: Color::rgb8(211, 214, 222),
                foreground: Color::rgb8(28, 30, 36),
                muted: Color::rgb8(99, 104, 116),
                accent: Color::rgb8(0, 122, 255),
                control_background: Color::rgb8(255, 255, 255),
                control_border: Color::rgb8(142, 142, 147),
                mark: Color::WHITE,
                hover: Color::rgba8(0, 0, 0, 8),
                active: Color::rgba8(0, 0, 0, 15),
                focus_ring: Color::rgb8(0, 122, 255),
            }
        } else {
            Self {
                background: Color::rgb8(17, 18, 21),
                surface: Color::rgb8(26, 28, 33),
                border: Color::rgb8(55, 59, 69),
                foreground: Color::rgb8(238, 240, 244),
                muted: Color::rgb8(155, 161, 174),
                accent: Color::rgb8(10, 132, 255),
                control_background: Color::rgb8(39, 41, 48),
                control_border: Color::rgb8(105, 110, 123),
                mark: Color::WHITE,
                hover: Color::rgba8(255, 255, 255, 10),
                active: Color::rgba8(255, 255, 255, 18),
                focus_ring: Color::rgb8(64, 156, 255),
            }
        }
    }
}

struct SelectionControlsDemo {
    save_automatically: bool,
    index_workspace: ToggleState,
    density: Density,
    sync_settings: bool,
    light: bool,
    status: Arc<str>,
}

impl Default for SelectionControlsDemo {
    fn default() -> Self {
        Self {
            save_automatically: true,
            index_workspace: ToggleState::Mixed,
            density: Density::Comfortable,
            sync_settings: false,
            light: false,
            status: Arc::from("Tab through controls; arrow keys move inside the radio group."),
        }
    }
}

impl SelectionControlsDemo {
    fn copy(title: &'static str, description: &'static str, muted: Color) -> Element {
        div()
            .min_w(0.0)
            .flex_1()
            .flex_col()
            .gap_1()
            .child(text(title).font_medium())
            .child(text(description).text_sm().text_color(muted))
    }

    fn section(title: &'static str, colors: GalleryColors, content: Element) -> Element {
        div()
            .w_full()
            .flex_col()
            .gap_3()
            .p_4()
            .rounded_xl()
            .border(1.0, colors.border)
            .bg(colors.surface)
            .child(text(title).text_sm().font_semibold())
            .child(content)
    }

    fn control_root(root: Element, colors: GalleryColors) -> Element {
        root.min_h(28.0)
            .flex_row()
            .items_center()
            .gap_2()
            .px(4.0)
            .py(4.0)
            .rounded_md()
            .hover(|state| state.bg(colors.hover))
            .active(|state| state.bg(colors.active))
            .focus(|state| state.border(2.0, colors.focus_ring))
            .disabled_style(|state| state.opacity(0.45))
            .transition(CONTROL_TRANSITION)
    }

    fn checkbox_indicator(control: Checkbox, colors: GalleryColors) -> Element {
        let selected = control.state() != ToggleState::Off;
        let indicator = div()
            .size(16.0, 16.0)
            .flex_none()
            .flex_row()
            .items_center()
            .justify_center()
            .rounded(4.0)
            .border(
                1.0,
                if selected {
                    colors.accent
                } else {
                    colors.control_border
                },
            )
            .bg(if selected {
                colors.accent
            } else {
                colors.control_background
            })
            .transition(CONTROL_TRANSITION);
        let indicator = match control.state() {
            ToggleState::Off => indicator,
            ToggleState::On => {
                indicator.child(text("✓").text_sm().font_bold().text_color(colors.mark))
            }
            ToggleState::Mixed => {
                indicator.child(div().size(8.0, 2.0).rounded(1.0).bg(colors.mark))
            }
        };
        control.indicator_part(indicator)
    }

    fn radio_indicator(control: Radio, colors: GalleryColors) -> Element {
        control.indicator_part(
            div()
                .size(16.0, 16.0)
                .flex_none()
                .flex_row()
                .items_center()
                .justify_center()
                .rounded(8.0)
                .border(
                    1.0,
                    if control.is_selected() {
                        colors.accent
                    } else {
                        colors.control_border
                    },
                )
                .bg(colors.control_background)
                .transition(CONTROL_TRANSITION)
                .children(
                    control
                        .is_selected()
                        .then(|| div().size(8.0, 8.0).rounded(4.0).bg(colors.accent)),
                ),
        )
    }

    fn switch_track(control: Switch, colors: GalleryColors) -> Element {
        let checked = control.is_checked();
        let knob_left = if checked { 14.0 } else { 2.0 };
        div()
            .relative()
            .size(30.0, 18.0)
            .flex_none()
            .rounded(9.0)
            .border(
                1.0,
                if checked {
                    colors.accent
                } else {
                    colors.control_border
                },
            )
            .bg(if checked {
                colors.accent
            } else {
                colors.control_background
            })
            .transition(CONTROL_TRANSITION)
            .child(
                control.thumb_part(
                    div()
                        .absolute()
                        .top(2.0)
                        .left(knob_left)
                        .size(14.0, 14.0)
                        .rounded(7.0)
                        .bg(colors.mark)
                        .shadow_sm()
                        .transition(CONTROL_TRANSITION),
                ),
            )
    }
}

impl View for SelectionControlsDemo {
    fn event(&mut self, event: &Event, cx: &mut EventContext) {
        if matches!(
            event,
            Event::KeyDown {
                key: quickgui::Key::Escape,
                ..
            }
        ) {
            cx.exit();
        }
    }

    fn render(&mut self, cx: &mut ViewContext<'_, Self>) -> impl quickgui::IntoElement {
        let colors = GalleryColors::new(self.light);

        let save = cx.listener("save-automatically", |view, cx: &mut EventContext| {
            view.save_automatically = !view.save_automatically;
            view.status = Arc::from(if view.save_automatically {
                "Automatic saving enabled"
            } else {
                "Automatic saving disabled"
            });
            cx.invalidate();
        });
        let index = cx.listener("index-workspace", |view, cx| {
            view.index_workspace = match view.index_workspace {
                ToggleState::Off => ToggleState::On,
                ToggleState::On => ToggleState::Mixed,
                ToggleState::Mixed => ToggleState::Off,
            };
            view.status = Arc::from(format!("Workspace index: {:?}", view.index_workspace));
            cx.invalidate();
        });
        let compact = cx.listener("density-compact", |view, cx| {
            view.density = Density::Compact;
            view.status = Arc::from("Compact density selected");
            cx.invalidate();
        });
        let comfortable = cx.listener("density-comfortable", |view, cx| {
            view.density = Density::Comfortable;
            view.status = Arc::from("Comfortable density selected");
            cx.invalidate();
        });
        let spacious = cx.listener("density-spacious", |view, cx| {
            view.density = Density::Spacious;
            view.status = Arc::from("Spacious density selected");
            cx.invalidate();
        });
        let sync = cx.listener("sync-settings", |view, cx| {
            view.sync_settings = !view.sync_settings;
            view.status = Arc::from(if view.sync_settings {
                "Settings sync enabled"
            } else {
                "Settings sync disabled"
            });
            cx.invalidate();
        });
        let theme = cx.listener("theme", |view, cx| {
            view.light = !view.light;
            view.status = Arc::from(if view.light {
                "Light application palette"
            } else {
                "Dark application palette"
            });
            cx.invalidate();
        });

        let save_control = Checkbox::new(self.save_automatically);
        let index_control = Checkbox::new(self.index_workspace);
        let checks = div()
            .flex_col()
            .gap_2()
            .child(
                save_control
                    .root_part(
                        Self::control_root(div(), colors)
                            .child(Self::checkbox_indicator(save_control, colors))
                            .child(Self::copy(
                                "Save automatically",
                                "Write edits after a short quiet period.",
                                colors.muted,
                            )),
                    )
                    .id("save-automatically")
                    .on_click(save),
            )
            .child(
                index_control
                    .root_part(
                        Self::control_root(div(), colors)
                            .child(Self::checkbox_indicator(index_control, colors))
                            .child(Self::copy(
                                "Index workspace",
                                "Mixed means only part of the workspace is indexed.",
                                colors.muted,
                            )),
                    )
                    .id("index-workspace")
                    .on_click(index),
            );

        let compact_control = Radio::new(self.density == Density::Compact);
        let comfortable_control = Radio::new(self.density == Density::Comfortable);
        let spacious_control = Radio::new(self.density == Density::Spacious);
        let densities = RadioGroup::new().root_part(
            div()
                .accessibility_label("Editor density")
                .flex_col()
                .gap_2()
                .children([
                    compact_control
                        .root_part(
                            Self::control_root(div(), colors)
                                .child(Self::radio_indicator(compact_control, colors))
                                .child(Self::copy(
                                    "Compact",
                                    "Fit the most information on screen.",
                                    colors.muted,
                                )),
                        )
                        .id("density-compact")
                        .on_click(compact),
                    comfortable_control
                        .root_part(
                            Self::control_root(div(), colors)
                                .child(Self::radio_indicator(comfortable_control, colors))
                                .child(Self::copy(
                                    "Comfortable",
                                    "Balanced spacing for everyday work.",
                                    colors.muted,
                                )),
                        )
                        .id("density-comfortable")
                        .on_click(comfortable),
                    spacious_control
                        .root_part(
                            Self::control_root(div(), colors)
                                .child(Self::radio_indicator(spacious_control, colors))
                                .child(Self::copy(
                                    "Spacious",
                                    "Larger targets and more breathing room.",
                                    colors.muted,
                                )),
                        )
                        .id("density-spacious")
                        .on_click(spacious),
                ]),
        );

        let sync_control = Switch::new(self.sync_settings);
        let managed_control = Switch::new(false);
        let switches = div()
            .flex_col()
            .gap_2()
            .child(
                sync_control
                    .root_part(
                        Self::control_root(div(), colors)
                            .child(Self::switch_track(sync_control, colors))
                            .child(Self::copy(
                                "Sync settings",
                                "Keep editor preferences consistent across Macs.",
                                colors.muted,
                            )),
                    )
                    .id("sync-settings")
                    .on_click(sync),
            )
            .child(
                managed_control
                    .root_part(
                        Self::control_root(div(), colors)
                            .child(Self::switch_track(managed_control, colors))
                            .child(Self::copy(
                                "Organization policy",
                                "Disabled controls retain semantics but expose no actions.",
                                colors.muted,
                            )),
                    )
                    .id("managed-setting")
                    .disabled(true),
            );

        div()
            .size_full()
            .overflow_y_scroll()
            .bg(colors.background)
            .text_color(colors.foreground)
            .child(
                div()
                    .w_full()
                    .max_w(660.0)
                    .mx_auto()
                    .flex_col()
                    .gap_4()
                    .p_5()
                    .child(
                        div()
                            .flex_row()
                            .items_center()
                            .justify_between()
                            .gap_3()
                            .child(
                                div()
                                    .min_w(0.0)
                                    .flex_col()
                                    .gap_1()
                                    .child(text("Selection controls").text_2xl().font_bold())
                                    .child(
                                        text("Framework-owned behavior, application-owned presentation")
                                            .text_sm()
                                            .text_color(colors.muted),
                                    ),
                            )
                            .child(
                                button()
                                    .id("theme")
                                    .on_click(theme)
                                    .cursor_default()
                                    .px_3()
                                    .py_2()
                                    .rounded_lg()
                                    .border(1.0, colors.border)
                                    .bg(colors.surface)
                                    .hover(|state| state.bg(colors.hover))
                                    .focus(|state| state.border(2.0, colors.focus_ring))
                                    .child(if self.light { "Use dark" } else { "Use light" }),
                            ),
                    )
                    .child(Self::section("Checkboxes", colors, checks))
                    .child(Self::section("Radio group", colors, densities))
                    .child(Self::section("Switches", colors, switches))
                    .child(
                        div()
                            .min_h(34.0)
                            .flex_row()
                            .items_center()
                            .px_3()
                            .rounded_lg()
                            .border(1.0, colors.border)
                            .bg(colors.surface)
                            .child(text(self.status.clone()).text_sm().text_color(colors.muted)),
                    ),
            )
    }
}
