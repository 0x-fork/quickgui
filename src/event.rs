use std::{
    any::{Any, TypeId},
    cell::{Ref, RefMut},
    future::Future,
    path::{Path, PathBuf},
    sync::Arc,
};

use bitflags::bitflags;
use thiserror::Error;

use crate::{
    Action, AnyAction, Assets, Display, DisplayId, Displays, ElementId, Entity, EntityId,
    EventEmitter, FocusHandle, Global, KeyboardLayout, Menu, Point, Size, Vector, View,
    WindowHandle, WindowOptions,
    clipboard::{ClipboardError, ClipboardItem, ClipboardService, ClipboardTarget},
    entity::{EntityEvent, MAX_ENTITY_EVENTS_PER_CALLBACK, MAX_ENTITY_NOTIFICATIONS_PER_EVENT},
    foreground::{AsyncViewContext, ForegroundTaskSpawnError, ForegroundTaskSpawner, Task},
    global::{GlobalStore, MAX_GLOBAL_NOTIFICATIONS_PER_EVENT},
    platform::{
        PathPromptOptions, PathPromptResponse, PlatformError, PlatformRequest, PlatformResponse,
        PromptButton, PromptLevel, SavePathOptions, SavePathResponse, SystemNotification,
    },
    runtime::{
        MAX_SYSTEM_WINDOW_TABS, WindowAppearance, WindowBackgroundAppearance, WindowCommand,
        WindowCommandError, WindowRequest, validate_window_bounds, validate_window_document_path,
        validate_window_position, validate_window_size, validate_window_tabbing_identifier,
        validate_window_title,
    },
};

/// Maximum actions one callback may target at another retained window.
pub const MAX_TARGETED_ACTIONS_PER_EVENT: usize = 256;
/// Maximum cross-window actions retained across one application effect cycle.
pub const MAX_PENDING_TARGETED_ACTIONS: usize = 1_024;

/// Framework-level input and window events, expressed in logical pixels.
#[derive(Clone, Debug, PartialEq)]
pub enum Event {
    /// The native window asked to close. Call [`EventContext::prevent_close`] to keep it open.
    CloseRequested,
    PointerMoved(Point),
    PointerLeft,
    MouseButton {
        button: MouseButton,
        pressed: bool,
    },
    Click(crate::ElementId),
    /// A secondary-click gesture targeted an element with a context-menu listener.
    ContextMenu(ContextMenuEvent),
    /// A top-level surface was dismissed by Escape or an outside pointer press.
    Dismiss(crate::ElementId),
    /// A coalesced scroll delta. Multiple platform wheel events may become one event.
    Scroll(Vector),
    /// Force-sensitive pointer pressure targeted at the current logical pointer position.
    MousePressure(MousePressureEvent),
    /// A native pinch-to-zoom gesture.
    Pinch(PinchEvent),
    /// A native two-finger rotation gesture.
    Rotation(RotationEvent),
    /// A native smart-magnify gesture, normally a two-finger double tap on macOS.
    SmartMagnify(SmartMagnifyEvent),
    /// One raw direct-touch contact in window coordinates.
    ///
    /// Indirect devices such as macOS trackpads are exposed through scroll and gesture events
    /// because their contacts have no corresponding position in the window.
    Touch(TouchEvent),
    KeyDown {
        /// Normalized command identity, independent from Caps Lock and text composition.
        key: Key,
        /// Normalized printable character this press could produce before IME composition.
        key_char: Option<Key>,
        modifiers: Modifiers,
        repeat: bool,
    },
    KeyUp {
        key: Key,
        key_char: Option<Key>,
        modifiers: Modifiers,
    },
    /// Committed text from the platform input method.
    TextInput(String),
    ModifiersChanged(Modifiers),
    /// The focused element changed within the window.
    FocusChanged(Option<ElementId>),
    /// The native window itself gained or lost focus.
    Focused(bool),
    /// The effective native light/dark appearance changed while following the system.
    AppearanceChanged(WindowAppearance),
    /// The native window moved in logical desktop coordinates.
    Moved {
        logical_position: Point,
        scale_factor: f32,
    },
    /// Files from another application are currently hovering this window.
    FilesHovered(DroppedFiles),
    /// A native file drag left the window without being dropped.
    FilesHoverCancelled,
    /// Files from another application were dropped into this window.
    FilesDropped(DroppedFiles),
    /// A drag promoted from this window to the native platform has ended.
    ExternalDragEnded(ExternalDragEndEvent),
    Resized {
        logical_size: Size,
        scale_factor: f32,
    },
}

/// Maximum controlled text fields exposed by one form submission.
pub const MAX_FORM_FIELDS: usize = 256;
/// Maximum invalid controls retained by one validation report.
pub const MAX_VALIDATION_ISSUES: usize = 256;
/// Maximum UTF-8 bytes retained for one declarative validation message.
pub const MAX_VALIDATION_MESSAGE_BYTES: usize = 4 * 1024;
/// Maximum programmatic form submissions queued by one event callback.
pub const MAX_FORM_SUBMISSIONS_PER_EVENT: usize = 64;

/// One controlled text value captured for a valid form submission.
///
/// Values stay shared with the retained text input, so submitting a large text area does not copy
/// its contents. Fields are emitted in document order.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FormField {
    id: ElementId,
    value: Arc<str>,
}

impl FormField {
    pub(crate) fn new(id: ElementId, value: Arc<str>) -> Self {
        Self { id, value }
    }

    pub const fn id(&self) -> ElementId {
        self.id
    }

    pub fn value(&self) -> &str {
        &self.value
    }
}

/// Data delivered after every enabled control in a form is valid.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FormSubmitEvent {
    form: ElementId,
    trigger: Option<ElementId>,
    fields: Arc<[FormField]>,
    truncated: bool,
}

impl FormSubmitEvent {
    pub(crate) fn new(
        form: ElementId,
        trigger: Option<ElementId>,
        fields: Vec<FormField>,
        truncated: bool,
    ) -> Self {
        debug_assert!(fields.len() <= MAX_FORM_FIELDS);
        Self {
            form,
            trigger,
            fields: Arc::from(fields),
            truncated,
        }
    }

    pub const fn form(&self) -> ElementId {
        self.form
    }

    /// The input, submit button, or custom action that initiated submission.
    pub const fn trigger(&self) -> Option<ElementId> {
        self.trigger
    }

    pub fn fields(&self) -> &[FormField] {
        &self.fields
    }

    pub fn value(&self, id: impl Into<ElementId>) -> Option<&str> {
        let id = id.into();
        self.fields
            .iter()
            .find(|field| field.id == id)
            .map(FormField::value)
    }

    /// Whether fields beyond [`MAX_FORM_FIELDS`] were omitted.
    pub const fn is_truncated(&self) -> bool {
        self.truncated
    }
}

/// One enabled control that prevented a form submission.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ValidationIssue {
    id: ElementId,
    message: Option<Arc<str>>,
    message_truncated: bool,
}

impl ValidationIssue {
    pub(crate) fn new(id: ElementId, message: Option<Arc<str>>, message_truncated: bool) -> Self {
        Self {
            id,
            message,
            message_truncated,
        }
    }

    pub const fn id(&self) -> ElementId {
        self.id
    }

    pub fn message(&self) -> Option<&str> {
        self.message.as_deref()
    }

    /// Whether the original message exceeded [`MAX_VALIDATION_MESSAGE_BYTES`].
    pub const fn is_message_truncated(&self) -> bool {
        self.message_truncated
    }
}

/// Bounded, document-ordered details for a form that could not be submitted.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ValidationReport {
    form: ElementId,
    trigger: Option<ElementId>,
    issues: Arc<[ValidationIssue]>,
    truncated: bool,
}

impl ValidationReport {
    pub(crate) fn new(
        form: ElementId,
        trigger: Option<ElementId>,
        issues: Vec<ValidationIssue>,
        truncated: bool,
    ) -> Self {
        debug_assert!(issues.len() <= MAX_VALIDATION_ISSUES);
        Self {
            form,
            trigger,
            issues: Arc::from(issues),
            truncated,
        }
    }

    pub const fn form(&self) -> ElementId {
        self.form
    }

    pub const fn trigger(&self) -> Option<ElementId> {
        self.trigger
    }

    pub fn issues(&self) -> &[ValidationIssue] {
        &self.issues
    }

    pub fn first(&self) -> Option<&ValidationIssue> {
        self.issues.first()
    }

    /// Whether invalid controls beyond [`MAX_VALIDATION_ISSUES`] were omitted.
    pub const fn is_truncated(&self) -> bool {
        self.truncated
    }
}

/// Maximum path count retained for one native file drag.
pub const MAX_DROPPED_FILES: usize = 4_096;

/// Maximum file count retained for one outbound native drag.
pub const MAX_EXTERNAL_DRAG_FILES: usize = 4_096;
/// Maximum encoded bytes retained for one outbound native-drag path.
pub const MAX_EXTERNAL_DRAG_PATH_BYTES: usize = 16 * 1024;
/// Maximum encoded path bytes retained across one outbound native drag.
pub const MAX_EXTERNAL_DRAG_TOTAL_PATH_BYTES: usize = 8 * 1024 * 1024;
/// Maximum UTF-8 storage retained by one inbound or outbound native text drag.
pub const MAX_EXTERNAL_DRAG_TEXT_BYTES: usize = 1024 * 1024;
/// Maximum UTF-8 storage retained by one inbound or outbound native URL drag.
pub const MAX_EXTERNAL_DRAG_URL_BYTES: usize = 16 * 1024;

/// A bounded, shared collection of paths delivered by a native file drag.
///
/// Winit reports one path at a time. QuickGUI retains the paths observed for the current native
/// drag and delivers one collection when the platform submits it. The same value can be accepted
/// by a typed [`crate::DropListener`].
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct DroppedFiles {
    paths: Arc<[PathBuf]>,
    truncated: bool,
}

impl DroppedFiles {
    pub fn new(paths: impl IntoIterator<Item = PathBuf>) -> Self {
        let mut paths = paths.into_iter();
        let retained = paths.by_ref().take(MAX_DROPPED_FILES).collect::<Vec<_>>();
        Self {
            paths: Arc::from(retained),
            truncated: paths.next().is_some(),
        }
    }

    pub fn paths(&self) -> &[PathBuf] {
        &self.paths
    }

    pub fn is_empty(&self) -> bool {
        self.paths.is_empty()
    }

    /// Whether paths beyond [`MAX_DROPPED_FILES`] were discarded.
    pub fn is_truncated(&self) -> bool {
        self.truncated
    }

    pub(crate) fn from_retained(paths: Vec<PathBuf>, truncated: bool) -> Self {
        debug_assert!(paths.len() <= MAX_DROPPED_FILES);
        Self {
            paths: Arc::from(paths),
            truncated,
        }
    }
}

/// Files offered to the platform when an internal drag leaves its QuickGUI window.
///
/// Directory metadata is supplied by the application so starting a native drag never performs a
/// synchronous filesystem query. Construction examines at most
/// [`MAX_EXTERNAL_DRAG_FILES`] plus one entry and retains bounded path storage.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct FileDragPaths {
    entries: Arc<[(PathBuf, bool)]>,
    truncated: bool,
}

