//! Caller-styled gallery for QuickGUI's newest unstyled Base UI-derived components.
//!
//! Every colour, radius, size, and transition below belongs to this example. The framework
//! supplies only roles, relationships, state, keyboard and pointer behavior, focus, dismissal,
//! placement, and exact one-shot deadlines.
//!
//! Run with `cargo run --release --example base_ui_components`.

use web_time::{Duration, Instant};

use quickgui::{
    Application, Avatar, AvatarLoadingStatus, AvatarState, CheckboxGroup, CheckboxGroupState,
    Color, Drawer, DrawerState, Element, IntoElement, NavigationMenu, NavigationMenuItem,
    NavigationMenuState, OtpField, OtpFieldState, PreviewCard, PreviewCardState, ScrollArea,
    ScrollAreaOrientation, ScrollAreaState, Separator, Size, StateAccessor, SwipeDirection,
    ToggleState, View, ViewContext, WindowOptions, button, div, navigation_menu_key_bindings,
    otp_field_key_bindings, text, text_input,
};

const SCROLL_VIEWPORT: Size = Size {
    width: 260.0,
    height: 160.0,
};
const SCROLL_CONTENT: Size = Size {
    width: 260.0,
    height: 900.0,
};
const SCROLL_TRACK: f32 = 160.0;
const DRAWER_EXTENT: f32 = 260.0;

fn main() -> Result<(), quickgui::AppError> {
    Application::new()
        .bind_keys(otp_field_key_bindings())
        .bind_keys(navigation_menu_key_bindings())
        .run(|cx| {
            cx.open_window(
                WindowOptions::new("QuickGUI — Base UI components").size(920.0, 820.0),
                BaseUiGallery::new(),
            );
        })
}

#[derive(Clone, Copy)]
struct Palette {
    background: Color,
    surface: Color,
    raised: Color,
    border: Color,
    foreground: Color,
    muted: Color,
    accent: Color,
    scrim: Color,
}

impl Palette {
    fn dark() -> Self {
        Self {
            background: Color::rgb8(17, 18, 21),
            surface: Color::rgb8(26, 28, 33),
            raised: Color::rgb8(36, 39, 46),
            border: Color::rgb8(55, 59, 69),
            foreground: Color::rgb8(238, 240, 244),
            muted: Color::rgb8(155, 161, 174),
            accent: Color::rgb8(10, 132, 255),
            scrim: Color::rgba8(0, 0, 0, 140),
        }
    }
}

struct BaseUiGallery {
    #[cfg(target_arch = "wasm32")]
    docs_component: String,
    palette: Palette,
    avatar: AvatarState,
    colors: CheckboxGroupState,
    card: PreviewCardState,
    log: ScrollAreaState,
    code: OtpFieldState,
    sheet: DrawerState,
    nav: NavigationMenuState,
    status: String,
}

impl BaseUiGallery {
    fn new() -> Self {
        Self {
            #[cfg(target_arch = "wasm32")]
            docs_component: String::new(),
            palette: Palette::dark(),
            avatar: AvatarState::new().delay(Duration::from_millis(200)),
            colors: CheckboxGroupState::new(["red", "green", "blue"]).checked(["green"]),
            card: PreviewCardState::new(),
            log: ScrollAreaState::new().overflow_edge_threshold(2.0),
            code: OtpFieldState::new(6).required(true),
            sheet: DrawerState::new(SwipeDirection::Down).snap_points(&[0.45, 1.0]),
            nav: NavigationMenuState::new(),
            status: "Ready".to_owned(),
        }
    }

    fn avatar_state(view: &mut Self) -> &mut AvatarState {
        &mut view.avatar
    }

    fn colors(view: &mut Self) -> &mut CheckboxGroupState {
        &mut view.colors
    }

    fn card(view: &mut Self) -> &mut PreviewCardState {
        &mut view.card
    }

    fn code(view: &mut Self) -> &mut OtpFieldState {
        &mut view.code
    }

    fn sheet(view: &mut Self) -> &mut DrawerState {
        &mut view.sheet
    }

    fn nav(view: &mut Self) -> &mut NavigationMenuState {
        &mut view.nav
    }

    fn section(&self, title: &'static str, body: Element) -> Element {
        div()
            .flex_col()
            .gap(10.0)
            .p(16.0)
            .rounded(12.0)
            .bg(self.palette.surface)
            .border(1.0, self.palette.border)
            .child(
                text(title)
                    .text_sm()
                    .font_semibold()
                    .text_color(self.palette.muted),
            )
            .child(body)
    }

