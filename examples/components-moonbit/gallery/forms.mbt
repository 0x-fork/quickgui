///|
fn field_demo(t : Theme) -> @ui.Element {
  let (email, set_email) = @ui.create_signal("")
  let state : @reactive.Signal[Json] = @reactive.signal({})
  let field = observe(
      @ui.field([])
      .required(true)
      .bind_invalid(() => !email().contains("@"))
      .bind_filled(() => email() != "")
      .validation_mode("onChange")
      .validation_debounce_time(200)
      .validation_message("Enter an address containing @"),
      state,
    )
    .flex_col()
    .gap(8)
  panel(
    t, "Field", "A control, label, description, error, and validity. The core reports validation triggers and debounce.",
  ).children([
    field.children([
      field.slot(@ui.Label).label("Email address"),
      editor(t, field.slot(@ui.Control))
      .width(280)
      .placeholder("you@example.com")
      .bind_value(() => email())
      .on_input(v => set_email(v)),
      field
      .slot(@ui.Description)
      .children([muted(t, "We only use this address for receipts.")]),
      field
      .slot(@ui.Error)
      .label("Enter an address containing @")
      .font_size(12)
      .bind_style(() => @ui.style().text_color(t.color("danger"))),
      field
      .slot(@ui.Validity)
      .children([
        note(t, () => {
          "filled \{email() != ""} · invalid \{!email().contains("@")}"
        }),
      ]),
    ]),
    readout(t, state),
  ])
}

///|
fn fieldset_demo(t : Theme) -> @ui.Element {
  let saving = @reactive.signal(false)
  let name = @reactive.signal("Ada")
  let org = @reactive.signal("Analytical Engines")
  let group = @ui.fieldset([])
    .bind_disabled(() => saving.get())
    .flex_col()
    .gap(8)

  ignore(
    group.children([
      group.slot(@ui.Legend).label("Profile"),
      group.slot(@ui.Description).label("Your public identity"),
    ]),
  )
  for entry in [("Name", name), ("Organization", org)] {
    let (title, value) = entry
    let field = @ui.field([])
      .bind_disabled(() => saving.get())
      .flex_col()
      .gap(8)
    ignore(
      group.children([
        field.children([
          field.slot(@ui.Label).label(title),
          editor(t, field.slot(@ui.Control))
          .width(280)
          .bind_disabled(() => saving.get())
          .bind_value(() => value.get())
          .on_input(v => value.set(v)),
        ]),
      ]),
    )
  }
  panel(
    t, "Fieldset", "A semantic group with a legend. A single flag disables every nested field.",
  ).children([
    switch_control(t, "Disable while saving", saving),
    group,
    note(t, () => "disabled \{saving.get()} · \{name.get()} · \{org.get()}"),
  ])
}

///|
fn form_demo(t : Theme) -> @ui.Element {
  let (email, set_email) = @ui.create_signal("")
  let plan = @reactive.signal("pro")
  let (terms, set_terms) = @ui.create_signal(false)
  let (errors, set_errors) = @ui.create_signal(false)
  let (attempts, set_attempts) = @ui.create_signal(0)
  let (result, set_result) = @ui.create_signal("nothing submitted yet")
  let submit = () => {
    set_attempts(@ui.untrack(attempts) + 1)
    let invalid = !@ui.untrack(email).contains("@") || !@ui.untrack(terms)
    set_errors(invalid)
    set_result(
      if invalid {
        "rejected: fix the fields below"
      } else {
        "accepted: \{@ui.untrack(email)} on the \{plan.peek()} plan"
      },
    )
  }
  let field = @ui.field([])
    .required(true)
    .bind_invalid(() => errors() && !email().contains("@"))
    .bind_touched(() => attempts() > 0)
    .bind_filled(() => email() != "")
    .validation_message("An address containing @ is required")
    .flex_col()
    .gap(8)
  let group = @ui.fieldset([]).flex_col().gap(8)

  panel(
    t, "Form", "Validation appears on submission. Return in the email field also submits; Reset clears all form state.",
  ).children([
    group.children(
      [
        group.slot(@ui.Legend).label("Sign up"),
        field.children([
          field.slot(@ui.Label).label("Email"),
          editor(t, field.slot(@ui.Control))
          .width(280)
          .placeholder("you@example.com")
          .bind_value(() => email())
          .on_input(v => set_email(v))
          .on_submit(_ => submit()),
          field
          .slot(@ui.Error)
          .label("An address containing @ is required")
          .font_size(12)
          .bind_style(() => @ui.style().text_color(t.color("danger"))),
        ]),
        label(t, "Plan"),
        radio_group(t, ["free", "pro", "team"], plan).flex_direction("row"),
        checkbox(t, "I accept the terms", () => terms(), v => set_terms(v)),
        ..if errors() && !terms() {
          [
            @ui.text("The terms must be accepted")
            .font_size(12)
            .bind_style(() => @ui.style().text_color(t.color("danger"))),
          ]
        } else {
          []
        },
        row().children([
          button(t, "Submit", submit, primary=true),
          button(t, "Reset", () => {
            @ui.batch(() => {
              set_email("")
              plan.set("pro")
              set_terms(false)
              set_errors(false)
              set_attempts(0)
              set_result("nothing submitted yet")
            })
          }),
        ]),
      ],
    ),
    note(t, () => "attempts \{attempts()} · \{result()}"),
  ])
}

