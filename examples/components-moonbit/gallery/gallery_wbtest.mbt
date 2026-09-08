///|
fn descendants(node : @native.Node) -> Array[@native.Node] {
  let result = [node]
  for child in node.children() {
    result.append(descendants(child))
  }
  result
}

///|
fn find_part(
  node : @native.Node,
  role : String,
  value? : String = "",
) -> @native.Node {
  descendants(node)
  .iter()
  .find_first(n => {
    n.property(@protocol.PART) == @protocol.Text(role) &&
    (value == "" || n.property(@protocol.PART_VALUE) == @protocol.Text(value))
  })
  .unwrap()
}

///|
fn find_label(node : @native.Node, label : String) -> @native.Node {
  descendants(node)
  .iter()
  .find_first(n => {
    n.property(@protocol.ACCESSIBILITY_LABEL) == @protocol.Text(label)
  })
  .unwrap()
}

///|
test "all 43 Go gallery demos mount independently in both appearances" {
  assert_eq(demos.length(), 43)
  let ids : Map[String, Unit] = Map([])
  for demo in demos {
    assert_true(!ids.contains(demo.id))
    ids[demo.id] = ()
    for appearance in ["light", "dark"] {
      @native.test_tree(root => {
        let t = theme()
        t.appearance.set(appearance)
        root.append((demo.render)(t).native_node())
        assert_eq(root.children().length(), 1)
        assert_true(root.pending_mutations() > 0)
        // Even the 5,000-row table starts with a bounded retained tree.
        assert_true(descendants(root).length() < 500)
      })
    }
  }
}

///|
test "gallery selection highlights the tab, retains the sidebar, and disposes old panels" {
  @native.test_tree(root => {
    let t = theme()
    let selected = @reactive.signal("accordion")
    root.append(
      gallery(t, selected, @reactive.signal("1080×780")).native_node(),
    )
    let first = find_part(root, "tab", value="accordion")
    let checkbox_tab = find_part(root, "tab", value="checkbox")
    let old = find_part(root, "accordion")
    ignore(root.take_batch())
    checkbox_tab.emit("click", {})
    assert_eq(selected.peek(), "checkbox")
    assert_true(find_part(root, "tab", value="accordion") == first)
    assert_true(find_part(root, "tab", value="checkbox") == checkbox_tab)
    assert_true(!descendants(root).contains(old))
    assert_value(
      checkbox_tab.property(@protocol.BACKGROUND_COLOR),
      @protocol.Color(t.color("accent").value),
    )
    assert_value(
      first.property(@protocol.BACKGROUND_COLOR),
      @protocol.Color(t.color("transparent").value),
    )
    t.appearance.set("dark")
    assert_value(
      checkbox_tab.property(@protocol.BACKGROUND_COLOR),
      @protocol.Color(t.color("accent").value),
    )
    for demo in demos {
      selected.set(demo.id)
    }
    assert_true(descendants(root).length() < 350)
  })
}

///|
test "gallery checkbox toggles both ways, clears mixed, preserves read-only and parent state" {
  @native.test_tree(root => {
    root.append(checkbox_demo(theme()).native_node())
    let box = find_label(root, "Email me about releases")
    let read_only = find_label(root, "Read-only: focusable, refuses changes")
    let parent = find_label(root, "Parent, derived from its children")
    assert_value(box.property(@protocol.INDETERMINATE), @protocol.Boolean(true))
    box.emit("click", {})
    assert_value(
      box.property(@protocol.INDETERMINATE),
      @protocol.Boolean(false),
    )
    assert_value(box.property(@protocol.CHECKED), @protocol.Boolean(true))
    box.emit("click", {})
    assert_value(box.property(@protocol.CHECKED), @protocol.Boolean(false))
    read_only.emit("click", {})
    assert_value(read_only.property(@protocol.CHECKED), @protocol.Boolean(true))
    assert_value(
      parent.property(@protocol.INDETERMINATE),
      @protocol.Boolean(true),
    )
    parent.emit("click", {})
    assert_value(
      find_label(root, "Analytics").property(@protocol.CHECKED),
      @protocol.Boolean(true),
    )
    assert_value(
      find_label(root, "Crash reports").property(@protocol.CHECKED),
      @protocol.Boolean(true),
    )
    parent.emit("click", {})
    assert_value(
      find_label(root, "Analytics").property(@protocol.CHECKED),
      @protocol.Boolean(false),
    )
    assert_value(
      find_label(root, "Crash reports").property(@protocol.CHECKED),
      @protocol.Boolean(false),
    )
  })
}

