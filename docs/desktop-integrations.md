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
| Message-box checkbox and custom icon | Yes | — | — |
| File preview panel (Quick Look) | Yes | — | — |
| System color panel | Yes | — | — |
| System font panel | Yes | — | — |
| Share sheet | Yes | — | — |
| Biometric authentication | Yes | — | — |

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

## Message boxes and file panels

`EventContext::prompt` remains the smallest native alert. `EventContext::message_box` is its richer
sibling: it accepts a level, a `message`/`detail` pair, explicit default and cancel button indices,
a suppression checkbox, and a custom icon, and its response carries both the chosen button index and
the final checkbox state.

```rust
use quickgui::{MessageBoxOptions, PromptButton, PromptLevel};

let answer = cx.message_box(
    MessageBoxOptions::new("Discard the unsaved draft?")
        .level(PromptLevel::Warning)
        .detail("This cannot be undone.")
        .buttons([
            PromptButton::new("Discard"),
            PromptButton::new("Keep editing"),
            PromptButton::cancel("Cancel"),
        ])
        .default_button(1)
        .cancel_button(2)
        .checkbox("Do not ask again", false),
)?;
// answer.await -> MessageBoxResponse { button, checkbox_checked }
```

Declaring no buttons yields one implicit `OK`. Every field is validated before the request is
retained: the message must be non-empty, text is NUL-free and within `MAX_PLATFORM_TEXT_BYTES`, the
checkbox label is within `MAX_MESSAGE_BOX_CHECKBOX_BYTES`, and the default/cancel indices must
address a declared button. When the context owns a window the box is presented as that window's
sheet; otherwise it is application-modal.

`PathPromptOptions` and `SavePathOptions` gained the remaining panel controls:

| Option | macOS | Windows | Linux/BSD |
| --- | :---: | :---: | :---: |
| `MessageBoxOptions::checkbox` | Yes | `Unsupported` | `Unsupported` |
| `MessageBoxOptions::icon` | Yes | `Unsupported` | `Unsupported` |
| `MessageBoxOptions::detail` | Yes | Title + description | Title + description |
| `MessageBoxOptions::default_button` / `cancel_button` | Yes | Button-set order | Button-set order |
| `PathPromptOptions::can_create_directories` | Yes | — | — |
| `PathPromptOptions::resolves_aliases` | Yes | — | — |
| `PathPromptOptions::treats_file_packages_as_directories` | Yes | — | — |
| `PathPromptOptions::message` | Yes | — | — |
| `SavePathOptions::name_field_label` | Yes | — | — |
| `SavePathOptions::shows_tag_field` | Yes | — | — |

A `Yes` maps to the AppKit panel property of the same name. A dash means the portable `rfd` backend
exposes no equivalent and silently keeps its own default; those options never change behavior there.
`Unsupported` means the request is rejected with `PlatformError::Unsupported` rather than dropping a
declared affordance: the portable message box refuses a checkbox or a custom icon instead of showing
a box that quietly lacks them. `PathPromptOptions::message` is the panel's explanatory header;
`title` keeps mapping to the same slot when no message is supplied.

## File previews, panels, and services

```rust
use quickgui::{ColorPanelMode, Font, Rect, ShareItem};

cx.preview_file("/absolute/path/report.pdf", Some("Quarterly report"))?;
cx.close_file_preview()?;

cx.show_color_panel(Color::rgb8(94, 234, 212), ColorPanelMode::Continuous)?;
cx.close_color_panel()?;
cx.show_font_panel(Font::default())?;

cx.share_items(
    &[ShareItem::Text("Shared from QuickGUI".into()), ShareItem::File(path)],
    Rect::new(30.0, 240.0, 1.0, 1.0),
)?;

let granted = cx.authenticate_with_biometrics("Unlock the vault")?;
```

macOS implements all five with `QLPreviewPanel`, `NSColorPanel`, `NSFontManager`,
`NSSharingServicePicker`, and `LAContext`. Every other target returns `PlatformError::Unsupported`
from the corresponding `DesktopIntegrationSupport` flag check before anything is queued.

Bounds: a preview display name is at most `MAX_FILE_PREVIEW_NAME_BYTES`; a share sheet accepts 1 to
`MAX_SHARE_ITEMS` items with at most `MAX_SHARE_ITEM_TEXT_BYTES` per text or URL item and an anchor
whose components are finite and non-negative; a biometric reason is at most
`MAX_BIOMETRIC_REASON_BYTES`.

The color and font panels are shared system resources, so their observations are application
callbacks rather than per-window listeners:

```rust
Application::new()
    .on_color_panel_change(|color, cx| cx.update_global::<Theme, _>(|theme| theme.accent = color))
    .on_font_panel_change(|font, cx| cx.update_global::<Theme, _>(|theme| theme.body = font))
    .run(|cx| { /* … */ })?;
```

`ColorPanelMode::Continuous` reports every intermediate color through the panel's target/action;
`ColorPanelMode::OnClose` installs no action and reports the final color once from the panel's
close notification. Closing the panel removes both the target and the observation, so a dismissed
panel costs nothing. QuickGUI's `Font` carries no point size, so a font-panel change reports the
chosen family, weight, and slant.

`LAContext` replies on a background queue. QuickGUI forwards that reply through the event loop and
completes the `PlatformResponse<bool>` on the main thread, so no responder is ever touched off the
application thread. A machine without biometric hardware, or a policy that forbids the check,
reports `PlatformError::Unsupported` before the system prompt appears.

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
