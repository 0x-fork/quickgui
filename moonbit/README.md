# QuickGUI for MoonBit

A native MoonBit frontend alongside Go. Views use child lists inspired by [Rabbita](https://github.com/moonbit-community/rabbita), with fluent styles, properties, and event handlers. Constructors create retained nodes once; signals update the affected properties or children. The same QuickGUI core handles layout, rendering, accessibility, input, state styles, and transitions.

```moonbit
fn counter() -> @ui.Element {
  let (count, set_count) = @ui.create_signal(0)
  @ui.div([
    @ui.text("Count: \{count()}").font_size(24),
    @ui.button("Increment")
    .on_click(() => set_count(count() + 1))
    .px(16)
    .py(8)
    .border_radius(8)
    .bg(@ui.rgb8(45, 105, 180))
    .hover(s => s.bg(@ui.rgb8(56, 122, 204))),
  ])
  .size_full()
  .flex_col()
  .items_center()
  .justify_center()
  .gap(12)
}
```

Pass an array to containers, including `div([])` for an empty view. Buttons accept a string, a single element, or an element array. Constructors accept only children or content. Set properties and handlers through fluent methods: `button("Save").on_click(save)`, `input().placeholder("Name").on_input(update_name)`, and `checkbox([]).checked(true)`. Use `.style(shared)` for reusable styles; fluent modifiers and styles merge in declaration order.

The QuickGUI CLI compiles expressions in text, button content, fluent properties and styles, and child lists into fine-grained bindings. Use `let (count, set_count) = @ui.create_signal(0)` and read `count()` directly in the view. Components construct once; each binding tracks only the signals it reads. Child arrays attach directly, without wrapper nodes or a virtual DOM.

Use `@ui.create_memo(() => count() * 2)` for cached derived state and read it with `doubled()`. Effects, cleanup, batching, and untracked reads are available as `@ui.create_effect`, `@ui.on_cleanup`, `@ui.batch`, and `@ui.untrack`.

## Run

Install [MoonBit](https://www.moonbitlang.com/download/), a C compiler, and Bun. Tested with `moon 0.1.20260827` / `moonc v0.10.11`. The CLI builds a standalone native executable that dynamically loads the same shared libraries as Go; there is no JavaScript runtime or IPC bridge. Element and node handles use MoonBit value structs.

Development builds use MoonBit's fast native backend (`MOONBIT_NEW_NATIVE=1`) on Apple Silicon macOS and x64 Linux/Windows. It emits native object code directly, so an application edit avoids recompiling generated C. Native bridge and runtime objects stay cached in `.quickgui/moonbit`. Release builds and other hosts use the optimizing C backend. Set `MOONBIT_NEW_NATIVE=0` when running `quickgui dev` to opt back into C compilation for troubleshooting.

From the QuickGUI checkout:

```sh
bun install
bun packages/cli/src/cli.ts dev --project examples/moonbit-counter
```

The repository's `moon.work` resolves the SDK locally. To create another project:

```sh
quickgui init my-app --frontend moonbit
cd my-app
bun run dev
```

The registry dependency is `egoist/quickgui`. Before this SDK is published to Mooncakes, use `--no-install`, add the generated module and this `moonbit` directory to a `moon.work`, and use a local CLI dependency. Existing Go projects remain the default.

```toml
name = "My App"
identifier = "com.example.myapp"
frontend = "moonbit"
entry = "main"
```

`quickgui dev` rebuilds and restarts after source edits; `quickgui build` packages the executable, native libraries, fonts, and resources; `quickgui fmt` runs `moon fmt`. `quickgui check` and `quickgui test` use the same view compiler as builds; add `--release` to test the release backend. Raw `moon check` still checks the original MoonBit types, but raw `moon test` bypasses implicit view bindings. Native builds currently target the build host's OS and architecture. macOS has been tested; Linux and Windows runtime/visual acceptance remain outstanding.

The compiler keeps original sources unchanged and caches generated files in `.quickgui/moonbit-sources`. Compiler diagnostics, test failures, and symbolicated stack locations printed by the CLI map back to original files. Native debugger breakpoints and stepping currently target generated files; original-file native debugger mapping is not yet supported. Editor completion and type checking use ordinary MoonBit source. See the [compiler contract](../docs/architecture/moonbit-views.md).

## Packages and ownership

- `egoist/quickgui/ui`: components, styles, signals, memos, effects, batching, cleanup, routing, and SwiftUI hosts.
- `egoist/quickgui/reactive`: low-level signal handles and ownership internals.
- `egoist/quickgui/native`: windows, native events, asynchronous commands/dialogs/services, extension loading, CPU-only calls, and retained-node primitives.
- `egoist/quickgui/protocol`: generated core constants and the bounded binary mutation encoder.
- `egoist/quickgui/terminal`: the optional terminal surface; package its separate native extension when used.

Components consistently return `@ui.Element`. Start the app once from `main` with `@native.run(() => { ignore(@ui.window(counter)) })`; handle its `Result`. Applications may open additional windows from callbacks. Each window owns its tree independently.

Window `options` use the native host's JSON schema: `background` is a packed `0xAABBGGRR` integer (`0` for a transparent color), `show` controls initial visibility, and `trafficLightX` / `trafficLightY` position macOS window buttons. For example, `options={ "background": 0, "show": false }` creates a hidden window with a transparent background color.

Read getters directly at native view sites: `text("Count: \{count()}")`, `input().value(name())`, or `.width(width())`. Plain setup such as `let initial = count()` remains a snapshot; event handlers execute on events. Binding expressions must be pure. Custom components returning `@ui.Element` accept plain typed values: `fn label(count : Int) -> @ui.Element { @ui.text("Count: \{count}") }` stays reactive when called as `label(count())`. The compiler preserves live expressions through direct component calls across local files and workspace packages; no annotations are needed. Explicit getter parameters still work. Passing a component as an ordinary function value uses normal MoonBit evaluation. Explicit `bind_*` methods remain available for lower-level composition and code built directly with Moon.

Use `if` expressions inside child lists for alternatives, or `..if expanded() { [details()] } else { [] }` to omit a branch. Boolean branch selection preserves nodes until the selected branch changes. Computed arrays bind at collection scope; use `children_keyed(read, key, render)` for lists that must retain per-item nodes, focus, and state. Its render callback receives a row signal. Keyed children alone do not virtualize a long list.

Effects and event callbacks execute on one UI worker thread. The native main thread only copies callbacks into a bounded C queue, and both threads sleep when idle. Do not call the MoonBit runtime from arbitrary extension threads. Subscriptions, asynchronous replies, and rows are retired with their owners. No per-frame tree reconstruction or idle polling occurs.

## Styling and native services

Rust's layout conveniences are available on both `Element` and `Style`: `.grid_cols(3)`, `.col_span_full()`, `.flex_col_reverse()`, `.justify_between()`, `.p_4()`, `.mx_auto()`, and `.rounded_lg()`. The generator checks 325 helpers against Rust, including spacing and radius presets. `.flex()` selects flex display; use `.flex_1()` for grow 1, shrink 1, and a zero basis. Popup anchor stickiness uses `.anchor_sticky(value)`; `.sticky()` selects sticky positioning.

Numeric dimensions are pixels; strings support native keywords and percentages. Colors use `rgb8` or `rgba8`. `.style(style)` merges with earlier declarations, with later values taking precedence. Styles snapshot when applied; later changes to a reusable builder do not update mounted nodes. `.bind_style(() => style)` owns its declared properties and clears properties omitted on the next run. Construct a complete merged style inside that callback when conditional styles need fallback values.

```moonbit
button.bind_style(() => @ui.style()
  .bg(@ui.rgb8(45, 105, 180))
  .when(disabled.get(), s => s.opacity(0.5)))
```

`.hover`, `.active`, and `.focus_style` declare native interaction states. The general `.property(code, value)` and `.on(kind, listener_property, callback)` methods expose the core protocol when a convenience wrapper is absent. Transitions use `.transition_duration(150).transition_property("opacity,background-color").transition_easing("ease-out")`; durations are milliseconds.

Native platform commands use `@native.command(json, callback)` and services use `@native.invoke(operation, params, callback)`. Results arrive on the UI thread and are cancelled when their registering owner is disposed. `@native.call` only supports the core's CPU-only allowlist. `Window.dialog` returns paths for open/save dialogs and the button identifier for alerts. Platform services without a convenience wrapper remain available through these command and service APIs.

To consume an independent extension, list its directory:

```toml
[native]
extensions = ["vendor/my-service"]
```

Directory paths are relative to the app project. The CLI reads `quickgui.extension.json` inside each directory and validates and packages the provider; `native.run` loads and registers it before startup. Invoke services with `@native.invoke("extension/my-service/method", params, callback)` and receive persistent session events with `@native.on_event("extension-event", listener, window=0)`, filtering `event.target` for the session. Direct runs may declare `require_extension(name, version)` before `run` and set `QUICKGUI_LIBRARY` / `QUICKGUI_EXTENSION_DIR`. MoonBit does not use Go import discovery. See the [extension contract](../docs/architecture/extensions.md).

## Quick Git and compound components

Create scoped parts from their component root. All component builders return `Element`:

```moonbit
fn notifications() -> @ui.Element {
  let (checked, set_checked) = @ui.create_signal(false)
  let box = @ui.checkbox([])
    .bind_checked(() => checked())
    .on_checked_change(value => set_checked(value))
  box.children([box.slot(@ui.Indicator)]).width(20).height(20)
}
```

`tabs([]).slot(@ui.Tab, value="settings")`, `accordion([]).slot(@ui.Item, value="details")`, and `dialog([]).slot(@ui.Portal)` use the same explicit scope model. Descendants created from an item retain its value and index. Static setters initialize component-owned state; `bind_*` accessors make that property controlled. Application event handlers compose with default actions, and `Event.prevent_default()` cancels a default action. Native interaction controllers, transitions, keyboard focus, and pointer capture stay in the core.

The [MoonBit reference](https://quickgui.dev/docs/moonbit/components) has the same component categories as Go, with MoonBit examples for each component. `scripts/check-moonbit-docs.ts` compiles every example; `scripts/generate-moonbit-components.ts` maintains the fluent wire bindings.

The [components gallery](../examples/components-moonbit/README.md) matches the Go gallery's 43 demos and sidebar, including mixed/read-only controls, native menus, dialogs, a 5,000-row virtual table, and lazy trees. Run `bun --cwd examples/components-moonbit dev`; `bun --cwd examples/components-moonbit check --native` renders every demo in both appearances through the real host.

[Quick Git in MoonBit](../examples/quick-git-moonbit/README.md) exercises the frontend in a complete application: changes and partial staging, a virtualized history graph, branches, worktrees, stashes, dialogs, native menus, and multiple windows. Run it with `bun --cwd examples/quick-git-moonbit dev`. Its process/file service is a separate extension; Git logic and UI stay in MoonBit.

Use `table`, `table_row`, and `table_cell` for core table layout and selection. `on_change` reports `visibleRange` and `selectedRanges`; `visible_range` normalizes the exclusive end index. Mount that range with `children_keyed`. The table knows the full row count and computes scrolling; the example demonstrates bounded row construction. `resizable_panel` uses the core splitter and reports new widths for persistence. `part` exposes other compound roles without duplicating their behavior.

Native menus use `menu_action`, `menu_role`, `submenu`, `set_application_menu`, and `Window.popup_menu`. `watch_files` subscribes to OS notifications and returns a cancellation function. Both subscriptions and menu callbacks retain the registering owner. Use `Window.with_owner` when an operation should outlive the row or dialog that starts it. Window creation is asynchronous: observe `window-ready-to-show` before querying native geometry/state.

## Development

MoonBit can also implement an independent native service extension:

```sh
bun packages/cli/src/cli.ts init-extension my-service --type moonbit --no-install
cd my-service
bun run build
moon -C native test --target native --release
```

Put operations in `native/extension.mbt`. Bun tooling compiles through MoonBit's C
backend and links a private runtime into the shared library. Both Go and MoonBit
apps can consume it using the existing service ABI. The provider alone requires
MoonBit and a C compiler; Go is only needed for the included Go wrapper/demo.
See the [authoring guide](../website/src/content/docs/moonbit/en/extensions.mdx)
for ownership, asynchronous native work, and packaging. Native provider checks
run as part of `scripts/check-moonbit.ts`, including two libraries loaded together.

From the repository root, validate the frontend and providers:

```sh
bun scripts/setup-moonbit.ts
export MOON_HOME="$PWD/target/moonbit-toolchain"
export PATH="$MOON_HOME/bin:$PATH"
bun scripts/check-moonbit.ts
# Also verify the actual staged native library and a self-closing hidden window:
bun scripts/check-moonbit.ts --native
```

Regenerate wire constants and fluent accessors with `bun scripts/generate-moonbit.ts`. Native symbols must match `crates/quickgui-host/src/capi.rs`. The test harness covers mutation equality, owner cleanup, conditional dependencies, keyed moves, and callback context. Native smoke tests establish ABI/lifecycle behavior, not visual acceptance.
