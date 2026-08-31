use std::{sync::Arc, time::Duration};

use quickgui::{
    Application, Color, Element, Event, EventContext, Tab, Tabs, TabsState, Transition, View,
    ViewContext, div, text,
};

const COLOR_TRANSITION: Duration = Duration::from_millis(100);

fn main() -> Result<(), quickgui::AppError> {
    Application::new().run(|cx| {
        cx.open_window(
            quickgui::WindowOptions::new("QuickGUI — Tabs").size(840.0, 680.0),
            TabsDemo::default(),
        );
    })
}

#[derive(Clone, Copy)]
struct Palette {
    background: Color,
    surface: Color,
    raised: Color,
    border: Color,
    foreground: Color,
    muted: Color,
    hover: Color,
    pressed: Color,
    selected: Color,
    accent: Color,
    focus: Color,
}

impl Default for Palette {
    fn default() -> Self {
        Self {
            background: Color::rgb8(17, 18, 21),
            surface: Color::rgb8(26, 28, 33),
            raised: Color::rgb8(34, 36, 43),
            border: Color::rgb8(55, 59, 69),
            foreground: Color::rgb8(238, 240, 244),
            muted: Color::rgb8(155, 161, 174),
            hover: Color::rgba8(255, 255, 255, 10),
            pressed: Color::rgba8(255, 255, 255, 18),
            selected: Color::rgba8(10, 132, 255, 28),
            accent: Color::rgb8(10, 132, 255),
            focus: Color::rgb8(64, 156, 255),
        }
    }
}

struct TabsDemo {
    workspace: TabsState,
    preferences: TabsState,
    status: Arc<str>,
}

impl Default for TabsDemo {
    fn default() -> Self {
        Self {
            workspace: TabsState::new("overview"),
            preferences: TabsState::new("editor"),
            status: Arc::from(
                "Horizontal tabs use Left/Right then Enter or Space. Vertical tabs activate with Up/Down.",
            ),
        }
    }
}

impl TabsDemo {
    fn card(title: &'static str, description: &'static str, palette: Palette) -> Element {
        div()
            .w_full()
            .max_w(760.0)
            .flex_col()
            .gap_3()
            .p_4()
            .rounded_xl()
            .border(1.0, palette.border)
            .bg(palette.surface)
            .child(text(title).text_lg().font_semibold())
            .child(text(description).text_sm().text_color(palette.muted).wrap())
    }

    fn horizontal_tab(tab: Tab, label: &'static str, palette: Palette) -> Element {
        let active = tab.is_active();
        let root = div()
            .relative()
            .min_h(38.0)
            .flex_row()
            .items_center()
            .justify_center()
            .px_4()
            .rounded(7.0)
            .bg(if active {
                palette.selected
            } else {
                Color::TRANSPARENT
            })
            .text_color(if active {
                palette.foreground
            } else {
                palette.muted
            })
            .hover(|state| state.bg(palette.hover).text_color(palette.foreground))
            .active(|state| state.bg(palette.pressed))
            .focus(|state| state.border(2.0, palette.focus))
            .disabled_style(|state| state.opacity(0.38))
            .transition(Transition::colors(COLOR_TRANSITION))
            .child(text(label).text_sm().font_medium())
            .children(
                tab.indicator_part(
                    div()
                        .absolute()
                        .left(9.0)
                        .right(9.0)
                        .bottom(0.0)
                        .h(2.0)
                        .rounded(1.0)
                        .bg(palette.accent),
                ),
            );
        tab.tab_part(root)
    }

