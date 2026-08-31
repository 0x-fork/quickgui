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
        let restored = PopoverObservation {
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
            popover: restored,
            inactive_teardown_observed: false,
        };
        assert!(context_cycle_complete(
            ContextInteraction::Command,
            complete,
        ));
        assert!(!context_cycle_complete(
            ContextInteraction::Command,
            ContextCycleObservation {
                popover: PopoverObservation {
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
                popover: PopoverObservation {
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
                popover: PopoverObservation {
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
                popover: PopoverObservation {
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
                popover: PopoverObservation {
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
        let sample = current_process_memory().expect("current process memory should be readable");
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
