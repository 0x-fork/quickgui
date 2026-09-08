# Dialogs

[Documentation index](README.md) · [Unstyled component roadmap](component-roadmap.md)

QuickGUI provides one controlled, unstyled in-window contract for ordinary dialogs and
consequential alert dialogs. Native AppKit sheets and `NSAlert` remain separate platform APIs for
workflows that should use system presentation; they live on `EventContext` as `prompt`,
`message_box`, `prompt_for_paths`, and `prompt_for_new_path`, and their options, per-OS support
table, and bounds are documented in
[Desktop integrations](desktop-integrations.md#message-boxes-and-file-panels).

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
let dismiss = cx.dismiss_listener(dialog.popover_id(), |view, cx| {
    view.settings_open = false;
    cx.invalidate();
});

let trigger = dialog
    .trigger_part("open-settings", button().child("Settings"))
    .on_click(open);

if self.settings_open {
    let popover = dialog
        .popover_part(
            div()
                .child(dialog.title_part(text("Settings")))
                .child(dialog.description_part(text("Edit account settings")))
                .child(button().id("account-name").child("First action")),
        )
        .on_dismiss(dismiss);

    let portal = dialog
        .root_part(div().flex_row().items_center().justify_center())
        .child(dialog.backdrop_part(div()))
        .child(popover);
}
```

Mount `root_part` only while the controlled value is open. `root_part` fills the viewport but does
not choose popover alignment; Flexbox, Grid, or absolute positioning on the caller root remains
application presentation. The backdrop and popover likewise receive no authored color or size.

`close_part(label, element)` decorates a caller element with a stable close-control ID and button
semantics. Its listener still belongs to the application. Explicit close callbacks call
`focus_restore(cx)` after changing the controlled state. Escape and backdrop dismissal use the same
retained restore target automatically.

## Focus and nesting

`.focus_trap()` is a general retained element primitive. Mounted traps are ordered by render plane,
effective `z_index`, and source order; only the topmost trap contributes framework focusability or
Tab stops. Mounting a trap focuses its first enabled Tab stop, falling back to the popover root. If a
focused child disappears, focus moves to the first remaining stop instead of escaping the modal.

Programmatic QuickGUI focus, pointer focus, Tab, Shift-Tab, and accessibility focus actions use the
same filtered focus index. Closing a nested dialog reveals the next trap and restores the supplied
control in that layer. No global focus registry or per-frame traversal is retained; the index is
rebuilt only with the ordinary declarative tree.

## Dismissal policy

An ordinary `Dialog::new` dismisses on Escape and a press outside its popover. `Dialog::alert`
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

Ordinary and alert popovers project distinct AccessKit `Dialog` and `AlertDialog` roles, explicit
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

## Go components

`ui.Dialog` and `ui.AlertDialog` expose `Root`, `Trigger`, `Portal`, `Backdrop`,
`Popup`, `Viewport`, `Title`, `Description`, and `Close` parts. A root coordinates
controlled state without creating a visible element; its portal supplies the
viewport overlay root. Give the popup its own size, colors, and spacing.

```go
func DeleteProject() {
	open, setOpen := ui.CreateSignal(false)
	ui.AlertDialog.Root(
		ui.DialogRootProps{
			Open:         open,
			OnOpenChange: func(value bool, _ ui.DialogOpenChangeDetails) { setOpen(value) },
			ExitDuration: 160,
			OnOpenChangeComplete: func(open bool, _ *native.Event) {
				log.Print("Dialog transition finished; open: ", open)
			},
		},
		func() {
			ui.AlertDialog.Trigger(ui.PartProps{}, "Delete project")
			ui.AlertDialog.Portal(
				ui.PartProps{},
				func() {
					ui.AlertDialog.Backdrop(ui.PartProps{Style: ui.Styles(
						ui.BackgroundColor("#0f172a80"),
					)})
					ui.AlertDialog.Popup(
						ui.DialogPopupProps{PartProps: ui.PartProps{
							Style: ui.Styles(
								ui.Width(360),
								ui.Padding(24),
								ui.BackgroundColor("white"),
							),
						}},
						func() {
							ui.AlertDialog.Title(ui.PartProps{}, "Delete project?")
							ui.AlertDialog.Viewport(
								ui.PartProps{},
								func() {
									ui.AlertDialog.Description(
										ui.PartProps{},
										"This cannot be undone.",
									)
								},
							)
							ui.AlertDialog.Close(ui.PartProps{AriaLabel: "Cancel"}, "Cancel")
						},
					)
				},
			)
		},
	)
}
```

`Viewport` supplies a scrollable region. `EnterDuration` and `ExitDuration` declare
the transition deadlines in milliseconds; `OnOpenChangeComplete` reports the
finished state. The core retains a closing dialog through its exit deadline so an
application's animation can finish. With the default zero duration, completion
and unmounting occur in the same update. External changes to `Open` also produce
completion events.

`DismissOnEscape` and `DismissOnBackdrop` are optional boolean pointers declared
ahead of input. Ordinary dialogs allow both; alert dialogs allow Escape and reject
backdrop dismissal by default. The native event calls `OnOpenChange` with a reason
of `ui.DialogDismiss`, `ui.DialogTriggerPress`, or `ui.DialogClosePress`. The core
owns overlay ordering, nested topmost focus containment, restoration, modal
accessibility, and stable identities.

The Go dialog parts currently use the trap's first enabled Tab stop and the core's
previous-focus restoration. They do not expose the Rust descriptor's explicit
initial/restore focus targets. Native alert and file panels use the separate
`native.ShowAlertDialog`, `native.ShowOpenDialog`, and `native.ShowSaveDialog`
functions; see [Go platform services](go.md#native-services).

## Viewport and transition completion

`Dialog::viewport_part(element)` is Base UI's Viewport: the scrolling region between the backdrop
and the popup. A dialog taller than the window must scroll as one surface rather than clipping its
own content, and the scroll has to live outside the popup so the popup keeps its padding and shadow.
QuickGUI supplies the stable identity and the scroll container; size, alignment, and padding stay
application-owned.

`DialogState` is Base UI's `onOpenChangeComplete`. QuickGUI owns no dialog animation — motion is
application presentation — so the state owns the one thing the framework can own exactly: the
deadline.

```rust,ignore
let state = DialogState::new()
    .enter_duration(Duration::from_millis(80))
    .exit_duration(Duration::from_millis(120));

// In a listener:
view.dialog.set_open(false, |view| &mut view.dialog, cx);
// Mount against is_mounted(), not is_open(), so the exit transition can play:
if self.dialog.is_mounted() { /* declare Dialog::root_part(..) */ }
```

`is_open` is the controlled value, `is_mounted` stays true through a closing transition, and
`open_change_complete()` reports `Some(true)` after an open finished and `Some(false)` after a close
finished. Every deadline is an exact one-shot task bounded by `MAX_DIALOG_TRANSITION`; a zero
duration completes in the same controlled update with no task at all, and a settled dialog owns no
timer, observer, or idle scheduler source. Both a `fn`-pointer and a `StateAccessor` entry point are
supplied.
