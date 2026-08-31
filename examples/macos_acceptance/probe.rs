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
struct PopoverObservation {
    owner_key: bool,
    framework_first_responder: bool,
    root_windows: usize,
    submenu_windows: usize,
}

impl PopoverObservation {
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
    popover: PopoverObservation,
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
    max_context_popover_windows: usize,
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
        self.max_visible_text_areas = self.max_visible_text_areas.max(metrics.render.text_areas);
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
    PopoverSoakSettling {
        stage: PopoverSoakSampleStage,
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
enum PopoverSoakSampleStage {
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
    popover: PopoverObservation,
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
        popover,
        inactive_teardown_observed,
    } = observation;
    let action_matches = match interaction {
        ContextInteraction::Command | ContextInteraction::SoakCommand => action_received,
        ContextInteraction::OutsideDismiss | ContextInteraction::EscapeDismiss => !action_received,
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
            !owner_window_focused && !popover.owner_key && popover.framework_first_responder
        }
        ContextInteraction::SoakCommand => popover.framework_first_responder,
        ContextInteraction::Command
        | ContextInteraction::OutsideDismiss
        | ContextInteraction::EscapeDismiss => {
            owner_window_focused && popover.owner_key && popover.framework_first_responder
        }
    };
    menu_open_observed
        && submenu_hover_observed
        && action_matches
        && lifecycle_matches
        && menu_closed
        && framework_focus_active
        && native_focus_matches
        && popover.window_count() == 0
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
                ContextInteraction::DeactivationDismiss => "context-menu-application-deactivation",
            },
            Self::PopoverSoakSettling { stage, .. } => match stage {
                PopoverSoakSampleStage::Baseline => "context-menu-soak-baseline",
                PopoverSoakSampleStage::Final => "context-menu-soak-final",
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
    max_context_popover_windows: usize,
    last_context_owner_window_focused: bool,
    last_context_framework_focus_active: bool,
    last_context_inactive_teardown_observed: bool,
    last_context_popover: PopoverObservation,
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
            max_context_popover_windows: 0,
            last_context_owner_window_focused: false,
            last_context_framework_focus_active: false,
            last_context_inactive_teardown_observed: false,
            last_context_popover: PopoverObservation::default(),
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
            popover,
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
        self.max_context_popover_windows =
            self.max_context_popover_windows.max(popover.window_count());
        if matches!(self.phase, ProbePhase::ContextMenu { .. }) {
            self.last_context_owner_window_focused = window_state.focused;
            self.last_context_framework_focus_active = framework_focus_active;
            self.last_context_inactive_teardown_observed = context_inactive_teardown_observed;
            self.last_context_popover = popover;
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
                ProbePhase::PopoverSoakSettling { deadline, .. }
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
                        ProbeAction::Window(WindowProbeCommand::SetMinimum(RUNTIME_MINIMUM_SIZE))
                    } else {
                        self.phase = ProbePhase::Constraints { stage, samples };
                        ProbeAction::Observe
                    }
                }
                ConstraintStage::RuntimeMinimumGrowth => {
                    let minimum_observed = window_state.minimum_size == Some(RUNTIME_MINIMUM_SIZE);
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
                    let native_minimum_cleared = native_minimum_approximately(native, Size::ZERO);
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
                        popover,
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
                                self.phase = ProbePhase::PopoverSoakSettling {
                                    stage: PopoverSoakSampleStage::Baseline,
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
                                self.phase = ProbePhase::PopoverSoakSettling {
                                    stage: PopoverSoakSampleStage::Final,
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
                                let cycle = self.context_menu_deactivation_dismiss_cycles_completed;
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
            ProbePhase::PopoverSoakSettling {
                stage,
                deadline,
                samples,
            } => {
                if now < deadline {
                    self.phase = ProbePhase::PopoverSoakSettling {
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
                        PopoverSoakSampleStage::Baseline => {
                            self.context_menu_soak_baseline_memory = Some(memory);
                            let interaction = ContextInteraction::SoakCommand;
                            let cycle = self.context_menu_soak_cycles_completed;
                            self.phase = ProbePhase::ContextMenu {
                                interaction,
                                samples,
                            };
                            ProbeAction::ReopenContext { interaction, cycle }
                        }
                        PopoverSoakSampleStage::Final => {
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
            .saturating_sub(soak_baseline.resident_bytes) as f64
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
            context_menu_soak_reopen_delay_ms: CONTEXT_MENU_SOAK_REOPEN_DELAY.as_millis() as u64,
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
            max_context_popover_windows: self.max_context_popover_windows,
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
            "QUICKGUI_ACCEPTANCE_ERROR timeout_seconds={} phase={phase} window_commands_completed={} context_menu_open={} context_activation_scheduled={} context_menu_cycles_completed={} context_menu_outside_dismiss_cycles_completed={} context_menu_escape_dismiss_cycles_completed={} context_menu_soak_cycles_completed={} context_menu_deactivation_dismiss_cycles_completed={} context_menu_open_observed={} context_submenu_hover_observed={} context_menu_action_received={} context_menu_close_observed={} inactive_teardown_observed={} owner_window_focused={} framework_focus_active={} owner_key={} framework_first_responder={} context_popover_windows={} max_context_popover_windows={} native_mouse_actions={} mouse_trace={:?} failures={:?}",
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
            self.last_context_popover.owner_key,
            self.last_context_popover.framework_first_responder,
            self.last_context_popover.window_count(),
            self.max_context_popover_windows,
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
    if report.max_context_popover_windows < 1 || report.max_context_popover_windows > 2 {
        report.failures.push(format!(
            "max_context_popover_windows was {}, expected 1..=2",
            report.max_context_popover_windows
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
    if (report.final_native_width - report.expected_native_width).abs() > NATIVE_GEOMETRY_TOLERANCE
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
        report
            .failures
            .push("idle_frame_delta was 0; the verification frame was not presented".to_owned());
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
