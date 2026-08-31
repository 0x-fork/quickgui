use std::{path::PathBuf, time::Duration};

use quickgui::{
    Application, AsyncViewContext, Color, Element, Event, Global, IntoElement, PathPromptOptions,
    PlatformError, PlatformResponse, PromptButton, PromptLevel, SavePathOptions,
    SystemNotification, SystemNotificationAction, Task, TitleBarStyle, View, ViewContext,
    WindowOptions, button, div, text,
};

const NOTIFICATION_TAG: &str = "quickgui-platform-services";

struct ApplicationStatus(String);

impl Global for ApplicationStatus {}

fn main() -> Result<(), quickgui::AppError> {
    Application::new()
        .global(ApplicationStatus(
            "No application lifecycle callback received yet.".to_owned(),
        ))
        .on_open_urls(|urls, cx| {
            let urls = urls.iter().collect::<Vec<_>>().join(", ");
            cx.update_global::<ApplicationStatus, _>(|status| {
                status.0 = format!("macOS asked QuickGUI to open: {urls}");
            });
        })
        .on_reopen(|had_visible_windows, cx| {
            cx.update_global::<ApplicationStatus, _>(|status| {
                status.0 = format!(
                    "macOS requested reopen; visible windows before request: {had_visible_windows}"
                );
            });
            if !had_visible_windows {
                cx.open_window(platform_window_options(), PlatformServices::default());
            }
        })
        .on_system_wake(|cx| {
            cx.update_global::<ApplicationStatus, _>(|status| {
                status.0 = "The system woke from sleep.".to_owned();
            });
        })
        .on_system_notification_response(|response, cx| {
            let action = response.action_id.as_deref().unwrap_or("notification body");
            cx.update_global::<ApplicationStatus, _>(|status| {
                status.0 = format!("Notification '{}' activated: {action}", response.tag);
            });
        })
        .on_window_closed(|window, cx| {
            cx.update_global::<ApplicationStatus, _>(|status| {
                status.0 = format!(
                    "Window {window:?} closed; the macOS application remains ready for Dock reopen."
                );
            });
        })
        .run(|cx| {
            cx.open_window(platform_window_options(), PlatformServices::default());
        })
}

fn platform_window_options() -> WindowOptions {
    WindowOptions::new("QuickGUI — Platform services")
        .size(780.0, 620.0)
        .minimum_size(620.0, 500.0)
        .title_bar_style(TitleBarStyle::HiddenInset)
        .traffic_light_position(16.0, 14.0)
        .background(Color::rgb8(15, 17, 22))
}

struct PlatformServices {
    pending: bool,
    intercept_next_close: bool,
    response_task: Option<Task<()>>,
    notification_revision: usize,
    selected_path: Option<PathBuf>,
    status: String,
}

impl Default for PlatformServices {
    fn default() -> Self {
        Self {
            pending: false,
            intercept_next_close: true,
            response_task: None,
            notification_revision: 0,
            selected_path: None,
            status: "Ready. Native services do not run a polling frame.".to_owned(),
        }
    }
}

impl PlatformServices {
    fn control(label: &'static str) -> Element {
        button()
            .app_region_no_drag()
            .px(14.0)
            .py(9.0)
            .rounded(7.0)
            .bg(Color::rgb8(38, 43, 54))
            .hover(|style| style.bg(Color::rgb8(51, 58, 72)))
            .text_color(Color::rgb8(231, 235, 243))
            .child(text(label))
    }

    fn await_response<T, Apply>(
        &mut self,
        cx: &mut quickgui::EventContext,
        response: PlatformResponse<T>,
        apply: Apply,
    ) where
        T: 'static,
        Apply: FnOnce(&mut Self, Result<T, PlatformError>) + 'static,
    {
        if let Some(task) = self.response_task.take() {
            task.cancel();
        }
        self.pending = true;
        self.status = "Waiting for the native sheet…".to_owned();
        match cx.spawn(|async_cx: AsyncViewContext<Self>| async move {
            let result = response.await;
            let _ = async_cx
                .update(move |this, cx| {
                    this.pending = false;
                    apply(this, result);
                    cx.invalidate();
                })
                .await;
        }) {
            Ok(task) => self.response_task = Some(task),
            Err(error) => {
                self.pending = false;
                self.status = format!("Could not start foreground response task: {error}");
            }
        }
        cx.invalidate();
    }

    fn selected_label(&self) -> String {
        self.selected_path.as_ref().map_or_else(
            || "No selected path".to_owned(),
            |path| path.display().to_string(),
        )
    }
}

impl View for PlatformServices {
    fn event(&mut self, event: &Event, cx: &mut quickgui::EventContext) {
        if matches!(event, Event::CloseRequested) && self.intercept_next_close {
            self.intercept_next_close = false;
            self.status =
                "Intercepted CloseRequested once. The next Cmd-W will close the window.".to_owned();
            cx.prevent_close();
            cx.invalidate();
        }
    }

