# Desktop integrations

[Documentation index](README.md) · [Windows and shared state](windows.md) · [macOS integration](macos.md)

QuickGUI exposes desktop-shell capabilities from the Rust core. `DesktopIntegrationSupport::current()`
reports which native backends were compiled for the target; callers do not need target-specific
binding logic:

```rust
use quickgui::{DesktopIntegrationSupport, TaskbarProgressState};

let support = DesktopIntegrationSupport::current();
if support.taskbar_progress {
    cx.set_taskbar_progress(TaskbarProgressState::Normal, 0.42)?;
}
if support.dock_badges {
    cx.set_dock_badge("3")?;
}
```

A `true` flag means a native implementation exists. Packaging, desktop services, user policy, or a
missing runtime permission can still reject an individual operation. The request and response types
remain renderer-independent and use the same bounded application effect queue as prompts and file
panels.

| Capability | macOS | Windows | Linux/BSD |
| --- | :---: | :---: | :---: |
| Taskbar progress and overlay icon | — | Yes | — |
| Dock badge, icon, and context menu | Yes | — | — |
| Recent documents | Yes | Yes | — |
| Native file icon | Yes | Yes | — |
| Standard About panel | Yes | Yes | — |
| Jump List user tasks | — | Yes | — |
| Native application and popup menus | Yes | Yes | — |
| System notifications | Yes | Yes | Portal / `notify-send` |
| Scheduled notifications | Yes | Yes | — |
| Notification inline replies | Yes | Yes | Portal-dependent / — |
| Dynamic protocol registration | Bundle metadata | Yes | Yes |

The complete machine-readable matrix also covers tray icons, global shortcuts, single-instance
ownership, autostart, window icons/focusability/opacity, workspace visibility, and cursor control.

## Window and taskbar state

Taskbar progress, taskbar overlays, native window icons, opacity, cursor visibility/confinement/
position/hit testing, focusability, taskbar omission, and workspace visibility are retained window
properties. They have startup builders on `WindowOptions`, current-window and handle-targeted
`EventContext` commands, externally pumped `AppRunner` commands, and observable `WindowState`
fields. Unsupported window-manager hints remain in the core snapshot so another runtime or binding
never becomes a competing source of truth.

Windows applies progress and overlays through `ITaskbarList3`. Because Explorer can create a
taskbar button after the `HWND` becomes visible, the runtime retries the complete retained taskbar
state on the first bounded redraws rather than losing an early request. macOS Dock badge/icon/menu
changes are application-wide. `AppRunner::dock_badge()` and `dock_menu()` report only successfully
processed native replacements.

## Recent documents, About, file icons, and user tasks

`EventContext` and `AppRunner` expose the same application services:

```rust
use quickgui::{AboutPanelOptions, FileIconSize, UserTask};

cx.add_recent_document("/absolute/path/report.quickgui")?;
cx.show_about_panel(
    AboutPanelOptions::new()
        .copyright("Copyright 2026 Example")
        .credits("Built with QuickGUI"),
)?;

let icon_response = cx.file_icon("/absolute/path/report.quickgui", FileIconSize::Normal)?;
cx.set_user_tasks([
    UserTask::new("New workspace", "--new-workspace")
        .description("Open an empty workspace"),
])?;
```

`file_icon` and Windows `set_user_tasks` return single-use `PlatformResponse` futures because the
native result can fail after queueing. The About panel fills missing application name/version fields
from `AppInfo`. macOS uses `NSDocumentController`, `NSWorkspace`, and the standard AppKit About
panel; Windows uses Shell recent-document APIs, Shell file icons/About UI, and a transactional
`ICustomDestinationList` Jump List update.

## Native menus

`MenuItem::role` declares standard About, Quit, Hide, window, help, and editing commands without an
application action type. `MenuItem::os_action` combines a typed fallback with an operating-system
responder command. Check and radio marks are controlled by the next declaration, and ordinary items
can carry a bounded `MenuIcon`.

Application menus can be replaced, queried, or cleared; a `WindowOptions::window_menus` override is
scoped to one window. `show_native_popup_menu` projects the same model at a screen point. macOS Dock
menus reuse the same typed action registry, while Windows application and popup menus use the native
menu host. No clean menu installs a polling source or animation frame.

## Notifications and deep links

`SystemNotification` supports subtitles, action buttons, inline replies, default/silent/named sound,
an icon, bounded attachments, and an optional delivery time. Permission status and explicit request
operations are separate futures, and `on_system_notification_response` receives the stable tag,
action identifier, and optional bounded reply text. Windows uses the identifier from `AppInfo` as its
application user model ID; notification operations fail when that identity is absent, and packaged or
unpackaged applications must provide the corresponding shell registration. macOS and Windows
implement the complete model; the Linux XDG portal implements immediate notifications, icons,
buttons, and portal-dependent reply text, and rejects scheduling, attachments, and named sounds
instead of silently dropping them.

