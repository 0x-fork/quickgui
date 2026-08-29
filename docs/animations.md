# Declarative motion

[Documentation index](README.md)

QuickGUI provides paint-only web-style transitions, declaration-time duration animations, and
retained physical springs. They participate in the same damage-driven scheduler, respect Reduce
Motion, pause while their window is not presenting, and release their scheduling source when
inactive. A clean window still returns to `ControlFlow::Wait`.

## Style transitions

Use `transition` to animate interaction-state or application-driven changes to background, border
color and width, corner radius, inherited text color, subtree opacity, and bounded CSS-like shadow
lists:

```rust
use std::time::Duration;
use quickgui::{Color, Transition, TransitionProperties, button, text};

let control = button()
    .bg(Color::rgb8(51, 65, 85))
    .opacity(0.88)
    .rounded_lg()
    .hover(|style| {
        style
            .bg(Color::rgb8(79, 70, 229))
            .border(2.0, Color::rgb8(165, 180, 252))
            .rounded(14.0)
            .shadow_lg()
            .opacity(1.0)
    })
    .transition(
        Transition::new(Duration::from_millis(140))
            .with_properties(TransitionProperties::ALL),
    )
    .child(text("Continue"));
```

A `Duration` converts directly to an all-property transition, so
`.transition(Duration::from_millis(140))` is the common form. `transition_colors` is the compact
color-only helper. The default easing is `ease_in_out`; custom finite easing and `with_max_fps`
use the same API shape as duration animations.

Opacity remains a scalar paint value throughout a transition. It multiplies with ancestor opacity
and reaches every specialized pipeline and embedded macOS child view, while Glyphon applies it
after resolving rich-text run colors. An opacity-only frame therefore reuses text shaping, image
textures, SVG masks, path tessellation, and custom-shader pipelines.

Transitions are keyed by the element's retained runtime identity. Hover, active, focus, invalid,
drag-source, and drag-over changes reuse the existing element tree and Taffy layout. Interrupting
or reversing a transition samples the current presentation value first, so the new target begins
without a visual jump. Properties not selected by `TransitionProperties` update immediately.
Shadow interpolation uses a fixed eight-entry inline list; compatible entries interpolate in
linear-light color and geometry, missing entries fade through a neutral shadow, and an inset/drop
mismatch changes discretely.

## Duration animations

`AnimationExt` is implemented for every `IntoElement` value. The callback receives the normalized
`Element` and the eased phase:

```rust
use std::time::Duration;
use quickgui::{Animation, AnimationExt, AnimationPhase, Color, div, ease_in_out};

let progress = div().h(8.0).with_animation(
    "load-progress",
    Animation::new(Duration::from_millis(240)).with_easing(ease_in_out),
    |element, value| {
        let phase = AnimationPhase(value);
        element
            .w(phase.interpolate_clamped(24.0, 420.0))
            .bg(phase.interpolate_clamped(
                Color::rgb8(249, 115, 22),
                Color::rgb8(34, 197, 94),
            ))
    },
);
```

A one-shot animation retains its terminal value and stops requesting frames. Change its ID to
intentionally replay it. `repeat()` uses a local start time; `repeat_synced()` joins the shared
application animation epoch so independently mounted elements can share one phase.

Use `with_animations` for a bounded sequence of up to 64 stages. Its callback receives the stage
index as well as that stage's phase. Elapsed time carries across stage boundaries, including a
frame that arrives late.

`with_max_fps` replaces continuous presentation-rate requests with one exact deadline at a time.
Invalid caps are treated as unthrottled, and accepted timer cadence is capped at 240 FPS. A
one-shot always wakes at its stage boundary or terminal frame even when the requested cadence is
slower than its duration. This is useful for ambient motion that does not need every display
refresh.

## Springs

Springs preserve position and velocity when the target changes under the same stable ID:

```rust
use quickgui::{AnimationExt, SpringAnimation, SpringConfig, div};

let knob = div().with_spring(
    "knob-position",
    SpringAnimation::new(SpringConfig::new(170.0, 14.0, 1.0))
        .to(target_x)
        .with_epsilon(0.05),
    |element, x| element.left(x),
);
```

The spring solver is analytic rather than an accumulated fixed-step simulation. Advancing by a
long delayed interval therefore produces the same state, within floating-point precision, as
advancing through smaller intervals. A runtime delay does not create an unbounded catch-up loop.

`SpringPlayback` supports running, paused, stopped, completed, and cancelled presentation. Pausing
freezes both position and velocity without counting hidden time. Completing snaps to the target;
cancelling returns to the retained initial value.

## Lifecycle, bounds, and accessibility

- The main view and each detached motion tree retain at most 4,096 mounted declarative motion IDs
  in separate namespaces; duplicate IDs within one tree fail declaration. A tooltip's 256-element
  content cap is the tighter practical bound there.
- A window retains at most 4,096 opted-in style transitions, each with at most eight box shadows.
  The count is rejected before layout and inactive entries are released when their element
  unmounts.
- Unthrottled active motion follows presented frames rather than creating an independent timer
  loop. A max-FPS animation retains only its next deadline.
- Local motion pauses while occluded and resumes without catch-up. Synchronized repeats rejoin the
  application epoch when visible again.
- With Reduce Motion enabled, style transitions and springs resolve directly to their target,
  one-shots resolve to their final phase, and repeating animations resolve to phase zero. No
  animation frame is scheduled. The effective value combines a window's explicit
  `WindowOptions::reduce_motion` policy with the Rust core's current `SystemPreferences` snapshot.
- Easing and spring inputs are checked for non-finite values before they can poison layout or the
  retained scheduler.

Duration animations and springs also work inside GPU tooltip content and drag previews. Each
detached surface owns a sanitized declaration template, a local motion-ID namespace, retained
playbacks, and its own Taffy tree. Only that small tree is re-resolved and laid out on an active
frame; the application `View::render` is not called. Dismissing the tooltip or clearing the preview
drops its playbacks and pending deadline together, so a detached animation cannot leave an
ownerless wake source.

Animator callbacks are sanitized again after every resolution. They cannot add pointer listeners,
focus, nested tooltips, application drag regions, or other interaction to a detached snapshot.
Paint-style transitions are removed as well because detached surfaces have no interaction-state
target; use a duration animation or spring for their entrance and motion.

Run the interactive sample with:

```console
cargo run --release --example animations
```
