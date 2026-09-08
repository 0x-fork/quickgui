///|
test "child lists keep node identity and update only the bound text" {
  let baseline = @reactive.active_scope_count()
  @native.test_tree(fn(root) raise {
    let count = @reactive.signal(0)
    let builds = @ref.Ref(0)
    fn counter() -> @ui.Element {
      builds.val += 1
      @ui.div([
        @ui.div([]).bind_content(() => "Count: \{count.get()}"),
        @ui.button("Increment").on_click(() => count.update(n => n + 1)),
        @ui.text("Unchanged"),
      ]).style(@ui.style().gap(12))
    }
    let view = counter()
    root.append(view.native_node())
    let children = view.native_node().children()
    let label = children[0].children()[0]
    ignore(root.take_batch())
    // A callback must restore its window even while another window is active.
    @native.test_tree(fn(other) raise {
      children[1].emit("click", {})
      assert_eq(other.pending_mutations(), 0)
    })
    assert_eq(builds.val, 1)
    assert_eq(label.text_content(), "Count: 1")
    assert_eq(root.pending_mutations(), 1)
    for index, child in view.native_node().children() {
      assert_true(child == children[index])
    }
    ignore(root.take_batch())
    count.set(1)
    assert_eq(root.pending_mutations(), 0)
    view.remove()
    ignore(root.take_batch())
    count.set(2)
    assert_eq(root.pending_mutations(), 0)
  })
  assert_eq(@reactive.active_scope_count(), baseline)
}

///|
test "buttons accept text and rich children without wrapper nodes" {
  @native.test_tree(fn(root) raise {
    let icon = @ui.svg("<svg viewBox=\"0 0 24 24\"/>")
    let label = @ui.text("Save")
    let rich = @ui.button([icon, label])
    let single_child = @ui.text("More")
    let single = @ui.button(single_child)
    let text = @ui.button("Plain label")
    let empty = @ui.div([])
    let view = @ui.div([rich, single, text, empty])
    root.append(view.native_node())
    assert_eq(view.native_node().children().length(), 4)
    assert_eq(rich.native_node().children().length(), 2)
    assert_true(rich.native_node().children()[0] == icon.native_node())
    assert_true(rich.native_node().children()[1] == label.native_node())
    assert_true(
      single.native_node().children()[0] == single_child.native_node(),
    )
    assert_eq(text.native_node().children()[0].text_content(), "Plain label")
    assert_eq(empty.native_node().children().length(), 0)
  })
}

///|
test "fluent styles snapshot and later modifiers retain precedence" {
  @native.test_tree(fn(root) raise {
    let shared = @ui.style().width(120).opacity(0.7)
    let first = @ui.button("First").style(shared).width(200)
    ignore(shared.width(300))
    let second = @ui.button("Second").style(shared)
    let absent : @ui.Style? = None
    let input = @ui.input()
      .value("Ada")
      .placeholder("Name")
      .disabled(true)
      .bind_optional_style(() => absent)
    let view = @ui.div([first, second, input])
    root.append(view.native_node())
    assert_true(
      first.native_node().property(@protocol.WIDTH) == @protocol.Number(200),
    )
    assert_true(
      second.native_node().property(@protocol.WIDTH) == @protocol.Number(300),
    )
    assert_true(
      input.native_node().property(@protocol.VALUE) == @protocol.Text("Ada"),
    )
    assert_true(
      input.native_node().property(@protocol.PLACEHOLDER) ==
      @protocol.Text("Name"),
    )
    assert_true(
      input.native_node().property(@protocol.DISABLED) ==
      @protocol.Boolean(true),
    )
  })
}

///|
test "fluent component callbacks compose and preserve controlled state" {
  let baseline = @reactive.active_scope_count()
  @native.test_tree(fn(root) raise {
    let checked = @reactive.signal(false)
    let changes = @ref.Ref(0)
    let box = @ui.checkbox([])
      .on_change(_ => changes.val += 1)
      .on_checked_change(value => checked.set(value))
      .bind_checked(() => checked.get())
    let indicator = box.slot(@ui.Indicator)
    ignore(box.child(indicator))
    let read_only = @ui.checkbox([]).checked(true).read_only(true)
    root.append(@ui.div([box, read_only]).native_node())
    ignore(root.take_batch())
    box.native_node().emit("click", {})
    assert_true(checked.get())
    assert_eq(changes.val, 1)
    assert_true(box.native_node().children()[0] == indicator.native_node())
    box.native_node().emit("click", {})
    assert_true(!checked.get())
    assert_eq(changes.val, 2)
    read_only.native_node().emit("click", {})
    assert_true(
      read_only.native_node().property(@protocol.CHECKED) ==
      @protocol.Boolean(true),
    )
  })
  assert_eq(@reactive.active_scope_count(), baseline)
}

///|
test "fluent dynamic button callbacks keep the label and button mounted" {
  @native.test_tree(fn(root) raise {
    let count = @reactive.signal(0)
    let button = @ui.button(([] : Array[@ui.Element]))
      .bind_content(() => "Count: \{count.get()}")
      .on_click(() => count.update(n => n + 1))
    root.append(button.native_node())
    let label = button.native_node().children()[0]
    ignore(root.take_batch())
    button.native_node().emit("click", {})
    assert_eq(label.text_content(), "Count: 1")
    assert_eq(root.pending_mutations(), 1)
    assert_true(button.native_node().children()[0] == label)
  })
}
