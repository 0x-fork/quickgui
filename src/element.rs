use std::{
    any::{Any, TypeId},
    borrow::Cow,
    fmt,
    rc::Rc,
    sync::Arc,
};

use glyphon::Weight;
use taffy::{
    Style,
    geometry::{Line as TaffyLine, Point as TaffyPoint, Rect as TaffyRect, Size as TaffySize},
    prelude::{
        AlignItems, Dimension, Display, FlexDirection, FlexWrap, GridAutoFlow, GridPlacement,
        GridTemplateComponent, JustifyContent, LengthPercentage, LengthPercentageAuto, Position,
        TrackSizingFunction,
    },
    style::Overflow,
    style_helpers::{
        auto, fit_content, flex, fr, length, line, max_content, min_content, minmax, percent,
        repeat,
    },
};

use crate::{
    AnimatedImage, Background, BoxShadow, Canvas, Color, CustomShader, FontFamily, Image,
    ImageSource, KeyContext, MAX_VALIDATION_MESSAGE_BYTES, ObjectFit, Path, Rect, ScenePlane,
    ShaderParameters, StyledText, Svg, SvgTransform, TextHighlight, TextShaping, TextStyle,
    TextWrap, Tooltip,
    virtual_list::{VirtualList, VirtualScrollHandle},
};

#[cfg(target_os = "macos")]
use crate::native_view::MacNativeView;
#[cfg(target_os = "macos")]
use objc2_app_kit::NSView;

const SPACING_UNIT: f32 = 4.0;
const DEFAULT_ANCHOR_GAP: f32 = 8.0;
const DEFAULT_VIEWPORT_MARGIN: f32 = 8.0;

/// Maximum explicit grid tracks accepted on either axis.
///
/// The public grid helpers retain a compact `repeat()` definition, but layout cost still scales
/// with the resolved track count. Keeping this below Taffy's much larger internal safety limit
/// prevents dynamic application data from accidentally creating an expensive desktop layout.
pub const MAX_GRID_TRACKS: u16 = 1_024;

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
    TextInput,
    MultilineTextInput,
    Dialog,
    Menu,
    MenuItem,
    Tooltip,
    Form,
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

#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct VisualStyle {
    pub background: Option<Color>,
    pub border_color: Option<Color>,
    pub border_width: f32,
    pub radius: f32,
    pub shadows: Option<Arc<[BoxShadow]>>,
}

/// Paint-only overrides for hover, pressed, and focus states. They never trigger layout.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ElementStateStyle {
    pub(crate) background: Option<Color>,
    pub(crate) border_color: Option<Color>,
    pub(crate) border_width: Option<f32>,
    pub(crate) text_color: Option<Color>,
    pub(crate) shadows: Option<Arc<[BoxShadow]>>,
}

impl ElementStateStyle {
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

    pub fn text_color(mut self, color: Color) -> Self {
        self.text_color = Some(color);
        self
    }

    /// Replace the element's shadows while this state is active.
    pub fn shadow(mut self, shadow: BoxShadow) -> Self {
        self.shadows = Some(Arc::from([shadow]));
        self
    }

