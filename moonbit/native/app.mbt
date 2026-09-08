///|
#valtype
pub(all) struct Window {
  id : Int
} derive(Eq)

///|
pub struct Event {
  kind : String
  window : Int
  target : Int
  value : String?
  extra : Json
  data : Bytes
  mut current_target : Int
  mut stopped : Bool
  mut default_prevented : Bool
}

///|
pub fn Event::stop_propagation(self : Event) -> Unit {
  self.stopped = true
}

///|
pub fn Event::prevent_default(self : Event) -> Unit {
  self.default_prevented = true
}

///|
priv struct WindowState {
  scope : Int
  nodes : Map[Int, NodeState]
  batch : @protocol.Batch
  mounted_tasks : Array[() -> Unit]
  mut created : Bool
  mut mounted : Bool
  mut closing : Bool
}

///|
priv struct Callback {
  scope : Int
  window : Int
  reply_kind : String
  run : (Result[Json, String]) -> Unit
  cancel_cleanup : () -> Unit
}

///|
priv struct Subscription {
  scope : Int
  window : Int
  kind : String
  run : (Event) -> Unit
  cancel_cleanup : () -> Unit
}

///|
let windows : Map[Int, WindowState] = Map([])

///|
let window_scopes : Map[Int, Int] = Map([])

///|
let dirty_windows : Map[Int, Unit] = Map([])

///|
let replies : Map[Int, Callback] = Map([])

///|
let subscriptions : Map[Int, Subscription] = Map([])

///|
let extensions : Map[String, String] = Map([])

///|
let rendering_window : @ref.Ref[Int] = @ref.Ref(0)

///|
let app_id : @ref.Ref[Int] = @ref.Ref(0)

///|
let application_scope : @ref.Ref[Int] = @ref.Ref(0)

///|
let next_request : @ref.Ref[Int] = @ref.Ref(1)

///|
let ready : @ref.Ref[Bool] = @ref.Ref(false)

///|
let exited : @ref.Ref[Bool] = @ref.Ref(false)

///|
let started : @ref.Ref[Bool] = @ref.Ref(false)

///|
let failure : @ref.Ref[String?] = @ref.Ref(None)

///|
let startup : @ref.Ref[() -> Unit] = @ref.Ref(() => ())

///|
let app_options : @ref.Ref[Json] = @ref.Ref(Json::empty_object())

///|
fn encode(value : Json) -> Bytes {
  @utf8.encode(value.stringify())
}

///|
fn object_fields(value : Json) -> Map[String, Json] {
  if value is Object(fields) {
    fields
  } else {
    Map([])
  }
}

///|
fn field(value : Json, key : String) -> Json? {
  if value is Object(fields) {
    fields.get(key)
  } else {
    None
  }
}

///|
fn error_field(value : Json) -> String? {
  if field(value, "error") is Some(String(error)) && error != "" {
    Some(error)
  } else {
    None
  }
}

///|
fn decode(bytes : Bytes) -> Json {
  if bytes.is_empty() {
    return Json::null()
  }
  @json.parse(@utf8.decode_lossy(bytes)) catch {
    _ => abort("QuickGUI returned invalid JSON")
  }
}

///|
fn check(code : Int) -> Unit {
  if code != 0 {
    abort("QuickGUI native operation failed (\{code})")
  }
}

///|
pub fn load() -> Result[Unit, String] {
  if ffi_load(@protocol.VERSION) == 0 {
    Ok(())
  } else {
    Err(@utf8.decode_lossy(ffi_error()))
  }
}

// Registration is declarative. Libraries are loaded once, before app startup.

///|
pub fn require_extension(name : String, version : String) -> Unit {
  if started.val {
    abort("Register extensions before native.run")
  }
  if extensions.get(name) is Some(previous) {
    if previous != version {
      abort("Conflicting versions for extension \{name}")
    }
    return
  }
  if extensions.length() >= 32 {
    abort("QuickGUI supports at most 32 extensions")
  }
  extensions[name] = version
}

