use std::sync::Arc;

use quickgui::{
    Accordion, AccordionItem, AccordionState, App, ClickListener, Collapsible, Color, Element,
    Event, EventContext, View, ViewContext, div, text,
};

fn main() -> Result<(), quickgui::AppError> {
    App::new(DisclosuresDemo::default())
        .title("QuickGUI — Disclosures")
        .size(760.0, 720.0)
        .run()
}

#[derive(Clone, Copy)]
struct Palette {
    background: Color,
    surface: Color,
    border: Color,
    foreground: Color,
    muted: Color,
    hover: Color,
    active: Color,
    focus: Color,
    accent: Color,
}

impl Default for Palette {
    fn default() -> Self {
        Self {
            background: Color::rgb8(17, 18, 21),
            surface: Color::rgb8(27, 29, 34),
            border: Color::rgb8(55, 59, 68),
            foreground: Color::rgb8(238, 240, 244),
            muted: Color::rgb8(158, 164, 177),
            hover: Color::rgba8(255, 255, 255, 10),
            active: Color::rgba8(255, 255, 255, 18),
            focus: Color::rgb8(64, 156, 255),
            accent: Color::rgb8(10, 132, 255),
        }
    }
}

struct DisclosuresDemo {
    recovery_open: bool,
    single: AccordionState,
    multiple: AccordionState,
    status: Arc<str>,
}

impl Default for DisclosuresDemo {
    fn default() -> Self {
        let mut single = AccordionState::new();
        single
            .set_open("rendering", true)
            .expect("one initial single value");
        let mut multiple = AccordionState::new().with_multiple(true);
        multiple
            .replace_open(["keyboard", "resources"])
            .expect("two initial multiple values");
        Self {
            recovery_open: false,
            single,
            multiple,
            status: Arc::from(
                "Tab visits every enabled heading; Enter and Space toggle the focused panel.",
            ),
        }
    }
}

impl DisclosuresDemo {
    fn section(title: &'static str, description: &'static str, palette: Palette) -> Element {
        div()
            .w_full()
            .max_w(680.0)
            .flex_col()
            .gap_3()
            .p_4()
            .rounded_xl()
            .border(1.0, palette.border)
            .bg(palette.surface)
            .child(text(title).text_lg().font_semibold())
            .child(text(description).text_sm().text_color(palette.muted))
    }

    fn trigger(root: Element, open: bool, palette: Palette) -> Element {
        root.w_full()
            .min_h(42.0)
            .flex_row()
            .items_center()
            .justify_between()
            .gap_3()
            .px_3()
            .py_2()
            .rounded_lg()
            .text_color(palette.foreground)
            .hover(|state| state.bg(palette.hover))
            .active(|state| state.bg(palette.active))
            .focus(|state| state.border(2.0, palette.focus))
            .disabled_style(|state| state.opacity(0.42))
            .child(if open { "⌄" } else { "›" })
    }

    fn panel(body: &'static str, palette: Palette) -> Element {
        div()
            .min_w(0.0)
            .p_3()
            .child(text(body).text_sm().text_color(palette.muted))
    }

    fn accordion_item<V>(
        item: AccordionItem,
        title: &'static str,
        body: &'static str,
        listener: ClickListener<V>,
        palette: Palette,
    ) -> Element {
        let open = item.is_open();
        item.root_part(
            div()
                .w_full()
                .flex_col()
                .rounded_lg()
                .border(1.0, palette.border)
                .child(
                    item.header_part(
                        div().child(
                            item.trigger_part(Self::trigger(
                                div().child(text(title).font_medium()),
                                open,
                                palette,
                            ))
                            .on_click(listener),
                        ),
                    ),
                )
                .children(item.panel_part(Self::panel(body, palette))),
        )
    }
}

impl View for DisclosuresDemo {
    fn event(&mut self, event: &Event, cx: &mut EventContext) {
        if matches!(
            event,
            Event::KeyDown {
                key: quickgui::Key::Escape,
                ..
            }
        ) {
            cx.exit();
        }
    }

