///|
fn dialog_content(t : Theme, alert : Bool) -> @ui.Element {
  let (open, set_open) = @ui.create_signal(false)
  let state : @reactive.Signal[Json] = @reactive.signal({})
  let (outcome, set_outcome) = @ui.create_signal("nothing yet")
  let notes = @reactive.signal("")
  let root = (if alert { @ui.alert_dialog([]) } else { @ui.dialog([]) })
    .bind_open(() => open())
    .exit_duration(120)
    .on_change(data => {
      if get(data, "open") is True {
        set_open(true)
      }
      if get(data, "open") is False {
        set_open(false)
      }
      state.set(data)
    })
  let content = popup(t, root.slot(@ui.Popup), if alert { 340 } else { 360 }).children([
      root
      .slot(@ui.Title)
      .label(
        if alert {
          "Delete “electron-parity”?"
        } else {
          "Publish this build?"
        },
      )
      .font_size(15)
      .font_weight(700),
      root
      .slot(@ui.Description)
      .children([
        muted(
          t,
          if alert {
            "This gallery records the answer; it does not delete a real branch."
          } else {
            "Escape, the backdrop, and Cancel restore focus to the trigger."
          },
        ),
      ]),
    ],
  )
  if !alert {
    ignore(
      content.children([
        root
        .slot(@ui.Viewport)
        .children([
          input(t, notes, "Release notes")
          .width("100%")
          .height(64)
          .multiline(true)
          .padding_top(6),
        ])
        .max_height(160)
        .padding(4),
      ]),
    )
  }
  ignore(
    content.children([
      row().children([
        control(t, root.slot(@ui.Close).label("Cancel")),
        button(
          t,
          if alert {
            "Delete"
          } else {
            "Publish"
          },
          () => {
            set_outcome(if alert { "deleted" } else { "published" })
            set_open(false)
          },
          primary=true,
        ),
      ]),
    ]),
  )
  let portal = root
    .slot(@ui.Portal)
    .children([
      root
      .slot(@ui.Backdrop)
      .position("absolute")
      .left(0)
      .top(0)
      .right(0)
      .bottom(0)
      .bind_style(() => @ui.style().bg(t.color("backdrop"))),
      content,
    ])
    .position("absolute")
    .left(0)
    .top(0)
    .right(0)
    .bottom(0)
    .display("flex")
    .items_center()
    .justify_center()
  panel(
    t,
    if alert {
      "Alert dialog"
    } else {
      "Dialog"
    },
    if alert {
      "The backdrop refuses dismissal. Choose an answer or press Escape; focus returns to the trigger."
    } else {
      "An in-window dialog with a focus trap, scrollable body, and a 120ms exit deadline."
    },
  ).children([
    root.children([
      control(
        t,
        root
        .slot(@ui.Trigger)
        .label(if alert { "Delete branch…" } else { "Open dialog" }),
      ),
      portal,
    ]),
    note(t, () => {
      "open \{open()} · outcome \{outcome()} · notes \{notes.get().length()} chars"
    }),
    readout(t, state),
  ])
}

///|
fn dialog_demo(t : Theme) -> @ui.Element {
  dialog_content(t, false)
}

///|
fn alert_dialog_demo(t : Theme) -> @ui.Element {
  dialog_content(t, true)
}

///|
fn menu_model(t : Theme, items : Array[Json]) -> Json {
  {
    "items": items,
    "width": 220,
    "itemHeight": 28,
    "fontSize": 12,
    "radius": 10,
    "background": t.color("popup"),
    "color": t.color("ink"),
    "highlightBackground": t.color("accent"),
    "highlightColor": t.color("on-accent"),
    "mutedColor": t.color("muted"),
  }
}

///|
fn menu_row(t : Theme, node : @ui.Element) -> @ui.Element {
  node
  .flex_row()
  .items_center()
  .justify_content("space-between")
  .height(28)
  .px(10)
  .border_radius(5)
  .font_size(12)
  .cursor("default")
  .user_select("none")
  .bind_style(() => @ui.style().text_color(t.color("ink")))
  .bind_json(@protocol.HOVER_STYLE, () => {
    @ui.style().bg(t.color("selection")).json()
  })
  .property(@protocol.DISABLED_STYLE, @protocol.Text("{\"opacity\":0.45}"))
}

