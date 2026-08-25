use std::sync::Arc;

use quickgui::{
    AccessibilityRole, AnchorPlacement, App, Color, ContextMenuEvent, Element, EventContext,
    FocusHandle, Point, TitleBarStyle, Tooltip, View, ViewContext, button, div, overlay, text,
};

fn main() -> Result<(), quickgui::AppError> {
    App::new(TooltipContextDemo::default())
        .title("QuickGUI — Tooltips and context menus")
        .size(840.0, 580.0)
        .title_bar_style(TitleBarStyle::HiddenInset)
        .traffic_light_position(16.0, 13.0)
        .run()
}

#[derive(Default)]
struct TooltipContextDemo {
    menu_position: Option<Point>,
    status: Option<Arc<str>>,
}

impl TooltipContextDemo {
    fn choose(&mut self, action: &'static str, cx: &mut EventContext) {
        self.menu_position = None;
        self.status = Some(Arc::from(format!("Context action: {action}")));
        cx.focus(FocusHandle::new("context-surface"));
        cx.invalidate();
    }

    fn menu_item(label: &'static str, listener: quickgui::ClickListener<Self>) -> Element {
        button()
            .on_click(listener)
            .accessibility_role(AccessibilityRole::MenuItem)
            .w_full()
            .min_h(36.0)
            .px_3()
            .flex_row()
            .items_center()
            .rounded_md()
            .hover(|style| style.bg(Color::rgb8(53, 95, 145)))
            .active(|style| style.bg(Color::rgb8(39, 75, 116)))
            .focus(|style| style.border(2.0, Color::rgb8(96, 165, 250)))
            .child(text(label).text_sm())
    }
}

