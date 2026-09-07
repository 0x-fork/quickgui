# Go port parity

The reference is the last TypeScript frontend, commit `544388a`. A Go entry point
or a passing build alone does not establish feature parity. The public API,
example implementations, documentation, and relevant tests have been compared
with that reference. The source and delivery work below is complete; the user
is doing the remaining visual acceptance testing. Checked items do not claim
that every interaction has been visually retested on every platform.

## Framework

- [x] In-process purego host boundary; application builds use `CGO_ENABLED=0`.
- [x] Fine-grained signals, owner disposal, batching, keyed children, conditional
  content, callback children, and `func()` components.
- [x] Children-first primitive declarations, merged `ui.Style{}` records,
  conditional options, and the Go-compatible `quickguifmt` formatter.
- [x] Typed numeric children and accessors without `fmt` on the numeric binding path.
- [x] Primitive keyboard, mouse, gesture, focus, action, and drag/drop events,
  including typed payloads and conditional listener cleanup.
- [x] Primitive accessibility, overlays, tooltips, keymaps, and drag declarations.
- [x] All core styles, interaction states, named group states, gradients,
  transitions, and Markdown styling.
- [x] Compound component and SwiftUI API comparison against the original exports.
- [x] Application/window operations and lifecycle events, including cancellation.
- [x] Clipboard, menus, notifications, shortcuts, tray, desktop, displays,
  preferences, keyboard, power, permissions, and native images.
- [x] Auto-start, protocols/deep links, secure storage, updates, crash reporting,
  metrics, and file watchers, with request and subscription lifecycles.

## Examples

All 13 examples have Go modules, with their features compared against the
TypeScript versions. The validation record distinguishes automated checks from
the native interactions exercised during this migration.

- [x] AI Chat: original saved-history compatibility, virtualized individual
  messages, streaming, context limits, provider configuration, and conversation UI.
- [x] Alert dialog and file dialog: choices, filters, cancellation, and sheet/modal variants.
- [x] Components: original 42 demos plus a system context-menu demo, state handling,
  and styling.
- [x] Popover: controlled native and in-window surfaces, counters, and dismissal.
- [x] Routing: active links, history, parameters, queries, nested layouts, and fallback.
- [x] Sidebar vibrancy: all 15 materials, effect states, selected rows, and readouts.
- [x] Counter, SwiftUI, system API.
- [x] Styling: all ten original galleries, named groups, drag/drop, sticky headers,
  and scroll-snap declarations.
- [x] Herdr GUI: spaces, tabs, retained split terminals, agent launcher/status,
  sidebar resizing, persistence, themes, keyboard actions, and fonts.
- [x] Quick Git: all views, commit graph, multi-file diffs, staging controls,
  resizable panels, selection, repository/window lifecycle, and SVG icons.

## Documentation and delivery

- [x] Replace obsolete TypeScript application API snippets in `docs/` and all
  website locales. Keep TypeScript build tooling and native library packaging
  documentation where they still apply.
- [x] Audit scaffolding, native-extension replacement, package layout, and release
  instructions against the Go-only application architecture.
- [x] Run SDK and example checks with CGO disabled, CLI tests and TypeScript 7
  checks, website checks, appropriate Rust tests, and package/version checks.
- [x] Record live native behavior separately from source/build validation.
- [x] Update the homepage and localized website guides, including transitions,
  animation, navigation, and search.

## Validation record

The migration commit (`2c3e0f9`) contains existing Go tests and earlier native
validation. New work must record its own checks here; those earlier results do
not establish that the remaining checklist is complete.

### September 7 continuation

- Compared compound and SwiftUI public props/methods with `544388a`; existing Go
  compound controls cover the original exports. Primitive event coverage now
  includes all 31 event types and conditional listener cleanup.
- Added typed Go bindings for the existing application, window, clipboard,
  notification, shortcut, tray, platform, updater, storage, crash, and metrics APIs.
  Request-payload and lifecycle tests cover acceptance, errors, disposal, and
  malformed replies. Most OS services have source/test coverage rather than live
  permission or external-service checks.
- AI Chat reads the original version-1 history and imports the interim Go history
  when that original file is absent. It restores separate keyed virtualized
  messages, local drafts, provider settings, bounded Unicode streaming, and
  debounced atomic saves, including the final canceled response on quit.
- Native AI Chat fixture check: loaded original-format conversations; switched
  between saved drafts; Send with no key opened the system popover; Cancel kept
  the draft; Cmd-Q persisted the final edited draft. No provider request or
  credential write was performed.
