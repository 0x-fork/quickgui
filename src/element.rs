use std::{borrow::Cow, sync::Arc};

use glyphon::Weight;
use taffy::{
    Style,
    geometry::{Point as TaffyPoint, Rect as TaffyRect, Size as TaffySize},
    prelude::{
        AlignItems, Dimension, Display, FlexDirection, JustifyContent, LengthPercentage,
        LengthPercentageAuto, Position,
    },
    style::Overflow,
};

use crate::{Color, FontFamily, KeyContext, ScenePlane, TextStyle, TextWrap};

#[cfg(target_os = "macos")]
use crate::native_view::MacNativeView;
#[cfg(target_os = "macos")]
use objc2_app_kit::NSView;

const SPACING_UNIT: f32 = 4.0;
const DEFAULT_ANCHOR_GAP: f32 = 8.0;
const DEFAULT_VIEWPORT_MARGIN: f32 = 8.0;

/// A stable identifier used for hit testing and retained state.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct ElementId(pub(crate) u64);

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
pub(crate) struct AnchorStyle {
    pub target: ElementId,
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
    Dialog,
    Menu,
    MenuItem,
    Tooltip,
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

#[derive(Clone, Debug)]
pub(crate) enum ElementKind {
    Container,
    Text(Arc<str>),
    TextInput(TextInputElement),
    #[cfg(target_os = "macos")]
    NativeView(MacNativeView),
}

#[derive(Clone, Debug)]
pub(crate) struct TextInputElement {
    pub value: Arc<str>,
    pub placeholder: Arc<str>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct VisualStyle {
    pub background: Option<Color>,
    pub border_color: Option<Color>,
    pub border_width: f32,
    pub radius: f32,
}

/// Paint-only overrides for hover and pressed states. They never trigger layout.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ElementStateStyle {
    pub(crate) background: Option<Color>,
    pub(crate) border_color: Option<Color>,
    pub(crate) border_width: Option<f32>,
    pub(crate) text_color: Option<Color>,
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
}

#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct AccessibilityStyle {
    pub role: AccessibilityRole,
    pub label: Option<Arc<str>>,
    pub value: Option<Arc<str>>,
    pub disabled: bool,
    pub selected: bool,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct TypographyStyle {
    pub color: Option<Color>,
    pub font_size: Option<f32>,
    pub line_height: Option<f32>,
    pub family: Option<FontFamily>,
    pub weight: Option<Weight>,
    pub wrap: Option<TextWrap>,
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
    pub(crate) clickable: bool,
    pub(crate) cursor_pointer: bool,
    pub(crate) cursor_text: bool,
    pub(crate) focusable: bool,
    pub(crate) key_context: Option<KeyContext>,
    pub(crate) tab_index: i16,
    pub(crate) auto_focus: bool,
    pub(crate) accessibility: AccessibilityStyle,
    pub(crate) plane: Option<ScenePlane>,
    pub(crate) z_index: Option<i16>,
    pub(crate) portal: bool,
    pub(crate) anchor: Option<AnchorStyle>,
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

/// Create a text element. Strings are owned through `Arc<str>` and cheap to retain.
pub fn text(content: impl Into<Arc<str>>) -> Element {
    Element::text(content.into())
}

/// Create a controlled, single-line text input.
///
/// Attach a stable [`crate::InputListener`] with [`Element::on_input`].
pub fn text_input(value: impl Into<Arc<str>>) -> Element {
    Element::text_input(value.into())
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
            clickable: false,
            cursor_pointer: false,
            cursor_text: false,
            focusable: false,
            key_context: None,
            tab_index: 0,
            auto_focus: false,
            accessibility: AccessibilityStyle::default(),
            plane: None,
            z_index: None,
            portal: false,
            anchor: None,
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

    fn text_input(value: Arc<str>) -> Self {
        let mut element = Self::container();
        element.kind = ElementKind::TextInput(TextInputElement {
            value,
            placeholder: Arc::from(""),
        });
        element.layout.size = TaffySize {
            width: Dimension::length(240.0),
            height: Dimension::length(40.0),
        };
        element.visual.background = Some(Color::rgb8(28, 30, 35));
        element.visual.border_color = Some(Color::rgb8(70, 74, 85));
        element.visual.border_width = 1.0;
        element.visual.radius = 8.0;
        element.focus = ElementStateStyle::default().border(2.0, Color::rgb8(94, 234, 212));
        element.focusable = true;
        element.cursor_text = true;
        element.accessibility.role = AccessibilityRole::TextInput;
        element.typography.wrap = Some(TextWrap::None);
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
        self.explicit_id = Some(id.into());
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
            target: target.into(),
            placement,
            gap: DEFAULT_ANCHOR_GAP,
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

    /// Prevent pointer events inside this element from reaching lower visual layers.
    pub fn block_pointer(mut self) -> Self {
        self.blocks_pointer = true;
        self
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
        self.explicit_id = Some(handle.id());
        self.focusable = true;
        self
    }

    /// Give a non-focusable ancestor a stable identity for scoped action dispatch.
    ///
    /// Descendant focus is still tracked through this element, but the scope itself is not added to
    /// Tab traversal.
    pub fn focus_scope(mut self, handle: FocusHandle) -> Self {
        self.explicit_id = Some(handle.id());
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
        self.explicit_id = Some(listener.id());
        self.clickable = true;
        self.cursor_pointer = true;
        self.focusable = true;
        if self.accessibility.role == AccessibilityRole::GenericContainer {
            self.accessibility.role = AccessibilityRole::Button;
        }
        self
    }

    /// Attach a controlled-value listener registered by [`crate::ViewContext::input_listener`].
    pub fn on_input<V>(mut self, listener: crate::InputListener<V>) -> Self {
        self.explicit_id = Some(listener.id());
        self.focusable = true;
        self.cursor_text = true;
        self.accessibility.role = AccessibilityRole::TextInput;
        self
    }

    /// Attach a typed action handler registered by [`crate::ViewContext::action_listener`].
    pub fn on_action<V, A>(mut self, listener: crate::ActionListener<V, A>) -> Self {
        self.explicit_id = Some(listener.id());
        self
    }

    /// Dismiss this surface on Escape or a pointer press outside its bounds.
    pub fn on_dismiss<V>(mut self, listener: crate::DismissListener<V>) -> Self {
        self.explicit_id = Some(listener.id());
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

    pub(crate) fn has_stateful_paint(&self) -> bool {
        self.hover != ElementStateStyle::default()
            || self.active != ElementStateStyle::default()
            || self.focus != ElementStateStyle::default()
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tailwind_spacing_uses_four_pixel_units() {
        let element = div().p_4().gap_2().h_8();
        assert_eq!(element.layout.size.height, Dimension::length(32.0));
        assert_eq!(element.layout.padding.left, LengthPercentage::length(16.0));
        assert_eq!(element.layout.gap.width, LengthPercentage::length(8.0));
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
    fn string_children_become_text_nodes() {
        let element = div().child("hello").child(String::from("world"));
        assert_eq!(element.children.len(), 2);
        assert!(
            matches!(&element.children[0].kind, ElementKind::Text(value) if &**value == "hello")
        );
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
    fn text_inputs_have_native_semantics_and_single_line_defaults() {
        let element = text_input("hello").placeholder("Type here");
        assert_eq!(element.accessibility.role, AccessibilityRole::TextInput);
        assert!(element.focusable);
        assert!(element.cursor_text);
        assert_eq!(element.typography.wrap, Some(TextWrap::None));
        assert!(matches!(
            &element.kind,
            ElementKind::TextInput(input)
                if input.value.as_ref() == "hello" && input.placeholder.as_ref() == "Type here"
        ));
    }
}
