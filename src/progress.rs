use std::sync::Arc;

use crate::{AccessibilityRole, AccessibilityValueRange, Element, div};

/// Determinate or indeterminate task-completion semantics for one unstyled progress indicator.
///
/// The application owns the track, fill geometry, colors, radii, label, and any motion. QuickGUI
/// supplies the progress role, exact numeric value and bounds, and an accessibility-hidden
/// indicator part. QuickGUI never animates an indeterminate indicator: a moving barber pole is
/// product motion, and a framework-owned animation would keep an otherwise settled window awake.
///
/// The descriptor retains no allocation beyond one optional shared value string, and no task,
/// timer, observer, or idle scheduler source.
#[derive(Clone, Debug, PartialEq)]
#[must_use = "a Progress descriptor has no effect until one of its parts is mounted"]
pub struct Progress {
    value: Option<f64>,
    maximum: f64,
    value_text: Option<Arc<str>>,
}

impl Progress {
    /// Create a determinate indicator between `0.0` and `maximum`.
    ///
    /// A non-finite or non-positive maximum falls back to `1.0`; the value is clamped into range.
    pub fn new(value: f64, maximum: f64) -> Self {
        let maximum = normalized_maximum(maximum);
        Self {
            value: Some(clamped(value, 0.0, maximum)),
            maximum,
            value_text: None,
        }
    }

    /// Create a determinate indicator from a `0.0..=1.0` completion fraction.
    pub fn fraction(fraction: f64) -> Self {
        Self::new(fraction, 1.0)
    }

    /// Create an indicator for work whose completion is unknown.
    pub fn indeterminate() -> Self {
        Self {
            value: None,
            maximum: 1.0,
            value_text: None,
        }
    }

    /// Attach a human-readable value such as `"3 of 12 files"`.
    ///
    /// Assistive technology prefers this over the raw number when present. An empty string clears
    /// it. The text is not rendered; the application owns every visible label.
    pub fn value_text(mut self, text: impl Into<Arc<str>>) -> Self {
        let text = text.into();
        self.value_text = (!text.is_empty()).then_some(text);
        self
    }

    pub const fn value(&self) -> Option<f64> {
        self.value
    }

    pub const fn maximum(&self) -> f64 {
        self.maximum
    }

    pub const fn is_indeterminate(&self) -> bool {
        self.value.is_none()
    }

    /// The `0.0..=1.0` completion fraction, or `None` while indeterminate.
    ///
    /// Applications use this to size a caller-owned fill.
    pub fn completion(&self) -> Option<f32> {
        let value = self.value?;
        if self.maximum <= 0.0 {
            return Some(0.0);
        }
        Some(((value / self.maximum) as f32).clamp(0.0, 1.0))
    }

    /// Decorate an application-owned root without adding layout or appearance.
    pub fn root_part(&self, root: Element) -> Element {
        let range = match self.value {
            Some(value) => AccessibilityValueRange::new(value, 0.0, self.maximum),
            None => AccessibilityValueRange::indeterminate(0.0, self.maximum),
        };
        let root = root
            .accessibility_role(AccessibilityRole::ProgressIndicator)
            .accessibility_value_range(range);
        match &self.value_text {
            Some(text) => root.accessibility_value(text.clone()),
            None => root,
        }
    }

    /// Hide an application-owned fill or animation from the accessible name.
    pub fn indicator_part(&self, indicator: Element) -> Element {
        indicator.accessibility_hidden(true)
    }
}

/// Static measurement semantics for one unstyled meter.
///
/// A meter reports a level inside a known range—disk usage, battery charge, a score—rather than
/// the progress of a task. Optional low, high, and optimum markers let an application color the
/// gauge without inventing thresholds inside the framework.
#[derive(Clone, Copy, Debug, PartialEq)]
#[must_use = "a Meter descriptor has no effect until one of its parts is mounted"]
pub struct Meter {
    value: f64,
    minimum: f64,
    maximum: f64,
    low: Option<f64>,
    high: Option<f64>,
    optimum: Option<f64>,
}

impl Meter {
    /// Create a meter. Non-finite bounds fall back to `0.0..=1.0` and an inverted range is swapped.
    pub fn new(value: f64, minimum: f64, maximum: f64) -> Self {
        let (minimum, maximum) = if !minimum.is_finite() || !maximum.is_finite() {
            (0.0, 1.0)
        } else if maximum < minimum {
            (maximum, minimum)
        } else {
            (minimum, maximum)
        };
        Self {
            value: clamped(value, minimum, maximum),
            minimum,
            maximum,
            low: None,
            high: None,
            optimum: None,
        }
    }

    /// Mark the upper end of the low range. Values outside the meter bounds are clamped.
    pub fn low(mut self, low: f64) -> Self {
        self.low = Some(clamped(low, self.minimum, self.maximum));
        self
    }

