//! A retained, PTY-backed terminal component powered by libghostty-vt.

mod graphics;

use std::{
    cell::RefCell,
    ffi::{OsStr, OsString},
    io::{Read, Write},
    ops::Range,
    path::PathBuf,
    rc::Rc,
    sync::{
        Arc, Mutex, RwLock,
        atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering},
        mpsc::{Receiver, SyncSender, TryRecvError, TrySendError, sync_channel},
    },
    thread,
    time::{Duration, Instant},
};

use libghostty_vt::{
    RenderState, Terminal as GhosttyTerminal, TerminalOptions as GhosttyTerminalOptions,
    key::{
        Action as GhosttyKeyAction, Encoder as GhosttyKeyEncoder, Event as GhosttyKeyEvent,
        Key as GhosttyKey, Mods as GhosttyMods, OptionAsAlt,
    },
    render::{CellIterator, CursorVisualStyle, RowIterator},
    screen::CellWide,
    style::{RgbColor, Underline},
    terminal::{Mode, ScrollViewport},
};
use portable_pty::{CommandBuilder, PtySize, native_pty_system};

use self::graphics::{
    CellMetrics, EdgeBackgrounds, is_block_element, paint_block, paint_padding_extension,
};
use crate::terminal_process::DetectedAgentProcess;
use crate::{
    AccessibilityRole, Color, Element, ElementId, EventContext, FontFamily, GesturePhase,
    HighlightStyle, IntoElement, Key, KeyDownEvent, KeyUpEvent, MAX_TEXT_HIGHLIGHTS, Modifiers,
    MouseButton, PointerPhase, StyledText, ViewContext, WindowInvalidator, canvas, div,
};

/// Maximum UTF-8 bytes accepted for a program, argument, environment entry, or working directory.
pub const MAX_TERMINAL_STRING_BYTES: usize = 32 * 1024;
/// Maximum arguments retained by one terminal process declaration.
pub const MAX_TERMINAL_ARGUMENTS: usize = 256;
/// Maximum environment overrides retained by one terminal process declaration.
pub const MAX_TERMINAL_ENVIRONMENT: usize = 256;
/// Maximum scrollback rows retained by libghostty-vt for one terminal.
pub const MAX_TERMINAL_SCROLLBACK: usize = 100_000;
/// Maximum bytes accepted by one direct write or paste operation.
pub const MAX_TERMINAL_INPUT_BYTES: usize = 1024 * 1024;
/// Number of configurable colors in the standard and bright ANSI palette.
pub const TERMINAL_ANSI_COLOR_COUNT: usize = 16;

const DEFAULT_COLS: u16 = 80;
const DEFAULT_ROWS: u16 = 24;
const DEFAULT_CELL_WIDTH_RATIO: f32 = 0.6;
const MAX_COLS: u16 = 512;
const MAX_ROWS: u16 = 256;
const MESSAGE_CAPACITY: usize = 256;
const READ_CHUNK_BYTES: usize = 16 * 1024;
const WORKER_BATCH_LIMIT: usize = 256;
const AGENT_PROCESS_PROBE_INTERVAL: Duration = Duration::from_millis(500);
const AGENT_RECENT_ACTIVITY: Duration = Duration::from_millis(1_200);
const TERM_PROGRAM_VALUE: &str = "ghostty";
const TERMINAL_SCROLLBAR_HIT_WIDTH: f32 = 12.0;
const TERMINAL_SCROLLBAR_IDLE_WIDTH: f32 = 3.0;
const TERMINAL_SCROLLBAR_HOVER_WIDTH: f32 = 7.0;
const TERMINAL_SCROLLBAR_EDGE_INSET: f32 = 1.0;
const TERMINAL_SCROLLBAR_VERTICAL_INSET: f32 = 2.0;
const TERMINAL_SCROLLBAR_MIN_THUMB: f32 = 24.0;
const TERMINAL_SCROLLBAR_HIDE_DELAY: Duration = Duration::from_millis(900);
const TERMINAL_SCROLLBAR_ID_TAG: u64 = 0x7465_726d_7363_726c;
const TERMINAL_TEXT_ID_TAG: u64 = 0x7465_726d_7465_7874;
const TERMINAL_CURSOR_BLINK_HALF_PERIOD: Duration = Duration::from_millis(500);

/// Process and scrollback configuration for a terminal session.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TerminalOptions {
    /// Program to execute. `None` starts the platform's default shell.
    pub program: Option<OsString>,
    /// Arguments passed without shell interpolation.
    pub arguments: Vec<OsString>,
    /// Initial working directory. `None` inherits the application directory.
    pub working_directory: Option<PathBuf>,
    /// Environment variables added to or replacing the inherited environment.
    pub environment: Vec<(OsString, OsString)>,
    /// Initial terminal width in cells.
    pub cols: u16,
    /// Initial terminal height in cells.
    pub rows: u16,
    /// Maximum libghostty scrollback rows.
    pub max_scrollback: usize,
}

impl Default for TerminalOptions {
    fn default() -> Self {
        Self {
            program: None,
            arguments: Vec::new(),
            working_directory: None,
            environment: Vec::new(),
            cols: DEFAULT_COLS,
            rows: DEFAULT_ROWS,
            max_scrollback: 10_000,
        }
    }
}

impl TerminalOptions {
    fn validate(&self) -> Result<(), TerminalError> {
        if !(1..=MAX_COLS).contains(&self.cols) || !(1..=MAX_ROWS).contains(&self.rows) {
            return Err(TerminalError::InvalidSize);
        }
        if self.arguments.len() > MAX_TERMINAL_ARGUMENTS {
            return Err(TerminalError::TooManyArguments);
        }
        if self.environment.len() > MAX_TERMINAL_ENVIRONMENT {
            return Err(TerminalError::TooManyEnvironmentVariables);
        }
        if self.max_scrollback > MAX_TERMINAL_SCROLLBACK {
            return Err(TerminalError::ScrollbackTooLarge);
        }
        if self
            .program
            .as_deref()
            .is_some_and(|program| program.is_empty() || !valid_os_string(program))
            || self.arguments.iter().any(|value| !valid_os_string(value))
            || self
                .working_directory
                .as_deref()
                .is_some_and(|value| !valid_os_string(value.as_os_str()))
            || self.environment.iter().any(|(key, value)| {
                key.is_empty()
                    || key.to_string_lossy().contains('=')
                    || !valid_os_string(key)
                    || !valid_os_string(value)
            })
        {
            return Err(TerminalError::InvalidString);
        }
        Ok(())
    }
}

/// Complete application-supplied color theme for a terminal emulator.
///
/// The first eight ANSI entries are the normal colors in black-through-white order. The final
/// eight are their bright variants. Changing this theme updates libghostty's defaults without
/// restarting the PTY, while OSC color overrides emitted by the running program remain honored.
#[derive(Clone, Debug, PartialEq)]
pub struct TerminalTheme {
    pub foreground: Color,
    pub background: Color,
    pub cursor: Color,
    pub ansi: [Color; TERMINAL_ANSI_COLOR_COUNT],
}

impl TerminalTheme {
    pub fn new(
        foreground: Color,
        background: Color,
        ansi: [Color; TERMINAL_ANSI_COLOR_COUNT],
    ) -> Self {
        Self {
            foreground,
            background,
            cursor: foreground,
            ansi,
        }
    }

    pub fn cursor(mut self, cursor: Color) -> Self {
        self.cursor = cursor;
        self
    }
}

/// Terminal session creation errors detected before its worker starts.
#[derive(Debug, thiserror::Error)]
pub enum TerminalError {
    #[error("terminal dimensions must be between 1x1 and {MAX_COLS}x{MAX_ROWS}")]
    InvalidSize,
    #[error("terminal process declarations support at most {MAX_TERMINAL_ARGUMENTS} arguments")]
    TooManyArguments,
    #[error(
        "terminal process declarations support at most {MAX_TERMINAL_ENVIRONMENT} environment variables"
    )]
    TooManyEnvironmentVariables,
    #[error("terminal scrollback cannot exceed {MAX_TERMINAL_SCROLLBACK} rows")]
    ScrollbackTooLarge,
    #[error(
        "terminal program, argument, directory, and environment strings must be non-NUL and at most {MAX_TERMINAL_STRING_BYTES} bytes"
    )]
    InvalidString,
    #[error("could not start the terminal worker: {0}")]
    Worker(String),
}

/// Current lifecycle state of the process attached to a terminal.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TerminalStatus {
    Starting,
    Running {
        process_id: Option<u32>,
    },
    Exited {
        exit_code: u32,
        signal: Option<Arc<str>>,
    },
    Failed {
        message: Arc<str>,
    },
}

/// Screen-derived activity for an agent detected in the terminal's foreground process group.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TerminalAgentStatus {
    Idle,
    Working,
    Blocked,
}

impl TerminalAgentStatus {
    pub const fn kind(self) -> &'static str {
        match self {
            Self::Idle => "idle",
            Self::Working => "working",
            Self::Blocked => "blocked",
        }
    }
}

/// An agent process discovered from the real PTY foreground job.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TerminalAgent {
    pub kind: Arc<str>,
    pub status: TerminalAgentStatus,
    pub process_id: u32,
}

/// Scrollbar geometry reported by libghostty for the visible terminal viewport.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TerminalScrollState {
    /// Total rows in the active screen, including retained history.
    pub total_rows: u64,
    /// Visible viewport offset measured from the top of retained history.
    pub offset_rows: u64,
    /// Number of rows visible in the viewport.
    pub viewport_rows: u64,
}

impl TerminalScrollState {
    const fn initial(rows: u16) -> Self {
        Self {
            total_rows: rows as u64,
            offset_rows: 0,
            viewport_rows: rows as u64,
        }
    }

    const fn max_offset_rows(self) -> u64 {
        self.total_rows.saturating_sub(self.viewport_rows)
    }
}

/// Cursor shape selected by the running terminal application through libghostty.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TerminalCursorStyle {
    Bar,
    Block,
    Underline,
    BlockHollow,
}

/// Cursor metadata for the currently visible libghostty viewport.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TerminalCursor {
    pub column: u16,
    pub row: u16,
    pub style: TerminalCursorStyle,
    pub blinking: bool,
    pub color: Color,
}

impl TerminalStatus {
    pub const fn kind(&self) -> &'static str {
        match self {
            Self::Starting => "starting",
            Self::Running { .. } => "running",
            Self::Exited { .. } => "exited",
            Self::Failed { .. } => "failed",
        }
    }
}

/// Immutable render and process metadata produced by a terminal worker.
#[derive(Clone, Debug)]
pub struct TerminalSnapshot {
    pub revision: u64,
    pub cols: u16,
    pub rows: u16,
    pub content: Arc<str>,
    pub highlights: Arc<[(Range<usize>, HighlightStyle)]>,
    pub foreground: Color,
    pub background: Color,
    pub title: Arc<str>,
    pub working_directory: Arc<str>,
    pub scroll: TerminalScrollState,
    pub cursor: Option<TerminalCursor>,
    cursor_range: Option<Range<usize>>,
    graphics: Arc<[TerminalCellGraphic]>,
    edge_backgrounds: EdgeBackgrounds,
    pub status: TerminalStatus,
    pub agent: Option<TerminalAgent>,
}

impl TerminalSnapshot {
    fn starting(cols: u16, rows: u16) -> Self {
        Self {
            revision: 0,
            cols,
            rows,
            content: Arc::from(""),
            highlights: Arc::from([]),
            foreground: Color::rgb8(224, 224, 224),
            background: Color::rgb8(20, 20, 20),
            title: Arc::from(""),
            working_directory: Arc::from(""),
            scroll: TerminalScrollState::initial(rows),
            cursor: None,
            cursor_range: None,
            graphics: Arc::from([]),
            edge_backgrounds: EdgeBackgrounds::empty(cols, rows),
            status: TerminalStatus::Starting,
            agent: None,
        }
    }
}

/// A Unicode block element rendered from terminal-cell geometry instead of a font outline.
///
/// Terminal emulators synthesize these glyphs so adjoining blocks cover the grid exactly. Keeping
/// the original character in `TerminalSnapshot::content` preserves selection and clipboard text.
#[derive(Clone, Copy, Debug, PartialEq)]
struct TerminalCellGraphic {
    column: u16,
    row: u16,
    character: char,
    foreground: Option<Color>,
}

