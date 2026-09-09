///|
fn branches_view(s : Store) -> @ui.Element {
  let search = @reactive.signal("")
  let (viewport, set_viewport) = @ui.create_signal(@ui.VisibleRange::{
    start: 0,
    end: 40,
  })
  let branches = () => {
    s.branches
    .get()
    .filter(b => b.name.to_lower().contains(search.get().to_lower()))
  }
  fill()
  .children([
    row()
    .children([
      caption(s, "BRANCHES AND TAGS"),
      @ui.view([]).flex_1(),
      field(s, "Filter branches", search).width(180),
      button(s, "New Branch", () => s.open_dialog("branch"), primary=true),
    ])
    .gap(10),
    @ui.table([])
    .columns([
      { "id": "name", "track": "1fr", "rowHeader": true },
      { "id": "upstream", "track": "200px" },
      { "id": "sha", "track": "85px" },
    ])
    .row_count(branches().length())
    .row_height(32)
    .flex_1()
    .on_change(data => {
      if @io.field(data, "visibleRange") is Object(_) {
        set_viewport(
          @ui.visible_range(
            @io.field(data, "visibleRange"),
            branches().length(),
          ),
        )
      }
    })
    .children_keyed(
      () => visible(branches(), viewport()),
      row => row.item.full,
      branch => {
        @ui.table_row([])
        .row_index(branch.get().index)
        .children([
          @ui.table_cell([])
          .column("name")
          .children(
            [
              icon(s, "branch"),
              label(s, () => branch.get().item.name).flex_1().line_clamp(1),
              ..if branch.get().item.current {
                [icon(s, "check", tone="accent")]
              } else {
                []
              },
            ],
          )
          .flex_row()
          .items_center()
          .gap(8)
          .px(6),
          @ui.table_cell([])
          .column("upstream")
          .children([
            label(
              s,
              () => branch.get().item.upstream + " " + branch.get().item.track,
              tone="muted",
            )
            .font_size(11)
            .line_clamp(1),
          ]),
          @ui.table_cell([])
          .column("sha")
          .children([
            label(s, () => branch.get().item.sha, tone="muted").font_size(11),
          ]),
        ])
        .bind_style(() => line_style(s, () => branch.get().item.current))
        .on_double_click(() => s.switch_branch(branch.peek().item))
        .on_context_menu(() => branch_menu(s, branch.peek().item))
      },
    ),
  ])
  .padding(12)
  .gap(8)
}

///|
fn worktrees_view(s : Store) -> @ui.Element {
  fill()
  .children([
    row()
    .children([
      caption(s, "WORKTREES"),
      @ui.view([]).flex_1(),
      button(s, "New Worktree", () => s.open_dialog("worktree"), primary=true),
    ])
    .height(30),
  ])
  .padding(12)
  .gap(8)
  .overflow_y("auto")
  .children_keyed(() => s.worktrees.get(), w => w.path, worktree => {
    let w = () => worktree.get()
    @ui.element(@protocol.TAG_BUTTON)
    .children(
      [
        icon(s, "branch", size=18),
        col()
        .children([
          label(s, () => {
            if w().branch == "" {
              "Detached HEAD"
            } else {
              w().branch
            }
          }).font_weight(600),
          label(s, () => w().path, tone="muted").font_size(11).line_clamp(1),
        ])
        .flex_1()
        .gap(3),
        ..if w().locked {
          [caption(s, "Locked")]
        } else {
          []
        },
        ..if w().prunable {
          [caption(s, "Prunable")]
        } else {
          []
        },
        ..if w().path == s.root.get() {
          [icon(s, "check", tone="accent")]
        } else {
          []
        },
      ],
    )
    .flex_row()
    .items_center()
    .padding(10)
    .gap(10)
    .border_radius(6)
    .bind_style(() => line_style(s, () => w().path == s.root.get()))
    .on_click(() => s.open(w().path))
    .on_context_menu(() => {
      s.window.popup_menu([
        @native.menu_action("Open in New Window", () => open_window(w().path)),
        @native.menu_action("Reveal in Finder", () => {
          shell("reveal-path", w().path)
        }),
        @native.menu_action("Copy Path", () => clipboard(w().path)),
        @native.Separator,
        @native.menu_action(
          "Remove Worktree…",
          () => {
            s.confirm("Remove worktree?", w().path, () => {
              s.mutate("Remove worktree", ["worktree", "remove", "--", w().path])
            })
          },
          enabled=w().path != s.root.peek(),
        ),
        @native.menu_action("Prune Stale Worktrees", () => {
          s.mutate("Prune worktrees", ["worktree", "prune"])
        }),
      ])
    })
  })
}

