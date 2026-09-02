use crate::{
    AccessibilityOrientation, AccessibilityRole, AccessibilityValueRange, Element, ElementId,
    KeyBinding, PointerEvent, PointerPhase, Size, ViewContext, div,
};

/// Maximum thumbs retained by one slider.
///
/// A slider is a fixed-size value; the bound keeps the retained array small enough to copy while
/// still covering multi-handle range selection.
pub const MAX_SLIDER_THUMBS: usize = 8;

const SLIDER_KEY_CONTEXT: &str = "Slider";
const SLIDER_TRACK_ID_TAG: u64 = 0x5f3a_11c8_9d47_2b60;
const SLIDER_RANGE_ID_TAG: u64 = 0x18d6_7b04_ee31_a5c9;
const SLIDER_THUMB_ID_TAG: u64 = 0xc70e_2a95_4413_86fd;

/// Move the active slider thumb one step toward the maximum.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct SliderIncrement;
/// Move the active slider thumb one step toward the minimum.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct SliderDecrement;
/// Move the active slider thumb one large step toward the maximum.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct SliderLargeIncrement;
/// Move the active slider thumb one large step toward the minimum.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct SliderLargeDecrement;
/// Move the active slider thumb to its minimum.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct SliderMinimum;
/// Move the active slider thumb to its maximum.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct SliderMaximum;

/// Contextual bindings used by [`Slider::key_part`] and [`SliderThumb::key_part`].
///
/// `Shift` selects the large step on the same arrow keys, matching the desktop convention that a
/// modified arrow moves by the same amount as PageUp and PageDown.
pub fn slider_key_bindings() -> [KeyBinding; 14] {
    [
        KeyBinding::new("right", SliderIncrement, Some(SLIDER_KEY_CONTEXT)),
        KeyBinding::new("up", SliderIncrement, Some(SLIDER_KEY_CONTEXT)),
        KeyBinding::new("left", SliderDecrement, Some(SLIDER_KEY_CONTEXT)),
        KeyBinding::new("down", SliderDecrement, Some(SLIDER_KEY_CONTEXT)),
        KeyBinding::new(
            "shift-right",
            SliderLargeIncrement,
            Some(SLIDER_KEY_CONTEXT),
        ),
        KeyBinding::new("shift-up", SliderLargeIncrement, Some(SLIDER_KEY_CONTEXT)),
        KeyBinding::new("shift-left", SliderLargeDecrement, Some(SLIDER_KEY_CONTEXT)),
        KeyBinding::new("shift-down", SliderLargeDecrement, Some(SLIDER_KEY_CONTEXT)),
        KeyBinding::new("pageup", SliderLargeIncrement, Some(SLIDER_KEY_CONTEXT)),
        KeyBinding::new("pagedown", SliderLargeDecrement, Some(SLIDER_KEY_CONTEXT)),
        KeyBinding::new("home", SliderMinimum, Some(SLIDER_KEY_CONTEXT)),
        KeyBinding::new("end", SliderMaximum, Some(SLIDER_KEY_CONTEXT)),
        KeyBinding::new("platform-left", SliderMinimum, Some(SLIDER_KEY_CONTEXT)),
        KeyBinding::new("platform-right", SliderMaximum, Some(SLIDER_KEY_CONTEXT)),
    ]
}

/// Layout and keyboard axis for one slider.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum SliderOrientation {
    #[default]
    Horizontal,
    Vertical,
}

impl SliderOrientation {
    const fn accessibility(self) -> AccessibilityOrientation {
        match self {
            Self::Horizontal => AccessibilityOrientation::Horizontal,
            Self::Vertical => AccessibilityOrientation::Vertical,
        }
    }
}

/// Controlled, allocation-free bounded value state for one slider.
///
/// The state is a plain copyable value. It owns the numeric contract—bounds, step snapping, thumb
/// ordering, and the active thumb—while the application owns every visual declaration and decides
/// when a change invalidates its window. It retains no allocation, task, timer, observer, or idle
/// scheduler source.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SliderState {
    minimum: f64,
    maximum: f64,
    step: f64,
    large_step: f64,
    values: [f64; MAX_SLIDER_THUMBS],
    thumbs: usize,
    active: usize,
    orientation: SliderOrientation,
    disabled: bool,
}

impl Default for SliderState {
    fn default() -> Self {
        Self::new(0.0, 1.0, 0.0)
    }
}

impl SliderState {
    /// Create a single-thumb slider.
    ///
    /// Non-finite bounds fall back to `0.0..=1.0`, an inverted range is swapped, and an empty
    /// range keeps one degenerate value at the minimum.
    pub fn new(minimum: f64, maximum: f64, value: f64) -> Self {
        let (minimum, maximum) = normalized_bounds(minimum, maximum);
        let mut state = Self {
            minimum,
            maximum,
            step: 1.0,
            large_step: 0.0,
            values: [minimum; MAX_SLIDER_THUMBS],
            thumbs: 1,
            active: 0,
            orientation: SliderOrientation::Horizontal,
            disabled: false,
        };
        state.values[0] = state.snap(value);
        state
    }

