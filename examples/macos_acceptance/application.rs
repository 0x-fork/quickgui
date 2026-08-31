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
    Application, AsyncContextError, AsyncViewContext, CONTEXT_MENU_SUBMENU_HOVER_DELAY, Color,
    ContextMenuLayout, ContextMenuState, ElementId, Event, EventContext, FocusHandle, FrameMetrics,
    IntoElement, MouseButton, PopoverMenu, PopoverMenuItem, PopoverMenuItemState, Size, View,
    ViewContext, WindowKind, WindowState, div, native_view, popover_menu_key_bindings, text,
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
// Application activation handoff bugs are timing-sensitive. Exercise enough independent handoffs that
// a global mouse-monitor or window-focus race cannot pass this gate by winning once.
const CONTEXT_MENU_DEACTIVATION_DISMISS_CYCLES: usize = 8;
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
    let view_failed = Arc::clone(&failed);

    Application::new()
        .bind_keys(popover_menu_key_bindings())
        .on_window_closed(move |_, _| {
            // SAFETY: QuickGUI invokes lifecycle callbacks on AppKit's application thread.
            let detached = unsafe { callback_field.superview().is_none() };
            if !detached {
                // Child popover teardown is an application-wide close event too. The native
                // field must remain attached until the owner window itself is destroyed.
                return;
            }
            callback_lifecycle.store(1, Ordering::Relaxed);
            emit_line("QUICKGUI_LIFECYCLE_RESULT {\"native_child_detached\":true,\"passed\":true}");
        })
        .run(move |cx| {
            cx.open_window(
                quickgui::WindowOptions::new("QuickGUI — macOS 0.1 acceptance")
                    .size(900.0, 640.0)
                    .position(80.0, 80.0)
                    .window_kind(WindowKind::Floating),
                MacAcceptanceView::new(field, view_failed),
            );
        })?;

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