    fn render(&mut self, cx: &mut ViewContext<'_, Self>) -> impl quickgui::IntoElement {
        let palette = Palette::default();
        let recovery = Collapsible::new("recovery-keys", self.recovery_open);
        let toggle_recovery = cx.listener(recovery.trigger_id(), |view, cx| {
            view.recovery_open = !view.recovery_open;
            view.status = Arc::from(if view.recovery_open {
                "Recovery keys expanded"
            } else {
                "Recovery keys collapsed"
            });
            cx.invalidate();
        });

        let single = Accordion::new("single-accordion").heading_level(3);
        let rendering = single.item_from_state("rendering", 0, &self.single);
        let native = single.item_from_state("native-views", 1, &self.single);
        let memory = single.item_from_state("memory", 2, &self.single);
        let toggle_rendering = cx.listener(rendering.trigger_id(), |view, cx| {
            view.single
                .toggle("rendering")
                .expect("bounded accordion state");
            view.status = Arc::from("Rendering panel toggled");
            cx.invalidate();
        });
        let toggle_native = cx.listener(native.trigger_id(), |view, cx| {
            view.single
                .toggle("native-views")
                .expect("bounded accordion state");
            view.status = Arc::from("Native views panel toggled");
            cx.invalidate();
        });
        let toggle_memory = cx.listener(memory.trigger_id(), |view, cx| {
            view.single
                .toggle("memory")
                .expect("bounded accordion state");
            view.status = Arc::from("Memory panel toggled");
            cx.invalidate();
        });

        let multiple = Accordion::new("multiple-accordion")
            .heading_level(3)
            .keep_mounted(true);
        let keyboard = multiple.item_from_state("keyboard", 0, &self.multiple);
        let resources = multiple.item_from_state("resources", 1, &self.multiple);
        let unavailable = multiple
            .item_from_state("unavailable", 2, &self.multiple)
            .disabled(true);
        let toggle_keyboard = cx.listener(keyboard.trigger_id(), |view, cx| {
            view.multiple
                .toggle("keyboard")
                .expect("bounded accordion state");
            view.status = Arc::from("Keyboard panel toggled independently");
            cx.invalidate();
        });
        let toggle_resources = cx.listener(resources.trigger_id(), |view, cx| {
            view.multiple
                .toggle("resources")
                .expect("bounded accordion state");
            view.status = Arc::from("Resource panel toggled independently");
            cx.invalidate();
        });
        let unavailable_click = cx.listener(unavailable.trigger_id(), |_view, _cx| {});

        let recovery_section = Self::section(
            "Collapsible",
            "A controlled disclosure with caller-owned presentation and default unmounting.",
            palette,
        )
        .child(
            recovery
                .root_part(
                    div().child(
                        recovery
                            .trigger_part(Self::trigger(
                                div().child(text("Recovery keys").font_medium()),
                                recovery.is_open(),
                                palette,
                            ))
                            .on_click(toggle_recovery),
                    ),
                )
                .children(recovery.panel_part(Self::panel(
                    "alien-bean-pasta · wild-irish-burrito · horse-battery-staple",
                    palette,
                ))),
        );

        let single_section = Self::section(
            "Accordion · single value",
            "Opening one panel closes the previously open panel. Every heading remains in Tab order.",
            palette,
        )
        .child(single.root_part(
            div()
                .w_full()
                .flex_col()
                .child(Self::accordion_item(
                    rendering,
                    "How is rendering scheduled?",
                    "Damage drives frames. A settled window does not continuously redraw.",
                    toggle_rendering,
                    palette,
                ))
                .child(Self::accordion_item(
                    native,
                    "Can an NSView be embedded?",
                    "Native children participate in composition, clipping, focus, and teardown.",
                    toggle_native,
                    palette,
                ))
                .child(Self::accordion_item(
                    memory,
                    "What remains retained?",
                    "Only bounded state and currently mounted application elements.",
                    toggle_memory,
                    palette,
                )),
        ));

        let multiple_section = Self::section(
            "Accordion · multiple values",
            "Panels toggle independently. Closed panels in this sample are retained as display: none.",
            palette,
        )
        .child(multiple.root_part(
            div()
                .w_full()
                .flex_col()
                .child(Self::accordion_item(
                    keyboard,
                    "Keyboard contract",
                    "Enter and Space use normal button activation; Tab visits each enabled trigger.",
                    toggle_keyboard,
                    palette,
                ))
                .child(Self::accordion_item(
                    resources,
                    "Resource contract",
                    "Disclosure state creates no timer, observer, task, animation, or idle frame source.",
                    toggle_resources,
                    palette,
                ))
                .child(Self::accordion_item(
                    unavailable,
                    "Disabled heading",
                    "This panel cannot be opened by pointer or keyboard interaction.",
                    unavailable_click,
                    palette,
                )),
        ));

        div()
            .size_full()
            .overflow_y_scroll()
            .bg(palette.background)
            .text_color(palette.foreground)
            .child(
                div()
                    .w_full()
                    .flex_col()
                    .items_center()
                    .gap_4()
                    .p_5()
                    .child(
                        div()
                            .w_full()
                            .max_w(680.0)
                            .flex_row()
                            .items_center()
                            .justify_between()
                            .child(
                                div()
                                    .flex_col()
                                    .gap_1()
                                    .child(text("Unstyled disclosures").text_2xl().font_bold())
                                    .child(
                                        text("The gallery supplies every visible pixel.")
                                            .text_sm()
                                            .text_color(palette.muted),
                                    ),
                            )
                            .child(
                                text("zero idle work")
                                    .text_xs()
                                    .font_semibold()
                                    .text_color(palette.accent),
                            ),
                    )
                    .child(recovery_section)
                    .child(single_section)
                    .child(multiple_section)
                    .child(
                        text(self.status.clone())
                            .w_full()
                            .max_w(680.0)
                            .text_sm()
                            .text_color(palette.muted),
                    ),
            )
    }
}
