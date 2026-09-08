///|
test "Rust presets use the same spacing, flex, grid, and corner values" {
  @native.test_tree(fn(root) raise {
    let view = @ui.div([])
      .flex()
      .flex_col_reverse()
      .flex_1()
      .items_end()
      .justify_between()
      .p_4()
      .px_2()
      .m_4()
      .ml_auto()
      .grid_cols_min_content(3)
      .col_span_full()
      .rounded_tl(2)
      .rounded_lg()
      .text_lg()
    let node = view.native_node()
    root.append(node)
    for
      pair in [
        (@protocol.DISPLAY, "flex"),
        (@protocol.FLEX_DIRECTION, "column-reverse"),
        (@protocol.ALIGN_ITEMS, "flex-end"),
        (@protocol.JUSTIFY_CONTENT, "space-between"),
        (@protocol.MARGIN_LEFT, "auto"),
        (@protocol.GRID_TEMPLATE_COLUMNS, "repeat(3, minmax(min-content, 1fr))"),
      ] {
      let (code, value) = pair
      assert_true(node.property(code) == @protocol.Text(value))
    }
    for
      pair in [
        (@protocol.FLEX_GROW, 1),
        (@protocol.FLEX_SHRINK, 1),
        (@protocol.FLEX_BASIS, 0),
        (@protocol.PADDING_TOP, 16),
        (@protocol.PADDING_RIGHT, 8),
        (@protocol.PADDING_BOTTOM, 16),
        (@protocol.PADDING_LEFT, 8),
        (@protocol.MARGIN_TOP, 16),
        (@protocol.GRID_COLUMN_START, 1),
        (@protocol.GRID_COLUMN_END, -1),
        (@protocol.BORDER_RADIUS, 8),
        (@protocol.FONT_SIZE, 18),
        (@protocol.LINE_HEIGHT, 26),
      ] {
      let (code, value) = pair
      assert_true(
        node.property(code) == @protocol.Number(Float::from_int(value)),
      )
    }
    assert_true(
      node.property(@protocol.BORDER_TOP_LEFT_RADIUS) == @protocol.Clear,
    )
    ignore(view.col_start(5).col_end(8).col_span(2))
    assert_true(node.property(@protocol.GRID_COLUMN_START) == @protocol.Clear)
    assert_true(node.property(@protocol.GRID_COLUMN_END) == @protocol.Clear)
    assert_true(
      node.property(@protocol.GRID_COLUMN_SPAN) == @protocol.Number(2),
    )
    ignore(view.grid_cols(0).w_fraction(-1))
    assert_true(
      node.property(@protocol.GRID_TEMPLATE_COLUMNS) == @protocol.Text("none"),
    )
    assert_true(node.property(@protocol.WIDTH) == @protocol.Text("0%"))
    let shared = @ui.style().rounded_lg().gap_4()
    ignore(view.rounded_tl(3).style(shared))
    assert_true(
      node.property(@protocol.BORDER_TOP_LEFT_RADIUS) == @protocol.Clear,
    )
    assert_eq(@ui.style().rounded_lg().json(), { "borderRadius": 8 })
  })
}
