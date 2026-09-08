///|
fn hex(value : String) -> @ui.Color {
  fn byte(start : Int) -> Int {
    @string.parse_int(
      value.view(start_offset=start, end_offset=start + 2).to_owned(),
      base=16,
    ) catch {
      _ => 0
    }
  }
  @ui.rgba8(
    byte(1),
    byte(3),
    byte(5),
    if value.length() == 9 {
      byte(7)
    } else {
      255
    },
  )
}

///|
fn Store::color_text(self : Store, name : String) -> String {
  let dark = self.appearance.get() == "dark"
  let pair = match name {
    "content" => ("#ffffff", "#1d1d1f")
    "alt" => ("#f6f6f8", "#232326")
    "raised" => ("#ffffff", "#2a2a2e")
    "text" => ("#1d1d1f", "#f2f2f5")
    "secondary" => ("#5f5f66", "#a9a9b2")
    "muted" => ("#9a9aa1", "#6f6f78")
    "border" => ("#e4e4e8", "#333338")
    "strong" => ("#cfcfd6", "#45454c")
    "hover" => ("#00000009", "#ffffff0d")
    "selection" => ("#0000001c", "#ffffff29")
    "accent" => ("#007aff", "#0a84ff")
    "success" => ("#1a7f37", "#4cc38a")
    "danger" => ("#cf222e", "#ff6b62")
    "warning" => ("#9a6700", "#e3b341")
    "added" => ("#e8ffee", "#15291c")
    "added-gutter" => ("#c9f2d4", "#1f4a2d")
    "removed" => ("#ffebe9", "#33191c")
    "removed-gutter" => ("#ffd0cc", "#5a2a2e")
    "hunk" => ("#eef4ff", "#1a2231")
    "hunk-text" => ("#3b6fd6", "#7aa7ff")
    "scrim" => ("#1a1a2266", "#00000099")
    _ => ("#00000000", "#00000000")
  }
  if dark {
    pair.1
  } else {
    pair.0
  }
}

///|
fn Store::color(self : Store, name : String) -> @ui.Color {
  hex(self.color_text(name))
}

///|
fn Store::palette(self : Store) -> Array[String] {
  if self.appearance.get() == "dark" {
    [
      "#4a86ff", "#3fbf7f", "#ff6b62", "#c792ea", "#f0a044", "#3ec1d3", "#e57bbd",
      "#8da2ff",
    ]
  } else {
    [
      "#2f6bff", "#1a9e63", "#d0432f", "#b56cd8", "#e08a1e", "#1b9fb7", "#c74f9d",
      "#6d7fd6",
    ]
  }
}

///|
fn Store::status_color(self : Store, code : String) -> @ui.Color {
  self.color(
    match code {
      "A" | "?" => "success"
      "D" => "danger"
      "R" | "C" => "accent"
      _ => "warning"
    },
  )
}

///|
fn col() -> @ui.Element {
  @ui.div([]).flex_col().min_width(0).min_height(0)
}

///|
fn row() -> @ui.Element {
  @ui.div([]).flex_row().items_center().min_width(0)
}

///|
fn fill() -> @ui.Element {
  col().flex_1()
}

///|
fn label(
  s : Store,
  value : () -> String,
  tone? : String = "text",
) -> @ui.Element {
  @ui.text(
    {
      let text = value()
      if text.length() > 16000 {
        text.view(end_offset=16000).to_owned() + "…"
      } else {
        text
      }
    },
  ).bind_style(() => @ui.style().text_color(s.color(tone)))
}

///|
fn caption(s : Store, value : String) -> @ui.Element {
  label(s, () => value, tone="muted").font_size(11).font_weight(600)
}

///|
fn line_style(s : Store, selected : () -> Bool) -> @ui.Style {
  @ui.style()
  .bg(s.color(if selected() { "selection" } else { "transparent" }))
  .text_color(s.color("text"))
}

///|
fn button(
  s : Store,
  title : String,
  click : () -> Unit,
  primary? : Bool = false,
  danger? : Bool = false,
  disabled? : () -> Bool = () => false,
  read? : (() -> String)? = None,
) -> @ui.Element {
  let element = match read {
    Some(read) => @ui.button(read())
    None => @ui.button(title)
  }
  element
  .flex_row()
  .items_center()
  .justify_center()
  .height(24)
  .flex_shrink(0)
  .px(10)
  .gap(5)
  .border_radius(6)
  .cursor("default")
  .user_select("none")
  .app_region("no-drag")
  .on_click(() => if !disabled() { s.window.with_owner(click) })
  .bind_disabled(disabled)
  .bind_style(() => {
    @ui.style()
    .bg(s.color(if primary { "accent" } else { "raised" }))
    .text_color(
      if primary {
        hex("#ffffff")
      } else {
        s.color(if danger { "danger" } else { "text" })
      },
    )
    .border_width(if primary { 0 } else { 1 })
    .border_color(s.color("strong"))
    .opacity(if disabled() { 0.4 } else { 1 })
  })
  .hover(style => style.opacity(0.8))
}

