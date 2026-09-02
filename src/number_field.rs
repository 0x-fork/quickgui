use std::{
    sync::Arc,
    time::{Duration, Instant},
};

use crate::{AccessibilityRole, AccessibilityValueRange, Element, ElementId, div, text_input};

/// Maximum UTF-8 bytes retained by one number field's editing text.
///
/// Numbers are short. The bound keeps a paste of arbitrary application or clipboard text from
/// turning one controlled field into an unbounded string.
pub const MAX_NUMBER_FIELD_TEXT_BYTES: usize = 64;

/// Maximum fractional digits one number field formats.
pub const MAX_NUMBER_FIELD_PRECISION: u8 = 15;

/// Delay before a held increment or decrement button starts repeating.
pub const NUMBER_FIELD_REPEAT_DELAY: Duration = Duration::from_millis(400);

/// Interval between repeats while an increment or decrement button stays held.
pub const NUMBER_FIELD_REPEAT_INTERVAL: Duration = Duration::from_millis(60);

const NUMBER_FIELD_INPUT_ID_TAG: u64 = 0x6a2f_c391_bd47_50e8;
const NUMBER_FIELD_INCREMENT_ID_TAG: u64 = 0xb185_7e2c_04af_39d6;
const NUMBER_FIELD_DECREMENT_ID_TAG: u64 = 0x27ce_4a80_f6d1_9b53;

/// Locale-shaped parsing and formatting rules for one number field.
///
/// QuickGUI deliberately has no locale database. The application supplies the separators its
/// users expect, which keeps the framework free of an unbounded data table while still supporting
/// `1 234,56` as readily as `1,234.56`.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NumberFieldFormat {
    decimal: char,
    group: Option<char>,
    sign: bool,
    exponent: bool,
    precision: Option<u8>,
}

impl Default for NumberFieldFormat {
    fn default() -> Self {
        Self {
            decimal: '.',
            group: None,
            sign: true,
            exponent: false,
            precision: None,
        }
    }
}

impl NumberFieldFormat {
    pub fn new() -> Self {
        Self::default()
    }

    /// Replace the decimal separator. A digit or ASCII sign is rejected and keeps the default.
    #[must_use]
    pub fn decimal_separator(mut self, separator: char) -> Self {
        if !separator.is_ascii_digit() && separator != '+' && separator != '-' {
            self.decimal = separator;
        }
        self
    }

    /// Accept and emit one grouping separator, such as `,` or a narrow space.
    #[must_use]
    pub fn group_separator(mut self, separator: Option<char>) -> Self {
        self.group = separator.filter(|separator| {
            !separator.is_ascii_digit() && *separator != '+' && *separator != '-'
        });
        self
    }

    /// Accept a leading `+` or `-`. The default is `true`.
    #[must_use]
    pub const fn sign(mut self, sign: bool) -> Self {
        self.sign = sign;
        self
    }

    /// Accept scientific notation such as `1.5e3`. The default is `false`.
    #[must_use]
    pub const fn exponent(mut self, exponent: bool) -> Self {
        self.exponent = exponent;
        self
    }

    /// Format committed values with exactly this many fractional digits.
    #[must_use]
    pub fn precision(mut self, precision: u8) -> Self {
        self.precision = Some(precision.min(MAX_NUMBER_FIELD_PRECISION));
        self
    }

    pub const fn decimal(&self) -> char {
        self.decimal
    }

    pub const fn group(&self) -> Option<char> {
        self.group
    }

    pub const fn allows_sign(&self) -> bool {
        self.sign
    }

    pub const fn allows_exponent(&self) -> bool {
        self.exponent
    }

    pub const fn precision_digits(&self) -> Option<u8> {
        self.precision
    }

