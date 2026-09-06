# Quick Git

A native macOS git client built with QuickGUI and QuickGUI UI. Unlike the other examples, this one is
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
- **Windows**: every repository gets its own window with its own state. The sidebar header drops
  down a switcher listing recent repositories; choosing one focuses its window or opens a new one,
  and *Open Repository…* fills an empty window or opens another.

## How it is built

- `git/` is a TypeScript git layer: a bounded process runner (concurrency, priority,
  cancellation, deadlines, output caps), parsers for porcelain v2 status, `for-each-ref`, `log`,
  `stash list`, and `worktree list`, plus patch formatting for partial staging. It has no UI
  dependency and is covered by `bun run test`, including an integration suite that runs real git in a
  temporary repository.
- `git/diff.ts` is the diff engine, TypeScript compiled to native code like the rest of the app.
  `Diff.open` reads git's raw bytes in one scan that keeps the lines and records where each file
  and hunk begins; lines become objects only where they are needed, so the table asks for the rows
  it is about to paint, a selection is resolved to the changed lines inside it, and a whole file is
  materialized only when a patch is formatted. The scan runs a megabyte at a time and yields to the
  event loop between chunks, so the window keeps painting while a big diff is read and a diff whose
  selection moved on is abandoned mid-scan. A 38 MB diff of 810,000 lines opens in 845 ms with no
  stall longer than 250 ms (an object per line was 3.1 s of frozen window), and a 60-row window
  costs 0.3 ms. Parsed diffs are cached by target, bounded by count and by the bytes they came
  from, and closed on eviction; closing a handle drops the whole scan at once.
- `agent/` builds the commit-message prompt, bounds the diff by whole files, runs the CLI, and
  parses the answer; it is tested with a fake process runner.
- `model/` owns application state (QuickGUI UI signals created outside any component), a recursive
  filesystem watcher that classifies changes and coalesces bursts, and persisted preferences.
- `ui/` renders everything with the core-virtualized `Table` (only visible rows cross the native
  boundary), native menus and alert sheets, and the sidebar vibrancy material.

Everything shown is the core's answer: selection, keyboard navigation, hover, and the new
`selected` state style come from the Rust core, never from a JavaScript round trip.

The tests load the real Zig static library through a test-only C adapter. Async results are copied
from Zig background threads and drained on the test thread. Production applications bind the same
C functions directly through scriptc; Bun is only the test runner.
