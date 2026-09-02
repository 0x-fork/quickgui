//! Caller-styled gallery for QuickGUI's unstyled range and feedback components.
//!
//! Run with `cargo run --release --example range_controls`.

use std::time::Instant;

use quickgui::{
    Application, AsyncViewContext, Color, Element, IntoElement, Key, Meter, MouseButton,
    NumberField, NumberFieldState, Progress, Size, Slider, SliderState, Splitter,
    SplitterOrientation, SplitterState, Task, View, ViewContext, WindowOptions, div,
    slider_key_bindings, splitter_key_bindings, text, text_input,
};

const TRACK: Size = Size {
    width: 260.0,
    height: 20.0,
};

fn main() -> Result<(), quickgui::AppError> {
    Application::new()
        .bind_keys(slider_key_bindings())
        .bind_keys(splitter_key_bindings())
        .run(|cx| {
            cx.open_window(
                WindowOptions::new("QuickGUI — Range and feedback").size(880.0, 720.0),
                RangeControlsDemo::default(),
            );
        })
}

#[derive(Clone, Copy)]
struct Palette {
    background: Color,
    surface: Color,
    border: Color,
    foreground: Color,
    muted: Color,
    accent: Color,
    track: Color,
    warning: Color,
}

impl Palette {
    fn dark() -> Self {
        Self {
            background: Color::rgb8(17, 18, 21),
            surface: Color::rgb8(26, 28, 33),
            border: Color::rgb8(55, 59, 69),
            foreground: Color::rgb8(238, 240, 244),
            muted: Color::rgb8(155, 161, 174),
            accent: Color::rgb8(10, 132, 255),
            track: Color::rgb8(52, 55, 64),
            warning: Color::rgb8(255, 159, 10),
        }
    }
}

struct RangeControlsDemo {
    volume: SliderState,
    price: SliderState,
    zoom: SliderState,
    quantity: NumberFieldState,
    panes: SplitterState,
    downloaded: f64,
    repeat: Option<Task<()>>,
}

impl Default for RangeControlsDemo {
    fn default() -> Self {
        Self {
            volume: SliderState::new(0.0, 100.0, 40.0).step(5.0),
            price: SliderState::range(0.0, 100.0, &[20.0, 80.0]).step(10.0),
            zoom: SliderState::new(50.0, 400.0, 100.0).step(25.0).vertical(),
            quantity: NumberFieldState::new(4.0).range(0.0, 99.0).step(1.0),
            panes: SplitterState::new(SplitterOrientation::Horizontal, &[220.0, 380.0])
                .min_size(0, 140.0)
                .min_size(1, 200.0)
                .collapsible(0, true),
            downloaded: 42.0,
            repeat: None,
        }
    }
}

impl RangeControlsDemo {
    fn volume(view: &mut Self) -> &mut SliderState {
        &mut view.volume
    }

    fn price(view: &mut Self) -> &mut SliderState {
        &mut view.price
    }

    fn zoom(view: &mut Self) -> &mut SliderState {
        &mut view.zoom
    }

    fn panes(view: &mut Self) -> &mut SplitterState {
        &mut view.panes
    }

    fn section(title: &'static str, colors: Palette, content: Element) -> Element {
        div()
            .w_full()
            .flex_col()
            .gap_3()
            .p_4()
            .rounded_xl()
            .border(1.0, colors.border)
            .bg(colors.surface)
            .child(text(title).text_sm().font_semibold())
            .child(content)
    }

    fn horizontal_slider(&self, cx: &mut ViewContext<'_, Self>, colors: Palette) -> Element {
        let slider = Slider::new("volume", &self.volume);
        let drag = cx.pointer_listener(slider.track_id(), |view, event, cx| {
            if view.volume.apply_pointer(event, TRACK) {
                cx.invalidate();
            }
        });
        let thumb = slider.thumb(0).expect("single thumb");
        let fill = self.volume.fraction(0) * TRACK.width;

        slider.key_part(
            cx,
            slider
                .root_part(div().flex_row().items_center().gap_3())
                .accessibility_label("Volume")
                .focus(|state| state.border(2.0, colors.accent))
                .child(
                    slider
                        .track_part(
                            div()
                                .relative()
                                .w(TRACK.width)
                                .h(TRACK.height)
                                .flex_none()
                                .on_pointer(drag),
                        )
                        .child(
                            div()
                                .absolute()
                                .top(8.0)
                                .left(0.0)
                                .w(TRACK.width)
                                .h(4.0)
                                .rounded(2.0)
                                .bg(colors.track),
                        )
                        .child(
                            slider.range_part(
                                div()
                                    .absolute()
                                    .top(8.0)
                                    .left(0.0)
                                    .w(fill)
                                    .h(4.0)
                                    .rounded(2.0)
                                    .bg(colors.accent),
                            ),
                        )
                        .child(
                            thumb.thumb_part(
                                div()
                                    .absolute()
                                    .top(3.0)
                                    .left(fill - 7.0)
                                    .size(14.0, 14.0)
                                    .rounded(7.0)
                                    .bg(colors.foreground),
                            ),
                        ),
                )
                .child(
                    text(format!("{:.0}", self.volume.value()))
                        .text_sm()
                        .text_color(colors.muted),
                ),
            Self::volume,
        )
    }

