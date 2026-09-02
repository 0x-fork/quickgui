# Changelog

All notable user-facing changes to QuickGUI are recorded here.

## Unreleased

### Framework

- Added `Accelerator::parse`, an Electron-compatible accelerator grammar that resolves to
  QuickGUI's `Keystroke`, bounded by the new `MAX_ACCELERATOR_BYTES`. `MenuItem::accelerator`,
  `MenuItem::try_accelerator`, and `MenuItem::keystroke` declare an explicit key equivalent that
  outranks keymap derivation, and `MenuItem::hidden` keeps a declared item out of the presented
  menu without changing its action id or collection order.
- Added the `paste-and-match-style`, `delete`, `start-speaking`, `stop-speaking`, `select-next-tab`,
  `select-previous-tab`, `merge-all-windows`, `move-tab-to-new-window`, `toggle-tab-bar`, and
  `toggle-tab-overview` menu roles. Each follows the platform responder chain first and falls back
  to the retained plain-text paste, selection delete, or window-tab command.
- Added `SystemMenuType::RecentDocuments`, an operating-system-populated recent-documents submenu.
- Added `AppRunner::request_quit()` for a preventable quit that runs the before-quit and will-quit
  phases, alongside the existing `AppRunner::exit()` forced shutdown.
- Added `AppRunner` character-palette, tabbing-identifier, tab selection, merge, detach, tab-bar,
  and tab-overview commands plus macOS Find-pasteboard read/write, so externally pumped hosts reach
  the same document-window and search integrations as `EventContext`.

### macOS

- Native menu items now honor an explicit declaration accelerator ahead of both the keymap binding
  and the AppKit standard binding for a role, and hidden items set `NSMenuItem.hidden` and no
  longer claim the Command-W close fallback.
- An "Open Recent" system submenu is built with a `clearRecentDocuments:` "Clear Menu" item so
  `NSDocumentController` populates it.
- Dock menu items now render their declared accelerators.

### JavaScript tooling

- Added `window.onCloseRequested(listener)`, `window.destroy()`, and `window.on("closed" |
  "closeRequested", …)`. While at least one listener is registered the window declares close
  interception to the core, which prevents the native close and emits `closeRequested` instead;
  `window.close()` or `window.destroy()` completes the held request, and withdrawing the last
  listener restores ordinary native closing.
- Added `app.on("beforeQuit" | "willQuit", …)`. A `beforeQuit` listener declares quit interception,
  so the core's `on_before_quit` hook prevents the quit and reports `{ reason }` to JavaScript.
  `app.quit()` now asks for a preventable quit, `app.quit({ force: true })` completes a held one,
  and the new `app.exit(code)` force-quits and reports `code` from `app.run()` and the `quit` event.
  `quitMode` semantics are unchanged.
- Added `accelerator` and `hidden` to `MenuActionItem`/`MenuRoleItem`, the new `MenuRole` names, and
  `{ type: "system-menu", menu: "recent-documents" }`. Invalid or oversized accelerators are
  rejected by the core rather than silently dropped.
- Added `window.setTabbingIdentifier`, `selectNextTab`, `selectPreviousTab`, `selectTab`,
  `mergeAllWindows`, `moveTabToNewWindow`, `toggleTabBar`, `toggleTabOverview`, and
  `showCharacterPalette`.
- Added `Clipboard.availableFormats()`, `Clipboard.has(format)`, `Clipboard.readBuffer(format)`,
  `Clipboard.writeBuffer(format, data)`, `Clipboard.readFindText()`, and
  `Clipboard.writeFindText(text)` over the core's typed clipboard entries.

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
