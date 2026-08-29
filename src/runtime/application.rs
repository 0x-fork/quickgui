use super::*;

/// A QuickGUI application whose native event loop is advanced by an external runtime.
///
/// Create and pump this value on the platform application thread. Each call still dispatches
/// redraw and lifecycle callbacks synchronously inside Winit, which is required for correct macOS
/// resize behavior. A blocking [`App::run`] remains the simplest choice for ordinary Rust apps.
#[cfg(not(target_arch = "wasm32"))]
pub struct AppRunner {
    pub(super) event_loop: EventLoop<RuntimeEvent>,
    pub(super) runtime: Runtime,
    pub(super) root_window: WindowHandle,
    pub(super) root_window_pending: bool,
    pub(super) status: AppRunStatus,
}

#[cfg(not(target_arch = "wasm32"))]
impl AppRunner {
    /// Advance native events until a redraw completes, the timeout elapses, or the app exits.
    ///
    /// `None` may block indefinitely. External runtimes can pair it with [`Self::waker`] so their
    /// command producer interrupts the blocked pump without periodic polling.
    pub fn pump(&mut self, timeout: Option<Duration>) -> Result<AppRunStatus, AppError> {
        if matches!(self.status, AppRunStatus::Exited(_)) {
            return Ok(self.status);
        }
        let status = self.event_loop.pump_app_events(timeout, &mut self.runtime);
        if let Some(error) = self.runtime.fatal_error.take() {
            self.status = AppRunStatus::Exited(1);
            return Err(error);
        }
        self.status = match status {
            PumpStatus::Continue => AppRunStatus::Continue,
            PumpStatus::Exit(code) => AppRunStatus::Exited(code),
        };
        Ok(self.status)
    }

    /// Return a thread-safe handle that interrupts a blocking [`Self::pump`] call.
    pub fn waker(&self) -> AppRunnerWaker {
        AppRunnerWaker {
            proxy: self.runtime.event_proxy.clone(),
        }
    }

    /// Stable handle of the initial application window.
    ///
    /// A windowless [`Application`] reserves this handle until [`Self::open_window`] queues its
    /// first top-level window.
    pub const fn root_window(&self) -> WindowHandle {
        self.root_window
    }

    /// Whether the platform application completed its native initialization.
    ///
    /// A newly created runner becomes ready during its first [`Self::pump`] call, even when it
    /// does not yet own a window. Embedding runtimes should wait for this boundary before opening
    /// their first window.
    pub const fn is_ready(&self) -> bool {
        self.runtime.ready
    }

    /// Window currently activated for a synchronous core callback, if any.
    ///
    /// View and event callbacks should normally use [`ViewContext::window_handle`] and
    /// [`EventContext::window_handle`] directly. This accessor lets bindings project that same
    /// core context into their host language without maintaining separate window identity.
    pub fn current_window(&self) -> Option<WindowHandle> {
        self.runtime.current_handle()
    }

    /// Queue a new top-level window from an embedding runtime.
    ///
    /// The handle is stable immediately. The platform window is created during the next event
    /// loop turn, so callers can finish installing retained state before pumping again.
    pub fn open_window<V: View>(
        &mut self,
        view: V,
        options: WindowOptions,
    ) -> Result<WindowHandle, AppError> {
        if !matches!(self.status, AppRunStatus::Continue) {
            return Err(AppError::Window(
                "cannot open a window after the application event loop exited".to_owned(),
            ));
        }
        validate_window_options(&options).map_err(|error| AppError::Window(error.to_string()))?;
        self.runtime
            .event_proxy
            .send_event(RuntimeEvent::ExternalCommandsReady)
            .map_err(|_| AppError::Window("application event loop is closed".to_owned()))?;
        let request = if self.root_window_pending {
            self.root_window_pending = false;
            WindowRequest::with_handle(view, options, None, self.root_window)
        } else {
            WindowRequest::new(view, options)
        };
        let handle = request.handle;
        self.runtime.pending_windows.push_back(request);
        Ok(handle)
    }