    fn range_slider(&self, cx: &mut ViewContext<'_, Self>, colors: Palette) -> Element {
        let slider = Slider::new("price", &self.price);
        let drag = cx.pointer_listener(slider.track_id(), |view, event, cx| {
            if view.price.apply_pointer(event, TRACK) {
                cx.invalidate();
            }
        });
        let lower = slider.thumb(0).expect("lower thumb");
        let upper = slider.thumb(1).expect("upper thumb");
        let start = lower.fraction() * TRACK.width;
        let end = upper.fraction() * TRACK.width;

        slider
            .root_part(div().flex_row().items_center().gap_3())
            .accessibility_label("Price range")
            .child(
                slider
                    .track_part(
                        div()
                            .relative()
                            .w(TRACK.width)
                            .h(TRACK.height)
                            .flex_none()
                            .on_pointer(drag),
                    )
                    .child(
                        div()
                            .absolute()
                            .top(8.0)
                            .left(0.0)
                            .w(TRACK.width)
                            .h(4.0)
                            .rounded(2.0)
                            .bg(colors.track),
                    )
                    .child(
                        slider.range_part(
                            div()
                                .absolute()
                                .top(8.0)
                                .left(start)
                                .w((end - start).max(0.0))
                                .h(4.0)
                                .rounded(2.0)
                                .bg(colors.accent),
                        ),
                    )
                    .child(
                        lower.key_part(
                            cx,
                            lower
                                .thumb_part(
                                    div()
                                        .absolute()
                                        .top(3.0)
                                        .left(start - 7.0)
                                        .size(14.0, 14.0)
                                        .rounded(7.0)
                                        .bg(colors.foreground),
                                )
                                .accessibility_label("Minimum price")
                                .focus(|state| state.border(2.0, colors.accent)),
                            Self::price,
                        ),
                    )
                    .child(
                        upper.key_part(
                            cx,
                            upper
                                .thumb_part(
                                    div()
                                        .absolute()
                                        .top(3.0)
                                        .left(end - 7.0)
                                        .size(14.0, 14.0)
                                        .rounded(7.0)
                                        .bg(colors.foreground),
                                )
                                .accessibility_label("Maximum price")
                                .focus(|state| state.border(2.0, colors.accent)),
                            Self::price,
                        ),
                    ),
            )
            .child(
                text(format!(
                    "{:.0} – {:.0}",
                    self.price.values()[0],
                    self.price.values()[1]
                ))
                .text_sm()
                .text_color(colors.muted),
            )
    }

    fn vertical_slider(&self, cx: &mut ViewContext<'_, Self>, colors: Palette) -> Element {
        let slider = Slider::new("zoom", &self.zoom);
        let track = Size {
            width: 20.0,
            height: 120.0,
        };
        let drag = cx.pointer_listener(slider.track_id(), move |view, event, cx| {
            if view.zoom.apply_pointer(event, track) {
                cx.invalidate();
            }
        });
        let thumb = slider.thumb(0).expect("single thumb");
        let offset = track.height - self.zoom.fraction(0) * track.height;

        slider.key_part(
            cx,
            slider
                .root_part(div().flex_col().items_center().gap_2())
                .accessibility_label("Zoom")
                .focus(|state| state.border(2.0, colors.accent))
                .child(
                    slider
                        .track_part(
                            div()
                                .relative()
                                .w(track.width)
                                .h(track.height)
                                .flex_none()
                                .on_pointer(drag),
                        )
                        .child(
                            div()
                                .absolute()
                                .left(8.0)
                                .top(0.0)
                                .w(4.0)
                                .h(track.height)
                                .rounded(2.0)
                                .bg(colors.track),
                        )
                        .child(
                            thumb.thumb_part(
                                div()
                                    .absolute()
                                    .left(3.0)
                                    .top(offset - 7.0)
                                    .size(14.0, 14.0)
                                    .rounded(7.0)
                                    .bg(colors.foreground),
                            ),
                        ),
                )
                .child(
                    text(format!("{:.0}%", self.zoom.value()))
                        .text_sm()
                        .text_color(colors.muted),
                ),
            Self::zoom,
        )
    }