    /// Parse one string into a number using these rules.
    ///
    /// Only ASCII digits are accepted. Whitespace around the value is ignored, group separators
    /// are removed, and an empty or malformed string returns `None`.
    pub fn parse(&self, text: &str) -> Option<f64> {
        let trimmed = text.trim();
        if trimmed.is_empty() || trimmed.len() > MAX_NUMBER_FIELD_TEXT_BYTES {
            return None;
        }
        let mut normalized = String::with_capacity(trimmed.len());
        let mut digits = 0_usize;
        let mut seen_decimal = false;
        let mut seen_exponent = false;
        let mut expect_sign = true;
        for character in trimmed.chars() {
            if Some(character) == self.group {
                if !seen_decimal && !seen_exponent && digits > 0 {
                    continue;
                }
                return None;
            }
            if character == self.decimal {
                if seen_decimal || seen_exponent {
                    return None;
                }
                seen_decimal = true;
                normalized.push('.');
                expect_sign = false;
                continue;
            }
            match character {
                '+' | '-' => {
                    if !expect_sign || !self.sign {
                        return None;
                    }
                    normalized.push(character);
                    expect_sign = false;
                }
                'e' | 'E' => {
                    if !self.exponent || seen_exponent || digits == 0 {
                        return None;
                    }
                    seen_exponent = true;
                    normalized.push('e');
                    expect_sign = true;
                }
                digit if digit.is_ascii_digit() => {
                    digits += 1;
                    normalized.push(digit);
                    expect_sign = false;
                }
                _ => return None,
            }
        }
        if digits == 0 {
            return None;
        }
        normalized
            .parse::<f64>()
            .ok()
            .filter(|value| value.is_finite())
    }

    /// Render one number using these rules.
    pub fn format(&self, value: f64) -> Arc<str> {
        if !value.is_finite() {
            return Arc::from("");
        }
        let rendered = match self.precision {
            Some(precision) => format!("{value:.*}", usize::from(precision)),
            None => format!("{value}"),
        };
        let (sign, digits) = match rendered.strip_prefix('-') {
            Some(rest) => ("-", rest),
            None => ("", rendered.as_str()),
        };
        let (integer, fraction) = match digits.split_once('.') {
            Some((integer, fraction)) => (integer, Some(fraction)),
            None => (digits, None),
        };
        let mut output = String::with_capacity(rendered.len() + 8);
        output.push_str(sign);
        match self.group {
            Some(separator)
                if integer.len() > 3 && integer.bytes().all(|byte| byte.is_ascii_digit()) =>
            {
                let lead = integer.len() % 3;
                if lead > 0 {
                    output.push_str(&integer[..lead]);
                }
                let mut index = lead;
                while index < integer.len() {
                    if index > 0 {
                        output.push(separator);
                    }
                    output.push_str(&integer[index..index + 3]);
                    index += 3;
                }
            }
            _ => output.push_str(integer),
        }
        if let Some(fraction) = fraction {
            output.push(self.decimal);
            output.push_str(fraction);
        }
        Arc::from(output)
    }
}

/// Which stepper is currently held.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum RepeatDirection {
    Increment,
    Decrement,
}

/// Controlled editing text, committed value, and bounded stepping state for one number field.
///
/// The application owns the input's appearance, the stepper buttons, and the repeat task it wires
/// to [`Self::repeat_deadline`]. QuickGUI owns parsing, clamping, formatting, and the exact repeat
/// schedule. Nothing here is a timer: the state only reports the next deadline, so a released
/// stepper leaves the window with no idle source at all.
#[derive(Clone, Debug, PartialEq)]
pub struct NumberFieldState {
    text: Arc<str>,
    value: Option<f64>,
    committed: Option<f64>,
    minimum: f64,
    maximum: f64,
    step: f64,
    format: NumberFieldFormat,
    disabled: bool,
    repeat: Option<(RepeatDirection, Instant)>,
}

impl NumberFieldState {
    /// Create a field showing one formatted value.
    pub fn new(value: f64) -> Self {
        let format = NumberFieldFormat::default();
        let value = value.is_finite().then_some(value);
        Self {
            text: value.map_or_else(|| Arc::from(""), |value| format.format(value)),
            value,
            committed: value,
            minimum: f64::NEG_INFINITY,
            maximum: f64::INFINITY,
            step: 1.0,
            format,
            disabled: false,
            repeat: None,
        }
    }

    /// Create an empty field.
    pub fn empty() -> Self {
        Self {
            text: Arc::from(""),
            value: None,
            committed: None,
            ..Self::new(0.0)
        }
    }

    /// Constrain committed values. An inverted range is swapped.
    #[must_use]
    pub fn range(mut self, minimum: f64, maximum: f64) -> Self {
        let (minimum, maximum) = if minimum.is_nan() || maximum.is_nan() {
            (f64::NEG_INFINITY, f64::INFINITY)
        } else if maximum < minimum {
            (maximum, minimum)
        } else {
            (minimum, maximum)
        };
        self.minimum = minimum;
        self.maximum = maximum;
        self
    }

