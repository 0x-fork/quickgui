use super::*;

impl AppRunner {
    /// Request an orderly application exit after native windows and owned resources close.
    pub fn exit(&mut self) -> bool {
        if !matches!(self.status, AppRunStatus::Continue) || self.runtime.exit_requested {
            return false;
        }
        if self
            .runtime
            .event_proxy
            .send_event(RuntimeEvent::ExternalCommandsReady)
            .is_err()
        {
            return false;
        }
        self.runtime.exit_requested = true;
        true
    }

    /// Return the latest bounded display snapshot retained by the application runtime.
    ///
    /// This does not poll the operating system. The snapshot is replaced at native display-change
    /// boundaries and is therefore safe for embedding runtimes to read after each pump.
    pub fn displays(&self) -> Displays {
        self.runtime.displays.clone()
    }

    /// Return the latest immutable keyboard-layout snapshot retained by the runtime.
    pub fn keyboard_layout(&self) -> KeyboardLayout {
        self.runtime.keyboard.layout().clone()
    }

    /// Return a constant-size snapshot of one mounted native window.
    pub fn window_state(&self, handle: WindowHandle) -> Option<WindowState> {
        if self.runtime.current_handle() == Some(handle) {
            return window_state_snapshot(
                handle,
                &self.runtime.config,
                self.runtime.window.as_ref()?,
            );
        }
        let window_id = self.runtime.window_handles.get(&handle)?;
        let entry = self.runtime.windows.get(window_id)?;
        window_state_snapshot(handle, &entry.config, &entry.state)
    }

    /// Read one bounded item from the operating system's general clipboard.
    pub fn read_from_clipboard(&self) -> Result<Option<ClipboardItem>, crate::ClipboardError> {
        self.runtime.clipboard.read(ClipboardTarget::General)
    }

    /// Atomically replace the operating system's general clipboard with one bounded item.
    pub fn write_to_clipboard(&self, item: ClipboardItem) -> Result<(), crate::ClipboardError> {
        self.runtime.clipboard.write(ClipboardTarget::General, item)
    }

    /// Ask the operating system to open a URL with its registered handler.
    pub fn open_external(
        &mut self,
        url: impl Into<Arc<str>>,
    ) -> Result<ShellResponse, PlatformError> {
        let (request, response) = PlatformRequest::open_url_response(url)?;
        self.queue_platform_response(request)?;
        Ok(response)
    }

    /// Ask the operating system to open a filesystem item with its registered application.
    pub fn open_path(&mut self, path: impl Into<PathBuf>) -> Result<ShellResponse, PlatformError> {
        let (request, response) = PlatformRequest::open_path_response(path)?;
        self.queue_platform_response(request)?;
        Ok(response)
    }

    /// Reveal a filesystem item in the platform file manager.
    pub fn reveal_path(
        &mut self,
        path: impl Into<PathBuf>,
    ) -> Result<ShellResponse, PlatformError> {
        let (request, response) = PlatformRequest::reveal_path_response(path)?;
        self.queue_platform_response(request)?;
        Ok(response)
    }

    /// Move a filesystem item to the operating system trash or recycle bin.
    pub fn trash_path(&mut self, path: impl Into<PathBuf>) -> Result<ShellResponse, PlatformError> {
        let (request, response) = PlatformRequest::trash_path_response(path)?;
        self.queue_platform_response(request)?;
        Ok(response)
    }

    /// Post or replace one operating-system notification.
    pub fn show_system_notification(
        &mut self,
        notification: SystemNotification,
    ) -> Result<(), PlatformError> {
        let request = PlatformRequest::show_system_notification(notification)?;
        self.queue_platform_request(request)
    }

    /// Dismiss the operating-system notification identified by `tag`, where supported.
    pub fn dismiss_system_notification(
        &mut self,
        tag: impl Into<Arc<str>>,
    ) -> Result<(), PlatformError> {
        let request = PlatformRequest::dismiss_system_notification(tag)?;
        self.queue_platform_request(request)
    }

