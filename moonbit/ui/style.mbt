// Reusable style builder. Applying it snapshots its values into the node.

///|
pub struct Style {
  properties : Map[Int, @protocol.Value]
  declaration : Map[String, Json]
}

///|
pub fn style() -> Style {
  { properties: Map([]), declaration: Map([]), }
}

///|
pub fn Style::merge(self : Style, other : Style) -> Style {
  for code, value in other.properties {
    self.properties[code] = value
  }
  for key, value in other.declaration {
    self.declaration[key] = value
  }
  self
}

///|
pub fn Style::when(
  self : Style,
  condition : Bool,
  apply : (Style) -> Style,
) -> Style {
  if condition {
    apply(self)
  } else {
    self
  }
}

///|
pub fn Style::property(
  self : Style,
  code : Int,
  key : String,
  value : @protocol.Value,
) -> Style {
  self.properties[code] = value
  self.declaration[key] = match value {
    Clear => Json::null()
    Boolean(value) => value.to_json()
    Number(value) => value.to_json()
    Color(value) => value.to_json()
    Text(value) => value.to_json()
  }
  self
}

///|
pub fn Element::style(self : Element, style : Style) -> Element {
  for code, value in style.properties {
    self.native_node().set(code, value)
  }
  self
}

///|
/// The core's state-style declaration, for dynamically bound interaction styles.
pub fn Style::json(self : Style) -> Json {
  self.declaration.to_json()
}

///|
pub fn Element::bind_style(self : Element, read : () -> Style) -> Element {
  let previous : @ref.Ref[Map[Int, @protocol.Value]] = @ref.Ref(Map([]))
  self
  .native_node()
  .effect(() => {
    let style = read()
    for code in previous.val.keys() {
      if !style.properties.contains(code) {
        self.native_node().set(code, @protocol.Clear)
      }
    }
    for code, value in style.properties {
      self.native_node().set(code, value)
    }
    previous.val = style.properties.copy()
  })
  self
}

///|
pub fn Element::hover(self : Element, build : (Style) -> Style) -> Element {
  self.property(
    @protocol.HOVER_STYLE,
    @protocol.Text(build(style()).declaration.to_json().stringify()),
  )
}

///|
pub fn Element::active(self : Element, build : (Style) -> Style) -> Element {
  self.property(
    @protocol.ACTIVE_STYLE,
    @protocol.Text(build(style()).declaration.to_json().stringify()),
  )
}

///|
pub fn Element::focus_style(
  self : Element,
  build : (Style) -> Style,
) -> Element {
  self.property(
    @protocol.FOCUS_STYLE,
    @protocol.Text(build(style()).declaration.to_json().stringify()),
  )
}

// RGBA bytes are written little-endian: the packed integer is 0xAABBGGRR.

///|
#valtype
pub struct Color {
  value : UInt
}

///|
/// Use the same packed native color in window options and style declarations.
pub impl ToJson for Color with fn to_json(self) {
  self.value.to_json()
}

///|
pub fn rgb8(red : Int, green : Int, blue : Int) -> Color {
  rgba8(red, green, blue, 255)
}

///|
pub fn rgba8(red : Int, green : Int, blue : Int, alpha : Int) -> Color {
  if red < 0 ||
    red > 255 ||
    green < 0 ||
    green > 255 ||
    blue < 0 ||
    blue > 255 ||
    alpha < 0 ||
    alpha > 255 {
    abort("Color channels must be between 0 and 255")
  }
  {
    value: red.reinterpret_as_uint() |
    (green.reinterpret_as_uint() << 8) |
    (blue.reinterpret_as_uint() << 16) |
    (alpha.reinterpret_as_uint() << 24),
  }
}

// Numeric lengths are pixels; strings support percentages and native keywords.

///|
pub(open) trait IntoValue {
  fn into_value(Self) -> @protocol.Value
}

///|
pub impl IntoValue for Int with fn into_value(self) {
  @protocol.Number(Float::from_int(self))
}

///|
pub impl IntoValue for Float with fn into_value(self) {
  @protocol.Number(self)
}

///|
pub impl IntoValue for Double with fn into_value(self) {
  @protocol.Number(Float::from_double(self))
}

///|
pub impl IntoValue for String with fn into_value(self) {
  @protocol.Text(self)
}

///|
pub impl IntoValue for Bool with fn into_value(self) {
  @protocol.Boolean(self)
}

///|
pub impl IntoValue for Color with fn into_value(self) {
  @protocol.Color(self.value)
}

///|
pub impl IntoValue for Json with fn into_value(self) {
  @protocol.Text(self.stringify())
}
