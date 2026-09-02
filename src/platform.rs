use std::{
    cell::RefCell,
    fmt,
    future::Future,
    path::{Path, PathBuf},
    pin::Pin,
    rc::Rc,
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
    task::{Context, Poll, Waker},
    time::SystemTime,
};

use thiserror::Error;
#[cfg(target_os = "macos")]
use winit::event_loop::EventLoopProxy;

#[cfg(target_os = "macos")]
use crate::runtime::RuntimeEvent;
use crate::{Image, Menu, WindowHandle};

/// Maximum native platform operations one event callback may queue.
pub const MAX_PLATFORM_REQUESTS_PER_EVENT: usize = 32;
/// Maximum deferred platform operations retained by one application effect cycle.
pub const MAX_PENDING_PLATFORM_REQUESTS: usize = 128;
/// Maximum simultaneously visible native prompts or file panels in one application.
pub const MAX_ACTIVE_PLATFORM_DIALOGS: usize = 32;
/// Maximum UTF-8 bytes retained by one native dialog text field.
pub const MAX_PLATFORM_TEXT_BYTES: usize = 64 * 1024;
/// Maximum UTF-8 bytes retained by one native prompt button label.
pub const MAX_PROMPT_BUTTON_BYTES: usize = 1_024;
/// Maximum buttons accepted by one native prompt.
pub const MAX_PROMPT_BUTTONS: usize = 16;
/// Maximum platform-native bytes accepted for one filesystem path.
pub const MAX_PLATFORM_PATH_BYTES: usize = 16 * 1024;
/// Maximum UTF-8 bytes accepted for one URL shell action.
pub const MAX_PLATFORM_URL_BYTES: usize = 16 * 1024;
/// Maximum paths copied out of one native open panel.
pub const MAX_SELECTED_PATHS: usize = 4_096;
/// Maximum aggregate filesystem bytes copied out of one native open panel.
pub const MAX_SELECTED_PATHS_TOTAL_BYTES: usize = 16 * 1024 * 1024;
/// Maximum named filters accepted by one native file dialog.
pub const MAX_FILE_DIALOG_FILTERS: usize = 64;
/// Maximum extensions retained across all filters in one native file dialog.
pub const MAX_FILE_DIALOG_FILTER_EXTENSIONS: usize = 256;
/// Maximum aggregate UTF-8 bytes retained by file-dialog filter names and extensions.
pub const MAX_FILE_DIALOG_FILTER_BYTES: usize = 64 * 1024;
/// Maximum URLs accepted from one native application-open callback.
pub const MAX_OPEN_URLS: usize = 256;
/// Maximum aggregate UTF-8 bytes retained by one native application-open callback.
pub const MAX_OPEN_URLS_TOTAL_BYTES: usize = 1024 * 1024;
/// Maximum UTF-8 bytes retained by a system-notification identity tag.
pub const MAX_SYSTEM_NOTIFICATION_TAG_BYTES: usize = 1_024;
/// Maximum UTF-8 bytes retained by a system-notification title.
pub const MAX_SYSTEM_NOTIFICATION_TITLE_BYTES: usize = 4 * 1024;
/// Maximum UTF-8 bytes retained by a system-notification body.
pub const MAX_SYSTEM_NOTIFICATION_BODY_BYTES: usize = 64 * 1024;
/// Maximum action buttons accepted by one system notification.
pub const MAX_SYSTEM_NOTIFICATION_ACTIONS: usize = 16;
/// Maximum UTF-8 bytes retained by one notification action identifier or label.
pub const MAX_SYSTEM_NOTIFICATION_ACTION_BYTES: usize = 1_024;
/// Maximum media attachments accepted by one system notification.
pub const MAX_SYSTEM_NOTIFICATION_ATTACHMENTS: usize = 8;
/// Maximum UTF-8 bytes retained by an attachment identifier, reply placeholder, or sound name.
pub const MAX_SYSTEM_NOTIFICATION_OPTION_BYTES: usize = 1_024;
/// Maximum UTF-8 bytes copied out of one native inline-reply response.
pub const MAX_SYSTEM_NOTIFICATION_REPLY_BYTES: usize = 64 * 1024;
/// Maximum icon bytes copied into a desktop notification portal request.
pub const MAX_SYSTEM_NOTIFICATION_ICON_BYTES: usize = 16 * 1024 * 1024;
/// Maximum distinct native notification action sets retained by one application.
pub const MAX_SYSTEM_NOTIFICATION_CATEGORIES: usize = 64;
/// Maximum distinct notification tags retained while native authorization is pending.
pub const MAX_PENDING_SYSTEM_NOTIFICATIONS: usize = 64;
/// Maximum permission futures coalesced behind one native notification authorization operation.
pub const MAX_PENDING_NOTIFICATION_PERMISSION_REQUESTS: usize = 64;
/// Maximum UTF-8 bytes accepted for a macOS Dock badge label.
pub const MAX_DOCK_BADGE_BYTES: usize = 1_024;
/// Maximum UTF-8 bytes retained by one About-panel field.
pub const MAX_ABOUT_PANEL_TEXT_BYTES: usize = 64 * 1_024;
/// Maximum UTF-8 bytes retained by a taskbar-overlay accessibility description.
pub const MAX_TASKBAR_OVERLAY_DESCRIPTION_BYTES: usize = 4 * 1_024;
/// Maximum Windows Jump List user tasks accepted in one replacement.
pub const MAX_USER_TASKS: usize = 32;
/// Maximum UTF-8 bytes retained by one user-task text field.
pub const MAX_USER_TASK_TEXT_BYTES: usize = 16 * 1_024;

// Apple reserves these identifiers for activating or dismissing the notification itself. Letting
// an application reuse them would make a response ambiguous at the cross-platform API boundary.
const SYSTEM_NOTIFICATION_DEFAULT_ACTION_ID: &str =
    "com.apple.UNNotificationDefaultActionIdentifier";
const SYSTEM_NOTIFICATION_DISMISS_ACTION_ID: &str =
    "com.apple.UNNotificationDismissActionIdentifier";

/// A platform-service request could not be accepted or completed.
#[derive(Clone, Debug, Eq, Error, PartialEq)]
pub enum PlatformError {
    #[error("this context is not attached to a native window")]
    Unavailable,
    #[error(
        "one event cannot queue more than {MAX_PLATFORM_REQUESTS_PER_EVENT} platform operations"
    )]
    QueueFull,
    #[error(
        "one effect cycle cannot retain more than {MAX_PENDING_PLATFORM_REQUESTS} platform operations"
    )]
    PendingQueueFull,
    #[error("the application or a related window already owns an active native dialog")]
    DialogBusy,
    #[error("the application already owns {MAX_ACTIVE_PLATFORM_DIALOGS} active native dialogs")]
    TooManyDialogs,
    #[error("this platform service is not implemented on the current operating system")]
    Unsupported,
    #[error("the native menu declaration is invalid or exceeds its public bounds")]
    InvalidMenu,
    #[error("a native popup-menu position must be finite and within the supported desktop range")]
    InvalidMenuPosition,
    #[error("a native prompt message cannot be empty")]
    EmptyPromptMessage,
    #[error(
        "native dialog text must be NUL-free and at most {MAX_PLATFORM_TEXT_BYTES} UTF-8 bytes"
    )]
    InvalidText,
    #[error("a native prompt requires 1 to {MAX_PROMPT_BUTTONS} valid buttons")]
    InvalidButtons,
    #[error(
        "native prompt button labels must be nonempty, NUL-free, and at most {MAX_PROMPT_BUTTON_BYTES} UTF-8 bytes"
    )]
    InvalidButton,
    #[error(
        "a path must be nonempty, NUL-free, and at most {MAX_PLATFORM_PATH_BYTES} platform bytes"
    )]
    InvalidPath,
    #[error("a URL must be nonempty, NUL-free, and at most {MAX_PLATFORM_URL_BYTES} UTF-8 bytes")]
    InvalidUrl,
    #[error(
        "a system-notification tag must be nonempty, NUL-free, and at most {MAX_SYSTEM_NOTIFICATION_TAG_BYTES} UTF-8 bytes"
    )]
    InvalidNotificationTag,
    #[error("system-notification text is invalid or exceeds the title/body byte limits")]
    InvalidNotificationText,
    #[error(
        "a system notification cannot contain more than {MAX_SYSTEM_NOTIFICATION_ACTIONS} actions"
    )]
    InvalidNotificationActions,
    #[error(
        "notification action ids must be unique and non-reserved; ids and labels must be nonempty, NUL-free, and at most {MAX_SYSTEM_NOTIFICATION_ACTION_BYTES} UTF-8 bytes"
    )]
    InvalidNotificationAction,
    #[error("system-notification media, sound, schedule, or reply options are invalid")]
    InvalidNotificationOptions,
    #[error("a Dock badge must be NUL-free and at most {MAX_DOCK_BADGE_BYTES} UTF-8 bytes")]
    InvalidDockBadge,
    #[error("About-panel options contain NULs or exceed their public text bounds")]
    InvalidAboutPanelOptions,
    #[error("a user-task declaration is invalid or exceeds its public bounds")]
    InvalidUserTasks,
    #[error("an open panel must allow files, directories, or both")]
    InvalidPathSelection,
    #[error("native file-dialog filters are invalid or exceed their count or byte limits")]
    InvalidFileDialogFilter,
    #[error("the native path selection exceeded QuickGUI's bounded result size")]
    SelectionTooLarge,
    #[error("native platform operation failed: {0}")]
    Platform(Arc<str>),
}

