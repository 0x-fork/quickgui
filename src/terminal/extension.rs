//! Retained-view adapter for the optional backend. The foreign session owns its PTY and worker;
//! only changed, complete frames are copied into the core's immutable rendering snapshot.
use super::*;
use crate::extension_api::{self as abi, Bytes};
use serde_json::json;
use std::ffi::c_void;

#[derive(Clone, Copy)]
pub(super) enum GhosttyKeyAction {
    Press,
    Release,
    Repeat,
}
#[derive(Clone, Copy, Debug)]
pub(super) struct GhosttySelectionGeometry {
    pub columns: u32,
    pub cell_width: u32,
    pub padding_left: u32,
    pub screen_height: u32,
}

pub(super) struct Session {
    api: abi::TerminalApi,
    handle: usize,
    cached: Mutex<FrameCache>,
}

struct FrameCache {
    initialized: bool,
    failed: bool,
    snapshot: Arc<TerminalSnapshot>,
}

impl Terminal {
    pub fn spawn(
        options: TerminalOptions,
        invalidator: WindowInvalidator,
    ) -> Result<Self, TerminalError> {
        options.validate()?;
        let size = TerminalSize {
            cols: options.cols,
            rows: options.rows,
            cell_width_px: 0,
            cell_height_px: 0,
        };
        let session = Session::new(options, invalidator)?;
        Ok(Self {
            inner: Arc::new(TerminalInner {
                session,
                last_size: AtomicU64::new(pack_size(size)),
                viewport_height_bits: AtomicU32::new(0),
                viewport_bounds: Mutex::new(Rect::ZERO),
                selection_epoch: Instant::now(),
                wheel_remainder: Mutex::new(0.0),
                scrollbar_drag_remainder: Mutex::new(0.0),
                scrollbar_interaction: Mutex::new(TerminalScrollbarInteraction::default()),
                cursor_blink: Mutex::new(TerminalCursorBlink::new(Instant::now())),
                theme: Mutex::new(None),
            }),
        })
    }
}

unsafe extern "C" fn wake(context: *mut c_void) {
    let invalidator = unsafe { &*context.cast::<WindowInvalidator>() };
    let _ = invalidator.invalidate();
}
unsafe extern "C" fn release(context: *mut c_void) {
    unsafe { drop(Box::from_raw(context.cast::<WindowInvalidator>())) };
}
unsafe extern "C" fn error_reply(context: *mut c_void, value: Bytes) {
    if let Some(bytes) = unsafe { borrow(value.data, value.len, MAX_TERMINAL_INPUT_BYTES) } {
        unsafe { *context.cast::<String>() = String::from_utf8_lossy(bytes).into_owned() };
    }
}

impl Session {
    fn new(
        options: TerminalOptions,
        invalidator: WindowInvalidator,
    ) -> Result<Self, TerminalError> {
        let api = *crate::extensions::terminal().ok_or_else(|| {
            TerminalError::Worker(
                "terminal extension is not loaded; import github.com/egoist/quickgui/go/terminal and rebuild the application".into(),
            )
        })?;
        let data = serde_json::to_vec(&json!({
            "program":options.program.as_ref().map(|s|s.to_string_lossy()),
            "arguments":options.arguments.iter().map(|s|s.to_string_lossy()).collect::<Vec<_>>(),
            "working_directory":options.working_directory.as_ref().map(|s|s.to_string_lossy()),
            "environment":options.environment.iter().map(|(k,v)|(k.to_string_lossy(),v.to_string_lossy())).collect::<Vec<_>>(),
            "cols":options.cols,"rows":options.rows,"max_scrollback":options.max_scrollback,
        }))
        .map_err(|e| TerminalError::Worker(e.to_string()))?;
        let callback = abi::Wake {
            context: Box::into_raw(Box::new(invalidator)).cast(),
            wake,
            release,
        };
        let mut error = String::new();
        // create owns the callback context even when it reports an error.
        let handle = unsafe {
            (api.create)(
                Bytes::new(&data),
                callback,
                (&mut error as *mut String).cast(),
                error_reply,
            )
        };
        if handle.is_null() {
            return Err(TerminalError::Worker(if error.is_empty() {
                "terminal extension failed to create a session".into()
            } else {
                error
            }));
        }
        Ok(Self {
            api,
            handle: handle as usize,
            cached: Mutex::new(FrameCache {
                initialized: false,
                failed: false,
                snapshot: Arc::new(TerminalSnapshot::starting(options.cols, options.rows)),
            }),
        })
    }