///|
fn context_menu_demo(t : Theme) -> @ui.Element {
  let (command, set_command) = @ui.create_signal("nothing yet")
  let (parts, set_parts) = @ui.create_signal("nothing yet")
  let declared = @ui.context_menu([]).bind_menu(() => {
    menu_model(t, [
      { "id": "cut", "label": "Cut" },
      { "id": "copy", "label": "Copy" },
      { "type": "separator" },
      { "id": "paste", "label": "Paste" },
    ])
  })
  let from_parts = @ui.context_menu([]).bind_menu(() => menu_model(t, []))
  let part_target = framed(t, from_parts.slot(@ui.Trigger))
    .height(64)
    .display("flex")
    .items_center()
    .justify_center()
    .border_style("dashed")
    .on("menuselect", @protocol.SELECT_LISTENER, e => {
      set_parts(string(get(e.json(), "id")))
    })
  let rows = @ui.menu([])
  for
    entry in [
      ("rename", "Rename"),
      ("duplicate", "Duplicate"),
      ("delete", "Delete"),
    ] {
    ignore(
      part_target.children([rows.slot(@ui.Item, value=entry.0).label(entry.1)]),
    )
  }
  panel(
    t, "Context menu", "Secondary-click menus from a declared row model or from Menu.Item parts.",
  ).children([
    declared.children([
      framed(t, declared.slot(@ui.Trigger))
      .height(64)
      .display("flex")
      .items_center()
      .justify_center()
      .border_style("dashed")
      .label("Right-click: declared rows")
      .on("menuselect", @protocol.SELECT_LISTENER, e => {
        set_command(string(get(e.json(), "id")))
      }),
    ]),
    from_parts.children([
      part_target.children([muted(t, "Right-click: Menu.Item parts")]),
    ]),
    note(t, () => "rows: \{command()} · parts: \{parts()}"),
  ])
}

///|
fn system_context_menu_demo(t : Theme) -> @ui.Element {
  let window = @native.current_window()
  let (action, set_action) = @ui.create_signal("nothing yet")
  let (status, set_status) = @ui.create_signal("closed")
  let (hidden, set_hidden) = @ui.create_signal(false)
  let (sort, set_sort) = @ui.create_signal("Name")
  let open = () => {
    let items = ["Name", "Date modified", "Size"].map(name => {
      @native.menu_action(
        name,
        () => {
          set_sort(name)
          set_action("Sort by \{name}")
        },
        checked=@ui.untrack(sort) == name,
      )
    })
    set_status("open")
    window.popup_menu(
      [
        @native.menu_action("Open", () => set_action("Open")),
        @native.menu_action("Rename", () => set_action("Rename")),
        @native.Separator,
        @native.menu_action(
          "Show hidden files",
          () => {
            set_hidden(!@ui.untrack(hidden))
            set_action("Show hidden files \{@ui.untrack(hidden)}")
          },
          checked=@ui.untrack(hidden),
        ),
        @native.submenu("Sort by", items),
        @native.Separator,
        @native.menu_action("Unavailable action", () => (), enabled=false),
      ],
      done=result => {
        set_status(
          match result {
            Ok(_) => "closed"
            Err(error) => "error: " + error
          },
        )
      },
    )
  }
  panel(
    t, "System context menu", "The operating system draws this menu. Actions update the readout below; reopen it to see the current checkmarks.",
  ).children([
    framed(t, @ui.div([]))
    .children([muted(t, "Right-click here for the system menu")])
    .height(96)
    .display("flex")
    .items_center()
    .justify_center()
    .border_style("dashed")
    .app_region("no-drag")
    .user_select("none")
    .on("contextmenu", @protocol.CONTEXT_MENU_LISTENER, event => {
      event.prevent_default()
      open()
    }),
    row().children([button(t, "Open system menu", open)]),
    note(t, () => "menu \{status()} · last action \{action()}"),
    note(t, () => "hidden files \{hidden()} · sort by \{sort()}"),
  ])
}

