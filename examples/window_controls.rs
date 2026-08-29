use std::{sync::Arc, time::Duration};

use quickgui::{
    App, AppConfig, AsyncContextError, AsyncViewContext, Color, Element, Rect, Task, TitleBarStyle,
    View, ViewContext, WindowBounds, WindowCommandError, WindowHandle, WindowKind, WindowOptions,
    button, div, text,
};

fn main() -> Result<(), quickgui::AppError> {
    App::new(WindowLauncher::default())
        .config(
            AppConfig::new("QuickGUI — Window controls")
                .size(760.0, 650.0)
                .minimum_size(620.0, 520.0)
                .title_bar_style(TitleBarStyle::HiddenInset)
                .traffic_light_position(16.0, 14.0)
                .background(Color::rgb8(15, 17, 22)),
        )
        .run()
}

#[derive(Default)]
struct WindowLauncher {
    opened: u32,
    last_window: Option<WindowHandle>,
    status: String,
}

impl WindowLauncher {
    fn open(&mut self, kind: WindowKind, delayed_show: bool, cx: &mut quickgui::EventContext) {
        self.opened = self.opened.saturating_add(1);
        let number = self.opened;
        let label = format!("{} #{number}", kind_label(kind));
        let mut options = WindowOptions::new(label.clone())
            .size(660.0, 520.0)
            .minimum_size(520.0, 400.0)
            .window_kind(kind)
            .show(!delayed_show)
            .background(Color::rgb8(17, 20, 26));
        if matches!(kind, WindowKind::Floating | WindowKind::Popover) {
            options = options.title_bar_style(TitleBarStyle::Hidden);
        }
        let handle = cx.open_window(
            ControlledWindow::new(label.clone(), kind, delayed_show),
            options,
        );
        self.last_window = Some(handle);
        self.status = if delayed_show {
            format!("Opened {label} hidden; its exact timer will reveal it after 900 ms")
        } else {
            format!("Opened {label}")
        };
        cx.invalidate();
    }

    fn target(
        &mut self,
        action: &str,
        apply: impl FnOnce(&mut quickgui::EventContext, WindowHandle) -> Result<(), WindowCommandError>,
        cx: &mut quickgui::EventContext,
    ) {
        let Some(handle) = self.last_window else {
            self.status = "Open a child window first".to_owned();
            cx.invalidate();
            return;
        };
        self.status = match apply(cx, handle) {
            Ok(()) => format!("Requested {action} for the latest window"),
            Err(error) => format!("Could not {action}: {error}"),
        };
        cx.invalidate();
    }

    fn control(label: impl Into<Arc<str>>) -> Element {
        control(label)
    }
}

