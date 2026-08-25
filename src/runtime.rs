#[cfg(target_os = "macos")]
use std::sync::atomic::AtomicBool;
use std::{
    any::{Any, TypeId},
    cell::{Ref, RefCell},
    collections::{HashMap, HashSet, VecDeque},
    fmt,
    future::Future,
    marker::PhantomData,
    path::PathBuf,
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
use arboard::Clipboard;
use thiserror::Error;
#[cfg(target_os = "macos")]
use winit::platform::macos::WindowAttributesExtMacOS;
use winit::{
    application::ApplicationHandler,
    dpi::{LogicalPosition, LogicalSize, PhysicalSize},
    event::{ElementState, Ime, MouseScrollDelta, WindowEvent},
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop, EventLoopProxy},
    keyboard::{Key as WinitKey, ModifiersState, NamedKey},
    window::{
        CursorIcon, Fullscreen, UserAttentionType, Window, WindowButtons, WindowId, WindowLevel,
    },
};

#[cfg(target_os = "macos")]
use crate::platform::PlatformDialogId;
use crate::{
    Action, ActionListener, AnyAction, Color, Element, ElementId, Entity, EntityId, EventEmitter,
    FocusHandle, Global, IntoElement, KeyBinding, Keymap, Keystroke,
    MAX_ENTITY_EVENT_DELIVERIES_PER_TURN, MAX_ENTITY_SUBSCRIPTIONS_PER_WINDOW,
    MAX_GLOBAL_OBSERVER_DELIVERIES_PER_TURN, MAX_GLOBAL_SUBSCRIPTIONS_PER_WINDOW,
    MAX_OBSERVED_ENTITIES_PER_WINDOW, MAX_OBSERVED_GLOBALS_PER_WINDOW, MAX_PENDING_ENTITY_EVENTS,
    MAX_PENDING_GLOBAL_NOTIFICATIONS, Menu, OpenUrls, OsAction, Point, Rect, Scene, Size,
    SystemNotificationResponse, Vector,
    background::{
        BackgroundCompletion, BackgroundTaskError, BackgroundTaskPoolHandle, TaskSpawnError,
    },
    entity::{EntityEvent, Subscription, SubscriptionState},
    event::{
        ContextMenuEvent, DragOrigin, DragStartEvent, DropEvent, DroppedFiles, Event, EventContext,
        ExternalDragPayload, ExternalDragText, ExternalDragUrl, FileDragPaths, FormSubmitEvent,
        Key, MAX_DROPPED_FILES, Modifiers, MouseButton, PointerEvent, PointerPhase,
        ValidationReport,
    },
    foreground::{
        AsyncViewContext, ForegroundTaskSpawnError, ForegroundTaskSpawner, ScheduledForegroundTask,
        Task,
    },
    global::GlobalStore,
    image_resource::{ImageAssetCache, ImageLoadCompletion, ImageWorkerPoolHandle},
    menu::{MenuAction, collect_menu_actions},
    metrics::{FrameMetrics, MetricsTracker},
    platform::{PlatformError, PlatformRequest},
    renderer::{GpuContext, GpuRenderer, RenderOutcome},
    scheduler::FrameScheduler,
    ui_tree::{DismissRequest, FormAttempt, InputResult, UiTree},
};

const MAX_NESTED_FORM_SUBMISSIONS: u8 = 8;

#[cfg(target_os = "macos")]
use crate::event::{ExternalDragEndEvent, ExternalDragOperation};
#[cfg(target_os = "macos")]
use crate::macos::{
    MacExternalDragMonitor, MacExternalDragSession, MacFirstFrameGuard, MacMouseDownEvent,
    MacNativeDropHost, MacNativeDropOffer, MacNativeDropPayload, MacNativeDropPending,
    MacNativeHost, MacPlatformDialog, MacPlatformDialogContext, MacTypedDragPayload,
    MacTypedDragRegistry, capture_left_mouse_down, configure_gpu_window_resize,
    configure_window_kind, current_pointer_position, dismiss_window_relation, is_window_fullscreen,
    is_window_maximized, perform_window_close, perform_window_drag, position_traffic_lights,
    present_native_open_panel, present_native_prompt, present_native_save_panel,
    present_window_relation, set_window_movable, shell_open_path, shell_open_url,
    shell_reveal_path, start_external_drag,
};
#[cfg(target_os = "macos")]
use crate::macos_application::MacApplicationHost;
#[cfg(target_os = "macos")]
use crate::macos_menu::{MacMenuHost, MacMenuItemState};

pub(crate) enum RuntimeEvent {
    Accessibility(AccessibilityEvent),
    ImageLoaded(WindowHandle, ImageLoadCompletion),
    BackgroundCompleted(BackgroundCompletion),
    ForegroundTasksReady,
    MenuWillOpen,
    MenuAction(usize),
    #[cfg(target_os = "macos")]
    ExternalDragBoundary(WindowHandle, Point),
    #[cfg(target_os = "macos")]
    ExternalDragEnded(WindowHandle, ExternalDragOperation),
    #[cfg(target_os = "macos")]
    NativeDropChanged(WindowHandle),
    #[cfg(target_os = "macos")]
    PlatformDialogClosed(WindowHandle, PlatformDialogId),
    #[cfg(target_os = "macos")]
    PlatformDialogCancelled(WindowHandle, PlatformDialogId),
    #[cfg(target_os = "macos")]
    OpenUrls(OpenUrls),
    #[cfg(target_os = "macos")]
    Reopen {
        has_visible_windows: bool,
    },
    #[cfg(target_os = "macos")]
    SystemWake,
    #[cfg(target_os = "macos")]
    SystemNotificationAuthorization {
        granted: bool,
        error: Option<Arc<str>>,
    },
    #[cfg(target_os = "macos")]
    SystemNotificationResponse(SystemNotificationResponse),
}

impl From<AccessibilityEvent> for RuntimeEvent {
    fn from(event: AccessibilityEvent) -> Self {
        Self::Accessibility(event)
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
    PopUp,
    /// A utility window that stays above ordinary application windows.
    Floating,
    /// A parent-owned modal sheet on macOS.
    Dialog,
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
    pub kind: WindowKind,
    pub bounds: WindowBounds,
    pub viewport_size: Size,
    pub scale_factor: f32,
    pub focused: bool,
    pub visible: bool,
    pub minimized: bool,
    pub maximized: bool,
    pub fullscreen: bool,
    pub occluded: bool,
    pub movable: bool,
    pub resizable: bool,
    pub minimizable: bool,
}

/// Maximum UTF-8 bytes accepted for a native window title.
pub const MAX_WINDOW_TITLE_BYTES: usize = 16 * 1024;
/// Maximum logical width or height accepted by a programmatic window-bounds request.
pub const MAX_WINDOW_LOGICAL_DIMENSION: f32 = 32_768.0;
/// Maximum absolute desktop coordinate accepted by a programmatic window-bounds request.
pub const MAX_WINDOW_LOGICAL_COORDINATE: f32 = 16_777_216.0;
/// Maximum window mutations one event callback may queue.
pub const MAX_WINDOW_COMMANDS_PER_EVENT: usize = 256;
/// Maximum deferred native window mutations retained by one application effect cycle.
pub const MAX_PENDING_WINDOW_COMMANDS: usize = 1_024;

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
    #[error("a hidden titlebar cannot expose or reposition native traffic-light buttons")]
    HiddenTitleBarTrafficLights,
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
}

