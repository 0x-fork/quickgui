//! QuickGUI is a small, damage-driven foundation for native desktop interfaces.
//!
//! It deliberately keeps the hot path narrow: application state is retained,
//! windows sleep while clean, long lists are virtualized, rectangles are
//! instanced in one draw call, and shaped text is cached by stable [`TextId`]s.

mod action;
mod animated_image;
mod canvas;
mod color;
mod element;
mod event;
mod geometry;
mod image;
mod image_renderer;
mod image_resource;
mod keymap;
#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "macos")]
mod macos_menu;
mod menu;
mod metrics;
#[cfg(target_os = "macos")]
mod native_view;
mod paint_order;
mod path;
mod path_renderer;
mod renderer;
mod runtime;
mod scene;
mod scheduler;
mod svg;
mod svg_renderer;
mod text_input;
mod ui_tree;
mod virtual_list;

pub use action::{Action, ActionListener, AnyAction};
pub use animated_image::{
    AnimatedImage, AnimatedImageFrame, AnimationRepeat, MAX_ANIMATED_IMAGE_BYTES,
    MAX_ANIMATION_FRAMES, MIN_ANIMATION_FRAME_DURATION,
};
pub use canvas::Canvas;
pub use color::Color;
#[cfg(target_os = "macos")]
pub use element::native_view;
pub use element::{
    AccessibilityRole, AnchorPlacement, Element, ElementId, ElementStateStyle, FocusHandle,
    IntoElement, button, canvas, div, img, overlay, path, svg, text, text_input,
};
pub use event::{Event, EventContext, Key, Modifiers, MouseButton};
pub use geometry::{Insets, Point, Rect, Size, Vector};
pub use glyphon::Weight as FontWeight;
pub use image::{
    Image, ImageError, ImageResource, ImageSource, MAX_DECODED_IMAGE_BYTES,
    MAX_ENCODED_IMAGE_BYTES, MAX_IMAGE_DIMENSION, ObjectFit,
};
pub use image_renderer::{MAX_GPU_IMAGE_CACHE_BYTES, MAX_GPU_IMAGE_CACHE_ENTRIES};
pub use image_resource::{
    IMAGE_LOADING_DELAY, ImageResourceStats, MAX_CPU_IMAGE_CACHE_BYTES,
    MAX_IMAGE_RESOURCE_CACHE_ENTRIES, MAX_PENDING_IMAGE_LOADS,
};
pub use keymap::{
    ContextPredicate, KeyBinding, KeyContext, Keymap, KeymapError, KeymapMatch, Keystroke,
};
pub use menu::{Menu, MenuItem, OsAction, OsMenu, SystemMenuType};
pub use metrics::{FrameMetrics, RenderStats};
#[cfg(target_os = "macos")]
pub use native_view::MacNativeView;
pub use path::{
    Background, FillOptions, FillRule, GradientColorSpace, LineCap, LineJoin, LinearColorStop,
    LinearGradient, MAX_PATH_BYTES, MAX_PATH_COMMANDS, MAX_PATH_COORDINATE, MAX_PATH_DASH_SEGMENTS,
    MAX_PATH_VERTICES, Path, PathBuilder, PathError, PathStyle, StrokeOptions, linear_color_stop,
    linear_gradient,
};
pub use path_renderer::{MAX_GPU_PATH_VERTICES, MAX_GPU_PATHS_PER_FRAME};
pub use runtime::{
    App, AppConfig, AppError, ClickListener, DismissListener, InputListener, PerformanceProfile,
    View, ViewContext,
};
pub use scene::{
    BoxShadow, FontFamily, ImagePrimitive, MAX_BOX_SHADOW_BLUR_RADIUS, MAX_BOX_SHADOW_EXTENT,
    PathPrimitive, Quad, Scene, ScenePlane, Shadow, SvgPrimitive, TextId, TextRun, TextStyle,
    TextWrap,
};
pub use svg::{
    MAX_SVG_RASTER_DIMENSION, MAX_SVG_RASTER_PIXELS, MAX_SVG_SOURCE_BYTES, Svg, SvgError,
    SvgTransform,
};
pub use svg_renderer::{MAX_GPU_SVG_CACHE_BYTES, MAX_GPU_SVG_CACHE_ENTRIES};
pub use virtual_list::{VirtualList, VisibleRows};

/// Run a view with the default application configuration.
pub fn run<V: View>(view: V) -> Result<(), AppError> {
    App::new(view).run()
}
