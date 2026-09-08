///|
struct Indexed[T] {
  index : Int
  item : T
} derive(Eq)

///|
fn[T] visible(items : Array[T], range : @ui.VisibleRange) -> Array[Indexed[T]] {
  let result : Array[Indexed[T]] = []
  for i = range.start.min(items.length())
      i < range.end.min(items.length())
      i = i + 1 {
    result.push({ index: i, item: items[i], })
  }
  result
}

///|
fn ranges(value : Json) -> Array[Array[Int]] {
  @io.array(value).map(r => @io.array(r).map(@io.number))
}

///|
fn selected_indices(value : Array[Array[Int]], count : Int) -> Array[Int] {
  let seen : Map[Int, Unit] = Map([])
  for range in value {
    if range.is_empty() {
      continue
    }
    let end = range.get(1).unwrap_or(range[0]).min(count - 1)
    for i = range[0].max(0); i <= end; i = i + 1 {
      seen[i] = ()
    }
  }
  let indices = seen.keys().to_array()
  indices.sort()
  indices
}

///|
fn changes_view(s : Store) -> @ui.Element {
  row()
  .children([
    col()
    .children([change_list(s, false), change_list(s, true), composer(s)])
    .flex(1)
    .resizable_panel(
      () => s.changes_width.get(),
      n => {
        s.changes_width.set(n)
        s.persist()
      },
      minimum=220,
      maximum=650,
      label="Resize changed files",
    ),
    diff_pane(s),
  ])
  .flex(1)
  .min_height(0)
  .align_items("stretch")
}

///|
fn change_list(s : Store, staged : Bool) -> @ui.Element {
  let (viewport, set_viewport) = @reactive.create_signal(@ui.VisibleRange::{
    start: 0,
    end: 30,
  })
  let items = () => s.items(staged)
  let table = @ui.table([])
    .columns([
      { "id": "stage", "track": "28px" },
      { "id": "name", "track": "1fr", "rowHeader": true },
      { "id": "status", "track": "24px" },
    ])
    .row_count(items().length())
    .row_height(26)
    .selection_mode("multiple")
    .flex(1)
    .bind_json(@protocol.SELECTION, () => {
      let selected : Array[Array[Int]] = []
      if s.staged.get() == staged {
        for i, item in items() {
          if s.selected.get().contains(item.path) {
            selected.push([i, i])
          }
        }
      }
      selected.to_json()
    })
    .on_change(data => {
      if @io.field(data, "visibleRange") is Object(_) {
        set_viewport(
          @ui.visible_range(@io.field(data, "visibleRange"), items().length()),
        )
      }
      if @io.field(data, "selectedRanges") is Array(_) {
        let paths = selected_indices(
          ranges(@io.field(data, "selectedRanges")),
          items().length(),
        ).map(i => items()[i].path)
        if !paths.is_empty() &&
          (s.selected.peek() != paths || s.staged.peek() != staged) {
          s.select_change(staged, paths)
        }
      }
    })
    .children_keyed(
      () => visible(items(), viewport()),
      value => value.item.path,
      item => {
        @ui.table_row([])
        .row_index(item.get().index)
        .children([
          @ui.table_cell([])
          .column("stage")
          .children([
            @ui.element(@protocol.TAG_BUTTON)
            .children([
              icon(
                s,
                if staged {
                  "check"
                } else {
                  "plus"
                },
                size=12,
                tone=if staged { "accent" } else { "muted" },
              ),
            ])
            .width(24)
            .height(24)
            .flex_row()
            .items_center()
            .justify_center()
            .on("click", @protocol.CLICK_LISTENER, event => {
              event.stop_propagation()
              let path = item.peek().item.path
              s.window.with_owner(() => {
                if staged {
                  s.unstage([path])
                } else {
                  s.stage([path])
                }
              })
            }),
          ]),
          @ui.table_cell([])
          .column("name")
          .children([label(s, () => item.get().item.path).line_clamp(1)]),
          @ui.table_cell([])
          .column("status")
          .children([
            @ui.text(
              {
                let value = item.get().item
                if value.conflict {
                  "U"
                } else if staged {
                  value.index
                } else {
                  value.worktree
                }
              },
            )
            .font_size(11)
            .font_weight(600)
            .bind_style(() => {
              @ui.style().text_color(
                s.status_color(
                  if staged {
                    item.get().item.index
                  } else {
                    item.get().item.worktree
                  },
                ),
              )
            }),
          ]),
        ])
        .bind_json(@protocol.SELECTED_STYLE, () => {
          @ui.style().bg(s.color("selection")).json()
        })
        .hover(style => style.bg(s.color("hover")))
        .on_context_menu(() => change_menu(s, staged, item.peek().item))
      },
    )
  col()
  .children([
    row()
    .children([
      caption(s, if staged { "STAGED" } else { "UNSTAGED" }),
      label(s, () => "\{items().length()}", tone="muted").font_size(11),
      @ui.div([]).flex(1),
      button(
        s,
        if staged {
          "Unstage All"
        } else {
          "Stage All"
        },
        () => if staged { s.unstage_all() } else { s.stage_all() },
        disabled=() => items().is_empty() || s.busy.get() != "",
      )
      .font_size(11)
      .height(22),
    ])
    .height(32)
    .flex_shrink(0)
    .px(10)
    .gap(8)
    .border_bottom_width(1)
    .bind_style(() => @ui.style().border_color(s.color("border"))),
    table,
  ])
  .flex(1)
}