    /// Queue a native popover anchored to one currently mounted element in a parent window.
    ///
    /// Embedding runtimes may call this before the parent's first presented frame. Resolution is
    /// queued with the child request and occurs at the window-creation boundary, after every
    /// earlier parent request has completed retained layout. This preserves the same display-aware
    /// behavior and core-owned trigger focus restoration as [`EventContext::open_system_popover`]
    /// without polling or a geometry observer.
    pub fn open_system_popover<V: View>(
        &mut self,
        parent: WindowHandle,
        anchor: ElementId,
        view: V,
        options: WindowOptions,
    ) -> Result<WindowHandle, AppError> {
        if !matches!(self.status, AppRunStatus::Continue) {
            return Err(AppError::Window(
                "cannot open a popover after the application event loop exited".to_owned(),
            ));
        }
        validate_window_options(&options).map_err(|error| AppError::Window(error.to_string()))?;
        if options.kind != WindowKind::SystemPopover || options.popover.is_none() {
            return Err(AppError::Window(
                WindowCommandError::InvalidPopoverConfiguration.to_string(),
            ));
        }
        self.runtime
            .event_proxy
            .send_event(RuntimeEvent::ExternalCommandsReady)
            .map_err(|_| AppError::Window("application event loop is closed".to_owned()))?;
        let mut request = WindowRequest::with_parent(view, options, Some(parent));
        request.popover_anchor_element = Some(anchor);
        let handle = request.handle;
        self.runtime.pending_windows.push_back(request);
        Ok(handle)
    }

    /// Mark one externally owned view dirty and request at most one native redraw.
    ///
    /// A window queued for creation also returns `true`: its first render will read the newest
    /// retained state without scheduling a redundant frame.
    pub fn invalidate_window(&mut self, handle: WindowHandle) -> bool {
        if !matches!(self.status, AppRunStatus::Continue) {
            return false;
        }
        if self
            .runtime
            .pending_windows
            .iter()
            .any(|request| request.handle == handle)
        {
            return true;
        }
        self.runtime.invalidate_external(handle)
    }

    /// Focus one mounted element from an embedding runtime.
    ///
    /// This is the imperative counterpart to [`crate::Element::auto_focus`]. It is intended for
    /// host bindings that expose web-like `element.focus()` behavior after an external event has
    /// returned to the host language. The request is applied synchronously and schedules at most
    /// one redraw when focus changes.
    pub fn focus_element(&mut self, handle: WindowHandle, element: ElementId) -> bool {
        if !matches!(self.status, AppRunStatus::Continue) {
            return false;
        }
        self.runtime.focus_external(handle, element)
    }

    /// Present a platform-native prompt owned by one mounted window.
    ///
    /// This is the embedding-runtime counterpart to [`EventContext::prompt`]. The returned
    /// future remains on the platform thread and resolves after the operating system closes the
    /// prompt; no redraw polling is introduced while it is visible.
    pub fn prompt(
        &mut self,
        window: WindowHandle,
        level: PromptLevel,
        message: impl Into<Arc<str>>,
        detail: Option<&str>,
        buttons: &[PromptButton],
    ) -> Result<PlatformResponse<usize>, PlatformError> {
        if !matches!(self.status, AppRunStatus::Continue)
            || !self.runtime.window_handles.contains_key(&window)
        {
            return Err(PlatformError::Unavailable);
        }
        if self.runtime.platform_requests.len() == crate::MAX_PENDING_PLATFORM_REQUESTS {
            return Err(PlatformError::PendingQueueFull);
        }
        let (request, response) =
            PlatformRequest::prompt(window, level, message, detail.map(Arc::from), buttons)?;
        self.runtime
            .event_proxy
            .send_event(RuntimeEvent::ExternalCommandsReady)
            .map_err(|_| PlatformError::Unavailable)?;
        self.runtime.platform_requests.push_back(request);
        Ok(response)
    }

