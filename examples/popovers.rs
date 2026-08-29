use std::sync::Arc;

use quickgui::{
    AccessibilityRole, AnchorPlacement, App, BoxShadow, Color, Element, EventContext, IntoElement,
    Popover, PopoverKind, View, ViewContext, WindowAppearance, button, div, text,
};

fn main() -> Result<(), quickgui::AppError> {
    App::new(PopoverGallery::default())
        .title("QuickGUI — Controlled popovers")
        .size(900.0, 620.0)
        .run()
}

#[derive(Default)]
struct PopoverGallery {
    account_open: bool,
    nested_open: bool,
    actions_open: bool,
    status: Option<Arc<str>>,
}

#[derive(Clone, Copy)]
struct GalleryPalette {
    background: Color,
    card: Color,
    border: Color,
    foreground: Color,
    muted: Color,
    popover: Color,
    popover_border: Color,
    popover_shadow: BoxShadow,
}

impl GalleryPalette {
    fn new(appearance: WindowAppearance) -> Self {
        if appearance == WindowAppearance::Light {
            Self {
                background: Color::rgb8(242, 244, 248),
                card: Color::rgb8(255, 255, 255),
                border: Color::rgb8(210, 214, 222),
                foreground: Color::rgb8(30, 34, 41),
                muted: Color::rgb8(92, 99, 112),
                popover: Color::rgb8(255, 255, 255),
                popover_border: Color::rgb8(205, 208, 216),
                popover_shadow: BoxShadow::new(0.0, 10.0, Color::rgba8(0, 0, 0, 48))
                    .blur_radius(32.0)
                    .spread_radius(-8.0),
            }
        } else {
            Self {
                background: Color::rgb8(17, 19, 23),
                card: Color::rgb8(25, 28, 34),
                border: Color::rgb8(55, 61, 72),
                foreground: Color::rgb8(235, 238, 244),
                muted: Color::rgb8(157, 166, 183),
                popover: Color::rgb8(31, 34, 40),
                popover_border: Color::rgb8(76, 82, 94),
                popover_shadow: BoxShadow::new(0.0, 10.0, Color::rgba8(0, 0, 0, 100))
                    .blur_radius(32.0)
                    .spread_radius(-8.0),
            }
        }
    }
}

impl PopoverGallery {
    fn close_all(&mut self) {
        self.account_open = false;
        self.nested_open = false;
        self.actions_open = false;
    }

    fn select(&mut self, value: &'static str, cx: &mut EventContext) {
        self.status = Some(Arc::from(value));
        self.close_all();
        cx.invalidate();
    }
}

