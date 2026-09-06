# QuickGUI system APIs

This example imports system services from `@quickgui/native` and the UI components from
`@quickgui/ui`.

```console
cd examples/system-api
bun run dev
```

The UI demonstrates core-owned app identity and paths, system information and preferences,
permission status, rich clipboard representations, displays and the global cursor, notifications,
shell launching, credential storage, autostart, custom protocols, global shortcuts, tray icons,
native menus, power snapshots and assertions, desktop integration discovery, Dock badges, native
file icons, window state and controls, and single-instance forwarding. It only changes OS state
after a button click. Remove the credential, autostart entry, or protocol registration after
experimenting if you do not want to keep it.

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
- `Updater` discovers, stages, re-verifies, and installs supported native artifacts. The example
  does not ship a fake endpoint, signing key, or disposable installation target, so it only shows
  the resolved update target.
