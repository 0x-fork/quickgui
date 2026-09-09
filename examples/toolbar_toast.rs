//! Caller-styled gallery for QuickGUI's unstyled toolbar, toggle group, and toast queue.
//!
//! Run with `cargo run --release --example toolbar_toast`.

use web_time::Instant;

use quickgui::{
    Application, AsyncViewContext, Color, Element, IntoElement, Task, Toast, ToastKind,
    ToastManager, ToastViewport, Toggle, ToggleGroup, ToggleGroupItem, ToggleGroupState, Toolbar,
    ToolbarItem, ToolbarState, View, ViewContext, WindowOptions, div, text,
    toggle_group_key_bindings, toolbar_key_bindings,
};

fn main() -> Result<(), quickgui::AppError> {
    Application::new()
        .bind_keys(toolbar_key_bindings())
        .bind_keys(toggle_group_key_bindings())
        .run(|cx| {
            cx.open_window(
                WindowOptions::new("QuickGUI — Toolbar and toasts").size(820.0, 600.0),
                ToolbarToastDemo::default(),
            );
        })
}

#[derive(Clone, Copy)]
struct Palette {
    background: Color,
    surface: Color,
    border: Color,
    foreground: Color,
    muted: Color,
    accent: Color,
    pressed: Color,
    warning: Color,
}

impl Palette {
    fn dark() -> Self {
        Self {
            background: Color::rgb8(17, 18, 21),
            surface: Color::rgb8(26, 28, 33),
            border: Color::rgb8(55, 59, 69),
            foreground: Color::rgb8(238, 240, 244),
            muted: Color::rgb8(155, 161, 174),
            accent: Color::rgb8(10, 132, 255),
            pressed: Color::rgb8(38, 62, 96),
            warning: Color::rgb8(255, 159, 10),
        }
    }
}

const COMMANDS: [(&str, &str); 4] = [
    ("undo", "Undo"),
    ("redo", "Redo"),
    ("share", "Share"),
    ("delete", "Delete"),
];

const ALIGNMENTS: [(&str, &str); 3] = [("left", "Left"), ("center", "Center"), ("right", "Right")];

const MARKS: [(&str, &str); 3] = [
    ("bold", "Bold"),
    ("italic", "Italic"),
    ("underline", "Underline"),
];

fn toolbar_items() -> [ToolbarItem; 4] {
    [
        ToolbarItem::new(COMMANDS[0].0),
        ToolbarItem::new(COMMANDS[1].0).disabled(true),
        ToolbarItem::new(COMMANDS[2].0),
        ToolbarItem::new(COMMANDS[3].0),
    ]
}

fn toggle_items<const N: usize>(source: [(&str, &str); N]) -> [ToggleGroupItem; N] {
    source.map(|(value, _)| ToggleGroupItem::new(value))
}

struct ToolbarToastDemo {
    #[cfg(target_arch = "wasm32")]
    docs_component: String,
    toolbar: ToolbarState,
    alignment: ToggleGroupState,
    marks: ToggleGroupState,
    inspector: bool,
    toasts: ToastManager,
    expiry: Option<Task<()>>,
}

impl Default for ToolbarToastDemo {
    fn default() -> Self {
        let mut alignment = ToggleGroupState::single();
        alignment.press("left");
        Self {
            #[cfg(target_arch = "wasm32")]
            docs_component: String::new(),
            toolbar: ToolbarState::empty(),
            alignment,
            marks: ToggleGroupState::multiple(),
            inspector: false,
            // Base UI's provider settings: three visible toasts, the rest flagged limited, and a
            // 48-point rightward swipe to dismiss.
            toasts: ToastManager::new().limit(3).swipe_threshold(48.0),
            expiry: None,
        }
    }
}

impl ToolbarToastDemo {
    fn toolbar_state(view: &mut Self) -> &mut ToolbarState {
        &mut view.toolbar
    }