    fn number_field(&self, cx: &mut ViewContext<'_, Self>, colors: Palette) -> Element {
        let field = NumberField::new("quantity");
        let edit = cx.input_listener(field.input_id(), |view: &mut Self, value, cx| {
            if view.quantity.set_text(value) {
                cx.invalidate();
            }
        });
        // The input is marked invalid while its text is out of range, and QuickGUI blocks Return
        // submission for an invalid control, so commit from an ordinary key listener.
        let commit = cx.key_down_listener(field.input_id(), |view: &mut Self, event, cx| {
            if event.key == Key::Enter && view.quantity.commit() {
                cx.invalidate();
            }
        });
        let press_up =
            cx.mouse_down_listener(field.increment_id(), |view: &mut Self, _event, cx| {
                view.quantity.press_step(true, Instant::now());
                cx.invalidate();
            });
        let press_down =
            cx.mouse_down_listener(field.decrement_id(), |view: &mut Self, _event, cx| {
                view.quantity.press_step(false, Instant::now());
                cx.invalidate();
            });
        let release = cx.mouse_up_listener("quantity-release", |view: &mut Self, _event, cx| {
            if view.quantity.release_step() {
                view.repeat = None;
                cx.invalidate();
            }
        });

        let stepper = |glyph: &'static str| {
            div()
                .size(22.0, 20.0)
                .flex_row()
                .items_center()
                .justify_center()
                .rounded(6.0)
                .border(1.0, colors.border)
                .bg(colors.track)
                .child(text(glyph).text_sm())
        };

