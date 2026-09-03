# Unstyled component roadmap

[Documentation index](README.md) · [Status and roadmap](status.md)

QuickGUI uses the [Base UI component catalog](https://base-ui.com/react/overview/quick-start) as
the inventory for common web-style application controls. It is a behavior and API reference, not
a visual design system or Rust dependency.

## Contract

QuickGUI ships only unstyled components. A component may own:

- semantic roles, relationships, state, and actions;
- keyboard navigation, typeahead, focus movement or trapping, and focus restoration;
- controlled open/value/selection state and bounded internal interaction state;
- portal placement, collision handling, dismissal, and pointer occlusion;
- deterministic accessibility projection, tests, and sleeping-resource ownership.

Every component that retains interaction state exposes two entry points to it. The original
`fn(&mut V) -> &mut State` methods stay exactly as they were, for an application that owns one
control per field. Alongside each of them is a `*_with` method taking a
[`StateAccessor<V, State>`](view-api.md) — one reference-counted closure that may capture which
instance it addresses. A host that renders many declared controls through a single view, a repeated
row, or a data-driven collection uses that form. `StateAccessor` is a single erased type rather than
a generic parameter, so adding it multiplies neither the component code nor the listener set: each
component registers exactly the listeners it always did, per mounted instance, and adds no idle
source. `NumberField`, `Progress`, `Meter`, `Toggle`, and the other pure decorators register no
listeners at all and therefore need no accessor.

The application owns colors, typography, spacing, borders, radii, icons, shadows, density, and
product motion. Examples may style components, but gallery presentation is not framework API.
Structural geometry required for behavior—such as popover positioning, a scroll viewport, or an
invisible dismissal backdrop—remains framework-owned and independently customizable.

`Popover`, `Checkbox`, `Radio`, `RadioGroup`, and `Switch` decorate caller-owned roots and parts
without adding appearance. Presentation that has to follow a placement QuickGUI resolved rather than
one the application declared — a popover arrow, a directional transform origin, a popup sized to the
room it was given — reads `AnchorPlacementHandle`, which the retained tree writes during the paint
it was already performing and which requests exactly one correcting frame when the resolved
placement changes. `SelectState`, `AutocompleteState`, and `ComboboxState` take
caller-owned controls, popover roots, and option parts. `ContextMenuState` likewise accepts a
caller-owned target, popover root, and rows while retaining only structural geometry and behavior.
`Tabs` decorates caller-owned root/list/tab/indicator/panel parts and leaves indicator geometry and
motion to the application. `PickerState`, `TableState`, and `TreeState` invoke caller-owned
input/header/row/cell renderers and retain only bounded virtual layout geometry; the table hands its
header renderer a behavior-only column-resize handle and its cell renderer an editing flag, and the
tree hands its row renderer a behavior-only disclosure and a loading-placeholder flag, so multiple
selection, column resizing and reordering, inline editing, and lazy children add no presentation.
`DateFieldState` and `TimeFieldState` decorate caller-owned root and segment parts with civil-date
validation, typed entry, and spin-button semantics. `Field` and
`Fieldset` decorate caller-owned parts
with stable label/description/error relationships, native label activation, controlled validity,
and explicit disabled propagation. `FindBar` decorates caller-owned find, replace, count, navigation, and dismissal parts while
`FindState` stays a pure application-owned value. `Collapsible` and `Accordion` decorate caller-owned
root/trigger/panel and item/header/trigger/panel parts with controlled open state, exact disclosure
relationships, and optional retained mounting. `Slider`, `Splitter`, `NumberField`, `Progress`,
`Meter`, `Toolbar`, `Toggle`, `ToggleGroup`, and `ToastViewport` decorate caller-owned parts while
QuickGUI keeps only the numeric, ordering, roving-focus, and dismissal contracts those behaviors
require; indeterminate progress motion, toast placement, and splitter handle appearance stay
application-owned. Do not add compatibility presets, a bundled theme,
or a token system.

## Dependency order

The component layer is implemented in this order:

1. shared parts: controlled root state, caller-owned trigger/surface decorators, window-bounded
   portal, overflow-capable `SystemPopover` host, backdrop, positioner, popover, focus scope,
   dismissal layer, composite navigation, collection metadata, and form-field metadata;
2. `PopoverMenu` and `ContextMenu` on the overflow-capable host, including items, groups, separators,
   checkbox/radio items, submenus, typeahead, and desktop menu keyboard behavior;
3. standalone `Select`, then free-form `Autocomplete` and constrained `Combobox`, sharing listbox
   collection and virtualization without conflating text and selected values or accessibility;
4. `Dialog` and `AlertDialog`, followed by `Drawer` and preview-card composition;
5. field, navigation, disclosure, range, and feedback components required by real applications;
6. advanced variants only after deterministic tests, live macOS accessibility acceptance, and
   sleeping CPU/memory gates are recorded.

`Menu` currently names QuickGUI's native application-menu model. The in-window component uses
`PopoverMenu` so native and GPU APIs remain unambiguous. `SelectState` is the non-editable
declared-value contract, `AutocompleteState` is the editable
free-form contract, and `ComboboxState` is the editable declared-value contract. Their names and
value semantics are no longer compatibility modes of one styled control.

## Catalog ledger

Status terms here are intentionally strict:

- **Behavior present**: reusable behavior exists, but the final unstyled parts API may still be
  incomplete.
- **Primitive only**: lower-level framework capability exists; applications still assemble the
  component themselves.
- **Platform only**: a native macOS counterpart exists, not an in-window GPU component.
- **Missing**: no reusable component contract exists yet.

| Base UI reference | QuickGUI today | Required component-layer work |
| --- | --- | --- |
| Accordion | Behavior present | `Accordion` composes caller-owned item/header/trigger/region parts with exact expanded/control/name relationships, normal trigger Tab order from current APG guidance, inherited disabled/keep-mounted policy, and a 4,096-open-value bounded single/multiple `AccordionState`. Complete live VoiceOver, application-styled focus/disabled, and retained-content acceptance. |
| Alert Dialog | Behavior present | `Dialog::alert` supplies caller-owned modal parts, topmost focus containment, Escape-only default dismissal, restoration, and exact alert-dialog semantics. Complete live VoiceOver/backdrop/native-view acceptance; keep native `NSAlert` for system presentation. |
| Autocomplete | Behavior present | `AutocompleteState` accepts caller-owned input/popover/option parts, preserves arbitrary bounded text, supports optional completion or action-only commits, async/filter-none source replacement, visible-only results, a never-key overflow child, owner-IME focus, and an owner-tree accessibility proxy. Complete live IME, VoiceOver, edge placement, and repeated-open resource acceptance. |
| Avatar | Primitive only | Compose image, fallback, accessible name, and load state without visual defaults. |
| Button | Behavior present | Preserve the semantic `button()` root; keep presentation application-owned. |
| Calendar | Behavior present | `CalendarState` supplies a bounded six-week month grid with roving day focus, arrow/Home/End/Page navigation that follows focus across month boundaries, declared week-start weekday, selection bounds that refuse selection rather than movement, and caller-owned grid/week/day parts with exact Grid/Row/GridCell semantics. Complete live VoiceOver and application-styled selection acceptance. |
| Checkbox | Behavior present | `Checkbox` supplies an unstyled caller-owned root plus accessibility-hidden indicator part with exact checked/mixed semantics, a `read_only` contract that stays focusable and refuses the transition through `next_state`, and a registry-free `parent` derivation over the children the application already renders. Complete live VoiceOver and application-styled disabled/focus acceptance. |
| Checkbox Group | Primitive only | Add group labeling, validation, controlled values, and keyboard/accessibility coverage. |
| Collapsible | Behavior present | `Collapsible` supplies caller-owned root/trigger/panel parts, controlled open/disabled state, exact mounted-panel relationships, default unmounting or explicit `display: none` retention, ordinary button keyboard behavior, and zero idle source. Complete live VoiceOver and application-styled open/closed acceptance. |
| Combobox | Behavior present | `ComboboxState` accepts caller-owned input/popover/option parts, constrains commits to enabled declared items, restores committed labels on every dismissal, retains stable selections across externally filtered absence, and uses the never-key overflow child plus owner-tree accessibility proxy. Complete live IME, VoiceOver, edge placement, and repeated-open resource acceptance. |
| Context Menu | Behavior present | `ContextMenuState` composes secondary-click point anchoring, a separate overflow-capable child surface, exact close synchronization, delayed hover, an actual-placement safe pointer corridor for right/left submenus, nested `PopoverMenu` models, caller-owned parts, and typed owner actions. The automated gate covers 128 bounded repeated native lifecycles; complete live VoiceOver, human-visible appearance, nested placement, and mixed-scale acceptance. |
| Date Field | Behavior present | `DateFieldState` supplies segmented `CivilDate` editing with leap-year validation, configurable YMD/DMY/MDY order, typed-digit entry with automatic advance, wrapping arrow steps, per-segment placeholders, min/max validity, caller-owned root/segment parts, and one spin button per segment inside a group root. Complete live VoiceOver segment announcement and application-styled focus/invalid acceptance. |
| Time Field | Behavior present | `TimeFieldState` projects one retained 24-hour `CivilTime` through 12- or 24-hour hour/minute/second/AM-PM segments sharing the date field's typed actions, entry, and spin-button semantics. Complete live VoiceOver and application-styled acceptance. |
| Dialog | Behavior present | `Dialog` supplies caller-owned portal/backdrop/viewport/popover/title/description/close parts, nested focus containment, independent dismissal, restoration, and exact modal semantics; `DialogState` keeps a dialog mounted for the application's own exit transition on an exact one-shot deadline and reports the completed direction, which is Base UI's `onOpenChangeComplete` without QuickGUI owning any animation. Complete live VoiceOver/backdrop/native-view acceptance; keep native dialog-window roles separate. |
| Drawer | Missing | Build after dialog focus and backdrop behavior is live accepted. |
| Field | Behavior present | `Field` supplies caller-owned root/item/label/control/description/error/validity parts, stable relationships, normal or passive labels, required/invalid/disabled state, bounded validation copy, controlled touched/dirty/filled render state, and a `validation_mode` with a bounded `validation_debounce` the application sleeps on exactly once. Complete live VoiceOver label activation, error wording/order, and application-styled state acceptance. |
| Fieldset | Behavior present | `Fieldset` supplies caller-owned group/legend/description parts and explicit registry-free disabled propagation through `.field(...)` or `.control_part(...)`. Complete live VoiceOver grouping and disabled-control acceptance. |
| Find Bar (QuickGUI addition) | Behavior present | `FindBar` decorates caller-owned root/query-input/replace-input/count/next/previous/replace/replace-all/close parts with stable identities, group and button roles, an accessible match count, state-derived disabling, the `FindBar` key context, and Return/Shift+Return/Escape bindings. Bounded literal `FindState` matching, wrap-around navigation, and single-edit replace-all remain application-applied. Complete live VoiceOver and application-styled acceptance. |
| Form | Behavior present | Preserve controlled fields, nearest-form submission, and first-invalid focus. |
| Input | Behavior present | Keep editing/IME behavior reusable while removing visual defaults from the component contract. |
| Menu | Behavior present | Native `Menu` remains the app menu; unstyled `PopoverMenu` provides bounded items, toggles, exact group-label/separator semantics, keyboard/typeahead behavior, owner-window actions, submenu composition, and an owner-controlled hover hook used by the native context-menu aim policy. Complete live VoiceOver acceptance. |
| Menubar | Behavior present | `MenubarState` supplies caller-owned root and trigger parts, a single roving Tab stop, wrapping arrow navigation that keeps switching while a menu is open, open-on-click, closed-bar-safe hover switching, Escape closing without leaving the bar, and exact menubar/menu-item semantics over ordinary `PopoverMenu` surfaces. Complete live VoiceOver and application-styled open/focus acceptance; native `Menu` remains the platform application menu. |
| Meter | Behavior present | `Meter` decorates caller-owned root, track, indicator, label, and value parts with the Meter role, a clamped numeric value and range, optional low/high/optimum markers, and the same `ValueFormat` and `value_text` contract as `Progress`. Complete live VoiceOver acceptance. |
| Navigation Menu | Missing | Defer until menu, tabs, and disclosure contracts are stable. |
| Number Field | Behavior present | `NumberFieldState` composes the existing `text_input()` with caller-supplied decimal/group separators, sign and exponent policy, precision, range clamping on commit, Shift/Alt large and small steps, `snap_on_step`, opt-out `allow_wheel_scrub`, `read_only` and `required` policy, captured scrub-area dragging that converts pointer travel into whole steps with a retained remainder, and press-and-hold repeat on exact one-shot deadlines. `NumberField` supplies root/group/scrub-area/scrub-cursor/decrement/input/increment parts with SpinButton semantics, invalid, read-only, and required state, and a copyable `NumberFieldPartState`. Complete live VoiceOver and IME acceptance. |
| OTP Field | Missing | Build on grouped controlled inputs, paste distribution, and accessible labeling. |
| Popover | Behavior present | `Popover` supplies caller-owned trigger, combined portal/positioner, popup, backdrop, arrow, viewport, title, description, and close parts with Base UI-shaped `side`/`align`/`side_offset`/`align_offset`/`collision_padding`/`sticky`/`anchor`/`modal` props, a copyable `PopoverPartState` carrying open/side/align/anchor-hidden state and measured anchor and available sizes, and `PopoverHoverState` hover opening on exact `delay`/`close_delay` deadlines. The arrow follows the placement QuickGUI actually resolved, published through `AnchorPlacementHandle` during the paint already being performed and corrected in exactly one frame, never a preferred-side guess. Complete live VoiceOver, pointer/focus, nesting, edge-placement, and repeated-open resource acceptance; a multi-trigger animated viewport remains a separate future capability. |
| Preview Card | Primitive only | Compose delayed pointer/focus opening from popover/tooltip foundations. |
| Progress | Behavior present | `Progress` supplies determinate and indeterminate roots plus caller-owned track, indicator, label, and value parts with exact numeric value and bounds, `ProgressStatus::{Progressing, Complete, Indeterminate}`, a copyable `ProgressPartState`, a bounded `ValueFormat` hook, and `value_text` taking `getAriaValueText` precedence over it. Declaring an identity relates the root to its mounted label and value parts. Indeterminate motion stays application-owned so no framework animation keeps a settled window awake. Complete live VoiceOver acceptance. |
| Radio | Behavior present | `RadioGroup` and `Radio` supply caller-owned group/item/indicator parts with checked-entry Tab behavior, wrapping arrow activation, disabled-item skipping, group `read_only`/`required` state, an `accepts_selection` contract, and no retained item registry. Complete live VoiceOver and application-styled focus acceptance. |
| Scroll Area | Primitive only | Retained scrolling and native-style overlay scrollbars exist; expose unstyled viewport/scrollbar parts only if product styling requires them. |
| Select | Behavior present | Standalone `SelectState` supplies caller-owned trigger/popover/option parts on the overflow-capable native host, exact lifecycle sync, typeahead, disabled options, and visible-only rows. Complete live VoiceOver, mixed-scale, pointer/keyboard/scroll, and repeated-open resource acceptance. |
| Separator | Primitive only | `AccessibilityRole::Separator` projects a non-interactive native divider; add an unstyled orientation part only when a product needs the standalone component. Adjustable splitters are a separate implemented behavior: `SplitterState` conserves pane sizes across captured drags and typed keyboard resizing, and `Splitter` supplies caller-owned root/pane/handle parts with focusable Splitter handles carrying numeric value, bounds, axis, and a controls relationship. |
| Slider | Behavior present | `SliderState` owns bounds, step snapping, thumb ordering, `min_steps_between_values` gaps, `thumb_alignment` offsets, the dragging flag, and the active thumb for up to `MAX_SLIDER_THUMBS` (8) values; `Slider` supplies caller-owned root/label/value/control/track/indicator/thumb parts, captured pointer arithmetic in either orientation with an exposed `onValueCommitted` boundary, typed arrow/page/Home/End actions, a `ValueFormat` hook for the value and per-thumb value text, a copyable `SliderThumbState` carrying index and dragging state, and Slider roles with numeric value/min/max/step. Complete live VoiceOver and pointer acceptance. |
| Switch | Behavior present | `Switch` supplies an unstyled caller-owned root/track plus accessibility-hidden thumb part, exact toggle semantics, and a `read_only` contract that stays focusable while `next_checked` refuses the toggle. Complete live VoiceOver and application-styled disabled/focus acceptance. |
| Tabs | Behavior present | `Tabs` composes caller-owned root/list/tab/indicator/panel parts with controlled selection, horizontal or vertical roving focus, manual or automatic activation, optional looping, disabled-item skipping, default unmounting or explicit `display: none` retention, exact TabList/Tab/TabPanel semantics and relationships, `TabsState::select_at` activation direction, an anchored indicator QuickGUI keeps on the tab that is really active, and a `TabsIndicatorGeometry` snapshot of that tab's laid-out box published through `AnchorPlacementHandle`. Indicator appearance and motion remain caller-owned and the tab set adds zero idle source. Complete live VoiceOver, application-styled focus/disabled, and pointer/keyboard acceptance; native document tabs remain a separate optional platform API. |
| Toast | Behavior present | `ToastManager` is the provider: a bounded queue with exact one-shot auto-dismiss deadlines, an inherited `timeout`, a visible `limit` that flags older toasts `limited` without silencing them, an expanded-stack flag, `add`/`update`/`close`/`close_all` and `promise`/`resolve` helpers the application drives from its own foreground task, captured swipe-to-dismiss with an exposed movement, pause on hover or focus, and no idle source once drained. `ToastViewport` supplies caller-owned portal/viewport/positioner/root/content/title/description/action/close parts with polite or assertive live regions by kind, per-toast index and stacking offset, and focused Escape dismissal. Native system notifications remain a separate platform API. |
| Toggle | Behavior present | `Toggle` decorates a caller-owned root with pressed-button semantics distinct from a checkbox's checked state, plus an accessibility-hidden indicator part. Complete live VoiceOver acceptance. |
| Toggle Group | Behavior present | `ToggleGroup` composes caller-owned group and item parts with single or multiple selection, inline bounded pressed values, one roving Tab stop, bounded arrow/Home/End navigation, and disabled-item skipping. Complete live VoiceOver acceptance. |
| Toolbar | Behavior present | `Toolbar` decorates a caller-owned root and ordered items with the Toolbar role and orientation, one roving Tab stop, per-item arrow/Home/End actions on the focused item, disabled-item skipping, optional looping, Button/Link/Input item parts, Group and Separator parts that create no second focus scope, and `focusable_when_disabled` items that stay keyboard-reachable while unavailable. Menu-trigger composition inside a toolbar remains application-assembled. |
| Tooltip | Behavior present | The framework-owned `.tooltip(...)` overlay keeps its exact delayed positioning and accessibility behavior. `TooltipProvider` and `TooltipState` add the Base UI-shaped compound parts — Provider, Root, Trigger, Portal/Positioner, Popup, Arrow — with shared `delay`/`close_delay`/`timeout` grouping so an adjacent trigger opens instantly, `hoverable`, `close_on_click`, `disabled`, `track_cursor_axis`, resolved-placement arrows, and framework-owned Escape dismissal. Every delay is an exact one-shot deadline and a settled group owns nothing. Keyboard-focus opening still needs a focus-listener primitive; complete live VoiceOver acceptance. |

This ledger is not a promise to implement every web component before 0.1. Release priority still
follows concrete editor-class workflows, macOS acceptance risk, and measured resource ownership.
The catalog prevents accidental omissions and misleading broad parity claims.