    fn control(&self, label: &str) -> Element {
        text(label.to_owned())
            .text_sm()
            .text_color(self.palette.foreground)
    }
}

impl View for BaseUiGallery {
    fn render(&mut self, cx: &mut ViewContext<'_, Self>) -> impl IntoElement {
        // Exactly one poll per frame retires any elapsed one-shot deadline; a settled gallery
        // schedules nothing and the window sleeps.
        let now = Instant::now();
        let _ = self.avatar.poll(now);
        let _ = self.card.poll(now);
        let _ = self.nav.poll(now);
        Avatar::schedule(cx, &self.avatar);
        PreviewCard::schedule(cx, &self.card);
        NavigationMenu::schedule(cx, &self.nav);
        self.log.set_geometry(SCROLL_VIEWPORT, SCROLL_CONTENT);

        let palette = self.palette;
        #[cfg(target_arch = "wasm32")]
        if !self.docs_component.is_empty() {
            let content = match self.docs_component.as_str() {
                "avatar" => self.avatar_section(cx),
                "checkbox-group" => self.checkbox_group_section(cx),
                "preview-card" => self.preview_card_section(cx),
                "scroll-area" => self.scroll_area_section(cx),
                "otp-field" => self.otp_section(cx),
                _ => self.navigation_section(cx),
            };
            return div()
                .size_full()
                .p(20.0)
                .flex_col()
                .items_center()
                .justify_center()
                .bg(palette.background)
                .text_color(palette.foreground)
                .child(content);
        }

        let body = div()
            .flex_col()
            .gap(16.0)
            .p(20.0)
            .size_full()
            .bg(palette.background)
            .child(self.navigation_section(cx))
            .child(
                div()
                    .flex_row()
                    .gap(16.0)
                    .child(self.avatar_section(cx))
                    .child(self.checkbox_group_section(cx))
                    .child(self.preview_card_section(cx)),
            )
            .child(
                div()
                    .flex_row()
                    .gap(16.0)
                    .child(self.scroll_area_section(cx))
                    .child(self.otp_section(cx))
                    .child(self.drawer_section(cx)),
            )
            .child(
                text(self.status.clone())
                    .text_sm()
                    .text_color(palette.muted),
            );

        div().size_full().child(body).child(self.drawer_layer(cx))
    }
}

impl BaseUiGallery {
    fn navigation_section(&mut self, cx: &mut ViewContext<'_, Self>) -> Element {
        let palette = self.palette;
        let items = [
            NavigationMenuItem::new("products"),
            NavigationMenuItem::new("solutions"),
            NavigationMenuItem::new("support").disabled(true),
        ];
        let menu = NavigationMenu::new("main-nav", &self.nav, &items);
        let mut list = menu.list_part(div().flex_row().gap(4.0));
        let mut panels = div();
        for item in items.iter() {
            let entry = menu.entry(item.value()).expect("declared item");
            let open = entry.is_open();
            let click = entry.on_trigger_click(cx, Self::nav, |view, value, _| {
                view.status = match value {
                    Some(value) => format!("Navigation menu opened {value:?}"),
                    None => "Navigation menu closed".to_owned(),
                };
            });
            let hover = entry.on_trigger_hover(cx, Self::nav, |_, _, _| {});
            let popup_hover = entry.on_popup_hover(cx, Self::nav, |_, _, _| {});
            let dismiss = entry.on_dismiss(cx, Self::nav, |view, _, _| {
                view.status = "Navigation menu dismissed".to_owned();
            });
            let trigger = entry.key_part(
                cx,
                entry
                    .trigger_part(
                        button()
                            .px(12.0)
                            .py(6.0)
                            .rounded(8.0)
                            .bg(if open {
                                palette.raised
                            } else {
                                palette.surface
                            })
                            .border(1.0, palette.border)
                            .child(text(format!("{:?}", item.value())).text_sm().text_color(
                                if item.is_disabled() {
                                    palette.muted
                                } else {
                                    palette.foreground
                                },
                            )),
                    )
                    .on_click(click)
                    .on_hover(hover),
                Self::nav,
            );
            list = list.child(entry.item_part(div()).child(trigger));
            if open {
                panels = panels.child(
                    entry.positioner_part(div()).child(
                        entry
                            .popup_part(
                                div()
                                    .w(260.0)
                                    .p(12.0)
                                    .rounded(10.0)
                                    .bg(palette.raised)
                                    .border(1.0, palette.border),
                            )
                            .on_hover(popup_hover)
                            .on_dismiss(dismiss)
                            .child(
                                entry.viewport_part(div()).child(
                                    entry
                                        .content_part(div().flex_col().gap(6.0))
                                        .child(self.control("Overview"))
                                        .child(menu.link_part(
                                            "pricing",
                                            false,
                                            text("Pricing").text_sm().text_color(palette.accent),
                                        )),
                                ),
                            ),
                    ),
                );
            }
        }
        self.section(
            "Navigation menu",
            div()
                .relative()
                .child(menu.root_part(div()).child(list))
                .child(panels),
        )
    }

