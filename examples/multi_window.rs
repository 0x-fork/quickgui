use std::sync::Arc;

use quickgui::{
    Application, Color, Drag, DragOrigin, Element, Entity, Event, EventContext, EventEmitter,
    Global, Subscription, View, ViewContext, WindowHandle, WindowOptions, button, div, text,
};

fn main() -> Result<(), quickgui::AppError> {
    Application::new()
        .global(AppAppearance::default())
        .run(|cx| {
            cx.open_window(
                WindowOptions::new("QuickGUI — Multi-window").size(700.0, 620.0),
                Launcher::default(),
            );
        })
}

#[derive(Default)]
struct AppAppearance {
    warm: bool,
    revision: usize,
    last_writer: String,
}

impl Global for AppAppearance {}

#[derive(Default)]
struct WorkspaceState {
    revision: usize,
    last_writer: String,
}

struct WorkspacePublished {
    revision: usize,
    writer: String,
}

impl EventEmitter<WorkspacePublished> for WorkspaceState {}

#[derive(Clone)]
struct WindowCard {
    label: Arc<str>,
    color: Color,
}

fn transfer_card<V>(
    payload: &WindowCard,
    listener: quickgui::DragListener<V, WindowCard>,
) -> Element {
    div()
        .on_drag(listener)
        .min_h(62.0)
        .flex_row()
        .items_center()
        .justify_center()
        .px_3()
        .rounded_xl()
        .border(1.0, payload.color)
        .bg(Color::rgb8(32, 36, 45))
        .hover(|style| style.bg(Color::rgb8(44, 49, 61)))
        .dragging(|style| {
            style
                .bg(Color::rgb8(25, 28, 35))
                .border(2.0, Color::rgb8(148, 163, 184))
        })
        .child(
            text(payload.label.clone())
                .font_semibold()
                .text_color(payload.color),
        )
}

fn transfer_preview(payload: &WindowCard) -> Element {
    div()
        .w(190.0)
        .h(62.0)
        .flex_row()
        .items_center()
        .justify_center()
        .px_3()
        .rounded_xl()
        .border(2.0, payload.color)
        .bg(Color::rgba8(27, 31, 39, 235))
        .shadow_lg()
        .child(
            text(payload.label.clone())
                .font_semibold()
                .text_color(payload.color),
        )
}

fn transfer_origin(origin: DragOrigin) -> String {
    match origin {
        DragOrigin::Internal(source) => format!("the same window ({source:?})"),
        DragOrigin::CrossWindow { window, source } => {
            format!("QuickGUI window {window:?} ({source:?})")
        }
        DragOrigin::External => "another application".to_owned(),
    }
}

fn transfer_target<V>(
    listener: quickgui::DropListener<V, WindowCard>,
    received: Option<&WindowCard>,
) -> Element {
    let (label, color) = received.map_or_else(
        || ("Drop a card here".to_owned(), Color::rgb8(148, 163, 184)),
        |payload| (format!("Received: {}", payload.label), payload.color),
    );
    div()
        .on_drop(listener)
        .min_h(76.0)
        .flex_row()
        .items_center()
        .justify_center()
        .px_3()
        .rounded_xl()
        .border(1.0, Color::rgb8(72, 79, 94))
        .bg(Color::rgb8(25, 28, 35))
        .drag_over(|style| {
            style
                .border(2.0, Color::rgb8(94, 234, 212))
                .bg(Color::rgb8(25, 55, 58))
        })
        .child(text(label).text_sm().font_medium().text_color(color))
}

struct Launcher {
    opened: usize,
    last_inspector: Option<WindowHandle>,
    received: Option<WindowCard>,
    transfer_status: String,
    workspace: Entity<WorkspaceState>,
    workspace_subscription: Option<Subscription>,
    workspace_event_status: String,
    appearance_subscription: Option<Subscription>,
    appearance_event_status: String,
}