    /// Replace the complete native application menu set on the next event-loop turn.
    pub fn set_application_menus(&mut self, menus: Vec<Menu>) -> Result<(), PlatformError> {
        #[cfg(not(any(target_os = "macos", target_os = "windows")))]
        {
            let _ = menus;
            return Err(PlatformError::Unsupported);
        }
        #[cfg(any(target_os = "macos", target_os = "windows"))]
        {
            if !matches!(self.status, AppRunStatus::Continue) {
                return Err(PlatformError::Unavailable);
            }
            self.runtime
                .event_proxy
                .send_event(RuntimeEvent::ExternalCommandsReady)
                .map_err(|_| PlatformError::Unavailable)?;
            self.runtime.external_menus = Some(menus);
            Ok(())
        }
    }

    /// Queue one title mutation for a mounted native window.
    pub fn set_window_title(
        &mut self,
        handle: WindowHandle,
        title: impl Into<String>,
    ) -> Result<(), WindowCommandError> {
        let title = title.into();
        validate_window_title(&title)?;
        self.queue_window_command(handle, WindowCommand::SetTitle(handle, title))
    }

    /// Queue a native minimize request for a mounted window.
    pub fn minimize_window(&mut self, handle: WindowHandle) -> Result<(), WindowCommandError> {
        self.queue_window_command(handle, WindowCommand::Minimize(handle))
    }

    /// Restore a minimized or maximized native window to its windowed bounds.
    pub fn restore_window(&mut self, handle: WindowHandle) -> Result<(), WindowCommandError> {
        self.queue_window_command(handle, WindowCommand::Restore(handle))
    }

    /// Toggle the platform-native maximized/zoomed state.
    pub fn maximize_window(&mut self, handle: WindowHandle) -> Result<(), WindowCommandError> {
        if self
            .window_state(handle)
            .is_some_and(|state| state.maximized)
        {
            return Ok(());
        }
        self.queue_window_command(handle, WindowCommand::Zoom(handle))
    }

    /// Set the native fullscreen state.
    pub fn set_window_fullscreen(
        &mut self,
        handle: WindowHandle,
        fullscreen: bool,
    ) -> Result<(), WindowCommandError> {
        self.queue_window_command(handle, WindowCommand::SetFullscreen(handle, fullscreen))
    }

    /// Show or hide a mounted native window.
    pub fn set_window_visible(
        &mut self,
        handle: WindowHandle,
        visible: bool,
    ) -> Result<(), WindowCommandError> {
        self.queue_window_command(handle, WindowCommand::SetVisible(handle, visible))
    }

    /// Bring a mounted native window to the front and give it keyboard focus.
    pub fn focus_window(&mut self, handle: WindowHandle) -> Result<(), WindowCommandError> {
        self.ensure_window_command_target(handle)?;
        if self.runtime.focus_requests.len() == MAX_PENDING_WINDOW_COMMANDS {
            return Err(WindowCommandError::QueueFull);
        }
        self.wake_for_external_command()?;
        if !self.runtime.focus_requests.contains(&handle) {
            self.runtime.focus_requests.push(handle);
        }
        Ok(())
    }

    /// Request informational attention for a mounted native window.
    pub fn request_window_attention(
        &mut self,
        handle: WindowHandle,
    ) -> Result<(), WindowCommandError> {
        self.queue_window_command(handle, WindowCommand::RequestAttention(handle))
    }

    /// Set or clear the represented document path in native window chrome.
    pub fn set_window_represented_file(
        &mut self,
        handle: WindowHandle,
        path: Option<PathBuf>,
    ) -> Result<(), WindowCommandError> {
        if let Some(path) = &path {
            validate_window_document_path(path)?;
        }
        self.queue_window_command(handle, WindowCommand::SetRepresentedFile(handle, path))
    }

    /// Set the native unsaved-document indicator for a mounted window.
    pub fn set_window_document_edited(
        &mut self,
        handle: WindowHandle,
        edited: bool,
    ) -> Result<(), WindowCommandError> {
        self.queue_window_command(handle, WindowCommand::SetDocumentEdited(handle, edited))
    }

