use std::sync::{Arc, LazyLock};

use glyphon::{Style as GlyphStyle, Weight};

use crate::{
    Background, Color, CustomShader, Font, FontFallbacks, FontFamily, FontFeatures, Gradient,
    Image, Insets, Path, Rect, ShaderParameters, Svg, SvgTransform, TextHighlight, TextUnderline,
    Vector,
    font::{assert_valid_font_family, normalize_fallbacks},
    paint_order::{BoundsOrderTree, valid_bounds},
};

const MAX_RETAINED_PAINT_LAYERS: usize = 16;
/// Largest accepted analytic shadow blur in logical pixels.
pub const MAX_BOX_SHADOW_BLUR_RADIUS: f32 = 4096.0;
/// Largest absolute shadow offset or spread in logical pixels.
pub const MAX_BOX_SHADOW_EXTENT: f32 = 1_000_000.0;

/// A stable identity used to retain shaped text across frames.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct TextId(u64);

impl TextId {
    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    /// Deterministic FNV-1a identity for static names and compound keys.
    pub fn named(value: &str) -> Self {
        let mut hash = 0xcbf2_9ce4_8422_2325_u64;
        for byte in value.bytes() {
            hash ^= u64::from(byte);
            hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
        }
        Self(hash)
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum TextWrap {
    None,
    Word,
    Glyph,
}

/// CSS-like whitespace handling for text descendants.
///
/// QuickGUI keeps [`TextWrap`] as the lower-level shaping control. This enum provides the GPUI and
/// web-facing vocabulary without duplicating state in the retained style.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub enum WhiteSpace {
    #[default]
    Normal,
    Nowrap,
}

impl From<WhiteSpace> for TextWrap {
    fn from(value: WhiteSpace) -> Self {
        match value {
            WhiteSpace::Normal => Self::Word,
            WhiteSpace::Nowrap => Self::None,
        }
    }
}

/// How overflowing text is replaced inside its assigned width.
///
/// The affix is commonly an ellipsis, but remains application-defined to match GPUI. Truncation
/// is performed at Unicode grapheme boundaries and becomes part of the bounded retained text
/// layout, so it adds no per-frame work once the width and style are stable.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum TextOverflow {
    /// Preserve the start and replace the omitted tail with the supplied affix.
    Truncate(Arc<str>),
    /// Preserve the end and replace the omitted start with the supplied affix.
    TruncateStart(Arc<str>),
    /// Preserve both ends and replace the omitted middle with the supplied affix.
    TruncateMiddle(Arc<str>),
}

impl TextOverflow {
    /// End truncation using the single-character Unicode ellipsis.
    pub fn ellipsis() -> Self {
        static ELLIPSIS: LazyLock<Arc<str>> = LazyLock::new(|| Arc::from("…"));
        Self::Truncate(Arc::clone(&ELLIPSIS))
    }

    /// Start truncation using the single-character Unicode ellipsis.
    pub fn ellipsis_start() -> Self {
        static ELLIPSIS: LazyLock<Arc<str>> = LazyLock::new(|| Arc::from("…"));
        Self::TruncateStart(Arc::clone(&ELLIPSIS))
    }

    /// Middle truncation using the single-character Unicode ellipsis.
    pub fn ellipsis_middle() -> Self {
        static ELLIPSIS: LazyLock<Arc<str>> = LazyLock::new(|| Arc::from("…"));
        Self::TruncateMiddle(Arc::clone(&ELLIPSIS))
    }
}

/// Horizontal alignment of text lines within their element bounds.
///
/// [`TextAlign::Start`] and [`TextAlign::End`] are direction relative: they resolve to `Left` and
/// `Right` in an LTR subtree and to `Right` and `Left` in an RTL one. Resolution happens once per
/// layout build, so retained shaping never observes an unresolved logical alignment.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub enum TextAlign {
    Left,
    Center,
    Right,
    Justify,
    /// Inline start edge of the resolved layout direction.
    #[default]
    Start,
    /// Inline end edge of the resolved layout direction.
    End,
}

/// Base paragraph direction used when shaping bidirectional text.
///
/// `Auto` follows the Unicode bidirectional algorithm's first strong character. `Ltr` and `Rtl`
/// force the paragraph embedding level so neutral characters and punctuation resolve against the
/// declared direction instead of the content.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub enum TextDirection {
    #[default]
    Auto,
    Ltr,
    Rtl,
}

/// Case mapping applied to non-editable text before shaping.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum TextTransform {
    Uppercase,
    Lowercase,
    /// Uppercase the first character of every whitespace-delimited word.
    Capitalize,
}

/// Where a line may break inside a run of characters.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub enum WordBreak {
    #[default]
    Normal,
    /// Allow a break between any two characters.
    BreakAll,
    /// Never break inside CJK text; only ordinary soft break opportunities apply.
    KeepAll,
}

/// Whether an otherwise unbreakable word may be broken to avoid overflow.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub enum OverflowWrap {
    #[default]
    Normal,
    /// Break anywhere as soon as the line would overflow.
    Anywhere,
    /// Break a long word only when it cannot fit on a line of its own.
    BreakWord,
}

/// Whether soft hyphens (`U+00AD`) may become visible break opportunities.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub enum Hyphens {
    /// Soft hyphens are removed before shaping and never render.
    #[default]
    None,
    /// Author-placed soft hyphens render as a hyphen when a line breaks there.
    Manual,
}

/// Maximum text primitives one blurred text shadow may add to the display list.
pub const MAX_TEXT_SHADOW_SAMPLES: usize = 5;

/// Alpha applied to each copy of a blur-approximated text shadow.
const TEXT_SHADOW_BLUR_ALPHA: f32 = 0.45;

/// Maximum absolute logical letter or word spacing retained from one declaration.
pub const MAX_TEXT_SPACING: f32 = 256.0;

/// Clamp a spacing declaration to a finite, bounded logical-pixel value.
pub(crate) fn sane_text_spacing(value: f32) -> f32 {
    if value.is_finite() {
        value.clamp(-MAX_TEXT_SPACING, MAX_TEXT_SPACING)
    } else {
        0.0
    }
}

/// One drop shadow painted beneath a text run.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TextShadow {
    /// Logical horizontal offset.
    pub offset_x: f32,
    /// Logical vertical offset.
    pub offset_y: f32,
    /// Logical blur radius. Approximated; see [`Element::text_shadow`](crate::Element::text_shadow).
    pub blur: f32,
    pub color: Color,
}

impl TextShadow {
    /// Maximum absolute logical offset retained for one text shadow.
    pub const MAX_OFFSET: f32 = 256.0;
    /// Maximum logical blur radius retained for one text shadow.
    pub const MAX_BLUR: f32 = 64.0;

    /// Create a bounded text shadow.
    ///
    /// Non-finite inputs collapse to zero and offsets and blur are clamped to [`Self::MAX_OFFSET`]
    /// and [`Self::MAX_BLUR`], so one declaration can never produce unbounded paint geometry.
    pub fn new(offset_x: f32, offset_y: f32, blur: f32, color: Color) -> Self {
        let offset = |value: f32| {
            if value.is_finite() {
                value.clamp(-Self::MAX_OFFSET, Self::MAX_OFFSET)
            } else {
                0.0
            }
        };
        Self {
            offset_x: offset(offset_x),
            offset_y: offset(offset_y),
            blur: if blur.is_finite() {
                blur.clamp(0.0, Self::MAX_BLUR)
            } else {
                0.0
            },
            color,
        }
    }
}

/// Text shaping strategy.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum TextShaping {
    /// Full script shaping, ligatures, and system-font fallback.
    Advanced,
    /// Cheap one-glyph-per-character shaping for app-controlled text and fonts.
    ///
    /// This does not provide complex-script shaping or general font fallback. It is intended for
    /// known ASCII/code/log content where the selected font contains every required glyph.
    Basic,
}

/// Text metrics and shaping properties. The ordinary foreground color does not invalidate
/// shaping; explicit decoration colors remain part of the retained attribute key.
#[derive(Clone, Debug, PartialEq)]
pub struct TextStyle {
    pub font_size: f32,
    pub line_height: f32,
    /// Logical width of one monospace cell. When set, glyph advances are quantized to this width.
    pub monospace_width: Option<f32>,
    pub family: FontFamily,
    pub features: FontFeatures,
    pub fallbacks: Option<FontFallbacks>,
    pub weight: Weight,
    pub font_style: GlyphStyle,
    /// Optically thicken rasterized glyph stems without selecting another font weight.
    pub font_thicken: bool,
    pub underline: TextUnderline,
    pub underline_color: Option<Color>,
    /// Whether underlines use a spell-checker-style wave instead of a solid line.
    pub underline_wavy: bool,
    /// Logical-pixel underline thickness. Zero intentionally suppresses underline paint.
    pub underline_thickness: f32,
    pub strikethrough: bool,
    pub strikethrough_color: Option<Color>,
    /// Whether a line is drawn above the text's ascent.
    pub overline: bool,
    pub overline_color: Option<Color>,
    pub align: TextAlign,
    pub wrap: TextWrap,
    pub text_overflow: Option<TextOverflow>,
    pub line_clamp: Option<usize>,
    pub shaping: TextShaping,
    /// Base paragraph direction used when shaping bidirectional content.
    pub direction: TextDirection,
    /// Extra logical-pixel advance added after every glyph cluster.
    pub letter_spacing: f32,
    /// Extra logical-pixel advance added after every space character.
    pub word_spacing: f32,
    /// Case mapping applied to non-editable text before shaping.
    pub transform: Option<TextTransform>,
    pub word_break: WordBreak,
    pub overflow_wrap: OverflowWrap,
    pub hyphens: Hyphens,
    /// Drop shadow painted beneath the glyphs.
    pub shadow: Option<TextShadow>,
    pub color: Color,
}

impl Default for TextStyle {
    fn default() -> Self {
        Self::new(14.0, Color::BLACK)
    }
}

impl TextStyle {
    pub fn new(font_size: f32, color: Color) -> Self {
        Self {
            font_size,
            line_height: font_size * 1.35,
            monospace_width: None,
            family: FontFamily::SansSerif,
            features: FontFeatures::new(),
            fallbacks: None,
            weight: Weight::NORMAL,
            font_style: GlyphStyle::Normal,
            font_thicken: false,
            underline: TextUnderline::None,
            underline_color: None,
            underline_wavy: false,
            underline_thickness: 1.0,
            strikethrough: false,
            strikethrough_color: None,
            overline: false,
            overline_color: None,
            align: TextAlign::Start,
            wrap: TextWrap::Word,
            text_overflow: None,
            line_clamp: None,
            shaping: TextShaping::Advanced,
            direction: TextDirection::Auto,
            letter_spacing: 0.0,
            word_spacing: 0.0,
            transform: None,
            word_break: WordBreak::Normal,
            overflow_wrap: OverflowWrap::Normal,
            hyphens: Hyphens::None,
            shadow: None,
            color,
        }
    }

