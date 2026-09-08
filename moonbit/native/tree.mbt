// Handles contain IDs, not back-references to their owning window or callbacks.

///|
#valtype
pub(all) struct Node {
  window : Int
  id : Int
} derive(Eq)

///|
priv struct NodeState {
  scope : Int
  children : Array[Int]
  properties : Map[Int, @protocol.Value]
  listeners : Map[String, (Event) -> Unit]
  defaults : Map[String, (Event) -> Unit]
  mut parent : Int
  mut text : String
}

///|
let next_node : @ref.Ref[Int] = @ref.Ref(0)

///|
fn state(node : Node) -> NodeState? {
  match windows.get(node.window) {
    Some(window) => window.nodes.get(node.id)
    None => None
  }
}

///|
fn changed(window : Int) -> @protocol.Batch {
  let state = windows.get(window).unwrap()
  dirty_windows[window] = ()
  state.batch
}

///|
fn node_state(scope : Int) -> NodeState {
  {
    scope,
    children: [],
    properties: Map([]),
    listeners: Map([]),
    defaults: Map([]),
    parent: -1,
    text: "",
  }
}

///|
fn allocate_node() -> Node {
  let window = current_window().id
  if windows.get(window).unwrap().nodes.length() > @protocol.MAX_NODES {
    abort("QuickGUI window exceeded the core node limit")
  }
  if next_node.val == 0x7fffffff {
    abort("QuickGUI node ID space exhausted")
  }
  next_node.val += 1
  let node = Node::{ window, id: next_node.val, }
  let scope = @reactive.create_scope()
  windows.get(window).unwrap().nodes[node.id] = node_state(scope)
  @reactive.with_owner(scope, () => @reactive.on_cleanup(() => node.remove()))
  node
}

///|
pub fn element(tag : Int) -> Node {
  let node = allocate_node()
  changed(node.window).create_element(node.id, tag)
  node
}

///|
pub fn text(value : String) -> Node {
  let node = allocate_node()
  state(node).unwrap().text = value
  changed(node.window).create_text(node.id, value)
  node
}

///|
pub fn sentinel() -> Node {
  let node = allocate_node()
  changed(node.window).create_sentinel(node.id)
  node
}

///|
pub fn Node::set(self : Node, property : Int, value : @protocol.Value) -> Unit {
  guard state(self) is Some(node) else { return }
  let previous = node.properties.get(property).unwrap_or(@protocol.Clear)
  if previous == value {
    return
  }
  if value == @protocol.Clear {
    node.properties.remove(property)
  } else {
    node.properties[property] = value
  }
  changed(self.window).set_property(self.id, property, value)
}

///|
pub fn Node::set_text(self : Node, value : String) -> Unit {
  guard state(self) is Some(node) else { return }
  if node.text == value {
    return
  }
  node.text = value
  changed(self.window).replace_text(self.id, value)
}

// Binding effects own any nested work and always restore the window context.

///|
pub fn Node::effect(self : Node, run : () -> Unit) -> Unit {
  guard state(self) is Some(node) else { return }
  @reactive.with_owner(node.scope, () => {
    @reactive.create_effect(() => rendering_window.protect(self.window, run))
  })
}

///|
/// Run once after the native owner window reports readiness. Child surfaces
/// cannot be submitted while their parent's creation is still pending.
pub fn Node::after_mount(self : Node, run : () -> Unit) -> Unit {
  guard state(self) is Some(_) else { return }
  let task = () => {
    if state(self) is Some(node) {
      rendering_window.protect(self.window, () => {
        @reactive.with_owner(node.scope, run)
      })
    }
  }
  let window = windows.get(self.window).unwrap()
  if window.mounted {
    task()
  } else {
    window.mounted_tasks.push(task)
  }
}

///|
pub fn Node::bind(
  self : Node,
  property : Int,
  read : () -> @protocol.Value,
) -> Unit {
  self.effect(() => self.set(property, read()))
}

