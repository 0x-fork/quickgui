use std::{
    collections::HashMap,
    error::Error,
    fmt,
    io::{self, Write},
    sync::Arc,
    sync::atomic::{AtomicBool, Ordering},
    time::{Duration, Instant},
};

use quickgui::{
    Application, AsyncContextError, AsyncViewContext, Color, Element, ElementId, Event,
    EventContext, FrameMetrics, Key, View, ViewContext, VirtualList, WindowKind, div, text,
};
use serde::Serialize;

const ROWS: usize = 100_000;
const ROW_HEIGHT: f32 = 32.0;
const HEADER_HEIGHT: f32 = 72.0;
const FOOTER_HEIGHT: f32 = 34.0;
const ROW_ID_BASE: u64 = 0x1000_0000_0000_0000;
const LABEL_CACHE_LIMIT: usize = 512;
const PERF_WARMUP_FRAMES: usize = 120;
const PERF_MEASURE_FRAMES: usize = 360;
const PERF_SETTLE_DURATION: Duration = Duration::from_secs(1);
const PERF_IDLE_DURATION: Duration = Duration::from_secs(2);
const PERF_TIMEOUT: Duration = Duration::from_secs(30);
const PERF_FRAMES_PER_TRAVERSAL: f32 = 90.0;

fn main() -> Result<(), Box<dyn Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .with_target(false)
        .compact()
        .init();

    let automated_probe = std::env::args_os().any(|argument| argument == "--perf-probe");
    let probe_failed = Arc::new(AtomicBool::new(false));

    let mut options =
        quickgui::WindowOptions::new("QuickGUI — 100,000 row stress test").size(1_000.0, 720.0);
    if automated_probe {
        // macOS suspends redraw delivery for fully occluded windows. Keep the live probe exposed
        // on the primary display while a terminal or CI harness captures its output.
        options = options
            .window_kind(WindowKind::Floating)
            .position(80.0, 80.0);
    }
    let view_probe_failed = Arc::clone(&probe_failed);
    Application::new().run(move |cx| {
        cx.open_window(options, ScrollDemo::new(automated_probe, view_probe_failed));
    })?;

    if probe_failed.load(Ordering::Relaxed) {
        return Err(Box::new(PerformanceGateFailed));
    }
    Ok(())
}

#[derive(Debug)]
struct PerformanceGateFailed;

impl fmt::Display for PerformanceGateFailed {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("the QuickGUI scrolling performance probe exceeded its budget")
    }
}

impl Error for PerformanceGateFailed {}

struct ScrollDemo {
    list: VirtualList,
    selected: Option<usize>,
    labels: HashMap<usize, Arc<str>>,
    title: Arc<str>,
    subtitle: Arc<str>,
    help: Arc<str>,
    metrics_text: Arc<str>,
    last_metrics_update: Instant,
    synthetic_scroll: bool,
    synthetic_scroll_forward: bool,
    perf_probe: Option<PerformanceProbe>,
    perf_watchdog_started: bool,
}

