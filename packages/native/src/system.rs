use std::{
    collections::HashMap,
    future::Future,
    path::PathBuf,
    pin::Pin,
    sync::Arc,
    task::{Context, Poll},
    time::{Duration, UNIX_EPOCH},
};

use napi::{
    Env, Error, Result, Task,
    bindgen_prelude::{AsyncTask, Buffer},
};
use napi_derive::napi;
use quickgui::{
    AboutPanelOptions, AppInfo, AppPaths, ClipboardEntry, ClipboardImage, ClipboardImageFormat,
    ClipboardItem, ClipboardString, DesktopIntegrationSupport, Display, Displays, ExternalPaths,
    FileIconResponse, FileIconSize, Image, KeyboardLayout, Menu, NotificationPermissionResponse,
    NotificationPermissionStatus, PermissionKind, PermissionManager, PermissionStatus, Point,
    PowerAssertion, PowerAssertionKind, PowerMonitor as CorePowerMonitor, PowerState, Rect,
    RelaunchOptions, ShellResponse, Size, SystemColor, SystemColorRole, SystemInfo,
    SystemNotification, SystemNotificationAction, SystemNotificationAttachment,
    SystemNotificationSound, SystemPreferences, TaskbarProgressState, UserTask, WindowAppearance,
    WindowBackgroundAppearance, WindowBounds, WindowLevel, WindowState,
};
use serde::Deserialize;

use super::{
    HOST, HostCommand, NativeAppOptions, NativeImageSource, NativeRuntime, QueuedEvent, ROOT_NODE,
    SyncReply, native_image, update_native_app_configuration, with_app_mut,
};

mod bindings;
pub(crate) mod menu;
mod tray;
pub use tray::NativeTrayIconOptions;

#[derive(Clone)]
#[napi(object)]
pub struct NativeRect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

#[derive(Clone)]
#[napi(object)]
pub struct NativeDisplay {
    pub id: String,
    pub uuid: Option<String>,
    pub name: String,
    pub bounds: NativeRect,
    pub work_area: NativeRect,
    pub scale_factor: f64,
    pub refresh_rate: Option<f64>,
    pub primary: bool,
}

#[derive(Clone)]
#[napi(object)]
pub struct NativeKeyboardLayout {
    pub id: String,
    pub name: String,
}

#[derive(Clone)]
#[napi(object)]
pub struct NativePoint {
    pub x: f64,
    pub y: f64,
}

#[derive(Clone)]
#[napi(object)]
pub struct NativeAppInfo {
    pub name: String,
    pub version: String,
    pub identifier: String,
}

#[derive(Clone)]
#[napi(object)]
pub struct NativeAppPaths {
    pub executable: String,
    pub executable_dir: String,
    pub resource_dir: String,
    pub home_dir: Option<String>,
    pub config_dir: Option<String>,
    pub data_dir: Option<String>,
    pub local_data_dir: Option<String>,
    pub cache_dir: Option<String>,
    pub log_dir: Option<String>,
    pub runtime_dir: Option<String>,
    pub temp_dir: String,
    pub audio_dir: Option<String>,
    pub desktop_dir: Option<String>,
    pub document_dir: Option<String>,
    pub download_dir: Option<String>,
    pub picture_dir: Option<String>,
    pub video_dir: Option<String>,
}

#[derive(Clone)]
#[napi(object)]
pub struct NativeSystemInfo {
    pub operating_system: String,
    pub family: String,
    pub name: String,
    pub version: Option<String>,
    pub edition: Option<String>,
    pub codename: Option<String>,
    pub architecture: String,
    pub bitness: String,
    pub hostname: Option<String>,
    pub locale: Option<String>,
    pub preferred_languages: Vec<String>,
    pub languages_truncated: bool,
}

#[derive(Clone)]
#[napi(object)]
pub struct NativeWindowRegistry {
    pub windows: Vec<u32>,
    pub active_window: Option<u32>,
    pub truncated: bool,
}

#[derive(Clone)]
#[napi(object)]
pub struct NativeDesktopIntegrationSupport {
    pub system_notifications: bool,
    pub scheduled_notifications: bool,
    pub notification_replies: bool,
    pub native_application_menus: bool,
    pub native_popup_menus: bool,
    pub tray_icons: bool,
    pub programmable_tray_popup: bool,
    pub global_shortcuts: bool,
    pub single_instance: bool,
    pub dynamic_protocol_registration: bool,
    pub autostart: bool,
    pub window_icons: bool,
    pub window_focusability: bool,
    pub window_opacity: bool,
    pub skip_taskbar: bool,
    pub visible_on_all_workspaces: bool,
    pub cursor_control: bool,
    pub cursor_screen_position: bool,
    pub taskbar_progress: bool,
    pub taskbar_overlay_icons: bool,
    pub dock_badges: bool,
    pub dock_icons: bool,
    pub dock_menus: bool,
    pub recent_documents: bool,
    pub file_icons: bool,
    pub native_about_panel: bool,
    pub user_tasks: bool,
}

#[derive(Clone)]
#[napi(object)]
pub struct NativeBatteryState {
    pub charge_percent: Option<u32>,
    pub status: String,
}

#[derive(Clone)]
#[napi(object)]
pub struct NativePowerState {
    pub source: String,
    pub battery: Option<NativeBatteryState>,
    pub thermal_state: String,
    pub low_power_mode: Option<bool>,
    pub cpu_speed_limit_percent: Option<u32>,
}

#[derive(Clone)]
#[napi(object)]
pub struct NativeSystemColor {
    pub red: u32,
    pub green: u32,
    pub blue: u32,
    pub alpha: u32,
}

#[derive(Clone)]
#[napi(object)]
pub struct NativeSystemPreferences {
    pub color_scheme: String,
    pub reduce_motion: Option<bool>,
    pub reduce_transparency: Option<bool>,
    pub increase_contrast: Option<bool>,
    pub differentiate_without_color: Option<bool>,
    pub invert_colors: Option<bool>,
    pub forced_colors: Option<bool>,
    pub screen_reader: Option<bool>,
    pub switch_control: Option<bool>,
    pub accent_color: Option<NativeSystemColor>,
    pub highlight_color: Option<NativeSystemColor>,
    pub highlight_text_color: Option<NativeSystemColor>,
    pub window_background_color: Option<NativeSystemColor>,
    pub window_text_color: Option<NativeSystemColor>,
    pub control_background_color: Option<NativeSystemColor>,
    pub control_text_color: Option<NativeSystemColor>,
    pub link_color: Option<NativeSystemColor>,
}

#[napi]
pub struct NativePowerAssertion {
    inner: PowerAssertion,
}

#[napi]
impl NativePowerAssertion {
    #[napi(constructor)]
    pub fn new(kind: String, reason: String) -> Result<Self> {
        let kind = match kind.as_str() {
            "prevent-application-suspension" | "preventAppSuspension" => {
                PowerAssertionKind::PreventApplicationSuspension
            }
            "prevent-display-sleep" | "preventDisplaySleep" => {
                PowerAssertionKind::PreventDisplaySleep
            }
            value => {
                return Err(Error::from_reason(format!(
                    "unknown power assertion kind `{value}`"
                )));
            }
        };
        Ok(Self {
            inner: PowerAssertion::acquire(kind, reason)
                .map_err(|error| Error::from_reason(error.to_string()))?,
        })
    }

    #[napi(getter)]
    pub fn kind(&self) -> String {
        match self.inner.kind() {
            PowerAssertionKind::PreventApplicationSuspension => "prevent-application-suspension",
            PowerAssertionKind::PreventDisplaySleep => "prevent-display-sleep",
        }
        .to_owned()
    }

    #[napi(getter)]
    pub fn reason(&self) -> String {
        self.inner.reason().to_owned()
    }

    #[napi(getter)]
    pub fn active(&self) -> bool {
        self.inner.is_active()
    }

    #[napi]
    pub fn release(&mut self) -> Result<bool> {
        self.inner
            .release()
            .map_err(|error| Error::from_reason(error.to_string()))
    }
}

#[derive(Clone)]
#[napi(object)]
pub struct NativeWindowState {
    pub display_id: Option<String>,
    pub kind: String,
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    pub viewport_width: f64,
    pub viewport_height: f64,
    pub minimum_width: Option<f64>,
    pub minimum_height: Option<f64>,
    pub maximum_width: Option<f64>,
    pub maximum_height: Option<f64>,
    pub scale_factor: f64,
    pub appearance: String,
    pub background_appearance: String,
    pub focused: bool,
    pub focusable: bool,
    pub visible: bool,
    pub minimized: bool,
    pub maximized: bool,
    pub fullscreen: bool,
    pub occluded: bool,
    pub movable: bool,
    pub resizable: bool,
    pub minimizable: bool,
    pub maximizable: bool,
    pub closable: bool,
    pub decorated: bool,
    pub shadow: bool,
    pub content_protected: bool,
    pub window_level: String,
    pub skip_taskbar: bool,
    pub visible_on_all_workspaces: bool,
    pub opacity: f64,
    pub has_icon: bool,
    pub taskbar_progress_state: String,
    pub taskbar_progress: f64,
    pub has_taskbar_overlay_icon: bool,
    pub cursor_visible: bool,
    pub cursor_grab: String,
    pub cursor_hit_test: bool,
    pub cursor_x: Option<f64>,
    pub cursor_y: Option<f64>,
    pub represented_file: bool,
    pub document_edited: bool,
    pub native_tabbing: bool,
    pub native_tab_count: u32,
    pub native_selected_tab: Option<u32>,
    pub native_tab_bar_visible: bool,
    pub native_tab_overview_visible: bool,
    pub native_tabs_truncated: bool,
}

