use std::{
    error::Error,
    fmt,
    io::{self, Write},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};

use quickgui::{
    Application, AsyncContextError, AsyncViewContext, Color, Display, DisplayId, Element,
    EventContext, IntoElement, Rect, Size, View, ViewContext, WindowOptions, WindowState, button,
    div, text,
};
use serde::Serialize;

#[cfg(target_os = "macos")]
use objc2::{rc::Retained, runtime::AnyObject};
#[cfg(target_os = "macos")]
use objc2_app_kit::{NSApplication, NSScreen, NSWindow};
#[cfg(target_os = "macos")]
use objc2_foundation::{MainThreadMarker, NSNumber, NSRect, ns_string};

const DISPLAY_ACCEPTANCE_ENV: &str = "QUICKGUI_DISPLAY_ACCEPTANCE";
const REQUIRE_SECONDARY_ENV: &str = "QUICKGUI_DISPLAY_ACCEPTANCE_REQUIRE_SECONDARY";
const REQUIRE_MIXED_SCALE_ENV: &str = "QUICKGUI_DISPLAY_ACCEPTANCE_REQUIRE_MIXED_SCALE";
const ACCEPTANCE_WINDOW_SIZE: Size = Size::new(560.0, 360.0);
const ACCEPTANCE_SETTLE_DURATION: Duration = Duration::from_millis(150);
const ACCEPTANCE_WATCHDOG_DURATION: Duration = Duration::from_secs(20);
const GEOMETRY_TOLERANCE: f32 = 2.0;
const SCALE_TOLERANCE: f32 = 0.001;

fn main() -> Result<(), Box<dyn Error>> {
    let acceptance_enabled = env_flag(DISPLAY_ACCEPTANCE_ENV);
    let acceptance_root_display = acceptance_enabled.then(native_primary_display_id).flatten();
    let failed = Arc::new(AtomicBool::new(false));
    let options = quickgui::WindowOptions::new("QuickGUI — Displays")
        .size(720.0, 560.0)
        .focus(!acceptance_enabled)
        .show(!acceptance_enabled);
    let options = match acceptance_root_display {
        Some(display) => options.display(display),
        None => options,
    };
    let view_failed = Arc::clone(&failed);
    Application::new()
        .run(move |cx| {
            cx.open_window(
                options,
                DisplayBrowser::new(acceptance_enabled, acceptance_root_display, view_failed),
            );
        })
        .map_err(|error| -> Box<dyn Error> { Box::new(error) })?;
    if failed.load(Ordering::Relaxed) {
        return Err(Box::new(DisplayAcceptanceFailed));
    }
    Ok(())
}

#[derive(Debug)]
struct DisplayAcceptanceFailed;

impl fmt::Display for DisplayAcceptanceFailed {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("the QuickGUI display acceptance probe failed")
    }
}

impl Error for DisplayAcceptanceFailed {}

fn env_flag(name: &str) -> bool {
    std::env::var_os(name).is_some_and(|value| {
        let value = value.to_string_lossy();
        value == "1" || value.eq_ignore_ascii_case("true") || value.eq_ignore_ascii_case("yes")
    })
}

struct DisplayBrowser {
    acceptance: Option<DisplayAcceptance>,
}

impl DisplayBrowser {
    fn new(
        acceptance_enabled: bool,
        acceptance_root_display: Option<DisplayId>,
        failed: Arc<AtomicBool>,
    ) -> Self {
        Self {
            acceptance: acceptance_enabled
                .then(|| DisplayAcceptance::new(acceptance_root_display, failed)),
        }
    }

    fn display_card(display: Display, open: quickgui::ClickListener<Self>) -> Element {
        let role = if display.is_primary() {
            "primary"
        } else {
            "active"
        };
        let bounds = display.bounds();
        let work_area = display.visible_bounds();
        div()
            .flex_col()
            .gap_2()
            .p_4()
            .rounded_xl()
            .border(1.0, Color::rgb8(61, 67, 80))
            .bg(Color::rgb8(27, 30, 37))
            .child(
                div()
                    .flex_row()
                    .items_center()
                    .justify_between()
                    .child(text(display.name()).font_semibold().text_lg())
                    .child(text(role).text_xs().text_color(Color::rgb8(94, 234, 212))),
            )
            .child(
                text(format!(
                    "id {} · {:.0}×{:.0} at ({:.0}, {:.0}) · {:.1}× scale",
                    display.id().get(),
                    bounds.width,
                    bounds.height,
                    bounds.x,
                    bounds.y,
                    display.scale_factor(),
                ))
                .text_sm()
                .text_color(Color::rgb8(184, 190, 204)),
            )
            .child(
                text(format!(
                    "work area {:.0}×{:.0} at ({:.0}, {:.0}) · {}",
                    work_area.width,
                    work_area.height,
                    work_area.x,
                    work_area.y,
                    display.refresh_rate_millihertz().map_or_else(
                        || "refresh unknown".to_owned(),
                        |rate| { format!("{:.1} Hz", rate as f32 / 1_000.0) }
                    ),
                ))
                .text_sm()
                .text_color(Color::rgb8(148, 156, 174)),
            )
            .child(
                button()
                    .on_click(open)
                    .min_h(38.0)
                    .px_4()
                    .rounded_lg()
                    .bg(Color::rgb8(42, 96, 160))
                    .hover(|style| style.bg(Color::rgb8(51, 112, 184)))
                    .child("Open centered window here"),
            )
    }

