///|
test "checkbox defaults toggle twice, preserve siblings and cancel user actions" {
  @native.test_tree(root => {
    let box = @ui.checkbox([])
    let indicator = box.slot(@ui.Indicator)
    ignore(box.child(indicator))
    let sibling = @ui.text("Unchanged")
    root.append(box.native_node())
    root.append(sibling.native_node())
    ignore(root.take_batch())
    box.native_node().emit("click", {})
    assert_true(
      box.native_node().property(@protocol.CHECKED) == @protocol.Boolean(true),
    )
    assert_true(root.pending_mutations() == 1)
    ignore(root.take_batch())
    box.native_node().emit("click", {})
    assert_true(
      box.native_node().property(@protocol.CHECKED) == @protocol.Boolean(false),
    )
    assert_true(root.pending_mutations() == 1)
    ignore(
      box.on("click", @protocol.CLICK_LISTENER, event => event.prevent_default()),
    )
    box.native_node().emit("click", {})
    assert_true(
      box.native_node().property(@protocol.CHECKED) == @protocol.Boolean(false),
    )
    assert_true(box.native_node().children()[0] == indicator.native_node())
  })
}

///|
test "read only, disabled, mixed and controlled checkbox contracts" {
  @native.test_tree(root => {
    let read_only_box = @ui.checkbox([]).read_only(true)
    root.append(read_only_box.native_node())
    read_only_box.native_node().emit("click", {})
    assert_true(
      read_only_box.native_node().property(@protocol.CHECKED) ==
      @protocol.Boolean(false),
    )
    let mixed = @ui.checkbox([]).indeterminate(true)
    root.append(mixed.native_node())
    mixed.native_node().emit("click", {})
    assert_true(
      mixed.native_node().property(@protocol.INDETERMINATE) ==
      @protocol.Boolean(false),
    )
    assert_true(
      mixed.native_node().property(@protocol.CHECKED) == @protocol.Boolean(true),
    )
    let checked = @reactive.signal(false)
    let controlled = @ui.checkbox([]).bind_checked(() => checked.get())
    root.append(controlled.native_node())
    controlled.native_node().emit("click", {})
    assert_true(!checked.get())
    assert_true(
      controlled.native_node().property(@protocol.CHECKED) ==
      @protocol.Boolean(false),
    )
    ignore(controlled.on_checked_change(value => checked.set(value)))
    controlled.native_node().emit("click", {})
    assert_true(checked.get())
    ignore(controlled.disabled(true))
    controlled.native_node().emit("click", {})
    assert_true(checked.get())
  })
}

///|
test "tabs propagate selection to mounted parts without replacing them" {
  @native.test_tree(root => {
    let tabs = @ui.tabs([]).active_value("one")
    let first = tabs.slot(@ui.Tab, value="one")
    let second = tabs.slot(@ui.Tab, value="two")
    let panel = tabs.slot(@ui.Panel, value="two")
    ignore(tabs.children([first, second, panel]))
    root.append(tabs.native_node())
    second.native_node().emit("click", {})
    assert_true(
      first.native_node().property(@protocol.ACTIVE_VALUE) ==
      @protocol.Text("two"),
    )
    assert_true(
      panel.native_node().property(@protocol.ACTIVE_VALUE) ==
      @protocol.Text("two"),
    )
    assert_true(tabs.native_node().children()[2] == panel.native_node())
    let other = @ui.tabs([]).active_value("elsewhere")
    assert_true(
      tabs.native_node().property(@protocol.SCOPE) !=
      other.native_node().property(@protocol.SCOPE),
    )
    assert_true(
      other.native_node().property(@protocol.ACTIVE_VALUE) ==
      @protocol.Text("elsewhere"),
    )
  })
}

///|
test "accordion parts retain item identity and derived open state" {
  @native.test_tree(root => {
    let accordion = @ui.accordion([])
    let item = accordion.slot(@ui.Item, value="details", index=2)
    let trigger = item.slot(@ui.Trigger)
    let panel = item.slot(@ui.Panel)
    root.append(accordion.child(item.children([trigger, panel])).native_node())
    trigger.native_node().emit("click", {})
    assert_true(
      panel.native_node().property(@protocol.OPEN) == @protocol.Boolean(true),
    )
    assert_true(
      panel.native_node().property(@protocol.PART_VALUE) ==
      @protocol.Text("details"),
    )
    assert_true(
      trigger.native_node().property(@protocol.ITEM_INDEX) ==
      @protocol.Number(2),
    )
    trigger.native_node().emit("click", {})
    assert_true(
      panel.native_node().property(@protocol.OPEN) == @protocol.Boolean(false),
    )
  })
}

///|
test "native slider and picker events persist uncontrolled values" {
  @native.test_tree(root => {
    let slider = @ui.slider([])
    let thumb = slider.slot(@ui.Thumb)
    root.append(slider.child(thumb).native_node())
    slider
    .native_node()
    .emit("componentchange", { "values": [42], "dragging": false })
    assert_true(
      slider.native_node().property(@protocol.VALUES) == @protocol.Text("[42]"),
    )
    assert_true(
      thumb.native_node().property(@protocol.VALUES) == @protocol.Clear,
    )
    let select = @ui.select([])
    root.append(select.native_node())
    select.native_node().emit("componentchange", { "value": "go" })
    assert_true(
      select.native_node().property(@protocol.ACTIVE_VALUE) ==
      @protocol.Text("go"),
    )
    select.native_node().emit("componentchange", { "value": Json::null() })
    assert_true(
      select.native_node().property(@protocol.ACTIVE_VALUE) == @protocol.Clear,
    )
  })
}

