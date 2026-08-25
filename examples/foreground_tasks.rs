use std::time::Duration;

use quickgui::{
    App, AppConfig, AsyncContextError, AsyncViewContext, Color, Element, EventContext, Task, View,
    ViewContext, button, div, text,
};

fn main() -> Result<(), quickgui::AppError> {
    App::new(ForegroundTasksDemo::default())
        .config(
            AppConfig::new("QuickGUI — Foreground tasks")
                .size(760.0, 520.0)
                .minimum_size(560.0, 420.0)
                .background(Color::rgb8(16, 18, 23)),
        )
        .run()
}

struct ForegroundTasksDemo {
    retained_task: Option<Task<Result<(), AsyncContextError>>>,
    generation: u64,
    retained_running: bool,
    retained_status: String,
    retained_completions: u32,
    detached_started: u32,
    detached_completions: u32,
    detached_status: String,
}

impl Default for ForegroundTasksDemo {
    fn default() -> Self {
        Self {
            retained_task: None,
            generation: 0,
            retained_running: false,
            retained_status: "Idle — start a retained task".to_owned(),
            retained_completions: 0,
            detached_started: 0,
            detached_completions: 0,
            detached_status: "No detached task started".to_owned(),
        }
    }
}

impl ForegroundTasksDemo {
    fn control(label: &'static str) -> Element {
        button()
            .min_h(40.0)
            .px_4()
            .py_2()
            .flex_row()
            .items_center()
            .justify_center()
            .rounded_lg()
            .border(1.0, Color::rgb8(69, 77, 93))
            .bg(Color::rgb8(35, 40, 50))
            .hover(|style| style.bg(Color::rgb8(47, 54, 67)))
            .active(|style| style.bg(Color::rgb8(31, 79, 117)))
            .child(text(label).font_medium())
    }

    fn start_retained(&mut self, cx: &mut EventContext) {
        if let Some(task) = self.retained_task.take() {
            task.cancel();
        }
        self.generation = self.generation.wrapping_add(1).max(1);
        let generation = self.generation;
        self.retained_running = true;
        self.retained_status = "Phase 0 — sleeping for 700 ms".to_owned();

        match cx.spawn(|task_cx: AsyncViewContext<Self>| async move {
            task_cx.sleep(Duration::from_millis(700)).await?;
            task_cx
                .update(move |this, cx| {
                    if this.generation == generation {
                        this.retained_status =
                            "Phase 1 — awake once, sleeping for 1.4 s".to_owned();
                        cx.invalidate();
                    }
                })
                .await?;

            task_cx.sleep(Duration::from_millis(1_400)).await?;
            task_cx
                .update(move |this, cx| {
                    if this.generation == generation {
                        this.retained_running = false;
                        this.retained_completions = this.retained_completions.saturating_add(1);
                        this.retained_status = "Complete — exactly two timer wakeups".to_owned();
                        cx.invalidate();
                    }
                })
                .await?;
            Ok::<(), AsyncContextError>(())
        }) {
            Ok(task) => self.retained_task = Some(task),
            Err(error) => {
                self.retained_running = false;
                self.retained_status = format!("Could not start: {error}");
            }
        }
        cx.invalidate();
    }

    fn cancel_retained(&mut self, cx: &mut EventContext) {
        self.generation = self.generation.wrapping_add(1).max(1);
        if let Some(task) = self.retained_task.take() {
            task.cancel();
            self.retained_status = "Cancelled by dropping the task handle".to_owned();
        } else {
            self.retained_status = "No retained task to cancel".to_owned();
        }
        self.retained_running = false;
        cx.invalidate();
    }

    fn start_detached(&mut self, cx: &mut EventContext) {
        self.detached_started = self.detached_started.saturating_add(1);
        let ticket = self.detached_started;
        match cx.spawn(|task_cx: AsyncViewContext<Self>| async move {
            task_cx.sleep(Duration::from_millis(1_200)).await?;
            task_cx
                .update(move |this, cx| {
                    this.detached_completions = this.detached_completions.saturating_add(1);
                    this.detached_status = format!("Detached task #{ticket} completed");
                    cx.invalidate();
                })
                .await?;
            Ok::<(), AsyncContextError>(())
        }) {
            Ok(task) => {
                task.detach();
                self.detached_status = format!("Detached task #{ticket} sleeping for 1.2 s");
            }
            Err(error) => self.detached_status = format!("Could not start: {error}"),
        }
        cx.invalidate();
    }
}

impl View for ForegroundTasksDemo {
    fn render(&mut self, cx: &mut ViewContext<'_, Self>) -> impl quickgui::IntoElement {
        let start = cx.listener("start-retained", |this, cx| this.start_retained(cx));
        let cancel = cx.listener("cancel-retained", |this, cx| this.cancel_retained(cx));
        let detach = cx.listener("start-detached", |this, cx| this.start_detached(cx));
        let close = cx.listener("close-window", |_this, cx| cx.close_window());
        let metrics = cx.metrics();
        let retained_color = if self.retained_running {
            Color::rgb8(250, 204, 21)
        } else {
            Color::rgb8(94, 234, 212)
        };

        div()
            .size_full()
            .p_6()
            .flex_col()
            .gap_5()
            .bg(Color::rgb8(16, 18, 23))
            .text_color(Color::rgb8(235, 238, 244))
            .child(
                div()
                    .flex_col()
                    .gap_1()
                    .child(text("Cancellable foreground tasks").text_2xl().font_bold())
                    .child(
                        text("Futures run only when woken. Exact timers leave Winit asleep between deadlines; no timer thread or animation frames are used.")
                            .wrap()
                            .text_sm()
                            .text_color(Color::rgb8(151, 160, 178)),
                    ),
            )
            .child(
                div()
                    .p_4()
                    .flex_col()
                    .gap_2()
                    .rounded_xl()
                    .border(1.0, Color::rgb8(56, 64, 79))
                    .bg(Color::rgb8(25, 29, 37))
                    .child(text("Retained task").font_semibold())
                    .child(text(self.retained_status.clone()).text_color(retained_color))
                    .child(
                        text(format!(
                            "Completed: {} · last rendered frame: {}",
                            self.retained_completions, metrics.frame_number
                        ))
                        .text_sm()
                        .text_color(Color::rgb8(151, 160, 178)),
                    )
                    .child(
                        div()
                            .flex_row()
                            .flex_wrap()
                            .gap_2()
                            .child(Self::control("Start / restart").on_click(start))
                            .child(Self::control("Cancel").on_click(cancel)),
                    ),
            )
            .child(
                div()
                    .p_4()
                    .flex_col()
                    .gap_2()
                    .rounded_xl()
                    .border(1.0, Color::rgb8(56, 64, 79))
                    .bg(Color::rgb8(25, 29, 37))
                    .child(text("Detached task").font_semibold())
                    .child(text(self.detached_status.clone()).text_color(Color::rgb8(125, 211, 252)))
                    .child(
                        text(format!("Completed: {}", self.detached_completions))
                            .text_sm()
                            .text_color(Color::rgb8(151, 160, 178)),
                    )
                    .child(
                        div()
                            .flex_row()
                            .flex_wrap()
                            .gap_2()
                            .child(Self::control("Start detached").on_click(detach))
                            .child(Self::control("Close window (cancels all)").on_click(close)),
                    ),
            )
    }
}
