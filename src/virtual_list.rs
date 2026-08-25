use std::{
    fmt,
    ops::Range,
    sync::{
        Arc,
        atomic::{AtomicU32, Ordering},
    },
};

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
#[derive(Debug)]
pub struct VirtualList {
    len: usize,
    row_height: f32,
    overscan: usize,
    viewport_height: f32,
    scroll_offset: Arc<AtomicU32>,
}

/// Shared offset storage used by a retained virtual-scroll viewport.
///
/// This stays crate-private: applications bind a [`VirtualList`] directly with
/// [`crate::Element::virtual_scroll`].
#[derive(Clone)]
pub(crate) struct VirtualScrollHandle(Arc<AtomicU32>);

impl VirtualScrollHandle {
    pub(crate) fn offset(&self) -> f32 {
        f32::from_bits(self.0.load(Ordering::Relaxed))
    }

    pub(crate) fn set_offset(&self, offset: f32) {
        debug_assert!(offset.is_finite() && offset >= 0.0);
        self.0.store(offset.to_bits(), Ordering::Relaxed);
    }
}

impl fmt::Debug for VirtualScrollHandle {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_tuple("VirtualScrollHandle")
            .field(&self.offset())
            .finish()
    }
}

impl Clone for VirtualList {
    fn clone(&self) -> Self {
        // Preserve VirtualList's value-like clone semantics. Element bindings clone only the
        // private handle above, while cloning the list itself starts an independent scroll state.
        Self {
            len: self.len,
            row_height: self.row_height,
            overscan: self.overscan,
            viewport_height: self.viewport_height,
            scroll_offset: Arc::new(AtomicU32::new(self.scroll_offset().to_bits())),
        }
    }
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
            scroll_offset: Arc::new(AtomicU32::new(0.0_f32.to_bits())),
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
        f32::from_bits(self.scroll_offset.load(Ordering::Relaxed))
    }

    pub fn max_scroll_offset(&self) -> f32 {
        (self.content_height() - self.viewport_height).max(0.0)
    }

    pub fn scroll_by(&mut self, delta: f32) -> bool {
        self.scroll_to(self.scroll_offset() + delta)
    }

    pub fn scroll_to(&mut self, offset: f32) -> bool {
        let old = self.scroll_offset();
        let next = if offset.is_finite() {
            offset.clamp(0.0, self.max_scroll_offset())
        } else {
            old
        };
        if old == next {
            false
        } else {
            self.scroll_offset.store(next.to_bits(), Ordering::Relaxed);
            true
        }
    }

    /// Scroll the smallest distance needed to expose one complete row.
    pub fn scroll_to_reveal(&mut self, index: usize) -> bool {
        if index >= self.len || self.viewport_height <= 0.0 {
            return false;
        }
        let top = index as f32 * self.row_height;
        let bottom = top + self.row_height;
        let viewport_top = self.scroll_offset();
        let viewport_bottom = viewport_top + self.viewport_height;
        if top < viewport_top {
            self.scroll_to(top)
        } else if bottom > viewport_bottom {
            self.scroll_to(bottom - self.viewport_height)
        } else {
            false
        }
    }

    pub fn visible_rows(&self) -> VisibleRows {
        if self.len == 0 || self.viewport_height <= 0.0 {
            return VisibleRows { range: 0..0 };
        }

        let scroll_offset = self.scroll_offset();
        let first = (scroll_offset / self.row_height).floor() as usize;
        let visible_end =
            ((scroll_offset + self.viewport_height) / self.row_height).ceil() as usize;
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
            viewport_y + index as f32 * self.row_height - self.scroll_offset(),
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
        let offset = travel * (self.scroll_offset() / self.max_scroll_offset());
        Some((offset, thumb_height))
    }

    fn clamp_offset(&mut self) {
        let offset = self.scroll_offset().clamp(0.0, self.max_scroll_offset());
        self.scroll_offset
            .store(offset.to_bits(), Ordering::Relaxed);
    }

    pub(crate) fn scroll_handle(&self) -> VirtualScrollHandle {
        VirtualScrollHandle(Arc::clone(&self.scroll_offset))
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

    #[test]
    fn cloned_lists_keep_independent_scroll_offsets() {
        let mut list = VirtualList::new(100, 10.0);
        list.set_viewport_height(100.0);
        list.scroll_to(240.0);
        let mut clone = list.clone();

        clone.scroll_to(500.0);

        assert_eq!(list.scroll_offset(), 240.0);
        assert_eq!(clone.scroll_offset(), 500.0);
    }

    #[test]
    fn revealing_rows_moves_only_when_the_complete_row_is_outside() {
        let mut list = VirtualList::new(100, 20.0);
        list.set_viewport_height(100.0);

        assert!(!list.scroll_to_reveal(4));
        assert!(list.scroll_to_reveal(5));
        assert_eq!(list.scroll_offset(), 20.0);
        assert!(list.scroll_to_reveal(0));
        assert_eq!(list.scroll_offset(), 0.0);
        assert!(!list.scroll_to_reveal(100));
    }
}
