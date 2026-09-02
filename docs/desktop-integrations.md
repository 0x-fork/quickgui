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

### Accelerators

A menu item's key equivalent normally comes from the keymap binding that dispatches its action.
`MenuItem::accelerator` and `MenuItem::keystroke` declare an explicit override that outranks the
derived binding, and `MenuItem::try_accelerator` reports an unparsable string instead of dropping it.

```rust
use quickgui::{Accelerator, Menu, MenuItem, OsAction};

let file = Menu::new("File")
    .item(MenuItem::action("Save As…", SaveAs).accelerator("CmdOrCtrl+Shift+S"))
    .item(MenuItem::role("Close Window", OsAction::CloseWindow).keystroke(
        Accelerator::parse("CommandOrControl+W").expect("a valid accelerator"),
    ))
    .item(MenuItem::action("Reload Fixtures", ReloadFixtures).hidden(true));
```

`Accelerator::parse` accepts Electron's accelerator grammar and returns QuickGUI's `Keystroke`:

- Modifiers: `CommandOrControl`/`CmdOrCtrl` (Command on macOS, Control elsewhere), `Command`/`Cmd`,
  `Control`/`Ctrl`, `Alt`/`Option`, `AltGr`, `Shift`, `Super`/`Meta`.
- Keys: `A`–`Z`, `0`–`9`, `F1`–`F24`, `Plus`, `Space`, `Tab`, `Capslock`, `Numlock`, `Scrolllock`,
  `Backspace`, `Delete`, `Insert`, `Return`/`Enter`, `Up`/`Down`/`Left`/`Right`, `Home`/`End`,
  `PageUp`/`PageDown`, `Escape`/`Esc`, `VolumeUp`/`VolumeDown`/`VolumeMute`, `MediaNextTrack`,
  `MediaPreviousTrack`, `MediaStop`, `MediaPlayPause`, `PrintScreen`, `num0`–`num9`, `numdec`,
  `numadd`, `numsub`, `nummult`, `numdiv`, and any single punctuation character.

Accelerators are bounded by `MAX_ACCELERATOR_BYTES`. Keypad names resolve to the character the
platform prints for that key, matching how Electron renders keypad menu accelerators. The lock,
media, volume, and print-screen keys have no `Key` identity in QuickGUI and parse to `Key::Other`:
the declaration stays valid, and platforms that cannot render them simply show no key equivalent.

On macOS the resolved keystroke becomes the item's `NSMenuItem` key equivalent and modifier mask,
replacing both the keymap-derived binding and any AppKit standard binding a role would otherwise
use. On Windows `muda` renders the accelerator after a tab in the item label and registers the
matching Win32 accelerator-table entry, so the printed shortcut and the dispatched command always
agree; keys a Win32 accelerator table cannot name keep the label and lose only the rendered
shortcut.

`MenuItem::hidden` keeps a declared item out of the presented menu without removing it from the
declaration, so its action id and collection order stay stable across menu revisions. macOS sets
`NSMenuItem.hidden`; Windows has no hidden flag, so the item is omitted from the Win32 menu.

### Roles and system submenus

Beyond the editing, application, window, and help commands, `OsAction` also covers
`PasteAndMatchStyle`, `Delete`, `StartSpeaking`, `StopSpeaking`, `SelectNextTab`,
`SelectPreviousTab`, `MergeAllWindows`, `MoveTabToNewWindow`, `ToggleTabBar`, and
`ToggleTabOverview`. Each first follows the platform responder chain; when no native responder
accepts it, QuickGUI falls back to the retained implementation — plain-text paste and selection
delete for the focused input, and the matching window-tab command for the tab roles. Speech
synthesis has no retained fallback and stays an AppKit responder command.

`SystemMenuType::RecentDocuments` declares an operating-system-populated recent-documents submenu.
QuickGUI builds a menu holding one "Clear Menu" item whose action is `clearRecentDocuments:`, which
is how `NSDocumentController` recognizes the menu and fills it from the recent-documents list that
`add_recent_document` maintains.

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
