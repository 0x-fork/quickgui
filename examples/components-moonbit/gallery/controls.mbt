///|
fn slider_parts(t : Theme, root : @ui.Element, count : Int) -> @ui.Element {
  let control = root
    .slot(@ui.Control)
    .width(420)
    .height(24)
    .position("relative")
    .display("flex")
    .items_center()
  let track = root
    .slot(@ui.Track)
    .children([
      root
      .slot(@ui.Indicator)
      .height(6)
      .border_radius(3)
      .bind_style(() => @ui.style().bg(t.color("accent"))),
    ])
    .height(6)
    .width("100%")
    .border_radius(3)
    .bind_style(() => @ui.style().bg(t.color("track")))
  ignore(control.children([track]))
  for index in 0..<count {
    ignore(
      control.children([
        root
        .slot(@ui.Thumb, index~)
        .width(16)
        .height(16)
        .border_radius(8)
        .border_width(2)
        .bg(hex("#ffffff"))
        .bind_style(() => @ui.style().border_color(t.color("accent")))
        .aria("Value \{index + 1}")
        .bind_json(@protocol.FOCUS_STYLE, () => focus_ring(t)),
      ]),
    )
  }
  root.children([control])
}

///|
fn slider_demo(t : Theme) -> @ui.Element {
  let volume : @reactive.Signal[Json] = @reactive.signal({ "values": [40] })
  let range : @reactive.Signal[Json] = @reactive.signal({ "values": [20, 70] })
  let first = observe(
    @ui.slider([])
    .values([40])
    .minimum(0)
    .maximum(100)
    .step(5)
    .large_step(25)
    .format("percent"),
    volume,
  )
  let second = observe(
    @ui.slider([])
    .values([20, 70])
    .minimum(0)
    .maximum(100)
    .step(1)
    .min_steps_between_values(5),
    range,
  )
  panel(
    t, "Slider", "Single and range sliders. Pointer capture and geometry stay native; keyboard arrows and PageUp/PageDown step values.",
  ).children([
    first.slot(@ui.Label).label("Volume"),
    slider_parts(t, first, 1),
    readout(t, volume),
    second.slot(@ui.Label).label("Visible range"),
    slider_parts(t, second, 2),
    readout(t, range),
  ])
}

///|
fn meter_demo(t : Theme) -> @ui.Element {
  let (value, set_value) = @ui.create_signal(62.0)
  let state : @reactive.Signal[Json] = @reactive.signal({})
  let meter = observe(
      @ui.meter([])
      .bind_values(() => [value()].to_json())
      .minimum(0)
      .maximum(100)
      .low(25)
      .high(80)
      .optimum(50)
      .format("percent"),
      state,
    )
    .flex_col()
    .gap(8)
  let slider = @ui.slider([])
    .values([62])
    .minimum(0)
    .maximum(100)
    .on_change(data => {
      if get(data, "values") is Array([Number(n, ..), ..]) {
        set_value(n)
      }
    })
  panel(
    t, "Meter", "A measurement with low, high, and optimum thresholds. The slider adjusts the gauge.",
  ).children([
    meter.children([
      meter.slot(@ui.Label).label("Storage capacity"),
      meter
      .slot(@ui.Track)
      .children([
        meter
        .slot(@ui.Indicator)
        .height(8)
        .border_radius(4)
        .bind_style(() => {
          @ui.style().bg(
            if value() < 25 {
              t.color("danger")
            } else if value() > 80 {
              hex("#d97706")
            } else {
              t.color("accent")
            },
          )
        }),
      ])
      .width(420)
      .height(8)
      .border_radius(4)
      .bind_style(() => @ui.style().bg(t.color("track"))),
      meter.slot(@ui.Value).bind_label(() => "\{value().to_int()}%"),
    ]),
    slider_parts(t, slider, 1),
    readout(t, state),
  ])
}

