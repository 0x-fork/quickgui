use std::sync::Arc;

use quickgui::{
    App, AppConfig, ClipboardEntry, ClipboardError, ClipboardImage, ClipboardImageFormat,
    ClipboardItem, Color, Element, ExternalPaths, IntoElement, TitleBarStyle, View, ViewContext,
    button, div, text, text_input,
};

const ONE_PIXEL_PNG: &[u8] = &[
    137, 80, 78, 71, 13, 10, 26, 10, 0, 0, 0, 13, 73, 72, 68, 82, 0, 0, 0, 1, 0, 0, 0, 1, 8, 6, 0,
    0, 0, 31, 21, 196, 137, 0, 0, 0, 13, 73, 68, 65, 84, 8, 215, 99, 248, 207, 192, 240, 31, 0, 5,
    0, 1, 255, 137, 153, 61, 29, 0, 0, 0, 0, 73, 69, 78, 68, 174, 66, 96, 130,
];

fn main() -> Result<(), quickgui::AppError> {
    App::new(ClipboardDemo::default())
        .config(
            AppConfig::new("QuickGUI — Clipboard")
                .size(760.0, 580.0)
                .minimum_size(620.0, 480.0)
                .title_bar_style(TitleBarStyle::HiddenInset)
                .traffic_light_position(16.0, 14.0)
                .background(Color::rgb8(17, 20, 27)),
        )
        .run()
}

struct ClipboardDemo {
    draft: Arc<str>,
    status: Arc<str>,
    reads: usize,
}

impl Default for ClipboardDemo {
    fn default() -> Self {
        Self {
            draft: Arc::from("QuickGUI clipboard text"),
            status: Arc::from("No clipboard operation yet."),
            reads: 0,
        }
    }
}

impl ClipboardDemo {
    fn finish(&mut self, result: Result<impl Into<Arc<str>>, ClipboardError>) {
        self.status = match result {
            Ok(message) => message.into(),
            Err(error) => Arc::from(format!("Error: {error}")),
        };
    }

    fn control(label: impl Into<Arc<str>>) -> Element {
        button()
            .min_h(40.0)
            .px_4()
            .py_2()
            .rounded_lg()
            .border(1.0, Color::rgb8(66, 75, 92))
            .bg(Color::rgb8(38, 44, 56))
            .hover(|style| style.bg(Color::rgb8(50, 58, 73)))
            .focus(|style| style.border(2.0, Color::rgb8(125, 211, 252)))
            .child(text(label).font_semibold())
    }
}

