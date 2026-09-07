//! Versioned, process-local extension ABI. Only C layouts, borrowed spans and opaque handles
//! cross this boundary. Extensions must not link another copy of the renderer or host runtime.

use std::ffi::c_void;

pub const ABI_VERSION: u32 = 1;
pub const TERMINAL_EXTENSION: u32 = 1;
pub const MAX_EXTENSION_NAME: usize = 64;
pub const MAX_FRAME_TEXT: usize = 16 * 1024 * 1024;
pub const MAX_FRAME_HIGHLIGHTS: usize = 4096;
pub const MAX_FRAME_CELLS: usize = 512 * 256;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct Bytes {
    pub data: *const u8,
    pub len: usize,
}

impl Bytes {
    pub fn new(value: &[u8]) -> Self {
        Self {
            data: value.as_ptr(),
            len: value.len(),
        }
    }
}

/// The descriptor and its function table remain valid until process exit. A consumer checks
/// the header before reading the table. Release versions additionally match the Go SDK version.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct Extension {
    pub abi_version: u32,
    pub descriptor_size: u32,
    pub kind: u32,
    pub api_size: u32,
    pub name: Bytes,
    pub version: Bytes,
    pub api: *const c_void,
}

// SAFETY: Published descriptors only refer to immutable static bytes and function tables.
unsafe impl Sync for Extension {}

/// Ownership of the callback context transfers to `create`, including its error path. The
/// extension calls release exactly once after its final worker can no longer call wake.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct Wake {
    pub context: *mut c_void,
    pub wake: unsafe extern "C" fn(*mut c_void),
    pub release: unsafe extern "C" fn(*mut c_void),
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Rgba {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct Highlight {
    pub start: u32,
    pub end: u32,
    // 1 foreground, 2 background, 4 bold, 8 italic, 16 underline,
    // 32 double underline, 64 wavy underline, 128 strikethrough.
    pub flags: u32,
    pub foreground: Rgba,
    pub background: Rgba,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct Graphic {
    pub column: u16,
    pub row: u16,
    pub character: u32,
    pub has_foreground: u32,
    pub foreground: Rgba,
}

/// One immutable visible frame. Every span is borrowed only during the frame callback. The
/// receiver copies it once per revision; unchanged reads return without allocating or copying.
#[repr(C)]
pub struct TerminalFrame {
    pub revision: u64,
    pub cols: u16,
    pub rows: u16,
    pub foreground: Rgba,
    pub background: Rgba,
    pub content: Bytes,
    pub title: Bytes,
    pub working_directory: Bytes,
    pub selected_text: Bytes,
    pub has_selection: u32,
    pub highlights: *const Highlight,
    pub highlights_len: usize,
    pub graphics: *const Graphic,
    pub graphics_len: usize,
    // top, right, bottom, left concatenated; a negative alpha represents an absent color.
    pub edges: *const Rgba,
    pub edges_len: usize,
    pub total_rows: u64,
    pub offset_rows: u64,
    pub viewport_rows: u64,
    pub has_cursor: u32,
    pub cursor_column: u16,
    pub cursor_row: u16,
    pub cursor_style: u32,
    pub cursor_blinking: u32,
    pub cursor_color: Rgba,
    pub cursor_start: u32,
    pub cursor_end: u32,
    pub has_cursor_range: u32,
    // 0 starting, 1 running, 2 exited, 3 failed.
    pub status: u32,
    pub process_id: u32,
    pub exit_code: u32,
    pub signal: Bytes,
    pub error: Bytes,
    pub agent: Bytes,
    pub agent_status: u32,
    pub agent_process_id: u32,
}

pub type FrameCallback = unsafe extern "C" fn(*mut c_void, *const TerminalFrame);
pub type Reply = unsafe extern "C" fn(*mut c_void, Bytes);

/// Commands carry bounded JSON configuration/input metadata; raw PTY input and paste use raw
/// UTF-8/byte spans. Visible frames use the typed batched interface above, never per-cell calls.
pub const INPUT: u32 = 1;
pub const PASTE: u32 = 2;
pub const KEY: u32 = 3;
pub const RESIZE: u32 = 4;
pub const SCROLL: u32 = 5;
pub const SELECTION: u32 = 6;
pub const SELECT_ALL: u32 = 7;
pub const THEME: u32 = 8;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct TerminalApi {
    pub create: unsafe extern "C" fn(Bytes, Wake, *mut c_void, Reply) -> *mut c_void,
    pub destroy: unsafe extern "C" fn(*mut c_void),
    pub command: unsafe extern "C" fn(*mut c_void, u32, Bytes) -> i32,
    pub frame: unsafe extern "C" fn(*mut c_void, u64, *mut c_void, FrameCallback) -> i32,
}