    /// Create a multi-thumb range slider.
    ///
    /// # Panics
    ///
    /// Panics when `values` is empty or longer than [`MAX_SLIDER_THUMBS`].
    pub fn range(minimum: f64, maximum: f64, values: &[f64]) -> Self {
        assert!(!values.is_empty(), "a slider needs at least one thumb");
        assert!(
            values.len() <= MAX_SLIDER_THUMBS,
            "a slider retains at most {MAX_SLIDER_THUMBS} thumbs"
        );
        let mut state = Self::new(minimum, maximum, values[0]);
        state.thumbs = values.len();
        for (index, value) in values.iter().enumerate() {
            state.values[index] = state.snap(*value);
        }
        state.enforce_order(0);
        state
    }

    /// Replace the step used by arrow keys and pointer snapping.
    ///
    /// A non-finite or non-positive step selects continuous movement, which snaps nothing.
    #[must_use]
    pub fn step(mut self, step: f64) -> Self {
        self.step = if step.is_finite() && step > 0.0 {
            step
        } else {
            0.0
        };
        for index in 0..self.thumbs {
            self.values[index] = self.snap(self.values[index]);
        }
        self.enforce_order(0);
        self
    }

    /// Replace the amount moved by PageUp, PageDown, and shifted arrows.
    ///
    /// The default is ten ordinary steps.
    #[must_use]
    pub fn large_step(mut self, large_step: f64) -> Self {
        self.large_step = if large_step.is_finite() && large_step > 0.0 {
            large_step
        } else {
            0.0
        };
        self
    }

    #[must_use]
    pub const fn orientation(mut self, orientation: SliderOrientation) -> Self {
        self.orientation = orientation;
        self
    }

    #[must_use]
    pub const fn vertical(self) -> Self {
        self.orientation(SliderOrientation::Vertical)
    }

    #[must_use]
    pub const fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    pub const fn minimum(&self) -> f64 {
        self.minimum
    }

    pub const fn maximum(&self) -> f64 {
        self.maximum
    }

    pub const fn step_value(&self) -> f64 {
        self.step
    }

    /// The amount one large step moves.
    ///
    /// Without an explicit [`Self::large_step`] this is ten ordinary steps, or one tenth of the
    /// range for a continuous slider.
    pub const fn large_step_value(&self) -> f64 {
        if self.large_step > 0.0 {
            self.large_step
        } else if self.step > 0.0 {
            self.step * 10.0
        } else {
            (self.maximum - self.minimum) / 10.0
        }
    }

    pub const fn axis(&self) -> SliderOrientation {
        self.orientation
    }

    pub const fn is_disabled(&self) -> bool {
        self.disabled
    }

    pub const fn thumb_count(&self) -> usize {
        self.thumbs
    }

    pub const fn active_thumb(&self) -> usize {
        self.active
    }

    /// The first thumb's value, which is the whole value of a single-thumb slider.
    pub const fn value(&self) -> f64 {
        self.values[0]
    }

    pub fn values(&self) -> &[f64] {
        &self.values[..self.thumbs]
    }

    /// Read one thumb value, returning `None` past [`Self::thumb_count`].
    pub fn thumb_value(&self, index: usize) -> Option<f64> {
        (index < self.thumbs).then(|| self.values[index])
    }

    /// The inclusive bounds one thumb may move between, honoring its neighbors.
    pub fn thumb_bounds(&self, index: usize) -> Option<(f64, f64)> {
        if index >= self.thumbs {
            return None;
        }
        let lower = if index == 0 {
            self.minimum
        } else {
            self.values[index - 1]
        };
        let upper = if index + 1 == self.thumbs {
            self.maximum
        } else {
            self.values[index + 1]
        };
        Some((lower, upper))
    }

    /// Make one thumb the target of keyboard actions, returning whether it changed.
    pub fn set_active_thumb(&mut self, index: usize) -> bool {
        if index >= self.thumbs || self.active == index {
            return false;
        }
        self.active = index;
        true
    }

    /// Replace one thumb value, returning whether it changed.
    ///
    /// The value is clamped into the slider bounds, snapped to the step, and then clamped between
    /// its neighboring thumbs so ordering can never invert.
    pub fn set_thumb_value(&mut self, index: usize, value: f64) -> bool {
        if index >= self.thumbs || self.disabled {
            return false;
        }
        let Some((lower, upper)) = self.thumb_bounds(index) else {
            return false;
        };
        let value = self.snap(value).clamp(lower, upper);
        if self.values[index] == value {
            return false;
        }
        self.values[index] = value;
        true
    }

    /// Replace the first thumb value, returning whether it changed.
    pub fn set_value(&mut self, value: f64) -> bool {
        self.set_thumb_value(0, value)
    }

    /// Move the active thumb by `steps` ordinary steps, returning whether it changed.
    ///
    /// A continuous slider uses one percent of its range per step.
    pub fn step_active(&mut self, steps: f64) -> bool {
        let step = if self.step > 0.0 {
            self.step
        } else {
            (self.maximum - self.minimum) / 100.0
        };
        let index = self.active;
        let Some(current) = self.thumb_value(index) else {
            return false;
        };
        self.set_thumb_value(index, current + step * steps)
    }