///|
pub fn Node::append(self : Node, child : Node, before? : Int = -1) -> Unit {
  if self.window != child.window {
    abort("Cannot move nodes between windows")
  }
  guard state(self) is Some(parent) else { return }
  guard state(child) is Some(node) else { return }
  if before == child.id {
    return
  }
  let mut ancestor = self.id
  while ancestor != -1 {
    if ancestor == child.id {
      abort("Cannot insert a node into its own subtree")
    }
    ancestor = windows.get(self.window).unwrap().nodes.get(ancestor).unwrap().parent
  }
  if before != -1 && !parent.children.contains(before) {
    abort("Insertion anchor must be a child of the parent")
  }
  if node.parent == self.id {
    let index = parent.children.search(child.id).unwrap()
    let next = if index + 1 < parent.children.length() {
      parent.children[index + 1]
    } else {
      -1
    }
    if next == before {
      return
    }
  }
  if node.parent != -1 {
    let previous = windows.get(self.window).unwrap().nodes
      .get(node.parent)
      .unwrap()
    ignore(
      previous.children.remove(previous.children.search(child.id).unwrap()),
    )
  }
  let index = if before == -1 {
    parent.children.length()
  } else {
    parent.children.search(before).unwrap()
  }
  parent.children.insert(index, child.id)
  node.parent = self.id
  changed(self.window).insert(self.id, child.id, before~)
}

///|
pub fn Node::remove(self : Node) -> Unit {
  guard state(self) is Some(node) else { return }
  let window = windows.get(self.window).unwrap()
  window.nodes.remove(self.id)
  if window.nodes.get(node.parent) is Some(parent) {
    if parent.children.search(self.id) is Some(index) {
      ignore(parent.children.remove(index))
    }
    changed(self.window).remove(node.parent, self.id)
  }
  for id in node.children.copy() {
    Node::{ window: self.window, id, }.remove()
  }
  @reactive.dispose(node.scope)
}

///|
pub fn Node::on(
  self : Node,
  kind : String,
  property : Int,
  listener : (Event) -> Unit,
) -> Unit {
  guard state(self) is Some(node) else { return }
  node.listeners[kind] = listener
  if property != 0 {
    self.set(property, @protocol.Boolean(true))
  }
}

///|
/// Component defaults compose with application listeners. A canceled event does
/// not run its default action; stopping propagation only stops ancestor delivery.
pub fn Node::on_default(
  self : Node,
  kind : String,
  property : Int,
  listener : (Event) -> Unit,
) -> Unit {
  guard state(self) is Some(node) else { return }
  node.defaults[kind] = listener
  if property != 0 {
    self.set(property, @protocol.Boolean(true))
  }
}

///|
pub fn Node::focus(self : Node) -> Unit {
  if state(self) is Some(_) {
    check(ffi_focus_node(app_id.val, self.window, self.id))
  }
}

///|
/// Deliver a semantic event to this retained subtree. This does not synthesize
/// OS input. Compound controls use it to report their default actions.
pub fn Node::emit(self : Node, kind : String, payload : Json) -> Unit {
  rendering_window.protect(self.window, () => {
    @reactive.batch(() => {
      dispatch_node({
        kind,
        window: self.window,
        target: self.id,
        current_target: self.id,
        value: Some(payload.stringify()),
        extra: payload,
        data: b"",
        stopped: false,
        default_prevented: false,
      })
    })
  })
}

///|
fn dispatch_node(event : Event) -> Unit {
  let mut id = event.target
  while id != -1 {
    guard state(Node::{ window: event.window, id, }) is Some(node) else {
      break
    }
    let parent = node.parent
    if node.listeners.get(event.kind) is Some(listener) {
      event.current_target = id
      @reactive.with_owner(node.scope, () => listener(event))
    }
    if !event.default_prevented &&
      state(Node::{ window: event.window, id, }) is Some(_) {
      if node.defaults.get(event.kind) is Some(listener) {
        event.current_target = id
        @reactive.with_owner(node.scope, () => listener(event))
      }
    }
    if event.stopped || event.kind == "mouseenter" || event.kind == "mouseleave" {
      break
    }
    id = parent
  }
}