    /// Mark the lower end of the high range.
    pub fn high(mut self, high: f64) -> Self {
        self.high = Some(clamped(high, self.minimum, self.maximum));
        self
    }

    /// Mark the most favorable value inside the range.
    pub fn optimum(mut self, optimum: f64) -> Self {
        self.optimum = Some(clamped(optimum, self.minimum, self.maximum));
        self
    }

    pub const fn value(&self) -> f64 {
        self.value
    }

    pub const fn minimum(&self) -> f64 {
        self.minimum
    }

    pub const fn maximum(&self) -> f64 {
        self.maximum
    }

    pub const fn low_value(&self) -> Option<f64> {
        self.low
    }

    pub const fn high_value(&self) -> Option<f64> {
        self.high
    }

    pub const fn optimum_value(&self) -> Option<f64> {
        self.optimum
    }

    /// The `0.0..=1.0` position of the value inside the meter's range.
    pub fn completion(&self) -> f32 {
        let span = self.maximum - self.minimum;
        if span <= 0.0 {
            return 0.0;
        }
        (((self.value - self.minimum) / span) as f32).clamp(0.0, 1.0)
    }

    /// Whether the value falls at or below [`Self::low`].
    pub fn is_low(&self) -> bool {
        self.low.is_some_and(|low| self.value <= low)
    }

    /// Whether the value falls at or above [`Self::high`].
    pub fn is_high(&self) -> bool {
        self.high.is_some_and(|high| self.value >= high)
    }

    /// Decorate an application-owned root without adding layout or appearance.
    pub fn root_part(&self, root: Element) -> Element {
        root.accessibility_role(AccessibilityRole::Meter)
            .accessibility_value_range(AccessibilityValueRange::new(
                self.value,
                self.minimum,
                self.maximum,
            ))
    }

    /// Hide an application-owned fill from the accessible name.
    pub fn indicator_part(&self, indicator: Element) -> Element {
        indicator.accessibility_hidden(true)
    }
}

/// Create an unstyled determinate progress root.
///
/// This shorthand is equivalent to `Progress::new(value, maximum).root_part(div())`.
pub fn progress(value: f64, maximum: f64) -> Element {
    Progress::new(value, maximum).root_part(div())
}

/// Create an unstyled meter root.
///
/// This shorthand is equivalent to `Meter::new(value, minimum, maximum).root_part(div())`.
pub fn meter(value: f64, minimum: f64, maximum: f64) -> Element {
    Meter::new(value, minimum, maximum).root_part(div())
}

fn normalized_maximum(maximum: f64) -> f64 {
    if maximum.is_finite() && maximum > 0.0 {
        maximum
    } else {
        1.0
    }
}

