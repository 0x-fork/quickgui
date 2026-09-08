///|
/// Native regression entry used only by tests/smoke. Its repository is created
/// and removed by check.ts, never taken from the user's working checkout.
pub fn smoke() -> Unit {
  @native.require_extension("quick-git-io", "1.0.0")
  let completed = @ref.Ref(false)
  match
    @native.run(
      () => {
        app_owner.val = @reactive.owner()
        @io.start()
        @io.service("environment", {}, result => {
          let path = @io.string(@io.field(result.unwrap(), "open"))
          if !path.contains("quick-git-moonbit-test-") {
            abort("Smoke test requires its disposable repository")
          }
          let window = create_window(path, show=false)
          let s = sessions.get(window.id).unwrap()
          @ui.create_effect(install_menu)
          window.with_owner(() => {
            let steps : Array[() -> Unit] = [
              () => {
                if s.root.peek() == "" || s.status.peek().entries.length() != 2 {
                  abort("Repository status was not loaded")
                }
                println("Smoke: repository opened")
                s.stage(["untracked[1].txt"])
              },
              () => {
                if s.items(true).length() != 1 ||
                  s.items(true)[0].path != "untracked[1].txt" {
                  abort("Literal filename staging failed")
                }
                s.unstage(["untracked[1].txt"])
              },
              () => s.select_change(false, ["one.txt", "untracked[1].txt"]),
              () => {
                if s.diff.peek().length() != 2 {
                  abort("Mixed tracked and untracked selection lost files")
                }
                s.stage(["one.txt"])
              },
              () => {
                if s.items(true).length() != 1 {
                  abort("Stage failed")
                }
                s.unstage(["one.txt"])
              },
              () => {
                if !s.items(true).is_empty() {
                  abort("Unstage failed")
                }
                s.select_change(false, ["one.txt"])
              },
              () => {
                if s.diff.peek().is_empty() || s.diff.peek()[0].hunks.is_empty() {
                  abort("Diff did not load")
                }
                s.apply_hunk(0, 0)
              },
              () => {
                if s.items(true).is_empty() {
                  abort("Hunk staging failed")
                }
                println(
                  "Smoke: staging, literal filenames, multi-file diff and hunk patch passed",
                )
                s.set_view("history")
              },
              () => {
                if s.commits.peek().length() != 2 ||
                  s.graph.peek().length() != 2 {
                  abort("History or graph missing")
                }
                if s.commit_files.peek().length() != 2 ||
                  s.diff.peek().length() != 2 {
                  abort("Commit detail lost files")
                }
                s.commit_path.set("two.txt")
                s.load_diff()
              },
              () => {
                if s.diff.peek().length() != 1 ||
                  s.diff.peek()[0].path != "two.txt" {
                  abort("Commit file selection failed")
                }
                println("Smoke: history and every commit file passed")
                s.set_view("branches")
                s.open_dialog("branch")
              },
              () => {
                s.dialog.set("")
                s.mutate("Create branch", ["branch", "moonbit-smoke"])
              },
              () => {
                if !s.branches.peek().any(b => b.name == "moonbit-smoke") {
                  abort("Branch refresh failed")
                }
                s.mutate("Stash", [
                  "stash", "push", "--include-untracked", "-m", "MoonBit smoke",
                ])
              },
              () => {
                if s.stashes.peek().is_empty() ||
                  !s.status.peek().entries.is_empty() {
                  abort("Stash did not clean tracked and untracked files")
                }
                s.set_view("stashes")
                s.stash.set(s.stashes.peek()[0].name)
                s.load_diff()
              },
              () => {
                if s.diff.peek().is_empty() {
                  abort("Stash diff missing")
                }
                s.mutate("Apply stash", [
                  "stash",
                  "apply",
                  s.stashes.peek()[0].name,
                ])
              },
              () => {
                if s.status.peek().entries.length() != 2 {
                  abort("Stash did not restore all files")
                }
                s.set_view("changes")
                s.stage_all()
              },
              () => {
                s.subject.set("Commit from MoonBit")
                s.body.set("Exercises the asynchronous stdin transport.")
                s.commit()
              },
              () => {
                if !s.status.peek().entries.is_empty() || s.subject.peek() != "" {
                  abort("Commit or draft reset failed")
                }
                s.set_view("history")
              },
              () => {
                if s.commits.peek().length() != 3 ||
                  s.commits.peek()[0].subject != "Commit from MoonBit" {
                  abort("New commit missing from history")
                }
                s.set_view("worktrees")
                s.mutate("Create worktree", [
                  "worktree",
                  "add",
                  "--",
                  path + "-worktree",
                  "moonbit-smoke",
                ])
              },
              () => {
                if s.worktrees.peek().length() != 2 {
                  abort("Worktree refresh failed")
                }
                println("Smoke: branches, stashes, commit and worktrees passed")
              },
            ]
            fn finish() -> Unit {
              let second = create_window("", show=false)
              // Exercise the real welcome-window constructor as well as the
              // repository window; both wait for native readiness before inspection.
              @native.on_event(
                "window-ready-to-show",
                _ => {
                  @native.command(
                    { "method": "get-window-state", "window": second.id },
                    result => {
                      check_smoke_window(result)
                      second.close()
                      completed.val = true
                      (s.watcher)()
                      window.close()
                      @native.quit()
                    },
                  )
                },
                window=second.id,
              )
            }
            fn next(index : Int) -> Unit {
              @io.after_jobs(() => {
                @native.flush()
                @native.command(
                  { "method": "get-window-state", "window": window.id },
                  result => {
                    check_smoke_window(result)
                    if s.notice.peek() != "" &&
                      !s.notice.peek().has_suffix("completed") {
                      abort(s.notice.peek())
                    }
                    if index == steps.length() {
                      finish()
                    } else {
                      steps[index]()
                      next(index + 1)
                    }
                  },
                )
              })
            }
            @native.on_event(
              "window-ready-to-show",
              _ => next(0),
              window=window.id,
            )
          })
        })
      },
      options={
        "name": "Quick Git MoonBit smoke",
        "version": "0.1.0",
        "identifier": "dev.quickgui.quick-git-moonbit-smoke",
        "quitMode": "explicit",
      },
    ) {
    Err(error) => abort(error)
    Ok(_) => ()
  }
  if !completed.val {
    abort("Native smoke did not complete")
  }
  println("Quick Git MoonBit native smoke passed")
}

///|
/// Verify that hidden startup options reached the native window, including
/// geometry, instead of being silently ignored as unknown JSON fields.
fn check_smoke_window(result : Result[Json, String]) -> Unit {
  match result {
    Err(error) => abort("Native window rejected: " + error)
    Ok(state) => {
      if @io.boolean(@io.field(state, "visible")) {
        abort("Smoke window unexpectedly became visible")
      }
      if @io.number(@io.field(state, "minimumWidth")) != 900 ||
        @io.number(@io.field(state, "minimumHeight")) != 560 {
        abort("Quick Git window geometry was not applied")
      }
    }
  }
}