    fn alignment_state(view: &mut Self) -> &mut ToggleGroupState {
        &mut view.alignment
    }

    fn marks_state(view: &mut Self) -> &mut ToggleGroupState {
        &mut view.marks
    }

    fn toasts(view: &mut Self) -> &mut ToastManager {
        &mut view.toasts
    }

    fn section(title: &'static str, colors: Palette, content: Element) -> Element {
        div()
            .w_full()
            .flex_col()
            .gap_3()
            .p_4()
            .rounded_xl()
            .border(1.0, colors.border)
            .bg(colors.surface)
            .child(text(title).text_sm().font_semibold())
            .child(content)
    }

    fn chrome(colors: Palette, pressed: bool) -> Element {
        div()
            .min_h(28.0)
            .px_3()
            .flex_row()
            .items_center()
            .gap_2()
            .rounded_md()
            .border(1.0, colors.border)
            .bg(if pressed {
                colors.pressed
            } else {
                colors.surface
            })
            .hover(|state| state.border(1.0, colors.muted))
            .focus(|state| state.border(2.0, colors.accent))
            .disabled_style(|state| state.opacity(0.4))
    }

    fn commands(&self, cx: &mut ViewContext<'_, Self>, colors: Palette) -> Element {
        let items = toolbar_items();
        let toolbar = Toolbar::new("commands", &self.toolbar, &items);
        let mut root = toolbar
            .root_part(div().flex_row().items_center().gap_2())
            .accessibility_label("Document commands");
        // Base UI's Toolbar.Group and Toolbar.Separator: a labelling and structural unit and a
        // divider that never takes focus, both inside the toolbar's single roving Tab stop.
        let mut group = toolbar
            .group_part(div().flex_row().gap_2())
            .accessibility_label("File commands");
        for (index, (item, (_, label))) in items.into_iter().zip(COMMANDS).enumerate() {
            let entry = toolbar.item(item.value()).expect("declared item");
            let value = item.value();
            let clicked = cx.listener(entry.item_id(), move |view: &mut Self, cx| {
                view.toolbar.focus(value);
                view.toasts.push(
                    Toast::new(format!("{label} requested"))
                        .description("QuickGUI announced this toast through a live region.")
                        .action("Undo")
                        .kind(if label == "Delete" {
                            ToastKind::Warning
                        } else {
                            ToastKind::Info
                        }),
                    Instant::now(),
                );
                cx.invalidate();
            });
            group = group.child(
                entry.key_part(
                    cx,
                    entry
                        .button_part(
                            Self::chrome(colors, false)
                                .child(text(label))
                                .on_click(clicked),
                        )
                        .accessibility_label(label),
                    Self::toolbar_state,
                ),
            );
            if index + 1 == COMMANDS.len() - 1 {
                root = root.child(group);
                root = root.child(toolbar.separator_part(div().w(1.0).h(20.0).bg(colors.border)));
                group = toolbar
                    .group_part(div().flex_row().gap_2())
                    .accessibility_label("Destructive commands");
            }
        }
        root.child(group)
    }

    fn alignment(&self, cx: &mut ViewContext<'_, Self>, colors: Palette) -> Element {
        let items = toggle_items(ALIGNMENTS);
        let group = ToggleGroup::new("alignment", &self.alignment, &items);
        let mut root = group
            .root_part(div().flex_row().gap_2())
            .accessibility_label("Text alignment");
        for (item, (_, label)) in items.into_iter().zip(ALIGNMENTS) {
            let entry = group.item(item.value()).expect("declared item");
            let value = item.value();
            let clicked = cx.listener(entry.item_id(), move |view: &mut Self, cx| {
                view.alignment.toggle(value);
                view.alignment.focus(value);
                cx.invalidate();
            });
            root = root.child(
                entry.key_part(
                    cx,
                    entry
                        .item_part(
                            Self::chrome(colors, entry.is_pressed())
                                .child(text(label))
                                .on_click(clicked),
                        )
                        .accessibility_label(label),
                    Self::alignment_state,
                ),
            );
        }
        root
    }