/// How a terminal paints the space between its grid and its element bounds.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum TerminalPaddingColor {
    /// Paint padding with the terminal's default background.
    #[default]
    Background,
    /// Extend the nearest grid-edge cell background through the padding.
    Extend,
}

/// Visual metrics for a [`Terminal`] element.
#[derive(Clone, Debug, PartialEq)]
pub struct TerminalStyle {
    pub font_family: FontFamily,
    pub font_size: f32,
    pub line_height: f32,
    /// Optically thicken glyph stems without selecting a heavier font face.
    pub font_thicken: bool,
    /// Logical cell width as a multiple of `font_size`.
    pub cell_width_ratio: f32,
    /// Space between the terminal surface and its PTY-backed content.
    pub padding_top: f32,
    pub padding_right: f32,
    pub padding_bottom: f32,
    pub padding_left: f32,
    /// Background treatment for the space around the terminal grid.
    pub padding_color: TerminalPaddingColor,
    /// Override the emulator's default foreground without changing explicit ANSI colors.
    pub foreground: Option<Color>,
    /// Override the emulator's default background without changing explicit ANSI colors.
    pub background: Option<Color>,
    /// Core-owned libghostty color theme, including all normal and bright ANSI colors.
    pub theme: Option<TerminalTheme>,
}

impl Default for TerminalStyle {
    fn default() -> Self {
        Self {
            font_family: FontFamily::Monospace,
            font_size: 13.0,
            line_height: 18.0,
            font_thicken: false,
            cell_width_ratio: DEFAULT_CELL_WIDTH_RATIO,
            padding_top: 0.0,
            padding_right: 0.0,
            padding_bottom: 0.0,
            padding_left: 0.0,
            padding_color: TerminalPaddingColor::Background,
            foreground: None,
            background: None,
            theme: None,
        }
    }
}

impl TerminalStyle {
    fn normalized(&self) -> Self {
        Self {
            font_family: self.font_family.clone(),
            font_size: finite_clamp(self.font_size, 1.0, 128.0, 13.0),
            line_height: finite_clamp(self.line_height, 1.0, 256.0, 18.0),
            font_thicken: self.font_thicken,
            cell_width_ratio: finite_clamp(
                self.cell_width_ratio,
                0.2,
                2.0,
                DEFAULT_CELL_WIDTH_RATIO,
            ),
            padding_top: finite_clamp(self.padding_top, 0.0, 256.0, 0.0),
            padding_right: finite_clamp(self.padding_right, 0.0, 256.0, 0.0),
            padding_bottom: finite_clamp(self.padding_bottom, 0.0, 256.0, 0.0),
            padding_left: finite_clamp(self.padding_left, 0.0, 256.0, 0.0),
            padding_color: self.padding_color,
            foreground: self.foreground,
            background: self.background,
            theme: self.theme.clone(),
        }
    }
}

/// A real pseudoterminal session whose VT state is owned by a dedicated worker thread.
#[derive(Clone)]
pub struct Terminal {
    inner: Arc<TerminalInner>,
}

struct TerminalInner {
    messages: SyncSender<WorkerMessage>,
    snapshot: Arc<RwLock<Arc<TerminalSnapshot>>>,
    last_size: AtomicU64,
    viewport_height_bits: AtomicU32,
    wheel_remainder: Mutex<f32>,
    scrollbar_drag_remainder: Mutex<f32>,
    scrollbar_interaction: Mutex<TerminalScrollbarInteraction>,
    cursor_blink: Mutex<TerminalCursorBlink>,
    theme: Mutex<Option<TerminalTheme>>,
    shutdown: Arc<AtomicBool>,
}

#[derive(Clone, Copy, Debug)]
struct TerminalCursorBlink {
    focused: bool,
    epoch: Instant,
}

impl TerminalCursorBlink {
    fn new(now: Instant) -> Self {
        Self {
            focused: false,
            epoch: now,
        }
    }

    fn reset(&mut self, now: Instant) {
        self.epoch = now;
    }

    fn update_focus(&mut self, focused: bool, now: Instant) -> Instant {
        if focused && !self.focused {
            self.reset(now);
        }
        self.focused = focused;
        self.epoch
    }
}

#[derive(Default)]
struct TerminalScrollbarInteraction {
    hovered: bool,
    dragging: bool,
    visible_until: Option<Instant>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct TerminalScrollbarPresentation {
    visible: bool,
    expanded: bool,
    hide_at: Option<Instant>,
}

impl TerminalScrollbarInteraction {
    fn reveal(&mut self, now: Instant) {
        self.visible_until = Some(now + TERMINAL_SCROLLBAR_HIDE_DELAY);
    }

    fn set_hovered(&mut self, hovered: bool, now: Instant) {
        self.hovered = hovered;
        self.reveal(now);
    }

    fn begin_drag(&mut self, now: Instant) {
        self.dragging = true;
        self.reveal(now);
    }

    fn end_drag(&mut self, now: Instant) {
        self.dragging = false;
        self.reveal(now);
    }

    fn presentation(&self, now: Instant) -> TerminalScrollbarPresentation {
        let expanded = self.hovered || self.dragging;
        let hide_at = self.visible_until.filter(|deadline| *deadline > now);
        TerminalScrollbarPresentation {
            visible: expanded || hide_at.is_some(),
            expanded,
            hide_at: if expanded { None } else { hide_at },
        }
    }
}

impl Drop for TerminalInner {
    fn drop(&mut self) {
        self.shutdown.store(true, Ordering::Release);
        let _ = self.messages.try_send(WorkerMessage::Shutdown);
    }
}

impl Terminal {
    /// Start a PTY and terminal-emulation worker. Process-launch failures are published through
    /// [`TerminalSnapshot::status`] so a declarative terminal remains mounted and informative.
    pub fn spawn(
        options: TerminalOptions,
        invalidator: WindowInvalidator,
    ) -> Result<Self, TerminalError> {
        options.validate()?;
        let initial_size = TerminalSize {
            cols: options.cols,
            rows: options.rows,
            cell_width_px: 0,
            cell_height_px: 0,
        };
        let snapshot = Arc::new(RwLock::new(Arc::new(TerminalSnapshot::starting(
            options.cols,
            options.rows,
        ))));
        let (messages, receiver) = sync_channel(MESSAGE_CAPACITY);
        let shutdown = Arc::new(AtomicBool::new(false));
        let worker_shutdown = Arc::clone(&shutdown);
        let worker_messages = messages.clone();
        let worker_snapshot = Arc::clone(&snapshot);
        thread::Builder::new()
            .name("quickgui-terminal".to_owned())
            .spawn(move || {
                run_terminal_worker(
                    options,
                    initial_size,
                    receiver,
                    worker_messages,
                    worker_snapshot,
                    invalidator,
                    worker_shutdown,
                );
            })
            .map_err(|error| TerminalError::Worker(error.to_string()))?;
        Ok(Self {
            inner: Arc::new(TerminalInner {
                messages,
                snapshot,
                last_size: AtomicU64::new(pack_size(initial_size)),
                viewport_height_bits: AtomicU32::new(0),
                wheel_remainder: Mutex::new(0.0),
                scrollbar_drag_remainder: Mutex::new(0.0),
                scrollbar_interaction: Mutex::new(TerminalScrollbarInteraction::default()),
                cursor_blink: Mutex::new(TerminalCursorBlink::new(Instant::now())),
                theme: Mutex::new(None),
                shutdown,
            }),
        })
    }

    /// Read the latest complete terminal frame without locking the emulator or PTY.
    pub fn snapshot(&self) -> Arc<TerminalSnapshot> {
        self.inner
            .snapshot
            .read()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clone()
    }

    /// Write raw bytes to the PTY. Prefer normal keyboard events or [`Self::paste`] for user input.
    pub fn write(&self, bytes: impl AsRef<[u8]>) -> bool {
        let bytes = bytes.as_ref();
        if bytes.is_empty() || bytes.len() > MAX_TERMINAL_INPUT_BYTES {
            return false;
        }
        let sent = self.try_send(WorkerMessage::Input(bytes.to_vec()));
        if sent {
            self.reset_cursor_blink();
        }
        sent
    }

    /// Encode text using libghostty's bracketed-paste and control-byte rules, then write it.
    pub fn paste(&self, value: impl Into<String>) -> bool {
        let value = value.into();
        if value.is_empty() || value.len() > MAX_TERMINAL_INPUT_BYTES {
            return false;
        }
        let sent = self.try_send(WorkerMessage::Paste(value));
        if sent {
            self.reset_cursor_blink();
        }
        sent
    }

    /// Scroll libghostty's retained viewport by terminal rows. Negative values move into history.
    pub fn scroll_rows(&self, rows: isize) -> bool {
        let sent = rows != 0
            && self.try_send(WorkerMessage::Scroll(
                rows.clamp(-(MAX_ROWS as isize), MAX_ROWS as isize),
            ));
        if sent {
            self.reveal_scrollbar();
        }
        sent
    }

    fn scroll_pixel_delta(&self, delta_y: f32, line_height: f32, phase: GesturePhase) -> bool {
        if matches!(phase, GesturePhase::Ended | GesturePhase::Cancelled) {
            *self
                .inner
                .wheel_remainder
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner()) = 0.0;
            return false;
        }
        if !delta_y.is_finite() || delta_y == 0.0 {
            return false;
        }
        self.reveal_scrollbar();
        let mut remainder = self
            .inner
            .wheel_remainder
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let rows = accumulate_terminal_rows(&mut remainder, -delta_y / line_height.max(1.0));
        drop(remainder);
        let _ = self.scroll_rows(rows);
        true
    }

    fn update_viewport_height(&self, height: f32) {
        let height = if height.is_finite() {
            height.max(0.0)
        } else {
            0.0
        };
        self.inner
            .viewport_height_bits
            .store(height.to_bits(), Ordering::Relaxed);
    }

    fn viewport_height(&self) -> f32 {
        f32::from_bits(self.inner.viewport_height_bits.load(Ordering::Relaxed))
    }