    fn drive_acceptance(&mut self, cx: &mut ViewContext<'_, Self>, displays: &[Display]) {
        let Some(acceptance) = self.acceptance.as_mut() else {
            return;
        };

        if acceptance.root_initial_placement.is_none() {
            let target = acceptance
                .root_target
                .and_then(|id| displays.iter().find(|display| display.id() == id));
            match target {
                Some(target) => {
                    let state = cx.window_state();
                    let observation = InitialPlacementObservation {
                        expected: target.centered_bounds(state.viewport_size),
                        actual: state.bounds.bounds(),
                        retained_display_matches: state.display_id == Some(target.id()),
                        scale_factor_matches: scale_approximately(
                            state.scale_factor,
                            target.scale_factor(),
                        ),
                    };
                    acceptance.root_initial_centered_match = observation.matches();
                    if acceptance.root_initial_centered_match {
                        acceptance.last_root_initial_error = None;
                    } else {
                        acceptance.last_root_initial_error = Some(format!(
                            "target={}, retained={:?}, expected={:?}, actual={:?}, retained_display_matches={}, scale_factor_matches={}",
                            target.id().get(),
                            state.display_id.map(DisplayId::get),
                            observation.expected,
                            observation.actual,
                            observation.retained_display_matches,
                            observation.scale_factor_matches,
                        ));
                    }
                    acceptance.root_initial_placement = Some(observation);
                }
                None => {
                    acceptance.last_root_initial_error = Some(
                        "the acceptance root target was absent from the first public snapshot"
                            .to_owned(),
                    );
                }
            }
        }

        if !acceptance.settled_snapshot_native_match {
            match native_display_snapshot_matches(displays) {
                Ok(()) => {
                    acceptance.settled_snapshot_native_match = true;
                    acceptance.last_snapshot_error = None;
                }
                Err(error) => acceptance.last_snapshot_error = Some(error),
            }
        }
        if acceptance.settled_snapshot_native_match && !acceptance.root_settled_placement_match {
            let target = acceptance
                .root_target
                .and_then(|id| displays.iter().find(|display| display.id() == id));
            match target {
                Some(target) => {
                    let target_id = target.id();
                    let state = cx.window_state();
                    let actual = state.bounds.bounds();
                    let current_display_id = cx.current_display().map(Display::id);
                    match native_window_observation("QuickGUI — Displays") {
                        Ok(native) => {
                            let matches = state.display_id == Some(target_id)
                                && current_display_id == Some(target_id)
                                && scale_approximately(state.scale_factor, target.scale_factor())
                                && native.display_id == target_id.get()
                                && scale_approximately(native.scale_factor, target.scale_factor())
                                && rect_origins_approximately(native.frame, actual)
                                && rect_approximately(native.screen_bounds, target.bounds())
                                && rect_approximately(
                                    native.visible_bounds,
                                    target.visible_bounds(),
                                )
                                && rect_contains_rect_with_tolerance(
                                    target.visible_bounds(),
                                    native.frame,
                                )
                                && !state.visible
                                && !state.focused
                                && !native.visible
                                && !native.key;
                            if matches {
                                acceptance.root_settled_placement_match = true;
                                acceptance.last_root_settled_placement_error = None;
                            } else {
                                acceptance.last_root_settled_placement_error = Some(format!(
                                    "target={}, retained={:?}, current={:?}, actual={actual:?}, target_bounds={:?}, target_visible={:?}, native={native:?}",
                                    target_id.get(),
                                    state.display_id.map(DisplayId::get),
                                    current_display_id.map(DisplayId::get),
                                    target.bounds(),
                                    target.visible_bounds(),
                                ));
                            }
                        }
                        Err(error) => acceptance.last_root_settled_placement_error = Some(error),
                    }
                }
                None => {
                    acceptance.last_root_settled_placement_error = Some(
                        "the acceptance root target was absent from the public snapshot".to_owned(),
                    );
                }
            }
        }

        if !acceptance.watchdog_started {
            acceptance.watchdog_started = true;
            let failed = Arc::clone(&acceptance.failed);
            match cx.spawn(|task_cx: AsyncViewContext<Self>| async move {
                task_cx.sleep(ACCEPTANCE_WATCHDOG_DURATION).await?;
                task_cx
                    .update(move |this, cx| {
                        let Some(acceptance) = this.acceptance.as_mut() else {
                            return;
                        };
                        if acceptance.phase.is_finished() {
                            return;
                        }
                        failed.store(true, Ordering::Relaxed);
                        emit_line(&format!(
                            "QUICKGUI_DISPLAY_ACCEPTANCE_ERROR timeout_seconds={} phase={} tested_displays={} failures={:?}",
                            ACCEPTANCE_WATCHDOG_DURATION.as_secs(),
                            acceptance.phase.name(),
                            acceptance.results.len(),
                            acceptance.failures,
                        ));
                        acceptance.phase = DisplayAcceptancePhase::Finished;
                        cx.exit();
                    })
                    .await?;
                Ok::<(), AsyncContextError>(())
            }) {
                Ok(task) => task.detach(),
                Err(error) => {
                    acceptance
                        .failures
                        .push(format!("could not start display watchdog: {error}"));
                    acceptance.failed.store(true, Ordering::Relaxed);
                    acceptance.phase = DisplayAcceptancePhase::Reporting;
                }
            }
        }

        if matches!(acceptance.phase, DisplayAcceptancePhase::Discover) {
            acceptance.targets = displays.to_vec();
            acceptance
                .targets
                .sort_by_key(|display| (!display.is_primary(), display.id()));
            if acceptance.targets.is_empty() {
                acceptance
                    .failures
                    .push("the native runtime reported no active displays".to_owned());
                acceptance.phase = DisplayAcceptancePhase::Reporting;
            } else {
                acceptance.phase = DisplayAcceptancePhase::Ready;
            }
        }

        let waiting_handle = match &acceptance.phase {
            DisplayAcceptancePhase::Waiting { handle, .. } => Some(*handle),
            _ => None,
        };
        if let Some(waiting_handle) = waiting_handle {
            cx.on_child_window_closed(waiting_handle, move |this, closed, cx| {
                let Some(acceptance) = this.acceptance.as_mut() else {
                    return;
                };
                let phase =
                    std::mem::replace(&mut acceptance.phase, DisplayAcceptancePhase::Finished);
                let DisplayAcceptancePhase::Waiting {
                    index,
                    handle,
                    result,
                } = phase
                else {
                    acceptance
                        .failures
                        .push("a display child closed outside the waiting phase".to_owned());
                    acceptance.phase = DisplayAcceptancePhase::Reporting;
                    cx.invalidate();
                    return;
                };
                if handle != closed {
                    acceptance.failures.push(format!(
                        "display child {closed:?} closed while waiting for {handle:?}"
                    ));
                }
                match result.lock() {
                    Ok(mut result) => match result.take() {
                        Some(result) => acceptance.results.push(result),
                        None => acceptance.failures.push(format!(
                            "display target {} closed without a placement result",
                            acceptance.targets[index].id().get(),
                        )),
                    },
                    Err(_) => acceptance
                        .failures
                        .push("display placement result storage was poisoned".to_owned()),
                }
                acceptance.next_index = index.saturating_add(1);
                acceptance.phase = DisplayAcceptancePhase::Ready;
                cx.invalidate();
            });
        }

        let scheduled_target = {
            let acceptance = self
                .acceptance
                .as_mut()
                .expect("acceptance presence checked above");
            if !matches!(acceptance.phase, DisplayAcceptancePhase::Ready) {
                None
            } else if acceptance.next_index == acceptance.targets.len() {
                acceptance.phase = DisplayAcceptancePhase::Reporting;
                None
            } else {
                let index = acceptance.next_index;
                let target = acceptance.targets[index].clone();
                let result = Arc::new(Mutex::new(None));
                acceptance.phase = DisplayAcceptancePhase::Opening {
                    index,
                    result: Arc::clone(&result),
                };
                Some((index, target, result))
            }
        };

        if let Some((index, target, result)) = scheduled_target {
            let title = format!("QuickGUI display acceptance {}", target.id().get());
            let child_target = target.clone();
            let child_result = Arc::clone(&result);
            match cx.spawn(move |task_cx: AsyncViewContext<Self>| async move {
                task_cx
                    .update(move |this, cx| {
                        let handle = cx.open_window(
                            WindowOptions::new(title.clone())
                                .size(ACCEPTANCE_WINDOW_SIZE.width, ACCEPTANCE_WINDOW_SIZE.height)
                                .display(child_target.id())
                                .background(Color::rgb8(20, 22, 27))
                                .focus(false)
                                .show(false),
                            TargetWindow::acceptance(
                                child_target.clone(),
                                title.clone(),
                                child_result,
                            ),
                        );
                        let Some(acceptance) = this.acceptance.as_mut() else {
                            cx.close_window_handle(handle);
                            return;
                        };
                        let phase = std::mem::replace(
                            &mut acceptance.phase,
                            DisplayAcceptancePhase::Finished,
                        );
                        match phase {
                            DisplayAcceptancePhase::Opening {
                                index: opening_index,
                                result,
                            } if opening_index == index => {
                                acceptance.phase = DisplayAcceptancePhase::Waiting {
                                    index,
                                    handle,
                                    result,
                                };
                            }
                            other => {
                                acceptance.failures.push(format!(
                                    "display target {} opened from unexpected phase {}",
                                    child_target.id().get(),
                                    other.name(),
                                ));
                                acceptance.phase = DisplayAcceptancePhase::Reporting;
                                cx.close_window_handle(handle);
                            }
                        }
                        cx.invalidate();
                    })
                    .await?;
                Ok::<(), AsyncContextError>(())
            }) {
                Ok(task) => task.detach(),
                Err(error) => {
                    let acceptance = self
                        .acceptance
                        .as_mut()
                        .expect("acceptance presence checked above");
                    acceptance.failures.push(format!(
                        "could not schedule display target {}: {error}",
                        target.id().get(),
                    ));
                    acceptance.next_index = index.saturating_add(1);
                    acceptance.phase = DisplayAcceptancePhase::Ready;
                    cx.request_animation_frame();
                }
            }
        }

        let report = {
            let acceptance = self
                .acceptance
                .as_mut()
                .expect("acceptance presence checked above");
            matches!(acceptance.phase, DisplayAcceptancePhase::Reporting)
                .then(|| acceptance.finish_report())
        };
        if let Some(report) = report {
            let passed = report.passed;
            if let Ok(serialized) = serde_json::to_string(&report) {
                emit_line(&format!("QUICKGUI_DISPLAY_ACCEPTANCE_RESULT {serialized}"));
            } else {
                emit_line(
                    "QUICKGUI_DISPLAY_ACCEPTANCE_RESULT {\"passed\":false,\"serialization_error\":true}",
                );
            }
            let acceptance = self
                .acceptance
                .as_mut()
                .expect("acceptance presence checked above");
            acceptance.failed.store(!passed, Ordering::Relaxed);
            acceptance.phase = DisplayAcceptancePhase::Finished;
            match cx.spawn(|task_cx: AsyncViewContext<Self>| async move {
                task_cx.update(|_, cx| cx.exit()).await
            }) {
                Ok(task) => task.detach(),
                Err(error) => panic!("could not exit the completed display probe: {error}"),
            }
        }
    }
}