    fn render(&mut self, cx: &mut ViewContext<'_, Self>) -> impl IntoElement {
        let application_status = cx.watch_global::<ApplicationStatus, _>(|status| status.0.clone());
        let show_prompt = cx.listener("show-native-prompt", |this, cx| {
            if this.pending {
                return;
            }
            match cx.prompt(
                PromptLevel::Warning,
                "Save changes before closing?",
                Some("This is an AppKit NSAlert sheet. Return chooses Save and Escape chooses Cancel."),
                &[
                    PromptButton::ok("Save"),
                    PromptButton::new("Don't Save"),
                    PromptButton::cancel("Cancel"),
                ],
            ) {
                Ok(response) => this.await_response(cx, response, |this, result| {
                    this.status = match result {
                        Ok(0) => "Prompt result: Save".to_owned(),
                        Ok(1) => "Prompt result: Don't Save".to_owned(),
                        Ok(2) => "Prompt result: Cancel".to_owned(),
                        Ok(index) => format!("Prompt result: button {index}"),
                        Err(error) => format!("Prompt failed: {error}"),
                    };
                }),
                Err(error) => this.status = format!("Prompt rejected: {error}"),
            }
            cx.invalidate();
        });

        let auto_cancel = cx.listener("auto-cancel-prompt", |this, cx| {
            if this.pending {
                return;
            }
            match cx.prompt(
                PromptLevel::Info,
                "Cancellation ownership",
                Some("QuickGUI will drop the awaiting response after one exact second and dismiss this sheet."),
                &[PromptButton::cancel("Cancel now")],
            ) {
                Ok(response) => {
                    this.await_response(cx, response, |this, result| {
                        this.status = match result {
                            Ok(_) => "Auto-cancel prompt answered before its deadline".to_owned(),
                            Err(error) => format!("Auto-cancel prompt failed: {error}"),
                        };
                    });
                    match cx.spawn(|async_cx: AsyncViewContext<Self>| async move {
                        if async_cx.sleep(Duration::from_secs(1)).await.is_err() {
                            return;
                        }
                        let _ = async_cx
                            .update(|this, cx| {
                                if this.pending {
                                    if let Some(task) = this.response_task.take() {
                                        task.cancel();
                                    }
                                    this.pending = false;
                                    this.status =
                                        "Dropped the response and cancelled the native sheet"
                                            .to_owned();
                                    cx.invalidate();
                                }
                            })
                            .await;
                    }) {
                        Ok(task) => task.detach(),
                        Err(error) => {
                            this.status = format!("Could not schedule cancellation: {error}")
                        }
                    }
                }
                Err(error) => this.status = format!("Prompt rejected: {error}"),
            }
            cx.invalidate();
        });

        let open_files = cx.listener("open-files", |this, cx| {
            if this.pending {
                return;
            }
            let options = PathPromptOptions::new()
                .multiple(true)
                .prompt("Open")
                .directory("/tmp");
            match cx.prompt_for_paths(options) {
                Ok(response) => this.await_response(cx, response, |this, result| match result {
                    Ok(Some(paths)) => {
                        this.selected_path = paths.first().cloned();
                        this.status = format!("Selected {} path(s)", paths.len());
                    }
                    Ok(None) => this.status = "Open panel cancelled".to_owned(),
                    Err(error) => this.status = format!("Open panel failed: {error}"),
                }),
                Err(error) => this.status = format!("Open panel rejected: {error}"),
            }
            cx.invalidate();
        });

        let open_folder = cx.listener("open-folder", |this, cx| {
            if this.pending {
                return;
            }
            let options = PathPromptOptions::new()
                .files(false)
                .directories(true)
                .prompt("Choose")
                .directory("/tmp");
            match cx.prompt_for_paths(options) {
                Ok(response) => this.await_response(cx, response, |this, result| match result {
                    Ok(Some(paths)) => {
                        this.selected_path = paths.first().cloned();
                        this.status = "Selected a folder".to_owned();
                    }
                    Ok(None) => this.status = "Folder panel cancelled".to_owned(),
                    Err(error) => this.status = format!("Folder panel failed: {error}"),
                }),
                Err(error) => this.status = format!("Folder panel rejected: {error}"),
            }
            cx.invalidate();
        });

        let save_file = cx.listener("save-file", |this, cx| {
            if this.pending {
                return;
            }
            let options = SavePathOptions::new("/tmp")
                .suggested_name("quickgui-example.txt")
                .prompt("Save");
            match cx.prompt_for_new_path(options) {
                Ok(response) => this.await_response(cx, response, |this, result| match result {
                    Ok(Some(path)) => {
                        this.selected_path = Some(path);
                        this.status = "Selected a save destination".to_owned();
                    }
                    Ok(None) => this.status = "Save panel cancelled".to_owned(),
                    Err(error) => this.status = format!("Save panel failed: {error}"),
                }),
                Err(error) => this.status = format!("Save panel rejected: {error}"),
            }
            cx.invalidate();
        });

        let open_url = cx.listener("open-url", |this, cx| {
            this.status = match cx.open_url("https://github.com/egoist/quickgui") {
                Ok(()) => "Asked macOS to open the QuickGUI repository".to_owned(),
                Err(error) => format!("URL action rejected: {error}"),
            };
            cx.invalidate();
        });
        let open_selected = cx.listener("open-selected", |this, cx| {
            let Some(path) = this.selected_path.clone() else {
                this.status = "Select a path first".to_owned();
                cx.invalidate();
                return;
            };
            this.status = match cx.open_path(path) {
                Ok(()) => "Asked macOS to open the selected path".to_owned(),
                Err(error) => format!("Open-path action rejected: {error}"),
            };
            cx.invalidate();
        });
        let reveal_selected = cx.listener("reveal-selected", |this, cx| {
            let Some(path) = this.selected_path.clone() else {
                this.status = "Select a path first".to_owned();
                cx.invalidate();
                return;
            };
            this.status = match cx.reveal_path(path) {
                Ok(()) => "Asked Finder to reveal the selected path".to_owned(),
                Err(error) => format!("Reveal action rejected: {error}"),
            };
            cx.invalidate();
        });
        let show_notification = cx.listener("show-system-notification", |this, cx| {
            this.notification_revision += 1;
            let revision = this.notification_revision;
            let notification = SystemNotification::new(
                NOTIFICATION_TAG,
                format!("QuickGUI notification {revision}"),
                "Posting the same tag replaces the previous notification.",
            )
            .action(SystemNotificationAction::new("open", "Open"))
            .action(SystemNotificationAction::new("snooze", "Snooze"));
            this.status = match cx.show_system_notification(notification) {
                Ok(()) => format!(
                    "Queued notification revision {revision}; macOS may request permission first"
                ),
                Err(error) => format!("Notification rejected: {error}"),
            };
            cx.invalidate();
        });
        let dismiss_notification = cx.listener("dismiss-system-notification", |this, cx| {
            this.status = match cx.dismiss_system_notification(NOTIFICATION_TAG) {
                Ok(()) => "Dismissed the system notification".to_owned(),
                Err(error) => format!("Notification dismissal rejected: {error}"),
            };
            cx.invalidate();
        });

        div()
            .size_full()
            .flex_col()
            .bg(Color::rgb8(15, 17, 22))
            .text_color(Color::rgb8(229, 233, 241))
            .child(
                div()
                    .h(48.0)
                    .flex_none()
                    .app_region_drag()
                    .border(1.0, Color::rgb8(40, 44, 54))
                    .items_center()
                    .px(88.0)
                    .child(text("Platform services").font_semibold()),
            )
            .child(
                div()
                    .flex_1()
                    .flex_col()
                    .gap(18.0)
                    .px(30.0)
                    .py(26.0)
                    .child(
                        text("Native macOS services")
                            .text_2xl()
                            .font_semibold()
                            .line_height(32.0),
                    )
                    .child(
                        text("Prompts and file panels are window-owned AppKit sheets. Results wake one foreground future; idle windows stay asleep.")
                            .text_color(Color::rgb8(160, 169, 186))
                            .line_height(22.0),
                    )
                    .child(
                        div()
                            .flex_row()
                            .flex_wrap()
                            .gap(10.0)
                            .child(Self::control("Native prompt").disabled(self.pending).on_click(show_prompt))
                            .child(Self::control("Auto-cancel sheet").disabled(self.pending).on_click(auto_cancel))
                            .child(Self::control("Open files…").disabled(self.pending).on_click(open_files))
                            .child(Self::control("Choose folder…").disabled(self.pending).on_click(open_folder))
                            .child(Self::control("Save as…").disabled(self.pending).on_click(save_file)),
                    )
                    .child(
                        div()
                            .flex_row()
                            .flex_wrap()
                            .gap(10.0)
                            .child(Self::control("Open URL").on_click(open_url))
                            .child(Self::control("Open selected").disabled(self.selected_path.is_none()).on_click(open_selected))
                            .child(Self::control("Reveal in Finder").disabled(self.selected_path.is_none()).on_click(reveal_selected))
                            .child(Self::control("Post notification").on_click(show_notification))
                            .child(Self::control("Dismiss notification").on_click(dismiss_notification)),
                    )
                    .child(
                        div()
                            .flex_col()
                            .gap(8.0)
                            .rounded(9.0)
                            .border(1.0, Color::rgb8(45, 50, 62))
                            .bg(Color::rgb8(22, 25, 32))
                            .p_4()
                            .child(text("Status").font_semibold())
                            .child(text(self.status.clone()).line_height(21.0))
                            .child(
                                text(application_status)
                                    .text_sm()
                                    .text_color(Color::rgb8(177, 188, 208))
                                    .line_height(20.0),
                            )
                            .child(
                                text(self.selected_label())
                                    .text_sm()
                                    .text_color(Color::rgb8(145, 156, 176))
                                    .line_height(20.0),
                            ),
                    )
                    .child(
                        text("The first Cmd-W is intercepted through Event::CloseRequested; the second closes. This proves prevent_close() remains authoritative.")
                            .text_sm()
                            .text_color(Color::rgb8(126, 139, 160))
                            .line_height(20.0),
                    ),
            )
    }
}
