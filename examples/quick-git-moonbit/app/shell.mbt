///|
fn app_view(s : Store) -> @ui.Element {
  @ui.div(
    [
      ..if s.root.get() != "" {
        [
          row()
          .children([
            sidebar(s).resizable_panel(
              () => s.sidebar_width.get(),
              n => {
                s.sidebar_width.set(n)
                s.persist()
              },
              minimum=180,
              maximum=420,
              label="Resize sidebar",
            ),
            fill()
            .children(
              [
                toolbar(s),
                ..if s.view.get() == "changes" {
                  [changes_view(s)]
                } else {
                  []
                },
                ..if s.view.get() == "history" {
                  [history_view(s)]
                } else {
                  []
                },
                ..if s.view.get() == "branches" {
                  [branches_view(s)]
                } else {
                  []
                },
                ..if s.view.get() == "worktrees" {
                  [worktrees_view(s)]
                } else {
                  []
                },
                ..if s.view.get() == "stashes" {
                  [stashes_view(s)]
                } else {
                  []
                },
              ],
            )
            .bind_style(() => @ui.style().bg(s.color("content"))),
          ])
          .size_full()
          .align_items("stretch"),
        ]
      } else {
        [welcome(s)]
      },
      ..if s.dialog.get() != "" {
        [dialogs(s)]
      } else {
        []
      },
      ..if s.notice.get() != "" {
        [
          row()
          .children([
            label(s, () => s.notice.get())
            .flex_grow(1)
            .min_width(0)
            .line_height(18),
            @ui.element(@protocol.TAG_BUTTON)
            .children([icon(s, "close")])
            .width(22)
            .height(22)
            .on_click(() => s.notice.set("")),
          ])
          .position("absolute")
          .right(16)
          .bottom(16)
          .max_width(560)
          .gap(12)
          .padding(12)
          .border_width(1)
          .border_radius(8)
          .bind_style(() => {
            @ui.style().bg(s.color("raised")).border_color(s.color("strong"))
          }),
        ]
      } else {
        []
      },
    ],
  )
  .size_full()
  .flex_row()
  .position("relative")
  .min_width(0)
  .min_height(0)
  .font_size(13)
  .bind_style(() => @ui.style().text_color(s.color("text")))
}

///|
fn welcome(s : Store) -> @ui.Element {
  fill()
  .children([
    row().height(52).flex_shrink(0).app_region("drag"),
    fill()
    .children(
      [
        icon(s, "branch", size=44, tone="accent"),
        label(s, () => "Quick Git").font_size(28).font_weight(700),
        label(
          s,
          () => "A native Git client, built with MoonBit and QuickGUI.",
          tone="secondary",
        ),
        button(s, "Open Repository…", open_dialog, primary=true).height(32),
        ..if s.opening.get() {
          [label(s, () => "Opening repository…", tone="muted")]
        } else {
          []
        },
        ..if !recents.get().is_empty() {
          [
            col()
            .children([caption(s, "RECENT REPOSITORIES").margin_bottom(6)])
            .width(480)
            .max_height(280)
            .overflow_y("auto")
            .gap(4)
            .margin_top(18)
            .children_keyed(() => recents.get(), p => p, path => {
              @ui.element(@protocol.TAG_BUTTON)
              .children([
                icon(s, "folder"),
                label(s, () => @git.base(path.get())).font_weight(600),
                label(s, () => @git.parent(path.get()), tone="muted")
                .flex_1()
                .line_clamp(1),
              ])
              .flex_row()
              .items_center()
              .height(36)
              .px(10)
              .border_radius(6)
              .gap(8)
              .on_click(() => open_repository(path.peek()))
              .hover(style => style.bg(s.color("hover")))
              .on_context_menu(() => {
                s.window.popup_menu([
                  @native.menu_action("Open Repository", () => {
                    open_repository(path.peek())
                  }),
                  @native.menu_action("Remove from Recent", () => {
                    recents.set(recents.peek().filter(p => p != path.peek()))
                    save()
                  }),
                ])
              })
            }),
          ]
        } else {
          []
        },
      ],
    )
    .items_center()
    .justify_center()
    .gap(18)
    .padding(30),
  ])
  .size_full()
  .bind_style(() => @ui.style().bg(s.color("content")))
}

