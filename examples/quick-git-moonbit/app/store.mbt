///|
priv struct Store {
  window : @native.Window
  root : @reactive.Signal[String]
  opening : @reactive.Signal[Bool]
  status : @reactive.Signal[@git.Status]
  view : @reactive.Signal[String]
  selected : @reactive.Signal[Array[String]]
  staged : @reactive.Signal[Bool]
  commits : @reactive.Signal[Array[@git.Commit]]
  graph : @reactive.Signal[Array[@git.GraphRow]]
  history_busy : @reactive.Signal[Bool]
  history_more : @reactive.Signal[Bool]
  all_branches : @reactive.Signal[Bool]
  history_ref : @reactive.Signal[String]
  commit_sha : @reactive.Signal[String]
  commit_files : @reactive.Signal[Array[@git.CommitFile]]
  commit_path : @reactive.Signal[String]
  branches : @reactive.Signal[Array[@git.Branch]]
  worktrees : @reactive.Signal[Array[@git.Worktree]]
  stashes : @reactive.Signal[Array[@git.Stash]]
  stash : @reactive.Signal[String]
  diff : @reactive.Signal[Array[@git.DiffFile]]
  rows : @reactive.Signal[Array[@git.DiffRow]]
  diff_selection : @reactive.Signal[Array[Array[Int]]]
  diff_busy : @reactive.Signal[Bool]
  subject : @reactive.Signal[String]
  body : @reactive.Signal[String]
  amend : @reactive.Signal[Bool]
  signoff : @reactive.Signal[Bool]
  busy : @reactive.Signal[String]
  notice : @reactive.Signal[String]
  dialog : @reactive.Signal[String]
  dialog_from : @reactive.Signal[String]
  sidebar_width : @reactive.Signal[Double]
  changes_width : @reactive.Signal[Double]
  history_width : @reactive.Signal[Double]
  appearance : @reactive.Signal[String]
  mut epoch : Int
  mut diff_epoch : Int
  mut history_epoch : Int
  mut history_dirty : Bool
  mut busy_epoch : Int
  mut refresh_busy : Bool
  mut refresh_again : Bool
  mut watcher : () -> Unit
  mut cancel_busy : () -> Unit
  mut cancel_diff : () -> Unit
}

///|
fn store(window : @native.Window) -> Store {
  {
    window,
    root: @reactive.signal(""),
    opening: @reactive.signal(false),
    status: @reactive.signal(@git.empty_status()),
    view: @reactive.signal("changes"),
    selected: @reactive.signal([]),
    staged: @reactive.signal(false),
    commits: @reactive.signal([]),
    graph: @reactive.signal([]),
    history_busy: @reactive.signal(false),
    history_more: @reactive.signal(false),
    all_branches: @reactive.signal(false),
    history_ref: @reactive.signal(""),
    commit_sha: @reactive.signal(""),
    commit_files: @reactive.signal([]),
    commit_path: @reactive.signal(""),
    branches: @reactive.signal([]),
    worktrees: @reactive.signal([]),
    stashes: @reactive.signal([]),
    stash: @reactive.signal(""),
    diff: @reactive.signal([]),
    rows: @reactive.signal([]),
    diff_selection: @reactive.signal([]),
    diff_busy: @reactive.signal(false),
    subject: @reactive.signal(""),
    body: @reactive.signal(""),
    amend: @reactive.signal(false),
    signoff: @reactive.signal(false),
    busy: @reactive.signal(""),
    notice: @reactive.signal(""),
    dialog: @reactive.signal(""),
    dialog_from: @reactive.signal(""),
    sidebar_width: @reactive.signal(230),
    changes_width: @reactive.signal(300),
    history_width: @reactive.signal(540),
    appearance: @reactive.signal("light"),
    epoch: 0,
    diff_epoch: 0,
    history_epoch: 0,
    history_dirty: true,
    busy_epoch: 0,
    refresh_busy: false,
    refresh_again: false,
    watcher: () => (),
    cancel_busy: () => (),
    cancel_diff: () => (),
  }
}

