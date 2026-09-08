///|
let sessions : Map[Int, Store] = Map([])

///|
let active : @reactive.Signal[Int] = @reactive.signal(0)

///|
let recents : @reactive.Signal[Array[String]] = @reactive.signal([])

///|
let preferences : Map[String, Json] = Map([])

///|
let state_path : @ref.Ref[String] = @ref.Ref("")

///|
let last_repository : @ref.Ref[String] = @ref.Ref("")

///|
let saving : @ref.Ref[Bool] = @ref.Ref(false)

///|
let save_again : @ref.Ref[Bool] = @ref.Ref(false)

///|
let app_owner : @ref.Ref[Int] = @ref.Ref(0)

///|
let clock_now : @ref.Ref[Int64] = @ref.Ref(0)

///|
let agents : @reactive.Signal[Array[String]] = @reactive.signal([])

///|
let preferred_agent : @reactive.Signal[String] = @reactive.signal("codex")

///|
fn save() -> Unit {
  if state_path.val == "" {
    return
  }
  if saving.val {
    save_again.val = true
    return
  }
  saving.val = true
  let state : Json = {
    "recent": recents.peek(),
    "last": last_repository.val,
    "repositories": preferences,
    "agent": preferred_agent.peek(),
  }
  @reactive.with_owner(app_owner.val, () => {
    @io.save_state(state_path.val, state, result => {
      saving.val = false
      if result is Err(error) {
        if sessions.get(active.peek()) is Some(s) {
          s.error("Could not save settings: " + error)
        }
      }
      if save_again.val {
        save_again.val = false
        save()
      }
    })
  })
}

///|
fn remember_repository(path : String) -> Unit {
  let items = [path, ..recents.peek().filter(p => p != path)]
  recents.set(items[:items.length().min(12)].to_owned())
  last_repository.val = path
  for key in preferences.keys().to_array() {
    if !recents.peek().contains(key) {
      preferences.remove(key)
    }
  }
  save()
}

///|
fn Store::persist(self : Store) -> Unit {
  if self.root.peek() == "" {
    return
  }
  preferences[self.root.peek()] = {
    "subject": self.subject.peek(),
    "body": self.body.peek(),
    "sidebar": self.sidebar_width.peek(),
    "changes": self.changes_width.peek(),
    "history": self.history_width.peek(),
  }
  save()
}

///|
fn Store::restore(self : Store) -> Unit {
  if preferences.get(self.root.peek()) is Some(pref) {
    self.subject.set(@io.string(@io.field(pref, "subject")))
    self.body.set(@io.string(@io.field(pref, "body")))
    if @io.field(pref, "sidebar") is Number(n, ..) {
      self.sidebar_width.set(n.clamp(min=180, max=420))
    }
    if @io.field(pref, "changes") is Number(n, ..) {
      self.changes_width.set(n.clamp(min=220, max=650))
    }
    if @io.field(pref, "history") is Number(n, ..) {
      self.history_width.set(n.clamp(min=360, max=900))
    }
  }
}

///|
fn shell(
  action : String,
  path : String,
  done? : (Result[Json, String]) -> Unit = _ => (),
) -> Unit {
  @native.command(
    { "method": "shell-action", "action": action, "value": path },
    done,
  )
}

///|
fn clipboard(text : String) -> Unit {
  @native.command(
    {
      "method": "write-clipboard",
      "item": { "entries": [{ "kind": "text", "text": text }] },
    },
    _ => (),
  )
}

///|
fn with_store(run : (Store) -> Unit) -> Unit {
  if sessions.get(active.peek()) is Some(s) && s.window.is_open() {
    s.window.with_owner(() => run(s))
  }
}

///|
fn open_repository(path : String) -> Unit {
  @io.service("canonical", { "path": path }, result => {
    let target = match result {
      Ok(value) => @io.string(value)
      Err(_) => path
    }
    for _, s in sessions {
      if s.root.peek() == target {
        s.window.focus()
        return
      }
    }
    if sessions.get(active.peek()) is Some(s) &&
      s.root.peek() == "" &&
      !s.opening.peek() {
      s.open(target)
    } else {
      open_window(target)
    }
  })
}

///|
fn open_dialog() -> Unit {
  if !sessions.contains(active.peek()) {
    open_window("")
  }
  with_store(s => {
    s.window.dialog(
      1,
      {
        "title": "Open Repository",
        "prompt": "Open",
        "directories": true,
        "files": false,
        "multiple": false,
      },
      result => {
        if result is Ok(Array([String(path), ..])) {
          open_repository(path)
        } else if result is Err(error) {
          s.error(error)
        }
      },
    )
  })
}