#[napi(object)]
pub struct NativeClipboardEntry {
    pub kind: String,
    pub text: Option<String>,
    pub metadata: Option<String>,
    pub format: Option<String>,
    pub data: Option<Buffer>,
    pub paths: Option<Vec<String>>,
    pub url: Option<String>,
}

#[napi(object)]
pub struct NativeClipboardItem {
    pub entries: Vec<NativeClipboardEntry>,
}

#[derive(Clone)]
#[napi(object)]
pub struct NativeNotificationAction {
    pub id: String,
    pub label: String,
    pub kind: Option<String>,
    pub placeholder: Option<String>,
}

#[derive(Clone)]
#[napi(object)]
pub struct NativeNotificationAttachment {
    pub id: String,
    pub path: String,
}

#[derive(Clone)]
#[napi(object)]
pub struct NativeNotificationOptions {
    pub tag: String,
    pub title: String,
    pub body: String,
    pub subtitle: Option<String>,
    pub actions: Vec<NativeNotificationAction>,
    /// `default`, `silent`, or a platform-recognized named sound.
    pub sound: Option<String>,
    pub icon_path: Option<String>,
    pub attachments: Option<Vec<NativeNotificationAttachment>>,
    /// Absolute Unix epoch milliseconds.
    pub delivery_at_ms: Option<f64>,
}

#[derive(Clone)]
#[napi(object)]
pub struct NativeRelaunchOptions {
    pub executable: Option<String>,
    pub arguments: Option<Vec<String>>,
    pub clear_arguments: Option<bool>,
    pub working_directory: Option<String>,
}

#[derive(Clone, Default)]
#[napi(object)]
pub struct NativeAboutPanelOptions {
    pub application_name: Option<String>,
    pub application_version: Option<String>,
    pub version: Option<String>,
    pub copyright: Option<String>,
    pub credits: Option<String>,
    pub icon: Option<NativeImageSource>,
}

#[derive(Clone)]
#[napi(object)]
pub struct NativeUserTask {
    pub title: String,
    pub arguments: String,
    pub program: Option<String>,
    pub description: Option<String>,
    pub working_directory: Option<String>,
    pub icon_path: Option<String>,
    pub icon_index: Option<i32>,
}

pub(super) enum SystemCommand {
    ConfigureApp(NativeAppOptions),
    Exit,
    Relaunch(NativeRelaunchOptions),
    GetAppInfo,
    GetAppPaths,
    GetSystemInfo,
    GetWindowRegistry,
    GetCursorScreenPosition,
    GetDesktopIntegrationSupport,
    GetSystemPreferences,
    GetDisplays,
    GetKeyboardLayout,
    GetWindowState(u32),
    ReadClipboard,
    WriteClipboard(ClipboardItem),
    ShowNotification(SystemNotification),
    DismissNotification(String),
    NotificationPermission {
        request: u32,
        prompt: bool,
    },
    SetDockBadge(Option<String>),
    SetDockIcon(Option<Image>),
    SetDockMenu(Option<String>),
    AddRecentDocument(PathBuf),
    ClearRecentDocuments,
    ShowAboutPanel(AboutPanelOptions),
    FileIcon {
        request: u32,
        path: PathBuf,
        size: FileIconSize,
    },
    SetUserTasks {
        request: u32,
        tasks: Vec<UserTask>,
    },
    SetApplicationMenu(String),
    RequestSingleInstanceLock(String),
    ReleaseSingleInstanceLock,
    GlobalShortcut {
        request: u32,
        action: GlobalShortcutAction,
    },
    Tray {
        request: u32,
        action: tray::TrayAction,
    },
    WindowAction {
        window: u32,
        action: WindowAction,
    },
    ShellAction {
        request: u32,
        action: ShellAction,
    },
}

pub(super) enum GlobalShortcutAction {
    Register {
        registration: u32,
        accelerator: String,
    },
    Unregister {
        registration: u32,
    },
    UnregisterAll,
}

pub(super) enum WindowAction {
    SetTitle(String),
    SetBounds(WindowBounds),
    Move(Point),
    Resize(Size),
    Minimize,
    Maximize,
    Restore,
    SetFullscreen(bool),
    SetVisible(bool),
    SetResizable(bool),
    SetMovable(bool),
    SetMinimumSize(Option<Size>),
    SetMaximumSize(Option<Size>),
    SetMinimizable(bool),
    SetMaximizable(bool),
    SetClosable(bool),
    SetDecorated(bool),
    SetShadow(bool),
    SetContentProtected(bool),
    SetWindowLevel(Option<WindowLevel>),
    SetFocusable(bool),
    SetSkipTaskbar(bool),
    SetVisibleOnAllWorkspaces(bool),
    SetOpacity(f32),
    SetIcon(Option<Image>),
    SetCursorVisible(bool),
    SetCursorGrab(quickgui::CursorGrabMode),
    SetCursorHitTest(bool),
    SetCursorPosition(Point),
    SetTaskbarProgress(TaskbarProgressState, f32),
    SetTaskbarOverlayIcon(Image, String),
    ClearTaskbarOverlayIcon,
    Focus,
    RequestAttention,
    SetRepresentedFile(Option<PathBuf>),
    SetDocumentEdited(bool),
    SetAppearance(Option<WindowAppearance>),
    SetBackgroundAppearance(WindowBackgroundAppearance),
}

pub(super) enum ShellAction {
    OpenExternal(String),
    OpenPath(PathBuf),
    RevealPath(PathBuf),
    TrashPath(PathBuf),
}

pub(super) enum SystemCommandResult {
    Unit,
    Boolean(bool),
    AppInfo(Option<NativeAppInfo>),
    AppPaths(Option<NativeAppPaths>),
    SystemInfo(NativeSystemInfo),
    WindowRegistry(NativeWindowRegistry),
    Point(NativePoint),
    DesktopIntegrationSupport(NativeDesktopIntegrationSupport),
    SystemPreferences(NativeSystemPreferences),
    Displays(Vec<NativeDisplay>),
    KeyboardLayout(NativeKeyboardLayout),
    WindowState(NativeWindowState),
    Clipboard(Option<ClipboardItem>),
}

#[derive(Default)]
pub(super) struct SystemObservation {
    displays: Option<Displays>,
    windows: HashMap<u32, WindowState>,
    preferences: Option<SystemPreferences>,
}

pub(super) struct PendingShell {
    request: u32,
    response: ShellResponse,
}

pub(super) struct PendingNotificationPermission {
    request: u32,
    response: NotificationPermissionResponse,
}

pub(super) struct PendingFileIcon {
    request: u32,
    response: FileIconResponse,
}

pub(super) struct PendingUserTasks {
    request: u32,
    response: ShellResponse,
}

pub(super) struct PendingGlobalShortcut {
    request: u32,
    response: quickgui::PlatformResponse<()>,
}

pub(super) struct PendingTray {
    request: u32,
    response: quickgui::PlatformResponse<()>,
}

impl PendingTray {
    fn new(request: u32, response: quickgui::PlatformResponse<()>) -> Self {
        Self { request, response }
    }

    pub(super) fn request(&self) -> u32 {
        self.request
    }

    pub(super) fn poll(&mut self, context: &mut Context<'_>) -> Poll<super::NativeEvent> {
        let error = match Pin::new(&mut self.response).poll(context) {
            Poll::Ready(Ok(())) => None,
            Poll::Ready(Err(error)) => Some(error.to_string()),
            Poll::Pending => return Poll::Pending,
        };
        Poll::Ready(super::NativeEvent {
            kind: "tray-operation".to_owned(),
            window: 0,
            target: self.request,
            value: None,
            paths: None,
            data: None,
            width: None,
            height: None,
            error,
        })
    }
}

impl PendingGlobalShortcut {
    fn new(request: u32, response: quickgui::PlatformResponse<()>) -> Self {
        Self { request, response }
    }

    pub(super) fn request(&self) -> u32 {
        self.request
    }

    pub(super) fn poll(&mut self, context: &mut Context<'_>) -> Poll<super::NativeEvent> {
        let error = match Pin::new(&mut self.response).poll(context) {
            Poll::Ready(Ok(())) => None,
            Poll::Ready(Err(error)) => Some(error.to_string()),
            Poll::Pending => return Poll::Pending,
        };
        Poll::Ready(super::NativeEvent {
            kind: "global-shortcut-operation".to_owned(),
            window: 0,
            target: self.request,
            value: None,
            paths: None,
            data: None,
            width: None,
            height: None,
            error,
        })
    }
}

impl PendingShell {
    fn new(request: u32, response: ShellResponse) -> Self {
        Self { request, response }
    }

    pub(super) fn request(&self) -> u32 {
        self.request
    }

    pub(super) fn poll(&mut self, context: &mut Context<'_>) -> Poll<super::NativeEvent> {
        let error = match Pin::new(&mut self.response).poll(context) {
            Poll::Ready(Ok(())) => None,
            Poll::Ready(Err(error)) => Some(error.to_string()),
            Poll::Pending => return Poll::Pending,
        };
        Poll::Ready(super::NativeEvent {
            kind: "shell".to_owned(),
            window: 0,
            target: self.request,
            value: None,
            paths: None,
            data: None,
            width: None,
            height: None,
            error,
        })
    }
}

impl PendingNotificationPermission {
    fn new(request: u32, response: NotificationPermissionResponse) -> Self {
        Self { request, response }
    }

    pub(super) fn request(&self) -> u32 {
        self.request
    }