///|
fn sidebar_row(
  s : Store,
  title : () -> String,
  selected : () -> Bool,
  click : () -> Unit,
) -> @ui.Element {
  @ui.element(@protocol.TAG_BUTTON)
  .children([label(s, title).flex_1().min_width(0).font_size(13).line_clamp(1)])
  .flex_row()
  .width("100%")
  .min_width(0)
  .height(28)
  .flex_shrink(0)
  .items_center()
  .gap(8)
  .padding_left(10)
  .padding_right(8)
  .border_radius(6)
  .cursor("default")
  .user_select("none")
  .app_region("no-drag")
  .on_click(click)
  .bind_style(() => line_style(s, selected))
  .bind_json(@protocol.HOVER_STYLE, () => {
    @ui.style()
    .bg(s.color(if selected() { "selection" } else { "hover" }))
    .json()
  })
}

///|
fn navigation(
  s : Store,
  title : String,
  view : String,
  count : () -> Int,
) -> @ui.Element {
  sidebar_row(s, () => title, () => s.view.get() == view, () => s.set_view(view)).children(
    [
      ..if count() > 0 {
        [label(s, () => "\{count()}", tone="muted").font_size(11)]
      } else {
        []
      },
    ],
  )
}

///|
fn sidebar_section(
  s : Store,
  title : String,
  view : String,
  count : () -> Int,
  action : () -> Unit,
) -> @ui.Element {
  row()
  .children([
    @ui.element(@protocol.TAG_BUTTON)
    .children(
      [
        @ui.text(title)
        .flex_1()
        .min_width(0)
        .font_size(11)
        .font_weight(600)
        .bind_style(() => {
          @ui.style().text_color(
            s.color(if s.view.get() == view { "accent" } else { "muted" }),
          )
        }),
        ..if count() > 0 {
          [
            label(s, () => "\{count()}", tone="muted")
            .font_size(11)
            .font_weight(600),
          ]
        } else {
          []
        },
      ],
    )
    .flex_row()
    .items_center()
    .flex_1()
    .min_width(0)
    .gap(6)
    .height(22)
    .padding_left(9)
    .padding_right(6)
    .border_radius(6)
    .on_click(() => s.set_view(view))
    .hover(style => style.bg(s.color("hover"))),
    @ui.element(@protocol.TAG_BUTTON)
    .children([icon(s, "plus")])
    .width(24)
    .height(24)
    .flex_row()
    .items_center()
    .justify_center()
    .border_radius(6)
    .property(@protocol.ACCESSIBILITY_LABEL, @protocol.Text("Add " + title))
    .on_click(action)
    .hover(style => style.bg(s.color("hover"))),
  ])
  .gap(4)
  .margin_top(14)
  .padding_right(2)
  .flex_shrink(0)
}

///|
fn repository_picker(s : Store) -> @ui.Element {
  @ui.element(@protocol.TAG_BUTTON)
  .children([
    row()
    .children([icon(s, "branch", size=18, tone="white")])
    .width(28)
    .height(28)
    .flex_shrink(0)
    .justify_center()
    .border_radius(7)
    .bind_style(() => @ui.style().bg(s.color("accent"))),
    col()
    .children([
      label(s, () => @git.base(s.root.get()))
      .font_size(13)
      .font_weight(700)
      .line_clamp(1),
      label(
        s,
        () => {
          let status = s.status.get()
          if status.branch == "(detached)" {
            "Detached HEAD"
          } else {
            status.branch
          }
        },
        tone="muted",
      )
      .font_size(11)
      .line_clamp(1),
    ])
    .flex_1()
    .gap(1),
    icon(s, "chevron", tone="muted"),
  ])
  .flex_row()
  .items_center()
  .gap(9)
  .height(44)
  .flex_shrink(0)
  .px(8)
  .margin_bottom(6)
  .border_radius(8)
  .app_region("no-drag")
  .property(@protocol.ACCESSIBILITY_LABEL, @protocol.Text("Repository actions"))
  .hover(style => style.bg(s.color("hover")))
  .on_click(() => {
    let items = recents
      .peek()
      .map(path => {
        @native.menu_action(@git.base(path), () => open_repository(path))
      })
    if !items.is_empty() {
      items.push(@native.Separator)
    }
    items.append([
      @native.menu_action("Open Repository\u2026", open_dialog),
      @native.Separator,
      @native.menu_action("Reveal in Finder", () => {
        shell("reveal-path", s.root.peek())
      }),
      @native.menu_action("Copy Path", () => clipboard(s.root.peek())),
      @native.Separator,
      @native.menu_action("Close Repository", () => s.close()),
    ])
    s.window.popup_menu(items, done=result => {
      if result is Err(error) {
        s.error(error)
      }
    })
  })
}