    pub fn line_height(mut self, line_height: f32) -> Self {
        self.line_height = line_height;
        self
    }

    pub fn monospace_width(mut self, width: f32) -> Self {
        self.monospace_width = Some(width.max(1.0));
        self
    }

    pub fn family(mut self, family: impl Into<FontFamily>) -> Self {
        let family = family.into();
        assert_valid_font_family(&family);
        self.family = family;
        self
    }

    pub fn font_features(mut self, features: FontFeatures) -> Self {
        self.features = features;
        self
    }

    pub fn font_fallbacks(mut self, fallbacks: FontFallbacks) -> Self {
        self.fallbacks = (!fallbacks.is_empty()).then_some(fallbacks);
        self
    }

    pub fn font(mut self, font: Font) -> Self {
        assert_valid_font_family(&font.family);
        self.family = font.family;
        self.features = font.features;
        self.fallbacks = normalize_fallbacks(font.fallbacks);
        self.weight = font.weight;
        self.font_style = font.style;
        self
    }

    pub fn weight(mut self, weight: Weight) -> Self {
        self.weight = weight;
        self
    }

    pub fn font_style(mut self, style: GlyphStyle) -> Self {
        self.font_style = style;
        self
    }

    pub fn font_thicken(mut self, thicken: bool) -> Self {
        self.font_thicken = thicken;
        self
    }

    pub fn underline(mut self) -> Self {
        self.underline = TextUnderline::Single;
        self.underline_wavy = false;
        self.underline_thickness = 1.0;
        self
    }

    pub fn double_underline(mut self) -> Self {
        self.underline = TextUnderline::Double;
        self.underline_wavy = false;
        self.underline_thickness = 1.0;
        self
    }

    pub fn underline_color(mut self, color: Color) -> Self {
        if self.underline == TextUnderline::None {
            self.underline = TextUnderline::Single;
            self.underline_wavy = false;
            self.underline_thickness = 1.0;
        }
        self.underline_color = Some(color);
        self
    }

    pub fn text_decoration_none(mut self) -> Self {
        self.underline = TextUnderline::None;
        self.underline_color = None;
        self.underline_wavy = false;
        self.underline_thickness = 1.0;
        self.strikethrough = false;
        self.strikethrough_color = None;
        self
    }

    pub fn text_decoration_solid(mut self) -> Self {
        if self.underline == TextUnderline::None {
            self.underline = TextUnderline::Single;
        }
        self.underline_wavy = false;
        self
    }

    pub fn text_decoration_wavy(mut self) -> Self {
        if self.underline == TextUnderline::None {
            self.underline = TextUnderline::Single;
        }
        self.underline_wavy = true;
        self
    }

    fn text_decoration_thickness(mut self, thickness: f32) -> Self {
        if self.underline == TextUnderline::None {
            self.underline = TextUnderline::Single;
        }
        self.underline_thickness = thickness;
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

    pub fn strikethrough(mut self) -> Self {
        self.strikethrough = true;
        self
    }

    pub fn strikethrough_color(mut self, color: Color) -> Self {
        self.strikethrough = true;
        self.strikethrough_color = Some(color);
        self
    }

    pub(crate) fn has_decorations(&self) -> bool {
        (self.underline != TextUnderline::None && self.underline_thickness > 0.0)
            || self.strikethrough
            || self.overline
    }

    /// Draw a line above the text's ascent.
    pub fn overline(mut self) -> Self {
        self.overline = true;
        self
    }

    /// Draw a colored line above the text's ascent.
    pub fn overline_color(mut self, color: Color) -> Self {
        self.overline = true;
        self.overline_color = Some(color);
        self
    }

    /// Force the base paragraph direction used when shaping bidirectional content.
    pub fn direction(mut self, direction: TextDirection) -> Self {
        self.direction = direction;
        self
    }

    /// Set extra logical-pixel advance after each glyph cluster.
    pub fn letter_spacing(mut self, spacing: f32) -> Self {
        self.letter_spacing = sane_text_spacing(spacing);
        self
    }

    /// Set extra logical-pixel advance after each space character.
    pub fn word_spacing(mut self, spacing: f32) -> Self {
        self.word_spacing = sane_text_spacing(spacing);
        self
    }

    /// Apply a case mapping to non-editable text before shaping.
    pub fn text_transform(mut self, transform: TextTransform) -> Self {
        self.transform = Some(transform);
        self
    }

    pub fn word_break(mut self, word_break: WordBreak) -> Self {
        self.word_break = word_break;
        self
    }

    pub fn overflow_wrap(mut self, overflow_wrap: OverflowWrap) -> Self {
        self.overflow_wrap = overflow_wrap;
        self
    }

    pub fn hyphens(mut self, hyphens: Hyphens) -> Self {
        self.hyphens = hyphens;
        self
    }

    /// Paint one drop shadow beneath the glyphs.
    pub fn text_shadow(mut self, shadow: TextShadow) -> Self {
        self.shadow = Some(shadow);
        self
    }

    pub fn align(mut self, align: TextAlign) -> Self {
        self.align = align;
        self
    }

    pub fn wrap(mut self, wrap: TextWrap) -> Self {
        self.wrap = wrap;
        self
    }

    pub fn white_space(mut self, white_space: WhiteSpace) -> Self {
        self.wrap = white_space.into();
        self
    }

    pub fn text_overflow(mut self, overflow: TextOverflow) -> Self {
        self.text_overflow = Some(overflow);
        self
    }

    pub fn line_clamp(mut self, lines: usize) -> Self {
        self.line_clamp = Some(lines.max(1));
        self
    }

    pub fn shaping(mut self, shaping: TextShaping) -> Self {
        self.shaping = shaping;
        self
    }
}

/// Per-corner radii ordered top-left, top-right, bottom-right, bottom-left.
///
/// A single `f32` converts into equal radii, so existing uniform-radius call sites are unchanged.
/// [`Corners::resolve`] applies the CSS uniform-scale rule so two radii sharing one edge can never
/// overlap, which keeps the analytic signed-distance evaluation valid for any declared value.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Corners {
    pub top_left: f32,
    pub top_right: f32,
    pub bottom_right: f32,
    pub bottom_left: f32,
}

impl Corners {
    /// Square corners.
    pub const ZERO: Self = Self::all(0.0);

    pub const fn all(radius: f32) -> Self {
        Self {
            top_left: radius,
            top_right: radius,
            bottom_right: radius,
            bottom_left: radius,
        }
    }

    pub const fn new(top_left: f32, top_right: f32, bottom_right: f32, bottom_left: f32) -> Self {
        Self {
            top_left,
            top_right,
            bottom_right,
            bottom_left,
        }
    }

    /// Round only the two top corners.
    pub const fn top(radius: f32) -> Self {
        Self::new(radius, radius, 0.0, 0.0)
    }

    /// Round only the two bottom corners.
    pub const fn bottom(radius: f32) -> Self {
        Self::new(0.0, 0.0, radius, radius)
    }

    /// Round only the two left corners.
    pub const fn left(radius: f32) -> Self {
        Self::new(radius, 0.0, 0.0, radius)
    }

    /// Round only the two right corners.
    pub const fn right(radius: f32) -> Self {
        Self::new(0.0, radius, radius, 0.0)
    }

    /// Replace non-finite and negative values with zero.
    pub fn sanitized(self) -> Self {
        Self {
            top_left: finite_or_zero(self.top_left).max(0.0),
            top_right: finite_or_zero(self.top_right).max(0.0),
            bottom_right: finite_or_zero(self.bottom_right).max(0.0),
            bottom_left: finite_or_zero(self.bottom_left).max(0.0),
        }
    }

    pub fn is_zero(self) -> bool {
        self.top_left <= 0.0
            && self.top_right <= 0.0
            && self.bottom_right <= 0.0
            && self.bottom_left <= 0.0
    }

    /// The largest declared radius.
    pub fn maximum(self) -> f32 {
        self.top_left
            .max(self.top_right)
            .max(self.bottom_right)
            .max(self.bottom_left)
    }

    /// Grow every corner by `amount`, clamping at zero. Used by outlines drawn outside the border.
    pub fn expanded(self, amount: f32) -> Self {
        Self {
            top_left: (self.top_left + amount).max(0.0),
            top_right: (self.top_right + amount).max(0.0),
            bottom_right: (self.bottom_right + amount).max(0.0),
            bottom_left: (self.bottom_left + amount).max(0.0),
        }
    }

    /// Apply the CSS uniform-scale rule so adjacent radii never exceed their shared edge.
    pub fn resolve(self, width: f32, height: f32) -> Self {
        let corners = self.sanitized();
        let width = finite_or_zero(width).max(0.0);
        let height = finite_or_zero(height).max(0.0);
        let mut scale = 1.0_f32;
        let mut constrain = |sum: f32, extent: f32| {
            if sum > 0.0 {
                scale = scale.min(extent / sum);
            }
        };
        constrain(corners.top_left + corners.top_right, width);
        constrain(corners.bottom_left + corners.bottom_right, width);
        constrain(corners.top_left + corners.bottom_left, height);
        constrain(corners.top_right + corners.bottom_right, height);
        if scale >= 1.0 || !scale.is_finite() {
            return corners;
        }
        Self {
            top_left: corners.top_left * scale,
            top_right: corners.top_right * scale,
            bottom_right: corners.bottom_right * scale,
            bottom_left: corners.bottom_left * scale,
        }
    }

    pub(crate) fn as_array(self) -> [f32; 4] {
        [
            self.top_left,
            self.top_right,
            self.bottom_right,
            self.bottom_left,
        ]
    }
}

impl From<f32> for Corners {
    fn from(radius: f32) -> Self {
        Self::all(radius)
    }
}

/// How a border or outline ring is painted along its perimeter.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum BorderStyle {
    #[default]
    Solid,
    /// Evenly distributed dashes three border widths long, separated by two-width gaps.
    Dashed,
    /// Evenly distributed square dots one border width long, separated by one-width gaps.
    Dotted,
}

impl BorderStyle {
    pub(crate) fn code(self) -> f32 {
        match self {
            Self::Solid => 0.0,
            Self::Dashed => 1.0,
            Self::Dotted => 2.0,
        }
    }
}