    /// Move the active thumb by one large step, returning whether it changed.
    pub fn large_step_active(&mut self, forward: bool) -> bool {
        let delta = self.large_step_value();
        let index = self.active;
        let Some(current) = self.thumb_value(index) else {
            return false;
        };
        self.set_thumb_value(index, current + if forward { delta } else { -delta })
    }

    /// Move the active thumb to its lowest reachable value, returning whether it changed.
    pub fn active_to_minimum(&mut self) -> bool {
        let index = self.active;
        self.set_thumb_value(index, self.minimum)
    }

    /// Move the active thumb to its highest reachable value, returning whether it changed.
    pub fn active_to_maximum(&mut self) -> bool {
        let index = self.active;
        self.set_thumb_value(index, self.maximum)
    }

    /// The `0.0..=1.0` position of one thumb along its track.
    ///
    /// Applications use this to place a caller-owned thumb and size a caller-owned range fill.
    /// A vertical slider still returns zero at its minimum; invert it in the caller's layout.
    pub fn fraction(&self, index: usize) -> f32 {
        self.thumb_value(index)
            .map(|value| self.fraction_of(value))
            .unwrap_or(0.0)
    }

    /// The `0.0..=1.0` position of an arbitrary value along the track.
    pub fn fraction_of(&self, value: f64) -> f32 {
        let span = self.maximum - self.minimum;
        if span <= 0.0 {
            return 0.0;
        }
        (((value - self.minimum) / span) as f32).clamp(0.0, 1.0)
    }

    /// The snapped value at one `0.0..=1.0` track position.
    pub fn value_for_fraction(&self, fraction: f32) -> f64 {
        self.value_for_exact_fraction(f64::from(fraction))
    }

    fn value_for_exact_fraction(&self, fraction: f64) -> f64 {
        let fraction = if fraction.is_finite() {
            fraction.clamp(0.0, 1.0)
        } else {
            0.0
        };
        self.snap(self.minimum + fraction * (self.maximum - self.minimum))
    }

    /// The snapped value under one pointer offset measured along a track of `length` pixels.
    ///
    /// A vertical track measures from its top, where the maximum lives, matching desktop sliders.
    pub fn value_at(&self, offset: f32, length: f32) -> f64 {
        if !length.is_finite() || length <= 0.0 {
            return self.minimum;
        }
        let raw = (f64::from(offset) / f64::from(length)).clamp(0.0, 1.0);
        let fraction = match self.orientation {
            SliderOrientation::Horizontal => raw,
            SliderOrientation::Vertical => 1.0 - raw,
        };
        self.value_for_exact_fraction(fraction)
    }

    /// The thumb whose value is closest to `value`, preferring the lower index on a tie.
    pub fn nearest_thumb(&self, value: f64) -> usize {
        let mut best = 0;
        let mut distance = f64::INFINITY;
        for index in 0..self.thumbs {
            let candidate = (self.values[index] - value).abs();
            if candidate < distance {
                distance = candidate;
                best = index;
            }
        }
        best
    }

    /// Apply one captured pointer event over a track of the given size.
    ///
    /// The application owns the track's layout, so it passes the size it declared. A press picks
    /// the nearest thumb, makes it active, and jumps it to the pointer; subsequent moves drag that
    /// same thumb even outside the track. Returns whether any value or the active thumb changed.
    pub fn apply_pointer(&mut self, event: &PointerEvent, track: Size) -> bool {
        if self.disabled {
            return false;
        }
        let (offset, length) = match self.orientation {
            SliderOrientation::Horizontal => (event.local_position.x, track.width),
            SliderOrientation::Vertical => (event.local_position.y, track.height),
        };
        let value = self.value_at(offset, length);
        let mut changed = false;
        if event.phase == PointerPhase::Down {
            changed |= self.set_active_thumb(self.nearest_thumb(value));
        }
        if matches!(
            event.phase,
            PointerPhase::Down | PointerPhase::Move | PointerPhase::Up
        ) {
            let index = self.active;
            changed |= self.set_thumb_value(index, value);
        }
        changed
    }

    fn snap(&self, value: f64) -> f64 {
        if !value.is_finite() {
            return self.minimum;
        }
        let value = value.clamp(self.minimum, self.maximum);
        if self.step <= 0.0 {
            return value;
        }
        let steps = ((value - self.minimum) / self.step).round();
        (self.minimum + steps * self.step).clamp(self.minimum, self.maximum)
    }

    fn enforce_order(&mut self, from: usize) {
        for index in from.max(1)..self.thumbs {
            if self.values[index] < self.values[index - 1] {
                self.values[index] = self.values[index - 1];
            }
        }
    }
}

fn normalized_bounds(minimum: f64, maximum: f64) -> (f64, f64) {
    if !minimum.is_finite() || !maximum.is_finite() {
        return (0.0, 1.0);
    }
    if maximum < minimum {
        (maximum, minimum)
    } else {
        (minimum, maximum)
    }
}