///|
fn open_window(initial : String) -> Unit {
  ignore(create_window(initial))
}

///|
/// Raw native window options, shared by the application and its native test.
/// Color is packed RGBA; these are wire names rather than Go WindowOptions.
fn window_options(show? : Bool = true) -> Json {
  {
    "title": "Quick Git · MoonBit",
    "width": 1240,
    "height": 800,
    "minimumWidth": 900,
    "minimumHeight": 560,
    "background": 0,
    "vibrancy": "sidebar",
    "visualEffectState": "followWindow",
    "appearance": "system",
    "titleBarStyle": "hiddenInset",
    "trafficLightX": 14,
    "trafficLightY": 19,
    "show": show,
    "focus": show,
  }
}

///|
/// The smoke test uses this same constructor, suppressing only visibility/focus.
fn create_window(initial : String, show? : Bool = true) -> @native.Window {
  let window = @ui.window(
    () => {
      let w = @native.current_window()
      let s = store(w)
      sessions[w.id] = s
      @reactive.on_cleanup(() => {
        s.persist()
        (s.watcher)()
        sessions.remove(w.id)
        if active.peek() == w.id {
          active.set(sessions.keys().to_array().get(0).unwrap_or(0))
        }
      })
      @native.on_event("window-focus", e => {
        if e.value == Some("true") {
          active.set(w.id)
          s.refresh()
        }
      })
      @native.on_event("window-appearance", e => {
        let value = e.value.unwrap_or("light")
        s.appearance.set(if value == "dark" { "dark" } else { "light" })
      })
      @native.on_event("window-ready-to-show", _ => {
        @native.command({ "method": "get-window-state", "window": w.id }, result => {
          if result is Ok(value) {
            s.appearance.set(
              if @io.string(@io.field(value, "appearance")) == "dark" {
                "dark"
              } else {
                "light"
              },
            )
          }
        })
      })
      let root = app_view(s)
      if initial != "" {
        s.open(initial, activate=show)
      }
      root
    },
    options=window_options(show~),
  )
  active.set(window.id)
  if show {
    window.focus()
  }
  window
}

