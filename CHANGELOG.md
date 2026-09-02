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

- Added per-input text checking with `.spellcheck(..)`, `.grammar_check(..)`, `.autocorrect(..)`,
  `.smart_quotes(..)`, `.smart_dashes(..)`, `.text_replacement(..)`, and
  `.lookup_on_force_click(..)`, layered over an application-wide `set_default_text_checking(..)`
  policy. Checking runs once on a single 300 ms settle deadline over at most 16 KiB around the
  caret, retains at most 512 flagged ranges, projects them as wavy underlines merged over the
  controlled run table without rewriting it, and skips the word the caret is inside. A settled input
  holds no timer.
- Added the `SpellCheckProvider` trait plus `set_spell_check_provider`/`clear_spell_check_provider`,
  bounded `guesses`/`learn`/`ignore` with per-input document tags, boundary autocorrect and
  replacement-dictionary substitutions applied as one undoable edit with a "Change back" record, and
  insertion-time smart quotes and dashes that never rewrite paste or IME commits. Portable targets
  fall back to `NoSpellCheckProvider` and report `TextServiceError::Unsupported`.
- Added `spelling_menu_items(..)` and the typed `ReplaceWord`, `LearnWord`, `IgnoreWord`, and
  `LookUpSelection` actions so applications can build the standard spelling context menu and bind
  dictionary lookup, plus `show_definition_for(..)` and `word_range_at(..)`.
- Added bounded literal find and replace: `FindState` with case-sensitive and whole-word options,
  4,096 retained matches, wrap-around `find_next`/`find_previous`, distinct all-match and
  current-match highlight runs, a `2 of 17` count label, and `replace_current`/`replace_all` results
  an application applies as one controlled edit. Added the unstyled `FindBar` component over
  caller-owned root, query, replace, count, next, previous, replace, replace-all, and close parts
  with `find_bar_key_bindings()` for Return, Shift+Return, and Escape.
- Added an application-wide `UndoManager` with 256 bounded named entries, `begin_group`/`end_group`
  collection, a value-based `UndoableChange<T>` shortcut, `undo_action_name`/`redo_action_name`
  menu titles, `Undo`/`Redo` actions with `undo_key_bindings()`, and a documented policy that a
  focused text input's own history claims Undo and Redo before the manager sees them.
- Added `examples/text_services.rs` and new `docs/text-and-forms.md` sections covering spelling,
  dictionary lookup, find and replace, and application undo.

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

- Added element background gradients: `bg_linear_gradient`, `bg_radial_gradient`,
  `bg_radial_gradient_at`, `bg_conic_gradient`, and the general `bg_gradient`, backed by a new
  `Gradient`/`ColorStops`/`GradientKind` API with `MAX_GRADIENT_STOPS` (8) stops, CSS angles or
  `GradientDirection`, `RadialGradientShape`/`RadialGradientExtent`/`GradientCenter`, and
  linear-sRGB, sRGB, or Oklab interpolation. Gradients are evaluated analytically in the existing
  instanced shape draw, honor rounded corners, borders, clipping, subtree opacity, and damage, and
  are swappable in hover, active, focus, validation, and drag states through
  `ElementStateStyle::bg_gradient`. `MAX_GRADIENTS_PER_FRAME` (4,096) bounds the per-frame upload.
- Extended `Background` so retained paths and canvas fills accept the same multi-stop linear,
  radial, and conic gradients instead of only two-stop linear gradients.
- Added per-corner radii: `rounded_tl`, `rounded_tr`, `rounded_br`, `rounded_bl`, `rounded_t`,
  `rounded_b`, `rounded_l`, `rounded_r`, `corner_radii(Corners)`, and `rounded_full()`. Radii are
  capped by `MAX_CORNER_RADIUS` and reduced by the CSS uniform-scale rule; element box shadows
  follow the same per-corner geometry.
- Added `border_solid()`, `border_dashed()`, and `border_dotted()`. Dash geometry is analytic arc
  length along the rounded outline, with the period scaled so a whole number of repeats closes the
  outline.
- Added `outline(width, color)`, `outline_offset(px)`, `outline_style`, `outline_dashed`,
  `outline_dotted`, and `outline_none`, plus `ElementStateStyle::outline`/`outline_offset` for
  focus rings. Outlines are painted outside the border box and never affect layout, bounded by
  `MAX_OUTLINE_WIDTH` and `MAX_OUTLINE_OFFSET`.