    fn marks(&self, cx: &mut ViewContext<'_, Self>, colors: Palette) -> Element {
        let items = toggle_items(MARKS);
        let group = ToggleGroup::new("marks", &self.marks, &items);
        let mut root = group
            .root_part(div().flex_row().gap_2())
            .accessibility_label("Text style");
        for (item, (_, label)) in items.into_iter().zip(MARKS) {
            let entry = group.item(item.value()).expect("declared item");
            let value = item.value();
            let clicked = cx.listener(entry.item_id(), move |view: &mut Self, cx| {
                view.marks.toggle(value);
                view.marks.focus(value);
                cx.invalidate();
            });
            root = root.child(
                entry.key_part(
                    cx,
                    entry
                        .item_part(
                            Self::chrome(colors, entry.is_pressed())
                                .child(text(label))
                                .on_click(clicked),
                        )
                        .accessibility_label(label),
                    Self::marks_state,
                ),
            );
        }
        root
    }

    fn inspector_toggle(&self, cx: &mut ViewContext<'_, Self>, colors: Palette) -> Element {
        let control = Toggle::new(self.inspector);
        let clicked = cx.listener("inspector", |view: &mut Self, cx| {
            view.inspector = !view.inspector;
            cx.invalidate();
        });
        control
            .root_part(
                Self::chrome(colors, control.is_pressed())
                    .child(text("Inspector"))
                    .on_click(clicked),
            )
            .id("inspector")
    }

    fn toasts_surface(&self, cx: &mut ViewContext<'_, Self>, colors: Palette) -> Element {
        let viewport = ToastViewport::new("toasts");
        let mut surface = viewport.viewport_part(div().flex_col().gap_2().w(320.0));
        // `toasts` walks the queue newest first and hands each descriptor its own stack index, the
        // provider's limited flag, the expanded state, and any swipe in flight.
        for parts in viewport.toasts(&self.toasts).collect::<Vec<_>>() {
            let id = parts.id();
            let entry = self
                .toasts
                .entry(id)
                .expect("the descriptor came from this queue");
            let dismiss = cx.listener(parts.close_id(), move |view: &mut Self, cx| {
                view.toasts.dismiss(id);
                cx.invalidate();
            });
            let undo = cx.listener(parts.action_id(), move |view: &mut Self, cx| {
                view.toasts.dismiss(id);
                cx.invalidate();
            });
            let hover = cx.hover_listener(parts.root_id(), move |view: &mut Self, hovered, cx| {
                let now = Instant::now();
                let changed = if *hovered {
                    view.toasts.pause(id, now)
                } else {
                    view.toasts.resume(id, now)
                };
                if changed {
                    cx.invalidate();
                }
            });

            // Base UI's swipe-to-dismiss: QuickGUI owns the threshold and the queue, the example
            // owns the translation it draws while the gesture is in flight.
            let swipe = cx.pointer_listener(parts.root_id(), move |view: &mut Self, event, cx| {
                if view.toasts.apply_swipe(id, event).changed {
                    cx.invalidate();
                }
            });

            let mut card = div()
                .flex_col()
                .gap_1()
                .p_3()
                .rounded_lg()
                .border(
                    1.0,
                    if entry.toast().toast_kind().is_assertive() {
                        colors.warning
                    } else {
                        colors.border
                    },
                )
                .bg(colors.surface)
                .child(
                    parts.title_part(
                        text(entry.toast().title().clone())
                            .text_sm()
                            .font_semibold(),
                    ),
                );
            if let Some(description) = entry.toast().description_text() {
                card = card.child(
                    parts.description_part(
                        text(description.clone())
                            .text_sm()
                            .wrap()
                            .text_color(colors.muted),
                    ),
                );
            }
            let mut controls = div().flex_row().gap_2();
            if let Some(label) = entry.toast().action_label() {
                controls = controls.child(
                    parts.action_part(
                        Self::chrome(colors, false)
                            .child(text(label.clone()))
                            .on_click(undo),
                    ),
                );
            }
            controls = controls.child(
                parts.close_part(
                    Self::chrome(colors, false)
                        .child(text("Dismiss"))
                        .on_click(dismiss),
                ),
            );
            card = card.child(controls);

            let root = parts.key_part(
                cx,
                parts
                    .root_part(div().child(parts.content_part(card)))
                    .focus(|state| state.border(2.0, colors.accent))
                    .opacity(if parts.is_limited() { 0.5 } else { 1.0 })
                    .on_hover(hover)
                    .on_pointer(swipe),
                Self::toasts,
            );
            surface = surface.child(
                parts.positioner_part(div().translate(parts.swipe_movement(), 0.0).child(root)),
            );
        }
        surface
    }