struct DisplayAcceptance {
    phase: DisplayAcceptancePhase,
    targets: Vec<Display>,
    next_index: usize,
    results: Vec<DisplayPlacementResult>,
    failures: Vec<String>,
    require_secondary: bool,
    require_mixed_scale: bool,
    failed: Arc<AtomicBool>,
    watchdog_started: bool,
    settled_snapshot_native_match: bool,
    last_snapshot_error: Option<String>,
    root_target: Option<DisplayId>,
    root_initial_placement: Option<InitialPlacementObservation>,
    root_initial_centered_match: bool,
    last_root_initial_error: Option<String>,
    root_settled_placement_match: bool,
    last_root_settled_placement_error: Option<String>,
}

impl DisplayAcceptance {
    fn new(root_target: Option<DisplayId>, failed: Arc<AtomicBool>) -> Self {
        Self {
            phase: DisplayAcceptancePhase::Discover,
            targets: Vec::new(),
            next_index: 0,
            results: Vec::new(),
            failures: Vec::new(),
            require_secondary: env_flag(REQUIRE_SECONDARY_ENV),
            require_mixed_scale: env_flag(REQUIRE_MIXED_SCALE_ENV),
            failed,
            watchdog_started: false,
            settled_snapshot_native_match: false,
            last_snapshot_error: None,
            root_target,
            root_initial_placement: None,
            root_initial_centered_match: false,
            last_root_initial_error: None,
            root_settled_placement_match: false,
            last_root_settled_placement_error: None,
        }
    }