///|
fn input_demo(t : Theme) -> @ui.Element {
  let text = @reactive.signal("")
  let secret = @reactive.signal("")
  let notes = @reactive.signal("Two\nlines")
  let (submits, set_submits) = @ui.create_signal(0)
  panel(
    t, "Input", "Controlled native editors: single-line, password, and multiline. Return in the first field reports submission.",
  ).children([
    input(t, text, "Type and press Return")
    .width(300)
    .on_submit(_ => set_submits(submits() + 1)),
    input(t, secret, "Password").width(300).password(true),
    input(t, notes, "Notes")
    .multiline(true)
    .width(420)
    .height(96)
    .padding_top(6),
    note(t, () => {
      "text \{text.get().to_json().stringify()} · password length \{secret.get().length()} · notes \{notes.get().to_json().stringify()} · submits \{submits()}"
    }),
  ])
}

///|
fn number_controls(t : Theme, root : @ui.Element) -> @ui.Element {
  root
  .slot(@ui.Group)
  .children([
    control(t, root.slot(@ui.Decrement))
    .children([icon("minus")])
    .aria("Decrease"),
    editor(t, root.slot(@ui.Input)).width(110),
    control(t, root.slot(@ui.Increment))
    .children([icon("plus")])
    .aria("Increase"),
  ])
  .flex_row()
  .gap(10)
  .items_center()
}

///|
fn number_field_demo(t : Theme) -> @ui.Element {
  let state : @reactive.Signal[Json] = @reactive.signal({ "numberValue": 8 })
  let root = observe(
      @ui.number_field([])
      .values([8])
      .minimum(0)
      .maximum(100)
      .step(1)
      .small_step(0.1)
      .large_step(10)
      .property(@protocol.PRECISION, @protocol.Number(1))
      .allow_wheel_scrub(true)
      .required(true),
      state,
    )
    .flex_col()
    .gap(8)
  let read = @ui.number_field([]).values([42]).read_only(true).flex_col().gap(8)
  panel(
    t, "Number field", "Parsing, clamping, stepping, and scrub gestures. Alt uses the small step, Shift the large step, and Return commits.",
  ).children([
    root.children([
      root
      .slot(@ui.ScrubArea)
      .label("Drag here to change quantity")
      .height(24)
      .cursor("ew-resize"),
      number_controls(t, root),
      root
      .slot(@ui.ScrubAreaCursor)
      .children(
        [
          ..if get(state.get(), "scrubbing") == Json::boolean(true) {
            [label(t, "Scrubbing")]
          } else {
            []
          },
        ],
      ),
    ]),
    label(t, "Read-only"),
    read.children([number_controls(t, read)]),
    readout(t, state),
  ])
}

