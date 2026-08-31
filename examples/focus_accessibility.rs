use std::sync::Arc;

use quickgui::{
    Application, Color, Element, ElementId, Event, EventContext, Fieldset, View, ViewContext,
    button, div, form, text, text_area, text_input,
};

fn main() -> Result<(), quickgui::AppError> {
    Application::new().run(|cx| {
        cx.open_window(
            quickgui::WindowOptions::new("QuickGUI — Focus and accessibility").size(720.0, 680.0),
            FocusDemo::new(),
        );
    })
}

struct FocusDemo {
    count: usize,
    name: Arc<str>,
    email: Arc<str>,
    notes: Arc<str>,
    status: Arc<str>,
}

impl FocusDemo {
    fn new() -> Self {
        Self {
            count: 0,
            name: Arc::from(""),
            email: Arc::from(""),
            notes: Arc::from(
                "QuickGUI text areas use the same retained Cosmic Text layout for painting, hit testing, selection, and caret movement.\n\nUse Option-Left or Option-Right for word navigation. Command-Left and Command-Right move within a logical line, while Command-Up and Command-Down move to the document edges.\n\nWrapped lines support Up, Down, Page Up, Page Down, drag selection, wheel scrolling, and a native-style overlay scrollbar without an idle frame loop.",
            ),
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
            this.status = Arc::from(format!(
                "Edited {} UTF-8 bytes · press Return to submit",
                value.len()
            ));
            cx.invalidate();
        });
        let edit_email = cx.input_listener("email", |this, value, cx| {
            this.email = Arc::from(value);
            this.status = Arc::from("Edited email · Return submits the nearest form");
            cx.invalidate();
        });
        let submit_profile = cx.form_submit_listener("profile", |this, event, cx| {
            let name = event.value("name").unwrap_or_default();
            let email = event.value("email").unwrap_or_default();
            this.status = Arc::from(format!("Submitted profile for {name} <{email}>"));
            cx.invalidate();
        });
        let invalid_profile = cx.form_invalid_listener("profile", |this, report, cx| {
            let first = report
                .first()
                .and_then(|issue| issue.message())
                .unwrap_or("A field needs attention");
            this.status = Arc::from(format!(
                "{} field{} need attention · {first}",
                report.issues().len(),
                if report.issues().len() == 1 { "" } else { "s" }
            ));
            cx.invalidate();
        });
        let edit_notes = cx.input_listener("notes", |this, value, cx| {
            this.notes = Arc::from(value);
            update_editor_status(this, value);
            cx.invalidate();
        });
        let profile_fields = Fieldset::new("profile-fields");
        let name_field = profile_fields
            .field("name")
            .required(true)
            .invalid(self.name.trim().is_empty())
            .dirty(!self.name.is_empty())
            .filled(!self.name.is_empty())
            .validation_message("Name is required");
        let email_field = profile_fields
            .field("email")
            .required(true)
            .invalid(!valid_email(&self.email))
            .dirty(!self.email.is_empty())
            .filled(!self.email.is_empty())
            .validation_message("Enter a complete email address");