- Numeric-child tests cover built-in signed/unsigned/float values and accessors,
  retained identity, one mutation per batch, and disposal. The counter displayed
  updated numbers natively. A fresh direct launch repainted the first click
  immediately, without resizing. Only the automation tool's background launch
  produced stale captures while accessibility state advanced; exposing the window
  restored presentation. No renderer change was needed for numeric children.
- `bash scripts/check-go.sh`: SDK and all 13 example modules passed with
  `CGO_ENABLED=0`, formatter and generated-file checks included.
- Local Apple M5 counter binding benchmark, 300 ms per case: numeric children
  137 ns/op, 352 B/op, 8 allocs/op; `fmt.Sprintf` 153 ns/op, 376 B/op,
  9 allocs/op. Includes mutation-batch allocation; excludes native drawing.

The entries below record subsequent example and delivery work. They are not a
claim that every original example interaction has been visually retested.

- Children-first example/scaffold migration: all 13 module suites and SDK checks
  passed; website client/SSR and all three search indexes built; CLI 55 tests and
  30-file release-version check passed.
- Documentation API migration: 21 Go snippets from 16 guides compiled with CGO
  disabled. Obsolete TypeScript application imports/JSX/renderer examples are gone
  from the guides and website locales; TypeScript remains the CLI/config language.
- Restored alert and file dialog choices, filtering, cancellation, and modal/sheet
  variants. Restored System API actions through typed Go bindings, including
  single-instance handling, subscriptions, and cleanup of tray/shortcut/power
  resources. All three modules compile; System API tests cover pending actions,
  retry after failure, and ignoring late callbacks after window closure.
- The CLI remains TypeScript/Bun, as requested. Added `quickgui.toml` discovery,
  explicit config selection, shared validation, and uncached reloads. CLI 63 tests,
  TypeScript 7 checking, and the website client/SSR/search builds passed.
- Both macOS native architectures were packaged and verified. Corrected `lipo`
  argument ordering in the npm release check. An extracted CLI/native tarball
  consumer with the local Go SDK and TOML-only config built an ad-hoc signed dev
  app and production app/DMG with CGO disabled. Public registry installation and
  release publication were not performed. The 30-file version check passed.
- Restored Popover's original comparison layout and controlled compound components.
  Live macOS checks covered both surfaces, counters, Close, Escape, outside-click
  dismissal, in-window edge adjustment, and local counter reset on remount.
- Restored Routing's original pages, navigation controls, nested settings, queries,
  and fallback. Fixed two SDK defects: empty index paths were omitted from the
  native payload, and leaf changes remounted common ancestor layouts. Regression
  tests cover explicit index serialization, retained layout state, query-only
  updates, and exact branch cleanup. Live checks covered Settings' index/child,
  active links, back/forward, Replace, dynamic parameters, queries, and fallback.
- SDK and all 13 example module checks passed again after the router fixes. The
  SDK also cross-compiled with CGO disabled for Windows amd64, Linux amd64, and
  macOS amd64; live native checks here use macOS arm64 only.
- Sidebar vibrancy restores the original translucent/opaque split, all 15 material
  choices, selected rows with SVG checkmarks, three effect states, and live readouts.
  Native checks covered HUD, appearance-based, under-page, list scrolling, and all
  three effect states without recreating the window.
- Alert dialog native checks covered Information/OK, the three-button save warning,
  and destructive-dialog cancellation. File dialog checks covered opening a real
  file, filename defaults, folder-only selection, and cancellation. No file was
  written or deleted by these checks.
- Styling restores all ten original demos. Native checks covered text/direction,
  gradients, borders, raster filters, hover transforms, named/nearest groups, the
  retained palette switch, local drag/drop, and nested sticky scrolling. Horizontal
  snap declarations match the original and have core tests; an automated native
  horizontal gesture has not yet been confirmed visually.
- Fixed a retained-scroll accessibility crash: when scrolling changes which nodes
  are exposed, send a complete accessibility snapshot instead of references to
  unpublished children. A regression walks the client's resulting tree across
  repeated scrolls; the native styling gallery now survives the same scroll.
- Fixed sticky headers painting below subsequent rows by assigning their default
  stacking level consistently for painting and input. The overlap regression and
  live nested-scroll check passed. All 139 UI-tree tests passed; both macOS native
  architectures were rebuilt.
- Restored the `FocusableWhenDisabled` primitive declaration from the original
  generated API. Its reactive binding test and another full SDK/all-13-example
  check passed with CGO disabled.
- Corrected invalid menu role names in System APIs and Herdr GUI. The System APIs
  app now starts and its asynchronous environment query completes; Herdr's module
  tests passed after the menu correction.