    pub(super) fn poll(&mut self, context: &mut Context<'_>) -> Poll<super::NativeEvent> {
        let (value, error) = match Pin::new(&mut self.response).poll(context) {
            Poll::Ready(Ok(status)) => {
                let status = match status {
                    NotificationPermissionStatus::NotDetermined => "not-determined",
                    NotificationPermissionStatus::Granted => "granted",
                    NotificationPermissionStatus::Denied => "denied",
                    NotificationPermissionStatus::Unsupported => "unsupported",
                };
                (Some(status.to_owned()), None)
            }
            Poll::Ready(Err(error)) => (None, Some(error.to_string())),
            Poll::Pending => return Poll::Pending,
        };
        Poll::Ready(super::NativeEvent {
            kind: "notification-permission".to_owned(),
            window: 0,
            target: self.request,
            value,
            paths: None,
            data: None,
            width: None,
            height: None,
            error,
        })
    }
}

impl PendingFileIcon {
    fn new(request: u32, response: FileIconResponse) -> Self {
        Self { request, response }
    }

    pub(super) fn request(&self) -> u32 {
        self.request
    }

    pub(super) fn poll(&mut self, context: &mut Context<'_>) -> Poll<super::NativeEvent> {
        let (data, width, height, error) = match Pin::new(&mut self.response).poll(context) {
            Poll::Ready(Ok(image)) => (
                Some(Buffer::from(image.rgba().to_vec())),
                Some(image.width()),
                Some(image.height()),
                None,
            ),
            Poll::Ready(Err(error)) => (None, None, None, Some(error.to_string())),
            Poll::Pending => return Poll::Pending,
        };
        Poll::Ready(super::NativeEvent {
            kind: "file-icon".to_owned(),
            window: 0,
            target: self.request,
            value: None,
            paths: None,
            data,
            width,
            height,
            error,
        })
    }
}

impl PendingUserTasks {
    fn new(request: u32, response: ShellResponse) -> Self {
        Self { request, response }
    }

    pub(super) fn request(&self) -> u32 {
        self.request
    }

    pub(super) fn poll(&mut self, context: &mut Context<'_>) -> Poll<super::NativeEvent> {
        let error = match Pin::new(&mut self.response).poll(context) {
            Poll::Ready(Ok(())) => None,
            Poll::Ready(Err(error)) => Some(error.to_string()),
            Poll::Pending => return Poll::Pending,
        };
        Poll::Ready(super::NativeEvent {
            kind: "user-tasks".to_owned(),
            window: 0,
            target: self.request,
            value: None,
            paths: None,
            data: None,
            width: None,
            height: None,
            error,
        })
    }
}

