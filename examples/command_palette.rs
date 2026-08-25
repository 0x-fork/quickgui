use std::sync::Arc;

use quickgui::{
    AnyAction, App, Color, Element, EventContext, FocusHandle, KeyBinding, PickerItem, PickerState,
    PickerStyle, TitleBarStyle, View, ViewContext, button, div, picker_key_bindings, text,
};

quickgui::actions!(
    palette_demo,
    [ToggleCommandPalette, SaveFile, ToggleSidebar, ToggleTheme,]
);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct RunCommand(&'static str);

fn command_items() -> Vec<PickerItem<AnyAction>> {
    vec![
        PickerItem::new("Save File", AnyAction::new(SaveFile))
            .detail("Write the active document to disk")
            .keywords("file write persist")
            .shortcut("⌘S"),
        PickerItem::new("Toggle Sidebar", AnyAction::new(ToggleSidebar))
            .detail("Show or hide project navigation")
            .keywords("view explorer panel")
            .shortcut("⌘B"),
        PickerItem::new("Toggle Theme", AnyAction::new(ToggleTheme))
            .detail("Switch between dark and light surfaces")
            .keywords("appearance color mode")
            .shortcut("⇧⌘T"),
        command(
            "Open Settings",
            "Preferences and framework options",
            "settings configuration",
        ),
        command("Go to File", "Open a file by name", "quick open navigation"),
        command(
            "Go to Symbol",
            "Jump to a symbol in the workspace",
            "outline definition",
        ),
        command(
            "Format Document",
            "Format the active buffer",
            "prettier rustfmt source",
        ),
        command(
            "Duplicate Line",
            "Duplicate the current editor line",
            "copy line",
        ),
        command(
            "Sort Lines Ascending",
            "Sort selected lines",
            "editor selection order",
        ),
        command(
            "Toggle Minimap",
            "Show or hide the editor minimap",
            "view map overview",
        ),
        command(
            "Split Editor Right",
            "Create an adjacent editor pane",
            "workspace pane",
        ),
        command(
            "Close Active Pane",
            "Close the focused editor pane",
            "workspace editor",
        ),
        command(
            "Reload Window",
            "Rebuild application state",
            "developer refresh",
        ),
        command(
            "Show Keyboard Shortcuts",
            "Inspect active contextual bindings",
            "keymap help",
        ),
        command(
            "Open Recent Project",
            "Choose a recently opened workspace",
            "history workspace",
        ),
        command(
            "New Terminal",
            "Create an embedded terminal panel",
            "shell console",
        ),
        command(
            "Toggle Zen Mode",
            "Hide surrounding workspace chrome",
            "focus distraction free",
        ),
        command(
            "Copy Relative Path",
            "Copy the active file path",
            "clipboard file",
        ),
        command(
            "Reveal in Finder",
            "Show the active file in Finder",
            "macos filesystem",
        ),
        PickerItem::new(
            "Install Framework Update",
            AnyAction::new(RunCommand("Install Framework Update")),
        )
        .detail("Unavailable in this local example")
        .keywords("upgrade release")
        .disabled(true),
    ]
}

fn command(
    label: &'static str,
    detail: &'static str,
    keywords: &'static str,
) -> PickerItem<AnyAction> {
    PickerItem::new(label, AnyAction::new(RunCommand(label)))
        .detail(detail)
        .keywords(keywords)
}

fn main() -> Result<(), quickgui::AppError> {
    let mut bindings = picker_key_bindings().to_vec();
    bindings.extend([
        KeyBinding::new("platform-shift-p", ToggleCommandPalette, Some("Workspace")),
        KeyBinding::new("platform-s", SaveFile, Some("Editor")),
        KeyBinding::new("platform-b", ToggleSidebar, Some("Workspace")),
        KeyBinding::new("platform-shift-t", ToggleTheme, Some("Workspace")),
    ]);

    App::new(CommandPaletteDemo::new())
        .title("QuickGUI — Command Palette")
        .size(900.0, 620.0)
        .title_bar_style(TitleBarStyle::HiddenInset)
        .traffic_light_position(16.0, 13.0)
        .bind_keys(bindings)
        .run()
}

struct CommandPaletteDemo {
    palette: PickerState<AnyAction>,
    palette_open: bool,
    sidebar_visible: bool,
    light_theme: bool,
    save_count: usize,
    status: Arc<str>,
}

impl CommandPaletteDemo {
    fn new() -> Self {
        let palette = PickerState::new(command_items())
            .expect("the static command registry is within picker limits")
            .with_style(
                PickerStyle::default()
                    .placeholder("Search commands by name or alias…")
                    .max_visible_rows(8),
            );
        Self {
            palette,
            palette_open: false,
            sidebar_visible: true,
            light_theme: false,
            save_count: 0,
            status: Arc::from("Press ⇧⌘P to open the command palette."),
        }
    }

    fn activate_palette_command(this: &mut Self, action: AnyAction, cx: &mut EventContext) {
        this.palette_open = false;
        cx.focus(FocusHandle::new("editor"));
        cx.dispatch_any_action(action);
        cx.invalidate();
    }

    fn toolbar_button(label: &'static str) -> Element {
        button()
            .h(32.0)
            .px_3()
            .items_center()
            .justify_center()
            .rounded_md()
            .border(1.0, Color::rgb8(79, 87, 103))
            .bg(Color::rgba8(255, 255, 255, 10))
            .hover(|style| style.bg(Color::rgba8(255, 255, 255, 24)))
            .active(|style| style.bg(Color::rgba8(59, 130, 246, 72)))
            .app_region_no_drag()
            .child(text(label).text_sm().font_medium())
    }
}

impl View for CommandPaletteDemo {
    fn render(&mut self, cx: &mut ViewContext<'_, Self>) -> impl quickgui::IntoElement {
        let workspace = cx.focus_handle("workspace");
        let editor = cx.focus_handle("editor");
        let palette_input = PickerState::<AnyAction>::input_focus_handle("command-palette");

        let toggle_palette = cx.action_listener(
            workspace.id(),
            move |this, _: &ToggleCommandPalette, event_cx| {
                this.palette_open = !this.palette_open;
                if this.palette_open {
                    this.palette.reset();
                    event_cx.focus(palette_input);
                } else {
                    event_cx.focus(editor);
                }
                event_cx.invalidate();
            },
        );
        let toggle_sidebar =
            cx.action_listener(workspace.id(), |this, _: &ToggleSidebar, event_cx| {
                this.sidebar_visible = !this.sidebar_visible;
                this.status = Arc::from(if this.sidebar_visible {
                    "Sidebar shown through the shared ToggleSidebar action."
                } else {
                    "Sidebar hidden through the shared ToggleSidebar action."
                });
                event_cx.invalidate();
            });
        let toggle_theme = cx.action_listener(workspace.id(), |this, _: &ToggleTheme, event_cx| {
            this.light_theme = !this.light_theme;
            this.status = Arc::from(if this.light_theme {
                "Light workspace surface enabled."
            } else {
                "Dark workspace surface enabled."
            });
            event_cx.invalidate();
        });
        let save = cx.action_listener(editor.id(), |this, _: &SaveFile, event_cx| {
            this.save_count += 1;
            this.status = Arc::from(format!(
                "SaveFile reached the restored editor focus path ({} invocation{}).",
                this.save_count,
                if this.save_count == 1 { "" } else { "s" }
            ));
            event_cx.invalidate();
        });
        let run = cx.action_listener(editor.id(), |this, command: &RunCommand, event_cx| {
            this.status = Arc::from(format!("Ran “{}” from the command palette.", command.0));
            event_cx.invalidate();
        });
        let open_button = cx.listener("open-command-palette", |_this, event_cx| {
            event_cx.dispatch_action(ToggleCommandPalette);
        });

        let background = if self.light_theme {
            Color::rgb8(226, 232, 240)
        } else {
            Color::rgb8(15, 18, 24)
        };
        let surface = if self.light_theme {
            Color::rgb8(248, 250, 252)
        } else {
            Color::rgb8(25, 29, 37)
        };
        let foreground = if self.light_theme {
            Color::rgb8(30, 41, 59)
        } else {
            Color::rgb8(226, 232, 240)
        };
        let muted = if self.light_theme {
            Color::rgb8(71, 85, 105)
        } else {
            Color::rgb8(148, 163, 184)
        };

        let mut root = div()
            .focus_scope(workspace)
            .key_context("Workspace")
            .on_action(toggle_palette)
            .on_action(toggle_sidebar)
            .on_action(toggle_theme)
            .size_full()
            .flex_col()
            .bg(background)
            .text_color(foreground)
            .child(
                div()
                    .h(54.0)
                    .flex_none()
                    .padding(0.0, 16.0, 0.0, 78.0)
                    .flex_row()
                    .items_center()
                    .justify_between()
                    .app_region_drag()
                    .border(1.0, Color::rgba8(100, 116, 139, 72))
                    .child(text("QuickGUI Picker").font_semibold())
                    .child(
                        Self::toolbar_button("Command Palette  ⇧⌘P").on_click(open_button),
                    ),
            )
            .child(
                div()
                    .flex_1()
                    .min_h(0.0)
                    .flex_row()
                    .child(
                        div()
                            .when(!self.sidebar_visible, |sidebar| sidebar.w(0.0))
                            .when(self.sidebar_visible, |sidebar| {
                                sidebar
                                    .w(210.0)
                                    .flex_none()
                                    .flex_col()
                                    .gap_2()
                                    .p_4()
                                    .border(1.0, Color::rgba8(100, 116, 139, 72))
                                    .bg(surface)
                                    .child(text("Workspace").font_semibold())
                                    .child(
                                        text("src\nexamples\nREADME.md\nARCHITECTURE.md")
                                            .text_sm()
                                            .line_height(26.0)
                                            .text_color(muted),
                                    )
                            }),
                    )
                    .child(
                        div()
                            .track_focus(editor)
                            .auto_focus()
                            .key_context("Editor")
                            .on_action(save)
                            .on_action(run)
                            .flex_1()
                            .min_w(0.0)
                            .p_5()
                            .flex_col()
                            .gap_4()
                            .focus(|style| {
                                style.border(2.0, Color::rgba8(45, 212, 191, 150))
                            })
                            .child(text("Production picker primitive").text_2xl().font_bold())
                            .child(
                                text("The palette filters a bounded registry only when the query changes, mounts visible rows through VirtualList, and dispatches the original typed action after restoring editor focus.")
                                    .text_base()
                                    .text_color(muted),
                            )
                            .child(
                                div()
                                    .flex_1()
                                    .min_h(0.0)
                                    .rounded_xl()
                                    .border(1.0, Color::rgba8(100, 116, 139, 72))
                                    .bg(surface)
                                    .p_5()
                                    .child(
                                        text("Try “thm”, “quick open”, or “configuration”. Use Up/Down, Page Up/Down, ⌘Up/⌘Down, and Return. Escape dismisses and restores focus.")
                                            .text_lg()
                                            .line_height(28.0),
                                    ),
                            )
                            .child(text(self.status.clone()).text_sm().text_color(Color::rgb8(45, 212, 191))),
                    ),
            );

        if self.palette_open {
            let dismiss = cx.dismiss_listener("command-palette", |this, event_cx| {
                this.palette_open = false;
                event_cx.invalidate();
            });
            let palette_width = self
                .palette
                .style()
                .width
                .min((cx.size().width - 24.0).max(1.0));
            let left = ((cx.size().width - palette_width) * 0.5).max(12.0);
            let palette = self
                .palette
                .element(
                    cx,
                    "command-palette",
                    |this| &mut this.palette,
                    Self::activate_palette_command,
                )
                .overlay()
                .top(72.0)
                .left(left)
                .max_w((cx.size().width - 24.0).max(1.0))
                .on_dismiss(dismiss)
                .restore_focus_to(editor);
            root = root.child(palette);
        }

        root
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn aliases_and_disabled_commands_are_part_of_the_bounded_registry() {
        let mut picker = PickerState::new(command_items()).unwrap();
        assert_eq!(picker.items().len(), 20);
        picker.set_query("configuration");
        assert_eq!(
            picker.selected_match().unwrap().item().label().as_ref(),
            "Open Settings"
        );
        picker.set_query("upgrade");
        assert_eq!(picker.result_count(), 1);
        assert!(picker.selected_match().is_none());
        assert!(picker.match_at(0).unwrap().item().is_disabled());
    }
}
