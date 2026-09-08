///|
test "compiled expressions update their own retained nodes" {
  let baseline = @reactive.active_scope_count()
  @native.test_tree(fn(root) raise {
    let (count, set_count) = @ui.create_signal(0)
    let name = @reactive.signal("Ada")
    let checked = @reactive.signal(false)
    let builds = @ref.Ref(0)
    fn view() -> @ui.Element {
      builds.val += 1
      @ui.div([
        @ui.text("Count: \{count()}"),
        @ui.button("Count: \{count()}").on_click(() => set_count(count() + 1)),
        @ui.input()
        .value(name.get())
        .placeholder("Name")
        .on_input(value => name.set(value)),
        @ui.checkbox([])
        .checked(checked.get())
        .on_checked_change(value => checked.set(value)),
        @ui.text("Unchanged"),
      ])
    }
    let node = view().native_node()
    root.append(node)
    let children = node.children()
    let text = children[0].children()[0]
    let button_label = children[1].children()[0]
    ignore(root.take_batch())
    children[1].emit("click", {})
    assert_eq(builds.val, 1)
    assert_eq(text.text_content(), "Count: 1")
    assert_eq(button_label.text_content(), "Count: 1")
    assert_eq(root.pending_mutations(), 2)
    for index, child in node.children() {
      assert_true(child == children[index])
    }
    ignore(root.take_batch())
    name.set("Grace")
    assert_true(
      children[2].property(@protocol.VALUE) == @protocol.Text("Grace"),
    )
    assert_eq(root.pending_mutations(), 1)
    ignore(root.take_batch())
    children[3].emit("click", {})
    assert_true(checked.get())
    children[3].emit("click", {})
    assert_true(!checked.get())
    node.remove()
    ignore(root.take_batch())
    set_count(2)
    name.set("Removed")
    checked.set(true)
    assert_eq(root.pending_mutations(), 0)
  })
  assert_eq(@reactive.active_scope_count(), baseline)
}

///|
test "ui memo getters update only their dependent text and release effects" {
  let baseline = @reactive.active_effect_count()
  let cleaned = @ref.Ref(0)
  @native.test_tree(fn(root) raise {
    let (quantity, set_quantity) = @ui.create_signal(1)
    let (unrelated, set_unrelated) = @ui.create_signal(0)
    let total = @ui.create_memo(() => quantity() * 25)
    let observed : Array[Int] = []
    @ui.create_effect(() => {
      observed.push(total())
      ignore(@ui.untrack(unrelated))
      @ui.on_cleanup(() => cleaned.val += 1)
    })
    let node = @ui.div([@ui.text("Total: \{total()}"), @ui.text("Unchanged")]).native_node()
    root.append(node)
    let label = node.children()[0].children()[0]
    let sibling = node.children()[1]
    ignore(root.take_batch())
    set_unrelated(1)
    assert_eq(observed, [25])
    assert_eq(root.pending_mutations(), 0)
    @ui.batch(() => {
      set_quantity(2)
      set_quantity(3)
    })
    assert_eq(total(), 75)
    assert_eq(observed, [25, 75])
    assert_eq(cleaned.val, 1)
    assert_eq(label.text_content(), "Total: 75")
    assert_true(node.children()[0].children()[0] == label)
    assert_true(node.children()[1] == sibling)
    assert_eq(root.pending_mutations(), 1)
  })
  assert_eq(cleaned.val, 2)
  assert_eq(@reactive.active_effect_count(), baseline)
}

