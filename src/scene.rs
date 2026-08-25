use std::sync::Arc;

use glyphon::Weight;

use crate::{
    Background, Color, CustomShader, Image, Path, Rect, ShaderParameters, Svg, SvgTransform,
    TextHighlight, Vector,
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

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum FontFamily {
    SansSerif,
    Serif,
    Monospace,
    Named(Arc<str>),
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum TextWrap {
    None,
    Word,
    Glyph,
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

/// Text metrics and shaping properties. Colors do not invalidate shaping.
#[derive(Clone, Debug, PartialEq)]
pub struct TextStyle {
    pub font_size: f32,
    pub line_height: f32,
    pub family: FontFamily,
    pub weight: Weight,
    pub wrap: TextWrap,
    pub shaping: TextShaping,
    pub color: Color,
}

impl TextStyle {
    pub fn new(font_size: f32, color: Color) -> Self {
        Self {
            font_size,
            line_height: font_size * 1.35,
            family: FontFamily::SansSerif,
            weight: Weight::NORMAL,
            wrap: TextWrap::Word,
            shaping: TextShaping::Advanced,
            color,
        }
    }

    pub fn line_height(mut self, line_height: f32) -> Self {
        self.line_height = line_height;
        self
    }

    pub fn family(mut self, family: FontFamily) -> Self {
        self.family = family;
        self
    }

    pub fn weight(mut self, weight: Weight) -> Self {
        self.weight = weight;
        self
    }

    pub fn wrap(mut self, wrap: TextWrap) -> Self {
        self.wrap = wrap;
        self
    }

    pub fn shaping(mut self, shaping: TextShaping) -> Self {
        self.shaping = shaping;
        self
    }
}

/// A filled rounded rectangle with an optional inside border and clip.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Quad {
    pub rect: Rect,
    pub fill: Color,
    pub radius: f32,
    pub border_width: f32,
    pub border_color: Color,
    pub clip: Option<Rect>,
}

impl Quad {
    pub fn new(rect: Rect, fill: Color) -> Self {
        Self {
            rect,
            fill,
            radius: 0.0,
            border_width: 0.0,
            border_color: Color::TRANSPARENT,
            clip: None,
        }
    }

    pub fn radius(mut self, radius: f32) -> Self {
        self.radius = radius.max(0.0);
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
}

fn finite_or_zero(value: f32) -> f32 {
    if value.is_finite() { value } else { 0.0 }
}

/// A lower-level shadow display-list primitive.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Shadow {
    pub element_rect: Rect,
    pub radius: f32,
    pub style: BoxShadow,
    pub clip: Option<Rect>,
}

impl Shadow {
    pub fn new(element_rect: Rect, style: BoxShadow) -> Self {
        Self {
            element_rect,
            radius: 0.0,
            style,
            clip: None,
        }
    }

    pub fn radius(mut self, radius: f32) -> Self {
        self.radius = finite_or_zero(radius).max(0.0);
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
    pub grayscale: bool,
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
            grayscale: false,
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

    pub fn grayscale(mut self, grayscale: bool) -> Self {
        self.grayscale = grayscale;
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
    pub clip: Option<Rect>,
}

impl CustomShaderPrimitive {
    pub fn new(shader: impl Into<CustomShader>, rect: Rect) -> Self {
        Self {
            shader: shader.into(),
            rect,
            parameters: ShaderParameters::default(),
            clip: None,
        }
    }

    pub fn parameters(mut self, parameters: impl Into<ShaderParameters>) -> Self {
        self.parameters = parameters.into();
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
            clip: None,
            highlights: None,
        }
    }

    pub fn clip(mut self, clip: Rect) -> Self {
        self.clip = Some(clip);
        self
    }

    pub(crate) fn with_highlights(mut self, highlights: Arc<[TextHighlight]>) -> Self {
        if !highlights.is_empty() {
            self.highlights = Some(highlights);
        }
        self
    }

    fn has_visible_paint(&self) -> bool {
        self.style.color.a > 0.0
            || self.highlights.as_deref().is_some_and(|highlights| {
                highlights.iter().any(|highlight| {
                    let style = &highlight.style;
                    style.color.is_some_and(|color| color.a > 0.0)
                        || style.background.is_some_and(|color| color.a > 0.0)
                        || style.underline_color.is_some_and(|color| color.a > 0.0)
                        || style.strikethrough_color.is_some_and(|color| color.a > 0.0)
                })
            })
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
}

impl Scene {
    pub fn new() -> Self {
        Self {
            background: Color::BLACK,
            layers: vec![PaintLayer {
                key: PaintLayerKey::default(),
                quads: Vec::with_capacity(256),
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
    }

    pub fn push_quad(&mut self, quad: Quad) {
        self.push_quad_in(PaintLayerKey::default(), quad);
    }

    pub(crate) fn push_quad_in(&mut self, key: PaintLayerKey, quad: Quad) {
        if (quad.fill.a > 0.0 || quad.border_color.a > 0.0)
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

    pub fn push_shadow(&mut self, shadow: Shadow) {
        self.push_shadow_in(PaintLayerKey::default(), shadow);
    }

    pub(crate) fn push_shadow_in(&mut self, key: PaintLayerKey, shadow: Shadow) {
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

    pub(crate) fn push_image_in(&mut self, key: PaintLayerKey, image: ImagePrimitive) {
        if valid_bounds(image.source_uv)
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

    pub(crate) fn push_path_in(&mut self, key: PaintLayerKey, path: PathPrimitive) {
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
        shader: CustomShaderPrimitive,
    ) {
        if let Some(bounds) = clipped_paint_bounds(shader.rect, [shader.clip]) {
            let layer = self.layer_mut(key);
            let index = layer.custom_shaders.len();
            layer.custom_shaders.push(shader);
            layer.push_paint(bounds, PrimitiveRef::CustomShader(index));
        }
    }

    pub(crate) fn push_svg_in(&mut self, key: PaintLayerKey, svg: SvgPrimitive) {
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

    pub(crate) fn push_text_in(&mut self, key: PaintLayerKey, text: TextRun) {
        if text.has_visible_paint()
            && let Some(bounds) = clipped_paint_bounds(text.bounds, [text.clip])
        {
            let layer = self.layer_mut(key);
            let index = layer.text.len();
            layer.text.push(text);
            layer.push_paint(bounds, PrimitiveRef::Text(index));
        }
    }

    pub fn background(&self) -> Color {
        self.background
    }

    pub fn quads(&self) -> &[Quad] {
        self.layers
            .iter()
            .take(self.used_layers)
            .find(|layer| layer.key == PaintLayerKey::default())
            .map(PaintLayer::quads)
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
    fn named_text_ids_are_stable_and_distinct() {
        assert_eq!(TextId::named("row:42"), TextId::named("row:42"));
        assert_ne!(TextId::named("row:42"), TextId::named("row:43"));
    }

    #[test]
    fn transparent_primitives_are_dropped() {
        let mut scene = Scene::new();
        scene.fill(Rect::new(0.0, 0.0, 10.0, 10.0), Color::TRANSPARENT);
        assert!(scene.quads().is_empty());
    }

    #[test]
    fn text_wraps_by_default_and_can_opt_out() {
        let style = TextStyle::new(14.0, Color::WHITE);
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
