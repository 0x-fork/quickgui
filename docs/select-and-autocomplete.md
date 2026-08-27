# Select and autocomplete

[Documentation index](README.md)

QuickGUI has three deliberately separate layers:

- `SelectState<T>` is the standalone unstyled non-editable select. Its caller-owned listbox opens
  in an overflow-capable anchored native child window, so it can extend beyond the owner window.
- `AutocompleteState<T>` is the standalone unstyled free-form input described by
  [Base UI's Autocomplete contract](https://base-ui.com/react/components/autocomplete). Suggestions
  are optional: arbitrary text remains valid, while committing a row may complete the input or
  report an action without changing it.
- `ComboboxState<T>` is the standalone unstyled constrained editable control. Typed text is only
  a query; the committed value is always one declared enabled item. It uses the same never-key
  overflow host and caller-owned input/popup/row contract without conflating that value model with
  Select or free-form Autocomplete.

The application owns committed values, source replacement, and every visual property. Install the
contextual bindings once:

```rust
use quickgui::{App, combobox_key_bindings, select_key_bindings};

App::new(Editor::new())
    .bind_keys(select_key_bindings())
    .bind_keys(combobox_key_bindings());
```

## Standalone unstyled select

Retain `SelectState<T>` in the application view. Give options stable IDs whenever their source can
be reordered or replaced:

```rust
use quickgui::{PickerItem, SelectPopupLayout, SelectState};

struct Settings {
    theme: SelectState<&'static str>,
}

impl Settings {
    fn theme(view: &mut Self) -> &mut SelectState<&'static str> {
        &mut view.theme
    }
}

let mut theme = SelectState::new([
    PickerItem::new("Follow system", "system").id("system"),
    PickerItem::new("Light", "light").id("light"),
    PickerItem::new("Dark", "dark").id("dark"),
])?
.with_layout(SelectPopupLayout::new(280.0, 36.0).max_visible_rows(8));
theme.select_id("system");
```

`element` decorates three caller-owned parts: one trigger, one popup-root factory, and one option
factory. QuickGUI adds behavior and structural geometry only:

```rust
let selected = self.theme
    .selected_item()
    .map(|item| item.label().clone())
    .unwrap_or_else(|| "Choose a theme".into());

self.theme.element(
    cx,
    "theme-select",
    "Editor theme",
    Self::theme,
    quickgui::div().child(selected),                 // application-styled trigger
    |_list| quickgui::div(),                         // application-styled popup root
    |item, state| {                                  // application-styled option
        quickgui::div()
            .opacity(if state.disabled { 0.45 } else { 1.0 })
            .child(item.label().clone())
    },
    |view, value, cx| {
        view.apply_theme(value);
        cx.invalidate();
    },
)
```

The popup is a transparent parent-owned `AnchoredPopover`/`NSPanel` with its own WGPU surface.
Up/Down, page and boundary navigation change only the active preview. Return, Space, or an option
click commits one enabled value. Incremental typeahead cycles matching labels. Escape, an outside
press, or programmatic child closure cancels and restores trigger focus; owner teardown closes the
child first. The runtime's declarative child-close listener clears the exact popup handle even if
a child opens and closes in one event turn; there is no polling or stale `is_open` guess.

`set_items(items, cx)` validates the complete replacement before mutation, preserves a selected
stable ID across reorder, and closes an obsolete open snapshot. A failed replacement changes
nothing. `set_disabled(disabled, cx)` likewise closes an open popup when disabling the control.

## Standalone unstyled free-form autocomplete

Retain `AutocompleteState<T>` beside the caller-owned callbacks. The input, popup root, and every
suggestion row are ordinary application-styled elements:

```rust
use quickgui::{AutocompletePopupLayout, AutocompleteState, PickerItem};

struct Search {
    symbols: AutocompleteState<u64>,
}

impl Search {
    fn symbols(view: &mut Self) -> &mut AutocompleteState<u64> {
        &mut view.symbols
    }
}

let symbols = AutocompleteState::new([
    PickerItem::new("open_file", 1).id("open-file"),
    PickerItem::new("open_project", 2).id("open-project"),
])?
.with_layout(AutocompletePopupLayout::new(360.0, 36.0).max_visible_rows(8));
```

During rendering, pass a `text_input` containing the state's controlled value plus caller-owned
popup and row factories:

```rust
let input = quickgui::text_input(self.symbols.value().clone())
    .h(38.0)
    .border(1.0, border)
    .bg(surface);

self.symbols.element(
    cx,
    "symbol-search",
    "Workspace symbol or path",
    Self::symbols,
    input,
    |_list| quickgui::div(),
    |item, state| {
        quickgui::div()
            .opacity(if state.disabled { 0.45 } else { 1.0 })
            .child(item.label().clone())
    },
    |view, free_form_value, cx| {
        view.query_changed(free_form_value);
        cx.invalidate();
    },
    |view, symbol_id, cx| {
        view.open_symbol(symbol_id);
        cx.invalidate();
    },
)
```

Typing updates the free-form value and bounded result set, then opens one parent-owned native child
surface. That panel can cross the owner edge but is permanently ineligible to become the macOS key
window. Pointer rows remain interactive while text, selection, clipboard, undo, and IME stay in the
owner input. Up/Down and Page Up/Page Down update the active suggestion. Return commits only an
enabled active row; with no active row it propagates to the ordinary input/form path. Escape, Tab,
an owner-window outside press, owner focus loss, or native child closure dismisses without
rewriting arbitrary text.

`AutocompleteSelectionBehavior::CompleteInput` replaces the input with the committed label before
the selection callback. `DismissOnly` reports the suggestion while preserving the exact text.
Neither mode retains a committed selection index: the value remains free-form.

`set_items(items, cx)` validates before mutation and updates an open child through one shared
bounded snapshot; it does not destroy/reopen the native surface. Stable `PickerItem` IDs preserve
row identity across reorder. `PickerFilterMode::None` disables local matching so an application can
publish already-filtered async results while retaining source order. Layout changes close the
fixed-size child before the next open; disabling closes it immediately.

## Standalone unstyled constrained combobox

Retain `ComboboxState<T>` when editing should search declared values but arbitrary text must never
become the value. Initial selection can be set before a runtime context exists:

```rust
use quickgui::{ComboboxPopupLayout, ComboboxState, PickerItem};

let symbols = ComboboxState::new([
    PickerItem::new("open_file", 1).id("open-file"),
    PickerItem::new("open_project", 2).id("open-project"),
])?
.with_layout(ComboboxPopupLayout::new(360.0, 36.0).max_visible_rows(8))
.with_selected_id("open-file");
```

As with Autocomplete, the application supplies every visual part:

```rust
let input = quickgui::text_input(self.symbol.input_value().clone())
    .h(38.0)
    .border(1.0, border)
    .bg(surface);

self.symbol.element(
    cx,
    "workspace-symbol",
    "Workspace symbol",
    Self::symbol,
    input,
    |_list| quickgui::div(),
    |item, state| {
        quickgui::div()
            .opacity(if state.disabled { 0.45 } else { 1.0 })
            .child(item.label().clone())
    },
    |view, query, cx| {
        view.replace_symbol_query(query); // application-owned sync or async work
        cx.invalidate();
    },
    |view, symbol_id, cx| {
        view.open_symbol(symbol_id);
        cx.invalidate();
    },
)
```

Typing updates a bounded query and opens a never-key native child that can cross the owner edge.
Left/Right, Home/End, deletion, selection, clipboard, undo, and IME input remain normal owner text
editor operations; only popup navigation keys are contextual actions. Return or a row click can
commit only an enabled declared option. Return with no active option propagates to the ordinary
input/form path. Escape, Tab, an owner-window outside press, owner focus loss, or native child
closure restores the last committed label; arbitrary edit text is never silently accepted.

The query callback receives an empty string after commit or cancellation. Applications can use
that boundary to cancel or supersede in-flight work and call `set_items` when results arrive.
Source validation is atomic, and an explicit `PickerItem::id` preserves committed identity and
option accessibility IDs across reordering. A stable committed selection remains available even
when an externally filtered source temporarily omits it, then rebinds when that ID returns. A
failed replacement changes neither source nor selection.

## Accessibility and forms

The standalone select trigger projects ComboBox role, expanded state, ListBox popup intent, disabled and
invalid state, and its committed label. The native child root projects ListBox role and a mounted
active descendant; every visible option exposes its logical position, selected state, disabled
state, and stable derived ID. Focus moves to the child listbox while open and returns to the trigger
on every close path.

Autocomplete deliberately uses a different cross-window projection. The visual child subtree is
hidden only from its independent AccessKit window tree; it remains painted, scrollable, and
pointer-interactive. A visually empty bounded ListBox/option proxy lives beside the input in the
owner tree, so `controls` and `active-descendant` never point across native accessibility trees.
The active proxy is always mounted, visible result proxies are capped to the viewport plus
overscan, and disabled/state/position metadata follows the logical result. Constrained Combobox
also mounts a committed option outside that range when necessary, so `selected` remains distinct
from the active keyboard preview without mounting the full collection. The owner input remains the
only keyboard/IME target.

Invalidity and bounded validation copy remain controlled on the focusable trigger:

```rust
self.theme.set_invalid(self.theme.selected_value().is_none());
self.theme.set_validation_message("Choose a theme");
```

An invalid enabled control participates in nearest-form validation, first-invalid focus, bounded
validation reports, and AccessKit announcements. `FormSubmitEvent::fields()` contains controlled
text editors; arbitrary `T` remains application state and should be read through
`selected_value()`. QuickGUI does not implicitly serialize arbitrary Rust values into forms.

## Bounds and sleeping behavior

- A standalone select accepts at most `MAX_PICKER_ITEMS` (65,536) options and shares the existing
  64 KiB per-option and 16 MiB total searchable-text budgets.
- It mounts at most `MAX_SELECT_VISIBLE_ROWS` (64) plus bounded overscan, even for a 20,000-option
  source. Its typeahead prefix retains at most 256 UTF-8 bytes and expires only on the next key.
- Constrained Combobox retains at most `MAX_PICKER_RESULTS` (2,048) ranked matches. Its query is
  capped at 128 grapheme clusters and 4 KiB. The visual child mounts at most 64 rows plus bounded
  overscan; the owner accessibility portal additionally retains only exceptional active and
  committed semantic nodes when they fall outside that range.
- Free-form autocomplete retains at most 64 KiB for its controlled value. Local fuzzy matching
  reads only the bounded 128-grapheme/4 KiB query prefix, retains at most 2,048 results, shares the
  source `Arc` with the child, and mounts at most 64 rows plus bounded overscan. `filter = none`
  still obeys the same result cap.
- An open standalone select owns one child window/surface but shares the application GPU device,
  queue, fonts, assets, and bounded caches. Closing it destroys that child ownership.
- An open autocomplete owns one never-key child, one bounded shared snapshot, and no native event
  monitor. It updates only on value, source, selection, or scroll input. Closed state drops both
  child and snapshot.
- Closed selects, autocompletes, and comboboxes own no task, timer, animation loop, polling
  observer, child renderer, or idle scheduler source.

Run the application-styled unstyled Select and 20,000-option constrained Combobox composition with:

```console
cargo run --release --example comboboxes
cargo run --release --example autocomplete
```
