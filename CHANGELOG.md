# Changelog

All notable user-facing changes to QuickGUI are recorded here.

## Unreleased

### Framework

- Added unstyled segmented `DateField` and `TimeField` editors over plain `CivilDate`/`CivilTime`
  values with no date dependency: proleptic Gregorian validation including leap years, configurable
  year/month/day order, 12- or 24-hour presentation over one retained 24-hour value, typed-digit
  entry that advances as soon as no further digit fits, wrapping arrow steps, Home/End segment
  bounds, Backspace clearing, bounded per-segment placeholders, `min`/`max` validity reported without
  rewriting typed text, caller-owned root and segment parts, `date_field_key_bindings()` and
  `time_field_key_bindings()`, and one native spin button per segment inside a group root.
- Added `CalendarState`, a bounded month grid with roving day focus: at most `MAX_CALENDAR_WEEKS`
  mounted week rows whatever the month, a declared week-start weekday, arrow/Home/End/Page/Shift-Page
  navigation that follows focus across month and year boundaries, selection bounds that refuse the
  selection rather than the movement, caller-owned grid/week/day parts, `calendar_key_bindings()`,
  and exact grid, row, and grid-cell accessibility.
- Added bounded multiple row selection to `TableState`: `TableSelection` retains merged inclusive
  ranges rather than one entry per row, Shift extends from an anchor, the platform modifier toggles
  one row, Space and Command-A work from the keyboard, rows project `selected` state, a multiple
  selection grid projects `multiselectable`, and a real change dispatches the typed
  `TableSelectionChanged` action.
- Added caller-placed column-resize handles, keyboard column reordering, and an inline-edit hook to
  `TableState`. The header renderer receives a behavior-only handle carrying captured pointer drag,
  Left/Right resizing in its own key context, declared minimum widths, and splitter semantics;
  Alt-Left and Alt-Right move the active column through a retained display order that keeps declared
  cell positions stable; `begin_edit` gives one cell its own key context and reports Return and
  Escape as the typed `TableEditEnded` action.
- Added lazy tree children: `TreeNode::pending(true)` mounts exactly one bounded loading placeholder
  row on first expansion, dispatches the typed `TreeLoadChildren` action once, and
  `TreeState::set_children` validates depth, node budget, label bytes, and duplicate IDs before
  splicing atomically, preserving selection, expansion, and the logical scroll anchor or leaving the
  tree untouched.
- Added an unstyled in-window `Menubar` over `PopoverMenu` surfaces: caller-owned root and trigger
  parts, one roving Tab stop over at most `MAX_MENUBAR_MENUS` menus, wrapping Left/Right navigation
  that switches menus rather than closing while one is open, Home/End, Down/Return/Space opening,
  Escape closing without leaving the bar, hover switching only while the bar is open, and exact
  menubar/menu-item accessibility. Native `Menu` remains the platform application menu.
- Added `Element::accessibility_multiselectable`, projected as the native multiselectable state, so
  a grid can announce that it accepts more than one selected descendant.

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
