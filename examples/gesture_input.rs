use quickgui::{
    App, AppConfig, Color, GesturePhase, IntoElement, Point, PressureStage, TouchId, TouchPhase,
    View, ViewContext, div, text,
};

const MAX_DEMO_TOUCHES: usize = 10;

fn main() -> Result<(), quickgui::AppError> {
    App::new(GestureDemo::default())
        .config(
            AppConfig::new("QuickGUI — Native gesture input")
                .size(820.0, 680.0)
                .minimum_size(640.0, 520.0)
                .background(Color::rgb8(14, 17, 23)),
        )
        .run()
}

#[derive(Clone, Copy, Debug, Default)]
enum LastGesture {
    #[default]
    None,
    Pressure,
    Pinch,
    Rotation,
    SmartMagnify,
    ScrollWheel,
    Touch,
}

struct GestureDemo {
    zoom: f32,
    rotation: f32,
    pressure: f32,
    pressure_stage: PressureStage,
    phase: GesturePhase,
    wheel_delta: f32,
    wheel_precise: bool,
    active_touches: [Option<TouchId>; MAX_DEMO_TOUCHES],
    last_touch_position: Point,
    last_touch_force: Option<f32>,
    last: LastGesture,
    event_count: u64,
}

impl Default for GestureDemo {
    fn default() -> Self {
        Self {
            zoom: 1.0,
            rotation: 0.0,
            pressure: 0.0,
            pressure_stage: PressureStage::Zero,
            phase: GesturePhase::Moved,
            wheel_delta: 0.0,
            wheel_precise: false,
            active_touches: [None; MAX_DEMO_TOUCHES],
            last_touch_position: Point::ZERO,
            last_touch_force: None,
            last: LastGesture::None,
            event_count: 0,
        }
    }
}

impl GestureDemo {
    fn record(&mut self, last: LastGesture) {
        self.last = last;
        self.event_count = self.event_count.saturating_add(1);
    }

    fn update_touch(&mut self, id: TouchId, phase: TouchPhase) {
        let existing = self
            .active_touches
            .iter()
            .position(|candidate| *candidate == Some(id));
        match phase {
            TouchPhase::Started | TouchPhase::Moved => {
                if existing.is_none()
                    && let Some(slot) = self.active_touches.iter_mut().find(|slot| slot.is_none())
                {
                    *slot = Some(id);
                }
            }
            TouchPhase::Ended | TouchPhase::Cancelled => {
                if let Some(index) = existing {
                    self.active_touches[index] = None;
                }
            }
        }
    }

    fn active_touch_count(&self) -> usize {
        self.active_touches
            .iter()
            .filter(|touch| touch.is_some())
            .count()
    }
}