    fn schedule_expiry(&mut self, cx: &mut ViewContext<'_, Self>) {
        let Some(deadline) = self.toasts.next_deadline() else {
            self.expiry = None;
            return;
        };
        if self.expiry.is_some() {
            return;
        }
        self.expiry = cx
            .spawn(move |task: AsyncViewContext<Self>| async move {
                let mut next = deadline;
                loop {
                    if task.sleep_until(next).await.is_err() {
                        return;
                    }
                    let now = task.now();
                    let outcome = task
                        .update(move |view: &mut Self, cx| {
                            if view.toasts.expire(now) {
                                cx.invalidate();
                            }
                            view.toasts.next_deadline()
                        })
                        .await;
                    match outcome {
                        Ok(Some(deadline)) => next = deadline,
                        _ => return,
                    }
                }
            })
            .ok();
    }
}

impl View for ToolbarToastDemo {
    fn render(&mut self, cx: &mut ViewContext<'_, Self>) -> impl IntoElement {
        let colors = Palette::dark();
        // One exact deadline while toasts are queued, rearmed from the current queue, and none at
        // all once the queue drains.
        self.expiry = None;
        self.schedule_expiry(cx);

        #[cfg(target_arch = "wasm32")]
        if !self.docs_component.is_empty() {
            let content = if self.docs_component == "toggle-group" {
                self.alignment(cx, colors)
            } else {
                self.commands(cx, colors)
            };
            return div()
                .size_full()
                .p(20.0)
                .flex_col()
                .gap(24.0)
                .items_center()
                .justify_center()
                .bg(colors.background)
                .text_color(colors.foreground)
                .child(content)
                .child(self.toasts_surface(cx, colors));
        }

        div()
            .size_full()
            .flex_col()
            .gap_4()
            .p_5()
            .bg(colors.background)
            .text_color(colors.foreground)
            .child(
                text("Unstyled toolbar, toggle group, and toasts")
                    .text_lg()
                    .font_semibold(),
            )
            .child(
                text("Tab reaches the toolbar once; arrow keys move between its commands. Toasts announce themselves through a live region and dismiss on Escape while focused.")
                    .text_sm()
                    .wrap()
                    .text_color(colors.muted),
            )
            .child(Self::section("Toolbar", colors, self.commands(cx, colors)))
            .child(Self::section(
                "Toggle groups",
                colors,
                div()
                    .flex_col()
                    .gap_3()
                    .child(self.alignment(cx, colors))
                    .child(self.marks(cx, colors))
                    .child(self.inspector_toggle(cx, colors)),
            ))
            .child(Self::section(
                "Toasts",
                colors,
                self.toasts_surface(cx, colors),
            ))
    }
}

/// Focused presentation of the same native example for browser documentation.
#[cfg(target_arch = "wasm32")]
pub fn docs_demo(component: String) -> impl quickgui::View {
    let mut view = ToolbarToastDemo::default();
    view.docs_component = component;
    view
}