    /// Present an application-modal native prompt without attaching it to a window.
    pub fn prompt_application(
        &mut self,
        level: PromptLevel,
        message: impl Into<Arc<str>>,
        detail: Option<&str>,
        buttons: &[PromptButton],
    ) -> Result<PlatformResponse<usize>, PlatformError> {
        if !matches!(self.status, AppRunStatus::Continue) {
            return Err(PlatformError::Unavailable);
        }
        if self.runtime.platform_requests.len() == crate::MAX_PENDING_PLATFORM_REQUESTS {
            return Err(PlatformError::PendingQueueFull);
        }
        let (request, response) =
            PlatformRequest::application_prompt(level, message, detail.map(Arc::from), buttons)?;
        self.runtime
            .event_proxy
            .send_event(RuntimeEvent::ExternalCommandsReady)
            .map_err(|_| PlatformError::Unavailable)?;
        self.runtime.platform_requests.push_back(request);
        Ok(response)
    }

    /// Present a platform-native open panel owned by one mounted window.
    ///
    /// `Ok(None)` means the user cancelled. The returned future stays on the platform thread and
    /// resolves after the operating system closes the panel.
    pub fn prompt_for_paths(
        &mut self,
        window: WindowHandle,
        options: PathPromptOptions,
    ) -> Result<PathPromptResponse, PlatformError> {
        if !matches!(self.status, AppRunStatus::Continue)
            || !self.runtime.window_handles.contains_key(&window)
        {
            return Err(PlatformError::Unavailable);
        }
        if self.runtime.platform_requests.len() == crate::MAX_PENDING_PLATFORM_REQUESTS {
            return Err(PlatformError::PendingQueueFull);
        }
        let (request, response) = PlatformRequest::open_paths(window, options)?;
        self.runtime
            .event_proxy
            .send_event(RuntimeEvent::ExternalCommandsReady)
            .map_err(|_| PlatformError::Unavailable)?;
        self.runtime.platform_requests.push_back(request);
        Ok(response)
    }

    /// Present an application-modal native open panel without attaching it to a window.
    pub fn prompt_for_paths_application(
        &mut self,
        options: PathPromptOptions,
    ) -> Result<PathPromptResponse, PlatformError> {
        if !matches!(self.status, AppRunStatus::Continue) {
            return Err(PlatformError::Unavailable);
        }
        if self.runtime.platform_requests.len() == crate::MAX_PENDING_PLATFORM_REQUESTS {
            return Err(PlatformError::PendingQueueFull);
        }
        let (request, response) = PlatformRequest::application_open_paths(options)?;
        self.runtime
            .event_proxy
            .send_event(RuntimeEvent::ExternalCommandsReady)
            .map_err(|_| PlatformError::Unavailable)?;
        self.runtime.platform_requests.push_back(request);
        Ok(response)
    }

    /// Present a platform-native save panel owned by one mounted window.
    ///
    /// `Ok(None)` means the user cancelled. The returned future stays on the platform thread and
    /// resolves after the operating system closes the panel.
    pub fn prompt_for_new_path(
        &mut self,
        window: WindowHandle,
        options: SavePathOptions,
    ) -> Result<SavePathResponse, PlatformError> {
        if !matches!(self.status, AppRunStatus::Continue)
            || !self.runtime.window_handles.contains_key(&window)
        {
            return Err(PlatformError::Unavailable);
        }
        if self.runtime.platform_requests.len() == crate::MAX_PENDING_PLATFORM_REQUESTS {
            return Err(PlatformError::PendingQueueFull);
        }
        let (request, response) = PlatformRequest::save_path(window, options)?;
        self.runtime
            .event_proxy
            .send_event(RuntimeEvent::ExternalCommandsReady)
            .map_err(|_| PlatformError::Unavailable)?;
        self.runtime.platform_requests.push_back(request);
        Ok(response)
    }

    /// Present an application-modal native save panel without attaching it to a window.
    pub fn prompt_for_new_path_application(
        &mut self,
        options: SavePathOptions,
    ) -> Result<SavePathResponse, PlatformError> {
        if !matches!(self.status, AppRunStatus::Continue) {
            return Err(PlatformError::Unavailable);
        }
        if self.runtime.platform_requests.len() == crate::MAX_PENDING_PLATFORM_REQUESTS {
            return Err(PlatformError::PendingQueueFull);
        }
        let (request, response) = PlatformRequest::application_save_path(options)?;
        self.runtime
            .event_proxy
            .send_event(RuntimeEvent::ExternalCommandsReady)
            .map_err(|_| PlatformError::Unavailable)?;
        self.runtime.platform_requests.push_back(request);
        Ok(response)
    }

