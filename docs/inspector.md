# Retained-tree inspector

[Documentation index](README.md)

The inspector is a read-only developer overlay built from the same retained `UiTree`, layout
bounds, hit stack, accessibility declarations, scene ordering, and frame metrics used by the
application. It is an explicit Cargo feature:

```toml
[dependencies]
quickgui = { version = "...", features = ["inspector"] }
```

Without that feature, the module, window field, commands, snapshot traversal, input branches, and
overlay renderer are all absent from the build. Enabling the feature does not open an inspector by
itself:

```rust
Application::new().run(|cx| {
    cx.open_window(
        WindowOptions::default().inspector(true), // optional initial state
        Editor::new(),
    );
})?;
```

An event or action callback can control the current or another window:

```rust
cx.toggle_inspector()?;
cx.set_inspector(true)?;
cx.set_inspector_handle(other_window, false)?;
```

`ViewContext::window_state().inspector_active` exposes the declarative state when a view needs to
update a checked menu item or label. A view that does not observe `WindowState` is not rebuilt when
the overlay opens or closes.

Run the complete example with:

```console
cargo run --features inspector --example inspector
```

## Interaction

An opened inspector starts in picking mode. Moving the pointer highlights the topmost painted
application element. Click to freeze that selection; ordinary application input then works outside
the inspector panel. Click **Pick** to resume picking and **Close** or Escape to close the overlay.
While picking, wheel movement cycles through elements occluded at the same pointer coordinate.
Ordering uses the real scene plane, accumulated z-index, and retained source order rather than a
second approximate hit-test hierarchy.

The right-side panel reports:

- retained and parent IDs, element kind, hierarchy depth, and whether the ID was explicit;
- layout bounds, effective clip, portal state, scene plane, z-index, and source order;
- exact hit-region flags, cursor, app region, and focus/focus-path state;
- accessibility role, disabled/selected/checked-mixed/expanded/invalid state, popover kind and
  controlled target, autocomplete mode, active descendant, table row/column counts and indices,
  tree level/set position, sort direction, and bounded label/value/description/validation text;
- previous completed-frame CPU/wall metrics, draw counts, and bounded cache sizes;
- the current frame's view, scroll, measurement, animation, and transition damage causes.

The highlight and panel use an independent overlay `UiTree` painted after application content.
They cannot collide with application element IDs, listeners, focus, retained scrolling, or view
state. With a macOS `NSView` child, the existing transparent overlay surface places the inspector
above native content through the same three-plane composition path.

## Resource contract

One open inspector retains at most `MAX_INSPECTOR_NODES` (8,192) painted application nodes. Each
captured accessibility string is capped at `MAX_INSPECTOR_TEXT_BYTES` (1,024 UTF-8 bytes), with
truncation reported in the snapshot. Candidate and hit lookup storage is reused and bounded by the
same node ceiling.

The inspector creates no timer, animation loop, polling task, native monitor, or background thread.
It redraws only for normal application damage, a pointer selection change, wheel depth change, or
an explicit open/close/pick command. Closing it drops the snapshot, scratch storage, and overlay
tree for that window.

## Deterministic inspection

Downstream tests can combine `inspector` and `test-support`:

```toml
[dev-dependencies]
quickgui = { version = "...", features = ["inspector", "test-support"] }
```

`TestAppContext::inspector_snapshot(window)` materializes production layout on demand and returns
the same bounded `InspectorSnapshot` without creating a native window or WGPU adapter. This is
appropriate for hierarchy, geometry, focus, hit-region, accessibility, and truncation assertions.
Pixel composition and interaction over a real AppKit child remain part of live macOS acceptance.

The current framework inspector intentionally observes rather than mutates application styles.
Live style editing and source-location metadata can be layered on later without making debug state
part of every release element or the normal paint hot path.