    /// Replace the amount one arrow key, wheel notch, or stepper press moves.
    #[must_use]
    pub fn step(mut self, step: f64) -> Self {
        if step.is_finite() && step > 0.0 {
            self.step = step;
        }
        self
    }

    /// Replace the parsing and formatting rules, reformatting any committed value.
    #[must_use]
    pub fn format(mut self, format: NumberFieldFormat) -> Self {
        self.format = format;
        if let Some(value) = self.value {
            self.text = self.format.format(value);
        }
        self
    }

    /// Format committed values with exactly this many fractional digits.
    #[must_use]
    pub fn precision(self, precision: u8) -> Self {
        let format = self.format.precision(precision);
        self.format(format)
    }

    #[must_use]
    pub const fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    pub fn text(&self) -> &Arc<str> {
        &self.text
    }

    /// The value the current text parses to, or `None` while the text is empty or malformed.
    pub const fn value(&self) -> Option<f64> {
        self.value
    }

    /// The last value committed by [`Self::commit`] or a step.
    ///
    /// Unparseable text is restored to this value on commit, so a half-typed entry cannot lose
    /// the user's previous number.
    pub const fn committed_value(&self) -> Option<f64> {
        self.committed
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

    pub const fn format_rules(&self) -> NumberFieldFormat {
        self.format
    }

    pub const fn is_disabled(&self) -> bool {
        self.disabled
    }

    /// Whether the current text parses to a value inside the field's range.
    ///
    /// An empty field is valid; use [`crate::Field`] for a required-value contract.
    pub fn is_valid(&self) -> bool {
        if self.text.trim().is_empty() {
            return true;
        }
        self.format
            .parse(&self.text)
            .is_some_and(|value| value >= self.minimum && value <= self.maximum)
    }

    /// Replace the editing text from a controlled input listener, returning whether it changed.
    ///
    /// Text is neither clamped nor reformatted while the user types; only [`Self::commit`] does
    /// that. Input longer than [`MAX_NUMBER_FIELD_TEXT_BYTES`] is truncated on a character
    /// boundary rather than retained.
    pub fn set_text(&mut self, text: impl Into<Arc<str>>) -> bool {
        if self.disabled {
            return false;
        }
        let text = bounded_text(text.into());
        if self.text == text {
            return false;
        }
        self.text = text;
        self.value = self.format.parse(&self.text);
        true
    }

    /// Clamp and reformat the current text, returning whether anything changed.
    ///
    /// Call this on Return and on blur. Unparseable text restores the last committed value; an
    /// empty field stays empty.
    pub fn commit(&mut self) -> bool {
        if self.disabled {
            return false;
        }
        if self.text.trim().is_empty() {
            let changed = self.value.is_some() || self.committed.is_some() || !self.text.is_empty();
            self.value = None;
            self.committed = None;
            self.text = Arc::from("");
            return changed;
        }
        let parsed = self.format.parse(&self.text).or(self.committed);
        let Some(value) = parsed else {
            let changed = !self.text.is_empty();
            self.text = Arc::from("");
            self.value = None;
            return changed;
        };
        let value = value.clamp(self.minimum, self.maximum);
        let text = self.format.format(value);
        let changed = self.value != Some(value) || self.text != text;
        self.value = Some(value);
        self.committed = Some(value);
        self.text = text;
        changed
    }

    /// Move the value by `steps` steps, clamping and reformatting, returning whether it changed.
    ///
    /// An empty field starts from zero clamped into range, matching desktop spin buttons.
    pub fn step_by(&mut self, steps: f64) -> bool {
        if self.disabled || !steps.is_finite() {
            return false;
        }
        let current = self
            .format
            .parse(&self.text)
            .or(self.committed)
            .unwrap_or_else(|| 0.0_f64.clamp(self.minimum, self.maximum));
        let value = (current + self.step * steps).clamp(self.minimum, self.maximum);
        let text = self.format.format(value);
        let changed = self.value != Some(value) || self.text != text;
        self.value = Some(value);
        self.committed = Some(value);
        self.text = text;
        changed
    }

    /// Step once toward the maximum.
    pub fn increment(&mut self) -> bool {
        self.step_by(1.0)
    }

    /// Step once toward the minimum.
    pub fn decrement(&mut self) -> bool {
        self.step_by(-1.0)
    }

    /// Step from a scroll wheel, which desktop platforms only apply to a focused field.
    ///
    /// `delta` is logical pixels or lines; only its sign is used, so trackpad inertia cannot run
    /// the value away.
    pub fn wheel(&mut self, delta: f32, focused: bool) -> bool {
        if !focused || !delta.is_finite() || delta == 0.0 {
            return false;
        }
        self.step_by(if delta > 0.0 { 1.0 } else { -1.0 })
    }

    /// Press and hold one stepper: steps once and arms the first repeat deadline.
    pub fn press_step(&mut self, forward: bool, now: Instant) -> bool {
        if self.disabled {
            return false;
        }
        self.repeat = Some((
            if forward {
                RepeatDirection::Increment
            } else {
                RepeatDirection::Decrement
            },
            now + NUMBER_FIELD_REPEAT_DELAY,
        ));
        self.step_by(if forward { 1.0 } else { -1.0 })
    }

    /// The exact instant at which the held stepper next repeats.
    ///
    /// This is `None` while no stepper is held, so a settled field schedules no wakeup at all.
    /// Sleep until this instant with [`crate::AsyncViewContext::sleep_until`] and then call
    /// [`Self::repeat`].
    pub const fn repeat_deadline(&self) -> Option<Instant> {
        match self.repeat {
            Some((_, deadline)) => Some(deadline),
            None => None,
        }
    }

    /// Apply every repeat that is due at `now` and re-arm the next one.
    ///
    /// Returns whether the value changed. A single late wakeup applies at most one step per
    /// elapsed interval, so a delayed event loop cannot make the value jump unpredictably.
    pub fn repeat(&mut self, now: Instant) -> bool {
        let Some((direction, deadline)) = self.repeat else {
            return false;
        };
        if now < deadline {
            return false;
        }
        let elapsed = now.duration_since(deadline);
        let extra = elapsed.as_nanos() / NUMBER_FIELD_REPEAT_INTERVAL.as_nanos().max(1);
        let steps = 1 + u32::try_from(extra).unwrap_or(u32::MAX);
        let next = deadline + NUMBER_FIELD_REPEAT_INTERVAL * steps;
        self.repeat = Some((direction, next));
        let amount = f64::from(steps)
            * match direction {
                RepeatDirection::Increment => 1.0,
                RepeatDirection::Decrement => -1.0,
            };
        self.step_by(amount)
    }

    /// Release the held stepper, returning whether a repeat was pending.
    ///
    /// After this the field owns no deadline, task, timer, observer, or idle scheduler source.
    pub fn release_step(&mut self) -> bool {
        self.repeat.take().is_some()
    }

    /// Whether a stepper is currently held.
    pub const fn is_stepping(&self) -> bool {
        self.repeat.is_some()
    }
}

fn bounded_text(text: Arc<str>) -> Arc<str> {
    if text.len() <= MAX_NUMBER_FIELD_TEXT_BYTES {
        return text;
    }
    let mut end = MAX_NUMBER_FIELD_TEXT_BYTES;
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    Arc::from(&text[..end])
}

/// A controlled, unstyled number-field descriptor.
///
/// The application owns the input's appearance, the stepper glyphs, and the layout. QuickGUI
/// supplies stable identities, the SpinButton role with numeric value, bounds and step, invalid
/// state, and the ordinary controlled text input underneath.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[must_use = "a NumberField descriptor has no effect until its parts are mounted"]
pub struct NumberField {
    root_id: ElementId,
}

impl NumberField {
    pub fn new(root_id: impl Into<ElementId>) -> Self {
        Self {
            root_id: root_id.into(),
        }
    }