///|
fn menu_demo(t : Theme) -> @ui.Element {
  let (open, set_open) = @ui.create_signal(false)
  let (wrap, set_wrap) = @ui.create_signal(false)
  let (density, set_density) = @ui.create_signal("cozy")
  let (activated, set_activated) = @ui.create_signal("nothing yet")
  let menu = @ui.menu([])
    .bind_open(() => open())
    .on_open_change(v => set_open(v))
    .side("bottom")
    .align("start")
    .side_offset(6)
  let rows = popup(t, menu.slot(@ui.Popup), 250).children([
    menu.slot(@ui.GroupLabel).label("Clipboard"),
  ])
  for entry in [("copy", "Copy"), ("cut", "Cut")] {
    ignore(
      rows.children([
        menu_row(t, menu.slot(@ui.Item, value=entry.0).label(entry.1)).on_click(() => {
            set_activated(entry.0)
          },
        ),
      ]),
    )
  }
  ignore(
    rows.children([
      menu_row(
        t,
        menu
        .slot(@ui.LinkItem, value="docs")
        .label("Documentation")
        .href("https://quickgui.dev"),
      ).on_click(() => set_activated("link https://quickgui.dev")),
    ]),
  )
  ignore(
    rows.children([
      menu
      .slot(@ui.Separator)
      .height(1)
      .bind_style(() => @ui.style().bg(t.color("border"))),
    ]),
  )
  let check = menu_row(
    t,
    menu
    .slot(@ui.CheckboxItem, value="wrap")
    .label("Wrap lines")
    .bind_checked(() => wrap()),
  ).on_click(() => set_wrap(!wrap()))
  ignore(
    rows.children([
      check.children([
        check
        .slot(@ui.CheckboxItemIndicator)
        .children([..if wrap() { [icon("check")] } else { [] }]),
      ]),
    ]),
  )
  let radio = menu
    .slot(@ui.RadioGroup)
    .group("density")
    .bind_active_value(() => density())
    .flex_col()
    .gap(2)
  for value in ["compact", "cozy", "comfortable"] {
    let item = menu_row(
      t,
      radio
      .slot(@ui.RadioItem, value~)
      .label(value)
      .group("density")
      .bind_checked(() => density() == value),
    ).on_click(() => set_density(value))
    ignore(
      radio.children([
        item.children([
          item
          .slot(@ui.RadioItemIndicator)
          .children([..if density() == value { [icon("check")] } else { [] }]),
        ]),
      ]),
    )
  }
  ignore(rows.children([radio]))
  let submenu = menu
    .slot(@ui.SubmenuRoot)
    .close_delay(100)
    .side("right")
    .align("start")
    .side_offset(4)
  let recent = popup(t, submenu.slot(@ui.Popup), 200)
  for value in ["notes.md", "README.md"] {
    ignore(
      recent.children([
        menu_row(t, submenu.slot(@ui.Item, value~).label(value)).on_click(() => {
          set_activated(value)
        }),
      ]),
    )
  }
  ignore(
    rows.children([
      submenu.children([
        menu_row(
          t,
          submenu
          .slot(@ui.SubmenuTrigger, value="recent")
          .label("Open recent")
          .open_on_hover(true),
        ).children([icon("right")]),
        submenu.slot(@ui.Positioner).children([recent]),
      ]),
    ]),
  )
  panel(
    t, "Menu", "Styled rows with keyboard navigation, checkbox state, radio exclusivity, and a nested submenu.",
  ).children([
    menu.children([
      control(t, menu.slot(@ui.Trigger).label("Edit")).children([
        icon("chevron"),
      ]),
      menu.slot(@ui.Positioner).children([rows]),
    ]),
    note(t, () => {
      "open \{open()} · activated \{activated()} · wrap \{wrap()} · density \{density()}"
    }),
  ])
}

