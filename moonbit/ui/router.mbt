///|
pub struct Route {
  priv path : String?
  priv render : (Router, () -> Element) -> Element
  priv children : Array[Route]
}

///|
pub struct Router {
  priv id : Json
  priv snapshot : @reactive.Signal[Json]
  priv renderers : Map[String, (Router, () -> Element) -> Element]
}

///|
pub fn route(path : String, render : (Router) -> Element) -> Route {
  { path: Some(path), render: (router, _) => render(router), children: [], }
}

///|
pub fn layout(
  render : (Router, () -> Element) -> Element,
  children : Array[Route],
  path? : String? = None,
) -> Route {
  { path, render, children, }
}

///|
fn router_call(operation : String, params : Json) -> Json {
  match @native.call(operation, params) {
    Ok(value) => value
    Err(message) => abort(message)
  }
}

///|
pub fn router(routes : Array[Route], initial? : String = "/") -> Router {
  let definitions : Array[Json] = []
  let renderers = Map([])
  fn visit(route : Route, parent : String) {
    let id = "route-\{definitions.length()}"
    let fields : Map[String, Json] = Map([("id", Json::string(id))])
    if parent != "" {
      fields["parentId"] = Json::string(parent)
    }
    if route.path is Some(path) {
      fields["path"] = Json::string(path)
    }
    definitions.push(Json::object(fields))
    renderers[id] = route.render
    for child in route.children {
      visit(child, id)
    }
  }
  for route in routes {
    visit(route, "")
  }
  let id = router_call("router-create", {
    "routes": definitions,
    "initialDestination": initial,
  })
  @reactive.on_cleanup(() => {
    ignore(@native.call("router-release", { "id": id }))
  })
  {
    id,
    renderers,
    snapshot: @reactive.signal(router_call("router-state", { "id": id })),
  }
}

///|
pub fn Router::state(self : Router) -> Json {
  self.snapshot.get()
}

///|
pub fn Router::navigate(
  self : Router,
  destination : String,
  replace? : Bool = false,
) -> Unit {
  let operation = if replace { "router-replace" } else { "router-push" }
  self.snapshot.set(
    router_call(operation, { "id": self.id, "destination": destination }),
  )
}

///|
pub fn Router::go(self : Router, delta : Int) -> Unit {
  self.snapshot.set(router_call("router-go", { "id": self.id, "delta": delta }))
}

///|
pub fn Router::is_active(
  self : Router,
  destination : String,
  end? : Bool = true,
) -> Bool {
  ignore(self.snapshot.get())
  router_call("router-is-active", {
    "id": self.id,
    "destination": destination,
    "end": end,
  }) ==
  Json::boolean(true)
}

///|
pub fn Router::link(
  self : Router,
  destination : String,
  label : String,
) -> Element {
  button(label).href(destination).on_click(() => self.navigate(destination))
}

///|
/// Key each depth by the matched route ID. Navigating between sibling routes
/// keeps their common layout nodes, bindings and component owners alive.
pub fn Router::outlet(self : Router, depth? : Int = 0) -> Element {
  view([]).children_keyed(
    () => {
      match self.snapshot.get() {
        { "matched": { "routeIds": Array(ids), .. }, .. } =>
          if depth < ids.length() {
            match ids[depth] {
              String(id) => [id]
              _ => []
            }
          } else {
            []
          }
        _ => []
      }
    },
    id => id,
    id => {
      match self.renderers.get(id.get()) {
        Some(render) => render(self, () => self.outlet(depth=depth + 1))
        None => view([])
      }
    },
  )
}