`EventContext` and `AppRunner` both expose portable URL, open-path, reveal-path, and trash-path
operations. Event callbacks enqueue the operation without blocking, while the externally pumped
runner returns a `ShellResponse` that reports completion or a platform error.

`on_open_urls` is the unified deep-link/file-open callback. macOS receives native application-open
events. Other desktop targets parse the bounded initial process arguments, and a primary
single-instance process also maps later launches into the same callback before delivering
`on_second_instance`. Dynamic protocol registration remains an explicit `ProtocolRegistration`
service: Windows and Linux can register at runtime, while macOS requires bundle metadata.

Native visual/runtime acceptance is platform-specific. Cross-compilation proves type and backend
integration, but does not by itself prove Explorer, AppKit, a Linux desktop portal, or application
packaging behavior on an end-user machine.
\n
## Application shell services

`EventContext` exposes the application-level shell operations Electron applications expect. They
follow the ordinary platform-request path: fire-and-forget mutations return `Result<(),
PlatformError>` immediately, while operations with a native answer return a `PlatformResponse`
future awaited from a foreground task. Every request shares the existing
`MAX_PLATFORM_REQUESTS_PER_EVENT` / `MAX_PENDING_PLATFORM_REQUESTS` bounds.

| Operation | macOS | Windows | Linux |
| --- | --- | --- | --- |
| `set_activation_policy(Regular\|Accessory\|Prohibited)` | `NSApplicationActivationPolicy` | `Unsupported` | `Unsupported` |
| `activate_application(force)` | `NSApp.activate` / `activateIgnoringOtherApps:` | logged, no-op | logged, no-op |
| `hide_application()` / `unhide_application()` | `NSApplication hide:` / `unhide:` | logged, no-op | logged, no-op |
| `request_dock_attention(Critical\|Informational)` | `requestUserAttention:` | `Unsupported` | `Unsupported` |
| `cancel_dock_attention(request)` | `cancelUserAttentionRequest:` | no-op | no-op |
| `set_dock_visible(bool)` | Regular / Accessory policy | `Unsupported` | `Unsupported` |
| `set_secure_keyboard_entry(bool)` | `EnableSecureEventInput` / `DisableSecureEventInput` | logged, no-op | logged, no-op |
| `beep()` | `NSBeep` | logged, no-op | logged, no-op |
| `applications_folder_support()` | bundle + install query | `{ supported: false }` | `{ supported: false }` |
| `move_to_applications_folder()` | moves the `.app` into `/Applications` | `Unsupported` | `Unsupported` |
| `exit_with_code(i32)` | ordinary teardown, then the code | same | same |
| `is_application_packaged()` | bundle heuristic | build-directory heuristic | build-directory heuristic |

`activate_application(false)` uses AppKit's ordinary activation; `true` uses
`activateIgnoringOtherApps:`, which takes focus away from the frontmost application. Prefer `false`
unless the user just asked for the application explicitly.

`DockAttention::Critical` bounces until the application is activated or the returned
`DockAttentionRequest` is cancelled; `Informational` bounces once. The request identifier comes back
through the `PlatformResponse`, so cancelling requires awaiting it first.

`set_secure_keyboard_entry` reads `IsSecureEventInputEnabled` before enabling or disabling, so
repeated calls cannot unbalance the system-wide counter. Secure event input is process-global on
macOS: leaving it enabled affects every application, so disable it as soon as the sensitive field
loses focus.

`move_to_applications_folder` returns `false` when the bundle is already installed under an
`Applications` directory, and fails with a platform error when a same-named application is already
there or the move is not permitted. QuickGUI never restarts the process on its own; call
`cx.relaunch()` after a successful move. `applications_folder_support()` is a pure query and is
safe to call during rendering.

`exit_with_code(code)` runs the identical structured teardown as `exit()` — windows close
child-first, `on_before_quit`/`on_will_quit`/`on_window_closed` still run, and single-instance and
process integrations are released. Only after the event loop has fully unwound is a non-zero code
applied to the process. `TestAppContext::exit_code()` exposes it deterministically.

`is_application_packaged()` is a documented heuristic, not a security boundary:

- **macOS** — the executable path contains a `<name>.app/Contents/MacOS/` component **and** is not
  under a Cargo build directory. `cargo run` therefore reports `false`.
- **Windows and Linux/BSD** — the executable is not under a Cargo build directory
  (`target/debug`, `target/release`, or `target/<triple>/<profile>`).

`TestAppContext::application_shell()` returns a `TestApplicationShell` snapshot of every shell
service a deterministic test requested: activation policy, hidden state, activation count and
whether the last activation was forced, Dock visibility, the in-flight Dock attention request,
secure keyboard entry, alert-sound count, and accepted Applications-folder moves.