impl NativeRuntime {
    pub(super) fn execute_system_command(
        &mut self,
        command: SystemCommand,
    ) -> std::result::Result<SystemCommandResult, String> {
        self.sync_closed_windows();
        match command {
            SystemCommand::ConfigureApp(options) => {
                if self.runner.is_some() {
                    return Err(
                        "application options must be configured before readiness".to_owned()
                    );
                }
                let (app_info, app_paths, quit_mode) = update_native_app_configuration(
                    self.app_info.clone(),
                    self.app_paths.clone(),
                    self.quit_mode,
                    options,
                )?;
                self.app_info = app_info;
                self.app_paths = app_paths;
                self.quit_mode = quit_mode;
                Ok(SystemCommandResult::Unit)
            }
            SystemCommand::Exit => Ok(SystemCommandResult::Boolean(
                self.running_runner_mut()?.exit(),
            )),
            SystemCommand::Relaunch(options) => {
                let mut relaunch = RelaunchOptions::new();
                if let Some(executable) = options.executable {
                    relaunch = relaunch.executable(executable);
                }
                if options.clear_arguments.unwrap_or(false) {
                    if options.arguments.is_some() {
                        return Err(
                            "relaunch arguments and clearArguments cannot be combined".to_owned()
                        );
                    }
                    relaunch = relaunch.without_arguments();
                } else if let Some(arguments) = options.arguments {
                    relaunch = relaunch.arguments(arguments);
                }
                if let Some(directory) = options.working_directory {
                    relaunch = relaunch.working_directory(directory);
                }
                self.running_runner_mut()?
                    .relaunch_with(relaunch)
                    .map(SystemCommandResult::Boolean)
                    .map_err(|error| error.to_string())
            }
            SystemCommand::GetAppInfo => Ok(SystemCommandResult::AppInfo(
                self.running_runner()?.app_info().map(Into::into),
            )),
            SystemCommand::GetAppPaths => Ok(SystemCommandResult::AppPaths(
                self.running_runner()?.app_paths().map(Into::into),
            )),
            SystemCommand::GetSystemInfo => Ok(SystemCommandResult::SystemInfo(
                self.running_runner()?.system_info().into(),
            )),
            SystemCommand::GetWindowRegistry => {
                let runner = self.running_runner()?;
                let registry = runner.window_registry();
                let handles = self.handles.borrow();
                Ok(SystemCommandResult::WindowRegistry(NativeWindowRegistry {
                    windows: registry
                        .windows()
                        .iter()
                        .filter_map(|handle| handles.get(handle).copied())
                        .collect(),
                    active_window: runner
                        .active_window()
                        .and_then(|handle| handles.get(&handle).copied()),
                    truncated: registry.is_truncated(),
                }))
            }
            SystemCommand::GetCursorScreenPosition => self
                .running_runner()?
                .cursor_screen_position()
                .map(|point| SystemCommandResult::Point(point.into()))
                .map_err(|error| error.to_string()),
            SystemCommand::GetDesktopIntegrationSupport => {
                Ok(SystemCommandResult::DesktopIntegrationSupport(
                    DesktopIntegrationSupport::current().into(),
                ))
            }
            SystemCommand::GetSystemPreferences => Ok(SystemCommandResult::SystemPreferences(
                self.running_runner()?.system_preferences().into(),
            )),
            SystemCommand::GetDisplays => {
                let runner = self.running_runner()?;
                Ok(SystemCommandResult::Displays(
                    runner
                        .displays()
                        .all()
                        .iter()
                        .map(NativeDisplay::from)
                        .collect(),
                ))
            }
            SystemCommand::GetKeyboardLayout => {
                let layout = self.running_runner()?.keyboard_layout();
                Ok(SystemCommandResult::KeyboardLayout((&layout).into()))
            }
            SystemCommand::GetWindowState(window) => {
                let handle = self.system_window_handle(window)?;
                let state = self
                    .running_runner()?
                    .window_state(handle)
                    .ok_or_else(|| format!("native window {window} is not mounted"))?;
                Ok(SystemCommandResult::WindowState(state.into()))
            }
            SystemCommand::ReadClipboard => self
                .running_runner()?
                .read_from_clipboard()
                .map(SystemCommandResult::Clipboard)
                .map_err(|error| error.to_string()),
            SystemCommand::WriteClipboard(item) => self
                .running_runner()?
                .write_to_clipboard(item)
                .map(|()| SystemCommandResult::Unit)
                .map_err(|error| error.to_string()),
            SystemCommand::ShowNotification(notification) => self
                .running_runner_mut()?
                .show_system_notification(notification)
                .map(|()| SystemCommandResult::Unit)
                .map_err(|error| error.to_string()),
            SystemCommand::DismissNotification(tag) => self
                .running_runner_mut()?
                .dismiss_system_notification(tag)
                .map(|()| SystemCommandResult::Unit)
                .map_err(|error| error.to_string()),
            SystemCommand::NotificationPermission { request, prompt } => {
                if request == 0
                    || self
                        .pending_notification_permissions
                        .iter()
                        .any(|pending| pending.request() == request)
                {
                    return Err(
                        "notification-permission request ids must be nonzero and unique".to_owned(),
                    );
                }
                let runner = self.running_runner_mut()?;
                let response = if prompt {
                    runner.request_notification_permission()
                } else {
                    runner.notification_permission_status()
                }
                .map_err(|error| error.to_string())?;
                self.pending_notification_permissions
                    .push(PendingNotificationPermission::new(request, response));
                Ok(SystemCommandResult::Unit)
            }
            SystemCommand::SetDockBadge(value) => {
                let runner = self.running_runner_mut()?;
                match value {
                    Some(value) => runner.set_dock_badge(value),
                    None => runner.clear_dock_badge(),
                }
                .map(|()| SystemCommandResult::Unit)
                .map_err(|error| error.to_string())
            }
            SystemCommand::SetDockIcon(value) => {
                let runner = self.running_runner_mut()?;
                match value {
                    Some(icon) => runner.set_dock_icon(icon),
                    None => runner.clear_dock_icon(),
                }
                .map(|()| SystemCommandResult::Unit)
                .map_err(|error| error.to_string())
            }
            SystemCommand::SetDockMenu(value) => {
                let value = dock_menu(value)?;
                let runner = self.running_runner_mut()?;
                match value {
                    Some(menu) => runner.set_dock_menu(menu),
                    None => runner.clear_dock_menu(),
                }
                .map(|()| SystemCommandResult::Unit)
                .map_err(|error| error.to_string())
            }
            SystemCommand::AddRecentDocument(path) => self
                .running_runner_mut()?
                .add_recent_document(path)
                .map(|()| SystemCommandResult::Unit)
                .map_err(|error| error.to_string()),
            SystemCommand::ClearRecentDocuments => self
                .running_runner_mut()?
                .clear_recent_documents()
                .map(|()| SystemCommandResult::Unit)
                .map_err(|error| error.to_string()),
            SystemCommand::ShowAboutPanel(options) => self
                .running_runner_mut()?
                .show_about_panel(options)
                .map(|()| SystemCommandResult::Unit)
                .map_err(|error| error.to_string()),
            SystemCommand::FileIcon {
                request,
                path,
                size,
            } => {
                if request == 0
                    || self
                        .pending_file_icons
                        .iter()
                        .any(|pending| pending.request() == request)
                {
                    return Err("file-icon request ids must be nonzero and unique".to_owned());
                }
                let response = self
                    .running_runner_mut()?
                    .file_icon(path, size)
                    .map_err(|error| error.to_string())?;
                self.pending_file_icons
                    .push(PendingFileIcon::new(request, response));
                Ok(SystemCommandResult::Unit)
            }
            SystemCommand::SetUserTasks { request, tasks } => {
                if request == 0
                    || self
                        .pending_user_tasks
                        .iter()
                        .any(|pending| pending.request() == request)
                {
                    return Err("user-task request ids must be nonzero and unique".to_owned());
                }
                let response = self
                    .running_runner_mut()?
                    .set_user_tasks(tasks)
                    .map_err(|error| error.to_string())?;
                self.pending_user_tasks
                    .push(PendingUserTasks::new(request, response));
                Ok(SystemCommandResult::Unit)
            }
            SystemCommand::SetApplicationMenu(json) => {
                let menus = menu::application_menus(&json)?;
                self.running_runner_mut()?
                    .set_application_menus(menus)
                    .map(|()| SystemCommandResult::Unit)
                    .map_err(|error| error.to_string())
            }
            SystemCommand::RequestSingleInstanceLock(identifier) => self
                .running_runner_mut()?
                .request_single_instance_lock(identifier)
                .map(SystemCommandResult::Boolean)
                .map_err(|error| error.to_string()),
            SystemCommand::ReleaseSingleInstanceLock => Ok(SystemCommandResult::Boolean(
                self.running_runner_mut()?.release_single_instance_lock(),
            )),
            SystemCommand::GlobalShortcut { request, action } => {
                if request == 0
                    || self
                        .pending_global_shortcuts
                        .iter()
                        .any(|pending| pending.request() == request)
                {
                    return Err(
                        "native global-shortcut request ids must be nonzero and unique".to_owned(),
                    );
                }
                let runner = self.running_runner_mut()?;
                let response = match action {
                    GlobalShortcutAction::Register {
                        registration,
                        accelerator,
                    } => runner.register_global_shortcut(registration, accelerator),
                    GlobalShortcutAction::Unregister { registration } => {
                        runner.unregister_global_shortcut(registration)
                    }
                    GlobalShortcutAction::UnregisterAll => runner.unregister_all_global_shortcuts(),
                }
                .map_err(|error| error.to_string())?;
                self.pending_global_shortcuts
                    .push(PendingGlobalShortcut::new(request, response));
                Ok(SystemCommandResult::Unit)
            }
            SystemCommand::Tray { request, action } => {
                if request == 0
                    || self
                        .pending_tray
                        .iter()
                        .any(|pending| pending.request() == request)
                {
                    return Err("native tray request ids must be nonzero and unique".to_owned());
                }
                let runner = self.running_runner_mut()?;
                let response = match action {
                    tray::TrayAction::Set(options) => runner.set_tray_icon(options),
                    tray::TrayAction::Remove(id) => runner.remove_tray_icon(id),
                    tray::TrayAction::ShowMenu(id) => runner.show_tray_menu(id),
                }
                .map_err(|error| error.to_string())?;
                self.pending_tray.push(PendingTray::new(request, response));
                Ok(SystemCommandResult::Unit)
            }
            SystemCommand::WindowAction { window, action } => {
                let handle = self.system_window_handle(window)?;
                let runner = self.running_runner_mut()?;
                match action {
                    WindowAction::SetTitle(title) => runner.set_window_title(handle, title),
                    WindowAction::SetBounds(bounds) => runner.set_window_bounds(handle, bounds),
                    WindowAction::Move(position) => runner.move_window(handle, position),
                    WindowAction::Resize(size) => runner.resize_window(handle, size),
                    WindowAction::Minimize => runner.minimize_window(handle),
                    WindowAction::Maximize => runner.maximize_window(handle),
                    WindowAction::Restore => runner.restore_window(handle),
                    WindowAction::SetFullscreen(fullscreen) => {
                        runner.set_window_fullscreen(handle, fullscreen)
                    }
                    WindowAction::SetVisible(visible) => runner.set_window_visible(handle, visible),
                    WindowAction::SetResizable(value) => runner.set_window_resizable(handle, value),
                    WindowAction::SetMovable(value) => runner.set_window_movable(handle, value),
                    WindowAction::SetMinimumSize(value) => {
                        runner.set_window_minimum_size(handle, value)
                    }
                    WindowAction::SetMaximumSize(value) => {
                        runner.set_window_maximum_size(handle, value)
                    }
                    WindowAction::SetMinimizable(value) => {
                        runner.set_window_minimizable(handle, value)
                    }
                    WindowAction::SetMaximizable(value) => {
                        runner.set_window_maximizable(handle, value)
                    }
                    WindowAction::SetClosable(value) => runner.set_window_closable(handle, value),
                    WindowAction::SetDecorated(value) => runner.set_window_decorated(handle, value),
                    WindowAction::SetShadow(value) => runner.set_window_shadow(handle, value),
                    WindowAction::SetContentProtected(value) => {
                        runner.set_window_content_protected(handle, value)
                    }
                    WindowAction::SetWindowLevel(value) => runner.set_window_level(handle, value),
                    WindowAction::SetFocusable(value) => runner.set_window_focusable(handle, value),
                    WindowAction::SetSkipTaskbar(value) => {
                        runner.set_window_skip_taskbar(handle, value)
                    }
                    WindowAction::SetVisibleOnAllWorkspaces(value) => {
                        runner.set_window_visible_on_all_workspaces(handle, value)
                    }
                    WindowAction::SetOpacity(value) => runner.set_window_opacity(handle, value),
                    WindowAction::SetIcon(value) => runner.set_window_icon(handle, value),
                    WindowAction::SetCursorVisible(value) => {
                        runner.set_cursor_visible(handle, value)
                    }
                    WindowAction::SetCursorGrab(value) => runner.set_cursor_grab(handle, value),
                    WindowAction::SetCursorHitTest(value) => {
                        runner.set_cursor_hit_test(handle, value)
                    }
                    WindowAction::SetCursorPosition(value) => {
                        runner.set_cursor_position(handle, value)
                    }
                    WindowAction::SetTaskbarProgress(state, progress) => {
                        runner.set_taskbar_progress(handle, state, progress)
                    }
                    WindowAction::SetTaskbarOverlayIcon(icon, description) => {
                        runner.set_taskbar_overlay_icon(handle, icon, description)
                    }
                    WindowAction::ClearTaskbarOverlayIcon => {
                        runner.clear_taskbar_overlay_icon(handle)
                    }
                    WindowAction::Focus => runner.focus_window(handle),
                    WindowAction::RequestAttention => runner.request_window_attention(handle),
                    WindowAction::SetRepresentedFile(path) => {
                        runner.set_window_represented_file(handle, path)
                    }
                    WindowAction::SetDocumentEdited(edited) => {
                        runner.set_window_document_edited(handle, edited)
                    }
                    WindowAction::SetAppearance(appearance) => {
                        runner.set_window_appearance(handle, appearance)
                    }
                    WindowAction::SetBackgroundAppearance(appearance) => {
                        runner.set_window_background_appearance(handle, appearance)
                    }
                }
                .map_err(|error| error.to_string())?;
                Ok(SystemCommandResult::Unit)
            }
            SystemCommand::ShellAction { request, action } => {
                if request == 0
                    || self
                        .pending_shell
                        .iter()
                        .any(|pending| pending.request() == request)
                {
                    return Err("native shell request ids must be nonzero and unique".to_owned());
                }
                let runner = self.running_runner_mut()?;
                let response = match action {
                    ShellAction::OpenExternal(url) => runner.open_external(url),
                    ShellAction::OpenPath(path) => runner.open_path(path),
                    ShellAction::RevealPath(path) => runner.reveal_path(path),
                    ShellAction::TrashPath(path) => runner.trash_path(path),
                }
                .map_err(|error| error.to_string())?;
                self.pending_shell
                    .push(PendingShell::new(request, response));
                Ok(SystemCommandResult::Unit)
            }
        }
    }

    pub(super) fn observe_system_state(&mut self) {
        let Some(runner) = self.runner.as_ref() else {
            return;
        };
        let displays = runner.displays();
        let preferences = runner.system_preferences();
        let window_states = self
            .windows
            .iter()
            .filter_map(|(id, window)| {
                let handle = window.handle?;
                runner.window_state(handle).map(|state| (*id, state))
            })
            .collect::<HashMap<_, _>>();

        if let Some(previous) = &self.system_observation.displays
            && previous != &displays
        {
            super::enqueue_event(
                &self.events,
                QueuedEvent {
                    kind: "screen-change",
                    window: 0,
                    target: ROOT_NODE,
                    value: None,
                },
            );
        }

        if self
            .system_observation
            .preferences
            .is_some_and(|previous| previous != preferences)
        {
            super::enqueue_event(
                &self.events,
                QueuedEvent {
                    kind: "system-preferences-change",
                    window: 0,
                    target: ROOT_NODE,
                    value: None,
                },
            );
        }

        for (window, state) in &window_states {
            let Some(previous) = self.system_observation.windows.get(window) else {
                continue;
            };
            if previous != state {
                super::enqueue_event(
                    &self.events,
                    QueuedEvent {
                        kind: "window-state-change",
                        window: *window,
                        target: ROOT_NODE,
                        value: None,
                    },
                );
            }
            if previous.appearance != state.appearance {
                let value: Arc<str> = match state.appearance {
                    WindowAppearance::Light => Arc::from("light"),
                    WindowAppearance::Dark => Arc::from("dark"),
                };
                super::enqueue_event(
                    &self.events,
                    QueuedEvent {
                        kind: "appearance-change",
                        window: *window,
                        target: ROOT_NODE,
                        value: Some(value),
                    },
                );
            }
        }

        self.system_observation.displays = Some(displays);
        self.system_observation.windows = window_states;
        self.system_observation.preferences = Some(preferences);
    }

