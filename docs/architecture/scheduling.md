# Scheduling and performance

[Architecture index](README.md) · [Documentation](../README.md)

## Frame scheduling

The event loop runs with `ControlFlow::Wait`. Per-window invalidation queues at most one Winit
redraw. Multiple wheel events accumulate into one logical delta for their owning window before its
redraw handler runs. An occluded or zero-sized window is skipped and is not placed in a retry loop.

Hover, pressed state, and retained scroll-container offsets only mark paint dirty. They reuse the
existing element declaration and Taffy layout. An application mutation calls
`EventContext::invalidate()`, which rebuilds the declarative tree at the next redraw.
`request_animation_frame()` is explicit and keeps rebuilding only while a view requests it;
`request_repaint_at()` instead stores one exact view deadline. Asynchronous images use owner-tagged
Winit user events for completion and one `WaitUntil` deadline for the 200 ms loading threshold.
Animated images add only the earliest visible frame deadline. Overlay scrollbar visibility adds
one hide deadline after the final scroll/leave/release transition. A pending tooltip adds one show
deadline and is cancelled without repaint if its pointer/focus target changes first. The runtime selects the earliest
key, loading, view, scrollbar, tooltip, task, or animation deadline across all windows; none of
these states introduces polling or a permanent frame loop.

Declarative duration animations either request the next presented frame or, when capped with
`with_max_fps`, retain one exact next-frame deadline. Stateful springs use the presented-frame path
and an analytic elapsed-time solution, so a delayed frame requires one constant-time state
transition rather than fixed-step catch-up work. Completed motion removes its request immediately.
One application animation epoch phase-locks synchronized repeats across windows. Occluded windows
pause local motion and resume without counting hidden time; synchronized repeats deliberately
rejoin the application epoch. Reduce Motion resolves a static value and contributes no deadline.

GPU tooltips and drag previews use the same clock contract without setting `view_dirty`. Each
detached surface retains its own sanitized template and playback registry. An active frame
re-resolves and lays out only that bounded detached tree; capped motion contributes one exact
deadline to the window's existing earliest-deadline selection. Dismissal drops the tree, registry,
and deadline as one owner, and stationary or completed detached content schedules nothing.

Web-style transitions run one layer lower: paint samples retained interaction targets and reuses
the existing declaration and Taffy boxes. A hover, active, focus, validation, or drag-state edge
therefore starts or reverses color, border, radius, text-color, and shadow interpolation without
setting `view_dirty`. Unthrottled transitions request another presented paint; capped transitions
retain one deadline no later than their terminal frame. Each active transition performs one
elapsed-time sample, never fixed-step catch-up.

Native light/dark appearance changes arrive as ordinary Winit window events. They update one
retained value and request a redraw only when the view observed window state; appearance does not
join the deadline set and never requires preference polling.

Native cursor styles are resolved on pointer movement and once after an already-damaged frame
rebuilds its hit stack under a stationary pointer. The runtime caches the last platform cursor and
skips identical native calls. A cursor declaration contributes no invalidation, deadline, timer,
animation-frame request, or paint by itself.

Wrapped text is keyed by all shaping inputs independently from its available width. When a stable
text ID changes only width, the retained Cosmic Text buffer updates its size and reruns line layout
while preserving the already-shaped glyph lines. Font, content, scale, alignment, or styled-run
changes still create the appropriate new shaped layout. Per-ID and shared caches retain their hard
bounds, and temporary visible-buffer references are dropped immediately after frame submission so
the next resize can reclaim a uniquely owned buffer in place.

Text overflow is likewise damage-driven. Standard `…` end/start/middle and line-clamp modes are
part of Cosmic Text's retained line layout, so width-only changes preserve the original shaped
glyph runs and reflow them in place. Unchanged frames do no work. A custom replacement affix uses a
grapheme-safe shortened projection; because that cannot be width-reflowed as the original string,
the per-ID replacement path removes its uniquely owned superseded shared entry immediately.
Genuinely shared layouts retain the normal age and count bounds.

