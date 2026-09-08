///|
priv struct Demo {
  id : String
  title : String
  source : String
  render : (Theme) -> @ui.Element
}

///|
let demos : Array[Demo] = [
  { id: "accordion", title: "Accordion", source: "", render: accordion_demo, },
  {
    id: "alert-dialog",
    title: "Alert Dialog",
    source: "",
    render: alert_dialog_demo,
  },
  {
    id: "autocomplete",
    title: "Autocomplete",
    source: "",
    render: autocomplete_demo,
  },
  { id: "avatar", title: "Avatar", source: "", render: avatar_demo, },
  { id: "button", title: "Button", source: "", render: button_demo, },
  {
    id: "calendar",
    title: "Calendar",
    source: "QuickGUI",
    render: calendar_demo,
  },
  { id: "checkbox", title: "Checkbox", source: "", render: checkbox_demo, },
  {
    id: "checkbox-group",
    title: "Checkbox Group",
    source: "",
    render: checkbox_group_demo,
  },
  {
    id: "collapsible",
    title: "Collapsible",
    source: "",
    render: collapsible_demo,
  },
  { id: "combobox", title: "Combobox", source: "", render: combobox_demo, },
  {
    id: "context-menu",
    title: "Context Menu",
    source: "",
    render: context_menu_demo,
  },
  {
    id: "system-context-menu",
    title: "System Context Menu",
    source: "System",
    render: system_context_menu_demo,
  },
  {
    id: "date-field",
    title: "Date Field",
    source: "QuickGUI",
    render: date_field_demo,
  },
  { id: "dialog", title: "Dialog", source: "", render: dialog_demo, },
  { id: "field", title: "Field", source: "", render: field_demo, },
  { id: "fieldset", title: "Fieldset", source: "", render: fieldset_demo, },
  { id: "form", title: "Form", source: "", render: form_demo, },
  { id: "input", title: "Input", source: "", render: input_demo, },
  { id: "menu", title: "Menu", source: "", render: menu_demo, },
  { id: "menubar", title: "Menubar", source: "", render: menubar_demo, },
  { id: "meter", title: "Meter", source: "", render: meter_demo, },
  {
    id: "navigation-menu",
    title: "Navigation Menu",
    source: "",
    render: navigation_menu_demo,
  },
  {
    id: "number-field",
    title: "Number Field",
    source: "",
    render: number_field_demo,
  },
  { id: "otp-field", title: "OTP Field", source: "", render: otp_field_demo, },
  { id: "popover", title: "Popover", source: "", render: popover_demo, },
  {
    id: "preview-card",
    title: "Preview Card",
    source: "",
    render: preview_card_demo,
  },
  { id: "progress", title: "Progress", source: "", render: progress_demo, },
  { id: "radio", title: "Radio", source: "", render: radio_demo, },
  {
    id: "scroll-area",
    title: "Scroll Area",
    source: "",
    render: scroll_area_demo,
  },
  { id: "select", title: "Select", source: "", render: select_demo, },
  { id: "separator", title: "Separator", source: "", render: separator_demo, },
  { id: "slider", title: "Slider", source: "", render: slider_demo, },
  {
    id: "splitter",
    title: "Splitter",
    source: "QuickGUI",
    render: splitter_demo,
  },
  { id: "switch", title: "Switch", source: "", render: switch_demo, },
  { id: "tabs", title: "Tabs", source: "", render: tabs_demo, },
  {
    id: "time-field",
    title: "Time Field",
    source: "QuickGUI",
    render: time_field_demo,
  },
  { id: "toast", title: "Toast", source: "", render: toast_demo, },
  { id: "toggle", title: "Toggle", source: "", render: toggle_demo, },
  {
    id: "toggle-group",
    title: "Toggle Group",
    source: "",
    render: toggle_group_demo,
  },
  { id: "toolbar", title: "Toolbar", source: "", render: toolbar_demo, },
  { id: "tooltip", title: "Tooltip", source: "", render: tooltip_demo, },
  { id: "table", title: "Table", source: "QuickGUI", render: table_demo, },
  { id: "tree", title: "Tree", source: "QuickGUI", render: tree_demo, },
]