///|
pub fn run(
  start : () -> Unit,
  options? : Json = Json::empty_object(),
) -> Result[Unit, String] {
  if started.val {
    return Err("native.run may only be called once")
  }
  match load() {
    Err(error) => return Err(error)
    Ok(_) => ()
  }
  // The CLI stages only the explicitly selected providers for this frontend.
  if field(build_config(), "extensions") is Some(Array(providers)) {
    for provider in providers {
      guard field(provider, "name") is Some(String(name)) &&
        field(provider, "version") is Some(String(version)) else {
        return Err("Invalid packaged extension metadata")
      }
      require_extension(name, version)
    }
  }
  started.val = true
  for name, version in extensions {
    if ffi_load_extension(@utf8.encode(name), @utf8.encode(version)) != 0 {
      return Err(@utf8.decode_lossy(ffi_error()))
    }
  }
  startup.val = start
  app_options.val = options
  let code = ffi_run(() => ui_main())
  startup.val = () => ()
  app_options.val = Json::null()
  if failure.val is Some(error) {
    return Err(error)
  }
  if code != 0 {
    return Err("QuickGUI host exited with status \{code}")
  }
  Ok(())
}

///|
fn ui_main() -> Unit {
  let scope = @reactive.create_scope(parent=0)
  application_scope.val = scope
  defer {
    for id in windows.keys().to_array() {
      close_state(id)
    }
    @reactive.dispose(scope)
    application_scope.val = 0
    ready.val = false
    replies.clear()
    subscriptions.clear()
    menu_callbacks.clear()
    app_menu_ids.val = []
  }
  let config = object_fields(decode(ffi_metadata())).copy()
  for key, value in object_fields(app_options.val) {
    config[key] = value
  }
  if config.contains("identifier") {
    config["resourceDir"] = Json::string(
      @utf8.decode_lossy(ffi_resource_directory()),
    )
  }
  app_id.val = ffi_create_app(Json::object(config) |> encode)
  if app_id.val == 0 {
    ffi_abort(b"Could not create QuickGUI app")
    return
  }
  check(ffi_prepare_app(app_id.val))
  while !ready.val && !exited.val {
    turn()
  }
  if exited.val {
    return
  }
  @reactive.with_owner(scope, () => @reactive.batch(startup.val))
  flush()
  while !exited.val {
    turn()
  }
}

///|
fn turn() -> Unit {
  let first = ffi_next_event(true)
  if first == -1 {
    failure.val = Some(@utf8.decode_lossy(ffi_error()))
  }
  if first <= 0 {
    exited.val = true
    return
  }
  @reactive.batch(() => {
    dispatch(read_event())
    // Bound a turn so continuous producers cannot starve rendering.
    for i = 0; i < 255 && !exited.val; i = i + 1 {
      let next = ffi_next_event(false)
      if next == -1 {
        failure.val = Some(@utf8.decode_lossy(ffi_error()))
        exited.val = true
      }
      if next != 1 {
        break
      }
      dispatch(read_event())
    }
  })
  flush()
}

///|
fn read_event() -> Event {
  let flags = ffi_event_number(2)
  {
    kind: @utf8.decode_lossy(ffi_event_bytes(0)),
    window: ffi_event_number(0),
    target: ffi_event_number(1),
    current_target: ffi_event_number(1),
    value: if (flags & 1) != 0 {
      Some(@utf8.decode_lossy(ffi_event_bytes(1)))
    } else {
      None
    },
    extra: if (flags & 2) != 0 {
      decode(ffi_event_bytes(2))
    } else {
      Json::null()
    },
    data: if (flags & 4) != 0 {
      ffi_event_bytes(3)
    } else {
      b""
    },
    stopped: false,
    default_prevented: false,
  }
}