impl View for PopoverGallery {
    fn render(&mut self, cx: &mut ViewContext<'_, Self>) -> impl IntoElement {
        let appearance = cx.appearance();
        let palette = GalleryPalette::new(appearance);
        let account = Popover::new("account-trigger", "account-popover", self.account_open)
            .placement(AnchorPlacement::BottomEnd)
            .initial_focus("profile");
        let nested = Popover::new("nested-trigger", "nested-popover", self.nested_open)
            .placement(AnchorPlacement::RightStart)
            .initial_focus("shortcut");
        let actions = Popover::new("actions-trigger", "actions-popover", self.actions_open)
            .kind(PopoverKind::Menu)
            .placement(AnchorPlacement::TopEnd)
            .initial_focus("duplicate");

        let toggle_account = cx.listener(account.trigger_id(), move |this, cx| {
            let opening = !this.account_open;
            this.close_all();
            this.account_open = opening;
            if opening {
                account.focus_surface(cx);
            } else {
                account.focus_trigger(cx);
            }
            cx.invalidate();
        });
        let dismiss_account = cx.dismiss_listener(account.surface_id(), |this, cx| {
            this.account_open = false;
            this.nested_open = false;
            cx.invalidate();
        });
        let close_account = cx.listener(account.close_id(), move |this, cx| {
            this.account_open = false;
            this.nested_open = false;
            account.focus_trigger(cx);
            cx.invalidate();
        });
        let toggle_nested = cx.listener(nested.trigger_id(), move |this, cx| {
            this.nested_open = !this.nested_open;
            if this.nested_open {
                nested.focus_surface(cx);
            } else {
                nested.focus_trigger(cx);
            }
            cx.invalidate();
        });
        let dismiss_nested = cx.dismiss_listener(nested.surface_id(), |this, cx| {
            this.nested_open = false;
            cx.invalidate();
        });
        let toggle_actions = cx.listener(actions.trigger_id(), move |this, cx| {
            let opening = !this.actions_open;
            this.close_all();
            this.actions_open = opening;
            if opening {
                actions.focus_surface(cx);
            } else {
                actions.focus_trigger(cx);
            }
            cx.invalidate();
        });
        let dismiss_actions = cx.dismiss_listener(actions.surface_id(), |this, cx| {
            this.actions_open = false;
            cx.invalidate();
        });
        let profile = cx.listener("profile", move |this, cx| {
            this.select("Profile selected", cx);
            account.focus_trigger(cx);
        });
        let preferences = cx.listener("preferences", move |this, cx| {
            this.select("Preferences selected", cx);
            account.focus_trigger(cx);
        });
        let shortcut = cx.listener("shortcut", move |this, cx| {
            this.select("Keyboard shortcut changed", cx);
            nested.focus_trigger(cx);
        });
        let duplicate = cx.listener("duplicate", move |this, cx| {
            this.select("Duplicate selected", cx);
            actions.focus_trigger(cx);
        });
        let archive = cx.listener("archive", move |this, cx| {
            this.select("Archive selected", cx);
            actions.focus_trigger(cx);
        });

        let mut root = div()
            .relative()
            .size_full()
            .p_5()
            .flex_col()
            .gap_5()
            .bg(palette.background)
            .text_color(palette.foreground)
            .child(
                div()
                    .flex_row()
                    .items_center()
                    .justify_between()
                    .gap_4()
                    .child(
                        div()
                            .min_w(0.0)
                            .flex_col()
                            .gap_1()
                            .child(
                                text("Controlled popovers")
                                    .accessibility_role(AccessibilityRole::Heading)
                                    .text_2xl()
                                    .font_bold(),
                            )
                            .child(
                                text("Ordinary trigger/content elements with retained placement, nested light dismissal, and exact focus restoration.")
                                    .wrap()
                                    .text_sm()
                                    .text_color(palette.muted),
                            ),
                    )
                    .child(
                        gallery_trigger(account.trigger_part(div()), "Account")
                            .on_click(toggle_account)
                            .accessibility_label("Account options"),
                    ),
            )
            .child(
                div()
                    .flex_1()
                    .min_h(0.0)
                    .relative()
                    .rounded_xl()
                    .border(1.0, palette.border)
                    .bg(palette.card)
                    .p_5()
                    .child(
                        div()
                            .w_full()
                            .max_w(420.0)
                            .flex_col()
                            .gap_3()
                            .child(text("What this layer guarantees").text_lg().font_semibold())
                            .child(
                                text("The application owns every open boolean. QuickGUI reuses one overlay path for anchor flip/shift, topmost Escape and outside-press dismissal, pointer blocking, and focus return. Opening focus is handed to content in the same state-changing callback—no next-frame task or polling loop.")
                                    .wrap()
                                    .text_sm()
                                    .text_color(palette.muted),
                            )
                            .child(
                                text(self.status.clone().unwrap_or_else(|| Arc::from(
                                    "Open a surface, Tab through it, then press Escape.",
                                )))
                                .wrap()
                                .text_sm()
                                .text_color(Color::rgb8(10, 132, 255)),
                            ),
                    )
                    .child(
                        gallery_trigger(actions.trigger_part(div()), "Actions")
                            .absolute()
                            .right(24.0)
                            .bottom(24.0)
                            .on_click(toggle_actions)
                            .accessibility_label("Document actions"),
                    ),
            );

        if account.is_open() {
            let mut popover = gallery_popover(account.popover_part(div()), 180.0, palette)
                .w(306.0)
                .gap_1()
                .on_dismiss(dismiss_account)
                .child(
                    div()
                        .flex_row()
                        .items_center()
                        .justify_between()
                        .gap_2()
                        .child(account.title_part(text("Account options").font_semibold()))
                        .child(
                            account
                                .close_part("Close account options", div())
                                .on_click(close_account)
                                .px_2()
                                .py_1()
                                .rounded_md()
                                .hover(|style| style.bg(Color::rgba8(10, 132, 255, 34)))
                                .focus(|style| style.border(2.0, Color::rgb8(10, 132, 255)))
                                .child(text("Close").text_sm()),
                        ),
                )
                .child(
                    account.description_part(
                        text("Choose an account action or open the nested shortcut popover.")
                            .wrap()
                            .text_sm()
                            .text_color(palette.muted),
                    ),
                )
                .child(popover_item("profile", "Profile", profile))
                .child(popover_item("preferences", "Preferences", preferences))
                .child(
                    gallery_trigger(nested.trigger_part(div()), "Keyboard shortcut…")
                        .w_full()
                        .on_click(toggle_nested)
                        .accessibility_label("Keyboard shortcut options"),
                );
            if nested.is_open() {
                let nested_popover = gallery_popover(nested.popover_part(div()), 210.0, palette)
                    .gap_2()
                    .on_dismiss(dismiss_nested)
                    .child(nested.title_part(text("Keyboard shortcut").font_semibold()))
                    .child(
                        nested.description_part(
                            text("Nested surfaces dismiss one level at a time.")
                                .wrap()
                                .text_sm()
                                .text_color(palette.muted),
                        ),
                    )
                    .child(
                        popover_item("shortcut", "Use ⌘⇧P", shortcut)
                            .accessibility_description("Set the keyboard shortcut"),
                    );
                popover = popover.child(nested.positioner_part(div().child(nested_popover)));
            }
            root = root.child(account.positioner_part(div().child(popover)));
        }

        if actions.is_open() {
            let popover = gallery_popover(actions.popover_part(div()), 224.0, palette)
                .gap_1()
                .on_dismiss(dismiss_actions)
                .accessibility_label("Document actions")
                .child(
                    popover_item("duplicate", "Duplicate", duplicate)
                        .accessibility_role(AccessibilityRole::MenuItem),
                )
                .child(
                    popover_item("archive", "Archive", archive)
                        .accessibility_role(AccessibilityRole::MenuItem),
                );
            root = root.child(actions.positioner_part(div().child(popover)));
        }

        root
    }
}

