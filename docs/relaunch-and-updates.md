# Relaunch and signed updates

[Documentation index](README.md)

QuickGUI owns relaunch scheduling in the Rust application core. Calling `cx.relaunch()` prepares
the replacement process immediately, requests the ordinary child-first application teardown, and
spawns only after native windows and callbacks are complete, foreground work is cancelled,
background queues are closed, and global shortcuts, tray icons, and the single-instance guard have
been released:

```rust
use quickgui::EventContext;

fn restart(cx: &mut EventContext) -> Result<(), quickgui::SystemIntegrationError> {
    cx.relaunch()
}
```

`RelaunchOptions` can replace the executable, arguments, or working directory. Unspecified values
preserve the current process. Paths must be absolute, all native strings are NUL-free and bounded,
and the replacement is launched directly without a shell. One application retains at most one
request. `AppRunner::relaunch` follows the same boundary; its next exit-producing `pump` releases
process services, spawns once, and exposes the resulting process ID through
`relaunched_process()`.

`on_before_quit` and `on_will_quit` receive a `QuitRequest` whose reason distinguishes an explicit
quit, operating-system termination, final-window policy, and relaunch. Calling `prevent_quit()` in
either phase cancels that attempt; cancelling a relaunch also discards its prepared replacement
process. macOS answers `applicationShouldTerminate` only after both Rust callbacks complete, so the
native application and core lifecycle cannot disagree.

The deterministic `TestAppContext` performs the complete window/callback teardown but never
spawns. `relaunch_request()` exposes the prepared value for assertions.

## Signed updater flow

With the default `updater` feature, `UpdateClient` accepts Tauri-compatible update JSON, requires
HTTPS, selects a newer target artifact, and verifies modern prehashed Minisign signatures while
streaming to disk. `download_and_stage_with_progress` reports connection-independent milestones
and bounded byte counts:

```rust
use quickgui::{UpdateClient, UpdateProgress};

let client = UpdateClient::new(env!("CARGO_PKG_VERSION"), PUBLIC_KEY)?;
if let Some(update) = client.check(UPDATE_ENDPOINT)? {
    let update_directory = paths
        .cache_dir()
        .expect("this platform has a cache directory")
        .join("updates");
    let artifact = client.download_and_stage_with_progress(
        &update,
        update_directory,
        |event| match event {
            UpdateProgress::Downloaded { downloaded_bytes, total_bytes, .. } => {
                report_download(downloaded_bytes, total_bytes);
            }
            _ => {}
        },
    )?;
    let installed = client.install_staged(
        &update,
        artifact,
        quickgui::UpdateInstallOptions::default(),
    )?;
    if installed.relaunch_recommended() {
        // Return to the application thread and call `cx.relaunch()`.
    }
}
```

Installation always re-verifies the staged file against the selected update before changing or
launching anything. Platform strategies are explicit:

- macOS accepts a gzip tar archive with one confined top-level `.app`, extracts beside the current
  bundle, and uses a rollback-capable same-filesystem rename;
- Linux accepts a raw executable or a gzip tar archive with one confined AppImage, preserves the
  installed file permissions, and uses the same rollback-capable replacement;
- Windows accepts `.exe` and `.msi` installers, launches them directly with basic, passive, or
  quiet policy plus bounded caller arguments, and returns the child process ID.

Archives are limited to 131,072 entries and 8 GiB expanded data, reject escaping paths and links,
and are extracted on the destination filesystem. Downloads default to a 2 GiB compressed limit.
`retain_backup(true)` returns the previous Unix application path for caller-managed cleanup;
otherwise QuickGUI removes it after a successful swap and reports it only if cleanup failed.

`InstalledUpdate::requires_application_exit()` is always true: an applied Unix update needs an
orderly relaunch to execute the new image, while a launched Windows installer needs the current
application to leave its files and locks. Downloading is synchronous and belongs in bounded
background work; lifecycle calls such as `cx.relaunch()` remain on the application thread.

Every check, download, verification, extraction, and installation entry point has a variant that
accepts the cloneable `UpdateCancellation` token. Cancellation is checked before network or
filesystem mutation and at bounded streaming/extraction boundaries. Progress callbacks report
checking, downloading, verifying, extracting, installing, and completion without retaining an
unbounded event history.
