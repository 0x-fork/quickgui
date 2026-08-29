# QuickGUI Solid alert dialogs

This example presents operating-system alert dialogs from Solid event handlers and updates the
retained UI after the user selects a button.

```console
cd examples/alert-dialog-solid
bun run dev
```

`Dialog.showAlertDialog(options)` presents an application-modal alert. Pass a `Window` first to
attach it as a native sheet: `Dialog.showAlertDialog(window, options)`. The promise resolves with
the zero-based index of the selected button. Button roles give the platform its default and cancel
keyboard behavior. Native alert dialogs are available on macOS, Windows, and Linux.