    fn avatar_section(&mut self, cx: &mut ViewContext<'_, Self>) -> Element {
        let palette = self.palette;
        let avatar = Avatar::new("member", "Ada Lovelace");
        let cycle = cx.listener("cycle-avatar", |view, cx| {
            let next = match view.avatar.loading_status() {
                AvatarLoadingStatus::Idle => AvatarLoadingStatus::Loading,
                AvatarLoadingStatus::Loading => AvatarLoadingStatus::Loaded,
                AvatarLoadingStatus::Loaded => AvatarLoadingStatus::Error,
                AvatarLoadingStatus::Error => AvatarLoadingStatus::Idle,
            };
            Avatar::apply_loading_status(
                view,
                cx,
                &StateAccessor::new(Self::avatar_state),
                next,
                Instant::now(),
                |view, status, cx| {
                    view.status = format!("Avatar loading status: {status:?}");
                    cx.invalidate();
                },
            );
        });

        let mut face = avatar.root_part(
            div()
                .size(48.0, 48.0)
                .rounded_full()
                .items_center()
                .justify_center()
                .bg(palette.raised)
                .border(1.0, palette.border),
        );
        if self.avatar.shows_image() {
            face =
                face.child(avatar.image_part(div().size_full().rounded_full().bg(palette.accent)));
        }
        if self.avatar.shows_fallback() {
            face = face
                .child(avatar.fallback_part(text("AL").text_sm().text_color(palette.foreground)));
        }

        self.section(
            "Avatar",
            div()
                .flex_col()
                .gap(10.0)
                .child(face)
                .child(
                    button()
                        .id("cycle-avatar")
                        .px(10.0)
                        .py(6.0)
                        .rounded(8.0)
                        .bg(palette.raised)
                        .border(1.0, palette.border)
                        .child(self.control("Cycle status"))
                        .on_click(cycle),
                )
                .child(Separator::horizontal().root_part(div().h(1.0).w_full().bg(palette.border)))
                .child(self.control(&format!("{:?}", self.avatar.loading_status()))),
        )
    }

    fn checkbox_group_section(&mut self, cx: &mut ViewContext<'_, Self>) -> Element {
        let palette = self.palette;
        let group = CheckboxGroup::new("colors", &self.colors);
        let parent = group.on_parent_click(cx, Self::colors, |view, values, _| {
            view.status = format!("Checkbox group: {} checked", values.len());
        });
        let mut root = group.root_part(div().flex_col().gap(8.0)).child(
            group
                .parent_part(div().flex_row().gap(8.0).items_center())
                .on_click(parent)
                .child(group.indicator_part(indicator_box(&palette, self.colors.parent_state())))
                .child(self.control("All colours")),
        );
        for value in ["red", "green", "blue"] {
            let checked = self.colors.is_checked(value);
            let click = group.on_checkbox_click(cx, value, Self::colors, |view, values, _| {
                view.status = format!("Checkbox group: {} checked", values.len());
            });
            root = root.child(
                group
                    .checkbox_part(value, div().flex_row().gap(8.0).items_center().ml(16.0))
                    .on_click(click)
                    .child(
                        group.indicator_part(indicator_box(&palette, ToggleState::from(checked))),
                    )
                    .child(self.control(value)),
            );
        }
        self.section("Checkbox group", root)
    }

    fn preview_card_section(&mut self, cx: &mut ViewContext<'_, Self>) -> Element {
        let palette = self.palette;
        let card = PreviewCard::from_state("profile-link", "profile-card", &self.card);
        let trigger_hover = card.on_trigger_hover(cx, Self::card, |view, open, _| {
            view.status = format!("Preview card open: {open}");
        });
        let popup_hover = card.on_popup_hover(cx, Self::card, |_, _, _| {});
        let dismiss = card.on_dismiss(cx, Self::card, |view, _, _| {
            view.status = "Preview card dismissed".to_owned();
        });

        let mut body = card.root_part(div().relative().flex_col().gap(8.0)).child(
            card.trigger_part(
                text("@ada")
                    .text_sm()
                    .text_color(palette.accent)
                    .underline(),
            )
            .on_hover(trigger_hover),
        );
        if card.is_open() {
            body = body.child(
                card.positioner_part(div()).child(
                    card.popup_part(
                        div()
                            .w(220.0)
                            .p(12.0)
                            .flex_col()
                            .gap(6.0)
                            .rounded(10.0)
                            .bg(palette.raised)
                            .border(1.0, palette.border),
                    )
                    .on_hover(popup_hover)
                    .on_dismiss(dismiss)
                    .child(self.control("Ada Lovelace"))
                    .child(
                        text("Mathematician and first programmer.")
                            .text_xs()
                            .text_color(palette.muted),
                    )
                    .child(card.arrow_part(div().size(8.0, 8.0).bg(palette.raised))),
                ),
            );
        }
        self.section("Preview card", body)
    }