/// Native prompt severity, matching the platform's informational, warning, and critical styles.
/// How the application appears to the operating system's window and application switchers.
///
/// This is AppKit's `NSApplicationActivationPolicy`. Other platforms retain the requested value
/// and report [`PlatformError::Unsupported`].
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub enum ActivationPolicy {
    /// An ordinary application with a Dock tile and a menu bar.
    #[default]
    Regular,
    /// A background application that can still show windows but owns no Dock tile.
    Accessory,
    /// A background application that cannot be activated at all.
    Prohibited,
}

/// Urgency of a request for the user's attention.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum DockAttention {
    /// Bounce until the application is activated or the request is cancelled.
    Critical,
    /// Bounce once.
    Informational,
}

/// Identifier for one in-flight [`DockAttention`] request.
///
/// Pass it to `cancel_dock_attention` to stop a critical bounce early. `0` is never a valid
/// request identifier.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct DockAttentionRequest(i64);

impl DockAttentionRequest {
    pub(crate) const fn new(value: i64) -> Self {
        Self(value)
    }

    pub const fn get(self) -> i64 {
        self.0
    }
}

/// Whether the running process can relocate itself into `/Applications`.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub struct ApplicationsFolderSupport {
    /// Whether the platform implements the move at all.
    pub supported: bool,
    /// Whether the executable is already inside an `/Applications` directory.
    pub already_installed: bool,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum PromptLevel {
    Info,
    Warning,
    Critical,
}

/// URLs supplied by the operating system when launching or reopening the application.
///
/// The collection is shared across callback clones and is always bounded by
/// [`MAX_OPEN_URLS`] and [`MAX_OPEN_URLS_TOTAL_BYTES`]. File opens are represented by ordinary
/// `file:` URLs, matching AppKit's modern `application:openURLs:` contract.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OpenUrls {
    urls: Arc<[Arc<str>]>,
}

impl OpenUrls {
    pub(crate) fn from_bounded(urls: Vec<Arc<str>>) -> Self {
        debug_assert!(urls.len() <= MAX_OPEN_URLS);
        debug_assert!(urls.iter().map(|url| url.len()).sum::<usize>() <= MAX_OPEN_URLS_TOTAL_BYTES);
        Self {
            urls: Arc::from(urls),
        }
    }

    pub fn as_slice(&self) -> &[Arc<str>] {
        &self.urls
    }

    pub fn iter(&self) -> impl ExactSizeIterator<Item = &str> {
        self.urls.iter().map(AsRef::as_ref)
    }

    pub fn len(&self) -> usize {
        self.urls.len()
    }

    pub fn is_empty(&self) -> bool {
        self.urls.is_empty()
    }
}

/// One button exposed by an operating-system notification.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct SystemNotificationAction {
    pub id: Arc<str>,
    pub label: Arc<str>,
    pub kind: SystemNotificationActionKind,
}

/// Interaction requested for a notification action.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum SystemNotificationActionKind {
    Button,
    TextInput { placeholder: Option<Arc<str>> },
}

impl SystemNotificationAction {
    pub fn new(id: impl Into<Arc<str>>, label: impl Into<Arc<str>>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            kind: SystemNotificationActionKind::Button,
        }
    }

    /// Request an inline text response from the operating system.
    pub fn text_input(mut self, placeholder: impl Into<Arc<str>>) -> Self {
        self.kind = SystemNotificationActionKind::TextInput {
            placeholder: Some(placeholder.into()),
        };
        self
    }

    pub fn text_input_without_placeholder(mut self) -> Self {
        self.kind = SystemNotificationActionKind::TextInput { placeholder: None };
        self
    }
}

/// Audio policy for one operating-system notification.
#[derive(Clone, Debug, Default, Eq, Hash, PartialEq)]
pub enum SystemNotificationSound {
    #[default]
    Default,
    Silent,
    /// A platform-recognized bundled or system sound name.
    Named(Arc<str>),
}

/// One local file attached to a rich operating-system notification.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct SystemNotificationAttachment {
    pub id: Arc<str>,
    pub path: PathBuf,
}

impl SystemNotificationAttachment {
    pub fn new(id: impl Into<Arc<str>>, path: impl Into<PathBuf>) -> Self {
        Self {
            id: id.into(),
            path: path.into(),
        }
    }
}

/// A notification posted to the operating system rather than rendered in a QuickGUI window.
///
/// Posting another notification with the same `tag` replaces the earlier request where the
/// platform supports replacement.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SystemNotification {
    pub tag: Arc<str>,
    pub title: Arc<str>,
    pub subtitle: Option<Arc<str>>,
    pub body: Arc<str>,
    pub actions: Vec<SystemNotificationAction>,
    pub sound: SystemNotificationSound,
    /// Local image used as the notification icon where the platform supports an explicit icon.
    pub icon: Option<PathBuf>,
    pub attachments: Vec<SystemNotificationAttachment>,
    /// Absolute delivery time. `None` posts immediately.
    pub delivery_at: Option<SystemTime>,
}

impl SystemNotification {
    pub fn new(
        tag: impl Into<Arc<str>>,
        title: impl Into<Arc<str>>,
        body: impl Into<Arc<str>>,
    ) -> Self {
        Self {
            tag: tag.into(),
            title: title.into(),
            subtitle: None,
            body: body.into(),
            actions: Vec::new(),
            sound: SystemNotificationSound::Default,
            icon: None,
            attachments: Vec::new(),
            delivery_at: None,
        }
    }

    pub fn subtitle(mut self, subtitle: impl Into<Arc<str>>) -> Self {
        self.subtitle = Some(subtitle.into());
        self
    }

    pub fn action(mut self, action: SystemNotificationAction) -> Self {
        self.actions.push(action);
        self
    }

    pub fn sound(mut self, sound: SystemNotificationSound) -> Self {
        self.sound = sound;
        self
    }

    pub fn icon(mut self, path: impl Into<PathBuf>) -> Self {
        self.icon = Some(path.into());
        self
    }

    pub fn attachment(mut self, attachment: SystemNotificationAttachment) -> Self {
        self.attachments.push(attachment);
        self
    }

    pub fn deliver_at(mut self, delivery_at: SystemTime) -> Self {
        self.delivery_at = Some(delivery_at);
        self
    }
}

/// The user's activation of an operating-system notification.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SystemNotificationResponse {
    pub tag: Arc<str>,
    /// The selected action id, or `None` when the notification body was activated.
    pub action_id: Option<Arc<str>>,
    /// Bounded inline text supplied for a text-input action.
    pub reply: Option<Arc<str>>,
}

/// Current operating-system authorization for application notifications.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum NotificationPermissionStatus {
    NotDetermined,
    Granted,
    Denied,
    Unsupported,
}

/// Requested native file-icon size.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub enum FileIconSize {
    Small,
    #[default]
    Normal,
    Large,
}

impl FileIconSize {
    pub(crate) const fn pixels(self) -> u32 {
        match self {
            Self::Small => 16,
            Self::Normal => 32,
            Self::Large => 48,
        }
    }
}