    fn begin_scrollbar_drag(&self) {
        *self
            .inner
            .scrollbar_drag_remainder
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner()) = 0.0;
        self.inner
            .scrollbar_interaction
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .begin_drag(Instant::now());
    }

    fn end_scrollbar_drag(&self) {
        *self
            .inner
            .scrollbar_drag_remainder
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner()) = 0.0;
        self.inner
            .scrollbar_interaction
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .end_drag(Instant::now());
    }

    fn set_scrollbar_hovered(&self, hovered: bool) {
        self.inner
            .scrollbar_interaction
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .set_hovered(hovered, Instant::now());
    }

    fn reveal_scrollbar(&self) {
        self.inner
            .scrollbar_interaction
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .reveal(Instant::now());
    }

    fn scrollbar_presentation(&self, now: Instant) -> TerminalScrollbarPresentation {
        self.inner
            .scrollbar_interaction
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .presentation(now)
    }

    fn reset_cursor_blink(&self) {
        self.inner
            .cursor_blink
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .reset(Instant::now());
    }

    fn cursor_blink_epoch_for_focus(&self, focused: bool, now: Instant) -> Instant {
        self.inner
            .cursor_blink
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .update_focus(focused, now)
    }

    fn drag_scrollbar(&self, delta_y: f32) -> bool {
        if !delta_y.is_finite() || delta_y == 0.0 {
            return false;
        }
        let snapshot = self.snapshot();
        let Some(geometry) = terminal_scrollbar_geometry(snapshot.scroll, self.viewport_height())
        else {
            return false;
        };
        if geometry.travel <= 0.0 || geometry.max_offset_rows == 0 {
            return false;
        }
        let mut remainder = self
            .inner
            .scrollbar_drag_remainder
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let rows = accumulate_terminal_rows(
            &mut remainder,
            delta_y * geometry.max_offset_rows as f32 / geometry.travel,
        );
        drop(remainder);
        let _ = self.scroll_rows(rows);
        true
    }

    /// Resize both the PTY and libghostty viewport, coalescing identical measurements.
    pub fn resize(&self, cols: u16, rows: u16, cell_width_px: u32, cell_height_px: u32) -> bool {
        let size = TerminalSize {
            cols: cols.clamp(1, MAX_COLS),
            rows: rows.clamp(1, MAX_ROWS),
            cell_width_px,
            cell_height_px,
        };
        let packed = pack_size(size);
        if self.inner.last_size.swap(packed, Ordering::Relaxed) == packed {
            return true;
        }
        self.try_send(WorkerMessage::Resize(size))
    }

    /// Build the complete retained terminal surface, including focus, keyboard, paste, scroll, and
    /// exact bounds-to-PTY resize behavior.
    pub fn element<V: 'static>(
        &self,
        id: impl Into<ElementId>,
        style: TerminalStyle,
        cx: &mut ViewContext<'_, V>,
    ) -> Element {
        let id = id.into();
        let style = style.normalized();
        self.sync_theme(style.theme.clone());
        let snapshot = self.snapshot();
        let scale_factor = cx.scale_factor();
        let cell_metrics = CellMetrics::new(
            style.font_size,
            style.line_height,
            style.cell_width_ratio,
            scale_factor,
        );
        let cell_width = cell_metrics.logical_width;
        let line_height = cell_metrics.logical_height;
        let focus = cx.focus_handle(id);
        let focused = cx.window_state().focused && cx.is_focused(focus);
        let now = Instant::now();

        let keyboard = self.clone();
        let key_down = cx.key_down_listener(id, move |_view, event, event_cx| {
            if handle_terminal_paste(&keyboard, event, event_cx) {
                return;
            }
            if event.modifiers.contains(Modifiers::SUPER) {
                return;
            }
            if keyboard.try_send(WorkerMessage::Key(TerminalKeyInput::down(event))) {
                keyboard.reset_cursor_blink();
                event_cx.clear_text_selection();
                event_cx.prevent_default();
                event_cx.stop_propagation();
                event_cx.invalidate();
            }
        });
        let keyboard = self.clone();
        let key_up = cx.key_up_listener(id, move |_view, event, event_cx| {
            if event.modifiers.contains(Modifiers::SUPER) {
                return;
            }
            if keyboard.try_send(WorkerMessage::Key(TerminalKeyInput::up(event))) {
                event_cx.prevent_default();
                event_cx.stop_propagation();
            }
        });
        let scroll_terminal = self.clone();
        let scroll = cx.scroll_wheel_listener(id, move |_view, event, event_cx| {
            let delta = event.delta.pixel_delta(line_height).y;
            if scroll_terminal.scroll_pixel_delta(delta, line_height, event.phase) {
                event_cx.prevent_default();
                event_cx.stop_propagation();
                event_cx.invalidate();
            }
        });

        let resize_terminal = self.clone();
        let resize_probe = canvas(move |_bounds, canvas| {
            let bounds = canvas.bounds();
            resize_terminal.update_viewport_height(bounds.height);
            let cols = (bounds.width / cell_width)
                .floor()
                .clamp(1.0, MAX_COLS as f32) as u16;
            let rows = (bounds.height / line_height)
                .floor()
                .clamp(1.0, MAX_ROWS as f32) as u16;
            let _ = resize_terminal.resize(
                cols,
                rows,
                cell_metrics.physical_width,
                cell_metrics.physical_height,
            );
        })
        .absolute()
        .inset_0()
        .size_full();

        let blink_epoch = self.cursor_blink_epoch_for_focus(focused, now);
        let cursor = snapshot
            .cursor
            .and_then(|cursor| terminal_cursor_presentation(cursor, focused, blink_epoch, now, cx));
        let cursor_color = if style.theme.is_some() {
            snapshot
                .cursor
                .map_or(snapshot.foreground, |cursor| cursor.color)
        } else {
            style.foreground.unwrap_or_else(|| {
                snapshot
                    .cursor
                    .map_or(snapshot.foreground, |cursor| cursor.color)
            })
        };
        let terminal_background = style.background.unwrap_or(snapshot.background);
        let padding_extension = (style.padding_color == TerminalPaddingColor::Extend).then(|| {
            let edges = snapshot.edge_backgrounds.clone();
            let padding_top = style.padding_top;
            let padding_left = style.padding_left;
            canvas(move |_bounds, canvas| {
                paint_padding_extension(canvas, &edges, cell_metrics, padding_top, padding_left);
            })
            .absolute()
            .inset_0()
            .size_full()
        });
        let block_cursor_graphic = cursor == Some(TerminalCursorStyle::Block)
            && snapshot.cursor.is_some_and(|cursor| {
                snapshot
                    .graphics
                    .iter()
                    .any(|graphic| graphic.column == cursor.column && graphic.row == cursor.row)
            });
        let highlights = if cursor == Some(TerminalCursorStyle::Block)
            && let Some(range) = snapshot.cursor_range.clone()
        {
            terminal_highlights_with_cursor(
                &snapshot.highlights,
                range,
                HighlightStyle::default()
                    .color(if block_cursor_graphic {
                        Color::TRANSPARENT
                    } else {
                        terminal_background
                    })
                    .background(cursor_color),
            )
        } else {
            snapshot.highlights.to_vec()
        };
        let styled = StyledText::new(snapshot.content.clone()).with_highlights(highlights);
        let terminal_text = styled
            .into_element()
            .id(derived_terminal_id(id, TERMINAL_TEXT_ID_TAG))
            .font_family(style.font_family)
            .text_size(style.font_size)
            .font_thicken(style.font_thicken)
            .line_height(line_height)
            .monospace_width(cell_width)
            .text_color(style.foreground.unwrap_or(snapshot.foreground))
            .whitespace_nowrap()
            .text_shaping_basic()
            .selectable();

        let terminal_graphics = (!snapshot.graphics.is_empty()).then(|| {
            let graphics = snapshot.graphics.clone();
            let default_foreground = style.foreground.unwrap_or(snapshot.foreground);
            let block_cursor = (cursor == Some(TerminalCursorStyle::Block))
                .then_some(snapshot.cursor)
                .flatten();
            canvas(move |_bounds, canvas| {
                for graphic in graphics.iter() {
                    let color = if block_cursor.is_some_and(|cursor| {
                        cursor.column == graphic.column && cursor.row == graphic.row
                    }) {
                        terminal_background
                    } else {
                        graphic.foreground.unwrap_or(default_foreground)
                    };
                    paint_block(
                        canvas,
                        graphic.column,
                        graphic.row,
                        graphic.character,
                        cell_metrics,
                        color,
                    );
                }
            })
            .absolute()
            .inset_0()
            .size_full()
        });

        let cursor_element = cursor.zip(snapshot.cursor).map(|(cursor_style, cursor)| {
            terminal_cursor_element(cursor, cursor_style, cell_width, line_height, cursor_color)
        });
        let mut terminal_content = div()
            .absolute()
            .top(style.padding_top)
            .right(style.padding_right)
            .bottom(style.padding_bottom)
            .left(style.padding_left)
            .min_w(0.0)
            .min_h(0.0)
            .overflow_hidden()
            .child(resize_probe);
        if cursor == Some(TerminalCursorStyle::Block)
            && let Some(cursor_element) = cursor_element.clone()
        {
            terminal_content = terminal_content.child(cursor_element);
        }
        terminal_content = terminal_content.child(terminal_text);
        if let Some(terminal_graphics) = terminal_graphics {
            terminal_content = terminal_content.child(terminal_graphics);
        }
        if cursor != Some(TerminalCursorStyle::Block)
            && let Some(cursor_element) = cursor_element
        {
            terminal_content = terminal_content.child(cursor_element);
        }

        let mut terminal = div()
            .id(id)
            .relative()
            .min_w(0.0)
            .min_h(0.0)
            .overflow_hidden()
            .bg(terminal_background)
            .track_focus(focus)
            .accessibility_role(AccessibilityRole::Group)
            .accessibility_label("Terminal")
            .cursor_text();
        if let Some(padding_extension) = padding_extension {
            terminal = terminal.child(padding_extension);
        }
        terminal = terminal
            .child(terminal_content)
            .on_key_down(key_down)
            .on_key_up(key_up)
            .on_scroll_wheel(scroll);

        if let Some(geometry) = terminal_scrollbar_geometry(snapshot.scroll, self.viewport_height())
        {
            let scrollbar_id = derived_terminal_id(id, TERMINAL_SCROLLBAR_ID_TAG);
            let scrollbar = self.scrollbar_presentation(now);
            if let Some(deadline) = scrollbar.hide_at {
                cx.request_repaint_at(deadline);
            }
            let hover_terminal = self.clone();
            let hover = cx.hover_listener(scrollbar_id, move |_view, hovered, event_cx| {
                hover_terminal.set_scrollbar_hovered(*hovered);
                event_cx.invalidate();
            });
            let scrollbar_terminal = self.clone();
            let pointer = cx.pointer_listener(scrollbar_id, move |_view, event, event_cx| {
                if event.button != MouseButton::Left {
                    return;
                }
                match event.phase {
                    PointerPhase::Down => scrollbar_terminal.begin_scrollbar_drag(),
                    PointerPhase::Move => {
                        let _ = scrollbar_terminal.drag_scrollbar(event.delta.y);
                    }
                    PointerPhase::Up | PointerPhase::Cancel => {
                        scrollbar_terminal.end_scrollbar_drag();
                    }
                }
                event_cx.clear_text_selection();
                event_cx.prevent_default();
                event_cx.stop_propagation();
                event_cx.invalidate();
            });
            let thumb_width = if scrollbar.expanded {
                TERMINAL_SCROLLBAR_HOVER_WIDTH
            } else {
                TERMINAL_SCROLLBAR_IDLE_WIDTH
            };
            let mut track = div()
                .id(scrollbar_id)
                .absolute()
                .top(style.padding_top)
                .right(0.0)
                .bottom(style.padding_bottom)
                .w(TERMINAL_SCROLLBAR_HIT_WIDTH)
                .cursor_default()
                .on_hover(hover)
                .on_pointer(pointer);
            if scrollbar.visible {
                track = track.child(
                    div()
                        .absolute()
                        .top(geometry.thumb_top + TERMINAL_SCROLLBAR_VERTICAL_INSET)
                        .right(TERMINAL_SCROLLBAR_EDGE_INSET)
                        .w(thumb_width)
                        .h(
                            (geometry.thumb_height - TERMINAL_SCROLLBAR_VERTICAL_INSET * 2.0)
                                .max(thumb_width),
                        )
                        .rounded(thumb_width * 0.5)
                        .bg(if scrollbar.expanded {
                            Color::rgba8(142, 147, 160, 210)
                        } else {
                            Color::rgba8(142, 147, 160, 160)
                        }),
                );
            }
            terminal = terminal.child(track);
        }

        terminal
    }

    fn try_send(&self, message: WorkerMessage) -> bool {
        match self.inner.messages.try_send(message) {
            Ok(()) => true,
            Err(TrySendError::Full(_)) | Err(TrySendError::Disconnected(_)) => false,
        }
    }

    fn sync_theme(&self, theme: Option<TerminalTheme>) {
        let mut current = self
            .inner
            .theme
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if *current == theme {
            return;
        }
        if self.try_send(WorkerMessage::Theme(theme.clone().map(Box::new))) {
            *current = theme;
        }
    }
}

fn terminal_cursor_presentation<V: 'static>(
    cursor: TerminalCursor,
    focused: bool,
    blink_epoch: Instant,
    now: Instant,
    cx: &mut ViewContext<'_, V>,
) -> Option<TerminalCursorStyle> {
    if !focused {
        return Some(TerminalCursorStyle::BlockHollow);
    }
    let elapsed = now.saturating_duration_since(blink_epoch);
    let half_periods = elapsed.as_millis() / TERMINAL_CURSOR_BLINK_HALF_PERIOD.as_millis();
    let phase_elapsed = elapsed.as_millis() % TERMINAL_CURSOR_BLINK_HALF_PERIOD.as_millis();
    let until_next = TERMINAL_CURSOR_BLINK_HALF_PERIOD
        - Duration::from_millis(phase_elapsed.try_into().unwrap_or(u64::MAX));
    cx.request_repaint_at(now + until_next);
    half_periods.is_multiple_of(2).then_some(cursor.style)
}

fn terminal_cursor_element(
    cursor: TerminalCursor,
    style: TerminalCursorStyle,
    cell_width: f32,
    line_height: f32,
    color: Color,
) -> Element {
    let left = f32::from(cursor.column) * cell_width;
    let top = f32::from(cursor.row) * line_height;
    match style {
        TerminalCursorStyle::Bar => div()
            .absolute()
            .left(left)
            .top(top)
            .w((cell_width * 0.18).clamp(1.5, 2.5))
            .h(line_height)
            .bg(color),
        TerminalCursorStyle::Block => div()
            .absolute()
            .left(left)
            .top(top)
            .w(cell_width)
            .h(line_height)
            .bg(color),
        TerminalCursorStyle::Underline => div()
            .absolute()
            .left(left)
            .top(top + (line_height - 2.0).max(0.0))
            .w(cell_width)
            .h(2.0_f32.min(line_height))
            .bg(color),
        TerminalCursorStyle::BlockHollow => div()
            .absolute()
            .left(left)
            .top(top)
            .w(cell_width)
            .h(line_height)
            .border(1.0, color),
    }
}

