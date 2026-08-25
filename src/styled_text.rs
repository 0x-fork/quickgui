use std::{ops::Range, sync::Arc};

use glyphon::{Style as GlyphStyle, Weight};

use crate::{Color, FontFamily};

/// Hard limit for the number of styled byte ranges retained by one text element.
///
/// This keeps adversarial syntax/highlight input from creating an unbounded amount of shaping
/// metadata or paint geometry in one element. Long documents should be split into visible blocks.
pub const MAX_TEXT_HIGHLIGHTS: usize = 4_096;

/// Maximum UTF-8 size of one custom family name retained by a highlighted run.
pub const MAX_HIGHLIGHT_FONT_FAMILY_BYTES: usize = 1_024;

/// Underline shape for one highlighted text range.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub enum TextUnderline {
    #[default]
    None,
    Single,
    Double,
}

/// A partial text style applied to one UTF-8 byte range in [`StyledText`].
///
/// Unspecified font properties inherit from the surrounding element. Backgrounds and text
/// decorations do not alter flexbox metrics. Foreground, font, and decoration configuration
/// participate in the retained Cosmic Text buffer key; changing only a background color does not.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct HighlightStyle {
    pub(crate) color: Option<Color>,
    pub(crate) background: Option<Color>,
    pub(crate) family: Option<FontFamily>,
    pub(crate) weight: Option<Weight>,
    pub(crate) glyph_style: Option<GlyphStyle>,
    pub(crate) underline: TextUnderline,
    pub(crate) underline_color: Option<Color>,
    pub(crate) strikethrough: bool,
    pub(crate) strikethrough_color: Option<Color>,
}

impl HighlightStyle {
    pub fn color(mut self, color: Color) -> Self {
        self.color = Some(color);
        self
    }

    pub fn background(mut self, color: Color) -> Self {
        self.background = Some(color);
        self
    }

    pub fn font_family(mut self, family: FontFamily) -> Self {
        self.family = Some(family);
        self
    }

    pub fn font_weight(mut self, weight: Weight) -> Self {
        self.weight = Some(weight);
        self
    }

    pub fn font_normal(mut self) -> Self {
        self.weight = Some(Weight::NORMAL);
        self
    }

    pub fn font_medium(mut self) -> Self {
        self.weight = Some(Weight::MEDIUM);
        self
    }

    pub fn font_semibold(mut self) -> Self {
        self.weight = Some(Weight::SEMIBOLD);
        self
    }

    pub fn font_bold(mut self) -> Self {
        self.weight = Some(Weight::BOLD);
        self
    }

    pub fn italic(mut self) -> Self {
        self.glyph_style = Some(GlyphStyle::Italic);
        self
    }

    pub fn not_italic(mut self) -> Self {
        self.glyph_style = Some(GlyphStyle::Normal);
        self
    }

    pub fn underline(mut self) -> Self {
        self.underline = TextUnderline::Single;
        self
    }

    pub fn double_underline(mut self) -> Self {
        self.underline = TextUnderline::Double;
        self
    }

    pub fn underline_color(mut self, color: Color) -> Self {
        self.underline_color = Some(color);
        if self.underline == TextUnderline::None {
            self.underline = TextUnderline::Single;
        }
        self
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

    pub(crate) fn is_empty(&self) -> bool {
        self.color.is_none()
            && self.background.is_none()
            && self.family.is_none()
            && self.weight.is_none()
            && self.glyph_style.is_none()
            && self.underline == TextUnderline::None
            && self.underline_color.is_none()
            && !self.strikethrough
            && self.strikethrough_color.is_none()
    }
}

/// One validated styled UTF-8 byte range.
#[derive(Clone, Debug, PartialEq)]
pub struct TextHighlight {
    pub(crate) range: Range<usize>,
    pub(crate) style: HighlightStyle,
}

impl TextHighlight {
    pub fn range(&self) -> Range<usize> {
        self.range.clone()
    }

    pub fn style(&self) -> &HighlightStyle {
        &self.style
    }
}

/// A retained text element with sorted, non-overlapping byte-range styles.
///
/// The complete string is shaped in one Cosmic Text buffer. Cloning this value shares both its
/// UTF-8 content and immutable highlight table.
#[derive(Clone, Debug, PartialEq)]
pub struct StyledText {
    content: Arc<str>,
    highlights: Arc<[TextHighlight]>,
}

impl StyledText {
    pub fn new(content: impl Into<Arc<str>>) -> Self {
        Self {
            content: content.into(),
            highlights: Arc::from([]),
        }
    }