///|
fn dispatch(event : Event) -> Unit {
  match event.kind {
    "window-ready-to-show" =>
      if windows.get(event.window) is Some(window) {
        window.mounted = true
        let tasks = window.mounted_tasks.copy()
        window.mounted_tasks.clear()
        for task in tasks {
          task()
        }
      }
    "app-ready" => {
      if error_field(event.extra) is Some(error) {
        failure.val = Some(error)
        ffi_abort(@utf8.encode(error))
        exited.val = true
      } else {
        ready.val = true
      }
      return
    }
    "host-error" => {
      let error = error_field(event.extra).unwrap_or("Native host failed")
      failure.val = Some(error)
      exited.val = true
      ffi_abort(@utf8.encode(error))
      return
    }
    "exit" => exited.val = true
    "invoke"
    | "command"
    | "shell"
    | "popup-menu"
    | "app-service"
    | "file-icon"
    | "notification-permission"
    | "global-shortcut-operation"
    | "tray-operation"
    | "user-tasks"
    | "alert-dialog"
    | "open-dialog"
    | "save-dialog" => {
      if replies.get(event.target) is Some(callback) {
        // Deferred services first acknowledge the command, then report the
        // actual result. Keep callbacks (including popup actions) until then.
        if callback.reply_kind != "" &&
          event.kind != callback.reply_kind &&
          error_field(event.extra) is None {
          return
        }
        replies.remove(event.target)
        (callback.cancel_cleanup)()
        let result = match error_field(event.extra) {
          Some(error) => Err(error)
          None =>
            Ok(
              match event.kind {
                "open-dialog" | "save-dialog" =>
                  field(event.extra, "paths").unwrap_or(Json::array([]))
                "alert-dialog" => event.value.unwrap_or("").to_json()
                _ =>
                  match event.value {
                    Some(text) => decode(@utf8.encode(text))
                    None => Json::null()
                  }
              },
            )
        }
        rendering_window.protect(callback.window, () => {
          @reactive.with_owner(callback.scope, () => (callback.run)(result))
        })
      }
      return
    }
    "menu-action" => {
      dispatch_menu(event.target)
      return
    }
    _ => ()
  }
  for id in subscriptions.keys().to_array() {
    if subscriptions.get(id) is Some(listener) {
      if listener.kind == event.kind &&
        (listener.window == 0 || listener.window == event.window) {
        rendering_window.protect(listener.window, () => {
          @reactive.with_owner(listener.scope, () => (listener.run)(event))
        })
      }
    }
  }
  if event.kind == "close" {
    close_state(event.window)
    return
  }
  if windows.contains(event.window) {
    rendering_window.protect(event.window, () => dispatch_node(event))
  }
}

///|
fn context_window_id() -> Int {
  // Effects can settle after the event callback's dynamic context has ended.
  // Their owner lineage is authoritative, including cross-window signal updates.
  let mut scope = @reactive.owner()
  while scope != 0 {
    if window_scopes.get(scope) is Some(id) {
      return id
    }
    scope = @reactive.parent_owner(scope)
  }
  rendering_window.val
}

///|
pub fn current_window() -> Window {
  let id = context_window_id()
  if id == 0 || !windows.contains(id) {
    abort("current_window requires a live window component or callback")
  }
  { id, }
}

///|
pub fn new_window(
  component : () -> Node,
  options? : Json = Json::empty_object(),
) -> Window {
  create_window(component, options, None)
}

///|
pub fn new_system_popover(
  anchor : Node,
  component : () -> Node,
  options? : Json = Json::empty_object(),
) -> Window {
  flush()
  create_window(component, options, Some((anchor.window, anchor.id, 0)))
}

///|
pub fn new_embedded_view(
  parent : Window,
  component : () -> Node,
  match_horizontal? : Bool = false,
  match_vertical? : Bool = false,
  options? : Json = Json::empty_object(),
) -> Window {
  flush()
  let mode = 1 +
    (if match_horizontal { 2 } else { 0 }) +
    (if match_vertical { 4 } else { 0 })
  create_window(component, options, Some((parent.id, 0, mode)))
}