///|
fn Store::live(self : Store, epoch : Int) -> Bool {
  self.window.is_open() && epoch == self.epoch
}

///|
fn Store::error(self : Store, error : String) -> Unit {
  self.notice.set(
    if error.length() > 16000 {
      error.view(end_offset=16000).to_owned() + "…"
    } else {
      error
    },
  )
}

///|
fn Store::items(self : Store, staged : Bool) -> Array[@git.Change] {
  @git.changes(self.status.get(), staged)
}

///|
fn Store::open_dialog(self : Store, kind : String, from? : String = "") -> Unit {
  self.dialog_from.set(from)
  self.dialog.set(kind)
}

///|
fn Store::open(self : Store, path : String, activate? : Bool = true) -> Unit {
  self.window.with_owner(() => {
    self.close()
    self.opening.set(true)
    let epoch = self.epoch
    ignore(
      @io.git(
        path,
        [
          "rev-parse", "--path-format=absolute", "--show-toplevel", "--absolute-git-dir",
          "--git-common-dir",
        ],
        result => {
          if !self.live(epoch) {
            return
          }
          self.opening.set(false)
          match result {
            Err(error) => self.error(error)
            Ok(output) => {
              let paths = @git.split(
                output.trim_end(chars="\n").to_owned(),
                "\n",
              )
              if paths.length() < 2 {
                self.error("Not a working Git repository")
                return
              }
              self.root.set(paths[0])
              remember_repository(paths[0])
              self.restore()
              self.watcher = @native.watch_files(
                paths,
                event => {
                  let paths = @io.strings(@io.field(event, "paths"))
                  if paths.is_empty() ||
                    paths.any(p => {
                      !p.contains("/.quickgui/") &&
                      !p.contains("/.git/objects/") &&
                      !p.has_suffix(".lock")
                    }) {
                    self.refresh()
                  }
                },
                result => {
                  if result is Err(error) {
                    self.error("Repository watcher: " + error)
                  }
                },
              )
              self.refresh()
              if activate {
                self.window.focus()
              }
            }
          }
        },
      ),
    )
  })
}

///|
fn Store::close(self : Store) -> Unit {
  self.persist()
  self.window.with_owner(@io.cancel_pending)
  self.epoch += 1
  self.diff_epoch += 1
  self.history_epoch += 1
  self.history_dirty = true
  self.busy_epoch += 1
  (self.watcher)()
  self.watcher = () => ()
  (self.cancel_busy)()
  (self.cancel_diff)()
  self.cancel_busy = () => ()
  self.cancel_diff = () => ()
  self.refresh_busy = false
  self.refresh_again = false
  @reactive.batch(() => {
    self.root.set("")
    self.opening.set(false)
    self.status.set(@git.empty_status())
    self.selected.set([])
    self.commits.set([])
    self.graph.set([])
    self.commit_sha.set("")
    self.commit_path.set("")
    self.commit_files.set([])
    self.branches.set([])
    self.worktrees.set([])
    self.stashes.set([])
    self.diff.set([])
    self.rows.set([])
    self.busy.set("")
    self.diff_busy.set(false)
    self.history_busy.set(false)
    self.subject.set("")
    self.body.set("")
    self.dialog.set("")
    self.notice.set("")
    self.view.set("changes")
    self.amend.set(false)
  })
}

