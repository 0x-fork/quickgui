use std::sync::Arc;

use quickgui::{
    App, Color, CustomShader, Element, Event, EventContext, ShaderParameters, View, ViewContext,
    button, custom_shader, div, text,
};

const AURORA: &str = r#"
fn palette(value: f32, tint: vec3<f32>) -> vec3<f32> {
    let wave = 0.5 + 0.5 * cos(6.2831853 * (value + vec3<f32>(0.0, 0.18, 0.36)));
    return mix(wave, tint, 0.42);
}

fn quickgui_fragment(input: QuickGuiShaderInput) -> vec4<f32> {
    let phase = input.params[0].x;
    let tint = input.params[1].xyz;
    let centered = input.uv * 2.0 - vec2<f32>(1.0);
    let radius = length(centered);
    let angle = atan2(centered.y, centered.x);
    let bands = sin(angle * 3.0 + radius * 9.0 - phase) * 0.5 + 0.5;
    let glow = exp(-2.2 * radius * radius);
    let color = palette(bands + phase * 0.04, tint) * (0.28 + 0.9 * glow);
    let vignette = smoothstep(1.35, 0.15, radius);
    return vec4<f32>(color * vignette, 1.0);
}
"#;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    App::new(ShaderDemo::new()?)
        .title("QuickGUI — Custom GPU shaders")
        .size(900.0, 680.0)
        .run()?;
    Ok(())
}

struct ShaderDemo {
    shader: CustomShader,
    phase: f32,
    status: Arc<str>,
}

impl ShaderDemo {
    fn new() -> Result<Self, quickgui::CustomShaderError> {
        Ok(Self {
            shader: CustomShader::new(AURORA)?,
            phase: 0.0,
            status: Arc::from("The window is sleeping. Click once to submit one new frame."),
        })
    }

    fn control(label: &'static str) -> Element {
        button()
            .h(40.0)
            .px_4()
            .flex_row()
            .items_center()
            .justify_center()
            .rounded_lg()
            .border(1.0, Color::rgb8(75, 82, 96))
            .bg(Color::rgb8(35, 39, 48))
            .hover(|style| style.bg(Color::rgb8(48, 54, 66)))
            .active(|style| style.bg(Color::rgb8(31, 82, 126)))
            .focus(|style| style.border(2.0, Color::rgb8(94, 234, 212)))
            .child(text(label).font_medium())
    }

    fn shader_card(
        &self,
        title: &'static str,
        detail: &'static str,
        tint: [f32; 3],
        phase_offset: f32,
    ) -> Element {
        let parameters = ShaderParameters::new()
            .vector(0, [self.phase + phase_offset, 0.0, 0.0, 0.0])
            .vector(1, [tint[0], tint[1], tint[2], 0.0]);
        div()
            .min_w(0.0)
            .flex_1()
            .flex_col()
            .gap_2()
            .child(
                div()
                    .h(230.0)
                    .overflow_hidden()
                    .rounded_xl()
                    .border(1.0, Color::rgba8(255, 255, 255, 32))
                    .bg(Color::rgb8(12, 14, 20))
                    .child(
                        custom_shader(self.shader.clone())
                            .shader_parameters(parameters)
                            .size_full(),
                    ),
            )
            .child(text(title).font_semibold().text_lg())
            .child(
                text(detail)
                    .text_sm()
                    .text_color(Color::rgb8(157, 164, 178)),
            )
    }
}

impl View for ShaderDemo {
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
        let advance = cx.listener("advance", |this, cx| {
            this.phase = (this.phase + 0.65) % 64.0;
            this.status = Arc::from(format!(
                "Phase {:.2} submitted; the renderer sleeps again after this frame.",
                this.phase
            ));
            cx.invalidate();
        });
        let reset = cx.listener("reset", |this, cx| {
            this.phase = 0.0;
            this.status = Arc::from("Parameters reset in one damage-driven frame.");
            cx.invalidate();
        });
        let metrics = cx.metrics();

        div()
            .size_full()
            .overflow_y_scroll()
            .flex_col()
            .gap_5()
            .p_5()
            .bg(Color::rgb8(15, 17, 22))
            .text_color(Color::rgb8(235, 237, 242))
            .child(
                text("Retained application shaders")
                    .text_2xl()
                    .font_bold()
                    .flex_none(),
            )
            .child(
                text("Trusted WGSL is validated once, compiled lazily, cached per window, clipped by the framework, and drawn from one bounded instanced buffer. There is no private device, texture copy, or idle frame loop.")
                    .max_w(780.0)
                    .text_color(Color::rgb8(164, 171, 185))
                    .flex_none(),
            )
            .child(
                div()
                    .flex_row()
                    .gap_4()
                    .flex_none()
                    .child(self.shader_card(
                        "One pipeline",
                        "Same validated asset, first parameter set.",
                        [0.10, 0.82, 0.92],
                        0.0,
                    ))
                    .child(self.shader_card(
                        "One instanced upload",
                        "Same pipeline, independent per-instance vectors.",
                        [0.78, 0.30, 0.95],
                        1.7,
                    )),
            )
            .child(
                div()
                    .flex_row()
                    .gap_3()
                    .flex_none()
                    .child(Self::control("Advance one frame").on_click(advance))
                    .child(Self::control("Reset parameters").on_click(reset)),
            )
            .child(
                div()
                    .flex_col()
                    .gap_2()
                    .p_4()
                    .rounded_xl()
                    .bg(Color::rgb8(23, 26, 33))
                    .flex_none()
                    .child(
                        text(self.status.clone())
                            .text_color(Color::rgb8(126, 231, 212)),
                    )
                    .child(
                        text(format!(
                            "Last frame: {} shader instances · {} cached pipelines · {} compilations · {} total draw calls · {:.2} ms CPU",
                            metrics.render.custom_shader_instances,
                            metrics.render.cached_custom_shader_pipelines,
                            metrics.render.custom_shader_compilations,
                            metrics.render.draw_calls,
                            metrics.cpu_milliseconds(),
                        ))
                        .text_sm()
                        .text_color(Color::rgb8(157, 164, 178)),
                    ),
            )
            .child(
                text("Escape quits. Resize and clipping stay on QuickGUI's normal retained layout path.")
                    .text_xs()
                    .text_color(Color::rgb8(127, 134, 148))
                    .flex_none(),
            )
    }
}
