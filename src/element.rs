use std::{
    any::{Any, TypeId},
    borrow::Cow,
    fmt,
    rc::Rc,
    sync::Arc,
};

use glyphon::{Style as GlyphStyle, Weight};
use taffy::{
    Style,
    geometry::{Line as TaffyLine, Point as TaffyPoint, Rect as TaffyRect, Size as TaffySize},
    prelude::{
        AlignContent, AlignItems, AlignSelf, Dimension, Display, FlexDirection, FlexWrap,
        GridAutoFlow, GridPlacement, GridTemplateComponent, JustifyContent, LengthPercentage,
        LengthPercentageAuto, Position, TrackSizingFunction,
    },
    style::Overflow,
    style_helpers::{
        auto, fit_content, flex, fr, length, line, max_content, min_content, minmax, percent,
        repeat,
    },
};

use crate::{
    AnimatedImage, Background, BoxShadow, Canvas, Color, CursorStyle, CustomShader, DispatchPhase,
    Font, FontFallbacks, FontFamily, FontFeatures, Image, ImageSource, KeyContext,
    MAX_VALIDATION_MESSAGE_BYTES, ObjectFit, Path, Rect, ScenePlane, ShaderParameters, StyledText,
    Svg, SvgTransform, TextAlign, TextHighlight, TextOverflow, TextShaping, TextStyle,
    TextUnderline, TextWrap, Tooltip, Transition, WhiteSpace,
    action::{ActionListenerBinding, MAX_ACTION_LISTENERS_PER_ELEMENT},
    animation::ElementAnimation,
    font::{assert_valid_font_family, normalize_fallbacks},
    spring::ElementSpring,
    virtual_list::{
        ListItemMeasurement, ListState, VirtualList, VirtualScrollHandle, VirtualScrollMount,
    },
};

#[cfg(target_os = "macos")]
use crate::native_view::MacNativeView;
#[cfg(target_os = "macos")]
use objc2_app_kit::NSView;

const SPACING_UNIT: f32 = 4.0;
const DEFAULT_ANCHOR_GAP: f32 = 8.0;
const DEFAULT_VIEWPORT_MARGIN: f32 = 8.0;

macro_rules! state_cursor_helper {
    ($method:ident, $variant:ident, $css:literal) => {
        #[doc = concat!("Use the native `", $css, "` cursor while this state is active.")]
        pub fn $method(self) -> Self {
            self.cursor(CursorStyle::$variant)
        }
    };
}

macro_rules! spacing_scale_methods {
    ($setter:ident; $($method:ident => $units:expr),+ $(,)?) => {
        $(
            pub fn $method(self) -> Self {
                self.$setter(SPACING_UNIT * $units)
            }
        )+
    };
}

/// Maximum CSS-like box shadows retained by one element or interaction-state override.
pub const MAX_BOX_SHADOWS_PER_ELEMENT: usize = 8;

/// Maximum explicit grid tracks accepted on either axis.
///
/// The public grid helpers retain a compact `repeat()` definition, but layout cost still scales
/// with the resolved track count. Keeping this below Taffy's much larger internal safety limit
/// prevents dynamic application data from accidentally creating an expensive desktop layout.
pub const MAX_GRID_TRACKS: u16 = 1_024;

/// Maximum CSS-like container query nodes retained by one window declaration.
pub const MAX_CONTAINER_QUERIES_PER_WINDOW: usize = 1_024;

/// Maximum nested container-query depth resolved in one declaration.
pub const MAX_CONTAINER_QUERY_DEPTH: usize = 16;

/// Maximum targeted desktop mouse declarations attached to one retained element.
///
/// Ordinary elements retain only one optional pointer and allocate nothing for mouse dispatch.
pub const MAX_MOUSE_LISTENERS_PER_ELEMENT: usize = 16;

/// Maximum focused key listeners attached to one retained element.
pub const MAX_KEY_LISTENERS_PER_ELEMENT: usize = 16;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct MouseListenerKey(pub(crate) u32);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum MouseListenerKind {
    Down,
    Up,
    Move,
    Exit,
    Hover,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct MouseListenerBinding {
    pub(crate) key: MouseListenerKey,
    pub(crate) kind: MouseListenerKind,
    pub(crate) phase: DispatchPhase,
    pub(crate) button: Option<crate::MouseButton>,
    pub(crate) outside: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct KeyListenerKey(pub(crate) u32);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum KeyListenerKind {
    Down,
    Up,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct KeyListenerBinding {
    pub(crate) key: KeyListenerKey,
    pub(crate) kind: KeyListenerKind,
    pub(crate) phase: DispatchPhase,
}

const DISMISS_ON_ESCAPE: u8 = 1 << 0;
const DISMISS_ON_POINTER_OUTSIDE: u8 = 1 << 1;

/// Independent event boundaries for one dismissible retained surface.
///
/// Keeping this as one byte lets modal components distinguish ordinary dialogs from alert
/// dialogs without adding another pair of booleans to every [`Element`].
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) struct DismissPolicy(u8);

impl DismissPolicy {
    pub(crate) const BOTH: Self = Self(DISMISS_ON_ESCAPE | DISMISS_ON_POINTER_OUTSIDE);

    pub(crate) const fn on_escape(self) -> bool {
        self.0 & DISMISS_ON_ESCAPE != 0
    }

    pub(crate) const fn on_pointer_outside(self) -> bool {
        self.0 & DISMISS_ON_POINTER_OUTSIDE != 0
    }

    pub(crate) const fn with_escape(self) -> Self {
        Self(self.0 | DISMISS_ON_ESCAPE)
    }

    pub(crate) const fn with_pointer_outside(self) -> Self {
        Self(self.0 | DISMISS_ON_POINTER_OUTSIDE)
    }

    pub(crate) const fn is_empty(self) -> bool {
        self.0 == 0
    }
}

const MAX_GRID_LINE: i16 = MAX_GRID_TRACKS as i16 + 1;

/// One explicit CSS-grid track used by [`Element::grid_template_columns`] and
/// [`Element::grid_template_rows`].
///
/// Fractional tracks use `minmax(0, Nfr)`, which matches the web-friendly behavior of GPUI's
/// equal-column helpers and allows content to shrink without forcing overflow.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GridTrack(TrackSizingFunction);

impl GridTrack {
    /// A content-sized `auto` track.
    pub fn auto() -> Self {
        Self(auto())
    }

    /// A track sized to its minimum content contribution.
    pub fn min_content() -> Self {
        Self(min_content())
    }

    /// A track sized to its maximum content contribution.
    pub fn max_content() -> Self {
        Self(max_content())
    }

    /// A fixed logical-pixel track.
    pub fn px(value: f32) -> Self {
        Self(length(finite_nonnegative(value)))
    }

    /// A percentage track expressed as a `0.0..=1.0` fraction of the grid container.
    pub fn percent(fraction: f32) -> Self {
        Self(percent(finite_nonnegative(fraction).min(1.0)))
    }

    /// A flexible `minmax(0, Nfr)` track.
    pub fn fr(fraction: f32) -> Self {
        Self(flex(finite_nonnegative(fraction)))
    }

    /// The common responsive web track `minmax(<minimum px>, <fraction>fr)`.
    pub fn minmax_px_fr(minimum: f32, fraction: f32) -> Self {
        Self(minmax(
            length(finite_nonnegative(minimum)),
            fr(finite_nonnegative(fraction)),
        ))
    }

    /// An `auto` minimum with a fixed fit-content limit in logical pixels.
    pub fn fit_content_px(limit: f32) -> Self {
        Self(fit_content(LengthPercentage::length(finite_nonnegative(
            limit,
        ))))
    }
}

#[derive(Clone, Copy, Debug)]
enum EqualGridTrackSizing {
    Zero,
    MinContent,
    MaxContent,
}

fn finite_nonnegative(value: f32) -> f32 {
    if value.is_finite() {
        value.max(0.0)
    } else {
        0.0
    }
}

fn finite_opacity(value: f32) -> f32 {
    if value.is_finite() {
        value.clamp(0.0, 1.0)
    } else {
        1.0
    }
}

fn finite_length(value: f32) -> f32 {
    if value.is_finite() { value } else { 0.0 }
}

fn equal_grid_tracks(
    count: u16,
    sizing: EqualGridTrackSizing,
) -> Vec<GridTemplateComponent<String>> {
    let count = count.min(MAX_GRID_TRACKS);
    if count == 0 {
        return Vec::new();
    }
    let track: TrackSizingFunction = match sizing {
        EqualGridTrackSizing::Zero => minmax(length(0.0_f32), fr(1.0_f32)),
        EqualGridTrackSizing::MinContent => minmax(min_content(), fr(1.0_f32)),
        EqualGridTrackSizing::MaxContent => minmax(length(0.0_f32), max_content()),
    };
    vec![repeat(count, vec![track])]
}

fn bounded_grid_line(index: i16) -> GridPlacement<String> {
    if index == 0 {
        GridPlacement::Auto
    } else {
        line(index.clamp(-MAX_GRID_LINE, MAX_GRID_LINE))
    }
}

fn bounded_grid_span(span: u16) -> GridPlacement<String> {
    GridPlacement::Span(span.clamp(1, MAX_GRID_TRACKS))
}

/// A stable identifier used for hit testing and retained state.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct ElementId(pub(crate) u64);

/// Web-style window drag behavior for a laid-out element.
///
/// A `Drag` region hands primary-button drags to the native window and ignores element-level
/// pointer input. Descendants such as buttons opt back into normal input with `NoDrag`.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum AppRegion {
    Drag,
    NoDrag,
}

impl ElementId {
    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    pub fn named(value: &str) -> Self {
        Self(stable_hash(value.as_bytes()))
    }

    pub const fn as_u64(self) -> u64 {
        self.0
    }

    pub(crate) const fn value(self) -> u64 {
        self.as_u64()
    }
}

impl From<u64> for ElementId {
    fn from(value: u64) -> Self {
        Self::new(value)
    }
}

impl From<usize> for ElementId {
    fn from(value: usize) -> Self {
        Self::new(value as u64)
    }
}

impl From<&str> for ElementId {
    fn from(value: &str) -> Self {
        Self::named(value)
    }
}

impl From<String> for ElementId {
    fn from(value: String) -> Self {
        Self::named(&value)
    }
}

/// A stable handle for programmatic and keyboard focus.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct FocusHandle(ElementId);

impl FocusHandle {
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self(id.into())
    }

    pub const fn id(self) -> ElementId {
        self.0
    }
}

impl From<FocusHandle> for ElementId {
    fn from(value: FocusHandle) -> Self {
        value.id()
    }
}

/// Preferred placement for a floating element relative to its anchor.
///
/// Placement automatically flips to the opposite side when it has more usable space, then shifts
/// inside the window's content viewport.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum AnchorPlacement {
    TopStart,
    Top,
    TopEnd,
    #[default]
    BottomStart,
    Bottom,
    BottomEnd,
    LeftStart,
    Left,
    LeftEnd,
    RightStart,
    Right,
    RightEnd,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum AnchorTarget {
    Element(ElementId),
    Point(crate::Point),
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct AnchorStyle {
    pub target: AnchorTarget,
    pub placement: AnchorPlacement,
    pub gap: f32,
    pub viewport_margin: f32,
}

/// Platform-neutral semantics used to build the native accessibility tree.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum AccessibilityRole {
    #[default]
    GenericContainer,
    Label,
    Button,
    Link,
    Image,
    List,
    ListItem,
    Heading,
    CheckBox,
    RadioButton,
    RadioGroup,
    Switch,
    TextInput,
    PasswordInput,
    MultilineTextInput,
    Dialog,
    AlertDialog,
    Menu,
    MenuItem,
    MenuItemCheckBox,
    MenuItemRadio,
    /// A non-interactive visual or structural divider.
    ///
    /// AccessKit names the platform-neutral role `Splitter`; without value-changing actions it
    /// projects as the native separator/divider role exposed by each platform adapter.
    Separator,
    Group,
    Region,
    ListBox,
    ListBoxOption,
    ComboBox,
    EditableComboBox,
    Table,
    Tree,
    Grid,
    Row,
    ColumnHeader,
    RowHeader,
    GridCell,
    TreeItem,
    Tab,
    TabList,
    TabPanel,
    Tooltip,
    Form,
}

/// Axis projected for accessibility roles whose behavior changes with orientation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum AccessibilityOrientation {
    Horizontal,
    Vertical,
}

/// Kind of popup exposed by a trigger to assistive technology.
///
/// This mirrors the finite ARIA/AccessKit `has-popup` vocabulary. It describes the controlled
/// surface without imposing a visual component implementation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AccessibilityPopup {
    Menu,
    ListBox,
    Tree,
    Grid,
    Dialog,
}

/// Sort order exposed by a table or grid column header.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AccessibilitySortDirection {
    Ascending,
    Descending,
    Other,
}

/// Suggestion presentation exposed by an editable combobox or text input.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AccessibilityAutoComplete {
    Inline,
    List,
    Both,
}

/// The controlled checked state exposed by checkbox-like accessibility roles.
///
/// [`ToggleState::Mixed`] corresponds to the web `indeterminate` state and is intended primarily
/// for checkboxes that summarize a partially selected collection.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum ToggleState {
    #[default]
    Off,
    On,
    Mixed,
}

impl ToggleState {
    pub const fn is_on(self) -> bool {
        matches!(self, Self::On)
    }

    pub const fn is_mixed(self) -> bool {
        matches!(self, Self::Mixed)
    }
}

impl From<bool> for ToggleState {
    fn from(checked: bool) -> Self {
        if checked { Self::On } else { Self::Off }
    }
}

/// CSS-like policy for selecting immutable text with the pointer.
///
/// [`UserSelect::Auto`] keeps ordinary text selectable while inheriting suppression from controls
/// such as buttons and drag sources. [`UserSelect::Text`] explicitly re-enables selection and
/// [`UserSelect::None`] disables it for the complete subtree.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum UserSelect {
    #[default]
    Auto,
    Text,
    None,
}

/// Whether an element subtree is painted while retaining its layout box.
///
/// This mirrors GPUI's visibility model: [`Visibility::Hidden`] suppresses the element and its
/// descendants from paint, input, focus, and accessibility, but their layout still participates.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum Visibility {
    #[default]
    Visible,
    Hidden,
}

/// Converts common values into an [`Element`] for `.child(...)` and `.children(...)`.
pub trait IntoElement {
    fn into_element(self) -> Element;
}

impl IntoElement for Element {
    fn into_element(self) -> Element {
        self
    }
}

impl IntoElement for String {
    fn into_element(self) -> Element {
        text(self)
    }
}

impl IntoElement for Arc<str> {
    fn into_element(self) -> Element {
        text(self)
    }
}

impl IntoElement for &str {
    fn into_element(self) -> Element {
        text(Arc::<str>::from(self))
    }
}

impl IntoElement for Cow<'_, str> {
    fn into_element(self) -> Element {
        text(Arc::<str>::from(self.as_ref()))
    }
}

impl IntoElement for StyledText {
    fn into_element(self) -> Element {
        Element::styled_text(self)
    }
}

#[derive(Clone, Debug)]
pub(crate) enum ElementKind {
    Container,
    ContainerQuery(ContainerQueryElement),
    Text(Arc<str>),
    StyledText(StyledText),
    Image(ImageElement),
    Svg(SvgElement),
    Path(PathElement),
    Canvas(CanvasElement),
    CustomShader(ShaderElement),
    TextInput(TextInputElement),
    #[cfg(target_os = "macos")]
    NativeView(MacNativeView),
}

#[derive(Clone)]
pub(crate) struct ContainerQueryElement {
    render: Rc<dyn Fn(crate::Size) -> Element>,
    pub(crate) resolved_size: Option<crate::Size>,
    /// Declarative motion mounted directly by this callback. Descendant query callbacks retain
    /// their own IDs, which lets a size change unmount exactly the replaced subtree without
    /// disturbing motion elsewhere in the window.
    pub(crate) resolved_motion_ids: Vec<ElementId>,
    pub(crate) layout_pending: bool,
}

impl ContainerQueryElement {
    pub(crate) fn render(&self, size: crate::Size) -> Element {
        (self.render)(size)
    }
}

impl fmt::Debug for ContainerQueryElement {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ContainerQueryElement")
            .field("resolved_size", &self.resolved_size)
            .field("resolved_motion_count", &self.resolved_motion_ids.len())
            .field("layout_pending", &self.layout_pending)
            .finish_non_exhaustive()
    }
}

#[derive(Clone, Debug)]
pub(crate) struct ImageElement {
    pub source: ImageSource,
    pub object_fit: ObjectFit,
    pub grayscale: bool,
    pub resolved: ImageResolution,
    pub loading: Option<ImageReplacement>,
    pub fallback: Option<ImageReplacement>,
}

#[derive(Clone, Debug)]
pub(crate) struct SvgElement {
    pub svg: Svg,
    pub object_fit: ObjectFit,
    pub transform: SvgTransform,
}

#[derive(Clone, Debug)]
pub(crate) struct PathElement {
    pub path: Path,
    pub object_fit: ObjectFit,
    pub background: Option<Background>,
}

#[derive(Clone)]
pub(crate) struct CanvasElement {
    pub painter: Rc<CanvasPainter>,
}

#[derive(Clone, Debug)]
pub(crate) struct ShaderElement {
    pub shader: CustomShader,
    pub parameters: ShaderParameters,
}

type CanvasPainter = dyn for<'a> Fn(Rect, &mut Canvas<'a>);

impl fmt::Debug for CanvasElement {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("CanvasElement(..)")
    }
}

#[derive(Clone, Debug)]
pub(crate) enum ImageResolution {
    Ready(Image),
    Animated(AnimatedImage),
    Loading,
    Failed,
}

#[derive(Clone)]
pub(crate) struct ImageReplacement(Rc<dyn Fn() -> Element>);

impl ImageReplacement {
    fn new<E: IntoElement + 'static>(render: impl Fn() -> E + 'static) -> Self {
        Self(Rc::new(move || render().into_element()))
    }

    pub(crate) fn render(&self) -> Element {
        (self.0)()
    }
}

impl fmt::Debug for ImageReplacement {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("ImageReplacement(..)")
    }
}

#[derive(Clone, Debug)]
pub(crate) struct TextInputElement {
    pub value: Arc<str>,
    pub highlights: Arc<[TextHighlight]>,
    pub placeholder: Arc<str>,
    pub multiline: bool,
    pub password: bool,
    pub constraints: InputConstraints,
}

pub(crate) type InputFilterCallback = Arc<dyn Fn(&str) -> bool>;

#[derive(Clone, Default)]
pub(crate) struct InputConstraints {
    pub max_length: Option<usize>,
    pub filter: Option<InputFilterCallback>,
}

impl fmt::Debug for InputConstraints {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("InputConstraints")
            .field("max_length", &self.max_length)
            .field("filter", &self.filter.as_ref().map(|_| "InputFilter(..)"))
            .finish()
    }
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct VisualStyle {
    pub background: Option<Color>,
    pub border_color: Option<Color>,
    pub border_width: f32,
    pub radius: f32,
    pub shadows: Option<Arc<[BoxShadow]>>,
    pub opacity: f32,
}

impl Default for VisualStyle {
    fn default() -> Self {
        Self {
            background: None,
            border_color: None,
            border_width: 0.0,
            radius: 0.0,
            shadows: None,
            opacity: 1.0,
        }
    }
}

/// Non-layout overrides for hover, pressed, focus, validation, and drag states.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ElementStateStyle {
    pub(crate) background: Option<Color>,
    pub(crate) border_color: Option<Color>,
    pub(crate) border_width: Option<f32>,
    pub(crate) radius: Option<f32>,
    pub(crate) text_color: Option<Color>,
    pub(crate) shadows: Option<Arc<[BoxShadow]>>,
    pub(crate) opacity: Option<f32>,
    pub(crate) cursor_style: Option<CursorStyle>,
}

impl ElementStateStyle {
    /// Set the native cursor while this state is active.
    pub fn cursor(mut self, cursor: CursorStyle) -> Self {
        self.cursor_style = Some(cursor);
        self
    }

    state_cursor_helper!(cursor_default, Arrow, "default");
    state_cursor_helper!(cursor_pointer, PointingHand, "pointer");
    state_cursor_helper!(cursor_text, IBeam, "text");
    state_cursor_helper!(cursor_move, ClosedHand, "move");
    state_cursor_helper!(cursor_not_allowed, OperationNotAllowed, "not-allowed");
    state_cursor_helper!(cursor_context_menu, ContextualMenu, "context-menu");
    state_cursor_helper!(cursor_crosshair, Crosshair, "crosshair");
    state_cursor_helper!(
        cursor_vertical_text,
        IBeamCursorForVerticalLayout,
        "vertical-text"
    );
    state_cursor_helper!(cursor_alias, DragLink, "alias");
    state_cursor_helper!(cursor_copy, DragCopy, "copy");
    state_cursor_helper!(cursor_no_drop, OperationNotAllowed, "no-drop");
    state_cursor_helper!(cursor_grab, OpenHand, "grab");
    state_cursor_helper!(cursor_grabbing, ClosedHand, "grabbing");
    state_cursor_helper!(cursor_ew_resize, ResizeLeftRight, "ew-resize");
    state_cursor_helper!(cursor_ns_resize, ResizeUpDown, "ns-resize");
    state_cursor_helper!(cursor_nesw_resize, ResizeUpRightDownLeft, "nesw-resize");
    state_cursor_helper!(cursor_nwse_resize, ResizeUpLeftDownRight, "nwse-resize");
    state_cursor_helper!(cursor_col_resize, ResizeColumn, "col-resize");
    state_cursor_helper!(cursor_row_resize, ResizeRow, "row-resize");
    state_cursor_helper!(cursor_n_resize, ResizeUp, "n-resize");
    state_cursor_helper!(cursor_e_resize, ResizeRight, "e-resize");
    state_cursor_helper!(cursor_s_resize, ResizeDown, "s-resize");
    state_cursor_helper!(cursor_w_resize, ResizeLeft, "w-resize");

