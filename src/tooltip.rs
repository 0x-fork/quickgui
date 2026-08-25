use std::{fmt, rc::Rc, sync::Arc, time::Duration};

use crate::{AccessibilityRole, AnchorPlacement, Color, Element, IntoElement, div, text};

/// Native-style delay before a newly hovered tooltip becomes visible.
pub const DEFAULT_TOOLTIP_DELAY: Duration = Duration::from_millis(500);

/// Longest delay retained by one tooltip declaration.
pub const MAX_TOOLTIP_DELAY: Duration = Duration::from_secs(10);

/// Maximum detached element count accepted by one tooltip.
///
/// Tooltips are deliberately small transient surfaces. This keeps accidental dynamic content from
/// turning pointer hover into an unbounded layout operation.
pub const MAX_TOOLTIP_CONTENT_NODES: usize = 256;

/// Maximum tooltip declarations indexed for one rendered window.
pub const MAX_TOOLTIPS_PER_WINDOW: usize = 4_096;

/// A lazily displayed, pointer-passive GPU overlay attached to an element.
///
/// The content is an ordinary QuickGUI element tree. It is laid out only when its trigger survives
/// the hover delay, then retained until the tooltip hides or the application view changes.
#[derive(Clone)]
pub struct Tooltip {
    pub(crate) content: Rc<Element>,
    pub(crate) placement: AnchorPlacement,
    pub(crate) delay: Duration,
    pub(crate) gap: f32,
    pub(crate) viewport_margin: f32,
    pub(crate) accessibility_description: Option<Arc<str>>,
}

impl Tooltip {
    /// Create a tooltip from an arbitrary GPU element tree.
    pub fn new(content: impl IntoElement) -> Self {
        let content = content.into_element();
        let nodes = tooltip_node_count(&content, MAX_TOOLTIP_CONTENT_NODES + 1);
        assert!(
            nodes <= MAX_TOOLTIP_CONTENT_NODES,
            "a tooltip cannot contain more than {MAX_TOOLTIP_CONTENT_NODES} elements"
        );
        Self {
            content: Rc::new(content),
            placement: AnchorPlacement::Top,
            delay: DEFAULT_TOOLTIP_DELAY,
            gap: 7.0,
            viewport_margin: 8.0,
            accessibility_description: None,
        }
    }

    /// Create a compact native-style text tooltip.
    pub fn text(label: impl Into<Arc<str>>) -> Self {
        let label = label.into();
        Self::new(
            div()
                .max_w(360.0)
                .px_3()
                .py_2()
                .rounded_md()
                .border(1.0, Color::rgba8(255, 255, 255, 30))
                .bg(Color::rgb8(42, 45, 53))
                .shadow_md()
                .accessibility_role(AccessibilityRole::Tooltip)
                .child(
                    text(label.clone())
                        .wrap()
                        .text_sm()
                        .text_color(Color::rgb8(244, 245, 247)),
                ),
        )
        .accessibility_description(label)
    }

    pub fn placement(mut self, placement: AnchorPlacement) -> Self {
        self.placement = placement;
        self
    }

    pub fn delay(mut self, delay: Duration) -> Self {
        self.delay = delay.min(MAX_TOOLTIP_DELAY);
        self
    }

    pub fn gap(mut self, gap: f32) -> Self {
        self.gap = finite_nonnegative(gap);
        self
    }

    pub fn viewport_margin(mut self, margin: f32) -> Self {
        self.viewport_margin = finite_nonnegative(margin);
        self
    }

    /// Text exposed as the trigger's native accessibility description while the visual tooltip
    /// stays transient.
    pub fn accessibility_description(mut self, description: impl Into<Arc<str>>) -> Self {
        let description = description.into();
        self.accessibility_description = (!description.is_empty()).then_some(description);
        self
    }

    pub(crate) fn content_mut(&mut self) -> &mut Element {
        Rc::make_mut(&mut self.content)
    }

    pub(crate) fn content_identity(&self) -> *const Element {
        Rc::as_ptr(&self.content)
    }
}

impl fmt::Debug for Tooltip {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("Tooltip")
            .field("placement", &self.placement)
            .field("delay", &self.delay)
            .field("gap", &self.gap)
            .field("viewport_margin", &self.viewport_margin)
            .field("accessibility_description", &self.accessibility_description)
            .finish_non_exhaustive()
    }
}

impl From<Element> for Tooltip {
    fn from(content: Element) -> Self {
        Self::new(content)
    }
}

impl From<&str> for Tooltip {
    fn from(label: &str) -> Self {
        Self::text(Arc::<str>::from(label))
    }
}

impl From<String> for Tooltip {
    fn from(label: String) -> Self {
        Self::text(Arc::<str>::from(label))
    }
}

impl From<Arc<str>> for Tooltip {
    fn from(label: Arc<str>) -> Self {
        Self::text(label)
    }
}

fn tooltip_node_count(element: &Element, stop_after: usize) -> usize {
    let mut count = 1;
    for child in &element.children {
        count += tooltip_node_count(child, stop_after.saturating_sub(count));
        if count >= stop_after {
            break;
        }
    }
    count
}

fn finite_nonnegative(value: f32) -> f32 {
    if value.is_finite() {
        value.max(0.0)
    } else {
        0.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text_tooltips_are_accessible_bounded_and_sanitized() {
        let tooltip = Tooltip::text("Save file")
            .delay(Duration::from_secs(60))
            .gap(f32::NAN)
            .viewport_margin(f32::INFINITY);

        assert_eq!(tooltip.delay, MAX_TOOLTIP_DELAY);
        assert_eq!(tooltip.gap, 0.0);
        assert_eq!(tooltip.viewport_margin, 0.0);
        assert_eq!(
            tooltip.accessibility_description.as_deref(),
            Some("Save file")
        );
    }

    #[test]
    #[should_panic(expected = "a tooltip cannot contain more than 256 elements")]
    fn oversized_tooltip_trees_fail_at_the_api_boundary() {
        let mut content = div();
        for _ in 0..MAX_TOOLTIP_CONTENT_NODES {
            content = content.child(div());
        }
        let _ = Tooltip::new(content);
    }
}