/// Window and renderer defaults used by [`App`].
#[derive(Clone, Debug)]
pub struct AppConfig {
    pub title: String,
    pub size: Size,
    pub window_bounds: Option<WindowBounds>,
    pub minimum_size: Option<Size>,
    pub background: Color,
    pub performance_profile: PerformanceProfile,
    pub title_bar_style: TitleBarStyle,
    pub kind: WindowKind,
    pub focus: bool,
    pub show: bool,
    pub is_movable: bool,
    pub is_resizable: bool,
    pub is_minimizable: bool,
    /// Top-left position of the macOS close button, in logical points from the window's top-left.
    pub traffic_light_position: Option<Point>,
    /// Logical pixels represented by one platform line-wheel unit.
    pub line_scroll_pixels: f32,
    /// How long an incomplete multi-stroke key binding waits before its prefix is replayed.
    pub key_sequence_timeout: Duration,
    /// Disable non-essential image animation. macOS Reduce Motion is always respected as well.
    pub reduce_motion: bool,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            title: "QuickGUI".to_owned(),
            size: Size::new(960.0, 640.0),
            window_bounds: None,
            minimum_size: Some(Size::new(320.0, 240.0)),
            background: Color::rgb8(18, 18, 20),
            performance_profile: PerformanceProfile::Balanced,
            title_bar_style: TitleBarStyle::Default,
            kind: WindowKind::Normal,
            focus: true,
            show: true,
            is_movable: true,
            is_resizable: true,
            is_minimizable: true,
            traffic_light_position: None,
            line_scroll_pixels: 40.0,
            key_sequence_timeout: Duration::from_secs(1),
            reduce_motion: false,
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

    pub fn background(mut self, background: Color) -> Self {
        self.background = background;
        self
    }

    pub fn performance_profile(mut self, profile: PerformanceProfile) -> Self {
        self.performance_profile = profile;
        self
    }

    pub fn title_bar_style(mut self, style: TitleBarStyle) -> Self {
        self.title_bar_style = style;
        self
    }

    pub fn window_kind(mut self, kind: WindowKind) -> Self {
        self.kind = kind;
        self
    }

    pub fn focus(mut self, focus: bool) -> Self {
        self.focus = focus;
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

fn validate_window_options(options: &WindowOptions) -> Result<(), WindowCommandError> {
    validate_window_title(&options.title)?;
    validate_window_size(options.size)?;
    if let Some(bounds) = options.window_bounds {
        validate_window_bounds(bounds)?;
    }
    if let Some(minimum) = options.minimum_size {
        validate_window_size(minimum)?;
    }
    if options.title_bar_style == TitleBarStyle::Hidden && options.traffic_light_position.is_some()
    {
        return Err(WindowCommandError::HiddenTitleBarTrafficLights);
    }
    Ok(())
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
        background_tasks: &BackgroundTaskPoolHandle,
        foreground_tasks: &ForegroundTaskSpawner,
        globals: &GlobalStore,
    ) -> (Element, bool, Option<Instant>);

    fn as_any_mut(&mut self) -> &mut dyn Any;
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
        background_tasks: &BackgroundTaskPoolHandle,
        foreground_tasks: &ForegroundTaskSpawner,
        globals: &GlobalStore,
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
            background_tasks,
            foreground_tasks,
            globals,
            marker: PhantomData,
        };
        cx.listeners.clear();
        let root = self.0.render(&mut cx).into_element();
        (root, cx.request_animation_frame, cx.repaint_deadline)
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        &mut self.0
    }
}

pub(crate) struct WindowRequest {
    pub(crate) handle: WindowHandle,
    view: Box<dyn AnyView>,
    pub(crate) options: WindowOptions,
    pub(crate) parent: Option<WindowHandle>,
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
        Self {
            handle: WindowHandle::next(),
            view: Box::new(ViewAdapter(view)),
            options,
            parent,
        }
    }
}

impl fmt::Debug for WindowRequest {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("WindowRequest")
            .field("handle", &self.handle)
            .field("parent", &self.parent)
            .field("options", &self.options)
            .finish_non_exhaustive()
    }
}

#[derive(Debug)]
pub(crate) enum WindowCommand {
    SetTitle(WindowHandle, String),
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
    SetMinimizable(WindowHandle, bool),
    RequestAttention(WindowHandle),
}

impl WindowCommand {
    pub(crate) fn handle(&self) -> WindowHandle {
        match self {
            Self::SetTitle(handle, _)
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
            | Self::SetMinimizable(handle, _)
            | Self::RequestAttention(handle) => *handle,
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
    background_tasks: &'a BackgroundTaskPoolHandle,
    foreground_tasks: &'a ForegroundTaskSpawner,
    globals: &'a GlobalStore,
    marker: PhantomData<fn(&mut V)>,
}

impl<V: 'static> ViewContext<'_, V> {
    pub fn size(&self) -> Size {
        self.size
    }

    pub fn scale_factor(&self) -> f32 {
        self.scale_factor
    }

    pub fn window_handle(&self) -> WindowHandle {
        self.window
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
        self.listeners
            .actions
            .entry((id, TypeId::of::<A>()))
            .or_default()
            .push(erased);
        ActionListener {
            id,
            marker: PhantomData,
        }
    }
}

type ClickCallback = Arc<dyn Fn(&mut dyn Any, &mut EventContext)>;
type PointerCallback = Arc<dyn Fn(&mut dyn Any, &PointerEvent, &mut EventContext)>;
type ContextMenuCallback = Arc<dyn Fn(&mut dyn Any, &ContextMenuEvent, &mut EventContext)>;
type InputCallback = Arc<dyn Fn(&mut dyn Any, &str, &mut EventContext)>;
type FormSubmitCallback = Arc<dyn Fn(&mut dyn Any, &FormSubmitEvent, &mut EventContext)>;
type FormInvalidCallback = Arc<dyn Fn(&mut dyn Any, &ValidationReport, &mut EventContext)>;
type ActionCallback = Arc<dyn Fn(&mut dyn Any, &dyn Any, &mut EventContext)>;
type DragStartCallback = Arc<dyn Fn(&mut dyn Any, &DragStartEvent, &mut EventContext) -> AnyDrag>;
type DropCallback = Arc<dyn Fn(&mut dyn Any, &dyn Any, &DropEvent, &mut EventContext)>;
type EntityEventCallback = Rc<RefCell<dyn FnMut(&mut dyn Any, &dyn Any, &mut EventContext)>>;
type GlobalObserverCallback = Rc<RefCell<dyn FnMut(&mut dyn Any, &mut EventContext)>>;

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
    context_menus: HashMap<ElementId, ContextMenuCallback>,
    drag_sources: HashMap<ElementId, (TypeId, DragStartCallback)>,
    drops: HashMap<(ElementId, TypeId), DropCallback>,
    drop_order: Vec<(ElementId, TypeId)>,
    inputs: HashMap<ElementId, InputCallback>,
    submits: HashMap<ElementId, InputCallback>,
    form_submits: HashMap<ElementId, FormSubmitCallback>,
    form_invalids: HashMap<ElementId, FormInvalidCallback>,
    dismisses: HashMap<ElementId, ClickCallback>,
    actions: HashMap<(ElementId, TypeId), Vec<ActionCallback>>,
    observed_entities: HashSet<EntityId>,
    observed_globals: HashSet<TypeId>,
    observes_window_state: bool,
    entity_events: HashMap<(EntityId, TypeId), Vec<EntityEventSubscription>>,
    entity_subscription_count: usize,
    global_observers: Vec<GlobalObserverSubscription>,
}

impl ListenerRegistry {
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
        self.context_menus.clear();
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

/// An opaque secondary-click binding returned by [`ViewContext::context_menu_listener`].
pub struct ContextMenuListener<V> {
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

impl<V> ContextMenuListener<V> {
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
}

type OpenUrlsCallback = Box<dyn FnMut(OpenUrls, &mut EventContext)>;
type ReopenCallback = Box<dyn FnMut(bool, &mut EventContext)>;
type SystemWakeCallback = Box<dyn FnMut(&mut EventContext)>;
type SystemNotificationResponseCallback =
    Box<dyn FnMut(SystemNotificationResponse, &mut EventContext)>;

#[derive(Default)]
struct ApplicationCallbacks {
    open_urls: Option<OpenUrlsCallback>,
    reopen: Option<ReopenCallback>,
    system_wake: Option<SystemWakeCallback>,
    system_notification_response: Option<SystemNotificationResponseCallback>,
}

/// Configures and runs one retained QuickGUI view.
pub struct App<V> {
    view: V,
    config: AppConfig,
    keymap: Keymap,
    menus: Vec<Menu>,
    globals: GlobalStore,
    application_callbacks: ApplicationCallbacks,
}

impl<V: View> App<V> {
    pub fn new(view: V) -> Self {
        Self {
            view,
            config: AppConfig::default(),
            keymap: Keymap::default(),
            menus: Vec::new(),
            globals: GlobalStore::default(),
            application_callbacks: ApplicationCallbacks::default(),
        }
    }

