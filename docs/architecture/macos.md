# macOS composition architecture

[Architecture index](README.md) · [Documentation](../README.md)

## macOS window chrome

`WindowKind::Dialog` is parent-modal and is presented as an AppKit sheet only after its hidden GPU
frame is complete. `Floating` uses its native elevated level. `Popover` and `SystemPopover` are
nonactivating panels, remain transient across spaces, and hide on application deactivation. Each
kind uses the same retained view, input, accessibility, scheduling, and bounded renderer ownership
as a normal window.

Anchored placement converts a validated parent-content rectangle into AppKit's global screen space
before GPU initialization, selects the visible work area containing the anchor, then deterministically
flips, slides, or resizes according to `PopoverConstraintAdjustment`. It resolves again immediately
before first presentation, so hidden first-frame preparation cannot flash on the wrong display.
The panel is attached with `addChildWindow`, and hiding or closing removes that native relation.
Menu-style grabs share one lazy local/global mouse-monitor pair across at most 32 nested popovers;
only the top entry can request dismissal, duplicate requests coalesce, and removing the last entry
removes both monitors. Escape and focus loss use the normal serialized Winit event path. Passive
panels retain no monitor, timer, thread, or redraw source. An explicit `close_popover_chain()` closes
the root and all descendants child-first and returns native keyboard focus to the nearest
non-popover owner; outside-click and application-deactivation dismissal do not steal focus back from
another window. The live acceptance gate repeats a real root-menu/submenu activation eight times
and then dismisses four more open root/submenu chains from real owner-window presses plus four from
native Escape key events delivered through the key popover's AppKit/Winit responder. It requires
owner key-window, Winit first-responder, retained-focus, and zero-popover teardown state after every
active-application cycle, and requires both dismissal routes not to deliver a menu command. The
gate next sustains 128 complete root-plus-submenu command lifecycles at an explicit 50 ms
inter-cycle cadence. It samples current process RSS and physical footprint after cycle 32 and cycle
128, requires positive growth to stay within 16 MiB and 24 MiB respectively, and requires no more
than two popover windows at once. A final cooperative activation handoff to Finder verifies that
application deactivation tears down the complete native chain while leaving the owner non-key and
unfocused; it deliberately queues no focus restoration that could steal activation back. The run
then finishes with zero extra idle frames. Those scripted surfaces are deliberately too short-lived
to serve as human-visible popover QA; they prove native creation, routing, and teardown instead.

`TitleBarStyle::HiddenInset` keeps AppKit's standard window controls while making the titlebar
transparent and extending the Winit content view through it. Traffic-light coordinates are logical
top-left points; the runtime converts them through AppKit's content-layout rect and preserves the
native inter-button spacing. Resize and scale transitions reapply the configured position.

Hidden-inset creation sets `NSWindow.movable` to false, removing AppKit's implicit titlebar drag
region. An element's optional `AppRegion::Drag` or `AppRegion::NoDrag` declaration enters the same
ordered hit list as pointer targets. The topmost explicit declaration wins, with built-in overlay
scrollbars taking precedence. A primary press in a drag region starts AppKit's synchronous native
window drag, enabling movability only for that call and restoring the hidden-inset restriction on
return. No drag-region hover or click state is painted and no continuous tracking loop is added.

## Document windows

Represented-file and edited-state mutations are serialized through the ordinary bounded window
command queue. The retained `WindowState` changes only after AppKit accepts the corresponding
`NSWindow` mutation; neither property is polled during normal rendering. The live macOS acceptance
gate sets both properties, requires the retained and native states to agree, clears both, and
requires agreement again. Native system-tab grouping and navigation are optional deferred
capabilities and do not block the macOS-first 0.1 release.

## Overlays and native composition

Anchored overlays resolve after natural Flexbox layout so their target bounds are stable. The
placement algorithm prefers the requested side, flips when the opposite side has more available
space, tries alternate alignment, shifts to the viewport margin, and pins oversized surfaces to
that margin. Portal overlays escape ancestor clips. Hit, scroll, hover, and dismissal regions use
the same plane/z/source order as paint, preventing click-through to visually covered content.

On macOS, each native child is retained by identity and mounted inside two flipped AppKit wrappers:
an outer clipping view and an inner rounded-corner view. Bounds are expressed in QuickGUI logical
points. The parent Winit/WGPU view remains the base surface and a transparent sibling is always
ordered above native children. Its hit test is disabled while no overlay is interactive; while an
overlay is open it forwards input to Winit so outside-click dismissal cannot activate the native
control underneath.

AppKit mouse-down monitoring only queues a redraw for the affected window. At that event boundary,
QuickGUI compares the real first responder with the Winit view and clears stale semantic focus.
Focusing a QuickGUI element makes the Winit view first responder again. This preserves an idle
event loop while giving native controls normal keyboard and IME ownership. The live macOS
acceptance gate verifies both sides of that handoff using an actual `NSTextField` field editor and
the framework's semantic focus state.

The surface uses guaranteed FIFO presentation and a two-frame latency hint. A latency of one is intentionally not the default because WGPU documents that it prevents CPU/GPU overlap and prioritizes latency over throughput.