///|
fn Store::refresh(self : Store) -> Unit {
  if self.root.peek() == "" {
    return
  }
  if self.refresh_busy {
    self.refresh_again = true
    return
  }
  self.refresh_busy = true
  self.window.with_owner(() => {
    let epoch = self.epoch
    let root = self.root.peek()
    let remaining = @ref.Ref(4)
    fn completed() -> Unit {
      if !self.live(epoch) {
        return
      }
      remaining.val -= 1
      if remaining.val == 0 {
        self.refresh_busy = false
        if self.history_dirty && self.view.peek() == "history" {
          self.load_history(true)
        }
        if self.refresh_again {
          self.refresh_again = false
          self.refresh()
        }
      }
    }
    ignore(
      @io.git(
        root,
        [
          "--no-optional-locks", "status", "--porcelain=v2", "-z", "--branch", "--show-stash",
          "--untracked-files=all", "--renames",
        ],
        result => {
          if !self.live(epoch) {
            return
          }
          match result {
            Err(error) => self.error(error)
            Ok(output) => {
              let status = @git.parse_status(output)
              if self.status.peek().head != status.head ||
                self.status.peek().branch != status.branch {
                self.history_dirty = true
              }
              self.status.set(status)
              let items = self.items(self.staged.peek())
              let selected = self.selected
                .peek()
                .filter(path => items.any(item => item.path == path))
              self.selected.set(
                if selected.is_empty() && !items.is_empty() {
                  [items[0].path]
                } else {
                  selected
                },
              )
              self.window.action(
                "set-title",
                value=@git.base(root) + " — " + status.branch,
              )
              self.window.action("set-represented-file", value=root)
              if self.view.peek() == "changes" {
                self.load_diff()
              }
            }
          }
          completed()
        },
      ),
    )
    ignore(
      @io.git(
        root,
        [
          "for-each-ref",
          "--sort=-committerdate",
          "--format=" + @git.ref_format,
          "refs/heads",
          "refs/remotes",
          "refs/tags",
        ],
        result => {
          if self.live(epoch) {
            match result {
              Ok(text) => {
                let refs = @git.parse_refs(text)
                if refs != self.branches.peek() {
                  self.history_dirty = true
                }
                self.branches.set(refs)
              }
              Err(error) => self.error(error)
            }
          }
          completed()
        },
      ),
    )
    ignore(
      @io.git(root, ["worktree", "list", "--porcelain", "-z"], result => {
        if self.live(epoch) {
          match result {
            Ok(text) => self.worktrees.set(@git.parse_worktrees(text))
            Err(error) => self.error(error)
          }
        }
        completed()
      }),
    )
    ignore(
      @io.git(root, ["stash", "list", "--format=%gd%x1f%H%x1f%at%x1f%gs%x00"], result => {
        if self.live(epoch) {
          match result {
            Ok(text) => self.stashes.set(@git.parse_stashes(text))
            Err(error) => self.error(error)
          }
        }
        completed()
      }),
    )
  })
}

///|
fn Store::set_view(self : Store, view : String) -> Unit {
  self.view.set(view)
  self.diff_selection.set([])
  if view == "history" {
    if self.commits.peek().is_empty() || self.history_dirty {
      self.load_history(true)
    } else {
      self.load_diff()
    }
  } else if view == "changes" {
    self.load_diff()
  }
}

///|
fn Store::select_change(
  self : Store,
  staged : Bool,
  paths : Array[String],
) -> Unit {
  self.staged.set(staged)
  self.selected.set(paths)
  self.diff_selection.set([])
  self.load_diff()
}

///|
fn Store::load_history(self : Store, reset : Bool) -> Unit {
  if self.root.peek() == "" || (!reset && self.history_busy.peek()) {
    return
  }
  self.history_epoch += 1
  let generation = self.history_epoch
  let epoch = self.epoch
  let skip = if reset { 0 } else { self.commits.peek().length() }
  if skip >= 10000 {
    self.history_more.set(false)
    return
  }
  self.history_busy.set(true)
  self.history_dirty = false
  let args = [
    "log",
    "--date-order",
    "--max-count=200",
    "--skip=\{skip}",
    "--format=" + @git.log_format,
  ]
  if self.all_branches.peek() {
    args.push("--all")
  }
  if self.history_ref.peek() != "" {
    args.push(self.history_ref.peek())
  }
  args.push("--")
  self.window.with_owner(() => {
    ignore(
      @io.git(self.root.peek(), args, result => {
        if !self.live(epoch) || generation != self.history_epoch {
          return
        }
        self.history_busy.set(false)
        match result {
          Err(error) =>
            if self.status.peek().head == "" {
              self.commits.set([])
              self.graph.set([])
            } else {
              self.error(error)
            }
          Ok(text) => {
            let added = @git.parse_log(text)
            let commits = if reset {
              added
            } else {
              [..self.commits.peek(), ..added]
            }
            self.commits.set(commits)
            self.graph.set(@git.graph(commits))
            self.history_more.set(added.length() == 200)
            if !commits.is_empty() &&
              !commits.any(c => c.sha == self.commit_sha.peek()) {
              self.select_commit(commits[0].sha)
            }
          }
        }
      }),
    )
  })
}

