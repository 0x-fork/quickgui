use quickgui::{
    AnchorPlacement, Application, Color, Element, IntoElement, PopoverMenu, PopoverMenuItem,
    PopoverMenuItemKind, PopoverMenuItemState, SystemPopover, View, ViewContext, WindowHandle,
    WindowOptions, button, div, popover_menu_key_bindings, text,
};

fn main() -> Result<(), quickgui::AppError> {
    Application::new()
        .bind_keys(popover_menu_key_bindings())
        .run(|cx| {
            cx.open_window(
                WindowOptions::new("QuickGUI — SystemPopover")
                    .size(720.0, 480.0)
                    .background(Color::rgb8(17, 19, 24)),
                PopoverLauncher::default(),
            );
        })
}

#[derive(Default)]
struct PopoverLauncher {
    popover: Option<WindowHandle>,
    opened: u32,
    sidebar: bool,
    last_command: Option<&'static str>,
}

#[derive(Clone, Debug, PartialEq)]
enum DemoCommand {
    NewWindow,
    Open,
    Settings,
    About,
    SetSidebar(bool),
}

impl View for PopoverLauncher {
    fn render(&mut self, cx: &mut ViewContext<'_, Self>) -> impl IntoElement {
        cx.on_any_child_window_closed(|this, closed, cx| {
            if this.popover == Some(closed) {
                this.popover = None;
                cx.invalidate();
            }
        });
        let command = cx.action_listener("popover-launcher", |this, command: &DemoCommand, cx| {
            this.last_command = Some(match command {
                DemoCommand::NewWindow => "New window",
                DemoCommand::Open => "Open",
                DemoCommand::Settings => "Settings",
                DemoCommand::About => "About",
                DemoCommand::SetSidebar(visible) => {
                    this.sidebar = *visible;
                    if *visible {
                        "Show sidebar"
                    } else {
                        "Hide sidebar"
                    }
                }
            });
            this.popover = None;
            cx.invalidate();
        });
        let open = cx.listener("open-native-popover", |this, cx| {
            if let Some(previous) = this.popover.take() {
                cx.close_window_handle(previous);
                cx.invalidate();
                return;
            }
            this.opened = this.opened.saturating_add(1);
            this.popover = Some(
                SystemPopover::new(244.0, 190.0)
                    .gap(6.0)
                    .open(
                        cx,
                        "open-native-popover",
                        "Actions",
                        PopoverMenuView::new(demo_menu(this.sidebar)),
                    )
                    .expect("the mounted trigger can open a native popover"),
            );
            cx.invalidate();
        });

        div()
            .focus_scope(cx.focus_handle("popover-launcher"))
            .on_action(command)
            .size_full()
            .relative()
            .bg(Color::rgb8(17, 19, 24))
            .text_color(Color::rgb8(235, 238, 244))
            .child(
                div()
                    .absolute()
                    .top(34.0)
                    .left(72.0)
                    .w(560.0)
                    .flex_col()
                    .gap_2()
                    .child(text("Real parent-anchored NSPanel").text_2xl().font_bold())
                    .child(
                        text("It follows programmatic parent moves, flips or slides at screen edges, and closes on Escape or an outside press without polling.")
                            .wrap()
                            .text_sm()
                            .text_color(Color::rgb8(157, 167, 185)),
                    ),
            )
            .child(
                button()
                    .absolute()
                    .top(122.0)
                    .right(24.0)
                    .w(190.0)
                    .h(42.0)
                    .flex_row()
                    .items_center()
                    .justify_center()
                    .rounded_lg()
                    .bg(Color::rgb8(65, 105, 225))
                    .hover(|style| style.bg(Color::rgb8(78, 118, 238)))
                    .child(
                        text(if self.popover.is_some() {
                            "Close actions"
                        } else {
                            "Open actions"
                        })
                        .font_medium(),
                    )
                    .on_click(open),
            )
            .child(
                div()
                    .absolute()
                    .top(196.0)
                    .left(72.0)
                    .w(560.0)
                    .p_5()
                    .rounded_xl()
                    .border(1.0, Color::rgb8(53, 58, 69))
                    .bg(Color::rgb8(24, 27, 33))
                    .flex_col()
                    .gap_3()
                    .child(text("What to verify").font_semibold())
                    .child(
                        text("The popover resolves this button's retained bounds by ID—there is no copied trigger rectangle. It can cross the parent edge, while native work-area constraints choose the better side near a display edge. Escape or an outside press dismisses this grabbing menu; passive system popovers remain attached while the parent moves.")
                            .wrap()
                            .text_sm()
                            .text_color(Color::rgb8(170, 179, 196)),
                    )
                    .child(
                        text(format!("Opened {} time(s)", self.opened))
                            .text_xs()
                            .text_color(Color::rgb8(125, 211, 252)),
                    )
                    .child(
                        text(format!(
                            "Sidebar: {} · Last command: {}",
                            if self.sidebar { "shown" } else { "hidden" },
                            self.last_command.unwrap_or("none")
                        ))
                        .text_xs()
                        .text_color(Color::rgb8(125, 211, 252)),
                    ),
            )
    }
}

