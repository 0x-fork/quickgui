#[cfg(target_os = "macos")]
mod app {
    use std::{
        error::Error,
        fmt,
        io::{self, Write},
        mem::MaybeUninit,
        sync::{
            Arc,
            atomic::{AtomicBool, AtomicU8, Ordering},
        },
        time::{Duration, Instant},
    };

    use objc2::rc::Retained;
    use objc2_app_kit::{
        NSApplication, NSApplicationActivationOptions, NSEvent, NSEventModifierFlags, NSEventType,
        NSRunningApplication, NSTextField, NSWindow,
    };
    use objc2_foundation::{MainThreadMarker, NSPoint, NSRect, NSString};
    use quickgui::{
        App, AsyncContextError, AsyncViewContext, CONTEXT_MENU_SUBMENU_HOVER_DELAY, Color,
        ContextMenuLayout, ContextMenuState, ElementId, Event, EventContext, FocusHandle,
        FrameMetrics, IntoElement, MouseButton, PopupMenu, PopupMenuItem, PopupMenuItemState, Size,
        View, ViewContext, WindowKind, WindowState, div, native_view, popup_menu_key_bindings,
        text,
    };
    use serde::Serialize;

    const WARMUP_FRAMES: usize = 45;
    const RESIZE_COMMANDS: usize = 180;
    const SETTLE_DURATION: Duration = Duration::from_secs(1);
    const IDLE_DURATION: Duration = Duration::from_secs(2);
    const WATCHDOG_DURATION: Duration = Duration::from_secs(210);
    const TEXT_COLUMNS: usize = 4;
    const TEXT_ROWS_PER_COLUMN: usize = 20;
    const ROOT_PADDING: f64 = 16.0;
    const NATIVE_HEIGHT: f64 = 36.0;
    const NATIVE_OPACITY: f32 = 0.625;
    const NATIVE_GEOMETRY_TOLERANCE: f64 = 2.0;
    const NATIVE_OPACITY_TOLERANCE: f64 = 0.01;
    const WINDOW_SIZE_TOLERANCE: f32 = 2.0;
    const DEFAULT_MINIMUM_SIZE: Size = Size::new(320.0, 240.0);
    const RUNTIME_MINIMUM_SIZE: Size = Size::new(960.0, 640.0);
    const PRE_MINIMUM_SIZE: Size = Size::new(760.0, 520.0);
    const FINAL_WINDOW_SIZE: Size = Size::new(1_180.0, 720.0);
    const FRAMEWORK_FOCUS_ID: &str = "acceptance-mouse-target";
    const DOCUMENT_FILE_PATH: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/Cargo.toml");
    const NATIVE_EDITOR_FOCUS_COMMANDS: usize = 6;
    const FRAMEWORK_FOCUS_COMMANDS: usize = 7;
    const DOCUMENT_CHROME_SET_COMMANDS: usize = 8;
    const DOCUMENT_CHROME_CLEAR_COMMANDS: usize = 9;
    const CONTEXT_MENU_CYCLES: usize = 8;
    const CONTEXT_MENU_OUTSIDE_DISMISS_CYCLES: usize = 4;
    const CONTEXT_MENU_ESCAPE_DISMISS_CYCLES: usize = 4;
    const CONTEXT_MENU_DEACTIVATION_DISMISS_CYCLES: usize = 1;
    const CONTEXT_MENU_SOAK_CYCLES: usize = 128;
    const CONTEXT_MENU_SOAK_BASELINE_CYCLE: usize = 32;
    const CONTEXT_MENU_SOAK_REOPEN_DELAY: Duration = Duration::from_millis(50);
    const CONTEXT_MENU_SOAK_SETTLE_DURATION: Duration = Duration::from_millis(250);
    const MAX_CONTEXT_SOAK_RSS_GROWTH_MIB: f64 = 16.0;
    const MAX_CONTEXT_SOAK_FOOTPRINT_GROWTH_MIB: f64 = 24.0;
    const MOUSE_CARD_TOP: f32 = 360.0;
    const MOUSE_TARGET_X: f64 = 140.0;
    const MOUSE_TARGET_Y: f64 = 408.0;
    const MOUSE_DRAG_X: f64 = 164.0;
    const MOUSE_DRAG_Y: f64 = 416.0;
    const MOUSE_OUTSIDE_X: f64 = 500.0;
    const MOUSE_OUTSIDE_Y: f64 = 408.0;
    const MAX_MOUSE_TRACE_STEPS: usize = 32;
    const EXPECTED_MOUSE_TRACE: &[&str] = &[
        "root-down-capture-left",
        "card-down-capture-left",
        "target-down-capture-left",
        "target-down-left-prevent",
        "card-down-bubble-left",
        "root-down-bubble-left",
        "root-up-capture-left",
        "card-up-capture-left",
        "target-up-left",
        "card-up-bubble-left",
        "root-up-bubble-left",
        "card-down-out-left",
        "root-down-capture-left",
        "root-down-bubble-left",
        "card-up-out-left",
        "root-up-capture-left",
        "root-up-bubble-left",
        "root-down-capture-right",
        "card-down-capture-right",
        "target-down-capture-right",
        "target-down-right-stop",
        "root-up-capture-right",
        "card-up-capture-right",
        "card-up-bubble-right",
        "root-up-bubble-right",
    ];
    pub fn run() -> Result<(), Box<dyn Error>> {
        tracing_subscriber::fmt()
            .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
            .with_target(false)
            .compact()
            .init();

        let mtm = MainThreadMarker::new()
            .expect("the QuickGUI runtime and macOS acceptance probe require the main thread");
        let field = unsafe { NSTextField::initWithFrame(mtm.alloc(), NSRect::ZERO) };
        unsafe {
            field.setStringValue(&NSString::from_str("Retained native child"));
            field.setPlaceholderString(Some(&NSString::from_str(
                "QuickGUI resizes this AppKit child in lockstep",
            )));
        }

        let failed = Arc::new(AtomicBool::new(false));
        let lifecycle = Arc::new(AtomicU8::new(0));
        let callback_field = field.clone();
        let callback_lifecycle = Arc::clone(&lifecycle);

        App::new(MacAcceptanceView::new(field, Arc::clone(&failed)))
            .bind_keys(popup_menu_key_bindings())
            .title("QuickGUI — macOS 0.1 acceptance")
            .size(900.0, 640.0)
            .position(80.0, 80.0)
            .window_kind(WindowKind::Floating)
            .on_window_closed(move |_, _| {
                // SAFETY: QuickGUI invokes lifecycle callbacks on AppKit's application thread.
                let detached = unsafe { callback_field.superview().is_none() };
                if !detached {
                    // Child popup teardown is an application-wide close event too. The native
                    // field must remain attached until the owner window itself is destroyed.
                    return;
                }
                callback_lifecycle.store(1, Ordering::Relaxed);
                emit_line(
                    "QUICKGUI_LIFECYCLE_RESULT {\"native_child_detached\":true,\"passed\":true}",
                );
            })
            .run()?;

        if failed.load(Ordering::Relaxed) || lifecycle.load(Ordering::Relaxed) != 1 {
            return Err(Box::new(AcceptanceFailed));
        }
        Ok(())
    }

    #[derive(Debug)]
    struct AcceptanceFailed;

