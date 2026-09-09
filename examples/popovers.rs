use std::sync::Arc;

use std::time::Duration;

use quickgui::{
    AccessibilityRole, AnchorAlign, AnchorPlacement, AnchorPlacementHandle, AnchorSide,
    Application, BoxShadow, Color, Element, EventContext, IntoElement, MenuState, Popover,
    PopoverHoverState, PopoverKind, PopoverMenu, PopoverMenuItem, View, ViewContext,
    WindowAppearance, button, div, popover_menu_key_bindings, text,
};

/// One command dispatched by the unstyled hover menu.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct MenuCommand(&'static str);

fn main() -> Result<(), quickgui::AppError> {
    Application::new()
        .bind_keys(popover_menu_key_bindings())
        .run(|cx| {
            cx.open_window(
                quickgui::WindowOptions::new("QuickGUI — Controlled popovers").size(900.0, 620.0),
                PopoverGallery::default(),
            );
        })
}

struct PopoverGallery {
    #[cfg(target_arch = "wasm32")]
    docs_component: String,
    account_open: bool,
    nested_open: bool,
    actions_open: bool,
    /// Hover opening is framework behavior on exact one-shot deadlines, not an application timer.
    preview: PopoverHoverState,
    /// The placement QuickGUI resolved for the preview surface on the previous painted frame.
    preview_placement: AnchorPlacementHandle,
    /// Base UI's `Menu.Root`: the controlled open flag plus hover opening on exact deadlines.
    menu: MenuState,
    /// The row model the menu surface wraps.
    menu_model: PopoverMenu,
    status: Option<Arc<str>>,
}

impl Default for PopoverGallery {
    fn default() -> Self {
        Self {
            #[cfg(target_arch = "wasm32")]
            docs_component: String::new(),
            account_open: false,
            nested_open: false,
            actions_open: false,
            preview: PopoverHoverState::new()
                .delay(Duration::from_millis(250))
                .close_delay(Duration::from_millis(120)),
            preview_placement: AnchorPlacementHandle::new(),
            menu: MenuState::new("menu-trigger", "menu-popup")
                .open_on_hover(true)
                .delay(Duration::from_millis(120))
                .close_delay(Duration::from_millis(80))
                .side(AnchorSide::Bottom)
                .align(AnchorAlign::Start)
                .side_offset(6.0),
            menu_model: PopoverMenu::new([
                PopoverMenuItem::group_label("Share"),
                PopoverMenuItem::action("copy", "Copy link", MenuCommand("Copied link")),
                PopoverMenuItem::separator(),
                PopoverMenuItem::action("email", "Email", MenuCommand("Emailed")),
            ])
            .expect("the static hover menu is valid"),
            status: None,
        }
    }
}

impl PopoverGallery {
    fn menu(view: &mut Self) -> &mut MenuState {
        &mut view.menu
    }

    fn menu_model(view: &mut Self) -> &mut PopoverMenu {
        &mut view.menu_model
    }
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
        self.preview.close_now();
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
        // Base UI-shaped positioner props. `side`/`align` are preferences; the arrow follows the
        // placement QuickGUI actually resolved, so it stays on the popup edge that faces the
        // trigger even when the surface flips near a window edge.
        let preview = Popover::new("preview-trigger", "preview-popover", self.preview.is_open())
            .side(AnchorSide::Top)
            .align(AnchorAlign::Center)
            .side_offset(12.0)
            .collision_padding(16.0)
            .arrow_size(12.0)
            .arrow_padding(10.0)
            .track_placement(&self.preview_placement);
        let preview_state = preview.state();