///|
fn menubar_demo(t : Theme) -> @ui.Element {
  let (opened, set_opened) = @ui.create_signal(-1)
  let (active, set_active) = @ui.create_signal(0)
  let (command, set_command) = @ui.create_signal("nothing yet")
  let menus : Array[(String, Array[Json])] = [
    (
      "File",
      [
        { "id": "new", "label": "New window", "shortcut": "Cmd+N" },
        { "id": "open", "label": "Open…", "shortcut": "Cmd+O" },
        { "type": "separator" },
        { "id": "close", "label": "Close", "shortcut": "Cmd+W" },
      ],
    ),
    (
      "Edit",
      [
        { "id": "undo", "label": "Undo", "shortcut": "Cmd+Z" },
        { "id": "redo", "label": "Redo", "shortcut": "Cmd+Shift+Z" },
        { "id": "paste", "label": "Paste", "shortcut": "Cmd+V" },
      ],
    ),
    (
      "View",
      [
        { "id": "zoom-in", "label": "Zoom in" },
        { "id": "zoom-out", "label": "Zoom out" },
      ],
    ),
  ]
  let bar = @ui.menubar([])
    .menu_count(3)
    .flex_row()
    .gap(10)
    .bind(@protocol.OPEN, () => {
      if opened() < 0 {
        @protocol.Clear
      } else {
        @protocol.Number(Float::from_int(opened()))
      }
    })
    .on_change(data => {
      if get(data, "openIndex") is Number(n, ..) {
        set_opened(n.to_int())
      } else if data is { "openIndex": Null, .. } {
        set_opened(-1)
      }
      if get(data, "focusedIndex") is Number(n, ..) {
        set_active(n.to_int())
      }
    })
  for index, entry in menus {
    let menu = @ui.popover_menu([])
      .bind_menu(() => menu_model(t, entry.1))
      .bind_open(() => opened() == index)
      .on_open_change(v => set_opened(if v { index } else { -1 }))
    ignore(
      bar.children([
        menu
        .children([
          menu
          .slot(@ui.Trigger)
          .children([
            control(t, bar.slot(@ui.Item, index~).label(entry.0), active=() => {
              opened() == index
            }),
          ]),
        ])
        .when_open(() => {
          popup(t, menu.slot(@ui.Popup), 220)
          .padding(0)
          .on("menuselect", @protocol.SELECT_LISTENER, e => {
            set_command(string(get(e.json(), "id")))
            set_opened(-1)
          })
        }),
      ]),
    )
  }
  panel(
    t, "Menubar", "One roving Tab stop. Arrow keys switch menus while open; Escape closes without leaving the bar.",
  ).children([
    bar,
    note(t, () => {
      "open \{opened()} · tab stop \{active()} · command \{command()}"
    }),
  ])
}

///|
fn navigation_menu_demo(t : Theme) -> @ui.Element {
  let state : @reactive.Signal[Json] = @reactive.signal({})
  let (value, set_value) = @ui.create_signal("")
  let (navigate, set_navigate) = @ui.create_signal("")
  let nav = @ui.navigation_menu([])
    .bind_active_value(() => value())
    .delay(80)
    .close_delay(80)
    .items([
      { "value": "products" },
      { "value": "solutions" },
      { "value": "support", "disabled": true },
    ])
    .on_change(data => {
      if get(data, "value") is String(v) {
        set_value(v)
      }
      state.set(data)
    })
  let list = nav.slot(@ui.List).flex_row().gap(10)
  for name in ["products", "solutions", "support"] {
    let item = nav.slot(@ui.Item, value=name)
    let content = item.slot(@ui.Content).flex_col().gap(8)
    for page in ["overview", "pricing"] {
      ignore(
        content.children([
          menu_row(
            t,
            item
            .slot(@ui.Link, value="\{name}-\{page}")
            .label("\{name} \{page}"),
          ).on_click(() => set_navigate("\{name} \{page}")),
        ]),
      )
    }
    ignore(
      list.children([
        item.children([
          control(
            t,
            item.slot(@ui.Trigger).label(name).disabled(name == "support"),
            active=() => value() == name,
          ),
          item
          .slot(@ui.Positioner)
          .children([
            popup(t, item.slot(@ui.Popup), 230).children([
              item.slot(@ui.Viewport).children([content]),
            ]),
          ]),
        ]),
      ]),
    )
  }
  panel(
    t, "Navigation menu", "Hover opens a panel after the declared delay. The navigation landmark has one Tab stop and reports activation direction.",
  ).children([
    nav.children([list]),
    readout(t, state),
    note(t, () => "navigate \{navigate()}"),
  ])
}