///|
test "styles retain overrides, clear absent declarations, and track interaction styles" {
  @native.test_tree(fn(root) raise {
    let count = @reactive.signal(0)
    let node = @ui.div([])
      .style(@ui.style().opacity(0.5))
      .width(count.get())
      .on_click(() => count.set(1))
      .width(100)
      .style(@ui.style().when(count.get() > 0, s => s.height(50)))
      .hover(s => s.opacity(if count.get() > 0 { 0.8 } else { 0.5 }))
      .native_node()
    root.append(node)
    ignore(root.take_batch())
    count.set(1)
    assert_true(node.property(@protocol.WIDTH) == @protocol.Number(100))
    assert_true(node.property(@protocol.HEIGHT) == @protocol.Number(50))
    match node.property(@protocol.HOVER_STYLE) {
      Text(source) =>
        match @json.parse(source) {
          { "opacity": Number(opacity, ..), .. } =>
            assert_true((opacity - 0.8).abs() < 0.0001)
          _ => fail("Missing hover opacity")
        }
      _ => fail("Missing hover style")
    }
    ignore(root.take_batch())
    count.set(0)
    assert_true(node.property(@protocol.WIDTH) == @protocol.Number(100))
    assert_true(node.property(@protocol.HEIGHT) == @protocol.Clear)
    assert_true(node.property(@protocol.OPACITY) == @protocol.Number(0.5))
  })
}

///|
test "conditional branches keep identity until selection changes" {
  let baseline = @reactive.active_scope_count()
  @native.test_tree(fn(root) raise {
    let count = @reactive.signal(0)
    let cleanups = @ref.Ref(0)
    let node = @ui.div([
      @ui.text("before"),
      if count.get() >= 5 {
        @reactive.on_cleanup(() => cleanups.val += 1)
        @ui.text("high: \{count.get()}")
      } else if count.get() > 0 {
        @reactive.on_cleanup(() => cleanups.val += 1)
        @ui.text("low: \{count.get()}")
      } else {
        @reactive.on_cleanup(() => cleanups.val += 1)
        @ui.text("zero")
      },
      @ui.text("after"),
    ]).native_node()
    root.append(node)
    let before = node.children()[0]
    let after = node.children().last().unwrap()
    ignore(root.take_batch())
    count.set(1)
    assert_eq(cleanups.val, 1)
    let low = node.children()[1]
    ignore(root.take_batch())
    count.set(2)
    assert_eq(cleanups.val, 1)
    assert_true(node.children()[1] == low)
    assert_eq(low.children()[0].text_content(), "low: 2")
    assert_eq(root.pending_mutations(), 1)
    count.set(5)
    assert_eq(cleanups.val, 2)
    assert_true(node.children()[0] == before)
    assert_true(node.children().last().unwrap() == after)
    node.remove()
    assert_eq(cleanups.val, 3)
  })
  assert_eq(@reactive.active_scope_count(), baseline)
}

///|
test "optional props and list-spread branches retain ownership and clear removed styles" {
  let baseline = @reactive.active_scope_count()
  @native.test_tree(fn(root) raise {
    let count = @reactive.signal(0)
    let node = @ui.div(
      [
        @ui.button("Save").bind_optional(@protocol.DISABLED, () => {
          if count.get() > 0 {
            Some(true)
          } else {
            None
          }
        }),
        @ui.div([])
        .bind_optional_style(() => {
          if count.get() > 0 {
            Some(@ui.style().width(80))
          } else {
            None
          }
        })
        .height(20),
        ..if count.get() > 0 {
          [
            @ui.text(
              (
                $| Count: \{count.get()}
              ),
            ),
          ]
        } else {
          []
        },
        @ui.text("last"),
      ],
    ).native_node()
    root.append(node)
    let button = node.children()[0]
    let styled = node.children()[1]
    let last = node.children().last().unwrap()
    count.set(1)
    let branch = node.children()[2]
    assert_true(button.property(@protocol.DISABLED) == @protocol.Boolean(true))
    assert_true(styled.property(@protocol.WIDTH) == @protocol.Number(80))
    count.set(2)
    assert_true(node.children()[2] == branch)
    assert_eq(branch.children()[0].text_content(), " Count: 2")
    count.set(0)
    assert_true(button.property(@protocol.DISABLED) == @protocol.Clear)
    assert_true(styled.property(@protocol.WIDTH) == @protocol.Clear)
    assert_true(styled.property(@protocol.HEIGHT) == @protocol.Number(20))
    assert_true(node.children().last().unwrap() == last)
  })
  assert_eq(@reactive.active_scope_count(), baseline)
}

