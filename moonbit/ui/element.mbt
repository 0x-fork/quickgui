// Value handles make fluent chains cheap; there is no virtual tree per frame.

///|
#valtype
pub struct Element {
  priv window_id : Int
  priv node_id : Int
  priv component : ComponentPart?
}

///|
pub fn Element::native_node(self : Element) -> @native.Node {
  @native.Node::{ window: self.window_id, id: self.node_id, }
}

///|
pub fn window(
  component : () -> Element,
  options? : Json = Json::empty_object(),
) -> @native.Window {
  @native.new_window(() => component().native_node(), options~)
}

///|
pub fn element(tag : Int) -> Element {
  let node = @native.element(tag)
  { window_id: node.window, node_id: node.id, component: None, }
}

///|
pub fn div(children : Array[Element]) -> Element {
  configure_element(element(@protocol.TAG_VIEW), children)
}

///|
pub fn text(value : String) -> Element {
  configure_element(element(@protocol.TAG_VIEW), value)
}

///|
pub fn[C : IsChildren] button(children : C) -> Element {
  configure_element(element(@protocol.TAG_BUTTON), children)
}

///|
pub fn input() -> Element {
  element(@protocol.TAG_INPUT)
}

///|
/// An SVG alpha mask tinted by text_color, as in the native renderer.
pub fn svg(source : String) -> Element {
  configure_element(element(@protocol.TAG_SVG), ([] : Array[Element])).value(
    source,
  )
}

///|
pub fn image(source : String) -> Element {
  configure_element(element(@protocol.TAG_IMAGE), ([] : Array[Element])).value(
    source,
  )
}

///|
pub fn markdown(source : String) -> Element {
  configure_element(element(@protocol.TAG_MARKDOWN), ([] : Array[Element])).value(
    source,
  )
}

///|
pub fn Element::property(
  self : Element,
  code : Int,
  value : @protocol.Value,
) -> Element {
  if !self.set_component_property(code, value) {
    self.native_node().set(code, value)
  }
  self
}

///|
pub fn Element::bind(
  self : Element,
  code : Int,
  read : () -> @protocol.Value,
) -> Element {
  if self.component is Some(part) && part.state.properties.contains(code) {
    part.controlled[code] = ()
    self.native_node().effect(() => ignore(self.property(code, read())))
  } else {
    self.native_node().bind(code, read)
  }
  self
}

///|
pub fn Element::child(self : Element, child : Element) -> Element {
  self.native_node().append(child.native_node())
  self
}

///|
pub fn Element::children(self : Element, children : Array[Element]) -> Element {
  for child in children {
    self.native_node().append(child.native_node())
  }
  self
}

///|
pub fn Element::child_if(
  self : Element,
  condition : () -> Bool,
  render : () -> Element,
  otherwise? : (() -> Element)? = None,
) -> Element {
  let anchor = @native.sentinel()
  self.native_node().append(anchor)
  // The outer effect owns the predicate memo and the current branch.
  anchor.effect(() => {
    let visible = @reactive.memo(condition)
    @reactive.create_effect(() => {
      let value = visible.get()
      @reactive.untrack(() => {
        if value {
          self.native_node().append(render().native_node(), before=anchor.id)
        } else if otherwise is Some(fallback) {
          self.native_node().append(fallback().native_node(), before=anchor.id)
        }
      })
    })
  })
  self
}

///|
pub fn Element::on(
  self : Element,
  kind : String,
  listener_property : Int,
  handler : (@native.Event) -> Unit,
) -> Element {
  self.native_node().on(kind, listener_property, handler)
  self
}

///|
pub fn Element::on_click(self : Element, handler : () -> Unit) -> Element {
  self.on("click", @protocol.CLICK_LISTENER, _ => handler())
}

///|
pub fn Element::on_input(self : Element, handler : (String) -> Unit) -> Element {
  self.on("input", @protocol.INPUT_LISTENER, event => {
    handler(event.value.unwrap_or(""))
  })
}

///|
pub fn Element::on_submit(
  self : Element,
  handler : (String) -> Unit,
) -> Element {
  self.on("submit", @protocol.SUBMIT_LISTENER, event => {
    handler(event.value.unwrap_or(""))
  })
}

///|
pub fn Element::bind_value(self : Element, read : () -> String) -> Element {
  self.bind(@protocol.VALUE, () => @protocol.Text(read()))
}

///|
pub fn Element::focus(self : Element) -> Unit {
  self.native_node().focus()
}

///|
pub fn Element::remove(self : Element) -> Unit {
  self.native_node().remove()
}

///|
pub fn Element::size_full(self : Element) -> Element {
  self.width("100%").height("100%")
}

///|
pub fn Element::flex_col(self : Element) -> Element {
  self.display("flex").flex_direction("column")
}

///|
/// A numeric flex shorthand: grow, shrink 1, zero basis.
pub fn Element::flex(self : Element, value : Float) -> Element {
  self.flex_grow(value).flex_shrink(1).flex_basis(0)
}

///|
pub fn Element::flex_row(self : Element) -> Element {
  self.display("flex").flex_direction("row")
}

///|
pub fn Element::items_center(self : Element) -> Element {
  self.align_items("center")
}

///|
pub fn Element::justify_center(self : Element) -> Element {
  self.justify_content("center")
}

///|
pub fn Element::px(self : Element, value : Float) -> Element {
  self.padding_left(value).padding_right(value)
}

///|
pub fn Element::py(self : Element, value : Float) -> Element {
  self.padding_top(value).padding_bottom(value)
}