    pub(super) fn snapshot(&self) -> Arc<TerminalSnapshot> {
        let mut cache = self.cached.lock().unwrap_or_else(|p| p.into_inner());
        if cache.failed {
            return Arc::clone(&cache.snapshot);
        }
        let previous = if cache.initialized {
            cache.snapshot.revision
        } else {
            u64::MAX
        };
        let mut next: Option<TerminalSnapshot> = None;
        let status = unsafe {
            (self.api.frame)(
                self.handle as *mut c_void,
                previous,
                (&mut next as *mut Option<TerminalSnapshot>).cast(),
                frame_reply,
            )
        };
        match (status, next) {
            (0, None) if cache.initialized => {}
            (1, Some(next)) => {
                cache.initialized = true;
                cache.snapshot = Arc::new(next);
            }
            _ => {
                let mut failed = (*cache.snapshot).clone();
                failed.revision = failed.revision.wrapping_add(1);
                failed.status = TerminalStatus::Failed {
                    message: Arc::from("terminal extension returned an invalid or oversized frame"),
                };
                cache.snapshot = Arc::new(failed);
                cache.failed = true;
            }
        }
        Arc::clone(&cache.snapshot)
    }

    pub(super) fn command(&self, message: WorkerMessage) -> bool {
        let (kind, data) = match message {
            WorkerMessage::Input(bytes) => (abi::INPUT, bytes),
            WorkerMessage::Paste(text) => (abi::PASTE, text.into_bytes()),
            other => {
                let (kind, value) = match other {
                    WorkerMessage::Key(k) => {
                        let (code, character) = match &k.key {
                            Key::Character(s) => (0, Some(s.as_str())),
                            Key::ArrowUp => (1, None),
                            Key::ArrowDown => (2, None),
                            Key::ArrowLeft => (3, None),
                            Key::ArrowRight => (4, None),
                            Key::PageUp => (5, None),
                            Key::PageDown => (6, None),
                            Key::Home => (7, None),
                            Key::End => (8, None),
                            Key::Enter => (9, None),
                            Key::Escape => (10, None),
                            Key::Space => (11, None),
                            Key::Tab => (12, None),
                            Key::Backspace => (13, None),
                            Key::Delete => (14, None),
                            Key::Insert => (15, None),
                            Key::Function(n) => (100 + u32::from(*n), None),
                            Key::Other => (99, None),
                        };
                        let mods = u32::from(k.modifiers.contains(Modifiers::SHIFT))
                            | (u32::from(k.modifiers.contains(Modifiers::CONTROL)) << 1)
                            | (u32::from(k.modifiers.contains(Modifiers::ALT)) << 2)
                            | (u32::from(k.modifiers.contains(Modifiers::SUPER)) << 3);
                        (
                            abi::KEY,
                            json!({"code":code,"character":character,"key_char":k.key_char.as_ref().and_then(key_character),"text":k.text,"modifiers":mods,
                            "action":match k.action {GhosttyKeyAction::Press=>0,GhosttyKeyAction::Release=>1,GhosttyKeyAction::Repeat=>2}}),
                        )
                    }
                    WorkerMessage::Resize(s) => (
                        abi::RESIZE,
                        json!([
                            s.cols as u32,
                            s.rows as u32,
                            s.cell_width_px,
                            s.cell_height_px
                        ]),
                    ),
                    WorkerMessage::Scroll(rows) => (abi::SCROLL, json!(rows)),
                    WorkerMessage::SelectAll => (abi::SELECT_ALL, json!(null)),
                    WorkerMessage::Selection(s) => (
                        abi::SELECTION,
                        json!({
                            "phase":match s.phase{TerminalSelectionPhase::Press=>0,TerminalSelectionPhase::Drag=>1,TerminalSelectionPhase::Release=>2,TerminalSelectionPhase::Cancel=>3},
                            "column":s.column,"row":s.row,"surface_x":s.surface_x,"surface_y":s.surface_y,"columns":s.geometry.columns,"cell_width":s.geometry.cell_width,
                            "padding_left":s.geometry.padding_left,"screen_height":s.geometry.screen_height,"time_ms":s.time.as_millis().min(u64::MAX as u128) as u64,"repeat_distance":s.repeat_distance,"rectangle":s.rectangle,
                        }),
                    ),
                    WorkerMessage::Theme(theme) => (
                        abi::THEME,
                        json!(theme.map(|t| {
                            [t.foreground, t.background, t.cursor]
                                .into_iter()
                                .chain(t.ansi)
                                .map(|c| c.as_array())
                                .collect::<Vec<_>>()
                        })),
                    ),
                    _ => unreachable!(),
                };
                let Ok(data) = serde_json::to_vec(&value) else {
                    return false;
                };
                (kind, data)
            }
        };
        unsafe { (self.api.command)(self.handle as *mut c_void, kind, Bytes::new(&data)) == 0 }
    }
}
impl Drop for Session {
    fn drop(&mut self) {
        unsafe { (self.api.destroy)(self.handle as *mut c_void) };
    }
}