    fn running_runner(&self) -> std::result::Result<&quickgui::AppRunner, String> {
        self.runner
            .as_ref()
            .ok_or_else(|| "the system API requires a running QuickGUI application".to_owned())
    }

    fn running_runner_mut(&mut self) -> std::result::Result<&mut quickgui::AppRunner, String> {
        self.runner
            .as_mut()
            .ok_or_else(|| "the system API requires a running QuickGUI application".to_owned())
    }

    fn system_window_handle(
        &self,
        window: u32,
    ) -> std::result::Result<quickgui::WindowHandle, String> {
        self.windows
            .get(&window)
            .ok_or_else(|| format!("unknown QuickGUI window {window}"))?
            .handle
            .ok_or_else(|| format!("native window {window} is not mounted"))
    }
}

impl From<&Display> for NativeDisplay {
    fn from(display: &Display) -> Self {
        Self {
            id: display.id().get().to_string(),
            uuid: display.uuid().map(|uuid| uuid.to_string()),
            name: display.name().to_owned(),
            bounds: display.bounds().into(),
            work_area: display.visible_bounds().into(),
            scale_factor: f64::from(display.scale_factor()),
            refresh_rate: display
                .refresh_rate_millihertz()
                .map(|rate| f64::from(rate) / 1_000.0),
            primary: display.is_primary(),
        }
    }
}

impl From<quickgui::Rect> for NativeRect {
    fn from(rect: quickgui::Rect) -> Self {
        Self {
            x: f64::from(rect.x),
            y: f64::from(rect.y),
            width: f64::from(rect.width),
            height: f64::from(rect.height),
        }
    }
}

impl From<&KeyboardLayout> for NativeKeyboardLayout {
    fn from(layout: &KeyboardLayout) -> Self {
        Self {
            id: layout.id().to_owned(),
            name: layout.name().to_owned(),
        }
    }
}

impl From<Point> for NativePoint {
    fn from(point: Point) -> Self {
        Self {
            x: f64::from(point.x),
            y: f64::from(point.y),
        }
    }
}

impl From<&AppInfo> for NativeAppInfo {
    fn from(info: &AppInfo) -> Self {
        Self {
            name: info.name().to_owned(),
            version: info.version().to_owned(),
            identifier: info.identifier().to_owned(),
        }
    }
}

impl From<&AppPaths> for NativeAppPaths {
    fn from(paths: &AppPaths) -> Self {
        let optional =
            |path: Option<&std::path::Path>| path.map(|path| path.to_string_lossy().into_owned());
        Self {
            executable: paths.executable().to_string_lossy().into_owned(),
            executable_dir: paths.executable_dir().to_string_lossy().into_owned(),
            resource_dir: paths.resource_dir().to_string_lossy().into_owned(),
            home_dir: optional(paths.home_dir()),
            config_dir: optional(paths.config_dir()),
            data_dir: optional(paths.data_dir()),
            local_data_dir: optional(paths.local_data_dir()),
            cache_dir: optional(paths.cache_dir()),
            log_dir: optional(paths.log_dir()),
            runtime_dir: optional(paths.runtime_dir()),
            temp_dir: paths.temp_dir().to_string_lossy().into_owned(),
            audio_dir: optional(paths.audio_dir()),
            desktop_dir: optional(paths.desktop_dir()),
            document_dir: optional(paths.document_dir()),
            download_dir: optional(paths.download_dir()),
            picture_dir: optional(paths.picture_dir()),
            video_dir: optional(paths.video_dir()),
        }
    }
}

impl From<&SystemInfo> for NativeSystemInfo {
    fn from(info: &SystemInfo) -> Self {
        Self {
            operating_system: info.operating_system().as_str().to_owned(),
            family: info.family().as_str().to_owned(),
            name: info.name().to_owned(),
            version: info.version().map(str::to_owned),
            edition: info.edition().map(str::to_owned),
            codename: info.codename().map(str::to_owned),
            architecture: info.architecture().to_owned(),
            bitness: match info.bitness() {
                quickgui::SystemBitness::X32 => "32",
                quickgui::SystemBitness::X64 => "64",
                quickgui::SystemBitness::Unknown => "unknown",
            }
            .to_owned(),
            hostname: info.hostname().map(str::to_owned),
            locale: info.locale().map(str::to_owned),
            preferred_languages: info
                .preferred_languages()
                .iter()
                .map(ToString::to_string)
                .collect(),
            languages_truncated: info.languages_truncated(),
        }
    }
}

impl From<DesktopIntegrationSupport> for NativeDesktopIntegrationSupport {
    fn from(support: DesktopIntegrationSupport) -> Self {
        Self {
            system_notifications: support.system_notifications,
            scheduled_notifications: support.scheduled_notifications,
            notification_replies: support.notification_replies,
            native_application_menus: support.native_application_menus,
            native_popup_menus: support.native_popup_menus,
            tray_icons: support.tray_icons,
            programmable_tray_popup: support.programmable_tray_popup,
            global_shortcuts: support.global_shortcuts,
            single_instance: support.single_instance,
            dynamic_protocol_registration: support.dynamic_protocol_registration,
            autostart: support.autostart,
            window_icons: support.window_icons,
            window_focusability: support.window_focusability,
            window_opacity: support.window_opacity,
            skip_taskbar: support.skip_taskbar,
            visible_on_all_workspaces: support.visible_on_all_workspaces,
            cursor_control: support.cursor_control,
            cursor_screen_position: support.cursor_screen_position,
            taskbar_progress: support.taskbar_progress,
            taskbar_overlay_icons: support.taskbar_overlay_icons,
            dock_badges: support.dock_badges,
            dock_icons: support.dock_icons,
            dock_menus: support.dock_menus,
            recent_documents: support.recent_documents,
            file_icons: support.file_icons,
            native_about_panel: support.native_about_panel,
            user_tasks: support.user_tasks,
        }
    }
}

impl From<PowerState> for NativePowerState {
    fn from(state: PowerState) -> Self {
        Self {
            source: match state.source() {
                quickgui::PowerSource::Ac => "ac",
                quickgui::PowerSource::Battery => "battery",
                quickgui::PowerSource::Unknown => "unknown",
            }
            .to_owned(),
            battery: state.battery().map(|battery| NativeBatteryState {
                charge_percent: battery.charge_percent().map(u32::from),
                status: match battery.status() {
                    quickgui::BatteryStatus::Charging => "charging",
                    quickgui::BatteryStatus::Discharging => "discharging",
                    quickgui::BatteryStatus::Full => "full",
                    quickgui::BatteryStatus::NotCharging => "not-charging",
                    quickgui::BatteryStatus::Unknown => "unknown",
                }
                .to_owned(),
            }),
            thermal_state: thermal_state_name(state.thermal_state()).to_owned(),
            low_power_mode: state.low_power_mode(),
            cpu_speed_limit_percent: state.cpu_speed_limit_percent().map(u32::from),
        }
    }
}

fn thermal_state_name(state: quickgui::ThermalState) -> &'static str {
    match state {
        quickgui::ThermalState::Unknown => "unknown",
        quickgui::ThermalState::Nominal => "nominal",
        quickgui::ThermalState::Fair => "fair",
        quickgui::ThermalState::Serious => "serious",
        quickgui::ThermalState::Critical => "critical",
    }
}

impl From<SystemColor> for NativeSystemColor {
    fn from(color: SystemColor) -> Self {
        Self {
            red: u32::from(color.red),
            green: u32::from(color.green),
            blue: u32::from(color.blue),
            alpha: u32::from(color.alpha),
        }
    }
}

impl From<SystemPreferences> for NativeSystemPreferences {
    fn from(preferences: SystemPreferences) -> Self {
        let color = |role| preferences.system_color(role).map(Into::into);
        Self {
            color_scheme: match preferences.color_scheme() {
                quickgui::ColorScheme::Light => "light",
                quickgui::ColorScheme::Dark => "dark",
                quickgui::ColorScheme::Unknown => "unknown",
            }
            .to_owned(),
            reduce_motion: preferences.reduce_motion(),
            reduce_transparency: preferences.reduce_transparency(),
            increase_contrast: preferences.increase_contrast(),
            differentiate_without_color: preferences.differentiate_without_color(),
            invert_colors: preferences.invert_colors(),
            forced_colors: preferences.forced_colors(),
            screen_reader: preferences.screen_reader(),
            switch_control: preferences.switch_control(),
            accent_color: color(SystemColorRole::Accent),
            highlight_color: color(SystemColorRole::Highlight),
            highlight_text_color: color(SystemColorRole::HighlightText),
            window_background_color: color(SystemColorRole::WindowBackground),
            window_text_color: color(SystemColorRole::WindowText),
            control_background_color: color(SystemColorRole::ControlBackground),
            control_text_color: color(SystemColorRole::ControlText),
            link_color: color(SystemColorRole::Link),
        }
    }
}

fn parse_permission_kind(value: &str) -> Result<PermissionKind> {
    match value {
        "camera" => Ok(PermissionKind::Camera),
        "microphone" => Ok(PermissionKind::Microphone),
        "screen-recording" | "screenRecording" => Ok(PermissionKind::ScreenRecording),
        "accessibility" => Ok(PermissionKind::Accessibility),
        value => Err(Error::from_reason(format!(
            "unknown native permission kind `{value}`"
        ))),
    }
}

fn permission_status_name(status: PermissionStatus) -> String {
    match status {
        PermissionStatus::NotDetermined => "not-determined",
        PermissionStatus::Granted => "granted",
        PermissionStatus::Denied => "denied",
        PermissionStatus::Restricted => "restricted",
        PermissionStatus::Unknown => "unknown",
    }
    .to_owned()
}

#[doc(hidden)]
pub struct PermissionRequestTask {
    kind: PermissionKind,
}

impl Task for PermissionRequestTask {
    type Output = PermissionStatus;
    type JsValue = String;

