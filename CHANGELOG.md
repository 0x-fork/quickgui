# Changelog

All notable user-facing changes to QuickGUI are recorded here.

## Unreleased

### Framework

- Added display rotation, built-in-panel, and color-depth metadata to `Display`, a deterministic
  bounded `Displays::diff`, the granular `DisplayEvent::{Added, Removed, MetricsChanged}` value, and
  `Application::on_display_event`. The coarse displays-changed observation is unchanged and no timer
  or polling source was added.
- Added `EventContext::message_box` with `MessageBoxOptions`: severity, separate `message`/`detail`
  text, explicit `default_button`/`cancel_button` indices, a suppression `checkbox`, and a custom
  `icon`. The bounded `MessageBoxResponse` carries the chosen button index and the checkbox state.
- Added open-panel `can_create_directories`, `resolves_aliases`,
  `treats_file_packages_as_directories`, and `message` options, and save-panel `name_field_label`
  and `shows_tag_field` options, each validated before the request is retained.
- Added `EventContext::preview_file`/`close_file_preview`, `show_color_panel`/`close_color_panel`,
  `show_font_panel`, `share_items`, and `authenticate_with_biometrics`, plus
  `Application::on_color_panel_change` and `on_font_panel_change`. Non-macOS targets return
  `PlatformError::Unsupported` and the new `DesktopIntegrationSupport` flags report it.
- Added `Image::from_data_url`, `Image::named_system`, `Image::named_system_sized`, `Image::resize`,
  `Image::crop`, `Image::to_png`, `Image::to_jpeg`, `Image::template`, and
  `Image::with_representations`, all bounded by existing and new `MAX_*` image constants.
- Added `AppRunner::tray_icon_bounds` and `TrayIconImage::template`.

### macOS

- Message boxes use `NSAlert` suppression buttons, custom icons, and reassigned key equivalents so
  an explicit default or cancel index wins over AppKit's first-button default.
- File previews use `QLPreviewPanel` with a core-owned `QLPreviewItem`/data-source pair retained
  only while the panel is open.
- The system color and font panels are driven through one core-owned responder;
  `ColorPanelMode::OnClose` observes the panel's close notification instead of installing an action,
  so a closed panel retains no observation.
- Share sheets use `NSSharingServicePicker` anchored to the current window's content view, and
  biometric authentication uses `LAContext` with `LAPolicyDeviceOwnerAuthenticationWithBiometrics`,
  forwarding its background-queue reply through the event loop.
- `Display` now reports `CGDisplayRotation`, `CGDisplayIsBuiltin`, and
  `NSBitsPerPixelFromDepth(NSScreen.depth)`.
- `NSImage` conversion honors QuickGUI template metadata and additional backing-scale
  representations for Dock, About-panel, message-box, and menu icons, and `Image::named_system`
  resolves `NSImage` names and SF Symbols.

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