    pub fn bg(mut self, color: Color) -> Self {
        self.background = Some(color);
        self
    }

    pub fn border_color(mut self, color: Color) -> Self {
        self.border_color = Some(color);
        self
    }

    /// Paint an inside border without changing layout.
    pub fn border(mut self, width: f32, color: Color) -> Self {
        self.border_width = Some(width.max(0.0));
        self.border_color = Some(color);
        self
    }

    /// Override the paint-only corner radius while this state is active.
    pub fn rounded(mut self, radius: f32) -> Self {
        self.radius = Some(finite_nonnegative(radius));
        self
    }

    pub fn text_color(mut self, color: Color) -> Self {
        self.text_color = Some(color);
        self
    }

    /// Set the opacity of this element and its descendants while this state is active.
    pub fn opacity(mut self, opacity: f32) -> Self {
        self.opacity = Some(finite_opacity(opacity));
        self
    }

    /// Replace the element's shadows while this state is active.
    pub fn shadow(mut self, shadow: BoxShadow) -> Self {
        self.shadows = Some(Arc::from([shadow]));
        self
    }

    /// Replace the element's shadows while this state is active.
    pub fn shadows(mut self, shadows: impl IntoIterator<Item = BoxShadow>) -> Self {
        self.shadows = Some(collect_box_shadows(shadows));
        self
    }

    /// Remove all shadows while this state is active.
    pub fn shadow_none(mut self) -> Self {
        self.shadows = Some(Arc::from([]));
        self
    }

    pub fn shadow_sm(self) -> Self {
        self.shadow(shadow_sm_preset())
    }

    pub fn shadow_md(self) -> Self {
        self.shadows(shadow_md_preset())
    }

    pub fn shadow_lg(self) -> Self {
        self.shadows(shadow_lg_preset())
    }

    pub fn shadow_xl(self) -> Self {
        self.shadows(shadow_xl_preset())
    }

    pub fn shadow_2xl(self) -> Self {
        self.shadow(shadow_2xl_preset())
    }

    fn has_paint_overrides(&self) -> bool {
        self.background.is_some()
            || self.border_color.is_some()
            || self.border_width.is_some()
            || self.radius.is_some()
            || self.text_color.is_some()
            || self.shadows.is_some()
            || self.opacity.is_some()
    }
}

fn shadow_sm_preset() -> BoxShadow {
    BoxShadow::new(0.0, 1.0, Color::rgba8(0, 0, 0, 13)).blur_radius(2.0)
}

fn shadow_md_preset() -> [BoxShadow; 2] {
    [
        BoxShadow::new(0.0, 4.0, Color::rgba8(0, 0, 0, 26))
            .blur_radius(6.0)
            .spread_radius(-1.0),
        BoxShadow::new(0.0, 2.0, Color::rgba8(0, 0, 0, 26))
            .blur_radius(4.0)
            .spread_radius(-2.0),
    ]
}

fn shadow_lg_preset() -> [BoxShadow; 2] {
    [
        BoxShadow::new(0.0, 10.0, Color::rgba8(0, 0, 0, 26))
            .blur_radius(15.0)
            .spread_radius(-3.0),
        BoxShadow::new(0.0, 4.0, Color::rgba8(0, 0, 0, 26))
            .blur_radius(6.0)
            .spread_radius(-4.0),
    ]
}

fn shadow_xl_preset() -> [BoxShadow; 2] {
    [
        BoxShadow::new(0.0, 20.0, Color::rgba8(0, 0, 0, 26))
            .blur_radius(25.0)
            .spread_radius(-5.0),
        BoxShadow::new(0.0, 8.0, Color::rgba8(0, 0, 0, 26))
            .blur_radius(10.0)
            .spread_radius(-6.0),
    ]
}

fn shadow_2xl_preset() -> BoxShadow {
    BoxShadow::new(0.0, 25.0, Color::rgba8(0, 0, 0, 64))
        .blur_radius(50.0)
        .spread_radius(-12.0)
}

fn collect_box_shadows(shadows: impl IntoIterator<Item = BoxShadow>) -> Arc<[BoxShadow]> {
    let mut retained = Vec::with_capacity(2);
    for shadow in shadows {
        assert!(
            retained.len() < MAX_BOX_SHADOWS_PER_ELEMENT,
            "one element retains at most {MAX_BOX_SHADOWS_PER_ELEMENT} box shadows"
        );
        retained.push(shadow);
    }
    Arc::from(retained)
}

#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct AccessibilityStyle {
    pub hidden: bool,
    pub role: AccessibilityRole,
    pub label: Option<Arc<str>>,
    pub value: Option<Arc<str>>,
    pub disabled: bool,
    pub selected: bool,
    pub toggled: Option<ToggleState>,
    pub expanded: Option<bool>,
    pub relations: AccessibilityRelationsStyle,
    pub has_popup: Option<AccessibilityPopup>,
    pub auto_complete: Option<AccessibilityAutoComplete>,
    pub collection: AccessibilityCollectionStyle,
    pub modal: bool,
    pub required: bool,
    pub invalid: bool,
    pub validation_message: Option<Arc<str>>,
    pub validation_message_truncated: bool,
    pub description: Option<Arc<str>>,
    pub orientation: Option<AccessibilityOrientation>,
}

/// Retained keyboard policy for an unstyled tab list.
///
/// This is present only on [`AccessibilityRole::TabList`] elements. It is scanned directly from
/// the mounted tree on explicit keyboard input and retains no item registry or scheduler source.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct TabListBehavior {
    pub(crate) vertical: bool,
    pub(crate) activate_on_focus: bool,
    pub(crate) loop_focus: bool,
}

/// Exact opt-in relationships stored out of line.
///
/// Most retained elements declare no relationship, so they pay one nullable pointer instead of
/// four discriminated `ElementId` words. Relationship-bearing controls allocate one fixed record
/// during declaration and can still represent every `u64` identity without sentinels.
#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct AccessibilityRelationsStyle(Option<Box<AccessibilityRelations>>);

#[derive(Clone, Copy, Debug, Default, PartialEq)]
struct AccessibilityRelations {
    controls: Option<ElementId>,
    active_descendant: Option<ElementId>,
    labelled_by: Option<ElementId>,
    described_by: Option<ElementId>,
    described_by_secondary: Option<ElementId>,
}

impl AccessibilityRelationsStyle {
    fn values_mut(&mut self) -> &mut AccessibilityRelations {
        self.0
            .get_or_insert_with(|| Box::new(AccessibilityRelations::default()))
    }

    pub(crate) fn controls(&self) -> Option<ElementId> {
        self.0.as_deref().and_then(|values| values.controls)
    }

    pub(crate) fn set_controls(&mut self, value: ElementId) {
        self.values_mut().controls = Some(value);
    }

    pub(crate) fn active_descendant(&self) -> Option<ElementId> {
        self.0
            .as_deref()
            .and_then(|values| values.active_descendant)
    }

    pub(crate) fn set_active_descendant(&mut self, value: ElementId) {
        self.values_mut().active_descendant = Some(value);
    }

    pub(crate) fn labelled_by(&self) -> Option<ElementId> {
        self.0.as_deref().and_then(|values| values.labelled_by)
    }

    pub(crate) fn set_labelled_by(&mut self, value: ElementId) {
        self.values_mut().labelled_by = Some(value);
    }

    pub(crate) fn described_by(&self) -> Option<ElementId> {
        self.0.as_deref().and_then(|values| values.described_by)
    }

    pub(crate) fn set_described_by(&mut self, value: ElementId) {
        let values = self.values_mut();
        values.described_by = Some(value);
        values.described_by_secondary = None;
    }

    pub(crate) fn described_by_secondary(&self) -> Option<ElementId> {
        self.0
            .as_deref()
            .and_then(|values| values.described_by_secondary)
    }

    pub(crate) fn set_described_by_pair(&mut self, first: ElementId, second: ElementId) {
        let values = self.values_mut();
        values.described_by = Some(first);
        values.described_by_secondary = (second != first).then_some(second);
    }
}

const ACCESSIBILITY_COLLECTION_UNSET: u32 = u32::MAX;

/// Compact inline storage for collection metadata.
///
/// A separate `Option<usize>` for every property would add more than one hundred bytes to every
/// retained element, including ordinary text. The reserved sentinel keeps the complete table/tree
/// vocabulary allocation-free in 32 bytes while still covering QuickGUI's much smaller hard
/// collection bounds.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct AccessibilityCollectionStyle {
    pub row_count: u32,
    pub column_count: u32,
    pub row_index: u32,
    pub column_index: u32,
    pub level: u32,
    pub size_of_set: u32,
    pub position_in_set: u32,
    pub sort_direction: Option<AccessibilitySortDirection>,
}

impl Default for AccessibilityCollectionStyle {
    fn default() -> Self {
        Self {
            row_count: ACCESSIBILITY_COLLECTION_UNSET,
            column_count: ACCESSIBILITY_COLLECTION_UNSET,
            row_index: ACCESSIBILITY_COLLECTION_UNSET,
            column_index: ACCESSIBILITY_COLLECTION_UNSET,
            level: ACCESSIBILITY_COLLECTION_UNSET,
            size_of_set: ACCESSIBILITY_COLLECTION_UNSET,
            position_in_set: ACCESSIBILITY_COLLECTION_UNSET,
            sort_direction: None,
        }
    }
}

impl AccessibilityCollectionStyle {
    pub(crate) fn row_count(self) -> Option<usize> {
        accessibility_collection_value(self.row_count)
    }

    pub(crate) fn column_count(self) -> Option<usize> {
        accessibility_collection_value(self.column_count)
    }

    pub(crate) fn row_index(self) -> Option<usize> {
        accessibility_collection_value(self.row_index)
    }

    pub(crate) fn column_index(self) -> Option<usize> {
        accessibility_collection_value(self.column_index)
    }

    pub(crate) fn level(self) -> Option<usize> {
        accessibility_collection_value(self.level)
    }

    pub(crate) fn size_of_set(self) -> Option<usize> {
        accessibility_collection_value(self.size_of_set)
    }

    pub(crate) fn position_in_set(self) -> Option<usize> {
        accessibility_collection_value(self.position_in_set)
    }
}

fn accessibility_collection_storage(value: usize) -> u32 {
    assert!(
        value < ACCESSIBILITY_COLLECTION_UNSET as usize,
        "accessibility collection values must fit below u32::MAX"
    );
    value as u32
}

fn accessibility_collection_value(value: u32) -> Option<usize> {
    (value != ACCESSIBILITY_COLLECTION_UNSET).then_some(value as usize)
}

#[derive(Clone, Debug)]
pub(crate) struct VirtualScrollStyle {
    pub handle: VirtualScrollHandle,
    pub max_offset_y: f32,
    pub measurement_revision: u64,
    pub mount: VirtualScrollMount,
}

pub(crate) type DropPredicateCallback = Arc<dyn Fn(&dyn Any) -> bool>;

#[derive(Clone)]
pub(crate) struct DropPredicate {
    pub type_id: TypeId,
    pub callback: DropPredicateCallback,
}

impl fmt::Debug for DropPredicate {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("DropPredicate")
            .field("type_id", &self.type_id)
            .finish_non_exhaustive()
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct TypographyStyle {
    pub color: Option<Color>,
    pub font_size: Option<f32>,
    pub line_height: Option<f32>,
    pub family: Option<FontFamily>,
    pub features: Option<FontFeatures>,
    pub fallbacks: Option<Option<FontFallbacks>>,
    pub weight: Option<Weight>,
    pub font_style: Option<GlyphStyle>,
    pub underline: Option<TextUnderline>,
    pub underline_color: Option<Option<Color>>,
    pub underline_wavy: Option<bool>,
    pub underline_thickness: Option<f32>,
    pub strikethrough: Option<bool>,
    pub strikethrough_color: Option<Option<Color>>,
    pub align: Option<TextAlign>,
    pub wrap: Option<TextWrap>,
    pub text_overflow: Option<TextOverflow>,
    pub line_clamp: Option<usize>,
    pub shaping: Option<TextShaping>,
}

impl TypographyStyle {
    pub(crate) fn resolve(&self, inherited: &TextStyle) -> TextStyle {
        let font_size = self.font_size.unwrap_or(inherited.font_size);
        TextStyle {
            font_size,
            line_height: self.line_height.unwrap_or_else(|| {
                self.font_size
                    .map(|_| font_size * 1.35)
                    .unwrap_or(inherited.line_height)
            }),
            family: self
                .family
                .clone()
                .unwrap_or_else(|| inherited.family.clone()),
            features: self
                .features
                .clone()
                .unwrap_or_else(|| inherited.features.clone()),
            fallbacks: normalize_fallbacks(
                self.fallbacks
                    .clone()
                    .unwrap_or_else(|| inherited.fallbacks.clone()),
            ),
            weight: self.weight.unwrap_or(inherited.weight),
            font_style: self.font_style.unwrap_or(inherited.font_style),
            underline: self.underline.unwrap_or(inherited.underline),
            underline_color: self.underline_color.unwrap_or(inherited.underline_color),
            underline_wavy: self.underline_wavy.unwrap_or(inherited.underline_wavy),
            underline_thickness: self
                .underline_thickness
                .unwrap_or(inherited.underline_thickness),
            strikethrough: self.strikethrough.unwrap_or(inherited.strikethrough),
            strikethrough_color: self
                .strikethrough_color
                .unwrap_or(inherited.strikethrough_color),
            align: self.align.unwrap_or(inherited.align),
            wrap: self.wrap.unwrap_or(inherited.wrap),
            text_overflow: self
                .text_overflow
                .clone()
                .or_else(|| inherited.text_overflow.clone()),
            line_clamp: self.line_clamp.or(inherited.line_clamp),
            shaping: self.shaping.unwrap_or(inherited.shaping),
            color: self.color.unwrap_or(inherited.color),
        }
    }
}

/// A declarative UI node with Tailwind-like fluent styling.
#[derive(Clone, Debug)]
pub struct Element {
    pub(crate) explicit_id: Option<ElementId>,
    pub(crate) runtime_id: ElementId,
    pub(crate) kind: ElementKind,
    pub(crate) layout: Style,
    pub(crate) visibility: Visibility,
    pub(crate) visual: VisualStyle,
    pub(crate) typography: TypographyStyle,
    pub(crate) resolved_typography: TextStyle,
    pub(crate) hover: ElementStateStyle,
    pub(crate) active: ElementStateStyle,
    pub(crate) focus: ElementStateStyle,
    pub(crate) disabled_style: ElementStateStyle,
    pub(crate) invalid_style: ElementStateStyle,
    pub(crate) dragging: ElementStateStyle,
    pub(crate) drag_over: ElementStateStyle,
    pub(crate) clickable: bool,
    pub(crate) pointer_listener: bool,
    pub(crate) scroll_wheel_listener: bool,
    pub(crate) touch_listener: bool,
    pub(crate) context_menu_listener: bool,
    // This extra indirection is intentional: listeners are uncommon, and keeping the
    // non-interactive Element representation compact matters more than one opt-in allocation.
    #[allow(clippy::box_collection)]
    pub(crate) mouse_listeners: Option<Box<Vec<MouseListenerBinding>>>,
    #[allow(clippy::box_collection)]
    pub(crate) key_listeners: Option<Box<Vec<KeyListenerBinding>>>,
    #[allow(clippy::box_collection)]
    pub(crate) action_listeners: Option<Box<Vec<ActionListenerBinding>>>,
    pub(crate) mouse_pressure_listener: bool,
    pub(crate) pinch_listener: bool,
    pub(crate) rotation_listener: bool,
    pub(crate) smart_magnify_listener: bool,
    pub(crate) drag_source: bool,
    pub(crate) drop_target: bool,
    pub(crate) drop_predicates: Vec<DropPredicate>,
    pub(crate) cursor_style: Option<CursorStyle>,
    pub(crate) cursor_style_explicit: bool,
    pub(crate) user_select: UserSelect,
    pub(crate) resolved_user_select: bool,
    pub(crate) focusable: bool,
    pub(crate) focus_trap: bool,
    pub(crate) key_context: Option<KeyContext>,
    pub(crate) tab_index: i16,
    pub(crate) auto_focus: bool,
    pub(crate) form: bool,
    pub(crate) form_submitter: bool,
    pub(crate) activation_target: Option<ElementId>,
    pub(crate) accessibility: AccessibilityStyle,
    pub(crate) tab_list_behavior: Option<TabListBehavior>,
    pub(crate) plane: Option<ScenePlane>,
    pub(crate) z_index: Option<i16>,
    pub(crate) portal: bool,
    pub(crate) anchor: Option<AnchorStyle>,
    pub(crate) tooltip: Option<Tooltip>,
    pub(crate) app_region: Option<AppRegion>,
    pub(crate) virtual_scroll: Option<VirtualScrollStyle>,
    pub(crate) scroll_to_end_revision: Option<u64>,
    pub(crate) list_item_measurement: Option<ListItemMeasurement>,
    pub(crate) animation: Option<ElementAnimation>,
    pub(crate) spring: Option<ElementSpring>,
    pub(crate) transition: Option<Transition>,
    pub(crate) blocks_pointer: bool,
    pub(crate) dismiss_policy: DismissPolicy,
    pub(crate) restore_focus: Option<FocusHandle>,
    pub(crate) children: Vec<Element>,
    pub(crate) taffy_node: Option<taffy::NodeId>,
}

/// Create a container element.
pub fn div() -> Element {
    Element::container()
}

/// Create a CSS-like container query whose contents are declared from its assigned size.
///
/// The query fills its parent by default. Its callback is evaluated after the query's own layout,
/// and the returned subtree is laid out independently inside that fixed box, so contents cannot
/// affect the query's intrinsic size.
///
/// An unchanged assigned size retains the existing subtree. A size change evaluates the callback
/// again, while stable declarative-animation IDs keep their playback state. Query contents must be
/// returned from this callback rather than attached with [`Element::child`] or
/// [`Element::children`].
///
/// # Example
///
/// ```
/// use quickgui::{container_query, div, text};
///
/// let responsive = container_query(|size| {
///     if size.width < 480.0 {
///         div().flex_col().child(text("Compact"))
///     } else {
///         div().grid().grid_cols(3).child(text("Wide"))
///     }
/// });
/// ```
pub fn container_query<E>(render: impl Fn(crate::Size) -> E + 'static) -> Element
where
    E: IntoElement,
{
    let mut element = Element::container();
    element.kind = ElementKind::ContainerQuery(ContainerQueryElement {
        render: Rc::new(move |size| render(size).into_element()),
        resolved_size: None,
        resolved_motion_ids: Vec::new(),
        layout_pending: false,
    });
    element.layout.size = TaffySize {
        width: Dimension::percent(1.0),
        height: Dimension::percent(1.0),
    };
    element
}

/// Create a semantic form container.
///
/// Attach callbacks with [`Element::on_form_submit`] and [`Element::on_form_invalid`]. Return in a
/// descendant single-line input and [`submit_button`] both validate the nearest form.
pub fn form() -> Element {
    let mut element = div().accessibility_role(AccessibilityRole::Form);
    element.form = true;
    element
}

/// Create a viewport-level element painted on the overlay plane.
///
/// Overlay elements are removed from normal flow, escape ancestor clipping, and block pointer
/// events inside their bounds. Use [`Element::anchor_to`] for popovers and menus.
pub fn overlay() -> Element {
    div().overlay()
}

/// Create a semantic, keyboard-focusable button container.
pub fn button() -> Element {
    let mut element = div()
        .accessibility_role(AccessibilityRole::Button)
        .focusable();
    element.set_implicit_cursor(CursorStyle::PointingHand);
    element
}

/// Create a button that validates and submits its nearest ancestor [`form`].
pub fn submit_button() -> Element {
    button().form_submitter()
}

/// Create a text element. Strings are owned through `Arc<str>` and cheap to retain.
pub fn text(content: impl Into<Arc<str>>) -> Element {
    Element::text(content.into())
}

/// Create an image element with intrinsic sizing and `object-fit: contain` behavior.
pub fn img(source: impl Into<ImageSource>) -> Element {
    Element::image(source.into())
}

/// Create a monochrome SVG element with intrinsic sizing and inherited text color.
pub fn svg(source: impl Into<Svg>) -> Element {
    Element::svg(source.into())
}

/// Create a retained vector path element with intrinsic sizing and inherited text color.
pub fn path(source: impl Into<Path>) -> Element {
    Element::path(source.into())
}

/// Create a web-like custom paint surface. The callback runs only when the view repaints.
///
/// Callback coordinates are local to the element and are clipped to its layout box.
pub fn canvas(painter: impl for<'a> Fn(Rect, &mut Canvas<'a>) + 'static) -> Element {
    Element::canvas(painter)
}

/// Create a retained rectangle painted by validated application WGSL.
///
/// Like the web canvas default, its initial size is 300 by 150 logical pixels. The shader remains
/// still until application state invalidates the view or explicitly requests another frame.
pub fn custom_shader(shader: impl Into<CustomShader>) -> Element {
    Element::custom_shader(shader.into())
}

/// Create a controlled, single-line text input.
///
/// Attach a stable [`crate::InputListener`] with [`Element::on_input`].
pub fn text_input(value: impl Into<Arc<str>>) -> Element {
    Element::text_input(value.into(), Arc::from([]), false)
}

/// Create a controlled, multiline text area with web-style soft wrapping.
///
/// Attach a stable [`crate::InputListener`] with [`Element::on_input`]. Use [`Element::no_wrap`]
/// for code-editor-style horizontal scrolling.
pub fn text_area(value: impl Into<Arc<str>>) -> Element {
    Element::text_input(value.into(), Arc::from([]), true)
}

/// Create a controlled, single-line input with bounded byte-range text styles.
///
/// Attach a stable [`crate::InputListener`] with [`Element::on_input`] and rebuild the
/// [`StyledText`] from the controlled value when it changes. Editing, selection, IME, hit testing,
/// and painting all reuse the same retained shaped buffer.
pub fn styled_text_input(value: StyledText) -> Element {
    let (value, highlights) = value.into_parts();
    Element::text_input(value, highlights, false)
}

/// Create a controlled, multiline text area with bounded byte-range text styles.
///
/// This is the attributed-text counterpart to [`text_area`]. It preserves wrapping and both-axis
/// scrolling while using the supplied styles for shaping, caret geometry, selection, and paint.
pub fn styled_text_area(value: StyledText) -> Element {
    let (value, highlights) = value.into_parts();
    Element::text_input(value, highlights, true)
}

/// Embed an AppKit view as a declarative leaf on macOS.
///
/// QuickGUI synchronizes layout, clipping, visibility, and sibling order while AppKit keeps
/// ownership of the view's rendering and input behavior.
#[cfg(target_os = "macos")]
pub fn native_view(view: &NSView) -> Element {
    Element::native_view(MacNativeView::new(view))
}

impl Element {
    fn container() -> Self {
        let default_text = TextStyle::new(14.0, Color::WHITE);
        Self {
            explicit_id: None,
            runtime_id: ElementId::new(0),
            kind: ElementKind::Container,
            layout: Style::default(),
            visibility: Visibility::Visible,
            visual: VisualStyle::default(),
            typography: TypographyStyle::default(),
            resolved_typography: default_text,
            hover: ElementStateStyle::default(),
            active: ElementStateStyle::default(),
            focus: ElementStateStyle::default(),
            disabled_style: ElementStateStyle::default(),
            invalid_style: ElementStateStyle::default(),
            dragging: ElementStateStyle::default(),
            drag_over: ElementStateStyle::default(),
            clickable: false,
            pointer_listener: false,
            scroll_wheel_listener: false,
            touch_listener: false,
            context_menu_listener: false,
            mouse_listeners: None,
            key_listeners: None,
            action_listeners: None,
            mouse_pressure_listener: false,
            pinch_listener: false,
            rotation_listener: false,
            smart_magnify_listener: false,
            drag_source: false,
            drop_target: false,
            drop_predicates: Vec::new(),
            cursor_style: None,
            cursor_style_explicit: false,
            user_select: UserSelect::Auto,
            resolved_user_select: false,
            focusable: false,
            focus_trap: false,
            key_context: None,
            tab_index: 0,
            auto_focus: false,
            form: false,
            form_submitter: false,
            activation_target: None,
            accessibility: AccessibilityStyle::default(),
            tab_list_behavior: None,
            plane: None,
            z_index: None,
            portal: false,
            anchor: None,
            tooltip: None,
            app_region: None,
            virtual_scroll: None,
            scroll_to_end_revision: None,
            list_item_measurement: None,
            animation: None,
            spring: None,
            transition: None,
            blocks_pointer: false,
            dismiss_policy: DismissPolicy::default(),
            restore_focus: None,
            children: Vec::new(),
            taffy_node: None,
        }
    }

