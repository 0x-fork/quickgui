///|
test "porcelain v2 preserves rename records, spaces, conflicts and unborn HEAD" {
  let status = @git.parse_status(
    "# branch.oid (initial)\u0000# branch.head main\u0000# branch.upstream origin/main\u0000# branch.ab +3 -2\u0000" +
    "1 .M N... 100644 100644 100644 abc def file with spaces.go\u0000" +
    "2 R. N... 100644 100644 100644 abc def R100 new name.go\u0000old name.go\u0000" +
    "u UU N... 100644 100644 100644 100644 a b c conflict.go\u0000? tabs\tand\nlines.go\u0000",
  )
  assert_eq(status.head, "")
  assert_eq(status.branch, "main")
  assert_eq(status.ahead, 3)
  assert_eq(status.behind, 2)
  assert_eq(status.entries.length(), 4)
  assert_eq(status.entries[1].original, "old name.go")
  assert_eq(status.entries[2].conflict, true)
  assert_eq(@git.changes(status, true).length(), 1)
  assert_eq(@git.changes(status, false).length(), 3)
}

///|
test "commit files include every path and rename destination" {
  let files = @git.parse_commit_files(
    "M\u0000first.go\u0000A\u0000second.go\u0000R100\u0000old.go\u0000new.go\u0000",
  )
  assert_eq(files.length(), 3)
  assert_eq(files[1].path, "second.go")
  assert_eq(files[2].path, "new.go")
  assert_eq(files[2].original, "old.go")
}

///|
test "multi-file patches parse bare hunk headers and no-newline markers" {
  let files = @git.parse_diff(
    "diff --git a/one b/one\n--- a/one\n+++ b/one\n@@ -1 +1 @@\n-old\n+new\n\\ No newline at end of file\n" +
    "diff --git a/two b/two\nnew file mode 100644\n--- /dev/null\n+++ b/two\n@@ -0,0 +1,2 @@\n+a\n+b\n",
  )
  assert_eq(files.length(), 2)
  assert_eq(files[0].hunks.length(), 1)
  assert_eq(files[0].hunks[0].lines[1].no_newline, true)
  assert_eq(files[1].kind, "A")
  assert_eq(@git.diff_rows(files).filter(row => row.kind == "file").length(), 2)
  let partial = @git.patch(files[1], Map([(0, Some([0]))]), false)
  assert_true(partial.contains("@@ -0,0 +1,1 @@"))
  assert_true(partial.contains("+a\n"))
  assert_false(partial.contains("+b\n"))
}

///|
test "quoted paths preserve octal UTF-8 and tabs" {
  assert_eq(@git.unquote("\"a/\\303\\251\\tfile\""), "a/é\tfile")
  assert_eq(
    @git.unquote(@git.quote("a/with\nline\tand\\slash")),
    "a/with\nline\tand\\slash",
  )
}

///|
test "reverse partial patches keep unselected additions as context" {
  let file = @git.parse_diff(
      "diff --git a/a b/a\n--- a/a\n+++ b/a\n@@ -1,2 +1,3 @@\n first\n-old\n+new\n+other\n",
    )[0]
  let patch = @git.patch(file, Map([(0, Some([1, 2]))]), true)
  assert_true(patch.contains("-old\n+new\n other\n"))
  assert_true(patch.contains("@@ -1,3 +1,3 @@"))
}

///|
test "merge graph joins lanes without replacing history rows" {
  fn commit(sha : String, parents : Array[String]) -> @git.Commit {
    {
      sha,
      short: sha,
      parents,
      author: "Test",
      email: "test@example.invalid",
      time: 0,
      refs: [],
      subject: sha,
      body: "",
    }
  }
  let graph = @git.graph([
    commit("merge", ["left", "right"]),
    commit("left", ["root"]),
    commit("right", ["root"]),
    commit("root", []),
  ])
  assert_eq(graph[0].edges.length(), 2)
  assert_eq(graph[2].incoming, true)
  assert_eq(graph[3].edges.length(), 0)
  assert_true(graph[0].width >= 2)
  let layers = @git.graph_layers(graph[0], 40)
  assert_eq(layers.length(), 2)
  assert_true(layers[0].color != layers[1].color)
  assert_true(layers[0].source.contains("<path"))
  assert_true(layers[0].source.contains("<circle"))
  assert_false(layers[1].source.contains("<circle"))
  assert_true(layers[1].source.contains("M13 13C13 26 27 13 27 26"))
}

///|
test "worktrees and stashes have stable full identifiers" {
  let trees = @git.parse_worktrees(
    "worktree /repo\u0000HEAD abc\u0000branch refs/heads/main\u0000\u0000worktree /repo-topic\u0000HEAD def\u0000detached\u0000locked reason\u0000\u0000",
  )
  assert_eq(trees.length(), 2)
  assert_eq(trees[0].branch, "main")
  assert_eq(trees[1].locked, true)
  let stashes = @git.parse_stashes(
    "stash@{0}\u001fabc\u001f123\u001fOn main: work\u0000\n",
  )
  assert_eq(stashes.length(), 1)
  assert_eq(stashes[0].name, "stash@{0}")
  assert_true(@git.branch_problem("topic/new-feature").is_empty())
  assert_false(@git.branch_problem("--force").is_empty())
}
