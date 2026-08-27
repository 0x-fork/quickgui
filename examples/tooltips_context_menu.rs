use std::{sync::Arc, time::Duration};

use quickgui::{
    AnchorPlacement, Animation, AnimationExt as _, AnimationPhase, App, Color, ContextMenuLayout,
    ContextMenuState, Element, PopupMenu, PopupMenuItem, PopupMenuItemKind, PopupMenuItemState,
    TitleBarStyle, Tooltip, View, ViewContext, button, div, ease_out_quint,
    popup_menu_key_bindings, text,
};

fn main() -> Result<(), quickgui::AppError> {
    App::new(TooltipContextDemo::default())
        .bind_keys(popup_menu_key_bindings())
        .title("QuickGUI — Tooltips and context menus")
        .size(840.0, 580.0)
        .title_bar_style(TitleBarStyle::HiddenInset)
        .traffic_light_position(16.0, 13.0)
        .run()
}

#[derive(Default)]
struct TooltipContextDemo {
    context_menu: ContextMenuState,
    status: Option<Arc<str>>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ContextCommand {
    Open,
    Duplicate,
    Reveal,
    CopyPath,
    Inspect,
}

impl TooltipContextDemo {
    fn context_menu(view: &mut Self) -> &mut ContextMenuState {
        &mut view.context_menu
    }

    fn choose(&mut self, command: ContextCommand) {
        let action = match command {
            ContextCommand::Open => "Open",
            ContextCommand::Duplicate => "Duplicate",
            ContextCommand::Reveal => "Reveal in Finder",
            ContextCommand::CopyPath => "Copy Path",
            ContextCommand::Inspect => "Inspect",
        };
        self.status = Some(Arc::from(format!("Context action: {action}")));
    }

    fn menu() -> PopupMenu {
        let more = PopupMenu::new([
            PopupMenuItem::action("copy-path", "Copy Path", ContextCommand::CopyPath),
            PopupMenuItem::action("inspect", "Inspect", ContextCommand::Inspect),
        ])
        .expect("the static context submenu is valid");
        PopupMenu::new([
            PopupMenuItem::group_label("File"),
            PopupMenuItem::action("open", "Open", ContextCommand::Open).shortcut("⌘O"),
            PopupMenuItem::action("duplicate", "Duplicate", ContextCommand::Duplicate)
                .shortcut("⌘D"),
            PopupMenuItem::separator(),
            PopupMenuItem::action("reveal", "Reveal in Finder", ContextCommand::Reveal),
            PopupMenuItem::submenu("more", "More", more),
        ])
        .expect("the static context menu is valid")
    }

    fn menu_item(item: &PopupMenuItem, state: PopupMenuItemState) -> Element {
        match item.kind() {
            PopupMenuItemKind::Separator => div()
                .px_2()
                .child(div().mt(4.0).h(1.0).bg(Color::rgb8(74, 81, 96))),
            PopupMenuItemKind::GroupLabel => div().px_3().flex_row().items_center().child(
                text(item.label().clone())
                    .text_xs()
                    .text_color(Color::rgb8(145, 154, 172)),
            ),
            _ => div()
                .w_full()
                .px_3()
                .rounded_md()
                .opacity(if state.disabled { 0.45 } else { 1.0 })
                .bg(if state.highlighted {
                    Color::rgb8(53, 95, 145)
                } else {
                    Color::TRANSPARENT
                })
                .child(
                    div()
                        .size_full()
                        .flex_row()
                        .items_center()
                        .justify_between()
                        .child(text(item.label().clone()).text_sm())
                        .child(if state.has_submenu {
                            text("›").text_sm()
                        } else {
                            text(item.shortcut_text().cloned().unwrap_or_else(|| "".into()))
                                .text_xs()
                                .text_color(Color::rgb8(145, 154, 172))
                        }),
                ),
        }
    }
}

impl View for TooltipContextDemo {
    fn render(&mut self, cx: &mut ViewContext<'_, Self>) -> impl quickgui::IntoElement {
        let surface_focus = cx.focus_handle("context-surface");
        let context_action =
            cx.action_listener("context-actions", |this, command: &ContextCommand, cx| {
                this.choose(*command);
                cx.invalidate();
            });
        let context_surface = self.context_menu.element(
            cx,
            "context-surface",
            Self::context_menu,
            div()
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
                .child(
                    text("Secondary-click this surface")
                        .text_lg()
                        .font_semibold(),
                )
                .child(
                    text(self.status.clone().unwrap_or_else(|| {
                        Arc::from(
                            "Hover More for delayed native submenu aim; popups fit the display",
                        )
                    }))
                    .text_sm()
                    .text_color(Color::rgb8(146, 154, 171)),
                ),
            ContextMenuLayout::new(224.0, 36.0)
                .separator_height(9.0)
                .group_label_height(22.0)
                .vertical_padding(4.0)
                .submenu_gap(2.0),
            |this, event| {
                this.status = Some(Arc::from(format!(
                    "Opened at {:.0}, {:.0}",
                    event.position.x, event.position.y
                )));
                Some(Self::menu())
            },
            || {
                div()
                    .p_1()
                    .flex_col()
                    .rounded_lg()
                    .border(1.0, Color::rgb8(74, 81, 96))
                    .bg(Color::rgb8(31, 34, 42))
                    .shadow_xl()
                    .text_color(Color::rgb8(235, 238, 244))
            },
            Self::menu_item,
        );
        let tooltip_click = cx.listener("tooltip-button", |this, cx| {
            this.status = Some(Arc::from("Tooltip trigger clicked"));
            cx.invalidate();
        });
        let custom_tooltip_click = cx.listener("custom-tooltip-button", |this, cx| {
            this.status = Some(Arc::from("Custom tooltip trigger clicked"));
            cx.invalidate();
        });

        div()
            .focus_scope(cx.focus_handle("context-actions"))
            .on_action(context_action)
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
                                text("Hover either trigger. Secondary-click the large surface, then move diagonally through More; the native submenu uses delayed safe-corridor aim.")
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
                                                .shadow_lg()
                                                .child(text("Arbitrary tooltip content").font_semibold())
                                                .child(
                                                    text("This is a detached QuickGUI element tree, not a platform label.")
                                                        .wrap()
                                                        .text_sm()
                                                        .text_color(Color::rgb8(174, 184, 201)),
                                                )
                                                .with_animation(
                                                    "tooltip-entrance",
                                                    Animation::new(Duration::from_millis(140))
                                                        .with_easing(ease_out_quint()),
                                                    |element, value| {
                                                        let phase = AnimationPhase(value);
                                                        element
                                                            .w(phase.interpolate_clamped(238.0, 270.0))
                                                            .rounded(phase.interpolate_clamped(4.0, 8.0))
                                                            .bg(phase.interpolate_clamped(
                                                                Color::rgb8(20, 24, 31),
                                                                Color::rgb8(31, 38, 49),
                                                            ))
                                                    },
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
                    .child(context_surface),
            )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn context_actions_close_the_surface_and_update_status() {
        let mut demo = TooltipContextDemo::default();
        demo.choose(ContextCommand::Open);

        assert_eq!(demo.status.as_deref(), Some("Context action: Open"));
    }
}
