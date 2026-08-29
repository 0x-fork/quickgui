use std::{
    any::type_name,
    cell::Cell,
    collections::{HashMap, HashSet, VecDeque},
    marker::PhantomData,
    panic::{AssertUnwindSafe, catch_unwind},
    rc::Rc,
    time::{Duration, Instant},
};

use thiserror::Error;

use super::*;
use crate::{
    ClipboardError, MAX_MOUSE_EVENT_PATH, TextHighlight, TextId, TextStyle, TextWrap,
    renderer::{
        OffscreenRenderer, SharedFontSystem, StyledTextGeometry, TextLayoutEngine,
        create_shared_font_system,
    },
};

/// Maximum scheduler/effect passes one public test operation may execute before failing.
///
/// This turns accidental recursive invalidation, action, entity-event, or foreground-task loops
/// into a deterministic test failure instead of hanging a test process.
pub const MAX_TEST_EFFECT_TURNS: usize = 1_024;
const MAX_TEST_PENDING_DISPATCHES: usize = 4_096;

/// CPU-only text estimates used to materialize size-dependent declarations for semantic tests.
/// Exact geometry and screenshots still replace these estimates through `OffscreenRenderer`.
struct SemanticTextLayout;

impl SemanticTextLayout {
    fn measure(content: &str, style: &TextStyle, max_width: Option<f32>) -> Size {
        let line_count = content.lines().count().max(1);
        let longest = content
            .lines()
            .map(|line| line.chars().count())
            .max()
            .unwrap_or(0) as f32;
        let natural_width = longest * style.font_size * 0.55;
        let mut visual_lines = line_count;
        let width = if style.wrap == TextWrap::Word {
            if let Some(max_width) = max_width.filter(|width| *width > 0.0) {
                visual_lines = visual_lines.max((natural_width / max_width).ceil() as usize);
                natural_width.min(max_width)
            } else {
                natural_width
            }
        } else {
            natural_width
        };
        Size::new(width, style.line_height * visual_lines as f32)
    }
}

impl TextLayoutEngine for SemanticTextLayout {
    fn measure_text(
        &mut self,
        _id: TextId,
        content: &std::sync::Arc<str>,
        style: &TextStyle,
        max_width: Option<f32>,
        _scale_factor: f32,
    ) -> Size {
        Self::measure(content, style, max_width)
    }

    fn measure_styled_text(
        &mut self,
        id: TextId,
        content: &std::sync::Arc<str>,
        style: &TextStyle,
        _highlights: &std::sync::Arc<[TextHighlight]>,
        max_width: Option<f32>,
        scale_factor: f32,
    ) -> Size {
        self.measure_text(id, content, style, max_width, scale_factor)
    }

    fn text_geometry(
        &mut self,
        _id: TextId,
        _content: &std::sync::Arc<str>,
        _style: &TextStyle,
        _highlights: Option<&std::sync::Arc<[TextHighlight]>>,
        _width: f32,
        _scale_factor: f32,
        _visible_y: std::ops::Range<f32>,
    ) -> StyledTextGeometry {
        StyledTextGeometry {
            backgrounds: Vec::new(),
            decorations: Vec::new(),
        }
    }

    fn text_caret_position_with_highlights(
        &mut self,
        _id: TextId,
        _content: &std::sync::Arc<str>,
        _style: &TextStyle,
        _highlights: Option<&std::sync::Arc<[TextHighlight]>>,
        _width: f32,
        _scale_factor: f32,
        _index: usize,
    ) -> Point {
        Point::ZERO
    }

    fn text_index_for_point_with_highlights(
        &mut self,
        _id: TextId,
        _content: &std::sync::Arc<str>,
        _style: &TextStyle,
        _highlights: Option<&std::sync::Arc<[TextHighlight]>>,
        _width: f32,
        _scale_factor: f32,
        _point: Point,
    ) -> usize {
        0
    }

    fn text_selection_rects_with_highlights(
        &mut self,
        _id: TextId,
        _content: &std::sync::Arc<str>,
        _style: &TextStyle,
        _highlights: Option<&std::sync::Arc<[TextHighlight]>>,
        _width: f32,
        _scale_factor: f32,
        _visible_y: std::ops::Range<f32>,
        _start: usize,
        _end: usize,
    ) -> Vec<Rect> {
        Vec::new()
    }
}

/// A deterministic test-context failure.
#[derive(Debug, Error)]
pub enum TestAppError {
    #[error("invalid test-window configuration: {0}")]
    InvalidWindow(#[from] WindowCommandError),
    #[error("a simulated native window-tab snapshot is internally inconsistent")]
    InvalidWindowTabState,
    #[error(transparent)]
    Asset(#[from] AssetError),
    #[error("test window {0:?} does not exist")]
    UnknownWindow(WindowHandle),
    #[error("test window {window:?} does not contain a view of type {expected}")]
    WrongViewType {
        window: WindowHandle,
        expected: &'static str,
    },
    #[error("test window {window:?} does not contain element {element:?}")]
    UnknownElement {
        window: WindowHandle,
        element: ElementId,
    },
    #[error("element {element:?} in test window {window:?} is not focusable")]
    NotFocusable {
        window: WindowHandle,
        element: ElementId,
    },
    #[error("element {element:?} in test window {window:?} is not clickable")]
    NotClickable {
        window: WindowHandle,
        element: ElementId,
    },
    #[error("element {element:?} in test window {window:?} does not listen for {kind}")]
    NotListening {
        window: WindowHandle,
        element: ElementId,
        kind: &'static str,
    },
    #[error("element {element:?} in test window {window:?} is not a retained scroll container")]
    NotScrollable {
        window: WindowHandle,
        element: ElementId,
    },
    #[error("test window {0:?} has no focused text input")]
    NoFocusedTextInput(WindowHandle),
    #[error("test view declaration failed: {0}")]
    View(String),
    #[error("a foreground task panicked while the deterministic executor was running")]
    ForegroundTaskPanicked,
    #[error("the deterministic test executor exceeded {MAX_TEST_EFFECT_TURNS} effect turns")]
    EffectTurnLimit,
    #[error("the deterministic test dispatch queue exceeded {MAX_TEST_PENDING_DISPATCHES} entries")]
    DispatchQueueFull,
    #[error("the retained mouse target path exceeded {MAX_MOUSE_EVENT_PATH} ancestors")]
    MouseDispatchPathLimit,
    #[error("the keystroke sequence ended while a longer binding was still pending")]
    IncompleteKeystrokeSequence,
    #[error("native platform requests require an explicit platform test adapter")]
    UnsupportedPlatformRequest,
    #[error(transparent)]
    Visual(#[from] crate::VisualTestError),
    #[error(
        "element {element:?} in test window {window:?} has bounds {actual:?}, expected {expected:?} within {tolerance} logical pixels"
    )]
    GeometryMismatch {
        window: WindowHandle,
        element: ElementId,
        actual: Rect,
        expected: Rect,
        tolerance: f32,
    },
    #[error("visual geometry tolerance must be finite and non-negative")]
    InvalidGeometryTolerance,
}

/// A typed handle for one window owned by [`TestAppContext`].
pub struct TestWindowHandle<V> {
    handle: WindowHandle,
    marker: PhantomData<fn() -> V>,
}

impl<V> TestWindowHandle<V> {
    fn new(handle: WindowHandle) -> Self {
        Self {
            handle,
            marker: PhantomData,
        }
    }

    pub const fn window_handle(self) -> WindowHandle {
        self.handle
    }
}

impl<V> Clone for TestWindowHandle<V> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<V> Copy for TestWindowHandle<V> {}

impl<V> fmt::Debug for TestWindowHandle<V> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_tuple("TestWindowHandle")
            .field(&self.handle)
            .finish()
    }
}

impl<V> PartialEq for TestWindowHandle<V> {
    fn eq(&self, other: &Self) -> bool {
        self.handle == other.handle
    }
}

impl<V> Eq for TestWindowHandle<V> {}

/// Window-scoped facade for deterministic geometry and screenshot tests.
pub struct VisualTestContext<'a> {
    context: &'a mut TestAppContext,
    window: WindowHandle,
}

impl VisualTestContext<'_> {
    pub const fn window_handle(&self) -> WindowHandle {
        self.window
    }

    pub fn element_bounds(&mut self, element: impl Into<ElementId>) -> Result<Rect, TestAppError> {
        self.context.element_bounds(self.window, element)
    }

    pub fn assert_element_bounds(
        &mut self,
        element: impl Into<ElementId>,
        expected: Rect,
        tolerance: f32,
    ) -> Result<(), TestAppError> {
        self.context
            .assert_element_bounds(self.window, element, expected, tolerance)
    }

    pub fn capture_screenshot(&mut self) -> Result<crate::VisualSnapshot, TestAppError> {
        self.context.capture_screenshot(self.window)
    }

    /// Move the deterministic pointer through the production hit-test and paint-only hover path.
    pub fn move_pointer(&mut self, point: Point) -> Result<bool, TestAppError> {
        self.context.move_visual_pointer(self.window, point)
    }

    /// Resolve the platform cursor from the production painted hit stack at one logical point.
    pub fn cursor_style_at(&mut self, point: Point) -> Result<Option<CursorStyle>, TestAppError> {
        self.context.render_visual(self.window, false)?;
        Ok(self.context.window(self.window)?.ui.cursor_style_at(point))
    }

    /// Advance the injected clock and paint the resulting visual frame without rebuilding a
    /// clean view declaration.
    pub fn advance_time(&mut self, duration: Duration) -> Result<(), TestAppError> {
        self.context.advance_time(duration)?;
        let now = self.context.now();
        if let Some(state) = self.context.windows.get_mut(&self.window) {
            state.ui.advance_tooltips(now);
        }
        self.context.render_visual(self.window, false)?;
        Ok(())
    }
}

struct TestWindow {
    parent: Option<WindowHandle>,
    restore_focus_on_close: Option<ElementId>,
    view: Box<dyn AnyView>,
    ui: UiTree,
    #[cfg(feature = "inspector")]
    inspector: Option<InspectorState>,
    listeners: ListenerRegistry,
    key_dispatch_scratch: Vec<KeyListenerBinding>,
    action_dispatch_scratch: Vec<ActionListenerBinding>,
    touch_captures: HashMap<TouchId, TouchCapture>,
    config: WindowOptions,
    state: WindowState,
    system_appearance: WindowAppearance,
    dirty: bool,
    pending_focus: Option<ElementId>,
    render_count: usize,
    retained_geometry_ready: bool,
    requested_animation_frame: bool,
    repaint_deadline: Option<Instant>,
    pointer: Option<Point>,
}

enum TestDispatch {
    Event(WindowHandle, Event),
    Action(WindowHandle, AnyAction),
    Form(WindowHandle, ElementId, Option<ElementId>),
}

/// Headless application context for deterministic view and interaction tests.
///
/// It reuses QuickGUI's production view adapter, retained identity/focus/form tree, listener
/// registry, entity/global delivery, keymap, and foreground executor. No native window, WGPU
/// adapter, background thread, wall-clock polling loop, or accessibility service is created.
/// Geometry-dependent pointer and visual assertions belong in the future visual test adapter.
pub struct TestAppContext {
    windows: HashMap<WindowHandle, TestWindow>,
    active_window: Option<WindowHandle>,
    pending_windows: VecDeque<WindowRequest>,
    pending_closes: Vec<WindowHandle>,
    pending_dispatches: VecDeque<TestDispatch>,
    pending_entity_events: VecDeque<EntityEvent>,
    pending_global_notifications: VecDeque<TypeId>,
    pending_global_notification_types: HashSet<TypeId>,
    pending_all_globals: bool,
    foreground_tasks: ForegroundTaskSpawner,
    clipboard: ClipboardService,
    displays: Displays,
    keyboard_layout: KeyboardLayout,
    globals: GlobalStore,
    assets: Assets,
    font_system: SharedFontSystem,
    keymap: Keymap,
    menus: Vec<Menu>,
    application_callbacks: ApplicationCallbacks,
    quit_mode: QuitMode,
    focus_history: Vec<WindowHandle>,
    now: Rc<Cell<Instant>>,
    animation_epoch: Instant,
    exited: bool,
    form_submission_depth: u8,
    visual_renderer: Option<OffscreenRenderer>,
}

impl TestAppContext {
    /// Create one headless application window with default configuration.
    pub fn new<V: View>(view: V) -> Result<(Self, TestWindowHandle<V>), TestAppError> {
        Self::from_app(App::new(view))
    }

    /// Consume a configured application without starting Winit or WGPU.
    pub fn from_app<V: View>(app: App<V>) -> Result<(Self, TestWindowHandle<V>), TestAppError> {
        validate_window_options(&app.config)?;
        let font_system = create_shared_font_system(&app.assets, &app.fonts)?;
        let request = WindowRequest::new(app.view, app.config);
        let initial = TestWindowHandle::new(request.handle);
        let now = Rc::new(Cell::new(Instant::now()));
        let animation_epoch = now.get();
        let mut keymap = app.keymap;
        keymap.set_key_equivalents(crate::keyboard::key_equivalents_for_layout(
            &KeyboardLayout::default(),
        ));
        let mut context = Self {
            windows: HashMap::new(),
            active_window: None,
            pending_windows: VecDeque::from([request]),
            pending_closes: Vec::new(),
            pending_dispatches: VecDeque::with_capacity(8),
            pending_entity_events: VecDeque::with_capacity(8),
            pending_global_notifications: VecDeque::with_capacity(8),
            pending_global_notification_types: HashSet::with_capacity(8),
            pending_all_globals: false,
            foreground_tasks: ForegroundTaskSpawner::new_for_test(now.clone()),
            clipboard: ClipboardService::memory(),
            displays: Displays::test_default(),
            keyboard_layout: KeyboardLayout::default(),
            globals: app.globals,
            assets: app.assets,
            font_system,
            keymap,
            menus: app.menus,
            application_callbacks: app.application_callbacks,
            quit_mode: app.quit_mode,
            focus_history: Vec::with_capacity(4),
            now,
            animation_epoch,
            exited: false,
            form_submission_depth: 0,
            visual_renderer: None,
        };
        context.run_until_idle()?;
        Ok((context, initial))
    }

    pub fn windows(&self) -> Vec<WindowHandle> {
        let mut windows = self.windows.keys().copied().collect::<Vec<_>>();
        windows.sort_unstable();
        windows
    }

    pub fn active_window(&self) -> Option<WindowHandle> {
        self.active_window
    }

    pub fn is_exited(&self) -> bool {
        self.exited
    }

    pub fn is_window_open(&self, window: WindowHandle) -> bool {
        self.windows.contains_key(&window)
    }

    pub fn menus(&self) -> &[Menu] {
        &self.menus
    }

    /// Inspect the current deterministic display snapshot.
    pub fn displays(&self) -> &[Display] {
        self.displays.all()
    }

    pub fn primary_display(&self) -> Option<&Display> {
        self.displays.primary()
    }

    /// Inspect the deterministic active keyboard layout.
    pub fn keyboard_layout(&self) -> &KeyboardLayout {
        &self.keyboard_layout
    }

    /// Replace the deterministic active-display snapshot and rebuild only observing views.
    pub fn simulate_displays_change(&mut self, displays: Displays) -> Result<(), TestAppError> {
        if self.displays == displays {
            return Ok(());
        }
        self.displays = displays;
        for state in self.windows.values_mut() {
            let previous = state.state.display_id;
            state.state.display_id =
                crate::display::display_for_rect(&self.displays, state.state.bounds.bounds());
            if state.listeners.observes_displays
                || previous != state.state.display_id && state.listeners.observes_window_state
            {
                state.dirty = true;
            }
        }
        self.run_until_idle()
    }

    /// Replace the deterministic keyboard-layout snapshot and rebuild only observing views.
    pub fn simulate_keyboard_layout_change(
        &mut self,
        keyboard_layout: KeyboardLayout,
    ) -> Result<(), TestAppError> {
        if self.keyboard_layout == keyboard_layout {
            return Ok(());
        }
        self.keyboard_layout = keyboard_layout;
        self.keymap
            .set_key_equivalents(crate::keyboard::key_equivalents_for_layout(
                &self.keyboard_layout,
            ));
        for state in self.windows.values_mut() {
            if state.listeners.observes_keyboard_layout {
                state.dirty = true;
            }
        }
        if let Some(mut callback) = self.application_callbacks.keyboard_layout.take() {
            let layout = self.keyboard_layout.clone();
            let mut cx = self.event_context(None);
            callback(&layout, &mut cx);
            self.application_callbacks.keyboard_layout = Some(callback);
            self.apply_context(None, cx)?;
        }
        self.run_until_idle()
    }

    /// Inspect the deterministic in-memory general clipboard.
    pub fn read_from_clipboard(&self) -> Result<Option<ClipboardItem>, ClipboardError> {
        self.clipboard.read(ClipboardTarget::General)
    }

    /// Seed or replace the deterministic in-memory general clipboard.
    pub fn write_to_clipboard(&self, item: ClipboardItem) -> Result<(), ClipboardError> {
        self.clipboard.write(ClipboardTarget::General, item)
    }

    /// Inspect the deterministic in-memory macOS Find pasteboard.
    #[cfg(target_os = "macos")]
    pub fn read_from_find_pasteboard(&self) -> Result<Option<ClipboardItem>, ClipboardError> {
        self.clipboard.read(ClipboardTarget::Find)
    }

    /// Seed or replace the deterministic in-memory macOS Find pasteboard.
    #[cfg(target_os = "macos")]
    pub fn write_to_find_pasteboard(&self, item: ClipboardItem) -> Result<(), ClipboardError> {
        self.clipboard.write(ClipboardTarget::Find, item)
    }

    /// Validate and type one raw window handle returned by [`EventContext::open_window`].
    pub fn typed_window<V: View>(
        &self,
        window: WindowHandle,
    ) -> Result<TestWindowHandle<V>, TestAppError> {
        let state = self.window(window)?;
        if state.view.as_any().is::<V>() {
            Ok(TestWindowHandle::new(window))
        } else {
            Err(TestAppError::WrongViewType {
                window,
                expected: type_name::<V>(),
            })
        }
    }

    pub fn read<V: View, R>(
        &self,
        window: TestWindowHandle<V>,
        read: impl FnOnce(&V) -> R,
    ) -> Result<R, TestAppError> {
        let state = self.window(window.handle)?;
        let view = state
            .view
            .as_any()
            .downcast_ref::<V>()
            .ok_or(TestAppError::WrongViewType {
                window: window.handle,
                expected: type_name::<V>(),
            })?;
        Ok(read(view))
    }

    pub fn update<V: View, R>(
        &mut self,
        window: TestWindowHandle<V>,
        update: impl FnOnce(&mut V, &mut EventContext) -> R,
    ) -> Result<R, TestAppError> {
        let mut cx = self.event_context(Some(window.handle));
        let result =
            {
                let state = self.window_mut(window.handle)?;
                let view = state.view.as_any_mut().downcast_mut::<V>().ok_or(
                    TestAppError::WrongViewType {
                        window: window.handle,
                        expected: type_name::<V>(),
                    },
                )?;
                update(view, &mut cx)
            };
        self.apply_context(Some(window.handle), cx)?;
        self.run_until_idle()?;
        Ok(result)
    }

    pub fn read_global<G: Global, R>(&self, read: impl FnOnce(&G) -> R) -> R {
        let global = self.globals.get::<G>();
        read(&global)
    }

    pub fn update_global<G: Global, R>(
        &mut self,
        update: impl FnOnce(&mut G, &mut EventContext) -> R,
    ) -> Result<R, TestAppError> {
        let mut cx = self.event_context(None);
        let result = {
            let globals = self.globals.clone();
            let mut global = globals.get_mut::<G>();
            update(&mut global, &mut cx)
        };
        cx.update_global::<G, _>(|_| {});
        self.apply_context(None, cx)?;
        self.run_until_idle()?;
        Ok(result)
    }

    pub fn render_count(&self, window: WindowHandle) -> Result<usize, TestAppError> {
        Ok(self.window(window)?.render_count)
    }

    pub fn window_state(&self, window: WindowHandle) -> Result<WindowState, TestAppError> {
        Ok(self.window(window)?.state)
    }

    #[cfg(test)]
    pub(crate) fn accessibility_update(
        &mut self,
        window: WindowHandle,
    ) -> Result<accesskit::TreeUpdate, TestAppError> {
        self.run_until_idle()?;
        self.prepare_retained_geometry(window)?;
        let title = self.window(window)?.config.title.clone();
        Ok(self.window(window)?.ui.accessibility_update(&title))
    }

    #[cfg(test)]
    pub(crate) fn popover_grabs_focus(
        &self,
        window: WindowHandle,
    ) -> Result<Option<bool>, TestAppError> {
        Ok(self
            .window(window)?
            .config
            .popover
            .as_ref()
            .map(|popover| popover.grab))
    }

    /// Inject a bounded native tab-group snapshot and rebuild only an observing view.
    pub fn simulate_window_tab_state(
        &mut self,
        window: WindowHandle,
        tabs: WindowTabState,
    ) -> Result<(), TestAppError> {
        if !tabs.is_valid() {
            return Err(TestAppError::InvalidWindowTabState);
        }
        let state = self.window_mut(window)?;
        if state.state.native_tabs != tabs {
            state.state.native_tabs = tabs;
            if state.listeners.observes_window_state {
                state.dirty = true;
            }
        }
        self.run_until_idle()
    }

    /// Capture the current bounded retained-tree snapshot without creating native or GPU state.
    #[cfg(feature = "inspector")]
    pub fn inspector_snapshot(
        &mut self,
        window: WindowHandle,
    ) -> Result<Option<crate::InspectorSnapshot>, TestAppError> {
        if self.window(window)?.inspector.is_none() {
            return Ok(None);
        }
        self.prepare_retained_geometry(window)?;
        let state = self.window_mut(window)?;
        let viewport = state.state.viewport_size;
        let scale_factor = state.state.scale_factor;
        let TestWindow { ui, inspector, .. } = state;
        let inspector = inspector
            .as_mut()
            .expect("inspector presence checked before geometry preparation");
        inspector.refresh(
            ui,
            FrameMetrics::default(),
            InspectorFrameDamage::default(),
            viewport,
            scale_factor,
        );
        Ok(Some(inspector.snapshot().clone()))
    }

    /// Deliver a deterministic operating-system appearance change.
    ///
    /// A window with an explicit preference retains that effective appearance. The simulated
    /// system value is still remembered so returning the window to system-following mode uses the
    /// latest value without a polling source.
    pub fn simulate_appearance_change(
        &mut self,
        window: WindowHandle,
        appearance: WindowAppearance,
    ) -> Result<(), TestAppError> {
        let changed = {
            let state = self.window_mut(window)?;
            state.system_appearance = appearance;
            if state.config.preferred_appearance.is_some() || state.state.appearance == appearance {
                false
            } else {
                state.state.appearance = appearance;
                if state.listeners.observes_window_state {
                    state.dirty = true;
                }
                true
            }
        };
        if changed {
            self.queue_dispatch(TestDispatch::Event(
                window,
                Event::AppearanceChanged(appearance),
            ))?;
        }
        self.run_until_idle()
    }

    pub fn window_title(&self, window: WindowHandle) -> Result<&str, TestAppError> {
        Ok(&self.window(window)?.config.title)
    }

