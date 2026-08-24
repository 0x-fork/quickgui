use std::{collections::HashMap, marker::PhantomData, sync::Arc, time::Instant};

use thiserror::Error;
use winit::{
    application::ApplicationHandler,
    dpi::LogicalSize,
    event::{ElementState, Ime, MouseScrollDelta, WindowEvent},
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    keyboard::{Key as WinitKey, ModifiersState, NamedKey},
    window::{CursorIcon, Window, WindowId},
};

use crate::{
    Color, ElementId, IntoElement, Point, Scene, Size, Vector,
    event::{Event, EventContext, Key, Modifiers, MouseButton},
    metrics::{FrameMetrics, MetricsTracker},
    renderer::{GpuRenderer, RenderOutcome},
    scheduler::FrameScheduler,
    ui_tree::UiTree,
};

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
        let previous = self.listeners.insert(id, Arc::new(callback));
        assert!(
            previous.is_none(),
            "listener id {id:?} was registered more than once"
        );
        ClickListener {
            id,
            marker: PhantomData,
        }
    }
}

type ListenerRegistry<V> = HashMap<ElementId, Arc<dyn Fn(&mut V, &mut EventContext)>>;

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
        let event_loop = EventLoop::new()?;
        event_loop.set_control_flow(ControlFlow::Wait);
        let mut runtime = Runtime::new(self.view, self.config);
        event_loop.run_app(&mut runtime)?;
        match runtime.fatal_error {
            Some(error) => Err(error),
            None => Ok(()),
        }
    }
}

struct RuntimeWindow<V> {
    renderer: GpuRenderer,
    ui: UiTree,
    scheduler: FrameScheduler,
    scene: Scene,
    metrics: MetricsTracker,
    scale_factor: f32,
    logical_size: Size,
    pointer: Option<Point>,
    pointer_cursor: bool,
    view_dirty: bool,
    listeners: ListenerRegistry<V>,
    // The window is last so GPU surface state is dropped before its native handle.
    window: Arc<Window>,
}

struct Runtime<V> {
    view: V,
    config: AppConfig,
    window: Option<RuntimeWindow<V>>,
    modifiers: Modifiers,
    fatal_error: Option<AppError>,
}

impl<V: View> Runtime<V> {
    fn new(view: V, config: AppConfig) -> Self {
        Self {
            view,
            config,
            window: None,
            modifiers: Modifiers::default(),
            fatal_error: None,
        }
    }

    fn fail(&mut self, event_loop: &ActiveEventLoop, error: AppError) {
        tracing::error!(%error, "QuickGUI is exiting after a fatal error");
        self.fatal_error = Some(error);
        event_loop.exit();
    }

    fn dispatch(&mut self, event_loop: &ActiveEventLoop, event: Event, force_redraw: bool) {
        let mut cx = EventContext::default();
        self.view.event(&event, &mut cx);
        if cx.exit {
            event_loop.exit();
            return;
        }
        if let Some(state) = &mut self.window {
            if cx.invalidate {
                state.view_dirty = true;
            }
            if (force_redraw || cx.invalidate) && state.scheduler.invalidate() {
                state.window.request_redraw();
            }
        }
    }

    fn redraw(&mut self, event_loop: &ActiveEventLoop) {
        let Some(state) = &mut self.window else {
            return;
        };
        state.scheduler.begin_redraw();

        let scroll = state.scheduler.take_scroll();
        if !scroll.is_zero() && !state.ui.scroll_at(state.pointer, scroll) {
            let mut event_cx = EventContext::default();
            self.view.event(&Event::Scroll(scroll), &mut event_cx);
            if event_cx.exit {
                event_loop.exit();
                return;
            }
            if event_cx.invalidate {
                state.view_dirty = true;
            }
        }

        let started = Instant::now();
        let mut request_animation_frame = false;
        if state.view_dirty {
            let mut view_cx = ViewContext {
                size: state.logical_size,
                scale_factor: state.scale_factor,
                metrics: state.metrics.current(),
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
        state.scene.clear(self.config.background);
        if let Err(error) = state.ui.paint(&mut state.scene) {
            self.fail(event_loop, AppError::View(error.to_string()));
            return;
        }

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

impl<V: View> ApplicationHandler for Runtime<V> {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() || self.fatal_error.is_some() {
            return;
        }

        event_loop.set_control_flow(ControlFlow::Wait);
        let mut attributes = Window::default_attributes()
            .with_title(self.config.title.clone())
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
        window.set_ime_allowed(true);
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
            ui: UiTree::new(),
            scheduler,
            scene: Scene::new(),
            metrics: MetricsTracker::default(),
            scale_factor,
            logical_size,
            pointer: None,
            pointer_cursor: false,
            view_dirty: true,
            listeners: HashMap::new(),
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
                let repaint = state.ui.pointer_moved(point);
                let pointer_cursor = state.ui.wants_pointer_cursor(point);
                if pointer_cursor != state.pointer_cursor {
                    state.pointer_cursor = pointer_cursor;
                    state.window.set_cursor(if pointer_cursor {
                        CursorIcon::Pointer
                    } else {
                        CursorIcon::Default
                    });
                }
                if repaint && state.scheduler.invalidate() {
                    state.window.request_redraw();
                }
                self.dispatch(event_loop, Event::PointerMoved(point), false);
            }
            WindowEvent::CursorLeft { .. } => {
                let state = self.window.as_mut().expect("window checked above");
                state.pointer = None;
                state.pointer_cursor = false;
                state.window.set_cursor(CursorIcon::Default);
                if state.ui.pointer_left() && state.scheduler.invalidate() {
                    state.window.request_redraw();
                }
                self.dispatch(event_loop, Event::PointerLeft, false);
            }
            WindowEvent::MouseInput { state, button, .. } => {
                let pressed = state == ElementState::Pressed;
                let pointer_result = {
                    let window = self.window.as_mut().expect("window checked above");
                    let result = window.ui.pointer_button(window.pointer, pressed);
                    if result.repaint && window.scheduler.invalidate() {
                        window.window.request_redraw();
                    }
                    result
                };
                self.dispatch(
                    event_loop,
                    Event::MouseButton {
                        button: map_mouse_button(button),
                        pressed,
                    },
                    false,
                );
                if let Some(id) = pointer_result.clicked {
                    let listener = self
                        .window
                        .as_ref()
                        .and_then(|window| window.listeners.get(&id).cloned());
                    if let Some(listener) = listener {
                        let mut cx = EventContext::default();
                        listener(&mut self.view, &mut cx);
                        if cx.exit {
                            event_loop.exit();
                            return;
                        }
                        if cx.invalidate
                            && let Some(window) = &mut self.window
                        {
                            window.view_dirty = true;
                            if window.scheduler.invalidate() {
                                window.window.request_redraw();
                            }
                        }
                    }
                    self.dispatch(event_loop, Event::Click(id), false);
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
            WindowEvent::Ime(Ime::Commit(text)) => {
                self.dispatch(event_loop, Event::TextInput(text), false);
            }
            WindowEvent::Focused(focused) => {
                self.dispatch(event_loop, Event::Focused(focused), true);
            }
            _ => {}
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
}