///|
fn popover_demo(t : Theme) -> @ui.Element {
  let state : @reactive.Signal[Json] = @reactive.signal({})
  let hover : @reactive.Signal[Json] = @reactive.signal({})
  let click = observe(
    @ui.popover([])
    .side("bottom")
    .align("start")
    .side_offset(8)
    .collision_padding(12),
    state,
  )
  let delayed = observe(
    @ui.popover([])
    .open_on_hover(true)
    .delay(200)
    .close_delay(120)
    .side("right")
    .align("center")
    .side_offset(8),
    hover,
  )
  panel(
    t, "Popover", "Preferred placement adapts to available space. Compare click and delayed hover triggers.",
  ).children([
    click.children([
      control(t, click.slot(@ui.Trigger).label("Account")),
      click
      .slot(@ui.Positioner)
      .children([
        popup(t, click.slot(@ui.Popup), 240).children([
          click
          .slot(@ui.Arrow)
          .width(10)
          .height(10)
          .bind_style(() => @ui.style().bg(t.color("popup"))),
          click.slot(@ui.Title).label("Signed in"),
          click.slot(@ui.Viewport).label("ada@example.com"),
          control(t, click.slot(@ui.Close).label("Done")),
        ]),
      ]),
    ]),
    delayed.children([
      control(t, delayed.slot(@ui.Trigger).label("Hover to open")),
      delayed
      .slot(@ui.Positioner)
      .children([
        popup(t, delayed.slot(@ui.Popup), 200).label(
          "Opened after the hover delay.",
        ),
      ]),
    ]),
    readout(t, state),
    readout(t, hover),
  ])
}

///|
fn preview_card_demo(t : Theme) -> @ui.Element {
  let state : @reactive.Signal[Json] = @reactive.signal({})
  let card = observe(
    @ui.preview_card([])
    .side("bottom")
    .align("start")
    .side_offset(8)
    .delay(350)
    .close_delay(200),
    state,
  )
  panel(
    t, "Preview card", "Rest the pointer to preview. Keyboard focus opens immediately.",
  ).children([
    row().children([
      muted(t, "Written by"),
      card.children([
        card
        .slot(@ui.Trigger)
        .label("@ada")
        .padding(2)
        .text_decoration_line("underline")
        .bind_style(() => @ui.style().text_color(t.color("accent"))),
        card
        .slot(@ui.Positioner)
        .children([
          popup(t, card.slot(@ui.Popup), 240).children([
            label(t, "Ada Lovelace"),
            muted(t, "Wrote the first algorithm intended for a machine."),
          ]),
        ]),
      ]),
    ]),
    readout(t, state),
  ])
}

///|
fn tooltip_demo(t : Theme) -> @ui.Element {
  let state : @reactive.Signal[Json] = @reactive.signal({})
  let provider = @ui.tooltip_provider([])
    .delay(500)
    .close_delay(120)
    .timeout(400)
  let scope = string_property(provider, @protocol.SCOPE)
  let triggers = row()
  for index, title in ["Hover me", "Tracks the cursor"] {
    let tooltip = observe(
      @ui.tooltip([])
      .side(if index == 0 { "top" } else { "bottom" })
      .side_offset(8)
      .property(@protocol.PROVIDER, @protocol.Text(scope))
      .track_cursor_axis(if index == 1 { "x" } else { "" }),
      state,
    )
    let tip = tooltip
      .slot(@ui.Popup)
      .padding(8)
      .border_radius(8)
      .font_size(11)
      .bind_style(() => {
        @ui.style().bg(t.color("ink")).text_color(t.color("panel"))
      })
      .label(
        if index == 0 {
          "Shared hover delay"
        } else {
          "Tracks the horizontal cursor"
        },
      )
    ignore(
      triggers.children([
        tooltip.children([
          control(t, tooltip.slot(@ui.Trigger).label(title)),
          tooltip.slot(@ui.Positioner).children([tip]),
        ]),
      ]),
    )
  }
  panel(
    t, "Tooltip", "A provider shares a warm delay across triggers. The second tooltip tracks the horizontal cursor.",
  ).children([provider.children([triggers]), readout(t, state)])
}

