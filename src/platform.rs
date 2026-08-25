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
};

use thiserror::Error;
#[cfg(target_os = "macos")]
use winit::event_loop::EventLoopProxy;

use crate::WindowHandle;
#[cfg(target_os = "macos")]
use crate::runtime::RuntimeEvent;

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
/// Maximum distinct native notification action sets retained by one application.
pub const MAX_SYSTEM_NOTIFICATION_CATEGORIES: usize = 64;
/// Maximum distinct notification tags retained while native authorization is pending.
pub const MAX_PENDING_SYSTEM_NOTIFICATIONS: usize = 64;

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
    #[error("the window already owns an active native dialog")]
    DialogBusy,
    #[error("the application already owns {MAX_ACTIVE_PLATFORM_DIALOGS} active native dialogs")]
    TooManyDialogs,
    #[error("this platform service is not implemented on the current operating system")]
    Unsupported,
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
    #[error("an open panel must allow files, directories, or both")]
    InvalidPathSelection,
    #[error("the native path selection exceeded QuickGUI's bounded result size")]
    SelectionTooLarge,
    #[error("native platform operation failed: {0}")]
    Platform(Arc<str>),
}

/// Native prompt severity, matching the platform's informational, warning, and critical styles.
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
}

impl SystemNotificationAction {
    pub fn new(id: impl Into<Arc<str>>, label: impl Into<Arc<str>>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
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
    pub body: Arc<str>,
    pub actions: Vec<SystemNotificationAction>,
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
            body: body.into(),
            actions: Vec::new(),
        }
    }

    pub fn action(mut self, action: SystemNotificationAction) -> Self {
        self.actions.push(action);
        self
    }
}

/// The user's activation of an operating-system notification.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SystemNotificationResponse {
    pub tag: Arc<str>,
    /// The selected action id, or `None` when the notification body was activated.
    pub action_id: Option<Arc<str>>,
}

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

/// Options for a native open panel.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PathPromptOptions {
    pub files: bool,
    pub directories: bool,
    pub multiple: bool,
    pub prompt: Option<Arc<str>>,
    pub directory: Option<PathBuf>,
    pub shows_hidden_files: bool,
}

