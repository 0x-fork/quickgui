use std::{
    any::Any,
    cell::RefCell,
    collections::{BTreeMap, HashMap, HashSet, VecDeque},
    fmt,
    path::PathBuf,
    rc::Rc,
    sync::{
        Arc, Condvar, LazyLock, Mutex,
        atomic::{AtomicU32, Ordering},
    },
    task::{Context, Poll, Waker},
    time::Duration,
};

use napi::{
    Env, Error, Result, Task,
    bindgen_prelude::{AsyncTask, Buffer, Function},
};
use napi_derive::napi;
use quickgui::{
    AccessibilityRole, Accordion, AccordionItem, AnchorPlacement, AppInfo, AppPaths, AppRegion,
    AppRunStatus, AppRunner, AppRunnerWaker, Application as QuickGuiApplication, BoxShadow,
    Checkbox, Collapsible, Color, CursorGrabMode, CursorStyle, Dialog as CoreDialog, DialogKind,
    DisplayId, Element, ElementId, Field, Fieldset, FollowMode, FontWeight, Image, Insets,
    IntoElement, ListAlignment, ListState, MAX_BOX_SHADOWS_PER_ELEMENT, MacOsVibrancy,
    MacOsVisualEffectState, Markdown, MarkdownStyle, PerformanceProfile, Point, PointerPhase,
    Popover, QuitMode, Radio, RadioGroup, Svg, Switch, SystemPopover, TERMINAL_ANSI_COLOR_COUNT,
    Tab, Tabs, TaskbarProgressState, Terminal, TerminalOptions, TerminalPaddingColor,
    TerminalStatus, TerminalStyle, TerminalTheme, TextAlign, TitleBarStyle, ToggleState, Tooltip,
    Transition, View, ViewContext, WindowAppearance, WindowBackgroundAppearance, WindowHandle,
    WindowKind, WindowOptions, button, div, svg as svg_element, text, text_area, text_input,
};
use quickgui::{Event, EventContext};
#[cfg(target_os = "macos")]
use quickgui::{
    MacEmbeddedView, MacSwiftUiHost, SwiftUiButton, SwiftUiButtonBorderShape, SwiftUiButtonRole,
    SwiftUiButtonStyle, SwiftUiControlSize, SwiftUiElement, SwiftUiLabelStyle, SwiftUiModifier,
    SwiftUiPopover, SwiftUiPopoverArrowEdge, SwiftUiPopoverAttachmentAnchor, SwiftUiQuickGuiHost,
    native_view,
};

mod dialog;
// napi-rs omits exported registration glue from lib-test builds, so these binding-only modules
// appear unreachable to rustc even though every item is reachable from the production addon.
#[cfg_attr(test, allow(dead_code))]
mod integrations;
#[cfg_attr(test, allow(dead_code))]
mod system;

pub use dialog::{
    NativeDialogButton, NativeDialogOptions, NativeFileDialogFilter, NativeOpenDialogOptions,
    NativeSaveDialogOptions,
};
use dialog::{
    PendingDialog, native_dialog_configuration, native_open_dialog_options,
    native_save_dialog_options,
};

const PROTOCOL_MAGIC: &[u8; 4] = b"QGMB";
const PROTOCOL_VERSION: u16 = 19;
const ROOT_NODE: u32 = 0;
const ROOT_ELEMENT_ID: u64 = u64::MAX - 1;
const MAX_BATCH_BYTES: usize = 16 * 1024 * 1024;
const MAX_MUTATIONS: usize = 131_072;
const MAX_NODES: usize = 262_144;
const MAX_TREE_DEPTH: usize = 512;
const MAX_STRING_BYTES: usize = 1024 * 1024;
const MAX_QUEUED_EVENTS: usize = 8_192;
const MAX_HOST_COMMANDS: usize = 8_192;
const MAX_WINDOWS: usize = 256;
const NO_ANCHOR: u32 = u32::MAX;
/// Longest compound scope key or item value accepted from one component part property.
const MAX_COMPONENT_VALUE_BYTES: usize = 256;
/// Longest tooltip label retained from one `tooltip` property.
const MAX_TOOLTIP_TEXT_BYTES: usize = 1_024;

