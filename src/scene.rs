use std::sync::Arc;

use glyphon::Weight;

use crate::{Color, Rect};

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

/// A reusable display list. [`Scene::clear`] retains its allocations.
#[derive(Debug)]
pub struct Scene {
    background: Color,
    quads: Vec<Quad>,
    text: Vec<TextRun>,
}

impl Scene {
    pub fn new() -> Self {
        Self {
            background: Color::BLACK,
            quads: Vec::with_capacity(256),
            text: Vec::with_capacity(128),
        }
    }

    pub fn clear(&mut self, background: Color) {
        self.background = background;
        self.quads.clear();
        self.text.clear();
    }

    pub fn push_quad(&mut self, quad: Quad) {
        if !quad.rect.is_empty() && (quad.fill.a > 0.0 || quad.border_color.a > 0.0) {
            self.quads.push(quad);
        }
    }

    pub fn fill(&mut self, rect: Rect, color: Color) {
        self.push_quad(Quad::new(rect, color));
    }

    pub fn push_text(&mut self, text: TextRun) {
        if !text.bounds.is_empty() && text.style.color.a > 0.0 {
            self.text.push(text);
        }
    }

    pub fn background(&self) -> Color {
        self.background
    }

    pub fn quads(&self) -> &[Quad] {
        &self.quads
    }

    pub fn text_runs(&self) -> &[TextRun] {
        &self.text
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
}
