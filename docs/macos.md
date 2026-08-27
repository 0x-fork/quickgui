# macOS integration

[Documentation index](README.md)

## macOS platform services

Event callbacks can start window-owned AppKit prompts and file panels, then await the result on the
existing main-thread foreground executor:

```rust
let response = cx.prompt(
    PromptLevel::Warning,
    "Save changes?",
    Some("Unsaved edits will be lost."),
    &[
        PromptButton::ok("Save"),
        PromptButton::new("Don't Save"),
        PromptButton::cancel("Cancel"),
    ],
).expect("valid native prompt");

cx.spawn(|async_cx: AsyncViewContext<MyView>| async move {
    let answer = response.await.ok();
    async_cx.update(move |view, cx| {
        view.answer = answer;
        cx.invalidate();
    }).await
}).expect("foreground capacity").detach();
```

System notifications are application-wide and use stable tags for replacement and dismissal:

```rust
let notification = SystemNotification::new(
    "background-build",
    "Build finished",
    "All checks passed.",
)
.action(SystemNotificationAction::new("open", "Open"));

cx.show_system_notification(notification)?;
// cx.dismiss_system_notification("background-build")?;
```

Native application events are configured once on `App`, outside any particular window:

```rust
App::new(view)
    .on_open_urls(|urls, cx| update_workspace(urls, cx))
    .on_reopen(|had_visible_windows, cx| reopen(had_visible_windows, cx))
    .on_system_wake(|cx| reconnect(cx))
    .on_system_notification_response(|response, cx| activate(response, cx))
    .on_window_closed(|window, cx| record_closed_window(window, cx))
    .run()?;
```

Those callbacks receive an application-wide `EventContext` with no current window. They can
update globals or entities and open new root windows; window-owned prompts and foreground tasks
still require a window callback. Native observers are installed only for callbacks the
application registered.

The default `QuitMode` keeps a macOS application resident after its final window closes, matching
GPUI and standard Dock behavior. An `on_reopen` callback can create a new root window from its
windowless application context. Choose `QuitMode::LastWindowClosed` for utilities and examples
that should terminate as soon as their final window closes.

`prompt_for_paths(PathPromptOptions)` and `prompt_for_new_path(SavePathOptions)` return the same
single-use future shape; cancellation is `Ok(None)`. `open_url`, `open_path`, and `reveal_path`
project through `NSWorkspace`. A dropped response cancels a sheet that has already started, window
teardown cancels both the response task and native panel, related parent/child windows cannot stack
conflicting sheets, and unrelated windows remain independent. Requests, active dialogs, selected
path counts, and returned path bytes all have public hard limits. Unbound Cmd-W now goes through
AppKit's ordinary close path and still delivers `Event::CloseRequested`.

On macOS, notifications require a real `.app` bundle with a `CFBundleIdentifier`; an unbundled
`cargo run` process cannot use `UNUserNotificationCenter`. The first post makes one contextual
authorization request, waits for its result, and coalesces replacement tags in a 64-entry bounded
queue. Dismissal also removes a matching authorization-pending post. Action category retention is
capped at 64 distinct action sets; later notifications still post without actions instead of
growing native state. Open callbacks retain at most 256 URLs and 1 MiB total. None of these
services installs a polling timer or idle frame. See
`cargo run --release --example platform_services`.

## Clipboard and Find pasteboard

Every event callback can synchronously read or replace the general pasteboard with a bounded
`ClipboardItem`. The direct AppKit backend supports UTF-8 text with hash-bound application metadata,
encoded image formats without eager decoding, and native filename property lists with a text
fallback. Native byte lengths are rejected before Rust copies them.

`read_from_find_pasteboard` and `write_to_find_pasteboard` use macOS's shared search pasteboard and
the same item model. Both pasteboard handles are lazy, and QuickGUI installs no change-count observer
or polling timer. See [Clipboard](clipboard.md) and run:

```console
cargo run --release --example clipboard
```

## Native appearance

macOS windows follow AppKit's effective Aqua or Dark Aqua appearance by default. QuickGUI maps
those platform names to `WindowAppearance::Light` and `WindowAppearance::Dark`, applies an explicit
preference through the native `NSWindow` when requested, and routes AppKit effective-appearance
changes through Winit's existing event source. Application content opts into palette updates with
`cx.appearance()`; native titlebar controls update as part of the same window appearance.

There is no distributed-notification observer, preference query timer, or animation loop. A system
change invalidates only views that observed native window state, while `Event::AppearanceChanged`
remains available for stateful application reactions. Run the example with:

```console
cargo run --release --example appearance
```

## Window backgrounds

`WindowBackgroundAppearance` controls composition independently from the window's light/dark
palette. `Opaque` uses Metal's opaque fast path. `Transparent` keeps scene alpha through the
`CAMetalLayer` and clears the `NSWindow` background. `Blurred` uses that same alpha-capable path
and enables the native window-server blur behind it; QuickGUI does not blur application pixels or
render a fullscreen blur pass.

Runtime switches retain the existing WGPU surface, pipelines, caches, and drawable chain. QuickGUI
changes `CAMetalLayer.opaque` directly on macOS and invokes AppKit opacity or blur operations only
when that exact component changes. This avoids both surface allocation and redundant native
transparency calls that can strand a previously presented Metal drawable. The transition requests
one damage-driven frame; a stationary transparent or blurred window then returns to
`ControlFlow::Wait`.

Use translucent scene colors to expose the effect and run the native sample with:

```console
cargo run --release --example window_background
```

## macOS window chrome

Represented file URLs, native edited-state indication, the character palette, and AppKit system
tabs are documented separately in [Native document windows](document-windows.md). That layer
reuses the same retained window command path, reapplies custom traffic-light placement after
AppKit titlebar mutations, and adds no polling or idle redraw source.

`HiddenInset` extends GPU content through a transparent titlebar while retaining native traffic
lights. Its implicit AppKit drag area is disabled; layout boxes explicitly opt into web-style
`drag` and `no-drag` regions:

```rust
App::new(view)
    .title_bar_style(TitleBarStyle::HiddenInset)
    .traffic_light_position(16.0, 13.0)
    .run()?;

div()
    .h(64.0)
    .app_region_drag()
    .child(button().app_region_no_drag().child("Refresh"))
```

The topmost declared app region wins, so interactive descendants use `.app_region_no_drag()`.
Drag regions do not receive element clicks or captured pointer gestures. Built-in overlay
scrollbars take input precedence over an ancestor drag region.

On macOS, any retained AppKit `NSView` subclass can participate in the same layout:

```rust
let field: Retained<NSTextField> = /* construct on the main thread */;

native_view(field.as_ref())
    .id("native-field")
    .w_full()
    .h(44.0)
    .rounded_lg()
```

QuickGUI switches that window to three-plane composition: WGPU base content, native child views,
then a transparent WGPU overlay. The second full-window swapchain is created only when the first
overlay opens, then retained for low-latency reuse. Native views are retained by Objective-C
identity, clipped and resized in logical points, reordered by `z_index`, and removed when their
elements disappear. AppKit and QuickGUI first-responder focus are synchronized without an idle
polling loop. A background-colored launch shield stays above every plane until WGPU successfully
presents and completes the first frame, so an AppKit child cannot flash over an otherwise empty
window. Layout, text shaping, native reconciliation, and buffer uploads run while the window is
still hidden. QuickGUI briefly detaches the retained content view from that hidden `NSWindow`, which
allows WGPU to present the actual Metal layer without ordering any window onscreen. It reattaches
the same view at the unchanged frame only after GPU completion, preventing both black intermediate
frames and cross-monitor movement in unoptimized builds.
See `cargo run --release --example native_view` and
`cargo run --release --example overlays`.