    fn text(content: Arc<str>) -> Self {
        let mut element = Self::container();
        element.kind = ElementKind::Text(content);
        element.accessibility.role = AccessibilityRole::Label;
        element
    }

    fn styled_text(content: StyledText) -> Self {
        let mut element = Self::container();
        element.kind = ElementKind::StyledText(content);
        element.accessibility.role = AccessibilityRole::Label;
        element
    }

    fn image(source: ImageSource) -> Self {
        let mut element = Self::container();
        let resolved = if let Some(image) = source.image() {
            ImageResolution::Ready(image.clone())
        } else if let Some(animation) = source.animated() {
            ImageResolution::Animated(animation.clone())
        } else {
            ImageResolution::Loading
        };
        element.kind = ElementKind::Image(ImageElement {
            source,
            object_fit: ObjectFit::Contain,
            grayscale: false,
            resolved,
            loading: None,
            fallback: None,
        });
        element.accessibility.role = AccessibilityRole::Image;
        element
    }

    fn svg(svg: Svg) -> Self {
        let mut element = Self::container();
        element.kind = ElementKind::Svg(SvgElement {
            svg,
            object_fit: ObjectFit::Contain,
            transform: SvgTransform::IDENTITY,
        });
        element.accessibility.role = AccessibilityRole::Image;
        element
    }

    fn path(path: Path) -> Self {
        let mut element = Self::container();
        element.kind = ElementKind::Path(PathElement {
            path,
            object_fit: ObjectFit::Contain,
            background: None,
        });
        element.accessibility.role = AccessibilityRole::Image;
        element
    }

    fn canvas(painter: impl for<'a> Fn(Rect, &mut Canvas<'a>) + 'static) -> Self {
        let mut element = Self::container();
        element.kind = ElementKind::Canvas(CanvasElement {
            painter: Rc::new(painter),
        });
        // Match the default dimensions of the web canvas element while still allowing normal
        // width/height utilities to override them.
        element.layout.size = TaffySize {
            width: Dimension::length(300.0),
            height: Dimension::length(150.0),
        };
        element
    }

    fn custom_shader(shader: CustomShader) -> Self {
        let mut element = Self::container();
        element.kind = ElementKind::CustomShader(ShaderElement {
            shader,
            parameters: ShaderParameters::default(),
        });
        element.layout.size = TaffySize {
            width: Dimension::length(300.0),
            height: Dimension::length(150.0),
        };
        element
    }

    fn text_input(value: Arc<str>, highlights: Arc<[TextHighlight]>, multiline: bool) -> Self {
        let mut element = Self::container();
        element.kind = ElementKind::TextInput(TextInputElement {
            value,
            highlights,
            placeholder: Arc::from(""),
            multiline,
            password: false,
            constraints: InputConstraints::default(),
        });
        element.layout.size = TaffySize {
            width: Dimension::length(if multiline { 320.0 } else { 240.0 }),
            height: Dimension::length(if multiline { 160.0 } else { 40.0 }),
        };
        element.layout.overflow = TaffyPoint {
            x: Overflow::Hidden,
            y: Overflow::Hidden,
        };
        element.visual.background = Some(Color::rgb8(28, 30, 35));
        element.visual.border_color = Some(Color::rgb8(70, 74, 85));
        element.visual.border_width = 1.0;
        element.visual.radius = 8.0;
        element.focus = ElementStateStyle::default().border(2.0, Color::rgb8(94, 234, 212));
        element.invalid_style =
            ElementStateStyle::default().border(2.0, Color::rgb8(248, 113, 113));
        element.focusable = true;
        element.set_implicit_cursor(CursorStyle::IBeam);
        element.accessibility.role = if multiline {
            AccessibilityRole::MultilineTextInput
        } else {
            AccessibilityRole::TextInput
        };
        element.typography.wrap = Some(if multiline {
            TextWrap::Word
        } else {
            TextWrap::None
        });
        element
    }

    #[cfg(target_os = "macos")]
    fn native_view(view: MacNativeView) -> Self {
        let mut element = Self::container();
        element.kind = ElementKind::NativeView(view);
        element.layout.size = TaffySize {
            width: Dimension::length(320.0),
            height: Dimension::length(200.0),
        };
        element.blocks_pointer = true;
        element
    }

    pub fn id(mut self, id: impl Into<ElementId>) -> Self {
        self.bind_listener_id(id.into());
        self
    }

    pub fn child(mut self, child: impl IntoElement) -> Self {
        assert!(
            !matches!(self.kind, ElementKind::ContainerQuery(_)),
            "container_query contents must come from its size callback"
        );
        self.children.push(child.into_element());
        self
    }

    pub fn children<I, E>(mut self, children: I) -> Self
    where
        I: IntoIterator<Item = E>,
        E: IntoElement,
    {
        assert!(
            !matches!(self.kind, ElementKind::ContainerQuery(_)),
            "container_query contents must come from its size callback"
        );
        self.children
            .extend(children.into_iter().map(IntoElement::into_element));
        self
    }

    pub fn when(self, condition: bool, apply: impl FnOnce(Self) -> Self) -> Self {
        if condition { apply(self) } else { self }
    }

    /// Lay out children with the CSS block algorithm.
    pub fn block(mut self) -> Self {
        self.layout.display = Display::Block;
        self
    }

    pub fn flex(mut self) -> Self {
        self.layout.display = Display::Flex;
        self
    }

    /// Lay out children with the CSS Grid algorithm.
    pub fn grid(mut self) -> Self {
        self.layout.display = Display::Grid;
        self
    }

    /// Remove this element and its complete subtree from layout and interaction.
    ///
    /// This matches GPUI and CSS `display: none`: descendants do not paint, receive input,
    /// participate in focus or accessibility, resolve container queries or image resources, or
    /// keep animation clocks active. Calling [`Element::block`], [`Element::flex`], or
    /// [`Element::grid`] later on the same declaration restores a display mode.
    pub fn hidden(mut self) -> Self {
        self.layout.display = Display::None;
        self
    }

    pub(crate) fn is_display_none(&self) -> bool {
        self.layout.display == Display::None
    }

    /// Paint this element subtree while retaining its existing display mode.
    pub fn visible(mut self) -> Self {
        self.visibility = Visibility::Visible;
        self
    }

    /// Suppress painting and interaction for this subtree without removing its layout box.
    pub fn invisible(mut self) -> Self {
        self.visibility = Visibility::Hidden;
        self
    }

    pub(crate) fn is_visibility_hidden(&self) -> bool {
        self.visibility == Visibility::Hidden
    }

    /// Set `count` equal `minmax(0, 1fr)` columns, matching GPUI's `grid_cols` helper.
    pub fn grid_cols(mut self, count: u16) -> Self {
        self.layout.grid_template_columns = equal_grid_tracks(count, EqualGridTrackSizing::Zero);
        self
    }

    /// Set equal columns with a `min-content` minimum.
    pub fn grid_cols_min_content(mut self, count: u16) -> Self {
        self.layout.grid_template_columns =
            equal_grid_tracks(count, EqualGridTrackSizing::MinContent);
        self
    }

    /// Set content-sized columns using `minmax(0, max-content)`.
    pub fn grid_cols_max_content(mut self, count: u16) -> Self {
        self.layout.grid_template_columns =
            equal_grid_tracks(count, EqualGridTrackSizing::MaxContent);
        self
    }

    /// Set `count` equal `minmax(0, 1fr)` rows, matching GPUI's `grid_rows` helper.
    pub fn grid_rows(mut self, count: u16) -> Self {
        self.layout.grid_template_rows = equal_grid_tracks(count, EqualGridTrackSizing::Zero);
        self
    }

    /// Set equal rows with a `min-content` minimum.
    pub fn grid_rows_min_content(mut self, count: u16) -> Self {
        self.layout.grid_template_rows = equal_grid_tracks(count, EqualGridTrackSizing::MinContent);
        self
    }

    /// Set content-sized rows using `minmax(0, max-content)`.
    pub fn grid_rows_max_content(mut self, count: u16) -> Self {
        self.layout.grid_template_rows = equal_grid_tracks(count, EqualGridTrackSizing::MaxContent);
        self
    }

    /// Set an explicit web-style column template. At most [`MAX_GRID_TRACKS`] entries are kept.
    pub fn grid_template_columns(mut self, tracks: impl IntoIterator<Item = GridTrack>) -> Self {
        self.layout.grid_template_columns = tracks
            .into_iter()
            .take(usize::from(MAX_GRID_TRACKS))
            .map(|track| GridTemplateComponent::Single(track.0))
            .collect();
        self
    }

    /// Set an explicit web-style row template. At most [`MAX_GRID_TRACKS`] entries are kept.
    pub fn grid_template_rows(mut self, tracks: impl IntoIterator<Item = GridTrack>) -> Self {
        self.layout.grid_template_rows = tracks
            .into_iter()
            .take(usize::from(MAX_GRID_TRACKS))
            .map(|track| GridTemplateComponent::Single(track.0))
            .collect();
        self
    }

    /// Auto-place items row by row.
    pub fn grid_flow_row(mut self) -> Self {
        self.layout.grid_auto_flow = GridAutoFlow::Row;
        self
    }

    /// Auto-place items column by column.
    pub fn grid_flow_col(mut self) -> Self {
        self.layout.grid_auto_flow = GridAutoFlow::Column;
        self
    }

    /// Densely backfill holes while auto-placing items row by row.
    pub fn grid_flow_row_dense(mut self) -> Self {
        self.layout.grid_auto_flow = GridAutoFlow::RowDense;
        self
    }

    /// Densely backfill holes while auto-placing items column by column.
    pub fn grid_flow_col_dense(mut self) -> Self {
        self.layout.grid_auto_flow = GridAutoFlow::ColumnDense;
        self
    }

    /// Start this grid item at a one-based CSS column line. Zero restores `auto`.
    pub fn col_start(mut self, start: i16) -> Self {
        self.layout.grid_column.start = bounded_grid_line(start);
        self
    }

    pub fn col_start_auto(mut self) -> Self {
        self.layout.grid_column.start = GridPlacement::Auto;
        self
    }

    /// End this grid item at a one-based CSS column line. Zero restores `auto`.
    pub fn col_end(mut self, end: i16) -> Self {
        self.layout.grid_column.end = bounded_grid_line(end);
        self
    }

    pub fn col_end_auto(mut self) -> Self {
        self.layout.grid_column.end = GridPlacement::Auto;
        self
    }

    /// Span this item across a bounded number of columns. Zero is treated as one.
    pub fn col_span(mut self, span: u16) -> Self {
        let span = bounded_grid_span(span);
        self.layout.grid_column = TaffyLine {
            start: span.clone(),
            end: span,
        };
        self
    }

    /// Span from the first to the final explicit column line.
    pub fn col_span_full(mut self) -> Self {
        self.layout.grid_column = TaffyLine {
            start: bounded_grid_line(1),
            end: bounded_grid_line(-1),
        };
        self
    }

    /// Start this grid item at a one-based CSS row line. Zero restores `auto`.
    pub fn row_start(mut self, start: i16) -> Self {
        self.layout.grid_row.start = bounded_grid_line(start);
        self
    }

    pub fn row_start_auto(mut self) -> Self {
        self.layout.grid_row.start = GridPlacement::Auto;
        self
    }

    /// End this grid item at a one-based CSS row line. Zero restores `auto`.
    pub fn row_end(mut self, end: i16) -> Self {
        self.layout.grid_row.end = bounded_grid_line(end);
        self
    }

    pub fn row_end_auto(mut self) -> Self {
        self.layout.grid_row.end = GridPlacement::Auto;
        self
    }

    /// Span this item across a bounded number of rows. Zero is treated as one.
    pub fn row_span(mut self, span: u16) -> Self {
        let span = bounded_grid_span(span);
        self.layout.grid_row = TaffyLine {
            start: span.clone(),
            end: span,
        };
        self
    }

    /// Span from the first to the final explicit row line.
    pub fn row_span_full(mut self) -> Self {
        self.layout.grid_row = TaffyLine {
            start: bounded_grid_line(1),
            end: bounded_grid_line(-1),
        };
        self
    }

    pub fn flex_row(mut self) -> Self {
        self.layout.display = Display::Flex;
        self.layout.flex_direction = FlexDirection::Row;
        self
    }

    pub fn flex_row_reverse(mut self) -> Self {
        self.layout.display = Display::Flex;
        self.layout.flex_direction = FlexDirection::RowReverse;
        self
    }

    pub fn flex_col(mut self) -> Self {
        self.layout.display = Display::Flex;
        self.layout.flex_direction = FlexDirection::Column;
        self
    }

    pub fn flex_col_reverse(mut self) -> Self {
        self.layout.display = Display::Flex;
        self.layout.flex_direction = FlexDirection::ColumnReverse;
        self
    }

    pub fn flex_wrap(mut self) -> Self {
        self.layout.flex_wrap = FlexWrap::Wrap;
        self
    }

    pub fn flex_wrap_reverse(mut self) -> Self {
        self.layout.flex_wrap = FlexWrap::WrapReverse;
        self
    }

    pub fn flex_nowrap(mut self) -> Self {
        self.layout.flex_wrap = FlexWrap::NoWrap;
        self
    }

    pub fn flex_1(mut self) -> Self {
        self.layout.flex_grow = 1.0;
        self.layout.flex_shrink = 1.0;
        self.layout.flex_basis = Dimension::length(0.0);
        self
    }

    pub fn flex_auto(mut self) -> Self {
        self.layout.flex_grow = 1.0;
        self.layout.flex_shrink = 1.0;
        self.layout.flex_basis = Dimension::auto();
        self
    }

    pub fn flex_initial(mut self) -> Self {
        self.layout.flex_grow = 0.0;
        self.layout.flex_shrink = 1.0;
        self.layout.flex_basis = Dimension::auto();
        self
    }

    pub fn flex_none(mut self) -> Self {
        self.layout.flex_grow = 0.0;
        self.layout.flex_shrink = 0.0;
        self.layout.flex_basis = Dimension::auto();
        self
    }

    /// Set an absolute logical-pixel flex basis.
    pub fn flex_basis(mut self, basis: f32) -> Self {
        self.layout.flex_basis = Dimension::length(finite_nonnegative(basis));
        self
    }

    pub fn flex_basis_auto(mut self) -> Self {
        self.layout.flex_basis = Dimension::auto();
        self
    }

    pub fn flex_grow(mut self, grow: f32) -> Self {
        self.layout.flex_grow = finite_nonnegative(grow);
        self
    }

    pub fn flex_grow_0(self) -> Self {
        self.flex_grow(0.0)
    }

    pub fn flex_grow_1(self) -> Self {
        self.flex_grow(1.0)
    }

    pub fn flex_shrink(mut self, shrink: f32) -> Self {
        self.layout.flex_shrink = finite_nonnegative(shrink);
        self
    }

    pub fn flex_shrink_0(self) -> Self {
        self.flex_shrink(0.0)
    }

    pub fn flex_shrink_1(self) -> Self {
        self.flex_shrink(1.0)
    }

    pub fn items_start(mut self) -> Self {
        self.layout.align_items = Some(AlignItems::FLEX_START);
        self
    }

    pub fn items_center(mut self) -> Self {
        self.layout.align_items = Some(AlignItems::CENTER);
        self
    }

    pub fn items_end(mut self) -> Self {
        self.layout.align_items = Some(AlignItems::FLEX_END);
        self
    }

    pub fn items_baseline(mut self) -> Self {
        self.layout.align_items = Some(AlignItems::BASELINE);
        self
    }

    pub fn items_stretch(mut self) -> Self {
        self.layout.align_items = Some(AlignItems::STRETCH);
        self
    }

    pub fn self_start(mut self) -> Self {
        self.layout.align_self = Some(AlignSelf::START);
        self
    }

    pub fn self_end(mut self) -> Self {
        self.layout.align_self = Some(AlignSelf::END);
        self
    }

    pub fn self_flex_start(mut self) -> Self {
        self.layout.align_self = Some(AlignSelf::FLEX_START);
        self
    }

    pub fn self_flex_end(mut self) -> Self {
        self.layout.align_self = Some(AlignSelf::FLEX_END);
        self
    }

    pub fn self_center(mut self) -> Self {
        self.layout.align_self = Some(AlignSelf::CENTER);
        self
    }

    pub fn self_baseline(mut self) -> Self {
        self.layout.align_self = Some(AlignSelf::BASELINE);
        self
    }

    pub fn self_stretch(mut self) -> Self {
        self.layout.align_self = Some(AlignSelf::STRETCH);
        self
    }

    pub fn justify_start(mut self) -> Self {
        self.layout.justify_content = Some(JustifyContent::START);
        self
    }

    pub fn justify_center(mut self) -> Self {
        self.layout.justify_content = Some(JustifyContent::CENTER);
        self
    }

    pub fn justify_end(mut self) -> Self {
        self.layout.justify_content = Some(JustifyContent::END);
        self
    }

    pub fn justify_between(mut self) -> Self {
        self.layout.justify_content = Some(JustifyContent::SPACE_BETWEEN);
        self
    }

    pub fn justify_around(mut self) -> Self {
        self.layout.justify_content = Some(JustifyContent::SPACE_AROUND);
        self
    }

    pub fn justify_evenly(mut self) -> Self {
        self.layout.justify_content = Some(JustifyContent::SPACE_EVENLY);
        self
    }

    pub fn content_normal(mut self) -> Self {
        self.layout.align_content = None;
        self
    }

    pub fn content_center(mut self) -> Self {
        self.layout.align_content = Some(AlignContent::CENTER);
        self
    }

    pub fn content_start(mut self) -> Self {
        self.layout.align_content = Some(AlignContent::FLEX_START);
        self
    }

    pub fn content_end(mut self) -> Self {
        self.layout.align_content = Some(AlignContent::FLEX_END);
        self
    }

    pub fn content_between(mut self) -> Self {
        self.layout.align_content = Some(AlignContent::SPACE_BETWEEN);
        self
    }

    pub fn content_around(mut self) -> Self {
        self.layout.align_content = Some(AlignContent::SPACE_AROUND);
        self
    }

    pub fn content_evenly(mut self) -> Self {
        self.layout.align_content = Some(AlignContent::SPACE_EVENLY);
        self
    }

    pub fn content_stretch(mut self) -> Self {
        self.layout.align_content = Some(AlignContent::STRETCH);
        self
    }

