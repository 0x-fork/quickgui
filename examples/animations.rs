use std::time::Duration;

use quickgui::{
    Animation, AnimationExt as _, AnimationPhase, Application, Color, Element, ElementId,
    IntoElement, SpringAnimation, SpringConfig, TitleBarStyle, View, ViewContext, WindowOptions,
    bounce, button, div, ease_in_out, text,
};

fn main() -> Result<(), quickgui::AppError> {
    Application::new().run(|cx| {
        cx.open_window(
            WindowOptions::new("QuickGUI — declarative motion")
                .size(900.0, 650.0)
                .minimum_size(720.0, 560.0)
                .title_bar_style(TitleBarStyle::HiddenInset)
                .traffic_light_position(16.0, 14.0)
                .background(Color::rgb8(16, 18, 23))
                .reduce_motion(std::env::var_os("QUICKGUI_REDUCE_MOTION").is_some()),
            AnimationDemo::default(),
        );
    })
}

#[derive(Default)]
struct AnimationDemo {
    replay: u64,
    spring_at_end: bool,
    breathing: bool,
}

impl View for AnimationDemo {
    fn render(&mut self, cx: &mut ViewContext<'_, Self>) -> impl IntoElement {
        let replay = cx.listener("replay-animation", |this, cx| {
            this.replay = this.replay.wrapping_add(1);
            cx.invalidate();
        });
        let retarget = cx.listener("retarget-spring", |this, cx| {
            this.spring_at_end = !this.spring_at_end;
            cx.invalidate();
        });
        let toggle_breathing = cx.listener("toggle-breathing", |this, cx| {
            this.breathing = !this.breathing;
            cx.invalidate();
        });
        let transition_probe = cx.listener("transition-probe", |_this, _cx| {});
        let metrics = cx.metrics();

        let progress =
            div().h(18.0).rounded_lg().with_animation(
                ElementId::new(0xa11c_e000_0000_0000 ^ self.replay),
                Animation::new(Duration::from_millis(620)).with_easing(ease_in_out),
                |element, value| {
                    let phase = AnimationPhase(value);
                    element.w(phase.interpolate_clamped(24.0, 420.0)).bg(phase
                        .interpolate_clamped(Color::rgb8(249, 115, 22), Color::rgb8(34, 197, 94)))
                },
            );

        let spring_target = if self.spring_at_end { 372.0 } else { 0.0 };
        let spring = div()
            .absolute()
            .top(4.0)
            .size(24.0, 24.0)
            .rounded(12.0)
            .bg(Color::rgb8(96, 165, 250))
            .with_spring(
                "motion-spring",
                SpringAnimation::new(SpringConfig::new(170.0, 14.0, 1.0))
                    .to(spring_target)
                    .with_epsilon(0.05),
                |element, left| element.left(left),
            );

        let breathing = if self.breathing {
            div().h(24.0).rounded(12.0).with_animation(
                "synced-breathing",
                Animation::new(Duration::from_millis(1_400))
                    .repeat_synced()
                    .with_easing(bounce(ease_in_out))
                    .with_max_fps(30.0),
                |element, value| {
                    let phase = AnimationPhase(value);
                    element.w(phase.interpolate_clamped(90.0, 360.0)).bg(phase
                        .interpolate_clamped(Color::rgb8(168, 85, 247), Color::rgb8(236, 72, 153)))
                },
            )
        } else {
            div()
                .h(24.0)
                .w(90.0)
                .rounded(12.0)
                .bg(Color::rgb8(168, 85, 247))
        };

        div()
            .size_full()
            .flex_col()
            .bg(Color::rgb8(16, 18, 23))
            .text_color(Color::rgb8(241, 245, 249))
            .child(
                div()
                    .h(62.0)
                    .w_full()
                    .flex_none()
                    .flex_row()
                    .items_center()
                    .justify_between()
                    .padding(0.0, 18.0, 0.0, 88.0)
                    .border(1.0, Color::rgb8(45, 50, 61))
                    .bg(Color::rgb8(22, 25, 31))
                    .app_region_drag()
                    .child(
                        div()
                            .flex_col()
                            .child(text("Declarative motion").text_lg().font_bold())
                            .child(
                                text("Time curves, exact throttling, and retargetable springs")
                                    .text_xs()
                                    .text_color(Color::rgb8(148, 163, 184)),
                            ),
                    )
                    .child(
                        text(format!(
                            "frame {} · active {}",
                            metrics.frame_number, metrics.render.active_animations
                        ))
                        .text_xs()
                        .text_color(Color::rgb8(148, 163, 184)),
                    ),
            )
            .child(
                div()
                    .flex_1()
                    .min_h(0.0)
                    .overflow_y_scroll()
                    .p_6()
                    .flex_col()
                    .gap_4()
                    .child(card(
                        "Paint-only web transitions",
                        "Hover, press, or keyboard-focus the control. Colors, border, radius, opacity, and shadows interpolate without rebuilding this view, reshaping text, or rerunning layout.",
                        control_button("Interact with me", transition_probe),
                    ))
                    .child(card(
                        "One-shot animation",
                        "Changing the animation ID restarts the sequence. The final frame removes its scheduling source.",
                        div()
                            .flex_col()
                            .gap_4()
                            .child(
                                div()
                                    .h(18.0)
                                    .w_full()
                                    .rounded_lg()
                                    .bg(Color::rgb8(38, 43, 53))
                                    .child(progress),
                            )
                            .child(control_button("Replay", replay)),
                    ))
                    .child(card(
                        "Retargetable spring",
                        "Click rapidly: position and velocity survive target changes, without frame-rate-dependent integration.",
                        div()
                            .flex_col()
                            .gap_4()
                            .child(
                                div()
                                    .relative()
                                    .h(32.0)
                                    .w(396.0)
                                    .rounded_lg()
                                    .bg(Color::rgb8(38, 43, 53))
                                    .child(spring),
                            )
                            .child(control_button("Retarget", retarget)),
                    ))
                    .child(card(
                        "Opt-in repeating motion",
                        "A 30 FPS cap uses one exact deadline at a time. Turning it off returns the application to ControlFlow::Wait.",
                        div()
                            .flex_col()
                            .gap_4()
                            .child(
                                div()
                                    .h(24.0)
                                    .w_full()
                                    .rounded(12.0)
                                    .bg(Color::rgb8(38, 43, 53))
                                    .child(breathing),
                            )
                            .child(control_button(
                                if self.breathing { "Stop" } else { "Start" },
                                toggle_breathing,
                            )),
                    )),
            )
    }
}

