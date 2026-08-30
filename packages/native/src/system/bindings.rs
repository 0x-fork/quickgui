use super::*;

#[napi]
pub fn get_power_state() -> Result<NativePowerState> {
    CorePowerMonitor::snapshot()
        .map(Into::into)
        .map_err(|error| Error::from_reason(error.to_string()))
}

#[napi]
pub fn get_system_idle_time() -> Result<f64> {
    CorePowerMonitor::system_idle_time()
        .map(|duration| duration.as_secs_f64())
        .map_err(|error| Error::from_reason(error.to_string()))
}

#[napi]
pub fn get_session_state() -> Result<String> {
    CorePowerMonitor::session_state()
        .map(|state| {
            match state {
                quickgui::SessionState::Active => "active",
                quickgui::SessionState::Inactive => "inactive",
                quickgui::SessionState::Locked => "locked",
                quickgui::SessionState::Unknown => "unknown",
            }
            .to_owned()
        })
        .map_err(|error| Error::from_reason(error.to_string()))
}

#[napi]
pub fn get_system_idle_state(threshold_seconds: f64) -> Result<String> {
    if !threshold_seconds.is_finite()
        || threshold_seconds <= 0.0
        || threshold_seconds > 366.0 * 24.0 * 60.0 * 60.0
    {
        return Err(Error::from_reason(
            "the idle threshold must be a positive finite number of seconds",
        ));
    }
    CorePowerMonitor::system_idle_state(std::time::Duration::from_secs_f64(threshold_seconds))
        .map(|state| {
            match state {
                quickgui::IdleState::Active => "active",
                quickgui::IdleState::Idle => "idle",
                quickgui::IdleState::Locked => "locked",
                quickgui::IdleState::Unknown => "unknown",
            }
            .to_owned()
        })
        .map_err(|error| Error::from_reason(error.to_string()))
}

#[napi]
pub fn get_permission_status(kind: String) -> Result<String> {
    PermissionManager::status(parse_permission_kind(&kind)?)
        .map(permission_status_name)
        .map_err(|error| Error::from_reason(error.to_string()))
}

#[napi(ts_return_type = "Promise<string>")]
pub fn request_permission(kind: String) -> Result<AsyncTask<PermissionRequestTask>> {
    Ok(AsyncTask::new(PermissionRequestTask {
        kind: parse_permission_kind(&kind)?,
    }))
}

#[napi]
pub fn configure_app(app: u32, options: NativeAppOptions) -> Result<()> {
    expect_unit(direct_command(app, SystemCommand::ConfigureApp(options))?)
}

#[napi]
pub fn configure_hosted_app(app: u32, options: NativeAppOptions) -> Result<()> {
    expect_unit(hosted_command(app, SystemCommand::ConfigureApp(options))?)
}

#[napi]
pub fn get_app_info(app: u32) -> Result<Option<NativeAppInfo>> {
    expect_app_info(direct_command(app, SystemCommand::GetAppInfo)?)
}

#[napi]
pub fn get_hosted_app_info(app: u32) -> Result<Option<NativeAppInfo>> {
    expect_app_info(hosted_command(app, SystemCommand::GetAppInfo)?)
}

#[napi]
pub fn get_app_paths(app: u32) -> Result<Option<NativeAppPaths>> {
    expect_app_paths(direct_command(app, SystemCommand::GetAppPaths)?)
}

#[napi]
pub fn get_hosted_app_paths(app: u32) -> Result<Option<NativeAppPaths>> {
    expect_app_paths(hosted_command(app, SystemCommand::GetAppPaths)?)
}

#[napi]
pub fn get_system_info(app: u32) -> Result<NativeSystemInfo> {
    expect_system_info(direct_command(app, SystemCommand::GetSystemInfo)?)
}

#[napi]
pub fn get_hosted_system_info(app: u32) -> Result<NativeSystemInfo> {
    expect_system_info(hosted_command(app, SystemCommand::GetSystemInfo)?)
}

#[napi]
pub fn get_window_registry(app: u32) -> Result<NativeWindowRegistry> {
    expect_window_registry(direct_command(app, SystemCommand::GetWindowRegistry)?)
}