    fn vertical_tab(tab: Tab, label: &'static str, palette: Palette) -> Element {
        let active = tab.is_active();
        let root = div()
            .relative()
            .w_full()
            .min_h(40.0)
            .flex_row()
            .items_center()
            .px_3()
            .rounded_md()
            .bg(if active {
                palette.selected
            } else {
                Color::TRANSPARENT
            })
            .text_color(if active {
                palette.foreground
            } else {
                palette.muted
            })
            .hover(|state| state.bg(palette.hover).text_color(palette.foreground))
            .active(|state| state.bg(palette.pressed))
            .focus(|state| state.border(2.0, palette.focus))
            .disabled_style(|state| state.opacity(0.38))
            .transition(Transition::colors(COLOR_TRANSITION))
            .child(text(label).text_sm().font_medium())
            .children(
                tab.indicator_part(
                    div()
                        .absolute()
                        .left(0.0)
                        .top(7.0)
                        .bottom(7.0)
                        .w(2.0)
                        .rounded(1.0)
                        .bg(palette.accent),
                ),
            );
        tab.tab_part(root)
    }

    fn panel(
        eyebrow: &'static str,
        title: &'static str,
        body: &'static str,
        palette: Palette,
    ) -> Element {
        div()
            .min_w(0.0)
            .w_full()
            .flex_col()
            .gap_2()
            .p_4()
            .rounded_lg()
            .border(1.0, palette.border)
            .bg(palette.raised)
            .child(
                text(eyebrow)
                    .text_xs()
                    .font_semibold()
                    .text_color(palette.accent),
            )
            .child(text(title).text_lg().font_semibold())
            .child(text(body).text_sm().text_color(palette.muted).wrap())
    }
}

impl View for TabsDemo {
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
        let palette = Palette::default();

        let workspace = Tabs::from_state("workspace-tabs", &self.workspace).keep_mounted(true);
        let overview = workspace.tab("overview");
        let files = workspace.tab("files");
        let history = workspace.tab("history").disabled(true);
        let settings = workspace.tab("settings");

        let select_overview = cx.listener(overview.tab_id(), |view, cx| {
            if view.workspace.select("overview") {
                view.status = Arc::from("Overview selected");
                cx.invalidate();
            }
        });
        let select_files = cx.listener(files.tab_id(), |view, cx| {
            if view.workspace.select("files") {
                view.status = Arc::from("Files selected");
                cx.invalidate();
            }
        });
        let select_settings = cx.listener(settings.tab_id(), |view, cx| {
            if view.workspace.select("settings") {
                view.status = Arc::from("Workspace settings selected");
                cx.invalidate();
            }
        });

        let workspace_card = Self::card(
            "Manual horizontal tabs",
            "Arrow keys move focus without changing the panel. Enter or Space commits the focused tab; disabled tabs are skipped.",
            palette,
        )
        .child(workspace.root_part(
            div()
                .w_full()
                .flex_col()
                .gap_3()
                .child(workspace.list_part(
                    div()
                        .w_full()
                        .flex_row()
                        .gap_1()
                        .p_1()
                        .rounded_lg()
                        .bg(palette.background)
                        .accessibility_label("Workspace sections")
                        .child(
                            Self::horizontal_tab(overview, "Overview", palette)
                                .on_click(select_overview),
                        )
                        .child(
                            Self::horizontal_tab(files, "Files", palette).on_click(select_files),
                        )
                        .child(Self::horizontal_tab(history, "History", palette))
                        .child(
                            Self::horizontal_tab(settings, "Settings", palette)
                                .on_click(select_settings),
                        ),
                ))
                .children(overview.panel_part(Self::panel(
                    "OVERVIEW",
                    "Workspace activity",
                    "Three views changed today. Tabs retain no collection registry and schedule no work while this window is idle.",
                    palette,
                )))
                .children(files.panel_part(Self::panel(
                    "FILES",
                    "Visible project files",
                    "A real editor can mount a virtual tree here. The inactive panel remains mounted in this sample as display: none.",
                    palette,
                )))
                .children(history.panel_part(Self::panel(
                    "HISTORY",
                    "Unavailable history",
                    "This disabled tab cannot receive pointer or keyboard activation.",
                    palette,
                )))
                .children(settings.panel_part(Self::panel(
                    "SETTINGS",
                    "Workspace settings",
                    "All typography, spacing, color, borders, focus treatment, and indicator geometry in this example belong to the application.",
                    palette,
                ))),
        ));

