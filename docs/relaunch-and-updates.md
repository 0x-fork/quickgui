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

### Download progress from Go

`native.Updater.Stage` downloads and verifies an artifact on a native worker.
Progress and completion return through the in-process purego callback boundary;
neither the download nor a Go callback blocks the native main thread.

```go
func StageUpdate(update native.AvailableUpdate, directory string, options native.UpdateClientOptions) {
	native.Updater.Stage(
		update,
		directory,
		options,
		func(progress native.UpdateProgress) {
			if progress.Phase == "downloaded" && progress.TotalBytes != nil && *progress.TotalBytes > 0 {
				percent := float64(progress.DownloadedBytes) / float64(*progress.TotalBytes) * 100
				log.Print("Downloaded percent: ", percent)
			}
		},
		func(artifact string, err error) {
			if err != nil {
				log.Print(err)
				return
			}
			log.Print("Verified artifact: ", artifact)
		},
	)
}
```

`Phase` is `download-started`, `downloaded`, `download-finished`,
`verification-started`, `verification-finished`, or `staged`. Byte counts are
`uint64`; `TotalBytes` is nil when the server supplies no length. Installation is
an explicit `native.Updater.Install` call and does not emit stage progress.
Pass nil for the progress callback to allocate no progress listener.

## Publishing updates from the CLI

`quickgui build --update-manifest` produces exactly the artifact `install_staged` accepts for the
target, signs it, and writes `latest.json` beside it.

```console
quickgui keygen                                  # writes quickgui-update.pub / .key
quickgui build --update-manifest --update-base-url https://dl.example.com/demo
```

```ts
// quickgui.config.ts
updates: {
  manifest: true,                                 // same as passing --update-manifest
  baseUrl: "https://dl.example.com/demo",
  minisignSecretKey: "keys/quickgui-update.key",  // or QUICKGUI_MINISIGN_SECRET_KEY
  notesFile: "RELEASE_NOTES.md",
},
```

| Target | Published artifact | Produced by |
| --- | --- | --- |
| `darwin-*` | `<Name>.app.tar.gz` — a gzip tar with one top-level `.app` | `tar -czf` over the signed bundle |
| `linux-*` with an AppImage | `<name>-<version>-<arch>.AppImage.tar.gz` | `tar -czf` over the AppImage |
| `linux-*` without `appimagetool` | the raw executable | the build itself |
| `windows-*` | `<name>-<version>-setup.exe` | `makensis`; the build fails with an actionable message when NSIS is missing |

Signing shells out to `minisign -S` or, if only that is installed, `rsign sign`; if neither is on
PATH the build fails and names both. The secret key comes from `QUICKGUI_MINISIGN_SECRET_KEY` when
set, otherwise `updates.minisignSecretKey`. Neither signing subcommand takes a password flag, so an
encrypted key reads its password from standard input: set `QUICKGUI_MINISIGN_PASSWORD` and QuickGUI
writes it there. Without that variable the child's stdin is closed, which is what an unencrypted key
(the default from `quickgui keygen`) needs.

The manifest is the `Manifest` struct `UpdateClient::parse_manifest` deserializes:

```json
{
  "version": "1.4.0",
  "notes": "Fixes a crash when reopening a document window.",
  "pub_date": "2026-09-03T12:00:00Z",
  "platforms": {
    "darwin-aarch64": { "url": "https://dl.example.com/demo/Demo.app.tar.gz", "signature": "…" }
  }
}
```

Platform keys match `default_update_target()` — `<darwin|linux|windows>-<std::env::consts::ARCH>`,
so ARM is `aarch64` and Intel is `x86_64`, not `arm64`/`x64`. The flat `url`/`signature` form is
also accepted by the core when a manifest describes exactly one artifact. `signature` is the
base64 of the `.minisig` file; the core accepts either that or the raw multi-line text.

Building a second target against the same `latest.json` merges its entry into `platforms` as long
as `version` still matches, so a matrix build can publish one manifest covering every target.
`tests/fixtures/updater-manifest.json` is the shared fixture: `parse_manifest` accepts it in a
Rust test and the CLI's TypeScript test asserts the same shape, so the generator and the installer
cannot drift apart.

Every check, download, verification, extraction, and installation entry point has a variant that
accepts the cloneable `UpdateCancellation` token. Cancellation is checked before network or
filesystem mutation and at bounded streaming/extraction boundaries. Progress callbacks report
checking, downloading, verifying, extracting, installing, and completion without retaining an
unbounded event history.
