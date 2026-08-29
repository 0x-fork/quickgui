use std::{
    collections::HashMap,
    future::Future,
    path::PathBuf,
    pin::Pin,
    sync::Arc,
    task::{Context, Poll},
};

use napi::{Error, Result, bindgen_prelude::Buffer};
use napi_derive::napi;
use quickgui::{
    ClipboardEntry, ClipboardImage, ClipboardImageFormat, ClipboardItem, ClipboardString, Display,
    Displays, ExternalPaths, KeyboardLayout, ShellResponse, SystemNotification,
    SystemNotificationAction, WindowAppearance, WindowState,
};

use super::{HOST, HostCommand, NativeRuntime, QueuedEvent, ROOT_NODE, SyncReply, with_app_mut};

mod bindings;
mod menu;
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
pub struct NativeWindowState {
    pub display_id: Option<String>,
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    pub scale_factor: f64,
    pub appearance: String,
    pub focused: bool,
    pub visible: bool,
    pub minimized: bool,
    pub maximized: bool,
    pub fullscreen: bool,
    pub occluded: bool,
    pub movable: bool,
    pub resizable: bool,
    pub minimizable: bool,
    pub represented_file: bool,
    pub document_edited: bool,
}

#[napi(object)]
pub struct NativeClipboardEntry {
    pub kind: String,
    pub text: Option<String>,
    pub metadata: Option<String>,
    pub format: Option<String>,
    pub data: Option<Buffer>,
    pub paths: Option<Vec<String>>,
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
}

#[derive(Clone)]
#[napi(object)]
pub struct NativeNotificationOptions {
    pub tag: String,
    pub title: String,
    pub body: String,
    pub actions: Vec<NativeNotificationAction>,
}

pub(super) enum SystemCommand {
    Exit,
    GetDisplays,
    GetKeyboardLayout,
    GetWindowState(u32),
    ReadClipboard,
    WriteClipboard(ClipboardItem),
    ShowNotification(SystemNotification),
    DismissNotification(String),
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
    Minimize,
    Maximize,
    Restore,
    SetFullscreen(bool),
    SetVisible(bool),
    Focus,
    RequestAttention,
    SetRepresentedFile(Option<PathBuf>),
    SetDocumentEdited(bool),
    SetAppearance(Option<WindowAppearance>),
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
    Displays(Vec<NativeDisplay>),
    KeyboardLayout(NativeKeyboardLayout),
    WindowState(NativeWindowState),
    Clipboard(Option<ClipboardItem>),
}

#[derive(Default)]
pub(super) struct SystemObservation {
    displays: Option<Displays>,
    windows: HashMap<u32, WindowState>,
}

pub(super) struct PendingShell {
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
            SystemCommand::Exit => Ok(SystemCommandResult::Boolean(
                self.running_runner_mut()?.exit(),
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
                    WindowAction::Minimize => runner.minimize_window(handle),
                    WindowAction::Maximize => runner.maximize_window(handle),
                    WindowAction::Restore => runner.restore_window(handle),
                    WindowAction::SetFullscreen(fullscreen) => {
                        runner.set_window_fullscreen(handle, fullscreen)
                    }
                    WindowAction::SetVisible(visible) => runner.set_window_visible(handle, visible),
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

impl From<WindowState> for NativeWindowState {
    fn from(state: WindowState) -> Self {
        let bounds = state.bounds.bounds();
        Self {
            display_id: state.display_id.map(|id| id.get().to_string()),
            x: f64::from(bounds.x),
            y: f64::from(bounds.y),
            width: f64::from(bounds.width),
            height: f64::from(bounds.height),
            scale_factor: f64::from(state.scale_factor),
            appearance: match state.appearance {
                WindowAppearance::Light => "light".to_owned(),
                WindowAppearance::Dark => "dark".to_owned(),
            },
            focused: state.focused,
            visible: state.visible,
            minimized: state.minimized,
            maximized: state.maximized,
            fullscreen: state.fullscreen,
            occluded: state.occluded,
            movable: state.movable,
            resizable: state.resizable,
            minimizable: state.minimizable,
            represented_file: state.represented_file,
            document_edited: state.document_edited,
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
            },
            ClipboardEntry::Image(value) => NativeClipboardEntry {
                kind: "image".to_owned(),
                text: None,
                metadata: None,
                format: Some(value.format().mime_type().to_owned()),
                data: Some(Buffer::from(value.bytes().to_vec())),
                paths: None,
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

fn parse_window_action(
    action: &str,
    value: Option<String>,
) -> std::result::Result<WindowAction, String> {
    match action {
        "set-title" => Ok(WindowAction::SetTitle(
            value.ok_or_else(|| "set-title requires a value".to_owned())?,
        )),
        "minimize" => Ok(WindowAction::Minimize),
        "maximize" => Ok(WindowAction::Maximize),
        "restore" => Ok(WindowAction::Restore),
        "set-fullscreen" => Ok(WindowAction::SetFullscreen(parse_bool(value)?)),
        "set-visible" => Ok(WindowAction::SetVisible(parse_bool(value)?)),
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
        action => Err(format!("unknown native window action `{action}`")),
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

fn system_notification(options: NativeNotificationOptions) -> SystemNotification {
    let mut notification = SystemNotification::new(options.tag, options.title, options.body);
    for action in options.actions {
        notification = notification.action(SystemNotificationAction::new(action.id, action.label));
    }
    notification
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
