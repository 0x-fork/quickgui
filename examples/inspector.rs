#[cfg(feature = "inspector")]
mod enabled {
    use quickgui::{
        Application, Color, Element, Event, EventContext, Key, KeyBinding, Menu, MenuItem, View,
        ViewContext, button, div, text,
    };

    quickgui::actions!(inspector_actions, [ToggleInspector]);

    pub fn run() -> Result<(), quickgui::AppError> {
        Application::new()
            .bind_keys([KeyBinding::new("platform-alt-i", ToggleInspector, None)])
            .menus([Menu::new("View").item(MenuItem::action("Toggle Inspector", ToggleInspector))])
            .run(|cx| {
                cx.open_window(
                    quickgui::WindowOptions::new("QuickGUI — Retained-tree inspector")
                        .size(920.0, 620.0)
                        .inspector(true),
                    InspectorExample,
                );
            })
    }

    struct InspectorExample;

    impl InspectorExample {
        fn sample_button(label: &'static str) -> Element {
            button()
                .h(36.0)
                .px_3()
                .flex_row()
                .items_center()
                .justify_center()
                .rounded_md()
                .border(1.0, Color::rgb8(72, 82, 98))
                .bg(Color::rgb8(38, 44, 55))
                .hover(|style| style.bg(Color::rgb8(50, 59, 73)))
                .child(text(label).text_sm().font_medium())
        }
    }

    impl View for InspectorExample {
        fn event(&mut self, event: &Event, cx: &mut EventContext) {
            if matches!(
                event,
                Event::KeyDown {
                    key: Key::Escape,
                    ..
                }
            ) {
                cx.exit();
            }
        }

        fn render(&mut self, cx: &mut ViewContext<'_, Self>) -> impl quickgui::IntoElement {
            let inspector_open = cx.window_state().inspector_active;
            let toggle =
                cx.action_listener("inspector-example", |_this, _: &ToggleInspector, cx| {
                    cx.toggle_inspector()
                        .expect("window-owned inspector command");
                });
            let toggle_button = cx.listener("toggle-inspector", |_this, cx| {
                cx.dispatch_action(ToggleInspector);
            });

            let rows = (0_usize..24).map(|index| {
                div()
                    .id(format!("row-{index}"))
                    .h(34.0)
                    .px_3()
                    .flex_row()
                    .items_center()
                    .justify_between()
                    .border(1.0, Color::rgb8(42, 48, 59))
                    .child(text(format!("Retained row {index}")))
                    .child(
                        text(if index.is_multiple_of(3) {
                            "interactive metadata"
                        } else {
                            "paint-only"
                        })
                        .text_xs()
                        .text_color(Color::rgb8(133, 148, 170)),
                    )
            });

            div()
                .id("inspector-example")
                .focusable()
                .auto_focus()
                .on_action(toggle)
                .size_full()
                .flex_col()
                .bg(Color::rgb8(17, 20, 25))
                .text_color(Color::rgb8(226, 232, 240))
                .child(
                    div()
                        .id("toolbar")
                        .h(58.0)
                        .px_4()
                        .flex_row()
                        .items_center()
                        .justify_between()
                        .border(1.0, Color::rgb8(45, 52, 64))
                        .child(
                            div()
                                .flex_col()
                                .child(text("Retained-tree inspector").text_lg().font_semibold())
                                .child(
                                    text("Cmd-Option-I toggles · click freezes · wheel changes occluded depth")
                                        .text_xs()
                                        .text_color(Color::rgb8(139, 154, 177)),
                                ),
                        )
                        .child(
                            Self::sample_button(if inspector_open {
                                "Close inspector"
                            } else {
                                "Open inspector"
                            })
                            .id("toggle-inspector")
                            .on_click(toggle_button),
                        ),
                )
                .child(
                    div()
                        .flex_1()
                        .min_h(0.0)
                        .p_4()
                        .flex_row()
                        .gap_4()
                        .child(
                            div()
                                .id("scroll-column")
                                .w(390.0)
                                .min_h(0.0)
                                .overflow_y_scroll()
                                .rounded_lg()
                                .border(1.0, Color::rgb8(49, 57, 70))
                                .children(rows),
                        )
                        .child(
                            div()
                                .id("paint-stack")
                                .relative()
                                .flex_1()
                                .min_w(0.0)
                                .rounded_lg()
                                .border(1.0, Color::rgb8(49, 57, 70))
                                .bg(Color::rgb8(23, 27, 34))
                                .child(
                                    div()
                                        .id("base-card")
                                        .absolute()
                                        .left(38.0)
                                        .top(48.0)
                                        .size(260.0, 180.0)
                                        .p_4()
                                        .rounded_xl()
                                        .bg(Color::rgb8(40, 58, 87))
                                        .child(text("Base plane · z 0").font_semibold()),
                                )
                                .child(
                                    div()
                                        .id("raised-card")
                                        .absolute()
                                        .left(105.0)
                                        .top(112.0)
                                        .size(260.0, 180.0)
                                        .z_index(4)
                                        .p_4()
                                        .rounded_xl()
                                        .border(1.0, Color::rgb8(132, 204, 255))
                                        .bg(Color::rgba8(32, 88, 119, 238))
                                        .accessibility_label("Raised overlapping card")
                                        .child(text("Raised · z 4").font_semibold())
                                        .child(
                                            text("Hover the overlap and use the wheel to inspect the card behind this one.")
                                                .text_sm(),
                                        ),
                                ),
                        ),
                )
        }
    }
}

#[cfg(feature = "inspector")]
fn main() -> Result<(), quickgui::AppError> {
    enabled::run()
}

#[cfg(not(feature = "inspector"))]
fn main() {
    eprintln!("run this example with `cargo run --features inspector --example inspector`");
}