#[napi]
pub fn get_hosted_window_registry(app: u32) -> Result<NativeWindowRegistry> {
    expect_window_registry(hosted_command(app, SystemCommand::GetWindowRegistry)?)
}

#[napi]
pub fn get_cursor_screen_position(app: u32) -> Result<NativePoint> {
    expect_point(direct_command(app, SystemCommand::GetCursorScreenPosition)?)
}

#[napi]
pub fn get_hosted_cursor_screen_position(app: u32) -> Result<NativePoint> {
    expect_point(hosted_command(app, SystemCommand::GetCursorScreenPosition)?)
}

#[napi]
pub fn get_desktop_integration_support(app: u32) -> Result<NativeDesktopIntegrationSupport> {
    expect_desktop_integration_support(direct_command(
        app,
        SystemCommand::GetDesktopIntegrationSupport,
    )?)
}

#[napi]
pub fn get_hosted_desktop_integration_support(app: u32) -> Result<NativeDesktopIntegrationSupport> {
    expect_desktop_integration_support(hosted_command(
        app,
        SystemCommand::GetDesktopIntegrationSupport,
    )?)
}

#[napi]
pub fn get_system_preferences(app: u32) -> Result<NativeSystemPreferences> {
    expect_system_preferences(direct_command(app, SystemCommand::GetSystemPreferences)?)
}

#[napi]
pub fn get_hosted_system_preferences(app: u32) -> Result<NativeSystemPreferences> {
    expect_system_preferences(hosted_command(app, SystemCommand::GetSystemPreferences)?)
}

#[napi]
pub fn exit_app(app: u32) -> Result<bool> {
    expect_boolean(direct_command(app, SystemCommand::Exit)?)
}

#[napi]
pub fn exit_hosted_app(app: u32) -> Result<bool> {
    expect_boolean(hosted_command(app, SystemCommand::Exit)?)
}

#[napi]
pub fn relaunch_app(app: u32, options: NativeRelaunchOptions) -> Result<bool> {
    expect_boolean(direct_command(app, SystemCommand::Relaunch(options))?)
}

#[napi]
pub fn relaunch_hosted_app(app: u32, options: NativeRelaunchOptions) -> Result<bool> {
    expect_boolean(hosted_command(app, SystemCommand::Relaunch(options))?)
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
    let notification = system_notification(options).map_err(Error::from_reason)?;
    expect_unit(direct_command(
        app,
        SystemCommand::ShowNotification(notification),
    )?)
}

#[napi]
pub fn show_hosted_notification(app: u32, options: NativeNotificationOptions) -> Result<()> {
    let notification = system_notification(options).map_err(Error::from_reason)?;
    expect_unit(hosted_command(
        app,
        SystemCommand::ShowNotification(notification),
    )?)
}

#[napi]
pub fn perform_notification_permission_request(app: u32, request: u32, prompt: bool) -> Result<()> {
    expect_unit(direct_command(
        app,
        SystemCommand::NotificationPermission { request, prompt },
    )?)
}