impl FileDragPaths {
    /// Build a bounded outbound file payload from `(path, is_directory)` pairs.
    pub fn new(entries: impl IntoIterator<Item = (PathBuf, bool)>) -> Self {
        let mut entries = entries.into_iter();
        let mut retained = Vec::with_capacity(entries.size_hint().0.min(MAX_EXTERNAL_DRAG_FILES));
        let mut retained_bytes = 0_usize;
        let mut truncated = false;

        for (path, is_directory) in entries.by_ref().take(MAX_EXTERNAL_DRAG_FILES) {
            let path_bytes = path.as_os_str().as_encoded_bytes().len();
            let next_bytes = retained_bytes.saturating_add(path_bytes);
            if path_bytes == 0
                || path_bytes > MAX_EXTERNAL_DRAG_PATH_BYTES
                || next_bytes > MAX_EXTERNAL_DRAG_TOTAL_PATH_BYTES
            {
                truncated = true;
                continue;
            }
            retained_bytes = next_bytes;
            retained.push((path, is_directory));
        }
        truncated |= entries.next().is_some();

        Self {
            entries: Arc::from(retained),
            truncated,
        }
    }

    /// Build a bounded payload containing only files.
    pub fn files(paths: impl IntoIterator<Item = PathBuf>) -> Self {
        Self::new(paths.into_iter().map(|path| (path, false)))
    }

    /// Build a bounded payload containing only directories.
    pub fn directories(paths: impl IntoIterator<Item = PathBuf>) -> Self {
        Self::new(paths.into_iter().map(|path| (path, true)))
    }

    /// The retained paths paired with whether each one is a directory.
    pub fn entries(&self) -> &[(PathBuf, bool)] {
        &self.entries
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Whether an entry was omitted by a count or byte bound.
    pub fn is_truncated(&self) -> bool {
        self.truncated
    }
}

/// Bounded plain text exchanged with a native drag destination.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ExternalDragText {
    text: Arc<str>,
    truncated: bool,
}

impl ExternalDragText {
    pub fn new(text: impl Into<Arc<str>>) -> Self {
        let text = text.into();
        if text.len() <= MAX_EXTERNAL_DRAG_TEXT_BYTES {
            return Self {
                text,
                truncated: false,
            };
        }

        let mut end = MAX_EXTERNAL_DRAG_TEXT_BYTES;
        while !text.is_char_boundary(end) {
            end -= 1;
        }
        Self {
            text: Arc::from(&text[..end]),
            truncated: true,
        }
    }

    pub fn as_str(&self) -> &str {
        &self.text
    }

    pub fn is_empty(&self) -> bool {
        self.text.is_empty()
    }

    /// Whether the UTF-8 suffix beyond [`MAX_EXTERNAL_DRAG_TEXT_BYTES`] was discarded.
    pub fn is_truncated(&self) -> bool {
        self.truncated
    }

    pub(crate) fn from_bounded(text: String, truncated: bool) -> Self {
        debug_assert!(text.len() <= MAX_EXTERNAL_DRAG_TEXT_BYTES);
        Self {
            text: Arc::from(text),
            truncated,
        }
    }
}

impl From<&str> for ExternalDragText {
    fn from(text: &str) -> Self {
        Self::new(text)
    }
}

impl From<String> for ExternalDragText {
    fn from(text: String) -> Self {
        Self::new(text)
    }
}

impl From<Arc<str>> for ExternalDragText {
    fn from(text: Arc<str>) -> Self {
        Self::new(text)
    }
}

/// A bounded absolute URL exchanged with a native drag destination.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExternalDragUrl(Arc<str>);

#[derive(Clone, Debug, Error, Eq, PartialEq)]
pub enum ExternalDragUrlError {
    #[error("external drag URL is empty")]
    Empty,
    #[error(
        "external drag URL is {bytes} bytes; the maximum is {MAX_EXTERNAL_DRAG_URL_BYTES} bytes"
    )]
    TooLarge { bytes: usize },
    #[error("external drag URL must have an absolute RFC 3986 scheme")]
    InvalidScheme,
    #[error("external drag URL contains whitespace or control characters")]
    InvalidCharacter,
}

impl ExternalDragUrl {
    pub fn new(url: impl Into<Arc<str>>) -> Result<Self, ExternalDragUrlError> {
        let url = url.into();
        if url.is_empty() {
            return Err(ExternalDragUrlError::Empty);
        }
        if url.len() > MAX_EXTERNAL_DRAG_URL_BYTES {
            return Err(ExternalDragUrlError::TooLarge { bytes: url.len() });
        }
        if url
            .chars()
            .any(|character| character.is_whitespace() || character.is_control())
        {
            return Err(ExternalDragUrlError::InvalidCharacter);
        }
        let Some((scheme, remainder)) = url.split_once(':') else {
            return Err(ExternalDragUrlError::InvalidScheme);
        };
        let mut characters = scheme.chars();
        if remainder.is_empty()
            || !characters
                .next()
                .is_some_and(|character| character.is_ascii_alphabetic())
            || !characters.all(|character| {
                character.is_ascii_alphanumeric() || matches!(character, '+' | '-' | '.')
            })
        {
            return Err(ExternalDragUrlError::InvalidScheme);
        }
        Ok(Self(url))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Plain text delivered by an inbound native drag.
///
/// This is the same exact typed value as [`ExternalDragText`], allowing one drop listener to
/// accept a drag originating in QuickGUI or another application.
pub type DroppedText = ExternalDragText;

/// An absolute URL delivered by an inbound native drag.
///
/// This is the same exact typed value as [`ExternalDragUrl`].
pub type DroppedUrl = ExternalDragUrl;

/// Data offered when an internal drag is promoted to a native platform drag.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ExternalDragPayload {
    /// Existing on-disk files or directories.
    Files(FileDragPaths),
    /// Plain UTF-8 text.
    Text(ExternalDragText),
    /// One absolute URL.
    Url(ExternalDragUrl),
}

/// The operation chosen by the native drag destination.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ExternalDragOperation {
    Cancelled,
    Copied,
    Moved,
    Linked,
    Deleted,
    Other,
}

/// Completion metadata for [`Event::ExternalDragEnded`].
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ExternalDragEndEvent {
    /// The stable element that originated the promoted drag.
    pub source: ElementId,
    pub operation: ExternalDragOperation,
}

/// Input geometry supplied when an internal drag crosses the movement threshold.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DragStartEvent {
    /// Position where the primary button was pressed.
    pub origin: Point,
    /// Position that crossed the drag threshold.
    pub position: Point,
    pub modifiers: Modifiers,
}

/// Identifies whether a typed drop originated in this window, another QuickGUI window, or another
/// application.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DragOrigin {
    /// The payload never left this QuickGUI window's retained drag path.
    Internal(ElementId),
    /// A process-local typed payload crossed between independently retained QuickGUI windows.
    CrossWindow {
        window: WindowHandle,
        source: ElementId,
    },
    /// A payload originated outside this QuickGUI application.
    External,
}

/// Geometry supplied to a typed drop listener.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DropEvent {
    pub position: Point,
    pub modifiers: Modifiers,
    pub origin: DragOrigin,
}

#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub enum MouseButton {
    #[default]
    Left,
    Right,
    Middle,
    Back,
    Forward,
    Other(u16),
}

/// One of the two ordered stages used for targeted desktop mouse dispatch.
///
/// Capture listeners run from the root toward the hit-tested target. Bubble listeners then run
/// from that target back toward the root. Stopping propagation ends the remainder of both stages
/// without changing the framework's default focus, selection, drag, or click behavior.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub enum DispatchPhase {
    #[default]
    Bubble,
    Capture,
}

impl DispatchPhase {
    pub const fn bubble(self) -> bool {
        matches!(self, Self::Bubble)
    }

    pub const fn capture(self) -> bool {
        matches!(self, Self::Capture)
    }
}

/// A desktop mouse-button press targeted through the retained element tree.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct MouseDownEvent {
    pub button: MouseButton,
    pub position: Point,
    pub modifiers: Modifiers,
    /// Native multi-click count. The first press is `1`.
    pub click_count: usize,
    /// Whether this press activated an otherwise unfocused native window.
    pub first_mouse: bool,
}

impl MouseDownEvent {
    pub const fn is_focusing(self) -> bool {
        matches!(self.button, MouseButton::Left)
    }
}

/// A desktop mouse-button release targeted through the retained element tree.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct MouseUpEvent {
    pub button: MouseButton,
    pub position: Point,
    pub modifiers: Modifiers,
    /// Count shared with the matching [`MouseDownEvent`].
    pub click_count: usize,
}

/// Mouse motion targeted through the retained element tree.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct MouseMoveEvent {
    pub position: Point,
    pub pressed_button: Option<MouseButton>,
    pub modifiers: Modifiers,
}

impl MouseMoveEvent {
    pub const fn dragging(self) -> bool {
        self.pressed_button.is_some()
    }

    pub fn dragging_button(self, button: MouseButton) -> bool {
        matches!(self.pressed_button, Some(pressed) if pressed == button)
    }
}

/// Mouse motion delivered when the pointer leaves the native window.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct MouseExitEvent {
    pub position: Point,
    pub pressed_button: Option<MouseButton>,
    pub modifiers: Modifiers,
}

/// The stage of a captured pointer interaction.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PointerPhase {
    Down,
    Move,
    Up,
    /// The window lost focus before the pressed button was released.
    Cancel,
}

/// A pointer event delivered to an element that owns pointer capture.
///
/// Positions and deltas use logical pixels. Once an element receives [`PointerPhase::Down`], it
/// continues to receive move events and the terminal up or cancel event even when the pointer is
/// outside its bounds.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PointerEvent {
    pub phase: PointerPhase,
    pub position: Point,
    /// Position at which this capture started.
    pub origin: Point,
    /// Motion since the preceding captured event.
    pub delta: Vector,
    pub button: MouseButton,
    pub modifiers: Modifiers,
}

/// Maximum absolute platform pixel delta retained from one scroll-wheel event.
///
/// Real trackpad deltas are many orders of magnitude smaller. The bound keeps malformed native
/// input finite before application zoom or pan arithmetic sees it.
pub const MAX_SCROLL_PIXELS_PER_EVENT: f32 = 1_048_576.0;

/// Maximum absolute platform line delta retained from one scroll-wheel event.
pub const MAX_SCROLL_LINES_PER_EVENT: f32 = 4_096.0;

/// Native scroll-wheel movement before conversion into an application-selected line height.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ScrollDelta {
    /// Exact logical-pixel movement from a trackpad or precise wheel.
    Pixels(Vector),
    /// Device line units from a discrete wheel.
    Lines(Vector),
}

impl Default for ScrollDelta {
    fn default() -> Self {
        Self::Lines(Vector::ZERO)
    }
}

impl ScrollDelta {
    /// Whether this delta came from a precise pixel-scrolling device.
    pub fn precise(&self) -> bool {
        matches!(self, Self::Pixels(_))
    }

    /// Convert this delta to finite logical pixels using `line_height` for discrete wheels.
    pub fn pixel_delta(&self, line_height: f32) -> Vector {
        match *self {
            Self::Pixels(delta) => bounded_scroll_vector(delta, MAX_SCROLL_PIXELS_PER_EVENT),
            Self::Lines(delta) => {
                let line_height = if line_height.is_finite() {
                    line_height.clamp(0.0, MAX_SCROLL_PIXELS_PER_EVENT)
                } else {
                    0.0
                };
                bounded_scroll_vector(
                    Vector::new(delta.x * line_height, delta.y * line_height),
                    MAX_SCROLL_PIXELS_PER_EVENT,
                )
            }
        }
    }