fn terminal_highlights_with_cursor(
    highlights: &[(Range<usize>, HighlightStyle)],
    cursor: Range<usize>,
    cursor_style: HighlightStyle,
) -> Vec<(Range<usize>, HighlightStyle)> {
    let mut combined = Vec::with_capacity((highlights.len() + 2).min(MAX_TEXT_HIGHLIGHTS + 2));
    let mut inserted = false;
    for (range, style) in highlights {
        if range.end <= cursor.start {
            combined.push((range.clone(), style.clone(), false));
            continue;
        }
        if range.start >= cursor.end {
            if !inserted {
                combined.push((cursor.clone(), cursor_style.clone(), true));
                inserted = true;
            }
            combined.push((range.clone(), style.clone(), false));
            continue;
        }
        if range.start < cursor.start {
            combined.push((range.start..cursor.start, style.clone(), false));
        }
        if !inserted {
            combined.push((cursor.clone(), cursor_style.clone(), true));
            inserted = true;
        }
        if range.end > cursor.end {
            combined.push((cursor.end..range.end, style.clone(), false));
        }
    }
    if !inserted {
        combined.push((cursor, cursor_style, true));
    }
    combined.sort_unstable_by_key(|(range, _, _)| range.start);
    while combined.len() > MAX_TEXT_HIGHLIGHTS {
        let Some(index) = combined.iter().rposition(|(_, _, cursor)| !cursor) else {
            break;
        };
        combined.remove(index);
    }
    combined
        .into_iter()
        .map(|(range, style, _)| (range, style))
        .collect()
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct TerminalScrollbarGeometry {
    thumb_top: f32,
    thumb_height: f32,
    travel: f32,
    max_offset_rows: u64,
}

fn terminal_scrollbar_geometry(
    scroll: TerminalScrollState,
    viewport_height: f32,
) -> Option<TerminalScrollbarGeometry> {
    let max_offset_rows = scroll.max_offset_rows();
    if max_offset_rows == 0
        || scroll.total_rows == 0
        || !viewport_height.is_finite()
        || viewport_height <= 0.0
    {
        return None;
    }
    let thumb_height = (viewport_height * scroll.viewport_rows as f32 / scroll.total_rows as f32)
        .max(TERMINAL_SCROLLBAR_MIN_THUMB)
        .min(viewport_height);
    let travel = viewport_height - thumb_height;
    let thumb_top =
        travel * (scroll.offset_rows.min(max_offset_rows) as f32 / max_offset_rows as f32);
    Some(TerminalScrollbarGeometry {
        thumb_top,
        thumb_height,
        travel,
        max_offset_rows,
    })
}

fn accumulate_terminal_rows(remainder: &mut f32, rows: f32) -> isize {
    if !rows.is_finite() || rows == 0.0 {
        return 0;
    }
    *remainder = (*remainder + rows).clamp(-(MAX_ROWS as f32), MAX_ROWS as f32);
    let complete = remainder.trunc();
    *remainder -= complete;
    complete as isize
}

fn derived_terminal_id(parent: ElementId, tag: u64) -> ElementId {
    let mut hash = parent.as_u64() ^ tag;
    hash ^= hash >> 30;
    hash = hash.wrapping_mul(0xbf58_476d_1ce4_e5b9);
    hash ^= hash >> 27;
    hash = hash.wrapping_mul(0x94d0_49bb_1331_11eb);
    hash ^= hash >> 31;
    if hash == 0 || hash == parent.as_u64() || hash == u64::MAX {
        hash ^= tag.rotate_left(17);
    }
    ElementId::new(hash)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct TerminalSize {
    cols: u16,
    rows: u16,
    cell_width_px: u32,
    cell_height_px: u32,
}

enum WorkerMessage {
    Output(Vec<u8>),
    ReaderClosed,
    Input(Vec<u8>),
    Paste(String),
    Key(TerminalKeyInput),
    Resize(TerminalSize),
    Scroll(isize),
    Theme(Option<Box<TerminalTheme>>),
    Shutdown,
}

#[derive(Clone)]
struct TerminalKeyInput {
    key: Key,
    key_char: Option<Key>,
    text: Option<String>,
    modifiers: Modifiers,
    action: GhosttyKeyAction,
}

impl TerminalKeyInput {
    fn down(event: &KeyDownEvent) -> Self {
        Self {
            key: event.key.clone(),
            key_char: event.key_char.clone(),
            text: event.text.clone(),
            modifiers: event.modifiers,
            action: if event.repeat {
                GhosttyKeyAction::Repeat
            } else {
                GhosttyKeyAction::Press
            },
        }
    }

    fn up(event: &KeyUpEvent) -> Self {
        Self {
            key: event.key.clone(),
            key_char: event.key_char.clone(),
            text: None,
            modifiers: event.modifiers,
            action: GhosttyKeyAction::Release,
        }
    }
}

#[allow(clippy::too_many_lines)]
fn run_terminal_worker(
    options: TerminalOptions,
    mut size: TerminalSize,
    receiver: Receiver<WorkerMessage>,
    reader_messages: SyncSender<WorkerMessage>,
    shared: Arc<RwLock<Arc<TerminalSnapshot>>>,
    invalidator: WindowInvalidator,
    shutdown: Arc<AtomicBool>,
) {
    let pty_responses = Rc::new(RefCell::new(Vec::<u8>::new()));
    let mut terminal = match GhosttyTerminal::new(GhosttyTerminalOptions {
        cols: size.cols,
        rows: size.rows,
        max_scrollback: options.max_scrollback,
    }) {
        Ok(terminal) => terminal,
        Err(error) => {
            publish_failure(
                &shared,
                &invalidator,
                size,
                format!("libghostty-vt: {error}"),
            );
            return;
        }
    };
    let callback_responses = Rc::clone(&pty_responses);
    if let Err(error) = terminal.on_pty_write(move |_terminal, bytes| {
        callback_responses.borrow_mut().extend_from_slice(bytes);
    }) {
        publish_failure(
            &shared,
            &invalidator,
            size,
            format!("libghostty-vt: {error}"),
        );
        return;
    }
    let mut render_state = match RenderState::new() {
        Ok(state) => state,
        Err(error) => {
            publish_failure(
                &shared,
                &invalidator,
                size,
                format!("libghostty-vt: {error}"),
            );
            return;
        }
    };
    let mut rows = match RowIterator::new() {
        Ok(rows) => rows,
        Err(error) => {
            publish_failure(
                &shared,
                &invalidator,
                size,
                format!("libghostty-vt: {error}"),
            );
            return;
        }
    };
    let mut cells = match CellIterator::new() {
        Ok(cells) => cells,
        Err(error) => {
            publish_failure(
                &shared,
                &invalidator,
                size,
                format!("libghostty-vt: {error}"),
            );
            return;
        }
    };
    let mut key_encoder = match GhosttyKeyEncoder::new() {
        Ok(encoder) => encoder,
        Err(error) => {
            publish_failure(
                &shared,
                &invalidator,
                size,
                format!("libghostty-vt: {error}"),
            );
            return;
        }
    };

    let pty_system = native_pty_system();
    let pair = match pty_system.openpty(pty_size(size)) {
        Ok(pair) => pair,
        Err(error) => {
            publish_failure(
                &shared,
                &invalidator,
                size,
                format!("could not open PTY: {error}"),
            );
            return;
        }
    };
    let mut command = terminal_command(&options);
    command.env("TERM", "xterm-256color");
    command.env("COLORTERM", "truecolor");
    command.env("TERM_PROGRAM", TERM_PROGRAM_VALUE);
    command.env("TERM_PROGRAM_VERSION", env!("CARGO_PKG_VERSION"));
    let mut child = match pair.slave.spawn_command(command) {
        Ok(child) => child,
        Err(error) => {
            publish_failure(
                &shared,
                &invalidator,
                size,
                format!("could not start terminal process: {error}"),
            );
            return;
        }
    };
    drop(pair.slave);
    let mut reader = match pair.master.try_clone_reader() {
        Ok(reader) => reader,
        Err(error) => {
            let _ = child.kill();
            publish_failure(
                &shared,
                &invalidator,
                size,
                format!("could not read PTY: {error}"),
            );
            return;
        }
    };
    let mut writer = match pair.master.take_writer() {
        Ok(writer) => writer,
        Err(error) => {
            let _ = child.kill();
            publish_failure(
                &shared,
                &invalidator,
                size,
                format!("could not write PTY: {error}"),
            );
            return;
        }
    };
    let master = pair.master;
    thread::Builder::new()
        .name("quickgui-terminal-reader".to_owned())
        .spawn(move || {
            let mut buffer = vec![0; READ_CHUNK_BYTES];
            loop {
                match reader.read(&mut buffer) {
                    Ok(0) | Err(_) => break,
                    Ok(length) => {
                        if reader_messages
                            .send(WorkerMessage::Output(buffer[..length].to_vec()))
                            .is_err()
                        {
                            return;
                        }
                    }
                }
            }
            let _ = reader_messages.send(WorkerMessage::ReaderClosed);
        })
        .ok();

    let child_process_id = child.process_id();
    let mut revision = 0;
    let mut status = TerminalStatus::Running {
        process_id: child_process_id,
    };
    let mut detected_agent =
        child_process_id.and_then(crate::terminal_process::detect_agent_process);
    let mut last_process_probe = Instant::now();
    let mut last_output = Instant::now();
    let mut recently_active = detected_agent.is_some();
    publish_terminal_snapshot(
        &mut terminal,
        &mut render_state,
        &mut rows,
        &mut cells,
        &shared,
        &invalidator,
        TerminalSnapshotMetadata::next(
            &mut revision,
            status.clone(),
            detected_agent,
            recently_active,
        ),
    );

    let mut reader_closed = false;
    'worker: loop {
        if shutdown.load(Ordering::Acquire) {
            break;
        }
        let first = match receiver.recv_timeout(Duration::from_millis(100)) {
            Ok(message) => Some(message),
            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => None,
            Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => break,
        };
        let mut redraw = false;
        let mut output_received = false;
        if let Some(message) = first {
            if !handle_worker_message(
                message,
                &mut terminal,
                &mut key_encoder,
                &pty_responses,
                &mut writer,
                master.as_ref(),
                &mut size,
                &mut child,
                &mut reader_closed,
                &mut redraw,
                &mut output_received,
            ) {
                break 'worker;
            }
            for _ in 1..WORKER_BATCH_LIMIT {
                match receiver.try_recv() {
                    Ok(message) => {
                        if !handle_worker_message(
                            message,
                            &mut terminal,
                            &mut key_encoder,
                            &pty_responses,
                            &mut writer,
                            master.as_ref(),
                            &mut size,
                            &mut child,
                            &mut reader_closed,
                            &mut redraw,
                            &mut output_received,
                        ) {
                            break 'worker;
                        }
                    }
                    Err(TryRecvError::Empty) => break,
                    Err(TryRecvError::Disconnected) => break 'worker,
                }
            }
        }

        let now = Instant::now();
        if output_received {
            last_output = now;
        }
        if now.duration_since(last_process_probe) >= AGENT_PROCESS_PROBE_INTERVAL {
            last_process_probe = now;
            let next = child_process_id.and_then(crate::terminal_process::detect_agent_process);
            if next != detected_agent {
                detected_agent = next;
                redraw = true;
            }
        }
        let next_recently_active =
            detected_agent.is_some() && now.duration_since(last_output) < AGENT_RECENT_ACTIVITY;
        if next_recently_active != recently_active {
            recently_active = next_recently_active;
            redraw = true;
        }
        if let Ok(Some(exit)) = child.try_wait() {
            status = TerminalStatus::Exited {
                exit_code: exit.exit_code(),
                signal: exit.signal().map(Arc::from),
            };
            redraw = true;
        }
        if redraw {
            publish_terminal_snapshot(
                &mut terminal,
                &mut render_state,
                &mut rows,
                &mut cells,
                &shared,
                &invalidator,
                TerminalSnapshotMetadata::next(
                    &mut revision,
                    status.clone(),
                    detected_agent,
                    recently_active,
                ),
            );
        }
        if reader_closed && matches!(status, TerminalStatus::Exited { .. }) {
            break;
        }
    }
    if !matches!(status, TerminalStatus::Exited { .. }) {
        let _ = child.kill();
    }
}

#[allow(clippy::too_many_arguments)]
fn handle_worker_message(
    message: WorkerMessage,
    terminal: &mut GhosttyTerminal<'_, '_>,
    key_encoder: &mut GhosttyKeyEncoder<'_>,
    pty_responses: &Rc<RefCell<Vec<u8>>>,
    writer: &mut Box<dyn Write + Send>,
    master: &dyn portable_pty::MasterPty,
    size: &mut TerminalSize,
    child: &mut Box<dyn portable_pty::Child + Send + Sync>,
    reader_closed: &mut bool,
    redraw: &mut bool,
    output_received: &mut bool,
) -> bool {
    match message {
        WorkerMessage::Output(bytes) => {
            terminal.vt_write(&bytes);
            let responses = std::mem::take(&mut *pty_responses.borrow_mut());
            if !responses.is_empty() {
                let _ = writer.write_all(&responses);
                let _ = writer.flush();
            }
            *redraw = true;
            *output_received = true;
        }
        WorkerMessage::ReaderClosed => *reader_closed = true,
        WorkerMessage::Input(bytes) => {
            let _ = writer.write_all(&bytes);
            let _ = writer.flush();
        }
        WorkerMessage::Paste(value) => {
            if write_paste(terminal, writer, value) {
                terminal.scroll_viewport(ScrollViewport::Bottom);
                *redraw = true;
            }
        }
        WorkerMessage::Key(input) => {
            if write_key(terminal, key_encoder, writer, input) {
                terminal.scroll_viewport(ScrollViewport::Bottom);
                *redraw = true;
            }
        }
        WorkerMessage::Resize(next) => {
            if *size != next {
                *size = next;
                let _ = master.resize(pty_size(next));
                let _ = terminal.resize(
                    next.cols,
                    next.rows,
                    next.cell_width_px,
                    next.cell_height_px,
                );
                *redraw = true;
            }
        }
        WorkerMessage::Scroll(rows) => {
            terminal.scroll_viewport(ScrollViewport::Delta(rows));
            *redraw = true;
        }
        WorkerMessage::Theme(theme) => {
            if apply_terminal_theme(terminal, theme.as_deref()).is_ok() {
                *redraw = true;
            }
        }
        WorkerMessage::Shutdown => {
            let _ = child.kill();
            return false;
        }
    }
    true
}

fn apply_terminal_theme(
    terminal: &mut GhosttyTerminal<'_, '_>,
    theme: Option<&TerminalTheme>,
) -> Result<(), libghostty_vt::Error> {
    terminal
        .set_default_fg_color(theme.map(|theme| terminal_rgb(theme.foreground)))?
        .set_default_bg_color(theme.map(|theme| terminal_rgb(theme.background)))?
        .set_default_cursor_color(theme.map(|theme| terminal_rgb(theme.cursor)))?;
    if let Some(theme) = theme {
        let mut palette = terminal.default_color_palette()?;
        for (target, source) in palette.0.iter_mut().zip(theme.ansi) {
            *target = terminal_rgb(source);
        }
        terminal.set_default_color_palette(Some(palette))?;
    } else {
        terminal.set_default_color_palette(None)?;
    }
    Ok(())
}

fn terminal_rgb(color: Color) -> RgbColor {
    let [r, g, b, _] = color.to_srgba8();
    RgbColor { r, g, b }
}

fn terminal_command(options: &TerminalOptions) -> CommandBuilder {
    let mut command = options
        .program
        .as_ref()
        .map_or_else(CommandBuilder::new_default_prog, |program| {
            CommandBuilder::new(program)
        });
    command.args(&options.arguments);
    if let Some(directory) = &options.working_directory {
        command.cwd(directory);
    }
    for (key, value) in &options.environment {
        command.env(key, value);
    }
    command
}

fn pty_size(size: TerminalSize) -> PtySize {
    PtySize {
        rows: size.rows,
        cols: size.cols,
        pixel_width: size.cell_width_px.min(u16::MAX as u32) as u16,
        pixel_height: size.cell_height_px.min(u16::MAX as u32) as u16,
    }
}

fn write_paste(
    terminal: &GhosttyTerminal<'_, '_>,
    writer: &mut Box<dyn Write + Send>,
    value: String,
) -> bool {
    let bracketed = terminal.mode(Mode::BRACKETED_PASTE).unwrap_or(false);
    let mut input = value.into_bytes();
    let mut encoded = vec![0; input.len().saturating_add(16)];
    let length = match libghostty_vt::paste::encode(&mut input, bracketed, &mut encoded) {
        Ok(length) => length,
        Err(libghostty_vt::Error::OutOfSpace { required })
            if required <= MAX_TERMINAL_INPUT_BYTES + 16 =>
        {
            encoded.resize(required, 0);
            match libghostty_vt::paste::encode(&mut input, bracketed, &mut encoded) {
                Ok(length) => length,
                Err(_) => return false,
            }
        }
        Err(_) => return false,
    };
    let _ = writer.write_all(&encoded[..length]);
    let _ = writer.flush();
    true
}

fn write_key(
    terminal: &GhosttyTerminal<'_, '_>,
    encoder: &mut GhosttyKeyEncoder<'_>,
    writer: &mut Box<dyn Write + Send>,
    input: TerminalKeyInput,
) -> bool {
    let Some((key, text, unshifted)) =
        ghostty_key(&input.key, input.key_char.as_ref(), input.text.as_deref())
    else {
        return false;
    };
    let mut event = match GhosttyKeyEvent::new() {
        Ok(event) => event,
        Err(_) => return false,
    };
    event
        .set_action(input.action)
        .set_key(key)
        .set_mods(ghostty_modifiers(input.modifiers))
        .set_consumed_mods(GhosttyMods::empty())
        .set_composing(false)
        .set_utf8(text);
    if let Some(unshifted) = unshifted {
        event.set_unshifted_codepoint(unshifted);
    }
    encoder
        .set_options_from_terminal(terminal)
        .set_macos_option_as_alt(OptionAsAlt::True);
    let mut bytes = Vec::with_capacity(16);
    if encoder.encode_to_vec(&event, &mut bytes).is_ok() && !bytes.is_empty() {
        let _ = writer.write_all(&bytes);
        let _ = writer.flush();
        true
    } else {
        false
    }
}

fn ghostty_modifiers(modifiers: Modifiers) -> GhosttyMods {
    let mut answer = GhosttyMods::empty();
    if modifiers.contains(Modifiers::SHIFT) {
        answer |= GhosttyMods::SHIFT;
    }
    if modifiers.contains(Modifiers::CONTROL) {
        answer |= GhosttyMods::CTRL;
    }
    if modifiers.contains(Modifiers::ALT) {
        answer |= GhosttyMods::ALT;
    }
    if modifiers.contains(Modifiers::SUPER) {
        answer |= GhosttyMods::SUPER;
    }
    answer
}

fn ghostty_key(
    key: &Key,
    key_char: Option<&Key>,
    composed_text: Option<&str>,
) -> Option<(GhosttyKey, Option<String>, Option<char>)> {
    let physical_character = key_char
        .and_then(key_character)
        .or_else(|| key_character(key));
    let composed_text = composed_text
        .filter(|value| !value.is_empty())
        .filter(|value| value.chars().all(|character| !character.is_control()));
    let character = composed_text.or(physical_character);
    let answer = match key {
        Key::Character(_) => character
            .and_then(|value| value.chars().next())
            .map(ghostty_character_key)
            .unwrap_or(GhosttyKey::Unidentified),
        Key::ArrowUp => GhosttyKey::ArrowUp,
        Key::ArrowDown => GhosttyKey::ArrowDown,
        Key::ArrowLeft => GhosttyKey::ArrowLeft,
        Key::ArrowRight => GhosttyKey::ArrowRight,
        Key::PageUp => GhosttyKey::PageUp,
        Key::PageDown => GhosttyKey::PageDown,
        Key::Home => GhosttyKey::Home,
        Key::End => GhosttyKey::End,
        Key::Enter => GhosttyKey::Enter,
        Key::Escape => GhosttyKey::Escape,
        Key::Space => GhosttyKey::Space,
        Key::Tab => GhosttyKey::Tab,
        Key::Backspace => GhosttyKey::Backspace,
        Key::Delete => GhosttyKey::Delete,
        Key::Insert => GhosttyKey::Insert,
        Key::Function(value) => ghostty_function_key(*value),
        Key::Other => GhosttyKey::Unidentified,
    };
    let text = match key {
        Key::Enter
        | Key::Escape
        | Key::Tab
        | Key::Backspace
        | Key::Delete
        | Key::Insert
        | Key::ArrowUp
        | Key::ArrowDown
        | Key::ArrowLeft
        | Key::ArrowRight
        | Key::PageUp
        | Key::PageDown
        | Key::Home
        | Key::End
        | Key::Function(_)
        | Key::Other => None,
        Key::Space => Some(" ".to_owned()),
        Key::Character(_) => character.map(str::to_owned),
    };
    let unshifted = physical_character
        .and_then(|value| value.chars().next())
        .map(unshift_character);
    Some((answer, text, unshifted))
}

fn key_character(key: &Key) -> Option<&str> {
    match key {
        Key::Character(value) => Some(value),
        _ => None,
    }
}

fn ghostty_character_key(value: char) -> GhosttyKey {
    match value.to_ascii_lowercase() {
        'a' => GhosttyKey::A,
        'b' => GhosttyKey::B,
        'c' => GhosttyKey::C,
        'd' => GhosttyKey::D,
        'e' => GhosttyKey::E,
        'f' => GhosttyKey::F,
        'g' => GhosttyKey::G,
        'h' => GhosttyKey::H,
        'i' => GhosttyKey::I,
        'j' => GhosttyKey::J,
        'k' => GhosttyKey::K,
        'l' => GhosttyKey::L,
        'm' => GhosttyKey::M,
        'n' => GhosttyKey::N,
        'o' => GhosttyKey::O,
        'p' => GhosttyKey::P,
        'q' => GhosttyKey::Q,
        'r' => GhosttyKey::R,
        's' => GhosttyKey::S,
        't' => GhosttyKey::T,
        'u' => GhosttyKey::U,
        'v' => GhosttyKey::V,
        'w' => GhosttyKey::W,
        'x' => GhosttyKey::X,
        'y' => GhosttyKey::Y,
        'z' => GhosttyKey::Z,
        '0' | ')' => GhosttyKey::Digit0,
        '1' | '!' => GhosttyKey::Digit1,
        '2' | '@' => GhosttyKey::Digit2,
        '3' | '#' => GhosttyKey::Digit3,
        '4' | '$' => GhosttyKey::Digit4,
        '5' | '%' => GhosttyKey::Digit5,
        '6' | '^' => GhosttyKey::Digit6,
        '7' | '&' => GhosttyKey::Digit7,
        '8' | '*' => GhosttyKey::Digit8,
        '9' | '(' => GhosttyKey::Digit9,
        '`' | '~' => GhosttyKey::Backquote,
        '\\' | '|' => GhosttyKey::Backslash,
        '[' | '{' => GhosttyKey::BracketLeft,
        ']' | '}' => GhosttyKey::BracketRight,
        ',' | '<' => GhosttyKey::Comma,
        '=' | '+' => GhosttyKey::Equal,
        '-' | '_' => GhosttyKey::Minus,
        '.' | '>' => GhosttyKey::Period,
        '\'' | '"' => GhosttyKey::Quote,
        ';' | ':' => GhosttyKey::Semicolon,
        '/' | '?' => GhosttyKey::Slash,
        _ => GhosttyKey::Unidentified,
    }
}

fn unshift_character(value: char) -> char {
    match value {
        ')' => '0',
        '!' => '1',
        '@' => '2',
        '#' => '3',
        '$' => '4',
        '%' => '5',
        '^' => '6',
        '&' => '7',
        '*' => '8',
        '(' => '9',
        '~' => '`',
        '|' => '\\',
        '{' => '[',
        '}' => ']',
        '<' => ',',
        '+' => '=',
        '_' => '-',
        '>' => '.',
        '"' => '\'',
        ':' => ';',
        '?' => '/',
        value => value.to_ascii_lowercase(),
    }
}

fn ghostty_function_key(value: u8) -> GhosttyKey {
    match value {
        1 => GhosttyKey::F1,
        2 => GhosttyKey::F2,
        3 => GhosttyKey::F3,
        4 => GhosttyKey::F4,
        5 => GhosttyKey::F5,
        6 => GhosttyKey::F6,
        7 => GhosttyKey::F7,
        8 => GhosttyKey::F8,
        9 => GhosttyKey::F9,
        10 => GhosttyKey::F10,
        11 => GhosttyKey::F11,
        12 => GhosttyKey::F12,
        13 => GhosttyKey::F13,
        14 => GhosttyKey::F14,
        15 => GhosttyKey::F15,
        16 => GhosttyKey::F16,
        17 => GhosttyKey::F17,
        18 => GhosttyKey::F18,
        19 => GhosttyKey::F19,
        20 => GhosttyKey::F20,
        21 => GhosttyKey::F21,
        22 => GhosttyKey::F22,
        23 => GhosttyKey::F23,
        24 => GhosttyKey::F24,
        _ => GhosttyKey::Unidentified,
    }
}

fn handle_terminal_paste(terminal: &Terminal, event: &KeyDownEvent, cx: &mut EventContext) -> bool {
    let paste = key_character(&event.key).is_some_and(|value| value.eq_ignore_ascii_case("v"))
        && (event.modifiers.contains(Modifiers::SUPER)
            || (cfg!(not(target_os = "macos"))
                && event
                    .modifiers
                    .contains(Modifiers::CONTROL | Modifiers::SHIFT)));
    if !paste {
        return false;
    }
    if let Ok(Some(item)) = cx.read_from_clipboard()
        && let Some(value) = item.text()
    {
        let _ = terminal.paste(value);
    }
    cx.clear_text_selection();
    cx.prevent_default();
    cx.stop_propagation();
    true
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
struct CellVisual {
    foreground: Option<Color>,
    background: Option<Color>,
    bold: bool,
    italic: bool,
    underline: Option<Underline>,
    strikethrough: bool,
}

impl CellVisual {
    fn is_empty(self) -> bool {
        self == Self::default()
    }

    fn highlight(self) -> HighlightStyle {
        let mut style = HighlightStyle::default();
        if let Some(color) = self.foreground {
            style = style.color(color);
        }
        if let Some(color) = self.background {
            style = style.background(color);
        }
        if self.bold {
            style = style.font_bold();
        }
        if self.italic {
            style = style.italic();
        }
        style = match self.underline {
            Some(Underline::Double) => style.double_underline(),
            Some(Underline::Curly) => style.text_decoration_wavy(),
            Some(Underline::Single | Underline::Dotted | Underline::Dashed) => style.underline(),
            Some(Underline::None) | None => style,
            Some(_) => style.underline(),
        };
        if self.strikethrough {
            style = style.strikethrough();
        }
        style
    }
}

struct TerminalSnapshotMetadata {
    revision: u64,
    status: TerminalStatus,
    detected_agent: Option<DetectedAgentProcess>,
    recently_active: bool,
}

impl TerminalSnapshotMetadata {
    fn next(
        revision: &mut u64,
        status: TerminalStatus,
        detected_agent: Option<DetectedAgentProcess>,
        recently_active: bool,
    ) -> Self {
        *revision = revision.wrapping_add(1);
        Self {
            revision: *revision,
            status,
            detected_agent,
            recently_active,
        }
    }

    #[cfg(test)]
    fn running(revision: u64) -> Self {
        Self {
            revision,
            status: TerminalStatus::Running { process_id: None },
            detected_agent: None,
            recently_active: false,
        }
    }
}

fn publish_terminal_snapshot<'alloc: 'cb, 'cb>(
    terminal: &mut GhosttyTerminal<'alloc, 'cb>,
    render_state: &mut RenderState<'alloc>,
    rows: &mut RowIterator<'alloc>,
    cells: &mut CellIterator<'alloc>,
    shared: &RwLock<Arc<TerminalSnapshot>>,
    invalidator: &WindowInvalidator,
    metadata: TerminalSnapshotMetadata,
) {
    match build_snapshot(terminal, render_state, rows, cells, metadata) {
        Ok(snapshot) => publish(shared, invalidator, snapshot),
        Err(error) => publish_failure(
            shared,
            invalidator,
            TerminalSize {
                cols: DEFAULT_COLS,
                rows: DEFAULT_ROWS,
                cell_width_px: 0,
                cell_height_px: 0,
            },
            format!("could not render libghostty-vt state: {error}"),
        ),
    }
}

fn build_snapshot<'alloc: 'cb, 'cb>(
    terminal: &GhosttyTerminal<'alloc, 'cb>,
    render_state: &mut RenderState<'alloc>,
    rows: &mut RowIterator<'alloc>,
    cells: &mut CellIterator<'alloc>,
    metadata: TerminalSnapshotMetadata,
) -> Result<TerminalSnapshot, libghostty_vt::Error> {
    let TerminalSnapshotMetadata {
        revision,
        status,
        detected_agent,
        recently_active,
    } = metadata;
    let snapshot = render_state.update(terminal)?;
    let cols = snapshot.cols()?;
    let row_count = snapshot.rows()?;
    let scrollbar = terminal.scrollbar()?;
    let colors = snapshot.colors()?;
    let cursor_position = if snapshot.cursor_visible()? {
        snapshot.cursor_viewport()?
    } else {
        None
    };
    let foreground = ghostty_color(colors.foreground);
    let background = ghostty_color(colors.background);
    let cursor_style = snapshot.cursor_visual_style()?;
    let cursor_blinking = snapshot.cursor_blinking()?;
    let cursor = cursor_position.map(|position| TerminalCursor {
        column: position.x,
        row: position.y,
        style: match cursor_style {
            CursorVisualStyle::Bar => TerminalCursorStyle::Bar,
            CursorVisualStyle::Block => TerminalCursorStyle::Block,
            CursorVisualStyle::Underline => TerminalCursorStyle::Underline,
            CursorVisualStyle::BlockHollow => TerminalCursorStyle::BlockHollow,
            _ => TerminalCursorStyle::Block,
        },
        blinking: cursor_blinking,
        color: ghostty_color(colors.cursor.unwrap_or(colors.foreground)),
    });
    let mut cursor_range = None;
    let mut content =
        String::with_capacity(cols as usize * row_count as usize + row_count as usize);
    let mut highlights = Vec::new();
    let mut graphics = Vec::new();
    let mut top_backgrounds = vec![None; usize::from(cols)];
    let mut right_backgrounds = vec![None; usize::from(row_count)];
    let mut bottom_backgrounds = vec![None; usize::from(cols)];
    let mut left_backgrounds = vec![None; usize::from(row_count)];
    let mut graphemes = Vec::new();
    let mut active_run: Option<(usize, usize, CellVisual)> = None;
    let mut row_iterator = rows.update(&snapshot)?;
    let mut row_index = 0_u16;
    while let Some(row) = row_iterator.next() {
        let row_start = content.len();
        let mut row_visible_end = row_start;
        let mut cell_iterator = cells.update(row)?;
        let mut column = 0_u16;
        let mut grid_column = 0_u16;
        while let Some(cell) = cell_iterator.next() {
            let raw = cell.raw_cell()?;
            let wide = raw.wide()?;
            let style = cell.style()?;
            let mut resolved_foreground = cell.fg_color()?.map(ghostty_color).unwrap_or(foreground);
            let mut resolved_background = cell.bg_color()?.map(ghostty_color).unwrap_or(background);
            if style.inverse {
                std::mem::swap(&mut resolved_foreground, &mut resolved_background);
            }
            let edge_background =
                (resolved_background != background).then_some(resolved_background);
            let edge_column = usize::from(grid_column);
            let edge_row = usize::from(row_index);
            if edge_column < usize::from(cols) {
                if edge_row == 0 {
                    top_backgrounds[edge_column] = edge_background;
                }
                if edge_row + 1 == usize::from(row_count) {
                    bottom_backgrounds[edge_column] = edge_background;
                }
            }
            if edge_row < usize::from(row_count) {
                if edge_column == 0 {
                    left_backgrounds[edge_row] = edge_background;
                }
                if edge_column + 1 == usize::from(cols) {
                    right_backgrounds[edge_row] = edge_background;
                }
            }
            grid_column = grid_column.saturating_add(1);
            if wide == CellWide::SpacerTail {
                column = column.saturating_add(1);
                continue;
            }
            let start = content.len();
            let mut has_visible_text = false;
            let mut graphic_character = None;
            if raw.has_text()? {
                let grapheme_count = cell.graphemes_len()?;
                graphemes.resize(grapheme_count, '\0');
                cell.graphemes_buf(&mut graphemes)?;
                has_visible_text = graphemes.iter().any(|value| !value.is_whitespace());
                graphic_character = (graphemes.len() == 1)
                    .then_some(graphemes[0])
                    .filter(|character| is_block_element(*character));
                content.extend(graphemes.iter().copied());
            } else {
                content.push(' ');
                if wide == CellWide::Wide {
                    content.push(' ');
                }
            }
            let end = content.len();
            if style.invisible {
                resolved_foreground = resolved_background;
            } else if style.faint {
                resolved_foreground = resolved_foreground.with_alpha(0.65);
            }
            let cursor_here = cursor_position.is_some_and(|cursor| {
                cursor.y == row_index
                    && (cursor.x == column
                        || (cursor.at_wide_tail
                            && wide == CellWide::Wide
                            && cursor.x == column.saturating_add(1)))
            });
            if let Some(character) = graphic_character {
                graphics.push(TerminalCellGraphic {
                    column,
                    row: row_index,
                    character,
                    foreground: (resolved_foreground != foreground).then_some(resolved_foreground),
                });
            }
            let visual = CellVisual {
                foreground: if graphic_character.is_some() {
                    Some(Color::TRANSPARENT)
                } else {
                    (resolved_foreground != foreground).then_some(resolved_foreground)
                },
                background: (resolved_background != background).then_some(resolved_background),
                bold: style.bold,
                italic: style.italic,
                underline: (style.underline != Underline::None).then_some(style.underline),
                strikethrough: style.strikethrough,
            };
            push_visual_run(&mut active_run, &mut highlights, start, end, visual);
            if cursor_here {
                cursor_range = Some(start..end);
            }
            if has_visible_text || cursor_here || visual.has_visible_blank_paint() {
                row_visible_end = end;
            }
            column = column.saturating_add(match wide {
                CellWide::Wide => 2,
                _ => 1,
            });
        }
        flush_visual_run(&mut active_run, &mut highlights);
        trim_terminal_row(&mut content, &mut highlights, row_visible_end);
        row_index = row_index.saturating_add(1);
        if row_index < row_count {
            content.push('\n');
        }
    }
    flush_visual_run(&mut active_run, &mut highlights);
    while content.ends_with('\n') {
        content.pop();
    }
    trim_terminal_highlights(&mut highlights, content.len());
    let title = terminal.title().unwrap_or_default();
    let agent = detected_agent.map(|detected| TerminalAgent {
        kind: Arc::from(detected.kind),
        status: detect_agent_status(detected.kind, &content, title, recently_active),
        process_id: detected.process_id,
    });
    Ok(TerminalSnapshot {
        revision,
        cols,
        rows: row_count,
        content: Arc::from(content),
        highlights: highlights.into(),
        foreground,
        background,
        title: Arc::from(title),
        working_directory: Arc::from(normalize_terminal_working_directory(
            terminal.pwd().unwrap_or_default(),
        )),
        scroll: TerminalScrollState {
            total_rows: scrollbar.total,
            offset_rows: scrollbar.offset,
            viewport_rows: scrollbar.len,
        },
        cursor,
        cursor_range,
        graphics: graphics.into(),
        edge_backgrounds: EdgeBackgrounds::new(
            top_backgrounds,
            right_backgrounds,
            bottom_backgrounds,
            left_backgrounds,
        ),
        status,
        agent,
    })
}

