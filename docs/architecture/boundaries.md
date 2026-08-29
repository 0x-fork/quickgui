# Current architecture boundaries

[Architecture index](README.md) · [Documentation](../README.md)

This milestone establishes the performance architecture, ordered overlays, bounded delayed
tooltips, cursor-anchored context surfaces, macOS anchored/nonactivating popover panels, native child
composition, hybrid AccessKit/AppKit accessibility, semantic focus, and production-oriented
single-line and multiline editing paths, typed actions, focus scopes, contextual keymaps, native macOS application
menus, event-driven macOS keyboard-layout command translation, bounded static/asynchronous/animated raster images, retained SVG icons, retained paths and
custom canvas painting, retained custom shaders, retained CSS Grid, static and controlled editable styled text, structured forms, typed entity events, application globals, cancellable foreground tasks, analytic CSS-ordered box shadows, and a compile-time opt-in bounded retained-tree inspector. A complete
GPUI-equivalent desktop toolkit still needs compositor-native non-macOS popover/menu projection,
cross-window native drag promotion outside macOS, and broader platform acceptance. Those features
should extend the retained tree and narrow renderer rather than bypass its scheduling and cache
invariants. Stylus/tablet axes are deliberately not treated as a parity or release requirement;
specialized drawing hardware should enter the roadmap only when an application establishes a real
contract for it.

Reusable selection controls remain declarations above the renderer. Checkbox, radio, and switch
values and all presentation are application-owned; their unstyled root/indicator/thumb parts add
roles and toggle states to the same AccessKit tree as custom elements. Radio roving focus scans the
retained declaration only on focus-index rebuild or arrow input. This layer creates no GPU asset,
component store, polling observer, per-frame allocation, or idle scheduler source.

Window-bounded popovers are controlled pairs above the same retained overlay path used by context
surfaces. Their descriptor copies stable trigger/content IDs, placement, semantic kind, and visual
tokens; application state decides whether content is mounted. Anchor placement, topmost dismissal,
pointer blocking, and focus restoration remain `UiTree` responsibilities. The component adds no
parallel popover registry, observer, timer, next-frame task, or native window, and closed content
retains no overlay or accessibility node.

Overflow-capable `SystemPopover` resolves one stable trigger ID through that same retained
geometry, then creates a parent-owned native child with an independent WGPU surface. On macOS the
child is an `NSPanel` constrained to the display work area rather than the parent viewport. Grab
and native key-window eligibility are independent policies: menu/select children grab and own key
focus, ordinary passive surfaces may become key after interaction, and autocomplete uses an
interactive never-key panel so the owner text input retains its IME session. The descriptor owns
no geometry observer or scheduler source; non-grabbing children own no monitor, and grabbing
children share the bounded process monitor pair. High-level menu, select, and autocomplete
dropdowns use this host so crossing a parent edge is an explicit guarantee rather than an
accidental clip.

`PopoverMenu` now supplies the unstyled behavior layer above that host. It validates the complete
bounded submenu tree before retention, derives row identities from the caller's menu ID, and uses
one composite focus root with active-descendant semantics. Pointer, keyboard, typeahead, and typed
commands reuse ordinary retained listeners and action dispatch. Nested popover commands target the
nearest non-popover owner directly, and chain close reuses parent-child window teardown; neither path
introduces a callback registry, serialized command format, timer, observer, or idle frame.

Standalone `SelectState`, `AutocompleteState`, and `ComboboxState` share bounded `PickerItem`
sources while retaining different value contracts. Select owns one optional committed index and
uses a key/grabbing native child. Autocomplete retains arbitrary bounded text without a committed
index. Combobox retains a declared committed value independently from its temporary edit query and
restores that label on every non-commit dismissal. Both editable controls send a bounded match
snapshot to an interactive never-key child. Pointer hover/commit returns as typed actions;
keyboard editing and IME never leave the owner input. The visual child subtree is
`accessibility_hidden`, while a viewport-bounded semantic ListBox proxy stays in the owner AccessKit
tree so controlled/active-descendant relationships never cross window trees. Source `T` values are
shared through `Arc`, match ranges are shared, externally filtered sources can disable local fuzzy
filtering, and open source replacement updates the existing child entity. Closed state retains no
child/entity; open settled state adds no timer, monitor, geometry poll, task, or idle frame. All
three public component contracts take caller-owned visual parts; there is no combobox appearance
preset or select-only combobox compatibility mode.

Reusable tables and trees remain controlled declarations above `ListState`. A table retains
constant-size cell/sort interaction state while the application owns records and ordering. A tree
validates source bounds, flattens nodes once into a compact preorder arena, and rebuilds only its
visible-index vectors when expansion changes. Both use one composite focus target and project
active descendants plus exact collection counts, indices, levels, set positions, and sort state
through the ordinary AccessKit tree. They create no parallel renderer, data model, observer,
timer, or idle scheduler source.

Native document-window integration is deliberately retained window state plus narrow platform
projection. Represented paths and tab identifiers are bounded application data; AppKit windows,
URLs, and tab groups never enter the public tree. Tab snapshots are sampled only at explicit
commands and native lifecycle boundaries, capped at 256 entries, and do not add a timer, polling
observer, scheduler source, renderer cache, or frame-owned allocation.

`WindowAppearance` intentionally models the portable semantic light/dark choice. AppKit vibrancy,
materials, contrast, and transparency are separate visual/composition capabilities rather than
pretend palette variants; native child views continue to inherit the effective `NSWindow`
appearance. `WindowBackgroundAppearance` currently exposes the portable semantic subset QuickGUI
can implement honestly—opaque, transparent, and blurred. Windows-only Mica names are intentionally
not presented as macOS features; platform-specific material controls can extend this boundary
without changing light/dark palette semantics.