///|
fn progress_demo(t : Theme) -> @ui.Element {
  let (done, set_done) = @ui.create_signal(3)
  let (indeterminate, set_indeterminate) = @ui.create_signal(false)
  let state : @reactive.Signal[Json] = @reactive.signal({})
  let progress = observe(
      @ui.progress([])
      .bind_values(() => [done()].to_json())
      .maximum(12)
      .bind_indeterminate(() => indeterminate())
      .format("fraction"),
      state,
    )
    .flex_col()
    .gap(8)
  panel(
    t, "Progress", "Determinate and indeterminate progress. Native status and completion update only when the value changes.",
  ).children([
    progress.children([
      progress.slot(@ui.Label).label("Uploading files"),
      progress
      .slot(@ui.Track)
      .children([
        progress
        .slot(@ui.Indicator)
        .height(8)
        .border_radius(4)
        .bind_style(() => @ui.style().bg(t.color("accent"))),
      ])
      .width(420)
      .height(8)
      .border_radius(4)
      .bind_style(() => @ui.style().bg(t.color("track"))),
      progress.slot(@ui.Value).bind_label(() => "\{done()} / 12"),
    ]),
    row().children([
      button(t, "-1", () => set_done((@ui.untrack(done) - 1).max(0))),
      button(t, "+1", () => set_done((@ui.untrack(done) + 1).min(12))),
      button(t, "Complete", () => set_done(12)),
      control(
        t,
        @ui.button(
          if indeterminate() {
            "Determinate"
          } else {
            "Indeterminate"
          },
        ),
      ).on_click(() => set_indeterminate(!indeterminate())),
    ]),
    readout(t, state),
  ])
}

///|
fn radio_group(
  t : Theme,
  values : Array[String],
  selected : @reactive.Signal[String],
  read_only? : Bool = false,
) -> @ui.Element {
  let group = @ui.radio_group([])
    .bind_active_value(() => selected.get())
    .required(true)
    .read_only(read_only)
    .on_value_change(v => selected.set(v.unwrap_or("")))
    .flex_col()
    .gap(8)
  for value in values {
    let item = group
      .slot(@ui.Item, value~)
      .flex_row()
      .items_center()
      .gap(8)
      .height(28)
      .aria(value)
    ignore(
      group.children([
        item.children([
          item
          .slot(@ui.Indicator)
          .width(15)
          .height(15)
          .border_radius(8)
          .bind_style(() => {
            @ui.style()
            .border_width(if selected.get() == value { 4 } else { 1 })
            .border_color(
              t.color(if selected.get() == value { "accent" } else { "border" }),
            )
            .bg(t.color("control"))
          }),
          label(t, value),
        ]),
      ]),
    )
  }
  group
}

///|
fn radio_demo(t : Theme) -> @ui.Element {
  let choice = @reactive.signal("system")
  let read = @reactive.signal("b")
  panel(
    t, "Radio", "A required exclusive group. Arrow keys move and select; the read-only group remains focusable.",
  ).children([
    radio_group(t, ["light", "dark", "system"], choice),
    label(t, "Read-only"),
    radio_group(t, ["a", "b", "c"], read, read_only=true),
    note(t, () => "theme \{choice.get()} · read-only \{read.get()}"),
  ])
}

///|
fn separator_demo(t : Theme) -> @ui.Element {
  panel(t, "Separator", "Horizontal and vertical semantic separators.").children([
      label(t, "Above"),
      @ui.separator([])
      .orientation("horizontal")
      .height(1)
      .width("100%")
      .bind_style(() => @ui.style().bg(t.color("border"))),
      label(t, "Below"),
      row().children([
        label(t, "Cut"),
        @ui.separator([])
        .orientation("vertical")
        .height(20)
        .width(1)
        .bind_style(() => @ui.style().bg(t.color("border"))),
        label(t, "Copy"),
        label(t, "Paste"),
      ]),
    ],
  )
}

