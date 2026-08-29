use super::*;

#[cfg(target_os = "macos")]
pub(super) struct ActivePlatformDialog {
    pub(super) id: PlatformDialogId,
    pub(super) open: Arc<AtomicBool>,
    pub(super) native: MacPlatformDialog,
}

#[cfg(any(
    target_os = "windows",
    target_os = "linux",
    target_os = "freebsd",
    target_os = "dragonfly",
    target_os = "openbsd",
    target_os = "netbsd"
))]
pub(super) struct ActivePlatformDialog {
    pub(super) id: PlatformDialogId,
    pub(super) _task: Task<()>,
}

#[cfg(any(
    target_os = "windows",
    target_os = "linux",
    target_os = "freebsd",
    target_os = "dragonfly",
    target_os = "openbsd",
    target_os = "netbsd"
))]
struct PlatformDialogCompletion {
    proxy: EventLoopProxy<RuntimeEvent>,
    owner: Option<WindowHandle>,
    id: PlatformDialogId,
}

#[cfg(any(
    target_os = "windows",
    target_os = "linux",
    target_os = "freebsd",
    target_os = "dragonfly",
    target_os = "openbsd",
    target_os = "netbsd"
))]
impl Drop for PlatformDialogCompletion {
    fn drop(&mut self) {
        let _ = self
            .proxy
            .send_event(RuntimeEvent::PlatformDialogClosed(self.owner, self.id));
    }
}

#[cfg(any(
    target_os = "windows",
    target_os = "linux",
    target_os = "freebsd",
    target_os = "dragonfly",
    target_os = "openbsd",
    target_os = "netbsd"
))]
async fn pick_rfd_paths(
    dialog: rfd::AsyncFileDialog,
    files: bool,
    directories: bool,
    multiple: bool,
) -> Result<Option<Vec<PathBuf>>, PlatformError> {
    let selected = match (files, directories, multiple) {
        (true, false, false) => dialog.pick_file().await.map(|path| vec![path]),
        (true, false, true) => dialog.pick_files().await,
        (false, true, false) => dialog.pick_folder().await.map(|path| vec![path]),
        (false, true, true) => dialog.pick_folders().await,
        // RFD only exposes the combined file-or-folder picker on macOS. QuickGUI's AppKit
        // backend handles that case there; other desktop targets fail explicitly.
        _ => return Err(PlatformError::Unsupported),
    };
    selected
        .map(|selected| {
            bounded_selected_paths(
                selected
                    .into_iter()
                    .map(|path| path.path().to_path_buf())
                    .collect(),
            )
        })
        .transpose()
}

#[cfg(any(
    target_os = "windows",
    target_os = "linux",
    target_os = "freebsd",
    target_os = "dragonfly",
    target_os = "openbsd",
    target_os = "netbsd"
))]
fn bounded_selected_paths(paths: Vec<PathBuf>) -> Result<Vec<PathBuf>, PlatformError> {
    if paths.len() > crate::MAX_SELECTED_PATHS {
        return Err(PlatformError::SelectionTooLarge);
    }
    let mut total_bytes = 0_usize;
    for path in &paths {
        let bytes = path.as_os_str().as_encoded_bytes();
        if bytes.is_empty() || bytes.len() > crate::MAX_PLATFORM_PATH_BYTES {
            return Err(PlatformError::SelectionTooLarge);
        }
        total_bytes = total_bytes.saturating_add(bytes.len());
        if total_bytes > crate::MAX_SELECTED_PATHS_TOTAL_BYTES {
            return Err(PlatformError::SelectionTooLarge);
        }
    }
    Ok(paths)
}

#[cfg(any(
    target_os = "windows",
    target_os = "linux",
    target_os = "freebsd",
    target_os = "dragonfly",
    target_os = "openbsd",
    target_os = "netbsd"
))]
fn bounded_selected_path(path: PathBuf) -> Result<PathBuf, PlatformError> {
    let bytes = path.as_os_str().as_encoded_bytes();
    if bytes.is_empty() || bytes.len() > crate::MAX_PLATFORM_PATH_BYTES {
        Err(PlatformError::SelectionTooLarge)
    } else {
        Ok(path)
    }
}

