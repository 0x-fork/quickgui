///|
fn scroll_area_demo(t : Theme) -> @ui.Element {
  let state : @reactive.Signal[Json] = @reactive.signal({})
  let root = observe(
      @ui.scroll_area([])
      .viewport_size({ "width": 320, "height": 160 })
      .content_size({ "width": 320, "height": 880 })
      .overflow_edge_threshold(2),
      state,
    )
    .flex_row()
    .gap(4)
    .height(160)
  let content = root
    .slot(@ui.Content)
    .flex_col()
    .padding_left(10)
    .bind_style(() => {
      @ui.style().transform(
        "translateY(\{-number(get(get(state.get(), "offset"), "y"))}px)",
      )
    })
  for index in 0..<40 {
    ignore(
      content.children([
        muted(t, "log line \{index + 1}")
        .height(22)
        .flex_shrink(0)
        .line_height(22),
      ]),
    )
  }
  panel(
    t, "Scroll area", "Forty rows in a fixed viewport. Offsets, overflow flags, and thumb geometry come from the native scroll model.",
  ).children([
    root.children([
      framed(t, root.slot(@ui.Viewport))
      .children([content])
      .width(320)
      .height(160)
      .overflow("hidden"),
      root
      .slot(@ui.Scrollbar)
      .children([
        root
        .slot(@ui.Thumb)
        .orientation("vertical")
        .width(8)
        .border_radius(4)
        .bind_style(() => {
          @ui.style().bg(
            t.color(
              if get(state.get(), "scrolling") == Json::boolean(true) {
                "accent"
              } else {
                "border"
              },
            ),
          )
        }),
      ])
      .orientation("vertical")
      .width(8)
      .height(160)
      .border_radius(4)
      .bind_style(() => @ui.style().bg(t.color("track"))),
    ]),
    readout(t, state),
  ])
}

///|
struct Asset {
  name : String
  size : Int
} derive(Eq)

///|
struct AssetRow {
  row : Int
  asset : Asset
} derive(Eq)

///|
fn asset_rows(count : Int) -> Array[Asset] {
  Array::makei(count, index => {
    let n = index.to_string()
    {
      name: "asset-" + "0".repeat((4 - n.length()).max(0)) + n + ".png",
      size: 8 + index * 37 % 4000,
    }
  })
}

///|
fn table_demo(t : Theme) -> @ui.Element {
  let assets = asset_rows(5000)
  let (sort, set_sort) = @ui.create_signal(Json::null())
  let (range, set_range) = @ui.create_signal(@ui.VisibleRange::{
    start: 0,
    end: 20,
  })
  let (selection, set_selection) = @ui.create_signal(([] : Json))
  let state : @reactive.Signal[Json] = @reactive.signal({})
  let (activated, set_activated) = @ui.create_signal("nothing yet")
  let ordered = @ui.create_memo(() => {
    let order = sort()
    if order == Json::null() {
      return assets
    }
    let sorted = assets.copy()
    let column = string(get(order, "column"))
    let descending = string(get(order, "direction")) == "descending"
    sorted.sort_by((a, b) => {
      let compared = if column == "size" {
        a.size.compare(b.size)
      } else {
        a.name.compare(b.name)
      }
      if descending {
        -compared
      } else {
        compared
      }
    })
    sorted
  })
  let root = framed(
      t,
      @ui.data_table([])
      .columns([
        {
          "id": "name",
          "label": "Name",
          "track": "1fr",
          "minWidth": 140,
          "sortable": true,
          "rowHeader": true,
        },
        {
          "id": "size",
          "label": "Size",
          "width": 140,
          "minWidth": 90,
          "align": "right",
          "sortable": true,
        },
      ])
      .row_count(5000)
      .row_height(26)
      .header_height(28)
      .selection_mode("multiple")
      .bind_selection(() => selection())
      .bind_sort_column(() => string(get(sort(), "column")))
      .bind_sort_direction(() => string(get(sort(), "direction")))
      .on_change(data => {
        if get(data, "visibleRange") is Object(_) {
          set_range(@ui.visible_range(get(data, "visibleRange"), 5000))
        }
        if get(data, "selectedRanges") is Array(_) {
          set_selection(get(data, "selectedRanges"))
        }
        if data is { "sort": order, .. } {
          set_sort(order)
        }
        state.set(data)
      })
      .on("commit", @protocol.COMMIT_LISTENER, e => {
        set_activated(e.json().stringify())
      }),
    )
    .height(260)
    .overflow_y("scroll")
  for name in ["name", "size"] {
    ignore(
      root.children([
        root
        .slot(@ui.Header, value=name)
        .children(
          [
            @ui.text(if name == "name" { "NAME" } else { "SIZE" }),
            ..if string(get(sort(), "column")) == name {
              [
                @ui.view(
                  [
                    ..if string(get(sort(), "direction")) == "ascending" {
                      [icon("up", size=12)]
                    } else {
                      [icon("chevron", size=12)]
                    },
                  ],
                ),
              ]
            } else {
              []
            },
          ],
        )
        .px(10)
        .font_size(11)
        .font_weight(700)
        .bind_style(() => @ui.style().text_color(t.color("faint"))),
      ]),
    )
  }
  ignore(
    root.children_keyed(
      () => {
        let list = ordered()
        let window = range()
        let start = window.start.clamp(min=0, max=list.length())
        let end = window.end.clamp(min=start, max=list.length())
        Array::makei(end - start, offset => {
          row: start + offset,
          asset: list[start + offset],
        })
      },
      entry => entry.row.to_string(),
      entry => {
        let node = root.slot(@ui.Row, index=entry.peek().row)
        node.children([
          node
          .slot(@ui.Cell, value="name")
          .children([
            note(t, () => entry.get().asset.name).font_family("system-ui"),
          ])
          .padding_left(10),
          node
          .slot(@ui.Cell, value="size")
          .children([
            note(t, () => "\{entry.get().asset.size} KB").font_family(
              "system-ui",
            ),
          ])
          .padding_right(10)
          .justify_content("flex-end"),
        ])
      },
    ),
  )
  panel(
    t, "Table", "Five thousand sortable rows. Only the visible range is mounted. Drag column edges, select multiple rows, or double-click to activate.",
  ).children([
    root,
    note(t, () => {
      "rows \{range().start}–\{range().end} of 5000 · selection \{selection().stringify()} · sort \{sort().stringify()}"
    }),
    note(t, () => "activated \{activated()}"),
    readout(t, state),
  ])
}