unsafe fn borrow<'a, T>(pointer: *const T, len: usize, limit: usize) -> Option<&'a [T]> {
    if len > limit || (len != 0 && pointer.is_null()) {
        return None;
    }
    Some(if len == 0 {
        &[]
    } else {
        unsafe { std::slice::from_raw_parts(pointer, len) }
    })
}
unsafe fn text(value: Bytes) -> Option<Arc<str>> {
    Some(Arc::from(
        std::str::from_utf8(unsafe { borrow(value.data, value.len, abi::MAX_FRAME_TEXT) }?).ok()?,
    ))
}
fn color(c: abi::Rgba) -> Color {
    Color::linear(c.r, c.g, c.b, c.a)
}
unsafe extern "C" fn frame_reply(context: *mut c_void, frame: *const abi::TerminalFrame) {
    if let Some(frame) = unsafe { frame.as_ref() } {
        unsafe { *context.cast::<Option<TerminalSnapshot>>() = decode_frame(frame) };
    }
}
unsafe fn decode_frame(f: &abi::TerminalFrame) -> Option<TerminalSnapshot> {
    if !(1..=MAX_COLS).contains(&f.cols)
        || !(1..=MAX_ROWS).contains(&f.rows)
        || f.edges_len != 2 * (usize::from(f.cols) + usize::from(f.rows))
    {
        return None;
    }
    let content = unsafe { text(f.content) }?;
    let valid_range = |start: u32, end: u32| {
        start <= end
            && content.is_char_boundary(start as usize)
            && content.is_char_boundary(end as usize)
    };
    let mut highlights = Vec::with_capacity(f.highlights_len.min(abi::MAX_FRAME_HIGHLIGHTS));
    for h in unsafe { borrow(f.highlights, f.highlights_len, abi::MAX_FRAME_HIGHLIGHTS) }? {
        if !valid_range(h.start, h.end) {
            return None;
        }
        let mut style = HighlightStyle::default();
        if h.flags & 1 != 0 {
            style = style.color(color(h.foreground));
        }
        if h.flags & 2 != 0 {
            style = style.background(color(h.background));
        }
        if h.flags & 4 != 0 {
            style = style.font_bold();
        }
        if h.flags & 8 != 0 {
            style = style.italic();
        }
        if h.flags & 16 != 0 {
            style = style.underline();
        }
        if h.flags & 32 != 0 {
            style = style.double_underline();
        }
        if h.flags & 64 != 0 {
            style = style.text_decoration_wavy();
        }
        if h.flags & 128 != 0 {
            style = style.strikethrough();
        }
        highlights.push((h.start as usize..h.end as usize, style));
    }
    let mut graphics = Vec::with_capacity(f.graphics_len.min(abi::MAX_FRAME_CELLS));
    for g in unsafe { borrow(f.graphics, f.graphics_len, abi::MAX_FRAME_CELLS) }? {
        if g.column >= f.cols || g.row >= f.rows {
            return None;
        }
        graphics.push(TerminalCellGraphic {
            column: g.column,
            row: g.row,
            character: char::from_u32(g.character)?,
            foreground: (g.has_foreground != 0).then(|| color(g.foreground)),
        });
    }
    let edges = unsafe {
        borrow(
            f.edges,
            f.edges_len,
            2 * (MAX_COLS as usize + MAX_ROWS as usize),
        )
    }?;
    let edge = |slice: &[abi::Rgba]| {
        slice
            .iter()
            .map(|c| (c.a >= 0.0).then(|| color(*c)))
            .collect()
    };
    let c = f.cols as usize;
    let r = f.rows as usize;
    let cursor = if f.has_cursor != 0 {
        if f.cursor_column >= f.cols || f.cursor_row >= f.rows {
            return None;
        }
        Some(TerminalCursor {
            column: f.cursor_column,
            row: f.cursor_row,
            style: match f.cursor_style {
                0 => TerminalCursorStyle::Bar,
                1 => TerminalCursorStyle::Block,
                2 => TerminalCursorStyle::Underline,
                3 => TerminalCursorStyle::BlockHollow,
                _ => return None,
            },
            blinking: f.cursor_blinking != 0,
            color: color(f.cursor_color),
        })
    } else {
        None
    };
    let cursor_range = if f.has_cursor_range != 0 {
        if !valid_range(f.cursor_start, f.cursor_end) {
            return None;
        };
        Some(f.cursor_start as usize..f.cursor_end as usize)
    } else {
        None
    };
    let status = match f.status {
        0 => TerminalStatus::Starting,
        1 => TerminalStatus::Running {
            process_id: (f.process_id != 0).then_some(f.process_id),
        },
        2 => TerminalStatus::Exited {
            exit_code: f.exit_code,
            signal: if f.signal.len != 0 {
                Some(unsafe { text(f.signal) }?)
            } else {
                None
            },
        },
        3 => TerminalStatus::Failed {
            message: unsafe { text(f.error) }?,
        },
        _ => return None,
    };
    let agent = if f.agent.len != 0 {
        Some(TerminalAgent {
            kind: unsafe { text(f.agent) }?,
            status: match f.agent_status {
                0 => TerminalAgentStatus::Idle,
                1 => TerminalAgentStatus::Working,
                2 => TerminalAgentStatus::Blocked,
                _ => return None,
            },
            process_id: f.agent_process_id,
        })
    } else {
        None
    };
    Some(TerminalSnapshot {
        revision: f.revision,
        cols: f.cols,
        rows: f.rows,
        content,
        highlights: highlights.into(),
        foreground: color(f.foreground),
        background: color(f.background),
        title: unsafe { text(f.title) }?,
        working_directory: unsafe { text(f.working_directory) }?,
        scroll: TerminalScrollState {
            total_rows: f.total_rows,
            offset_rows: f.offset_rows,
            viewport_rows: f.viewport_rows,
        },
        cursor,
        selected_text: if f.has_selection != 0 {
            Some(unsafe { text(f.selected_text) }?)
        } else {
            None
        },
        cursor_range,
        graphics: graphics.into(),
        edge_backgrounds: EdgeBackgrounds::new(
            edge(&edges[..c]),
            edge(&edges[c..c + r]),
            edge(&edges[c + r..2 * c + r]),
            edge(&edges[2 * c + r..]),
        ),
        status,
        agent,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    struct Fake {
        calls: AtomicUsize,
        invalid: bool,
    }
    unsafe extern "C" fn create(
        _: Bytes,
        _: abi::Wake,
        _: *mut c_void,
        _: abi::Reply,
    ) -> *mut c_void {
        unreachable!()
    }
    unsafe extern "C" fn destroy(handle: *mut c_void) {
        unsafe { drop(Box::from_raw(handle.cast::<Fake>())) };
    }
    unsafe extern "C" fn command(_: *mut c_void, _: u32, _: Bytes) -> i32 {
        0
    }
    unsafe extern "C" fn frame(
        handle: *mut c_void,
        previous: u64,
        context: *mut c_void,
        reply: abi::FrameCallback,
    ) -> i32 {
        let state = unsafe { &*handle.cast::<Fake>() };
        state.calls.fetch_add(1, Ordering::Relaxed);
        if state.invalid {
            return -1;
        }
        if previous == 7 {
            return 0;
        }
        // This ABI structure consists entirely of integers, floats, and nullable spans.
        let mut frame: abi::TerminalFrame = unsafe { std::mem::zeroed() };
        let edges = [abi::Rgba::default(); 4];
        frame.cols = 1;
        frame.rows = 1;
        frame.revision = 7;
        frame.edges = edges.as_ptr();
        frame.edges_len = edges.len();
        frame.content = Bytes::new("é".as_bytes());
        frame.status = 1;
        unsafe { reply(context, &frame) };
        1
    }
    fn session(invalid: bool) -> Session {
        Session {
            api: abi::TerminalApi {
                create,
                destroy,
                command,
                frame,
            },
            handle: Box::into_raw(Box::new(Fake {
                calls: AtomicUsize::new(0),
                invalid,
            })) as usize,
            cached: Mutex::new(FrameCache {
                initialized: false,
                failed: false,
                snapshot: Arc::new(TerminalSnapshot::starting(1, 1)),
            }),
        }
    }

    #[test]
    fn unchanged_frames_reuse_the_entire_immutable_snapshot() {
        let session = session(false);
        let first = session.snapshot();
        assert_eq!(&*first.content, "é");
        assert_eq!(first.revision, 7);
        for _ in 0..100 {
            assert!(Arc::ptr_eq(&first, &session.snapshot()));
        }
    }

    #[test]
    fn a_failed_frame_surfaces_status_and_stops_polling() {
        let session = session(true);
        let first = session.snapshot();
        assert!(matches!(first.status, TerminalStatus::Failed { .. }));
        assert!(Arc::ptr_eq(&first, &session.snapshot()));
        let state = unsafe { &*(session.handle as *const Fake) };
        assert_eq!(state.calls.load(Ordering::Relaxed), 1);
    }

    #[test]
    fn decoder_rejects_oversized_spans_and_non_character_boundaries() {
        let mut frame: abi::TerminalFrame = unsafe { std::mem::zeroed() };
        let edges = [abi::Rgba::default(); 4];
        frame.cols = 1;
        frame.rows = 1;
        frame.edges = edges.as_ptr();
        frame.edges_len = 4;
        frame.content = Bytes::new("é".as_bytes());
        let mut highlight = abi::Highlight {
            start: 0,
            end: 1,
            ..Default::default()
        };
        frame.highlights = &highlight;
        frame.highlights_len = 1;
        assert!(unsafe { decode_frame(&frame) }.is_none());
        highlight.end = 2;
        frame.highlights = &highlight;
        assert!(unsafe { decode_frame(&frame) }.is_some());
        frame.content.len = abi::MAX_FRAME_TEXT + 1;
        assert!(unsafe { decode_frame(&frame) }.is_none());
    }
}
