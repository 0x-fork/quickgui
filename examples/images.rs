use quickgui::{
    Application, Color, Element, Image, ObjectFit, Rect, View, ViewContext, div, img, text,
};

fn main() -> Result<(), quickgui::AppError> {
    Application::new().run(|cx| {
        cx.open_window(
            quickgui::WindowOptions::new("QuickGUI — GPU images").size(1040.0, 680.0),
            ImageDemo::new(),
        );
    })
}

struct ImageDemo {
    landscape: Image,
    thumbnail: Image,
    /// The landscape re-encoded to PNG and decoded back through a base64 `data:` URL.
    round_tripped: Image,
    /// A bilinear resize of a crop taken from the landscape's center.
    cropped: Image,
    summary: String,
}

impl ImageDemo {
    fn new() -> Self {
        let landscape = generated_image(640, 360);
        let thumbnail = generated_image(120, 68);

        // PNG keeps alpha; JPEG composites it away. Both are decoded back into ordinary images.
        let png = landscape.to_png().expect("PNG encoding always succeeds");
        let jpeg = landscape
            .to_jpeg(85)
            .expect("JPEG encoding always succeeds");
        let data_url = format!("data:image/png;base64,{}", encode_base64(&png));
        let round_tripped =
            Image::from_data_url(&data_url).expect("QuickGUI decodes its own PNG data URLs");

        let cropped = landscape
            .crop(Rect::new(160.0, 90.0, 320.0, 180.0))
            .and_then(|cropped| cropped.resize(640, 360))
            .expect("the crop lies inside the source image");

        // Template metadata and multi-scale representations travel with the image into AppKit.
        let retina = generated_image(240, 136);
        let system = Image::named_system("NSFolder").ok().and_then(|image| {
            image
                .template(true)
                .with_representations([(2.0, retina)])
                .ok()
        });

        let summary = format!(
            "PNG {} bytes · JPEG {} bytes · data URL {} bytes · system image {}",
            png.len(),
            jpeg.len(),
            data_url.len(),
            system.map_or_else(
                || "unavailable on this platform".to_owned(),
                |image| format!("{}x{} template", image.width(), image.height())
            ),
        );

        Self {
            landscape,
            thumbnail,
            round_tripped,
            cropped,
            summary,
        }
    }

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
            .child(
                div()
                    .flex_row()
                    .gap_4()
                    .child(Self::card(
                        "PNG encode + data URL decode",
                        &self.round_tripped,
                        ObjectFit::Cover,
                    ))
                    .child(Self::card(
                        "crop + bilinear resize",
                        &self.cropped,
                        ObjectFit::Cover,
                    )),
            )
            .child(
                text(self.summary.clone())
                    .text_sm()
                    .text_color(Color::rgb8(158, 166, 181)),
            )
    }
}

/// Minimal standard-base64 encoder used to build the example's `data:` URL.
fn encode_base64(bytes: &[u8]) -> String {
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut encoded = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let mut buffer = [0_u8; 3];
        buffer[..chunk.len()].copy_from_slice(chunk);
        let value = u32::from_be_bytes([0, buffer[0], buffer[1], buffer[2]]);
        for index in 0..4 {
            if index <= chunk.len() {
                encoded.push(ALPHABET[((value >> (18 - index * 6)) & 0x3F) as usize] as char);
            } else {
                encoded.push('=');
            }
        }
    }
    encoded
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