#[napi]
pub fn perform_hosted_notification_permission_request(
    app: u32,
    request: u32,
    prompt: bool,
) -> Result<()> {
    expect_unit(hosted_command(
        app,
        SystemCommand::NotificationPermission { request, prompt },
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
pub fn set_dock_badge(app: u32, value: Option<String>) -> Result<()> {
    expect_unit(direct_command(app, SystemCommand::SetDockBadge(value))?)
}

#[napi]
pub fn set_hosted_dock_badge(app: u32, value: Option<String>) -> Result<()> {
    expect_unit(hosted_command(app, SystemCommand::SetDockBadge(value))?)
}

#[napi]
pub fn set_dock_icon(app: u32, icon: Option<NativeImageSource>) -> Result<()> {
    let icon = icon
        .map(native_image)
        .transpose()
        .map_err(Error::from_reason)?;
    expect_unit(direct_command(app, SystemCommand::SetDockIcon(icon))?)
}

#[napi]
pub fn set_hosted_dock_icon(app: u32, icon: Option<NativeImageSource>) -> Result<()> {
    let icon = icon
        .map(native_image)
        .transpose()
        .map_err(Error::from_reason)?;
    expect_unit(hosted_command(app, SystemCommand::SetDockIcon(icon))?)
}

#[napi]
pub fn set_dock_menu(app: u32, menu: Option<String>) -> Result<()> {
    expect_unit(direct_command(app, SystemCommand::SetDockMenu(menu))?)
}

#[napi]
pub fn set_hosted_dock_menu(app: u32, menu: Option<String>) -> Result<()> {
    expect_unit(hosted_command(app, SystemCommand::SetDockMenu(menu))?)
}

#[napi]
pub fn add_recent_document(app: u32, path: String) -> Result<()> {
    expect_unit(direct_command(
        app,
        SystemCommand::AddRecentDocument(PathBuf::from(path)),
    )?)
}

#[napi]
pub fn add_hosted_recent_document(app: u32, path: String) -> Result<()> {
    expect_unit(hosted_command(
        app,
        SystemCommand::AddRecentDocument(PathBuf::from(path)),
    )?)
}

#[napi]
pub fn clear_recent_documents(app: u32) -> Result<()> {
    expect_unit(direct_command(app, SystemCommand::ClearRecentDocuments)?)
}

#[napi]
pub fn clear_hosted_recent_documents(app: u32) -> Result<()> {
    expect_unit(hosted_command(app, SystemCommand::ClearRecentDocuments)?)
}

#[napi]
pub fn show_about_panel(app: u32, options: NativeAboutPanelOptions) -> Result<()> {
    let options = about_panel_options(options).map_err(Error::from_reason)?;
    expect_unit(direct_command(app, SystemCommand::ShowAboutPanel(options))?)
}

#[napi]
pub fn show_hosted_about_panel(app: u32, options: NativeAboutPanelOptions) -> Result<()> {
    let options = about_panel_options(options).map_err(Error::from_reason)?;
    expect_unit(hosted_command(app, SystemCommand::ShowAboutPanel(options))?)
}

#[napi]
pub fn request_file_icon(app: u32, request: u32, path: String, size: String) -> Result<()> {
    let size = file_icon_size(&size).map_err(Error::from_reason)?;
    expect_unit(direct_command(
        app,
        SystemCommand::FileIcon {
            request,
            path: PathBuf::from(path),
            size,
        },
    )?)
}

#[napi]
pub fn request_hosted_file_icon(app: u32, request: u32, path: String, size: String) -> Result<()> {
    let size = file_icon_size(&size).map_err(Error::from_reason)?;
    expect_unit(hosted_command(
        app,
        SystemCommand::FileIcon {
            request,
            path: PathBuf::from(path),
            size,
        },
    )?)
}

#[napi]
pub fn set_user_tasks(app: u32, request: u32, tasks: Vec<NativeUserTask>) -> Result<()> {
    let tasks = user_tasks(tasks).map_err(Error::from_reason)?;
    expect_unit(direct_command(
        app,
        SystemCommand::SetUserTasks { request, tasks },
    )?)
}

#[napi]
pub fn set_hosted_user_tasks(app: u32, request: u32, tasks: Vec<NativeUserTask>) -> Result<()> {
    let tasks = user_tasks(tasks).map_err(Error::from_reason)?;
    expect_unit(hosted_command(
        app,
        SystemCommand::SetUserTasks { request, tasks },
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
pub fn perform_window_image_action(
    app: u32,
    window: u32,
    action: String,
    image: Option<NativeImageSource>,
    description: Option<String>,
) -> Result<()> {
    let action =
        parse_window_image_action(&action, image, description).map_err(Error::from_reason)?;
    expect_unit(direct_command(
        app,
        SystemCommand::WindowAction { window, action },
    )?)
}

#[napi]
pub fn perform_hosted_window_image_action(
    app: u32,
    window: u32,
    action: String,
    image: Option<NativeImageSource>,
    description: Option<String>,
) -> Result<()> {
    let action =
        parse_window_image_action(&action, image, description).map_err(Error::from_reason)?;
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
