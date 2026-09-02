# Changelog

All notable user-facing changes to QuickGUI are recorded here.

## Unreleased

### Framework

- Added a bounded `CrashReporter` behind the default `crash-reporter` feature: a panic hook that
  writes a size-limited JSON report atomically, an async-signal-safe native fatal-fault path
  (`SIGSEGV`/`SIGBUS`/`SIGILL`/`SIGFPE`/`SIGABRT` through `sigaction`, `SetUnhandledExceptionFilter`
  on Windows) built on a pre-opened descriptor and a pre-rendered report template, bounded
  retention, `last_crash_report`/`pending_reports`/`delete_report`, mutable extra parameters, and
  `upload_pending` over HTTPS. The `Watchdog` main-thread hang detector is opt-in and documented
  with its idle cost.
- Added `ProcessMetrics`, `SystemMemory`, and `CpuUsageSampler`: explicit, allocation-light process
  and system readings using `proc_pidinfo`/`proc_pid_rusage`/`host_statistics64` on macOS,
  `/proc` on Linux, and `GetProcessTimes`/`GlobalMemoryStatusEx` on Windows, reporting `None`
  rather than a guess where a platform does not expose a value.

### JavaScript tooling

- Added `CrashReporter` and `Metrics` to `@quickgui/native`, both Promise-backed by `AsyncTask`,
  and an `onProgress` option for `Updater.downloadAndStage` delivered through a napi threadsafe
  function from the download worker thread.
- Added `quickgui keygen` and `quickgui build --update-manifest [--update-base-url <url>]`, which
  produce the exact artifact the Rust updater installs for each target, sign it with `minisign` or
  `rsign`, and write a `latest.json` whose platform keys match `default_update_target()`.
- Added `documentTypes` file associations that reach macOS `Info.plist`, the Linux desktop entry
  and `shared-mime-info` package, and the Windows NSIS registry; `icon` PNG-to-`.icns`/`.ico`/
  `hicolor` generation written in pure TypeScript; Linux AppDir/AppImage output and a pure
  TypeScript `.deb` writer; an NSIS installer with shortcuts, uninstall registration, protocol
  handlers, and an Authenticode `signtool` hook; and `quickgui build --mas` for Mac App Store
  `.pkg` submission.

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