///|
fn Store::select_commit(self : Store, sha : String) -> Unit {
  self.commit_sha.set(sha)
  self.commit_path.set("")
  self.diff_selection.set([])
  self.commit_files.set([])
  let epoch = self.epoch
  self.window.with_owner(() => {
    ignore(
      @io.git(
        self.root.peek(),
        [
          "show", "--format=", "--first-parent", "-M", "-z", "--name-status", sha,
          "--",
        ],
        result => {
          if !self.live(epoch) || self.commit_sha.peek() != sha {
            return
          }
          match result {
            Ok(text) => self.commit_files.set(@git.parse_commit_files(text))
            Err(error) => self.error(error)
          }
        },
      ),
    )
  })
  self.load_diff()
}

///|
fn Store::load_diff(self : Store) -> Unit {
  if self.root.peek() == "" {
    return
  }
  self.diff_epoch += 1
  let generation = self.diff_epoch
  let epoch = self.epoch
  (self.cancel_diff)()
  self.diff_busy.set(true)
  self.cancel_diff = () => ()
  let common = [
    "--no-ext-diff", "--no-textconv", "--unified=3", "--find-renames", "--",
  ]
  let requests : Array[(Array[String], Bool)] = []
  if self.view.peek() == "history" {
    if self.commit_sha.peek() != "" {
      let paths = if self.commit_path.peek() == "" {
        []
      } else {
        [@git.literal(self.commit_path.peek())]
      }
      requests.push(
        (
          [
            "show",
            "--format=",
            "--first-parent",
            self.commit_sha.peek(),
            ..common,
            ..paths,
          ],
          false,
        ),
      )
    }
  } else if self.view.peek() == "stashes" {
    if self.stash.peek() != "" {
      requests.push(
        (
          [
            "stash",
            "show",
            "-p",
            "--include-untracked",
            self.stash.peek(),
            ..common,
          ],
          false,
        ),
      )
    }
  } else {
    let selected = self.selected.peek()
    let untracked = self.status.peek().entries
      .filter(e => {
        e.untracked && selected.contains(e.path) && !self.staged.peek()
      })
      .map(e => e.path)
    let tracked = selected.filter(path => !untracked.contains(path))
    if !tracked.is_empty() {
      let args = ["diff"]
      if self.staged.peek() {
        args.push("--cached")
      }
      let paths = tracked.copy()
      for entry in self.status.peek().entries {
        if tracked.contains(entry.path) && entry.original != "" {
          paths.push(entry.original)
        }
      }
      requests.push(([..args, ..common, ..paths.map(@git.literal)], false))
    }
    for path in untracked {
      requests.push((["diff", "--no-index", ..common, "/dev/null", path], true))
    }
  }
  let files : Array[@git.DiffFile] = []
  let size = @ref.Ref(0)
  let stopped = @ref.Ref(false)
  let cancel : @ref.Ref[() -> Unit] = @ref.Ref(() => ())
  self.cancel_diff = () => {
    stopped.val = true
    (cancel.val)()
  }
  // Process selected untracked files one at a time so a large selection cannot
  // flood the process queue or multiply the preview's memory limit.
  fn next(index : Int) -> Unit {
    if stopped.val || !self.live(epoch) || generation != self.diff_epoch {
      return
    }
    if index == requests.length() {
      self.cancel_diff = () => ()
      self.diff_busy.set(false)
      self.diff.set(files)
      self.rows.set(@git.diff_rows(files))
      return
    }
    let request = requests[index]
    cancel.val = @io.git(
      self.root.peek(),
      request.0,
      result => {
        if stopped.val || !self.live(epoch) || generation != self.diff_epoch {
          return
        }
        match result {
          Err(error) => {
            self.cancel_diff = () => ()
            self.diff_busy.set(false)
            self.diff.set([])
            self.rows.set([])
            self.error(error)
          }
          Ok(text) => {
            size.val += text.length()
            if size.val > 16 * 1024 * 1024 {
              self.cancel_diff = () => ()
              self.diff_busy.set(false)
              self.error(
                "Diff selection exceeds the preview limit; select fewer files",
              )
              return
            }
            files.append(@git.parse_diff(text))
            next(index + 1)
          }
        }
      },
      allow_one=request.1,
    )
  }
  self.window.with_owner(() => next(0))
}

