// Compound elements retain one native node each. Only declared state lives in
// MoonBit; pointer capture, focus, layout, accessibility and animation stay native.

///|
priv struct ComponentSpec {
  role : String
  tag : Int
  slots : Map[String, String]
  properties : Array[Int]
  defaults : Map[Int, @protocol.Value]
}

///|
priv struct ComponentState {
  root : @native.Node
  kind : String
  spec : ComponentSpec
  scope : String
  properties : Map[Int, @reactive.Signal[@protocol.Value]]
  controlled : Map[Int, Unit]
}

///|
priv struct ComponentPart {
  state : ComponentState
  name : String
  value : String
  index : Int
  overrides : Map[Int, @reactive.Signal[@protocol.Value]]
  controlled : Map[Int, Unit]
  mut change_handlers : Map[String, (Json) -> Unit]?
}

///|
fn ComponentState::read(self : ComponentState, code : Int) -> @protocol.Value {
  match self.properties.get(code) {
    Some(value) => value.get()
    None => @protocol.Clear
  }
}

///|
fn ComponentState::adopt(
  self : ComponentState,
  code : Int,
  value : @protocol.Value,
) -> Unit {
  if !self.controlled.contains(code) {
    if self.properties.get(code) is Some(signal) {
      signal.set(value)
    }
  }
}

///|
fn json_value(value : @protocol.Value) -> Json {
  match value {
    @protocol.Text(value) => @json.parse(value) catch { _ => Json::null() }
    _ => Json::null()
  }
}

///|
fn has_value(value : @protocol.Value, item : String) -> Bool {
  match json_value(value) {
    Array(values) => values.contains(Json::string(item))
    _ => false
  }
}

///|
fn ComponentPart::read(self : ComponentPart, code : Int) -> @protocol.Value {
  let inherited = self.state.read(code)
  let overrides = match self.overrides.get(code) {
    Some(signal) => signal.get()
    None => @protocol.Clear
  }
  if code == @protocol.DISABLED {
    if inherited == @protocol.Boolean(true) ||
      overrides == @protocol.Boolean(true) {
      return @protocol.Boolean(true)
    }
  }
  if overrides != @protocol.Clear {
    return overrides
  }
  if self.state.kind == "accordion" &&
    self.name != "root" &&
    code == @protocol.OPEN {
    return @protocol.Boolean(
      has_value(self.state.read(@protocol.VALUES), self.value),
    )
  }
  inherited
}

///|
fn Element::set_component_property(
  self : Element,
  code : Int,
  value : @protocol.Value,
) -> Bool {
  guard self.component is Some(part) else { return false }
  // The picker API calls its source "items"; the native protocol shares the
  // OPTIONS field across select, combobox, and autocomplete.
  let code = if code == @protocol.ITEMS &&
    ["select", "combobox", "autocomplete"].contains(part.state.kind) {
    @protocol.OPTIONS
  } else {
    code
  }
  guard part.state.properties.get(code) is Some(signal) else { return false }
  if part.name == "root" {
    signal.set(value)
  } else if part.overrides.get(code) is Some(overrides) {
    overrides.set(value)
    if !part.inherits(code) {
      self.native_node().set(code, value)
    }
  }
  true
}

///|
fn component_root(kind : String) -> Element {
  let spec = component_spec(kind)
  let node = @native.element(spec.tag)
  let state = ComponentState::{
    root: node,
    kind,
    spec,
    scope: "moon-\{kind}-\{node.id}",
    properties: Map([]),
    controlled: Map([]),
  }
  for code in spec.properties {
    state.properties[code] = @reactive.signal(
      spec.defaults.get(code).unwrap_or(@protocol.Clear),
    )
  }
  let part = ComponentPart::{
    state,
    name: "root",
    value: "",
    index: 0,
    overrides: Map([]),
    controlled: state.controlled,
    change_handlers: None,
  }
  let root = Element::{
    window_id: node.window,
    node_id: node.id,
    component: Some(part),
  }
  if spec.role != "" {
    node.set(@protocol.PART, @protocol.Text(spec.role))
    node.set(@protocol.SCOPE, @protocol.Text(state.scope))
  }
  for code in spec.properties {
    // A scope-only wrapper is not itself a surface. Its options belong on the
    // declared parts, otherwise anchor/dismissal options turn it into a popup.
    if spec.role != "" || spec.tag != @protocol.TAG_VIEW {
      node.bind(code, () => state.read(code))
    }
  }
  node.on_default("componentchange", @protocol.COMPONENT_CHANGE_LISTENER, event => {
    event.stop_propagation()
    state.adopt_event(event_json(event))
  })
  root.install_component_actions()
  root
}

///|
fn event_json(event : @native.Event) -> Json {
  @json.parse(event.value.unwrap_or("null")) catch {
    _ => event.extra
  }
}

