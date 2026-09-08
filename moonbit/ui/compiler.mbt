// Targets for the MoonBit view compiler. These also work as explicit bindings.

///|
pub fn Element::children_dynamic(
  self : Element,
  read : () -> Array[Element],
) -> Element {
  let anchor = @native.sentinel()
  self.native_node().append(anchor)
  let previous : @ref.Ref[Array[Element]] = @ref.Ref([])
  anchor.effect(() => {
    let next = read()
    let next_ids : Map[Int, Unit] = Map([])
    for node in next {
      next_ids[node.node_id] = ()
    }
    for old in previous.val {
      if !next_ids.contains(old.node_id) {
        old.remove()
      }
    }
    let mut before = anchor.id
    for index = next.length() - 1; index >= 0; index = index - 1 {
      self.native_node().append(next[index].native_node(), before~)
      before = next[index].node_id
    }
    previous.val = next
  })
  self
}

///|
/// Track only the selected branch. Reads inside its bindings belong to those
/// bindings, and changing a predicate without changing branches keeps its nodes.
pub fn[T : Eq] Element::children_switch(
  self : Element,
  select : () -> T,
  render : (T) -> Array[Element],
) -> Element {
  let anchor = @native.sentinel()
  self.native_node().append(anchor)
  anchor.effect(() => {
    let selected = @reactive.create_memo(select)
    @reactive.create_effect(() => {
      let value = selected()
      @reactive.untrack(() => {
        for child in render(value) {
          self.native_node().append(child.native_node(), before=anchor.id)
        }
      })
    })
  })
  self
}

///|
pub fn[C : IsChildren] Element::bind_content(
  self : Element,
  read : () -> C,
) -> Element {
  IsChildren::bind_to(read, self)
  self
}

///|
/// Unlike general effects, property bindings make compound state controlled.
/// The compiler uses the existing typed bind_* methods whenever possible.
pub fn[T : IntoValue] Element::bind_optional(
  self : Element,
  property : Int,
  read : () -> T?,
) -> Element {
  self.bind(property, () => {
    match read() {
      Some(value) => IntoValue::into_value(value)
      None => @protocol.Clear
    }
  })
}

///|
pub fn Element::bind_optional_style(
  self : Element,
  read : () -> Style?,
) -> Element {
  self.bind_style(() => read().unwrap_or_else(style))
}

///|
pub fn[T : IntoValue] Element::bind_compiled(
  self : Element,
  property : Int,
  read : () -> T,
) -> Element {
  self.bind(property, () => IntoValue::into_value(read()))
}

///|
pub fn Element::bind_interaction(
  self : Element,
  property : Int,
  build : (Style) -> Style,
) -> Element {
  self.bind(property, () => @protocol.Text(build(style()).json().stringify()))
}
