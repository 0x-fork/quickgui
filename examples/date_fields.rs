//! Segmented date and time editors.
//!
//! QuickGUI owns the calendar contract, typed-digit entry, wrapping arrow steps, segment focus,
//! and spin-button accessibility. Every separator, width, color, and focus ring below belongs to
//! this application.

use std::sync::Arc;

use quickgui::{
    Application, Calendar, CalendarState, CivilDate, CivilTime, Color, DateField, DateFieldOrder,
    DateFieldState, DateSegment, IntoElement, TimeField, TimeFieldState, TimeSegment,
    TitleBarStyle, View, ViewContext, calendar_key_bindings, date_field_key_bindings, div, text,
    time_field_key_bindings,
};

fn main() -> Result<(), quickgui::AppError> {
    Application::new()
        .bind_keys(date_field_key_bindings())
        .bind_keys(time_field_key_bindings())
        .bind_keys(calendar_key_bindings())
        .run(|cx| {
            cx.open_window(
                quickgui::WindowOptions::new("QuickGUI — Date and time fields")
                    .size(760.0, 520.0)
                    .title_bar_style(TitleBarStyle::HiddenInset)
                    .traffic_light_position(16.0, 13.0),
                DateGallery::new(),
            );
        })
}

#[derive(Clone, Copy)]
struct FieldPalette {
    page: Color,
    panel: Color,
    foreground: Color,
    muted: Color,
    border: Color,
    segment_active: Color,
    invalid: Color,
    focus_ring: Color,
}

impl FieldPalette {
    fn new(dark: bool) -> Self {
        if dark {
            Self {
                page: Color::rgb8(12, 15, 20),
                panel: Color::rgb8(18, 21, 27),
                foreground: Color::rgb8(232, 234, 239),
                muted: Color::rgb8(143, 149, 162),
                border: Color::rgb8(61, 67, 79),
                segment_active: Color::rgb8(31, 62, 98),
                invalid: Color::rgb8(255, 132, 122),
                focus_ring: Color::rgb8(64, 156, 255),
            }
        } else {
            Self {
                page: Color::rgb8(239, 241, 245),
                panel: Color::WHITE,
                foreground: Color::rgb8(28, 31, 38),
                muted: Color::rgb8(102, 108, 120),
                border: Color::rgb8(211, 215, 222),
                segment_active: Color::rgb8(220, 235, 252),
                invalid: Color::rgb8(191, 42, 34),
                focus_ring: Color::rgb8(0, 122, 255),
            }
        }
    }
}

struct DateGallery {
    due: DateFieldState,
    start: TimeFieldState,
    month: CalendarState,
}

impl DateGallery {
    fn new() -> Self {
        Self {
            due: DateFieldState::from_date(CivilDate::new(2026, 9, 3).expect("a real day"))
                .order(DateFieldOrder::DayMonthYear)
                .minimum(CivilDate::new(2026, 1, 1).expect("a real day"))
                .maximum(CivilDate::new(2026, 12, 31).expect("a real day")),
            start: TimeFieldState::from_time(CivilTime::new(9, 30, 0).expect("a real time"))
                .hour12(true),
            month: CalendarState::selected(CivilDate::new(2026, 9, 3).expect("a real day"))
                .minimum(CivilDate::new(2026, 1, 1).expect("a real day"))
                .maximum(CivilDate::new(2026, 12, 31).expect("a real day")),
        }
    }

    fn due(view: &mut Self) -> &mut DateFieldState {
        &mut view.due
    }

    fn start(view: &mut Self) -> &mut TimeFieldState {
        &mut view.start
    }

    fn month(view: &mut Self) -> &mut CalendarState {
        &mut view.month
    }

    fn summary(&self) -> Arc<str> {
        match (self.due.value(), self.start.value()) {
            (Some(date), Some(time)) => Arc::from(format!(
                "{:04}-{:02}-{:02} at {:02}:{:02}",
                date.year, date.month, date.day, time.hour, time.minute
            )),
            _ => Arc::from("Incomplete"),
        }
    }
}