    fn finish_report(&mut self) -> DisplayAcceptanceReport {
        let display_count = self.targets.len();
        let tested_displays = self.results.len();
        let secondary_display_available = self.targets.iter().any(|display| !display.is_primary());
        let secondary_display_tested = secondary_display_available
            && self
                .targets
                .iter()
                .filter(|display| !display.is_primary())
                .all(|display| {
                    self.results
                        .iter()
                        .any(|result| result.display_id == display.id().get() && result.passed)
                });
        let primary_scale = self
            .targets
            .iter()
            .find(|display| display.is_primary())
            .or_else(|| self.targets.first())
            .map(Display::scale_factor);
        let mixed_scale_available = primary_scale.is_some_and(|primary_scale| {
            self.targets
                .iter()
                .any(|display| (display.scale_factor() - primary_scale).abs() > SCALE_TOLERANCE)
        });
        let mixed_scale_tested = mixed_scale_available
            && self
                .results
                .iter()
                .filter(|result| result.passed)
                .any(|left| {
                    self.results
                        .iter()
                        .filter(|result| result.passed)
                        .any(|right| {
                            (left.scale_factor - right.scale_factor).abs() > SCALE_TOLERANCE
                        })
                });

        let secondary_status =
            capability_status(secondary_display_available, secondary_display_tested);
        let mixed_scale_status = capability_status(mixed_scale_available, mixed_scale_tested);
        let mut failures = std::mem::take(&mut self.failures);
        if tested_displays != display_count {
            failures.push(format!(
                "tested {tested_displays} displays, expected {display_count}"
            ));
        }
        for result in &self.results {
            if !result.passed {
                failures.push(format!(
                    "display {} placement failed: {}",
                    result.display_id,
                    result.failures.join("; "),
                ));
            }
        }
        if self.require_secondary && secondary_status != "passed" {
            failures.push(format!(
                "secondary-display proof was required but its status was {secondary_status}"
            ));
        }
        if self.require_mixed_scale && mixed_scale_status != "passed" {
            failures.push(format!(
                "mixed-scale proof was required but its status was {mixed_scale_status}"
            ));
        }
        if !self.settled_snapshot_native_match {
            failures.push(format!(
                "the settled public display snapshot never matched AppKit: {}",
                self.last_snapshot_error
                    .as_deref()
                    .unwrap_or("no native comparison was recorded"),
            ));
        }
        if !self.root_initial_centered_match {
            failures.push(format!(
                "the first hidden `.display(id)` window was not centered on its first native render: {}",
                self.last_root_initial_error
                    .as_deref()
                    .unwrap_or("no initial root placement comparison was recorded"),
            ));
        }
        if !self.root_settled_placement_match {
            failures.push(format!(
                "the first hidden `.display(id)` window never matched settled retained and native placement: {}",
                self.last_root_settled_placement_error
                    .as_deref()
                    .unwrap_or("no settled root placement comparison was recorded"),
            ));
        }

        DisplayAcceptanceReport {
            schema_version: 3,
            display_count,
            tested_displays,
            secondary_display_available,
            secondary_display_tested,
            secondary_status,
            mixed_scale_available,
            mixed_scale_tested,
            mixed_scale_status,
            require_secondary: self.require_secondary,
            require_mixed_scale: self.require_mixed_scale,
            settled_snapshot_native_match: self.settled_snapshot_native_match,
            root_initial_centered_match: self.root_initial_centered_match,
            root_settled_placement_match: self.root_settled_placement_match,
            results: std::mem::take(&mut self.results),
            passed: failures.is_empty(),
            failures,
        }
    }
}

enum DisplayAcceptancePhase {
    Discover,
    Ready,
    Opening {
        index: usize,
        result: SharedDisplayPlacementResult,
    },
    Waiting {
        index: usize,
        handle: quickgui::WindowHandle,
        result: SharedDisplayPlacementResult,
    },
    Reporting,
    Finished,
}

impl DisplayAcceptancePhase {
    const fn name(&self) -> &'static str {
        match self {
            Self::Discover => "discover",
            Self::Ready => "ready",
            Self::Opening { .. } => "opening",
            Self::Waiting { .. } => "waiting",
            Self::Reporting => "reporting",
            Self::Finished => "finished",
        }
    }

    const fn is_finished(&self) -> bool {
        matches!(self, Self::Finished)
    }
}

type SharedDisplayPlacementResult = Arc<Mutex<Option<DisplayPlacementResult>>>;

