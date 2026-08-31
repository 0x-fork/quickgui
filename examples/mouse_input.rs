use std::sync::Arc;

use quickgui::{
    Application, Color, IntoElement, MouseButton, View, ViewContext, WindowOptions, div, text,
};

fn main() -> Result<(), quickgui::AppError> {
    Application::new().run(|cx| {
        cx.open_window(
            WindowOptions::new("QuickGUI — Desktop mouse dispatch")
                .size(760.0, 520.0)
                .minimum_size(560.0, 420.0)
                .background(Color::rgb8(13, 16, 22)),
            MouseDemo::default(),
        );
    })
}

struct MouseDemo {
    status: Arc<str>,
    hovered: bool,
    presses: usize,
}

impl Default for MouseDemo {
    fn default() -> Self {
        Self {
            status: Arc::from("Press the target or click outside the card"),
            hovered: false,
            presses: 0,
        }
    }
}

impl View for MouseDemo {
    fn render(&mut self, cx: &mut ViewContext<'_, Self>) -> impl IntoElement {
        let outside = cx.mouse_down_listener("mouse-card", |this, event, cx| {
            this.status = Arc::from(format!(
                "outside capture: {:?} at {:.0}, {:.0}",
                event.button, event.position.x, event.position.y
            ));
            cx.invalidate();
        });
        let capture = cx.mouse_down_listener("mouse-card", |this, event, cx| {
            this.status = Arc::from(format!("card capture: click count {}", event.click_count));
            cx.invalidate();
        });
        let down = cx.mouse_down_listener("mouse-target", |this, event, cx| {
            this.presses = this.presses.saturating_add(1);
            this.status = Arc::from(format!(
                "target bubble: {:?}, native click count {}",
                event.button, event.click_count
            ));
            cx.invalidate();
        });
        let up = cx.mouse_up_listener("mouse-target", |this, event, cx| {
            this.status = Arc::from(format!(
                "mouse up: {:?}, matching click count {}",
                event.button, event.click_count
            ));
            cx.invalidate();
        });
        let hover = cx.hover_listener("mouse-target", |this, hovered, cx| {
            this.hovered = *hovered;
            cx.invalidate();
        });

        let target_color = if self.hovered {
            Color::rgb8(80, 123, 255)
        } else {
            Color::rgb8(55, 86, 188)
        };

        div()
            .size_full()
            .p(32.0)
            .flex_col()
            .gap_6()
            .text_color(Color::rgb8(237, 241, 248))
            .child(text("Desktop mouse dispatch").text_2xl().font_bold())
            .child(
                text("Outside capture runs first, capture descends to the target, and bubble returns to the root. Double- and triple-click the target to inspect AppKit's exact count.")
                    .max_w(660.0)
                    .wrap()
                    .text_sm()
                    .text_color(Color::rgb8(154, 166, 187)),
            )
            .child(
                div()
                    .id("mouse-card")
                    .w_full()
                    .max_w(560.0)
                    .h(230.0)
                    .p_6()
                    .flex_col()
                    .items_center()
                    .justify_center()
                    .gap_4()
                    .rounded_xl()
                    .bg(Color::rgb8(25, 30, 41))
                    .border(1.0, Color::rgb8(48, 57, 75))
                    .on_mouse_down_out(outside)
                    .capture_any_mouse_down(capture)
                    .child(
                        div()
                            .id("mouse-target")
                            .w(260.0)
                            .h(72.0)
                            .flex_row()
                            .items_center()
                            .justify_center()
                            .rounded_lg()
                            .bg(target_color)
                            .cursor_default()
                            .on_mouse_down(MouseButton::Left, down)
                            .on_mouse_up(MouseButton::Left, up)
                            .on_hover(hover)
                            .child(text("Native multi-click target").font_semibold()),
                    )
                    .child(
                        text(format!("{} presses observed", self.presses))
                            .text_sm()
                            .text_color(Color::rgb8(154, 166, 187)),
                    ),
            )
            .child(
                text(self.status.clone())
                    .max_w(660.0)
                    .wrap()
                    .text_sm()
                    .text_color(Color::rgb8(176, 202, 255)),
            )
    }
}