impl View for WindowLauncher {
    fn render(&mut self, cx: &mut ViewContext<'_, Self>) -> impl quickgui::IntoElement {
        let state = cx.window_state();
        let open_normal = cx.listener("open-normal", |this, cx| {
            this.open(WindowKind::Normal, false, cx)
        });
        let open_floating = cx.listener("open-floating", |this, cx| {
            this.open(WindowKind::Floating, false, cx)
        });
        let open_popover = cx.listener("open-popover", |this, cx| {
            this.open(WindowKind::Popover, false, cx)
        });
        let open_dialog = cx.listener("open-dialog", |this, cx| {
            this.open(WindowKind::Dialog, false, cx)
        });
        let open_hidden = cx.listener("open-hidden", |this, cx| {
            this.open(WindowKind::Normal, true, cx)
        });
        let focus_last = cx.listener("focus-last", |this, cx| {
            let Some(handle) = this.last_window else {
                this.status = "Open a child window first".to_owned();
                cx.invalidate();
                return;
            };
            cx.focus_window(handle);
            this.status = "Focused the latest window".to_owned();
            cx.invalidate();
        });
        let restore_last = cx.listener("restore-last", |this, cx| {
            this.target("restore", |cx, handle| cx.restore_window_handle(handle), cx)
        });
        let show_last = cx.listener("show-last", |this, cx| {
            this.target("show", |cx, handle| cx.show_window_handle(handle), cx)
        });
        let close_last = cx.listener("close-last", |this, cx| {
            if let Some(handle) = this.last_window.take() {
                cx.close_window_handle(handle);
                this.status = "Closed the latest window and its descendants".to_owned();
            } else {
                this.status = "Open a child window first".to_owned();
            }
            cx.invalidate();
        });
        let bounds = state.bounds.bounds();

        div()
            .size_full()
            .flex_col()
            .bg(Color::rgb8(15, 17, 22))
            .text_color(Color::rgb8(234, 238, 245))
            .child(
                div()
                    .h(62.0)
                    .w_full()
                    .flex_none()
                    .flex_row()
                    .items_center()
                    .justify_between()
                    .padding(0.0, 18.0, 0.0, 88.0)
                    .bg(Color::rgb8(29, 34, 44))
                    .border(1.0, Color::rgb8(53, 61, 76))
                    .app_region_drag()
                    .child(
                        div()
                            .flex_col()
                            .child(text("Native window controls").text_lg().font_bold())
                            .child(
                                text("macOS roles, state snapshots, and bounded commands")
                                    .text_xs()
                                    .text_color(Color::rgb8(148, 158, 177)),
                            ),
                    )
                    .child(
                        text(format!("frame {}", cx.metrics().frame_number))
                            .text_xs()
                            .text_color(Color::rgb8(148, 163, 184)),
                    ),
            )
            .child(
                div()
                    .flex_1()
                    .min_h(0.0)
                    .overflow_y_scroll()
                    .p_5()
                    .flex_col()
                    .gap_4()
                    .child(
                        card()
                            .child(text("Open a parent-owned window").font_semibold())
                            .child(
                                text("Dialog becomes an AppKit sheet. Floating and Popover use elevated native levels; the hidden window proves first-frame preparation and one-shot timer wakeup.")
                                    .wrap()
                                    .text_sm()
                                    .text_color(Color::rgb8(157, 167, 185)),
                            )
                            .child(
                                div()
                                    .flex_row()
                                    .flex_wrap()
                                    .gap_2()
                                    .child(Self::control("Normal").on_click(open_normal))
                                    .child(Self::control("Floating").on_click(open_floating))
                                    .child(Self::control("Popover").on_click(open_popover))
                                    .child(Self::control("Dialog sheet").on_click(open_dialog))
                                    .child(Self::control("Hidden → show").on_click(open_hidden)),
                            ),
                    )
                    .child(
                        card()
                            .child(text("Latest child").font_semibold())
                            .child(
                                div()
                                    .flex_row()
                                    .flex_wrap()
                                    .gap_2()
                                    .child(Self::control("Focus").on_click(focus_last))
                                    .child(Self::control("Restore").on_click(restore_last))
                                    .child(Self::control("Show").on_click(show_last))
                                    .child(Self::control("Close tree").on_click(close_last)),
                            )
                            .child(
                                text(if self.status.is_empty() {
                                    "No child window opened yet".to_owned()
                                } else {
                                    self.status.clone()
                                })
                                .wrap()
                                .text_sm()
                                .text_color(Color::rgb8(125, 211, 252)),
                            ),
                    )
                    .child(
                        card()
                            .child(text("Launcher state snapshot").font_semibold())
                            .child(state_text(state, bounds)),
                    )
                    .child(
                        text("Idle invariant: the frame counter does not advance unless an OS event, exact deadline, or application invalidation requests a frame.")
                            .wrap()
                            .text_xs()
                            .text_color(Color::rgb8(128, 138, 158)),
                    ),
            )
    }
}

struct ControlledWindow {
    label: String,
    kind: WindowKind,
    alternate_title: bool,
    movable: bool,
    resizable: bool,
    minimizable: bool,
    delayed_show: bool,
    task: Option<Task<Result<(), AsyncContextError>>>,
    status: String,
}

impl ControlledWindow {
    fn new(label: String, kind: WindowKind, delayed_show: bool) -> Self {
        Self {
            label,
            kind,
            alternate_title: false,
            movable: true,
            resizable: true,
            minimizable: true,
            delayed_show,
            task: None,
            status: if delayed_show {
                "Prepared offscreen; waiting for one exact timer".to_owned()
            } else {
                "Ready".to_owned()
            },
        }
    }

    fn schedule_initial_show(&mut self, cx: &ViewContext<'_, Self>) {
        if !self.delayed_show || self.task.is_some() {
            return;
        }
        match cx.spawn(|task_cx: AsyncViewContext<Self>| async move {
            task_cx.sleep(Duration::from_millis(900)).await?;
            task_cx
                .update(|this, cx| {
                    this.delayed_show = false;
                    this.status = match cx.show_window() {
                        Ok(()) => "Shown after one exact 900 ms wakeup".to_owned(),
                        Err(error) => format!("Could not show: {error}"),
                    };
                    cx.invalidate();
                })
                .await?;
            Ok(())
        }) {
            Ok(task) => self.task = Some(task),
            Err(error) => {
                self.delayed_show = false;
                self.status = format!("Could not schedule delayed show: {error}");
            }
        }
    }