    /// Scope visual operations to one existing deterministic window.
    pub fn visual(&mut self, window: WindowHandle) -> Result<VisualTestContext<'_>, TestAppError> {
        self.window(window)?;
        Ok(VisualTestContext {
            context: self,
            window,
        })
    }

    pub fn contains_element(
        &self,
        window: WindowHandle,
        element: impl Into<ElementId>,
    ) -> Result<bool, TestAppError> {
        Ok(self.window(window)?.ui.contains_element(element.into()))
    }

    /// Lay out and paint the current declaration, then return one element's logical bounds.
    ///
    /// The first geometry or screenshot request lazily initializes one offscreen WGPU renderer.
    /// Ordinary semantic tests continue to create no GPU resources.
    pub fn element_bounds(
        &mut self,
        window: WindowHandle,
        element: impl Into<ElementId>,
    ) -> Result<Rect, TestAppError> {
        let element = element.into();
        self.render_visual(window, false)?;
        self.window(window)?
            .ui
            .element_bounds(element)
            .ok_or(TestAppError::UnknownElement { window, element })
    }

    /// Assert deterministic logical geometry with an explicit floating-point tolerance.
    pub fn assert_element_bounds(
        &mut self,
        window: WindowHandle,
        element: impl Into<ElementId>,
        expected: Rect,
        tolerance: f32,
    ) -> Result<(), TestAppError> {
        if !tolerance.is_finite() || tolerance < 0.0 {
            return Err(TestAppError::InvalidGeometryTolerance);
        }
        let element = element.into();
        let actual = self.element_bounds(window, element)?;
        if [
            (actual.x, expected.x),
            (actual.y, expected.y),
            (actual.width, expected.width),
            (actual.height, expected.height),
        ]
        .into_iter()
        .all(|(actual, expected)| (actual - expected).abs() <= tolerance)
        {
            Ok(())
        } else {
            Err(TestAppError::GeometryMismatch {
                window,
                element,
                actual,
                expected,
                tolerance,
            })
        }
    }

    /// Capture one bounded physical-pixel frame through the production WGPU pipelines.
    ///
    /// The headless target has no presentation loop and performs work only for this call. Native
    /// `NSView` children are rejected instead of silently producing an incomplete baseline.
    pub fn capture_screenshot(
        &mut self,
        window: WindowHandle,
    ) -> Result<crate::VisualSnapshot, TestAppError> {
        self.render_visual(window, true)?.ok_or_else(|| {
            TestAppError::Visual(crate::VisualTestError::Render(
                "the visual frame did not produce a snapshot".to_owned(),
            ))
        })
    }

    fn move_visual_pointer(
        &mut self,
        window: WindowHandle,
        point: Point,
    ) -> Result<bool, TestAppError> {
        self.render_visual(window, false)?;
        self.window_mut(window)?.pointer = Some(point);
        let mut renderer = self.visual_renderer.take().ok_or_else(|| {
            TestAppError::Visual(crate::VisualTestError::Render(
                "visual renderer was not retained after layout".to_owned(),
            ))
        })?;
        let changed = self
            .window_mut(window)?
            .ui
            .pointer_moved(point, &mut renderer);
        self.visual_renderer = Some(renderer);
        let hover_changed = self.invoke_pending_mouse_hover(window)?;
        if changed || hover_changed {
            self.render_visual(window, false)?;
        }
        Ok(changed || hover_changed)
    }

    fn invoke_pending_mouse_hover(&mut self, window: WindowHandle) -> Result<bool, TestAppError> {
        let mut changes = Vec::with_capacity(4);
        self.window_mut(window)?
            .ui
            .take_mouse_hover_changes(&mut changes);
        let changed = !changes.is_empty();
        for change in changes {
            let listener = self.window(window)?.listeners.mouse_listener(change.key);
            let Some(listener) = listener else {
                continue;
            };
            let mut cx = self.event_context(Some(window));
            listener(
                self.window_mut(window)?.view.as_any_mut(),
                &MouseListenerEvent::Hover(change.hovered),
                &mut cx,
            );
            self.apply_context(Some(window), cx)?;
        }
        self.run_until_idle()?;
        Ok(changed)
    }

    pub fn focused(&self, window: WindowHandle) -> Result<Option<ElementId>, TestAppError> {
        Ok(self.window(window)?.ui.focused())
    }

    pub fn focused_input_value(
        &self,
        window: WindowHandle,
    ) -> Result<Option<Arc<str>>, TestAppError> {
        Ok(self.window(window)?.ui.focused_text_input_value())
    }

    /// Current deterministic monotonic time used by foreground timers.
    pub fn now(&self) -> Instant {
        self.now.get()
    }

    pub fn focus(
        &mut self,
        window: WindowHandle,
        element: impl Into<ElementId>,
    ) -> Result<(), TestAppError> {
        let element = element.into();
        let previous = self.window(window)?.ui.focused();
        if !self.window(window)?.ui.is_focusable(element) {
            return Err(TestAppError::NotFocusable { window, element });
        }
        self.window_mut(window)?.ui.focus(element);
        self.focus_changed(window, previous)?;
        self.run_until_idle()
    }

    pub fn blur(&mut self, window: WindowHandle) -> Result<(), TestAppError> {
        let previous = self.window(window)?.ui.focused();
        self.window_mut(window)?.ui.blur();
        self.focus_changed(window, previous)?;
        self.run_until_idle()
    }

    pub fn click(
        &mut self,
        window: WindowHandle,
        element: impl Into<ElementId>,
    ) -> Result<(), TestAppError> {
        let element = element.into();
        self.require_element(window, element)?;
        if !self.window(window)?.ui.is_clickable(element) {
            return Err(TestAppError::NotClickable { window, element });
        }
        if self.window(window)?.ui.is_focusable(element) {
            let previous = self.window(window)?.ui.focused();
            self.window_mut(window)?.ui.focus(element);
            self.focus_changed(window, previous)?;
            self.run_until_idle()?;
            self.require_element(window, element)?;
            if !self.window(window)?.ui.is_clickable(element) {
                return Err(TestAppError::NotClickable { window, element });
            }
        }
        let activation_target = self.window(window)?.ui.activation_target(element);
        let form = self.window(window)?.ui.form_for_submitter(element);
        let listener = self.window(window)?.listeners.clicks.get(&element).cloned();
        let mut default_prevented = false;
        if let Some(listener) = listener {
            let mut cx = self.event_context(Some(window));
            listener(self.window_mut(window)?.view.as_any_mut(), &mut cx);
            default_prevented = cx.prevent_default;
            self.apply_context(Some(window), cx)?;
        }
        self.queue_dispatch(TestDispatch::Event(window, Event::Click(element)))?;
        if let Some(form) = form {
            self.queue_dispatch(TestDispatch::Form(window, form, Some(element)))?;
        }
        self.run_until_idle()?;
        let Some(target) = activation_target
            .filter(|target| *target != element)
            .filter(|_| !default_prevented)
        else {
            return Ok(());
        };
        let focusable = self.window(window)?.ui.is_focusable(target);
        let clickable = self.window(window)?.ui.is_clickable(target);
        if focusable {
            let previous = self.window(window)?.ui.focused();
            self.window_mut(window)?.ui.focus(target);
            self.focus_changed(window, previous)?;
            self.run_until_idle()?;
        }
        if clickable {
            self.click(window, target)?;
        }
        Ok(())
    }

    /// Deliver one deterministic web-style context-menu request to a declared secondary-click
    /// target without creating native or GPU resources for the owner window.
    pub fn simulate_context_menu(
        &mut self,
        window: WindowHandle,
        element: impl Into<ElementId>,
        position: Point,
        modifiers: Modifiers,
    ) -> Result<(), TestAppError> {
        let element = element.into();
        self.require_element(window, element)?;
        let listener = self
            .window(window)?
            .listeners
            .context_menus
            .get(&element)
            .cloned()
            .ok_or(TestAppError::NotListening {
                window,
                element,
                kind: "context menu",
            })?;
        let event = ContextMenuEvent {
            target: element,
            position,
            modifiers,
        };
        let mut cx = self.event_context(Some(window));
        listener(self.window_mut(window)?.view.as_any_mut(), &event, &mut cx);
        self.apply_context(Some(window), cx)?;
        self.queue_dispatch(TestDispatch::Event(window, Event::ContextMenu(event)))?;
        self.run_until_idle()
    }

    /// Deliver a targeted desktop mouse press through outside capture, capture, and bubble.
    ///
    /// `element` is the deterministic hit target. The returned value reports whether any callback
    /// prevented the framework default; propagation and default prevention remain independent.
    pub fn simulate_mouse_down(
        &mut self,
        window: WindowHandle,
        element: impl Into<ElementId>,
        event: MouseDownEvent,
    ) -> Result<bool, TestAppError> {
        self.simulate_mouse_event(
            window,
            element.into(),
            MouseListenerKind::Down,
            Some(event.button),
            MouseListenerEvent::Down(event),
        )
    }

    /// Deliver a targeted desktop mouse release through outside capture, capture, and bubble.
    pub fn simulate_mouse_up(
        &mut self,
        window: WindowHandle,
        element: impl Into<ElementId>,
        event: MouseUpEvent,
    ) -> Result<bool, TestAppError> {
        self.simulate_mouse_event(
            window,
            element.into(),
            MouseListenerKind::Up,
            Some(event.button),
            MouseListenerEvent::Up(event),
        )
    }

    /// Deliver targeted desktop mouse motion through capture and bubble.
    pub fn simulate_mouse_move(
        &mut self,
        window: WindowHandle,
        element: impl Into<ElementId>,
        event: MouseMoveEvent,
    ) -> Result<(), TestAppError> {
        self.window_mut(window)?.pointer = Some(event.position);
        self.simulate_mouse_event(
            window,
            element.into(),
            MouseListenerKind::Move,
            None,
            MouseListenerEvent::Move(event),
        )?;
        Ok(())
    }

    /// Deliver a native-window mouse-exit event along the preceding deterministic target path.
    pub fn simulate_mouse_exit(
        &mut self,
        window: WindowHandle,
        element: impl Into<ElementId>,
        event: MouseExitEvent,
    ) -> Result<(), TestAppError> {
        self.window_mut(window)?.pointer = Some(event.position);
        let result = self.simulate_mouse_event(
            window,
            element.into(),
            MouseListenerKind::Exit,
            None,
            MouseListenerEvent::Exit(event),
        );
        if let Ok(state) = self.window_mut(window) {
            state.pointer = None;
        }
        result.map(|_| ())
    }

    fn simulate_mouse_event(
        &mut self,
        window: WindowHandle,
        element: ElementId,
        kind: MouseListenerKind,
        button: Option<MouseButton>,
        event: MouseListenerEvent,
    ) -> Result<bool, TestAppError> {
        self.require_element(window, element)?;
        let mut path = Vec::with_capacity(8);
        if !self
            .window(window)?
            .ui
            .mouse_event_path_for_target(element, &mut path)
        {
            return Err(TestAppError::MouseDispatchPathLimit);
        }
        let mut dispatch = Vec::with_capacity(8);
        self.window(window)?
            .ui
            .collect_mouse_dispatch(&path, kind, button, &mut dispatch);
        let mut default_prevented = false;
        for key in dispatch {
            let listener = self.window(window)?.listeners.mouse_listener(key);
            let Some(listener) = listener else {
                continue;
            };
            let mut cx = self.event_context(Some(window));
            listener(self.window_mut(window)?.view.as_any_mut(), &event, &mut cx);
            let stop_propagation = cx.stop_event_propagation;
            default_prevented |= cx.prevent_default;
            self.apply_context(Some(window), cx)?;
            if stop_propagation {
                break;
            }
        }
        self.run_until_idle()?;
        Ok(default_prevented)
    }

    /// Deliver one deterministic scroll-wheel event through an element's listening ancestors.
    ///
    /// The returned value is true when a callback called [`EventContext::prevent_default`]. This
    /// semantic helper exercises listener ordering without native or GPU resources; production
    /// retained scrolling continues to use the geometry-driven runtime path.
    pub fn simulate_scroll_wheel(
        &mut self,
        window: WindowHandle,
        element: impl Into<ElementId>,
        event: ScrollWheelEvent,
    ) -> Result<bool, TestAppError> {
        let element = element.into();
        self.require_element(window, element)?;
        let mut current = self
            .window(window)?
            .ui
            .scroll_wheel_listener_for_target(element)
            .ok_or(TestAppError::NotListening {
                window,
                element,
                kind: "scroll wheel",
            })?;
        let event = event.bounded();
        let mut default_prevented = false;
        loop {
            let listener = self
                .window(window)?
                .listeners
                .scroll_wheels
                .get(&current)
                .cloned();
            if let Some(listener) = listener {
                let mut cx = self.event_context(Some(window));
                listener(self.window_mut(window)?.view.as_any_mut(), &event, &mut cx);
                let stop_propagation = cx.stop_event_propagation;
                default_prevented |= cx.prevent_default;
                self.apply_context(Some(window), cx)?;
                if stop_propagation {
                    break;
                }
            }
            let Some(parent) = self
                .window(window)?
                .ui
                .parent_scroll_wheel_listener(current)
            else {
                break;
            };
            current = parent;
        }
        self.run_until_idle()?;
        Ok(default_prevented)
    }

    /// Apply one production retained-scroll delta at the center of an element.
    ///
    /// Unlike [`Self::simulate_scroll_wheel`], this exercises the geometry-driven default scroll
    /// path. Ordinary overflow scrolling updates retained paint state without rebuilding a clean
    /// view declaration; virtual scrolling rebuilds when its mounted slice must follow the shared
    /// offset. The returned value reports whether retained state changed.
    pub fn simulate_retained_scroll(
        &mut self,
        window: WindowHandle,
        element: impl Into<ElementId>,
        delta: Vector,
    ) -> Result<bool, TestAppError> {
        let element = element.into();
        self.require_element(window, element)?;
        self.prepare_retained_geometry(window)?;
        let bounds = self
            .window(window)?
            .ui
            .element_bounds(element)
            .ok_or(TestAppError::UnknownElement { window, element })?;
        let point = Point::new(
            bounds.x + bounds.width * 0.5,
            bounds.y + bounds.height * 0.5,
        );
        let now = self.now();
        let result = self
            .window_mut(window)?
            .ui
            .scroll_at(Some(point), delta, now);
        if result.view_dirty {
            let state = self.window_mut(window)?;
            state.dirty = true;
            state.retained_geometry_ready = false;
        }
        self.run_until_idle()?;
        Ok(result.changed)
    }

    /// Read one retained scroll container's current logical offset.
    pub fn retained_scroll_offset(
        &mut self,
        window: WindowHandle,
        element: impl Into<ElementId>,
    ) -> Result<Vector, TestAppError> {
        let element = element.into();
        self.require_element(window, element)?;
        self.prepare_retained_geometry(window)?;
        self.window(window)?
            .ui
            .scroll_offset(element)
            .ok_or(TestAppError::NotScrollable { window, element })
    }

    /// Deliver one deterministic raw touch sample through production capture semantics.
    ///
    /// `element` chooses the hit target only for [`TouchPhase::Started`]. Later samples with the
    /// same [`TouchId`] use the retained capture even if a different element is supplied.
    pub fn simulate_touch(
        &mut self,
        window: WindowHandle,
        element: impl Into<ElementId>,
        event: TouchEvent,
    ) -> Result<(), TestAppError> {
        let element = element.into();
        self.require_element(window, element)?;
        let event = event.bounded();
        let target = match event.phase {
            TouchPhase::Started => {
                let state = self.window_mut(window)?;
                state.touch_captures.remove(&event.id);
                let target = state.ui.touch_listener_for_target(element);
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
            TouchPhase::Moved => self
                .window_mut(window)?
                .touch_captures
                .get_mut(&event.id)
                .map(|capture| {
                    capture.last_event = event;
                    capture.target
                }),
            TouchPhase::Ended | TouchPhase::Cancelled => self
                .window_mut(window)?
                .touch_captures
                .remove(&event.id)
                .map(|capture| capture.target),
        };

        let mut current = target;
        while let Some(id) = current {
            let listener = self.window(window)?.listeners.touches.get(&id).cloned();
            if let Some(listener) = listener {
                let mut cx = self.event_context(Some(window));
                listener(self.window_mut(window)?.view.as_any_mut(), &event, &mut cx);
                let stop_propagation = cx.stop_event_propagation;
                self.apply_context(Some(window), cx)?;
                if stop_propagation {
                    break;
                }
            }
            current = self.window(window)?.ui.parent_touch_listener(id);
        }
        self.queue_dispatch(TestDispatch::Event(window, Event::Touch(event)))?;
        self.run_until_idle()
    }

    /// Deliver one deterministic Force Touch event to an attached element listener.
    pub fn simulate_mouse_pressure(
        &mut self,
        window: WindowHandle,
        element: impl Into<ElementId>,
        event: MousePressureEvent,
    ) -> Result<(), TestAppError> {
        let element = element.into();
        self.require_element(window, element)?;
        let listener = self
            .window(window)?
            .listeners
            .mouse_pressures
            .get(&element)
            .cloned()
            .ok_or(TestAppError::NotListening {
                window,
                element,
                kind: "mouse pressure",
            })?;
        let mut cx = self.event_context(Some(window));
        listener(self.window_mut(window)?.view.as_any_mut(), &event, &mut cx);
        self.apply_context(Some(window), cx)?;
        self.queue_dispatch(TestDispatch::Event(window, Event::MousePressure(event)))?;
        self.run_until_idle()
    }

    /// Deliver one deterministic pinch event to an attached element listener.
    pub fn simulate_pinch(
        &mut self,
        window: WindowHandle,
        element: impl Into<ElementId>,
        event: PinchEvent,
    ) -> Result<(), TestAppError> {
        let element = element.into();
        self.require_element(window, element)?;
        let listener = self
            .window(window)?
            .listeners
            .pinches
            .get(&element)
            .cloned()
            .ok_or(TestAppError::NotListening {
                window,
                element,
                kind: "pinch",
            })?;
        let mut cx = self.event_context(Some(window));
        listener(self.window_mut(window)?.view.as_any_mut(), &event, &mut cx);
        self.apply_context(Some(window), cx)?;
        self.queue_dispatch(TestDispatch::Event(window, Event::Pinch(event)))?;
        self.run_until_idle()
    }

    /// Deliver one deterministic rotation event to an attached element listener.
    pub fn simulate_rotation(
        &mut self,
        window: WindowHandle,
        element: impl Into<ElementId>,
        event: RotationEvent,
    ) -> Result<(), TestAppError> {
        let element = element.into();
        self.require_element(window, element)?;
        let listener = self
            .window(window)?
            .listeners
            .rotations
            .get(&element)
            .cloned()
            .ok_or(TestAppError::NotListening {
                window,
                element,
                kind: "rotation",
            })?;
        let mut cx = self.event_context(Some(window));
        listener(self.window_mut(window)?.view.as_any_mut(), &event, &mut cx);
        self.apply_context(Some(window), cx)?;
        self.queue_dispatch(TestDispatch::Event(window, Event::Rotation(event)))?;
        self.run_until_idle()
    }

    /// Deliver one deterministic smart-magnify event to an attached element listener.
    pub fn simulate_smart_magnify(
        &mut self,
        window: WindowHandle,
        element: impl Into<ElementId>,
        event: SmartMagnifyEvent,
    ) -> Result<(), TestAppError> {
        let element = element.into();
        self.require_element(window, element)?;
        let listener = self
            .window(window)?
            .listeners
            .smart_magnifies
            .get(&element)
            .cloned()
            .ok_or(TestAppError::NotListening {
                window,
                element,
                kind: "smart magnify",
            })?;
        let mut cx = self.event_context(Some(window));
        listener(self.window_mut(window)?.view.as_any_mut(), &event, &mut cx);
        self.apply_context(Some(window), cx)?;
        self.queue_dispatch(TestDispatch::Event(window, Event::SmartMagnify(event)))?;
        self.run_until_idle()
    }

    pub fn dispatch_action<A: Action>(
        &mut self,
        window: WindowHandle,
        action: A,
    ) -> Result<bool, TestAppError> {
        self.window(window)?;
        let consumed = self.invoke_action(window, &AnyAction::new(action))?;
        self.run_until_idle()?;
        Ok(consumed)
    }

    pub fn simulate_input(&mut self, window: WindowHandle, text: &str) -> Result<(), TestAppError> {
        if self.window(window)?.ui.focused_text_input().is_none() {
            return Err(TestAppError::NoFocusedTextInput(window));
        }
        let result = self.window_mut(window)?.ui.input_replace(text);
        let changed = result.change.is_some();
        self.apply_input_result(window, result)?;
        if changed {
            self.queue_dispatch(TestDispatch::Event(
                window,
                Event::TextInput(text.to_owned()),
            ))?;
        }
        self.run_until_idle()
    }

    /// Simulate one or more whitespace-separated normalized keystrokes.
    ///
    /// Multi-stroke bindings are resolved without a wall-clock timeout: an exact shorter binding
    /// is selected when this supplied sequence ends, while a prefix with no exact binding fails.
    pub fn simulate_keystrokes(
        &mut self,
        window: WindowHandle,
        sequence: &str,
    ) -> Result<(), TestAppError> {
        let strokes = sequence
            .split_whitespace()
            .map(Keystroke::parse)
            .collect::<Result<Vec<_>, _>>()
            .map_err(|error| TestAppError::View(error.to_string()))?;
        self.simulate_keystroke_values(window, strokes)
    }

    /// Simulate one already-normalized platform keystroke, including optional `key_char`
    /// metadata.
    pub fn simulate_keystroke(
        &mut self,
        window: WindowHandle,
        stroke: Keystroke,
    ) -> Result<(), TestAppError> {
        self.simulate_keystroke_values(window, vec![stroke])
    }

    /// Simulate one normalized key release through focused capture and bubble listeners.
    pub fn simulate_key_up(
        &mut self,
        window: WindowHandle,
        stroke: Keystroke,
    ) -> Result<(), TestAppError> {
        self.window(window)?;
        self.invoke_key_event(
            window,
            KeyListenerEvent::Up(KeyUpEvent {
                key: stroke.key.clone(),
                key_char: stroke.key_char.clone(),
                modifiers: stroke.modifiers,
            }),
        )?;
        self.queue_dispatch(TestDispatch::Event(
            window,
            Event::KeyUp {
                key: stroke.key,
                key_char: stroke.key_char,
                modifiers: stroke.modifiers,
            },
        ))?;
        self.run_until_idle()
    }

    fn simulate_keystroke_values(
        &mut self,
        window: WindowHandle,
        strokes: Vec<Keystroke>,
    ) -> Result<(), TestAppError> {
        self.window(window)?;
        let mut prefix = Vec::new();
        for stroke in strokes {
            prefix.push(stroke);
            loop {
                let contexts = self.window(window)?.ui.key_context_stack();
                let matched = self.keymap.bindings_for_input(&prefix, &contexts);
                if matched.pending {
                    break;
                }
                if !matched.bindings.is_empty() {
                    let fallback = prefix.last().cloned();
                    let consumed = self.dispatch_bindings(window, &matched.bindings)?;
                    if !consumed && let Some(fallback) = fallback {
                        self.handle_keystroke_fallback(window, fallback)?;
                    }
                    self.run_until_idle()?;
                    prefix.clear();
                    break;
                }
                let replay = prefix.remove(0);
                self.handle_keystroke_fallback(window, replay)?;
                self.run_until_idle()?;
                if prefix.is_empty() {
                    break;
                }
            }
        }
        if !prefix.is_empty() {
            let contexts = self.window(window)?.ui.key_context_stack();
            let matched = self.keymap.bindings_for_input(&prefix, &contexts);
            if matched.bindings.is_empty() {
                return Err(TestAppError::IncompleteKeystrokeSequence);
            }
            let fallback = prefix.last().cloned();
            let consumed = self.dispatch_bindings(window, &matched.bindings)?;
            if !consumed && let Some(fallback) = fallback {
                self.handle_keystroke_fallback(window, fallback)?;
            }
            self.run_until_idle()?;
        }
        self.run_until_idle()
    }

    /// Run queued callbacks, foreground futures, observation delivery, and dirty declarations.
    pub fn run_until_idle(&mut self) -> Result<(), TestAppError> {
        for _ in 0..MAX_TEST_EFFECT_TURNS {
            let mut progress = false;
            progress |= self.create_pending_windows()?;
            progress |= self.process_one_dispatch()?;
            if self.pending_dispatches.is_empty() {
                progress |= self.process_deferred_effects()?;
            }
            progress |= self.process_foreground_tasks()?;
            // Production delivers queued cross-window actions before applying the close tree from
            // the same callback. Popover commands therefore reach their owner before the popover
            // chain is destroyed; the deterministic runtime preserves that exact ordering.
            progress |= self.close_pending_windows()?;
            progress |= self.rebuild_dirty_windows()?;
            if !progress {
                return Ok(());
            }
        }
        Err(TestAppError::EffectTurnLimit)
    }

    /// Advance exact foreground timers and view repaint deadlines without sleeping.
    pub fn advance_time(&mut self, duration: Duration) -> Result<(), TestAppError> {
        let current = self.now.get();
        let now = current.checked_add(duration).unwrap_or(current);
        self.now.set(now);
        self.foreground_tasks.wake_due_timers(now);
        for state in self.windows.values_mut() {
            if state
                .repaint_deadline
                .is_some_and(|deadline| deadline <= now)
            {
                state.repaint_deadline = None;
                state.dirty = true;
            }
            if state.ui.declarative_animation_due(now) {
                state.dirty = true;
            }
        }
        self.run_until_idle()
    }

    /// Advance one explicitly requested animation frame. Continuous frames never run implicitly.
    pub fn advance_frame(&mut self) -> Result<usize, TestAppError> {
        let mut scheduled = 0;
        for state in self.windows.values_mut() {
            if state.requested_animation_frame {
                state.dirty = true;
                scheduled += 1;
            }
        }
        self.run_until_idle()?;
        Ok(scheduled)
    }

    pub fn simulate_open_urls(
        &mut self,
        urls: impl IntoIterator<Item = impl Into<Arc<str>>>,
    ) -> Result<(), TestAppError> {
        let Some(mut callback) = self.application_callbacks.open_urls.take() else {
            return Ok(());
        };
        let mut retained = Vec::with_capacity(4);
        let mut total = 0_usize;
        for url in urls.into_iter().take(crate::MAX_OPEN_URLS) {
            let url = url.into();
            if url.len() > crate::MAX_PLATFORM_URL_BYTES || url.contains('\0') {
                continue;
            }
            let Some(next_total) = total.checked_add(url.len()) else {
                break;
            };
            if next_total > crate::MAX_OPEN_URLS_TOTAL_BYTES {
                break;
            }
            total = next_total;
            retained.push(url);
        }
        let urls = OpenUrls::from_bounded(retained);
        let mut cx = self.event_context(None);
        callback(urls, &mut cx);
        self.application_callbacks.open_urls = Some(callback);
        self.apply_context(None, cx)?;
        self.run_until_idle()
    }

    pub fn simulate_reopen(&mut self, has_visible_windows: bool) -> Result<(), TestAppError> {
        let Some(mut callback) = self.application_callbacks.reopen.take() else {
            return Ok(());
        };
        let mut cx = self.event_context(None);
        callback(has_visible_windows, &mut cx);
        self.application_callbacks.reopen = Some(callback);
        self.apply_context(None, cx)?;
        self.run_until_idle()
    }

    pub fn simulate_system_wake(&mut self) -> Result<(), TestAppError> {
        let Some(mut callback) = self.application_callbacks.system_wake.take() else {
            return Ok(());
        };
        let mut cx = self.event_context(None);
        callback(&mut cx);
        self.application_callbacks.system_wake = Some(callback);
        self.apply_context(None, cx)?;
        self.run_until_idle()
    }

    pub fn simulate_system_notification_response(
        &mut self,
        response: SystemNotificationResponse,
    ) -> Result<(), TestAppError> {
        let Some(mut callback) = self
            .application_callbacks
            .system_notification_response
            .take()
        else {
            return Ok(());
        };
        let mut cx = self.event_context(None);
        callback(response, &mut cx);
        self.application_callbacks.system_notification_response = Some(callback);
        self.apply_context(None, cx)?;
        self.run_until_idle()
    }

    /// Deliver a native close request and return whether the window was closed.
    pub fn simulate_close_requested(&mut self, window: WindowHandle) -> Result<bool, TestAppError> {
        self.window(window)?;
        let mut cx = self.event_context(Some(window));
        self.window_mut(window)?
            .view
            .event(&Event::CloseRequested, &mut cx);
        let prevent_close = cx.prevent_close;
        let explicitly_closed = cx.close_current_window;
        self.apply_context(Some(window), cx)?;
        if !prevent_close && !explicitly_closed {
            self.pending_closes.push(window);
        }
        self.run_until_idle()?;
        Ok(!self.is_window_open(window))
    }

    fn window(&self, window: WindowHandle) -> Result<&TestWindow, TestAppError> {
        self.windows
            .get(&window)
            .ok_or(TestAppError::UnknownWindow(window))
    }

    fn window_mut(&mut self, window: WindowHandle) -> Result<&mut TestWindow, TestAppError> {
        self.windows
            .get_mut(&window)
            .ok_or(TestAppError::UnknownWindow(window))
    }

    fn require_element(
        &self,
        window: WindowHandle,
        element: ElementId,
    ) -> Result<(), TestAppError> {
        if self.window(window)?.ui.contains_element(element) {
            Ok(())
        } else {
            Err(TestAppError::UnknownElement { window, element })
        }
    }

    fn event_context(&self, window: Option<WindowHandle>) -> EventContext {
        let parent = window
            .and_then(|window| self.windows.get(&window))
            .and_then(|window| window.parent);
        let popover_context = window.and_then(|window| self.popover_context(window));
        let pointer_position = window
            .and_then(|window| self.windows.get(&window))
            .and_then(|window| window.pointer);
        EventContext::with_runtime(
            self.globals.clone(),
            self.foreground_tasks.clone(),
            self.clipboard.clone(),
            self.displays.clone(),
            self.keyboard_layout.clone(),
            self.assets.clone(),
            crate::event::EventWindowContext {
                window,
                parent,
                popover_owner: popover_context.map(|context| context.owner),
                popover_root: popover_context.map(|context| context.root),
                pointer_position,
            },
        )
    }

    fn popover_context(&self, window: WindowHandle) -> Option<PopoverWindowContext> {
        let current = self.windows.get(&window)?;
        if current.config.kind != WindowKind::SystemPopover {
            return None;
        }
        let mut root = window;
        let mut ancestor = current.parent?;
        loop {
            let window = self.windows.get(&ancestor)?;
            if window.config.kind != WindowKind::SystemPopover {
                return Some(PopoverWindowContext {
                    owner: ancestor,
                    root,
                });
            }
            root = ancestor;
            ancestor = window.parent?;
        }
    }

    fn apply_context(
        &mut self,
        origin: Option<WindowHandle>,
        mut cx: EventContext,
    ) -> Result<(), TestAppError> {
        if !cx.platform_requests.is_empty() {
            return Err(TestAppError::UnsupportedPlatformRequest);
        }
        if cx.exit {
            self.exited = true;
            self.pending_closes.extend(self.windows.keys().copied());
        }
        if !enqueue_global_notifications(
            &mut self.pending_global_notifications,
            &mut self.pending_global_notification_types,
            &mut self.pending_all_globals,
            &cx.global_notifications,
            cx.notify_all_globals,
        ) {
            return Err(TestAppError::EffectTurnLimit);
        }
        if !enqueue_entity_events(&mut self.pending_entity_events, &mut cx.entity_events) {
            return Err(TestAppError::EffectTurnLimit);
        }
        if self.exited {
            cx.open_windows.clear();
        } else {
            for request in &mut cx.open_windows {
                let Some(anchor) = request.popover_anchor_element else {
                    continue;
                };
                let origin = origin.ok_or_else(|| {
                    TestAppError::View("a SystemPopover requires a parent test window".to_owned())
                })?;
                // Production pointer events arrive after a presented frame has populated retained
                // geometry. Headless semantic tests intentionally skip paint, so prepare the same
                // CPU-only geometry lazily only when an element-anchored child actually needs it.
                self.prepare_retained_geometry(origin)?;
                let bounds = self.window(origin)?.ui.element_bounds(anchor).ok_or(
                    TestAppError::UnknownElement {
                        window: origin,
                        element: anchor,
                    },
                )?;
                let popover = request.options.popover.as_mut().ok_or_else(|| {
                    TestAppError::View(WindowCommandError::InvalidPopoverConfiguration.to_string())
                })?;
                popover.anchor_rect = bounds;
            }
            self.pending_windows.extend(cx.open_windows.drain(..));
        }
        if cx.close_current_window
            && let Some(origin) = origin
        {
            self.pending_closes.push(origin);
        }
        self.pending_closes.append(&mut cx.close_windows);
        for window in cx.focus_windows.drain(..) {
            if self.windows.contains_key(&window) {
                self.set_active_window(Some(window))?;
            }
        }
        for window in cx.invalidate_windows.drain(..) {
            if let Some(state) = self.windows.get_mut(&window) {
                state.dirty = true;
            }
        }
        for command in cx.window_commands.drain(..) {
            self.apply_window_command(command)?;
        }
        if let Some(menus) = cx.menus.take() {
            self.menus = menus;
        }

        let mut immediate = Vec::new();
        if let Some(origin) = origin {
            let previous = self.window(origin)?.ui.focused();
            if let Some(request) = cx.focus {
                match request {
                    Some(element) if self.window(origin)?.ui.is_focusable(element) => {
                        let state = self.window_mut(origin)?;
                        state.pending_focus = None;
                        state.ui.focus(element);
                    }
                    Some(element) => {
                        let state = self.window_mut(origin)?;
                        state.pending_focus = Some(element);
                        state.dirty = true;
                    }
                    None => {
                        let state = self.window_mut(origin)?;
                        state.pending_focus = None;
                        state.ui.blur();
                    }
                }
            }
            if cx.invalidate {
                self.window_mut(origin)?.dirty = true;
            }
            let focused = self.window(origin)?.ui.focused();
            if focused != previous {
                self.window_mut(origin)?.dirty = true;
                immediate.push(TestDispatch::Event(origin, Event::FocusChanged(focused)));
            }
            immediate.extend(
                cx.actions
                    .drain(..)
                    .map(|action| TestDispatch::Action(origin, action)),
            );
            immediate.extend(
                cx.form_submissions
                    .drain(..)
                    .map(|form| TestDispatch::Form(origin, form, None)),
            );
        } else if let Some(active) = self.active_window {
            immediate.extend(
                cx.actions
                    .drain(..)
                    .map(|action| TestDispatch::Action(active, action)),
            );
        }
        immediate.extend(
            cx.targeted_actions
                .drain(..)
                .filter(|(window, _)| self.windows.contains_key(window))
                .map(|(window, action)| TestDispatch::Action(window, action)),
        );
        self.queue_immediate(immediate)?;

        let entities = cx.entity_notifications;
        for state in self.windows.values_mut() {
            if state
                .listeners
                .observes_entity_change(&entities, cx.notify_all_entities)
                || state
                    .listeners
                    .observes_global_change(&cx.global_notifications, cx.notify_all_globals)
            {
                state.dirty = true;
            }
        }
        Ok(())
    }

    fn render_visual(
        &mut self,
        window: WindowHandle,
        capture: bool,
    ) -> Result<Option<crate::VisualSnapshot>, TestAppError> {
        self.run_until_idle()?;
        let (profile, viewport, scale_factor) = {
            let state = self.window(window)?;
            (
                state.config.performance_profile,
                state.state.viewport_size,
                state.state.scale_factor,
            )
        };
        if capture {
            OffscreenRenderer::validate_snapshot_size(viewport, scale_factor)?;
        }
        let mut renderer = match self.visual_renderer.take() {
            Some(renderer) => renderer,
            None => pollster::block_on(OffscreenRenderer::new(profile, self.font_system.clone()))?,
        };
        let now = self.now.get();
        let result = (|| {
            // A variable list can learn new row heights only after production layout. Rebuild the
            // declaration until its sparse metrics agree, with a hard cap for a pathological view
            // whose item height changes on every render. Ordinary visual tests still take one pass.
            for _ in 0..8 {
                let mut scene = Scene::new();
                let measurements_changed = {
                    let state = self.window_mut(window)?;
                    state
                        .ui
                        .invalidate_layout_measurements_for_test()
                        .map_err(|error| TestAppError::View(error.to_string()))?;
                    state
                        .ui
                        .layout_for_test(viewport, scale_factor, &mut renderer, now)
                        .map_err(|error| TestAppError::View(error.to_string()))?;
                    state.requested_animation_frame |=
                        state.ui.declarative_animation_frame_requested();
                    state.ui.advance_animations(now);
                    state.ui.advance_scrollbars(now);
                    scene.clear(state.config.background);
                    state
                        .ui
                        .paint_at(&mut scene, &mut renderer, now)
                        .map_err(|error| TestAppError::View(error.to_string()))?;
                    #[cfg(feature = "inspector")]
                    if state.inspector.is_some() {
                        let TestWindow { ui, inspector, .. } = state;
                        let inspector = inspector
                            .as_mut()
                            .expect("inspector presence checked before split borrow");
                        inspector.refresh(
                            ui,
                            FrameMetrics::default(),
                            InspectorFrameDamage::default(),
                            viewport,
                            scale_factor,
                        );
                        inspector
                            .paint(&mut scene, &mut renderer, viewport, scale_factor, now)
                            .map_err(|error| TestAppError::View(error.to_string()))?;
                    }
                    let update = state.ui.take_variable_list_measurement_update();
                    state.dirty |= update.view_dirty;
                    update.changed
                };
                if measurements_changed {
                    self.run_until_idle()?;
                    continue;
                }
                if !capture {
                    return Ok(None);
                }
                #[cfg(target_os = "macos")]
                if !self.window(window)?.ui.native_views().is_empty() {
                    return Err(TestAppError::Visual(
                        crate::VisualTestError::NativeViewUnsupported,
                    ));
                }
                return renderer
                    .render_to_snapshot(&scene, viewport, scale_factor)
                    .map(Some)
                    .map_err(TestAppError::from);
            }
            Err(TestAppError::View(
                "variable-list measurements did not converge within 8 visual layout passes"
                    .to_owned(),
            ))
        })();
        self.visual_renderer = Some(renderer);
        result
    }

    fn prepare_retained_geometry(&mut self, window: WindowHandle) -> Result<(), TestAppError> {
        if self.window(window)?.retained_geometry_ready {
            return Ok(());
        }
        let now = self.now();
        let (viewport, scale_factor, background) = {
            let state = self.window(window)?;
            (
                state.state.viewport_size,
                state.state.scale_factor,
                state.config.background,
            )
        };
        let mut layout = SemanticTextLayout;
        let mut scene = Scene::new();
        scene.clear(background);
        let state = self.window_mut(window)?;
        state
            .ui
            .layout_for_test(viewport, scale_factor, &mut layout, now)
            .map_err(|error| TestAppError::View(error.to_string()))?;
        state
            .ui
            .paint_at(&mut scene, &mut layout, now)
            .map_err(|error| TestAppError::View(error.to_string()))?;
        state.retained_geometry_ready = true;
        Ok(())
    }

    fn queue_dispatch(&mut self, dispatch: TestDispatch) -> Result<(), TestAppError> {
        if self.pending_dispatches.len() == MAX_TEST_PENDING_DISPATCHES {
            return Err(TestAppError::DispatchQueueFull);
        }
        self.pending_dispatches.push_back(dispatch);
        Ok(())
    }

    fn queue_immediate(
        &mut self,
        dispatches: impl IntoIterator<Item = TestDispatch>,
    ) -> Result<(), TestAppError> {
        let dispatches = dispatches.into_iter().collect::<Vec<_>>();
        if self.pending_dispatches.len() + dispatches.len() > MAX_TEST_PENDING_DISPATCHES {
            return Err(TestAppError::DispatchQueueFull);
        }
        for dispatch in dispatches.into_iter().rev() {
            self.pending_dispatches.push_front(dispatch);
        }
        Ok(())
    }

    fn create_pending_windows(&mut self) -> Result<bool, TestAppError> {
        let mut progress = false;
        while let Some(request) = self.pending_windows.pop_front() {
            validate_window_options(&request.options)?;
            let handle = request.handle;
            let parent_state = request
                .parent
                .and_then(|parent| self.windows.get(&parent))
                .map(|window| window.state);
            let state = test_window_state(
                handle,
                &request.options,
                request.options.focus,
                &self.displays,
                parent_state,
            );
            let window = TestWindow {
                parent: request.parent,
                restore_focus_on_close: request.popover_anchor_element,
                view: request.view,
                ui: UiTree::new_at(self.animation_epoch),
                #[cfg(feature = "inspector")]
                inspector: request
                    .options
                    .inspector
                    .then(|| InspectorState::new(self.animation_epoch)),
                listeners: ListenerRegistry::default(),
                key_dispatch_scratch: Vec::with_capacity(16),
                action_dispatch_scratch: Vec::with_capacity(16),
                touch_captures: HashMap::with_capacity(8),
                config: request.options,
                state,
                system_appearance: WindowAppearance::default(),
                dirty: true,
                pending_focus: None,
                render_count: 0,
                retained_geometry_ready: false,
                requested_animation_frame: false,
                repaint_deadline: None,
                pointer: None,
            };
            self.windows.insert(handle, window);
            if self.active_window.is_none() || self.window(handle)?.config.focus {
                self.set_active_window(Some(handle))?;
            }
            progress = true;
        }
        Ok(progress)
    }

    fn close_pending_windows(&mut self) -> Result<bool, TestAppError> {
        if self.pending_closes.is_empty() {
            return Ok(false);
        }
        let roots = std::mem::take(&mut self.pending_closes);
        let mut discovered = HashSet::new();
        let mut order = Vec::new();
        let mut stack = roots
            .into_iter()
            .map(|window| (window, false))
            .collect::<Vec<_>>();
        while let Some((window, expanded)) = stack.pop() {
            if expanded {
                order.push(window);
                continue;
            }
            if !discovered.insert(window) {
                continue;
            }
            stack.push((window, true));
            let mut children = self
                .windows
                .iter()
                .filter_map(|(handle, state)| (state.parent == Some(window)).then_some(*handle))
                .collect::<Vec<_>>();
            children.sort_unstable();
            for child in children.into_iter().rev() {
                stack.push((child, false));
            }
        }

        let mut closed = Vec::with_capacity(order.len());
        for window in order {
            let (parent, restore_focus) = self
                .windows
                .get(&window)
                .map(|window| (window.parent, window.restore_focus_on_close))
                .unwrap_or((None, None));
            self.foreground_tasks.cancel_window(window);
            if self.windows.remove(&window).is_some() {
                closed.push(ClosedWindow {
                    handle: window,
                    parent,
                    restore_focus,
                });
            }
        }
        self.focus_history
            .retain(|window| !discovered.contains(window));
        if self
            .active_window
            .is_some_and(|window| discovered.contains(&window))
        {
            self.active_window = self.focus_history.last().copied();
            let _ = self.refresh_window_focus_states();
        }

        for closed in &closed {
            if let Some(parent) = closed.parent
                && self.windows.contains_key(&parent)
            {
                let callbacks = {
                    let listeners = &self.window(parent)?.listeners;
                    let mut callbacks =
                        Vec::with_capacity(listeners.any_child_window_closed.len() + 1);
                    if let Some(callback) =
                        listeners.child_window_closed.get(&closed.handle).cloned()
                    {
                        callbacks.push(callback);
                    }
                    callbacks.extend(listeners.any_child_window_closed.iter().cloned());
                    callbacks
                };
                if !callbacks.is_empty() || closed.restore_focus.is_some() {
                    let mut cx = self.event_context(Some(parent));
                    cx.focus = closed.restore_focus.map(Some);
                    for callback in callbacks {
                        callback(
                            self.window_mut(parent)?.view.as_any_mut(),
                            closed.handle,
                            &mut cx,
                        );
                    }
                    self.apply_context(Some(parent), cx)?;
                }
            }

            if let Some(mut callback) = self.application_callbacks.window_closed.take() {
                let mut cx = self.event_context(None);
                callback(closed.handle, &mut cx);
                self.application_callbacks.window_closed = Some(callback);
                self.apply_context(None, cx)?;
            }
        }

        if !self.exited
            && self.windows.is_empty()
            && self.pending_windows.is_empty()
            && self.quit_mode.quits_when_empty()
        {
            self.exited = true;
        }
        Ok(!closed.is_empty())
    }

    fn process_one_dispatch(&mut self) -> Result<bool, TestAppError> {
        let Some(dispatch) = self.pending_dispatches.pop_front() else {
            return Ok(false);
        };
        match dispatch {
            TestDispatch::Event(window, event) => {
                if !self.windows.contains_key(&window) {
                    return Ok(true);
                }
                let mut cx = self.event_context(Some(window));
                self.window_mut(window)?.view.event(&event, &mut cx);
                self.apply_context(Some(window), cx)?;
            }
            TestDispatch::Action(window, action) => {
                if self.windows.contains_key(&window) {
                    self.invoke_action(window, &action)?;
                }
            }
            TestDispatch::Form(window, form, trigger) => {
                if self.windows.contains_key(&window) {
                    self.invoke_form_submission(window, form, trigger)?;
                }
            }
        }
        Ok(true)
    }

    fn process_deferred_effects(&mut self) -> Result<bool, TestAppError> {
        let mut progress = false;
        let mut global_deliveries = 0;
        let mut entity_deliveries = 0;
        loop {
            let global_type = if self.pending_all_globals {
                self.pending_all_globals = false;
                self.pending_global_notifications.clear();
                self.pending_global_notification_types.clear();
                Some(None)
            } else {
                self.pending_global_notifications
                    .pop_front()
                    .map(|global_type| {
                        self.pending_global_notification_types.remove(&global_type);
                        Some(global_type)
                    })
            };
            if let Some(global_type) = global_type {
                let targets = self.windows();
                for window in targets {
                    if !self
                        .window(window)?
                        .listeners
                        .has_global_subscribers(global_type)
                    {
                        continue;
                    }
                    let subscriptions = self
                        .window(window)?
                        .listeners
                        .global_subscriptions(global_type);
                    for subscription in subscriptions {
                        if !subscription.is_active() {
                            continue;
                        }
                        if !reserve_global_observer_delivery(&mut global_deliveries) {
                            return Err(TestAppError::EffectTurnLimit);
                        }
                        let mut cx = self.event_context(Some(window));
                        subscription.callback.borrow_mut()(
                            self.window_mut(window)?.view.as_any_mut(),
                            &mut cx,
                        );
                        self.apply_context(Some(window), cx)?;
                        progress = true;
                    }
                }
                continue;
            }

            let Some(event) = self.pending_entity_events.pop_front() else {
                break;
            };
            for window in self.windows() {
                if !self
                    .window(window)?
                    .listeners
                    .has_entity_event_subscribers(event.source, event.event_type)
                {
                    continue;
                }
                let callbacks = self
                    .window(window)?
                    .listeners
                    .entity_event_callbacks(event.source, event.event_type);
                for callback in callbacks {
                    if !reserve_entity_event_delivery(&mut entity_deliveries) {
                        return Err(TestAppError::EffectTurnLimit);
                    }
                    let mut cx = self.event_context(Some(window));
                    callback.borrow_mut()(
                        self.window_mut(window)?.view.as_any_mut(),
                        event.value.as_ref(),
                        &mut cx,
                    );
                    self.apply_context(Some(window), cx)?;
                    progress = true;
                }
            }
        }
        Ok(progress)
    }

    fn process_foreground_tasks(&mut self) -> Result<bool, TestAppError> {
        let mut batch = self.foreground_tasks.take_ready_batch();
        let progress = !batch.is_empty();
        while let Some(ScheduledForegroundTask {
            task,
            window,
            runnable,
        }) = batch.pop_front()
        {
            if !self.foreground_tasks.owns(task, window) {
                drop(runnable);
                continue;
            }
            if window.is_some_and(|window| !self.windows.contains_key(&window)) {
                self.foreground_tasks.cancel_task(task);
                drop(runnable);
                continue;
            }
            if catch_unwind(AssertUnwindSafe(|| runnable.run())).is_err() {
                self.foreground_tasks.cancel_task(task);
                return Err(TestAppError::ForegroundTaskPanicked);
            }
            let mut updates = self.foreground_tasks.take_updates(task);
            debug_assert!(window.is_some() || updates.is_empty());
            while let Some(update) = updates.pop_front() {
                let window = window.expect("view updates require a window-owned task");
                let mut cx = self.event_context(Some(window));
                update(self.window_mut(window)?.view.as_any_mut(), &mut cx);
                self.apply_context(Some(window), cx)?;
            }
        }
        Ok(progress)
    }

    fn rebuild_dirty_windows(&mut self) -> Result<bool, TestAppError> {
        let windows = self
            .windows()
            .into_iter()
            .filter(|window| self.windows[window].dirty)
            .collect::<Vec<_>>();
        if windows.is_empty() {
            return Ok(false);
        }
        let now = self.now.get();
        for window in windows {
            let globals = self.globals.clone();
            let foreground_tasks = self.foreground_tasks.clone();
            let displays = self.displays.clone();
            let keyboard_layout = self.keyboard_layout.clone();
            let assets = self.assets.clone();
            let state_snapshot = self.window(window)?.state;
            let (root, requested, repaint_deadline) = {
                let state = self.window_mut(window)?;
                state.view.render(
                    state.state.viewport_size,
                    state.state.scale_factor,
                    FrameMetrics::default(),
                    state.ui.focused(),
                    state.ui.focus_path(),
                    &mut state.listeners,
                    window,
                    state_snapshot,
                    &displays,
                    &keyboard_layout,
                    &assets,
                    None,
                    &foreground_tasks,
                    &globals,
                )
            };
            let previous_focus = self.window(window)?.ui.focused();
            {
                let state = self.window_mut(window)?;
                state.ui.set_reduce_motion(state.config.reduce_motion);
                state
                    .ui
                    .set_animations_enabled(!state.config.reduce_motion, now);
                state
                    .ui
                    .set_root_for_test(
                        root,
                        state.state.viewport_size,
                        state.state.scale_factor,
                        now,
                    )
                    .map_err(|error| TestAppError::View(error.to_string()))?;
                let mut semantic_layout = SemanticTextLayout;
                state
                    .ui
                    .layout_for_test(
                        state.state.viewport_size,
                        state.state.scale_factor,
                        &mut semantic_layout,
                        now,
                    )
                    .map_err(|error| TestAppError::View(error.to_string()))?;
                if let Some(request) = state.pending_focus.take()
                    && state.ui.is_focusable(request)
                {
                    state.ui.focus(request);
                }
                state.dirty = false;
                state.render_count += 1;
                state.retained_geometry_ready = false;
                state.requested_animation_frame =
                    requested || state.ui.declarative_animation_frame_requested();
                state.repaint_deadline = repaint_deadline;
            }
            self.focus_changed(window, previous_focus)?;
        }
        Ok(true)
    }

    fn invoke_action(
        &mut self,
        window: WindowHandle,
        action: &AnyAction,
    ) -> Result<bool, TestAppError> {
        let path = self.window(window)?.ui.focus_path();
        let mut dispatch = std::mem::take(&mut self.window_mut(window)?.action_dispatch_scratch);
        self.window(window)?
            .ui
            .collect_action_dispatch(&path, action.type_id(), &mut dispatch);
        for binding in dispatch.iter().copied() {
            let listener = self.window(window)?.listeners.action_listener(binding.key);
            let Some(listener) = listener else {
                continue;
            };
            let mut cx = self.event_context(Some(window));
            listener(
                self.window_mut(window)?.view.as_any_mut(),
                action.as_any(),
                &mut cx,
            );
            let propagate = cx.propagate_action;
            let stopped = cx.stop_event_propagation;
            self.apply_context(Some(window), cx)?;
            let consumed = match binding.phase {
                crate::DispatchPhase::Capture => stopped,
                crate::DispatchPhase::Bubble => !propagate,
            };
            if consumed {
                dispatch.clear();
                if let Ok(state) = self.window_mut(window) {
                    state.action_dispatch_scratch = dispatch;
                }
                return Ok(true);
            }
        }
        dispatch.clear();
        if let Ok(state) = self.window_mut(window) {
            state.action_dispatch_scratch = dispatch;
        }
        Ok(false)
    }

    fn invoke_key_event(
        &mut self,
        window: WindowHandle,
        event: KeyListenerEvent,
    ) -> Result<bool, TestAppError> {
        let path = self.window(window)?.ui.focus_path();
        let mut dispatch = std::mem::take(&mut self.window_mut(window)?.key_dispatch_scratch);
        self.window(window)?
            .ui
            .collect_key_dispatch(&path, event.kind(), &mut dispatch);
        let mut default_prevented = false;
        for binding in dispatch.iter().copied() {
            let listener = self.window(window)?.listeners.key_listener(binding.key);
            let Some(listener) = listener else {
                continue;
            };
            let mut cx = self.event_context(Some(window));
            listener(self.window_mut(window)?.view.as_any_mut(), &event, &mut cx);
            let stopped = cx.stop_event_propagation;
            default_prevented |= cx.prevent_default;
            self.apply_context(Some(window), cx)?;
            if stopped {
                break;
            }
        }
        dispatch.clear();
        if let Ok(state) = self.window_mut(window) {
            state.key_dispatch_scratch = dispatch;
        }
        Ok(default_prevented)
    }

    fn invoke_form_submission(
        &mut self,
        window: WindowHandle,
        form: ElementId,
        trigger: Option<ElementId>,
    ) -> Result<(), TestAppError> {
        if self.form_submission_depth >= MAX_NESTED_FORM_SUBMISSIONS {
            return Ok(());
        }
        self.form_submission_depth += 1;
        let result = (|| {
            let previous_focus = self.window(window)?.ui.focused();
            let attempt = self
                .window_mut(window)?
                .ui
                .attempt_form_submission(form, trigger);
            if let Some(attempt) = attempt {
                self.window_mut(window)?.dirty = true;
                self.focus_changed(window, previous_focus)?;
                let mut cx = self.event_context(Some(window));
                match attempt {
                    FormAttempt::Valid(event) => {
                        let listener = self
                            .window(window)?
                            .listeners
                            .form_submits
                            .get(&form)
                            .cloned();
                        if let Some(listener) = listener {
                            listener(self.window_mut(window)?.view.as_any_mut(), &event, &mut cx);
                        }
                    }
                    FormAttempt::Invalid(report) => {
                        let listener = self
                            .window(window)?
                            .listeners
                            .form_invalids
                            .get(&form)
                            .cloned();
                        if let Some(listener) = listener {
                            listener(self.window_mut(window)?.view.as_any_mut(), &report, &mut cx);
                        }
                    }
                }
                self.apply_context(Some(window), cx)?;
            }
            Ok(())
        })();
        self.form_submission_depth -= 1;
        result
    }

    fn apply_input_result(
        &mut self,
        window: WindowHandle,
        result: InputResult,
    ) -> Result<(), TestAppError> {
        let Some(change) = result.change else {
            return Ok(());
        };
        let listener = self
            .window(window)?
            .listeners
            .inputs
            .get(&change.id)
            .cloned();
        if let Some(listener) = listener {
            let mut cx = self.event_context(Some(window));
            listener(
                self.window_mut(window)?.view.as_any_mut(),
                &change.value,
                &mut cx,
            );
            self.apply_context(Some(window), cx)?;
        }
        Ok(())
    }

    fn dispatch_bindings(
        &mut self,
        window: WindowHandle,
        bindings: &[KeyBinding],
    ) -> Result<bool, TestAppError> {
        for binding in bindings {
            if self.invoke_action(window, binding.action())? {
                return Ok(true);
            }
        }
        Ok(false)
    }

    fn handle_keystroke_fallback(
        &mut self,
        window: WindowHandle,
        stroke: Keystroke,
    ) -> Result<(), TestAppError> {
        let modifiers = stroke.modifiers;
        let default_prevented = self.invoke_key_event(
            window,
            KeyListenerEvent::Down(KeyDownEvent {
                key: stroke.key.clone(),
                key_char: stroke.key_char.clone(),
                modifiers,
                repeat: false,
            }),
        )?;
        if !default_prevented {
            let input_focused = self.window(window)?.ui.focused_text_input().is_some();
            let mut input_result = None;
            if input_focused {
                let extend = modifiers.contains(Modifiers::SHIFT);
                let primary = primary_modifier(modifiers);
                input_result = match &stroke.key {
                    Key::Character(value)
                        if primary
                            && modifiers.contains(Modifiers::SHIFT)
                            && value.eq_ignore_ascii_case("z") =>
                    {
                        Some(self.window_mut(window)?.ui.input_redo())
                    }
                    Key::Character(value) if primary && value.eq_ignore_ascii_case("z") => {
                        Some(self.window_mut(window)?.ui.input_undo())
                    }
                    Key::Character(value) if primary && value.eq_ignore_ascii_case("y") => {
                        Some(self.window_mut(window)?.ui.input_redo())
                    }
                    Key::Character(value) if primary && value.eq_ignore_ascii_case("a") => {
                        Some(self.window_mut(window)?.ui.input_select_all())
                    }
                    Key::Character(value) if primary && value.eq_ignore_ascii_case("c") => {
                        let selected = self.window(window)?.ui.selected_input_text();
                        if let Some(selected) = selected
                            && let Ok(item) = ClipboardItem::new_string(selected)
                        {
                            let _ = self.clipboard.write(ClipboardTarget::General, item);
                        }
                        None
                    }
                    Key::Character(value) if primary && value.eq_ignore_ascii_case("x") => {
                        let selected = self.window(window)?.ui.selected_input_text();
                        let copied = selected.is_some_and(|selected| {
                            ClipboardItem::new_string(selected)
                                .and_then(|item| {
                                    self.clipboard.write(ClipboardTarget::General, item)
                                })
                                .is_ok()
                        });
                        if copied {
                            Some(self.window_mut(window)?.ui.input_backspace())
                        } else {
                            None
                        }
                    }
                    Key::Character(value) if primary && value.eq_ignore_ascii_case("v") => {
                        let pasted = self
                            .clipboard
                            .read(ClipboardTarget::General)
                            .ok()
                            .flatten()
                            .and_then(|item| item.text());
                        if let Some(text) = pasted {
                            Some(self.window_mut(window)?.ui.input_replace(&text))
                        } else {
                            None
                        }
                    }
                    _ if !modifiers.intersects(Modifiers::CONTROL | Modifiers::SUPER)
                        && matches!(stroke.key_char.as_ref(), Some(Key::Character(_))) =>
                    {
                        let Some(Key::Character(value)) = stroke.key_char.as_ref() else {
                            unreachable!();
                        };
                        Some(self.window_mut(window)?.ui.input_replace(value))
                    }
                    Key::Character(value)
                        if !modifiers.intersects(Modifiers::CONTROL | Modifiers::SUPER) =>
                    {
                        Some(self.window_mut(window)?.ui.input_replace(value))
                    }
                    Key::Space if !modifiers.intersects(Modifiers::CONTROL | Modifiers::SUPER) => {
                        Some(self.window_mut(window)?.ui.input_replace(" "))
                    }
                    Key::Backspace => Some(self.window_mut(window)?.ui.input_backspace()),
                    Key::Delete => Some(self.window_mut(window)?.ui.input_delete()),
                    Key::ArrowLeft => Some(self.window_mut(window)?.ui.input_move_left(extend)),
                    Key::ArrowRight => Some(self.window_mut(window)?.ui.input_move_right(extend)),
                    Key::Home => Some(self.window_mut(window)?.ui.input_move_home(extend)),
                    Key::End => Some(self.window_mut(window)?.ui.input_move_end(extend)),
                    Key::Enter if self.window(window)?.ui.focused_text_input_is_multiline() => {
                        Some(self.window_mut(window)?.ui.input_insert_newline())
                    }
                    _ => None,
                };
            }
            let submitted = matches!(&stroke.key, Key::Enter)
                && input_focused
                && !self.window(window)?.ui.focused_text_input_is_multiline()
                && self.submit_focused_input(window)?;
            if !submitted {
                if let Some(result) = input_result {
                    self.apply_input_result(window, result)?;
                } else {
                    match &stroke.key {
                        Key::Tab => {
                            let previous = self.window(window)?.ui.focused();
                            self.window_mut(window)?
                                .ui
                                .focus_next(modifiers.contains(Modifiers::SHIFT));
                            self.focus_changed(window, previous)?;
                        }
                        Key::ArrowLeft if modifiers.is_empty() => {
                            if !self.navigate_adjacent_tab(window, false, true)?
                                && let Some(target) = self.window(window)?.ui.adjacent_radio(true)
                            {
                                self.click(window, target)?;
                            }
                        }
                        Key::ArrowRight if modifiers.is_empty() => {
                            if !self.navigate_adjacent_tab(window, false, false)?
                                && let Some(target) = self.window(window)?.ui.adjacent_radio(false)
                            {
                                self.click(window, target)?;
                            }
                        }
                        Key::ArrowUp if modifiers.is_empty() => {
                            if !self.navigate_adjacent_tab(window, true, true)?
                                && let Some(target) = self.window(window)?.ui.adjacent_radio(true)
                            {
                                self.click(window, target)?;
                            }
                        }
                        Key::ArrowDown if modifiers.is_empty() => {
                            if !self.navigate_adjacent_tab(window, true, false)?
                                && let Some(target) = self.window(window)?.ui.adjacent_radio(false)
                            {
                                self.click(window, target)?;
                            }
                        }
                        Key::Home if modifiers.is_empty() => {
                            self.navigate_edge_tab(window, false)?;
                        }
                        Key::End if modifiers.is_empty() => {
                            self.navigate_edge_tab(window, true)?;
                        }
                        Key::Escape => {
                            // Production always has a painted dismissal stack before native key
                            // input can arrive. Semantic tests normally skip paint, so materialize
                            // the same retained geometry lazily before querying the top surface.
                            self.prepare_retained_geometry(window)?;
                            if let Some(request) = self.window(window)?.ui.dismiss_topmost() {
                                self.dismiss(window, request)?;
                            }
                        }
                        Key::Enter | Key::Space => {
                            if let Some(element) = self.window(window)?.ui.activate_focused() {
                                self.click(window, element)?;
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
        self.queue_dispatch(TestDispatch::Event(
            window,
            Event::KeyDown {
                key: stroke.key,
                key_char: stroke.key_char,
                modifiers,
                repeat: false,
            },
        ))?;
        Ok(())
    }

    fn dismiss(
        &mut self,
        window: WindowHandle,
        request: DismissRequest,
    ) -> Result<(), TestAppError> {
        let listener = self
            .window(window)?
            .listeners
            .dismisses
            .get(&request.id)
            .cloned();
        let mut cx = self.event_context(Some(window));
        if let Some(focus) = request.restore_focus {
            cx.focus = Some(Some(focus));
        }
        if let Some(listener) = listener {
            listener(self.window_mut(window)?.view.as_any_mut(), &mut cx);
        }
        self.apply_context(Some(window), cx)?;
        self.queue_dispatch(TestDispatch::Event(window, Event::Dismiss(request.id)))
    }

    fn submit_focused_input(&mut self, window: WindowHandle) -> Result<bool, TestAppError> {
        let Some(id) = self.window(window)?.ui.focused_text_input() else {
            return Ok(false);
        };
        let value = self
            .window(window)?
            .ui
            .focused_text_input_value()
            .unwrap_or_else(|| Arc::from(""));
        let invalid = self.window(window)?.ui.focused_text_input_is_invalid();
        if let Some(form) = self.window(window)?.ui.form_for_control(id) {
            self.invoke_form_submission(window, form, Some(id))?;
            return Ok(true);
        }
        let listener = self.window(window)?.listeners.submits.get(&id).cloned();
        let Some(listener) = listener else {
            return Ok(false);
        };
        if !invalid {
            let mut cx = self.event_context(Some(window));
            listener(self.window_mut(window)?.view.as_any_mut(), &value, &mut cx);
            self.apply_context(Some(window), cx)?;
        }
        Ok(true)
    }

    fn focus_changed(
        &mut self,
        window: WindowHandle,
        previous: Option<ElementId>,
    ) -> Result<(), TestAppError> {
        let focused = self.window(window)?.ui.focused();
        if focused != previous {
            self.window_mut(window)?.dirty = true;
            self.queue_immediate([TestDispatch::Event(window, Event::FocusChanged(focused))])?;
        }
        Ok(())
    }

    fn navigate_adjacent_tab(
        &mut self,
        window: WindowHandle,
        vertical_axis: bool,
        reverse: bool,
    ) -> Result<bool, TestAppError> {
        let target = self.window(window)?.ui.adjacent_tab(vertical_axis, reverse);
        self.apply_tab_navigation(window, target)
    }

    fn navigate_edge_tab(
        &mut self,
        window: WindowHandle,
        last: bool,
    ) -> Result<bool, TestAppError> {
        let target = self.window(window)?.ui.edge_tab(last);
        self.apply_tab_navigation(window, target)
    }

    fn apply_tab_navigation(
        &mut self,
        window: WindowHandle,
        target: Option<TabNavigationTarget>,
    ) -> Result<bool, TestAppError> {
        let Some(target) = target else {
            return Ok(false);
        };
        let previous = self.window(window)?.ui.focused();
        self.window_mut(window)?.ui.focus(target.id);
        self.focus_changed(window, previous)?;
        self.run_until_idle()?;
        if target.activate {
            self.click(window, target.id)?;
        }
        Ok(true)
    }

    fn set_active_window(&mut self, window: Option<WindowHandle>) -> Result<(), TestAppError> {
        if let Some(window) = window {
            self.window(window)?;
            self.focus_history.retain(|candidate| *candidate != window);
            self.focus_history.push(window);
        }
        self.active_window = window;
        self.refresh_window_focus_states()
    }

    fn refresh_window_focus_states(&mut self) -> Result<(), TestAppError> {
        for (handle, state) in &mut self.windows {
            let focused = Some(*handle) == self.active_window;
            if state.state.focused != focused {
                state.state.focused = focused;
                if state.listeners.observes_window_state {
                    state.dirty = true;
                }
            }
        }
        Ok(())
    }

    fn apply_window_command(&mut self, command: WindowCommand) -> Result<(), TestAppError> {
        let window = command.handle();
        #[cfg(feature = "inspector")]
        let animation_epoch = self.animation_epoch;
        let updates_display = matches!(
            &command,
            WindowCommand::SetBounds(_, _)
                | WindowCommand::Move(_, _)
                | WindowCommand::Resize(_, _)
                | WindowCommand::Restore(_)
        );
        let displays = self.displays.clone();
        let Some(state) = self.windows.get_mut(&window) else {
            return Ok(());
        };
        let previous = state.state;
        match command {
            WindowCommand::SetTitle(_, title) => state.config.title = title,
            WindowCommand::SetRepresentedFile(_, represented_file) => {
                state.config.represented_file = represented_file;
                state.state.represented_file = state.config.represented_file.is_some();
            }
            WindowCommand::SetDocumentEdited(_, edited) => {
                state.config.document_edited = edited;
                state.state.document_edited = edited;
            }
            WindowCommand::ShowCharacterPalette(_) => {}
            WindowCommand::SetTabbingIdentifier(_, identifier) => {
                state.config.tabbing_identifier = identifier;
                state.state.native_tabbing = state.config.tabbing_identifier.is_some();
                if !state.state.native_tabbing {
                    state.state.native_tabs = WindowTabState::default();
                }
            }
            WindowCommand::SelectNextTab(_) => {
                if let Some(selected) = state.state.native_tabs.selected_index {
                    state.state.native_tabs.selected_index =
                        Some(selected.saturating_add(1) % state.state.native_tabs.count.max(1));
                }
            }
            WindowCommand::SelectPreviousTab(_) => {
                if let Some(selected) = state.state.native_tabs.selected_index {
                    state.state.native_tabs.selected_index = Some(if selected == 0 {
                        state.state.native_tabs.count.saturating_sub(1)
                    } else {
                        selected - 1
                    });
                }
            }
            WindowCommand::SelectTab(_, index) => {
                if index < state.state.native_tabs.count {
                    state.state.native_tabs.selected_index = Some(index);
                }
            }
            WindowCommand::MergeAllWindows(_) => {}
            WindowCommand::MoveTabToNewWindow(_) => {
                state.state.native_tabs = WindowTabState::default();
            }
            WindowCommand::ToggleTabBar(_) => {
                state.state.native_tabs.tab_bar_visible = !state.state.native_tabs.tab_bar_visible;
            }
            WindowCommand::ToggleTabOverview(_) => {
                state.state.native_tabs.overview_visible =
                    !state.state.native_tabs.overview_visible;
            }
            WindowCommand::SetBounds(_, bounds) => set_test_window_bounds(state, bounds),
            WindowCommand::Move(_, position) => {
                let bounds = state.state.bounds.bounds();
                set_test_window_bounds(
                    state,
                    WindowBounds::Windowed(Rect::new(
                        position.x,
                        position.y,
                        bounds.width,
                        bounds.height,
                    )),
                );
            }
            WindowCommand::Resize(_, size) => {
                let bounds = state.state.bounds.bounds();
                set_test_window_bounds(
                    state,
                    WindowBounds::Windowed(Rect::new(bounds.x, bounds.y, size.width, size.height)),
                );
            }
            WindowCommand::Minimize(_) => state.state.minimized = true,
            WindowCommand::Restore(_) => {
                state.state.minimized = false;
                let bounds = state.state.bounds.bounds();
                set_test_window_bounds(state, WindowBounds::Windowed(bounds));
            }
            WindowCommand::Zoom(_) => {
                let bounds = state.state.bounds.bounds();
                set_test_window_bounds(
                    state,
                    if state.state.maximized {
                        WindowBounds::Windowed(bounds)
                    } else {
                        WindowBounds::Maximized(bounds)
                    },
                );
            }
            WindowCommand::ToggleFullscreen(_) => {
                let bounds = state.state.bounds.bounds();
                set_test_window_bounds(
                    state,
                    if state.state.fullscreen {
                        WindowBounds::Windowed(bounds)
                    } else {
                        WindowBounds::Fullscreen(bounds)
                    },
                );
            }
            WindowCommand::SetFullscreen(_, fullscreen) => {
                let bounds = state.state.bounds.bounds();
                set_test_window_bounds(
                    state,
                    if fullscreen {
                        WindowBounds::Fullscreen(bounds)
                    } else {
                        WindowBounds::Windowed(bounds)
                    },
                );
            }
            WindowCommand::SetVisible(_, visible) => state.state.visible = visible,
            WindowCommand::SetMovable(_, movable) => state.state.movable = movable,
            WindowCommand::SetResizable(_, resizable) => state.state.resizable = resizable,
            WindowCommand::SetMinimumSize(_, minimum) => {
                if state.config.minimum_size != minimum {
                    state.config.minimum_size = minimum;
                    state.state.minimum_size = minimum;
                }
            }
            WindowCommand::SetMinimizable(_, minimizable) => {
                state.state.minimizable = minimizable;
            }
            WindowCommand::SetAppearance(_, preference) => {
                state.config.preferred_appearance = preference;
                state.state.appearance = preference.unwrap_or(state.system_appearance);
            }
            WindowCommand::SetBackgroundAppearance(_, appearance) => {
                if state.config.window_background != appearance {
                    state.config.window_background = appearance;
                    state.state.background_appearance = appearance;
                    state.dirty = true;
                }
            }
            #[cfg(feature = "inspector")]
            WindowCommand::SetInspector(_, open) => {
                if state.inspector.is_some() != open {
                    state.config.inspector = open;
                    state.state.inspector_active = open;
                    state.inspector = open.then(|| InspectorState::new(animation_epoch));
                    state.retained_geometry_ready = false;
                }
            }
            #[cfg(feature = "inspector")]
            WindowCommand::ToggleInspector(_) => {
                let open = state.inspector.is_none();
                state.config.inspector = open;
                state.state.inspector_active = open;
                state.inspector = open.then(|| InspectorState::new(animation_epoch));
                state.retained_geometry_ready = false;
            }
            WindowCommand::RequestAttention(_) => {}
        }
        if updates_display {
            state.state.display_id =
                crate::display::display_for_rect(&displays, state.state.bounds.bounds());
        }
        if state.state != previous && state.listeners.observes_window_state {
            state.dirty = true;
        }
        Ok(())
    }
}

impl<V: View> App<V> {
    /// Consume this configured application into a deterministic headless test context.
    pub fn into_test_context(self) -> Result<(TestAppContext, TestWindowHandle<V>), TestAppError> {
        TestAppContext::from_app(self)
    }
}

impl Drop for TestAppContext {
    fn drop(&mut self) {
        self.foreground_tasks.shutdown();
    }
}

fn test_window_state(
    handle: WindowHandle,
    options: &WindowOptions,
    focused: bool,
    displays: &Displays,
    parent: Option<WindowState>,
) -> WindowState {
    let preferred_display = options.display_id.and_then(|id| displays.find(id));
    let placement_display = preferred_display.or_else(|| displays.primary());
    let popover_bounds = options
        .popover
        .as_ref()
        .zip(parent)
        .map(|(popover, parent)| {
            let parent = parent.bounds.bounds();
            let anchor = Rect::new(
                parent.x + popover.anchor_rect.x,
                parent.y + popover.anchor_rect.y,
                popover.anchor_rect.width,
                popover.anchor_rect.height,
            );
            #[cfg(any(target_os = "macos", test))]
            {
                let anchor_center = Point::new(
                    anchor.x + anchor.width * 0.5,
                    anchor.y + anchor.height * 0.5,
                );
                let visible = displays
                    .all()
                    .iter()
                    .find(|display| display.visible_bounds().contains(anchor_center))
                    .or_else(|| displays.primary())
                    .map_or_else(
                        || Rect::new(-1_000_000.0, -1_000_000.0, 2_000_000.0, 2_000_000.0),
                        Display::visible_bounds,
                    );
                crate::popover::place_popover(anchor, options.size, visible, popover)
            }
            #[cfg(not(any(target_os = "macos", test)))]
            {
                crate::popover::unconstrained_popover_rect(anchor, options.size, popover)
            }
        });
    let bounds = popover_bounds.map_or_else(
        || {
            options.window_bounds.unwrap_or_else(|| {
                WindowBounds::Windowed(if options.display_id.is_some() {
                    placement_display.map_or_else(
                        || Rect::from_size(options.size),
                        |display| display.centered_bounds(options.size),
                    )
                } else {
                    Rect::from_size(options.size)
                })
            })
        },
        WindowBounds::Windowed,
    );
    let rect = bounds.bounds();
    let display_id = preferred_display
        .map(Display::id)
        .or_else(|| crate::display::display_for_rect(displays, rect));
    WindowState {
        handle,
        display_id,
        kind: options.kind,
        bounds,
        viewport_size: Size::new(rect.width, rect.height),
        minimum_size: options.minimum_size,
        scale_factor: display_id
            .and_then(|id| displays.find(id))
            .map_or(1.0, Display::scale_factor),
        appearance: options.preferred_appearance.unwrap_or_default(),
        background_appearance: options.window_background,
        focused,
        visible: options.show,
        minimized: false,
        maximized: matches!(bounds, WindowBounds::Maximized(_)),
        fullscreen: matches!(bounds, WindowBounds::Fullscreen(_)),
        occluded: false,
        movable: options.is_movable,
        resizable: options.is_resizable,
        minimizable: options.is_minimizable,
        represented_file: options.represented_file.is_some(),
        document_edited: options.document_edited,
        native_tabbing: options.tabbing_identifier.is_some(),
        native_tabs: WindowTabState::default(),
        #[cfg(feature = "inspector")]
        inspector_active: options.inspector,
    }
}

fn set_test_window_bounds(window: &mut TestWindow, bounds: WindowBounds) {
    let rect = bounds.bounds();
    window.config.size = Size::new(rect.width, rect.height);
    window.config.window_bounds = Some(bounds);
    window.state.bounds = bounds;
    window.state.viewport_size = Size::new(rect.width, rect.height);
    window.state.maximized = matches!(bounds, WindowBounds::Maximized(_));
    window.state.fullscreen = matches!(bounds, WindowBounds::Fullscreen(_));
    window.dirty = true;
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;

    #[cfg(feature = "inspector")]
    use crate::AccessibilityRole;
    use crate::{
        AnchorPlacement, Animation, AnimationExt as _, BundledAssets, Interpolate, ListState,
        MAX_CUSTOM_FONTS, SpringAnimation, SpringConfig, SpringPlayback, Tooltip, button,
        container_query, div, form, submit_button, text, text_input,
    };

    crate::actions!(
        test_context_commands,
        [SaveForTest, BubbleForTest, LoopForTest]
    );

    #[cfg(feature = "inspector")]
    struct InspectorTestView;

    #[cfg(feature = "inspector")]
    impl View for InspectorTestView {
        fn render(&mut self, cx: &mut ViewContext<'_, Self>) -> impl IntoElement {
            let focus = cx.focus_handle("inspector-button");
            div().id("inspector-root").size_full().child(
                button()
                    .id("inspector-button")
                    .track_focus(focus)
                    .auto_focus()
                    .accessibility_label("Inspectable button")
                    .size(120.0, 36.0)
                    .child(text("Inspect me")),
            )
        }
    }

    #[cfg(feature = "inspector")]
    #[test]
    fn inspector_snapshot_reuses_production_tree_and_toggle_does_not_rebuild_clean_view() {
        let (mut cx, view) = App::new(InspectorTestView)
            .size(480.0, 320.0)
            .inspector(true)
            .into_test_context()
            .unwrap();
        let window = view.window_handle();
        assert!(cx.window_state(window).unwrap().inspector_active);

        let snapshot = cx.inspector_snapshot(window).unwrap().unwrap();
        assert_eq!(snapshot.viewport, Size::new(480.0, 320.0));
        assert!(!snapshot.nodes_truncated);
        let root = snapshot
            .nodes
            .iter()
            .find(|node| node.id == ElementId::named("inspector-root"))
            .unwrap();
        let control = snapshot
            .nodes
            .iter()
            .find(|node| node.id == ElementId::named("inspector-button"))
            .unwrap();
        assert_eq!(control.parent, Some(root.id));
        assert!(control.focused && control.on_focus_path);
        assert_eq!(control.accessibility.role, AccessibilityRole::Button);
        assert_eq!(
            control.accessibility.label.as_deref(),
            Some("Inspectable button")
        );
        assert!(control.hit_region.is_some_and(|hit| hit.focusable));

        let frame = cx.capture_screenshot(window).unwrap();
        let y = frame.height() / 2;
        assert_ne!(frame.pixel(8, y), frame.pixel(frame.width() - 8, y));

        let renders = cx.render_count(window).unwrap();
        cx.update(view, |_view, cx| cx.toggle_inspector().unwrap())
            .unwrap();
        assert!(!cx.window_state(window).unwrap().inspector_active);
        assert!(cx.inspector_snapshot(window).unwrap().is_none());
        assert_eq!(cx.render_count(window).unwrap(), renders);

        cx.update(view, |_view, cx| cx.set_inspector(true).unwrap())
            .unwrap();
        assert!(cx.window_state(window).unwrap().inspector_active);
        assert_eq!(cx.render_count(window).unwrap(), renders);
    }

    #[derive(Default)]
    struct AssetAccessView {
        rendered: Arc<str>,
        clicked: Arc<str>,
    }

    impl View for AssetAccessView {
        fn render(&mut self, cx: &mut ViewContext<'_, Self>) -> impl IntoElement {
            let message = cx.assets().load_required("copy/message.txt").unwrap();
            self.rendered = Arc::from(std::str::from_utf8(message.as_ref()).unwrap());
            let load_from_event = cx.listener("load-asset", |this, cx| {
                let message = cx.asset_source().load_required("copy/message.txt").unwrap();
                this.clicked = Arc::from(std::str::from_utf8(message.as_ref()).unwrap());
                cx.invalidate();
            });
            button().on_click(load_from_event).child("Load")
        }
    }

    #[test]
    fn application_assets_are_shared_with_view_and_event_contexts() {
        let bundle = BundledAssets::new()
            .with("copy/message.txt", &b"bundled message"[..])
            .unwrap();
        let (mut cx, view) = App::new(AssetAccessView::default())
            .with_assets(bundle)
            .into_test_context()
            .unwrap();

        assert_eq!(
            cx.read(view, |view| view.rendered.clone())
                .unwrap()
                .as_ref(),
            "bundled message"
        );
        cx.click(view.window_handle(), "load-asset").unwrap();
        assert_eq!(
            cx.read(view, |view| view.clicked.clone()).unwrap().as_ref(),
            "bundled message"
        );
    }

    #[test]
    fn invalid_or_missing_custom_fonts_fail_before_creating_a_test_window() {
        assert!(matches!(
            App::new(AssetAccessView::default())
                .font(&b"not-a-font"[..])
                .into_test_context(),
            Err(TestAppError::Asset(AssetError::InvalidFont { index: 0 }))
        ));
        assert!(matches!(
            App::new(AssetAccessView::default())
                .font("fonts/missing.ttf")
                .into_test_context(),
            Err(TestAppError::Asset(AssetError::NotFound(path)))
                if path.as_ref() == "fonts/missing.ttf"
        ));
        assert!(matches!(
            App::new(AssetAccessView::default())
                .fonts((0..=MAX_CUSTOM_FONTS).map(|_| vec![0_u8]))
                .into_test_context(),
            Err(TestAppError::Asset(AssetError::TooManyFonts))
        ));
    }

    #[derive(Default)]
    struct ContainerQuerySemanticView {
        clicks: usize,
    }

    impl View for ContainerQuerySemanticView {
        fn render(&mut self, cx: &mut ViewContext<'_, Self>) -> impl IntoElement {
            let activate = cx.listener("responsive-query-action", |this, cx| {
                this.clicks += 1;
                cx.invalidate();
            });
            container_query(move |size| {
                div()
                    .id(if size.width < 480.0 {
                        "compact-query-branch"
                    } else {
                        "wide-query-branch"
                    })
                    .child(button().on_click(activate).child("Activate"))
            })
        }
    }

    #[test]
    fn semantic_context_materializes_and_reconciles_container_queries_without_a_gpu() {
        let (mut cx, view) = App::new(ContainerQuerySemanticView::default())
            .size(360.0, 240.0)
            .into_test_context()
            .unwrap();
        let window = view.window_handle();

        assert!(cx.contains_element(window, "compact-query-branch").unwrap());
        assert!(!cx.contains_element(window, "wide-query-branch").unwrap());
        cx.click(window, "responsive-query-action").unwrap();
        assert_eq!(cx.read(view, |view| view.clicks).unwrap(), 1);
        assert!(cx.visual_renderer.is_none());

        cx.update(view, |_view, cx| {
            cx.resize_window(Size::new(720.0, 240.0)).unwrap();
        })
        .unwrap();
        assert!(!cx.contains_element(window, "compact-query-branch").unwrap());
        assert!(cx.contains_element(window, "wide-query-branch").unwrap());
        assert!(cx.visual_renderer.is_none());
    }

    #[derive(Default)]
    struct InteractionView {
        value: Arc<str>,
        submitted: Arc<str>,
        submissions: usize,
        clicks: usize,
        click_events: usize,
        key_events: usize,
        focus_events: usize,
        saves: usize,
        child_bubbles: usize,
        parent_bubbles: usize,
    }

    impl View for InteractionView {
        fn event(&mut self, event: &Event, _cx: &mut EventContext) {
            match event {
                Event::Click(_) => self.click_events += 1,
                Event::KeyDown { .. } => self.key_events += 1,
                Event::FocusChanged(_) => self.focus_events += 1,
                _ => {}
            }
        }

        fn render(&mut self, cx: &mut ViewContext<'_, Self>) -> impl IntoElement {
            let root = cx.focus_handle("interaction-root");
            let input_focus = cx.focus_handle("name");
            let edit = cx.input_listener("name", |this, value, cx| {
                this.value = Arc::from(value);
                cx.invalidate();
            });
            let submit = cx.form_submit_listener("profile", |this, event, cx| {
                this.submitted = Arc::from(event.value("name").unwrap_or_default());
                this.submissions += 1;
                cx.invalidate();
            });
            let increment = cx.listener("increment", |this, cx| {
                this.clicks += 1;
                cx.invalidate();
            });
            let save = cx.action_listener("name", |this, _: &SaveForTest, cx| {
                this.saves += 1;
                cx.invalidate();
            });
            let child_bubble = cx.action_listener("name", |this, _: &BubbleForTest, cx| {
                this.child_bubbles += 1;
                cx.propagate();
            });
            let parent_bubble =
                cx.action_listener("interaction-root", |this, _: &BubbleForTest, cx| {
                    this.parent_bubbles += 1;
                    cx.invalidate();
                });

            div()
                .focus_scope(root)
                .key_context("Workspace")
                .on_action(parent_bubble)
                .child(
                    form()
                        .on_form_submit(submit)
                        .child(
                            text_input(self.value.clone())
                                .track_focus(input_focus)
                                .auto_focus()
                                .key_context("Editor")
                                .on_input(edit)
                                .on_action(save)
                                .on_action(child_bubble),
                        )
                        .child(submit_button().id("submit").child(text("Submit"))),
                )
                .child(button().on_click(increment).child(text("Increment")))
        }
    }

    #[test]
    fn interaction_focus_actions_keymap_input_and_forms_share_production_state() {
        let app = App::new(InteractionView::default()).bind_keys([
            KeyBinding::new("ctrl-s", SaveForTest, Some("Editor")),
            KeyBinding::new("ctrl-k left", BubbleForTest, Some("Workspace > Editor")),
        ]);
        let (mut cx, view) = app.into_test_context().unwrap();
        let window = view.window_handle();

        assert!(cx.contains_element(window, "name").unwrap());
        assert!(cx.contains_element(window, "increment").unwrap());
        assert_eq!(cx.focused(window).unwrap(), Some(ElementId::named("name")));

        cx.simulate_input(window, "Ada").unwrap();
        assert_eq!(
            cx.focused_input_value(window).unwrap().as_deref(),
            Some("Ada")
        );
        cx.simulate_keystrokes(window, "ctrl-s").unwrap();
        assert!(cx.dispatch_action(window, BubbleForTest).unwrap());
        cx.simulate_keystrokes(window, "x backspace").unwrap();
        cx.simulate_keystrokes(window, "enter").unwrap();

        cx.click(window, "submit").unwrap();
        cx.click(window, "increment").unwrap();
        assert_eq!(
            cx.focused(window).unwrap(),
            Some(ElementId::named("increment"))
        );
        cx.read(view, |view| {
            assert_eq!(&*view.value, "Ada");
            assert_eq!(&*view.submitted, "Ada");
            assert_eq!(view.submissions, 2);
            assert_eq!(view.saves, 1);
            assert_eq!(view.child_bubbles, 1);
            assert_eq!(view.parent_bubbles, 1);
            assert_eq!(view.clicks, 1);
            assert_eq!(view.click_events, 2);
            assert_eq!(view.key_events, 3);
            assert!(view.focus_events >= 3);
        })
        .unwrap();
    }

    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    struct PhaseAction {
        stop_at_parent: bool,
        move_focus: bool,
        fall_through: bool,
    }

    #[derive(Default)]
    struct FocusedDispatchView {
        trace: Vec<&'static str>,
        value: Arc<str>,
        root_key_events: usize,
        disable_parent: bool,
    }

    impl View for FocusedDispatchView {
        fn event(&mut self, event: &Event, _cx: &mut EventContext) {
            if matches!(event, Event::KeyDown { .. } | Event::KeyUp { .. }) {
                self.root_key_events += 1;
            }
        }

        fn render(&mut self, cx: &mut ViewContext<'_, Self>) -> impl IntoElement {
            let root = cx.focus_handle("phase-root");
            let target = cx.focus_handle("phase-target");
            let alternate = cx.focus_handle("phase-alternate");

            let root_action_capture =
                cx.action_listener("phase-root", move |this, action: &PhaseAction, cx| {
                    this.trace.push("action-root-capture");
                    if action.move_focus {
                        cx.focus(alternate);
                    }
                });
            let parent_action_capture =
                cx.action_listener("phase-parent", |this, action: &PhaseAction, cx| {
                    this.trace.push("action-parent-capture");
                    if action.stop_at_parent {
                        cx.stop_propagation();
                    }
                });
            let target_action_capture =
                cx.action_listener("phase-target", |this, _: &PhaseAction, _cx| {
                    this.trace.push("action-target-capture");
                });
            let target_action_bubble =
                cx.action_listener("phase-target", |this, _: &PhaseAction, cx| {
                    this.trace.push("action-target-bubble");
                    cx.propagate();
                });
            let parent_action_bubble =
                cx.action_listener("phase-parent", |this, _: &PhaseAction, cx| {
                    this.trace.push("action-parent-bubble");
                    cx.propagate();
                });
            let root_action_bubble =
                cx.action_listener("phase-root", |this, action: &PhaseAction, cx| {
                    this.trace.push("action-root-bubble");
                    if action.fall_through {
                        cx.propagate();
                    }
                });

            let root_key_down = cx.key_down_listener("phase-root", |this, _event, _cx| {
                this.trace.push("key-root-capture-or-bubble");
            });
            let root_key_down_bubble = cx.key_down_listener("phase-root", |this, _event, _cx| {
                this.trace.push("key-root-bubble");
            });
            let parent_key_down = cx.key_down_listener("phase-parent", |this, event, cx| {
                this.trace.push("key-parent-capture");
                if matches!(
                    &event.key,
                    Key::Character(value) if value == "s" || value == "p"
                ) {
                    if matches!(&event.key, Key::Character(value) if value == "p") {
                        cx.prevent_default();
                    }
                    cx.stop_propagation();
                }
            });
            let parent_key_down_bubble =
                cx.key_down_listener("phase-parent", |this, _event, _cx| {
                    this.trace.push("key-parent-bubble");
                });
            let target_key_down = cx.key_down_listener("phase-target", |this, event, cx| {
                this.trace.push("key-target-capture");
                if matches!(&event.key, Key::Character(value) if value == "x") {
                    cx.prevent_default();
                }
            });
            let target_key_down_bubble =
                cx.key_down_listener("phase-target", |this, _event, _cx| {
                    this.trace.push("key-target-bubble");
                });

            let root_key_up = cx.key_up_listener("phase-root", |this, _event, _cx| {
                this.trace.push("up-root-capture");
            });
            let root_key_up_bubble = cx.key_up_listener("phase-root", |this, _event, _cx| {
                this.trace.push("up-root-bubble");
            });
            let target_key_up = cx.key_up_listener("phase-target", |this, _event, _cx| {
                this.trace.push("up-target-capture");
            });
            let target_key_up_bubble = cx.key_up_listener("phase-target", |this, _event, _cx| {
                this.trace.push("up-target-bubble");
            });
            let edit = cx.input_listener("phase-target", |this, value, cx| {
                this.value = Arc::from(value);
                cx.invalidate();
            });

            div()
                .focus_scope(root)
                .capture_action(root_action_capture)
                .on_action(root_action_bubble)
                .capture_key_down(root_key_down)
                .on_key_down(root_key_down_bubble)
                .capture_key_up(root_key_up)
                .on_key_up(root_key_up_bubble)
                .child(
                    div()
                        .id("phase-parent")
                        .disabled(self.disable_parent)
                        .capture_action(parent_action_capture)
                        .on_action(parent_action_bubble)
                        .capture_key_down(parent_key_down)
                        .on_key_down(parent_key_down_bubble)
                        .child(
                            text_input(self.value.clone())
                                .track_focus(target)
                                .auto_focus()
                                .on_input(edit)
                                .capture_action(target_action_capture)
                                .on_action(target_action_bubble)
                                .capture_key_down(target_key_down)
                                .on_key_down(target_key_down_bubble)
                                .capture_key_up(target_key_up)
                                .on_key_up(target_key_up_bubble),
                        ),
                )
                .child(button().track_focus(alternate).child("Alternate"))
        }
    }

    #[test]
    fn focused_actions_use_capture_then_default_consuming_bubble_without_idle_rebuilds() {
        let (mut cx, view) = TestAppContext::new(FocusedDispatchView::default()).unwrap();
        let window = view.window_handle();
        let renders = cx.render_count(window).unwrap();

        assert!(
            cx.dispatch_action(
                window,
                PhaseAction {
                    stop_at_parent: false,
                    move_focus: false,
                    fall_through: false,
                },
            )
            .unwrap()
        );
        cx.read(view, |view| {
            assert_eq!(
                view.trace,
                [
                    "action-root-capture",
                    "action-parent-capture",
                    "action-target-capture",
                    "action-target-bubble",
                    "action-parent-bubble",
                    "action-root-bubble",
                ]
            );
        })
        .unwrap();
        assert_eq!(cx.render_count(window).unwrap(), renders);
    }

    #[test]
    fn keymap_actions_precede_raw_key_listeners_and_can_explicitly_fall_through() {
        let consuming = PhaseAction {
            stop_at_parent: false,
            move_focus: false,
            fall_through: false,
        };
        let (mut cx, view) = App::new(FocusedDispatchView::default())
            .bind_keys([KeyBinding::new("x", consuming, None)])
            .into_test_context()
            .unwrap();
        let window = view.window_handle();
        cx.simulate_keystrokes(window, "x").unwrap();
        cx.read(view, |view| {
            assert_eq!(
                view.trace,
                [
                    "action-root-capture",
                    "action-parent-capture",
                    "action-target-capture",
                    "action-target-bubble",
                    "action-parent-bubble",
                    "action-root-bubble",
                ]
            );
            assert!(view.value.is_empty());
            assert_eq!(view.root_key_events, 0);
        })
        .unwrap();

        let falling_through = PhaseAction {
            stop_at_parent: false,
            move_focus: false,
            fall_through: true,
        };
        let (mut cx, view) = App::new(FocusedDispatchView::default())
            .bind_keys([KeyBinding::new("a", falling_through, None)])
            .into_test_context()
            .unwrap();
        let window = view.window_handle();
        cx.simulate_keystrokes(window, "a").unwrap();
        cx.read(view, |view| {
            assert_eq!(
                view.trace,
                [
                    "action-root-capture",
                    "action-parent-capture",
                    "action-target-capture",
                    "action-target-bubble",
                    "action-parent-bubble",
                    "action-root-bubble",
                    "key-root-capture-or-bubble",
                    "key-parent-capture",
                    "key-target-capture",
                    "key-target-bubble",
                    "key-parent-bubble",
                    "key-root-bubble",
                ]
            );
            assert_eq!(&*view.value, "a");
            assert_eq!(view.root_key_events, 1);
        })
        .unwrap();
    }

    #[test]
    fn capture_actions_can_stop_or_retarget_focus_without_mutating_the_frozen_path() {
        let (mut cx, view) = TestAppContext::new(FocusedDispatchView::default()).unwrap();
        let window = view.window_handle();
        assert!(
            cx.dispatch_action(
                window,
                PhaseAction {
                    stop_at_parent: true,
                    move_focus: false,
                    fall_through: false,
                },
            )
            .unwrap()
        );
        cx.read(view, |view| {
            assert_eq!(view.trace, ["action-root-capture", "action-parent-capture"]);
        })
        .unwrap();

        let (mut cx, view) = TestAppContext::new(FocusedDispatchView::default()).unwrap();
        let window = view.window_handle();
        assert!(
            cx.dispatch_action(
                window,
                PhaseAction {
                    stop_at_parent: false,
                    move_focus: true,
                    fall_through: false,
                },
            )
            .unwrap()
        );
        cx.read(view, |view| {
            assert_eq!(
                view.trace,
                [
                    "action-root-capture",
                    "action-parent-capture",
                    "action-target-capture",
                    "action-target-bubble",
                    "action-parent-bubble",
                    "action-root-bubble",
                ]
            );
        })
        .unwrap();
        assert_eq!(
            cx.focused(window).unwrap(),
            Some(ElementId::named("phase-alternate"))
        );
    }

    #[test]
    fn focused_key_dispatch_orders_phases_and_separates_stop_from_prevent_default() {
        let (mut cx, view) = TestAppContext::new(FocusedDispatchView::default()).unwrap();
        let window = view.window_handle();
        cx.simulate_keystrokes(window, "x").unwrap();
        cx.read(view, |view| {
            assert_eq!(
                view.trace,
                [
                    "key-root-capture-or-bubble",
                    "key-parent-capture",
                    "key-target-capture",
                    "key-target-bubble",
                    "key-parent-bubble",
                    "key-root-bubble",
                ]
            );
            assert!(view.value.is_empty());
            assert_eq!(view.root_key_events, 1);
        })
        .unwrap();

        let (mut cx, view) = TestAppContext::new(FocusedDispatchView::default()).unwrap();
        let window = view.window_handle();
        cx.simulate_keystrokes(window, "s").unwrap();
        cx.read(view, |view| {
            assert_eq!(
                view.trace,
                ["key-root-capture-or-bubble", "key-parent-capture"]
            );
            assert_eq!(&*view.value, "s");
        })
        .unwrap();

        let (mut cx, view) = TestAppContext::new(FocusedDispatchView::default()).unwrap();
        let window = view.window_handle();
        cx.simulate_keystrokes(window, "p").unwrap();
        cx.read(view, |view| {
            assert_eq!(
                view.trace,
                ["key-root-capture-or-bubble", "key-parent-capture"]
            );
            assert!(view.value.is_empty());
        })
        .unwrap();
    }

    #[test]
    fn focused_key_up_uses_capture_and_bubble_without_scheduling_a_rebuild() {
        let (mut cx, view) = TestAppContext::new(FocusedDispatchView::default()).unwrap();
        let window = view.window_handle();
        let renders = cx.render_count(window).unwrap();
        cx.simulate_key_up(window, Keystroke::parse("k").unwrap())
            .unwrap();
        cx.read(view, |view| {
            assert_eq!(
                view.trace,
                [
                    "up-root-capture",
                    "up-target-capture",
                    "up-target-bubble",
                    "up-root-bubble",
                ]
            );
            assert_eq!(view.root_key_events, 1);
        })
        .unwrap();
        assert_eq!(cx.render_count(window).unwrap(), renders);
    }

    #[test]
    fn disabled_ancestors_do_not_contribute_action_or_key_listeners_to_the_focus_path() {
        let view = FocusedDispatchView {
            disable_parent: true,
            ..Default::default()
        };
        let (mut cx, view) = TestAppContext::new(view).unwrap();
        let window = view.window_handle();
        assert!(
            cx.dispatch_action(
                window,
                PhaseAction {
                    stop_at_parent: false,
                    move_focus: false,
                    fall_through: false,
                },
            )
            .unwrap()
        );
        cx.read(view, |view| {
            assert_eq!(
                view.trace,
                [
                    "action-root-capture",
                    "action-target-capture",
                    "action-target-bubble",
                    "action-root-bubble",
                ]
            );
        })
        .unwrap();

        let view = FocusedDispatchView {
            disable_parent: true,
            ..Default::default()
        };
        let (mut cx, view) = TestAppContext::new(view).unwrap();
        let window = view.window_handle();
        cx.simulate_keystrokes(window, "x").unwrap();
        cx.read(view, |view| {
            assert_eq!(
                view.trace,
                [
                    "key-root-capture-or-bubble",
                    "key-target-capture",
                    "key-target-bubble",
                    "key-root-bubble",
                ]
            );
            assert!(view.value.is_empty());
        })
        .unwrap();
    }

    struct ExcessKeyListeners;

    impl View for ExcessKeyListeners {
        fn render(&mut self, cx: &mut ViewContext<'_, Self>) -> impl IntoElement {
            let id = ElementId::named("excess-key-listeners");
            let mut element = div().id(id);
            for _ in 0..=crate::MAX_KEY_LISTENERS_PER_ELEMENT {
                let listener = cx.key_down_listener(id, |_this, _event, _cx| {});
                element = element.on_key_down(listener);
            }
            element
        }
    }

    #[test]
    #[should_panic(expected = "focused key listeners")]
    fn focused_key_listener_count_is_bounded_per_element() {
        let _ = TestAppContext::new(ExcessKeyListeners);
    }

    struct ExcessActionListeners;

    impl View for ExcessActionListeners {
        fn render(&mut self, cx: &mut ViewContext<'_, Self>) -> impl IntoElement {
            let id = ElementId::named("excess-action-listeners");
            let mut element = div().id(id);
            for _ in 0..=crate::MAX_ACTION_LISTENERS_PER_ELEMENT {
                let listener = cx.action_listener(id, |_this, _: &PhaseAction, _cx| {});
                element = element.on_action(listener);
            }
            element
        }
    }

    #[test]
    #[should_panic(expected = "typed action listeners")]
    fn focused_action_listener_count_is_bounded_per_element() {
        let _ = TestAppContext::new(ExcessActionListeners);
    }

    #[test]
    fn focused_listener_registries_have_hard_per_window_bounds() {
        let mut listeners = ListenerRegistry::default();
        let key_callback: KeyListenerCallback = Arc::new(|_view, _event, _cx| {});
        for _ in 0..MAX_KEY_LISTENERS_PER_WINDOW {
            listeners.push_key_listener(key_callback.clone());
        }
        assert!(
            catch_unwind(AssertUnwindSafe(|| {
                listeners.push_key_listener(key_callback.clone());
            }))
            .is_err()
        );

        let mut listeners = ListenerRegistry::default();
        let action_callback: ActionCallback = Arc::new(|_view, _action, _cx| {});
        for _ in 0..MAX_ACTION_LISTENERS_PER_WINDOW {
            listeners.push_action_listener(action_callback.clone());
        }
        assert!(
            catch_unwind(AssertUnwindSafe(|| {
                listeners.push_action_listener(action_callback.clone());
            }))
            .is_err()
        );
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn localized_key_equivalents_follow_the_simulated_layout_without_reloading_bindings() {
        let app = App::new(InteractionView::default()).bind_keys([KeyBinding::new(
            "cmd-[",
            SaveForTest,
            Some("Editor"),
        )
        .use_key_equivalents()]);
        let (mut cx, view) = app.into_test_context().unwrap();
        cx.simulate_keyboard_layout_change(
            KeyboardLayout::new("com.apple.keylayout.German", "German").unwrap(),
        )
        .unwrap();
        cx.simulate_keystrokes(view.window_handle(), "cmd-ö")
            .unwrap();
        assert_eq!(cx.read(view, |view| view.saves).unwrap(), 1);
    }

    #[test]
    fn printable_key_char_binding_consumes_without_synthesizing_text() {
        let stroke = Keystroke::new(Key::Character("a".to_owned()), Modifiers::ALT)
            .with_key_char(Key::Character("å".to_owned()));
        let (mut cx, view) = App::new(InteractionView::default())
            .bind_keys([KeyBinding::new("å", SaveForTest, Some("Editor"))])
            .into_test_context()
            .unwrap();
        let window = view.window_handle();
        cx.simulate_keystroke(window, stroke.clone()).unwrap();
        cx.read(view, |view| {
            assert_eq!(view.saves, 1);
            assert!(view.value.is_empty());
        })
        .unwrap();

        let (mut cx, view) = TestAppContext::new(InteractionView::default()).unwrap();
        cx.simulate_keystroke(view.window_handle(), stroke).unwrap();
        assert_eq!(&*cx.read(view, |view| view.value.clone()).unwrap(), "å");
    }

    #[test]
    fn editor_shortcuts_and_event_context_share_the_bounded_clipboard_service() {
        let (mut cx, view) = TestAppContext::new(InteractionView::default()).unwrap();
        let window = view.window_handle();
        let primary = if cfg!(target_os = "macos") {
            "cmd"
        } else {
            "ctrl"
        };

        cx.simulate_input(window, "copied from editor").unwrap();
        cx.simulate_keystrokes(window, &format!("{primary}-a {primary}-c"))
            .unwrap();
        assert_eq!(
            cx.read_from_clipboard()
                .unwrap()
                .and_then(|item| item.text())
                .as_deref(),
            Some("copied from editor")
        );

        cx.update(view, |_view, cx| {
            cx.write_to_clipboard(
                ClipboardItem::new_string_with_metadata("pasted through context", "metadata")
                    .unwrap(),
            )
            .unwrap();
        })
        .unwrap();
        cx.simulate_keystrokes(window, &format!("{primary}-a {primary}-v"))
            .unwrap();
        assert_eq!(
            cx.focused_input_value(window).unwrap().as_deref(),
            Some("pasted through context")
        );
        assert_eq!(
            cx.read_from_clipboard().unwrap().unwrap().metadata(),
            Some("metadata")
        );
    }

    struct WatchedGlobal(u32);

    impl Global for WatchedGlobal {}

    struct GlobalView {
        observing: bool,
        observed: u32,
    }

    impl View for GlobalView {
        fn render(&mut self, cx: &mut ViewContext<'_, Self>) -> impl IntoElement {
            if self.observing {
                self.observed = cx.watch_global::<WatchedGlobal, _>(|global| global.0);
            }
            div()
        }
    }

    #[test]
    fn global_observation_invalidates_only_current_declarative_subscribers() {
        let (mut cx, view) = App::new(GlobalView {
            observing: true,
            observed: 0,
        })
        .global(WatchedGlobal(1))
        .into_test_context()
        .unwrap();
        let window = view.window_handle();
        let initial_renders = cx.render_count(window).unwrap();

        cx.update_global::<WatchedGlobal, _>(|global, _| global.0 = 2)
            .unwrap();
        assert_eq!(cx.read(view, |view| view.observed).unwrap(), 2);
        assert_eq!(cx.render_count(window).unwrap(), initial_renders + 1);

        cx.update(view, |view, cx| {
            view.observing = false;
            cx.invalidate();
        })
        .unwrap();
        let unsubscribed_renders = cx.render_count(window).unwrap();
        cx.update_global::<WatchedGlobal, _>(|global, _| global.0 = 3)
            .unwrap();
        assert_eq!(cx.render_count(window).unwrap(), unsubscribed_renders);
        assert_eq!(cx.read_global::<WatchedGlobal, _>(|global| global.0), 3);
    }

    #[derive(Default)]
    struct AsyncView {
        phase: u8,
        animate: bool,
    }

    impl View for AsyncView {
        fn render(&mut self, cx: &mut ViewContext<'_, Self>) -> impl IntoElement {
            if self.animate {
                cx.request_animation_frame();
            }
            div()
        }
    }

    #[test]
    fn foreground_updates_timers_and_frames_advance_only_when_requested() {
        let (mut cx, view) = TestAppContext::new(AsyncView::default()).unwrap();
        let window = view.window_handle();
        cx.update(view, |_view, cx| {
            cx.spawn(|task_cx: AsyncViewContext<AsyncView>| async move {
                task_cx
                    .update(|view, cx| {
                        view.phase = 1;
                        cx.invalidate();
                    })
                    .await
                    .unwrap();
                task_cx.sleep(Duration::from_millis(10)).await.unwrap();
                task_cx
                    .update(|view, cx| {
                        view.phase = 2;
                        cx.invalidate();
                    })
                    .await
                    .unwrap();
            })
            .unwrap()
            .detach();
        })
        .unwrap();

        assert_eq!(cx.read(view, |view| view.phase).unwrap(), 1);
        cx.advance_time(Duration::from_millis(9)).unwrap();
        assert_eq!(cx.read(view, |view| view.phase).unwrap(), 1);
        cx.advance_time(Duration::from_millis(1)).unwrap();
        assert_eq!(cx.read(view, |view| view.phase).unwrap(), 2);

        cx.update(view, |view, cx| {
            view.animate = true;
            cx.invalidate();
        })
        .unwrap();
        let before_frame = cx.render_count(window).unwrap();
        assert_eq!(cx.advance_frame().unwrap(), 1);
        assert_eq!(cx.render_count(window).unwrap(), before_frame + 1);
        assert_eq!(cx.advance_frame().unwrap(), 1);
        assert_eq!(cx.render_count(window).unwrap(), before_frame + 2);
    }

    struct DeclarativeAnimationView {
        values: Rc<RefCell<Vec<f32>>>,
        max_fps: Option<f32>,
        repeating: bool,
        synced: bool,
    }

    impl View for DeclarativeAnimationView {
        fn render(&mut self, _cx: &mut ViewContext<'_, Self>) -> impl IntoElement {
            let mut animation = Animation::new(Duration::from_millis(100));
            if self.repeating {
                animation = if self.synced {
                    animation.repeat_synced()
                } else {
                    animation.repeat()
                };
            }
            if let Some(max_fps) = self.max_fps {
                animation = animation.with_max_fps(max_fps);
            }
            let values = self.values.clone();
            div().with_animation("declarative-progress", animation, move |element, value| {
                values.borrow_mut().push(value);
                element.id("animated-progress").h(8.0).w(value * 100.0)
            })
        }
    }

    #[test]
    fn declarative_animation_advances_only_on_explicit_frames_and_sleeps_when_done() {
        let values = Rc::new(RefCell::new(Vec::new()));
        let (mut cx, view) = TestAppContext::new(DeclarativeAnimationView {
            values: values.clone(),
            max_fps: None,
            repeating: false,
            synced: false,
        })
        .unwrap();
        let window = view.window_handle();
        let initial_renders = cx.render_count(window).unwrap();
        assert_eq!(values.borrow().as_slice(), &[0.0]);

        cx.advance_time(Duration::from_millis(50)).unwrap();
        assert_eq!(cx.render_count(window).unwrap(), initial_renders);
        assert_eq!(cx.advance_frame().unwrap(), 1);
        assert!((values.borrow().last().copied().unwrap() - 0.5).abs() < 0.001);

        cx.advance_time(Duration::from_millis(50)).unwrap();
        assert_eq!(cx.advance_frame().unwrap(), 1);
        assert_eq!(values.borrow().last().copied(), Some(1.0));
        assert_eq!(cx.advance_frame().unwrap(), 0);
        assert_eq!(cx.render_count(window).unwrap(), initial_renders + 2);
    }

    #[test]
    fn declarative_animation_max_fps_uses_exact_deadlines_without_frame_requests() {
        let values = Rc::new(RefCell::new(Vec::new()));
        let (mut cx, view) = TestAppContext::new(DeclarativeAnimationView {
            values: values.clone(),
            max_fps: Some(10.0),
            repeating: true,
            synced: false,
        })
        .unwrap();
        let window = view.window_handle();
        let initial_renders = cx.render_count(window).unwrap();
        assert_eq!(cx.advance_frame().unwrap(), 0);

        cx.advance_time(Duration::from_millis(99)).unwrap();
        assert_eq!(cx.render_count(window).unwrap(), initial_renders);
        cx.advance_time(Duration::from_millis(1)).unwrap();
        assert_eq!(cx.render_count(window).unwrap(), initial_renders + 1);
        assert!((values.borrow().last().copied().unwrap() - 0.0).abs() < 0.001);
    }

    #[test]
    fn reduced_motion_resolves_static_terminal_and_repeating_phases() {
        let oneshot_values = Rc::new(RefCell::new(Vec::new()));
        let (mut cx, oneshot) = App::new(DeclarativeAnimationView {
            values: oneshot_values.clone(),
            max_fps: None,
            repeating: false,
            synced: false,
        })
        .reduce_motion(true)
        .into_test_context()
        .unwrap();
        assert_eq!(oneshot_values.borrow().as_slice(), &[1.0]);
        assert_eq!(cx.advance_frame().unwrap(), 0);

        let repeating_values = Rc::new(RefCell::new(Vec::new()));
        let repeating_window = cx
            .update(oneshot, |_view, cx| {
                cx.open_window(
                    DeclarativeAnimationView {
                        values: repeating_values.clone(),
                        max_fps: None,
                        repeating: true,
                        synced: false,
                    },
                    WindowOptions::default().reduce_motion(true),
                )
            })
            .unwrap();
        assert_eq!(repeating_values.borrow().as_slice(), &[0.0]);
        assert_eq!(cx.advance_frame().unwrap(), 0);
        assert!(cx.is_window_open(oneshot.window_handle()));
        assert!(cx.is_window_open(repeating_window));
    }

    #[test]
    fn repeat_synced_animations_share_one_application_epoch_across_windows() {
        let first_values = Rc::new(RefCell::new(Vec::new()));
        let (mut cx, first) = TestAppContext::new(DeclarativeAnimationView {
            values: first_values.clone(),
            max_fps: None,
            repeating: true,
            synced: true,
        })
        .unwrap();
        cx.advance_time(Duration::from_millis(25)).unwrap();

        let second_values = Rc::new(RefCell::new(Vec::new()));
        cx.update(first, |_view, cx| {
            cx.open_window(
                DeclarativeAnimationView {
                    values: second_values.clone(),
                    max_fps: None,
                    repeating: true,
                    synced: true,
                },
                WindowOptions::default(),
            );
        })
        .unwrap();
        assert!((second_values.borrow().last().copied().unwrap() - 0.25).abs() < 0.001);

        assert_eq!(cx.advance_frame().unwrap(), 2);
        assert!((first_values.borrow().last().copied().unwrap() - 0.25).abs() < 0.001);
        assert!((second_values.borrow().last().copied().unwrap() - 0.25).abs() < 0.001);
    }

    struct SpringAnimationView {
        target: f32,
        playback: SpringPlayback,
        initial: Option<f32>,
        values: Rc<RefCell<Vec<f32>>>,
    }

    impl View for SpringAnimationView {
        fn render(&mut self, _cx: &mut ViewContext<'_, Self>) -> impl IntoElement {
            let mut spring = SpringAnimation::new(SpringConfig::new(100.0, 2.0, 1.0))
                .to(self.target)
                .with_epsilon(0.01)
                .playback(self.playback);
            if let Some(initial) = self.initial {
                spring = spring.from(initial);
            }
            let values = self.values.clone();
            div().with_spring("retargetable-spring", spring, move |element, value| {
                values.borrow_mut().push(value);
                element.id("spring-position").relative().left(value)
            })
        }
    }

    #[test]
    fn springs_preserve_velocity_when_retargeted_and_pause_without_catching_up() {
        let values = Rc::new(RefCell::new(Vec::new()));
        let (mut cx, view) = TestAppContext::new(SpringAnimationView {
            target: 0.0,
            playback: SpringPlayback::Running,
            initial: None,
            values: values.clone(),
        })
        .unwrap();
        let window = view.window_handle();
        assert_eq!(values.borrow().as_slice(), &[0.0]);
        assert_eq!(cx.advance_frame().unwrap(), 0);

        cx.update(view, |view, cx| {
            view.target = 100.0;
            cx.invalidate();
        })
        .unwrap();
        cx.advance_time(Duration::from_millis(50)).unwrap();
        assert_eq!(cx.advance_frame().unwrap(), 1);
        let before_retarget = values.borrow().last().copied().unwrap();
        assert!(before_retarget > 0.0 && before_retarget < 100.0);

        cx.update(view, |view, cx| {
            view.target = 0.0;
            cx.invalidate();
        })
        .unwrap();
        cx.advance_time(Duration::from_millis(5)).unwrap();
        assert_eq!(cx.advance_frame().unwrap(), 1);
        let after_retarget = values.borrow().last().copied().unwrap();
        assert!(after_retarget > before_retarget);

        cx.update(view, |view, cx| {
            view.playback = SpringPlayback::Paused;
            cx.invalidate();
        })
        .unwrap();
        let paused = values.borrow().last().copied().unwrap();
        cx.advance_time(Duration::from_secs(1)).unwrap();
        assert_eq!(cx.advance_frame().unwrap(), 0);
        assert_eq!(values.borrow().last().copied(), Some(paused));

        cx.update(view, |view, cx| {
            view.playback = SpringPlayback::Running;
            cx.invalidate();
        })
        .unwrap();
        assert_eq!(values.borrow().last().copied(), Some(paused));
        cx.advance_time(Duration::from_millis(5)).unwrap();
        assert_eq!(cx.advance_frame().unwrap(), 1);
        assert_ne!(values.borrow().last().copied(), Some(paused));
        assert!(cx.render_count(window).unwrap() >= 7);
    }

    #[test]
    fn reduced_motion_snaps_springs_to_their_target_without_scheduling() {
        let values = Rc::new(RefCell::new(Vec::new()));
        let (mut cx, _view) = App::new(SpringAnimationView {
            target: 100.0,
            playback: SpringPlayback::Running,
            initial: Some(0.0),
            values: values.clone(),
        })
        .reduce_motion(true)
        .into_test_context()
        .unwrap();
        assert_eq!(values.borrow().as_slice(), &[100.0]);
        assert_eq!(cx.advance_frame().unwrap(), 0);
    }

    struct ChildView(u32);

    impl View for ChildView {
        fn render(&mut self, _cx: &mut ViewContext<'_, Self>) -> impl IntoElement {
            div().child(text(self.0.to_string()))
        }
    }

    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    struct ChildCommand(u32);

    struct ActionChildView;

    impl View for ActionChildView {
        fn render(&mut self, cx: &mut ViewContext<'_, Self>) -> impl IntoElement {
            let choose = cx.listener("child-command", |_view, cx| {
                assert!(cx.dispatch_action_to_parent(ChildCommand(7)));
                cx.close_window();
            });
            button().on_click(choose).auto_focus().child(text("Choose"))
        }
    }

    #[derive(Default)]
    struct ActionParentView {
        child: Option<WindowHandle>,
        command_total: u32,
    }

    impl View for ActionParentView {
        fn render(&mut self, cx: &mut ViewContext<'_, Self>) -> impl IntoElement {
            let command =
                cx.action_listener("parent-action-root", |view, action: &ChildCommand, cx| {
                    view.command_total = view.command_total.saturating_add(action.0);
                    cx.invalidate();
                });
            div()
                .focus_scope(cx.focus_handle("parent-action-root"))
                .on_action(command)
                .child(button().id("parent-focus").auto_focus().child("Parent"))
        }
    }

    #[test]
    fn child_actions_dispatch_to_the_parent_before_the_runtime_returns_idle() {
        let (mut cx, parent) = TestAppContext::new(ActionParentView::default()).unwrap();
        let child = cx
            .update(parent, |view, cx| {
                let child = cx.open_window(ActionChildView, WindowOptions::new("Child action"));
                view.child = Some(child);
                child
            })
            .unwrap();
        assert_eq!(
            cx.window(child).unwrap().parent,
            Some(parent.window_handle())
        );

        cx.click(child, "child-command").unwrap();
        assert!(!cx.is_window_open(child));
        assert_eq!(cx.read(parent, |view| view.command_total).unwrap(), 7);
        let renders = cx.render_count(parent.window_handle()).unwrap();
        cx.run_until_idle().unwrap();
        assert_eq!(cx.render_count(parent.window_handle()).unwrap(), renders);
    }

    #[derive(Default)]
    struct ChildLifecycleParentView {
        child: Option<WindowHandle>,
        closed_count: usize,
    }

    impl View for ChildLifecycleParentView {
        fn render(&mut self, cx: &mut ViewContext<'_, Self>) -> impl IntoElement {
            cx.on_any_child_window_closed(|view, closed, cx| {
                if view.child == Some(closed) {
                    view.child = None;
                    view.closed_count += 1;
                    cx.invalidate();
                }
            });
            div()
        }
    }

    #[test]
    fn declarative_child_close_listener_synchronizes_owner_state_and_returns_to_sleep() {
        let (mut cx, parent) = TestAppContext::new(ChildLifecycleParentView::default()).unwrap();
        let child = cx
            .update(parent, |view, cx| {
                let child = cx.open_window(ChildView(9), WindowOptions::new("Observed child"));
                view.child = Some(child);
                cx.invalidate();
                child
            })
            .unwrap();

        cx.update(parent, |_view, cx| cx.close_window_handle(child))
            .unwrap();

        assert!(!cx.is_window_open(child));
        assert_eq!(
            cx.read(parent, |view| (view.child, view.closed_count))
                .unwrap(),
            (None, 1)
        );
        let renders = cx.render_count(parent.window_handle()).unwrap();
        cx.run_until_idle().unwrap();
        assert_eq!(cx.render_count(parent.window_handle()).unwrap(), renders);
    }

    #[test]
    fn any_child_close_listener_covers_open_and_close_in_one_effect_turn() {
        let (mut cx, parent) = TestAppContext::new(ChildLifecycleParentView::default()).unwrap();
        let child = cx
            .update(parent, |view, cx| {
                let child = cx.open_window(ChildView(11), WindowOptions::new("Ephemeral child"));
                view.child = Some(child);
                cx.close_window_handle(child);
                child
            })
            .unwrap();

        assert!(!cx.is_window_open(child));
        assert_eq!(
            cx.read(parent, |view| (view.child, view.closed_count))
                .unwrap(),
            (None, 1)
        );
    }

    #[derive(Default)]
    struct ParentView {
        child: Option<WindowHandle>,
    }

    impl View for ParentView {
        fn render(&mut self, cx: &mut ViewContext<'_, Self>) -> impl IntoElement {
            let _state = cx.window_state();
            div()
        }
    }

    #[test]
    fn multiple_windows_keep_typed_views_commands_focus_and_tree_ownership() {
        let (mut cx, parent) = App::new(ParentView::default())
            .title("Parent")
            .into_test_context()
            .unwrap();
        let parent_window = parent.window_handle();
        cx.update(parent, |view, cx| {
            let child = cx.open_window(
                ChildView(7),
                WindowOptions::default().title("Child").size(320.0, 180.0),
            );
            view.child = Some(child);
            cx.set_window_title("Renamed parent").unwrap();
            cx.resize_window(Size::new(640.0, 360.0)).unwrap();
        })
        .unwrap();

        let child_window = cx.read(parent, |view| view.child.unwrap()).unwrap();
        let child = cx.typed_window::<ChildView>(child_window).unwrap();
        assert_eq!(cx.read(child, |view| view.0).unwrap(), 7);
        assert_eq!(cx.windows().len(), 2);
        assert_eq!(cx.window_title(parent_window).unwrap(), "Renamed parent");
        assert_eq!(
            cx.window_state(parent_window).unwrap().viewport_size,
            Size::new(640.0, 360.0)
        );
        assert_eq!(cx.active_window(), Some(child_window));

        cx.update(parent, |_view, cx| cx.close_window_handle(child_window))
            .unwrap();
        assert_eq!(cx.active_window(), Some(parent_window));
        assert_eq!(cx.windows(), [parent_window]);

        let replacement = cx
            .update(parent, |view, cx| {
                let child = cx.open_window(ChildView(8), WindowOptions::new("Replacement"));
                view.child = Some(child);
                child
            })
            .unwrap();
        cx.update(parent, |_view, cx| cx.close_window()).unwrap();
        assert!(cx.windows().is_empty());
        assert!(!cx.is_window_open(parent_window));
        assert!(!cx.is_window_open(child_window));
        assert!(!cx.is_window_open(replacement));
    }

    #[test]
    fn minimum_window_size_is_observable_and_mutable_without_polling() {
        let initial = Size::new(480.0, 300.0);
        let (mut cx, view) = App::new(ParentView::default())
            .minimum_size(initial.width, initial.height)
            .into_test_context()
            .unwrap();
        let window = view.window_handle();
        assert_eq!(cx.window_state(window).unwrap().minimum_size, Some(initial));
        let renders = cx.render_count(window).unwrap();

        let runtime_minimum = Size::new(640.0, 420.0);
        cx.update(view, |_view, cx| {
            cx.set_window_minimum_size(runtime_minimum).unwrap();
        })
        .unwrap();
        assert_eq!(
            cx.window_state(window).unwrap().minimum_size,
            Some(runtime_minimum)
        );
        assert_eq!(cx.render_count(window).unwrap(), renders + 1);

        // Repeating the effective constraint does not rebuild an observing declaration.
        cx.update(view, |_view, cx| {
            cx.set_window_minimum_size(runtime_minimum).unwrap();
        })
        .unwrap();
        assert_eq!(cx.render_count(window).unwrap(), renders + 1);

        let invalid = cx
            .update(view, |_view, cx| {
                cx.set_window_minimum_size(Size::new(f32::NAN, 100.0))
            })
            .unwrap();
        assert_eq!(invalid, Err(WindowCommandError::InvalidBounds));
        assert_eq!(
            cx.window_state(window).unwrap().minimum_size,
            Some(runtime_minimum)
        );

        cx.update(view, |_view, cx| {
            cx.clear_window_minimum_size().unwrap();
        })
        .unwrap();
        assert_eq!(cx.window_state(window).unwrap().minimum_size, None);
        assert_eq!(cx.render_count(window).unwrap(), renders + 2);
    }

    #[test]
    fn document_window_state_and_bounded_native_tabs_are_deterministic() {
        let (mut cx, view) = App::new(ParentView::default())
            .document_path("Cargo.toml")
            .document_edited(true)
            .tabbing_identifier("dev.quickgui.workspace")
            .into_test_context()
            .unwrap();
        let window = view.window_handle();
        let initial = cx.window_state(window).unwrap();
        assert!(initial.represented_file);
        assert!(initial.document_edited);
        assert!(initial.native_tabbing);
        assert_eq!(initial.native_tabs, WindowTabState::default());

        cx.update(view, |_view, cx| {
            cx.clear_represented_file().unwrap();
            cx.set_window_edited(false).unwrap();
        })
        .unwrap();
        let state = cx.window_state(window).unwrap();
        assert!(!state.represented_file);
        assert!(!state.document_edited);

        let renders = cx.render_count(window).unwrap();
        let tabs = WindowTabState {
            count: 4,
            selected_index: Some(2),
            tab_bar_visible: true,
            overview_visible: false,
            truncated: false,
        };
        cx.simulate_window_tab_state(window, tabs).unwrap();
        assert_eq!(cx.window_state(window).unwrap().native_tabs, tabs);
        assert_eq!(cx.render_count(window).unwrap(), renders + 1);

        cx.update(view, |_view, cx| cx.select_next_tab().unwrap())
            .unwrap();
        assert_eq!(
            cx.window_state(window).unwrap().native_tabs.selected_index,
            Some(3)
        );
        assert!(matches!(
            cx.simulate_window_tab_state(
                window,
                WindowTabState {
                    count: 0,
                    selected_index: None,
                    tab_bar_visible: false,
                    overview_visible: false,
                    truncated: false,
                },
            ),
            Err(TestAppError::InvalidWindowTabState)
        ));
    }

    struct LifecycleView {
        prevent_close: bool,
        close_requests: usize,
    }

    impl View for LifecycleView {
        fn event(&mut self, event: &Event, cx: &mut EventContext) {
            if matches!(event, Event::CloseRequested) {
                self.close_requests += 1;
                if self.prevent_close {
                    cx.prevent_close();
                }
                cx.invalidate();
            }
        }

        fn render(&mut self, _cx: &mut ViewContext<'_, Self>) -> impl IntoElement {
            div()
        }
    }

    #[test]
    fn lifecycle_callbacks_and_interceptable_close_requests_are_synchronous() {
        let opened = Rc::new(RefCell::new(Vec::new()));
        let reopened = Rc::new(Cell::new(false));
        let wakes = Rc::new(Cell::new(0_usize));
        let keyboard_layouts = Rc::new(RefCell::new(Vec::new()));
        let notification = Rc::new(RefCell::new(None));
        let app = App::new(LifecycleView {
            prevent_close: true,
            close_requests: 0,
        })
        .menu(Menu::new("File"))
        .on_open_urls({
            let opened = opened.clone();
            move |urls, _cx| opened.borrow_mut().extend(urls.iter().map(str::to_owned))
        })
        .on_reopen({
            let reopened = reopened.clone();
            move |visible, _cx| reopened.set(visible)
        })
        .on_system_wake({
            let wakes = wakes.clone();
            move |_cx| wakes.set(wakes.get() + 1)
        })
        .on_keyboard_layout_change({
            let keyboard_layouts = keyboard_layouts.clone();
            move |layout, cx| {
                keyboard_layouts
                    .borrow_mut()
                    .push((layout.id().to_owned(), cx.keyboard_layout().id().to_owned()));
            }
        })
        .on_system_notification_response({
            let notification = notification.clone();
            move |response, _cx| {
                *notification.borrow_mut() = Some((response.tag, response.action_id));
            }
        });
        let (mut cx, view) = app.into_test_context().unwrap();
        let window = view.window_handle();

        assert_eq!(cx.menus().len(), 1);
        cx.simulate_open_urls(["quickgui://one", "quickgui://two"])
            .unwrap();
        cx.simulate_reopen(true).unwrap();
        cx.simulate_system_wake().unwrap();
        cx.simulate_keyboard_layout_change(
            KeyboardLayout::new("com.apple.keylayout.German", "German").unwrap(),
        )
        .unwrap();
        cx.simulate_system_notification_response(SystemNotificationResponse {
            tag: Arc::from("build"),
            action_id: Some(Arc::from("open")),
        })
        .unwrap();
        assert_eq!(&*opened.borrow(), &["quickgui://one", "quickgui://two"]);
        assert!(reopened.get());
        assert_eq!(wakes.get(), 1);
        assert_eq!(
            &*keyboard_layouts.borrow(),
            &[(
                "com.apple.keylayout.German".to_owned(),
                "com.apple.keylayout.German".to_owned(),
            )]
        );
        assert_eq!(
            &*notification.borrow(),
            &Some((Arc::from("build"), Some(Arc::from("open"))))
        );

        assert!(!cx.simulate_close_requested(window).unwrap());
        assert_eq!(cx.read(view, |view| view.close_requests).unwrap(), 1);
        cx.update(view, |view, _cx| view.prevent_close = false)
            .unwrap();
        assert!(cx.simulate_close_requested(window).unwrap());
        assert!(cx.is_exited() || cx.windows().is_empty());
    }

    #[test]
    fn quit_modes_and_post_close_callbacks_preserve_native_lifecycle() {
        let closed = Rc::new(RefCell::new(Vec::new()));
        let reopened = Rc::new(Cell::new(None));
        let app = App::new(ParentView::default())
            .quit_mode(QuitMode::Explicit)
            .on_window_closed({
                let closed = closed.clone();
                move |window, _cx| closed.borrow_mut().push(window)
            })
            .on_reopen({
                let reopened = reopened.clone();
                move |_visible, cx| {
                    reopened.set(Some(
                        cx.open_window(ChildView(99), WindowOptions::new("Reopened")),
                    ));
                }
            });
        let (mut cx, parent) = app.into_test_context().unwrap();
        let parent_window = parent.window_handle();
        let child_window = cx
            .update(parent, |_view, cx| {
                cx.open_window(ChildView(7), WindowOptions::new("Child"))
            })
            .unwrap();

        cx.update(parent, |_view, cx| cx.close_window()).unwrap();
        assert_eq!(&*closed.borrow(), &[child_window, parent_window]);
        assert!(cx.windows().is_empty());
        assert!(!cx.is_exited());

        cx.simulate_reopen(false).unwrap();
        let reopened_window = reopened
            .get()
            .expect("reopen callback should create a window");
        assert!(cx.is_window_open(reopened_window));
        assert_eq!(cx.windows(), [reopened_window]);

        let (mut cx, root) = App::new(ChildView(1))
            .with_quit_mode(QuitMode::LastWindowClosed)
            .into_test_context()
            .unwrap();
        cx.update(root, |_view, cx| cx.close_window()).unwrap();
        assert!(cx.windows().is_empty());
        assert!(cx.is_exited());
    }

    #[test]
    fn explicit_exit_closes_owned_windows_and_runs_each_close_callback() {
        let closed = Rc::new(RefCell::new(Vec::new()));
        let (mut cx, parent) = App::new(ParentView::default())
            .quit_mode(QuitMode::Explicit)
            .on_window_closed({
                let closed = closed.clone();
                move |window, _cx| closed.borrow_mut().push(window)
            })
            .into_test_context()
            .unwrap();
        let parent_window = parent.window_handle();
        let child_window = cx
            .update(parent, |_view, cx| {
                cx.open_window(ChildView(3), WindowOptions::new("Child"))
            })
            .unwrap();

        cx.update(parent, |_view, cx| cx.exit()).unwrap();

        assert!(cx.is_exited());
        assert!(cx.windows().is_empty());
        assert_eq!(&*closed.borrow(), &[child_window, parent_window]);
    }

    struct AppearanceView {
        observe: bool,
        rendered: WindowAppearance,
        native_events: Vec<WindowAppearance>,
    }

    impl View for AppearanceView {
        fn event(&mut self, event: &Event, _cx: &mut EventContext) {
            if let Event::AppearanceChanged(appearance) = event {
                self.native_events.push(*appearance);
            }
        }

        fn render(&mut self, cx: &mut ViewContext<'_, Self>) -> impl IntoElement {
            if self.observe {
                self.rendered = cx.appearance();
            }
            div()
        }
    }

    #[test]
    fn appearance_simulation_respects_forced_and_system_following_modes() {
        let (mut cx, view) = App::new(AppearanceView {
            observe: true,
            rendered: WindowAppearance::Light,
            native_events: Vec::new(),
        })
        .window_appearance(WindowAppearance::Dark)
        .into_test_context()
        .unwrap();
        let window = view.window_handle();
        let initial_renders = cx.render_count(window).unwrap();

        assert_eq!(
            cx.window_state(window).unwrap().appearance,
            WindowAppearance::Dark
        );
        assert_eq!(
            cx.read(view, |view| view.rendered).unwrap(),
            WindowAppearance::Dark
        );

        // A forced window remembers, but does not adopt or dispatch, the system change.
        cx.simulate_appearance_change(window, WindowAppearance::Light)
            .unwrap();
        assert_eq!(
            cx.window_state(window).unwrap().appearance,
            WindowAppearance::Dark
        );
        assert_eq!(cx.render_count(window).unwrap(), initial_renders);
        assert!(cx.read(view, |view| view.native_events.is_empty()).unwrap());

        // Returning to system mode adopts the remembered value and rebuilds the observer.
        cx.update(view, |_view, cx| {
            cx.follow_system_window_appearance().unwrap();
        })
        .unwrap();
        assert_eq!(
            cx.window_state(window).unwrap().appearance,
            WindowAppearance::Light
        );
        assert_eq!(
            cx.read(view, |view| view.rendered).unwrap(),
            WindowAppearance::Light
        );
        assert_eq!(cx.render_count(window).unwrap(), initial_renders + 1);

        cx.simulate_appearance_change(window, WindowAppearance::Dark)
            .unwrap();
        assert_eq!(
            cx.window_state(window).unwrap().appearance,
            WindowAppearance::Dark
        );
        assert_eq!(
            cx.read(view, |view| view.native_events.clone()).unwrap(),
            [WindowAppearance::Dark]
        );
        assert_eq!(cx.render_count(window).unwrap(), initial_renders + 2);

        // Explicit commands update retained state immediately without fabricating an OS event.
        cx.update(view, |_view, cx| {
            cx.set_window_appearance(WindowAppearance::Light).unwrap();
        })
        .unwrap();
        assert_eq!(
            cx.window_state(window).unwrap().appearance,
            WindowAppearance::Light
        );
        assert_eq!(
            cx.read(view, |view| view.native_events.clone()).unwrap(),
            [WindowAppearance::Dark]
        );
    }

    #[test]
    fn unobserved_appearance_event_does_not_rebuild_the_view() {
        let (mut cx, view) = TestAppContext::new(AppearanceView {
            observe: false,
            rendered: WindowAppearance::Light,
            native_events: Vec::new(),
        })
        .unwrap();
        let window = view.window_handle();
        let initial_renders = cx.render_count(window).unwrap();

        cx.simulate_appearance_change(window, WindowAppearance::Dark)
            .unwrap();

        assert_eq!(
            cx.window_state(window).unwrap().appearance,
            WindowAppearance::Dark
        );
        assert_eq!(
            cx.read(view, |view| view.native_events.clone()).unwrap(),
            [WindowAppearance::Dark]
        );
        assert_eq!(cx.render_count(window).unwrap(), initial_renders);
    }

    struct BackgroundAppearanceView {
        rendered: WindowBackgroundAppearance,
    }

    impl View for BackgroundAppearanceView {
        fn render(&mut self, cx: &mut ViewContext<'_, Self>) -> impl IntoElement {
            self.rendered = cx.window_state().background_appearance;
            div()
        }
    }

    #[test]
    fn background_appearance_commands_update_retained_state_once() {
        let (mut cx, view) = App::new(BackgroundAppearanceView {
            rendered: WindowBackgroundAppearance::Opaque,
        })
        .window_background(WindowBackgroundAppearance::Blurred)
        .into_test_context()
        .unwrap();
        let window = view.window_handle();

        assert_eq!(
            cx.window_state(window).unwrap().background_appearance,
            WindowBackgroundAppearance::Blurred
        );
        assert_eq!(
            cx.read(view, |view| view.rendered).unwrap(),
            WindowBackgroundAppearance::Blurred
        );
        let initial_renders = cx.render_count(window).unwrap();

        cx.update(view, |_view, cx| {
            cx.set_window_background_appearance(WindowBackgroundAppearance::Transparent)
                .unwrap();
        })
        .unwrap();
        assert_eq!(
            cx.window_state(window).unwrap().background_appearance,
            WindowBackgroundAppearance::Transparent
        );
        assert_eq!(cx.render_count(window).unwrap(), initial_renders + 1);

        // Repeating an identical compositor command is a no-op and schedules no frame.
        cx.update(view, |_view, cx| {
            cx.set_window_background_appearance_handle(
                window,
                WindowBackgroundAppearance::Transparent,
            )
            .unwrap();
        })
        .unwrap();
        assert_eq!(cx.render_count(window).unwrap(), initial_renders + 1);

        cx.update(view, |_view, cx| {
            cx.set_window_background_appearance(WindowBackgroundAppearance::Opaque)
                .unwrap();
        })
        .unwrap();
        assert_eq!(
            cx.read(view, |view| view.rendered).unwrap(),
            WindowBackgroundAppearance::Opaque
        );
        assert_eq!(cx.render_count(window).unwrap(), initial_renders + 2);
    }

    #[derive(Default)]
    struct GestureView {
        pressure: Option<MousePressureEvent>,
        pinch: Option<PinchEvent>,
        rotation: Option<RotationEvent>,
        smart_magnify: Option<SmartMagnifyEvent>,
        window_events: usize,
    }

    impl View for GestureView {
        fn event(&mut self, event: &Event, _cx: &mut EventContext) {
            if matches!(
                event,
                Event::MousePressure(_)
                    | Event::Pinch(_)
                    | Event::Rotation(_)
                    | Event::SmartMagnify(_)
            ) {
                self.window_events += 1;
            }
        }

        fn render(&mut self, cx: &mut ViewContext<'_, Self>) -> impl IntoElement {
            let pressure = cx.mouse_pressure_listener("pressure", |view, event, cx| {
                view.pressure = Some(*event);
                cx.invalidate();
            });
            let pinch = cx.pinch_listener("pinch", |view, event, cx| {
                view.pinch = Some(*event);
                cx.invalidate();
            });
            let rotation = cx.rotation_listener("rotation", |view, event, cx| {
                view.rotation = Some(*event);
                cx.invalidate();
            });
            let smart_magnify = cx.smart_magnify_listener("smart-magnify", |view, event, cx| {
                view.smart_magnify = Some(*event);
                cx.invalidate();
            });
            div()
                .child(div().on_mouse_pressure(pressure))
                .child(div().on_pinch(pinch))
                .child(div().on_rotation(rotation))
                .child(div().on_smart_magnify(smart_magnify))
        }
    }

    #[test]
    fn gesture_simulation_runs_element_then_window_callbacks_without_native_services() {
        let (mut cx, view) = TestAppContext::new(GestureView::default()).unwrap();
        let window = view.window_handle();
        let pressure = MousePressureEvent {
            position: Point::new(10.0, 20.0),
            pressure: 0.75,
            stage: PressureStage::Force,
            modifiers: Modifiers::SUPER,
        };
        let pinch = PinchEvent {
            position: Point::new(30.0, 40.0),
            delta: 0.125,
            phase: GesturePhase::Moved,
            modifiers: Modifiers::empty(),
        };
        let rotation = RotationEvent {
            position: Point::new(50.0, 60.0),
            delta: -12.0,
            phase: GesturePhase::Ended,
            modifiers: Modifiers::SHIFT,
        };
        let smart_magnify = SmartMagnifyEvent {
            position: Point::new(70.0, 80.0),
            modifiers: Modifiers::ALT,
        };

        cx.simulate_mouse_pressure(window, ElementId::named("pressure"), pressure)
            .unwrap();
        cx.simulate_pinch(window, ElementId::named("pinch"), pinch)
            .unwrap();
        cx.simulate_rotation(window, ElementId::named("rotation"), rotation)
            .unwrap();
        cx.simulate_smart_magnify(window, ElementId::named("smart-magnify"), smart_magnify)
            .unwrap();

        cx.read(view, |view| {
            assert_eq!(view.pressure, Some(pressure));
            assert_eq!(view.pinch, Some(pinch));
            assert_eq!(view.rotation, Some(rotation));
            assert_eq!(view.smart_magnify, Some(smart_magnify));
            assert_eq!(view.window_events, 4);
        })
        .unwrap();
        assert!(matches!(
            cx.simulate_pinch(window, ElementId::named("pressure"), pinch),
            Err(TestAppError::NotListening { kind: "pinch", .. })
        ));
    }

    #[derive(Default)]
    struct ScrollWheelView {
        stop_at_child: bool,
        order: Vec<&'static str>,
        events: Vec<ScrollWheelEvent>,
    }

    impl View for ScrollWheelView {
        fn render(&mut self, cx: &mut ViewContext<'_, Self>) -> impl IntoElement {
            let parent = cx.scroll_wheel_listener("wheel-parent", |view, event, _cx| {
                view.order.push("parent");
                view.events.push(*event);
            });
            let child = cx.scroll_wheel_listener("wheel-child", |view, event, cx| {
                view.order.push("child");
                view.events.push(*event);
                cx.prevent_default();
                if view.stop_at_child {
                    cx.stop_propagation();
                }
            });

            div()
                .on_scroll_wheel(parent)
                .child(div().on_scroll_wheel(child).child(div().id("wheel-target")))
        }
    }

    #[test]
    fn scroll_wheel_simulation_bubbles_and_controls_default_independently() {
        let (mut cx, view) = TestAppContext::new(ScrollWheelView::default()).unwrap();
        let window = view.window_handle();
        let event = ScrollWheelEvent {
            position: Point::new(24.0, 36.0),
            delta: ScrollDelta::Pixels(Vector::new(2.0, -18.0)),
            phase: GesturePhase::Moved,
            modifiers: Modifiers::SUPER,
        };

        assert!(
            cx.simulate_scroll_wheel(window, "wheel-target", event)
                .unwrap()
        );
        cx.read(view, |view| {
            assert_eq!(view.order, ["child", "parent"]);
            assert_eq!(view.events, [event, event]);
        })
        .unwrap();

        cx.update(view, |view, cx| {
            view.stop_at_child = true;
            view.order.clear();
            view.events.clear();
            cx.invalidate();
        })
        .unwrap();
        assert!(
            cx.simulate_scroll_wheel(window, "wheel-target", event)
                .unwrap()
        );
        cx.read(view, |view| {
            assert_eq!(view.order, ["child"]);
            assert_eq!(view.events, [event]);
        })
        .unwrap();

        assert!(matches!(
            cx.simulate_scroll_wheel(window, "missing-wheel-listener", event),
            Err(TestAppError::UnknownElement { .. })
        ));
    }

    #[derive(Default)]
    struct DesktopMouseView {
        stop_at_child: bool,
        order: Vec<&'static str>,
        downs: Vec<MouseDownEvent>,
        ups: Vec<MouseUpEvent>,
        moves: Vec<MouseMoveEvent>,
        exits: Vec<MouseExitEvent>,
        hover: Vec<bool>,
    }

    impl View for DesktopMouseView {
        fn render(&mut self, cx: &mut ViewContext<'_, Self>) -> impl IntoElement {
            let outside = cx.mouse_down_listener("mouse-outside", |view, event, _cx| {
                view.order.push("outside-capture");
                view.downs.push(*event);
            });
            let parent_capture = cx.mouse_down_listener("mouse-parent", |view, event, _cx| {
                view.order.push("parent-capture");
                view.downs.push(*event);
            });
            let parent_bubble = cx.mouse_down_listener("mouse-parent", |view, event, _cx| {
                view.order.push("parent-bubble");
                view.downs.push(*event);
            });
            let child_down = cx.mouse_down_listener("mouse-child", |view, event, cx| {
                view.order.push("child-bubble");
                view.downs.push(*event);
                cx.prevent_default();
                if view.stop_at_child {
                    cx.stop_propagation();
                }
            });
            let child_up = cx.mouse_up_listener("mouse-child", |view, event, _cx| {
                view.ups.push(*event);
            });
            let child_move = cx.mouse_move_listener("mouse-child", |view, event, cx| {
                assert_eq!(cx.pointer_position(), Some(event.position));
                view.moves.push(*event);
            });
            let child_exit = cx.mouse_exit_listener("mouse-child", |view, event, cx| {
                assert_eq!(cx.pointer_position(), Some(event.position));
                view.exits.push(*event);
            });
            let child_hover = cx.hover_listener("mouse-child", |view, hovered, _cx| {
                view.hover.push(*hovered);
            });

            div().children([
                div().id("mouse-outside").on_mouse_down_out(outside),
                div()
                    .id("mouse-parent")
                    .capture_any_mouse_down(parent_capture)
                    .on_mouse_down(MouseButton::Left, parent_bubble)
                    .child(
                        div()
                            .id("mouse-child")
                            .size(100.0, 100.0)
                            .on_any_mouse_down(child_down)
                            .on_mouse_up(MouseButton::Left, child_up)
                            .on_mouse_move(child_move)
                            .on_mouse_exit(child_exit)
                            .on_hover(child_hover),
                    ),
            ])
        }
    }

    #[test]
    fn desktop_mouse_simulation_preserves_capture_bubble_and_default_control() {
        let (mut cx, view) = TestAppContext::new(DesktopMouseView::default()).unwrap();
        let window = view.window_handle();
        let down = MouseDownEvent {
            button: MouseButton::Left,
            position: Point::new(20.0, 30.0),
            modifiers: Modifiers::SUPER,
            click_count: 2,
            first_mouse: true,
        };
        assert!(cx.simulate_mouse_down(window, "mouse-child", down).unwrap());
        cx.read(view, |view| {
            assert_eq!(
                view.order,
                [
                    "outside-capture",
                    "parent-capture",
                    "child-bubble",
                    "parent-bubble",
                ]
            );
            assert_eq!(view.downs, [down, down, down, down]);
        })
        .unwrap();

        cx.update(view, |view, cx| {
            view.stop_at_child = true;
            view.order.clear();
            view.downs.clear();
            cx.invalidate();
        })
        .unwrap();
        assert!(cx.simulate_mouse_down(window, "mouse-child", down).unwrap());
        cx.read(view, |view| {
            assert_eq!(
                view.order,
                ["outside-capture", "parent-capture", "child-bubble"]
            );
            assert_eq!(view.downs, [down, down, down]);
        })
        .unwrap();

        let up = MouseUpEvent {
            button: MouseButton::Left,
            position: down.position,
            modifiers: Modifiers::SHIFT,
            click_count: 2,
        };
        let moved = MouseMoveEvent {
            position: Point::new(40.0, 50.0),
            pressed_button: Some(MouseButton::Left),
            modifiers: Modifiers::ALT,
        };
        let exited = MouseExitEvent {
            position: Point::new(101.0, 50.0),
            pressed_button: None,
            modifiers: Modifiers::empty(),
        };
        assert!(!cx.simulate_mouse_up(window, "mouse-child", up).unwrap());
        cx.simulate_mouse_move(window, "mouse-child", moved)
            .unwrap();
        cx.simulate_mouse_exit(window, "mouse-child", exited)
            .unwrap();
        cx.read(view, |view| {
            assert_eq!(view.ups, [up]);
            assert_eq!(view.moves, [moved]);
            assert_eq!(view.exits, [exited]);
        })
        .unwrap();
    }

    #[test]
    fn visual_pointer_delivers_hover_entry_and_exit_without_idle_frames() {
        let (mut cx, view) = TestAppContext::new(DesktopMouseView::default()).unwrap();
        let window = view.window_handle();
        {
            let mut visual = cx.visual(window).unwrap();
            assert!(visual.move_pointer(Point::new(10.0, 10.0)).unwrap());
            assert!(visual.move_pointer(Point::new(300.0, 300.0)).unwrap());
        }
        assert_eq!(
            cx.read(view, |view| view.hover.clone()).unwrap(),
            [true, false]
        );
        let renders = cx.render_count(window).unwrap();
        cx.run_until_idle().unwrap();
        assert_eq!(cx.render_count(window).unwrap(), renders);
    }

    #[derive(Default)]
    struct MouseMutationView {
        remove_parent: bool,
        child_disabled: bool,
        order: Vec<&'static str>,
    }

    impl View for MouseMutationView {
        fn render(&mut self, cx: &mut ViewContext<'_, Self>) -> impl IntoElement {
            let parent_down = cx.mouse_down_listener("mutation-parent", |view, _event, _cx| {
                view.order.push("parent");
            });
            let child = cx.mouse_down_listener("mutation-child", |view, _event, cx| {
                view.order.push("child");
                view.remove_parent = true;
                cx.invalidate();
            });
            let child = div()
                .id("mutation-child")
                .disabled(self.child_disabled)
                .on_any_mouse_down(child);
            let parent = div().id("mutation-parent").child(child);
            if self.remove_parent {
                parent
            } else {
                parent.on_any_mouse_down(parent_down)
            }
        }
    }

    #[test]
    fn mouse_path_survives_callback_invalidation_and_disabled_nodes_do_not_listen() {
        let (mut cx, view) = TestAppContext::new(MouseMutationView::default()).unwrap();
        let window = view.window_handle();
        let event = MouseDownEvent {
            button: MouseButton::Left,
            position: Point::new(4.0, 4.0),
            modifiers: Modifiers::empty(),
            click_count: 1,
            first_mouse: false,
        };

        assert!(
            !cx.simulate_mouse_down(window, "mutation-child", event)
                .unwrap()
        );
        assert_eq!(
            cx.read(view, |view| view.order.clone()).unwrap(),
            ["child", "parent"]
        );

        cx.update(view, |view, cx| {
            view.order.clear();
            view.remove_parent = false;
            view.child_disabled = true;
            cx.invalidate();
        })
        .unwrap();
        assert!(
            !cx.simulate_mouse_down(window, "mutation-child", event)
                .unwrap()
        );
        assert_eq!(
            cx.read(view, |view| view.order.clone()).unwrap(),
            ["parent"]
        );
    }

    struct RetainedOverflowStressView;

    impl View for RetainedOverflowStressView {
        fn render(&mut self, _cx: &mut ViewContext<'_, Self>) -> impl IntoElement {
            div().size_full().child(
                div()
                    .id("retained-overflow")
                    .size(320.0, 120.0)
                    .overflow_y_scroll()
                    .child(div().w_full().h(100_000.0).flex_none()),
            )
        }
    }

    #[test]
    fn retained_overflow_scrolls_back_and_forth_without_rebuilding_the_view() {
        let (mut cx, view) = TestAppContext::new(RetainedOverflowStressView).unwrap();
        let window = view.window_handle();
        let initial_renders = cx.render_count(window).unwrap();
        assert_eq!(
            cx.retained_scroll_offset(window, "retained-overflow")
                .unwrap(),
            Vector::ZERO
        );

        for _ in 0..4 {
            let mut moved_down = false;
            for _ in 0..256 {
                moved_down |= cx
                    .simulate_retained_scroll(window, "retained-overflow", Vector::new(0.0, -400.0))
                    .unwrap();
            }
            assert!(moved_down);
            assert!(
                cx.retained_scroll_offset(window, "retained-overflow")
                    .unwrap()
                    .y
                    > 0.0
            );

            for _ in 0..256 {
                cx.simulate_retained_scroll(window, "retained-overflow", Vector::new(0.0, 400.0))
                    .unwrap();
            }
            assert_eq!(
                cx.retained_scroll_offset(window, "retained-overflow")
                    .unwrap(),
                Vector::ZERO
            );
        }

        assert_eq!(cx.render_count(window).unwrap(), initial_renders);
        cx.run_until_idle().unwrap();
        assert_eq!(cx.render_count(window).unwrap(), initial_renders);
    }

    #[derive(Default)]
    struct RawTouchView {
        stop_at_child: bool,
        deliveries: Vec<(&'static str, TouchEvent)>,
        window_events: usize,
    }

    impl View for RawTouchView {
        fn event(&mut self, event: &Event, _cx: &mut EventContext) {
            if matches!(event, Event::Touch(_)) {
                self.window_events += 1;
            }
        }

        fn render(&mut self, cx: &mut ViewContext<'_, Self>) -> impl IntoElement {
            let parent = cx.touch_listener("touch-parent", |view, event, _cx| {
                view.deliveries.push(("parent", *event));
            });
            let child = cx.touch_listener("touch-child", |view, event, cx| {
                view.deliveries.push(("child", *event));
                if view.stop_at_child {
                    cx.stop_propagation();
                }
            });
            div()
                .on_touch(parent)
                .child(div().on_touch(child).child(div().id("touch-start-target")))
                .child(div().id("touch-outside-target"))
        }
    }

    #[test]
    fn raw_touch_is_hit_once_captured_bounded_and_cancel_safe() {
        let (mut cx, view) = TestAppContext::new(RawTouchView::default()).unwrap();
        let window = view.window_handle();
        let sample = |id, phase, x| TouchEvent {
            id: TouchId(id),
            phase,
            position: Point::new(x, 30.0),
            force: Some(0.625),
        };

        let started = sample(7, TouchPhase::Started, 20.0);
        let moved = sample(7, TouchPhase::Moved, 800.0);
        let ended = sample(7, TouchPhase::Ended, 900.0);
        cx.simulate_touch(window, "touch-start-target", started)
            .unwrap();
        // The supplied element changes, but ID 7 stays on its original listener path.
        cx.simulate_touch(window, "touch-outside-target", moved)
            .unwrap();
        cx.simulate_touch(window, "touch-outside-target", ended)
            .unwrap();
        // A post-terminal sample has no capture and reaches only the window-level raw event.
        cx.simulate_touch(
            window,
            "touch-outside-target",
            sample(7, TouchPhase::Moved, 950.0),
        )
        .unwrap();

        cx.read(view, |view| {
            assert_eq!(
                view.deliveries,
                [
                    ("child", started),
                    ("parent", started),
                    ("child", moved),
                    ("parent", moved),
                    ("child", ended),
                    ("parent", ended),
                ]
            );
            assert_eq!(view.window_events, 4);
        })
        .unwrap();
        assert!(cx.window(window).unwrap().touch_captures.is_empty());

        cx.update(view, |view, cx| {
            view.stop_at_child = true;
            view.deliveries.clear();
            cx.invalidate();
        })
        .unwrap();
        let cancelled = sample(8, TouchPhase::Cancelled, 40.0);
        cx.simulate_touch(
            window,
            "touch-start-target",
            sample(8, TouchPhase::Started, 30.0),
        )
        .unwrap();
        cx.simulate_touch(window, "touch-outside-target", cancelled)
            .unwrap();
        cx.read(view, |view| {
            assert_eq!(view.deliveries.len(), 2);
            assert!(view.deliveries.iter().all(|(owner, _)| *owner == "child"));
        })
        .unwrap();
        assert!(cx.window(window).unwrap().touch_captures.is_empty());

        cx.update(view, |view, _cx| {
            view.stop_at_child = false;
            view.deliveries.clear();
        })
        .unwrap();
        for id in 0..MAX_ACTIVE_TOUCHES_PER_WINDOW as u64 {
            cx.simulate_touch(
                window,
                "touch-start-target",
                sample(100 + id, TouchPhase::Started, id as f32),
            )
            .unwrap();
        }
        assert_eq!(
            cx.window(window).unwrap().touch_captures.len(),
            MAX_ACTIVE_TOUCHES_PER_WINDOW
        );
        let before_overflow = cx.read(view, |view| view.deliveries.len()).unwrap();
        cx.simulate_touch(
            window,
            "touch-start-target",
            sample(999, TouchPhase::Started, 0.0),
        )
        .unwrap();
        assert_eq!(
            cx.read(view, |view| view.deliveries.len()).unwrap(),
            before_overflow
        );
        assert_eq!(
            cx.window(window).unwrap().touch_captures.len(),
            MAX_ACTIVE_TOUCHES_PER_WINDOW
        );
    }

    struct DisplayObserverView {
        observe: bool,
        display_count: usize,
        primary: Option<DisplayId>,
    }

    impl View for DisplayObserverView {
        fn render(&mut self, cx: &mut ViewContext<'_, Self>) -> impl IntoElement {
            if self.observe {
                self.display_count = cx.displays().len();
                self.primary = cx.primary_display().map(Display::id);
            }
            div()
        }
    }

    fn two_test_displays() -> Displays {
        let primary = DisplayId::new(10);
        let secondary = DisplayId::new(20);
        Displays::new(
            vec![
                Display::new(
                    primary,
                    "Primary",
                    Rect::new(0.0, 0.0, 1_200.0, 800.0),
                    Rect::new(0.0, 24.0, 1_200.0, 776.0),
                    2.0,
                )
                .unwrap(),
                Display::new(
                    secondary,
                    "Secondary",
                    Rect::new(1_200.0, 0.0, 1_000.0, 700.0),
                    Rect::new(1_200.0, 0.0, 1_000.0, 700.0),
                    1.0,
                )
                .unwrap(),
            ],
            Some(primary),
        )
        .unwrap()
    }

    #[test]
    fn display_changes_only_rebuild_declarative_observers() {
        let (mut cx, view) = TestAppContext::new(DisplayObserverView {
            observe: true,
            display_count: 0,
            primary: None,
        })
        .unwrap();
        assert_eq!(cx.render_count(view.window_handle()).unwrap(), 1);

        cx.simulate_displays_change(two_test_displays()).unwrap();
        assert_eq!(cx.render_count(view.window_handle()).unwrap(), 2);
        cx.read(view, |view| {
            assert_eq!(view.display_count, 2);
            assert_eq!(view.primary, Some(DisplayId::new(10)));
        })
        .unwrap();

        cx.update(view, |view, cx| {
            view.observe = false;
            cx.invalidate();
        })
        .unwrap();
        let renders = cx.render_count(view.window_handle()).unwrap();
        cx.simulate_displays_change(Displays::test_default())
            .unwrap();
        assert_eq!(cx.render_count(view.window_handle()).unwrap(), renders);
    }

    struct KeyboardLayoutObserverView {
        observe: bool,
        layout_id: String,
    }

    impl View for KeyboardLayoutObserverView {
        fn render(&mut self, cx: &mut ViewContext<'_, Self>) -> impl IntoElement {
            if self.observe {
                self.layout_id = cx.keyboard_layout().id().to_owned();
            }
            div()
        }
    }

    #[test]
    fn keyboard_layout_changes_only_rebuild_declarative_observers() {
        let (mut cx, view) = TestAppContext::new(KeyboardLayoutObserverView {
            observe: true,
            layout_id: String::new(),
        })
        .unwrap();
        assert_eq!(cx.render_count(view.window_handle()).unwrap(), 1);

        let german = KeyboardLayout::new("com.apple.keylayout.German", "German").unwrap();
        cx.simulate_keyboard_layout_change(german).unwrap();
        assert_eq!(cx.render_count(view.window_handle()).unwrap(), 2);
        assert_eq!(
            cx.read(view, |view| view.layout_id.clone()).unwrap(),
            "com.apple.keylayout.German"
        );

        cx.update(view, |view, cx| {
            view.observe = false;
            cx.invalidate();
        })
        .unwrap();
        let renders = cx.render_count(view.window_handle()).unwrap();
        cx.simulate_keyboard_layout_change(
            KeyboardLayout::new("com.apple.keylayout.US", "U.S.").unwrap(),
        )
        .unwrap();
        assert_eq!(cx.render_count(view.window_handle()).unwrap(), renders);
    }

    #[test]
    fn explicit_display_centers_new_windows_and_falls_back_safely() {
        let (mut cx, root) = TestAppContext::new(DisplayObserverView {
            observe: false,
            display_count: 0,
            primary: None,
        })
        .unwrap();
        let displays = two_test_displays();
        cx.simulate_displays_change(displays).unwrap();

        let secondary = cx
            .update(root, |_view, cx| {
                cx.open_window(
                    DisplayObserverView {
                        observe: false,
                        display_count: 0,
                        primary: None,
                    },
                    WindowOptions::new("Secondary")
                        .size(600.0, 400.0)
                        .display(DisplayId::new(20)),
                )
            })
            .unwrap();
        let state = cx.window_state(secondary).unwrap();
        assert_eq!(state.display_id, Some(DisplayId::new(20)));
        assert_eq!(
            state.bounds,
            WindowBounds::Windowed(Rect::new(1_400.0, 150.0, 600.0, 400.0))
        );

        let fallback = cx
            .update(root, |_view, cx| {
                cx.open_window(
                    DisplayObserverView {
                        observe: false,
                        display_count: 0,
                        primary: None,
                    },
                    WindowOptions::new("Fallback")
                        .size(600.0, 400.0)
                        .display(DisplayId::new(999)),
                )
            })
            .unwrap();
        assert_eq!(
            cx.window_state(fallback).unwrap().display_id,
            Some(DisplayId::new(10))
        );
    }

    struct EmptyVisualTestView;

    impl View for EmptyVisualTestView {
        fn render(&mut self, _cx: &mut ViewContext<'_, Self>) -> impl IntoElement {
            div()
        }
    }

    #[test]
    fn oversized_visual_capture_fails_before_allocating_a_gpu() {
        let (mut cx, view) = App::new(EmptyVisualTestView)
            .size(3_000.0, 32.0)
            .into_test_context()
            .unwrap();
        assert!(matches!(
            cx.capture_screenshot(view.window_handle()),
            Err(TestAppError::Visual(
                crate::VisualTestError::InvalidDimensions
            ))
        ));
        assert!(cx.visual_renderer.is_none());
    }

    #[cfg(target_os = "macos")]
    struct VisualTestView;

    #[cfg(target_os = "macos")]
    impl View for VisualTestView {
        fn render(&mut self, _cx: &mut ViewContext<'_, Self>) -> impl IntoElement {
            div()
                .size_full()
                .bg(Color::rgb8(190, 30, 45))
                .child(
                    div()
                        .id("visual-card")
                        .w(20.0)
                        .h(10.0)
                        .bg(Color::rgb8(20, 80, 210)),
                )
                .child(
                    text("Visual")
                        .id("visual-label")
                        .text_sm()
                        .text_color(Color::WHITE),
                )
        }
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn visual_context_uses_production_layout_and_offscreen_wgpu_capture() {
        let (mut cx, view) = App::new(VisualTestView)
            .size(64.0, 48.0)
            .into_test_context()
            .unwrap();
        let window = view.window_handle();
        let mut visual = cx.visual(window).unwrap();
        visual
            .assert_element_bounds("visual-card", Rect::new(0.0, 0.0, 20.0, 10.0), 0.0)
            .unwrap();
        let label = visual.element_bounds("visual-label").unwrap();
        assert_eq!((label.x, label.y), (20.0, 0.0));
        assert!(label.width > 0.0 && label.height > 0.0);
        let first = visual.capture_screenshot().unwrap();
        let second = visual.capture_screenshot().unwrap();
        assert_eq!((first.width(), first.height()), (128, 96));
        assert_eq!(first.pixel(20, 10), Some([20, 80, 210, 255]));
        assert_eq!(first.pixel(127, 95), Some([190, 30, 45, 255]));
        assert_eq!(
            second
                .assert_matches(&first, crate::VisualTolerance::EXACT)
                .unwrap()
                .differing_pixels,
            0
        );
    }

    #[cfg(target_os = "macos")]
    struct WavyUnderlineVisualView;

    #[cfg(target_os = "macos")]
    impl View for WavyUnderlineVisualView {
        fn render(&mut self, _cx: &mut ViewContext<'_, Self>) -> impl IntoElement {
            div().size_full().p_4().bg(Color::rgb8(8, 10, 14)).child(
                text("GPU WAVY UNDERLINE")
                    .id("wavy-label")
                    .text_size(24.0)
                    .line_height(36.0)
                    .text_color(Color::TRANSPARENT)
                    .text_decoration_color(Color::rgb8(248, 113, 113))
                    .text_decoration_2()
                    .text_decoration_wavy()
                    .whitespace_nowrap(),
            )
        }
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn visual_wavy_underline_is_analytic_stable_and_not_a_solid_bar() {
        let (mut cx, view) = App::new(WavyUnderlineVisualView)
            .size(280.0, 72.0)
            .into_test_context()
            .unwrap();
        let mut visual = cx.visual(view.window_handle()).unwrap();
        let first = visual.capture_screenshot().unwrap();
        let second = visual.capture_screenshot().unwrap();
        assert_eq!(
            second
                .assert_matches(&first, crate::VisualTolerance::EXACT)
                .unwrap()
                .differing_pixels,
            0
        );

        let mut colored_pixels = 0_usize;
        let mut colored_rows = HashSet::new();
        let mut columns = HashMap::<u32, (u64, u32)>::new();
        for (index, pixel) in first.rgba().as_chunks::<4>().0.iter().enumerate() {
            if pixel[0] > 180
                && (70..=160).contains(&pixel[1])
                && (70..=160).contains(&pixel[2])
                && pixel[3] > 0
            {
                let x = index as u32 % first.width();
                let y = index as u32 / first.width();
                colored_pixels += 1;
                colored_rows.insert(y);
                let column = columns.entry(x).or_default();
                column.0 += u64::from(y);
                column.1 += 1;
            }
        }
        assert!(
            colored_pixels > 100,
            "the underline must paint a visible span"
        );
        assert!(
            colored_rows.len() >= 6,
            "a wavy underline must cover several physical rows"
        );
        let center_rows = columns
            .values()
            .filter(|(_, count)| *count > 0)
            .map(|(sum, count)| (sum / u64::from(*count)) as u32)
            .collect::<HashSet<_>>();
        assert!(
            center_rows.len() >= 4,
            "the underline center must vary across x instead of forming a solid bar"
        );
    }

    #[cfg(target_os = "macos")]
    struct OpacityVisualView;

    #[cfg(target_os = "macos")]
    impl View for OpacityVisualView {
        fn render(&mut self, _cx: &mut ViewContext<'_, Self>) -> impl IntoElement {
            let rich = || {
                crate::styled_text("RICH").with_highlights([(
                    0..4,
                    crate::HighlightStyle::default().color(Color::rgb8(248, 40, 72)),
                )])
            };
            div()
                .size_full()
                .flex_col()
                .bg(Color::BLACK)
                .child(
                    div()
                        .id("opacity-reference")
                        .h(36.0)
                        .flex_none()
                        .text_size(24.0)
                        .line_height(32.0)
                        .child(rich()),
                )
                .child(
                    div().h(36.0).flex_none().opacity(0.5).child(
                        div()
                            .id("opacity-target")
                            .size_full()
                            .opacity(0.5)
                            .hover(|style| style.opacity(1.0))
                            .transition(Duration::from_millis(100))
                            .text_size(24.0)
                            .line_height(32.0)
                            .child(rich()),
                    ),
                )
        }
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn subtree_opacity_multiplies_rich_text_and_transitions_without_reshaping() {
        fn maximum_red(snapshot: &crate::VisualSnapshot, bounds: Rect, scale: f32) -> u8 {
            let left = (bounds.x * scale).floor().max(0.0) as u32;
            let top = (bounds.y * scale).floor().max(0.0) as u32;
            let right = (bounds.right() * scale).ceil().min(snapshot.width() as f32) as u32;
            let bottom = (bounds.bottom() * scale)
                .ceil()
                .min(snapshot.height() as f32) as u32;
            (top..bottom)
                .flat_map(|y| (left..right).filter_map(move |x| snapshot.pixel(x, y)))
                .map(|pixel| pixel[0])
                .max()
                .unwrap_or(0)
        }

        let (mut cx, view) = App::new(OpacityVisualView)
            .size(96.0, 72.0)
            .into_test_context()
            .unwrap();
        let window = view.window_handle();
        let render_count = cx.render_count(window).unwrap();
        {
            let mut visual = cx.visual(window).unwrap();
            let reference_bounds = visual.element_bounds("opacity-reference").unwrap();
            let target_bounds = visual.element_bounds("opacity-target").unwrap();
            let scale = 2.0;

            let initial = visual.capture_screenshot().unwrap();
            let stable = visual.capture_screenshot().unwrap();
            stable
                .assert_matches(&initial, crate::VisualTolerance::EXACT)
                .unwrap();
            assert_eq!(
                visual
                    .context
                    .visual_renderer
                    .as_ref()
                    .unwrap()
                    .last_reshaped_text_areas(),
                0
            );
            let reference_red = maximum_red(&initial, reference_bounds, scale);
            let initial_red = maximum_red(&initial, target_bounds, scale);
            assert!(reference_red > initial_red && initial_red > 40);

            assert!(visual.move_pointer(Point::new(12.0, 48.0)).unwrap());
            visual.capture_screenshot().unwrap();
            assert_eq!(
                visual
                    .context
                    .visual_renderer
                    .as_ref()
                    .unwrap()
                    .last_reshaped_text_areas(),
                0
            );

            visual.advance_time(Duration::from_millis(50)).unwrap();
            let midpoint = visual.capture_screenshot().unwrap();
            let midpoint_red = maximum_red(&midpoint, target_bounds, scale);
            assert!(midpoint_red > initial_red);
            assert_eq!(
                visual
                    .context
                    .visual_renderer
                    .as_ref()
                    .unwrap()
                    .last_reshaped_text_areas(),
                0
            );

            visual.advance_time(Duration::from_millis(50)).unwrap();
            let completed = visual.capture_screenshot().unwrap();
            let completed_red = maximum_red(&completed, target_bounds, scale);
            assert!(reference_red > completed_red && completed_red > midpoint_red);
            assert_eq!(
                visual
                    .context
                    .visual_renderer
                    .as_ref()
                    .unwrap()
                    .last_reshaped_text_areas(),
                0
            );
        }
        assert_eq!(cx.render_count(window).unwrap(), render_count);
    }

    #[cfg(target_os = "macos")]
    struct TextOverflowVisualView;

    #[cfg(target_os = "macos")]
    impl View for TextOverflowVisualView {
        fn render(&mut self, _cx: &mut ViewContext<'_, Self>) -> impl IntoElement {
            div()
                .size_full()
                .flex_col()
                .gap_2()
                .p_2()
                .bg(Color::rgb8(18, 19, 23))
                .text_color(Color::WHITE)
                .child(
                    text("Unicode 🙂 alpha beta gamma delta epsilon zeta eta theta iota kappa")
                        .id("clamped-label")
                        .w(132.0)
                        .flex_none()
                        .text_sm()
                        .line_clamp(2)
                        .text_ellipsis(),
                )
                .child(
                    text("/Users/example/a-very-long-directory/important-file.rs")
                        .id("middle-label")
                        .w(132.0)
                        .h(20.0)
                        .flex_none()
                        .text_sm()
                        .whitespace_nowrap()
                        .text_ellipsis_middle()
                        .overflow_hidden(),
                )
        }
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn visual_text_overflow_clamps_layout_and_reuses_an_exact_frame() {
        let (mut cx, view) = App::new(TextOverflowVisualView)
            .size(180.0, 100.0)
            .into_test_context()
            .unwrap();
        let mut visual = cx.visual(view.window_handle()).unwrap();
        let clamped = visual.element_bounds("clamped-label").unwrap();
        assert_eq!((clamped.width, clamped.height), (132.0, 40.0));
        let middle = visual.element_bounds("middle-label").unwrap();
        assert_eq!((middle.width, middle.height), (132.0, 20.0));

        let first = visual.capture_screenshot().unwrap();
        let second = visual.capture_screenshot().unwrap();
        assert_eq!(
            second
                .assert_matches(&first, crate::VisualTolerance::EXACT)
                .unwrap()
                .differing_pixels,
            0
        );
    }

    #[cfg(target_os = "macos")]
    struct TransitionVisualView;

    #[cfg(target_os = "macos")]
    impl View for TransitionVisualView {
        fn render(&mut self, _cx: &mut ViewContext<'_, Self>) -> impl IntoElement {
            div()
                .id("transition-surface")
                .size_full()
                .bg(Color::BLACK)
                .hover(|style| style.bg(Color::WHITE).rounded(12.0))
                .transition(Duration::from_millis(100))
        }
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn paint_only_hover_transitions_reverse_without_rebuilding_the_view() {
        let (mut cx, view) = App::new(TransitionVisualView)
            .size(32.0, 32.0)
            .into_test_context()
            .unwrap();
        let window = view.window_handle();
        let render_count = cx.render_count(window).unwrap();
        {
            let mut visual = cx.visual(window).unwrap();
            let initial = visual.capture_screenshot().unwrap();
            assert_eq!(initial.pixel(32, 32), Some([0, 0, 0, 255]));

            assert!(visual.move_pointer(Point::new(16.0, 16.0)).unwrap());
            let start = visual.capture_screenshot().unwrap();
            assert_eq!(start.pixel(32, 32), Some([0, 0, 0, 255]));

            visual.advance_time(Duration::from_millis(50)).unwrap();
            let midpoint = visual.capture_screenshot().unwrap();
            let midpoint_pixel = midpoint.pixel(32, 32).unwrap();
            assert!((186..=190).contains(&midpoint_pixel[0]));
            assert_eq!(midpoint_pixel[0], midpoint_pixel[1]);
            assert_eq!(midpoint_pixel[1], midpoint_pixel[2]);

            assert!(visual.move_pointer(Point::new(48.0, 48.0)).unwrap());
            let reversed = visual.capture_screenshot().unwrap();
            assert_eq!(reversed.pixel(32, 32), Some(midpoint_pixel));

            visual.advance_time(Duration::from_millis(100)).unwrap();
            let completed = visual.capture_screenshot().unwrap();
            assert_eq!(completed.pixel(32, 32), Some([0, 0, 0, 255]));
        }
        assert_eq!(cx.render_count(window).unwrap(), render_count);
    }

    #[cfg(target_os = "macos")]
    struct DetachedTooltipAnimationVisualView;

    #[cfg(target_os = "macos")]
    impl View for DetachedTooltipAnimationVisualView {
        fn render(&mut self, _cx: &mut ViewContext<'_, Self>) -> impl IntoElement {
            let tooltip = Tooltip::new(div().w(24.0).h(12.0).with_animation(
                "tooltip-fade",
                Animation::new(Duration::from_millis(100)),
                |element, phase| element.bg(Color::interpolate(Color::BLACK, Color::WHITE, phase)),
            ))
            .placement(AnchorPlacement::Bottom)
            .delay(Duration::ZERO)
            .gap(0.0)
            .viewport_margin(0.0);
            div()
                .size_full()
                .items_center()
                .justify_center()
                .bg(Color::BLACK)
                .child(
                    div()
                        .id("detached-tooltip-trigger")
                        .w(24.0)
                        .h(12.0)
                        .tooltip(tooltip),
                )
        }
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn detached_tooltip_motion_rebuilds_only_its_tree_and_drops_its_frame_source() {
        fn brightest(snapshot: &crate::VisualSnapshot) -> u8 {
            snapshot
                .rgba()
                .as_chunks::<4>()
                .0
                .iter()
                .map(|pixel| pixel[0].max(pixel[1]).max(pixel[2]))
                .max()
                .unwrap_or(0)
        }

        let (mut cx, view) = App::new(DetachedTooltipAnimationVisualView)
            .size(64.0, 64.0)
            .into_test_context()
            .unwrap();
        let window = view.window_handle();
        let render_count = cx.render_count(window).unwrap();
        {
            let mut visual = cx.visual(window).unwrap();
            let baseline = brightest(&visual.capture_screenshot().unwrap());
            assert!(baseline <= 4);
            assert!(visual.move_pointer(Point::new(32.0, 32.0)).unwrap());

            let start = visual.capture_screenshot().unwrap();
            assert_eq!(brightest(&start), baseline);
            assert!(
                visual
                    .context
                    .window(window)
                    .unwrap()
                    .ui
                    .detached_animation_frame_requested()
            );

            visual.advance_time(Duration::from_millis(50)).unwrap();
            let midpoint = brightest(&visual.capture_screenshot().unwrap());
            assert!(
                (186..=190).contains(&midpoint),
                "unexpected tooltip midpoint brightness: {midpoint}"
            );

            visual.advance_time(Duration::from_millis(50)).unwrap();
            assert_eq!(brightest(&visual.capture_screenshot().unwrap()), 255);
            assert!(
                !visual
                    .context
                    .window(window)
                    .unwrap()
                    .ui
                    .detached_animation_frame_requested()
            );

            assert!(visual.move_pointer(Point::new(80.0, 80.0)).unwrap());
            assert_eq!(brightest(&visual.capture_screenshot().unwrap()), baseline);
        }
        let ui = &cx.window(window).unwrap().ui;
        assert_eq!(ui.animation_counts().1, 0);
        assert_eq!(ui.next_animation_deadline(), None);
        assert!(!ui.detached_animation_frame_requested());
        assert_eq!(cx.render_count(window).unwrap(), render_count);
    }

    #[cfg(target_os = "macos")]
    struct DetachedDragPreviewAnimationVisualView;

    #[cfg(target_os = "macos")]
    impl View for DetachedDragPreviewAnimationVisualView {
        fn render(&mut self, _cx: &mut ViewContext<'_, Self>) -> impl IntoElement {
            div()
                .size_full()
                .items_center()
                .justify_center()
                .bg(Color::BLACK)
                .child(div().id("detached-drag-source").w(12.0).h(12.0))
        }
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn detached_drag_preview_motion_drops_all_scheduling_when_cleared() {
        fn brightest(snapshot: &crate::VisualSnapshot) -> u8 {
            snapshot
                .rgba()
                .as_chunks::<4>()
                .0
                .iter()
                .map(|pixel| pixel[0].max(pixel[1]).max(pixel[2]))
                .max()
                .unwrap_or(0)
        }

        let (mut cx, view) = App::new(DetachedDragPreviewAnimationVisualView)
            .size(64.0, 64.0)
            .into_test_context()
            .unwrap();
        let window = view.window_handle();
        let render_count = cx.render_count(window).unwrap();
        {
            let mut visual = cx.visual(window).unwrap();
            let baseline = brightest(&visual.capture_screenshot().unwrap());
            assert!(baseline <= 4);
            let preview = div().w(16.0).h(12.0).with_animation(
                "drag-preview-fade",
                Animation::new(Duration::from_millis(100)),
                |element, phase| {
                    element
                        .bg(Color::interpolate(Color::BLACK, Color::WHITE, phase))
                        .clickable()
                },
            );
            let now = visual.context.now();
            let mut renderer = visual.context.visual_renderer.take().unwrap();
            let installed = visual
                .context
                .window_mut(window)
                .unwrap()
                .ui
                .set_drag_preview(
                    Some(preview),
                    ElementId::named("detached-drag-source"),
                    Point::new(32.0, 32.0),
                    Point::new(16.0, 16.0),
                    Some(Point::ZERO),
                    &mut renderer,
                    now,
                )
                .unwrap();
            visual.context.visual_renderer = Some(renderer);
            assert!(installed);

            assert_eq!(brightest(&visual.capture_screenshot().unwrap()), baseline);
            assert!(
                visual
                    .context
                    .window(window)
                    .unwrap()
                    .ui
                    .detached_animation_frame_requested()
            );

            visual.advance_time(Duration::from_millis(50)).unwrap();
            let midpoint = brightest(&visual.capture_screenshot().unwrap());
            assert!((186..=190).contains(&midpoint));

            visual.advance_time(Duration::from_millis(50)).unwrap();
            assert_eq!(brightest(&visual.capture_screenshot().unwrap()), 255);
            assert!(
                visual
                    .context
                    .window_mut(window)
                    .unwrap()
                    .ui
                    .clear_drag_preview()
            );
            assert_eq!(brightest(&visual.capture_screenshot().unwrap()), baseline);
        }
        let ui = &cx.window(window).unwrap().ui;
        assert_eq!(ui.animation_counts().1, 0);
        assert_eq!(ui.next_animation_deadline(), None);
        assert!(!ui.detached_animation_frame_requested());
        assert_eq!(cx.render_count(window).unwrap(), render_count);
    }

    #[cfg(target_os = "macos")]
    struct VariableListVisualView {
        list: ListState,
    }

    #[cfg(target_os = "macos")]
    impl View for VariableListVisualView {
        fn render(&mut self, _cx: &mut ViewContext<'_, Self>) -> impl IntoElement {
            self.list.set_viewport_size(100.0, 60.0);
            let rows = self
                .list
                .render_rows(self.list.visible_rows().range, |index| {
                    div()
                        .id(ElementId::new(0x7000 + index as u64))
                        .h([20.0, 40.0, 30.0, 18.0, 26.0][index])
                        .bg(Color::rgb8(20 + index as u8 * 20, 80, 160))
                });
            div().size_full().child(
                div()
                    .id("measured-list")
                    .relative()
                    .size(100.0, 60.0)
                    .variable_virtual_scroll(&self.list)
                    .child(rows),
            )
        }
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn visual_context_converges_variable_list_measurements_without_view_or_idle_frames() {
        let list = ListState::new(5, 24.0).with_overscan(1);
        let (mut cx, view) = App::new(VariableListVisualView { list: list.clone() })
            .size(100.0, 60.0)
            .into_test_context()
            .unwrap();
        let window = view.window_handle();
        let initial_renders = cx.render_count(window).unwrap();
        let mut visual = cx.visual(window).unwrap();
        visual
            .assert_element_bounds(
                ElementId::new(0x7001),
                Rect::new(0.0, 20.0, 100.0, 40.0),
                0.0,
            )
            .unwrap();
        assert!(list.stats().measured_items >= 3);
        let converged_renders = cx.render_count(window).unwrap();
        assert_eq!(
            converged_renders, initial_renders,
            "mounted measurement convergence must not rerender the declarative view"
        );

        // Re-reading settled geometry performs layout on demand but does not rebuild the view.
        cx.element_bounds(window, ElementId::new(0x7001)).unwrap();
        assert_eq!(cx.render_count(window).unwrap(), converged_renders);
    }

    struct LoopView;

    impl View for LoopView {
        fn render(&mut self, cx: &mut ViewContext<'_, Self>) -> impl IntoElement {
            let focus = cx.focus_handle("loop");
            let listener = cx.action_listener("loop", |_view, _: &LoopForTest, cx| {
                cx.dispatch_action(LoopForTest);
            });
            div().track_focus(focus).auto_focus().on_action(listener)
        }
    }

    #[test]
    fn recursive_effects_fail_at_a_bounded_turn_instead_of_hanging() {
        let (mut cx, view) = TestAppContext::new(LoopView).unwrap();
        assert!(matches!(
            cx.dispatch_action(view.window_handle(), LoopForTest),
            Err(TestAppError::EffectTurnLimit)
        ));
    }
}