mod property {
    pub const DISPLAY: u16 = 1;
    pub const FLEX_DIRECTION: u16 = 2;
    pub const FLEX_WRAP: u16 = 3;
    pub const FLEX_GROW: u16 = 4;
    pub const FLEX_SHRINK: u16 = 5;
    pub const FLEX_BASIS: u16 = 6;
    pub const ALIGN_ITEMS: u16 = 7;
    pub const ALIGN_SELF: u16 = 8;
    pub const JUSTIFY_CONTENT: u16 = 9;
    pub const ALIGN_CONTENT: u16 = 10;
    pub const GAP: u16 = 11;
    pub const COLUMN_GAP: u16 = 12;
    pub const ROW_GAP: u16 = 13;
    pub const WIDTH: u16 = 14;
    pub const HEIGHT: u16 = 15;
    pub const MIN_WIDTH: u16 = 16;
    pub const MIN_HEIGHT: u16 = 17;
    pub const MAX_WIDTH: u16 = 18;
    pub const MAX_HEIGHT: u16 = 19;
    pub const PADDING: u16 = 20;
    pub const PADDING_TOP: u16 = 21;
    pub const PADDING_RIGHT: u16 = 22;
    pub const PADDING_BOTTOM: u16 = 23;
    pub const PADDING_LEFT: u16 = 24;
    pub const MARGIN: u16 = 25;
    pub const MARGIN_TOP: u16 = 26;
    pub const MARGIN_RIGHT: u16 = 27;
    pub const MARGIN_BOTTOM: u16 = 28;
    pub const MARGIN_LEFT: u16 = 29;
    pub const BACKGROUND_COLOR: u16 = 30;
    pub const COLOR: u16 = 31;
    pub const OPACITY: u16 = 32;
    pub const BORDER_WIDTH: u16 = 33;
    pub const BORDER_COLOR: u16 = 34;
    pub const BORDER_RADIUS: u16 = 35;
    pub const FONT_SIZE: u16 = 36;
    pub const FONT_WEIGHT: u16 = 37;
    pub const LINE_HEIGHT: u16 = 38;
    pub const TEXT_ALIGN: u16 = 39;
    pub const WHITE_SPACE: u16 = 40;
    pub const TEXT_OVERFLOW: u16 = 41;
    pub const LINE_CLAMP: u16 = 42;
    pub const OVERFLOW: u16 = 43;
    pub const OVERFLOW_X: u16 = 44;
    pub const OVERFLOW_Y: u16 = 45;
    pub const CURSOR: u16 = 46;
    pub const APP_REGION: u16 = 47;
    pub const DISABLED: u16 = 48;
    pub const ACCESSIBILITY_LABEL: u16 = 49;
    pub const ROLE: u16 = 50;
    pub const TAB_INDEX: u16 = 51;
    pub const POSITION: u16 = 52;
    pub const TOP: u16 = 53;
    pub const RIGHT: u16 = 54;
    pub const BOTTOM: u16 = 55;
    pub const LEFT: u16 = 56;
    pub const USER_SELECT: u16 = 57;
    pub const CLICK_LISTENER: u16 = 58;
    pub const HOVER_LISTENER: u16 = 59;
    pub const VISIBILITY: u16 = 60;
    pub const ASPECT_RATIO: u16 = 61;
    pub const VALUE: u16 = 62;
    pub const PLACEHOLDER: u16 = 63;
    pub const MULTILINE: u16 = 64;
    pub const INPUT_LISTENER: u16 = 65;
    pub const SUBMIT_LISTENER: u16 = 66;
    pub const STREAMING: u16 = 67;
    pub const MARKDOWN_CODE_BACKGROUND: u16 = 68;
    pub const MARKDOWN_BORDER_COLOR: u16 = 69;
    pub const MARKDOWN_MUTED_COLOR: u16 = 70;
    pub const MARKDOWN_LINK_COLOR: u16 = 71;
    pub const MARKDOWN_CODE_TEXT_COLOR: u16 = 72;
    pub const MARKDOWN_BLOCK_GAP: u16 = 73;
    pub const MARKDOWN_CODE_FONT_SIZE: u16 = 74;
    pub const SCROLL_TO_END_REVISION: u16 = 75;
    pub const PASSWORD: u16 = 76;
    pub const ESTIMATED_ITEM_HEIGHT: u16 = 77;
    pub const OVERSCAN: u16 = 78;
    pub const LIST_ALIGNMENT: u16 = 79;
    pub const FOLLOW_MODE: u16 = 80;
    pub const ANCHOR_TARGET: u16 = 81;
    pub const ANCHOR_PLACEMENT: u16 = 82;
    pub const ANCHOR_GAP: u16 = 83;
    pub const VIEWPORT_MARGIN: u16 = 84;
    pub const DISMISS_ON_ESCAPE: u16 = 85;
    pub const DISMISS_ON_POINTER_OUTSIDE: u16 = 86;
    pub const DISMISS_LISTENER: u16 = 87;
    pub const TERMINAL_PROGRAM: u16 = 88;
    pub const TERMINAL_ARGUMENTS: u16 = 89;
    pub const TERMINAL_WORKING_DIRECTORY: u16 = 90;
    pub const TERMINAL_ENVIRONMENT: u16 = 91;
    pub const TERMINAL_SCROLLBACK: u16 = 92;
    pub const TERMINAL_STATUS_LISTENER: u16 = 93;
    pub const HOVER_BACKGROUND_COLOR: u16 = 94;
    pub const HOVER_COLOR: u16 = 95;
    pub const ACTIVE_BACKGROUND_COLOR: u16 = 96;
    pub const ACTIVE_COLOR: u16 = 97;
    pub const TRANSITION: u16 = 98;
    pub const POINTER_LISTENER: u16 = 99;
    pub const FOCUS_ON_POINTER: u16 = 100;
    pub const FONT_FAMILY: u16 = 101;
    pub const TERMINAL_PALETTE: u16 = 102;
    pub const TERMINAL_CURSOR_COLOR: u16 = 103;
    pub const HIT_SLOP: u16 = 104;
    pub const HIT_SLOP_TOP: u16 = 105;
    pub const HIT_SLOP_RIGHT: u16 = 106;
    pub const HIT_SLOP_BOTTOM: u16 = 107;
    pub const HIT_SLOP_LEFT: u16 = 108;
    pub const OVERLAY: u16 = 109;
    pub const FOCUS_TRAP: u16 = 110;
    pub const RESTORE_PREVIOUS_FOCUS: u16 = 111;
    pub const AUTO_FOCUS: u16 = 112;
    pub const ACCESSIBILITY_MODAL: u16 = 113;
    pub const TERMINAL_PADDING_COLOR: u16 = 114;
    pub const TERMINAL_FONT_THICKEN: u16 = 115;
    pub const SWIFT_UI_SYSTEM_IMAGE: u16 = 116;
    pub const SWIFT_UI_BUTTON_STYLE: u16 = 117;
    pub const SWIFT_UI_CONTROL_SIZE: u16 = 118;
    pub const SWIFT_UI_MATCH_CONTENTS_HORIZONTAL: u16 = 119;
    pub const SWIFT_UI_MATCH_CONTENTS_VERTICAL: u16 = 120;
    pub const SWIFT_UI_TARGET: u16 = 121;
    pub const SWIFT_UI_TEST_ID: u16 = 122;
    pub const SWIFT_UI_MODIFIERS: u16 = 123;
    pub const SWIFT_UI_EMBEDDED_WINDOW: u16 = 124;
    pub const SWIFT_UI_IS_PRESENTED: u16 = 125;
    pub const SWIFT_UI_ATTACHMENT_ANCHOR: u16 = 126;
    pub const SWIFT_UI_ARROW_EDGE: u16 = 127;
    pub const BORDER_TOP_WIDTH: u16 = 129;
    pub const BORDER_RIGHT_WIDTH: u16 = 130;
    pub const BORDER_BOTTOM_WIDTH: u16 = 131;
    pub const BORDER_LEFT_WIDTH: u16 = 132;
    pub const BOX_SHADOW: u16 = 133;
    pub const PART: u16 = 134;
    pub const CHECKED: u16 = 135;
    pub const INDETERMINATE: u16 = 136;
    pub const SCOPE: u16 = 137;
    pub const PART_VALUE: u16 = 138;
    pub const ACTIVE_VALUE: u16 = 139;
    pub const ORIENTATION: u16 = 140;
    pub const ACTIVATE_ON_FOCUS: u16 = 141;
    pub const LOOP_FOCUS: u16 = 142;
    pub const KEEP_MOUNTED: u16 = 143;
    pub const OPEN: u16 = 144;
    pub const ITEM_INDEX: u16 = 145;
    pub const HEADING_LEVEL: u16 = 146;
    pub const REQUIRED: u16 = 147;
    pub const INVALID: u16 = 148;
    pub const VALIDATION_MESSAGE: u16 = 149;
    pub const TOUCHED: u16 = 150;
    pub const DIRTY: u16 = 151;
    pub const FILLED: u16 = 152;
    pub const TOOLTIP: u16 = 153;
    pub const TOOLTIP_PLACEMENT: u16 = 154;
    pub const TOOLTIP_DELAY: u16 = 155;
    pub const TOOLTIP_GAP: u16 = 156;
    pub const TOOLTIP_VIEWPORT_MARGIN: u16 = 157;
    pub const VARIANT: u16 = 158;
    pub const LAST: u16 = VARIANT;
}

