use std::{
    any::{Any, TypeId},
    collections::HashMap,
    marker::PhantomData,
    sync::Arc,
    time::{Duration, Instant},
};

use accesskit::{Action as AccessibilityAction, ActionData, ActionRequest};
use accesskit_winit::{
    Adapter as AccessibilityAdapter, Event as AccessibilityEvent,
    WindowEvent as AccessibilityWindowEvent,
};
use arboard::Clipboard;
use thiserror::Error;
use winit::{
    application::ApplicationHandler,
    dpi::{LogicalPosition, LogicalSize},
    event::{ElementState, Ime, MouseScrollDelta, WindowEvent},
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop, EventLoopProxy},
    keyboard::{Key as WinitKey, ModifiersState, NamedKey},
    window::{CursorIcon, Window, WindowId},
};

use crate::{
    Action, ActionListener, AnyAction, Color, ElementId, FocusHandle, IntoElement, KeyBinding,
    Keymap, Keystroke, Menu, OsAction, Point, Scene, Size, Vector,
    event::{Event, EventContext, Key, Modifiers, MouseButton},
    menu::{MenuAction, collect_menu_actions},
    metrics::{FrameMetrics, MetricsTracker},
    renderer::{GpuRenderer, RenderOutcome},
    scheduler::FrameScheduler,
    ui_tree::{DismissRequest, InputResult, UiTree},
};

#[cfg(target_os = "macos")]
use crate::macos::{MacFirstFrameGuard, MacNativeHost};
#[cfg(target_os = "macos")]
use crate::macos_menu::{MacMenuHost, MacMenuItemState};

pub(crate) enum RuntimeEvent {
    Accessibility(AccessibilityEvent),
    MenuWillOpen,
    MenuAction(usize),
}

impl From<AccessibilityEvent> for RuntimeEvent {
    fn from(event: AccessibilityEvent) -> Self {
        Self::Accessibility(event)
    }
}

/// GPU selection policy. `Balanced` lets WGPU choose the most appropriate adapter.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum PerformanceProfile {
    #[default]
    Balanced,
    LowPower,
    HighPerformance,
}

/// Window and renderer defaults used by [`App`].
#[derive(Clone, Debug)]
pub struct AppConfig {
    pub title: String,
    pub size: Size,
    pub minimum_size: Option<Size>,
    pub background: Color,
    pub performance_profile: PerformanceProfile,
    /// Logical pixels represented by one platform line-wheel unit.
    pub line_scroll_pixels: f32,
    /// How long an incomplete multi-stroke key binding waits before its prefix is replayed.
    pub key_sequence_timeout: Duration,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            title: "QuickGUI".to_owned(),
            size: Size::new(960.0, 640.0),
            minimum_size: Some(Size::new(320.0, 240.0)),
            background: Color::rgb8(18, 18, 20),
            performance_profile: PerformanceProfile::Balanced,
            line_scroll_pixels: 40.0,
            key_sequence_timeout: Duration::from_secs(1),
        }
    }
}

/// A retained application view. It is only rendered after explicit invalidation or OS damage.
pub trait View: Sized + 'static {
    fn event(&mut self, _event: &Event, _cx: &mut EventContext) {}
    fn render(&mut self, cx: &mut ViewContext<'_, Self>) -> impl IntoElement;
}

/// Context provided while a view declares its element tree.
pub struct ViewContext<'a, V> {
    size: Size,
    scale_factor: f32,
    metrics: FrameMetrics,
    focused: Option<ElementId>,
    focused_path: Vec<ElementId>,
    request_animation_frame: bool,
    listeners: &'a mut ListenerRegistry<V>,
}