///|
/// Create a part in this component's scope. Item descendants retain its value
/// and index; separate component instances never share selection or focus IDs.
pub fn Element::slot(
  self : Element,
  slot : Slot,
  value? : String = "",
  index? : Int = 0,
) -> Element {
  guard self.component is Some(parent) else {
    abort("slot requires a component root or part")
  }
  let name = slot_name(slot)
  if parent.state.kind == "menu" && name == "submenu-root" {
    let submenu = component_root("menu")
    submenu
    .native_node()
    .set(@protocol.PART, @protocol.Text("menu-submenu-root"))
    return submenu
  }
  guard parent.state.spec.slots.get(name) is Some(role) else {
    abort("\{parent.state.kind} has no \{name} part")
  }
  let tag = match name {
    "trigger"
    | "tab"
    | "increment"
    | "decrement"
    | "close"
    | "action"
    | "button"
    | "link"
    | "link-item"
    | "checkbox-item"
    | "radio-item"
    | "submenu-trigger"
    | "day" => @protocol.TAG_BUTTON
    "input" => @protocol.TAG_INPUT
    "control" =>
      if parent.state.kind == "field" || parent.state.kind == "fieldset" {
        @protocol.TAG_INPUT
      } else {
        @protocol.TAG_VIEW
      }
    "item" =>
      if [
          "checkbox-group", "radio-group", "toggle-group", "menu", "toolbar", "menubar",
        ].contains(parent.state.kind) {
        @protocol.TAG_BUTTON
      } else {
        @protocol.TAG_VIEW
      }
    "parent" => @protocol.TAG_BUTTON
    "image" => @protocol.TAG_IMAGE
    _ => @protocol.TAG_VIEW
  }
  let node = @native.element(tag)
  let part = ComponentPart::{
    state: parent.state,
    name,
    value: if value == "" {
      parent.value
    } else {
      value
    },
    index: if parent.name == "item" {
      parent.index
    } else {
      index
    },
    overrides: Map([]),
    controlled: Map([]),
    change_handlers: None,
  }
  let element = Element::{
    window_id: node.window,
    node_id: node.id,
    component: Some(part),
  }
  node.set(@protocol.PART, @protocol.Text(role))
  node.set(@protocol.SCOPE, @protocol.Text(part.state.scope))
  node.set(@protocol.PART_VALUE, @protocol.Text(part.value))
  node.set(@protocol.ITEM_INDEX, @protocol.Number(Float::from_int(part.index)))
  // Native controllers report state at their trigger, popup or item identity.
  // Request those events here; they bubble to this component's root observer.
  node.set(@protocol.COMPONENT_CHANGE_LISTENER, @protocol.Boolean(true))
  for code in part.state.spec.properties {
    part.overrides[code] = @reactive.signal(@protocol.Clear)
    if part.inherits(code) {
      node.bind(code, () => part.read(code))
    }
  }
  if part.state.kind == "radio-group" && name == "item" {
    node.bind(@protocol.CHECKED, () => {
      @protocol.Boolean(
        part.state.read(@protocol.ACTIVE_VALUE) == @protocol.Text(part.value),
      )
    })
  }
  if part.state.kind == "tabs" && name == "indicator" && part.value == "" {
    node.bind(@protocol.PART_VALUE, () => {
      part.state.read(@protocol.ACTIVE_VALUE)
    })
  }
  if name == "segment" {
    node.set(@protocol.SEGMENT, @protocol.Text(part.value))
  }
  if name == "day" {
    node.set(@protocol.CIVIL_VALUE, @protocol.Text(part.value))
  }
  if name == "row" {
    node.set(@protocol.ROW_INDEX, @protocol.Number(Float::from_int(part.index)))
  }
  if name == "trigger" &&
    part.state.kind == "popover-menu" &&
    @reactive.untrack(() => part.state.read(@protocol.ANCHOR_TARGET)) ==
    @protocol.Clear {
    // Controlled and keyboard-driven opening may happen before a trigger click.
    // Register the anchor when it mounts, as well as updating it on activation.
    part.state.adopt(
      @protocol.ANCHOR_TARGET,
      @protocol.Text(node.id.to_string()),
    )
  }
  element.install_component_actions()
  element
}