/// libghostty exposes OSC 7 working directories exactly as emitted by the child. Shells commonly
/// use a `file://host/path` URI there, while the public terminal status contract is a filesystem
/// path. Keep ordinary paths untouched and decode the path portion of local file URIs.
fn normalize_terminal_working_directory(value: &str) -> String {
    let Some(uri) = value.strip_prefix("file://") else {
        return value.to_owned();
    };
    let path = if uri.starts_with('/') {
        uri
    } else {
        uri.find('/').map_or("", |separator| &uri[separator..])
    };
    let bytes = path.as_bytes();
    let mut decoded = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'%'
            && index + 2 < bytes.len()
            && let (Some(high), Some(low)) =
                (hex_digit(bytes[index + 1]), hex_digit(bytes[index + 2]))
        {
            decoded.push((high << 4) | low);
            index += 3;
            continue;
        }
        decoded.push(bytes[index]);
        index += 1;
    }
    String::from_utf8_lossy(&decoded).into_owned()
}

const fn hex_digit(value: u8) -> Option<u8> {
    match value {
        b'0'..=b'9' => Some(value - b'0'),
        b'a'..=b'f' => Some(value - b'a' + 10),
        b'A'..=b'F' => Some(value - b'A' + 10),
        _ => None,
    }
}

fn detect_agent_status(
    kind: &str,
    content: &str,
    title: &str,
    recently_active: bool,
) -> TerminalAgentStatus {
    let tail = content
        .lines()
        .rev()
        .take(16)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect::<Vec<_>>()
        .join("\n")
        .to_ascii_lowercase();
    let title = title.to_ascii_lowercase();
    let blocked = [
        "action required",
        "permission required",
        "do you want to proceed?",
        "do you trust the contents of this directory?",
        "allow command?",
        "press enter to confirm or esc to cancel",
        "enter to submit answer",
        "waiting for permission",
        "do you want to allow this connection?",
    ]
    .iter()
    .any(|marker| tail.contains(marker) || title.contains(marker));
    if blocked {
        return TerminalAgentStatus::Blocked;
    }

    let working = [
        "esc to interrupt",
        "ctrl+c to interrupt",
        "press esc to interrupt",
        "shells ·",
        "mcp tasks still running",
        "waiting for background agent",
    ]
    .iter()
    .any(|marker| tail.contains(marker) || title.contains(marker))
        || title.chars().next().is_some_and(|character| {
            matches!(
                character,
                '⠋' | '⠙'
                    | '⠹'
                    | '⠸'
                    | '⠼'
                    | '⠴'
                    | '⠦'
                    | '⠧'
                    | '⠇'
                    | '⠏'
                    | '◐'
                    | '◓'
                    | '◑'
                    | '◒'
            )
        });
    if working || recently_active {
        TerminalAgentStatus::Working
    } else {
        let _ = kind;
        TerminalAgentStatus::Idle
    }
}