    pub const fn root_id(self) -> ElementId {
        self.root_id
    }

    pub fn input_id(self) -> ElementId {
        derived_number_field_id(self.root_id, NUMBER_FIELD_INPUT_ID_TAG)
    }

    pub fn increment_id(self) -> ElementId {
        derived_number_field_id(self.root_id, NUMBER_FIELD_INCREMENT_ID_TAG)
    }

    pub fn decrement_id(self) -> ElementId {
        derived_number_field_id(self.root_id, NUMBER_FIELD_DECREMENT_ID_TAG)
    }

    /// Decorate an application-owned root without adding layout or appearance.
    pub fn root_part(self, root: Element) -> Element {
        root.id(self.root_id)
            .accessibility_role(AccessibilityRole::Group)
    }

    /// Decorate the application-owned controlled input.
    ///
    /// Pass a [`crate::text_input`] built from [`NumberFieldState::text`] with the caller's own
    /// [`crate::Element::on_input`] listener. QuickGUI adds the spin-button semantics: the
    /// committed numeric value, the field's bounds and step, and invalid state.
    pub fn input_part(self, state: &NumberFieldState, input: Element) -> Element {
        let mut range = AccessibilityValueRange {
            value: state.value,
            min: state.minimum.is_finite().then_some(state.minimum),
            max: state.maximum.is_finite().then_some(state.maximum),
            step: None,
        };
        range = range.step(state.step);
        input
            .id(self.input_id())
            .accessibility_role(AccessibilityRole::SpinButton)
            .accessibility_value_range(range)
            .invalid(!state.is_valid())
            .disabled(state.disabled)
            .app_region_no_drag()
    }