///|
fn ComponentPart::inherits(self : ComponentPart, code : Int) -> Bool {
  // Most parts locate their controller by scope. Copy only the descriptor fields
  // the host reads on that part, rather than sending root datasets to every cell.
  if code == @protocol.ANCHOR_TARGET {
    return (self.state.kind == "popover-menu" && self.name == "popup") ||
      (
        ["menu", "popover", "tooltip", "preview-card"].contains(self.state.kind) &&
        ["trigger", "positioner"].contains(self.name)
      )
  }
  if [@protocol.DISMISS_ON_ESCAPE, @protocol.DISMISS_ON_POINTER_OUTSIDE].contains(
      code,
    ) {
    return self.name == "popup"
  }
  if ["dialog", "alert-dialog"].contains(self.state.kind) {
    return [@protocol.OPEN, @protocol.VARIANT, @protocol.DISABLED].contains(
        code,
      ) ||
      (
        self.name == "portal" &&
        [@protocol.ENTER_DURATION, @protocol.EXIT_DURATION].contains(code)
      )
  }
  match self.state.kind {
    "tabs" =>
      [
        @protocol.ACTIVE_VALUE,
        @protocol.ORIENTATION,
        @protocol.ACTIVATE_ON_FOCUS,
        @protocol.LOOP_FOCUS,
        @protocol.KEEP_MOUNTED,
        @protocol.DISABLED,
      ].contains(code)
    "accordion" =>
      [
        @protocol.OPEN,
        @protocol.DISABLED,
        @protocol.KEEP_MOUNTED,
        @protocol.HEADING_LEVEL,
      ].contains(code)
    "checkbox" | "switch" | "toggle" | "radio" => false
    "collapsible" | "field" | "fieldset" | "progress" | "meter" => true
    "radio-group" =>
      [@protocol.READ_ONLY, @protocol.REQUIRED, @protocol.DISABLED].contains(
        code,
      )
    "popover"
    | "tooltip"
    | "preview-card"
    | "drawer"
    | "navigation-menu"
    | "menu"
    | "popover-menu"
    | "context-menu" => true
    _ => code == @protocol.DISABLED || code == @protocol.READ_ONLY
  }
}

///|
fn ComponentState::change(self : ComponentState, payload : Json) -> Unit {
  self.root.emit("componentchange", payload)
}

///|
fn ComponentState::toggle_values(self : ComponentState, value : String) -> Unit {
  let values = match json_value(self.read(@protocol.VALUES)) {
    Array(values) => values.copy()
    _ => []
  }
  let item = Json::string(value)
  if values.search(item) is Some(index) {
    ignore(values.remove(index))
  } else {
    if self.read(@protocol.MULTIPLE) != @protocol.Boolean(true) &&
      self.kind != "checkbox-group" {
      values.clear()
    }
    values.push(item)
  }
  self.change({ "values": values })
}

///|
fn Element::install_component_actions(self : Element) -> Unit {
  guard self.component is Some(part) else { return }
  let state = part.state
  let kind = state.kind
  let name = part.name
  let toggles = (
      name == "root" && ["checkbox", "radio", "switch", "toggle"].contains(kind)
    ) ||
    (name == "item" && kind == "radio-group") ||
    (name == "tab" && kind == "tabs") ||
    (
      name == "trigger" &&
      [
        "collapsible", "accordion", "dialog", "alert-dialog", "popover", "popover-menu",
      ].contains(kind)
    ) ||
    (name == "close" && ["dialog", "alert-dialog", "popover"].contains(kind))
  if toggles {
    self
    .native_node()
    .on_default("click", @protocol.CLICK_LISTENER, _ => {
      if part.read(@protocol.DISABLED) == @protocol.Boolean(true) ||
        part.read(@protocol.READ_ONLY) == @protocol.Boolean(true) {
        return
      }
      match kind {
        "checkbox" | "switch" | "radio" =>
          state.change({
            "checked": kind == "radio" ||
            state.read(@protocol.CHECKED) != @protocol.Boolean(true),
            "indeterminate": false,
          })
        "toggle" =>
          state.change({
            "pressed": state.read(@protocol.PRESSED) != @protocol.Boolean(true),
          })
        "tabs" | "radio-group" => state.change({ "value": part.value })
        "accordion" => state.toggle_values(part.value)
        _ => {
          // Ordinary compound surfaces resolve their anchor by scope in the
          // native core. Only the model-driven popup needs a raw node anchor.
          if kind == "popover-menu" {
            state.adopt(
              @protocol.ANCHOR_TARGET,
              @protocol.Text(self.native_node().id.to_string()),
            )
          }
          state.change({
            "open": name != "close" &&
            state.read(@protocol.OPEN) != @protocol.Boolean(true),
          })
        }
      }
    })
  }
  if name == "popup" &&
    ["dialog", "alert-dialog", "popover", "popover-menu"].contains(kind) {
    self
    .native_node()
    .on_default("dismiss", @protocol.DISMISS_LISTENER, _ => {
      state.change({ "open": false })
    })
  }
}