impl Default for PathPromptOptions {
    fn default() -> Self {
        Self {
            files: true,
            directories: false,
            multiple: false,
            prompt: None,
            directory: None,
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

    pub fn prompt(mut self, prompt: impl Into<Arc<str>>) -> Self {
        self.prompt = Some(prompt.into());
        self
    }

    pub fn directory(mut self, directory: impl Into<PathBuf>) -> Self {
        self.directory = Some(directory.into());
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
    pub suggested_name: Option<Arc<str>>,
    pub prompt: Option<Arc<str>>,
    pub shows_hidden_files: bool,
}

impl SavePathOptions {
    pub fn new(directory: impl Into<PathBuf>) -> Self {
        Self {
            directory: directory.into(),
            suggested_name: None,
            prompt: None,
            shows_hidden_files: false,
        }
    }

    pub fn suggested_name(mut self, suggested_name: impl Into<Arc<str>>) -> Self {
        self.suggested_name = Some(suggested_name.into());
        self
    }

    pub fn prompt(mut self, prompt: impl Into<Arc<str>>) -> Self {
        self.prompt = Some(prompt.into());
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
    owner: WindowHandle,
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
        owner: WindowHandle,
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

fn response_channel<T>() -> (PlatformResponder<T>, PlatformResponse<T>) {
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
        window: WindowHandle,
        level: PromptLevel,
        message: Arc<str>,
        detail: Option<Arc<str>>,
        buttons: Vec<PromptButton>,
        responder: PlatformResponder<usize>,
    },
    OpenPaths {
        window: WindowHandle,
        options: PathPromptOptions,
        responder: PlatformResponder<Option<Vec<PathBuf>>>,
    },
    SavePath {
        window: WindowHandle,
        options: SavePathOptions,
        responder: PlatformResponder<Option<PathBuf>>,
    },
    ShowSystemNotification(SystemNotification),
    DismissSystemNotification(Arc<str>),
    OpenUrl(Arc<str>),
    OpenPath(PathBuf),
    RevealPath(PathBuf),
}

impl PlatformRequest {
    pub(crate) fn prompt(
        window: WindowHandle,
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
        Ok(Self::OpenUrl(url))
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

    pub(crate) fn open_path(path: impl Into<PathBuf>) -> Result<Self, PlatformError> {
        let path = path.into();
        validate_path(&path)?;
        Ok(Self::OpenPath(path))
    }

    pub(crate) fn reveal_path(path: impl Into<PathBuf>) -> Result<Self, PlatformError> {
        let path = path.into();
        validate_path(&path)?;
        Ok(Self::RevealPath(path))
    }

    pub(crate) fn window(&self) -> Option<WindowHandle> {
        match self {
            Self::Prompt { window, .. }
            | Self::OpenPaths { window, .. }
            | Self::SavePath { window, .. } => Some(*window),
            Self::ShowSystemNotification(_)
            | Self::DismissSystemNotification(_)
            | Self::OpenUrl(_)
            | Self::OpenPath(_)
            | Self::RevealPath(_) => None,
        }
    }

    pub(crate) fn response_cancelled(&self) -> bool {
        match self {
            Self::Prompt { responder, .. } => responder.is_cancelled(),
            Self::OpenPaths { responder, .. } => responder.is_cancelled(),
            Self::SavePath { responder, .. } => responder.is_cancelled(),
            Self::ShowSystemNotification(_)
            | Self::DismissSystemNotification(_)
            | Self::OpenUrl(_)
            | Self::OpenPath(_)
            | Self::RevealPath(_) => false,
        }
    }

    #[cfg(target_os = "macos")]
    pub(crate) fn bind_cancellation(
        &self,
        proxy: EventLoopProxy<RuntimeEvent>,
        owner: WindowHandle,
        id: PlatformDialogId,
    ) -> bool {
        match self {
            Self::Prompt { responder, .. } => responder.bind_cancellation(proxy, owner, id),
            Self::OpenPaths { responder, .. } => responder.bind_cancellation(proxy, owner, id),
            Self::SavePath { responder, .. } => responder.bind_cancellation(proxy, owner, id),
            Self::ShowSystemNotification(_)
            | Self::DismissSystemNotification(_)
            | Self::OpenUrl(_)
            | Self::OpenPath(_)
            | Self::RevealPath(_) => false,
        }
    }

    pub(crate) fn complete_error(self, error: PlatformError) {
        match self {
            Self::Prompt { responder, .. } => responder.complete(Err(error)),
            Self::OpenPaths { responder, .. } => responder.complete(Err(error)),
            Self::SavePath { responder, .. } => responder.complete(Err(error)),
            Self::ShowSystemNotification(_)
            | Self::DismissSystemNotification(_)
            | Self::OpenUrl(_)
            | Self::OpenPath(_)
            | Self::RevealPath(_) => {}
        }
    }
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
    if let Some(prompt) = &options.prompt {
        validate_optional_text(prompt)?;
    }
    if let Some(directory) = &options.directory {
        validate_path(directory)?;
    }
    Ok(())
}

fn validate_save_path_options(options: &SavePathOptions) -> Result<(), PlatformError> {
    validate_path(&options.directory)?;
    if let Some(name) = &options.suggested_name {
        validate_optional_text(name)?;
    }
    if let Some(prompt) = &options.prompt {
        validate_optional_text(prompt)?;
    }
    Ok(())
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
    }
    Ok(())
}

fn normalize_system_notification(
    notification: SystemNotification,
) -> Result<SystemNotification, PlatformError> {
    validate_system_notification(&notification)?;
    let SystemNotification {
        tag,
        title,
        body,
        actions: source_actions,
    } = notification;
    let mut actions = Vec::with_capacity(source_actions.len());
    for action in source_actions {
        actions.push(action);
    }
    Ok(SystemNotification {
        tag,
        title,
        body,
        actions,
    })
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
    }

    #[test]
    fn path_options_keep_native_ux_configuration_and_bounds() {
        let options = PathPromptOptions::new()
            .files(false)
            .directories(true)
            .multiple(true)
            .prompt("Choose")
            .directory("/tmp")
            .shows_hidden_files(true);
        assert!(!options.files);
        assert!(options.directories);
        assert!(options.multiple);
        assert_eq!(options.prompt.as_deref(), Some("Choose"));
        assert_eq!(options.directory.as_deref(), Some(Path::new("/tmp")));
        assert!(options.shows_hidden_files);

        let invalid = PathPromptOptions::new().files(false);
        assert_eq!(
            PlatformRequest::open_paths(WindowHandle::next(), invalid).unwrap_err(),
            PlatformError::InvalidPathSelection
        );
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
        let normalized = PlatformRequest::show_system_notification(SystemNotification {
            tag: Arc::from("tag"),
            title: Arc::from("Title"),
            body: Arc::from("Body"),
            actions: oversized_allocation,
        })
        .unwrap();
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