impl View for TooltipContextDemo {
    fn render(&mut self, cx: &mut ViewContext<'_, Self>) -> impl quickgui::IntoElement {
        let surface_focus = cx.focus_handle("context-surface");
        let open_context =
            cx.context_menu_listener("context-surface", |this, event: &ContextMenuEvent, cx| {
                this.menu_position = Some(event.position);
                this.status = Some(Arc::from(format!(
                    "Opened at {:.0}, {:.0}",
                    event.position.x, event.position.y
                )));
                cx.focus(FocusHandle::new("context-open"));
                cx.invalidate();
            });
        let dismiss = cx.dismiss_listener("context-popup", |this, cx| {
            this.menu_position = None;
            cx.invalidate();
        });
        let open = cx.listener("context-open", |this, cx| this.choose("Open", cx));
        let duplicate = cx.listener("context-duplicate", |this, cx| {
            this.choose("Duplicate", cx);
        });
        let reveal = cx.listener("context-reveal", |this, cx| {
            this.choose("Reveal in Finder", cx);
        });
        let tooltip_click = cx.listener("tooltip-button", |this, cx| {
            this.status = Some(Arc::from("Tooltip trigger clicked"));
            cx.invalidate();
        });
        let custom_tooltip_click = cx.listener("custom-tooltip-button", |this, cx| {
            this.status = Some(Arc::from("Custom tooltip trigger clicked"));
            cx.invalidate();
        });

        let mut root = div()
            .relative()
            .size_full()
            .flex_col()
            .bg(Color::rgb8(17, 19, 24))
            .text_color(Color::rgb8(235, 238, 244))
            .child(
                div()
                    .h(58.0)
                    .w_full()
                    .flex_none()
                    .flex_row()
                    .items_center()
                    .padding(0.0, 20.0, 0.0, 82.0)
                    .bg(Color::rgb8(27, 31, 39))
                    .border(1.0, Color::rgb8(49, 54, 65))
                    .app_region_drag()
                    .child(text("Tooltips + context menus").font_semibold()),
            )
            .child(
                div()
                    .flex_1()
                    .min_h(0.0)
                    .p_6()
                    .flex_col()
                    .gap_5()
                    .child(
                        div()
                            .flex_col()
                            .gap_1()
                            .child(text("Production interaction primitives").text_2xl().font_bold())
                            .child(
                                text("Hover either trigger. Secondary-click anywhere in the large surface; the menu is point-anchored and edge-aware.")
                                    .text_sm()
                                    .text_color(Color::rgb8(151, 158, 174)),
                            ),
                    )
                    .child(
                        div()
                            .flex_row()
                            .gap_4()
                            .child(
                                button()
                                    .on_click(tooltip_click)
                                    .id("tooltip-button")
                                    .h(42.0)
                                    .px_4()
                                    .flex_row()
                                    .items_center()
                                    .rounded_lg()
                                    .border(1.0, Color::rgb8(69, 76, 91))
                                    .bg(Color::rgb8(34, 38, 47))
                                    .hover(|style| style.bg(Color::rgb8(44, 50, 62)))
                                    .tooltip("One exact 500 ms deadline; no polling frames")
                                    .child("Text tooltip"),
                            )
                            .child(
                                button()
                                    .id("custom-tooltip-button")
                                    .on_click(custom_tooltip_click)
                                    .size(42.0, 42.0)
                                    .flex_row()
                                    .items_center()
                                    .justify_center()
                                    .rounded(21.0)
                                    .bg(Color::rgb8(39, 74, 114))
                                    .accessibility_label("About arbitrary tooltips")
                                    .tooltip(
                                        Tooltip::new(
                                            div()
                                                .w(270.0)
                                                .p_3()
                                                .flex_col()
                                                .gap_1()
                                                .rounded_lg()
                                                .border(1.0, Color::rgb8(73, 91, 116))
                                                .bg(Color::rgb8(31, 38, 49))
                                                .shadow_lg()
                                                .child(text("Arbitrary tooltip content").font_semibold())
                                                .child(
                                                    text("This is a detached QuickGUI element tree, not a platform label.")
                                                        .wrap()
                                                        .text_sm()
                                                        .text_color(Color::rgb8(174, 184, 201)),
                                                ),
                                        )
                                        .placement(AnchorPlacement::Right)
                                        .accessibility_description(
                                            "An arbitrary detached QuickGUI element tree",
                                        ),
                                    )
                                    .child(text("i").font_bold()),
                            ),
                    )
                    .child(
                        div()
                            .id("context-surface")
                            .on_context_menu(open_context)
                            .track_focus(surface_focus)
                            .flex_1()
                            .min_h(180.0)
                            .w_full()
                            .p_5()
                            .flex_col()
                            .items_center()
                            .justify_center()
                            .gap_2()
                            .rounded_xl()
                            .border(1.0, Color::rgb8(62, 70, 85))
                            .bg(Color::rgb8(26, 29, 36))
                            .focus(|style| style.border(2.0, Color::rgb8(96, 165, 250)))
                            .child(text("Secondary-click this surface").text_lg().font_semibold())
                            .child(
                                text(self.status.clone().unwrap_or_else(|| {
                                    Arc::from("The popup flips and clamps near every window edge")
                                }))
                                .text_sm()
                                .text_color(Color::rgb8(146, 154, 171)),
                            ),
                    ),
            );

        if let Some(position) = self.menu_position {
            root = root.child(
                overlay()
                    .id("context-popup")
                    .anchor_at(position, AnchorPlacement::BottomStart)
                    .anchor_gap(3.0)
                    .viewport_margin(8.0)
                    .w(224.0)
                    .p_1()
                    .flex_col()
                    .gap_1()
                    .rounded_lg()
                    .border(1.0, Color::rgb8(74, 81, 96))
                    .bg(Color::rgb8(31, 34, 42))
                    .shadow_xl()
                    .accessibility_role(AccessibilityRole::Menu)
                    .on_dismiss(dismiss)
                    .restore_focus_to(FocusHandle::new("context-surface"))
                    .child(Self::menu_item("Open", open).auto_focus())
                    .child(Self::menu_item("Duplicate", duplicate))
                    .child(Self::menu_item("Reveal in Finder", reveal)),
            );
        }
        root
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn context_actions_close_the_surface_and_update_status() {
        let mut demo = TooltipContextDemo {
            menu_position: Some(Point::new(10.0, 20.0)),
            status: None,
        };
        let mut cx = EventContext::default();

        demo.choose("Open", &mut cx);

        assert!(demo.menu_position.is_none());
        assert_eq!(demo.status.as_deref(), Some("Context action: Open"));
    }
}