/// A controlled, unstyled slider descriptor.
///
/// The application owns the track, range fill, thumb, tick marks, labels, colors, and motion.
/// QuickGUI supplies stable part identities, the Slider role with numeric value/min/max/step and
/// orientation, captured pointer arithmetic, and typed keyboard actions.
///
/// A single-thumb slider projects the Slider role on its root. A multi-thumb slider projects a
/// group root and one Slider role per thumb, each bounded by its neighbors, matching the WAI-ARIA
/// multi-thumb pattern.
#[derive(Clone, Copy, Debug, PartialEq)]
#[must_use = "a Slider descriptor has no effect until its parts are mounted"]
pub struct Slider {
    root_id: ElementId,
    state: SliderState,
}

impl Slider {
    pub fn new(root_id: impl Into<ElementId>, state: &SliderState) -> Self {
        Self {
            root_id: root_id.into(),
            state: *state,
        }
    }

    pub const fn root_id(self) -> ElementId {
        self.root_id
    }

    pub const fn state(self) -> SliderState {
        self.state
    }

    pub fn track_id(self) -> ElementId {
        derived_slider_id(self.root_id, SLIDER_TRACK_ID_TAG, 0)
    }

    pub fn range_id(self) -> ElementId {
        derived_slider_id(self.root_id, SLIDER_RANGE_ID_TAG, 0)
    }

    pub fn thumb_id(self, index: usize) -> ElementId {
        derived_slider_id(self.root_id, SLIDER_THUMB_ID_TAG, index as u64)
    }

    /// Whether this slider projects its value on the root instead of on each thumb.
    pub const fn is_single_thumb(self) -> bool {
        self.state.thumbs == 1
    }

    /// Decorate an application-owned root without adding layout or appearance.
    ///
    /// A single-thumb slider becomes the focusable Slider itself. Pair it with
    /// [`Self::key_part`] to attach the typed keyboard actions.
    pub fn root_part(self, root: Element) -> Element {
        let disabled = self.state.disabled || root.accessibility.disabled;
        let root = root
            .id(self.root_id)
            .cursor_default()
            .app_region_no_drag()
            .user_select_none()
            .disabled(disabled);
        if self.is_single_thumb() {
            root.accessibility_role(AccessibilityRole::Slider)
                .accessibility_orientation(self.state.orientation.accessibility())
                .accessibility_value_range(
                    AccessibilityValueRange::new(
                        self.state.values[0],
                        self.state.minimum,
                        self.state.maximum,
                    )
                    .step(self.state.step),
                )
                .focusable()
                .tab_index(0)
                .key_context(SLIDER_KEY_CONTEXT)
        } else {
            root.accessibility_role(AccessibilityRole::Group)
                .accessibility_orientation(self.state.orientation.accessibility())
        }
    }

    /// Attach QuickGUI's typed slider keyboard actions to a single-thumb root.
    ///
    /// Install [`slider_key_bindings`] once on the application keymap. Multi-thumb sliders use
    /// [`SliderThumb::key_part`] instead so each thumb owns its own focus.
    pub fn key_part<V: 'static>(
        self,
        cx: &mut ViewContext<'_, V>,
        root: Element,
        access: fn(&mut V) -> &mut SliderState,
    ) -> Element {
        bind_slider_actions(cx, root, self.root_id, access, None)
    }

    /// Decorate an application-owned track.
    ///
    /// The track carries the slider's pointer capture. Attach a
    /// [`crate::ViewContext::pointer_listener`] registered for [`Self::track_id`] and forward the
    /// event to [`SliderState::apply_pointer`] with the size the caller laid out.
    pub fn track_part(self, track: Element) -> Element {
        track
            .id(self.track_id())
            .accessibility_hidden(true)
            .cursor_default()
            .app_region_no_drag()
            .user_select_none()
    }

    /// Decorate the application-owned fill between the slider's minimum and its value.
    pub fn range_part(self, range: Element) -> Element {
        range.id(self.range_id()).accessibility_hidden(true)
    }

    /// Describe one thumb.
    ///
    /// Indices at or past [`SliderState::thumb_count`] return `None` so a caller-driven loop stays
    /// bounded by the state instead of by its own arithmetic.
    pub fn thumb(self, index: usize) -> Option<SliderThumb> {
        let (lower, upper) = self.state.thumb_bounds(index)?;
        Some(SliderThumb {
            slider: self,
            index,
            value: self.state.values[index],
            lower,
            upper,
        })
    }
}

/// A copyable declaration for one slider thumb.
#[derive(Clone, Copy, Debug, PartialEq)]
#[must_use = "a SliderThumb descriptor has no effect until its part is mounted"]
pub struct SliderThumb {
    slider: Slider,
    index: usize,
    value: f64,
    lower: f64,
    upper: f64,
}

impl SliderThumb {
    pub const fn index(self) -> usize {
        self.index
    }

    pub const fn value(self) -> f64 {
        self.value
    }

    /// The inclusive bounds this thumb may move between, honoring its neighbors.
    pub const fn bounds(self) -> (f64, f64) {
        (self.lower, self.upper)
    }

    /// The `0.0..=1.0` position of this thumb along the track.
    pub fn fraction(self) -> f32 {
        self.slider.state.fraction_of(self.value)
    }