    pub fn config(mut self, config: AppConfig) -> Self {
        self.config = config;
        self
    }

    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.config.title = title.into();
        self
    }

    pub fn size(mut self, width: f32, height: f32) -> Self {
        self.config = self.config.size(width, height);
        self
    }

    pub fn window_bounds(mut self, bounds: WindowBounds) -> Self {
        self.config = self.config.window_bounds(bounds);
        self
    }

    pub fn position(mut self, x: f32, y: f32) -> Self {
        self.config = self.config.position(x, y);
        self
    }

    pub fn maximized(mut self, maximized: bool) -> Self {
        self.config = self.config.maximized(maximized);
        self
    }

    pub fn fullscreen(mut self, fullscreen: bool) -> Self {
        self.config = self.config.fullscreen(fullscreen);
        self
    }

    pub fn performance_profile(mut self, profile: PerformanceProfile) -> Self {
        self.config.performance_profile = profile;
        self
    }

    pub fn title_bar_style(mut self, style: TitleBarStyle) -> Self {
        self.config.title_bar_style = style;
        self
    }

    pub fn window_kind(mut self, kind: WindowKind) -> Self {
        self.config.kind = kind;
        self
    }

    pub fn focus(mut self, focus: bool) -> Self {
        self.config.focus = focus;
        self
    }

    pub fn show(mut self, show: bool) -> Self {
        self.config.show = show;
        self
    }

    pub fn movable(mut self, movable: bool) -> Self {
        self.config.is_movable = movable;
        self
    }

    pub fn resizable(mut self, resizable: bool) -> Self {
        self.config.is_resizable = resizable;
        self
    }

    pub fn minimizable(mut self, minimizable: bool) -> Self {
        self.config.is_minimizable = minimizable;
        self
    }

    /// Position the macOS close button in logical points from the window's top-left.
    pub fn traffic_light_position(mut self, x: f32, y: f32) -> Self {
        self.config.traffic_light_position = Some(Point::new(x, y));
        self
    }

    pub fn without_traffic_light_position(mut self) -> Self {
        self.config.traffic_light_position = None;
        self
    }

    pub fn reduce_motion(mut self, reduce_motion: bool) -> Self {
        self.config.reduce_motion = reduce_motion;
        self
    }

    /// Add application key bindings. Later bindings take precedence at equal context depth.
    pub fn bind_keys(mut self, bindings: impl IntoIterator<Item = KeyBinding>) -> Self {
        self.keymap.add_bindings(bindings);
        self
    }

    /// Replace the complete application keymap.
    pub fn keymap(mut self, keymap: Keymap) -> Self {
        self.keymap = keymap;
        self
    }

    /// Append one declarative application menu.
    pub fn menu(mut self, menu: Menu) -> Self {
        self.menus.push(menu);
        self
    }

    /// Replace the complete declarative application menu set.
    pub fn menus(mut self, menus: impl IntoIterator<Item = Menu>) -> Self {
        self.menus = menus.into_iter().collect();
        self
    }

    /// Install or replace one main-thread application-global value before launch.
    ///
    /// Every native window opened by this application reads the same typed store. Values are
    /// dropped with the runtime after the final window closes.
    pub fn global<G: Global>(self, global: G) -> Self {
        self.globals.set(global);
        self
    }

    /// Handle URLs supplied by the operating system, including `file:` URLs.
    ///
    /// The callback is application-wide and receives a context without a current window. It can
    /// update globals/entities or open a new top-level window. Subsequent registration replaces
    /// the previous callback.
    pub fn on_open_urls(
        mut self,
        callback: impl FnMut(OpenUrls, &mut EventContext) + 'static,
    ) -> Self {
        self.application_callbacks.open_urls = Some(Box::new(callback));
        self
    }

    /// Handle a Dock/Finder request to reopen an already-running macOS application.
    pub fn on_reopen(mut self, callback: impl FnMut(bool, &mut EventContext) + 'static) -> Self {
        self.application_callbacks.reopen = Some(Box::new(callback));
        self
    }

    /// Handle the operating system waking from sleep.
    pub fn on_system_wake(mut self, callback: impl FnMut(&mut EventContext) + 'static) -> Self {
        self.application_callbacks.system_wake = Some(Box::new(callback));
        self
    }

    /// Handle activation of a delivered system notification or one of its action buttons.
    pub fn on_system_notification_response(
        mut self,
        callback: impl FnMut(SystemNotificationResponse, &mut EventContext) + 'static,
    ) -> Self {
        self.application_callbacks.system_notification_response = Some(Box::new(callback));
        self
    }

