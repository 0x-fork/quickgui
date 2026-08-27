# Native document windows

[Documentation index](README.md) · [Windows](windows.md) · [macOS integration](macos.md)

QuickGUI projects editor document state into AppKit without putting native queries in the render
loop. A window can represent a filesystem path, display the unsaved-change indicator, present the
system character palette, and optionally participate in macOS system tabs. The existing tab API is
documented for completeness, but native system-tab acceptance is explicitly deferred beyond 0.1:

```rust
use quickgui::{App, WindowOptions};

App::new(Editor::new("notes.md"))
    .document_path("notes.md")
    .document_edited(false)
    .tabbing_identifier("com.example.editor.workspace")
    .run()?;

let second = cx.open_window(
    Editor::new("draft.md"),
    WindowOptions::new("Draft")
        .document_path("draft.md")
        .tabbing_identifier("com.example.editor.workspace"),
);
```

`represented_file` is the explicit spelling; `document_path` is its GPUI-compatible alias. On
macOS the value is projected as an `NSURL`, so the titlebar receives the native document icon and
path menu. QuickGUI preserves non-UTF-8 Unix paths by constructing the URL from its filesystem
representation rather than converting through a Rust string. A path is non-empty, NUL-free, and
bounded to `MAX_WINDOW_DOCUMENT_PATH_BYTES` before any command is queued.

Runtime changes use the same bounded `EventContext` command queue:

```rust
cx.set_document_path(&new_path)?;
cx.clear_represented_file()?;
cx.set_window_edited(true)?;
cx.show_character_palette()?;

cx.set_tabbing_identifier("com.example.editor.workspace")?;
cx.select_previous_tab()?;
cx.select_next_tab()?;
cx.select_tab(2)?;
cx.merge_all_windows()?;
cx.move_tab_to_new_window()?;
cx.toggle_tab_bar()?;
cx.toggle_tab_overview()?;

// Every operation also has a `_handle` form for `second` or another retained handle.
```

Changing a represented URL or edited state can make AppKit rebuild titlebar controls. QuickGUI
therefore reapplies an explicitly configured `traffic_light_position` immediately after either
mutation. It does not recreate the WGPU surface, renderer, retained view, or native child host.

## Observable tab state

`ViewContext::window_state()` exposes constant-size native document state:

```rust
let state = cx.window_state();
let tabs = state.native_tabs;

assert!(state.represented_file);
assert!(state.native_tabbing);
println!("{} tabs, selected {:?}", tabs.count, tabs.selected_index);
```

`WindowTabState` includes the retained count, selected index, tab-bar visibility, overview
visibility, and a `truncated` flag. QuickGUI inspects at most `MAX_SYSTEM_WINDOW_TABS` (256) native
windows. It never retains a per-frame `Vec<NSWindow>` and never exposes AppKit object ownership to
application code. `TestAppContext::simulate_window_tab_state` injects the same bounded state for
deterministic tests.

The snapshot is refreshed after an explicit tab command and at native focus, move, resize, or
scale-change boundaries. It is not queried during ordinary pointer input, redraw, animation, or
`about_to_wait`. A view is rebuilt only if it called `window_state()` and the effective snapshot
changed.

## AppKit ownership

Every window without a tabbing identifier is explicitly `Disallowed`; opting one window in cannot
silently group utility, dialog, popup, or ordinary application windows. QuickGUI captures
AppKit's process-wide automatic-tabbing policy when the first opted-in window appears, enables it
for the lifetime of opted-in windows, and restores the exact prior value after the last one closes
or the runtime exits.

The configuration remains portable Rust state on other targets. Native document chrome,
character palette, and system-tab actions are macOS capabilities; Windows and Linux projections
belong to their later native-runtime milestones rather than being simulated as custom GPU chrome.

Run the interactive macOS sample with:

```console
cargo run --release --example document_window
```

The sample exercises represented-file changes, edited state, tab grouping/navigation, detaching,
tab-bar and overview commands, and the character palette. Automated tests verify validation,
command retention, deterministic projection, and zero dependency on a native presentation loop.
Native system-tab interaction is optional and deferred from the macOS-first 0.1 release gate; its
real tab-bar acceptance belongs to a later milestone described in [Status and
roadmap](status.md).