///|
fn string_property(node : @ui.Element, code : Int) -> String {
  match node.native_node().property(code) {
    @protocol.Text(value) => value
    _ => ""
  }
}

///|
struct Notification {
  id : String
  title : String
  kind : String
} derive(Eq)

///|
fn toast_demo(t : Theme) -> @ui.Element {
  let counter = @ref.Ref(0)
  let (items, set_items) = @ui.create_signal(([] : Array[Notification]))
  let state : @reactive.Signal[Json] = @reactive.signal({})
  let viewport = @ui.toast_viewport([])
    .timeout(5000)
    .limit(3)
    .pitch(8)
    .swipe_direction("right")
    .bind_toasts(() => {
      items()
      .map(n => {
        (
          {
            "id": n.id,
            "title": n.title,
            "description": "Auto-dismisses in 5 seconds",
            "type": n.kind,
            "duration": 5000,
          } : Json)
      })
      .to_json()
    })
    .on_change(data => {
      let dismissed = strings(get(data, "dismissed"))
      if !dismissed.is_empty() {
        set_items(items().filter(n => !dismissed.contains(n.id)))
      }
      state.set(data)
    })
    .flex_col()
    .gap(8)
    .min_height(40)
  ignore(
    viewport.children_keyed(() => items(), n => n.id, entry => {
      let id = entry.peek().id
      let geometry = () => {
        match get(state.get(), "toasts") {
          Array(entries) =>
            entries
            .iter()
            .find_first(e => get(e, "id") == id.to_json())
            .unwrap_or(Json::null())
          _ => Json::null()
        }
      }
      let toast = popup(t, viewport.slot(@ui.Item, value=id), 420)
        .padding(10)
        .bind_style(() => {
          @ui.style()
          .bg(t.color("popup"))
          .border_color(
            t.color(
              match entry.get().kind {
                "error" => "danger"
                "success" => "accent"
                _ => "border"
              },
            ),
          )
          .opacity(
            if get(geometry(), "limited") == Json::boolean(true) {
              0.55
            } else {
              1.0
            },
          )
          .transform(
            "translateX(\{number(get(geometry(), "swipeMovement"))}px)",
          )
        })
      viewport
      .slot(@ui.Positioner, value=id)
      .children([
        toast.children([
          viewport
          .slot(@ui.Content, value=id)
          .children([
            row().children([
              viewport
              .slot(@ui.Title, value=id)
              .bind_label(() => entry.get().title),
              note(t, () => {
                "#\{number(get(geometry(), "index"))} · +\{number(get(geometry(), "offset"))}px"
              }),
              control(
                t,
                viewport
                .slot(@ui.Close, value=id)
                .children([icon("close")])
                .aria("Dismiss notification"),
              ),
            ]),
            viewport
            .slot(@ui.Description, value=id)
            .children([muted(t, "Auto-dismisses in 5 seconds")]),
          ])
          .flex_col()
          .gap(8),
        ]),
      ])
    }),
  )
  let controls = row()
  for kind in ["info", "success", "error"] {
    ignore(
      controls.children([
        button(t, kind, () => {
          counter.val += 1
          let n = Notification::{
            id: counter.val.to_string(),
            title: "Build \{counter.val} · \{kind}",
            kind,
          }
          set_items([..@ui.untrack(items), n])
        }),
      ]),
    )
  }
  ignore(
    controls.children([
      button(t, "Update last", () => {
        let current = items()
        if current.is_empty() {
          return
        }
        let copy = current.copy()
        let last = copy[copy.length() - 1]
        copy[copy.length() - 1] = { ..last, title: "Updated in place", }
        set_items(copy)
      }),
      button(t, "Clear", () => set_items([])),
    ]),
  )
  panel(
    t, "Toast", "Info, success, and error notifications with update, close, swipe, and automatic dismissal. Deadlines stay in the core.",
  ).children([
    controls,
    viewport,
    note(t, () => "queued \{items().length()}"),
    readout(t, state),
  ])
}
