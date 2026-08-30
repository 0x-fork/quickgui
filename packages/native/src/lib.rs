use std::{
    any::Any,
    cell::RefCell,
    collections::{BTreeMap, HashMap, VecDeque},
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
    AccessibilityRole, AnchorPlacement, AppConfig, AppInfo, AppPaths, AppRegion, AppRunStatus,
    AppRunner, AppRunnerWaker, Application as QuickGuiApplication, Color, CursorGrabMode,
    CursorStyle, DisplayId, Element, ElementId, FollowMode, FontWeight, Image, IntoElement,
    ListAlignment, ListState, Markdown, MarkdownStyle, PerformanceProfile, Point, PointerPhase,
    Popover, QuitMode, Svg, SystemPopover, TERMINAL_ANSI_COLOR_COUNT, TaskbarProgressState,
    Terminal, TerminalOptions, TerminalStatus, TerminalStyle, TerminalTheme, TextAlign,
    TitleBarStyle, Transition, View, ViewContext, WindowAppearance, WindowBackgroundAppearance,
    WindowHandle, WindowKind, WindowLevel, button, div,
    svg as svg_element, text, text_area, text_input,
};

mod dialog;
mod integrations;
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
const PROTOCOL_VERSION: u16 = 13;
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
    pub const LAST: u16 = ACCESSIBILITY_MODAL;
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

struct SyncReply<T> {
    value: Mutex<Option<std::result::Result<T, String>>>,
    ready: Condvar,
}