///|
fn install_menu() -> Unit {
  let current = sessions.get(active.get())
  let has_window = current is Some(_)
  let has_repo = match current {
    Some(s) => s.root.get() != ""
    None => false
  }
  fn action(
    label : String,
    accelerator : String,
    run : (Store) -> Unit,
  ) -> @native.MenuItem {
    @native.menu_action(
      label,
      () => with_store(run),
      accelerator~,
      enabled=has_repo,
    )
  }
  @native.set_application_menu(
    [
      (
        "File",
        [
          @native.menu_action(
            "Open Repository…",
            open_dialog,
            accelerator="CmdOrCtrl+O",
          ),
          @native.submenu(
            "Open Recent",
            recents
            .get()
            .map(path => @native.menu_action(path, () => open_repository(path))),
            enabled=!recents.get().is_empty(),
          ),
          @native.Separator,
          action("New Branch…", "CmdOrCtrl+Shift+N", s => {
            s.open_dialog("branch")
          }),
          action("New Worktree…", "CmdOrCtrl+Alt+N", s => {
            s.open_dialog("worktree")
          }),
          action("Stash Changes…", "CmdOrCtrl+Shift+T", s => {
            s.open_dialog("stash")
          }),
          @native.Separator,
          action("Close Repository", "CmdOrCtrl+Shift+W", s => s.close()),
          @native.menu_role(
            "Close Window",
            "close-window",
            accelerator="CmdOrCtrl+W",
            enabled=has_window,
          ),
        ],
      ),
      (
        "Edit",
        [
          @native.menu_role("Undo", "undo", accelerator="CmdOrCtrl+Z"),
          @native.menu_role("Redo", "redo", accelerator="CmdOrCtrl+Shift+Z"),
          @native.Separator,
          @native.menu_role("Cut", "cut", accelerator="CmdOrCtrl+X"),
          @native.menu_role("Copy", "copy", accelerator="CmdOrCtrl+C"),
          @native.menu_role("Paste", "paste", accelerator="CmdOrCtrl+V"),
          @native.menu_role(
            "Select All",
            "select-all",
            accelerator="CmdOrCtrl+A",
          ),
        ],
      ),
      (
        "View",
        [
          action("Changes", "CmdOrCtrl+1", s => s.set_view("changes")),
          action("History", "CmdOrCtrl+2", s => s.set_view("history")),
          action("Branches", "CmdOrCtrl+3", s => s.set_view("branches")),
          action("Worktrees", "CmdOrCtrl+4", s => s.set_view("worktrees")),
          action("Stashes", "CmdOrCtrl+5", s => s.set_view("stashes")),
          @native.Separator,
          action("Refresh", "CmdOrCtrl+R", s => s.refresh()),
          @native.menu_role(
            "Toggle Full Screen",
            "toggle-fullscreen",
            accelerator="Ctrl+Cmd+F",
            enabled=has_window,
          ),
        ],
      ),
      (
        "Repository",
        [
          action("Commit", "CmdOrCtrl+Enter", s => s.commit()),
          action("Generate Commit Message", "CmdOrCtrl+Shift+G", s => {
            s.generate("")
          }),
          @native.Separator,
          action("Stage All", "CmdOrCtrl+Shift+A", s => s.stage_all()),
          action("Unstage All", "CmdOrCtrl+Shift+U", s => s.unstage_all()),
          @native.Separator,
          action("Fetch", "CmdOrCtrl+Shift+F", s => {
            s.mutate("Fetch", ["fetch", "--prune"])
          }),
          action("Pull", "CmdOrCtrl+Shift+L", s => {
            s.mutate("Pull", ["pull", "--no-edit"])
          }),
          action("Push", "CmdOrCtrl+Shift+P", s => s.push()),
          @native.Separator,
          action("Reveal in Finder", "", s => {
            shell("reveal-path", s.root.peek())
          }),
        ],
      ),
      (
        "Window",
        [
          @native.menu_role(
            "Minimize",
            "minimize-window",
            accelerator="CmdOrCtrl+M",
            enabled=has_window,
          ),
          @native.menu_role("Zoom", "zoom-window", enabled=has_window),
          @native.Separator,
          @native.menu_role(
            "Bring All to Front",
            "bring-all-to-front",
            enabled=has_window,
          ),
        ],
      ),
      (
        "Help",
        [
          @native.menu_action("QuickGUI on GitHub", () => {
            shell("open-external", "https://github.com/egoist/quickgui")
          }),
        ],
      ),
    ],
    done=result => {
      if result is Err(error) {
        if sessions.get(active.peek()) is Some(s) {
          s.error("Application menu: " + error)
        } else {
          println("Application menu: " + error)
        }
      }
    },
  )
}

///|
pub fn run() -> Unit {
  @native.require_extension("quick-git-io", "1.0.0")
  match
    @native.run(
      () => {
        app_owner.val = @reactive.owner()
        @io.start()
        @native.on_event(
          "reopen",
          _ => if sessions.is_empty() { open_window("") },
          window=0,
        )
        @native.command({ "method": "get-app-paths" }, result => {
          if result is Ok(paths) {
            let data = @io.string(@io.field(paths, "dataDir"))
            if data != "" {
              state_path.val = data + "/quick-git-state.json"
            }
          }
          @io.service("environment", {}, environment => {
            let env = environment.unwrap_or(Json::empty_object())
            if @io.field(env, "now") is Number(n, ..) {
              clock_now.val = n.to_int64()
            }
            let available : Array[String] = []
            for agent in ["codex", "claude"] {
              if @io.boolean(@io.field(env, agent)) {
                available.push(agent)
              }
            }
            agents.set(available)
            @io.load_state(state_path.val, result => {
              let state = result.unwrap_or(Json::empty_object())
              let recent = @io.strings(@io.field(state, "recent"))
              recents.set(recent[:recent.length().min(12)].to_owned())
              last_repository.val = @io.string(@io.field(state, "last"))
              if @io.field(state, "repositories") is Object(items) {
                for key, value in items {
                  if recents.peek().contains(key) {
                    preferences[key] = value
                  }
                }
              }
              let agent = @io.string(@io.field(state, "agent"))
              preferred_agent.set(
                if available.contains(agent) {
                  agent
                } else {
                  available.get(0).unwrap_or("codex")
                },
              )
              @reactive.create_effect(install_menu)
              let initial = @io.string(@io.field(env, "open"))
              open_window(
                if initial != "" {
                  initial
                } else {
                  last_repository.val
                },
              )
            })
          })
        })
      },
      options={ "quitMode": "explicit" },
    ) {
    Err(error) => abort(error)
    Ok(_) => ()
  }
}