    fn scroll_area_section(&mut self, cx: &mut ViewContext<'_, Self>) -> Element {
        let palette = self.palette;
        let area = ScrollArea::new("log");
        let offset = self.log.offset();
        let style = self.log.style_state();
        let wheel = cx.scroll_wheel_listener(area.viewport_id(), |view, event, cx| {
            if view.log.apply_scroll_wheel(event) {
                cx.invalidate();
            }
        });
        let drag = cx.pointer_listener(
            area.thumb_id(ScrollAreaOrientation::Vertical),
            |view, event, cx| {
                if view.log.apply_thumb_pointer(
                    event,
                    ScrollAreaOrientation::Vertical,
                    SCROLL_TRACK,
                ) {
                    cx.invalidate();
                }
            },
        );
        let track = cx.pointer_listener(
            area.scrollbar_id(ScrollAreaOrientation::Vertical),
            |view, event, cx| {
                if view.log.apply_track_pointer(
                    event,
                    ScrollAreaOrientation::Vertical,
                    SCROLL_TRACK,
                ) {
                    cx.invalidate();
                }
            },
        );

        let mut lines = area
            .content_part(div().flex_col().gap(4.0).w_full())
            .translate(0.0, -offset.y);
        for index in 0..40 {
            lines = lines.child(
                text(format!("line {index:02}"))
                    .text_xs()
                    .text_color(palette.muted),
            );
        }

        let mut row = div().flex_row().gap(6.0).child(
            area.viewport_part(
                div()
                    .size(SCROLL_VIEWPORT.width, SCROLL_VIEWPORT.height)
                    .p(8.0)
                    .rounded(8.0)
                    .bg(palette.raised)
                    .border(
                        1.0,
                        if style.overflow_y_start {
                            palette.accent
                        } else {
                            palette.border
                        },
                    )
                    .on_scroll_wheel(wheel),
            )
            .child(lines),
        );
        if area.shows_scrollbar(&self.log, ScrollAreaOrientation::Vertical) {
            row = row.child(
                area.scrollbar_part(
                    &self.log,
                    ScrollAreaOrientation::Vertical,
                    div()
                        .w(8.0)
                        .h(SCROLL_TRACK)
                        .relative()
                        .rounded_full()
                        .bg(palette.surface)
                        .on_pointer(track),
                )
                .child(
                    area.thumb_part(
                        ScrollAreaOrientation::Vertical,
                        div()
                            .absolute()
                            .w(8.0)
                            .h(self
                                .log
                                .thumb_length(ScrollAreaOrientation::Vertical, SCROLL_TRACK))
                            .rounded_full()
                            .bg(palette.muted)
                            .translate(
                                0.0,
                                self.log
                                    .thumb_offset(ScrollAreaOrientation::Vertical, SCROLL_TRACK),
                            )
                            .on_pointer(drag),
                    ),
                ),
            );
        }

        self.section(
            "Scroll area",
            div()
                .flex_col()
                .gap(8.0)
                .child(area.root_part(row))
                .child(self.control(&format!(
                    "start {} · end {}",
                    style.overflow_y_start, style.overflow_y_end
                ))),
        )
    }

    fn otp_section(&mut self, cx: &mut ViewContext<'_, Self>) -> Element {
        let palette = self.palette;
        let field = OtpField::new("code");
        let mut row = field.root_part(&self.code, div().flex_row().items_center().gap(6.0));
        for index in 0..self.code.length() {
            if index == 3 {
                row = row.child(
                    field.separator_part(index - 1, text("–").text_sm().text_color(palette.muted)),
                );
            }
            row = row.child(
                field.slot_part(
                    cx,
                    &self.code,
                    index,
                    text_input(self.code.slot_text(index))
                        .w(34.0)
                        .h(40.0)
                        .rounded(8.0)
                        .text_center()
                        .bg(palette.raised)
                        .border(1.0, palette.border)
                        .text_color(palette.foreground),
                    Self::code,
                    |view, value, _| {
                        view.status = format!("Code: {value}");
                    },
                    |view, value, _| {
                        view.status = format!("Code complete: {value}");
                    },
                ),
            );
        }
        self.section(
            "OTP field",
            div()
                .flex_col()
                .gap(8.0)
                .child(row)
                .child(self.control(if self.code.is_complete() {
                    "Complete"
                } else {
                    "Enter six digits"
                })),
        )
    }

