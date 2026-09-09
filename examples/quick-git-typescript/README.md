# Quick Git in TypeScript

The TypeScript counterpart of the [current Go Quick Git example](../quick-git), built with Bun and Solid 2. The toolbar, composer, lists, dialogs, SVG icons, and progress indicator all use QuickGUI components. Bun runs application code and Git subprocesses in a worker; `bun:ffi` connects to the same in-process Rust core as the other frontends.

From the repository root:

```sh
bun install
bun run build:native # once
bun run --cwd examples/quick-git-typescript dev
```

Set `QUICK_GIT_OPEN=/absolute/path` to choose the initial repository. Otherwise, the last repository opens. Settings use this example's own application identifier, `dev.quickgui.quick-git.typescript`.

The interface follows the current Go example:

- Changes has staged and unstaged tables, per-file counts, partial staging, confirmed discard actions, and an inline commit composer with amend and optional Codex/Claude generation.
- History has paged commits, SVG graph lanes, bounded reference badges, commit metadata, and a virtualized changed-file table with the same 12px status inset. Refresh keeps the selected commit, file, diff, and line selection, including when a new commit moves the selected row.
- Branches, remote branches, tags, worktrees, and stashes use compact rows and native context menus. Worktree removal preserves the branch and lets Git reject a dirty checkout.
- Sidebar, changes, and history widths use native `Splitter` parts. Rust handles pointer capture and resizing; reported sizes update persisted preferences. The titlebar uses the same 52px height and traffic-light placement as Go.
- Each repository window owns an independent Solid root and cancellable store. Reopening a repository focuses its existing window. OS filesystem events coalesce into refreshes; closing or switching repositories cancels obsolete requests.

Git subprocess concurrency is capped at four, requests accept cancellation and deadlines, and output is bounded. The diff cache is limited to 64 entries and 64 MiB of estimated retained data. Only visible table rows are mounted. Long diff lines are truncated for display while patches retain their contents.

`git/` contains the Git runner, parsers, and patch operations; `model/` owns state and persistence; `ui/` contains the current application views; `agent/` implements optional commit-message generation. SVG attribution is in [ICONS-LICENSE](ICONS-LICENSE).

```sh
bun run --cwd examples/quick-git-typescript check
bun run --cwd examples/quick-git-typescript test
bun scripts/check-typescript-examples.ts --quick-git
# Also validate visible native table rows, then close both test windows:
bun scripts/check-typescript-examples.ts --quick-git --visible
bun run --cwd examples/quick-git-typescript build
```

Native checks create a disposable repository, render all five views, stage and unstage a file, preserve history across refresh, and open/close independent windows. They never use the user's repository or saved preferences. Headless tests also cover selection retention, native splitter callbacks, toolbar updates, unusual filenames, and cancellation. Pointer dragging, remote authentication, and installed coding agents require manual testing. Performance benchmarks are separate.
