use quickgui::{Application, Color, Element, Image, ObjectFit, View, ViewContext, div, img, text};

fn main() -> Result<(), quickgui::AppError> {
    Application::new().run(|cx| {
        cx.open_window(
            quickgui::WindowOptions::new("QuickGUI — GPU images").size(1040.0, 680.0),
            ImageDemo {
                landscape: generated_image(640, 360),
                thumbnail: generated_image(120, 68),
            },
        );
    })
}

struct ImageDemo {
    landscape: Image,
    thumbnail: Image,
}

impl ImageDemo {
    fn card(label: &'static str, image: &Image, fit: ObjectFit) -> Element {
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
                img(image)
                    .w_full()
                    .h(180.0)
                    .object_fit(fit)
                    .rounded_lg()
                    .bg(Color::rgb8(11, 13, 17))
                    .accessibility_label(label),
            )
    }
}

impl View for ImageDemo {
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
                    .child(text("GPU image primitives").text_2xl().font_bold())
                    .child(
                        text("Intrinsic sizing, web-style object-fit, rounded clipping, and one bounded texture cache.")
                            .text_sm()
                            .text_color(Color::rgb8(158, 166, 181)),
                    ),
            )
            .child(
                div()
                    .flex_row()
                    .gap_4()
                    .child(Self::card(
                        "object-fit: contain",
                        &self.landscape,
                        ObjectFit::Contain,
                    ))
                    .child(Self::card(
                        "object-fit: cover",
                        &self.landscape,
                        ObjectFit::Cover,
                    ))
                    .child(Self::card(
                        "object-fit: fill",
                        &self.landscape,
                        ObjectFit::Fill,
                    )),
            )
            .child(
                div()
                    .flex_1()
                    .min_h(0.0)
                    .flex_row()
                    .gap_4()
                    .child(Self::card(
                        "object-fit: none (intrinsic pixels)",
                        &self.thumbnail,
                        ObjectFit::None,
                    ))
                    .child(Self::card(
                        "object-fit: scale-down",
                        &self.thumbnail,
                        ObjectFit::ScaleDown,
                    ))
                    .child(
                        div()
                            .flex_1()
                            .min_w(0.0)
                            .flex_col()
                            .gap_2()
                            .p_3()
                            .rounded_xl()
                            .border(1.0, Color::rgb8(56, 61, 72))
                            .bg(Color::rgb8(25, 28, 34))
                            .child(text("GPU grayscale").font_semibold())
                            .child(
                                img(&self.landscape)
                                    .w_full()
                                    .h(180.0)
                                    .object_fit(ObjectFit::Cover)
                                    .grayscale(true)
                                    .rounded_lg()
                                    .accessibility_label("Generated landscape in grayscale"),
                            ),
                    ),
            )
    }
}

fn generated_image(width: u32, height: u32) -> Image {
    let mut rgba = Vec::with_capacity(width as usize * height as usize * 4);
    for y in 0..height {
        for x in 0..width {
            let fx = x as f32 / (width - 1) as f32;
            let fy = y as f32 / (height - 1) as f32;
            let checker = if (x / 40 + y / 40) % 2 == 0 { 24 } else { 0 };
            let border = x < 7 || y < 7 || x >= width - 7 || y >= height - 7;
            let (red, green, blue) = if border {
                (245, 248, 255)
            } else {
                (
                    (35.0 + fx * 195.0) as u8,
                    (45.0 + fy * 165.0) as u8,
                    (215.0 - fx * 125.0 + checker as f32) as u8,
                )
            };
            rgba.extend_from_slice(&[red, green, blue, 255]);
        }
    }
    Image::from_rgba(width, height, rgba).expect("generated dimensions and pixels are valid")
}
