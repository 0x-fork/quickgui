///|
test "native menu omits empty shortcuts and keeps callbacks in their window owner" {
  let invoked = @ref.Ref(0)
  let ids : Array[Int] = []
  test_tree(fn(root) raise {
    let encoded = encode_menu(
      [
        menu_action(
          "Open",
          () => invoked.val = current_window().id,
          checked=true,
        ),
        menu_role("Copy", "copy", accelerator="CmdOrCtrl+C"),
        submenu("Recent", []),
      ],
      ids,
    )
    assert_true(
      encoded
      is [
        { "accelerator": Null, "checked": True, .. },
        { "accelerator": String("CmdOrCtrl+C"), .. },
        { "items": [], .. },
      ],
    )
    dispatch_menu(ids[0])
    assert_eq(invoked.val, root.window)
  })
  invoked.val = 0
  dispatch_menu(ids[0])
  assert_eq(invoked.val, 0)
  for id in ids {
    menu_callbacks.remove(id)
  }
}

///|
test "popup requests wait for dismissal and retain menu actions after command acceptance" {
  let request : Json = {
    "method": "window-popup-menu",
    "window": 1,
    "menu": "[]",
  }
  assert_eq(
    field(command_request(request, 42), "request"),
    Some((42).to_json()),
  )
  assert_eq(field(request, "request"), None)
  let invoked = @ref.Ref(false)
  let completed = @ref.Ref(false)
  let ids : Array[Int] = []
  test_tree(fn(_) raise {
    ignore(encode_menu([menu_action("Open", () => invoked.val = true)], ids))
    ready.protect(true, fn() raise {
      let id = callback_id(
        _ => {
          completed.val = true
          for id in ids {
            menu_callbacks.remove(id)
          }
        },
        reply_kind=deferred_reply_kind(request),
      )
      fn reply(kind : String) -> Unit {
        dispatch({
          kind,
          window: 0,
          target: id,
          current_target: 0,
          value: None,
          extra: Json::null(),
          data: b"",
          stopped: false,
          default_prevented: false,
        })
      }
      reply("command")
      assert_false(completed.val)
      dispatch_menu(ids[0])
      assert_true(invoked.val)
      reply("popup-menu")
      assert_true(completed.val)
      assert_false(menu_callbacks.contains(ids[0]))
      assert_false(replies.contains(id))
    })
  })
}

///|
test "deferred command errors complete immediately" {
  let result = @ref.Ref(Ok(Json::null()))
  test_tree(fn(_) raise {
    ready.protect(true, fn() raise {
      let id = callback_id(value => result.val = value, reply_kind="popup-menu")
      dispatch({
        kind: "command",
        window: 0,
        target: id,
        current_target: 0,
        value: None,
        extra: { "error": "Window closed" },
        data: b"",
        stopped: false,
        default_prevented: false,
      })
      assert_eq(result.val, Err("Window closed"))
      assert_false(replies.contains(id))
    })
  })
}