const fn capability_status(available: bool, tested: bool) -> &'static str {
    if !available {
        "skipped"
    } else if tested {
        "passed"
    } else {
        "failed"
    }
}

#[derive(Debug, Serialize)]
struct DisplayAcceptanceReport {
    schema_version: u32,
    display_count: usize,
    tested_displays: usize,
    secondary_display_available: bool,
    secondary_display_tested: bool,
    secondary_status: &'static str,
    mixed_scale_available: bool,
    mixed_scale_tested: bool,
    mixed_scale_status: &'static str,
    require_secondary: bool,
    require_mixed_scale: bool,
    settled_snapshot_native_match: bool,
    root_initial_centered_match: bool,
    root_settled_placement_match: bool,
    results: Vec<DisplayPlacementResult>,
    passed: bool,
    failures: Vec<String>,
}

impl View for DisplayBrowser {
    fn render(&mut self, cx: &mut ViewContext<'_, Self>) -> impl IntoElement {
        let current = cx
            .current_display()
            .map(|display| display.name().to_owned());
        let displays = cx.displays().to_vec();
        self.drive_acceptance(cx, &displays);
        let mut cards = Vec::with_capacity(displays.len());
        for display in displays {
            let id = display.id();
            let name: Arc<str> = Arc::from(display.name());
            let open = cx.listener(format!("open-display-{}", id.get()), move |_this, cx| {
                cx.open_window(
                    WindowOptions::new(format!("Window on display {}", id.get()))
                        .size(560.0, 360.0)
                        .display(id)
                        .background(Color::rgb8(20, 22, 27)),
                    TargetWindow::manual(id, name.clone()),
                );
            });
            cards.push(Self::display_card(display, open));
        }

        div()
            .size_full()
            .flex_col()
            .gap_4()
            .p_6()
            .overflow_y_scroll()
            .bg(Color::rgb8(18, 20, 25))
            .text_color(Color::rgb8(240, 242, 247))
            .child(text("Active displays").text_2xl().font_bold())
            .child(
                text(current.map_or_else(
                    || "This window is not attached to a known display".to_owned(),
                    |name| format!("This window is on {name}"),
                ))
                .text_sm()
                .text_color(Color::rgb8(148, 156, 174)),
            )
            .children(cards)
    }
}

struct TargetWindow {
    requested: DisplayId,
    requested_name: Arc<str>,
    acceptance: Option<TargetAcceptance>,
}

impl TargetWindow {
    fn manual(requested: DisplayId, requested_name: Arc<str>) -> Self {
        Self {
            requested,
            requested_name,
            acceptance: None,
        }
    }

    fn acceptance(requested: Display, title: String, result: SharedDisplayPlacementResult) -> Self {
        Self {
            requested: requested.id(),
            requested_name: Arc::from(requested.name()),
            acceptance: Some(TargetAcceptance {
                requested,
                title,
                result,
                settle_started: false,
                ready: false,
                close_scheduled: false,
                setup_failure: None,
                initial_placement: None,
            }),
        }
    }

    fn drive_acceptance(&mut self, cx: &mut ViewContext<'_, Self>) {
        let Some(acceptance) = self.acceptance.as_mut() else {
            return;
        };
        if acceptance.initial_placement.is_none() {
            let requested = cx
                .find_display(acceptance.requested.id())
                .cloned()
                .unwrap_or_else(|| acceptance.requested.clone());
            let state = cx.window_state();
            acceptance.initial_placement = Some(InitialPlacementObservation {
                expected: requested.centered_bounds(state.viewport_size),
                actual: state.bounds.bounds(),
                retained_display_matches: state.display_id == Some(requested.id()),
                scale_factor_matches: scale_approximately(
                    state.scale_factor,
                    requested.scale_factor(),
                ),
            });
        }
        if !acceptance.settle_started {
            acceptance.settle_started = true;
            match cx.spawn(|task_cx: AsyncViewContext<Self>| async move {
                task_cx.sleep(ACCEPTANCE_SETTLE_DURATION).await?;
                task_cx
                    .update(|this, cx| {
                        if let Some(acceptance) = this.acceptance.as_mut() {
                            acceptance.ready = true;
                            cx.invalidate();
                        }
                    })
                    .await?;
                Ok::<(), AsyncContextError>(())
            }) {
                Ok(task) => task.detach(),
                Err(error) => {
                    acceptance.setup_failure = Some(format!(
                        "could not schedule placement settle deadline: {error}"
                    ));
                    acceptance.ready = true;
                    cx.request_animation_frame();
                }
            }
        }

        let placement = if acceptance.ready && !acceptance.close_scheduled {
            let requested = cx
                .find_display(acceptance.requested.id())
                .cloned()
                .unwrap_or_else(|| acceptance.requested.clone());
            let current_display_id = cx.current_display().map(Display::id);
            let window_state = cx.window_state();
            let mut result = evaluate_display_placement(
                &requested,
                &acceptance.title,
                current_display_id,
                window_state,
                acceptance.initial_placement,
            );
            if let Some(failure) = acceptance.setup_failure.take() {
                result.failures.push(failure);
                result.passed = false;
            }
            Some(result)
        } else {
            None
        };

        if let Some(placement) = placement {
            acceptance.close_scheduled = true;
            if let Ok(mut result) = acceptance.result.lock() {
                *result = Some(placement);
            }
            match cx.spawn(|task_cx: AsyncViewContext<Self>| async move {
                task_cx.update(|_, cx| cx.close_window()).await
            }) {
                Ok(task) => task.detach(),
                Err(error) => {
                    if let Ok(mut result) = acceptance.result.lock()
                        && let Some(result) = result.as_mut()
                    {
                        result
                            .failures
                            .push(format!("could not schedule display child close: {error}"));
                        result.passed = false;
                    }
                }
            }
        }
    }
}