    fn compute(&mut self) -> Result<Self::Output> {
        let (sender, receiver) = std::sync::mpsc::sync_channel(1);
        PermissionManager::request(self.kind, move |result| {
            let _ = sender.send(result);
        })
        .map_err(|error| Error::from_reason(error.to_string()))?;
        receiver
            .recv()
            .map_err(|_| Error::from_reason("the native permission request was cancelled"))?
            .map_err(|error| Error::from_reason(error.to_string()))
    }

    fn resolve(&mut self, _env: Env, output: Self::Output) -> Result<Self::JsValue> {
        Ok(permission_status_name(output))
    }
}

impl From<WindowState> for NativeWindowState {
    fn from(state: WindowState) -> Self {
        let bounds = state.bounds.bounds();
        Self {
            display_id: state.display_id.map(|id| id.get().to_string()),
            kind: match state.kind {
                quickgui::WindowKind::Normal => "normal",
                quickgui::WindowKind::Popover => "popover",
                quickgui::WindowKind::SystemPopover => "system-popover",
                quickgui::WindowKind::Floating => "floating",
                quickgui::WindowKind::Dialog => "dialog",
            }
            .to_owned(),
            x: f64::from(bounds.x),
            y: f64::from(bounds.y),
            width: f64::from(bounds.width),
            height: f64::from(bounds.height),
            viewport_width: f64::from(state.viewport_size.width),
            viewport_height: f64::from(state.viewport_size.height),
            minimum_width: state.minimum_size.map(|size| f64::from(size.width)),
            minimum_height: state.minimum_size.map(|size| f64::from(size.height)),
            maximum_width: state.maximum_size.map(|size| f64::from(size.width)),
            maximum_height: state.maximum_size.map(|size| f64::from(size.height)),
            scale_factor: f64::from(state.scale_factor),
            appearance: match state.appearance {
                WindowAppearance::Light => "light".to_owned(),
                WindowAppearance::Dark => "dark".to_owned(),
            },
            background_appearance: match state.background_appearance {
                WindowBackgroundAppearance::Opaque => "opaque",
                WindowBackgroundAppearance::Transparent => "transparent",
                WindowBackgroundAppearance::Blurred => "blurred",
            }
            .to_owned(),
            focused: state.focused,
            focusable: state.focusable,
            visible: state.visible,
            minimized: state.minimized,
            maximized: state.maximized,
            fullscreen: state.fullscreen,
            occluded: state.occluded,
            movable: state.movable,
            resizable: state.resizable,
            minimizable: state.minimizable,
            maximizable: state.maximizable,
            closable: state.closable,
            decorated: state.decorated,
            shadow: state.shadow,
            content_protected: state.content_protected,
            window_level: match state.window_level {
                WindowLevel::AlwaysOnBottom => "always-on-bottom",
                WindowLevel::Normal => "normal",
                WindowLevel::AlwaysOnTop => "always-on-top",
            }
            .to_owned(),
            skip_taskbar: state.skip_taskbar,
            visible_on_all_workspaces: state.visible_on_all_workspaces,
            opacity: f64::from(state.opacity),
            has_icon: state.has_icon,
            taskbar_progress_state: match state.taskbar_progress_state {
                TaskbarProgressState::None => "none",
                TaskbarProgressState::Normal => "normal",
                TaskbarProgressState::Indeterminate => "indeterminate",
                TaskbarProgressState::Paused => "paused",
                TaskbarProgressState::Error => "error",
            }
            .to_owned(),
            taskbar_progress: f64::from(state.taskbar_progress),
            has_taskbar_overlay_icon: state.has_taskbar_overlay_icon,
            cursor_visible: state.cursor_visible,
            cursor_grab: match state.cursor_grab {
                quickgui::CursorGrabMode::None => "none",
                quickgui::CursorGrabMode::Confined => "confined",
                quickgui::CursorGrabMode::Locked => "locked",
            }
            .to_owned(),
            cursor_hit_test: state.cursor_hit_test,
            cursor_x: state.cursor_position.map(|position| f64::from(position.x)),
            cursor_y: state.cursor_position.map(|position| f64::from(position.y)),
            represented_file: state.represented_file,
            document_edited: state.document_edited,
            native_tabbing: state.native_tabbing,
            native_tab_count: state.native_tabs.count as u32,
            native_selected_tab: state.native_tabs.selected_index.map(|index| index as u32),
            native_tab_bar_visible: state.native_tabs.tab_bar_visible,
            native_tab_overview_visible: state.native_tabs.overview_visible,
            native_tabs_truncated: state.native_tabs.truncated,
        }
    }
}

fn native_clipboard_item(item: ClipboardItem) -> NativeClipboardItem {
    let entries = item
        .entries()
        .iter()
        .map(|entry| match entry {
            ClipboardEntry::String(value) => NativeClipboardEntry {
                kind: "text".to_owned(),
                text: Some(value.text().to_owned()),
                metadata: value.metadata().map(str::to_owned),
                format: None,
                data: None,
                paths: None,
                url: None,
            },
            ClipboardEntry::Image(value) => NativeClipboardEntry {
                kind: "image".to_owned(),
                text: None,
                metadata: None,
                format: Some(value.format().mime_type().to_owned()),
                data: Some(Buffer::from(value.bytes().to_vec())),
                paths: None,
                url: None,
            },
            ClipboardEntry::Data(value) => NativeClipboardEntry {
                kind: "data".to_owned(),
                text: None,
                metadata: None,
                format: Some(value.mime_type().to_owned()),
                data: Some(Buffer::from(value.bytes().to_vec())),
                paths: None,
                url: None,
            },
            ClipboardEntry::Bookmark(value) => NativeClipboardEntry {
                kind: "bookmark".to_owned(),
                text: Some(value.title().to_owned()),
                metadata: None,
                format: None,
                data: None,
                paths: None,
                url: Some(value.url().to_owned()),
            },
            ClipboardEntry::ExternalPaths(value) => NativeClipboardEntry {
                kind: "files".to_owned(),
                text: None,
                metadata: None,
                format: None,
                data: None,
                paths: Some(
                    value
                        .paths()
                        .iter()
                        .map(|path| path.to_string_lossy().into_owned())
                        .collect(),
                ),
                url: None,
            },
        })
        .collect();
    NativeClipboardItem { entries }
}

fn clipboard_item(item: NativeClipboardItem) -> std::result::Result<ClipboardItem, String> {
    let entries = item
        .entries
        .into_iter()
        .map(|entry| match entry.kind.as_str() {
            "text" => {
                let text = entry
                    .text
                    .ok_or_else(|| "a text clipboard entry requires text".to_owned())?;
                let mut value = ClipboardString::new(text).map_err(|error| error.to_string())?;
                if let Some(metadata) = entry.metadata {
                    value = value
                        .with_metadata(metadata)
                        .map_err(|error| error.to_string())?;
                }
                Ok(ClipboardEntry::String(value))
            }
            "image" => {
                let format = entry
                    .format
                    .as_deref()
                    .and_then(ClipboardImageFormat::from_mime_type)
                    .ok_or_else(|| {
                        "an image clipboard entry requires a supported MIME type".to_owned()
                    })?;
                let data = entry
                    .data
                    .ok_or_else(|| "an image clipboard entry requires data".to_owned())?;
                ClipboardImage::new(format, Arc::<[u8]>::from(data.as_ref()))
                    .map(ClipboardEntry::Image)
                    .map_err(|error| error.to_string())
            }
            "data" => {
                let mime_type = entry
                    .format
                    .ok_or_else(|| "a data clipboard entry requires a MIME type".to_owned())?;
                let data = entry
                    .data
                    .ok_or_else(|| "a data clipboard entry requires data".to_owned())?;
                quickgui::ClipboardData::new(mime_type, Arc::<[u8]>::from(data.as_ref()))
                    .map(ClipboardEntry::Data)
                    .map_err(|error| error.to_string())
            }
            "bookmark" => {
                let title = entry
                    .text
                    .ok_or_else(|| "a bookmark clipboard entry requires a title".to_owned())?;
                let url = entry
                    .url
                    .ok_or_else(|| "a bookmark clipboard entry requires a URL".to_owned())?;
                quickgui::ClipboardBookmark::new(title, url)
                    .map(ClipboardEntry::Bookmark)
                    .map_err(|error| error.to_string())
            }
            "files" => {
                let paths = entry
                    .paths
                    .ok_or_else(|| "a files clipboard entry requires paths".to_owned())?;
                ExternalPaths::new(paths.into_iter().map(PathBuf::from))
                    .map(ClipboardEntry::ExternalPaths)
                    .map_err(|error| error.to_string())
            }
            kind => Err(format!("unknown clipboard entry kind `{kind}`")),
        })
        .collect::<std::result::Result<Vec<_>, _>>()?;
    ClipboardItem::new(entries).map_err(|error| error.to_string())
}

fn direct_command(app: u32, command: SystemCommand) -> Result<SystemCommandResult> {
    with_app_mut(app, |runtime| runtime.execute_system_command(command))
}

fn hosted_command(app: u32, command: SystemCommand) -> Result<SystemCommandResult> {
    let reply = Arc::new(SyncReply::new());
    HOST.enqueue(HostCommand::System {
        app,
        command,
        reply: Arc::clone(&reply),
    })
    .map_err(Error::from_reason)?;
    reply.wait().map_err(Error::from_reason)
}

fn expect_displays(result: SystemCommandResult) -> Result<Vec<NativeDisplay>> {
    match result {
        SystemCommandResult::Displays(displays) => Ok(displays),
        _ => Err(Error::from_reason(
            "native system command returned the wrong result",
        )),
    }
}

