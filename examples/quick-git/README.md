# Quick Git

A native macOS git client built with QuickGUI and Solid 2. Unlike the other examples, this one is
built as a real product: correct git semantics, bounded resource use, and a polished, keyboard-first
interface, with worktrees and local coding agents as first-class citizens.

```console
bun install
bun run --filter quick-git dev
```

Pass `QUICK_GIT_OPEN=/path/to/repo` to open a repository at launch; otherwise the last one opens.

## What it does

- **Changes**: unstaged and staged lists with per-file line counts, stage or unstage by
  checkbox, double-click, Return, or *Stage all*; discard to the Trash with a native confirmation
  sheet; native context menus.
- **Diffs**: a virtualized unified diff with line numbers, hunk headers, and hover-revealed
  *Stage hunk* / *Unstage hunk* / *Discard hunk*; select lines (Shift-click, ⇧↑/⇧↓) and stage,
  unstage, or discard exactly those lines. Patches are built from the parsed diff and applied with
  `git apply`, so partial staging of new, deleted, and renamed files is handled correctly.
- **Commit**: summary and description fields with a 72-column hint, amend, ⌘⏎ to commit, and
  *Generate with Codex / Claude*, which runs the agent CLI you already have non-interactively
  (`codex exec --sandbox read-only`, `claude -p --tools ""`) over the staged diff and recent
  subjects, with cancellation and a hard timeout.
- **History**: a paged commit table with a lane graph, decorations, and a commit detail pane with
  its files and per-file diff; new branch from a commit, detached checkout, copy SHA.
- **Branches, tags, stashes**: switch, create, delete, stash, apply, pop, drop.
- **Worktrees**: every worktree in the sidebar; switch between them, add one for a new or existing
  branch in a sibling directory, remove it, reveal it, open Terminal there, or launch Codex or
  Claude in it. A branch checked out in another worktree opens that worktree instead of failing.
- Fetch, pull, and push (with automatic upstream), ahead/behind counts, and conflict awareness.

## How it is built

- `git/` is a pure TypeScript git layer: a bounded process runner (concurrency, priority,
  cancellation, deadlines, output caps), parsers for porcelain v2 status, unified diffs,
  `for-each-ref`, `log`, `stash list`, and `worktree list`, plus patch formatting for partial
  staging. It has no UI dependency and is covered by `bun test`, including an integration suite
  that runs real git in a temporary repository.
- `agent/` builds the commit-message prompt, bounds the diff by whole files, runs the CLI, and
  parses the answer; it is tested with a fake process runner.
- `model/` owns application state (Solid signals created outside any component), a recursive
  filesystem watcher that classifies changes and coalesces bursts, and persisted preferences.
- `ui/` renders everything with the core-virtualized `Table` (only visible rows cross the native
  boundary), native menus and alert sheets, and the sidebar vibrancy material.

Everything shown is the core's answer: selection, keyboard navigation, hover, and the new
`selected` state style come from the Rust core, never from a JavaScript round trip.