///|
fn change_menu(s : Store, staged : Bool, item : @git.Change) -> Unit {
  let paths = if s.staged.peek() == staged &&
    s.selected.peek().contains(item.path) {
    s.selected.peek()
  } else {
    [item.path]
  }
  let items : Array[@native.MenuItem] = [
    @native.menu_action(if staged { "Unstage" } else { "Stage" }, () => {
      if staged {
        s.unstage(paths)
      } else {
        s.stage(paths)
      }
    }),
    @native.menu_action(
      "Discard Changes…",
      () => s.discard(paths),
      enabled=!staged,
    ),
    @native.Separator,
    @native.menu_action("Open File", () => {
      shell("open-path", s.root.peek() + "/" + item.path)
    }),
    @native.menu_action("Reveal in Finder", () => {
      shell("reveal-path", s.root.peek() + "/" + item.path)
    }),
    @native.menu_action("Copy Relative Path", () => clipboard(item.path)),
  ]
  if item.conflict {
    items.append([
      @native.Separator,
      @native.menu_action("Resolve Using Ours…", () => {
        s.resolve_conflict(item.path, "ours")
      }),
      @native.menu_action("Resolve Using Theirs…", () => {
        s.resolve_conflict(item.path, "theirs")
      }),
    ])
  }
  s.window.popup_menu(items)
}

///|
fn composer(s : Store) -> @ui.Element {
  col()
  .children([
    field(s, "Commit subject", s.subject).on_input(text => {
      s.subject.set(text)
      s.persist()
    }),
    field(s, "Description (optional)", s.body, multiline=true).on_input(text => {
      s.body.set(text)
      s.persist()
    }),
    row()
    .children([
      check(s, "Amend", () => s.amend.get(), v => s.set_amend(v)),
      check(s, "Sign off", () => s.signoff.get(), v => s.signoff.set(v)),
    ])
    .gap(10),
    row()
    .children([
      button(s, "Generate", () => s.generate(""), disabled=() => {
        agents.get().is_empty() || s.busy.get() != ""
      })
      .flex(1)
      .on_context_menu(() => {
        s.window.popup_menu([
          @native.menu_action("Generate with instructions…", () => {
            s.open_dialog("generate")
          }),
          @native.submenu(
            "Agent",
            agents
            .peek()
            .map(agent => {
              @native.menu_action(agent, () => {
                preferred_agent.set(agent)
                save()
              })
            }),
          ),
        ])
      }),
      button(s, "Commit", () => s.commit(), primary=true, disabled=() => {
        s.subject.get().trim().is_empty() ||
        s.busy.get() != "" ||
        (s.items(true).is_empty() && !s.amend.get())
      }).flex(1),
    ])
    .gap(6),
  ])
  .flex_shrink(0)
  .padding(12)
  .gap(8)
  .border_top_width(1)
  .bind_style(() => @ui.style().border_color(s.color("border")))
}