///|
test "native table range and sort retain row identities and never mount all 5000 rows" {
  @native.test_tree(root => {
    root.append(table_demo(theme()).native_node())
    let table = find_part(root, "table")
    let rows = () => {
      descendants(root).filter(n => {
        n.property(@protocol.PART) == @protocol.Text("table-row")
      })
    }
    assert_eq(rows().length(), 20)
    for row in rows() {
      assert_eq(row.children().length(), 2)
      for cell in row.children() {
        assert_value(
          cell.property(@protocol.PART),
          @protocol.Text("table-cell"),
        )
        assert_true(!cell.children().is_empty())
      }
    }
    table.emit("componentchange", {
      "visibleRange": { "start": 110, "end": 130 },
    })
    assert_eq(rows().length(), 20)
    let first = rows()[0]
    assert_value(first.property(@protocol.ROW_INDEX), @protocol.Number(110))
    table.emit("componentchange", {
      "sort": { "column": "size", "direction": "descending" },
    })
    assert_true(rows()[0] == first)
    // A row has two cells and retained labels; include fixed headers/readouts.
    assert_true(descendants(root).length() <= rows().length() * 8 + 40)
    table.emit("componentchange", {
      "visibleRange": { "start": 4995, "end": 7000 },
    })
    assert_eq(rows().length(), 5)
  })
}

///|
test "splitter labels stay inside their scoped panes" {
  @native.test_tree(root => {
    root.append(splitter_demo(theme()).native_node())
    let split = find_part(root, "splitter")
    let panes = split
      .children()
      .filter(n => n.property(@protocol.PART) == @protocol.Text("splitter-pane"))
    assert_eq(panes.length(), 3)
    for pane in panes {
      assert_eq(pane.children().length(), 1)
      assert_true(!pane.children()[0].children().is_empty())
    }
  })
}

///|
test "tree lazy children are requested once and collapse removes their rows" {
  @native.test_tree(root => {
    root.append(tree_demo(theme()).native_node())
    let tree = find_part(root, "tree")
    let count = () => {
      descendants(root)
      .filter(n => n.property(@protocol.PART) == @protocol.Text("tree-row"))
      .length()
    }
    assert_eq(count(), 5)
    tree.emit("componentchange", { "loadChildren": "src/element" })
    assert_eq(count(), 7)
    let splice = tree.property(@protocol.SET_CHILDREN)
    assert_true(splice != @protocol.Clear)
    tree.emit("componentchange", { "loadChildren": "src/element" })
    assert_value(tree.property(@protocol.SET_CHILDREN), splice)
    tree.emit("componentchange", { "expanded": ["src"] })
    assert_eq(count(), 5)
  })
}

///|
test "civil calendar covers leap days, six complete weeks and month boundaries" {
  let sept = month_grid("2026-09")
  assert_eq(sept.length(), 6)
  assert_eq(sept[0][0], "2026-08-30")
  assert_eq(sept[5][6], "2026-10-10")
  assert_eq(month_days(2000, 2), 29)
  assert_eq(month_days(2100, 2), 28)
  assert_eq(month_shift("2026-12", 1), "2027-01")
  assert_eq(month_shift("2026-01", -1), "2025-12")
  @native.test_tree(root => {
    root.append(calendar_demo(theme()).native_node())
    let weeks = descendants(root).filter(n => {
      n.property(@protocol.PART) == @protocol.Text("calendar-week")
    })
    assert_eq(weeks.length(), 6)
    for index, week in weeks {
      assert_value(
        week.property(@protocol.ITEM_INDEX),
        @protocol.Number(Float::from_int(index)),
      )
    }
  })
}

///|
fn assert_value(a : @protocol.Value, b : @protocol.Value) -> Unit raise {
  assert_true(a == b)
}