/// Largest number of color filters retained by one element.
pub const MAX_FILTERS_PER_ELEMENT: usize = 8;

/// A CSS-shaped color filter.
///
/// Every variant is expressible as one color matrix, so a whole chain collapses into a single
/// per-primitive matrix on the CPU and costs one multiply-add in the shader. Filters that need a
/// convolution or an offscreen group — `blur` and `drop-shadow` — are deliberately absent; use
/// [`Element::shadow`](crate::Element::shadow) for elevation.
///
/// Amounts follow CSS: `1.0` is the unmodified image for `brightness`, `contrast`, and
/// `saturate`, and `0.0` is the unmodified image for `grayscale`, `invert`, and `sepia`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Filter {
    Brightness(f32),
    Contrast(f32),
    Saturate(f32),
    Grayscale(f32),
    Invert(f32),
    Sepia(f32),
    /// Rotate hues by the given number of degrees.
    HueRotate(f32),
    Opacity(f32),
}

/// A 4x5 color matrix applied to straight-alpha, encoded-sRGB color.
///
/// Matching CSS, filters operate on encoded sRGB rather than the framework's linear-light
/// working space; the shader converts in and out around the multiply.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ColorMatrix([f32; 20]);

impl Default for ColorMatrix {
    fn default() -> Self {
        Self::IDENTITY
    }
}

impl ColorMatrix {
    /// The matrix that leaves color unchanged.
    pub const IDENTITY: Self = Self([
        1.0, 0.0, 0.0, 0.0, 0.0, //
        0.0, 1.0, 0.0, 0.0, 0.0, //
        0.0, 0.0, 1.0, 0.0, 0.0, //
        0.0, 0.0, 0.0, 1.0, 0.0,
    ]);

    pub const fn new(values: [f32; 20]) -> Self {
        Self(values)
    }

    pub const fn as_array(self) -> [f32; 20] {
        self.0
    }

    pub fn is_identity(self) -> bool {
        self == Self::IDENTITY
    }

    /// Apply `self` first and `next` second.
    pub fn then(self, next: Self) -> Self {
        let mut combined = [0.0_f32; 20];
        for row in 0..4 {
            for column in 0..4 {
                let mut sum = 0.0;
                for inner in 0..4 {
                    sum += next.0[row * 5 + inner] * self.0[inner * 5 + column];
                }
                combined[row * 5 + column] = sum;
            }
            let mut offset = next.0[row * 5 + 4];
            for inner in 0..4 {
                offset += next.0[row * 5 + inner] * self.0[inner * 5 + 4];
            }
            combined[row * 5 + 4] = offset;
        }
        Self(combined)
    }
}

fn finite_amount(value: f32, fallback: f32) -> f32 {
    if value.is_finite() {
        value.max(0.0)
    } else {
        fallback
    }
}

impl From<Filter> for ColorMatrix {
    fn from(filter: Filter) -> Self {
        match filter {
            Filter::Brightness(amount) => {
                let amount = finite_amount(amount, 1.0);
                Self([
                    amount, 0.0, 0.0, 0.0, 0.0, //
                    0.0, amount, 0.0, 0.0, 0.0, //
                    0.0, 0.0, amount, 0.0, 0.0, //
                    0.0, 0.0, 0.0, 1.0, 0.0,
                ])
            }
            Filter::Contrast(amount) => {
                let amount = finite_amount(amount, 1.0);
                let offset = 0.5 - amount * 0.5;
                Self([
                    amount, 0.0, 0.0, 0.0, offset, //
                    0.0, amount, 0.0, 0.0, offset, //
                    0.0, 0.0, amount, 0.0, offset, //
                    0.0, 0.0, 0.0, 1.0, 0.0,
                ])
            }
            Filter::Saturate(amount) => saturate_matrix(finite_amount(amount, 1.0)),
            Filter::Grayscale(amount) => saturate_matrix(1.0 - finite_amount(amount, 0.0).min(1.0)),
            Filter::Invert(amount) => {
                let amount = finite_amount(amount, 0.0).min(1.0);
                let scale = 1.0 - 2.0 * amount;
                Self([
                    scale, 0.0, 0.0, 0.0, amount, //
                    0.0, scale, 0.0, 0.0, amount, //
                    0.0, 0.0, scale, 0.0, amount, //
                    0.0, 0.0, 0.0, 1.0, 0.0,
                ])
            }
            Filter::Sepia(amount) => {
                let amount = finite_amount(amount, 0.0).min(1.0);
                let mix = |full: f32, identity: f32| identity + (full - identity) * amount;
                Self([
                    mix(0.393, 1.0),
                    mix(0.769, 0.0),
                    mix(0.189, 0.0),
                    0.0,
                    0.0, //
                    mix(0.349, 0.0),
                    mix(0.686, 1.0),
                    mix(0.168, 0.0),
                    0.0,
                    0.0, //
                    mix(0.272, 0.0),
                    mix(0.534, 0.0),
                    mix(0.131, 1.0),
                    0.0,
                    0.0, //
                    0.0,
                    0.0,
                    0.0,
                    1.0,
                    0.0,
                ])
            }
            Filter::HueRotate(degrees) => {
                let radians = if degrees.is_finite() {
                    degrees.to_radians()
                } else {
                    0.0
                };
                let (sine, cosine) = radians.sin_cos();
                Self([
                    0.213 + cosine * 0.787 - sine * 0.213,
                    0.715 - cosine * 0.715 - sine * 0.715,
                    0.072 - cosine * 0.072 + sine * 0.928,
                    0.0,
                    0.0,
                    0.213 - cosine * 0.213 + sine * 0.143,
                    0.715 + cosine * 0.285 + sine * 0.140,
                    0.072 - cosine * 0.072 - sine * 0.283,
                    0.0,
                    0.0,
                    0.213 - cosine * 0.213 - sine * 0.787,
                    0.715 - cosine * 0.715 + sine * 0.715,
                    0.072 + cosine * 0.928 + sine * 0.072,
                    0.0,
                    0.0,
                    0.0,
                    0.0,
                    0.0,
                    1.0,
                    0.0,
                ])
            }
            Filter::Opacity(amount) => {
                let amount = finite_amount(amount, 1.0).min(1.0);
                Self([
                    1.0, 0.0, 0.0, 0.0, 0.0, //
                    0.0, 1.0, 0.0, 0.0, 0.0, //
                    0.0, 0.0, 1.0, 0.0, 0.0, //
                    0.0, 0.0, 0.0, amount, 0.0,
                ])
            }
        }
    }
}

fn saturate_matrix(amount: f32) -> ColorMatrix {
    // The CSS/SVG luminance-preserving saturation matrix.
    let (red, green, blue) = (0.213, 0.715, 0.072);
    ColorMatrix([
        red + amount * (1.0 - red),
        green - amount * green,
        blue - amount * blue,
        0.0,
        0.0,
        red - amount * red,
        green + amount * (1.0 - green),
        blue - amount * blue,
        0.0,
        0.0,
        red - amount * red,
        green - amount * green,
        blue + amount * (1.0 - blue),
        0.0,
        0.0,
        0.0,
        0.0,
        0.0,
        1.0,
        0.0,
    ])
}

/// A bounded, ordered chain of at most [`MAX_FILTERS_PER_ELEMENT`] color filters.
///
/// The chain is collapsed into one [`ColorMatrix`] when it reaches the scene, so the number of
/// declared filters never affects per-frame GPU work.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Filters {
    length: u8,
    filters: [Option<Filter>; MAX_FILTERS_PER_ELEMENT],
}

impl Filters {
    pub const fn none() -> Self {
        Self {
            length: 0,
            filters: [None; MAX_FILTERS_PER_ELEMENT],
        }
    }

    /// Collect at most [`MAX_FILTERS_PER_ELEMENT`] filters in declaration order.
    pub fn new(filters: impl IntoIterator<Item = Filter>) -> Self {
        let mut collected = Self::none();
        for filter in filters {
            if usize::from(collected.length) == MAX_FILTERS_PER_ELEMENT {
                break;
            }
            collected.filters[usize::from(collected.length)] = Some(filter);
            collected.length += 1;
        }
        collected
    }

    /// Append one filter, ignoring it once the chain is full.
    pub fn push(mut self, filter: Filter) -> Self {
        if usize::from(self.length) < MAX_FILTERS_PER_ELEMENT {
            self.filters[usize::from(self.length)] = Some(filter);
            self.length += 1;
        }
        self
    }

    pub fn len(self) -> usize {
        usize::from(self.length)
    }

    pub fn is_empty(self) -> bool {
        self.length == 0
    }

    /// Collapse the chain into one color matrix applied in declaration order.
    pub fn color_matrix(self) -> ColorMatrix {
        let mut matrix = ColorMatrix::IDENTITY;
        for filter in self.filters.iter().take(usize::from(self.length)).flatten() {
            matrix = matrix.then(ColorMatrix::from(*filter));
        }
        matrix
    }
}

impl FromIterator<Filter> for Filters {
    fn from_iter<T: IntoIterator<Item = Filter>>(filters: T) -> Self {
        Self::new(filters)
    }
}

/// A filled rounded rectangle with an optional inside border and clip.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Quad {
    pub rect: Rect,
    pub fill: Color,
    /// An optional multi-stop gradient replacing `fill` inside the rounded box.
    pub background: Option<Gradient>,
    pub radius: Corners,
    pub border_width: f32,
    pub border_color: Color,
    pub clip: Option<Rect>,
}

impl Quad {
    pub fn new(rect: Rect, fill: Color) -> Self {
        Self {
            rect,
            fill,
            background: None,
            radius: Corners::ZERO,
            border_width: 0.0,
            border_color: Color::TRANSPARENT,
            clip: None,
        }
    }

    pub fn radius(mut self, radius: f32) -> Self {
        self.radius = Corners::all(radius.max(0.0));
        self
    }

    /// Round each corner independently.
    pub fn corner_radii(mut self, radii: Corners) -> Self {
        self.radius = radii.sanitized();
        self
    }

    /// Fill with a solid color or a bounded multi-stop gradient resolved against `rect`.
    pub fn background(mut self, background: impl Into<Background>) -> Self {
        match background.into() {
            Background::Solid(color) => {
                self.fill = color;
                self.background = None;
            }
            other => {
                self.background = other.as_gradient();
            }
        }
        self
    }

    pub fn border(mut self, width: f32, color: Color) -> Self {
        self.border_width = width.max(0.0);
        self.border_color = color;
        self
    }