impl View for GestureDemo {
    fn render(&mut self, cx: &mut ViewContext<'_, Self>) -> impl IntoElement {
        // All six typed callbacks intentionally share one stable element identity. An element can
        // compose input capabilities just like one DOM node can own several event listeners.
        let pressure = cx.mouse_pressure_listener("gesture-pad", |this, event, cx| {
            this.pressure = event.pressure;
            this.pressure_stage = event.stage;
            this.record(LastGesture::Pressure);
            cx.invalidate();
        });
        let pinch = cx.pinch_listener("gesture-pad", |this, event, cx| {
            let factor = (1.0 + event.delta).max(0.1);
            this.zoom = (this.zoom * factor).clamp(0.55, 2.25);
            this.phase = event.phase;
            this.record(LastGesture::Pinch);
            cx.invalidate();
        });
        let rotation = cx.rotation_listener("gesture-pad", |this, event, cx| {
            this.rotation = (this.rotation + event.delta).rem_euclid(360.0);
            this.phase = event.phase;
            this.record(LastGesture::Rotation);
            cx.invalidate();
        });
        let smart_magnify = cx.smart_magnify_listener("gesture-pad", |this, _event, cx| {
            this.zoom = if this.zoom > 1.25 { 1.0 } else { 2.0 };
            this.record(LastGesture::SmartMagnify);
            cx.invalidate();
        });
        let scroll_wheel = cx.scroll_wheel_listener("gesture-pad", |this, event, cx| {
            let delta = event.delta.pixel_delta(40.0).y;
            let factor = (1.0 + delta * 0.003).clamp(0.75, 1.25);
            this.zoom = (this.zoom * factor).clamp(0.55, 2.25);
            this.phase = event.phase;
            this.wheel_delta = delta;
            this.wheel_precise = event.delta.precise();
            this.record(LastGesture::ScrollWheel);
            // This pad treats wheel motion as zoom, so the surrounding page must not also scroll.
            cx.prevent_default();
            cx.invalidate();
        });
        let touch = cx.touch_listener("gesture-pad", |this, event, cx| {
            this.update_touch(event.id, event.phase);
            this.last_touch_position = event.position;
            this.last_touch_force = event.force;
            this.record(LastGesture::Touch);
            cx.invalidate();
        });

        let tile_size = 88.0 * self.zoom;
        let angle = (self.rotation - 90.0).to_radians();
        let marker_x = 180.0 + angle.cos() * 102.0 - 7.0;
        let marker_y = 140.0 + angle.sin() * 102.0 - 7.0;
        let pressure_width = 264.0 * self.pressure;
        let metrics = cx.metrics();

        div()
            .size_full()
            .overflow_y_scroll()
            .p_6()
            .flex_col()
            .gap_5()
            .bg(Color::rgb8(14, 17, 23))
            .text_color(Color::rgb8(237, 240, 246))
            .child(text("Native trackpad input").text_2xl().font_bold())
            .child(
                text("Scroll to zoom, pinch, rotate, two-finger double-tap, Force Touch, or place raw touch contacts inside the pad. Input is delivered from the native event loop only; idle windows install no monitor and render no frames.")
                    .max_w(720.0)
                    .wrap()
                    .text_sm()
                    .text_color(Color::rgb8(158, 169, 188)),
            )
            .child(
                div()
                    .flex_row()
                    .gap_5()
                    .flex_wrap()
                    .child(
                        div()
                            .relative()
                            .w(360.0)
                            .h(280.0)
                            .flex_none()
                            .overflow_hidden()
                            .rounded_2xl()
                            .border(1.0, Color::rgb8(57, 67, 84))
                            .bg(Color::rgb8(22, 27, 36))
                            .on_mouse_pressure(pressure)
                            .on_scroll_wheel(scroll_wheel)
                            .on_touch(touch)
                            .on_pinch(pinch)
                            .on_rotation(rotation)
                            .on_smart_magnify(smart_magnify)
                            .child(
                                div()
                                    .absolute()
                                    .left(78.0)
                                    .top(38.0)
                                    .w(204.0)
                                    .h(204.0)
                                    .rounded(102.0)
                                    .border(1.0, Color::rgba8(125, 211, 252, 90)),
                            )
                            .child(
                                div()
                                    .absolute()
                                    .left(180.0 - tile_size * 0.5)
                                    .top(140.0 - tile_size * 0.5)
                                    .size(tile_size, tile_size)
                                    .rounded_xl()
                                    .border(1.0, Color::rgba8(255, 255, 255, 42))
                                    .bg(Color::rgba8(99, 102, 241, 225))
                                    .shadow_lg(),
                            )
                            .child(
                                div()
                                    .absolute()
                                    .left(marker_x)
                                    .top(marker_y)
                                    .size(14.0, 14.0)
                                    .rounded(7.0)
                                    .bg(Color::rgb8(125, 211, 252))
                                    .shadow_sm(),
                            )
                            .child(
                                text("GESTURE PAD")
                                    .absolute()
                                    .left(16.0)
                                    .bottom(13.0)
                                    .text_xs()
                                    .font_semibold()
                                    .text_color(Color::rgb8(111, 124, 148)),
                            ),
                    )
                    .child(
                        div()
                            .w(300.0)
                            .min_h(280.0)
                            .p_5()
                            .flex_col()
                            .gap_4()
                            .rounded_2xl()
                            .border(1.0, Color::rgb8(57, 67, 84))
                            .bg(Color::rgb8(22, 27, 36))
                            .child(readout("Zoom", format!("{:.0}%", self.zoom * 100.0)))
                            .child(readout("Rotation", format!("{:.1}°", self.rotation)))
                            .child(readout(
                                "Last wheel",
                                format!(
                                    "{:+.1} px · {}",
                                    self.wheel_delta,
                                    if self.wheel_precise { "precise" } else { "line" }
                                ),
                            ))
                            .child(readout(
                                "Raw touches",
                                format!(
                                    "{} · ({:.0}, {:.0}) · {}",
                                    self.active_touch_count(),
                                    self.last_touch_position.x,
                                    self.last_touch_position.y,
                                    self.last_touch_force.map_or_else(
                                        || "no force".to_owned(),
                                        |force| format!("{:.0}%", force * 100.0)
                                    )
                                ),
                            ))
                            .child(readout("Phase", format!("{:?}", self.phase)))
                            .child(readout("Last input", format!("{:?}", self.last)))
                            .child(
                                div()
                                    .flex_col()
                                    .gap_2()
                                    .child(
                                        div()
                                            .flex_row()
                                            .justify_between()
                                            .child(
                                                text("Force Touch")
                                                    .text_sm()
                                                    .text_color(Color::rgb8(158, 169, 188)),
                                            )
                                            .child(
                                                text(format!(
                                                    "{:.0}% · {:?}",
                                                    self.pressure * 100.0,
                                                    self.pressure_stage
                                                ))
                                                .text_sm()
                                                .font_semibold(),
                                            ),
                                    )
                                    .child(
                                        div()
                                            .relative()
                                            .w_full()
                                            .h(8.0)
                                            .overflow_hidden()
                                            .rounded(4.0)
                                            .bg(Color::rgb8(42, 49, 63))
                                            .child(
                                                div()
                                                    .absolute()
                                                    .left(0.0)
                                                    .top(0.0)
                                                    .w(pressure_width)
                                                    .h(8.0)
                                                    .rounded(4.0)
                                                    .bg(Color::rgb8(52, 211, 153)),
                                            ),
                                    ),
                            ),
                    ),
            )
            .child(
                div()
                    .max_w(685.0)
                    .p_4()
                    .rounded_xl()
                    .border(1.0, Color::rgb8(50, 58, 73))
                    .bg(Color::rgb8(18, 22, 30))
                    .flex_col()
                    .gap_2()
                    .child(text(format!("{} native event(s)", self.event_count)).font_semibold())
                    .child(
                        text(format!(
                            "Rendered frame {} · {:.2} ms CPU · no continuous animation request",
                            metrics.frame_number,
                            metrics.cpu_milliseconds()
                        ))
                        .wrap()
                        .text_xs()
                        .text_color(Color::rgb8(132, 147, 171)),
                    ),
            )
    }
}

fn readout(label: &'static str, value: String) -> quickgui::Element {
    div()
        .flex_row()
        .justify_between()
        .child(text(label).text_sm().text_color(Color::rgb8(158, 169, 188)))
        .child(text(value).text_sm().font_semibold())
        .into_element()
}