    /// Decorate the application-owned increment button.
    ///
    /// Steppers stay out of the Tab sequence because the input already answers arrow keys, which
    /// matches native desktop spin buttons.
    pub fn increment_part(self, state: &NumberFieldState, increment: Element) -> Element {
        stepper(increment, self.increment_id(), state.disabled)
    }

    /// Decorate the application-owned decrement button.
    pub fn decrement_part(self, state: &NumberFieldState, decrement: Element) -> Element {
        stepper(decrement, self.decrement_id(), state.disabled)
    }
}

fn stepper(element: Element, id: ElementId, disabled: bool) -> Element {
    element
        .id(id)
        .accessibility_role(AccessibilityRole::Button)
        .clickable()
        .tab_index(-1)
        .cursor_default()
        .app_region_no_drag()
        .user_select_none()
        .disabled(disabled)
}

/// Create an unstyled controlled number-field input.
///
/// This shorthand is equivalent to
/// `NumberField::new(id).input_part(state, text_input(state.text().clone()))`.
pub fn number_field(id: impl Into<ElementId>, state: &NumberFieldState) -> Element {
    NumberField::new(id).input_part(state, text_input(state.text().clone()))
}

/// Create an unstyled number-field root.
pub fn number_field_root(id: impl Into<ElementId>) -> Element {
    NumberField::new(id).root_part(div())
}

fn derived_number_field_id(scope: ElementId, tag: u64) -> ElementId {
    let mut hash = scope.as_u64().rotate_left(11) ^ tag;
    hash ^= hash >> 30;
    hash = hash.wrapping_mul(0xbf58_476d_1ce4_e5b9);
    hash ^= hash >> 27;
    hash = hash.wrapping_mul(0x94d0_49bb_1331_11eb);
    hash ^= hash >> 31;
    if hash == 0 || hash == u64::MAX || hash == scope.as_u64() {
        hash ^= tag.rotate_left(17);
    }
    ElementId::new(hash)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Color, IntoElement, Key, TestAppContext, View, ViewContext, button, text};

    #[test]
    fn parsing_and_formatting_follow_caller_supplied_separators() {
        let plain = NumberFieldFormat::default();
        assert_eq!(plain.parse("42"), Some(42.0));
        assert_eq!(plain.parse("  -3.5 "), Some(-3.5));
        assert_eq!(plain.parse("+7"), Some(7.0));
        assert_eq!(plain.parse(""), None);
        assert_eq!(plain.parse("."), None);
        assert_eq!(plain.parse("1.2.3"), None);
        assert_eq!(plain.parse("12a"), None);
        assert_eq!(plain.parse("1e3"), None);
        assert_eq!(plain.parse("１２３"), None);
        assert_eq!(
            plain.parse(&"9".repeat(MAX_NUMBER_FIELD_TEXT_BYTES + 1)),
            None
        );
        assert_eq!(plain.format(42.0).as_ref(), "42");
        assert_eq!(plain.format(f64::NAN).as_ref(), "");

        let unsigned = NumberFieldFormat::default().sign(false);
        assert_eq!(unsigned.parse("-3"), None);
        assert_eq!(unsigned.parse("3"), Some(3.0));

        let scientific = NumberFieldFormat::default().exponent(true);
        assert_eq!(scientific.parse("1.5e3"), Some(1_500.0));
        assert_eq!(scientific.parse("1.5e-3"), Some(0.0015));
        assert_eq!(scientific.parse("e3"), None);

        let european = NumberFieldFormat::default()
            .decimal_separator(',')
            .group_separator(Some('.'));
        assert_eq!(european.parse("1.234,56"), Some(1_234.56));
        assert_eq!(european.parse("1234,56"), Some(1_234.56));
        assert_eq!(european.parse(".5"), None);
        assert_eq!(european.format(1_234.5).as_ref(), "1.234,5");
        assert_eq!(european.format(-9_876_543.0).as_ref(), "-9.876.543");
        assert_eq!(european.format(12.0).as_ref(), "12");

        // A digit or sign is never accepted as a separator.
        assert_eq!(
            NumberFieldFormat::default()
                .decimal_separator('5')
                .decimal(),
            '.'
        );
        assert_eq!(
            NumberFieldFormat::default()
                .group_separator(Some('-'))
                .group(),
            None
        );

        let fixed = NumberFieldFormat::default().precision(2);
        assert_eq!(fixed.format(4.56789).as_ref(), "4.57");
        assert_eq!(fixed.format(2.0).as_ref(), "2.00");
        assert_eq!(
            NumberFieldFormat::default()
                .precision(200)
                .precision_digits(),
            Some(MAX_NUMBER_FIELD_PRECISION)
        );
    }

