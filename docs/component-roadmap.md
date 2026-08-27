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

The application owns colors, typography, spacing, borders, radii, icons, shadows, density, and
product motion. Examples may style components, but gallery presentation is not framework API.
Structural geometry required for behavior—such as popup positioning, a scroll viewport, or an
invisible dismissal backdrop—remains framework-owned and independently customizable.

`Popover`, `Checkbox`, `Radio`, `RadioGroup`, and `Switch` decorate caller-owned roots and parts
without adding appearance. `SelectState`, `AutocompleteState`, and `ComboboxState` take
caller-owned controls, popup roots, and option parts. `ContextMenuState` likewise accepts a
caller-owned target, popup root, and rows while retaining only structural geometry and behavior.
`Tabs` decorates caller-owned root/list/tab/indicator/panel parts and leaves indicator geometry and
motion to the application. `PickerState`, `TableState`, and `TreeState` invoke caller-owned
input/header/row/cell renderers and retain only bounded virtual layout geometry. `Field` and
`Fieldset` decorate caller-owned parts
with stable label/description/error relationships, native label activation, controlled validity,
and explicit disabled propagation. `Collapsible` and `Accordion` decorate caller-owned
root/trigger/panel and item/header/trigger/panel parts with controlled open state, exact disclosure
relationships, and optional retained mounting. Do not add compatibility presets, a bundled theme,
or a token system.

## Dependency order

The component layer is implemented in this order:

1. shared parts: controlled root state, caller-owned trigger/surface decorators, window-bounded
   portal, overflow-capable anchored native host, backdrop, positioner, popup, focus scope,
   dismissal layer, composite navigation, collection metadata, and form-field metadata;
2. `PopupMenu` and `ContextMenu` on the overflow-capable host, including items, groups, separators,
   checkbox/radio items, submenus, typeahead, and desktop menu keyboard behavior;
3. standalone `Select`, then free-form `Autocomplete` and constrained `Combobox`, sharing listbox
   collection and virtualization without conflating text and selected values or accessibility;
4. `Dialog` and `AlertDialog`, followed by `Drawer` and preview-card composition;
5. field, navigation, disclosure, range, and feedback components required by real applications;
6. advanced variants only after deterministic tests, live macOS accessibility acceptance, and
   sleeping CPU/memory gates are recorded.