    /// Preserve `width / height` when exactly one axis is definite.
    pub fn aspect_ratio(mut self, ratio: f32) -> Self {
        self.layout.aspect_ratio = (ratio.is_finite() && ratio > 0.0).then_some(ratio);
        self
    }

    pub fn aspect_square(self) -> Self {
        self.aspect_ratio(1.0)
    }

    pub fn gap(mut self, value: f32) -> Self {
        let value = finite_nonnegative(value);
        self.layout.gap = TaffySize {
            width: LengthPercentage::length(value),
            height: LengthPercentage::length(value),
        };
        self
    }

    pub fn gap_x(mut self, value: f32) -> Self {
        self.layout.gap.width = LengthPercentage::length(finite_nonnegative(value));
        self
    }

    pub fn gap_y(mut self, value: f32) -> Self {
        self.layout.gap.height = LengthPercentage::length(finite_nonnegative(value));
        self
    }

    pub fn gap_1(self) -> Self {
        self.gap(SPACING_UNIT)
    }
    pub fn gap_2(self) -> Self {
        self.gap(SPACING_UNIT * 2.0)
    }
    pub fn gap_3(self) -> Self {
        self.gap(SPACING_UNIT * 3.0)
    }
    pub fn gap_4(self) -> Self {
        self.gap(SPACING_UNIT * 4.0)
    }
    pub fn gap_5(self) -> Self {
        self.gap(SPACING_UNIT * 5.0)
    }
    pub fn gap_6(self) -> Self {
        self.gap(SPACING_UNIT * 6.0)
    }

    spacing_scale_methods!(gap_x;
        gap_x_0 => 0.0,
        gap_x_1 => 1.0,
        gap_x_2 => 2.0,
        gap_x_3 => 3.0,
        gap_x_4 => 4.0,
        gap_x_5 => 5.0,
        gap_x_6 => 6.0,
    );
    spacing_scale_methods!(gap_y;
        gap_y_0 => 0.0,
        gap_y_1 => 1.0,
        gap_y_2 => 2.0,
        gap_y_3 => 3.0,
        gap_y_4 => 4.0,
        gap_y_5 => 5.0,
        gap_y_6 => 6.0,
    );

    /// Set all four margins in logical pixels. Finite negative margins are supported.
    pub fn m(self, value: f32) -> Self {
        self.margin(value, value, value, value)
    }

    pub fn mx(mut self, value: f32) -> Self {
        let value = LengthPercentageAuto::length(finite_length(value));
        self.layout.margin.left = value;
        self.layout.margin.right = value;
        self
    }

    pub fn my(mut self, value: f32) -> Self {
        let value = LengthPercentageAuto::length(finite_length(value));
        self.layout.margin.top = value;
        self.layout.margin.bottom = value;
        self
    }

    pub fn mt(mut self, value: f32) -> Self {
        self.layout.margin.top = LengthPercentageAuto::length(finite_length(value));
        self
    }

    pub fn mr(mut self, value: f32) -> Self {
        self.layout.margin.right = LengthPercentageAuto::length(finite_length(value));
        self
    }

    pub fn mb(mut self, value: f32) -> Self {
        self.layout.margin.bottom = LengthPercentageAuto::length(finite_length(value));
        self
    }

    pub fn ml(mut self, value: f32) -> Self {
        self.layout.margin.left = LengthPercentageAuto::length(finite_length(value));
        self
    }

    /// Set margins in CSS order: top, right, bottom, left.
    pub fn margin(mut self, top: f32, right: f32, bottom: f32, left: f32) -> Self {
        self.layout.margin = TaffyRect {
            left: LengthPercentageAuto::length(finite_length(left)),
            right: LengthPercentageAuto::length(finite_length(right)),
            top: LengthPercentageAuto::length(finite_length(top)),
            bottom: LengthPercentageAuto::length(finite_length(bottom)),
        };
        self
    }

    pub fn m_auto(mut self) -> Self {
        self.layout.margin = TaffyRect::auto();
        self
    }

    pub fn mx_auto(mut self) -> Self {
        self.layout.margin.left = LengthPercentageAuto::auto();
        self.layout.margin.right = LengthPercentageAuto::auto();
        self
    }

    pub fn my_auto(mut self) -> Self {
        self.layout.margin.top = LengthPercentageAuto::auto();
        self.layout.margin.bottom = LengthPercentageAuto::auto();
        self
    }

    pub fn mt_auto(mut self) -> Self {
        self.layout.margin.top = LengthPercentageAuto::auto();
        self
    }

    pub fn mr_auto(mut self) -> Self {
        self.layout.margin.right = LengthPercentageAuto::auto();
        self
    }

    pub fn mb_auto(mut self) -> Self {
        self.layout.margin.bottom = LengthPercentageAuto::auto();
        self
    }

    pub fn ml_auto(mut self) -> Self {
        self.layout.margin.left = LengthPercentageAuto::auto();
        self
    }

    spacing_scale_methods!(m;
        m_0 => 0.0, m_1 => 1.0, m_2 => 2.0, m_3 => 3.0, m_4 => 4.0, m_5 => 5.0,
        m_6 => 6.0, m_8 => 8.0, m_10 => 10.0, m_12 => 12.0, m_16 => 16.0,
        m_20 => 20.0, m_24 => 24.0, m_32 => 32.0,
    );
    spacing_scale_methods!(mx;
        mx_0 => 0.0, mx_1 => 1.0, mx_2 => 2.0, mx_3 => 3.0, mx_4 => 4.0, mx_5 => 5.0,
        mx_6 => 6.0, mx_8 => 8.0, mx_10 => 10.0, mx_12 => 12.0, mx_16 => 16.0,
        mx_20 => 20.0, mx_24 => 24.0, mx_32 => 32.0,
    );
    spacing_scale_methods!(my;
        my_0 => 0.0, my_1 => 1.0, my_2 => 2.0, my_3 => 3.0, my_4 => 4.0, my_5 => 5.0,
        my_6 => 6.0, my_8 => 8.0, my_10 => 10.0, my_12 => 12.0, my_16 => 16.0,
        my_20 => 20.0, my_24 => 24.0, my_32 => 32.0,
    );
    spacing_scale_methods!(mt;
        mt_0 => 0.0, mt_1 => 1.0, mt_2 => 2.0, mt_3 => 3.0, mt_4 => 4.0, mt_5 => 5.0,
        mt_6 => 6.0, mt_8 => 8.0, mt_10 => 10.0, mt_12 => 12.0, mt_16 => 16.0,
        mt_20 => 20.0, mt_24 => 24.0, mt_32 => 32.0,
    );
    spacing_scale_methods!(mr;
        mr_0 => 0.0, mr_1 => 1.0, mr_2 => 2.0, mr_3 => 3.0, mr_4 => 4.0, mr_5 => 5.0,
        mr_6 => 6.0, mr_8 => 8.0, mr_10 => 10.0, mr_12 => 12.0, mr_16 => 16.0,
        mr_20 => 20.0, mr_24 => 24.0, mr_32 => 32.0,
    );
    spacing_scale_methods!(mb;
        mb_0 => 0.0, mb_1 => 1.0, mb_2 => 2.0, mb_3 => 3.0, mb_4 => 4.0, mb_5 => 5.0,
        mb_6 => 6.0, mb_8 => 8.0, mb_10 => 10.0, mb_12 => 12.0, mb_16 => 16.0,
        mb_20 => 20.0, mb_24 => 24.0, mb_32 => 32.0,
    );
    spacing_scale_methods!(ml;
        ml_0 => 0.0, ml_1 => 1.0, ml_2 => 2.0, ml_3 => 3.0, ml_4 => 4.0, ml_5 => 5.0,
        ml_6 => 6.0, ml_8 => 8.0, ml_10 => 10.0, ml_12 => 12.0, ml_16 => 16.0,
        ml_20 => 20.0, ml_24 => 24.0, ml_32 => 32.0,
    );

    pub fn size(mut self, width: f32, height: f32) -> Self {
        self.layout.size = TaffySize {
            width: Dimension::length(width),
            height: Dimension::length(height),
        };
        self
    }

    pub fn size_full(mut self) -> Self {
        self.layout.size = TaffySize {
            width: Dimension::percent(1.0),
            height: Dimension::percent(1.0),
        };
        self
    }

    pub fn w(mut self, width: f32) -> Self {
        self.layout.size.width = Dimension::length(width);
        self
    }

    pub fn h(mut self, height: f32) -> Self {
        self.layout.size.height = Dimension::length(height);
        self
    }

    pub fn w_full(mut self) -> Self {
        self.layout.size.width = Dimension::percent(1.0);
        self
    }

    pub fn h_full(mut self) -> Self {
        self.layout.size.height = Dimension::percent(1.0);
        self
    }

    pub fn min_w(mut self, width: f32) -> Self {
        self.layout.min_size.width = Dimension::length(width);
        self
    }

    pub fn min_h(mut self, height: f32) -> Self {
        self.layout.min_size.height = Dimension::length(height);
        self
    }

    pub fn max_w(mut self, width: f32) -> Self {
        self.layout.max_size.width = Dimension::length(width);
        self
    }

    pub fn max_h(mut self, height: f32) -> Self {
        self.layout.max_size.height = Dimension::length(height);
        self
    }

    pub fn h_8(self) -> Self {
        self.h(SPACING_UNIT * 8.0)
    }
    pub fn h_10(self) -> Self {
        self.h(SPACING_UNIT * 10.0)
    }
    pub fn h_12(self) -> Self {
        self.h(SPACING_UNIT * 12.0)
    }

    pub fn p(self, value: f32) -> Self {
        self.padding(value, value, value, value)
    }

    pub fn px(mut self, value: f32) -> Self {
        self.layout.padding.left = LengthPercentage::length(value);
        self.layout.padding.right = LengthPercentage::length(value);
        self
    }

    pub fn py(mut self, value: f32) -> Self {
        self.layout.padding.top = LengthPercentage::length(value);
        self.layout.padding.bottom = LengthPercentage::length(value);
        self
    }

    pub fn p_1(self) -> Self {
        self.p(SPACING_UNIT)
    }
    pub fn p_2(self) -> Self {
        self.p(SPACING_UNIT * 2.0)
    }
    pub fn p_3(self) -> Self {
        self.p(SPACING_UNIT * 3.0)
    }
    pub fn p_4(self) -> Self {
        self.p(SPACING_UNIT * 4.0)
    }
    pub fn p_5(self) -> Self {
        self.p(SPACING_UNIT * 5.0)
    }
    pub fn p_6(self) -> Self {
        self.p(SPACING_UNIT * 6.0)
    }
    pub fn px_2(self) -> Self {
        self.px(SPACING_UNIT * 2.0)
    }
    pub fn px_3(self) -> Self {
        self.px(SPACING_UNIT * 3.0)
    }
    pub fn px_4(self) -> Self {
        self.px(SPACING_UNIT * 4.0)
    }
    pub fn py_1(self) -> Self {
        self.py(SPACING_UNIT)
    }
    pub fn py_2(self) -> Self {
        self.py(SPACING_UNIT * 2.0)
    }

    pub fn padding(mut self, top: f32, right: f32, bottom: f32, left: f32) -> Self {
        self.layout.padding = TaffyRect {
            left: LengthPercentage::length(left),
            right: LengthPercentage::length(right),
            top: LengthPercentage::length(top),
            bottom: LengthPercentage::length(bottom),
        };
        self
    }

    pub fn padding_axis(mut self, vertical: f32, horizontal: f32) -> Self {
        self.layout.padding = TaffyRect {
            left: LengthPercentage::length(horizontal),
            right: LengthPercentage::length(horizontal),
            top: LengthPercentage::length(vertical),
            bottom: LengthPercentage::length(vertical),
        };
        self
    }

    pub fn bg(mut self, color: Color) -> Self {
        self.visual.background = Some(color);
        self
    }

    /// Set the opacity of this element and all of its descendants.
    ///
    /// Opacity is paint-only: transparent elements keep their layout, pointer behavior, focus,
    /// and accessibility semantics. Nested opacity values multiply like GPUI and the web.
    pub fn opacity(mut self, opacity: f32) -> Self {
        self.visual.opacity = finite_opacity(opacity);
        self
    }

    pub fn border(mut self, width: f32, color: Color) -> Self {
        let width = width.max(0.0);
        self.visual.border_width = width;
        self.visual.border_color = Some(color);
        self.layout.border = TaffyRect {
            left: LengthPercentage::length(width),
            right: LengthPercentage::length(width),
            top: LengthPercentage::length(width),
            bottom: LengthPercentage::length(width),
        };
        self
    }

    pub fn rounded(mut self, radius: f32) -> Self {
        self.visual.radius = radius.max(0.0);
        self
    }

    pub fn rounded_sm(self) -> Self {
        self.rounded(4.0)
    }
    pub fn rounded_md(self) -> Self {
        self.rounded(6.0)
    }
    pub fn rounded_lg(self) -> Self {
        self.rounded(8.0)
    }
    pub fn rounded_xl(self) -> Self {
        self.rounded(12.0)
    }
    pub fn rounded_2xl(self) -> Self {
        self.rounded(16.0)
    }

    /// Paint one CSS-like box shadow without affecting layout.
    pub fn shadow(mut self, shadow: BoxShadow) -> Self {
        self.visual.shadows = Some(Arc::from([shadow]));
        self
    }

    /// Paint multiple CSS-like box shadows in declaration order.
    ///
    /// As on the web, the first shadow is painted on top of later shadows.
    pub fn shadows(mut self, shadows: impl IntoIterator<Item = BoxShadow>) -> Self {
        self.visual.shadows = Some(collect_box_shadows(shadows));
        self
    }

    pub fn shadow_none(mut self) -> Self {
        self.visual.shadows = None;
        self
    }

    pub fn shadow_sm(self) -> Self {
        self.shadow(shadow_sm_preset())
    }

    pub fn shadow_md(self) -> Self {
        self.shadows(shadow_md_preset())
    }

    pub fn shadow_lg(self) -> Self {
        self.shadows(shadow_lg_preset())
    }

    pub fn shadow_xl(self) -> Self {
        self.shadows(shadow_xl_preset())
    }

    pub fn shadow_2xl(self) -> Self {
        self.shadow(shadow_2xl_preset())
    }

    /// Choose how image, SVG, or path content is fitted into its layout box.
    pub fn object_fit(mut self, fit: ObjectFit) -> Self {
        match &mut self.kind {
            ElementKind::Image(image) => image.object_fit = fit,
            ElementKind::Svg(svg) => svg.object_fit = fit,
            ElementKind::Path(path) => path.object_fit = fit,
            _ => {}
        }
        self
    }

    /// Override a path element's inherited text color with a solid color or linear gradient.
    pub fn path_background(mut self, background: impl Into<Background>) -> Self {
        if let ElementKind::Path(path) = &mut self.kind {
            path.background = Some(background.into());
        }
        self
    }

    /// Replace the four parameter vectors supplied to this custom shader instance.
    pub fn shader_parameters(mut self, parameters: impl Into<ShaderParameters>) -> Self {
        let ElementKind::CustomShader(shader) = &mut self.kind else {
            panic!("shader_parameters can only be applied to a custom shader element");
        };
        shader.parameters = parameters.into();
        self
    }

    /// Apply a render-only transform to SVG content without affecting flexbox layout.
    pub fn svg_transform(mut self, transform: SvgTransform) -> Self {
        if let ElementKind::Svg(svg) = &mut self.kind {
            svg.transform = transform;
        }
        self
    }

    /// Render an image in grayscale without creating another decoded image.
    pub fn grayscale(mut self, grayscale: bool) -> Self {
        if let ElementKind::Image(image) = &mut self.kind {
            image.grayscale = grayscale;
        }
        self
    }