    impl fmt::Display for AcceptanceFailed {
        fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
            formatter.write_str("the QuickGUI macOS acceptance probe failed")
        }
    }

    impl Error for AcceptanceFailed {}

    #[derive(Default)]
    struct MouseAcceptanceState {
        recording: bool,
        total_pointer_moves: usize,
        script_pointer_moves: usize,
        script_button_events: usize,
        script_pointer_exits: usize,
        native_actions_scheduled: usize,
        dispatch_trace: Vec<&'static str>,
        trace_overflow: bool,
        down_click_count: usize,
        up_click_count: usize,
        hover_entries: usize,
        hover_exits: usize,
        move_events: usize,
        drag_move_events: usize,
        exit_events: usize,
        exit_without_pressed_button: bool,
        default_clicks: usize,
    }

    impl MouseAcceptanceState {
        fn reset_for_script(&mut self) {
            let total_pointer_moves = self.total_pointer_moves;
            *self = Self {
                recording: true,
                total_pointer_moves,
                ..Self::default()
            };
        }

        fn stop_recording(&mut self) {
            self.recording = false;
        }

        fn push_trace(&mut self, step: &'static str) {
            if !self.recording {
                return;
            }
            if self.dispatch_trace.len() < MAX_MOUSE_TRACE_STEPS {
                self.dispatch_trace.push(step);
            } else {
                self.trace_overflow = true;
            }
        }

        fn push_button_trace(
            &mut self,
            button: MouseButton,
            left: &'static str,
            right: &'static str,
        ) {
            self.push_trace(match button {
                MouseButton::Left => left,
                MouseButton::Right => right,
                _ => "unexpected-mouse-button",
            });
        }

        fn note_pointer_move(&mut self) {
            self.total_pointer_moves = self.total_pointer_moves.saturating_add(1);
            if self.recording {
                self.script_pointer_moves = self.script_pointer_moves.saturating_add(1);
            }
        }

        fn note_button_event(&mut self) {
            if self.recording {
                self.script_button_events = self.script_button_events.saturating_add(1);
            }
        }

        fn note_pointer_exit(&mut self) {
            if self.recording {
                self.script_pointer_exits = self.script_pointer_exits.saturating_add(1);
            }
        }
    }

    #[derive(Clone, Copy, Debug)]
    enum WindowProbeCommand {
        Resize(Size),
        SetMinimum(Size),
        ClearMinimum,
        FocusNativeEditor,
        FocusFrameworkControl,
        SetDocumentChrome,
        ClearDocumentChrome,
    }

    #[derive(Clone, Copy, Debug)]
    enum MouseScriptAction {
        MoveTarget,
        LeftDownTarget,
        LeftDragTarget,
        LeftUpTarget,
        MoveOutside,
        LeftDownOutside,
        LeftUpOutside,
        MoveTargetAgain,
        RightDownTarget,
        RightUpTarget,
        ExitTarget,
    }

    impl MouseScriptAction {
        const SCRIPT: [Self; 11] = [
            Self::MoveTarget,
            Self::LeftDownTarget,
            Self::LeftDragTarget,
            Self::LeftUpTarget,
            Self::MoveOutside,
            Self::LeftDownOutside,
            Self::LeftUpOutside,
            Self::MoveTargetAgain,
            Self::RightDownTarget,
            Self::RightUpTarget,
            Self::ExitTarget,
        ];

        fn signal(self) -> MouseSignal {
            match self {
                Self::MoveTarget
                | Self::LeftDragTarget
                | Self::MoveOutside
                | Self::MoveTargetAgain => MouseSignal::Move,
                Self::LeftDownTarget
                | Self::LeftUpTarget
                | Self::LeftDownOutside
                | Self::LeftUpOutside
                | Self::RightDownTarget
                | Self::RightUpTarget => MouseSignal::Button,
                Self::ExitTarget => MouseSignal::Exit,
            }
        }

        fn name(&self) -> &'static str {
            match self {
                Self::MoveTarget => "mouse-move-target",
                Self::LeftDownTarget => "mouse-left-down-target",
                Self::LeftDragTarget => "mouse-left-drag-target",
                Self::LeftUpTarget => "mouse-left-up-target",
                Self::MoveOutside => "mouse-move-outside",
                Self::LeftDownOutside => "mouse-left-down-outside",
                Self::LeftUpOutside => "mouse-left-up-outside",
                Self::MoveTargetAgain => "mouse-move-target-again",
                Self::RightDownTarget => "mouse-right-down-target",
                Self::RightUpTarget => "mouse-right-up-target",
                Self::ExitTarget => "mouse-exit-target",
            }
        }
    }

    #[derive(Clone, Copy, Debug)]
    enum MouseSignal {
        Move,
        Button,
        Exit,
    }

    #[derive(Clone, Copy, Debug)]
    struct MouseWait {
        signal: MouseSignal,
        baseline: usize,
    }

    impl MouseWait {
        fn new(action: MouseScriptAction, mouse: &MouseAcceptanceState) -> Self {
            let signal = action.signal();
            let baseline = match signal {
                MouseSignal::Move => mouse.script_pointer_moves,
                MouseSignal::Button => mouse.script_button_events,
                MouseSignal::Exit => mouse.script_pointer_exits,
            };
            Self { signal, baseline }
        }

        fn completed(self, mouse: &MouseAcceptanceState) -> bool {
            let current = match self.signal {
                MouseSignal::Move => mouse.script_pointer_moves,
                MouseSignal::Button => mouse.script_button_events,
                MouseSignal::Exit => mouse.script_pointer_exits,
            };
            current > self.baseline
        }
    }

    struct MacAcceptanceView {
        field: Retained<NSTextField>,
        paragraphs: Arc<[Arc<str>]>,
        native_visible: bool,
        mouse: MouseAcceptanceState,
        context_menu: ContextMenuState,
        context_interaction: ContextInteraction,
        context_cycle: usize,
        context_open_observed: bool,
        context_activation_scheduled: bool,
        context_submenu_hover_observed: bool,
        context_action_received: bool,
        context_close_observed: bool,
        context_inactive_teardown_observed: bool,
        probe: AcceptanceProbe,
        watchdog_started: bool,
    }

    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    struct ContextAcceptanceCommand;

    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    enum ContextInteraction {
        Command,
        OutsideDismiss,
        EscapeDismiss,
        SoakCommand,
        DeactivationDismiss,
    }

    impl ContextInteraction {
        const fn requires_action(self) -> bool {
            matches!(self, Self::Command | Self::SoakCommand)
        }
    }

    impl MacAcceptanceView {
        fn new(field: Retained<NSTextField>, failed: Arc<AtomicBool>) -> Self {
            let paragraphs = (0..TEXT_COLUMNS * TEXT_ROWS_PER_COLUMN)
                .map(|index| {
                    Arc::<str>::from(format!(
                        "Pane {index:02} · wrapped Unicode text stays aligned during every native resize · 你好 مرحبا"
                    ))
                })
                .collect::<Vec<_>>()
                .into();
            Self {
                field,
                paragraphs,
                native_visible: true,
                mouse: MouseAcceptanceState::default(),
                context_menu: ContextMenuState::new(),
                context_interaction: ContextInteraction::Command,
                context_cycle: 0,
                context_open_observed: false,
                context_activation_scheduled: false,
                context_submenu_hover_observed: false,
                context_action_received: false,
                context_close_observed: false,
                context_inactive_teardown_observed: false,
                probe: AcceptanceProbe::new(failed),
                watchdog_started: false,
            }
        }

        fn context_menu(view: &mut Self) -> &mut ContextMenuState {
            &mut view.context_menu
        }

        fn native_observation(&self, observe_document_chrome: bool) -> NativeObservation {
            let frame = self.field.frame();
            // QuickGUI inserts a rounded host between the content and its clipping host. The
            // clipping host owns the effective subtree alpha so the native child is composited
            // exactly once while retaining its own alpha value.
            let opacity = unsafe {
                self.field
                    .superview()
                    .and_then(|rounded| rounded.superview())
                    .map_or(0.0, |clip| clip.alphaValue())
            };
            let window = self.field.window();
            let content_minimum = window
                .as_ref()
                .map(|window| unsafe { window.contentMinSize() });
            let (represented_file_matches, represented_file_cleared, document_edited) =
                if observe_document_chrome {
                    window
                        .as_ref()
                        .map(|window| {
                            // SAFETY: The URL and its path are queried on AppKit's application
                            // thread while the retained native window remains alive.
                            let represented_url = unsafe { window.representedURL() };
                            let represented_file_cleared = represented_url.is_none();
                            let represented_file_matches = represented_url
                                .as_ref()
                                .and_then(|url| unsafe { url.path() })
                                .is_some_and(|path| path.to_string() == DOCUMENT_FILE_PATH);
                            (
                                represented_file_matches,
                                represented_file_cleared,
                                window.isDocumentEdited(),
                            )
                        })
                        .unwrap_or((false, false, false))
                } else {
                    (false, false, false)
                };
            NativeObservation {
                // SAFETY: View rendering is confined to AppKit's application thread.
                mounted: unsafe { self.field.superview().is_some() },
                // SAFETY: `currentEditor` is queried on the AppKit application thread while the
                // retained text field remains alive. A field editor exists only while this native
                // control owns text focus.
                editor_active: unsafe { self.field.currentEditor().is_some() },
                width: frame.size.width,
                height: frame.size.height,
                opacity,
                content_minimum_width: content_minimum.map_or(0.0, |size| size.width),
                content_minimum_height: content_minimum.map_or(0.0, |size| size.height),
                represented_file_matches,
                represented_file_cleared,
                document_edited,
            }
        }

        fn start_watchdog(&mut self, cx: &ViewContext<'_, Self>) {
            if self.watchdog_started {
                return;
            }
            self.watchdog_started = true;
            match cx.spawn(|task_cx: AsyncViewContext<Self>| async move {
                task_cx.sleep(WATCHDOG_DURATION).await?;
                task_cx
                    .update(|this, cx| {
                        if !this.probe.is_finished() {
                            this.probe.fail_timeout(
                                &this.mouse,
                                this.context_menu.is_open(),
                                this.context_activation_scheduled,
                            );
                            cx.exit();
                        }
                    })
                    .await?;
                Ok::<(), AsyncContextError>(())
            }) {
                Ok(task) => task.detach(),
                Err(error) => panic!("could not start the macOS acceptance watchdog: {error}"),
            }
        }

        fn schedule_resize(&mut self, cx: &ViewContext<'_, Self>, size: Size) {
            match cx.spawn(move |task_cx: AsyncViewContext<Self>| async move {
                task_cx
                    .update(move |this, cx| {
                        if let Err(error) = cx.resize_window(size) {
                            this.probe.fail_command(error.to_string());
                        }
                        cx.invalidate();
                    })
                    .await
            }) {
                Ok(task) => task.detach(),
                Err(error) => self.probe.fail_command(error.to_string()),
            }
        }

        fn schedule_window_probe_command(
            &mut self,
            cx: &ViewContext<'_, Self>,
            command: WindowProbeCommand,
        ) {
            match cx.spawn(move |task_cx: AsyncViewContext<Self>| async move {
                task_cx
                    .update(move |this, cx| {
                        let result: Result<(), String> = match command {
                            WindowProbeCommand::Resize(size) => {
                                cx.resize_window(size).map_err(|error| error.to_string())
                            }
                            WindowProbeCommand::SetMinimum(size) => cx
                                .set_window_minimum_size(size)
                                .map_err(|error| error.to_string()),
                            WindowProbeCommand::ClearMinimum => cx
                                .clear_window_minimum_size()
                                .map_err(|error| error.to_string()),
                            WindowProbeCommand::FocusNativeEditor => {
                                let focused = this.field.window().is_some_and(|window| {
                                    window.makeFirstResponder(Some(this.field.as_ref()))
                                });
                                focused.then_some(()).ok_or_else(|| {
                                    "AppKit rejected the native editor as first responder"
                                        .to_owned()
                                })
                            }
                            WindowProbeCommand::FocusFrameworkControl => {
                                cx.focus(FocusHandle::new(FRAMEWORK_FOCUS_ID));
                                Ok(())
                            }
                            WindowProbeCommand::SetDocumentChrome => cx
                                .set_represented_file(DOCUMENT_FILE_PATH)
                                .and_then(|()| cx.set_window_edited(true))
                                .map_err(|error| error.to_string()),
                            WindowProbeCommand::ClearDocumentChrome => cx
                                .clear_represented_file()
                                .and_then(|()| cx.set_window_edited(false))
                                .map_err(|error| error.to_string()),
                        };
                        this.probe.note_window_command_completed();
                        if let Err(error) = result {
                            this.probe.fail_command(error);
                        }
                        cx.invalidate();
                    })
                    .await
            }) {
                Ok(task) => task.detach(),
                Err(error) => self.probe.fail_command(error.to_string()),
            }
        }

        fn schedule_mouse_action(&mut self, cx: &ViewContext<'_, Self>, action: MouseScriptAction) {
            match cx.spawn(move |task_cx: AsyncViewContext<Self>| async move {
                task_cx
                    .update(move |this, cx| {
                        if let Err(error) = post_mouse_action(&this.field, action, 0) {
                            this.probe.fail_command(error);
                        }
                        cx.invalidate();
                    })
                    .await
            }) {
                Ok(task) => task.detach(),
                Err(error) => self.probe.fail_command(error.to_string()),
            }
        }

        fn schedule_context_interaction(&mut self, cx: &ViewContext<'_, Self>) {
            if self.context_menu.popup_window().is_none() {
                return;
            }
            self.context_activation_scheduled = true;
            let interaction = self.context_interaction;
            let cycle = self.context_cycle;
            match cx.spawn(move |task_cx: AsyncViewContext<Self>| async move {
                if interaction == ContextInteraction::SoakCommand {
                    task_cx.sleep(Duration::from_millis(10)).await?;
                    let root_result = task_cx
                        .update(|_this, _cx| {
                            post_popup_pointer("Context menu", 16.0, 20.0, true)
                        })
                        .await?;
                    if let Err(error) = root_result {
                        task_cx
                            .update(move |this, cx| {
                                this.probe.fail_command(error);
                                cx.invalidate();
                            })
                            .await?;
                        return Ok::<(), AsyncContextError>(());
                    }
                    task_cx.sleep(Duration::from_millis(20)).await?;
                    let mut result = task_cx
                        .update(|_this, _cx| {
                            post_popup_pointer("Context submenu", 16.0, 20.0, true)
                        })
                        .await?;
                    if result.is_err() {
                        task_cx.sleep(Duration::from_millis(100)).await?;
                        result = task_cx
                            .update(|_this, _cx| {
                                post_popup_pointer("Context submenu", 16.0, 20.0, true)
                            })
                            .await?;
                    }
                    task_cx
                        .update(move |this, cx| {
                            match result {
                                Ok(()) => this.context_submenu_hover_observed = true,
                                Err(error) => this.probe.fail_command(error),
                            }
                            cx.invalidate();
                        })
                        .await?;
                    return Ok::<(), AsyncContextError>(());
                }
                task_cx.sleep(Duration::from_millis(50)).await?;
                task_cx
                    .update(|this, cx| {
                        if let Err(error) = post_popup_pointer("Context menu", 16.0, 20.0, false) {
                            this.probe.fail_command(error);
                        }
                        cx.invalidate();
                    })
                    .await?;
                task_cx
                    .sleep(CONTEXT_MENU_SUBMENU_HOVER_DELAY + Duration::from_millis(100))
                    .await?;
                let activate = interaction.requires_action();
                let mut result = task_cx
                    .update(move |_this, _cx| {
                        post_popup_pointer("Context submenu", 16.0, 20.0, activate)
                    })
                    .await?;
                if result.is_err() {
                    // The child geometry exists before AppKit exposes the hidden-first-frame
                    // panel relation. Retry once at a later exact deadline; never poll or animate.
                    task_cx.sleep(Duration::from_millis(400)).await?;
                    result = task_cx
                        .update(move |_this, _cx| {
                            post_popup_pointer("Context submenu", 16.0, 20.0, activate)
                        })
                        .await?;
                }
                let interaction_completed = result.is_ok();
                task_cx
                    .update(move |this, cx| {
                        match result {
                            Ok(()) => this.context_submenu_hover_observed = true,
                            Err(error) => this.probe.fail_command(error),
                        }
                        cx.invalidate();
                    })
                    .await?;
                if interaction_completed {
                    match interaction {
                        ContextInteraction::Command => {}
                        ContextInteraction::SoakCommand => {
                            unreachable!("soak interactions use the immediate submenu path")
                        }
                        ContextInteraction::OutsideDismiss => {
                            let event_number_offset = (CONTEXT_MENU_CYCLES
                                .saturating_add(cycle)
                                .saturating_add(1)
                                .saturating_mul(16))
                                as isize;
                            task_cx.sleep(Duration::from_millis(50)).await?;
                            task_cx
                                .update(move |this, cx| {
                                    if let Err(error) = post_mouse_action(
                                        &this.field,
                                        MouseScriptAction::LeftDownTarget,
                                        event_number_offset,
                                    ) {
                                        this.probe.fail_command(error);
                                    }
                                    cx.invalidate();
                                })
                                .await?;
                            task_cx.sleep(Duration::from_millis(20)).await?;
                            task_cx
                                .update(move |this, cx| {
                                    if let Err(error) = post_mouse_action(
                                        &this.field,
                                        MouseScriptAction::LeftUpTarget,
                                        event_number_offset,
                                    ) {
                                        this.probe.fail_command(error);
                                    }
                                    cx.invalidate();
                                })
                                .await?;
                        }
                        ContextInteraction::EscapeDismiss => {
                            task_cx.sleep(Duration::from_millis(50)).await?;
                            task_cx
                                .update(|this, cx| {
                                    if let Err(error) = send_escape_to_key_popup() {
                                        this.probe.fail_command(error);
                                    }
                                    cx.invalidate();
                                })
                                .await?;
                        }
                        ContextInteraction::DeactivationDismiss => {
                            task_cx.sleep(Duration::from_millis(50)).await?;
                            task_cx
                                .update(|this, cx| {
                                    if let Err(error) = deactivate_acceptance_application() {
                                        this.probe.fail_command(error);
                                    }
                                    cx.invalidate();
                                })
                                .await?;
                            // One exact deadline lets AppKit emit the focus transition and Winit
                            // serialize popup teardown. This is deliberately not a polling loop.
                            task_cx.sleep(Duration::from_millis(750)).await?;
                            task_cx
                                .update(|this, cx| {
                                    let inactive = match acceptance_application_is_active() {
                                        Ok(active) => !active,
                                        Err(error) => {
                                            this.probe.fail_command(error);
                                            false
                                        }
                                    };
                                    let popup = this.popup_observation();
                                    let teardown = !this.context_menu.is_open()
                                        && popup.window_count() == 0
                                        && !popup.owner_key;
                                    this.context_inactive_teardown_observed |=
                                        inactive && teardown;
                                    if !inactive || !teardown {
                                        this.probe.fail_command(format!(
                                            "application deactivation observed inactive={inactive} context_open={} owner_key={} popup_windows={}",
                                            this.context_menu.is_open(),
                                            popup.owner_key,
                                            popup.window_count(),
                                        ));
                                    }
                                    cx.invalidate();
                                })
                                .await?;
                        }
                    }
                }
                Ok::<(), AsyncContextError>(())
            }) {
                Ok(task) => task.detach(),
                Err(error) => self.probe.fail_command(error.to_string()),
            }
        }

        fn schedule_context_reopen(
            &mut self,
            cx: &ViewContext<'_, Self>,
            interaction: ContextInteraction,
            cycle: usize,
        ) {
            self.context_interaction = interaction;
            self.context_cycle = cycle;
            self.context_open_observed = false;
            self.context_activation_scheduled = false;
            self.context_submenu_hover_observed = false;
            self.context_action_received = false;
            self.context_close_observed = false;
            self.context_inactive_teardown_observed = false;
            let sequence = match interaction {
                ContextInteraction::Command => cycle,
                ContextInteraction::OutsideDismiss => CONTEXT_MENU_CYCLES.saturating_add(cycle),
                ContextInteraction::EscapeDismiss => CONTEXT_MENU_CYCLES
                    .saturating_add(CONTEXT_MENU_OUTSIDE_DISMISS_CYCLES)
                    .saturating_add(cycle),
                ContextInteraction::SoakCommand => CONTEXT_MENU_CYCLES
                    .saturating_add(CONTEXT_MENU_OUTSIDE_DISMISS_CYCLES)
                    .saturating_add(CONTEXT_MENU_ESCAPE_DISMISS_CYCLES)
                    .saturating_add(cycle),
                ContextInteraction::DeactivationDismiss => CONTEXT_MENU_CYCLES
                    .saturating_add(CONTEXT_MENU_OUTSIDE_DISMISS_CYCLES)
                    .saturating_add(CONTEXT_MENU_ESCAPE_DISMISS_CYCLES)
                    .saturating_add(CONTEXT_MENU_SOAK_CYCLES)
                    .saturating_add(cycle),
            };
            let event_number_offset = (sequence.saturating_add(1).saturating_mul(16)) as isize;
            match cx.spawn(move |task_cx: AsyncViewContext<Self>| async move {
                if interaction == ContextInteraction::SoakCommand {
                    // Keep this a sustained resource-lifecycle workload rather than an impossible
                    // zero-think-time throughput loop. The exact cadence remains much faster than
                    // a person can repeatedly traverse and activate a nested menu.
                    task_cx.sleep(CONTEXT_MENU_SOAK_REOPEN_DELAY).await?;
                    task_cx
                        .update(move |this, cx| {
                            if let Err(error) =
                                send_owner_context_click(&this.field, event_number_offset)
                            {
                                this.probe.fail_command(error);
                            }
                            cx.invalidate();
                        })
                        .await?;
                    return Ok::<(), AsyncContextError>(());
                }
                task_cx
                    .update(move |this, cx| {
                        if let Err(error) = post_mouse_action(
                            &this.field,
                            MouseScriptAction::RightDownTarget,
                            event_number_offset,
                        ) {
                            this.probe.fail_command(error);
                        }
                        cx.invalidate();
                    })
                    .await?;
                task_cx.sleep(Duration::from_millis(20)).await?;
                task_cx
                    .update(move |this, cx| {
                        if let Err(error) = post_mouse_action(
                            &this.field,
                            MouseScriptAction::RightUpTarget,
                            event_number_offset,
                        ) {
                            this.probe.fail_command(error);
                        }
                        cx.invalidate();
                    })
                    .await?;
                Ok::<(), AsyncContextError>(())
            }) {
                Ok(task) => task.detach(),
                Err(error) => self.probe.fail_command(error.to_string()),
            }
        }

        fn popup_observation(&self) -> PopupObservation {
            let Some(owner) = self.field.window() else {
                return PopupObservation::default();
            };
            let owner_key = owner.isKeyWindow();
            let framework_first_responder = owner
                .firstResponder()
                .zip(owner.contentView())
                .is_some_and(|(responder, content)| {
                    Retained::as_ptr(&responder).cast::<()>()
                        == Retained::as_ptr(&content).cast::<()>()
                });
            let mut windows = Vec::new();
            collect_window_tree(owner, &mut windows);
            let mut root_windows = 0_usize;
            let mut submenu_windows = 0_usize;
            for window in windows {
                match window.title().to_string().as_str() {
                    "Context menu" => root_windows = root_windows.saturating_add(1),
                    "Context submenu" => submenu_windows = submenu_windows.saturating_add(1),
                    _ => {}
                }
            }
            PopupObservation {
                owner_key,
                framework_first_responder,
                root_windows,
                submenu_windows,
            }
        }

        fn request_exit(&mut self, cx: &ViewContext<'_, Self>) {
            match cx.spawn(|task_cx: AsyncViewContext<Self>| async move {
                task_cx.update(|_, cx| cx.exit()).await
            }) {
                Ok(task) => task.detach(),
                Err(error) => {
                    panic!("could not exit the completed macOS acceptance probe: {error}")
                }
            }
        }

        fn text_columns(&self) -> impl Iterator<Item = quickgui::Element> + '_ {
            (0..TEXT_COLUMNS).map(|column| {
                let start = column * TEXT_ROWS_PER_COLUMN;
                div().flex_1().min_w(0.0).flex_col().gap_1().children(
                    self.paragraphs[start..start + TEXT_ROWS_PER_COLUMN]
                        .iter()
                        .enumerate()
                        .map(|(row, paragraph)| {
                            let paragraph = text(Arc::clone(paragraph))
                                .h(24.0)
                                .flex_none()
                                .overflow_hidden()
                                .text_xs()
                                .text_color(Color::rgb8(184, 190, 202));
                            if row % 5 == 0 {
                                paragraph.line_clamp(1).text_ellipsis()
                            } else {
                                paragraph
                            }
                        }),
                )
            })
        }
    }

    fn post_mouse_action(
        field: &NSTextField,
        action: MouseScriptAction,
        event_number_offset: isize,
    ) -> Result<(), String> {
        let mtm = MainThreadMarker::new()
            .ok_or_else(|| "mouse acceptance action left the AppKit thread".to_owned())?;
        let window = field
            .window()
            .ok_or_else(|| "native acceptance child is not attached to a window".to_owned())?;
        let content = window
            .contentView()
            .ok_or_else(|| "acceptance window has no content view".to_owned())?;
        let content_height = content.bounds().size.height;
        let (event_type, x, y, click_count, pressure, event_number) = match action {
            MouseScriptAction::MoveTarget => (
                NSEventType::MouseMoved,
                MOUSE_TARGET_X,
                MOUSE_TARGET_Y,
                0,
                0.0,
                1,
            ),
            MouseScriptAction::LeftDownTarget => (
                NSEventType::LeftMouseDown,
                MOUSE_TARGET_X,
                MOUSE_TARGET_Y,
                2,
                1.0,
                2,
            ),
            MouseScriptAction::LeftDragTarget => (
                NSEventType::LeftMouseDragged,
                MOUSE_DRAG_X,
                MOUSE_DRAG_Y,
                2,
                1.0,
                3,
            ),
            MouseScriptAction::LeftUpTarget => (
                NSEventType::LeftMouseUp,
                MOUSE_DRAG_X,
                MOUSE_DRAG_Y,
                2,
                0.0,
                4,
            ),
            MouseScriptAction::MoveOutside => (
                NSEventType::MouseMoved,
                MOUSE_OUTSIDE_X,
                MOUSE_OUTSIDE_Y,
                0,
                0.0,
                5,
            ),
            MouseScriptAction::LeftDownOutside => (
                NSEventType::LeftMouseDown,
                MOUSE_OUTSIDE_X,
                MOUSE_OUTSIDE_Y,
                1,
                1.0,
                6,
            ),
            MouseScriptAction::LeftUpOutside => (
                NSEventType::LeftMouseUp,
                MOUSE_OUTSIDE_X,
                MOUSE_OUTSIDE_Y,
                1,
                0.0,
                7,
            ),
            MouseScriptAction::MoveTargetAgain => (
                NSEventType::MouseMoved,
                MOUSE_TARGET_X,
                MOUSE_TARGET_Y,
                0,
                0.0,
                8,
            ),
            MouseScriptAction::RightDownTarget => (
                NSEventType::RightMouseDown,
                MOUSE_TARGET_X,
                MOUSE_TARGET_Y,
                1,
                1.0,
                9,
            ),
            MouseScriptAction::RightUpTarget => (
                NSEventType::RightMouseUp,
                MOUSE_TARGET_X,
                MOUSE_TARGET_Y,
                1,
                0.0,
                10,
            ),
            MouseScriptAction::ExitTarget => (
                NSEventType::MouseMoved,
                MOUSE_TARGET_X,
                MOUSE_TARGET_Y,
                0,
                0.0,
                11,
            ),
        };
        let event_number: isize = event_number;
        let event_number = event_number.saturating_add(event_number_offset);
        let location = NSPoint::new(x, content_height - y);
        // SAFETY: The factory arguments are finite window-local coordinates, a live native
        // window number, and an event type whose button/click payload matches the action above.
        let event = unsafe {
            NSEvent::mouseEventWithType_location_modifierFlags_timestamp_windowNumber_context_eventNumber_clickCount_pressure(
                event_type,
                location,
                NSEventModifierFlags(0),
                0.0,
                window.windowNumber(),
                None,
                event_number,
                click_count,
                pressure,
            )
        }
        .ok_or_else(|| format!("AppKit refused synthetic acceptance event {action:?}"))?;

        if matches!(action, MouseScriptAction::ExitTarget) {
            // Winit's production `mouseExited:` handler ignores the event payload and queues the
            // ordinary CursorLeft event. Calling the responder directly avoids fabricating a
            // private AppKit tracking-area number while still exercising Winit and QuickGUI's
            // native exit path.
            unsafe { content.mouseExited(&event) };
        } else {
            NSApplication::sharedApplication(mtm).postEvent_atStart(&event, false);
        }
        Ok(())
    }

    fn send_owner_context_click(
        field: &NSTextField,
        event_number_offset: isize,
    ) -> Result<(), String> {
        let window = field
            .window()
            .ok_or_else(|| "native acceptance child is not attached to a window".to_owned())?;
        let content = window
            .contentView()
            .ok_or_else(|| "acceptance window has no content view".to_owned())?;
        let location = NSPoint::new(
            MOUSE_TARGET_X,
            content.bounds().size.height - MOUSE_TARGET_Y,
        );
        for (event_type, pressure, event_number) in [
            (NSEventType::RightMouseDown, 1.0, 9_isize),
            (NSEventType::RightMouseUp, 0.0, 10_isize),
        ] {
            // SAFETY: The retained owner and Winit content responder stay alive for the complete
            // synchronous click. The event uses finite window-local coordinates and a matching
            // secondary-button payload.
            let event = unsafe {
                NSEvent::mouseEventWithType_location_modifierFlags_timestamp_windowNumber_context_eventNumber_clickCount_pressure(
                    event_type,
                    location,
                    NSEventModifierFlags(0),
                    0.0,
                    window.windowNumber(),
                    None,
                    event_number.saturating_add(event_number_offset),
                    1,
                    pressure,
                )
            }
            .ok_or_else(|| format!("AppKit refused {event_type:?} for the acceptance owner"))?;
            // The repeated resource soak must not depend on whichever application happens to own
            // the shared AppKit event queue. This still crosses Winit's production responder and
            // QuickGUI's ordinary secondary-click route.
            unsafe {
                match event_type {
                    NSEventType::RightMouseDown => content.rightMouseDown(&event),
                    NSEventType::RightMouseUp => content.rightMouseUp(&event),
                    _ => unreachable!("owner context acceptance uses only secondary clicks"),
                }
            }
        }
        Ok(())
    }

    fn post_popup_pointer(title: &str, x: f64, y: f64, click: bool) -> Result<(), String> {
        let mtm = MainThreadMarker::new()
            .ok_or_else(|| "popup acceptance action left the AppKit thread".to_owned())?;
        let application = NSApplication::sharedApplication(mtm);
        let mut windows = Vec::new();
        for window in application.windows().iter_retained() {
            collect_window_tree(window, &mut windows);
        }
        let live_titles = windows
            .iter()
            .map(|window| window.title().to_string())
            .collect::<Vec<_>>();
        let target = windows
            .iter()
            .find(|window| window.title().to_string() == title)
            .or_else(|| {
                (title == "Context submenu")
                    .then(|| {
                        windows
                            .iter()
                            .filter(|window| window_parent_depth(window) >= 2)
                            .max_by_key(|window| window_parent_depth(window))
                    })
                    .flatten()
            })
            .ok_or_else(|| {
                format!(
                    "could not find live AppKit window titled {title:?}; live titles={live_titles:?}"
                )
            })?;
        post_pointer_to_window(target, title, x, y, click)
    }

    fn send_escape_to_key_popup() -> Result<(), String> {
        const MACOS_ESCAPE_KEY_CODE: u16 = 53;

        let mtm = MainThreadMarker::new()
            .ok_or_else(|| "popup Escape acceptance action left the AppKit thread".to_owned())?;
        let application = NSApplication::sharedApplication(mtm);
        let window = application
            .keyWindow()
            .ok_or_else(|| "no AppKit key window received popup Escape".to_owned())?;
        let title = window.title().to_string();
        if title != "Context menu" && title != "Context submenu" {
            return Err(format!(
                "popup Escape key window was {title:?}, expected the context-menu chain"
            ));
        }
        let content = window
            .contentView()
            .ok_or_else(|| format!("popup Escape key window {title:?} has no content view"))?;
        let characters = NSString::from_str("\u{1b}");
        // SAFETY: This is a retained, window-local keyboard event delivered synchronously to the
        // live Winit content responder on AppKit's application thread. Key code 53 is the stable
        // macOS virtual-key code for Escape, and both character strings match that key.
        let event = unsafe {
            NSEvent::keyEventWithType_location_modifierFlags_timestamp_windowNumber_context_characters_charactersIgnoringModifiers_isARepeat_keyCode(
                NSEventType::KeyDown,
                NSPoint::new(0.0, 0.0),
                NSEventModifierFlags(0),
                0.0,
                window.windowNumber(),
                None,
                &characters,
                &characters,
                false,
                MACOS_ESCAPE_KEY_CODE,
            )
        }
        .ok_or_else(|| format!("AppKit refused Escape for popup window {title:?}"))?;
        // Calling the responder is deterministic for nonactivating popup panels while preserving
        // Winit's production `keyDown:` translation and QuickGUI's normal KeyboardInput route.
        unsafe { content.keyDown(&event) };
        Ok(())
    }

    fn deactivate_acceptance_application() -> Result<(), String> {
        let mtm = MainThreadMarker::new().ok_or_else(|| {
            "application deactivation acceptance action left the AppKit thread".to_owned()
        })?;
        let application = NSApplication::sharedApplication(mtm);
        // SAFETY: Application activation state is queried on AppKit's main thread.
        if !unsafe { application.isActive() } {
            return Err("acceptance application was not active before its handoff".to_owned());
        }
        let finder_identifier = NSString::from_str("com.apple.finder");
        // SAFETY: Running applications are queried and activated from AppKit's main thread. Finder
        // is part of the logged-in macOS desktop required by this live gate.
        let finder = unsafe {
            NSRunningApplication::runningApplicationsWithBundleIdentifier(&finder_identifier)
        }
        .iter_retained()
        .next()
        .ok_or_else(|| "Finder was not running for the activation handoff".to_owned())?;
        // SAFETY: Both retained application objects are live on AppKit's main thread. Yielding
        // first and then activating the receiver is Apple's cooperative activation sequence.
        let activated = unsafe {
            let current = NSRunningApplication::currentApplication();
            application.yieldActivationToApplication(&finder);
            finder.activateFromApplication_options(&current, NSApplicationActivationOptions(0))
        };
        if !activated {
            return Err("Finder rejected the cooperative activation handoff".to_owned());
        }
        Ok(())
    }

    fn acceptance_application_is_active() -> Result<bool, String> {
        let mtm = MainThreadMarker::new().ok_or_else(|| {
            "application activation state was queried away from the AppKit thread".to_owned()
        })?;
        // SAFETY: AppKit application state is queried synchronously on its main thread.
        Ok(unsafe { NSApplication::sharedApplication(mtm).isActive() })
    }

    fn collect_window_tree(window: Retained<NSWindow>, windows: &mut Vec<Retained<NSWindow>>) {
        let children = unsafe { window.childWindows() };
        windows.push(window);
        if let Some(children) = children {
            for child in children.iter_retained() {
                collect_window_tree(child, windows);
            }
        }
    }

    fn window_parent_depth(window: &NSWindow) -> usize {
        let mut depth = 0_usize;
        let mut parent = unsafe { window.parentWindow() };
        while let Some(window) = parent {
            depth = depth.saturating_add(1);
            parent = unsafe { window.parentWindow() };
        }
        depth
    }

    fn post_pointer_to_window(
        window: &NSWindow,
        title: &str,
        x: f64,
        y: f64,
        click: bool,
    ) -> Result<(), String> {
        let content = window
            .contentView()
            .ok_or_else(|| format!("popup window {title:?} has no content view"))?;
        let location = NSPoint::new(x, content.bounds().size.height - y);
        let events: &[(NSEventType, isize, f32, isize)] = if click {
            &[
                (NSEventType::MouseMoved, 0, 0.0, 101),
                (NSEventType::LeftMouseDown, 1, 1.0, 102),
                (NSEventType::LeftMouseUp, 1, 0.0, 103),
            ]
        } else {
            &[(NSEventType::MouseMoved, 0, 0.0, 100)]
        };
        for (event_type, click_count, pressure, event_number) in events.iter().copied() {
            // SAFETY: Coordinates are finite and window-local, and the retained window stays
            // alive for the complete event construction/posting loop.
            let event = unsafe {
                NSEvent::mouseEventWithType_location_modifierFlags_timestamp_windowNumber_context_eventNumber_clickCount_pressure(
                    event_type,
                    location,
                    NSEventModifierFlags(0),
                    0.0,
                    window.windowNumber(),
                    None,
                    event_number,
                    click_count,
                    pressure,
                )
            }
            .ok_or_else(|| {
                format!("AppKit refused {event_type:?} for popup window {title:?}")
            })?;
            // Nonactivating popup panels are not reliably selected by the application's shared
            // event queue. Invoke the live Winit content responder exactly as AppKit would so all
            // three events reach one retained popup even while key-window state is transitioning.
            unsafe {
                match event_type {
                    NSEventType::MouseMoved => content.mouseMoved(&event),
                    NSEventType::LeftMouseDown => content.mouseDown(&event),
                    NSEventType::LeftMouseUp => content.mouseUp(&event),
                    _ => unreachable!("popup acceptance uses only move and primary click events"),
                }
            }
        }
        Ok(())
    }

    impl View for MacAcceptanceView {
        fn event(&mut self, event: &Event, cx: &mut EventContext) {
            match event {
                Event::PointerMoved(_) => {
                    self.mouse.note_pointer_move();
                    cx.invalidate();
                }
                Event::MouseButton { .. } => {
                    self.mouse.note_button_event();
                    cx.invalidate();
                }
                Event::PointerLeft => {
                    self.mouse.note_pointer_exit();
                    cx.invalidate();
                }
                Event::Click(id) if *id == ElementId::named("acceptance-mouse-target") => {
                    if self.mouse.recording {
                        self.mouse.default_clicks = self.mouse.default_clicks.saturating_add(1);
                    }
                    cx.invalidate();
                }
                _ => {}
            }
        }

        fn render(&mut self, cx: &mut ViewContext<'_, Self>) -> impl IntoElement {
            self.start_watchdog(cx);
            if self.context_activation_scheduled
                && self.context_open_observed
                && (!self.context_interaction.requires_action() || self.context_action_received)
                && !self.context_menu.is_open()
            {
                self.context_close_observed = true;
            }
            if self.context_menu.is_open() {
                self.context_open_observed = true;
                if !self.context_activation_scheduled {
                    self.schedule_context_interaction(cx);
                }
            }
            let context_action = cx.action_listener(
                "acceptance-root",
                |this, _: &ContextAcceptanceCommand, cx| {
                    this.context_action_received = true;
                    cx.invalidate();
                },
            );
            let root_down_capture = cx.mouse_down_listener("acceptance-root", |this, event, cx| {
                this.mouse.push_button_trace(
                    event.button,
                    "root-down-capture-left",
                    "root-down-capture-right",
                );
                cx.invalidate();
            });
            let root_down_bubble = cx.mouse_down_listener("acceptance-root", |this, event, cx| {
                this.mouse.push_button_trace(
                    event.button,
                    "root-down-bubble-left",
                    "root-down-bubble-right",
                );
                cx.invalidate();
            });
            let root_up_capture = cx.mouse_up_listener("acceptance-root", |this, event, cx| {
                this.mouse.push_button_trace(
                    event.button,
                    "root-up-capture-left",
                    "root-up-capture-right",
                );
                cx.invalidate();
            });
            let root_up_bubble = cx.mouse_up_listener("acceptance-root", |this, event, cx| {
                this.mouse.push_button_trace(
                    event.button,
                    "root-up-bubble-left",
                    "root-up-bubble-right",
                );
                cx.invalidate();
            });
            let card_down_out =
                cx.mouse_down_listener("acceptance-mouse-card", |this, event, cx| {
                    this.mouse.push_button_trace(
                        event.button,
                        "card-down-out-left",
                        "card-down-out-right",
                    );
                    cx.invalidate();
                });
            let card_down_capture =
                cx.mouse_down_listener("acceptance-mouse-card", |this, event, cx| {
                    this.mouse.push_button_trace(
                        event.button,
                        "card-down-capture-left",
                        "card-down-capture-right",
                    );
                    cx.invalidate();
                });
            let card_down_bubble =
                cx.mouse_down_listener("acceptance-mouse-card", |this, event, cx| {
                    this.mouse.push_button_trace(
                        event.button,
                        "card-down-bubble-left",
                        "card-down-bubble-right",
                    );
                    cx.invalidate();
                });
            let card_up_out = cx.mouse_up_listener("acceptance-mouse-card", |this, event, cx| {
                this.mouse
                    .push_button_trace(event.button, "card-up-out-left", "card-up-out-right");
                cx.invalidate();
            });
            let card_up_capture =
                cx.mouse_up_listener("acceptance-mouse-card", |this, event, cx| {
                    this.mouse.push_button_trace(
                        event.button,
                        "card-up-capture-left",
                        "card-up-capture-right",
                    );
                    cx.invalidate();
                });
            let card_up_bubble =
                cx.mouse_up_listener("acceptance-mouse-card", |this, event, cx| {
                    this.mouse.push_button_trace(
                        event.button,
                        "card-up-bubble-left",
                        "card-up-bubble-right",
                    );
                    cx.invalidate();
                });
            let target_down_capture =
                cx.mouse_down_listener("acceptance-mouse-target", |this, event, cx| {
                    this.mouse.push_button_trace(
                        event.button,
                        "target-down-capture-left",
                        "target-down-capture-right",
                    );
                    cx.invalidate();
                });
            let target_left_down =
                cx.mouse_down_listener("acceptance-mouse-target", |this, event, cx| {
                    if this.mouse.recording {
                        this.mouse.down_click_count = event.click_count;
                    }
                    this.mouse.push_trace("target-down-left-prevent");
                    cx.prevent_default();
                    cx.invalidate();
                });
            let target_right_down =
                cx.mouse_down_listener("acceptance-mouse-target", |this, _event, cx| {
                    this.mouse.push_trace("target-down-right-stop");
                    // The earlier outside-click probe intentionally blurs retained focus. Give
                    // every context-menu cycle an exact owner focus to preserve so the popup gate
                    // cannot pass by merely returning native key-window focus.
                    cx.focus(FocusHandle::new(FRAMEWORK_FOCUS_ID));
                    cx.stop_propagation();
                    cx.invalidate();
                });
            let target_left_up =
                cx.mouse_up_listener("acceptance-mouse-target", |this, event, cx| {
                    if this.mouse.recording {
                        this.mouse.up_click_count = event.click_count;
                    }
                    this.mouse.push_trace("target-up-left");
                    cx.invalidate();
                });
            let target_move =
                cx.mouse_move_listener("acceptance-mouse-target", |this, event, cx| {
                    if this.mouse.recording {
                        this.mouse.move_events = this.mouse.move_events.saturating_add(1);
                        if event.dragging_button(MouseButton::Left) {
                            this.mouse.drag_move_events =
                                this.mouse.drag_move_events.saturating_add(1);
                        }
                    }
                    cx.invalidate();
                });
            let target_exit =
                cx.mouse_exit_listener("acceptance-mouse-target", |this, event, cx| {
                    if this.mouse.recording {
                        this.mouse.exit_events = this.mouse.exit_events.saturating_add(1);
                        this.mouse.exit_without_pressed_button |= event.pressed_button.is_none();
                    }
                    cx.invalidate();
                });
            let target_hover = cx.hover_listener("acceptance-mouse-target", |this, hovered, cx| {
                if this.mouse.recording {
                    if *hovered {
                        this.mouse.hover_entries = this.mouse.hover_entries.saturating_add(1);
                    } else {
                        this.mouse.hover_exits = this.mouse.hover_exits.saturating_add(1);
                    }
                }
                cx.invalidate();
            });

            let observation = self.native_observation(self.probe.observes_document_chrome());
            let window_state = cx.window_state();
            let framework_focus_active = cx.is_focused(cx.focus_handle(FRAMEWORK_FOCUS_ID));
            let popup = if self.probe.observes_context_menu() {
                self.popup_observation()
            } else {
                PopupObservation::default()
            };
            let action = self.probe.observe(
                ProbeObservation {
                    metrics: cx.metrics(),
                    now: Instant::now(),
                    viewport: cx.size(),
                    native_visible: self.native_visible,
                    native: observation,
                    window_state,
                    framework_focus_active,
                    popup,
                    context_menu_open_observed: self.context_open_observed,
                    context_submenu_hover_observed: self.context_submenu_hover_observed,
                    context_action_received: self.context_action_received,
                    context_menu_closed: self.context_close_observed,
                    context_inactive_teardown_observed: self.context_inactive_teardown_observed,
                },
                &mut self.mouse,
            );
            match action {
                ProbeAction::Animate => cx.request_animation_frame(),
                ProbeAction::Resize {
                    size,
                    native_visible,
                } => {
                    self.native_visible = native_visible;
                    self.schedule_resize(cx, size);
                    cx.request_animation_frame();
                }
                ProbeAction::Window(command) => self.schedule_window_probe_command(cx, command),
                ProbeAction::Mouse(action) => self.schedule_mouse_action(cx, action),
                ProbeAction::ReopenContext { interaction, cycle } => {
                    self.schedule_context_reopen(cx, interaction, cycle)
                }
                ProbeAction::Observe => {}
                ProbeAction::Wait(deadline) => cx.request_repaint_at(deadline),
                ProbeAction::Exit => self.request_exit(cx),
            }

            let native_row = if self.native_visible {
                native_view(self.field.as_ref())
                    .id("acceptance-native-field")
                    .w_full()
                    .h(NATIVE_HEIGHT as f32)
                    .flex_none()
                    .rounded_md()
                    .opacity(NATIVE_OPACITY)
            } else {
                div()
                    .w_full()
                    .h(NATIVE_HEIGHT as f32)
                    .flex_none()
                    .rounded_md()
                    .border(1.0, Color::rgb8(77, 83, 96))
                    .child("Native child intentionally detached")
            };

            let mouse_target = div()
                .id("acceptance-mouse-target")
                .size_full()
                .flex_row()
                .items_center()
                .justify_center()
                .rounded_md()
                .bg(Color::rgb8(45, 82, 170))
                .clickable()
                .block_pointer()
                .capture_any_mouse_down(target_down_capture)
                .on_mouse_down(MouseButton::Left, target_left_down)
                .on_mouse_down(MouseButton::Right, target_right_down)
                .on_mouse_up(MouseButton::Left, target_left_up)
                .on_mouse_move(target_move)
                .on_mouse_exit(target_exit)
                .on_hover(target_hover)
                .child(text("Native mouse gate").text_xs().font_semibold());
            let mouse_target = self.context_menu.element(
                cx,
                "acceptance-mouse-target",
                Self::context_menu,
                mouse_target,
                ContextMenuLayout::new(176.0, 32.0).vertical_padding(4.0),
                |_view, _event| {
                    let submenu = PopupMenu::new([PopupMenuItem::action(
                        "accept-context-command",
                        "Accept context command",
                        ContextAcceptanceCommand,
                    )])
                    .expect("the static acceptance context submenu is valid");
                    Some(
                        PopupMenu::new([PopupMenuItem::submenu(
                            "accept-context-submenu",
                            "Hover submenu",
                            submenu,
                        )])
                        .expect("the static acceptance context menu is valid"),
                    )
                },
                || {
                    div()
                        .p_1()
                        .flex_col()
                        .bg(Color::rgb8(27, 31, 40))
                        .border(1.0, Color::rgb8(81, 91, 112))
                },
                |item, state: PopupMenuItemState| {
                    div()
                        .w_full()
                        .px_2()
                        .flex_row()
                        .items_center()
                        .bg(if state.highlighted {
                            Color::rgb8(45, 82, 170)
                        } else {
                            Color::TRANSPARENT
                        })
                        .child(text(item.label().clone()).text_xs())
                },
            );

            let mouse_card = div()
                .id("acceptance-mouse-card")
                .absolute()
                .left(20.0)
                .top(MOUSE_CARD_TOP)
                .w(240.0)
                .h(96.0)
                .p_2()
                .z_index(100)
                .rounded_lg()
                .bg(Color::rgb8(27, 31, 40))
                .border(1.0, Color::rgb8(81, 91, 112))
                .on_mouse_down_out(card_down_out)
                .capture_any_mouse_down(card_down_capture)
                .on_any_mouse_down(card_down_bubble)
                .on_mouse_up_out(MouseButton::Left, card_up_out)
                .capture_any_mouse_up(card_up_capture)
                .on_any_mouse_up(card_up_bubble)
                .child(mouse_target);

            div()
                .id("acceptance-root")
                .focus_scope(cx.focus_handle("acceptance-root"))
                .on_action(context_action)
                .size_full()
                .relative()
                .flex_col()
                .gap_2()
                .p_4()
                .overflow_hidden()
                .bg(Color::rgb8(18, 19, 23))
                .text_color(Color::rgb8(235, 237, 242))
                .capture_any_mouse_down(root_down_capture)
                .on_any_mouse_down(root_down_bubble)
                .capture_any_mouse_up(root_up_capture)
                .on_any_mouse_up(root_up_bubble)
                .child(
                    text("macOS resize · text · NSView · mouse acceptance")
                        .w_full()
                        .text_center()
                        .text_lg()
                        .font_semibold(),
                )
                .child(native_row)
                .child(
                    text("64 wrapped + 16 ellipsized areas · 180 resizes · constraints · mouse")
                        .text_xs()
                        .text_color(Color::rgb8(94, 234, 212)),
                )
                .child(
                    div()
                        .flex_1()
                        .min_h(0.0)
                        .flex_row()
                        .gap_2()
                        .overflow_hidden()
                        .children(self.text_columns()),
                )
                .child(mouse_card)
        }
    }

    #[derive(Clone, Copy)]
    struct NativeObservation {
        mounted: bool,
        editor_active: bool,
        width: f64,
        height: f64,
        opacity: f64,
        content_minimum_width: f64,
        content_minimum_height: f64,
        represented_file_matches: bool,
        represented_file_cleared: bool,
        document_edited: bool,
    }

    #[derive(Clone, Copy, Default)]
    struct PopupObservation {
        owner_key: bool,
        framework_first_responder: bool,
        root_windows: usize,
        submenu_windows: usize,
    }

    impl PopupObservation {
        const fn window_count(self) -> usize {
            self.root_windows.saturating_add(self.submenu_windows)
        }
    }

    #[derive(Clone, Copy, Debug, Default)]
    struct ProcessMemorySample {
        resident_bytes: u64,
        physical_footprint_bytes: u64,
    }

    impl ProcessMemorySample {
        const MIB: f64 = 1024.0 * 1024.0;

        fn resident_mib(self) -> f64 {
            self.resident_bytes as f64 / Self::MIB
        }

        fn physical_footprint_mib(self) -> f64 {
            self.physical_footprint_bytes as f64 / Self::MIB
        }
    }

    fn current_process_memory() -> Result<ProcessMemorySample, String> {
        let mut usage = MaybeUninit::<libc::rusage_info_v0>::zeroed();
        // SAFETY: `usage` points to writable storage for the exact V0 flavor, the current process
        // ID is valid, and the value is read only after libproc reports success.
        let result = unsafe {
            libc::proc_pid_rusage(
                libc::getpid(),
                libc::RUSAGE_INFO_V0,
                usage.as_mut_ptr() as _,
            )
        };
        if result != 0 {
            return Err(format!(
                "proc_pid_rusage failed with OS error {}",
                io::Error::last_os_error()
            ));
        }
        // SAFETY: A zero return from `proc_pid_rusage` initializes the requested V0 structure.
        let usage = unsafe { usage.assume_init() };
        Ok(ProcessMemorySample {
            resident_bytes: usage.ri_resident_size,
            physical_footprint_bytes: usage.ri_phys_footprint,
        })
    }

    #[derive(Clone, Copy)]
    struct ProbeObservation {
        metrics: FrameMetrics,
        now: Instant,
        viewport: Size,
        native_visible: bool,
        native: NativeObservation,
        window_state: WindowState,
        framework_focus_active: bool,
        popup: PopupObservation,
        context_menu_open_observed: bool,
        context_submenu_hover_observed: bool,
        context_action_received: bool,
        context_menu_closed: bool,
        context_inactive_teardown_observed: bool,
    }

    #[derive(Clone, Copy, Debug, Serialize)]
    struct AcceptanceBudgets {
        min_presentation_hz: f64,
        max_main_thread_cpu_percent: f64,
        max_p95_frame_cpu_ms: f64,
        max_frame_cpu_ms: f64,
        max_visible_text_areas: usize,
        max_retained_text_areas: usize,
        max_retained_text_layouts: usize,
        max_retained_text_renderers: usize,
        max_draw_calls: usize,
        max_idle_extra_frames: u64,
        max_context_soak_rss_growth_mib: f64,
        max_context_soak_footprint_growth_mib: f64,
    }

    impl Default for AcceptanceBudgets {
        fn default() -> Self {
            Self {
                min_presentation_hz: 45.0,
                max_main_thread_cpu_percent: 35.0,
                max_p95_frame_cpu_ms: 10.0,
                max_frame_cpu_ms: 25.0,
                max_visible_text_areas: 96,
                max_retained_text_areas: 256,
                max_retained_text_layouts: 256,
                max_retained_text_renderers: 32,
                max_draw_calls: 8,
                max_idle_extra_frames: 0,
                max_context_soak_rss_growth_mib: MAX_CONTEXT_SOAK_RSS_GROWTH_MIB,
                max_context_soak_footprint_growth_mib: MAX_CONTEXT_SOAK_FOOTPRINT_GROWTH_MIB,
            }
        }
    }

    #[derive(Debug, Serialize)]
    struct AcceptanceReport {
        schema_version: u32,
        resize_commands: usize,
        presented_frames: usize,
        elapsed_ms: f64,
        presentation_hz: f64,
        main_thread_cpu_percent: f64,
        average_frame_cpu_ms: f64,
        p95_frame_cpu_ms: f64,
        max_frame_cpu_ms: f64,
        average_frame_submission_ms: f64,
        p95_frame_submission_ms: f64,
        max_frame_submission_ms: f64,
        min_viewport_width: f32,
        max_viewport_width: f32,
        min_viewport_height: f32,
        max_viewport_height: f32,
        max_visible_text_areas: usize,
        max_reshaped_text_areas: usize,
        max_retained_text_areas: usize,
        max_retained_text_layouts: usize,
        max_retained_text_renderers: usize,
        max_draw_calls: usize,
        initial_minimum_size_observed: bool,
        pre_minimum_small_size_observed: bool,
        runtime_minimum_size_observed: bool,
        runtime_minimum_growth_observed: bool,
        native_minimum_size_observed: bool,
        minimum_size_clear_observed: bool,
        native_minimum_clear_observed: bool,
        post_clear_small_size_observed: bool,
        final_size_restored: bool,
        native_editor_focus_observed: bool,
        framework_focus_restored: bool,
        represented_file_observed: bool,
        document_edited_observed: bool,
        represented_file_clear_observed: bool,
        document_edited_clear_observed: bool,
        native_mouse_actions: usize,
        mouse_dispatch_trace: Vec<&'static str>,
        mouse_trace_overflow: bool,
        mouse_down_click_count: usize,
        mouse_up_click_count: usize,
        mouse_hover_entries: usize,
        mouse_hover_exits: usize,
        mouse_move_events: usize,
        mouse_drag_move_events: usize,
        mouse_exit_events: usize,
        mouse_exit_without_pressed_button: bool,
        mouse_default_clicks: usize,
        mouse_pointer_events: usize,
        mouse_button_events: usize,
        mouse_pointer_exits: usize,
        context_menu_open_observed: bool,
        context_submenu_hover_observed: bool,
        context_menu_action_received: bool,
        context_menu_close_observed: bool,
        context_menu_cycles_completed: usize,
        context_menu_focus_restored_cycles: usize,
        context_menu_teardown_cycles: usize,
        context_menu_outside_dismiss_cycles_completed: usize,
        context_menu_outside_focus_restored_cycles: usize,
        context_menu_outside_teardown_cycles: usize,
        context_menu_escape_dismiss_cycles_completed: usize,
        context_menu_escape_focus_restored_cycles: usize,
        context_menu_escape_teardown_cycles: usize,
        context_menu_soak_cycles_completed: usize,
        context_menu_soak_baseline_cycle: usize,
        context_menu_soak_reopen_delay_ms: u64,
        context_menu_soak_baseline_rss_mib: f64,
        context_menu_soak_final_rss_mib: f64,
        context_menu_soak_rss_growth_mib: f64,
        context_menu_soak_baseline_footprint_mib: f64,
        context_menu_soak_final_footprint_mib: f64,
        context_menu_soak_footprint_growth_mib: f64,
        context_menu_deactivation_dismiss_cycles_completed: usize,
        context_menu_deactivation_no_focus_steal_cycles: usize,
        context_menu_deactivation_teardown_cycles: usize,
        max_context_popup_windows: usize,
        initial_native_mount: bool,
        native_detach_observed: bool,
        native_remount_observed: bool,
        final_native_mounted: bool,
        final_native_width: f64,
        expected_native_width: f64,
        final_native_height: f64,
        final_native_opacity: f64,
        expected_native_opacity: f64,
        idle_frame_delta: u64,
        idle_extra_frames: u64,
        budgets: AcceptanceBudgets,
        passed: bool,
        failures: Vec<String>,
    }

    struct ResizeSamples {
        started_at: Instant,
        finished_at: Option<Instant>,
        cpu_ms: Vec<f64>,
        submission_ms: Vec<f64>,
        min_viewport: Size,
        max_viewport: Size,
        max_visible_text_areas: usize,
        max_reshaped_text_areas: usize,
        max_retained_text_areas: usize,
        max_retained_text_layouts: usize,
        max_retained_text_renderers: usize,
        max_draw_calls: usize,
    }

    impl ResizeSamples {
        fn new(now: Instant) -> Self {
            Self {
                started_at: now,
                finished_at: None,
                cpu_ms: Vec::with_capacity(RESIZE_COMMANDS),
                submission_ms: Vec::with_capacity(RESIZE_COMMANDS),
                min_viewport: Size::new(f32::MAX, f32::MAX),
                max_viewport: Size::ZERO,
                max_visible_text_areas: 0,
                max_reshaped_text_areas: 0,
                max_retained_text_areas: 0,
                max_retained_text_layouts: 0,
                max_retained_text_renderers: 0,
                max_draw_calls: 0,
            }
        }

        fn push(&mut self, metrics: FrameMetrics, viewport: Size) {
            self.cpu_ms.push(metrics.cpu_milliseconds());
            self.submission_ms.push(metrics.frame_milliseconds());
            self.min_viewport.width = self.min_viewport.width.min(viewport.width);
            self.min_viewport.height = self.min_viewport.height.min(viewport.height);
            self.max_viewport.width = self.max_viewport.width.max(viewport.width);
            self.max_viewport.height = self.max_viewport.height.max(viewport.height);
            self.max_visible_text_areas =
                self.max_visible_text_areas.max(metrics.render.text_areas);
            self.max_reshaped_text_areas = self
                .max_reshaped_text_areas
                .max(metrics.render.reshaped_text_areas);
            self.max_retained_text_areas = self
                .max_retained_text_areas
                .max(metrics.render.retained_text_areas);
            self.max_retained_text_layouts = self
                .max_retained_text_layouts
                .max(metrics.render.retained_text_layouts);
            self.max_retained_text_renderers = self
                .max_retained_text_renderers
                .max(metrics.render.retained_text_renderers);
            self.max_draw_calls = self.max_draw_calls.max(metrics.render.draw_calls);
        }
    }

    enum ProbePhase {
        Warmup {
            observed: usize,
        },
        Resizing {
            commands: usize,
            samples: ResizeSamples,
        },
        Constraints {
            stage: ConstraintStage,
            samples: ResizeSamples,
        },
        NativeFocus {
            stage: NativeFocusStage,
            samples: ResizeSamples,
        },
        DocumentChrome {
            stage: DocumentChromeStage,
            samples: ResizeSamples,
        },
        MousePriming {
            baseline_pointer_moves: usize,
            samples: ResizeSamples,
        },
        Mouse {
            next_action: usize,
            wait: MouseWait,
            samples: ResizeSamples,
        },
        ContextMenu {
            interaction: ContextInteraction,
            samples: ResizeSamples,
        },
        PopupSoakSettling {
            stage: PopupSoakSampleStage,
            deadline: Instant,
            samples: ResizeSamples,
        },
        Settling {
            deadline: Instant,
            samples: ResizeSamples,
        },
        Idle {
            deadline: Instant,
            baseline_frame: u64,
            samples: ResizeSamples,
        },
        Finished,
    }

    #[derive(Clone, Copy, Debug)]
    enum ConstraintStage {
        PreMinimumSmall,
        RuntimeMinimumGrowth,
        MinimumCleared,
        PostClearSmall,
        FinalSizeRestored,
    }

    #[derive(Clone, Copy, Debug)]
    enum NativeFocusStage {
        NativeEditor,
        FrameworkControl,
    }

    #[derive(Clone, Copy, Debug)]
    enum DocumentChromeStage {
        Set,
        Clear,
    }

    #[derive(Clone, Copy, Debug)]
    enum PopupSoakSampleStage {
        Baseline,
        Final,
    }

    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    enum NativeFocusProgress {
        Wait,
        RequestFramework,
        Complete,
    }

    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    enum DocumentChromeProgress {
        Wait,
        RequestClear,
        Complete,
    }

    fn native_focus_progress(
        stage: NativeFocusStage,
        completed_commands: usize,
        native_editor_active: bool,
        framework_focus_active: bool,
    ) -> NativeFocusProgress {
        match stage {
            NativeFocusStage::NativeEditor
                if completed_commands >= NATIVE_EDITOR_FOCUS_COMMANDS && native_editor_active =>
            {
                NativeFocusProgress::RequestFramework
            }
            NativeFocusStage::FrameworkControl
                if completed_commands >= FRAMEWORK_FOCUS_COMMANDS
                    && framework_focus_active
                    && !native_editor_active =>
            {
                NativeFocusProgress::Complete
            }
            _ => NativeFocusProgress::Wait,
        }
    }

    fn document_chrome_progress(
        stage: DocumentChromeStage,
        completed_commands: usize,
        retained_represented_file: bool,
        retained_document_edited: bool,
        native_represented_file_matches: bool,
        native_represented_file_cleared: bool,
        native_document_edited: bool,
    ) -> DocumentChromeProgress {
        match stage {
            DocumentChromeStage::Set
                if completed_commands >= DOCUMENT_CHROME_SET_COMMANDS
                    && retained_represented_file
                    && retained_document_edited
                    && native_represented_file_matches
                    && native_document_edited =>
            {
                DocumentChromeProgress::RequestClear
            }
            DocumentChromeStage::Clear
                if completed_commands >= DOCUMENT_CHROME_CLEAR_COMMANDS
                    && !retained_represented_file
                    && !retained_document_edited
                    && native_represented_file_cleared
                    && !native_document_edited =>
            {
                DocumentChromeProgress::Complete
            }
            _ => DocumentChromeProgress::Wait,
        }
    }

    #[derive(Clone, Copy)]
    struct ContextCycleObservation {
        menu_open_observed: bool,
        submenu_hover_observed: bool,
        action_received: bool,
        menu_closed: bool,
        owner_window_focused: bool,
        framework_focus_active: bool,
        popup: PopupObservation,
        inactive_teardown_observed: bool,
    }

    fn context_cycle_complete(
        interaction: ContextInteraction,
        observation: ContextCycleObservation,
    ) -> bool {
        let ContextCycleObservation {
            menu_open_observed,
            submenu_hover_observed,
            action_received,
            menu_closed,
            owner_window_focused,
            framework_focus_active,
            popup,
            inactive_teardown_observed,
        } = observation;
        let action_matches = match interaction {
            ContextInteraction::Command | ContextInteraction::SoakCommand => action_received,
            ContextInteraction::OutsideDismiss | ContextInteraction::EscapeDismiss => {
                !action_received
            }
            ContextInteraction::DeactivationDismiss => !action_received,
        };
        let lifecycle_matches = match interaction {
            ContextInteraction::DeactivationDismiss => inactive_teardown_observed,
            ContextInteraction::Command
            | ContextInteraction::SoakCommand
            | ContextInteraction::OutsideDismiss
            | ContextInteraction::EscapeDismiss => true,
        };
        let native_focus_matches = match interaction {
            ContextInteraction::DeactivationDismiss => {
                !owner_window_focused && !popup.owner_key && popup.framework_first_responder
            }
            ContextInteraction::SoakCommand => popup.framework_first_responder,
            ContextInteraction::Command
            | ContextInteraction::OutsideDismiss
            | ContextInteraction::EscapeDismiss => {
                owner_window_focused && popup.owner_key && popup.framework_first_responder
            }
        };
        menu_open_observed
            && submenu_hover_observed
            && action_matches
            && lifecycle_matches
            && menu_closed
            && framework_focus_active
            && native_focus_matches
            && popup.window_count() == 0
    }

    impl ProbePhase {
        fn name(&self) -> &'static str {
            match self {
                Self::Warmup { .. } => "warmup",
                Self::Resizing { .. } => "resizing",
                Self::Constraints { stage, .. } => match stage {
                    ConstraintStage::PreMinimumSmall => "constraint-pre-minimum-small",
                    ConstraintStage::RuntimeMinimumGrowth => "constraint-runtime-growth",
                    ConstraintStage::MinimumCleared => "constraint-clear",
                    ConstraintStage::PostClearSmall => "constraint-post-clear-small",
                    ConstraintStage::FinalSizeRestored => "constraint-final-restore",
                },
                Self::NativeFocus { stage, .. } => match stage {
                    NativeFocusStage::NativeEditor => "native-focus-editor",
                    NativeFocusStage::FrameworkControl => "native-focus-framework",
                },
                Self::DocumentChrome { stage, .. } => match stage {
                    DocumentChromeStage::Set => "document-chrome-set",
                    DocumentChromeStage::Clear => "document-chrome-clear",
                },
                Self::MousePriming { .. } => "mouse-prime",
                Self::Mouse { next_action, .. } => MouseScriptAction::SCRIPT
                    .get(*next_action)
                    .map_or("mouse-complete", MouseScriptAction::name),
                Self::ContextMenu { interaction, .. } => match interaction {
                    ContextInteraction::Command => "context-menu-command",
                    ContextInteraction::OutsideDismiss => "context-menu-outside-dismiss",
                    ContextInteraction::EscapeDismiss => "context-menu-escape-dismiss",
                    ContextInteraction::SoakCommand => "context-menu-resource-soak",
                    ContextInteraction::DeactivationDismiss => {
                        "context-menu-application-deactivation"
                    }
                },
                Self::PopupSoakSettling { stage, .. } => match stage {
                    PopupSoakSampleStage::Baseline => "context-menu-soak-baseline",
                    PopupSoakSampleStage::Final => "context-menu-soak-final",
                },
                Self::Settling { .. } => "settling",
                Self::Idle { .. } => "idle",
                Self::Finished => "finished",
            }
        }
    }

    enum ProbeAction {
        Animate,
        Resize {
            size: Size,
            native_visible: bool,
        },
        Window(WindowProbeCommand),
        Mouse(MouseScriptAction),
        ReopenContext {
            interaction: ContextInteraction,
            cycle: usize,
        },
        Observe,
        Wait(Instant),
        Exit,
    }

    struct AcceptanceProbe {
        phase: ProbePhase,
        last_frame: u64,
        failed: Arc<AtomicBool>,
        budgets: AcceptanceBudgets,
        initial_native_mount: bool,
        native_detach_observed: bool,
        native_remount_observed: bool,
        initial_minimum_size_observed: bool,
        pre_minimum_small_size_observed: bool,
        runtime_minimum_size_observed: bool,
        runtime_minimum_growth_observed: bool,
        native_minimum_size_observed: bool,
        minimum_size_clear_observed: bool,
        native_minimum_clear_observed: bool,
        post_clear_small_size_observed: bool,
        final_size_restored: bool,
        native_editor_focus_observed: bool,
        framework_focus_restored: bool,
        represented_file_observed: bool,
        document_edited_observed: bool,
        represented_file_clear_observed: bool,
        document_edited_clear_observed: bool,
        context_menu_open_observed: bool,
        context_submenu_hover_observed: bool,
        context_menu_action_received: bool,
        context_menu_close_observed: bool,
        context_menu_cycles_completed: usize,
        context_menu_focus_restored_cycles: usize,
        context_menu_teardown_cycles: usize,
        context_menu_outside_dismiss_cycles_completed: usize,
        context_menu_outside_focus_restored_cycles: usize,
        context_menu_outside_teardown_cycles: usize,
        context_menu_escape_dismiss_cycles_completed: usize,
        context_menu_escape_focus_restored_cycles: usize,
        context_menu_escape_teardown_cycles: usize,
        context_menu_soak_cycles_completed: usize,
        context_menu_soak_baseline_memory: Option<ProcessMemorySample>,
        context_menu_soak_final_memory: Option<ProcessMemorySample>,
        context_menu_deactivation_dismiss_cycles_completed: usize,
        context_menu_deactivation_no_focus_steal_cycles: usize,
        context_menu_deactivation_teardown_cycles: usize,
        max_context_popup_windows: usize,
        last_context_owner_window_focused: bool,
        last_context_framework_focus_active: bool,
        last_context_inactive_teardown_observed: bool,
        last_context_popup: PopupObservation,
        window_commands_completed: usize,
        command_failures: Vec<String>,
    }

    impl AcceptanceProbe {
        fn new(failed: Arc<AtomicBool>) -> Self {
            Self {
                phase: ProbePhase::Warmup { observed: 0 },
                last_frame: 0,
                failed,
                budgets: AcceptanceBudgets::default(),
                initial_native_mount: false,
                native_detach_observed: false,
                native_remount_observed: false,
                initial_minimum_size_observed: false,
                pre_minimum_small_size_observed: false,
                runtime_minimum_size_observed: false,
                runtime_minimum_growth_observed: false,
                native_minimum_size_observed: false,
                minimum_size_clear_observed: false,
                native_minimum_clear_observed: false,
                post_clear_small_size_observed: false,
                final_size_restored: false,
                native_editor_focus_observed: false,
                framework_focus_restored: false,
                represented_file_observed: false,
                document_edited_observed: false,
                represented_file_clear_observed: false,
                document_edited_clear_observed: false,
                context_menu_open_observed: false,
                context_submenu_hover_observed: false,
                context_menu_action_received: false,
                context_menu_close_observed: false,
                context_menu_cycles_completed: 0,
                context_menu_focus_restored_cycles: 0,
                context_menu_teardown_cycles: 0,
                context_menu_outside_dismiss_cycles_completed: 0,
                context_menu_outside_focus_restored_cycles: 0,
                context_menu_outside_teardown_cycles: 0,
                context_menu_escape_dismiss_cycles_completed: 0,
                context_menu_escape_focus_restored_cycles: 0,
                context_menu_escape_teardown_cycles: 0,
                context_menu_soak_cycles_completed: 0,
                context_menu_soak_baseline_memory: None,
                context_menu_soak_final_memory: None,
                context_menu_deactivation_dismiss_cycles_completed: 0,
                context_menu_deactivation_no_focus_steal_cycles: 0,
                context_menu_deactivation_teardown_cycles: 0,
                max_context_popup_windows: 0,
                last_context_owner_window_focused: false,
                last_context_framework_focus_active: false,
                last_context_inactive_teardown_observed: false,
                last_context_popup: PopupObservation::default(),
                window_commands_completed: 0,
                command_failures: Vec::new(),
            }
        }

        fn observe(
            &mut self,
            observation: ProbeObservation,
            mouse: &mut MouseAcceptanceState,
        ) -> ProbeAction {
            let ProbeObservation {
                metrics,
                now,
                viewport,
                native_visible,
                native,
                window_state,
                framework_focus_active,
                popup,
                context_menu_open_observed,
                context_submenu_hover_observed,
                context_action_received,
                context_menu_closed,
                context_inactive_teardown_observed,
            } = observation;
            if native_visible && native.mounted {
                if self.native_detach_observed {
                    self.native_remount_observed = true;
                } else {
                    self.initial_native_mount = true;
                }
            } else if !native_visible && !native.mounted {
                self.native_detach_observed = true;
            }
            self.initial_minimum_size_observed |=
                window_state.minimum_size == Some(DEFAULT_MINIMUM_SIZE);
            self.context_menu_open_observed |= context_menu_open_observed;
            self.context_submenu_hover_observed |= context_submenu_hover_observed;
            self.context_menu_action_received |= context_action_received;
            self.context_menu_close_observed |= context_menu_closed;
            self.max_context_popup_windows =
                self.max_context_popup_windows.max(popup.window_count());
            if matches!(self.phase, ProbePhase::ContextMenu { .. }) {
                self.last_context_owner_window_focused = window_state.focused;
                self.last_context_framework_focus_active = framework_focus_active;
                self.last_context_inactive_teardown_observed = context_inactive_teardown_observed;
                self.last_context_popup = popup;
            }

            if metrics.frame_number == 0 || metrics.frame_number == self.last_frame {
                return match self.phase {
                    ProbePhase::Warmup { .. } | ProbePhase::Resizing { .. } => ProbeAction::Animate,
                    ProbePhase::Constraints { .. }
                    | ProbePhase::NativeFocus { .. }
                    | ProbePhase::DocumentChrome { .. }
                    | ProbePhase::MousePriming { .. }
                    | ProbePhase::Mouse { .. } => ProbeAction::Observe,
                    ProbePhase::ContextMenu { .. } => ProbeAction::Observe,
                    ProbePhase::PopupSoakSettling { deadline, .. }
                    | ProbePhase::Settling { deadline, .. }
                    | ProbePhase::Idle { deadline, .. } => ProbeAction::Wait(deadline),
                    ProbePhase::Finished => ProbeAction::Exit,
                };
            }
            self.last_frame = metrics.frame_number;

            let phase = std::mem::replace(&mut self.phase, ProbePhase::Finished);
            match phase {
                ProbePhase::Warmup { observed } => {
                    let observed = observed + 1;
                    if observed < WARMUP_FRAMES {
                        self.phase = ProbePhase::Warmup { observed };
                        ProbeAction::Animate
                    } else {
                        emit_line(&format!(
                            "QUICKGUI_ACCEPTANCE_BEGIN warmup_frames={WARMUP_FRAMES} resize_commands={RESIZE_COMMANDS} text_areas={} native_views=1 native_focus_handoffs=1 document_chrome_cycles=1 window_constraints=1 context_menu_cycles={CONTEXT_MENU_CYCLES} context_menu_outside_dismiss_cycles={CONTEXT_MENU_OUTSIDE_DISMISS_CYCLES} context_menu_escape_dismiss_cycles={CONTEXT_MENU_ESCAPE_DISMISS_CYCLES} context_menu_soak_cycles={CONTEXT_MENU_SOAK_CYCLES} context_menu_soak_baseline_cycle={CONTEXT_MENU_SOAK_BASELINE_CYCLE} context_menu_soak_reopen_delay_ms={} context_menu_deactivation_dismiss_cycles={CONTEXT_MENU_DEACTIVATION_DISMISS_CYCLES} mouse_actions={}",
                            TEXT_COLUMNS * TEXT_ROWS_PER_COLUMN,
                            CONTEXT_MENU_SOAK_REOPEN_DELAY.as_millis(),
                            MouseScriptAction::SCRIPT.len(),
                        ));
                        let samples = ResizeSamples::new(now);
                        let command = 0;
                        let native_visible = native_visibility_for_command(command);
                        self.phase = ProbePhase::Resizing {
                            commands: 1,
                            samples,
                        };
                        ProbeAction::Resize {
                            size: resize_size(command),
                            native_visible,
                        }
                    }
                }
                ProbePhase::Resizing {
                    commands,
                    mut samples,
                } => {
                    samples.push(metrics, viewport);
                    if commands < RESIZE_COMMANDS {
                        let command = commands;
                        self.phase = ProbePhase::Resizing {
                            commands: commands + 1,
                            samples,
                        };
                        ProbeAction::Resize {
                            size: resize_size(command),
                            native_visible: native_visibility_for_command(command),
                        }
                    } else {
                        samples.finished_at = Some(now);
                        self.phase = ProbePhase::Constraints {
                            stage: ConstraintStage::PreMinimumSmall,
                            samples,
                        };
                        ProbeAction::Window(WindowProbeCommand::Resize(PRE_MINIMUM_SIZE))
                    }
                }
                ProbePhase::Constraints { stage, samples } => match stage {
                    ConstraintStage::PreMinimumSmall => {
                        if self.window_commands_completed >= 1
                            && size_approximately(viewport, PRE_MINIMUM_SIZE)
                        {
                            self.pre_minimum_small_size_observed = true;
                            self.phase = ProbePhase::Constraints {
                                stage: ConstraintStage::RuntimeMinimumGrowth,
                                samples,
                            };
                            ProbeAction::Window(WindowProbeCommand::SetMinimum(
                                RUNTIME_MINIMUM_SIZE,
                            ))
                        } else {
                            self.phase = ProbePhase::Constraints { stage, samples };
                            ProbeAction::Observe
                        }
                    }
                    ConstraintStage::RuntimeMinimumGrowth => {
                        let minimum_observed =
                            window_state.minimum_size == Some(RUNTIME_MINIMUM_SIZE);
                        let native_minimum_observed =
                            native_minimum_approximately(native, RUNTIME_MINIMUM_SIZE);
                        self.runtime_minimum_size_observed |= minimum_observed;
                        self.native_minimum_size_observed |= native_minimum_observed;
                        if self.window_commands_completed >= 2
                            && minimum_observed
                            && native_minimum_observed
                            && size_at_least(viewport, RUNTIME_MINIMUM_SIZE)
                        {
                            self.runtime_minimum_growth_observed = true;
                            self.phase = ProbePhase::Constraints {
                                stage: ConstraintStage::MinimumCleared,
                                samples,
                            };
                            ProbeAction::Window(WindowProbeCommand::ClearMinimum)
                        } else {
                            self.phase = ProbePhase::Constraints { stage, samples };
                            ProbeAction::Observe
                        }
                    }
                    ConstraintStage::MinimumCleared => {
                        let native_minimum_cleared =
                            native_minimum_approximately(native, Size::ZERO);
                        if self.window_commands_completed >= 3
                            && window_state.minimum_size.is_none()
                            && native_minimum_cleared
                        {
                            self.minimum_size_clear_observed = true;
                            self.native_minimum_clear_observed = true;
                            self.phase = ProbePhase::Constraints {
                                stage: ConstraintStage::PostClearSmall,
                                samples,
                            };
                            ProbeAction::Window(WindowProbeCommand::Resize(PRE_MINIMUM_SIZE))
                        } else {
                            self.phase = ProbePhase::Constraints { stage, samples };
                            ProbeAction::Observe
                        }
                    }
                    ConstraintStage::PostClearSmall => {
                        if self.window_commands_completed >= 4
                            && window_state.minimum_size.is_none()
                            && size_approximately(viewport, PRE_MINIMUM_SIZE)
                        {
                            self.post_clear_small_size_observed = true;
                            self.phase = ProbePhase::Constraints {
                                stage: ConstraintStage::FinalSizeRestored,
                                samples,
                            };
                            ProbeAction::Window(WindowProbeCommand::Resize(FINAL_WINDOW_SIZE))
                        } else {
                            self.phase = ProbePhase::Constraints { stage, samples };
                            ProbeAction::Observe
                        }
                    }
                    ConstraintStage::FinalSizeRestored => {
                        if self.window_commands_completed >= 5
                            && window_state.minimum_size.is_none()
                            && size_approximately(viewport, FINAL_WINDOW_SIZE)
                        {
                            self.final_size_restored = true;
                            self.phase = ProbePhase::NativeFocus {
                                stage: NativeFocusStage::NativeEditor,
                                samples,
                            };
                            ProbeAction::Window(WindowProbeCommand::FocusNativeEditor)
                        } else {
                            self.phase = ProbePhase::Constraints { stage, samples };
                            ProbeAction::Observe
                        }
                    }
                },
                ProbePhase::NativeFocus { stage, samples } => match stage {
                    NativeFocusStage::NativeEditor => {
                        if native_focus_progress(
                            stage,
                            self.window_commands_completed,
                            native.editor_active,
                            framework_focus_active,
                        ) == NativeFocusProgress::RequestFramework
                        {
                            self.native_editor_focus_observed = true;
                            self.phase = ProbePhase::NativeFocus {
                                stage: NativeFocusStage::FrameworkControl,
                                samples,
                            };
                            ProbeAction::Window(WindowProbeCommand::FocusFrameworkControl)
                        } else {
                            self.phase = ProbePhase::NativeFocus { stage, samples };
                            ProbeAction::Observe
                        }
                    }
                    NativeFocusStage::FrameworkControl => {
                        if native_focus_progress(
                            stage,
                            self.window_commands_completed,
                            native.editor_active,
                            framework_focus_active,
                        ) == NativeFocusProgress::Complete
                        {
                            self.framework_focus_restored = true;
                            self.phase = ProbePhase::DocumentChrome {
                                stage: DocumentChromeStage::Set,
                                samples,
                            };
                            ProbeAction::Window(WindowProbeCommand::SetDocumentChrome)
                        } else {
                            self.phase = ProbePhase::NativeFocus { stage, samples };
                            ProbeAction::Observe
                        }
                    }
                },
                ProbePhase::DocumentChrome { stage, samples } => {
                    let progress = document_chrome_progress(
                        stage,
                        self.window_commands_completed,
                        window_state.represented_file,
                        window_state.document_edited,
                        native.represented_file_matches,
                        native.represented_file_cleared,
                        native.document_edited,
                    );
                    match (stage, progress) {
                        (DocumentChromeStage::Set, DocumentChromeProgress::RequestClear) => {
                            self.represented_file_observed = true;
                            self.document_edited_observed = true;
                            self.phase = ProbePhase::DocumentChrome {
                                stage: DocumentChromeStage::Clear,
                                samples,
                            };
                            ProbeAction::Window(WindowProbeCommand::ClearDocumentChrome)
                        }
                        (DocumentChromeStage::Clear, DocumentChromeProgress::Complete) => {
                            self.represented_file_clear_observed = true;
                            self.document_edited_clear_observed = true;
                            self.phase = ProbePhase::MousePriming {
                                baseline_pointer_moves: mouse.total_pointer_moves,
                                samples,
                            };
                            ProbeAction::Mouse(MouseScriptAction::MoveOutside)
                        }
                        _ => {
                            self.phase = ProbePhase::DocumentChrome { stage, samples };
                            ProbeAction::Observe
                        }
                    }
                }
                ProbePhase::MousePriming {
                    baseline_pointer_moves,
                    samples,
                } => {
                    if mouse.total_pointer_moves > baseline_pointer_moves {
                        mouse.reset_for_script();
                        let action = MouseScriptAction::SCRIPT[0];
                        mouse.native_actions_scheduled = 1;
                        self.phase = ProbePhase::Mouse {
                            next_action: 1,
                            wait: MouseWait::new(action, mouse),
                            samples,
                        };
                        ProbeAction::Mouse(action)
                    } else {
                        self.phase = ProbePhase::MousePriming {
                            baseline_pointer_moves,
                            samples,
                        };
                        ProbeAction::Observe
                    }
                }
                ProbePhase::Mouse {
                    next_action,
                    wait,
                    samples,
                } => {
                    if !wait.completed(mouse) {
                        self.phase = ProbePhase::Mouse {
                            next_action,
                            wait,
                            samples,
                        };
                        ProbeAction::Observe
                    } else if next_action < MouseScriptAction::SCRIPT.len() {
                        let action = MouseScriptAction::SCRIPT[next_action];
                        mouse.native_actions_scheduled =
                            mouse.native_actions_scheduled.saturating_add(1);
                        self.phase = ProbePhase::Mouse {
                            next_action: next_action + 1,
                            wait: MouseWait::new(action, mouse),
                            samples,
                        };
                        ProbeAction::Mouse(action)
                    } else {
                        mouse.stop_recording();
                        self.phase = ProbePhase::ContextMenu {
                            interaction: ContextInteraction::Command,
                            samples,
                        };
                        ProbeAction::Animate
                    }
                }
                ProbePhase::ContextMenu {
                    interaction,
                    samples,
                } => {
                    if context_cycle_complete(
                        interaction,
                        ContextCycleObservation {
                            menu_open_observed: context_menu_open_observed,
                            submenu_hover_observed: context_submenu_hover_observed,
                            action_received: context_action_received,
                            menu_closed: context_menu_closed,
                            owner_window_focused: window_state.focused,
                            framework_focus_active,
                            popup,
                            inactive_teardown_observed: context_inactive_teardown_observed,
                        },
                    ) {
                        match interaction {
                            ContextInteraction::Command => {
                                self.context_menu_cycles_completed =
                                    self.context_menu_cycles_completed.saturating_add(1);
                                self.context_menu_focus_restored_cycles =
                                    self.context_menu_focus_restored_cycles.saturating_add(1);
                                self.context_menu_teardown_cycles =
                                    self.context_menu_teardown_cycles.saturating_add(1);
                                if self.context_menu_cycles_completed < CONTEXT_MENU_CYCLES {
                                    let cycle = self.context_menu_cycles_completed;
                                    self.phase = ProbePhase::ContextMenu {
                                        interaction,
                                        samples,
                                    };
                                    ProbeAction::ReopenContext { interaction, cycle }
                                } else {
                                    let interaction = ContextInteraction::OutsideDismiss;
                                    self.phase = ProbePhase::ContextMenu {
                                        interaction,
                                        samples,
                                    };
                                    ProbeAction::ReopenContext {
                                        interaction,
                                        cycle: 0,
                                    }
                                }
                            }
                            ContextInteraction::OutsideDismiss => {
                                self.context_menu_outside_dismiss_cycles_completed = self
                                    .context_menu_outside_dismiss_cycles_completed
                                    .saturating_add(1);
                                self.context_menu_outside_focus_restored_cycles = self
                                    .context_menu_outside_focus_restored_cycles
                                    .saturating_add(1);
                                self.context_menu_outside_teardown_cycles =
                                    self.context_menu_outside_teardown_cycles.saturating_add(1);
                                if self.context_menu_outside_dismiss_cycles_completed
                                    < CONTEXT_MENU_OUTSIDE_DISMISS_CYCLES
                                {
                                    let cycle = self.context_menu_outside_dismiss_cycles_completed;
                                    self.phase = ProbePhase::ContextMenu {
                                        interaction,
                                        samples,
                                    };
                                    ProbeAction::ReopenContext { interaction, cycle }
                                } else {
                                    let interaction = ContextInteraction::EscapeDismiss;
                                    self.phase = ProbePhase::ContextMenu {
                                        interaction,
                                        samples,
                                    };
                                    ProbeAction::ReopenContext {
                                        interaction,
                                        cycle: 0,
                                    }
                                }
                            }
                            ContextInteraction::EscapeDismiss => {
                                self.context_menu_escape_dismiss_cycles_completed = self
                                    .context_menu_escape_dismiss_cycles_completed
                                    .saturating_add(1);
                                self.context_menu_escape_focus_restored_cycles = self
                                    .context_menu_escape_focus_restored_cycles
                                    .saturating_add(1);
                                self.context_menu_escape_teardown_cycles =
                                    self.context_menu_escape_teardown_cycles.saturating_add(1);
                                if self.context_menu_escape_dismiss_cycles_completed
                                    < CONTEXT_MENU_ESCAPE_DISMISS_CYCLES
                                {
                                    let cycle = self.context_menu_escape_dismiss_cycles_completed;
                                    self.phase = ProbePhase::ContextMenu {
                                        interaction,
                                        samples,
                                    };
                                    ProbeAction::ReopenContext { interaction, cycle }
                                } else {
                                    let interaction = ContextInteraction::SoakCommand;
                                    self.phase = ProbePhase::ContextMenu {
                                        interaction,
                                        samples,
                                    };
                                    ProbeAction::ReopenContext {
                                        interaction,
                                        cycle: 0,
                                    }
                                }
                            }
                            ContextInteraction::SoakCommand => {
                                self.context_menu_soak_cycles_completed =
                                    self.context_menu_soak_cycles_completed.saturating_add(1);
                                if self.context_menu_soak_cycles_completed
                                    == CONTEXT_MENU_SOAK_BASELINE_CYCLE
                                {
                                    let deadline = now
                                        .checked_add(CONTEXT_MENU_SOAK_SETTLE_DURATION)
                                        .unwrap_or(now);
                                    self.phase = ProbePhase::PopupSoakSettling {
                                        stage: PopupSoakSampleStage::Baseline,
                                        deadline,
                                        samples,
                                    };
                                    ProbeAction::Wait(deadline)
                                } else if self.context_menu_soak_cycles_completed
                                    < CONTEXT_MENU_SOAK_CYCLES
                                {
                                    let cycle = self.context_menu_soak_cycles_completed;
                                    self.phase = ProbePhase::ContextMenu {
                                        interaction,
                                        samples,
                                    };
                                    ProbeAction::ReopenContext { interaction, cycle }
                                } else {
                                    let deadline = now
                                        .checked_add(CONTEXT_MENU_SOAK_SETTLE_DURATION)
                                        .unwrap_or(now);
                                    self.phase = ProbePhase::PopupSoakSettling {
                                        stage: PopupSoakSampleStage::Final,
                                        deadline,
                                        samples,
                                    };
                                    ProbeAction::Wait(deadline)
                                }
                            }
                            ContextInteraction::DeactivationDismiss => {
                                self.context_menu_deactivation_dismiss_cycles_completed = self
                                    .context_menu_deactivation_dismiss_cycles_completed
                                    .saturating_add(1);
                                self.context_menu_deactivation_no_focus_steal_cycles = self
                                    .context_menu_deactivation_no_focus_steal_cycles
                                    .saturating_add(1);
                                self.context_menu_deactivation_teardown_cycles = self
                                    .context_menu_deactivation_teardown_cycles
                                    .saturating_add(1);
                                if self.context_menu_deactivation_dismiss_cycles_completed
                                    < CONTEXT_MENU_DEACTIVATION_DISMISS_CYCLES
                                {
                                    let cycle =
                                        self.context_menu_deactivation_dismiss_cycles_completed;
                                    self.phase = ProbePhase::ContextMenu {
                                        interaction,
                                        samples,
                                    };
                                    ProbeAction::ReopenContext { interaction, cycle }
                                } else {
                                    let deadline = now.checked_add(SETTLE_DURATION).unwrap_or(now);
                                    self.phase = ProbePhase::Settling { deadline, samples };
                                    ProbeAction::Wait(deadline)
                                }
                            }
                        }
                    } else {
                        self.phase = ProbePhase::ContextMenu {
                            interaction,
                            samples,
                        };
                        ProbeAction::Observe
                    }
                }
                ProbePhase::PopupSoakSettling {
                    stage,
                    deadline,
                    samples,
                } => {
                    if now < deadline {
                        self.phase = ProbePhase::PopupSoakSettling {
                            stage,
                            deadline,
                            samples,
                        };
                        ProbeAction::Wait(deadline)
                    } else {
                        let memory = match current_process_memory() {
                            Ok(memory) => memory,
                            Err(error) => {
                                self.fail_command(error);
                                ProcessMemorySample::default()
                            }
                        };
                        match stage {
                            PopupSoakSampleStage::Baseline => {
                                self.context_menu_soak_baseline_memory = Some(memory);
                                let interaction = ContextInteraction::SoakCommand;
                                let cycle = self.context_menu_soak_cycles_completed;
                                self.phase = ProbePhase::ContextMenu {
                                    interaction,
                                    samples,
                                };
                                ProbeAction::ReopenContext { interaction, cycle }
                            }
                            PopupSoakSampleStage::Final => {
                                self.context_menu_soak_final_memory = Some(memory);
                                let interaction = ContextInteraction::DeactivationDismiss;
                                self.phase = ProbePhase::ContextMenu {
                                    interaction,
                                    samples,
                                };
                                ProbeAction::ReopenContext {
                                    interaction,
                                    cycle: 0,
                                }
                            }
                        }
                    }
                }
                ProbePhase::Settling { deadline, samples } => {
                    if now < deadline {
                        self.phase = ProbePhase::Settling { deadline, samples };
                        ProbeAction::Wait(deadline)
                    } else {
                        let deadline = now.checked_add(IDLE_DURATION).unwrap_or(now);
                        self.phase = ProbePhase::Idle {
                            deadline,
                            baseline_frame: metrics.frame_number,
                            samples,
                        };
                        ProbeAction::Wait(deadline)
                    }
                }
                ProbePhase::Idle {
                    deadline: _,
                    baseline_frame,
                    samples,
                } => {
                    let report = self.finish_report(
                        samples,
                        now,
                        viewport,
                        native,
                        metrics.frame_number.saturating_sub(baseline_frame),
                        mouse,
                    );
                    self.failed.store(!report.passed, Ordering::Relaxed);
                    let serialized = serde_json::to_string(&report).unwrap_or_else(|error| {
                        format!("{{\"passed\":false,\"serialization_error\":\"{error}\"}}")
                    });
                    emit_line(&format!("QUICKGUI_ACCEPTANCE_RESULT {serialized}"));
                    self.phase = ProbePhase::Finished;
                    ProbeAction::Exit
                }
                ProbePhase::Finished => {
                    self.phase = ProbePhase::Finished;
                    ProbeAction::Exit
                }
            }
        }

        fn finish_report(
            &mut self,
            samples: ResizeSamples,
            now: Instant,
            viewport: Size,
            native: NativeObservation,
            idle_frame_delta: u64,
            mouse: &MouseAcceptanceState,
        ) -> AcceptanceReport {
            // Settling and the idle audit intentionally present only their two verification
            // frames. Measure resize cadence over the resize phase itself, not over those three
            // seconds of deliberate sleep.
            let resize_finished_at = samples.finished_at.unwrap_or(now);
            let elapsed = resize_finished_at.saturating_duration_since(samples.started_at);
            let elapsed_seconds = elapsed.as_secs_f64().max(f64::EPSILON);
            let presented_frames = samples.cpu_ms.len();
            let total_cpu_ms = samples.cpu_ms.iter().sum::<f64>();
            let mut cpu_ms = samples.cpu_ms;
            cpu_ms.sort_by(f64::total_cmp);
            let average_frame_cpu_ms = if presented_frames == 0 {
                0.0
            } else {
                total_cpu_ms / presented_frames as f64
            };
            let mut submission_ms = samples.submission_ms;
            let average_frame_submission_ms = if presented_frames == 0 {
                0.0
            } else {
                submission_ms.iter().sum::<f64>() / presented_frames as f64
            };
            submission_ms.sort_by(f64::total_cmp);
            let expected_native_width = (f64::from(viewport.width) - ROOT_PADDING * 2.0).max(0.0);
            let idle_extra_frames = idle_frame_delta.saturating_sub(1);
            let soak_baseline = self.context_menu_soak_baseline_memory.unwrap_or_default();
            let soak_final = self.context_menu_soak_final_memory.unwrap_or_default();
            let soak_rss_growth_mib = soak_final
                .resident_bytes
                .saturating_sub(soak_baseline.resident_bytes)
                as f64
                / ProcessMemorySample::MIB;
            let soak_footprint_growth_mib = soak_final
                .physical_footprint_bytes
                .saturating_sub(soak_baseline.physical_footprint_bytes)
                as f64
                / ProcessMemorySample::MIB;
            let mut report = AcceptanceReport {
                schema_version: 11,
                resize_commands: RESIZE_COMMANDS,
                presented_frames,
                elapsed_ms: elapsed.as_secs_f64() * 1_000.0,
                presentation_hz: presented_frames as f64 / elapsed_seconds,
                main_thread_cpu_percent: total_cpu_ms / 1_000.0 / elapsed_seconds * 100.0,
                average_frame_cpu_ms,
                p95_frame_cpu_ms: percentile(&cpu_ms, 0.95),
                max_frame_cpu_ms: cpu_ms.last().copied().unwrap_or_default(),
                average_frame_submission_ms,
                p95_frame_submission_ms: percentile(&submission_ms, 0.95),
                max_frame_submission_ms: submission_ms.last().copied().unwrap_or_default(),
                min_viewport_width: samples.min_viewport.width,
                max_viewport_width: samples.max_viewport.width,
                min_viewport_height: samples.min_viewport.height,
                max_viewport_height: samples.max_viewport.height,
                max_visible_text_areas: samples.max_visible_text_areas,
                max_reshaped_text_areas: samples.max_reshaped_text_areas,
                max_retained_text_areas: samples.max_retained_text_areas,
                max_retained_text_layouts: samples.max_retained_text_layouts,
                max_retained_text_renderers: samples.max_retained_text_renderers,
                max_draw_calls: samples.max_draw_calls,
                initial_minimum_size_observed: self.initial_minimum_size_observed,
                pre_minimum_small_size_observed: self.pre_minimum_small_size_observed,
                runtime_minimum_size_observed: self.runtime_minimum_size_observed,
                runtime_minimum_growth_observed: self.runtime_minimum_growth_observed,
                native_minimum_size_observed: self.native_minimum_size_observed,
                minimum_size_clear_observed: self.minimum_size_clear_observed,
                native_minimum_clear_observed: self.native_minimum_clear_observed,
                post_clear_small_size_observed: self.post_clear_small_size_observed,
                final_size_restored: self.final_size_restored,
                native_editor_focus_observed: self.native_editor_focus_observed,
                framework_focus_restored: self.framework_focus_restored,
                represented_file_observed: self.represented_file_observed,
                document_edited_observed: self.document_edited_observed,
                represented_file_clear_observed: self.represented_file_clear_observed,
                document_edited_clear_observed: self.document_edited_clear_observed,
                native_mouse_actions: mouse.native_actions_scheduled,
                mouse_dispatch_trace: mouse.dispatch_trace.clone(),
                mouse_trace_overflow: mouse.trace_overflow,
                mouse_down_click_count: mouse.down_click_count,
                mouse_up_click_count: mouse.up_click_count,
                mouse_hover_entries: mouse.hover_entries,
                mouse_hover_exits: mouse.hover_exits,
                mouse_move_events: mouse.move_events,
                mouse_drag_move_events: mouse.drag_move_events,
                mouse_exit_events: mouse.exit_events,
                mouse_exit_without_pressed_button: mouse.exit_without_pressed_button,
                mouse_default_clicks: mouse.default_clicks,
                mouse_pointer_events: mouse.script_pointer_moves,
                mouse_button_events: mouse.script_button_events,
                mouse_pointer_exits: mouse.script_pointer_exits,
                context_menu_open_observed: self.context_menu_open_observed,
                context_submenu_hover_observed: self.context_submenu_hover_observed,
                context_menu_action_received: self.context_menu_action_received,
                context_menu_close_observed: self.context_menu_close_observed,
                context_menu_cycles_completed: self.context_menu_cycles_completed,
                context_menu_focus_restored_cycles: self.context_menu_focus_restored_cycles,
                context_menu_teardown_cycles: self.context_menu_teardown_cycles,
                context_menu_outside_dismiss_cycles_completed: self
                    .context_menu_outside_dismiss_cycles_completed,
                context_menu_outside_focus_restored_cycles: self
                    .context_menu_outside_focus_restored_cycles,
                context_menu_outside_teardown_cycles: self.context_menu_outside_teardown_cycles,
                context_menu_escape_dismiss_cycles_completed: self
                    .context_menu_escape_dismiss_cycles_completed,
                context_menu_escape_focus_restored_cycles: self
                    .context_menu_escape_focus_restored_cycles,
                context_menu_escape_teardown_cycles: self.context_menu_escape_teardown_cycles,
                context_menu_soak_cycles_completed: self.context_menu_soak_cycles_completed,
                context_menu_soak_baseline_cycle: CONTEXT_MENU_SOAK_BASELINE_CYCLE,
                context_menu_soak_reopen_delay_ms: CONTEXT_MENU_SOAK_REOPEN_DELAY.as_millis()
                    as u64,
                context_menu_soak_baseline_rss_mib: soak_baseline.resident_mib(),
                context_menu_soak_final_rss_mib: soak_final.resident_mib(),
                context_menu_soak_rss_growth_mib: soak_rss_growth_mib,
                context_menu_soak_baseline_footprint_mib: soak_baseline.physical_footprint_mib(),
                context_menu_soak_final_footprint_mib: soak_final.physical_footprint_mib(),
                context_menu_soak_footprint_growth_mib: soak_footprint_growth_mib,
                context_menu_deactivation_dismiss_cycles_completed: self
                    .context_menu_deactivation_dismiss_cycles_completed,
                context_menu_deactivation_no_focus_steal_cycles: self
                    .context_menu_deactivation_no_focus_steal_cycles,
                context_menu_deactivation_teardown_cycles: self
                    .context_menu_deactivation_teardown_cycles,
                max_context_popup_windows: self.max_context_popup_windows,
                initial_native_mount: self.initial_native_mount,
                native_detach_observed: self.native_detach_observed,
                native_remount_observed: self.native_remount_observed,
                final_native_mounted: native.mounted,
                final_native_width: native.width,
                expected_native_width,
                final_native_height: native.height,
                final_native_opacity: native.opacity,
                expected_native_opacity: f64::from(NATIVE_OPACITY),
                idle_frame_delta,
                idle_extra_frames,
                budgets: self.budgets,
                passed: false,
                failures: std::mem::take(&mut self.command_failures),
            };
            evaluate_report(&mut report);
            report.passed = report.failures.is_empty();
            report
        }

        fn fail_command(&mut self, error: String) {
            if self.command_failures.len() < 8 {
                self.command_failures
                    .push(format!("acceptance command failed: {error}"));
            }
        }

        fn note_window_command_completed(&mut self) {
            self.window_commands_completed = self.window_commands_completed.saturating_add(1);
        }

        fn fail_timeout(
            &mut self,
            mouse: &MouseAcceptanceState,
            context_menu_open: bool,
            context_activation_scheduled: bool,
        ) {
            let phase = self.phase.name();
            self.failed.store(true, Ordering::Relaxed);
            self.phase = ProbePhase::Finished;
            emit_line(&format!(
                "QUICKGUI_ACCEPTANCE_ERROR timeout_seconds={} phase={phase} window_commands_completed={} context_menu_open={} context_activation_scheduled={} context_menu_cycles_completed={} context_menu_outside_dismiss_cycles_completed={} context_menu_escape_dismiss_cycles_completed={} context_menu_soak_cycles_completed={} context_menu_deactivation_dismiss_cycles_completed={} context_menu_open_observed={} context_submenu_hover_observed={} context_menu_action_received={} context_menu_close_observed={} inactive_teardown_observed={} owner_window_focused={} framework_focus_active={} owner_key={} framework_first_responder={} context_popup_windows={} max_context_popup_windows={} native_mouse_actions={} mouse_trace={:?} failures={:?}",
                WATCHDOG_DURATION.as_secs(),
                self.window_commands_completed,
                context_menu_open,
                context_activation_scheduled,
                self.context_menu_cycles_completed,
                self.context_menu_outside_dismiss_cycles_completed,
                self.context_menu_escape_dismiss_cycles_completed,
                self.context_menu_soak_cycles_completed,
                self.context_menu_deactivation_dismiss_cycles_completed,
                self.context_menu_open_observed,
                self.context_submenu_hover_observed,
                self.context_menu_action_received,
                self.context_menu_close_observed,
                self.last_context_inactive_teardown_observed,
                self.last_context_owner_window_focused,
                self.last_context_framework_focus_active,
                self.last_context_popup.owner_key,
                self.last_context_popup.framework_first_responder,
                self.last_context_popup.window_count(),
                self.max_context_popup_windows,
                mouse.native_actions_scheduled,
                mouse.dispatch_trace,
                self.command_failures,
            ));
        }

        fn is_finished(&self) -> bool {
            matches!(self.phase, ProbePhase::Finished)
        }

        fn observes_document_chrome(&self) -> bool {
            matches!(self.phase, ProbePhase::DocumentChrome { .. })
        }

        fn observes_context_menu(&self) -> bool {
            matches!(self.phase, ProbePhase::ContextMenu { .. })
        }
    }

    fn native_visibility_for_command(command: usize) -> bool {
        !(60..75).contains(&command)
    }

    fn resize_size(command: usize) -> Size {
        let phase = command % 120;
        let progress = if phase < 60 {
            phase as f32 / 59.0
        } else {
            (119 - phase) as f32 / 59.0
        };
        Size::new(760.0 + 420.0 * progress, 520.0 + 200.0 * progress)
    }

    fn size_approximately(actual: Size, expected: Size) -> bool {
        (actual.width - expected.width).abs() <= WINDOW_SIZE_TOLERANCE
            && (actual.height - expected.height).abs() <= WINDOW_SIZE_TOLERANCE
    }

    fn size_at_least(actual: Size, minimum: Size) -> bool {
        actual.width + WINDOW_SIZE_TOLERANCE >= minimum.width
            && actual.height + WINDOW_SIZE_TOLERANCE >= minimum.height
    }

    fn native_minimum_approximately(native: NativeObservation, expected: Size) -> bool {
        (native.content_minimum_width - f64::from(expected.width)).abs()
            <= f64::from(WINDOW_SIZE_TOLERANCE)
            && (native.content_minimum_height - f64::from(expected.height)).abs()
                <= f64::from(WINDOW_SIZE_TOLERANCE)
    }

    fn percentile(sorted: &[f64], fraction: f64) -> f64 {
        if sorted.is_empty() {
            return 0.0;
        }
        let rank = (sorted.len() as f64 * fraction.clamp(0.0, 1.0)).ceil() as usize;
        sorted[rank.saturating_sub(1).min(sorted.len() - 1)]
    }

    fn evaluate_report(report: &mut AcceptanceReport) {
        push_f64_max(
            &mut report.failures,
            "main_thread_cpu_percent",
            report.main_thread_cpu_percent,
            report.budgets.max_main_thread_cpu_percent,
        );
        push_f64_max(
            &mut report.failures,
            "p95_frame_cpu_ms",
            report.p95_frame_cpu_ms,
            report.budgets.max_p95_frame_cpu_ms,
        );
        push_f64_max(
            &mut report.failures,
            "max_frame_cpu_ms",
            report.max_frame_cpu_ms,
            report.budgets.max_frame_cpu_ms,
        );
        if report.presentation_hz < report.budgets.min_presentation_hz {
            report.failures.push(format!(
                "presentation_hz was {:.3}, minimum is {:.3}",
                report.presentation_hz, report.budgets.min_presentation_hz
            ));
        }
        push_usize_max(
            &mut report.failures,
            "max_visible_text_areas",
            report.max_visible_text_areas,
            report.budgets.max_visible_text_areas,
        );
        push_usize_max(
            &mut report.failures,
            "max_retained_text_areas",
            report.max_retained_text_areas,
            report.budgets.max_retained_text_areas,
        );
        push_usize_max(
            &mut report.failures,
            "max_retained_text_layouts",
            report.max_retained_text_layouts,
            report.budgets.max_retained_text_layouts,
        );
        push_usize_max(
            &mut report.failures,
            "max_retained_text_renderers",
            report.max_retained_text_renderers,
            report.budgets.max_retained_text_renderers,
        );
        push_usize_max(
            &mut report.failures,
            "max_draw_calls",
            report.max_draw_calls,
            report.budgets.max_draw_calls,
        );
        if report.min_viewport_width > 780.0
            || report.max_viewport_width < 1_150.0
            || report.min_viewport_height > 540.0
            || report.max_viewport_height < 700.0
        {
            report.failures.push(format!(
                "native resize range was only {:.1}x{:.1} through {:.1}x{:.1}",
                report.min_viewport_width,
                report.min_viewport_height,
                report.max_viewport_width,
                report.max_viewport_height
            ));
        }
        for (name, passed) in [
            (
                "initial_minimum_size_observed",
                report.initial_minimum_size_observed,
            ),
            (
                "pre_minimum_small_size_observed",
                report.pre_minimum_small_size_observed,
            ),
            (
                "runtime_minimum_size_observed",
                report.runtime_minimum_size_observed,
            ),
            (
                "runtime_minimum_growth_observed",
                report.runtime_minimum_growth_observed,
            ),
            (
                "native_minimum_size_observed",
                report.native_minimum_size_observed,
            ),
            (
                "minimum_size_clear_observed",
                report.minimum_size_clear_observed,
            ),
            (
                "native_minimum_clear_observed",
                report.native_minimum_clear_observed,
            ),
            (
                "post_clear_small_size_observed",
                report.post_clear_small_size_observed,
            ),
            ("final_size_restored", report.final_size_restored),
            (
                "native_editor_focus_observed",
                report.native_editor_focus_observed,
            ),
            ("framework_focus_restored", report.framework_focus_restored),
            (
                "represented_file_observed",
                report.represented_file_observed,
            ),
            ("document_edited_observed", report.document_edited_observed),
            (
                "represented_file_clear_observed",
                report.represented_file_clear_observed,
            ),
            (
                "document_edited_clear_observed",
                report.document_edited_clear_observed,
            ),
        ] {
            if !passed {
                report.failures.push(format!("{name} was false"));
            }
        }
        if report.native_mouse_actions != MouseScriptAction::SCRIPT.len() {
            report.failures.push(format!(
                "native_mouse_actions was {}, expected {}",
                report.native_mouse_actions,
                MouseScriptAction::SCRIPT.len()
            ));
        }
        if report.mouse_dispatch_trace.as_slice() != EXPECTED_MOUSE_TRACE {
            report.failures.push(format!(
                "mouse_dispatch_trace was {:?}, expected {:?}",
                report.mouse_dispatch_trace, EXPECTED_MOUSE_TRACE
            ));
        }
        if report.mouse_trace_overflow {
            report
                .failures
                .push("mouse_trace_overflow was true".to_owned());
        }
        if report.mouse_down_click_count != 2 || report.mouse_up_click_count != 2 {
            report.failures.push(format!(
                "native click counts were down={} up={}, expected 2/2",
                report.mouse_down_click_count, report.mouse_up_click_count
            ));
        }
        // The synthetic script has two exact crossings, but the real pointer can also cross the
        // content while this live WindowServer probe repeatedly resizes underneath it. Require
        // the complete balanced semantics without treating incidental native motion as a defect.
        if report.mouse_hover_entries < 2
            || report.mouse_hover_exits < 2
            || report.mouse_hover_entries != report.mouse_hover_exits
        {
            report.failures.push(format!(
                "mouse hover transitions were entries={} exits={}, expected at least two balanced transitions",
                report.mouse_hover_entries, report.mouse_hover_exits
            ));
        }
        if report.mouse_move_events < 3 {
            report.failures.push(format!(
                "mouse_move_events was {}, expected at least 3",
                report.mouse_move_events
            ));
        }
        if report.mouse_drag_move_events == 0 {
            report
                .failures
                .push("mouse_drag_move_events was 0".to_owned());
        }
        if report.mouse_exit_events == 0 || !report.mouse_exit_without_pressed_button {
            report.failures.push(format!(
                "mouse exit was count={} without_pressed_button={}, expected at least one released-button exit",
                report.mouse_exit_events, report.mouse_exit_without_pressed_button
            ));
        }
        if report.mouse_default_clicks != 0 {
            report.failures.push(format!(
                "mouse_default_clicks was {}, expected prevent_default to keep it at 0",
                report.mouse_default_clicks
            ));
        }
        if report.mouse_pointer_events < 4
            || report.mouse_button_events != 6
            || report.mouse_pointer_exits == 0
        {
            report.failures.push(format!(
                "native mouse event totals were moves={} buttons={} exits={}, expected >=4/6/>=1",
                report.mouse_pointer_events, report.mouse_button_events, report.mouse_pointer_exits
            ));
        }
        for (name, passed) in [
            (
                "context_menu_open_observed",
                report.context_menu_open_observed,
            ),
            (
                "context_submenu_hover_observed",
                report.context_submenu_hover_observed,
            ),
            (
                "context_menu_action_received",
                report.context_menu_action_received,
            ),
            (
                "context_menu_close_observed",
                report.context_menu_close_observed,
            ),
        ] {
            if !passed {
                report.failures.push(format!("{name} was false"));
            }
        }
        for (name, actual) in [
            (
                "context_menu_cycles_completed",
                report.context_menu_cycles_completed,
            ),
            (
                "context_menu_focus_restored_cycles",
                report.context_menu_focus_restored_cycles,
            ),
            (
                "context_menu_teardown_cycles",
                report.context_menu_teardown_cycles,
            ),
        ] {
            if actual != CONTEXT_MENU_CYCLES {
                report.failures.push(format!(
                    "{name} was {actual}, expected {CONTEXT_MENU_CYCLES}"
                ));
            }
        }
        for (name, actual) in [
            (
                "context_menu_outside_dismiss_cycles_completed",
                report.context_menu_outside_dismiss_cycles_completed,
            ),
            (
                "context_menu_outside_focus_restored_cycles",
                report.context_menu_outside_focus_restored_cycles,
            ),
            (
                "context_menu_outside_teardown_cycles",
                report.context_menu_outside_teardown_cycles,
            ),
        ] {
            if actual != CONTEXT_MENU_OUTSIDE_DISMISS_CYCLES {
                report.failures.push(format!(
                    "{name} was {actual}, expected {CONTEXT_MENU_OUTSIDE_DISMISS_CYCLES}"
                ));
            }
        }
        for (name, actual) in [
            (
                "context_menu_escape_dismiss_cycles_completed",
                report.context_menu_escape_dismiss_cycles_completed,
            ),
            (
                "context_menu_escape_focus_restored_cycles",
                report.context_menu_escape_focus_restored_cycles,
            ),
            (
                "context_menu_escape_teardown_cycles",
                report.context_menu_escape_teardown_cycles,
            ),
        ] {
            if actual != CONTEXT_MENU_ESCAPE_DISMISS_CYCLES {
                report.failures.push(format!(
                    "{name} was {actual}, expected {CONTEXT_MENU_ESCAPE_DISMISS_CYCLES}"
                ));
            }
        }
        if report.context_menu_soak_cycles_completed != CONTEXT_MENU_SOAK_CYCLES {
            report.failures.push(format!(
                "context_menu_soak_cycles_completed was {}, expected {CONTEXT_MENU_SOAK_CYCLES}",
                report.context_menu_soak_cycles_completed
            ));
        }
        if report.context_menu_soak_baseline_cycle != CONTEXT_MENU_SOAK_BASELINE_CYCLE {
            report.failures.push(format!(
                "context_menu_soak_baseline_cycle was {}, expected {CONTEXT_MENU_SOAK_BASELINE_CYCLE}",
                report.context_menu_soak_baseline_cycle
            ));
        }
        for (name, actual) in [
            (
                "context_menu_soak_baseline_rss_mib",
                report.context_menu_soak_baseline_rss_mib,
            ),
            (
                "context_menu_soak_final_rss_mib",
                report.context_menu_soak_final_rss_mib,
            ),
            (
                "context_menu_soak_baseline_footprint_mib",
                report.context_menu_soak_baseline_footprint_mib,
            ),
            (
                "context_menu_soak_final_footprint_mib",
                report.context_menu_soak_final_footprint_mib,
            ),
        ] {
            if !actual.is_finite() || actual <= 0.0 {
                report.failures.push(format!(
                    "{name} was {actual}, expected a positive finite sample"
                ));
            }
        }
        push_f64_max(
            &mut report.failures,
            "context_menu_soak_rss_growth_mib",
            report.context_menu_soak_rss_growth_mib,
            report.budgets.max_context_soak_rss_growth_mib,
        );
        push_f64_max(
            &mut report.failures,
            "context_menu_soak_footprint_growth_mib",
            report.context_menu_soak_footprint_growth_mib,
            report.budgets.max_context_soak_footprint_growth_mib,
        );
        for (name, actual) in [
            (
                "context_menu_deactivation_dismiss_cycles_completed",
                report.context_menu_deactivation_dismiss_cycles_completed,
            ),
            (
                "context_menu_deactivation_no_focus_steal_cycles",
                report.context_menu_deactivation_no_focus_steal_cycles,
            ),
            (
                "context_menu_deactivation_teardown_cycles",
                report.context_menu_deactivation_teardown_cycles,
            ),
        ] {
            if actual != CONTEXT_MENU_DEACTIVATION_DISMISS_CYCLES {
                report.failures.push(format!(
                    "{name} was {actual}, expected {CONTEXT_MENU_DEACTIVATION_DISMISS_CYCLES}"
                ));
            }
        }
        if report.max_context_popup_windows < 1 || report.max_context_popup_windows > 2 {
            report.failures.push(format!(
                "max_context_popup_windows was {}, expected 1..=2",
                report.max_context_popup_windows
            ));
        }
        for (name, passed) in [
            ("initial_native_mount", report.initial_native_mount),
            ("native_detach_observed", report.native_detach_observed),
            ("native_remount_observed", report.native_remount_observed),
            ("final_native_mounted", report.final_native_mounted),
        ] {
            if !passed {
                report.failures.push(format!("{name} was false"));
            }
        }
        if (report.final_native_width - report.expected_native_width).abs()
            > NATIVE_GEOMETRY_TOLERANCE
        {
            report.failures.push(format!(
                "final native width was {:.2}, expected {:.2}",
                report.final_native_width, report.expected_native_width
            ));
        }
        if (report.final_native_height - NATIVE_HEIGHT).abs() > NATIVE_GEOMETRY_TOLERANCE {
            report.failures.push(format!(
                "final native height was {:.2}, expected {:.2}",
                report.final_native_height, NATIVE_HEIGHT
            ));
        }
        if (report.final_native_opacity - report.expected_native_opacity).abs()
            > NATIVE_OPACITY_TOLERANCE
        {
            report.failures.push(format!(
                "final native opacity was {:.3}, expected {:.3}",
                report.final_native_opacity, report.expected_native_opacity
            ));
        }
        if report.idle_frame_delta == 0 {
            report.failures.push(
                "idle_frame_delta was 0; the verification frame was not presented".to_owned(),
            );
        }
        if report.idle_extra_frames > report.budgets.max_idle_extra_frames {
            report.failures.push(format!(
                "idle_extra_frames was {}, budget is {}",
                report.idle_extra_frames, report.budgets.max_idle_extra_frames
            ));
        }
    }

    fn push_f64_max(failures: &mut Vec<String>, name: &str, actual: f64, maximum: f64) {
        if actual > maximum {
            failures.push(format!("{name} was {actual:.3}, budget is {maximum:.3}"));
        }
    }

    fn push_usize_max(failures: &mut Vec<String>, name: &str, actual: usize, maximum: usize) {
        if actual > maximum {
            failures.push(format!("{name} was {actual}, budget is {maximum}"));
        }
    }

    fn emit_line(line: &str) {
        let mut stdout = io::stdout().lock();
        let _ = writeln!(stdout, "{line}");
        let _ = stdout.flush();
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn resize_wave_reaches_both_bounds_and_native_visibility_recovers() {
            let sizes = (0..RESIZE_COMMANDS).map(resize_size).collect::<Vec<_>>();
            assert!(sizes.iter().any(|size| size.width <= 760.0));
            assert!(sizes.iter().any(|size| size.width >= 1_180.0));
            assert!(!native_visibility_for_command(60));
            assert!(!native_visibility_for_command(74));
            assert!(native_visibility_for_command(75));
            assert!(native_visibility_for_command(RESIZE_COMMANDS - 1));
        }

        #[test]
        fn percentile_uses_the_bounded_nearest_rank_sample() {
            let samples: Vec<_> = (1..=100).map(f64::from).collect();
            assert_eq!(percentile(&samples, 0.95), 95.0);
            assert_eq!(percentile(&[], 0.95), 0.0);
        }

        #[test]
        fn constraint_probe_crosses_and_clears_the_runtime_minimum() {
            assert!(RUNTIME_MINIMUM_SIZE.width > PRE_MINIMUM_SIZE.width);
            assert!(RUNTIME_MINIMUM_SIZE.height > PRE_MINIMUM_SIZE.height);
            assert!(size_at_least(RUNTIME_MINIMUM_SIZE, RUNTIME_MINIMUM_SIZE));
            assert!(size_approximately(PRE_MINIMUM_SIZE, PRE_MINIMUM_SIZE));

            let native = NativeObservation {
                mounted: true,
                editor_active: false,
                width: 0.0,
                height: 0.0,
                opacity: 1.0,
                content_minimum_width: f64::from(RUNTIME_MINIMUM_SIZE.width),
                content_minimum_height: f64::from(RUNTIME_MINIMUM_SIZE.height),
                represented_file_matches: false,
                represented_file_cleared: true,
                document_edited: false,
            };
            assert!(native_minimum_approximately(native, RUNTIME_MINIMUM_SIZE));
            assert!(!native_minimum_approximately(native, Size::ZERO));
        }

        #[test]
        fn native_focus_probe_requires_both_observed_sides_of_the_handoff() {
            assert_eq!(
                native_focus_progress(
                    NativeFocusStage::NativeEditor,
                    NATIVE_EDITOR_FOCUS_COMMANDS - 1,
                    true,
                    false,
                ),
                NativeFocusProgress::Wait
            );
            assert_eq!(
                native_focus_progress(
                    NativeFocusStage::NativeEditor,
                    NATIVE_EDITOR_FOCUS_COMMANDS,
                    true,
                    false,
                ),
                NativeFocusProgress::RequestFramework
            );
            assert_eq!(
                native_focus_progress(
                    NativeFocusStage::FrameworkControl,
                    FRAMEWORK_FOCUS_COMMANDS,
                    true,
                    true,
                ),
                NativeFocusProgress::Wait
            );
            assert_eq!(
                native_focus_progress(
                    NativeFocusStage::FrameworkControl,
                    FRAMEWORK_FOCUS_COMMANDS,
                    false,
                    true,
                ),
                NativeFocusProgress::Complete
            );
        }

        #[test]
        fn document_chrome_probe_requires_retained_and_native_set_and_clear_state() {
            assert_eq!(
                document_chrome_progress(
                    DocumentChromeStage::Set,
                    DOCUMENT_CHROME_SET_COMMANDS - 1,
                    true,
                    true,
                    true,
                    false,
                    true,
                ),
                DocumentChromeProgress::Wait
            );
            assert_eq!(
                document_chrome_progress(
                    DocumentChromeStage::Set,
                    DOCUMENT_CHROME_SET_COMMANDS,
                    true,
                    true,
                    true,
                    false,
                    true,
                ),
                DocumentChromeProgress::RequestClear
            );
            assert_eq!(
                document_chrome_progress(
                    DocumentChromeStage::Clear,
                    DOCUMENT_CHROME_CLEAR_COMMANDS,
                    false,
                    false,
                    false,
                    false,
                    false,
                ),
                DocumentChromeProgress::Wait
            );
            assert_eq!(
                document_chrome_progress(
                    DocumentChromeStage::Clear,
                    DOCUMENT_CHROME_CLEAR_COMMANDS,
                    false,
                    false,
                    false,
                    true,
                    false,
                ),
                DocumentChromeProgress::Complete
            );
        }

        #[test]
        fn document_chrome_native_queries_are_scoped_out_of_normal_frames() {
            let mut probe = AcceptanceProbe::new(Arc::new(AtomicBool::new(false)));
            assert!(!probe.observes_document_chrome());

            probe.phase = ProbePhase::DocumentChrome {
                stage: DocumentChromeStage::Set,
                samples: ResizeSamples::new(Instant::now()),
            };
            assert!(probe.observes_document_chrome());

            probe.phase = ProbePhase::MousePriming {
                baseline_pointer_moves: 0,
                samples: ResizeSamples::new(Instant::now()),
            };
            assert!(!probe.observes_document_chrome());
        }

        #[test]
        fn context_cycle_requires_interaction_specific_focus_and_complete_native_teardown() {
            let restored = PopupObservation {
                owner_key: true,
                framework_first_responder: true,
                root_windows: 0,
                submenu_windows: 0,
            };
            let complete = ContextCycleObservation {
                menu_open_observed: true,
                submenu_hover_observed: true,
                action_received: true,
                menu_closed: true,
                owner_window_focused: true,
                framework_focus_active: true,
                popup: restored,
                inactive_teardown_observed: false,
            };
            assert!(context_cycle_complete(
                ContextInteraction::Command,
                complete,
            ));
            assert!(!context_cycle_complete(
                ContextInteraction::Command,
                ContextCycleObservation {
                    popup: PopupObservation {
                        root_windows: 1,
                        ..restored
                    },
                    ..complete
                },
            ));
            assert!(context_cycle_complete(
                ContextInteraction::SoakCommand,
                ContextCycleObservation {
                    owner_window_focused: false,
                    popup: PopupObservation {
                        owner_key: false,
                        ..restored
                    },
                    ..complete
                },
            ));
            assert!(!context_cycle_complete(
                ContextInteraction::SoakCommand,
                ContextCycleObservation {
                    action_received: false,
                    ..complete
                },
            ));
            assert!(!context_cycle_complete(
                ContextInteraction::Command,
                ContextCycleObservation {
                    framework_focus_active: false,
                    ..complete
                },
            ));
            assert!(!context_cycle_complete(
                ContextInteraction::Command,
                ContextCycleObservation {
                    popup: PopupObservation {
                        owner_key: false,
                        ..restored
                    },
                    ..complete
                },
            ));
            assert!(context_cycle_complete(
                ContextInteraction::OutsideDismiss,
                ContextCycleObservation {
                    action_received: false,
                    ..complete
                },
            ));
            assert!(!context_cycle_complete(
                ContextInteraction::OutsideDismiss,
                complete,
            ));
            assert!(context_cycle_complete(
                ContextInteraction::EscapeDismiss,
                ContextCycleObservation {
                    action_received: false,
                    ..complete
                },
            ));
            assert!(!context_cycle_complete(
                ContextInteraction::EscapeDismiss,
                complete,
            ));
            assert!(context_cycle_complete(
                ContextInteraction::DeactivationDismiss,
                ContextCycleObservation {
                    action_received: false,
                    inactive_teardown_observed: true,
                    owner_window_focused: false,
                    popup: PopupObservation {
                        owner_key: false,
                        ..restored
                    },
                    ..complete
                },
            ));
            assert!(!context_cycle_complete(
                ContextInteraction::DeactivationDismiss,
                ContextCycleObservation {
                    action_received: false,
                    owner_window_focused: false,
                    popup: PopupObservation {
                        owner_key: false,
                        ..restored
                    },
                    ..complete
                },
            ));
            assert!(!context_cycle_complete(
                ContextInteraction::DeactivationDismiss,
                ContextCycleObservation {
                    action_received: false,
                    inactive_teardown_observed: true,
                    ..complete
                },
            ));
        }

        #[test]
        fn current_process_memory_reports_resident_and_physical_footprint() {
            let sample =
                current_process_memory().expect("current process memory should be readable");
            assert!(sample.resident_bytes > 0);
            assert!(sample.physical_footprint_bytes > 0);
            assert!(CONTEXT_MENU_SOAK_BASELINE_CYCLE < CONTEXT_MENU_SOAK_CYCLES);
        }

        #[test]
        fn mouse_acceptance_script_and_trace_fit_their_hard_bounds() {
            assert_eq!(MouseScriptAction::SCRIPT.len(), 11);
            assert!(EXPECTED_MOUSE_TRACE.len() <= MAX_MOUSE_TRACE_STEPS);
            assert_eq!(
                MouseScriptAction::SCRIPT
                    .iter()
                    .filter(|action| matches!(action.signal(), MouseSignal::Button))
                    .count(),
                6
            );
            assert_eq!(
                MouseScriptAction::SCRIPT
                    .iter()
                    .filter(|action| matches!(action.signal(), MouseSignal::Exit))
                    .count(),
                1
            );
        }
    }
}

#[cfg(target_os = "macos")]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    app::run()
}

#[cfg(not(target_os = "macos"))]
fn main() {
    eprintln!("The macos_acceptance example requires macOS.");
}