    pub(crate) fn bounded(self) -> Self {
        match self {
            Self::Pixels(delta) => {
                Self::Pixels(bounded_scroll_vector(delta, MAX_SCROLL_PIXELS_PER_EVENT))
            }
            Self::Lines(delta) => {
                Self::Lines(bounded_scroll_vector(delta, MAX_SCROLL_LINES_PER_EVENT))
            }
        }
    }
}

/// A scroll-wheel event delivered through the topmost element's ancestor path.
///
/// Call [`EventContext::prevent_default`] to suppress retained scrolling and
/// [`EventContext::stop_propagation`] to keep the event from reaching a listening ancestor.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ScrollWheelEvent {
    pub position: Point,
    pub delta: ScrollDelta,
    pub phase: GesturePhase,
    pub modifiers: Modifiers,
}

impl ScrollWheelEvent {
    pub(crate) fn bounded(mut self) -> Self {
        self.delta = self.delta.bounded();
        self
    }
}

fn bounded_scroll_vector(delta: Vector, limit: f32) -> Vector {
    let component = |value: f32| {
        if value.is_finite() {
            value.clamp(-limit, limit)
        } else {
            0.0
        }
    };
    Vector::new(component(delta.x), component(delta.y))
}

/// Maximum absolute magnification retained from one native pinch event.
///
/// Native deltas are normally small fractions. Bounding malformed platform input keeps
/// application zoom arithmetic finite without changing ordinary gestures.
pub const MAX_PINCH_DELTA_PER_EVENT: f32 = 8.0;

/// Maximum absolute rotation retained from one native gesture event, in degrees.
pub const MAX_ROTATION_DEGREES_PER_EVENT: f32 = 360.0;

/// The lifecycle phase of a native continuous gesture.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub enum GesturePhase {
    Started,
    #[default]
    Moved,
    Ended,
    Cancelled,
}

/// Maximum simultaneously captured touch contacts retained by one window.
pub const MAX_ACTIVE_TOUCHES_PER_WINDOW: usize = 32;

/// Maximum absolute logical coordinate accepted from a native touch sample.
pub const MAX_TOUCH_COORDINATE: f32 = 16_777_216.0;

/// Opaque identity for one touch from start through end or cancellation.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct TouchId(pub u64);

/// The lifecycle phase of one raw touch contact.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub enum TouchPhase {
    Started,
    #[default]
    Moved,
    Ended,
    Cancelled,
}

/// One fixed-size direct-touch sample in logical top-left window coordinates.
///
/// QuickGUI hit-tests a contact only at [`TouchPhase::Started`] and captures the nearest listening
/// element for the rest of that contact. This is distinct from mouse pointer capture and supports
/// multiple simultaneous touch IDs.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct TouchEvent {
    pub id: TouchId,
    pub phase: TouchPhase,
    pub position: Point,
    /// Normalized pressure in `0.0..=1.0` when reported by the platform.
    pub force: Option<f32>,
}

impl TouchEvent {
    pub(crate) fn bounded(mut self) -> Self {
        let coordinate = |value: f32| {
            if value.is_finite() {
                value.clamp(-MAX_TOUCH_COORDINATE, MAX_TOUCH_COORDINATE)
            } else {
                0.0
            }
        };
        self.position = Point::new(coordinate(self.position.x), coordinate(self.position.y));
        self.force = self.force.map(|force| {
            if force.is_finite() {
                force.clamp(0.0, 1.0)
            } else {
                0.0
            }
        });
        self
    }
}

/// The click level reported by a force-sensitive pointing device.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub enum PressureStage {
    #[default]
    Zero,
    Normal,
    Force,
    /// A future platform stage that QuickGUI does not assign a semantic name yet.
    Other(i64),
}

/// Force-sensitive pointer input, currently produced by macOS Force Touch trackpads.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct MousePressureEvent {
    /// Logical top-left window coordinate of the pressure event.
    pub position: Point,
    /// Pressure within the current stage, normalized to `0.0..=1.0`.
    pub pressure: f32,
    pub stage: PressureStage,
    pub modifiers: Modifiers,
}

/// A native pinch-to-zoom event targeted at the pointer's logical window position.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct PinchEvent {
    pub position: Point,
    /// Positive values magnify and negative values shrink. `0.1` represents a 10% increment.
    pub delta: f32,
    pub phase: GesturePhase,
    pub modifiers: Modifiers,
}

/// A native two-finger rotation event.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct RotationEvent {
    pub position: Point,
    /// Incremental rotation in degrees. Positive values rotate counterclockwise.
    pub delta: f32,
    pub phase: GesturePhase,
    pub modifiers: Modifiers,
}

/// A native smart-magnify request, normally a two-finger double tap on macOS.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct SmartMagnifyEvent {
    pub position: Point,
    pub modifiers: Modifiers,
}

/// Web-style `contextmenu` event delivered on secondary-button press.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ContextMenuEvent {
    pub target: ElementId,
    pub position: Point,
    pub modifiers: Modifiers,
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum Key {
    Character(String),
    ArrowUp,
    ArrowDown,
    ArrowLeft,
    ArrowRight,
    PageUp,
    PageDown,
    Home,
    End,
    Enter,
    Escape,
    Space,
    Tab,
    Backspace,
    Delete,
    Insert,
    Function(u8),
    Other,
}

/// One normalized key press delivered through the focused element path.
///
/// Key listeners run after keymap actions have had a chance to consume the keystroke and before
/// QuickGUI applies text-editing, focus-traversal, activation, or dismissal defaults.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct KeyDownEvent {
    pub key: Key,
    /// Printable character the physical press could produce before IME composition.
    pub key_char: Option<Key>,
    pub modifiers: Modifiers,
    pub repeat: bool,
}

/// One normalized key release delivered through the focused element path.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct KeyUpEvent {
    pub key: Key,
    /// Printable character the physical key could produce before IME composition.
    pub key_char: Option<Key>,
    pub modifiers: Modifiers,
}

bitflags! {
    #[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
    pub struct Modifiers: u8 {
        const SHIFT = 1 << 0;
        const CONTROL = 1 << 1;
        const ALT = 1 << 2;
        const SUPER = 1 << 3;
    }
}

/// Commands emitted while a [`crate::View`] handles an event.
#[derive(Debug, Default)]
pub struct EventContext {
    pub(crate) globals: GlobalStore,
    pub(crate) foreground_tasks: Option<ForegroundTaskSpawner>,
    pub(crate) clipboard: Option<ClipboardService>,
    pub(crate) displays: Displays,
    pub(crate) keyboard_layout: KeyboardLayout,
    pub(crate) assets: Assets,
    pub(crate) window: Option<WindowHandle>,
    pub(crate) parent_window: Option<WindowHandle>,
    pub(crate) popover_owner_window: Option<WindowHandle>,
    pub(crate) popover_root_window: Option<WindowHandle>,
    pub(crate) pointer_position: Option<Point>,
    pub(crate) invalidate: bool,
    pub(crate) exit: bool,
    pub(crate) focus: Option<Option<ElementId>>,
    pub(crate) actions: Vec<AnyAction>,
    pub(crate) targeted_actions: Vec<(WindowHandle, AnyAction)>,
    pub(crate) menus: Option<Vec<Menu>>,
    pub(crate) propagate_action: bool,
    pub(crate) stop_event_propagation: bool,
    pub(crate) prevent_default: bool,
    pub(crate) open_windows: Vec<WindowRequest>,
    pub(crate) close_current_window: bool,
    pub(crate) close_windows: Vec<WindowHandle>,
    pub(crate) focus_windows: Vec<WindowHandle>,
    pub(crate) invalidate_windows: Vec<WindowHandle>,
    pub(crate) window_commands: Vec<WindowCommand>,
    pub(crate) platform_requests: Vec<PlatformRequest>,
    pub(crate) prevent_close: bool,
    pub(crate) form_submissions: Vec<ElementId>,
    pub(crate) entity_notifications: Vec<EntityId>,
    pub(crate) notify_all_entities: bool,
    pub(crate) entity_events: Vec<EntityEvent>,
    pub(crate) global_notifications: Vec<TypeId>,
    pub(crate) notify_all_globals: bool,
}

#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct EventWindowContext {
    pub(crate) window: Option<WindowHandle>,
    pub(crate) parent: Option<WindowHandle>,
    pub(crate) popover_owner: Option<WindowHandle>,
    pub(crate) popover_root: Option<WindowHandle>,
    pub(crate) pointer_position: Option<Point>,
}

impl EventContext {
    pub(crate) fn with_runtime(
        globals: GlobalStore,
        foreground_tasks: ForegroundTaskSpawner,
        clipboard: ClipboardService,
        displays: Displays,
        keyboard_layout: KeyboardLayout,
        assets: Assets,
        window_context: EventWindowContext,
    ) -> Self {
        Self {
            globals,
            foreground_tasks: Some(foreground_tasks),
            clipboard: Some(clipboard),
            displays,
            keyboard_layout,
            assets,
            window: window_context.window,
            parent_window: window_context.parent,
            popover_owner_window: window_context.popover_owner,
            popover_root_window: window_context.popover_root,
            pointer_position: window_context.pointer_position,
            invalidate: false,
            exit: false,
            focus: None,
            actions: Vec::new(),
            targeted_actions: Vec::new(),
            menus: None,
            propagate_action: false,
            stop_event_propagation: false,
            prevent_default: false,
            open_windows: Vec::new(),
            close_current_window: false,
            close_windows: Vec::new(),
            focus_windows: Vec::new(),
            invalidate_windows: Vec::new(),
            window_commands: Vec::new(),
            platform_requests: Vec::new(),
            prevent_close: false,
            form_submissions: Vec::new(),
            entity_notifications: Vec::new(),
            notify_all_entities: false,
            entity_events: Vec::new(),
            global_notifications: Vec::new(),
            notify_all_globals: false,
        }
    }

    /// Read the latest bounded display snapshot without polling the operating system.
    pub fn displays(&self) -> &[Display] {
        self.displays.all()
    }

    pub fn primary_display(&self) -> Option<&Display> {
        self.displays.primary()
    }

    pub fn find_display(&self, id: DisplayId) -> Option<&Display> {
        self.displays.find(id)
    }

    /// Read the latest immutable keyboard-layout snapshot without querying the platform.
    pub fn keyboard_layout(&self) -> &KeyboardLayout {
        &self.keyboard_layout
    }

    /// Access the application's immutable asset source.
    pub fn assets(&self) -> &Assets {
        &self.assets
    }

    /// GPUI-shaped alias for [`Self::assets`].
    pub fn asset_source(&self) -> &Assets {
        self.assets()
    }

    /// Read a bounded item from the operating system's general clipboard.
    ///
    /// The operation is synchronous and belongs on QuickGUI's application thread. macOS checks
    /// native NSData lengths before copying text or encoded image bytes into Rust-owned storage.
    pub fn read_from_clipboard(&self) -> Result<Option<ClipboardItem>, ClipboardError> {
        self.clipboard
            .as_ref()
            .ok_or(ClipboardError::Unavailable)?
            .read(ClipboardTarget::General)
    }