impl ScrollDemo {
    fn new(automated_probe: bool, probe_failed: Arc<AtomicBool>) -> Self {
        Self {
            list: VirtualList::new(ROWS, ROW_HEIGHT).with_overscan(2),
            selected: None,
            labels: HashMap::with_capacity(LABEL_CACHE_LIMIT),
            title: Arc::from("QuickGUI virtual list"),
            subtitle: Arc::from(
                "100,000 rows · only visible rows are shaped, laid out, and painted",
            ),
            help: Arc::from(
                "Scroll or use ↑ ↓ Page Up Page Down Home End · Space stress scroll · Esc quits",
            ),
            metrics_text: Arc::from(if automated_probe {
                "Automated performance probe · warmup"
            } else {
                "Waiting for the first frame…"
            }),
            last_metrics_update: Instant::now(),
            synthetic_scroll: automated_probe,
            synthetic_scroll_forward: true,
            perf_probe: automated_probe.then(|| PerformanceProbe::new(probe_failed)),
            perf_watchdog_started: false,
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

    fn row(&mut self, index: usize) -> Element {
        let top = index as f32 * ROW_HEIGHT - self.list.scroll_offset();
        let selected = self.selected == Some(index);
        div()
            .id(ElementId::new(ROW_ID_BASE | index as u64))
            .clickable()
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
                    .h(20.0)
                    .flex_1()
                    .min_w(0.0)
                    .no_wrap()
                    .text_shaping_basic()
                    .text_color(Color::rgb8(218, 221, 228)),
            )
    }

    fn prepare_label_cache(&mut self, visible_start: usize, visible_end: usize) {
        let visible_len = visible_end.saturating_sub(visible_start);
        if self.labels.len().saturating_add(visible_len) <= LABEL_CACHE_LIMIT {
            return;
        }

        // Keep a contiguous window around the incoming rows. Filling every missing visible label
        // afterwards still cannot take the map above LABEL_CACHE_LIMIT, even for discontinuous
        // multi-viewport jumps in the automated probe.
        let spare = LABEL_CACHE_LIMIT.saturating_sub(visible_len);
        let before = spare / 2;
        let after = spare - before;
        let keep_start = visible_start.saturating_sub(before);
        let keep_end = visible_end.saturating_add(after).min(self.list.len());
        self.labels
            .retain(|index, _| *index >= keep_start && *index < keep_end);
    }

    fn update_performance_probe(&mut self, cx: &mut ViewContext<'_, Self>) {
        if self.perf_probe.is_some() && !self.perf_watchdog_started {
            self.perf_watchdog_started = true;
            match cx.spawn(|task_cx: AsyncViewContext<Self>| async move {
                task_cx.sleep(PERF_TIMEOUT).await?;
                task_cx
                    .update(|this, cx| {
                        if let Some(probe) = &mut this.perf_probe
                            && !probe.is_finished()
                        {
                            probe.fail_timeout();
                            cx.exit();
                        }
                    })
                    .await?;
                Ok::<(), AsyncContextError>(())
            }) {
                Ok(task) => task.detach(),
                Err(error) => panic!("could not start the performance-probe watchdog: {error}"),
            }
        }

        let Some(probe) = &mut self.perf_probe else {
            return;
        };

        match probe.observe(cx.metrics(), Instant::now(), self.labels.len()) {
            ProbeAction::Scroll => self.synthetic_scroll = true,
            ProbeAction::WaitForIdle(deadline) => {
                self.synthetic_scroll = false;
                self.metrics_text = Arc::from("Automated performance probe · idle verification");
                cx.request_repaint_at(deadline);
            }
            ProbeAction::Exit => {
                self.synthetic_scroll = false;
                match cx.spawn(|task_cx: AsyncViewContext<Self>| async move {
                    task_cx.update(|_, cx| cx.exit()).await
                }) {
                    Ok(task) => task.detach(),
                    Err(error) => panic!("could not exit the completed performance probe: {error}"),
                }
            }
        }
    }
}

impl View for ScrollDemo {
    fn event(&mut self, event: &Event, cx: &mut EventContext) {
        match event {
            Event::Click(id)
                if (ROW_ID_BASE..ROW_ID_BASE + self.list.len() as u64).contains(&id.as_u64()) =>
            {
                self.selected = Some((id.as_u64() - ROW_ID_BASE) as usize);
                cx.invalidate();
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
                    Key::Space => {
                        if self.perf_probe.is_none() {
                            self.synthetic_scroll = !self.synthetic_scroll;
                            cx.invalidate();
                        }
                        None
                    }
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
        self.update_performance_probe(cx);
        let list_height = (cx.size().height - HEADER_HEIGHT - FOOTER_HEIGHT).max(0.0);
        self.list.set_viewport_height(list_height);
        if self.synthetic_scroll {
            let step = if self.perf_probe.is_some() {
                (self.list.max_scroll_offset() / PERF_FRAMES_PER_TRAVERSAL).max(ROW_HEIGHT)
            } else {
                (list_height * 6.0).max(ROW_HEIGHT)
            };
            let next = if self.synthetic_scroll_forward {
                self.list.scroll_offset() + step
            } else {
                self.list.scroll_offset() - step
            };
            if next >= self.list.max_scroll_offset() {
                self.synthetic_scroll_forward = false;
            } else if next <= 0.0 {
                self.synthetic_scroll_forward = true;
            }
            self.list.scroll_to(next);
            cx.request_animation_frame();
        }
        let visible = self.list.visible_rows();
        self.prepare_label_cache(visible.range.start, visible.range.end);
        let rows: Vec<_> = visible.range.clone().map(|index| self.row(index)).collect();

        if self.perf_probe.is_none()
            && self.last_metrics_update.elapsed() >= Duration::from_millis(250)
        {
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

        div()
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
                    .virtual_scroll(&self.list)
                    .children(rows),
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
            )
    }
}

#[derive(Clone, Copy, Debug, Serialize)]
struct PerformanceBudgets {
    max_framework_cpu_percent: f64,
    max_p95_frame_cpu_ms: f64,
    max_frame_cpu_ms: f64,
    max_visible_text_areas: usize,
    max_retained_text_areas: usize,
    max_retained_text_layouts: usize,
    max_retained_text_renderers: usize,
    max_draw_calls: usize,
    max_label_cache_entries: usize,
    max_idle_extra_frames: u64,
}

impl Default for PerformanceBudgets {
    fn default() -> Self {
        Self {
            max_framework_cpu_percent: 30.0,
            max_p95_frame_cpu_ms: 8.0,
            max_frame_cpu_ms: 20.0,
            max_visible_text_areas: 32,
            max_retained_text_areas: 256,
            max_retained_text_layouts: 256,
            max_retained_text_renderers: 32,
            max_draw_calls: 8,
            max_label_cache_entries: LABEL_CACHE_LIMIT,
            max_idle_extra_frames: 0,
        }
    }
}

#[derive(Debug, Serialize)]
struct PerformanceReport {
    schema_version: u32,
    rows: usize,
    warmup_frames: usize,
    measured_frames: usize,
    elapsed_ms: f64,
    observed_presentation_hz: f64,
    total_frame_cpu_ms: f64,
    framework_cpu_percent: f64,
    average_frame_cpu_ms: f64,
    p95_frame_cpu_ms: f64,
    max_frame_cpu_ms: f64,
    average_frame_submission_ms: f64,
    p95_frame_submission_ms: f64,
    max_frame_submission_ms: f64,
    max_visible_text_areas: usize,
    max_reshaped_text_areas: usize,
    max_retained_text_areas: usize,
    max_retained_text_layouts: usize,
    max_retained_text_renderers: usize,
    max_draw_calls: usize,
    max_label_cache_entries: usize,
    idle_seconds: f64,
    idle_frame_delta: u64,
    idle_extra_frames: u64,
    budgets: PerformanceBudgets,
    passed: bool,
    failures: Vec<String>,
}

struct PerformanceSamples {
    started_at: Instant,
    cpu_ms: Vec<f64>,
    submission_ms: Vec<f64>,
    max_visible_text_areas: usize,
    max_reshaped_text_areas: usize,
    max_retained_text_areas: usize,
    max_retained_text_layouts: usize,
    max_retained_text_renderers: usize,
    max_draw_calls: usize,
    max_label_cache_entries: usize,
}

impl PerformanceSamples {
    fn new(started_at: Instant) -> Self {
        Self {
            started_at,
            cpu_ms: Vec::with_capacity(PERF_MEASURE_FRAMES),
            submission_ms: Vec::with_capacity(PERF_MEASURE_FRAMES),
            max_visible_text_areas: 0,
            max_reshaped_text_areas: 0,
            max_retained_text_areas: 0,
            max_retained_text_layouts: 0,
            max_retained_text_renderers: 0,
            max_draw_calls: 0,
            max_label_cache_entries: 0,
        }
    }

    fn push(&mut self, metrics: FrameMetrics, label_cache_entries: usize) {
        self.cpu_ms.push(metrics.cpu_milliseconds());
        self.submission_ms.push(metrics.frame_milliseconds());
        self.max_visible_text_areas = self.max_visible_text_areas.max(metrics.render.text_areas);
        self.max_reshaped_text_areas = self
            .max_reshaped_text_areas
            .max(metrics.render.reshaped_text_areas);
        self.max_retained_text_areas = self
            .max_retained_text_areas
            .max(metrics.render.retained_text_areas);
        self.max_retained_text_layouts = self
            .max_retained_text_layouts
            .max(metrics.render.retained_text_layouts);
        self.max_retained_text_renderers = self
            .max_retained_text_renderers
            .max(metrics.render.retained_text_renderers);
        self.max_draw_calls = self.max_draw_calls.max(metrics.render.draw_calls);
        self.max_label_cache_entries = self.max_label_cache_entries.max(label_cache_entries);
    }

    fn finish(self, now: Instant, budgets: PerformanceBudgets) -> PerformanceReport {
        let elapsed = now.saturating_duration_since(self.started_at);
        let elapsed_seconds = elapsed.as_secs_f64().max(f64::EPSILON);
        let total_frame_cpu_ms = self.cpu_ms.iter().sum::<f64>();
        let measured_frames = self.cpu_ms.len();
        let average_frame_cpu_ms = if measured_frames == 0 {
            0.0
        } else {
            total_frame_cpu_ms / measured_frames as f64
        };
        let mut sorted_cpu_ms = self.cpu_ms;
        sorted_cpu_ms.sort_by(f64::total_cmp);
        let p95_frame_cpu_ms = percentile(&sorted_cpu_ms, 0.95);
        let max_frame_cpu_ms = sorted_cpu_ms.last().copied().unwrap_or_default();
        let average_frame_submission_ms = if measured_frames == 0 {
            0.0
        } else {
            self.submission_ms.iter().sum::<f64>() / measured_frames as f64
        };
        let mut sorted_submission_ms = self.submission_ms;
        sorted_submission_ms.sort_by(f64::total_cmp);
        let p95_frame_submission_ms = percentile(&sorted_submission_ms, 0.95);
        let max_frame_submission_ms = sorted_submission_ms.last().copied().unwrap_or_default();
        let mut report = PerformanceReport {
            schema_version: 1,
            rows: ROWS,
            warmup_frames: PERF_WARMUP_FRAMES,
            measured_frames,
            elapsed_ms: elapsed.as_secs_f64() * 1_000.0,
            observed_presentation_hz: measured_frames as f64 / elapsed_seconds,
            total_frame_cpu_ms,
            framework_cpu_percent: total_frame_cpu_ms / 1_000.0 / elapsed_seconds * 100.0,
            average_frame_cpu_ms,
            p95_frame_cpu_ms,
            max_frame_cpu_ms,
            average_frame_submission_ms,
            p95_frame_submission_ms,
            max_frame_submission_ms,
            max_visible_text_areas: self.max_visible_text_areas,
            max_reshaped_text_areas: self.max_reshaped_text_areas,
            max_retained_text_areas: self.max_retained_text_areas,
            max_retained_text_layouts: self.max_retained_text_layouts,
            max_retained_text_renderers: self.max_retained_text_renderers,
            max_draw_calls: self.max_draw_calls,
            max_label_cache_entries: self.max_label_cache_entries,
            idle_seconds: PERF_IDLE_DURATION.as_secs_f64(),
            idle_frame_delta: 0,
            idle_extra_frames: 0,
            budgets,
            passed: true,
            failures: Vec::new(),
        };
        evaluate_active_budgets(&mut report);
        report
    }
}

enum ProbePhase {
    Warmup {
        observed_frames: usize,
    },
    Measuring(PerformanceSamples),
    Settling {
        deadline: Instant,
        report: PerformanceReport,
    },
    Idle {
        deadline: Instant,
        baseline_frame: u64,
        report: PerformanceReport,
    },
    Finished,
}

enum ProbeAction {
    Scroll,
    WaitForIdle(Instant),
    Exit,
}

struct PerformanceProbe {
    phase: ProbePhase,
    last_frame: u64,
    budgets: PerformanceBudgets,
    failed: Arc<AtomicBool>,
}

impl PerformanceProbe {
    fn new(failed: Arc<AtomicBool>) -> Self {
        Self {
            phase: ProbePhase::Warmup { observed_frames: 0 },
            last_frame: 0,
            budgets: PerformanceBudgets::default(),
            failed,
        }
    }

    fn observe(
        &mut self,
        metrics: FrameMetrics,
        now: Instant,
        label_cache_entries: usize,
    ) -> ProbeAction {
        if metrics.frame_number == 0 || metrics.frame_number == self.last_frame {
            return match self.phase {
                ProbePhase::Warmup { .. } | ProbePhase::Measuring(_) => ProbeAction::Scroll,
                ProbePhase::Settling { deadline, .. } => ProbeAction::WaitForIdle(deadline),
                ProbePhase::Idle { deadline, .. } => ProbeAction::WaitForIdle(deadline),
                ProbePhase::Finished => ProbeAction::Exit,
            };
        }
        self.last_frame = metrics.frame_number;

        let phase = std::mem::replace(&mut self.phase, ProbePhase::Finished);
        match phase {
            ProbePhase::Warmup { observed_frames } => {
                let observed_frames = observed_frames + 1;
                if observed_frames >= PERF_WARMUP_FRAMES {
                    emit_probe_line(&format!(
                        "QUICKGUI_PERF_BEGIN warmup_frames={PERF_WARMUP_FRAMES} measured_frames={PERF_MEASURE_FRAMES} rows={ROWS}"
                    ));
                    self.phase = ProbePhase::Measuring(PerformanceSamples::new(now));
                } else {
                    self.phase = ProbePhase::Warmup { observed_frames };
                }
                ProbeAction::Scroll
            }
            ProbePhase::Measuring(mut samples) => {
                samples.push(metrics, label_cache_entries);
                if samples.cpu_ms.len() >= PERF_MEASURE_FRAMES {
                    let report = samples.finish(now, self.budgets);
                    let deadline = now.checked_add(PERF_SETTLE_DURATION).unwrap_or(now);
                    self.phase = ProbePhase::Settling { deadline, report };
                    ProbeAction::WaitForIdle(deadline)
                } else {
                    self.phase = ProbePhase::Measuring(samples);
                    ProbeAction::Scroll
                }
            }
            ProbePhase::Settling { deadline, report } => {
                if now < deadline {
                    self.phase = ProbePhase::Settling { deadline, report };
                    ProbeAction::WaitForIdle(deadline)
                } else {
                    let deadline = now.checked_add(PERF_IDLE_DURATION).unwrap_or(now);
                    self.phase = ProbePhase::Idle {
                        deadline,
                        baseline_frame: metrics.frame_number,
                        report,
                    };
                    ProbeAction::WaitForIdle(deadline)
                }
            }
            ProbePhase::Idle {
                deadline: _,
                baseline_frame,
                mut report,
            } => {
                report.idle_frame_delta = metrics.frame_number.saturating_sub(baseline_frame);
                report.idle_extra_frames = report.idle_frame_delta.saturating_sub(1);
                evaluate_idle_budget(&mut report);
                report.passed = report.failures.is_empty();
                self.failed.store(!report.passed, Ordering::Relaxed);
                let serialized = serde_json::to_string(&report).unwrap_or_else(|error| {
                    format!("{{\"passed\":false,\"serialization_error\":\"{error}\"}}")
                });
                emit_probe_line(&format!("QUICKGUI_PERF_RESULT {serialized}"));
                self.phase = ProbePhase::Finished;
                ProbeAction::Exit
            }
            ProbePhase::Finished => {
                self.phase = ProbePhase::Finished;
                ProbeAction::Exit
            }
        }
    }

    fn is_finished(&self) -> bool {
        matches!(self.phase, ProbePhase::Finished)
    }

    fn fail_timeout(&mut self) {
        self.failed.store(true, Ordering::Relaxed);
        self.phase = ProbePhase::Finished;
        emit_probe_line(&format!(
            "QUICKGUI_PERF_ERROR timeout_seconds={}",
            PERF_TIMEOUT.as_secs()
        ));
    }
}

fn percentile(sorted: &[f64], percentile: f64) -> f64 {
    if sorted.is_empty() {
        return 0.0;
    }
    let rank = (sorted.len() as f64 * percentile.clamp(0.0, 1.0)).ceil() as usize;
    sorted[rank.saturating_sub(1).min(sorted.len() - 1)]
}

fn evaluate_active_budgets(report: &mut PerformanceReport) {
    push_failure_if_above(
        &mut report.failures,
        "framework_cpu_percent",
        report.framework_cpu_percent,
        report.budgets.max_framework_cpu_percent,
    );
    push_failure_if_above(
        &mut report.failures,
        "p95_frame_cpu_ms",
        report.p95_frame_cpu_ms,
        report.budgets.max_p95_frame_cpu_ms,
    );
    push_failure_if_above(
        &mut report.failures,
        "max_frame_cpu_ms",
        report.max_frame_cpu_ms,
        report.budgets.max_frame_cpu_ms,
    );
    push_usize_failure_if_above(
        &mut report.failures,
        "max_visible_text_areas",
        report.max_visible_text_areas,
        report.budgets.max_visible_text_areas,
    );
    push_usize_failure_if_above(
        &mut report.failures,
        "max_retained_text_areas",
        report.max_retained_text_areas,
        report.budgets.max_retained_text_areas,
    );
    push_usize_failure_if_above(
        &mut report.failures,
        "max_retained_text_layouts",
        report.max_retained_text_layouts,
        report.budgets.max_retained_text_layouts,
    );
    push_usize_failure_if_above(
        &mut report.failures,
        "max_retained_text_renderers",
        report.max_retained_text_renderers,
        report.budgets.max_retained_text_renderers,
    );
    push_usize_failure_if_above(
        &mut report.failures,
        "max_draw_calls",
        report.max_draw_calls,
        report.budgets.max_draw_calls,
    );
    push_usize_failure_if_above(
        &mut report.failures,
        "max_label_cache_entries",
        report.max_label_cache_entries,
        report.budgets.max_label_cache_entries,
    );
}

fn evaluate_idle_budget(report: &mut PerformanceReport) {
    if report.idle_frame_delta == 0 {
        report.failures.push(
            "idle_frame_delta was 0; the scheduled verification frame was not presented".to_owned(),
        );
    }
    if report.idle_extra_frames > report.budgets.max_idle_extra_frames {
        report.failures.push(format!(
            "idle_extra_frames was {}, budget is {}",
            report.idle_extra_frames, report.budgets.max_idle_extra_frames
        ));
    }
}

fn push_failure_if_above(failures: &mut Vec<String>, name: &str, actual: f64, maximum: f64) {
    if actual > maximum {
        failures.push(format!("{name} was {actual:.3}, budget is {maximum:.3}"));
    }
}

fn push_usize_failure_if_above(
    failures: &mut Vec<String>,
    name: &str,
    actual: usize,
    maximum: usize,
) {
    if actual > maximum {
        failures.push(format!("{name} was {actual}, budget is {maximum}"));
    }
}

fn emit_probe_line(line: &str) {
    let mut stdout = io::stdout().lock();
    let _ = writeln!(stdout, "{line}");
    let _ = stdout.flush();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn percentile_uses_nearest_rank_without_interpolation() {
        let samples: Vec<_> = (1..=100).map(f64::from).collect();
        assert_eq!(percentile(&samples, 0.95), 95.0);
        assert_eq!(percentile(&samples, 0.0), 1.0);
        assert_eq!(percentile(&[], 0.95), 0.0);
    }

    #[test]
    fn active_and_idle_budgets_report_exact_failures() {
        let now = Instant::now();
        let mut samples = PerformanceSamples::new(now);
        let mut metrics = FrameMetrics::default();
        metrics.cpu_time = Duration::from_millis(9);
        metrics.render.text_areas = 33;
        metrics.render.retained_text_areas = 257;
        metrics.render.retained_text_layouts = 257;
        metrics.render.retained_text_renderers = 33;
        metrics.render.draw_calls = 9;
        samples.push(metrics, LABEL_CACHE_LIMIT + 1);
        let mut report = samples.finish(
            now + Duration::from_millis(9),
            PerformanceBudgets::default(),
        );
        report.idle_frame_delta = 3;
        report.idle_extra_frames = 2;
        evaluate_idle_budget(&mut report);

        assert!(
            report
                .failures
                .iter()
                .any(|failure| failure.starts_with("framework_cpu_percent"))
        );
        assert!(
            report
                .failures
                .iter()
                .any(|failure| failure.starts_with("p95_frame_cpu_ms"))
        );
        assert!(
            report
                .failures
                .iter()
                .any(|failure| failure.starts_with("max_visible_text_areas"))
        );
        assert!(
            report
                .failures
                .iter()
                .any(|failure| failure.starts_with("max_retained_text_areas"))
        );
        assert!(
            report
                .failures
                .iter()
                .any(|failure| failure.starts_with("max_retained_text_layouts"))
        );
        assert!(
            report
                .failures
                .iter()
                .any(|failure| failure.starts_with("max_retained_text_renderers"))
        );
        assert!(
            report
                .failures
                .iter()
                .any(|failure| failure.starts_with("max_draw_calls"))
        );
        assert!(
            report
                .failures
                .iter()
                .any(|failure| failure.starts_with("max_label_cache_entries"))
        );
        assert!(
            report
                .failures
                .iter()
                .any(|failure| failure.starts_with("idle_extra_frames"))
        );
    }
}