#[derive(Default)]
#[napi(object)]
pub struct NativeImageSource {
    /// Encoded image bytes, or raw RGBA8 when width and height are both supplied.
    pub data: Option<Buffer>,
    /// Image path used when data is omitted.
    pub path: Option<String>,
    pub width: Option<u32>,
    pub height: Option<u32>,
}

impl Clone for NativeImageSource {
    fn clone(&self) -> Self {
        Self {
            data: self
                .data
                .as_ref()
                .map(|data| Buffer::from(data.as_ref().to_vec())),
            path: self.path.clone(),
            width: self.width,
            height: self.height,
        }
    }
}

#[derive(Default)]
#[napi(object)]
pub struct NativeAppOptions {
    pub name: Option<String>,
    pub version: Option<String>,
    pub identifier: Option<String>,
    pub resource_dir: Option<String>,
    pub config_dir: Option<String>,
    pub data_dir: Option<String>,
    pub local_data_dir: Option<String>,
    pub cache_dir: Option<String>,
    pub log_dir: Option<String>,
    pub runtime_dir: Option<String>,
    pub temp_dir: Option<String>,
    /// `default`, `last-window-closed`, or `explicit`.
    pub quit_mode: Option<String>,
    /// OpenType font files embedded by the JavaScript host and registered by the Rust core.
    pub font_data: Option<Vec<Buffer>>,
}