        let preferences = Tabs::from_state("preference-tabs", &self.preferences)
            .vertical()
            .activate_on_focus(true);
        let editor = preferences.tab("editor");
        let appearance = preferences.tab("appearance");
        let terminal = preferences.tab("terminal");
        let remote = preferences.tab("remote").disabled(true);

        let select_editor = cx.listener(editor.tab_id(), |view, cx| {
            if view.preferences.select("editor") {
                view.status = Arc::from("Editor preference panel selected");
                cx.invalidate();
            }
        });
        let select_appearance = cx.listener(appearance.tab_id(), |view, cx| {
            if view.preferences.select("appearance") {
                view.status = Arc::from("Appearance preference panel selected");
                cx.invalidate();
            }
        });
        let select_terminal = cx.listener(terminal.tab_id(), |view, cx| {
            if view.preferences.select("terminal") {
                view.status = Arc::from("Terminal preference panel selected");
                cx.invalidate();
            }
        });

        let preferences_card = Self::card(
            "Automatic vertical tabs",
            "Up and Down move focus, activate immediately, wrap at the ends, and skip the disabled Remote item.",
            palette,
        )
        .child(preferences.root_part(
            div()
                .w_full()
                .min_h(190.0)
                .flex_row()
                .items_stretch()
                .gap_3()
                .child(preferences.list_part(
                    div()
                        .w(160.0)
                        .flex_none()
                        .flex_col()
                        .gap_1()
                        .accessibility_label("Preference sections")
                        .child(
                            Self::vertical_tab(editor, "Editor", palette).on_click(select_editor),
                        )
                        .child(
                            Self::vertical_tab(appearance, "Appearance", palette)
                                .on_click(select_appearance),
                        )
                        .child(
                            Self::vertical_tab(terminal, "Terminal", palette)
                                .on_click(select_terminal),
                        )
                        .child(Self::vertical_tab(remote, "Remote", palette)),
                ))
                .child(
                    div()
                        .min_w(0.0)
                        .flex_1()
                        .children(editor.panel_part(Self::panel(
                            "EDITOR",
                            "Editing preferences",
                            "Automatic activation is suitable when every panel is already available without noticeable latency.",
                            palette,
                        )))
                        .children(appearance.panel_part(Self::panel(
                            "APPEARANCE",
                            "Application appearance",
                            "The same unstyled parts can render native-looking preferences, document tabs, or product-specific navigation.",
                            palette,
                        )))
                        .children(terminal.panel_part(Self::panel(
                            "TERMINAL",
                            "Integrated terminal",
                            "Only the active panel is mounted in this set. Switching replaces one bounded subtree and then returns to sleep.",
                            palette,
                        )))
                        .children(remote.panel_part(Self::panel(
                            "REMOTE",
                            "Remote development",
                            "This panel remains absent because its tab is disabled.",
                            palette,
                        ))),
                ),
        ));

        div()
            .size_full()
            .overflow_y_scroll()
            .bg(palette.background)
            .text_color(palette.foreground)
            .child(
                div()
                    .w_full()
                    .flex_col()
                    .items_center()
                    .gap_4()
                    .p_5()
                    .child(
                        div()
                            .w_full()
                            .max_w(760.0)
                            .flex_row()
                            .items_center()
                            .justify_between()
                            .gap_4()
                            .child(
                                div()
                                    .min_w(0.0)
                                    .flex_col()
                                    .gap_1()
                                    .child(text("Unstyled tabs").text_2xl().font_bold())
                                    .child(
                                        text("The gallery supplies every visible pixel; QuickGUI owns behavior and semantics.")
                                            .text_sm()
                                            .text_color(palette.muted)
                                            .wrap(),
                                    ),
                            )
                            .child(
                                text("zero idle work")
                                    .flex_none()
                                    .text_xs()
                                    .font_semibold()
                                    .text_color(palette.accent),
                            ),
                    )
                    .child(workspace_card)
                    .child(preferences_card)
                    .child(
                        text(self.status.clone())
                            .w_full()
                            .max_w(760.0)
                            .text_sm()
                            .text_color(palette.muted)
                            .wrap(),
                    ),
            )
    }
}
