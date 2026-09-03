# Context menus

[Documentation index](README.md) · [Popovers and popover menus](popovers.md) · [Component roadmap](component-roadmap.md)

QuickGUI separates the context-menu trigger, retained menu model, native popover host, and visual
presentation:

- `ContextMenuState` is application-owned and retains only the current direct child handle;
- `PopoverMenu` owns the validated items, keyboard navigation, typeahead, toggles, and submenus;
- `ContextMenuLayout` owns structural popover and row measurements, never appearance;
- the application supplies the target, popover root, and every rendered row;
- the root menu opens at the exact secondary-click point in a separate WGPU child surface, so it
  may extend beyond the owner window while native work-area constraints flip and slide it;
- typed item actions are delivered to the nearest non-popover owner before the popover chain closes.

Bind `popover_menu_key_bindings()` once on the application, retain one state value, then attach the
behavior to any caller-owned element:

```rust
struct EditorView {
    context_menu: ContextMenuState,
}

impl EditorView {
    fn context_menu(view: &mut Self) -> &mut ContextMenuState {
        &mut view.context_menu
    }
}

let target = self.context_menu.element(
    cx,
    "editor-surface",
    Self::context_menu,
    div().size_full(),
    ContextMenuLayout::new(224.0, 36.0)
        .separator_height(9.0)
        .group_label_height(22.0)
        .vertical_padding(4.0),
    |_view, _event| Some(application_menu()),
    || div().p_1(),
    render_menu_item,
);
```

`target_part` can apply only the `has-popup` and expanded semantics when an application needs to
compose the low-level `on_context_menu` listener itself. Neither API changes colors, borders,
typography, cursor policy, focusability, drag regions, or ordinary click behavior.

## Lifecycle and resource ownership

Opening replaces any prior root surface. Exact direct-child close notification clears stale state
after Escape, an outside press, command activation, programmatic close, or native teardown. A late
close from an older replacement cannot clear the newer handle. Submenus are parented to the menu
level that opened them and close child-first with the root. The root alone owns the menu-style
grab and outside-click monitor. Attached submenu panels do not add monitors; AppKit focus moving
between levels remains inside the chain, while focus leaving the complete chain dismisses its
root.

Closed state owns no window, renderer, timer, task, observer, event monitor, or scheduler source.
An open root owns one child surface; each open submenu owns one additional child surface, bounded
by `MAX_POPOVER_MENU_DEPTH`. Pointer hover owns at most one cancellable exact deadline per open menu
level while submenu intent is pending. Replacing the hovered row, leaving before open, entering the
existing child, closing the child, or tearing down the popover cancels that deadline. A settled menu
has no deadline, animation frame, or polling source. Menu item count, nesting, label bytes,
aggregate text, and typeahead are bounded by the `PopoverMenu` constants. Typeahead expiration is
checked on the next key and schedules no timer.

Large searchable datasets belong in a virtualized select, autocomplete, or command palette rather
than a thousands-of-rows context menu. `ContextMenuLayout` clamps native dimensions to the window
limits, while ordinary menu rows remain finite under the popover-menu model bounds.

## Base UI parts

`ContextMenuState` shares the [`PopoverMenu`](popovers.md#unstyled-popover-menus) row model, so its
Item, LinkItem, CheckboxItem, RadioItem, indicator, Group, GroupLabel, RadioGroup, and Separator
parts are exactly the ones documented there. Two names are its own:

| Base UI part | QuickGUI decorator | What QuickGUI owns |
| --- | --- | --- |
| Trigger | `trigger_part(id, element)` (alias of `target_part`) | `has-popup`, `expanded`, secondary-click opening |
| Backdrop | `backdrop_part(id, element)` | full-viewport, accessibility-hidden pointer layer in the owner window |

`ContextMenuState::item_part_state(menu, index, submenu_open)` returns the same `MenuItemPartState`
snapshot the in-window menu publishes. Portal, Positioner, Popup, and Arrow have no separate
decorator here: the menu surface is its own native child window, so QuickGUI resolves its placement
against the display work area rather than a parent stacking context, and the popover view already
applies the popup semantics to the caller's `render_root` result.

## JavaScript bindings

The Solid renderer exposes this adapter as `ContextMenu.Root` / `Trigger` with a bounded JSON item
model and an `onSelect` event carrying the declared item id. One window owns exactly one
`ContextMenuState`, matching the native invariant that opening a menu replaces the one already open.
The cursor-point surface is a separate native window, so its rows are rendered by the binding from
the declaration's `appearance` values instead of from JavaScript. See
[Solid 2 renderer](solid.md#declared-popover-and-context-menus).

## Submenu pointer behavior

Hovering a submenu row highlights it immediately and opens it after the exact
`CONTEXT_MENU_SUBMENU_HOVER_DELAY` deadline. Right Arrow and click remain immediate. Once a child is
open, QuickGUI receives its actual post-constraint desktop rectangle and protects a diagonal safe
corridor from the parent pointer to the child's near edge. Crossing sibling rows inside that
corridor defers replacement for at most `CONTEXT_MENU_SUBMENU_AIM_DELAY`; entering the child cancels
the replacement. The near edge is derived from actual placement, so the same behavior applies when
work-area fitting flips a submenu to the left.

This coordination uses one root motion listener and row hover transitions, not one motion listener
or timer per row. `PopoverMenu::element_with_submenus_and_hover` exposes the appearance-free hover
hook for another native host; `ContextMenuState` supplies the native menu-aim policy by default.

Keyboard/click/hover submenu opening, nested native focus transfer, descendant-aware outside
clicks, nested command dispatch, point placement, close synchronization, accessibility item roles,
and edge constraints are implemented. The self-driving macOS gate now exercises eight submenu
command closes, four owner-press dismissals, and four native Escape dismissals with a submenu open,
requiring exact focus recovery, no accidental command, complete child teardown, and zero extra idle
frames. A final cooperative application-deactivation cycle requires both popover windows to close
while the owner remains non-key and unfocused. The resource phase additionally completes 128 real
root-plus-submenu command lifecycles at a reported 50 ms inter-cycle cadence. Sampling after cycles
32 and 128 on the current reference run showed 1.109 MiB positive RSS growth and no positive
physical-footprint growth, below the enforced 16 MiB and 24 MiB budgets, with no more than two live
popover windows and zero extra idle frames. These direct responder interactions are intentionally
too short-lived for visual inspection. Remaining acceptance work is live VoiceOver confirmation
of the implemented group/separator roles and relationships, focus announcements, and multi-monitor
mixed-scale and nested-submenu placement.

See [`tooltips_context_menu.rs`](../examples/tooltips_context_menu.rs) for a styled application-owned
root and row renderer. The styling is gallery code, not framework API.