///|
fn gallery(
  t : Theme,
  selected : @reactive.Signal[String],
  size : @reactive.Signal[String],
) -> @ui.Element {
  let current = () => {
    demos.iter().find_first(d => d.id == selected.get()).unwrap_or(demos[0])
  }
  let tabs = @ui.tabs([])
    .orientation("vertical")
    .activate_on_focus(false)
    .bind_active_value(() => selected.get())
    .on_value_change(value => selected.set(value.unwrap_or("accordion")))
    .size_full()
    .flex_row()
    .bind_style(() => {
      @ui.style().bg(t.color("window")).text_color(t.color("ink"))
    })
  let sidebar = tabs
    .slot(@ui.List)
    .width("100%")
    .flex_col()
    .gap(1)
    .px(8)
    .padding_bottom(12)
  for index, demo in demos {
    ignore(
      sidebar.children([
        tabs
        .slot(@ui.Tab, value=demo.id, index~)
        .width("100%")
        .label(demo.title)
        .height(28)
        .flex_shrink(0)
        .flex_row()
        .items_center()
        .px(10)
        .border_radius(7)
        .font_size(12)
        .cursor("default")
        .user_select("none")
        .app_region("no-drag")
        .bind_style(() => {
          @ui.style()
          .bg(
            t.color(
              if selected.get() == demo.id {
                "accent"
              } else {
                "transparent"
              },
            ),
          )
          .text_color(
            t.color(if selected.get() == demo.id { "on-accent" } else { "ink" }),
          )
          .font_weight(if selected.get() == demo.id { 600 } else { 400 })
        })
        .bind_json(@protocol.HOVER_STYLE, () => {
          @ui.style()
          .bg(
            t.color(if selected.get() == demo.id { "accent" } else { "hover" }),
          )
          .json()
        })
        .bind_json(@protocol.FOCUS_STYLE, () => focus_ring(t)),
      ]),
    )
  }
  // Each panel owns its effects and native nodes. Changing the sidebar retires
  // the old demo, including popups, and constructs only the new one.
  let body = @ui.div([])
    .flex_col()
    .flex_1()
    .min_height(0)
    .padding(20)
    .gap(16)
    .overflow_y("scroll")
  for demo in demos {
    ignore(
      body.children(
        [
          ..if selected.get() == demo.id {
            [
              tabs
              .slot(@ui.Panel, value=demo.id)
              .children([(demo.render)(t)])
              .flex_col()
              .gap(16)
              .max_width(720)
              .flex_shrink(0),
            ]
          } else {
            []
          },
        ],
      ),
    )
  }
  tabs.children([
    @ui.div([
      @ui.text("Components")
      .height(52)
      .flex_shrink(0)
      .display("flex")
      .items_center()
      .padding_left(82)
      .font_size(13)
      .font_weight(700)
      .app_region("drag"),
      @ui.div([sidebar]).flex_1().min_height(0).overflow_y("scroll"),
    ])
    .width(214)
    .height("100%")
    .flex_shrink(0)
    .flex_col()
    .border_right_width(1)
    .bind_style(() => {
      @ui.style().bg(t.color("sidebar")).border_color(t.color("border"))
    }),
    @ui.div([
      @ui.div([
        col()
        .children([
          @ui.text(current().title).font_size(15).font_weight(700),
          note(t, () => {
            match current().source {
              "QuickGUI" => "QuickGUI component"
              "System" => "Native system menu"
              _ => "Base UI part set"
            }
          })
          .font_family("system-ui")
          .font_size(11),
        ])
        .gap(2),
        note(t, () => {
          "43 components · \{t.appearance.get()} appearance · \{size.get()}"
        })
        .font_family("system-ui")
        .font_size(11),
      ])
      .flex_row()
      .items_center()
      .justify_content("space-between")
      .height(52)
      .flex_shrink(0)
      .px(20)
      .border_bottom_width(1)
      .app_region("drag")
      .bind_style(() => @ui.style().border_color(t.color("border"))),
      body,
    ])
    .flex_col()
    .flex_1()
    .min_width(0)
    .height("100%"),
  ])
}

///|
fn window_options(show? : Bool = true) -> Json {
  {
    "title": "QuickGUI Components · MoonBit",
    "width": 1080,
    "height": 780,
    "minimumWidth": 880,
    "minimumHeight": 620,
    "background": @ui.rgb8(244, 245, 247),
    "titleBarStyle": "hiddenInset",
    "trafficLightX": 16,
    "trafficLightY": 18,
    "show": show,
    "focus": show,
  }
}

///|
fn open_window() -> Unit {
  ignore(
    @ui.window(
      () => {
        let window = @native.current_window()
        let t = theme()
        let selected = @reactive.signal("accordion")
        let size = @reactive.signal("1080×780")
        let refresh = () => {
          @native.command(
            { "method": "get-window-state", "window": window.id },
            result => {
              if result is Ok(value) {
                @ui.batch(() => {
                  t.appearance.set(
                    string(get(value, "appearance"), fallback="light"),
                  )
                  if value
                    is {
                      "viewportWidth": Number(w, ..),
                      "viewportHeight": Number(h, ..),
                      ..
                    } {
                    size.set("\{w.to_int()}×\{h.to_int()}")
                  }
                })
              }
            },
          )
        }
        @native.on_event("window-ready-to-show", _ => refresh())
        @native.on_event("window-resize", _ => refresh())
        @native.on_event("window-appearance", e => {
          t.appearance.set(e.value.unwrap_or("light"))
        })
        gallery(t, selected, size)
      },
      options=window_options(),
    ),
  )
}

///|
pub fn run() -> Unit {
  match
    @native.run(() => {
      open_window()
      @native.on_event("reopen", _ => open_window(), window=0)
    }) {
    Ok(_) => ()
    Err(error) => abort(error)
  }
}