        div()
            .size_full()
            .flex_col()
            .gap_4()
            .p_4()
            .overflow_y_scroll()
            .bg(Color::rgb8(18, 19, 22))
            .text_color(Color::rgb8(232, 234, 239))
            .child(
                text("Native focus and accessibility")
                    .accessibility_role(quickgui::AccessibilityRole::Heading)
                    .text_2xl()
                    .font_bold()
                    .flex_none(),
            )
            .child(
                text("The fields support wrapped multiline editing, macOS navigation and deletion shortcuts, Unicode graphemes, constrained IME composition, accessible validation, and Return submission. Buttons activate with Space and Return.")
                    .text_sm()
                    .text_color(Color::rgb8(165, 170, 181))
                    .flex_none(),
            )
            .child(
                form()
                    .on_form_submit(submit_profile)
                    .on_form_invalid(invalid_profile)
                    .flex_col()
                    .flex_none()
                    .gap_3()
                    .child(
                        profile_fields.root_part(
                            div()
                                .flex_col()
                                .gap_3()
                                .child(
                                    profile_fields.legend_part(
                                        text("Profile details").text_lg().font_semibold(),
                                    ),
                                )
                                .child(profile_fields.description_part(
                                    text("These labels focus their controls just like web labels.")
                                        .text_xs()
                                        .text_color(Color::rgb8(165, 170, 181)),
                                ))
                                .child(name_field.root_part(
                                    div()
                                        .flex_col()
                                        .gap_1()
                                        .child(
                                            name_field.label_part(
                                                text("Name").text_sm().font_medium(),
                                            ),
                                        )
                                        .child(name_field.control_part(
                                            text_input(self.name.clone())
                                                .on_input(edit_name)
                                                .max_length(32)
                                                .input_filter(|value| {
                                                    !value
                                                        .chars()
                                                        .any(|character| character == '\t')
                                                })
                                                .placeholder("Type a name, emoji, or CJK text…")
                                                .w_full()
                                                .max_w(480.0)
                                                .auto_focus(),
                                        ))
                                        .child(name_field.description_part(
                                            text("Shown on your public profile.")
                                                .text_xs()
                                                .text_color(Color::rgb8(165, 170, 181)),
                                        ))
                                        .child(name_field.error_part(
                                            text("Name is required")
                                                .text_xs()
                                                .text_color(Color::rgb8(248, 113, 113)),
                                        )),
                                ))
                                .child(email_field.root_part(
                                    div()
                                        .flex_col()
                                        .gap_1()
                                        .child(
                                            email_field.label_part(
                                                text("Email").text_sm().font_medium(),
                                            ),
                                        )
                                        .child(email_field.control_part(
                                            text_input(self.email.clone())
                                                .on_input(edit_email)
                                                .max_length(254)
                                                .placeholder("you@example.com")
                                                .w_full()
                                                .max_w(480.0),
                                        ))
                                        .child(email_field.description_part(
                                            text("Used only for account notifications.")
                                                .text_xs()
                                                .text_color(Color::rgb8(165, 170, 181)),
                                        ))
                                        .child(email_field.error_part(
                                            text("Enter a complete email address")
                                                .text_xs()
                                                .text_color(Color::rgb8(248, 113, 113)),
                                        )),
                                )),
                        ),
                    )
                    .child(
                        div()
                            .flex_col()
                            .gap_2()
                            .child(
                                text("Field and Fieldset contribute no colors, spacing, borders, or typography.")
                                    .text_xs()
                                    .text_color(Color::rgb8(165, 170, 181)),
                            ),
                    )
                    .child(
                        Self::control("Submit profile")
                            .form_submitter()
                            .max_w(180.0),
                    ),
            )
            .child(
                text_area(self.notes.clone())
                    .on_input(edit_notes)
                    .placeholder("Write multiline notes…")
                    .accessibility_label("Notes")
                    .w_full()
                    .max_w(600.0)
                    .h(180.0)
                    .flex_none(),
            )
            .child(
                div()
                    .flex_row()
                    .flex_none()
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
                    .flex_none()
                    .gap_2()
                    .p_4()
                    .rounded_lg()
                    .bg(Color::rgb8(25, 27, 32))
                    .child(text(format!("Count: {}", self.count)).text_xl().font_semibold())
                    .child(text(format!("Value: {}", self.name)).text_sm())
                    .child(text(format!("Email: {}", self.email)).text_sm())
                    .child(
                        text(format!(
                            "Notes: {} lines · {} UTF-8 bytes",
                            self.notes.lines().count().max(1),
                            self.notes.len()
                        ))
                        .text_sm(),
                    )
                    .child(
                        text(self.status.clone())
                            .text_sm()
                            .text_color(Color::rgb8(126, 231, 212)),
                    ),
            )
            .child(
                text("Escape quits. The native accessibility tree exposes editable value and selection state alongside heading, labels, button state, focus, and actions.")
                    .text_xs()
                    .text_color(Color::rgb8(132, 137, 148))
                    .flex_none(),
            )
    }
}

fn valid_email(value: &str) -> bool {
    let Some((local, domain)) = value.trim().split_once('@') else {
        return false;
    };
    !local.is_empty() && domain.contains('.') && !domain.starts_with('.') && !domain.ends_with('.')
}

fn update_editor_status(this: &mut FocusDemo, value: &str) {
    this.status = Arc::from(format!(
        "Edited {} lines · {} UTF-8 bytes",
        value.lines().count().max(1),
        value.len()
    ));
}
