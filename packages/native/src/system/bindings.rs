use super::*;

#[napi]
pub fn exit_app(app: u32) -> Result<bool> {
    expect_boolean(direct_command(app, SystemCommand::Exit)?)
}

#[napi]
pub fn exit_hosted_app(app: u32) -> Result<bool> {
    expect_boolean(hosted_command(app, SystemCommand::Exit)?)
}

#[napi]
pub fn get_displays(app: u32) -> Result<Vec<NativeDisplay>> {
    expect_displays(direct_command(app, SystemCommand::GetDisplays)?)
}

#[napi]
pub fn get_hosted_displays(app: u32) -> Result<Vec<NativeDisplay>> {
    expect_displays(hosted_command(app, SystemCommand::GetDisplays)?)
}

#[napi]
pub fn get_keyboard_layout(app: u32) -> Result<NativeKeyboardLayout> {
    expect_keyboard_layout(direct_command(app, SystemCommand::GetKeyboardLayout)?)
}

#[napi]
pub fn get_hosted_keyboard_layout(app: u32) -> Result<NativeKeyboardLayout> {
    expect_keyboard_layout(hosted_command(app, SystemCommand::GetKeyboardLayout)?)
}

#[napi]
pub fn get_window_state(app: u32, window: u32) -> Result<NativeWindowState> {
    expect_window_state(direct_command(app, SystemCommand::GetWindowState(window))?)
}

#[napi]
pub fn get_hosted_window_state(app: u32, window: u32) -> Result<NativeWindowState> {
    expect_window_state(hosted_command(app, SystemCommand::GetWindowState(window))?)
}

#[napi]
pub fn read_clipboard(app: u32) -> Result<Option<NativeClipboardItem>> {
    expect_clipboard(direct_command(app, SystemCommand::ReadClipboard)?)
}

#[napi]
pub fn read_hosted_clipboard(app: u32) -> Result<Option<NativeClipboardItem>> {
    expect_clipboard(hosted_command(app, SystemCommand::ReadClipboard)?)
}

#[napi]
pub fn write_clipboard(app: u32, item: NativeClipboardItem) -> Result<()> {
    let item = clipboard_item(item).map_err(Error::from_reason)?;
    expect_unit(direct_command(app, SystemCommand::WriteClipboard(item))?)
}

#[napi]
pub fn write_hosted_clipboard(app: u32, item: NativeClipboardItem) -> Result<()> {
    let item = clipboard_item(item).map_err(Error::from_reason)?;
    expect_unit(hosted_command(app, SystemCommand::WriteClipboard(item))?)
}

#[napi]
pub fn show_notification(app: u32, options: NativeNotificationOptions) -> Result<()> {
    expect_unit(direct_command(
        app,
        SystemCommand::ShowNotification(system_notification(options)),
    )?)
}

#[napi]
pub fn show_hosted_notification(app: u32, options: NativeNotificationOptions) -> Result<()> {
    expect_unit(hosted_command(
        app,
        SystemCommand::ShowNotification(system_notification(options)),
    )?)
}

#[napi]
pub fn dismiss_notification(app: u32, tag: String) -> Result<()> {
    expect_unit(direct_command(
        app,
        SystemCommand::DismissNotification(tag),
    )?)
}

#[napi]
pub fn dismiss_hosted_notification(app: u32, tag: String) -> Result<()> {
    expect_unit(hosted_command(
        app,
        SystemCommand::DismissNotification(tag),
    )?)
}

#[napi]
pub fn set_application_menu(app: u32, menu: String) -> Result<()> {
    expect_unit(direct_command(
        app,
        SystemCommand::SetApplicationMenu(menu),
    )?)
}

#[napi]
pub fn set_hosted_application_menu(app: u32, menu: String) -> Result<()> {
    expect_unit(hosted_command(
        app,
        SystemCommand::SetApplicationMenu(menu),
    )?)
}

#[napi]
pub fn request_single_instance_lock(app: u32, identifier: String) -> Result<bool> {
    expect_boolean(direct_command(
        app,
        SystemCommand::RequestSingleInstanceLock(identifier),
    )?)
}

#[napi]
pub fn request_hosted_single_instance_lock(app: u32, identifier: String) -> Result<bool> {
    expect_boolean(hosted_command(
        app,
        SystemCommand::RequestSingleInstanceLock(identifier),
    )?)
}

