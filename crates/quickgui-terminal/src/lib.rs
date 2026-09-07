//! Optional terminal backend. Shared backend sources deliberately do not depend on quickgui,
//! WGPU, Taffy, or the application runtime. Rendering stays in the host's retained view adapter.
#![allow(dead_code, unused_imports)]

#[path = "../../../src/color.rs"]
mod color;
#[path = "../../../src/extension_api.rs"]
mod extension_api;
#[path = "../../../src/terminal.rs"]
mod terminal;
#[path = "../../../src/terminal_process.rs"]
mod terminal_process;

use color::Color;
use extension_api::Wake;
use std::sync::Arc;

const MAX_TEXT_HIGHLIGHTS: usize = extension_api::MAX_FRAME_HIGHLIGHTS;

// Screen-run data used by the backend; actual font/layout types remain in the renderer.
#[derive(Clone, Debug, Default, PartialEq)]
struct HighlightStyle {
    color: Option<Color>,
    background: Option<Color>,
    flags: u32,
}
impl HighlightStyle {
    fn color(mut self, color: Color) -> Self {
        self.color = Some(color);
        self
    }
    fn background(mut self, color: Color) -> Self {
        self.background = Some(color);
        self
    }
    fn font_bold(mut self) -> Self {
        self.flags |= 4;
        self
    }
    fn italic(mut self) -> Self {
        self.flags |= 8;
        self
    }
    fn underline(mut self) -> Self {
        self.flags |= 16;
        self
    }
    fn double_underline(mut self) -> Self {
        self.flags |= 32;
        self
    }
    fn text_decoration_wavy(mut self) -> Self {
        self.flags |= 64;
        self
    }
    fn strikethrough(mut self) -> Self {
        self.flags |= 128;
        self
    }
}

#[derive(Clone)]
enum Key {
    Character(String),
    ArrowUp,
    ArrowDown,
    ArrowLeft,
    ArrowRight,
    PageUp,
    PageDown,
    Home,
    End,
    Enter,
    Escape,
    Space,
    Tab,
    Backspace,
    Delete,
    Insert,
    Function(u8),
    Other,
}
bitflags::bitflags! {
    #[derive(Clone, Copy)]
    struct Modifiers: u32 { const SHIFT = 1; const CONTROL = 2; const ALT = 4; const SUPER = 8; }
}

struct WakeOwner(Wake);
// SAFETY: the ABI requires a callback safe to invoke from a worker, with a context that remains
// owned by this guard until the final worker reference goes away.
unsafe impl Send for WakeOwner {}
unsafe impl Sync for WakeOwner {}
impl Drop for WakeOwner {
    fn drop(&mut self) {
        unsafe { (self.0.release)(self.0.context) };
    }
}
#[derive(Clone)]
struct WindowInvalidator(Arc<WakeOwner>);
impl WindowInvalidator {
    fn invalidate(&self) -> bool {
        unsafe { (self.0.0.wake)(self.0.0.context) };
        true
    }
}

fn static_selection_color() -> Color {
    Color::rgba8(48, 120, 196, 105)
}

/// # Safety
/// The returned descriptor is immutable and valid for the lifetime of the loaded image.
#[unsafe(no_mangle)]
pub extern "C" fn quickgui_extension_v1() -> *const extension_api::Extension {
    terminal::extension_backend::descriptor()
}