    /// Render a replacement element when an image resource has been loading for 200 ms.
    pub fn with_loading<E: IntoElement + 'static>(
        mut self,
        render: impl Fn() -> E + 'static,
    ) -> Self {
        if let ElementKind::Image(image) = &mut self.kind {
            image.loading = Some(ImageReplacement::new(render));
        }
        self
    }

    /// Render a replacement element when an image resource fails to load.
    pub fn with_fallback<E: IntoElement + 'static>(
        mut self,
        render: impl Fn() -> E + 'static,
    ) -> Self {
        if let ElementKind::Image(image) = &mut self.kind {
            image.fallback = Some(ImageReplacement::new(render));
        }
        self
    }

    pub fn text_color(mut self, color: Color) -> Self {
        self.typography.color = Some(color);
        self
    }

    pub fn text_size(mut self, size: f32) -> Self {
        self.typography.font_size = Some(size.max(1.0));
        self
    }

    pub fn line_height(mut self, height: f32) -> Self {
        self.typography.line_height = Some(height.max(1.0));
        self
    }

    pub fn text_xs(self) -> Self {
        self.text_size(12.0).line_height(16.0)
    }
    pub fn text_sm(self) -> Self {
        self.text_size(14.0).line_height(20.0)
    }
    pub fn text_base(self) -> Self {
        self.text_size(16.0).line_height(24.0)
    }
    pub fn text_lg(self) -> Self {
        self.text_size(18.0).line_height(26.0)
    }
    pub fn text_xl(self) -> Self {
        self.text_size(20.0).line_height(28.0)
    }
    pub fn text_2xl(self) -> Self {
        self.text_size(24.0).line_height(32.0)
    }
    pub fn text_3xl(self) -> Self {
        self.text_size(30.0).line_height(36.0)
    }

    pub fn font_weight(mut self, weight: Weight) -> Self {
        self.typography.weight = Some(weight);
        self
    }

    pub fn font_normal(self) -> Self {
        self.font_weight(Weight::NORMAL)
    }

    pub fn font_medium(self) -> Self {
        self.font_weight(Weight::MEDIUM)
    }

    pub fn font_semibold(self) -> Self {
        self.font_weight(Weight::SEMIBOLD)
    }

    pub fn font_bold(self) -> Self {
        self.font_weight(Weight::BOLD)
    }

    /// Set the inherited font slant.
    pub fn font_style(mut self, style: GlyphStyle) -> Self {
        self.typography.font_style = Some(style);
        self
    }

    pub fn italic(self) -> Self {
        self.font_style(GlyphStyle::Italic)
    }

    pub fn not_italic(self) -> Self {
        self.font_style(GlyphStyle::Normal)
    }

    /// Underline descendant text with the font's native single-line metrics.
    pub fn underline(mut self) -> Self {
        self.typography.underline = Some(TextUnderline::Single);
        self.typography.underline_wavy = Some(false);
        self.typography.underline_thickness = Some(1.0);
        self
    }

    /// Underline descendant text with two native lines.
    pub fn double_underline(mut self) -> Self {
        self.typography.underline = Some(TextUnderline::Double);
        self.typography.underline_wavy = Some(false);
        self.typography.underline_thickness = Some(1.0);
        self
    }

    /// Strike through descendant text with the font's native metrics.
    pub fn line_through(mut self) -> Self {
        self.typography.strikethrough = Some(true);
        self
    }

    /// Remove inherited underline and strikethrough decoration.
    pub fn text_decoration_none(mut self) -> Self {
        self.typography.underline = Some(TextUnderline::None);
        self.typography.underline_color = Some(None);
        self.typography.underline_wavy = Some(false);
        self.typography.underline_thickness = Some(1.0);
        self.typography.strikethrough = Some(false);
        self.typography.strikethrough_color = Some(None);
        self
    }

    /// Set the inherited underline color, enabling a single underline when needed.
    pub fn text_decoration_color(mut self, color: Color) -> Self {
        if self
            .typography
            .underline
            .is_none_or(|underline| underline == TextUnderline::None)
        {
            self.typography.underline = Some(TextUnderline::Single);
            self.typography.underline_wavy = Some(false);
            self.typography.underline_thickness = Some(1.0);
        }
        self.typography.underline_color = Some(Some(color));
        self
    }

    /// Use an ordinary solid underline, enabling one when no underline is inherited.
    pub fn text_decoration_solid(mut self) -> Self {
        let enables_underline = self
            .typography
            .underline
            .is_none_or(|underline| underline == TextUnderline::None);
        if enables_underline {
            self.typography.underline = Some(TextUnderline::Single);
            self.typography.underline_thickness = Some(1.0);
        }
        self.typography.underline_wavy = Some(false);
        self
    }

    /// Use a GPU-rendered spell-checker-style wavy underline.
    pub fn text_decoration_wavy(mut self) -> Self {
        let enables_underline = self
            .typography
            .underline
            .is_none_or(|underline| underline == TextUnderline::None);
        if enables_underline {
            self.typography.underline = Some(TextUnderline::Single);
            self.typography.underline_thickness = Some(1.0);
        }
        self.typography.underline_wavy = Some(true);
        self
    }

    fn text_decoration_thickness(mut self, thickness: f32) -> Self {
        if self
            .typography
            .underline
            .is_none_or(|underline| underline == TextUnderline::None)
        {
            self.typography.underline = Some(TextUnderline::Single);
        }
        self.typography.underline_thickness = Some(thickness);
        self
    }

    pub fn text_decoration_0(self) -> Self {
        self.text_decoration_thickness(0.0)
    }

    pub fn text_decoration_1(self) -> Self {
        self.text_decoration_thickness(1.0)
    }

    pub fn text_decoration_2(self) -> Self {
        self.text_decoration_thickness(2.0)
    }

    pub fn text_decoration_4(self) -> Self {
        self.text_decoration_thickness(4.0)
    }

    pub fn text_decoration_8(self) -> Self {
        self.text_decoration_thickness(8.0)
    }

    pub fn font_family(mut self, family: impl Into<FontFamily>) -> Self {
        let family = family.into();
        assert_valid_font_family(&family);
        self.typography.family = Some(family);
        self
    }

    /// Set the inherited OpenType feature table used while shaping descendant text.
    pub fn font_features(mut self, features: FontFeatures) -> Self {
        self.typography.features = Some(features);
        self
    }

    /// Set the ordered families tried after the primary family and before platform fallbacks.
    ///
    /// Passing an empty stack intentionally clears an inherited custom fallback stack.
    pub fn font_fallbacks(mut self, fallbacks: FontFallbacks) -> Self {
        self.typography.fallbacks = Some((!fallbacks.is_empty()).then_some(fallbacks));
        self
    }

    /// Replace the complete inherited font configuration.
    pub fn font(mut self, font: Font) -> Self {
        assert_valid_font_family(&font.family);
        self.typography.family = Some(font.family);
        self.typography.features = Some(font.features);
        self.typography.fallbacks = Some(normalize_fallbacks(font.fallbacks));
        self.typography.weight = Some(font.weight);
        self.typography.font_style = Some(font.style);
        self
    }

    /// Align descendant text lines within their assigned element width.
    pub fn text_align(mut self, align: TextAlign) -> Self {
        self.typography.align = Some(align);
        self
    }

    pub fn text_left(self) -> Self {
        self.text_align(TextAlign::Left)
    }

    pub fn text_center(self) -> Self {
        self.text_align(TextAlign::Center)
    }

    pub fn text_right(self) -> Self {
        self.text_align(TextAlign::Right)
    }

    pub fn text_justify(self) -> Self {
        self.text_align(TextAlign::Justify)
    }

    pub fn no_wrap(mut self) -> Self {
        self.typography.wrap = Some(TextWrap::None);
        self
    }

    /// Set inherited CSS-like whitespace behavior.
    pub fn white_space(mut self, white_space: WhiteSpace) -> Self {
        self.typography.wrap = Some(white_space.into());
        self
    }

    /// Allow ordinary word wrapping, matching GPUI and `white-space: normal` on the web.
    pub fn whitespace_normal(self) -> Self {
        self.white_space(WhiteSpace::Normal)
    }

    /// Keep each logical line unwrapped, matching GPUI and `white-space: nowrap` on the web.
    pub fn whitespace_nowrap(self) -> Self {
        self.white_space(WhiteSpace::Nowrap)
    }

    /// Set the replacement used when text exceeds its assigned width.
    pub fn text_overflow(mut self, overflow: TextOverflow) -> Self {
        self.typography.text_overflow = Some(overflow);
        self
    }

    /// Preserve the start of overflowing text and append an ellipsis.
    pub fn text_ellipsis(self) -> Self {
        self.text_overflow(TextOverflow::ellipsis())
    }

    /// Preserve the end of overflowing text and prepend an ellipsis.
    pub fn text_ellipsis_start(self) -> Self {
        self.text_overflow(TextOverflow::ellipsis_start())
    }

    /// Preserve both ends of overflowing text and place an ellipsis in the middle.
    pub fn text_ellipsis_middle(self) -> Self {
        self.text_overflow(TextOverflow::ellipsis_middle())
    }

    /// Limit descendant text leaves to a fixed number of visible lines.
    ///
    /// Combine this with [`Self::text_ellipsis`] when the final visible line should carry an
    /// ellipsis. Values below one are normalized to one.
    pub fn line_clamp(mut self, lines: usize) -> Self {
        self.typography.line_clamp = Some(lines.max(1));
        self.overflow_hidden()
    }

    /// Web-style single-line truncation: no wrapping, hidden overflow, and a trailing ellipsis.
    pub fn truncate(self) -> Self {
        self.overflow_hidden().whitespace_nowrap().text_ellipsis()
    }

    /// Use cheap one-glyph-per-character shaping for app-controlled text and fonts.
    ///
    /// Keep the default advanced mode for complex scripts, ligatures, or general font fallback.
    pub fn text_shaping_basic(mut self) -> Self {
        self.typography.shaping = Some(TextShaping::Basic);
        self
    }

    pub fn text_shaping(mut self, shaping: TextShaping) -> Self {
        self.typography.shaping = Some(shaping);
        self
    }

    pub fn wrap(mut self) -> Self {
        self.typography.wrap = Some(TextWrap::Word);
        self
    }

    pub fn overflow_hidden(mut self) -> Self {
        self.layout.overflow = TaffyPoint {
            x: Overflow::Hidden,
            y: Overflow::Hidden,
        };
        self
    }

    pub fn overflow_y_scroll(mut self) -> Self {
        self.layout.overflow = TaffyPoint {
            x: Overflow::Hidden,
            y: Overflow::Scroll,
        };
        self
    }

    /// Follow the end of an ordinary vertical overflow container when `revision` changes.
    ///
    /// The first declaration starts at the end. Later revisions keep following only while the
    /// user was already at the previous end; scrolling away pauses following, and returning to
    /// the end resumes it on the next revision. This performs no scheduling or per-frame work.
    pub fn scroll_to_end(mut self, revision: u64) -> Self {
        self.scroll_to_end_revision = Some(revision);
        self
    }

    /// Bind this clipped viewport to a fixed-height [`VirtualList`].
    ///
    /// The mounted rows remain application-controlled, while QuickGUI owns the native-style
    /// retained scrollbar, wheel routing, pointer capture, hover expansion, and one-shot
    /// autohide. Offset changes rebuild the virtualized view only when the offset actually moves;
    /// hover and visibility changes stay paint-only.
    pub fn virtual_scroll(mut self, list: &VirtualList) -> Self {
        self.layout.overflow.y = Overflow::Hidden;
        self.virtual_scroll = Some(VirtualScrollStyle {
            handle: list.scroll_handle(),
            max_offset_y: list.max_scroll_offset(),
            measurement_revision: 0,
            mount: list.scroll_mount(),
        });
        self
    }

    /// Bind this clipped viewport to a differently sized [`ListState`].
    ///
    /// Use [`ListState::visible_rows`] and [`ListState::render_rows`] to mount a bounded normal-flow
    /// slice. QuickGUI measures those rows after Taffy layout, preserves the logical top item while
    /// estimates converge, and schedules only the correcting rebuilds that actually changed a
    /// measurement. Wheel input, scrollbar capture, hover expansion, and autohide use the same
    /// retained path as ordinary scrolling and fixed-height virtualization.
    pub fn variable_virtual_scroll(mut self, list: &ListState) -> Self {
        self.layout.overflow.y = Overflow::Hidden;
        let (handle, max_offset_y, measurement_revision, mount) = list.scroll_binding();
        self.virtual_scroll = Some(VirtualScrollStyle {
            handle,
            max_offset_y,
            measurement_revision,
            mount,
        });
        self
    }

    pub fn absolute(mut self) -> Self {
        self.layout.position = Position::Absolute;
        self
    }

    /// Create a stacking context within the current render plane.
    ///
    /// Values are relative to the nearest ancestor stacking context. Elements sharing a value stay
    /// batched and retain source order.
    pub fn z_index(mut self, value: i16) -> Self {
        self.z_index = Some(value);
        self
    }

    /// Paint this subtree in the viewport overlay plane above ordinary application content.
    ///
    /// The element becomes absolutely positioned, escapes ancestor clipping, and blocks pointer
    /// events from falling through its own bounds.
    pub fn overlay(mut self) -> Self {
        self.layout.position = Position::Absolute;
        self.plane = Some(ScenePlane::Overlay);
        self.portal = true;
        self.blocks_pointer = true;
        self
    }

    /// Position this floating element relative to a stable element ID.
    pub fn anchor_to(mut self, target: impl Into<ElementId>, placement: AnchorPlacement) -> Self {
        self = self.overlay();
        self.anchor = Some(AnchorStyle {
            target: AnchorTarget::Element(target.into()),
            placement,
            gap: DEFAULT_ANCHOR_GAP,
            viewport_margin: DEFAULT_VIEWPORT_MARGIN,
        });
        self
    }

    /// Position a viewport overlay relative to a logical point.
    ///
    /// This is the cursor-point counterpart of [`Self::anchor_to`] and is intended for context
    /// menus. Placement uses the same flip, alternate-alignment, and viewport-clamping rules.
    pub fn anchor_at(mut self, point: crate::Point, placement: AnchorPlacement) -> Self {
        self = self.overlay();
        let point = crate::Point::new(
            if point.x.is_finite() { point.x } else { 0.0 },
            if point.y.is_finite() { point.y } else { 0.0 },
        );
        self.anchor = Some(AnchorStyle {
            target: AnchorTarget::Point(point),
            placement,
            gap: 0.0,
            viewport_margin: DEFAULT_VIEWPORT_MARGIN,
        });
        self
    }

    /// Set the distance between an anchored surface and its trigger.
    pub fn anchor_gap(mut self, gap: f32) -> Self {
        if let Some(anchor) = &mut self.anchor {
            anchor.gap = gap.max(0.0);
        }
        self
    }

    /// Set the minimum distance between an anchored surface and the content viewport edge.
    pub fn viewport_margin(mut self, margin: f32) -> Self {
        if let Some(anchor) = &mut self.anchor {
            anchor.viewport_margin = margin.max(0.0);
        }
        self
    }

    /// Show a delayed, pointer-passive GPU tooltip while this element is hovered.
    ///
    /// The detached tooltip tree is laid out only after its exact delay expires. Entering a
    /// pending tooltip schedules no frame loop, and moving between ordinary points inside the
    /// trigger does not rebuild the application view.
    pub fn tooltip(mut self, tooltip: impl Into<Tooltip>) -> Self {
        let tooltip = tooltip.into();
        if self.accessibility.description.is_none() {
            self.accessibility.description = tooltip.accessibility_description.clone();
        }
        self.tooltip = Some(tooltip);
        self
    }

    /// Prevent pointer events inside this element from reaching lower visual layers.
    pub fn block_pointer(mut self) -> Self {
        self.blocks_pointer = true;
        self
    }

    /// Set web-style native window dragging behavior for this element's layout box.
    pub fn app_region(mut self, region: AppRegion) -> Self {
        self.app_region = Some(region);
        self
    }

    /// Make this element a native window drag region.
    pub fn app_region_drag(self) -> Self {
        self.app_region(AppRegion::Drag)
    }

    /// Restore normal pointer input inside an ancestor window drag region.
    pub fn app_region_no_drag(self) -> Self {
        self.app_region(AppRegion::NoDrag)
    }

    pub fn relative(mut self) -> Self {
        self.layout.position = Position::Relative;
        self
    }

    pub fn top(mut self, value: f32) -> Self {
        self.layout.inset.top = LengthPercentageAuto::length(value);
        self
    }

    pub fn right(mut self, value: f32) -> Self {
        self.layout.inset.right = LengthPercentageAuto::length(value);
        self
    }

    pub fn bottom(mut self, value: f32) -> Self {
        self.layout.inset.bottom = LengthPercentageAuto::length(value);
        self
    }

    pub fn left(mut self, value: f32) -> Self {
        self.layout.inset.left = LengthPercentageAuto::length(value);
        self
    }

    pub fn inset_0(mut self) -> Self {
        let zero = LengthPercentageAuto::length(0.0);
        self.layout.inset = TaffyRect {
            left: zero,
            right: zero,
            top: zero,
            bottom: zero,
        };
        self
    }

    pub fn hover(mut self, style: impl FnOnce(ElementStateStyle) -> ElementStateStyle) -> Self {
        self.hover = style(ElementStateStyle::default());
        self
    }

    pub fn active(mut self, style: impl FnOnce(ElementStateStyle) -> ElementStateStyle) -> Self {
        self.active = style(ElementStateStyle::default());
        self
    }

    /// Smooth supported paint-only style changes under one retained element identity.
    ///
    /// A [`Duration`](std::time::Duration) converts to an all-property [`Transition`]. Layout
    /// values use [`crate::AnimationExt`] instead because they require a declaration rebuild and
    /// Taffy layout.
    pub fn transition(mut self, transition: impl Into<Transition>) -> Self {
        self.transition = Some(transition.into());
        self
    }

    /// Smooth only background, border, and inherited text-color changes.
    pub fn transition_colors(mut self, duration: std::time::Duration) -> Self {
        self.transition = Some(Transition::colors(duration));
        self
    }

    /// Paint-only styling while this element owns keyboard focus.
    pub fn focus(mut self, style: impl FnOnce(ElementStateStyle) -> ElementStateStyle) -> Self {
        self.focus = style(ElementStateStyle::default());
        self
    }

    /// Paint-only styling while this element is the source of an active internal drag.
    pub fn dragging(mut self, style: impl FnOnce(ElementStateStyle) -> ElementStateStyle) -> Self {
        self.dragging = style(ElementStateStyle::default());
        self
    }

    /// Paint-only styling while a compatible typed payload is over this drop target.
    pub fn drag_over(mut self, style: impl FnOnce(ElementStateStyle) -> ElementStateStyle) -> Self {
        self.drag_over = style(ElementStateStyle::default());
        self
    }

    pub fn accessibility_role(mut self, role: AccessibilityRole) -> Self {
        self.accessibility.role = role;
        self
    }

    pub fn accessibility_label(mut self, label: impl Into<Arc<str>>) -> Self {
        self.accessibility.label = Some(label.into());
        self
    }

    pub fn accessibility_value(mut self, value: impl Into<Arc<str>>) -> Self {
        self.accessibility.value = Some(value.into());
        self
    }

    /// Set placeholder text for a [`text_input`] element.
    pub fn placeholder(mut self, placeholder: impl Into<Arc<str>>) -> Self {
        if let ElementKind::TextInput(input) = &mut self.kind {
            input.placeholder = placeholder.into();
        }
        self
    }

    /// Mask the visible value and expose native secure-text-field semantics.
    ///
    /// Password inputs retain their real controlled value for editing and submit listeners, but
    /// paint one bullet per Unicode grapheme and do not expose selections to clipboard actions.
    /// Calling this with `false` restores an ordinary single-line text input, which supports
    /// web-style reveal buttons without replacing the retained input state.
    pub fn password(mut self, password: bool) -> Self {
        let ElementKind::TextInput(input) = &mut self.kind else {
            panic!("password can only be applied to a text input");
        };
        assert!(
            !password || !input.multiline,
            "a text area cannot be a password input"
        );
        input.password = password;
        self.accessibility.role = if password {
            AccessibilityRole::PasswordInput
        } else {
            AccessibilityRole::TextInput
        };
        self
    }

    /// Limit user edits to at most this many Unicode grapheme clusters.
    ///
    /// Pasted and committed IME text is truncated at a grapheme boundary before the input filter
    /// runs. Controlled values supplied by the application remain authoritative and are not
    /// rewritten during rendering.
    pub fn max_length(mut self, length: usize) -> Self {
        let ElementKind::TextInput(input) = &mut self.kind else {
            panic!("max_length can only be applied to a text input or text area");
        };
        input.constraints.max_length = Some(length);
        self
    }

    /// Accept or reject a proposed complete value before retained text and history are mutated.
    ///
    /// The callback runs only for edit attempts—not during paint, layout, pointer movement, or
    /// controlled-value synchronization. Returning `false` rejects typing, paste, IME commit,
    /// accessibility value changes, and undo/redo consistently.
    pub fn input_filter(mut self, filter: impl Fn(&str) -> bool + 'static) -> Self {
        let ElementKind::TextInput(input) = &mut self.kind else {
            panic!("input_filter can only be applied to a text input or text area");
        };
        assert!(
            input.constraints.filter.is_none(),
            "input_filter was registered more than once on one element"
        );
        input.constraints.filter = Some(Arc::new(filter));
        self
    }

    /// Expose that a form control requires a value before submission.
    ///
    /// This projects the native accessibility state only. The application remains responsible for
    /// deriving [`Self::invalid`] and its validation message from the controlled value.
    pub fn required(mut self, required: bool) -> Self {
        self.accessibility.required = required;
        self
    }

    /// Expose web-style invalid state to paint and the native accessibility tree.
    pub fn invalid(mut self, invalid: bool) -> Self {
        self.accessibility.invalid = invalid;
        self
    }

    /// Describe why an invalid control cannot currently be submitted.
    ///
    /// Call [`Self::invalid`] separately so clearing or replacing a message never changes validity
    /// accidentally.
    pub fn validation_message(mut self, message: impl Into<Arc<str>>) -> Self {
        let (message, truncated) = bounded_validation_message(message.into());
        self.accessibility.validation_message = message;
        self.accessibility.validation_message_truncated = truncated;
        self
    }

    pub(crate) fn validation_message_retained(
        mut self,
        message: Arc<str>,
        truncated: bool,
    ) -> Self {
        debug_assert!(message.len() <= MAX_VALIDATION_MESSAGE_BYTES);
        self.accessibility.validation_message = (!message.is_empty()).then_some(message);
        self.accessibility.validation_message_truncated = truncated;
        self
    }

    /// Set a native accessibility description independently of visible text.
    pub fn accessibility_description(mut self, description: impl Into<Arc<str>>) -> Self {
        let description = description.into();
        self.accessibility.description = (!description.is_empty()).then_some(description);
        self
    }

    /// Paint-only styling while this element is marked invalid.
    pub fn invalid_style(
        mut self,
        style: impl FnOnce(ElementStateStyle) -> ElementStateStyle,
    ) -> Self {
        self.invalid_style = style(ElementStateStyle::default());
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.accessibility.disabled = disabled;
        self
    }

    /// Paint-only styling while this element is disabled.
    ///
    /// Disabled elements are already removed from pointer, keyboard, and accessibility actions;
    /// this method adds an optional visual treatment without changing layout.
    pub fn disabled_style(
        mut self,
        style: impl FnOnce(ElementStateStyle) -> ElementStateStyle,
    ) -> Self {
        self.disabled_style = style(ElementStateStyle::default());
        self
    }

    pub fn selected(mut self, selected: bool) -> Self {
        self.accessibility.selected = selected;
        self
    }

    /// Hide this element and its complete subtree from the native accessibility tree.
    ///
    /// Painting, layout, pointer input, and keyboard behavior are unchanged. This is useful for
    /// a visual surface whose semantics are projected into another native window, such as a
    /// never-key autocomplete panel controlled by an owner-window text input.
    pub fn accessibility_hidden(mut self, hidden: bool) -> Self {
        self.accessibility.hidden = hidden;
        self
    }

    /// Expose whether a disclosure, popover, or similar controlled surface is expanded.
    pub fn accessibility_expanded(mut self, expanded: bool) -> Self {
        self.accessibility.expanded = Some(expanded);
        self
    }

    /// Relate this control to one stable element that it controls.
    ///
    /// The relationship is projected only when the target is present in the mounted
    /// accessibility tree. This avoids dangling native node references for closed controlled
    /// popovers whose content is intentionally unmounted.
    pub fn accessibility_controls(mut self, target: impl Into<ElementId>) -> Self {
        self.accessibility.relations.set_controls(target.into());
        self
    }

    /// Identify the mounted active option, row, cell, or tree item for a composite control.
    ///
    /// Keep DOM-style keyboard focus on the composite root and update this relationship as its
    /// controlled selection moves. The native relationship is omitted while the target is not
    /// mounted, which is important for virtualized collections.
    pub fn accessibility_active_descendant(mut self, target: impl Into<ElementId>) -> Self {
        self.accessibility
            .relations
            .set_active_descendant(target.into());
        self
    }

    /// Use one mounted element as this element's accessible label.
    ///
    /// The relationship is projected only while the target is present and is omitted for a
    /// self-reference. Prefer this to copying visible group, field, or section text into a second
    /// accessibility-only string.
    pub fn accessibility_labelled_by(mut self, target: impl Into<ElementId>) -> Self {
        self.accessibility.relations.set_labelled_by(target.into());
        self
    }

    /// Use one mounted element as this element's accessible description.
    ///
    /// Dangling and self-referential relationships are omitted from the native tree, matching
    /// [`Self::accessibility_labelled_by`].
    pub fn accessibility_described_by(mut self, target: impl Into<ElementId>) -> Self {
        self.accessibility.relations.set_described_by(target.into());
        self
    }

    pub(crate) fn accessibility_described_by_pair(
        mut self,
        first: impl Into<ElementId>,
        second: impl Into<ElementId>,
    ) -> Self {
        self.accessibility
            .relations
            .set_described_by_pair(first.into(), second.into());
        self
    }

    /// Describe the kind of popup opened by this control.
    pub fn accessibility_has_popup(mut self, popup: AccessibilityPopup) -> Self {
        self.accessibility.has_popup = Some(popup);
        self
    }

    /// Describe how an editable control presents completion suggestions.
    pub fn accessibility_auto_complete(mut self, behavior: AccessibilityAutoComplete) -> Self {
        self.accessibility.auto_complete = Some(behavior);
        self
    }

    /// Mark a dialog or alert-dialog as explicitly modal for assistive technology.
    pub fn accessibility_modal(mut self, modal: bool) -> Self {
        self.accessibility.modal = modal;
        self
    }

    /// Expose the complete logical row count for a table or grid, including unmounted rows.
    pub fn accessibility_row_count(mut self, count: usize) -> Self {
        self.accessibility.collection.row_count = accessibility_collection_storage(count);
        self
    }

    /// Expose the complete logical column count for a table or grid.
    pub fn accessibility_column_count(mut self, count: usize) -> Self {
        self.accessibility.collection.column_count = accessibility_collection_storage(count);
        self
    }

    /// Set the zero-based logical row index for a row or cell.
    pub fn accessibility_row_index(mut self, index: usize) -> Self {
        self.accessibility.collection.row_index = accessibility_collection_storage(index);
        self
    }

    /// Set the zero-based logical column index for a header or cell.
    pub fn accessibility_column_index(mut self, index: usize) -> Self {
        self.accessibility.collection.column_index = accessibility_collection_storage(index);
        self
    }

    /// Set the zero-based nesting level for a hierarchical item.
    pub fn accessibility_level(mut self, level: usize) -> Self {
        self.accessibility.collection.level = accessibility_collection_storage(level);
        self
    }

    /// Expose the total logical sibling count for a virtualized collection item.
    pub fn accessibility_size_of_set(mut self, size: usize) -> Self {
        self.accessibility.collection.size_of_set = accessibility_collection_storage(size);
        self
    }

    /// Set the zero-based logical position among an item's siblings.
    pub fn accessibility_position_in_set(mut self, position: usize) -> Self {
        self.accessibility.collection.position_in_set = accessibility_collection_storage(position);
        self
    }

    /// Expose the active ordering of a sortable table or grid column.
    pub fn accessibility_sort_direction(mut self, direction: AccessibilitySortDirection) -> Self {
        self.accessibility.collection.sort_direction = Some(direction);
        self
    }

    /// Expose a controlled checked, unchecked, or mixed state to assistive technology.
    pub fn toggle_state(mut self, state: impl Into<ToggleState>) -> Self {
        self.accessibility.toggled = Some(state.into());
        self
    }

    /// Web-style boolean shorthand for [`Self::toggle_state`].
    pub fn checked(self, checked: bool) -> Self {
        self.toggle_state(checked)
    }

    /// Set or clear the web-style indeterminate checkbox state.
    ///
    /// Clearing indeterminate preserves an existing on/off state and maps a previously mixed
    /// state to off.
    pub fn indeterminate(mut self, indeterminate: bool) -> Self {
        self.accessibility.toggled = Some(if indeterminate {
            ToggleState::Mixed
        } else {
            match self.accessibility.toggled {
                Some(ToggleState::On) => ToggleState::On,
                _ => ToggleState::Off,
            }
        });
        self
    }

    /// Include this element in the window's focus path and Tab traversal.
    pub fn focusable(mut self) -> Self {
        self.focusable = true;
        self
    }

    /// Assign a stable focus identity to this element.
    pub fn track_focus(mut self, handle: FocusHandle) -> Self {
        self.bind_listener_id(handle.id());
        self.focusable = true;
        self
    }

    /// Give a non-focusable ancestor a stable identity for scoped action dispatch.
    ///
    /// Descendant focus is still tracked through this element, but the scope itself is not added to
    /// Tab traversal.
    pub fn focus_scope(mut self, handle: FocusHandle) -> Self {
        self.bind_listener_id(handle.id());
        self
    }

    /// Contain keyboard focus within this subtree while it is mounted and topmost.
    ///
    /// Nested traps are resolved by overlay plane, `z_index`, and declaration order. Only the
    /// topmost trap participates in Tab traversal or programmatic framework focus; mounting one
    /// moves focus to its first enabled Tab stop (or the trap root when it is focusable), and
    /// removing a focused child keeps focus inside the remaining trap. The marker retains no
    /// observer, timer, task, or idle scheduler source.
    pub fn focus_trap(mut self) -> Self {
        self.focus_trap = true;
        self
    }

    /// Attach contextual keymap properties to this node in the focused ancestor path.
    pub fn key_context(mut self, context: impl Into<KeyContext>) -> Self {
        self.key_context = Some(context.into());
        self
    }

    /// Set keyboard traversal order. Negative values remove the element from Tab traversal.
    pub fn tab_index(mut self, index: i16) -> Self {
        self.tab_index = index;
        self
    }

    /// Focus this element the first time it appears if the window has no focused element.
    pub fn auto_focus(mut self) -> Self {
        self.auto_focus = true;
        self.focusable = true;
        self
    }

    /// Include this node in click hit testing. Clicks arrive as [`crate::Event::Click`].
    pub fn clickable(mut self) -> Self {
        self.clickable = true;
        self.set_implicit_cursor(CursorStyle::PointingHand);
        self.focusable = true;
        if self.accessibility.role == AccessibilityRole::GenericContainer {
            self.accessibility.role = AccessibilityRole::Button;
        }
        self
    }

    /// Give an otherwise non-focusable label native click-to-activate behavior.
    ///
    /// This stays crate-private because arbitrary activation chains need cycle and ownership
    /// policy. The unstyled field layer uses one derived label-to-control edge.
    pub(crate) fn activate_target_on_click(mut self, target: impl Into<ElementId>) -> Self {
        self.activation_target = Some(target.into());
        self.clickable = true;
        self.set_implicit_cursor(CursorStyle::Arrow);
        self
    }

    /// Attach a listener registered by [`crate::ViewContext::listener`].
    pub fn on_click<V>(mut self, listener: crate::ClickListener<V>) -> Self {
        self.bind_listener_id(listener.id());
        self.clickable = true;
        self.set_implicit_cursor(CursorStyle::PointingHand);
        self.focusable = true;
        if self.accessibility.role == AccessibilityRole::GenericContainer {
            self.accessibility.role = AccessibilityRole::Button;
        }
        self
    }

    /// Capture pointer motion from press through release, including outside this element.
    ///
    /// Attach a listener registered by [`crate::ViewContext::pointer_listener`]. The element also
    /// occludes click and hover hit testing behind its bounds while allowing wheel scrolling.
    pub fn on_pointer<V>(mut self, listener: crate::PointerListener<V>) -> Self {
        assert!(
            !self.drag_source,
            "one element cannot own both a captured pointer listener and a typed drag source"
        );
        self.bind_listener_id(listener.id());
        self.pointer_listener = true;
        self
    }

    /// Handle a matching desktop mouse-button press during the bubble phase.
    pub fn on_mouse_down<V>(
        self,
        button: crate::MouseButton,
        listener: crate::MouseDownListener<V>,
    ) -> Self {
        self.bind_mouse_listener(
            listener.id(),
            MouseListenerBinding {
                key: listener.key(),
                kind: MouseListenerKind::Down,
                phase: DispatchPhase::Bubble,
                button: Some(button),
                outside: false,
            },
        )
    }

    /// Handle a desktop mouse-button press for any button during the bubble phase.
    pub fn on_any_mouse_down<V>(self, listener: crate::MouseDownListener<V>) -> Self {
        self.bind_mouse_listener(
            listener.id(),
            MouseListenerBinding {
                key: listener.key(),
                kind: MouseListenerKind::Down,
                phase: DispatchPhase::Bubble,
                button: None,
                outside: false,
            },
        )
    }

    /// Handle any desktop mouse-button press during capture, from the root toward the target.
    pub fn capture_any_mouse_down<V>(self, listener: crate::MouseDownListener<V>) -> Self {
        self.bind_mouse_listener(
            listener.id(),
            MouseListenerBinding {
                key: listener.key(),
                kind: MouseListenerKind::Down,
                phase: DispatchPhase::Capture,
                button: None,
                outside: false,
            },
        )
    }

    /// Handle a desktop mouse-button press outside this element during the capture stage.
    pub fn on_mouse_down_out<V>(self, listener: crate::MouseDownListener<V>) -> Self {
        self.bind_mouse_listener(
            listener.id(),
            MouseListenerBinding {
                key: listener.key(),
                kind: MouseListenerKind::Down,
                phase: DispatchPhase::Capture,
                button: None,
                outside: true,
            },
        )
    }

    /// Handle a matching desktop mouse-button release during the bubble phase.
    pub fn on_mouse_up<V>(
        self,
        button: crate::MouseButton,
        listener: crate::MouseUpListener<V>,
    ) -> Self {
        self.bind_mouse_listener(
            listener.id(),
            MouseListenerBinding {
                key: listener.key(),
                kind: MouseListenerKind::Up,
                phase: DispatchPhase::Bubble,
                button: Some(button),
                outside: false,
            },
        )
    }

    /// Handle a desktop mouse-button release for any button during the bubble phase.
    pub fn on_any_mouse_up<V>(self, listener: crate::MouseUpListener<V>) -> Self {
        self.bind_mouse_listener(
            listener.id(),
            MouseListenerBinding {
                key: listener.key(),
                kind: MouseListenerKind::Up,
                phase: DispatchPhase::Bubble,
                button: None,
                outside: false,
            },
        )
    }

    /// Handle any desktop mouse-button release during capture, from the root toward the target.
    pub fn capture_any_mouse_up<V>(self, listener: crate::MouseUpListener<V>) -> Self {
        self.bind_mouse_listener(
            listener.id(),
            MouseListenerBinding {
                key: listener.key(),
                kind: MouseListenerKind::Up,
                phase: DispatchPhase::Capture,
                button: None,
                outside: false,
            },
        )
    }

    /// Handle a matching button release outside this element during the capture stage.
    pub fn on_mouse_up_out<V>(
        self,
        button: crate::MouseButton,
        listener: crate::MouseUpListener<V>,
    ) -> Self {
        self.bind_mouse_listener(
            listener.id(),
            MouseListenerBinding {
                key: listener.key(),
                kind: MouseListenerKind::Up,
                phase: DispatchPhase::Capture,
                button: Some(button),
                outside: true,
            },
        )
    }

    /// Handle pointer motion over this element during the bubble phase.
    pub fn on_mouse_move<V>(self, listener: crate::MouseMoveListener<V>) -> Self {
        self.bind_mouse_listener(
            listener.id(),
            MouseListenerBinding {
                key: listener.key(),
                kind: MouseListenerKind::Move,
                phase: DispatchPhase::Bubble,
                button: None,
                outside: false,
            },
        )
    }

    /// Handle the pointer leaving the native window while this element is hovered.
    pub fn on_mouse_exit<V>(self, listener: crate::MouseExitListener<V>) -> Self {
        self.bind_mouse_listener(
            listener.id(),
            MouseListenerBinding {
                key: listener.key(),
                kind: MouseListenerKind::Exit,
                phase: DispatchPhase::Bubble,
                button: None,
                outside: false,
            },
        )
    }

    /// Observe web-style hover entry and exit, including layout changes beneath a still pointer.
    pub fn on_hover<V>(self, listener: crate::HoverListener<V>) -> Self {
        self.bind_mouse_listener(
            listener.id(),
            MouseListenerBinding {
                key: listener.key(),
                kind: MouseListenerKind::Hover,
                phase: DispatchPhase::Bubble,
                button: None,
                outside: false,
            },
        )
    }

    /// Handle native scroll-wheel input while the pointer is over this element or a descendant.
    ///
    /// Events bubble from the nearest listening element through listening ancestors. Call
    /// [`crate::EventContext::prevent_default`] when the gesture drives zoom, pan, or another
    /// custom interaction instead of the retained scroll container below it.
    pub fn on_scroll_wheel<V>(mut self, listener: crate::ScrollWheelListener<V>) -> Self {
        self.bind_listener_id(listener.id());
        self.scroll_wheel_listener = true;
        self
    }

    /// Capture raw touch contacts that start over this element or its descendants.
    ///
    /// Each contact keeps its own [`crate::TouchId`] and continues reaching this listening path
    /// until ended or cancelled, even after moving outside the element.
    pub fn on_touch<V>(mut self, listener: crate::TouchListener<V>) -> Self {
        self.bind_listener_id(listener.id());
        self.touch_listener = true;
        self
    }

    /// Open application-defined context UI from a secondary click on this element.
    pub fn on_context_menu<V>(mut self, listener: crate::ContextMenuListener<V>) -> Self {
        self.bind_listener_id(listener.id());
        self.context_menu_listener = true;
        self
    }

    /// Handle Force Touch pressure while the logical pointer is over this element.
    pub fn on_mouse_pressure<V>(mut self, listener: crate::MousePressureListener<V>) -> Self {
        self.bind_listener_id(listener.id());
        self.mouse_pressure_listener = true;
        self
    }

    /// Handle native pinch-to-zoom input while the logical pointer is over this element.
    pub fn on_pinch<V>(mut self, listener: crate::PinchListener<V>) -> Self {
        self.bind_listener_id(listener.id());
        self.pinch_listener = true;
        self
    }

    /// Handle native two-finger rotation while the logical pointer is over this element.
    pub fn on_rotation<V>(mut self, listener: crate::RotationListener<V>) -> Self {
        self.bind_listener_id(listener.id());
        self.rotation_listener = true;
        self
    }

    /// Handle a native smart-magnify request, normally a two-finger double tap on macOS.
    pub fn on_smart_magnify<V>(mut self, listener: crate::SmartMagnifyListener<V>) -> Self {
        self.bind_listener_id(listener.id());
        self.smart_magnify_listener = true;
        self
    }

    /// Start a typed drag after primary-button motion crosses the native-style threshold.
    ///
    /// The source owns the gesture, so do not attach [`Self::on_pointer`] to the same element.
    pub fn on_drag<V, T>(mut self, listener: crate::DragListener<V, T>) -> Self {
        assert!(
            !self.pointer_listener,
            "one element cannot own both a captured pointer listener and a typed drag source"
        );
        self.bind_listener_id(listener.id());
        self.drag_source = true;
        self
    }

    /// Accept a compatible typed payload when it is released over this element.
    ///
    /// Multiple payload types may be registered for the same element by using the same stable id.
    pub fn on_drop<V, T>(mut self, listener: crate::DropListener<V, T>) -> Self {
        self.bind_listener_id(listener.id());
        self.drop_target = true;
        self
    }

    /// Restrict whether a payload of type `T` may be dropped on this element.
    ///
    /// The same predicate controls both `drag_over` paint state and final delivery. Omitting it
    /// accepts every payload matching an attached [`crate::DropListener`].
    pub fn can_drop<T: 'static>(mut self, predicate: impl Fn(&T) -> bool + 'static) -> Self {
        let type_id = TypeId::of::<T>();
        assert!(
            self.drop_predicates
                .iter()
                .all(|existing| existing.type_id != type_id),
            "can_drop was registered more than once for the same payload type"
        );
        self.drop_predicates.push(DropPredicate {
            type_id,
            callback: Arc::new(move |value| {
                predicate(
                    value
                        .downcast_ref::<T>()
                        .expect("can_drop received the wrong payload type"),
                )
            }),
        });
        self
    }

    /// Attach a controlled-value listener registered by [`crate::ViewContext::input_listener`].
    pub fn on_input<V>(mut self, listener: crate::InputListener<V>) -> Self {
        self.bind_listener_id(listener.id());
        self.focusable = true;
        self.set_implicit_cursor(CursorStyle::IBeam);
        self.accessibility.role = match &self.kind {
            ElementKind::TextInput(TextInputElement {
                multiline: true, ..
            }) => AccessibilityRole::MultilineTextInput,
            ElementKind::TextInput(TextInputElement { password: true, .. }) => {
                AccessibilityRole::PasswordInput
            }
            _ => AccessibilityRole::TextInput,
        };
        self
    }

    /// Submit a valid single-line input when Return is pressed without key repeat.
    pub fn on_submit<V>(mut self, listener: crate::SubmitListener<V>) -> Self {
        assert!(
            matches!(
                &self.kind,
                ElementKind::TextInput(TextInputElement {
                    multiline: false,
                    ..
                })
            ),
            "on_submit can only be attached to a single-line text input"
        );
        self.bind_listener_id(listener.id());
        self.focusable = true;
        self.set_implicit_cursor(CursorStyle::IBeam);
        self.accessibility.role = if matches!(
            &self.kind,
            ElementKind::TextInput(TextInputElement { password: true, .. })
        ) {
            AccessibilityRole::PasswordInput
        } else {
            AccessibilityRole::TextInput
        };
        self
    }

    /// Attach a valid-form callback registered by [`crate::ViewContext::form_submit_listener`].
    pub fn on_form_submit<V>(mut self, listener: crate::FormSubmitListener<V>) -> Self {
        self.bind_listener_id(listener.id());
        self.form = true;
        self.accessibility.role = AccessibilityRole::Form;
        self
    }

    /// Attach a validation callback registered by
    /// [`crate::ViewContext::form_invalid_listener`].
    pub fn on_form_invalid<V>(mut self, listener: crate::FormInvalidListener<V>) -> Self {
        self.bind_listener_id(listener.id());
        self.form = true;
        self.accessibility.role = AccessibilityRole::Form;
        self
    }

    /// Make this control validate and submit its nearest ancestor form when activated.
    pub fn form_submitter(mut self) -> Self {
        self.form_submitter = true;
        self.clickable = true;
        self.set_implicit_cursor(CursorStyle::PointingHand);
        self.focusable = true;
        if self.accessibility.role == AccessibilityRole::GenericContainer {
            self.accessibility.role = AccessibilityRole::Button;
        }
        self
    }

    /// Handle focused key presses during the bubble phase, from focus toward the root.
    pub fn on_key_down<V>(self, listener: crate::KeyDownListener<V>) -> Self {
        self.bind_key_listener(
            listener.id(),
            KeyListenerBinding {
                key: listener.key(),
                kind: KeyListenerKind::Down,
                phase: DispatchPhase::Bubble,
            },
        )
    }

    /// Handle focused key presses during capture, from the root toward focus.
    pub fn capture_key_down<V>(self, listener: crate::KeyDownListener<V>) -> Self {
        self.bind_key_listener(
            listener.id(),
            KeyListenerBinding {
                key: listener.key(),
                kind: KeyListenerKind::Down,
                phase: DispatchPhase::Capture,
            },
        )
    }

    /// Handle focused key releases during the bubble phase, from focus toward the root.
    pub fn on_key_up<V>(self, listener: crate::KeyUpListener<V>) -> Self {
        self.bind_key_listener(
            listener.id(),
            KeyListenerBinding {
                key: listener.key(),
                kind: KeyListenerKind::Up,
                phase: DispatchPhase::Bubble,
            },
        )
    }

    /// Handle focused key releases during capture, from the root toward focus.
    pub fn capture_key_up<V>(self, listener: crate::KeyUpListener<V>) -> Self {
        self.bind_key_listener(
            listener.id(),
            KeyListenerBinding {
                key: listener.key(),
                kind: KeyListenerKind::Up,
                phase: DispatchPhase::Capture,
            },
        )
    }

    /// Attach a typed action handler during the bubble phase.
    ///
    /// Bubble action handlers run from focus toward the root and consume by default. Call
    /// [`crate::EventContext::propagate`] to continue to the next handler.
    pub fn on_action<V, A>(self, listener: crate::ActionListener<V, A>) -> Self {
        self.bind_action_listener(
            listener.id(),
            ActionListenerBinding {
                key: listener.key(),
                action_type: listener.action_type(),
                phase: DispatchPhase::Bubble,
            },
        )
    }

    /// Attach a typed action handler during capture, from the root toward focus.
    ///
    /// Capture actions propagate by default. Call [`crate::EventContext::stop_propagation`] to
    /// consume the action before it reaches deeper capture listeners or the bubble phase.
    pub fn capture_action<V, A>(self, listener: crate::ActionListener<V, A>) -> Self {
        self.bind_action_listener(
            listener.id(),
            ActionListenerBinding {
                key: listener.key(),
                action_type: listener.action_type(),
                phase: DispatchPhase::Capture,
            },
        )
    }

    /// Dismiss this surface on Escape or a pointer press outside its bounds.
    ///
    /// A dismissible element always emits [`crate::Event::Dismiss`]. Attach a typed callback with
    /// [`Self::on_dismiss`] when handling the event in [`crate::View::event`] is inconvenient.
    pub fn dismissible(mut self) -> Self {
        self.dismiss_policy = DismissPolicy::BOTH;
        self.blocks_pointer = true;
        self
    }

    /// Dismiss this surface on Escape without enabling outside-pointer dismissal.
    pub fn dismiss_on_escape(mut self) -> Self {
        self.dismiss_policy = self.dismiss_policy.with_escape();
        self.blocks_pointer = true;
        self
    }

    /// Dismiss this surface on a pointer press outside its bounds without consuming Escape.
    pub fn dismiss_on_pointer_outside(mut self) -> Self {
        self.dismiss_policy = self.dismiss_policy.with_pointer_outside();
        self.blocks_pointer = true;
        self
    }

    /// Attach a typed callback to a dismissible surface.
    pub fn on_dismiss<V>(mut self, listener: crate::DismissListener<V>) -> Self {
        self.bind_listener_id(listener.id());
        if self.dismiss_policy.is_empty() {
            self.dismiss_policy = DismissPolicy::BOTH;
        }
        self.blocks_pointer = true;
        self
    }

    /// Restore focus to this handle when a dismissible surface closes.
    pub fn restore_focus_to(mut self, handle: FocusHandle) -> Self {
        self.restore_focus = Some(handle);
        self
    }

    /// Set the native cursor shown while the pointer is over this element.
    pub fn cursor(mut self, cursor: CursorStyle) -> Self {
        self.cursor_style = Some(cursor);
        self.cursor_style_explicit = true;
        self
    }

    /// Use the platform's default arrow cursor (`default`).
    pub fn cursor_default(self) -> Self {
        self.cursor(CursorStyle::Arrow)
    }

    /// Use a pointing hand cursor (`pointer`).
    pub fn cursor_pointer(self) -> Self {
        self.cursor(CursorStyle::PointingHand)
    }

    /// Use a text-selection cursor (`text`).
    pub fn cursor_text(self) -> Self {
        self.cursor(CursorStyle::IBeam)
    }

    /// Use a closed hand cursor (`move`).
    pub fn cursor_move(self) -> Self {
        self.cursor(CursorStyle::ClosedHand)
    }

    /// Indicate that the operation is unavailable (`not-allowed`).
    pub fn cursor_not_allowed(self) -> Self {
        self.cursor(CursorStyle::OperationNotAllowed)
    }

    /// Indicate that a context menu is available (`context-menu`).
    pub fn cursor_context_menu(self) -> Self {
        self.cursor(CursorStyle::ContextualMenu)
    }

    /// Use a crosshair cursor (`crosshair`).
    pub fn cursor_crosshair(self) -> Self {
        self.cursor(CursorStyle::Crosshair)
    }

    /// Use the vertical text-selection cursor (`vertical-text`).
    pub fn cursor_vertical_text(self) -> Self {
        self.cursor(CursorStyle::IBeamCursorForVerticalLayout)
    }

    /// Indicate that a drag will create an alias (`alias`).
    pub fn cursor_alias(self) -> Self {
        self.cursor(CursorStyle::DragLink)
    }

    /// Indicate that a drag will copy its payload (`copy`).
    pub fn cursor_copy(self) -> Self {
        self.cursor(CursorStyle::DragCopy)
    }

    /// Indicate that a payload cannot be dropped here (`no-drop`).
    pub fn cursor_no_drop(self) -> Self {
        self.cursor(CursorStyle::OperationNotAllowed)
    }

    /// Use an open hand cursor (`grab`).
    pub fn cursor_grab(self) -> Self {
        self.cursor(CursorStyle::OpenHand)
    }

    /// Use a closed hand cursor (`grabbing`).
    pub fn cursor_grabbing(self) -> Self {
        self.cursor(CursorStyle::ClosedHand)
    }

    /// Use a horizontal resize cursor (`ew-resize`).
    pub fn cursor_ew_resize(self) -> Self {
        self.cursor(CursorStyle::ResizeLeftRight)
    }

    /// Use a vertical resize cursor (`ns-resize`).
    pub fn cursor_ns_resize(self) -> Self {
        self.cursor(CursorStyle::ResizeUpDown)
    }

    /// Use a north-east/south-west resize cursor (`nesw-resize`).
    pub fn cursor_nesw_resize(self) -> Self {
        self.cursor(CursorStyle::ResizeUpRightDownLeft)
    }

    /// Use a north-west/south-east resize cursor (`nwse-resize`).
    pub fn cursor_nwse_resize(self) -> Self {
        self.cursor(CursorStyle::ResizeUpLeftDownRight)
    }

    /// Use a column resize cursor (`col-resize`).
    pub fn cursor_col_resize(self) -> Self {
        self.cursor(CursorStyle::ResizeColumn)
    }

    /// Use a row resize cursor (`row-resize`).
    pub fn cursor_row_resize(self) -> Self {
        self.cursor(CursorStyle::ResizeRow)
    }

    /// Use a north-edge resize cursor (`n-resize`).
    pub fn cursor_n_resize(self) -> Self {
        self.cursor(CursorStyle::ResizeUp)
    }

    /// Use an east-edge resize cursor (`e-resize`).
    pub fn cursor_e_resize(self) -> Self {
        self.cursor(CursorStyle::ResizeRight)
    }

    /// Use a south-edge resize cursor (`s-resize`).
    pub fn cursor_s_resize(self) -> Self {
        self.cursor(CursorStyle::ResizeDown)
    }

    /// Use a west-edge resize cursor (`w-resize`).
    pub fn cursor_w_resize(self) -> Self {
        self.cursor(CursorStyle::ResizeLeft)
    }

    /// Explicitly allow browser-style pointer selection in this text subtree.
    ///
    /// Ordinary immutable text already uses `auto`, which is selectable outside controls. This
    /// override is useful for text nested in a custom clickable or draggable surface.
    pub fn user_select_text(mut self) -> Self {
        self.user_select = UserSelect::Text;
        self
    }

    /// Disable browser-style pointer selection for this element and its descendants.
    pub fn user_select_none(mut self) -> Self {
        self.user_select = UserSelect::None;
        self
    }

    /// Ergonomic alias for [`Self::user_select_text`].
    pub fn selectable(self) -> Self {
        self.user_select_text()
    }

    pub(crate) fn has_stateful_paint(&self) -> bool {
        self.hover.has_paint_overrides()
            || self.active.has_paint_overrides()
            || self.focus.has_paint_overrides()
            || self.disabled_style.has_paint_overrides()
            || self.invalid_style.has_paint_overrides()
            || self.dragging.has_paint_overrides()
            || self.drag_over.has_paint_overrides()
    }

    pub(crate) fn has_stateful_cursor(&self) -> bool {
        self.hover.cursor_style.is_some()
            || self.active.cursor_style.is_some()
            || self.focus.cursor_style.is_some()
            || self.disabled_style.cursor_style.is_some()
            || self.invalid_style.cursor_style.is_some()
            || self.dragging.cursor_style.is_some()
            || self.drag_over.cursor_style.is_some()
    }

    fn set_implicit_cursor(&mut self, cursor: CursorStyle) {
        if !self.cursor_style_explicit {
            self.cursor_style = Some(cursor);
        }
    }

    fn bind_listener_id(&mut self, id: ElementId) {
        if let Some(existing) = self.explicit_id {
            assert_eq!(
                existing, id,
                "one element cannot attach listeners with different stable ids"
            );
        }
        self.explicit_id = Some(id);
    }

    fn bind_mouse_listener(mut self, id: ElementId, binding: MouseListenerBinding) -> Self {
        self.bind_listener_id(id);
        let listeners = self
            .mouse_listeners
            .get_or_insert_with(|| Box::new(Vec::with_capacity(2)));
        assert!(
            listeners.len() < MAX_MOUSE_LISTENERS_PER_ELEMENT,
            "one element cannot attach more than {MAX_MOUSE_LISTENERS_PER_ELEMENT} desktop mouse listeners"
        );
        listeners.push(binding);
        self
    }

    fn bind_key_listener(mut self, id: ElementId, binding: KeyListenerBinding) -> Self {
        self.bind_listener_id(id);
        let listeners = self
            .key_listeners
            .get_or_insert_with(|| Box::new(Vec::with_capacity(2)));
        assert!(
            listeners.len() < MAX_KEY_LISTENERS_PER_ELEMENT,
            "one element cannot attach more than {MAX_KEY_LISTENERS_PER_ELEMENT} focused key listeners"
        );
        listeners.push(binding);
        self
    }

    fn bind_action_listener(mut self, id: ElementId, binding: ActionListenerBinding) -> Self {
        self.bind_listener_id(id);
        let listeners = self
            .action_listeners
            .get_or_insert_with(|| Box::new(Vec::with_capacity(2)));
        assert!(
            listeners.len() < MAX_ACTION_LISTENERS_PER_ELEMENT,
            "one element cannot attach more than {MAX_ACTION_LISTENERS_PER_ELEMENT} typed action listeners"
        );
        listeners.push(binding);
        self
    }
}