impl CellVisual {
    fn has_visible_blank_paint(self) -> bool {
        self.background.is_some() || self.underline.is_some() || self.strikethrough
    }
}

fn trim_terminal_row(
    content: &mut String,
    highlights: &mut Vec<(Range<usize>, HighlightStyle)>,
    visible_end: usize,
) {
    content.truncate(visible_end);
    trim_terminal_highlights(highlights, visible_end);
}

fn trim_terminal_highlights(
    highlights: &mut Vec<(Range<usize>, HighlightStyle)>,
    content_len: usize,
) {
    while highlights
        .last()
        .is_some_and(|(range, _)| range.start >= content_len)
    {
        highlights.pop();
    }
    if let Some((range, _)) = highlights.last_mut() {
        range.end = range.end.min(content_len);
    }
}

fn push_visual_run(
    active: &mut Option<(usize, usize, CellVisual)>,
    highlights: &mut Vec<(Range<usize>, HighlightStyle)>,
    start: usize,
    end: usize,
    visual: CellVisual,
) {
    if start == end || visual.is_empty() {
        flush_visual_run(active, highlights);
        return;
    }
    if let Some((_, run_end, current)) = active
        && *run_end == start
        && *current == visual
    {
        *run_end = end;
        return;
    }
    flush_visual_run(active, highlights);
    *active = Some((start, end, visual));
}