fn card(title: &'static str, detail: &'static str, content: Element) -> Element {
    div()
        .w_full()
        .p_5()
        .flex_col()
        .gap_3()
        .rounded_2xl()
        .border(1.0, Color::rgb8(48, 54, 66))
        .bg(Color::rgb8(24, 27, 34))
        .shadow_md()
        .child(text(title).text_xl().font_bold())
        .child(
            text(detail)
                .max_w(680.0)
                .wrap()
                .text_sm()
                .text_color(Color::rgb8(148, 163, 184)),
        )
        .child(content)
}

fn control_button<V>(label: &'static str, listener: quickgui::ClickListener<V>) -> Element {
    button()
        .px_4()
        .py_2()
        .rounded_lg()
        .bg(Color::rgb8(51, 65, 85))
        .opacity(0.88)
        .border(1.0, Color::rgb8(71, 85, 105))
        .shadow_sm()
        .hover(|style| {
            style
                .bg(Color::rgb8(79, 70, 229))
                .border(2.0, Color::rgb8(165, 180, 252))
                .rounded(14.0)
                .shadow_lg()
                .opacity(1.0)
        })
        .active(|style| {
            style
                .bg(Color::rgb8(67, 56, 202))
                .rounded(6.0)
                .opacity(0.76)
        })
        .focus(|style| style.border(2.0, Color::rgb8(196, 181, 253)))
        .transition(Duration::from_millis(140))
        .child(text(label).font_semibold())
        .on_click(listener)
}