#[derive(Clone, Default)]
#[napi(object)]
pub struct NativeWindowOptions {
    pub title: Option<String>,
    pub width: Option<f64>,
    pub height: Option<f64>,
    pub x: Option<f64>,
    pub y: Option<f64>,
    /// `normal`, `maximized`, or `fullscreen`.
    pub initial_state: Option<String>,
    pub display_id: Option<String>,
    /// Set to false to remove QuickGUI's default minimum size.
    pub minimum_size_enabled: Option<bool>,
    pub minimum_width: Option<f64>,
    pub minimum_height: Option<f64>,
    pub maximum_width: Option<f64>,
    pub maximum_height: Option<f64>,
    pub represented_file: Option<String>,
    pub document_edited: Option<bool>,
    pub tabbing_identifier: Option<String>,
    pub background: Option<u32>,
    pub performance_profile: Option<String>,
    pub appearance: Option<String>,
    pub vibrancy: Option<String>,
    pub visual_effect_state: Option<String>,
    pub title_bar_style: Option<String>,
    pub kind: Option<String>,
    pub focus: Option<bool>,
    pub focusable: Option<bool>,
    pub show: Option<bool>,
    pub movable: Option<bool>,
    pub resizable: Option<bool>,
    pub minimizable: Option<bool>,
    pub maximizable: Option<bool>,
    pub closable: Option<bool>,
    pub decorated: Option<bool>,
    pub shadow: Option<bool>,
    pub content_protected: Option<bool>,
    pub window_level: Option<String>,
    pub skip_taskbar: Option<bool>,
    pub visible_on_all_workspaces: Option<bool>,
    pub opacity: Option<f64>,
    pub icon: Option<NativeImageSource>,
    pub taskbar_progress_state: Option<String>,
    pub taskbar_progress: Option<f64>,
    pub taskbar_overlay_icon: Option<NativeImageSource>,
    pub taskbar_overlay_description: Option<String>,
    pub cursor_visible: Option<bool>,
    pub cursor_grab: Option<String>,
    pub cursor_hit_test: Option<bool>,
    pub cursor_x: Option<f64>,
    pub cursor_y: Option<f64>,
    /// Bounded JSON encoding of a per-window native menu. Omitted windows inherit the app menu.
    pub menu: Option<String>,
    /// Persisted geometry and display identity captured with `window.getRestoreState()`.
    ///
    /// The core re-validates every field, so a stale value can never place a window off every
    /// connected display.
    pub restore_state: Option<system::NativeWindowRestoreState>,
    pub line_scroll_pixels: Option<f64>,
    pub key_sequence_timeout_ms: Option<f64>,
    pub reduce_motion: Option<bool>,
    pub traffic_light_x: Option<f64>,
    pub traffic_light_y: Option<f64>,
    pub transparent: Option<bool>,
    pub blur: Option<bool>,
    pub popover_placement: Option<String>,
    pub popover_gap: Option<f64>,
    pub popover_offset_x: Option<f64>,
    pub popover_offset_y: Option<f64>,
    pub popover_viewport_margin: Option<f64>,
    pub popover_dismiss_on_escape: Option<bool>,
    pub popover_dismiss_on_pointer_outside: Option<bool>,
    pub popover_grab: Option<bool>,
    pub popover_accepts_key_focus: Option<bool>,
}