    pub fn run(self) -> Result<(), AppError> {
        let event_loop = EventLoop::with_user_event().build()?;
        event_loop.set_control_flow(ControlFlow::Wait);
        let mut runtime = Runtime::new(
            WindowRequest::new(self.view, self.config),
            self.globals,
            self.keymap,
            self.menus,
            self.application_callbacks,
            event_loop.create_proxy(),
        )?;
        event_loop.run_app(&mut runtime)?;
        match runtime.fatal_error.take() {
            Some(error) => Err(error),
            None => Ok(()),
        }
    }
}

struct RuntimeWindow {
    parent: Option<WindowHandle>,
    view: Box<dyn AnyView>,
    renderer: GpuRenderer,
    image_assets: ImageAssetCache,
    #[cfg(target_os = "macos")]
    native_host: Option<MacNativeHost>,
    #[cfg(target_os = "macos")]
    native_drop_host: MacNativeDropHost,
    #[cfg(target_os = "macos")]
    first_frame_guard: Option<MacFirstFrameGuard>,
    ui: UiTree,
    scheduler: FrameScheduler,
    scene: Scene,
    metrics: MetricsTracker,
    scale_factor: f32,
    logical_size: Size,
    logical_position: Point,
    restore_bounds: Rect,
    maximized: bool,
    pointer: Option<Point>,
    pointer_capture: Option<PointerCapture>,
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
    view_deadline: Option<Instant>,
    listeners: ListenerRegistry,
    accessibility: AccessibilityAdapter,
    // The window is last so GPU surface state is dropped before its native handle.
    window: Arc<Window>,
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
    windows: HashMap<WindowId, WindowEntry>,
    window_handles: HashMap<WindowHandle, WindowId>,
    current_window: Option<(WindowId, WindowHandle)>,
    active_window: Option<WindowId>,
    focus_history: Vec<WindowId>,
    close_requests: Vec<WindowHandle>,
    focus_requests: Vec<WindowHandle>,
    invalidate_requests: Vec<WindowHandle>,
    window_commands: Vec<WindowCommand>,
    platform_requests: VecDeque<PlatformRequest>,
    image_workers: ImageWorkerPoolHandle,
    background_tasks: BackgroundTaskPoolHandle,
    foreground_tasks: ForegroundTaskSpawner,
    globals: GlobalStore,
    gpu_contexts: HashMap<PerformanceProfile, GpuContext>,
    #[cfg(target_os = "macos")]
    native_drag_registry: MacTypedDragRegistry,
    #[cfg(target_os = "macos")]
    active_platform_dialogs: HashMap<WindowHandle, ActivePlatformDialog>,
    #[cfg(target_os = "macos")]
    mac_application_host: MacApplicationHost,
    application_callbacks: ApplicationCallbacks,
    // The following four fields are the currently activated window. Event delivery is serialized
    // by Winit, so moving one entry into this slot keeps the mature single-window hot path narrow
    // while every inactive window remains independently retained in `windows`.
    config: AppConfig,
    keymap: Keymap,
    #[cfg(target_os = "macos")]
    menus: Vec<Menu>,
    menu_actions: Vec<MenuAction>,
    #[cfg(target_os = "macos")]
    menu_host: Option<MacMenuHost>,
    pending_input: Option<PendingInput>,
    window: Option<RuntimeWindow>,
    modifiers: Modifiers,
    fatal_error: Option<AppError>,
    event_proxy: EventLoopProxy<RuntimeEvent>,
    clipboard: Option<Clipboard>,
    form_submission_depth: u8,
}

#[cfg(target_os = "macos")]
struct ActivePlatformDialog {
    id: PlatformDialogId,
    open: Arc<AtomicBool>,
    native: MacPlatformDialog,
}

#[derive(Clone, Debug)]
struct PendingKey {
    key: Key,
    modifiers: Modifiers,
    repeat: bool,
    text: Option<String>,
}

impl PendingKey {
    fn keystroke(&self) -> Keystroke {
        Keystroke::from_key_event(&self.key, self.modifiers)
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
        initial_window: WindowRequest,
        globals: GlobalStore,
        keymap: Keymap,
        menus: Vec<Menu>,
        application_callbacks: ApplicationCallbacks,
        event_proxy: EventLoopProxy<RuntimeEvent>,
    ) -> Result<Self, AppError> {
        let menu_actions = collect_menu_actions(&menus);
        let mut pending_windows = VecDeque::with_capacity(2);
        pending_windows.push_back(initial_window);
        let image_workers = ImageWorkerPoolHandle::new(event_proxy.clone());
        let background_tasks = BackgroundTaskPoolHandle::new(event_proxy.clone());
        let foreground_tasks = ForegroundTaskSpawner::new(event_proxy.clone());
        #[cfg(target_os = "macos")]
        let mac_application_host = MacApplicationHost::new(
            event_proxy.clone(),
            application_callbacks.open_urls.is_some(),
            application_callbacks.reopen.is_some(),
            application_callbacks.system_wake.is_some(),
            application_callbacks.system_notification_response.is_some(),
        )
        .map_err(AppError::Platform)?;
        Ok(Self {
            pending_windows,
            pending_entity_events: VecDeque::with_capacity(8),
            pending_global_notifications: VecDeque::with_capacity(8),
            pending_global_notification_types: HashSet::with_capacity(8),
            pending_all_globals: false,
            windows: HashMap::new(),
            window_handles: HashMap::new(),
            current_window: None,
            active_window: None,
            focus_history: Vec::new(),
            close_requests: Vec::new(),
            focus_requests: Vec::new(),
            invalidate_requests: Vec::new(),
            window_commands: Vec::with_capacity(8),
            platform_requests: VecDeque::with_capacity(8),
            image_workers,
            background_tasks,
            foreground_tasks,
            globals,
            gpu_contexts: HashMap::new(),
            #[cfg(target_os = "macos")]
            native_drag_registry: MacTypedDragRegistry::new(),
            #[cfg(target_os = "macos")]
            active_platform_dialogs: HashMap::new(),
            #[cfg(target_os = "macos")]
            mac_application_host,
            application_callbacks,
            config: AppConfig::default(),
            keymap,
            #[cfg(target_os = "macos")]
            menus,
            menu_actions,
            #[cfg(target_os = "macos")]
            menu_host: None,
            pending_input: None,
            window: None,
            modifiers: Modifiers::default(),
            fatal_error: None,
            event_proxy,
            clipboard: None,
            form_submission_depth: 0,
        })
    }

