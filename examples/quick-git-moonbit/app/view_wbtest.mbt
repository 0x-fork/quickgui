///|
fn node_count(node : @native.Node) -> Int {
  let mut count = 1
  for child in node.children() {
    count += node_count(child)
  }
  count
}

///|
fn all_text(node : @native.Node) -> String {
  node.text_content() + node.children().map(all_text).join("")
}

///|
test "application and hidden startup share native window options" {
  let shown = window_options()
  let hidden = window_options(show=false)
  assert_eq(@io.field(shown, "background"), (0).to_json())
  assert_eq(@io.field(hidden, "background"), @io.field(shown, "background"))
  assert_eq(@io.field(shown, "show"), true.to_json())
  assert_eq(@io.field(hidden, "show"), false.to_json())
  assert_eq(@io.field(hidden, "focus"), false.to_json())
  assert_eq(@io.field(shown, "trafficLightX"), (14).to_json())
  assert_eq(@io.field(shown, "trafficLightY"), (19).to_json())
}

///|
test "diff hunk labels react to staging without replacing the table" {
  @native.test_tree(fn(root) raise {
    let s = store(@native.current_window())
    let files = @git.parse_diff(
      "diff --git a/a.txt b/a.txt\n--- a/a.txt\n+++ b/a.txt\n@@ -1 +1 @@\n-before\n+after\n",
    )
    s.diff.set(files)
    s.rows.set(@git.diff_rows(files))
    root.append(diff_table(s).native_node())
    assert_true(all_text(root).contains("Stage hunk"))
    let count = node_count(root)
    ignore(root.take_batch())
    s.staged.set(true)
    assert_true(all_text(root).contains("Unstage hunk"))
    assert_eq(node_count(root), count)
    assert_eq(root.pending_mutations(), 1)
  })
}

///|
test "Quick Git mounts bounded visible rows and retains the history graph" {
  @native.test_tree(fn(root) raise {
    let s = store(@native.current_window())
    s.root.set("/fixture")
    let commits : Array[@git.Commit] = []
    for i = 0; i < 2000; i = i + 1 {
      commits.push({
        sha: "commit-\{i}",
        short: "\{i}",
        parents: if i < 1999 {
          ["commit-\{i+1}"]
        } else {
          []
        },
        author: "Author",
        email: "a@example.invalid",
        time: 0,
        refs: [],
        subject: "Subject \{i}",
        body: "",
      })
    }
    s.commits.set(commits)
    s.graph.set(@git.graph(commits))
    s.view.set("history")
    s.commit_sha.set(commits[0].sha)
    let view = app_view(s)
    root.append(view.native_node())
    let mounted = node_count(root)
    assert_true(mounted < 1000)
    ignore(root.take_batch())
    s.history_width.set(650)
    assert_eq(node_count(root), mounted)
    assert_true(root.pending_mutations() < 5)
  })
}

///|
test "navigation selected styling changes without rebuilding the row" {
  @native.test_tree(fn(root) raise {
    let s = store(@native.current_window())
    let view = navigation(s, "History", "history", () => 0)
    root.append(view.native_node())
    let id = view.native_node().id
    ignore(root.take_batch())
    s.view.set("history")
    assert_eq(view.native_node().id, id)
    assert_true(root.pending_mutations() <= 2)
    let hover = view.native_node().property(@protocol.HOVER_STYLE)
    assert_true(hover is @protocol.Text(_))
  })
}