impl Default for Launcher {
    fn default() -> Self {
        Self {
            opened: 0,
            last_inspector: None,
            received: None,
            transfer_status: String::new(),
            workspace: Entity::new(WorkspaceState::default()),
            workspace_subscription: None,
            workspace_event_status: String::new(),
            appearance_subscription: None,
            appearance_event_status: String::new(),
        }
    }
}

impl Launcher {
    fn control(label: impl Into<Arc<str>>) -> Element {
        button()
            .min_h(40.0)
            .flex_row()
            .items_center()
            .justify_center()
            .px_4()
            .py_2()
            .rounded_lg()
            .border(1.0, Color::rgb8(76, 82, 96))
            .bg(Color::rgb8(38, 42, 51))
            .hover(|style| style.bg(Color::rgb8(51, 57, 69)))
            .active(|style| style.bg(Color::rgb8(31, 82, 126)))
            .focus(|style| style.border(2.0, Color::rgb8(94, 234, 212)))
            .child(text(label).font_medium())
    }
}

impl View for Launcher {
    fn render(&mut self, cx: &mut ViewContext<'_, Self>) -> impl quickgui::IntoElement {
        if self.workspace_subscription.is_none() {
            self.workspace_subscription = Some(cx.subscribe(
                &self.workspace,
                |this, _workspace, event: &WorkspacePublished, cx| {
                    this.workspace_event_status = format!(
                        "Typed event received: revision {} from {}",
                        event.revision, event.writer
                    );
                    cx.invalidate();
                },
            ));
        }
        if self.appearance_subscription.is_none() {
            self.appearance_subscription = Some(cx.observe_global::<AppAppearance>(|this, cx| {
                let (revision, writer) = {
                    let appearance = cx.global::<AppAppearance>();
                    (appearance.revision, appearance.last_writer.clone())
                };
                this.appearance_event_status =
                    format!("Global observer: revision {revision} from {writer}");
                cx.invalidate();
            }));
        }
        let (appearance_warm, appearance_revision, appearance_writer) = cx
            .watch_global::<AppAppearance, _>(|appearance| {
                (
                    appearance.warm,
                    appearance.revision,
                    appearance.last_writer.clone(),
                )
            });
        let (workspace_revision, workspace_writer) = cx.observe(&self.workspace, |workspace| {
            (workspace.revision, workspace.last_writer.clone())
        });
        let open = cx.listener("open-inspector", |this, cx| {
            this.opened += 1;
            let number = this.opened;
            let handle = cx.open_window(
                WindowOptions::new(format!("Inspector {number}"))
                    .size(560.0, 500.0)
                    .minimum_size(400.0, 340.0)
                    .background(Color::rgb8(20, 22, 27)),
                Inspector::new(number, this.workspace.clone()),
            );
            this.last_inspector = Some(handle);
            cx.invalidate();
        });
        let focus = cx.listener("focus-inspector", |this, cx| {
            if let Some(handle) = this.last_inspector {
                cx.focus_window(handle);
            }
        });
        let close = cx.listener("close-inspector", |this, cx| {
            if let Some(handle) = this.last_inspector.take() {
                cx.close_window_handle(handle);
                cx.invalidate();
            }
        });
        let publish = cx.listener("publish-shared-update", |this, cx| {
            let event = this.workspace.update(cx, |workspace, _cx| {
                workspace.revision = workspace.revision.saturating_add(1);
                workspace.last_writer = "Launcher".to_owned();
                WorkspacePublished {
                    revision: workspace.revision,
                    writer: workspace.last_writer.clone(),
                }
            });
            if !this.workspace.emit(cx, event) {
                this.workspace_event_status = "Typed event queue is full".to_owned();
                cx.invalidate();
            }
        });
        let toggle_appearance = cx.listener("toggle-global-appearance", |_this, cx| {
            cx.update_global::<AppAppearance, _>(|appearance| {
                appearance.warm = !appearance.warm;
                appearance.revision = appearance.revision.saturating_add(1);
                appearance.last_writer = "Launcher".to_owned();
            });
        });
        let accent = if appearance_warm {
            Color::rgb8(251, 191, 36)
        } else {
            Color::rgb8(94, 234, 212)
        };
        let payload = WindowCard {
            label: Arc::from("Launcher state card"),
            color: Color::rgb8(96, 165, 250),
        };
        let drag_payload = payload.clone();
        let drag = cx.drag_listener("launcher-transfer-source", move |_this, _event, _cx| {
            Drag::new(drag_payload.clone()).preview(transfer_preview(&drag_payload))
        });
        let drop = cx.drop_listener(
            "launcher-transfer-target",
            |this, payload: &WindowCard, event, cx| {
                this.received = Some(payload.clone());
                this.transfer_status = format!(
                    "'{}' arrived from {}",
                    payload.label,
                    transfer_origin(event.origin)
                );
                cx.invalidate();
            },
        );

        div()
            .size_full()
            .flex_col()
            .gap_4()
            .p_4()
            .bg(if appearance_warm {
                Color::rgb8(28, 23, 18)
            } else {
                Color::rgb8(18, 19, 23)
            })
            .text_color(Color::rgb8(234, 236, 241))
            .child(text("Independent retained windows").text_2xl().font_bold())
            .child(
                text("Each native window owns its view type, UI tree, frame scheduler, input state, renderer, and bounded caches. Closing an inspector leaves this launcher running.")
                    .text_sm()
                    .text_color(Color::rgb8(159, 166, 180)),
            )
            .child(
                div()
                    .flex_row()
                    .gap_2()
                    .child(Self::control("Open inspector").on_click(open))
                    .child(Self::control("Focus latest").on_click(focus))
                    .child(Self::control("Close latest").on_click(close))
                    .child(Self::control("Toggle global accent").on_click(toggle_appearance)),
            )
            .child(
                div()
                    .flex_1()
                    .flex_col()
                    .gap_2()
                    .p_4()
                    .rounded_xl()
                    .border(1.0, Color::rgb8(55, 60, 71))
                    .bg(Color::rgb8(25, 27, 33))
                    .child(text(format!("Inspectors opened: {}", self.opened)).text_lg())
                    .child(
                        text(match self.last_inspector {
                            Some(handle) => format!(
                                "Latest stable handle: {handle:?} (closed handles are safe no-ops)"
                            ),
                            None => "No inspector has been opened".to_owned(),
                        })
                        .text_sm()
                        .text_color(Color::rgb8(126, 231, 212)),
                    )
                    .child(
                        text("Try scrolling or resizing both windows: clean windows sleep independently and do not receive another window's redraws.")
                            .text_sm()
                            .text_color(Color::rgb8(159, 166, 180)),
                    )
                    .child(
                        text(if appearance_writer.is_empty() {
                            "Application global has not changed".to_owned()
                        } else {
                            format!(
                                "Global appearance revision {appearance_revision} from {appearance_writer}"
                            )
                        })
                        .text_sm()
                        .text_color(accent),
                    )
                    .child(
                        text(if self.appearance_event_status.is_empty() {
                            "No global observer callback received"
                        } else {
                            self.appearance_event_status.as_str()
                        })
                        .text_sm()
                        .text_color(accent),
                    )
                    .child(
                        div()
                            .flex_row()
                            .items_center()
                            .gap_3()
                            .child(Self::control("Publish shared update").on_click(publish))
                            .child(
                                text(if workspace_writer.is_empty() {
                                    "Shared entity has not changed".to_owned()
                                } else {
                                    format!(
                                        "Shared revision {workspace_revision} from {workspace_writer}"
                                    )
                                })
                                .text_sm()
                                .text_color(Color::rgb8(126, 231, 212)),
                            ),
                    )
                    .child(
                        text(if self.workspace_event_status.is_empty() {
                            "No typed workspace event received"
                        } else {
                            self.workspace_event_status.as_str()
                        })
                        .text_sm()
                        .text_color(Color::rgb8(167, 139, 250)),
                    )
                    .child(
                        text("Open an inspector, then drag either card across windows. The arbitrary Rust value stays shared in-process; only an opaque token enters AppKit.")
                            .text_sm()
                            .text_color(Color::rgb8(159, 166, 180)),
                    )
                    .child(
                        div()
                            .flex_row()
                            .gap_3()
                            .child(div().flex_1().child(transfer_card(&payload, drag)))
                            .child(
                                div()
                                    .flex_1()
                                    .child(transfer_target(drop, self.received.as_ref())),
                            ),
                    )
                    .child(
                        text(if self.transfer_status.is_empty() {
                            "No typed cross-window transfer yet"
                        } else {
                            self.transfer_status.as_str()
                        })
                        .text_sm()
                        .text_color(Color::rgb8(126, 231, 212)),
                    ),
            )
    }
}