    /// Replace the operating system's general clipboard with one validated item.
    ///
    /// Writing [`ClipboardItem::default`] clears the clipboard.
    pub fn write_to_clipboard(&self, item: ClipboardItem) -> Result<(), ClipboardError> {
        self.clipboard
            .as_ref()
            .ok_or(ClipboardError::Unavailable)?
            .write(ClipboardTarget::General, item)
    }

    /// Read macOS's shared Find pasteboard without polling it.
    #[cfg(target_os = "macos")]
    pub fn read_from_find_pasteboard(&self) -> Result<Option<ClipboardItem>, ClipboardError> {
        self.clipboard
            .as_ref()
            .ok_or(ClipboardError::Unavailable)?
            .read(ClipboardTarget::Find)
    }

    /// Replace macOS's shared Find pasteboard. An empty item clears it.
    #[cfg(target_os = "macos")]
    pub fn write_to_find_pasteboard(&self, item: ClipboardItem) -> Result<(), ClipboardError> {
        self.clipboard
            .as_ref()
            .ok_or(ClipboardError::Unavailable)?
            .write(ClipboardTarget::Find, item)
    }

    /// Run a non-blocking future for the current window on the application thread.
    ///
    /// Annotate the callback context with the current view type so async updates remain typed:
    /// `cx.spawn(|cx: AsyncViewContext<MyView>| async move { ... })`. Dropping the returned
    /// [`Task`] cancels it; detaching keeps it alive until completion or window close.
    pub fn spawn<V, Build, Fut, R>(&self, build: Build) -> Result<Task<R>, ForegroundTaskSpawnError>
    where
        V: 'static,
        Build: FnOnce(AsyncViewContext<V>) -> Fut,
        Fut: Future<Output = R> + 'static,
        R: 'static,
    {
        let foreground_tasks = self
            .foreground_tasks
            .as_ref()
            .ok_or(ForegroundTaskSpawnError::Unavailable)?;
        let window = self.window.ok_or(ForegroundTaskSpawnError::Unavailable)?;
        foreground_tasks.spawn::<V, _, _, _>(window, build)
    }

    /// Mark the window dirty. Calls are coalesced into a single redraw.
    pub fn invalidate(&mut self) {
        self.invalidate = true;
    }

    /// Notify windows that observed this shared entity during their latest retained render.
    ///
    /// Repeated notifications for the same entity in one callback coalesce. Prefer
    /// [`Entity::update`], which performs the mutation and notification together.
    pub fn notify<T>(&mut self, entity: &Entity<T>) {
        if self.notify_all_entities || self.entity_notifications.contains(&entity.id()) {
            return;
        }
        if self.entity_notifications.len() == MAX_ENTITY_NOTIFICATIONS_PER_EVENT {
            self.entity_notifications.clear();
            self.notify_all_entities = true;
            return;
        }
        self.entity_notifications.push(entity.id());
    }

    /// Emit one typed event from an entity.
    ///
    /// Delivery is deferred until the current callback releases all application borrows, then
    /// runs in FIFO order across subscribing windows. Unlike state notifications, events are not
    /// coalesced. `false` means this callback reached its hard event-count limit.
    pub fn emit<T, E>(&mut self, entity: &Entity<T>, event: E) -> bool
    where
        T: EventEmitter<E>,
        E: Any,
    {
        if self.entity_events.len() == MAX_ENTITY_EVENTS_PER_CALLBACK {
            return false;
        }
        self.entity_events.push(EntityEvent::new(entity, event));
        true
    }

    /// Whether an application-global value of this type has been installed.
    pub fn has_global<G: Global>(&self) -> bool {
        self.globals.has::<G>()
    }