    fn hide_temporarily(&mut self, cx: &mut quickgui::EventContext) {
        if let Some(task) = self.task.take() {
            task.cancel();
        }
        if let Err(error) = cx.hide_window() {
            self.status = format!("Could not hide: {error}");
            cx.invalidate();
            return;
        }
        self.status = "Hidden for 1 second without polling".to_owned();
        match cx.spawn(|task_cx: AsyncViewContext<Self>| async move {
            task_cx.sleep(Duration::from_secs(1)).await?;
            task_cx
                .update(|this, cx| {
                    this.status = match cx.show_window() {
                        Ok(()) => "Shown after one exact 1 second wakeup".to_owned(),
                        Err(error) => format!("Could not show: {error}"),
                    };
                    cx.invalidate();
                })
                .await?;
            Ok(())
        }) {
            Ok(task) => self.task = Some(task),
            Err(error) => self.status = format!("Could not schedule show: {error}"),
        }
        cx.invalidate();
    }

    fn apply(
        &mut self,
        action: &str,
        result: Result<(), WindowCommandError>,
        cx: &mut quickgui::EventContext,
    ) {
        self.status = match result {
            Ok(()) => format!("Requested {action}"),
            Err(error) => format!("Could not {action}: {error}"),
        };
        cx.invalidate();
    }
}

impl View for ControlledWindow {
    fn render(&mut self, cx: &mut ViewContext<'_, Self>) -> impl quickgui::IntoElement {
        self.schedule_initial_show(cx);
        let state = cx.window_state();
        let current = state.bounds.bounds();
        let next_bounds =
            WindowBounds::Windowed(Rect::new(current.x + 24.0, current.y + 24.0, 680.0, 480.0));
        let rename = cx.listener("rename", |this, cx| {
            this.alternate_title = !this.alternate_title;
            let title = if this.alternate_title {
                format!("{} — renamed", this.label)
            } else {
                this.label.clone()
            };
            let result = cx.set_window_title(title);
            this.apply("title change", result, cx);
        });
        let set_bounds = cx.listener("set-bounds", move |this, cx| {
            let result = cx.set_window_bounds(next_bounds);
            this.apply("windowed bounds", result, cx);
        });
        let zoom = cx.listener("zoom", |this, cx| {
            let result = cx.zoom_window();
            this.apply("zoom toggle", result, cx);
        });
        let fullscreen = cx.listener("fullscreen", |this, cx| {
            let result = cx.toggle_fullscreen();
            this.apply("fullscreen toggle", result, cx);
        });
        let minimize = cx.listener("minimize", |this, cx| {
            let result = cx.minimize_window();
            this.apply("minimize", result, cx);
        });
        let movable = cx.listener("movable", |this, cx| {
            this.movable = !this.movable;
            let result = cx.set_window_movable(this.movable);
            this.apply("movability change", result, cx);
        });
        let resizable = cx.listener("resizable", |this, cx| {
            this.resizable = !this.resizable;
            let result = cx.set_window_resizable(this.resizable);
            this.apply("resizability change", result, cx);
        });
        let minimizable = cx.listener("minimizable", |this, cx| {
            this.minimizable = !this.minimizable;
            let result = cx.set_window_minimizable(this.minimizable);
            this.apply("minimizability change", result, cx);
        });
        let hide = cx.listener("hide", |this, cx| this.hide_temporarily(cx));
        let attention = cx.listener("attention", |this, cx| {
            let result = cx.request_window_attention();
            this.apply("attention", result, cx);
        });
        let close = cx.listener("close", |_this, cx| cx.close_window());
        let title = self.label.clone();
        let kind = kind_label(self.kind);

        div()
            .size_full()
            .flex_col()
            .bg(Color::rgb8(17, 20, 26))
            .text_color(Color::rgb8(234, 238, 245))
            .child(
                div()
                    .h(58.0)
                    .w_full()
                    .flex_none()
                    .flex_row()
                    .items_center()
                    .justify_between()
                    .px_4()
                    .bg(Color::rgb8(30, 36, 47))
                    .border(1.0, Color::rgb8(57, 67, 83))
                    .app_region_drag()
                    .child(
                        div()
                            .flex_col()
                            .child(text(title).font_bold())
                            .child(
                                text(kind)
                                    .text_xs()
                                    .text_color(Color::rgb8(148, 163, 184)),
                            ),
                    )
                    .child(control("Close").on_click(close)),
            )
            .child(
                div()
                    .flex_1()
                    .min_h(0.0)
                    .overflow_y_scroll()
                    .p_5()
                    .flex_col()
                    .gap_4()
                    .child(card().child(text("Live state").font_semibold()).child(state_text(
                        state, current,
                    )))
                    .child(
                        card()
                            .child(text("Geometry and native state").font_semibold())
                            .child(
                                div()
                                    .flex_row()
                                    .flex_wrap()
                                    .gap_2()
                                    .child(control("Rename").on_click(rename))
                                    .child(control("Move + resize").on_click(set_bounds))
                                    .child(control("Zoom").on_click(zoom))
                                    .child(control("Fullscreen").on_click(fullscreen))
                                    .child(control("Minimize").on_click(minimize))
                                    .child(control("Attention").on_click(attention)),
                            ),
                    )
                    .child(
                        card()
                            .child(text("Runtime capabilities").font_semibold())
                            .child(
                                div()
                                    .flex_row()
                                    .flex_wrap()
                                    .gap_2()
                                    .child(control(format!("Movable: {}", self.movable)).on_click(movable))
                                    .child(control(format!("Resizable: {}", self.resizable)).on_click(resizable))
                                    .child(control(format!("Minimizable: {}", self.minimizable)).on_click(minimizable))
                                    .child(control("Hide for 1 s").on_click(hide)),
                            )
                            .child(
                                text(self.status.clone())
                                    .wrap()
                                    .text_sm()
                                    .text_color(Color::rgb8(125, 211, 252)),
                            ),
                    )
                    .child(
                        text(format!(
                            "Last rendered frame: {}. Window-state observation rebuilds only on native changes; there is no idle sampling loop.",
                            cx.metrics().frame_number
                        ))
                        .wrap()
                        .text_xs()
                        .text_color(Color::rgb8(128, 138, 158)),
                    ),
            )
    }
}