    fn drawer_section(&mut self, cx: &mut ViewContext<'_, Self>) -> Element {
        let palette = self.palette;
        let drawer = Drawer::from_state("filters", &self.sheet);
        let open = cx.listener(drawer.trigger_id(), move |view, cx| {
            if view.sheet.open() {
                view.status = "Drawer opened".to_owned();
                drawer.focus_initial(cx);
                cx.invalidate();
            }
        });
        self.section(
            "Drawer",
            drawer
                .trigger_part(
                    button()
                        .px(12.0)
                        .py(8.0)
                        .rounded(8.0)
                        .bg(palette.raised)
                        .border(1.0, palette.border)
                        .child(self.control("Open filters")),
                )
                .on_click(open),
        )
    }

    fn drawer_layer(&mut self, cx: &mut ViewContext<'_, Self>) -> Element {
        let palette = self.palette;
        let drawer = Drawer::from_state("filters", &self.sheet)
            .initial_focus("sheet-first")
            .restore_focus_to(Drawer::new("filters", false).trigger_id());
        let close = cx.listener(drawer.close_id(), move |view, cx| {
            if view.sheet.close() {
                view.status = "Drawer closed".to_owned();
                drawer.focus_restore(cx);
                cx.invalidate();
            }
        });
        let dismiss = drawer.on_dismiss(cx, Self::sheet, |view, _, _| {
            view.status = "Drawer dismissed".to_owned();
        });
        let swipe = drawer.on_swipe(
            cx,
            DRAWER_EXTENT,
            Self::sheet,
            |view, index, _| view.status = format!("Drawer snapped to {index}"),
            |view, _, _| view.status = "Drawer swiped away".to_owned(),
        );

        if !drawer.is_open() {
            return div();
        }
        drawer
            .portal_part(div())
            .child(drawer.backdrop_part(div().bg(palette.scrim)))
            .child(
                drawer.viewport_part(div().flex_col().justify_end()).child(
                    drawer
                        .popup_part(
                            div()
                                .w_full()
                                .h(DRAWER_EXTENT)
                                .flex_col()
                                .gap(10.0)
                                .p(16.0)
                                .rounded_t(16.0)
                                .bg(palette.surface)
                                .border(1.0, palette.border),
                        )
                        .translate(0.0, self.sheet.swipe_offset())
                        .on_dismiss(dismiss)
                        .child(
                            drawer.swipe_area_part(
                                div()
                                    .w(48.0)
                                    .h(5.0)
                                    .mx_auto()
                                    .rounded_full()
                                    .bg(palette.border)
                                    .on_pointer(swipe),
                            ),
                        )
                        .child(
                            drawer.title_part(
                                text("Filters")
                                    .font_semibold()
                                    .text_color(palette.foreground),
                            ),
                        )
                        .child(
                            drawer.description_part(
                                text("Narrow the results")
                                    .text_sm()
                                    .text_color(palette.muted),
                            ),
                        )
                        .child(
                            drawer
                                .content_part(div().flex_col().gap(8.0))
                                .child(button().id("sheet-first").child(self.control("Recent")))
                                .child(button().id("sheet-second").child(self.control("Starred"))),
                        )
                        .child(
                            drawer
                                .close_part(
                                    "Close filters",
                                    div()
                                        .px(12.0)
                                        .py(8.0)
                                        .rounded(8.0)
                                        .bg(palette.raised)
                                        .border(1.0, palette.border)
                                        .child(self.control("Done")),
                                )
                                .on_click(close),
                        ),
                ),
            )
    }
}

fn indicator_box(palette: &Palette, state: ToggleState) -> Element {
    let fill = match state {
        ToggleState::On => palette.accent,
        ToggleState::Mixed => palette.muted,
        ToggleState::Off => palette.surface,
    };
    div()
        .size(16.0, 16.0)
        .rounded(4.0)
        .bg(fill)
        .border(1.0, palette.border)
}

/// Focused presentation of the same native example for browser documentation.
#[cfg(target_arch = "wasm32")]
pub fn docs_demo(component: String) -> impl quickgui::View {
    let mut view = BaseUiGallery::new();
    view.docs_component = component;
    view
}