impl Runtime {
    pub(super) fn process_platform_requests(&mut self) {
        #[cfg(target_os = "macos")]
        self.active_platform_dialogs.retain(|owner, dialog| {
            owner
                .as_ref()
                .is_none_or(|handle| self.window_handles.contains_key(handle))
                && dialog.open.load(Ordering::Acquire)
        });

        while let Some(request) = self.platform_requests.pop_front() {
            if request.response_cancelled() {
                continue;
            }
            #[cfg(target_os = "macos")]
            self.process_macos_platform_request(request);
            #[cfg(any(
                target_os = "windows",
                target_os = "linux",
                target_os = "freebsd",
                target_os = "dragonfly",
                target_os = "openbsd",
                target_os = "netbsd"
            ))]
            self.process_rfd_platform_request(request);
            #[cfg(not(any(
                target_os = "macos",
                target_os = "windows",
                target_os = "linux",
                target_os = "freebsd",
                target_os = "dragonfly",
                target_os = "openbsd",
                target_os = "netbsd"
            )))]
            request.complete_error(PlatformError::Unsupported);
        }
    }

    #[cfg(any(
        target_os = "windows",
        target_os = "linux",
        target_os = "freebsd",
        target_os = "dragonfly",
        target_os = "openbsd",
        target_os = "netbsd"
    ))]
    fn process_rfd_platform_request(&mut self, request: PlatformRequest) {
        match request {
            PlatformRequest::OpenPaths {
                window,
                options,
                responder,
            } => self.start_rfd_open_dialog(window, options, responder),
            PlatformRequest::SavePath {
                window,
                options,
                responder,
            } => self.start_rfd_save_dialog(window, options, responder),
            request => request.complete_error(PlatformError::Unsupported),
        }
    }

    #[cfg(any(
        target_os = "windows",
        target_os = "linux",
        target_os = "freebsd",
        target_os = "dragonfly",
        target_os = "openbsd",
        target_os = "netbsd"
    ))]
    fn rfd_parent_window(
        &self,
        owner: Option<WindowHandle>,
    ) -> Result<Option<Arc<Window>>, PlatformError> {
        self.validate_platform_dialog_owner(owner)?;
        let Some(owner) = owner else {
            return Ok(None);
        };
        let window_id = self
            .window_handles
            .get(&owner)
            .copied()
            .ok_or(PlatformError::Unavailable)?;
        self.windows
            .get(&window_id)
            .map(|entry| Some(entry.state.window.clone()))
            .ok_or(PlatformError::Unavailable)
    }

    #[cfg(any(
        target_os = "macos",
        target_os = "windows",
        target_os = "linux",
        target_os = "freebsd",
        target_os = "dragonfly",
        target_os = "openbsd",
        target_os = "netbsd"
    ))]
    fn validate_platform_dialog_owner(
        &self,
        owner: Option<WindowHandle>,
    ) -> Result<(), PlatformError> {
        let busy = self.active_platform_dialogs.keys().any(|candidate| {
            match (owner, *candidate) {
                (Some(owner), Some(candidate)) => self.windows_share_parent_chain(owner, candidate),
                // An application-modal dialog is exclusive across the whole application.
                (None, _) | (_, None) => true,
            }
        });
        if busy {
            return Err(PlatformError::DialogBusy);
        }
        if self.active_platform_dialogs.len() == crate::MAX_ACTIVE_PLATFORM_DIALOGS {
            return Err(PlatformError::TooManyDialogs);
        }
        Ok(())
    }

    #[cfg(any(
        target_os = "windows",
        target_os = "linux",
        target_os = "freebsd",
        target_os = "dragonfly",
        target_os = "openbsd",
        target_os = "netbsd"
    ))]
    fn start_rfd_open_dialog(
        &mut self,
        owner: Option<WindowHandle>,
        options: PathPromptOptions,
        responder: crate::platform::PlatformResponder<Option<Vec<PathBuf>>>,
    ) {
        let native_window = match self.rfd_parent_window(owner) {
            Ok(window) => window,
            Err(error) => {
                responder.complete(Err(error));
                return;
            }
        };
        if options.files && options.directories {
            responder.complete(Err(PlatformError::Unsupported));
            return;
        }

        let mut dialog = rfd::AsyncFileDialog::new();
        if let Some(native_window) = &native_window {
            dialog = dialog.set_parent(native_window.as_ref());
        }
        if let Some(directory) = &options.directory {
            dialog = dialog.set_directory(directory);
        }
        if let Some(name) = &options.suggested_name {
            dialog = dialog.set_file_name(name.as_ref());
        }
        if let Some(title) = &options.title {
            dialog = dialog.set_title(title.as_ref());
        }
        for filter in &options.filters {
            let extensions = filter
                .extensions
                .iter()
                .map(|extension| extension.as_ref())
                .collect::<Vec<&str>>();
            dialog = dialog.add_filter(filter.name.as_ref(), &extensions);
        }
        let files = options.files;
        let directories = options.directories;
        let multiple = options.multiple;
        let id = PlatformDialogId::next();
        let completion = PlatformDialogCompletion {
            proxy: self.event_proxy.clone(),
            owner,
            id,
        };
        let failed_responder = responder.clone();
        let future = async move {
            let _completion = completion;
            responder.complete(pick_rfd_paths(dialog, files, directories, multiple).await);
        };
        let task = match owner {
            Some(owner) => self
                .foreground_tasks
                .spawn::<(), _, _, _>(owner, move |_| future),
            None => self.foreground_tasks.spawn_application(future),
        };
        match task {
            Ok(task) => {
                self.active_platform_dialogs
                    .insert(owner, ActivePlatformDialog { id, _task: task });
            }
            Err(error) => {
                failed_responder.complete(Err(PlatformError::Platform(error.to_string().into())))
            }
        }
    }

    #[cfg(any(
        target_os = "windows",
        target_os = "linux",
        target_os = "freebsd",
        target_os = "dragonfly",
        target_os = "openbsd",
        target_os = "netbsd"
    ))]
    fn start_rfd_save_dialog(
        &mut self,
        owner: Option<WindowHandle>,
        options: SavePathOptions,
        responder: crate::platform::PlatformResponder<Option<PathBuf>>,
    ) {
        let native_window = match self.rfd_parent_window(owner) {
            Ok(window) => window,
            Err(error) => {
                responder.complete(Err(error));
                return;
            }
        };

        let mut dialog = rfd::AsyncFileDialog::new().set_directory(&options.directory);
        if let Some(native_window) = &native_window {
            dialog = dialog.set_parent(native_window.as_ref());
        }
        if let Some(name) = &options.suggested_name {
            dialog = dialog.set_file_name(name.as_ref());
        }
        if let Some(title) = &options.title {
            dialog = dialog.set_title(title.as_ref());
        }
        for filter in &options.filters {
            let extensions = filter
                .extensions
                .iter()
                .map(|extension| extension.as_ref())
                .collect::<Vec<&str>>();
            dialog = dialog.add_filter(filter.name.as_ref(), &extensions);
        }
        let id = PlatformDialogId::next();
        let completion = PlatformDialogCompletion {
            proxy: self.event_proxy.clone(),
            owner,
            id,
        };
        let failed_responder = responder.clone();
        let future = async move {
            let _completion = completion;
            let result = dialog
                .save_file()
                .await
                .map(|path| bounded_selected_path(path.path().to_path_buf()))
                .transpose();
            responder.complete(result);
        };
        let task = match owner {
            Some(owner) => self
                .foreground_tasks
                .spawn::<(), _, _, _>(owner, move |_| future),
            None => self.foreground_tasks.spawn_application(future),
        };
        match task {
            Ok(task) => {
                self.active_platform_dialogs
                    .insert(owner, ActivePlatformDialog { id, _task: task });
            }
            Err(error) => {
                failed_responder.complete(Err(PlatformError::Platform(error.to_string().into())))
            }
        }
    }

    #[cfg(target_os = "macos")]
    fn process_macos_platform_request(&mut self, request: PlatformRequest) {
        match request {
            PlatformRequest::ShowSystemNotification(notification) => {
                self.mac_application_host
                    .show_system_notification(notification);
            }
            PlatformRequest::DismissSystemNotification(tag) => {
                self.mac_application_host.dismiss_system_notification(&tag);
            }
            PlatformRequest::OpenUrl(url) => {
                if let Err(error) = shell_open_url(&url) {
                    tracing::warn!(%error, url = %url, "could not open URL with NSWorkspace");
                }
            }
            PlatformRequest::OpenPath(path) => {
                if let Err(error) = shell_open_path(&path) {
                    tracing::warn!(%error, path = %path.display(), "could not open path with NSWorkspace");
                }
            }
            PlatformRequest::RevealPath(path) => {
                if let Err(error) = shell_reveal_path(&path) {
                    tracing::warn!(%error, path = %path.display(), "could not reveal path with NSWorkspace");
                }
            }
            request => {
                let owner = request.window();
                let native_window = match owner {
                    Some(owner) => {
                        let Some(window_id) = self.window_handles.get(&owner).copied() else {
                            request.complete_error(PlatformError::Unavailable);
                            return;
                        };
                        let Some(native_window) = self
                            .windows
                            .get(&window_id)
                            .map(|entry| entry.state.window.clone())
                        else {
                            request.complete_error(PlatformError::Unavailable);
                            return;
                        };
                        Some(native_window)
                    }
                    None => None,
                };
                if let Err(error) = self.validate_platform_dialog_owner(owner) {
                    request.complete_error(error);
                    return;
                }

                let id = PlatformDialogId::next();
                let open = Arc::new(AtomicBool::new(true));
                if !request.bind_cancellation(self.event_proxy.clone(), owner, id) {
                    return;
                }
                let context = MacPlatformDialogContext::new(
                    owner,
                    id,
                    open.clone(),
                    self.event_proxy.clone(),
                );
                let native = match request {
                    PlatformRequest::Prompt {
                        level,
                        message,
                        detail,
                        buttons,
                        responder,
                        ..
                    } => {
                        let completion = responder.clone();
                        match present_native_prompt(
                            native_window.as_ref(),
                            context,
                            level,
                            &message,
                            detail.as_deref(),
                            &buttons,
                            completion,
                        ) {
                            Ok(native) => native,
                            Err(error) => {
                                responder.complete(Err(PlatformError::Platform(error.into())));
                                return;
                            }
                        }
                    }
                    PlatformRequest::OpenPaths {
                        options, responder, ..
                    } => {
                        let completion = responder.clone();
                        match present_native_open_panel(
                            native_window.as_ref(),
                            context,
                            &options,
                            completion,
                        ) {
                            Ok(native) => native,
                            Err(error) => {
                                responder.complete(Err(PlatformError::Platform(error.into())));
                                return;
                            }
                        }
                    }
                    PlatformRequest::SavePath {
                        options, responder, ..
                    } => {
                        let completion = responder.clone();
                        match present_native_save_panel(
                            native_window.as_ref(),
                            context,
                            &options,
                            completion,
                        ) {
                            Ok(native) => native,
                            Err(error) => {
                                responder.complete(Err(PlatformError::Platform(error.into())));
                                return;
                            }
                        }
                    }
                    PlatformRequest::ShowSystemNotification(_)
                    | PlatformRequest::DismissSystemNotification(_)
                    | PlatformRequest::OpenUrl(_)
                    | PlatformRequest::OpenPath(_)
                    | PlatformRequest::RevealPath(_) => {
                        unreachable!("application-wide platform actions returned above")
                    }
                };
                if open.load(Ordering::Acquire) {
                    self.active_platform_dialogs
                        .insert(owner, ActivePlatformDialog { id, open, native });
                }
            }
        }
    }
}
