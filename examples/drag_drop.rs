use std::{path::PathBuf, sync::Arc, time::Duration};

use quickgui::{
    Animation, AnimationExt as _, AnimationPhase, App, Color, Drag, DragOrigin, DroppedFiles,
    DroppedText, DroppedUrl, Element, Event, EventContext, ExternalDragOperation,
    ExternalDragPayload, ExternalDragText, ExternalDragUrl, FileDragPaths, View, ViewContext, div,
    ease_out_quint, text,
};

fn main() -> Result<(), quickgui::AppError> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();
    App::new(DragDropDemo::default())
        .title("QuickGUI — Typed drag and drop")
        .size(820.0, 560.0)
        .run()
}

#[derive(Clone)]
struct CardPayload {
    label: Arc<str>,
    color: Color,
}

struct DragDropDemo {
    dropped_card: Option<CardPayload>,
    dropped_files: Vec<PathBuf>,
    dropped_text: Option<Arc<str>>,
    dropped_url: Option<Arc<str>>,
    status: Arc<str>,
    external_text: ExternalDragText,
    external_url: ExternalDragUrl,
}

impl Default for DragDropDemo {
    fn default() -> Self {
        Self {
            dropped_card: None,
            dropped_files: Vec::new(),
            dropped_text: None,
            dropped_url: None,
            status: Arc::from(""),
            external_text: ExternalDragText::new("QuickGUI native text drag"),
            external_url: ExternalDragUrl::new("https://github.com/egoist/quickgui")
                .expect("the example URL is absolute"),
        }
    }
}

impl DragDropDemo {
    fn card(
        label: &'static str,
        color: Color,
        listener: quickgui::DragListener<Self, CardPayload>,
    ) -> Element {
        div()
            .on_drag(listener)
            .w(190.0)
            .h(72.0)
            .flex_row()
            .items_center()
            .justify_center()
            .rounded_xl()
            .border(1.0, color)
            .bg(Color::rgb8(32, 36, 45))
            .hover(|style| style.bg(Color::rgb8(44, 49, 61)))
            .dragging(|style| {
                style
                    .bg(Color::rgb8(25, 28, 35))
                    .border(2.0, Color::rgb8(148, 163, 184))
            })
            .child(text(label).font_semibold().text_color(color))
    }

    fn preview(label: Arc<str>, color: Color) -> Element {
        div()
            .flex_row()
            .items_center()
            .justify_center()
            .rounded_xl()
            .border(2.0, color)
            .bg(Color::rgba8(27, 31, 39, 235))
            .shadow_lg()
            .child(text(label).font_semibold().text_color(color))
            .with_animation(
                "drag-preview-entrance",
                Animation::new(Duration::from_millis(120)).with_easing(ease_out_quint()),
                |element, value| {
                    let phase = AnimationPhase(value);
                    element
                        .w(phase.interpolate_clamped(166.0, 190.0))
                        .h(phase.interpolate_clamped(62.0, 72.0))
                        .rounded(phase.interpolate_clamped(6.0, 12.0))
                },
            )
    }
}

impl View for DragDropDemo {
    fn event(&mut self, event: &Event, cx: &mut EventContext) {
        match event {
            Event::FilesHovered(files) => {
                self.status = Arc::from(format!(
                    "{} Finder item{} hovering",
                    files.paths().len(),
                    if files.paths().len() == 1 { "" } else { "s" }
                ));
                cx.invalidate();
            }
            Event::FilesHoverCancelled => {
                self.status = Arc::from("Finder drag cancelled");
                cx.invalidate();
            }
            Event::ExternalDragEnded(event) => {
                let operation = match event.operation {
                    ExternalDragOperation::Cancelled => "cancelled",
                    ExternalDragOperation::Copied => "copied",
                    ExternalDragOperation::Moved => "moved",
                    ExternalDragOperation::Linked => "linked",
                    ExternalDragOperation::Deleted => "deleted",
                    ExternalDragOperation::Other => "finished",
                };
                self.status = Arc::from(format!("Native drag {operation}"));
                cx.invalidate();
            }
            _ => {}
        }
    }