#[napi(object)]
pub struct NativeEvent {
    pub kind: String,
    pub window: u32,
    pub target: u32,
    pub value: Option<String>,
    pub paths: Option<Vec<String>>,
    pub data: Option<Buffer>,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub error: Option<String>,
}

impl Clone for NativeEvent {
    fn clone(&self) -> Self {
        Self {
            kind: self.kind.clone(),
            window: self.window,
            target: self.target,
            value: self.value.clone(),
            paths: self.paths.clone(),
            data: self
                .data
                .as_ref()
                .map(|data| Buffer::from(data.as_ref().to_vec())),
            width: self.width,
            height: self.height,
            error: self.error.clone(),
        }
    }
}

#[derive(Clone)]
#[napi(object)]
pub struct HostedAppUpdate {
    pub events: Vec<NativeEvent>,
    pub exit_code: Option<i32>,
}

struct HostReply<T> {
    app: u32,
    value: Mutex<Option<std::result::Result<T, String>>>,
    ready: Condvar,
}

impl<T> HostReply<T> {
    fn new(app: u32) -> Self {
        Self {
            app,
            value: Mutex::new(None),
            ready: Condvar::new(),
        }
    }

    fn complete(&self, value: std::result::Result<T, String>) {
        *lock(&self.value) = Some(value);
        self.ready.notify_all();
    }

