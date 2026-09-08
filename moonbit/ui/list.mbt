///|
priv struct Row[T] {
  scope : Int
  element : Element
  value : @reactive.Signal[T]
}

// Keys retain native nodes, focus, input state, and row-local signal ownership.
// render runs once per key; bind row.value.get() in the returned component.

///|
pub fn[T : Eq] Element::children_keyed(
  self : Element,
  read : () -> Array[T],
  key : (T) -> String,
  render : (@reactive.Signal[T]) -> Element,
) -> Element {
  let anchor = @native.sentinel()
  self.native_node().append(anchor)
  anchor.effect(() => {
    let owner = @reactive.owner()
    let rows : Map[String, Row[T]] = Map([])
    @reactive.create_effect(() => {
      let items = read()
      @reactive.untrack(() => {
        let next : Map[String, Unit] = Map([])
        let ordered : Array[Row[T]] = []
        // Validate before modifying either the retained tree or row scopes.
        for item in items {
          let id = key(item)
          if next.contains(id) {
            abort("Duplicate QuickGUI list key: \{id}")
          }
          next[id] = ()
        }
        for id, row in rows.copy() {
          if !next.contains(id) {
            rows.remove(id)
            @reactive.dispose(row.scope)
          }
        }
        for item in items {
          let id = key(item)
          let row = match rows.get(id) {
            Some(row) => {
              row.value.set(item)
              row
            }
            None => {
              let scope = @reactive.create_scope(parent=owner)
              let value = @reactive.signal(item)
              let element = @reactive.with_owner(scope, () => render(value))
              let row = { scope, value, element, }
              rows[id] = row
              row
            }
          }
          ordered.push(row)
        }
        let mut before = anchor.id
        for i = ordered.length() - 1; i >= 0; i = i - 1 {
          let child = ordered[i].element.native_node()
          self.native_node().append(child, before~)
          before = child.id
        }
      })
    })
  })
  self
}