///|
fn splitter_demo(t : Theme) -> @ui.Element {
  let state : @reactive.Signal[Json] = @reactive.signal({
    "sizes": [200, 160, 160],
  })
  let root = framed(
      t,
      observe(
        @ui.splitter([])
        .values([200, 160, 160])
        .items([
          { "min": 80 },
          { "min": 80 },
          { "min": 60, "collapsible": true },
        ]),
        state,
      ),
    )
    .height(140)
    .flex_row()
    .overflow("hidden")
  for index, title in ["Sidebar", "Editor", "Preview (collapsible)"] {
    if index > 0 {
      ignore(
        root.children([
          root
          .slot(@ui.Handle, index=index - 1)
          .width(6)
          .height("100%")
          .cursor("col-resize")
          .aria("Resize \{title}")
          .bind_style(() => @ui.style().bg(t.color("border"))),
        ]),
      )
    }
    ignore(
      root.children([
        root
        .slot(@ui.Pane, index~)
        .children([label(t, title)])
        .display("flex")
        .items_center()
        .justify_center()
        .min_width(0)
        .overflow("hidden"),
      ]),
    )
  }
  panel(
    t, "Splitter", "Three panes with minimum sizes and a collapsible last pane. Drag a handle or use the arrow keys.",
  ).children([root, readout(t, state)])
}

///|
fn switch_demo(t : Theme) -> @ui.Element {
  let wifi = @reactive.signal(true)
  let beta = @reactive.signal(false)
  let managed = @reactive.signal(true)
  panel(
    t, "Switch", "Controlled switches with native track and thumb transitions. Read-only settings refuse changes.",
  ).children([
    switch_control(t, "Wi-Fi", wifi),
    switch_control(t, "Enable beta features", beta),
    switch_control(t, "Managed by administrator", managed, read_only=true),
    note(t, () => {
      "wifi \{wifi.get()} · beta \{beta.get()} · managed \{managed.get()}"
    }),
  ])
}

///|
fn tabs_demo(t : Theme) -> @ui.Element {
  let (active, set_active) = @ui.create_signal("overview")
  let state : @reactive.Signal[Json] = @reactive.signal({})
  let tabs = @ui.tabs([])
    .bind_active_value(() => active())
    .activate_on_focus(true)
    .on_change(data => {
      if get(data, "value") is String(value) {
        set_active(value)
      }
      state.set(data)
    })
    .flex_col()
    .gap(12)
  let list = tabs.slot(@ui.List).flex_row().gap(4).position("relative")
  for index, name in ["overview", "usage", "limits"] {
    ignore(
      list.children([
        control(t, tabs.slot(@ui.Tab, value=name, index~).label(name), active=() => {
          active() == name
        }),
      ]),
    )
  }
  ignore(
    list.children([
      tabs
      .slot(@ui.Indicator)
      .height(2)
      .position("absolute")
      .bottom(0)
      .bind_style(() => @ui.style().bg(t.color("accent"))),
    ]),
  )
  ignore(tabs.children([list]))
  for
    entry in [
      ("overview", "Each tab exposes its role and selected state."),
      (
        "usage", "Panels mount on demand; inactive content has no input or layout work.",
      ),
      ("limits", "Arrow keys activate tabs automatically in this example."),
    ] {
    let (name, body) = entry
    ignore(
      tabs.children(
        [
          ..if active() == name {
            [tabs.slot(@ui.Panel, value=name).children([muted(t, body)])]
          } else {
            []
          },
        ],
      ),
    )
  }
  panel(
    t, "Tabs", "Automatic activation and a native indicator. The gallery sidebar uses manual activation.",
  ).children([tabs, note(t, () => "active \{active()}"), readout(t, state)])
}