    pub const fn is_active(self) -> bool {
        self.slider.state.active == self.index
    }

    pub fn thumb_id(self) -> ElementId {
        self.slider.thumb_id(self.index)
    }

    /// Decorate an application-owned thumb without adding layout or appearance.
    ///
    /// A single-thumb slider keeps its value on the root, so its thumb is decorative and hidden
    /// from assistive technology. Every thumb of a multi-thumb slider is an independently
    /// focusable Slider bounded by its neighbors.
    pub fn thumb_part(self, thumb: Element) -> Element {
        let disabled = self.slider.state.disabled || thumb.accessibility.disabled;
        let thumb = thumb.id(self.thumb_id());
        if self.slider.is_single_thumb() {
            return thumb.accessibility_hidden(true);
        }
        thumb
            .accessibility_role(AccessibilityRole::Slider)
            .accessibility_orientation(self.slider.state.orientation.accessibility())
            .accessibility_value_range(
                AccessibilityValueRange::new(self.value, self.lower, self.upper)
                    .step(self.slider.state.step),
            )
            .focusable()
            .tab_index(0)
            .key_context(SLIDER_KEY_CONTEXT)
            .cursor_default()
            .app_region_no_drag()
            .user_select_none()
            .disabled(disabled)
    }

    /// Attach QuickGUI's typed slider keyboard actions to this thumb.
    ///
    /// Focusing the thumb also makes it the active thumb, so arrows move the thumb the user sees.
    pub fn key_part<V: 'static>(
        self,
        cx: &mut ViewContext<'_, V>,
        thumb: Element,
        access: fn(&mut V) -> &mut SliderState,
    ) -> Element {
        bind_slider_actions(cx, thumb, self.thumb_id(), access, Some(self.index))
    }
}

fn bind_slider_actions<V: 'static>(
    cx: &mut ViewContext<'_, V>,
    element: Element,
    id: ElementId,
    access: fn(&mut V) -> &mut SliderState,
    thumb: Option<usize>,
) -> Element {
    let activate = move |view: &mut V| {
        if let Some(index) = thumb {
            access(view).set_active_thumb(index);
        }
    };
    let increment = cx.action_listener(id, move |view, _: &SliderIncrement, cx| {
        activate(view);
        if access(view).step_active(1.0) {
            cx.invalidate();
        }
    });
    let decrement = cx.action_listener(id, move |view, _: &SliderDecrement, cx| {
        activate(view);
        if access(view).step_active(-1.0) {
            cx.invalidate();
        }
    });
    let large_increment = cx.action_listener(id, move |view, _: &SliderLargeIncrement, cx| {
        activate(view);
        if access(view).large_step_active(true) {
            cx.invalidate();
        }
    });
    let large_decrement = cx.action_listener(id, move |view, _: &SliderLargeDecrement, cx| {
        activate(view);
        if access(view).large_step_active(false) {
            cx.invalidate();
        }
    });
    let minimum = cx.action_listener(id, move |view, _: &SliderMinimum, cx| {
        activate(view);
        if access(view).active_to_minimum() {
            cx.invalidate();
        }
    });
    let maximum = cx.action_listener(id, move |view, _: &SliderMaximum, cx| {
        activate(view);
        if access(view).active_to_maximum() {
            cx.invalidate();
        }
    });
    element
        .on_action(increment)
        .on_action(decrement)
        .on_action(large_increment)
        .on_action(large_decrement)
        .on_action(minimum)
        .on_action(maximum)
}

/// Create an unstyled controlled single-thumb slider root.
///
/// This shorthand is equivalent to `Slider::new(id, state).root_part(div())`.
pub fn slider(id: impl Into<ElementId>, state: &SliderState) -> Element {
    Slider::new(id, state).root_part(div())
}