impl<V> ViewContext<'_, V> {
    pub fn size(&self) -> Size {
        self.size
    }

    pub fn scale_factor(&self) -> f32 {
        self.scale_factor
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

    /// Keep rendering at the display's cadence until a future frame omits this call.
    pub fn request_animation_frame(&mut self) {
        self.request_animation_frame = true;
    }

    /// Register a stable, view-local click callback for use with [`crate::Element::on_click`].
    pub fn listener(
        &mut self,
        id: impl Into<ElementId>,
        callback: impl Fn(&mut V, &mut EventContext) + 'static,
    ) -> ClickListener<V> {
        let id = id.into();
        let previous = self.listeners.clicks.insert(id, Arc::new(callback));
        assert!(
            previous.is_none(),
            "listener id {id:?} was registered more than once"
        );
        ClickListener {
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
        let previous = self.listeners.inputs.insert(id, Arc::new(callback));
        assert!(
            previous.is_none(),
            "input listener id {id:?} was registered more than once"
        );
        InputListener {
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
        let previous = self.listeners.dismisses.insert(id, Arc::new(callback));
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
        let erased: ActionCallback<V> = Arc::new(move |view, action, context| {
            let action = action
                .downcast_ref::<A>()
                .expect("action listener received the wrong concrete action type");
            callback(view, action, context);
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

type ClickCallback<V> = Arc<dyn Fn(&mut V, &mut EventContext)>;
type InputCallback<V> = Arc<dyn Fn(&mut V, &str, &mut EventContext)>;
type ActionCallback<V> = Arc<dyn Fn(&mut V, &dyn Any, &mut EventContext)>;

struct ListenerRegistry<V> {
    clicks: HashMap<ElementId, ClickCallback<V>>,
    inputs: HashMap<ElementId, InputCallback<V>>,
    dismisses: HashMap<ElementId, ClickCallback<V>>,
    actions: HashMap<(ElementId, TypeId), Vec<ActionCallback<V>>>,
}

impl<V> ListenerRegistry<V> {
    fn clear(&mut self) {
        self.clicks.clear();
        self.inputs.clear();
        self.dismisses.clear();
        self.actions.clear();
    }
}

impl<V> Default for ListenerRegistry<V> {
    fn default() -> Self {
        Self {
            clicks: HashMap::new(),
            inputs: HashMap::new(),
            dismisses: HashMap::new(),
            actions: HashMap::new(),
        }
    }
}

/// An opaque click binding returned by [`ViewContext::listener`].
pub struct ClickListener<V> {
    id: ElementId,
    marker: PhantomData<fn(&mut V)>,
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

impl<V> InputListener<V> {
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

/// Configures and runs one retained QuickGUI view.
pub struct App<V> {
    view: V,
    config: AppConfig,
    keymap: Keymap,
    menus: Vec<Menu>,
}

impl<V: View> App<V> {
    pub fn new(view: V) -> Self {
        Self {
            view,
            config: AppConfig::default(),
            keymap: Keymap::default(),
            menus: Vec::new(),
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
        self.config.size = Size::new(width, height);
        self
    }

    pub fn performance_profile(mut self, profile: PerformanceProfile) -> Self {
        self.config.performance_profile = profile;
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

    pub fn run(self) -> Result<(), AppError> {
        let event_loop = EventLoop::with_user_event().build()?;
        event_loop.set_control_flow(ControlFlow::Wait);
        let mut runtime = Runtime::new(
            self.view,
            self.config,
            self.keymap,
            self.menus,
            event_loop.create_proxy(),
        );
        event_loop.run_app(&mut runtime)?;
        match runtime.fatal_error {
            Some(error) => Err(error),
            None => Ok(()),
        }
    }
}

struct RuntimeWindow<V> {
    renderer: GpuRenderer,
    #[cfg(target_os = "macos")]
    native_host: Option<MacNativeHost>,
    #[cfg(target_os = "macos")]
    first_frame_guard: Option<MacFirstFrameGuard>,
    ui: UiTree,
    scheduler: FrameScheduler,
    scene: Scene,
    metrics: MetricsTracker,
    scale_factor: f32,
    logical_size: Size,
    pointer: Option<Point>,
    cursor: CursorIcon,
    ime_target: Option<ElementId>,
    view_dirty: bool,
    listeners: ListenerRegistry<V>,
    accessibility: AccessibilityAdapter,
    // The window is last so GPU surface state is dropped before its native handle.
    window: Arc<Window>,
}

struct Runtime<V> {
    view: V,
    config: AppConfig,
    keymap: Keymap,
    #[cfg(target_os = "macos")]
    menus: Vec<Menu>,
    menu_actions: Vec<MenuAction>,
    #[cfg(target_os = "macos")]
    menu_host: Option<MacMenuHost>,
    pending_input: Option<PendingInput>,
    window: Option<RuntimeWindow<V>>,
    modifiers: Modifiers,
    fatal_error: Option<AppError>,
    event_proxy: EventLoopProxy<RuntimeEvent>,
    clipboard: Option<Clipboard>,
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

impl<V: View> Runtime<V> {
    fn new(
        view: V,
        config: AppConfig,
        keymap: Keymap,
        menus: Vec<Menu>,
        event_proxy: EventLoopProxy<RuntimeEvent>,
    ) -> Self {
        let menu_actions = collect_menu_actions(&menus);
        Self {
            view,
            config,
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
        }
    }

    fn fail(&mut self, event_loop: &ActiveEventLoop, error: AppError) {
        tracing::error!(%error, "QuickGUI is exiting after a fatal error");
        self.fatal_error = Some(error);
        event_loop.exit();
    }

    fn dispatch(&mut self, event_loop: &ActiveEventLoop, event: Event, force_redraw: bool) -> bool {
        let mut cx = EventContext::default();
        self.view.event(&event, &mut cx);
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
        let actions = std::mem::take(&mut cx.actions);
        let menus = cx.menus.take();
        let mut focus_changed = false;
        if let Some(state) = &mut self.window {
            let previous_focus = state.ui.focused();
            if let Some(request) = cx.focus {
                match request {
                    Some(id) => {
                        state.ui.focus(id);
                    }
                    None => {
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
            if (force_redraw || cx.invalidate || focus_changed) && state.scheduler.invalidate() {
                state.window.request_redraw();
            }
        }
        if focus_changed && announce_focus {
            let focused = self.window.as_ref().and_then(|state| state.ui.focused());
            let mut focus_cx = EventContext::default();
            self.view
                .event(&Event::FocusChanged(focused), &mut focus_cx);
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
                let mut cx = EventContext::default();
                listener(&mut self.view, action.as_any(), &mut cx);
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
        let next_host = if menus.is_empty() {
            None
        } else {
            match MacMenuHost::new(&menus, self.event_proxy.clone()) {
                Ok(host) => Some(host),
                Err(error) => {
                    self.fail(event_loop, AppError::Platform(error));
                    return false;
                }
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
        if window.ui.focused_text_input().is_none() {
            return false;
        }
        match action {
            OsAction::Cut | OsAction::Copy => window
                .ui
                .selected_text()
                .is_some_and(|selection| !selection.is_empty()),
            OsAction::Paste | OsAction::SelectAll => true,
            OsAction::Undo => window.ui.input_can_undo(),
            OsAction::Redo => window.ui.input_can_redo(),
        }
    }

    fn invoke_os_action(&mut self, event_loop: &ActiveEventLoop, action: OsAction) -> bool {
        if self
            .window
            .as_ref()
            .and_then(|window| window.ui.focused_text_input())
            .is_none()
        {
            return false;
        }
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
                }
                true
            }
            OsAction::Cut => {
                let selected = self
                    .window
                    .as_ref()
                    .and_then(|window| window.ui.selected_text());
                let copied = selected.is_some_and(|selection| {
                    self.clipboard()
                        .is_some_and(|clipboard| clipboard.set_text(selection.as_ref()).is_ok())
                });
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
                let result = self
                    .window
                    .as_mut()
                    .map(|window| window.ui.input_select_all())
                    .unwrap_or_default();
                self.apply_input_result(event_loop, result, false);
                true
            }
            OsAction::Undo => {
                let result = self
                    .window
                    .as_mut()
                    .map(|window| window.ui.input_undo())
                    .unwrap_or_default();
                self.apply_input_result(event_loop, result, true)
            }
            OsAction::Redo => {
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
        host.update(&states, native_focus_active);
    }

    fn invoke_click(&mut self, event_loop: &ActiveEventLoop, id: ElementId) {
        let listener = self
            .window
            .as_ref()
            .and_then(|window| window.listeners.clicks.get(&id).cloned());
        if let Some(listener) = listener {
            let mut cx = EventContext::default();
            listener(&mut self.view, &mut cx);
            if !self.apply_event_context(event_loop, cx, false, true) {
                return;
            }
        }
        self.dispatch(event_loop, Event::Click(id), false);
    }

    fn invoke_dismiss(&mut self, event_loop: &ActiveEventLoop, request: DismissRequest) {
        let listener = self
            .window
            .as_ref()
            .and_then(|window| window.listeners.dismisses.get(&request.id).cloned());
        let mut cx = EventContext::default();
        if let Some(focus) = request.restore_focus {
            cx.focus = Some(Some(focus));
        }
        if let Some(listener) = listener {
            listener(&mut self.view, &mut cx);
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
            let mut cx = EventContext::default();
            listener(&mut self.view, value, &mut cx);
            return self.apply_event_context(event_loop, cx, false, true);
        }
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

    fn handle_text_input_key(
        &mut self,
        event_loop: &ActiveEventLoop,
        key: &Key,
        modifiers: Modifiers,
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
        let result = match key {
            Key::ArrowLeft if primary => self
                .window
                .as_mut()
                .map(|window| window.ui.input_move_home(extend)),
            Key::ArrowRight if primary => self
                .window
                .as_mut()
                .map(|window| window.ui.input_move_end(extend)),
            Key::ArrowLeft => self
                .window
                .as_mut()
                .map(|window| window.ui.input_move_left(extend)),
            Key::ArrowRight => self
                .window
                .as_mut()
                .map(|window| window.ui.input_move_right(extend)),
            Key::Home => self
                .window
                .as_mut()
                .map(|window| window.ui.input_move_home(extend)),
            Key::End => self
                .window
                .as_mut()
                .map(|window| window.ui.input_move_end(extend)),
            Key::Backspace => self
                .window
                .as_mut()
                .map(|window| window.ui.input_backspace()),
            Key::Delete => self.window.as_mut().map(|window| window.ui.input_delete()),
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
                    .and_then(|window| window.ui.selected_text());
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
                    .and_then(|window| window.ui.selected_text());
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

        let mut handled_by_input = self.handle_text_input_key(event_loop, &key, modifiers);
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
            if !self.apply_input_result(event_loop, result, true)
                || !self.dispatch(event_loop, Event::TextInput(text.to_owned()), false)
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
        let mut cx = EventContext::default();
        self.view.event(&Event::FocusChanged(focused), &mut cx);
        self.apply_event_context(event_loop, cx, false, false);
        #[cfg(target_os = "macos")]
        self.sync_native_menu_state();
    }

    fn redraw(&mut self, event_loop: &ActiveEventLoop) {
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
        let Some(state) = &mut self.window else {
            return;
        };
        state.scheduler.begin_redraw();

        let scroll = state.scheduler.take_scroll();
        let scroll_for_view =
            (!scroll.is_zero() && !state.ui.scroll_at(state.pointer, scroll)).then_some(scroll);
        if let Some(scroll) = scroll_for_view {
            let mut event_cx = EventContext::default();
            self.view.event(&Event::Scroll(scroll), &mut event_cx);
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
            let mut view_cx = ViewContext {
                size: state.logical_size,
                scale_factor: state.scale_factor,
                metrics: state.metrics.current(),
                focused: state.ui.focused(),
                focused_path,
                request_animation_frame: false,
                listeners: &mut state.listeners,
            };
            view_cx.listeners.clear();
            let root = self.view.render(&mut view_cx).into_element();
            request_animation_frame = view_cx.request_animation_frame;
            if let Err(error) = state.ui.set_root(
                root,
                state.logical_size,
                state.scale_factor,
                &mut state.renderer,
            ) {
                self.fail(event_loop, AppError::View(error.to_string()));
                return;
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

        state.scene.clear(self.config.background);
        if let Err(error) = state.ui.paint(&mut state.scene, &mut state.renderer) {
            self.fail(event_loop, AppError::View(error.to_string()));
            return;
        }
        #[cfg(target_os = "macos")]
        {
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
        if ime_target.is_some()
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
            Ok(RenderOutcome::Presented(stats)) => {
                #[cfg(target_os = "macos")]
                if state.first_frame_guard.is_some()
                    && let Err(error) = state.renderer.wait_for_submitted_work()
                {
                    self.fail(event_loop, AppError::Render(error.to_string()));
                    return;
                }
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
        #[cfg(target_os = "macos")]
        self.sync_native_menu_state();
    }
}

impl<V: View> ApplicationHandler<RuntimeEvent> for Runtime<V> {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() || self.fatal_error.is_some() {
            return;
        }

        event_loop.set_control_flow(ControlFlow::Wait);
        let mut attributes = Window::default_attributes()
            .with_title(self.config.title.clone())
            .with_visible(false)
            .with_inner_size(LogicalSize::new(
                self.config.size.width as f64,
                self.config.size.height as f64,
            ));
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
        window.set_ime_allowed(false);
        #[cfg(target_os = "macos")]
        if self.menu_host.is_none() && !self.menus.is_empty() {
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
        let renderer = match pollster::block_on(GpuRenderer::new(
            window.clone(),
            event_loop,
            self.config.performance_profile,
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
        let logical_size = Size::new(
            physical.width as f32 / scale_factor,
            physical.height as f32 / scale_factor,
        );
        let mut scheduler = FrameScheduler::default();
        scheduler.invalidate();
        self.window = Some(RuntimeWindow {
            renderer,
            #[cfg(target_os = "macos")]
            native_host: None,
            #[cfg(target_os = "macos")]
            first_frame_guard,
            ui: UiTree::new(),
            scheduler,
            scene: Scene::new(),
            metrics: MetricsTracker::default(),
            scale_factor,
            logical_size,
            pointer: None,
            cursor: CursorIcon::Default,
            ime_target: None,
            view_dirty: true,
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
                    return;
                }
            };

            // With the content detached, WGPU can acquire the actual CAMetalLayer drawable even
            // though the NSWindow remains hidden. The renderer completes that real surface frame
            // and removes the shield before RAII reattaches the unchanged content view.
            self.redraw(event_loop);
            drop(detached);
            if self.fatal_error.is_some() {
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
                return;
            }
        }

        if let Some(state) = &mut self.window {
            state.window.set_visible(true);
            state.scheduler.invalidate();
            state.window.request_redraw();
        }
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
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(physical) => {
                let state = self.window.as_mut().expect("window checked above");
                state.renderer.resize(physical.width, physical.height);
                state.logical_size = Size::new(
                    physical.width as f32 / state.scale_factor,
                    physical.height as f32 / state.scale_factor,
                );
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
                state.logical_size = Size::new(
                    physical.width as f32 / state.scale_factor,
                    physical.height as f32 / state.scale_factor,
                );
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
                state.scheduler.invalidate();
                state.window.request_redraw();
            }
            WindowEvent::Occluded(true) => {}
            WindowEvent::RedrawRequested => self.redraw(event_loop),
            WindowEvent::CursorMoved { position, .. } => {
                let state = self.window.as_mut().expect("window checked above");
                let scale = state.scale_factor;
                let point = Point::new(position.x as f32 / scale, position.y as f32 / scale);
                state.pointer = Some(point);
                let repaint = {
                    let RuntimeWindow { ui, renderer, .. } = state;
                    ui.pointer_moved(point, renderer)
                };
                let cursor = if state.ui.wants_text_cursor(point) {
                    CursorIcon::Text
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
                self.dispatch(event_loop, Event::PointerMoved(point), false);
            }
            WindowEvent::CursorLeft { .. } => {
                let state = self.window.as_mut().expect("window checked above");
                state.pointer = None;
                if state.cursor != CursorIcon::Default {
                    state.cursor = CursorIcon::Default;
                    state.window.set_cursor(CursorIcon::Default);
                }
                if state.ui.pointer_left() && state.scheduler.invalidate() {
                    state.window.request_redraw();
                }
                self.dispatch(event_loop, Event::PointerLeft, false);
            }
            WindowEvent::MouseInput { state, button, .. } => {
                let pressed = state == ElementState::Pressed;
                let button = map_mouse_button(button);
                let (pointer_result, previous_focus) = {
                    let window = self.window.as_mut().expect("window checked above");
                    let previous_focus = window.ui.focused();
                    let result = if button == MouseButton::Left {
                        let RuntimeWindow { ui, renderer, .. } = window;
                        ui.pointer_button(
                            window.pointer,
                            pressed,
                            self.modifiers.contains(Modifiers::SHIFT),
                            renderer,
                        )
                    } else {
                        crate::ui_tree::PointerResult {
                            repaint: false,
                            clicked: None,
                            dismissed: None,
                        }
                    };
                    if result.repaint && window.scheduler.invalidate() {
                        window.window.request_redraw();
                    }
                    (result, previous_focus)
                };
                self.announce_focus_change(event_loop, previous_focus);
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
                if self.apply_input_result(event_loop, result, true) {
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
                self.dispatch(event_loop, Event::Focused(focused), true);
            }
            _ => {}
        }
    }

    fn user_event(&mut self, event_loop: &ActiveEventLoop, event: RuntimeEvent) {
        let event = match event {
            RuntimeEvent::Accessibility(event) => event,
            RuntimeEvent::MenuWillOpen => {
                self.pending_input = None;
                #[cfg(target_os = "macos")]
                self.sync_native_menu_state();
                return;
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
                return;
            }
        };
        let Some(window) = self.window.as_ref() else {
            return;
        };
        if window.window.id() != event.window_id {
            return;
        }

        match event.window_event {
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
                let previous_focus = self.window.as_ref().and_then(|window| window.ui.focused());
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
                                    .input_set_accessibility_selection(target, &selection)
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
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        if self
            .pending_input
            .as_ref()
            .is_some_and(|pending| pending.deadline <= Instant::now())
        {
            self.flush_pending_input(event_loop);
        }
        if let Some(pending) = &self.pending_input {
            event_loop.set_control_flow(ControlFlow::WaitUntil(pending.deadline));
        } else {
            event_loop.set_control_flow(ControlFlow::Wait);
        }
    }
}

fn sane_scale_factor(value: f64) -> f32 {
    if value.is_finite() && value > 0.0 {
        value as f32
    } else {
        1.0
    }
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

    #[test]
    fn invalid_scale_factors_fall_back_to_one() {
        assert_eq!(sane_scale_factor(0.0), 1.0);
        assert_eq!(sane_scale_factor(f64::NAN), 1.0);
        assert_eq!(sane_scale_factor(2.0), 2.0);
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
        } else {
            assert!(primary_modifier(Modifiers::CONTROL));
            assert!(!primary_modifier(Modifiers::SUPER));
        }
    }
}
