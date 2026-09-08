///|
pub(all) enum MenuItem {
  Action(
    String,
    () -> Unit,
    accelerator~ : String,
    enabled~ : Bool,
    checked~ : Bool
  )
  Role(String, String, accelerator~ : String, enabled~ : Bool)
  Submenu(String, Array[MenuItem], enabled~ : Bool)
  Separator
}

///|
pub fn menu_action(
  label : String,
  run : () -> Unit,
  accelerator? : String = "",
  enabled? : Bool = true,
  checked? : Bool = false,
) -> MenuItem {
  Action(label, run, accelerator~, enabled~, checked~)
}

///|
pub fn menu_role(
  label : String,
  role : String,
  accelerator? : String = "",
  enabled? : Bool = true,
) -> MenuItem {
  Role(label, role, accelerator~, enabled~)
}

///|
pub fn submenu(
  label : String,
  items : Array[MenuItem],
  enabled? : Bool = true,
) -> MenuItem {
  Submenu(label, items, enabled~)
}

///|
priv struct MenuCallback {
  scope : Int
  window : Int
  run : () -> Unit
}

///|
let menu_callbacks : Map[Int, MenuCallback] = Map([])

///|
let app_menu_ids : @ref.Ref[Array[Int]] = @ref.Ref([])

///|
fn encode_menu(items : Array[MenuItem], ids : Array[Int]) -> Json {
  let result : Array[Json] = []
  for item in items {
    result.push(
      match item {
        Separator => { "type": "separator" }
        Submenu(label, children, enabled~) =>
          {
            "type": "submenu",
            "label": label,
            "enabled": enabled,
            "items": encode_menu(children, ids),
          }
        Role(label, role, accelerator~, enabled~) =>
          {
            "type": "role",
            "label": label,
            "role": role,
            "accelerator": if accelerator == "" {
              Json::null()
            } else {
              accelerator.to_json()
            },
            "enabled": enabled,
          }
        Action(label, run, accelerator~, enabled~, checked~) => {
          let id = request_id()
          menu_callbacks[id] = {
            scope: callback_owner(),
            window: context_window_id(),
            run,
          }
          ids.push(id)
          {
            "type": "action",
            "id": id,
            "label": label,
            "enabled": enabled,
            "checked": checked,
            "accelerator": if accelerator == "" {
              Json::null()
            } else {
              accelerator.to_json()
            },
          }
        }
      },
    )
  }
  result.to_json()
}

///|
fn dispatch_menu(id : Int) -> Unit {
  if menu_callbacks.get(id) is Some(callback) &&
    @reactive.owner_alive(callback.scope) {
    rendering_window.protect(callback.window, () => {
      @reactive.with_owner(callback.scope, callback.run)
    })
  }
}

///|
pub fn set_application_menu(
  menus : Array[(String, Array[MenuItem])],
  done? : (Result[Json, String]) -> Unit = _ => (),
) -> Unit {
  for id in app_menu_ids.val {
    menu_callbacks.remove(id)
  }
  let ids : Array[Int] = []
  let menu : Array[Json] = menus.map(pair => {
    "label": pair.0,
    "enabled": true,
    "items": encode_menu(pair.1, ids),
  })
  app_menu_ids.val = ids
  command(
    { "method": "set-application-menu", "menu": menu.to_json().stringify() },
    done,
  )
}

///|
pub fn Window::popup_menu(
  self : Window,
  items : Array[MenuItem],
  done? : (Result[Json, String]) -> Unit = _ => (),
) -> Unit {
  let ids : Array[Int] = []
  let menu : Json = [
    { "label": "Context", "enabled": true, "items": encode_menu(items, ids) },
  ]
  let release = () => {
    for id in ids {
      menu_callbacks.remove(id)
    }
  }
  let cancel_cleanup = @reactive.on_cleanup_cancelable(release)
  command(
    {
      "method": "window-popup-menu",
      "window": self.id,
      "menu": menu.stringify(),
    },
    result => {
      release()
      cancel_cleanup()
      done(result)
    },
  )
}

///|
/// Recursive OS notifications. Retiring the owner cancels even a registration
/// whose asynchronous completion has not arrived yet.
pub fn watch_files(
  paths : Array[String],
  changed : (Json) -> Unit,
  ready : (Result[Unit, String]) -> Unit,
) -> () -> Unit {
  let id = request_id()
  let stopped = @ref.Ref(false)
  let registered = @ref.Ref(false)
  let owner = callback_owner()
  let off = subscribe(
    "file-watch",
    e => if e.target == id { changed(e.json()) },
    window=0,
  )
  let unwatch = () => invoke("unwatch-files", { "id": id }, _ => ())
  let stop = () => {
    if !stopped.val {
      stopped.val = true
      off()
      if registered.val {
        unwatch()
      }
    }
  }
  let cancel_cleanup = @reactive.on_cleanup_cancelable(stop)
  // Keep registration completion on the application scope so a disposed window
  // cannot strand a watcher that native has just accepted.
  @reactive.with_owner(application_scope.val, () => {
    invoke("watch-files", { "id": id, "paths": paths }, result => {
      match result {
        Ok(_) => {
          registered.val = true
          if stopped.val {
            unwatch()
          }
        }
        Err(_) => off()
      }
      if !stopped.val && @reactive.owner_alive(owner) {
        @reactive.with_owner(owner, () => ready(result.map(_ => ())))
      }
    })
  })
  () => {
    stop()
    cancel_cleanup()
  }
}