    fn event_context(&self) -> EventContext {
        EventContext::with_runtime(
            self.globals.clone(),
            self.foreground_tasks.clone(),
            self.current_handle(),
        )
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

    fn current_window_state(&self) -> Option<WindowState> {
        let handle = self.current_handle()?;
        let state = self.window.as_ref()?;
        let platform_content_attached = runtime_window_content_attached(state);
        let fullscreen = runtime_window_is_fullscreen(state);
        let maximized = !fullscreen && runtime_window_is_maximized(state, &self.config);
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
        Some(WindowState {
            handle,
            kind: self.config.kind,
            bounds,
            viewport_size: state.logical_size,
            scale_factor: state.scale_factor,
            focused: state.focused,
            visible: state.visible,
            minimized,
            maximized,
            fullscreen,
            occluded: state.occluded,
            movable: self.config.is_movable,
            resizable: self.config.is_resizable,
            minimizable: self.config.is_minimizable,
        })
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

    fn process_queued_window_commands(&mut self) {
        for command in std::mem::take(&mut self.window_commands) {
            let handle = command.handle();
            let Some(window_id) = self.window_handles.get(&handle).copied() else {
                continue;
            };
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

            match command {
                WindowCommand::SetTitle(_, title) => {
                    if entry.config.title != title {
                        entry.config.title = title;
                        state.window.set_title(&entry.config.title);
                        force_redraw = true;
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
                    let bounds = Rect::new(
                        state.restore_bounds.x,
                        state.restore_bounds.y,
                        size.width,
                        size.height,
                    );
                    apply_window_bounds(state, WindowBounds::Windowed(bounds));
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
                    } else if !runtime_window_is_fullscreen(state) && entry.config.is_resizable {
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
                        state.window.set_visible(visible);
                        state.visible = visible;
                        if visible {
                            if entry.config.focus {
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
                WindowCommand::SetMinimizable(_, minimizable) => {
                    if entry.config.is_minimizable != minimizable {
                        entry.config.is_minimizable = minimizable;
                        state
                            .window
                            .set_enabled_buttons(window_buttons(&entry.config));
                        state_changed = true;
                    }
                }
                WindowCommand::RequestAttention(_) => state
                    .window
                    .request_user_attention(Some(UserAttentionType::Informational)),
            }

            if force_redraw || state_changed && state.listeners.observes_window_state {
                state.view_dirty |= force_redraw || state.listeners.observes_window_state;
                if state.visible && state.scheduler.invalidate() {
                    state.window.request_redraw();
                }
            }
        }
    }

    fn process_platform_requests(&mut self) {
        #[cfg(target_os = "macos")]
        self.active_platform_dialogs.retain(|handle, dialog| {
            self.window_handles.contains_key(handle) && dialog.open.load(Ordering::Acquire)
        });

        while let Some(request) = self.platform_requests.pop_front() {
            if request.response_cancelled() {
                continue;
            }
            #[cfg(target_os = "macos")]
            self.process_macos_platform_request(request);
            #[cfg(not(target_os = "macos"))]
            request.complete_error(PlatformError::Unsupported);
        }
    }

    #[cfg(target_os = "macos")]
    fn process_macos_platform_request(&mut self, request: PlatformRequest) {
        match request {
            PlatformRequest::ShowSystemNotification(notification) => {
                self.mac_application_host
                    .show_system_notification(notification);
            }
            PlatformRequest::DismissSystemNotification(tag) => {
                self.mac_application_host.dismiss_system_notification(&tag);
            }
            PlatformRequest::OpenUrl(url) => {
                if let Err(error) = shell_open_url(&url) {
                    tracing::warn!(%error, url = %url, "could not open URL with NSWorkspace");
                }
            }
            PlatformRequest::OpenPath(path) => {
                if let Err(error) = shell_open_path(&path) {
                    tracing::warn!(%error, path = %path.display(), "could not open path with NSWorkspace");
                }
            }
            PlatformRequest::RevealPath(path) => {
                if let Err(error) = shell_reveal_path(&path) {
                    tracing::warn!(%error, path = %path.display(), "could not reveal path with NSWorkspace");
                }
            }
            request => {
                let Some(owner) = request.window() else {
                    return;
                };
                let Some(window_id) = self.window_handles.get(&owner).copied() else {
                    request.complete_error(PlatformError::Unavailable);
                    return;
                };
                let Some(native_window) = self
                    .windows
                    .get(&window_id)
                    .map(|entry| entry.state.window.clone())
                else {
                    request.complete_error(PlatformError::Unavailable);
                    return;
                };
                if self
                    .active_platform_dialogs
                    .keys()
                    .any(|candidate| self.windows_share_parent_chain(owner, *candidate))
                {
                    request.complete_error(PlatformError::DialogBusy);
                    return;
                }
                if self.active_platform_dialogs.len() == crate::MAX_ACTIVE_PLATFORM_DIALOGS {
                    request.complete_error(PlatformError::TooManyDialogs);
                    return;
                }

                let id = PlatformDialogId::next();
                let open = Arc::new(AtomicBool::new(true));
                if !request.bind_cancellation(self.event_proxy.clone(), owner, id) {
                    return;
                }
                let context = MacPlatformDialogContext::new(
                    owner,
                    id,
                    open.clone(),
                    self.event_proxy.clone(),
                );
                let native = match request {
                    PlatformRequest::Prompt {
                        level,
                        message,
                        detail,
                        buttons,
                        responder,
                        ..
                    } => {
                        let completion = responder.clone();
                        match present_native_prompt(
                            &native_window,
                            context,
                            level,
                            &message,
                            detail.as_deref(),
                            &buttons,
                            completion,
                        ) {
                            Ok(native) => native,
                            Err(error) => {
                                responder.complete(Err(PlatformError::Platform(error.into())));
                                return;
                            }
                        }
                    }
                    PlatformRequest::OpenPaths {
                        options, responder, ..
                    } => {
                        let completion = responder.clone();
                        match present_native_open_panel(
                            &native_window,
                            context,
                            &options,
                            completion,
                        ) {
                            Ok(native) => native,
                            Err(error) => {
                                responder.complete(Err(PlatformError::Platform(error.into())));
                                return;
                            }
                        }
                    }
                    PlatformRequest::SavePath {
                        options, responder, ..
                    } => {
                        let completion = responder.clone();
                        match present_native_save_panel(
                            &native_window,
                            context,
                            &options,
                            completion,
                        ) {
                            Ok(native) => native,
                            Err(error) => {
                                responder.complete(Err(PlatformError::Platform(error.into())));
                                return;
                            }
                        }
                    }
                    PlatformRequest::ShowSystemNotification(_)
                    | PlatformRequest::DismissSystemNotification(_)
                    | PlatformRequest::OpenUrl(_)
                    | PlatformRequest::OpenPath(_)
                    | PlatformRequest::RevealPath(_) => {
                        unreachable!("application-wide platform actions returned above")
                    }
                };
                self.active_platform_dialogs
                    .insert(owner, ActivePlatformDialog { id, open, native });
            }
        }
    }

    #[cfg(target_os = "macos")]
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

    #[cfg(target_os = "macos")]
    fn apply_application_context(&mut self, event_loop: &ActiveEventLoop, context: EventContext) {
        debug_assert!(self.current_window.is_none());
        debug_assert!(self.window.is_none());
        if self.apply_event_context(event_loop, context, false, false) {
            self.process_window_commands(event_loop);
        }
    }

    #[cfg(target_os = "macos")]
    fn windows_share_parent_chain(&self, first: WindowHandle, second: WindowHandle) -> bool {
        self.window_is_ancestor(first, second) || self.window_is_ancestor(second, first)
    }

    #[cfg(target_os = "macos")]
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

    fn close_requested_window_trees(&mut self) {
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

        for handle in order {
            let Some(window_id) = self.window_handles.remove(&handle) else {
                continue;
            };
            self.foreground_tasks.cancel_window(handle);
            #[cfg(target_os = "macos")]
            if let Some(dialog) = self.active_platform_dialogs.remove(&handle) {
                dialog.native.cancel();
            }
            if let Some(entry) = self.windows.remove(&window_id) {
                #[cfg(target_os = "macos")]
                if entry.state.relation_presented
                    && let Err(error) =
                        dismiss_window_relation(&entry.state.window, entry.config.kind)
                {
                    tracing::warn!(%error, "could not dismiss closing native window relation");
                }
                entry.state.window.set_visible(false);
            }
            self.focus_history
                .retain(|candidate| *candidate != window_id);
            if self.active_window == Some(window_id) {
                self.active_window = self.focus_history.last().copied();
            }
        }
    }

    fn process_window_commands(&mut self, event_loop: &ActiveEventLoop) {
        debug_assert!(self.current_window.is_none());
        debug_assert!(self.window.is_none());

        if !self.process_deferred_effects(event_loop) {
            return;
        }

        while let Some(request) = self.pending_windows.pop_front() {
            self.create_window(event_loop, request);
            if self.fatal_error.is_some() {
                return;
            }
            if !self.process_deferred_effects(event_loop) {
                return;
            }
        }

        self.process_queued_window_commands();

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
                entry.state.window.focus_window();
                self.note_window_focused(window_id);
            }
        }

        self.close_requested_window_trees();
        self.process_platform_requests();

        #[cfg(target_os = "macos")]
        self.sync_active_native_menu_state();

        if self.windows.is_empty() && self.pending_windows.is_empty() {
            event_loop.exit();
        }
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
        if cx.exit {
            event_loop.exit();
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
        let mut focus_changed = false;
        if let Some(state) = &mut self.window {
            let previous_focus = state.ui.focused();
            let mut deferred_focus = false;
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
            if (force_redraw || cx.invalidate || focus_changed || deferred_focus)
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

    /// Returns `None` after exit, otherwise whether a handler consumed the action.
    fn invoke_action(&mut self, event_loop: &ActiveEventLoop, action: &AnyAction) -> Option<bool> {
        let path = self
            .window
            .as_ref()
            .map(|window| window.ui.focus_path())
            .unwrap_or_default();
        for id in path.into_iter().rev() {
            let listeners = self
                .window
                .as_ref()
                .and_then(|window| {
                    window
                        .listeners
                        .actions
                        .get(&(id, action.type_id()))
                        .cloned()
                })
                .unwrap_or_default();
            for listener in listeners {
                let mut cx = self.event_context();
                if let Some(window) = &mut self.window {
                    listener(window.view.as_any_mut(), action.as_any(), &mut cx);
                }
                let propagate = cx.propagate_action;
                if !self.apply_event_context(event_loop, cx, false, true) {
                    return None;
                }
                if !propagate {
                    return Some(true);
                }
            }
        }
        Some(false)
    }

    fn action_available(&self, action: &AnyAction) -> bool {
        let Some(window) = &self.window else {
            return false;
        };
        window.ui.focus_path().into_iter().rev().any(|id| {
            window
                .listeners
                .actions
                .contains_key(&(id, action.type_id()))
        })
    }

    fn replace_menus(&mut self, event_loop: &ActiveEventLoop, menus: Vec<Menu>) -> bool {
        let menu_actions = collect_menu_actions(&menus);
        #[cfg(target_os = "macos")]
        let next_host = match MacMenuHost::new(&menus, self.event_proxy.clone()) {
            Ok(host) => Some(host),
            Err(error) => {
                self.fail(event_loop, AppError::Platform(error));
                return false;
            }
        };

        self.menu_actions = menu_actions;
        #[cfg(target_os = "macos")]
        {
            self.menus = menus;
            self.menu_host = next_host;
            self.sync_native_menu_state();
        }
        #[cfg(not(target_os = "macos"))]
        let _ = menus;
        true
    }

    fn os_action_available(&self, action: OsAction) -> bool {
        let Some(window) = &self.window else {
            return false;
        };
        let input_focused = window.ui.focused_text_input().is_some();
        match action {
            OsAction::Cut => {
                input_focused
                    && window
                        .ui
                        .selected_input_text()
                        .is_some_and(|selection| !selection.is_empty())
            }
            OsAction::Copy => window
                .ui
                .selected_text()
                .is_some_and(|selection| !selection.is_empty()),
            OsAction::Paste => input_focused,
            OsAction::SelectAll => input_focused || window.ui.has_selectable_text(),
            OsAction::Undo => input_focused && window.ui.input_can_undo(),
            OsAction::Redo => input_focused && window.ui.input_can_redo(),
        }
    }

    fn invoke_os_action(&mut self, event_loop: &ActiveEventLoop, action: OsAction) -> bool {
        match action {
            OsAction::Copy => {
                let selected = self
                    .window
                    .as_ref()
                    .and_then(|window| window.ui.selected_text());
                if let Some(selected) = selected
                    && let Some(clipboard) = self.clipboard()
                {
                    let _ = clipboard.set_text(selected.as_ref());
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
                let copied = self
                    .clipboard()
                    .is_some_and(|clipboard| clipboard.set_text(selected.as_ref()).is_ok());
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
                let pasted = self
                    .clipboard()
                    .and_then(|clipboard| clipboard.get_text().ok());
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
        let states = self
            .menu_actions
            .iter()
            .map(|item| MacMenuItemState {
                disabled: item.disabled,
                action_available: self.action_available(&item.action)
                    || item
                        .os_action
                        .is_some_and(|action| self.os_action_available(action)),
                checked: item.checked,
                shortcut: self
                    .keymap
                    .shortcut_for_action_value(&item.action, &contexts),
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
        let form = self
            .window
            .as_ref()
            .and_then(|window| window.ui.form_for_submitter(id));
        let listener = self
            .window
            .as_ref()
            .and_then(|window| window.listeners.clicks.get(&id).cloned());
        if let Some(listener) = listener {
            let mut cx = self.event_context();
            if let Some(window) = &mut self.window {
                listener(window.view.as_any_mut(), &mut cx);
            }
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

    fn clipboard(&mut self) -> Option<&mut Clipboard> {
        if self.clipboard.is_none() {
            self.clipboard = Clipboard::new().ok();
        }
        self.clipboard.as_mut()
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
                if let Some(selected) = selected
                    && let Some(clipboard) = self.clipboard()
                {
                    let _ = clipboard.set_text(selected.as_ref());
                }
                return true;
            }
            Key::Character(value) if primary && value.eq_ignore_ascii_case("x") => {
                let selected = self
                    .window
                    .as_ref()
                    .and_then(|window| window.ui.selected_input_text());
                let copied = selected.is_some_and(|selected| {
                    self.clipboard()
                        .is_some_and(|clipboard| clipboard.set_text(selected.as_ref()).is_ok())
                });
                if !copied {
                    return true;
                }
                self.window
                    .as_mut()
                    .map(|window| window.ui.input_backspace())
            }
            Key::Character(value) if primary && value.eq_ignore_ascii_case("v") => {
                let pasted = self
                    .clipboard()
                    .and_then(|clipboard| clipboard.get_text().ok());
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
            key,
            modifiers,
            repeat,
            text,
        } = key_event;
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

        let mut handled_by_input = self.handle_static_text_key(event_loop, &key, modifiers, repeat)
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
                || (changed && !self.dispatch(event_loop, Event::TextInput(text.to_owned()), false))
            {
                return false;
            }
            handled_by_input = true;
        }

        match &key {
            _ if handled_by_input => {}
            Key::Tab => {
                let previous_focus = self.window.as_ref().and_then(|window| window.ui.focused());
                if let Some(window) = &mut self.window {
                    window.ui.focus_next(modifiers.contains(Modifiers::SHIFT));
                }
                self.announce_focus_change(event_loop, previous_focus);
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

        self.dispatch(
            event_loop,
            Event::KeyDown {
                key,
                modifiers,
                repeat,
            },
            false,
        )
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
            state.view_dirty = true;
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
        let started = Instant::now();
        let mut request_animation_frame = false;
        if state.view_dirty {
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
                &background_tasks,
                &foreground_tasks,
                &globals,
            );
            request_animation_frame = requested;
            state.view_deadline = repaint_deadline;
            state.image_assets.resolve_tree(&mut root);
            if let Err(error) = state.ui.set_root(
                root,
                state.logical_size,
                state.scale_factor,
                &mut state.renderer,
            ) {
                self.fail(event_loop, AppError::View(error.to_string()));
                return;
            }
            if let Some(request) = state.pending_focus.take()
                && state.ui.is_focusable(request)
            {
                let previous = state.ui.focused();
                state.ui.focus(request);
                if previous != state.ui.focused() {
                    mounted_focus_previous = Some(previous);
                }
            }
            state.view_dirty = false;
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
        let window_title = self.config.title.as_str();
        let RuntimeWindow {
            accessibility, ui, ..
        } = state;
        accessibility.update_if_active(|| ui.accessibility_update(window_title));

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
                if request_animation_frame && state.scheduler.invalidate() {
                    state.view_dirty = true;
                    state.window.request_redraw();
                }
            }
            Ok(RenderOutcome::Retry) => {
                if state.scheduler.invalidate() {
                    state.window.request_redraw();
                }
            }
            Ok(RenderOutcome::Occluded) => {
                // Wait for the platform to expose or resize the window; do not spin while hidden.
            }
            Err(error) => self.fail(event_loop, AppError::Render(error.to_string())),
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
        } = request;
        if let Err(error) = validate_window_options(&options) {
            self.fail(event_loop, AppError::Window(error.to_string()));
            return;
        }
        self.config = options;
        self.pending_input = None;
        self.modifiers = Modifiers::default();

        let requested_bounds = self.config.window_bounds;
        let restore_rect = requested_bounds
            .map(WindowBounds::bounds)
            .unwrap_or_else(|| Rect::from_size(self.config.size));
        let parent_window = parent
            .and_then(|parent| self.window_handles.get(&parent).copied())
            .and_then(|window_id| self.windows.get(&window_id))
            .map(|entry| entry.state.window.clone());

        let mut attributes = Window::default_attributes()
            .with_title(self.config.title.clone())
            .with_visible(false)
            .with_resizable(self.config.is_resizable)
            .with_enabled_buttons(window_buttons(&self.config))
            .with_window_level(match self.config.kind {
                WindowKind::Floating | WindowKind::PopUp => WindowLevel::AlwaysOnTop,
                WindowKind::Normal | WindowKind::Dialog => WindowLevel::Normal,
            })
            .with_inner_size(LogicalSize::new(
                restore_rect.width as f64,
                restore_rect.height as f64,
            ));
        if self.config.window_bounds.is_some() {
            attributes = attributes.with_position(LogicalPosition::new(
                restore_rect.x as f64,
                restore_rect.y as f64,
            ));
        }
        match requested_bounds {
            Some(WindowBounds::Maximized(_)) => attributes = attributes.with_maximized(true),
            Some(WindowBounds::Fullscreen(_)) => {
                attributes = attributes.with_fullscreen(Some(Fullscreen::Borderless(None)))
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
        if let Some(minimum) = self.config.minimum_size {
            attributes = attributes.with_min_inner_size(LogicalSize::new(
                minimum.width as f64,
                minimum.height as f64,
            ));
        }
        let window = match event_loop.create_window(attributes) {
            Ok(window) => Arc::new(window),
            Err(error) => {
                self.fail(event_loop, AppError::Window(error.to_string()));
                return;
            }
        };
        let window_id = window.id();
        window.set_ime_allowed(false);
        #[cfg(target_os = "macos")]
        if self.menu_host.is_none() {
            self.menu_host = match MacMenuHost::new(&self.menus, self.event_proxy.clone()) {
                Ok(host) => Some(host),
                Err(error) => {
                    self.fail(event_loop, AppError::Platform(error));
                    return;
                }
            };
        }
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
        if let Err(error) = configure_window_kind(&window, self.config.kind) {
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
        let mut scheduler = FrameScheduler::default();
        scheduler.invalidate();
        #[cfg(target_os = "macos")]
        let reduce_motion = self.config.reduce_motion || crate::macos::system_reduce_motion();
        #[cfg(not(target_os = "macos"))]
        let reduce_motion = self.config.reduce_motion;
        let mut ui = UiTree::new();
        ui.set_animations_enabled(!reduce_motion, Instant::now());
        self.current_window = Some((window_id, handle));
        self.window_handles.insert(handle, window_id);
        if self.active_window.is_none() && self.config.show {
            self.note_window_focused(window_id);
        }
        self.window = Some(RuntimeWindow {
            parent,
            view,
            renderer,
            image_assets: ImageAssetCache::new(handle, self.image_workers.clone()),
            #[cfg(target_os = "macos")]
            native_host: None,
            #[cfg(target_os = "macos")]
            native_drop_host,
            #[cfg(target_os = "macos")]
            first_frame_guard,
            ui,
            scheduler,
            scene: Scene::new(),
            metrics: MetricsTracker::default(),
            scale_factor,
            logical_size,
            logical_position,
            restore_bounds,
            maximized,
            pointer: None,
            pointer_capture: None,
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
            view_deadline: None,
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
        }

        if self.config.show {
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
            if let Some(state) = &mut self.window {
                state.relation_presented = relation_presented;
                state.visible = true;
                state.window.set_visible(true);
                if self.config.focus {
                    state.window.focus_window();
                }
                state.scheduler.invalidate();
                state.window.request_redraw();
            }
        }
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

            let poll = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| runnable.run()));
            if poll.is_err() {
                tracing::error!(?task, ?owner, "foreground task panicked and was cancelled");
                self.foreground_tasks.cancel_task(task);
            }

            let mut continue_running = true;
            let mut updates = self.foreground_tasks.take_updates(task);
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
        #[cfg(target_os = "macos")]
        for (_, dialog) in self.active_platform_dialogs.drain() {
            dialog.native.cancel();
        }
        self.foreground_tasks.shutdown();
    }
}

impl ApplicationHandler<RuntimeEvent> for Runtime {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.fatal_error.is_some() {
            return;
        }

        event_loop.set_control_flow(ControlFlow::Wait);
        self.process_window_commands(event_loop);
    }

    fn suspended(&mut self, _event_loop: &ActiveEventLoop) {
        // Desktop surfaces remain valid. Mobile surface teardown will be added with mobile shells.
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
                    state.maximized = runtime_window_is_maximized(state, &self.config);
                    if !runtime_window_is_fullscreen(state) && !state.maximized {
                        state.restore_bounds.x = state.logical_position.x;
                        state.restore_bounds.y = state.logical_position.y;
                    }
                    let logical_position = state.logical_position;
                    let scale_factor = state.scale_factor;
                    let observe = state.listeners.observes_window_state;
                    self.dispatch(
                        event_loop,
                        Event::Moved {
                            logical_position,
                            scale_factor,
                        },
                        observe,
                    );
                }
                WindowEvent::Resized(physical) => {
                    let state = self.window.as_mut().expect("window checked above");
                    state.renderer.resize(physical.width, physical.height);
                    state.logical_size = logical_window_size(physical, state.scale_factor);
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
                    state.view_dirty = true;
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
                    state.view_dirty = true;
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
                    if state.listeners.observes_window_state {
                        state.view_dirty = true;
                    }
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
                WindowEvent::RedrawRequested => self.redraw(event_loop),
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
                    } else if state.ui.wants_text_cursor(point) {
                        CursorIcon::Text
                    } else if state.ui.drag_source_at(point).is_some() {
                        CursorIcon::Grab
                    } else if state.ui.wants_pointer_cursor(point) {
                        CursorIcon::Pointer
                    } else {
                        CursorIcon::Default
                    };
                    if cursor != state.cursor {
                        state.cursor = cursor;
                        state.window.set_cursor(cursor);
                    }
                    if repaint && state.scheduler.invalidate() {
                        state.window.request_redraw();
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
                    self.dispatch(event_loop, Event::PointerMoved(point), false);
                }
                WindowEvent::CursorLeft { .. } => {
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
                    state.pointer = None;
                    if state.cursor != CursorIcon::Default {
                        state.cursor = CursorIcon::Default;
                        state.window.set_cursor(CursorIcon::Default);
                    }
                    let repaint = state.ui.pointer_left()
                        | state.ui.update_scrollbar_hover(None, Instant::now())
                        | state.ui.set_drag_over(None);
                    if repaint && state.scheduler.invalidate() {
                        state.window.request_redraw();
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
                                    window.ui.end_scrollbar_drag(now);
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
                    if button == MouseButton::Left
                        && !pressed
                        && let Some(position) = self
                            .window
                            .as_ref()
                            .filter(|window| window.drag_session.is_some())
                            .and_then(|window| {
                                window.pointer.or_else(|| {
                                    window.drag_session.as_ref().map(|drag| drag.position)
                                })
                            })
                    {
                        if !self.finish_internal_drag(event_loop, position) {
                            return;
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
                WindowEvent::MouseWheel { delta, .. } => {
                    let state = self.window.as_mut().expect("window checked above");
                    if state
                        .pointer
                        .is_some_and(|point| state.ui.is_app_region_drag(point))
                    {
                        return;
                    }
                    let delta = match delta {
                        MouseScrollDelta::LineDelta(x, y) => Vector::new(
                            x * self.config.line_scroll_pixels,
                            y * self.config.line_scroll_pixels,
                        ),
                        MouseScrollDelta::PixelDelta(position) => Vector::new(
                            position.x as f32 / state.scale_factor,
                            position.y as f32 / state.scale_factor,
                        ),
                    };
                    if state.scheduler.accumulate_scroll(delta) {
                        state.window.request_redraw();
                    }
                }
                WindowEvent::ModifiersChanged(modifiers) => {
                    self.modifiers = map_modifiers(modifiers.state());
                    self.dispatch(event_loop, Event::ModifiersChanged(self.modifiers), false);
                }
                WindowEvent::KeyboardInput { event, .. } => {
                    let key = map_key(&event.logical_key);
                    if event.state == ElementState::Pressed {
                        if key == Key::Escape && self.cancel_internal_drag() {
                            self.dispatch(
                                event_loop,
                                Event::KeyDown {
                                    key,
                                    modifiers: self.modifiers,
                                    repeat: event.repeat,
                                },
                                false,
                            );
                            return;
                        }
                        self.handle_pressed_key(
                            event_loop,
                            PendingKey {
                                key,
                                modifiers: self.modifiers,
                                repeat: event.repeat,
                                text: event.text.map(|text| text.to_string()),
                            },
                        );
                    } else {
                        self.dispatch(
                            event_loop,
                            Event::KeyUp {
                                key,
                                modifiers: self.modifiers,
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
                    if let Some(state) = &mut self.window {
                        state.focused = focused;
                    }
                    if focused {
                        self.note_window_focused(window_id);
                    }
                    if !focused {
                        let cancelled = self.window.as_mut().and_then(|state| {
                            let capture = state.pointer_capture.take();
                            let internal_drag = state.drag_session.take().is_some();
                            state.drag_candidate = None;
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
                    }
                    #[cfg(target_os = "macos")]
                    if focused {
                        let reduce_motion =
                            self.config.reduce_motion || crate::macos::system_reduce_motion();
                        let state = self.window.as_mut().expect("window checked above");
                        state.reduce_motion = reduce_motion;
                        state.ui.set_animations_enabled(
                            !state.occluded && !reduce_motion,
                            Instant::now(),
                        );
                    }
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
        if matches!(&event, RuntimeEvent::ForegroundTasksReady) {
            self.process_foreground_tasks(event_loop);
            return;
        }
        #[cfg(target_os = "macos")]
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
        if matches!(&event, RuntimeEvent::SystemWake) {
            self.invoke_system_wake(event_loop);
            return;
        }
        #[cfg(target_os = "macos")]
        if let RuntimeEvent::SystemNotificationAuthorization { granted, error } = &event {
            self.mac_application_host
                .complete_system_notification_authorization(*granted, error.clone());
            return;
        }
        #[cfg(target_os = "macos")]
        if let RuntimeEvent::SystemNotificationResponse(response) = &event {
            self.invoke_system_notification_response(event_loop, response.clone());
            return;
        }
        #[cfg(target_os = "macos")]
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
            RuntimeEvent::PlatformDialogClosed(_, _) => unreachable!("handled before routing"),
            #[cfg(target_os = "macos")]
            RuntimeEvent::PlatformDialogCancelled(_, _) => unreachable!("handled before routing"),
            #[cfg(target_os = "macos")]
            RuntimeEvent::OpenUrls(_)
            | RuntimeEvent::Reopen { .. }
            | RuntimeEvent::SystemWake
            | RuntimeEvent::SystemNotificationAuthorization { .. }
            | RuntimeEvent::SystemNotificationResponse(_) => {
                unreachable!("handled before window routing")
            }
            RuntimeEvent::MenuWillOpen | RuntimeEvent::MenuAction(_) => self.active_window,
        };
        let Some(target) = target else {
            return;
        };
        if !self.activate_window(target) {
            return;
        }
        (|| match event {
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
                let item = self
                    .menu_actions
                    .get(action_id)
                    .filter(|item| !item.disabled)
                    .map(|item| (item.action.clone(), item.os_action));
                if let Some((action, os_action)) = item {
                    let Some(handled) = self.invoke_action(event_loop, &action) else {
                        return;
                    };
                    if !handled && let Some(os_action) = os_action {
                        self.invoke_os_action(event_loop, os_action);
                    }
                    #[cfg(target_os = "macos")]
                    self.sync_native_menu_state();
                }
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
            RuntimeEvent::PlatformDialogClosed(_, _) => unreachable!("handled before routing"),
            #[cfg(target_os = "macos")]
            RuntimeEvent::PlatformDialogCancelled(_, _) => {
                unreachable!("handled before routing")
            }
            #[cfg(target_os = "macos")]
            RuntimeEvent::OpenUrls(_)
            | RuntimeEvent::Reopen { .. }
            | RuntimeEvent::SystemWake
            | RuntimeEvent::SystemNotificationAuthorization { .. }
            | RuntimeEvent::SystemNotificationResponse(_) => {
                unreachable!("handled before window routing")
            }
            RuntimeEvent::Accessibility(event) => match event.window_event {
                AccessibilityWindowEvent::InitialTreeRequested => {
                    let window_title = self.config.title.as_str();
                    let window = self.window.as_mut().expect("window checked above");
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
                AccessibilityWindowEvent::AccessibilityDeactivated => {}
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
                    if state.ui.advance_scrollbars(now) {
                        redraw = true;
                    }
                    if state.ui.advance_tooltips(now) {
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
                    )
                })
                .unwrap_or((None, None, None, None, None));
            let pending_deadline = self.pending_input.as_ref().map(|pending| pending.deadline);
            let window_deadline = [
                pending_deadline,
                image_deadline,
                animation_deadline,
                scrollbar_deadline,
                tooltip_deadline,
                view_deadline,
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
    state.view_dirty = true;
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
            .set_fullscreen(Some(Fullscreen::Borderless(None))),
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
            .set_fullscreen(Some(Fullscreen::Borderless(None)));
    } else {
        apply_window_bounds(state, WindowBounds::Windowed(state.restore_bounds));
    }
    true
}

fn sane_scale_factor(value: f64) -> f32 {
    if value.is_finite() && value > 0.0 {
        value as f32
    } else {
        1.0
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

fn window_buttons(config: &AppConfig) -> WindowButtons {
    let mut buttons = WindowButtons::CLOSE;
    if config.is_minimizable {
        buttons |= WindowButtons::MINIMIZE;
    }
    if config.is_resizable {
        buttons |= WindowButtons::MAXIMIZE;
    }
    buttons
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

fn map_key(key: &WinitKey) -> Key {
    match key {
        WinitKey::Character(value) => Key::Character(value.to_string()),
        WinitKey::Named(NamedKey::ArrowUp) => Key::ArrowUp,
        WinitKey::Named(NamedKey::ArrowDown) => Key::ArrowDown,
        WinitKey::Named(NamedKey::ArrowLeft) => Key::ArrowLeft,
        WinitKey::Named(NamedKey::ArrowRight) => Key::ArrowRight,
        WinitKey::Named(NamedKey::PageUp) => Key::PageUp,
        WinitKey::Named(NamedKey::PageDown) => Key::PageDown,
        WinitKey::Named(NamedKey::Home) => Key::Home,
        WinitKey::Named(NamedKey::End) => Key::End,
        WinitKey::Named(NamedKey::Enter) => Key::Enter,
        WinitKey::Named(NamedKey::Escape) => Key::Escape,
        WinitKey::Named(NamedKey::Space) => Key::Space,
        WinitKey::Named(NamedKey::Tab) => Key::Tab,
        WinitKey::Named(NamedKey::Backspace) => Key::Backspace,
        WinitKey::Named(NamedKey::Delete) => Key::Delete,
        WinitKey::Named(NamedKey::Insert) => Key::Insert,
        WinitKey::Named(NamedKey::F1) => Key::Function(1),
        WinitKey::Named(NamedKey::F2) => Key::Function(2),
        WinitKey::Named(NamedKey::F3) => Key::Function(3),
        WinitKey::Named(NamedKey::F4) => Key::Function(4),
        WinitKey::Named(NamedKey::F5) => Key::Function(5),
        WinitKey::Named(NamedKey::F6) => Key::Function(6),
        WinitKey::Named(NamedKey::F7) => Key::Function(7),
        WinitKey::Named(NamedKey::F8) => Key::Function(8),
        WinitKey::Named(NamedKey::F9) => Key::Function(9),
        WinitKey::Named(NamedKey::F10) => Key::Function(10),
        WinitKey::Named(NamedKey::F11) => Key::Function(11),
        WinitKey::Named(NamedKey::F12) => Key::Function(12),
        WinitKey::Named(NamedKey::F13) => Key::Function(13),
        WinitKey::Named(NamedKey::F14) => Key::Function(14),
        WinitKey::Named(NamedKey::F15) => Key::Function(15),
        WinitKey::Named(NamedKey::F16) => Key::Function(16),
        WinitKey::Named(NamedKey::F17) => Key::Function(17),
        WinitKey::Named(NamedKey::F18) => Key::Function(18),
        WinitKey::Named(NamedKey::F19) => Key::Function(19),
        WinitKey::Named(NamedKey::F20) => Key::Function(20),
        WinitKey::Named(NamedKey::F21) => Key::Function(21),
        WinitKey::Named(NamedKey::F22) => Key::Function(22),
        WinitKey::Named(NamedKey::F23) => Key::Function(23),
        WinitKey::Named(NamedKey::F24) => Key::Function(24),
        _ => Key::Other,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
            .on_system_notification_response(|_, _| {});

        assert!(app.application_callbacks.open_urls.is_some());
        assert!(app.application_callbacks.reopen.is_some());
        assert!(app.application_callbacks.system_wake.is_some());
        assert!(
            app.application_callbacks
                .system_notification_response
                .is_some()
        );

        let app = App::new(CounterView(0));
        assert!(app.application_callbacks.open_urls.is_none());
        assert!(app.application_callbacks.reopen.is_none());
        assert!(app.application_callbacks.system_wake.is_none());
        assert!(
            app.application_callbacks
                .system_notification_response
                .is_none()
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
    }

    #[test]
    fn window_state_observation_is_declarative() {
        let mut listeners = ListenerRegistry {
            observes_window_state: true,
            ..ListenerRegistry::default()
        };

        listeners.clear();

        assert!(!listeners.observes_window_state);
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