///|
fn toggle_demo(t : Theme) -> @ui.Element {
  let bold = @reactive.signal(false)
  let pinned = @reactive.signal(true)
  let controls = row()
  for entry in [("Bold", bold), ("Pinned", pinned)] {
    let (title, value) = entry
    let toggle = @ui.toggle([])
      .bind_pressed(() => value.get())
      .on_change(data => {
        if get(data, "pressed") is True {
          value.set(true)
        } else if get(data, "pressed") is False {
          value.set(false)
        }
      })
    ignore(
      controls.children([
        control(t, toggle, active=() => value.get()).children([
          label(t, title),
          toggle
          .slot(@ui.Indicator)
          .width(6)
          .height(6)
          .border_radius(3)
          .bind_style(() => @ui.style().bg(t.color("accent"))),
        ]),
      ]),
    )
  }
  panel(
    t, "Toggle", "Independent pressed buttons with a native indicator and accessible pressed state.",
  ).children([
    controls,
    note(t, () => "bold \{bold.get()} · pinned \{pinned.get()}"),
  ])
}

///|
fn toggle_group_demo(t : Theme) -> @ui.Element {
  let single = @reactive.signal(["left"])
  let multiple = @reactive.signal(["bold"])
  let content = panel(
    t, "Toggle group", "Single and multiple selection, roving focus, and a disabled item.",
  )
  for
    entry in [
      (["left", "center", "right"], single, false),
      (["bold", "italic", "underline"], multiple, true),
    ] {
    let (names, selected, many) = entry
    let group = @ui.toggle_group([])
      .bind_values(() => selected.get().to_json())
      .multiple(many)
      .flex_row()
      .gap(4)
      .items(
        names
        .map(v => ({ "value": v, "disabled": v == "underline" } : Json))
        .to_json(),
      )
      .on_change(data => {
        if get(data, "pressed") is Array(_) {
          selected.set(strings(get(data, "pressed")))
        }
      })
    for index, value in names {
      ignore(
        group.children([
          control(
            t,
            group
            .slot(@ui.Item, value~, index~)
            .label(value)
            .disabled(value == "underline"),
            active=() => selected.get().contains(value),
          ),
        ]),
      )
    }
    ignore(content.children([group]))
  }
  content.children([
    note(t, () => {
      "single \{single.get().to_json().stringify()} · multiple \{multiple.get().to_json().stringify()}"
    }),
  ])
}

///|
fn toolbar_demo(t : Theme) -> @ui.Element {
  let (active, set_active) = @ui.create_signal("select")
  let (query, set_query) = @ui.create_signal("")
  let state : @reactive.Signal[Json] = @reactive.signal({})
  let bar = @ui.toolbar([])
    .orientation("horizontal")
    .items([
      { "value": "select" },
      { "value": "pen" },
      { "value": "erase", "disabled": true },
      { "value": "share" },
      { "value": "docs" },
      { "value": "find", "kind": "input" },
    ])
    .bind_active_value(() => active())
    .on_change(data => {
      if get(data, "active") is String(value) {
        set_active(value)
      }
      state.set(data)
    })
    .flex_row()
    .items_center()
    .gap(6)
  for index, name in ["select", "pen", "erase"] {
    ignore(
      bar.children([
        control(
          t,
          bar
          .slot(@ui.Item, value=name, index~)
          .label(name)
          .disabled(name == "erase"),
          active=() => active() == name,
        ),
      ]),
    )
  }
  ignore(
    bar.children([
      bar
      .slot(@ui.Separator)
      .width(1)
      .height(20)
      .bind_style(() => @ui.style().bg(t.color("border"))),
    ]),
  )
  ignore(
    bar.children([
      control(t, bar.slot(@ui.Button, value="share", index=3).label("Share")),
    ]),
  )
  ignore(
    bar.children([
      control(t, bar.slot(@ui.Link, value="docs", index=4).label("Docs")),
    ]),
  )
  ignore(
    bar.children([
      editor(t, bar.slot(@ui.Input, value="find", index=5))
      .width(140)
      .placeholder("Find")
      .bind_value(() => query())
      .on_input(v => set_query(v)),
    ]),
  )
  panel(
    t, "Toolbar", "Buttons, a link, and an editor share one roving Tab stop.",
  ).children([
    bar,
    note(t, () => "active \{active()} · find \{query()}"),
    readout(t, state),
  ])
}
