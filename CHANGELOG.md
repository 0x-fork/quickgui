# Changelog

All notable user-facing changes to QuickGUI are recorded here.

## Unreleased

### Framework

- Added inherited right-to-left layout with `direction(Direction::Rtl)`, `rtl()`, and `ltr()`.
  In an RTL subtree in-flow positions, physical horizontal insets, and margins mirror inside the
  parent's content box for paint, hit testing, and accessibility together; horizontal scrolling
  starts at the right edge and inverts physical wheel deltas; and shaping uses a forced RTL base
  paragraph direction. Added the logical edge helpers `ps`, `pe`, `ms`, `me`, `border_s`, and
  `border_e`, plus the `TextAlign::Start`/`TextAlign::End` alignments behind `text_start()` and
  `text_end()`. Focus order still follows document order and caret movement stays logical.
- Added CSS-style sticky positioning with `sticky()`, `sticky_top`, `sticky_bottom`, `sticky_left`,
  and `sticky_right`. A sticky element pins inside the nearest scroll container and releases at its
  parent's end, changing painted geometry and hit regions only, and never triggering a relayout.
  Bounded by the new `MAX_STICKY_ELEMENTS_PER_WINDOW`.
- Added scroll snapping with `scroll_snap_x`, `scroll_snap_y`, `snap_align`, and
  `snap_stop_always`. Snapping resolves at a native momentum end phase, at a bounded settle
  deadline for wheels without phases, at scrollbar release, or programmatically, then animates to
  the target on exact deadlines and leaves the window fully settled. Bounded by the new
  `MAX_SCROLL_SNAP_CONTAINERS_PER_WINDOW` and `MAX_SCROLL_SNAP_POINTS_PER_WINDOW`.
- Added extended text styling: `text_shadow` (exact offset and color, bounded blur approximation),
  `letter_spacing`, `word_spacing`, `text_transform` with `uppercase`/`lowercase`/`capitalize` for
  non-editable text, `overline`, `word_break`, `overflow_wrap`, `hyphens`, and `text_direction`.
  Every new property is part of the canonical retained shaping key; case mapping and soft-hyphen
  removal keep selection, copy, and accessibility mapped to the original string, and editable
  inputs are never transformed.
- Added `overflow_x_scroll()` and `overflow_scroll()`.

## 0.1.1 - 2026-08-31

### JavaScript tooling

- Changed Solid window mounting to `new Window({ renderer: createRenderer(() => <App />) })`.
  Applications now import native lifecycle APIs from `@quickgui/native` directly; the Solid
  package no longer re-exports them. The Electron-style singleton `app` exposes `whenReady()`, the
  CLI owns its application loop, and creating a window before readiness now throws explicitly.
  Readiness comes from the new windowless Rust-core `Application`/`AppRunner` lifecycle, while
  `Window.getCurrentWindow()` projects the core-routed render or event window.
- Added `@quickgui/cli` project initialization, target-aware production builds, and a stable native
  development host. On macOS, development runs a signed `.app`, loads project TS/TSX on demand,
  restarts without repackaging on source edits, and keeps the prior app alive when a candidate
  cannot start.
- Production macOS builds now retain the signed `.app`, create a versioned DMG with `create-dmg`,
  and can submit, staple, and validate the DMG with an Apple Notary Keychain profile.
- Published the initial `@quickgui/native`, `@quickgui/solid`, and `@quickgui/cli` packages for
  macOS arm64 and x64.
- Added controlled `Input`/`TextArea` and retained core `Markdown` bindings to
  `@quickgui/native`/`@quickgui/solid`, plus a packaged Solid AI chat example using Vercel AI SDK
  streaming and the DeepSeek provider.
- Added Base-UI-shaped `Root`/`Trigger`/`Popup` parts for Solid `Popover` and `SystemPopover`.
  Triggers now register their retained native anchor internally, while only the system popup subtree
  crosses into its parent-owned native child window renderer.
- Fixed native controlled inputs resetting each keystroke before Solid could commit the queued
  value update.
- Fixed the Solid compiler plugin to preserve and then correctly erase TypeScript syntax after JSX
  lowering, allowing typed `.tsx` applications to build for production.

### Framework

- Added a bounded retained native `Markdown` document with CommonMark tables, task lists, streamed
  tail mending, append-only settled-prefix parsing, and cached `StyledText` flattening.

## 0.1.0 - 2026-08-27

Initial macOS-first framework release.

### Framework

- Damage-driven WGPU rendering with clean-window sleep, bounded renderer caches, batched shapes,
  retained text, images, SVGs, paths, custom shaders, shadows, and declarative motion.
- GPUI-shaped retained views with Flexbox, CSS Grid, container queries, inherited typography,
  Tailwind-style helpers, typed actions, entities, globals, async tasks, and multi-window ownership.
- Uniform and measured variable-height virtualization with native-style captured overlay scrollbars.

### macOS

- Native window roles, hidden-inset title bars, traffic-light positioning, display-aware placement,
  menus, dialogs, clipboard, notifications, document state, drag and drop, and platform services.
- Embedded `NSView` children composed between retained WGPU base and overlay planes without a black
  first frame or cross-display startup movement.
- Overflow-capable nonactivating child panels for popovers, menus, select, autocomplete, combobox,
  and context menus.

### Interaction and components

- Keyboard, mouse, IME, editable and selectable text, forms, accessibility, gestures, native
  cursors, tooltips, typed drag and drop, and deterministic input simulation.
- Unstyled selection controls, tabs, fields, disclosures, dialogs, popover menus, pickers, virtual
  tables, and virtual trees with application-owned presentation.

### Validation and resource ownership

- A 561-test library suite, downstream test-support coverage, all-example and benchmark compilation,
  warning-free Clippy/rustdoc gates, and Rust 1.89 downstream package verification.
- Live 100,000-row scrolling and AppKit composition gates enforce frame-time, CPU, memory, cache,
  lifecycle, resize, focus, native-view, popover, and idle-frame budgets.

### Platform scope

- macOS is the accepted 0.1 platform. Windows and Linux compile through Winit/WGPU but do not yet
  carry equivalent native runtime, visual, accessibility, or resource acceptance evidence.