fn flush_visual_run(
    active: &mut Option<(usize, usize, CellVisual)>,
    highlights: &mut Vec<(Range<usize>, HighlightStyle)>,
) {
    let Some((start, end, visual)) = active.take() else {
        return;
    };
    if highlights.len() < MAX_TEXT_HIGHLIGHTS {
        highlights.push((start..end, visual.highlight()));
    }
}

fn publish(
    shared: &RwLock<Arc<TerminalSnapshot>>,
    invalidator: &WindowInvalidator,
    snapshot: TerminalSnapshot,
) {
    *shared
        .write()
        .unwrap_or_else(|poisoned| poisoned.into_inner()) = Arc::new(snapshot);
    let _ = invalidator.invalidate();
}

fn publish_failure(
    shared: &RwLock<Arc<TerminalSnapshot>>,
    invalidator: &WindowInvalidator,
    size: TerminalSize,
    message: String,
) {
    let content = Arc::<str>::from(format!("QuickGUI terminal error\n\n{message}"));
    publish(
        shared,
        invalidator,
        TerminalSnapshot {
            revision: shared
                .read()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .revision
                .wrapping_add(1),
            cols: size.cols,
            rows: size.rows,
            content,
            highlights: Arc::from([]),
            foreground: Color::rgb8(248, 113, 113),
            background: Color::rgb8(20, 20, 20),
            title: Arc::from("Terminal error"),
            working_directory: Arc::from(""),
            scroll: TerminalScrollState::initial(size.rows),
            cursor: None,
            cursor_range: None,
            graphics: Arc::from([]),
            edge_backgrounds: EdgeBackgrounds::empty(size.cols, size.rows),
            status: TerminalStatus::Failed {
                message: Arc::from(message),
            },
            agent: None,
        },
    );
}

fn ghostty_color(color: RgbColor) -> Color {
    Color::rgb8(color.r, color.g, color.b)
}

fn valid_os_string(value: &OsStr) -> bool {
    let value = value.to_string_lossy();
    value.len() <= MAX_TERMINAL_STRING_BYTES && !value.contains('\0')
}

fn finite_clamp(value: f32, min: f32, max: f32, fallback: f32) -> f32 {
    if value.is_finite() {
        value.clamp(min, max)
    } else {
        fallback
    }
}

