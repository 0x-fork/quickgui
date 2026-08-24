use std::ops::Range;

use crate::Rect;

/// The visible slice of a [`VirtualList`].
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VisibleRows {
    pub range: Range<usize>,
}

impl VisibleRows {
    pub fn len(&self) -> usize {
        self.range.len()
    }

    pub fn is_empty(&self) -> bool {
        self.range.is_empty()
    }
}

/// Constant-height virtual scrolling with bounded memory and O(1) range calculation.
#[derive(Clone, Debug)]
pub struct VirtualList {
    len: usize,
    row_height: f32,
    overscan: usize,
    viewport_height: f32,
    scroll_offset: f32,
}

impl VirtualList {
    pub fn new(len: usize, row_height: f32) -> Self {
        assert!(
            row_height.is_finite() && row_height > 0.0,
            "row height must be positive"
        );
        Self {
            len,
            row_height,
            overscan: 2,
            viewport_height: 0.0,
            scroll_offset: 0.0,
        }
    }

    pub fn with_overscan(mut self, rows: usize) -> Self {
        self.overscan = rows;
        self
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub fn row_height(&self) -> f32 {
        self.row_height
    }

    pub fn content_height(&self) -> f32 {
        self.row_height * self.len as f32
    }

    pub fn viewport_height(&self) -> f32 {
        self.viewport_height
    }

    pub fn set_viewport_height(&mut self, height: f32) -> bool {
        let height = if height.is_finite() {
            height.max(0.0)
        } else {
            0.0
        };
        let changed = self.viewport_height != height;
        self.viewport_height = height;
        self.clamp_offset();
        changed
    }

    pub fn set_len(&mut self, len: usize) -> bool {
        let changed = self.len != len;
        self.len = len;
        self.clamp_offset();
        changed
    }

    pub fn scroll_offset(&self) -> f32 {
        self.scroll_offset
    }

    pub fn max_scroll_offset(&self) -> f32 {
        (self.content_height() - self.viewport_height).max(0.0)
    }

    pub fn scroll_by(&mut self, delta: f32) -> bool {
        self.scroll_to(self.scroll_offset + delta)
    }

    pub fn scroll_to(&mut self, offset: f32) -> bool {
        let old = self.scroll_offset;
        self.scroll_offset = if offset.is_finite() {
            offset.clamp(0.0, self.max_scroll_offset())
        } else {
            old
        };
        old != self.scroll_offset
    }

    pub fn visible_rows(&self) -> VisibleRows {
        if self.len == 0 || self.viewport_height <= 0.0 {
            return VisibleRows { range: 0..0 };
        }

        let first = (self.scroll_offset / self.row_height).floor() as usize;
        let visible_end =
            ((self.scroll_offset + self.viewport_height) / self.row_height).ceil() as usize;
        VisibleRows {
            range: first.saturating_sub(self.overscan)
                ..visible_end.saturating_add(self.overscan).min(self.len),
        }
    }

    /// The logical rectangle for a row inside a viewport whose top is `viewport_y`.
    pub fn row_rect(&self, index: usize, viewport_x: f32, viewport_y: f32, width: f32) -> Rect {
        debug_assert!(index < self.len);
        Rect::new(
            viewport_x,
            viewport_y + index as f32 * self.row_height - self.scroll_offset,
            width,
            self.row_height,
        )
    }

    /// Returns `(thumb_offset, thumb_height)` in viewport-local coordinates.
    pub fn scrollbar_thumb(&self, minimum_height: f32) -> Option<(f32, f32)> {
        let content = self.content_height();
        if content <= self.viewport_height || self.viewport_height <= 0.0 {
            return None;
        }
        let thumb_height = (self.viewport_height * self.viewport_height / content)
            .max(minimum_height)
            .min(self.viewport_height);
        let travel = self.viewport_height - thumb_height;
        let offset = travel * (self.scroll_offset / self.max_scroll_offset());
        Some((offset, thumb_height))
    }

    fn clamp_offset(&mut self) {
        self.scroll_offset = self.scroll_offset.clamp(0.0, self.max_scroll_offset());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_viewport_rows_and_overscan_are_returned() {
        let mut list = VirtualList::new(100_000, 20.0).with_overscan(2);
        list.set_viewport_height(100.0);
        list.scroll_to(10_000.0);
        assert_eq!(list.visible_rows().range, 498..507);
    }

    #[test]
    fn offsets_are_clamped_after_content_shrinks() {
        let mut list = VirtualList::new(100, 10.0);
        list.set_viewport_height(100.0);
        list.scroll_to(f32::MAX);
        assert_eq!(list.scroll_offset(), 900.0);
        list.set_len(5);
        assert_eq!(list.scroll_offset(), 0.0);
    }

    #[test]
    fn scrollbar_reaches_both_ends() {
        let mut list = VirtualList::new(100, 10.0);
        list.set_viewport_height(100.0);
        assert_eq!(list.scrollbar_thumb(10.0), Some((0.0, 10.0)));
        list.scroll_to(list.max_scroll_offset());
        assert_eq!(list.scrollbar_thumb(10.0), Some((90.0, 10.0)));
    }
}