impl View for TargetWindow {
    fn render(&mut self, cx: &mut ViewContext<'_, Self>) -> impl IntoElement {
        self.drive_acceptance(cx);
        let actual = cx
            .current_display()
            .map(|display| format!("Mounted on {} (id {})", display.name(), display.id().get()));
        let bounds = cx.window_bounds().bounds();
        let close = cx.listener("close", |_this, cx: &mut EventContext| {
            cx.close_window();
        });
        div()
            .size_full()
            .flex_col()
            .items_center()
            .justify_center()
            .gap_3()
            .p_6()
            .bg(Color::rgb8(20, 22, 27))
            .text_color(Color::rgb8(240, 242, 247))
            .child(text(self.requested_name.clone()).text_2xl().font_bold())
            .child(
                text(format!("Requested display id {}", self.requested.get()))
                    .text_sm()
                    .text_color(Color::rgb8(148, 156, 174)),
            )
            .child(
                text(actual.unwrap_or_else(|| "Waiting for native placement".to_owned()))
                    .text_sm()
                    .text_color(Color::rgb8(94, 234, 212)),
            )
            .child(
                text(format!(
                    "Window bounds {:.0}×{:.0} at ({:.0}, {:.0})",
                    bounds.width, bounds.height, bounds.x, bounds.y,
                ))
                .text_sm()
                .text_color(Color::rgb8(148, 156, 174)),
            )
            .child(
                button()
                    .on_click(close)
                    .min_h(38.0)
                    .px_4()
                    .rounded_lg()
                    .bg(Color::rgb8(45, 49, 60))
                    .hover(|style| style.bg(Color::rgb8(58, 64, 78)))
                    .child("Close"),
            )
    }
}

struct TargetAcceptance {
    requested: Display,
    title: String,
    result: SharedDisplayPlacementResult,
    settle_started: bool,
    ready: bool,
    close_scheduled: bool,
    setup_failure: Option<String>,
    initial_placement: Option<InitialPlacementObservation>,
}

#[derive(Clone, Copy, Debug)]
struct InitialPlacementObservation {
    expected: Rect,
    actual: Rect,
    retained_display_matches: bool,
    scale_factor_matches: bool,
}

impl InitialPlacementObservation {
    fn matches(self) -> bool {
        self.retained_display_matches
            && self.scale_factor_matches
            && rect_approximately(self.actual, self.expected)
    }
}

#[derive(Debug, Serialize)]
struct DisplayPlacementResult {
    display_id: u64,
    display_name: String,
    primary: bool,
    scale_factor: f32,
    initial_expected_x: f32,
    initial_expected_y: f32,
    initial_actual_x: f32,
    initial_actual_y: f32,
    initial_actual_width: f32,
    initial_actual_height: f32,
    settled_actual_x: f32,
    settled_actual_y: f32,
    settled_actual_width: f32,
    settled_actual_height: f32,
    settled_retained_display_id: Option<u64>,
    settled_current_display_id: Option<u64>,
    native_display_id: Option<u64>,
    native_scale_factor: Option<f32>,
    native_frame_x: Option<f32>,
    native_frame_y: Option<f32>,
    native_frame_width: Option<f32>,
    native_frame_height: Option<f32>,
    initial_retained_display_matches: bool,
    initial_scale_factor_matches: bool,
    initial_centered_bounds_match: bool,
    settled_retained_display_matches: bool,
    settled_current_display_matches: bool,
    settled_scale_factor_matches: bool,
    native_display_matches: bool,
    native_scale_factor_matches: bool,
    native_frame_origin_matches: bool,
    native_screen_bounds_match: bool,
    native_visible_bounds_match: bool,
    settled_work_area_contains_native_frame: bool,
    remained_hidden: bool,
    remained_non_key: bool,
    passed: bool,
    failures: Vec<String>,
}

#[derive(Clone, Copy, Debug)]
struct NativeWindowObservation {
    display_id: u64,
    scale_factor: f32,
    frame: Rect,
    screen_bounds: Rect,
    visible_bounds: Rect,
    visible: bool,
    key: bool,
}

