use std::sync::Arc;

use quickgui::{
    Application, Color, Element, Event, EventContext, KeyBinding, Menu, MenuItem, View,
    ViewContext, button, div, text,
};

quickgui::actions!(
    commands,
    [Save, ToggleSidebar, SplitLeft, NavigateBack, InspectPath]
);

fn app_menus(sidebar_visible: bool) -> Vec<Menu> {
    vec![
        Menu::new("File").items([
            MenuItem::action("Save", Save),
            MenuItem::separator(),
            MenuItem::submenu(Menu::new("Layout").items([
                MenuItem::action("Split Left", SplitLeft),
                MenuItem::action("Navigate Back", NavigateBack),
            ])),
        ]),
        Menu::new("View").items([
            MenuItem::action("Show Sidebar", ToggleSidebar).checked(sidebar_visible),
            MenuItem::action("Inspect Action Path", InspectPath),
        ]),
    ]
}

fn main() -> Result<(), quickgui::AppError> {
    Application::new()
        .bind_keys([
            KeyBinding::new("platform-s", Save, Some("Editor")),
            KeyBinding::new("platform-b", ToggleSidebar, Some("Workspace")),
            KeyBinding::new("platform-k left", SplitLeft, Some("Workspace > Editor")),
            KeyBinding::new("platform-[", NavigateBack, Some("Editor")).use_key_equivalents(),
            KeyBinding::new("platform-i", InspectPath, Some("Editor")),
        ])
        .menus(app_menus(true))
        .run(|cx| {
            cx.open_window(
                quickgui::WindowOptions::new("QuickGUI — Actions and contextual keymaps")
                    .size(760.0, 480.0),
                ActionDemo::new(),
            );
        })
}

struct ActionDemo {
    sidebar_visible: bool,
    save_count: usize,
    raw_key_count: usize,
    action_capture_count: usize,
    focused_key_count: usize,
    status: Arc<str>,
}

impl ActionDemo {
    fn new() -> Self {
        Self {
            sidebar_visible: true,
            save_count: 0,
            raw_key_count: 0,
            action_capture_count: 0,
            focused_key_count: 0,
            status: Arc::from(
                "Try Cmd-S, Cmd-B, Cmd-I, Cmd-K then Left, or the localized Cmd-[ binding.",
            ),
        }
    }

    fn control(label: &'static str) -> Element {
        button()
            .h(36.0)
            .px_3()
            .flex_row()
            .items_center()
            .justify_center()
            .rounded_md()
            .border(1.0, Color::rgb8(75, 80, 92))
            .bg(Color::rgb8(40, 43, 51))
            .hover(|style| style.bg(Color::rgb8(51, 56, 67)))
            .active(|style| style.bg(Color::rgb8(37, 82, 120)))
            .child(text(label).font_medium())
    }
}

impl View for ActionDemo {
    fn event(&mut self, event: &Event, cx: &mut EventContext) {
        if let Event::KeyDown {
            key,
            key_char,
            modifiers,
            ..
        } = event
        {
            if *key == quickgui::Key::Escape {
                cx.exit();
            } else {
                self.raw_key_count += 1;
                self.status = Arc::from(format!(
                    "Raw key event #{}: {modifiers:?} command={key:?} printable={key_char:?}",
                    self.raw_key_count,
                ));
                cx.invalidate();
            }
        }
    }

