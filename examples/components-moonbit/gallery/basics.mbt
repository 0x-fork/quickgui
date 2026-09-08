///|
fn accordion_demo(t : Theme) -> @ui.Element {
  let (open, set_open) = @reactive.create_signal(["declared"])
  let root = @ui.accordion([])
    .multiple(true)
    .heading_level(3)
    .bind_values(() => open().to_json())
    .on_change(data => {
      if get(data, "values") is Array(_) {
        set_open(strings(get(data, "values")))
      }
    })
    .flex_col()
    .gap(8)
  let entries = [
    (
      "declared", "Everything is declared", "Declare values and elements in MoonBit. The native core owns roles, focus, and deadlines.",
    ),
    (
      "panels", "Closed panels are not mounted", "A closed Accordion.Panel contributes no layout, paint, input, or accessibility node.",
    ),
    (
      "heading", "Heading level belongs to the core", "HeadingLevel is clamped to 1 through 6 and projected on every header.",
    ),
  ]
  for index, entry in entries {
    let item = framed(t, root.slot(@ui.Item, value=entry.0, index~))
      .flex_col()
      .overflow("hidden")
    let trigger = control(t, item.slot(@ui.Trigger))
      .children([
        label(t, entry.1),
        @ui.div(
          [
            ..if open().contains(entry.0) {
              [icon("minus")]
            } else {
              [icon("plus")]
            },
          ],
        ),
      ])
      .width("100%")
      .justify_content("space-between")
      .border_width(0)
      .height(34)
    ignore(
      root.children([
        item.children(
          [
            item
            .slot(@ui.Header)
            .children([trigger])
            .display("flex")
            .width("100%"),
            ..if open().contains(entry.0) {
              [
                item
                .slot(@ui.Panel)
                .children([muted(t, entry.2)])
                .padding(12)
                .padding_top(4),
              ]
            } else {
              []
            },
          ],
        ),
      ]),
    )
  }
  panel(
    t, "Accordion", "Multiple open items, semantic headings, and lazy panels. Click a header or use the arrow keys.",
  ).children([root, note(t, () => "open: " + open().join(", "))])
}

///|
fn button_demo(t : Theme) -> @ui.Element {
  let (clicks, set_clicks) = @reactive.create_signal(0)
  let (last, set_last) = @reactive.create_signal("nothing yet")
  panel(
    t, "Button", "Native press, focus, disabled state, and double-click handling.",
  ).children([
    row().children([
      button(
        t,
        "Primary",
        () => {
          set_clicks(@reactive.untrack(clicks) + 1)
          set_last("primary")
        },
        primary=true,
      ),
      button(t, "Secondary", () => {
        set_clicks(@reactive.untrack(clicks) + 1)
        set_last("secondary")
      }),
      button(t, "Disabled", () => set_last("never")).disabled(true),
      button(t, "Double-click me", () => set_last("single click")).on_double_click(() => {
          set_last("double click")
        },
      ),
    ]),
    note(t, () => "clicks \{clicks()} · last \{last()}"),
  ])
}

///|
fn checkbox_demo(t : Theme) -> @ui.Element {
  let (notify, set_notify) = @reactive.create_signal(false)
  let (mixed, set_mixed) = @reactive.create_signal(true)
  let (kids, set_kids) = @reactive.create_signal([true, false])
  let children = row()
  for i, name in ["Analytics", "Crash reports"] {
    ignore(
      children.children([
        checkbox(t, name, () => kids()[i], value => {
          let next = @reactive.untrack(kids).copy()
          next[i] = value
          set_kids(next)
        }),
      ]),
    )
  }
  panel(
    t, "Checkbox", "Tri-state, read-only, and a parent whose mixed state is derived from its children.",
  ).children([
    checkbox(
      t,
      "Email me about releases",
      () => notify(),
      value => {
        @reactive.batch(() => {
          set_notify(value)
          set_mixed(false)
        })
      },
      mixed=() => mixed(),
    ),
    checkbox(
      t,
      "Read-only: focusable, refuses changes",
      () => true,
      _ => (),
      read_only=true,
    ),
    checkbox(
      t,
      "Parent, derived from its children",
      () => kids().all(v => v),
      value => set_kids([value, value]),
      mixed=() => kids().any(v => v) && !kids().all(v => v),
    ),
    children,
    note(t, () => {
      "notify \{if mixed() { "indeterminate" } else { notify().to_string() }} · children \{kids().to_json().stringify()}"
    }),
  ])
}