- Replaced TIFF round trips for native file icons and named system images with
  bounded AppKit RGBA8 rasterization. A main-thread integration test exercises
  8-bit and half-float native images, transparent edges, channel order, orientation,
  and symbol sizing. After rebuilding the arm64 host, the live Executable icon
  action completed with a 32-by-32 image. The Go app compiled in 361 ms and packaged
  in 422 ms using the prebuilt host.
- Rebuilt both macOS native architectures with the image fix, regenerated the
  local CLI/native npm archives, and passed the archive and 30-file version checks.
  The new main-thread image regression is included in the Rust source archive.

### Completion checks

- Quick Git retains all views, the commit graph, multi-file diffs, panel resizing,
  SVG actions, and repository/window lifecycle. Real temporary-repository tests
  cover staging and unstaging partial patches, deleted files, missing final
  newlines, overlapping selections, and ambiguous diff-header paths. Process
  tests cover output limits, cancellation, and child-pipe shutdown. Restored menu
  actions use the active repository window.
- Herdr restores spaces, tabs, retained split PTYs, agent discovery/status, the
  agent sheet, both sidebar dividers, original-format persistence, themes,
  shortcuts, and bundled terminal fonts. Model tests cover selection, closing,
  node retention, agent discovery, launch arguments, and persistence. Terminal
  font appearance was confirmed by the user; remaining visual testing is theirs.
- SwiftUI restores all 13 gallery pages with retained shared state. Fixed the
  selected page's subtree, selected-sidebar hover, deferred window ownership,
  and reverse-host identifier conversion. Gallery tests passed; native checks
  exercised button, slider, text input, navigation, popover, and reverse-hosted
  input/Save behavior.
- The Components gallery contains all 42 original demos plus a separate System
  Context Menu page using `native.Window.PopupMenu`. It includes checked items,
  a sorting submenu, disabled actions, keyboard-accessible opening, and close/error
  feedback. The new page has source/build validation; the user is testing its
  native appearance and interactions. Earlier native checks exercised lazy
  Accordion content and keyboard Combobox selection. Checkbox regression tests
  cover repeated toggles, uncontrolled callbacks, parent/child mixed state, and
  native group updates. Indicators now follow their checked state and use
  centered SVG marks instead of font glyphs. Fixed Go activation handlers that
  ignored read-only declarations for checkboxes, switches, radios, and radio
  groups. Controlled and uncontrolled regression cases verify that read-only
  activation preserves the click event without changing state or emitting a
  value-change callback; editable controls still change normally.
- Counter native checks covered numeric updates, newly opened window ordering,
  and window shortcuts. Restored the default macOS Minimize menu action while
  preserving application-defined menus/keymaps; five menu tests passed. The
  diagnostic window state reported minimized after Cmd-M, and macOS reported
  the application hidden after Cmd-H.
- System API native checks exercised environment, displays, preferences,
  keyboard, power, window/capability queries, second-instance delivery, and the
  executable icon. Permission-gated and external services retain source and
  request/subscription test coverage rather than exhaustive live validation.
- Rechecked the CLI scaffold, Go module/version references, native library
  resolution, TOML configuration, npm archive layout, and native-extension
  documentation. The CLI stays TypeScript/Bun; applications use Go and purego.
- Final SDK and all 13 example checks passed with `CGO_ENABLED=0`, including
  formatting and generated-file checks. The SDK cross-compiled for macOS arm64
  and amd64, Linux amd64, and Windows amd64. CLI tests passed with 63 tests and
  271 assertions; TypeScript 7 and the 30-file version check passed.
- Rust checks passed: 139 UI-tree tests, five macOS menu tests, and the main-thread
  native image integration test. Both macOS native architectures were packaged
  in the local 0.1.3 CLI/native archives and archive verification passed.
- The homepage, metadata, and 1200-by-630 social image now emphasize fast Go
  builds. The website includes English, Chinese, and Japanese transition and
  animation guides covering state changes, supported properties, timing,
  reduced motion, and animated images. The checkbox reference now shows a
  conditional SVG indicator and explains read-only behavior in all three locales.
  Its example and all four motion examples compile and mount;
  outline checks, TypeScript 7, client/SSR builds, and 380 search entries per
  locale passed. Each localized guide returned HTTP 200 with its navigation,
  headings, and search entry. `git diff --check` passed.

### Acceptance boundary

The user requested completion of the implementation and will perform visual
testing. The native checks recorded above ran on macOS arm64; cross-compilation
does not establish Linux, Windows, or macOS amd64 runtime acceptance. Local
package/consumer checks do not establish public registry installation. No release,
website deployment, or external-service acceptance is claimed by this checklist.