fn gallery_trigger(trigger: Element, label: &'static str) -> Element {
    trigger
        .min_h(38.0)
        .px_3()
        .py_2()
        .rounded_lg()
        .border(1.0, Color::rgba8(127, 127, 127, 90))
        .bg(Color::rgba8(127, 127, 127, 22))
        .hover(|style| style.bg(Color::rgba8(10, 132, 255, 35)))
        .active(|style| style.bg(Color::rgba8(10, 132, 255, 56)))
        .focus(|style| style.border(2.0, Color::rgb8(10, 132, 255)))
        .child(text(label).font_medium())
}

fn gallery_popover(popover: Element, minimum_width: f32, palette: GalleryPalette) -> Element {
    popover
        .min_w(minimum_width)
        .flex_col()
        .p_2()
        .rounded_lg()
        .border(1.0, palette.popover_border)
        .bg(palette.popover)
        .text_color(palette.foreground)
        .shadow(palette.popover_shadow)
}

fn popover_item(
    id: &'static str,
    label: &'static str,
    listener: quickgui::ClickListener<PopoverGallery>,
) -> Element {
    button()
        .id(id)
        .on_click(listener)
        .cursor_default()
        .w_full()
        .min_h(38.0)
        .px_3()
        .py_2()
        .rounded_md()
        .flex_row()
        .items_center()
        .hover(|style| style.bg(Color::rgba8(10, 132, 255, 34)))
        .active(|style| style.bg(Color::rgba8(10, 132, 255, 55)))
        .focus(|style| style.border(2.0, Color::rgb8(10, 132, 255)))
        .child(text(label))
}