        let toggle_account = cx.listener(account.trigger_id(), move |this, cx| {
            let opening = !this.account_open;
            this.close_all();
            this.account_open = opening;
            if opening {
                account.focus_surface(cx);
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
            cx.invalidate();
        });
        let toggle_nested = cx.listener(nested.trigger_id(), move |this, cx| {
            this.nested_open = !this.nested_open;
            if this.nested_open {
                nested.focus_surface(cx);
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
            }
            cx.invalidate();
        });
        let dismiss_actions = cx.dismiss_listener(actions.surface_id(), |this, cx| {
            this.actions_open = false;
            cx.invalidate();
        });
        let profile = cx.listener("profile", move |this, cx| {
            this.select("Profile selected", cx);
        });
        let preferences = cx.listener("preferences", move |this, cx| {
            this.select("Preferences selected", cx);
        });
        let shortcut = cx.listener("shortcut", move |this, cx| {
            this.select("Keyboard shortcut changed", cx);
        });
        let duplicate = cx.listener("duplicate", move |this, cx| {
            this.select("Duplicate selected", cx);
        });
        let archive = cx.listener("archive", move |this, cx| {
            this.select("Archive selected", cx);
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
                    )
                    .child(
                        gallery_trigger(
                            self.preview.trigger_part_with(
                                cx,
                                preview,
                                preview_access(),
                                div(),
                            ),
                            "Hover for details",
                        )
                        .absolute()
                        .left(24.0)
                        .bottom(24.0)
                        .accessibility_label("Release details"),
                    ),
            );

        if preview.is_open() {
            let popup = gallery_popover(
                self.preview.popup_part_with(cx, preview, preview_access(), div()),
                240.0,
                palette,
            )
            .relative()
            .gap_2()
            .child(preview.title_part(text("Release 0.1").font_semibold()))
            .child(
                preview.description_part(
                    text("Hover opening, the grace interval back to this surface, and the arrow edge are all framework behavior.")
                        .wrap()
                        .text_sm()
                        .text_color(palette.muted),
                ),
            )
            .child(
                text(format!(
                    "resolved side {:?}, align {:?}, available height {:.0}",
                    preview_state.side, preview_state.align, preview_state.available_height
                ))
                .text_sm()
                .text_color(palette.muted),
            )
            .child(
                preview.arrow_part(div()).w(12.0).h(12.0).rotate_degrees(45.0).bg(palette.popover),
            );
            root = root.child(
                preview.tracked_positioner_part(div().child(popup), &self.preview_placement),
            );
        }

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

        // Base UI's Menu.Root/Trigger/Positioner/Popup over the same in-window popover, opened on
        // hover after an exact 120 ms deadline.
        let menu_command =
            cx.action_listener("menu-commands", |this, command: &MenuCommand, cx| {
                this.status = Some(Arc::from(command.0));
                cx.invalidate();
            });
        let menu_trigger = self.menu.trigger_part(
            cx,
            Self::menu,
            |_view: &mut Self, _open, _cx| {},
            gallery_trigger(div(), "Share"),
        );
        root = root.child(menu_trigger).on_action(menu_command);
        if self.menu.is_open() {
            let rows = self.menu_model.element(
                cx,
                "share-menu",
                Self::menu_model,
                div().flex_col().gap_1(),
                |item, state| {
                    div()
                        .min_h(30.0)
                        .px_2()
                        .rounded_md()
                        .flex_row()
                        .items_center()
                        .opacity(if state.disabled { 0.45 } else { 1.0 })
                        .bg(if state.highlighted {
                            Color::rgba8(10, 132, 255, 34)
                        } else {
                            Color::TRANSPARENT
                        })
                        .child(text(item.label().clone()))
                },
                |view: &mut Self, cx| {
                    view.menu.close_now();
                    cx.invalidate();
                },
            );
            let popup = gallery_popover(
                self.menu
                    .popup_part(cx, Self::menu, |_view: &mut Self, _open, _cx| {}, div()),
                200.0,
                palette,
            )
            .child(rows);
            root = root.child(self.menu.positioner_part(div().child(popup)));
        }

        root
    }
}

/// One per-instance accessor to the gallery's hover state.
///
/// A view that owns a single popover can pass the `fn` pointer form instead; this shows the
/// accessor entry point a host with many declared popovers uses.
fn preview_access() -> quickgui::StateAccessor<PopoverGallery, PopoverHoverState> {
    quickgui::StateAccessor::new(|view: &mut PopoverGallery| &mut view.preview)
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

/// Browser docs reuse this interactive native gallery.
#[cfg(target_arch = "wasm32")]
pub fn docs_demo(component: String) -> impl quickgui::View {
    let mut view = PopoverGallery::default();
    view.docs_component = component;
    view
}
