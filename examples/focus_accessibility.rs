use std::sync::Arc;

use quickgui::{
    App, Color, Element, ElementId, Event, EventContext, View, ViewContext, button, div, text,
    text_input,
};

fn main() -> Result<(), quickgui::AppError> {
    App::new(FocusDemo::new())
        .title("QuickGUI — Focus and accessibility")
        .size(680.0, 460.0)
        .run()
}

struct FocusDemo {
    count: usize,
    name: Arc<str>,
    status: Arc<str>,
}

impl FocusDemo {
    fn new() -> Self {
        Self {
            count: 0,
            name: Arc::from(""),
            status: Arc::from("Type Unicode text, then Tab through the controls."),
        }
    }

    fn control(content: impl Into<Arc<str>>) -> Element {
        button()
            .min_h(40.0)
            .flex_row()
            .items_center()
            .justify_center()
            .px_4()
            .py_2()
            .rounded_lg()
            .border(1.0, Color::rgb8(70, 74, 85))
            .bg(Color::rgb8(37, 40, 48))
            .hover(|style| style.bg(Color::rgb8(48, 53, 64)))
            .active(|style| style.bg(Color::rgb8(31, 82, 126)))
            .focus(|style| style.border(2.0, Color::rgb8(94, 234, 212)))
            .child(text(content).font_medium())
    }
}

impl View for FocusDemo {
    fn event(&mut self, event: &Event, cx: &mut EventContext) {
        match event {
            Event::FocusChanged(Some(id)) => {
                self.status = Arc::from(format!("Keyboard focus: 0x{:016x}", id.as_u64()));
                cx.invalidate();
            }
            Event::FocusChanged(None) => {
                self.status = Arc::from("Keyboard focus cleared");
                cx.invalidate();
            }
            Event::KeyDown {
                key: quickgui::Key::Escape,
                ..
            } => cx.exit(),
            _ => {}
        }
    }

    fn render(&mut self, cx: &mut ViewContext<'_, Self>) -> impl quickgui::IntoElement {
        let increment_focus = cx.focus_handle("increment");
        let reset_focus = cx.focus_handle("reset");

        let increment = cx.listener(increment_focus.id(), |this, cx| {
            this.count += 1;
            this.status = Arc::from(format!("Incremented to {}", this.count));
            cx.invalidate();
        });
        let reset = cx.listener(reset_focus.id(), move |this, cx| {
            this.count = 0;
            this.status = Arc::from("Reset to zero; focus returned to Increment");
            cx.focus(increment_focus);
            cx.invalidate();
        });
        let edit_name = cx.input_listener("name", |this, value, cx| {
            this.name = Arc::from(value);
            this.status = Arc::from(format!("Edited {} UTF-8 bytes", value.len()));
            cx.invalidate();
        });

        div()
            .size_full()
            .flex_col()
            .gap_4()
            .p_4()
            .bg(Color::rgb8(18, 19, 22))
            .text_color(Color::rgb8(232, 234, 239))
            .child(
                text("Native focus and accessibility")
                    .accessibility_role(quickgui::AccessibilityRole::Heading)
                    .text_2xl()
                    .font_bold(),
            )
            .child(
                text("The field supports selection, macOS clipboard shortcuts, Unicode graphemes, and IME composition. Buttons activate with Space and Return.")
                    .text_sm()
                    .text_color(Color::rgb8(165, 170, 181)),
            )
            .child(
                text_input(self.name.clone())
                    .on_input(edit_name)
                    .placeholder("Type a name, emoji, or CJK text…")
                    .accessibility_label("Name")
                    .w_full()
                    .max_w(480.0)
                    .auto_focus(),
            )
            .child(
                div()
                    .flex_row()
                    .gap_3()
                    .child(
                        Self::control("Increment")
                            .on_click(increment),
                    )
                    .child(Self::control("Reset and focus Increment").on_click(reset))
                    .child(
                        Self::control("Unavailable")
                            .id(ElementId::named("disabled"))
                            .clickable()
                            .disabled(true),
                    ),
            )
            .child(
                div()
                    .flex_col()
                    .gap_2()
                    .p_4()
                    .rounded_lg()
                    .bg(Color::rgb8(25, 27, 32))
                    .child(text(format!("Count: {}", self.count)).text_xl().font_semibold())
                    .child(text(format!("Value: {}", self.name)).text_sm())
                    .child(
                        text(self.status.clone())
                            .text_sm()
                            .text_color(Color::rgb8(126, 231, 212)),
                    ),
            )
            .child(
                text("Escape quits. The native accessibility tree exposes editable value and selection state alongside heading, labels, button state, focus, and actions.")
                    .text_xs()
                    .text_color(Color::rgb8(132, 137, 148)),
            )
    }
}
