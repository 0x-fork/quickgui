use super::*;

pub(super) fn thermal_state_name(state: quickgui::ThermalState) -> &'static str {
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

pub(super) fn parse_permission_kind(value: &str) -> Result<PermissionKind> {
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

pub(super) fn permission_status_name(status: PermissionStatus) -> String {
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
    pub(super) kind: PermissionKind,
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
            vibrancy: state.macos_vibrancy.map(macos_vibrancy_name),
            visual_effect_state: macos_visual_effect_state_name(state.macos_visual_effect_state)
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

pub(super) fn native_clipboard_item(item: ClipboardItem) -> NativeClipboardItem {
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

pub(super) fn clipboard_item(
    item: NativeClipboardItem,
) -> std::result::Result<ClipboardItem, String> {
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

pub(super) fn direct_command(app: u32, command: SystemCommand) -> Result<SystemCommandResult> {
    with_app_mut(app, |runtime| runtime.execute_system_command(command))
}

pub(super) fn hosted_command(
    app: u32,
    command: SystemCommand,
) -> Result<Arc<HostReply<SystemCommandResult>>> {
    let reply = Arc::new(HostReply::new(app));
    HOST.enqueue(HostCommand::System {
        app,
        command,
        reply: Arc::clone(&reply),
    })
    .map_err(Error::from_reason)?;
    Ok(reply)
}

pub(super) fn enqueue_hosted_mutation(app: u32, command: SystemCommand) -> Result<()> {
    HOST.enqueue(HostCommand::Mutation { app, command })
        .map_err(Error::from_reason)
}

macro_rules! hosted_system_task {
    ($name:ident, $output:ty, $expect:ident) => {
        #[doc(hidden)]
        pub struct $name {
            pub(super) reply: Arc<HostReply<SystemCommandResult>>,
        }

        impl Task for $name {
            type Output = $output;
            type JsValue = $output;

            fn compute(&mut self) -> Result<Self::Output> {
                let result = self.reply.wait().map_err(Error::from_reason)?;
                $expect(result)
            }

            fn resolve(&mut self, _env: Env, output: Self::Output) -> Result<Self::JsValue> {
                Ok(output)
            }
        }
    };
}

hosted_system_task!(HostedUnitCommandTask, (), expect_unit);
hosted_system_task!(HostedBooleanCommandTask, bool, expect_boolean);
hosted_system_task!(
    HostedAppInfoCommandTask,
    Option<NativeAppInfo>,
    expect_app_info
);
hosted_system_task!(
    HostedAppPathsCommandTask,
    Option<NativeAppPaths>,
    expect_app_paths
);
hosted_system_task!(
    HostedSystemInfoCommandTask,
    NativeSystemInfo,
    expect_system_info
);
hosted_system_task!(
    HostedWindowRegistryCommandTask,
    NativeWindowRegistry,
    expect_window_registry
);
hosted_system_task!(HostedPointCommandTask, NativePoint, expect_point);
hosted_system_task!(
    HostedDesktopIntegrationSupportCommandTask,
    NativeDesktopIntegrationSupport,
    expect_desktop_integration_support
);
hosted_system_task!(
    HostedSystemPreferencesCommandTask,
    NativeSystemPreferences,
    expect_system_preferences
);
hosted_system_task!(
    HostedDisplaysCommandTask,
    Vec<NativeDisplay>,
    expect_displays
);
hosted_system_task!(
    HostedKeyboardLayoutCommandTask,
    NativeKeyboardLayout,
    expect_keyboard_layout
);
hosted_system_task!(
    HostedWindowStateCommandTask,
    NativeWindowState,
    expect_window_state
);
hosted_system_task!(
    HostedClipboardCommandTask,
    Option<NativeClipboardItem>,
    expect_clipboard
);

macro_rules! hosted_task_constructor {
    ($name:ident, $task:ident) => {
        pub(super) fn $name(app: u32, command: SystemCommand) -> Result<AsyncTask<$task>> {
            Ok(AsyncTask::new($task {
                reply: hosted_command(app, command)?,
            }))
        }
    };
}

hosted_task_constructor!(hosted_unit_command, HostedUnitCommandTask);
hosted_task_constructor!(hosted_boolean_command, HostedBooleanCommandTask);
hosted_task_constructor!(hosted_app_info_command, HostedAppInfoCommandTask);
hosted_task_constructor!(hosted_app_paths_command, HostedAppPathsCommandTask);
hosted_task_constructor!(hosted_system_info_command, HostedSystemInfoCommandTask);
hosted_task_constructor!(
    hosted_window_registry_command,
    HostedWindowRegistryCommandTask
);
hosted_task_constructor!(hosted_point_command, HostedPointCommandTask);
hosted_task_constructor!(
    hosted_desktop_integration_support_command,
    HostedDesktopIntegrationSupportCommandTask
);
hosted_task_constructor!(
    hosted_system_preferences_command,
    HostedSystemPreferencesCommandTask
);
hosted_task_constructor!(hosted_displays_command, HostedDisplaysCommandTask);
hosted_task_constructor!(
    hosted_keyboard_layout_command,
    HostedKeyboardLayoutCommandTask
);
hosted_task_constructor!(hosted_window_state_command, HostedWindowStateCommandTask);
hosted_task_constructor!(hosted_clipboard_command, HostedClipboardCommandTask);

pub(super) fn expect_displays(result: SystemCommandResult) -> Result<Vec<NativeDisplay>> {
    match result {
        SystemCommandResult::Displays(displays) => Ok(displays),
        _ => Err(Error::from_reason(
            "native system command returned the wrong result",
        )),
    }
}

pub(super) fn expect_keyboard_layout(result: SystemCommandResult) -> Result<NativeKeyboardLayout> {
    match result {
        SystemCommandResult::KeyboardLayout(layout) => Ok(layout),
        _ => Err(Error::from_reason(
            "native system command returned the wrong result",
        )),
    }
}

pub(super) fn expect_window_state(result: SystemCommandResult) -> Result<NativeWindowState> {
    match result {
        SystemCommandResult::WindowState(state) => Ok(state),
        _ => Err(Error::from_reason(
            "native system command returned the wrong result",
        )),
    }
}

pub(super) fn expect_clipboard(result: SystemCommandResult) -> Result<Option<NativeClipboardItem>> {
    match result {
        SystemCommandResult::Clipboard(item) => Ok(item.map(native_clipboard_item)),
        _ => Err(Error::from_reason(
            "native system command returned the wrong result",
        )),
    }
}

pub(super) fn expect_unit(result: SystemCommandResult) -> Result<()> {
    match result {
        SystemCommandResult::Unit => Ok(()),
        _ => Err(Error::from_reason(
            "native system command returned the wrong result",
        )),
    }
}

pub(super) fn expect_boolean(result: SystemCommandResult) -> Result<bool> {
    match result {
        SystemCommandResult::Boolean(value) => Ok(value),
        _ => Err(Error::from_reason(
            "native system command returned the wrong result",
        )),
    }
}

pub(super) fn expect_app_info(result: SystemCommandResult) -> Result<Option<NativeAppInfo>> {
    match result {
        SystemCommandResult::AppInfo(value) => Ok(value),
        _ => Err(Error::from_reason(
            "native system command returned the wrong result",
        )),
    }
}

pub(super) fn expect_app_paths(result: SystemCommandResult) -> Result<Option<NativeAppPaths>> {
    match result {
        SystemCommandResult::AppPaths(value) => Ok(value),
        _ => Err(Error::from_reason(
            "native system command returned the wrong result",
        )),
    }
}

pub(super) fn expect_system_info(result: SystemCommandResult) -> Result<NativeSystemInfo> {
    match result {
        SystemCommandResult::SystemInfo(value) => Ok(value),
        _ => Err(Error::from_reason(
            "native system command returned the wrong result",
        )),
    }
}

pub(super) fn expect_window_registry(result: SystemCommandResult) -> Result<NativeWindowRegistry> {
    match result {
        SystemCommandResult::WindowRegistry(value) => Ok(value),
        _ => Err(Error::from_reason(
            "native system command returned the wrong result",
        )),
    }
}

pub(super) fn expect_point(result: SystemCommandResult) -> Result<NativePoint> {
    match result {
        SystemCommandResult::Point(value) => Ok(value),
        _ => Err(Error::from_reason(
            "native system command returned the wrong result",
        )),
    }
}

pub(super) fn expect_desktop_integration_support(
    result: SystemCommandResult,
) -> Result<NativeDesktopIntegrationSupport> {
    match result {
        SystemCommandResult::DesktopIntegrationSupport(value) => Ok(value),
        _ => Err(Error::from_reason(
            "native system command returned the wrong result",
        )),
    }
}

pub(super) fn expect_system_preferences(
    result: SystemCommandResult,
) -> Result<NativeSystemPreferences> {
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

pub(super) fn parse_window_action(
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
        "set-vibrancy" => Ok(WindowAction::SetMacOsVibrancy(
            value
                .as_deref()
                .filter(|value| !value.is_empty())
                .map(parse_macos_vibrancy)
                .transpose()?,
        )),
        "set-visual-effect-state" => Ok(WindowAction::SetMacOsVisualEffectState(
            parse_macos_visual_effect_state(
                value
                    .as_deref()
                    .ok_or_else(|| "set-visual-effect-state requires a value".to_owned())?,
            )?,
        )),
        action => Err(format!("unknown native window action `{action}`")),
    }
}

fn macos_vibrancy_name(vibrancy: MacOsVibrancy) -> String {
    match vibrancy {
        MacOsVibrancy::AppearanceBased => "appearance-based",
        MacOsVibrancy::Titlebar => "titlebar",
        MacOsVibrancy::Selection => "selection",
        MacOsVibrancy::Menu => "menu",
        MacOsVibrancy::Popover => "popover",
        MacOsVibrancy::Sidebar => "sidebar",
        MacOsVibrancy::Header => "header",
        MacOsVibrancy::Sheet => "sheet",
        MacOsVibrancy::Window => "window",
        MacOsVibrancy::Hud => "hud",
        MacOsVibrancy::FullscreenUi => "fullscreen-ui",
        MacOsVibrancy::Tooltip => "tooltip",
        MacOsVibrancy::Content => "content",
        MacOsVibrancy::UnderWindow => "under-window",
        MacOsVibrancy::UnderPage => "under-page",
    }
    .to_owned()
}

fn macos_visual_effect_state_name(state: MacOsVisualEffectState) -> &'static str {
    match state {
        MacOsVisualEffectState::FollowWindow => "followWindow",
        MacOsVisualEffectState::Active => "active",
        MacOsVisualEffectState::Inactive => "inactive",
    }
}

pub(super) fn parse_window_image_action(
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

pub(super) fn parse_json_value<T: for<'de> Deserialize<'de>>(
    value: Option<String>,
    action: &str,
) -> std::result::Result<T, String> {
    let value = value.ok_or_else(|| format!("{action} requires a value"))?;
    serde_json::from_str(&value).map_err(|error| format!("invalid {action} value: {error}"))
}

pub(super) fn parse_optional_size(
    value: Option<String>,
) -> std::result::Result<Option<Size>, String> {
    value
        .map(|value| {
            serde_json::from_str::<NativeSizePayload>(&value)
                .map(|value| Size::new(value.width, value.height))
                .map_err(|error| format!("invalid window size: {error}"))
        })
        .transpose()
}

pub(super) fn parse_f32(value: Option<String>, action: &str) -> std::result::Result<f32, String> {
    value
        .ok_or_else(|| format!("{action} requires a value"))?
        .parse::<f32>()
        .map_err(|_| format!("{action} requires a finite number"))
}

pub(super) fn parse_taskbar_progress_state(
    state: &str,
) -> std::result::Result<TaskbarProgressState, String> {
    match state {
        "none" => Ok(TaskbarProgressState::None),
        "normal" => Ok(TaskbarProgressState::Normal),
        "indeterminate" => Ok(TaskbarProgressState::Indeterminate),
        "paused" => Ok(TaskbarProgressState::Paused),
        "error" => Ok(TaskbarProgressState::Error),
        value => Err(format!("unknown taskbar progress state `{value}`")),
    }
}

pub(super) fn parse_global_shortcut_action(
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

pub(super) fn system_notification(
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

pub(super) fn about_panel_options(
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

pub(super) fn user_tasks(tasks: Vec<NativeUserTask>) -> std::result::Result<Vec<UserTask>, String> {
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

pub(super) fn file_icon_size(value: &str) -> std::result::Result<FileIconSize, String> {
    match value {
        "small" => Ok(FileIconSize::Small),
        "normal" => Ok(FileIconSize::Normal),
        "large" => Ok(FileIconSize::Large),
        value => Err(format!("unknown file icon size `{value}`")),
    }
}

pub(super) fn dock_menu(json: Option<String>) -> std::result::Result<Option<Menu>, String> {
    let Some(json) = json else {
        return Ok(None);
    };
    let mut menus = menu::application_menus(&json)?;
    if menus.len() != 1 {
        return Err("a Dock menu requires exactly one menu definition".to_owned());
    }
    Ok(menus.pop())
}

pub(super) fn parse_bool(value: Option<String>) -> std::result::Result<bool, String> {
    match value.as_deref() {
        Some("true") => Ok(true),
        Some("false") => Ok(false),
        _ => Err("native window action requires a boolean value".to_owned()),
    }
}

pub(super) fn parse_shell_action(
    action: &str,
    value: String,
) -> std::result::Result<ShellAction, String> {
    match action {
        "open-external" => Ok(ShellAction::OpenExternal(value)),
        "open-path" => Ok(ShellAction::OpenPath(PathBuf::from(value))),
        "reveal-path" => Ok(ShellAction::RevealPath(PathBuf::from(value))),
        "trash-path" => Ok(ShellAction::TrashPath(PathBuf::from(value))),
        action => Err(format!("unknown native shell action `{action}`")),
    }
}
