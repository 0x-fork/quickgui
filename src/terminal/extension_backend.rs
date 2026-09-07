//! C adapter for the shared PTY worker. Screen frames are borrowed in one callback; neither
//! allocator ownership nor Rust objects cross the extension boundary.
use super::*;
use crate::extension_api::{self as abi, Bytes, Rgba};
use serde::Deserialize;
use std::{
    ffi::c_void,
    panic::{AssertUnwindSafe, catch_unwind},
};

static API: abi::TerminalApi = abi::TerminalApi {
    create,
    destroy,
    command,
    frame,
};
static DESCRIPTOR: abi::Extension = abi::Extension {
    abi_version: abi::ABI_VERSION,
    descriptor_size: size_of::<abi::Extension>() as u32,
    kind: abi::TERMINAL_EXTENSION,
    api_size: size_of::<abi::TerminalApi>() as u32,
    name: Bytes {
        data: b"terminal".as_ptr(),
        len: 8,
    },
    version: Bytes {
        data: env!("CARGO_PKG_VERSION").as_ptr(),
        len: env!("CARGO_PKG_VERSION").len(),
    },
    api: (&API as *const abi::TerminalApi).cast(),
};

pub(crate) fn descriptor() -> *const abi::Extension {
    &DESCRIPTOR
}

unsafe fn bytes<'a>(value: Bytes, limit: usize) -> Option<&'a [u8]> {
    if value.len > limit || (value.len != 0 && value.data.is_null()) {
        return None;
    }
    Some(if value.len == 0 {
        &[]
    } else {
        unsafe { std::slice::from_raw_parts(value.data, value.len) }
    })
}

#[derive(Deserialize)]
struct Options {
    program: Option<String>,
    arguments: Vec<String>,
    working_directory: Option<String>,
    environment: Vec<(String, String)>,
    cols: u16,
    rows: u16,
    max_scrollback: usize,
}

unsafe extern "C" fn create(
    input: Bytes,
    wake: abi::Wake,
    context: *mut c_void,
    reply: abi::Reply,
) -> *mut c_void {
    // Construct the guard before parsing so every failure releases the transferred context.
    let invalidator = WindowInvalidator(Arc::new(crate::WakeOwner(wake)));
    let result = catch_unwind(AssertUnwindSafe(|| -> Result<Terminal, String> {
        let data = unsafe { bytes(input, MAX_TERMINAL_INPUT_BYTES) }
            .ok_or("invalid terminal options span")?;
        let options: Options = serde_json::from_slice(data).map_err(|e| e.to_string())?;
        Terminal::spawn(
            TerminalOptions {
                program: options.program.map(Into::into),
                arguments: options.arguments.into_iter().map(Into::into).collect(),
                working_directory: options.working_directory.map(Into::into),
                environment: options
                    .environment
                    .into_iter()
                    .map(|(k, v)| (k.into(), v.into()))
                    .collect(),
                cols: options.cols,
                rows: options.rows,
                max_scrollback: options.max_scrollback,
            },
            invalidator,
        )
        .map_err(|e| e.to_string())
    }));
    match result {
        Ok(Ok(terminal)) => Box::into_raw(Box::new(terminal)).cast(),
        failure => {
            let error = match failure {
                Ok(Err(error)) => error,
                _ => "terminal extension panicked during creation".into(),
            };
            unsafe { reply(context, Bytes::new(error.as_bytes())) };
            std::ptr::null_mut()
        }
    }
}

unsafe extern "C" fn destroy(handle: *mut c_void) {
    if !handle.is_null() {
        let _ = catch_unwind(AssertUnwindSafe(|| unsafe {
            drop(Box::from_raw(handle.cast::<Terminal>()))
        }));
    }
}

