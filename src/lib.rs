//! QuickGUI is a small, damage-driven foundation for native desktop interfaces.
//!
//! It deliberately keeps the hot path narrow: application state is retained,
//! windows sleep while clean, long lists are virtualized, rectangles are
//! instanced in one draw call, and shaped text is cached by stable [`TextId`]s.

mod color;
mod element;
mod event;
mod geometry;
mod metrics;
mod renderer;
mod runtime;
mod scene;
mod scheduler;
mod text_input;
mod ui_tree;
mod virtual_list;

pub use color::Color;
pub use element::{
    AccessibilityRole, Element, ElementId, ElementStateStyle, FocusHandle, IntoElement, button,
    div, text, text_input,
};
pub use event::{Event, EventContext, Key, Modifiers, MouseButton};
pub use geometry::{Insets, Point, Rect, Size, Vector};
pub use glyphon::Weight as FontWeight;
pub use metrics::{FrameMetrics, RenderStats};
pub use runtime::{
    App, AppConfig, AppError, ClickListener, InputListener, PerformanceProfile, View, ViewContext,
};
pub use scene::{FontFamily, Quad, Scene, TextId, TextRun, TextStyle, TextWrap};
pub use virtual_list::{VirtualList, VisibleRows};

/// Run a view with the default application configuration.
pub fn run<V: View>(view: V) -> Result<(), AppError> {
    App::new(view).run()
}