///|
struct TreeEntry {
  id : String
  label : String
  children : Array[TreeEntry]
  pending : Bool
} derive(Eq, ToJson)

///|
struct TreeRow {
  entry : TreeEntry
  depth : Int
} derive(Eq)

///|
fn leaf(id : String, label : String, pending? : Bool = false) -> TreeEntry {
  { id, label, children: [], pending, }
}

///|
fn flatten_tree(
  nodes : Array[TreeEntry],
  expanded : Array[String],
  loaded : Array[TreeEntry],
  depth : Int,
  rows : Array[TreeRow],
) -> Unit {
  for entry in nodes {
    rows.push({ entry, depth, })
    if expanded.contains(entry.id) {
      flatten_tree(
        if entry.pending {
          loaded
        } else {
          entry.children
        },
        expanded,
        loaded,
        depth + 1,
        rows,
      )
    }
  }
}

///|
fn tree_demo(t : Theme) -> @ui.Element {
  let nodes = [
    TreeEntry::{
      id: "src",
      label: "src",
      pending: false,
      children: [
        leaf("src/main.mbt", "main.mbt"),
        leaf("src/ui.mbt", "ui.mbt"),
        leaf("src/element", "element", pending=true),
      ],
    },
    TreeEntry::{
      id: "docs",
      label: "docs",
      pending: false,
      children: [
        leaf("docs/native.md", "native.md"),
        leaf("docs/tabs.md", "tabs.md"),
      ],
    },
  ]
  let (expanded, set_expanded) = @ui.create_signal(["src"])
  let (selected, set_selected) = @ui.create_signal("")
  let (activated, set_activated) = @ui.create_signal("nothing yet")
  let (range, set_range) = @ui.create_signal(@ui.VisibleRange::{
    start: 0,
    end: 30,
  })
  let (loaded, set_loaded) = @ui.create_signal([])
  let (asked, set_asked) = @ui.create_signal(0)
  let root = framed(
      t,
      @ui.tree([])
      .nodes(nodes.to_json())
      .bind_expanded(() => expanded().to_json())
      .bind_selected_value(() => selected())
      .row_height(26)
      .property(@protocol.LOADING_LABEL, @protocol.Text("Loading…"))
      .property(@protocol.DISCLOSURE, @protocol.Text("leading"))
      .bind_json(@protocol.SET_CHILDREN, () => {
        if loaded().is_empty() {
          Json::null()
        } else {
          { "id": "src/element", "children": loaded() }
        }
      })
      .on_change(data => {
        if get(data, "expanded") is Array(_) {
          set_expanded(strings(get(data, "expanded")))
        }
        if get(data, "value") is String(v) {
          set_selected(v)
        }
        if get(data, "visibleRange") is Object(_) {
          set_range(@ui.visible_range(get(data, "visibleRange"), 100))
        }
        if get(data, "loadChildren") is String(id) &&
          @ui.untrack(loaded).is_empty() {
          set_asked(asked() + 1)
          // Completion of an expansion request must preserve that expansion,
          // including when the load request precedes the native state report.
          if !@ui.untrack(expanded).contains(id) {
            set_expanded([..expanded(), id])
          }
          set_loaded([
            leaf(id + "/style.mbt", "style.mbt"),
            leaf(id + "/state.mbt", "state.mbt"),
          ])
        }
      })
      .on("commit", @protocol.COMMIT_LISTENER, e => {
        set_activated(string(get(e.json(), "value")))
      }),
    )
    .height(220)
    .overflow_y("scroll")
  ignore(
    root.children_keyed(
      () => {
        let rows : Array[TreeRow] = []
        flatten_tree(nodes, expanded(), loaded(), 0, rows)
        let window = range()
        let start = window.start.clamp(min=0, max=rows.length())
        let end = window.end.clamp(min=start, max=rows.length())
        rows[start:end].to_owned()
      },
      r => r.entry.id,
      entry => {
        root
        .slot(@ui.Row, value=entry.peek().entry.id)
        .children([label(t, entry.peek().entry.label)])
        .height(26)
        .flex_row()
        .items_center()
        .bind_style(() => @ui.style().padding_left(10 + entry.get().depth * 16))
      },
    ),
  )
  panel(
    t, "Tree", "Expand the pending element branch to request its children once. Arrow keys navigate; double-click reports activation.",
  ).children([
    root,
    note(t, () => {
      "expanded \{expanded().to_json().stringify()} · selected \{selected()} · activated \{activated()} · load requests \{asked()}"
    }),
  ])
}