    /// Read an application-global value without subscribing the current window.
    ///
    /// Panics if the value has not been installed. Use [`Self::try_global`] for an optional read.
    pub fn global<G: Global>(&self) -> Ref<'_, G> {
        self.globals.get::<G>()
    }

    /// Read an application-global value if one has been installed.
    pub fn try_global<G: Global>(&self) -> Option<Ref<'_, G>> {
        self.globals.try_get::<G>()
    }

    /// Mutably access an application-global value and notify its observers after this callback.
    ///
    /// Like GPUI, requesting mutable access is treated as a change. The returned guard is
    /// main-thread-only and fails loudly if application code attempts an overlapping global borrow.
    pub fn global_mut<G: Global>(&mut self) -> RefMut<'_, G> {
        self.note_global_changed(TypeId::of::<G>());
        self.globals.get_mut::<G>()
    }

    /// Mutably access a global, inserting its default value if it is not installed yet.
    pub fn default_global<G: Global + Default>(&mut self) -> RefMut<'_, G> {
        self.note_global_changed(TypeId::of::<G>());
        self.globals.default_mut::<G>()
    }

    /// Install or replace one application-global value and notify observers after this callback.
    pub fn set_global<G: Global>(&mut self, global: G) {
        self.globals.set(global);
        self.note_global_changed(TypeId::of::<G>());
    }

    /// Mutate one global inside a scoped borrow and return the callback result.
    pub fn update_global<G: Global, R>(&mut self, update: impl FnOnce(&mut G) -> R) -> R {
        self.note_global_changed(TypeId::of::<G>());
        let mut global = self.globals.get_mut::<G>();
        update(&mut global)
    }

    /// Remove and return one application-global value, notifying observers after this callback.
    pub fn remove_global<G: Global>(&mut self) -> G {
        let global = self.globals.remove::<G>();
        self.note_global_changed(TypeId::of::<G>());
        global
    }

    fn note_global_changed(&mut self, global_type: TypeId) {
        if self.notify_all_globals || self.global_notifications.contains(&global_type) {
            return;
        }
        if self.global_notifications.len() == MAX_GLOBAL_NOTIFICATIONS_PER_EVENT {
            self.global_notifications.clear();
            self.notify_all_globals = true;
            return;
        }
        self.global_notifications.push(global_type);
    }

    /// Ask the application event loop to exit cleanly.
    ///
    /// Every owned native window is torn down child-first, foreground work is cancelled, and the
    /// application-level window-closed callback runs before the event loop terminates.
    pub fn exit(&mut self) {
        self.exit = true;
    }

    /// Create another native window hosting an independently retained view.
    ///
    /// The stable handle is available immediately, before the platform window is mounted.
    pub fn open_window<V: View>(&mut self, view: V, options: WindowOptions) -> WindowHandle {
        let request = WindowRequest::with_parent(view, options, self.window);
        let handle = request.handle;
        self.open_windows.push(request);
        handle
    }

    /// Open a parent-owned native popover anchored to the latest retained bounds of an element.
    ///
    /// Unlike an in-window overlay, this popover owns a separate native window and WGPU surface, so
    /// it may extend beyond the parent window while the platform constrains it to the display work
    /// area. The anchor is resolved after the listener returns and before any invalidated rebuild;
    /// it also remains the parent's focus-restoration target until the child closes. No geometry
    /// observer, polling task, or hard-coded duplicate rectangle is required.
    pub fn open_system_popover<V: View>(
        &mut self,
        anchor: impl Into<ElementId>,
        view: V,
        options: WindowOptions,
    ) -> Result<WindowHandle, WindowCommandError> {
        if self.window.is_none() {
            return Err(WindowCommandError::Unavailable);
        }
        if options.kind != crate::WindowKind::SystemPopover || options.popover.is_none() {
            return Err(WindowCommandError::InvalidPopoverConfiguration);
        }
        let mut request = WindowRequest::with_parent(view, options, self.window);
        request.popover_anchor_element = Some(anchor.into());
        let handle = request.handle;
        self.open_windows.push(request);
        Ok(handle)
    }

    pub fn window_handle(&self) -> Option<WindowHandle> {
        self.window
    }

    /// Latest logical pointer position in the current native window, when the pointer is inside.
    ///
    /// The value is captured from the input event being delivered and never polls the platform.
    /// It is therefore safe to use from hover callbacks, whose compact payload contains only the
    /// entered/exited state.
    pub const fn pointer_position(&self) -> Option<Point> {
        self.pointer_position
    }

    fn current_window_handle(&self) -> Result<WindowHandle, WindowCommandError> {
        self.window.ok_or(WindowCommandError::Unavailable)
    }

    fn push_window_command(&mut self, command: WindowCommand) -> Result<(), WindowCommandError> {
        if self.window_commands.len() == crate::MAX_WINDOW_COMMANDS_PER_EVENT {
            return Err(WindowCommandError::QueueFull);
        }
        self.window_commands.push(command);
        Ok(())
    }

    fn push_platform_request(&mut self, request: PlatformRequest) -> Result<(), PlatformError> {
        if self.platform_requests.len() == crate::MAX_PLATFORM_REQUESTS_PER_EVENT {
            return Err(PlatformError::QueueFull);
        }
        self.platform_requests.push(request);
        Ok(())
    }

    fn ensure_platform_capacity(&self) -> Result<(), PlatformError> {
        if self.platform_requests.len() == crate::MAX_PLATFORM_REQUESTS_PER_EVENT {
            Err(PlatformError::QueueFull)
        } else {
            Ok(())
        }
    }

    /// Present a native prompt owned by the current window.
    ///
    /// The returned future resolves to the selected button index. Construct it in an event
    /// callback, then await it from [`Self::spawn`]; the native sheet performs no redraw polling.
    pub fn prompt(
        &mut self,
        level: PromptLevel,
        message: impl Into<Arc<str>>,
        detail: Option<&str>,
        buttons: &[PromptButton],
    ) -> Result<PlatformResponse<usize>, PlatformError> {
        self.ensure_platform_capacity()?;
        let window = self.window.ok_or(PlatformError::Unavailable)?;
        let (request, response) =
            PlatformRequest::prompt(window, level, message, detail.map(Arc::from), buttons)?;
        self.push_platform_request(request)?;
        Ok(response)
    }

    /// Present a native open panel owned by the current window.
    ///
    /// `Ok(None)` means the user cancelled. Selected paths retain their platform-native bytes and
    /// are bounded by [`crate::MAX_SELECTED_PATHS`] and
    /// [`crate::MAX_SELECTED_PATHS_TOTAL_BYTES`].
    pub fn prompt_for_paths(
        &mut self,
        options: PathPromptOptions,
    ) -> Result<PathPromptResponse, PlatformError> {
        self.ensure_platform_capacity()?;
        let window = self.window.ok_or(PlatformError::Unavailable)?;
        let (request, response) = PlatformRequest::open_paths(window, options)?;
        self.push_platform_request(request)?;
        Ok(response)
    }

    /// Present a native save panel owned by the current window.
    ///
    /// `Ok(None)` means the user cancelled.
    pub fn prompt_for_new_path(
        &mut self,
        options: SavePathOptions,
    ) -> Result<SavePathResponse, PlatformError> {
        self.ensure_platform_capacity()?;
        let window = self.window.ok_or(PlatformError::Unavailable)?;
        let (request, response) = PlatformRequest::save_path(window, options)?;
        self.push_platform_request(request)?;
        Ok(response)
    }

    /// Post or replace an operating-system notification.
    ///
    /// On macOS this requests notification authorization at most once, only after the first post.
    /// The request is rejected before retention if any text or action exceeds its public bound.
    pub fn show_system_notification(
        &mut self,
        notification: SystemNotification,
    ) -> Result<(), PlatformError> {
        self.ensure_platform_capacity()?;
        let request = PlatformRequest::show_system_notification(notification)?;
        #[cfg(target_os = "macos")]
        {
            self.push_platform_request(request)
        }
        #[cfg(not(target_os = "macos"))]
        {
            let _ = request;
            Err(PlatformError::Unsupported)
        }
    }

    /// Remove a pending or delivered operating-system notification by its stable tag.
    pub fn dismiss_system_notification(
        &mut self,
        tag: impl Into<Arc<str>>,
    ) -> Result<(), PlatformError> {
        self.ensure_platform_capacity()?;
        let request = PlatformRequest::dismiss_system_notification(tag)?;
        #[cfg(target_os = "macos")]
        {
            self.push_platform_request(request)
        }
        #[cfg(not(target_os = "macos"))]
        {
            let _ = request;
            Err(PlatformError::Unsupported)
        }
    }

    /// Ask the operating system to open a URL with its registered application.
    pub fn open_url(&mut self, url: impl Into<Arc<str>>) -> Result<(), PlatformError> {
        self.ensure_platform_capacity()?;
        let request = PlatformRequest::open_url(url)?;
        #[cfg(target_os = "macos")]
        {
            self.push_platform_request(request)
        }
        #[cfg(not(target_os = "macos"))]
        {
            let _ = request;
            Err(PlatformError::Unsupported)
        }
    }

    /// Ask the operating system to open a filesystem path with its default application.
    pub fn open_path(&mut self, path: impl Into<PathBuf>) -> Result<(), PlatformError> {
        self.ensure_platform_capacity()?;
        let request = PlatformRequest::open_path(path)?;
        #[cfg(target_os = "macos")]
        {
            self.push_platform_request(request)
        }
        #[cfg(not(target_os = "macos"))]
        {
            let _ = request;
            Err(PlatformError::Unsupported)
        }
    }

    /// Reveal a filesystem path in the operating system's file browser.
    pub fn reveal_path(&mut self, path: impl Into<PathBuf>) -> Result<(), PlatformError> {
        self.ensure_platform_capacity()?;
        let request = PlatformRequest::reveal_path(path)?;
        #[cfg(target_os = "macos")]
        {
            self.push_platform_request(request)
        }
        #[cfg(not(target_os = "macos"))]
        {
            let _ = request;
            Err(PlatformError::Unsupported)
        }
    }

    pub fn set_window_title(&mut self, title: impl Into<String>) -> Result<(), WindowCommandError> {
        let handle = self.current_window_handle()?;
        self.set_window_title_handle(handle, title)
    }

    pub fn set_window_title_handle(
        &mut self,
        handle: WindowHandle,
        title: impl Into<String>,
    ) -> Result<(), WindowCommandError> {
        let title = title.into();
        validate_window_title(&title)?;
        self.push_window_command(WindowCommand::SetTitle(handle, title))
    }

    /// Represent a file in the current window's native document chrome.
    pub fn set_represented_file(
        &mut self,
        path: impl Into<PathBuf>,
    ) -> Result<(), WindowCommandError> {
        let handle = self.current_window_handle()?;
        self.set_represented_file_handle(handle, path)
    }

    /// Represent a file in a target window's native document chrome.
    pub fn set_represented_file_handle(
        &mut self,
        handle: WindowHandle,
        path: impl Into<PathBuf>,
    ) -> Result<(), WindowCommandError> {
        let path = path.into();
        validate_window_document_path(&path)?;
        self.push_window_command(WindowCommand::SetRepresentedFile(handle, Some(path)))
    }

    /// GPUI-compatible alias for [`Self::set_represented_file`].
    pub fn set_document_path(&mut self, path: impl AsRef<Path>) -> Result<(), WindowCommandError> {
        self.set_represented_file(path.as_ref().to_path_buf())
    }

    /// GPUI-compatible target-window alias for [`Self::set_represented_file_handle`].
    pub fn set_document_path_handle(
        &mut self,
        handle: WindowHandle,
        path: impl AsRef<Path>,
    ) -> Result<(), WindowCommandError> {
        self.set_represented_file_handle(handle, path.as_ref().to_path_buf())
    }

    pub fn clear_represented_file(&mut self) -> Result<(), WindowCommandError> {
        let handle = self.current_window_handle()?;
        self.clear_represented_file_handle(handle)
    }

    pub fn clear_represented_file_handle(
        &mut self,
        handle: WindowHandle,
    ) -> Result<(), WindowCommandError> {
        self.push_window_command(WindowCommand::SetRepresentedFile(handle, None))
    }

    /// Set the current window's native unsaved-document indication.
    pub fn set_window_edited(&mut self, edited: bool) -> Result<(), WindowCommandError> {
        let handle = self.current_window_handle()?;
        self.set_window_edited_handle(handle, edited)
    }

    pub fn set_window_edited_handle(
        &mut self,
        handle: WindowHandle,
        edited: bool,
    ) -> Result<(), WindowCommandError> {
        self.push_window_command(WindowCommand::SetDocumentEdited(handle, edited))
    }

    /// Alias for [`Self::set_window_edited`].
    pub fn set_document_edited(&mut self, edited: bool) -> Result<(), WindowCommandError> {
        self.set_window_edited(edited)
    }

    /// Present AppKit's character palette for the current window.
    pub fn show_character_palette(&mut self) -> Result<(), WindowCommandError> {
        let handle = self.current_window_handle()?;
        self.show_character_palette_handle(handle)
    }

    pub fn show_character_palette_handle(
        &mut self,
        handle: WindowHandle,
    ) -> Result<(), WindowCommandError> {
        self.push_window_command(WindowCommand::ShowCharacterPalette(handle))
    }

    /// Opt the current window into native system tabbing.
    pub fn set_tabbing_identifier(
        &mut self,
        identifier: impl Into<String>,
    ) -> Result<(), WindowCommandError> {
        let handle = self.current_window_handle()?;
        self.set_tabbing_identifier_handle(handle, identifier)
    }

    pub fn set_tabbing_identifier_handle(
        &mut self,
        handle: WindowHandle,
        identifier: impl Into<String>,
    ) -> Result<(), WindowCommandError> {
        let identifier = identifier.into();
        validate_window_tabbing_identifier(&identifier)?;
        self.push_window_command(WindowCommand::SetTabbingIdentifier(
            handle,
            Some(identifier),
        ))
    }

    pub fn clear_tabbing_identifier(&mut self) -> Result<(), WindowCommandError> {
        let handle = self.current_window_handle()?;
        self.clear_tabbing_identifier_handle(handle)
    }

    pub fn clear_tabbing_identifier_handle(
        &mut self,
        handle: WindowHandle,
    ) -> Result<(), WindowCommandError> {
        self.push_window_command(WindowCommand::SetTabbingIdentifier(handle, None))
    }

    pub fn select_next_tab(&mut self) -> Result<(), WindowCommandError> {
        let handle = self.current_window_handle()?;
        self.select_next_tab_handle(handle)
    }

    pub fn select_next_tab_handle(
        &mut self,
        handle: WindowHandle,
    ) -> Result<(), WindowCommandError> {
        self.push_window_command(WindowCommand::SelectNextTab(handle))
    }

    pub fn select_previous_tab(&mut self) -> Result<(), WindowCommandError> {
        let handle = self.current_window_handle()?;
        self.select_previous_tab_handle(handle)
    }

    pub fn select_previous_tab_handle(
        &mut self,
        handle: WindowHandle,
    ) -> Result<(), WindowCommandError> {
        self.push_window_command(WindowCommand::SelectPreviousTab(handle))
    }

    pub fn select_tab(&mut self, index: usize) -> Result<(), WindowCommandError> {
        let handle = self.current_window_handle()?;
        self.select_tab_handle(handle, index)
    }

    pub fn select_tab_handle(
        &mut self,
        handle: WindowHandle,
        index: usize,
    ) -> Result<(), WindowCommandError> {
        if index >= MAX_SYSTEM_WINDOW_TABS {
            return Err(WindowCommandError::InvalidTabIndex);
        }
        self.push_window_command(WindowCommand::SelectTab(handle, index))
    }

    pub fn merge_all_windows(&mut self) -> Result<(), WindowCommandError> {
        let handle = self.current_window_handle()?;
        self.merge_all_windows_handle(handle)
    }

    pub fn merge_all_windows_handle(
        &mut self,
        handle: WindowHandle,
    ) -> Result<(), WindowCommandError> {
        self.push_window_command(WindowCommand::MergeAllWindows(handle))
    }

    pub fn move_tab_to_new_window(&mut self) -> Result<(), WindowCommandError> {
        let handle = self.current_window_handle()?;
        self.move_tab_to_new_window_handle(handle)
    }

    pub fn move_tab_to_new_window_handle(
        &mut self,
        handle: WindowHandle,
    ) -> Result<(), WindowCommandError> {
        self.push_window_command(WindowCommand::MoveTabToNewWindow(handle))
    }

    pub fn toggle_tab_bar(&mut self) -> Result<(), WindowCommandError> {
        let handle = self.current_window_handle()?;
        self.toggle_tab_bar_handle(handle)
    }

    pub fn toggle_tab_bar_handle(
        &mut self,
        handle: WindowHandle,
    ) -> Result<(), WindowCommandError> {
        self.push_window_command(WindowCommand::ToggleTabBar(handle))
    }

    pub fn toggle_tab_overview(&mut self) -> Result<(), WindowCommandError> {
        let handle = self.current_window_handle()?;
        self.toggle_tab_overview_handle(handle)
    }

    pub fn toggle_tab_overview_handle(
        &mut self,
        handle: WindowHandle,
    ) -> Result<(), WindowCommandError> {
        self.push_window_command(WindowCommand::ToggleTabOverview(handle))
    }

    pub fn set_window_bounds(
        &mut self,
        bounds: crate::WindowBounds,
    ) -> Result<(), WindowCommandError> {
        let handle = self.current_window_handle()?;
        self.set_window_bounds_handle(handle, bounds)
    }

    pub fn set_window_bounds_handle(
        &mut self,
        handle: WindowHandle,
        bounds: crate::WindowBounds,
    ) -> Result<(), WindowCommandError> {
        validate_window_bounds(bounds)?;
        self.push_window_command(WindowCommand::SetBounds(handle, bounds))
    }

    pub fn move_window(&mut self, position: Point) -> Result<(), WindowCommandError> {
        let handle = self.current_window_handle()?;
        self.move_window_handle(handle, position)
    }

    pub fn move_window_handle(
        &mut self,
        handle: WindowHandle,
        position: Point,
    ) -> Result<(), WindowCommandError> {
        validate_window_position(position)?;
        self.push_window_command(WindowCommand::Move(handle, position))
    }

    pub fn resize_window(&mut self, size: Size) -> Result<(), WindowCommandError> {
        let handle = self.current_window_handle()?;
        self.resize_window_handle(handle, size)
    }

    pub fn resize_window_handle(
        &mut self,
        handle: WindowHandle,
        size: Size,
    ) -> Result<(), WindowCommandError> {
        validate_window_size(size)?;
        self.push_window_command(WindowCommand::Resize(handle, size))
    }

    pub fn minimize_window(&mut self) -> Result<(), WindowCommandError> {
        let handle = self.current_window_handle()?;
        self.minimize_window_handle(handle)
    }

    pub fn minimize_window_handle(
        &mut self,
        handle: WindowHandle,
    ) -> Result<(), WindowCommandError> {
        self.push_window_command(WindowCommand::Minimize(handle))
    }

    pub fn restore_window(&mut self) -> Result<(), WindowCommandError> {
        let handle = self.current_window_handle()?;
        self.restore_window_handle(handle)
    }

    pub fn restore_window_handle(
        &mut self,
        handle: WindowHandle,
    ) -> Result<(), WindowCommandError> {
        self.push_window_command(WindowCommand::Restore(handle))
    }

    pub fn zoom_window(&mut self) -> Result<(), WindowCommandError> {
        let handle = self.current_window_handle()?;
        self.zoom_window_handle(handle)
    }

    pub fn zoom_window_handle(&mut self, handle: WindowHandle) -> Result<(), WindowCommandError> {
        self.push_window_command(WindowCommand::Zoom(handle))
    }

    pub fn toggle_fullscreen(&mut self) -> Result<(), WindowCommandError> {
        let handle = self.current_window_handle()?;
        self.toggle_fullscreen_handle(handle)
    }

    pub fn toggle_fullscreen_handle(
        &mut self,
        handle: WindowHandle,
    ) -> Result<(), WindowCommandError> {
        self.push_window_command(WindowCommand::ToggleFullscreen(handle))
    }

    pub fn set_fullscreen(&mut self, fullscreen: bool) -> Result<(), WindowCommandError> {
        let handle = self.current_window_handle()?;
        self.set_fullscreen_handle(handle, fullscreen)
    }

    pub fn set_fullscreen_handle(
        &mut self,
        handle: WindowHandle,
        fullscreen: bool,
    ) -> Result<(), WindowCommandError> {
        self.push_window_command(WindowCommand::SetFullscreen(handle, fullscreen))
    }

    pub fn show_window(&mut self) -> Result<(), WindowCommandError> {
        let handle = self.current_window_handle()?;
        self.show_window_handle(handle)
    }

    pub fn show_window_handle(&mut self, handle: WindowHandle) -> Result<(), WindowCommandError> {
        self.push_window_command(WindowCommand::SetVisible(handle, true))
    }

    pub fn hide_window(&mut self) -> Result<(), WindowCommandError> {
        let handle = self.current_window_handle()?;
        self.hide_window_handle(handle)
    }

    pub fn hide_window_handle(&mut self, handle: WindowHandle) -> Result<(), WindowCommandError> {
        self.push_window_command(WindowCommand::SetVisible(handle, false))
    }

    pub fn set_window_movable(&mut self, movable: bool) -> Result<(), WindowCommandError> {
        let handle = self.current_window_handle()?;
        self.set_window_movable_handle(handle, movable)
    }

    pub fn set_window_movable_handle(
        &mut self,
        handle: WindowHandle,
        movable: bool,
    ) -> Result<(), WindowCommandError> {
        self.push_window_command(WindowCommand::SetMovable(handle, movable))
    }

    pub fn set_window_resizable(&mut self, resizable: bool) -> Result<(), WindowCommandError> {
        let handle = self.current_window_handle()?;
        self.set_window_resizable_handle(handle, resizable)
    }

    pub fn set_window_resizable_handle(
        &mut self,
        handle: WindowHandle,
        resizable: bool,
    ) -> Result<(), WindowCommandError> {
        self.push_window_command(WindowCommand::SetResizable(handle, resizable))
    }

    /// Set the current window's minimum logical inner size.
    pub fn set_window_minimum_size(&mut self, size: Size) -> Result<(), WindowCommandError> {
        let handle = self.current_window_handle()?;
        self.set_window_minimum_size_handle(handle, size)
    }

    /// Set a target window's minimum logical inner size.
    pub fn set_window_minimum_size_handle(
        &mut self,
        handle: WindowHandle,
        size: Size,
    ) -> Result<(), WindowCommandError> {
        validate_window_size(size)?;
        self.push_window_command(WindowCommand::SetMinimumSize(handle, Some(size)))
    }

    /// Remove the current window's minimum-size constraint.
    pub fn clear_window_minimum_size(&mut self) -> Result<(), WindowCommandError> {
        let handle = self.current_window_handle()?;
        self.clear_window_minimum_size_handle(handle)
    }

    /// Remove a target window's minimum-size constraint.
    pub fn clear_window_minimum_size_handle(
        &mut self,
        handle: WindowHandle,
    ) -> Result<(), WindowCommandError> {
        self.push_window_command(WindowCommand::SetMinimumSize(handle, None))
    }

    pub fn set_window_minimizable(&mut self, minimizable: bool) -> Result<(), WindowCommandError> {
        let handle = self.current_window_handle()?;
        self.set_window_minimizable_handle(handle, minimizable)
    }

    pub fn set_window_minimizable_handle(
        &mut self,
        handle: WindowHandle,
        minimizable: bool,
    ) -> Result<(), WindowCommandError> {
        self.push_window_command(WindowCommand::SetMinimizable(handle, minimizable))
    }

    /// Force the current window's native chrome to one light/dark appearance.
    pub fn set_window_appearance(
        &mut self,
        appearance: WindowAppearance,
    ) -> Result<(), WindowCommandError> {
        let handle = self.current_window_handle()?;
        self.set_window_appearance_handle(handle, appearance)
    }

    /// Force a target window's native chrome to one light/dark appearance.
    pub fn set_window_appearance_handle(
        &mut self,
        handle: WindowHandle,
        appearance: WindowAppearance,
    ) -> Result<(), WindowCommandError> {
        self.push_window_command(WindowCommand::SetAppearance(handle, Some(appearance)))
    }

    /// Return the current window to the operating system's effective appearance.
    pub fn follow_system_window_appearance(&mut self) -> Result<(), WindowCommandError> {
        let handle = self.current_window_handle()?;
        self.follow_system_window_appearance_handle(handle)
    }

    /// Return a target window to the operating system's effective appearance.
    pub fn follow_system_window_appearance_handle(
        &mut self,
        handle: WindowHandle,
    ) -> Result<(), WindowCommandError> {
        self.push_window_command(WindowCommand::SetAppearance(handle, None))
    }

    /// Change how the native compositor treats transparent pixels in the current window.
    pub fn set_window_background_appearance(
        &mut self,
        appearance: WindowBackgroundAppearance,
    ) -> Result<(), WindowCommandError> {
        let handle = self.current_window_handle()?;
        self.set_window_background_appearance_handle(handle, appearance)
    }

    /// Change how the native compositor treats transparent pixels in a target window.
    pub fn set_window_background_appearance_handle(
        &mut self,
        handle: WindowHandle,
        appearance: WindowBackgroundAppearance,
    ) -> Result<(), WindowCommandError> {
        self.push_window_command(WindowCommand::SetBackgroundAppearance(handle, appearance))
    }

    /// Open or close the retained-tree inspector for the current window.
    #[cfg(feature = "inspector")]
    pub fn set_inspector(&mut self, open: bool) -> Result<(), WindowCommandError> {
        let handle = self.current_window_handle()?;
        self.set_inspector_handle(handle, open)
    }

    /// Open or close the retained-tree inspector for a target window.
    #[cfg(feature = "inspector")]
    pub fn set_inspector_handle(
        &mut self,
        handle: WindowHandle,
        open: bool,
    ) -> Result<(), WindowCommandError> {
        self.push_window_command(WindowCommand::SetInspector(handle, open))
    }

    /// Toggle the retained-tree inspector for the current window.
    #[cfg(feature = "inspector")]
    pub fn toggle_inspector(&mut self) -> Result<(), WindowCommandError> {
        let handle = self.current_window_handle()?;
        self.toggle_inspector_handle(handle)
    }

    /// Toggle the retained-tree inspector for a target window.
    #[cfg(feature = "inspector")]
    pub fn toggle_inspector_handle(
        &mut self,
        handle: WindowHandle,
    ) -> Result<(), WindowCommandError> {
        self.push_window_command(WindowCommand::ToggleInspector(handle))
    }

    pub fn request_window_attention(&mut self) -> Result<(), WindowCommandError> {
        let handle = self.current_window_handle()?;
        self.request_window_attention_handle(handle)
    }

    pub fn request_window_attention_handle(
        &mut self,
        handle: WindowHandle,
    ) -> Result<(), WindowCommandError> {
        self.push_window_command(WindowCommand::RequestAttention(handle))
    }

    /// Close the window currently delivering this event.
    pub fn close_window(&mut self) {
        self.close_current_window = true;
    }

    /// Close the complete system-popover chain containing the current window.
    ///
    /// Closing the first popover lets the runtime tear down every descendant child-first and gives
    /// native keyboard focus back to the nearest non-popover owner. Outside a system popover this
    /// is a no-op and returns `false`.
    pub fn close_popover_chain(&mut self) -> bool {
        let Some(root) = self.popover_root_window else {
            return false;
        };
        if let Some(owner) = self.popover_owner_window
            && !self.focus_windows.contains(&owner)
        {
            self.focus_windows.push(owner);
        }
        if Some(root) == self.window {
            self.close_current_window = true;
        } else {
            self.close_windows.push(root);
        }
        true
    }

    /// Close a window previously returned by [`Self::open_window`].
    pub fn close_window_handle(&mut self, handle: WindowHandle) {
        self.close_windows.push(handle);
    }

    /// Bring a window to the front and give it native keyboard focus.
    pub fn focus_window(&mut self, handle: WindowHandle) {
        self.focus_windows.push(handle);
    }

    /// Mark another window's view dirty and schedule one coalesced redraw.
    pub fn invalidate_window(&mut self, handle: WindowHandle) {
        self.invalidate_windows.push(handle);
    }

    /// Keep the current window open after receiving [`Event::CloseRequested`].
    pub fn prevent_close(&mut self) {
        self.prevent_close = true;
    }

    /// Move keyboard focus to a stable element handle.
    ///
    /// If the target is introduced by the view invalidation from this same event, QuickGUI keeps
    /// the request through exactly that next rebuild. A missing target is then discarded rather
    /// than becoming a persistent focus trap.
    pub fn focus(&mut self, handle: FocusHandle) {
        self.focus = Some(Some(handle.id()));
    }

    /// Clear keyboard focus within the window.
    pub fn blur(&mut self) {
        self.focus = Some(None);
    }

    /// Validate and submit a mounted form after the current callback completes.
    ///
    /// Repeated requests for the same form in one callback coalesce. The bounded queue prevents a
    /// callback from retaining unbounded work; `false` reports that the limit was reached.
    pub fn submit_form(&mut self, form: impl Into<ElementId>) -> bool {
        let form = form.into();
        if self.form_submissions.contains(&form) {
            return true;
        }
        if self.form_submissions.len() == MAX_FORM_SUBMISSIONS_PER_EVENT {
            return false;
        }
        self.form_submissions.push(form);
        true
    }

    /// Dispatch a typed action through the currently focused element path.
    ///
    /// This is useful for buttons, menus, command palettes, and native menu items that should use
    /// exactly the same command handlers as keyboard bindings.
    pub fn dispatch_action<A: Action>(&mut self, action: A) {
        self.actions.push(AnyAction::new(action));
    }

    /// Dispatch a previously type-erased action through the focused element path.
    ///
    /// Command registries and pickers can retain heterogeneous actions as [`AnyAction`] values,
    /// then restore the intended focus and dispatch the original concrete payload without a type
    /// switch or a parallel command system.
    pub fn dispatch_any_action(&mut self, action: AnyAction) {
        self.actions.push(action);
    }

    /// Dispatch a typed action through another window's focused retained path.
    ///
    /// Delivery is deferred until the current callback releases its view borrow. `false` means
    /// this callback reached the hard cross-window action bound; a target that closes before
    /// delivery is ignored safely.
    pub fn dispatch_action_to_window<A: Action>(
        &mut self,
        window: WindowHandle,
        action: A,
    ) -> bool {
        self.dispatch_any_action_to_window(window, AnyAction::new(action))
    }

    /// Dispatch a previously type-erased action through another window's focused retained path.
    pub fn dispatch_any_action_to_window(
        &mut self,
        window: WindowHandle,
        action: AnyAction,
    ) -> bool {
        if self.targeted_actions.len() == MAX_TARGETED_ACTIONS_PER_EVENT {
            return false;
        }
        self.targeted_actions.push((window, action));
        true
    }

    /// Dispatch a typed action to this native child window's parent.
    ///
    /// Returns `false` when the context is not attached to a child or the callback reached the
    /// cross-window action bound.
    pub fn dispatch_action_to_parent<A: Action>(&mut self, action: A) -> bool {
        let Some(parent) = self.parent_window else {
            return false;
        };
        self.dispatch_action_to_window(parent, action)
    }

    /// Dispatch a previously type-erased action to this native child window's parent.
    pub fn dispatch_any_action_to_parent(&mut self, action: AnyAction) -> bool {
        let Some(parent) = self.parent_window else {
            return false;
        };
        self.dispatch_any_action_to_window(parent, action)
    }

    /// The parent of the current native child window, when one exists.
    pub const fn parent_window_handle(&self) -> Option<WindowHandle> {
        self.parent_window
    }

    /// Dispatch a typed action to the nearest non-popover owner of this system popover chain.
    ///
    /// This differs from [`Self::dispatch_action_to_parent`] for nested menus: a submenu's direct
    /// parent is another popover window, while commands should reach the application window that
    /// opened the popover chain.
    pub fn dispatch_action_to_popover_owner<A: Action>(&mut self, action: A) -> bool {
        let Some(owner) = self.popover_owner_window else {
            return false;
        };
        self.dispatch_action_to_window(owner, action)
    }

    /// Dispatch a previously type-erased action to the nearest non-popover owner.
    pub fn dispatch_any_action_to_popover_owner(&mut self, action: AnyAction) -> bool {
        let Some(owner) = self.popover_owner_window else {
            return false;
        };
        self.dispatch_any_action_to_window(owner, action)
    }

    /// The nearest non-popover owner of the current system popover chain, when one exists.
    pub const fn popover_owner_window_handle(&self) -> Option<WindowHandle> {
        self.popover_owner_window
    }

    /// The first system popover below the non-popover owner of the current popover chain.
    pub const fn popover_root_window_handle(&self) -> Option<WindowHandle> {
        self.popover_root_window
    }

    /// Replace the application's native menu declaration.
    ///
    /// Use this after state changes that affect labels, checked state, or static availability.
    /// Focused action-handler availability and contextual key equivalents update automatically.
    pub fn set_menus(&mut self, menus: impl IntoIterator<Item = Menu>) {
        self.menus = Some(menus.into_iter().collect());
    }

    /// Allow the current action to continue bubbling to the next ancestor handler.
    ///
    /// Action handlers consume by default, matching GPUI's command dispatch behavior. For input
    /// and capture-phase action events that propagate by default, this also cancels an earlier
    /// [`Self::stop_propagation`] call made during the same callback.
    pub fn propagate(&mut self) {
        self.propagate_action = true;
        self.stop_event_propagation = false;
    }

    /// Stop the current input event before it reaches another listening ancestor.
    ///
    /// Input events bubble by default. Capture-phase action listeners also use this method.
    /// Stopping propagation does not suppress native default behavior; use
    /// [`Self::prevent_default`] separately when replacing retained mouse, focus, selection,
    /// drag, click, scroll, or key behavior.
    pub fn stop_propagation(&mut self) {
        self.stop_event_propagation = true;
    }

    /// Suppress the framework's default behavior for the current input event.
    ///
    /// For a [`ScrollWheelEvent`] this prevents retained scrolling. For a targeted desktop mouse
    /// press or release it suppresses the framework's focus, text-selection, click, context-menu,
    /// dismissal, and drag-start defaults. For a [`KeyDownEvent`] it suppresses text editing,
    /// focus traversal, focused activation, dismissal, and the default macOS close shortcut.
    /// Terminal pointer capture and drag cleanup still run. This does not stop propagation to
    /// another listener.
    pub fn prevent_default(&mut self) {
        self.prevent_default = true;
    }
}