impl View for ClipboardDemo {
    fn render(&mut self, cx: &mut ViewContext<'_, Self>) -> impl IntoElement {
        let edit = cx.input_listener("clipboard-text", |this, value, cx| {
            this.draft = Arc::from(value);
            cx.invalidate();
        });
        let copy_text = cx.listener("copy-text", |this, cx| {
            let metadata = serde_json::json!({
                "source": "quickgui-clipboard-example",
                "utf8_bytes": this.draft.len(),
            });
            let result =
                ClipboardItem::new_string_with_json_metadata(this.draft.clone(), &metadata)
                    .and_then(|item| cx.write_to_clipboard(item))
                    .map(|()| Arc::from("Copied text with hash-bound JSON metadata."));
            this.finish(result);
            cx.invalidate();
        });
        let copy_image = cx.listener("copy-image", |this, cx| {
            let result = ClipboardImage::new(ClipboardImageFormat::Png, ONE_PIXEL_PNG.to_vec())
                .and_then(ClipboardItem::new_image)
                .and_then(|item| cx.write_to_clipboard(item))
                .map(|()| Arc::from("Copied one encoded PNG without decoding it in QuickGUI."));
            this.finish(result);
            cx.invalidate();
        });
        let copy_path = cx.listener("copy-path", |this, cx| {
            // Launch Services gives app bundles `/` as their working directory. The compile-time
            // manifest directory keeps this demonstration path meaningful in both `cargo run`
            // and a native `.app` bundle.
            let path = env!("CARGO_MANIFEST_DIR");
            let result = ExternalPaths::new([path])
                .and_then(ClipboardItem::new_paths)
                .and_then(|item| cx.write_to_clipboard(item))
                .map(|()| Arc::from("Copied the project directory as a native file list."));
            this.finish(result);
            cx.invalidate();
        });
        let read = cx.listener("read", |this, cx| {
            let result = cx.read_from_clipboard().map(|item| {
                this.reads += 1;
                item.as_ref().map_or_else(
                    || Arc::from("General clipboard is empty or has no supported representation."),
                    describe_item,
                )
            });
            this.finish(result);
            cx.invalidate();
        });
        let write_find = cx.listener("write-find", |this, cx| {
            #[cfg(target_os = "macos")]
            let result = ClipboardItem::new_string(this.draft.clone())
                .and_then(|item| cx.write_to_find_pasteboard(item))
                .map(|()| Arc::from("Updated macOS's shared Find pasteboard."));
            #[cfg(not(target_os = "macos"))]
            let result: Result<Arc<str>, ClipboardError> = Ok(Arc::from(
                "The shared Find pasteboard is a macOS-specific service.",
            ));
            this.finish(result);
            cx.invalidate();
        });
        let read_find = cx.listener("read-find", |this, cx| {
            #[cfg(target_os = "macos")]
            let result = cx.read_from_find_pasteboard().map(|item| {
                item.as_ref().map_or_else(
                    || Arc::from("The shared Find pasteboard is empty."),
                    describe_item,
                )
            });
            #[cfg(not(target_os = "macos"))]
            let result: Result<Arc<str>, ClipboardError> = Ok(Arc::from(
                "The shared Find pasteboard is a macOS-specific service.",
            ));
            this.finish(result);
            cx.invalidate();
        });

        div()
            .size_full()
            .flex_col()
            .bg(Color::rgb8(17, 20, 27))
            .text_color(Color::rgb8(238, 241, 247))
            .child(
                div()
                    .h(62.0)
                    .w_full()
                    .flex_none()
                    .flex_row()
                    .items_center()
                    .justify_between()
                    .padding(0.0, 18.0, 0.0, 88.0)
                    .border(1.0, Color::rgb8(48, 55, 68))
                    .bg(Color::rgb8(24, 28, 36))
                    .app_region_drag()
                    .child(
                        div()
                            .flex_col()
                            .child(text("Bounded clipboard").text_lg().font_bold())
                            .child(
                                text("Text, metadata, encoded images, files, and Find pasteboard")
                                    .text_xs()
                                    .text_color(Color::rgb8(153, 165, 185)),
                            ),
                    )
                    .child(
                        text(format!("{} reads", self.reads))
                            .text_xs()
                            .text_color(Color::rgb8(153, 165, 185)),
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
                            .border(1.0, Color::rgb8(58, 66, 81))
                            .bg(Color::rgb8(28, 33, 42))
                            .shadow_lg()
                            .child(text("General clipboard").text_xl().font_bold())
                            .child(
                                text_input(self.draft.clone())
                                    .id("clipboard-text")
                                    .w_full()
                                    .min_h(42.0)
                                    .px_3()
                                    .rounded_lg()
                                    .border(1.0, Color::rgb8(72, 82, 100))
                                    .bg(Color::rgb8(20, 24, 31))
                                    .on_input(edit),
                            )
                            .child(
                                div()
                                    .flex_row()
                                    .flex_wrap()
                                    .gap_2()
                                    .child(Self::control("Copy text + metadata").on_click(copy_text))
                                    .child(Self::control("Copy encoded PNG").on_click(copy_image))
                                    .child(Self::control("Copy project path").on_click(copy_path))
                                    .child(Self::control("Read clipboard").on_click(read)),
                            ),
                    )
                    .child(
                        div()
                            .p_5()
                            .flex_col()
                            .gap_3()
                            .rounded_2xl()
                            .border(1.0, Color::rgb8(58, 66, 81))
                            .bg(Color::rgb8(28, 33, 42))
                            .child(text("Shared Find pasteboard").text_lg().font_bold())
                            .child(
                                text("On macOS this shares search text with native Find panels and other applications.")
                                    .wrap()
                                    .text_sm()
                                    .text_color(Color::rgb8(163, 174, 193)),
                            )
                            .child(
                                div()
                                    .flex_row()
                                    .flex_wrap()
                                    .gap_2()
                                    .child(Self::control("Write Find").on_click(write_find))
                                    .child(Self::control("Read Find").on_click(read_find)),
                            ),
                    )
                    .child(
                        div()
                            .p_4()
                            .rounded_xl()
                            .border(1.0, Color::rgb8(63, 75, 94))
                            .bg(Color::rgb8(22, 28, 38))
                            .child(
                                text(self.status.clone())
                                    .wrap()
                                    .text_sm()
                                    .text_color(Color::rgb8(186, 205, 232)),
                            ),
                    )
                    .child(
                        text("Idle invariant: pasteboards are opened only on demand. QuickGUI installs no clipboard observer, polling timer, or redraw loop.")
                            .max_w(680.0)
                            .wrap()
                            .text_xs()
                            .text_color(Color::rgb8(145, 157, 177)),
                    ),
            )
    }
}

fn describe_item(item: &ClipboardItem) -> Arc<str> {
    let mut kinds = Vec::new();
    for entry in item.entries() {
        kinds.push(match entry {
            ClipboardEntry::String(value) => format!(
                "text {} bytes{}",
                value.text().len(),
                if value.metadata().is_some() {
                    " + metadata"
                } else {
                    ""
                }
            ),
            ClipboardEntry::Image(value) => format!(
                "{} image {} bytes",
                value.format().extension(),
                value.bytes().len()
            ),
            ClipboardEntry::ExternalPaths(value) => {
                format!("{} native path(s)", value.paths().len())
            }
        });
    }
    Arc::from(format!("Read: {}.", kinds.join(", ")))
}