///|
fn otp_field_demo(t : Theme) -> @ui.Element {
  let (code, set_code) = @ui.create_signal("")
  let (completed, set_completed) = @ui.create_signal("not yet")
  let root = @ui.otp_field([])
    .length(6)
    .variant("numeric")
    .bind_value(() => code())
    .on_change(data => {
      if get(data, "value") is String(v) {
        set_code(v)
      }
      if get(data, "complete") is String(v) {
        set_completed(v)
      }
    })
    .flex_row()
    .gap(10)
    .items_center()
  for index in 0..<6 {
    if index == 3 {
      ignore(
        root.children([
          root.slot(@ui.Separator, index~).children([icon("minus")]),
        ]),
      )
    }
    ignore(
      root.children([
        editor(t, root.slot(@ui.Input, index~))
        .width(36)
        .height(40)
        .text_align("center")
        .font_size(18),
      ]),
    )
  }
  panel(
    t, "OTP field", "Six slots: typing advances, paste distributes, and Backspace walks back. Completion is reported once per completed value.",
  ).children([
    root,
    row().children([
      button(t, "Fill 123456", () => set_code("123456")),
      button(t, "Clear", () => set_code("")),
    ]),
    note(t, () => "code \{code()} · completed \{completed()}"),
  ])
}

///|
fn date_field_demo(t : Theme) -> @ui.Element {
  let us = @reactive.signal("2026-09-03")
  let iso = @reactive.signal("2026-01-15")
  let content = panel(
    t, "Date field", "Civil dates with no time zone. Up/Down steps a segment; typing advances; min/max constrain the result.",
  )
  for
    entry in [
      (us, "mdy", ["month", "day", "year"], "/"),
      (iso, "ymd", ["year", "month", "day"], "-"),
    ] {
    let (value, order, segments, separator) = entry
    let field = @ui.date_field([])
      .bind_civil_value(() => value.get())
      .on_value_change(v => value.set(v.unwrap_or("")))
      .civil_minimum("2026-01-01")
      .civil_maximum("2026-12-31")
      .segment_order(order)
      .flex_row()
      .items_center()
      .gap(10)
    for index, name in segments {
      if index > 0 {
        ignore(field.children([label(t, separator)]))
      }
      ignore(
        field.children([
          editor(t, field.slot(@ui.Segment, value=name))
          .width(if name == "year" { 64 } else { 40 })
          .px(4)
          .text_align("center"),
        ]),
      )
    }
    ignore(content.children([field]))
  }
  content.children([note(t, () => "US \{us.get()} · ISO \{iso.get()}")])
}

///|
fn time_field_demo(t : Theme) -> @ui.Element {
  let at = @reactive.signal("09:30")
  let precise = @reactive.signal("14:05:30")
  let content = panel(
    t, "Time field", "12-hour and 24-hour civil times. Seconds are declared only for the precise field.",
  )
  for
    entry in [
      (at, "h12", ["hour", "minute", "period"]),
      (precise, "h23", ["hour", "minute", "second"]),
    ] {
    let (value, order, segments) = entry
    let field = @ui.time_field([])
      .bind_civil_value(() => value.get())
      .on_value_change(v => value.set(v.unwrap_or("")))
      .segment_order(order)
      .variant(if order == "h23" { "seconds" } else { "" })
      .flex_row()
      .items_center()
      .gap(10)
    for index, name in segments {
      if index > 0 && name != "period" {
        ignore(field.children([label(t, ":")]))
      }
      ignore(
        field.children([
          editor(t, field.slot(@ui.Segment, value=name))
          .width(if name == "period" { 48 } else { 40 })
          .px(4)
          .text_align("center"),
        ]),
      )
    }
    ignore(content.children([field]))
  }
  content.children([
    note(t, () => "12-hour \{at.get()} · precise \{precise.get()}"),
  ])
}

// Civil-date arithmetic for the declared six-week grid. Interaction and
// constraints stay in the native calendar; this only supplies date labels.

///|
fn pad2(n : Int) -> String {
  if n < 10 {
    "0\{n}"
  } else {
    n.to_string()
  }
}

///|
fn month_parts(value : String) -> (Int, Int) {
  let parts = value
    .split("-")
    .map(s => @string.parse_int(s.to_owned()) catch { _ => 1 })
    .to_array()
  (parts.get(0).unwrap_or(2026), parts.get(1).unwrap_or(9))
}

