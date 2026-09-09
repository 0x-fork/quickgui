///|
fn history_view(s : Store) -> @ui.Element {
  row()
  .children([
    col()
    .children(
      [
        row()
        .children([
          label(
            s,
            () => {
              if s.all_branches.get() {
                "ALL BRANCHES"
              } else if s.history_ref.get() != "" {
                s.history_ref.get()
              } else {
                s.status.get().branch
              }
            },
            tone="muted",
          )
          .font_weight(600)
          .font_size(11)
          .line_clamp(1),
          label(s, () => "\{s.commits.get().length()} commits", tone="muted").font_size(
            11,
          ),
          @ui.view([]).flex_1(),
          check(s, "All branches", () => s.all_branches.get(), value => {
            s.all_branches.set(value)
            s.history_ref.set("")
            s.load_history(true)
          }),
        ])
        .height(32)
        .flex_shrink(0)
        .px(12)
        .gap(8)
        .border_bottom_width(1)
        .bind_style(() => @ui.style().border_color(s.color("border"))),
        history_table(s),
        ..if s.history_more.get() {
          [
            button(s, "Load more commits", () => s.load_history(false), disabled=() => {
              s.history_busy.get()
            })
            .height(28)
            .margin(8),
          ]
        } else {
          []
        },
      ],
    )
    .flex_1()
    .resizable_panel(
      () => s.history_width.get(),
      n => {
        s.history_width.set(n)
        s.persist()
      },
      minimum=360,
      maximum=900,
      label="Resize commit history",
    ),
    fill().children([commit_detail(s), diff_pane(s)]),
  ])
  .flex_1()
  .min_height(0)
  .align_items("stretch")
}

///|
fn history_table(s : Store) -> @ui.Element {
  let (viewport, set_viewport) = @ui.create_signal(@ui.VisibleRange::{
    start: 0,
    end: 35,
  })
  let width = @ui.create_memo(() => {
    let mut lanes = 1
    for row in s.graph.get() {
      lanes = lanes.max(row.width)
    }
    12 + 14 * lanes
  })
  @ui.table([])
  .columns([
    { "id": "graph", "track": "\{width()}px" },
    { "id": "subject", "track": "1fr", "rowHeader": true },
    { "id": "author", "track": "120px" },
    { "id": "time", "track": "70px", "align": "end" },
  ])
  .row_count(s.commits.get().length())
  .row_height(26)
  .flex_1()
  .bind_json(@protocol.SELECTION, () => {
    let selected : Array[Array[Int]] = []
    for i, commit in s.commits.get() {
      if commit.sha == s.commit_sha.get() {
        selected.push([i, i])
        break
      }
    }
    selected.to_json()
  })
  .on_change(data => {
    if @io.field(data, "visibleRange") is Object(_) {
      let range = @ui.visible_range(
        @io.field(data, "visibleRange"),
        s.commits.peek().length(),
      )
      set_viewport(range)
      if range.end > 0 &&
        range.end >= s.commits.peek().length() - 5 &&
        s.history_more.peek() {
        s.load_history(false)
      }
    }
    if @io.field(data, "selectedRanges") is Array(_) {
      let selection = selected_indices(
        ranges(@io.field(data, "selectedRanges")),
        s.commits.peek().length(),
      )
      if !selection.is_empty() {
        let sha = s.commits.peek()[selection[0]].sha
        if sha != s.commit_sha.peek() {
          s.select_commit(sha)
        }
      }
    }
  })
  .children_keyed(
    () => visible(s.commits.get(), viewport()),
    row => row.item.sha,
    commit => {
      @ui.table_row([])
      .row_index(commit.get().index)
      .children([
        @ui.table_cell([])
        .column("graph")
        .children([
          @ui.view([])
          .position("relative")
          .overflow("hidden")
          .height(26)
          .bind(@protocol.WIDTH, () => {
            @protocol.Number(Float::from_int(width()))
          })
          .children_keyed(
            () => {
              let index = commit.get().index
              if s.graph.get().get(index) is Some(row) {
                @git.graph_layers(row, width())
              } else {
                []
              }
            },
            layer => "\{layer.color}",
            layer => {
              @ui.svg("")
              .position("absolute")
              .left(0)
              .top(0)
              .size_full()
              .bind_value(() => layer.get().source)
              .bind_style(() => {
                let palette = s.palette()
                @ui.style().text_color(
                  hex(palette[layer.get().color % palette.length()]),
                )
              })
            },
          ),
        ]),
        @ui.table_cell([])
        .column("subject")
        .children([
          row()
          .max_width("55%")
          .gap(4)
          .overflow("hidden")
          .children_keyed(
            () => commit.get().item.refs,
            reference => reference,
            reference => {
              @ui.text(reference.get())
              .font_size(10)
              .font_weight(600)
              .line_clamp(1)
              .px(4)
              .py(1)
              .border_radius(3)
              .bind_style(() => {
                @ui.style().text_color(s.color("accent")).bg(s.color("hunk"))
              })
            },
          ),
          label(s, () => commit.get().item.subject)
          .line_clamp(1)
          .min_width(0)
          .flex_1(),
        ])
        .flex_row()
        .items_center()
        .gap(5)
        .overflow("hidden"),
        @ui.table_cell([])
        .column("author")
        .children([
          label(s, () => commit.get().item.author, tone="muted")
          .font_size(11)
          .line_clamp(1),
        ])
        .padding_left(8),
        @ui.table_cell([])
        .column("time")
        .children([
          label(
            s,
            () => @git.relative_time(commit.get().item.time, clock_now.val),
            tone="muted",
          )
          .font_size(11)
          .text_align("right"),
        ])
        .padding_right(8),
      ])
      .bind_json(@protocol.SELECTED_STYLE, () => {
        @ui.style().bg(s.color("selection")).json()
      })
      .on_context_menu(() => {
        let value = commit.peek().item
        s.window.popup_menu([
          @native.menu_action("Copy Commit Hash", () => clipboard(value.sha)),
          @native.menu_action("Copy Commit Message", () => {
            clipboard(value.subject + "\n\n" + value.body)
          }),
          @native.Separator,
          @native.menu_action("New Branch from Commit…", () => {
            s.open_dialog("branch", from=value.sha)
          }),
          @native.menu_action("Check Out Commit…", () => {
            s.confirm("Check out commit?", "This enters detached HEAD mode.", () => {
              s.mutate("Checkout", ["switch", "--detach", value.sha])
            })
          }),
        ])
      })
    },
  )
}