    /// Replace the complete highlight table.
    ///
    /// Ranges are UTF-8 byte offsets and must be sorted, non-overlapping, in bounds, and on
    /// character boundaries. Empty ranges and empty styles are omitted. The hard run limit is
    /// checked after those no-op entries are removed.
    pub fn with_highlights(
        mut self,
        highlights: impl IntoIterator<Item = (Range<usize>, HighlightStyle)>,
    ) -> Self {
        let mut validated = Vec::new();
        let mut previous_end = 0;
        for (range, style) in highlights {
            assert!(
                range.start <= range.end && range.end <= self.content.len(),
                "styled-text range {range:?} is outside content length {}",
                self.content.len()
            );
            assert!(
                self.content.is_char_boundary(range.start)
                    && self.content.is_char_boundary(range.end),
                "styled-text range {range:?} must use UTF-8 character boundaries"
            );
            if range.is_empty() || style.is_empty() {
                continue;
            }
            assert!(
                !matches!(
                    style.family.as_ref(),
                    Some(FontFamily::Named(name)) if name.len() > MAX_HIGHLIGHT_FONT_FAMILY_BYTES
                ),
                "styled-text font family names support at most {MAX_HIGHLIGHT_FONT_FAMILY_BYTES} UTF-8 bytes"
            );
            assert!(
                range.start >= previous_end,
                "styled-text ranges must be sorted and non-overlapping"
            );
            previous_end = range.end;
            assert!(
                validated.len() < MAX_TEXT_HIGHLIGHTS,
                "styled text supports at most {MAX_TEXT_HIGHLIGHTS} non-empty highlights"
            );
            validated.push(TextHighlight { range, style });
        }
        self.highlights = validated.into();
        self
    }

    /// Add one highlight after every currently retained range.
    pub fn highlight(self, range: Range<usize>, style: HighlightStyle) -> Self {
        let mut highlights = self.highlights.to_vec();
        highlights.push(TextHighlight { range, style });
        self.with_highlights(
            highlights
                .into_iter()
                .map(|highlight| (highlight.range, highlight.style)),
        )
    }

    pub fn content(&self) -> &Arc<str> {
        &self.content
    }

    pub fn highlights(&self) -> &[TextHighlight] {
        &self.highlights
    }

    pub(crate) fn shared_highlights(&self) -> &Arc<[TextHighlight]> {
        &self.highlights
    }

    pub(crate) fn into_parts(self) -> (Arc<str>, Arc<[TextHighlight]>) {
        (self.content, self.highlights)
    }
}

/// Construct a retained styled-text element for use with `.child(...)`.
pub fn styled_text(content: impl Into<Arc<str>>) -> StyledText {
    StyledText::new(content)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn highlights_are_shared_and_keep_unicode_safe_byte_ranges() {
        let text = StyledText::new("a🙂z").with_highlights([
            (0..1, HighlightStyle::default().font_bold()),
            (
                1..5,
                HighlightStyle::default().color(Color::rgb8(56, 189, 248)),
            ),
        ]);
        let clone = text.clone();

        assert_eq!(text.highlights().len(), 2);
        assert_eq!(text.highlights()[1].range(), 1..5);
        assert!(Arc::ptr_eq(
            text.shared_highlights(),
            clone.shared_highlights()
        ));
    }

    #[test]
    #[should_panic(expected = "UTF-8 character boundaries")]
    fn highlights_reject_ranges_inside_a_unicode_scalar() {
        let _ =
            StyledText::new("🙂").with_highlights([(1..4, HighlightStyle::default().font_bold())]);
    }

    #[test]
    #[should_panic(expected = "sorted and non-overlapping")]
    fn highlights_reject_overlapping_ranges() {
        let _ = StyledText::new("abcdef").with_highlights([
            (1..4, HighlightStyle::default().font_bold()),
            (3..5, HighlightStyle::default().underline()),
        ]);
    }

    #[test]
    fn no_op_highlights_do_not_consume_retained_runs() {
        let text = StyledText::new("abc").with_highlights([
            (0..0, HighlightStyle::default().font_bold()),
            (0..3, HighlightStyle::default()),
        ]);
        assert!(text.highlights().is_empty());
    }

    #[test]
    #[should_panic(expected = "font family names support at most")]
    fn highlighted_family_names_are_byte_bounded() {
        let family = FontFamily::Named(Arc::from("x".repeat(MAX_HIGHLIGHT_FONT_FAMILY_BYTES + 1)));
        let _ = StyledText::new("x")
            .with_highlights([(0..1, HighlightStyle::default().font_family(family))]);
    }
}
