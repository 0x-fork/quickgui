use std::{collections::HashMap, marker::PhantomData, sync::Arc, time::Instant};

use accesskit::{Action, ActionData, ActionRequest};
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
    Color, ElementId, FocusHandle, IntoElement, Point, Scene, Size, Vector,
    event::{Event, EventContext, Key, Modifiers, MouseButton},
    metrics::{FrameMetrics, MetricsTracker},
    renderer::{GpuRenderer, RenderOutcome},
    scheduler::FrameScheduler,
    ui_tree::{DismissRequest, InputResult, UiTree},
};

#[cfg(target_os = "macos")]
use crate::macos::MacNativeHost;

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
}

type ClickCallback<V> = Arc<dyn Fn(&mut V, &mut EventContext)>;
type InputCallback<V> = Arc<dyn Fn(&mut V, &str, &mut EventContext)>;

struct ListenerRegistry<V> {
    clicks: HashMap<ElementId, ClickCallback<V>>,
    inputs: HashMap<ElementId, InputCallback<V>>,
    dismisses: HashMap<ElementId, ClickCallback<V>>,
}

impl<V> ListenerRegistry<V> {
    fn clear(&mut self) {
        self.clicks.clear();
        self.inputs.clear();
        self.dismisses.clear();
    }
}

impl<V> Default for ListenerRegistry<V> {
    fn default() -> Self {
        Self {
            clicks: HashMap::new(),
            inputs: HashMap::new(),
            dismisses: HashMap::new(),
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
}

/// Configures and runs one retained QuickGUI view.
pub struct App<V> {
    view: V,
    config: AppConfig,
}

impl<V: View> App<V> {
    pub fn new(view: V) -> Self {
        Self {
            view,
            config: AppConfig::default(),
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

    pub fn run(self) -> Result<(), AppError> {
        let event_loop = EventLoop::with_user_event().build()?;
        event_loop.set_control_flow(ControlFlow::Wait);
        let mut runtime = Runtime::new(self.view, self.config, event_loop.create_proxy());
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
    window: Option<RuntimeWindow<V>>,
    modifiers: Modifiers,
    fatal_error: Option<AppError>,
    accessibility_proxy: EventLoopProxy<AccessibilityEvent>,
    clipboard: Option<Clipboard>,
}

impl<V: View> Runtime<V> {
    fn new(
        view: V,
        config: AppConfig,
        accessibility_proxy: EventLoopProxy<AccessibilityEvent>,
    ) -> Self {
        Self {
            view,
            config,
            window: None,
            modifiers: Modifiers::default(),
            fatal_error: None,
            accessibility_proxy,
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
        cx: EventContext,
        force_redraw: bool,
        announce_focus: bool,
    ) -> bool {
        if cx.exit {
            event_loop.exit();
            return false;
        }
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
            return self.apply_event_context(event_loop, focus_cx, false, false);
        }
        true
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

    fn handle_text_input_key(&mut self, event_loop: &ActiveEventLoop, key: &Key) -> bool {
        let focused = self
            .window
            .as_ref()
            .and_then(|window| window.ui.focused_text_input());
        if focused.is_none() {
            return false;
        }

        let extend = self.modifiers.contains(Modifiers::SHIFT);
        let primary = primary_modifier(self.modifiers);
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

    fn announce_focus_change(&mut self, event_loop: &ActiveEventLoop, previous: Option<ElementId>) {
        let focused = self.window.as_ref().and_then(|state| state.ui.focused());
        if previous == focused {
            return;
        }
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
            let mut view_cx = ViewContext {
                size: state.logical_size,
                scale_factor: state.scale_factor,
                metrics: state.metrics.current(),
                focused: state.ui.focused(),
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
                host.reconcile(state.ui.native_views());
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
                state.metrics.record(started.elapsed(), stats);
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
    }
}

impl<V: View> ApplicationHandler<AccessibilityEvent> for Runtime<V> {
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
        let accessibility = AccessibilityAdapter::with_event_loop_proxy(
            event_loop,
            &window,
            self.accessibility_proxy.clone(),
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
        let scale_factor = sane_scale_factor(window.scale_factor());
        let physical = window.inner_size();
        let logical_size = Size::new(
            physical.width as f32 / scale_factor,
            physical.height as f32 / scale_factor,
        );
        let mut scheduler = FrameScheduler::default();
        scheduler.invalidate();
        window.request_redraw();
        self.window = Some(RuntimeWindow {
            renderer,
            #[cfg(target_os = "macos")]
            native_host: None,
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
        if let Some(state) = &self.window {
            state.window.set_visible(true);
        }

        self.dispatch(
            event_loop,
            Event::Resized {
                logical_size,
                scale_factor,
            },
            false,
        );
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
                if event.state == ElementState::Pressed
                    && !event.repeat
                    && matches!(&key, Key::Escape)
                    && let Some(request) = self
                        .window
                        .as_ref()
                        .and_then(|window| window.ui.dismiss_topmost())
                {
                    self.invoke_dismiss(event_loop, request);
                    return;
                }
                if event.state == ElementState::Pressed {
                    let mut handled_by_input = self.handle_text_input_key(event_loop, &key);
                    if !handled_by_input
                        && !self
                            .modifiers
                            .intersects(Modifiers::CONTROL | Modifiers::SUPER)
                        && let Some(text) = event.text.as_deref().filter(|text| {
                            !text.is_empty()
                                && text.chars().all(|character| !character.is_control())
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
                            return;
                        }
                        handled_by_input = true;
                    }
                    match &key {
                        _ if handled_by_input => {}
                        Key::Tab => {
                            let previous_focus =
                                self.window.as_ref().and_then(|window| window.ui.focused());
                            if let Some(window) = &mut self.window {
                                window
                                    .ui
                                    .focus_next(self.modifiers.contains(Modifiers::SHIFT));
                            }
                            self.announce_focus_change(event_loop, previous_focus);
                        }
                        Key::Enter | Key::Space if !event.repeat => {
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
                let event = if event.state == ElementState::Pressed {
                    Event::KeyDown {
                        key,
                        modifiers: self.modifiers,
                        repeat: event.repeat,
                    }
                } else {
                    Event::KeyUp {
                        key,
                        modifiers: self.modifiers,
                    }
                };
                self.dispatch(event_loop, event, false);
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

    fn user_event(&mut self, event_loop: &ActiveEventLoop, event: AccessibilityEvent) {
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
                    Action::Focus => {
                        if let Some(window) = &mut self.window {
                            window.ui.focus(target);
                        }
                    }
                    Action::Blur => {
                        if let Some(window) = &mut self.window
                            && window.ui.focused() == Some(target)
                        {
                            window.ui.blur();
                        }
                    }
                    Action::Click => {
                        if let Some(window) = &mut self.window {
                            window.ui.focus(target);
                        }
                        self.announce_focus_change(event_loop, previous_focus);
                        self.invoke_click(event_loop, target);
                        return;
                    }
                    Action::SetValue => {
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
                    Action::SetTextSelection => {
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
        event_loop.set_control_flow(ControlFlow::Wait);
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
