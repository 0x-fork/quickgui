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
- Added window lifecycle events `Event::Minimized`, `Event::Maximized`,
  `Event::FullscreenChanged`, `Event::FirstPresented`, `Event::OcclusionChanged`, and
  `Event::WindowLevelChanged`. `FirstPresented` is the flicker-free ready-to-show moment for a
  window created with `WindowOptions::show(false)` and is delivered exactly once; the state events
  are equality-suppressed and derived event-driven, with no observer, timer, or idle sampling.
- Added the `Event::WillResize`/`Event::WillMove` constrain hooks with
  `EventContext::constrain_resize` and `constrain_move`. Each proposal issues at most one
  corrective native resize or move, and the corrective value is remembered so a hook can never
  loop. Both validate through the existing window-bounds range.
- Added `Application::on_did_become_active` and `on_did_resign_active`.
- Extended `WindowLevel` with `Floating`, `ModalPanel`, `MainMenu`, `Status`, `PopUpMenu`, and
  `ScreenSaver`, plus `WindowLevel::macos_level()` and `is_above_normal()`. Non-macOS backends
  collapse every above-normal level to the topmost hint while retaining the exact requested level.
- Added `move_window_top`, `move_window_above`, `set_ignore_mouse_events(ignore, forward)`,
  `set_window_enabled`, `set_aspect_ratio`/`clear_aspect_ratio`, and
  `set_window_button_visibility`, each with a `_handle` variant, matching `WindowOptions` builders,
  and new `WindowState` fields. Aspect ratios are validated against the new
  `MAX_WINDOW_ASPECT_RATIO` bound.
- Added the serde-serializable `WindowRestoreState`, `WindowState::restore_state`,
  `WindowRestoreState::resolve`, `ResolvedWindowRestoreState`, and `WindowOptions::restore`, which
  validate persisted geometry against the connected displays, clamp into the remembered work area,
  and fall back to centered placement rather than placing a window off every display.
- Added application-shell services on `EventContext`: `set_activation_policy`,
  `activate_application`, `hide_application`, `unhide_application`, `request_dock_attention`,
  `cancel_dock_attention`, `set_dock_visible`, `set_secure_keyboard_entry`, `beep`,
  `applications_folder_support`, `move_to_applications_folder`, `is_application_packaged`, and
  `exit_with_code`, plus the free `quickgui::is_application_packaged()`.
- Added deterministic coverage through `TestAppContext::simulate_minimize`, `simulate_maximize`,
  `simulate_fullscreen_change`, `simulate_occlusion_change`, `simulate_first_presented`,
  `simulate_window_resize`, `simulate_window_move`, `window_restore_state`, `exit_code`, and the
  `TestApplicationShell` snapshot.
- Added controlled unstyled `Slider` with bounded `MAX_SLIDER_THUMBS` values, step snapping, thumb
  ordering, horizontal or vertical captured pointer arithmetic, typed arrow/page/Home/End actions,
  and Slider accessibility with numeric value, minimum, maximum, step, and orientation.
- Added `Progress` and `Meter` descriptors with determinate and indeterminate semantics, optional
  low/high/optimum meter markers, and no framework-owned animation, so an indeterminate indicator
  never keeps a settled window awake.
- Added `NumberFieldState` and `NumberField` composing the existing `text_input()` with
  caller-supplied decimal and grouping separators, sign and exponent policy, fixed precision,
  commit-time clamping and formatting, arrow/wheel/stepper stepping, press-and-hold repeat on exact
  one-shot deadlines that leave no idle source once released, and SpinButton accessibility with
  numeric value, bounds, step, and invalid state.
- Added `SplitterState` and `Splitter` for resizable panes with size-conserving drags from captured
  pointer deltas, per-pane minimum sizes, collapse and restore, proportional `set_total` rescaling,
  typed keyboard resizing, and focusable Splitter handles carrying numeric value, bounds, axis, and
  a controls relationship to the pane they resize.
- Added `Toolbar` with the Toolbar role, one roving Tab stop over the caller's ordered items,
  per-item arrow/Home/End navigation, disabled-item skipping, and optional looping.
- Added `Toggle` and `ToggleGroup` with pressed-state button semantics distinct from checkbox state,
  single or multiple selection over inline bounded pressed values, and the same roving focus
  contract as the toolbar.
- Added `ToastManager` and `ToastViewport` with a bounded `MAX_TOASTS` queue, exact one-shot
  auto-dismiss deadlines reported through `next_deadline`, pause on hover or focus, polite or
  assertive live regions chosen by toast kind, focused Escape dismissal, and caller-owned
  viewport/root/title/description/action/close parts.
- Added `AccessibilityRole::Slider`, `SpinButton`, `ProgressIndicator`, `Meter`, `SplitterHandle`,
  `Toolbar`, `ToggleButton`, `MenuBar`, `Alert`, and `Status`, plus public
  `AccessibilityOrientation`, `AccessibilityLive`, and `AccessibilityValueRange` with
  `Element::accessibility_orientation`, `accessibility_live`, and `accessibility_value_range`.

### macOS
- Native menu items now honor an explicit declaration accelerator ahead of both the keymap binding
  and the AppKit standard binding for a role, and hidden items set `NSMenuItem.hidden` and no
  longer claim the Command-W close fallback.
- An "Open Recent" system submenu is built with a `clearRecentDocuments:` "Clear Menu" item so
  `NSDocumentController` populates it.
- Dock menu items now render their declared accelerators.




- Window levels now apply the exact `NSWindowLevel` after Winit's three-level hint;
  `move_window_top`/`move_window_above` use `orderFront:` and `orderWindow:relativeTo:` so a window
  restacks without activating the application or becoming key.
- Click-through uses `ignoresMouseEvents` with `acceptsMouseMovedEvents`, so a forwarding window
  keeps AppKit's existing event-driven `mouseMoved:` stream without a tracking area or timer.
- `set_window_enabled` expresses "visible but inert" with `ignoresMouseEvents` plus refusing and
  resigning key-window status; `set_aspect_ratio` uses `contentAspectRatio`; and
  `set_window_button_visibility` hides the standard traffic-light buttons while keeping the
  titlebar.
- Application-shell services use `NSApplicationActivationPolicy`, `NSApp.activate` /
  `activateIgnoringOtherApps:`, `hide:`/`unhide:`, `requestUserAttention:` /
  `cancelUserAttentionRequest:`, Carbon `EnableSecureEventInput`/`DisableSecureEventInput` (guarded
  by `IsSecureEventInputEnabled` so the process-global counter stays balanced), and `NSBeep`.
- `on_did_become_active`/`on_did_resign_active` reuse the existing application observer, adding no
  new native observer.


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
