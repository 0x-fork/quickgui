use std::{
    collections::HashMap,
    sync::Arc,
    time::{Duration, Instant},
};

use quickgui::{
    App, ClickListener, Color, Element, ElementId, Event, EventContext, Key, View, ViewContext,
    VirtualList, div, text,
};

const ROWS: usize = 100_000;
const ROW_HEIGHT: f32 = 32.0;
const HEADER_HEIGHT: f32 = 72.0;
const FOOTER_HEIGHT: f32 = 34.0;
const ROW_ID_BASE: u64 = 0x1000_0000_0000_0000;
const LABEL_CACHE_LIMIT: usize = 512;

fn main() -> Result<(), quickgui::AppError> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .with_target(false)
        .compact()
        .init();

    App::new(ScrollDemo::new())
        .title("QuickGUI — 100,000 row stress test")
        .size(1_000.0, 720.0)
        .run()
}

struct ScrollDemo {
    list: VirtualList,
    selected: Option<usize>,
    labels: HashMap<usize, Arc<str>>,
    title: Arc<str>,
    subtitle: Arc<str>,
    help: Arc<str>,
    metrics_text: Arc<str>,
    last_metrics_update: Instant,
}

impl ScrollDemo {
    fn new() -> Self {
        Self {
            list: VirtualList::new(ROWS, ROW_HEIGHT).with_overscan(2),
            selected: None,
            labels: HashMap::with_capacity(LABEL_CACHE_LIMIT),
            title: Arc::from("QuickGUI virtual list"),
            subtitle: Arc::from(
                "100,000 rows · only visible rows are shaped, laid out, and painted",
            ),
            help: Arc::from("Scroll or use ↑ ↓ Page Up Page Down Home End · Esc quits"),
            metrics_text: Arc::from("Waiting for the first frame…"),
            last_metrics_update: Instant::now(),
        }
    }

    fn select_and_reveal(&mut self, index: usize) {
        let index = index.min(self.list.len().saturating_sub(1));
        self.selected = Some(index);
        let top = index as f32 * ROW_HEIGHT;
        let bottom = top + ROW_HEIGHT;
        if top < self.list.scroll_offset() {
            self.list.scroll_to(top);
        } else if bottom > self.list.scroll_offset() + self.list.viewport_height() {
            self.list.scroll_to(bottom - self.list.viewport_height());
        }
    }

    fn label(&mut self, index: usize) -> Arc<str> {
        self.labels
            .entry(index)
            .or_insert_with(|| {
                Arc::from(format!(
                    "Row {index:06}    The quick brown fox jumps over the lazy GPU"
                ))
            })
            .clone()
    }

    fn row(&mut self, index: usize, listener: ClickListener<Self>) -> Element {
        let top = index as f32 * ROW_HEIGHT - self.list.scroll_offset();
        let selected = self.selected == Some(index);
        div()
            .on_click(listener)
            .absolute()
            .top(top)
            .left(0.0)
            .w_full()
            .h_8()
            .flex_row()
            .flex_none()
            .items_center()
            .px_4()
            .gap_2()
            .bg(if selected {
                Color::rgb8(39, 76, 119)
            } else if index.is_multiple_of(2) {
                Color::rgb8(20, 21, 24)
            } else {
                Color::rgb8(23, 24, 27)
            })
            .hover(|style| {
                style.bg(if selected {
                    Color::rgb8(45, 88, 137)
                } else {
                    Color::rgb8(35, 39, 47)
                })
            })
            .active(|style| {
                style.bg(if selected {
                    Color::rgb8(50, 96, 149)
                } else {
                    Color::rgb8(39, 68, 104)
                })
            })
            .child(
                div()
                    .size(4.0, 4.0)
                    .flex_none()
                    .rounded(2.0)
                    .bg(if index.is_multiple_of(10) {
                        Color::rgb8(94, 234, 212)
                    } else {
                        Color::rgb8(82, 86, 97)
                    }),
            )
            .child(
                text(self.label(index))
                    .text_sm()
                    .no_wrap()
                    .text_color(Color::rgb8(218, 221, 228)),
            )
    }

    fn prune_labels(&mut self, visible_start: usize, visible_end: usize) {
        if self.labels.len() <= LABEL_CACHE_LIMIT {
            return;
        }
        let margin = LABEL_CACHE_LIMIT / 4;
        let keep_start = visible_start.saturating_sub(margin);
        let keep_end = visible_end.saturating_add(margin).min(self.list.len());
        self.labels
            .retain(|index, _| *index >= keep_start && *index < keep_end);
    }
}