///|
fn commit_detail(s : Store) -> @ui.Element {
  let commit = @ui.create_memo(() => {
    s.commits.get().filter(c => c.sha == s.commit_sha.get()).get(0)
  })
  let (viewport, set_viewport) = @ui.create_signal(@ui.VisibleRange::{
    start: 0,
    end: 8,
  })
  col()
  .children(
    [
      ..if commit() is Some(_) {
        [
          col()
          .children([
            label(s, () => {
              match commit() {
                Some(c) => c.subject
                None => ""
              }
            })
            .font_size(14)
            .font_weight(700),
            label(
              s,
              () => {
                match commit() {
                  Some(c) => c.body
                  None => ""
                }
              },
              tone="secondary",
            )
            .line_height(18)
            .max_height(72)
            .overflow_y("auto"),
            label(
              s,
              () => {
                match commit() {
                  Some(c) => c.author + " · " + c.email + " · " + c.short
                  None => ""
                }
              },
              tone="secondary",
            ).font_size(11),
          ])
          .padding(12)
          .gap(5)
          .flex_shrink(0),
        ]
      } else {
        []
      },
      row()
      .children([
        caption(s, "CHANGED FILES"),
        @ui.view([]).flex_1(),
        button(s, "Show all", () => {
          s.commit_path.set("")
          s.load_diff()
        })
        .height(20)
        .font_size(11),
      ])
      .height(26)
      .px(12)
      .gap(8),
      @ui.table([])
      .columns([
        { "id": "status", "track": "32px" },
        { "id": "path", "track": "1fr", "rowHeader": true },
      ])
      .row_count(s.commit_files.get().length())
      .row_height(24)
      .bind(@protocol.HEIGHT, () => {
        @protocol.Number(
          Float::from_int(s.commit_files.get().length().min(8) * 24),
        )
      })
      .flex_shrink(0)
      .bind_json(@protocol.SELECTION, () => {
        let result : Array[Array[Int]] = []
        for i, file in s.commit_files.get() {
          if file.path == s.commit_path.get() {
            result.push([i, i])
          }
        }
        result.to_json()
      })
      .on_change(data => {
        if @io.field(data, "visibleRange") is Object(_) {
          set_viewport(
            @ui.visible_range(
              @io.field(data, "visibleRange"),
              s.commit_files.peek().length(),
            ),
          )
        }
        if @io.field(data, "selectedRanges") is Array(_) {
          let selected = selected_indices(
            ranges(@io.field(data, "selectedRanges")),
            s.commit_files.peek().length(),
          )
          if !selected.is_empty() {
            let path = s.commit_files.peek()[selected[0]].path
            if path != s.commit_path.peek() {
              s.commit_path.set(path)
              s.load_diff()
            }
          }
        }
      })
      .children_keyed(
        () => visible(s.commit_files.get(), viewport()),
        file => file.item.path,
        file => {
          @ui.table_row([])
          .row_index(file.get().index)
          .children([
            @ui.table_cell([])
            .column("status")
            .children([
              @ui.text(file.get().item.status)
              .font_size(11)
              .bind_style(() => {
                @ui.style().text_color(s.status_color(file.get().item.status))
              }),
            ])
            .padding_left(12),
            @ui.table_cell([])
            .column("path")
            .children([label(s, () => file.get().item.path).line_clamp(1)]),
          ])
          .bind_json(@protocol.SELECTED_STYLE, () => {
            @ui.style().bg(s.color("selection")).json()
          })
        },
      ),
    ],
  )
  .max_height("45%")
  .flex_shrink(0)
  .border_bottom_width(1)
  .bind_style(() => @ui.style().border_color(s.color("border")))
}
