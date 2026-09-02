//! Background gradients, per-corner radii, dashed and dotted borders, outlines, and raster
//! backgrounds.
//!
//! Everything here is analytic paint state: no element in this example creates an offscreen
//! texture, a gradient ramp cache, a timer, or an extra draw call, and the window sleeps as soon
//! as the pointer stops moving.

use quickgui::{
    Application, Color, ColorStops, Corners, Element, Filter, Gradient, GradientCenter,
    GradientColorSpace, GradientDirection, Image, IntoElement, RadialGradientExtent,
    RadialGradientShape, View, ViewContext, WindowOptions, div, linear_color_stop, text,
};

fn main() -> Result<(), quickgui::AppError> {
    Application::new().run(|cx| {
        cx.open_window(
            WindowOptions::new("QuickGUI — gradients, corners, borders, outlines")
                .size(1120.0, 860.0),
            EffectsDemo::new(),
        );
    })
}

struct EffectsDemo {
    tile: Image,
    swatch_image: Image,
}

impl EffectsDemo {
    fn new() -> Self {
        Self {
            tile: checkerboard(),
            swatch_image: color_ramp(),
        }
    }
}

/// A tiny generated hue ramp, so the filter row needs no asset on disk.
fn color_ramp() -> Image {
    const WIDTH: u32 = 32;
    const HEIGHT: u32 = 32;
    let mut pixels = Vec::with_capacity((WIDTH * HEIGHT * 4) as usize);
    for y in 0..HEIGHT {
        for x in 0..WIDTH {
            let red = (x * 255 / (WIDTH - 1)) as u8;
            let green = (y * 255 / (HEIGHT - 1)) as u8;
            pixels.extend_from_slice(&[red, green, 200_u8.saturating_sub(red / 2), 255]);
        }
    }
    Image::from_rgba(WIDTH, HEIGHT, pixels).expect("the generated ramp is valid")
}

/// A tiny generated checkerboard, so the example needs no asset on disk.
fn checkerboard() -> Image {
    const SIZE: u32 = 16;
    let mut pixels = Vec::with_capacity((SIZE * SIZE * 4) as usize);
    for y in 0..SIZE {
        for x in 0..SIZE {
            let dark = ((x / 8) + (y / 8)) % 2 == 0;
            let value = if dark { 30 } else { 46 };
            pixels.extend_from_slice(&[value, value + 6, value + 16, 255]);
        }
    }
    Image::from_rgba(SIZE, SIZE, pixels).expect("the generated tile is valid")
}

fn card(title: &'static str, detail: &'static str, content: Element) -> Element {
    div()
        .flex_1()
        .min_w(0.0)
        .flex_col()
        .gap_3()
        .p_4()
        .rounded_xl()
        .border(1.0, Color::rgb8(51, 65, 85))
        .bg(Color::rgb8(22, 28, 39))
        .child(
            div()
                .h(140.0)
                .items_center()
                .justify_center()
                .child(content),
        )
        .child(
            div()
                .flex_col()
                .gap_1()
                .child(text(title).font_semibold())
                .child(
                    text(detail)
                        .text_sm()
                        .text_color(Color::rgb8(148, 163, 184)),
                ),
        )
}

fn row(children: impl IntoIterator<Item = Element>) -> Element {
    div().flex_row().gap_4().children(children)
}

fn swatch() -> Element {
    div().w(150.0).h(110.0).rounded_lg()
}