fn evaluate_display_placement(
    requested: &Display,
    title: &str,
    current_display_id: Option<DisplayId>,
    window_state: WindowState,
    initial_placement: Option<InitialPlacementObservation>,
) -> DisplayPlacementResult {
    let settled_actual = window_state.bounds.bounds();
    let initial_expected = initial_placement
        .map(|initial| initial.expected)
        .unwrap_or_else(|| requested.centered_bounds(ACCEPTANCE_WINDOW_SIZE));
    let initial_actual = initial_placement
        .map(|initial| initial.actual)
        .unwrap_or(settled_actual);
    let native = native_window_observation(title);
    let initial_retained_display_matches =
        initial_placement.is_some_and(|initial| initial.retained_display_matches);
    let initial_scale_factor_matches =
        initial_placement.is_some_and(|initial| initial.scale_factor_matches);
    let initial_centered_bounds_match =
        initial_placement.is_some_and(InitialPlacementObservation::matches);
    let settled_retained_display_matches = window_state.display_id == Some(requested.id());
    let settled_current_display_matches = current_display_id == Some(requested.id());
    let settled_scale_factor_matches =
        scale_approximately(window_state.scale_factor, requested.scale_factor());
    let native_display_matches = native
        .as_ref()
        .is_ok_and(|native| native.display_id == requested.id().get());
    let native_scale_factor_matches = native
        .as_ref()
        .is_ok_and(|native| scale_approximately(native.scale_factor, requested.scale_factor()));
    let native_frame_origin_matches = native
        .as_ref()
        .is_ok_and(|native| rect_origins_approximately(native.frame, settled_actual));
    let native_screen_bounds_match = native
        .as_ref()
        .is_ok_and(|native| rect_approximately(native.screen_bounds, requested.bounds()));
    let native_visible_bounds_match = native
        .as_ref()
        .is_ok_and(|native| rect_approximately(native.visible_bounds, requested.visible_bounds()));
    let settled_work_area_contains_native_frame = native.as_ref().is_ok_and(|native| {
        rect_contains_rect_with_tolerance(requested.visible_bounds(), native.frame)
    });
    let remained_hidden =
        !window_state.visible && native.as_ref().is_ok_and(|native| !native.visible);
    let remained_non_key = !window_state.focused && native.as_ref().is_ok_and(|native| !native.key);

    let mut failures = Vec::new();
    for (name, passed) in [
        ("initial_placement_recorded", initial_placement.is_some()),
        (
            "initial_retained_display_matches",
            initial_retained_display_matches,
        ),
        ("initial_scale_factor_matches", initial_scale_factor_matches),
        (
            "initial_centered_bounds_match",
            initial_centered_bounds_match,
        ),
        (
            "settled_retained_display_matches",
            settled_retained_display_matches,
        ),
        (
            "settled_current_display_matches",
            settled_current_display_matches,
        ),
        ("settled_scale_factor_matches", settled_scale_factor_matches),
        ("native_display_matches", native_display_matches),
        ("native_scale_factor_matches", native_scale_factor_matches),
        ("native_frame_origin_matches", native_frame_origin_matches),
        ("native_screen_bounds_match", native_screen_bounds_match),
        ("native_visible_bounds_match", native_visible_bounds_match),
        (
            "settled_work_area_contains_native_frame",
            settled_work_area_contains_native_frame,
        ),
        ("remained_hidden", remained_hidden),
        ("remained_non_key", remained_non_key),
    ] {
        if !passed {
            failures.push(format!("{name} was false"));
        }
    }
    if let Err(error) = &native {
        failures.push(format!("native observation failed: {error}"));
    }
    if !native_screen_bounds_match && let Ok(native) = &native {
        failures.push(format!(
            "native screen bounds {:?} did not match retained {:?}",
            native.screen_bounds,
            requested.bounds(),
        ));
    }
    if !native_visible_bounds_match && let Ok(native) = &native {
        failures.push(format!(
            "native visible bounds {:?} did not match retained {:?}",
            native.visible_bounds,
            requested.visible_bounds(),
        ));
    }
    let (
        native_display_id,
        native_scale_factor,
        native_frame_x,
        native_frame_y,
        native_frame_width,
        native_frame_height,
    ) = native
        .as_ref()
        .map_or((None, None, None, None, None, None), |native| {
            (
                Some(native.display_id),
                Some(native.scale_factor),
                Some(native.frame.x),
                Some(native.frame.y),
                Some(native.frame.width),
                Some(native.frame.height),
            )
        });

    DisplayPlacementResult {
        display_id: requested.id().get(),
        display_name: requested.name().to_owned(),
        primary: requested.is_primary(),
        scale_factor: requested.scale_factor(),
        initial_expected_x: initial_expected.x,
        initial_expected_y: initial_expected.y,
        initial_actual_x: initial_actual.x,
        initial_actual_y: initial_actual.y,
        initial_actual_width: initial_actual.width,
        initial_actual_height: initial_actual.height,
        settled_actual_x: settled_actual.x,
        settled_actual_y: settled_actual.y,
        settled_actual_width: settled_actual.width,
        settled_actual_height: settled_actual.height,
        settled_retained_display_id: window_state.display_id.map(DisplayId::get),
        settled_current_display_id: current_display_id.map(DisplayId::get),
        native_display_id,
        native_scale_factor,
        native_frame_x,
        native_frame_y,
        native_frame_width,
        native_frame_height,
        initial_retained_display_matches,
        initial_scale_factor_matches,
        initial_centered_bounds_match,
        settled_retained_display_matches,
        settled_current_display_matches,
        settled_scale_factor_matches,
        native_display_matches,
        native_scale_factor_matches,
        native_frame_origin_matches,
        native_screen_bounds_match,
        native_visible_bounds_match,
        settled_work_area_contains_native_frame,
        remained_hidden,
        remained_non_key,
        passed: failures.is_empty(),
        failures,
    }
}

fn approximately(actual: f32, expected: f32) -> bool {
    (actual - expected).abs() <= GEOMETRY_TOLERANCE
}

fn scale_approximately(actual: f32, expected: f32) -> bool {
    (actual - expected).abs() <= SCALE_TOLERANCE
}

fn rect_approximately(actual: Rect, expected: Rect) -> bool {
    approximately(actual.x, expected.x)
        && approximately(actual.y, expected.y)
        && approximately(actual.width, expected.width)
        && approximately(actual.height, expected.height)
}

fn rect_origins_approximately(actual: Rect, expected: Rect) -> bool {
    approximately(actual.x, expected.x) && approximately(actual.y, expected.y)
}

fn rect_contains_rect_with_tolerance(container: Rect, rect: Rect) -> bool {
    rect.x >= container.x - GEOMETRY_TOLERANCE
        && rect.y >= container.y - GEOMETRY_TOLERANCE
        && rect.x + rect.width <= container.x + container.width + GEOMETRY_TOLERANCE
        && rect.y + rect.height <= container.y + container.height + GEOMETRY_TOLERANCE
}

