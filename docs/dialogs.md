# Dialogs

[Documentation index](README.md) · [Unstyled component roadmap](component-roadmap.md)

QuickGUI provides one controlled, unstyled in-window contract for ordinary dialogs and
consequential alert dialogs. Native AppKit sheets and `NSAlert` remain separate platform APIs for
workflows that should use system presentation.

`Dialog` owns behavior and structural geometry only:

- a full-window overlay portal above WGPU content and embedded `NSView` children;
- nested topmost focus containment and wrapping Tab traversal;
- independent Escape and outside-backdrop dismissal policies;
- initial focus and restoration to a stable application control;
- exact Dialog or AlertDialog role, modal state, and mounted title/description relationships;
- explicit `app-region: no-drag` behavior for hidden-inset windows; and
- no theme, appearance tokens, task, timer, observer, registry, or idle scheduler source.

The application owns the controlled `open` value, colors, typography, spacing, borders, radii,
shadows, sizing, motion, and the callbacks that change its state.

## Composition

Declare a copyable descriptor from one stable base identity. Every part receives a deterministic
derived ID, so callers do not need to coordinate an internal ID namespace:

```rust
use quickgui::{Dialog, button, div, text};

let dialog = Dialog::new("settings-dialog", self.settings_open)
    .initial_focus("account-name")
    .restore_focus_to("open-settings");

let open = cx.listener("open-settings", move |view, cx| {
    view.settings_open = true;
    dialog.focus_initial(cx);
    cx.invalidate();
});
let dismiss = cx.dismiss_listener(dialog.popup_id(), |view, cx| {
    view.settings_open = false;
    cx.invalidate();
});

let trigger = dialog
    .trigger_part("open-settings", button().child("Settings"))
    .on_click(open);

if self.settings_open {
    let popup = dialog
        .popup_part(
            div()
                .child(dialog.title_part(text("Settings")))
                .child(dialog.description_part(text("Edit account settings")))
                .child(button().id("account-name").child("First action")),
        )
        .on_dismiss(dismiss);

    let portal = dialog
        .root_part(div().flex_row().items_center().justify_center())
        .child(dialog.backdrop_part(div()))
        .child(popup);
}
```

Mount `root_part` only while the controlled value is open. `root_part` fills the viewport but does
not choose popup alignment; Flexbox, Grid, or absolute positioning on the caller root remains
application presentation. The backdrop and popup likewise receive no authored color or size.

`close_part(label, element)` decorates a caller element with a stable close-control ID and button
semantics. Its listener still belongs to the application. Explicit close callbacks call
`focus_restore(cx)` after changing the controlled state. Escape and backdrop dismissal use the same
retained restore target automatically.

## Focus and nesting

`.focus_trap()` is a general retained element primitive. Mounted traps are ordered by render plane,
effective `z_index`, and source order; only the topmost trap contributes framework focusability or
Tab stops. Mounting a trap focuses its first enabled Tab stop, falling back to the popup root. If a
focused child disappears, focus moves to the first remaining stop instead of escaping the modal.

Programmatic QuickGUI focus, pointer focus, Tab, Shift-Tab, and accessibility focus actions use the
same filtered focus index. Closing a nested dialog reveals the next trap and restores the supplied
control in that layer. No global focus registry or per-frame traversal is retained; the index is
rebuilt only with the ordinary declarative tree.

## Dismissal policy

An ordinary `Dialog::new` dismisses on Escape and a press outside its popup. `Dialog::alert`
dismisses on Escape but blocks backdrop presses by default, preventing accidental confirmation
loss. Both policies are configurable:

```rust
let persistent = Dialog::alert("publish", self.open)
    .dismiss_on_escape(false)
    .dismiss_on_backdrop(false);
```

The topmost dismissal boundary never falls through to a lower dialog. Pointer blocking remains in
the overlay root even when backdrop dismissal is disabled.

## Accessibility and native composition

Ordinary and alert popups project distinct AccessKit `Dialog` and `AlertDialog` roles, explicit
modal state, and `labelled-by`/`described-by` relationships to the mounted visible parts. Dangling
or self-referential targets are omitted. Accessibility relationships use one nullable pointer on
ordinary elements and allocate one fixed record only for elements that declare relations.

The portal paints in QuickGUI's overlay plane. On macOS, mounting it activates the transparent
overlay WGPU surface above embedded AppKit children, so the backdrop both paints over and blocks
input to an `NSView`; removing the dialog returns to the sleeping native composition path.

## Verification and resources

Deterministic tests cover unstyled parts, stable identities, nested z-ordered traps, wrapping Tab
focus, rejected outside focus, independent dismissal policies, restoration, exact AccessKit
projection, and zero extra renders after settling. Run the styled ordinary/nested-alert gallery:

```console
cargo run --release --example dialogs
```

Live VoiceOver wording, pointer backdrop behavior, embedded-native-view occlusion, and mixed-scale
multi-monitor behavior remain release-candidate acceptance items until recorded on the intended
macOS build.