fn derived_slider_id(scope: ElementId, tag: u64, index: u64) -> ElementId {
    let mut hash = scope
        .as_u64()
        .rotate_left(23)
        .wrapping_add(index.rotate_right(11))
        ^ tag;
    hash ^= hash >> 30;
    hash = hash.wrapping_mul(0xbf58_476d_1ce4_e5b9);
    hash ^= hash >> 27;
    hash = hash.wrapping_mul(0x94d0_49bb_1331_11eb);
    hash ^= hash >> 31;
    if hash == 0 || hash == u64::MAX || hash == scope.as_u64() {
        hash ^= tag.rotate_left(31);
    }
    ElementId::new(hash)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        AppRegion, Application, Color, CursorStyle, IntoElement, Modifiers, MouseButton, Point,
        UserSelect, Vector, View, WindowOptions, text,
    };

    fn pointer_event(phase: PointerPhase, x: f32, y: f32) -> PointerEvent {
        PointerEvent {
            phase,
            position: Point::new(x, y),
            origin: Point::new(x, y),
            local_position: Point::new(x, y),
            local_origin: Point::new(x, y),
            delta: Vector::ZERO,
            button: MouseButton::Left,
            modifiers: Modifiers::empty(),
        }
    }

    #[test]
    fn captured_pointer_selects_then_drags_one_thumb() {
        let mut state = SliderState::range(0.0, 100.0, &[20.0, 80.0]).step(10.0);
        let track = Size::new(200.0, 20.0);

        assert!(state.apply_pointer(&pointer_event(PointerPhase::Down, 150.0, 10.0), track));
        assert_eq!(state.active_thumb(), 1);
        assert_eq!(state.values(), &[20.0, 80.0]);

        assert!(state.apply_pointer(&pointer_event(PointerPhase::Move, 190.0, 10.0), track));
        assert_eq!(state.values(), &[20.0, 100.0]);

        // Capture continues outside the track and stays clamped by the lower thumb.
        assert!(state.apply_pointer(&pointer_event(PointerPhase::Move, -400.0, 10.0), track));
        assert_eq!(state.values(), &[20.0, 20.0]);
        assert!(!state.apply_pointer(&pointer_event(PointerPhase::Up, -400.0, 10.0), track));

        let mut vertical = SliderState::new(0.0, 100.0, 0.0).step(10.0).vertical();
        // Pressing the bottom of a vertical track is already the minimum, so nothing changes.
        assert!(!vertical.apply_pointer(&pointer_event(PointerPhase::Down, 5.0, 20.0), track));
        assert_eq!(vertical.value(), 0.0);
        assert!(vertical.apply_pointer(&pointer_event(PointerPhase::Down, 5.0, 5.0), track));
        assert_eq!(vertical.value(), 80.0);

        let mut disabled = SliderState::new(0.0, 1.0, 0.5).disabled(true);
        assert!(!disabled.apply_pointer(&pointer_event(PointerPhase::Down, 200.0, 0.0), track));
        assert!(!state.apply_pointer(&pointer_event(PointerPhase::Cancel, 0.0, 0.0), track));
    }

    #[test]
    fn state_snaps_clamps_and_orders_bounded_values() {
        let mut state = SliderState::new(0.0, 100.0, 42.0).step(5.0);
        assert_eq!(state.minimum(), 0.0);
        assert_eq!(state.maximum(), 100.0);
        assert_eq!(state.value(), 40.0);
        assert!(state.set_value(97.0));
        assert_eq!(state.value(), 95.0);
        assert!(!state.set_value(96.0));
        assert!(state.set_value(1_000.0));
        assert_eq!(state.value(), 100.0);
        assert!(state.set_value(f64::NAN));
        assert_eq!(state.value(), 0.0);

        assert!(state.step_active(1.0));
        assert_eq!(state.value(), 5.0);
        assert!(state.large_step_active(true));
        assert_eq!(state.value(), 55.0);
        assert!(state.large_step_active(false));
        assert_eq!(state.value(), 5.0);
        assert!(state.active_to_maximum());
        assert_eq!(state.value(), 100.0);
        assert!(state.active_to_minimum());
        assert_eq!(state.value(), 0.0);
        assert!(!state.active_to_minimum());

        let inverted = SliderState::new(10.0, -10.0, 0.0);
        assert_eq!(inverted.minimum(), -10.0);
        assert_eq!(inverted.maximum(), 10.0);
        let broken = SliderState::new(f64::NAN, f64::INFINITY, 0.5);
        assert_eq!((broken.minimum(), broken.maximum()), (0.0, 1.0));

        let mut range = SliderState::range(0.0, 10.0, &[8.0, 2.0]);
        assert_eq!(range.thumb_count(), 2);
        assert_eq!(range.values(), &[8.0, 8.0]);
        assert!(range.set_thumb_value(0, 3.0));
        assert_eq!(range.values(), &[3.0, 8.0]);
        assert!(range.set_thumb_value(1, 1.0));
        assert_eq!(range.values(), &[3.0, 3.0]);
        assert_eq!(range.thumb_bounds(0), Some((0.0, 3.0)));
        assert_eq!(range.thumb_bounds(1), Some((3.0, 10.0)));
        assert_eq!(range.thumb_bounds(2), None);
        assert!(!range.set_thumb_value(2, 5.0));
        assert!(range.set_active_thumb(1));
        assert!(!range.set_active_thumb(1));
        assert!(!range.set_active_thumb(9));
        assert_eq!(range.active_thumb(), 1);

        let disabled = &mut SliderState::new(0.0, 1.0, 0.5).disabled(true);
        assert!(!disabled.set_value(0.75));
        assert!(disabled.is_disabled());
    }

    #[test]
    fn pointer_geometry_is_orientation_aware_and_bounded() {
        let horizontal = SliderState::new(0.0, 200.0, 0.0).step(0.0);
        assert_eq!(horizontal.value_at(50.0, 100.0), 100.0);
        assert_eq!(horizontal.value_at(-40.0, 100.0), 0.0);
        assert_eq!(horizontal.value_at(400.0, 100.0), 200.0);
        assert_eq!(horizontal.value_at(50.0, 0.0), 0.0);
        assert_eq!(horizontal.fraction_of(50.0), 0.25);

        let vertical = SliderState::new(0.0, 200.0, 0.0).step(0.0).vertical();
        assert_eq!(vertical.value_at(0.0, 100.0), 200.0);
        assert_eq!(vertical.value_at(100.0, 100.0), 0.0);
        assert_eq!(vertical.value_at(25.0, 100.0), 150.0);

        let degenerate = SliderState::new(5.0, 5.0, 5.0);
        assert_eq!(degenerate.fraction(0), 0.0);
        assert_eq!(degenerate.value_for_fraction(1.0), 5.0);
        assert_eq!(degenerate.value_for_fraction(f32::NAN), 5.0);
    }

    #[test]
    fn parts_add_exact_semantics_without_appearance() {
        let state = SliderState::new(0.0, 10.0, 4.0).step(2.0);
        let slider = Slider::new("volume", &state);
        let root = slider.root_part(div().w(240.0).bg(Color::rgb8(1, 2, 3)));
        assert_eq!(root.explicit_id, Some("volume".into()));
        assert_eq!(root.accessibility.role, AccessibilityRole::Slider);
        assert_eq!(
            root.accessibility.orientation,
            Some(AccessibilityOrientation::Horizontal)
        );
        assert_eq!(
            root.accessibility.value_range.as_deref(),
            Some(&AccessibilityValueRange::new(4.0, 0.0, 10.0).step(2.0))
        );
        assert!(root.focusable);
        assert_eq!(root.tab_index, 0);
        assert_eq!(root.cursor_style, Some(CursorStyle::Arrow));
        assert_eq!(root.app_region, Some(AppRegion::NoDrag));
        assert_eq!(root.user_select, UserSelect::None);
        assert_eq!(root.visual.background, Some(Color::rgb8(1, 2, 3)));
        assert!(root.transition.is_none());

        let track = slider.track_part(div().h(4.0).bg(Color::rgb8(4, 5, 6)));
        assert_eq!(track.explicit_id, Some(slider.track_id()));
        assert!(track.accessibility.hidden);
        assert_eq!(track.visual.background, Some(Color::rgb8(4, 5, 6)));

        let range = slider.range_part(div().bg(Color::rgb8(7, 8, 9)));
        assert_eq!(range.explicit_id, Some(slider.range_id()));
        assert!(range.accessibility.hidden);

        let thumb = slider.thumb(0).expect("first thumb");
        assert_eq!(thumb.value(), 4.0);
        assert_eq!(thumb.fraction(), 0.4);
        assert!(thumb.is_active());
        let thumb_element = thumb.thumb_part(div().size(12.0, 12.0));
        assert_eq!(thumb_element.explicit_id, Some(slider.thumb_id(0)));
        assert!(thumb_element.accessibility.hidden);
        assert!(slider.thumb(1).is_none());

        let ids = [
            slider.root_id(),
            slider.track_id(),
            slider.range_id(),
            slider.thumb_id(0),
            slider.thumb_id(1),
        ];
        for (index, id) in ids.iter().enumerate() {
            assert!(!ids[..index].contains(id));
        }

        let range_state = SliderState::range(0.0, 100.0, &[20.0, 80.0]).step(10.0);
        let range_slider = Slider::new("price", &range_state);
        let range_root = range_slider.root_part(div());
        assert_eq!(range_root.accessibility.role, AccessibilityRole::Group);
        assert!(!range_root.focusable);
        let lower = range_slider.thumb(0).expect("lower thumb");
        let lower_element = lower.thumb_part(div());
        assert_eq!(lower_element.accessibility.role, AccessibilityRole::Slider);
        assert_eq!(
            lower_element.accessibility.value_range.as_deref(),
            Some(&AccessibilityValueRange::new(20.0, 0.0, 80.0).step(10.0))
        );
        assert!(lower_element.focusable);
        let upper = range_slider.thumb(1).expect("upper thumb");
        assert_eq!(upper.bounds(), (20.0, 100.0));
        assert!(!upper.is_active());

        let disabled_state = SliderState::new(0.0, 1.0, 0.5).disabled(true);
        let disabled = Slider::new("muted", &disabled_state).root_part(div());
        assert!(disabled.accessibility.disabled);
    }

    #[derive(Default)]
    struct SliderView {
        volume: SliderState,
        price: SliderState,
    }

    impl SliderView {
        fn build() -> Self {
            Self {
                volume: SliderState::new(0.0, 100.0, 40.0).step(5.0),
                price: SliderState::range(0.0, 100.0, &[20.0, 80.0]).step(10.0),
            }
        }

        fn volume(view: &mut Self) -> &mut SliderState {
            &mut view.volume
        }

        fn price(view: &mut Self) -> &mut SliderState {
            &mut view.price
        }
    }

    impl View for SliderView {
        fn render(&mut self, cx: &mut ViewContext<'_, Self>) -> impl IntoElement {
            let volume = Slider::new("volume", &self.volume);
            let volume_root = volume.key_part(cx, volume.root_part(div()), Self::volume);
            let drag = cx.pointer_listener(volume.track_id(), |view, event, cx| {
                if view.volume.apply_pointer(event, Size::new(200.0, 20.0)) {
                    cx.invalidate();
                }
            });
            let volume_thumb = volume.thumb(0).expect("volume thumb");

            let price = Slider::new("price", &self.price);
            let lower = price.thumb(0).expect("lower thumb");
            let upper = price.thumb(1).expect("upper thumb");
            let lower_element = lower.key_part(cx, lower.thumb_part(div()), Self::price);
            let upper_element = upper.key_part(cx, upper.thumb_part(div()), Self::price);

            div()
                .child(text("Slider gallery"))
                .child(
                    volume_root.child(
                        volume
                            .track_part(div().w(200.0).h(20.0).on_pointer(drag))
                            .child(volume.range_part(div()))
                            .child(volume_thumb.thumb_part(div())),
                    ),
                )
                .child(
                    price
                        .root_part(div())
                        .child(price.track_part(div().w(200.0).h(20.0)))
                        .child(lower_element)
                        .child(upper_element),
                )
        }
    }

    #[test]
    fn keyboard_pointer_and_accessibility_paths_stay_deterministic() {
        let (mut cx, view) = Application::new()
            .bind_keys(slider_key_bindings())
            .into_test_context(WindowOptions::default(), SliderView::build())
            .unwrap();
        let window = view.window_handle();
        let volume = Slider::new("volume", &SliderState::new(0.0, 100.0, 40.0));
        let price = Slider::new("price", &SliderState::range(0.0, 100.0, &[20.0, 80.0]));

        cx.focus(window, volume.root_id()).unwrap();
        cx.simulate_keystrokes(window, "right").unwrap();
        assert_eq!(cx.read(view, |view| view.volume.value()).unwrap(), 45.0);
        cx.simulate_keystrokes(window, "left left").unwrap();
        assert_eq!(cx.read(view, |view| view.volume.value()).unwrap(), 35.0);
        cx.simulate_keystrokes(window, "pageup").unwrap();
        assert_eq!(cx.read(view, |view| view.volume.value()).unwrap(), 85.0);
        cx.simulate_keystrokes(window, "shift-down").unwrap();
        assert_eq!(cx.read(view, |view| view.volume.value()).unwrap(), 35.0);
        cx.simulate_keystrokes(window, "home").unwrap();
        assert_eq!(cx.read(view, |view| view.volume.value()).unwrap(), 0.0);
        cx.simulate_keystrokes(window, "end").unwrap();
        assert_eq!(cx.read(view, |view| view.volume.value()).unwrap(), 100.0);

        cx.focus(window, price.thumb_id(1)).unwrap();
        cx.simulate_keystrokes(window, "right").unwrap();
        assert_eq!(
            cx.read(view, |view| view.price.values().to_vec()).unwrap(),
            vec![20.0, 90.0]
        );
        cx.simulate_keystrokes(window, "home").unwrap();
        assert_eq!(
            cx.read(view, |view| view.price.values().to_vec()).unwrap(),
            vec![20.0, 20.0]
        );

        let update = cx.accessibility_update(window).unwrap();
        let node = |id: ElementId| {
            update
                .nodes
                .iter()
                .find_map(|(node_id, node)| (node_id.0 == id.as_u64()).then_some(node))
                .expect("slider accessibility node")
        };
        let root = node(volume.root_id());
        assert_eq!(root.role(), accesskit::Role::Slider);
        assert_eq!(root.numeric_value(), Some(100.0));
        assert_eq!(root.min_numeric_value(), Some(0.0));
        assert_eq!(root.max_numeric_value(), Some(100.0));
        assert_eq!(root.numeric_value_step(), Some(5.0));
        assert_eq!(root.orientation(), Some(accesskit::Orientation::Horizontal));
        let upper = node(price.thumb_id(1));
        assert_eq!(upper.role(), accesskit::Role::Slider);
        assert_eq!(upper.numeric_value(), Some(20.0));
        assert_eq!(upper.min_numeric_value(), Some(20.0));
        assert_eq!(upper.max_numeric_value(), Some(100.0));

        let renders = cx.render_count(window).unwrap();
        cx.run_until_idle().unwrap();
        assert_eq!(cx.render_count(window).unwrap(), renders);
    }

    #[test]
    fn bindings_are_contextual_and_complete() {
        let bindings = slider_key_bindings();
        assert_eq!(bindings.len(), 14);
        assert!(
            bindings.iter().all(
                |binding| binding.context_predicate().is_some_and(|context| context
                    .depth_of(&[crate::KeyContext::parse(SLIDER_KEY_CONTEXT).unwrap()])
                    .is_some())
            )
        );
    }

    #[test]
    #[should_panic(expected = "a slider needs at least one thumb")]
    fn empty_range_is_rejected() {
        let _ = SliderState::range(0.0, 1.0, &[]);
    }

    #[test]
    #[should_panic(expected = "at most")]
    fn too_many_thumbs_are_rejected() {
        let _ = SliderState::range(0.0, 1.0, &[0.0; MAX_SLIDER_THUMBS + 1]);
    }

    #[test]
    fn shorthand_is_a_semantic_unstyled_root() {
        let state = SliderState::new(0.0, 1.0, 0.25);
        let element = slider("brightness", &state);
        assert_eq!(element.accessibility.role, AccessibilityRole::Slider);
        assert!(element.children.is_empty());
        assert_eq!(element.visual.background, None);
    }
}
