# QuickGUI file dialogs

This example presents operating-system open and save panels from event handlers. The result
objects follow Electron's familiar cancellation and selected-path shape.

```console
cd examples/file-dialog
bun run dev
```

`Dialog.showOpenDialog` resolves to `{ canceled, filePaths }`. `Dialog.showSaveDialog` resolves to
`{ canceled, filePath }`; choosing a destination does not create or write the file. Native file
dialogs are available on macOS, Windows, and Linux. Pass a `Window` first to attach the dialog, or
omit it for an application-modal dialog. macOS uses QuickGUI's AppKit panels; the other desktop
targets use `rfd`.

Linux builds use the XDG Desktop Portal backend. Packaged applications should ensure a GTK,
GNOME, or KDE portal backend and Zenity are installed; Zenity is the fallback when the portal is
unavailable. Custom button labels and hidden-file visibility are currently macOS-only options.
Native save dialogs differ in how they enforce or append filter extensions, so applications should
validate the returned destination instead of assuming the selected filter rewrote it.
