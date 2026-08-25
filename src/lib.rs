//! QuickGUI is a small, damage-driven foundation for native desktop interfaces.
//!
//! It deliberately keeps the hot path narrow: application state is retained,
//! windows sleep while clean, long lists are virtualized, rectangles are
//! instanced in one draw call, and shaped text is cached by stable [`TextId`]s.

mod action;
mod animated_image;
mod background;
mod canvas;
mod color;
mod custom_shader;
mod custom_shader_renderer;
mod element;
mod entity;
mod event;
mod foreground;
mod geometry;
mod global;
mod image;
mod image_renderer;
mod image_resource;
mod keymap;
#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "macos")]
mod macos_application;
#[cfg(target_os = "macos")]
mod macos_menu;
mod menu;
mod metrics;
#[cfg(target_os = "macos")]
mod native_view;
mod paint_order;
mod path;
mod path_renderer;
mod picker;
mod platform;
mod renderer;
mod runtime;
mod scene;
mod scheduler;
mod styled_text;
mod svg;
mod svg_renderer;
mod text_input;
mod tooltip;
mod ui_tree;
mod virtual_list;

pub use action::{Action, ActionListener, AnyAction};
pub use animated_image::{
    AnimatedImage, AnimatedImageFrame, AnimationRepeat, MAX_ANIMATED_IMAGE_BYTES,
    MAX_ANIMATION_FRAMES, MIN_ANIMATION_FRAME_DURATION,
};
pub use background::{BackgroundTaskError, MAX_PENDING_BACKGROUND_TASKS, TaskSpawnError};
pub use canvas::Canvas;
pub use color::Color;
pub use custom_shader::{
    CUSTOM_SHADER_PARAMETER_VECTORS, CustomShader, CustomShaderError,
    MAX_CUSTOM_SHADER_SOURCE_BYTES, ShaderParameters,
};
pub use custom_shader_renderer::{
    MAX_CUSTOM_SHADER_INSTANCES_PER_FRAME, MAX_CUSTOM_SHADER_PIPELINES_PER_WINDOW,
};
#[cfg(target_os = "macos")]
pub use element::native_view;
pub use element::{
    AccessibilityRole, AnchorPlacement, AppRegion, Element, ElementId, ElementStateStyle,
    FocusHandle, GridTrack, IntoElement, MAX_GRID_TRACKS, UserSelect, button, canvas,
    custom_shader, div, form, img, overlay, path, styled_text_area, styled_text_input,
    submit_button, svg, text, text_area, text_input,
};
pub use entity::{
    Entity, EntityId, EventEmitter, MAX_ENTITY_EVENT_DELIVERIES_PER_TURN,
    MAX_ENTITY_EVENTS_PER_CALLBACK, MAX_ENTITY_NOTIFICATIONS_PER_EVENT,
    MAX_ENTITY_SUBSCRIPTIONS_PER_WINDOW, MAX_OBSERVED_ENTITIES_PER_WINDOW,
    MAX_PENDING_ENTITY_EVENTS, Subscription, WeakEntity,
};
pub use event::{
    ContextMenuEvent, DragOrigin, DragStartEvent, DropEvent, DroppedFiles, DroppedText, DroppedUrl,
    Event, EventContext, ExternalDragEndEvent, ExternalDragOperation, ExternalDragPayload,
    ExternalDragText, ExternalDragUrl, ExternalDragUrlError, FileDragPaths, FormField,
    FormSubmitEvent, Key, MAX_DROPPED_FILES, MAX_EXTERNAL_DRAG_FILES, MAX_EXTERNAL_DRAG_PATH_BYTES,
    MAX_EXTERNAL_DRAG_TEXT_BYTES, MAX_EXTERNAL_DRAG_TOTAL_PATH_BYTES, MAX_EXTERNAL_DRAG_URL_BYTES,
    MAX_FORM_FIELDS, MAX_FORM_SUBMISSIONS_PER_EVENT, MAX_VALIDATION_ISSUES,
    MAX_VALIDATION_MESSAGE_BYTES, Modifiers, MouseButton, PointerEvent, PointerPhase,
    ValidationIssue, ValidationReport,
};
pub use foreground::{
    AsyncContextError, AsyncViewContext, AsyncViewUpdate, ForegroundTaskSpawnError,
    ForegroundTimer, MAX_FOREGROUND_POLLS_PER_TURN, MAX_FOREGROUND_TASKS_PER_APPLICATION,
    MAX_FOREGROUND_TASKS_PER_WINDOW, MAX_FOREGROUND_TIMERS_PER_APPLICATION,
    MAX_FOREGROUND_TIMERS_PER_TASK, MAX_FOREGROUND_UPDATES_PER_TASK, Task,
};
pub use geometry::{Insets, Point, Rect, Size, Vector};
pub use global::{
    Global, MAX_APPLICATION_GLOBALS, MAX_GLOBAL_NOTIFICATIONS_PER_EVENT,
    MAX_GLOBAL_OBSERVER_DELIVERIES_PER_TURN, MAX_GLOBAL_SUBSCRIPTIONS_PER_WINDOW,
    MAX_OBSERVED_GLOBALS_PER_WINDOW, MAX_PENDING_GLOBAL_NOTIFICATIONS,
};
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
pub use picker::{
    MAX_PICKER_ITEM_TEXT_BYTES, MAX_PICKER_ITEMS, MAX_PICKER_QUERY_BYTES,
    MAX_PICKER_QUERY_GRAPHEMES, MAX_PICKER_RESULTS, MAX_PICKER_TEXT_BYTES, PickerConfirm,
    PickerError, PickerFirst, PickerItem, PickerLast, PickerMatch, PickerNext, PickerPageDown,
    PickerPageUp, PickerPrevious, PickerState, PickerStyle, picker_key_bindings,
};
pub use platform::{
    MAX_ACTIVE_PLATFORM_DIALOGS, MAX_OPEN_URLS, MAX_OPEN_URLS_TOTAL_BYTES,
    MAX_PENDING_PLATFORM_REQUESTS, MAX_PENDING_SYSTEM_NOTIFICATIONS, MAX_PLATFORM_PATH_BYTES,
    MAX_PLATFORM_REQUESTS_PER_EVENT, MAX_PLATFORM_TEXT_BYTES, MAX_PLATFORM_URL_BYTES,
    MAX_PROMPT_BUTTON_BYTES, MAX_PROMPT_BUTTONS, MAX_SELECTED_PATHS,
    MAX_SELECTED_PATHS_TOTAL_BYTES, MAX_SYSTEM_NOTIFICATION_ACTION_BYTES,
    MAX_SYSTEM_NOTIFICATION_ACTIONS, MAX_SYSTEM_NOTIFICATION_BODY_BYTES,
    MAX_SYSTEM_NOTIFICATION_CATEGORIES, MAX_SYSTEM_NOTIFICATION_TAG_BYTES,
    MAX_SYSTEM_NOTIFICATION_TITLE_BYTES, OpenUrls, PathPromptOptions, PathPromptResponse,
    PlatformError, PlatformResponse, PromptButton, PromptLevel, SavePathOptions, SavePathResponse,
    SystemNotification, SystemNotificationAction, SystemNotificationResponse,
};
pub use runtime::{
    App, AppConfig, AppError, ClickListener, ContextMenuListener, DismissListener, Drag,
    DragListener, DropListener, FormInvalidListener, FormSubmitListener, InputListener,
    MAX_PENDING_WINDOW_COMMANDS, MAX_WINDOW_COMMANDS_PER_EVENT, MAX_WINDOW_LOGICAL_COORDINATE,
    MAX_WINDOW_LOGICAL_DIMENSION, MAX_WINDOW_TITLE_BYTES, PerformanceProfile, PointerListener,
    SubmitListener, TitleBarStyle, View, ViewContext, WindowBounds, WindowCommandError,
    WindowHandle, WindowKind, WindowOptions, WindowState,
};
pub use scene::{
    BoxShadow, CustomShaderPrimitive, FontFamily, ImagePrimitive, MAX_BOX_SHADOW_BLUR_RADIUS,
    MAX_BOX_SHADOW_EXTENT, PathPrimitive, Quad, Scene, ScenePlane, Shadow, SvgPrimitive, TextId,
    TextRun, TextShaping, TextStyle, TextWrap,
};
pub use styled_text::{
    HighlightStyle, MAX_HIGHLIGHT_FONT_FAMILY_BYTES, MAX_TEXT_HIGHLIGHTS, StyledText,
    TextHighlight, TextUnderline, styled_text,
};
pub use svg::{
    MAX_SVG_RASTER_DIMENSION, MAX_SVG_RASTER_PIXELS, MAX_SVG_SOURCE_BYTES, Svg, SvgError,
    SvgTransform,
};
pub use svg_renderer::{MAX_GPU_SVG_CACHE_BYTES, MAX_GPU_SVG_CACHE_ENTRIES};
pub use tooltip::{
    DEFAULT_TOOLTIP_DELAY, MAX_TOOLTIP_CONTENT_NODES, MAX_TOOLTIP_DELAY, MAX_TOOLTIPS_PER_WINDOW,
    Tooltip,
};
pub use ui_tree::MAX_STATIC_TEXT_COPY_BYTES;
pub use virtual_list::{VirtualList, VisibleRows};

/// Run a view with the default application configuration.
pub fn run<V: View>(view: V) -> Result<(), AppError> {
    App::new(view).run()
}