    fn render(&mut self, cx: &mut ViewContext<'_, Self>) -> impl quickgui::IntoElement {
        let workspace = cx.focus_handle("workspace");
        let editor = cx.focus_handle("editor");
        let keyboard_layout = cx.keyboard_layout().name().to_owned();

        let toggle_sidebar = cx.action_listener("workspace", |this, _: &ToggleSidebar, cx| {
            this.sidebar_visible = !this.sidebar_visible;
            this.status = Arc::from(if this.sidebar_visible {
                "Workspace handled ToggleSidebar: sidebar shown."
            } else {
                "Workspace handled ToggleSidebar: sidebar hidden."
            });
            cx.set_menus(app_menus(this.sidebar_visible));
            cx.invalidate();
        });
        let capture_save = cx.action_listener("workspace", |this, _: &Save, _cx| {
            this.action_capture_count += 1;
        });
        let workspace_save = cx.action_listener("workspace", |this, _: &Save, cx| {
            this.status = Arc::from("Workspace Save fallback ran unexpectedly.");
            cx.invalidate();
        });
        let workspace_inspect = cx.action_listener("workspace", |this, _: &InspectPath, cx| {
            this.status = Arc::from("InspectPath propagated Editor → Workspace.");
            cx.invalidate();
        });
        let save = cx.action_listener("editor", |this, _: &Save, cx| {
            this.save_count += 1;
            this.status = Arc::from(format!(
                "Editor handled Save ({} time{}).",
                this.save_count,
                if this.save_count == 1 { "" } else { "s" }
            ));
            cx.invalidate();
        });
        let split_left = cx.action_listener("editor", |this, _: &SplitLeft, cx| {
            this.status = Arc::from("Editor handled the multi-stroke SplitLeft action.");
            cx.invalidate();
        });
        let navigate_back = cx.action_listener("editor", |this, _: &NavigateBack, cx| {
            this.status = Arc::from(
                "Editor handled NavigateBack through the layout-localized Cmd-[ binding.",
            );
            cx.invalidate();
        });
        let editor_inspect = cx.action_listener("editor", |this, _: &InspectPath, cx| {
            this.status = Arc::from("Editor observed InspectPath and propagated it.");
            cx.invalidate();
            cx.propagate();
        });
        let capture_key = cx.key_down_listener("workspace", |this, _event, _cx| {
            this.focused_key_count += 1;
        });
        let editor_key = cx.key_down_listener("editor", |this, _event, _cx| {
            this.focused_key_count += 1;
        });
        let save_button = cx.listener("save-button", |_this, cx| {
            cx.dispatch_action(Save);
        });

        let editor_contains_focus = cx.contains_focused(editor);
        div()
            .focus_scope(workspace)
            .key_context("Workspace")
            .capture_action(capture_save)
            .on_action(toggle_sidebar)
            .on_action(workspace_save)
            .on_action(workspace_inspect)
            .capture_key_down(capture_key)
            .size_full()
            .flex_col()
            .bg(Color::rgb8(18, 19, 22))
            .text_color(Color::rgb8(232, 234, 239))
            .child(
                div()
                    .h(54.0)
                    .px_4()
                    .flex_row()
                    .items_center()
                    .justify_between()
                    .border(1.0, Color::rgb8(48, 51, 60))
                    .child(
                        text("Actions, contextual keymaps, and native menus")
                            .text_lg()
                            .font_semibold(),
                    )
                    .child(
                        text(if editor_contains_focus {
                            format!("Editor focused · {keyboard_layout}")
                        } else {
                            format!("Editor unfocused · {keyboard_layout}")
                        })
                        .text_xs()
                        .text_color(Color::rgb8(126, 231, 212)),
                    ),
            )
            .child(
                div()
                    .flex_1()
                    .min_h(0.0)
                    .flex_row()
                    .child(
                        div()
                            .when(self.sidebar_visible, |sidebar| {
                                sidebar
                                    .w(190.0)
                                    .p_4()
                                    .border(1.0, Color::rgb8(48, 51, 60))
                                    .bg(Color::rgb8(24, 26, 31))
                                    .child(text("Workspace sidebar").font_medium())
                            }),
                    )
                    .child(
                        div()
                            .track_focus(editor)
                            .auto_focus()
                            .key_context("Editor mode=insert")
                            .on_action(save)
                            .on_action(split_left)
                            .on_action(navigate_back)
                            .on_action(editor_inspect)
                            .on_key_down(editor_key)
                            .flex_1()
                            .min_w(0.0)
                            .p_4()
                            .flex_col()
                            .gap_4()
                            .focus(|style| style.border(1.0, Color::rgb8(70, 116, 145)))
                            .child(
                                div()
                                    .flex_row()
                                    .items_center()
                                    .gap_3()
                                    .child(Self::control("Save action").on_click(save_button))
                                    .child(
                                        text("The button, File menu, and Cmd-S dispatch the same typed Save action.")
                                            .text_sm()
                                            .text_color(Color::rgb8(155, 160, 171)),
                                    ),
                            )
                            .child(
                                div()
                                    .flex_1()
                                    .p_4()
                                    .rounded_lg()
                                    .bg(Color::rgb8(27, 29, 35))
                                    .child(
                                        text("Focused editor surface\n\nActions capture from the workspace toward focus, then bubble back to the workspace. Raw key listeners use the same two phases only after keymap actions propagate. Context predicates choose the deepest applicable binding. Cmd-[ opts into Apple's localized equivalent (for example Cmd-Ö on German). Incomplete key sequences sleep until the next stroke or one timeout wake-up.")
                                            .text_base()
                                            .line_height(24.0)
                                            .wrap(),
                                    ),
                            )
                            .child(
                                text(self.status.clone())
                                    .text_sm()
                                    .text_color(Color::rgb8(126, 231, 212)),
                            ),
                    ),
            )
            .child(
                text(format!(
                    "Escape quits. Save captures: {} · focused key phase callbacks: {} · Cmd-K prefixes replay on mismatch or timeout.",
                    self.action_capture_count, self.focused_key_count
                ))
                    .px_4()
                    .py_2()
                    .text_xs()
                    .text_color(Color::rgb8(132, 137, 148)),
            )
    }
}