    /// Close a queued or mounted window.
    ///
    /// Returns `false` when the handle is unknown or the application already exited.
    pub fn close_window(&mut self, handle: WindowHandle) -> bool {
        if !matches!(self.status, AppRunStatus::Continue) {
            return false;
        }
        let pending = self
            .runtime
            .pending_windows
            .iter()
            .position(|request| request.handle == handle);
        let mounted = self.runtime.window_handles.contains_key(&handle)
            || self.runtime.current_handle() == Some(handle);
        if pending.is_none() && !mounted {
            return false;
        }
        if self
            .runtime
            .event_proxy
            .send_event(RuntimeEvent::ExternalCommandsReady)
            .is_err()
        {
            return false;
        }
        if let Some(index) = pending {
            self.runtime.pending_windows.remove(index);
        } else if !self.runtime.close_requests.contains(&handle) {
            self.runtime.close_requests.push(handle);
        }
        true
    }

    /// Mark the root view dirty and request exactly one native redraw.
    ///
    /// Returns `false` only after exit or before the first pump has mounted the root window. State
    /// installed before that first pump is naturally read by the initial render.
    pub fn invalidate_root(&mut self) -> bool {
        self.invalidate_window(self.root_window)
    }

    pub const fn status(&self) -> AppRunStatus {
        self.status
    }
}

/// Configures a native application independently from its windows.
///
/// This is the core lifecycle used by language bindings and other embedders. Convert it into an
/// [`AppRunner`], pump once until [`AppRunner::is_ready`] is true, then queue the first window with
/// [`AppRunner::open_window`]. Ordinary Rust applications can continue to use [`App::new`], which
/// combines this lifecycle with an initial root view.
pub struct Application {
    pub(super) keymap: Keymap,
    pub(super) menus: Vec<Menu>,
    pub(super) globals: GlobalStore,
    pub(super) assets: Assets,
    pub(super) fonts: Vec<FontSource>,
    pub(super) application_callbacks: ApplicationCallbacks,
    pub(super) quit_mode: QuitMode,
}

impl Application {
    pub fn new() -> Self {
        Self {
            keymap: Keymap::default(),
            menus: Vec::new(),
            globals: GlobalStore::default(),
            assets: Assets::default(),
            fonts: Vec::new(),
            application_callbacks: ApplicationCallbacks::default(),
            quit_mode: QuitMode::Default,
        }
    }

    /// Configure when closing the final window terminates the application.
    pub fn quit_mode(mut self, mode: QuitMode) -> Self {
        self.quit_mode = mode;
        self
    }

    /// GPUI-compatible alias for [`Self::quit_mode`].
    pub fn with_quit_mode(self, mode: QuitMode) -> Self {
        self.quit_mode(mode)
    }

    /// Install the immutable application asset source used by every window.
    pub fn with_assets(mut self, source: impl crate::AssetSource) -> Self {
        self.assets = Assets::new(source);
        self
    }

    /// Install an already shared application asset handle.
    pub fn assets(mut self, assets: Assets) -> Self {
        self.assets = assets;
        self
    }

    /// Register one custom OpenType font file or asset path before launch.
    pub fn font(mut self, font: impl Into<FontSource>) -> Self {
        self.fonts.push(font.into());
        self
    }