fn clamped(value: f64, minimum: f64, maximum: f64) -> f64 {
    if value.is_finite() {
        value.clamp(minimum, maximum)
    } else {
        minimum
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Color, ElementId, IntoElement, TestAppContext, View, ViewContext, text};

    #[test]
    fn progress_values_are_bounded_and_unstyled() {
        let determinate = Progress::new(3.0, 12.0).value_text("3 of 12 files");
        assert_eq!(determinate.value(), Some(3.0));
        assert_eq!(determinate.maximum(), 12.0);
        assert_eq!(determinate.completion(), Some(0.25));
        assert!(!determinate.is_indeterminate());

        assert_eq!(Progress::new(-5.0, 10.0).value(), Some(0.0));
        assert_eq!(Progress::new(50.0, 10.0).value(), Some(10.0));
        assert_eq!(Progress::new(1.0, f64::NAN).maximum(), 1.0);
        assert_eq!(Progress::new(1.0, -4.0).maximum(), 1.0);
        assert_eq!(Progress::new(f64::INFINITY, 10.0).value(), Some(0.0));
        assert_eq!(Progress::fraction(0.5).completion(), Some(0.5));

        let indeterminate = Progress::indeterminate();
        assert!(indeterminate.is_indeterminate());
        assert_eq!(indeterminate.completion(), None);

        let root = determinate.root_part(div().w(200.0).bg(Color::rgb8(1, 2, 3)));
        assert_eq!(
            root.accessibility.role,
            AccessibilityRole::ProgressIndicator
        );
        assert_eq!(
            root.accessibility.value_range.as_deref(),
            Some(&AccessibilityValueRange::new(3.0, 0.0, 12.0))
        );
        assert_eq!(root.accessibility.value.as_deref(), Some("3 of 12 files"));
        assert_eq!(root.visual.background, Some(Color::rgb8(1, 2, 3)));
        assert!(!root.focusable);
        assert!(!root.clickable);
        assert!(root.transition.is_none());
        assert!(root.animation.is_none());

        let empty_text = Progress::new(1.0, 2.0).value_text("");
        assert!(empty_text.root_part(div()).accessibility.value.is_none());

        let unknown = indeterminate.root_part(div());
        let range = unknown
            .accessibility
            .value_range
            .as_deref()
            .expect("indeterminate range");
        assert_eq!(range.value, None);
        assert_eq!(range.min, Some(0.0));
        assert_eq!(range.max, Some(1.0));

        let indicator = determinate.indicator_part(div().bg(Color::rgb8(4, 5, 6)));
        assert!(indicator.accessibility.hidden);
        assert_eq!(indicator.visual.background, Some(Color::rgb8(4, 5, 6)));
    }

    #[test]
    fn meter_thresholds_are_clamped_and_optional() {
        let meter = Meter::new(72.0, 0.0, 100.0)
            .low(20.0)
            .high(80.0)
            .optimum(50.0);
        assert_eq!(meter.value(), 72.0);
        assert_eq!(meter.low_value(), Some(20.0));
        assert_eq!(meter.high_value(), Some(80.0));
        assert_eq!(meter.optimum_value(), Some(50.0));
        assert!(!meter.is_low());
        assert!(!meter.is_high());
        assert_eq!(meter.completion(), 0.72);

        let full = Meter::new(200.0, 0.0, 100.0).high(80.0);
        assert_eq!(full.value(), 100.0);
        assert!(full.is_high());
        assert_eq!(Meter::new(0.0, 5.0, 5.0).completion(), 0.0);
        let inverted = Meter::new(1.0, 10.0, -10.0);
        assert_eq!((inverted.minimum(), inverted.maximum()), (-10.0, 10.0));
        let broken = Meter::new(0.5, f64::NAN, 4.0);
        assert_eq!((broken.minimum(), broken.maximum()), (0.0, 1.0));
        assert_eq!(Meter::new(0.0, 0.0, 10.0).low(-4.0).low_value(), Some(0.0));

        let root = meter.root_part(div().bg(Color::rgb8(7, 8, 9)));
        assert_eq!(root.accessibility.role, AccessibilityRole::Meter);
        assert_eq!(
            root.accessibility.value_range.as_deref(),
            Some(&AccessibilityValueRange::new(72.0, 0.0, 100.0))
        );
        assert_eq!(root.visual.background, Some(Color::rgb8(7, 8, 9)));
        assert!(meter.indicator_part(div()).accessibility.hidden);
    }

    struct FeedbackView;

    impl View for FeedbackView {
        fn render(&mut self, _cx: &mut ViewContext<'_, Self>) -> impl IntoElement {
            let download = Progress::new(40.0, 100.0).value_text("40 percent");
            let unknown = Progress::indeterminate();
            let disk = Meter::new(72.0, 0.0, 100.0).low(20.0).high(80.0);
            div()
                .child(
                    download
                        .root_part(div().id("download").accessibility_label("Download"))
                        .child(download.indicator_part(div().w(80.0))),
                )
                .child(
                    unknown
                        .root_part(div().id("scanning").accessibility_label("Scanning"))
                        .child(text("Scanning")),
                )
                .child(disk.root_part(div().id("disk").accessibility_label("Disk usage")))
        }
    }

    #[test]
    fn roles_and_values_reach_the_native_tree_without_idle_work() {
        let (mut cx, view) = TestAppContext::new(FeedbackView).unwrap();
        let window = view.window_handle();
        let update = cx.accessibility_update(window).unwrap();
        let node = |id: ElementId| {
            update
                .nodes
                .iter()
                .find_map(|(node_id, node)| (node_id.0 == id.as_u64()).then_some(node))
                .expect("feedback accessibility node")
        };

        let download = node("download".into());
        assert_eq!(download.role(), accesskit::Role::ProgressIndicator);
        assert_eq!(download.numeric_value(), Some(40.0));
        assert_eq!(download.min_numeric_value(), Some(0.0));
        assert_eq!(download.max_numeric_value(), Some(100.0));
        assert_eq!(download.value(), Some("40 percent"));
        assert_eq!(download.label(), Some("Download"));

        let scanning = node("scanning".into());
        assert_eq!(scanning.role(), accesskit::Role::ProgressIndicator);
        assert_eq!(scanning.numeric_value(), None);
        assert_eq!(scanning.max_numeric_value(), Some(1.0));

        let disk = node("disk".into());
        assert_eq!(disk.role(), accesskit::Role::Meter);
        assert_eq!(disk.numeric_value(), Some(72.0));

        let renders = cx.render_count(window).unwrap();
        cx.run_until_idle().unwrap();
        assert_eq!(cx.render_count(window).unwrap(), renders);
    }

    #[test]
    fn shorthands_are_semantic_unstyled_roots() {
        let bar = progress(0.5, 1.0);
        assert_eq!(bar.accessibility.role, AccessibilityRole::ProgressIndicator);
        assert!(bar.children.is_empty());
        assert_eq!(bar.visual.background, None);

        let gauge = meter(1.0, 0.0, 4.0);
        assert_eq!(gauge.accessibility.role, AccessibilityRole::Meter);
        assert!(gauge.children.is_empty());
    }
}
