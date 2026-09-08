///|
test "mounted work waits for native readiness and skips removed nodes" {
  test_tree(root => {
    let alive = element(@protocol.TAG_VIEW)
    let removed = element(@protocol.TAG_VIEW)
    root.append(alive)
    root.append(removed)
    let ran = @ref.Ref(0)
    alive.after_mount(() => {
      ran.val += if current_window().id == root.window { 1 } else { 1000 }
    })
    removed.after_mount(() => ran.val += 100)
    removed.remove()
    assert_true(ran.val == 0)
    dispatch({
      kind: "window-ready-to-show",
      window: root.window,
      target: 0,
      value: None,
      extra: Json::null(),
      data: b"",
      current_target: 0,
      stopped: false,
      default_prevented: false,
    })
    assert_true(ran.val == 1)
    alive.after_mount(() => ran.val += 1)
    assert_true(ran.val == 2)
  })
}