#[derive(Deserialize)]
struct KeyInput {
    code: u32,
    character: Option<String>,
    key_char: Option<String>,
    text: Option<String>,
    modifiers: u32,
    action: u32,
}
fn key(code: u32, character: Option<String>) -> Key {
    match code {
        0 => Key::Character(character.unwrap_or_default()),
        1 => Key::ArrowUp,
        2 => Key::ArrowDown,
        3 => Key::ArrowLeft,
        4 => Key::ArrowRight,
        5 => Key::PageUp,
        6 => Key::PageDown,
        7 => Key::Home,
        8 => Key::End,
        9 => Key::Enter,
        10 => Key::Escape,
        11 => Key::Space,
        12 => Key::Tab,
        13 => Key::Backspace,
        14 => Key::Delete,
        15 => Key::Insert,
        101..=124 => Key::Function((code - 100) as u8),
        _ => Key::Other,
    }
}
#[derive(Deserialize)]
struct SelectionInput {
    phase: u32,
    column: u16,
    row: u16,
    surface_x: f64,
    surface_y: f64,
    columns: u32,
    cell_width: u32,
    padding_left: u32,
    screen_height: u32,
    time_ms: u64,
    repeat_distance: f64,
    rectangle: bool,
}

unsafe extern "C" fn command(handle: *mut c_void, kind: u32, input: Bytes) -> i32 {
    if handle.is_null() {
        return -1;
    }
    catch_unwind(AssertUnwindSafe(|| -> Option<i32> {
        let data = unsafe { bytes(input, MAX_TERMINAL_INPUT_BYTES) }?;
        let terminal = unsafe { &*handle.cast::<Terminal>() };
        let message = match kind {
            abi::INPUT if !data.is_empty() => WorkerMessage::Input(data.to_vec()),
            abi::PASTE if !data.is_empty() => {
                WorkerMessage::Paste(std::str::from_utf8(data).ok()?.to_owned())
            }
            abi::KEY => {
                let k: KeyInput = serde_json::from_slice(data).ok()?;
                if k.action > 2
                    || k.character
                        .as_ref()
                        .is_some_and(|s| s.len() > MAX_TERMINAL_STRING_BYTES)
                    || k.key_char
                        .as_ref()
                        .is_some_and(|s| s.len() > MAX_TERMINAL_STRING_BYTES)
                {
                    return None;
                }
                WorkerMessage::Key(TerminalKeyInput {
                    key: key(k.code, k.character),
                    key_char: k.key_char.map(Key::Character),
                    text: k.text,
                    modifiers: Modifiers::from_bits_truncate(k.modifiers),
                    action: match k.action {
                        0 => GhosttyKeyAction::Press,
                        1 => GhosttyKeyAction::Release,
                        _ => GhosttyKeyAction::Repeat,
                    },
                })
            }
            abi::RESIZE => {
                let [cols, rows, cell_width_px, cell_height_px]: [u32; 4] =
                    serde_json::from_slice(data).ok()?;
                if !(1..=u32::from(MAX_COLS)).contains(&cols)
                    || !(1..=u32::from(MAX_ROWS)).contains(&rows)
                {
                    return None;
                }
                WorkerMessage::Resize(TerminalSize {
                    cols: cols as u16,
                    rows: rows as u16,
                    cell_width_px,
                    cell_height_px,
                })
            }
            abi::SCROLL => WorkerMessage::Scroll(
                serde_json::from_slice::<isize>(data)
                    .ok()?
                    .clamp(-(MAX_ROWS as isize), MAX_ROWS as isize),
            ),
            abi::SELECTION => {
                let s: SelectionInput = serde_json::from_slice(data).ok()?;
                if s.phase > 3
                    || s.column >= MAX_COLS
                    || s.row >= MAX_ROWS
                    || s.columns == 0
                    || s.columns > u32::from(MAX_COLS)
                    || !s.surface_x.is_finite()
                    || !s.surface_y.is_finite()
                    || !s.repeat_distance.is_finite()
                {
                    return None;
                }
                WorkerMessage::Selection(TerminalSelectionInput {
                    phase: match s.phase {
                        0 => TerminalSelectionPhase::Press,
                        1 => TerminalSelectionPhase::Drag,
                        2 => TerminalSelectionPhase::Release,
                        _ => TerminalSelectionPhase::Cancel,
                    },
                    column: s.column,
                    row: s.row,
                    surface_x: s.surface_x,
                    surface_y: s.surface_y,
                    geometry: GhosttySelectionGeometry {
                        columns: s.columns,
                        cell_width: s.cell_width.max(1),
                        padding_left: s.padding_left,
                        screen_height: s.screen_height,
                    },
                    time: Duration::from_millis(s.time_ms),
                    repeat_distance: s.repeat_distance,
                    rectangle: s.rectangle,
                })
            }
            abi::SELECT_ALL => WorkerMessage::SelectAll,
            abi::THEME => {
                let values: Option<Vec<[f32; 4]>> = serde_json::from_slice(data).ok()?;
                let theme = if let Some(values) = values {
                    if values.len() != 19 || values.iter().flatten().any(|v| !v.is_finite()) {
                        return None;
                    }
                    let color = |i: usize| {
                        let [r, g, b, a] = values[i];
                        Color::linear(r, g, b, a)
                    };
                    Some(Box::new(TerminalTheme {
                        foreground: color(0),
                        background: color(1),
                        cursor: color(2),
                        ansi: std::array::from_fn(|i| color(i + 3)),
                    }))
                } else {
                    None
                };
                WorkerMessage::Theme(theme)
            }
            _ => return None,
        };
        Some(if terminal.try_send(message) { 0 } else { 1 })
    }))
    .ok()
    .flatten()
    .unwrap_or(-1)
}