- Added raster element backgrounds: `bg_image(image, BackgroundSize, BackgroundRepeat,
  BackgroundPosition)` with `bg_image_cover`, `bg_image_contain`, `bg_image_tiled`, and
  `bg_image_none`. Tiles reuse the existing bounded image primitive and GPU texture cache, are
  masked by the element's rounded corners, and are capped by `MAX_BACKGROUND_IMAGE_TILES` (256).
- Added bounded CSS-shaped color filters: `Filter`, `Filters`, `ColorMatrix`,
  `MAX_FILTERS_PER_ELEMENT`, `Element::filters`/`filter`, and the `brightness`, `contrast`,
  `saturate`, `invert`, `sepia`, and `hue_rotate` shorthands. `grayscale(bool)` now routes through
  the same chain, and `ImagePrimitive::grayscale` is expressed as a `ColorMatrix`. Filters apply to
  an element's own image and background-image pixels; subtree filters, blur, and drop-shadow are
  not implemented because they require an offscreen group texture.
- Added the `effects` example.

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
- Added `AppRunner`-level window stacking, input-policy, and application-shell commands, so an
  externally pumped host reaches `move_window_to_top`, `move_window_above`,
  `set_window_ignore_mouse_events`, `set_window_enabled`, `set_window_aspect_ratio`,
  `set_window_button_visibility`, `set_window_always_on_top`, `set_activation_policy`,
  `activate_application`, `hide_application`/`unhide_application`, `request_dock_attention`/
  `cancel_dock_attention`, `set_dock_visible`, `set_secure_keyboard_entry`, `beep`,
  `applications_folder_support`, `move_to_applications_folder`, `is_application_packaged`, and
  `exit_with_code` under the identical queue bounds as the `EventContext` forms.
- Added `AppRunner::set_window_menus`, `AppRunner::use_application_menus_for_window`, and
  `AppRunner::show_window_popup_menu`. Per-window menus and native popup menus need the runtime's
  window-scoped `EventContext`, so each is declared as a deferred request that the runtime resolves
  inside its own effect cycle. Both queues carry the public `MAX_PENDING_NATIVE_POPUP_MENUS` bound,
  the popup response completes when the menu closes, and a popup declared for a window that closes
  first completes with `PlatformError::Unavailable`.

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





- Added an `NSSpellChecker`-backed default text checking provider covering spelling, guesses,
  corrections, the user replacement dictionary, learned and ignored words, and per-input
  `uniqueSpellDocumentTag` sessions released on unmount, with explicit UTF-8/UTF-16 offset
  conversion at the AppKit boundary.
- Added the macOS dictionary popover through `NSView showDefinitionForAttributedString:atPoint:` on
  the key window's content view, positioned at a window-local logical point.





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


### JavaScript tooling
- Added declared `PopoverMenu` and `ContextMenu` compound components to `@quickgui/solid`. Rows are
  one bounded JSON model rather than JSX children, so the Rust core keeps ownership of validation,
  highlighting, typeahead, checkbox/radio policy, submenu models, work-area placement, and the
  cursor-point native surface. `onSelect` reports the application's own stable item id, and the
  binding installs `popover_menu_key_bindings()` once so declared menus adopt the core's contextual
  navigation.
- Added CSS Grid props (`gridTemplateColumns`/`gridTemplateRows`, `gridAutoFlow`, and the
  `gridColumn`/`gridRow` shorthands with their start/end/span forms) and the complete paint
  transition declaration (`transitionProperty`, `transitionDuration`, `transitionTimingFunction`,
  `transitionMaxFps`, plus object and CSS-shorthand forms) mapped onto the core's own
  `TransitionProperties` flags and easing curves.
- Added retained `Image` and `Shader` nodes: a filesystem path stays a lazy core `ImageResource`, a
  base64 `data:` URL is decoded once, `fit` selects the core's `ObjectFit`, and `shaderParameters`
  fills the core's four fixed vectors behind its WGSL validation.
- Added `Progress`, `Meter`, and `Toggle` compound parts over the core's value-range and
  toggle-button descriptors.
- Added declared input listeners — `onKeyDown`/`onKeyUp`, `onMouseDown`/`onMouseUp`/`onMouseMove`,
  `onDoubleClick` with the exact native click count, `onWheel`, `onContextMenu`, `onPinch`,
  `onRotate`, `onSmartMagnify`, `onPressure`, and `onFocus`/`onBlur` — with bounded JSON payloads
  and `keyEventFromEvent`/`mouseEventFromEvent`/`wheelEventFromEvent`/`gestureEventFromEvent`
  decoders. A declared `tabIndex` now makes an ordinary container focusable, matching the web.