fn expect_keyboard_layout(result: SystemCommandResult) -> Result<NativeKeyboardLayout> {
    match result {
        SystemCommandResult::KeyboardLayout(layout) => Ok(layout),
        _ => Err(Error::from_reason(
            "native system command returned the wrong result",
        )),
    }
}

fn expect_window_state(result: SystemCommandResult) -> Result<NativeWindowState> {
    match result {
        SystemCommandResult::WindowState(state) => Ok(state),
        _ => Err(Error::from_reason(
            "native system command returned the wrong result",
        )),
    }
}

fn expect_clipboard(result: SystemCommandResult) -> Result<Option<NativeClipboardItem>> {
    match result {
        SystemCommandResult::Clipboard(item) => Ok(item.map(native_clipboard_item)),
        _ => Err(Error::from_reason(
            "native system command returned the wrong result",
        )),
    }
}

fn expect_unit(result: SystemCommandResult) -> Result<()> {
    match result {
        SystemCommandResult::Unit => Ok(()),
        _ => Err(Error::from_reason(
            "native system command returned the wrong result",
        )),
    }
}

fn expect_boolean(result: SystemCommandResult) -> Result<bool> {
    match result {
        SystemCommandResult::Boolean(value) => Ok(value),
        _ => Err(Error::from_reason(
            "native system command returned the wrong result",
        )),
    }
}

fn expect_app_info(result: SystemCommandResult) -> Result<Option<NativeAppInfo>> {
    match result {
        SystemCommandResult::AppInfo(value) => Ok(value),
        _ => Err(Error::from_reason(
            "native system command returned the wrong result",
        )),
    }
}

fn expect_app_paths(result: SystemCommandResult) -> Result<Option<NativeAppPaths>> {
    match result {
        SystemCommandResult::AppPaths(value) => Ok(value),
        _ => Err(Error::from_reason(
            "native system command returned the wrong result",
        )),
    }
}

fn expect_system_info(result: SystemCommandResult) -> Result<NativeSystemInfo> {
    match result {
        SystemCommandResult::SystemInfo(value) => Ok(value),
        _ => Err(Error::from_reason(
            "native system command returned the wrong result",
        )),
    }
}

fn expect_window_registry(result: SystemCommandResult) -> Result<NativeWindowRegistry> {
    match result {
        SystemCommandResult::WindowRegistry(value) => Ok(value),
        _ => Err(Error::from_reason(
            "native system command returned the wrong result",
        )),
    }
}

fn expect_point(result: SystemCommandResult) -> Result<NativePoint> {
    match result {
        SystemCommandResult::Point(value) => Ok(value),
        _ => Err(Error::from_reason(
            "native system command returned the wrong result",
        )),
    }
}

fn expect_desktop_integration_support(
    result: SystemCommandResult,
) -> Result<NativeDesktopIntegrationSupport> {
    match result {
        SystemCommandResult::DesktopIntegrationSupport(value) => Ok(value),
        _ => Err(Error::from_reason(
            "native system command returned the wrong result",
        )),
    }
}

