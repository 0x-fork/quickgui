///|
fn hex(value : String) -> @ui.Color {
  fn channel(offset : Int) -> Int {
    @string.parse_int(
      value.view(start_offset=offset, end_offset=offset + 2).to_owned(),
      base=16,
    ) catch {
      _ => 0
    }
  }
  @ui.rgba8(
    channel(1),
    channel(3),
    channel(5),
    if value.length() == 9 {
      channel(7)
    } else {
      255
    },
  )
}

///|
fn picker(t : Theme, node : @ui.Element, title : String) -> @ui.Element {
  editor(t, node)
  .width(280)
  .aria(title)
  .bind_appearance(() => {
    "width": 280,
    "rowHeight": 30,
    "maxVisibleRows": 7,
    "fontSize": 12,
    "radius": 10,
    "background": t.color("popup"),
    "color": t.color("ink"),
    "highlightBackground": t.color("accent"),
    "highlightColor": t.color("on-accent"),
    "selectedBackground": t.color("selection"),
    "mutedColor": t.color("muted"),
  })
}

///|
fn autocomplete_demo(t : Theme) -> @ui.Element {
  let state : @reactive.Signal[Json] = @reactive.signal({})
  let items : Json = [
    { "value": "window", "label": "Window" },
    { "value": "widget", "label": "Widget" },
    { "value": "tabs", "label": "Tabs" },
    { "value": "table", "label": "Table" },
    { "value": "toolbar", "label": "Toolbar" },
    { "value": "tooltip", "label": "Tooltip" },
  ]
  panel(
    t, "Autocomplete", "Free-form text with suggestions. The core filters, ranks, and paints the popup rows.",
  ).children([
    picker(
      t,
      observe(
        @ui.autocomplete([])
        .items(items)
        .filter_mode("fuzzy")
        .placeholder("Type win, tab, or tool"),
        state,
      ),
      "Search components",
    ),
    readout(t, state),
  ])
}

///|
fn combobox_demo(t : Theme) -> @ui.Element {
  let fruit : @reactive.Signal[Json] = @reactive.signal({ "value": "apple" })
  let state : @reactive.Signal[Json] = @reactive.signal({})
  let (tags, set_tags) = @reactive.create_signal(["go"])
  let fruits : Json = [
    { "value": "apple", "label": "Apple", "group": "Common" },
    { "value": "banana", "label": "Banana", "group": "Common" },
    { "value": "lychee", "label": "Lychee", "group": "Tropical" },
    { "value": "mango", "label": "Mango", "group": "Tropical" },
  ]
  let languages : Json = [
    { "value": "go", "label": "Go" },
    { "value": "moonbit", "label": "MoonBit" },
    { "value": "zig", "label": "Zig" },
    { "value": "swift", "label": "Swift" },
    { "value": "typescript", "label": "TypeScript" },
  ]
  let many = picker(
    t,
    @ui.combobox([])
    .items(languages)
    .filter_mode("startsWith")
    .multiple(true)
    .auto_highlight(true)
    .placeholder("Add tags")
    .bind_values(() => tags().to_json())
    .on_change(data => {
      if get(data, "chipValues") is Array(_) {
        set_tags(strings(get(data, "chipValues")))
      }
      state.set(data)
    }),
    "Tags",
  )
  let chips = many
    .slot(@ui.Chips)
    .flex_row()
    .gap(6)
    .flex_wrap("wrap")
    .children_keyed(() => tags(), v => v, value => {
      let index = () => tags().search(value.get()).unwrap_or(0)
      let chip = many
        .slot(@ui.Chip)
        .bind_item_index(() => Float::from_int(index()))
        .flex_row()
        .items_center()
        .gap(4)
        .height(22)
        .padding_left(8)
        .padding_right(6)
        .border_radius(11)
        .bind_style(() => @ui.style().bg(t.color("selection")))
      chip.children([
        label(t, value.peek()),
        many
        .slot(@ui.ChipRemove)
        .children([icon("close", size=12)])
        .bind_item_index(() => Float::from_int(index()))
        .aria("Remove \{value.peek()}")
        .width(16)
        .height(16),
      ])
    })
  panel(
    t, "Combobox", "A constrained picker and a multiple combobox. Removable chips reflect the values reported by the core.",
  ).children([
    picker(
      t,
      observe(
        @ui.combobox([])
        .items(fruits)
        .active_value("apple")
        .filter_mode("contains")
        .auto_highlight(true)
        .placeholder("Pick a fruit"),
        fruit,
      ),
      "Fruit",
    ),
    col().children([many, chips]),
    readout(t, fruit),
    note(t, () => "tags: " + tags().join(", ")),
    readout(t, state),
  ])
}

///|
fn select_demo(t : Theme) -> @ui.Element {
  let (theme, set_theme) = @reactive.create_signal("system")
  let (sizes, set_sizes) = @reactive.create_signal(["m"])
  let state : @reactive.Signal[Json] = @reactive.signal({})
  let single = picker(
      t,
      @ui.select([])
      .items([
        { "value": "light", "label": "Light" },
        { "value": "dark", "label": "Dark" },
        { "value": "system", "label": "Match system" },
      ])
      .bind_active_value(() => theme())
      .on_value_change(v => set_theme(v.unwrap_or(""))),
      "Theme",
    )
    .flex_row()
    .items_center()
    .justify_content("space-between")
  let multiple = picker(
      t,
      @ui.select([])
      .items([
        { "value": "s", "label": "Small" },
        { "value": "m", "label": "Medium" },
        { "value": "l", "label": "Large" },
        { "value": "xl", "label": "Extra large" },
      ])
      .multiple(true)
      .bind_values(() => sizes().to_json())
      .align_item_with_trigger(true)
      .on_change(data => {
        if get(data, "selectedValues") is Array(_) {
          set_sizes(strings(get(data, "selectedValues")))
        }
        state.set(data)
      }),
      "Sizes",
    )
    .flex_row()
    .items_center()
    .justify_content("space-between")
  panel(
    t, "Select", "Single and multiple selection. The popup uses the declared appearance, with ordered selected values.",
  ).children([
    single.children([
      single
      .slot(@ui.Value)
      .bind_label(() => {
        match theme() {
          "light" => "Light"
          "dark" => "Dark"
          _ => "Match system"
        }
      }),
      single.slot(@ui.Icon).children([icon("chevron")]),
    ]),
    multiple.children([
      multiple
      .slot(@ui.Value)
      .bind_label(() => {
        if sizes().is_empty() {
          "Pick sizes"
        } else {
          sizes().join(", ")
        }
      }),
      multiple.slot(@ui.Icon).children([icon("chevron")]),
      multiple
      .slot(@ui.Positioner)
      .children([
        multiple.slot(@ui.ScrollUpArrow),
        multiple.slot(@ui.ScrollDownArrow),
      ])
      .side("bottom")
      .align("start")
      .side_offset(6),
    ]),
    note(t, () => "theme \{theme()} · sizes \{sizes().to_json().stringify()}"),
    readout(t, state),
  ])
}
