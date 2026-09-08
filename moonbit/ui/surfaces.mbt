///|
/// A system popover builds its content in a child window. The anchor owns that
/// window's lifetime; its content is never moved across native window IDs.
pub fn Element::system_content(
  self : Element,
  render : () -> Element,
  options? : Json = Json::empty_object(),
) -> Element {
  guard self.component is Some(part) && part.state.kind == "system-popover" else {
    abort("system_content requires system_popover()")
  }
  let state = part.state
  self
  .native_node()
  .on_default("click", @protocol.CLICK_LISTENER, _ => {
    if part.read(@protocol.DISABLED) != @protocol.Boolean(true) {
      state.change({
        "open": state.read(@protocol.OPEN) != @protocol.Boolean(true),
      })
    }
  })
  self
  .native_node()
  .effect(() => {
    if state.read(@protocol.OPEN) != @protocol.Boolean(true) {
      return
    }
    @reactive.untrack(() => {
      let child = @native.new_system_popover(
        self.native_node(),
        () => render().native_node(),
        options~,
      )
      let closing = @ref.Ref(false)
      @native.on_event(
        "close",
        _ => if !closing.val { state.change({ "open": false }) },
        window=child.id,
      )
      @reactive.on_cleanup(() => {
        closing.val = true
        if child.is_open() {
          child.close()
        }
      })
    })
  })
  self
}

///|
/// Model-driven popup windows exist only while open. Ordinary dialog/popover
/// parts instead leave exit timing and mounting to the native core.
pub fn Element::when_open(self : Element, render : () -> Element) -> Element {
  guard self.component is Some(part) else {
    abort("when_open requires a component")
  }
  self.child_if(
    () => part.state.read(@protocol.OPEN) == @protocol.Boolean(true),
    render,
  )
}

///|
pub fn swift_ui_quickgui_host_view(render : () -> Element) -> Element {
  let parent = @native.current_window()
  let host = element(@protocol.TAG_SWIFT_UI_QUICK_GUI_HOST)
    .width(320)
    .height(200)
  host
  .native_node()
  .after_mount(() => {
    let width = match host.native_node().property(@protocol.WIDTH) {
      Number(value) => value
      _ => 320
    }
    let height = match host.native_node().property(@protocol.HEIGHT) {
      Number(value) => value
      _ => 200
    }
    let child = @native.new_embedded_view(
      parent,
      () => render().native_node(),
      match_horizontal=host
        .native_node()
        .property(@protocol.SWIFT_UI_MATCH_CONTENTS_HORIZONTAL) ==
        @protocol.Boolean(true),
      match_vertical=host
        .native_node()
        .property(@protocol.SWIFT_UI_MATCH_CONTENTS_VERTICAL) ==
        @protocol.Boolean(true),
      options={
        "title": "QuickGUI embedded view",
        "width": width,
        "height": height,
        "show": false,
        "decorated": false,
        "shadow": false,
        "transparent": true,
      },
    )
    host
    .native_node()
    .set(
      @protocol.SWIFT_UI_EMBEDDED_WINDOW,
      @protocol.Number(Float::from_int(child.id)),
    )
    @native.on_event(
      "close",
      _ => {
        host
        .native_node()
        .set(@protocol.SWIFT_UI_EMBEDDED_WINDOW, @protocol.Clear)
      },
      window=child.id,
    )
    @reactive.on_cleanup(() => if child.is_open() { child.close() })
  })
  host
}

///|
pub fn swift_ui_secure_field() -> Element {
  swift_ui_text_field().password(true)
}

///|
pub fn swift_ui_segmented_control() -> Element {
  swift_ui_picker().swift_ui_picker_style("segmented")
}

///|
pub fn Element::label(self : Element, value : String) -> Element {
  self.native_node().set(@protocol.ACCESSIBILITY_LABEL, @protocol.Text(value))
  self.native_node().append(@native.text(value))
  self
}

///|
pub fn Element::bind_label(self : Element, read : () -> String) -> Element {
  let label = @native.text("")
  self.native_node().append(label)
  label.effect(() => {
    let value = read()
    label.set_text(value)
    self.native_node().set(@protocol.ACCESSIBILITY_LABEL, @protocol.Text(value))
  })
  self
}

///|
pub fn Element::bind_number(self : Element, read : () -> Float) -> Element {
  self.bind(@protocol.VALUE, () => @protocol.Number(read()))
}

///|
pub fn Element::on_number_input(
  self : Element,
  handler : (Double) -> Unit,
) -> Element {
  self.on_input(value => {
    let parsed = @json.parse(value) catch { _ => Json::null() }
    if parsed is Number(value, ..) {
      handler(value)
    }
  })
}

///|
pub fn Element::on_bool_input(
  self : Element,
  handler : (Bool) -> Unit,
) -> Element {
  self.on_input(value => {
    if value == "true" || value == "false" {
      handler(value == "true")
    }
  })
}