    /// Register custom fonts in declaration order before launch.
    pub fn fonts(mut self, fonts: impl IntoIterator<Item = impl Into<FontSource>>) -> Self {
        self.fonts.extend(fonts.into_iter().map(Into::into));
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
    pub fn global<G: Global>(self, global: G) -> Self {
        self.globals.set(global);
        self
    }

    /// Handle URLs supplied by the operating system, including `file:` URLs.
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

    /// Handle a native keyboard-layout change.
    pub fn on_keyboard_layout_change(
        mut self,
        callback: impl FnMut(&KeyboardLayout, &mut EventContext) + 'static,
    ) -> Self {
        self.application_callbacks.keyboard_layout = Some(Box::new(callback));
        self
    }

    /// Handle activation of a delivered system notification or action button.
    pub fn on_system_notification_response(
        mut self,
        callback: impl FnMut(SystemNotificationResponse, &mut EventContext) + 'static,
    ) -> Self {
        self.application_callbacks.system_notification_response = Some(Box::new(callback));
        self
    }

    /// Handle a registered system-wide keyboard shortcut when it is pressed.
    pub fn on_global_shortcut(
        mut self,
        callback: impl FnMut(GlobalShortcutEvent, &mut EventContext) + 'static,
    ) -> Self {
        self.application_callbacks.global_shortcut = Some(Box::new(callback));
        self
    }

    /// Handle arguments and the working directory forwarded by a later process.
    pub fn on_second_instance(
        mut self,
        callback: impl FnMut(SecondInstanceEvent, &mut EventContext) + 'static,
    ) -> Self {
        self.application_callbacks.second_instance = Some(Box::new(callback));
        self
    }

    /// Handle native system suspend, resume, lock-screen, and unlock-screen events.
    pub fn on_power_event(
        mut self,
        callback: impl FnMut(PowerEvent, &mut EventContext) + 'static,
    ) -> Self {
        self.application_callbacks.power_event = Some(Box::new(callback));
        self
    }

    /// Handle clicks, scrolling, and native menu actions from application tray icons.
    pub fn on_tray_event(
        mut self,
        callback: impl FnMut(TrayEvent, &mut EventContext) + 'static,
    ) -> Self {
        self.application_callbacks.tray_event = Some(Box::new(callback));
        self
    }

    /// Run after a native window and its owned resources have been removed.
    pub fn on_window_closed(
        mut self,
        callback: impl FnMut(WindowHandle, &mut EventContext) + 'static,
    ) -> Self {
        self.application_callbacks.window_closed = Some(Box::new(callback));
        self
    }

    /// Convert this windowless application into an externally pumped native event loop.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn into_runner(self) -> Result<AppRunner, AppError> {
        let event_loop = EventLoop::with_user_event().build()?;
        event_loop.set_control_flow(ControlFlow::Wait);
        let root_window = WindowHandle::next();
        let runtime = Runtime::new(
            RuntimeStartup {
                initial_window: None,
                globals: self.globals,
                keymap: self.keymap,
                menus: self.menus,
                assets: self.assets,
                fonts: self.fonts,
                application_callbacks: self.application_callbacks,
                quit_mode: self.quit_mode,
            },
            event_loop.create_proxy(),
        )?;
        Ok(AppRunner {
            event_loop,
            runtime,
            root_window,
            root_window_pending: true,
            status: AppRunStatus::Continue,
        })
    }
}

impl Default for Application {
    fn default() -> Self {
        Self::new()
    }
}

/// Configures and runs one retained QuickGUI view.
pub struct App<V> {
    pub(super) view: V,
    pub(super) config: AppConfig,
    pub(super) keymap: Keymap,
    pub(super) menus: Vec<Menu>,
    pub(super) globals: GlobalStore,
    pub(super) assets: Assets,
    pub(super) fonts: Vec<FontSource>,
    pub(super) application_callbacks: ApplicationCallbacks,
    pub(super) quit_mode: QuitMode,
}

impl<V: View> App<V> {
    pub fn new(view: V) -> Self {
        Self {
            view,
            config: AppConfig::default(),
            keymap: Keymap::default(),
            menus: Vec::new(),
            globals: GlobalStore::default(),
            assets: Assets::default(),
            fonts: Vec::new(),
            application_callbacks: ApplicationCallbacks::default(),
            quit_mode: QuitMode::Default,
        }
    }

    /// Configure when closing the final window terminates the application.
    pub fn quit_mode(mut self, mode: QuitMode) -> Self {
        self.quit_mode = mode;
        self
    }

    /// GPUI-compatible alias for [`Self::quit_mode`].
    pub fn with_quit_mode(self, mode: QuitMode) -> Self {
        self.quit_mode(mode)
    }

    pub fn config(mut self, config: AppConfig) -> Self {
        self.config = config;
        self
    }

