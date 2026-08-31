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

    fn push_button_trace(&mut self, button: MouseButton, left: &'static str, right: &'static str) {
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
            Self::MoveTarget | Self::LeftDragTarget | Self::MoveOutside | Self::MoveTargetAgain => {
                MouseSignal::Move
            }
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
                                "AppKit rejected the native editor as first responder".to_owned()
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
        if self.context_menu.popover_window().is_none() {
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
                        post_popover_pointer("Context menu", 16.0, 20.0, true)
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
                        post_popover_pointer("Context submenu", 16.0, 20.0, true)
                    })
                    .await?;
                if result.is_err() {
                    task_cx.sleep(Duration::from_millis(100)).await?;
                    result = task_cx
                        .update(|_this, _cx| {
                            post_popover_pointer("Context submenu", 16.0, 20.0, true)
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
                    if let Err(error) = post_popover_pointer("Context menu", 16.0, 20.0, false) {
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
                    post_popover_pointer("Context submenu", 16.0, 20.0, activate)
                })
                .await?;
            if result.is_err() {
                // The child geometry exists before AppKit exposes the hidden-first-frame
                // panel relation. Retry once at a later exact deadline; never poll or animate.
                task_cx.sleep(Duration::from_millis(400)).await?;
                result = task_cx
                    .update(move |_this, _cx| {
                        post_popover_pointer("Context submenu", 16.0, 20.0, activate)
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
                                if let Err(error) = send_escape_to_key_popover() {
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
                        // Observe a complete stability window rather than one lucky instant.
                        // Nonactivating-panel focus restoration can arrive more than a second
                        // after Finder initially wins the activation handoff.
                        let mut inactive_throughout = true;
                        let mut teardown = false;
                        for sample in 1..=6 {
                            task_cx.sleep(Duration::from_millis(250)).await?;
                            let observation = task_cx
                                .update(move |this, cx| {
                                    let inactive = match acceptance_application_is_active() {
                                        Ok(active) => !active,
                                        Err(error) => {
                                            this.probe.fail_command(error);
                                            false
                                        }
                                    };
                                    let popover = this.popover_observation();
                                    let teardown = !this.context_menu.is_open()
                                        && popover.window_count() == 0
                                        && !popover.owner_key;
                                    if !inactive {
                                        this.probe.fail_command(format!(
                                            "application reactivated during deactivation stability sample {sample}"
                                        ));
                                    }
                                    cx.invalidate();
                                    (inactive, teardown)
                                })
                                .await?;
                            inactive_throughout &= observation.0;
                            teardown = observation.1;
                        }
                        task_cx
                            .update(move |this, cx| {
                                let popover = this.popover_observation();
                                this.context_inactive_teardown_observed |=
                                    inactive_throughout && teardown;
                                if !inactive_throughout || !teardown {
                                    this.probe.fail_command(format!(
                                        "application deactivation observed inactive_throughout={inactive_throughout} context_open={} owner_key={} popover_windows={}",
                                        this.context_menu.is_open(),
                                        popover.owner_key,
                                        popover.window_count(),
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
                    if interaction == ContextInteraction::DeactivationDismiss
                        && let Err(error) = activate_acceptance_application()
                    {
                        this.probe.fail_command(error);
                    }
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

    fn popover_observation(&self) -> PopoverObservation {
        let Some(owner) = self.field.window() else {
            return PopoverObservation::default();
        };
        let owner_key = owner.isKeyWindow();
        let framework_first_responder = owner
            .firstResponder()
            .zip(owner.contentView())
            .is_some_and(|(responder, content)| {
                Retained::as_ptr(&responder).cast::<()>() == Retained::as_ptr(&content).cast::<()>()
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
        PopoverObservation {
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

fn send_owner_context_click(field: &NSTextField, event_number_offset: isize) -> Result<(), String> {
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

fn post_popover_pointer(title: &str, x: f64, y: f64, click: bool) -> Result<(), String> {
    let mtm = MainThreadMarker::new()
        .ok_or_else(|| "popover acceptance action left the AppKit thread".to_owned())?;
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

fn send_escape_to_key_popover() -> Result<(), String> {
    const MACOS_ESCAPE_KEY_CODE: u16 = 53;

    let mtm = MainThreadMarker::new()
        .ok_or_else(|| "popover Escape acceptance action left the AppKit thread".to_owned())?;
    let application = NSApplication::sharedApplication(mtm);
    let window = application
        .keyWindow()
        .ok_or_else(|| "no AppKit key window received popover Escape".to_owned())?;
    let title = window.title().to_string();
    if title != "Context menu" && title != "Context submenu" {
        return Err(format!(
            "popover Escape key window was {title:?}, expected the context-menu chain"
        ));
    }
    let content = window
        .contentView()
        .ok_or_else(|| format!("popover Escape key window {title:?} has no content view"))?;
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
    .ok_or_else(|| format!("AppKit refused Escape for popover window {title:?}"))?;
    // Calling the responder is deterministic for nonactivating popover panels while preserving
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

fn activate_acceptance_application() -> Result<(), String> {
    let mtm = MainThreadMarker::new().ok_or_else(|| {
        "application activation acceptance action left the AppKit thread".to_owned()
    })?;
    let application = NSApplication::sharedApplication(mtm);
    // SAFETY: Application activation state is queried on AppKit's main thread.
    if unsafe { application.isActive() } {
        return Ok(());
    }
    // SAFETY: The current process application is retained and activated from its AppKit
    // thread. This explicit reset starts the next independent handoff cycle.
    let activated = unsafe {
        NSRunningApplication::currentApplication()
            .activateWithOptions(NSApplicationActivationOptions(0))
    };
    if !activated {
        return Err(
            "the acceptance application could not begin its next activation cycle".to_owned(),
        );
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
        .ok_or_else(|| format!("popover window {title:?} has no content view"))?;
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
            format!("AppKit refused {event_type:?} for popover window {title:?}")
        })?;
        // Invoke the live Winit content responder exactly as AppKit would so all three events
        // reach one retained popover while key-window state is transitioning.
        unsafe {
            match event_type {
                NSEventType::MouseMoved => content.mouseMoved(&event),
                NSEventType::LeftMouseDown => content.mouseDown(&event),
                NSEventType::LeftMouseUp => content.mouseUp(&event),
                _ => unreachable!("popover acceptance uses only move and primary click events"),
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
        let card_down_out = cx.mouse_down_listener("acceptance-mouse-card", |this, event, cx| {
            this.mouse
                .push_button_trace(event.button, "card-down-out-left", "card-down-out-right");
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
        let card_up_capture = cx.mouse_up_listener("acceptance-mouse-card", |this, event, cx| {
            this.mouse.push_button_trace(
                event.button,
                "card-up-capture-left",
                "card-up-capture-right",
            );
            cx.invalidate();
        });
        let card_up_bubble = cx.mouse_up_listener("acceptance-mouse-card", |this, event, cx| {
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
                // every context-menu cycle an exact owner focus to preserve so the popover gate
                // cannot pass by merely returning native key-window focus.
                cx.focus(FocusHandle::new(FRAMEWORK_FOCUS_ID));
                cx.stop_propagation();
                cx.invalidate();
            });
        let target_left_up = cx.mouse_up_listener("acceptance-mouse-target", |this, event, cx| {
            if this.mouse.recording {
                this.mouse.up_click_count = event.click_count;
            }
            this.mouse.push_trace("target-up-left");
            cx.invalidate();
        });
        let target_move = cx.mouse_move_listener("acceptance-mouse-target", |this, event, cx| {
            if this.mouse.recording {
                this.mouse.move_events = this.mouse.move_events.saturating_add(1);
                if event.dragging_button(MouseButton::Left) {
                    this.mouse.drag_move_events = this.mouse.drag_move_events.saturating_add(1);
                }
            }
            cx.invalidate();
        });
        let target_exit = cx.mouse_exit_listener("acceptance-mouse-target", |this, event, cx| {
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
        let popover = if self.probe.observes_context_menu() {
            self.popover_observation()
        } else {
            PopoverObservation::default()
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
                popover,
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
                let submenu = PopoverMenu::new([PopoverMenuItem::action(
                    "accept-context-command",
                    "Accept context command",
                    ContextAcceptanceCommand,
                )])
                .expect("the static acceptance context submenu is valid");
                Some(
                    PopoverMenu::new([PopoverMenuItem::submenu(
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
            |item, state: PopoverMenuItemState| {
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