    pub fn clip(mut self, clip: Rect) -> Self {
        self.clip = Some(clip);
        self
    }
}

/// A framework element quad whose inside border can use a different width on each edge.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct EdgeQuad {
    pub(crate) rect: Rect,
    pub(crate) fill: Color,
    pub(crate) background: Option<Gradient>,
    pub(crate) radius: Corners,
    pub(crate) border_widths: Insets,
    pub(crate) border_color: Color,
    pub(crate) border_style: BorderStyle,
    pub(crate) clip: Option<Rect>,
}

impl EdgeQuad {
    pub(crate) fn new(rect: Rect, fill: Color) -> Self {
        Self {
            rect,
            fill,
            background: None,
            radius: Corners::ZERO,
            border_widths: Insets::default(),
            border_color: Color::TRANSPARENT,
            border_style: BorderStyle::Solid,
            clip: None,
        }
    }

    pub(crate) fn corner_radii(mut self, radii: Corners) -> Self {
        self.radius = radii.sanitized();
        self
    }

    pub(crate) fn background(mut self, gradient: Option<Gradient>) -> Self {
        self.background = gradient;
        self
    }

    pub(crate) fn border_style(mut self, style: BorderStyle) -> Self {
        self.border_style = style;
        self
    }

    pub(crate) fn border(mut self, widths: Insets, color: Color) -> Self {
        self.border_widths = Insets {
            top: finite_or_zero(widths.top).max(0.0),
            right: finite_or_zero(widths.right).max(0.0),
            bottom: finite_or_zero(widths.bottom).max(0.0),
            left: finite_or_zero(widths.left).max(0.0),
        };
        self.border_color = color;
        self
    }

    pub(crate) fn clip(mut self, clip: Rect) -> Self {
        self.clip = Some(clip);
        self
    }
}

/// One underline wave evaluated analytically by the shared instanced-shape shader.
///
/// Keeping a whole visual span in one instance avoids generating or retaining a CPU-side path for
/// every wave crest. The geometry is already clipped to visible text lines before it reaches the
/// scene.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct WavyUnderline {
    pub rect: Rect,
    pub baseline: f32,
    pub amplitude: f32,
    pub thickness: f32,
    pub wavelength: f32,
    pub color: Color,
    pub clip: Option<Rect>,
}

impl WavyUnderline {
    pub fn new(
        rect: Rect,
        baseline: f32,
        amplitude: f32,
        thickness: f32,
        wavelength: f32,
        color: Color,
    ) -> Self {
        Self {
            rect,
            baseline,
            amplitude,
            thickness,
            wavelength,
            color,
            clip: None,
        }
    }

    pub fn clip(mut self, clip: Rect) -> Self {
        self.clip = Some(clip);
        self
    }
}

/// A CSS-like shadow attached to an element's rounded border box.
///
/// Multiple shadows are painted in declaration order, with the first shadow on top. Drop
/// shadows paint behind the element; inset shadows paint above its background and border.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BoxShadow {
    color: Color,
    offset: Vector,
    blur_radius: f32,
    spread_radius: f32,
    inset: bool,
}

impl BoxShadow {
    pub fn new(offset_x: f32, offset_y: f32, color: Color) -> Self {
        Self {
            color,
            offset: Vector::new(
                finite_or_zero(offset_x).clamp(-MAX_BOX_SHADOW_EXTENT, MAX_BOX_SHADOW_EXTENT),
                finite_or_zero(offset_y).clamp(-MAX_BOX_SHADOW_EXTENT, MAX_BOX_SHADOW_EXTENT),
            ),
            blur_radius: 0.0,
            spread_radius: 0.0,
            inset: false,
        }
    }

    pub fn blur_radius(mut self, radius: f32) -> Self {
        self.blur_radius = finite_or_zero(radius).clamp(0.0, MAX_BOX_SHADOW_BLUR_RADIUS);
        self
    }

    pub fn spread_radius(mut self, radius: f32) -> Self {
        self.spread_radius =
            finite_or_zero(radius).clamp(-MAX_BOX_SHADOW_EXTENT, MAX_BOX_SHADOW_EXTENT);
        self
    }

    pub fn inset(mut self, inset: bool) -> Self {
        self.inset = inset;
        self
    }

    pub const fn color(self) -> Color {
        self.color
    }

    pub const fn offset(self) -> Vector {
        self.offset
    }

    pub const fn blur(self) -> f32 {
        self.blur_radius
    }

    pub const fn spread(self) -> f32 {
        self.spread_radius
    }

    pub const fn is_inset(self) -> bool {
        self.inset
    }

    fn multiply_alpha(mut self, opacity: f32) -> Self {
        self.color = self.color.multiply_alpha(opacity);
        self
    }
}

fn finite_or_zero(value: f32) -> f32 {
    if value.is_finite() { value } else { 0.0 }
}

/// A lower-level shadow display-list primitive.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Shadow {
    pub element_rect: Rect,
    pub radius: Corners,
    pub style: BoxShadow,
    pub clip: Option<Rect>,
}

impl Shadow {
    pub fn new(element_rect: Rect, style: BoxShadow) -> Self {
        Self {
            element_rect,
            radius: Corners::ZERO,
            style,
            clip: None,
        }
    }

    pub fn radius(mut self, radius: f32) -> Self {
        self.radius = Corners::all(finite_or_zero(radius).max(0.0));
        self
    }

    /// Follow each of the element's corners independently.
    pub fn corner_radii(mut self, radii: Corners) -> Self {
        self.radius = radii.sanitized();
        self
    }

    pub fn clip(mut self, clip: Rect) -> Self {
        self.clip = Some(clip);
        self
    }
}

/// One sampled image rectangle with normalized source coordinates and a rounded content mask.
#[derive(Clone, Debug, PartialEq)]
pub struct ImagePrimitive {
    pub image: Image,
    pub destination: Rect,
    pub source_uv: Rect,
    pub mask: Rect,
    pub radius: f32,
    /// A collapsed color-filter chain applied to sampled pixels.
    pub color_matrix: ColorMatrix,
    pub opacity: f32,
    pub clip: Option<Rect>,
}

impl ImagePrimitive {
    pub fn new(image: Image, destination: Rect) -> Self {
        Self {
            image,
            destination,
            source_uv: Rect::new(0.0, 0.0, 1.0, 1.0),
            mask: destination,
            radius: 0.0,
            color_matrix: ColorMatrix::IDENTITY,
            opacity: 1.0,
            clip: None,
        }
    }

    pub fn source_uv(mut self, source_uv: Rect) -> Self {
        self.source_uv = source_uv;
        self
    }

    pub fn mask(mut self, mask: Rect) -> Self {
        self.mask = mask;
        self
    }

    pub fn radius(mut self, radius: f32) -> Self {
        self.radius = radius.max(0.0);
        self
    }

    /// Fully desaturate sampled pixels without creating another decoded image.
    pub fn grayscale(self, grayscale: bool) -> Self {
        self.color_matrix(if grayscale {
            ColorMatrix::from(Filter::Grayscale(1.0))
        } else {
            ColorMatrix::IDENTITY
        })
    }

    /// Apply a collapsed color-filter chain to sampled pixels.
    pub fn color_matrix(mut self, matrix: ColorMatrix) -> Self {
        self.color_matrix = matrix;
        self
    }

    pub fn opacity(mut self, opacity: f32) -> Self {
        self.opacity = sanitize_opacity(opacity);
        self
    }

    pub fn clip(mut self, clip: Rect) -> Self {
        self.clip = Some(clip);
        self
    }
}

/// One tinted SVG mask with normalized source coordinates and a rounded content mask.
#[derive(Clone, Debug, PartialEq)]
pub struct SvgPrimitive {
    pub svg: Svg,
    pub destination: Rect,
    pub source_uv: Rect,
    pub mask: Rect,
    pub radius: f32,
    pub color: Color,
    pub transform: SvgTransform,
    pub clip: Option<Rect>,
}

/// One retained tessellated path with a paint, affine scale/translation, and logical clip.
#[derive(Clone, Debug, PartialEq)]
pub struct PathPrimitive {
    pub path: Path,
    pub background: Background,
    scale: [f32; 2],
    translation: Vector,
    pub clip: Option<Rect>,
}

impl PathPrimitive {
    pub fn new(path: impl Into<Path>, background: impl Into<Background>) -> Self {
        Self {
            path: path.into(),
            background: background.into(),
            scale: [1.0, 1.0],
            translation: Vector::ZERO,
            clip: None,
        }
    }

    pub fn translate(mut self, x: f32, y: f32) -> Self {
        self.translation = Vector::new(sanitize_path_translation(x), sanitize_path_translation(y));
        self
    }

    pub fn scale(mut self, scale: f32) -> Self {
        let scale = sanitize_path_scale(scale);
        self.scale = [scale, scale];
        self
    }

    pub fn scale_xy(mut self, x: f32, y: f32) -> Self {
        self.scale = [sanitize_path_scale(x), sanitize_path_scale(y)];
        self
    }

    pub fn clip(mut self, clip: Rect) -> Self {
        self.clip = Some(clip);
        self
    }

    pub(crate) const fn scale_factors(&self) -> [f32; 2] {
        self.scale
    }

    pub(crate) const fn translation(&self) -> Vector {
        self.translation
    }

    pub(crate) fn render_bounds(&self) -> Rect {
        let bounds = self.path.bounds();
        let first_x = bounds.x * self.scale[0] + self.translation.x;
        let second_x = bounds.right() * self.scale[0] + self.translation.x;
        let first_y = bounds.y * self.scale[1] + self.translation.y;
        let second_y = bounds.bottom() * self.scale[1] + self.translation.y;
        Rect::new(
            first_x.min(second_x),
            first_y.min(second_y),
            (second_x - first_x).abs(),
            (second_y - first_y).abs(),
        )
    }
}

/// One retained rectangle painted by validated application WGSL.
#[derive(Clone, Debug, PartialEq)]
pub struct CustomShaderPrimitive {
    pub shader: CustomShader,
    pub rect: Rect,
    pub parameters: ShaderParameters,
    pub opacity: f32,
    pub clip: Option<Rect>,
}

impl CustomShaderPrimitive {
    pub fn new(shader: impl Into<CustomShader>, rect: Rect) -> Self {
        Self {
            shader: shader.into(),
            rect,
            parameters: ShaderParameters::default(),
            opacity: 1.0,
            clip: None,
        }
    }

    pub fn parameters(mut self, parameters: impl Into<ShaderParameters>) -> Self {
        self.parameters = parameters.into();
        self
    }

