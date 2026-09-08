# Range and feedback components

[Documentation index](README.md) · [Unstyled component roadmap](component-roadmap.md)

QuickGUI provides controlled, unstyled sliders, number fields, splitters, progress indicators, and
meters. The application owns every track, thumb, fill, stepper glyph, divider, color, radius, and
animation. QuickGUI owns the numeric contract — bounds, step snapping, ordering, clamping — plus
pointer capture, keyboard behavior, and native accessibility semantics.

The behavior follows the current [Base UI](https://base-ui.com/react/overview/quick-start)
component contracts and the WAI-ARIA [Slider](https://www.w3.org/WAI/ARIA/apg/patterns/slider/),
[Slider (Multi-Thumb)](https://www.w3.org/WAI/ARIA/apg/patterns/slider-multithumb/),
[Spinbutton](https://www.w3.org/WAI/ARIA/apg/patterns/spinbutton/), and
[Window Splitter](https://www.w3.org/WAI/ARIA/apg/patterns/windowsplitter/) patterns.

Run the caller-styled gallery with:

```console
cargo run --release --example range_controls
```

## Slider

`SliderState` is a copyable value. It holds the bounds, the step, up to `MAX_SLIDER_THUMBS` values,
the orientation, and the active thumb. Every mutation returns whether anything changed, so the
owning view invalidates only on a real change.

```rust
use quickgui::{Size, Slider, SliderState, div};

struct Mixer {
    volume: SliderState, // SliderState::new(0.0, 100.0, 40.0).step(5.0)
}

const TRACK: Size = Size { width: 260.0, height: 20.0 };

let slider = Slider::new("volume", &self.volume);
let drag = cx.pointer_listener(slider.track_id(), |view: &mut Mixer, event, cx| {
    if view.volume.apply_pointer(event, TRACK) {
        cx.invalidate();
    }
});
let thumb = slider.thumb(0).expect("single thumb");

slider.key_part(
    cx,
    slider
        .root_part(div())
        .accessibility_label("Volume")
        .child(
            slider
                .track_part(div().w(TRACK.width).h(TRACK.height).on_pointer(drag))
                .child(slider.range_part(div()))
                .child(thumb.thumb_part(div())),
        ),
    |view: &mut Mixer| &mut view.volume,
)
```

`root_part`, `track_part`, `range_part`, and `SliderThumb::thumb_part` are pure decorators; they add
identity, roles, and interaction contracts and never add layout or paint. `key_part` attaches the
typed keyboard actions to whichever part owns focus.

### Parts

| Base UI part | QuickGUI decorator | What QuickGUI owns |
| --- | --- | --- |
| Root | `root_part(element)` | the Slider or Group role, orientation, numeric value and bounds, and the label/value relationships |
| Label | `label_part(element)` | the accessible-name target the root points at |
| Value | `value_part(element)` | the accessible-description target the root points at |
| Control | `control_part(element)` | a second, outer identity to attach the pointer capture to |
| Track | `track_part(element)` | the pointer-capture identity, kept out of the accessible name |
| Indicator | `indicator_part(element)` | the filled part of the track; the same decorator as `range_part` |
| Thumb | `SliderThumb::thumb_part(element)` | per-thumb Slider role, neighbour-derived bounds, focus, and key context |

`control_part` is Base UI's separation of the region a press acts on from the track it paints. A
caller that already attached the pointer listener to `track_part` keeps working unchanged.

### Commits, dragging, and formatting

`SliderState::apply_pointer_change` is `apply_pointer` with Base UI's `onValueCommitted` boundary
exposed. It returns `SliderPointerChange { changed, values_changed, committed }`: `values_changed`
is what `apply_pointer` returns, `changed` also covers the dragging flag so a thumb can restyle
mid-drag, and `committed` is true exactly on the event that releases or cancels the capture — the
moment a previewed value becomes final. `SliderState::is_dragging` is Base UI's `data-dragging`, and
`end_drag()` clears it when the application cancels a drag itself.

`SliderThumb::state()` returns a copyable `SliderThumbState` carrying `index` (Base UI's
`data-index`), the value, the fraction, and the active, dragging, and disabled flags.

`Slider::display_value(&format)` and `SliderThumb::value_text(&format)` apply the shared
[`ValueFormat`](#status-and-formatting) — Base UI's Root `format` and Thumb `getAriaValueText`. A
range slider joins its thumbs with an en dash. QuickGUI never renders the result: put it inside
`value_part`, or pass it to `Element::accessibility_value` on the thumb. The Thumb `getAriaLabel`
equivalent is the ordinary `Element::accessibility_label` on the caller-owned thumb.

### Geometry

The application owns the track's layout, so it passes the size it declared to
`SliderState::apply_pointer`. A press picks the nearest thumb, makes it active, and jumps it to the
pointer; captured motion continues to drag that thumb outside the track and outside the window.
`SliderState::fraction` returns each thumb's `0.0..=1.0` position for caller-owned placement, and
`value_at` converts a pointer offset into a snapped value. `SliderThumb::offset(track_length,
thumb_length)` turns that fraction into the leading-edge offset the application lays the thumb out
at, honoring Base UI's `thumbAlignment`: `SliderThumbAlignment::Center` lets the thumb overhang both
ends, `Edge` keeps its box inside the track. Alignment changes nothing about the numeric
contract. A vertical track measures from its top,
where the maximum lives, so vertical sliders behave like their desktop counterparts without the
application inverting anything.

### Keyboard

Install `slider_key_bindings()` once on the application keymap. Left and Down decrement, Right and
Up increment, Shift with any arrow and PageUp/PageDown move one large step, Home and End move to the
bounds. The large step defaults to ten ordinary steps, or one tenth of the range for a continuous
slider; `SliderState::large_step` replaces it.

### Multiple thumbs

`SliderState::range(min, max, &[..])` creates up to `MAX_SLIDER_THUMBS` thumbs. Values stay ordered:
each thumb is clamped between its neighbors, so a drag can push a thumb to its neighbor's value but
never past it. `min_steps_between_values(n)` is Base UI's `minStepsBetweenValues`: it holds a gap of
`n` whole steps open between adjacent thumbs, re-orders the declared values outward from the first
thumb, and clamps every later movement so the gap can never close. A continuous slider has no step
to count, so the gap is inert there. A single-thumb slider projects the Slider role on its root; a multi-thumb slider
projects a group root and one Slider role per thumb, each carrying its own value and its
neighbor-derived bounds. Give each thumb an `.accessibility_label(...)`.

### Bounds

| Constant | Value | Meaning |
| --- | --- | --- |
| `MAX_SLIDER_THUMBS` | 8 | Values retained by one slider. |

Non-finite bounds fall back to `0.0..=1.0`, an inverted range is swapped, a non-positive step
selects continuous movement, and every value is clamped and snapped before it is retained.

## Number field

`NumberFieldState` wraps a controlled text value with parsing, clamping, formatting, and bounded
stepping. It composes the ordinary `text_input()` element, so editing, selection, IME, and
clipboard behavior are unchanged.

`NumberFieldFormat` owns the locale-shaped rules. QuickGUI has no locale database; the application
supplies the decimal separator, the optional grouping separator, whether a leading sign is accepted,
whether scientific notation is accepted, and the fixed fractional precision used when formatting.
Only ASCII digits parse.

```rust
let field = NumberField::new("quantity");
let edit = cx.input_listener(field.input_id(), |view: &mut Order, value, cx| {
    if view.quantity.set_text(value) {
        cx.invalidate();
    }
});

field.root_part(
    div()
        .child(field.input_part(&self.quantity, text_input(self.quantity.text().clone()).on_input(edit)))
        .child(field.decrement_part(&self.quantity, div().child("−")))
        .child(field.increment_part(&self.quantity, div().child("+"))),
)
```

Typing keeps arbitrary text: `set_text` records it and reparses, but never clamps or reformats.
`commit()` clamps into range and reformats; call it on Return and when focus leaves. Unparseable
text restores the last committed value, and an empty field stays empty. `step_by`, `increment`,
`decrement`, and `wheel` step and reformat immediately; `wheel` applies only when the field is
focused and uses only the delta's sign.

The input projects the SpinButton role with the committed numeric value, the finite parts of the
range, the step, and invalid state whenever the current text does not parse into range. QuickGUI
blocks Return submission for an invalid control, so commit from an ordinary key listener rather than
`on_submit` when a field can hold out-of-range text.

### Parts

| Base UI part | QuickGUI decorator | What QuickGUI owns |
| --- | --- | --- |
| Root | `root_part(element)` | the Group role and the identity every other part derives from |
| Group | `group_part(element)` | one addressable unit around the steppers and the input |
| Input | `input_part(state, element)` | the SpinButton role, numeric value and bounds, invalid, read-only, and required state |
| Increment / Decrement | `increment_part(state, element)` / `decrement_part(state, element)` | button semantics outside the Tab sequence |
| ScrubArea | `scrub_area_part(state, element)` | the pointer-capture identity, the axis cursor, drag exclusion, and selection suppression |
| ScrubAreaCursor | `scrub_area_cursor_part(element)` | a stable identity for the caller-owned cursor shown while scrubbing |

### Modifiers, snapping, and read-only

`small_step` (Alt) and `large_step` (Shift) default to one tenth and ten times the declared step.
`NumberFieldStepSize::from_modifiers` reads the held modifiers the way Base UI does — Shift wins
when both are held — and `step_by_size` / `step_with_modifiers` apply the result, so one path covers
the keyboard, the wheel, and the scrub area. `step_by` keeps its original meaning of one ordinary
step.

`snap_on_step(true)` is Base UI's `snapOnStep`: a stepped value lands on the nearest multiple of the
step measured from the minimum instead of adding to whatever the user typed.
`allow_wheel_scrub(false)` is Base UI's `allowWheelScrub`; QuickGUI has always applied focused wheel
input, so it defaults to `true` rather than Base UI's `false`.

`read_only(true)` refuses every commit, step, wheel notch, and scrub while the field stays focusable
and in the Tab sequence, which is what separates it from `disabled(true)`. `required(true)` projects
the native required state. `NumberFieldState::state()` returns a copyable `NumberFieldPartState`
carrying the scrubbing, stepping, disabled, read-only, required, and valid flags.

### Scrub area

`apply_scrub` takes a captured pointer event from a mounted scrub area and converts pointer travel
into whole steps at `scrub_sensitivity` logical pixels per step, bounded by
`MAX_NUMBER_FIELD_SCRUB_SENSITIVITY`. The unconverted remainder is retained for the length of the
gesture, so a slow drag still moves exactly one step at a time and a fast one loses no fraction.
`scrub_direction` chooses the axis: dragging right increases a horizontal area, dragging up
increases a vertical one, and `Both` accepts either. Held modifiers select the small or large step.

The gesture is pure pointer capture. It schedules no task, timer, or repeat, `is_scrubbing` and
`scrub_position` let the application mount and place its own cursor, and a released or cancelled
scrub leaves the field with no retained session at all.

### Press and hold

Holding a stepper repeats on exact one-shot deadlines and nothing else:

```rust
view.quantity.press_step(true, Instant::now()); // steps once, arms the first deadline
let deadline = view.quantity.repeat_deadline(); // None once released
view.quantity.repeat(now);                      // applies every step that came due
view.quantity.release_step();                   // no deadline, task, or timer remains
```

The state never owns a timer. It reports one deadline, the application sleeps until exactly that
instant with `AsyncViewContext::sleep_until`, and a released stepper leaves the window with no idle
source. A single late wakeup applies exactly the steps that came due rather than an unbounded burst.

### Bounds

| Constant | Value | Meaning |
| --- | --- | --- |
| `MAX_NUMBER_FIELD_TEXT_BYTES` | 64 | UTF-8 bytes retained by one field's editing text. |
| `MAX_NUMBER_FIELD_PRECISION` | 15 | Fractional digits one field formats. |
| `NUMBER_FIELD_REPEAT_DELAY` | 400 ms | Delay before a held stepper starts repeating. |
| `NUMBER_FIELD_REPEAT_INTERVAL` | 60 ms | Interval between repeats while held. |

Longer text is truncated on a character boundary instead of retained.

## Splitter

`SplitterState` owns pane sizes in logical pixels, per-pane minimum sizes, and which panes may
collapse. Sizes always sum to the splitter's total: moving a handle takes exactly what it gives.

```rust
let splitter = Splitter::new("workspace", &self.panes);
let sidebar = splitter.pane(0).expect("sidebar");
let content = splitter.pane(1).expect("content");
let handle = splitter.handle(0).expect("handle");
let drag = cx.pointer_listener(handle.handle_id(), |view: &mut Workspace, event, cx| {
    if view.panes.apply_pointer(0, event) {
        cx.invalidate();
    }
});

splitter.root_part(
    div()
        .child(sidebar.pane_part(div()))
        .child(handle.key_part(cx, handle.handle_part(div().w(6.0).on_pointer(drag)), access))
        .child(content.pane_part(div())),
)
```

Pane sizes along the split axis are framework-owned structural geometry: without them the resize
behavior would not exist. Every other layout and paint declaration is caller-owned, including the
handle's thickness and hit area. `handle_part` applies the platform column or row resize cursor
unless the caller sets `.cursor(...)` explicitly.

Captured pointer motion stays anchored to the handle position and window coordinate recorded on
pointer-down, so a splitter needs no container geometry, remains correct outside the window, and
does not accumulate drift if layout normalizes the panes between events. `set_total` rescales the
panes proportionally and then honors minimums; call it from a `container_query` when the surrounding
layout changes.
`reset` restores the sizes the splitter was created with — bind it to a handle double-click if the
product wants that gesture; QuickGUI does not assume it. `is_dragging` is true from the press
that starts a captured drag to the release or cancel that ends it; an owner that applies sizes
asynchronously, as the hosted bindings do, lets the drag own them until then instead of reseeding
them from a declaration that lags the pointer.

Install `splitter_key_bindings()` once. Left and Up shrink the preceding pane, Right and Down grow
it, Home and End move to its limits, and Enter collapses or restores a pane marked
`.collapsible(index, true)`. Enter on a pane that is not collapsible does nothing rather than
resizing silently.

Each handle projects the Splitter role with the preceding pane's size as its numeric value, that
pane's reachable minimum and maximum as the bounds, the split axis, and a controls relationship to
the pane it resizes. A row of panes is divided by vertical handles, matching the WAI-ARIA
window-splitter pattern where the orientation describes the separator rather than the pane axis.

### Bounds

| Constant | Value | Meaning |
| --- | --- | --- |
| `MAX_SPLITTER_PANES` | 16 | Panes managed by one splitter. |

A splitter needs at least two panes. Deeper layouts nest splitters instead of growing one.

## Progress and meter

`Progress` reports the completion of a task; `Meter` reports a static level inside a known range.
Both are plain descriptors with root, track, indicator, label, and value parts; the track and
indicator are accessibility-hidden decoration.

```rust
let download = Progress::new(3.0, 12.0).value_text("3 of 12 files");
download
    .root_part(div().accessibility_label("Download"))
    .child(download.indicator_part(div().w(track * download.completion().unwrap_or(0.0))))
```

`Progress::indeterminate()` reports work whose completion is unknown: it projects the progress role
with bounds but no value. **QuickGUI never animates an indeterminate indicator.** A moving barber
pole is product motion, and a framework-owned animation would keep an otherwise settled window
awake every frame. Use [declarative motion](animations.md) in the application when a product wants
one.

`Meter::new(value, min, max)` accepts optional `low`, `high`, and `optimum` markers, all clamped
into the meter's range, so an application can color a gauge without QuickGUI inventing thresholds.
`is_low` and `is_high` report which band the value falls in, and `completion` returns the
`0.0..=1.0` position for caller-owned fill geometry.

Non-finite inputs are dropped: a non-positive progress maximum falls back to `1.0`, an inverted
meter range is swapped, and values are clamped into range before they are retained.

### Parts

| Base UI part | QuickGUI decorator | What QuickGUI owns |
| --- | --- | --- |
| Root | `root_part(element)` | the progress or meter role, exact numeric value and bounds, the accessible value, and the label/value relationships |
| Track | `track_part(element)` | a stable identity, and keeping the fill out of the accessible name |
| Indicator | `indicator_part(element)` | a stable identity, and keeping the fill out of the accessible name |
| Label | `label_part(element)` | the accessible-name target the root points at |
| Value | `value_part(element)` | the accessible-description target the root points at |

Declaring `.id(...)` gives the descriptor a stable identity, from which the label, value, track, and
indicator identities are derived without allocation, and points the root's accessible name and
description at the mounted label and value parts. A descriptor without an identity keeps decorating
every part exactly as before and leaves the accessible name to the application's own
`accessibility_label`, so existing code is unchanged.

### Status and formatting

`Progress::status()` returns `ProgressStatus::{Progressing, Complete, Indeterminate}` — Base UI's
`data-progressing`, `data-complete`, and `data-indeterminate` — and `Progress::state()` returns a
copyable `ProgressPartState` carrying the status, value, maximum, and completion fraction so the
application can style a fill from one snapshot.

`ValueFormat` is Base UI's `format` prop: a bounded formatter of `(value, maximum)`.
`ValueFormat::percent()` and `ValueFormat::fraction()` cover the common shapes and
`ValueFormat::new(...)` takes any closure. QuickGUI never renders the result: `display_value()`
returns the text for a caller-owned `value_part`, and `accessible_value()` returns what assistive
technology reads. An explicit `value_text(...)` — Base UI's `getAriaValueText` — wins over the
formatter for assistive technology while the visible value part keeps the formatted string. `Meter`
takes the same `format` and `value_text`.

## Go components

`Progress.Root` / `Indicator` and `Meter.Root` / `Indicator` declare the value range, thresholds,
and value text these descriptors own; the QuickGUI UI renderer contributes no measurement logic.

`Slider.Root` / `Track` / `Range` / `Thumb` and `Splitter.Root` / `Pane` / `Handle` declare bounds,
values, step, and pane constraints; the hosted view reaches each declared instance's retained
`SliderState` or `SplitterState` through a per-instance [`StateAccessor`](view-api.md), so many
sliders and splitters in one window stay independent. Keyboard stepping, snapping, thumb ordering,
captured pointer arithmetic, and size conservation all run in the core, and the result reaches
Go as one asynchronous `OnValueChange` or `OnSizesChange` payload. `Slider.Track` uses the
track's own laid-out size, which the core now delivers on `PointerEvent::size`.

`NumberField.Root` / `Input` / `Increment` / `Decrement` declares the controlled value, the range,
the step, and the formatting precision; the hosted view reaches each declared instance's retained
`NumberFieldState` through a per-instance [`StateAccessor`](view-api.md). The core owns parsing,
clamping, formatting, and the exact stepper repeat: the binding arms the repeat on press, releases
it on lift, and asks the window for one repaint at the deadline the core reports, so it owns no
timer of its own. Editing text and validity travel back as one asynchronous `OnValueChange`, and a
Return that commits reports the clamped, reformatted value through `OnCommit` on the input part.
Because a control whose text does not parse into range refuses to submit, an invalid field reports
`valid: false` and commits nothing. See
[Go components](go.md).

## Resource contract

`SliderState`, `SplitterState`, `Progress`, and `Meter` are plain values; `NumberFieldState` retains
one bounded shared string. None of them retains an item registry, task, timer, observer, animation,
GPU resource, or idle scheduler source. The only deadline any of these components can produce is the
number field's stepper repeat, and it exists only while a stepper is held.

State changes rebuild only when the caller's listener requests invalidation. Every component in this
page returns a settled window to zero extra frames.

## Additional Go parts

`Slider` gained `Label`, `Value`, `Control`, and `Indicator` (Base UI's name for the range fill)
alongside the existing root, track, and thumb parts, plus `MinStepsBetweenValues`,
`ThumbAlignment`, and a bounded `Format` of `"percent"` or `"fraction"`:

```go
func VolumeSlider() {
	volume, setVolume := ui.CreateSignal([]float64{50})
	thumb := 0
	ui.Slider.Root(
		ui.SliderRootProps{
			Value:            volume,
			Min:              0,
			Max:              100,
			Step:             5,
			Format:           "percent",
			OnValueChange:    func(values []float64, _ *native.Event) { setVolume(values) },
			OnValueCommitted: func(values []float64, _ *native.Event) { log.Print(values) },
		},
		func() {
			slider := ui.UseSliderState()
			ui.Slider.Label(ui.PartProps{}, "Volume")
			ui.Slider.Value(
				ui.PartProps{},
				func() string {
					if value := slider().DisplayValue; value != nil {
						return *value
					}
					return ""
				},
			)
			ui.Slider.Control(
				ui.PartProps{},
				func() {
					ui.Slider.Track(
						ui.PartProps{},
						func() {
							ui.Slider.Indicator(ui.PartProps{})
						},
					)
				},
			)
			ui.Slider.Thumb(ui.SliderThumbProps{
				Index: &thumb,
				PartProps: ui.PartProps{Style: ui.Style().
					Opacity(func() float64 {
					if slider().Dragging {
						return 0.8
					}
					return 1
				})},
			})
		},
	)
}
```

`OnValueCommitted` is the core's own captured-pointer boundary, and `UseSliderState().Dragging` is
its `data-Dragging` flag; neither is a debounce or a guess in Go. A keyboard change reports
through `OnValueChange` alone, because the core exposes no keyboard commit.

`NumberField` gained `Group`, `ScrubArea`, and `ScrubAreaCursor`, plus `SmallStep` (Alt),
`LargeStep` (Shift), `SnapOnStep`, `AllowWheelScrub`, `ReadOnly`, `Required`, `ScrubDirection`,
`ScrubSensitivity`, and `OnValueCommitted`. The core turns a captured drag into whole steps at the
declared sensitivity, keeping the unconverted remainder so a slow drag moves one step at a time, and
`UseNumberFieldState().Scrubbing` is what a caller-drawn cursor styles from. `ReadOnly` refuses every
change while the control stays focusable, unlike `Disabled`.

`Progress` and `Meter` gained `Track`, `Label`, and `Value` parts and the same bounded `Format`.
`UseGaugeState()` reports the core's derived `Status` — `"progressing"`, `"complete"`, or
`"indeterminate"` — the formatted `DisplayValue`, and the `Completion` fraction. See
[Go components](go.md).