On macOS, a resize command updates the desired drawable extent without moving the window or
immediately rebuilding WGPU's surface. The renderer keeps a bounded grow-only validated surface
capacity and a stable Metal drawable pool through a resize burst; QuickGUI still lays out and
projects each frame against the exact current viewport while Core Animation scales that temporary
drawable. The first stable frame outside AppKit live resize restores an exact-size drawable for
stationary text. This avoids a device-idle surface configure at every pixel boundary, preserves
native-child geometry, and adds no timer or idle frame.

Window background commands likewise mutate one retained compositor policy and coalesce into one
redraw. Transparency and blur transitions are diffed separately, so an unchanged native component
is not called again. Transparent and blurred windows add no deadline and return to the same idle
wait after their transition frame.

## Foreground tasks

`ViewContext::spawn` and `EventContext::spawn` schedule a non-`Send` future with `async-task`'s
local executor. The scheduler callback may be woken from any thread, but it only moves a `Runnable`
into one bounded synchronized ready queue and sends one coalesced Winit user event. The application
thread drains at most 256 ready futures per event-loop turn before yielding to platform input; it
is the only thread that polls or destroys application future state. The application admits at most
4,096 live foreground tasks and each native window owns at most 1,024.

The public `Task<T>` handle is deliberately main-thread-only. Dropping it requests cancellation;
`detach()` releases the result handle without releasing window ownership. Closing the owner window
aborts both retained and detached work, discards its queued view updates and timers, and prevents a
late wake from touching a replacement view. Runtime shutdown closes and drains the ready queue on
the application thread. A bounded leak is used only if an external waker races after that thread
has stopped accepting work, because destroying captured `Rc` state on the waking thread would be
unsound.

`AsyncViewContext<V>::update` does not reenter the view from inside a future poll. It moves one
typed closure into the task's at-most-64-entry update queue, returns `Pending`, and the runtime
executes that closure against the owning view after the runnable releases executor state. The
closure result then wakes the future. Missing owners and mismatched view types are reported through
`AsyncContextError` rather than dereferencing stale state.

`AsyncViewContext::sleep` registers one monotonic deadline in the task registry. The earliest task
timer joins image, animation, scrollbar, tooltip, key-sequence, and requested-repaint deadlines in
Winit's `ControlFlow::WaitUntil`; when due, its waker is scheduled exactly once. There is no timer
thread, interval tick, polling future, or animation-frame request. A task owns at most 64 pending
timers, the application retains at most 4,096 timers, and one task can queue at most 64 view
updates.

## Performance rules

Changes should preserve these invariants:

- no redraw request merely because a window exists;
- no full data-set walk in scrolling or painting;
- no text shaping keyed by screen position;
- no unbounded retained cache;
- no per-primitive GPU draw call;
- no steady-state or per-pixel wait for GPU completion on the UI thread;
- no layout-affecting hover style.

Application-thread CPU time, frame-submission wall time, quad/shadow/image/SVG/path/text counts,
path vertices and skipped paths, CPU/GPU image residency, GPU SVG-mask residency, resource
loading/failure counts, mounted/active animation counts, image uploads, SVG rasterizations, text
cache hits and retained-entry counts, and draw-call counts are exposed through `FrameMetrics`.
Unix targets read the monotonic per-thread CPU clock, keeping FIFO surface waits out of CPU
telemetry; `frame_time` deliberately includes that wait. These are not GPU timestamps or
end-to-end display latency.

`scripts/macos-performance-gate.sh` runs an automated bidirectional 100,000-row scroll against a
real WindowServer, checks active and idle frame telemetry, and adds whole-process CPU and peak-RSS
acceptance. `scripts/macos-acceptance-gate.sh` separately drives real native resizes with 64 wrapped
and 16 line-clamped/ellipsized Unicode areas plus an embedded `NSView`, then verifies presentation cadence, CPU/cache/draw bounds,
native detach/remount/geometry/teardown, idle frames, and process RSS. Those two performance gates
also bound macOS's physical footprint so GPU and IOSurface ownership cannot hide behind a small
resident-set number. `scripts/macos-display-acceptance-gate.sh` uses bounded one-shot deadlines to
compare hidden, non-key windows on every display with first-render centering and settled live
AppKit identity, scale, frame origin, and work-area containment; it performs no idle monitor
polling. The deterministic test context routes `simulate_retained_scroll` through
`UiTree::scroll_at`, proving that thousands of ordinary overflow updates preserve a clean
declaration and stable render count.
