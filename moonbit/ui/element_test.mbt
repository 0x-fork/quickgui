///|
test "color channels match the native RGBA byte layout, including alpha" {
  assert_eq(@ui.rgba8(0x12, 0x34, 0x56, 0x78).to_json(), 0x78563412U.to_json())
  @native.test_tree(fn(root) raise {
    let view = @ui.view([])
      .bg(@ui.rgb8(0, 122, 255))
      .text_color(@ui.rgb8(29, 29, 31))
      .border_color(@ui.rgba8(0x12, 0x34, 0x56, 0x78))
      .hover(s => s.bg(@ui.rgba8(0, 0, 0, 28)))
    root.append(view.native_node())
    assert_true(
      view.native_node().property(@protocol.BACKGROUND_COLOR) ==
      @protocol.Color(0xffff7a00),
    )
    assert_true(
      view.native_node().property(@protocol.COLOR) ==
      @protocol.Color(0xff1f1d1d),
    )
    assert_true(
      view.native_node().property(@protocol.BORDER_COLOR) ==
      @protocol.Color(0x78563412),
    )
    assert_true(
      view.native_node().property(@protocol.HOVER_STYLE) ==
      @protocol.Text("{\"backgroundColor\":469762048}"),
    )
    let batch = @protocol.batch()
    batch.set_property(
      1,
      @protocol.COLOR,
      view.native_node().property(@protocol.BORDER_COLOR),
    )
    let bytes = batch.finish()
    assert_eq(bytes[bytes.length() - 4:], b"\x12\x34\x56\x78"[:])
  })
}

///|
test "fluent chains retain identities and change a single text node" {
  let baseline = @reactive.active_scope_count()
  @native.test_tree(fn(root) raise {
    let count = @reactive.signal(0)
    let view = @ui.view([])
      .size_full()
      .flex_col()
      .gap(12)
      .child(@ui.view([]).bind_content(() => "Count: \{count.get()}"))
      .child(@ui.button("Increment").on_click(() => count.update(n => n + 1)))
    root.append(view.native_node())
    let child = view.native_node().children()[0].children()[0]
    ignore(root.take_batch())
    count.set(1)
    assert_eq(root.pending_mutations(), 1)
    assert_eq(child.text_content(), "Count: 1")
    assert_eq(view.native_node().children()[0].children()[0].id, child.id)
    count.set(1)
    assert_eq(root.pending_mutations(), 1)
    ignore(root.take_batch())
    view.remove()
    ignore(root.take_batch())
    count.set(2)
    assert_eq(root.pending_mutations(), 0)
  })
  assert_eq(@reactive.active_scope_count(), baseline)
}

///|
test "conditional children mount only on predicate changes and clean up" {
  @native.test_tree(fn(root) raise {
    let count = @reactive.signal(0)
    let mounts = @ref.Ref(0)
    let view = @ui.view([]).child_if(() => count.get() >= 2, () => {
      mounts.val += 1
      @ui.view([]).bind_content(() => "Value: \{count.get()}")
    })
    root.append(view.native_node())
    count.set(2)
    assert_eq(mounts.val, 1)
    let text = view.native_node().children()[0].children()[0]
    ignore(root.take_batch())
    count.set(3)
    assert_eq(mounts.val, 1)
    assert_eq(text.text_content(), "Value: 3")
    assert_eq(root.pending_mutations(), 1)
    count.set(0)
    assert_eq(view.native_node().children().length(), 1)
    count.set(2)
    assert_eq(mounts.val, 2)
  })
}

///|
test "dynamic styles merge, clear removed properties, and skip equality" {
  @native.test_tree(fn(root) raise {
    let active = @reactive.signal(false)
    let view = @ui.view([])
      .width(100)
      .style(@ui.style().width(200))
      .bind_style(() => {
        if active.get() {
          @ui.style().opacity(0.5).height(40)
        } else {
          @ui.style().opacity(1.0)
        }
      })
    root.append(view.native_node())
    assert_true(
      view.native_node().property(@protocol.WIDTH) == @protocol.Number(200),
    )
    ignore(root.take_batch())
    active.set(true)
    assert_eq(root.pending_mutations(), 2)
    ignore(root.take_batch())
    active.set(false)
    assert_eq(root.pending_mutations(), 2)
    assert_true(
      view.native_node().property(@protocol.HEIGHT) == @protocol.Clear,
    )
  })
}

///|
test "keyed children move existing rows and retire only removed rows" {
  let baseline = @reactive.active_scope_count()
  @native.test_tree(fn(root) raise {
    let values = @reactive.signal(["a", "b", "c"])
    let mounts = @ref.Ref(0)
    let view = @ui.view([]).children_keyed(() => values.get(), value => value, value => {
      mounts.val += 1
      @ui.view([]).bind_content(() => value.get())
    })
    root.append(view.native_node())
    let original = view.native_node().children()
    ignore(root.take_batch())
    values.set(["c", "a", "b"])
    let moved = view.native_node().children()
    assert_eq(mounts.val, 3)
    assert_eq(moved[0].id, original[2].id)
    assert_eq(moved[1].id, original[0].id)
    assert_eq(moved[2].id, original[1].id)
    ignore(root.take_batch())
    values.set(["c", "b", "d"])
    assert_eq(mounts.val, 4)
    assert_eq(original[0].children().length(), 0)
  })
  assert_eq(@reactive.active_scope_count(), baseline)
}

///|
test "deferred child effects retain the owning window across windows" {
  let baseline = @reactive.active_scope_count()
  @native.test_tree(fn(first) raise {
    let value = @reactive.signal(false)
    let view = @ui.view([]).child_if(() => value.get(), () => {
      @ui.text("First window")
    })
    first.append(view.native_node())
    @native.test_tree(fn(second) raise {
      assert_true(first.window != second.window)
      value.set(true)
      let child = view.native_node().children()[0]
      assert_eq(child.window, first.window)
      assert_eq(child.children()[0].text_content(), "First window")
      assert_eq(second.children().length(), 0)
    })
  })
  assert_eq(@reactive.active_scope_count(), baseline)
}