///|
fn diff_pane(s : Store) -> @ui.Element {
  fill().children(
    [
      row()
      .children(
        [
          label(s, () => {
            if s.diff_busy.get() {
              "Loading diff…"
            } else if s.diff.get().length() == 1 {
              s.diff.get()[0].path
            } else {
              "\{s.diff.get().length()} files"
            }
          })
          .font_weight(600)
          .line_clamp(1)
          .flex(1),
          ..if s.view.get() == "changes" && !s.selected.get().is_empty() {
            [
              row()
              .children([
                button(
                  s,
                  "Discard…",
                  () => s.discard(s.selected.peek()),
                  danger=true,
                  disabled=() => s.staged.get() || s.busy.get() != "",
                ),
                button(
                  s,
                  "",
                  () => s.apply_selection(),
                  read=Some(() => {
                    if s.staged.get() {
                      "Unstage selection"
                    } else {
                      "Stage selection"
                    }
                  }),
                  disabled=() => {
                    s.diff_selection.get().is_empty() || s.busy.get() != ""
                  },
                ),
                button(
                  s,
                  "",
                  () => {
                    if s.staged.peek() {
                      s.unstage(s.selected.peek())
                    } else {
                      s.stage(s.selected.peek())
                    }
                  },
                  primary=true,
                  read=Some(() => {
                    if s.staged.get() {
                      "Unstage"
                    } else {
                      "Stage"
                    }
                  }),
                  disabled=() => s.busy.get() != "",
                ),
              ])
              .gap(6),
            ]
          } else {
            []
          },
        ],
      )
      .height(40)
      .flex_shrink(0)
      .px(12)
      .gap(8)
      .border_bottom_width(1)
      .bind_style(() => @ui.style().border_color(s.color("border"))),
      ..if !s.rows.get().is_empty() {
        [diff_table(s)]
      } else {
        [
          empty(
            s, "No diff selected", "Select a file or commit to inspect its changes.",
          ),
        ]
      },
    ],
  )
}