#[cfg(test)]
mod tests {
    use crate::{Global, IntoElement, ViewContext, div};

    use super::*;

    struct SecondaryView;

    #[derive(Default)]
    struct TestGlobal(u32);

    impl Global for TestGlobal {}

    impl View for SecondaryView {
        fn render(&mut self, _cx: &mut ViewContext<'_, Self>) -> impl IntoElement {
            div()
        }
    }

    #[test]
    fn window_commands_keep_stable_handles_and_options() {
        let parent = WindowHandle::next();
        let mut cx = EventContext {
            window: Some(parent),
            ..EventContext::default()
        };
        let first = cx.open_window(
            SecondaryView,
            WindowOptions::new("First").size(480.0, 320.0),
        );
        let second = cx.open_window(SecondaryView, WindowOptions::new("Second"));

        assert_ne!(first, second);
        assert_eq!(cx.open_windows[0].handle, first);
        assert_eq!(cx.open_windows[0].options.title, "First");
        assert_eq!(cx.open_windows[0].options.size, Size::new(480.0, 320.0));
        assert_eq!(cx.open_windows[0].parent, Some(parent));
        assert_eq!(cx.open_windows[1].parent, Some(parent));
        cx.focus_window(first);
        cx.invalidate_window(second);
        cx.close_window_handle(first);
        cx.close_window();
        cx.prevent_close();
        assert_eq!(cx.focus_windows, [first]);
        assert_eq!(cx.invalidate_windows, [second]);
        assert_eq!(cx.close_windows, [first]);
        assert!(cx.close_current_window);
        assert!(cx.prevent_close);
    }