    #[test]
    fn state_edits_commit_clamp_and_step_within_bounds() {
        let mut state = NumberFieldState::new(5.0).range(0.0, 10.0).step(2.0);
        assert_eq!(state.text().as_ref(), "5");
        assert_eq!(state.value(), Some(5.0));
        assert!(state.is_valid());

        assert!(state.set_text("7"));
        assert!(!state.set_text("7"));
        assert_eq!(state.value(), Some(7.0));
        assert!(state.is_valid());
        assert!(!state.commit());

        // Typing keeps arbitrary text; only committing clamps and reformats it.
        assert!(state.set_text("400"));
        assert_eq!(state.text().as_ref(), "400");
        assert!(!state.is_valid());
        assert!(state.commit());
        assert_eq!(state.text().as_ref(), "10");
        assert_eq!(state.value(), Some(10.0));

        assert!(state.set_text("abc"));
        assert_eq!(state.value(), None);
        assert_eq!(state.committed_value(), Some(10.0));
        assert!(!state.is_valid());
        assert!(state.commit());
        assert_eq!(state.value(), Some(10.0));
        assert_eq!(state.text().as_ref(), "10");

        assert!(state.set_text(""));
        assert!(state.is_valid());
        assert!(state.commit());
        assert_eq!(state.value(), None);
        assert_eq!(state.text().as_ref(), "");

        // Stepping an empty field starts from zero clamped into range.
        assert!(state.step_by(1.0));
        assert_eq!(state.value(), Some(2.0));
        assert!(state.increment());
        assert_eq!(state.value(), Some(4.0));
        assert!(state.decrement());
        assert_eq!(state.value(), Some(2.0));
        assert!(state.step_by(-100.0));
        assert_eq!(state.value(), Some(0.0));
        assert!(!state.step_by(-1.0));
        assert!(!state.step_by(f64::NAN));

        assert!(state.wheel(1.0, true));
        assert_eq!(state.value(), Some(2.0));
        assert!(!state.wheel(1.0, false));
        assert!(!state.wheel(0.0, true));
        assert!(state.wheel(-40.0, true));
        assert_eq!(state.value(), Some(0.0));

        let long = "1".repeat(MAX_NUMBER_FIELD_TEXT_BYTES + 12);
        assert!(state.set_text(long));
        assert_eq!(state.text().len(), MAX_NUMBER_FIELD_TEXT_BYTES);

        let mut disabled = NumberFieldState::new(1.0).disabled(true);
        assert!(!disabled.set_text("9"));
        assert!(!disabled.increment());
        assert!(!disabled.commit());

        let inverted = NumberFieldState::new(0.0).range(10.0, -10.0);
        assert_eq!((inverted.minimum(), inverted.maximum()), (-10.0, 10.0));
        let unbounded = NumberFieldState::new(0.0).range(f64::NAN, 4.0);
        assert_eq!(unbounded.minimum(), f64::NEG_INFINITY);
        assert!(NumberFieldState::empty().value().is_none());
        assert!(NumberFieldState::new(f64::NAN).value().is_none());
        assert_eq!(
            NumberFieldState::new(1.5).precision(2).text().as_ref(),
            "1.50"
        );
        assert_eq!(NumberFieldState::new(1.0).step(-4.0).step_value(), 1.0);
    }