    pub fn opacity(mut self, opacity: f32) -> Self {
        self.opacity = sanitize_opacity(opacity);
        self
    }

    pub fn clip(mut self, clip: Rect) -> Self {
        self.clip = Some(clip);
        self
    }
}

const MAX_PATH_PRIMITIVE_SCALE: f32 = 1_024.0;

fn sanitize_path_scale(value: f32) -> f32 {
    if value.is_finite() {
        value.clamp(-MAX_PATH_PRIMITIVE_SCALE, MAX_PATH_PRIMITIVE_SCALE)
    } else {
        1.0
    }
}

fn sanitize_path_translation(value: f32) -> f32 {
    if value.is_finite() {
        value.clamp(-crate::MAX_PATH_COORDINATE, crate::MAX_PATH_COORDINATE)
    } else {
        0.0
    }
}

impl SvgPrimitive {
    pub fn new(svg: Svg, destination: Rect, color: Color) -> Self {
        Self {
            svg,
            destination,
            source_uv: Rect::new(0.0, 0.0, 1.0, 1.0),
            mask: destination,
            radius: 0.0,
            color,
            transform: SvgTransform::IDENTITY,
            clip: None,
        }
    }

    pub fn source_uv(mut self, source_uv: Rect) -> Self {
        self.source_uv = source_uv;
        self
    }

    pub fn mask(mut self, mask: Rect) -> Self {
        self.mask = mask;
        self
    }

    pub fn radius(mut self, radius: f32) -> Self {
        self.radius = finite_or_zero(radius).max(0.0);
        self
    }

    pub fn transform(mut self, transform: SvgTransform) -> Self {
        self.transform = transform;
        self
    }

    pub fn clip(mut self, clip: Rect) -> Self {
        self.clip = Some(clip);
        self
    }

    pub(crate) fn render_bounds(&self) -> Rect {
        self.transform.transformed_bounds(self.destination)
    }
}

/// A retained text command. Reuse `id` and the same `Arc<str>` to avoid shaping and allocation.
#[derive(Clone, Debug, PartialEq)]
pub struct TextRun {
    pub id: TextId,
    pub content: Arc<str>,
    pub bounds: Rect,
    pub style: TextStyle,
    pub opacity: f32,
    pub clip: Option<Rect>,
    pub(crate) highlights: Option<Arc<[TextHighlight]>>,
}

impl TextRun {
    pub fn new(id: TextId, content: Arc<str>, bounds: Rect, style: TextStyle) -> Self {
        Self {
            id,
            content,
            bounds,
            style,
            opacity: 1.0,
            clip: None,
            highlights: None,
        }
    }

    pub fn clip(mut self, clip: Rect) -> Self {
        self.clip = Some(clip);
        self
    }

    pub fn opacity(mut self, opacity: f32) -> Self {
        self.opacity = sanitize_opacity(opacity);
        self
    }

    pub(crate) fn with_highlights(mut self, highlights: Arc<[TextHighlight]>) -> Self {
        if !highlights.is_empty() {
            self.highlights = Some(highlights);
        }
        self
    }

    fn has_visible_paint(&self) -> bool {
        self.opacity > 0.0
            && (self.style.color.a > 0.0
                || (self.style.underline != TextUnderline::None
                    && self.style.underline_thickness > 0.0
                    && self
                        .style
                        .underline_color
                        .is_some_and(|color| color.a > 0.0))
                || self
                    .style
                    .strikethrough_color
                    .is_some_and(|color| self.style.strikethrough && color.a > 0.0)
                || self.highlights.as_deref().is_some_and(|highlights| {
                    highlights.iter().any(|highlight| {
                        let style = &highlight.style;
                        style.color.is_some_and(|color| color.a > 0.0)
                            || style.background.is_some_and(|color| color.a > 0.0)
                            || (style.underline != TextUnderline::None
                                && style.underline_thickness != Some(0.0)
                                && style.underline_color.is_some_and(|color| color.a > 0.0))
                            || (style.strikethrough
                                && style.strikethrough_color.is_some_and(|color| color.a > 0.0))
                    })
                }))
    }
}

/// The two framework-rendered planes around platform-native child content.
///
/// Without native children, both planes share the window's WGPU surface. When a macOS `NSView` is
/// mounted, QuickGUI lazily presents the overlay plane through a transparent second WGPU surface,
/// placing AppKit content between the two GPU planes without changing application view code.
#[derive(Clone, Copy, Debug, Default, Eq, Ord, PartialEq, PartialOrd)]
pub enum ScenePlane {
    #[default]
    Base,
    Overlay,
}

#[derive(Clone, Copy, Debug, Default, Eq, Ord, PartialEq, PartialOrd)]
pub(crate) struct PaintLayerKey {
    pub plane: ScenePlane,
    pub z_index: i16,
}

