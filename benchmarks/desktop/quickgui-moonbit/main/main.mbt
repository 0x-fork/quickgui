///|
struct Issue {
  id : String
  title : String
  project : String
  owner : String
  priority : String
  description : String
  status : @reactive.Signal[String]
  notes : @reactive.Signal[String]
}

///|
fn field(item : Json, key : String) -> String {
  match item {
    Object(fields) =>
      match fields.get(key) {
        Some(String(value)) => value
        _ => abort("Missing issue field: \{key}")
      }
    _ => abort("Invalid issue record")
  }
}

///|
fn load_issues() -> Array[Issue] {
  let json = @json.parse(issues_json) catch {
    _ => abort("Invalid issue dataset")
  }
  let result : Array[Issue] = []
  if json is Array(items) {
    for item in items {
      result.push({
        id: field(item, "id"),
        title: field(item, "title"),
        project: field(item, "project"),
        owner: field(item, "owner"),
        priority: field(item, "priority"),
        description: field(item, "description"),
        status: @reactive.signal(field(item, "status")),
        notes: @reactive.signal(field(item, "notes")),
      })
    }
  }
  guard result.length() == 1000 else {
    abort("Expected 1,000 benchmark issues")
  }
  result
}

///|
fn ink() -> @ui.Color {
  @ui.rgb8(32, 36, 44)
}

///|
fn muted() -> @ui.Color {
  @ui.rgb8(115, 124, 140)
}

///|
fn line() -> @ui.Color {
  @ui.rgb8(226, 229, 235)
}

///|
fn accent() -> @ui.Color {
  @ui.rgb8(40, 91, 212)
}

///|
fn white() -> @ui.Color {
  @ui.rgb8(255, 255, 255)
}

///|
fn col() -> @ui.Element {
  @ui.div([]).flex_col().min_width(0).min_height(0)
}

///|
fn row() -> @ui.Element {
  @ui.div([]).flex_row().items_center().min_width(0)
}

///|
fn caption(read : () -> String) -> @ui.Element {
  @ui.text(read()).font_size(12).line_height(18).text_color(muted())
}

///|
fn control(
  label : String,
  click : () -> Unit,
  disabled : () -> Bool,
) -> @ui.Element {
  @ui.button(label)
  .on_click(click)
  .flex_row()
  .items_center()
  .justify_center()
  .height(32)
  .px(12)
  .border_width(1)
  .border_color(@ui.rgb8(220, 224, 231))
  .border_radius(6)
  .bg(white())
  .font_size(12)
  .bind_disabled(disabled)
  .bind_style(() => @ui.style().opacity(if disabled() { 0.4 } else { 1.0 }))
}

///|
fn property(label : String, read : () -> String) -> @ui.Element {
  row()
  .children([
    caption(() => label),
    @ui.text(read()).font_size(12).font_weight(500),
  ])
  .justify_content("space-between")
}

