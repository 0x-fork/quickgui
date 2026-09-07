# Date and time fields

[Documentation index](README.md) · [Unstyled component roadmap](component-roadmap.md)

QuickGUI provides controlled, unstyled segmented date and time editors. The application owns every
separator, segment width, color, placeholder appearance, and focus ring. QuickGUI owns the calendar
contract — leap years, month lengths, wrapping steps, typed-digit entry with automatic advance,
bounded validation — plus which segment is being edited and the native spin-button semantics of
each one.

The behavior follows the WAI-ARIA [Spinbutton](https://www.w3.org/WAI/ARIA/apg/patterns/spinbutton/)
pattern and the current [Base UI](https://base-ui.com/react/overview/quick-start) field contracts.

Run the caller-styled gallery with:

```console
cargo run --release --example date_fields
```

## Civil values

`CivilDate { year, month, day }` and `CivilTime { hour, minute, second }` are plain copyable value
structs with no time zone, clock, locale, or crate dependency. They order chronologically, so a
range check is an ordinary comparison.

```rust
use quickgui::{CivilDate, CivilTime};

let day = CivilDate::new(2024, 2, 29).expect("2024 is a leap year");
assert!(CivilDate::new(2023, 2, 29).is_none());
assert_eq!(CivilDate::days_in_month(2023, 2), 28);
assert_eq!(day.weekday(), 3); // Monday is 0
let time = CivilTime::new(13, 45, 0).expect("a real time");
assert_eq!(time.hour12(), 1);
```

`CivilDate::new` validates the proleptic Gregorian calendar, including the 100- and 400-year leap
rules, and rejects years outside `MIN_CIVIL_YEAR..=MAX_CIVIL_YEAR`. `clamped()` repairs an
out-of-range value instead of rejecting it, shortening the day for short months. `epoch_day` and
`weekday` are exact integer arithmetic with no floating point and no lookup table.

## Date field

`DateFieldState` retains one segment value per part, the segment being edited, the declared order
and bounds, and one pending typed-digit buffer.

```rust
use quickgui::{CivilDate, DateField, DateFieldOrder, DateFieldState, div, text};

struct Task {
    due: DateFieldState, // DateFieldState::new().order(DateFieldOrder::DayMonthYear)
}

let field = DateField::new("due");
let mut row = field
    .root_part(&self.due, div().flex_row().gap_1())
    .accessibility_label("Due date");
for segment in self.due.segment_order().segments() {
    let part = field.segment(segment);
    let element = part.segment_part(
        &self.due,
        div().child(text(self.due.segment_text(segment))),
    );
    row = row.child(part.key_part(cx, element, |view: &mut Task| &mut view.due));
}
```

`root_part` and `segment_part` are pure decorators; they add identity, roles, relationships, and
interaction contracts and never add layout or paint. `key_part` attaches the typed keyboard actions
and digit entry to one segment.

`segment_text` returns the zero-padded digits, or that segment's placeholder while it is empty, and
`is_filled` reports which of the two the caller is painting. Replace a placeholder with
`DateFieldState::placeholder(segment, text)`; text longer than
`MAX_DATE_FIELD_PLACEHOLDER_BYTES` keeps the default.

### Keyboard

Install `date_field_key_bindings()` once. Up and Down step the edited segment by one and wrap at its
bounds, Left and Right move between segments, Home and End jump to a segment's smallest and largest
legal value, and Backspace or Delete clears exactly one segment. Tab and Shift-Tab already move
between segments because each segment is an ordinary focusable control, and the final segment
reports that it cannot advance so Tab leaves the field instead of trapping focus.

Typing digits fills the edited segment and advances as soon as no further digit could fit: a month
of `4`, a day of `9`, or a complete four-digit year all move on immediately. The day segment accepts
two typed digits in every month and the retained value is then shortened to that month's real
length, so `3` `1` in February keeps `28`. Arrow steps and the projected spin-button range are
month-aware: February in a leap year offers `29`.

### Bounds and validity

`minimum` and `maximum` are reported through validity; they never rewrite what the user typed, which
is what keeps a half-typed date editable. A field is valid while it is completely empty or holds a
real date inside its declared range; a partly filled date is invalid, matching the desktop and web
behavior of a segmented control. The root part projects that state, so
`.invalid_style(...)` paints it.

### Accessibility

The root is a group carrying the field's accessible name, its disabled state, its validity, and an
active-descendant relationship to the segment being edited. Every segment is a focusable spin button
with its current numeric value and its live minimum and maximum, or bounds without a value while it
is empty. QuickGUI supplies a default English name per segment; call `.accessibility_label(...)`
after `segment_part` to replace it with localized product text.

## Time field

`TimeFieldState` always retains a 24-hour value. A 12-hour field projects the same value through an
hour segment of `1..=12` plus an AM/PM segment, so switching presentation never rewrites the
application's data.

```rust
use quickgui::{CivilTime, TimeField, TimeFieldState};

let start = TimeFieldState::from_time(CivilTime::new(9, 30, 0).unwrap())
    .hour12(true)
    .seconds(true);
assert_eq!(start.segment_text(TimeSegment::Hour), "09");
assert_eq!(start.segment_text(TimeSegment::Period), "AM");
```

`segments()` is a copyable iterator over exactly the segments the field presents, so a caller loop
stays bounded by the state rather than by its own arithmetic. `seconds(false)` (the default) drops
the second segment and treats the value's seconds as zero.

Install `time_field_key_bindings()` once; it binds the same typed actions in the time field's own key
context, so one application can install both. Up and Down toggle the AM/PM segment, `a` and `p` set
it directly, and clearing it clears the hour it describes, because a 12-hour clock cannot retain an
hour without a half.

## Month grid

`CalendarState` is the month grid that pairs with a date field. It retains the month it shows, the
day that holds the grid's single Tab stop, the selected day, and optional bounds — eight small
fields, no allocation.

```rust
use quickgui::{Calendar, CalendarState, CalendarWeekday, CivilDate, div, text};

let grid = Calendar::new("month");
let mut month = grid.grid_part(self.month, div().flex_col());
for index in 0..self.month.week_count() {
    let days = self.month.week(index).expect("a mounted week row");
    let mut row = grid.week_part(index, div().flex_row());
    for day in days {
        let cell = grid.day_part(self.month, day, div().child(text(day.day.to_string())));
        row = row.child(grid.key_part(cx, day, cell, |view: &mut Task| &mut view.month));
    }
    month = month.child(row);
}
```

`week(index)` returns seven consecutive days including the leading and trailing cells that belong to
the adjacent months; `is_in_displayed_month` tells the caller which is which. `week_count()` is
never more than `MAX_CALENDAR_WEEKS`, so a month grid's mounted rows are a constant rather than a
function of application data. `CalendarWeekday` declares which weekday a row starts on, because
QuickGUI has no locale database.

Install `calendar_key_bindings()` once. Left and Right move one day, Up and Down one week, Home and
End reach the edges of the focused day's own week row, Page Up and Page Down change months, Shift
with either changes years, and Return or Space selects. Focus follows a step into an adjacent month
and the grid shows that month; `minimum` and `maximum` refuse the *selection* rather than the
movement, so a user can always see why a day is unavailable.

The grid projects the native grid role with its exact row and column counts and an active-descendant
relationship to the focused day. Each cell is a grid cell carrying its zero-based row and column, its
selected state, its ISO date as the accessible value, and disabled state outside the declared range.
Exactly one day carries the Tab stop; the rest are reached with the arrow keys.

## Bounds

| Constant | Value | Meaning |
| --- | --- | --- |
| `MIN_CIVIL_YEAR` | 1 | Smallest year a date field or civil date accepts. |
| `MAX_CIVIL_YEAR` | 9999 | Largest year, keeping every segment a fixed width. |
| `MAX_DATE_FIELD_PLACEHOLDER_BYTES` | 16 | UTF-8 bytes retained by one segment placeholder. |
| `MAX_CALENDAR_WEEKS` | 6 | Week rows one month grid mounts. |
| `CALENDAR_WEEK_DAYS` | 7 | Days in one week row. |

A date field retains three optional integers, a time field four, plus one small typed-digit buffer
and one bounded placeholder per segment; a calendar retains eight small fields. None of them retains
an item registry, allocation-sized event payload, task, timer, observer, animation, GPU resource, or
idle scheduler source, and none can produce a deadline. State changes rebuild only when the caller's
listener requests invalidation, so a settled window renders zero extra frames.

## Go components

`DateField.Root` / `Segment`, `TimeField.Root` / `Segment`, and `Calendar.Root` / `Week` / `Day`
declare the controlled civil value, the civil bounds, the segment order, the twelve-hour and
seconds policy, and the first weekday. Civil values cross the boundary as ISO strings with no time
zone: `YYYY-MM-DD` for a date or a calendar day, `HH:MM` or `HH:MM:SS` for a time. A string the
core would not accept as a real calendar day or wall-clock time declares no value at all.

The hosted view reaches each declared instance's retained `DateFieldState`, `TimeFieldState`, or
`CalendarState` through a per-instance [`StateAccessor`](view-api.md). Segment arithmetic, digit
entry, leap years, clamping into the declared range, day and week movement, month and year
movement, and the single Tab stop all stay in the core; the value it decided travels back as one
asynchronous `OnValueChange`, and a month grid additionally reports the focused day and the
displayed month. A time-field segment the core does not own — the seconds segment of a field
without `ShowSeconds`, or the period segment of a twenty-four-hour field — contributes no layout,
paint, or accessibility node. See the
[Go components](go.md).
