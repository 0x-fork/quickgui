# Quick Git in MoonBit

A native Git client built with QuickGUI's MoonBit frontend, using child lists and fluent properties and handlers. The Go example remains in [`../quick-git`](../quick-git). This application uses MoonBit for its components, signals, Git parsing, history graph, partial patches, repository state, and application menus.

## Run

Install Bun, Git, [MoonBit](https://www.moonbitlang.com/download/), a C compiler, and Rust for the example's I/O extension. From the repository root:

```sh
bun install
bun run build:native # once, if the shared core is not already built
bun --cwd examples/quick-git-moonbit dev
```

To use the repository's tested MoonBit toolchain:

```sh
bun scripts/setup-moonbit.ts
export MOON_HOME="$PWD/target/moonbit-toolchain"
export PATH="$MOON_HOME/bin:$PATH"
bun --cwd examples/quick-git-moonbit dev
```

Open a repository with **File → Open Repository** (`Cmd/Ctrl+O`), or set `QUICK_GIT_OPEN=/absolute/path`. Opening another repository creates a window; reopening an existing repository focuses its window. Settings and commit drafts are stored under this app's own data directory.

```sh
bun --cwd examples/quick-git-moonbit build
```

The Bun launcher builds the small I/O extension once, reuses Cargo's cache, and invokes the TypeScript QuickGUI CLI. During development, MoonBit edits rebuild the application and reuse both shared libraries. Restart the command after changing the extension's source. Packaged apps need Git, but do not need Bun, MoonBit, a Go runtime, or Rust installed. Codex CLI or Claude Code is optional for commit-message generation.

## Interface and behavior

- Changes: separate staged and unstaged lists, multiple selection, virtualized multi-file diffs, stage/unstage files, hunks, or selected lines; confirmed discard actions and conflict resolution.
- Commit composer: subject/body, amend, sign-off, persisted drafts, optional Codex/Claude generation with instructions, cancellation, and preservation of edits made while generating.
- History: paged commits, branch lane SVG graph, reference labels, all-branches mode, commit metadata, and every changed file. Sidebar, change-list, and history panels use native splitters.
- Branches and tags: filtering, checkout, create/rename/delete branches, and history for a reference.
- Worktrees and stashes: add/open/remove/prune worktrees; create, inspect, apply, pop, or drop stashes.
- Native application and context menus, keyboard shortcuts, repository picker, clipboard, OS file watching, light/dark appearance, SVG icons, and multiple windows.

All interface elements use QuickGUI components. Lists mount only their reported visible rows. Signals update retained properties; no render loop rebuilds the app. Process concurrency is limited to four, queued work and output are bounded, and repository/window cleanup cancels its jobs. Watchers use OS notifications rather than idle polling. Very long diff lines are truncated for display; patch generation retains their original contents.

## Source layout

- `main/`: application entry.
- `app/`: components, menus, dialogs, repository state and actions.
- `git/`: NUL-safe Git parsers, graph lanes and partial patch generation.
- `io/`: asynchronous process queue, chunk transport, cancellation and settings.
- `native/`: an independent QuickGUI service extension exposing process/file I/O through the public extension ABI. It contains no UI or Git model, and requires no core integration.
- `tests/smoke/`: hidden-window integration entry, run only against a disposable repository created by the test script.

See the [MoonBit SDK guide](../../moonbit/README.md) and [extension contract](../../docs/architecture/extensions.md). SVG icon licenses are in [`app/ICONS-LICENSE`](app/ICONS-LICENSE).

## Checks

```sh
bun --cwd examples/quick-git-moonbit test
# Also exercise the staged shared core and I/O extension in hidden native windows:
bun examples/quick-git-moonbit/check.ts --native
```

Tests cover Git parsing, partial patches, graph lanes, bounded mounted rows, retained selection styling, process byte streams, output limits, cancellation, deadlines, settings and temporary-file cleanup. The native test uses the application's actual window constructor and chrome options for both repository and welcome windows, checking native visibility and minimum dimensions. It stages unusual filenames, previews multiple files, applies a hunk patch, loads commit history, creates a branch, stashes/restores changes, commits, creates a worktree, and opens/closes a second window.

Native execution and packaging are verified on macOS arm64. Linux and Windows runtime behavior, pointer interactions, visual appearance, remote authentication, and installed AI agents still need manual/platform testing. The automated smoke test does not drive or alter a user's repository.