struct Inspector {
    number: usize,
    protected: bool,
    blocked_close: bool,
    received: Option<WindowCard>,
    transfer_status: String,
    workspace: Entity<WorkspaceState>,
    workspace_subscription: Option<Subscription>,
    workspace_event_status: String,
    appearance_subscription: Option<Subscription>,
    appearance_event_status: String,
}

impl Inspector {
    fn new(number: usize, workspace: Entity<WorkspaceState>) -> Self {
        Self {
            number,
            protected: false,
            blocked_close: false,
            received: None,
            transfer_status: String::new(),
            workspace,
            workspace_subscription: None,
            workspace_event_status: String::new(),
            appearance_subscription: None,
            appearance_event_status: String::new(),
        }
    }
}

impl View for Inspector {
    fn event(&mut self, event: &Event, cx: &mut EventContext) {
        if matches!(event, Event::CloseRequested) && self.protected {
            self.blocked_close = true;
            cx.prevent_close();
            cx.invalidate();
        }
    }

    fn render(&mut self, cx: &mut ViewContext<'_, Self>) -> impl quickgui::IntoElement {
        if self.workspace_subscription.is_none() {
            self.workspace_subscription = Some(cx.subscribe(
                &self.workspace,
                |this, _workspace, event: &WorkspacePublished, cx| {
                    this.workspace_event_status = format!(
                        "Typed event received: revision {} from {}",
                        event.revision, event.writer
                    );
                    cx.invalidate();
                },
            ));
        }
        if self.appearance_subscription.is_none() {
            self.appearance_subscription = Some(cx.observe_global::<AppAppearance>(|this, cx| {
                let (revision, writer) = {
                    let appearance = cx.global::<AppAppearance>();
                    (appearance.revision, appearance.last_writer.clone())
                };
                this.appearance_event_status =
                    format!("Global observer: revision {revision} from {writer}");
                cx.invalidate();
            }));
        }
        let (appearance_warm, appearance_revision, appearance_writer) = cx
            .watch_global::<AppAppearance, _>(|appearance| {
                (
                    appearance.warm,
                    appearance.revision,
                    appearance.last_writer.clone(),
                )
            });
        let (workspace_revision, workspace_writer) = cx.observe(&self.workspace, |workspace| {
            (workspace.revision, workspace.last_writer.clone())
        });
        let toggle_protection = cx.listener("toggle-protection", |this, cx| {
            this.protected = !this.protected;
            this.blocked_close = false;
            cx.invalidate();
        });
        let close = cx.listener("close-this-window", |_this, cx| cx.close_window());
        let publish = cx.listener("publish-shared-update", |this, cx| {
            let number = this.number;
            let event = this.workspace.update(cx, |workspace, _cx| {
                workspace.revision = workspace.revision.saturating_add(1);
                workspace.last_writer = format!("Inspector {number}");
                WorkspacePublished {
                    revision: workspace.revision,
                    writer: workspace.last_writer.clone(),
                }
            });
            if !this.workspace.emit(cx, event) {
                this.workspace_event_status = "Typed event queue is full".to_owned();
                cx.invalidate();
            }
        });
        let toggle_appearance = cx.listener("toggle-global-appearance", |this, cx| {
            let writer = format!("Inspector {}", this.number);
            cx.update_global::<AppAppearance, _>(|appearance| {
                appearance.warm = !appearance.warm;
                appearance.revision = appearance.revision.saturating_add(1);
                appearance.last_writer = writer;
            });
        });
        let accent = if appearance_warm {
            Color::rgb8(251, 191, 36)
        } else {
            Color::rgb8(94, 234, 212)
        };
        let payload = WindowCard {
            label: Arc::from(format!("Inspector {} state card", self.number)),
            color: Color::rgb8(167, 139, 250),
        };
        let drag_payload = payload.clone();
        let drag = cx.drag_listener("inspector-transfer-source", move |_this, _event, _cx| {
            Drag::new(drag_payload.clone()).preview(transfer_preview(&drag_payload))
        });
        let drop = cx.drop_listener(
            "inspector-transfer-target",
            |this, payload: &WindowCard, event, cx| {
                this.received = Some(payload.clone());
                this.transfer_status = format!(
                    "'{}' arrived from {}",
                    payload.label,
                    transfer_origin(event.origin)
                );
                cx.invalidate();
            },
        );

        div()
            .size_full()
            .flex_col()
            .gap_4()
            .p_4()
            .bg(if appearance_warm {
                Color::rgb8(30, 25, 20)
            } else {
                Color::rgb8(20, 22, 27)
            })
            .text_color(Color::rgb8(234, 236, 241))
            .child(
                text(format!("Inspector {}", self.number))
                    .text_2xl()
                    .font_bold(),
            )
            .child(
                text("This is a different concrete Rust view type, erased only at the application window boundary.")
                    .text_sm()
                    .text_color(Color::rgb8(159, 166, 180)),
            )
            .child(
                div()
                    .flex_row()
                    .gap_2()
                    .child(
                        Launcher::control(if self.protected {
                            "Allow native close"
                        } else {
                            "Protect native close"
                        })
                        .on_click(toggle_protection),
                    )
                    .child(Launcher::control("Close this window").on_click(close))
                    .child(Launcher::control("Toggle global accent").on_click(toggle_appearance)),
            )
            .child(
                text(if appearance_writer.is_empty() {
                    "Application global has not changed".to_owned()
                } else {
                    format!(
                        "Global appearance revision {appearance_revision} from {appearance_writer}"
                    )
                })
                .text_sm()
                .text_color(accent),
            )
            .child(
                text(if self.appearance_event_status.is_empty() {
                    "No global observer callback received"
                } else {
                    self.appearance_event_status.as_str()
                })
                .text_sm()
                .text_color(accent),
            )
            .child(
                div()
                    .flex_row()
                    .items_center()
                    .gap_3()
                    .child(Launcher::control("Publish shared update").on_click(publish))
                    .child(
                        text(if workspace_writer.is_empty() {
                            "Shared entity has not changed".to_owned()
                        } else {
                            format!(
                                "Shared revision {workspace_revision} from {workspace_writer}"
                            )
                        })
                        .text_sm()
                        .text_color(Color::rgb8(126, 231, 212)),
                    ),
            )
            .child(
                text(if self.workspace_event_status.is_empty() {
                    "No typed workspace event received"
                } else {
                    self.workspace_event_status.as_str()
                })
                .text_sm()
                .text_color(Color::rgb8(167, 139, 250)),
            )
            .child(
                text(if self.blocked_close {
                    "The native close request was intercepted by this view. The explicit close button still works."
                } else if self.protected {
                    "Protection is active. The red native close button will emit CloseRequested without destroying the window."
                } else {
                    "Native close is allowed. Closing this window does not exit the application while the launcher remains."
                })
                .text_sm()
                .text_color(Color::rgb8(126, 231, 212)),
            )
            .child(
                div()
                    .flex_1()
                    .flex_row()
                    .gap_3()
                    .child(div().flex_1().child(transfer_card(&payload, drag)))
                    .child(
                        div()
                            .flex_1()
                            .child(transfer_target(drop, self.received.as_ref())),
                    ),
            )
            .child(
                text(if self.transfer_status.is_empty() {
                    "Drag this card to the launcher, or receive its blue card here"
                } else {
                    self.transfer_status.as_str()
                })
                .text_sm()
                .text_color(Color::rgb8(159, 166, 180)),
            )
    }
}