        field.root_part(
            div()
                .flex_row()
                .items_center()
                .gap_2()
                .on_mouse_up(MouseButton::Left, release)
                .child(
                    field.input_part(
                        &self.quantity,
                        text_input(self.quantity.text().clone())
                            .w(96.0)
                            .px_2()
                            .py_1()
                            .rounded_md()
                            .border(1.0, colors.border)
                            .bg(colors.track)
                            .text_color(colors.foreground)
                            .invalid_style(|state| state.border(1.0, colors.warning))
                            .on_input(edit)
                            .on_key_down(commit),
                    ),
                )
                .child(field.decrement_part(
                    &self.quantity,
                    stepper("−").on_mouse_down(MouseButton::Left, press_down),
                ))
                .child(field.increment_part(
                    &self.quantity,
                    stepper("+").on_mouse_down(MouseButton::Left, press_up),
                )),
        )
    }

    fn feedback(&self, colors: Palette) -> Element {
        let download = Progress::new(self.downloaded, 100.0)
            .value_text(format!("{:.0} percent", self.downloaded));
        let scanning = Progress::indeterminate();
        let disk = Meter::new(72.0, 0.0, 100.0).low(20.0).high(80.0);
        let completion = download.completion().unwrap_or(0.0);

        div()
            .flex_col()
            .gap_3()
            .child(
                download
                    .root_part(div().flex_col().gap_1())
                    .accessibility_label("Download")
                    .child(text("Downloading").text_sm())
                    .child(
                        div()
                            .w(TRACK.width)
                            .h(6.0)
                            .rounded(3.0)
                            .bg(colors.track)
                            .child(
                                download.indicator_part(
                                    div()
                                        .w(TRACK.width * completion)
                                        .h(6.0)
                                        .rounded(3.0)
                                        .bg(colors.accent),
                                ),
                            ),
                    ),
            )
            .child(
                scanning
                    .root_part(div().flex_col().gap_1())
                    .accessibility_label("Scanning")
                    .child(text("Scanning library").text_sm())
                    .child(
                        div()
                            .w(TRACK.width)
                            .h(6.0)
                            .rounded(3.0)
                            .bg(colors.track)
                            .child(scanning.indicator_part(
                                div().w(80.0).h(6.0).rounded(3.0).bg(colors.muted),
                            )),
                    ),
            )
            .child(
                disk.root_part(div().flex_col().gap_1())
                    .accessibility_label("Disk usage")
                    .child(text("Disk usage").text_sm())
                    .child(
                        div()
                            .w(TRACK.width)
                            .h(6.0)
                            .rounded(3.0)
                            .bg(colors.track)
                            .child(
                                disk.indicator_part(
                                    div()
                                        .w(TRACK.width * disk.completion())
                                        .h(6.0)
                                        .rounded(3.0)
                                        .bg(if disk.is_high() {
                                            colors.warning
                                        } else {
                                            colors.accent
                                        }),
                                ),
                            ),
                    ),
            )
    }

    fn splitter(&self, cx: &mut ViewContext<'_, Self>, colors: Palette) -> Element {
        let splitter = Splitter::new("workspace", &self.panes);
        let sidebar = splitter.pane(0).expect("sidebar");
        let content = splitter.pane(1).expect("content");
        let handle = splitter.handle(0).expect("handle");
        let drag = cx.pointer_listener(handle.handle_id(), |view, event, cx| {
            if view.panes.apply_pointer(0, event) {
                cx.invalidate();
            }
        });

        splitter.root_part(
            div()
                .h(160.0)
                .rounded_lg()
                .overflow_hidden()
                .border(1.0, colors.border)
                .child(
                    sidebar.pane_part(
                        div()
                            .p_3()
                            .bg(colors.track)
                            .child(text("Sidebar").text_sm()),
                    ),
                )
                .child(handle.key_part(
                    cx,
                    handle
                        .handle_part(div().w(6.0).bg(colors.border).on_pointer(drag))
                        .accessibility_label("Resize sidebar")
                        .focus(|state| state.bg(colors.accent)),
                    Self::panes,
                ))
                .child(
                    content.pane_part(
                        div()
                            .p_3()
                            .child(text("Editor").text_sm())
                            .child(
                                text("Drag the divider, or focus it and press the arrow keys. Enter collapses the sidebar.")
                                    .text_sm()
                                    .wrap()
                                    .text_color(colors.muted),
                            ),
                    ),
                ),
        )
    }

    fn schedule_repeat(&mut self, cx: &mut ViewContext<'_, Self>) {
        let Some(deadline) = self.quantity.repeat_deadline() else {
            self.repeat = None;
            return;
        };
        if self.repeat.is_some() {
            return;
        }
        self.repeat = cx
            .spawn(move |task: AsyncViewContext<Self>| async move {
                let mut next = deadline;
                loop {
                    if task.sleep_until(next).await.is_err() {
                        return;
                    }
                    let now = task.now();
                    let outcome = task
                        .update(move |view: &mut Self, cx| {
                            if view.quantity.repeat(now) {
                                cx.invalidate();
                            }
                            view.quantity.repeat_deadline()
                        })
                        .await;
                    match outcome {
                        Ok(Some(deadline)) => next = deadline,
                        _ => return,
                    }
                }
            })
            .ok();
    }
}

impl View for RangeControlsDemo {
    fn render(&mut self, cx: &mut ViewContext<'_, Self>) -> impl IntoElement {
        let colors = Palette::dark();
        self.schedule_repeat(cx);

        let sliders = Self::section(
            "Sliders",
            colors,
            div()
                .flex_col()
                .gap_4()
                .child(self.horizontal_slider(cx, colors))
                .child(self.range_slider(cx, colors))
                .child(self.vertical_slider(cx, colors)),
        );
        let number = Self::section("Number field", colors, self.number_field(cx, colors));
        let feedback = Self::section("Progress and meter", colors, self.feedback(colors));
        let panes = Self::section("Splitter", colors, self.splitter(cx, colors));

        div()
            .size_full()
            .flex_col()
            .gap_4()
            .p_5()
            .overflow_y_scroll()
            .bg(colors.background)
            .text_color(colors.foreground)
            .child(
                text("Unstyled range and feedback components")
                    .text_lg()
                    .font_semibold(),
            )
            .child(
                text("Every color, size, and animation below belongs to this example. QuickGUI owns only roles, values, keyboard behavior, and pointer capture.")
                    .text_sm()
                    .wrap()
                    .text_color(colors.muted),
            )
            .child(sliders)
            .child(number)
            .child(feedback)
            .child(panes)
    }
}