#[cfg(target_os = "macos")]
fn native_window_observation(title: &str) -> Result<NativeWindowObservation, String> {
    let mtm = MainThreadMarker::new()
        .ok_or_else(|| "display acceptance left the AppKit main thread".to_owned())?;
    let application = NSApplication::sharedApplication(mtm);
    let mut windows = Vec::new();
    for window in application.windows().iter_retained() {
        collect_window_tree(window, &mut windows);
    }
    let window = windows
        .iter()
        .find(|window| window.title().to_string() == title)
        .ok_or_else(|| format!("could not find AppKit window titled {title:?}"))?;
    let screen = window
        .screen()
        .ok_or_else(|| format!("AppKit window {title:?} was not attached to an NSScreen"))?;
    let screens = NSScreen::screens(mtm);
    let primary = screens
        .iter()
        .find(|screen| {
            let origin = screen.frame().origin;
            origin.x == 0.0 && origin.y == 0.0
        })
        .or_else(|| screens.iter().next())
        .ok_or_else(|| "AppKit reported no primary screen".to_owned())?;
    let primary_height = primary.frame().size.height as f32;
    Ok(NativeWindowObservation {
        display_id: u64::from(native_screen_id(&screen)),
        scale_factor: screen.backingScaleFactor() as f32,
        frame: top_left_screen_rect(window.frame(), primary_height),
        screen_bounds: top_left_screen_rect(screen.frame(), primary_height),
        visible_bounds: top_left_screen_rect(screen.visibleFrame(), primary_height),
        visible: window.isVisible(),
        key: window.isKeyWindow(),
    })
}

#[cfg(target_os = "macos")]
fn native_primary_display_id() -> Option<DisplayId> {
    let mtm = MainThreadMarker::new()?;
    let screens = NSScreen::screens(mtm);
    screens
        .iter()
        .find(|screen| {
            let origin = screen.frame().origin;
            origin.x == 0.0 && origin.y == 0.0
        })
        .or_else(|| screens.iter().next())
        .map(|screen| DisplayId::new(u64::from(native_screen_id(screen))))
}

#[cfg(target_os = "macos")]
fn native_display_snapshot_matches(displays: &[Display]) -> Result<(), String> {
    let mtm = MainThreadMarker::new()
        .ok_or_else(|| "display snapshot comparison left the AppKit main thread".to_owned())?;
    let screens = NSScreen::screens(mtm);
    let primary = screens
        .iter()
        .find(|screen| {
            let origin = screen.frame().origin;
            origin.x == 0.0 && origin.y == 0.0
        })
        .or_else(|| screens.iter().next())
        .ok_or_else(|| "AppKit reported no primary screen".to_owned())?;
    let primary_height = primary.frame().size.height as f32;
    if displays.len() != screens.len() {
        return Err(format!(
            "QuickGUI retained {} displays while AppKit reported {}",
            displays.len(),
            screens.len(),
        ));
    }
    for display in displays {
        let screen = screens
            .iter()
            .find(|screen| u64::from(native_screen_id(screen)) == display.id().get())
            .ok_or_else(|| format!("AppKit omitted display {}", display.id().get()))?;
        let native_bounds = top_left_screen_rect(screen.frame(), primary_height);
        let native_visible = top_left_screen_rect(screen.visibleFrame(), primary_height);
        let native_scale = screen.backingScaleFactor() as f32;
        if !rect_approximately(native_bounds, display.bounds())
            || !rect_approximately(native_visible, display.visible_bounds())
            || !scale_approximately(native_scale, display.scale_factor())
        {
            return Err(format!(
                "display {} retained bounds {:?}, visible {:?}, scale {:.3}; AppKit bounds {:?}, visible {:?}, scale {:.3}",
                display.id().get(),
                display.bounds(),
                display.visible_bounds(),
                display.scale_factor(),
                native_bounds,
                native_visible,
                native_scale,
            ));
        }
    }
    Ok(())
}

#[cfg(not(target_os = "macos"))]
fn native_window_observation(_title: &str) -> Result<NativeWindowObservation, String> {
    Err("native display acceptance currently requires macOS".to_owned())
}

#[cfg(not(target_os = "macos"))]
fn native_primary_display_id() -> Option<DisplayId> {
    None
}

#[cfg(not(target_os = "macos"))]
fn native_display_snapshot_matches(_displays: &[Display]) -> Result<(), String> {
    Err("native display acceptance currently requires macOS".to_owned())
}

#[cfg(target_os = "macos")]
fn native_screen_id(screen: &NSScreen) -> u32 {
    let key = ns_string!("NSScreenNumber");
    let description = screen.deviceDescription();
    let object = description
        .get(key)
        .expect("NSScreenNumber is required in an NSScreen device description");
    let object: *const AnyObject = object;
    let number: *const NSNumber = object.cast();
    // SAFETY: AppKit documents NSScreenNumber as an NSNumber containing CGDirectDisplayID.
    unsafe { &*number }.as_u32()
}

#[cfg(target_os = "macos")]
fn top_left_screen_rect(rect: NSRect, primary_height: f32) -> Rect {
    Rect::new(
        rect.origin.x as f32,
        primary_height - (rect.origin.y + rect.size.height) as f32,
        rect.size.width as f32,
        rect.size.height as f32,
    )
}

#[cfg(target_os = "macos")]
fn collect_window_tree(window: Retained<NSWindow>, windows: &mut Vec<Retained<NSWindow>>) {
    if windows
        .iter()
        .any(|known| std::ptr::eq(known.as_ref(), window.as_ref()))
    {
        return;
    }
    windows.push(window.clone());
    // SAFETY: The retained window and its AppKit-owned child list are read synchronously on the
    // application main thread; every child is retained before the recursive call.
    if let Some(children) = unsafe { window.childWindows() } {
        for child in children.iter_retained() {
            collect_window_tree(child, windows);
        }
    }
}

fn emit_line(line: &str) {
    let stdout = io::stdout();
    let mut stdout = stdout.lock();
    let _ = writeln!(stdout, "{line}");
    let _ = stdout.flush();
}
