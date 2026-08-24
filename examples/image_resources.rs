use std::{path::PathBuf, thread, time::Duration};

use quickgui::{
    App, Color, Element, Image, ImageResource, ObjectFit, View, ViewContext, div, img, text,
};

fn main() -> Result<(), quickgui::AppError> {
    let delayed_ms = std::env::var("QUICKGUI_IMAGE_RESOURCE_DELAY_MS")
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(1_500);
    let missing_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("examples")
        .join("assets")
        .join("missing-image.png");
    App::new(ResourceDemo {
        fast: ImageResource::custom(|| Ok::<_, &'static str>(generated_image(640, 360, 24))),
        delayed: ImageResource::custom(move || {
            thread::sleep(Duration::from_millis(delayed_ms));
            Ok::<_, &'static str>(generated_image(640, 360, 92))
        }),
        missing: ImageResource::from_path(missing_path),
    })
    .title("QuickGUI — asynchronous image resources")
    .size(1080.0, 560.0)
    .run()
}

struct ResourceDemo {
    fast: ImageResource,
    delayed: ImageResource,
    missing: ImageResource,
}

impl ResourceDemo {
    fn card(label: &'static str, detail: &'static str, resource: &ImageResource) -> Element {
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
            .child(
                img(resource)
                    .w_full()
                    .h(280.0)
                    .object_fit(ObjectFit::Cover)
                    .rounded_lg()
                    .bg(Color::rgb8(11, 13, 17))
                    .with_loading(|| {
                        replacement("Loading on a bounded worker…", Color::rgb8(68, 76, 92))
                    })
                    .with_fallback(|| {
                        replacement("Fallback: image unavailable", Color::rgb8(112, 52, 58))
                    })
                    .accessibility_label(label),
            )
    }
}

impl View for ResourceDemo {
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
                    .child(text("Event-driven image resources").text_2xl().font_bold())
                    .child(
                        text("Fast loads avoid flicker; slow loads reveal after 200 ms; errors render a normal flexbox fallback.")
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
                        "Fast custom resource",
                        "Completes before loading UI is eligible.",
                        &self.fast,
                    ))
                    .child(Self::card(
                        "Delayed custom resource",
                        "An artificial delay exposes the loading state.",
                        &self.delayed,
                    ))
                    .child(Self::card(
                        "Missing path resource",
                        "Fails on the worker and selects fallback content.",
                        &self.missing,
                    )),
            )
    }
}

fn replacement(label: &'static str, color: Color) -> Element {
    div()
        .size_full()
        .items_center()
        .justify_center()
        .p_4()
        .bg(color)
        .child(text(label).font_semibold())
}

fn generated_image(width: u32, height: u32, hue: u8) -> Image {
    let mut rgba = Vec::with_capacity(width as usize * height as usize * 4);
    for y in 0..height {
        for x in 0..width {
            let fx = x as f32 / (width - 1) as f32;
            let fy = y as f32 / (height - 1) as f32;
            let stripe = if (x / 36 + y / 36) % 2 == 0 { 28 } else { 0 };
            rgba.extend_from_slice(&[
                hue.saturating_add((fx * 135.0) as u8),
                52_u8.saturating_add((fy * 160.0) as u8),
                214_u8.saturating_sub((fx * 70.0) as u8) + stripe,
                255,
            ]);
        }
    }
    Image::from_rgba(width, height, rgba).expect("generated dimensions and pixels are valid")
}
