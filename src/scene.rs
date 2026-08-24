use std::sync::Arc;

use glyphon::Weight;

use crate::{Color, Rect};

const MAX_RETAINED_PAINT_LAYERS: usize = 16;

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

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum FontFamily {
    SansSerif,
    Serif,
    Monospace,
    Named(Arc<str>),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TextWrap {
    None,
    Word,
    Glyph,
}

/// Text metrics and shaping properties. Colors do not invalidate shaping.
#[derive(Clone, Debug, PartialEq)]
pub struct TextStyle {
    pub font_size: f32,
    pub line_height: f32,
    pub family: FontFamily,
    pub weight: Weight,
    pub wrap: TextWrap,
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

/// A retained text command. Reuse `id` and the same `Arc<str>` to avoid shaping and allocation.
#[derive(Clone, Debug, PartialEq)]
pub struct TextRun {
    pub id: TextId,
    pub content: Arc<str>,
    pub bounds: Rect,
    pub style: TextStyle,
    pub clip: Option<Rect>,
}

impl TextRun {
    pub fn new(id: TextId, content: Arc<str>, bounds: Rect, style: TextStyle) -> Self {
        Self {
            id,
            content,
            bounds,
            style,
            clip: None,
        }
    }

    pub fn clip(mut self, clip: Rect) -> Self {
        self.clip = Some(clip);
        self
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
    text: Vec<TextRun>,
}

impl PaintLayer {
    fn new(key: PaintLayerKey) -> Self {
        Self {
            key,
            quads: Vec::new(),
            text: Vec::new(),
        }
    }

    pub(crate) fn key(&self) -> PaintLayerKey {
        self.key
    }

    pub(crate) fn quads(&self) -> &[Quad] {
        &self.quads
    }

    pub(crate) fn text_runs(&self) -> &[TextRun] {
        &self.text
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
                text: Vec::with_capacity(128),
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
            layer.quads.clear();
            layer.text.clear();
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
        if !quad.rect.is_empty() && (quad.fill.a > 0.0 || quad.border_color.a > 0.0) {
            let layer = self.layer_mut(key);
            layer.quads.push(quad);
        }
    }

    pub fn fill(&mut self, rect: Rect, color: Color) {
        self.push_quad(Quad::new(rect, color));
    }

    pub fn push_text(&mut self, text: TextRun) {
        self.push_text_in(PaintLayerKey::default(), text);
    }

    pub(crate) fn push_text_in(&mut self, key: PaintLayerKey, text: TextRun) {
        if !text.bounds.is_empty() && text.style.color.a > 0.0 {
            let layer = self.layer_mut(key);
            layer.text.push(text);
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

    pub(crate) fn finish(&mut self) {
        self.layers[..self.used_layers].sort_by_key(PaintLayer::key);
    }

    pub(crate) fn paint_layers(&self) -> &[PaintLayer] {
        &self.layers[..self.used_layers]
    }

    pub(crate) fn has_content_in_plane(&self, plane: ScenePlane) -> bool {
        self.paint_layers().iter().any(|layer| {
            layer.key.plane == plane && (!layer.quads.is_empty() || !layer.text.is_empty())
        })
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
            self.layers[index].quads.clear();
            self.layers[index].text.clear();
        }
        &mut self.layers[index]
    }
}

impl Default for Scene {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
