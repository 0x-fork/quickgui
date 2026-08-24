use bitflags::bitflags;

use crate::{Action, AnyAction, ElementId, FocusHandle, Menu, Point, Size, Vector};

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
    /// A top-level surface was dismissed by Escape or an outside pointer press.
    Dismiss(crate::ElementId),
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
    /// The focused element changed within the window.
    FocusChanged(Option<ElementId>),
    /// The native window itself gained or lost focus.
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

/// The stage of a captured pointer interaction.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PointerPhase {
    Down,
    Move,
    Up,
    /// The window lost focus before the pressed button was released.
    Cancel,
}

/// A pointer event delivered to an element that owns pointer capture.
///
/// Positions and deltas use logical pixels. Once an element receives [`PointerPhase::Down`], it
/// continues to receive move events and the terminal up or cancel event even when the pointer is
/// outside its bounds.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PointerEvent {
    pub phase: PointerPhase,
    pub position: Point,
    /// Position at which this capture started.
    pub origin: Point,
    /// Motion since the preceding captured event.
    pub delta: Vector,
    pub button: MouseButton,
    pub modifiers: Modifiers,
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
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
    Insert,
    Function(u8),
    Other,
}

bitflags! {
    #[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
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
    pub(crate) focus: Option<Option<ElementId>>,
    pub(crate) actions: Vec<AnyAction>,
    pub(crate) menus: Option<Vec<Menu>>,
    pub(crate) propagate_action: bool,
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

    /// Move keyboard focus to a stable element handle.
    pub fn focus(&mut self, handle: FocusHandle) {
        self.focus = Some(Some(handle.id()));
    }

    /// Clear keyboard focus within the window.
    pub fn blur(&mut self) {
        self.focus = Some(None);
    }

    /// Dispatch a typed action through the currently focused element path.
    ///
    /// This is useful for buttons, menus, command palettes, and native menu items that should use
    /// exactly the same command handlers as keyboard bindings.
    pub fn dispatch_action<A: Action>(&mut self, action: A) {
        self.actions.push(AnyAction::new(action));
    }

    /// Replace the application's native menu declaration.
    ///
    /// Use this after state changes that affect labels, checked state, or static availability.
    /// Focused action-handler availability and contextual key equivalents update automatically.
    pub fn set_menus(&mut self, menus: impl IntoIterator<Item = Menu>) {
        self.menus = Some(menus.into_iter().collect());
    }

    /// Allow the current action to continue bubbling to the next ancestor handler.
    ///
    /// Action handlers consume by default, matching GPUI's command dispatch behavior.
    pub fn propagate(&mut self) {
        self.propagate_action = true;
    }
}