fn stable_hash(bytes: &[u8]) -> u64 {
    let mut hash = 0xcbf2_9ce4_8422_2325_u64;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash
}

fn bounded_validation_message(message: Arc<str>) -> (Option<Arc<str>>, bool) {
    if message.is_empty() {
        return (None, false);
    }
    if message.len() <= MAX_VALIDATION_MESSAGE_BYTES {
        return (Some(message), false);
    }
    let mut end = MAX_VALIDATION_MESSAGE_BYTES;
    while !message.is_char_boundary(end) {
        end -= 1;
    }
    (Some(Arc::from(&message[..end])), true)
}

#[cfg(test)]
mod tests {
    use super::*;
    use taffy::{AvailableSpace, TaffyTree, style::RepetitionCount};

    #[test]
    fn accessibility_relations_are_exact_and_cost_one_pointer_when_absent() {
        assert!(
            std::mem::size_of::<AccessibilityRelationsStyle>()
                <= std::mem::size_of::<Option<ElementId>>()
        );
        let element = div()
            .accessibility_controls(0_u64)
            .accessibility_active_descendant(u64::MAX)
            .accessibility_labelled_by(42_u64)
            .accessibility_described_by_pair(43_u64, 44_u64)
            .required(true);
        assert_eq!(
            element.accessibility.relations.controls(),
            Some(ElementId::new(0))
        );
        assert_eq!(
            element.accessibility.relations.active_descendant(),
            Some(ElementId::new(u64::MAX))
        );
        assert_eq!(
            element.accessibility.relations.labelled_by(),
            Some(ElementId::new(42))
        );
        assert_eq!(
            element.accessibility.relations.described_by(),
            Some(ElementId::new(43))
        );
        assert_eq!(
            element.accessibility.relations.described_by_secondary(),
            Some(ElementId::new(44))
        );
        assert!(element.accessibility.required);

        let replaced = element.accessibility_described_by(45_u64);
        assert_eq!(
            replaced.accessibility.relations.described_by(),
            Some(ElementId::new(45))
        );
        assert_eq!(
            replaced.accessibility.relations.described_by_secondary(),
            None
        );
    }