///|
fn create_window(
  component : () -> Node,
  options : Json,
  child : (Int, Int, Int)?,
) -> Window {
  if !ready.val {
    abort("Create windows inside native.run or an application callback")
  }
  let id = ffi_allocate_window()
  if id == 0 {
    abort("Could not allocate native window")
  }
  // A second window outlives the component or event which opened it.
  let scope = @reactive.create_scope(parent=0)
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
  rendering_window.protect(id, () => {
    @reactive.with_owner(scope, () => {
      let root = component()
      Node::{ window: id, id: 0, }.append(root)
    })
  })
  let initial = window.batch.finish()
  match child {
    None => check(ffi_create_window(app_id.val, id, encode(options), initial))
    Some((parent, anchor, mode)) =>
      check(
        ffi_create_child(
          app_id.val,
          id,
          parent,
          anchor,
          mode,
          encode(options),
          initial,
        ),
      )
  }
  window.created = true
  dirty_windows.remove(id)
  { id, }
}

///|
fn close_state(id : Int) -> Unit {
  if windows.get(id) is Some(window) {
    windows.remove(id)
    window_scopes.remove(window.scope)
    dirty_windows.remove(id)
    for key, listener in subscriptions.copy() {
      if listener.window == id {
        subscriptions.remove(key)
        (listener.cancel_cleanup)()
      }
    }
    @reactive.dispose(window.scope)
  }
}

///|
pub fn flush() -> Unit {
  @reactive.flush()
  for id in dirty_windows.keys().to_array() {
    if windows.get(id) is Some(window) {
      if window.closing {
        // Close is enqueued immediately. State callbacks already in this turn
        // may still settle effects, but must not enqueue a batch after it.
        ignore(window.batch.finish())
      } else if window.created && !window.batch.is_empty() {
        check(ffi_apply_batch(app_id.val, id, window.batch.finish()))
      }
    }
    dirty_windows.remove(id)
  }
}

///|
pub fn Window::close(self : Window) -> Unit {
  if windows.get(self.id) is Some(window) && !window.closing {
    window.closing = true
    check(ffi_close_window(app_id.val, self.id))
  }
}

///|
fn request_id() -> Int {
  if next_request.val == 0x7fffffff {
    abort("QuickGUI request ID space exhausted")
  }
  next_request.val += 1
  next_request.val
}

///|
fn callback_id(
  callback : (Result[Json, String]) -> Unit,
  reply_kind? : String = "",
) -> Int {
  if !ready.val {
    abort("Native requests require a running application")
  }
  let id = request_id()
  let scope = callback_owner()
  let cancel_cleanup = if scope == 0 {
    () => ()
  } else {
    @reactive.with_owner(scope, () => {
      @reactive.on_cleanup_cancelable(() => replies.remove(id))
    })
  }
  replies[id] = {
    scope,
    window: context_window_id(),
    reply_kind,
    run: callback,
    cancel_cleanup,
  }
  id
}

///|
fn callback_owner() -> Int {
  let scope = @reactive.owner()
  if @reactive.owner_alive(scope) {
    scope
  } else if @reactive.owner_alive(application_scope.val) {
    application_scope.val
  } else {
    0
  }
}

// Platform commands and services return asynchronously on the MoonBit UI thread.

///|
pub fn command(value : Json, callback : (Result[Json, String]) -> Unit) -> Unit {
  let reply_kind = deferred_reply_kind(value)
  let id = callback_id(callback, reply_kind~)
  check(ffi_command(app_id.val, id, encode(command_request(value, id))))
}

///|
fn deferred_reply_kind(value : Json) -> String {
  match field(value, "method") {
    Some(
      String("app-service" | "file-icon" | "notification-permission" as name)
    ) => name
    Some(String("window-popup-menu")) => "popup-menu"
    Some(String("shell-action")) => "shell"
    Some(String("global-shortcut")) => "global-shortcut-operation"
    Some(String("set-tray-icon" | "remove-tray-icon" | "show-tray-menu")) =>
      "tray-operation"
    Some(String("set-user-tasks")) => "user-tasks"
    _ => ""
  }
}

///|
fn command_request(value : Json, request : Int) -> Json {
  if deferred_reply_kind(value) == "" {
    return value
  }
  let fields = object_fields(value).copy()
  fields["request"] = request.to_json()
  fields.to_json()
}