///|
fn checkbox_group_demo(t : Theme) -> @ui.Element {
  let (selected, set_selected) = @reactive.create_signal(["green"])
  let all = ["red", "green", "blue", "violet"]
  let root = @ui.checkbox_group([])
    .items(all.to_json())
    .bind_values(() => selected().to_json())
    .on_change(data => {
      if get(data, "checkedValues") is Array(_) {
        set_selected(strings(get(data, "checkedValues")))
      }
    })
    .flex_col()
    .gap(8)
  let parent = root
    .slot(@ui.Parent)
    .children([
      check_indicator(
        t,
        root.slot(@ui.Indicator),
        () => selected().length() == all.length(),
        mixed=() => {
          !selected().is_empty() && selected().length() != all.length()
        },
      ),
      label(t, "All colours"),
    ])
    .flex_row()
    .items_center()
    .gap(8)
    .height(28)
  ignore(root.children([parent]))
  for value in all {
    let item = root
      .slot(@ui.Item, value~)
      .flex_row()
      .items_center()
      .gap(8)
      .height(28)
    ignore(
      root.children([
        item.children([
          check_indicator(t, item.slot(@ui.Indicator), () => {
            selected().contains(value)
          }),
          label(t, value),
        ]),
      ]),
    )
  }
  panel(
    t, "Checkbox group", "The parent toggles all values and derives its mixed state from the group.",
  ).children([root, note(t, () => "checked: " + selected().join(", "))])
}

///|
fn collapsible_demo(t : Theme) -> @ui.Element {
  let (open, set_open) = @reactive.create_signal(false)
  let (kept, set_kept) = @reactive.create_signal(true)
  let root = @ui.collapsible([])
    .bind_open(() => open())
    .on_open_change(v => set_open(v))
    .flex_col()
    .gap(8)
  let retained = @ui.collapsible([])
    .bind_open(() => kept())
    .on_open_change(v => set_kept(v))
    .keep_mounted(true)
    .flex_col()
    .gap(8)
  panel(
    t, "Collapsible", "Closed panels are omitted; KeepMounted instead retains the panel with display:none.",
  ).children([
    root.children(
      [
        control(
          t,
          root
          .slot(@ui.Trigger)
          .bind_label(() => {
            if open() {
              "Hide advanced options"
            } else {
              "Show advanced options"
            }
          }),
        ),
        ..if open() {
          [
            root
            .slot(@ui.Panel)
            .children([
              muted(
                t, "While closed, this panel contributes no layout, paint, input, or accessibility node.",
              ),
            ]),
          ]
        } else {
          []
        },
      ],
    ),
    retained.children([
      control(t, retained.slot(@ui.Trigger).label("Toggle kept panel")),
      retained
      .slot(@ui.Panel)
      .label("Retained as display:none instead of omitted."),
    ]),
    note(t, () => "plain \{open()} · keepMounted \{kept()}"),
  ])
}

///|
fn avatar_demo(t : Theme) -> @ui.Element {
  let missing : @reactive.Signal[Json] = @reactive.signal({})
  let fallback : @reactive.Signal[Json] = @reactive.signal({})
  let avatars = row()
  for
    entry in [
      ("Ada Lovelace", "AL", true, missing),
      ("Grace Hopper", "GH", false, fallback),
    ] {
    let (name, initials, broken, state) = entry
    let avatar = observe(@ui.avatar([]), state)
      .aria(name)
      .width(44)
      .height(44)
      .border_radius(22)
      .display("flex")
      .items_center()
      .justify_center()
      .border_width(1)
      .overflow("hidden")
      .bind_style(() => {
        @ui.style().bg(t.color("selection")).border_color(t.color("border"))
      })
    if broken {
      ignore(
        avatar.children([
          avatar
          .slot(@ui.Image)
          .value("./avatar-does-not-exist.png")
          .width(44)
          .height(44),
        ]),
      )
    }
    ignore(
      avatars.children([
        avatar.children([
          avatar
          .slot(@ui.Fallback)
          .delay(if broken { 120 } else { 0 })
          .label(initials),
        ]),
      ]),
    )
  }
  panel(
    t, "Avatar", "Images and fallbacks share one accessible name. Failed loads report their actual status.",
  ).children([avatars, readout(t, missing), readout(t, fallback)])
}