    #[test]
    fn press_and_hold_uses_exact_deadlines_and_stops_on_release() {
        let start = Instant::now();
        let mut state = NumberFieldState::new(0.0).range(0.0, 1_000.0).step(1.0);
        assert_eq!(state.repeat_deadline(), None);
        assert!(!state.repeat(start));

        assert!(state.press_step(true, start));
        assert_eq!(state.value(), Some(1.0));
        assert_eq!(
            state.repeat_deadline(),
            Some(start + NUMBER_FIELD_REPEAT_DELAY)
        );
        assert!(state.is_stepping());

        // Nothing happens before the exact deadline.
        assert!(!state.repeat(start + NUMBER_FIELD_REPEAT_DELAY - Duration::from_millis(1)));
        assert_eq!(state.value(), Some(1.0));

        let first = start + NUMBER_FIELD_REPEAT_DELAY;
        assert!(state.repeat(first));
        assert_eq!(state.value(), Some(2.0));
        assert_eq!(
            state.repeat_deadline(),
            Some(first + NUMBER_FIELD_REPEAT_INTERVAL)
        );

        // One late wakeup applies exactly the steps that came due, not an unbounded burst.
        let late = first + NUMBER_FIELD_REPEAT_INTERVAL * 4;
        assert!(state.repeat(late));
        assert_eq!(state.value(), Some(6.0));
        assert_eq!(
            state.repeat_deadline(),
            Some(first + NUMBER_FIELD_REPEAT_INTERVAL * 5)
        );

        assert!(state.release_step());
        assert!(!state.release_step());
        assert_eq!(state.repeat_deadline(), None);
        assert!(!state.is_stepping());
        assert!(!state.repeat(late + Duration::from_secs(10)));
        assert_eq!(state.value(), Some(6.0));

        let mut down = NumberFieldState::new(5.0).range(0.0, 10.0);
        assert!(down.press_step(false, start));
        assert_eq!(down.value(), Some(4.0));
        assert!(down.repeat(start + NUMBER_FIELD_REPEAT_DELAY));
        assert_eq!(down.value(), Some(3.0));

        let mut disabled = NumberFieldState::new(1.0).disabled(true);
        assert!(!disabled.press_step(true, start));
        assert_eq!(disabled.repeat_deadline(), None);
    }

    #[test]
    fn parts_add_exact_semantics_without_appearance() {
        let state = NumberFieldState::new(5.0).range(0.0, 10.0).step(2.0);
        let field = NumberField::new("quantity");
        let root = field.root_part(div().bg(Color::rgb8(1, 2, 3)));
        assert_eq!(root.explicit_id, Some("quantity".into()));
        assert_eq!(root.accessibility.role, AccessibilityRole::Group);
        assert_eq!(root.visual.background, Some(Color::rgb8(1, 2, 3)));

        let input = field.input_part(&state, text_input(state.text().clone()).w(80.0));
        assert_eq!(input.explicit_id, Some(field.input_id()));
        assert_eq!(input.accessibility.role, AccessibilityRole::SpinButton);
        assert_eq!(
            input.accessibility.value_range.as_deref(),
            Some(&AccessibilityValueRange::new(5.0, 0.0, 10.0).step(2.0))
        );
        assert!(!input.accessibility.invalid);

        let mut invalid = state.clone();
        assert!(invalid.set_text("99"));
        let invalid_input = field.input_part(&invalid, text_input(invalid.text().clone()));
        assert!(invalid_input.accessibility.invalid);

        let unbounded = NumberFieldState::new(3.0);
        let unbounded_input = field.input_part(&unbounded, text_input(unbounded.text().clone()));
        let range = unbounded_input
            .accessibility
            .value_range
            .as_deref()
            .expect("value range");
        assert_eq!(range.value, Some(3.0));
        assert_eq!(range.min, None);
        assert_eq!(range.max, None);

        let increment = field.increment_part(&state, div().child("+"));
        assert_eq!(increment.explicit_id, Some(field.increment_id()));
        assert_eq!(increment.accessibility.role, AccessibilityRole::Button);
        assert!(increment.clickable);
        assert_eq!(increment.tab_index, -1);
        let decrement = field.decrement_part(&state, div().child("-"));
        assert_eq!(decrement.explicit_id, Some(field.decrement_id()));

        let disabled = NumberFieldState::new(1.0).disabled(true);
        assert!(
            field
                .increment_part(&disabled, div())
                .accessibility
                .disabled
        );

        let ids = [
            field.root_id(),
            field.input_id(),
            field.increment_id(),
            field.decrement_id(),
        ];
        for (index, id) in ids.iter().enumerate() {
            assert!(!ids[..index].contains(id));
        }
    }

