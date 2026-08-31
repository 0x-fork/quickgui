use std::{path::PathBuf, sync::Arc};

use quickgui::{
    Application, Color, Element, IntoElement, View, ViewContext, WindowOptions, button, div, text,
};

const WORKSPACE_TABS: &str = "dev.quickgui.example.document-window";

fn main() -> Result<(), quickgui::AppError> {
    let path = manifest_path();
    Application::new().run(move |cx| {
        cx.open_window(
            WindowOptions::new("QuickGUI — Document window")
                .size(760.0, 620.0)
                .minimum_size(620.0, 480.0)
                .represented_file(path.clone())
                .tabbing_identifier(WORKSPACE_TABS),
            DocumentWindow::new(path, 1),
        );
    })
}

struct DocumentWindow {
    path: PathBuf,
    number: usize,
    opened: usize,
    edited: bool,
    represented: bool,
    status: String,
}

impl DocumentWindow {
    fn new(path: PathBuf, number: usize) -> Self {
        Self {
            path,
            number,
            opened: number,
            edited: false,
            represented: true,
            status: "Native document chrome is ready".to_owned(),
        }
    }

    fn control(label: impl Into<Arc<str>>) -> Element {
        button()
            .px_3()
            .py_2()
            .rounded_md()
            .bg(Color::rgb8(43, 49, 63))
            .hover(|style| style.bg(Color::rgb8(56, 64, 82)))
            .text_color(Color::rgb8(235, 239, 247))
            .child(text(label))
    }
}

impl View for DocumentWindow {
    fn render(&mut self, cx: &mut ViewContext<'_, Self>) -> impl IntoElement {
        let state = cx.window_state();
        let toggle_edited = cx.listener("toggle-edited", |this, cx| {
            this.edited = !this.edited;
            match cx.set_window_edited(this.edited) {
                Ok(()) => {
                    this.status = if this.edited {
                        "Marked as edited; AppKit shows the unsaved indicator"
                    } else {
                        "Marked as saved"
                    }
                    .to_owned();
                }
                Err(error) => this.status = error.to_string(),
            }
            cx.invalidate();
        });
        let toggle_file = cx.listener("toggle-file", |this, cx| {
            this.represented = !this.represented;
            let result = if this.represented {
                cx.set_document_path(&this.path)
            } else {
                cx.clear_represented_file()
            };
            this.status = result.map_or_else(
                |error| error.to_string(),
                |()| {
                    if this.represented {
                        "Restored the represented file URL"
                    } else {
                        "Cleared the represented file URL"
                    }
                    .to_owned()
                },
            );
            cx.invalidate();
        });
        let open_tab = cx.listener("open-tab", |this, cx| {
            this.opened = this.opened.saturating_add(1);
            let number = this.opened;
            let path = this.path.clone();
            cx.open_window(
                WindowOptions::new(format!("QuickGUI document #{number}"))
                    .size(760.0, 620.0)
                    .minimum_size(620.0, 480.0)
                    .represented_file(path.clone())
                    .tabbing_identifier(WORKSPACE_TABS),
                DocumentWindow::new(path, number),
            );
            this.status = format!("Opened document window #{number} in the native tab group");
            cx.invalidate();
        });
        let next = cx.listener("next-tab", |this, cx| {
            this.status = command_status(cx.select_next_tab(), "Selected the next native tab");
            cx.invalidate();
        });
        let previous = cx.listener("previous-tab", |this, cx| {
            this.status =
                command_status(cx.select_previous_tab(), "Selected the previous native tab");
            cx.invalidate();
        });
        let merge = cx.listener("merge-tabs", |this, cx| {
            this.status = command_status(cx.merge_all_windows(), "Requested Merge All Windows");
            cx.invalidate();
        });
        let detach = cx.listener("detach-tab", |this, cx| {
            this.status = command_status(
                cx.move_tab_to_new_window(),
                "Moved this tab to a separate native window",
            );
            cx.invalidate();
        });
        let toggle_bar = cx.listener("toggle-tab-bar", |this, cx| {
            this.status = command_status(cx.toggle_tab_bar(), "Toggled the native tab bar");
            cx.invalidate();
        });
        let overview = cx.listener("toggle-overview", |this, cx| {
            this.status =
                command_status(cx.toggle_tab_overview(), "Toggled the native tab overview");
            cx.invalidate();
        });
        let characters = cx.listener("characters", |this, cx| {
            this.status = command_status(
                cx.show_character_palette(),
                "Presented the character palette",
            );
            cx.invalidate();
        });

        div()
            .size_full()
            .flex_col()
            .gap_4()
            .p_6()
            .bg(Color::rgb8(17, 19, 24))
            .text_color(Color::rgb8(232, 236, 244))
            .child(text(format!("Document window #{}", self.number)).text_2xl().font_bold())
            .child(
                text("AppKit represented URLs, edited state, character palette, and system tabs")
                    .wrap()
                    .text_color(Color::rgb8(151, 162, 181)),
            )
            .child(
                div()
                    .flex_row()
                    .flex_wrap()
                    .gap_2()
                    .child(Self::control(if self.edited { "Mark saved" } else { "Mark edited" }).on_click(toggle_edited))
                    .child(Self::control(if self.represented { "Clear file" } else { "Represent file" }).on_click(toggle_file))
                    .child(Self::control("Character palette").on_click(characters)),
            )
            .child(
                div()
                    .flex_row()
                    .flex_wrap()
                    .gap_2()
                    .child(Self::control("New document window").on_click(open_tab))
                    .child(Self::control("Previous tab").on_click(previous))
                    .child(Self::control("Next tab").on_click(next))
                    .child(Self::control("Merge all").on_click(merge))
                    .child(Self::control("Detach tab").on_click(detach))
                    .child(Self::control("Toggle tab bar").on_click(toggle_bar))
                    .child(Self::control("Tab overview").on_click(overview)),
            )
            .child(
                div()
                    .flex_col()
                    .gap_2()
                    .p_4()
                    .rounded_lg()
                    .bg(Color::rgb8(27, 31, 40))
                    .child(text(format!("represented file: {}", state.represented_file)))
                    .child(text(format!("document edited: {}", state.document_edited)))
                    .child(text(format!("native tabbing: {}", state.native_tabbing)))
                    .child(text(format!(
                        "tabs: {}, selected: {:?}, bar: {}, overview: {}, truncated: {}",
                        state.native_tabs.count,
                        state.native_tabs.selected_index,
                        state.native_tabs.tab_bar_visible,
                        state.native_tabs.overview_visible,
                        state.native_tabs.truncated,
                    )).wrap())
                    .child(text(self.path.display().to_string()).wrap().text_sm().text_color(Color::rgb8(147, 197, 253))),
            )
            .child(text(self.status.clone()).wrap().text_color(Color::rgb8(125, 211, 252)))
            .child(
                text("Tab snapshots update only at explicit commands and native lifecycle events; this example adds no idle redraw source.")
                    .wrap()
                    .text_sm()
                    .text_color(Color::rgb8(132, 143, 162)),
            )
    }
}

fn manifest_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml")
}

fn command_status(result: Result<(), quickgui::WindowCommandError>, success: &str) -> String {
    result.map_or_else(|error| error.to_string(), |()| success.to_owned())
}
