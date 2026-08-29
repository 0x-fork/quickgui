# System preferences and permissions

[Documentation index](README.md)

QuickGUI exposes renderer-independent appearance, accessibility, and privacy services from the
Rust `quickgui-system` core. `SystemPreferences::snapshot()` performs an explicit point-in-time
query; unsupported individual settings remain `None` rather than being guessed:

```rust
use quickgui::{ColorScheme, SystemPreferences};

let preferences = SystemPreferences::snapshot()?;
if preferences.color_scheme() == ColorScheme::Dark {
    install_dark_palette();
}
if preferences.reduce_motion() == Some(true) {
    disable_decorative_motion();
}
```

The bounded snapshot includes the preferred color scheme, Reduce Motion, Reduce Transparency,
Increase Contrast, Differentiate Without Color, inverted colors, screen-reader state, and switch
control. It also retains available `SystemColorRole` values for accent, highlight and highlight
text, window background/text, control background/text, and link colors;
`Color::from(SystemColor)` converts a native sRGB result for rendering. macOS publishes its native
appearance/accessibility fields and every system-color role.
Windows publishes its application theme, animation, transparency, high-contrast, screen-reader,
accent, window, label, and highlight values. Linux reads standardized XDG Settings portal values
where available and otherwise leaves fields unknown; `GTK_THEME` is only a bounded dark-theme
fallback.

## Runtime observation

`AppRunner::system_preferences()` and `EventContext::system_preferences()` return the runtime's
latest immutable value. A view calls `ViewContext::system_preferences()` to subscribe its current
declaration. Only subscribing windows rebuild when that value changes:

```rust
use quickgui::{Color, IntoElement, ViewContext, div};

fn render(&mut self, cx: &mut ViewContext<'_, Self>) -> impl IntoElement {
    let preferences = cx.system_preferences();
    div().when(
        preferences.increase_contrast() == Some(true),
        |element| element.border(2.0, Color::BLACK),
    )
}
```

The runtime captures its initial value before creating a window. Native light/dark window-theme
events update the stored color scheme, and macOS accessibility-display notifications replace the
whole snapshot. QuickGUI also combines system Reduce Motion with each window's explicit
`WindowOptions::reduce_motion`; either source disables declarative transitions, animations, and
springs. Explicit per-window `preferred_appearance` still controls that window's effective native
appearance independently of the system snapshot.

`TestAppContext::simulate_system_preferences_change` provides the same selective invalidation in
headless tests. Builder methods such as `with_reduce_motion` and `with_screen_reader` construct
fully deterministic snapshots without reading the developer machine.

## Privacy permissions

Permission checks never prompt:

```rust
use quickgui::{PermissionKind, PermissionManager, PermissionStatus};

if PermissionManager::status(PermissionKind::Camera)? == PermissionStatus::NotDetermined {
    PermissionManager::request(PermissionKind::Camera, |result| {
        handle_camera_authorization(result);
    })?;
}
```

`PermissionKind` covers camera, microphone, screen recording, and accessibility. An explicit
`request` invokes its completion exactly once when the platform supports that request. The
completion may run inline or on an operating-system callback thread, so it must hand application
state back to the appropriate executor.

macOS uses AVFoundation for camera and microphone, CoreGraphics for screen recording, and the
Accessibility trust API. Camera and microphone requests are rejected before the native prompt if
the application bundle lacks `NSCameraUsageDescription` or `NSMicrophoneUsageDescription`.
Windows reports screen recording and accessibility as available process capabilities while camera
and microphone privacy state remains `Unknown`; Linux currently reports all four as `Unknown`.
Explicit permission requests on Windows and Linux return `SystemIntegrationError::Unsupported`
instead of pretending that a prompt occurred.