impl<T> SyncReply<T> {
    fn new() -> Self {
        Self {
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
            value = wait(&self.ready, value);
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
        reply: Arc<SyncReply<()>>,
    },
    CreateWindow {
        app: u32,
        options: NativeWindowOptions,
        initial_batch: Vec<u8>,
        reply: Arc<SyncReply<u32>>,
    },
    CreateSystemPopover {
        app: u32,
        parent: u32,
        anchor: u32,
        options: NativeWindowOptions,
        initial_batch: Vec<u8>,
        reply: Arc<SyncReply<u32>>,
    },
    ApplyBatch {
        app: u32,
        window: u32,
        batch: Vec<u8>,
    },
    CloseWindow {
        app: u32,
        window: u32,
        reply: Arc<SyncReply<bool>>,
    },
    FocusNode {
        app: u32,
        window: u32,
        node: u32,
        reply: Arc<SyncReply<bool>>,
    },
    ShowAlertDialog {
        app: u32,
        window: Option<u32>,
        request: u32,
        options: NativeDialogOptions,
        reply: Arc<SyncReply<()>>,
    },
    ShowOpenDialog {
        app: u32,
        window: Option<u32>,
        request: u32,
        options: NativeOpenDialogOptions,
        reply: Arc<SyncReply<()>>,
    },
    ShowSaveDialog {
        app: u32,
        window: Option<u32>,
        request: u32,
        options: NativeSaveDialogOptions,
        reply: Arc<SyncReply<()>>,
    },
    System {
        app: u32,
        command: system::SystemCommand,
        reply: Arc<SyncReply<system::SystemCommandResult>>,
    },
    PrepareApp {
        app: u32,
        reply: Arc<SyncReply<()>>,
    },
    IsAppReady {
        app: u32,
        reply: Arc<SyncReply<bool>>,
    },
    DestroyApp {
        app: u32,
        reply: Arc<SyncReply<bool>>,
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
    state: Mutex<HostState>,
    changed: Condvar,
}

impl HostCoordinator {
    fn new() -> Self {
        Self {
            next_app: AtomicU32::new(1),
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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum NodeTag {
    Root,
    View,
    Button,
    Text,
    Sentinel,
    Input,
    Markdown,
    VirtualList,
    Terminal,
    Svg,
}

impl NodeTag {
    fn decode(value: u8) -> std::result::Result<Self, ProtocolError> {
        match value {
            1 => Ok(Self::View),
            2 => Ok(Self::Button),
            3 => Ok(Self::Text),
            4 => Ok(Self::Sentinel),
            5 => Ok(Self::Input),
            6 => Ok(Self::Markdown),
            7 => Ok(Self::VirtualList),
            8 => Ok(Self::Terminal),
            9 => Ok(Self::Svg),
            _ => Err(ProtocolError::new(format!("unknown node tag {value}"))),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
enum PropertyValue {
    Bool(bool),
    Number(f32),
    Color(u32),
    String(Arc<str>),
}

#[derive(Clone, Debug)]
struct NativeNode {
    tag: NodeTag,
    parent: Option<u32>,
    children: Vec<u32>,
    text: Arc<str>,
    properties: Vec<(u16, PropertyValue)>,
}

impl NativeNode {
    fn new(tag: NodeTag) -> Self {
        Self {
            tag,
            parent: None,
            children: Vec::new(),
            text: Arc::from(""),
            properties: Vec::new(),
        }
    }

    fn property(&self, key: u16) -> Option<&PropertyValue> {
        self.properties
            .binary_search_by_key(&key, |(property, _)| *property)
            .ok()
            .map(|index| &self.properties[index].1)
    }

    fn number(&self, key: u16) -> Option<f32> {
        match self.property(key) {
            Some(PropertyValue::Number(value)) if value.is_finite() => Some(*value),
            _ => None,
        }
    }

    fn boolean(&self, key: u16) -> Option<bool> {
        match self.property(key) {
            Some(PropertyValue::Bool(value)) => Some(*value),
            _ => None,
        }
    }

    fn string(&self, key: u16) -> Option<&str> {
        match self.property(key) {
            Some(PropertyValue::String(value)) => Some(value),
            _ => None,
        }
    }

    fn color(&self, key: u16) -> Option<Color> {
        match self.property(key) {
            Some(PropertyValue::Color(value)) => Some(unpack_color(*value)),
            _ => None,
        }
    }

    fn set_property(&mut self, key: u16, value: Option<PropertyValue>) {
        match self
            .properties
            .binary_search_by_key(&key, |(property, _)| *property)
        {
            Ok(index) => match value {
                Some(value) => self.properties[index].1 = value,
                None => {
                    self.properties.remove(index);
                }
            },
            Err(index) => {
                if let Some(value) = value {
                    self.properties.insert(index, (key, value));
                }
            }
        }
    }
}

#[derive(Clone, Debug)]
struct NativeTree {
    nodes: HashMap<u32, NativeNode>,
    revision: u32,
}

impl Default for NativeTree {
    fn default() -> Self {
        let mut nodes = HashMap::with_capacity(64);
        nodes.insert(ROOT_NODE, NativeNode::new(NodeTag::Root));
        Self { nodes, revision: 0 }
    }
}

#[derive(Clone, Debug)]
enum Mutation {
    Create {
        id: u32,
        tag: NodeTag,
        text: Arc<str>,
    },
    SetProperty {
        id: u32,
        key: u16,
        value: Option<PropertyValue>,
    },
    ReplaceText {
        id: u32,
        text: Arc<str>,
    },
    Insert {
        parent: u32,
        child: u32,
        before: Option<u32>,
    },
    Remove {
        parent: u32,
        child: u32,
    },
    Cleanup {
        parent: u32,
        children: Vec<u32>,
    },
}

struct TreeTransaction<'a> {
    base: &'a NativeTree,
    overlay: HashMap<u32, Option<NativeNode>>,
}

impl<'a> TreeTransaction<'a> {
    fn new(base: &'a NativeTree) -> Self {
        Self {
            base,
            overlay: HashMap::new(),
        }
    }

    fn node(&self, id: u32) -> Option<&NativeNode> {
        match self.overlay.get(&id) {
            Some(node) => node.as_ref(),
            None => self.base.nodes.get(&id),
        }
    }

    fn edit(&mut self, id: u32) -> std::result::Result<&mut NativeNode, ProtocolError> {
        if !self.overlay.contains_key(&id) {
            let node = self
                .base
                .nodes
                .get(&id)
                .cloned()
                .ok_or_else(|| ProtocolError::new(format!("node {id} does not exist")))?;
            self.overlay.insert(id, Some(node));
        }
        self.overlay
            .get_mut(&id)
            .and_then(Option::as_mut)
            .ok_or_else(|| ProtocolError::new(format!("node {id} was already removed")))
    }

    fn create(
        &mut self,
        id: u32,
        tag: NodeTag,
        text: Arc<str>,
    ) -> std::result::Result<(), ProtocolError> {
        if id == ROOT_NODE {
            return Err(ProtocolError::new("node id 0 is reserved for the root"));
        }
        if self.node(id).is_some() {
            return Err(ProtocolError::new(format!("node {id} already exists")));
        }
        let mut node = NativeNode::new(tag);
        node.text = text;
        self.overlay.insert(id, Some(node));
        Ok(())
    }

    fn set_property(
        &mut self,
        id: u32,
        key: u16,
        value: Option<PropertyValue>,
    ) -> std::result::Result<(), ProtocolError> {
        if !(1..=property::LAST).contains(&key) {
            return Err(ProtocolError::new(format!("unknown property code {key}")));
        }
        self.edit(id)?.set_property(key, value);
        Ok(())
    }

    fn replace_text(&mut self, id: u32, text: Arc<str>) -> std::result::Result<(), ProtocolError> {
        let node = self.edit(id)?;
        if node.tag != NodeTag::Text {
            return Err(ProtocolError::new(format!("node {id} is not a text node")));
        }
        node.text = text;
        Ok(())
    }

    fn insert(
        &mut self,
        parent: u32,
        child: u32,
        before: Option<u32>,
    ) -> std::result::Result<(), ProtocolError> {
        if parent == child {
            return Err(ProtocolError::new("a node cannot contain itself"));
        }
        if self.node(parent).is_none() || self.node(child).is_none() {
            return Err(ProtocolError::new(format!(
                "cannot insert missing node {child} into {parent}"
            )));
        }
        if before == Some(child) && self.node(child).and_then(|node| node.parent) == Some(parent) {
            return Ok(());
        }
        if let Some(anchor) = before
            && self.node(anchor).and_then(|node| node.parent) != Some(parent)
        {
            return Err(ProtocolError::new(format!(
                "anchor {anchor} is not a child of {parent}"
            )));
        }

        let mut ancestor = Some(parent);
        for _ in 0..MAX_TREE_DEPTH {
            let Some(id) = ancestor else {
                break;
            };
            if id == child {
                return Err(ProtocolError::new("insertion would create a cycle"));
            }
            ancestor = self.node(id).and_then(|node| node.parent);
        }
        if ancestor.is_some() {
            return Err(ProtocolError::new(format!(
                "tree depth exceeds {MAX_TREE_DEPTH}"
            )));
        }

        if let Some(previous_parent) = self.node(child).and_then(|node| node.parent) {
            self.edit(previous_parent)?
                .children
                .retain(|candidate| *candidate != child);
        }
        let index = before
            .and_then(|anchor| {
                self.node(parent)?
                    .children
                    .iter()
                    .position(|candidate| *candidate == anchor)
            })
            .unwrap_or_else(|| self.node(parent).map_or(0, |node| node.children.len()));
        self.edit(parent)?.children.insert(index, child);
        self.edit(child)?.parent = Some(parent);
        Ok(())
    }

    fn remove(&mut self, parent: u32, child: u32) -> std::result::Result<(), ProtocolError> {
        if child == ROOT_NODE {
            return Err(ProtocolError::new("the root node cannot be removed"));
        }
        if self.node(child).and_then(|node| node.parent) != Some(parent) {
            return Err(ProtocolError::new(format!(
                "node {child} is not a child of {parent}"
            )));
        }
        self.edit(parent)?
            .children
            .retain(|candidate| *candidate != child);

        let mut pending = vec![child];
        let mut removed = 0_usize;
        while let Some(id) = pending.pop() {
            removed += 1;
            if removed > MAX_NODES {
                return Err(ProtocolError::new("removed subtree exceeds the node limit"));
            }
            let children = self
                .node(id)
                .map(|node| node.children.clone())
                .unwrap_or_default();
            pending.extend(children);
            self.overlay.insert(id, None);
        }
        Ok(())
    }

    fn finish(self) -> std::result::Result<HashMap<u32, Option<NativeNode>>, ProtocolError> {
        let removed = self.overlay.values().filter(|node| node.is_none()).count();
        let inserted = self
            .overlay
            .iter()
            .filter(|(id, node)| node.is_some() && !self.base.nodes.contains_key(id))
            .count();
        let final_len = self
            .base
            .nodes
            .len()
            .saturating_sub(removed)
            .saturating_add(inserted);
        if final_len > MAX_NODES + 1 {
            return Err(ProtocolError::new(format!(
                "tree cannot contain more than {MAX_NODES} application nodes"
            )));
        }
        Ok(self.overlay)
    }
}

fn commit_overlay(tree: &mut NativeTree, overlay: HashMap<u32, Option<NativeNode>>) -> u32 {
    for (id, node) in overlay {
        match node {
            Some(node) => {
                tree.nodes.insert(id, node);
            }
            None => {
                tree.nodes.remove(&id);
            }
        }
    }
    tree.revision = tree.revision.wrapping_add(1).max(1);
    tree.revision
}

fn apply_mutations(
    tree: &mut NativeTree,
    mutations: Vec<Mutation>,
) -> std::result::Result<u32, ProtocolError> {
    let mut transaction = TreeTransaction::new(tree);
    for mutation in mutations {
        match mutation {
            Mutation::Create { id, tag, text } => transaction.create(id, tag, text)?,
            Mutation::SetProperty { id, key, value } => transaction.set_property(id, key, value)?,
            Mutation::ReplaceText { id, text } => transaction.replace_text(id, text)?,
            Mutation::Insert {
                parent,
                child,
                before,
            } => transaction.insert(parent, child, before)?,
            Mutation::Remove { parent, child } => transaction.remove(parent, child)?,
            Mutation::Cleanup { parent, children } => {
                for child in children {
                    if transaction.node(child).is_some() {
                        transaction.remove(parent, child)?;
                    }
                }
            }
        }
    }
    let overlay = transaction.finish()?;
    Ok(commit_overlay(tree, overlay))
}

#[derive(Debug)]
struct ProtocolError(String);

impl ProtocolError {
    fn new(reason: impl Into<String>) -> Self {
        Self(reason.into())
    }
}

impl fmt::Display for ProtocolError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

struct Reader<'a> {
    bytes: &'a [u8],
    offset: usize,
}

impl<'a> Reader<'a> {
    fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, offset: 0 }
    }

    fn take(&mut self, length: usize) -> std::result::Result<&'a [u8], ProtocolError> {
        let end = self
            .offset
            .checked_add(length)
            .filter(|end| *end <= self.bytes.len())
            .ok_or_else(|| ProtocolError::new("truncated mutation batch"))?;
        let value = &self.bytes[self.offset..end];
        self.offset = end;
        Ok(value)
    }

    fn u8(&mut self) -> std::result::Result<u8, ProtocolError> {
        Ok(self.take(1)?[0])
    }

    fn u16(&mut self) -> std::result::Result<u16, ProtocolError> {
        Ok(u16::from_le_bytes(self.take(2)?.try_into().unwrap()))
    }

    fn u32(&mut self) -> std::result::Result<u32, ProtocolError> {
        Ok(u32::from_le_bytes(self.take(4)?.try_into().unwrap()))
    }

    fn f32(&mut self) -> std::result::Result<f32, ProtocolError> {
        let value = f32::from_le_bytes(self.take(4)?.try_into().unwrap());
        if value.is_finite() {
            Ok(value)
        } else {
            Err(ProtocolError::new("property numbers must be finite"))
        }
    }

    fn string(&mut self) -> std::result::Result<Arc<str>, ProtocolError> {
        let length = self.u32()? as usize;
        if length > MAX_STRING_BYTES {
            return Err(ProtocolError::new(format!(
                "strings cannot exceed {MAX_STRING_BYTES} bytes"
            )));
        }
        let value = std::str::from_utf8(self.take(length)?)
            .map_err(|_| ProtocolError::new("strings must contain valid UTF-8"))?;
        Ok(Arc::from(value))
    }

    fn finished(&self) -> bool {
        self.offset == self.bytes.len()
    }
}

fn decode_batch(bytes: &[u8]) -> std::result::Result<Vec<Mutation>, ProtocolError> {
    if bytes.len() > MAX_BATCH_BYTES {
        return Err(ProtocolError::new(format!(
            "one mutation batch cannot exceed {MAX_BATCH_BYTES} bytes"
        )));
    }
    let mut reader = Reader::new(bytes);
    if reader.take(4)? != PROTOCOL_MAGIC {
        return Err(ProtocolError::new("invalid mutation batch magic"));
    }
    let version = reader.u16()?;
    if version != PROTOCOL_VERSION {
        return Err(ProtocolError::new(format!(
            "unsupported mutation protocol version {version}"
        )));
    }
    let count = reader.u32()? as usize;
    if count > MAX_MUTATIONS {
        return Err(ProtocolError::new(format!(
            "one batch cannot contain more than {MAX_MUTATIONS} mutations"
        )));
    }
    let mut mutations = Vec::with_capacity(count.min(4_096));
    for _ in 0..count {
        let mutation = match reader.u8()? {
            1 => Mutation::Create {
                id: reader.u32()?,
                tag: NodeTag::decode(reader.u8()?)?,
                text: Arc::from(""),
            },
            2 => Mutation::Create {
                id: reader.u32()?,
                tag: NodeTag::Text,
                text: reader.string()?,
            },
            3 => Mutation::Create {
                id: reader.u32()?,
                tag: NodeTag::Sentinel,
                text: Arc::from(""),
            },
            4 => {
                let id = reader.u32()?;
                let key = reader.u16()?;
                let value = match reader.u8()? {
                    0 => None,
                    1 => Some(PropertyValue::Bool(match reader.u8()? {
                        0 => false,
                        1 => true,
                        value => {
                            return Err(ProtocolError::new(format!(
                                "invalid boolean byte {value}"
                            )));
                        }
                    })),
                    2 => Some(PropertyValue::Number(reader.f32()?)),
                    3 => Some(PropertyValue::Color(reader.u32()?)),
                    4 => Some(PropertyValue::String(reader.string()?)),
                    value => {
                        return Err(ProtocolError::new(format!(
                            "unknown property value tag {value}"
                        )));
                    }
                };
                Mutation::SetProperty { id, key, value }
            }
            5 => Mutation::ReplaceText {
                id: reader.u32()?,
                text: reader.string()?,
            },
            6 => {
                let parent = reader.u32()?;
                let child = reader.u32()?;
                let anchor = reader.u32()?;
                Mutation::Insert {
                    parent,
                    child,
                    before: (anchor != NO_ANCHOR).then_some(anchor),
                }
            }
            7 => Mutation::Remove {
                parent: reader.u32()?,
                child: reader.u32()?,
            },
            8 => {
                let parent = reader.u32()?;
                let count = reader.u32()? as usize;
                if count > MAX_MUTATIONS {
                    return Err(ProtocolError::new(
                        "cleanup list exceeds the mutation limit",
                    ));
                }
                let mut children = Vec::with_capacity(count.min(4_096));
                for _ in 0..count {
                    children.push(reader.u32()?);
                }
                Mutation::Cleanup { parent, children }
            }
            opcode => {
                return Err(ProtocolError::new(format!(
                    "unknown mutation opcode {opcode}"
                )));
            }
        };
        mutations.push(mutation);
    }
    if !reader.finished() {
        return Err(ProtocolError::new("mutation batch has trailing bytes"));
    }
    Ok(mutations)
}

#[derive(Clone, Debug)]
struct QueuedEvent {
    kind: &'static str,
    window: u32,
    target: u32,
    value: Option<Arc<str>>,
}

type EventQueue = Rc<RefCell<VecDeque<QueuedEvent>>>;

#[derive(Clone, Copy, Debug, PartialEq)]
struct NativeListConfig {
    estimated_item_height: f32,
    overscan: usize,
    alignment: ListAlignment,
    follow_mode: FollowMode,
}

impl NativeListConfig {
    fn from_node(node: &NativeNode) -> Self {
        let estimated_item_height = node
            .number(property::ESTIMATED_ITEM_HEIGHT)
            .unwrap_or(160.0)
            .clamp(1.0, 1_048_576.0);
        let overscan = node.number(property::OVERSCAN).unwrap_or(2.0).max(0.0) as usize;
        let alignment = match node.string(property::LIST_ALIGNMENT) {
            Some("bottom") => ListAlignment::Bottom,
            _ => ListAlignment::Top,
        };
        let follow_mode = match node.string(property::FOLLOW_MODE) {
            Some("tail") => FollowMode::Tail,
            _ => FollowMode::Normal,
        };
        Self {
            estimated_item_height,
            overscan,
            alignment,
            follow_mode,
        }
    }

    fn create_state(self, item_count: usize) -> ListState {
        ListState::new(item_count, self.estimated_item_height)
            .with_overscan(self.overscan)
            .with_alignment(self.alignment)
            .with_follow_mode(self.follow_mode)
    }
}

struct NativeListState {
    config: NativeListConfig,
    children: Vec<u32>,
    list: ListState,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct NativeTerminalConfig {
    options: TerminalOptions,
}

impl NativeTerminalConfig {
    fn from_node(node: &NativeNode) -> std::result::Result<Self, String> {
        let arguments = node
            .string(property::TERMINAL_ARGUMENTS)
            .map(|value| {
                serde_json::from_str::<Vec<String>>(value)
                    .map_err(|error| format!("invalid terminal arguments: {error}"))
            })
            .transpose()?
            .unwrap_or_default()
            .into_iter()
            .map(Into::into)
            .collect();
        let environment = node
            .string(property::TERMINAL_ENVIRONMENT)
            .map(|value| {
                serde_json::from_str::<BTreeMap<String, String>>(value)
                    .map_err(|error| format!("invalid terminal environment: {error}"))
            })
            .transpose()?
            .unwrap_or_default()
            .into_iter()
            .map(|(key, value)| (key.into(), value.into()))
            .collect();
        let max_scrollback = node
            .number(property::TERMINAL_SCROLLBACK)
            .unwrap_or(10_000.0)
            .max(0.0) as usize;
        Ok(Self {
            options: TerminalOptions {
                program: node
                    .string(property::TERMINAL_PROGRAM)
                    .filter(|program| !program.is_empty())
                    .map(Into::into),
                arguments,
                working_directory: node
                    .string(property::TERMINAL_WORKING_DIRECTORY)
                    .filter(|directory| !directory.is_empty())
                    .map(PathBuf::from),
                environment,
                max_scrollback,
                ..TerminalOptions::default()
            },
        })
    }
}

struct NativeTerminalState {
    config: std::result::Result<NativeTerminalConfig, Arc<str>>,
    terminal: Option<Terminal>,
    spawn_error: Option<Arc<str>>,
    last_event: Option<Arc<str>>,
}

struct NativeSvgState {
    source: Arc<str>,
    parsed: std::result::Result<Svg, Arc<str>>,
}

impl NativeSvgState {
    fn new(source: Arc<str>) -> Self {
        let parsed = Svg::from_svg(source.as_ref()).map_err(|error| Arc::from(error.to_string()));
        Self { source, parsed }
    }

    fn sync(&mut self, source: &str) {
        if self.source.as_ref() == source {
            return;
        }
        *self = Self::new(Arc::from(source));
    }

    fn element(&self) -> Element {
        match &self.parsed {
            Ok(svg) => svg_element(svg),
            Err(_) => div().hidden(),
        }
    }
}

impl NativeTerminalState {
    fn new(node: &NativeNode, cx: &ViewContext<'_, NativeView>) -> Self {
        let config = NativeTerminalConfig::from_node(node).map_err(Arc::from);
        let (terminal, spawn_error) = match &config {
            Ok(config) => match Terminal::spawn(config.options.clone(), cx.window_invalidator()) {
                Ok(terminal) => (Some(terminal), None),
                Err(error) => (None, Some(Arc::from(error.to_string()))),
            },
            Err(_) => (None, None),
        };
        Self {
            config,
            terminal,
            spawn_error,
            last_event: None,
        }
    }

    fn sync(&mut self, node: &NativeNode, cx: &ViewContext<'_, NativeView>) {
        let next = NativeTerminalConfig::from_node(node).map_err(Arc::from);
        if self.config == next {
            return;
        }
        *self = Self::new(node, cx);
    }

    fn error(&self) -> Option<&str> {
        match (&self.config, &self.terminal, &self.spawn_error) {
            (Err(error), _, _) => Some(error),
            (Ok(_), None, Some(error)) => Some(error),
            (Ok(_), None, None) => Some("could not start terminal session"),
            (Ok(_), Some(_), _) => None,
        }
    }
}

impl NativeListState {
    fn new(node: &NativeNode) -> Self {
        let config = NativeListConfig::from_node(node);
        Self {
            config,
            children: node.children.clone(),
            list: config.create_state(node.children.len()),
        }
    }

    fn sync(&mut self, node: &NativeNode) {
        let config = NativeListConfig::from_node(node);
        if self.config != config {
            self.config = config;
            self.children.clone_from(&node.children);
            self.list = config.create_state(self.children.len());
            return;
        }
        if self.children == node.children {
            return;
        }
        let stable_prefix =
            self.children.starts_with(&node.children) || node.children.starts_with(&self.children);
        if stable_prefix {
            self.list.set_item_count(node.children.len());
        } else {
            self.list.reset(node.children.len());
        }
        self.children.clone_from(&node.children);
    }
}

struct NativeView {
    window: u32,
    handles: Option<Rc<RefCell<HashMap<WindowHandle, u32>>>>,
    tree: Rc<RefCell<NativeTree>>,
    events: EventQueue,
    markdown: Rc<RefCell<HashMap<u32, Markdown>>>,
    svgs: Rc<RefCell<HashMap<u32, NativeSvgState>>>,
    lists: Rc<RefCell<HashMap<u32, NativeListState>>>,
    terminals: Rc<RefCell<HashMap<u32, NativeTerminalState>>>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct NativeMenuAction(u32);

impl View for NativeView {
    fn render(&mut self, cx: &mut ViewContext<'_, Self>) -> impl IntoElement {
        let window = self.handles.as_ref().map_or(self.window, |handles| {
            handles
                .borrow()
                .get(&cx.window_handle())
                .copied()
                .expect("a native binding view must retain its core window handle")
        });
        let tree = self.tree.borrow();
        let mut markdown = self.markdown.borrow_mut();
        markdown.retain(|id, _| {
            tree.nodes
                .get(id)
                .is_some_and(|node| node.tag == NodeTag::Markdown)
        });
        let mut svgs = self.svgs.borrow_mut();
        svgs.retain(|id, _| {
            tree.nodes
                .get(id)
                .is_some_and(|node| node.tag == NodeTag::Svg)
        });
        let mut lists = self.lists.borrow_mut();
        lists.retain(|id, _| {
            tree.nodes
                .get(id)
                .is_some_and(|node| node.tag == NodeTag::VirtualList)
        });
        let mut terminals = self.terminals.borrow_mut();
        terminals.retain(|id, _| {
            tree.nodes
                .get(id)
                .is_some_and(|node| node.tag == NodeTag::Terminal)
        });
        let mut root = div()
            .id(ElementId::new(ROOT_ELEMENT_ID))
            .size_full()
            .min_w(0.0)
            .min_h(0.0);
        if let Some(node) = tree.nodes.get(&ROOT_NODE) {
            root = root.children(node.children.iter().filter_map(|id| {
                build_element(
                    *id,
                    window,
                    &tree,
                    &self.events,
                    &mut markdown,
                    &mut svgs,
                    &mut lists,
                    &mut terminals,
                    cx,
                    0,
                )
            }));
        }
        let events = Rc::clone(&self.events);
        let menu_action = cx.action_listener(
            ElementId::new(ROOT_ELEMENT_ID),
            move |_view, action: &NativeMenuAction, _cx| {
                enqueue_event(
                    &events,
                    QueuedEvent {
                        kind: "menu-action",
                        window,
                        target: action.0,
                        value: None,
                    },
                );
            },
        );
        root.on_action(menu_action)
    }
}

fn build_element(
    id: u32,
    window: u32,
    tree: &NativeTree,
    events: &EventQueue,
    markdown: &mut HashMap<u32, Markdown>,
    svgs: &mut HashMap<u32, NativeSvgState>,
    lists: &mut HashMap<u32, NativeListState>,
    terminals: &mut HashMap<u32, NativeTerminalState>,
    cx: &mut ViewContext<'_, NativeView>,
    depth: usize,
) -> Option<Element> {
    if depth >= MAX_TREE_DEPTH {
        return None;
    }
    let node = tree.nodes.get(&id)?;
    let element_id = ElementId::new(id as u64);
    let mut element = match node.tag {
        NodeTag::Root => return None,
        NodeTag::View => div(),
        NodeTag::Button => button().cursor_default(),
        NodeTag::Text => text(node.text.clone()),
        NodeTag::Sentinel => div().hidden(),
        NodeTag::Input => {
            let multiline = node.boolean(property::MULTILINE).unwrap_or(false);
            let mut input = if multiline {
                text_area(node.string(property::VALUE).unwrap_or_default())
            } else {
                text_input(node.string(property::VALUE).unwrap_or_default())
            }
            .bg(Color::TRANSPARENT)
            .border(0.0, Color::TRANSPARENT)
            .rounded(0.0);
            if let Some(placeholder) = node.string(property::PLACEHOLDER) {
                input = input.placeholder(placeholder);
            }
            if !multiline && node.boolean(property::PASSWORD).unwrap_or(false) {
                input = input.password(true);
            }
            if node.boolean(property::INPUT_LISTENER).unwrap_or(false) {
                let events = Rc::clone(events);
                // The retained input state already schedules its paint. Rebuilding here would read
                // the previous JavaScript-controlled value before Bun drains this queued event,
                // resetting every keystroke before Solid can commit the matching mutation batch.
                let listener = cx.input_listener(element_id, move |_view, value, _cx| {
                    enqueue_event(
                        &events,
                        QueuedEvent {
                            kind: "input",
                            window,
                            target: id,
                            value: Some(Arc::from(value)),
                        },
                    );
                });
                input = input.on_input(listener);
            }
            if !multiline && node.boolean(property::SUBMIT_LISTENER).unwrap_or(false) {
                let events = Rc::clone(events);
                // Submit has the same controlled-state boundary as input: JavaScript must consume
                // the queued value before a render can safely read the controlled property again.
                let listener = cx.submit_listener(element_id, move |_view, value, _cx| {
                    enqueue_event(
                        &events,
                        QueuedEvent {
                            kind: "submit",
                            window,
                            target: id,
                            value: Some(Arc::from(value)),
                        },
                    );
                });
                input = input.on_submit(listener);
            }
            input
        }
        NodeTag::Markdown => {
            let mut markdown_style = MarkdownStyle::default();
            markdown_style.text_color = node.color(property::COLOR);
            markdown_style.font_size = node
                .number(property::FONT_SIZE)
                .unwrap_or(markdown_style.font_size);
            markdown_style.line_height = node
                .number(property::LINE_HEIGHT)
                .unwrap_or(markdown_style.line_height);
            markdown_style.code_background = node.color(property::MARKDOWN_CODE_BACKGROUND);
            markdown_style.border_color = node.color(property::MARKDOWN_BORDER_COLOR);
            markdown_style.muted_color = node.color(property::MARKDOWN_MUTED_COLOR);
            markdown_style.link_color = node.color(property::MARKDOWN_LINK_COLOR);
            markdown_style.code_text_color = node.color(property::MARKDOWN_CODE_TEXT_COLOR);
            markdown_style.block_gap = node
                .number(property::MARKDOWN_BLOCK_GAP)
                .unwrap_or(markdown_style.block_gap);
            markdown_style.code_font_size = node
                .number(property::MARKDOWN_CODE_FONT_SIZE)
                .unwrap_or(markdown_style.code_font_size);
            let state = markdown.entry(id).or_default();
            state.set_streaming(node.boolean(property::STREAMING).unwrap_or(false));
            state.set_style(markdown_style);
            state.set_text(node.string(property::VALUE).unwrap_or_default());
            state.element(element_id)
        }
        NodeTag::Svg => {
            let source = node.string(property::VALUE).unwrap_or_default();
            let state = svgs
                .entry(id)
                .or_insert_with(|| NativeSvgState::new(Arc::from(source)));
            state.sync(source);
            state.element()
        }
        NodeTag::VirtualList => div(),
        NodeTag::Terminal => {
            let state = terminals
                .entry(id)
                .or_insert_with(|| NativeTerminalState::new(node, cx));
            state.sync(node, cx);
            let terminal = state.terminal.clone();
            if let Some(terminal) = terminal {
                let snapshot = terminal.snapshot();
                if node
                    .boolean(property::TERMINAL_STATUS_LISTENER)
                    .unwrap_or(false)
                {
                    let value = terminal_event_json(&snapshot);
                    if state.last_event.as_deref() != Some(value.as_str()) {
                        state.last_event = Some(Arc::from(value.as_str()));
                        enqueue_event(
                            events,
                            QueuedEvent {
                                kind: "terminal",
                                window,
                                target: id,
                                value: Some(value.into()),
                            },
                        );
                    }
                } else {
                    state.last_event = None;
                }
                terminal.element(
                    element_id,
                    TerminalStyle {
                        font_family: node
                            .string(property::FONT_FAMILY)
                            .and_then(native_font_family)
                            .unwrap_or(quickgui::FontFamily::Monospace),
                        font_size: node.number(property::FONT_SIZE).unwrap_or(13.0),
                        line_height: node.number(property::LINE_HEIGHT).unwrap_or(18.0),
                        padding_top: terminal_padding(node, property::PADDING_TOP),
                        padding_right: terminal_padding(node, property::PADDING_RIGHT),
                        padding_bottom: terminal_padding(node, property::PADDING_BOTTOM),
                        padding_left: terminal_padding(node, property::PADDING_LEFT),
                        foreground: node.color(property::COLOR),
                        background: node.color(property::BACKGROUND_COLOR),
                        theme: native_terminal_theme(node),
                        ..TerminalStyle::default()
                    },
                    cx,
                )
            } else {
                div()
                    .size_full()
                    .bg(Color::rgb8(20, 20, 20))
                    .text_color(Color::rgb8(248, 113, 113))
                    .font_family(quickgui::FontFamily::Monospace)
                    .text_sm()
                    .p_4()
                    .child(text(format!(
                        "QuickGUI terminal error\n\n{}",
                        state.error().unwrap_or("unknown terminal error")
                    )))
            }
        }
    }
    .id(element_id);

    element = apply_properties(element, node);

    let anchor_id = node
        .string(property::ANCHOR_TARGET)
        .and_then(|value| value.parse::<u32>().ok())
        .filter(|anchor_id| *anchor_id != id && tree.nodes.contains_key(anchor_id));
    if let Some(anchor_id) = anchor_id {
        let dismiss_on_escape = node.boolean(property::DISMISS_ON_ESCAPE).unwrap_or(true);
        let dismiss_on_pointer_outside = node
            .boolean(property::DISMISS_ON_POINTER_OUTSIDE)
            .unwrap_or(true);
        let mut popover = Popover::new(ElementId::new(anchor_id as u64), element_id, true)
            .placement(
                node.string(property::ANCHOR_PLACEMENT)
                    .and_then(parse_anchor_placement)
                    .unwrap_or_default(),
            )
            .dismiss_on_escape(dismiss_on_escape)
            .dismiss_on_pointer_outside(dismiss_on_pointer_outside);
        if let Some(gap) = node.number(property::ANCHOR_GAP) {
            popover = popover.anchor_gap(gap);
        }
        if let Some(margin) = node.number(property::VIEWPORT_MARGIN) {
            popover = popover.viewport_margin(margin);
        }
        element = popover.surface_part(element);

        element = attach_dismiss_listener(
            element,
            node.boolean(property::DISMISS_LISTENER).unwrap_or(false)
                && (dismiss_on_escape || dismiss_on_pointer_outside),
            element_id,
            id,
            window,
            events,
            cx,
        );
    } else {
        let dismiss_on_escape = node.boolean(property::DISMISS_ON_ESCAPE).unwrap_or(false);
        let dismiss_on_pointer_outside = node
            .boolean(property::DISMISS_ON_POINTER_OUTSIDE)
            .unwrap_or(false);
        if dismiss_on_escape {
            element = element.dismiss_on_escape();
        }
        if dismiss_on_pointer_outside {
            element = element.dismiss_on_pointer_outside();
        }
        element = attach_dismiss_listener(
            element,
            node.boolean(property::DISMISS_LISTENER).unwrap_or(false)
                && (dismiss_on_escape || dismiss_on_pointer_outside),
            element_id,
            id,
            window,
            events,
            cx,
        );
    }

    if node.boolean(property::CLICK_LISTENER).unwrap_or(false) {
        let events = Rc::clone(events);
        let listener = cx.listener(element_id, move |_view, cx| {
            enqueue_event(
                &events,
                QueuedEvent {
                    kind: "click",
                    window,
                    target: id,
                    value: None,
                },
            );
            cx.invalidate();
        });
        element = element.on_click(listener);
    }
    if node.boolean(property::HOVER_LISTENER).unwrap_or(false) {
        let events = Rc::clone(events);
        let listener = cx.hover_listener(element_id, move |_view, hovered, cx| {
            enqueue_event(
                &events,
                QueuedEvent {
                    kind: if *hovered { "mouseenter" } else { "mouseleave" },
                    window,
                    target: id,
                    value: None,
                },
            );
            cx.invalidate();
        });
        element = element.on_hover(listener);
    }
    if node.boolean(property::POINTER_LISTENER).unwrap_or(false) {
        let events = Rc::clone(events);
        let listener = cx.pointer_listener(element_id, move |_view, event, cx| {
            enqueue_event(
                &events,
                QueuedEvent {
                    kind: "pointer",
                    window,
                    target: id,
                    value: Some(pointer_event_json(event).into()),
                },
            );
            cx.prevent_default();
            cx.stop_propagation();
            cx.invalidate();
        });
        element = element.on_pointer(listener);
    }

    match node.tag {
        NodeTag::VirtualList => {
            let state = lists
                .entry(id)
                .or_insert_with(|| NativeListState::new(node));
            state.sync(node);
            let list = state.list.clone();
            let children = state.children.clone();
            let visible = list.visible_rows().range;
            let gap = node
                .number(property::ROW_GAP)
                .or_else(|| node.number(property::GAP))
                .unwrap_or(0.0)
                .max(0.0);
            let alignment = node.string(property::ALIGN_ITEMS);
            let item_count = children.len();
            let rows = list.render_rows(visible, |index| {
                let child = build_element(
                    children[index],
                    window,
                    tree,
                    events,
                    markdown,
                    svgs,
                    lists,
                    terminals,
                    cx,
                    depth + 1,
                )
                .unwrap_or_else(|| div().hidden());
                let mut row = div().w_full().flex_none().flex_row().child(child);
                row = match alignment {
                    Some("center") => row.justify_center(),
                    Some("flex-end" | "end") => row.justify_end(),
                    _ => row.justify_start(),
                };
                if gap > 0.0 && index + 1 < item_count {
                    row = row.padding(0.0, 0.0, gap, 0.0);
                }
                row
            });
            element = element.child(rows).variable_virtual_scroll(&list);
        }
        NodeTag::Text
        | NodeTag::Sentinel
        | NodeTag::Input
        | NodeTag::Markdown
        | NodeTag::Svg
        | NodeTag::Terminal => {}
        NodeTag::Root | NodeTag::View | NodeTag::Button => {
            element = element.children(node.children.iter().filter_map(|child| {
                build_element(
                    *child,
                    window,
                    tree,
                    events,
                    markdown,
                    svgs,
                    lists,
                    terminals,
                    cx,
                    depth + 1,
                )
            }));
        }
    }
    Some(element)
}

fn attach_dismiss_listener(
    mut element: Element,
    enabled: bool,
    element_id: ElementId,
    node_id: u32,
    window: u32,
    events: &EventQueue,
    cx: &mut ViewContext<'_, NativeView>,
) -> Element {
    if !enabled {
        return element;
    }
    let events = Rc::clone(events);
    let listener = cx.dismiss_listener(element_id, move |_view, cx| {
        enqueue_event(
            &events,
            QueuedEvent {
                kind: "dismiss",
                window,
                target: node_id,
                value: None,
            },
        );
        cx.invalidate();
    });
    element = element.on_dismiss(listener);
    element
}

fn enqueue_event(events: &EventQueue, event: QueuedEvent) {
    let mut events = events.borrow_mut();
    if events.len() < MAX_QUEUED_EVENTS {
        events.push_back(event);
    }
}

fn terminal_event_json(snapshot: &quickgui::TerminalSnapshot) -> String {
    let mut event = serde_json::json!({
        "status": snapshot.status.kind(),
        "title": snapshot.title.as_ref(),
        "workingDirectory": snapshot.working_directory.as_ref(),
    });
    let object = event
        .as_object_mut()
        .expect("terminal event JSON starts as an object");
    match &snapshot.status {
        TerminalStatus::Starting => {}
        TerminalStatus::Running { process_id } => {
            object.insert("processId".to_owned(), serde_json::json!(process_id));
        }
        TerminalStatus::Exited { exit_code, signal } => {
            object.insert("exitCode".to_owned(), serde_json::json!(exit_code));
            object.insert("signal".to_owned(), serde_json::json!(signal.as_deref()));
        }
        TerminalStatus::Failed { message } => {
            object.insert("message".to_owned(), serde_json::json!(message.as_ref()));
        }
    }
    if let Some(agent) = &snapshot.agent {
        object.insert("agent".to_owned(), serde_json::json!(agent.kind.as_ref()));
        object.insert(
            "agentStatus".to_owned(),
            serde_json::json!(agent.status.kind()),
        );
        object.insert(
            "agentProcessId".to_owned(),
            serde_json::json!(agent.process_id),
        );
    }
    event.to_string()
}

fn pointer_event_json(event: &quickgui::PointerEvent) -> String {
    let phase = match event.phase {
        PointerPhase::Down => "down",
        PointerPhase::Move => "move",
        PointerPhase::Up => "up",
        PointerPhase::Cancel => "cancel",
    };
    let button = match event.button {
        quickgui::MouseButton::Left => "left",
        quickgui::MouseButton::Right => "right",
        quickgui::MouseButton::Middle => "middle",
        quickgui::MouseButton::Back => "back",
        quickgui::MouseButton::Forward => "forward",
        quickgui::MouseButton::Other(_) => "other",
    };
    serde_json::json!({
        "phase": phase,
        "position": { "x": event.position.x, "y": event.position.y },
        "origin": { "x": event.origin.x, "y": event.origin.y },
        "delta": { "x": event.delta.x, "y": event.delta.y },
        "button": button,
    })
    .to_string()
}

fn apply_properties(mut element: Element, node: &NativeNode) -> Element {
    if let Some(display) = node.string(property::DISPLAY) {
        element = match display {
            "none" => element.hidden(),
            "flex" => match node.string(property::FLEX_DIRECTION) {
                Some("row") => element.flex_row(),
                Some("row-reverse") => element.flex_row_reverse(),
                Some("column") => element.flex_col(),
                Some("column-reverse") => element.flex_col_reverse(),
                _ => element.flex(),
            },
            "grid" => element.grid(),
            _ => element.block(),
        };
    }
    if let Some(wrap) = node.string(property::FLEX_WRAP) {
        element = match wrap {
            "wrap" => element.flex_wrap(),
            "wrap-reverse" => element.flex_wrap_reverse(),
            _ => element.flex_nowrap(),
        };
    }
    if let Some(value) = node.number(property::FLEX_GROW) {
        element = element.flex_grow(value);
    }
    if let Some(value) = node.number(property::FLEX_SHRINK) {
        element = element.flex_shrink(value);
    }
    if let Some(value) = node.number(property::FLEX_BASIS) {
        element = element.flex_basis(value);
    } else if node.string(property::FLEX_BASIS) == Some("auto") {
        element = element.flex_basis_auto();
    }
    if let Some(value) = node.string(property::ALIGN_ITEMS) {
        element = match value {
            "center" => element.items_center(),
            "flex-end" | "end" => element.items_end(),
            "baseline" => element.items_baseline(),
            "stretch" => element.items_stretch(),
            _ => element.items_start(),
        };
    }
    if let Some(value) = node.string(property::ALIGN_SELF) {
        element = match value {
            "center" => element.self_center(),
            "flex-end" => element.self_flex_end(),
            "end" => element.self_end(),
            "baseline" => element.self_baseline(),
            "stretch" => element.self_stretch(),
            "start" => element.self_start(),
            _ => element.self_flex_start(),
        };
    }
    if let Some(value) = node.string(property::JUSTIFY_CONTENT) {
        element = match value {
            "center" => element.justify_center(),
            "flex-end" | "end" => element.justify_end(),
            "space-between" => element.justify_between(),
            "space-around" => element.justify_around(),
            "space-evenly" => element.justify_evenly(),
            _ => element.justify_start(),
        };
    }
    if let Some(value) = node.string(property::ALIGN_CONTENT) {
        element = match value {
            "center" => element.content_center(),
            "flex-end" | "end" => element.content_end(),
            "space-between" => element.content_between(),
            "space-around" => element.content_around(),
            "space-evenly" => element.content_evenly(),
            "stretch" => element.content_stretch(),
            "normal" => element.content_normal(),
            _ => element.content_start(),
        };
    }
    if let Some(value) = node.number(property::GAP) {
        element = element.gap(value);
    }
    if let Some(value) = node.number(property::COLUMN_GAP) {
        element = element.gap_x(value);
    }
    if let Some(value) = node.number(property::ROW_GAP) {
        element = element.gap_y(value);
    }

    element = apply_dimension(element, node, property::WIDTH, DimensionKind::Width);
    element = apply_dimension(element, node, property::HEIGHT, DimensionKind::Height);
    if let Some(value) = node.number(property::MIN_WIDTH) {
        element = element.min_w(value);
    }
    if let Some(value) = node.number(property::MIN_HEIGHT) {
        element = element.min_h(value);
    }
    if let Some(value) = node.number(property::MAX_WIDTH) {
        element = element.max_w(value);
    }
    if let Some(value) = node.number(property::MAX_HEIGHT) {
        element = element.max_h(value);
    }

    let padding = node.number(property::PADDING).unwrap_or(0.0);
    let padding_top = node.number(property::PADDING_TOP).unwrap_or(padding);
    let padding_right = node.number(property::PADDING_RIGHT).unwrap_or(padding);
    let padding_bottom = node.number(property::PADDING_BOTTOM).unwrap_or(padding);
    let padding_left = node.number(property::PADDING_LEFT).unwrap_or(padding);
    if node.tag != NodeTag::Terminal
        && [padding_top, padding_right, padding_bottom, padding_left]
            .iter()
            .any(|value| *value != 0.0)
    {
        element = element.padding(padding_top, padding_right, padding_bottom, padding_left);
    }
    let margin = node.number(property::MARGIN).unwrap_or(0.0);
    let margin_top = node.number(property::MARGIN_TOP).unwrap_or(margin);
    let margin_right = node.number(property::MARGIN_RIGHT).unwrap_or(margin);
    let margin_bottom = node.number(property::MARGIN_BOTTOM).unwrap_or(margin);
    let margin_left = node.number(property::MARGIN_LEFT).unwrap_or(margin);
    if [margin_top, margin_right, margin_bottom, margin_left]
        .iter()
        .any(|value| *value != 0.0)
    {
        element = element.margin(margin_top, margin_right, margin_bottom, margin_left);
    }

    if let Some(color) = node.color(property::BACKGROUND_COLOR) {
        element = element.bg(color);
    }
    if let Some(color) = node.color(property::COLOR) {
        element = element.text_color(color);
    }
    let hover_background = node.color(property::HOVER_BACKGROUND_COLOR);
    let hover_color = node.color(property::HOVER_COLOR);
    if hover_background.is_some() || hover_color.is_some() {
        element = element.hover(move |mut style| {
            if let Some(color) = hover_background {
                style = style.bg(color);
            }
            if let Some(color) = hover_color {
                style = style.text_color(color);
            }
            style
        });
    }
    let active_background = node.color(property::ACTIVE_BACKGROUND_COLOR);
    let active_color = node.color(property::ACTIVE_COLOR);
    if active_background.is_some() || active_color.is_some() {
        element = element.active(move |mut style| {
            if let Some(color) = active_background {
                style = style.bg(color);
            }
            if let Some(color) = active_color {
                style = style.text_color(color);
            }
            style
        });
    }
    if let Some(milliseconds) = node.number(property::TRANSITION) {
        element = element.transition(Transition::colors(Duration::from_secs_f32(
            (milliseconds / 1_000.0).clamp(0.0, 10.0),
        )));
    }
    if let Some(value) = node.number(property::OPACITY) {
        element = element.opacity(value);
    }
    let border_width = node.number(property::BORDER_WIDTH).unwrap_or(0.0);
    if border_width > 0.0 {
        element = element.border(
            border_width,
            node.color(property::BORDER_COLOR)
                .unwrap_or(Color::TRANSPARENT),
        );
    }
    if let Some(value) = node.number(property::BORDER_RADIUS) {
        element = element.rounded(value);
    }
    if let Some(value) = node.number(property::FONT_SIZE) {
        element = element.text_size(value);
    }
    if let Some(family) = node
        .string(property::FONT_FAMILY)
        .and_then(native_font_family)
    {
        element = element.font_family(family);
    }
    if let Some(value) = node.number(property::LINE_HEIGHT) {
        element = element.line_height(value);
    }
    if let Some(weight) = font_weight(node.property(property::FONT_WEIGHT)) {
        element = element.font_weight(weight);
    }
    if let Some(value) = node.string(property::TEXT_ALIGN) {
        element = element.text_align(match value {
            "center" => TextAlign::Center,
            "right" | "end" => TextAlign::Right,
            "justify" => TextAlign::Justify,
            _ => TextAlign::Left,
        });
    }
    if let Some(value) = node.string(property::WHITE_SPACE) {
        element = match value {
            "nowrap" => element.whitespace_nowrap(),
            _ => element.whitespace_normal(),
        };
    }
    if node.string(property::TEXT_OVERFLOW) == Some("ellipsis") {
        element = element.text_ellipsis();
    }
    if let Some(value) = node.number(property::LINE_CLAMP) {
        element = element.line_clamp(value.max(1.0) as usize);
    }
    if [
        node.string(property::OVERFLOW),
        node.string(property::OVERFLOW_X),
        node.string(property::OVERFLOW_Y),
    ]
    .into_iter()
    .flatten()
    .any(|value| value == "hidden")
    {
        element = element.overflow_hidden();
    }
    if node
        .string(property::OVERFLOW_Y)
        .is_some_and(|value| matches!(value, "auto" | "scroll"))
        || node
            .string(property::OVERFLOW)
            .is_some_and(|value| matches!(value, "auto" | "scroll"))
    {
        element = element.overflow_y_scroll();
    }
    if let Some(value) = node.number(property::SCROLL_TO_END_REVISION) {
        element = element.scroll_to_end(value.max(0.0) as u64);
    }
    if let Some(value) = node.string(property::CURSOR) {
        element = element.cursor(cursor(value));
    }
    if let Some(value) = node.string(property::APP_REGION) {
        element = element.app_region(if value == "drag" {
            AppRegion::Drag
        } else {
            AppRegion::NoDrag
        });
    }
    if let Some(value) = node.boolean(property::DISABLED) {
        element = element.disabled(value);
    }
    if let Some(value) = node.string(property::ACCESSIBILITY_LABEL) {
        element = element.accessibility_label(value.to_owned());
    }
    if let Some(value) = node.string(property::ROLE).and_then(accessibility_role) {
        element = element.accessibility_role(value);
    }
    if let Some(value) = node.number(property::TAB_INDEX) {
        element = element.tab_index(value.clamp(i16::MIN as f32, i16::MAX as f32) as i16);
    }
    if let Some(value) = node.boolean(property::FOCUS_ON_POINTER) {
        element = element.focus_on_pointer(value);
    }
    if node.boolean(property::OVERLAY) == Some(true) {
        element = element.overlay();
    }
    if node.boolean(property::FOCUS_TRAP) == Some(true) {
        element = element.focus_trap();
    }
    if node.boolean(property::RESTORE_PREVIOUS_FOCUS) == Some(true) {
        element = element.restore_previous_focus();
    }
    if node.boolean(property::AUTO_FOCUS) == Some(true) {
        element = element.auto_focus();
    }
    if let Some(value) = node.boolean(property::ACCESSIBILITY_MODAL) {
        element = element.accessibility_modal(value);
    }
    let hit_slop = node.number(property::HIT_SLOP).unwrap_or(0.0);
    let hit_slop = quickgui::Insets {
        top: node.number(property::HIT_SLOP_TOP).unwrap_or(hit_slop),
        right: node.number(property::HIT_SLOP_RIGHT).unwrap_or(hit_slop),
        bottom: node.number(property::HIT_SLOP_BOTTOM).unwrap_or(hit_slop),
        left: node.number(property::HIT_SLOP_LEFT).unwrap_or(hit_slop),
    };
    if hit_slop != quickgui::Insets::default() {
        element = element.hit_slop(hit_slop);
    }
    if let Some(value) = node.string(property::POSITION) {
        element = if value == "absolute" {
            element.absolute()
        } else {
            element.relative()
        };
    }
    if let Some(value) = node.number(property::TOP) {
        element = element.top(value);
    }
    if let Some(value) = node.number(property::RIGHT) {
        element = element.right(value);
    }
    if let Some(value) = node.number(property::BOTTOM) {
        element = element.bottom(value);
    }
    if let Some(value) = node.number(property::LEFT) {
        element = element.left(value);
    }
    if let Some(value) = node.string(property::USER_SELECT) {
        element = match value {
            "none" => element.user_select_none(),
            "text" => element.user_select_text(),
            _ => element,
        };
    }
    if let Some(value) = node.string(property::VISIBILITY) {
        element = if value == "hidden" {
            element.invisible()
        } else {
            element.visible()
        };
    }
    if let Some(value) = node.number(property::ASPECT_RATIO) {
        element = element.aspect_ratio(value);
    }
    element
}

fn terminal_padding(node: &NativeNode, side: u16) -> f32 {
    node.number(side)
        .or_else(|| node.number(property::PADDING))
        .unwrap_or(0.0)
}

enum DimensionKind {
    Width,
    Height,
}

fn apply_dimension(
    element: Element,
    node: &NativeNode,
    property: u16,
    kind: DimensionKind,
) -> Element {
    match node.property(property) {
        Some(PropertyValue::Number(value)) => match kind {
            DimensionKind::Width => element.w(*value),
            DimensionKind::Height => element.h(*value),
        },
        Some(PropertyValue::String(value)) if value.as_ref() == "100%" => match kind {
            DimensionKind::Width => element.w_full(),
            DimensionKind::Height => element.h_full(),
        },
        _ => element,
    }
}

fn unpack_color(value: u32) -> Color {
    Color::rgba8(
        value as u8,
        (value >> 8) as u8,
        (value >> 16) as u8,
        (value >> 24) as u8,
    )
}

fn native_font_family(value: &str) -> Option<quickgui::FontFamily> {
    match value.trim() {
        "sans-serif" | "system-ui" => Some(quickgui::FontFamily::SansSerif),
        "serif" => Some(quickgui::FontFamily::Serif),
        "monospace" => Some(quickgui::FontFamily::Monospace),
        name if !name.is_empty() && name.len() <= quickgui::MAX_FONT_FAMILY_BYTES => {
            Some(quickgui::FontFamily::named(Arc::<str>::from(name)))
        }
        _ => None,
    }
}

fn native_terminal_theme(node: &NativeNode) -> Option<TerminalTheme> {
    let encoded = node.string(property::TERMINAL_PALETTE)?;
    let packed = serde_json::from_str::<Vec<u32>>(encoded).ok()?;
    let packed: [u32; TERMINAL_ANSI_COLOR_COUNT] = packed.try_into().ok()?;
    let ansi = packed.map(unpack_color);
    let foreground = node.color(property::COLOR)?;
    let background = node.color(property::BACKGROUND_COLOR)?;
    Some(
        TerminalTheme::new(foreground, background, ansi).cursor(
            node.color(property::TERMINAL_CURSOR_COLOR)
                .unwrap_or(foreground),
        ),
    )
}

fn font_weight(value: Option<&PropertyValue>) -> Option<FontWeight> {
    let weight = match value {
        Some(PropertyValue::Number(value)) => *value,
        Some(PropertyValue::String(value)) => match value.as_ref() {
            "thin" => 100.0,
            "extralight" | "extra-light" => 200.0,
            "light" => 300.0,
            "medium" => 500.0,
            "semibold" | "semi-bold" => 600.0,
            "bold" => 700.0,
            "extrabold" | "extra-bold" => 800.0,
            "black" => 900.0,
            _ => 400.0,
        },
        _ => return None,
    };
    Some(if weight < 150.0 {
        FontWeight::THIN
    } else if weight < 250.0 {
        FontWeight::EXTRA_LIGHT
    } else if weight < 350.0 {
        FontWeight::LIGHT
    } else if weight < 450.0 {
        FontWeight::NORMAL
    } else if weight < 550.0 {
        FontWeight::MEDIUM
    } else if weight < 650.0 {
        FontWeight::SEMIBOLD
    } else if weight < 750.0 {
        FontWeight::BOLD
    } else if weight < 850.0 {
        FontWeight::EXTRA_BOLD
    } else {
        FontWeight::BLACK
    })
}

fn cursor(value: &str) -> CursorStyle {
    match value {
        "text" => CursorStyle::IBeam,
        "pointer" => CursorStyle::PointingHand,
        "crosshair" => CursorStyle::Crosshair,
        "grab" => CursorStyle::OpenHand,
        "grabbing" => CursorStyle::ClosedHand,
        "not-allowed" | "no-drop" => CursorStyle::OperationNotAllowed,
        "copy" => CursorStyle::DragCopy,
        "alias" => CursorStyle::DragLink,
        "context-menu" => CursorStyle::ContextualMenu,
        "ew-resize" => CursorStyle::ResizeLeftRight,
        "ns-resize" => CursorStyle::ResizeUpDown,
        "nwse-resize" => CursorStyle::ResizeUpLeftDownRight,
        "nesw-resize" => CursorStyle::ResizeUpRightDownLeft,
        "col-resize" => CursorStyle::ResizeColumn,
        "row-resize" => CursorStyle::ResizeRow,
        "n-resize" => CursorStyle::ResizeUp,
        "e-resize" => CursorStyle::ResizeRight,
        "s-resize" => CursorStyle::ResizeDown,
        "w-resize" => CursorStyle::ResizeLeft,
        "vertical-text" => CursorStyle::IBeamCursorForVerticalLayout,
        _ => CursorStyle::Arrow,
    }
}

fn accessibility_role(value: &str) -> Option<AccessibilityRole> {
    Some(match value {
        "button" => AccessibilityRole::Button,
        "link" => AccessibilityRole::Link,
        "img" | "image" => AccessibilityRole::Image,
        "list" => AccessibilityRole::List,
        "listitem" => AccessibilityRole::ListItem,
        "heading" => AccessibilityRole::Heading,
        "checkbox" => AccessibilityRole::CheckBox,
        "radio" => AccessibilityRole::RadioButton,
        "radiogroup" => AccessibilityRole::RadioGroup,
        "switch" => AccessibilityRole::Switch,
        "dialog" => AccessibilityRole::Dialog,
        "alertdialog" => AccessibilityRole::AlertDialog,
        "menu" => AccessibilityRole::Menu,
        "menuitem" => AccessibilityRole::MenuItem,
        "separator" => AccessibilityRole::Separator,
        "group" => AccessibilityRole::Group,
        "region" => AccessibilityRole::Region,
        "listbox" => AccessibilityRole::ListBox,
        "option" => AccessibilityRole::ListBoxOption,
        "combobox" => AccessibilityRole::ComboBox,
        "table" => AccessibilityRole::Table,
        "tree" => AccessibilityRole::Tree,
        "grid" => AccessibilityRole::Grid,
        "row" => AccessibilityRole::Row,
        "columnheader" => AccessibilityRole::ColumnHeader,
        "rowheader" => AccessibilityRole::RowHeader,
        "gridcell" => AccessibilityRole::GridCell,
        "treeitem" => AccessibilityRole::TreeItem,
        "tab" => AccessibilityRole::Tab,
        "tablist" => AccessibilityRole::TabList,
        "tabpanel" => AccessibilityRole::TabPanel,
        "tooltip" => AccessibilityRole::Tooltip,
        "form" => AccessibilityRole::Form,
        "label" => AccessibilityRole::Label,
        _ => return None,
    })
}

struct NativeWindowRuntime {
    config: AppConfig,
    tree: Rc<RefCell<NativeTree>>,
    markdown: Rc<RefCell<HashMap<u32, Markdown>>>,
    svgs: Rc<RefCell<HashMap<u32, NativeSvgState>>>,
    lists: Rc<RefCell<HashMap<u32, NativeListState>>>,
    terminals: Rc<RefCell<HashMap<u32, NativeTerminalState>>>,
    handle: Option<WindowHandle>,
}

impl NativeWindowRuntime {
    fn view(
        &self,
        window: u32,
        events: &EventQueue,
        handles: &Rc<RefCell<HashMap<WindowHandle, u32>>>,
    ) -> NativeView {
        NativeView {
            window,
            handles: Some(Rc::clone(handles)),
            tree: Rc::clone(&self.tree),
            events: Rc::clone(events),
            markdown: Rc::clone(&self.markdown),
            svgs: Rc::clone(&self.svgs),
            lists: Rc::clone(&self.lists),
            terminals: Rc::clone(&self.terminals),
        }
    }
}

fn native_tree_from_initial_batch(batch: &[u8]) -> std::result::Result<NativeTree, String> {
    if batch.is_empty() {
        return Ok(NativeTree::default());
    }
    if batch.len() > MAX_BATCH_BYTES {
        return Err(format!(
            "initial mutation batch exceeds {MAX_BATCH_BYTES} bytes"
        ));
    }
    let mutations = decode_batch(batch).map_err(|error| error.to_string())?;
    let mut tree = NativeTree::default();
    apply_mutations(&mut tree, mutations).map_err(|error| error.to_string())?;
    Ok(tree)
}

struct NativeRuntime {
    next_window_id: u32,
    windows: HashMap<u32, NativeWindowRuntime>,
    window_order: Vec<u32>,
    events: EventQueue,
    handles: Rc<RefCell<HashMap<WindowHandle, u32>>>,
    closed_windows: Rc<RefCell<Vec<u32>>>,
    pending_dialogs: Vec<PendingDialog>,
    pending_shell: Vec<system::PendingShell>,
    pending_notification_permissions: Vec<system::PendingNotificationPermission>,
    pending_file_icons: Vec<system::PendingFileIcon>,
    pending_user_tasks: Vec<system::PendingUserTasks>,
    pending_global_shortcuts: Vec<system::PendingGlobalShortcut>,
    pending_tray: Vec<system::PendingTray>,
    system_observation: system::SystemObservation,
    app_info: Option<AppInfo>,
    app_paths: Option<AppPaths>,
    quit_mode: QuitMode,
    fonts: Vec<Arc<[u8]>>,
    runner: Option<AppRunner>,
}

impl NativeRuntime {
    fn new(options: NativeAppOptions) -> std::result::Result<Self, String> {
        let fonts = native_font_data(&options).unwrap_or_default();
        let (app_info, app_paths, quit_mode) = native_app_configuration(options)?;
        Ok(Self {
            next_window_id: 1,
            windows: HashMap::new(),
            window_order: Vec::with_capacity(2),
            events: Rc::new(RefCell::new(VecDeque::with_capacity(32))),
            handles: Rc::new(RefCell::new(HashMap::with_capacity(2))),
            closed_windows: Rc::new(RefCell::new(Vec::with_capacity(2))),
            pending_dialogs: Vec::with_capacity(2),
            pending_shell: Vec::with_capacity(2),
            pending_notification_permissions: Vec::with_capacity(1),
            pending_file_icons: Vec::with_capacity(1),
            pending_user_tasks: Vec::with_capacity(1),
            pending_global_shortcuts: Vec::with_capacity(2),
            pending_tray: Vec::with_capacity(2),
            system_observation: system::SystemObservation::default(),
            app_info,
            app_paths,
            quit_mode,
            fonts,
            runner: None,
        })
    }

    fn create_window(
        &mut self,
        options: NativeWindowOptions,
        initial_batch: &[u8],
    ) -> std::result::Result<u32, String> {
        self.sync_closed_windows();
        if self.windows.len() >= MAX_WINDOWS {
            return Err(format!(
                "an application cannot own more than {MAX_WINDOWS} windows"
            ));
        }
        let id = self.next_window_id.max(1);
        self.next_window_id = id
            .checked_add(1)
            .ok_or_else(|| "QuickGUI window id space exhausted".to_owned())?;
        let config = window_config(&options)?;
        let mut window = NativeWindowRuntime {
            config,
            tree: Rc::new(RefCell::new(native_tree_from_initial_batch(initial_batch)?)),
            markdown: Rc::new(RefCell::new(HashMap::new())),
            svgs: Rc::new(RefCell::new(HashMap::new())),
            lists: Rc::new(RefCell::new(HashMap::new())),
            terminals: Rc::new(RefCell::new(HashMap::new())),
            handle: None,
        };
        if let Some(runner) = &mut self.runner {
            let handle = runner
                .open_window(
                    window.view(id, &self.events, &self.handles),
                    window.config.clone(),
                )
                .map_err(|error| error.to_string())?;
            self.handles.borrow_mut().insert(handle, id);
            window.handle = Some(handle);
        }
        self.windows.insert(id, window);
        self.window_order.push(id);
        Ok(id)
    }

    fn create_system_popover(
        &mut self,
        parent: u32,
        anchor: u32,
        options: NativeWindowOptions,
        initial_batch: &[u8],
    ) -> std::result::Result<u32, String> {
        self.sync_closed_windows();
        if self.windows.len() >= MAX_WINDOWS {
            return Err(format!(
                "an application cannot own more than {MAX_WINDOWS} windows"
            ));
        }
        let parent_handle = {
            let parent_window = self
                .windows
                .get(&parent)
                .ok_or_else(|| format!("unknown QuickGUI parent window {parent}"))?;
            if !parent_window.tree.borrow().nodes.contains_key(&anchor) {
                return Err(format!(
                    "system popover anchor node {anchor} is not mounted in parent window {parent}"
                ));
            }
            parent_window
                .handle
                .ok_or_else(|| "a system popover requires a running parent window".to_owned())?
        };
        let id = self.next_window_id.max(1);
        self.next_window_id = id
            .checked_add(1)
            .ok_or_else(|| "QuickGUI window id space exhausted".to_owned())?;
        let config = system_popover_config(&options)?;
        let mut window = NativeWindowRuntime {
            config,
            tree: Rc::new(RefCell::new(native_tree_from_initial_batch(initial_batch)?)),
            markdown: Rc::new(RefCell::new(HashMap::new())),
            svgs: Rc::new(RefCell::new(HashMap::new())),
            lists: Rc::new(RefCell::new(HashMap::new())),
            terminals: Rc::new(RefCell::new(HashMap::new())),
            handle: None,
        };
        let runner = self
            .runner
            .as_mut()
            .ok_or_else(|| "a system popover requires a running application".to_owned())?;
        let handle = runner
            .open_system_popover(
                parent_handle,
                ElementId::new(anchor as u64),
                window.view(id, &self.events, &self.handles),
                window.config.clone(),
            )
            .map_err(|error| error.to_string())?;
        self.handles.borrow_mut().insert(handle, id);
        window.handle = Some(handle);
        self.windows.insert(id, window);
        self.window_order.push(id);
        Ok(id)
    }

    fn prepare(&mut self) -> std::result::Result<(), String> {
        if self.runner.is_some() {
            return self.finish_preparing();
        }
        let staged = self
            .window_order
            .iter()
            .filter_map(|id| {
                self.windows.get(id).map(|window| {
                    (
                        *id,
                        window.config.clone(),
                        window.view(*id, &self.events, &self.handles),
                    )
                })
            })
            .collect::<Vec<_>>();
        let handles = Rc::clone(&self.handles);
        let callback_handles = Rc::clone(&self.handles);
        let callback_events = Rc::clone(&self.events);
        let open_url_events = Rc::clone(&self.events);
        let reopen_events = Rc::clone(&self.events);
        let wake_events = Rc::clone(&self.events);
        let keyboard_events = Rc::clone(&self.events);
        let notification_events = Rc::clone(&self.events);
        let global_shortcut_events = Rc::clone(&self.events);
        let second_instance_events = Rc::clone(&self.events);
        let power_events = Rc::clone(&self.events);
        let tray_events = Rc::clone(&self.events);
        let closed_windows = Rc::clone(&self.closed_windows);
        let mut application = QuickGuiApplication::new().quit_mode(self.quit_mode);
        if let Some(info) = self.app_info.clone() {
            application = application.app_info(info);
        }
        if let Some(paths) = self.app_paths.clone() {
            application = application.app_paths(paths);
        }
        application = application.fonts(self.fonts.iter().cloned());
        let mut runner = application
            .on_open_urls(move |urls, _cx| {
                let value = serde_json::to_string(&urls.iter().collect::<Vec<_>>())
                    .ok()
                    .map(Arc::<str>::from);
                enqueue_event(
                    &open_url_events,
                    QueuedEvent {
                        kind: "open-urls",
                        window: 0,
                        target: ROOT_NODE,
                        value,
                    },
                );
            })
            .on_reopen(move |has_visible_windows, _cx| {
                enqueue_event(
                    &reopen_events,
                    QueuedEvent {
                        kind: "reopen",
                        window: 0,
                        target: ROOT_NODE,
                        value: Some(Arc::from(if has_visible_windows {
                            "true"
                        } else {
                            "false"
                        })),
                    },
                );
            })
            .on_system_wake(move |_cx| {
                enqueue_event(
                    &wake_events,
                    QueuedEvent {
                        kind: "system-wake",
                        window: 0,
                        target: ROOT_NODE,
                        value: None,
                    },
                );
            })
            .on_keyboard_layout_change(move |_layout, _cx| {
                enqueue_event(
                    &keyboard_events,
                    QueuedEvent {
                        kind: "keyboard-layout-change",
                        window: 0,
                        target: ROOT_NODE,
                        value: None,
                    },
                );
            })
            .on_system_notification_response(move |response, _cx| {
                let value = serde_json::json!({
                    "tag": response.tag.as_ref(),
                    "actionId": response.action_id.as_deref(),
                    "reply": response.reply.as_deref(),
                })
                .to_string();
                enqueue_event(
                    &notification_events,
                    QueuedEvent {
                        kind: "notification-response",
                        window: 0,
                        target: ROOT_NODE,
                        value: Some(Arc::from(value)),
                    },
                );
            })
            .on_global_shortcut(move |shortcut, _cx| {
                enqueue_event(
                    &global_shortcut_events,
                    QueuedEvent {
                        kind: "global-shortcut",
                        window: 0,
                        target: shortcut.registration_id,
                        value: None,
                    },
                );
            })
            .on_second_instance(move |instance, _cx| {
                let value = serde_json::json!({
                    "argv": instance
                        .argv()
                        .iter()
                        .map(AsRef::as_ref)
                        .collect::<Vec<&str>>(),
                    "cwd": instance.cwd().to_string_lossy(),
                })
                .to_string();
                enqueue_event(
                    &second_instance_events,
                    QueuedEvent {
                        kind: "second-instance",
                        window: 0,
                        target: ROOT_NODE,
                        value: Some(Arc::from(value)),
                    },
                );
            })
            .on_power_event(move |event, _cx| {
                let value = match event {
                    quickgui::PowerEvent::Suspend => serde_json::json!({ "type": "suspend" }),
                    quickgui::PowerEvent::Resume => serde_json::json!({ "type": "resume" }),
                    quickgui::PowerEvent::LockScreen => {
                        serde_json::json!({ "type": "lock-screen" })
                    }
                    quickgui::PowerEvent::UnlockScreen => {
                        serde_json::json!({ "type": "unlock-screen" })
                    }
                    quickgui::PowerEvent::ShutdownRequested => {
                        serde_json::json!({ "type": "shutdown-requested" })
                    }
                    quickgui::PowerEvent::PowerSourceChanged(source) => serde_json::json!({
                        "type": "power-source-changed",
                        "source": match source {
                            quickgui::PowerSource::Ac => "ac",
                            quickgui::PowerSource::Battery => "battery",
                            quickgui::PowerSource::Unknown => "unknown",
                        },
                    }),
                    quickgui::PowerEvent::ThermalStateChanged(state) => serde_json::json!({
                        "type": "thermal-state-changed",
                        "state": match state {
                            quickgui::ThermalState::Unknown => "unknown",
                            quickgui::ThermalState::Nominal => "nominal",
                            quickgui::ThermalState::Fair => "fair",
                            quickgui::ThermalState::Serious => "serious",
                            quickgui::ThermalState::Critical => "critical",
                        },
                    }),
                    quickgui::PowerEvent::LowPowerModeChanged(enabled) => serde_json::json!({
                        "type": "low-power-mode-changed",
                        "enabled": enabled,
                    }),
                    quickgui::PowerEvent::CpuSpeedLimitChanged(percent) => serde_json::json!({
                        "type": "cpu-speed-limit-changed",
                        "percent": percent,
                    }),
                }
                .to_string();
                enqueue_event(
                    &power_events,
                    QueuedEvent {
                        kind: "power-event",
                        window: 0,
                        target: ROOT_NODE,
                        value: Some(Arc::from(value)),
                    },
                );
            })
            .on_tray_event(move |event, _cx| {
                let kind = match event.kind {
                    quickgui::TrayEventKind::Click => "click",
                    quickgui::TrayEventKind::DoubleClick => "double-click",
                    quickgui::TrayEventKind::Enter => "enter",
                    quickgui::TrayEventKind::Move => "move",
                    quickgui::TrayEventKind::Leave => "leave",
                    quickgui::TrayEventKind::MenuItem => "menu-item",
                    quickgui::TrayEventKind::Scroll => "scroll",
                };
                let button = event.button.map(|button| match button {
                    quickgui::TrayMouseButton::Left => "left",
                    quickgui::TrayMouseButton::Right => "right",
                    quickgui::TrayMouseButton::Middle => "middle",
                });
                let value = serde_json::json!({
                    "kind": kind,
                    "menuItemId": event.menu_item_id,
                    "button": button,
                    "position": event.position.map(|(x, y)| serde_json::json!({ "x": x, "y": y })),
                    "pressed": event.pressed,
                    "scrollDelta": event.scroll_delta,
                    "horizontal": event.horizontal,
                })
                .to_string();
                enqueue_event(
                    &tray_events,
                    QueuedEvent {
                        kind: "tray-event",
                        window: 0,
                        target: event.tray_id,
                        value: Some(Arc::from(value)),
                    },
                );
            })
            .on_window_closed(move |handle, _cx| {
                let Some(window) = callback_handles.borrow_mut().remove(&handle) else {
                    return;
                };
                enqueue_event(
                    &callback_events,
                    QueuedEvent {
                        kind: "close",
                        window,
                        target: ROOT_NODE,
                        value: None,
                    },
                );
                let mut closed = closed_windows.borrow_mut();
                if closed.len() < MAX_WINDOWS {
                    closed.push(window);
                }
            })
            .into_runner()
            .map_err(|error| error.to_string())?;
        let mut mounted = Vec::with_capacity(self.windows.len());
        for (id, config, view) in staged {
            let handle = match runner.open_window(view, config) {
                Ok(handle) => handle,
                Err(error) => {
                    handles.borrow_mut().clear();
                    return Err(error.to_string());
                }
            };
            handles.borrow_mut().insert(handle, id);
            mounted.push((id, handle));
        }
        for (id, handle) in mounted {
            if let Some(window) = self.windows.get_mut(&id) {
                window.handle = Some(handle);
            }
        }
        self.runner = Some(runner);
        self.finish_preparing()
    }

    fn finish_preparing(&mut self) -> std::result::Result<(), String> {
        let runner = self
            .runner
            .as_mut()
            .ok_or_else(|| "the QuickGUI application runner is unavailable".to_owned())?;
        for _ in 0..8 {
            if runner.is_ready() {
                return Ok(());
            }
            match runner
                .pump(Some(Duration::ZERO))
                .map_err(|error| error.to_string())?
            {
                AppRunStatus::Continue => {}
                AppRunStatus::Exited(code) => {
                    return Err(format!(
                        "the QuickGUI application exited with code {code} before becoming ready"
                    ));
                }
            }
        }
        Err("the native QuickGUI application did not become ready".to_owned())
    }

    fn is_ready(&self) -> bool {
        self.runner.as_ref().is_some_and(AppRunner::is_ready)
    }

    fn close_window(&mut self, window: u32) -> bool {
        self.sync_closed_windows();
        let Some(handle) = self.windows.get(&window).and_then(|window| window.handle) else {
            if self.windows.remove(&window).is_none() {
                return false;
            }
            self.window_order.retain(|id| *id != window);
            enqueue_event(
                &self.events,
                QueuedEvent {
                    kind: "close",
                    window,
                    target: ROOT_NODE,
                    value: None,
                },
            );
            return true;
        };
        let Some(runner) = &mut self.runner else {
            return false;
        };
        if !runner.close_window(handle) {
            return false;
        }
        self.handles.borrow_mut().remove(&handle);
        self.windows.remove(&window);
        self.window_order.retain(|id| *id != window);
        enqueue_event(
            &self.events,
            QueuedEvent {
                kind: "close",
                window,
                target: ROOT_NODE,
                value: None,
            },
        );
        true
    }

    fn apply_batch(&mut self, window: u32, batch: &[u8]) -> std::result::Result<u32, String> {
        let mutations = decode_batch(batch).map_err(|error| error.to_string())?;
        self.sync_closed_windows();
        let native_window = self
            .windows
            .get(&window)
            .ok_or_else(|| format!("unknown QuickGUI window {window}"))?;
        let revision = apply_mutations(&mut native_window.tree.borrow_mut(), mutations)
            .map_err(|error| error.to_string())?;
        if let (Some(runner), Some(handle)) = (&mut self.runner, native_window.handle) {
            runner.invalidate_window(handle);
        }
        Ok(revision)
    }

    fn focus_node(&mut self, window: u32, node: u32) -> std::result::Result<bool, String> {
        self.sync_closed_windows();
        let native_window = self
            .windows
            .get(&window)
            .ok_or_else(|| format!("unknown QuickGUI window {window}"))?;
        if !native_window.tree.borrow().nodes.contains_key(&node) {
            return Ok(false);
        }
        let Some(handle) = native_window.handle else {
            return Ok(false);
        };
        let Some(runner) = &mut self.runner else {
            return Ok(false);
        };
        Ok(runner.focus_element(handle, ElementId::new(node as u64)))
    }

    fn show_alert_dialog(
        &mut self,
        window: Option<u32>,
        request: u32,
        options: NativeDialogOptions,
    ) -> std::result::Result<(), String> {
        let handle = self.dialog_handle(window, request)?;
        let (level, buttons) = native_dialog_configuration(&options)?;
        let runner = self
            .runner
            .as_mut()
            .ok_or_else(|| "a native dialog requires a running application".to_owned())?;
        let response = match handle {
            Some(handle) => runner.prompt(
                handle,
                level,
                options.message,
                options.detail.as_deref(),
                &buttons,
            ),
            None => runner.prompt_application(
                level,
                options.message,
                options.detail.as_deref(),
                &buttons,
            ),
        }
        .map_err(|error| error.to_string())?;
        self.pending_dialogs
            .push(PendingDialog::alert(window, request, response));
        Ok(())
    }

    fn show_open_dialog(
        &mut self,
        window: Option<u32>,
        request: u32,
        options: NativeOpenDialogOptions,
    ) -> std::result::Result<(), String> {
        let handle = self.dialog_handle(window, request)?;
        let native_options = native_open_dialog_options(options);
        let runner = self
            .runner
            .as_mut()
            .ok_or_else(|| "a native file dialog requires a running application".to_owned())?;
        let response = match handle {
            Some(handle) => runner.prompt_for_paths(handle, native_options),
            None => runner.prompt_for_paths_application(native_options),
        }
        .map_err(|error| error.to_string())?;
        self.pending_dialogs
            .push(PendingDialog::open(window, request, response));
        Ok(())
    }

    fn show_save_dialog(
        &mut self,
        window: Option<u32>,
        request: u32,
        options: NativeSaveDialogOptions,
    ) -> std::result::Result<(), String> {
        let handle = self.dialog_handle(window, request)?;
        let native_options = native_save_dialog_options(options);
        let runner = self
            .runner
            .as_mut()
            .ok_or_else(|| "a native file dialog requires a running application".to_owned())?;
        let response = match handle {
            Some(handle) => runner.prompt_for_new_path(handle, native_options),
            None => runner.prompt_for_new_path_application(native_options),
        }
        .map_err(|error| error.to_string())?;
        self.pending_dialogs
            .push(PendingDialog::save(window, request, response));
        Ok(())
    }

    fn dialog_handle(
        &mut self,
        window: Option<u32>,
        request: u32,
    ) -> std::result::Result<Option<WindowHandle>, String> {
        self.sync_closed_windows();
        if request == 0
            || self
                .pending_dialogs
                .iter()
                .any(|dialog| dialog.request() == request)
        {
            return Err("native dialog request ids must be nonzero and unique".to_owned());
        }
        window
            .map(|window| {
                self.windows
                    .get(&window)
                    .ok_or_else(|| format!("unknown QuickGUI window {window}"))?
                    .handle
                    .map(Some)
                    .ok_or_else(|| "a native dialog requires a running window".to_owned())
            })
            .unwrap_or(Ok(None))
    }

    fn sync_closed_windows(&mut self) {
        let closed = std::mem::take(&mut *self.closed_windows.borrow_mut());
        if closed.is_empty() {
            return;
        }
        for id in &closed {
            self.windows.remove(id);
        }
        self.window_order.retain(|id| !closed.contains(id));
    }

    fn drain_events(&mut self) -> Vec<NativeEvent> {
        self.observe_system_state();
        let mut events = Vec::with_capacity(
            self.events.borrow().len()
                + self.pending_dialogs.len()
                + self.pending_shell.len()
                + self.pending_notification_permissions.len()
                + self.pending_file_icons.len()
                + self.pending_user_tasks.len()
                + self.pending_global_shortcuts.len()
                + self.pending_tray.len(),
        );
        let waker = Waker::noop();
        let mut context = Context::from_waker(waker);
        let mut still_pending = Vec::with_capacity(self.pending_dialogs.len());
        for mut dialog in std::mem::take(&mut self.pending_dialogs) {
            match dialog.poll(&mut context) {
                Poll::Ready(event) => events.push(event),
                Poll::Pending => still_pending.push(dialog),
            }
        }
        self.pending_dialogs = still_pending;
        let mut still_pending = Vec::with_capacity(self.pending_shell.len());
        for mut request in std::mem::take(&mut self.pending_shell) {
            match request.poll(&mut context) {
                Poll::Ready(event) => events.push(event),
                Poll::Pending => still_pending.push(request),
            }
        }
        self.pending_shell = still_pending;
        let mut still_pending = Vec::with_capacity(self.pending_notification_permissions.len());
        for mut request in std::mem::take(&mut self.pending_notification_permissions) {
            match request.poll(&mut context) {
                Poll::Ready(event) => events.push(event),
                Poll::Pending => still_pending.push(request),
            }
        }
        self.pending_notification_permissions = still_pending;
        let mut still_pending = Vec::with_capacity(self.pending_file_icons.len());
        for mut request in std::mem::take(&mut self.pending_file_icons) {
            match request.poll(&mut context) {
                Poll::Ready(event) => events.push(event),
                Poll::Pending => still_pending.push(request),
            }
        }
        self.pending_file_icons = still_pending;
        let mut still_pending = Vec::with_capacity(self.pending_user_tasks.len());
        for mut request in std::mem::take(&mut self.pending_user_tasks) {
            match request.poll(&mut context) {
                Poll::Ready(event) => events.push(event),
                Poll::Pending => still_pending.push(request),
            }
        }
        self.pending_user_tasks = still_pending;
        let mut still_pending = Vec::with_capacity(self.pending_global_shortcuts.len());
        for mut request in std::mem::take(&mut self.pending_global_shortcuts) {
            match request.poll(&mut context) {
                Poll::Ready(event) => events.push(event),
                Poll::Pending => still_pending.push(request),
            }
        }
        self.pending_global_shortcuts = still_pending;
        let mut still_pending = Vec::with_capacity(self.pending_tray.len());
        for mut request in std::mem::take(&mut self.pending_tray) {
            match request.poll(&mut context) {
                Poll::Ready(event) => events.push(event),
                Poll::Pending => still_pending.push(request),
            }
        }
        self.pending_tray = still_pending;
        events.extend(self.events.borrow_mut().drain(..).map(|event| NativeEvent {
            kind: event.kind.to_owned(),
            window: event.window,
            target: event.target,
            value: event.value.map(|value| value.to_string()),
            paths: None,
            data: None,
            width: None,
            height: None,
            error: None,
        }));
        events
    }
}

pub(crate) fn native_font_data(options: &NativeAppOptions) -> Option<Vec<Arc<[u8]>>> {
    options.font_data.as_ref().map(|fonts| {
        fonts
            .iter()
            .map(|font| Arc::<[u8]>::from(font.as_ref()))
            .collect()
    })
}

pub(crate) fn native_app_configuration(
    options: NativeAppOptions,
) -> std::result::Result<(Option<AppInfo>, Option<AppPaths>, QuitMode), String> {
    update_native_app_configuration(None, None, QuitMode::LastWindowClosed, options)
}

pub(crate) fn update_native_app_configuration(
    current_info: Option<AppInfo>,
    current_paths: Option<AppPaths>,
    current_quit_mode: QuitMode,
    options: NativeAppOptions,
) -> std::result::Result<(Option<AppInfo>, Option<AppPaths>, QuitMode), String> {
    let NativeAppOptions {
        name,
        version,
        identifier,
        resource_dir,
        config_dir,
        data_dir,
        local_data_dir,
        cache_dir,
        log_dir,
        runtime_dir,
        temp_dir,
        quit_mode,
        font_data: _,
    } = options;
    let identity_replaced = name.is_some() || version.is_some() || identifier.is_some();
    let info = match (name, version, identifier) {
        (None, None, None) => None,
        (Some(name), Some(version), Some(identifier)) => {
            Some(AppInfo::new(name, version, identifier).map_err(|error| error.to_string())?)
        }
        _ => {
            return Err(
                "application name, version, and identifier must be supplied together".to_owned(),
            );
        }
    }
    .or(current_info);
    let has_path_overrides = resource_dir.is_some()
        || config_dir.is_some()
        || data_dir.is_some()
        || local_data_dir.is_some()
        || cache_dir.is_some()
        || log_dir.is_some()
        || runtime_dir.is_some()
        || temp_dir.is_some();
    let mut paths = if identity_replaced {
        info.as_ref()
            .map(AppInfo::paths)
            .transpose()
            .map_err(|error| error.to_string())?
    } else if let Some(paths) = current_paths {
        Some(paths)
    } else {
        info.as_ref()
            .map(AppInfo::paths)
            .transpose()
            .map_err(|error| error.to_string())?
    };
    if has_path_overrides && paths.is_none() {
        return Err("application path overrides require application identity".to_owned());
    }
    if let Some(mut configured) = paths.take() {
        if let Some(path) = resource_dir {
            configured = configured.with_resource_dir(path);
        }
        if let Some(path) = config_dir {
            configured = configured.with_config_dir(Some(path));
        }
        if let Some(path) = data_dir {
            configured = configured.with_data_dir(Some(path));
        }
        if let Some(path) = local_data_dir {
            configured = configured.with_local_data_dir(Some(path));
        }
        if let Some(path) = cache_dir {
            configured = configured.with_cache_dir(Some(path));
        }
        if let Some(path) = log_dir {
            configured = configured.with_log_dir(Some(path));
        }
        if let Some(path) = runtime_dir {
            configured = configured.with_runtime_dir(Some(path));
        }
        if let Some(path) = temp_dir {
            configured = configured.with_temp_dir(path);
        }
        paths = Some(configured);
    }
    let quit_mode = match quit_mode.as_deref() {
        None => current_quit_mode,
        Some("default") => QuitMode::Default,
        Some("last-window-closed" | "lastWindowClosed") => QuitMode::LastWindowClosed,
        Some("explicit") => QuitMode::Explicit,
        Some(value) => return Err(format!("unknown application quit mode `{value}`")),
    };
    Ok((info, paths, quit_mode))
}

fn window_config(options: &NativeWindowOptions) -> std::result::Result<AppConfig, String> {
    let width = optional_finite(options.width, "window width")?.unwrap_or(960.0);
    let height = optional_finite(options.height, "window height")?.unwrap_or(640.0);
    let title_bar_style = match options.title_bar_style.as_deref() {
        Some("hiddenInset") | Some("hidden-inset") => TitleBarStyle::HiddenInset,
        Some("hidden") => TitleBarStyle::Hidden,
        Some("default") | None => TitleBarStyle::Default,
        Some(value) => return Err(format!("unknown titleBarStyle `{value}`")),
    };
    let background = options
        .background
        .map(unpack_color)
        .unwrap_or_else(|| Color::rgb8(18, 18, 20));
    let background_appearance = if options.blur.unwrap_or(false) {
        WindowBackgroundAppearance::Blurred
    } else if options.transparent.unwrap_or(false) {
        WindowBackgroundAppearance::Transparent
    } else {
        WindowBackgroundAppearance::Opaque
    };
    let mut config = AppConfig::new(
        options
            .title
            .clone()
            .unwrap_or_else(|| "QuickGUI".to_owned()),
    )
    .size(width, height)
    .background(background)
    .window_background(background_appearance)
    .title_bar_style(title_bar_style);

    if let Some((x, y)) = optional_pair(options.x, options.y, "window position")? {
        config = config.position(x, y);
    }
    if let Some(display) = &options.display_id {
        let id = display
            .parse::<u64>()
            .map_err(|_| "displayId must be an unsigned 64-bit integer string".to_owned())?;
        config = config.display(DisplayId::new(id));
    }
    match options.initial_state.as_deref().unwrap_or("normal") {
        "normal" | "windowed" => {}
        "maximized" => config = config.maximized(true),
        "fullscreen" => config = config.fullscreen(true),
        state => return Err(format!("unknown initial window state `{state}`")),
    }

    if options.minimum_size_enabled == Some(false) {
        if options.minimum_width.is_some() || options.minimum_height.is_some() {
            return Err("minimumWidth/minimumHeight cannot accompany minimumSize: null".to_owned());
        }
        config = config.without_minimum_size();
    } else if options.minimum_width.is_some() || options.minimum_height.is_some() {
        let (width, height) = required_pair(
            options.minimum_width,
            options.minimum_height,
            "minimum window size",
        )?;
        config = config.minimum_size(width, height);
    }
    if options.maximum_width.is_some() || options.maximum_height.is_some() {
        let (width, height) = required_pair(
            options.maximum_width,
            options.maximum_height,
            "maximum window size",
        )?;
        config = config.maximum_size(width, height);
    }

    if let Some(path) = &options.represented_file {
        config = config.represented_file(PathBuf::from(path));
    }
    if let Some(edited) = options.document_edited {
        config = config.document_edited(edited);
    }
    if let Some(identifier) = &options.tabbing_identifier {
        config = config.tabbing_identifier(identifier.clone());
    }
    if let Some(profile) = options.performance_profile.as_deref() {
        config = config.performance_profile(match profile {
            "low-power" | "lowPower" => PerformanceProfile::LowPower,
            "balanced" => PerformanceProfile::Balanced,
            "performance" | "high-performance" | "highPerformance" => {
                PerformanceProfile::HighPerformance
            }
            value => return Err(format!("unknown performanceProfile `{value}`")),
        });
    }
    if let Some(appearance) = options.appearance.as_deref() {
        config = match appearance {
            "system" => config.follow_system_appearance(),
            "light" => config.window_appearance(WindowAppearance::Light),
            "dark" => config.window_appearance(WindowAppearance::Dark),
            value => return Err(format!("unknown window appearance `{value}`")),
        };
    }
    if let Some(kind) = options.kind.as_deref() {
        config = config.window_kind(match kind {
            "normal" => WindowKind::Normal,
            "popover" => WindowKind::Popover,
            "floating" => WindowKind::Floating,
            "dialog" => WindowKind::Dialog,
            "system-popover" | "systemPopover" => WindowKind::SystemPopover,
            value => return Err(format!("unknown window kind `{value}`")),
        });
    }
    if let Some(value) = options.focus {
        config = config.focus(value);
    }
    if let Some(value) = options.focusable {
        config = config.focusable(value);
    }
    if let Some(value) = options.show {
        config = config.show(value);
    }
    if let Some(value) = options.movable {
        config = config.movable(value);
    }
    if let Some(value) = options.resizable {
        config = config.resizable(value);
    }
    if let Some(value) = options.minimizable {
        config = config.minimizable(value);
    }
    if let Some(value) = options.maximizable {
        config = config.maximizable(value);
    }
    if let Some(value) = options.closable {
        config = config.closable(value);
    }
    if let Some(value) = options.decorated {
        config = config.decorations(value);
    }
    if let Some(value) = options.shadow {
        config = config.shadow(value);
    }
    if let Some(value) = options.content_protected {
        config = config.content_protected(value);
    }
    if let Some(level) = options.window_level.as_deref() {
        config = match level {
            "automatic" => config.automatic_window_level(),
            "always-on-bottom" | "alwaysOnBottom" => {
                config.window_level(WindowLevel::AlwaysOnBottom)
            }
            "normal" => config.window_level(WindowLevel::Normal),
            "always-on-top" | "alwaysOnTop" => config.window_level(WindowLevel::AlwaysOnTop),
            value => return Err(format!("unknown windowLevel `{value}`")),
        };
    }
    if let Some(value) = options.skip_taskbar {
        config = config.skip_taskbar(value);
    }
    if let Some(value) = options.visible_on_all_workspaces {
        config = config.visible_on_all_workspaces(value);
    }
    if let Some(opacity) = optional_finite(options.opacity, "window opacity")? {
        config = config.opacity(opacity);
    }
    if let Some(icon) = options.icon.clone() {
        config = config.icon(native_image(icon)?);
    }
    if options.taskbar_progress_state.is_some() || options.taskbar_progress.is_some() {
        let state = parse_taskbar_progress_state(
            options
                .taskbar_progress_state
                .as_deref()
                .unwrap_or("normal"),
        )?;
        let progress =
            optional_finite(options.taskbar_progress, "taskbar progress")?.unwrap_or(0.0);
        config = config.taskbar_progress(state, progress);
    }
    match (
        options.taskbar_overlay_icon.clone(),
        options.taskbar_overlay_description.as_deref(),
    ) {
        (Some(icon), Some(description)) => {
            config = config.taskbar_overlay_icon(native_image(icon)?, description);
        }
        (None, None) => {}
        _ => {
            return Err(
                "taskbarOverlayIcon and taskbarOverlayDescription must be supplied together"
                    .to_owned(),
            );
        }
    }
    if let Some(value) = options.cursor_visible {
        config = config.cursor_visible(value);
    }
    if let Some(mode) = options.cursor_grab.as_deref() {
        config = config.cursor_grab(parse_cursor_grab_mode(mode)?);
    }
    if let Some(value) = options.cursor_hit_test {
        config = config.cursor_hit_test(value);
    }
    if let Some((x, y)) = optional_pair(options.cursor_x, options.cursor_y, "cursor position")? {
        config = config.cursor_position(Point::new(x, y));
    }
    if let Some(menu) = &options.menu {
        config = config.window_menus(system::menu::application_menus(menu)?);
    }
    if let Some(value) = optional_finite(options.line_scroll_pixels, "line scroll pixels")? {
        config.line_scroll_pixels = value;
    }
    if let Some(milliseconds) =
        optional_finite(options.key_sequence_timeout_ms, "key sequence timeout")?
    {
        if milliseconds < 0.0 {
            return Err("keySequenceTimeoutMs cannot be negative".to_owned());
        }
        config.key_sequence_timeout = Duration::from_secs_f32(milliseconds / 1_000.0);
    }
    if let Some(value) = options.reduce_motion {
        config = config.reduce_motion(value);
    }

    if let Some((x, y)) = optional_pair(
        options.traffic_light_x,
        options.traffic_light_y,
        "traffic light position",
    )? {
        config = config.traffic_light_position(x, y);
    }
    Ok(config)
}

fn optional_finite(value: Option<f64>, name: &str) -> std::result::Result<Option<f32>, String> {
    value
        .map(|value| {
            let value = value as f32;
            if value.is_finite() {
                Ok(value)
            } else {
                Err(format!("{name} must be finite"))
            }
        })
        .transpose()
}

fn required_pair(
    first: Option<f64>,
    second: Option<f64>,
    name: &str,
) -> std::result::Result<(f32, f32), String> {
    optional_pair(first, second, name)?.ok_or_else(|| format!("{name} requires both values"))
}

fn optional_pair(
    first: Option<f64>,
    second: Option<f64>,
    name: &str,
) -> std::result::Result<Option<(f32, f32)>, String> {
    match (first, second) {
        (None, None) => Ok(None),
        (Some(first), Some(second)) => Ok(Some((
            optional_finite(Some(first), name)?.expect("present finite value"),
            optional_finite(Some(second), name)?.expect("present finite value"),
        ))),
        _ => Err(format!("{name} requires both values")),
    }
}

fn parse_taskbar_progress_state(state: &str) -> std::result::Result<TaskbarProgressState, String> {
    match state {
        "none" => Ok(TaskbarProgressState::None),
        "normal" => Ok(TaskbarProgressState::Normal),
        "indeterminate" => Ok(TaskbarProgressState::Indeterminate),
        "paused" => Ok(TaskbarProgressState::Paused),
        "error" => Ok(TaskbarProgressState::Error),
        value => Err(format!("unknown taskbar progress state `{value}`")),
    }
}

fn parse_cursor_grab_mode(mode: &str) -> std::result::Result<CursorGrabMode, String> {
    match mode {
        "none" => Ok(CursorGrabMode::None),
        "confined" => Ok(CursorGrabMode::Confined),
        "locked" => Ok(CursorGrabMode::Locked),
        value => Err(format!("unknown cursor grab mode `{value}`")),
    }
}

pub(crate) fn native_image(source: NativeImageSource) -> std::result::Result<Image, String> {
    match (source.data, source.path) {
        (Some(data), None) => match (source.width, source.height) {
            (Some(width), Some(height)) => {
                Image::from_rgba(width, height, Arc::<[u8]>::from(data.as_ref()))
            }
            (None, None) => Image::decode(data.as_ref()),
            _ => {
                return Err(
                    "raw image data requires both width and height, or neither for encoded data"
                        .to_owned(),
                );
            }
        },
        (None, Some(path)) if source.width.is_none() && source.height.is_none() => {
            Image::open(PathBuf::from(path))
        }
        (Some(_), Some(_)) => {
            return Err("an image accepts data or path, but not both".to_owned());
        }
        _ => return Err("an image requires data or path".to_owned()),
    }
    .map_err(|error| error.to_string())
}

fn parse_anchor_placement(value: &str) -> Option<AnchorPlacement> {
    match value {
        "top-start" => Some(AnchorPlacement::TopStart),
        "top" => Some(AnchorPlacement::Top),
        "top-end" => Some(AnchorPlacement::TopEnd),
        "bottom-start" => Some(AnchorPlacement::BottomStart),
        "bottom" => Some(AnchorPlacement::Bottom),
        "bottom-end" => Some(AnchorPlacement::BottomEnd),
        "left-start" => Some(AnchorPlacement::LeftStart),
        "left" => Some(AnchorPlacement::Left),
        "left-end" => Some(AnchorPlacement::LeftEnd),
        "right-start" => Some(AnchorPlacement::RightStart),
        "right" => Some(AnchorPlacement::Right),
        "right-end" => Some(AnchorPlacement::RightEnd),
        _ => None,
    }
}

fn system_popover_config(options: &NativeWindowOptions) -> std::result::Result<AppConfig, String> {
    let placement = match options.popover_placement.as_deref() {
        Some(value) => parse_anchor_placement(value)
            .ok_or_else(|| format!("unknown popover placement `{value}`"))?,
        None => AnchorPlacement::BottomStart,
    };
    let mut popover = SystemPopover::new(
        finite_dimension(options.width, 420.0),
        finite_dimension(options.height, 300.0),
    )
    .placement(placement)
    .gap(finite_number(options.popover_gap).unwrap_or(6.0))
    .offset(
        finite_number(options.popover_offset_x).unwrap_or(0.0),
        finite_number(options.popover_offset_y).unwrap_or(0.0),
    )
    .viewport_margin(finite_number(options.popover_viewport_margin).unwrap_or(8.0));
    if let Some(grab) = options.popover_grab {
        popover = popover.grab(grab);
    }
    if let Some(accepts_key_focus) = options.popover_accepts_key_focus {
        popover = popover.accepts_key_focus(accepts_key_focus);
    }
    if let Some(dismiss_on_escape) = options.popover_dismiss_on_escape {
        popover = popover.dismiss_on_escape(dismiss_on_escape);
    }
    if let Some(dismiss_on_pointer_outside) = options.popover_dismiss_on_pointer_outside {
        popover = popover.dismiss_on_pointer_outside(dismiss_on_pointer_outside);
    }
    Ok(popover.window_options(
        options
            .title
            .clone()
            .unwrap_or_else(|| "QuickGUI popover".to_owned()),
    ))
}

#[derive(Default)]
struct Registry {
    next_id: u32,
    apps: HashMap<u32, NativeRuntime>,
}

thread_local! {
    static REGISTRY: RefCell<Registry> = RefCell::new(Registry {
        next_id: 1,
        apps: HashMap::new(),
    });
}

fn with_app_mut<T>(
    app: u32,
    callback: impl FnOnce(&mut NativeRuntime) -> std::result::Result<T, String>,
) -> Result<T> {
    REGISTRY.with(|registry| {
        let mut registry = registry.borrow_mut();
        let runtime = registry
            .apps
            .get_mut(&app)
            .ok_or_else(|| Error::from_reason(format!("unknown QuickGUI app {app}")))?;
        callback(runtime).map_err(Error::from_reason)
    })
}

#[napi]
pub fn create_app(options: Option<NativeAppOptions>) -> Result<u32> {
    REGISTRY.with(|registry| {
        let mut registry = registry.borrow_mut();
        let id = registry.next_id.max(1);
        registry.next_id = id
            .checked_add(1)
            .ok_or_else(|| Error::from_reason("QuickGUI app id space exhausted"))?;
        let runtime =
            NativeRuntime::new(options.unwrap_or_default()).map_err(Error::from_reason)?;
        registry.apps.insert(id, runtime);
        Ok(id)
    })
}

#[napi]
pub fn create_window(
    app: u32,
    options: Option<NativeWindowOptions>,
    initial_batch: Option<Buffer>,
) -> Result<u32> {
    with_app_mut(app, |runtime| {
        runtime.create_window(
            options.unwrap_or_default(),
            initial_batch.as_deref().unwrap_or_default(),
        )
    })
}

#[napi]
pub fn create_system_popover(
    app: u32,
    parent: u32,
    anchor: u32,
    options: Option<NativeWindowOptions>,
    initial_batch: Option<Buffer>,
) -> Result<u32> {
    with_app_mut(app, |runtime| {
        runtime.create_system_popover(
            parent,
            anchor,
            options.unwrap_or_default(),
            initial_batch.as_deref().unwrap_or_default(),
        )
    })
}

#[napi]
pub fn apply_batch(app: u32, window: u32, batch: Buffer) -> Result<u32> {
    with_app_mut(app, |runtime| runtime.apply_batch(window, &batch))
}

#[napi]
pub fn close_window(app: u32, window: u32) -> Result<bool> {
    with_app_mut(app, |runtime| Ok(runtime.close_window(window)))
}

#[napi]
pub fn focus_node(app: u32, window: u32, node: u32) -> Result<bool> {
    with_app_mut(app, |runtime| runtime.focus_node(window, node))
}

#[napi]
pub fn show_alert_dialog(
    app: u32,
    window: Option<u32>,
    request: u32,
    options: NativeDialogOptions,
) -> Result<()> {
    with_app_mut(app, |runtime| {
        runtime.show_alert_dialog(window, request, options)
    })
}

#[napi]
pub fn show_open_dialog(
    app: u32,
    window: Option<u32>,
    request: u32,
    options: NativeOpenDialogOptions,
) -> Result<()> {
    with_app_mut(app, |runtime| {
        runtime.show_open_dialog(window, request, options)
    })
}

#[napi]
pub fn show_save_dialog(
    app: u32,
    window: Option<u32>,
    request: u32,
    options: NativeSaveDialogOptions,
) -> Result<()> {
    with_app_mut(app, |runtime| {
        runtime.show_save_dialog(window, request, options)
    })
}

#[napi]
pub fn start_app(app: u32) -> Result<()> {
    prepare_app(app)
}

#[napi]
pub fn prepare_app(app: u32) -> Result<()> {
    with_app_mut(app, NativeRuntime::prepare)
}

#[napi]
pub fn is_app_ready(app: u32) -> Result<bool> {
    with_app_mut(app, |runtime| Ok(runtime.is_ready()))
}

/// Return `-1` while running or the non-negative native exit code after termination.
#[napi]
pub fn pump_app(app: u32, timeout_ms: Option<f64>) -> Result<i32> {
    with_app_mut(app, |runtime| {
        let timeout = timeout_ms
            .filter(|value| value.is_finite() && *value >= 0.0)
            .unwrap_or(16.0)
            .min(1_000.0);
        let status = runtime
            .runner
            .as_mut()
            .ok_or_else(|| "prepareApp must be called before pumpApp".to_owned())?
            .pump(Some(Duration::from_secs_f64(timeout / 1_000.0)))
            .map_err(|error| error.to_string())?;
        runtime.sync_closed_windows();
        match status {
            AppRunStatus::Continue => Ok(-1),
            AppRunStatus::Exited(code) => Ok(code.max(0)),
        }
    })
}

#[napi]
pub fn take_events(app: u32) -> Result<Vec<NativeEvent>> {
    with_app_mut(app, |runtime| Ok(runtime.drain_events()))
}

#[napi]
pub fn destroy_app(app: u32) -> Result<bool> {
    REGISTRY.with(|registry| Ok(registry.borrow_mut().apps.remove(&app).is_some()))
}

#[napi]
pub fn create_hosted_app(options: Option<NativeAppOptions>) -> Result<u32> {
    let app = HOST.allocate_app().map_err(Error::from_reason)?;
    HOST.set_app(app).map_err(Error::from_reason)?;
    let reply = Arc::new(SyncReply::new());
    HOST.enqueue(HostCommand::CreateApp {
        app,
        options: options.unwrap_or_default(),
        reply: Arc::clone(&reply),
    })
    .map_err(Error::from_reason)?;
    reply.wait().map_err(Error::from_reason)?;
    Ok(app)
}

#[napi]
pub fn create_hosted_window(
    app: u32,
    options: Option<NativeWindowOptions>,
    initial_batch: Option<Buffer>,
) -> Result<u32> {
    let reply = Arc::new(SyncReply::new());
    HOST.enqueue(HostCommand::CreateWindow {
        app,
        options: options.unwrap_or_default(),
        initial_batch: initial_batch.map_or_else(Vec::new, |batch| batch.to_vec()),
        reply: Arc::clone(&reply),
    })
    .map_err(Error::from_reason)?;
    reply.wait().map_err(Error::from_reason)
}

#[napi]
pub fn create_hosted_system_popover(
    app: u32,
    parent: u32,
    anchor: u32,
    options: Option<NativeWindowOptions>,
    initial_batch: Option<Buffer>,
) -> Result<u32> {
    let reply = Arc::new(SyncReply::new());
    HOST.enqueue(HostCommand::CreateSystemPopover {
        app,
        parent,
        anchor,
        options: options.unwrap_or_default(),
        initial_batch: initial_batch.map_or_else(Vec::new, |batch| batch.to_vec()),
        reply: Arc::clone(&reply),
    })
    .map_err(Error::from_reason)?;
    reply.wait().map_err(Error::from_reason)
}

#[napi]
pub fn apply_hosted_batch(app: u32, window: u32, batch: Buffer) -> Result<u32> {
    if batch.len() > MAX_BATCH_BYTES {
        return Err(Error::from_reason(format!(
            "mutation batch exceeds {MAX_BATCH_BYTES} bytes"
        )));
    }
    HOST.enqueue(HostCommand::ApplyBatch {
        app,
        window,
        batch: batch.to_vec(),
    })
    .map_err(Error::from_reason)?;
    Ok(0)
}

#[napi]
pub fn close_hosted_window(app: u32, window: u32) -> Result<bool> {
    let reply = Arc::new(SyncReply::new());
    HOST.enqueue(HostCommand::CloseWindow {
        app,
        window,
        reply: Arc::clone(&reply),
    })
    .map_err(Error::from_reason)?;
    reply.wait().map_err(Error::from_reason)
}

#[napi]
pub fn focus_hosted_node(app: u32, window: u32, node: u32) -> Result<bool> {
    let reply = Arc::new(SyncReply::new());
    HOST.enqueue(HostCommand::FocusNode {
        app,
        window,
        node,
        reply: Arc::clone(&reply),
    })
    .map_err(Error::from_reason)?;
    reply.wait().map_err(Error::from_reason)
}

#[napi]
pub fn show_hosted_alert_dialog(
    app: u32,
    window: Option<u32>,
    request: u32,
    options: NativeDialogOptions,
) -> Result<()> {
    let reply = Arc::new(SyncReply::new());
    HOST.enqueue(HostCommand::ShowAlertDialog {
        app,
        window,
        request,
        options,
        reply: Arc::clone(&reply),
    })
    .map_err(Error::from_reason)?;
    reply.wait().map_err(Error::from_reason)
}

#[napi]
pub fn show_hosted_open_dialog(
    app: u32,
    window: Option<u32>,
    request: u32,
    options: NativeOpenDialogOptions,
) -> Result<()> {
    let reply = Arc::new(SyncReply::new());
    HOST.enqueue(HostCommand::ShowOpenDialog {
        app,
        window,
        request,
        options,
        reply: Arc::clone(&reply),
    })
    .map_err(Error::from_reason)?;
    reply.wait().map_err(Error::from_reason)
}

#[napi]
pub fn show_hosted_save_dialog(
    app: u32,
    window: Option<u32>,
    request: u32,
    options: NativeSaveDialogOptions,
) -> Result<()> {
    let reply = Arc::new(SyncReply::new());
    HOST.enqueue(HostCommand::ShowSaveDialog {
        app,
        window,
        request,
        options,
        reply: Arc::clone(&reply),
    })
    .map_err(Error::from_reason)?;
    reply.wait().map_err(Error::from_reason)
}

#[napi]
pub fn start_hosted_app(app: u32) -> Result<()> {
    prepare_hosted_app(app)
}

#[napi]
pub fn prepare_hosted_app(app: u32) -> Result<()> {
    let reply = Arc::new(SyncReply::new());
    HOST.enqueue(HostCommand::PrepareApp {
        app,
        reply: Arc::clone(&reply),
    })
    .map_err(Error::from_reason)?;
    reply.wait().map_err(Error::from_reason)
}

#[napi]
pub fn is_hosted_app_ready(app: u32) -> Result<bool> {
    let reply = Arc::new(SyncReply::new());
    HOST.enqueue(HostCommand::IsAppReady {
        app,
        reply: Arc::clone(&reply),
    })
    .map_err(Error::from_reason)?;
    reply.wait().map_err(Error::from_reason)
}

#[napi]
pub fn destroy_hosted_app(app: u32) -> Result<bool> {
    let reply = Arc::new(SyncReply::new());
    HOST.enqueue(HostCommand::DestroyApp {
        app,
        reply: Arc::clone(&reply),
    })
    .map_err(Error::from_reason)?;
    reply.wait().map_err(Error::from_reason)
}

#[doc(hidden)]
pub struct WaitForHostedEvents {
    app: u32,
}

impl Task for WaitForHostedEvents {
    type Output = HostedAppUpdate;
    type JsValue = HostedAppUpdate;

    fn compute(&mut self) -> Result<Self::Output> {
        HOST.wait_for_update(self.app).map_err(Error::from_reason)
    }

    fn resolve(&mut self, _env: Env, output: Self::Output) -> Result<Self::JsValue> {
        Ok(output)
    }
}

#[napi(ts_return_type = "Promise<HostedAppUpdate>")]
pub fn wait_for_hosted_events(app: u32) -> AsyncTask<WaitForHostedEvents> {
    AsyncTask::new(WaitForHostedEvents { app })
}

#[napi]
pub fn abort_app_host(message: String) {
    HOST.fail(message);
}

#[napi]
pub fn run_app_host(on_ready: Option<Function<'_, (), ()>>) -> Result<i32> {
    HOST.begin().map_err(Error::from_reason)?;
    let result = run_app_host_loop(on_ready.as_ref());
    if let Err(error) = &result {
        HOST.fail(error.clone());
    }
    HOST.finish();
    result.map_err(Error::from_reason)
}

fn run_app_host_loop(on_ready: Option<&Function<'_, (), ()>>) -> std::result::Result<i32, String> {
    let mut active_app = None;
    let mut runtime: Option<NativeRuntime> = None;
    let mut ready_reported = false;

    loop {
        let running = runtime
            .as_ref()
            .is_some_and(|runtime| runtime.runner.is_some());
        let mut commands = if running {
            HOST.take_commands()?
        } else {
            HOST.wait_for_commands()?
        };

        while let Some(command) = commands.pop_front() {
            match command {
                HostCommand::CreateApp {
                    app,
                    options,
                    reply,
                } => {
                    let result = if runtime.is_some() {
                        Err("a QuickGUI native host can own only one app".to_owned())
                    } else {
                        match NativeRuntime::new(options) {
                            Ok(created) => {
                                active_app = Some(app);
                                runtime = Some(created);
                                Ok(())
                            }
                            Err(error) => Err(error),
                        }
                    };
                    reply.complete(result);
                }
                HostCommand::CreateWindow {
                    app,
                    options,
                    initial_batch,
                    reply,
                } => {
                    reply.complete(with_hosted_runtime(
                        active_app,
                        runtime.as_mut(),
                        app,
                        |runtime| runtime.create_window(options, &initial_batch),
                    ));
                }
                HostCommand::CreateSystemPopover {
                    app,
                    parent,
                    anchor,
                    options,
                    initial_batch,
                    reply,
                } => {
                    reply.complete(with_hosted_runtime(
                        active_app,
                        runtime.as_mut(),
                        app,
                        |runtime| {
                            runtime.create_system_popover(parent, anchor, options, &initial_batch)
                        },
                    ));
                }
                HostCommand::ApplyBatch { app, window, batch } => {
                    with_hosted_runtime(active_app, runtime.as_mut(), app, |runtime| {
                        runtime.apply_batch(window, &batch)
                    })?;
                }
                HostCommand::CloseWindow { app, window, reply } => {
                    reply.complete(with_hosted_runtime(
                        active_app,
                        runtime.as_mut(),
                        app,
                        |runtime| Ok(runtime.close_window(window)),
                    ));
                }
                HostCommand::FocusNode {
                    app,
                    window,
                    node,
                    reply,
                } => {
                    reply.complete(with_hosted_runtime(
                        active_app,
                        runtime.as_mut(),
                        app,
                        |runtime| runtime.focus_node(window, node),
                    ));
                }
                HostCommand::ShowAlertDialog {
                    app,
                    window,
                    request,
                    options,
                    reply,
                } => {
                    reply.complete(with_hosted_runtime(
                        active_app,
                        runtime.as_mut(),
                        app,
                        |runtime| runtime.show_alert_dialog(window, request, options),
                    ));
                }
                HostCommand::ShowOpenDialog {
                    app,
                    window,
                    request,
                    options,
                    reply,
                } => {
                    reply.complete(with_hosted_runtime(
                        active_app,
                        runtime.as_mut(),
                        app,
                        |runtime| runtime.show_open_dialog(window, request, options),
                    ));
                }
                HostCommand::ShowSaveDialog {
                    app,
                    window,
                    request,
                    options,
                    reply,
                } => {
                    reply.complete(with_hosted_runtime(
                        active_app,
                        runtime.as_mut(),
                        app,
                        |runtime| runtime.show_save_dialog(window, request, options),
                    ));
                }
                HostCommand::System {
                    app,
                    command,
                    reply,
                } => {
                    reply.complete(with_hosted_runtime(
                        active_app,
                        runtime.as_mut(),
                        app,
                        |runtime| runtime.execute_system_command(command),
                    ));
                }
                HostCommand::PrepareApp { app, reply } => {
                    let result = with_hosted_runtime(
                        active_app,
                        runtime.as_mut(),
                        app,
                        NativeRuntime::prepare,
                    );
                    if result.is_ok() {
                        let waker = runtime
                            .as_ref()
                            .and_then(|runtime| runtime.runner.as_ref())
                            .expect("a started QuickGUI host must own an AppRunner")
                            .waker();
                        HOST.set_waker(waker);
                    }
                    reply.complete(result);
                }
                HostCommand::IsAppReady { app, reply } => {
                    reply.complete(with_hosted_runtime(
                        active_app,
                        runtime.as_mut(),
                        app,
                        |runtime| Ok(runtime.is_ready()),
                    ));
                }
                HostCommand::DestroyApp { app, reply } => {
                    let result = if active_app == Some(app) && runtime.is_some() {
                        HOST.publish_exit(0);
                        Ok(true)
                    } else {
                        Ok(false)
                    };
                    reply.complete(result);
                    return Ok(0);
                }
            }
        }

        let Some(runtime) = runtime.as_mut() else {
            continue;
        };
        runtime.sync_closed_windows();
        HOST.publish_events(runtime.drain_events());

        let Some(runner) = runtime.runner.as_mut() else {
            continue;
        };
        let status = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| runner.pump(None)))
            .map_err(|payload| {
                format!(
                    "QuickGUI event loop panicked: {}",
                    panic_payload_message(payload.as_ref())
                )
            })?
            .map_err(|error| error.to_string())?;
        if !ready_reported {
            if let Some(on_ready) = on_ready {
                on_ready.call(()).map_err(|error| error.to_string())?;
            }
            ready_reported = true;
        }
        runtime.sync_closed_windows();
        HOST.publish_events(runtime.drain_events());
        if let AppRunStatus::Exited(code) = status {
            let code = code.max(0);
            HOST.publish_exit(code);
            return Ok(code);
        }
    }
}

fn panic_payload_message(payload: &(dyn Any + Send)) -> &str {
    payload
        .downcast_ref::<String>()
        .map(String::as_str)
        .or_else(|| payload.downcast_ref::<&'static str>().copied())
        .unwrap_or("unknown panic payload")
}

fn with_hosted_runtime<T>(
    active_app: Option<u32>,
    runtime: Option<&mut NativeRuntime>,
    app: u32,
    callback: impl FnOnce(&mut NativeRuntime) -> std::result::Result<T, String>,
) -> std::result::Result<T, String> {
    if active_app != Some(app) {
        return Err(format!("unknown QuickGUI hosted app {app}"));
    }
    callback(runtime.ok_or_else(|| format!("unknown QuickGUI hosted app {app}"))?)
}

#[napi]
pub fn protocol_version() -> u32 {
    PROTOCOL_VERSION as u32
}

fn finite_number(value: Option<f64>) -> Option<f32> {
    value
        .filter(|value| value.is_finite())
        .map(|value| value.clamp(f32::MIN as f64, f32::MAX as f64) as f32)
}

fn finite_dimension(value: Option<f64>, fallback: f32) -> f32 {
    finite_number(value)
        .filter(|value| *value > 0.0)
        .unwrap_or(fallback)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn app_configuration_updates_preserve_embedded_identity_and_paths() {
        let (info, paths, quit_mode) = native_app_configuration(NativeAppOptions {
            name: Some("QuickGUI Test".to_owned()),
            version: Some("1.2.3".to_owned()),
            identifier: Some("dev.quickgui.binding-test".to_owned()),
            ..NativeAppOptions::default()
        })
        .expect("valid initial application configuration");
        let initial_paths = paths.clone().expect("identity resolves application paths");

        let (info, paths, quit_mode) = update_native_app_configuration(
            info,
            paths,
            quit_mode,
            NativeAppOptions {
                quit_mode: Some("explicit".to_owned()),
                ..NativeAppOptions::default()
            },
        )
        .expect("quit-only configuration update");

        assert_eq!(
            info.as_ref().map(AppInfo::identifier),
            Some("dev.quickgui.binding-test")
        );
        assert_eq!(paths.as_ref(), Some(&initial_paths));
        assert_eq!(quit_mode, QuitMode::Explicit);

        let overridden = PathBuf::from("/tmp/quickgui-binding-test-config");
        let (info, paths, quit_mode) = update_native_app_configuration(
            info,
            paths,
            quit_mode,
            NativeAppOptions {
                config_dir: Some(overridden.to_string_lossy().into_owned()),
                ..NativeAppOptions::default()
            },
        )
        .expect("path-only configuration update");

        assert_eq!(
            info.as_ref().map(AppInfo::identifier),
            Some("dev.quickgui.binding-test")
        );
        assert_eq!(
            paths.as_ref().and_then(AppPaths::config_dir),
            Some(overridden.as_path())
        );
        assert_eq!(quit_mode, QuitMode::Explicit);
    }

    struct BatchWriter {
        bytes: Vec<u8>,
        count: u32,
    }

    impl BatchWriter {
        fn new() -> Self {
            let mut bytes = PROTOCOL_MAGIC.to_vec();
            bytes.extend_from_slice(&PROTOCOL_VERSION.to_le_bytes());
            bytes.extend_from_slice(&0_u32.to_le_bytes());
            Self { bytes, count: 0 }
        }

        fn op(&mut self, opcode: u8) {
            self.count += 1;
            self.bytes.push(opcode);
        }

        fn u32(&mut self, value: u32) {
            self.bytes.extend_from_slice(&value.to_le_bytes());
        }

        fn string(&mut self, value: &str) {
            self.u32(value.len() as u32);
            self.bytes.extend_from_slice(value.as_bytes());
        }

        fn finish(mut self) -> Vec<u8> {
            self.bytes[6..10].copy_from_slice(&self.count.to_le_bytes());
            self.bytes
        }
    }

    #[test]
    fn binary_batch_builds_and_removes_a_tree_atomically() {
        let mut writer = BatchWriter::new();
        writer.op(1);
        writer.u32(1);
        writer.bytes.push(1);
        writer.op(2);
        writer.u32(2);
        writer.string("Hello");
        writer.op(6);
        writer.u32(1);
        writer.u32(2);
        writer.u32(NO_ANCHOR);
        writer.op(6);
        writer.u32(ROOT_NODE);
        writer.u32(1);
        writer.u32(NO_ANCHOR);

        let mut tree = NativeTree::default();
        let revision = apply_mutations(&mut tree, decode_batch(&writer.finish()).unwrap()).unwrap();
        assert_eq!(revision, 1);
        assert_eq!(tree.nodes[&ROOT_NODE].children, [1]);
        assert_eq!(tree.nodes[&1].children, [2]);
        assert_eq!(tree.nodes[&2].text.as_ref(), "Hello");

        let mut remove = BatchWriter::new();
        remove.op(7);
        remove.u32(ROOT_NODE);
        remove.u32(1);
        apply_mutations(&mut tree, decode_batch(&remove.finish()).unwrap()).unwrap();
        assert_eq!(tree.nodes.len(), 1);
        assert!(tree.nodes[&ROOT_NODE].children.is_empty());
    }

    #[test]
    fn initial_window_batch_is_committed_before_the_core_view_opens() {
        let mut writer = BatchWriter::new();
        writer.op(2);
        writer.u32(1);
        writer.string("Ready");
        writer.op(6);
        writer.u32(ROOT_NODE);
        writer.u32(1);
        writer.u32(NO_ANCHOR);

        let tree = native_tree_from_initial_batch(&writer.finish()).unwrap();
        assert_eq!(tree.revision, 1);
        assert_eq!(tree.nodes[&ROOT_NODE].children, [1]);
        assert_eq!(tree.nodes[&1].text.as_ref(), "Ready");
    }

    #[test]
    fn failed_cycle_does_not_mutate_the_committed_tree() {
        let mut tree = NativeTree::default();
        let mut setup = BatchWriter::new();
        for id in [1, 2] {
            setup.op(1);
            setup.u32(id);
            setup.bytes.push(1);
        }
        setup.op(6);
        setup.u32(ROOT_NODE);
        setup.u32(1);
        setup.u32(NO_ANCHOR);
        setup.op(6);
        setup.u32(1);
        setup.u32(2);
        setup.u32(NO_ANCHOR);
        apply_mutations(&mut tree, decode_batch(&setup.finish()).unwrap()).unwrap();
        let before = tree.clone();

        let mut cycle = BatchWriter::new();
        cycle.op(6);
        cycle.u32(2);
        cycle.u32(1);
        cycle.u32(NO_ANCHOR);
        assert!(apply_mutations(&mut tree, decode_batch(&cycle.finish()).unwrap()).is_err());
        assert_eq!(tree.revision, before.revision);
        assert_eq!(
            tree.nodes[&ROOT_NODE].children,
            before.nodes[&ROOT_NODE].children
        );
        assert_eq!(tree.nodes[&1].children, before.nodes[&1].children);
    }

    #[test]
    fn malformed_batch_is_rejected_before_tree_mutation() {
        let error = decode_batch(b"not a batch").unwrap_err();
        assert!(error.to_string().contains("magic"));
    }
    #[test]
    fn queued_input_and_submit_survive_until_javascript_commits_the_controlled_value() {
        let input_id = 7;
        let mut tree = NativeTree::default();
        let mut input = NativeNode::new(NodeTag::Input);
        input.parent = Some(ROOT_NODE);
        input.set_property(property::INPUT_LISTENER, Some(PropertyValue::Bool(true)));
        input.set_property(property::SUBMIT_LISTENER, Some(PropertyValue::Bool(true)));
        tree.nodes.insert(input_id, input);
        tree.nodes
            .get_mut(&ROOT_NODE)
            .unwrap()
            .children
            .push(input_id);

        let events = Rc::new(RefCell::new(VecDeque::new()));
        let view = NativeView {
            window: 3,
            handles: None,
            tree: Rc::new(RefCell::new(tree)),
            events: Rc::clone(&events),
            markdown: Rc::new(RefCell::new(HashMap::new())),
            svgs: Rc::new(RefCell::new(HashMap::new())),
            lists: Rc::new(RefCell::new(HashMap::new())),
            terminals: Rc::new(RefCell::new(HashMap::new())),
        };
        let (mut cx, view) = quickgui::TestAppContext::new(view).unwrap();
        let window = view.window_handle();

        cx.focus(window, ElementId::new(input_id as u64)).unwrap();
        cx.simulate_input(window, "hello").unwrap();

        assert_eq!(
            cx.focused_input_value(window).unwrap().as_deref(),
            Some("hello")
        );
        let event = events.borrow_mut().pop_front().unwrap();
        assert_eq!(event.kind, "input");
        assert_eq!(event.window, 3);
        assert_eq!(event.target, input_id);
        assert_eq!(event.value.as_deref(), Some("hello"));

        cx.simulate_keystrokes(window, "enter").unwrap();

        assert_eq!(
            cx.focused(window).unwrap(),
            Some(ElementId::new(input_id as u64))
        );
        assert_eq!(
            cx.focused_input_value(window).unwrap().as_deref(),
            Some("hello")
        );
        let event = events.borrow_mut().pop_front().unwrap();
        assert_eq!(event.kind, "submit");
        assert_eq!(event.window, 3);
        assert_eq!(event.target, input_id);
        assert_eq!(event.value.as_deref(), Some("hello"));
    }

    #[test]
    fn flex_without_direction_uses_css_row_default() {
        let implicit_row_id = 10;
        let row_first_id = 11;
        let row_second_id = 12;
        let explicit_column_id = 20;
        let column_first_id = 21;
        let column_second_id = 22;
        let mut tree = NativeTree::default();

        let mut implicit_row = NativeNode::new(NodeTag::Button);
        implicit_row.parent = Some(ROOT_NODE);
        implicit_row.children.extend([row_first_id, row_second_id]);
        implicit_row.set_property(
            property::DISPLAY,
            Some(PropertyValue::String(Arc::from("flex"))),
        );
        implicit_row.set_property(property::WIDTH, Some(PropertyValue::Number(200.0)));
        implicit_row.set_property(property::HEIGHT, Some(PropertyValue::Number(60.0)));
        tree.nodes.insert(implicit_row_id, implicit_row);

        let mut explicit_column = NativeNode::new(NodeTag::Button);
        explicit_column.parent = Some(ROOT_NODE);
        explicit_column
            .children
            .extend([column_first_id, column_second_id]);
        explicit_column.set_property(
            property::DISPLAY,
            Some(PropertyValue::String(Arc::from("flex"))),
        );
        explicit_column.set_property(
            property::FLEX_DIRECTION,
            Some(PropertyValue::String(Arc::from("column"))),
        );
        explicit_column.set_property(property::WIDTH, Some(PropertyValue::Number(200.0)));
        explicit_column.set_property(property::HEIGHT, Some(PropertyValue::Number(60.0)));
        tree.nodes.insert(explicit_column_id, explicit_column);

        for (id, parent, value) in [
            (row_first_id, implicit_row_id, "Count:"),
            (row_second_id, implicit_row_id, "0"),
            (column_first_id, explicit_column_id, "Count:"),
            (column_second_id, explicit_column_id, "0"),
        ] {
            let mut text = NativeNode::new(NodeTag::Text);
            text.parent = Some(parent);
            text.text = Arc::from(value);
            tree.nodes.insert(id, text);
        }
        tree.nodes
            .get_mut(&ROOT_NODE)
            .unwrap()
            .children
            .extend([implicit_row_id, explicit_column_id]);

        let view = NativeView {
            window: 4,
            handles: None,
            tree: Rc::new(RefCell::new(tree)),
            events: Rc::new(RefCell::new(VecDeque::new())),
            markdown: Rc::new(RefCell::new(HashMap::new())),
            svgs: Rc::new(RefCell::new(HashMap::new())),
            lists: Rc::new(RefCell::new(HashMap::new())),
            terminals: Rc::new(RefCell::new(HashMap::new())),
        };
        let (mut cx, view) = quickgui::TestAppContext::new(view).unwrap();
        let window = view.window_handle();

        let row_first = cx
            .element_bounds(window, ElementId::new(row_first_id as u64))
            .unwrap();
        let row_second = cx
            .element_bounds(window, ElementId::new(row_second_id as u64))
            .unwrap();
        assert_eq!(row_second.y, row_first.y);
        assert!(row_second.x >= row_first.right());

        let column_first = cx
            .element_bounds(window, ElementId::new(column_first_id as u64))
            .unwrap();
        let column_second = cx
            .element_bounds(window, ElementId::new(column_second_id as u64))
            .unwrap();
        assert_eq!(column_second.x, column_first.x);
        assert!(column_second.y >= column_first.bottom());
    }

    #[test]
    fn retained_popover_uses_core_placement_dismissal_and_focus_restoration() {
        let trigger_id = 20;
        let popover_id = 21;
        let mut tree = NativeTree::default();

        let mut trigger = NativeNode::new(NodeTag::Button);
        trigger.parent = Some(ROOT_NODE);
        trigger.set_property(property::WIDTH, Some(PropertyValue::Number(120.0)));
        trigger.set_property(property::HEIGHT, Some(PropertyValue::Number(40.0)));
        tree.nodes.insert(trigger_id, trigger);

        let mut popover = NativeNode::new(NodeTag::View);
        popover.parent = Some(ROOT_NODE);
        popover.set_property(property::WIDTH, Some(PropertyValue::Number(200.0)));
        popover.set_property(property::HEIGHT, Some(PropertyValue::Number(100.0)));
        popover.set_property(
            property::ANCHOR_TARGET,
            Some(PropertyValue::String(Arc::from(trigger_id.to_string()))),
        );
        popover.set_property(
            property::ANCHOR_PLACEMENT,
            Some(PropertyValue::String(Arc::from("bottom-start"))),
        );
        popover.set_property(property::ANCHOR_GAP, Some(PropertyValue::Number(6.0)));
        popover.set_property(property::VIEWPORT_MARGIN, Some(PropertyValue::Number(12.0)));
        popover.set_property(property::DISMISS_LISTENER, Some(PropertyValue::Bool(true)));
        tree.nodes.insert(popover_id, popover);
        tree.nodes
            .get_mut(&ROOT_NODE)
            .unwrap()
            .children
            .extend([trigger_id, popover_id]);

        let events = Rc::new(RefCell::new(VecDeque::new()));
        let view = NativeView {
            window: 5,
            handles: None,
            tree: Rc::new(RefCell::new(tree)),
            events: Rc::clone(&events),
            markdown: Rc::new(RefCell::new(HashMap::new())),
            svgs: Rc::new(RefCell::new(HashMap::new())),
            lists: Rc::new(RefCell::new(HashMap::new())),
            terminals: Rc::new(RefCell::new(HashMap::new())),
        };
        let (mut cx, view) = quickgui::TestAppContext::new(view).unwrap();
        let window = view.window_handle();
        let trigger_element = ElementId::new(trigger_id as u64);
        let popover_element = ElementId::new(popover_id as u64);

        let trigger_bounds = cx.element_bounds(window, trigger_element).unwrap();
        let popover_bounds = cx.element_bounds(window, popover_element).unwrap();
        assert_eq!(popover_bounds.x, 12.0);
        assert_eq!(popover_bounds.y, trigger_bounds.bottom() + 6.0);
        assert_eq!(popover_bounds.width, 200.0);
        assert_eq!(popover_bounds.height, 100.0);

        cx.focus(window, popover_element).unwrap();
        cx.simulate_keystrokes(window, "escape").unwrap();

        assert_eq!(cx.focused(window).unwrap(), Some(trigger_element));
        let event = events.borrow_mut().pop_front().unwrap();
        assert_eq!(event.kind, "dismiss");
        assert_eq!(event.window, 5);
        assert_eq!(event.target, popover_id);

        cx.focus(window, popover_element).unwrap();
        cx.update(view, |view, cx| {
            let mut tree = view.tree.borrow_mut();
            tree.nodes
                .get_mut(&ROOT_NODE)
                .unwrap()
                .children
                .retain(|child| *child != popover_id);
            tree.nodes.remove(&popover_id);
            cx.invalidate();
        })
        .unwrap();

        assert!(!cx.contains_element(window, popover_element).unwrap());
        assert_eq!(cx.focused(window).unwrap(), Some(trigger_element));
    }

    #[test]
    fn unanchored_overlay_traps_autofocus_dismisses_and_restores_previous_focus() {
        let outside_id = 40;
        let overlay_id = 41;
        let surface_id = 42;
        let prompt_id = 43;
        let mut tree = NativeTree::default();

        let mut outside = NativeNode::new(NodeTag::Button);
        outside.parent = Some(ROOT_NODE);
        tree.nodes.insert(outside_id, outside);
        tree.nodes
            .get_mut(&ROOT_NODE)
            .unwrap()
            .children
            .push(outside_id);

        let events = Rc::new(RefCell::new(VecDeque::new()));
        let view = NativeView {
            window: 6,
            handles: None,
            tree: Rc::new(RefCell::new(tree)),
            events: Rc::clone(&events),
            markdown: Rc::new(RefCell::new(HashMap::new())),
            svgs: Rc::new(RefCell::new(HashMap::new())),
            lists: Rc::new(RefCell::new(HashMap::new())),
            terminals: Rc::new(RefCell::new(HashMap::new())),
        };
        let (mut cx, view) = quickgui::TestAppContext::new(view).unwrap();
        let window = view.window_handle();
        let outside_element = ElementId::new(outside_id as u64);
        let prompt_element = ElementId::new(prompt_id as u64);

        cx.focus(window, outside_element).unwrap();
        cx.update(view, |view, cx| {
            let mut tree = view.tree.borrow_mut();

            let mut overlay = NativeNode::new(NodeTag::View);
            overlay.parent = Some(ROOT_NODE);
            overlay.children.push(surface_id);
            overlay.set_property(property::OVERLAY, Some(PropertyValue::Bool(true)));
            overlay.set_property(property::FOCUS_TRAP, Some(PropertyValue::Bool(true)));
            overlay.set_property(
                property::RESTORE_PREVIOUS_FOCUS,
                Some(PropertyValue::Bool(true)),
            );

            let mut surface = NativeNode::new(NodeTag::View);
            surface.parent = Some(overlay_id);
            surface.children.push(prompt_id);
            surface.set_property(
                property::DISMISS_ON_ESCAPE,
                Some(PropertyValue::Bool(true)),
            );
            surface.set_property(
                property::DISMISS_ON_POINTER_OUTSIDE,
                Some(PropertyValue::Bool(true)),
            );
            surface.set_property(
                property::DISMISS_LISTENER,
                Some(PropertyValue::Bool(true)),
            );
            surface.set_property(
                property::ACCESSIBILITY_MODAL,
                Some(PropertyValue::Bool(true)),
            );

            let mut prompt = NativeNode::new(NodeTag::Input);
            prompt.parent = Some(surface_id);
            prompt.set_property(property::MULTILINE, Some(PropertyValue::Bool(true)));
            prompt.set_property(property::AUTO_FOCUS, Some(PropertyValue::Bool(true)));

            tree.nodes.insert(overlay_id, overlay);
            tree.nodes.insert(surface_id, surface);
            tree.nodes.insert(prompt_id, prompt);
            tree.nodes
                .get_mut(&ROOT_NODE)
                .unwrap()
                .children
                .push(overlay_id);
            cx.invalidate();
        })
        .unwrap();

        assert_eq!(cx.focused(window).unwrap(), Some(prompt_element));
        cx.simulate_keystrokes(window, "escape").unwrap();
        let event = events.borrow_mut().pop_front().unwrap();
        assert_eq!(event.kind, "dismiss");
        assert_eq!(event.target, surface_id);

        cx.update(view, |view, cx| {
            let mut tree = view.tree.borrow_mut();
            tree.nodes
                .get_mut(&ROOT_NODE)
                .unwrap()
                .children
                .retain(|child| *child != overlay_id);
            tree.nodes.remove(&prompt_id);
            tree.nodes.remove(&surface_id);
            tree.nodes.remove(&overlay_id);
            cx.invalidate();
        })
        .unwrap();

        assert_eq!(cx.focused(window).unwrap(), Some(outside_element));
    }

    #[test]
    fn native_svg_is_parsed_once_until_its_source_changes() {
        let svg_id = 30;
        let first_source = r#"<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24"><path d="M2 2h20v20H2z"/></svg>"#;
        let second_source = r#"<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24"><circle cx="12" cy="12" r="10"/></svg>"#;
        let mut tree = NativeTree::default();
        let mut icon = NativeNode::new(NodeTag::Svg);
        icon.parent = Some(ROOT_NODE);
        icon.set_property(
            property::VALUE,
            Some(PropertyValue::String(Arc::from(first_source))),
        );
        icon.set_property(property::WIDTH, Some(PropertyValue::Number(16.0)));
        icon.set_property(property::HEIGHT, Some(PropertyValue::Number(16.0)));
        tree.nodes.insert(svg_id, icon);
        tree.nodes
            .get_mut(&ROOT_NODE)
            .unwrap()
            .children
            .push(svg_id);

        let svgs = Rc::new(RefCell::new(HashMap::new()));
        let view = NativeView {
            window: 7,
            handles: None,
            tree: Rc::new(RefCell::new(tree)),
            events: Rc::new(RefCell::new(VecDeque::new())),
            markdown: Rc::new(RefCell::new(HashMap::new())),
            svgs: Rc::clone(&svgs),
            lists: Rc::new(RefCell::new(HashMap::new())),
            terminals: Rc::new(RefCell::new(HashMap::new())),
        };
        let (mut cx, view) = quickgui::TestAppContext::new(view).unwrap();
        let window = view.window_handle();
        let bounds = cx
            .element_bounds(window, ElementId::new(svg_id as u64))
            .unwrap();
        assert_eq!((bounds.width, bounds.height), (16.0, 16.0));

        let first = svgs.borrow()[&svg_id].parsed.as_ref().unwrap().clone();
        cx.update(view, |_view, cx| cx.invalidate()).unwrap();
        let unchanged = svgs.borrow()[&svg_id].parsed.as_ref().unwrap().clone();
        assert_eq!(first, unchanged);

        cx.update(view, |view, cx| {
            view.tree
                .borrow_mut()
                .nodes
                .get_mut(&svg_id)
                .unwrap()
                .set_property(
                    property::VALUE,
                    Some(PropertyValue::String(Arc::from(second_source))),
                );
            cx.invalidate();
        })
        .unwrap();
        let changed = svgs.borrow()[&svg_id].parsed.as_ref().unwrap().clone();
        assert_ne!(first, changed);
    }

    #[test]
    fn native_virtual_list_mounts_only_the_initial_window_and_overscan() {
        let list_id = 40;
        let first_item_id = 1_000;
        let item_count = 100;
        let mut tree = NativeTree::default();
        let mut list = NativeNode::new(NodeTag::VirtualList);
        list.parent = Some(ROOT_NODE);
        list.set_property(
            property::ESTIMATED_ITEM_HEIGHT,
            Some(PropertyValue::Number(24.0)),
        );
        list.set_property(property::OVERSCAN, Some(PropertyValue::Number(2.0)));
        for index in 0..item_count {
            let id = first_item_id + index;
            let mut item = NativeNode::new(NodeTag::Text);
            item.parent = Some(list_id);
            item.text = Arc::from(format!("Item {index}"));
            tree.nodes.insert(id, item);
            list.children.push(id);
        }
        tree.nodes.insert(list_id, list);
        tree.nodes
            .get_mut(&ROOT_NODE)
            .unwrap()
            .children
            .push(list_id);

        let lists = Rc::new(RefCell::new(HashMap::new()));
        let view = NativeView {
            window: 4,
            handles: None,
            tree: Rc::new(RefCell::new(tree)),
            events: Rc::new(RefCell::new(VecDeque::new())),
            markdown: Rc::new(RefCell::new(HashMap::new())),
            svgs: Rc::new(RefCell::new(HashMap::new())),
            lists: Rc::clone(&lists),
            terminals: Rc::new(RefCell::new(HashMap::new())),
        };
        let (cx, view) = quickgui::TestAppContext::new(view).unwrap();
        let window = view.window_handle();

        assert!(
            cx.contains_element(window, ElementId::new(first_item_id as u64))
                .unwrap()
        );
        assert!(
            !cx.contains_element(
                window,
                ElementId::new((first_item_id + item_count - 1) as u64),
            )
            .unwrap()
        );
        let mounted = lists.borrow()[&list_id].list.visible_rows().len();
        assert!(mounted < item_count as usize);
    }

    #[cfg(unix)]
    #[test]
    fn native_terminal_runs_a_real_pty_and_rerenders_ghostty_output() {
        let terminal_id = 50;
        let mut tree = NativeTree::default();
        let mut terminal = NativeNode::new(NodeTag::Terminal);
        terminal.parent = Some(ROOT_NODE);
        terminal.set_property(
            property::TERMINAL_PROGRAM,
            Some(PropertyValue::String(Arc::from("/bin/sh"))),
        );
        terminal.set_property(
            property::TERMINAL_ARGUMENTS,
            Some(PropertyValue::String(Arc::from(
                serde_json::json!(["-c", "printf 'quickgui-pty-ok\\n'"]).to_string(),
            ))),
        );
        terminal.set_property(
            property::TERMINAL_STATUS_LISTENER,
            Some(PropertyValue::Bool(true)),
        );
        terminal.set_property(
            property::POSITION,
            Some(PropertyValue::String(Arc::from("absolute"))),
        );
        terminal.set_property(property::TOP, Some(PropertyValue::Number(8.0)));
        terminal.set_property(property::RIGHT, Some(PropertyValue::Number(9.0)));
        terminal.set_property(property::BOTTOM, Some(PropertyValue::Number(8.0)));
        terminal.set_property(property::LEFT, Some(PropertyValue::Number(9.0)));
        terminal.set_property(property::PADDING_TOP, Some(PropertyValue::Number(8.0)));
        terminal.set_property(property::PADDING_RIGHT, Some(PropertyValue::Number(9.0)));
        terminal.set_property(property::PADDING_BOTTOM, Some(PropertyValue::Number(8.0)));
        terminal.set_property(property::PADDING_LEFT, Some(PropertyValue::Number(9.0)));
        tree.nodes.insert(terminal_id, terminal);
        tree.nodes
            .get_mut(&ROOT_NODE)
            .unwrap()
            .children
            .push(terminal_id);

        let terminals = Rc::new(RefCell::new(HashMap::new()));
        let events = Rc::new(RefCell::new(VecDeque::new()));
        let view = NativeView {
            window: 6,
            handles: None,
            tree: Rc::new(RefCell::new(tree)),
            events: Rc::clone(&events),
            markdown: Rc::new(RefCell::new(HashMap::new())),
            svgs: Rc::new(RefCell::new(HashMap::new())),
            lists: Rc::new(RefCell::new(HashMap::new())),
            terminals: Rc::clone(&terminals),
        };
        let (mut cx, view) = quickgui::TestAppContext::new(view).unwrap();
        let window = view.window_handle();
        for _ in 0..200 {
            let finished_with_output = terminals
                .borrow()
                .get(&terminal_id)
                .and_then(|state| state.terminal.as_ref())
                .is_some_and(|terminal| {
                    let snapshot = terminal.snapshot();
                    matches!(snapshot.status, TerminalStatus::Exited { .. })
                        && snapshot.content.contains("quickgui-pty-ok")
                });
            if finished_with_output {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        cx.update(view, |_view, cx| cx.invalidate()).unwrap();

        let snapshot = terminals.borrow()[&terminal_id]
            .terminal
            .as_ref()
            .unwrap()
            .snapshot();
        assert!(
            snapshot.content.contains("quickgui-pty-ok"),
            "snapshot: {snapshot:#?}"
        );
        assert!(matches!(snapshot.status, TerminalStatus::Exited { .. }));
        let root_bounds = cx
            .element_bounds(window, ElementId::new(ROOT_ELEMENT_ID))
            .unwrap();
        let terminal_bounds = cx
            .element_bounds(window, ElementId::new(terminal_id as u64))
            .unwrap();
        assert_eq!(terminal_bounds.x, root_bounds.x + 9.0);
        assert_eq!(terminal_bounds.y, root_bounds.y + 8.0);
        assert_eq!(terminal_bounds.right(), root_bounds.right() - 9.0);
        assert_eq!(terminal_bounds.bottom(), root_bounds.bottom() - 8.0);
        assert!(
            events
                .borrow()
                .iter()
                .any(|event| event.kind == "terminal" && event.target == terminal_id)
        );
    }
}
