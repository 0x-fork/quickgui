// Headless retained-tree harness. It does not load a shared library or open a window.
// Inspection is explicit so normal application turns never serialize the tree.

///|
pub fn[T] test_tree(run : (Node) -> T raise?) -> T raise? {
  let scope = @reactive.create_scope(parent=0)
  let mut id = -1
  while windows.contains(id) {
    id -= 1
  }
  let window = {
    scope,
    nodes: Map([]),
    batch: @protocol.batch(),
    mounted_tasks: [],
    created: false,
    mounted: false,
    closing: false,
  }
  window.nodes[0] = node_state(scope)
  windows[id] = window
  window_scopes[scope] = id
  defer close_state(id)
  rendering_window.protect(id, () => {
    @reactive.with_owner(scope, () => run(Node::{ window: id, id: 0, }))
  })
}

///|
pub fn Node::pending_mutations(self : Node) -> Int {
  windows.get(self.window).unwrap().batch.mutation_count()
}

///|
pub fn Node::take_batch(self : Node) -> Bytes {
  dirty_windows.remove(self.window)
  windows.get(self.window).unwrap().batch.finish()
}

///|
pub fn Node::children(self : Node) -> Array[Node] {
  match state(self) {
    Some(node) => node.children.map(id => Node::{ window: self.window, id, })
    None => []
  }
}

///|
pub fn Node::text_content(self : Node) -> String {
  match state(self) {
    Some(node) => node.text
    None => ""
  }
}

///|
pub fn Node::property(self : Node, code : Int) -> @protocol.Value {
  match state(self) {
    Some(node) => node.properties.get(code).unwrap_or(@protocol.Clear)
    None => @protocol.Clear
  }
}
