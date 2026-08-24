use std::{thread, time::Duration};

use quickgui::{
    AnimatedImage, AnimatedImageFrame, AnimationRepeat, App, Color, Element, Image, ImageResource,
    ObjectFit, View, ViewContext, div, img, text,
};

fn main() -> Result<(), quickgui::AppError> {
    let reduce_motion = std::env::var_os("QUICKGUI_REDUCE_MOTION").is_some();
    let frames = generated_frames(480, 300);
    let looping = AnimatedImage::new(frames.clone()).expect("generated animation is bounded");
    let finite = AnimatedImage::with_repeat(frames.clone(), AnimationRepeat::Finite(2))
        .expect("generated animation is bounded");
    let delayed_animation = AnimatedImage::new(frames).expect("generated animation is bounded");
    let delayed = ImageResource::custom_animated(move || {
        thread::sleep(Duration::from_millis(700));
        Ok::<_, &'static str>(delayed_animation.clone())
    });

    App::new(AnimationDemo {
        looping,
        finite,
        delayed,
    })
    .title("QuickGUI — event-driven animated images")
    .size(1080.0, 560.0)
    .reduce_motion(reduce_motion)
    .run()
}

struct AnimationDemo {
    looping: AnimatedImage,
    finite: AnimatedImage,
    delayed: ImageResource,
}

impl AnimationDemo {
    fn card(label: &'static str, detail: &'static str, image: Element) -> Element {
        div()
            .flex_1()
            .min_w(0.0)
            .flex_col()
            .gap_2()
            .p_3()
            .rounded_xl()
            .border(1.0, Color::rgb8(56, 61, 72))
            .bg(Color::rgb8(25, 28, 34))
            .child(text(label).font_semibold())
            .child(
                text(detail)
                    .text_sm()
                    .text_color(Color::rgb8(158, 166, 181)),
            )
            .child(image)
    }
}

impl View for AnimationDemo {
    fn render(&mut self, _cx: &mut ViewContext<'_, Self>) -> impl quickgui::IntoElement {
        div()
            .size_full()
            .flex_col()
            .gap_4()
            .p_4()
            .bg(Color::rgb8(17, 19, 23))
            .text_color(Color::rgb8(234, 237, 242))
            .child(
                div()
                    .flex_col()
                    .gap_1()
                    .child(text("Animated images without a frame loop").text_2xl().font_bold())
                    .child(
                        text("Each visible element owns its phase and wakes only for its next frame deadline.")
                            .text_sm()
                            .text_color(Color::rgb8(158, 166, 181)),
                    ),
            )
            .child(
                div()
                    .flex_1()
                    .min_h(0.0)
                    .flex_row()
                    .gap_4()
                    .child(Self::card(
                        "Infinite animation",
                        "Six frames at 240 ms each.",
                        image_element(&self.looping, "Infinitely looping generated animation"),
                    ))
                    .child(Self::card(
                        "Finite animation",
                        "Stops on its final frame after two iterations.",
                        image_element(&self.finite, "Finite generated animation"),
                    ))
                    .child(Self::card(
                        "Asynchronous animation",
                        "Decodes off-thread, then joins exact-deadline playback.",
                        img(&self.delayed)
                            .w_full()
                            .h(280.0)
                            .object_fit(ObjectFit::Cover)
                            .rounded_lg()
                            .bg(Color::rgb8(11, 13, 17))
                            .with_loading(|| {
                                div()
                                    .size_full()
                                    .items_center()
                                    .justify_center()
                                    .bg(Color::rgb8(68, 76, 92))
                                    .child(text("Loading animation…").font_semibold())
                            })
                            .accessibility_label("Asynchronously loaded generated animation"),
                    )),
            )
    }
}

fn image_element(animation: &AnimatedImage, label: &'static str) -> Element {
    img(animation)
        .w_full()
        .h(280.0)
        .object_fit(ObjectFit::Cover)
        .rounded_lg()
        .bg(Color::rgb8(11, 13, 17))
        .accessibility_label(label)
}

fn generated_frames(width: u32, height: u32) -> Vec<AnimatedImageFrame> {
    (0..6)
        .map(|frame| {
            let marker_start = frame * width / 6;
            let marker_end = (marker_start + width / 6).min(width);
            let mut rgba = Vec::with_capacity(width as usize * height as usize * 4);
            for y in 0..height {
                for x in 0..width {
                    let fx = x as f32 / (width - 1) as f32;
                    let fy = y as f32 / (height - 1) as f32;
                    let marker = x >= marker_start && x < marker_end;
                    let (red, green, blue) = if marker {
                        (244, 247, 255)
                    } else {
                        (
                            30_u8.saturating_add((fx * 170.0) as u8),
                            48_u8.saturating_add((fy * 145.0) as u8),
                            205_u8.saturating_sub((fx * 85.0) as u8),
                        )
                    };
                    rgba.extend_from_slice(&[red, green, blue, 255]);
                }
            }
            AnimatedImageFrame::new(
                Image::from_rgba(width, height, rgba)
                    .expect("generated dimensions and pixels are valid"),
                Duration::from_millis(240),
            )
        })
        .collect()
}
