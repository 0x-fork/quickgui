# QuickGUI documentation

QuickGUI's documentation is split by concern so public API guidance stays separate from renderer and platform internals.

## Guides

- [View API and layout](view-api.md) — declarative views, Tailwind-style helpers, Flexbox, CSS Grid, and parent-size container queries.
- [Solid 2 renderer](solid.md) — Bun/N-API hosting, unstyled native components, JSX compilation, reactive event boundaries, and the current binding scope.
- [Windows and shared state](windows.md) — multiple native windows, entities, globals, roles, bounds, appearance, backgrounds, and commands.
- [Native document windows](document-windows.md) — represented files, edited state, character palette, system tabs, and bounded observation.
- [Displays and window placement](displays.md) — bounded monitor snapshots, work areas, scale factors, stable macOS identities, and targeted placement.
- [macOS integration](macos.md) — platform services, notifications, compositor backgrounds, hidden-inset chrome, native views, and first-frame presentation.
- [Text, editing, and forms](text-and-forms.md) — wrapping, rich text, selection, controlled editors, validation, and forms.
- [Selection controls](selection-controls.md) — controlled checkboxes, radio groups, switches, checked/mixed accessibility, and roving keyboard behavior.
- [Collapsible and accordion](disclosures.md) — controlled unstyled disclosure parts, current Tab/Enter/Space behavior, exact heading/region relationships, bounded open values, and mounting policy.
- [Tabs](tabs.md) — controlled unstyled in-window tab parts, manual or automatic activation, horizontal or vertical roving focus, panel mounting, and exact accessibility relationships.
- [Popovers and popup menus](popovers.md) — controlled in-window overlays, cross-edge native WGPU hosts, unstyled menu parts, typed owner actions, nesting, placement, and popup accessibility.
- [Context menus](context-menus.md) — unstyled cursor-point triggers, native overflow surfaces, popup-menu composition, exact lifecycle, nested actions, and resource bounds.
- [Select, autocomplete, and combobox](select-and-autocomplete.md) — three distinct unstyled value contracts with overflow-capable native children, never-key owner-IME behavior, accessibility portals, async source replacement, validation, and bounded virtualization.
- [Dialogs](dialogs.md) — unstyled modal portal/backdrop/popup/title/description/close parts, nested focus containment, independent dismissal, restoration, and exact accessibility.
- [Unstyled component roadmap](component-roadmap.md) — Base UI-derived component inventory, behavior/presentation boundary, dependency order, and current gaps.
- [Virtual tables and trees](data-collections.md) — controlled sortable grids, bounded hierarchies, composite keyboard focus, collection accessibility, and visible-only mounting.
- [Application assets and custom fonts](assets-and-fonts.md) — bounded bundled resources, stable image handles, view access, and startup font registration.
- [Clipboard](clipboard.md) — bounded text and metadata, encoded images, native file lists, macOS Find pasteboard, and deterministic tests.
- [Graphics and media](graphics.md) — shadows, raster and animated images, SVG, paths, canvas, and custom WGSL.
- [Declarative motion](animations.md) — paint-only web transitions, duration curves, chained stages, retained springs, independently owned tooltip/drag-preview motion, exact throttling, and Reduce Motion.
- [Input and interaction](input.md) — native cursors, exact wheel input, keyboard layouts, IME-safe commands, two-phase focused keys and typed actions, pickers, menus, overlays, tooltips, context menus, and drag/drop.
- [Asynchronous work](async.md) — cancellable foreground futures, exact timers, and bounded background work.
- [Deterministic testing](testing.md) — headless interaction, exact time, geometry assertions, and bounded WGPU screenshots.
- [Retained-tree inspector](inspector.md) — feature-gated picking, hierarchy/layout/hit/accessibility snapshots, frame damage, and resource bounds.
- [Examples and performance checks](examples-and-performance.md) — Hacker News, the 100,000-row stress case, benchmarks, and validation.
- [Releasing QuickGUI 0.1](releasing.md) — clean gates, package-archive smoke testing, and dependency-ordered publication.
- [Changelog](../CHANGELOG.md) — user-facing changes in each published QuickGUI version.
- [Status and roadmap](status.md) — implemented surface, ordered release blockers, later work, and explicit non-goals.

## Architecture

The [architecture index](architecture/README.md) routes to focused notes:

- [Runtime and ownership](architecture/runtime.md)
- [Scheduling and performance](architecture/scheduling.md)
- [Input and interaction](architecture/input.md)
- [Layout and rendering](architecture/rendering.md)
- [macOS composition](architecture/macos.md)
- [Deterministic testing](architecture/testing.md)
- [Current boundaries](architecture/boundaries.md)

Return to the [project README](../README.md).