impl View for ScrollDemo {
    fn event(&mut self, event: &Event, cx: &mut EventContext) {
        match event {
            Event::Scroll(delta) => {
                // Platform deltas describe content motion; list offsets move oppositely.
                if self.list.scroll_by(-delta.y) {
                    cx.invalidate();
                }
            }
            Event::KeyDown { key, .. } => {
                let selected = self
                    .selected
                    .unwrap_or_else(|| (self.list.scroll_offset() / ROW_HEIGHT).round() as usize);
                let page = (self.list.viewport_height() / ROW_HEIGHT).floor().max(1.0) as usize;
                let target = match key {
                    Key::ArrowUp => Some(selected.saturating_sub(1)),
                    Key::ArrowDown => Some(selected.saturating_add(1)),
                    Key::PageUp => Some(selected.saturating_sub(page)),
                    Key::PageDown => Some(selected.saturating_add(page)),
                    Key::Home => Some(0),
                    Key::End => Some(self.list.len().saturating_sub(1)),
                    Key::Escape => {
                        cx.exit();
                        None
                    }
                    _ => None,
                };
                if let Some(index) = target {
                    self.select_and_reveal(index);
                    cx.invalidate();
                }
            }
            _ => {}
        }
    }

    fn render(&mut self, cx: &mut ViewContext<'_, Self>) -> impl quickgui::IntoElement {
        let list_height = (cx.size().height - HEADER_HEIGHT - FOOTER_HEIGHT).max(0.0);
        self.list.set_viewport_height(list_height);
        let visible = self.list.visible_rows();
        let rows: Vec<_> = visible
            .range
            .clone()
            .map(|index| {
                let listener = cx.listener(
                    ElementId::new(ROW_ID_BASE | index as u64),
                    move |this, event_cx| {
                        this.selected = Some(index);
                        event_cx.invalidate();
                    },
                );
                self.row(index, listener)
            })
            .collect();

        if self.last_metrics_update.elapsed() >= Duration::from_millis(250) {
            let metrics = cx.metrics();
            self.metrics_text = Arc::from(format!(
                "CPU {:.2} ms  ·  {} quads  ·  {} text  ·  {} cached",
                metrics.smoothed_cpu_milliseconds(),
                metrics.render.quads,
                metrics.render.text_areas,
                metrics.render.cached_text_areas,
            ));
            self.last_metrics_update = Instant::now();
        }

        let scrollbar = self.list.scrollbar_thumb(28.0).map(|(offset, height)| {
            div()
                .absolute()
                .top(offset + 2.0)
                .right(3.0)
                .size(5.0, (height - 4.0).max(4.0))
                .rounded(2.5)
                .bg(Color::rgba8(133, 139, 153, 150))
        });

        let tree = div()
            .size_full()
            .flex_col()
            .bg(Color::rgb8(18, 18, 20))
            .text_color(Color::rgb8(218, 221, 228))
            .child(
                div()
                    .h(HEADER_HEIGHT)
                    .w_full()
                    .flex_none()
                    .flex_row()
                    .items_center()
                    .justify_between()
                    .px_4()
                    .border(1.0, Color::rgb8(49, 51, 58))
                    .bg(Color::rgb8(24, 25, 29))
                    .child(
                        div()
                            .flex_col()
                            .child(
                                text(self.title.clone())
                                    .text_xl()
                                    .font_semibold()
                                    .text_color(Color::rgb8(244, 245, 247)),
                            )
                            .child(
                                text(self.subtitle.clone())
                                    .text_xs()
                                    .text_color(Color::rgb8(151, 156, 168)),
                            ),
                    )
                    .child(
                        text(self.metrics_text.clone())
                            .text_xs()
                            .no_wrap()
                            .text_color(Color::rgb8(94, 234, 212)),
                    ),
            )
            .child(
                div()
                    .id("virtual-list-viewport")
                    .relative()
                    .flex_1()
                    .w_full()
                    .overflow_hidden()
                    .children(rows)
                    .when(scrollbar.is_some(), |element| {
                        element.child(scrollbar.expect("checked above"))
                    }),
            )
            .child(
                div()
                    .h(FOOTER_HEIGHT)
                    .w_full()
                    .flex_none()
                    .flex_row()
                    .items_center()
                    .px_4()
                    .border(1.0, Color::rgb8(49, 51, 58))
                    .bg(Color::rgb8(24, 25, 29))
                    .child(
                        text(self.help.clone())
                            .text_xs()
                            .text_color(Color::rgb8(139, 144, 156)),
                    ),
            );

        self.prune_labels(visible.range.start, visible.range.end);
        tree
    }
}
