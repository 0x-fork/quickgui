use quickgui::{
    App, Color, Element, ObjectFit, Svg, SvgTransform, View, ViewContext, button, div, svg, text,
};

fn main() -> Result<(), quickgui::AppError> {
    App::new(SvgDemo {
        spark: Svg::from_svg(SPARK_ICON).expect("the embedded spark SVG is valid"),
        orbit: Svg::from_svg(ORBIT_ICON).expect("the embedded orbit SVG is valid"),
    })
    .title("QuickGUI — retained SVG icons")
    .size(1040.0, 700.0)
    .run()
}

struct SvgDemo {
    spark: Svg,
    orbit: Svg,
}

impl SvgDemo {
    fn card(title: &'static str, detail: &'static str, content: Element) -> Element {
        div()
            .flex_1()
            .min_w(0.0)
            .h(230.0)
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

impl View for SvgDemo {
    fn render(&mut self, _cx: &mut ViewContext<'_, Self>) -> impl quickgui::IntoElement {
        let inherited = div()
            .size(112.0, 112.0)
            .items_center()
            .justify_center()
            .rounded_xl()
            .bg(Color::rgb8(49, 46, 129))
            .text_color(Color::rgb8(196, 181, 253))
            .child(svg(&self.spark).size(72.0, 72.0));

        let interactive = button()
            .size(112.0, 112.0)
            .items_center()
            .justify_center()
            .rounded_xl()
            .bg(Color::rgb8(30, 41, 59))
            .text_color(Color::rgb8(56, 189, 248))
            .hover(|style| {
                style
                    .bg(Color::rgb8(12, 74, 110))
                    .text_color(Color::rgb8(186, 230, 253))
            })
            .active(|style| style.bg(Color::rgb8(14, 116, 144)).text_color(Color::WHITE))
            .child(svg(&self.spark).size(72.0, 72.0))
            .accessibility_label("Interactive spark icon");

        let transformed = div()
            .size(112.0, 112.0)
            .items_center()
            .justify_center()
            .rounded_xl()
            .bg(Color::rgb8(69, 26, 3))
            .text_color(Color::rgb8(253, 186, 116))
            .child(
                svg(&self.spark).size(72.0, 72.0).svg_transform(
                    SvgTransform::new()
                        .scale(0.82)
                        .rotate(-0.28)
                        .translate(6.0, -4.0),
                ),
            );

        let fitted = svg(&self.orbit)
            .w_full()
            .h(112.0)
            .object_fit(ObjectFit::Contain)
            .text_color(Color::rgb8(74, 222, 128));

        div()
            .size_full()
            .flex_col()
            .gap(24.0)
            .p(24.0)
            .bg(Color::rgb8(15, 20, 29))
            .text_color(Color::rgb8(241, 245, 249))
            .child(
                div()
                    .flex_col()
                    .gap_1()
                    .child(text("Retained SVG icons").text_size(30.0).font_bold())
                    .child(
                        text("Parsed once, rasterized at physical size, cached as one-channel masks, and recolored on the GPU.")
                            .text_color(Color::rgb8(148, 163, 184)),
                    ),
            )
            .child(
                div()
                    .flex_row()
                    .gap_4()
                    .child(Self::card(
                        "Inherited tint",
                        "SVG color follows the nearest text color.",
                        inherited,
                    ))
                    .child(Self::card(
                        "Hover and press",
                        "Recoloring reuses the exact same cached mask.",
                        interactive,
                    ))
                    .child(Self::card(
                        "Render transform",
                        "Scale, rotation, and translation do not relayout.",
                        transformed,
                    )),
            )
            .child(
                div()
                    .flex_1()
                    .min_h(0.0)
                    .flex_col()
                    .gap_2()
                    .p_4()
                    .rounded_xl()
                    .border(1.0, Color::rgb8(51, 65, 85))
                    .bg(Color::rgb8(22, 28, 39))
                    .child(text("Intrinsic aspect ratio + object-fit: contain").font_semibold())
                    .child(fitted)
                    .child(
                        text("The 160×60 source remains sharp at the window scale factor and shares the same flexbox sizing rules as raster images.")
                            .text_sm()
                            .text_color(Color::rgb8(148, 163, 184)),
                    ),
            )
    }
}

const SPARK_ICON: &str = r##"
<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24">
  <path fill="#000" d="M12 1.5l2.56 7.01L21.5 11l-6.94 2.49L12 20.5l-2.56-7.01L2.5 11l6.94-2.49L12 1.5z"/>
  <circle cx="19" cy="4.5" r="2" fill="#000"/>
</svg>
"##;

const ORBIT_ICON: &str = r##"
<svg xmlns="http://www.w3.org/2000/svg" width="160" height="60" viewBox="0 0 160 60">
  <g fill="none" stroke="#000" stroke-width="4">
    <ellipse cx="80" cy="30" rx="66" ry="20"/>
    <ellipse cx="80" cy="30" rx="66" ry="20" transform="rotate(60 80 30)"/>
    <ellipse cx="80" cy="30" rx="66" ry="20" transform="rotate(120 80 30)"/>
  </g>
  <circle cx="80" cy="30" r="8" fill="#000"/>
</svg>
"##;