fn rgba(color: Color) -> Rgba {
    Rgba {
        r: color.r,
        g: color.g,
        b: color.b,
        a: color.a,
    }
}
fn optional_rgba(color: Option<Color>) -> Rgba {
    color.map(rgba).unwrap_or(Rgba {
        a: -1.0,
        ..Rgba::default()
    })
}
fn string(value: &str) -> Bytes {
    Bytes::new(value.as_bytes())
}

unsafe extern "C" fn frame(
    handle: *mut c_void,
    previous: u64,
    context: *mut c_void,
    reply: abi::FrameCallback,
) -> i32 {
    if handle.is_null() {
        return -1;
    }
    catch_unwind(AssertUnwindSafe(|| {
        let snapshot = unsafe { &*handle.cast::<Terminal>() }.snapshot();
        if snapshot.revision == previous {
            return 0;
        }
        if snapshot.content.len() > abi::MAX_FRAME_TEXT
            || snapshot.highlights.len() > abi::MAX_FRAME_HIGHLIGHTS
            || snapshot.graphics.len() > abi::MAX_FRAME_CELLS
            || snapshot
                .selected_text
                .as_ref()
                .is_some_and(|s| s.len() > abi::MAX_FRAME_TEXT)
        {
            return -1;
        }
        let highlights: Vec<_> = snapshot
            .highlights
            .iter()
            .map(|(range, style)| abi::Highlight {
                start: range.start as u32,
                end: range.end as u32,
                flags: style.flags
                    | u32::from(style.color.is_some())
                    | (u32::from(style.background.is_some()) << 1),
                foreground: optional_rgba(style.color),
                background: optional_rgba(style.background),
            })
            .collect();
        let graphics: Vec<_> = snapshot
            .graphics
            .iter()
            .map(|g| abi::Graphic {
                column: g.column,
                row: g.row,
                character: g.character as u32,
                has_foreground: u32::from(g.foreground.is_some()),
                foreground: optional_rgba(g.foreground),
            })
            .collect();
        let edge = &snapshot.edge_backgrounds;
        let edges: Vec<_> = edge
            .top
            .iter()
            .chain(edge.right.iter())
            .chain(edge.bottom.iter())
            .chain(edge.left.iter())
            .copied()
            .map(optional_rgba)
            .collect();
        let cursor = snapshot.cursor;
        let (status, process_id, exit_code, signal, error) = match &snapshot.status {
            TerminalStatus::Starting => (0, 0, 0, "", ""),
            TerminalStatus::Running { process_id } => (1, process_id.unwrap_or(0), 0, "", ""),
            TerminalStatus::Exited { exit_code, signal } => {
                (2, 0, *exit_code, signal.as_deref().unwrap_or(""), "")
            }
            TerminalStatus::Failed { message } => (3, 0, 0, "", message.as_ref()),
        };
        let frame = abi::TerminalFrame {
            revision: snapshot.revision,
            cols: snapshot.cols,
            rows: snapshot.rows,
            foreground: rgba(snapshot.foreground),
            background: rgba(snapshot.background),
            content: string(&snapshot.content),
            title: string(&snapshot.title),
            working_directory: string(&snapshot.working_directory),
            selected_text: string(snapshot.selected_text.as_deref().unwrap_or("")),
            has_selection: u32::from(snapshot.selected_text.is_some()),
            highlights: highlights.as_ptr(),
            highlights_len: highlights.len(),
            graphics: graphics.as_ptr(),
            graphics_len: graphics.len(),
            edges: edges.as_ptr(),
            edges_len: edges.len(),
            total_rows: snapshot.scroll.total_rows,
            offset_rows: snapshot.scroll.offset_rows,
            viewport_rows: snapshot.scroll.viewport_rows,
            has_cursor: u32::from(cursor.is_some()),
            cursor_column: cursor.map_or(0, |c| c.column),
            cursor_row: cursor.map_or(0, |c| c.row),
            cursor_style: cursor.map_or(0, |c| match c.style {
                TerminalCursorStyle::Bar => 0,
                TerminalCursorStyle::Block => 1,
                TerminalCursorStyle::Underline => 2,
                TerminalCursorStyle::BlockHollow => 3,
            }),
            cursor_blinking: u32::from(cursor.is_some_and(|c| c.blinking)),
            cursor_color: cursor.map_or(Rgba::default(), |c| rgba(c.color)),
            cursor_start: snapshot.cursor_range.as_ref().map_or(0, |r| r.start as u32),
            cursor_end: snapshot.cursor_range.as_ref().map_or(0, |r| r.end as u32),
            has_cursor_range: u32::from(snapshot.cursor_range.is_some()),
            status,
            process_id,
            exit_code,
            signal: string(signal),
            error: string(error),
            agent: string(snapshot.agent.as_ref().map_or("", |a| a.kind.as_ref())),
            agent_status: snapshot.agent.as_ref().map_or(0, |a| match a.status {
                TerminalAgentStatus::Idle => 0,
                TerminalAgentStatus::Working => 1,
                TerminalAgentStatus::Blocked => 2,
            }),
            agent_process_id: snapshot.agent.as_ref().map_or(0, |a| a.process_id),
        };
        unsafe { reply(context, &frame) };
        1
    }))
    .unwrap_or(-1)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[derive(Default)]
    struct Notifications {
        wakes: AtomicUsize,
        releases: AtomicUsize,
    }
    unsafe extern "C" fn wake(context: *mut c_void) {
        unsafe { &*context.cast::<Arc<Notifications>>() }
            .wakes
            .fetch_add(1, Ordering::Relaxed);
    }
    unsafe extern "C" fn release(context: *mut c_void) {
        let state = unsafe { Box::from_raw(context.cast::<Arc<Notifications>>()) };
        state.releases.fetch_add(1, Ordering::Relaxed);
    }
    fn callbacks(state: &Arc<Notifications>) -> abi::Wake {
        abi::Wake {
            context: Box::into_raw(Box::new(Arc::clone(state))).cast(),
            wake,
            release,
        }
    }
    unsafe extern "C" fn error(context: *mut c_void, value: Bytes) {
        let value = unsafe { bytes(value, MAX_TERMINAL_INPUT_BYTES) }.unwrap();
        unsafe { *context.cast::<String>() = String::from_utf8_lossy(value).into_owned() };
    }

    #[test]
    fn invalid_creation_releases_the_callback_context_once() {
        let notifications = Arc::new(Notifications::default());
        let mut message = String::new();
        let handle = unsafe {
            create(
                Bytes::new(b"{}"),
                callbacks(&notifications),
                (&mut message as *mut String).cast(),
                error,
            )
        };
        assert!(handle.is_null());
        assert!(!message.is_empty());
        assert_eq!(notifications.releases.load(Ordering::Relaxed), 1);
        assert_eq!(notifications.wakes.load(Ordering::Relaxed), 0);
    }

    #[cfg(unix)]
    #[test]
    fn abi_session_runs_a_pty_and_releases_its_worker() {
        #[derive(Default)]
        struct Screen {
            revision: u64,
            content: String,
            cols: u16,
            rows: u16,
            styles: usize,
        }
        unsafe extern "C" fn screen(context: *mut c_void, frame: *const abi::TerminalFrame) {
            let frame = unsafe { &*frame };
            let content = unsafe { bytes(frame.content, abi::MAX_FRAME_TEXT) }.unwrap();
            unsafe {
                *context.cast::<Screen>() = Screen {
                    revision: frame.revision,
                    content: String::from_utf8_lossy(content).into_owned(),
                    cols: frame.cols,
                    rows: frame.rows,
                    styles: frame.highlights_len,
                }
            };
        }
        let options = serde_json::to_vec(&serde_json::json!({
            "program": "/bin/sh", "arguments": ["-c", "printf '\\033[31mextension-ready\\033[0m\\n'; IFS= read -r line; printf 'reply:%s\\n' \"$line\""],
            "working_directory": null, "environment": [], "cols": 80, "rows": 24, "max_scrollback": 100,
        })).unwrap();
        let notifications = Arc::new(Notifications::default());
        let mut message = String::new();
        let handle = unsafe {
            create(
                Bytes::new(&options),
                callbacks(&notifications),
                (&mut message as *mut String).cast(),
                error,
            )
        };
        assert!(!handle.is_null(), "{message}");
        struct Guard(*mut c_void);
        impl Drop for Guard {
            fn drop(&mut self) {
                unsafe { destroy(self.0) };
            }
        }
        let guard = Guard(handle);
        let mut current = Screen {
            revision: u64::MAX,
            ..Default::default()
        };
        let deadline = Instant::now() + Duration::from_secs(10);
        loop {
            let result = unsafe {
                frame(
                    handle,
                    current.revision,
                    (&mut current as *mut Screen).cast(),
                    screen,
                )
            };
            assert!(result >= 0);
            if current.content.contains("extension-ready") {
                break;
            }
            assert!(
                Instant::now() < deadline,
                "PTY never produced a frame: {}",
                current.content
            );
            std::thread::sleep(Duration::from_millis(10));
        }
        assert!(current.styles > 0, "ANSI styles must survive the ABI");
        // With no new terminal output, a revision read must not invoke the frame callback.
        loop {
            let result = unsafe {
                frame(
                    handle,
                    current.revision,
                    (&mut current as *mut Screen).cast(),
                    screen,
                )
            };
            if result == 0 {
                break;
            }
            assert!(result == 1 && Instant::now() < deadline);
        }
        assert_eq!(
            unsafe { command(handle, abi::RESIZE, Bytes::new(b"[90,30,8,16]")) },
            0
        );
        assert_eq!(
            unsafe { command(handle, abi::INPUT, Bytes::new(b"hello-extension\n")) },
            0
        );
        loop {
            assert!(
                unsafe {
                    frame(
                        handle,
                        current.revision,
                        (&mut current as *mut Screen).cast(),
                        screen,
                    )
                } >= 0
            );
            if current.content.contains("reply:hello-extension")
                && current.cols == 90
                && current.rows == 30
            {
                break;
            }
            assert!(
                Instant::now() < deadline,
                "PTY input/resize did not arrive: {}",
                current.content
            );
            std::thread::sleep(Duration::from_millis(10));
        }
        assert!(notifications.wakes.load(Ordering::Relaxed) > 0);
        drop(guard);
        while notifications.releases.load(Ordering::Relaxed) == 0 {
            assert!(
                Instant::now() < deadline,
                "worker did not release the callback context"
            );
            std::thread::sleep(Duration::from_millis(10));
        }
        assert_eq!(notifications.releases.load(Ordering::Relaxed), 1);
    }
}
