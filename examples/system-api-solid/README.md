# QuickGUI Solid system APIs

This example exercises system services owned by QuickGUI core and re-exported unchanged through
`@quickgui/native` and `@quickgui/solid`.

```console
cd examples/system-api-solid
bun run dev
```

The UI demonstrates clipboard access, displays, notifications, shell launching, credential
storage, autostart, custom protocols, global shortcuts, tray icons, native menus, power events,
and single-instance forwarding. It only changes OS state after a button click. Remove the
credential, autostart entry, or protocol registration after experimenting if you do not want to
keep it.

Platform notes:

- `DeepLink.register` is dynamic on Windows and Linux. macOS reads schemes from the signed bundle,
  so the same scheme is declared through `protocols` in `quickgui.config.ts`.
- Linux tray icons use StatusNotifierItem over D-Bus. `showMenu()` is not portable there because
  the desktop shell owns menu presentation.
- Linux notifications and trashing prefer XDG Desktop Portals. Trashing falls back to `gio`; the
  BSD fallback uses the platform's `notify-send`/`gio` commands.
- Linux global shortcuts currently require X11; Wayland intentionally does not permit the same
  unrestricted registration model.
- Secure storage uses Keychain Services, Windows Credential Manager, or Secret Service.
- `Updater` discovers and stages a Minisign-verified artifact. Installing it is deliberately left
  to the app's package format and is not demonstrated with a fake endpoint or signing key.