    #[test]
    fn tailwind_spacing_uses_four_pixel_units() {
        let element = div().p_4().gap_2().h_8();
        assert_eq!(element.layout.size.height, Dimension::length(32.0));
        assert_eq!(element.layout.padding.left, LengthPercentage::length(16.0));
        assert_eq!(element.layout.gap.width, LengthPercentage::length(8.0));
    }

    #[test]
    fn flex_helpers_match_gpui_and_css_shorthands() {
        let row = div()
            .flex_row_reverse()
            .flex_auto()
            .flex_basis(48.0)
            .flex_grow(f32::NAN)
            .flex_shrink(f32::INFINITY)
            .items_baseline()
            .justify_evenly()
            .content_around();
        assert_eq!(row.layout.display, Display::Flex);
        assert_eq!(row.layout.flex_direction, FlexDirection::RowReverse);
        assert_eq!(row.layout.flex_basis, Dimension::length(48.0));
        assert_eq!(row.layout.flex_grow, 0.0);
        assert_eq!(row.layout.flex_shrink, 0.0);
        assert_eq!(row.layout.align_items, Some(AlignItems::BASELINE));
        assert_eq!(
            row.layout.justify_content,
            Some(JustifyContent::SPACE_EVENLY)
        );
        assert_eq!(row.layout.align_content, Some(AlignContent::SPACE_AROUND));

        let item = div().flex_initial().self_end().aspect_square();
        assert_eq!(item.layout.flex_grow, 0.0);
        assert_eq!(item.layout.flex_shrink, 1.0);
        assert_eq!(item.layout.flex_basis, Dimension::auto());
        assert_eq!(item.layout.align_self, Some(AlignSelf::END));
        assert_eq!(item.layout.aspect_ratio, Some(1.0));
        assert_eq!(div().aspect_ratio(0.0).layout.aspect_ratio, None);
        assert_eq!(div().aspect_ratio(f32::NAN).layout.aspect_ratio, None);
    }

    #[test]
    fn margin_and_axis_gap_helpers_are_finite_and_web_shaped() {
        let element = div().mx_4().mt(-8.0).mb(f32::NAN).gap_x_3().gap_y(10.0);
        assert_eq!(
            element.layout.margin.left,
            LengthPercentageAuto::length(16.0)
        );
        assert_eq!(
            element.layout.margin.right,
            LengthPercentageAuto::length(16.0)
        );
        assert_eq!(
            element.layout.margin.top,
            LengthPercentageAuto::length(-8.0)
        );
        assert_eq!(
            element.layout.margin.bottom,
            LengthPercentageAuto::length(0.0)
        );
        assert_eq!(element.layout.gap.width, LengthPercentage::length(12.0));
        assert_eq!(element.layout.gap.height, LengthPercentage::length(10.0));

        let centered = div().mx_auto();
        assert_eq!(centered.layout.margin.left, LengthPercentageAuto::auto());
        assert_eq!(centered.layout.margin.right, LengthPercentageAuto::auto());
    }

    #[test]
    fn flex_reverse_aspect_and_auto_margins_reach_taffy_layout() {
        let children = [div().w(50.0).h(20.0), div().w(40.0).aspect_square()];
        let mut taffy = TaffyTree::<()>::new();
        let child_nodes = children
            .iter()
            .map(|child| taffy.new_leaf(child.layout.clone()).unwrap())
            .collect::<Vec<_>>();
        let root = div().flex_row_reverse().items_start().size(200.0, 100.0);
        let root_node = taffy.new_with_children(root.layout, &child_nodes).unwrap();
        taffy
            .compute_layout(
                root_node,
                TaffySize {
                    width: AvailableSpace::Definite(200.0),
                    height: AvailableSpace::Definite(100.0),
                },
            )
            .unwrap();

        let first = taffy.layout(child_nodes[0]).unwrap();
        let second = taffy.layout(child_nodes[1]).unwrap();
        assert_eq!((first.location.x, first.size.width), (150.0, 50.0));
        assert_eq!((second.size.width, second.size.height), (40.0, 40.0));
        assert!(second.location.x < first.location.x);

        let centered = div().w(40.0).aspect_square().mx_auto();
        let centered_node = taffy.new_leaf(centered.layout).unwrap();
        let block = div().block().size(200.0, 100.0);
        let block_node = taffy
            .new_with_children(block.layout, &[centered_node])
            .unwrap();
        taffy
            .compute_layout(
                block_node,
                TaffySize {
                    width: AvailableSpace::Definite(200.0),
                    height: AvailableSpace::Definite(100.0),
                },
            )
            .unwrap();
        let centered = taffy.layout(centered_node).unwrap();
        assert_eq!(centered.location.x, 80.0);
        assert_eq!((centered.size.width, centered.size.height), (40.0, 40.0));
    }

    #[test]
    fn forms_and_submit_buttons_keep_web_semantics_explicit() {
        let form = form();
        let submit = submit_button();

        assert!(form.form);
        assert_eq!(form.accessibility.role, AccessibilityRole::Form);
        assert!(submit.form_submitter);
        assert!(submit.clickable);
        assert!(submit.focusable);
        assert_eq!(submit.accessibility.role, AccessibilityRole::Button);
    }

    #[test]
    fn validation_messages_are_utf8_safe_and_bounded() {
        let message = "你".repeat(MAX_VALIDATION_MESSAGE_BYTES);
        let element = text_input("").validation_message(message);
        let retained = element
            .accessibility
            .validation_message
            .as_deref()
            .expect("bounded validation message");

        assert!(retained.len() <= MAX_VALIDATION_MESSAGE_BYTES);
        assert!(retained.is_char_boundary(retained.len()));
        assert!(element.accessibility.validation_message_truncated);
    }

    #[test]
    fn axis_padding_utilities_compose_like_tailwind() {
        let element = div().px_4().py_2();
        assert_eq!(element.layout.padding.left, LengthPercentage::length(16.0));
        assert_eq!(element.layout.padding.right, LengthPercentage::length(16.0));
        assert_eq!(element.layout.padding.top, LengthPercentage::length(8.0));
        assert_eq!(element.layout.padding.bottom, LengthPercentage::length(8.0));
    }

    #[test]
    fn grid_helpers_match_gpui_tracks_and_css_placements() {
        let grid = div()
            .grid()
            .grid_cols(5)
            .grid_rows_min_content(3)
            .grid_flow_col_dense();
        assert_eq!(grid.layout.display, Display::Grid);
        assert_eq!(grid.layout.grid_auto_flow, GridAutoFlow::ColumnDense);

        let GridTemplateComponent::Repeat(columns) = &grid.layout.grid_template_columns[0] else {
            panic!("equal columns should retain one compact repeat component");
        };
        assert_eq!(columns.count, RepetitionCount::Count(5));
        let expected_column: TrackSizingFunction = minmax(length(0.0_f32), fr(1.0_f32));
        assert_eq!(columns.tracks, [expected_column]);

        let GridTemplateComponent::Repeat(rows) = &grid.layout.grid_template_rows[0] else {
            panic!("equal rows should retain one compact repeat component");
        };
        assert_eq!(rows.count, RepetitionCount::Count(3));
        let expected_row: TrackSizingFunction = minmax(min_content(), fr(1.0_f32));
        assert_eq!(rows.tracks, [expected_row]);

        let item = div().col_start(2).col_end(-2).row_span(3).row_start_auto();
        assert!(
            matches!(item.layout.grid_column.start, GridPlacement::Line(line) if line.as_i16() == 2)
        );
        assert!(
            matches!(item.layout.grid_column.end, GridPlacement::Line(line) if line.as_i16() == -2)
        );
        assert_eq!(item.layout.grid_row.start, GridPlacement::Auto);
        assert_eq!(item.layout.grid_row.end, GridPlacement::Span(3));
    }

    #[test]
    fn grid_templates_and_placements_are_hard_bounded() {
        let grid = div()
            .grid_cols(u16::MAX)
            .grid_template_rows(std::iter::repeat_n(
                GridTrack::fr(1.0),
                usize::from(MAX_GRID_TRACKS) + 50,
            ));
        let GridTemplateComponent::Repeat(columns) = &grid.layout.grid_template_columns[0] else {
            panic!("equal columns should use repeat");
        };
        assert_eq!(columns.count, RepetitionCount::Count(MAX_GRID_TRACKS));
        assert_eq!(
            grid.layout.grid_template_rows.len(),
            usize::from(MAX_GRID_TRACKS)
        );

        let item = div()
            .col_start(i16::MAX)
            .row_end(i16::MIN)
            .col_span(u16::MAX);
        assert_eq!(
            item.layout.grid_column,
            TaffyLine {
                start: GridPlacement::Span(MAX_GRID_TRACKS),
                end: GridPlacement::Span(MAX_GRID_TRACKS),
            }
        );
        assert!(
            matches!(item.layout.grid_row.end, GridPlacement::Line(line) if line.as_i16() == -MAX_GRID_LINE)
        );
        assert_eq!(GridTrack::px(f32::NAN), GridTrack::px(0.0));
        assert_eq!(GridTrack::fr(f32::NEG_INFINITY), GridTrack::fr(0.0));
        assert_eq!(GridTrack::percent(4.0), GridTrack::percent(1.0));
    }

    #[test]
    fn grid_layout_places_the_gpui_holy_grail_in_one_pass() {
        let root = div().grid().grid_cols(5).grid_rows(5).size(500.0, 500.0);
        let children = [
            div().row_span(1).col_span_full(),
            div().col_span(1).row_span(3),
            div().col_span(3).row_span(3),
            div().col_span(1).row_span(3),
            div().row_span(1).col_span_full(),
        ];
        let mut taffy = TaffyTree::<()>::new();
        let child_nodes = children
            .iter()
            .map(|child| taffy.new_leaf(child.layout.clone()).unwrap())
            .collect::<Vec<_>>();
        let root_node = taffy.new_with_children(root.layout, &child_nodes).unwrap();
        taffy
            .compute_layout(
                root_node,
                TaffySize {
                    width: AvailableSpace::Definite(500.0),
                    height: AvailableSpace::Definite(500.0),
                },
            )
            .unwrap();

        let expected = [
            (0.0, 0.0, 500.0, 100.0),
            (0.0, 100.0, 100.0, 300.0),
            (100.0, 100.0, 300.0, 300.0),
            (400.0, 100.0, 100.0, 300.0),
            (0.0, 400.0, 500.0, 100.0),
        ];
        for (node, (x, y, width, height)) in child_nodes.into_iter().zip(expected) {
            let layout = taffy.layout(node).unwrap();
            assert_eq!((layout.location.x, layout.location.y), (x, y));
            assert_eq!((layout.size.width, layout.size.height), (width, height));
        }
    }

    #[test]
    fn string_children_become_text_nodes() {
        let element = div().child("hello").child(String::from("world"));
        assert_eq!(element.children.len(), 2);
        assert!(
            matches!(&element.children[0].kind, ElementKind::Text(value) if &**value == "hello")
        );
    }

    #[test]
    fn styled_text_is_a_single_inherited_text_leaf() {
        let content = StyledText::new("hello world")
            .with_highlights([(6..11, crate::HighlightStyle::default().font_bold())]);
        let element = div().text_lg().child(content);

        assert_eq!(element.children.len(), 1);
        let ElementKind::StyledText(styled) = &element.children[0].kind else {
            panic!("styled text should remain one leaf");
        };
        assert_eq!(&**styled.content(), "hello world");
        assert_eq!(styled.highlights().len(), 1);
    }

    #[test]
    fn svg_elements_retain_asset_fit_and_render_transform() {
        let asset = Svg::from_svg(
            r#"<svg xmlns="http://www.w3.org/2000/svg" width="20" height="10"><rect width="20" height="10"/></svg>"#,
        )
        .unwrap();
        let transform = SvgTransform::new().scale(1.5).translate(2.0, 3.0);
        let element = svg(&asset)
            .object_fit(ObjectFit::Cover)
            .svg_transform(transform);
        let ElementKind::Svg(svg) = element.kind else {
            panic!("expected an SVG element");
        };
        assert_eq!(svg.svg, asset);
        assert_eq!(svg.object_fit, ObjectFit::Cover);
        assert_eq!(svg.transform, transform);
    }

    #[test]
    fn path_elements_retain_fit_and_optional_background() {
        let mut builder = crate::PathBuilder::fill();
        builder.move_to(crate::Point::new(0.0, 0.0));
        builder.line_to(crate::Point::new(20.0, 0.0));
        builder.line_to(crate::Point::new(10.0, 10.0));
        builder.close();
        let asset = builder.build().unwrap();
        let color = Color::rgb8(14, 165, 233);
        let element = path(&asset)
            .object_fit(ObjectFit::Cover)
            .path_background(color);
        let ElementKind::Path(path) = element.kind else {
            panic!("expected a path element");
        };
        assert_eq!(path.path, asset);
        assert_eq!(path.object_fit, ObjectFit::Cover);
        assert_eq!(path.background, Some(Background::Solid(color)));
    }

    #[test]
    fn canvas_uses_web_default_dimensions() {
        let element = canvas(|bounds, context| {
            assert_eq!(bounds, context.bounds());
        });
        assert_eq!(element.layout.size.width, Dimension::length(300.0));
        assert_eq!(element.layout.size.height, Dimension::length(150.0));
        assert!(matches!(element.kind, ElementKind::Canvas(_)));
    }