///|
fn Store::mutate(
  self : Store,
  label : String,
  args : Array[String],
  input? : String = "",
  done? : () -> Unit = () => (),
) -> Unit {
  if self.busy.peek() != "" || self.root.peek() == "" {
    return
  }
  self.busy.set(label)
  self.busy_epoch += 1
  let generation = self.busy_epoch
  let epoch = self.epoch
  self.cancel_busy = self.window.with_owner(() => {
    @io.git(
      self.root.peek(),
      args,
      result => {
        if !self.live(epoch) || generation != self.busy_epoch {
          return
        }
        self.cancel_busy = () => ()
        self.busy.set("")
        match result {
          Err(error) => self.error(error)
          Ok(_) => {
            self.history_dirty = true
            done()
            self.notice.set(label + " completed")
            self.refresh()
            if self.view.peek() == "history" {
              self.load_history(true)
            }
          }
        }
      },
      input~,
      timeout=300000,
    )
  })
}

///|
fn Store::cancel(self : Store) -> Unit {
  self.busy_epoch += 1
  (self.cancel_busy)()
  self.cancel_busy = () => ()
  self.busy.set("")
  self.notice.set("Operation cancelled")
  self.refresh()
}

///|
fn Store::stage(self : Store, paths : Array[String]) -> Unit {
  self.mutate("Stage", ["add", "--", ..paths.map(@git.literal)])
}

///|
fn Store::unstage(self : Store, paths : Array[String]) -> Unit {
  self.mutate(
    "Unstage",
    if self.status.peek().head == "" {
      ["rm", "--cached", "-r", "--", ..paths.map(@git.literal)]
    } else {
      ["reset", "-q", "HEAD", "--", ..paths.map(@git.literal)]
    },
  )
}

///|
fn Store::stage_all(self : Store) -> Unit {
  self.mutate("Stage all", ["add", "-A"])
}

///|
fn Store::unstage_all(self : Store) -> Unit {
  self.mutate(
    "Unstage all",
    if self.status.peek().head == "" {
      ["rm", "--cached", "-r", "--", "."]
    } else {
      ["reset", "-q", "HEAD", "--", "."]
    },
  )
}

///|
fn Store::commit(self : Store) -> Unit {
  if self.subject.peek().trim().is_empty() {
    self.error("Enter a commit subject")
    return
  }
  let args = ["commit", "--file=-", "--cleanup=strip"]
  if self.amend.peek() {
    args.push("--amend")
  }
  if self.signoff.peek() {
    args.push("--signoff")
  }
  self.mutate(
    "Commit",
    args,
    input=self.subject.peek().trim().to_owned() + "\n\n" + self.body.peek(),
    done=() => {
      self.subject.set("")
      self.body.set("")
      self.amend.set(false)
      self.persist()
    },
  )
}

///|
fn Store::set_amend(self : Store, value : Bool) -> Unit {
  self.amend.set(value)
  if value && self.subject.peek() == "" {
    self.window.with_owner(() => {
      ignore(
        @io.git(self.root.peek(), ["log", "-1", "--format=%s%x00%b"], result => {
          if self.amend.peek() &&
            self.subject.peek() == "" &&
            result is Ok(text) {
            let parts = @git.split(text, "\u0000")
            self.subject.set(parts[0])
            if parts.length() > 1 {
              self.body.set(parts[1].trim().to_owned())
            }
          }
        }),
      )
    })
  }
}