    /// Install the immutable application asset source used by every window and background image
    /// resource. Later registration replaces the previous source.
    pub fn with_assets(mut self, source: impl crate::AssetSource) -> Self {
        self.assets = Assets::new(source);
        self
    }

    /// Install an already shared application asset handle.
    pub fn assets(mut self, assets: Assets) -> Self {
        self.assets = assets;
        self
    }

    /// Register one custom OpenType font file or one path in the application asset source.
    ///
    /// `include_bytes!("Inter.ttf")` remains zero-copy. String values resolve through
    /// [`Self::with_assets`] once during startup. Counts, individual bytes, aggregate bytes, and
    /// collection faces are validated before any native window or renderer is created.
    pub fn font(mut self, font: impl Into<FontSource>) -> Self {
        self.fonts.push(font.into());
        self
    }

    /// Register custom fonts in declaration order.
    pub fn fonts(mut self, fonts: impl IntoIterator<Item = impl Into<FontSource>>) -> Self {
        self.fonts.extend(fonts.into_iter().map(Into::into));
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

    /// Select the display used for automatic root-window placement and fullscreen creation.
    pub fn display(mut self, display: DisplayId) -> Self {
        self.config.display_id = Some(display);
        self
    }

    pub fn without_display(mut self) -> Self {
        self.config.display_id = None;
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

    pub fn minimum_size(mut self, width: f32, height: f32) -> Self {
        self.config = self.config.minimum_size(width, height);
        self
    }

    pub fn without_minimum_size(mut self) -> Self {
        self.config = self.config.without_minimum_size();
        self
    }

    /// Represent a file in the root window's native document chrome.
    pub fn represented_file(mut self, path: impl Into<PathBuf>) -> Self {
        self.config.represented_file = Some(path.into());
        self
    }

    /// GPUI-compatible alias for [`Self::represented_file`].
    pub fn document_path(self, path: impl Into<PathBuf>) -> Self {
        self.represented_file(path)
    }

    pub fn without_represented_file(mut self) -> Self {
        self.config.represented_file = None;
        self
    }

    /// Set the root window's initial native unsaved-document indication.
    pub fn document_edited(mut self, edited: bool) -> Self {
        self.config.document_edited = edited;
        self
    }

    /// Opt the root window into native system tabbing.
    pub fn tabbing_identifier(mut self, identifier: impl Into<String>) -> Self {
        self.config.tabbing_identifier = Some(identifier.into());
        self
    }

    pub fn without_tabbing_identifier(mut self) -> Self {
        self.config.tabbing_identifier = None;
        self
    }

    pub fn performance_profile(mut self, profile: PerformanceProfile) -> Self {
        self.config.performance_profile = profile;
        self
    }

    /// Force the initial native light/dark appearance for the root window.
    pub fn window_appearance(mut self, appearance: WindowAppearance) -> Self {
        self.config.preferred_appearance = Some(appearance);
        self
    }

    /// Let the root window follow the operating system appearance.
    pub fn follow_system_appearance(mut self) -> Self {
        self.config.preferred_appearance = None;
        self
    }

    /// Configure how the native compositor treats transparent root-window pixels.
    pub fn window_background(mut self, appearance: WindowBackgroundAppearance) -> Self {
        self.config.window_background = appearance;
        self
    }

    pub fn title_bar_style(mut self, style: TitleBarStyle) -> Self {
        self.config.title_bar_style = style;
        self
    }

    pub fn window_kind(mut self, kind: WindowKind) -> Self {
        self.config = self.config.window_kind(kind);
        self
    }

    pub fn system_popover(mut self, popover: crate::PopoverOptions) -> Self {
        self.config = self.config.system_popover(popover);
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

    /// Open or suppress the retained-tree inspector for the root window.
    #[cfg(feature = "inspector")]
    pub fn inspector(mut self, inspector: bool) -> Self {
        self.config.inspector = inspector;
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

    /// Handle a native keyboard-layout change after QuickGUI installs the new command map.
    ///
    /// The callback is notification-driven and application-wide. Views that only need to render
    /// the layout name should prefer [`ViewContext::keyboard_layout`], which invalidates only the
    /// views that read it.
    pub fn on_keyboard_layout_change(
        mut self,
        callback: impl FnMut(&KeyboardLayout, &mut EventContext) + 'static,
    ) -> Self {
        self.application_callbacks.keyboard_layout = Some(Box::new(callback));
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

    /// Handle a registered system-wide keyboard shortcut when it is pressed.
    pub fn on_global_shortcut(
        mut self,
        callback: impl FnMut(GlobalShortcutEvent, &mut EventContext) + 'static,
    ) -> Self {
        self.application_callbacks.global_shortcut = Some(Box::new(callback));
        self
    }

    /// Handle arguments and the working directory forwarded by a later application process.
    pub fn on_second_instance(
        mut self,
        callback: impl FnMut(SecondInstanceEvent, &mut EventContext) + 'static,
    ) -> Self {
        self.application_callbacks.second_instance = Some(Box::new(callback));
        self
    }

    /// Handle native system suspend, resume, lock-screen, and unlock-screen events.
    pub fn on_power_event(
        mut self,
        callback: impl FnMut(PowerEvent, &mut EventContext) + 'static,
    ) -> Self {
        self.application_callbacks.power_event = Some(Box::new(callback));
        self
    }

    /// Handle clicks, scrolling, and native menu actions from application tray icons.
    pub fn on_tray_event(
        mut self,
        callback: impl FnMut(TrayEvent, &mut EventContext) + 'static,
    ) -> Self {
        self.application_callbacks.tray_event = Some(Box::new(callback));
        self
    }

    /// Run after a native window and its owned resources have been removed from the application.
    ///
    /// Owned child windows close first. The callback has no current window, so opening a window
    /// creates a new top-level window. Subsequent registration replaces the previous callback.
    pub fn on_window_closed(
        mut self,
        callback: impl FnMut(WindowHandle, &mut EventContext) + 'static,
    ) -> Self {
        self.application_callbacks.window_closed = Some(Box::new(callback));
        self
    }

    /// Convert this application into an externally pumped native event loop.
    ///
    /// This is intended for embedders which already own a language runtime on the platform main
    /// thread. It preserves QuickGUI's damage-driven scheduling: the caller chooses only the
    /// maximum time before control is yielded back to that runtime.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn into_runner(self) -> Result<AppRunner, AppError> {
        let event_loop = EventLoop::with_user_event().build()?;
        event_loop.set_control_flow(ControlFlow::Wait);
        let initial_window = WindowRequest::new(self.view, self.config);
        let root_window = initial_window.handle;
        let runtime = Runtime::new(
            RuntimeStartup {
                initial_window: Some(initial_window),
                globals: self.globals,
                keymap: self.keymap,
                menus: self.menus,
                assets: self.assets,
                fonts: self.fonts,
                application_callbacks: self.application_callbacks,
                quit_mode: self.quit_mode,
            },
            event_loop.create_proxy(),
        )?;
        Ok(AppRunner {
            event_loop,
            runtime,
            root_window,
            root_window_pending: false,
            status: AppRunStatus::Continue,
        })
    }

    pub fn run(self) -> Result<(), AppError> {
        #[cfg(not(target_arch = "wasm32"))]
        {
            let AppRunner {
                event_loop,
                mut runtime,
                ..
            } = self.into_runner()?;
            event_loop.run_app(&mut runtime)?;
            return match runtime.fatal_error.take() {
                Some(error) => Err(error),
                None => Ok(()),
            };
        }

        #[cfg(target_arch = "wasm32")]
        {
            let event_loop = EventLoop::with_user_event().build()?;
            event_loop.set_control_flow(ControlFlow::Wait);
            let mut runtime = Runtime::new(
                RuntimeStartup {
                    initial_window: Some(WindowRequest::new(self.view, self.config)),
                    globals: self.globals,
                    keymap: self.keymap,
                    menus: self.menus,
                    assets: self.assets,
                    fonts: self.fonts,
                    application_callbacks: self.application_callbacks,
                    quit_mode: self.quit_mode,
                },
                event_loop.create_proxy(),
            )?;
            event_loop.run_app(&mut runtime)?;
            match runtime.fatal_error.take() {
                Some(error) => Err(error),
                None => Ok(()),
            }
        }
    }
}
