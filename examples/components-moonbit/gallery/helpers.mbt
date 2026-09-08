///|
priv struct Theme {
  appearance : @reactive.Signal[String]
}

///|
fn theme() -> Theme {
  { appearance: @reactive.signal("light"), }
}

///|
fn Theme::color_text(self : Theme, name : String) -> String {
  let colors = match name {
    "window" => ("#f4f5f7", "#0e1117")
    "sidebar" => ("#ebedf1", "#131924")
    "panel" => ("#ffffff", "#171e2a")
    "alt" => ("#f8f9fb", "#1b2331")
    "control" => ("#ffffff", "#1c2432")
    "hover" => ("#eef1f6", "#263042")
    "active" => ("#e2e8f4", "#2f3a50")
    "ink" => ("#131820", "#e7edf7")
    "muted" => ("#5a6472", "#98a4b6")
    "faint" => ("#8b94a3", "#6e7a8c")
    "border" => ("#d9dde4", "#2a3446")
    "accent" => ("#2563eb", "#5b93f7")
    "accent-hover" => ("#1d4fd7", "#7aa7ff")
    "on-accent" => ("#ffffff", "#08111f")
    "selection" => ("#dbe6fd", "#1f2d47")
    "popup" => ("#ffffff", "#171e2a")
    "backdrop" => ("#1e293b66", "#010409aa")
    "track" => ("#e4e7ec", "#252f41")
    "danger" => ("#b42318", "#f0736a")
    _ => ("#00000000", "#00000000")
  }
  if self.appearance.get() == "dark" {
    colors.1
  } else {
    colors.0
  }
}

///|
fn col() -> @ui.Element {
  @ui.div([]).flex_col().gap(8).min_width(0).flex_shrink(0)
}

///|
fn row() -> @ui.Element {
  @ui.div([]).flex_row().items_center().flex_wrap().gap(10).flex_shrink(0)
}

///|
fn label(t : Theme, text : String) -> @ui.Element {
  @ui.text(text)
  .font_size(12)
  .bind_style(() => @ui.style().text_color(t.color("ink")))
}

///|
fn muted(t : Theme, text : String) -> @ui.Element {
  @ui.text(text)
  .font_size(12)
  .line_height(18)
  .bind_style(() => @ui.style().text_color(t.color("muted")))
}

///|
fn note(t : Theme, read : () -> String) -> @ui.Element {
  @ui.text(read())
  .font_size(12)
  .line_height(18)
  .font_family("monospace")
  .bind_style(() => @ui.style().text_color(t.color("muted")))
}

///|
fn panel(t : Theme, title : String, hint : String) -> @ui.Element {
  col()
  .children([@ui.text(title).font_size(17).font_weight(700), muted(t, hint)])
  .gap(14)
  .padding(20)
  .border_radius(12)
  .border_width(1)
  .bind_style(() => {
    @ui.style().bg(t.color("panel")).border_color(t.color("border"))
  })
}

///|
fn control(
  t : Theme,
  node : @ui.Element,
  active? : () -> Bool = () => false,
  primary? : Bool = false,
) -> @ui.Element {
  node
  .flex_row()
  .items_center()
  .justify_center()
  .gap(6)
  .height(30)
  .flex_shrink(0)
  .px(12)
  .border_radius(8)
  .border_width(1)
  .font_size(13)
  .cursor("default")
  .user_select("none")
  .app_region("no-drag")
  .bind_style(() => {
    @ui.style()
    .bg(
      t.color(
        if primary {
          "accent"
        } else if active() {
          "selection"
        } else {
          "control"
        },
      ),
    )
    .text_color(t.color(if primary { "on-accent" } else { "ink" }))
    .border_color(t.color("border"))
  })
  .bind_json(@protocol.HOVER_STYLE, () => {
    @ui.style()
    .bg(
      t.color(
        if primary {
          "accent-hover"
        } else if active() {
          "selection"
        } else {
          "hover"
        },
      ),
    )
    .json()
  })
  .bind_json(@protocol.FOCUS_STYLE, () => focus_ring(t))
  .property(@protocol.DISABLED_STYLE, @protocol.Text("{\"opacity\":0.45}"))
}

///|
fn button(
  t : Theme,
  title : String,
  click : () -> Unit,
  primary? : Bool = false,
) -> @ui.Element {
  control(t, @ui.button(title), primary~).on_click(click)
}

///|
fn editor(t : Theme, node : @ui.Element) -> @ui.Element {
  node
  .height(30)
  .px(10)
  .border_radius(8)
  .border_width(1)
  .font_size(13)
  .app_region("no-drag")
  .bind_style(() => {
    @ui.style()
    .bg(t.color("alt"))
    .border_color(t.color("border"))
    .text_color(t.color("ink"))
  })
  .bind_json(@protocol.FOCUS_STYLE, () => focus_ring(t))
  .property(@protocol.DISABLED_STYLE, @protocol.Text("{\"opacity\":0.45}"))
}

///|
fn input(
  t : Theme,
  value : @reactive.Signal[String],
  placeholder : String,
) -> @ui.Element {
  editor(t, @ui.input())
  .width(280)
  .placeholder(placeholder)
  .bind_value(() => value.get())
  .on_input(v => value.set(v))
}

///|
fn popup(t : Theme, node : @ui.Element, width : Int) -> @ui.Element {
  node
  .flex_col()
  .gap(8)
  .padding(14)
  .width(width)
  .border_radius(12)
  .border_width(1)
  .box_shadow("0 18px 40px #00000033")
  .bind_style(() => {
    @ui.style()
    .bg(t.color("popup"))
    .border_color(t.color("border"))
    .text_color(t.color("ink"))
  })
}