    /// Force one native window to light/dark appearance, or follow the operating system with
    /// `None`.
    pub fn set_window_appearance(
        &mut self,
        handle: WindowHandle,
        appearance: Option<WindowAppearance>,
    ) -> Result<(), WindowCommandError> {
        self.queue_window_command(handle, WindowCommand::SetAppearance(handle, appearance))
    }

    fn queue_window_command(
        &mut self,
        handle: WindowHandle,
        command: WindowCommand,
    ) -> Result<(), WindowCommandError> {
        self.ensure_window_command_target(handle)?;
        if self.runtime.window_commands.len() == MAX_PENDING_WINDOW_COMMANDS {
            return Err(WindowCommandError::QueueFull);
        }
        self.wake_for_external_command()?;
        self.runtime.window_commands.push(command);
        Ok(())
    }

    fn queue_platform_response(&mut self, request: PlatformRequest) -> Result<(), PlatformError> {
        self.queue_platform_request(request)
    }

    fn queue_platform_request(&mut self, request: PlatformRequest) -> Result<(), PlatformError> {
        if !matches!(self.status, AppRunStatus::Continue) {
            return Err(PlatformError::Unavailable);
        }
        if self.runtime.platform_requests.len() == crate::MAX_PENDING_PLATFORM_REQUESTS {
            return Err(PlatformError::PendingQueueFull);
        }
        self.runtime
            .event_proxy
            .send_event(RuntimeEvent::ExternalCommandsReady)
            .map_err(|_| PlatformError::Unavailable)?;
        self.runtime.platform_requests.push_back(request);
        Ok(())
    }

    fn ensure_window_command_target(&self, handle: WindowHandle) -> Result<(), WindowCommandError> {
        if !matches!(self.status, AppRunStatus::Continue)
            || !(self.runtime.window_handles.contains_key(&handle)
                || self.runtime.current_handle() == Some(handle))
        {
            Err(WindowCommandError::Unavailable)
        } else {
            Ok(())
        }
    }

    fn wake_for_external_command(&self) -> Result<(), WindowCommandError> {
        self.runtime
            .event_proxy
            .send_event(RuntimeEvent::ExternalCommandsReady)
            .map_err(|_| WindowCommandError::Unavailable)
    }
}

fn window_state_snapshot(
    handle: WindowHandle,
    config: &AppConfig,
    state: &RuntimeWindow,
) -> Option<WindowState> {
    let platform_content_attached = runtime_window_content_attached(state);
    let fullscreen = runtime_window_is_fullscreen(state);
    let maximized = !fullscreen && runtime_window_is_maximized(state, config);
    let minimized = platform_content_attached && state.window.is_minimized().unwrap_or(false);
    let current_bounds = Rect::new(
        state.logical_position.x,
        state.logical_position.y,
        state.logical_size.width,
        state.logical_size.height,
    );
    let bounds = if fullscreen {
        WindowBounds::Fullscreen(state.restore_bounds)
    } else if maximized {
        WindowBounds::Maximized(state.restore_bounds)
    } else {
        WindowBounds::Windowed(current_bounds)
    };
    Some(WindowState {
        handle,
        display_id: state.display_id,
        kind: config.kind,
        bounds,
        viewport_size: state.logical_size,
        minimum_size: config.minimum_size,
        scale_factor: state.scale_factor,
        appearance: state.appearance,
        background_appearance: config.window_background,
        focused: state.focused,
        visible: state.visible,
        minimized,
        maximized,
        fullscreen,
        occluded: state.occluded,
        movable: config.is_movable,
        resizable: config.is_resizable,
        minimizable: config.is_minimizable,
        represented_file: config.represented_file.is_some(),
        document_edited: config.document_edited,
        native_tabbing: config.tabbing_identifier.is_some(),
        native_tabs: state.native_tabs,
        #[cfg(feature = "inspector")]
        inspector_active: state.inspector.is_some(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn external_window_commands_keep_public_validation() {
        assert_eq!(
            validate_window_title(&"x".repeat(MAX_WINDOW_TITLE_BYTES + 1)),
            Err(WindowCommandError::TitleTooLong)
        );
        assert_eq!(
            validate_window_document_path(Path::new("")),
            Err(WindowCommandError::InvalidDocumentPath)
        );
    }
}
