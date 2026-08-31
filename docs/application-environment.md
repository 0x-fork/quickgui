# Application identity, paths, and system information

[Documentation index](README.md)

QuickGUI keeps application identity and environment discovery in the Rust core. Supply validated
package metadata before startup and the runtime resolves one immutable path snapshot from its
identifier:

```rust
use quickgui::{Application, AppInfo, WindowOptions};

let info = AppInfo::new("Example", "1.2.3", "dev.example.desktop")?;

Application::new()
    .app_info(info)
    .run(|cx| {
        cx.open_window(WindowOptions::default(), MyView::default());
    })?;
```

`AppInfo` rejects empty, padded, multiline, NUL-containing, or oversized values. The identifier is
also restricted to a portable ASCII path component, so it cannot escape the standard base
directories.

## Resolved paths

`AppPaths` resolves without creating directories. `config_dir`, `data_dir`, `local_data_dir`,
`cache_dir`, `log_dir`, and `runtime_dir` are app-scoped with the package identifier. It also
captures the executable and executable directory, the macOS bundle `Contents/Resources` directory
when applicable, the system temporary directory, and available home/Desktop/Documents/Downloads/
Music/Pictures/Videos paths.

Applications with a packaging-specific resource layout can resolve and override individual paths
before launch:

```rust
let paths = info.paths()?.with_resource_dir("/opt/example/resources");

Application::new()
    .app_info(info)
    .app_paths(paths)
    .run(|cx| {
        cx.open_window(WindowOptions::default(), MyView::default());
    })?;
```

Overrides replace only the immutable snapshot; they do not mutate process environment variables or
create filesystem entries.

## Runtime access

`AppRunner`, `ViewContext`, `EventContext`, and `TestAppContext` expose `app_info()`, `app_paths()`,
and `system_info()`. Package identity and paths are optional to preserve metadata-free embedders.
`SystemInfo` is always present and is captured once per runtime. It includes the compile-time OS and
family, platform/distribution name and available version details, architecture and bitness,
hostname, preferred locale, and a bounded ordered language list. Rendering and event callbacks read
the retained snapshot and never poll the operating system. `AppInfo`, `AppPaths`, and `SystemInfo`
clones share their immutable string/path storage, so constructing an event context does not copy the
directory tree.

On Windows, the runtime also applies a valid `AppInfo` identifier as the process application user
model ID before it creates native windows. Windows limits this identity to 128 ASCII characters;
longer portable identifiers can still scope application paths, but cannot provide Windows shell
grouping or system-notification identity.

At most 64 preferred languages and 16 KiB of language data are retained. Individual system and
locale strings are also bounded; malformed or unavailable optional platform values are omitted.
