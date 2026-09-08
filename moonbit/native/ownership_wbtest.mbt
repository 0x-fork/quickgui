///|
test "completed native requests do not retain owner cleanup closures" {
  let root = @reactive.create_scope(parent=0)
  let baseline = @reactive.active_cleanup_count()
  ready.protect(true, fn() raise {
    @reactive.with_owner(root, fn() raise {
      for _ in 0..<200 {
        let id = callback_id(_ => ())
        dispatch({
          kind: "command",
          window: 0,
          target: id,
          current_target: 0,
          value: Some("true"),
          extra: Json::null(),
          data: b"",
          stopped: false,
          default_prevented: false,
        })
      }
      assert_eq(replies.length(), 0)
      assert_eq(@reactive.active_cleanup_count(), baseline)
    })
  })
  @reactive.dispose(root)
}

///|
test "window subscriptions registered from an application owner retire on close" {
  let root = @reactive.create_scope(parent=0)
  let baseline = @reactive.active_cleanup_count()
  test_tree(window => {
    @reactive.with_owner(root, () => {
      on_event("close", _ => (), window=window.window)
    })
  })
  assert_eq(subscriptions.length(), 0)
  assert_eq(@reactive.active_cleanup_count(), baseline)
  @reactive.dispose(root)
}

///|
test "cleanup can schedule native service shutdown without retaining a disposed owner" {
  let root = @reactive.create_scope(parent=0)
  application_scope.protect(root, () => {
    ready.protect(true, () => {
      let child = @reactive.create_scope(parent=root)
      @reactive.with_owner(child, () => {
        @reactive.on_cleanup(() => ignore(callback_id(_ => ())))
      })
      @reactive.dispose(child)
    })
  })
  assert_eq(replies.length(), 1)
  for _, callback in replies {
    assert_eq(callback.scope, root)
  }
  @reactive.dispose(root)
  assert_eq(replies.length(), 0)
}

///|
test "effects settling after a close request never send a batch to the closed native window" {
  test_tree(root => {
    let window = windows.get(root.window).unwrap()
    window.created = true
    window.closing = true
    let count = @reactive.signal(0)
    let label = text("0")
    root.append(label)
    label.effect(() => label.set_text(count.get().to_string()))
    count.set(1)
    // Would call the unavailable native host if closing windows were flushed.
    flush()
    assert_true(window.batch.is_empty())
    assert_false(dirty_windows.contains(root.window))
    // The owner remains available until native reports the close event.
    assert_eq(current_window().id, root.window)
  })
}