    #[test]
    fn closing_popover_chain_restores_the_non_popover_owner_focus_once() {
        let owner = WindowHandle::next();
        let root = WindowHandle::next();
        let child = WindowHandle::next();
        let mut cx = EventContext {
            window: Some(child),
            popover_owner_window: Some(owner),
            popover_root_window: Some(root),
            ..EventContext::default()
        };

        assert!(cx.close_popover_chain());
        assert!(cx.close_popover_chain());
        assert_eq!(cx.focus_windows, [owner]);
        assert_eq!(cx.close_windows, [root, root]);
        assert!(!cx.close_current_window);

        let mut root_context = EventContext {
            window: Some(root),
            popover_owner_window: Some(owner),
            popover_root_window: Some(root),
            ..EventContext::default()
        };
        assert!(root_context.close_popover_chain());
        assert_eq!(root_context.focus_windows, [owner]);
        assert!(root_context.close_current_window);

        let mut ordinary_window = EventContext::default();
        assert!(!ordinary_window.close_popover_chain());
        assert!(ordinary_window.focus_windows.is_empty());
    }

    #[test]
    fn window_mutation_queue_is_bounded_and_validates_before_retaining() {
        let window = WindowHandle::next();
        let mut cx = EventContext {
            window: Some(window),
            ..EventContext::default()
        };

        assert_eq!(
            cx.set_window_title("x".repeat(crate::MAX_WINDOW_TITLE_BYTES + 1)),
            Err(WindowCommandError::TitleTooLong)
        );
        assert!(cx.window_commands.is_empty());

        for _ in 0..crate::MAX_WINDOW_COMMANDS_PER_EVENT {
            assert!(cx.request_window_attention().is_ok());
        }
        assert_eq!(
            cx.request_window_attention(),
            Err(WindowCommandError::QueueFull)
        );
        assert_eq!(
            cx.window_commands.len(),
            crate::MAX_WINDOW_COMMANDS_PER_EVENT
        );
        assert!(
            cx.window_commands
                .iter()
                .all(|command| command.handle() == window)
        );
    }

    #[test]
    fn document_window_commands_validate_and_retain_exact_native_intent() {
        let window = WindowHandle::next();
        let mut cx = EventContext {
            window: Some(window),
            ..EventContext::default()
        };

        assert_eq!(
            cx.set_document_path(PathBuf::new()),
            Err(WindowCommandError::InvalidDocumentPath)
        );
        assert_eq!(
            cx.set_tabbing_identifier(""),
            Err(WindowCommandError::InvalidTabbingIdentifier)
        );
        assert_eq!(
            cx.select_tab(MAX_SYSTEM_WINDOW_TABS),
            Err(WindowCommandError::InvalidTabIndex)
        );
        assert!(cx.window_commands.is_empty());

        cx.set_document_path("Cargo.toml").unwrap();
        cx.set_window_edited(true).unwrap();
        cx.show_character_palette().unwrap();
        cx.set_tabbing_identifier("dev.quickgui.workspace").unwrap();
        cx.select_next_tab().unwrap();
        cx.select_previous_tab().unwrap();
        cx.select_tab(7).unwrap();
        cx.merge_all_windows().unwrap();
        cx.move_tab_to_new_window().unwrap();
        cx.toggle_tab_bar().unwrap();
        cx.toggle_tab_overview().unwrap();
        cx.clear_represented_file().unwrap();
        cx.clear_tabbing_identifier().unwrap();

        assert_eq!(cx.window_commands.len(), 13);
        assert!(matches!(
            &cx.window_commands[0],
            WindowCommand::SetRepresentedFile(handle, Some(path))
                if *handle == window && path == &PathBuf::from("Cargo.toml")
        ));
        assert!(matches!(
            &cx.window_commands[1],
            WindowCommand::SetDocumentEdited(handle, true) if *handle == window
        ));
        assert!(matches!(
            &cx.window_commands[3],
            WindowCommand::SetTabbingIdentifier(handle, Some(identifier))
                if *handle == window && identifier == "dev.quickgui.workspace"
        ));
        assert!(matches!(
            &cx.window_commands[11],
            WindowCommand::SetRepresentedFile(handle, None) if *handle == window
        ));
        assert!(matches!(
            &cx.window_commands[12],
            WindowCommand::SetTabbingIdentifier(handle, None) if *handle == window
        ));
    }

    #[test]
    fn current_window_commands_fail_without_a_native_owner() {
        let mut cx = EventContext::default();

        assert_eq!(
            cx.resize_window(Size::new(640.0, 480.0)),
            Err(WindowCommandError::Unavailable)
        );
        assert!(cx.window_commands.is_empty());
    }