///|
fn sidebar(s : Store) -> @ui.Element {
  let local_branches = () => {
    s.branches.get().filter(b => !b.remote && b.full.has_prefix("refs/heads/"))
  }
  col()
  .children([
    row().height(52).flex_shrink(0).app_region("drag"),
    col()
    .children([
      repository_picker(s),
      navigation(s, "Changes", "changes", () => s.status.get().entries.length()),
      navigation(s, "History", "history", () => 0),
      sidebar_section(
        s,
        "Branches",
        "branches",
        () => local_branches().length(),
        () => s.open_dialog("branch"),
      ),
    ])
    .flex_1()
    .overflow_y("auto")
    .gap(2)
    .padding_top(4)
    .px(10)
    .padding_bottom(12)
    .children_keyed(
      () => {
        let branches = local_branches()
        visible(branches, { start: 0, end: 8, }).map(row => row.item)
      },
      branch => branch.full,
      branch => {
        sidebar_row(s, () => branch.get().name, () => false, () => {
          let current = branch.peek()
          if !current.current {
            let linked = s.worktrees
              .peek()
              .filter(w => w.branch == current.name && w.path != s.root.peek())
            if linked.get(0) is Some(worktree) {
              s.open(worktree.path)
            } else {
              s.switch_branch(current)
            }
          }
        })
        .children(
          [..if branch.get().current { [icon(s, "check")] } else { [] }],
        )
        .on_context_menu(() => branch_menu(s, branch.peek()))
      },
    )
    .children([
      sidebar_section(
        s,
        "Worktrees",
        "worktrees",
        () => s.worktrees.get().length(),
        () => s.open_dialog("worktree"),
      ),
    ])
    .children_keyed(() => s.worktrees.get(), worktree => worktree.path, worktree => {
      sidebar_row(
        s,
        () => {
          let value = worktree.get()
          if value.branch != "" {
            value.branch
          } else if value.head.length() >= 7 {
            value.head.view(end_offset=7).to_owned() + " (detached)"
          } else {
            @git.base(value.path)
          }
        },
        () => false,
        () => {
          if worktree.peek().path != s.root.peek() {
            s.open(worktree.peek().path)
          }
        },
      ).children(
        [
          ..if worktree.get().path == s.root.get() {
            [icon(s, "check")]
          } else {
            []
          },
        ],
      )
    })
    .children([
      sidebar_section(s, "Stashes", "stashes", () => s.stashes.get().length(), () => {
        s.open_dialog("stash")
      }),
    ])
    .children_keyed(
      () => visible(s.stashes.get(), { start: 0, end: 5, }).map(row => row.item),
      stash => stash.name,
      stash => {
        sidebar_row(s, () => stash.get().summary, () => false, () => {
          s.set_view("stashes")
        }).children([
          label(
            s,
            () => @git.relative_time(stash.get().time, clock_now.val),
            tone="muted",
          ).font_size(11),
        ])
      },
    ),
  ])
  .height("100%")
}

///|
fn toolbar(s : Store) -> @ui.Element {
  fn tool(title : String, name : String, action : () -> Unit) -> @ui.Element {
    button(s, title, action, disabled=() => s.busy.get() != "")
    .children([icon(s, name)])
    .height(28)
  }
  row()
  .children(
    [
      icon(s, "branch", size=16),
      label(s, () => s.status.get().branch).line_clamp(1).max_width(200),
      ..if s.status.get().ahead + s.status.get().behind > 0 {
        [
          label(
            s,
            () => {
              "\{s.status.get().ahead} ahead · \{s.status.get().behind} behind"
            },
            tone="muted",
          ).font_size(11),
        ]
      } else {
        []
      },
      @ui.div([]).flex_1(),
      ..if s.busy.get() != "" {
        [
          row()
          .children([
            label(s, () => s.busy.get() + "…", tone="secondary").font_size(12),
            button(s, "Cancel", () => s.cancel()),
          ])
          .gap(8),
        ]
      } else {
        []
      },
      tool("Fetch", "fetch", () => s.mutate("Fetch", ["fetch", "--prune"])),
      tool("Pull", "pull", () => s.mutate("Pull", ["pull", "--no-edit"])),
      tool("Push", "push", () => s.push()),
      tool("", "refresh", () => s.refresh()).width(32).px(7),
    ],
  )
  .height(52)
  .flex_shrink(0)
  .padding_left(18)
  .padding_right(12)
  .gap(8)
  .app_region("drag")
  .border_bottom_width(1)
  .bind_style(() => @ui.style().border_color(s.color("border")))
}
