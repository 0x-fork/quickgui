///|
/// Content accepted by view constructors: text, one element, or an element list.
/// Children attach directly to the retained parent without a wrapper node.
pub(open) trait IsChildren {
  fn append_to(Self, Element) -> Unit
  fn bind_to(() -> Self, Element) -> Unit
}

///|
pub impl IsChildren for String with fn append_to(self, parent) {
  parent.native_node().append(@native.text(self))
}

///|
pub impl IsChildren for String with fn bind_to(read, parent) {
  let label = @native.text("")
  parent.native_node().append(label)
  label.effect(() => label.set_text(read()))
}

///|
pub impl IsChildren for Element with fn append_to(self, parent) {
  parent.native_node().append(self.native_node())
}

///|
pub impl IsChildren for Element with fn bind_to(read, parent) {
  ignore(parent.children_dynamic(() => [read()]))
}

///|
pub impl IsChildren for Array[Element] with fn append_to(self, parent) {
  for child in self {
    parent.native_node().append(child.native_node())
  }
}

///|
pub impl IsChildren for Array[Element] with fn bind_to(read, parent) {
  ignore(parent.children_dynamic(read))
}

///|
fn[C : IsChildren] configure_element(
  element : Element,
  children : C,
) -> Element {
  IsChildren::append_to(children, element)
  element
}