impl View for EffectsDemo {
    fn render(&mut self, _cx: &mut ViewContext<'_, Self>) -> impl IntoElement {
        div()
            .size_full()
            .flex_col()
            .gap_5()
            .p_6()
            .overflow_y_scroll()
            .bg(Color::rgb8(12, 16, 24))
            .text_color(Color::rgb8(226, 232, 240))
            .child(text("Gradients").text_lg().font_semibold())
            .child(row([
                card(
                    "Linear",
                    "Eight bounded stops, CSS angles, any interpolation space.",
                    swatch().bg_linear_gradient(
                        GradientDirection::ToBottomRight,
                        [
                            Color::rgb8(56, 189, 248),
                            Color::rgb8(129, 140, 248),
                            Color::rgb8(244, 114, 182),
                        ],
                    ),
                ),
                card(
                    "Positioned stops",
                    "Explicit stop positions interpolated in Oklab.",
                    swatch().bg_gradient(
                        Gradient::linear(
                            90.0,
                            ColorStops::new([
                                linear_color_stop(Color::rgb8(14, 165, 233), 0.0),
                                linear_color_stop(Color::rgb8(250, 204, 21), 0.35),
                                linear_color_stop(Color::rgb8(239, 68, 68), 1.0),
                            ]),
                        )
                        .color_space(GradientColorSpace::Oklab),
                    ),
                ),
                card(
                    "Radial",
                    "Circle or ellipse, any center, three ending-shape extents.",
                    swatch().bg_gradient(
                        Gradient::radial([Color::rgb8(250, 250, 250), Color::rgb8(30, 41, 59)])
                            .shape(RadialGradientShape::Circle)
                            .extent(RadialGradientExtent::FarthestCorner)
                            .center(GradientCenter::new(0.3, 0.25)),
                    ),
                ),
                card(
                    "Conic",
                    "One instance, evaluated analytically from a start angle.",
                    swatch().rounded(55.0).bg_conic_gradient(
                        0.0,
                        [
                            Color::rgb8(248, 113, 113),
                            Color::rgb8(250, 204, 21),
                            Color::rgb8(74, 222, 128),
                            Color::rgb8(56, 189, 248),
                            Color::rgb8(248, 113, 113),
                        ],
                    ),
                ),
            ]))
            .child(text("Corners and borders").text_lg().font_semibold())
            .child(row([
                card(
                    "Per-corner radii",
                    "rounded_tl / tr / br / bl, rounded_t / b / l / r, corner_radii.",
                    swatch()
                        .bg(Color::rgb8(30, 41, 59))
                        .corner_radii(Corners::new(28.0, 4.0, 28.0, 4.0)),
                ),
                card(
                    "Dashed border",
                    "Dashes are distributed evenly around the whole outline.",
                    swatch()
                        .bg(Color::rgb8(17, 24, 39))
                        .rounded(18.0)
                        .border(3.0, Color::rgb8(148, 163, 184))
                        .border_dashed(),
                ),
                card(
                    "Dotted border",
                    "The same analytic perimeter with a one-width pattern.",
                    swatch()
                        .bg(Color::rgb8(17, 24, 39))
                        .rounded_t(30.0)
                        .border(3.0, Color::rgb8(56, 189, 248))
                        .border_dotted(),
                ),
                card(
                    "Outline",
                    "Painted outside the border box; layout never moves. Hover me.",
                    swatch()
                        .bg(Color::rgb8(30, 41, 59))
                        .rounded_lg()
                        .cursor_pointer()
                        .hover(|style| style.outline_offset(3.0, Color::rgb8(250, 204, 21), 4.0)),
                ),
            ]))
            .child(text("Raster backgrounds").text_lg().font_semibold())
            .child(row([
                card(
                    "Tiled",
                    "Repeated tiles reuse one bounded GPU texture and one draw.",
                    swatch()
                        .rounded_xl()
                        .bg_image_tiled(self.tile.clone())
                        .border(1.0, Color::rgb8(51, 65, 85)),
                ),
                card(
                    "Cover",
                    "Scaled to cover the box and clipped by the rounded corners.",
                    swatch().rounded(40.0).bg_image_cover(self.tile.clone()),
                ),
                card(
                    "Contain",
                    "Scaled to fit, anchored by a background position.",
                    swatch()
                        .rounded_lg()
                        .bg(Color::rgb8(17, 24, 39))
                        .bg_image_contain(self.tile.clone()),
                ),
                card(
                    "Gradient over image",
                    "The color background paints first, then the raster tile.",
                    swatch()
                        .rounded_lg()
                        .bg_linear_gradient(
                            GradientDirection::ToTop,
                            [Color::rgb8(56, 189, 248), Color::rgb8(15, 23, 42)],
                        )
                        .bg_image_tiled(self.tile.clone())
                        .opacity(0.85),
                ),
            ]))
            .child(text("Color filters").text_lg().font_semibold())
            .child(row([
                card(
                    "Unfiltered",
                    "The reference raster content.",
                    swatch()
                        .rounded_lg()
                        .bg_image_cover(self.swatch_image.clone()),
                ),
                card(
                    "Grayscale",
                    "grayscale(true) is the luminance-preserving saturate(0) matrix.",
                    swatch()
                        .rounded_lg()
                        .bg_image_cover(self.swatch_image.clone())
                        .grayscale(true),
                ),
                card(
                    "Hue rotate",
                    "One chained color matrix; the filter count never costs GPU work.",
                    swatch()
                        .rounded_lg()
                        .bg_image_cover(self.swatch_image.clone())
                        .hue_rotate(140.0)
                        .saturate(1.4),
                ),
                card(
                    "Invert and contrast",
                    "Chains collapse into a single 4x5 multiply-add.",
                    swatch()
                        .rounded_lg()
                        .bg_image_cover(self.swatch_image.clone())
                        .filters([Filter::Invert(1.0), Filter::Contrast(1.2)]),
                ),
            ]))
    }
}
