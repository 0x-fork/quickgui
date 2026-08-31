use std::sync::Arc;

use quickgui::{
    AccessibilityRole, AnchorPlacement, Application, Color, Element, EventContext, FocusHandle,
    View, ViewContext, button, div, overlay, text,
};

fn main() -> Result<(), quickgui::AppError> {
    Application::new().run(|cx| {
        cx.open_window(
            quickgui::WindowOptions::new("QuickGUI — Web-style overlays").size(760.0, 520.0),
            OverlayDemo::default(),
        );
    })
}

#[derive(Default)]
struct OverlayDemo {
    menu_open: bool,
    choice: Option<&'static str>,
}

impl OverlayDemo {
    fn button(label: impl Into<Arc<str>>) -> Element {
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

    fn menu_item(label: &'static str, listener: quickgui::ClickListener<Self>) -> Element {
        button()
            .on_click(listener)
            .accessibility_role(AccessibilityRole::MenuItem)
            .w_full()
            .min_h(38.0)
            .flex_row()
            .items_center()
            .px_3()
            .py_2()
            .rounded_md()
            .hover(|style| style.bg(Color::rgb8(55, 72, 94)))
            .active(|style| style.bg(Color::rgb8(42, 91, 139)))
            .focus(|style| style.border(2.0, Color::rgb8(94, 234, 212)))
            .child(label)
    }

    fn choose(&mut self, value: &'static str, cx: &mut EventContext) {
        self.choice = Some(value);
        self.menu_open = false;
        cx.invalidate();
    }
}

impl View for OverlayDemo {
    fn render(&mut self, cx: &mut ViewContext<'_, Self>) -> impl quickgui::IntoElement {
        let trigger = cx.focus_handle("actions-trigger");
        let toggle = cx.listener(trigger.id(), |this, cx| {
            this.menu_open = !this.menu_open;
            cx.invalidate();
        });
        let dismiss = cx.dismiss_listener("actions-menu", |this, cx| {
            this.menu_open = false;
            cx.invalidate();
        });
        let duplicate = cx.listener("duplicate", move |this, cx| {
            this.choose("Duplicate", cx);
            cx.focus(trigger);
        });
        let archive = cx.listener("archive", move |this, cx| {
            this.choose("Archive", cx);
            cx.focus(trigger);
        });
        let delete = cx.listener("delete", move |this, cx| {
            this.choose("Delete", cx);
            cx.focus(trigger);
        });

        let root = div()
            .relative()
            .size_full()
            .flex_col()
            .gap_4()
            .p_4()
            .bg(Color::rgb8(18, 19, 23))
            .text_color(Color::rgb8(234, 236, 241))
            .child(
                div()
                    .flex_row()
                    .items_center()
                    .justify_between()
                    .gap_4()
                    .child(
                        div()
                            .flex_col()
                            .gap_1()
                            .child(
                                text("Ordered overlays")
                                    .accessibility_role(AccessibilityRole::Heading)
                                    .text_2xl()
                                    .font_bold(),
                            )
                            .child(
                                text("The menu flips, shifts, blocks click-through, and restores focus like a web popover.")
                                    .text_sm()
                                    .text_color(Color::rgb8(159, 166, 180)),
                            ),
                    )
                    .child(
                        Self::button(if self.menu_open {
                            "Close actions"
                        } else {
                            "Open actions"
                        })
                        .on_click(toggle)
                        .track_focus(trigger)
                        .accessibility_label("Actions menu"),
                    ),
            )
            .child(
                div()
                    .flex_1()
                    .relative()
                    .overflow_hidden()
                    .rounded_xl()
                    .border(1.0, Color::rgb8(55, 60, 71))
                    .bg(Color::rgb8(25, 27, 33))
                    .child(
                        div()
                            .absolute()
                            .top(36.0)
                            .left(36.0)
                            .w(390.0)
                            .p_4()
                            .rounded_xl()
                            .bg(Color::rgb8(36, 40, 49))
                            .child(text("Base-plane text remains behind the opaque menu surface, even though glyphs use a separate GPU pipeline.").text_lg()),
                    )
                    .child(
                        div()
                            .absolute()
                            .top(150.0)
                            .left(120.0)
                            .w(430.0)
                            .p_4()
                            .rounded_xl()
                            .bg(Color::rgb8(43, 47, 58))
                            .child(text("Resize the window or move the trigger toward an edge: placement is recomputed from retained layout bounds.").text_lg()),
                    )
                    .child(
                        div()
                            .absolute()
                            .bottom(24.0)
                            .left(24.0)
                            .child(
                                text(match self.choice {
                                    Some(choice) => format!("Last action: {choice}"),
                                    None => "No action selected".to_owned(),
                                })
                                .text_sm()
                                .text_color(Color::rgb8(126, 231, 212)),
                            ),
                    ),
            );

        if self.menu_open {
            root.child(
                overlay()
                    .anchor_to(trigger.id(), AnchorPlacement::BottomEnd)
                    .anchor_gap(8.0)
                    .viewport_margin(12.0)
                    .on_dismiss(dismiss)
                    .restore_focus_to(FocusHandle::new(trigger.id()))
                    .accessibility_role(AccessibilityRole::Menu)
                    .accessibility_label("Actions")
                    .w(228.0)
                    .flex_col()
                    .gap_1()
                    .p_2()
                    .rounded_xl()
                    .border(1.0, Color::rgb8(85, 92, 108))
                    .bg(Color::rgb8(29, 32, 39))
                    .child(Self::menu_item("Duplicate", duplicate))
                    .child(Self::menu_item("Archive", archive))
                    .child(
                        Self::menu_item("Delete", delete).text_color(Color::rgb8(248, 113, 113)),
                    ),
            )
        } else {
            root
        }
    }
}
