//! Native recursive file notifications, independent of the UI and application main thread.

use notify::{RecommendedWatcher, RecursiveMode, Watcher};
use std::path::PathBuf;

pub const MAX_WATCH_ROOTS: usize = 16;
pub const MAX_WATCH_EVENT_PATHS: usize = 256;

/// A bounded batch of absolute changed paths. `rescan` asks the caller to refresh its snapshot.
#[derive(Clone, Debug)]
pub struct FileWatchEvent {
    pub paths: Vec<PathBuf>,
    pub rescan: bool,
    pub error: Option<String>,
}

/// Keeps native recursive watches alive. Dropping it releases the OS registrations.
pub struct FileWatcher {
    _watcher: RecommendedWatcher,
}

impl FileWatcher {
    /// Create on a background thread; notifications arrive on the platform watcher's thread.
    pub fn new(
        paths: &[PathBuf],
        mut on_change: impl FnMut(FileWatchEvent) + Send + 'static,
    ) -> Result<Self, String> {
        if paths.is_empty() || paths.len() > MAX_WATCH_ROOTS {
            return Err(format!(
                "a file watcher requires 1 to {MAX_WATCH_ROOTS} roots"
            ));
        }
        if paths.iter().any(|path| !path.is_absolute()) {
            return Err("file watcher roots must be absolute paths".into());
        }
        let mut watcher =
            notify::recommended_watcher(move |result: notify::Result<notify::Event>| {
                let event = match result {
                    Ok(event) => {
                        if event.kind.is_access() {
                            return;
                        }
                        let rescan =
                            event.need_rescan() || event.paths.len() > MAX_WATCH_EVENT_PATHS;
                        FileWatchEvent {
                            paths: event
                                .paths
                                .into_iter()
                                .take(MAX_WATCH_EVENT_PATHS)
                                .collect(),
                            rescan,
                            error: None,
                        }
                    }
                    Err(error) => FileWatchEvent {
                        paths: Vec::new(),
                        rescan: true,
                        error: Some(error.to_string()),
                    },
                };
                on_change(event);
            })
            .map_err(|error| error.to_string())?;
        for path in paths {
            watcher
                .watch(path, RecursiveMode::Recursive)
                .map_err(|error| error.to_string())?;
        }
        Ok(Self { _watcher: watcher })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        fs,
        sync::mpsc,
        time::{Duration, Instant, SystemTime, UNIX_EPOCH},
    };

    #[test]
    fn watches_nested_changes_and_releases_registrations() {
        let directory = std::env::temp_dir().join(format!(
            "quickgui-watch-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(directory.join("nested")).unwrap();
        let directory = fs::canonicalize(directory).unwrap();
        let path = directory.join("nested/file.txt");
        let (send, receive) = mpsc::channel();
        let watcher = FileWatcher::new(std::slice::from_ref(&directory), move |event| {
            let _ = send.send(event);
        })
        .unwrap();
        fs::write(&path, "native notification").unwrap();
        let deadline = Instant::now() + Duration::from_secs(10);
        loop {
            let event = receive
                .recv_timeout(deadline.saturating_duration_since(Instant::now()))
                .expect("recursive file notification");
            assert!(event.error.is_none(), "{:?}", event.error);
            if event.rescan || event.paths.iter().any(|changed| changed == &path) {
                break;
            }
        }
        drop(watcher);
        fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn rejects_empty_or_relative_roots() {
        assert!(FileWatcher::new(&[], |_| {}).is_err());
        assert!(FileWatcher::new(&[PathBuf::from("relative")], |_| {}).is_err());
    }
}