#[napi]
pub fn release_single_instance_lock(app: u32) -> Result<bool> {
    expect_boolean(direct_command(
        app,
        SystemCommand::ReleaseSingleInstanceLock,
    )?)
}

#[napi]
pub fn release_hosted_single_instance_lock(app: u32) -> Result<bool> {
    expect_boolean(hosted_command(
        app,
        SystemCommand::ReleaseSingleInstanceLock,
    )?)
}

#[napi]
pub fn perform_global_shortcut_action(
    app: u32,
    request: u32,
    action: String,
    registration: Option<u32>,
    accelerator: Option<String>,
) -> Result<()> {
    let action = parse_global_shortcut_action(&action, registration, accelerator)
        .map_err(Error::from_reason)?;
    expect_unit(direct_command(
        app,
        SystemCommand::GlobalShortcut { request, action },
    )?)
}

#[napi]
pub fn perform_hosted_global_shortcut_action(
    app: u32,
    request: u32,
    action: String,
    registration: Option<u32>,
    accelerator: Option<String>,
) -> Result<()> {
    let action = parse_global_shortcut_action(&action, registration, accelerator)
        .map_err(Error::from_reason)?;
    expect_unit(hosted_command(
        app,
        SystemCommand::GlobalShortcut { request, action },
    )?)
}

#[napi]
pub fn set_tray_icon(app: u32, request: u32, options: NativeTrayIconOptions) -> Result<()> {
    let options = tray::tray_options(options).map_err(Error::from_reason)?;
    expect_unit(direct_command(
        app,
        SystemCommand::Tray {
            request,
            action: tray::TrayAction::Set(options),
        },
    )?)
}

#[napi]
pub fn set_hosted_tray_icon(app: u32, request: u32, options: NativeTrayIconOptions) -> Result<()> {
    let options = tray::tray_options(options).map_err(Error::from_reason)?;
    expect_unit(hosted_command(
        app,
        SystemCommand::Tray {
            request,
            action: tray::TrayAction::Set(options),
        },
    )?)
}

#[napi]
pub fn remove_tray_icon(app: u32, request: u32, id: u32) -> Result<()> {
    expect_unit(direct_command(
        app,
        SystemCommand::Tray {
            request,
            action: tray::TrayAction::Remove(id),
        },
    )?)
}

#[napi]
pub fn remove_hosted_tray_icon(app: u32, request: u32, id: u32) -> Result<()> {
    expect_unit(hosted_command(
        app,
        SystemCommand::Tray {
            request,
            action: tray::TrayAction::Remove(id),
        },
    )?)
}

#[napi]
pub fn show_tray_menu(app: u32, request: u32, id: u32) -> Result<()> {
    expect_unit(direct_command(
        app,
        SystemCommand::Tray {
            request,
            action: tray::TrayAction::ShowMenu(id),
        },
    )?)
}

#[napi]
pub fn show_hosted_tray_menu(app: u32, request: u32, id: u32) -> Result<()> {
    expect_unit(hosted_command(
        app,
        SystemCommand::Tray {
            request,
            action: tray::TrayAction::ShowMenu(id),
        },
    )?)
}

#[napi]
pub fn perform_window_action(
    app: u32,
    window: u32,
    action: String,
    value: Option<String>,
) -> Result<()> {
    let action = parse_window_action(&action, value).map_err(Error::from_reason)?;
    expect_unit(direct_command(
        app,
        SystemCommand::WindowAction { window, action },
    )?)
}

#[napi]
pub fn perform_hosted_window_action(
    app: u32,
    window: u32,
    action: String,
    value: Option<String>,
) -> Result<()> {
    let action = parse_window_action(&action, value).map_err(Error::from_reason)?;
    expect_unit(hosted_command(
        app,
        SystemCommand::WindowAction { window, action },
    )?)
}

#[napi]
pub fn perform_shell_action(app: u32, request: u32, action: String, value: String) -> Result<()> {
    let action = parse_shell_action(&action, value).map_err(Error::from_reason)?;
    expect_unit(direct_command(
        app,
        SystemCommand::ShellAction { request, action },
    )?)
}

#[napi]
pub fn perform_hosted_shell_action(
    app: u32,
    request: u32,
    action: String,
    value: String,
) -> Result<()> {
    let action = parse_shell_action(&action, value).map_err(Error::from_reason)?;
    expect_unit(hosted_command(
        app,
        SystemCommand::ShellAction { request, action },
    )?)
}