///|
test "computed child lists and spreads follow collection changes without retaining old branches" {
  let baseline = @reactive.active_scope_count()
  @native.test_tree(fn(root) raise {
    let names = @reactive.signal(["Ada"])
    let node = @ui.div(
      [
        @ui.text("before"),
        ..names.get().map(name => @ui.text(name)),
        @ui.text("after"),
      ],
    ).native_node()
    root.append(node)
    let first = node.children()[0]
    let last = node.children().last().unwrap()
    names.set(["Grace", "Linus"])
    assert_true(node.children()[0] == first)
    assert_true(node.children().last().unwrap() == last)
    assert_eq(node.children()[1].children()[0].text_content(), "Grace")
    assert_eq(node.children()[2].children()[0].text_content(), "Linus")
    names.set([])
    assert_eq(node.children().length(), 3)
    node.remove()
    ignore(root.take_batch())
    names.set(["Removed"])
    assert_eq(root.pending_mutations(), 0)
  })
  assert_eq(@reactive.active_scope_count(), baseline)
}

///|
test "getter expressions bind scoped labels and generic native properties" {
  @native.test_tree(fn(root) raise {
    let (name, set_name) = @reactive.create_signal("Ada")
    let box = @ui.field([])
    let label = box
      .slot(@ui.Label)
      .label(name())
      .property(@protocol.VALUE, @protocol.Text(name()))
      .native_node()
    box.native_node().append(label)
    root.append(box.native_node())
    let text = label.children()[0]
    set_name("Grace")
    assert_eq(text.text_content(), "Grace")
    assert_true(label.children()[0] == text)
    assert_true(
      label.property(@protocol.ACCESSIBILITY_LABEL) == @protocol.Text("Grace"),
    )
    assert_true(label.property(@protocol.VALUE) == @protocol.Text("Grace"))
    ignore(root.take_batch())
    set_name("Grace")
    assert_eq(root.pending_mutations(), 0)
  })
}

///|
test "interaction style factories read signals inside their binding" {
  @native.test_tree(fn(root) raise {
    let (count, set_count) = @reactive.create_signal(0)
    fn build_style(value : Int) -> (@ui.Style) -> @ui.Style {
      s => s.opacity(if value > 0 { 1.0 } else { 0.5 })
    }
    let node = @ui.div([]).hover(build_style(count())).native_node()
    root.append(node)
    set_count(1)
    match node.property(@protocol.HOVER_STYLE) {
      Text(source) =>
        match @json.parse(source) {
          { "opacity": Number(value, ..), .. } => assert_eq(value, 1)
          _ => fail("Missing opacity")
        }
      _ => fail("Missing hover style")
    }
  })
}

///|
test "fluent JSON props keep their type and slot styles stay on their own nodes" {
  @native.test_tree(fn(root) raise {
    let (count, set_count) = @reactive.create_signal(1)
    let tabs = @ui.tabs([])
    let panel = tabs
      .width(count())
      .slot(@ui.Panel, value="one")
      .height(count() + 10)
    root.append(tabs.children([panel]).native_node())
    let table = @ui.table([])
      .columns([{ "id": "name", "track": "\{count()}px", "rowHeader": true }])
      .row_count(count())
      .native_node()
    root.append(table)
    ignore(root.take_batch())
    set_count(2)
    assert_true(
      tabs.native_node().property(@protocol.WIDTH) == @protocol.Number(2),
    )
    assert_true(
      panel.native_node().property(@protocol.WIDTH) == @protocol.Clear,
    )
    assert_true(
      panel.native_node().property(@protocol.HEIGHT) == @protocol.Number(12),
    )
    assert_true(table.property(@protocol.ROW_COUNT) == @protocol.Number(2))
    match table.property(@protocol.COLUMNS) {
      Text(value) =>
        assert_eq(@json.parse(value), [
          { "id": "name", "track": "2px", "rowHeader": true },
        ])
      _ => fail("Missing columns")
    }
  })
}