///|
fn issue_tracker() -> @ui.Element {
  let issues = load_issues()
  let (query, set_query) = @reactive.create_signal("")
  let (filter, set_filter) = @reactive.create_signal("All issues")
  let (page, set_page) = @reactive.create_signal(0)
  let (selected, set_selected) = @reactive.create_signal(0)
  let current = () => issues[selected()]
  let completed = @reactive.memo(() => {
    issues.filter(item => item.status.get() == "Done").length()
  })
  let matching = @reactive.memo(() => {
    let needle = query().trim().to_owned().to_lower()
    let mode = filter()
    let result : Array[Int] = []
    for i = 0; i < issues.length(); i = i + 1 {
      let item = issues[i]
      let status = item.status.get()
      if (
          mode == "All issues" ||
          (mode == "Open" && status != "Done") ||
          (mode == "Completed" && status == "Done")
        ) &&
        (item.id + " " + item.title + " " + item.project + " " + item.owner)
        .to_lower()
        .contains(needle) {
        result.push(i)
      }
    }
    result
  })
  let pages = () => ((matching.get().length() + 99) / 100).max(1)
  let current_page = () => page().min(pages() - 1)
  let visible = () => {
    let matches = matching.get()
    let start = current_page() * 100
    matches[start:(start + 100).min(matches.length())].to_owned()
  }

  let sidebar = col()
    .children([
      @ui.text("Orbit")
      .font_size(22)
      .font_weight(700)
      .padding_left(12)
      .margin_bottom(6),
      caption(() => "Product workspace").padding_left(12).margin_bottom(24),
    ])
    .width(176)
    .flex_shrink(0)
    .py(24)
    .px(12)
    .gap(8)
    .bg(@ui.rgb8(244, 245, 247))
    .border_right_width(1)
    .border_color(line())
  for name in ["All issues", "Open", "Completed"] {
    ignore(
      sidebar.children([
        @ui.button(name)
        .on_click(() => {
          @reactive.batch(() => {
            set_filter(name)
            set_page(0)
          })
        })
        .flex_row()
        .height(42)
        .padding_left(12)
        .items_center()
        .justify_content("flex-start")
        .border_width(0)
        .border_radius(7)
        .bind_style(() => {
          @ui.style()
          .bg(
            if filter() == name {
              @ui.rgb8(228, 235, 251)
            } else {
              @ui.rgba8(0, 0, 0, 0)
            },
          )
          .text_color(if filter() == name { accent() } else { ink() })
          .font_weight(if filter() == name { 600 } else { 400 })
        }),
      ]),
    )
  }
  ignore(
    sidebar.children([
      @ui.div([]).flex(1),
      caption(() => "September cycle\n4 projects · 5 teammates").padding(12),
    ]),
  )
  let header = row()
    .children([
      col()
      .children([
        @ui.text("Issue inbox").font_size(24).font_weight(700),
        caption(() => {
          "\{issues.length() - completed.get()} open · \{completed.get()} completed"
        }),
      ])
      .gap(6),
      @ui.input()
      .on_input(value => {
        @reactive.batch(() => {
          set_query(value)
          set_page(0)
        })
      })
      .bind_value(() => query())
      .placeholder("Search issues, projects, people")
      .property(@protocol.ACCESSIBILITY_LABEL, @protocol.Text("Search issues"))
      .width(280)
      .height(38)
      .px(12)
      .bg(@ui.rgb8(248, 249, 251))
      .border_width(1)
      .border_color(@ui.rgb8(220, 224, 231))
      .border_radius(7),
    ])
    .height(94)
    .flex_shrink(0)
    .px(24)
    .justify_content("space-between")
    .border_bottom_width(1)
    .border_color(line())
  let inbox = col()
    .children([
      row()
      .children([
        caption(() => "\{matching.get().length()} issues"),
        caption(() => "Updated this week"),
      ])
      .height(48)
      .flex_shrink(0)
      .px(20)
      .justify_content("space-between")
      .bg(@ui.rgb8(250, 251, 252))
      .border_bottom_width(1)
      .border_color(line()),
    ])
    .flex(1)
    // A new result page starts at the top; selection and edits keep its viewport.
    .children_keyed(
      () => ["\{query()}|\{filter()}|\{current_page()}"],
      key => key,
      _ => {
        col()
        .flex(1)
        .overflow_y("scroll")
        .children_keyed(visible, index => index.to_string(), item => {
          let index = item.peek()
          let issue = issues[index]
          @ui.element(@protocol.TAG_BUTTON)
          .children([
            @ui.text(issue.title)
            .font_size(14)
            .font_weight(500)
            .white_space("nowrap")
            .text_overflow("ellipsis")
            .overflow("hidden"),
            @ui.text(
              "\{issue.id}  ·  \{issue.project}  ·  \{issue.status.get()}  ·  \{issue.owner}",
            )
            .font_size(11)
            .text_color(muted())
            .white_space("nowrap"),
          ])
          .flex_col()
          .min_width(0)
          .height(68)
          .flex_shrink(0)
          .justify_center()
          .align_items("stretch")
          .gap(8)
          .px(20)
          .border_width(0)
          .border_bottom_width(1)
          .border_radius(0)
          .border_color(@ui.rgb8(237, 240, 244))
          .bind_style(() => {
            @ui.style().bg(
              if selected() == index {
                @ui.rgb8(237, 243, 255)
              } else {
                white()
              },
            )
          })
          .on_click(() => set_selected(index))
        })
        .children(
          [
            ..if matching.get().is_empty() {
              [caption(() => "No matching issues")]
            } else {
              []
            },
          ],
        )
      },
    )
    .children([
      row()
      .children([
        caption(() => "Page \{current_page() + 1} of \{pages()}"),
        row()
        .children([
          control("Previous", () => set_page(current_page() - 1), () => {
            current_page() == 0
          }),
          control("Next", () => set_page(current_page() + 1), () => {
            current_page() + 1 >= pages()
          }),
        ])
        .gap(8),
      ])
      .height(58)
      .flex_shrink(0)
      .px(20)
      .justify_content("space-between")
      .border_top_width(1)
      .border_color(line()),
    ])
  let details_content = col()
    .children([
      caption(() => "\{current().id} / \{current().project}"),
      @ui.text(current().title)
      .font_size(21)
      .line_height(28)
      .font_weight(700)
      .margin_top(14)
      .margin_bottom(22),
      col()
      .children([
        property("Status", () => current().status.get()),
        property("Assignee", () => current().owner),
        property("Priority", () => current().priority),
      ])
      .gap(12)
      .margin_bottom(24),
      @ui.text(current().description)
      .font_size(13)
      .line_height(20)
      .margin_bottom(22),
      @ui.text("Working notes").font_size(12).font_weight(600).margin_bottom(8),
      @ui.input()
      .on_input(value => current().notes.set(value))
      .multiline(true)
      .bind_value(() => current().notes.get())
      .property(@protocol.ACCESSIBILITY_LABEL, @protocol.Text("Working notes"))
      .height(100)
      .flex_shrink(0)
      .padding(10)
      .font_size(13)
      .line_height(19)
      .border_width(1)
      .border_color(@ui.rgb8(220, 224, 231))
      .border_radius(7),
      @ui.button({
        if current().status.get() == "Done" {
          "Reopen issue"
        } else {
          "Mark complete"
        }
})
      .on_click(() => {
        let issue = current()
        issue.status.set(
          if issue.status.get() == "Done" {
            "Open"
          } else {
            "Done"
          },
        )
      })
      .flex_row()
      .height(36)
      .flex_shrink(0)
      .items_center()
      .justify_center()
      .border_width(0)
      .border_radius(7)
      .margin_top(16)
      .bg(accent())
      .text_color(white())
      .font_weight(500),
      @ui.text("Changes are kept for this session.")
      .font_size(11)
      .text_color(muted())
      .margin_top(10),
    ])
    .flex_shrink(0)
  let details = col()
    .children([details_content])
    .width(350)
    .flex_shrink(0)
    .overflow_y("auto")
    .padding(24)
    .border_left_width(1)
    .border_color(line())
  row()
  .children([
    sidebar,
    col()
    .children([
      header,
      row()
      .children([inbox, details])
      .flex(1)
      .min_height(0)
      .align_items("stretch"),
    ])
    .flex(1),
  ])
  .size_full()
  .align_items("stretch")
  .text_color(ink())
  .font_size(14)
  .bg(white())
}

///|
fn main {
  match
    @native.run(() => {
      ignore(
        @ui.window(issue_tracker, options={
          "title": "Issue tracker — ready",
          "width": 1100,
          "height": 720,
          "background": @ui.rgb8(255, 255, 255),
        }),
      )
    }) {
    Ok(_) => ()
    Err(error) => abort(error)
  }
}