///|
fn stashes_view(s : Store) -> @ui.Element {
  row()
  .children([
    col()
    .children([
      row()
      .children([
        caption(s, "STASHES"),
        @ui.view([]).flex_1(),
        button(s, "Stash Changes", () => s.open_dialog("stash"), primary=true, disabled=() => {
          s.status.get().entries.is_empty()
        }),
      ])
      .height(30)
      .gap(8),
    ])
    .width(320)
    .flex_shrink(0)
    .padding(12)
    .gap(8)
    .overflow_y("auto")
    .border_right_width(1)
    .bind_style(() => @ui.style().border_color(s.color("border")))
    .children_keyed(() => s.stashes.get(), stash => stash.name, stash => {
      let value = () => stash.get()
      col()
      .children([
        label(s, () => value().summary).line_clamp(2),
        label(
          s,
          () => {
            value().name +
            " · " +
            @git.relative_time(value().time, clock_now.val)
          },
          tone="muted",
        ).font_size(11),
        row()
        .children([
          button(s, "Apply", () => {
            s.mutate("Apply stash", ["stash", "apply", value().name])
          }),
          button(s, "Pop", () => {
            s.mutate("Pop stash", ["stash", "pop", value().name])
          }),
          button(
            s,
            "Drop…",
            () => {
              s.confirm("Drop stash?", value().summary, () => {
                s.mutate("Drop stash", ["stash", "drop", value().name])
              })
            },
            danger=true,
          ),
        ])
        .gap(6),
      ])
      .padding(10)
      .gap(8)
      .border_radius(6)
      .bind_style(() => line_style(s, () => value().name == s.stash.get()))
      .on_click(() => {
        s.stash.set(value().name)
        s.load_diff()
      })
      .on_context_menu(() => {
        s.window.popup_menu([
          @native.menu_action("Apply", () => {
            s.mutate("Apply stash", ["stash", "apply", value().name])
          }),
          @native.menu_action("Pop", () => {
            s.mutate("Pop stash", ["stash", "pop", value().name])
          }),
          @native.menu_action("Drop…", () => {
            s.confirm("Drop stash?", value().summary, () => {
              s.mutate("Drop stash", ["stash", "drop", value().name])
            })
          }),
        ])
      })
    })
    .children(
      [
        ..if s.stashes.get().is_empty() {
          [empty(s, "No stashes", "Save changes for later with Stash Changes.")]
        } else {
          []
        },
      ],
    ),
    diff_pane(s),
  ])
  .flex_1()
  .min_height(0)
  .align_items("stretch")
}