    fn wait(&self) -> std::result::Result<T, String> {
        let mut value = lock(&self.value);
        while value.is_none() {
            if let Some(error) = HOST.failure_for(self.app) {
                return Err(error);
            }
            value = self
                .ready
                .wait_timeout(value, Duration::from_millis(50))
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .0;
        }
        value
            .take()
            .expect("a completed QuickGUI host reply must contain a value")
    }
}

enum HostCommand {
    CreateApp {
        app: u32,
        options: NativeAppOptions,
    },
    CreateWindow {
        app: u32,
        window: u32,
        options: NativeWindowOptions,
        initial_batch: Vec<u8>,
    },
    CreateSystemPopover {
        app: u32,
        window: u32,
        parent: u32,
        anchor: u32,
        options: NativeWindowOptions,
        initial_batch: Vec<u8>,
    },
    #[cfg(target_os = "macos")]
    CreateEmbeddedView {
        app: u32,
        window: u32,
        parent: u32,
        match_horizontal: bool,
        match_vertical: bool,
        options: NativeWindowOptions,
        initial_batch: Vec<u8>,
    },
    ApplyBatch {
        app: u32,
        window: u32,
        batch: Vec<u8>,
    },
    Mutation {
        app: u32,
        command: system::SystemCommand,
    },
    CloseWindow {
        app: u32,
        window: u32,
    },
    FocusNode {
        app: u32,
        window: u32,
        node: u32,
    },
    ShowAlertDialog {
        app: u32,
        window: Option<u32>,
        request: u32,
        options: NativeDialogOptions,
        reply: Arc<HostReply<()>>,
    },
    ShowOpenDialog {
        app: u32,
        window: Option<u32>,
        request: u32,
        options: NativeOpenDialogOptions,
        reply: Arc<HostReply<()>>,
    },
    ShowSaveDialog {
        app: u32,
        window: Option<u32>,
        request: u32,
        options: NativeSaveDialogOptions,
        reply: Arc<HostReply<()>>,
    },
    #[cfg_attr(test, allow(dead_code))]
    System {
        app: u32,
        command: system::SystemCommand,
        reply: Arc<HostReply<system::SystemCommandResult>>,
    },
    PrepareApp {
        app: u32,
        reply: Arc<HostReply<()>>,
    },
    DestroyApp {
        app: u32,
    },
}

#[derive(Default)]
struct HostState {
    running: bool,
    app: Option<u32>,
    commands: VecDeque<HostCommand>,
    events: VecDeque<NativeEvent>,
    exit_code: Option<i32>,
    failure: Option<String>,
    waker: Option<AppRunnerWaker>,
}

struct HostCoordinator {
    next_app: AtomicU32,
    next_window: AtomicU32,
    state: Mutex<HostState>,
    changed: Condvar,
}

impl HostCoordinator {
    fn new() -> Self {
        Self {
            next_app: AtomicU32::new(1),
            next_window: AtomicU32::new(1),
            state: Mutex::new(HostState::default()),
            changed: Condvar::new(),
        }
    }