/// Metadata shown by the operating system's standard About UI.
///
/// Omitted fields fall back to package metadata on platforms that provide such defaults.
#[derive(Clone, Debug, Default)]
pub struct AboutPanelOptions {
    pub application_name: Option<Arc<str>>,
    pub application_version: Option<Arc<str>>,
    pub version: Option<Arc<str>>,
    pub copyright: Option<Arc<str>>,
    pub credits: Option<Arc<str>>,
    pub icon: Option<Image>,
}

impl AboutPanelOptions {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn application_name(mut self, value: impl Into<Arc<str>>) -> Self {
        self.application_name = Some(value.into());
        self
    }

    pub fn application_version(mut self, value: impl Into<Arc<str>>) -> Self {
        self.application_version = Some(value.into());
        self
    }

    pub fn version(mut self, value: impl Into<Arc<str>>) -> Self {
        self.version = Some(value.into());
        self
    }

    pub fn copyright(mut self, value: impl Into<Arc<str>>) -> Self {
        self.copyright = Some(value.into());
        self
    }

    pub fn credits(mut self, value: impl Into<Arc<str>>) -> Self {
        self.credits = Some(value.into());
        self
    }

    pub fn icon(mut self, icon: Image) -> Self {
        self.icon = Some(icon);
        self
    }
}

/// One command exposed in the Windows taskbar Jump List.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UserTask {
    pub title: Arc<str>,
    pub program: Option<PathBuf>,
    pub arguments: Arc<str>,
    pub description: Option<Arc<str>>,
    pub working_directory: Option<PathBuf>,
    pub icon_path: Option<PathBuf>,
    pub icon_index: i32,
}

impl UserTask {
    /// Create a task that launches the current executable with `arguments`.
    pub fn new(title: impl Into<Arc<str>>, arguments: impl Into<Arc<str>>) -> Self {
        Self {
            title: title.into(),
            program: None,
            arguments: arguments.into(),
            description: None,
            working_directory: None,
            icon_path: None,
            icon_index: 0,
        }
    }

    pub fn program(mut self, path: impl Into<PathBuf>) -> Self {
        self.program = Some(path.into());
        self
    }

    pub fn description(mut self, value: impl Into<Arc<str>>) -> Self {
        self.description = Some(value.into());
        self
    }

    pub fn working_directory(mut self, path: impl Into<PathBuf>) -> Self {
        self.working_directory = Some(path.into());
        self
    }

    pub fn icon(mut self, path: impl Into<PathBuf>, index: i32) -> Self {
        self.icon_path = Some(path.into());
        self.icon_index = index;
        self
    }
}

/// Future-like result of an operating-system file-icon lookup.
pub type FileIconResponse = PlatformResponse<Image>;

/// One semantically classified native prompt button.
///
/// The first `Ok` button keeps the native Return-key equivalent. `Cancel` receives Escape on
/// macOS. `Other` is useful for alternatives such as “Don't Save”.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PromptButton {
    Ok(Arc<str>),
    Cancel(Arc<str>),
    Other(Arc<str>),
}

impl PromptButton {
    pub fn ok(label: impl Into<Arc<str>>) -> Self {
        Self::Ok(label.into())
    }

    pub fn cancel(label: impl Into<Arc<str>>) -> Self {
        Self::Cancel(label.into())
    }

    pub fn new(label: impl Into<Arc<str>>) -> Self {
        Self::Other(label.into())
    }

    pub fn label(&self) -> &str {
        match self {
            Self::Ok(label) | Self::Cancel(label) | Self::Other(label) => label,
        }
    }

    pub fn is_cancel(&self) -> bool {
        matches!(self, Self::Cancel(_))
    }
}

impl From<&str> for PromptButton {
    fn from(label: &str) -> Self {
        Self::new(label)
    }
}

impl From<String> for PromptButton {
    fn from(label: String) -> Self {
        Self::new(label)
    }
}

/// One named native file-dialog filter. Extensions omit the leading dot; `*` matches all files.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FileDialogFilter {
    pub name: Arc<str>,
    pub extensions: Vec<Arc<str>>,
}

impl FileDialogFilter {
    pub fn new<I, S>(name: impl Into<Arc<str>>, extensions: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<Arc<str>>,
    {
        Self {
            name: name.into(),
            extensions: extensions.into_iter().map(Into::into).collect(),
        }
    }
}

/// Options for a native open panel.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PathPromptOptions {
    pub files: bool,
    pub directories: bool,
    pub multiple: bool,
    pub title: Option<Arc<str>>,
    pub prompt: Option<Arc<str>>,
    pub directory: Option<PathBuf>,
    pub suggested_name: Option<Arc<str>>,
    pub filters: Vec<FileDialogFilter>,
    pub shows_hidden_files: bool,
}

impl Default for PathPromptOptions {
    fn default() -> Self {
        Self {
            files: true,
            directories: false,
            multiple: false,
            title: None,
            prompt: None,
            directory: None,
            suggested_name: None,
            filters: Vec::new(),
            shows_hidden_files: false,
        }
    }
}

impl PathPromptOptions {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn files(mut self, files: bool) -> Self {
        self.files = files;
        self
    }

    pub fn directories(mut self, directories: bool) -> Self {
        self.directories = directories;
        self
    }

    pub fn multiple(mut self, multiple: bool) -> Self {
        self.multiple = multiple;
        self
    }

    pub fn title(mut self, title: impl Into<Arc<str>>) -> Self {
        self.title = Some(title.into());
        self
    }

    pub fn prompt(mut self, prompt: impl Into<Arc<str>>) -> Self {
        self.prompt = Some(prompt.into());
        self
    }

    pub fn directory(mut self, directory: impl Into<PathBuf>) -> Self {
        self.directory = Some(directory.into());
        self
    }

    pub fn suggested_name(mut self, suggested_name: impl Into<Arc<str>>) -> Self {
        self.suggested_name = Some(suggested_name.into());
        self
    }

    pub fn filters(mut self, filters: impl IntoIterator<Item = FileDialogFilter>) -> Self {
        self.filters = filters.into_iter().collect();
        self
    }

    pub fn shows_hidden_files(mut self, shows_hidden_files: bool) -> Self {
        self.shows_hidden_files = shows_hidden_files;
        self
    }
}

/// Options for a native save panel.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SavePathOptions {
    pub directory: PathBuf,
    pub title: Option<Arc<str>>,
    pub suggested_name: Option<Arc<str>>,
    pub prompt: Option<Arc<str>>,
    pub filters: Vec<FileDialogFilter>,
    pub shows_hidden_files: bool,
}

impl SavePathOptions {
    pub fn new(directory: impl Into<PathBuf>) -> Self {
        Self {
            directory: directory.into(),
            title: None,
            suggested_name: None,
            prompt: None,
            filters: Vec::new(),
            shows_hidden_files: false,
        }
    }

    pub fn title(mut self, title: impl Into<Arc<str>>) -> Self {
        self.title = Some(title.into());
        self
    }

    pub fn suggested_name(mut self, suggested_name: impl Into<Arc<str>>) -> Self {
        self.suggested_name = Some(suggested_name.into());
        self
    }

    pub fn prompt(mut self, prompt: impl Into<Arc<str>>) -> Self {
        self.prompt = Some(prompt.into());
        self
    }

    pub fn filters(mut self, filters: impl IntoIterator<Item = FileDialogFilter>) -> Self {
        self.filters = filters.into_iter().collect();
        self
    }

    pub fn shows_hidden_files(mut self, shows_hidden_files: bool) -> Self {
        self.shows_hidden_files = shows_hidden_files;
        self
    }
}

struct PlatformResponseState<T> {
    result: RefCell<Option<Result<T, PlatformError>>>,
    waker: RefCell<Option<Waker>>,
    receiver_alive: std::cell::Cell<bool>,
    #[cfg(target_os = "macos")]
    cancellation: RefCell<Option<PlatformCancellation>>,
}

impl<T> Default for PlatformResponseState<T> {
    fn default() -> Self {
        Self {
            result: RefCell::new(None),
            waker: RefCell::new(None),
            receiver_alive: std::cell::Cell::new(true),
            #[cfg(target_os = "macos")]
            cancellation: RefCell::new(None),
        }
    }
}

/// A single-use, main-thread future resolved by a native platform operation.
///
/// It stores no timer and performs no polling while the operating system owns the prompt. Native
/// completion wakes QuickGUI's foreground executor exactly once.
#[must_use = "await the platform response from a foreground task"]
pub struct PlatformResponse<T> {
    state: Rc<PlatformResponseState<T>>,
}