fn pack_size(size: TerminalSize) -> u64 {
    u64::from(size.cols)
        | (u64::from(size.rows) << 16)
        | (u64::from(size.cell_width_px.min(u16::MAX as u32)) << 32)
        | (u64::from(size.cell_height_px.min(u16::MAX as u32)) << 48)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn options_reject_unbounded_process_declarations() {
        let options = TerminalOptions {
            arguments: vec![OsString::from("x"); MAX_TERMINAL_ARGUMENTS + 1],
            ..TerminalOptions::default()
        };
        assert!(matches!(
            options.validate(),
            Err(TerminalError::TooManyArguments)
        ));
    }

    #[test]
    fn ghostty_character_mapping_covers_terminal_control_keys() {
        assert_eq!(ghostty_character_key('c'), GhosttyKey::C);
        assert_eq!(ghostty_character_key('C'), GhosttyKey::C);
        assert_eq!(ghostty_character_key('!'), GhosttyKey::Digit1);
        assert_eq!(unshift_character('?'), '/');
    }

    #[test]
    fn composed_key_text_overrides_a_synthetic_physical_key() {
        let physical = Key::Character("a".to_owned());
        let (key, text, unshifted) =
            ghostty_key(&physical, Some(&physical), Some("Z")).expect("mapped key");
        assert_eq!(key, GhosttyKey::Z);
        assert_eq!(text.as_deref(), Some("Z"));
        assert_eq!(unshifted, Some('a'));
    }

    #[test]
    fn terminal_rows_drop_invisible_grid_padding_and_clip_highlights() {
        let mut content = String::from("prompt    ");
        let mut highlights = vec![(4..10, HighlightStyle::default())];

        trim_terminal_row(&mut content, &mut highlights, 6);

        assert_eq!(content, "prompt");
        assert_eq!(highlights.len(), 1);
        assert_eq!(highlights[0].0, 4..6);
    }

    #[test]
    fn terminal_cursor_and_blank_cell_backgrounds_remain_visible() {
        assert!(!CellVisual::default().has_visible_blank_paint());
        assert!(
            CellVisual {
                background: Some(Color::WHITE),
                ..CellVisual::default()
            }
            .has_visible_blank_paint()
        );
    }

    #[test]
    fn terminal_cursor_highlight_overrides_and_splits_ansi_runs() {
        let ansi = HighlightStyle::default().color(Color::rgb8(255, 0, 0));
        let cursor = HighlightStyle::default()
            .color(Color::BLACK)
            .background(Color::WHITE);
        let highlights = terminal_highlights_with_cursor(&[(0..10, ansi.clone())], 4..5, cursor);

        assert_eq!(highlights.len(), 3);
        assert_eq!(highlights[0], (0..4, ansi.clone()));
        assert_eq!(highlights[1].0, 4..5);
        assert_eq!(highlights[2], (5..10, ansi));
    }

    #[test]
    fn terminal_cursor_blink_restarts_immediately_on_each_focus_gain() {
        let start = Instant::now();
        let mut blink = TerminalCursorBlink::new(start);
        let first_focus = start + Duration::from_millis(750);

        assert_eq!(blink.update_focus(true, first_focus), first_focus);
        assert_eq!(
            blink.update_focus(true, first_focus + Duration::from_millis(250)),
            first_focus,
            "ordinary focused redraws must not restart the blink cycle"
        );
        assert_eq!(
            blink.update_focus(false, first_focus + Duration::from_secs(1)),
            first_focus
        );

        let second_focus = first_focus + Duration::from_secs(2);
        assert_eq!(blink.update_focus(true, second_focus), second_focus);
    }

    #[test]
    fn terminal_wheel_follows_platform_content_motion_and_retains_fractional_rows() {
        let mut remainder = 0.0;
        assert_eq!(accumulate_terminal_rows(&mut remainder, -0.4), 0);
        assert_eq!(accumulate_terminal_rows(&mut remainder, -0.7), -1);
        assert!((-0.11..=-0.09).contains(&remainder));
        assert_eq!(accumulate_terminal_rows(&mut remainder, 1.1), 1);

        let mut wheel = 0.0;
        let line_height = 18.0;
        assert_eq!(
            accumulate_terminal_rows(&mut wheel, -line_height / line_height),
            -1,
            "positive platform content motion must move the terminal viewport into history"
        );
        assert_eq!(
            accumulate_terminal_rows(&mut wheel, line_height / line_height),
            1,
            "negative platform content motion must move the terminal viewport toward the prompt"
        );
    }

    #[test]
    fn terminal_scrollbar_geometry_reaches_both_ends() {
        let mut state = TerminalScrollState {
            total_rows: 100,
            offset_rows: 0,
            viewport_rows: 20,
        };
        let top = terminal_scrollbar_geometry(state, 200.0).unwrap();
        assert_eq!(top.thumb_top, 0.0);
        assert_eq!(top.thumb_height, 40.0);
        assert_eq!(top.travel, 160.0);

        state.offset_rows = state.max_offset_rows();
        let bottom = terminal_scrollbar_geometry(state, 200.0).unwrap();
        assert_eq!(bottom.thumb_top, bottom.travel);
        assert!(terminal_scrollbar_geometry(TerminalScrollState::initial(24), 200.0).is_none());
    }

    #[test]
    fn terminal_scrollbar_reveals_expands_and_auto_hides() {
        let start = Instant::now();
        let mut interaction = TerminalScrollbarInteraction::default();
        assert_eq!(
            interaction.presentation(start),
            TerminalScrollbarPresentation {
                visible: false,
                expanded: false,
                hide_at: None,
            }
        );

        interaction.reveal(start);
        let revealed = interaction.presentation(start);
        assert!(revealed.visible);
        assert!(!revealed.expanded);
        assert_eq!(
            revealed.hide_at,
            Some(start + TERMINAL_SCROLLBAR_HIDE_DELAY)
        );

        interaction.set_hovered(true, start + Duration::from_millis(100));
        let hovered = interaction.presentation(start + Duration::from_millis(100));
        assert!(hovered.visible);
        assert!(hovered.expanded);
        assert_eq!(hovered.hide_at, None);

        interaction.set_hovered(false, start + Duration::from_millis(200));
        assert!(
            interaction
                .presentation(start + Duration::from_millis(200))
                .visible
        );
        assert!(
            !interaction
                .presentation(start + Duration::from_millis(200) + TERMINAL_SCROLLBAR_HIDE_DELAY)
                .visible
        );
    }

    #[test]
    fn every_encoded_keystroke_follows_the_live_prompt() {
        let terminal = GhosttyTerminal::new(GhosttyTerminalOptions {
            cols: 80,
            rows: 24,
            max_scrollback: 100,
        })
        .unwrap();
        let mut encoder = GhosttyKeyEncoder::new().unwrap();
        let mut writer: Box<dyn Write + Send> = Box::new(Vec::<u8>::new());
        let input = |key, text, action| TerminalKeyInput {
            key,
            key_char: None,
            text,
            modifiers: Modifiers::empty(),
            action,
        };

        assert!(write_key(
            &terminal,
            &mut encoder,
            &mut writer,
            input(
                Key::Character("a".to_owned()),
                Some("a".to_owned()),
                GhosttyKeyAction::Press,
            ),
        ));
        assert!(write_key(
            &terminal,
            &mut encoder,
            &mut writer,
            input(Key::Backspace, None, GhosttyKeyAction::Press),
        ));
        assert!(write_key(
            &terminal,
            &mut encoder,
            &mut writer,
            input(Key::ArrowUp, None, GhosttyKeyAction::Press),
        ));
        assert!(write_key(
            &terminal,
            &mut encoder,
            &mut writer,
            input(Key::Enter, None, GhosttyKeyAction::Repeat),
        ));
        assert!(!write_key(
            &terminal,
            &mut encoder,
            &mut writer,
            input(Key::Other, None, GhosttyKeyAction::Press),
        ));
    }

    #[test]
    fn terminal_style_normalizes_content_padding() {
        let style = TerminalStyle {
            font_thicken: true,
            padding_top: 8.0,
            padding_right: 9.0,
            padding_bottom: f32::INFINITY,
            padding_left: -4.0,
            padding_color: TerminalPaddingColor::Extend,
            ..TerminalStyle::default()
        }
        .normalized();

        assert_eq!(style.padding_top, 8.0);
        assert_eq!(style.padding_right, 9.0);
        assert_eq!(style.padding_bottom, 0.0);
        assert_eq!(style.padding_left, 0.0);
        assert_eq!(style.padding_color, TerminalPaddingColor::Extend);
        assert!(style.font_thicken);
    }

    #[test]
    fn ghostty_snapshot_retains_complete_lines() {
        let mut terminal = GhosttyTerminal::new(GhosttyTerminalOptions {
            cols: 80,
            rows: 24,
            max_scrollback: 100,
        })
        .unwrap();
        terminal.vt_write(b"quickgui-pty-ok\r\n");
        let mut render_state = RenderState::new().unwrap();
        let mut rows = RowIterator::new().unwrap();
        let mut cells = CellIterator::new().unwrap();
        let snapshot = build_snapshot(
            &terminal,
            &mut render_state,
            &mut rows,
            &mut cells,
            TerminalSnapshotMetadata::running(1),
        )
        .unwrap();
        assert!(
            snapshot.content.contains("quickgui-pty-ok"),
            "snapshot: {snapshot:#?}"
        );
        assert_eq!(snapshot.scroll.viewport_rows, 24);
        assert_eq!(snapshot.scroll.total_rows, 24);
        assert_eq!(snapshot.scroll.offset_rows, 0);
    }

    #[test]
    fn ghostty_snapshot_retains_grid_edge_backgrounds() {
        let mut terminal = GhosttyTerminal::new(GhosttyTerminalOptions {
            cols: 4,
            rows: 3,
            max_scrollback: 100,
        })
        .unwrap();
        terminal.vt_write(b"\x1b[48;2;10;20;30m\x1b[2J\x1b[H");
        let mut render_state = RenderState::new().unwrap();
        let mut rows = RowIterator::new().unwrap();
        let mut cells = CellIterator::new().unwrap();
        let snapshot = build_snapshot(
            &terminal,
            &mut render_state,
            &mut rows,
            &mut cells,
            TerminalSnapshotMetadata::running(1),
        )
        .unwrap();
        let surface = Color::rgb8(10, 20, 30);

        assert!(
            snapshot
                .edge_backgrounds
                .top
                .iter()
                .chain(snapshot.edge_backgrounds.right.iter())
                .chain(snapshot.edge_backgrounds.bottom.iter())
                .chain(snapshot.edge_backgrounds.left.iter())
                .all(|color| *color == Some(surface)),
            "edge backgrounds: {:?}",
            snapshot.edge_backgrounds,
        );
    }

    #[test]
    fn ghostty_snapshot_preserves_block_text_and_uses_cell_graphics() {
        let mut terminal = GhosttyTerminal::new(GhosttyTerminalOptions {
            cols: 20,
            rows: 4,
            max_scrollback: 100,
        })
        .unwrap();
        terminal.vt_write("█▀▄\r\n".as_bytes());
        let mut render_state = RenderState::new().unwrap();
        let mut rows = RowIterator::new().unwrap();
        let mut cells = CellIterator::new().unwrap();
        let snapshot = build_snapshot(
            &terminal,
            &mut render_state,
            &mut rows,
            &mut cells,
            TerminalSnapshotMetadata::running(1),
        )
        .unwrap();

        assert!(snapshot.content.starts_with("█▀▄"));
        assert_eq!(
            snapshot
                .graphics
                .iter()
                .map(|graphic| (graphic.column, graphic.row, graphic.character))
                .collect::<Vec<_>>(),
            [(0, 0, '█'), (1, 0, '▀'), (2, 0, '▄')]
        );
        assert_eq!(snapshot.highlights[0].0, 0.."█▀▄".len());
        assert_eq!(snapshot.highlights[0].1.color, Some(Color::TRANSPARENT));
    }

    #[test]
    fn ghostty_snapshot_preserves_spinner_columns() {
        let mut terminal = GhosttyTerminal::new(GhosttyTerminalOptions {
            cols: 20,
            rows: 4,
            max_scrollback: 100,
        })
        .unwrap();
        terminal.vt_write("■■■■⬝⬝⬝⬝".as_bytes());
        let mut render_state = RenderState::new().unwrap();
        let mut rows = RowIterator::new().unwrap();
        let mut cells = CellIterator::new().unwrap();
        let snapshot = build_snapshot(
            &terminal,
            &mut render_state,
            &mut rows,
            &mut cells,
            TerminalSnapshotMetadata::running(1),
        )
        .unwrap();

        assert!(
            snapshot.content.starts_with("■■■■⬝⬝⬝⬝"),
            "snapshot content: {:?}",
            snapshot.content,
        );
        assert_eq!(snapshot.cursor.unwrap().column, 8);
    }

    #[test]
    fn terminal_theme_updates_libghostty_defaults_and_ansi_palette() {
        let mut terminal = GhosttyTerminal::new(GhosttyTerminalOptions {
            cols: 20,
            rows: 4,
            max_scrollback: 100,
        })
        .unwrap();
        let ansi = std::array::from_fn(|index| {
            Color::rgb8(index as u8, (index as u8).saturating_add(16), 240)
        });
        let theme = TerminalTheme::new(Color::rgb8(31, 35, 40), Color::WHITE, ansi)
            .cursor(Color::rgb8(9, 105, 218));
        apply_terminal_theme(&mut terminal, Some(&theme)).unwrap();
        terminal.vt_write(b"\x1b[31mred");

        let colors = terminal.default_color_palette().unwrap();
        assert_eq!(colors.0[1], terminal_rgb(theme.ansi[1]));
        assert_eq!(
            terminal.default_fg_color().unwrap(),
            Some(terminal_rgb(theme.foreground))
        );
        assert_eq!(
            terminal.default_bg_color().unwrap(),
            Some(terminal_rgb(theme.background))
        );
        assert_eq!(
            terminal.default_cursor_color().unwrap(),
            Some(terminal_rgb(theme.cursor))
        );

        let mut render_state = RenderState::new().unwrap();
        let mut rows = RowIterator::new().unwrap();
        let mut cells = CellIterator::new().unwrap();
        let snapshot = build_snapshot(
            &terminal,
            &mut render_state,
            &mut rows,
            &mut cells,
            TerminalSnapshotMetadata::running(1),
        )
        .unwrap();
        assert_eq!(snapshot.foreground, theme.foreground);
        assert_eq!(snapshot.background, theme.background);
        assert_eq!(snapshot.highlights[0].1.color, Some(theme.ansi[1]));
    }

    #[test]
    fn ghostty_snapshot_retains_the_cursor_cell_and_shape() {
        let mut terminal = GhosttyTerminal::new(GhosttyTerminalOptions {
            cols: 20,
            rows: 4,
            max_scrollback: 100,
        })
        .unwrap();
        terminal.vt_write(b"prompt");
        let mut render_state = RenderState::new().unwrap();
        let mut rows = RowIterator::new().unwrap();
        let mut cells = CellIterator::new().unwrap();
        let snapshot = build_snapshot(
            &terminal,
            &mut render_state,
            &mut rows,
            &mut cells,
            TerminalSnapshotMetadata::running(1),
        )
        .unwrap();

        let cursor = snapshot.cursor.expect("visible cursor metadata");
        assert_eq!((cursor.column, cursor.row), (6, 0));
        assert_eq!(cursor.style, TerminalCursorStyle::Block);
        assert_eq!(snapshot.cursor_range, Some(6..7));
        assert_eq!(snapshot.content.as_ref(), "prompt ");
    }

    #[test]
    fn ghostty_snapshot_reports_real_scrollback_position() {
        let mut terminal = GhosttyTerminal::new(GhosttyTerminalOptions {
            cols: 40,
            rows: 10,
            max_scrollback: 100,
        })
        .unwrap();
        for line in 0..40 {
            terminal.vt_write(format!("line {line}\r\n").as_bytes());
        }
        let mut render_state = RenderState::new().unwrap();
        let mut rows = RowIterator::new().unwrap();
        let mut cells = CellIterator::new().unwrap();
        let bottom = build_snapshot(
            &terminal,
            &mut render_state,
            &mut rows,
            &mut cells,
            TerminalSnapshotMetadata::running(1),
        )
        .unwrap();
        assert!(bottom.scroll.total_rows > bottom.scroll.viewport_rows);
        assert_eq!(
            bottom.scroll.offset_rows,
            bottom.scroll.max_offset_rows(),
            "the live prompt is the bottom of libghostty's scroll range"
        );

        terminal.scroll_viewport(ScrollViewport::Top);
        let top = build_snapshot(
            &terminal,
            &mut render_state,
            &mut rows,
            &mut cells,
            TerminalSnapshotMetadata::running(2),
        )
        .unwrap();
        assert_eq!(top.scroll.offset_rows, 0);
    }

    #[test]
    fn visual_runs_merge_and_never_exceed_styled_text_limit() {
        let visual = CellVisual {
            foreground: Some(Color::rgb8(255, 0, 0)),
            ..CellVisual::default()
        };
        let mut active = None;
        let mut highlights = Vec::new();
        push_visual_run(&mut active, &mut highlights, 0, 1, visual);
        push_visual_run(&mut active, &mut highlights, 1, 2, visual);
        flush_visual_run(&mut active, &mut highlights);
        assert_eq!(highlights.len(), 1);
        assert_eq!(highlights[0].0, 0..2);
    }

    #[test]
    fn terminal_identity_matches_ghostty_compatibility_contract() {
        assert_eq!(TERM_PROGRAM_VALUE, "ghostty");
    }

    #[test]
    fn terminal_working_directory_is_a_local_path_not_an_osc_7_uri() {
        assert_eq!(
            normalize_terminal_working_directory(
                "file://egoists-MacBook-Air.local/Users/egoist/My%20Project"
            ),
            "/Users/egoist/My Project"
        );
        assert_eq!(
            normalize_terminal_working_directory("file:///private/tmp/herdr"),
            "/private/tmp/herdr"
        );
        assert_eq!(
            normalize_terminal_working_directory("/Users/egoist/dev/quickgui"),
            "/Users/egoist/dev/quickgui"
        );
    }

    #[test]
    fn detected_agent_status_uses_live_terminal_signals() {
        assert_eq!(
            detect_agent_status(
                "claude",
                "Bash command\nDo you want to proceed?\n1. Yes\n2. No",
                "Action Required",
                false,
            ),
            TerminalAgentStatus::Blocked
        );
        assert_eq!(
            detect_agent_status("codex", "Working (esc to interrupt)", "", false),
            TerminalAgentStatus::Working
        );
        assert_eq!(
            detect_agent_status("opencode", "ready", "", false),
            TerminalAgentStatus::Idle
        );
    }
}