///|
fn ComponentState::adopt_event(self : ComponentState, data : Json) -> Unit {
  guard data is Object(fields) else { return }
  @reactive.batch(() => {
    for
      key, code in Map([
        ("checked", @protocol.CHECKED),
        ("indeterminate", @protocol.INDETERMINATE),
        ("pressed", @protocol.PRESSED),
        ("open", @protocol.OPEN),
      ]) {
      if fields.get(key) is Some(True) {
        self.adopt(code, @protocol.Boolean(true))
      }
      if fields.get(key) is Some(False) {
        self.adopt(code, @protocol.Boolean(false))
      }
    }
    for
      key in [
        "values", "checkedValues", "selectedValues", "chipValues", "sizes", "pressed",
      ] {
      if fields.get(key) is Some(Array(values)) {
        self.adopt(
          @protocol.VALUES,
          @protocol.Text(Json::array(values).stringify()),
        )
      }
    }
    if fields.get("numberValue") is Some(value) {
      self.adopt(
        @protocol.VALUES,
        @protocol.Text(
          if value == Json::null() {
            "null"
          } else {
            Json::array([value]).stringify()
          },
        ),
      )
    }
    if fields.get("value") is Some(value) {
      let code = if ["date-field", "time-field", "calendar"].contains(self.kind) {
        @protocol.CIVIL_VALUE
      } else if self.kind == "otp-field" {
        @protocol.VALUE
      } else {
        @protocol.ACTIVE_VALUE
      }
      match value {
        String(value) => self.adopt(code, @protocol.Text(value))
        Null => self.adopt(code, @protocol.Clear)
        _ => ()
      }
    }
    if fields.get("inputValue") is Some(String(value)) {
      self.adopt(@protocol.INPUT_VALUE, @protocol.Text(value))
    }
    for
      key, code in Map([
        ("expanded", @protocol.EXPANDED),
        ("selection", @protocol.SELECTION),
      ]) {
      if fields.get(key) is Some(value) {
        self.adopt(code, @protocol.Text(value.stringify()))
      }
    }
  })
}

///|
pub fn Element::on_checked_change(
  self : Element,
  handler : (Bool) -> Unit,
) -> Element {
  self.configure_change_callbacks(on_checked_change=handler)
}

///|
pub fn Element::on_open_change(
  self : Element,
  handler : (Bool) -> Unit,
) -> Element {
  self.configure_change_callbacks(on_open_change=handler)
}

///|
pub fn Element::on_value_change(
  self : Element,
  handler : (String?) -> Unit,
) -> Element {
  self.configure_change_callbacks(on_value_change=handler)
}

///|
fn Element::configure_change_callbacks(
  self : Element,
  on_change? : (Json) -> Unit,
  on_checked_change? : (Bool) -> Unit,
  on_open_change? : (Bool) -> Unit,
  on_value_change? : (String?) -> Unit,
) -> Element {
  guard self.component is Some(part) else {
    if on_change is Some(handler) {
      return self.on("componentchange", @protocol.COMPONENT_CHANGE_LISTENER, e => {
        handler(event_json(e))
      })
    }
    abort("state change handlers require a component root or part")
  }
  let handlers = match part.change_handlers {
    Some(handlers) => handlers
    None => {
      let handlers : Map[String, (Json) -> Unit] = Map([])
      part.change_handlers = Some(handlers)
      self
      .native_node()
      .effect(() => @reactive.on_cleanup(() => handlers.clear()))
      ignore(
        self.on("componentchange", @protocol.COMPONENT_CHANGE_LISTENER, e => {
          let data = event_json(e)
          if part.name == "root" {
            part.state.adopt_event(data)
          }
          for key in ["change", "checked", "open", "value"] {
            if handlers.get(key) is Some(handler) {
              handler(data)
            }
          }
        }),
      )
      handlers
    }
  }
  if on_change is Some(handler) {
    handlers["change"] = handler
  }
  if on_checked_change is Some(handler) {
    handlers["checked"] = data => {
      if data is { "checked": value, .. } {
        handler(value == Json::boolean(true))
      }
    }
  }
  if on_open_change is Some(handler) {
    handlers["open"] = data => {
      if data is { "open": value, .. } {
        handler(value == Json::boolean(true))
      }
    }
  }
  if on_value_change is Some(handler) {
    handlers["value"] = data => {
      match data {
        { "value": String(value), .. } => handler(Some(value))
        { "value": Null, .. } => handler(None)
        _ => ()
      }
    }
  }
  self
}

///|
pub fn text_area() -> Element {
  input().multiline(true)
}

///|
pub fn shader(source : String) -> Element {
  configure_element(element(@protocol.TAG_SHADER), ([] : Array[Element])).value(
    source,
  )
}

///|
pub fn virtual_list(children : Array[Element]) -> Element {
  configure_element(element(@protocol.TAG_VIRTUAL_LIST), children)
}