/// Future returned by a native open panel.
pub type PathPromptResponse = PlatformResponse<Option<Vec<PathBuf>>>;
/// Future returned by a native save panel.
pub type SavePathResponse = PlatformResponse<Option<PathBuf>>;
/// Future returned by a notification authorization status query or explicit request.
pub type NotificationPermissionResponse = PlatformResponse<NotificationPermissionStatus>;
/// Future returned after the operating system accepts or rejects one shell integration request.
pub type ShellResponse = PlatformResponse<()>;

impl<T> Future for PlatformResponse<T> {
    type Output = Result<T, PlatformError>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        if let Some(result) = self.state.result.borrow_mut().take() {
            self.state.waker.borrow_mut().take();
            return Poll::Ready(result);
        }
        let mut waker = self.state.waker.borrow_mut();
        if waker
            .as_ref()
            .is_none_or(|current| !current.will_wake(cx.waker()))
        {
            *waker = Some(cx.waker().clone());
        }
        Poll::Pending
    }
}

impl<T> Drop for PlatformResponse<T> {
    fn drop(&mut self) {
        self.state.receiver_alive.set(false);
        self.state.waker.borrow_mut().take();
        #[cfg(target_os = "macos")]
        if let Some(cancellation) = self.state.cancellation.borrow_mut().take() {
            let _ = cancellation
                .proxy
                .send_event(RuntimeEvent::PlatformDialogCancelled(
                    cancellation.owner,
                    cancellation.id,
                ));
        }
    }
}

impl<T> fmt::Debug for PlatformResponse<T> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("PlatformResponse")
            .field("ready", &self.state.result.borrow().is_some())
            .finish_non_exhaustive()
    }
}

pub(crate) struct PlatformResponder<T> {
    state: Rc<PlatformResponseState<T>>,
}

#[cfg(target_os = "macos")]
struct PlatformCancellation {
    proxy: EventLoopProxy<RuntimeEvent>,
    owner: Option<WindowHandle>,
    id: PlatformDialogId,
}

impl<T> Clone for PlatformResponder<T> {
    fn clone(&self) -> Self {
        Self {
            state: self.state.clone(),
        }
    }
}

impl<T> PlatformResponder<T> {
    pub(crate) fn is_cancelled(&self) -> bool {
        !self.state.receiver_alive.get()
    }

    pub(crate) fn complete(&self, result: Result<T, PlatformError>) {
        #[cfg(target_os = "macos")]
        self.state.cancellation.borrow_mut().take();
        if self.is_cancelled() {
            return;
        }
        let mut slot = self.state.result.borrow_mut();
        if slot.is_some() {
            return;
        }
        *slot = Some(result);
        drop(slot);
        if let Some(waker) = self.state.waker.borrow_mut().take() {
            waker.wake();
        }
    }

    #[cfg(target_os = "macos")]
    fn bind_cancellation(
        &self,
        proxy: EventLoopProxy<RuntimeEvent>,
        owner: Option<WindowHandle>,
        id: PlatformDialogId,
    ) -> bool {
        if self.is_cancelled() {
            return false;
        }
        *self.state.cancellation.borrow_mut() = Some(PlatformCancellation { proxy, owner, id });
        true
    }
}

impl<T> fmt::Debug for PlatformResponder<T> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("PlatformResponder")
            .finish_non_exhaustive()
    }
}

