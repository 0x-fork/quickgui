///|
/// A compound part delegates interaction, layout and accessibility to the core.
pub fn Element::part(
  self : Element,
  name : String,
  scope? : String = "",
  value? : String = "",
) -> Element {
  ignore(self.property(@protocol.PART, @protocol.Text(name)))
  if scope != "" {
    ignore(self.property(@protocol.SCOPE, @protocol.Text(scope)))
  }
  if value != "" {
    ignore(self.property(@protocol.PART_VALUE, @protocol.Text(value)))
  }
  self
}

///|
pub fn Element::on_change(self : Element, handler : (Json) -> Unit) -> Element {
  self.configure_change_callbacks(on_change=handler)
}

///|
pub fn Element::on_context_menu(
  self : Element,
  handler : () -> Unit,
) -> Element {
  self.on("contextmenu", @protocol.CONTEXT_MENU_LISTENER, e => {
    e.stop_propagation()
    handler()
  })
}

///|
pub fn Element::on_double_click(
  self : Element,
  handler : () -> Unit,
) -> Element {
  self.on("dblclick", @protocol.DOUBLE_CLICK_LISTENER, _ => handler())
}

///|
pub fn Element::bind_json(
  self : Element,
  property : Int,
  read : () -> Json,
) -> Element {
  self.bind(property, () => @protocol.Text(read().stringify()))
}

///|
pub fn Element::bind_disabled(self : Element, read : () -> Bool) -> Element {
  self.bind(@protocol.DISABLED, () => @protocol.Boolean(read()))
}

///|
pub fn Element::selected_style(
  self : Element,
  build : (Style) -> Style,
) -> Element {
  self.property(
    @protocol.SELECTED_STYLE,
    @protocol.Text(build(style()).declaration.to_json().stringify()),
  )
}

///|
pub(all) struct VisibleRange {
  start : Int
  end : Int
} derive(Eq, Debug)

///|
/// End is exclusive, as reported by the native table. The first frame has a
/// small bounded seed; native visible-range events take over after layout.
pub fn visible_range(value : Json, count : Int) -> VisibleRange {
  match value {
    { "start": Number(start, ..), "end": Number(end, ..), .. } =>
      {
        start: start.to_int().clamp(min=0, max=count),
        end: end.to_int().clamp(min=0, max=count),
      }
    _ => { start: 0, end: count.min(40), }
  }
}

///|
pub fn table(children : Array[Element]) -> Element {
  data_table(children)
  .header_height(0)
  .selection_mode("single")
  .overflow_y("scroll")
  .min_height(0)
  .min_width(0)
}

///|
pub fn table_row(children : Array[Element]) -> Element {
  configure_element(element(@protocol.TAG_VIEW), children).part("table-row")
}

///|
pub fn table_cell(children : Array[Element]) -> Element {
  configure_element(element(@protocol.TAG_VIEW), children).part("table-cell")
}

///|
pub fn Element::column(self : Element, name : String) -> Element {
  self.property(@protocol.PART_VALUE, @protocol.Text(name))
}

///|
/// The native splitter changes pane geometry during a drag. The callback is an
/// asynchronous persistence echo, never part of pointer tracking.
pub fn Element::resizable_panel(
  self : Element,
  width : () -> Double,
  changed : (Double) -> Unit,
  minimum? : Double = 180,
  maximum? : Double = 900,
  label? : String = "Resize panel",
) -> Element {
  let root = view([])
  let scope = "moon-splitter-\{root.native_node().id}"
  root
  .part("splitter", scope~)
  .flex_row()
  .flex_shrink(0)
  .min_height(0)
  .min_width(0)
  .bind_json(@protocol.VALUES, () => [width(), maximum - width()].to_json())
  .property(
    @protocol.ITEMS,
    @protocol.Text([{ "min": minimum }, {}].to_json().stringify()),
  )
  .on_change(data => {
    if data is { "sizes": [Number(size, ..), ..], .. } {
      changed(size.clamp(min=minimum, max=maximum))
    }
  })
  .child(
    self
    .part("splitter-pane", scope~)
    .property(@protocol.ITEM_INDEX, @protocol.Number(0)),
  )
  .child(
    view([])
    .part("splitter-handle", scope~)
    .property(@protocol.ITEM_INDEX, @protocol.Number(0))
    .property(@protocol.ACCESSIBILITY_LABEL, @protocol.Text(label))
    .property(@protocol.HIT_SLOP_LEFT, @protocol.Number(4))
    .property(@protocol.HIT_SLOP_RIGHT, @protocol.Number(4))
    .width(1)
    .flex_shrink(0)
    .cursor("col-resize")
    .app_region("no-drag")
    .bg(rgba8(128, 128, 140, 45)),
  )
}