    struct NumberFieldView {
        quantity: NumberFieldState,
    }

    impl Default for NumberFieldView {
        fn default() -> Self {
            Self {
                quantity: NumberFieldState::new(5.0).range(0.0, 10.0).step(2.0),
            }
        }
    }

    impl View for NumberFieldView {
        fn render(&mut self, cx: &mut ViewContext<'_, Self>) -> impl IntoElement {
            let field = NumberField::new("quantity");
            let edit = cx.input_listener(field.input_id(), |view: &mut Self, value, cx| {
                if view.quantity.set_text(value) {
                    cx.invalidate();
                }
            });
            // The input is marked invalid while its text is out of range, and QuickGUI blocks
            // Return submission for an invalid control, so commit from an ordinary key listener.
            let commit = cx.key_down_listener(field.input_id(), |view: &mut Self, event, cx| {
                if event.key == Key::Enter && view.quantity.commit() {
                    cx.invalidate();
                }
            });
            let up = cx.listener(field.increment_id(), |view: &mut Self, cx| {
                if view.quantity.increment() {
                    cx.invalidate();
                }
            });
            let down = cx.listener(field.decrement_id(), |view: &mut Self, cx| {
                if view.quantity.decrement() {
                    cx.invalidate();
                }
            });

            field.root_part(
                div()
                    .child(
                        field.input_part(
                            &self.quantity,
                            text_input(self.quantity.text().clone())
                                .w(120.0)
                                .on_input(edit)
                                .on_key_down(commit),
                        ),
                    )
                    .child(
                        field
                            .increment_part(&self.quantity, button().child(text("+")).on_click(up)),
                    )
                    .child(
                        field.decrement_part(
                            &self.quantity,
                            button().child(text("-")).on_click(down),
                        ),
                    ),
            )
        }
    }

    #[test]
    fn controlled_editing_and_accessibility_paths_stay_deterministic() {
        let (mut cx, view) = TestAppContext::new(NumberFieldView::default()).unwrap();
        let window = view.window_handle();
        let field = NumberField::new("quantity");

        cx.click(window, field.increment_id()).unwrap();
        assert_eq!(
            cx.read(view, |view| view.quantity.value()).unwrap(),
            Some(7.0)
        );
        cx.click(window, field.decrement_id()).unwrap();
        cx.click(window, field.decrement_id()).unwrap();
        assert_eq!(
            cx.read(view, |view| view.quantity.value()).unwrap(),
            Some(3.0)
        );

        cx.focus(window, field.input_id()).unwrap();
        cx.simulate_input(window, "9").unwrap();
        assert_eq!(
            cx.read(view, |view| view.quantity.text().to_string())
                .unwrap(),
            "39"
        );
        cx.simulate_keystrokes(window, "enter").unwrap();
        assert_eq!(
            cx.read(view, |view| view.quantity.value()).unwrap(),
            Some(10.0)
        );

        let update = cx.accessibility_update(window).unwrap();
        let node = |id: ElementId| {
            update
                .nodes
                .iter()
                .find_map(|(node_id, node)| (node_id.0 == id.as_u64()).then_some(node))
                .expect("number field accessibility node")
        };
        let input = node(field.input_id());
        assert_eq!(input.role(), accesskit::Role::SpinButton);
        assert_eq!(input.numeric_value(), Some(10.0));
        assert_eq!(input.min_numeric_value(), Some(0.0));
        assert_eq!(input.max_numeric_value(), Some(10.0));
        assert_eq!(input.numeric_value_step(), Some(2.0));
        assert_eq!(node(field.increment_id()).role(), accesskit::Role::Button);

        let renders = cx.render_count(window).unwrap();
        cx.run_until_idle().unwrap();
        assert_eq!(cx.render_count(window).unwrap(), renders);
    }

    #[test]
    fn shorthands_are_semantic_unstyled_parts() {
        let state = NumberFieldState::new(2.0);
        let input = number_field("count", &state);
        assert_eq!(input.accessibility.role, AccessibilityRole::SpinButton);
        let root = number_field_root("count");
        assert_eq!(root.accessibility.role, AccessibilityRole::Group);
        assert!(root.children.is_empty());
    }
}