///|
test "nested controls keep events scoped and dispose all subscriptions" {
  let scopes = @reactive.active_scope_count()
  let effects = @reactive.active_effect_count()
  @native.test_tree(root => {
    let checked = @reactive.signal(false)
    let outer = @ui.checkbox([])
    let inner = @ui.checkbox([]).bind_checked(() => checked.get())
    root.append(outer.child(inner).native_node())
    inner.native_node().emit("componentchange", { "checked": true })
    assert_true(
      outer.native_node().property(@protocol.CHECKED) ==
      @protocol.Boolean(false),
    )
    outer.remove()
    ignore(root.take_batch())
    checked.set(true)
    assert_true(root.pending_mutations() == 0)
  })
  assert_true(@reactive.active_scope_count() == scopes)
  assert_true(@reactive.active_effect_count() == effects)
}

///|
test "click handlers may remove a component before its default action" {
  @native.test_tree(root => {
    let box = @ui.checkbox([])
    root.append(box.native_node())
    ignore(box.on_click(() => box.remove()))
    box.native_node().emit("click", {})
    assert_true(root.children().is_empty())
  })
}

///|
test "menu labels remain native declaration and typeahead labels" {
  @native.test_tree(root => {
    let menu = @ui.menu([])
    let label = @reactive.signal("Copy")
    let item = menu.slot(@ui.Item, value="copy").bind_label(() => label.get())
    root.append(menu.child(item).native_node())
    assert_true(
      item.native_node().property(@protocol.ACCESSIBILITY_LABEL) ==
      @protocol.Text("Copy"),
    )
    label.set("Copy path")
    assert_true(
      item.native_node().property(@protocol.ACCESSIBILITY_LABEL) ==
      @protocol.Text("Copy path"),
    )
  })
}

///|
test "dialog and menu parts expose native state without accidental anchored surfaces" {
  @native.test_tree(root => {
    let dialog = @ui.dialog([])
    let trigger = dialog.slot(@ui.Trigger)
    let portal = dialog.slot(@ui.Portal)
    let popup = dialog.slot(@ui.Popup)
    root.append(dialog.child(trigger).child(portal.child(popup)).native_node())
    trigger.native_node().emit("click", {})
    assert_true(
      popup.native_node().property(@protocol.OPEN) == @protocol.Boolean(true),
    )
    for node in [dialog, trigger, portal, popup] {
      assert_true(
        node.native_node().property(@protocol.ANCHOR_TARGET) == @protocol.Clear,
      )
    }
    assert_true(
      dialog.native_node().property(@protocol.DISMISS_ON_ESCAPE) ==
      @protocol.Clear,
    )
    assert_true(
      trigger.native_node().property(@protocol.DISMISS_ON_ESCAPE) ==
      @protocol.Clear,
    )
    assert_true(
      popup.native_node().property(@protocol.DISMISS_ON_ESCAPE) ==
      @protocol.Boolean(true),
    )
    let menu = @ui.menu([])
    let menu_trigger = menu.slot(@ui.Trigger)
    root.append(menu.child(menu_trigger).native_node())
    assert_true(
      menu_trigger.native_node().property(@protocol.COMPONENT_CHANGE_LISTENER) ==
      @protocol.Boolean(true),
    )
    menu_trigger.native_node().emit("componentchange", { "open": true })
    assert_true(
      menu.native_node().property(@protocol.OPEN) == @protocol.Boolean(true),
    )
  })
}

///|
test "picker items populate the native option source and reactive updates replace it" {
  @native.test_tree(root => {
    let options : @reactive.Signal[Json] = @reactive.signal([
      { "value": "moonbit", "label": "MoonBit" },
    ])
    for
      make in [
        () => @ui.select([]),
        () => @ui.combobox([]),
        () => @ui.autocomplete([]),
      ] {
      let picker = make().bind_items(() => options.get())
      root.append(picker.native_node())
      @reactive.flush()
      let actual = match picker.native_node().property(@protocol.OPTIONS) {
        Text(text) => text
        _ => "missing options"
      }
      assert_eq(actual, options.peek().stringify())
      options.set([{ "value": "go", "label": "Go" }])
      let actual = match picker.native_node().property(@protocol.OPTIONS) {
        Text(text) => text
        _ => "missing options"
      }
      assert_eq(actual, options.peek().stringify())
      options.set([{ "value": "moonbit", "label": "MoonBit" }])
    }
  })
}

///|
test "a controlled model menu has an anchor before its first trigger click" {
  @native.test_tree(root => {
    let menu = @ui.popover_menu([]).open(true)
    let trigger = menu.slot(@ui.Trigger)
    let popup = menu.slot(@ui.Popup)
    root.append(menu.children([trigger, popup]).native_node())
    @reactive.flush()
    assert_true(
      popup.native_node().property(@protocol.ANCHOR_TARGET) ==
      @protocol.Text(trigger.native_node().id.to_string()),
    )
    assert_true(
      trigger.native_node().property(@protocol.ANCHOR_TARGET) == @protocol.Clear,
    )
  })
}