#[derive(Debug)]
pub(crate) struct PaintLayer {
    key: PaintLayerKey,
    quads: Vec<Quad>,
    edge_quads: Vec<EdgeQuad>,
    wavy_underlines: Vec<WavyUnderline>,
    shadows: Vec<Shadow>,
    shapes: Vec<ShapeRef>,
    images: Vec<ImagePrimitive>,
    svgs: Vec<SvgPrimitive>,
    paths: Vec<PathPrimitive>,
    custom_shaders: Vec<CustomShaderPrimitive>,
    text: Vec<TextRun>,
    paint: Vec<PaintItem>,
    order_tree: BoundsOrderTree,
    max_order: u32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ShapeRef {
    Quad(usize),
    EdgeQuad(usize),
    WavyUnderline(usize),
    Shadow(usize),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum PrimitiveRef {
    Shape(ShapeRef),
    Image(usize),
    Svg(usize),
    Path(usize),
    CustomShader(usize),
    Text(usize),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct PaintItem {
    pub order: u32,
    pub primitive: PrimitiveRef,
}

impl PaintLayer {
    fn new(key: PaintLayerKey) -> Self {
        Self {
            key,
            quads: Vec::new(),
            edge_quads: Vec::new(),
            wavy_underlines: Vec::new(),
            shadows: Vec::new(),
            shapes: Vec::new(),
            images: Vec::new(),
            svgs: Vec::new(),
            paths: Vec::new(),
            custom_shaders: Vec::new(),
            text: Vec::new(),
            paint: Vec::new(),
            order_tree: BoundsOrderTree::default(),
            max_order: 0,
        }
    }

    pub(crate) fn key(&self) -> PaintLayerKey {
        self.key
    }

    pub(crate) fn quads(&self) -> &[Quad] {
        &self.quads
    }

    pub(crate) fn edge_quads(&self) -> &[EdgeQuad] {
        &self.edge_quads
    }

    pub(crate) fn wavy_underlines(&self) -> &[WavyUnderline] {
        &self.wavy_underlines
    }

    pub(crate) fn shadows(&self) -> &[Shadow] {
        &self.shadows
    }

    #[cfg(test)]
    pub(crate) fn shapes(&self) -> &[ShapeRef] {
        &self.shapes
    }

    pub(crate) fn images(&self) -> &[ImagePrimitive] {
        &self.images
    }

    pub(crate) fn svgs(&self) -> &[SvgPrimitive] {
        &self.svgs
    }

    pub(crate) fn paths(&self) -> &[PathPrimitive] {
        &self.paths
    }

    pub(crate) fn custom_shaders(&self) -> &[CustomShaderPrimitive] {
        &self.custom_shaders
    }

    pub(crate) fn text_runs(&self) -> &[TextRun] {
        &self.text
    }

    pub(crate) fn paint(&self) -> &[PaintItem] {
        &self.paint
    }

    pub(crate) fn max_order(&self) -> u32 {
        self.max_order
    }

    fn push_paint(&mut self, bounds: Rect, primitive: PrimitiveRef) {
        debug_assert!(valid_bounds(bounds));
        let order = self.order_tree.insert(bounds);
        self.max_order = self.max_order.max(order);
        self.paint.push(PaintItem { order, primitive });
    }

    fn clear(&mut self) {
        self.quads.clear();
        self.edge_quads.clear();
        self.wavy_underlines.clear();
        self.shadows.clear();
        self.shapes.clear();
        self.images.clear();
        self.svgs.clear();
        self.paths.clear();
        self.custom_shaders.clear();
        self.text.clear();
        self.paint.clear();
        self.order_tree.clear();
        self.max_order = 0;
    }
}

/// A reusable display list. [`Scene::clear`] retains its allocations.
#[derive(Debug)]
pub struct Scene {
    background: Color,
    layers: Vec<PaintLayer>,
    used_layers: usize,
    opacity: f32,
}

impl Scene {
    pub fn new() -> Self {
        Self {
            background: Color::BLACK,
            layers: vec![PaintLayer {
                key: PaintLayerKey::default(),
                quads: Vec::with_capacity(256),
                edge_quads: Vec::with_capacity(64),
                wavy_underlines: Vec::with_capacity(32),
                shadows: Vec::with_capacity(64),
                shapes: Vec::with_capacity(320),
                images: Vec::with_capacity(64),
                svgs: Vec::with_capacity(64),
                paths: Vec::with_capacity(64),
                custom_shaders: Vec::with_capacity(16),
                text: Vec::with_capacity(128),
                paint: Vec::with_capacity(512),
                order_tree: BoundsOrderTree::with_capacity(512),
                max_order: 0,
            }],
            used_layers: 1,
            opacity: 1.0,
        }
    }

    pub fn clear(&mut self, background: Color) {
        self.background = background;
        if self.layers.len() > MAX_RETAINED_PAINT_LAYERS {
            self.layers.truncate(MAX_RETAINED_PAINT_LAYERS);
        }
        for layer in &mut self.layers {
            layer.clear();
        }
        if self.layers.is_empty() {
            self.layers.push(PaintLayer::new(PaintLayerKey::default()));
        }
        self.layers[0].key = PaintLayerKey::default();
        self.used_layers = 1;
        self.opacity = 1.0;
    }

    pub fn push_quad(&mut self, quad: Quad) {
        self.push_quad_in(PaintLayerKey::default(), quad);
    }

    pub(crate) fn push_quad_in(&mut self, key: PaintLayerKey, mut quad: Quad) {
        quad.fill = quad.fill.multiply_alpha(self.opacity);
        quad.border_color = quad.border_color.multiply_alpha(self.opacity);
        quad.background = quad
            .background
            .map(|gradient| gradient.multiply_alpha(self.opacity));
        let gradient_visible = quad
            .background
            .is_some_and(|gradient| gradient.is_visible());
        if (quad.fill.a > 0.0 || quad.border_color.a > 0.0 || gradient_visible)
            && let Some(bounds) = clipped_paint_bounds(quad.rect, [quad.clip])
        {
            let layer = self.layer_mut(key);
            let index = layer.quads.len();
            layer.quads.push(quad);
            let shape = ShapeRef::Quad(index);
            layer.shapes.push(shape);
            layer.push_paint(bounds, PrimitiveRef::Shape(shape));
        }
    }

    pub(crate) fn push_edge_quad_in(&mut self, key: PaintLayerKey, mut quad: EdgeQuad) {
        quad.fill = quad.fill.multiply_alpha(self.opacity);
        quad.border_color = quad.border_color.multiply_alpha(self.opacity);
        quad.background = quad
            .background
            .map(|gradient| gradient.multiply_alpha(self.opacity));
        let gradient_visible = quad
            .background
            .is_some_and(|gradient| gradient.is_visible());
        if (quad.fill.a > 0.0 || quad.border_color.a > 0.0 || gradient_visible)
            && let Some(bounds) = clipped_paint_bounds(quad.rect, [quad.clip])
        {
            let layer = self.layer_mut(key);
            let index = layer.edge_quads.len();
            layer.edge_quads.push(quad);
            let shape = ShapeRef::EdgeQuad(index);
            layer.shapes.push(shape);
            layer.push_paint(bounds, PrimitiveRef::Shape(shape));
        }
    }

    pub(crate) fn push_wavy_underline_in(
        &mut self,
        key: PaintLayerKey,
        mut underline: WavyUnderline,
    ) {
        underline.color = underline.color.multiply_alpha(self.opacity);
        if underline.color.a > 0.0
            && underline.thickness > 0.0
            && underline.amplitude >= 0.0
            && underline.wavelength > 0.0
            && underline.baseline.is_finite()
            && let Some(bounds) = clipped_paint_bounds(underline.rect, [underline.clip])
        {
            let layer = self.layer_mut(key);
            let index = layer.wavy_underlines.len();
            layer.wavy_underlines.push(underline);
            let shape = ShapeRef::WavyUnderline(index);
            layer.shapes.push(shape);
            layer.push_paint(bounds, PrimitiveRef::Shape(shape));
        }
    }

    pub fn push_shadow(&mut self, shadow: Shadow) {
        self.push_shadow_in(PaintLayerKey::default(), shadow);
    }

    pub(crate) fn push_shadow_in(&mut self, key: PaintLayerKey, mut shadow: Shadow) {
        shadow.style = shadow.style.multiply_alpha(self.opacity);
        if shadow.style.color().a > 0.0
            && let Some(bounds) = shadow_render_bounds(&shadow)
            && let Some(bounds) = clipped_paint_bounds(bounds, [shadow.clip])
        {
            let layer = self.layer_mut(key);
            let index = layer.shadows.len();
            layer.shadows.push(shadow);
            let shape = ShapeRef::Shadow(index);
            layer.shapes.push(shape);
            layer.push_paint(bounds, PrimitiveRef::Shape(shape));
        }
    }

    pub fn fill(&mut self, rect: Rect, color: Color) {
        self.push_quad(Quad::new(rect, color));
    }

    pub fn push_image(&mut self, image: ImagePrimitive) {
        self.push_image_in(PaintLayerKey::default(), image);
    }

    pub(crate) fn push_image_in(&mut self, key: PaintLayerKey, mut image: ImagePrimitive) {
        image.opacity = sanitize_opacity(image.opacity * self.opacity);
        if image.opacity > 0.0
            && valid_bounds(image.source_uv)
            && let Some(bounds) =
                clipped_paint_bounds(image.destination, [Some(image.mask), image.clip])
        {
            let layer = self.layer_mut(key);
            let index = layer.images.len();
            layer.images.push(image);
            layer.push_paint(bounds, PrimitiveRef::Image(index));
        }
    }

    pub fn push_svg(&mut self, svg: SvgPrimitive) {
        self.push_svg_in(PaintLayerKey::default(), svg);
    }

    pub fn push_path(&mut self, path: PathPrimitive) {
        self.push_path_in(PaintLayerKey::default(), path);
    }

    pub(crate) fn push_path_in(&mut self, key: PaintLayerKey, mut path: PathPrimitive) {
        path.background = path.background.multiply_alpha(self.opacity);
        if !path.path.is_empty()
            && path.background.is_visible()
            && let Some(bounds) = clipped_paint_bounds(path.render_bounds(), [path.clip])
        {
            let layer = self.layer_mut(key);
            let index = layer.paths.len();
            layer.paths.push(path);
            layer.push_paint(bounds, PrimitiveRef::Path(index));
        }
    }

    pub fn push_custom_shader(&mut self, shader: CustomShaderPrimitive) {
        self.push_custom_shader_in(PaintLayerKey::default(), shader);
    }

    pub(crate) fn push_custom_shader_in(
        &mut self,
        key: PaintLayerKey,
        mut shader: CustomShaderPrimitive,
    ) {
        shader.opacity = sanitize_opacity(shader.opacity * self.opacity);
        if shader.opacity > 0.0
            && let Some(bounds) = clipped_paint_bounds(shader.rect, [shader.clip])
        {
            let layer = self.layer_mut(key);
            let index = layer.custom_shaders.len();
            layer.custom_shaders.push(shader);
            layer.push_paint(bounds, PrimitiveRef::CustomShader(index));
        }
    }

    pub(crate) fn push_svg_in(&mut self, key: PaintLayerKey, mut svg: SvgPrimitive) {
        svg.color = svg.color.multiply_alpha(self.opacity);
        if !svg.destination.is_empty()
            && !svg.source_uv.is_empty()
            && !svg.mask.is_empty()
            && svg.color.a > 0.0
            && let Some(bounds) =
                clipped_paint_bounds(svg.render_bounds(), [Some(svg.mask), svg.clip])
        {
            let layer = self.layer_mut(key);
            let index = layer.svgs.len();
            layer.svgs.push(svg);
            layer.push_paint(bounds, PrimitiveRef::Svg(index));
        }
    }

    pub fn push_text(&mut self, text: TextRun) {
        self.push_text_in(PaintLayerKey::default(), text);
    }

    pub(crate) fn push_text_in(&mut self, key: PaintLayerKey, mut text: TextRun) {
        text.opacity = sanitize_opacity(text.opacity * self.opacity);
        self.push_text_shadow_in(key, &text);
        if text.has_visible_paint()
            && let Some(bounds) = clipped_paint_bounds(text.bounds, [text.clip])
        {
            let layer = self.layer_mut(key);
            let index = layer.text.len();
            layer.text.push(text);
            layer.push_paint(bounds, PrimitiveRef::Text(index));
        }
    }

    /// Emit the offset copies that stand in for one text run's drop shadow.
    ///
    /// The offset and color are exact. A blur radius is approximated by a bounded, fixed set of
    /// additional offset copies at reduced alpha, so a declaration can never grow the display
    /// list without limit. Every copy reuses the run's shaping key: only its bounds and color
    /// change, so no extra shaping or glyph atlas work is performed.
    fn push_text_shadow_in(&mut self, key: PaintLayerKey, text: &TextRun) {
        let Some(shadow) = text.style.shadow else {
            return;
        };
        if shadow.color.a <= 0.0 || text.content.is_empty() {
            return;
        }
        let blur = shadow.blur.max(0.0);
        let spread = blur * 0.5;
        let samples: &[(f32, f32, f32)] = if spread <= 0.25 {
            &[(0.0, 0.0, 1.0)]
        } else {
            &[
                (0.0, 0.0, TEXT_SHADOW_BLUR_ALPHA),
                (-1.0, -1.0, TEXT_SHADOW_BLUR_ALPHA),
                (1.0, -1.0, TEXT_SHADOW_BLUR_ALPHA),
                (-1.0, 1.0, TEXT_SHADOW_BLUR_ALPHA),
                (1.0, 1.0, TEXT_SHADOW_BLUR_ALPHA),
            ]
        };
        debug_assert!(samples.len() <= MAX_TEXT_SHADOW_SAMPLES);
        for (dx, dy, alpha) in samples.iter().copied() {
            let mut copy = text.clone();
            copy.bounds = Rect::new(
                text.bounds.x + shadow.offset_x + dx * spread,
                text.bounds.y + shadow.offset_y + dy * spread,
                text.bounds.width,
                text.bounds.height,
            );
            copy.style.shadow = None;
            copy.style.color = shadow.color;
            copy.style.underline_color = Some(shadow.color);
            copy.style.strikethrough_color = Some(shadow.color);
            copy.style.overline_color = Some(shadow.color);
            copy.opacity = sanitize_opacity(text.opacity * alpha);
            if !copy.has_visible_paint() {
                continue;
            }
            let Some(bounds) = clipped_paint_bounds(copy.bounds, [copy.clip]) else {
                continue;
            };
            let layer = self.layer_mut(key);
            let index = layer.text.len();
            layer.text.push(copy);
            layer.push_paint(bounds, PrimitiveRef::Text(index));
        }
    }

    pub fn background(&self) -> Color {
        self.background
    }

    pub(crate) fn multiply_opacity(&mut self, opacity: f32) -> f32 {
        let previous = self.opacity;
        self.opacity = sanitize_opacity(previous * sanitize_opacity(opacity));
        previous
    }

    pub(crate) fn restore_opacity(&mut self, opacity: f32) {
        self.opacity = sanitize_opacity(opacity);
    }

    pub(crate) fn current_opacity(&self) -> f32 {
        self.opacity
    }

    pub fn quads(&self) -> &[Quad] {
        self.layers
            .iter()
            .take(self.used_layers)
            .find(|layer| layer.key == PaintLayerKey::default())
            .map(PaintLayer::quads)
            .unwrap_or_default()
    }

    #[cfg(test)]
    pub(crate) fn edge_quads(&self) -> &[EdgeQuad] {
        self.layers
            .iter()
            .take(self.used_layers)
            .find(|layer| layer.key == PaintLayerKey::default())
            .map(PaintLayer::edge_quads)
            .unwrap_or_default()
    }

    pub fn text_runs(&self) -> &[TextRun] {
        self.layers
            .iter()
            .take(self.used_layers)
            .find(|layer| layer.key == PaintLayerKey::default())
            .map(PaintLayer::text_runs)
            .unwrap_or_default()
    }

    pub fn shadows(&self) -> &[Shadow] {
        self.layers
            .iter()
            .take(self.used_layers)
            .find(|layer| layer.key == PaintLayerKey::default())
            .map(PaintLayer::shadows)
            .unwrap_or_default()
    }

    pub fn images(&self) -> &[ImagePrimitive] {
        self.layers
            .iter()
            .take(self.used_layers)
            .find(|layer| layer.key == PaintLayerKey::default())
            .map(PaintLayer::images)
            .unwrap_or_default()
    }

    pub fn svgs(&self) -> &[SvgPrimitive] {
        self.layers
            .iter()
            .take(self.used_layers)
            .find(|layer| layer.key == PaintLayerKey::default())
            .map(PaintLayer::svgs)
            .unwrap_or_default()
    }

    pub fn paths(&self) -> &[PathPrimitive] {
        self.layers
            .iter()
            .take(self.used_layers)
            .find(|layer| layer.key == PaintLayerKey::default())
            .map(PaintLayer::paths)
            .unwrap_or_default()
    }

    pub fn custom_shaders(&self) -> &[CustomShaderPrimitive] {
        self.layers
            .iter()
            .take(self.used_layers)
            .find(|layer| layer.key == PaintLayerKey::default())
            .map(PaintLayer::custom_shaders)
            .unwrap_or_default()
    }

    pub(crate) fn finish(&mut self) {
        self.layers[..self.used_layers].sort_by_key(PaintLayer::key);
    }

    pub(crate) fn paint_layers(&self) -> &[PaintLayer] {
        &self.layers[..self.used_layers]
    }

    pub(crate) fn has_content_in_plane(&self, plane: ScenePlane) -> bool {
        self.paint_layers()
            .iter()
            .any(|layer| layer.key.plane == plane && !layer.paint.is_empty())
    }

    fn layer_mut(&mut self, key: PaintLayerKey) -> &mut PaintLayer {
        if let Some(index) = self.layers[..self.used_layers]
            .iter()
            .position(|layer| layer.key == key)
        {
            return &mut self.layers[index];
        }

        let index = self.used_layers;
        self.used_layers += 1;
        if index == self.layers.len() {
            self.layers.push(PaintLayer::new(key));
        } else {
            self.layers[index].key = key;
            self.layers[index].clear();
        }
        &mut self.layers[index]
    }
}

const SHADOW_SIGMA_PER_BLUR_RADIUS: f32 = 0.5;
const SHADOW_MARGIN_SIGMAS: f32 = 3.0;

pub(crate) fn shadow_render_bounds(shadow: &Shadow) -> Option<Rect> {
    let style = shadow.style;
    let geometry = if style.is_inset() {
        shadow.element_rect
    } else {
        let subject = dilate_rect(
            shadow.element_rect.translate(style.offset()),
            style.spread(),
        );
        if !valid_bounds(subject) {
            return None;
        }
        let margin = style.blur() * SHADOW_SIGMA_PER_BLUR_RADIUS * SHADOW_MARGIN_SIGMAS + 1.0;
        dilate_rect(subject, margin)
    };
    valid_bounds(geometry).then_some(geometry)
}

fn clipped_paint_bounds<const N: usize>(
    mut bounds: Rect,
    clips: [Option<Rect>; N],
) -> Option<Rect> {
    if !valid_bounds(bounds) {
        return None;
    }
    for clip in clips.into_iter().flatten() {
        if !valid_bounds(clip) {
            return None;
        }
        bounds = bounds.intersection(clip)?;
    }
    Some(bounds)
}

fn sanitize_opacity(opacity: f32) -> f32 {
    if opacity.is_finite() {
        opacity.clamp(0.0, 1.0)
    } else {
        1.0
    }
}

fn dilate_rect(rect: Rect, amount: f32) -> Rect {
    let left = rect.x - amount;
    let top = rect.y - amount;
    let right = rect.right() + amount;
    let bottom = rect.bottom() + amount;
    let width = (right - left).max(0.0);
    let height = (bottom - top).max(0.0);
    Rect::new(
        if width > 0.0 {
            left
        } else {
            (left + right) * 0.5
        },
        if height > 0.0 {
            top
        } else {
            (top + bottom) * 0.5
        },
        width,
        height,
    )
}

impl Default for Scene {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Point;

    #[test]
    fn corner_radii_scale_uniformly_when_a_shared_edge_overflows() {
        let corners = Corners::new(40.0, 40.0, 0.0, 0.0).resolve(40.0, 100.0);
        assert_eq!(corners.top_left, 20.0);
        assert_eq!(corners.top_right, 20.0);
        assert_eq!(corners.bottom_right, 0.0);

        let fitting = Corners::new(4.0, 8.0, 12.0, 2.0).resolve(200.0, 200.0);
        assert_eq!(fitting, Corners::new(4.0, 8.0, 12.0, 2.0));
        assert_eq!(fitting.maximum(), 12.0);
        assert!(Corners::ZERO.is_zero());
        assert_eq!(Corners::top(6.0), Corners::new(6.0, 6.0, 0.0, 0.0));
        assert_eq!(Corners::bottom(6.0), Corners::new(0.0, 0.0, 6.0, 6.0));
        assert_eq!(Corners::left(6.0), Corners::new(6.0, 0.0, 0.0, 6.0));
        assert_eq!(Corners::right(6.0), Corners::new(0.0, 6.0, 6.0, 0.0));
        assert_eq!(Corners::all(f32::NAN).sanitized(), Corners::ZERO);
        assert_eq!(Corners::all(2.0).expanded(3.0), Corners::all(5.0));
        assert_eq!(Corners::all(2.0).expanded(-6.0), Corners::ZERO);
    }

    #[test]
    fn quad_backgrounds_accept_solid_colors_and_gradients() {
        let solid =
            Quad::new(Rect::new(0.0, 0.0, 10.0, 10.0), Color::BLACK).background(Color::WHITE);
        assert_eq!(solid.fill, Color::WHITE);
        assert!(solid.background.is_none());

        let gradient = Quad::new(Rect::new(0.0, 0.0, 10.0, 10.0), Color::BLACK)
            .background(crate::Gradient::conic(0.0, [Color::WHITE, Color::BLACK]))
            .corner_radii(Corners::new(1.0, 2.0, 3.0, 4.0));
        assert!(gradient.background.is_some());
        assert_eq!(gradient.radius, Corners::new(1.0, 2.0, 3.0, 4.0));
    }

    #[test]
    fn color_filters_collapse_into_one_bounded_matrix() {
        assert!(Filters::none().is_empty());
        assert!(Filters::none().color_matrix().is_identity());

        let saturated = Filters::new((0..32).map(|_| Filter::Grayscale(1.0)));
        assert_eq!(saturated.len(), MAX_FILTERS_PER_ELEMENT);
        assert_eq!(
            Filters::none().push(Filter::Invert(1.0)).len(),
            1,
            "a pushed filter is retained"
        );

        // Identity amounts leave the matrix untouched.
        assert!(ColorMatrix::from(Filter::Brightness(1.0)).is_identity());
        assert!(ColorMatrix::from(Filter::Contrast(1.0)).is_identity());
        assert!(ColorMatrix::from(Filter::Saturate(1.0)).is_identity());
        assert!(ColorMatrix::from(Filter::Grayscale(0.0)).is_identity());
        assert!(ColorMatrix::from(Filter::Invert(0.0)).is_identity());
        assert!(ColorMatrix::from(Filter::Sepia(0.0)).is_identity());
        assert!(ColorMatrix::from(Filter::Opacity(1.0)).is_identity());
        // Non-finite amounts fall back to the identity amount instead of poisoning the matrix.
        assert!(ColorMatrix::from(Filter::Brightness(f32::NAN)).is_identity());
        assert!(ColorMatrix::from(Filter::HueRotate(f32::INFINITY)).is_identity());

        // Full inversion maps one to zero.
        let invert = ColorMatrix::from(Filter::Invert(1.0)).as_array();
        assert!((invert[0] + 1.0).abs() < 0.0001);
        assert!((invert[4] - 1.0).abs() < 0.0001);

        // Composition applies the first filter first: inverting twice is the identity.
        let twice = Filters::new([Filter::Invert(1.0), Filter::Invert(1.0)]).color_matrix();
        for (value, expected) in twice
            .as_array()
            .iter()
            .zip(ColorMatrix::IDENTITY.as_array().iter())
        {
            assert!((value - expected).abs() < 0.0001, "{twice:?}");
        }

        // Opacity only scales alpha.
        let faded = ColorMatrix::from(Filter::Opacity(0.25)).as_array();
        assert_eq!(faded[18], 0.25);
        assert_eq!(faded[0], 1.0);
    }

    #[test]
    fn image_primitives_expose_grayscale_through_the_shared_color_matrix() {
        let image = crate::Image::from_rgba(1, 1, vec![255, 0, 0, 255]).unwrap();
        let primitive = ImagePrimitive::new(image, Rect::new(0.0, 0.0, 4.0, 4.0));
        assert!(primitive.color_matrix.is_identity());
        assert!(!primitive.clone().grayscale(true).color_matrix.is_identity());
        assert!(
            primitive
                .clone()
                .grayscale(true)
                .grayscale(false)
                .color_matrix
                .is_identity()
        );
    }

    #[test]
    fn named_text_ids_are_stable_and_distinct() {
        assert_eq!(TextId::named("row:42"), TextId::named("row:42"));
        assert_ne!(TextId::named("row:42"), TextId::named("row:43"));
    }

    #[test]
    fn default_text_style_uses_a_black_foreground() {
        let style = TextStyle::default();

        assert_eq!(style.font_size, 14.0);
        assert_eq!(style.color, Color::BLACK);
    }

    #[test]
    fn transparent_primitives_are_dropped() {
        let mut scene = Scene::new();
        scene.fill(Rect::new(0.0, 0.0, 10.0, 10.0), Color::TRANSPARENT);
        assert!(scene.quads().is_empty());
    }

    #[test]
    fn scoped_opacity_multiplies_retained_primitives_and_restores_exactly() {
        let mut scene = Scene::new();
        let previous = scene.multiply_opacity(0.5);
        assert_eq!(previous, 1.0);
        let parent = scene.multiply_opacity(0.5);
        assert_eq!(parent, 0.5);

        scene.push_quad(
            Quad::new(Rect::new(0.0, 0.0, 10.0, 10.0), Color::WHITE).border(1.0, Color::WHITE),
        );
        scene.push_shadow(Shadow::new(
            Rect::new(0.0, 0.0, 10.0, 10.0),
            BoxShadow::new(0.0, 1.0, Color::WHITE),
        ));
        scene.push_text(
            TextRun::new(
                TextId::new(91),
                Arc::from("faded"),
                Rect::new(0.0, 0.0, 40.0, 20.0),
                TextStyle::new(14.0, Color::WHITE),
            )
            .opacity(0.5),
        );
        let shader = CustomShader::new(
            "fn quickgui_fragment(input: QuickGuiShaderInput) -> vec4<f32> { return vec4<f32>(1.0, 1.0, 1.0, 1.0 + input.uv.x * 0.0); }",
        )
        .unwrap();
        scene.push_custom_shader(
            CustomShaderPrimitive::new(shader, Rect::new(0.0, 0.0, 10.0, 10.0)).opacity(0.5),
        );

        assert_eq!(scene.quads()[0].fill.a, 0.25);
        assert_eq!(scene.quads()[0].border_color.a, 0.25);
        assert_eq!(scene.shadows()[0].style.color().a, 0.25);
        assert_eq!(scene.text_runs()[0].opacity, 0.125);
        assert_eq!(scene.custom_shaders()[0].opacity, 0.125);

        scene.restore_opacity(parent);
        scene.restore_opacity(previous);
        scene.push_quad(Quad::new(Rect::new(20.0, 0.0, 10.0, 10.0), Color::WHITE));
        assert_eq!(scene.quads()[1].fill.a, 1.0);

        scene.multiply_opacity(0.0);
        scene.clear(Color::BLACK);
        assert_eq!(scene.current_opacity(), 1.0);
    }

    #[test]
    fn text_wraps_by_default_and_can_opt_out() {
        let style = TextStyle::new(14.0, Color::WHITE);
        assert_eq!(style.align, TextAlign::Start);
        assert_eq!(style.wrap, TextWrap::Word);
        assert_eq!(style.wrap(TextWrap::None).wrap, TextWrap::None);
    }

    #[test]
    fn box_shadow_sanitizes_non_finite_and_negative_blur_values() {
        let shadow = BoxShadow::new(f32::NAN, f32::INFINITY, Color::WHITE)
            .blur_radius(-8.0)
            .spread_radius(f32::NEG_INFINITY)
            .inset(true);
        assert_eq!(shadow.offset(), Vector::ZERO);
        assert_eq!(shadow.blur(), 0.0);
        assert_eq!(shadow.spread(), 0.0);
        assert!(shadow.is_inset());

        let clamped = BoxShadow::new(f32::MAX, f32::MIN, Color::WHITE)
            .blur_radius(f32::MAX)
            .spread_radius(f32::MAX);
        assert_eq!(clamped.offset().x, MAX_BOX_SHADOW_EXTENT);
        assert_eq!(clamped.offset().y, -MAX_BOX_SHADOW_EXTENT);
        assert_eq!(clamped.blur(), MAX_BOX_SHADOW_BLUR_RADIUS);
        assert_eq!(clamped.spread(), MAX_BOX_SHADOW_EXTENT);
    }

    #[test]
    fn shape_stream_preserves_quad_and_shadow_insertion_order() {
        let mut scene = Scene::new();
        let rect = Rect::new(10.0, 10.0, 40.0, 30.0);
        scene.push_shadow(Shadow::new(rect, BoxShadow::new(0.0, 4.0, Color::WHITE)));
        scene.push_quad(Quad::new(rect, Color::WHITE));
        scene.push_shadow(Shadow::new(
            rect,
            BoxShadow::new(0.0, 0.0, Color::WHITE).inset(true),
        ));

        let layer = &scene.paint_layers()[0];
        assert_eq!(layer.quads().len(), 1);
        assert_eq!(layer.shadows().len(), 2);
        assert_eq!(
            layer.shapes(),
            &[ShapeRef::Shadow(0), ShapeRef::Quad(0), ShapeRef::Shadow(1)]
        );
    }

    #[test]
    fn paint_stream_assigns_cross_primitive_overlap_depth() {
        let mut scene = Scene::new();
        scene.push_quad(Quad::new(Rect::new(0.0, 0.0, 10.0, 10.0), Color::WHITE));
        scene.push_text(TextRun::new(
            TextId::new(1),
            Arc::from("disjoint"),
            Rect::new(20.0, 0.0, 10.0, 10.0),
            TextStyle::new(12.0, Color::WHITE),
        ));
        scene.push_text(TextRun::new(
            TextId::new(2),
            Arc::from("bridge"),
            Rect::new(5.0, 0.0, 20.0, 10.0),
            TextStyle::new(12.0, Color::WHITE),
        ));
        scene.push_quad(Quad::new(Rect::new(6.0, 1.0, 2.0, 2.0), Color::WHITE));

        let layer = &scene.paint_layers()[0];
        assert_eq!(
            layer.paint(),
            &[
                PaintItem {
                    order: 0,
                    primitive: PrimitiveRef::Shape(ShapeRef::Quad(0)),
                },
                PaintItem {
                    order: 0,
                    primitive: PrimitiveRef::Text(0),
                },
                PaintItem {
                    order: 1,
                    primitive: PrimitiveRef::Text(1),
                },
                PaintItem {
                    order: 2,
                    primitive: PrimitiveRef::Shape(ShapeRef::Quad(1)),
                },
            ]
        );
        assert_eq!(layer.max_order(), 2);
    }

    #[test]
    fn custom_shaders_participate_in_cross_primitive_paint_order() {
        let shader = CustomShader::new(
            r#"
fn quickgui_fragment(input: QuickGuiShaderInput) -> vec4<f32> {
    return vec4<f32>(input.uv, 0.0, 1.0);
}
"#,
        )
        .unwrap();
        let mut scene = Scene::new();
        scene.push_quad(Quad::new(Rect::new(0.0, 0.0, 20.0, 20.0), Color::WHITE));
        scene.push_custom_shader(CustomShaderPrimitive::new(
            shader,
            Rect::new(5.0, 5.0, 10.0, 10.0),
        ));

        let layer = &scene.paint_layers()[0];
        assert_eq!(layer.custom_shaders().len(), 1);
        assert_eq!(layer.paint()[1].order, 1);
        assert_eq!(layer.paint()[1].primitive, PrimitiveRef::CustomShader(0));
    }

    #[test]
    fn paint_order_uses_effective_clipped_bounds() {
        let mut scene = Scene::new();
        let raw = Rect::new(0.0, 0.0, 100.0, 20.0);
        scene.push_quad(Quad::new(raw, Color::WHITE).clip(Rect::new(0.0, 0.0, 10.0, 20.0)));
        scene.push_quad(Quad::new(raw, Color::WHITE).clip(Rect::new(20.0, 0.0, 10.0, 20.0)));

        let layer = &scene.paint_layers()[0];
        assert_eq!(
            layer
                .paint()
                .iter()
                .map(|item| item.order)
                .collect::<Vec<_>>(),
            vec![0, 0]
        );
    }

    #[test]
    fn paths_participate_in_cross_primitive_overlap_order() {
        let mut builder = crate::PathBuilder::fill();
        builder.move_to(Point::new(0.0, 0.0));
        builder.line_to(Point::new(10.0, 0.0));
        builder.line_to(Point::new(10.0, 10.0));
        builder.line_to(Point::new(0.0, 10.0));
        builder.close();
        let path = builder.build().unwrap();

        let mut scene = Scene::new();
        scene.push_quad(Quad::new(Rect::new(0.0, 0.0, 10.0, 10.0), Color::WHITE));
        scene.push_path(PathPrimitive::new(&path, Color::WHITE).translate(5.0, 0.0));
        scene.push_text(TextRun::new(
            TextId::new(9),
            Arc::from("above"),
            Rect::new(12.0, 0.0, 10.0, 10.0),
            TextStyle::new(12.0, Color::WHITE),
        ));

        let layer = &scene.paint_layers()[0];
        assert_eq!(scene.paths().len(), 1);
        assert_eq!(
            layer.paint(),
            &[
                PaintItem {
                    order: 0,
                    primitive: PrimitiveRef::Shape(ShapeRef::Quad(0)),
                },
                PaintItem {
                    order: 1,
                    primitive: PrimitiveRef::Path(0),
                },
                PaintItem {
                    order: 2,
                    primitive: PrimitiveRef::Text(0),
                },
            ]
        );
    }

    #[test]
    fn path_bounds_include_scale_translation_and_effective_clip() {
        let mut builder = crate::PathBuilder::fill();
        builder.move_to(Point::new(10.0, 20.0));
        builder.line_to(Point::new(30.0, 20.0));
        builder.line_to(Point::new(30.0, 40.0));
        builder.line_to(Point::new(10.0, 40.0));
        builder.close();
        let path = builder.build().unwrap();
        let primitive = PathPrimitive::new(path, Color::WHITE)
            .scale_xy(-2.0, 3.0)
            .translate(100.0, -20.0)
            .clip(Rect::new(45.0, 45.0, 20.0, 20.0));
        assert_eq!(primitive.render_bounds(), Rect::new(40.0, 40.0, 40.0, 60.0));

        let mut scene = Scene::new();
        scene.push_path(primitive);
        scene.push_quad(Quad::new(Rect::new(66.0, 45.0, 5.0, 5.0), Color::WHITE));
        assert_eq!(
            scene.paint_layers()[0]
                .paint()
                .iter()
                .map(|item| item.order)
                .collect::<Vec<_>>(),
            vec![0, 0]
        );
    }

    #[test]
    fn paint_layers_sort_base_before_overlay_and_reuse_matching_z_indices() {
        let mut scene = Scene::new();
        let overlay = PaintLayerKey {
            plane: ScenePlane::Overlay,
            z_index: -2,
        };
        let raised_base = PaintLayerKey {
            plane: ScenePlane::Base,
            z_index: 3,
        };
        scene.push_quad_in(
            overlay,
            Quad::new(Rect::new(0.0, 0.0, 2.0, 2.0), Color::WHITE),
        );
        scene.push_quad_in(
            raised_base,
            Quad::new(Rect::new(0.0, 0.0, 2.0, 2.0), Color::WHITE),
        );
        scene.push_quad_in(
            overlay,
            Quad::new(Rect::new(2.0, 2.0, 2.0, 2.0), Color::WHITE),
        );
        scene.finish();

        let layers = scene.paint_layers();
        assert_eq!(layers.len(), 3);
        assert_eq!(layers[0].key(), PaintLayerKey::default());
        assert_eq!(layers[1].key(), raised_base);
        assert_eq!(layers[2].key(), overlay);
        assert_eq!(layers[2].quads().len(), 2);
    }
}
