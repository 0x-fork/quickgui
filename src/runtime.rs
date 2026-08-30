#[cfg(target_os = "macos")]
use std::sync::atomic::AtomicBool;
use std::{
    any::{Any, TypeId},
    cell::{Ref, RefCell},
    collections::{HashMap, HashSet, VecDeque},
    fmt,
    future::Future,
    marker::PhantomData,
    path::{Path, PathBuf},
    rc::Rc,
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
    time::{Duration, Instant},
};

use accesskit::{Action as AccessibilityAction, ActionData, ActionRequest};
use accesskit_winit::{
    Adapter as AccessibilityAdapter, Event as AccessibilityEvent,
    WindowEvent as AccessibilityWindowEvent,
};
use thiserror::Error;
#[cfg(target_os = "macos")]
use winit::platform::macos::{ActiveEventLoopExtMacOS, WindowAttributesExtMacOS, WindowExtMacOS};
#[cfg(not(target_arch = "wasm32"))]
use winit::platform::pump_events::{EventLoopExtPumpEvents, PumpStatus};
#[cfg(target_os = "windows")]
use winit::platform::windows::{WindowAttributesExtWindows, WindowExtWindows};
use winit::{
    application::ApplicationHandler,
    dpi::{LogicalPosition, LogicalSize, PhysicalSize},
    event::{ElementState, Force, Ime, MouseScrollDelta, WindowEvent},
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop, EventLoopProxy},
    keyboard::ModifiersState,
    window::{
        CursorGrabMode as WinitCursorGrabMode, CursorIcon, Fullscreen, Icon, Theme,
        UserAttentionType, Window, WindowButtons, WindowId, WindowLevel as WinitWindowLevel,
    },
};
#[cfg(not(target_os = "macos"))]
use winit::{dpi::PhysicalPosition, raw_window_handle::HasWindowHandle};

#[cfg(any(
    target_os = "macos",
    target_os = "windows",
    target_os = "linux",
    target_os = "freebsd",
    target_os = "dragonfly",
    target_os = "openbsd",
    target_os = "netbsd"
))]
use crate::platform::PlatformDialogId;
use crate::{
    AboutPanelOptions, Action, ActionListener, AnyAction, AppInfo, AppPaths, AssetError, Assets,
    Color, ColorScheme, CursorStyle, Display, DisplayId, Displays, Element, ElementId, Entity,
    EntityId, EventEmitter, FileIconResponse, FileIconSize, FocusHandle, FontSource, Global, Image,
    IntoElement, KeyBinding, KeyboardLayout, Keymap, Keystroke,
    MAX_ENTITY_EVENT_DELIVERIES_PER_TURN, MAX_ENTITY_SUBSCRIPTIONS_PER_WINDOW,
    MAX_GLOBAL_OBSERVER_DELIVERIES_PER_TURN, MAX_GLOBAL_SUBSCRIPTIONS_PER_WINDOW,
    MAX_OBSERVED_ENTITIES_PER_WINDOW, MAX_OBSERVED_GLOBALS_PER_WINDOW, MAX_PENDING_ENTITY_EVENTS,
    MAX_PENDING_GLOBAL_NOTIFICATIONS, MAX_TASKBAR_OVERLAY_DESCRIPTION_BYTES, Menu,
    NotificationPermissionResponse, NotificationPermissionStatus, OpenUrls, OsAction, Point,
    PowerSource, Rect, RelaunchOptions, RelaunchRequest, RelaunchedProcess, Scene, Size,
    SystemInfo, SystemNotification, SystemNotificationResponse, SystemPreferences, ThermalState,
    UserTask, Vector,
    action::{ActionListenerBinding, ActionListenerKey},
    background::{
        BackgroundCompletion, BackgroundTaskError, BackgroundTaskPoolHandle, TaskSpawnError,
    },
    clipboard::{ClipboardItem, ClipboardService, ClipboardTarget},
    element::{
        KeyListenerBinding, KeyListenerKey, KeyListenerKind, MouseListenerKey, MouseListenerKind,
    },
    entity::{EntityEvent, Subscription, SubscriptionState},
    event::{
        ContextMenuEvent, DragOrigin, DragStartEvent, DropEvent, DroppedFiles, Event, EventContext,
        EventRuntimeContext, ExternalDragPayload, ExternalDragText, ExternalDragUrl, FileDragPaths,
        FormSubmitEvent, GesturePhase, Key, KeyDownEvent, KeyUpEvent,
        MAX_ACTIVE_TOUCHES_PER_WINDOW, MAX_DROPPED_FILES, MAX_PINCH_DELTA_PER_EVENT,
        MAX_ROTATION_DEGREES_PER_EVENT, Modifiers, MouseButton, MouseDownEvent, MouseExitEvent,
        MouseMoveEvent, MousePressureEvent, MouseUpEvent, PinchEvent, PointerEvent, PointerPhase,
        PressureStage, RotationEvent, ScrollDelta, ScrollWheelEvent, SmartMagnifyEvent, TouchEvent,
        TouchId, TouchPhase, ValidationReport,
    },
    foreground::{
        AsyncViewContext, ForegroundTaskSpawnError, ForegroundTaskSpawner, ScheduledForegroundTask,
        Task,
    },
    global::GlobalStore,
    image_resource::{ImageAssetCache, ImageLoadCompletion, ImageWorkerPoolHandle},
    keyboard::KeyboardState,
    menu::{MenuAction, collect_menu_actions, validate_menus},
    metrics::{FrameMetrics, FrameTimer, MetricsTracker},
    platform::{
        PathPromptOptions, PathPromptResponse, PlatformError, PlatformRequest, PlatformResponse,
        PromptButton, PromptLevel, SavePathOptions, SavePathResponse, ShellResponse,
    },
    renderer::{
        GpuContext, GpuRenderer, RenderOutcome, SharedFontSystem, create_shared_font_system,
    },
    scheduler::FrameScheduler,
    ui_tree::{
        DismissRequest, FormAttempt, InputResult, MouseHoverChange, TabNavigationTarget, UiTree,
    },
};

#[cfg(feature = "inspector")]
use crate::inspector::{
    InspectorFrameDamage, InspectorMode, InspectorPointerAction, InspectorState,
};

const MAX_NESTED_FORM_SUBMISSIONS: u8 = 8;
const MAX_WINDOW_LIFECYCLE_TURNS: usize = 1_024;
const ACCESSIBILITY_GEOMETRY_UPDATE_INTERVAL: Duration = Duration::from_millis(100);

/// Maximum targeted desktop mouse callbacks declared by one window render.
pub const MAX_MOUSE_LISTENERS_PER_WINDOW: usize = 8_192;

/// Maximum focused key callbacks declared by one window render.
pub const MAX_KEY_LISTENERS_PER_WINDOW: usize = 4_096;

/// Maximum typed action callbacks declared by one window render.
pub const MAX_ACTION_LISTENERS_PER_WINDOW: usize = 4_096;

#[cfg(target_os = "macos")]
use crate::event::{ExternalDragEndEvent, ExternalDragOperation};
#[cfg(target_os = "macos")]
use crate::macos::{
    MacExternalDragMonitor, MacExternalDragSession, MacFirstFrameGuard, MacMouseDownEvent,
    MacNativeDropHost, MacNativeDropOffer, MacNativeDropPayload, MacNativeDropPending,
    MacNativeHost, MacPlatformDialog, MacPlatformDialogContext, MacPopoverMonitor,
    MacTypedDragPayload, MacTypedDragRegistry, MacWindowTabAction, capture_left_mouse_down,
    configure_document_window, configure_gpu_window_resize, configure_window_kind,
    current_cursor_screen_position as macos_cursor_screen_position, current_pointer_position,
    dismiss_window_relation, is_window_fullscreen, is_window_maximized, perform_window_close,
    perform_window_drag, perform_window_tab_action, position_system_popover,
    position_traffic_lights, present_native_open_panel, present_native_prompt,
    present_native_save_panel, present_window_relation, set_window_document_edited,
    set_window_focusable, set_window_movable, set_window_opacity, set_window_represented_file,
    set_window_tabbing_identifier, set_window_visibility, set_window_visible_on_all_workspaces,
    shell_open_path, shell_open_url, shell_reveal_path, shell_trash_path, show_character_palette,
    start_external_drag, window_tab_state,
};
#[cfg(target_os = "macos")]
use crate::macos_application::MacApplicationHost;
#[cfg(target_os = "macos")]
use crate::macos_menu::{MacMenuHost, MacMenuItemState};

pub(crate) enum RuntimeEvent {
    ExternalCommandsReady,
    InvalidateWindow(WindowHandle),
    Accessibility(AccessibilityEvent),
    ImageLoaded(WindowHandle, ImageLoadCompletion),
    BackgroundCompleted(BackgroundCompletion),
    ForegroundTasksReady,
    MenuWillOpen,
    MenuAction(usize),
    #[cfg(target_os = "macos")]
    DockMenuAction(usize),
    #[cfg(any(target_os = "macos", target_os = "windows"))]
    NativePopupMenuAction(u64, usize),
    #[cfg(any(target_os = "macos", target_os = "windows"))]
    NativePopupMenuClosed(u64),
    #[cfg(target_os = "macos")]
    ExternalDragBoundary(WindowHandle, Point),
    #[cfg(target_os = "macos")]
    ExternalDragEnded(WindowHandle, ExternalDragOperation),
    #[cfg(target_os = "macos")]
    NativeDropChanged(WindowHandle),
    #[cfg(target_os = "macos")]
    ApplicationDeactivated,
    #[cfg(target_os = "macos")]
    PopoverPointerDismissRequested(WindowHandle),
    #[cfg(any(
        target_os = "macos",
        target_os = "windows",
        target_os = "linux",
        target_os = "freebsd",
        target_os = "dragonfly",
        target_os = "openbsd",
        target_os = "netbsd"
    ))]
    PlatformDialogClosed(Option<WindowHandle>, PlatformDialogId),
    #[cfg(target_os = "macos")]
    PlatformDialogCancelled(Option<WindowHandle>, PlatformDialogId),
    #[cfg_attr(not(target_os = "macos"), allow(dead_code))]
    OpenUrls(OpenUrls),
    #[cfg(target_os = "macos")]
    Reopen {
        has_visible_windows: bool,
    },
    #[cfg(target_os = "macos")]
    QuitRequested,
    #[cfg(target_os = "macos")]
    SystemWake,
    #[cfg(target_os = "macos")]
    DisplaysChanged,
    #[cfg(target_os = "macos")]
    KeyboardLayoutChanged,
    #[cfg(target_os = "macos")]
    SystemNotificationAuthorization {
        granted: bool,
        error: Option<Arc<str>>,
    },
    #[cfg(target_os = "macos")]
    SystemNotificationPermissionStatus(NotificationPermissionStatus),
    SystemNotificationResponse(SystemNotificationResponse),
    #[cfg(any(target_os = "macos", target_os = "windows", target_os = "linux"))]
    GlobalShortcut(u32),
    #[cfg(any(
        target_os = "macos",
        target_os = "windows",
        target_os = "linux",
        target_os = "freebsd",
        target_os = "dragonfly",
        target_os = "openbsd",
        target_os = "netbsd"
    ))]
    SecondInstance(SecondInstanceEvent),
    SystemPreferencesChanged(SystemPreferences),
    Power(PowerEvent),
    Tray(TrayEvent),
}

impl From<AccessibilityEvent> for RuntimeEvent {
    fn from(event: AccessibilityEvent) -> Self {
        Self::Accessibility(event)
    }
}

/// Defines when closing the final native window should terminate the application.
///
/// The default follows native desktop convention: macOS applications stay resident for Dock
/// reopen and menu commands, while other desktop targets quit with their last window.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub enum QuitMode {
    /// Use [`Self::Explicit`] on macOS and [`Self::LastWindowClosed`] elsewhere.
    #[default]
    Default,
    /// Quit automatically after the final window and all of its owned children close.
    LastWindowClosed,
    /// Stay in the event loop until [`EventContext::exit`] or the operating system quits the app.
    Explicit,
}

/// Source of one application-level quit request.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum QuitReason {
    Explicit,
    Relaunch,
    LastWindowClosed,
    OperatingSystem,
}

/// Immutable input delivered to the preventable before-quit and will-quit callbacks.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct QuitRequest {
    pub reason: QuitReason,
}

impl QuitMode {
    const fn quits_when_empty(self) -> bool {
        match self {
            Self::Default => cfg!(not(target_os = "macos")),
            Self::LastWindowClosed => true,
            Self::Explicit => false,
        }
    }
}

/// GPU selection policy. `Balanced` lets WGPU choose the most appropriate adapter.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub enum PerformanceProfile {
    #[default]
    Balanced,
    LowPower,
    HighPerformance,
}

/// Effective light or dark appearance of a native window.
///
/// QuickGUI intentionally exposes the semantic palette instead of platform appearance names.
/// Application content can observe this value through [`ViewContext::appearance`], while native
/// chrome follows the system unless an explicit per-window preference is configured.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub enum WindowAppearance {
    #[default]
    Light,
    Dark,
}

impl WindowAppearance {
    pub const fn is_dark(self) -> bool {
        matches!(self, Self::Dark)
    }
}

/// Compositor treatment for pixels not covered by fully opaque application content.
///
/// `Transparent` exposes the desktop directly. `Blurred` uses the platform's native background
/// blur behind the same alpha-capable WGPU surface. Both remain damage-driven; they do not add an
/// application animation or polling loop.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub enum WindowBackgroundAppearance {
    #[default]
    Opaque,
    Transparent,
    Blurred,
}

impl WindowBackgroundAppearance {
    pub const fn is_transparent(self) -> bool {
        !matches!(self, Self::Opaque)
    }

    pub const fn is_blurred(self) -> bool {
        matches!(self, Self::Blurred)
    }

    const fn changes_from(self, previous: Self) -> WindowBackgroundChanges {
        WindowBackgroundChanges {
            transparency: self.is_transparent() != previous.is_transparent(),
            blur: self.is_blurred() != previous.is_blurred(),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct WindowBackgroundChanges {
    transparency: bool,
    blur: bool,
}

/// Native window titlebar presentation.
///
/// `HiddenInset` is currently meaningful on macOS. It keeps the native window controls while
/// extending application content through a transparent, titleless titlebar.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub enum TitleBarStyle {
    #[default]
    Default,
    HiddenInset,
    /// Hide native titlebar chrome and let application content own the complete window.
    Hidden,
}

/// Native role and parent relationship of a top-level window.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub enum WindowKind {
    #[default]
    Normal,
    /// A high-level utility or notification window.
    Popover,
    /// A transient native popover positioned relative to its parent window's content.
    SystemPopover,
    /// A utility window that stays above ordinary application windows.
    Floating,
    /// A parent-owned modal sheet on macOS.
    Dialog,
}

/// Requested native stacking level independent from a window's ownership role.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub enum WindowLevel {
    AlwaysOnBottom,
    #[default]
    Normal,
    AlwaysOnTop,
}

/// Native taskbar progress presentation for one window.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub enum TaskbarProgressState {
    /// Remove progress from the taskbar button.
    #[default]
    None,
    Normal,
    Indeterminate,
    Paused,
    Error,
}

/// Native pointer confinement policy for one window.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub enum CursorGrabMode {
    /// Let the pointer move without confinement.
    #[default]
    None,
    /// Keep the pointer inside the window when the backend supports confinement.
    Confined,
    /// Lock the pointer to the window while continuing to report relative movement.
    Locked,
}

impl CursorGrabMode {
    const fn to_winit(self) -> WinitCursorGrabMode {
        match self {
            Self::None => WinitCursorGrabMode::None,
            Self::Confined => WinitCursorGrabMode::Confined,
            Self::Locked => WinitCursorGrabMode::Locked,
        }
    }
}

impl WindowLevel {
    const fn to_winit(self) -> WinitWindowLevel {
        match self {
            Self::AlwaysOnBottom => WinitWindowLevel::AlwaysOnBottom,
            Self::Normal => WinitWindowLevel::Normal,
            Self::AlwaysOnTop => WinitWindowLevel::AlwaysOnTop,
        }
    }
}

/// Persistable window state together with its windowed restore geometry.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum WindowBounds {
    Windowed(Rect),
    Maximized(Rect),
    Fullscreen(Rect),
}

impl WindowBounds {
    pub const fn bounds(self) -> Rect {
        match self {
            Self::Windowed(bounds) | Self::Maximized(bounds) | Self::Fullscreen(bounds) => bounds,
        }
    }

    pub const fn windowed(x: f32, y: f32, width: f32, height: f32) -> Self {
        Self::Windowed(Rect::new(x, y, width, height))
    }
}

impl Default for WindowBounds {
    fn default() -> Self {
        Self::Windowed(Rect::new(0.0, 0.0, 960.0, 640.0))
    }
}

/// Retained native state available during declarative rendering.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WindowState {
    pub handle: WindowHandle,
    /// Display currently containing the native window, when known.
    pub display_id: Option<DisplayId>,
    pub kind: WindowKind,
    pub bounds: WindowBounds,
    pub viewport_size: Size,
    /// Effective native minimum inner size, or `None` when unconstrained.
    pub minimum_size: Option<Size>,
    /// Effective native maximum inner size, or `None` when unconstrained.
    pub maximum_size: Option<Size>,
    pub scale_factor: f32,
    pub appearance: WindowAppearance,
    pub background_appearance: WindowBackgroundAppearance,
    pub focused: bool,
    /// Whether the native window is permitted to receive keyboard focus.
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
    pub window_level: WindowLevel,
    /// Whether this window is omitted from the taskbar on supported platforms.
    pub skip_taskbar: bool,
    /// Whether this window follows the user across virtual desktops/spaces.
    pub visible_on_all_workspaces: bool,
    /// Whole-window native alpha in the inclusive `0.0..=1.0` range.
    pub opacity: f32,
    /// Whether an explicit native window icon is installed.
    pub has_icon: bool,
    /// Retained taskbar progress mode and bounded completion value.
    pub taskbar_progress_state: TaskbarProgressState,
    pub taskbar_progress: f32,
    /// Whether a Windows taskbar overlay icon is installed.
    pub has_taskbar_overlay_icon: bool,
    pub cursor_visible: bool,
    pub cursor_grab: CursorGrabMode,
    /// Whether the native window participates in pointer hit testing.
    pub cursor_hit_test: bool,
    /// Last known pointer position in logical window coordinates.
    pub cursor_position: Option<Point>,
    /// Whether the native titlebar currently represents a document file.
    pub represented_file: bool,
    /// Whether native chrome indicates that the represented document has unsaved changes.
    pub document_edited: bool,
    /// Whether this window opted into native system tabbing.
    pub native_tabbing: bool,
    /// Bounded cached state for the native AppKit tab group.
    pub native_tabs: WindowTabState,
    /// Whether the feature-gated retained-tree inspector is open for this window.
    #[cfg(feature = "inspector")]
    pub inspector_active: bool,
}

/// Maximum UTF-8 bytes accepted for a native window title.
pub const MAX_WINDOW_TITLE_BYTES: usize = 16 * 1024;
/// Maximum encoded bytes accepted for a represented document path.
pub const MAX_WINDOW_DOCUMENT_PATH_BYTES: usize = 16 * 1024;
/// Maximum UTF-8 bytes accepted for a native window tabbing identifier.
pub const MAX_WINDOW_TABBING_IDENTIFIER_BYTES: usize = 4 * 1024;
/// Maximum native tabs inspected or exposed through one retained [`WindowState`].
pub const MAX_SYSTEM_WINDOW_TABS: usize = 256;
/// Maximum logical width or height accepted by a programmatic window-bounds request.
pub const MAX_WINDOW_LOGICAL_DIMENSION: f32 = 32_768.0;
/// Maximum absolute desktop coordinate accepted by a programmatic window-bounds request.
pub const MAX_WINDOW_LOGICAL_COORDINATE: f32 = 16_777_216.0;
/// Maximum window mutations one event callback may queue.
pub const MAX_WINDOW_COMMANDS_PER_EVENT: usize = 256;
/// Maximum deferred native window mutations retained by one application effect cycle.
pub const MAX_PENDING_WINDOW_COMMANDS: usize = 1_024;
/// Maximum declarative child-window close callbacks retained by one parent window.
pub const MAX_CHILD_WINDOW_CLOSE_LISTENERS_PER_WINDOW: usize = 256;
/// Maximum handles returned by one immutable application-window registry snapshot.
pub const MAX_APPLICATION_WINDOWS: usize = 4_096;
/// Maximum selected native popup menus retained until their callback is routed.
pub const MAX_PENDING_NATIVE_POPUP_MENUS: usize = 16;

#[derive(Clone, Copy, Debug, Eq, Error, PartialEq)]
pub enum WindowCommandError {
    #[error("this context is not attached to a native window")]
    Unavailable,
    #[error("one event cannot queue more than {MAX_WINDOW_COMMANDS_PER_EVENT} window commands")]
    QueueFull,
    #[error("window bounds must be finite, positive, and within the supported desktop range")]
    InvalidBounds,
    #[error("a window title cannot exceed {MAX_WINDOW_TITLE_BYTES} UTF-8 bytes")]
    TitleTooLong,
    #[error(
        "a represented document path must be non-empty, NUL-free, and at most {MAX_WINDOW_DOCUMENT_PATH_BYTES} encoded bytes"
    )]
    InvalidDocumentPath,
    #[error(
        "a native tabbing identifier must be non-empty, NUL-free, and at most {MAX_WINDOW_TABBING_IDENTIFIER_BYTES} UTF-8 bytes"
    )]
    InvalidTabbingIdentifier,
    #[error("a native tab index must be smaller than {MAX_SYSTEM_WINDOW_TABS}")]
    InvalidTabIndex,
    #[error("a hidden titlebar cannot expose or reposition native traffic-light buttons")]
    HiddenTitleBarTrafficLights,
    #[error("system popovers require one finite parent-relative popover configuration")]
    InvalidPopoverConfiguration,
    #[error("a system popover must be opened from an existing parent window")]
    PopoverParentRequired,
    #[error("minimum window size cannot exceed maximum window size")]
    InvalidSizeConstraints,
    #[error("window opacity must be finite and between 0.0 and 1.0")]
    InvalidOpacity,
    #[error("taskbar progress must be finite and between 0.0 and 1.0")]
    InvalidTaskbarProgress,
    #[error(
        "a taskbar overlay description must be NUL-free and at most {MAX_TASKBAR_OVERLAY_DESCRIPTION_BYTES} UTF-8 bytes"
    )]
    InvalidTaskbarOverlayDescription,
    #[error("the per-window native menu declaration is invalid")]
    InvalidMenus,
}

/// Constant-size snapshot of one native system window-tab group.
///
/// AppKit can technically retain an arbitrary number of windows. QuickGUI inspects at most
/// [`MAX_SYSTEM_WINDOW_TABS`] at an event boundary and reports `truncated` rather than allocating
/// a per-frame list.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct WindowTabState {
    pub count: usize,
    pub selected_index: Option<usize>,
    pub tab_bar_visible: bool,
    pub overview_visible: bool,
    pub truncated: bool,
}

impl Default for WindowTabState {
    fn default() -> Self {
        Self {
            count: 1,
            selected_index: Some(0),
            tab_bar_visible: false,
            overview_visible: false,
            truncated: false,
        }
    }
}

impl WindowTabState {
    pub const fn is_valid(self) -> bool {
        self.count > 0
            && self.count <= MAX_SYSTEM_WINDOW_TABS
            && match self.selected_index {
                Some(index) => index < self.count,
                None => true,
            }
    }
}

static NEXT_WINDOW_HANDLE: AtomicU64 = AtomicU64::new(1);

/// Stable application-level identity for a QuickGUI window.
///
/// Unlike Winit's native identifier, this handle exists before the platform window is created and
/// can therefore be returned immediately from [`EventContext::open_window`](crate::EventContext::open_window).
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct WindowHandle(u64);

impl WindowHandle {
    pub(crate) fn next() -> Self {
        Self(NEXT_WINDOW_HANDLE.fetch_add(1, Ordering::Relaxed).max(1))
    }

    pub const fn as_u64(self) -> u64 {
        self.0
    }

    pub const fn from_u64(value: u64) -> Option<Self> {
        if value == 0 { None } else { Some(Self(value)) }
    }
}

/// Immutable bounded application-wide window lookup captured at one core event boundary.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct WindowRegistry {
    handles: Arc<[WindowHandle]>,
    active_window: Option<WindowHandle>,
    truncated: bool,
}

impl WindowRegistry {
    pub(crate) fn new(
        handles: impl Into<Arc<[WindowHandle]>>,
        active_window: Option<WindowHandle>,
        truncated: bool,
    ) -> Self {
        Self {
            handles: handles.into(),
            active_window,
            truncated,
        }
    }

    pub fn windows(&self) -> &[WindowHandle] {
        &self.handles
    }

    pub const fn active_window(&self) -> Option<WindowHandle> {
        self.active_window
    }

    pub fn contains(&self, handle: WindowHandle) -> bool {
        self.handles.binary_search(&handle).is_ok()
    }

    pub const fn is_truncated(&self) -> bool {
        self.truncated
    }
}

/// Window and renderer defaults used by [`App`].
#[derive(Clone, Debug)]
pub struct AppConfig {
    pub title: String,
    pub size: Size,
    pub window_bounds: Option<WindowBounds>,
    /// Preferred display for default placement and borderless fullscreen.
    ///
    /// A disconnected or unknown display falls back to the current primary display.
    pub display_id: Option<DisplayId>,
    pub minimum_size: Option<Size>,
    pub maximum_size: Option<Size>,
    /// File represented by native document chrome, if any.
    pub represented_file: Option<PathBuf>,
    /// Initial unsaved-document indication in native chrome.
    pub document_edited: bool,
    /// Non-empty identifier opting this window into native system tabbing.
    pub tabbing_identifier: Option<String>,
    pub background: Color,
    /// Native compositor treatment behind transparent application pixels.
    pub window_background: WindowBackgroundAppearance,
    pub performance_profile: PerformanceProfile,
    /// Explicit native light/dark preference. `None` follows the current system appearance.
    pub preferred_appearance: Option<WindowAppearance>,
    pub title_bar_style: TitleBarStyle,
    pub kind: WindowKind,
    /// Parent-relative native placement when `kind` is [`WindowKind::SystemPopover`].
    pub popover: Option<crate::PopoverOptions>,
    pub focus: bool,
    /// Whether the window may receive native keyboard focus after creation.
    pub focusable: bool,
    pub show: bool,
    pub is_movable: bool,
    pub is_resizable: bool,
    pub is_minimizable: bool,
    pub is_maximizable: bool,
    pub is_closable: bool,
    /// Whether native border and titlebar decorations are present.
    pub decorated: bool,
    /// Requested native shadow. Some platforms always draw decorated-window shadows.
    pub shadow: bool,
    /// Prevent supported desktop capture APIs from reading this window's contents.
    pub content_protected: bool,
    /// Explicit stacking override. `None` derives a role-appropriate level from [`Self::kind`].
    pub window_level: Option<WindowLevel>,
    /// Hide the per-window taskbar entry where the platform exposes one.
    pub skip_taskbar: bool,
    /// Keep the window visible on every virtual desktop/space where supported.
    pub visible_on_all_workspaces: bool,
    /// Whole-window native alpha.
    pub opacity: f32,
    /// Native window icon. macOS uses an application icon rather than per-window icons.
    pub icon: Option<Image>,
    /// Initial Windows taskbar progress and overlay state.
    pub taskbar_progress_state: TaskbarProgressState,
    pub taskbar_progress: f32,
    pub taskbar_overlay_icon: Option<Image>,
    pub taskbar_overlay_description: Option<String>,
    /// Initial pointer visibility and confinement policy.
    pub cursor_visible: bool,
    pub cursor_grab: CursorGrabMode,
    /// Whether pointer events hit this native window.
    pub cursor_hit_test: bool,
    /// Optional initial logical pointer position relative to the window.
    pub cursor_position: Option<Point>,
    /// Per-window native menus. `None` inherits the application's current menu declaration.
    pub window_menus: Option<Vec<Menu>>,
    /// Top-left position of the macOS close button, in logical points from the window's top-left.
    pub traffic_light_position: Option<Point>,
    /// Logical pixels represented by one platform line-wheel unit.
    pub line_scroll_pixels: f32,
    /// How long an incomplete multi-stroke key binding waits before its prefix is replayed.
    pub key_sequence_timeout: Duration,
    /// Disable non-essential image animation. macOS Reduce Motion is always respected as well.
    pub reduce_motion: bool,
    /// Open the retained-tree inspector with this window.
    #[cfg(feature = "inspector")]
    pub inspector: bool,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            title: "QuickGUI".to_owned(),
            size: Size::new(960.0, 640.0),
            window_bounds: None,
            display_id: None,
            minimum_size: Some(Size::new(320.0, 240.0)),
            maximum_size: None,
            represented_file: None,
            document_edited: false,
            tabbing_identifier: None,
            background: Color::rgb8(18, 18, 20),
            window_background: WindowBackgroundAppearance::Opaque,
            performance_profile: PerformanceProfile::Balanced,
            preferred_appearance: None,
            title_bar_style: TitleBarStyle::Default,
            kind: WindowKind::Normal,
            popover: None,
            focus: true,
            focusable: true,
            show: true,
            is_movable: true,
            is_resizable: true,
            is_minimizable: true,
            is_maximizable: true,
            is_closable: true,
            decorated: true,
            shadow: true,
            content_protected: false,
            window_level: None,
            skip_taskbar: false,
            visible_on_all_workspaces: false,
            opacity: 1.0,
            icon: None,
            taskbar_progress_state: TaskbarProgressState::None,
            taskbar_progress: 0.0,
            taskbar_overlay_icon: None,
            taskbar_overlay_description: None,
            cursor_visible: true,
            cursor_grab: CursorGrabMode::None,
            cursor_hit_test: true,
            cursor_position: None,
            window_menus: None,
            traffic_light_position: None,
            line_scroll_pixels: 40.0,
            key_sequence_timeout: Duration::from_secs(1),
            reduce_motion: false,
            #[cfg(feature = "inspector")]
            inspector: false,
        }
    }
}

impl AppConfig {
    /// Create window options with a title and otherwise production-safe defaults.
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            ..Self::default()
        }
    }

    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = title.into();
        self
    }

    pub fn size(mut self, width: f32, height: f32) -> Self {
        self.size = Size::new(width, height);
        if let Some(bounds) = &mut self.window_bounds {
            let state = *bounds;
            let rect = state.bounds();
            let rect = Rect::new(rect.x, rect.y, width, height);
            *bounds = match state {
                WindowBounds::Windowed(_) => WindowBounds::Windowed(rect),
                WindowBounds::Maximized(_) => WindowBounds::Maximized(rect),
                WindowBounds::Fullscreen(_) => WindowBounds::Fullscreen(rect),
            };
        }
        self
    }

    pub fn window_bounds(mut self, bounds: WindowBounds) -> Self {
        let rect = bounds.bounds();
        self.size = Size::new(rect.width, rect.height);
        self.window_bounds = Some(bounds);
        self
    }

    pub fn position(mut self, x: f32, y: f32) -> Self {
        let bounds = self
            .window_bounds
            .unwrap_or_else(|| WindowBounds::Windowed(Rect::from_size(self.size)));
        let rect = bounds.bounds();
        let rect = Rect::new(x, y, rect.width, rect.height);
        self.window_bounds = Some(match bounds {
            WindowBounds::Windowed(_) => WindowBounds::Windowed(rect),
            WindowBounds::Maximized(_) => WindowBounds::Maximized(rect),
            WindowBounds::Fullscreen(_) => WindowBounds::Fullscreen(rect),
        });
        self
    }

    /// Select the display used for automatic placement and fullscreen creation.
    pub fn display(mut self, display: DisplayId) -> Self {
        self.display_id = Some(display);
        self
    }

    pub fn without_display(mut self) -> Self {
        self.display_id = None;
        self
    }

    pub fn maximized(mut self, maximized: bool) -> Self {
        let rect = self
            .window_bounds
            .map(WindowBounds::bounds)
            .unwrap_or_else(|| Rect::from_size(self.size));
        self.window_bounds = Some(if maximized {
            WindowBounds::Maximized(rect)
        } else {
            WindowBounds::Windowed(rect)
        });
        self
    }

    pub fn fullscreen(mut self, fullscreen: bool) -> Self {
        let rect = self
            .window_bounds
            .map(WindowBounds::bounds)
            .unwrap_or_else(|| Rect::from_size(self.size));
        self.window_bounds = Some(if fullscreen {
            WindowBounds::Fullscreen(rect)
        } else {
            WindowBounds::Windowed(rect)
        });
        self
    }

    pub fn minimum_size(mut self, width: f32, height: f32) -> Self {
        self.minimum_size = Some(Size::new(width, height));
        self
    }

    pub fn without_minimum_size(mut self) -> Self {
        self.minimum_size = None;
        self
    }

    pub fn maximum_size(mut self, width: f32, height: f32) -> Self {
        self.maximum_size = Some(Size::new(width, height));
        self
    }

    pub fn without_maximum_size(mut self) -> Self {
        self.maximum_size = None;
        self
    }

    /// Represent a file in native document chrome.
    pub fn represented_file(mut self, path: impl Into<PathBuf>) -> Self {
        self.represented_file = Some(path.into());
        self
    }

    /// GPUI-compatible alias for [`Self::represented_file`].
    pub fn document_path(self, path: impl Into<PathBuf>) -> Self {
        self.represented_file(path)
    }

    pub fn without_represented_file(mut self) -> Self {
        self.represented_file = None;
        self
    }

    /// Override the application's native menus for this window.
    pub fn window_menus(mut self, menus: impl IntoIterator<Item = Menu>) -> Self {
        self.window_menus = Some(menus.into_iter().collect());
        self
    }

    /// Inherit the application's native menus, including later app-wide replacements.
    pub fn use_application_menus(mut self) -> Self {
        self.window_menus = None;
        self
    }

    pub fn document_edited(mut self, edited: bool) -> Self {
        self.document_edited = edited;
        self
    }

    /// Opt this window into AppKit system tabbing with an application-defined group identifier.
    pub fn tabbing_identifier(mut self, identifier: impl Into<String>) -> Self {
        self.tabbing_identifier = Some(identifier.into());
        self
    }

    pub fn without_tabbing_identifier(mut self) -> Self {
        self.tabbing_identifier = None;
        self
    }

    pub fn background(mut self, background: Color) -> Self {
        self.background = background;
        self
    }

    pub fn window_background(mut self, appearance: WindowBackgroundAppearance) -> Self {
        self.window_background = appearance;
        self
    }

    pub fn performance_profile(mut self, profile: PerformanceProfile) -> Self {
        self.performance_profile = profile;
        self
    }

    /// Force this window's native chrome to use one appearance.
    pub fn window_appearance(mut self, appearance: WindowAppearance) -> Self {
        self.preferred_appearance = Some(appearance);
        self
    }

    /// Follow the operating system's effective light/dark appearance.
    pub fn follow_system_appearance(mut self) -> Self {
        self.preferred_appearance = None;
        self
    }

    pub fn title_bar_style(mut self, style: TitleBarStyle) -> Self {
        self.title_bar_style = style;
        self.decorated = style != TitleBarStyle::Hidden;
        self
    }

    pub fn window_kind(mut self, kind: WindowKind) -> Self {
        self.kind = kind;
        if kind != WindowKind::SystemPopover {
            self.popover = None;
        }
        self
    }

    /// Configure a borderless native `SystemPopover`.
    ///
    /// Menu-style grabs focus the panel and dismiss on Escape or an outside mouse press.
    /// Non-grabbing popovers install no event monitor; `PopoverOptions::accepts_key_focus` independently
    /// decides whether pointer interaction may make the panel key.
    pub fn system_popover(mut self, popover: crate::PopoverOptions) -> Self {
        self.kind = WindowKind::SystemPopover;
        self.focus = popover.grab;
        self.focusable = popover.accepts_key_focus;
        self.popover = Some(popover);
        self.minimum_size = None;
        self.maximum_size = None;
        self.title_bar_style = TitleBarStyle::Hidden;
        self.decorated = false;
        self.traffic_light_position = None;
        self.is_movable = false;
        self.is_resizable = false;
        self.is_minimizable = false;
        self.is_maximizable = false;
        self.is_closable = false;
        self
    }

    pub fn focus(mut self, focus: bool) -> Self {
        self.focus = focus;
        self
    }

    /// Allow or prevent this window from receiving native keyboard focus.
    pub fn focusable(mut self, focusable: bool) -> Self {
        self.focusable = focusable;
        if !focusable {
            self.focus = false;
        }
        self
    }

    pub fn show(mut self, show: bool) -> Self {
        self.show = show;
        self
    }

    pub fn movable(mut self, movable: bool) -> Self {
        self.is_movable = movable;
        self
    }

    pub fn resizable(mut self, resizable: bool) -> Self {
        self.is_resizable = resizable;
        self
    }

    pub fn minimizable(mut self, minimizable: bool) -> Self {
        self.is_minimizable = minimizable;
        self
    }

    pub fn maximizable(mut self, maximizable: bool) -> Self {
        self.is_maximizable = maximizable;
        self
    }

    pub fn closable(mut self, closable: bool) -> Self {
        self.is_closable = closable;
        self
    }

    pub fn decorations(mut self, decorated: bool) -> Self {
        self.decorated = decorated;
        if !decorated {
            self.title_bar_style = TitleBarStyle::Hidden;
            self.traffic_light_position = None;
        } else if self.title_bar_style == TitleBarStyle::Hidden {
            self.title_bar_style = TitleBarStyle::Default;
        }
        self
    }

    pub fn shadow(mut self, shadow: bool) -> Self {
        self.shadow = shadow;
        self
    }

    pub fn content_protected(mut self, protected: bool) -> Self {
        self.content_protected = protected;
        self
    }

    pub fn window_level(mut self, level: WindowLevel) -> Self {
        self.window_level = Some(level);
        self
    }

    pub fn automatic_window_level(mut self) -> Self {
        self.window_level = None;
        self
    }

    pub fn skip_taskbar(mut self, skip: bool) -> Self {
        self.skip_taskbar = skip;
        self
    }

    pub fn visible_on_all_workspaces(mut self, visible: bool) -> Self {
        self.visible_on_all_workspaces = visible;
        self
    }

    /// Set whole-window native opacity. Invalid values are rejected when the window is opened.
    pub fn opacity(mut self, opacity: f32) -> Self {
        self.opacity = opacity;
        self
    }

    pub fn icon(mut self, icon: Image) -> Self {
        self.icon = Some(icon);
        self
    }

    pub fn without_icon(mut self) -> Self {
        self.icon = None;
        self
    }

    pub fn taskbar_progress(mut self, state: TaskbarProgressState, progress: f32) -> Self {
        self.taskbar_progress_state = state;
        self.taskbar_progress = progress;
        self
    }

    pub fn taskbar_overlay_icon(mut self, icon: Image, description: impl Into<String>) -> Self {
        self.taskbar_overlay_icon = Some(icon);
        self.taskbar_overlay_description = Some(description.into());
        self
    }

    pub fn without_taskbar_overlay_icon(mut self) -> Self {
        self.taskbar_overlay_icon = None;
        self.taskbar_overlay_description = None;
        self
    }

    pub fn cursor_visible(mut self, visible: bool) -> Self {
        self.cursor_visible = visible;
        self
    }

    pub fn cursor_grab(mut self, mode: CursorGrabMode) -> Self {
        self.cursor_grab = mode;
        self
    }

    pub fn cursor_hit_test(mut self, hit_test: bool) -> Self {
        self.cursor_hit_test = hit_test;
        self
    }

    pub fn cursor_position(mut self, position: Point) -> Self {
        self.cursor_position = Some(position);
        self
    }

    pub fn without_cursor_position(mut self) -> Self {
        self.cursor_position = None;
        self
    }

    pub fn always_on_top(self, always_on_top: bool) -> Self {
        self.window_level(if always_on_top {
            WindowLevel::AlwaysOnTop
        } else {
            WindowLevel::Normal
        })
    }

    pub fn always_on_bottom(self, always_on_bottom: bool) -> Self {
        self.window_level(if always_on_bottom {
            WindowLevel::AlwaysOnBottom
        } else {
            WindowLevel::Normal
        })
    }

    /// Position the macOS traffic lights in logical points from the window's top-left.
    ///
    /// The close button uses this exact position; minimize and zoom retain native spacing.
    pub fn traffic_light_position(mut self, x: f32, y: f32) -> Self {
        self.traffic_light_position = Some(Point::new(x, y));
        self
    }

    pub fn without_traffic_light_position(mut self) -> Self {
        self.traffic_light_position = None;
        self
    }

    pub fn reduce_motion(mut self, reduce_motion: bool) -> Self {
        self.reduce_motion = reduce_motion;
        self
    }

    /// Open or suppress the retained-tree inspector when this window is created.
    #[cfg(feature = "inspector")]
    pub fn inspector(mut self, inspector: bool) -> Self {
        self.inspector = inspector;
        self
    }
}

/// Window-local configuration accepted by [`EventContext::open_window`](crate::EventContext::open_window).
pub type WindowOptions = AppConfig;

pub(crate) fn validate_window_bounds(bounds: WindowBounds) -> Result<(), WindowCommandError> {
    let bounds = bounds.bounds();
    let valid = bounds.x.is_finite()
        && bounds.y.is_finite()
        && bounds.width.is_finite()
        && bounds.height.is_finite()
        && bounds.x.abs() <= MAX_WINDOW_LOGICAL_COORDINATE
        && bounds.y.abs() <= MAX_WINDOW_LOGICAL_COORDINATE
        && bounds.width > 0.0
        && bounds.height > 0.0
        && bounds.width <= MAX_WINDOW_LOGICAL_DIMENSION
        && bounds.height <= MAX_WINDOW_LOGICAL_DIMENSION;
    if valid {
        Ok(())
    } else {
        Err(WindowCommandError::InvalidBounds)
    }
}

pub(crate) fn validate_window_size(size: Size) -> Result<(), WindowCommandError> {
    validate_window_bounds(WindowBounds::Windowed(Rect::from_size(size)))
}

pub(crate) fn validate_window_position(position: Point) -> Result<(), WindowCommandError> {
    validate_window_bounds(WindowBounds::Windowed(Rect::new(
        position.x, position.y, 1.0, 1.0,
    )))
}

pub(crate) fn validate_window_title(title: &str) -> Result<(), WindowCommandError> {
    if title.len() <= MAX_WINDOW_TITLE_BYTES {
        Ok(())
    } else {
        Err(WindowCommandError::TitleTooLong)
    }
}

pub(crate) fn validate_window_document_path(path: &Path) -> Result<(), WindowCommandError> {
    let bytes = path.as_os_str().as_encoded_bytes();
    if !bytes.is_empty() && bytes.len() <= MAX_WINDOW_DOCUMENT_PATH_BYTES && !bytes.contains(&0) {
        Ok(())
    } else {
        Err(WindowCommandError::InvalidDocumentPath)
    }
}

pub(crate) fn validate_window_tabbing_identifier(
    identifier: &str,
) -> Result<(), WindowCommandError> {
    if !identifier.is_empty()
        && identifier.len() <= MAX_WINDOW_TABBING_IDENTIFIER_BYTES
        && !identifier.as_bytes().contains(&0)
    {
        Ok(())
    } else {
        Err(WindowCommandError::InvalidTabbingIdentifier)
    }
}

fn validate_window_options(options: &WindowOptions) -> Result<(), WindowCommandError> {
    validate_window_title(&options.title)?;
    validate_window_size(options.size)?;
    if let Some(bounds) = options.window_bounds {
        validate_window_bounds(bounds)?;
    }
    if let Some(minimum) = options.minimum_size {
        validate_window_size(minimum)?;
    }
    if let Some(maximum) = options.maximum_size {
        validate_window_size(maximum)?;
    }
    validate_window_opacity(options.opacity)?;
    validate_taskbar_progress(options.taskbar_progress)?;
    validate_taskbar_overlay_description(options.taskbar_overlay_description.as_deref())?;
    if let Some(position) = options.cursor_position {
        validate_window_position(position)?;
    }
    if let (Some(minimum), Some(maximum)) = (options.minimum_size, options.maximum_size)
        && (minimum.width > maximum.width || minimum.height > maximum.height)
    {
        return Err(WindowCommandError::InvalidSizeConstraints);
    }
    if let Some(path) = options.represented_file.as_deref() {
        validate_window_document_path(path)?;
    }
    if let Some(identifier) = options.tabbing_identifier.as_deref() {
        validate_window_tabbing_identifier(identifier)?;
    }
    if let Some(menus) = options.window_menus.as_deref() {
        validate_menus(menus).map_err(|_| WindowCommandError::InvalidMenus)?;
    }
    if options.title_bar_style == TitleBarStyle::Hidden && options.traffic_light_position.is_some()
    {
        return Err(WindowCommandError::HiddenTitleBarTrafficLights);
    }
    match (options.kind, options.popover.as_ref()) {
        (WindowKind::SystemPopover, Some(popover))
            if popover.is_valid(MAX_WINDOW_LOGICAL_COORDINATE, MAX_WINDOW_LOGICAL_DIMENSION)
                && !options.decorated
                && options.title_bar_style == TitleBarStyle::Hidden
                && options.minimum_size.is_none()
                && options.maximum_size.is_none()
                && !options.is_movable
                && !options.is_resizable
                && !options.is_minimizable
                && !options.is_maximizable
                && !options.is_closable
                && !matches!(
                    options.window_bounds,
                    Some(WindowBounds::Maximized(_) | WindowBounds::Fullscreen(_))
                ) => {}
        (WindowKind::SystemPopover, _) | (_, Some(_)) => {
            return Err(WindowCommandError::InvalidPopoverConfiguration);
        }
        (_, None) => {}
    }
    Ok(())
}

pub(crate) fn validate_window_opacity(opacity: f32) -> Result<(), WindowCommandError> {
    if opacity.is_finite() && (0.0..=1.0).contains(&opacity) {
        Ok(())
    } else {
        Err(WindowCommandError::InvalidOpacity)
    }
}

pub(crate) fn validate_taskbar_progress(progress: f32) -> Result<(), WindowCommandError> {
    if progress.is_finite() && (0.0..=1.0).contains(&progress) {
        Ok(())
    } else {
        Err(WindowCommandError::InvalidTaskbarProgress)
    }
}

pub(crate) fn validate_taskbar_overlay_description(
    description: Option<&str>,
) -> Result<(), WindowCommandError> {
    if description.is_some_and(|description| {
        description.len() > crate::MAX_TASKBAR_OVERLAY_DESCRIPTION_BYTES
            || description.as_bytes().contains(&0)
    }) {
        Err(WindowCommandError::InvalidTaskbarOverlayDescription)
    } else {
        Ok(())
    }
}

/// A retained application view. It is only rendered after explicit invalidation or OS damage.
pub trait View: Sized + 'static {
    fn event(&mut self, _event: &Event, _cx: &mut EventContext) {}
    fn render(&mut self, cx: &mut ViewContext<'_, Self>) -> impl IntoElement;
}

trait AnyView {
    fn event(&mut self, event: &Event, cx: &mut EventContext);

    #[allow(clippy::too_many_arguments)]
    fn render(
        &mut self,
        size: Size,
        scale_factor: f32,
        metrics: FrameMetrics,
        focused: Option<ElementId>,
        focused_path: Vec<ElementId>,
        listeners: &mut ListenerRegistry,
        window: WindowHandle,
        window_state: WindowState,
        displays: &Displays,
        keyboard_layout: &KeyboardLayout,
        assets: &Assets,
        app_info: Option<&AppInfo>,
        app_paths: Option<&AppPaths>,
        system_info: &SystemInfo,
        system_preferences: &SystemPreferences,
        background_tasks: Option<&BackgroundTaskPoolHandle>,
        foreground_tasks: &ForegroundTaskSpawner,
        globals: &GlobalStore,
        event_proxy: Option<&EventLoopProxy<RuntimeEvent>>,
    ) -> (Element, bool, Option<Instant>);

    fn as_any_mut(&mut self) -> &mut dyn Any;

    #[cfg(any(test, feature = "test-support"))]
    fn as_any(&self) -> &dyn Any;
}

struct ViewAdapter<V>(V);

impl<V: View> AnyView for ViewAdapter<V> {
    fn event(&mut self, event: &Event, cx: &mut EventContext) {
        self.0.event(event, cx);
    }

    fn render(
        &mut self,
        size: Size,
        scale_factor: f32,
        metrics: FrameMetrics,
        focused: Option<ElementId>,
        focused_path: Vec<ElementId>,
        listeners: &mut ListenerRegistry,
        window: WindowHandle,
        window_state: WindowState,
        displays: &Displays,
        keyboard_layout: &KeyboardLayout,
        assets: &Assets,
        app_info: Option<&AppInfo>,
        app_paths: Option<&AppPaths>,
        system_info: &SystemInfo,
        system_preferences: &SystemPreferences,
        background_tasks: Option<&BackgroundTaskPoolHandle>,
        foreground_tasks: &ForegroundTaskSpawner,
        globals: &GlobalStore,
        event_proxy: Option<&EventLoopProxy<RuntimeEvent>>,
    ) -> (Element, bool, Option<Instant>) {
        let mut cx = ViewContext::<V> {
            size,
            scale_factor,
            metrics,
            focused,
            focused_path,
            request_animation_frame: false,
            repaint_deadline: None,
            listeners,
            window,
            window_state,
            displays,
            keyboard_layout,
            assets,
            app_info,
            app_paths,
            system_info,
            system_preferences,
            background_tasks,
            foreground_tasks,
            globals,
            event_proxy,
            marker: PhantomData,
        };
        cx.listeners.clear();
        let root = self.0.render(&mut cx).into_element();
        (root, cx.request_animation_frame, cx.repaint_deadline)
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        &mut self.0
    }

    #[cfg(any(test, feature = "test-support"))]
    fn as_any(&self) -> &dyn Any {
        &self.0
    }
}

pub(crate) struct WindowRequest {
    pub(crate) handle: WindowHandle,
    view: Box<dyn AnyView>,
    pub(crate) options: WindowOptions,
    pub(crate) parent: Option<WindowHandle>,
    /// Resolve this anchor from the parent window's retained layout at the event boundary and
    /// retain it as the focus-restoration target for the child lifetime.
    pub(crate) popover_anchor_element: Option<ElementId>,
}

impl WindowRequest {
    pub(crate) fn new<V: View>(view: V, options: WindowOptions) -> Self {
        Self::with_parent(view, options, None)
    }

    pub(crate) fn with_parent<V: View>(
        view: V,
        options: WindowOptions,
        parent: Option<WindowHandle>,
    ) -> Self {
        Self::with_handle(view, options, parent, WindowHandle::next())
    }

    pub(crate) fn with_handle<V: View>(
        view: V,
        options: WindowOptions,
        parent: Option<WindowHandle>,
        handle: WindowHandle,
    ) -> Self {
        Self {
            handle,
            view: Box::new(ViewAdapter(view)),
            options,
            parent,
            popover_anchor_element: None,
        }
    }
}

impl fmt::Debug for WindowRequest {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("WindowRequest")
            .field("handle", &self.handle)
            .field("parent", &self.parent)
            .field("popover_anchor_element", &self.popover_anchor_element)
            .field("options", &self.options)
            .finish_non_exhaustive()
    }
}

#[derive(Debug)]
pub(crate) enum WindowCommand {
    SetTitle(WindowHandle, String),
    SetRepresentedFile(WindowHandle, Option<PathBuf>),
    SetDocumentEdited(WindowHandle, bool),
    ShowCharacterPalette(WindowHandle),
    SetTabbingIdentifier(WindowHandle, Option<String>),
    SelectNextTab(WindowHandle),
    SelectPreviousTab(WindowHandle),
    SelectTab(WindowHandle, usize),
    MergeAllWindows(WindowHandle),
    MoveTabToNewWindow(WindowHandle),
    ToggleTabBar(WindowHandle),
    ToggleTabOverview(WindowHandle),
    SetBounds(WindowHandle, WindowBounds),
    Move(WindowHandle, Point),
    Resize(WindowHandle, Size),
    Minimize(WindowHandle),
    Restore(WindowHandle),
    Zoom(WindowHandle),
    ToggleFullscreen(WindowHandle),
    SetFullscreen(WindowHandle, bool),
    SetVisible(WindowHandle, bool),
    SetMovable(WindowHandle, bool),
    SetResizable(WindowHandle, bool),
    SetMinimumSize(WindowHandle, Option<Size>),
    SetMaximumSize(WindowHandle, Option<Size>),
    SetMinimizable(WindowHandle, bool),
    SetMaximizable(WindowHandle, bool),
    SetClosable(WindowHandle, bool),
    SetDecorated(WindowHandle, bool),
    SetShadow(WindowHandle, bool),
    SetContentProtected(WindowHandle, bool),
    SetWindowLevel(WindowHandle, Option<WindowLevel>),
    SetFocusable(WindowHandle, bool),
    SetSkipTaskbar(WindowHandle, bool),
    SetVisibleOnAllWorkspaces(WindowHandle, bool),
    SetOpacity(WindowHandle, f32),
    SetIcon(WindowHandle, Option<Image>),
    SetTaskbarProgress(WindowHandle, TaskbarProgressState, f32),
    SetTaskbarOverlayIcon(WindowHandle, Option<Image>, Option<String>),
    SetCursorVisible(WindowHandle, bool),
    SetCursorGrab(WindowHandle, CursorGrabMode),
    SetCursorHitTest(WindowHandle, bool),
    SetCursorPosition(WindowHandle, Point),
    SetAppearance(WindowHandle, Option<WindowAppearance>),
    SetBackgroundAppearance(WindowHandle, WindowBackgroundAppearance),
    #[cfg(feature = "inspector")]
    SetInspector(WindowHandle, bool),
    #[cfg(feature = "inspector")]
    ToggleInspector(WindowHandle),
    RequestAttention(WindowHandle),
}

impl WindowCommand {
    pub(crate) fn handle(&self) -> WindowHandle {
        match self {
            Self::SetTitle(handle, _)
            | Self::SetRepresentedFile(handle, _)
            | Self::SetDocumentEdited(handle, _)
            | Self::ShowCharacterPalette(handle)
            | Self::SetTabbingIdentifier(handle, _)
            | Self::SelectNextTab(handle)
            | Self::SelectPreviousTab(handle)
            | Self::SelectTab(handle, _)
            | Self::MergeAllWindows(handle)
            | Self::MoveTabToNewWindow(handle)
            | Self::ToggleTabBar(handle)
            | Self::ToggleTabOverview(handle)
            | Self::SetBounds(handle, _)
            | Self::Move(handle, _)
            | Self::Resize(handle, _)
            | Self::Minimize(handle)
            | Self::Restore(handle)
            | Self::Zoom(handle)
            | Self::ToggleFullscreen(handle)
            | Self::SetFullscreen(handle, _)
            | Self::SetVisible(handle, _)
            | Self::SetMovable(handle, _)
            | Self::SetResizable(handle, _)
            | Self::SetMinimumSize(handle, _)
            | Self::SetMaximumSize(handle, _)
            | Self::SetMinimizable(handle, _)
            | Self::SetMaximizable(handle, _)
            | Self::SetClosable(handle, _)
            | Self::SetDecorated(handle, _)
            | Self::SetShadow(handle, _)
            | Self::SetContentProtected(handle, _)
            | Self::SetWindowLevel(handle, _)
            | Self::SetFocusable(handle, _)
            | Self::SetSkipTaskbar(handle, _)
            | Self::SetVisibleOnAllWorkspaces(handle, _)
            | Self::SetOpacity(handle, _)
            | Self::SetIcon(handle, _)
            | Self::SetTaskbarProgress(handle, _, _)
            | Self::SetTaskbarOverlayIcon(handle, _, _)
            | Self::SetCursorVisible(handle, _)
            | Self::SetCursorGrab(handle, _)
            | Self::SetCursorHitTest(handle, _)
            | Self::SetCursorPosition(handle, _)
            | Self::SetAppearance(handle, _)
            | Self::SetBackgroundAppearance(handle, _)
            | Self::RequestAttention(handle) => *handle,
            #[cfg(feature = "inspector")]
            Self::SetInspector(handle, _) | Self::ToggleInspector(handle) => *handle,
        }
    }
}

/// Context provided while a view declares its element tree.
pub struct ViewContext<'a, V> {
    size: Size,
    scale_factor: f32,
    metrics: FrameMetrics,
    focused: Option<ElementId>,
    focused_path: Vec<ElementId>,
    request_animation_frame: bool,
    repaint_deadline: Option<Instant>,
    listeners: &'a mut ListenerRegistry,
    window: WindowHandle,
    window_state: WindowState,
    displays: &'a Displays,
    keyboard_layout: &'a KeyboardLayout,
    assets: &'a Assets,
    app_info: Option<&'a AppInfo>,
    app_paths: Option<&'a AppPaths>,
    system_info: &'a SystemInfo,
    system_preferences: &'a SystemPreferences,
    background_tasks: Option<&'a BackgroundTaskPoolHandle>,
    foreground_tasks: &'a ForegroundTaskSpawner,
    globals: &'a GlobalStore,
    event_proxy: Option<&'a EventLoopProxy<RuntimeEvent>>,
    marker: PhantomData<fn(&mut V)>,
}

impl<V: 'static> ViewContext<'_, V> {
    /// Read the viewport size and observe future size or scale-factor changes for this view.
    ///
    /// Views that do not read viewport geometry stay mounted during native window resize; the
    /// runtime relays out their retained declaration directly.
    pub fn size(&mut self) -> Size {
        self.listeners.observes_viewport = true;
        self.size
    }

    pub fn scale_factor(&mut self) -> f32 {
        self.listeners.observes_viewport = true;
        self.scale_factor
    }

    pub fn window_handle(&self) -> WindowHandle {
        self.window
    }

    /// Create a thread-safe handle that invalidates this window from background work.
    ///
    /// The returned handle wakes the native event loop and marks only this view dirty. It is
    /// suitable for long-lived producers such as terminal sessions, file watchers, or streaming
    /// transports that should leave the application asleep while no updates are available.
    pub fn window_invalidator(&self) -> WindowInvalidator {
        WindowInvalidator {
            runtime: self.event_proxy.map(|proxy| (proxy.clone(), self.window)),
        }
    }

    /// Read retained native window state and observe future state changes for this render branch.
    ///
    /// The observation is rebuilt declaratively. A view that never calls this method does not
    /// rebuild merely because its window moved, minimized, or changed z-order state.
    pub fn window_state(&mut self) -> WindowState {
        self.listeners.observes_window_state = true;
        self.window_state
    }

    pub fn window_bounds(&mut self) -> WindowBounds {
        self.window_state().bounds
    }

    /// Read and observe the bounded active-display snapshot.
    ///
    /// AppKit screen-parameter notifications replace this immutable snapshot only when its value
    /// changes. Views which never call a display method do not rebuild for display changes.
    pub fn displays(&mut self) -> &[Display] {
        self.listeners.observes_displays = true;
        self.displays.all()
    }

    pub fn primary_display(&mut self) -> Option<&Display> {
        self.listeners.observes_displays = true;
        self.displays.primary()
    }

    pub fn find_display(&mut self, id: DisplayId) -> Option<&Display> {
        self.listeners.observes_displays = true;
        self.displays.find(id)
    }

    /// Display currently containing this window, when both native placement and the latest
    /// snapshot are known.
    pub fn current_display(&mut self) -> Option<&Display> {
        self.listeners.observes_displays = true;
        self.listeners.observes_window_state = true;
        self.window_state
            .display_id
            .and_then(|id| self.displays.find(id))
    }

    /// Read and observe the active native keyboard-layout snapshot.
    ///
    /// macOS input-source notifications replace this immutable value only when the layout or its
    /// command translation changes. Views which never call this method do not rebuild for keyboard
    /// layout changes.
    pub fn keyboard_layout(&mut self) -> &KeyboardLayout {
        self.listeners.observes_keyboard_layout = true;
        self.keyboard_layout
    }

    /// Access the application's immutable asset source without subscribing the view to changes.
    pub fn assets(&self) -> &Assets {
        self.assets
    }

    /// GPUI-shaped alias for [`Self::assets`].
    pub fn asset_source(&self) -> &Assets {
        self.assets()
    }

    /// Immutable package identity supplied before application startup.
    pub fn app_info(&self) -> Option<&AppInfo> {
        self.app_info
    }

    /// Standard application paths resolved once during startup.
    pub fn app_paths(&self) -> Option<&AppPaths> {
        self.app_paths
    }

    /// Immutable operating-system and preferred-language snapshot captured at startup.
    pub fn system_info(&self) -> &SystemInfo {
        self.system_info
    }

    /// Read and observe the current system appearance and accessibility preferences.
    pub fn system_preferences(&mut self) -> SystemPreferences {
        self.listeners.observes_system_preferences = true;
        *self.system_preferences
    }

    /// Read and observe the effective native light/dark appearance for this window.
    pub fn appearance(&mut self) -> WindowAppearance {
        self.window_state().appearance
    }

    /// Metrics from the previously completed frame.
    pub fn metrics(&self) -> FrameMetrics {
        self.metrics
    }

    /// Create a stable identity that can be attached with [`crate::Element::track_focus`].
    pub fn focus_handle(&self, id: impl Into<ElementId>) -> FocusHandle {
        FocusHandle::new(id)
    }

    pub fn focused(&self) -> Option<ElementId> {
        self.focused
    }

    pub fn is_focused(&self, handle: FocusHandle) -> bool {
        self.focused == Some(handle.id())
    }

    /// Whether this scope is the focused element or an ancestor of it.
    pub fn contains_focused(&self, handle: FocusHandle) -> bool {
        self.focused_path.contains(&handle.id())
    }

    /// Whether an application-global value of this type has been installed.
    pub fn has_global<G: Global>(&self) -> bool {
        self.globals.has::<G>()
    }

    /// Read an application-global value without retaining a render observation.
    pub fn global<G: Global>(&self) -> Ref<'_, G> {
        self.globals.get::<G>()
    }

    /// Read an application-global value if one has been installed.
    pub fn try_global<G: Global>(&self) -> Option<Ref<'_, G>> {
        self.globals.try_get::<G>()
    }

    /// Read a global and conditionally invalidate this window when that type changes later.
    ///
    /// The observation is refreshed on every declarative rebuild, so omitting this call from a
    /// later branch automatically unsubscribes the window from repaint notifications.
    pub fn watch_global<G: Global, R>(&mut self, read: impl FnOnce(&G) -> R) -> R {
        self.listeners.observe_global(TypeId::of::<G>());
        let global = self.globals.get::<G>();
        read(&global)
    }

    /// Observe application-global changes with an explicit RAII lifetime.
    ///
    /// This matches GPUI's `observe_global` shape. Delivery is deferred until the mutating callback
    /// releases its borrows. Store the returned [`Subscription`] on the view, or call
    /// [`Subscription::detach`] to keep it until the window closes. The callback can read the new
    /// value through [`EventContext::global`].
    pub fn observe_global<G: Global>(
        &mut self,
        mut callback: impl FnMut(&mut V, &mut EventContext) + 'static,
    ) -> Subscription {
        let callback: GlobalObserverCallback = Rc::new(RefCell::new(
            move |view: &mut dyn Any, cx: &mut EventContext| {
                callback(
                    view.downcast_mut::<V>()
                        .expect("global observer received the wrong view type"),
                    cx,
                );
            },
        ));
        self.listeners.subscribe_global(TypeId::of::<G>(), callback)
    }

    /// Explicitly named alias for [`Self::observe_global`].
    pub fn subscribe_global<G: Global>(
        &mut self,
        callback: impl FnMut(&mut V, &mut EventContext) + 'static,
    ) -> Subscription {
        self.observe_global::<G>(callback)
    }

    /// Read shared state and retain a window-level observation for later updates.
    ///
    /// Calling [`Entity::update`] from any window then invalidates this view exactly once. The
    /// observation is refreshed on each declarative rebuild, so conditional reads automatically
    /// unsubscribe when that branch is no longer rendered.
    pub fn observe<T, R>(&mut self, entity: &Entity<T>, read: impl FnOnce(&T) -> R) -> R {
        self.listeners.observe_entity(entity.id());
        entity.read(read)
    }

    /// Subscribe this retained view to one typed event emitted by another entity.
    ///
    /// Delivery passes a temporary strong handle to the source entity and runs only after the
    /// emitting callback releases its borrows. Store the returned [`Subscription`] on the view to
    /// control its lifetime, or call [`Subscription::detach`] to retain it until the window closes.
    pub fn subscribe<T, E>(
        &mut self,
        entity: &Entity<T>,
        mut callback: impl FnMut(&mut V, Entity<T>, &E, &mut EventContext) + 'static,
    ) -> Subscription
    where
        T: EventEmitter<E>,
        E: Any,
    {
        let source = entity.downgrade();
        let callback: EntityEventCallback = Rc::new(RefCell::new(
            move |view: &mut dyn Any, event: &dyn Any, cx: &mut EventContext| {
                let Some(source) = source.upgrade() else {
                    return;
                };
                callback(
                    view.downcast_mut::<V>()
                        .expect("entity-event subscriber received the wrong view type"),
                    source,
                    event
                        .downcast_ref::<E>()
                        .expect("entity-event subscriber received the wrong event type"),
                    cx,
                );
            },
        ));
        self.listeners
            .subscribe_entity_event(entity.id(), TypeId::of::<E>(), callback)
    }

    /// Keep rendering at the display's cadence until a future frame omits this call.
    pub fn request_animation_frame(&mut self) {
        self.request_animation_frame = true;
    }

    /// Request one view repaint at or after an exact deadline.
    ///
    /// Repeated calls keep only the earliest deadline. Unlike [`Self::request_animation_frame`],
    /// this leaves the application asleep between now and the requested repaint.
    pub fn request_repaint_at(&mut self, deadline: Instant) {
        self.repaint_deadline = Some(
            self.repaint_deadline
                .map_or(deadline, |current| current.min(deadline)),
        );
    }

    /// Run a non-blocking future on QuickGUI's application-thread executor.
    ///
    /// The future starts on the next event-loop turn. Dropping the returned handle cancels it;
    /// [`Task::detach`] lets it continue until completion or until this window closes. Use the
    /// supplied [`AsyncViewContext`] for fallible view updates and exact, idle event-loop timers.
    pub fn spawn<Build, Fut, R>(&self, build: Build) -> Result<Task<R>, ForegroundTaskSpawnError>
    where
        Build: FnOnce(AsyncViewContext<V>) -> Fut,
        Fut: Future<Output = R> + 'static,
        R: 'static,
    {
        self.foreground_tasks
            .spawn::<V, _, _, _>(self.window, build)
    }

    /// Run blocking or CPU-heavy application work on QuickGUI's bounded worker pool.
    ///
    /// The completion is delivered on this window's UI thread and wakes the event loop exactly
    /// once. The window remains asleep while work is pending; no polling frame is required.
    pub fn spawn_background<T, Work, Complete>(
        &self,
        work: Work,
        complete: Complete,
    ) -> Result<(), TaskSpawnError>
    where
        T: Send + 'static,
        Work: FnOnce() -> T + Send + 'static,
        Complete:
            FnOnce(&mut V, Result<T, BackgroundTaskError>, &mut EventContext) + Send + 'static,
    {
        self.background_tasks
            .ok_or(TaskSpawnError::Unavailable)?
            .spawn::<V, T, Work, Complete>(self.window, work, complete)
    }

    /// Explicit alias for [`Self::spawn_background`].
    pub fn spawn_blocking<T, Work, Complete>(
        &self,
        work: Work,
        complete: Complete,
    ) -> Result<(), TaskSpawnError>
    where
        T: Send + 'static,
        Work: FnOnce() -> T + Send + 'static,
        Complete:
            FnOnce(&mut V, Result<T, BackgroundTaskError>, &mut EventContext) + Send + 'static,
    {
        self.spawn_background(work, complete)
    }

    /// Observe the exact teardown of one child window owned by this view.
    ///
    /// Registration is declarative and refreshed on every rebuild. The callback runs after the
    /// child and all of its descendants have been removed, but only while this parent remains
    /// open. It owns no native observer, polling task, timer, or idle scheduler source.
    pub fn on_child_window_closed(
        &mut self,
        child: WindowHandle,
        callback: impl Fn(&mut V, WindowHandle, &mut EventContext) + 'static,
    ) {
        assert_ne!(
            child, self.window,
            "a window cannot observe itself as a child"
        );
        assert!(
            self.listeners.child_window_closed.len() + self.listeners.any_child_window_closed.len()
                < MAX_CHILD_WINDOW_CLOSE_LISTENERS_PER_WINDOW,
            "a window cannot declare more than {MAX_CHILD_WINDOW_CLOSE_LISTENERS_PER_WINDOW} child-window close listeners"
        );
        let callback: ChildWindowClosedCallback = Arc::new(move |view, child, context| {
            callback(
                view.downcast_mut::<V>()
                    .expect("child-window close listener received the wrong view type"),
                child,
                context,
            );
        });
        let previous = self.listeners.child_window_closed.insert(child, callback);
        assert!(
            previous.is_none(),
            "child window {child:?} was observed more than once by the same view"
        );
    }

    /// Observe teardown of any direct child owned by this view.
    ///
    /// Unlike [`Self::on_child_window_closed`], this can be declared before a child handle exists,
    /// so a child opened and closed within the same event turn is still reported exactly once.
    /// Components should compare the delivered handle with their controlled child state.
    pub fn on_any_child_window_closed(
        &mut self,
        callback: impl Fn(&mut V, WindowHandle, &mut EventContext) + 'static,
    ) {
        assert!(
            self.listeners.child_window_closed.len() + self.listeners.any_child_window_closed.len()
                < MAX_CHILD_WINDOW_CLOSE_LISTENERS_PER_WINDOW,
            "a window cannot declare more than {MAX_CHILD_WINDOW_CLOSE_LISTENERS_PER_WINDOW} child-window close listeners"
        );
        self.listeners
            .any_child_window_closed
            .push(Arc::new(move |view, child, context| {
                callback(
                    view.downcast_mut::<V>()
                        .expect("child-window close listener received the wrong view type"),
                    child,
                    context,
                );
            }));
    }

    /// Register a stable, view-local click callback for use with [`crate::Element::on_click`].
    pub fn listener(
        &mut self,
        id: impl Into<ElementId>,
        callback: impl Fn(&mut V, &mut EventContext) + 'static,
    ) -> ClickListener<V> {
        let id = id.into();
        let callback = Arc::new(move |view: &mut dyn Any, context: &mut EventContext| {
            callback(
                view.downcast_mut::<V>()
                    .expect("click listener received the wrong view type"),
                context,
            );
        });
        let previous = self.listeners.clicks.insert(id, callback);
        assert!(
            previous.is_none(),
            "listener id {id:?} was registered more than once"
        );
        ClickListener {
            id,
            marker: PhantomData,
        }
    }

    /// Register a web-style secondary-click callback.
    ///
    /// Attach the returned handle with [`crate::Element::on_context_menu`]. The callback runs on
    /// button press at the original logical pointer position, before a native right-button drag
    /// can begin.
    pub fn context_menu_listener(
        &mut self,
        id: impl Into<ElementId>,
        callback: impl Fn(&mut V, &ContextMenuEvent, &mut EventContext) + 'static,
    ) -> ContextMenuListener<V> {
        let id = id.into();
        let callback: ContextMenuCallback = Arc::new(
            move |view: &mut dyn Any, event: &ContextMenuEvent, context: &mut EventContext| {
                callback(
                    view.downcast_mut::<V>()
                        .expect("context-menu listener received the wrong view type"),
                    event,
                    context,
                );
            },
        );
        let previous = self.listeners.context_menus.insert(id, callback);
        assert!(
            previous.is_none(),
            "context-menu listener id {id:?} was registered more than once"
        );
        ContextMenuListener {
            id,
            marker: PhantomData,
        }
    }

    /// Register a stable callback for a captured pointer interaction.
    ///
    /// Attach the returned handle with [`crate::Element::on_pointer`]. A press inside the element
    /// starts capture; move events and the terminal up/cancel event continue outside its bounds.
    pub fn pointer_listener(
        &mut self,
        id: impl Into<ElementId>,
        callback: impl Fn(&mut V, &PointerEvent, &mut EventContext) + 'static,
    ) -> PointerListener<V> {
        let id = id.into();
        let callback = Arc::new(
            move |view: &mut dyn Any, event: &PointerEvent, context: &mut EventContext| {
                callback(
                    view.downcast_mut::<V>()
                        .expect("pointer listener received the wrong view type"),
                    event,
                    context,
                );
            },
        );
        let previous = self.listeners.pointers.insert(id, callback);
        assert!(
            previous.is_none(),
            "pointer listener id {id:?} was registered more than once"
        );
        PointerListener {
            id,
            marker: PhantomData,
        }
    }

    /// Register a desktop mouse-down callback for attachment to one retained element.
    pub fn mouse_down_listener(
        &mut self,
        id: impl Into<ElementId>,
        callback: impl Fn(&mut V, &MouseDownEvent, &mut EventContext) + 'static,
    ) -> MouseDownListener<V> {
        let id = id.into();
        let callback: MouseListenerCallback = Arc::new(move |view, event, context| {
            let MouseListenerEvent::Down(event) = event else {
                unreachable!("mouse-down callback received the wrong event kind")
            };
            callback(
                view.downcast_mut::<V>()
                    .expect("mouse-down listener received the wrong view type"),
                event,
                context,
            );
        });
        MouseDownListener {
            id,
            key: self.listeners.push_mouse_listener(callback),
            marker: PhantomData,
        }
    }

    /// Register a desktop mouse-up callback for attachment to one retained element.
    pub fn mouse_up_listener(
        &mut self,
        id: impl Into<ElementId>,
        callback: impl Fn(&mut V, &MouseUpEvent, &mut EventContext) + 'static,
    ) -> MouseUpListener<V> {
        let id = id.into();
        let callback: MouseListenerCallback = Arc::new(move |view, event, context| {
            let MouseListenerEvent::Up(event) = event else {
                unreachable!("mouse-up callback received the wrong event kind")
            };
            callback(
                view.downcast_mut::<V>()
                    .expect("mouse-up listener received the wrong view type"),
                event,
                context,
            );
        });
        MouseUpListener {
            id,
            key: self.listeners.push_mouse_listener(callback),
            marker: PhantomData,
        }
    }

    /// Register a desktop mouse-motion callback for attachment to one retained element.
    pub fn mouse_move_listener(
        &mut self,
        id: impl Into<ElementId>,
        callback: impl Fn(&mut V, &MouseMoveEvent, &mut EventContext) + 'static,
    ) -> MouseMoveListener<V> {
        let id = id.into();
        let callback: MouseListenerCallback = Arc::new(move |view, event, context| {
            let MouseListenerEvent::Move(event) = event else {
                unreachable!("mouse-move callback received the wrong event kind")
            };
            callback(
                view.downcast_mut::<V>()
                    .expect("mouse-move listener received the wrong view type"),
                event,
                context,
            );
        });
        MouseMoveListener {
            id,
            key: self.listeners.push_mouse_listener(callback),
            marker: PhantomData,
        }
    }

    /// Register a native-window mouse-exit callback for attachment to one retained element.
    pub fn mouse_exit_listener(
        &mut self,
        id: impl Into<ElementId>,
        callback: impl Fn(&mut V, &MouseExitEvent, &mut EventContext) + 'static,
    ) -> MouseExitListener<V> {
        let id = id.into();
        let callback: MouseListenerCallback = Arc::new(move |view, event, context| {
            let MouseListenerEvent::Exit(event) = event else {
                unreachable!("mouse-exit callback received the wrong event kind")
            };
            callback(
                view.downcast_mut::<V>()
                    .expect("mouse-exit listener received the wrong view type"),
                event,
                context,
            );
        });
        MouseExitListener {
            id,
            key: self.listeners.push_mouse_listener(callback),
            marker: PhantomData,
        }
    }

    /// Register a web-style hover transition callback.
    pub fn hover_listener(
        &mut self,
        id: impl Into<ElementId>,
        callback: impl Fn(&mut V, &bool, &mut EventContext) + 'static,
    ) -> HoverListener<V> {
        let id = id.into();
        let callback: MouseListenerCallback = Arc::new(move |view, event, context| {
            let MouseListenerEvent::Hover(hovered) = event else {
                unreachable!("hover callback received the wrong event kind")
            };
            callback(
                view.downcast_mut::<V>()
                    .expect("hover listener received the wrong view type"),
                hovered,
                context,
            );
        });
        HoverListener {
            id,
            key: self.listeners.push_mouse_listener(callback),
            marker: PhantomData,
        }
    }

    /// Register focused key-down input for capture or bubble attachment on an element.
    ///
    /// Keymap actions run first. If they propagate, the raw key press traverses the retained
    /// focus path. Call [`EventContext::prevent_default`] to replace QuickGUI's editing, focus,
    /// activation, dismissal, or default close behavior without stopping another listener.
    pub fn key_down_listener(
        &mut self,
        id: impl Into<ElementId>,
        callback: impl Fn(&mut V, &KeyDownEvent, &mut EventContext) + 'static,
    ) -> KeyDownListener<V> {
        let id = id.into();
        let callback: KeyListenerCallback = Arc::new(move |view, event, context| {
            let KeyListenerEvent::Down(event) = event else {
                unreachable!("key-down callback received the wrong event kind")
            };
            callback(
                view.downcast_mut::<V>()
                    .expect("key-down listener received the wrong view type"),
                event,
                context,
            );
        });
        KeyDownListener {
            id,
            key: self.listeners.push_key_listener(callback),
            marker: PhantomData,
        }
    }

    /// Register focused key-up input for capture or bubble attachment on an element.
    pub fn key_up_listener(
        &mut self,
        id: impl Into<ElementId>,
        callback: impl Fn(&mut V, &KeyUpEvent, &mut EventContext) + 'static,
    ) -> KeyUpListener<V> {
        let id = id.into();
        let callback: KeyListenerCallback = Arc::new(move |view, event, context| {
            let KeyListenerEvent::Up(event) = event else {
                unreachable!("key-up callback received the wrong event kind")
            };
            callback(
                view.downcast_mut::<V>()
                    .expect("key-up listener received the wrong view type"),
                event,
                context,
            );
        });
        KeyUpListener {
            id,
            key: self.listeners.push_key_listener(callback),
            marker: PhantomData,
        }
    }

    /// Register scroll-wheel input for attachment with [`crate::Element::on_scroll_wheel`].
    ///
    /// Scroll events bubble through listening ancestors. Call
    /// [`EventContext::stop_propagation`] to stop that path or [`EventContext::prevent_default`]
    /// when the gesture should not move the retained scroll container underneath it.
    pub fn scroll_wheel_listener(
        &mut self,
        id: impl Into<ElementId>,
        callback: impl Fn(&mut V, &ScrollWheelEvent, &mut EventContext) + 'static,
    ) -> ScrollWheelListener<V> {
        let id = id.into();
        let callback: ScrollWheelCallback = Arc::new(move |view, event, context| {
            callback(
                view.downcast_mut::<V>()
                    .expect("scroll-wheel listener received the wrong view type"),
                event,
                context,
            );
        });
        let previous = self.listeners.scroll_wheels.insert(id, callback);
        assert!(
            previous.is_none(),
            "scroll-wheel listener id {id:?} was registered more than once"
        );
        ScrollWheelListener {
            id,
            marker: PhantomData,
        }
    }

    /// Register raw multi-contact touch input for attachment with [`crate::Element::on_touch`].
    ///
    /// The contact is hit-tested once at start and remains captured until its terminal end or
    /// cancellation event. Touch callbacks bubble through listening ancestors by default.
    pub fn touch_listener(
        &mut self,
        id: impl Into<ElementId>,
        callback: impl Fn(&mut V, &TouchEvent, &mut EventContext) + 'static,
    ) -> TouchListener<V> {
        let id = id.into();
        let callback: TouchCallback = Arc::new(move |view, event, context| {
            callback(
                view.downcast_mut::<V>()
                    .expect("touch listener received the wrong view type"),
                event,
                context,
            );
        });
        let previous = self.listeners.touches.insert(id, callback);
        assert!(
            previous.is_none(),
            "touch listener id {id:?} was registered more than once"
        );
        TouchListener {
            id,
            marker: PhantomData,
        }
    }

    /// Register Force Touch input for attachment with [`crate::Element::on_mouse_pressure`].
    pub fn mouse_pressure_listener(
        &mut self,
        id: impl Into<ElementId>,
        callback: impl Fn(&mut V, &MousePressureEvent, &mut EventContext) + 'static,
    ) -> MousePressureListener<V> {
        let id = id.into();
        let callback: MousePressureCallback = Arc::new(move |view, event, context| {
            callback(
                view.downcast_mut::<V>()
                    .expect("mouse-pressure listener received the wrong view type"),
                event,
                context,
            );
        });
        let previous = self.listeners.mouse_pressures.insert(id, callback);
        assert!(
            previous.is_none(),
            "mouse-pressure listener id {id:?} was registered more than once"
        );
        MousePressureListener {
            id,
            marker: PhantomData,
        }
    }

    /// Register pinch-to-zoom input for attachment with [`crate::Element::on_pinch`].
    pub fn pinch_listener(
        &mut self,
        id: impl Into<ElementId>,
        callback: impl Fn(&mut V, &PinchEvent, &mut EventContext) + 'static,
    ) -> PinchListener<V> {
        let id = id.into();
        let callback: PinchCallback = Arc::new(move |view, event, context| {
            callback(
                view.downcast_mut::<V>()
                    .expect("pinch listener received the wrong view type"),
                event,
                context,
            );
        });
        let previous = self.listeners.pinches.insert(id, callback);
        assert!(
            previous.is_none(),
            "pinch listener id {id:?} was registered more than once"
        );
        PinchListener {
            id,
            marker: PhantomData,
        }
    }

    /// Register two-finger rotation input for attachment with [`crate::Element::on_rotation`].
    pub fn rotation_listener(
        &mut self,
        id: impl Into<ElementId>,
        callback: impl Fn(&mut V, &RotationEvent, &mut EventContext) + 'static,
    ) -> RotationListener<V> {
        let id = id.into();
        let callback: RotationCallback = Arc::new(move |view, event, context| {
            callback(
                view.downcast_mut::<V>()
                    .expect("rotation listener received the wrong view type"),
                event,
                context,
            );
        });
        let previous = self.listeners.rotations.insert(id, callback);
        assert!(
            previous.is_none(),
            "rotation listener id {id:?} was registered more than once"
        );
        RotationListener {
            id,
            marker: PhantomData,
        }
    }

    /// Register smart-magnify input for attachment with [`crate::Element::on_smart_magnify`].
    pub fn smart_magnify_listener(
        &mut self,
        id: impl Into<ElementId>,
        callback: impl Fn(&mut V, &SmartMagnifyEvent, &mut EventContext) + 'static,
    ) -> SmartMagnifyListener<V> {
        let id = id.into();
        let callback: SmartMagnifyCallback = Arc::new(move |view, event, context| {
            callback(
                view.downcast_mut::<V>()
                    .expect("smart-magnify listener received the wrong view type"),
                event,
                context,
            );
        });
        let previous = self.listeners.smart_magnifies.insert(id, callback);
        assert!(
            previous.is_none(),
            "smart-magnify listener id {id:?} was registered more than once"
        );
        SmartMagnifyListener {
            id,
            marker: PhantomData,
        }
    }

    /// Register a typed drag source.
    ///
    /// The callback runs once after primary-button motion crosses the drag threshold. Its payload
    /// is retained only for that gesture, and its optional preview is painted on the GPU overlay
    /// plane without rebuilding the application view on every pointer move. On macOS, the same
    /// arbitrary Rust value can cross directly into another QuickGUI window without serialization.
    pub fn drag_listener<T: 'static>(
        &mut self,
        id: impl Into<ElementId>,
        callback: impl Fn(&mut V, &DragStartEvent, &mut EventContext) -> Drag<T> + 'static,
    ) -> DragListener<V, T> {
        let id = id.into();
        let callback: DragStartCallback = Arc::new(move |view, event, context| {
            let drag = callback(
                view.downcast_mut::<V>()
                    .expect("drag listener received the wrong view type"),
                event,
                context,
            );
            AnyDrag::new(drag)
        });
        let previous = self
            .listeners
            .drag_sources
            .insert(id, (TypeId::of::<T>(), callback));
        assert!(
            previous.is_none(),
            "drag listener id {id:?} was registered more than once"
        );
        DragListener {
            id,
            marker: PhantomData,
        }
    }

    /// Register a typed drop callback.
    ///
    /// The callback is considered compatible only when the active payload has exactly type `T`.
    /// [`DroppedFiles`] uses this same path for native file drops.
    pub fn drop_listener<T: 'static>(
        &mut self,
        id: impl Into<ElementId>,
        callback: impl Fn(&mut V, &T, &DropEvent, &mut EventContext) + 'static,
    ) -> DropListener<V, T> {
        let id = id.into();
        let callback: DropCallback = Arc::new(move |view, value, event, context| {
            callback(
                view.downcast_mut::<V>()
                    .expect("drop listener received the wrong view type"),
                value
                    .downcast_ref::<T>()
                    .expect("drop listener received the wrong payload type"),
                event,
                context,
            );
        });
        let previous = self
            .listeners
            .drops
            .insert((id, TypeId::of::<T>()), callback);
        assert!(
            previous.is_none(),
            "drop listener id {id:?} for this payload type was registered more than once"
        );
        self.listeners.drop_order.push((id, TypeId::of::<T>()));
        DropListener {
            id,
            marker: PhantomData,
        }
    }

    /// Register a stable controlled-value callback for [`crate::Element::on_input`].
    pub fn input_listener(
        &mut self,
        id: impl Into<ElementId>,
        callback: impl Fn(&mut V, &str, &mut EventContext) + 'static,
    ) -> InputListener<V> {
        let id = id.into();
        let callback = Arc::new(
            move |view: &mut dyn Any, value: &str, context: &mut EventContext| {
                callback(
                    view.downcast_mut::<V>()
                        .expect("input listener received the wrong view type"),
                    value,
                    context,
                );
            },
        );
        let previous = self.listeners.inputs.insert(id, callback);
        assert!(
            previous.is_none(),
            "input listener id {id:?} was registered more than once"
        );
        InputListener {
            id,
            marker: PhantomData,
        }
    }

    /// Register a stable callback for Return in a valid, single-line text input.
    ///
    /// The callback receives the current committed value. IME preedit text is never submitted,
    /// key repeat is ignored, and an element marked with [`crate::Element::invalid`] blocks the
    /// callback while retaining focus.
    pub fn submit_listener(
        &mut self,
        id: impl Into<ElementId>,
        callback: impl Fn(&mut V, &str, &mut EventContext) + 'static,
    ) -> SubmitListener<V> {
        let id = id.into();
        let callback = Arc::new(
            move |view: &mut dyn Any, value: &str, context: &mut EventContext| {
                callback(
                    view.downcast_mut::<V>()
                        .expect("submit listener received the wrong view type"),
                    value,
                    context,
                );
            },
        );
        let previous = self.listeners.submits.insert(id, callback);
        assert!(
            previous.is_none(),
            "submit listener id {id:?} was registered more than once"
        );
        SubmitListener {
            id,
            marker: PhantomData,
        }
    }

    /// Register a callback for a valid form submission.
    ///
    /// The event contains document-ordered, shared controlled values and identifies the input or
    /// button that initiated submission. Attach the returned binding with
    /// [`crate::Element::on_form_submit`].
    pub fn form_submit_listener(
        &mut self,
        id: impl Into<ElementId>,
        callback: impl Fn(&mut V, &FormSubmitEvent, &mut EventContext) + 'static,
    ) -> FormSubmitListener<V> {
        let id = id.into();
        let callback: FormSubmitCallback = Arc::new(move |view, event, context| {
            callback(
                view.downcast_mut::<V>()
                    .expect("form submit listener received the wrong view type"),
                event,
                context,
            );
        });
        let previous = self.listeners.form_submits.insert(id, callback);
        assert!(
            previous.is_none(),
            "form submit listener id {id:?} was registered more than once"
        );
        FormSubmitListener {
            id,
            marker: PhantomData,
        }
    }

    /// Register a callback for a form blocked by declaratively invalid controls.
    ///
    /// Reports are bounded, document ordered, and delivered after QuickGUI moves focus to the
    /// first focusable invalid control. Attach the binding with
    /// [`crate::Element::on_form_invalid`].
    pub fn form_invalid_listener(
        &mut self,
        id: impl Into<ElementId>,
        callback: impl Fn(&mut V, &ValidationReport, &mut EventContext) + 'static,
    ) -> FormInvalidListener<V> {
        let id = id.into();
        let callback: FormInvalidCallback = Arc::new(move |view, report, context| {
            callback(
                view.downcast_mut::<V>()
                    .expect("form invalid listener received the wrong view type"),
                report,
                context,
            );
        });
        let previous = self.listeners.form_invalids.insert(id, callback);
        assert!(
            previous.is_none(),
            "form invalid listener id {id:?} was registered more than once"
        );
        FormInvalidListener {
            id,
            marker: PhantomData,
        }
    }

    /// Register a callback for Escape and outside-pointer dismissal.
    pub fn dismiss_listener(
        &mut self,
        id: impl Into<ElementId>,
        callback: impl Fn(&mut V, &mut EventContext) + 'static,
    ) -> DismissListener<V> {
        let id = id.into();
        let callback = Arc::new(move |view: &mut dyn Any, context: &mut EventContext| {
            callback(
                view.downcast_mut::<V>()
                    .expect("dismiss listener received the wrong view type"),
                context,
            );
        });
        let previous = self.listeners.dismisses.insert(id, callback);
        assert!(
            previous.is_none(),
            "dismiss listener id {id:?} was registered more than once"
        );
        DismissListener {
            id,
            marker: PhantomData,
        }
    }

    /// Register a typed action callback for attachment with [`crate::Element::on_action`].
    pub fn action_listener<A: Action>(
        &mut self,
        id: impl Into<ElementId>,
        callback: impl Fn(&mut V, &A, &mut EventContext) + 'static,
    ) -> ActionListener<V, A> {
        let id = id.into();
        let erased: ActionCallback = Arc::new(move |view, action, context| {
            let action = action
                .downcast_ref::<A>()
                .expect("action listener received the wrong concrete action type");
            callback(
                view.downcast_mut::<V>()
                    .expect("action listener received the wrong view type"),
                action,
                context,
            );
        });
        let action_type = TypeId::of::<A>();
        ActionListener {
            id,
            key: self.listeners.push_action_listener(erased),
            action_type,
            marker: PhantomData,
        }
    }
}

type ClickCallback = Arc<dyn Fn(&mut dyn Any, &mut EventContext)>;
type PointerCallback = Arc<dyn Fn(&mut dyn Any, &PointerEvent, &mut EventContext)>;
type MouseListenerCallback = Arc<dyn Fn(&mut dyn Any, &MouseListenerEvent, &mut EventContext)>;
type KeyListenerCallback = Arc<dyn Fn(&mut dyn Any, &KeyListenerEvent, &mut EventContext)>;
type ScrollWheelCallback = Arc<dyn Fn(&mut dyn Any, &ScrollWheelEvent, &mut EventContext)>;
type TouchCallback = Arc<dyn Fn(&mut dyn Any, &TouchEvent, &mut EventContext)>;
type ContextMenuCallback = Arc<dyn Fn(&mut dyn Any, &ContextMenuEvent, &mut EventContext)>;
type MousePressureCallback = Arc<dyn Fn(&mut dyn Any, &MousePressureEvent, &mut EventContext)>;
type PinchCallback = Arc<dyn Fn(&mut dyn Any, &PinchEvent, &mut EventContext)>;
type RotationCallback = Arc<dyn Fn(&mut dyn Any, &RotationEvent, &mut EventContext)>;
type SmartMagnifyCallback = Arc<dyn Fn(&mut dyn Any, &SmartMagnifyEvent, &mut EventContext)>;
type InputCallback = Arc<dyn Fn(&mut dyn Any, &str, &mut EventContext)>;
type FormSubmitCallback = Arc<dyn Fn(&mut dyn Any, &FormSubmitEvent, &mut EventContext)>;
type FormInvalidCallback = Arc<dyn Fn(&mut dyn Any, &ValidationReport, &mut EventContext)>;
type ActionCallback = Arc<dyn Fn(&mut dyn Any, &dyn Any, &mut EventContext)>;
type DragStartCallback = Arc<dyn Fn(&mut dyn Any, &DragStartEvent, &mut EventContext) -> AnyDrag>;
type DropCallback = Arc<dyn Fn(&mut dyn Any, &dyn Any, &DropEvent, &mut EventContext)>;
type EntityEventCallback = Rc<RefCell<dyn FnMut(&mut dyn Any, &dyn Any, &mut EventContext)>>;
type GlobalObserverCallback = Rc<RefCell<dyn FnMut(&mut dyn Any, &mut EventContext)>>;
type ChildWindowClosedCallback = Arc<dyn Fn(&mut dyn Any, WindowHandle, &mut EventContext)>;

#[derive(Clone, Copy, Debug, PartialEq)]
enum MouseListenerEvent {
    Down(MouseDownEvent),
    Up(MouseUpEvent),
    Move(MouseMoveEvent),
    Exit(MouseExitEvent),
    Hover(bool),
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum KeyListenerEvent {
    Down(KeyDownEvent),
    Up(KeyUpEvent),
}

impl KeyListenerEvent {
    fn kind(&self) -> KeyListenerKind {
        match self {
            Self::Down(_) => KeyListenerKind::Down,
            Self::Up(_) => KeyListenerKind::Up,
        }
    }
}

#[derive(Clone)]
struct EntityEventSubscription {
    state: Rc<SubscriptionState>,
    callback: EntityEventCallback,
}

impl EntityEventSubscription {
    fn is_active(&self) -> bool {
        self.state.is_active()
    }
}

#[derive(Clone)]
struct GlobalObserverSubscription {
    global_type: TypeId,
    state: Rc<SubscriptionState>,
    callback: GlobalObserverCallback,
}

impl GlobalObserverSubscription {
    fn is_active(&self) -> bool {
        self.state.is_active()
    }
}

#[derive(Default)]
struct ListenerRegistry {
    clicks: HashMap<ElementId, ClickCallback>,
    pointers: HashMap<ElementId, PointerCallback>,
    mouse_listeners: Vec<MouseListenerCallback>,
    key_listeners: Vec<KeyListenerCallback>,
    scroll_wheels: HashMap<ElementId, ScrollWheelCallback>,
    touches: HashMap<ElementId, TouchCallback>,
    context_menus: HashMap<ElementId, ContextMenuCallback>,
    mouse_pressures: HashMap<ElementId, MousePressureCallback>,
    pinches: HashMap<ElementId, PinchCallback>,
    rotations: HashMap<ElementId, RotationCallback>,
    smart_magnifies: HashMap<ElementId, SmartMagnifyCallback>,
    drag_sources: HashMap<ElementId, (TypeId, DragStartCallback)>,
    drops: HashMap<(ElementId, TypeId), DropCallback>,
    drop_order: Vec<(ElementId, TypeId)>,
    inputs: HashMap<ElementId, InputCallback>,
    submits: HashMap<ElementId, InputCallback>,
    form_submits: HashMap<ElementId, FormSubmitCallback>,
    form_invalids: HashMap<ElementId, FormInvalidCallback>,
    dismisses: HashMap<ElementId, ClickCallback>,
    actions: Vec<ActionCallback>,
    observed_entities: HashSet<EntityId>,
    observed_globals: HashSet<TypeId>,
    observes_window_state: bool,
    observes_viewport: bool,
    observes_displays: bool,
    observes_keyboard_layout: bool,
    observes_system_preferences: bool,
    entity_events: HashMap<(EntityId, TypeId), Vec<EntityEventSubscription>>,
    entity_subscription_count: usize,
    global_observers: Vec<GlobalObserverSubscription>,
    child_window_closed: HashMap<WindowHandle, ChildWindowClosedCallback>,
    any_child_window_closed: Vec<ChildWindowClosedCallback>,
}

impl ListenerRegistry {
    fn requires_window_state_rebuild(&self, state_changed: bool) -> bool {
        state_changed && self.observes_window_state
    }

    fn push_mouse_listener(&mut self, callback: MouseListenerCallback) -> MouseListenerKey {
        assert!(
            self.mouse_listeners.len() < MAX_MOUSE_LISTENERS_PER_WINDOW,
            "a window cannot declare more than {MAX_MOUSE_LISTENERS_PER_WINDOW} targeted desktop mouse listeners"
        );
        let key = MouseListenerKey(self.mouse_listeners.len() as u32);
        self.mouse_listeners.push(callback);
        key
    }

    fn mouse_listener(&self, key: MouseListenerKey) -> Option<MouseListenerCallback> {
        self.mouse_listeners.get(key.0 as usize).cloned()
    }

    fn push_key_listener(&mut self, callback: KeyListenerCallback) -> KeyListenerKey {
        assert!(
            self.key_listeners.len() < MAX_KEY_LISTENERS_PER_WINDOW,
            "a window cannot declare more than {MAX_KEY_LISTENERS_PER_WINDOW} focused key listeners"
        );
        let key = KeyListenerKey(self.key_listeners.len() as u32);
        self.key_listeners.push(callback);
        key
    }

    fn key_listener(&self, key: KeyListenerKey) -> Option<KeyListenerCallback> {
        self.key_listeners.get(key.0 as usize).cloned()
    }

    fn push_action_listener(&mut self, callback: ActionCallback) -> ActionListenerKey {
        assert!(
            self.actions.len() < MAX_ACTION_LISTENERS_PER_WINDOW,
            "a window cannot declare more than {MAX_ACTION_LISTENERS_PER_WINDOW} typed action listeners"
        );
        let key = ActionListenerKey(self.actions.len() as u32);
        self.actions.push(callback);
        key
    }

    fn action_listener(&self, key: ActionListenerKey) -> Option<ActionCallback> {
        self.actions.get(key.0 as usize).cloned()
    }

    fn observe_global(&mut self, global_type: TypeId) {
        if self.observed_globals.contains(&global_type) {
            return;
        }
        assert!(
            self.observed_globals.len() < MAX_OBSERVED_GLOBALS_PER_WINDOW,
            "a window cannot observe more than {MAX_OBSERVED_GLOBALS_PER_WINDOW} global types"
        );
        self.observed_globals.insert(global_type);
    }

    fn observes_global_change(&self, global_types: &[TypeId], all: bool) -> bool {
        all || global_types
            .iter()
            .any(|global_type| self.observed_globals.contains(global_type))
    }

    fn subscribe_global(
        &mut self,
        global_type: TypeId,
        callback: GlobalObserverCallback,
    ) -> Subscription {
        if self.global_observers.len() >= MAX_GLOBAL_SUBSCRIPTIONS_PER_WINDOW {
            self.prune_global_subscriptions();
        }
        assert!(
            self.global_observers.len() < MAX_GLOBAL_SUBSCRIPTIONS_PER_WINDOW,
            "a window cannot retain more than {MAX_GLOBAL_SUBSCRIPTIONS_PER_WINDOW} global subscriptions"
        );
        let (subscription, state) = Subscription::new();
        self.global_observers.push(GlobalObserverSubscription {
            global_type,
            state,
            callback,
        });
        subscription
    }

    fn has_global_subscribers(&self, global_type: Option<TypeId>) -> bool {
        self.global_observers.iter().any(|subscription| {
            subscription.is_active()
                && global_type.is_none_or(|global_type| subscription.global_type == global_type)
        })
    }

    fn global_subscriptions(&self, global_type: Option<TypeId>) -> Vec<GlobalObserverSubscription> {
        self.global_observers
            .iter()
            .filter(|subscription| {
                subscription.is_active()
                    && global_type.is_none_or(|global_type| subscription.global_type == global_type)
            })
            .cloned()
            .collect()
    }

    fn prune_global_subscriptions(&mut self) {
        self.global_observers
            .retain(GlobalObserverSubscription::is_active);
    }

    fn observe_entity(&mut self, entity: EntityId) {
        if self.observed_entities.contains(&entity) {
            return;
        }
        assert!(
            self.observed_entities.len() < MAX_OBSERVED_ENTITIES_PER_WINDOW,
            "a window cannot observe more than {MAX_OBSERVED_ENTITIES_PER_WINDOW} entities"
        );
        self.observed_entities.insert(entity);
    }

    fn observes_entity_change(&self, entities: &[EntityId], all: bool) -> bool {
        all || entities
            .iter()
            .any(|entity| self.observed_entities.contains(entity))
    }

    fn subscribe_entity_event(
        &mut self,
        entity: EntityId,
        event_type: TypeId,
        callback: EntityEventCallback,
    ) -> Subscription {
        if self.entity_subscription_count >= MAX_ENTITY_SUBSCRIPTIONS_PER_WINDOW {
            self.prune_entity_event_subscriptions();
        }
        assert!(
            self.entity_subscription_count < MAX_ENTITY_SUBSCRIPTIONS_PER_WINDOW,
            "a window cannot retain more than {MAX_ENTITY_SUBSCRIPTIONS_PER_WINDOW} entity-event subscriptions"
        );
        let (subscription, state) = Subscription::new();
        self.entity_events
            .entry((entity, event_type))
            .or_default()
            .push(EntityEventSubscription { state, callback });
        self.entity_subscription_count += 1;
        subscription
    }

    fn has_entity_event_subscribers(&self, entity: EntityId, event_type: TypeId) -> bool {
        self.entity_events
            .get(&(entity, event_type))
            .is_some_and(|subscriptions| {
                subscriptions.iter().any(EntityEventSubscription::is_active)
            })
    }

    fn entity_event_callbacks(
        &self,
        entity: EntityId,
        event_type: TypeId,
    ) -> Vec<EntityEventCallback> {
        self.entity_events
            .get(&(entity, event_type))
            .into_iter()
            .flatten()
            .filter(|subscription| subscription.is_active())
            .map(|subscription| subscription.callback.clone())
            .collect()
    }

    fn prune_entity_event_subscriptions(&mut self) {
        self.entity_events.retain(|_, subscriptions| {
            subscriptions.retain(EntityEventSubscription::is_active);
            !subscriptions.is_empty()
        });
        self.entity_subscription_count = self.entity_events.values().map(Vec::len).sum();
    }

    fn clear(&mut self) {
        self.clicks.clear();
        self.pointers.clear();
        self.mouse_listeners.clear();
        self.key_listeners.clear();
        self.scroll_wheels.clear();
        self.touches.clear();
        self.context_menus.clear();
        self.mouse_pressures.clear();
        self.pinches.clear();
        self.rotations.clear();
        self.smart_magnifies.clear();
        self.drag_sources.clear();
        self.drops.clear();
        self.drop_order.clear();
        self.inputs.clear();
        self.submits.clear();
        self.form_submits.clear();
        self.form_invalids.clear();
        self.dismisses.clear();
        self.actions.clear();
        self.observed_entities.clear();
        self.observed_globals.clear();
        self.observes_window_state = false;
        self.observes_viewport = false;
        self.observes_displays = false;
        self.observes_keyboard_layout = false;
        self.observes_system_preferences = false;
        self.child_window_closed.clear();
        self.any_child_window_closed.clear();
        self.prune_entity_event_subscriptions();
        self.prune_global_subscriptions();
    }
}

/// An opaque click binding returned by [`ViewContext::listener`].
pub struct ClickListener<V> {
    id: ElementId,
    marker: PhantomData<fn(&mut V)>,
}

/// An opaque captured-pointer binding returned by [`ViewContext::pointer_listener`].
pub struct PointerListener<V> {
    id: ElementId,
    marker: PhantomData<fn(&mut V)>,
}

/// An opaque targeted mouse-down binding returned by [`ViewContext::mouse_down_listener`].
pub struct MouseDownListener<V> {
    id: ElementId,
    key: MouseListenerKey,
    marker: PhantomData<fn(&mut V)>,
}

/// An opaque targeted mouse-up binding returned by [`ViewContext::mouse_up_listener`].
pub struct MouseUpListener<V> {
    id: ElementId,
    key: MouseListenerKey,
    marker: PhantomData<fn(&mut V)>,
}

/// An opaque targeted mouse-motion binding returned by [`ViewContext::mouse_move_listener`].
pub struct MouseMoveListener<V> {
    id: ElementId,
    key: MouseListenerKey,
    marker: PhantomData<fn(&mut V)>,
}

/// An opaque targeted native-window mouse-exit binding.
pub struct MouseExitListener<V> {
    id: ElementId,
    key: MouseListenerKey,
    marker: PhantomData<fn(&mut V)>,
}

/// An opaque web-style hover transition binding returned by [`ViewContext::hover_listener`].
pub struct HoverListener<V> {
    id: ElementId,
    key: MouseListenerKey,
    marker: PhantomData<fn(&mut V)>,
}

/// An opaque focused key-down binding returned by [`ViewContext::key_down_listener`].
pub struct KeyDownListener<V> {
    id: ElementId,
    key: KeyListenerKey,
    marker: PhantomData<fn(&mut V)>,
}

/// An opaque focused key-up binding returned by [`ViewContext::key_up_listener`].
pub struct KeyUpListener<V> {
    id: ElementId,
    key: KeyListenerKey,
    marker: PhantomData<fn(&mut V)>,
}

/// An opaque scroll-wheel binding returned by [`ViewContext::scroll_wheel_listener`].
pub struct ScrollWheelListener<V> {
    id: ElementId,
    marker: PhantomData<fn(&mut V)>,
}

/// An opaque raw-touch binding returned by [`ViewContext::touch_listener`].
pub struct TouchListener<V> {
    id: ElementId,
    marker: PhantomData<fn(&mut V)>,
}

/// An opaque secondary-click binding returned by [`ViewContext::context_menu_listener`].
pub struct ContextMenuListener<V> {
    id: ElementId,
    marker: PhantomData<fn(&mut V)>,
}

/// An opaque Force Touch binding returned by [`ViewContext::mouse_pressure_listener`].
pub struct MousePressureListener<V> {
    id: ElementId,
    marker: PhantomData<fn(&mut V)>,
}

/// An opaque pinch binding returned by [`ViewContext::pinch_listener`].
pub struct PinchListener<V> {
    id: ElementId,
    marker: PhantomData<fn(&mut V)>,
}

/// An opaque rotation binding returned by [`ViewContext::rotation_listener`].
pub struct RotationListener<V> {
    id: ElementId,
    marker: PhantomData<fn(&mut V)>,
}

/// An opaque smart-magnify binding returned by [`ViewContext::smart_magnify_listener`].
pub struct SmartMagnifyListener<V> {
    id: ElementId,
    marker: PhantomData<fn(&mut V)>,
}

/// A typed payload and optional GPU preview produced when a drag starts.
pub struct Drag<T> {
    value: T,
    preview: Option<Element>,
    cursor_offset: Option<Point>,
    external_payload: Option<ExternalDragPayload>,
}

impl<T> Drag<T> {
    pub fn new(value: T) -> Self {
        Self {
            value,
            preview: None,
            cursor_offset: None,
            external_payload: None,
        }
    }

    /// Paint this element above the application while the pointer is dragging.
    pub fn preview(mut self, preview: impl IntoElement) -> Self {
        self.preview = Some(preview.into_element());
        self
    }

    /// Position the preview at this logical offset from the pointer.
    ///
    /// Without an explicit offset, QuickGUI preserves the pointer's location inside the source
    /// element, matching a native direct-manipulation drag.
    pub fn cursor_offset(mut self, offset: Point) -> Self {
        self.cursor_offset = Some(offset);
        self
    }

    /// Add a public native representation when this drag leaves its QuickGUI window.
    ///
    /// macOS always offers the original typed Rust value to other QuickGUI windows in this process.
    /// This optional representation additionally exposes existing files/directories, plain text,
    /// or an absolute URL to native applications. The retained GPU path remains active until the
    /// pointer exits, so attaching a representation adds no idle work.
    pub fn external_payload(mut self, payload: ExternalDragPayload) -> Self {
        self.external_payload = Some(payload);
        self
    }

    /// Offer existing files or directories when this drag leaves the window.
    pub fn external_files(self, paths: FileDragPaths) -> Self {
        self.external_payload(ExternalDragPayload::Files(paths))
    }

    /// Offer bounded plain text when this drag leaves the window.
    pub fn external_text(self, text: impl Into<ExternalDragText>) -> Self {
        self.external_payload(ExternalDragPayload::Text(text.into()))
    }

    /// Offer one validated absolute URL when this drag leaves the window.
    pub fn external_url(self, url: ExternalDragUrl) -> Self {
        self.external_payload(ExternalDragPayload::Url(url))
    }
}

struct AnyDrag {
    value: Arc<dyn Any>,
    value_type: TypeId,
    preview: Option<Element>,
    cursor_offset: Option<Point>,
    external_payload: Option<ExternalDragPayload>,
}

impl AnyDrag {
    fn new<T: 'static>(drag: Drag<T>) -> Self {
        Self {
            value: Arc::new(drag.value),
            value_type: TypeId::of::<T>(),
            preview: drag.preview,
            cursor_offset: drag.cursor_offset,
            external_payload: drag.external_payload,
        }
    }
}

/// An opaque typed drag-source binding returned by [`ViewContext::drag_listener`].
pub struct DragListener<V, T> {
    id: ElementId,
    marker: PhantomData<fn(&mut V, T)>,
}

impl<V, T> DragListener<V, T> {
    pub(crate) fn id(&self) -> ElementId {
        self.id
    }
}

/// An opaque typed drop-target binding returned by [`ViewContext::drop_listener`].
pub struct DropListener<V, T> {
    id: ElementId,
    marker: PhantomData<fn(&mut V, T)>,
}

impl<V, T> DropListener<V, T> {
    pub(crate) fn id(&self) -> ElementId {
        self.id
    }
}

impl<V> PointerListener<V> {
    pub(crate) fn id(&self) -> ElementId {
        self.id
    }
}

macro_rules! impl_mouse_listener_handle {
    ($name:ident) => {
        impl<V> $name<V> {
            pub(crate) fn id(&self) -> ElementId {
                self.id
            }

            pub(crate) fn key(&self) -> MouseListenerKey {
                self.key
            }
        }
    };
}

impl_mouse_listener_handle!(MouseDownListener);
impl_mouse_listener_handle!(MouseUpListener);
impl_mouse_listener_handle!(MouseMoveListener);
impl_mouse_listener_handle!(MouseExitListener);
impl_mouse_listener_handle!(HoverListener);

macro_rules! impl_key_listener_handle {
    ($name:ident) => {
        impl<V> $name<V> {
            pub(crate) fn id(&self) -> ElementId {
                self.id
            }

            pub(crate) fn key(&self) -> KeyListenerKey {
                self.key
            }
        }
    };
}

impl_key_listener_handle!(KeyDownListener);
impl_key_listener_handle!(KeyUpListener);

impl<V> ScrollWheelListener<V> {
    pub(crate) fn id(&self) -> ElementId {
        self.id
    }
}

impl<V> TouchListener<V> {
    pub(crate) fn id(&self) -> ElementId {
        self.id
    }
}

impl<V> ContextMenuListener<V> {
    pub(crate) fn id(&self) -> ElementId {
        self.id
    }
}

impl<V> MousePressureListener<V> {
    pub(crate) fn id(&self) -> ElementId {
        self.id
    }
}

impl<V> PinchListener<V> {
    pub(crate) fn id(&self) -> ElementId {
        self.id
    }
}

impl<V> RotationListener<V> {
    pub(crate) fn id(&self) -> ElementId {
        self.id
    }
}

impl<V> SmartMagnifyListener<V> {
    pub(crate) fn id(&self) -> ElementId {
        self.id
    }
}

impl<V> ClickListener<V> {
    pub(crate) fn id(&self) -> ElementId {
        self.id
    }
}

/// An opaque text-change binding returned by [`ViewContext::input_listener`].
pub struct InputListener<V> {
    id: ElementId,
    marker: PhantomData<fn(&mut V)>,
}

/// An opaque single-line submit binding returned by [`ViewContext::submit_listener`].
pub struct SubmitListener<V> {
    id: ElementId,
    marker: PhantomData<fn(&mut V)>,
}

/// An opaque valid-form binding returned by [`ViewContext::form_submit_listener`].
pub struct FormSubmitListener<V> {
    id: ElementId,
    marker: PhantomData<fn(&mut V)>,
}

/// An opaque invalid-form binding returned by [`ViewContext::form_invalid_listener`].
pub struct FormInvalidListener<V> {
    id: ElementId,
    marker: PhantomData<fn(&mut V)>,
}

impl<V> InputListener<V> {
    pub(crate) fn id(&self) -> ElementId {
        self.id
    }
}

impl<V> SubmitListener<V> {
    pub(crate) fn id(&self) -> ElementId {
        self.id
    }
}

impl<V> FormSubmitListener<V> {
    pub(crate) fn id(&self) -> ElementId {
        self.id
    }
}

impl<V> FormInvalidListener<V> {
    pub(crate) fn id(&self) -> ElementId {
        self.id
    }
}

/// An opaque dismissal binding returned by [`ViewContext::dismiss_listener`].
pub struct DismissListener<V> {
    id: ElementId,
    marker: PhantomData<fn(&mut V)>,
}

impl<V> DismissListener<V> {
    pub(crate) fn id(&self) -> ElementId {
        self.id
    }
}

macro_rules! impl_copy_listener_handle {
    ($name:ident<$($parameter:ident),+>) => {
        impl<$($parameter),+> Copy for $name<$($parameter),+> {}

        impl<$($parameter),+> Clone for $name<$($parameter),+> {
            fn clone(&self) -> Self {
                *self
            }
        }
    };
}

impl_copy_listener_handle!(ClickListener<V>);
impl_copy_listener_handle!(PointerListener<V>);
impl_copy_listener_handle!(MouseDownListener<V>);
impl_copy_listener_handle!(MouseUpListener<V>);
impl_copy_listener_handle!(MouseMoveListener<V>);
impl_copy_listener_handle!(MouseExitListener<V>);
impl_copy_listener_handle!(HoverListener<V>);
impl_copy_listener_handle!(ScrollWheelListener<V>);
impl_copy_listener_handle!(TouchListener<V>);
impl_copy_listener_handle!(ContextMenuListener<V>);
impl_copy_listener_handle!(MousePressureListener<V>);
impl_copy_listener_handle!(PinchListener<V>);
impl_copy_listener_handle!(RotationListener<V>);
impl_copy_listener_handle!(SmartMagnifyListener<V>);
impl_copy_listener_handle!(DragListener<V, T>);
impl_copy_listener_handle!(DropListener<V, T>);
impl_copy_listener_handle!(InputListener<V>);
impl_copy_listener_handle!(SubmitListener<V>);
impl_copy_listener_handle!(FormSubmitListener<V>);
impl_copy_listener_handle!(FormInvalidListener<V>);
impl_copy_listener_handle!(DismissListener<V>);

#[derive(Debug, Error)]
pub enum AppError {
    #[error("could not create the application event loop: {0}")]
    EventLoop(#[from] winit::error::EventLoopError),
    #[error("could not create the application window: {0}")]
    Window(String),
    #[error("could not initialize GPU rendering: {0}")]
    GraphicsInitialization(String),
    #[error("GPU rendering failed: {0}")]
    Render(String),
    #[error("view layout or painting failed: {0}")]
    View(String),
    #[error("platform integration failed: {0}")]
    Platform(String),
    #[error(transparent)]
    Asset(#[from] AssetError),
}

/// Result of one externally driven application-loop turn.
///
/// [`AppRunner`] returns control after a native redraw or after its caller-provided timeout. This
/// lets another runtime, such as Bun, service its own tasks without moving QuickGUI or AppKit off
/// the platform application thread.
#[cfg(not(target_arch = "wasm32"))]
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum AppRunStatus {
    Continue,
    Exited(i32),
}

/// Thread-safe wake handle for an externally pumped [`AppRunner`].
///
/// Embedding runtimes can block the platform event loop indefinitely, then use this handle from a
/// worker or command producer when native work becomes ready. A blocked [`AppRunner::pump`] call
/// returns without forcing a periodic polling timeout.
#[cfg(not(target_arch = "wasm32"))]
#[derive(Clone)]
pub struct AppRunnerWaker {
    proxy: EventLoopProxy<RuntimeEvent>,
}

#[cfg(not(target_arch = "wasm32"))]
impl AppRunnerWaker {
    /// Wake the application event loop, returning `false` after it has closed.
    pub fn wake(&self) -> bool {
        self.proxy
            .send_event(RuntimeEvent::ExternalCommandsReady)
            .is_ok()
    }
}

/// Thread-safe invalidation handle for one retained window.
///
/// Clones can be moved into background threads. Calling [`Self::invalidate`] is coalesced by the
/// window scheduler, so a burst of updates results in at most one pending redraw.
#[derive(Clone)]
pub struct WindowInvalidator {
    runtime: Option<(EventLoopProxy<RuntimeEvent>, WindowHandle)>,
}

impl WindowInvalidator {
    /// Wake the application and rebuild this window, returning `false` after the event loop closes.
    pub fn invalidate(&self) -> bool {
        self.runtime.as_ref().is_some_and(|(proxy, window)| {
            proxy
                .send_event(RuntimeEvent::InvalidateWindow(*window))
                .is_ok()
        })
    }
}

type OpenUrlsCallback = Box<dyn FnMut(OpenUrls, &mut EventContext)>;
type ReopenCallback = Box<dyn FnMut(bool, &mut EventContext)>;
type SystemWakeCallback = Box<dyn FnMut(&mut EventContext)>;
type KeyboardLayoutCallback = Box<dyn FnMut(&KeyboardLayout, &mut EventContext)>;
type SystemNotificationResponseCallback =
    Box<dyn FnMut(SystemNotificationResponse, &mut EventContext)>;
type GlobalShortcutCallback = Box<dyn FnMut(GlobalShortcutEvent, &mut EventContext)>;
type SecondInstanceCallback = Box<dyn FnMut(SecondInstanceEvent, &mut EventContext)>;
type PowerEventCallback = Box<dyn FnMut(PowerEvent, &mut EventContext)>;
type TrayEventCallback = Box<dyn FnMut(TrayEvent, &mut EventContext)>;
type WindowClosedCallback = Box<dyn FnMut(WindowHandle, &mut EventContext)>;
type QuitCallback = Box<dyn FnMut(QuitRequest, &mut EventContext)>;

#[derive(Default)]
struct ApplicationCallbacks {
    open_urls: Option<OpenUrlsCallback>,
    reopen: Option<ReopenCallback>,
    system_wake: Option<SystemWakeCallback>,
    keyboard_layout: Option<KeyboardLayoutCallback>,
    system_notification_response: Option<SystemNotificationResponseCallback>,
    global_shortcut: Option<GlobalShortcutCallback>,
    second_instance: Option<SecondInstanceCallback>,
    power_event: Option<PowerEventCallback>,
    tray_event: Option<TrayEventCallback>,
    window_closed: Option<WindowClosedCallback>,
    before_quit: Option<QuitCallback>,
    will_quit: Option<QuitCallback>,
}

mod application;
#[cfg(not(target_arch = "wasm32"))]
mod deep_link;
mod external;
#[cfg(not(target_arch = "wasm32"))]
pub use application::AppRunner;
pub use application::{App, Application};
mod global_shortcut;
mod integration;
mod platform_dialog;
mod power_monitor;
#[cfg(any(
    target_os = "macos",
    target_os = "windows",
    target_os = "linux",
    target_os = "freebsd",
    target_os = "dragonfly",
    target_os = "openbsd",
    target_os = "netbsd"
))]
mod single_instance;
mod tray;
#[cfg(target_os = "windows")]
mod windows_menu;
#[cfg(target_os = "windows")]
mod windows_shell;
#[cfg(target_os = "windows")]
mod windows_window;

#[cfg(target_os = "windows")]
pub(crate) fn validate_windows_notification_app_info(
    app_info: Option<&AppInfo>,
) -> Result<(), PlatformError> {
    let app_id = app_info.map(AppInfo::identifier).ok_or_else(|| {
        PlatformError::Platform(
            "Windows system notifications require AppInfo with an application identifier".into(),
        )
    })?;
    windows_shell::validate_app_id(app_id)
}

#[cfg(target_os = "windows")]
use windows_window::{
    current_cursor_screen_position as windows_cursor_screen_position, set_window_focusable,
    set_window_opacity,
};

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
fn set_window_focusable(_window: &Arc<Window>, _focusable: bool) -> Result<(), String> {
    Ok(())
}

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
fn set_window_opacity(_window: &Arc<Window>, _opacity: f32) -> Result<(), String> {
    Ok(())
}

#[cfg(not(target_os = "macos"))]
fn set_window_visible_on_all_workspaces(
    _window: &Arc<Window>,
    _visible: bool,
) -> Result<(), String> {
    Ok(())
}

pub(crate) fn cursor_screen_position(displays: &Displays) -> Result<Point, PlatformError> {
    #[cfg(target_os = "macos")]
    {
        let _ = displays;
        macos_cursor_screen_position().ok_or(PlatformError::Unavailable)
    }
    #[cfg(target_os = "windows")]
    {
        windows_cursor_screen_position(displays)
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        let _ = displays;
        Err(PlatformError::Unsupported)
    }
}
#[cfg(not(target_arch = "wasm32"))]
pub use deep_link::MAX_DEEP_LINK_ARGUMENTS;
pub use global_shortcut::{
    GlobalShortcutEvent, MAX_GLOBAL_SHORTCUT_ACCELERATOR_BYTES, MAX_GLOBAL_SHORTCUTS,
};
pub use integration::DesktopIntegrationSupport;
pub use power_monitor::PowerEvent;
#[cfg(any(
    target_os = "macos",
    target_os = "windows",
    target_os = "linux",
    target_os = "freebsd",
    target_os = "dragonfly",
    target_os = "openbsd",
    target_os = "netbsd"
))]
pub use single_instance::{
    MAX_SECOND_INSTANCE_ARGUMENTS, MAX_SECOND_INSTANCE_MESSAGE_BYTES,
    MAX_SINGLE_INSTANCE_IDENTIFIER_BYTES, SecondInstanceEvent, SingleInstanceError,
};
pub use tray::{
    MAX_TRAY_ENCODED_ICON_BYTES, MAX_TRAY_ICON_DIMENSION, MAX_TRAY_ICONS, MAX_TRAY_MENU_DEPTH,
    MAX_TRAY_MENU_ITEMS, MAX_TRAY_TEXT_BYTES, TrayEvent, TrayEventKind, TrayIconImage,
    TrayIconOptions, TrayMenuItem, TrayMouseButton,
};
#[cfg(any(test, feature = "test-support"))]
mod test_context;
#[cfg(any(test, feature = "test-support"))]
pub use test_context::{
    MAX_TEST_EFFECT_TURNS, TestAppContext, TestAppError, TestWindowHandle, VisualTestContext,
};

#[derive(Clone, Copy)]
struct PopoverWindowContext {
    owner: WindowHandle,
    root: WindowHandle,
}

#[derive(Clone, Copy)]
struct ClosedWindow {
    handle: WindowHandle,
    parent: Option<WindowHandle>,
    restore_focus: Option<ElementId>,
}

struct RuntimeWindow {
    parent: Option<WindowHandle>,
    restore_focus_on_close: Option<ElementId>,
    view: Box<dyn AnyView>,
    renderer: GpuRenderer,
    image_assets: ImageAssetCache,
    #[cfg(target_os = "macos")]
    native_host: Option<MacNativeHost>,
    #[cfg(target_os = "macos")]
    native_drop_host: MacNativeDropHost,
    #[cfg(target_os = "macos")]
    first_frame_guard: Option<MacFirstFrameGuard>,
    #[cfg(target_os = "windows")]
    window_menu_host: Option<windows_menu::WindowsMenuHost>,
    #[cfg(target_os = "windows")]
    taskbar_state_applied: bool,
    #[cfg(target_os = "windows")]
    taskbar_apply_attempts: u8,
    ui: UiTree,
    #[cfg(feature = "inspector")]
    inspector: Option<InspectorState>,
    scheduler: FrameScheduler,
    scene: Scene,
    metrics: MetricsTracker,
    scale_factor: f32,
    logical_size: Size,
    logical_position: Point,
    display_id: Option<DisplayId>,
    appearance: WindowAppearance,
    native_tabs: WindowTabState,
    restore_bounds: Rect,
    maximized: bool,
    pointer: Option<Point>,
    pointer_capture: Option<PointerCapture>,
    pressed_mouse_buttons: PressedMouseButtons,
    mouse_clicks: MouseClickTracker,
    mouse_event_path_scratch: Vec<ElementId>,
    mouse_dispatch_scratch: Vec<MouseListenerKey>,
    mouse_hover_changes_scratch: Vec<MouseHoverChange>,
    key_dispatch_scratch: Vec<KeyListenerBinding>,
    action_dispatch_scratch: Vec<ActionListenerBinding>,
    touch_captures: HashMap<TouchId, TouchCapture>,
    drag_candidate: Option<DragCandidate>,
    drag_session: Option<DragSession>,
    native_file_drag: Option<NativeFileDrag>,
    #[cfg(target_os = "macos")]
    native_external_drag: Option<MacNativeDropOffer>,
    #[cfg(target_os = "macos")]
    external_drag_mouse_down: Option<MacMouseDownEvent>,
    #[cfg(target_os = "macos")]
    external_drag_monitor: Option<MacExternalDragMonitor>,
    #[cfg(target_os = "macos")]
    outbound_external_drag: Option<OutboundExternalDrag>,
    #[cfg(target_os = "macos")]
    suppress_external_drag_release: bool,
    cursor: CursorIcon,
    ime_target: Option<ElementId>,
    pending_focus: Option<ElementId>,
    occluded: bool,
    focused: bool,
    visible: bool,
    relation_presented: bool,
    reduce_motion: bool,
    view_dirty: bool,
    layout_dirty: bool,
    view_deadline: Option<Instant>,
    accessibility_updates: AccessibilityUpdateSchedule,
    listeners: ListenerRegistry,
    accessibility: AccessibilityAdapter,
    // The window is last so GPU surface state is dropped before its native handle.
    window: Arc<Window>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum AccessibilityUpdateKind {
    Full,
    ScrollGeometry,
    LayoutGeometry,
}

#[derive(Default)]
struct AccessibilityUpdateSchedule {
    active: bool,
    geometry_deadline: Option<Instant>,
    pending_geometry: Option<AccessibilityUpdateKind>,
    update_due: bool,
}

impl AccessibilityUpdateSchedule {
    fn activate(&mut self) {
        self.active = true;
    }

    fn deactivate(&mut self) {
        *self = Self::default();
    }

    /// Coalesce geometry-only scroll and resize updates while keeping accessibility responsive at
    /// 10 Hz. Semantic redraws are never delayed, and an idle correction is scheduled after the
    /// last geometry frame without keeping the event loop awake in between.
    fn should_update(
        &mut self,
        retained_geometry: Option<AccessibilityUpdateKind>,
        now: Instant,
    ) -> Option<AccessibilityUpdateKind> {
        if !self.active {
            return None;
        }

        if let Some(kind) = retained_geometry {
            debug_assert!(kind != AccessibilityUpdateKind::Full);
            self.pending_geometry = Some(match (self.pending_geometry, kind) {
                (Some(AccessibilityUpdateKind::LayoutGeometry), _)
                | (_, AccessibilityUpdateKind::LayoutGeometry) => {
                    AccessibilityUpdateKind::LayoutGeometry
                }
                _ => AccessibilityUpdateKind::ScrollGeometry,
            });
            if self.pending_geometry == Some(AccessibilityUpdateKind::LayoutGeometry) {
                // A resize can produce a new full layout every display refresh. Intermediate
                // accessibility geometry is immediately obsolete, so debounce it and publish one
                // complete correction after the live resize settles. Scroll geometry remains
                // throttled below because assistive navigation benefits from progress updates.
                self.update_due = false;
                self.geometry_deadline = Some(now + ACCESSIBILITY_GEOMETRY_UPDATE_INTERVAL);
                return None;
            }
            let update = self
                .update_due
                .then(|| self.pending_geometry.take())
                .flatten();
            self.update_due = false;
            self.geometry_deadline
                .get_or_insert(now + ACCESSIBILITY_GEOMETRY_UPDATE_INTERVAL);
            return update;
        }

        if std::mem::take(&mut self.update_due) {
            // This is the correction frame requested by `advance`, not a semantic redraw. Retain
            // the pending geometry kind so a finished scroll can still use the one-node update.
            return self.pending_geometry.take();
        }

        self.geometry_deadline = None;
        self.pending_geometry = None;
        Some(AccessibilityUpdateKind::Full)
    }

    fn advance(&mut self, now: Instant) -> bool {
        if !self.active {
            return false;
        }
        if self.geometry_deadline.is_none_or(|deadline| deadline > now) {
            return false;
        }
        self.geometry_deadline = None;
        self.update_due = true;
        true
    }

    fn deadline(&self) -> Option<Instant> {
        self.geometry_deadline
    }
}

impl RuntimeWindow {
    fn any_drag_active(&self) -> bool {
        let active = self.drag_session.is_some() || self.native_file_drag.is_some();
        #[cfg(target_os = "macos")]
        {
            active || self.native_external_drag.is_some() || self.outbound_external_drag.is_some()
        }
        #[cfg(not(target_os = "macos"))]
        {
            active
        }
    }
}

struct WindowEntry {
    handle: WindowHandle,
    config: AppConfig,
    pending_input: Option<PendingInput>,
    modifiers: Modifiers,
    state: RuntimeWindow,
}

#[derive(Clone, Copy, Debug)]
struct PointerCapture {
    target: ElementId,
    button: MouseButton,
    origin: Point,
    position: Point,
}

const MAX_SIMULTANEOUS_MOUSE_BUTTONS: usize = 8;
const MOUSE_MULTI_CLICK_INTERVAL: Duration = Duration::from_millis(500);
const MOUSE_MULTI_CLICK_DISTANCE: f32 = 4.0;

#[derive(Clone, Copy, Debug)]
struct PressedMouseButtons {
    buttons: [MouseButton; MAX_SIMULTANEOUS_MOUSE_BUTTONS],
    len: u8,
}

impl Default for PressedMouseButtons {
    fn default() -> Self {
        Self {
            buttons: [MouseButton::Left; MAX_SIMULTANEOUS_MOUSE_BUTTONS],
            len: 0,
        }
    }
}

impl PressedMouseButtons {
    fn press(&mut self, button: MouseButton) {
        self.release(button);
        let len = usize::from(self.len);
        if len < self.buttons.len() {
            self.buttons[len] = button;
            self.len += 1;
        } else {
            self.buttons.rotate_left(1);
            self.buttons[MAX_SIMULTANEOUS_MOUSE_BUTTONS - 1] = button;
        }
    }

    fn release(&mut self, button: MouseButton) {
        let len = usize::from(self.len);
        let Some(index) = self.buttons[..len]
            .iter()
            .position(|pressed| *pressed == button)
        else {
            return;
        };
        self.buttons.copy_within(index + 1..len, index);
        self.len -= 1;
    }

    fn current(self) -> Option<MouseButton> {
        self.len
            .checked_sub(1)
            .map(|index| self.buttons[usize::from(index)])
    }
}

#[derive(Clone, Copy, Debug)]
struct MouseClick {
    button: MouseButton,
    position: Point,
    at: Instant,
    count: usize,
}

#[derive(Clone, Copy, Debug, Default)]
struct ActiveMouseClick {
    button: MouseButton,
    count: usize,
}

#[derive(Debug)]
struct MouseClickTracker {
    last_press: Option<MouseClick>,
    active: [ActiveMouseClick; MAX_SIMULTANEOUS_MOUSE_BUTTONS],
    active_len: u8,
}

impl Default for MouseClickTracker {
    fn default() -> Self {
        Self {
            last_press: None,
            active: [ActiveMouseClick::default(); MAX_SIMULTANEOUS_MOUSE_BUTTONS],
            active_len: 0,
        }
    }
}

impl MouseClickTracker {
    fn press(
        &mut self,
        button: MouseButton,
        position: Point,
        now: Instant,
        native_count: Option<usize>,
    ) -> usize {
        let count = native_count.unwrap_or_else(|| {
            self.last_press
                .filter(|last| {
                    let delta = position - last.position;
                    last.button == button
                        && now.saturating_duration_since(last.at) <= MOUSE_MULTI_CLICK_INTERVAL
                        && delta.x * delta.x + delta.y * delta.y
                            <= MOUSE_MULTI_CLICK_DISTANCE * MOUSE_MULTI_CLICK_DISTANCE
                })
                .map_or(1, |last| last.count.saturating_add(1))
        });
        let count = count.max(1);
        self.last_press = Some(MouseClick {
            button,
            position,
            at: now,
            count,
        });
        self.remove_active(button);
        let len = usize::from(self.active_len);
        if len < self.active.len() {
            self.active[len] = ActiveMouseClick { button, count };
            self.active_len += 1;
        }
        count
    }

    fn release(&mut self, button: MouseButton, native_count: Option<usize>) -> usize {
        let retained = self.active[..usize::from(self.active_len)]
            .iter()
            .find(|active| active.button == button)
            .map(|active| active.count);
        self.remove_active(button);
        native_count.or(retained).unwrap_or(1).max(1)
    }

    fn remove_active(&mut self, button: MouseButton) {
        let len = usize::from(self.active_len);
        let Some(index) = self.active[..len]
            .iter()
            .position(|active| active.button == button)
        else {
            return;
        };
        self.active.copy_within(index + 1..len, index);
        self.active_len -= 1;
    }

    fn cancel(&mut self) {
        self.last_press = None;
        self.active_len = 0;
    }
}

#[derive(Clone, Copy, Debug)]
struct TouchCapture {
    target: ElementId,
    last_event: TouchEvent,
}

const DRAG_THRESHOLD: f32 = 2.0;

#[derive(Clone, Copy, Debug)]
struct DragCandidate {
    source: ElementId,
    origin: Point,
}

struct DragSession {
    source: ElementId,
    position: Point,
    value: Arc<dyn Any>,
    value_type: TypeId,
    external_payload: Option<ExternalDragPayload>,
}

#[cfg(target_os = "macos")]
struct OutboundExternalDrag {
    source: ElementId,
    _session: MacExternalDragSession,
}

#[cfg(target_os = "macos")]
fn native_drop_origin(
    destination: Option<WindowHandle>,
    payload: &MacNativeDropPayload,
) -> DragOrigin {
    match payload {
        MacNativeDropPayload::Typed(payload) if destination == Some(payload.source_window()) => {
            DragOrigin::Internal(payload.source())
        }
        MacNativeDropPayload::Typed(payload) => DragOrigin::CrossWindow {
            window: payload.source_window(),
            source: payload.source(),
        },
        MacNativeDropPayload::Text(_) | MacNativeDropPayload::Url(_) => DragOrigin::External,
    }
}

#[derive(Default)]
struct NativeFileDrag {
    hovered_count: usize,
    dropped_count: usize,
    hovered_paths: Vec<PathBuf>,
    dropped_paths: Vec<PathBuf>,
    truncated: bool,
    hover_pending: bool,
    hover_value: Option<Arc<DroppedFiles>>,
}

impl NativeFileDrag {
    fn hover(&mut self, path: PathBuf) {
        self.hovered_count = self.hovered_count.saturating_add(1);
        if self.hovered_paths.len() < MAX_DROPPED_FILES {
            self.hovered_paths.push(path);
        } else {
            self.truncated = true;
        }
        self.hover_pending = true;
    }

    fn drop_path(&mut self, path: PathBuf) -> bool {
        self.dropped_count = self.dropped_count.saturating_add(1);
        if self.dropped_paths.len() < MAX_DROPPED_FILES {
            self.dropped_paths.push(path);
        } else {
            self.truncated = true;
        }
        self.hovered_count == 0 || self.dropped_count >= self.hovered_count
    }

    fn take_hovered_files(&mut self) -> Option<DroppedFiles> {
        std::mem::take(&mut self.hover_pending)
            .then(|| DroppedFiles::from_retained(self.hovered_paths.clone(), self.truncated))
    }

    fn into_dropped_files(self) -> DroppedFiles {
        DroppedFiles::from_retained(self.dropped_paths, self.truncated)
    }
}

fn enqueue_entity_events(
    pending: &mut VecDeque<EntityEvent>,
    incoming: &mut Vec<EntityEvent>,
) -> bool {
    if pending.len().saturating_add(incoming.len()) > MAX_PENDING_ENTITY_EVENTS {
        return false;
    }
    pending.extend(incoming.drain(..));
    true
}

fn reserve_entity_event_delivery(deliveries: &mut usize) -> bool {
    if *deliveries >= MAX_ENTITY_EVENT_DELIVERIES_PER_TURN {
        return false;
    }
    *deliveries += 1;
    true
}

fn reserve_global_observer_delivery(deliveries: &mut usize) -> bool {
    if *deliveries >= MAX_GLOBAL_OBSERVER_DELIVERIES_PER_TURN {
        return false;
    }
    *deliveries += 1;
    true
}

fn enqueue_global_notifications(
    pending: &mut VecDeque<TypeId>,
    pending_types: &mut HashSet<TypeId>,
    pending_all: &mut bool,
    incoming: &[TypeId],
    all: bool,
) -> bool {
    if all {
        pending.clear();
        pending_types.clear();
        *pending_all = true;
        return true;
    }
    if *pending_all {
        return true;
    }
    for &global_type in incoming {
        if !pending_types.insert(global_type) {
            continue;
        }
        if pending.len() == MAX_PENDING_GLOBAL_NOTIFICATIONS {
            pending_types.remove(&global_type);
            return false;
        }
        pending.push_back(global_type);
    }
    true
}

fn enqueue_platform_requests(
    pending: &mut VecDeque<PlatformRequest>,
    incoming: &mut Vec<PlatformRequest>,
) {
    for request in incoming.drain(..) {
        if pending.len() == crate::MAX_PENDING_PLATFORM_REQUESTS {
            request.complete_error(PlatformError::PendingQueueFull);
        } else {
            pending.push_back(request);
        }
    }
}

struct Runtime {
    pending_windows: VecDeque<WindowRequest>,
    pending_entity_events: VecDeque<EntityEvent>,
    pending_global_notifications: VecDeque<TypeId>,
    pending_global_notification_types: HashSet<TypeId>,
    pending_all_globals: bool,
    targeted_actions: VecDeque<(WindowHandle, AnyAction)>,
    windows: HashMap<WindowId, WindowEntry>,
    window_handles: HashMap<WindowHandle, WindowId>,
    window_registry_cache: RefCell<WindowRegistryCache>,
    current_window: Option<(WindowId, WindowHandle)>,
    active_window: Option<WindowId>,
    focus_history: Vec<WindowId>,
    close_requests: Vec<WindowHandle>,
    focus_requests: Vec<WindowHandle>,
    invalidate_requests: Vec<WindowHandle>,
    window_commands: Vec<WindowCommand>,
    external_menus: Option<Vec<Menu>>,
    pending_initial_open_urls: Option<OpenUrls>,
    #[cfg(any(target_os = "macos", target_os = "windows"))]
    pending_native_popup_menus: HashMap<u64, PendingNativePopupMenu>,
    platform_requests: VecDeque<PlatformRequest>,
    pending_global_shortcut_commands: VecDeque<global_shortcut::GlobalShortcutCommand>,
    pending_tray_commands: VecDeque<tray::TrayCommand>,
    global_shortcut_state: global_shortcut::GlobalShortcutState,
    #[cfg(any(target_os = "macos", target_os = "windows", target_os = "linux"))]
    global_shortcuts: HashMap<u32, global_shortcut::RegisteredGlobalShortcut>,
    tray_icons: HashMap<u32, tray::NativeTrayIcon>,
    #[cfg(any(
        target_os = "macos",
        target_os = "windows",
        target_os = "linux",
        target_os = "freebsd",
        target_os = "dragonfly",
        target_os = "openbsd",
        target_os = "netbsd"
    ))]
    single_instance: Option<single_instance::SingleInstanceGuard>,
    image_workers: ImageWorkerPoolHandle,
    background_tasks: BackgroundTaskPoolHandle,
    foreground_tasks: ForegroundTaskSpawner,
    app_info: Option<AppInfo>,
    app_paths: Option<AppPaths>,
    system_info: SystemInfo,
    system_preferences: SystemPreferences,
    globals: GlobalStore,
    assets: Assets,
    font_system: SharedFontSystem,
    gpu_contexts: HashMap<PerformanceProfile, GpuContext>,
    displays: Displays,
    keyboard: KeyboardState,
    #[cfg(target_os = "macos")]
    native_drag_registry: MacTypedDragRegistry,
    #[cfg(target_os = "macos")]
    popover_monitor: MacPopoverMonitor,
    #[cfg(any(
        target_os = "macos",
        target_os = "windows",
        target_os = "linux",
        target_os = "freebsd",
        target_os = "dragonfly",
        target_os = "openbsd",
        target_os = "netbsd"
    ))]
    active_platform_dialogs: HashMap<Option<WindowHandle>, platform_dialog::ActivePlatformDialog>,
    #[cfg(target_os = "macos")]
    automatic_tabbing_baseline: Option<bool>,
    #[cfg(target_os = "macos")]
    tabbing_window_count: usize,
    #[cfg(target_os = "macos")]
    mac_application_host: Option<MacApplicationHost>,
    #[cfg(target_os = "macos")]
    native_termination_pending: bool,
    #[cfg(target_os = "windows")]
    _windows_power_monitor: Option<power_monitor::WindowsPowerMonitor>,
    #[cfg(target_os = "linux")]
    _linux_power_monitor: Option<power_monitor::LinuxPowerMonitor>,
    application_callbacks: ApplicationCallbacks,
    quit_mode: QuitMode,
    ready: bool,
    opened_window: bool,
    exit_requested: bool,
    pending_quit: Option<QuitReason>,
    quit_phase_active: bool,
    last_window_quit_prevented: bool,
    relaunch_request: Option<RelaunchRequest>,
    process_services_finalized: bool,
    // The following four fields are the currently activated window. Event delivery is serialized
    // by Winit, so moving one entry into this slot keeps the mature single-window hot path narrow
    // while every inactive window remains independently retained in `windows`.
    config: AppConfig,
    keymap: Keymap,
    menus: Vec<Menu>,
    menu_actions: Vec<MenuAction>,
    #[cfg(target_os = "macos")]
    dock_menu: Option<Menu>,
    #[cfg(target_os = "macos")]
    dock_menu_actions: Vec<MenuAction>,
    #[cfg(target_os = "macos")]
    dock_badge: Option<Arc<str>>,
    #[cfg(target_os = "macos")]
    dock_icon: Option<Image>,
    #[cfg(target_os = "macos")]
    menu_host: Option<MacMenuHost>,
    #[cfg(target_os = "windows")]
    windows_menu_host: Option<windows_menu::WindowsMenuHost>,
    pending_input: Option<PendingInput>,
    window: Option<RuntimeWindow>,
    modifiers: Modifiers,
    fatal_error: Option<AppError>,
    event_proxy: EventLoopProxy<RuntimeEvent>,
    clipboard: ClipboardService,
    form_submission_depth: u8,
    animation_epoch: Instant,
}

struct RuntimeStartup {
    initial_window: Option<WindowRequest>,
    app_info: Option<AppInfo>,
    app_paths: Option<AppPaths>,
    globals: GlobalStore,
    keymap: Keymap,
    menus: Vec<Menu>,
    assets: Assets,
    fonts: Vec<FontSource>,
    application_callbacks: ApplicationCallbacks,
    quit_mode: QuitMode,
}

#[derive(Default)]
struct WindowRegistryCache {
    handles: Arc<[WindowHandle]>,
    truncated: bool,
}

#[cfg(any(target_os = "macos", target_os = "windows"))]
struct PendingNativePopupMenu {
    window: WindowHandle,
    actions: Vec<MenuAction>,
}

#[derive(Clone, Debug)]
struct PendingKey {
    stroke: Keystroke,
    repeat: bool,
    text: Option<String>,
}

impl PendingKey {
    fn keystroke(&self) -> Keystroke {
        self.stroke.clone()
    }
}

#[derive(Clone, Debug)]
struct PendingInput {
    keys: Vec<PendingKey>,
    focus: Option<ElementId>,
    deadline: Instant,
}

impl Runtime {
    fn new(
        startup: RuntimeStartup,
        event_proxy: EventLoopProxy<RuntimeEvent>,
    ) -> Result<Self, AppError> {
        let RuntimeStartup {
            initial_window,
            app_info,
            mut app_paths,
            globals,
            mut keymap,
            menus,
            assets,
            fonts,
            application_callbacks,
            quit_mode,
        } = startup;
        if app_paths.is_none()
            && let Some(info) = &app_info
        {
            app_paths = Some(
                info.paths()
                    .map_err(|error| AppError::Platform(error.to_string()))?,
            );
        }
        #[cfg(target_os = "windows")]
        if let Some(info) = &app_info
            && let Err(error) = windows_shell::set_current_app_id(info.identifier())
        {
            tracing::warn!(%error, "could not apply AppInfo as the Windows application identity");
        }
        let system_info = SystemInfo::current();
        let system_preferences = SystemPreferences::snapshot().unwrap_or_default();
        #[cfg(all(not(target_arch = "wasm32"), not(target_os = "macos")))]
        let pending_initial_open_urls = application_callbacks
            .open_urls
            .is_some()
            .then(deep_link::initial_open_urls)
            .flatten();
        #[cfg(any(target_arch = "wasm32", target_os = "macos"))]
        let pending_initial_open_urls = None;
        let font_system = create_shared_font_system(&assets, &fonts)?;
        let keyboard = KeyboardState::native();
        keymap.set_key_equivalents(keyboard.key_equivalents());
        validate_menus(&menus).map_err(|error| AppError::Platform(error.to_string()))?;
        let menu_actions = collect_menu_actions(&menus);
        let mut pending_windows = VecDeque::with_capacity(2);
        pending_windows.extend(initial_window);
        let image_workers = ImageWorkerPoolHandle::new(event_proxy.clone());
        let background_tasks = BackgroundTaskPoolHandle::new(event_proxy.clone());
        let foreground_tasks = ForegroundTaskSpawner::new(event_proxy.clone());
        let animation_epoch = Instant::now();
        #[cfg(target_os = "windows")]
        let windows_power_monitor = application_callbacks
            .power_event
            .is_some()
            .then(|| power_monitor::WindowsPowerMonitor::start(event_proxy.clone()))
            .transpose()
            .map_err(AppError::Platform)?;
        #[cfg(target_os = "linux")]
        let linux_power_monitor = if application_callbacks.power_event.is_some() {
            match power_monitor::LinuxPowerMonitor::start(event_proxy.clone()) {
                Ok(monitor) => Some(monitor),
                Err(error) => {
                    tracing::warn!(%error, "Linux power monitoring is unavailable");
                    None
                }
            }
        } else {
            None
        };
        #[cfg(target_os = "macos")]
        let mac_application_host = MacApplicationHost::new(
            event_proxy.clone(),
            application_callbacks.open_urls.is_some(),
            application_callbacks.reopen.is_some(),
            application_callbacks.before_quit.is_some()
                || application_callbacks.will_quit.is_some(),
            application_callbacks.system_wake.is_some()
                || application_callbacks.power_event.is_some(),
            application_callbacks.system_notification_response.is_some(),
        )
        .map_err(AppError::Platform)?;
        Ok(Self {
            pending_windows,
            pending_entity_events: VecDeque::with_capacity(8),
            pending_global_notifications: VecDeque::with_capacity(8),
            pending_global_notification_types: HashSet::with_capacity(8),
            pending_all_globals: false,
            targeted_actions: VecDeque::with_capacity(8),
            windows: HashMap::new(),
            window_handles: HashMap::new(),
            window_registry_cache: RefCell::new(WindowRegistryCache::default()),
            current_window: None,
            active_window: None,
            focus_history: Vec::new(),
            close_requests: Vec::new(),
            focus_requests: Vec::new(),
            invalidate_requests: Vec::new(),
            window_commands: Vec::with_capacity(8),
            external_menus: None,
            pending_initial_open_urls,
            #[cfg(any(target_os = "macos", target_os = "windows"))]
            pending_native_popup_menus: HashMap::new(),
            platform_requests: VecDeque::with_capacity(8),
            pending_global_shortcut_commands: VecDeque::with_capacity(4),
            pending_tray_commands: VecDeque::with_capacity(4),
            #[cfg(any(target_os = "macos", target_os = "windows", target_os = "linux"))]
            global_shortcut_state: global_shortcut::GlobalShortcutState::Pending,
            #[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
            global_shortcut_state: global_shortcut::GlobalShortcutState::Unavailable,
            #[cfg(any(target_os = "macos", target_os = "windows", target_os = "linux"))]
            global_shortcuts: HashMap::new(),
            tray_icons: HashMap::new(),
            #[cfg(any(
                target_os = "macos",
                target_os = "windows",
                target_os = "linux",
                target_os = "freebsd",
                target_os = "dragonfly",
                target_os = "openbsd",
                target_os = "netbsd"
            ))]
            single_instance: None,
            image_workers,
            background_tasks,
            foreground_tasks,
            app_info,
            app_paths,
            system_info,
            system_preferences,
            globals,
            assets,
            font_system,
            gpu_contexts: HashMap::new(),
            displays: Displays::default(),
            keyboard,
            #[cfg(target_os = "macos")]
            native_drag_registry: MacTypedDragRegistry::new(),
            #[cfg(target_os = "macos")]
            popover_monitor: MacPopoverMonitor::new(event_proxy.clone()),
            #[cfg(any(
                target_os = "macos",
                target_os = "windows",
                target_os = "linux",
                target_os = "freebsd",
                target_os = "dragonfly",
                target_os = "openbsd",
                target_os = "netbsd"
            ))]
            active_platform_dialogs: HashMap::new(),
            #[cfg(target_os = "macos")]
            automatic_tabbing_baseline: None,
            #[cfg(target_os = "macos")]
            tabbing_window_count: 0,
            #[cfg(target_os = "macos")]
            mac_application_host: Some(mac_application_host),
            #[cfg(target_os = "macos")]
            native_termination_pending: false,
            #[cfg(target_os = "windows")]
            _windows_power_monitor: windows_power_monitor,
            #[cfg(target_os = "linux")]
            _linux_power_monitor: linux_power_monitor,
            application_callbacks,
            quit_mode,
            ready: false,
            opened_window: false,
            exit_requested: false,
            pending_quit: None,
            quit_phase_active: false,
            last_window_quit_prevented: false,
            relaunch_request: None,
            process_services_finalized: false,
            config: AppConfig::default(),
            keymap,
            menus,
            menu_actions,
            #[cfg(target_os = "macos")]
            dock_menu: None,
            #[cfg(target_os = "macos")]
            dock_menu_actions: Vec::new(),
            #[cfg(target_os = "macos")]
            dock_badge: None,
            #[cfg(target_os = "macos")]
            dock_icon: None,
            #[cfg(target_os = "macos")]
            menu_host: None,
            #[cfg(target_os = "windows")]
            windows_menu_host: None,
            pending_input: None,
            window: None,
            modifiers: Modifiers::default(),
            fatal_error: None,
            event_proxy,
            clipboard: ClipboardService::system(),
            form_submission_depth: 0,
            animation_epoch,
        })
    }

    fn event_context(&self) -> EventContext {
        let parent = self.window.as_ref().and_then(|window| window.parent);
        let popover_context = self.current_popover_context();
        EventContext::with_runtime(EventRuntimeContext {
            globals: self.globals.clone(),
            foreground_tasks: self.foreground_tasks.clone(),
            clipboard: self.clipboard.clone(),
            displays: self.displays.clone(),
            keyboard_layout: self.keyboard.layout().clone(),
            assets: self.assets.clone(),
            app_info: self.app_info.clone(),
            app_paths: self.app_paths.clone(),
            system_info: self.system_info.clone(),
            system_preferences: self.system_preferences,
            window_registry: self.window_registry(),
            window: crate::event::EventWindowContext {
                window: self.current_handle(),
                parent,
                popover_owner: popover_context.map(|context| context.owner),
                popover_root: popover_context.map(|context| context.root),
                pointer_position: self.window.as_ref().and_then(|window| window.pointer),
            },
        })
    }

    fn invalidate_external(&mut self, handle: WindowHandle) -> bool {
        if self.current_handle() == Some(handle) {
            let Some(window) = self.window.as_mut() else {
                return false;
            };
            window.view_dirty = true;
            if window.visible && window.scheduler.invalidate() {
                window.window.request_redraw();
            }
            return true;
        }

        let Some(window_id) = self.window_handles.get(&handle).copied() else {
            return false;
        };
        let Some(entry) = self.windows.get_mut(&window_id) else {
            return false;
        };
        entry.state.view_dirty = true;
        if entry.state.visible && entry.state.scheduler.invalidate() {
            entry.state.window.request_redraw();
        }
        true
    }

    fn focus_external(&mut self, handle: WindowHandle, element: ElementId) -> bool {
        if self.current_handle() == Some(handle) {
            let Some(window) = self.window.as_mut() else {
                return false;
            };
            if !window.ui.is_focusable(element) {
                return false;
            }
            let changed = window.ui.focus(element);
            window.pending_focus = None;
            if changed {
                self.pending_input = None;
            }
            #[cfg(target_os = "macos")]
            if let Some(host) = &window.native_host {
                host.focus_framework();
            }
            window.view_dirty = true;
            if window.visible && window.scheduler.invalidate() {
                window.window.request_redraw();
            }
            return true;
        }

        let Some(window_id) = self.window_handles.get(&handle).copied() else {
            return false;
        };
        let Some(entry) = self.windows.get_mut(&window_id) else {
            return false;
        };
        if !entry.state.ui.is_focusable(element) {
            return false;
        }
        let changed = entry.state.ui.focus(element);
        entry.state.pending_focus = None;
        if changed {
            entry.pending_input = None;
        }
        #[cfg(target_os = "macos")]
        if let Some(host) = &entry.state.native_host {
            host.focus_framework();
        }
        entry.state.view_dirty = true;
        if entry.state.visible && entry.state.scheduler.invalidate() {
            entry.state.window.request_redraw();
        }
        true
    }

    fn current_popover_context(&self) -> Option<PopoverWindowContext> {
        if self.config.kind != WindowKind::SystemPopover {
            return None;
        }
        let mut root = self.current_handle()?;
        let mut ancestor = self.window.as_ref()?.parent?;
        loop {
            let window_id = *self.window_handles.get(&ancestor)?;
            let entry = self.windows.get(&window_id)?;
            if entry.config.kind != WindowKind::SystemPopover {
                return Some(PopoverWindowContext {
                    owner: ancestor,
                    root,
                });
            }
            root = ancestor;
            ancestor = entry.state.parent?;
        }
    }

    fn current_never_key_popover_children(&self) -> Vec<WindowHandle> {
        let Some(owner) = self.current_handle() else {
            return Vec::new();
        };
        self.windows
            .values()
            .filter_map(|entry| {
                (entry.state.visible
                    && entry.state.parent == Some(owner)
                    && window_is_never_key_popover(&entry.config))
                .then_some(entry.handle)
            })
            .collect()
    }

    #[cfg(target_os = "macos")]
    fn popovers_to_close_after_application_deactivation(&self) -> Vec<WindowHandle> {
        let candidates = self
            .windows
            .values()
            .filter_map(|entry| {
                (entry.state.visible
                    && window_dismisses_system_popover_on_pointer_outside(&entry.config))
                .then_some(entry.handle)
            })
            .collect::<Vec<_>>();

        // One close request tears down its whole child tree. Keep only the highest dismissible
        // ancestor so nested system popovers cannot produce duplicate close callbacks.
        let mut roots = candidates
            .iter()
            .copied()
            .filter(|handle| {
                !candidates.iter().copied().any(|ancestor| {
                    ancestor != *handle && self.window_is_ancestor(ancestor, *handle)
                })
            })
            .collect::<Vec<_>>();
        roots.sort_unstable();
        roots
    }

    fn fail(&mut self, event_loop: &ActiveEventLoop, error: AppError) {
        tracing::error!(%error, "QuickGUI is exiting after a fatal error");
        self.fatal_error = Some(error);
        event_loop.exit();
    }

    fn activate_window(&mut self, window_id: WindowId) -> bool {
        debug_assert!(self.window.is_none());
        debug_assert!(self.current_window.is_none());
        let Some(entry) = self.windows.remove(&window_id) else {
            return false;
        };
        self.current_window = Some((window_id, entry.handle));
        self.config = entry.config;
        self.pending_input = entry.pending_input;
        self.modifiers = entry.modifiers;
        self.window = Some(entry.state);
        true
    }

    fn deactivate_window(&mut self) {
        let Some((window_id, handle)) = self.current_window.take() else {
            return;
        };
        let state = self
            .window
            .take()
            .expect("an activated window always owns runtime state");
        let entry = WindowEntry {
            handle,
            config: std::mem::take(&mut self.config),
            pending_input: self.pending_input.take(),
            modifiers: std::mem::take(&mut self.modifiers),
            state,
        };
        let previous = self.windows.insert(window_id, entry);
        debug_assert!(previous.is_none());
    }

    fn current_handle(&self) -> Option<WindowHandle> {
        self.current_window.map(|(_, handle)| handle)
    }

    fn active_window_handle(&self) -> Option<WindowHandle> {
        let active = self.active_window?;
        if self.current_window.is_some_and(|(id, _)| id == active) {
            return self.current_handle();
        }
        self.windows.get(&active).map(|entry| entry.handle)
    }

    fn window_registry(&self) -> WindowRegistry {
        let total = self.window_handles.len() + self.pending_windows.len();
        let mut cache = self.window_registry_cache.borrow_mut();
        let cache_matches = total <= MAX_APPLICATION_WINDOWS
            && cache.handles.len() == total
            && self
                .window_handles
                .keys()
                .all(|handle| cache.handles.binary_search(handle).is_ok())
            && self
                .pending_windows
                .iter()
                .all(|request| cache.handles.binary_search(&request.handle).is_ok());
        if !cache_matches {
            let mut handles = Vec::with_capacity(total.min(MAX_APPLICATION_WINDOWS));
            handles.extend(
                self.window_handles
                    .keys()
                    .copied()
                    .take(MAX_APPLICATION_WINDOWS),
            );
            handles.extend(
                self.pending_windows
                    .iter()
                    .take(MAX_APPLICATION_WINDOWS.saturating_sub(handles.len()))
                    .map(|request| request.handle),
            );
            handles.sort_unstable();
            handles.dedup();
            handles.truncate(MAX_APPLICATION_WINDOWS);
            cache.handles = handles.into();
            cache.truncated = total > MAX_APPLICATION_WINDOWS;
        } else {
            cache.truncated = false;
        }
        let active_window = self.active_window_handle();
        if let Some(active) = active_window
            && cache.handles.binary_search(&active).is_err()
        {
            cache.truncated = true;
            let mut handles = cache.handles.to_vec();
            if let Some(last) = handles.last_mut() {
                *last = active;
                handles.sort_unstable();
                cache.handles = handles.into();
            }
        }
        WindowRegistry::new(cache.handles.clone(), active_window, cache.truncated)
    }

    fn window_state_for(&self, handle: WindowHandle) -> Option<WindowState> {
        if self.current_handle() == Some(handle) {
            return self.current_window_state();
        }
        let window_id = self.window_handles.get(&handle)?;
        let entry = self.windows.get(window_id)?;
        Some(runtime_window_state(handle, &entry.config, &entry.state))
    }

    fn current_window_state(&self) -> Option<WindowState> {
        let handle = self.current_handle()?;
        let state = self.window.as_ref()?;
        Some(runtime_window_state(handle, &self.config, state))
    }

    #[cfg(target_os = "macos")]
    fn register_native_tabbing(&mut self, event_loop: &ActiveEventLoop) {
        if self.tabbing_window_count == 0 {
            let baseline = event_loop.allows_automatic_window_tabbing();
            self.automatic_tabbing_baseline = Some(baseline);
            if !baseline {
                event_loop.set_allows_automatic_window_tabbing(true);
            }
        }
        self.tabbing_window_count = self.tabbing_window_count.saturating_add(1);
    }

    #[cfg(target_os = "macos")]
    fn unregister_native_tabbing(&mut self, event_loop: &ActiveEventLoop) {
        self.tabbing_window_count = self.tabbing_window_count.saturating_sub(1);
        if self.tabbing_window_count == 0
            && let Some(baseline) = self.automatic_tabbing_baseline.take()
        {
            event_loop.set_allows_automatic_window_tabbing(baseline);
        }
    }

    #[cfg(target_os = "macos")]
    fn restore_native_tabbing_baseline(&mut self, event_loop: &ActiveEventLoop) {
        self.tabbing_window_count = 0;
        if let Some(baseline) = self.automatic_tabbing_baseline.take() {
            event_loop.set_allows_automatic_window_tabbing(baseline);
        }
    }

    #[cfg(target_os = "macos")]
    fn refresh_native_tab_states(&mut self) {
        for entry in self.windows.values_mut() {
            if entry.config.tabbing_identifier.is_none()
                && entry.state.native_tabs == WindowTabState::default()
            {
                continue;
            }
            let Ok(tabs) = window_tab_state(&entry.state.window) else {
                continue;
            };
            if entry.state.native_tabs != tabs {
                entry.state.native_tabs = tabs;
                if entry.state.listeners.observes_window_state {
                    entry.state.view_dirty = true;
                    if entry.state.visible && entry.state.scheduler.invalidate() {
                        entry.state.window.request_redraw();
                    }
                }
            }
        }
    }

    #[cfg(target_os = "macos")]
    fn refresh_current_native_tab_state(&mut self) {
        if self.config.tabbing_identifier.is_none()
            && self
                .window
                .as_ref()
                .is_none_or(|state| state.native_tabs == WindowTabState::default())
        {
            return;
        }
        let Some(state) = self.window.as_mut() else {
            return;
        };
        let Ok(tabs) = window_tab_state(&state.window) else {
            return;
        };
        if state.native_tabs != tabs {
            state.native_tabs = tabs;
            if state.listeners.observes_window_state {
                state.view_dirty = true;
                if state.visible && state.scheduler.invalidate() {
                    state.window.request_redraw();
                }
            }
        }
    }

    /// Refresh the bounded monitor snapshot only at a native lifecycle boundary.
    ///
    /// This is intentionally absent from `about_to_wait`: unchanged applications retain no
    /// monitor polling cost and no display-owned native handles in public state.
    fn refresh_displays(&mut self, event_loop: &ActiveEventLoop) -> bool {
        let displays = crate::display::native_displays(event_loop);
        let snapshot_changed = self.displays != displays;
        self.displays = displays;

        let refresh_window = |state: &mut RuntimeWindow| {
            let previous = state.display_id;
            state.display_id = runtime_window_display_id(state, &self.displays);
            let display_changed = previous != state.display_id;
            if snapshot_changed && state.listeners.observes_displays
                || display_changed && state.listeners.observes_window_state
            {
                state.view_dirty = true;
                if state.visible && state.scheduler.invalidate() {
                    state.window.request_redraw();
                }
            }
        };

        for entry in self.windows.values_mut() {
            refresh_window(&mut entry.state);
        }
        if let Some(state) = &mut self.window {
            refresh_window(state);
        }
        snapshot_changed
    }

    #[cfg(target_os = "macos")]
    fn refresh_keyboard_layout(&mut self) -> bool {
        let keyboard = KeyboardState::native();
        if self.keyboard == keyboard {
            return false;
        }
        self.keyboard = keyboard;
        self.keymap
            .set_key_equivalents(self.keyboard.key_equivalents());

        // A prefix cannot safely span two command layouts. Key releases remain independently
        // translated by their native event, while future presses use the new immutable table.
        self.pending_input = None;
        for entry in self.windows.values_mut() {
            entry.pending_input = None;
            if entry.state.listeners.observes_keyboard_layout {
                entry.state.view_dirty = true;
                if entry.state.visible && entry.state.scheduler.invalidate() {
                    entry.state.window.request_redraw();
                }
            }
        }
        if let Some(state) = &mut self.window
            && state.listeners.observes_keyboard_layout
        {
            state.view_dirty = true;
            if state.visible && state.scheduler.invalidate() {
                state.window.request_redraw();
            }
        }
        true
    }

    fn refresh_system_preferences(&mut self, preferences: SystemPreferences) -> bool {
        if self.system_preferences == preferences {
            return false;
        }
        self.system_preferences = preferences;
        let now = Instant::now();

        let refresh_window =
            |state: &mut RuntimeWindow, config: &AppConfig, preferences: SystemPreferences| {
                let reduce_motion = config.reduce_motion
                    || preferences.reduce_motion().is_some_and(|enabled| enabled);
                if state.reduce_motion != reduce_motion {
                    state.reduce_motion = reduce_motion;
                    state.ui.set_reduce_motion(reduce_motion);
                    state
                        .ui
                        .set_animations_enabled(!state.occluded && !reduce_motion, now);
                }
                if state.listeners.observes_system_preferences {
                    state.view_dirty = true;
                    if state.visible && state.scheduler.invalidate() {
                        state.window.request_redraw();
                    }
                }
            };

        for entry in self.windows.values_mut() {
            refresh_window(&mut entry.state, &entry.config, preferences);
        }
        if let Some(state) = &mut self.window {
            refresh_window(state, &self.config, preferences);
        }
        true
    }

    fn note_window_focused(&mut self, window_id: WindowId) {
        self.focus_history
            .retain(|candidate| *candidate != window_id);
        self.focus_history.push(window_id);
        self.active_window = Some(window_id);
    }

    fn process_entity_events(
        &mut self,
        event_loop: &ActiveEventLoop,
        deliveries: &mut usize,
    ) -> bool {
        debug_assert!(self.current_window.is_none());
        debug_assert!(self.window.is_none());

        while let Some(event) = self.pending_entity_events.pop_front() {
            let mut targets = self
                .windows
                .iter()
                .filter_map(|(window_id, entry)| {
                    entry
                        .state
                        .listeners
                        .has_entity_event_subscribers(event.source, event.event_type)
                        .then_some((entry.handle, *window_id))
                })
                .collect::<Vec<_>>();
            targets.sort_unstable_by_key(|(handle, _)| *handle);

            for (_, window_id) in targets {
                if !self.activate_window(window_id) {
                    continue;
                }
                let callbacks = self
                    .window
                    .as_ref()
                    .map(|window| {
                        window
                            .listeners
                            .entity_event_callbacks(event.source, event.event_type)
                    })
                    .unwrap_or_default();

                for callback in callbacks {
                    if !reserve_entity_event_delivery(deliveries) {
                        self.deactivate_window();
                        self.fail(
                            event_loop,
                            AppError::View(format!(
                                "one effect cycle exceeded {MAX_ENTITY_EVENT_DELIVERIES_PER_TURN} entity-event callback deliveries"
                            )),
                        );
                        return false;
                    }
                    let mut cx = self.event_context();
                    if let Some(window) = &mut self.window {
                        callback.borrow_mut()(
                            window.view.as_any_mut(),
                            event.value.as_ref(),
                            &mut cx,
                        );
                    }
                    if !self.apply_event_context(event_loop, cx, false, true) {
                        self.deactivate_window();
                        return false;
                    }
                }
                self.deactivate_window();
            }
        }
        true
    }

    fn process_global_notifications(
        &mut self,
        event_loop: &ActiveEventLoop,
        deliveries: &mut usize,
    ) -> bool {
        debug_assert!(self.current_window.is_none());
        debug_assert!(self.window.is_none());

        loop {
            let global_type = if self.pending_all_globals {
                self.pending_all_globals = false;
                self.pending_global_notifications.clear();
                self.pending_global_notification_types.clear();
                None
            } else if let Some(global_type) = self.pending_global_notifications.pop_front() {
                self.pending_global_notification_types.remove(&global_type);
                Some(global_type)
            } else {
                break;
            };

            let mut targets = self
                .windows
                .iter()
                .filter_map(|(window_id, entry)| {
                    entry
                        .state
                        .listeners
                        .has_global_subscribers(global_type)
                        .then_some((entry.handle, *window_id))
                })
                .collect::<Vec<_>>();
            targets.sort_unstable_by_key(|(handle, _)| *handle);

            for (_, window_id) in targets {
                if !self.activate_window(window_id) {
                    continue;
                }
                let subscriptions = self
                    .window
                    .as_ref()
                    .map(|window| window.listeners.global_subscriptions(global_type))
                    .unwrap_or_default();
                for subscription in subscriptions {
                    // A preceding callback may have dropped this subscription from the same view.
                    if !subscription.is_active() {
                        continue;
                    }
                    if !reserve_global_observer_delivery(deliveries) {
                        self.deactivate_window();
                        self.fail(
                            event_loop,
                            AppError::View(format!(
                                "one effect cycle exceeded {MAX_GLOBAL_OBSERVER_DELIVERIES_PER_TURN} global observer callback deliveries"
                            )),
                        );
                        return false;
                    }
                    let mut cx = self.event_context();
                    if let Some(window) = &mut self.window {
                        subscription.callback.borrow_mut()(window.view.as_any_mut(), &mut cx);
                    }
                    if !self.apply_event_context(event_loop, cx, false, true) {
                        self.deactivate_window();
                        return false;
                    }
                }
                self.deactivate_window();
            }
        }
        true
    }

    fn process_deferred_effects(&mut self, event_loop: &ActiveEventLoop) -> bool {
        let mut global_deliveries = 0_usize;
        let mut entity_deliveries = 0_usize;
        loop {
            if !self.process_global_notifications(event_loop, &mut global_deliveries)
                || !self.process_entity_events(event_loop, &mut entity_deliveries)
            {
                return false;
            }
            if !self.pending_all_globals
                && self.pending_global_notifications.is_empty()
                && self.pending_entity_events.is_empty()
            {
                return true;
            }
        }
    }

    fn process_queued_window_commands(&mut self, event_loop: &ActiveEventLoop) {
        #[cfg(target_os = "macos")]
        let mut refresh_native_tabs = false;
        for command in std::mem::take(&mut self.window_commands) {
            let handle = command.handle();
            let Some(window_id) = self.window_handles.get(&handle).copied() else {
                continue;
            };
            #[cfg(target_os = "macos")]
            let parent_window = self.windows.get(&window_id).and_then(|entry| {
                entry
                    .state
                    .parent
                    .and_then(|parent| self.window_handles.get(&parent).copied())
                    .and_then(|parent_id| self.windows.get(&parent_id))
                    .map(|parent| parent.state.window.clone())
            });
            let Some(entry) = self.windows.get_mut(&window_id) else {
                continue;
            };
            let state = &mut entry.state;
            let mut state_changed = false;
            let mut force_redraw = false;
            #[cfg(target_os = "macos")]
            let mut tabbing_ownership_delta = 0_i8;
            #[cfg(feature = "inspector")]
            let mut inspector_redraw = false;

            match command {
                WindowCommand::SetTitle(_, title) => {
                    if entry.config.title != title {
                        entry.config.title = title;
                        state.window.set_title(&entry.config.title);
                        force_redraw = true;
                    }
                }
                WindowCommand::SetRepresentedFile(_, represented_file) => {
                    if entry.config.represented_file != represented_file {
                        #[cfg(target_os = "macos")]
                        let applied = set_window_represented_file(
                            &state.window,
                            represented_file.as_deref(),
                        )
                        .map_err(|error| {
                            tracing::warn!(%error, "could not change represented document file");
                        })
                        .is_ok();
                        #[cfg(not(target_os = "macos"))]
                        let applied = true;
                        if applied {
                            entry.config.represented_file = represented_file;
                            #[cfg(target_os = "macos")]
                            if let Some(position) = entry.config.traffic_light_position
                                && let Err(error) = position_traffic_lights(&state.window, position)
                            {
                                tracing::warn!(%error, "could not restore traffic lights after changing represented document file");
                            }
                            state_changed = true;
                        }
                    }
                }
                WindowCommand::SetDocumentEdited(_, edited) => {
                    if entry.config.document_edited != edited {
                        #[cfg(target_os = "macos")]
                        let applied = set_window_document_edited(&state.window, edited)
                            .map_err(|error| {
                                tracing::warn!(%error, "could not change native document edited state");
                            })
                            .is_ok();
                        #[cfg(not(target_os = "macos"))]
                        let applied = true;
                        if applied {
                            entry.config.document_edited = edited;
                            #[cfg(target_os = "macos")]
                            if let Some(position) = entry.config.traffic_light_position
                                && let Err(error) = position_traffic_lights(&state.window, position)
                            {
                                tracing::warn!(%error, "could not restore traffic lights after changing document edited state");
                            }
                            state_changed = true;
                        }
                    }
                }
                WindowCommand::ShowCharacterPalette(_) => {
                    #[cfg(target_os = "macos")]
                    if let Err(error) = show_character_palette(&state.window) {
                        tracing::warn!(%error, "could not present the native character palette");
                    }
                }
                WindowCommand::SetTabbingIdentifier(_, identifier) => {
                    if entry.config.tabbing_identifier != identifier {
                        #[cfg(target_os = "macos")]
                        let applied = set_window_tabbing_identifier(
                            &state.window,
                            identifier.as_deref(),
                        )
                        .map_err(|error| {
                            tracing::warn!(%error, "could not change native tabbing identifier");
                        })
                        .is_ok();
                        #[cfg(not(target_os = "macos"))]
                        let applied = true;
                        if applied {
                            #[cfg(target_os = "macos")]
                            {
                                tabbing_ownership_delta = match (
                                    entry.config.tabbing_identifier.is_some(),
                                    identifier.is_some(),
                                ) {
                                    (false, true) => 1,
                                    (true, false) => -1,
                                    _ => 0,
                                };
                                refresh_native_tabs = true;
                            }
                            entry.config.tabbing_identifier = identifier;
                            state_changed = true;
                        }
                    }
                }
                WindowCommand::SelectNextTab(_) => {
                    #[cfg(target_os = "macos")]
                    {
                        if let Err(error) =
                            perform_window_tab_action(&state.window, MacWindowTabAction::SelectNext)
                        {
                            tracing::warn!(%error, "could not select the next native window tab");
                        }
                        refresh_native_tabs = true;
                    }
                }
                WindowCommand::SelectPreviousTab(_) => {
                    #[cfg(target_os = "macos")]
                    {
                        if let Err(error) = perform_window_tab_action(
                            &state.window,
                            MacWindowTabAction::SelectPrevious,
                        ) {
                            tracing::warn!(%error, "could not select the previous native window tab");
                        }
                        refresh_native_tabs = true;
                    }
                }
                WindowCommand::SelectTab(_, index) => {
                    #[cfg(target_os = "macos")]
                    {
                        if let Err(error) = perform_window_tab_action(
                            &state.window,
                            MacWindowTabAction::Select(index),
                        ) {
                            tracing::warn!(%error, "could not select a native window tab");
                        }
                        refresh_native_tabs = true;
                    }
                    #[cfg(not(target_os = "macos"))]
                    let _ = index;
                }
                WindowCommand::MergeAllWindows(_) => {
                    #[cfg(target_os = "macos")]
                    {
                        if let Err(error) =
                            perform_window_tab_action(&state.window, MacWindowTabAction::MergeAll)
                        {
                            tracing::warn!(%error, "could not merge native windows into tabs");
                        }
                        refresh_native_tabs = true;
                    }
                }
                WindowCommand::MoveTabToNewWindow(_) => {
                    #[cfg(target_os = "macos")]
                    {
                        if let Err(error) = perform_window_tab_action(
                            &state.window,
                            MacWindowTabAction::MoveToNewWindow,
                        ) {
                            tracing::warn!(%error, "could not move native tab to a new window");
                        }
                        refresh_native_tabs = true;
                    }
                }
                WindowCommand::ToggleTabBar(_) => {
                    #[cfg(target_os = "macos")]
                    {
                        if let Err(error) =
                            perform_window_tab_action(&state.window, MacWindowTabAction::ToggleBar)
                        {
                            tracing::warn!(%error, "could not toggle the native window tab bar");
                        }
                        refresh_native_tabs = true;
                    }
                }
                WindowCommand::ToggleTabOverview(_) => {
                    #[cfg(target_os = "macos")]
                    {
                        if let Err(error) = perform_window_tab_action(
                            &state.window,
                            MacWindowTabAction::ToggleOverview,
                        ) {
                            tracing::warn!(%error, "could not toggle the native window tab overview");
                        }
                        refresh_native_tabs = true;
                    }
                }
                WindowCommand::SetBounds(_, bounds) => {
                    apply_window_bounds(state, bounds);
                    state_changed = true;
                    force_redraw = true;
                }
                WindowCommand::Move(_, position) => {
                    let bounds = Rect::new(
                        position.x,
                        position.y,
                        state.restore_bounds.width,
                        state.restore_bounds.height,
                    );
                    apply_window_bounds(state, WindowBounds::Windowed(bounds));
                    state_changed = true;
                }
                WindowCommand::Resize(_, size) => {
                    apply_window_size(state, size);
                    state_changed = true;
                    force_redraw = true;
                }
                WindowCommand::Minimize(_) => {
                    if entry.config.is_minimizable && state.window.is_minimized() != Some(true) {
                        state.window.set_minimized(true);
                        state_changed = true;
                    }
                }
                WindowCommand::Restore(_) => {
                    state.window.set_minimized(false);
                    apply_window_bounds(state, WindowBounds::Windowed(state.restore_bounds));
                    state_changed = true;
                    force_redraw = true;
                }
                WindowCommand::Zoom(_) => {
                    state.maximized = runtime_window_is_maximized(state, &entry.config);
                    if state.maximized {
                        apply_window_bounds(state, WindowBounds::Windowed(state.restore_bounds));
                        state_changed = true;
                        force_redraw = true;
                    } else if !runtime_window_is_fullscreen(state)
                        && entry.config.is_resizable
                        && entry.config.is_maximizable
                    {
                        let bounds = Rect::new(
                            state.logical_position.x,
                            state.logical_position.y,
                            state.logical_size.width,
                            state.logical_size.height,
                        );
                        apply_window_bounds(state, WindowBounds::Maximized(bounds));
                        state_changed = true;
                        force_redraw = true;
                    }
                }
                WindowCommand::ToggleFullscreen(_) => {
                    // Winit retains the requested state while AppKit animates between spaces.
                    // Flipping that value lets a second toggle reverse an in-flight transition
                    // instead of mistaking the still-old native style mask for the target state.
                    let fullscreen = state.window.fullscreen().is_none();
                    state_changed = set_runtime_window_fullscreen(state, fullscreen);
                    force_redraw = state_changed;
                }
                WindowCommand::SetFullscreen(_, fullscreen) => {
                    state_changed = set_runtime_window_fullscreen(state, fullscreen);
                    force_redraw = state_changed;
                }
                WindowCommand::SetVisible(_, visible) => {
                    if state.visible != visible {
                        #[cfg(target_os = "macos")]
                        if !visible
                            && window_dismisses_system_popover_on_pointer_outside(&entry.config)
                        {
                            self.popover_monitor.unwatch(handle);
                        }
                        #[cfg(target_os = "macos")]
                        if state.relation_presented && !visible {
                            if let Err(error) =
                                dismiss_window_relation(&state.window, entry.config.kind)
                            {
                                tracing::warn!(%error, "could not dismiss native window relation");
                            }
                            state.relation_presented = false;
                        }
                        #[cfg(target_os = "macos")]
                        if visible && !state.relation_presented {
                            if let Some(popover) = entry.config.popover.as_ref()
                                && let Some(parent) = parent_window.as_ref()
                                && let Err(error) =
                                    position_system_popover(&state.window, parent, popover)
                            {
                                tracing::warn!(%error, "could not restore system popover placement");
                            }
                            match present_window_relation(
                                &state.window,
                                parent_window.as_ref(),
                                entry.config.kind,
                            ) {
                                Ok(presented) => state.relation_presented = presented,
                                Err(error) => {
                                    tracing::warn!(%error, "could not present native window relation");
                                }
                            }
                        }
                        #[cfg(target_os = "macos")]
                        if visible
                            && window_dismisses_system_popover_on_pointer_outside(&entry.config)
                            && let Some(popover) = entry.config.popover.as_ref()
                            && let Some(parent) = parent_window.as_ref()
                            && let Err(error) = self.popover_monitor.watch(
                                handle,
                                &state.window,
                                parent,
                                popover.anchor_rect,
                            )
                        {
                            tracing::warn!(%error, "could not restore native popover grab");
                        }
                        #[cfg(target_os = "macos")]
                        if let Err(error) = set_window_visibility(
                            &state.window,
                            visible,
                            entry.config.focus && entry.config.focusable,
                        ) {
                            tracing::warn!(%error, "could not change native window visibility");
                        }
                        #[cfg(target_os = "macos")]
                        if visible && window_presentation_activates_application(&entry.config) {
                            state.window.focus_window();
                        }
                        #[cfg(not(target_os = "macos"))]
                        state.window.set_visible(visible);
                        state.visible = visible;
                        if visible {
                            #[cfg(target_os = "windows")]
                            {
                                // Explorer creates the taskbar button asynchronously after the
                                // HWND becomes visible. The first redraw retries both retained
                                // taskbar properties at that native boundary.
                                state.taskbar_state_applied = false;
                                state.taskbar_apply_attempts = 0;
                            }
                            #[cfg(not(target_os = "macos"))]
                            if entry.config.focus && entry.config.focusable {
                                state.window.focus_window();
                            }
                            force_redraw = true;
                        }
                        state_changed = true;
                    }
                }
                WindowCommand::SetMovable(_, movable) => {
                    if entry.config.is_movable != movable {
                        entry.config.is_movable = movable;
                        #[cfg(target_os = "macos")]
                        if let Err(error) = set_window_movable(
                            &state.window,
                            implicit_native_movable(&entry.config),
                        ) {
                            tracing::warn!(%error, "could not change native window movability");
                        }
                        state_changed = true;
                    }
                }
                WindowCommand::SetResizable(_, resizable) => {
                    if entry.config.is_resizable != resizable {
                        entry.config.is_resizable = resizable;
                        state.window.set_resizable(resizable);
                        state
                            .window
                            .set_enabled_buttons(window_buttons(&entry.config));
                        state_changed = true;
                    }
                }
                WindowCommand::SetMinimumSize(_, minimum) => {
                    if entry.config.minimum_size != minimum {
                        entry.config.minimum_size = minimum;
                        state.window.set_min_inner_size(minimum.map(|minimum| {
                            LogicalSize::new(minimum.width as f64, minimum.height as f64)
                        }));
                        if let Some(minimum) = minimum {
                            state.restore_bounds.width =
                                state.restore_bounds.width.max(minimum.width);
                            state.restore_bounds.height =
                                state.restore_bounds.height.max(minimum.height);
                            let constrained = constrained_window_size(
                                state.logical_size,
                                Some(minimum),
                                entry.config.maximum_size,
                            );
                            if constrained != state.logical_size
                                && !runtime_window_is_fullscreen(state)
                                && !runtime_window_is_maximized(state, &entry.config)
                            {
                                if let Some(physical) =
                                    state.window.request_inner_size(LogicalSize::new(
                                        constrained.width as f64,
                                        constrained.height as f64,
                                    ))
                                {
                                    state.renderer.resize(physical.width, physical.height);
                                    state.logical_size =
                                        logical_window_size(physical, state.scale_factor);
                                }
                                state.layout_dirty = true;
                                state.view_dirty |= state.listeners.observes_viewport;
                                force_redraw = true;
                            }
                        }
                        state_changed = true;
                    }
                }
                WindowCommand::SetMaximumSize(_, maximum) => {
                    let compatible = match (entry.config.minimum_size, maximum) {
                        (Some(minimum), Some(maximum)) => {
                            minimum.width <= maximum.width && minimum.height <= maximum.height
                        }
                        _ => true,
                    };
                    if !compatible {
                        tracing::warn!(
                            "ignored a maximum window size smaller than the current minimum size"
                        );
                    } else if entry.config.maximum_size != maximum {
                        entry.config.maximum_size = maximum;
                        state.window.set_max_inner_size(maximum.map(|maximum| {
                            LogicalSize::new(maximum.width as f64, maximum.height as f64)
                        }));
                        if let Some(maximum) = maximum {
                            state.restore_bounds.width =
                                state.restore_bounds.width.min(maximum.width);
                            state.restore_bounds.height =
                                state.restore_bounds.height.min(maximum.height);
                            let constrained = constrained_window_size(
                                state.logical_size,
                                entry.config.minimum_size,
                                Some(maximum),
                            );
                            if constrained != state.logical_size
                                && !runtime_window_is_fullscreen(state)
                                && !runtime_window_is_maximized(state, &entry.config)
                            {
                                if let Some(physical) =
                                    state.window.request_inner_size(LogicalSize::new(
                                        constrained.width as f64,
                                        constrained.height as f64,
                                    ))
                                {
                                    state.renderer.resize(physical.width, physical.height);
                                    state.logical_size =
                                        logical_window_size(physical, state.scale_factor);
                                }
                                state.layout_dirty = true;
                                state.view_dirty |= state.listeners.observes_viewport;
                                force_redraw = true;
                            }
                        }
                        state_changed = true;
                    }
                }
                WindowCommand::SetMinimizable(_, minimizable) => {
                    if entry.config.is_minimizable != minimizable {
                        entry.config.is_minimizable = minimizable;
                        state
                            .window
                            .set_enabled_buttons(window_buttons(&entry.config));
                        state_changed = true;
                    }
                }
                WindowCommand::SetMaximizable(_, maximizable) => {
                    if entry.config.is_maximizable != maximizable {
                        entry.config.is_maximizable = maximizable;
                        state
                            .window
                            .set_enabled_buttons(window_buttons(&entry.config));
                        state_changed = true;
                    }
                }
                WindowCommand::SetClosable(_, closable) => {
                    if entry.config.is_closable != closable {
                        entry.config.is_closable = closable;
                        state
                            .window
                            .set_enabled_buttons(window_buttons(&entry.config));
                        state_changed = true;
                    }
                }
                WindowCommand::SetDecorated(_, decorated) => {
                    if entry.config.decorated != decorated {
                        entry.config.decorated = decorated;
                        state.window.set_decorations(decorated);
                        state_changed = true;
                        force_redraw = true;
                    }
                }
                WindowCommand::SetShadow(_, shadow) => {
                    if entry.config.shadow != shadow {
                        entry.config.shadow = shadow;
                        #[cfg(target_os = "macos")]
                        state.window.set_has_shadow(shadow);
                        state_changed = true;
                    }
                }
                WindowCommand::SetContentProtected(_, protected) => {
                    if entry.config.content_protected != protected {
                        entry.config.content_protected = protected;
                        state.window.set_content_protected(protected);
                        state_changed = true;
                    }
                }
                WindowCommand::SetWindowLevel(_, level) => {
                    if entry.config.window_level != level {
                        entry.config.window_level = level;
                        state
                            .window
                            .set_window_level(effective_window_level(&entry.config).to_winit());
                        state_changed = true;
                    }
                }
                WindowCommand::SetFocusable(_, focusable) => {
                    if entry.config.focusable != focusable {
                        entry.config.focusable = focusable;
                        if !focusable {
                            entry.config.focus = false;
                        }
                        if let Err(error) = set_window_focusable(&state.window, focusable) {
                            tracing::warn!(%error, "could not change native window focusability");
                        }
                        state_changed = true;
                    }
                }
                WindowCommand::SetSkipTaskbar(_, skip) => {
                    if entry.config.skip_taskbar != skip {
                        entry.config.skip_taskbar = skip;
                        #[cfg(target_os = "windows")]
                        state.window.set_skip_taskbar(skip);
                        state_changed = true;
                    }
                }
                WindowCommand::SetVisibleOnAllWorkspaces(_, visible) => {
                    if entry.config.visible_on_all_workspaces != visible {
                        entry.config.visible_on_all_workspaces = visible;
                        if let Err(error) = set_window_visible_on_all_workspaces(
                            &state.window,
                            effective_visible_on_all_workspaces(&entry.config),
                        ) {
                            tracing::warn!(%error, "could not change native workspace visibility");
                        }
                        state_changed = true;
                    }
                }
                WindowCommand::SetOpacity(_, opacity) => {
                    if entry.config.opacity != opacity {
                        entry.config.opacity = opacity;
                        if let Err(error) = set_window_opacity(&state.window, opacity) {
                            tracing::warn!(%error, "could not change native window opacity");
                        }
                        state_changed = true;
                    }
                }
                WindowCommand::SetIcon(_, icon) => {
                    if entry.config.icon.as_ref().map(Image::id) != icon.as_ref().map(Image::id) {
                        state
                            .window
                            .set_window_icon(icon.as_ref().map(winit_window_icon));
                        entry.config.icon = icon;
                        state_changed = true;
                    }
                }
                WindowCommand::SetTaskbarProgress(_, progress_state, progress) => {
                    if entry.config.taskbar_progress_state != progress_state
                        || entry.config.taskbar_progress != progress
                    {
                        #[cfg(target_os = "windows")]
                        {
                            state.taskbar_state_applied &= windows_window::set_taskbar_progress(
                                &state.window,
                                progress_state,
                                progress,
                            )
                            .map_err(|error| {
                                tracing::warn!(%error, "could not change native taskbar progress");
                            })
                            .is_ok();
                            state.taskbar_apply_attempts = 0;
                        }
                        entry.config.taskbar_progress_state = progress_state;
                        entry.config.taskbar_progress = progress;
                        state_changed = true;
                        #[cfg(target_os = "windows")]
                        {
                            force_redraw = true;
                        }
                    }
                }
                WindowCommand::SetTaskbarOverlayIcon(_, icon, description) => {
                    if entry.config.taskbar_overlay_icon.as_ref().map(Image::id)
                        != icon.as_ref().map(Image::id)
                        || entry.config.taskbar_overlay_description != description
                    {
                        #[cfg(target_os = "windows")]
                        {
                            state.taskbar_state_applied &=
                                windows_window::set_taskbar_overlay_icon(
                                    &state.window,
                                    icon.as_ref(),
                                    description.as_deref(),
                                )
                                .map_err(|error| {
                                    tracing::warn!(%error, "could not change native taskbar overlay icon");
                                })
                                .is_ok();
                            state.taskbar_apply_attempts = 0;
                        }
                        entry.config.taskbar_overlay_icon = icon;
                        entry.config.taskbar_overlay_description = description;
                        state_changed = true;
                        #[cfg(target_os = "windows")]
                        {
                            force_redraw = true;
                        }
                    }
                }
                WindowCommand::SetCursorVisible(_, visible) => {
                    if entry.config.cursor_visible != visible {
                        entry.config.cursor_visible = visible;
                        state.window.set_cursor_visible(visible);
                        state_changed = true;
                    }
                }
                WindowCommand::SetCursorGrab(_, mode) => {
                    if entry.config.cursor_grab != mode {
                        match state.window.set_cursor_grab(mode.to_winit()) {
                            Ok(()) => {
                                entry.config.cursor_grab = mode;
                                state_changed = true;
                            }
                            Err(error) => {
                                tracing::warn!(%error, "could not change native cursor confinement");
                            }
                        }
                    }
                }
                WindowCommand::SetCursorHitTest(_, hit_test) => {
                    if entry.config.cursor_hit_test != hit_test {
                        match state.window.set_cursor_hittest(hit_test) {
                            Ok(()) => {
                                entry.config.cursor_hit_test = hit_test;
                                state_changed = true;
                            }
                            Err(error) => {
                                tracing::warn!(%error, "could not change native pointer hit testing");
                            }
                        }
                    }
                }
                WindowCommand::SetCursorPosition(_, position) => {
                    if let Err(error) = state.window.set_cursor_position(LogicalPosition::new(
                        f64::from(position.x),
                        f64::from(position.y),
                    )) {
                        tracing::warn!(%error, "could not change native cursor position");
                    } else {
                        entry.config.cursor_position = Some(position);
                        state.pointer = Some(position);
                        state_changed = true;
                    }
                }
                WindowCommand::SetAppearance(_, preference) => {
                    if entry.config.preferred_appearance != preference {
                        entry.config.preferred_appearance = preference;
                        state.window.set_theme(preference.map(to_winit_theme));
                        let appearance = preference
                            .or_else(|| state.window.theme().map(map_window_appearance))
                            .unwrap_or(state.appearance);
                        if state.appearance != appearance {
                            state.appearance = appearance;
                            state_changed = true;
                        }
                    }
                }
                WindowCommand::SetBackgroundAppearance(_, appearance) => {
                    if entry.config.window_background != appearance {
                        let previous = entry.config.window_background;
                        let changes = appearance.changes_from(previous);
                        let surface_change = if changes.transparency {
                            state.renderer.set_transparent(appearance.is_transparent())
                        } else {
                            Ok(false)
                        };
                        match surface_change {
                            Ok(_) => {
                                if changes.transparency {
                                    state.window.set_transparent(appearance.is_transparent());
                                }
                                if changes.blur {
                                    state.window.set_blur(appearance.is_blurred());
                                }
                                entry.config.window_background = appearance;
                                state_changed = true;
                                force_redraw = true;
                            }
                            Err(error) => {
                                tracing::warn!(%error, "could not change window background appearance");
                            }
                        }
                    }
                }
                #[cfg(feature = "inspector")]
                WindowCommand::SetInspector(_, open) => {
                    if state.inspector.is_some() != open {
                        entry.config.inspector = open;
                        state.inspector = open.then(|| InspectorState::new(self.animation_epoch));
                        reconcile_inspector_pointer_state(state);
                        state_changed = true;
                        inspector_redraw = true;
                    }
                }
                #[cfg(feature = "inspector")]
                WindowCommand::ToggleInspector(_) => {
                    let open = state.inspector.is_none();
                    entry.config.inspector = open;
                    state.inspector = open.then(|| InspectorState::new(self.animation_epoch));
                    reconcile_inspector_pointer_state(state);
                    state_changed = true;
                    inspector_redraw = true;
                }
                WindowCommand::RequestAttention(_) => state
                    .window
                    .request_user_attention(Some(UserAttentionType::Informational)),
            }

            #[cfg(feature = "inspector")]
            let redraw = force_redraw
                || inspector_redraw
                || state_changed && state.listeners.observes_window_state;
            #[cfg(not(feature = "inspector"))]
            let redraw = force_redraw || state_changed && state.listeners.observes_window_state;
            if redraw {
                state.view_dirty |= force_redraw || state.listeners.observes_window_state;
                if state.visible && state.scheduler.invalidate() {
                    state.window.request_redraw();
                }
            }

            #[cfg(target_os = "macos")]
            match tabbing_ownership_delta {
                1 => self.register_native_tabbing(event_loop),
                -1 => self.unregister_native_tabbing(event_loop),
                _ => {}
            }
        }
        #[cfg(target_os = "macos")]
        if refresh_native_tabs {
            self.refresh_native_tab_states();
        }
    }

    fn invoke_open_urls(&mut self, event_loop: &ActiveEventLoop, urls: OpenUrls) {
        let Some(mut callback) = self.application_callbacks.open_urls.take() else {
            return;
        };
        let mut context = self.event_context();
        callback(urls, &mut context);
        self.application_callbacks.open_urls = Some(callback);
        self.apply_application_context(event_loop, context);
    }

    #[cfg(target_os = "macos")]
    fn invoke_reopen(&mut self, event_loop: &ActiveEventLoop, has_visible_windows: bool) {
        let Some(mut callback) = self.application_callbacks.reopen.take() else {
            return;
        };
        let mut context = self.event_context();
        callback(has_visible_windows, &mut context);
        self.application_callbacks.reopen = Some(callback);
        self.apply_application_context(event_loop, context);
    }

    #[cfg(target_os = "macos")]
    fn invoke_system_wake(&mut self, event_loop: &ActiveEventLoop) {
        let Some(mut callback) = self.application_callbacks.system_wake.take() else {
            return;
        };
        let mut context = self.event_context();
        callback(&mut context);
        self.application_callbacks.system_wake = Some(callback);
        self.apply_application_context(event_loop, context);
    }

    #[cfg(target_os = "macos")]
    fn invoke_keyboard_layout_change(&mut self, event_loop: &ActiveEventLoop) {
        let Some(mut callback) = self.application_callbacks.keyboard_layout.take() else {
            return;
        };
        let layout = self.keyboard.layout().clone();
        let mut context = self.event_context();
        callback(&layout, &mut context);
        self.application_callbacks.keyboard_layout = Some(callback);
        self.apply_application_context(event_loop, context);
    }

    fn invoke_system_notification_response(
        &mut self,
        event_loop: &ActiveEventLoop,
        response: SystemNotificationResponse,
    ) {
        let Some(mut callback) = self
            .application_callbacks
            .system_notification_response
            .take()
        else {
            return;
        };
        let mut context = self.event_context();
        callback(response, &mut context);
        self.application_callbacks.system_notification_response = Some(callback);
        self.apply_application_context(event_loop, context);
    }

    fn apply_application_context(&mut self, event_loop: &ActiveEventLoop, context: EventContext) {
        debug_assert!(self.current_window.is_none());
        debug_assert!(self.window.is_none());
        if self.apply_event_context(event_loop, context, false, false) || self.exit_requested {
            self.process_window_commands(event_loop);
        }
    }

    #[cfg(any(
        target_os = "macos",
        target_os = "windows",
        target_os = "linux",
        target_os = "freebsd",
        target_os = "dragonfly",
        target_os = "openbsd",
        target_os = "netbsd"
    ))]
    fn windows_share_parent_chain(&self, first: WindowHandle, second: WindowHandle) -> bool {
        self.window_is_ancestor(first, second) || self.window_is_ancestor(second, first)
    }

    #[cfg(any(
        target_os = "macos",
        target_os = "windows",
        target_os = "linux",
        target_os = "freebsd",
        target_os = "dragonfly",
        target_os = "openbsd",
        target_os = "netbsd"
    ))]
    fn window_is_ancestor(&self, ancestor: WindowHandle, mut window: WindowHandle) -> bool {
        for _ in 0..=self.windows.len() {
            if ancestor == window {
                return true;
            }
            let Some(window_id) = self.window_handles.get(&window) else {
                return false;
            };
            let Some(parent) = self
                .windows
                .get(window_id)
                .and_then(|entry| entry.state.parent)
            else {
                return false;
            };
            window = parent;
        }
        false
    }

    fn close_requested_window_trees(&mut self, event_loop: &ActiveEventLoop) -> Vec<ClosedWindow> {
        let mut stack = std::mem::take(&mut self.close_requests)
            .into_iter()
            .map(|handle| (handle, false))
            .collect::<Vec<_>>();
        let mut discovered = HashSet::new();
        let mut order = Vec::new();
        while let Some((handle, expanded)) = stack.pop() {
            if expanded {
                order.push(handle);
                continue;
            }
            if !discovered.insert(handle) {
                continue;
            }
            stack.push((handle, true));
            let mut children = self
                .windows
                .values()
                .filter_map(|entry| (entry.state.parent == Some(handle)).then_some(entry.handle))
                .collect::<Vec<_>>();
            children.sort_unstable();
            for child in children.into_iter().rev() {
                stack.push((child, false));
            }
        }

        let mut closed = Vec::with_capacity(order.len());
        for handle in order {
            let Some(window_id) = self.window_handles.remove(&handle) else {
                continue;
            };
            let (parent, restore_focus) = self
                .windows
                .get(&window_id)
                .map(|entry| (entry.state.parent, entry.state.restore_focus_on_close))
                .unwrap_or((None, None));
            self.foreground_tasks.cancel_window(handle);
            #[cfg(any(target_os = "macos", target_os = "windows"))]
            self.pending_native_popup_menus
                .retain(|_, popup| popup.window != handle);
            #[cfg(target_os = "macos")]
            if let Some(dialog) = self.active_platform_dialogs.remove(&Some(handle)) {
                dialog.native.cancel();
            }
            #[cfg(any(
                target_os = "windows",
                target_os = "linux",
                target_os = "freebsd",
                target_os = "dragonfly",
                target_os = "openbsd",
                target_os = "netbsd"
            ))]
            self.active_platform_dialogs.remove(&Some(handle));
            #[cfg(target_os = "macos")]
            self.popover_monitor.unwatch(handle);
            if let Some(entry) = self.windows.remove(&window_id) {
                #[cfg(target_os = "macos")]
                let native_tabbing = entry.config.tabbing_identifier.is_some();
                #[cfg(target_os = "macos")]
                if entry.state.relation_presented
                    && let Err(error) =
                        dismiss_window_relation(&entry.state.window, entry.config.kind)
                {
                    tracing::warn!(%error, "could not dismiss closing native window relation");
                }
                #[cfg(target_os = "macos")]
                if let Err(error) = set_window_visibility(&entry.state.window, false, false) {
                    tracing::warn!(%error, "could not hide closing native window");
                }
                #[cfg(not(target_os = "macos"))]
                entry.state.window.set_visible(false);
                #[cfg(target_os = "macos")]
                if native_tabbing {
                    self.unregister_native_tabbing(event_loop);
                }
            }
            self.focus_history
                .retain(|candidate| *candidate != window_id);
            if self.active_window == Some(window_id) {
                self.active_window = self.focus_history.last().copied();
            }
            closed.push(ClosedWindow {
                handle,
                parent,
                restore_focus,
            });
        }
        #[cfg(target_os = "macos")]
        if !closed.is_empty() {
            self.refresh_native_tab_states();
        }
        closed
    }

    fn invoke_window_closed_callbacks(
        &mut self,
        event_loop: &ActiveEventLoop,
        closed: Vec<ClosedWindow>,
    ) {
        for closed in closed {
            if let Some(parent) = closed.parent
                && let Some(window_id) = self.window_handles.get(&parent).copied()
                && self.activate_window(window_id)
            {
                let callbacks = self
                    .window
                    .as_ref()
                    .map(|window| {
                        let mut callbacks =
                            Vec::with_capacity(window.listeners.any_child_window_closed.len() + 1);
                        if let Some(callback) = window
                            .listeners
                            .child_window_closed
                            .get(&closed.handle)
                            .cloned()
                        {
                            callbacks.push(callback);
                        }
                        callbacks.extend(window.listeners.any_child_window_closed.iter().cloned());
                        callbacks
                    })
                    .unwrap_or_default();
                if !callbacks.is_empty() || closed.restore_focus.is_some() {
                    let mut context = self.event_context();
                    context.focus = closed.restore_focus.map(Some);
                    if let Some(window) = &mut self.window {
                        for callback in callbacks {
                            callback(window.view.as_any_mut(), closed.handle, &mut context);
                        }
                    }
                    let _ = self.apply_event_context(event_loop, context, false, true);
                }
                self.deactivate_window();
                if self.fatal_error.is_some() {
                    return;
                }
            }

            if let Some(mut callback) = self.application_callbacks.window_closed.take() {
                let mut context = self.event_context();
                callback(closed.handle, &mut context);
                self.application_callbacks.window_closed = Some(callback);
                let _ = self.apply_event_context(event_loop, context, false, false);
                if self.fatal_error.is_some() {
                    return;
                }
            }
        }
    }

    fn invoke_quit_callback(
        &mut self,
        event_loop: &ActiveEventLoop,
        request: QuitRequest,
        before: bool,
    ) -> bool {
        let callback = if before {
            self.application_callbacks.before_quit.take()
        } else {
            self.application_callbacks.will_quit.take()
        };
        let Some(mut callback) = callback else {
            return true;
        };
        let mut context = self.event_context();
        self.quit_phase_active = true;
        callback(request, &mut context);
        let prevented = context.prevent_quit;
        if before {
            self.application_callbacks.before_quit = Some(callback);
        } else {
            self.application_callbacks.will_quit = Some(callback);
        }
        let _ = self.apply_event_context(event_loop, context, false, false);
        self.quit_phase_active = false;
        !prevented && self.fatal_error.is_none()
    }

    fn process_pending_quit(&mut self, event_loop: &ActiveEventLoop) {
        let Some(reason) = self.pending_quit.take() else {
            return;
        };
        if self.exit_requested {
            return;
        }
        let request = QuitRequest { reason };
        let accepted = self.invoke_quit_callback(event_loop, request, true)
            && self.invoke_quit_callback(event_loop, request, false);
        if !accepted {
            if reason == QuitReason::Relaunch {
                self.relaunch_request = None;
            }
            if reason == QuitReason::LastWindowClosed {
                self.last_window_quit_prevented = true;
            }
            #[cfg(target_os = "macos")]
            if reason == QuitReason::OperatingSystem && self.native_termination_pending {
                if let Some(host) = self.mac_application_host.as_ref() {
                    host.reply_to_application_should_terminate(false);
                }
                self.native_termination_pending = false;
            }
            return;
        }
        self.exit_requested = true;
        self.pending_windows.clear();
        self.targeted_actions.clear();
        self.close_requests
            .extend(self.window_handles.keys().copied());
    }

    fn process_window_commands(&mut self, event_loop: &ActiveEventLoop) {
        debug_assert!(self.current_window.is_none());
        debug_assert!(self.window.is_none());

        for _ in 0..MAX_WINDOW_LIFECYCLE_TURNS {
            if !self.process_deferred_effects(event_loop) {
                return;
            }

            self.process_pending_quit(event_loop);
            if self.fatal_error.is_some() {
                return;
            }

            if self.exit_requested {
                self.pending_windows.clear();
                self.close_requests
                    .extend(self.window_handles.keys().copied());
            } else {
                // Window targeting must use the work area that exists at the placement boundary.
                // On macOS, adding a command-line application's Dock presence can resize a
                // left/right Dock after `resumed` without a screen-parameters notification. One
                // bounded refresh per non-popover creation batch keeps default centering current;
                // system-popover churn, the idle path, and ordinary frames perform no monitor
                // query.
                if self
                    .pending_windows
                    .iter()
                    .any(|request| request.options.kind != WindowKind::SystemPopover)
                {
                    self.refresh_displays(event_loop);
                }
                while let Some(mut request) = self.pending_windows.pop_front() {
                    if let Err(error) = self.resolve_pending_system_popover_anchor(&mut request) {
                        self.fail(event_loop, error);
                        return;
                    }
                    self.create_window(event_loop, request);
                    if self.fatal_error.is_some() {
                        return;
                    }
                    if !self.process_deferred_effects(event_loop) {
                        return;
                    }
                    if self.exit_requested {
                        self.pending_windows.clear();
                        self.close_requests
                            .extend(self.window_handles.keys().copied());
                        break;
                    }
                }
            }

            if !self.process_targeted_actions(event_loop) {
                return;
            }

            self.process_queued_window_commands(event_loop);

            self.process_global_shortcut_commands();
            self.process_tray_commands();

            if let Some(menus) = self.external_menus.take()
                && !self.replace_menus(event_loop, menus)
            {
                return;
            }

            for handle in std::mem::take(&mut self.invalidate_requests) {
                let Some(window_id) = self.window_handles.get(&handle).copied() else {
                    continue;
                };
                let Some(entry) = self.windows.get_mut(&window_id) else {
                    continue;
                };
                entry.state.view_dirty = true;
                if entry.state.scheduler.invalidate() {
                    entry.state.window.request_redraw();
                }
            }

            for handle in std::mem::take(&mut self.focus_requests) {
                let Some(window_id) = self.window_handles.get(&handle).copied() else {
                    continue;
                };
                if let Some(entry) = self.windows.get(&window_id) {
                    if !entry.config.focusable {
                        continue;
                    }
                    #[cfg(target_os = "macos")]
                    if matches!(
                        entry.config.kind,
                        WindowKind::Popover | WindowKind::SystemPopover
                    ) {
                        if let Err(error) = set_window_visibility(&entry.state.window, true, true) {
                            tracing::warn!(%error, "could not focus native popover without activation");
                        }
                    } else {
                        entry.state.window.focus_window();
                    }
                    #[cfg(not(target_os = "macos"))]
                    entry.state.window.focus_window();
                    self.note_window_focused(window_id);
                }
            }

            let closed = self.close_requested_window_trees(event_loop);
            self.invoke_window_closed_callbacks(event_loop, closed);
            if self.fatal_error.is_some() {
                return;
            }

            self.process_platform_requests();

            #[cfg(target_os = "macos")]
            self.sync_active_native_menu_state();

            if self.exit_requested {
                self.pending_windows.clear();
                if self.windows.is_empty() {
                    #[cfg(target_os = "macos")]
                    if self.native_termination_pending {
                        if let Some(host) = self.mac_application_host.as_ref() {
                            host.reply_to_application_should_terminate(true);
                        }
                        self.native_termination_pending = false;
                        return;
                    }
                    event_loop.exit();
                    return;
                }
                continue;
            }

            let lifecycle_pending = !self.pending_windows.is_empty()
                || !self.targeted_actions.is_empty()
                || !self.close_requests.is_empty()
                || !self.window_commands.is_empty()
                || !self.focus_requests.is_empty()
                || !self.invalidate_requests.is_empty();
            if lifecycle_pending {
                continue;
            }

            if self.opened_window
                && self.windows.is_empty()
                && self.quit_mode.quits_when_empty()
                && !self.last_window_quit_prevented
            {
                self.pending_quit = Some(QuitReason::LastWindowClosed);
                continue;
            }
            return;
        }

        self.fail(
            event_loop,
            AppError::View(format!(
                "window lifecycle exceeded {MAX_WINDOW_LIFECYCLE_TURNS} effect turns"
            )),
        );
    }

    fn resolve_pending_system_popover_anchor(
        &self,
        request: &mut WindowRequest,
    ) -> Result<(), AppError> {
        let Some(anchor) = request.popover_anchor_element else {
            return Ok(());
        };
        let parent = request.parent.ok_or_else(|| {
            AppError::Window(WindowCommandError::PopoverParentRequired.to_string())
        })?;
        let bounds = self
            .window_handles
            .get(&parent)
            .and_then(|window_id| self.windows.get(window_id))
            .and_then(|entry| entry.state.ui.element_bounds(anchor))
            .ok_or_else(|| {
                AppError::Window(format!(
                    "system popover anchor {anchor:?} is not mounted in its parent window"
                ))
            })?;
        let popover = request.options.popover.as_mut().ok_or_else(|| {
            AppError::Window(WindowCommandError::InvalidPopoverConfiguration.to_string())
        })?;
        popover.anchor_rect = bounds;
        Ok(())
    }

    fn process_targeted_actions(&mut self, event_loop: &ActiveEventLoop) -> bool {
        let mut deliveries = 0_usize;
        while let Some((handle, action)) = self.targeted_actions.pop_front() {
            if deliveries == crate::MAX_PENDING_TARGETED_ACTIONS {
                self.fail(
                    event_loop,
                    AppError::View(format!(
                        "one effect cycle exceeded {} cross-window action deliveries",
                        crate::MAX_PENDING_TARGETED_ACTIONS
                    )),
                );
                return false;
            }
            deliveries += 1;
            let Some(window_id) = self.window_handles.get(&handle).copied() else {
                continue;
            };
            if !self.activate_window(window_id) {
                continue;
            }
            let delivered = self.invoke_action(event_loop, &action).is_some();
            self.deactivate_window();
            if !delivered {
                return false;
            }
        }
        true
    }

    fn resume_deferred_image_loads(&mut self) {
        debug_assert!(self.current_window.is_none());
        let window_ids = self.windows.keys().copied().collect::<Vec<_>>();
        for window_id in window_ids {
            if !self.activate_window(window_id) {
                continue;
            }
            if let Some(state) = &mut self.window
                && state.image_assets.resume_deferred()
            {
                state.view_dirty = true;
                if state.scheduler.invalidate() {
                    state.window.request_redraw();
                }
            }
            self.deactivate_window();
        }
    }

    fn invalidate_entity_observers(&mut self, entities: &[EntityId], all: bool) {
        if entities.is_empty() && !all {
            return;
        }

        if let Some(state) = &mut self.window
            && state.listeners.observes_entity_change(entities, all)
        {
            state.view_dirty = true;
            if state.scheduler.invalidate() {
                state.window.request_redraw();
            }
        }

        for entry in self.windows.values_mut() {
            if !entry.state.listeners.observes_entity_change(entities, all) {
                continue;
            }
            entry.state.view_dirty = true;
            if entry.state.scheduler.invalidate() {
                entry.state.window.request_redraw();
            }
        }
    }

    fn invalidate_global_observers(&mut self, global_types: &[TypeId], all: bool) {
        if global_types.is_empty() && !all {
            return;
        }

        if let Some(state) = &mut self.window
            && state.listeners.observes_global_change(global_types, all)
        {
            state.view_dirty = true;
            if state.scheduler.invalidate() {
                state.window.request_redraw();
            }
        }

        for entry in self.windows.values_mut() {
            if !entry
                .state
                .listeners
                .observes_global_change(global_types, all)
            {
                continue;
            }
            entry.state.view_dirty = true;
            if entry.state.scheduler.invalidate() {
                entry.state.window.request_redraw();
            }
        }
    }

    fn dispatch(&mut self, event_loop: &ActiveEventLoop, event: Event, force_redraw: bool) -> bool {
        let mut cx = self.event_context();
        let Some(window) = &mut self.window else {
            return false;
        };
        window.view.event(&event, &mut cx);
        self.apply_event_context(event_loop, cx, force_redraw, true)
    }

    fn apply_event_context(
        &mut self,
        event_loop: &ActiveEventLoop,
        mut cx: EventContext,
        force_redraw: bool,
        announce_focus: bool,
    ) -> bool {
        let relaunch_requested = cx.relaunch.is_some();
        if let Some(request) = cx.relaunch.take() {
            self.relaunch_request = Some(request);
        }
        if cx.exit && !self.quit_phase_active {
            self.pending_quit = Some(if relaunch_requested {
                QuitReason::Relaunch
            } else {
                QuitReason::Explicit
            });
            return false;
        }
        let entity_notifications = std::mem::take(&mut cx.entity_notifications);
        let notify_all_entities = cx.notify_all_entities;
        let global_notifications = std::mem::take(&mut cx.global_notifications);
        let notify_all_globals = cx.notify_all_globals;
        if !enqueue_global_notifications(
            &mut self.pending_global_notifications,
            &mut self.pending_global_notification_types,
            &mut self.pending_all_globals,
            &global_notifications,
            notify_all_globals,
        ) {
            self.fail(
                event_loop,
                AppError::View(format!(
                    "one effect cycle cannot retain more than {MAX_PENDING_GLOBAL_NOTIFICATIONS} pending global notifications"
                )),
            );
            return false;
        }
        if !enqueue_entity_events(&mut self.pending_entity_events, &mut cx.entity_events) {
            self.fail(
                event_loop,
                AppError::View(format!(
                    "one effect cycle cannot retain more than {MAX_PENDING_ENTITY_EVENTS} pending entity events"
                )),
            );
            return false;
        }
        if self.targeted_actions.len() + cx.targeted_actions.len()
            > crate::MAX_PENDING_TARGETED_ACTIONS
        {
            self.fail(
                event_loop,
                AppError::View(format!(
                    "one effect cycle cannot retain more than {} cross-window actions",
                    crate::MAX_PENDING_TARGETED_ACTIONS
                )),
            );
            return false;
        }
        self.targeted_actions.extend(cx.targeted_actions.drain(..));
        for request in &mut cx.open_windows {
            let Some(anchor) = request.popover_anchor_element else {
                continue;
            };
            let Some(bounds) = self
                .window
                .as_ref()
                .and_then(|state| state.ui.element_bounds(anchor))
            else {
                self.fail(
                    event_loop,
                    AppError::View(format!(
                        "system popover anchor {anchor:?} is not mounted in its parent window"
                    )),
                );
                return false;
            };
            let Some(popover) = request.options.popover.as_mut() else {
                self.fail(
                    event_loop,
                    AppError::Window(WindowCommandError::InvalidPopoverConfiguration.to_string()),
                );
                return false;
            };
            popover.anchor_rect = bounds;
        }
        self.pending_windows.extend(cx.open_windows.drain(..));
        if cx.close_current_window
            && let Some(handle) = self.current_handle()
        {
            self.close_requests.push(handle);
        }
        self.close_requests.append(&mut cx.close_windows);
        self.focus_requests.append(&mut cx.focus_windows);
        self.invalidate_requests.append(&mut cx.invalidate_windows);
        if self.window_commands.len() + cx.window_commands.len() > MAX_PENDING_WINDOW_COMMANDS {
            self.fail(
                event_loop,
                AppError::View(format!(
                    "one effect cycle cannot retain more than {MAX_PENDING_WINDOW_COMMANDS} pending window commands"
                )),
            );
            return false;
        }
        self.window_commands.append(&mut cx.window_commands);
        enqueue_platform_requests(&mut self.platform_requests, &mut cx.platform_requests);
        let actions = std::mem::take(&mut cx.actions);
        let form_submissions = std::mem::take(&mut cx.form_submissions);
        let menus = cx.menus.take();
        let window_menus = cx.window_menus.take();
        let native_popup_menus = std::mem::take(&mut cx.native_popup_menus);
        let mut focus_changed = false;
        if let Some(state) = &mut self.window {
            let previous_focus = state.ui.focused();
            let mut deferred_focus = false;
            let selection_changed =
                cx.clear_text_selection && state.ui.clear_static_text_selection();
            if let Some(request) = cx.focus {
                match request {
                    Some(id) => {
                        if state.ui.is_focusable(id) {
                            state.pending_focus = None;
                            state.ui.focus(id);
                        } else {
                            // The caller may be opening a view that declares this focus handle in
                            // the rebuild requested by the same event. Keep the request for exactly
                            // that rebuild instead of adding a timer or requiring a second event.
                            state.pending_focus = Some(id);
                            state.view_dirty = true;
                            deferred_focus = true;
                        }
                    }
                    None => {
                        state.pending_focus = None;
                        state.ui.blur();
                    }
                }
            }
            focus_changed = previous_focus != state.ui.focused();
            if focus_changed {
                self.pending_input = None;
            }
            #[cfg(target_os = "macos")]
            if focus_changed
                && state.ui.focused().is_some()
                && let Some(host) = &state.native_host
            {
                host.focus_framework();
            }
            if cx.invalidate {
                state.view_dirty = true;
            }
            if (force_redraw
                || cx.invalidate
                || focus_changed
                || deferred_focus
                || selection_changed)
                && state.scheduler.invalidate()
            {
                state.window.request_redraw();
            }
        }
        if focus_changed && announce_focus {
            let focused = self.window.as_ref().and_then(|state| state.ui.focused());
            let mut focus_cx = self.event_context();
            if let Some(window) = &mut self.window {
                window
                    .view
                    .event(&Event::FocusChanged(focused), &mut focus_cx);
            }
            if !self.apply_event_context(event_loop, focus_cx, false, false) {
                return false;
            }
        }
        #[cfg(target_os = "macos")]
        if focus_changed {
            self.sync_native_menu_state();
        }
        if let Some(menus) = menus
            && !self.replace_menus(event_loop, menus)
        {
            return false;
        }
        if let Some(window_menus) = window_menus
            && !self.replace_current_window_menus(event_loop, window_menus)
        {
            return false;
        }
        #[cfg(any(target_os = "macos", target_os = "windows"))]
        for popup in native_popup_menus {
            if !self.show_current_native_popup_menu(event_loop, popup) {
                return false;
            }
        }
        #[cfg(not(any(target_os = "macos", target_os = "windows")))]
        debug_assert!(native_popup_menus.is_empty());
        for action in actions {
            if self.invoke_action(event_loop, &action).is_none() {
                return false;
            }
        }
        for form in form_submissions {
            if !self.invoke_form_submission(event_loop, form, None) {
                return false;
            }
        }
        self.invalidate_entity_observers(&entity_notifications, notify_all_entities);
        self.invalidate_global_observers(&global_notifications, notify_all_globals);
        true
    }

    fn finalize_process_services(&mut self) {
        if self.process_services_finalized {
            return;
        }
        self.process_services_finalized = true;
        global_shortcut::clear_global_shortcut_handler_proxy();
        tray::clear_tray_handler_proxy();
        #[cfg(any(target_os = "macos", target_os = "windows", target_os = "linux"))]
        self.global_shortcuts.clear();
        self.tray_icons.clear();
        #[cfg(any(
            target_os = "macos",
            target_os = "windows",
            target_os = "linux",
            target_os = "freebsd",
            target_os = "dragonfly",
            target_os = "openbsd",
            target_os = "netbsd"
        ))]
        {
            self.single_instance.take();
        }
        #[cfg(target_os = "macos")]
        {
            self.menu_host.take();
            self.mac_application_host.take();
            for (_, dialog) in self.active_platform_dialogs.drain() {
                dialog.native.cancel();
            }
        }
        #[cfg(target_os = "windows")]
        {
            self.windows_menu_host.take();
            self._windows_power_monitor.take();
        }
        #[cfg(target_os = "linux")]
        {
            self._linux_power_monitor.take();
        }
        #[cfg(any(
            target_os = "windows",
            target_os = "linux",
            target_os = "freebsd",
            target_os = "dragonfly",
            target_os = "openbsd",
            target_os = "netbsd"
        ))]
        self.active_platform_dialogs.clear();
        self.foreground_tasks.shutdown();
        self.background_tasks.shutdown();
        self.image_workers.shutdown();
    }

    /// Returns `None` after exit, otherwise whether a handler consumed the action.
    fn invoke_action(&mut self, event_loop: &ActiveEventLoop, action: &AnyAction) -> Option<bool> {
        let Some(window) = &mut self.window else {
            return Some(false);
        };
        let path = window.ui.focus_path();
        let mut dispatch = std::mem::take(&mut window.action_dispatch_scratch);
        window
            .ui
            .collect_action_dispatch(&path, action.type_id(), &mut dispatch);

        for binding in dispatch.iter().copied() {
            let listener = self
                .window
                .as_ref()
                .and_then(|window| window.listeners.action_listener(binding.key));
            let Some(listener) = listener else {
                continue;
            };
            let mut cx = self.event_context();
            if let Some(window) = &mut self.window {
                listener(window.view.as_any_mut(), action.as_any(), &mut cx);
            }
            let propagate = cx.propagate_action;
            let stopped = cx.stop_event_propagation;
            if !self.apply_event_context(event_loop, cx, false, true) {
                return None;
            }
            let consumed = match binding.phase {
                crate::DispatchPhase::Capture => stopped,
                crate::DispatchPhase::Bubble => !propagate,
            };
            if consumed {
                dispatch.clear();
                if let Some(window) = &mut self.window {
                    window.action_dispatch_scratch = dispatch;
                }
                return Some(true);
            }
        }

        dispatch.clear();
        if let Some(window) = &mut self.window {
            window.action_dispatch_scratch = dispatch;
        }
        Some(false)
    }

    fn action_available(&self, action: &AnyAction) -> bool {
        let Some(window) = &self.window else {
            return false;
        };
        let path = window.ui.focus_path();
        window.ui.action_available(&path, action.type_id())
    }

    #[cfg(target_os = "macos")]
    fn invoke_dock_menu_action(&mut self, event_loop: &ActiveEventLoop, action_id: usize) {
        let item = self
            .dock_menu_actions
            .get(action_id)
            .filter(|item| !item.disabled)
            .map(|item| (item.action.clone(), item.os_action));
        let Some((action, os_action)) = item else {
            return;
        };
        let target = self
            .active_window
            .or_else(|| self.focus_history.last().copied());
        if let Some(target) = target
            && !self.activate_window(target)
        {
            return;
        }
        let handled = action
            .as_ref()
            .is_some_and(|action| self.invoke_action(event_loop, action).unwrap_or(true));
        if !handled && let Some(os_action) = os_action {
            self.invoke_os_action(event_loop, os_action);
        }
        if target.is_some() {
            self.deactivate_window();
        }
        self.process_window_commands(event_loop);
    }

    #[cfg(any(target_os = "macos", target_os = "windows"))]
    fn active_menu_declaration(&self) -> &[Menu] {
        let Some(active) = self.active_window else {
            return &self.menus;
        };
        if self.current_window.is_some_and(|(id, _)| id == active) {
            return self.config.window_menus.as_deref().unwrap_or(&self.menus);
        }
        self.windows
            .get(&active)
            .and_then(|entry| entry.config.window_menus.as_deref())
            .unwrap_or(&self.menus)
    }

    #[cfg(target_os = "macos")]
    fn install_active_mac_menu(&mut self, event_loop: &ActiveEventLoop) -> bool {
        let menus = self.active_menu_declaration().to_vec();
        let host = match MacMenuHost::new(&menus, self.event_proxy.clone()) {
            Ok(host) => host,
            Err(error) => {
                self.fail(event_loop, AppError::Platform(error));
                return false;
            }
        };
        self.menu_actions = collect_menu_actions(&menus);
        self.menu_host = Some(host);
        self.sync_active_native_menu_state();
        true
    }

    fn replace_menus(&mut self, event_loop: &ActiveEventLoop, menus: Vec<Menu>) -> bool {
        if let Err(error) = validate_menus(&menus) {
            self.fail(event_loop, AppError::Platform(error.to_string()));
            return false;
        }
        #[cfg(not(any(target_os = "macos", target_os = "windows")))]
        let _ = event_loop;
        #[cfg(target_os = "windows")]
        let next_host = if menus.is_empty() {
            None
        } else {
            match windows_menu::WindowsMenuHost::new(&menus, self.event_proxy.clone()) {
                Ok(host) => Some(host),
                Err(error) => {
                    self.fail(event_loop, AppError::Platform(error));
                    return false;
                }
            }
        };

        self.menus = menus;
        #[cfg(target_os = "macos")]
        {
            if !self.install_active_mac_menu(event_loop) {
                return false;
            }
        }
        #[cfg(target_os = "windows")]
        {
            // Dropping the old host detaches its menu before the replacement attaches.
            self.windows_menu_host.take();
            if let Some(next_host) = &next_host {
                for entry in self.windows.values() {
                    if entry.config.window_menus.is_some() {
                        continue;
                    }
                    if let Err(error) = next_host.attach(&entry.state.window) {
                        self.fail(event_loop, AppError::Platform(error));
                        return false;
                    }
                }
                if self.config.window_menus.is_none()
                    && let Some(window) = &self.window
                    && let Err(error) = next_host.attach(&window.window)
                {
                    self.fail(event_loop, AppError::Platform(error));
                    return false;
                }
            }
            self.windows_menu_host = next_host;
        }
        #[cfg(not(any(target_os = "macos", target_os = "windows")))]
        let _ = menus;
        true
    }

    fn replace_current_window_menus(
        &mut self,
        event_loop: &ActiveEventLoop,
        menus: Option<Vec<Menu>>,
    ) -> bool {
        if let Some(menus) = menus.as_deref()
            && let Err(error) = validate_menus(menus)
        {
            self.fail(event_loop, AppError::Platform(error.to_string()));
            return false;
        }
        let Some((_window_id, _)) = self.current_window else {
            return false;
        };

        #[cfg(target_os = "windows")]
        let next_host = match menus.as_deref() {
            Some([]) | None => None,
            Some(menus) => {
                match windows_menu::WindowsMenuHost::new(menus, self.event_proxy.clone()) {
                    Ok(host) => Some(host),
                    Err(error) => {
                        self.fail(event_loop, AppError::Platform(error));
                        return false;
                    }
                }
            }
        };

        #[cfg(target_os = "windows")]
        {
            let Some(state) = self.window.as_mut() else {
                return false;
            };
            if self.config.window_menus.is_none()
                && let Some(host) = &self.windows_menu_host
                && let Err(error) = host.detach(&state.window)
            {
                self.fail(event_loop, AppError::Platform(error));
                return false;
            }
            state.window_menu_host.take();
            if menus.is_some() {
                if let Some(host) = &next_host
                    && let Err(error) = host.attach(&state.window)
                {
                    self.fail(event_loop, AppError::Platform(error));
                    return false;
                }
                state.window_menu_host = next_host;
            } else if let Some(host) = &self.windows_menu_host
                && let Err(error) = host.attach(&state.window)
            {
                self.fail(event_loop, AppError::Platform(error));
                return false;
            }
        }

        self.config.window_menus = menus;

        #[cfg(target_os = "macos")]
        if self.active_window == Some(_window_id) && !self.install_active_mac_menu(event_loop) {
            return false;
        }
        #[cfg(not(any(target_os = "macos", target_os = "windows")))]
        let _ = (event_loop, _window_id);
        true
    }

    #[cfg(any(target_os = "macos", target_os = "windows"))]
    fn show_current_native_popup_menu(
        &mut self,
        event_loop: &ActiveEventLoop,
        request: crate::event::NativePopupMenuRequest,
    ) -> bool {
        if self.pending_native_popup_menus.len() == MAX_PENDING_NATIVE_POPUP_MENUS {
            self.fail(
                event_loop,
                AppError::View(format!(
                    "the application cannot retain more than {MAX_PENDING_NATIVE_POPUP_MENUS} selected native popup menus"
                )),
            );
            return false;
        }
        let Some(handle) = self.current_handle() else {
            return false;
        };
        let actions = collect_menu_actions(std::slice::from_ref(&request.menu));
        static NEXT_POPUP_MENU_ID: AtomicU64 = AtomicU64::new(1);
        let popup_id = NEXT_POPUP_MENU_ID.fetch_add(1, Ordering::Relaxed).max(1);

        #[cfg(target_os = "macos")]
        let states = {
            let contexts = self
                .window
                .as_ref()
                .map(|window| window.ui.key_context_stack())
                .unwrap_or_default();
            actions
                .iter()
                .map(|item| MacMenuItemState {
                    disabled: item.disabled,
                    action_available: item
                        .action
                        .as_ref()
                        .is_some_and(|action| self.action_available(action))
                        || item
                            .os_action
                            .is_some_and(|action| self.os_action_available(action)),
                    checked: item.checked,
                    shortcut: item.action.as_ref().and_then(|action| {
                        self.keymap.shortcut_for_action_value(action, &contexts)
                    }),
                })
                .collect::<Vec<_>>()
        };
        #[cfg(target_os = "macos")]
        let native_focus_active = self
            .window
            .as_ref()
            .and_then(|window| window.native_host.as_ref())
            .is_some_and(MacNativeHost::native_focus_active);

        self.pending_native_popup_menus.insert(
            popup_id,
            PendingNativePopupMenu {
                window: handle,
                actions,
            },
        );

        #[cfg(target_os = "windows")]
        tray::install_native_menu_handlers(self.event_proxy.clone());
        let result = {
            let Some(window) = self.window.as_ref() else {
                self.pending_native_popup_menus.remove(&popup_id);
                return false;
            };
            #[cfg(target_os = "macos")]
            {
                crate::macos_menu::show_popup_menu(
                    &request.menu,
                    popup_id,
                    &window.window,
                    request.position,
                    &states,
                    native_focus_active,
                    self.event_proxy.clone(),
                )
            }
            #[cfg(target_os = "windows")]
            {
                windows_menu::show_popup_menu(
                    &request.menu,
                    popup_id,
                    &window.window,
                    request.position,
                )
            }
        };
        let shown = match result {
            Ok(shown) => shown,
            Err(error) => {
                self.pending_native_popup_menus.remove(&popup_id);
                self.fail(event_loop, AppError::Platform(error));
                return false;
            }
        };
        if !shown {
            self.pending_native_popup_menus.remove(&popup_id);
            return true;
        }
        if self
            .event_proxy
            .send_event(RuntimeEvent::NativePopupMenuClosed(popup_id))
            .is_err()
        {
            self.pending_native_popup_menus.remove(&popup_id);
        }
        true
    }

    fn os_action_available(&self, action: OsAction) -> bool {
        let window = self.window.as_ref();
        let input_focused = window.is_some_and(|window| window.ui.focused_text_input().is_some());
        match action {
            OsAction::Cut => {
                input_focused
                    && window
                        .expect("input focus requires a current window")
                        .ui
                        .selected_input_text()
                        .is_some_and(|selection| !selection.is_empty())
            }
            OsAction::Copy => window.is_some_and(|window| {
                window
                    .ui
                    .selected_text()
                    .is_some_and(|selection| !selection.is_empty())
            }),
            OsAction::Paste => input_focused,
            OsAction::SelectAll => {
                input_focused || window.is_some_and(|window| window.ui.has_selectable_text())
            }
            OsAction::Undo => {
                input_focused && window.is_some_and(|window| window.ui.input_can_undo())
            }
            OsAction::Redo => {
                input_focused && window.is_some_and(|window| window.ui.input_can_redo())
            }
            OsAction::About => cfg!(any(target_os = "macos", target_os = "windows")),
            OsAction::ShowHelp => cfg!(target_os = "macos"),
            OsAction::HideApplication
            | OsAction::ShowAllApplications
            | OsAction::Quit
            | OsAction::BringAllToFront => !self.window_handles.is_empty(),
            OsAction::HideOtherApplications => cfg!(target_os = "macos"),
            OsAction::CloseWindow => window.is_some() && self.config.is_closable,
            OsAction::MinimizeWindow => window.is_some() && self.config.is_minimizable,
            OsAction::ZoomWindow => {
                window.is_some() && self.config.is_resizable && self.config.is_maximizable
            }
            OsAction::ToggleFullscreen => window.is_some(),
        }
    }

    fn invoke_os_action(&mut self, event_loop: &ActiveEventLoop, action: OsAction) -> bool {
        match action {
            OsAction::Copy => {
                let selected = self
                    .window
                    .as_ref()
                    .and_then(|window| window.ui.selected_text());
                if let Some(selected) = selected {
                    let _ = self.write_clipboard_text(selected.as_ref());
                    return true;
                }
                false
            }
            OsAction::Cut => {
                let selected = self
                    .window
                    .as_ref()
                    .and_then(|window| window.ui.selected_input_text());
                let Some(selected) = selected else {
                    return false;
                };
                let copied = self.write_clipboard_text(selected.as_ref());
                if copied {
                    let result = self
                        .window
                        .as_mut()
                        .map(|window| window.ui.input_backspace())
                        .unwrap_or_default();
                    self.apply_input_result(event_loop, result, true);
                }
                true
            }
            OsAction::Paste => {
                if self
                    .window
                    .as_ref()
                    .and_then(|window| window.ui.focused_text_input())
                    .is_none()
                {
                    return false;
                }
                let pasted = self.read_clipboard_text();
                if let Some(value) = pasted {
                    let result = self
                        .window
                        .as_mut()
                        .map(|window| window.ui.input_replace(&value))
                        .unwrap_or_default();
                    self.apply_input_result(event_loop, result, true);
                }
                true
            }
            OsAction::SelectAll => {
                if !self.os_action_available(OsAction::SelectAll) {
                    return false;
                }
                let result = self
                    .window
                    .as_mut()
                    .map_or_else(InputResult::default, |window| {
                        if window.ui.focused_text_input().is_some() {
                            window.ui.input_select_all()
                        } else {
                            InputResult {
                                repaint: window.ui.select_all_static_text(),
                                change: None,
                            }
                        }
                    });
                self.apply_input_result(event_loop, result, false);
                true
            }
            OsAction::Undo => {
                if self
                    .window
                    .as_ref()
                    .and_then(|window| window.ui.focused_text_input())
                    .is_none()
                {
                    return false;
                }
                let result = self
                    .window
                    .as_mut()
                    .map(|window| window.ui.input_undo())
                    .unwrap_or_default();
                self.apply_input_result(event_loop, result, true)
            }
            OsAction::Redo => {
                if self
                    .window
                    .as_ref()
                    .and_then(|window| window.ui.focused_text_input())
                    .is_none()
                {
                    return false;
                }
                let result = self
                    .window
                    .as_mut()
                    .map(|window| window.ui.input_redo())
                    .unwrap_or_default();
                self.apply_input_result(event_loop, result, true)
            }
            OsAction::About => {
                if !DesktopIntegrationSupport::current().native_about_panel
                    || self.platform_requests.len() == crate::MAX_PENDING_PLATFORM_REQUESTS
                {
                    return false;
                }
                let Ok(request) = PlatformRequest::show_about_panel(AboutPanelOptions::default())
                else {
                    return false;
                };
                self.platform_requests.push_back(request);
                true
            }
            OsAction::HideOtherApplications | OsAction::ShowHelp => false,
            OsAction::HideApplication | OsAction::ShowAllApplications => {
                let visible = action == OsAction::ShowAllApplications;
                let mut handles = self.window_handles.keys().copied().collect::<Vec<_>>();
                handles.sort_unstable();
                self.window_commands.extend(
                    handles
                        .into_iter()
                        .map(|handle| WindowCommand::SetVisible(handle, visible)),
                );
                true
            }
            OsAction::Quit => {
                self.pending_quit = Some(QuitReason::Explicit);
                true
            }
            OsAction::CloseWindow => {
                if !self.config.is_closable {
                    return false;
                }
                let Some(handle) = self.current_handle() else {
                    return false;
                };
                self.close_requests.push(handle);
                true
            }
            OsAction::MinimizeWindow => {
                if !self.config.is_minimizable {
                    return false;
                }
                let Some(handle) = self.current_handle() else {
                    return false;
                };
                self.window_commands.push(WindowCommand::Minimize(handle));
                true
            }
            OsAction::ZoomWindow => {
                if !self.config.is_resizable || !self.config.is_maximizable {
                    return false;
                }
                let Some(handle) = self.current_handle() else {
                    return false;
                };
                self.window_commands.push(WindowCommand::Zoom(handle));
                true
            }
            OsAction::ToggleFullscreen => {
                let Some(handle) = self.current_handle() else {
                    return false;
                };
                self.window_commands
                    .push(WindowCommand::ToggleFullscreen(handle));
                true
            }
            OsAction::BringAllToFront => {
                let mut handles = self.window_handles.keys().copied().collect::<Vec<_>>();
                handles.sort_unstable();
                self.focus_requests.extend(handles);
                true
            }
        }
    }

    #[cfg(target_os = "macos")]
    fn sync_native_menu_state(&self) {
        let Some(host) = &self.menu_host else {
            return;
        };
        let contexts = self
            .window
            .as_ref()
            .map(|window| window.ui.key_context_stack())
            .unwrap_or_default();
        let states =
            self.menu_actions
                .iter()
                .map(|item| MacMenuItemState {
                    disabled: item.disabled,
                    action_available: item
                        .action
                        .as_ref()
                        .is_some_and(|action| self.action_available(action))
                        || item
                            .os_action
                            .is_some_and(|action| self.os_action_available(action)),
                    checked: item.checked,
                    shortcut: item.action.as_ref().and_then(|action| {
                        self.keymap.shortcut_for_action_value(action, &contexts)
                    }),
                })
                .collect::<Vec<_>>();
        let native_focus_active = self
            .window
            .as_ref()
            .and_then(|window| window.native_host.as_ref())
            .is_some_and(MacNativeHost::native_focus_active);
        let close_match = self.keymap.bindings_for_input(
            &[Keystroke::new(
                Key::Character("w".to_owned()),
                Modifiers::SUPER,
            )],
            &contexts,
        );
        let keymap_claims_close = close_match.pending
            || close_match
                .bindings
                .iter()
                .any(|binding| self.action_available(binding.action()));
        host.update(&states, native_focus_active, keymap_claims_close);
    }

    #[cfg(target_os = "macos")]
    fn sync_active_native_menu_state(&mut self) {
        if self.window.is_some() {
            self.sync_native_menu_state();
            return;
        }
        let Some(window_id) = self.active_window else {
            return;
        };
        if self.activate_window(window_id) {
            self.sync_native_menu_state();
            self.deactivate_window();
        }
    }

    fn invoke_click(&mut self, event_loop: &ActiveEventLoop, id: ElementId) {
        let activation_target = self
            .window
            .as_ref()
            .and_then(|window| window.ui.activation_target(id));
        let form = self
            .window
            .as_ref()
            .and_then(|window| window.ui.form_for_submitter(id));
        let listener = self
            .window
            .as_ref()
            .and_then(|window| window.listeners.clicks.get(&id).cloned());
        let mut default_prevented = false;
        if let Some(listener) = listener {
            let mut cx = self.event_context();
            if let Some(window) = &mut self.window {
                listener(window.view.as_any_mut(), &mut cx);
            }
            default_prevented = cx.prevent_default;
            if !self.apply_event_context(event_loop, cx, false, true) {
                return;
            }
        }
        if !self.dispatch(event_loop, Event::Click(id), false) {
            return;
        }
        if let Some(form) = form {
            self.invoke_form_submission(event_loop, form, Some(id));
        }
        if default_prevented {
            return;
        }
        let Some(target) = activation_target.filter(|target| *target != id) else {
            return;
        };
        let previous_focus = self.window.as_ref().and_then(|window| window.ui.focused());
        if let Some(window) = &mut self.window {
            window.ui.focus(target);
        }
        self.announce_focus_change(event_loop, previous_focus);
        let clickable = self
            .window
            .as_ref()
            .is_some_and(|window| window.ui.is_clickable(target));
        if clickable {
            self.invoke_click(event_loop, target);
        }
    }

    fn invoke_context_menu(
        &mut self,
        event_loop: &ActiveEventLoop,
        event: ContextMenuEvent,
    ) -> bool {
        let listener = self
            .window
            .as_ref()
            .and_then(|window| window.listeners.context_menus.get(&event.target).cloned());
        if let Some(listener) = listener {
            let mut cx = self.event_context();
            if let Some(window) = &mut self.window {
                listener(window.view.as_any_mut(), &event, &mut cx);
            }
            if !self.apply_event_context(event_loop, cx, false, true) {
                return false;
            }
        }
        self.dispatch(event_loop, Event::ContextMenu(event), false);
        true
    }

    fn invoke_pointer(
        &mut self,
        event_loop: &ActiveEventLoop,
        id: ElementId,
        event: PointerEvent,
    ) -> bool {
        let listener = self
            .window
            .as_ref()
            .and_then(|window| window.listeners.pointers.get(&id).cloned());
        let Some(listener) = listener else {
            return true;
        };
        let mut cx = self.event_context();
        if let Some(window) = &mut self.window {
            listener(window.view.as_any_mut(), &event, &mut cx);
        }
        self.apply_event_context(event_loop, cx, false, true)
    }

    /// Dispatch one bounded desktop mouse event through outside capture, capture, and bubble.
    ///
    /// `None` means the callback closed the runtime. `Some(true)` means at least one listener
    /// prevented the retained default behavior.
    fn invoke_mouse_event_at(
        &mut self,
        event_loop: &ActiveEventLoop,
        position: Point,
        kind: MouseListenerKind,
        button: Option<MouseButton>,
        event: MouseListenerEvent,
    ) -> Option<bool> {
        let (mut path, mut dispatch) = {
            let window = self.window.as_mut()?;
            (
                std::mem::take(&mut window.mouse_event_path_scratch),
                std::mem::take(&mut window.mouse_dispatch_scratch),
            )
        };
        let path_complete = {
            let window = self.window.as_ref()?;
            let complete = window.ui.mouse_event_path_at(position, &mut path);
            if complete {
                window
                    .ui
                    .collect_mouse_dispatch(&path, kind, button, &mut dispatch);
            }
            complete
        };
        if let Some(window) = &mut self.window {
            path.clear();
            window.mouse_event_path_scratch = path;
        }
        if !path_complete {
            tracing::warn!(
                limit = crate::MAX_MOUSE_EVENT_PATH,
                "ignored a targeted mouse event whose retained ancestor path exceeded the safety bound"
            );
            dispatch.clear();
        }

        let mut default_prevented = false;
        for key in dispatch.iter().copied() {
            let listener = self
                .window
                .as_ref()
                .and_then(|window| window.listeners.mouse_listener(key));
            let Some(listener) = listener else {
                continue;
            };
            let mut cx = self.event_context();
            if let Some(window) = &mut self.window {
                listener(window.view.as_any_mut(), &event, &mut cx);
            }
            let stop_propagation = cx.stop_event_propagation;
            default_prevented |= cx.prevent_default;
            if !self.apply_event_context(event_loop, cx, false, true) {
                return None;
            }
            if stop_propagation {
                break;
            }
        }
        if let Some(window) = &mut self.window {
            dispatch.clear();
            window.mouse_dispatch_scratch = dispatch;
        }
        Some(default_prevented)
    }

    /// Dispatch one raw key event through the retained root-to-focus capture path and reverse
    /// bubble path. `None` means a callback exited the runtime; otherwise the result reports
    /// whether any listener prevented QuickGUI's key-down default behavior.
    fn invoke_key_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        event: KeyListenerEvent,
    ) -> Option<bool> {
        let Some(window) = &mut self.window else {
            return Some(false);
        };
        let path = window.ui.focus_path();
        let mut dispatch = std::mem::take(&mut window.key_dispatch_scratch);
        window
            .ui
            .collect_key_dispatch(&path, event.kind(), &mut dispatch);

        let mut default_prevented = false;
        for binding in dispatch.iter().copied() {
            let listener = self
                .window
                .as_ref()
                .and_then(|window| window.listeners.key_listener(binding.key));
            let Some(listener) = listener else {
                continue;
            };
            let mut cx = self.event_context();
            if let Some(window) = &mut self.window {
                listener(window.view.as_any_mut(), &event, &mut cx);
            }
            let stop_propagation = cx.stop_event_propagation;
            default_prevented |= cx.prevent_default;
            if !self.apply_event_context(event_loop, cx, false, true) {
                return None;
            }
            if stop_propagation {
                break;
            }
        }

        dispatch.clear();
        if let Some(window) = &mut self.window {
            window.key_dispatch_scratch = dispatch;
        }
        Some(default_prevented)
    }

    fn invoke_pending_mouse_hover(&mut self, event_loop: &ActiveEventLoop) -> bool {
        let mut changes = {
            let Some(window) = &mut self.window else {
                return false;
            };
            let mut changes = std::mem::take(&mut window.mouse_hover_changes_scratch);
            window.ui.take_mouse_hover_changes(&mut changes);
            changes
        };
        for change in changes.iter().copied() {
            let listener = self
                .window
                .as_ref()
                .and_then(|window| window.listeners.mouse_listener(change.key));
            let Some(listener) = listener else {
                continue;
            };
            let mut cx = self.event_context();
            if let Some(window) = &mut self.window {
                listener(
                    window.view.as_any_mut(),
                    &MouseListenerEvent::Hover(change.hovered),
                    &mut cx,
                );
            }
            if !self.apply_event_context(event_loop, cx, false, true) {
                return false;
            }
        }
        if let Some(window) = &mut self.window {
            changes.clear();
            window.mouse_hover_changes_scratch = changes;
        }
        true
    }

    /// Dispatch one wheel event from the topmost listener through listening ancestors.
    ///
    /// `None` means event processing closed the runtime. `Some(true)` means at least one listener
    /// prevented retained default scrolling.
    fn invoke_scroll_wheel(
        &mut self,
        event_loop: &ActiveEventLoop,
        target: ElementId,
        event: ScrollWheelEvent,
    ) -> Option<bool> {
        let mut current = Some(target);
        let mut default_prevented = false;
        while let Some(id) = current {
            let listener = self
                .window
                .as_ref()
                .and_then(|window| window.listeners.scroll_wheels.get(&id).cloned());
            if let Some(listener) = listener {
                let mut cx = self.event_context();
                if let Some(window) = &mut self.window {
                    listener(window.view.as_any_mut(), &event, &mut cx);
                }
                let stop_propagation = cx.stop_event_propagation;
                default_prevented |= cx.prevent_default;
                if !self.apply_event_context(event_loop, cx, false, true) {
                    return None;
                }
                if stop_propagation {
                    break;
                }
            }
            current = self
                .window
                .as_ref()
                .and_then(|window| window.ui.parent_scroll_wheel_listener(id));
        }
        Some(default_prevented)
    }

    fn invoke_touch(
        &mut self,
        event_loop: &ActiveEventLoop,
        target: Option<ElementId>,
        event: TouchEvent,
    ) -> bool {
        let mut current = target;
        while let Some(id) = current {
            let listener = self
                .window
                .as_ref()
                .and_then(|window| window.listeners.touches.get(&id).cloned());
            if let Some(listener) = listener {
                let mut cx = self.event_context();
                if let Some(window) = &mut self.window {
                    listener(window.view.as_any_mut(), &event, &mut cx);
                }
                let stop_propagation = cx.stop_event_propagation;
                if !self.apply_event_context(event_loop, cx, false, true) {
                    return false;
                }
                if stop_propagation {
                    break;
                }
            }
            current = self
                .window
                .as_ref()
                .and_then(|window| window.ui.parent_touch_listener(id));
        }
        self.dispatch(event_loop, Event::Touch(event), false)
    }

    fn invoke_mouse_pressure(
        &mut self,
        event_loop: &ActiveEventLoop,
        target: Option<ElementId>,
        event: MousePressureEvent,
    ) -> bool {
        let listener = target.and_then(|target| {
            self.window
                .as_ref()
                .and_then(|window| window.listeners.mouse_pressures.get(&target).cloned())
        });
        if let Some(listener) = listener {
            let mut cx = self.event_context();
            if let Some(window) = &mut self.window {
                listener(window.view.as_any_mut(), &event, &mut cx);
            }
            if !self.apply_event_context(event_loop, cx, false, true) {
                return false;
            }
        }
        self.dispatch(event_loop, Event::MousePressure(event), false)
    }

    fn invoke_pinch(
        &mut self,
        event_loop: &ActiveEventLoop,
        target: Option<ElementId>,
        event: PinchEvent,
    ) -> bool {
        let listener = target.and_then(|target| {
            self.window
                .as_ref()
                .and_then(|window| window.listeners.pinches.get(&target).cloned())
        });
        if let Some(listener) = listener {
            let mut cx = self.event_context();
            if let Some(window) = &mut self.window {
                listener(window.view.as_any_mut(), &event, &mut cx);
            }
            if !self.apply_event_context(event_loop, cx, false, true) {
                return false;
            }
        }
        self.dispatch(event_loop, Event::Pinch(event), false)
    }

    fn invoke_rotation(
        &mut self,
        event_loop: &ActiveEventLoop,
        target: Option<ElementId>,
        event: RotationEvent,
    ) -> bool {
        let listener = target.and_then(|target| {
            self.window
                .as_ref()
                .and_then(|window| window.listeners.rotations.get(&target).cloned())
        });
        if let Some(listener) = listener {
            let mut cx = self.event_context();
            if let Some(window) = &mut self.window {
                listener(window.view.as_any_mut(), &event, &mut cx);
            }
            if !self.apply_event_context(event_loop, cx, false, true) {
                return false;
            }
        }
        self.dispatch(event_loop, Event::Rotation(event), false)
    }

    fn invoke_smart_magnify(
        &mut self,
        event_loop: &ActiveEventLoop,
        target: Option<ElementId>,
        event: SmartMagnifyEvent,
    ) -> bool {
        let listener = target.and_then(|target| {
            self.window
                .as_ref()
                .and_then(|window| window.listeners.smart_magnifies.get(&target).cloned())
        });
        if let Some(listener) = listener {
            let mut cx = self.event_context();
            if let Some(window) = &mut self.window {
                listener(window.view.as_any_mut(), &event, &mut cx);
            }
            if !self.apply_event_context(event_loop, cx, false, true) {
                return false;
            }
        }
        self.dispatch(event_loop, Event::SmartMagnify(event), false)
    }

    fn invoke_drag_start(
        &mut self,
        event_loop: &ActiveEventLoop,
        source: ElementId,
        event: DragStartEvent,
    ) -> bool {
        let listener = self
            .window
            .as_ref()
            .and_then(|window| window.listeners.drag_sources.get(&source).cloned());
        let Some((registered_type, listener)) = listener else {
            if let Some(window) = &mut self.window {
                window.drag_candidate = None;
            }
            return true;
        };
        let mut cx = self.event_context();
        let drag = {
            let Some(window) = &mut self.window else {
                return false;
            };
            listener(window.view.as_any_mut(), &event, &mut cx)
        };
        debug_assert_eq!(drag.value_type, registered_type);
        if !self.apply_event_context(event_loop, cx, false, true) {
            return false;
        }
        let Some(window) = &mut self.window else {
            return false;
        };
        let mut preview = drag.preview;
        if let Some(preview) = &mut preview {
            window.image_assets.resolve_tree(preview);
        }
        window.drag_candidate = None;
        window.drag_session = Some(DragSession {
            source,
            position: event.position,
            value: drag.value,
            value_type: drag.value_type,
            external_payload: drag.external_payload,
        });
        let changed = window.ui.begin_drag(source);
        // Preview installation is independent of the retained application view and therefore does
        // not make each drag move rebuild `View::render`.
        let preview_changed = match window.ui.set_drag_preview(
            preview,
            source,
            event.origin,
            event.position,
            drag.cursor_offset,
            &mut window.renderer,
            Instant::now(),
        ) {
            Ok(changed) => changed,
            Err(error) => {
                self.fail(event_loop, AppError::View(error.to_string()));
                return false;
            }
        };
        if (changed || preview_changed) && window.scheduler.invalidate() {
            window.window.request_redraw();
        }
        true
    }

    fn invoke_drop(
        &mut self,
        event_loop: &ActiveEventLoop,
        target: ElementId,
        value_type: TypeId,
        value: &dyn Any,
        event: DropEvent,
    ) -> bool {
        let listener = self
            .window
            .as_ref()
            .and_then(|window| window.listeners.drops.get(&(target, value_type)).cloned());
        let Some(listener) = listener else {
            return true;
        };
        let mut cx = self.event_context();
        if let Some(window) = &mut self.window {
            listener(window.view.as_any_mut(), value, &event, &mut cx);
        }
        self.apply_event_context(event_loop, cx, false, true)
    }

    fn compatible_drop_target(
        &self,
        point: Point,
        value_type: TypeId,
        value: &dyn Any,
    ) -> Option<ElementId> {
        let window = self.window.as_ref()?;
        window.ui.drop_target_at(point, |id| {
            window.listeners.drops.contains_key(&(id, value_type))
                && window.ui.can_drop(id, value_type, value)
        })
    }

    fn update_drag_target(&mut self, point: Point) -> bool {
        let active = self.window.as_ref().and_then(|window| {
            if let Some(drag) = &window.drag_session {
                return Some((drag.value_type, Arc::clone(&drag.value)));
            }
            let files = window.native_file_drag.as_ref()?.hover_value.as_ref()?;
            let files: Arc<DroppedFiles> = Arc::clone(files);
            let value: Arc<dyn Any> = files;
            Some((TypeId::of::<DroppedFiles>(), value))
        });
        let Some((value_type, value)) = active else {
            return false;
        };
        let target = self.compatible_drop_target(point, value_type, value.as_ref());
        self.window
            .as_mut()
            .is_some_and(|window| window.ui.set_drag_over(target))
    }

    fn finish_internal_drag(&mut self, event_loop: &ActiveEventLoop, position: Point) -> bool {
        let target = self
            .window
            .as_ref()
            .and_then(|window| window.drag_session.as_ref())
            .and_then(|drag| {
                self.compatible_drop_target(position, drag.value_type, drag.value.as_ref())
            });
        let Some(window) = &mut self.window else {
            return false;
        };
        let Some(drag) = window.drag_session.take() else {
            return false;
        };
        #[cfg(target_os = "macos")]
        if let Some(monitor) = &window.external_drag_monitor {
            monitor.disarm();
        }
        window.drag_candidate = None;
        let repaint = window.ui.end_drag() | window.ui.clear_drag_preview();
        if window.cursor != CursorIcon::Default {
            window.cursor = CursorIcon::Default;
            window.window.set_cursor(CursorIcon::Default);
        }
        if repaint && window.scheduler.invalidate() {
            window.window.request_redraw();
        }
        if let Some(target) = target {
            return self.invoke_drop(
                event_loop,
                target,
                drag.value_type,
                drag.value.as_ref(),
                DropEvent {
                    position,
                    modifiers: self.modifiers,
                    origin: DragOrigin::Internal(drag.source),
                },
            );
        }
        true
    }

    fn cancel_internal_drag(&mut self) -> bool {
        let Some(window) = &mut self.window else {
            return false;
        };
        let had_drag = window.drag_session.take().is_some();
        let had_candidate = window.drag_candidate.take().is_some();
        #[cfg(target_os = "macos")]
        if let Some(monitor) = &window.external_drag_monitor {
            monitor.disarm();
        }
        let repaint = window.ui.end_drag() | window.ui.clear_drag_preview();
        if window.cursor != CursorIcon::Default {
            window.cursor = CursorIcon::Default;
            window.window.set_cursor(CursorIcon::Default);
        }
        if repaint && window.scheduler.invalidate() {
            window.window.request_redraw();
        }
        had_drag || had_candidate
    }

    #[cfg(target_os = "macos")]
    fn arm_external_drag_monitor(&mut self) {
        let Some(handle) = self.current_handle() else {
            return;
        };
        let proxy = self.event_proxy.clone();
        let Some(window) = &mut self.window else {
            return;
        };
        if window.external_drag_monitor.is_none() {
            match MacExternalDragMonitor::new(&window.window, handle, proxy) {
                Ok(monitor) => window.external_drag_monitor = Some(monitor),
                Err(error) => {
                    tracing::warn!(%error, "could not install the AppKit drag boundary monitor");
                }
            }
        }
        if let Some(monitor) = &window.external_drag_monitor {
            monitor.arm();
        }
    }

    #[cfg(target_os = "macos")]
    fn promote_external_drag_at_boundary(
        &mut self,
        event_loop: &ActiveEventLoop,
        point: Point,
    ) -> Option<bool> {
        if let Some(state) = &mut self.window {
            state.pointer = Some(point);
        }
        let boundary_start = self.window.as_ref().and_then(|state| {
            let candidate = state.drag_candidate?;
            if state.drag_session.is_some() || state.pointer_capture.is_some() {
                return None;
            }
            let delta = point - candidate.origin;
            (delta.x.abs().max(delta.y.abs()) >= DRAG_THRESHOLD)
                .then_some((candidate.source, candidate.origin))
        });
        if let Some((source, origin)) = boundary_start
            && !self.invoke_drag_start(
                event_loop,
                source,
                DragStartEvent {
                    origin,
                    position: point,
                    modifiers: self.modifiers,
                },
            )
        {
            return None;
        }
        let promoted = self.promote_external_drag();
        if promoted {
            if let Some(state) = &mut self.window {
                state.pointer = None;
            }
            self.dispatch(event_loop, Event::PointerLeft, false);
        }
        Some(promoted)
    }

    #[cfg(target_os = "macos")]
    fn promote_external_drag(&mut self) -> bool {
        let Some(handle) = self.current_handle() else {
            return false;
        };
        let typed_registry = self.native_drag_registry.clone();
        let proxy = self.event_proxy.clone();
        let Some(state) = &mut self.window else {
            return false;
        };
        let Some(drag) = &mut state.drag_session else {
            return false;
        };
        let Some(mouse_down) = state.external_drag_mouse_down.clone() else {
            if let Some(monitor) = &state.external_drag_monitor {
                monitor.disarm();
            }
            tracing::warn!(
                "could not promote the internal drag because AppKit did not expose its mouse-down event"
            );
            return false;
        };
        let payload = drag.external_payload.take();
        let typed_payload = MacTypedDragPayload::new(
            Arc::clone(&drag.value),
            drag.value_type,
            handle,
            drag.source,
        );
        let window = Arc::clone(&state.window);

        let native = match start_external_drag(
            &window,
            &mouse_down,
            payload.as_ref(),
            typed_payload,
            &typed_registry,
            handle,
            proxy,
        ) {
            Ok(native) => native,
            Err(error) => {
                if let Some(drag) = self
                    .window
                    .as_mut()
                    .and_then(|window| window.drag_session.as_mut())
                {
                    drag.external_payload = payload;
                }
                if let Some(monitor) = self
                    .window
                    .as_ref()
                    .and_then(|window| window.external_drag_monitor.as_ref())
                {
                    monitor.disarm();
                }
                tracing::warn!(%error, "could not promote the internal drag to AppKit");
                return false;
            }
        };

        let Some(state) = &mut self.window else {
            return false;
        };
        let Some(drag) = state.drag_session.take() else {
            return false;
        };
        state.drag_candidate = None;
        state.external_drag_mouse_down = None;
        if let Some(monitor) = &state.external_drag_monitor {
            monitor.disarm();
        }
        state.suppress_external_drag_release = true;
        state.outbound_external_drag = Some(OutboundExternalDrag {
            source: drag.source,
            _session: native,
        });
        let repaint = state.ui.end_drag()
            | state.ui.clear_drag_preview()
            | state.ui.pointer_left()
            | state.ui.set_drag_over(None);
        if state.cursor != CursorIcon::Default {
            state.cursor = CursorIcon::Default;
            state.window.set_cursor(CursorIcon::Default);
        }
        if repaint && state.scheduler.invalidate() {
            state.window.request_redraw();
        }
        true
    }

    fn refresh_native_file_pointer(&mut self) {
        #[cfg(target_os = "macos")]
        if let Some(window) = &mut self.window
            && let Some(point) = current_pointer_position(&window.window)
        {
            window.pointer = Some(point);
        }
    }

    #[cfg(target_os = "macos")]
    fn compatible_native_offer(
        &self,
        point: Point,
        offer: &MacNativeDropOffer,
    ) -> Option<(ElementId, usize)> {
        let window = self.window.as_ref()?;
        window
            .ui
            .drop_offer_target_at(point, offer.iter(), |id, value_type, value| {
                window.listeners.drops.contains_key(&(id, value_type))
                    && window.ui.can_drop(id, value_type, value)
            })
    }

    #[cfg(target_os = "macos")]
    fn hover_native_offer(&mut self, offer: MacNativeDropOffer, point: Point) -> bool {
        // A platform-owned drag supersedes any unpromoted local pointer gesture.
        if self
            .window
            .as_ref()
            .is_none_or(|window| window.native_external_drag.is_none())
        {
            self.cancel_internal_drag();
        }
        let target = self
            .compatible_native_offer(point, &offer)
            .map(|(target, _)| target);
        let Some(window) = &mut self.window else {
            return false;
        };
        window.pointer = Some(point);
        window.native_external_drag = Some(offer);
        let repaint = window.ui.begin_external_drag() | window.ui.set_drag_over(target);
        if repaint && window.scheduler.invalidate() {
            window.window.request_redraw();
        }
        true
    }

    #[cfg(target_os = "macos")]
    fn drop_native_offer(
        &mut self,
        event_loop: &ActiveEventLoop,
        offer: MacNativeDropOffer,
        point: Point,
    ) -> bool {
        let destination = self.current_handle();
        let target = self.compatible_native_offer(point, &offer);
        let Some(window) = &mut self.window else {
            return false;
        };
        window.pointer = Some(point);
        window.native_external_drag = None;
        let repaint = window.ui.end_drag();
        if repaint && window.scheduler.invalidate() {
            window.window.request_redraw();
        }
        if let Some((target, payload_index)) = target
            && let Some(payload) = offer.payload(payload_index)
        {
            let origin = native_drop_origin(destination, payload);
            return self.invoke_drop(
                event_loop,
                target,
                payload.value_type(),
                payload.value(),
                DropEvent {
                    position: point,
                    modifiers: self.modifiers,
                    origin,
                },
            );
        }
        true
    }

    #[cfg(target_os = "macos")]
    fn cancel_native_payload(&mut self) -> bool {
        let Some(window) = &mut self.window else {
            return false;
        };
        if window.native_external_drag.take().is_none() {
            return true;
        }
        window.pointer = None;
        let repaint = window.ui.end_drag();
        if repaint && window.scheduler.invalidate() {
            window.window.request_redraw();
        }
        true
    }

    #[cfg(target_os = "macos")]
    fn handle_native_drop_pending(
        &mut self,
        event_loop: &ActiveEventLoop,
        pending: MacNativeDropPending,
    ) -> bool {
        match pending {
            MacNativeDropPending::Hover { offer, point } => self.hover_native_offer(offer, point),
            MacNativeDropPending::Exit => self.cancel_native_payload(),
            MacNativeDropPending::Drop { offer, point } => {
                self.drop_native_offer(event_loop, offer, point)
            }
        }
    }

    fn hover_native_file(&mut self, path: PathBuf) -> bool {
        // A platform-owned drag supersedes a local pointer gesture.
        self.cancel_internal_drag();
        {
            let Some(window) = &mut self.window else {
                return false;
            };
            let drag = window.native_file_drag.get_or_insert_with(Default::default);
            drag.hover(path);
            let repaint = window.ui.begin_external_drag();
            if repaint && window.scheduler.invalidate() {
                window.window.request_redraw();
            }
        }
        true
    }

    fn flush_native_file_hover(&mut self, event_loop: &ActiveEventLoop) -> bool {
        let files = self
            .window
            .as_mut()
            .and_then(|window| window.native_file_drag.as_mut())
            .and_then(NativeFileDrag::take_hovered_files);
        let Some(files) = files else {
            return true;
        };
        if let Some(window) = &mut self.window
            && let Some(drag) = &mut window.native_file_drag
        {
            drag.hover_value = Some(Arc::new(files.clone()));
        }
        if let Some(point) = self.window.as_ref().and_then(|window| window.pointer) {
            let changed = self.update_drag_target(point);
            if changed
                && let Some(window) = &mut self.window
                && window.scheduler.invalidate()
            {
                window.window.request_redraw();
            }
        }
        self.dispatch(event_loop, Event::FilesHovered(files), false)
    }

    fn drop_native_file(&mut self, event_loop: &ActiveEventLoop, path: PathBuf) -> bool {
        if !self.flush_native_file_hover(event_loop) {
            return false;
        }
        let complete = {
            let Some(window) = &mut self.window else {
                return false;
            };
            window
                .native_file_drag
                .get_or_insert_with(Default::default)
                .drop_path(path)
        };
        if !complete {
            return true;
        }
        let position = self
            .window
            .as_ref()
            .and_then(|window| window.pointer)
            .unwrap_or(Point::ZERO);
        let files = {
            let Some(window) = &mut self.window else {
                return false;
            };
            let files = window
                .native_file_drag
                .take()
                .expect("native file drag was created above")
                .into_dropped_files();
            let repaint = window.ui.end_drag();
            if repaint && window.scheduler.invalidate() {
                window.window.request_redraw();
            }
            files
        };
        let target = self.compatible_drop_target(position, TypeId::of::<DroppedFiles>(), &files);
        if let Some(target) = target
            && !self.invoke_drop(
                event_loop,
                target,
                TypeId::of::<DroppedFiles>(),
                &files,
                DropEvent {
                    position,
                    modifiers: self.modifiers,
                    origin: DragOrigin::External,
                },
            )
        {
            return false;
        }
        self.dispatch(event_loop, Event::FilesDropped(files), false)
    }

    fn cancel_native_file_hover(&mut self, event_loop: &ActiveEventLoop) -> bool {
        if !self.flush_native_file_hover(event_loop) {
            return false;
        }
        let Some(window) = &mut self.window else {
            return false;
        };
        if window.native_file_drag.take().is_none() {
            return true;
        }
        let repaint = window.ui.end_drag();
        if repaint && window.scheduler.invalidate() {
            window.window.request_redraw();
        }
        self.dispatch(event_loop, Event::FilesHoverCancelled, false)
    }

    fn invoke_dismiss(&mut self, event_loop: &ActiveEventLoop, request: DismissRequest) {
        let listener = self
            .window
            .as_ref()
            .and_then(|window| window.listeners.dismisses.get(&request.id).cloned());
        let mut cx = self.event_context();
        if let Some(focus) = request.restore_focus {
            cx.focus = Some(Some(focus));
        }
        if let Some(listener) = listener
            && let Some(window) = &mut self.window
        {
            listener(window.view.as_any_mut(), &mut cx);
        }
        if !self.apply_event_context(event_loop, cx, false, true) {
            return;
        }
        self.dispatch(event_loop, Event::Dismiss(request.id), false);
    }

    fn invoke_input(&mut self, event_loop: &ActiveEventLoop, id: ElementId, value: &str) -> bool {
        let listener = self
            .window
            .as_ref()
            .and_then(|window| window.listeners.inputs.get(&id).cloned());
        if let Some(listener) = listener {
            let mut cx = self.event_context();
            if let Some(window) = &mut self.window {
                listener(window.view.as_any_mut(), value, &mut cx);
            }
            return self.apply_event_context(event_loop, cx, false, true);
        }
        true
    }

    fn invoke_form_submission(
        &mut self,
        event_loop: &ActiveEventLoop,
        form: ElementId,
        trigger: Option<ElementId>,
    ) -> bool {
        if self.form_submission_depth >= MAX_NESTED_FORM_SUBMISSIONS {
            tracing::warn!(?form, "nested form-submission limit reached");
            return true;
        }
        self.form_submission_depth += 1;
        let result = self.invoke_form_submission_inner(event_loop, form, trigger);
        self.form_submission_depth -= 1;
        result
    }

    fn invoke_form_submission_inner(
        &mut self,
        event_loop: &ActiveEventLoop,
        form: ElementId,
        trigger: Option<ElementId>,
    ) -> bool {
        let previous_focus = self.window.as_ref().and_then(|window| window.ui.focused());
        let attempt = self
            .window
            .as_mut()
            .and_then(|window| window.ui.attempt_form_submission(form, trigger));
        let Some(attempt) = attempt else {
            return true;
        };

        // Every attempt changes the accessibility tree: invalid attempts replace the assertive
        // live node, while valid attempts remove the preceding report. This is one action-driven
        // frame and never creates a continuous animation or polling loop.
        if let Some(window) = &mut self.window
            && window.scheduler.invalidate()
        {
            window.window.request_redraw();
        }
        self.announce_focus_change(event_loop, previous_focus);

        let mut cx = self.event_context();
        match attempt {
            FormAttempt::Valid(event) => {
                let listener = self
                    .window
                    .as_ref()
                    .and_then(|window| window.listeners.form_submits.get(&form).cloned());
                if let (Some(window), Some(listener)) = (&mut self.window, listener) {
                    listener(window.view.as_any_mut(), &event, &mut cx);
                }
            }
            FormAttempt::Invalid(report) => {
                let listener = self
                    .window
                    .as_ref()
                    .and_then(|window| window.listeners.form_invalids.get(&form).cloned());
                if let (Some(window), Some(listener)) = (&mut self.window, listener) {
                    listener(window.view.as_any_mut(), &report, &mut cx);
                }
            }
        }
        self.apply_event_context(event_loop, cx, false, true)
    }

    /// Returns whether the focused input owns a submit listener, including when invalid.
    fn submit_focused_input(&mut self, event_loop: &ActiveEventLoop) -> bool {
        let Some((id, value, invalid, form)) = self.window.as_ref().and_then(|window| {
            let id = window.ui.focused_text_input()?;
            let value = window.ui.focused_text_input_value()?;
            Some((
                id,
                value,
                window.ui.focused_text_input_is_invalid(),
                window.ui.form_for_control(id),
            ))
        }) else {
            return false;
        };
        if let Some(form) = form {
            self.invoke_form_submission(event_loop, form, Some(id));
            return true;
        }
        let listener = self
            .window
            .as_ref()
            .and_then(|window| window.listeners.submits.get(&id).cloned());
        let Some(listener) = listener else {
            return false;
        };
        if invalid {
            return true;
        }

        let mut cx = self.event_context();
        if let Some(window) = &mut self.window {
            listener(window.view.as_any_mut(), &value, &mut cx);
        }
        self.apply_event_context(event_loop, cx, false, true);
        true
    }

    fn apply_input_result(
        &mut self,
        event_loop: &ActiveEventLoop,
        result: InputResult,
        notify_listener: bool,
    ) -> bool {
        if result.repaint
            && let Some(state) = &mut self.window
            && state.scheduler.invalidate()
        {
            state.window.request_redraw();
        }
        if notify_listener && let Some(change) = result.change {
            return self.invoke_input(event_loop, change.id, &change.value);
        }
        true
    }

    fn write_clipboard_text(&self, text: &str) -> bool {
        ClipboardItem::new_string(text)
            .and_then(|item| self.clipboard.write(ClipboardTarget::General, item))
            .is_ok()
    }

    fn read_clipboard_text(&self) -> Option<String> {
        self.clipboard
            .read(ClipboardTarget::General)
            .ok()
            .flatten()
            .and_then(|item| item.text())
    }

    fn handle_static_text_key(
        &mut self,
        event_loop: &ActiveEventLoop,
        key: &Key,
        modifiers: Modifiers,
        repeat: bool,
    ) -> bool {
        if self
            .window
            .as_ref()
            .and_then(|window| window.ui.focused_text_input())
            .is_some()
        {
            return false;
        }
        if primary_modifier(modifiers)
            && matches!(key, Key::Character(value) if value.eq_ignore_ascii_case("c"))
        {
            return self.invoke_os_action(event_loop, OsAction::Copy);
        }
        if primary_modifier(modifiers)
            && matches!(key, Key::Character(value) if value.eq_ignore_ascii_case("a"))
        {
            return self.invoke_os_action(event_loop, OsAction::SelectAll);
        }
        if !repeat && matches!(key, Key::Escape) {
            let repaint = self
                .window
                .as_mut()
                .is_some_and(|window| window.ui.clear_static_text_selection());
            if repaint
                && let Some(window) = &mut self.window
                && window.scheduler.invalidate()
            {
                window.window.request_redraw();
            }
            return repaint;
        }
        false
    }

    fn handle_text_input_key(
        &mut self,
        event_loop: &ActiveEventLoop,
        key: &Key,
        modifiers: Modifiers,
        repeat: bool,
    ) -> bool {
        let focused = self
            .window
            .as_ref()
            .and_then(|window| window.ui.focused_text_input());
        if focused.is_none() {
            return false;
        }

        let extend = modifiers.contains(Modifiers::SHIFT);
        let primary = primary_modifier(modifiers);
        let word = word_modifier(modifiers) && (!cfg!(target_os = "macos") || !primary);
        let multiline = self
            .window
            .as_ref()
            .is_some_and(|window| window.ui.focused_text_input_is_multiline());
        if matches!(key, Key::Enter)
            && !multiline
            && !repeat
            && self.submit_focused_input(event_loop)
        {
            return true;
        }
        let result = match key {
            Key::ArrowLeft if cfg!(target_os = "macos") && primary => self
                .window
                .as_mut()
                .map(|window| window.ui.input_move_line_start(extend)),
            Key::ArrowRight if cfg!(target_os = "macos") && primary => self
                .window
                .as_mut()
                .map(|window| window.ui.input_move_line_end(extend)),
            Key::ArrowLeft if word => self
                .window
                .as_mut()
                .map(|window| window.ui.input_move_word_left(extend)),
            Key::ArrowRight if word => self
                .window
                .as_mut()
                .map(|window| window.ui.input_move_word_right(extend)),
            Key::ArrowLeft => self
                .window
                .as_mut()
                .map(|window| window.ui.input_move_left(extend)),
            Key::ArrowRight => self
                .window
                .as_mut()
                .map(|window| window.ui.input_move_right(extend)),
            Key::ArrowUp if cfg!(target_os = "macos") && primary => self
                .window
                .as_mut()
                .map(|window| window.ui.input_move_home(extend)),
            Key::ArrowDown if cfg!(target_os = "macos") && primary => self
                .window
                .as_mut()
                .map(|window| window.ui.input_move_end(extend)),
            Key::ArrowUp if multiline => self.window.as_mut().map(|window| {
                let RuntimeWindow { ui, renderer, .. } = window;
                ui.input_move_vertical(-1, extend, renderer)
            }),
            Key::ArrowDown if multiline => self.window.as_mut().map(|window| {
                let RuntimeWindow { ui, renderer, .. } = window;
                ui.input_move_vertical(1, extend, renderer)
            }),
            Key::PageUp if multiline => self.window.as_mut().map(|window| {
                let RuntimeWindow { ui, renderer, .. } = window;
                ui.input_move_page(-1, extend, renderer)
            }),
            Key::PageDown if multiline => self.window.as_mut().map(|window| {
                let RuntimeWindow { ui, renderer, .. } = window;
                ui.input_move_page(1, extend, renderer)
            }),
            Key::Home if !cfg!(target_os = "macos") && primary => self
                .window
                .as_mut()
                .map(|window| window.ui.input_move_home(extend)),
            Key::End if !cfg!(target_os = "macos") && primary => self
                .window
                .as_mut()
                .map(|window| window.ui.input_move_end(extend)),
            Key::Home => self
                .window
                .as_mut()
                .map(|window| window.ui.input_move_line_start(extend)),
            Key::End => self
                .window
                .as_mut()
                .map(|window| window.ui.input_move_line_end(extend)),
            Key::Backspace if cfg!(target_os = "macos") && primary => self
                .window
                .as_mut()
                .map(|window| window.ui.input_delete_to_line_start()),
            Key::Delete if cfg!(target_os = "macos") && primary => self
                .window
                .as_mut()
                .map(|window| window.ui.input_delete_to_line_end()),
            Key::Backspace if word => self
                .window
                .as_mut()
                .map(|window| window.ui.input_delete_word_backward()),
            Key::Delete if word => self
                .window
                .as_mut()
                .map(|window| window.ui.input_delete_word_forward()),
            Key::Backspace => self
                .window
                .as_mut()
                .map(|window| window.ui.input_backspace()),
            Key::Delete => self.window.as_mut().map(|window| window.ui.input_delete()),
            Key::Enter if multiline => self
                .window
                .as_mut()
                .map(|window| window.ui.input_insert_newline()),
            Key::Character(value)
                if cfg!(target_os = "macos")
                    && modifiers.contains(Modifiers::CONTROL)
                    && !primary
                    && value.eq_ignore_ascii_case("a") =>
            {
                self.window
                    .as_mut()
                    .map(|window| window.ui.input_move_line_start(extend))
            }
            Key::Character(value)
                if cfg!(target_os = "macos")
                    && modifiers.contains(Modifiers::CONTROL)
                    && !primary
                    && value.eq_ignore_ascii_case("e") =>
            {
                self.window
                    .as_mut()
                    .map(|window| window.ui.input_move_line_end(extend))
            }
            Key::Character(value)
                if primary
                    && modifiers.contains(Modifiers::SHIFT)
                    && value.eq_ignore_ascii_case("z") =>
            {
                self.window.as_mut().map(|window| window.ui.input_redo())
            }
            Key::Character(value) if primary && value.eq_ignore_ascii_case("z") => {
                self.window.as_mut().map(|window| window.ui.input_undo())
            }
            Key::Character(value) if primary && value.eq_ignore_ascii_case("y") => {
                self.window.as_mut().map(|window| window.ui.input_redo())
            }
            Key::Character(value) if primary && value.eq_ignore_ascii_case("a") => self
                .window
                .as_mut()
                .map(|window| window.ui.input_select_all()),
            Key::Character(value) if primary && value.eq_ignore_ascii_case("c") => {
                let selected = self
                    .window
                    .as_ref()
                    .and_then(|window| window.ui.selected_input_text());
                if let Some(selected) = selected {
                    let _ = self.write_clipboard_text(selected.as_ref());
                }
                return true;
            }
            Key::Character(value) if primary && value.eq_ignore_ascii_case("x") => {
                let selected = self
                    .window
                    .as_ref()
                    .and_then(|window| window.ui.selected_input_text());
                let copied =
                    selected.is_some_and(|selected| self.write_clipboard_text(selected.as_ref()));
                if !copied {
                    return true;
                }
                self.window
                    .as_mut()
                    .map(|window| window.ui.input_backspace())
            }
            Key::Character(value) if primary && value.eq_ignore_ascii_case("v") => {
                let pasted = self.read_clipboard_text();
                pasted.and_then(|value| {
                    self.window
                        .as_mut()
                        .map(|window| window.ui.input_replace(&value))
                })
            }
            _ => return false,
        };

        if let Some(result) = result {
            self.apply_input_result(event_loop, result, true);
        }
        true
    }

    fn dispatch_binding_actions(
        &mut self,
        event_loop: &ActiveEventLoop,
        bindings: &[KeyBinding],
    ) -> Option<bool> {
        for binding in bindings {
            match self.invoke_action(event_loop, binding.action()) {
                None => return None,
                Some(true) => return Some(true),
                Some(false) => {}
            }
        }
        Some(false)
    }

    fn handle_pressed_key(&mut self, event_loop: &ActiveEventLoop, key: PendingKey) -> bool {
        let focused = self.window.as_ref().and_then(|window| window.ui.focused());
        let mut prefix = self
            .pending_input
            .take()
            .filter(|pending| pending.focus == focused)
            .map(|pending| pending.keys)
            .unwrap_or_default();
        let mut input = prefix.iter().map(PendingKey::keystroke).collect::<Vec<_>>();
        input.push(key.keystroke());
        let contexts = self
            .window
            .as_ref()
            .map(|window| window.ui.key_context_stack())
            .unwrap_or_default();
        let matched = self.keymap.bindings_for_input(&input, &contexts);

        if matched.pending {
            prefix.push(key);
            self.pending_input = Some(PendingInput {
                keys: prefix,
                focus: focused,
                deadline: Instant::now() + self.config.key_sequence_timeout,
            });
            return true;
        }

        if !matched.bindings.is_empty() {
            match self.dispatch_binding_actions(event_loop, &matched.bindings) {
                None => return false,
                Some(true) => return true,
                Some(false) => return self.handle_pressed_key_fallback(event_loop, key),
            }
        }

        if prefix.is_empty() {
            return self.handle_pressed_key_fallback(event_loop, key);
        }
        if !self.replay_pending_keys(event_loop, prefix) {
            return false;
        }
        self.handle_pressed_key(event_loop, key)
    }

    /// Replay timed-out or mismatched prefixes without recursively re-entering key matching.
    fn replay_pending_keys(
        &mut self,
        event_loop: &ActiveEventLoop,
        mut keys: Vec<PendingKey>,
    ) -> bool {
        while !keys.is_empty() {
            let contexts = self
                .window
                .as_ref()
                .map(|window| window.ui.key_context_stack())
                .unwrap_or_default();
            let strokes = keys.iter().map(PendingKey::keystroke).collect::<Vec<_>>();
            let exact_prefix = (1..=strokes.len()).rev().find_map(|length| {
                let matched = self
                    .keymap
                    .bindings_for_input(&strokes[..length], &contexts);
                (!matched.bindings.is_empty()).then_some((length, matched.bindings))
            });

            if let Some((length, bindings)) = exact_prefix {
                let replay_key = keys[length - 1].clone();
                keys.drain(..length);
                match self.dispatch_binding_actions(event_loop, &bindings) {
                    None => return false,
                    Some(true) => {}
                    Some(false) => {
                        if !self.handle_pressed_key_fallback(event_loop, replay_key) {
                            return false;
                        }
                    }
                }
            } else {
                let replay_key = keys.remove(0);
                if !self.handle_pressed_key_fallback(event_loop, replay_key) {
                    return false;
                }
            }
        }
        true
    }

    fn flush_pending_input(&mut self, event_loop: &ActiveEventLoop) -> bool {
        let Some(pending) = self.pending_input.take() else {
            return true;
        };
        let focused = self.window.as_ref().and_then(|window| window.ui.focused());
        if pending.focus != focused {
            return true;
        }
        self.replay_pending_keys(event_loop, pending.keys)
    }

    fn handle_pressed_key_fallback(
        &mut self,
        event_loop: &ActiveEventLoop,
        key_event: PendingKey,
    ) -> bool {
        let PendingKey {
            stroke,
            repeat,
            text,
        } = key_event;
        let Keystroke {
            key,
            modifiers,
            key_char,
        } = stroke;
        let default_prevented = match self.invoke_key_event(
            event_loop,
            KeyListenerEvent::Down(KeyDownEvent {
                key: key.clone(),
                key_char: key_char.clone(),
                text: text.clone(),
                modifiers,
                repeat,
            }),
        ) {
            Some(default_prevented) => default_prevented,
            None => return false,
        };

        if !default_prevented {
            if !repeat
                && matches!(&key, Key::Escape)
                && let Some(request) = self
                    .window
                    .as_ref()
                    .and_then(|window| window.ui.dismiss_topmost())
            {
                self.invoke_dismiss(event_loop, request);
                return true;
            }
            #[cfg(target_os = "macos")]
            if is_default_close_shortcut(&key, modifiers, repeat) {
                if let Some(window) = self.window.as_ref()
                    && let Err(error) = perform_window_close(&window.window)
                {
                    tracing::warn!(%error, "could not perform the default macOS close command");
                }
                return true;
            }

            let mut handled_by_input = self
                .handle_static_text_key(event_loop, &key, modifiers, repeat)
                || self.handle_text_input_key(event_loop, &key, modifiers, repeat);
            if !handled_by_input
                && !modifiers.intersects(Modifiers::CONTROL | Modifiers::SUPER)
                && let Some(text) = text.as_deref().filter(|text| {
                    !text.is_empty() && text.chars().all(|character| !character.is_control())
                })
                && self
                    .window
                    .as_ref()
                    .is_some_and(|window| window.ui.focused_text_input().is_some())
            {
                let result = self
                    .window
                    .as_mut()
                    .map(|window| window.ui.input_replace(text))
                    .unwrap_or_default();
                let changed = result.change.is_some();
                if !self.apply_input_result(event_loop, result, true)
                    || (changed
                        && !self.dispatch(event_loop, Event::TextInput(text.to_owned()), false))
                {
                    return false;
                }
                handled_by_input = true;
            }

            match &key {
                _ if handled_by_input => {}
                Key::Tab => {
                    let previous_focus =
                        self.window.as_ref().and_then(|window| window.ui.focused());
                    if let Some(window) = &mut self.window {
                        window.ui.focus_next(modifiers.contains(Modifiers::SHIFT));
                    }
                    self.announce_focus_change(event_loop, previous_focus);
                }
                Key::ArrowLeft if modifiers.is_empty() => {
                    if !self.navigate_adjacent_tab(event_loop, false, true) {
                        self.activate_adjacent_radio(event_loop, true);
                    }
                }
                Key::ArrowRight if modifiers.is_empty() => {
                    if !self.navigate_adjacent_tab(event_loop, false, false) {
                        self.activate_adjacent_radio(event_loop, false);
                    }
                }
                Key::ArrowUp if modifiers.is_empty() => {
                    if !self.navigate_adjacent_tab(event_loop, true, true) {
                        self.activate_adjacent_radio(event_loop, true);
                    }
                }
                Key::ArrowDown if modifiers.is_empty() => {
                    if !self.navigate_adjacent_tab(event_loop, true, false) {
                        self.activate_adjacent_radio(event_loop, false);
                    }
                }
                Key::Home if modifiers.is_empty() => {
                    self.navigate_edge_tab(event_loop, false);
                }
                Key::End if modifiers.is_empty() => {
                    self.navigate_edge_tab(event_loop, true);
                }
                Key::Enter | Key::Space if !repeat => {
                    let target = self
                        .window
                        .as_ref()
                        .and_then(|window| window.ui.activate_focused());
                    if let Some(id) = target {
                        self.invoke_click(event_loop, id);
                    }
                }
                _ => {}
            }
        }

        self.dispatch(
            event_loop,
            Event::KeyDown {
                key,
                key_char,
                modifiers,
                repeat,
            },
            false,
        )
    }

    fn activate_adjacent_radio(&mut self, event_loop: &ActiveEventLoop, reverse: bool) {
        let previous_focus = self.window.as_ref().and_then(|window| window.ui.focused());
        let target = self
            .window
            .as_ref()
            .and_then(|window| window.ui.adjacent_radio(reverse));
        let Some(target) = target else {
            return;
        };
        if let Some(window) = &mut self.window {
            window.ui.focus(target);
        }
        self.announce_focus_change(event_loop, previous_focus);
        self.invoke_click(event_loop, target);
    }

    fn navigate_adjacent_tab(
        &mut self,
        event_loop: &ActiveEventLoop,
        vertical_axis: bool,
        reverse: bool,
    ) -> bool {
        let target = self
            .window
            .as_ref()
            .and_then(|window| window.ui.adjacent_tab(vertical_axis, reverse));
        self.apply_tab_navigation(event_loop, target)
    }

    fn navigate_edge_tab(&mut self, event_loop: &ActiveEventLoop, last: bool) -> bool {
        let target = self
            .window
            .as_ref()
            .and_then(|window| window.ui.edge_tab(last));
        self.apply_tab_navigation(event_loop, target)
    }

    fn apply_tab_navigation(
        &mut self,
        event_loop: &ActiveEventLoop,
        target: Option<TabNavigationTarget>,
    ) -> bool {
        let Some(target) = target else {
            return false;
        };
        let previous_focus = self.window.as_ref().and_then(|window| window.ui.focused());
        if let Some(window) = &mut self.window {
            window.ui.focus(target.id);
        }
        self.announce_focus_change(event_loop, previous_focus);
        if target.activate {
            self.invoke_click(event_loop, target.id);
        }
        true
    }

    fn announce_focus_change(&mut self, event_loop: &ActiveEventLoop, previous: Option<ElementId>) {
        let focused = self.window.as_ref().and_then(|state| state.ui.focused());
        if previous == focused {
            return;
        }
        self.pending_input = None;
        if let Some(state) = &mut self.window {
            #[cfg(target_os = "macos")]
            if focused.is_some()
                && let Some(host) = &state.native_host
            {
                host.focus_framework();
            }
            if state.scheduler.invalidate() {
                state.window.request_redraw();
            }
        }
        let mut cx = self.event_context();
        if let Some(window) = &mut self.window {
            window.view.event(&Event::FocusChanged(focused), &mut cx);
        }
        self.apply_event_context(event_loop, cx, false, false);
        #[cfg(target_os = "macos")]
        self.sync_native_menu_state();
    }

    fn redraw(&mut self, event_loop: &ActiveEventLoop) {
        let mut mounted_focus_previous = None;
        #[cfg(target_os = "macos")]
        {
            let previous_focus = self.window.as_ref().and_then(|state| state.ui.focused());
            let native_focus_active = self
                .window
                .as_ref()
                .and_then(|state| state.native_host.as_ref())
                .is_some_and(MacNativeHost::native_focus_active);
            if native_focus_active && previous_focus.is_some() {
                if let Some(state) = &mut self.window {
                    state.ui.blur();
                    state.view_dirty = true;
                }
                self.announce_focus_change(event_loop, previous_focus);
            }
        }
        let Some(window_handle) = self.current_handle() else {
            return;
        };
        let Some(window_state) = self.current_window_state() else {
            return;
        };
        let background_tasks = self.background_tasks.clone();
        let foreground_tasks = self.foreground_tasks.clone();
        let globals = self.globals.clone();
        let displays = self.displays.clone();
        let keyboard_layout = self.keyboard.layout().clone();
        let assets = self.assets.clone();
        let app_info = self.app_info.clone();
        let app_paths = self.app_paths.clone();
        let system_info = self.system_info.clone();
        let system_preferences = self.system_preferences;
        let event_proxy = self.event_proxy.clone();
        let Some(state) = &mut self.window else {
            return;
        };
        // `NSViewLayerContentsRedrawDuringViewResize` can make AppKit ask for a draw before
        // Winit's frame-change notification reaches `WindowEvent::Resized`. Always reconcile the
        // retained layout with the drawable's current geometry before building that frame, so a
        // new-size surface never presents text or overlays laid out for the preceding size.
        let physical_size = state.window.inner_size();
        let scale_factor = sane_scale_factor(state.window.scale_factor());
        let logical_size = logical_window_size(physical_size, scale_factor);
        if state.scale_factor != scale_factor || state.logical_size != logical_size {
            state.scale_factor = scale_factor;
            state.logical_size = logical_size;
            state
                .renderer
                .resize(physical_size.width, physical_size.height);
            state.layout_dirty = true;
            state.view_dirty |= state.listeners.observes_viewport;
        }
        state.scheduler.begin_redraw();

        let scroll = state.scheduler.take_scroll();
        let scroll_result = if scroll.is_zero() {
            Default::default()
        } else {
            state.ui.scroll_at(state.pointer, scroll, Instant::now())
        };
        if scroll_result.view_dirty {
            state.view_dirty = true;
        }
        let scroll_for_view = (!scroll.is_zero() && !scroll_result.changed).then_some(scroll);
        if let Some(scroll) = scroll_for_view {
            let mut event_cx = self.event_context();
            if let Some(state) = &mut self.window {
                state.view.event(&Event::Scroll(scroll), &mut event_cx);
            }
            if !self.apply_event_context(event_loop, event_cx, false, true) {
                return;
            }
        }

        let Some(state) = &mut self.window else {
            return;
        };
        let retained_scroll_only = scroll_result.changed
            && !scroll_result.view_dirty
            && !state.view_dirty
            && !state.layout_dirty;
        let retained_layout_only = state.layout_dirty && !state.view_dirty;
        let accessibility_geometry = if retained_layout_only {
            Some(AccessibilityUpdateKind::LayoutGeometry)
        } else if retained_scroll_only {
            Some(AccessibilityUpdateKind::ScrollGeometry)
        } else {
            None
        };
        let started = FrameTimer::start();
        #[cfg(feature = "inspector")]
        let view_rebuilt = state.view_dirty || state.layout_dirty;
        let mut request_animation_frame = false;
        if state.view_dirty {
            let previous_mounted_focus = state.ui.focused();
            let focused_path = state.ui.focus_path();
            let (mut root, requested, repaint_deadline) = state.view.render(
                state.logical_size,
                state.scale_factor,
                state.metrics.current(),
                state.ui.focused(),
                focused_path,
                &mut state.listeners,
                window_handle,
                window_state,
                &displays,
                &keyboard_layout,
                &assets,
                app_info.as_ref(),
                app_paths.as_ref(),
                &system_info,
                &system_preferences,
                Some(&background_tasks),
                &foreground_tasks,
                &globals,
                Some(&event_proxy),
            );
            request_animation_frame = requested;
            state.view_deadline = repaint_deadline;
            state.image_assets.begin_resolve_tree(&mut root);
            let logical_size = state.logical_size;
            let scale_factor = state.scale_factor;
            let image_assets = &mut state.image_assets;
            let ui = &mut state.ui;
            let renderer = &mut state.renderer;
            if let Err(error) =
                ui.set_root_with_prepare(root, logical_size, scale_factor, renderer, |subtree| {
                    image_assets.resolve_subtree(subtree)
                })
            {
                self.fail(event_loop, AppError::View(error.to_string()));
                return;
            }
            let image_resolution_changed = image_assets.finish_resolve_frame();
            request_animation_frame |= image_resolution_changed;
            if let Some(request) = state.pending_focus.take()
                && state.ui.is_focusable(request)
            {
                state.ui.focus(request);
            }
            if previous_mounted_focus != state.ui.focused() {
                mounted_focus_previous = Some(previous_mounted_focus);
            }
            state.layout_dirty = false;
            state.view_dirty = image_resolution_changed;
        } else if state.layout_dirty {
            let logical_size = state.logical_size;
            let scale_factor = state.scale_factor;
            let image_assets = &mut state.image_assets;
            let ui = &mut state.ui;
            let renderer = &mut state.renderer;
            if let Err(error) =
                ui.relayout_with_prepare(logical_size, scale_factor, renderer, |subtree| {
                    image_assets.resolve_subtree(subtree)
                })
            {
                self.fail(event_loop, AppError::View(error.to_string()));
                return;
            }
            request_animation_frame |= image_assets.finish_resolve_frame();
            state.layout_dirty = false;
        }
        let ime_target = state.ui.focused_text_input();
        if ime_target != state.ime_target {
            let previous_target = state.ime_target;
            if let Some(previous_target) = previous_target {
                state.ui.input_cancel_preedit(previous_target);
            }
            state.ime_target = ime_target;
            if previous_target.is_some() != ime_target.is_some() {
                state.window.set_ime_allowed(ime_target.is_some());
            }
        }

        state.ui.advance_animations(Instant::now());
        state.ui.advance_scrollbars(Instant::now());
        state.scene.clear(self.config.background);
        if let Err(error) = state.ui.paint(&mut state.scene, &mut state.renderer) {
            self.fail(event_loop, AppError::View(error.to_string()));
            return;
        }
        #[cfg(feature = "inspector")]
        let retained_hover_pointer = state.pointer.filter(|point| {
            !state
                .inspector
                .as_ref()
                .is_some_and(|inspector| inspector.captures_pointer(*point))
        });
        #[cfg(not(feature = "inspector"))]
        let retained_hover_pointer = state.pointer;
        state.ui.refresh_mouse_hover(retained_hover_pointer);
        // Paint rebuilds the retained hit stack even when only scrolling moved content. Resolve
        // once during this already-damaged frame so a stationary pointer cannot keep the cursor
        // belonging to the element that used to be underneath it.
        if let Some(point) = state.pointer {
            let cursor = desired_cursor(state, point);
            set_cursor_if_changed(state, cursor);
        }
        let variable_list_measurement_update = state.ui.take_variable_list_measurement_update();
        let variable_list_measurements_changed = variable_list_measurement_update.changed;
        let declarative_animation_frame_requested =
            state.ui.declarative_animation_frame_requested();
        let detached_animation_frame_requested = state.ui.detached_animation_frame_requested();
        let style_transition_frame_requested = state.ui.style_transition_frame_requested();
        if variable_list_measurement_update.view_dirty {
            state.view_dirty = true;
        }
        #[cfg(feature = "inspector")]
        if state.inspector.is_some() {
            let metrics = state.metrics.current();
            let viewport = state.logical_size;
            let scale_factor = state.scale_factor;
            let damage = InspectorFrameDamage {
                view_rebuilt,
                retained_scroll_changed: scroll_result.changed,
                variable_measurements_changed: variable_list_measurements_changed,
                animation_requested: request_animation_frame,
                declarative_animation_requested: declarative_animation_frame_requested,
                detached_animation_requested: detached_animation_frame_requested,
                style_transition_requested: style_transition_frame_requested,
            };
            let RuntimeWindow {
                ui,
                inspector,
                scene,
                renderer,
                ..
            } = state;
            let inspector = inspector
                .as_mut()
                .expect("inspector presence checked before split borrow");
            inspector.refresh(ui, metrics, damage, viewport, scale_factor);
            if let Err(error) =
                inspector.paint(scene, renderer, viewport, scale_factor, Instant::now())
            {
                self.fail(event_loop, AppError::View(error.to_string()));
                return;
            }
        }
        #[cfg(target_os = "macos")]
        {
            let text_type = TypeId::of::<ExternalDragText>();
            let url_type = TypeId::of::<ExternalDragUrl>();
            let has_text = state
                .listeners
                .drop_order
                .iter()
                .any(|(_, value_type)| *value_type == text_type);
            let has_url = state
                .listeners
                .drop_order
                .iter()
                .any(|(_, value_type)| *value_type == url_type);
            let has_typed = !state.listeners.drop_order.is_empty();
            let RuntimeWindow {
                native_drop_host,
                ui,
                listeners,
                ..
            } = state;
            native_drop_host.update(has_text, has_url, has_typed, |snapshot| {
                ui.update_external_drop_snapshot(snapshot, &listeners.drop_order);
            });

            let has_native_views = !state.ui.native_views().is_empty();
            if has_native_views && state.native_host.is_none() {
                let host = match MacNativeHost::new(&state.window) {
                    Ok(host) => host,
                    Err(error) => {
                        self.fail(event_loop, AppError::View(error));
                        return;
                    }
                };
                if let Err(error) = state
                    .renderer
                    .enable_native_composition(host.overlay_pointer())
                {
                    self.fail(event_loop, AppError::Render(error.to_string()));
                    return;
                }
                state.native_host = Some(host);
            }
            if let Some(host) = &mut state.native_host {
                if let Err(error) = host.reconcile(state.ui.native_views()) {
                    self.fail(event_loop, AppError::View(error));
                    return;
                }
                let overlay_active = has_native_views
                    && (state.scene.has_content_in_plane(crate::ScenePlane::Overlay)
                        || state.ui.overlay_input_active());
                host.set_overlay_active(overlay_active);
                if let Err(error) = state.renderer.set_native_overlay_active(overlay_active) {
                    self.fail(event_loop, AppError::Render(error.to_string()));
                    return;
                }
            }
            state
                .renderer
                .set_native_composition_active(has_native_views);
            if let Some(guard) = state.first_frame_guard.as_ref() {
                guard.cover();
            }
        }
        let platform_content_attached = runtime_window_content_attached(state);
        if platform_content_attached
            && ime_target.is_some()
            && let Some(caret) = state.ui.ime_cursor_area()
        {
            state.window.set_ime_cursor_area(
                LogicalPosition::new(caret.x as f64, caret.y as f64),
                LogicalSize::new(caret.width.max(1.0) as f64, caret.height.max(1.0) as f64),
            );
        }
        if let Some(update_kind) = state
            .accessibility_updates
            .should_update(accessibility_geometry, Instant::now())
        {
            let window_title = self.config.title.as_str();
            let RuntimeWindow {
                accessibility, ui, ..
            } = state;
            accessibility.update_if_active(|| match update_kind {
                AccessibilityUpdateKind::ScrollGeometry => ui.accessibility_scroll_update(),
                AccessibilityUpdateKind::Full | AccessibilityUpdateKind::LayoutGeometry => {
                    ui.accessibility_update(window_title)
                }
            });
        }

        match state.renderer.render(&state.scene, state.scale_factor) {
            Ok(RenderOutcome::Presented(mut stats)) => {
                #[cfg(target_os = "macos")]
                if state.first_frame_guard.is_some()
                    && let Err(error) = state.renderer.wait_for_submitted_work()
                {
                    self.fail(event_loop, AppError::Render(error.to_string()));
                    return;
                }
                let image_assets = state.image_assets.stats();
                stats.cpu_image_cache_bytes = image_assets.decoded_bytes;
                stats.image_resource_entries = image_assets.entries;
                stats.image_resources_loading = image_assets.loading;
                stats.image_resources_failed = image_assets.failed;
                (stats.animated_images, stats.active_animations) = state.ui.animation_counts();
                state.metrics.record(started.elapsed(), stats);
                #[cfg(target_os = "macos")]
                if let Some(guard) = state.first_frame_guard.take() {
                    guard.reveal();
                }
                if (request_animation_frame
                    || declarative_animation_frame_requested
                    || detached_animation_frame_requested
                    || style_transition_frame_requested
                    || variable_list_measurements_changed)
                    && state.scheduler.invalidate()
                {
                    state.view_dirty |=
                        request_animation_frame || declarative_animation_frame_requested;
                    state.window.request_redraw();
                }
            }
            Ok(RenderOutcome::Retry) => {
                state.view_dirty |=
                    request_animation_frame || declarative_animation_frame_requested;
                if state.scheduler.invalidate() {
                    state.window.request_redraw();
                }
            }
            Ok(RenderOutcome::Occluded) => {
                // Wait for the platform to expose or resize the window; do not spin while hidden.
                // Keep view-owned frame requests dirty so the first exposed frame (including the
                // detached macOS first-present pass) can resume them instead of silently losing
                // an animation that was declared while the surface was unavailable.
                state.view_dirty |=
                    request_animation_frame || declarative_animation_frame_requested;
            }
            Err(error) => self.fail(event_loop, AppError::Render(error.to_string())),
        }
        if !self.invoke_pending_mouse_hover(event_loop) {
            return;
        }
        if let Some(previous) = mounted_focus_previous {
            self.announce_focus_change(event_loop, previous);
        } else {
            #[cfg(target_os = "macos")]
            self.sync_native_menu_state();
        }
    }

    fn create_window(&mut self, event_loop: &ActiveEventLoop, request: WindowRequest) {
        let WindowRequest {
            handle,
            view,
            options,
            parent,
            popover_anchor_element,
        } = request;
        if let Err(error) = validate_window_options(&options) {
            self.fail(event_loop, AppError::Window(error.to_string()));
            return;
        }
        self.config = options;
        self.pending_input = None;
        self.modifiers = Modifiers::default();

        let requested_bounds = self.config.window_bounds;
        let selected_display_id = self
            .config
            .display_id
            .filter(|id| self.displays.find(*id).is_some())
            .or_else(|| self.displays.primary_id());
        let selected_display = selected_display_id
            .and_then(|id| self.displays.find(id))
            .cloned();
        let selected_monitor = selected_display_id
            .and_then(|id| crate::display::native_monitor(event_loop, id))
            .or_else(|| event_loop.primary_monitor());
        let constrained_size = constrained_window_size(
            self.config.size,
            self.config.minimum_size,
            self.config.maximum_size,
        );
        let requested_rect = requested_bounds
            .map(WindowBounds::bounds)
            .unwrap_or_else(|| {
                if self.config.display_id.is_some() {
                    selected_display.as_ref().map_or_else(
                        || Rect::from_size(constrained_size),
                        |display| display.centered_bounds(constrained_size),
                    )
                } else {
                    Rect::from_size(constrained_size)
                }
            });
        let constrained_size = constrained_window_size(
            Size::new(requested_rect.width, requested_rect.height),
            self.config.minimum_size,
            self.config.maximum_size,
        );
        let restore_rect = Rect::new(
            requested_rect.x,
            requested_rect.y,
            constrained_size.width,
            constrained_size.height,
        );
        let parent_window = parent
            .and_then(|parent| self.window_handles.get(&parent).copied())
            .and_then(|window_id| self.windows.get(&window_id))
            .map(|entry| entry.state.window.clone());
        if self.config.kind == WindowKind::SystemPopover && parent_window.is_none() {
            self.fail(
                event_loop,
                AppError::Window(WindowCommandError::PopoverParentRequired.to_string()),
            );
            return;
        }

        #[cfg(target_os = "macos")]
        if self.config.tabbing_identifier.is_some() {
            // Enable AppKit's process-wide automatic tabbing only while at least one QuickGUI
            // window explicitly opts in. Every other window is configured as Disallowed below.
            self.register_native_tabbing(event_loop);
        }

        let mut attributes = Window::default_attributes()
            .with_title(self.config.title.clone())
            .with_visible(false)
            .with_resizable(self.config.is_resizable)
            .with_decorations(self.config.decorated)
            .with_transparent(self.config.window_background.is_transparent())
            .with_blur(self.config.window_background.is_blurred())
            .with_content_protected(self.config.content_protected)
            .with_theme(self.config.preferred_appearance.map(to_winit_theme))
            .with_enabled_buttons(window_buttons(&self.config))
            .with_window_level(effective_window_level(&self.config).to_winit())
            .with_window_icon(self.config.icon.as_ref().map(winit_window_icon))
            .with_inner_size(LogicalSize::new(
                restore_rect.width as f64,
                restore_rect.height as f64,
            ));
        #[cfg(target_os = "windows")]
        {
            attributes = attributes.with_skip_taskbar(self.config.skip_taskbar);
        }
        if (self.config.window_bounds.is_some() || self.config.display_id.is_some())
            && self.config.kind != WindowKind::SystemPopover
        {
            attributes = attributes.with_position(LogicalPosition::new(
                restore_rect.x as f64,
                restore_rect.y as f64,
            ));
        }
        match requested_bounds {
            // Maximization is applied after the hidden concrete window is placed. AppKit can
            // otherwise constrain the initializer frame to the main screen before maximizing.
            Some(WindowBounds::Maximized(_)) => {}
            Some(WindowBounds::Fullscreen(_)) => {
                attributes = attributes
                    .with_fullscreen(Some(Fullscreen::Borderless(selected_monitor.clone())))
            }
            Some(WindowBounds::Windowed(_)) | None => {}
        }
        #[cfg(target_os = "macos")]
        match self.config.title_bar_style {
            TitleBarStyle::Default => {}
            TitleBarStyle::HiddenInset => {
                attributes = attributes
                    .with_titlebar_transparent(true)
                    .with_title_hidden(true)
                    .with_fullsize_content_view(true);
            }
            TitleBarStyle::Hidden => {
                attributes = attributes
                    .with_titlebar_transparent(true)
                    .with_title_hidden(true)
                    .with_titlebar_hidden(true)
                    .with_titlebar_buttons_hidden(true)
                    .with_fullsize_content_view(true);
            }
        }
        #[cfg(target_os = "macos")]
        if let Some(identifier) = self.config.tabbing_identifier.as_deref() {
            attributes = attributes.with_tabbing_identifier(identifier);
        }
        #[cfg(target_os = "macos")]
        if matches!(
            self.config.kind,
            WindowKind::Popover | WindowKind::SystemPopover
        ) {
            attributes = attributes.with_panel(true);
        }
        #[cfg(target_os = "macos")]
        if self.config.kind == WindowKind::SystemPopover {
            attributes = attributes
                .with_titlebar_transparent(true)
                .with_title_hidden(true)
                .with_titlebar_hidden(true)
                .with_titlebar_buttons_hidden(true)
                .with_fullsize_content_view(true);
        }
        #[cfg(not(target_os = "macos"))]
        if let (Some(popover), Some(parent)) =
            (self.config.popover.as_ref(), parent_window.as_ref())
        {
            let local = crate::popover::unconstrained_popover_rect(
                popover.anchor_rect,
                Size::new(restore_rect.width, restore_rect.height),
                popover,
            );
            if let Ok(parent_position) = parent.inner_position() {
                let scale = sane_scale_factor(parent.scale_factor());
                attributes = attributes.with_position(PhysicalPosition::new(
                    parent_position.x + (local.x * scale).round() as i32,
                    parent_position.y + (local.y * scale).round() as i32,
                ));
            }
            if let Ok(parent_handle) = parent.window_handle() {
                // SAFETY: `parent_window` retains the referenced native window through creation,
                // and RuntimeWindow retains the parent handle for the complete child lifetime.
                attributes = unsafe { attributes.with_parent_window(Some(parent_handle.as_raw())) };
            }
            attributes = attributes.with_decorations(false);
        }
        if let Some(minimum) = self.config.minimum_size {
            attributes = attributes.with_min_inner_size(LogicalSize::new(
                minimum.width as f64,
                minimum.height as f64,
            ));
        }
        if let Some(maximum) = self.config.maximum_size {
            attributes = attributes.with_max_inner_size(LogicalSize::new(
                maximum.width as f64,
                maximum.height as f64,
            ));
        }
        #[cfg(target_os = "macos")]
        {
            attributes = attributes.with_has_shadow(self.config.shadow);
        }
        let window = match event_loop.create_window(attributes) {
            Ok(window) => Arc::new(window),
            Err(error) => {
                self.fail(event_loop, AppError::Window(error.to_string()));
                return;
            }
        };
        // AppKit may constrain an initializer-created window to the main screen before Winit has a
        // concrete `NSScreen` for a windowed request. Re-apply explicit global placement while the
        // window is still hidden so a selected secondary display is authoritative on first frame.
        if (self.config.window_bounds.is_some() || self.config.display_id.is_some())
            && self.config.kind != WindowKind::SystemPopover
            && !matches!(requested_bounds, Some(WindowBounds::Fullscreen(_)))
        {
            window.set_outer_position(LogicalPosition::new(
                restore_rect.x as f64,
                restore_rect.y as f64,
            ));
        }
        if matches!(requested_bounds, Some(WindowBounds::Maximized(_))) {
            window.set_maximized(true);
        }
        let window_id = window.id();
        window.set_ime_allowed(false);
        window.set_cursor_visible(self.config.cursor_visible);
        if let Err(error) = window.set_cursor_grab(self.config.cursor_grab.to_winit()) {
            tracing::warn!(%error, "could not apply initial cursor confinement");
        }
        if let Err(error) = window.set_cursor_hittest(self.config.cursor_hit_test) {
            tracing::warn!(%error, "could not apply initial native pointer hit testing");
        }
        if let Some(position) = self.config.cursor_position
            && let Err(error) = window.set_cursor_position(LogicalPosition::new(
                f64::from(position.x),
                f64::from(position.y),
            ))
        {
            tracing::warn!(%error, "could not apply initial native cursor position");
        }
        let appearance = self
            .config
            .preferred_appearance
            .or_else(|| window.theme().map(map_window_appearance))
            .or_else(|| event_loop.system_theme().map(map_window_appearance))
            .unwrap_or_default();
        #[cfg(target_os = "macos")]
        if let Err(error) = configure_window_kind(
            &window,
            self.config.kind,
            self.config.focus,
            self.config
                .popover
                .as_ref()
                .is_none_or(|popover| popover.accepts_key_focus),
        ) {
            self.fail(event_loop, AppError::Platform(error));
            return;
        }
        if let Err(error) = set_window_focusable(&window, self.config.focusable) {
            tracing::warn!(%error, "could not apply native window focusability");
        }
        if let Err(error) = set_window_opacity(&window, self.config.opacity) {
            tracing::warn!(%error, "could not apply native window opacity");
        }
        if let Err(error) = set_window_visible_on_all_workspaces(
            &window,
            effective_visible_on_all_workspaces(&self.config),
        ) {
            tracing::warn!(%error, "could not apply native workspace visibility");
        }
        if self.config.window_level.is_some() {
            window.set_window_level(effective_window_level(&self.config).to_winit());
        }
        #[cfg(target_os = "macos")]
        if (self.config.represented_file.is_some()
            || self.config.document_edited
            || self.config.tabbing_identifier.is_some())
            && let Err(error) = configure_document_window(
                &window,
                self.config.represented_file.as_deref(),
                self.config.document_edited,
                self.config.tabbing_identifier.as_deref(),
            )
        {
            self.fail(event_loop, AppError::Platform(error));
            return;
        }
        #[cfg(target_os = "macos")]
        if self.config.represented_file.is_none()
            && !self.config.document_edited
            && self.config.tabbing_identifier.is_none()
            && let Err(error) = set_window_tabbing_identifier(&window, None)
        {
            self.fail(event_loop, AppError::Platform(error));
            return;
        }
        #[cfg(target_os = "macos")]
        if let Some(popover) = self.config.popover.as_ref()
            && let Err(error) = position_system_popover(
                &window,
                parent_window
                    .as_ref()
                    .expect("system popover parent checked above"),
                popover,
            )
        {
            self.fail(event_loop, AppError::Platform(error));
            return;
        }
        #[cfg(target_os = "macos")]
        if self.menu_host.is_none() {
            let menus = self.config.window_menus.as_deref().unwrap_or(&self.menus);
            self.menu_actions = collect_menu_actions(menus);
            self.menu_host = match MacMenuHost::new(menus, self.event_proxy.clone()) {
                Ok(host) => Some(host),
                Err(error) => {
                    self.fail(event_loop, AppError::Platform(error));
                    return;
                }
            };
        }
        #[cfg(target_os = "windows")]
        let window_menu_host = {
            if self.windows_menu_host.is_none() && !self.menus.is_empty() {
                self.windows_menu_host =
                    match windows_menu::WindowsMenuHost::new(&self.menus, self.event_proxy.clone())
                    {
                        Ok(host) => Some(host),
                        Err(error) => {
                            self.fail(event_loop, AppError::Platform(error));
                            return;
                        }
                    };
            }
            if let Some(menus) = self.config.window_menus.as_deref() {
                let host = if menus.is_empty() {
                    None
                } else {
                    match windows_menu::WindowsMenuHost::new(menus, self.event_proxy.clone()) {
                        Ok(host) => Some(host),
                        Err(error) => {
                            self.fail(event_loop, AppError::Platform(error));
                            return;
                        }
                    }
                };
                if let Some(host) = &host
                    && let Err(error) = host.attach(&window)
                {
                    self.fail(event_loop, AppError::Platform(error));
                    return;
                }
                host
            } else {
                if let Some(host) = &self.windows_menu_host
                    && let Err(error) = host.attach(&window)
                {
                    self.fail(event_loop, AppError::Platform(error));
                    return;
                }
                None
            }
        };
        let accessibility = AccessibilityAdapter::with_event_loop_proxy(
            event_loop,
            &window,
            self.event_proxy.clone(),
        );
        let profile = self.config.performance_profile;
        let shared_gpu = self.gpu_contexts.get(&profile).cloned();
        let renderer = match pollster::block_on(GpuRenderer::new(
            window.clone(),
            event_loop,
            profile,
            self.config.window_background,
            self.font_system.clone(),
            shared_gpu.as_ref(),
        )) {
            Ok(renderer) => renderer,
            Err(error) => {
                self.fail(
                    event_loop,
                    AppError::GraphicsInitialization(error.to_string()),
                );
                return;
            }
        };
        self.gpu_contexts
            .entry(profile)
            .or_insert_with(|| renderer.context());
        #[cfg(target_os = "macos")]
        if let Err(error) = configure_gpu_window_resize(&window) {
            self.fail(event_loop, AppError::Platform(error));
            return;
        }
        #[cfg(target_os = "macos")]
        if let Err(error) = set_window_movable(&window, implicit_native_movable(&self.config)) {
            self.fail(event_loop, AppError::Platform(error));
            return;
        }
        #[cfg(target_os = "macos")]
        if let Some(position) = self.config.traffic_light_position
            && let Err(error) = position_traffic_lights(&window, position)
        {
            self.fail(event_loop, AppError::Platform(error));
            return;
        }
        #[cfg(target_os = "macos")]
        let native_drop_host = match MacNativeDropHost::new(
            &window,
            handle,
            self.event_proxy.clone(),
            self.native_drag_registry.clone(),
        ) {
            Ok(host) => host,
            Err(error) => {
                self.fail(event_loop, AppError::Platform(error));
                return;
            }
        };
        #[cfg(target_os = "macos")]
        let first_frame_guard = match MacFirstFrameGuard::new(&window, self.config.background) {
            Ok(guard) => Some(guard),
            Err(error) => {
                self.fail(event_loop, AppError::Platform(error));
                return;
            }
        };
        let scale_factor = sane_scale_factor(window.scale_factor());
        let physical = window.inner_size();
        let logical_size = logical_window_size(physical, scale_factor);
        let logical_position = logical_window_position(&window, scale_factor)
            .unwrap_or_else(|| Point::new(restore_rect.x, restore_rect.y));
        let display_id = crate::display::display_for_rect(
            &self.displays,
            Rect::new(
                logical_position.x,
                logical_position.y,
                logical_size.width,
                logical_size.height,
            ),
        )
        .or_else(|| {
            window
                .current_monitor()
                .map(|monitor| crate::display::native_display_id(&monitor))
                .filter(|id| self.displays.find(*id).is_some())
        });
        let restore_bounds = requested_bounds.map_or_else(
            || {
                Rect::new(
                    logical_position.x,
                    logical_position.y,
                    logical_size.width,
                    logical_size.height,
                )
            },
            WindowBounds::bounds,
        );
        let maximized = matches!(requested_bounds, Some(WindowBounds::Maximized(_)));
        #[cfg(target_os = "macos")]
        let native_tabs = if self.config.tabbing_identifier.is_some() {
            window_tab_state(&window).unwrap_or_else(|error| {
                tracing::warn!(%error, "could not read initial native window tab state");
                WindowTabState::default()
            })
        } else {
            WindowTabState::default()
        };
        #[cfg(not(target_os = "macos"))]
        let native_tabs = WindowTabState::default();
        let mut scheduler = FrameScheduler::default();
        scheduler.invalidate();
        let reduce_motion = self.config.reduce_motion
            || self
                .system_preferences
                .reduce_motion()
                .is_some_and(|enabled| enabled);
        let mut ui = UiTree::new_at(self.animation_epoch);
        ui.set_reduce_motion(reduce_motion);
        ui.set_animations_enabled(!reduce_motion, Instant::now());
        self.current_window = Some((window_id, handle));
        self.window_handles.insert(handle, window_id);
        if self.active_window.is_none()
            && self.config.show
            && self.config.focus
            && self.config.focusable
        {
            self.note_window_focused(window_id);
        }
        self.window = Some(RuntimeWindow {
            parent,
            restore_focus_on_close: popover_anchor_element,
            view,
            renderer,
            image_assets: ImageAssetCache::new(handle, self.image_workers.clone()),
            #[cfg(target_os = "macos")]
            native_host: None,
            #[cfg(target_os = "macos")]
            native_drop_host,
            #[cfg(target_os = "macos")]
            first_frame_guard,
            #[cfg(target_os = "windows")]
            window_menu_host,
            #[cfg(target_os = "windows")]
            taskbar_state_applied: false,
            #[cfg(target_os = "windows")]
            taskbar_apply_attempts: 0,
            ui,
            #[cfg(feature = "inspector")]
            inspector: self
                .config
                .inspector
                .then(|| InspectorState::new(self.animation_epoch)),
            scheduler,
            scene: Scene::new(),
            metrics: MetricsTracker::default(),
            scale_factor,
            logical_size,
            logical_position,
            display_id,
            appearance,
            native_tabs,
            restore_bounds,
            maximized,
            pointer: None,
            pointer_capture: None,
            pressed_mouse_buttons: PressedMouseButtons::default(),
            mouse_clicks: MouseClickTracker::default(),
            mouse_event_path_scratch: Vec::with_capacity(16),
            mouse_dispatch_scratch: Vec::with_capacity(16),
            mouse_hover_changes_scratch: Vec::with_capacity(8),
            key_dispatch_scratch: Vec::with_capacity(16),
            action_dispatch_scratch: Vec::with_capacity(16),
            touch_captures: HashMap::with_capacity(8),
            drag_candidate: None,
            drag_session: None,
            native_file_drag: None,
            #[cfg(target_os = "macos")]
            native_external_drag: None,
            #[cfg(target_os = "macos")]
            external_drag_mouse_down: None,
            #[cfg(target_os = "macos")]
            external_drag_monitor: None,
            #[cfg(target_os = "macos")]
            outbound_external_drag: None,
            #[cfg(target_os = "macos")]
            suppress_external_drag_release: false,
            cursor: CursorIcon::Default,
            ime_target: None,
            pending_focus: None,
            occluded: false,
            focused: false,
            visible: false,
            relation_presented: false,
            reduce_motion,
            view_dirty: true,
            layout_dirty: true,
            view_deadline: None,
            accessibility_updates: AccessibilityUpdateSchedule::default(),
            listeners: ListenerRegistry::default(),
            accessibility,
            window,
        });
        self.dispatch(
            event_loop,
            Event::Resized {
                logical_size,
                scale_factor,
            },
            false,
        );

        #[cfg(target_os = "macos")]
        {
            // Populate layout, text, scene, and native composition while the window is hidden.
            // This attached preparation pass stops at the expected surface-occlusion boundary.
            self.redraw(event_loop);
            if self.fatal_error.is_some() {
                self.deactivate_window();
                return;
            }
            let detached = match self
                .window
                .as_ref()
                .and_then(|state| state.first_frame_guard.as_ref())
                .map(|guard| guard.detach_content_for_first_present())
                .transpose()
            {
                Ok(detached) => detached,
                Err(error) => {
                    self.fail(event_loop, AppError::Platform(error));
                    self.deactivate_window();
                    return;
                }
            };

            // With the content detached, WGPU can acquire the actual CAMetalLayer drawable even
            // though the NSWindow remains hidden. The renderer completes that real surface frame
            // and removes the shield before RAII reattaches the unchanged content view.
            self.redraw(event_loop);
            drop(detached);
            if self.fatal_error.is_some() {
                self.deactivate_window();
                return;
            }
            if self
                .window
                .as_ref()
                .is_some_and(|state| state.first_frame_guard.is_some())
            {
                self.fail(
                    event_loop,
                    AppError::Render(
                        "the hidden Metal surface did not present its first frame".to_owned(),
                    ),
                );
                self.deactivate_window();
                return;
            }

            if self.config.kind != WindowKind::SystemPopover {
                // GPU initialization gives AppKit and the Dock a complete launch turn while this
                // window remains hidden. Re-read the bounded snapshot now so every ordinary
                // window uses the work area that exists at its actual presentation boundary.
                // Explicit global bounds remain authoritative; only `.display(id)` automatic
                // centering is reconciled, before a single pixel can become visible.
                self.refresh_displays(event_loop);
                if requested_bounds.is_none()
                    && self.config.display_id.is_some()
                    && let Some(target) = self
                        .config
                        .display_id
                        .filter(|id| self.displays.find(*id).is_some())
                        .or_else(|| self.displays.primary_id())
                        .and_then(|id| self.displays.find(id))
                        .cloned()
                    && let Some(state) = self.window.as_mut()
                {
                    let centered = target.centered_bounds(state.logical_size);
                    state.window.set_outer_position(LogicalPosition::new(
                        centered.x as f64,
                        centered.y as f64,
                    ));
                    state.logical_position = Point::new(centered.x, centered.y);
                    state.restore_bounds.x = centered.x;
                    state.restore_bounds.y = centered.y;
                    state.display_id = Some(target.id());
                }
            }
        }

        if self.config.show {
            #[cfg(target_os = "macos")]
            if let Some(popover) = self.config.popover.as_ref()
                && let Some(state) = self.window.as_ref()
                && let Err(error) = position_system_popover(
                    &state.window,
                    parent_window
                        .as_ref()
                        .expect("system popover parent checked above"),
                    popover,
                )
            {
                self.fail(event_loop, AppError::Platform(error));
                self.deactivate_window();
                return;
            }
            #[cfg(target_os = "macos")]
            let relation_presented = match self.window.as_ref() {
                Some(state) => match present_window_relation(
                    &state.window,
                    parent_window.as_ref(),
                    self.config.kind,
                ) {
                    Ok(presented) => presented,
                    Err(error) => {
                        self.fail(event_loop, AppError::Platform(error));
                        self.deactivate_window();
                        return;
                    }
                },
                None => false,
            };
            #[cfg(not(target_os = "macos"))]
            let relation_presented = false;
            #[cfg(target_os = "macos")]
            if window_dismisses_system_popover_on_pointer_outside(&self.config)
                && let Some(popover) = self.config.popover.as_ref()
                && let Some(parent) = parent_window.as_ref()
                && let Some(state) = self.window.as_ref()
                && let Err(error) =
                    self.popover_monitor
                        .watch(handle, &state.window, parent, popover.anchor_rect)
            {
                self.fail(event_loop, AppError::Platform(error));
                self.deactivate_window();
                return;
            }
            #[cfg(target_os = "macos")]
            if let Some(state) = self.window.as_ref()
                && let Err(error) = set_window_visibility(
                    &state.window,
                    true,
                    self.config.focus && self.config.focusable,
                )
            {
                self.popover_monitor.unwatch(handle);
                self.fail(event_loop, AppError::Platform(error));
                self.deactivate_window();
                return;
            }
            #[cfg(target_os = "macos")]
            if window_presentation_activates_application(&self.config)
                && let Some(state) = self.window.as_ref()
            {
                // A windowless AppRunner completes AppKit launch before an embedding runtime
                // queues its first window. Winit's launch-time activation pass therefore cannot
                // promote that window, so use its ordinary focus path after native presentation.
                state.window.focus_window();
            }
            if let Some(state) = &mut self.window {
                state.relation_presented = relation_presented;
                state.visible = true;
                #[cfg(not(target_os = "macos"))]
                state.window.set_visible(true);
                #[cfg(not(target_os = "macos"))]
                if self.config.focus && self.config.focusable {
                    state.window.focus_window();
                }
                state.scheduler.invalidate();
                state.window.request_redraw();
            }
        }
        self.opened_window = true;
        self.last_window_quit_prevented = false;
        self.deactivate_window();
    }

    fn process_foreground_tasks(&mut self, event_loop: &ActiveEventLoop) {
        debug_assert!(self.current_window.is_none());
        debug_assert!(self.window.is_none());

        let mut batch = self.foreground_tasks.take_ready_batch();
        while let Some(ScheduledForegroundTask {
            task,
            window: owner,
            runnable,
        }) = batch.pop_front()
        {
            if !self.foreground_tasks.owns(task, owner) {
                drop(runnable);
                continue;
            }
            if let Some(owner) = owner {
                let Some(window_id) = self.window_handles.get(&owner).copied() else {
                    self.foreground_tasks.cancel_task(task);
                    drop(runnable);
                    continue;
                };
                if !self.activate_window(window_id) {
                    self.foreground_tasks.cancel_task(task);
                    drop(runnable);
                    continue;
                }
            }

            let poll = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| runnable.run()));
            if poll.is_err() {
                tracing::error!(?task, ?owner, "foreground task panicked and was cancelled");
                self.foreground_tasks.cancel_task(task);
            }

            let mut continue_running = true;
            let mut updates = self.foreground_tasks.take_updates(task);
            debug_assert!(owner.is_some() || updates.is_empty());
            while let Some(update) = updates.pop_front() {
                let mut cx = self.event_context();
                if let Some(window) = &mut self.window {
                    update(window.view.as_any_mut(), &mut cx);
                }
                if !self.apply_event_context(event_loop, cx, false, true) {
                    continue_running = false;
                    break;
                }
            }

            self.deactivate_window();
            self.process_window_commands(event_loop);
            if !continue_running || self.fatal_error.is_some() {
                return;
            }
        }
    }
}

impl Drop for Runtime {
    fn drop(&mut self) {
        self.finalize_process_services();
    }
}

impl ApplicationHandler<RuntimeEvent> for Runtime {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.fatal_error.is_some() {
            return;
        }

        self.ready = true;
        event_loop.set_control_flow(ControlFlow::Wait);
        self.initialize_global_shortcuts();
        self.refresh_displays(event_loop);
        self.process_window_commands(event_loop);
        if let Some(urls) = self.pending_initial_open_urls.take() {
            self.invoke_open_urls(event_loop, urls);
        }
    }

    fn suspended(&mut self, _event_loop: &ActiveEventLoop) {
        // Desktop surfaces remain valid. Mobile surface teardown will be added with mobile shells.
    }

    fn exiting(&mut self, event_loop: &ActiveEventLoop) {
        // Native termination (for example macOS Quit) can bypass EventContext::exit. Route it
        // through the same child-first ownership teardown so foreground tasks and window-closed
        // callbacks never depend on which quit path the operating system selected.
        self.exit_requested = true;
        self.pending_windows.clear();
        self.close_requests
            .extend(self.window_handles.keys().copied());
        let closed = self.close_requested_window_trees(event_loop);
        self.invoke_window_closed_callbacks(event_loop, closed);
        #[cfg(target_os = "macos")]
        self.restore_native_tabbing_baseline(event_loop);
        self.pending_windows.clear();
        self.platform_requests.clear();
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: WindowId,
        event: WindowEvent,
    ) {
        if !self.activate_window(window_id) {
            return;
        }
        #[cfg(target_os = "macos")]
        let (event, platform_click_count) = match event {
            WindowEvent::MouseInputWithClickCount {
                device_id,
                state,
                button,
                click_count,
            } => (
                WindowEvent::MouseInput {
                    device_id,
                    state,
                    button,
                },
                Some(usize::from(click_count)),
            ),
            event => (event, None),
        };
        #[cfg(not(target_os = "macos"))]
        let platform_click_count: Option<usize> = None;
        (|| {
            if let Some(state) = self.window.as_mut()
                && state.window.id() == window_id
            {
                state.accessibility.process_event(&state.window, &event);
            }
            let Some(state) = self.window.as_ref() else {
                return;
            };
            if state.window.id() != window_id {
                return;
            }

            match event {
                WindowEvent::CloseRequested => {
                    let mut cx = self.event_context();
                    if let Some(window) = &mut self.window {
                        window.view.event(&Event::CloseRequested, &mut cx);
                    }
                    let prevent_close = cx.prevent_close;
                    let explicitly_closed = cx.close_current_window;
                    if !self.apply_event_context(event_loop, cx, false, true) {
                        return;
                    }
                    if !prevent_close
                        && !explicitly_closed
                        && let Some(handle) = self.current_handle()
                    {
                        self.close_requests.push(handle);
                    }
                }
                WindowEvent::Moved(physical) => {
                    let state = self.window.as_mut().expect("window checked above");
                    state.logical_position = Point::new(
                        physical.x as f32 / state.scale_factor,
                        physical.y as f32 / state.scale_factor,
                    );
                    state.display_id = runtime_window_display_id(state, &self.displays);
                    state.maximized = runtime_window_is_maximized(state, &self.config);
                    if !runtime_window_is_fullscreen(state) && !state.maximized {
                        state.restore_bounds.x = state.logical_position.x;
                        state.restore_bounds.y = state.logical_position.y;
                    }
                    let logical_position = state.logical_position;
                    let scale_factor = state.scale_factor;
                    let observe = state.listeners.observes_window_state;
                    #[cfg(target_os = "macos")]
                    self.refresh_current_native_tab_state();
                    self.dispatch(
                        event_loop,
                        Event::Moved {
                            logical_position,
                            scale_factor,
                        },
                        observe,
                    );
                }
                WindowEvent::ThemeChanged(theme) => {
                    let appearance = map_window_appearance(theme);
                    let color_scheme = match appearance {
                        WindowAppearance::Light => ColorScheme::Light,
                        WindowAppearance::Dark => ColorScheme::Dark,
                    };
                    self.refresh_system_preferences(SystemPreferences::snapshot().unwrap_or_else(
                        |error| {
                            tracing::warn!(%error, "could not refresh native system colors");
                            self.system_preferences.with_color_scheme(color_scheme)
                        },
                    ));
                    if self.config.preferred_appearance.is_none() {
                        let state = self.window.as_mut().expect("window checked above");
                        if state.appearance != appearance {
                            state.appearance = appearance;
                            let observe = state.listeners.observes_window_state;
                            self.dispatch(
                                event_loop,
                                Event::AppearanceChanged(appearance),
                                observe,
                            );
                        }
                    }
                }
                WindowEvent::Resized(physical) => {
                    let state = self.window.as_mut().expect("window checked above");
                    state.renderer.resize(physical.width, physical.height);
                    state.logical_size = logical_window_size(physical, state.scale_factor);
                    state.display_id = runtime_window_display_id(state, &self.displays);
                    state.maximized = runtime_window_is_maximized(state, &self.config);
                    if !runtime_window_is_fullscreen(state) && !state.maximized {
                        state.restore_bounds.width = state.logical_size.width;
                        state.restore_bounds.height = state.logical_size.height;
                    }
                    #[cfg(target_os = "macos")]
                    if let Some(position) = self.config.traffic_light_position
                        && let Err(error) = position_traffic_lights(&state.window, position)
                    {
                        tracing::warn!(%error, "could not restore the configured traffic-light position");
                    }
                    #[cfg(target_os = "macos")]
                    if let Err(error) =
                        set_window_movable(&state.window, implicit_native_movable(&self.config))
                    {
                        tracing::warn!(%error, "could not restore native window movability");
                    }
                    let logical_size = state.logical_size;
                    let scale_factor = state.scale_factor;
                    state.layout_dirty = true;
                    state.view_dirty |= state.listeners.observes_viewport;
                    #[cfg(target_os = "macos")]
                    self.refresh_current_native_tab_state();
                    self.dispatch(
                        event_loop,
                        Event::Resized {
                            logical_size,
                            scale_factor,
                        },
                        true,
                    );
                }
                WindowEvent::ScaleFactorChanged { scale_factor, .. } => {
                    let state = self.window.as_mut().expect("window checked above");
                    state.scale_factor = sane_scale_factor(scale_factor);
                    let physical = state.window.inner_size();
                    state.renderer.resize(physical.width, physical.height);
                    state.logical_size = logical_window_size(physical, state.scale_factor);
                    if let Some(position) =
                        logical_window_position(&state.window, state.scale_factor)
                    {
                        state.logical_position = position;
                    }
                    state.display_id = runtime_window_display_id(state, &self.displays);
                    state.maximized = runtime_window_is_maximized(state, &self.config);
                    if !runtime_window_is_fullscreen(state) && !state.maximized {
                        state.restore_bounds = Rect::new(
                            state.logical_position.x,
                            state.logical_position.y,
                            state.logical_size.width,
                            state.logical_size.height,
                        );
                    }
                    #[cfg(target_os = "macos")]
                    if let Some(position) = self.config.traffic_light_position
                        && let Err(error) = position_traffic_lights(&state.window, position)
                    {
                        tracing::warn!(%error, "could not restore the configured traffic-light position");
                    }
                    #[cfg(target_os = "macos")]
                    if let Err(error) =
                        set_window_movable(&state.window, implicit_native_movable(&self.config))
                    {
                        tracing::warn!(%error, "could not restore native window movability");
                    }
                    let logical_size = state.logical_size;
                    let scale_factor = state.scale_factor;
                    state.layout_dirty = true;
                    state.view_dirty |= state.listeners.observes_viewport;
                    #[cfg(target_os = "macos")]
                    self.refresh_current_native_tab_state();
                    self.dispatch(
                        event_loop,
                        Event::Resized {
                            logical_size,
                            scale_factor,
                        },
                        true,
                    );
                }
                WindowEvent::Occluded(false) => {
                    let state = self.window.as_mut().expect("window checked above");
                    state.occluded = false;
                    if state.listeners.observes_window_state
                        || state.ui.has_declarative_animations()
                    {
                        state.view_dirty = true;
                    }
                    state.ui.set_reduce_motion(state.reduce_motion);
                    state
                        .ui
                        .set_animations_enabled(!state.reduce_motion, Instant::now());
                    state.scheduler.invalidate();
                    state.window.request_redraw();
                }
                WindowEvent::Occluded(true) => {
                    let state = self.window.as_mut().expect("window checked above");
                    state.occluded = true;
                    state.ui.set_animations_enabled(false, Instant::now());
                }
                WindowEvent::RedrawRequested => {
                    #[cfg(target_os = "windows")]
                    if let Some(state) = &mut self.window
                        && state.visible
                        && !state.taskbar_state_applied
                        && state.taskbar_apply_attempts < 3
                    {
                        state.taskbar_apply_attempts += 1;
                        let progress = windows_window::set_taskbar_progress(
                            &state.window,
                            self.config.taskbar_progress_state,
                            self.config.taskbar_progress,
                        );
                        let overlay = windows_window::set_taskbar_overlay_icon(
                            &state.window,
                            self.config.taskbar_overlay_icon.as_ref(),
                            self.config.taskbar_overlay_description.as_deref(),
                        );
                        state.taskbar_state_applied = progress.is_ok() && overlay.is_ok();
                        if !state.taskbar_state_applied {
                            if state.taskbar_apply_attempts < 3 {
                                state.window.request_redraw();
                            } else {
                                let error =
                                    progress.err().or_else(|| overlay.err()).unwrap_or_default();
                                tracing::warn!(%error, "could not apply retained taskbar state");
                            }
                        }
                    }
                    self.redraw(event_loop)
                }
                WindowEvent::CursorMoved { position, .. } => {
                    let scale = self
                        .window
                        .as_ref()
                        .expect("window checked above")
                        .scale_factor;
                    let point = Point::new(position.x as f32 / scale, position.y as f32 / scale);
                    if let Some(state) = &mut self.window {
                        state.pointer = Some(point);
                    }
                    #[cfg(feature = "inspector")]
                    {
                        let consumed = self.window.as_ref().is_some_and(|state| {
                            state
                                .inspector
                                .as_ref()
                                .is_some_and(|inspector| inspector.captures_pointer(point))
                        });
                        if consumed {
                            let state = self.window.as_mut().expect("window checked above");
                            let inspector_changed = state
                                .inspector
                                .as_mut()
                                .is_some_and(|inspector| inspector.pointer_moved(point));
                            let app_changed = state.ui.pointer_left()
                                | state.ui.update_scrollbar_hover(None, Instant::now());
                            let cursor =
                                state
                                    .inspector
                                    .as_ref()
                                    .map_or(CursorIcon::Default, |inspector| {
                                        if inspector.mode() == InspectorMode::Picking
                                            && !inspector.panel_contains(point)
                                        {
                                            CursorIcon::Crosshair
                                        } else {
                                            CursorIcon::Default
                                        }
                                    });
                            set_cursor_if_changed(state, cursor);
                            if (inspector_changed || app_changed) && state.scheduler.invalidate() {
                                state.window.request_redraw();
                            }
                            if !self.invoke_pending_mouse_hover(event_loop) {
                                return;
                            }
                            return;
                        }
                    }
                    let drag_start = self.window.as_ref().and_then(|state| {
                        let candidate = state.drag_candidate?;
                        let delta = point - candidate.origin;
                        (state.drag_session.is_none()
                            && state.pointer_capture.is_none()
                            && delta.x.abs().max(delta.y.abs()) >= DRAG_THRESHOLD)
                            .then_some((candidate.source, candidate.origin))
                    });
                    if let Some((source, origin)) = drag_start
                        && !self.invoke_drag_start(
                            event_loop,
                            source,
                            DragStartEvent {
                                origin,
                                position: point,
                                modifiers: self.modifiers,
                            },
                        )
                    {
                        return;
                    }
                    #[cfg(target_os = "macos")]
                    if self.window.as_ref().is_some_and(|state| {
                        state.drag_session.is_some()
                            && point_outside_viewport(point, state.logical_size)
                    }) && self.promote_external_drag()
                    {
                        if let Some(state) = &mut self.window {
                            state.pointer = None;
                        }
                        self.dispatch(event_loop, Event::PointerLeft, false);
                        return;
                    }
                    let target_changed = self.update_drag_target(point);
                    let state = self.window.as_mut().expect("window checked above");
                    if let Some(drag) = &mut state.drag_session {
                        drag.position = point;
                    }
                    let preview_changed = state.ui.move_drag_preview(point);
                    let now = Instant::now();
                    let captured = state.pointer_capture.as_mut().map(|capture| {
                        let delta = point - capture.position;
                        capture.position = point;
                        (
                            capture.target,
                            PointerEvent {
                                phase: PointerPhase::Move,
                                position: point,
                                origin: capture.origin,
                                delta,
                                button: capture.button,
                                modifiers: self.modifiers,
                            },
                        )
                    });
                    let drag_active = state.any_drag_active();
                    let scrollbar_dragging = !drag_active && state.ui.scrollbar_drag_active();
                    let (over_scrollbar, app_region_drag, repaint) = if drag_active {
                        let hover_changed = state.ui.update_scrollbar_hover(None, now);
                        let RuntimeWindow { ui, renderer, .. } = state;
                        (
                            false,
                            false,
                            target_changed
                                | preview_changed
                                | hover_changed
                                | ui.pointer_moved(point, renderer),
                        )
                    } else if scrollbar_dragging {
                        let scroll_result = state.ui.drag_scrollbar(point);
                        if scroll_result.view_dirty {
                            state.view_dirty = true;
                        }
                        let repaint = scroll_result.changed | state.ui.pointer_left();
                        (true, false, repaint)
                    } else if captured.is_none() {
                        let hover_changed = state.ui.update_scrollbar_hover(Some(point), now);
                        let over_scrollbar = state.ui.is_over_scrollbar(point);
                        let app_region_drag = !over_scrollbar && state.ui.is_app_region_drag(point);
                        let pointer_changed = if over_scrollbar || app_region_drag {
                            state.ui.pointer_left()
                        } else {
                            let RuntimeWindow { ui, renderer, .. } = state;
                            ui.pointer_moved(point, renderer)
                        };
                        (
                            over_scrollbar,
                            app_region_drag,
                            hover_changed | pointer_changed,
                        )
                    } else {
                        let hover_changed = state.ui.update_scrollbar_hover(None, now);
                        let RuntimeWindow { ui, renderer, .. } = state;
                        (
                            false,
                            false,
                            hover_changed | ui.pointer_moved(point, renderer),
                        )
                    };
                    let cursor = if state.drag_session.is_some() {
                        CursorIcon::Grabbing
                    } else if over_scrollbar || scrollbar_dragging || app_region_drag {
                        CursorIcon::Default
                    } else {
                        state
                            .ui
                            .cursor_style_at(point)
                            .map_or(CursorIcon::Default, platform_cursor)
                    };
                    set_cursor_if_changed(state, cursor);
                    let pressed_button = state.pressed_mouse_buttons.current();
                    if repaint && state.scheduler.invalidate() {
                        state.window.request_redraw();
                    }
                    if !self.invoke_pending_mouse_hover(event_loop) {
                        return;
                    }
                    if over_scrollbar || scrollbar_dragging || app_region_drag {
                        // Keep window-level pointer tracking coherent while element-level hit
                        // testing remains occluded by the scrollbar or drag region.
                        self.dispatch(event_loop, Event::PointerMoved(point), false);
                        return;
                    }
                    if let Some((target, event)) = captured
                        && !self.invoke_pointer(event_loop, target, event)
                    {
                        return;
                    }
                    if self
                        .invoke_mouse_event_at(
                            event_loop,
                            point,
                            MouseListenerKind::Move,
                            None,
                            MouseListenerEvent::Move(MouseMoveEvent {
                                position: point,
                                pressed_button,
                                modifiers: self.modifiers,
                            }),
                        )
                        .is_none()
                    {
                        return;
                    }
                    self.dispatch(event_loop, Event::PointerMoved(point), false);
                }
                WindowEvent::CursorLeft { .. } => {
                    // Preserve the last retained in-window point for element-level exit dispatch.
                    // The macOS external-drag probe below samples the hardware boundary and may
                    // temporarily replace `state.pointer` with an out-of-window coordinate.
                    let exit_position = self
                        .window
                        .as_ref()
                        .and_then(|state| state.pointer)
                        .unwrap_or(Point::ZERO);
                    #[cfg(target_os = "macos")]
                    {
                        let boundary_point = self
                            .window
                            .as_ref()
                            .and_then(|state| current_pointer_position(&state.window));
                        if let Some(point) = boundary_point {
                            match self.promote_external_drag_at_boundary(event_loop, point) {
                                Some(true) => return,
                                Some(false) => {}
                                None => return,
                            }
                        } else {
                            self.promote_external_drag();
                        }
                    }
                    let state = self.window.as_mut().expect("window checked above");
                    let pressed_button = state.pressed_mouse_buttons.current();
                    state.pointer = None;
                    if state.cursor != CursorIcon::Default {
                        state.cursor = CursorIcon::Default;
                        state.window.set_cursor(CursorIcon::Default);
                    }
                    let repaint = state.ui.pointer_left()
                        | state.ui.update_scrollbar_hover(None, Instant::now())
                        | state.ui.set_drag_over(None)
                        | {
                            #[cfg(feature = "inspector")]
                            {
                                state
                                    .inspector
                                    .as_mut()
                                    .is_some_and(InspectorState::pointer_left)
                            }
                            #[cfg(not(feature = "inspector"))]
                            {
                                false
                            }
                        };
                    if repaint && state.scheduler.invalidate() {
                        state.window.request_redraw();
                    }
                    if !self.invoke_pending_mouse_hover(event_loop) {
                        return;
                    }
                    if self
                        .invoke_mouse_event_at(
                            event_loop,
                            exit_position,
                            MouseListenerKind::Exit,
                            None,
                            MouseListenerEvent::Exit(MouseExitEvent {
                                position: exit_position,
                                pressed_button,
                                modifiers: self.modifiers,
                            }),
                        )
                        .is_none()
                    {
                        return;
                    }
                    self.dispatch(event_loop, Event::PointerLeft, false);
                }
                WindowEvent::HoveredFile(path) => {
                    self.refresh_native_file_pointer();
                    self.hover_native_file(path);
                }
                WindowEvent::DroppedFile(path) => {
                    self.refresh_native_file_pointer();
                    self.drop_native_file(event_loop, path);
                }
                WindowEvent::HoveredFileCancelled => {
                    self.cancel_native_file_hover(event_loop);
                }
                WindowEvent::MouseInput { state, button, .. } => {
                    let pressed = state == ElementState::Pressed;
                    let button = map_mouse_button(button);
                    #[cfg(feature = "inspector")]
                    {
                        let (consumed, mut repaint, action) = {
                            let window = self.window.as_mut().expect("window checked above");
                            let point = window.pointer.unwrap_or(Point::ZERO);
                            window.inspector.as_mut().map_or(
                                (false, false, InspectorPointerAction::None),
                                |inspector| {
                                    inspector.pointer_button(
                                        point,
                                        pressed,
                                        button == MouseButton::Left,
                                    )
                                },
                            )
                        };
                        if consumed {
                            match action {
                                InspectorPointerAction::None => {}
                                InspectorPointerAction::StartPicking => {
                                    repaint |= self
                                        .window
                                        .as_mut()
                                        .and_then(|window| window.inspector.as_mut())
                                        .is_some_and(InspectorState::start_picking);
                                }
                                InspectorPointerAction::Close => {
                                    self.config.inspector = false;
                                    if let Some(window) = &mut self.window {
                                        window.inspector = None;
                                        reconcile_inspector_pointer_state(window);
                                        if window.listeners.observes_window_state {
                                            window.view_dirty = true;
                                        }
                                    }
                                    repaint = true;
                                }
                            }
                            let window = self.window.as_mut().expect("window retained");
                            if repaint && window.scheduler.invalidate() {
                                window.window.request_redraw();
                            }
                            return;
                        }
                    }
                    #[cfg(target_os = "macos")]
                    let native_click_count = platform_click_count;
                    #[cfg(not(target_os = "macos"))]
                    let native_click_count = platform_click_count;
                    let (mouse_position, click_count, first_mouse) = {
                        let window = self.window.as_mut().expect("window checked above");
                        let position = window.pointer;
                        let click_position = position.unwrap_or(Point::ZERO);
                        let first_mouse = !window.focused;
                        let click_count = if pressed {
                            window.pressed_mouse_buttons.press(button);
                            window.mouse_clicks.press(
                                button,
                                click_position,
                                Instant::now(),
                                native_click_count,
                            )
                        } else {
                            let count = window.mouse_clicks.release(button, native_click_count);
                            window.pressed_mouse_buttons.release(button);
                            count
                        };
                        (position, click_count, first_mouse)
                    };
                    #[cfg(target_os = "macos")]
                    let suppress_external_release = if button == MouseButton::Left {
                        let window = self.window.as_mut().expect("window checked above");
                        if pressed {
                            window.suppress_external_drag_release = false;
                            if let Some(monitor) = &window.external_drag_monitor {
                                monitor.disarm();
                            }
                            window.external_drag_mouse_down = None;
                            false
                        } else {
                            window.external_drag_mouse_down = None;
                            if let Some(monitor) = &window.external_drag_monitor {
                                monitor.disarm();
                            }
                            std::mem::take(&mut window.suppress_external_drag_release)
                        }
                    } else {
                        false
                    };
                    #[cfg(target_os = "macos")]
                    if suppress_external_release {
                        // Preserve raw button symmetry for application event handlers, but do not
                        // route the native drag's terminal release back through retained hit
                        // testing where it could activate a control under the drop position.
                        self.dispatch(event_loop, Event::MouseButton { button, pressed }, false);
                        return;
                    }
                    if pressed {
                        let window = self.window.as_mut().expect("window checked above");
                        if window.ui.clear_tooltip() && window.scheduler.invalidate() {
                            window.window.request_redraw();
                        }
                    }
                    let scrollbar_consumed = {
                        let window = self.window.as_mut().expect("window checked above");
                        if window.pointer_capture.is_some() || window.any_drag_active() {
                            false
                        } else {
                            let now = Instant::now();
                            let consumed = if button == MouseButton::Left
                                && window.ui.scrollbar_drag_active()
                            {
                                if !pressed {
                                    let result = window.ui.end_scrollbar_drag(now);
                                    window.view_dirty |= result.view_dirty;
                                    window.ui.update_scrollbar_hover(window.pointer, now);
                                }
                                true
                            } else if button == MouseButton::Left && pressed {
                                if let Some(view_dirty) =
                                    window.ui.begin_scrollbar_drag(window.pointer)
                                {
                                    window.view_dirty |= view_dirty;
                                    true
                                } else {
                                    false
                                }
                            } else {
                                window
                                    .pointer
                                    .is_some_and(|point| window.ui.is_over_scrollbar(point))
                            };
                            if consumed && window.scheduler.invalidate() {
                                window.window.request_redraw();
                            }
                            consumed
                        }
                    };
                    if scrollbar_consumed {
                        return;
                    }
                    let app_region_consumed = self.window.as_ref().is_some_and(|window| {
                        window.pointer_capture.is_none()
                            && !window.any_drag_active()
                            && window
                                .pointer
                                .is_some_and(|point| window.ui.is_app_region_drag(point))
                    });
                    if app_region_consumed {
                        if button == MouseButton::Left
                            && pressed
                            && self.config.is_movable
                            && let Some(window) = self.window.as_ref()
                        {
                            #[cfg(target_os = "macos")]
                            let result = perform_window_drag(
                                &window.window,
                                self.config.title_bar_style != TitleBarStyle::Default,
                            );
                            #[cfg(not(target_os = "macos"))]
                            let result = window
                                .window
                                .drag_window()
                                .map_err(|error| error.to_string());
                            if let Err(error) = result {
                                tracing::warn!(%error, "could not start window drag from app region");
                            }
                            #[cfg(target_os = "macos")]
                            if let Some(window) = &mut self.window {
                                window.external_drag_mouse_down = None;
                                if let Some(monitor) = &window.external_drag_monitor {
                                    monitor.disarm();
                                }
                            }
                        }
                        return;
                    }
                    // A direct captured-pointer release is terminal cleanup, not a preventable
                    // default. Deliver it before general mouse-up callbacks so closing or
                    // preventing from those callbacks cannot strand capture.
                    let terminal_capture = if pressed {
                        None
                    } else {
                        let window = self.window.as_mut().expect("window checked above");
                        window.drag_candidate = None;
                        window
                            .pointer_capture
                            .filter(|capture| capture.button == button)
                            .map(|capture| {
                                window.pointer_capture = None;
                                let position = window.pointer.unwrap_or(capture.position);
                                (
                                    capture.target,
                                    PointerEvent {
                                        phase: PointerPhase::Up,
                                        position,
                                        origin: capture.origin,
                                        delta: position - capture.position,
                                        button,
                                        modifiers: self.modifiers,
                                    },
                                )
                            })
                    };
                    if let Some((target, event)) = terminal_capture
                        && !self.invoke_pointer(event_loop, target, event)
                    {
                        return;
                    }

                    let default_prevented = if let Some(position) = mouse_position {
                        let result = if pressed {
                            self.invoke_mouse_event_at(
                                event_loop,
                                position,
                                MouseListenerKind::Down,
                                Some(button),
                                MouseListenerEvent::Down(MouseDownEvent {
                                    button,
                                    position,
                                    modifiers: self.modifiers,
                                    click_count,
                                    first_mouse,
                                }),
                            )
                        } else {
                            self.invoke_mouse_event_at(
                                event_loop,
                                position,
                                MouseListenerKind::Up,
                                Some(button),
                                MouseListenerEvent::Up(MouseUpEvent {
                                    button,
                                    position,
                                    modifiers: self.modifiers,
                                    click_count,
                                }),
                            )
                        };
                        let Some(default_prevented) = result else {
                            return;
                        };
                        default_prevented
                    } else {
                        false
                    };

                    let internal_drag_release = (button == MouseButton::Left && !pressed)
                        .then(|| {
                            self.window
                                .as_ref()
                                .filter(|window| window.drag_session.is_some())
                                .and_then(|window| {
                                    window.pointer.or_else(|| {
                                        window.drag_session.as_ref().map(|drag| drag.position)
                                    })
                                })
                        })
                        .flatten();
                    if let Some(position) = internal_drag_release {
                        if !self.finish_internal_drag(event_loop, position) {
                            return;
                        }
                        self.dispatch(event_loop, Event::MouseButton { button, pressed }, false);
                        return;
                    }
                    if default_prevented {
                        if !pressed {
                            let window = self.window.as_mut().expect("window checked above");
                            if window.ui.cancel_pointer_interaction()
                                && window.scheduler.invalidate()
                            {
                                window.window.request_redraw();
                            }
                        }
                        self.dispatch(event_loop, Event::MouseButton { button, pressed }, false);
                        return;
                    }
                    if button == MouseButton::Right && pressed {
                        let (target, position, dismiss) = self
                            .window
                            .as_ref()
                            .and_then(|window| {
                                let position = window.pointer?;
                                Some((
                                    window.ui.context_menu_listener_at(position),
                                    position,
                                    window.ui.dismiss_request_for_pointer(Some(position)),
                                ))
                            })
                            .unwrap_or((None, Point::ZERO, None));
                        if let Some(dismiss) = dismiss {
                            self.invoke_dismiss(event_loop, dismiss);
                            if self.window.is_none() {
                                return;
                            }
                        }
                        if let Some(target) = target {
                            self.dispatch(
                                event_loop,
                                Event::MouseButton { button, pressed },
                                false,
                            );
                            self.invoke_context_menu(
                                event_loop,
                                ContextMenuEvent {
                                    target,
                                    position,
                                    modifiers: self.modifiers,
                                },
                            );
                            return;
                        }
                    }
                    let (pointer_result, previous_focus, captured) = {
                        let window = self.window.as_mut().expect("window checked above");
                        let previous_focus = window.ui.focused();
                        let result = if button == MouseButton::Left {
                            let RuntimeWindow { ui, renderer, .. } = window;
                            ui.pointer_button(
                                window.pointer,
                                pressed,
                                self.modifiers.contains(Modifiers::SHIFT),
                                Instant::now(),
                                renderer,
                            )
                        } else {
                            crate::ui_tree::PointerResult {
                                repaint: false,
                                clicked: None,
                                dismissed: None,
                                pointer_listener: window
                                    .pointer
                                    .and_then(|point| window.ui.pointer_listener_at(point)),
                                drag_source: None,
                            }
                        };
                        let captured = if pressed {
                            if window.pointer_capture.is_none() {
                                result.pointer_listener.and_then(|target| {
                                    let position = window.pointer?;
                                    let capture = PointerCapture {
                                        target,
                                        button,
                                        origin: position,
                                        position,
                                    };
                                    window.pointer_capture = Some(capture);
                                    Some((
                                        target,
                                        PointerEvent {
                                            phase: PointerPhase::Down,
                                            position,
                                            origin: position,
                                            delta: Vector::ZERO,
                                            button,
                                            modifiers: self.modifiers,
                                        },
                                    ))
                                })
                            } else {
                                None
                            }
                        } else {
                            window.drag_candidate = None;
                            window
                                .pointer_capture
                                .filter(|capture| capture.button == button)
                                .map(|capture| {
                                    window.pointer_capture = None;
                                    let position = window.pointer.unwrap_or(capture.position);
                                    (
                                        capture.target,
                                        PointerEvent {
                                            phase: PointerPhase::Up,
                                            position,
                                            origin: capture.origin,
                                            delta: position - capture.position,
                                            button,
                                            modifiers: self.modifiers,
                                        },
                                    )
                                })
                        };
                        if button == MouseButton::Left && pressed {
                            window.drag_candidate = result.drag_source.and_then(|source| {
                                window
                                    .pointer
                                    .map(|origin| DragCandidate { source, origin })
                            });
                        }
                        if result.repaint && window.scheduler.invalidate() {
                            window.window.request_redraw();
                        }
                        (result, previous_focus, captured)
                    };
                    #[cfg(target_os = "macos")]
                    if button == MouseButton::Left
                        && pressed
                        && pointer_result.drag_source.is_some()
                    {
                        if let Some(window) = &mut self.window {
                            window.external_drag_mouse_down = capture_left_mouse_down();
                        }
                        self.arm_external_drag_monitor();
                    }
                    self.announce_focus_change(event_loop, previous_focus);
                    if let Some((target, event)) = captured
                        && !self.invoke_pointer(event_loop, target, event)
                    {
                        return;
                    }
                    self.dispatch(event_loop, Event::MouseButton { button, pressed }, false);
                    if let Some(request) = pointer_result.dismissed {
                        self.invoke_dismiss(event_loop, request);
                    }
                    if let Some(id) = pointer_result.clicked {
                        self.invoke_click(event_loop, id);
                    }
                }
                WindowEvent::Touch(touch) => {
                    let event = {
                        let state = self.window.as_ref().expect("window checked above");
                        TouchEvent {
                            id: TouchId(touch.id),
                            phase: map_touch_phase(touch.phase),
                            position: Point::new(
                                touch.location.x as f32 / state.scale_factor,
                                touch.location.y as f32 / state.scale_factor,
                            ),
                            force: touch.force.map(bounded_touch_force),
                        }
                        .bounded()
                    };
                    let target = {
                        let state = self.window.as_mut().expect("window checked above");
                        match event.phase {
                            TouchPhase::Started => {
                                state.touch_captures.remove(&event.id);
                                let target = state.ui.touch_listener_at(event.position);
                                if let Some(target) = target
                                    && state.touch_captures.len() < MAX_ACTIVE_TOUCHES_PER_WINDOW
                                {
                                    state.touch_captures.insert(
                                        event.id,
                                        TouchCapture {
                                            target,
                                            last_event: event,
                                        },
                                    );
                                    Some(target)
                                } else {
                                    None
                                }
                            }
                            TouchPhase::Moved => {
                                state.touch_captures.get_mut(&event.id).map(|capture| {
                                    capture.last_event = event;
                                    capture.target
                                })
                            }
                            TouchPhase::Ended | TouchPhase::Cancelled => state
                                .touch_captures
                                .remove(&event.id)
                                .map(|capture| capture.target),
                        }
                    };
                    self.invoke_touch(event_loop, target, event);
                }
                WindowEvent::MouseWheel { delta, phase, .. } => {
                    let (position, scale_factor, target) = {
                        let state = self.window.as_ref().expect("window checked above");
                        if state
                            .pointer
                            .is_some_and(|point| state.ui.is_app_region_drag(point))
                        {
                            return;
                        }
                        let position = state.pointer.unwrap_or(Point::ZERO);
                        let target = state
                            .pointer
                            .and_then(|point| state.ui.scroll_wheel_listener_at(point));
                        (position, state.scale_factor, target)
                    };
                    let delta = match delta {
                        MouseScrollDelta::LineDelta(x, y) => ScrollDelta::Lines(Vector::new(x, y)),
                        MouseScrollDelta::PixelDelta(delta) => ScrollDelta::Pixels(Vector::new(
                            delta.x as f32 / scale_factor,
                            delta.y as f32 / scale_factor,
                        )),
                    };
                    let event = ScrollWheelEvent {
                        position,
                        delta,
                        phase: map_gesture_phase(phase),
                        modifiers: self.modifiers,
                    }
                    .bounded();
                    #[cfg(feature = "inspector")]
                    if let Some((consumed, changed)) = self.window.as_mut().and_then(|state| {
                        let inspector = state.inspector.as_mut()?;
                        Some(inspector.scroll(
                            position,
                            event.delta.pixel_delta(self.config.line_scroll_pixels),
                        ))
                    }) && consumed
                    {
                        let state = self.window.as_mut().expect("window retained");
                        if changed && state.scheduler.invalidate() {
                            state.window.request_redraw();
                        }
                        return;
                    }
                    if let Some(target) = target {
                        let Some(default_prevented) =
                            self.invoke_scroll_wheel(event_loop, target, event)
                        else {
                            return;
                        };
                        if default_prevented {
                            return;
                        }
                    }
                    let delta = event.delta.pixel_delta(self.config.line_scroll_pixels);
                    if delta.is_zero() {
                        return;
                    }
                    let state = self
                        .window
                        .as_mut()
                        .expect("window retained after callback");
                    if state.scheduler.accumulate_scroll(delta) {
                        state.window.request_redraw();
                    }
                }
                WindowEvent::TouchpadPressure {
                    pressure, stage, ..
                } => {
                    let (position, target) = self
                        .window
                        .as_ref()
                        .and_then(|window| {
                            let position = window.pointer?;
                            Some((position, window.ui.mouse_pressure_listener_at(position)))
                        })
                        .unwrap_or((Point::ZERO, None));
                    self.invoke_mouse_pressure(
                        event_loop,
                        target,
                        MousePressureEvent {
                            position,
                            pressure: bounded_pressure(pressure),
                            stage: map_pressure_stage(stage),
                            modifiers: self.modifiers,
                        },
                    );
                }
                WindowEvent::PinchGesture { delta, phase, .. } => {
                    let (position, target) = self
                        .window
                        .as_ref()
                        .and_then(|window| {
                            let position = window.pointer?;
                            Some((position, window.ui.pinch_listener_at(position)))
                        })
                        .unwrap_or((Point::ZERO, None));
                    self.invoke_pinch(
                        event_loop,
                        target,
                        PinchEvent {
                            position,
                            delta: bounded_gesture_delta(delta, MAX_PINCH_DELTA_PER_EVENT),
                            phase: map_gesture_phase(phase),
                            modifiers: self.modifiers,
                        },
                    );
                }
                WindowEvent::RotationGesture { delta, phase, .. } => {
                    let (position, target) = self
                        .window
                        .as_ref()
                        .and_then(|window| {
                            let position = window.pointer?;
                            Some((position, window.ui.rotation_listener_at(position)))
                        })
                        .unwrap_or((Point::ZERO, None));
                    self.invoke_rotation(
                        event_loop,
                        target,
                        RotationEvent {
                            position,
                            delta: bounded_gesture_delta(
                                f64::from(delta),
                                MAX_ROTATION_DEGREES_PER_EVENT,
                            ),
                            phase: map_gesture_phase(phase),
                            modifiers: self.modifiers,
                        },
                    );
                }
                WindowEvent::DoubleTapGesture { .. } => {
                    let (position, target) = self
                        .window
                        .as_ref()
                        .and_then(|window| {
                            let position = window.pointer?;
                            Some((position, window.ui.smart_magnify_listener_at(position)))
                        })
                        .unwrap_or((Point::ZERO, None));
                    self.invoke_smart_magnify(
                        event_loop,
                        target,
                        SmartMagnifyEvent {
                            position,
                            modifiers: self.modifiers,
                        },
                    );
                }
                WindowEvent::ModifiersChanged(modifiers) => {
                    self.modifiers = map_modifiers(modifiers.state());
                    self.dispatch(event_loop, Event::ModifiersChanged(self.modifiers), false);
                }
                WindowEvent::KeyboardInput { event, .. } => {
                    let stroke = self.keyboard.keystroke(&event, self.modifiers);
                    let key = stroke.key.clone();
                    #[cfg(feature = "inspector")]
                    if event.state == ElementState::Pressed
                        && key == Key::Escape
                        && self
                            .window
                            .as_ref()
                            .is_some_and(|state| state.inspector.is_some())
                    {
                        self.config.inspector = false;
                        let state = self.window.as_mut().expect("window checked above");
                        state.inspector = None;
                        reconcile_inspector_pointer_state(state);
                        if state.listeners.observes_window_state {
                            state.view_dirty = true;
                        }
                        if state.scheduler.invalidate() {
                            state.window.request_redraw();
                        }
                        return;
                    }
                    if event.state == ElementState::Pressed {
                        if key == Key::Escape
                            && window_dismisses_system_popover_on_escape(&self.config)
                        {
                            if let Some(handle) = self.current_handle() {
                                self.close_requests.push(handle);
                            }
                            return;
                        }
                        if key == Key::Escape && self.cancel_internal_drag() {
                            if self
                                .invoke_key_event(
                                    event_loop,
                                    KeyListenerEvent::Down(KeyDownEvent {
                                        key: key.clone(),
                                        key_char: stroke.key_char.clone(),
                                        text: event.text.as_ref().map(ToString::to_string),
                                        modifiers: stroke.modifiers,
                                        repeat: event.repeat,
                                    }),
                                )
                                .is_none()
                            {
                                return;
                            }
                            self.dispatch(
                                event_loop,
                                Event::KeyDown {
                                    key,
                                    key_char: stroke.key_char,
                                    modifiers: stroke.modifiers,
                                    repeat: event.repeat,
                                },
                                false,
                            );
                            return;
                        }
                        self.handle_pressed_key(
                            event_loop,
                            PendingKey {
                                stroke,
                                repeat: event.repeat,
                                text: event.text.map(|text| text.to_string()),
                            },
                        );
                    } else {
                        let Keystroke {
                            key,
                            key_char,
                            modifiers,
                        } = stroke;
                        if self
                            .invoke_key_event(
                                event_loop,
                                KeyListenerEvent::Up(KeyUpEvent {
                                    key: key.clone(),
                                    key_char: key_char.clone(),
                                    modifiers,
                                }),
                            )
                            .is_none()
                        {
                            return;
                        }
                        self.dispatch(
                            event_loop,
                            Event::KeyUp {
                                key,
                                key_char,
                                modifiers,
                            },
                            false,
                        );
                    }
                }
                WindowEvent::Ime(Ime::Preedit(text, cursor)) => {
                    let result = self
                        .window
                        .as_mut()
                        .map(|window| window.ui.input_preedit(&text, cursor))
                        .unwrap_or_default();
                    self.apply_input_result(event_loop, result, false);
                }
                WindowEvent::Ime(Ime::Commit(text)) => {
                    let result = self
                        .window
                        .as_mut()
                        .map(|window| window.ui.input_replace(&text))
                        .unwrap_or_default();
                    let changed = result.change.is_some();
                    if self.apply_input_result(event_loop, result, true) && changed {
                        self.dispatch(event_loop, Event::TextInput(text), false);
                    }
                }
                WindowEvent::Ime(Ime::Disabled) => {
                    let result = self
                        .window
                        .as_mut()
                        .map(|window| window.ui.input_preedit("", None))
                        .unwrap_or_default();
                    self.apply_input_result(event_loop, result, false);
                }
                WindowEvent::Ime(Ime::Enabled) => {}
                WindowEvent::Focused(focused) => {
                    let was_focused = self.window.as_ref().is_some_and(|state| state.focused);
                    let never_key_popovers_to_close = if !focused && was_focused {
                        self.current_never_key_popover_children()
                    } else {
                        Vec::new()
                    };
                    #[cfg(not(target_os = "macos"))]
                    let popover_root_to_close = (!focused
                        && was_focused
                        && window_dismisses_system_popover_on_pointer_outside(&self.config))
                    .then(|| self.current_handle())
                    .flatten();
                    if let Some(state) = &mut self.window {
                        let rebuild = state
                            .listeners
                            .requires_window_state_rebuild(state.focused != focused);
                        state.focused = focused;
                        state.view_dirty |= rebuild;
                    }
                    if focused {
                        self.note_window_focused(window_id);
                        #[cfg(target_os = "macos")]
                        if !self.install_active_mac_menu(event_loop) {
                            return;
                        }
                    }
                    if !focused {
                        #[cfg(not(target_os = "macos"))]
                        if let Some(root) = popover_root_to_close {
                            self.close_requests.push(root);
                        }
                        self.close_requests.extend(never_key_popovers_to_close);
                        let cancelled = self.window.as_mut().and_then(|state| {
                            let capture = state.pointer_capture.take();
                            let internal_drag = state.drag_session.take().is_some();
                            state.drag_candidate = None;
                            state.pressed_mouse_buttons = PressedMouseButtons::default();
                            state.mouse_clicks.cancel();
                            #[cfg(target_os = "macos")]
                            {
                                state.external_drag_mouse_down = None;
                                if let Some(monitor) = &state.external_drag_monitor {
                                    monitor.disarm();
                                }
                                if state.outbound_external_drag.is_none() {
                                    state.suppress_external_drag_release = false;
                                }
                            }
                            let repaint = state.ui.cancel_pointer_interaction()
                                | (internal_drag && state.ui.end_drag())
                                | (internal_drag && state.ui.clear_drag_preview());
                            if repaint && state.scheduler.invalidate() {
                                state.window.request_redraw();
                            }
                            let capture = capture?;
                            Some((
                                capture.target,
                                PointerEvent {
                                    phase: PointerPhase::Cancel,
                                    position: capture.position,
                                    origin: capture.origin,
                                    delta: Vector::ZERO,
                                    button: capture.button,
                                    modifiers: self.modifiers,
                                },
                            ))
                        });
                        if let Some((target, event)) = cancelled
                            && !self.invoke_pointer(event_loop, target, event)
                        {
                            return;
                        }
                        loop {
                            let cancelled_touch = self.window.as_mut().and_then(|state| {
                                let id = state.touch_captures.keys().next().copied()?;
                                let capture = state.touch_captures.remove(&id)?;
                                let mut event = capture.last_event;
                                event.phase = TouchPhase::Cancelled;
                                Some((capture.target, event))
                            });
                            let Some((target, event)) = cancelled_touch else {
                                break;
                            };
                            if !self.invoke_touch(event_loop, Some(target), event) {
                                return;
                            }
                        }
                    }
                    if focused {
                        let reduce_motion = self.config.reduce_motion
                            || self
                                .system_preferences
                                .reduce_motion()
                                .is_some_and(|enabled| enabled);
                        let state = self.window.as_mut().expect("window checked above");
                        state.reduce_motion = reduce_motion;
                        state.ui.set_reduce_motion(reduce_motion);
                        state.ui.set_animations_enabled(
                            !state.occluded && !reduce_motion,
                            Instant::now(),
                        );
                    }
                    #[cfg(target_os = "macos")]
                    self.refresh_current_native_tab_state();
                    self.dispatch(event_loop, Event::Focused(focused), true);
                }
                WindowEvent::Destroyed => {
                    if let Some(handle) = self.current_handle() {
                        self.close_requests.push(handle);
                    }
                }
                _ => {}
            }
        })();
        self.deactivate_window();
        self.process_window_commands(event_loop);
    }

    fn user_event(&mut self, event_loop: &ActiveEventLoop, event: RuntimeEvent) {
        if matches!(&event, RuntimeEvent::ExternalCommandsReady) {
            self.process_window_commands(event_loop);
            return;
        }
        if let RuntimeEvent::InvalidateWindow(handle) = &event {
            self.invalidate_external(*handle);
            return;
        }
        if matches!(&event, RuntimeEvent::ForegroundTasksReady) {
            self.process_foreground_tasks(event_loop);
            return;
        }
        #[cfg(target_os = "macos")]
        if let RuntimeEvent::DockMenuAction(action_id) = &event {
            self.invoke_dock_menu_action(event_loop, *action_id);
            return;
        }
        #[cfg(any(target_os = "macos", target_os = "windows"))]
        if let RuntimeEvent::NativePopupMenuClosed(popup_id) = &event {
            self.pending_native_popup_menus.remove(popup_id);
            return;
        }
        if let RuntimeEvent::OpenUrls(urls) = &event {
            self.invoke_open_urls(event_loop, urls.clone());
            return;
        }
        #[cfg(target_os = "macos")]
        if let RuntimeEvent::Reopen {
            has_visible_windows,
        } = &event
        {
            self.invoke_reopen(event_loop, *has_visible_windows);
            return;
        }
        #[cfg(target_os = "macos")]
        if matches!(&event, RuntimeEvent::QuitRequested) {
            self.native_termination_pending = true;
            self.pending_quit = Some(QuitReason::OperatingSystem);
            self.process_window_commands(event_loop);
            return;
        }
        #[cfg(target_os = "macos")]
        if matches!(&event, RuntimeEvent::SystemWake) {
            self.invoke_system_wake(event_loop);
            return;
        }
        #[cfg(target_os = "macos")]
        if matches!(&event, RuntimeEvent::DisplaysChanged) {
            self.refresh_displays(event_loop);
            return;
        }
        #[cfg(target_os = "macos")]
        if matches!(&event, RuntimeEvent::ApplicationDeactivated) {
            let popovers = self.popovers_to_close_after_application_deactivation();
            self.close_requests.extend(popovers);
            self.process_window_commands(event_loop);
            return;
        }
        #[cfg(target_os = "macos")]
        if matches!(&event, RuntimeEvent::KeyboardLayoutChanged) {
            if self.refresh_keyboard_layout() {
                self.invoke_keyboard_layout_change(event_loop);
                self.sync_active_native_menu_state();
            }
            return;
        }
        if let RuntimeEvent::SystemPreferencesChanged(preferences) = &event {
            self.refresh_system_preferences(*preferences);
            return;
        }
        #[cfg(target_os = "macos")]
        if let RuntimeEvent::SystemNotificationAuthorization { granted, error } = &event {
            if let Some(host) = self.mac_application_host.as_mut() {
                host.complete_system_notification_authorization(*granted, error.clone());
            }
            return;
        }
        #[cfg(target_os = "macos")]
        if let RuntimeEvent::SystemNotificationPermissionStatus(status) = &event {
            if let Some(host) = self.mac_application_host.as_mut() {
                host.complete_system_notification_permission_status(*status);
            }
            return;
        }
        if let RuntimeEvent::SystemNotificationResponse(response) = &event {
            self.invoke_system_notification_response(event_loop, response.clone());
            return;
        }
        #[cfg(any(target_os = "macos", target_os = "windows", target_os = "linux"))]
        if let RuntimeEvent::GlobalShortcut(hotkey_id) = &event {
            self.invoke_global_shortcut(event_loop, *hotkey_id);
            return;
        }
        #[cfg(any(
            target_os = "macos",
            target_os = "windows",
            target_os = "linux",
            target_os = "freebsd",
            target_os = "dragonfly",
            target_os = "openbsd",
            target_os = "netbsd"
        ))]
        if let RuntimeEvent::SecondInstance(second_instance) = &event {
            self.invoke_second_instance(event_loop, second_instance.clone());
            return;
        }
        if let RuntimeEvent::Power(power_event) = &event {
            self.invoke_power_event(event_loop, *power_event);
            return;
        }
        if let RuntimeEvent::Tray(tray_event) = &event {
            self.invoke_tray_event(event_loop, tray_event.clone());
            return;
        }
        #[cfg(target_os = "macos")]
        if let RuntimeEvent::PopoverPointerDismissRequested(handle) = &event {
            let should_close = self
                .window_handles
                .get(handle)
                .and_then(|window_id| self.windows.get(window_id))
                .is_some_and(|entry| {
                    entry.state.visible
                        && window_dismisses_system_popover_on_pointer_outside(&entry.config)
                });
            if should_close {
                self.close_requests.push(*handle);
                self.process_window_commands(event_loop);
            }
            return;
        }
        #[cfg(any(
            target_os = "macos",
            target_os = "windows",
            target_os = "linux",
            target_os = "freebsd",
            target_os = "dragonfly",
            target_os = "openbsd",
            target_os = "netbsd"
        ))]
        if let RuntimeEvent::PlatformDialogClosed(owner, id) = &event {
            if self
                .active_platform_dialogs
                .get(owner)
                .is_some_and(|dialog| dialog.id == *id)
            {
                self.active_platform_dialogs.remove(owner);
            }
            return;
        }
        #[cfg(target_os = "macos")]
        if let RuntimeEvent::PlatformDialogCancelled(owner, id) = &event {
            if self
                .active_platform_dialogs
                .get(owner)
                .is_some_and(|dialog| dialog.id == *id)
                && let Some(dialog) = self.active_platform_dialogs.remove(owner)
            {
                dialog.native.cancel();
            }
            return;
        }
        let released_image_capacity = matches!(&event, RuntimeEvent::ImageLoaded(_, _));
        let target = match &event {
            RuntimeEvent::ExternalCommandsReady => unreachable!("handled before target routing"),
            RuntimeEvent::InvalidateWindow(_) => unreachable!("handled before target routing"),
            RuntimeEvent::Accessibility(event) => Some(event.window_id),
            RuntimeEvent::ImageLoaded(handle, _) => self.window_handles.get(handle).copied(),
            RuntimeEvent::BackgroundCompleted(completion) => {
                self.window_handles.get(&completion.window).copied()
            }
            RuntimeEvent::ForegroundTasksReady => unreachable!("handled before target routing"),
            #[cfg(target_os = "macos")]
            RuntimeEvent::ExternalDragBoundary(handle, _) => {
                self.window_handles.get(handle).copied()
            }
            #[cfg(target_os = "macos")]
            RuntimeEvent::ExternalDragEnded(handle, _) => self.window_handles.get(handle).copied(),
            #[cfg(target_os = "macos")]
            RuntimeEvent::NativeDropChanged(handle) => self.window_handles.get(handle).copied(),
            #[cfg(target_os = "macos")]
            RuntimeEvent::ApplicationDeactivated => unreachable!("handled before routing"),
            #[cfg(target_os = "macos")]
            RuntimeEvent::PopoverPointerDismissRequested(_) => {
                unreachable!("handled before routing")
            }
            #[cfg(any(
                target_os = "macos",
                target_os = "windows",
                target_os = "linux",
                target_os = "freebsd",
                target_os = "dragonfly",
                target_os = "openbsd",
                target_os = "netbsd"
            ))]
            RuntimeEvent::PlatformDialogClosed(_, _) => unreachable!("handled before routing"),
            #[cfg(target_os = "macos")]
            RuntimeEvent::PlatformDialogCancelled(_, _) => unreachable!("handled before routing"),
            RuntimeEvent::SystemNotificationResponse(_) => {
                unreachable!("handled before window routing")
            }
            #[cfg(any(target_os = "macos", target_os = "windows", target_os = "linux"))]
            RuntimeEvent::GlobalShortcut(_) => unreachable!("handled before window routing"),
            #[cfg(any(
                target_os = "macos",
                target_os = "windows",
                target_os = "linux",
                target_os = "freebsd",
                target_os = "dragonfly",
                target_os = "openbsd",
                target_os = "netbsd"
            ))]
            RuntimeEvent::SecondInstance(_) => unreachable!("handled before window routing"),
            RuntimeEvent::SystemPreferencesChanged(_) => {
                unreachable!("handled before window routing")
            }
            RuntimeEvent::Power(_) => unreachable!("handled before window routing"),
            RuntimeEvent::Tray(_) => unreachable!("handled before window routing"),
            RuntimeEvent::OpenUrls(_) => unreachable!("handled before window routing"),
            #[cfg(target_os = "macos")]
            RuntimeEvent::Reopen { .. }
            | RuntimeEvent::QuitRequested
            | RuntimeEvent::SystemWake
            | RuntimeEvent::DisplaysChanged
            | RuntimeEvent::KeyboardLayoutChanged
            | RuntimeEvent::SystemNotificationPermissionStatus(_)
            | RuntimeEvent::SystemNotificationAuthorization { .. } => {
                unreachable!("handled before window routing")
            }
            RuntimeEvent::MenuWillOpen | RuntimeEvent::MenuAction(_) => self.active_window,
            #[cfg(target_os = "macos")]
            RuntimeEvent::DockMenuAction(_) => unreachable!("handled before window routing"),
            #[cfg(any(target_os = "macos", target_os = "windows"))]
            RuntimeEvent::NativePopupMenuAction(popup_id, _) => self
                .pending_native_popup_menus
                .get(popup_id)
                .and_then(|popup| self.window_handles.get(&popup.window).copied()),
            #[cfg(any(target_os = "macos", target_os = "windows"))]
            RuntimeEvent::NativePopupMenuClosed(_) => {
                unreachable!("handled before window routing")
            }
        };
        let Some(target) = target else {
            return;
        };
        if !self.activate_window(target) {
            return;
        }
        (|| match event {
            RuntimeEvent::ExternalCommandsReady => unreachable!("handled before window routing"),
            RuntimeEvent::InvalidateWindow(_) => unreachable!("handled before window routing"),
            RuntimeEvent::ImageLoaded(_, completion) => {
                let Some(state) = &mut self.window else {
                    return;
                };
                if state.image_assets.complete(completion) {
                    state.view_dirty = true;
                    if state.scheduler.invalidate() {
                        state.window.request_redraw();
                    }
                }
            }
            RuntimeEvent::BackgroundCompleted(completion) => {
                let mut context = self.event_context();
                if let Some(state) = &mut self.window {
                    (completion.callback)(state.view.as_any_mut(), &mut context);
                }
                self.apply_event_context(event_loop, context, false, true);
            }
            RuntimeEvent::ForegroundTasksReady => unreachable!("handled before window routing"),
            RuntimeEvent::MenuWillOpen => {
                self.pending_input = None;
                #[cfg(target_os = "macos")]
                self.sync_native_menu_state();
            }
            RuntimeEvent::MenuAction(action_id) => {
                #[cfg(any(target_os = "macos", target_os = "windows"))]
                {
                    let menus = self.active_menu_declaration().to_vec();
                    self.menu_actions = collect_menu_actions(&menus);
                }
                let item = self
                    .menu_actions
                    .get(action_id)
                    .filter(|item| !item.disabled)
                    .map(|item| (item.action.clone(), item.os_action));
                if let Some((action, os_action)) = item {
                    let handled = if let Some(action) = action {
                        let Some(handled) = self.invoke_action(event_loop, &action) else {
                            return;
                        };
                        handled
                    } else {
                        false
                    };
                    if !handled && let Some(os_action) = os_action {
                        self.invoke_os_action(event_loop, os_action);
                    }
                    #[cfg(target_os = "macos")]
                    self.sync_native_menu_state();
                }
            }
            #[cfg(target_os = "macos")]
            RuntimeEvent::DockMenuAction(_) => unreachable!("handled before window routing"),
            #[cfg(any(target_os = "macos", target_os = "windows"))]
            RuntimeEvent::NativePopupMenuAction(popup_id, action_id) => {
                let item = self
                    .pending_native_popup_menus
                    .remove(&popup_id)
                    .and_then(|popup| popup.actions.into_iter().nth(action_id))
                    .filter(|item| !item.disabled)
                    .map(|item| (item.action, item.os_action));
                if let Some((action, os_action)) = item {
                    let handled = if let Some(action) = action {
                        let Some(handled) = self.invoke_action(event_loop, &action) else {
                            return;
                        };
                        handled
                    } else {
                        false
                    };
                    if !handled && let Some(os_action) = os_action {
                        self.invoke_os_action(event_loop, os_action);
                    }
                    #[cfg(target_os = "macos")]
                    self.sync_native_menu_state();
                }
            }
            #[cfg(any(target_os = "macos", target_os = "windows"))]
            RuntimeEvent::NativePopupMenuClosed(_) => {
                unreachable!("handled before window routing")
            }
            #[cfg(target_os = "macos")]
            RuntimeEvent::ExternalDragBoundary(_, point) => {
                let _ = self.promote_external_drag_at_boundary(event_loop, point);
            }
            #[cfg(target_os = "macos")]
            RuntimeEvent::ExternalDragEnded(_, operation) => {
                let source = self
                    .window
                    .as_mut()
                    .and_then(|state| state.outbound_external_drag.take().map(|drag| drag.source));
                if let Some(source) = source {
                    self.dispatch(
                        event_loop,
                        Event::ExternalDragEnded(ExternalDragEndEvent { source, operation }),
                        false,
                    );
                }
            }
            #[cfg(target_os = "macos")]
            RuntimeEvent::NativeDropChanged(_) => {
                let pending = self
                    .window
                    .as_ref()
                    .and_then(|state| state.native_drop_host.take_pending());
                if let Some(pending) = pending {
                    self.handle_native_drop_pending(event_loop, pending);
                }
            }
            #[cfg(target_os = "macos")]
            RuntimeEvent::ApplicationDeactivated => unreachable!("handled before routing"),
            #[cfg(target_os = "macos")]
            RuntimeEvent::PopoverPointerDismissRequested(_) => {
                unreachable!("handled before routing")
            }
            #[cfg(any(
                target_os = "macos",
                target_os = "windows",
                target_os = "linux",
                target_os = "freebsd",
                target_os = "dragonfly",
                target_os = "openbsd",
                target_os = "netbsd"
            ))]
            RuntimeEvent::PlatformDialogClosed(_, _) => unreachable!("handled before routing"),
            #[cfg(target_os = "macos")]
            RuntimeEvent::PlatformDialogCancelled(_, _) => {
                unreachable!("handled before routing")
            }
            RuntimeEvent::SystemNotificationResponse(_) => {
                unreachable!("handled before window routing")
            }
            #[cfg(any(target_os = "macos", target_os = "windows", target_os = "linux"))]
            RuntimeEvent::GlobalShortcut(_) => unreachable!("handled before window routing"),
            #[cfg(any(
                target_os = "macos",
                target_os = "windows",
                target_os = "linux",
                target_os = "freebsd",
                target_os = "dragonfly",
                target_os = "openbsd",
                target_os = "netbsd"
            ))]
            RuntimeEvent::SecondInstance(_) => unreachable!("handled before window routing"),
            RuntimeEvent::SystemPreferencesChanged(_) => {
                unreachable!("handled before window routing")
            }
            RuntimeEvent::Power(_) => unreachable!("handled before window routing"),
            RuntimeEvent::Tray(_) => unreachable!("handled before window routing"),
            RuntimeEvent::OpenUrls(_) => unreachable!("handled before window routing"),
            #[cfg(target_os = "macos")]
            RuntimeEvent::Reopen { .. }
            | RuntimeEvent::QuitRequested
            | RuntimeEvent::SystemWake
            | RuntimeEvent::DisplaysChanged
            | RuntimeEvent::KeyboardLayoutChanged
            | RuntimeEvent::SystemNotificationPermissionStatus(_)
            | RuntimeEvent::SystemNotificationAuthorization { .. } => {
                unreachable!("handled before window routing")
            }
            RuntimeEvent::Accessibility(event) => match event.window_event {
                AccessibilityWindowEvent::InitialTreeRequested => {
                    let window_title = self.config.title.as_str();
                    let window = self.window.as_mut().expect("window checked above");
                    window.accessibility_updates.activate();
                    let RuntimeWindow {
                        accessibility, ui, ..
                    } = window;
                    accessibility.update_if_active(|| ui.accessibility_update(window_title));
                }
                AccessibilityWindowEvent::ActionRequested(ActionRequest {
                    action,
                    target_node,
                    data,
                    ..
                }) => {
                    let target = self
                        .window
                        .as_ref()
                        .and_then(|window| window.ui.accessibility_element(target_node));
                    let Some(target) = target else {
                        return;
                    };
                    let previous_focus =
                        self.window.as_ref().and_then(|window| window.ui.focused());
                    match action {
                        AccessibilityAction::Focus => {
                            if let Some(window) = &mut self.window {
                                window.ui.focus(target);
                            }
                        }
                        AccessibilityAction::Blur => {
                            if let Some(window) = &mut self.window
                                && window.ui.focused() == Some(target)
                            {
                                window.ui.blur();
                            }
                        }
                        AccessibilityAction::Click => {
                            if let Some(window) = &mut self.window {
                                window.ui.focus(target);
                            }
                            self.announce_focus_change(event_loop, previous_focus);
                            self.invoke_click(event_loop, target);
                            return;
                        }
                        AccessibilityAction::SetValue => {
                            let Some(ActionData::Value(value)) = data else {
                                return;
                            };
                            let result = self
                                .window
                                .as_mut()
                                .map(|window| window.ui.input_set_value(target, &value))
                                .unwrap_or_default();
                            self.apply_input_result(event_loop, result, true);
                            return;
                        }
                        AccessibilityAction::SetTextSelection => {
                            let Some(ActionData::SetTextSelection(selection)) = data else {
                                return;
                            };
                            let result = self
                                .window
                                .as_mut()
                                .map(|window| {
                                    window
                                        .ui
                                        .set_accessibility_text_selection(target, &selection)
                                })
                                .unwrap_or_default();
                            self.apply_input_result(event_loop, result, false);
                            return;
                        }
                        _ => return,
                    }
                    self.announce_focus_change(event_loop, previous_focus);
                }
                AccessibilityWindowEvent::AccessibilityDeactivated => {
                    if let Some(window) = &mut self.window {
                        window.accessibility_updates.deactivate();
                    }
                }
            },
        })();
        self.deactivate_window();
        if released_image_capacity {
            self.resume_deferred_image_loads();
        }
        self.process_window_commands(event_loop);
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        let now = Instant::now();
        self.foreground_tasks.wake_due_timers(now);
        let window_ids = self.windows.keys().copied().collect::<Vec<_>>();
        let mut deadline = self.foreground_tasks.next_timer_deadline();
        for window_id in window_ids {
            if !self.activate_window(window_id) {
                continue;
            }
            if !self.flush_native_file_hover(event_loop) {
                self.deactivate_window();
                self.process_window_commands(event_loop);
                return;
            }
            if self
                .pending_input
                .as_ref()
                .is_some_and(|pending| pending.deadline <= now)
            {
                self.flush_pending_input(event_loop);
            }

            let (
                image_deadline,
                animation_deadline,
                scrollbar_deadline,
                tooltip_deadline,
                view_deadline,
                accessibility_deadline,
            ) = self
                .window
                .as_mut()
                .map(|state| {
                    let mut redraw = false;
                    if state.view_deadline.is_some_and(|deadline| deadline <= now) {
                        state.view_deadline = None;
                        state.view_dirty = true;
                        redraw = true;
                    }
                    if state.image_assets.announce_due_loading(now) {
                        state.view_dirty = true;
                        redraw = true;
                    }
                    if state.ui.advance_animations(now) {
                        redraw = true;
                    }
                    if state.ui.declarative_animation_due(now) {
                        state.view_dirty = true;
                        redraw = true;
                    }
                    if state.ui.advance_scrollbars(now) {
                        redraw = true;
                    }
                    if state.ui.advance_tooltips(now) {
                        redraw = true;
                    }
                    if state.accessibility_updates.advance(now) {
                        redraw = true;
                    }
                    if redraw && state.scheduler.invalidate() {
                        state.window.request_redraw();
                    }
                    (
                        state.image_assets.next_loading_deadline(),
                        state.ui.next_animation_deadline(),
                        state.ui.next_scrollbar_deadline(),
                        state.ui.next_tooltip_deadline(),
                        state.view_deadline,
                        state.accessibility_updates.deadline(),
                    )
                })
                .unwrap_or((None, None, None, None, None, None));
            let pending_deadline = self.pending_input.as_ref().map(|pending| pending.deadline);
            let window_deadline = [
                pending_deadline,
                image_deadline,
                animation_deadline,
                scrollbar_deadline,
                tooltip_deadline,
                view_deadline,
                accessibility_deadline,
            ]
            .into_iter()
            .flatten()
            .min();
            if let Some(window_deadline) = window_deadline {
                deadline = Some(
                    deadline.map_or(window_deadline, |value: Instant| value.min(window_deadline)),
                );
            }
            self.deactivate_window();
            self.process_window_commands(event_loop);
            if self.fatal_error.is_some() {
                return;
            }
        }
        if let Some(deadline) = deadline {
            event_loop.set_control_flow(ControlFlow::WaitUntil(deadline));
        } else {
            event_loop.set_control_flow(ControlFlow::Wait);
        }
    }
}

fn apply_windowed_geometry(state: &mut RuntimeWindow, bounds: Rect) {
    state.restore_bounds = bounds;
    state.logical_position = Point::new(bounds.x, bounds.y);
    state
        .window
        .set_outer_position(LogicalPosition::new(bounds.x as f64, bounds.y as f64));
    if let Some(physical) = state
        .window
        .request_inner_size(LogicalSize::new(bounds.width as f64, bounds.height as f64))
    {
        state.renderer.resize(physical.width, physical.height);
        state.logical_size = logical_window_size(physical, state.scale_factor);
    }
    state.layout_dirty = true;
    state.view_dirty |= state.listeners.observes_viewport;
}

/// Apply a resize command without sending a redundant native move for the unchanged origin.
///
/// On macOS, repeatedly calling `setFrameOrigin:` as part of a size-only animation forces extra
/// window-server work and can make the content visibly lag behind the resize wave. Exiting a
/// special state still follows the same contract as `set_window_bounds`; AppKit restores the
/// saved origin and this function changes only the requested content size.
fn apply_window_size(state: &mut RuntimeWindow, size: Size) {
    state.window.set_minimized(false);
    state.window.set_fullscreen(None);
    if state.maximized {
        state.window.set_maximized(false);
    }
    state.maximized = false;
    state.restore_bounds.width = size.width;
    state.restore_bounds.height = size.height;
    if let Some(physical) = state
        .window
        .request_inner_size(LogicalSize::new(size.width as f64, size.height as f64))
    {
        state.renderer.resize(physical.width, physical.height);
        state.logical_size = logical_window_size(physical, state.scale_factor);
    }
    state.layout_dirty = true;
    state.view_dirty |= state.listeners.observes_viewport;
}

fn apply_window_bounds(state: &mut RuntimeWindow, bounds: WindowBounds) {
    let restore = bounds.bounds();
    state.window.set_minimized(false);
    state.window.set_fullscreen(None);
    if state.maximized {
        state.window.set_maximized(false);
    }
    state.maximized = false;
    apply_windowed_geometry(state, restore);
    match bounds {
        WindowBounds::Windowed(_) => {}
        WindowBounds::Maximized(_) => {
            state.window.set_maximized(true);
            state.maximized = true;
        }
        WindowBounds::Fullscreen(_) => state
            .window
            .set_fullscreen(Some(Fullscreen::Borderless(state.window.current_monitor()))),
    }
}

fn set_runtime_window_fullscreen(state: &mut RuntimeWindow, fullscreen: bool) -> bool {
    if state.window.fullscreen().is_some() == fullscreen {
        return false;
    }
    if fullscreen {
        if !state.maximized {
            state.restore_bounds = Rect::new(
                state.logical_position.x,
                state.logical_position.y,
                state.logical_size.width,
                state.logical_size.height,
            );
        }
        if state.maximized {
            state.window.set_maximized(false);
            state.maximized = false;
        }
        state
            .window
            .set_fullscreen(Some(Fullscreen::Borderless(state.window.current_monitor())));
    } else {
        apply_window_bounds(state, WindowBounds::Windowed(state.restore_bounds));
    }
    true
}

fn constrained_window_size(size: Size, minimum: Option<Size>, maximum: Option<Size>) -> Size {
    let size = minimum.map_or(size, |minimum| {
        Size::new(
            size.width.max(minimum.width),
            size.height.max(minimum.height),
        )
    });
    maximum.map_or(size, |maximum| {
        Size::new(
            size.width.min(maximum.width),
            size.height.min(maximum.height),
        )
    })
}

fn sane_scale_factor(value: f64) -> f32 {
    if value.is_finite() && value > 0.0 {
        value as f32
    } else {
        1.0
    }
}

fn map_window_appearance(theme: Theme) -> WindowAppearance {
    match theme {
        Theme::Light => WindowAppearance::Light,
        Theme::Dark => WindowAppearance::Dark,
    }
}

fn to_winit_theme(appearance: WindowAppearance) -> Theme {
    match appearance {
        WindowAppearance::Light => Theme::Light,
        WindowAppearance::Dark => Theme::Dark,
    }
}

fn logical_window_size(physical: PhysicalSize<u32>, scale_factor: f32) -> Size {
    Size::new(
        physical.width as f32 / scale_factor,
        physical.height as f32 / scale_factor,
    )
}

fn logical_window_position(window: &Window, scale_factor: f32) -> Option<Point> {
    let position = window.outer_position().ok()?;
    Some(Point::new(
        position.x as f32 / scale_factor,
        position.y as f32 / scale_factor,
    ))
}

fn runtime_window_is_fullscreen(state: &RuntimeWindow) -> bool {
    #[cfg(target_os = "macos")]
    {
        if !runtime_window_content_attached(state) {
            state.window.fullscreen().is_some()
        } else {
            is_window_fullscreen(&state.window)
                .unwrap_or_else(|_| state.window.fullscreen().is_some())
        }
    }
    #[cfg(not(target_os = "macos"))]
    {
        state.window.fullscreen().is_some()
    }
}

fn runtime_window_is_maximized(state: &RuntimeWindow, config: &AppConfig) -> bool {
    #[cfg(target_os = "macos")]
    {
        if !runtime_window_content_attached(state)
            || config.title_bar_style == TitleBarStyle::Hidden
        {
            state.maximized
        } else {
            is_window_maximized(&state.window).unwrap_or(state.maximized)
        }
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = config;
        state.window.is_maximized()
    }
}

fn runtime_window_display_id(state: &RuntimeWindow, displays: &Displays) -> Option<DisplayId> {
    crate::display::display_for_rect(
        displays,
        Rect::new(
            state.logical_position.x,
            state.logical_position.y,
            state.logical_size.width,
            state.logical_size.height,
        ),
    )
    .or_else(|| {
        state
            .window
            .current_monitor()
            .map(|monitor| crate::display::native_display_id(&monitor))
            .filter(|id| displays.find(*id).is_some())
    })
}

fn runtime_window_content_attached(state: &RuntimeWindow) -> bool {
    #[cfg(target_os = "macos")]
    {
        state
            .first_frame_guard
            .as_ref()
            .is_none_or(MacFirstFrameGuard::content_attached)
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = state;
        true
    }
}

fn runtime_window_state(
    handle: WindowHandle,
    config: &AppConfig,
    state: &RuntimeWindow,
) -> WindowState {
    let platform_content_attached = runtime_window_content_attached(state);
    let fullscreen = runtime_window_is_fullscreen(state);
    let maximized = !fullscreen && runtime_window_is_maximized(state, config);
    let minimized = platform_content_attached && state.window.is_minimized().unwrap_or(false);
    let current_bounds = Rect::new(
        state.logical_position.x,
        state.logical_position.y,
        state.logical_size.width,
        state.logical_size.height,
    );
    let bounds = if fullscreen {
        WindowBounds::Fullscreen(state.restore_bounds)
    } else if maximized {
        WindowBounds::Maximized(state.restore_bounds)
    } else {
        WindowBounds::Windowed(current_bounds)
    };
    WindowState {
        handle,
        display_id: state.display_id,
        kind: config.kind,
        bounds,
        viewport_size: state.logical_size,
        minimum_size: config.minimum_size,
        maximum_size: config.maximum_size,
        scale_factor: state.scale_factor,
        appearance: state.appearance,
        background_appearance: config.window_background,
        focused: state.focused,
        focusable: config.focusable,
        visible: state.visible,
        minimized,
        maximized,
        fullscreen,
        occluded: state.occluded,
        movable: config.is_movable,
        resizable: config.is_resizable,
        minimizable: config.is_minimizable,
        maximizable: config.is_maximizable,
        closable: config.is_closable,
        decorated: config.decorated,
        shadow: config.shadow,
        content_protected: config.content_protected,
        window_level: effective_window_level(config),
        skip_taskbar: config.skip_taskbar,
        visible_on_all_workspaces: effective_visible_on_all_workspaces(config),
        opacity: config.opacity,
        has_icon: config.icon.is_some(),
        taskbar_progress_state: config.taskbar_progress_state,
        taskbar_progress: config.taskbar_progress,
        has_taskbar_overlay_icon: config.taskbar_overlay_icon.is_some(),
        cursor_visible: config.cursor_visible,
        cursor_grab: config.cursor_grab,
        cursor_hit_test: config.cursor_hit_test,
        cursor_position: state.pointer,
        represented_file: config.represented_file.is_some(),
        document_edited: config.document_edited,
        native_tabbing: config.tabbing_identifier.is_some(),
        native_tabs: state.native_tabs,
        #[cfg(feature = "inspector")]
        inspector_active: state.inspector.is_some(),
    }
}

fn window_buttons(config: &AppConfig) -> WindowButtons {
    let mut buttons = WindowButtons::empty();
    if config.is_closable {
        buttons |= WindowButtons::CLOSE;
    }
    if config.is_minimizable {
        buttons |= WindowButtons::MINIMIZE;
    }
    if config.is_resizable && config.is_maximizable {
        buttons |= WindowButtons::MAXIMIZE;
    }
    buttons
}

fn effective_window_level(config: &AppConfig) -> WindowLevel {
    config.window_level.unwrap_or(match config.kind {
        WindowKind::Floating | WindowKind::Popover | WindowKind::SystemPopover => {
            WindowLevel::AlwaysOnTop
        }
        WindowKind::Normal | WindowKind::Dialog => WindowLevel::Normal,
    })
}

fn effective_visible_on_all_workspaces(config: &AppConfig) -> bool {
    config.visible_on_all_workspaces
        || matches!(config.kind, WindowKind::Popover | WindowKind::SystemPopover)
}

#[cfg(any(target_os = "macos", test))]
fn window_presentation_activates_application(config: &AppConfig) -> bool {
    config.focus
        && config.focusable
        && !matches!(config.kind, WindowKind::Popover | WindowKind::SystemPopover)
}

fn winit_window_icon(image: &Image) -> Icon {
    Icon::from_rgba(image.rgba().to_vec(), image.width(), image.height())
        .expect("QuickGUI Image has already validated its RGBA dimensions")
}

fn window_dismisses_system_popover_on_escape(config: &AppConfig) -> bool {
    config.kind == WindowKind::SystemPopover
        && config
            .popover
            .as_ref()
            .is_some_and(|popover| popover.dismiss_on_escape)
}

fn window_dismisses_system_popover_on_pointer_outside(config: &AppConfig) -> bool {
    config.kind == WindowKind::SystemPopover
        && config
            .popover
            .as_ref()
            .is_some_and(|popover| popover.dismiss_on_pointer_outside)
}

fn window_is_never_key_popover(config: &AppConfig) -> bool {
    config.kind == WindowKind::SystemPopover
        && config
            .popover
            .as_ref()
            .is_some_and(|popover| !popover.accepts_key_focus)
}

#[cfg(target_os = "macos")]
fn implicit_native_movable(config: &AppConfig) -> bool {
    config.is_movable && config.title_bar_style == TitleBarStyle::Default
}

#[cfg(target_os = "macos")]
fn point_outside_viewport(point: Point, viewport: Size) -> bool {
    point.x < 0.0 || point.y < 0.0 || point.x > viewport.width || point.y > viewport.height
}

fn map_mouse_button(button: winit::event::MouseButton) -> MouseButton {
    match button {
        winit::event::MouseButton::Left => MouseButton::Left,
        winit::event::MouseButton::Right => MouseButton::Right,
        winit::event::MouseButton::Middle => MouseButton::Middle,
        winit::event::MouseButton::Back => MouseButton::Back,
        winit::event::MouseButton::Forward => MouseButton::Forward,
        winit::event::MouseButton::Other(value) => MouseButton::Other(value),
    }
}

fn platform_cursor(style: CursorStyle) -> CursorIcon {
    match style {
        CursorStyle::Arrow => CursorIcon::Default,
        CursorStyle::IBeam => CursorIcon::Text,
        CursorStyle::Crosshair => CursorIcon::Crosshair,
        CursorStyle::ClosedHand => CursorIcon::Grabbing,
        CursorStyle::OpenHand => CursorIcon::Grab,
        CursorStyle::PointingHand => CursorIcon::Pointer,
        CursorStyle::ResizeLeft => CursorIcon::WResize,
        CursorStyle::ResizeRight => CursorIcon::EResize,
        CursorStyle::ResizeLeftRight => CursorIcon::EwResize,
        CursorStyle::ResizeUp => CursorIcon::NResize,
        CursorStyle::ResizeDown => CursorIcon::SResize,
        CursorStyle::ResizeUpDown => CursorIcon::NsResize,
        CursorStyle::ResizeUpLeftDownRight => CursorIcon::NwseResize,
        CursorStyle::ResizeUpRightDownLeft => CursorIcon::NeswResize,
        CursorStyle::ResizeColumn => CursorIcon::ColResize,
        CursorStyle::ResizeRow => CursorIcon::RowResize,
        CursorStyle::IBeamCursorForVerticalLayout => CursorIcon::VerticalText,
        CursorStyle::OperationNotAllowed => CursorIcon::NotAllowed,
        CursorStyle::DragLink => CursorIcon::Alias,
        CursorStyle::DragCopy => CursorIcon::Copy,
        CursorStyle::ContextualMenu => CursorIcon::ContextMenu,
    }
}

#[cfg(feature = "inspector")]
fn reconcile_inspector_pointer_state(state: &mut RuntimeWindow) -> bool {
    state.pointer_capture = None;
    state.drag_candidate = None;
    let now = Instant::now();
    let repaint = if state.inspector.is_some() {
        state.ui.cancel_pointer_interaction()
            | state.ui.pointer_left()
            | state.ui.update_scrollbar_hover(None, now)
    } else {
        let scrollbar_changed = state.ui.update_scrollbar_hover(state.pointer, now);
        let pointer_changed = state.pointer.is_some_and(|point| {
            let RuntimeWindow { ui, renderer, .. } = state;
            ui.pointer_moved(point, renderer)
        });
        scrollbar_changed | pointer_changed
    };
    let cursor = state
        .pointer
        .map_or(CursorIcon::Default, |point| desired_cursor(state, point));
    set_cursor_if_changed(state, cursor);
    repaint
}

fn desired_cursor(state: &RuntimeWindow, point: Point) -> CursorIcon {
    #[cfg(feature = "inspector")]
    if let Some(inspector) = &state.inspector
        && inspector.captures_pointer(point)
    {
        return if inspector.mode() == InspectorMode::Picking && !inspector.panel_contains(point) {
            CursorIcon::Crosshair
        } else {
            CursorIcon::Default
        };
    }
    if state.drag_session.is_some() {
        return CursorIcon::Grabbing;
    }
    if state.ui.scrollbar_drag_active()
        || (state.pointer_capture.is_none()
            && (state.ui.is_over_scrollbar(point) || state.ui.is_app_region_drag(point)))
    {
        return CursorIcon::Default;
    }
    state
        .ui
        .cursor_style_at(point)
        .map_or(CursorIcon::Default, platform_cursor)
}

fn set_cursor_if_changed(state: &mut RuntimeWindow, cursor: CursorIcon) {
    if cursor != state.cursor {
        state.cursor = cursor;
        state.window.set_cursor(cursor);
    }
}

fn map_gesture_phase(phase: winit::event::TouchPhase) -> GesturePhase {
    match phase {
        winit::event::TouchPhase::Started => GesturePhase::Started,
        winit::event::TouchPhase::Moved => GesturePhase::Moved,
        winit::event::TouchPhase::Ended => GesturePhase::Ended,
        winit::event::TouchPhase::Cancelled => GesturePhase::Cancelled,
    }
}

fn map_touch_phase(phase: winit::event::TouchPhase) -> TouchPhase {
    match phase {
        winit::event::TouchPhase::Started => TouchPhase::Started,
        winit::event::TouchPhase::Moved => TouchPhase::Moved,
        winit::event::TouchPhase::Ended => TouchPhase::Ended,
        winit::event::TouchPhase::Cancelled => TouchPhase::Cancelled,
    }
}

fn bounded_touch_force(force: Force) -> f32 {
    bounded_pressure(force.normalized() as f32)
}

fn bounded_pressure(pressure: f32) -> f32 {
    if pressure.is_finite() {
        pressure.clamp(0.0, 1.0)
    } else {
        0.0
    }
}

fn bounded_gesture_delta(delta: f64, limit: f32) -> f32 {
    if delta.is_finite() {
        (delta as f32).clamp(-limit, limit)
    } else {
        0.0
    }
}

fn map_pressure_stage(stage: i64) -> PressureStage {
    match stage {
        0 => PressureStage::Zero,
        1 => PressureStage::Normal,
        2 => PressureStage::Force,
        stage => PressureStage::Other(stage),
    }
}

fn map_modifiers(state: ModifiersState) -> Modifiers {
    let mut result = Modifiers::empty();
    result.set(Modifiers::SHIFT, state.shift_key());
    result.set(Modifiers::CONTROL, state.control_key());
    result.set(Modifiers::ALT, state.alt_key());
    result.set(Modifiers::SUPER, state.super_key());
    result
}

fn primary_modifier(modifiers: Modifiers) -> bool {
    if cfg!(target_os = "macos") {
        modifiers.contains(Modifiers::SUPER)
    } else {
        modifiers.contains(Modifiers::CONTROL)
    }
}

fn word_modifier(modifiers: Modifiers) -> bool {
    if cfg!(target_os = "macos") {
        modifiers.contains(Modifiers::ALT)
    } else {
        modifiers.contains(Modifiers::CONTROL)
    }
}

#[cfg(target_os = "macos")]
fn is_default_close_shortcut(key: &Key, modifiers: Modifiers, repeat: bool) -> bool {
    !repeat
        && modifiers == Modifiers::SUPER
        && matches!(key, Key::Character(value) if value.eq_ignore_ascii_case("w"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cursor_styles_map_to_the_matching_platform_cursor() {
        let cases = [
            (CursorStyle::Arrow, CursorIcon::Default),
            (CursorStyle::IBeam, CursorIcon::Text),
            (CursorStyle::Crosshair, CursorIcon::Crosshair),
            (CursorStyle::ClosedHand, CursorIcon::Grabbing),
            (CursorStyle::OpenHand, CursorIcon::Grab),
            (CursorStyle::PointingHand, CursorIcon::Pointer),
            (CursorStyle::ResizeLeft, CursorIcon::WResize),
            (CursorStyle::ResizeRight, CursorIcon::EResize),
            (CursorStyle::ResizeLeftRight, CursorIcon::EwResize),
            (CursorStyle::ResizeUp, CursorIcon::NResize),
            (CursorStyle::ResizeDown, CursorIcon::SResize),
            (CursorStyle::ResizeUpDown, CursorIcon::NsResize),
            (CursorStyle::ResizeUpLeftDownRight, CursorIcon::NwseResize),
            (CursorStyle::ResizeUpRightDownLeft, CursorIcon::NeswResize),
            (CursorStyle::ResizeColumn, CursorIcon::ColResize),
            (CursorStyle::ResizeRow, CursorIcon::RowResize),
            (
                CursorStyle::IBeamCursorForVerticalLayout,
                CursorIcon::VerticalText,
            ),
            (CursorStyle::OperationNotAllowed, CursorIcon::NotAllowed),
            (CursorStyle::DragLink, CursorIcon::Alias),
            (CursorStyle::DragCopy, CursorIcon::Copy),
            (CursorStyle::ContextualMenu, CursorIcon::ContextMenu),
        ];

        for (style, expected) in cases {
            assert_eq!(platform_cursor(style), expected);
        }
    }

    struct CounterView(u32);

    struct RuntimeEmitter;

    struct RuntimeEntityEvent(u32);

    impl EventEmitter<RuntimeEntityEvent> for RuntimeEmitter {}

    struct RuntimeGlobal(u32);

    impl Global for RuntimeGlobal {}

    struct AnotherRuntimeGlobal;

    impl Global for AnotherRuntimeGlobal {}

    impl View for CounterView {
        fn render(&mut self, cx: &mut ViewContext<'_, Self>) -> impl IntoElement {
            let increment = cx.listener("increment", |this, cx| {
                this.0 += 1;
                cx.invalidate();
            });
            crate::div().on_click(increment)
        }
    }

    #[test]
    fn erased_views_keep_typed_listener_state() {
        let mut view = CounterView(0);
        let listener: ClickCallback = Arc::new(|view, cx| {
            let view = view
                .downcast_mut::<CounterView>()
                .expect("click listener received the wrong view type");
            view.0 += 1;
            cx.invalidate();
        });
        let mut cx = EventContext::default();
        listener(&mut view, &mut cx);

        assert_eq!(view.0, 1);
        assert!(cx.invalidate);
    }

    #[test]
    fn retained_entity_observations_are_exact_and_refresh_per_render() {
        let first = Entity::new(1_u32);
        let second = Entity::new(2_u32);
        let mut listeners = ListenerRegistry::default();

        listeners.observe_entity(first.id());
        listeners.observe_entity(first.id());
        assert_eq!(listeners.observed_entities.len(), 1);
        assert!(listeners.observes_entity_change(&[first.id()], false));
        assert!(!listeners.observes_entity_change(&[second.id()], false));
        assert!(listeners.observes_entity_change(&[], true));

        listeners.clear();
        assert!(!listeners.observes_entity_change(&[first.id()], false));
    }

    #[test]
    fn app_builder_retains_typed_globals_for_the_runtime() {
        let app = App::new(CounterView(0)).global(RuntimeGlobal(7));

        assert!(app.globals.has::<RuntimeGlobal>());
        assert_eq!(app.globals.get::<RuntimeGlobal>().0, 7);
    }

    #[test]
    fn app_builder_retains_only_explicit_application_callbacks() {
        let app = App::new(CounterView(0))
            .on_open_urls(|_, _| {})
            .on_reopen(|_, _| {})
            .on_system_wake(|_| {})
            .on_keyboard_layout_change(|_, _| {})
            .on_system_notification_response(|_, _| {})
            .on_window_closed(|_, _| {});

        assert!(app.application_callbacks.open_urls.is_some());
        assert!(app.application_callbacks.reopen.is_some());
        assert!(app.application_callbacks.system_wake.is_some());
        assert!(app.application_callbacks.keyboard_layout.is_some());
        assert!(
            app.application_callbacks
                .system_notification_response
                .is_some()
        );
        assert!(app.application_callbacks.window_closed.is_some());

        let app = App::new(CounterView(0));
        assert!(app.application_callbacks.open_urls.is_none());
        assert!(app.application_callbacks.reopen.is_none());
        assert!(app.application_callbacks.system_wake.is_none());
        assert!(app.application_callbacks.keyboard_layout.is_none());
        assert!(
            app.application_callbacks
                .system_notification_response
                .is_none()
        );
        assert!(app.application_callbacks.window_closed.is_none());
    }

    #[test]
    fn windowless_application_builder_retains_core_lifecycle_configuration() {
        let application = Application::new()
            .with_quit_mode(QuitMode::Explicit)
            .global(RuntimeGlobal(9))
            .on_open_urls(|_, _| {})
            .on_window_closed(|_, _| {});

        assert_eq!(application.quit_mode, QuitMode::Explicit);
        assert_eq!(application.globals.get::<RuntimeGlobal>().0, 9);
        assert!(application.application_callbacks.open_urls.is_some());
        assert!(application.application_callbacks.window_closed.is_some());
    }

    #[test]
    fn quit_mode_default_matches_native_desktop_convention() {
        assert_eq!(
            QuitMode::Default.quits_when_empty(),
            cfg!(not(target_os = "macos"))
        );
        assert!(QuitMode::LastWindowClosed.quits_when_empty());
        assert!(!QuitMode::Explicit.quits_when_empty());
        assert_eq!(
            App::new(CounterView(0))
                .with_quit_mode(QuitMode::Explicit)
                .quit_mode,
            QuitMode::Explicit
        );
    }

    #[test]
    fn retained_global_observations_are_exact_and_refresh_per_render() {
        let first = TypeId::of::<RuntimeGlobal>();
        let second = TypeId::of::<AnotherRuntimeGlobal>();
        let mut listeners = ListenerRegistry::default();

        listeners.observe_global(first);
        listeners.observe_global(first);
        assert_eq!(listeners.observed_globals.len(), 1);
        assert!(listeners.observes_global_change(&[first], false));
        assert!(!listeners.observes_global_change(&[second], false));
        assert!(listeners.observes_global_change(&[], true));

        listeners.clear();
        assert!(!listeners.observes_global_change(&[first], false));
    }

    #[test]
    fn window_option_builders_preserve_restore_geometry_and_state() {
        let bounds = Rect::new(120.0, 80.0, 640.0, 420.0);
        let options = WindowOptions::new("Inspector")
            .window_bounds(WindowBounds::Fullscreen(bounds))
            .position(200.0, 140.0)
            .size(720.0, 480.0)
            .maximized(true);

        assert_eq!(options.size, Size::new(720.0, 480.0));
        assert_eq!(
            options.window_bounds,
            Some(WindowBounds::Maximized(Rect::new(
                200.0, 140.0, 720.0, 480.0,
            )))
        );
    }

    #[test]
    fn minimum_window_size_bounds_runtime_growth_without_rewriting_explicit_geometry() {
        let minimum = Size::new(640.0, 420.0);
        assert_eq!(
            constrained_window_size(Size::new(320.0, 800.0), Some(minimum), None),
            Size::new(640.0, 800.0)
        );
        assert_eq!(
            App::new(CounterView(0))
                .minimum_size(500.0, 360.0)
                .config
                .minimum_size,
            Some(Size::new(500.0, 360.0))
        );
        assert!(
            App::new(CounterView(0))
                .without_minimum_size()
                .config
                .minimum_size
                .is_none()
        );
    }

    #[test]
    fn window_appearance_builders_and_native_mapping_are_exact() {
        let forced = WindowOptions::new("Inspector").window_appearance(WindowAppearance::Dark);
        assert_eq!(forced.preferred_appearance, Some(WindowAppearance::Dark));
        assert_eq!(forced.follow_system_appearance().preferred_appearance, None);

        for appearance in [WindowAppearance::Light, WindowAppearance::Dark] {
            assert_eq!(
                map_window_appearance(to_winit_theme(appearance)),
                appearance
            );
        }
    }

    #[test]
    fn window_background_builder_retains_the_compositor_policy() {
        let options =
            WindowOptions::new("Palette").window_background(WindowBackgroundAppearance::Blurred);
        assert_eq!(
            options.window_background,
            WindowBackgroundAppearance::Blurred
        );
        assert!(options.window_background.is_transparent());
        assert!(options.window_background.is_blurred());
        assert!(!WindowBackgroundAppearance::Opaque.is_transparent());

        assert_eq!(
            WindowBackgroundAppearance::Transparent
                .changes_from(WindowBackgroundAppearance::Blurred),
            WindowBackgroundChanges {
                transparency: false,
                blur: true,
            }
        );
        assert_eq!(
            WindowBackgroundAppearance::Opaque
                .changes_from(WindowBackgroundAppearance::Transparent),
            WindowBackgroundChanges {
                transparency: true,
                blur: false,
            }
        );
        assert_eq!(
            WindowBackgroundAppearance::Blurred.changes_from(WindowBackgroundAppearance::Opaque),
            WindowBackgroundChanges {
                transparency: true,
                blur: true,
            }
        );
    }

    #[test]
    fn focused_top_level_presentation_activates_the_application_but_popovers_do_not() {
        for kind in [WindowKind::Normal, WindowKind::Floating, WindowKind::Dialog] {
            let options = WindowOptions::new("Window").window_kind(kind);
            assert!(window_presentation_activates_application(&options));
        }

        for kind in [WindowKind::Popover, WindowKind::SystemPopover] {
            let options = WindowOptions::new("Popover").window_kind(kind);
            assert!(!window_presentation_activates_application(&options));
        }

        assert!(!window_presentation_activates_application(
            &WindowOptions::new("Inactive").focus(false)
        ));
        assert!(!window_presentation_activates_application(
            &WindowOptions::new("Never key").focusable(false)
        ));
    }

    #[test]
    fn system_popover_builder_selects_bounded_native_menu_defaults() {
        let popover = crate::PopoverOptions::new(Rect::new(24.0, 40.0, 120.0, 32.0));
        let options = WindowOptions::new("Menu")
            .size(240.0, 180.0)
            .system_popover(popover.clone());

        assert_eq!(options.kind, WindowKind::SystemPopover);
        assert_eq!(options.popover, Some(popover));
        assert_eq!(options.title_bar_style, TitleBarStyle::Hidden);
        assert!(options.focus);
        assert!(!options.is_movable);
        assert!(!options.is_resizable);
        assert!(!options.is_minimizable);
        assert!(options.minimum_size.is_none());
        assert_eq!(validate_window_options(&options), Ok(()));

        let never_key_popover = crate::PopoverOptions::new(Rect::ZERO)
            .grab(false)
            .accepts_key_focus(false);
        let never_key = WindowOptions::new("Suggestions").system_popover(never_key_popover);
        assert!(!never_key.focus);
        assert!(window_is_never_key_popover(&never_key));
        assert!(!never_key.popover.as_ref().unwrap().grab);
        assert_eq!(validate_window_options(&never_key), Ok(()));

        let escape_only = WindowOptions::new("Escape only").system_popover(
            crate::PopoverOptions::new(Rect::ZERO)
                .dismiss_on_escape(true)
                .dismiss_on_pointer_outside(false),
        );
        assert!(window_dismisses_system_popover_on_escape(&escape_only));
        assert!(!window_dismisses_system_popover_on_pointer_outside(
            &escape_only
        ));
    }

    #[test]
    fn window_options_reject_unbounded_native_inputs() {
        assert_eq!(
            validate_window_bounds(WindowBounds::windowed(f32::NAN, 0.0, 1.0, 1.0)),
            Err(WindowCommandError::InvalidBounds)
        );
        assert_eq!(
            validate_window_size(Size::new(0.0, 100.0)),
            Err(WindowCommandError::InvalidBounds)
        );
        assert_eq!(
            validate_window_position(Point::new(MAX_WINDOW_LOGICAL_COORDINATE * 2.0, 0.0)),
            Err(WindowCommandError::InvalidBounds)
        );
        assert_eq!(
            validate_window_title(&"x".repeat(MAX_WINDOW_TITLE_BYTES + 1)),
            Err(WindowCommandError::TitleTooLong)
        );
        assert_eq!(
            validate_window_options(
                &WindowOptions::new("Hidden")
                    .title_bar_style(TitleBarStyle::Hidden)
                    .traffic_light_position(12.0, 12.0),
            ),
            Err(WindowCommandError::HiddenTitleBarTrafficLights)
        );
        assert_eq!(
            validate_window_options(
                &WindowOptions::new("Missing popover").window_kind(WindowKind::SystemPopover),
            ),
            Err(WindowCommandError::InvalidPopoverConfiguration)
        );
        assert_eq!(
            validate_window_options(&WindowOptions::new("Invalid popover").system_popover(
                crate::PopoverOptions::new(Rect::new(f32::INFINITY, 0.0, 0.0, 0.0)),
            ),),
            Err(WindowCommandError::InvalidPopoverConfiguration)
        );
    }

    #[test]
    fn document_window_options_are_bounded_and_keep_gpui_aliases() {
        let options = WindowOptions::new("Document")
            .document_path("Cargo.toml")
            .document_edited(true)
            .tabbing_identifier("dev.quickgui.workspace");
        assert_eq!(options.represented_file, Some(PathBuf::from("Cargo.toml")));
        assert!(options.document_edited);
        assert_eq!(
            options.tabbing_identifier.as_deref(),
            Some("dev.quickgui.workspace")
        );
        assert_eq!(validate_window_options(&options), Ok(()));

        assert_eq!(
            validate_window_options(&WindowOptions::new("Empty path").document_path("")),
            Err(WindowCommandError::InvalidDocumentPath)
        );
        assert_eq!(
            validate_window_options(&WindowOptions::new("NUL path").document_path("a\0b")),
            Err(WindowCommandError::InvalidDocumentPath)
        );
        assert_eq!(
            validate_window_options(&WindowOptions::new("Empty tab").tabbing_identifier("")),
            Err(WindowCommandError::InvalidTabbingIdentifier)
        );
        assert_eq!(
            validate_window_options(
                &WindowOptions::new("Long tab")
                    .tabbing_identifier("x".repeat(MAX_WINDOW_TABBING_IDENTIFIER_BYTES + 1)),
            ),
            Err(WindowCommandError::InvalidTabbingIdentifier)
        );

        assert!(WindowTabState::default().is_valid());
        assert!(
            !WindowTabState {
                count: 0,
                selected_index: None,
                ..WindowTabState::default()
            }
            .is_valid()
        );
        assert!(
            !WindowTabState {
                count: 2,
                selected_index: Some(2),
                ..WindowTabState::default()
            }
            .is_valid()
        );

        let app = App::new(CounterView(0))
            .represented_file("src/lib.rs")
            .document_edited(true)
            .tabbing_identifier("dev.quickgui.source");
        assert_eq!(
            app.config.represented_file,
            Some(PathBuf::from("src/lib.rs"))
        );
        assert!(app.config.document_edited);
        assert_eq!(
            app.config.tabbing_identifier.as_deref(),
            Some("dev.quickgui.source")
        );
    }

    #[test]
    fn window_state_observation_is_declarative() {
        let mut listeners = ListenerRegistry {
            observes_window_state: true,
            observes_viewport: true,
            ..ListenerRegistry::default()
        };

        assert!(listeners.requires_window_state_rebuild(true));
        assert!(!listeners.requires_window_state_rebuild(false));

        listeners.clear();

        assert!(!listeners.observes_window_state);
        assert!(!listeners.observes_viewport);
        assert!(!listeners.requires_window_state_rebuild(true));
    }

    #[test]
    fn accessibility_geometry_updates_are_coalesced_without_an_idle_loop() {
        let now = Instant::now();
        let mut updates = AccessibilityUpdateSchedule::default();

        assert_eq!(
            updates.should_update(Some(AccessibilityUpdateKind::ScrollGeometry), now),
            None
        );
        assert_eq!(updates.deadline(), None);
        updates.activate();
        assert_eq!(
            updates.should_update(Some(AccessibilityUpdateKind::ScrollGeometry), now),
            None
        );
        assert_eq!(
            updates.deadline(),
            Some(now + ACCESSIBILITY_GEOMETRY_UPDATE_INTERVAL)
        );
        assert!(
            !updates
                .advance(now + ACCESSIBILITY_GEOMETRY_UPDATE_INTERVAL - Duration::from_millis(1))
        );
        assert!(updates.advance(now + ACCESSIBILITY_GEOMETRY_UPDATE_INTERVAL));
        assert_eq!(
            updates.should_update(None, now + ACCESSIBILITY_GEOMETRY_UPDATE_INTERVAL),
            Some(AccessibilityUpdateKind::ScrollGeometry),
            "the timer correction must retain the incremental scroll update kind"
        );
        assert_eq!(updates.deadline(), None);

        assert_eq!(
            updates.should_update(
                Some(AccessibilityUpdateKind::LayoutGeometry),
                now + ACCESSIBILITY_GEOMETRY_UPDATE_INTERVAL
            ),
            None
        );
        assert_eq!(
            updates.deadline(),
            Some(now + ACCESSIBILITY_GEOMETRY_UPDATE_INTERVAL * 2)
        );
        let later_resize = now
            + ACCESSIBILITY_GEOMETRY_UPDATE_INTERVAL
            + ACCESSIBILITY_GEOMETRY_UPDATE_INTERVAL / 2;
        assert_eq!(
            updates.should_update(Some(AccessibilityUpdateKind::LayoutGeometry), later_resize),
            None
        );
        assert_eq!(
            updates.deadline(),
            Some(later_resize + ACCESSIBILITY_GEOMETRY_UPDATE_INTERVAL),
            "continuous resize frames move the trailing correction deadline"
        );
        assert!(!updates.advance(now + ACCESSIBILITY_GEOMETRY_UPDATE_INTERVAL * 2));
        assert!(updates.advance(later_resize + ACCESSIBILITY_GEOMETRY_UPDATE_INTERVAL));
        assert_eq!(
            updates.should_update(None, later_resize + ACCESSIBILITY_GEOMETRY_UPDATE_INTERVAL),
            Some(AccessibilityUpdateKind::LayoutGeometry)
        );

        assert_eq!(
            updates.should_update(None, now),
            Some(AccessibilityUpdateKind::Full)
        );
        assert_eq!(updates.deadline(), None);
        assert!(!updates.advance(now + Duration::from_secs(1)));

        updates.deactivate();
        assert_eq!(updates.should_update(None, now), None);
    }

    #[test]
    fn platform_effect_queue_completes_overflow_without_retaining_it() {
        let mut pending = (0..crate::MAX_PENDING_PLATFORM_REQUESTS)
            .map(|index| {
                PlatformRequest::open_url(format!("https://example.com/{index}"))
                    .expect("bounded test URL")
            })
            .collect::<VecDeque<_>>();
        let (overflow, mut response) = PlatformRequest::prompt(
            WindowHandle::next(),
            crate::PromptLevel::Info,
            "Overflow",
            None,
            &[crate::PromptButton::ok("OK")],
        )
        .expect("valid prompt");
        let mut incoming = vec![overflow];

        enqueue_platform_requests(&mut pending, &mut incoming);

        assert_eq!(pending.len(), crate::MAX_PENDING_PLATFORM_REQUESTS);
        assert!(incoming.is_empty());
        let mut context = std::task::Context::from_waker(std::task::Waker::noop());
        assert_eq!(
            std::pin::Pin::new(&mut response).poll(&mut context),
            std::task::Poll::Ready(Err(PlatformError::PendingQueueFull))
        );
    }

    #[test]
    fn global_subscriptions_preserve_order_and_cancel_on_drop() {
        let global_type = TypeId::of::<RuntimeGlobal>();
        let deliveries = Rc::new(RefCell::new(Vec::new()));
        let mut listeners = ListenerRegistry::default();

        let callback = |label, deliveries: Rc<RefCell<Vec<&'static str>>>| {
            Rc::new(RefCell::new(
                move |_view: &mut dyn Any, _cx: &mut EventContext| {
                    deliveries.borrow_mut().push(label);
                },
            )) as GlobalObserverCallback
        };
        let first = listeners.subscribe_global(global_type, callback("first", deliveries.clone()));
        let second =
            listeners.subscribe_global(global_type, callback("second", deliveries.clone()));

        assert!(listeners.has_global_subscribers(Some(global_type)));
        for subscription in listeners.global_subscriptions(Some(global_type)) {
            if subscription.is_active() {
                subscription.callback.borrow_mut()(&mut (), &mut EventContext::default());
            }
        }
        assert_eq!(*deliveries.borrow(), ["first", "second"]);

        drop(first);
        listeners.prune_global_subscriptions();
        assert_eq!(listeners.global_observers.len(), 1);
        deliveries.borrow_mut().clear();
        for subscription in listeners.global_subscriptions(Some(global_type)) {
            if subscription.is_active() {
                subscription.callback.borrow_mut()(&mut (), &mut EventContext::default());
            }
        }
        assert_eq!(*deliveries.borrow(), ["second"]);
        drop(second);
        assert!(!listeners.has_global_subscribers(Some(global_type)));
    }

    #[test]
    fn dropping_a_global_subscription_cancels_its_snapshotted_delivery() {
        let global_type = TypeId::of::<RuntimeGlobal>();
        let deliveries = Rc::new(RefCell::new(Vec::new()));
        let second_handle = Rc::new(RefCell::new(None));
        let mut listeners = ListenerRegistry::default();

        let first_deliveries = deliveries.clone();
        let first_second_handle = second_handle.clone();
        let first_callback: GlobalObserverCallback = Rc::new(RefCell::new(
            move |_view: &mut dyn Any, _cx: &mut EventContext| {
                first_deliveries.borrow_mut().push("first");
                first_second_handle.borrow_mut().take();
            },
        ));
        let second_deliveries = deliveries.clone();
        let second_callback: GlobalObserverCallback = Rc::new(RefCell::new(
            move |_view: &mut dyn Any, _cx: &mut EventContext| {
                second_deliveries.borrow_mut().push("second");
            },
        ));
        let _first = listeners.subscribe_global(global_type, first_callback);
        *second_handle.borrow_mut() =
            Some(listeners.subscribe_global(global_type, second_callback));

        for subscription in listeners.global_subscriptions(Some(global_type)) {
            if subscription.is_active() {
                subscription.callback.borrow_mut()(&mut (), &mut EventContext::default());
            }
        }

        assert_eq!(*deliveries.borrow(), ["first"]);
    }

    #[test]
    fn global_subscription_capacity_reuses_a_dropped_slot_without_growing() {
        let global_type = TypeId::of::<RuntimeGlobal>();
        let callback: GlobalObserverCallback = Rc::new(RefCell::new(
            |_view: &mut dyn Any, _cx: &mut EventContext| {},
        ));
        let mut listeners = ListenerRegistry::default();
        let mut subscriptions = Vec::with_capacity(MAX_GLOBAL_SUBSCRIPTIONS_PER_WINDOW);

        for _ in 0..MAX_GLOBAL_SUBSCRIPTIONS_PER_WINDOW {
            subscriptions.push(listeners.subscribe_global(global_type, callback.clone()));
        }
        assert_eq!(
            listeners.global_observers.len(),
            MAX_GLOBAL_SUBSCRIPTIONS_PER_WINDOW
        );

        drop(subscriptions.pop());
        subscriptions.push(listeners.subscribe_global(global_type, callback));
        assert_eq!(
            listeners.global_observers.len(),
            MAX_GLOBAL_SUBSCRIPTIONS_PER_WINDOW
        );
        assert_eq!(
            listeners.global_subscriptions(Some(global_type)).len(),
            MAX_GLOBAL_SUBSCRIPTIONS_PER_WINDOW
        );
    }

    #[test]
    fn pending_global_notifications_coalesce_and_all_supersedes_exact_types() {
        let first = TypeId::of::<RuntimeGlobal>();
        let second = TypeId::of::<AnotherRuntimeGlobal>();
        let mut pending = VecDeque::new();
        let mut pending_types = HashSet::new();
        let mut pending_all = false;

        assert!(enqueue_global_notifications(
            &mut pending,
            &mut pending_types,
            &mut pending_all,
            &[first, first, second],
            false,
        ));
        assert_eq!(pending.iter().copied().collect::<Vec<_>>(), [first, second]);
        assert_eq!(pending_types.len(), 2);

        assert!(enqueue_global_notifications(
            &mut pending,
            &mut pending_types,
            &mut pending_all,
            &[],
            true,
        ));
        assert!(pending.is_empty());
        assert!(pending_types.is_empty());
        assert!(pending_all);
    }

    #[test]
    fn entity_event_subscriptions_preserve_order_and_cancel_on_drop() {
        let source = Entity::new(());
        let event_type = TypeId::of::<u32>();
        let deliveries = Rc::new(RefCell::new(Vec::new()));
        let mut listeners = ListenerRegistry::default();

        let callback = |label, deliveries: Rc<RefCell<Vec<&'static str>>>| {
            Rc::new(RefCell::new(
                move |_view: &mut dyn Any, event: &dyn Any, _cx: &mut EventContext| {
                    assert_eq!(event.downcast_ref::<u32>(), Some(&7));
                    deliveries.borrow_mut().push(label);
                },
            )) as EntityEventCallback
        };
        let first = listeners.subscribe_entity_event(
            source.id(),
            event_type,
            callback("first", deliveries.clone()),
        );
        let second = listeners.subscribe_entity_event(
            source.id(),
            event_type,
            callback("second", deliveries.clone()),
        );

        assert!(listeners.has_entity_event_subscribers(source.id(), event_type));
        assert_eq!(listeners.entity_subscription_count, 2);
        for callback in listeners.entity_event_callbacks(source.id(), event_type) {
            callback.borrow_mut()(&mut (), &7_u32, &mut EventContext::default());
        }
        assert_eq!(*deliveries.borrow(), ["first", "second"]);

        drop(first);
        assert_eq!(
            listeners
                .entity_event_callbacks(source.id(), event_type)
                .len(),
            1
        );
        listeners.clear();
        assert_eq!(listeners.entity_subscription_count, 1);

        drop(second);
        assert!(!listeners.has_entity_event_subscribers(source.id(), event_type));
        listeners.clear();
        assert!(listeners.entity_events.is_empty());
        assert_eq!(listeners.entity_subscription_count, 0);
    }

    #[test]
    fn pending_entity_events_and_recursive_delivery_are_hard_bounded() {
        let source = Entity::new(RuntimeEmitter);
        let mut pending = VecDeque::new();
        let mut incoming = (0..MAX_PENDING_ENTITY_EVENTS)
            .map(|value| EntityEvent::new(&source, RuntimeEntityEvent(value as u32)))
            .collect::<Vec<_>>();

        assert!(enqueue_entity_events(&mut pending, &mut incoming));
        assert!(incoming.is_empty());
        assert_eq!(pending.len(), MAX_PENDING_ENTITY_EVENTS);
        assert_eq!(
            pending
                .front()
                .and_then(|event| event.value.downcast_ref::<RuntimeEntityEvent>())
                .map(|event| event.0),
            Some(0)
        );

        let mut overflow = vec![EntityEvent::new(&source, RuntimeEntityEvent(u32::MAX))];
        assert!(!enqueue_entity_events(&mut pending, &mut overflow));
        assert_eq!(pending.len(), MAX_PENDING_ENTITY_EVENTS);
        assert_eq!(overflow.len(), 1);

        let mut deliveries = MAX_ENTITY_EVENT_DELIVERIES_PER_TURN - 1;
        assert!(reserve_entity_event_delivery(&mut deliveries));
        assert_eq!(deliveries, MAX_ENTITY_EVENT_DELIVERIES_PER_TURN);
        assert!(!reserve_entity_event_delivery(&mut deliveries));
        assert_eq!(deliveries, MAX_ENTITY_EVENT_DELIVERIES_PER_TURN);
    }

    #[test]
    fn subscription_capacity_reuses_a_dropped_slot_without_growing() {
        let source = Entity::new(());
        let event_type = TypeId::of::<u8>();
        let callback: EntityEventCallback = Rc::new(RefCell::new(
            |_view: &mut dyn Any, _event: &dyn Any, _cx: &mut EventContext| {},
        ));
        let mut listeners = ListenerRegistry::default();
        let mut subscriptions = Vec::with_capacity(MAX_ENTITY_SUBSCRIPTIONS_PER_WINDOW);

        for _ in 0..MAX_ENTITY_SUBSCRIPTIONS_PER_WINDOW {
            subscriptions.push(listeners.subscribe_entity_event(
                source.id(),
                event_type,
                callback.clone(),
            ));
        }
        assert_eq!(
            listeners.entity_subscription_count,
            MAX_ENTITY_SUBSCRIPTIONS_PER_WINDOW
        );

        drop(subscriptions.pop());
        subscriptions.push(listeners.subscribe_entity_event(source.id(), event_type, callback));
        assert_eq!(
            listeners.entity_subscription_count,
            MAX_ENTITY_SUBSCRIPTIONS_PER_WINDOW
        );
        assert_eq!(
            listeners
                .entity_event_callbacks(source.id(), event_type)
                .len(),
            MAX_ENTITY_SUBSCRIPTIONS_PER_WINDOW
        );
    }

    #[test]
    fn erased_drag_payloads_preserve_their_exact_type_and_preview() {
        let files = FileDragPaths::new([(PathBuf::from("Cargo.toml"), false)]);
        let drag = AnyDrag::new(
            Drag::new(42_u32)
                .preview(crate::div().size(80.0, 40.0))
                .external_files(files.clone()),
        );

        assert_eq!(drag.value_type, TypeId::of::<u32>());
        assert_eq!(drag.value.downcast_ref::<u32>(), Some(&42));
        assert!(drag.preview.is_some());
        assert_eq!(
            drag.external_payload,
            Some(ExternalDragPayload::Files(files))
        );

        let text = AnyDrag::new(Drag::new(7_u8).external_text("QuickGUI native text"));
        assert_eq!(
            text.external_payload,
            Some(ExternalDragPayload::Text(ExternalDragText::new(
                "QuickGUI native text"
            )))
        );

        let url = ExternalDragUrl::new("https://quickgui.dev/").unwrap();
        let drag = AnyDrag::new(Drag::new(9_u8).external_url(url.clone()));
        assert_eq!(drag.external_payload, Some(ExternalDragPayload::Url(url)));
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn external_drag_promotion_starts_only_after_leaving_the_viewport() {
        let viewport = Size::new(800.0, 600.0);
        assert!(!point_outside_viewport(Point::new(0.0, 0.0), viewport));
        assert!(!point_outside_viewport(Point::new(800.0, 600.0), viewport));
        assert!(point_outside_viewport(Point::new(-0.1, 300.0), viewport));
        assert!(point_outside_viewport(Point::new(400.0, 600.1), viewport));
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn native_typed_drop_origin_distinguishes_same_and_cross_window_delivery() {
        let source_window = WindowHandle::next();
        let destination_window = WindowHandle::next();
        let source = ElementId::named("source");
        let typed = MacNativeDropPayload::Typed(MacTypedDragPayload::new(
            Arc::new(42_u32),
            TypeId::of::<u32>(),
            source_window,
            source,
        ));
        assert_eq!(
            native_drop_origin(Some(source_window), &typed),
            DragOrigin::Internal(source)
        );
        assert_eq!(
            native_drop_origin(Some(destination_window), &typed),
            DragOrigin::CrossWindow {
                window: source_window,
                source,
            }
        );

        let text = MacNativeDropPayload::Text(Arc::new(ExternalDragText::new("native")));
        assert_eq!(
            native_drop_origin(Some(destination_window), &text),
            DragOrigin::External
        );
    }

    #[test]
    fn native_file_events_are_grouped_until_every_hovered_path_drops() {
        let mut drag = NativeFileDrag::default();
        drag.hover(PathBuf::from("first.txt"));
        drag.hover(PathBuf::from("second.txt"));

        assert!(!drag.drop_path(PathBuf::from("first.txt")));
        assert!(drag.drop_path(PathBuf::from("second.txt")));
        assert_eq!(
            drag.into_dropped_files().paths(),
            [PathBuf::from("first.txt"), PathBuf::from("second.txt")]
        );
    }

    #[test]
    fn invalid_scale_factors_fall_back_to_one() {
        assert_eq!(sane_scale_factor(0.0), 1.0);
        assert_eq!(sane_scale_factor(f64::NAN), 1.0);
        assert_eq!(sane_scale_factor(2.0), 2.0);
    }

    #[test]
    fn native_gesture_values_are_finite_bounded_and_semantic() {
        assert_eq!(bounded_pressure(-1.0), 0.0);
        assert_eq!(bounded_pressure(0.625), 0.625);
        assert_eq!(bounded_pressure(2.0), 1.0);
        assert_eq!(bounded_pressure(f32::NAN), 0.0);

        assert_eq!(bounded_gesture_delta(0.25, 8.0), 0.25);
        assert_eq!(bounded_gesture_delta(99.0, 8.0), 8.0);
        assert_eq!(bounded_gesture_delta(-99.0, 8.0), -8.0);
        assert_eq!(bounded_gesture_delta(f64::INFINITY, 8.0), 0.0);

        assert_eq!(map_pressure_stage(0), PressureStage::Zero);
        assert_eq!(map_pressure_stage(1), PressureStage::Normal);
        assert_eq!(map_pressure_stage(2), PressureStage::Force);
        assert_eq!(map_pressure_stage(7), PressureStage::Other(7));

        assert_eq!(bounded_touch_force(Force::Normalized(0.625)), 0.625);
        assert_eq!(bounded_touch_force(Force::Normalized(2.0)), 1.0);
        assert_eq!(bounded_touch_force(Force::Normalized(f64::NAN)), 0.0);
        assert_eq!(
            bounded_touch_force(Force::Calibrated {
                force: 2.0,
                max_possible_force: 4.0,
                altitude_angle: None,
            }),
            0.5
        );

        assert_eq!(
            map_gesture_phase(winit::event::TouchPhase::Started),
            GesturePhase::Started
        );
        assert_eq!(
            map_gesture_phase(winit::event::TouchPhase::Moved),
            GesturePhase::Moved
        );
        assert_eq!(
            map_gesture_phase(winit::event::TouchPhase::Ended),
            GesturePhase::Ended
        );
        assert_eq!(
            map_gesture_phase(winit::event::TouchPhase::Cancelled),
            GesturePhase::Cancelled
        );
        assert_eq!(
            map_touch_phase(winit::event::TouchPhase::Started),
            TouchPhase::Started
        );
        assert_eq!(
            map_touch_phase(winit::event::TouchPhase::Moved),
            TouchPhase::Moved
        );
        assert_eq!(
            map_touch_phase(winit::event::TouchPhase::Ended),
            TouchPhase::Ended
        );
        assert_eq!(
            map_touch_phase(winit::event::TouchPhase::Cancelled),
            TouchPhase::Cancelled
        );
    }

    #[test]
    fn drawable_geometry_converts_to_logical_points() {
        assert_eq!(
            logical_window_size(PhysicalSize::new(1200, 800), 2.0),
            Size::new(600.0, 400.0)
        );
    }

    #[test]
    fn hidden_inset_window_chrome_options_are_composable() {
        let options = WindowOptions::new("Custom chrome")
            .title_bar_style(TitleBarStyle::HiddenInset)
            .traffic_light_position(16.0, 13.0);

        assert_eq!(options.title_bar_style, TitleBarStyle::HiddenInset);
        assert_eq!(options.traffic_light_position, Some(Point::new(16.0, 13.0)));
        assert_eq!(
            options
                .without_traffic_light_position()
                .traffic_light_position,
            None
        );
    }

    #[test]
    fn modifier_mapping_preserves_all_flags() {
        let mapped =
            map_modifiers(ModifiersState::SHIFT | ModifiersState::CONTROL | ModifiersState::SUPER);
        assert!(mapped.contains(Modifiers::SHIFT));
        assert!(mapped.contains(Modifiers::CONTROL));
        assert!(mapped.contains(Modifiers::SUPER));
        assert!(!mapped.contains(Modifiers::ALT));
    }

    #[test]
    fn platform_primary_modifier_matches_native_shortcuts() {
        if cfg!(target_os = "macos") {
            assert!(primary_modifier(Modifiers::SUPER));
            assert!(!primary_modifier(Modifiers::CONTROL));
            assert!(word_modifier(Modifiers::ALT));
            assert!(!word_modifier(Modifiers::CONTROL));
        } else {
            assert!(primary_modifier(Modifiers::CONTROL));
            assert!(!primary_modifier(Modifiers::SUPER));
            assert!(word_modifier(Modifiers::CONTROL));
            assert!(!word_modifier(Modifiers::ALT));
        }
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn default_close_shortcut_is_exact_and_non_repeating() {
        let close = Key::Character("w".to_owned());
        assert!(is_default_close_shortcut(&close, Modifiers::SUPER, false));
        assert!(!is_default_close_shortcut(&close, Modifiers::SUPER, true));
        assert!(!is_default_close_shortcut(
            &close,
            Modifiers::SUPER | Modifiers::SHIFT,
            false,
        ));
        assert!(!is_default_close_shortcut(
            &Key::Character("q".to_owned()),
            Modifiers::SUPER,
            false,
        ));
    }
}
