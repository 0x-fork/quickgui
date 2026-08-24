use quickgui::{
    App, Background, Color, Element, FillOptions, FillRule, GradientColorSpace, LineCap, LineJoin,
    LinearGradient, ObjectFit, Path, PathBuilder, PathStyle, Point, Rect, Size, StrokeOptions,
    View, ViewContext, canvas, div, linear_color_stop, path, text,
};

fn main() -> Result<(), quickgui::AppError> {
    App::new(PathDemo::new())
        .title("QuickGUI — retained paths and canvas")
        .size(1080.0, 760.0)
        .run()
}

struct PathDemo {
    star: Path,
    ring: Path,
    wave: Path,
}

impl PathDemo {
    fn new() -> Self {
        Self {
            star: build_star(),
            ring: build_ring(),
            wave: build_wave(),
        }
    }

    fn card(title: &'static str, detail: &'static str, content: Element) -> Element {
        div()
            .flex_1()
            .min_w(0.0)
            .h(236.0)
            .flex_col()
            .gap_3()
            .p_4()
            .rounded_xl()
            .border(1.0, Color::rgb8(51, 65, 85))
            .bg(Color::rgb8(22, 28, 39))
            .child(
                div()
                    .flex_1()
                    .min_h(0.0)
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
}

impl View for PathDemo {
    fn render(&mut self, _cx: &mut ViewContext<'_, Self>) -> impl quickgui::IntoElement {
        let linear = Background::LinearGradient(
            LinearGradient::new(
                135.0,
                linear_color_stop(Color::rgb8(34, 211, 238), 0.0),
                linear_color_stop(Color::rgb8(139, 92, 246), 1.0),
            )
            .color_space(GradientColorSpace::LinearSrgb),
        );
        let perceptual = Background::LinearGradient(
            LinearGradient::new(
                90.0,
                linear_color_stop(Color::rgb8(251, 113, 133), 0.1),
                linear_color_stop(Color::rgb8(250, 204, 21), 0.9),
            )
            .color_space(GradientColorSpace::Oklab),
        );
        let traditional = Background::LinearGradient(
            LinearGradient::new(
                180.0,
                linear_color_stop(Color::rgb8(74, 222, 128), 0.0),
                linear_color_stop(Color::rgb8(14, 116, 144), 1.0),
            )
            .color_space(GradientColorSpace::Srgb),
        );

        let star = path(&self.star)
            .size(118.0, 118.0)
            .object_fit(ObjectFit::Contain)
            .path_background(linear);
        let ring = path(&self.ring)
            .size(118.0, 118.0)
            .object_fit(ObjectFit::Contain)
            .path_background(perceptual);
        let wave = path(&self.wave)
            .w_full()
            .h(118.0)
            .object_fit(ObjectFit::Contain)
            .path_background(Color::rgb8(56, 189, 248));

        let canvas_wave = self.wave.clone();
        let canvas_star = self.star.clone();
        let custom_paint = canvas(move |bounds, surface| {
            surface.fill_rect(bounds, Color::rgb8(17, 24, 39));
            let columns = (bounds.width / 24.0).ceil() as usize;
            let rows = (bounds.height / 24.0).ceil() as usize;
            for column in 0..=columns {
                surface.fill_rect(
                    Rect::new(column as f32 * 24.0, 0.0, 1.0, bounds.height),
                    Color::rgba8(100, 116, 139, 35),
                );
            }
            for row in 0..=rows {
                surface.fill_rect(
                    Rect::new(0.0, row as f32 * 24.0, bounds.width, 1.0),
                    Color::rgba8(100, 116, 139, 35),
                );
            }
            surface.paint_path_at(
                &canvas_wave,
                Point::new(24.0, 48.0),
                Color::rgb8(45, 212, 191),
            );
            surface.paint_path_transformed(
                &canvas_star,
                [0.72, 0.72],
                Point::new(bounds.width - 112.0, 22.0),
                traditional,
            );
        })
        .size_full();

        let ordered_canvas = div()
            .relative()
            .w_full()
            .h(196.0)
            .overflow_hidden()
            .rounded_xl()
            .border(1.0, Color::rgb8(51, 65, 85))
            .child(custom_paint)
            .child(
                div()
                    .absolute()
                    .right(22.0)
                    .bottom(18.0)
                    .px_3()
                    .py_2()
                    .rounded_lg()
                    .bg(Color::rgba8(15, 23, 42, 225))
                    .child(
                        text("Text stays above canvas paths")
                            .text_sm()
                            .font_semibold(),
                    ),
            );

        div()
            .size_full()
            .flex_col()
            .gap(22.0)
            .p(24.0)
            .bg(Color::rgb8(15, 20, 29))
            .text_color(Color::rgb8(241, 245, 249))
            .child(
                div()
                    .flex_col()
                    .gap_1()
                    .child(text("Retained paths + custom canvas").text_size(30.0).font_bold())
                    .child(
                        text("Lyon tessellates once on the CPU; WGPU batches retained geometry by overlap order and antialiases only true outline edges.")
                            .text_color(Color::rgb8(148, 163, 184)),
                    ),
            )
            .child(
                div()
                    .flex_row()
                    .gap_4()
                    .child(Self::card(
                        "Linear-light gradient",
                        "A declarative fill fitted like an image.",
                        star,
                    ))
                    .child(Self::card(
                        "Even-odd + Oklab",
                        "Arcs form a cutout with perceptual interpolation.",
                        ring,
                    ))
                    .child(Self::card(
                        "Dashed cubic stroke",
                        "Round caps, joins, and a bounded dash expansion.",
                        wave,
                    )),
            )
            .child(
                div()
                    .flex_1()
                    .min_h(0.0)
                    .flex_col()
                    .gap_2()
                    .child(text("Canvas uses local coordinates and clips like the web").font_semibold())
                    .child(ordered_canvas)
                    .child(
                        text("The grid, stroke, gradient star, translucent badge, and glyphs share one source-correct paint stream across GPU pipelines.")
                            .text_sm()
                            .text_color(Color::rgb8(148, 163, 184)),
                    ),
            )
    }
}

fn build_star() -> Path {
    let mut points = Vec::with_capacity(10);
    for index in 0..10 {
        let angle = -std::f32::consts::FRAC_PI_2 + index as f32 * std::f32::consts::PI / 5.0;
        let radius = if index % 2 == 0 { 50.0 } else { 22.0 };
        points.push(Point::new(
            50.0 + radius * angle.cos(),
            50.0 + radius * angle.sin(),
        ));
    }
    let mut builder = PathBuilder::fill();
    builder.add_polygon(&points, true);
    builder.build().expect("the star path is valid")
}

fn build_ring() -> Path {
    let mut builder = PathBuilder::fill().with_style(PathStyle::Fill(
        FillOptions::new().with_fill_rule(FillRule::EvenOdd),
    ));
    builder.move_to(Point::new(50.0, 0.0));
    builder.arc_to(
        Size::new(50.0, 50.0),
        0.0,
        false,
        true,
        Point::new(50.0, 100.0),
    );
    builder.arc_to(
        Size::new(50.0, 50.0),
        0.0,
        false,
        true,
        Point::new(50.0, 0.0),
    );
    builder.close();
    builder.move_to(Point::new(50.0, 24.0));
    builder.arc_to(
        Size::new(26.0, 26.0),
        0.0,
        false,
        true,
        Point::new(50.0, 76.0),
    );
    builder.arc_to(
        Size::new(26.0, 26.0),
        0.0,
        false,
        true,
        Point::new(50.0, 24.0),
    );
    builder.close();
    builder.build().expect("the even-odd ring path is valid")
}

fn build_wave() -> Path {
    let style = StrokeOptions::new()
        .with_line_width(7.0)
        .with_line_cap(LineCap::Round)
        .with_line_join(LineJoin::Round);
    let mut builder = PathBuilder::fill()
        .with_style(PathStyle::Stroke(style))
        .dash_array(&[18.0, 10.0]);
    builder.move_to(Point::new(4.0, 54.0));
    builder.cubic_to(
        Point::new(56.0, -4.0),
        Point::new(112.0, 112.0),
        Point::new(164.0, 54.0),
    );
    builder.cubic_to(
        Point::new(188.0, 28.0),
        Point::new(210.0, 30.0),
        Point::new(232.0, 54.0),
    );
    builder.build().expect("the dashed wave path is valid")
}