pub(crate) fn response_channel<T>() -> (PlatformResponder<T>, PlatformResponse<T>) {
    let state = Rc::new(PlatformResponseState::default());
    (
        PlatformResponder {
            state: state.clone(),
        },
        PlatformResponse { state },
    )
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub(crate) struct PlatformDialogId(u64);

impl PlatformDialogId {
    pub(crate) fn next() -> Self {
        static NEXT_DIALOG_ID: AtomicU64 = AtomicU64::new(1);
        Self(NEXT_DIALOG_ID.fetch_add(1, Ordering::Relaxed).max(1))
    }
}

#[derive(Debug)]
pub(crate) enum PlatformRequest {
    Prompt {
        window: Option<WindowHandle>,
        level: PromptLevel,
        message: Arc<str>,
        detail: Option<Arc<str>>,
        buttons: Vec<PromptButton>,
        responder: PlatformResponder<usize>,
    },
    OpenPaths {
        window: Option<WindowHandle>,
        options: PathPromptOptions,
        responder: PlatformResponder<Option<Vec<PathBuf>>>,
    },
    SavePath {
        window: Option<WindowHandle>,
        options: SavePathOptions,
        responder: PlatformResponder<Option<PathBuf>>,
    },
    ShowSystemNotification(SystemNotification),
    DismissSystemNotification(Arc<str>),
    NotificationPermissionStatus {
        responder: PlatformResponder<NotificationPermissionStatus>,
    },
    RequestNotificationPermission {
        responder: PlatformResponder<NotificationPermissionStatus>,
    },
    OpenUrl {
        url: Arc<str>,
        responder: Option<PlatformResponder<()>>,
    },
    OpenPath {
        path: PathBuf,
        responder: Option<PlatformResponder<()>>,
    },
    RevealPath {
        path: PathBuf,
        responder: Option<PlatformResponder<()>>,
    },
    TrashPath {
        path: PathBuf,
        responder: Option<PlatformResponder<()>>,
    },
    SetDockBadge(Option<Arc<str>>),
    SetDockIcon(Option<Image>),
    SetDockMenu(Option<Menu>),
    AddRecentDocument(PathBuf),
    ClearRecentDocuments,
    ShowAboutPanel(AboutPanelOptions),
    GetFileIcon {
        path: PathBuf,
        size: FileIconSize,
        responder: PlatformResponder<Image>,
    },
    SetUserTasks {
        tasks: Vec<UserTask>,
        responder: PlatformResponder<()>,
    },
    SetActivationPolicy {
        policy: ActivationPolicy,
        responder: PlatformResponder<()>,
    },
    ActivateApplication {
        force: bool,
    },
    HideApplication,
    UnhideApplication,
    RequestDockAttention {
        attention: DockAttention,
        responder: PlatformResponder<DockAttentionRequest>,
    },
    CancelDockAttention(DockAttentionRequest),
    SetDockVisible {
        visible: bool,
        responder: PlatformResponder<()>,
    },
    SetSecureKeyboardEntry(bool),
    Beep,
    MoveToApplicationsFolder {
        responder: PlatformResponder<bool>,
    },
}

impl PlatformRequest {
    pub(crate) fn prompt(
        window: WindowHandle,
        level: PromptLevel,
        message: impl Into<Arc<str>>,
        detail: Option<Arc<str>>,
        buttons: &[PromptButton],
    ) -> Result<(Self, PlatformResponse<usize>), PlatformError> {
        Self::prompt_with_owner(Some(window), level, message, detail, buttons)
    }

    pub(crate) fn application_prompt(
        level: PromptLevel,
        message: impl Into<Arc<str>>,
        detail: Option<Arc<str>>,
        buttons: &[PromptButton],
    ) -> Result<(Self, PlatformResponse<usize>), PlatformError> {
        Self::prompt_with_owner(None, level, message, detail, buttons)
    }

    fn prompt_with_owner(
        window: Option<WindowHandle>,
        level: PromptLevel,
        message: impl Into<Arc<str>>,
        detail: Option<Arc<str>>,
        buttons: &[PromptButton],
    ) -> Result<(Self, PlatformResponse<usize>), PlatformError> {
        let message = message.into();
        validate_required_text(&message)?;
        if let Some(detail) = &detail {
            validate_optional_text(detail)?;
        }
        validate_prompt_buttons(buttons)?;
        let (responder, response) = response_channel();
        Ok((
            Self::Prompt {
                window,
                level,
                message,
                detail,
                buttons: buttons.to_vec(),
                responder,
            },
            response,
        ))
    }

    pub(crate) fn open_paths(
        window: WindowHandle,
        options: PathPromptOptions,
    ) -> Result<(Self, PathPromptResponse), PlatformError> {
        Self::open_paths_with_owner(Some(window), options)
    }

    pub(crate) fn application_open_paths(
        options: PathPromptOptions,
    ) -> Result<(Self, PathPromptResponse), PlatformError> {
        Self::open_paths_with_owner(None, options)
    }

    fn open_paths_with_owner(
        window: Option<WindowHandle>,
        options: PathPromptOptions,
    ) -> Result<(Self, PathPromptResponse), PlatformError> {
        validate_path_prompt_options(&options)?;
        let (responder, response) = response_channel();
        Ok((
            Self::OpenPaths {
                window,
                options,
                responder,
            },
            response,
        ))
    }

    pub(crate) fn save_path(
        window: WindowHandle,
        options: SavePathOptions,
    ) -> Result<(Self, PlatformResponse<Option<PathBuf>>), PlatformError> {
        Self::save_path_with_owner(Some(window), options)
    }

    pub(crate) fn application_save_path(
        options: SavePathOptions,
    ) -> Result<(Self, PlatformResponse<Option<PathBuf>>), PlatformError> {
        Self::save_path_with_owner(None, options)
    }

    fn save_path_with_owner(
        window: Option<WindowHandle>,
        options: SavePathOptions,
    ) -> Result<(Self, PlatformResponse<Option<PathBuf>>), PlatformError> {
        validate_save_path_options(&options)?;
        let (responder, response) = response_channel();
        Ok((
            Self::SavePath {
                window,
                options,
                responder,
            },
            response,
        ))
    }

    pub(crate) fn open_url(url: impl Into<Arc<str>>) -> Result<Self, PlatformError> {
        let url = url.into();
        validate_url(&url)?;
        Ok(Self::OpenUrl {
            url,
            responder: None,
        })
    }

    pub(crate) fn open_url_response(
        url: impl Into<Arc<str>>,
    ) -> Result<(Self, ShellResponse), PlatformError> {
        let url = url.into();
        validate_url(&url)?;
        let (responder, response) = response_channel();
        Ok((
            Self::OpenUrl {
                url,
                responder: Some(responder),
            },
            response,
        ))
    }

    pub(crate) fn show_system_notification(
        notification: SystemNotification,
    ) -> Result<Self, PlatformError> {
        let notification = normalize_system_notification(notification)?;
        Ok(Self::ShowSystemNotification(notification))
    }

    pub(crate) fn dismiss_system_notification(
        tag: impl Into<Arc<str>>,
    ) -> Result<Self, PlatformError> {
        let tag = tag.into();
        validate_notification_tag(&tag)?;
        Ok(Self::DismissSystemNotification(tag))
    }

    pub(crate) fn notification_permission_status() -> (Self, NotificationPermissionResponse) {
        let (responder, response) = response_channel();
        (Self::NotificationPermissionStatus { responder }, response)
    }

    pub(crate) fn request_notification_permission() -> (Self, NotificationPermissionResponse) {
        let (responder, response) = response_channel();
        (Self::RequestNotificationPermission { responder }, response)
    }

    pub(crate) fn open_path(path: impl Into<PathBuf>) -> Result<Self, PlatformError> {
        let path = path.into();
        validate_path(&path)?;
        Ok(Self::OpenPath {
            path,
            responder: None,
        })
    }

    pub(crate) fn open_path_response(
        path: impl Into<PathBuf>,
    ) -> Result<(Self, ShellResponse), PlatformError> {
        let path = path.into();
        validate_path(&path)?;
        let (responder, response) = response_channel();
        Ok((
            Self::OpenPath {
                path,
                responder: Some(responder),
            },
            response,
        ))
    }

    pub(crate) fn reveal_path(path: impl Into<PathBuf>) -> Result<Self, PlatformError> {
        let path = path.into();
        validate_path(&path)?;
        Ok(Self::RevealPath {
            path,
            responder: None,
        })
    }

    pub(crate) fn trash_path(path: impl Into<PathBuf>) -> Result<Self, PlatformError> {
        let path = path.into();
        validate_path(&path)?;
        Ok(Self::TrashPath {
            path,
            responder: None,
        })
    }

    pub(crate) fn reveal_path_response(
        path: impl Into<PathBuf>,
    ) -> Result<(Self, ShellResponse), PlatformError> {
        let path = path.into();
        validate_path(&path)?;
        let (responder, response) = response_channel();
        Ok((
            Self::RevealPath {
                path,
                responder: Some(responder),
            },
            response,
        ))
    }

    pub(crate) fn trash_path_response(
        path: impl Into<PathBuf>,
    ) -> Result<(Self, ShellResponse), PlatformError> {
        let path = path.into();
        validate_path(&path)?;
        let (responder, response) = response_channel();
        Ok((
            Self::TrashPath {
                path,
                responder: Some(responder),
            },
            response,
        ))
    }

    pub(crate) fn set_dock_badge(value: Option<Arc<str>>) -> Result<Self, PlatformError> {
        if value
            .as_deref()
            .is_some_and(|value| value.len() > MAX_DOCK_BADGE_BYTES || value.contains('\0'))
        {
            return Err(PlatformError::InvalidDockBadge);
        }
        Ok(Self::SetDockBadge(value))
    }

    pub(crate) fn set_dock_icon(icon: Option<Image>) -> Self {
        Self::SetDockIcon(icon)
    }

    pub(crate) fn set_dock_menu(menu: Option<Menu>) -> Result<Self, PlatformError> {
        if let Some(menu) = &menu {
            crate::menu::validate_menus(std::slice::from_ref(menu))
                .map_err(|_| PlatformError::InvalidMenu)?;
        }
        Ok(Self::SetDockMenu(menu))
    }

    pub(crate) fn add_recent_document(path: impl Into<PathBuf>) -> Result<Self, PlatformError> {
        let path = path.into();
        validate_path(&path)?;
        Ok(Self::AddRecentDocument(path))
    }

    pub(crate) fn show_about_panel(options: AboutPanelOptions) -> Result<Self, PlatformError> {
        validate_about_panel_options(&options)?;
        Ok(Self::ShowAboutPanel(options))
    }

    pub(crate) fn get_file_icon(
        path: impl Into<PathBuf>,
        size: FileIconSize,
    ) -> Result<(Self, FileIconResponse), PlatformError> {
        let path = path.into();
        validate_path(&path)?;
        let (responder, response) = response_channel();
        Ok((
            Self::GetFileIcon {
                path,
                size,
                responder,
            },
            response,
        ))
    }

    pub(crate) fn set_user_tasks(
        tasks: Vec<UserTask>,
    ) -> Result<(Self, ShellResponse), PlatformError> {
        validate_user_tasks(&tasks)?;
        let (responder, response) = response_channel();
        Ok((Self::SetUserTasks { tasks, responder }, response))
    }

    pub(crate) fn window(&self) -> Option<WindowHandle> {
        match self {
            Self::Prompt { window, .. }
            | Self::OpenPaths { window, .. }
            | Self::SavePath { window, .. } => *window,
            Self::ShowSystemNotification(_)
            | Self::DismissSystemNotification(_)
            | Self::NotificationPermissionStatus { .. }
            | Self::RequestNotificationPermission { .. }
            | Self::OpenUrl { .. }
            | Self::OpenPath { .. }
            | Self::RevealPath { .. }
            | Self::TrashPath { .. }
            | Self::SetDockBadge(_)
            | Self::SetDockIcon(_)
            | Self::SetDockMenu(_)
            | Self::AddRecentDocument(_)
            | Self::ClearRecentDocuments
            | Self::ShowAboutPanel(_)
            | Self::GetFileIcon { .. }
            | Self::SetUserTasks { .. }
            | Self::SetActivationPolicy { .. }
            | Self::ActivateApplication { .. }
            | Self::HideApplication
            | Self::UnhideApplication
            | Self::RequestDockAttention { .. }
            | Self::CancelDockAttention(_)
            | Self::SetDockVisible { .. }
            | Self::SetSecureKeyboardEntry(_)
            | Self::Beep
            | Self::MoveToApplicationsFolder { .. } => None,
        }
    }

    pub(crate) fn response_cancelled(&self) -> bool {
        match self {
            Self::Prompt { responder, .. } => responder.is_cancelled(),
            Self::OpenPaths { responder, .. } => responder.is_cancelled(),
            Self::SavePath { responder, .. } => responder.is_cancelled(),
            Self::OpenUrl { responder, .. }
            | Self::OpenPath { responder, .. }
            | Self::RevealPath { responder, .. }
            | Self::TrashPath { responder, .. } => responder
                .as_ref()
                .is_some_and(PlatformResponder::is_cancelled),
            Self::NotificationPermissionStatus { responder }
            | Self::RequestNotificationPermission { responder } => responder.is_cancelled(),
            Self::GetFileIcon { responder, .. } => responder.is_cancelled(),
            Self::SetUserTasks { responder, .. } => responder.is_cancelled(),
            Self::SetActivationPolicy { responder, .. }
            | Self::SetDockVisible { responder, .. } => responder.is_cancelled(),
            Self::RequestDockAttention { responder, .. } => responder.is_cancelled(),
            Self::MoveToApplicationsFolder { responder } => responder.is_cancelled(),
            Self::ShowSystemNotification(_)
            | Self::DismissSystemNotification(_)
            | Self::SetDockBadge(_)
            | Self::SetDockIcon(_)
            | Self::SetDockMenu(_)
            | Self::AddRecentDocument(_)
            | Self::ClearRecentDocuments
            | Self::ActivateApplication { .. }
            | Self::HideApplication
            | Self::UnhideApplication
            | Self::CancelDockAttention(_)
            | Self::SetSecureKeyboardEntry(_)
            | Self::Beep
            | Self::ShowAboutPanel(_) => false,
        }
    }

    #[cfg(target_os = "macos")]
    pub(crate) fn bind_cancellation(
        &self,
        proxy: EventLoopProxy<RuntimeEvent>,
        owner: Option<WindowHandle>,
        id: PlatformDialogId,
    ) -> bool {
        match self {
            Self::Prompt { responder, .. } => responder.bind_cancellation(proxy, owner, id),
            Self::OpenPaths { responder, .. } => responder.bind_cancellation(proxy, owner, id),
            Self::SavePath { responder, .. } => responder.bind_cancellation(proxy, owner, id),
            Self::ShowSystemNotification(_)
            | Self::DismissSystemNotification(_)
            | Self::NotificationPermissionStatus { .. }
            | Self::RequestNotificationPermission { .. }
            | Self::OpenUrl { .. }
            | Self::OpenPath { .. }
            | Self::RevealPath { .. }
            | Self::TrashPath { .. }
            | Self::SetDockBadge(_)
            | Self::SetDockIcon(_)
            | Self::SetDockMenu(_)
            | Self::AddRecentDocument(_)
            | Self::ClearRecentDocuments
            | Self::ShowAboutPanel(_)
            | Self::GetFileIcon { .. }
            | Self::SetUserTasks { .. }
            | Self::SetActivationPolicy { .. }
            | Self::ActivateApplication { .. }
            | Self::HideApplication
            | Self::UnhideApplication
            | Self::RequestDockAttention { .. }
            | Self::CancelDockAttention(_)
            | Self::SetDockVisible { .. }
            | Self::SetSecureKeyboardEntry(_)
            | Self::Beep
            | Self::MoveToApplicationsFolder { .. } => false,
        }
    }

    pub(crate) fn complete_error(self, error: PlatformError) {
        match self {
            Self::Prompt { responder, .. } => responder.complete(Err(error)),
            Self::OpenPaths { responder, .. } => responder.complete(Err(error)),
            Self::SavePath { responder, .. } => responder.complete(Err(error)),
            Self::OpenUrl { responder, .. }
            | Self::OpenPath { responder, .. }
            | Self::RevealPath { responder, .. }
            | Self::TrashPath { responder, .. } => {
                if let Some(responder) = responder {
                    responder.complete(Err(error));
                }
            }
            Self::NotificationPermissionStatus { responder }
            | Self::RequestNotificationPermission { responder } => {
                responder.complete(Err(error));
            }
            Self::GetFileIcon { responder, .. } => responder.complete(Err(error)),
            Self::SetUserTasks { responder, .. } => responder.complete(Err(error)),
            Self::SetActivationPolicy { responder, .. }
            | Self::SetDockVisible { responder, .. } => responder.complete(Err(error)),
            Self::RequestDockAttention { responder, .. } => responder.complete(Err(error)),
            Self::MoveToApplicationsFolder { responder } => responder.complete(Err(error)),
            Self::ShowSystemNotification(_)
            | Self::DismissSystemNotification(_)
            | Self::SetDockBadge(_)
            | Self::SetDockIcon(_)
            | Self::SetDockMenu(_)
            | Self::AddRecentDocument(_)
            | Self::ClearRecentDocuments
            | Self::ActivateApplication { .. }
            | Self::HideApplication
            | Self::UnhideApplication
            | Self::CancelDockAttention(_)
            | Self::SetSecureKeyboardEntry(_)
            | Self::Beep
            | Self::ShowAboutPanel(_) => {}
        }
    }
}

impl PlatformRequest {
    pub(crate) fn set_activation_policy(policy: ActivationPolicy) -> (Self, PlatformResponse<()>) {
        let (responder, response) = response_channel();
        (Self::SetActivationPolicy { policy, responder }, response)
    }

    pub(crate) fn request_dock_attention(
        attention: DockAttention,
    ) -> (Self, PlatformResponse<DockAttentionRequest>) {
        let (responder, response) = response_channel();
        (
            Self::RequestDockAttention {
                attention,
                responder,
            },
            response,
        )
    }

    pub(crate) fn set_dock_visible(visible: bool) -> (Self, PlatformResponse<()>) {
        let (responder, response) = response_channel();
        (Self::SetDockVisible { visible, responder }, response)
    }

    pub(crate) fn move_to_applications_folder() -> (Self, PlatformResponse<bool>) {
        let (responder, response) = response_channel();
        (Self::MoveToApplicationsFolder { responder }, response)
    }
}

/// Whether this process is running from an installed application bundle.
///
/// The answer is a documented heuristic, not a security boundary:
///
/// * **macOS** — the executable path contains a `.app/Contents/MacOS/` component and is not under
///   a Cargo build directory (`target/debug`, `target/release`, or any `target/<triple>/…`).
///   `cargo run` therefore reports `false` even when it produced a bundle-shaped path.
/// * **Windows** — the executable is not under a Cargo build directory.
/// * **Linux and the BSDs** — the executable is not under a Cargo build directory and does not
///   live in the current working directory tree used by `cargo run`.
///
/// It reads `std::env::current_exe` once per call and installs no observer, cache, or timer.
pub fn is_application_packaged() -> bool {
    let Ok(path) = std::env::current_exe() else {
        return false;
    };
    if path_is_cargo_build_output(&path) {
        return false;
    }
    #[cfg(target_os = "macos")]
    {
        let components: Vec<_> = path
            .components()
            .map(|component| component.as_os_str().to_string_lossy().into_owned())
            .collect();
        components.windows(3).any(|window| {
            window[0].ends_with(".app") && window[1] == "Contents" && window[2] == "MacOS"
        })
    }
    #[cfg(not(target_os = "macos"))]
    {
        true
    }
}

fn path_is_cargo_build_output(path: &Path) -> bool {
    let mut components = path.components().peekable();
    while let Some(component) = components.next() {
        if component.as_os_str() != "target" {
            continue;
        }
        let Some(next) = components.peek() else {
            continue;
        };
        let next = next.as_os_str().to_string_lossy();
        // `target/debug`, `target/release`, and `target/<triple>/<profile>` all mark build output.
        if next == "debug" || next == "release" || next.contains('-') {
            return true;
        }
    }
    false
}

fn validate_about_panel_options(options: &AboutPanelOptions) -> Result<(), PlatformError> {
    let fields = [
        options.application_name.as_deref(),
        options.application_version.as_deref(),
        options.version.as_deref(),
        options.copyright.as_deref(),
        options.credits.as_deref(),
    ];
    if fields
        .into_iter()
        .flatten()
        .any(|value| value.len() > MAX_ABOUT_PANEL_TEXT_BYTES || value.as_bytes().contains(&0))
    {
        Err(PlatformError::InvalidAboutPanelOptions)
    } else {
        Ok(())
    }
}

fn validate_user_tasks(tasks: &[UserTask]) -> Result<(), PlatformError> {
    if tasks.len() > MAX_USER_TASKS {
        return Err(PlatformError::InvalidUserTasks);
    }
    for task in tasks {
        let required = [task.title.as_ref()];
        let optional = [
            task.arguments.as_ref(),
            task.description.as_deref().unwrap_or_default(),
        ];
        if required.into_iter().any(|value| {
            value.is_empty()
                || value.len() > MAX_USER_TASK_TEXT_BYTES
                || value.as_bytes().contains(&0)
        }) || optional
            .into_iter()
            .any(|value| value.len() > MAX_USER_TASK_TEXT_BYTES || value.as_bytes().contains(&0))
        {
            return Err(PlatformError::InvalidUserTasks);
        }
        for path in [
            task.program.as_deref(),
            task.working_directory.as_deref(),
            task.icon_path.as_deref(),
        ]
        .into_iter()
        .flatten()
        {
            validate_path(path).map_err(|_| PlatformError::InvalidUserTasks)?;
        }
    }
    Ok(())
}

fn validate_required_text(value: &str) -> Result<(), PlatformError> {
    if value.is_empty() {
        return Err(PlatformError::EmptyPromptMessage);
    }
    validate_optional_text(value)
}

fn validate_optional_text(value: &str) -> Result<(), PlatformError> {
    if value.len() > MAX_PLATFORM_TEXT_BYTES || value.contains('\0') {
        Err(PlatformError::InvalidText)
    } else {
        Ok(())
    }
}

fn validate_prompt_buttons(buttons: &[PromptButton]) -> Result<(), PlatformError> {
    if buttons.is_empty() || buttons.len() > MAX_PROMPT_BUTTONS {
        return Err(PlatformError::InvalidButtons);
    }
    if buttons.iter().any(|button| {
        let label = button.label();
        label.is_empty() || label.len() > MAX_PROMPT_BUTTON_BYTES || label.contains('\0')
    }) {
        return Err(PlatformError::InvalidButton);
    }
    Ok(())
}

fn validate_path_prompt_options(options: &PathPromptOptions) -> Result<(), PlatformError> {
    if !options.files && !options.directories {
        return Err(PlatformError::InvalidPathSelection);
    }
    if let Some(title) = &options.title {
        validate_optional_text(title)?;
    }
    if let Some(prompt) = &options.prompt {
        validate_optional_text(prompt)?;
    }
    if let Some(directory) = &options.directory {
        validate_path(directory)?;
    }
    if let Some(name) = &options.suggested_name {
        validate_optional_text(name)?;
    }
    validate_file_dialog_filters(&options.filters)?;
    Ok(())
}

fn validate_save_path_options(options: &SavePathOptions) -> Result<(), PlatformError> {
    validate_path(&options.directory)?;
    if let Some(title) = &options.title {
        validate_optional_text(title)?;
    }
    if let Some(name) = &options.suggested_name {
        validate_optional_text(name)?;
    }
    if let Some(prompt) = &options.prompt {
        validate_optional_text(prompt)?;
    }
    validate_file_dialog_filters(&options.filters)?;
    Ok(())
}

fn validate_file_dialog_filters(filters: &[FileDialogFilter]) -> Result<(), PlatformError> {
    if filters.len() > MAX_FILE_DIALOG_FILTERS {
        return Err(PlatformError::InvalidFileDialogFilter);
    }
    let mut extension_count = 0_usize;
    let mut total_bytes = 0_usize;
    for filter in filters {
        if filter.name.is_empty() || filter.name.contains('\0') || filter.extensions.is_empty() {
            return Err(PlatformError::InvalidFileDialogFilter);
        }
        total_bytes = total_bytes.saturating_add(filter.name.len());
        extension_count = extension_count.saturating_add(filter.extensions.len());
        for extension in &filter.extensions {
            if extension.is_empty()
                || extension.contains(['\0', '/', '\\'])
                || extension.starts_with('.')
            {
                return Err(PlatformError::InvalidFileDialogFilter);
            }
            total_bytes = total_bytes.saturating_add(extension.len());
        }
    }
    if extension_count > MAX_FILE_DIALOG_FILTER_EXTENSIONS
        || total_bytes > MAX_FILE_DIALOG_FILTER_BYTES
    {
        Err(PlatformError::InvalidFileDialogFilter)
    } else {
        Ok(())
    }
}

fn validate_path(path: &Path) -> Result<(), PlatformError> {
    let bytes = path.as_os_str().as_encoded_bytes();
    if bytes.is_empty() || bytes.len() > MAX_PLATFORM_PATH_BYTES || bytes.contains(&0) {
        Err(PlatformError::InvalidPath)
    } else {
        Ok(())
    }
}

fn validate_url(url: &str) -> Result<(), PlatformError> {
    if url.is_empty() || url.len() > MAX_PLATFORM_URL_BYTES || url.contains('\0') {
        Err(PlatformError::InvalidUrl)
    } else {
        Ok(())
    }
}

fn validate_notification_tag(tag: &str) -> Result<(), PlatformError> {
    if tag.is_empty() || tag.len() > MAX_SYSTEM_NOTIFICATION_TAG_BYTES || tag.contains('\0') {
        Err(PlatformError::InvalidNotificationTag)
    } else {
        Ok(())
    }
}

fn validate_system_notification(notification: &SystemNotification) -> Result<(), PlatformError> {
    validate_notification_tag(&notification.tag)?;
    if notification.title.is_empty()
        || notification.title.len() > MAX_SYSTEM_NOTIFICATION_TITLE_BYTES
        || notification.title.contains('\0')
        || notification.subtitle.as_deref().is_some_and(|subtitle| {
            subtitle.len() > MAX_SYSTEM_NOTIFICATION_TITLE_BYTES || subtitle.contains('\0')
        })
        || notification.body.len() > MAX_SYSTEM_NOTIFICATION_BODY_BYTES
        || notification.body.contains('\0')
    {
        return Err(PlatformError::InvalidNotificationText);
    }
    if notification.actions.len() > MAX_SYSTEM_NOTIFICATION_ACTIONS {
        return Err(PlatformError::InvalidNotificationActions);
    }
    for (index, action) in notification.actions.iter().enumerate() {
        if action.id.is_empty()
            || action.label.is_empty()
            || matches!(
                action.id.as_ref(),
                SYSTEM_NOTIFICATION_DEFAULT_ACTION_ID | SYSTEM_NOTIFICATION_DISMISS_ACTION_ID
            )
            || action.id.len() > MAX_SYSTEM_NOTIFICATION_ACTION_BYTES
            || action.label.len() > MAX_SYSTEM_NOTIFICATION_ACTION_BYTES
            || action.id.contains('\0')
            || action.label.contains('\0')
            || notification.actions[..index]
                .iter()
                .any(|previous| previous.id == action.id)
        {
            return Err(PlatformError::InvalidNotificationAction);
        }
        if let SystemNotificationActionKind::TextInput { placeholder } = &action.kind
            && placeholder.as_deref().is_some_and(|placeholder| {
                placeholder.is_empty()
                    || placeholder.len() > MAX_SYSTEM_NOTIFICATION_OPTION_BYTES
                    || placeholder.contains('\0')
            })
        {
            return Err(PlatformError::InvalidNotificationOptions);
        }
    }
    if notification.attachments.len() > MAX_SYSTEM_NOTIFICATION_ATTACHMENTS {
        return Err(PlatformError::InvalidNotificationOptions);
    }
    if let Some(icon) = notification.icon.as_deref() {
        validate_path(icon).map_err(|_| PlatformError::InvalidNotificationOptions)?;
    }
    for (index, attachment) in notification.attachments.iter().enumerate() {
        if attachment.id.is_empty()
            || attachment.id.len() > MAX_SYSTEM_NOTIFICATION_OPTION_BYTES
            || attachment.id.contains('\0')
            || notification.attachments[..index]
                .iter()
                .any(|previous| previous.id == attachment.id)
            || validate_path(&attachment.path).is_err()
        {
            return Err(PlatformError::InvalidNotificationOptions);
        }
    }
    if let SystemNotificationSound::Named(name) = &notification.sound
        && (name.is_empty()
            || name.len() > MAX_SYSTEM_NOTIFICATION_OPTION_BYTES
            || name.contains('\0'))
    {
        return Err(PlatformError::InvalidNotificationOptions);
    }
    Ok(())
}

fn normalize_system_notification(
    mut notification: SystemNotification,
) -> Result<SystemNotification, PlatformError> {
    validate_system_notification(&notification)?;
    notification.actions.shrink_to_fit();
    notification.attachments.shrink_to_fit();
    Ok(notification)
}

#[cfg(test)]
mod tests {
    use std::{
        future::Future,
        pin::pin,
        sync::{
            Arc,
            atomic::{AtomicUsize, Ordering},
        },
        task::{Context, Wake, Waker},
    };

    use super::*;

    struct WakeCounter(AtomicUsize);

    impl Wake for WakeCounter {
        fn wake(self: Arc<Self>) {
            self.0.fetch_add(1, Ordering::Relaxed);
        }
    }

    #[test]
    fn native_response_is_single_shot_and_wakes_once() {
        let (responder, response) = response_channel::<usize>();
        let counter = Arc::new(WakeCounter(AtomicUsize::new(0)));
        let waker = Waker::from(counter.clone());
        let mut context = Context::from_waker(&waker);
        let mut response = pin!(response);

        assert!(response.as_mut().poll(&mut context).is_pending());
        responder.complete(Ok(7));
        responder.complete(Ok(8));
        assert_eq!(counter.0.load(Ordering::Relaxed), 1);
        assert_eq!(response.as_mut().poll(&mut context), Poll::Ready(Ok(7)));
    }

    #[test]
    fn dropping_a_response_cancels_unstarted_native_work() {
        let (responder, response) = response_channel::<usize>();
        assert!(!responder.is_cancelled());
        drop(response);
        assert!(responder.is_cancelled());
        responder.complete(Ok(1));
        assert!(responder.state.result.borrow().is_none());
    }

    #[test]
    fn prompt_validation_happens_before_retaining_native_work() {
        let window = WindowHandle::next();
        assert!(matches!(
            PlatformRequest::prompt(window, PromptLevel::Info, "", None, &["OK".into()]),
            Err(PlatformError::EmptyPromptMessage)
        ));
        assert!(matches!(
            PlatformRequest::prompt(window, PromptLevel::Info, "Message", None, &[]),
            Err(PlatformError::InvalidButtons)
        ));
        assert!(matches!(
            PlatformRequest::prompt(
                window,
                PromptLevel::Info,
                "Message",
                None,
                &[PromptButton::ok("x".repeat(MAX_PROMPT_BUTTON_BYTES + 1))],
            ),
            Err(PlatformError::InvalidButton)
        ));

        let (request, response) = PlatformRequest::application_prompt(
            PromptLevel::Info,
            "Message",
            None,
            &[PromptButton::ok("OK")],
        )
        .unwrap();
        assert_eq!(request.window(), None);
        drop(response);
    }

    #[test]
    fn path_options_keep_native_ux_configuration_and_bounds() {
        let options = PathPromptOptions::new()
            .files(false)
            .directories(true)
            .multiple(true)
            .title("Open content")
            .prompt("Choose")
            .directory("/tmp")
            .suggested_name("notes.md")
            .filters([FileDialogFilter::new("Markdown", ["md"])])
            .shows_hidden_files(true);
        assert!(!options.files);
        assert!(options.directories);
        assert!(options.multiple);
        assert_eq!(options.title.as_deref(), Some("Open content"));
        assert_eq!(options.prompt.as_deref(), Some("Choose"));
        assert_eq!(options.directory.as_deref(), Some(Path::new("/tmp")));
        assert_eq!(options.suggested_name.as_deref(), Some("notes.md"));
        assert_eq!(options.filters[0].extensions[0].as_ref(), "md");
        assert!(options.shows_hidden_files);

        let invalid = PathPromptOptions::new().files(false);
        assert_eq!(
            PlatformRequest::open_paths(WindowHandle::next(), invalid).unwrap_err(),
            PlatformError::InvalidPathSelection
        );

        let invalid_filter =
            PathPromptOptions::new().filters([FileDialogFilter::new("Images", [".png"])]);
        assert_eq!(
            PlatformRequest::open_paths(WindowHandle::next(), invalid_filter).unwrap_err(),
            PlatformError::InvalidFileDialogFilter
        );

        let (request, response) =
            PlatformRequest::application_open_paths(PathPromptOptions::new()).unwrap();
        assert_eq!(request.window(), None);
        drop(response);

        let (request, response) =
            PlatformRequest::application_save_path(SavePathOptions::new("/tmp")).unwrap();
        assert_eq!(request.window(), None);
        drop(response);
    }

    #[test]
    fn shell_arguments_are_validated_before_queueing() {
        assert_eq!(
            PlatformRequest::open_url("").unwrap_err(),
            PlatformError::InvalidUrl
        );
        assert_eq!(
            PlatformRequest::open_path(PathBuf::new()).unwrap_err(),
            PlatformError::InvalidPath
        );
        assert!(PlatformRequest::reveal_path("/tmp").is_ok());
        assert!(PlatformRequest::trash_path("/tmp/discarded-item").is_ok());
    }

    #[test]
    fn desktop_shell_declarations_are_bounded_before_queueing() {
        assert_eq!(
            PlatformRequest::set_dock_badge(Some(Arc::from("x".repeat(MAX_DOCK_BADGE_BYTES + 1))))
                .unwrap_err(),
            PlatformError::InvalidDockBadge
        );
        assert_eq!(
            PlatformRequest::show_about_panel(
                AboutPanelOptions::new().credits("x".repeat(MAX_ABOUT_PANEL_TEXT_BYTES + 1))
            )
            .unwrap_err(),
            PlatformError::InvalidAboutPanelOptions
        );
        let tasks = (0..=MAX_USER_TASKS)
            .map(|index| UserTask::new(format!("Task {index}"), "--task"))
            .collect();
        assert_eq!(
            PlatformRequest::set_user_tasks(tasks).unwrap_err(),
            PlatformError::InvalidUserTasks
        );
        assert_eq!(
            PlatformRequest::set_user_tasks(vec![UserTask::new("", "--task")]).unwrap_err(),
            PlatformError::InvalidUserTasks
        );

        let (request, response) = PlatformRequest::get_file_icon("/tmp", FileIconSize::Large)
            .expect("a bounded path should produce an asynchronous lookup");
        assert!(!request.response_cancelled());
        drop(response);
        assert!(request.response_cancelled());
    }

    #[test]
    fn system_notifications_are_validated_before_retention() {
        let valid = SystemNotification::new("build", "Build finished", "All checks passed")
            .action(SystemNotificationAction::new("open", "Open"));
        assert!(PlatformRequest::show_system_notification(valid).is_ok());

        assert_eq!(
            PlatformRequest::show_system_notification(SystemNotification::new("", "Title", "Body"))
                .unwrap_err(),
            PlatformError::InvalidNotificationTag
        );
        assert_eq!(
            PlatformRequest::show_system_notification(SystemNotification::new("tag", "", "Body"))
                .unwrap_err(),
            PlatformError::InvalidNotificationText
        );
        let duplicate = SystemNotification::new("tag", "Title", "Body")
            .action(SystemNotificationAction::new("open", "Open"))
            .action(SystemNotificationAction::new("open", "Open again"));
        assert_eq!(
            PlatformRequest::show_system_notification(duplicate).unwrap_err(),
            PlatformError::InvalidNotificationAction
        );
        for reserved in [
            SYSTEM_NOTIFICATION_DEFAULT_ACTION_ID,
            SYSTEM_NOTIFICATION_DISMISS_ACTION_ID,
        ] {
            let notification = SystemNotification::new("tag", "Title", "Body")
                .action(SystemNotificationAction::new(reserved, "Reserved"));
            assert_eq!(
                PlatformRequest::show_system_notification(notification).unwrap_err(),
                PlatformError::InvalidNotificationAction
            );
        }
        let mut oversized_allocation = Vec::with_capacity(65_536);
        oversized_allocation.push(SystemNotificationAction::new("open", "Open"));
        let mut notification = SystemNotification::new("tag", "Title", "Body");
        notification.actions = oversized_allocation;
        let normalized = PlatformRequest::show_system_notification(notification).unwrap();
        let PlatformRequest::ShowSystemNotification(normalized) = normalized else {
            unreachable!("the constructor returns the matching request variant");
        };
        assert_eq!(normalized.actions.len(), 1);
        assert!(normalized.actions.capacity() <= MAX_SYSTEM_NOTIFICATION_ACTIONS);
        assert_eq!(
            PlatformRequest::dismiss_system_notification("").unwrap_err(),
            PlatformError::InvalidNotificationTag
        );
    }

    #[test]
    fn native_open_urls_keep_a_shared_bounded_collection() {
        let urls = OpenUrls::from_bounded(vec![
            Arc::from("quickgui://open/7"),
            Arc::from("file:///tmp/example.txt"),
        ]);
        let clone = urls.clone();
        assert_eq!(urls.len(), 2);
        assert_eq!(
            clone.iter().collect::<Vec<_>>(),
            ["quickgui://open/7", "file:///tmp/example.txt"]
        );
        assert!(Arc::ptr_eq(&urls.urls, &clone.urls));
    }
}