    #[test]
    fn platform_requests_are_bounded_and_validate_before_retaining() {
        let window = WindowHandle::next();
        let mut cx = EventContext {
            window: Some(window),
            ..EventContext::default()
        };

        assert_eq!(cx.open_url(""), Err(PlatformError::InvalidUrl));
        assert!(cx.platform_requests.is_empty());

        for index in 0..crate::MAX_PLATFORM_REQUESTS_PER_EVENT {
            assert!(cx.open_url(format!("https://example.com/{index}")).is_ok());
        }
        assert_eq!(
            cx.open_url("https://example.com/overflow"),
            Err(PlatformError::QueueFull)
        );
        assert_eq!(
            cx.platform_requests.len(),
            crate::MAX_PLATFORM_REQUESTS_PER_EVENT
        );
    }

    #[test]
    fn native_dialogs_require_a_window_owner() {
        let mut cx = EventContext::default();
        assert!(matches!(
            cx.prompt(
                PromptLevel::Info,
                "Message",
                None,
                &[PromptButton::ok("OK")],
            ),
            Err(PlatformError::Unavailable)
        ));
        assert!(matches!(
            cx.prompt_for_paths(PathPromptOptions::new()),
            Err(PlatformError::Unavailable)
        ));
        assert!(cx.platform_requests.is_empty());
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn system_notifications_are_app_wide_but_still_bounded() {
        let mut cx = EventContext::default();
        assert_eq!(
            cx.show_system_notification(SystemNotification::new("", "Title", "Body")),
            Err(PlatformError::InvalidNotificationTag)
        );
        assert!(cx.platform_requests.is_empty());

        assert!(
            cx.show_system_notification(SystemNotification::new(
                "background-job",
                "Finished",
                "The export is ready",
            ))
            .is_ok()
        );
        assert!(cx.dismiss_system_notification("background-job").is_ok());
        assert_eq!(cx.platform_requests.len(), 2);
    }

    #[test]
    fn event_context_globals_are_typed_and_changes_coalesce() {
        let mut cx = EventContext::default();
        assert!(!cx.has_global::<TestGlobal>());
        cx.set_global(TestGlobal(2));
        cx.update_global::<TestGlobal, _>(|global| global.0 += 3);
        cx.global_mut::<TestGlobal>().0 += 5;

        assert_eq!(cx.global::<TestGlobal>().0, 10);
        assert_eq!(cx.global_notifications, [TypeId::of::<TestGlobal>()]);
        assert!(!cx.notify_all_globals);
        assert_eq!(cx.remove_global::<TestGlobal>().0, 10);
        assert!(!cx.has_global::<TestGlobal>());
        assert_eq!(cx.global_notifications, [TypeId::of::<TestGlobal>()]);
    }

    #[test]
    fn too_many_global_changes_fall_back_to_notify_all() {
        let mut cx = EventContext {
            global_notifications: vec![
                TypeId::of::<TestGlobal>();
                MAX_GLOBAL_NOTIFICATIONS_PER_EVENT
            ],
            ..EventContext::default()
        };
        cx.note_global_changed(TypeId::of::<SecondaryView>());

        assert!(cx.global_notifications.is_empty());
        assert!(cx.notify_all_globals);
    }

    #[test]
    fn native_file_payloads_have_a_hard_path_count_bound() {
        let files = DroppedFiles::new(
            (0..=MAX_DROPPED_FILES).map(|index| PathBuf::from(format!("file-{index}"))),
        );

        assert_eq!(files.paths().len(), MAX_DROPPED_FILES);
        assert_eq!(files.paths().first(), Some(&PathBuf::from("file-0")));
        assert_eq!(
            files.paths().last(),
            Some(&PathBuf::from(format!("file-{}", MAX_DROPPED_FILES - 1)))
        );
        assert!(files.is_truncated());
    }

    #[test]
    fn outbound_file_payloads_bound_count_and_path_storage() {
        let files = FileDragPaths::new(
            (0..=MAX_EXTERNAL_DRAG_FILES)
                .map(|index| (PathBuf::from(format!("file-{index}")), index % 2 == 0)),
        );

        assert_eq!(files.entries().len(), MAX_EXTERNAL_DRAG_FILES);
        assert!(files.is_truncated());

        let oversized = PathBuf::from("x".repeat(MAX_EXTERNAL_DRAG_PATH_BYTES + 1));
        let files = FileDragPaths::new([
            (PathBuf::new(), false),
            (oversized, false),
            (PathBuf::from("kept.txt"), false),
        ]);
        assert_eq!(files.entries(), [(PathBuf::from("kept.txt"), false)]);
        assert!(files.is_truncated());

        assert_eq!(
            FileDragPaths::files([PathBuf::from("file.txt")]).entries(),
            [(PathBuf::from("file.txt"), false)]
        );
        assert_eq!(
            FileDragPaths::directories([PathBuf::from("folder")]).entries(),
            [(PathBuf::from("folder"), true)]
        );
    }

    #[test]
    fn outbound_text_and_url_payloads_are_utf8_safe_and_bounded() {
        let shared: Arc<str> = Arc::from("shared native drag text");
        let shared_text = ExternalDragText::new(Arc::clone(&shared));
        assert!(Arc::ptr_eq(&shared_text.text, &shared));

        let text =
            ExternalDragText::new(format!("{}é", "x".repeat(MAX_EXTERNAL_DRAG_TEXT_BYTES - 1)));
        assert_eq!(text.as_str().len(), MAX_EXTERNAL_DRAG_TEXT_BYTES - 1);
        assert!(text.is_truncated());
        assert!(text.as_str().is_char_boundary(text.as_str().len()));

        let shared_url: Arc<str> = Arc::from("https://example.com/路径?q=quickgui");
        let url = ExternalDragUrl::new(Arc::clone(&shared_url)).unwrap();
        assert!(Arc::ptr_eq(&url.0, &shared_url));
        assert_eq!(url.as_str(), "https://example.com/路径?q=quickgui");
        assert_eq!(
            ExternalDragUrl::new("relative/path").unwrap_err(),
            ExternalDragUrlError::InvalidScheme
        );
        assert_eq!(
            ExternalDragUrl::new("https://example.com/a b").unwrap_err(),
            ExternalDragUrlError::InvalidCharacter
        );
        assert!(matches!(
            ExternalDragUrl::new(format!(
                "https://example.com/{}",
                "x".repeat(MAX_EXTERNAL_DRAG_URL_BYTES)
            )),
            Err(ExternalDragUrlError::TooLarge { .. })
        ));
    }

    #[test]
    fn scroll_deltas_preserve_precision_and_bound_platform_values() {
        let pixels = ScrollDelta::Pixels(Vector::new(12.5, -24.0)).bounded();
        assert!(pixels.precise());
        assert_eq!(pixels.pixel_delta(40.0), Vector::new(12.5, -24.0));

        let lines = ScrollDelta::Lines(Vector::new(2.0, -3.0)).bounded();
        assert!(!lines.precise());
        assert_eq!(lines.pixel_delta(32.0), Vector::new(64.0, -96.0));

        assert_eq!(
            ScrollDelta::Pixels(Vector::new(f32::NAN, f32::INFINITY))
                .bounded()
                .pixel_delta(40.0),
            Vector::ZERO
        );
        assert_eq!(
            ScrollDelta::Lines(Vector::new(MAX_SCROLL_LINES_PER_EVENT * 2.0, -1.0))
                .bounded()
                .pixel_delta(f32::INFINITY),
            Vector::ZERO
        );
        assert_eq!(
            ScrollDelta::Pixels(Vector::new(MAX_SCROLL_PIXELS_PER_EVENT * 2.0, 0.0))
                .bounded()
                .pixel_delta(40.0),
            Vector::new(MAX_SCROLL_PIXELS_PER_EVENT, 0.0)
        );
    }

    #[test]
    fn input_propagation_and_default_prevention_are_independent() {
        let mut cx = EventContext::default();
        cx.stop_propagation();
        assert!(cx.stop_event_propagation);
        assert!(!cx.prevent_default);

        cx.prevent_default();
        assert!(cx.stop_event_propagation);
        assert!(cx.prevent_default);

        cx.propagate();
        assert!(!cx.stop_event_propagation);
        assert!(cx.prevent_default);
        assert!(cx.propagate_action);
    }

    #[test]
    fn raw_touch_samples_are_finite_and_pressure_bounded() {
        let event = TouchEvent {
            id: TouchId(42),
            phase: TouchPhase::Moved,
            position: Point::new(f32::INFINITY, -MAX_TOUCH_COORDINATE * 2.0),
            force: Some(1.5),
        }
        .bounded();

        assert_eq!(event.id, TouchId(42));
        assert_eq!(event.position, Point::new(0.0, -MAX_TOUCH_COORDINATE));
        assert_eq!(event.force, Some(1.0));
        assert_eq!(
            TouchEvent {
                force: Some(f32::NAN),
                ..TouchEvent::default()
            }
            .bounded()
            .force,
            Some(0.0)
        );
    }

    #[test]
    fn erased_command_registry_actions_keep_their_original_payload() {
        #[derive(Clone, Debug, Eq, PartialEq)]
        struct OpenLine(usize);

        let mut cx = EventContext::default();
        cx.dispatch_any_action(AnyAction::new(OpenLine(42)));

        assert_eq!(cx.actions.len(), 1);
        assert_eq!(
            cx.actions[0].downcast_ref::<OpenLine>(),
            Some(&OpenLine(42))
        );
    }

    #[test]
    fn cross_window_actions_are_parent_aware_and_hard_bounded() {
        #[derive(Clone, Debug, Eq, PartialEq)]
        struct OpenLine(usize);

        let parent = WindowHandle::next();
        let sibling = WindowHandle::next();
        let mut cx = EventContext {
            window: Some(WindowHandle::next()),
            parent_window: Some(parent),
            ..EventContext::default()
        };
        assert_eq!(cx.parent_window_handle(), Some(parent));
        assert!(cx.dispatch_action_to_parent(OpenLine(7)));
        assert!(cx.dispatch_action_to_window(sibling, OpenLine(9)));
        assert_eq!(cx.targeted_actions.len(), 2);
        assert_eq!(cx.targeted_actions[0].0, parent);
        assert_eq!(
            cx.targeted_actions[0].1.downcast_ref::<OpenLine>(),
            Some(&OpenLine(7))
        );
        assert_eq!(cx.targeted_actions[1].0, sibling);

        cx.targeted_actions.clear();
        for line in 0..MAX_TARGETED_ACTIONS_PER_EVENT {
            assert!(cx.dispatch_action_to_parent(OpenLine(line)));
        }
        assert!(!cx.dispatch_action_to_parent(OpenLine(usize::MAX)));
        assert_eq!(cx.targeted_actions.len(), MAX_TARGETED_ACTIONS_PER_EVENT);

        let mut root = EventContext::default();
        assert!(!root.dispatch_action_to_parent(OpenLine(1)));
        assert!(root.targeted_actions.is_empty());
    }

    #[test]
    fn programmatic_form_submissions_coalesce_and_stay_bounded() {
        let mut cx = EventContext::default();
        assert!(cx.submit_form("profile"));
        assert!(cx.submit_form("profile"));
        assert_eq!(cx.form_submissions, [ElementId::named("profile")]);

        for index in 1..MAX_FORM_SUBMISSIONS_PER_EVENT {
            assert!(cx.submit_form(index));
        }
        assert_eq!(cx.form_submissions.len(), MAX_FORM_SUBMISSIONS_PER_EVENT);
        assert!(!cx.submit_form(usize::MAX));
    }
}