///|
fn diff_table(s : Store) -> @ui.Element {
  let (viewport, set_viewport) = @reactive.create_signal(@ui.VisibleRange::{
    start: 0,
    end: 50,
  })
  @ui.table([])
  .columns([
    { "id": "old", "track": "46px", "align": "end" },
    { "id": "new", "track": "46px", "align": "end" },
    { "id": "text", "track": "1fr" },
  ])
  .row_count(s.rows.get().length())
  .row_height(20)
  .selection_mode("multiple")
  .flex(1)
  .font_family("monospace")
  .font_size(12)
  .bind_json(@protocol.SELECTION, () => s.diff_selection.get().to_json())
  .on_change(data => {
    if @io.field(data, "visibleRange") is Object(_) {
      set_viewport(
        @ui.visible_range(
          @io.field(data, "visibleRange"),
          s.rows.peek().length(),
        ),
      )
    }
    if @io.field(data, "selectedRanges") is Array(_) {
      s.diff_selection.set(ranges(@io.field(data, "selectedRanges")))
    }
  })
  .on_context_menu(() => {
    s.window.popup_menu([
      @native.menu_action(
        if s.staged.peek() {
          "Unstage Selected Lines"
        } else {
          "Stage Selected Lines"
        },
        () => s.apply_selection(),
        enabled=s.view.peek() == "changes" &&
          !s.diff_selection.peek().is_empty(),
      ),
      @native.menu_action(
        "Discard Selected Lines…",
        () => s.apply_selection(discard=true),
        enabled=s.view.peek() == "changes" &&
          !s.staged.peek() &&
          !s.diff_selection.peek().is_empty(),
      ),
      @native.menu_action("Copy Selected Lines", () => {
        clipboard(
          selected_indices(s.diff_selection.peek(), s.rows.peek().length())
          .map(i => s.rows.peek()[i].text)
          .join("\n"),
        )
      }),
    ])
  })
  .children_keyed(
    () => visible(s.rows.get(), viewport()),
    row => "\{row.index}",
    indexed => {
      let value = () => indexed.get().item
      let tone = () => {
        match value().kind {
          "+" => "success"
          "-" => "danger"
          "hunk" => "hunk-text"
          _ => "text"
        }
      }
      let bg = () => {
        match value().kind {
          "+" => "added"
          "-" => "removed"
          "hunk" => "hunk"
          "file" => "alt"
          _ => "transparent"
        }
      }
      let gutter = () => {
        match value().kind {
          "+" => "added-gutter"
          "-" => "removed-gutter"
          "hunk" => "hunk"
          _ => "alt"
        }
      }
      @ui.table_row([])
      .row_index(indexed.get().index)
      .children([
        @ui.table_cell([])
        .column("old")
        .children([
          label(
            s,
            () => if value().old > 0 { "\{value().old}" } else { "" },
            tone="muted",
          )
          .font_size(11)
          .text_align("right"),
        ])
        .padding_right(6)
        .bind_style(() => @ui.style().bg(s.color(gutter()))),
        @ui.table_cell([])
        .column("new")
        .children([
          label(
            s,
            () => if value().new > 0 { "\{value().new}" } else { "" },
            tone="muted",
          )
          .font_size(11)
          .text_align("right"),
        ])
        .padding_right(6)
        .bind_style(() => @ui.style().bg(s.color(gutter()))),
        @ui.table_cell([])
        .column("text")
        .children(
          [
            ..if value().kind == "+" || value().kind == "-" {
              [
                icon(
                  s,
                  if value().kind == "+" {
                    "plus"
                  } else {
                    "minus"
                  },
                  size=12,
                  tone=tone(),
                ),
              ]
            } else {
              []
            },
            @ui.text(
              {
                let text = value().text
                if text.length() > 4096 {
                  text.view(end_offset=4096).to_owned() +
                  "… [line truncated in preview]"
                } else {
                  text
                }
              },
            )
            .white_space("pre")
            .flex(1)
            .min_width(0)
            .line_clamp(1)
            .bind_style(() => {
              @ui.style()
              .text_color(s.color(tone()))
              .font_weight(if value().kind == "file" { 600 } else { 400 })
            }),
            ..if value().kind == "hunk" && s.view.get() == "changes" {
              [
                button(
                  s,
                  "",
                  () => s.apply_hunk(value().file, value().hunk),
                  read=Some(() => {
                    if s.staged.get() {
                      "Unstage hunk"
                    } else {
                      "Stage hunk"
                    }
                  }),
                  disabled=() => s.busy.get() != "",
                )
                .height(18)
                .font_size(11)
                .font_family("system-ui")
                .px(6)
                .on_context_menu(() => {
                  s.window.popup_menu([
                    @native.menu_action(
                      "Discard Hunk…",
                      () => {
                        s.apply_hunk(value().file, value().hunk, discard=true)
                      },
                      enabled=!s.staged.peek() && s.busy.peek() == "",
                    ),
                  ])
                }),
              ]
            } else {
              []
            },
          ],
        )
        .flex_row()
        .items_center()
        .gap(5)
        .px(8)
        .overflow("hidden"),
      ])
      .bind_style(() => @ui.style().bg(s.color(bg())))
      .bind_json(@protocol.SELECTED_STYLE, () => {
        @ui.style().bg(s.color("selection")).json()
      })
    },
  )
}