///|
fn month_shift(value : String, delta : Int) -> String {
  let (year, month) = month_parts(value)
  let total = year * 12 + month - 1 + delta
  "\{total / 12}-\{pad2(total % 12 + 1)}"
}

///|
fn month_days(year : Int, month : Int) -> Int {
  if month == 2 {
    if year % 4 == 0 && (year % 100 != 0 || year % 400 == 0) {
      29
    } else {
      28
    }
  } else if [4, 6, 9, 11].contains(month) {
    30
  } else {
    31
  }
}

///|
fn month_grid(value : String) -> Array[Array[String]] {
  let (year, month) = month_parts(value)
  let y = if month < 3 { year - 1 } else { year }
  let offsets = [0, 3, 2, 5, 0, 3, 5, 1, 4, 6, 2, 4]
  let weekday = (y + y / 4 - y / 100 + y / 400 + offsets[month - 1] + 1) % 7
  let (py, pm) = month_parts(month_shift(value, -1))
  let previous = month_days(py, pm)
  let days = month_days(year, month)
  Array::makei(6, week => {
    Array::makei(7, day => {
      let n = week * 7 + day - weekday + 1
      if n < 1 {
        "\{py}-\{pad2(pm)}-\{pad2(previous + n)}"
      } else if n > days {
        month_shift(value, 1) + "-\{pad2(n - days)}"
      } else {
        value + "-\{pad2(n)}"
      }
    })
  })
}

///|
fn calendar_demo(t : Theme) -> @ui.Element {
  let (day, set_day) = @ui.create_signal("2026-09-03")
  let (month, set_month) = @ui.create_signal("2026-09")
  let (focused, set_focused) = @ui.create_signal("2026-09-03")
  let root = @ui.calendar([])
    .bind_civil_value(() => day())
    .civil_minimum("2026-01-01")
    .civil_maximum("2026-12-31")
    .first_weekday(0)
    .on_change(data => {
      if get(data, "value") is String(v) {
        set_day(v)
      }
      if get(data, "month") is String(v) {
        set_month(v)
      }
      if get(data, "focused") is String(v) {
        set_focused(v)
      }
    })
    .flex_col()
    .gap(8)
  let headings = row().gap(3)
  for name in ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"] {
    ignore(
      headings.children([
        @ui.text(name).width(32).text_align("center").font_size(10),
      ]),
    )
  }
  ignore(root.children([headings]))
  ignore(
    root.children_keyed(() => month_grid(month()), week => week[0], week => {
      let line = root
        .slot(
          @ui.Week,
          index=month_grid(@ui.untrack(month)).search(week.peek()).unwrap_or(0),
        )
        .flex_row()
        .gap(3)
      for date in week.peek() {
        ignore(
          line.children([
            root
            .slot(@ui.Day, value=date)
            .label(date.view(start_offset=8).to_owned())
            .width(32)
            .height(28)
            .border_radius(7)
            .display("flex")
            .items_center()
            .justify_center()
            .font_size(11)
            .bind_style(() => {
              @ui.style()
              .bg(t.color(if day() == date { "accent" } else { "alt" }))
              .text_color(
                t.color(
                  if day() == date {
                    "on-accent"
                  } else if date.has_prefix(month()) {
                    "ink"
                  } else {
                    "faint"
                  },
                ),
              )
            })
            .bind_json(@protocol.HOVER_STYLE, () => {
              @ui.style()
              .bg(t.color(if day() == date { "accent" } else { "hover" }))
              .json()
            })
            .bind_json(@protocol.FOCUS_STYLE, () => focus_ring(t)),
          ]),
        )
      }
      line
    }),
  )
  panel(
    t, "Calendar", "An application-declared grid with native keyboard navigation, one Tab stop, and reported month changes.",
  ).children([
    row().children([
      button(t, "Previous month", () => {
        set_month(month_shift(@ui.untrack(month), -1))
      }),
      note(t, () => month()),
      button(t, "Next month", () => {
        set_month(month_shift(@ui.untrack(month), 1))
      }),
    ]),
    root,
    note(t, () => {
      "selected \{day()} · showing \{month()} · tab stop \{focused()}"
    }),
  ])
}