- Added a bounded `keymap` prop whose Electron-shaped accelerators are parsed by the core's own
  `Accelerator::parse` and dispatched to JavaScript as one `onAction` event carrying the binding id.
- Added declared drag and drop: `draggable` carries an application-local id plus optional text, URL,
  or file payloads promoted to other applications, `dropKinds` declares the accepted payload kinds
  ahead of the native drag, and `onDragStart`/`onDragEnd`/`onDrop`/`onFilesDropped` report the
  core's typed outcome.
- Bumped the hosted mutation protocol to version 20 for the new component parts, node tags, and
  declared listener properties.

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
- Added window lifecycle events to `@quickgui/native`: `window.on("minimize" | "restore" |
  "maximize" | "unmaximize" | "enterFullScreen" | "leaveFullScreen" | "readyToShow" |
  "occlusionChange" | "levelChange" | "willResize" | "willMove" | "resize" | "move" | "focus" |
  "blur" | "appearanceChange")`, plus `app.on("activate" | "deactivate")`. `willResize` and
  `willMove` are notifications: the core answers the window manager synchronously, so the narrowing
  is declared ahead with `window.setResizePolicy({ aspectRatio, minimum, maximum, snap })` and
  `window.setMovePolicy({ keepOnScreen })` instead of blocking on a JavaScript callback.
- Added `window.setAlwaysOnTop(flag, level?)` accepting the Electron level names over the extended
  core `WindowLevel`, `window.moveTop()`, `window.moveAbove(other)`,
  `window.setIgnoreMouseEvents(ignore, { forward })`, `window.setEnabled`,
  `window.setAspectRatio(ratio | null)`, `window.setWindowButtonVisibility`, `window.setHasShadow`,
  and `window.getRestoreState()` round-tripping into `WindowOptions.restoreState`.
- Added the application shell to `app`: `setActivationPolicy`, `focus({ steal })`, `hide()`,
  `show()`, `dock.bounce(type)`/`dock.cancelBounce(id)`/`dock.hide()`/`dock.show()`/
  `dock.isVisible()` alongside the dock badge, icon, and menu, `setSecureKeyboardEntryEnabled`,
  `isInApplicationsFolder()`, `moveToApplicationsFolder()`, `isPackaged`, and `Shell.beep()`.
  `app.exit(code)` now maps onto the core's `exit_with_code`. Mutations the operating system applies
  at once stay fire-and-forget; operations with a native answer resolve a Promise from an
  asynchronous event.
- Added `Menu.popup(items, { window, x, y })`, which resolves after the popup closes and releases
  its item callbacks, and `window.setMenu(definitions | null)`. Both reuse `serializeNativeMenu`, so
  popup and per-window items keep the same roles, marks, icons, accelerators, and bounds as the
  application menu.
- Added application-level `SpellChecker.learnWord(word)` and `SpellChecker.ignoreWord(word)` over
  the installed core spell-check provider, bounded to 256 UTF-8 bytes.

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


- Added Base-UI-shaped Solid `Checkbox`, `Radio`, `RadioGroup`, `Switch`, `Tabs`, `Collapsible`,
  `Accordion`, `Field`, and `Fieldset` compound parts. Each part is one native node that declares
  which Rust core part descriptor to rebuild, so the core keeps ownership of part identity, roles,
  toggle and selected state, roving Tab and arrow navigation, label/description/error
  relationships, and whether an inactive tab or disclosure panel is mounted at all. Controlled
  values, scope keys, and item values are declared ahead of time as bounded protocol properties;
  nothing about a component is answered by a synchronous JavaScript callback.
- Added Solid `Dialog` and `AlertDialog` compound parts for the Rust core's caller-styled in-window
  modal surface, separate from the operating-system panels in `@quickgui/native`'s `Dialog`
  namespace. The overlay portal is mounted by the core only while the dialog is open, and the
  Escape/backdrop dismissal policy is declared ahead of time instead of answered by a callback.
- Added a `tooltip` property with `tooltipPlacement`, `tooltipDelay`, `tooltipGap`, and
  `tooltipViewportMargin` to every Solid host component, projecting the Rust core's delayed,
  pointer-passive tooltip and its native accessibility description.
- Raised the mutation protocol to version 19 for the new component-part and tooltip properties.

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