impl View for DateGallery {
    fn render(&mut self, cx: &mut ViewContext<'_, Self>) -> impl IntoElement {
        let palette = FieldPalette::new(cx.appearance().is_dark());

        let date = DateField::new("due");
        let mut date_row = date
            .root_part(&self.due, div())
            .accessibility_label("Due date")
            .flex_row()
            .items_center()
            .gap_1()
            .px(10.0)
            .h(36.0)
            .rounded(8.0)
            .border(1.0, palette.border)
            .bg(palette.panel)
            .invalid_style(move |invalid| invalid.border(1.0, palette.invalid));
        let segments = self.due.segment_order().segments();
        for (index, segment) in segments.into_iter().enumerate() {
            if index > 0 {
                date_row = date_row.child(
                    text("/")
                        .text_color(palette.muted)
                        .accessibility_hidden(true),
                );
            }
            let part = date.segment(segment);
            let filled = self.due.is_filled(segment);
            let element = part
                .segment_part(
                    &self.due,
                    div()
                        .px(4.0)
                        .rounded(4.0)
                        .text_color(if filled {
                            palette.foreground
                        } else {
                            palette.muted
                        })
                        .bg(if self.due.focused_segment() == segment {
                            palette.segment_active
                        } else {
                            Color::TRANSPARENT
                        })
                        .focus(move |focus| focus.border(1.0, palette.focus_ring))
                        .child(text(self.due.segment_text(segment)).no_wrap()),
                )
                .accessibility_label(match segment {
                    DateSegment::Year => "Due year",
                    DateSegment::Month => "Due month",
                    DateSegment::Day => "Due day",
                });
            date_row = date_row.child(part.key_part(cx, element, Self::due));
        }

        let time = TimeField::new("start");
        let mut time_row = time
            .root_part(&self.start, div())
            .accessibility_label("Start time")
            .flex_row()
            .items_center()
            .gap_1()
            .px(10.0)
            .h(36.0)
            .rounded(8.0)
            .border(1.0, palette.border)
            .bg(palette.panel)
            .invalid_style(move |invalid| invalid.border(1.0, palette.invalid));
        for (index, segment) in self.start.segments().enumerate() {
            if index > 0 && segment != TimeSegment::Period {
                time_row = time_row.child(
                    text(":")
                        .text_color(palette.muted)
                        .accessibility_hidden(true),
                );
            }
            let part = time.segment(segment);
            let filled = self.start.is_filled(segment);
            let element = part.segment_part(
                &self.start,
                div()
                    .px(4.0)
                    .rounded(4.0)
                    .text_color(if filled {
                        palette.foreground
                    } else {
                        palette.muted
                    })
                    .bg(if self.start.focused_segment() == segment {
                        palette.segment_active
                    } else {
                        Color::TRANSPARENT
                    })
                    .focus(move |focus| focus.border(1.0, palette.focus_ring))
                    .child(text(self.start.segment_text(segment)).no_wrap()),
            );
            time_row = time_row.child(part.key_part(cx, element, Self::start));
        }

        let grid = Calendar::new("month");
        let mut month = grid
            .grid_part(self.month, div())
            .accessibility_label("Choose a due date")
            .flex_col()
            .gap_1()
            .p(8.0)
            .rounded(8.0)
            .border(1.0, palette.border)
            .bg(palette.panel);
        for index in 0..self.month.week_count() {
            let days = self.month.week(index).expect("a mounted week row");
            let mut row = grid.week_part(index, div().flex_row().gap_1());
            for day in days {
                let inside = self.month.is_in_displayed_month(day);
                let selected = self.month.selected_day() == Some(day);
                let cell = grid.day_part(
                    self.month,
                    day,
                    div()
                        .w(30.0)
                        .h(26.0)
                        .flex_row()
                        .items_center()
                        .justify_center()
                        .rounded(5.0)
                        .bg(if selected {
                            palette.segment_active
                        } else {
                            Color::TRANSPARENT
                        })
                        .text_color(if inside {
                            palette.foreground
                        } else {
                            palette.muted
                        })
                        .hover(move |hover| hover.bg(palette.segment_active))
                        .focus(move |focus| focus.border(1.0, palette.focus_ring))
                        .disabled_style(|disabled| disabled.opacity(0.4))
                        .child(text(day.day.to_string()).text_xs()),
                );
                row = row.child(grid.key_part(cx, day, cell, Self::month));
            }
            month = month.child(row);
        }
        let (year, month_number) = self.month.displayed_month();

        let valid = self.due.is_valid() && self.start.is_valid();
        div()
            .size_full()
            .flex_col()
            .bg(palette.page)
            .text_color(palette.foreground)
            .child(
                div()
                    .h(52.0)
                    .flex_none()
                    .flex_row()
                    .items_center()
                    .px(20.0)
                    .app_region_drag()
                    .child(text("Segmented date and time").font_semibold()),
            )
            .child(
                div()
                    .flex_1()
                    .min_h(0.0)
                    .flex_col()
                    .gap_4()
                    .p(20.0)
                    .app_region_no_drag()
                    .child(
                        div()
                            .flex_col()
                            .gap_2()
                            .child(text("Due date").text_sm().font_semibold())
                            .child(date_row)
                            .child(
                                text("Type digits to fill each segment, arrows step and wrap, Backspace clears, Tab and Left/Right move between segments.")
                                    .text_xs()
                                    .text_color(palette.muted),
                            ),
                    )
                    .child(
                        div()
                            .flex_col()
                            .gap_2()
                            .child(text("Start time").text_sm().font_semibold())
                            .child(time_row)
                            .child(
                                text("A 12-hour clock keeps one 24-hour value; `a` and `p` set the period.")
                                    .text_xs()
                                    .text_color(palette.muted),
                            ),
                    )
                    .child(
                        div()
                            .flex_col()
                            .gap_2()
                            .child(
                                text(format!("Month grid — {year:04}-{month_number:02}"))
                                    .text_sm()
                                    .font_semibold(),
                            )
                            .child(month)
                            .child(
                                text("Arrows move one day or one week, Home and End reach the week edges, Page Up and Page Down change months, and Return selects.")
                                    .text_xs()
                                    .text_color(palette.muted),
                            ),
                    )
                    .child(
                        text(self.summary())
                            .text_sm()
                            .text_color(if valid { palette.muted } else { palette.invalid }),
                    )
                    .child(
                        text(if valid {
                            "Inside the declared 2026 range"
                        } else {
                            "Outside the declared 2026 range or incomplete"
                        })
                        .text_xs()
                        .text_color(if valid { palette.muted } else { palette.invalid }),
                    ),
            )
    }
}