///|
fn field(
  s : Store,
  placeholder : String,
  value : @reactive.Signal[String],
  multiline? : Bool = false,
) -> @ui.Element {
  @ui.input()
  .on_input(text => value.set(text))
  .bind_value(() => value.get())
  .property(@protocol.PLACEHOLDER, @protocol.Text(placeholder))
  .property(@protocol.MULTILINE, @protocol.Boolean(multiline))
  .width("100%")
  .min_width(0)
  .height(if multiline { 70 } else { 26 })
  .px(7)
  .font_size(13)
  .border_radius(6)
  .border_width(1)
  .app_region("no-drag")
  .bind_style(() => {
    @ui.style()
    .bg(s.color("content"))
    .text_color(s.color("text"))
    .border_color(s.color("strong"))
  })
}

///|
fn check(
  s : Store,
  title : String,
  read : () -> Bool,
  write : (Bool) -> Unit,
) -> @ui.Element {
  @ui.element(@protocol.TAG_BUTTON)
  .children([
    @ui.div(
      [..if read() { [icon(s, "check", size=11, tone="white")] } else { [] }],
    )
    .width(14)
    .height(14)
    .border_radius(3)
    .border_width(1)
    .items_center()
    .justify_center()
    .display("flex")
    .bind_style(() => {
      @ui.style()
      .bg(s.color(if read() { "accent" } else { "content" }))
      .border_color(s.color(if read() { "accent" } else { "strong" }))
    }),
    label(s, () => title).font_size(12),
  ])
  .flex_row()
  .items_center()
  .gap(7)
  .height(24)
  .app_region("no-drag")
  .on_click(() => write(!read()))
  .property(@protocol.ACCESSIBILITY_LABEL, @protocol.Text(title))
}

///|
fn icon(
  s : Store,
  name : String,
  size? : Int = 14,
  tone? : String = "secondary",
) -> @ui.Element {
  let path = match name {
    "branch" =>
      "<path d=\"M6 6v12m12-12v2a6 6 0 0 1-6 6H6\"/><circle cx=\"6\" cy=\"3\" r=\"2\"/><circle cx=\"6\" cy=\"20\" r=\"2\"/><circle cx=\"18\" cy=\"3\" r=\"2\"/>"
    "fetch" => "<path d=\"M12 3v12m-4-4 4 4 4-4M4 16v4h16v-4\"/>"
    "pull" => "<path d=\"M12 4v16m-6-6 6 6 6-6\"/>"
    "push" => "<path d=\"M12 20V4m-6 6 6-6 6 6\"/>"
    "check" => "<path d=\"m5 12 4 4L19 6\"/>"
    "close" => "<path d=\"m6 6 12 12M6 18 18 6\"/>"
    "chevron" => "<path d=\"m6 9 6 6 6-6\"/>"
    "plus" => "<path d=\"M12 5v14M5 12h14\"/>"
    "minus" => "<path d=\"M5 12h14\"/>"
    "refresh" =>
      "<path d=\"M3 12a9 9 0 0 1 15.36-6.36L21 8M21 3v5h-5M21 12a9 9 0 0 1-15.36 6.36L3 16M8 16H3v5\"/>"
    _ => "<path d=\"M3 7h7l2-3h9v16H3z\"/>"
  }
  @ui.svg(
    "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 24 24\" fill=\"none\" stroke=\"#000\" stroke-width=\"1.8\" stroke-linecap=\"round\" stroke-linejoin=\"round\">" +
    path +
    "</svg>",
  )
  .width(size)
  .height(size)
  .flex_shrink(0)
  .bind_style(() => {
    @ui.style().text_color(
      if tone == "white" {
        hex("#ffffff")
      } else {
        s.color(tone)
      },
    )
  })
}

///|
fn empty(s : Store, title : String, subtitle : String) -> @ui.Element {
  fill()
  .children([
    label(s, () => title, tone="muted").font_size(15).text_align("center"),
    label(s, () => subtitle, tone="muted")
    .font_size(12)
    .line_height(18)
    .max_width(360)
    .text_align("center"),
  ])
  .items_center()
  .justify_center()
  .padding(32)
  .gap(8)
}