`Menu` currently names QuickGUI's native application-menu model. The in-window component uses
`PopupMenu` so native and GPU APIs remain unambiguous. `SelectState` is the non-editable
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
| Autocomplete | Behavior present | `AutocompleteState` accepts caller-owned input/popup/option parts, preserves arbitrary bounded text, supports optional completion or action-only commits, async/filter-none source replacement, visible-only results, a never-key overflow child, owner-IME focus, and an owner-tree accessibility proxy. Complete live IME, VoiceOver, edge placement, and repeated-open resource acceptance. |
| Avatar | Primitive only | Compose image, fallback, accessible name, and load state without visual defaults. |
| Button | Behavior present | Preserve the semantic `button()` root; keep presentation application-owned. |
| Checkbox | Behavior present | `Checkbox` supplies an unstyled caller-owned root plus accessibility-hidden indicator part with exact checked/mixed semantics. Complete live VoiceOver and application-styled disabled/focus acceptance. |
| Checkbox Group | Primitive only | Add group labeling, validation, controlled values, and keyboard/accessibility coverage. |
| Collapsible | Behavior present | `Collapsible` supplies caller-owned root/trigger/panel parts, controlled open/disabled state, exact mounted-panel relationships, default unmounting or explicit `display: none` retention, ordinary button keyboard behavior, and zero idle source. Complete live VoiceOver and application-styled open/closed acceptance. |
| Combobox | Behavior present | `ComboboxState` accepts caller-owned input/popup/option parts, constrains commits to enabled declared items, restores committed labels on every dismissal, retains stable selections across externally filtered absence, and uses the never-key overflow child plus owner-tree accessibility proxy. Complete live IME, VoiceOver, edge placement, and repeated-open resource acceptance. |
| Context Menu | Behavior present | `ContextMenuState` composes secondary-click point anchoring, a separate overflow-capable child surface, exact close synchronization, delayed hover, an actual-placement safe pointer corridor for right/left submenus, nested `PopupMenu` models, caller-owned parts, and typed owner actions. The automated gate covers 128 bounded repeated native lifecycles; complete live VoiceOver, human-visible appearance, nested placement, and mixed-scale acceptance. |
| Dialog | Behavior present | `Dialog` supplies caller-owned portal/backdrop/popup/title/description/close parts, nested focus containment, independent dismissal, restoration, and exact modal semantics. Complete live VoiceOver/backdrop/native-view acceptance; keep native dialog-window roles separate. |
| Drawer | Missing | Build after dialog focus and backdrop behavior is live accepted. |
| Field | Behavior present | `Field` supplies caller-owned root/label/control/description/error parts, stable relationships, normal or passive labels, required/invalid/disabled state, bounded validation copy, and controlled touched/dirty/filled render state. Complete live VoiceOver label activation, error wording/order, and application-styled state acceptance. |
| Fieldset | Behavior present | `Fieldset` supplies caller-owned group/legend/description parts and explicit registry-free disabled propagation through `.field(...)` or `.control_part(...)`. Complete live VoiceOver grouping and disabled-control acceptance. |
| Form | Behavior present | Preserve controlled fields, nearest-form submission, and first-invalid focus. |
| Input | Behavior present | Keep editing/IME behavior reusable while removing visual defaults from the component contract. |
| Menu | Behavior present | Native `Menu` remains the app menu; unstyled `PopupMenu` provides bounded items, toggles, exact group-label/separator semantics, keyboard/typeahead behavior, owner-window actions, submenu composition, and an owner-controlled hover hook used by the native context-menu aim policy. Complete live VoiceOver acceptance. |
| Menubar | Platform only | Native macOS menubar exists; add an in-window menubar only for a demonstrated product need. |
| Meter | Missing | Add semantic bounded-value behavior independently of presentation. |
| Navigation Menu | Missing | Defer until menu, tabs, and disclosure contracts are stable. |
| Number Field | Missing | Build on input constraints with locale-aware parsing and step behavior. |
| OTP Field | Missing | Build on grouped controlled inputs, paste distribution, and accessible labeling. |
| Popover | Behavior present | `Popover` supplies caller-owned trigger, combined portal/positioner, popup, backdrop, title, description, and close parts; bounded placement, independent dismissal, initial focus, relationships, and merged-surface shorthand add no appearance or idle source. Complete live VoiceOver, pointer/focus, nesting, edge-placement, and repeated-open resource acceptance. A flip-aware arrow and multi-trigger animated viewport remain separate future capabilities rather than guessed presentation. |
| Preview Card | Primitive only | Compose delayed pointer/focus opening from popover/tooltip foundations. |
| Progress | Missing | Add determinate/indeterminate semantics without framework-owned animation or styling. |
| Radio | Behavior present | `RadioGroup` and `Radio` supply caller-owned group/item/indicator parts with checked-entry Tab behavior, wrapping arrow activation, disabled-item skipping, and no retained item registry. Complete live VoiceOver and application-styled focus acceptance. |
| Scroll Area | Primitive only | Retained scrolling and native-style overlay scrollbars exist; expose unstyled viewport/scrollbar parts only if product styling requires them. |
| Select | Behavior present | Standalone `SelectState` supplies caller-owned trigger/popup/option parts on the overflow-capable native host, exact lifecycle sync, typeahead, disabled options, and visible-only rows. Complete live VoiceOver, mixed-scale, pointer/keyboard/scroll, and repeated-open resource acceptance. |
| Separator | Primitive only | `AccessibilityRole::Separator` projects a non-interactive native divider; add an unstyled orientation part only when a product needs the standalone component. Keep adjustable splitters a separate behavior. |
| Slider | Missing | Add captured pointer/keyboard range behavior, steps, orientation, and value semantics. |
| Switch | Behavior present | `Switch` supplies an unstyled caller-owned root/track plus accessibility-hidden thumb part and exact toggle semantics. Complete live VoiceOver and application-styled disabled/focus acceptance. |
| Tabs | Behavior present | `Tabs` composes caller-owned root/list/tab/indicator/panel parts with controlled selection, horizontal or vertical roving focus, manual or automatic activation, optional looping, disabled-item skipping, default unmounting or explicit `display: none` retention, exact TabList/Tab/TabPanel semantics and relationships, and zero idle source. Indicator geometry/motion remains caller-owned. Complete live VoiceOver, application-styled focus/disabled, and pointer/keyboard acceptance; native document tabs remain a separate optional platform API. |
| Toast | Platform only | System notifications exist; add a bounded in-window live-region queue only when required. |
| Toggle | Primitive only | Add pressed-state button semantics distinct from checkbox state. |
| Toggle Group | Missing | Build after toggle and composite focus behavior. |
| Toolbar | Missing | Build on roving focus, button/toggle groups, and menu triggers. |
| Tooltip | Behavior present | Keep delayed positioning/accessibility behavior; presentation remains caller content. |

This ledger is not a promise to implement every web component before 0.1. Release priority still
follows concrete editor-class workflows, macOS acceptance risk, and measured resource ownership.
The catalog prevents accidental omissions and misleading broad parity claims.