    /// Replace the element's shadows while this state is active.
    pub fn shadows(mut self, shadows: impl IntoIterator<Item = BoxShadow>) -> Self {
        self.shadows = Some(Arc::from(shadows.into_iter().collect::<Vec<_>>()));
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

#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct AccessibilityStyle {
    pub role: AccessibilityRole,
    pub label: Option<Arc<str>>,
    pub value: Option<Arc<str>>,
    pub disabled: bool,
    pub selected: bool,
    pub invalid: bool,
    pub validation_message: Option<Arc<str>>,
    pub validation_message_truncated: bool,
    pub description: Option<Arc<str>>,
}

#[derive(Clone, Debug)]
pub(crate) struct VirtualScrollStyle {
    pub handle: VirtualScrollHandle,
    pub max_offset_y: f32,
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
    pub weight: Option<Weight>,
    pub wrap: Option<TextWrap>,
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
            weight: self.weight.unwrap_or(inherited.weight),
            wrap: self.wrap.unwrap_or(inherited.wrap),
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
    pub(crate) visual: VisualStyle,
    pub(crate) typography: TypographyStyle,
    pub(crate) resolved_typography: TextStyle,
    pub(crate) hover: ElementStateStyle,
    pub(crate) active: ElementStateStyle,
    pub(crate) focus: ElementStateStyle,
    pub(crate) invalid_style: ElementStateStyle,
    pub(crate) dragging: ElementStateStyle,
    pub(crate) drag_over: ElementStateStyle,
    pub(crate) clickable: bool,
    pub(crate) pointer_listener: bool,
    pub(crate) context_menu_listener: bool,
    pub(crate) drag_source: bool,
    pub(crate) drop_target: bool,
    pub(crate) drop_predicates: Vec<DropPredicate>,
    pub(crate) cursor_pointer: bool,
    pub(crate) cursor_text: bool,
    pub(crate) user_select: UserSelect,
    pub(crate) resolved_user_select: bool,
    pub(crate) focusable: bool,
    pub(crate) key_context: Option<KeyContext>,
    pub(crate) tab_index: i16,
    pub(crate) auto_focus: bool,
    pub(crate) form: bool,
    pub(crate) form_submitter: bool,
    pub(crate) accessibility: AccessibilityStyle,
    pub(crate) plane: Option<ScenePlane>,
    pub(crate) z_index: Option<i16>,
    pub(crate) portal: bool,
    pub(crate) anchor: Option<AnchorStyle>,
    pub(crate) tooltip: Option<Tooltip>,
    pub(crate) app_region: Option<AppRegion>,
    pub(crate) virtual_scroll: Option<VirtualScrollStyle>,
    pub(crate) blocks_pointer: bool,
    pub(crate) dismissible: bool,
    pub(crate) restore_focus: Option<FocusHandle>,
    pub(crate) children: Vec<Element>,
    pub(crate) taffy_node: Option<taffy::NodeId>,
}

/// Create a container element.
pub fn div() -> Element {
    Element::container()
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
    div()
        .accessibility_role(AccessibilityRole::Button)
        .focusable()
        .cursor_pointer()
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
            visual: VisualStyle::default(),
            typography: TypographyStyle::default(),
            resolved_typography: default_text,
            hover: ElementStateStyle::default(),
            active: ElementStateStyle::default(),
            focus: ElementStateStyle::default(),
            invalid_style: ElementStateStyle::default(),
            dragging: ElementStateStyle::default(),
            drag_over: ElementStateStyle::default(),
            clickable: false,
            pointer_listener: false,
            context_menu_listener: false,
            drag_source: false,
            drop_target: false,
            drop_predicates: Vec::new(),
            cursor_pointer: false,
            cursor_text: false,
            user_select: UserSelect::Auto,
            resolved_user_select: false,
            focusable: false,
            key_context: None,
            tab_index: 0,
            auto_focus: false,
            form: false,
            form_submitter: false,
            accessibility: AccessibilityStyle::default(),
            plane: None,
            z_index: None,
            portal: false,
            anchor: None,
            tooltip: None,
            app_region: None,
            virtual_scroll: None,
            blocks_pointer: false,
            dismissible: false,
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
        element.cursor_text = true;
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
        self.children.push(child.into_element());
        self
    }

    pub fn children<I, E>(mut self, children: I) -> Self
    where
        I: IntoIterator<Item = E>,
        E: IntoElement,
    {
        self.children
            .extend(children.into_iter().map(IntoElement::into_element));
        self
    }