///|
pub fn invoke(
  operation : String,
  params : Json,
  callback : (Result[Json, String]) -> Unit,
) -> Unit {
  let id = callback_id(callback)
  check(ffi_invoke(id, @utf8.encode(operation), encode(params)))
}

// Only the Rust core's CPU-only service allowlist is synchronous.

///|
pub fn call(operation : String, params : Json) -> Result[Json, String] {
  match load() {
    Err(error) => return Err(error)
    Ok(_) => ()
  }
  let reply = decode(ffi_call(@utf8.encode(operation), encode(params)))
  if field(reply, "ok") is Some(True) {
    Ok(field(reply, "value").unwrap_or(Json::null()))
  } else {
    Err(error_field(reply).unwrap_or("Native call failed"))
  }
}

// kind: 0 alert, 1 open, 2 save. Options are the core's native dialog schema.

///|
fn dialog_options(kind : Int, options : Json) -> Json {
  guard options is Object(fields) else { return options }
  let defaults : Json = match kind {
    1 =>
      {
        "files": true,
        "directories": false,
        "multiple": false,
        "filters": [],
        "showsHiddenFiles": false,
      }
    2 => { "directory": "", "filters": [], "showsHiddenFiles": false }
    _ => Json::empty_object()
  }
  let merged = object_fields(defaults)
  for key, value in fields {
    merged[key] = value
  }
  merged.to_json()
}

///|
pub fn Window::dialog(
  self : Window,
  kind : Int,
  options : Json,
  callback : (Result[Json, String]) -> Unit,
) -> Unit {
  let id = callback_id(callback)
  check(
    ffi_dialog(
      app_id.val,
      self.id,
      id,
      kind,
      encode(dialog_options(kind, options)),
    ),
  )
}

///|
pub fn on_event(
  kind : String,
  listener : (Event) -> Unit,
  window? : Int = context_window_id(),
) -> Unit {
  ignore(subscribe(kind, listener, window~))
}

///|
/// A cancellable subscription; its registering owner also cancels it on disposal.
pub fn subscribe(
  kind : String,
  listener : (Event) -> Unit,
  window? : Int = context_window_id(),
) -> () -> Unit {
  if window != 0 && !windows.contains(window) {
    return () => ()
  }
  let id = request_id()
  let scope = callback_owner()
  if scope == 0 {
    abort("Event subscriptions require a live owner")
  }
  let cancel_cleanup = @reactive.with_owner(scope, () => {
    @reactive.on_cleanup_cancelable(() => subscriptions.remove(id))
  })
  subscriptions[id] = { scope, window, kind, run: listener, cancel_cleanup, }
  () => {
    subscriptions.remove(id)
    cancel_cleanup()
  }
}

///|
/// Schedule callbacks against a window's lifetime rather than a transient view.
pub fn[T] Window::with_owner(self : Window, run : () -> T) -> T {
  let state = windows.get(self.id).unwrap()
  rendering_window.protect(self.id, () => @reactive.with_owner(state.scope, run))
}

///|
pub fn Window::is_open(self : Window) -> Bool {
  windows.contains(self.id)
}

///|
pub fn Window::focus(self : Window) -> Unit {
  self.action("focus")
}

///|
pub fn Event::json(self : Event) -> Json {
  match self.value {
    Some(value) => @json.parse(value) catch { _ => self.extra }
    None => self.extra
  }
}

///|
pub fn build_config() -> Json {
  decode(ffi_metadata())
}

///|
pub fn Window::action(
  self : Window,
  action : String,
  value? : String = "",
) -> Unit {
  if windows.contains(self.id) {
    check(
      ffi_command(
        app_id.val,
        0,
        encode({
          "method": "window-action",
          "window": self.id,
          "action": action,
          "value": value,
        }),
      ),
    )
  }
}

///|
pub fn quit() -> Unit {
  if !ready.val {
    abort("quit requires a running application")
  }
  command({ "method": "request-quit" }, _ => ())
}
