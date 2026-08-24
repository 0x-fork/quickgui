//! QuickGUI is a small, damage-driven foundation for native desktop interfaces.
//!
//! It deliberately keeps the hot path narrow: application state is retained,
//! windows sleep while clean, long lists are virtualized, rectangles are
//! instanced in one draw call, and shaped text is cached by stable [`TextId`]s.

mod color;
mod element;
mod event;
mod geometry;
#[cfg(target_os = "macos")]
mod macos;
mod metrics;
#[cfg(target_os = "macos")]
mod native_view;
mod renderer;
mod runtime;
mod scene;
mod scheduler;
mod text_input;
mod ui_tree;
mod virtual_list;

pub use color::Color;
#[cfg(target_os = "macos")]
pub use element::native_view;
pub use element::{
    AccessibilityRole, AnchorPlacement, Element, ElementId, ElementStateStyle, FocusHandle,
    IntoElement, button, div, overlay, text, text_input,
};
pub use event::{Event, EventContext, Key, Modifiers, MouseButton};
pub use geometry::{Insets, Point, Rect, Size, Vector};
pub use glyphon::Weight as FontWeight;
pub use metrics::{FrameMetrics, RenderStats};
#[cfg(target_os = "macos")]
pub use native_view::MacNativeView;
pub use runtime::{
    App, AppConfig, AppError, ClickListener, DismissListener, InputListener, PerformanceProfile,
    View, ViewContext,
};
pub use scene::{FontFamily, Quad, Scene, ScenePlane, TextId, TextRun, TextStyle, TextWrap};
pub use virtual_list::{VirtualList, VisibleRows};

/// Run a view with the default application configuration.
pub fn run<V: View>(view: V) -> Result<(), AppError> {
    App::new(view).run()
}