    pub fn when(self, condition: bool, apply: impl FnOnce(Self) -> Self) -> Self {
        if condition { apply(self) } else { self }
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

    pub fn flex_col(mut self) -> Self {
        self.layout.display = Display::Flex;
        self.layout.flex_direction = FlexDirection::Column;
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

    pub fn flex_none(mut self) -> Self {
        self.layout.flex_grow = 0.0;
        self.layout.flex_shrink = 0.0;
        self
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

    pub fn items_stretch(mut self) -> Self {
        self.layout.align_items = Some(AlignItems::STRETCH);
        self
    }

    pub fn justify_start(mut self) -> Self {
        self.layout.justify_content = Some(JustifyContent::FLEX_START);
        self
    }

    pub fn justify_center(mut self) -> Self {
        self.layout.justify_content = Some(JustifyContent::CENTER);
        self
    }

    pub fn justify_end(mut self) -> Self {
        self.layout.justify_content = Some(JustifyContent::FLEX_END);
        self
    }

    pub fn justify_between(mut self) -> Self {
        self.layout.justify_content = Some(JustifyContent::SPACE_BETWEEN);
        self
    }

    pub fn gap(mut self, value: f32) -> Self {
        self.layout.gap = TaffySize {
            width: LengthPercentage::length(value),
            height: LengthPercentage::length(value),
        };
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
        self.visual.shadows = Some(Arc::from(shadows.into_iter().collect::<Vec<_>>()));
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

    pub fn font_normal(mut self) -> Self {
        self.typography.weight = Some(Weight::NORMAL);
        self
    }

    pub fn font_medium(mut self) -> Self {
        self.typography.weight = Some(Weight::MEDIUM);
        self
    }

    pub fn font_semibold(mut self) -> Self {
        self.typography.weight = Some(Weight::SEMIBOLD);
        self
    }

    pub fn font_bold(mut self) -> Self {
        self.typography.weight = Some(Weight::BOLD);
        self
    }

    pub fn font_family(mut self, family: FontFamily) -> Self {
        self.typography.family = Some(family);
        self
    }

    pub fn no_wrap(mut self) -> Self {
        self.typography.wrap = Some(TextWrap::None);
        self
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

    pub fn selected(mut self, selected: bool) -> Self {
        self.accessibility.selected = selected;
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
        self.cursor_pointer = true;
        self.focusable = true;
        if self.accessibility.role == AccessibilityRole::GenericContainer {
            self.accessibility.role = AccessibilityRole::Button;
        }
        self
    }

    /// Attach a listener registered by [`crate::ViewContext::listener`].
    pub fn on_click<V>(mut self, listener: crate::ClickListener<V>) -> Self {
        self.bind_listener_id(listener.id());
        self.clickable = true;
        self.cursor_pointer = true;
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

    /// Open application-defined context UI from a secondary click on this element.
    pub fn on_context_menu<V>(mut self, listener: crate::ContextMenuListener<V>) -> Self {
        self.bind_listener_id(listener.id());
        self.context_menu_listener = true;
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
        self.cursor_text = true;
        if self.accessibility.role != AccessibilityRole::MultilineTextInput {
            self.accessibility.role = AccessibilityRole::TextInput;
        }
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
        self.cursor_text = true;
        self.accessibility.role = AccessibilityRole::TextInput;
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
        self.cursor_pointer = true;
        self.focusable = true;
        if self.accessibility.role == AccessibilityRole::GenericContainer {
            self.accessibility.role = AccessibilityRole::Button;
        }
        self
    }

    /// Attach a typed action handler registered by [`crate::ViewContext::action_listener`].
    pub fn on_action<V, A>(mut self, listener: crate::ActionListener<V, A>) -> Self {
        self.bind_listener_id(listener.id());
        self
    }

    /// Dismiss this surface on Escape or a pointer press outside its bounds.
    pub fn on_dismiss<V>(mut self, listener: crate::DismissListener<V>) -> Self {
        self.bind_listener_id(listener.id());
        self.dismissible = true;
        self.blocks_pointer = true;
        self
    }

    /// Restore focus to this handle when a dismissible surface closes.
    pub fn restore_focus_to(mut self, handle: FocusHandle) -> Self {
        self.restore_focus = Some(handle);
        self
    }

    pub fn cursor_pointer(mut self) -> Self {
        self.cursor_pointer = true;
        self
    }

    pub fn cursor_text(mut self) -> Self {
        self.cursor_text = true;
        self
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
        self.hover != ElementStateStyle::default()
            || self.active != ElementStateStyle::default()
            || self.focus != ElementStateStyle::default()
            || self.dragging != ElementStateStyle::default()
            || self.drag_over != ElementStateStyle::default()
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
    fn tailwind_spacing_uses_four_pixel_units() {
        let element = div().p_4().gap_2().h_8();
        assert_eq!(element.layout.size.height, Dimension::length(32.0));
        assert_eq!(element.layout.padding.left, LengthPercentage::length(16.0));
        assert_eq!(element.layout.gap.width, LengthPercentage::length(8.0));
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
    fn text_inputs_have_native_semantics_and_single_line_defaults() {
        let element = text_input("hello").placeholder("Type here");
        assert_eq!(element.accessibility.role, AccessibilityRole::TextInput);
        assert!(element.focusable);
        assert!(element.cursor_text);
        assert_eq!(element.typography.wrap, Some(TextWrap::None));
        assert!(matches!(
            &element.kind,
            ElementKind::TextInput(input)
                if input.value.as_ref() == "hello"
                    && input.placeholder.as_ref() == "Type here"
                    && !input.multiline
        ));
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
}