fn kind_label(kind: WindowKind) -> &'static str {
    match kind {
        WindowKind::Normal => "Normal",
        WindowKind::Popover => "Popover",
        WindowKind::SystemPopover => "SystemPopover",
        WindowKind::Floating => "Floating",
        WindowKind::Dialog => "Dialog",
    }
}

fn state_text(state: quickgui::WindowState, bounds: Rect) -> Element {
    text(format!(
        "role={} · bounds=({:.0}, {:.0}) {:.0}×{:.0} · viewport={:.0}×{:.0} @ {:.2}x\nfocused={} · visible={} · minimized={} · maximized={} · fullscreen={} · occluded={}\nmovable={} · resizable={} · minimizable={}",
        kind_label(state.kind),
        bounds.x,
        bounds.y,
        bounds.width,
        bounds.height,
        state.viewport_size.width,
        state.viewport_size.height,
        state.scale_factor,
        state.focused,
        state.visible,
        state.minimized,
        state.maximized,
        state.fullscreen,
        state.occluded,
        state.movable,
        state.resizable,
        state.minimizable,
    ))
    .w_full()
    .wrap()
    .text_sm()
    .line_height(20.0)
    .text_color(Color::rgb8(181, 190, 207))
}

fn card() -> Element {
    div()
        .w_full()
        .p_4()
        .flex_col()
        .gap_3()
        .rounded_xl()
        .border(1.0, Color::rgb8(54, 63, 78))
        .bg(Color::rgb8(24, 28, 36))
}

fn control(label: impl Into<Arc<str>>) -> Element {
    button()
        .app_region_no_drag()
        .min_h(38.0)
        .px_3()
        .py_2()
        .flex_row()
        .items_center()
        .justify_center()
        .rounded_lg()
        .border(1.0, Color::rgb8(73, 84, 103))
        .bg(Color::rgb8(37, 43, 54))
        .hover(|style| style.bg(Color::rgb8(50, 59, 73)))
        .active(|style| style.bg(Color::rgb8(31, 81, 124)))
        .child(text(label).text_sm().font_medium())
}