///|
fn dialogs(s : Store) -> @ui.Element {
  let kind = s.dialog.peek()
  let name = @reactive.signal(
    if kind == "rename" {
      s.dialog_from.peek()
    } else {
      ""
    },
  )
  let from = @reactive.signal(
    if s.dialog_from.peek() == "" {
      s.status.peek().branch
    } else {
      s.dialog_from.peek()
    },
  )
  let path = @reactive.signal("")
  let (checkout, set_checkout) = @ui.create_signal(true)
  let (untracked, set_untracked) = @ui.create_signal(true)
  let (keep_index, set_keep_index) = @ui.create_signal(false)
  let (new_branch, set_new_branch) = @ui.create_signal(true)
  let title = match kind {
    "branch" => "New Branch"
    "rename" => "Rename Branch"
    "worktree" => "New Worktree"
    "stash" => "Stash Changes"
    _ => "Generate Commit Message"
  }
  let problem = () => {
    if (kind == "branch" || kind == "worktree") && from.get().has_prefix("-") {
      return "A starting revision cannot begin with a hyphen"
    }
    if kind == "branch" ||
      kind == "rename" ||
      (kind == "worktree" && new_branch()) {
      @git.branch_problem(name.get())
    } else {
      ""
    }
  }
  fn submit() -> Unit {
    if problem() != "" {
      return
    }
    let value = name.peek().trim().to_owned()
    if kind == "branch" {
      let args = if @ui.untrack(checkout) {
        ["switch", "-c", value]
      } else {
        ["branch", value]
      }
      if from.peek() != "" {
        args.push(from.peek())
      }
      s.mutate("Create branch", args)
    } else if kind == "rename" {
      s.mutate("Rename branch", [
        "branch",
        "-m",
        "--",
        s.dialog_from.peek(),
        value,
      ])
    } else if kind == "worktree" {
      let location = if path.peek() != "" {
        path.peek()
      } else {
        @git.parent(s.root.peek()) +
        "/" +
        @git.base(s.root.peek()) +
        "-" +
        value.replace_all(old="/", new="-")
      }
      let args = ["worktree", "add"]
      if @ui.untrack(new_branch) {
        args.append(["-b", value])
      }
      args.append(["--", location])
      if from.peek() != "" {
        args.push(from.peek())
      }
      s.mutate("Create worktree", args)
    } else if kind == "stash" {
      let args = ["stash", "push"]
      if value != "" {
        args.append(["-m", value])
      }
      if @ui.untrack(untracked) {
        args.push("--include-untracked")
      }
      if @ui.untrack(keep_index) {
        args.push("--keep-index")
      }
      s.mutate("Stash", args)
    } else {
      s.generate(value)
    }
    s.dialog.set("")
  }
  let portal = @ui.view([])
  let scope = "quick-git-dialog-\{portal.native_node().id}"
  fn part(node : @ui.Element, name : String) -> @ui.Element {
    node
    .part(name, scope~)
    .property(@protocol.VARIANT, @protocol.Text("dialog"))
    .property(@protocol.OPEN, @protocol.Boolean(true))
  }
  let body = col()
    .children([
      caption(
        s,
        if kind == "stash" {
          "MESSAGE (OPTIONAL)"
        } else if kind == "generate" {
          "INSTRUCTIONS (OPTIONAL)"
        } else {
          "BRANCH NAME"
        },
      ),
      field(
        s,
        if kind == "generate" {
          "Mention the motivation for this change…"
        } else {
          "Name"
        },
        name,
        multiline=kind == "generate",
      ).on_submit(_ => submit()),
    ])
    .gap(10)
  if kind == "branch" || kind == "worktree" {
    ignore(
      body.children([
        caption(s, "STARTING POINT"),
        field(s, "Branch, tag, or commit", from),
      ]),
    )
  }
  if kind == "branch" {
    ignore(
      body.children([
        check(s, "Switch to the new branch", () => checkout(), v => {
          set_checkout(v)
        }),
      ]),
    )
  }
  if kind == "worktree" {
    ignore(
      body.children([
        check(s, "Create a new branch", () => new_branch(), v => {
          set_new_branch(v)
        }),
        caption(s, "DIRECTORY (OPTIONAL)"),
        row()
        .children([
          field(s, "Choose a directory or use the suggested path", path).flex_1(),
          button(s, "Browse…", () => {
            s.window.dialog(
              1,
              {
                "directories": true,
                "files": false,
                "title": "Worktree Directory",
              },
              result => if result is Ok(Array([String(p), ..])) { path.set(p) },
            )
          }),
        ])
        .gap(8),
      ]),
    )
  }
  if kind == "stash" {
    ignore(
      body.children([
        check(s, "Include untracked files", () => untracked(), v => {
          set_untracked(v)
        }),
        check(s, "Keep staged changes", () => keep_index(), v => {
          set_keep_index(v)
        }),
      ]),
    )
  }
  ignore(
    body.children(
      [
        ..if name.get() != "" && problem() != "" {
          [label(s, problem, tone="danger").font_size(11)]
        } else {
          []
        },
      ],
    ),
  )
  part(portal, "dialog")
  .children([
    part(@ui.view([]), "dialog-backdrop")
    .position("absolute")
    .top(0)
    .bottom(0)
    .left(0)
    .right(0)
    .bind_style(() => @ui.style().bg(s.color("scrim"))),
    part(col(), "dialog-popup")
    .children([
      part(label(s, () => title), "dialog-title").font_size(15).font_weight(700),
      part(body, "dialog-viewport"),
      row()
      .children([
        button(s, "Cancel", () => s.dialog.set("")),
        button(
          s,
          match kind {
            "generate" => "Generate"
            "rename" => "Rename"
            "stash" => "Stash"
            _ => "Create"
          },
          submit,
          primary=true,
          disabled=() => problem() != "" || s.busy.get() != "",
        ),
      ])
      .justify_content("flex-end")
      .gap(8),
    ])
    .width(460)
    .padding(20)
    .gap(14)
    .border_radius(10)
    .border_width(1)
    .bind_style(() => {
      @ui.style().bg(s.color("raised")).border_color(s.color("strong"))
    })
    .property(@protocol.DISMISS_ON_ESCAPE, @protocol.Boolean(true))
    .property(@protocol.DISMISS_ON_POINTER_OUTSIDE, @protocol.Boolean(true))
    .on("dismiss", @protocol.DISMISS_LISTENER, _ => s.dialog.set("")),
  ])
  .position("absolute")
  .top(0)
  .bottom(0)
  .left(0)
  .right(0)
  .flex_col()
  .items_center()
  .justify_center()
  .property(@protocol.EXIT_DURATION, @protocol.Number(0))
}