    fn render(&mut self, cx: &mut ViewContext<'_, Self>) -> impl quickgui::IntoElement {
        let colors = [
            Color::rgb8(96, 165, 250),
            Color::rgb8(167, 139, 250),
            Color::rgb8(52, 211, 153),
            Color::rgb8(251, 191, 36),
        ];
        let manifest =
            FileDragPaths::files([PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml")]);
        let source_specs = [
            (
                "Cargo.toml → Finder",
                Some(ExternalDragPayload::Files(manifest)),
            ),
            (
                "Text → native app",
                Some(ExternalDragPayload::Text(self.external_text.clone())),
            ),
            (
                "URL → browser",
                Some(ExternalDragPayload::Url(self.external_url.clone())),
            ),
            ("Internal only", None),
        ];
        let mut sources = div().flex_row().gap_3().flex_wrap();
        for (index, ((label, external_payload), color)) in
            source_specs.into_iter().zip(colors).enumerate()
        {
            let payload = CardPayload {
                label: Arc::from(label),
                color,
            };
            let preview_payload = payload.clone();
            let drag = cx.drag_listener(0xd000_u64 + index as u64, move |_this, _event, _cx| {
                let drag = Drag::new(preview_payload.clone()).preview(Self::preview(
                    preview_payload.label.clone(),
                    preview_payload.color,
                ));
                if let Some(payload) = &external_payload {
                    drag.external_payload(payload.clone())
                } else {
                    drag
                }
            });
            sources = sources.child(Self::card(label, color, drag));
        }

        let card_drop = cx.drop_listener("drop-zone", |this, payload: &CardPayload, event, cx| {
            this.dropped_card = Some(payload.clone());
            this.status = Arc::from(format!(
                "Dropped '{}' from {:?}",
                payload.label, event.origin
            ));
            cx.invalidate();
        });
        let files_drop = cx.drop_listener("drop-zone", |this, files: &DroppedFiles, event, cx| {
            this.dropped_files = files.paths().to_vec();
            this.status = Arc::from(format!(
                "Dropped {} Finder item{}{}",
                files.paths().len(),
                if files.paths().len() == 1 { "" } else { "s" },
                if files.is_truncated() {
                    " (list truncated)"
                } else {
                    ""
                }
            ));
            debug_assert_eq!(event.origin, DragOrigin::External);
            cx.invalidate();
        });
        let text_drop = cx.drop_listener("drop-zone", |this, value: &DroppedText, event, cx| {
            this.dropped_text = Some(Arc::from(value.as_str()));
            this.status = Arc::from(format!(
                "Dropped native text{} at {:.0}, {:.0}",
                if value.is_truncated() {
                    " (truncated)"
                } else {
                    ""
                },
                event.position.x,
                event.position.y
            ));
            debug_assert_eq!(event.origin, DragOrigin::External);
            cx.invalidate();
        });
        let url_drop = cx.drop_listener("drop-zone", |this, value: &DroppedUrl, event, cx| {
            this.dropped_url = Some(Arc::from(value.as_str()));
            this.status = Arc::from(format!(
                "Dropped native URL at {:.0}, {:.0}",
                event.position.x, event.position.y
            ));
            debug_assert_eq!(event.origin, DragOrigin::External);
            cx.invalidate();
        });

        let card_result = self
            .dropped_card
            .as_ref()
            .map(|card| format!("Internal payload: {}", card.label))
            .unwrap_or_else(|| "Internal payload: none".to_owned());
        let file_result = if self.dropped_files.is_empty() {
            "Finder files: none".to_owned()
        } else {
            format!(
                "Finder files: {}",
                self.dropped_files
                    .iter()
                    .take(3)
                    .map(|path| path.display().to_string())
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        };
        let current_card = self.dropped_card.as_ref().map(|card| card.label.clone());
        let text_result = self
            .dropped_text
            .as_deref()
            .map(|value| format!("Native text: {value}"))
            .unwrap_or_else(|| "Native text: none".to_owned());
        let url_result = self
            .dropped_url
            .as_deref()
            .map(|value| format!("Native URL: {value}"))
            .unwrap_or_else(|| "Native URL: none".to_owned());

        div()
            .size_full()
            .flex_col()
            .gap_5()
            .p_6()
            .bg(Color::rgb8(18, 19, 23))
            .text_color(Color::rgb8(232, 235, 241))
            .child(
                div()
                    .flex_col()
                    .gap_1()
                    .child(text("Typed drag and drop").text_2xl().font_bold())
                    .child(
                        text("Drag a card internally; take bounded files, text, and URLs out to native macOS apps; or drop Finder files, native text, and URLs back here. The preview and drop highlight are paint-only, with no idle frame loop.")
                            .text_sm()
                            .text_color(Color::rgb8(157, 164, 178)),
                    ),
            )
            .child(sources)
            .child(
                div()
                    .on_drop(card_drop)
                    .on_drop(files_drop)
                    .on_drop(text_drop)
                    .on_drop(url_drop)
                    .can_drop::<CardPayload>(move |payload| {
                        current_card
                            .as_ref()
                            .is_none_or(|current| current != &payload.label)
                    })
                    .min_h(190.0)
                    .w_full()
                    .flex_col()
                    .items_center()
                    .justify_center()
                    .gap_2()
                    .rounded_2xl()
                    .border(2.0, Color::rgb8(71, 79, 96))
                    .bg(Color::rgb8(24, 27, 34))
                    .drag_over(|style| {
                        style
                            .bg(Color::rgb8(29, 57, 61))
                            .border(3.0, Color::rgb8(94, 234, 212))
                    })
                    .child(text("Drop here").text_xl().font_bold())
                    .child(text(card_result).text_sm())
                    .child(
                        text(file_result)
                            .text_sm()
                            .text_color(Color::rgb8(157, 164, 178)),
                    )
                    .child(
                        text(text_result)
                            .text_sm()
                            .text_color(Color::rgb8(157, 164, 178)),
                    )
                    .child(
                        text(url_result)
                            .text_sm()
                            .text_color(Color::rgb8(157, 164, 178)),
                    ),
            )
            .child(
                text(if self.status.is_empty() {
                    Arc::from("Escape cancels an internal drag")
                } else {
                    self.status.clone()
                })
                .text_sm()
                .text_color(Color::rgb8(148, 163, 184)),
            )
    }
}
