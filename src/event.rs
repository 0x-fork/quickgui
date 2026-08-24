use bitflags::bitflags;

use crate::{Point, Size, Vector};

/// Framework-level input and window events, expressed in logical pixels.
#[derive(Clone, Debug, PartialEq)]
pub enum Event {
    PointerMoved(Point),
    PointerLeft,
    MouseButton {
        button: MouseButton,
        pressed: bool,
    },
    Click(crate::ElementId),
    /// A coalesced scroll delta. Multiple platform wheel events may become one event.
    Scroll(Vector),
    KeyDown {
        key: Key,
        modifiers: Modifiers,
        repeat: bool,
    },
    KeyUp {
        key: Key,
        modifiers: Modifiers,
    },
    /// Committed text from the platform input method.
    TextInput(String),
    ModifiersChanged(Modifiers),
    Focused(bool),
    Resized {
        logical_size: Size,
        scale_factor: f32,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MouseButton {
    Left,
    Right,
    Middle,
    Back,
    Forward,
    Other(u16),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Key {
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
    Other,
}

bitflags! {
    #[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
    pub struct Modifiers: u8 {
        const SHIFT = 1 << 0;
        const CONTROL = 1 << 1;
        const ALT = 1 << 2;
        const SUPER = 1 << 3;
    }
}

/// Commands emitted while a [`crate::View`] handles an event.
#[derive(Debug, Default)]
pub struct EventContext {
    pub(crate) invalidate: bool,
    pub(crate) exit: bool,
}

impl EventContext {
    /// Mark the window dirty. Calls are coalesced into a single redraw.
    pub fn invalidate(&mut self) {
        self.invalidate = true;
    }

    /// Ask the application event loop to exit cleanly.
    pub fn exit(&mut self) {
        self.exit = true;
    }
}