struct PopoverMenuView {
    menu: PopoverMenu,
    submenu: Option<WindowHandle>,
}

impl PopoverMenuView {
    fn new(menu: PopoverMenu) -> Self {
        Self {
            menu,
            submenu: None,
        }
    }
}

impl View for PopoverMenuView {
    fn render(&mut self, cx: &mut ViewContext<'_, Self>) -> impl IntoElement {
        cx.on_any_child_window_closed(|view, closed, cx| {
            if view.submenu == Some(closed) {
                view.submenu = None;
                cx.invalidate();
            }
        });
        self.menu.element_with_submenus(
            cx,
            "popover-menu",
            |view| &mut view.menu,
            div()
                .size_full()
                .p_2()
                .flex_col()
                .rounded_lg()
                .border(1.0, Color::rgb8(58, 63, 75))
                .bg(Color::rgb8(29, 32, 39))
                .text_color(Color::rgb8(235, 238, 244)),
            menu_item,
            |_view, cx| {
                cx.close_popover_chain();
            },
            |view, anchor, menu, cx| {
                if let Some(previous) = view.submenu.take() {
                    cx.close_window_handle(previous);
                }
                view.submenu = Some(
                    SystemPopover::new(220.0, menu_height(&menu))
                        .placement(AnchorPlacement::RightStart)
                        .gap(2.0)
                        .open(cx, anchor, "More actions", PopoverMenuView::new(menu))
                        .expect("a mounted menu item can anchor a nested popover"),
                );
            },
        )
    }
}

fn demo_menu(sidebar: bool) -> PopoverMenu {
    let more = PopoverMenu::new([
        PopoverMenuItem::action("settings", "Settings", DemoCommand::Settings).shortcut("⌘,"),
        PopoverMenuItem::action("about", "About QuickGUI", DemoCommand::About),
    ])
    .expect("static submenu is valid");
    PopoverMenu::new([
        PopoverMenuItem::group_label("File"),
        PopoverMenuItem::action("new", "New window", DemoCommand::NewWindow).shortcut("⌘N"),
        PopoverMenuItem::action("open", "Open…", DemoCommand::Open).shortcut("⌘O"),
        PopoverMenuItem::separator(),
        PopoverMenuItem::checkbox_with("sidebar", "Show sidebar", sidebar, DemoCommand::SetSidebar),
        PopoverMenuItem::submenu("more", "More", more),
    ])
    .expect("static popover menu is valid")
}

fn menu_item(item: &PopoverMenuItem, state: PopoverMenuItemState) -> Element {
    match item.kind() {
        PopoverMenuItemKind::Separator => div()
            .h(9.0)
            .px_2()
            .child(div().mt(4.0).h(1.0).bg(Color::rgb8(58, 63, 75))),
        PopoverMenuItemKind::GroupLabel => div().h(22.0).px_3().flex_row().items_center().child(
            text(item.label().clone())
                .text_xs()
                .text_color(Color::rgb8(145, 154, 172)),
        ),
        _ => div()
            .w_full()
            .h(34.0)
            .px_3()
            .rounded_md()
            .opacity(if state.disabled { 0.45 } else { 1.0 })
            .bg(if state.highlighted {
                Color::rgb8(62, 70, 88)
            } else {
                Color::TRANSPARENT
            })
            .child(
                div()
                    .size_full()
                    .flex_row()
                    .items_center()
                    .justify_between()
                    .child(
                        div()
                            .flex_row()
                            .items_center()
                            .gap_2()
                            .child(
                                text(if state.checked == Some(true) {
                                    "✓"
                                } else {
                                    ""
                                })
                                .w(12.0)
                                .text_xs(),
                            )
                            .child(text(item.label().clone()).text_sm()),
                    )
                    .child(if state.has_submenu {
                        text("›").text_sm()
                    } else {
                        text(item.shortcut_text().cloned().unwrap_or_else(|| "".into()))
                            .text_xs()
                            .text_color(Color::rgb8(145, 154, 172))
                    }),
            )
            .into_element(),
    }
}

fn menu_height(menu: &PopoverMenu) -> f32 {
    16.0 + menu
        .items()
        .iter()
        .map(|item| match item.kind() {
            PopoverMenuItemKind::Separator => 9.0,
            PopoverMenuItemKind::GroupLabel => 22.0,
            _ => 34.0,
        })
        .sum::<f32>()
}