    #[test]
    fn custom_shader_elements_retain_assets_and_sanitized_parameters() {
        let shader = CustomShader::new(
            r#"
fn quickgui_fragment(input: QuickGuiShaderInput) -> vec4<f32> {
    return vec4<f32>(input.uv, input.params[0].x, 1.0);
}
"#,
        )
        .unwrap();
        let element = custom_shader(shader.clone())
            .shader_parameters(ShaderParameters::new().float(0, f32::NAN));
        let ElementKind::CustomShader(element_shader) = element.kind else {
            panic!("expected a custom shader element");
        };

        assert_eq!(element_shader.shader, shader);
        assert_eq!(element_shader.parameters.vectors()[0][0], 0.0);
    }

    #[test]
    fn named_ids_are_stable() {
        assert_eq!(ElementId::named("save"), ElementId::named("save"));
        assert_ne!(ElementId::named("save"), ElementId::named("cancel"));
    }

    #[test]
    fn button_is_semantic_and_focusable() {
        let element = button().focus(|style| style.border(2.0, Color::WHITE));
        assert_eq!(element.accessibility.role, AccessibilityRole::Button);
        assert!(element.focusable);
        assert_eq!(element.focus.border_width, Some(2.0));
    }

    #[test]
    fn click_handlers_upgrade_plain_divs_to_buttons() {
        let element = div().clickable();
        assert_eq!(element.accessibility.role, AccessibilityRole::Button);
        assert!(element.focusable);
    }

    #[test]
    fn app_region_builders_match_web_drag_and_no_drag_values() {
        assert_eq!(div().app_region_drag().app_region, Some(AppRegion::Drag));
        assert_eq!(
            button().app_region_no_drag().app_region,
            Some(AppRegion::NoDrag)
        );
    }

    #[test]
    fn hidden_matches_display_none_and_display_helpers_restore_it() {
        let hidden = div().flex().hidden();
        assert!(hidden.is_display_none());
        assert_eq!(hidden.layout.display, Display::None);
        assert_eq!(hidden.clone().block().layout.display, Display::Block);
        assert_eq!(hidden.clone().flex().layout.display, Display::Flex);
        assert_eq!(hidden.grid().layout.display, Display::Grid);

        let invisible = div().grid().invisible();
        assert!(invisible.is_visibility_hidden());
        assert_eq!(invisible.layout.display, Display::Grid);
        assert_eq!(invisible.clone().visible().visibility, Visibility::Visible);
        assert_eq!(invisible.visibility, Visibility::Hidden);
    }

    #[test]
    fn text_alignment_helpers_match_gpui_and_inherit() {
        assert_eq!(div().text_left().typography.align, Some(TextAlign::Left));
        assert_eq!(
            div().text_center().typography.align,
            Some(TextAlign::Center)
        );
        assert_eq!(div().text_right().typography.align, Some(TextAlign::Right));
        assert_eq!(
            div().text_justify().typography.align,
            Some(TextAlign::Justify)
        );

        let inherited = TextStyle::new(14.0, Color::WHITE).align(TextAlign::Right);
        assert_eq!(
            TypographyStyle::default().resolve(&inherited).align,
            TextAlign::Right
        );
        assert_eq!(
            div().text_center().typography.resolve(&inherited).align,
            TextAlign::Center
        );
    }

    #[test]
    fn font_configuration_is_inherited_and_complete_fonts_can_clear_fallbacks() {
        let inherited_features =
            FontFeatures::new().disable(crate::FontFeatureTag::CONTEXTUAL_ALTERNATES);
        let inherited_fallbacks = FontFallbacks::from_fonts(["Apple Color Emoji"]);
        let inherited = TextStyle::new(14.0, Color::WHITE)
            .family(FontFamily::Monospace)
            .font_features(inherited_features.clone())
            .font_fallbacks(inherited_fallbacks.clone())
            .weight(Weight::SEMIBOLD)
            .font_style(GlyphStyle::Italic);

        let resolved = TypographyStyle::default().resolve(&inherited);
        assert_eq!(resolved.features, inherited_features);
        assert_eq!(resolved.fallbacks, Some(inherited_fallbacks));

        let local_features = FontFeatures::new().enable(crate::FontFeatureTag::SLASHED_ZERO);
        let resolved = div()
            .font_features(local_features.clone())
            .font_fallbacks(FontFallbacks::new())
            .typography
            .resolve(&inherited);
        assert_eq!(resolved.family, FontFamily::Monospace);
        assert_eq!(resolved.features, local_features);
        assert_eq!(resolved.fallbacks, None);
        assert_eq!(resolved.weight, Weight::SEMIBOLD);

        let resolved = div()
            .font(
                Font::new("Inter")
                    .features(FontFeatures::new().enable(crate::FontFeatureTag::TABULAR_NUMBERS))
                    .bold(),
            )
            .typography
            .resolve(&inherited);
        assert_eq!(resolved.family, FontFamily::named("Inter"));
        assert_eq!(
            resolved
                .features
                .value(crate::FontFeatureTag::TABULAR_NUMBERS),
            Some(1)
        );
        assert_eq!(resolved.fallbacks, None);
        assert_eq!(resolved.weight, Weight::BOLD);
        assert_eq!(resolved.font_style, GlyphStyle::Normal);
    }

    #[test]
    fn font_style_and_text_decorations_inherit_and_can_be_reset() {
        let accent = Color::rgb8(56, 189, 248);
        let inherited = TextStyle::new(14.0, Color::WHITE)
            .font_style(GlyphStyle::Italic)
            .underline_color(accent)
            .strikethrough();
        let resolved = TypographyStyle::default().resolve(&inherited);
        assert_eq!(resolved.font_style, GlyphStyle::Italic);
        assert_eq!(resolved.underline, TextUnderline::Single);
        assert_eq!(resolved.underline_color, Some(accent));
        assert!(resolved.strikethrough);

        let reset = div()
            .not_italic()
            .text_decoration_none()
            .typography
            .resolve(&inherited);
        assert_eq!(reset.font_style, GlyphStyle::Normal);
        assert_eq!(reset.underline, TextUnderline::None);
        assert_eq!(reset.underline_color, None);
        assert!(!reset.underline_wavy);
        assert_eq!(reset.underline_thickness, 1.0);
        assert!(!reset.strikethrough);
        assert_eq!(reset.strikethrough_color, None);

        let decorated = div()
            .italic()
            .double_underline()
            .text_decoration_color(accent)
            .line_through();
        assert_eq!(decorated.typography.font_style, Some(GlyphStyle::Italic));
        assert_eq!(decorated.typography.underline, Some(TextUnderline::Double));
        assert_eq!(decorated.typography.underline_color, Some(Some(accent)));
        assert_eq!(decorated.typography.underline_wavy, Some(false));
        assert_eq!(decorated.typography.underline_thickness, Some(1.0));
        assert_eq!(decorated.typography.strikethrough, Some(true));
    }

    #[test]
    fn advanced_text_decoration_helpers_match_gpui_and_inherit() {
        let inherited = TextStyle::new(14.0, Color::WHITE)
            .underline()
            .text_decoration_8()
            .text_decoration_wavy();
        let resolved = TypographyStyle::default().resolve(&inherited);
        assert_eq!(resolved.underline, TextUnderline::Single);
        assert!(resolved.underline_wavy);
        assert_eq!(resolved.underline_thickness, 8.0);

        let local = div().underline().text_decoration_4().text_decoration_wavy();
        assert_eq!(local.typography.underline, Some(TextUnderline::Single));
        assert_eq!(local.typography.underline_wavy, Some(true));
        assert_eq!(local.typography.underline_thickness, Some(4.0));
        let solid = local.text_decoration_solid().typography.resolve(&inherited);
        assert!(!solid.underline_wavy);
        assert_eq!(solid.underline_thickness, 4.0);

        for (element, thickness) in [
            (div().text_decoration_0(), 0.0),
            (div().text_decoration_1(), 1.0),
            (div().text_decoration_2(), 2.0),
            (div().text_decoration_4(), 4.0),
            (div().text_decoration_8(), 8.0),
        ] {
            assert_eq!(element.typography.underline, Some(TextUnderline::Single));
            assert_eq!(element.typography.underline_thickness, Some(thickness));
        }
    }

    #[test]
    fn text_overflow_helpers_match_gpui_and_inherit() {
        assert_eq!(
            div().whitespace_nowrap().typography.wrap,
            Some(TextWrap::None)
        );
        assert_eq!(
            div().whitespace_normal().typography.wrap,
            Some(TextWrap::Word)
        );
        assert!(matches!(
            div().text_ellipsis().typography.text_overflow,
            Some(TextOverflow::Truncate(affix)) if affix.as_ref() == "…"
        ));
        assert!(matches!(
            div().text_ellipsis_start().typography.text_overflow,
            Some(TextOverflow::TruncateStart(affix)) if affix.as_ref() == "…"
        ));
        assert!(matches!(
            div().text_ellipsis_middle().typography.text_overflow,
            Some(TextOverflow::TruncateMiddle(affix)) if affix.as_ref() == "…"
        ));

        let truncated = div().truncate();
        assert_eq!(truncated.typography.wrap, Some(TextWrap::None));
        assert!(matches!(
            truncated.typography.text_overflow,
            Some(TextOverflow::Truncate(_))
        ));
        assert_eq!(truncated.layout.overflow.x, Overflow::Hidden);
        assert_eq!(truncated.layout.overflow.y, Overflow::Hidden);

        let clamped = div().line_clamp(0);
        assert_eq!(clamped.typography.line_clamp, Some(1));
        assert_eq!(clamped.layout.overflow.x, Overflow::Hidden);

        let inherited = TextStyle::new(14.0, Color::WHITE)
            .white_space(WhiteSpace::Nowrap)
            .text_overflow(TextOverflow::ellipsis_start())
            .line_clamp(3);
        let resolved = TypographyStyle::default().resolve(&inherited);
        assert_eq!(resolved.wrap, TextWrap::None);
        assert!(matches!(
            resolved.text_overflow,
            Some(TextOverflow::TruncateStart(_))
        ));
        assert_eq!(resolved.line_clamp, Some(3));

        let overridden = div()
            .whitespace_normal()
            .text_ellipsis_middle()
            .line_clamp(2)
            .typography
            .resolve(&inherited);
        assert_eq!(overridden.wrap, TextWrap::Word);
        assert!(matches!(
            overridden.text_overflow,
            Some(TextOverflow::TruncateMiddle(_))
        ));
        assert_eq!(overridden.line_clamp, Some(2));
    }

    #[test]
    fn virtual_scroll_binds_the_list_offset_and_clips_the_viewport() {
        let mut list = VirtualList::new(100, 10.0);
        list.set_viewport_height(100.0);
        list.scroll_to(240.0);
        let element = div().virtual_scroll(&list);
        let scroll = element.virtual_scroll.as_ref().expect("virtual scroll");

        assert_eq!(element.layout.overflow.y, Overflow::Hidden);
        assert_eq!(scroll.max_offset_y, 900.0);
        assert_eq!(scroll.handle.offset(), 240.0);
        list.scroll_to(500.0);
        assert_eq!(scroll.handle.offset(), 500.0);
    }

    #[test]
    fn variable_virtual_scroll_binds_sparse_state_and_measurement_revision() {
        let list = ListState::new(100, 24.0);
        list.set_viewport_size(300.0, 120.0);
        list.scroll_to(crate::ListOffset {
            item_ix: 10,
            offset_in_item: 4.0,
        });
        let element = div().variable_virtual_scroll(&list);
        let scroll = element.virtual_scroll.as_ref().expect("virtual scroll");

        assert_eq!(element.layout.overflow.y, Overflow::Hidden);
        assert_eq!(scroll.handle.offset(), 244.0);
        assert_eq!(scroll.max_offset_y, 2_280.0);
        assert_eq!(
            scroll.measurement_revision,
            scroll.handle.measurement_revision()
        );
    }

    #[test]
    fn text_inputs_have_native_semantics_and_single_line_defaults() {
        let element = text_input("hello").placeholder("Type here");
        assert_eq!(element.accessibility.role, AccessibilityRole::TextInput);
        assert!(element.focusable);
        assert_eq!(element.cursor_style, Some(CursorStyle::IBeam));
        assert!(!element.cursor_style_explicit);
        assert_eq!(element.typography.wrap, Some(TextWrap::None));
        assert!(matches!(
            &element.kind,
            ElementKind::TextInput(input)
                if input.value.as_ref() == "hello"
                    && input.placeholder.as_ref() == "Type here"
                    && !input.multiline
                    && !input.password
        ));
    }

    #[test]
    fn password_inputs_expose_secure_semantics_without_replacing_the_value() {
        let element = text_input("sk-secret").password(true);

        assert_eq!(element.accessibility.role, AccessibilityRole::PasswordInput);
        assert!(matches!(
            &element.kind,
            ElementKind::TextInput(input) if input.password && !input.multiline
        ));

        let revealed = element.password(false);
        assert_eq!(revealed.accessibility.role, AccessibilityRole::TextInput);
        assert!(matches!(
            &revealed.kind,
            ElementKind::TextInput(input) if !input.password && input.value.as_ref() == "sk-secret"
        ));
    }

    #[test]
    fn cursor_helpers_cover_the_gpui_and_tailwind_vocabulary() {
        let cases = [
            (div().cursor_default(), CursorStyle::Arrow),
            (div().cursor_pointer(), CursorStyle::PointingHand),
            (div().cursor_text(), CursorStyle::IBeam),
            (div().cursor_move(), CursorStyle::ClosedHand),
            (div().cursor_not_allowed(), CursorStyle::OperationNotAllowed),
            (div().cursor_context_menu(), CursorStyle::ContextualMenu),
            (div().cursor_crosshair(), CursorStyle::Crosshair),
            (
                div().cursor_vertical_text(),
                CursorStyle::IBeamCursorForVerticalLayout,
            ),
            (div().cursor_alias(), CursorStyle::DragLink),
            (div().cursor_copy(), CursorStyle::DragCopy),
            (div().cursor_no_drop(), CursorStyle::OperationNotAllowed),
            (div().cursor_grab(), CursorStyle::OpenHand),
            (div().cursor_grabbing(), CursorStyle::ClosedHand),
            (div().cursor_ew_resize(), CursorStyle::ResizeLeftRight),
            (div().cursor_ns_resize(), CursorStyle::ResizeUpDown),
            (
                div().cursor_nesw_resize(),
                CursorStyle::ResizeUpRightDownLeft,
            ),
            (
                div().cursor_nwse_resize(),
                CursorStyle::ResizeUpLeftDownRight,
            ),
            (div().cursor_col_resize(), CursorStyle::ResizeColumn),
            (div().cursor_row_resize(), CursorStyle::ResizeRow),
            (div().cursor_n_resize(), CursorStyle::ResizeUp),
            (div().cursor_e_resize(), CursorStyle::ResizeRight),
            (div().cursor_s_resize(), CursorStyle::ResizeDown),
            (div().cursor_w_resize(), CursorStyle::ResizeLeft),
        ];

        for (element, expected) in cases {
            assert_eq!(element.cursor_style, Some(expected));
            assert!(element.cursor_style_explicit);
        }
    }

    #[test]
    fn explicit_cursor_wins_regardless_of_builder_order() {
        let cursor_before_behavior = div().cursor_crosshair().clickable();
        let cursor_after_behavior = div().clickable().cursor_default();
        let automatic_button = button();

        assert_eq!(
            cursor_before_behavior.cursor_style,
            Some(CursorStyle::Crosshair)
        );
        assert!(cursor_before_behavior.cursor_style_explicit);
        assert_eq!(cursor_after_behavior.cursor_style, Some(CursorStyle::Arrow));
        assert!(cursor_after_behavior.cursor_style_explicit);
        assert_eq!(
            automatic_button.cursor_style,
            Some(CursorStyle::PointingHand)
        );
        assert!(!automatic_button.cursor_style_explicit);
    }

    #[test]
    fn interaction_states_accept_the_same_cursor_helpers() {
        let element = div()
            .hover(|style| style.cursor_crosshair())
            .active(|style| style.cursor_grabbing())
            .focus(|style| style.cursor_text())
            .invalid_style(|style| style.cursor_not_allowed())
            .dragging(|style| style.cursor_copy())
            .drag_over(|style| style.cursor_alias());

        assert_eq!(element.hover.cursor_style, Some(CursorStyle::Crosshair));
        assert_eq!(element.active.cursor_style, Some(CursorStyle::ClosedHand));
        assert_eq!(element.focus.cursor_style, Some(CursorStyle::IBeam));
        assert_eq!(
            element.invalid_style.cursor_style,
            Some(CursorStyle::OperationNotAllowed)
        );
        assert_eq!(element.dragging.cursor_style, Some(CursorStyle::DragCopy));
        assert_eq!(element.drag_over.cursor_style, Some(CursorStyle::DragLink));
        assert!(!element.has_stateful_paint());
        assert!(element.has_stateful_cursor());
    }

    #[test]
    fn opacity_is_bounded_and_interaction_state_opacity_is_paint_only() {
        assert_eq!(div().opacity(-1.0).visual.opacity, 0.0);
        assert_eq!(div().opacity(2.0).visual.opacity, 1.0);
        assert_eq!(div().opacity(f32::NAN).visual.opacity, 1.0);

        let element = div().hover(|style| style.opacity(0.35));
        assert_eq!(element.hover.opacity, Some(0.35));
        assert!(element.has_stateful_paint());
        assert!(!element.has_stateful_cursor());
    }

    #[test]
    fn styled_text_areas_keep_one_bounded_controlled_run_table() {
        let element = styled_text_area(
            crate::styled_text("let answer = 42").with_highlights([(
                0..3,
                crate::HighlightStyle::default()
                    .font_bold()
                    .color(Color::rgb8(196, 181, 253)),
            )]),
        );
        let ElementKind::TextInput(input) = &element.kind else {
            panic!("expected attributed text area");
        };

        assert!(input.multiline);
        assert_eq!(input.value.as_ref(), "let answer = 42");
        assert_eq!(input.highlights.len(), 1);
        assert_eq!(input.highlights[0].range(), 0..3);
        assert_eq!(element.typography.wrap, Some(TextWrap::Word));
    }

    #[test]
    fn text_input_constraints_and_invalid_state_are_declarative() {
        let element = text_input("12")
            .max_length(4)
            .input_filter(|value| value.chars().all(|character| character.is_ascii_digit()))
            .invalid(true)
            .validation_message("Digits only")
            .invalid_style(|style| style.border(3.0, Color::rgb8(239, 68, 68)));
        let ElementKind::TextInput(input) = &element.kind else {
            panic!("expected text input");
        };

        assert_eq!(input.constraints.max_length, Some(4));
        assert!(input.constraints.filter.as_ref().unwrap()("1234"));
        assert!(!input.constraints.filter.as_ref().unwrap()("12a"));
        assert!(element.accessibility.invalid);
        assert_eq!(
            element.accessibility.validation_message.as_deref(),
            Some("Digits only")
        );
        assert_eq!(element.invalid_style.border_width, Some(3.0));
    }

    #[test]
    fn tooltips_attach_accessible_descriptions_without_changing_control_semantics() {
        let element = div().id(41_u64).tooltip("Inspect details");

        assert!(element.tooltip.is_some());
        assert_eq!(
            element.accessibility.description.as_deref(),
            Some("Inspect details")
        );
        assert!(!element.clickable);
        assert!(!element.focusable);
    }

    #[test]
    fn point_anchors_sanitize_geometry_and_default_to_zero_gap() {
        let element = overlay().anchor_at(
            crate::Point::new(f32::NAN, f32::INFINITY),
            AnchorPlacement::BottomStart,
        );
        let anchor = element.anchor.expect("point anchor");

        assert_eq!(anchor.target, AnchorTarget::Point(crate::Point::ZERO));
        assert_eq!(anchor.gap, 0.0);
        assert_eq!(anchor.viewport_margin, DEFAULT_VIEWPORT_MARGIN);
    }

    #[test]
    fn text_areas_have_multiline_semantics_and_wrapping_defaults() {
        let element = text_area("one\ntwo").placeholder("Notes");
        assert_eq!(
            element.accessibility.role,
            AccessibilityRole::MultilineTextInput
        );
        assert_eq!(element.typography.wrap, Some(TextWrap::Word));
        assert_eq!(element.layout.size.width, Dimension::length(320.0));
        assert_eq!(element.layout.size.height, Dimension::length(160.0));
        assert!(matches!(
            &element.kind,
            ElementKind::TextInput(input)
                if input.value.as_ref() == "one\ntwo" && input.multiline
        ));
    }

    #[test]
    fn shadow_utilities_are_paint_only_and_state_styles_can_remove_them() {
        let element = div().shadow_md().hover(|style| style.shadow_none());
        let shadows = element.visual.shadows.as_deref().unwrap();
        assert_eq!(shadows.len(), 2);
        assert_eq!(shadows[0].offset().y, 4.0);
        assert_eq!(element.layout, Style::default());
        assert!(
            element
                .hover
                .shadows
                .as_deref()
                .is_some_and(<[BoxShadow]>::is_empty)
        );

        let hover_elevation = ElementStateStyle::default().shadow_lg();
        assert_eq!(hover_elevation.shadows.as_deref().unwrap().len(), 2);
    }

    #[test]
    fn box_shadow_lists_fail_at_the_fixed_retention_bound() {
        let shadows = || {
            (0..=MAX_BOX_SHADOWS_PER_ELEMENT)
                .map(|index| BoxShadow::new(0.0, index as f32, Color::BLACK))
        };
        assert!(std::panic::catch_unwind(|| div().shadows(shadows())).is_err());
        assert!(
            std::panic::catch_unwind(|| ElementStateStyle::default().shadows(shadows())).is_err()
        );
    }
}