///|
fn framed(t : Theme, node : @ui.Element) -> @ui.Element {
  node
  .border_radius(10)
  .border_width(1)
  .bind_style(() => {
    @ui.style().bg(t.color("alt")).border_color(t.color("border"))
  })
}

///|
fn icon(name : String, size? : Int = 14) -> @ui.Element {
  let path = match name {
    "check" => "M3 6l2 2 4-4"
    "minus" => "M3 6h6"
    "plus" => "M3 6h6M6 3v6"
    "close" => "m3 3 6 6M3 9l6-6"
    "chevron" => "m3 4 3 3 3-3"
    "up" => "m3 8 3-4 3 4"
    _ => "m4 3 3 3-3 3"
  }
  @ui.svg(
    "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 12 12\" fill=\"none\" stroke=\"#000\" stroke-width=\"1.6\" stroke-linecap=\"round\" stroke-linejoin=\"round\"><path d=\"" +
    path +
    "\"/></svg>",
  )
  .width(size)
  .height(size)
  .flex_shrink(0)
}

///|
fn get(data : Json, key : String) -> Json {
  if data is Object(fields) {
    fields.get(key).unwrap_or(Json::null())
  } else {
    Json::null()
  }
}

///|
fn string(data : Json, fallback? : String = "") -> String {
  if data is String(value) {
    value
  } else {
    fallback
  }
}

///|
fn number(data : Json, fallback? : Double = 0) -> Double {
  if data is Number(value, ..) {
    value
  } else {
    fallback
  }
}

///|
fn strings(data : Json) -> Array[String] {
  if data is Array(items) {
    items.filter_map(item => {
      if item is String(value) {
        Some(value)
      } else {
        None
      }
    })
  } else {
    []
  }
}

///|
fn observe(node : @ui.Element, state : @reactive.Signal[Json]) -> @ui.Element {
  node.on_change(next => {
    let fields = if state.peek() is Object(fields) {
      fields.copy()
    } else {
      Map([])
    }
    if next is Object(values) {
      for key, value in values {
        fields[key] = value
      }
    }
    state.set(fields.to_json())
  })
}

///|
fn readout(t : Theme, state : @reactive.Signal[Json]) -> @ui.Element {
  note(t, () => state.get().stringify())
  .max_width("100%")
  .overflow_wrap("anywhere")
}

///|
fn check_indicator(
  t : Theme,
  node : @ui.Element,
  checked : () -> Bool,
  mixed? : () -> Bool = () => false,
) -> @ui.Element {
  node
  .children(
    [
      ..if mixed() {
        [icon("minus", size=12)]
      } else {
        [@ui.div([..if checked() { [icon("check", size=12)] } else { [] }])]
      },
    ],
  )
  .width(16)
  .height(16)
  .flex_shrink(0)
  .border_radius(5)
  .border_width(1)
  .display("flex")
  .items_center()
  .justify_center()
  .bind_style(() => {
    @ui.style()
    .bg(t.color(if checked() || mixed() { "accent" } else { "control" }))
    .border_color(
      t.color(if checked() || mixed() { "accent" } else { "border" }),
    )
    .text_color(t.color("on-accent"))
  })
}

///|
fn checkbox(
  t : Theme,
  title : String,
  checked : () -> Bool,
  change : (Bool) -> Unit,
  mixed? : () -> Bool = () => false,
  read_only? : Bool = false,
) -> @ui.Element {
  let root = @ui.checkbox([])
    .bind_checked(checked)
    .bind_indeterminate(mixed)
    .read_only(read_only)
    .on_checked_change(change)
    .flex_row()
    .items_center()
    .gap(8)
    .height(28)
    .px(8)
    .border_radius(6)
    .cursor("default")
    .user_select("none")
    .app_region("no-drag")
    .aria(title)
    .bind_json(@protocol.FOCUS_STYLE, () => focus_ring(t))
  root.children([
    check_indicator(t, root.slot(@ui.Indicator), checked, mixed~),
    label(t, title),
  ])
}

///|
fn switch_control(
  t : Theme,
  title : String,
  value : @reactive.Signal[Bool],
  read_only? : Bool = false,
) -> @ui.Element {
  let root = @ui.switch([])
    .bind_checked(() => value.get())
    .read_only(read_only)
    .on_checked_change(v => value.set(v))
    .width(44)
    .height(24)
    .border_radius(12)
    .padding(2)
    .flex_shrink(0)
    .flex_row()
    .items_center()
    .aria(title)
    .transition("background-color 160ms ease-out")
    .bind_style(() => {
      @ui.style().bg(t.color(if value.get() { "accent" } else { "track" }))
    })
    .bind_json(@protocol.FOCUS_STYLE, () => focus_ring(t))
  row().children([
    root.children([
      root
      .slot(@ui.Thumb)
      .width(20)
      .height(20)
      .flex_shrink(0)
      .border_radius(10)
      .bg(hex("#ffffff"))
      .transition("transform 160ms ease-out")
      .bind_style(() => {
        @ui.style().transform(
          if value.get() {
            "translateX(20px)"
          } else {
            "translateX(0px)"
          },
        )
      }),
    ]),
    label(t, title),
  ])
}

///|
fn Theme::color(self : Theme, name : String) -> @ui.Color {
  hex(self.color_text(name))
}

///|
fn @ui.Element::aria(self : @ui.Element, title : String) -> @ui.Element {
  self.property(@protocol.ACCESSIBILITY_LABEL, @protocol.Text(title))
}

///|
fn focus_ring(t : Theme) -> Json {
  { "outline": "2px solid " + t.color_text("accent") }
}