    fn allocate_app(&self) -> std::result::Result<u32, String> {
        let app = self
            .next_app
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |id| {
                id.checked_add(1).filter(|next| *next != 0)
            })
            .map_err(|_| "QuickGUI hosted app id space exhausted".to_owned())?
            .max(1);
        Ok(app)
    }

    fn allocate_window(&self) -> std::result::Result<u32, String> {
        self.next_window
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |id| {
                id.checked_add(1).filter(|next| *next != 0)
            })
            .map(|id| id.max(1))
            .map_err(|_| "QuickGUI hosted window id space exhausted".to_owned())
    }

    fn enqueue(&self, command: HostCommand) -> std::result::Result<(), String> {
        let waker = {
            let mut state = lock(&self.state);
            if let Some(error) = state.failure.clone() {
                return Err(error);
            }
            if state.commands.len() >= MAX_HOST_COMMANDS {
                return Err("QuickGUI host command queue is full".to_owned());
            }
            state.commands.push_back(command);
            state.waker.clone()
        };
        self.changed.notify_all();
        if let Some(waker) = waker {
            waker.wake();
        }
        Ok(())
    }

    fn begin(&self) -> std::result::Result<(), String> {
        let mut state = lock(&self.state);
        if state.running {
            return Err("the QuickGUI native host is already running".to_owned());
        }
        state.running = true;
        Ok(())
    }

    fn finish(&self) {
        let mut state = lock(&self.state);
        state.running = false;
        state.waker = None;
        self.changed.notify_all();
    }

    fn wait_for_commands(&self) -> std::result::Result<VecDeque<HostCommand>, String> {
        let mut state = lock(&self.state);
        while state.commands.is_empty() && state.failure.is_none() {
            state = wait(&self.changed, state);
        }
        if let Some(error) = state.failure.clone() {
            return Err(error);
        }
        Ok(std::mem::take(&mut state.commands))
    }

    fn take_commands(&self) -> std::result::Result<VecDeque<HostCommand>, String> {
        let mut state = lock(&self.state);
        if let Some(error) = state.failure.clone() {
            return Err(error);
        }
        Ok(std::mem::take(&mut state.commands))
    }

    fn set_app(&self, app: u32) -> std::result::Result<(), String> {
        let mut state = lock(&self.state);
        if state.app.is_some() {
            return Err("a QuickGUI hosted app is already active".to_owned());
        }
        state.app = Some(app);
        state.events.clear();
        state.exit_code = None;
        state.failure = None;
        Ok(())
    }

    fn set_waker(&self, waker: AppRunnerWaker) {
        lock(&self.state).waker = Some(waker);
    }

    fn failure_for(&self, app: u32) -> Option<String> {
        let state = lock(&self.state);
        if state.app != Some(app) {
            return Some(format!("unknown QuickGUI hosted app {app}"));
        }
        state.failure.clone().or_else(|| {
            (!state.running && state.exit_code.is_some())
                .then(|| "the QuickGUI hosted application has exited".to_owned())
        })
    }

    fn publish_events(&self, events: impl IntoIterator<Item = NativeEvent>) {
        let mut state = lock(&self.state);
        for event in events {
            if state.events.len() >= MAX_QUEUED_EVENTS {
                break;
            }
            state.events.push_back(event);
        }
        drop(state);
        self.changed.notify_all();
    }

    fn publish_exit(&self, code: i32) {
        let mut state = lock(&self.state);
        state.exit_code = Some(code.max(0));
        state.waker = None;
        drop(state);
        self.changed.notify_all();
    }

    fn fail(&self, error: String) {
        let waker = {
            let mut state = lock(&self.state);
            if state.failure.is_none() {
                state.failure = Some(error);
            }
            state.waker.clone()
        };
        self.changed.notify_all();
        if let Some(waker) = waker {
            waker.wake();
        }
    }

    fn wait_for_update(&self, app: u32) -> std::result::Result<HostedAppUpdate, String> {
        let mut state = lock(&self.state);
        loop {
            if state.app != Some(app) {
                return Err(format!("unknown QuickGUI hosted app {app}"));
            }
            if let Some(error) = state.failure.clone() {
                return Err(error);
            }
            if !state.events.is_empty() || state.exit_code.is_some() {
                return Ok(HostedAppUpdate {
                    events: state.events.drain(..).collect(),
                    exit_code: state.exit_code,
                });
            }
            state = wait(&self.changed, state);
        }
    }
}

static HOST: LazyLock<HostCoordinator> = LazyLock::new(HostCoordinator::new);

fn lock<T>(mutex: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    mutex
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

fn wait<'a, T>(
    condvar: &Condvar,
    guard: std::sync::MutexGuard<'a, T>,
) -> std::sync::MutexGuard<'a, T> {
    condvar
        .wait(guard)
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

mod api;
mod runtime;
mod tree;
mod view;

pub use api::*;

use runtime::*;
use tree::*;
use view::*;

#[cfg(test)]
mod tests;