fn expect_system_preferences(result: SystemCommandResult) -> Result<NativeSystemPreferences> {
    match result {
        SystemCommandResult::SystemPreferences(value) => Ok(value),
        _ => Err(Error::from_reason(
            "native system command returned the wrong result",
        )),
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct NativeBoundsPayload {
    x: f32,
    y: f32,
    width: f32,
    height: f32,
    #[serde(default)]
    state: Option<String>,
}

#[derive(Deserialize)]
struct NativePointPayload {
    x: f32,
    y: f32,
}

#[derive(Deserialize)]
struct NativeSizePayload {
    width: f32,
    height: f32,
}

#[derive(Deserialize)]
struct NativeTaskbarProgressPayload {
    state: String,
    progress: f32,
}

fn parse_window_action(
    action: &str,
    value: Option<String>,
) -> std::result::Result<WindowAction, String> {
    match action {
        "set-title" => Ok(WindowAction::SetTitle(
            value.ok_or_else(|| "set-title requires a value".to_owned())?,
        )),
        "set-bounds" => {
            let value: NativeBoundsPayload = parse_json_value(value, "set-bounds")?;
            let bounds = Rect::new(value.x, value.y, value.width, value.height);
            Ok(WindowAction::SetBounds(
                match value.state.as_deref().unwrap_or("normal") {
                    "normal" | "windowed" => WindowBounds::Windowed(bounds),
                    "maximized" => WindowBounds::Maximized(bounds),
                    "fullscreen" => WindowBounds::Fullscreen(bounds),
                    state => return Err(format!("unknown window bounds state `{state}`")),
                },
            ))
        }
        "move" => {
            let value: NativePointPayload = parse_json_value(value, "move")?;
            Ok(WindowAction::Move(Point::new(value.x, value.y)))
        }
        "resize" => {
            let value: NativeSizePayload = parse_json_value(value, "resize")?;
            Ok(WindowAction::Resize(Size::new(value.width, value.height)))
        }
        "minimize" => Ok(WindowAction::Minimize),
        "maximize" => Ok(WindowAction::Maximize),
        "restore" => Ok(WindowAction::Restore),
        "set-fullscreen" => Ok(WindowAction::SetFullscreen(parse_bool(value)?)),
        "set-visible" => Ok(WindowAction::SetVisible(parse_bool(value)?)),
        "set-resizable" => Ok(WindowAction::SetResizable(parse_bool(value)?)),
        "set-movable" => Ok(WindowAction::SetMovable(parse_bool(value)?)),
        "set-minimum-size" => Ok(WindowAction::SetMinimumSize(parse_optional_size(value)?)),
        "set-maximum-size" => Ok(WindowAction::SetMaximumSize(parse_optional_size(value)?)),
        "set-minimizable" => Ok(WindowAction::SetMinimizable(parse_bool(value)?)),
        "set-maximizable" => Ok(WindowAction::SetMaximizable(parse_bool(value)?)),
        "set-closable" => Ok(WindowAction::SetClosable(parse_bool(value)?)),
        "set-decorated" => Ok(WindowAction::SetDecorated(parse_bool(value)?)),
        "set-shadow" => Ok(WindowAction::SetShadow(parse_bool(value)?)),
        "set-content-protected" => Ok(WindowAction::SetContentProtected(parse_bool(value)?)),
        "set-window-level" => Ok(WindowAction::SetWindowLevel(match value.as_deref() {
            None | Some("automatic") => None,
            Some("always-on-bottom") | Some("alwaysOnBottom") => Some(WindowLevel::AlwaysOnBottom),
            Some("normal") => Some(WindowLevel::Normal),
            Some("always-on-top") | Some("alwaysOnTop") => Some(WindowLevel::AlwaysOnTop),
            Some(value) => return Err(format!("unknown window level `{value}`")),
        })),
        "set-focusable" => Ok(WindowAction::SetFocusable(parse_bool(value)?)),
        "set-skip-taskbar" => Ok(WindowAction::SetSkipTaskbar(parse_bool(value)?)),
        "set-visible-on-all-workspaces" => {
            Ok(WindowAction::SetVisibleOnAllWorkspaces(parse_bool(value)?))
        }
        "set-opacity" => Ok(WindowAction::SetOpacity(parse_f32(value, "set-opacity")?)),
        "set-cursor-visible" => Ok(WindowAction::SetCursorVisible(parse_bool(value)?)),
        "set-cursor-grab" => Ok(WindowAction::SetCursorGrab(match value.as_deref() {
            Some("none") => quickgui::CursorGrabMode::None,
            Some("confined") => quickgui::CursorGrabMode::Confined,
            Some("locked") => quickgui::CursorGrabMode::Locked,
            Some(value) => return Err(format!("unknown cursor grab mode `{value}`")),
            None => return Err("set-cursor-grab requires a value".to_owned()),
        })),
        "set-cursor-hit-test" => Ok(WindowAction::SetCursorHitTest(parse_bool(value)?)),
        "set-cursor-position" => {
            let value: NativePointPayload = parse_json_value(value, "set-cursor-position")?;
            Ok(WindowAction::SetCursorPosition(Point::new(
                value.x, value.y,
            )))
        }
        "set-taskbar-progress" => {
            let value: NativeTaskbarProgressPayload =
                parse_json_value(value, "set-taskbar-progress")?;
            Ok(WindowAction::SetTaskbarProgress(
                parse_taskbar_progress_state(&value.state)?,
                value.progress,
            ))
        }
        "clear-taskbar-overlay-icon" => Ok(WindowAction::ClearTaskbarOverlayIcon),
        "focus" => Ok(WindowAction::Focus),
        "request-attention" => Ok(WindowAction::RequestAttention),
        "set-represented-file" => Ok(WindowAction::SetRepresentedFile(
            value.filter(|value| !value.is_empty()).map(PathBuf::from),
        )),
        "set-document-edited" => Ok(WindowAction::SetDocumentEdited(parse_bool(value)?)),
        "set-appearance" => Ok(WindowAction::SetAppearance(match value.as_deref() {
            None | Some("system") => None,
            Some("light") => Some(WindowAppearance::Light),
            Some("dark") => Some(WindowAppearance::Dark),
            Some(value) => return Err(format!("unknown native appearance `{value}`")),
        })),
        "set-background-appearance" => Ok(WindowAction::SetBackgroundAppearance(
            match value.as_deref() {
                Some("opaque") => WindowBackgroundAppearance::Opaque,
                Some("transparent") => WindowBackgroundAppearance::Transparent,
                Some("blurred") | Some("blur") => WindowBackgroundAppearance::Blurred,
                Some(value) => return Err(format!("unknown background appearance `{value}`")),
                None => return Err("set-background-appearance requires a value".to_owned()),
            },
        )),
        action => Err(format!("unknown native window action `{action}`")),
    }
}

fn parse_window_image_action(
    action: &str,
    image: Option<NativeImageSource>,
    description: Option<String>,
) -> std::result::Result<WindowAction, String> {
    match action {
        "set-icon" => Ok(WindowAction::SetIcon(Some(native_image(
            image.ok_or_else(|| "set-icon requires an image".to_owned())?,
        )?))),
        "clear-icon" if image.is_none() && description.is_none() => Ok(WindowAction::SetIcon(None)),
        "set-taskbar-overlay-icon" => Ok(WindowAction::SetTaskbarOverlayIcon(
            native_image(
                image.ok_or_else(|| "set-taskbar-overlay-icon requires an image".to_owned())?,
            )?,
            description.ok_or_else(|| {
                "set-taskbar-overlay-icon requires an accessibility description".to_owned()
            })?,
        )),
        "clear-icon" => Err("clear-icon does not accept an image or description".to_owned()),
        value => Err(format!("unknown native window image action `{value}`")),
    }
}

fn parse_json_value<T: for<'de> Deserialize<'de>>(
    value: Option<String>,
    action: &str,
) -> std::result::Result<T, String> {
    let value = value.ok_or_else(|| format!("{action} requires a value"))?;
    serde_json::from_str(&value).map_err(|error| format!("invalid {action} value: {error}"))
}

fn parse_optional_size(value: Option<String>) -> std::result::Result<Option<Size>, String> {
    value
        .map(|value| {
            serde_json::from_str::<NativeSizePayload>(&value)
                .map(|value| Size::new(value.width, value.height))
                .map_err(|error| format!("invalid window size: {error}"))
        })
        .transpose()
}

fn parse_f32(value: Option<String>, action: &str) -> std::result::Result<f32, String> {
    value
        .ok_or_else(|| format!("{action} requires a value"))?
        .parse::<f32>()
        .map_err(|_| format!("{action} requires a finite number"))
}

fn parse_taskbar_progress_state(state: &str) -> std::result::Result<TaskbarProgressState, String> {
    match state {
        "none" => Ok(TaskbarProgressState::None),
        "normal" => Ok(TaskbarProgressState::Normal),
        "indeterminate" => Ok(TaskbarProgressState::Indeterminate),
        "paused" => Ok(TaskbarProgressState::Paused),
        "error" => Ok(TaskbarProgressState::Error),
        value => Err(format!("unknown taskbar progress state `{value}`")),
    }
}

fn parse_global_shortcut_action(
    action: &str,
    registration: Option<u32>,
    accelerator: Option<String>,
) -> std::result::Result<GlobalShortcutAction, String> {
    match action {
        "register" => Ok(GlobalShortcutAction::Register {
            registration: registration
                .filter(|registration| *registration != 0)
                .ok_or_else(|| {
                    "global-shortcut register requires a nonzero registration id".to_owned()
                })?,
            accelerator: accelerator
                .ok_or_else(|| "global-shortcut register requires an accelerator".to_owned())?,
        }),
        "unregister" => Ok(GlobalShortcutAction::Unregister {
            registration: registration
                .filter(|registration| *registration != 0)
                .ok_or_else(|| {
                    "global-shortcut unregister requires a nonzero registration id".to_owned()
                })?,
        }),
        "unregister-all" => Ok(GlobalShortcutAction::UnregisterAll),
        action => Err(format!("unknown global-shortcut action `{action}`")),
    }
}

fn system_notification(
    options: NativeNotificationOptions,
) -> std::result::Result<SystemNotification, String> {
    let mut notification = SystemNotification::new(options.tag, options.title, options.body);
    if let Some(subtitle) = options.subtitle {
        notification = notification.subtitle(subtitle);
    }
    for action in options.actions {
        let mut native = SystemNotificationAction::new(action.id, action.label);
        match action.kind.as_deref().unwrap_or("button") {
            "button" if action.placeholder.is_none() => {}
            "text-input" | "textInput" => {
                native = match action.placeholder {
                    Some(placeholder) => native.text_input(placeholder),
                    None => native.text_input_without_placeholder(),
                };
            }
            "button" => {
                return Err("a button notification action cannot have a placeholder".to_owned());
            }
            value => return Err(format!("unknown notification action kind `{value}`")),
        }
        notification = notification.action(native);
    }
    notification = notification.sound(match options.sound.as_deref().unwrap_or("default") {
        "default" => SystemNotificationSound::Default,
        "silent" => SystemNotificationSound::Silent,
        value => SystemNotificationSound::Named(Arc::from(value)),
    });
    if let Some(path) = options.icon_path {
        notification = notification.icon(path);
    }
    for attachment in options.attachments.unwrap_or_default() {
        notification = notification.attachment(SystemNotificationAttachment::new(
            attachment.id,
            attachment.path,
        ));
    }
    if let Some(milliseconds) = options.delivery_at_ms {
        if !milliseconds.is_finite() || milliseconds < 0.0 {
            return Err("notification delivery time must be finite epoch milliseconds".to_owned());
        }
        let duration = Duration::try_from_secs_f64(milliseconds / 1_000.0)
            .map_err(|_| "notification delivery time is out of range".to_owned())?;
        notification = notification.deliver_at(
            UNIX_EPOCH
                .checked_add(duration)
                .ok_or_else(|| "notification delivery time is out of range".to_owned())?,
        );
    }
    Ok(notification)
}

fn about_panel_options(
    options: NativeAboutPanelOptions,
) -> std::result::Result<AboutPanelOptions, String> {
    let mut native = AboutPanelOptions::new();
    if let Some(value) = options.application_name {
        native = native.application_name(value);
    }
    if let Some(value) = options.application_version {
        native = native.application_version(value);
    }
    if let Some(value) = options.version {
        native = native.version(value);
    }
    if let Some(value) = options.copyright {
        native = native.copyright(value);
    }
    if let Some(value) = options.credits {
        native = native.credits(value);
    }
    if let Some(icon) = options.icon {
        native = native.icon(native_image(icon)?);
    }
    Ok(native)
}

fn user_tasks(tasks: Vec<NativeUserTask>) -> std::result::Result<Vec<UserTask>, String> {
    tasks
        .into_iter()
        .map(|task| {
            let mut native = UserTask::new(task.title, task.arguments);
            if let Some(program) = task.program {
                native = native.program(program);
            }
            if let Some(description) = task.description {
                native = native.description(description);
            }
            if let Some(directory) = task.working_directory {
                native = native.working_directory(directory);
            }
            match (task.icon_path, task.icon_index) {
                (Some(path), Some(index)) => native = native.icon(path, index),
                (None, None) => {}
                _ => return Err("a user task icon requires both path and index".to_owned()),
            }
            Ok(native)
        })
        .collect()
}

fn file_icon_size(value: &str) -> std::result::Result<FileIconSize, String> {
    match value {
        "small" => Ok(FileIconSize::Small),
        "normal" => Ok(FileIconSize::Normal),
        "large" => Ok(FileIconSize::Large),
        value => Err(format!("unknown file icon size `{value}`")),
    }
}

fn dock_menu(json: Option<String>) -> std::result::Result<Option<Menu>, String> {
    let Some(json) = json else {
        return Ok(None);
    };
    let mut menus = menu::application_menus(&json)?;
    if menus.len() != 1 {
        return Err("a Dock menu requires exactly one menu definition".to_owned());
    }
    Ok(menus.pop())
}

fn parse_bool(value: Option<String>) -> std::result::Result<bool, String> {
    match value.as_deref() {
        Some("true") => Ok(true),
        Some("false") => Ok(false),
        _ => Err("native window action requires a boolean value".to_owned()),
    }
}

fn parse_shell_action(action: &str, value: String) -> std::result::Result<ShellAction, String> {
    match action {
        "open-external" => Ok(ShellAction::OpenExternal(value)),
        "open-path" => Ok(ShellAction::OpenPath(PathBuf::from(value))),
        "reveal-path" => Ok(ShellAction::RevealPath(PathBuf::from(value))),
        "trash-path" => Ok(ShellAction::TrashPath(PathBuf::from(value))),
        action => Err(format!("unknown native shell action `{action}`")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clipboard_payloads_validate_kind_and_image_format() {
        let unknown = NativeClipboardItem {
            entries: vec![NativeClipboardEntry {
                kind: "unknown".to_owned(),
                text: None,
                metadata: None,
                format: None,
                data: None,
                paths: None,
                url: None,
            }],
        };
        assert!(
            clipboard_item(unknown)
                .unwrap_err()
                .contains("unknown clipboard")
        );

        let missing_image = NativeClipboardItem {
            entries: vec![NativeClipboardEntry {
                kind: "image".to_owned(),
                text: None,
                metadata: None,
                format: Some("image/unknown".to_owned()),
                data: Some(Buffer::from(Vec::new())),
                paths: None,
                url: None,
            }],
        };
        assert!(
            clipboard_item(missing_image)
                .unwrap_err()
                .contains("MIME type")
        );
    }

    #[test]
    fn window_actions_parse_exact_values() {
        assert!(matches!(
            parse_window_action("set-fullscreen", Some("true".to_owned())),
            Ok(WindowAction::SetFullscreen(true))
        ));
        assert!(parse_window_action("set-fullscreen", Some("yes".to_owned())).is_err());
        assert!(parse_window_action("missing", None).is_err());
    }

    #[test]
    fn shell_actions_preserve_urls_and_paths() {
        assert!(matches!(
            parse_shell_action("open-external", "https://quickgui.dev".to_owned()),
            Ok(ShellAction::OpenExternal(url)) if url == "https://quickgui.dev"
        ));
        assert!(matches!(
            parse_shell_action("trash-path", "/tmp/example".to_owned()),
            Ok(ShellAction::TrashPath(path)) if path == PathBuf::from("/tmp/example")
        ));
        assert!(parse_shell_action("missing", String::new()).is_err());
    }
}
