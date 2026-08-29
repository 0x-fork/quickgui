use crate::Result;
#[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
use crate::SystemIntegrationError;

#[cfg(target_os = "linux")]
#[path = "preferences/linux.rs"]
mod platform;
#[cfg(target_os = "macos")]
#[path = "preferences/macos.rs"]
mod platform;
#[cfg(target_os = "windows")]
#[path = "preferences/windows.rs"]
mod platform;
#[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
mod platform {
    use super::*;

    pub(super) fn system_preferences() -> Result<SystemPreferences> {
        Ok(SystemPreferences::default())
    }

    pub(super) fn permission_status(_kind: PermissionKind) -> Result<PermissionStatus> {
        Ok(PermissionStatus::Unknown)
    }

    pub(super) fn request_permission(
        _kind: PermissionKind,
        _completion: impl FnOnce(Result<PermissionStatus>) + Send + 'static,
    ) -> Result<()> {
        Err(SystemIntegrationError::Unsupported)
    }
}

/// The operating system's preferred application color scheme.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub enum ColorScheme {
    Light,
    Dark,
    #[default]
    Unknown,
}

/// Renderer-independent sRGB system color.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct SystemColor {
    pub red: u8,
    pub green: u8,
    pub blue: u8,
    pub alpha: u8,
}

impl SystemColor {
    pub const fn rgba(red: u8, green: u8, blue: u8, alpha: u8) -> Self {
        Self {
            red,
            green,
            blue,
            alpha,
        }
    }

    pub const fn rgb(red: u8, green: u8, blue: u8) -> Self {
        Self::rgba(red, green, blue, 255)
    }
}

/// Semantic operating-system color exposed without retaining a native color object.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[repr(u8)]
pub enum SystemColorRole {
    Accent,
    Highlight,
    HighlightText,
    WindowBackground,
    WindowText,
    ControlBackground,
    ControlText,
    Link,
}

impl SystemColorRole {
    const COUNT: usize = 8;

    const fn index(self) -> usize {
        self as usize
    }
}

/// A point-in-time, renderer-independent system appearance and accessibility snapshot.
///
/// Optional values are `None` when the operating system does not publish an equivalent setting.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub struct SystemPreferences {
    color_scheme: ColorScheme,
    reduce_motion: Option<bool>,
    reduce_transparency: Option<bool>,
    increase_contrast: Option<bool>,
    differentiate_without_color: Option<bool>,
    invert_colors: Option<bool>,
    forced_colors: Option<bool>,
    screen_reader: Option<bool>,
    switch_control: Option<bool>,
    system_colors: [Option<SystemColor>; SystemColorRole::COUNT],
}

impl SystemPreferences {
    pub fn snapshot() -> Result<Self> {
        platform::system_preferences()
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) const fn new(
        color_scheme: ColorScheme,
        reduce_motion: Option<bool>,
        reduce_transparency: Option<bool>,
        increase_contrast: Option<bool>,
        differentiate_without_color: Option<bool>,
        invert_colors: Option<bool>,
        screen_reader: Option<bool>,
        switch_control: Option<bool>,
    ) -> Self {
        Self {
            color_scheme,
            reduce_motion,
            reduce_transparency,
            increase_contrast,
            differentiate_without_color,
            invert_colors,
            forced_colors: None,
            screen_reader,
            switch_control,
            system_colors: [None; SystemColorRole::COUNT],
        }
    }

    pub const fn color_scheme(self) -> ColorScheme {
        self.color_scheme
    }

    pub const fn with_color_scheme(mut self, color_scheme: ColorScheme) -> Self {
        self.color_scheme = color_scheme;
        self
    }

    pub const fn with_reduce_motion(mut self, reduce_motion: Option<bool>) -> Self {
        self.reduce_motion = reduce_motion;
        self
    }

    pub const fn with_reduce_transparency(mut self, reduce_transparency: Option<bool>) -> Self {
        self.reduce_transparency = reduce_transparency;
        self
    }

    pub const fn with_increase_contrast(mut self, increase_contrast: Option<bool>) -> Self {
        self.increase_contrast = increase_contrast;
        self
    }

    pub const fn with_differentiate_without_color(
        mut self,
        differentiate_without_color: Option<bool>,
    ) -> Self {
        self.differentiate_without_color = differentiate_without_color;
        self
    }

    pub const fn with_invert_colors(mut self, invert_colors: Option<bool>) -> Self {
        self.invert_colors = invert_colors;
        self
    }

    pub const fn with_forced_colors(mut self, forced_colors: Option<bool>) -> Self {
        self.forced_colors = forced_colors;
        self
    }

    pub const fn with_system_color(
        mut self,
        role: SystemColorRole,
        color: Option<SystemColor>,
    ) -> Self {
        self.system_colors[role.index()] = color;
        self
    }

    pub const fn with_screen_reader(mut self, screen_reader: Option<bool>) -> Self {
        self.screen_reader = screen_reader;
        self
    }

    pub const fn with_switch_control(mut self, switch_control: Option<bool>) -> Self {
        self.switch_control = switch_control;
        self
    }

    pub const fn reduce_motion(self) -> Option<bool> {
        self.reduce_motion
    }

    pub const fn reduce_transparency(self) -> Option<bool> {
        self.reduce_transparency
    }

    pub const fn increase_contrast(self) -> Option<bool> {
        self.increase_contrast
    }

    pub const fn differentiate_without_color(self) -> Option<bool> {
        self.differentiate_without_color
    }

    pub const fn invert_colors(self) -> Option<bool> {
        self.invert_colors
    }

    pub const fn forced_colors(self) -> Option<bool> {
        self.forced_colors
    }

    pub const fn system_color(self, role: SystemColorRole) -> Option<SystemColor> {
        self.system_colors[role.index()]
    }

    pub const fn accent_color(self) -> Option<SystemColor> {
        self.system_color(SystemColorRole::Accent)
    }

    pub const fn screen_reader(self) -> Option<bool> {
        self.screen_reader
    }

    pub const fn switch_control(self) -> Option<bool> {
        self.switch_control
    }
}

/// A privacy-controlled native capability.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum PermissionKind {
    Camera,
    Microphone,
    ScreenRecording,
    Accessibility,
}

/// Current operating-system authorization state for a native capability.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum PermissionStatus {
    NotDetermined,
    Granted,
    Denied,
    Restricted,
    Unknown,
}

/// Stateless entry point for permission status and explicit user-prompt requests.
pub struct PermissionManager;

impl PermissionManager {
    pub fn status(kind: PermissionKind) -> Result<PermissionStatus> {
        platform::permission_status(kind)
    }

    /// Explicitly request one permission and invoke `completion` exactly once when supported.
    ///
    /// The completion may run inline or on an operating-system callback thread. Camera and
    /// microphone requests require matching usage-description keys in the macOS application
    /// bundle; QuickGUI rejects the request before invoking AVFoundation when they are absent.
    pub fn request(
        kind: PermissionKind,
        completion: impl FnOnce(Result<PermissionStatus>) + Send + 'static,
    ) -> Result<()> {
        platform::request_permission(kind, completion)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unknown_preferences_preserve_unsupported_fields() {
        let preferences = SystemPreferences::default();
        assert_eq!(preferences.color_scheme(), ColorScheme::Unknown);
        assert_eq!(preferences.reduce_motion(), None);
        assert_eq!(preferences.reduce_transparency(), None);
        assert_eq!(preferences.increase_contrast(), None);
        assert_eq!(preferences.differentiate_without_color(), None);
        assert_eq!(preferences.invert_colors(), None);
        assert_eq!(preferences.forced_colors(), None);
        assert_eq!(preferences.accent_color(), None);
        assert_eq!(preferences.screen_reader(), None);
        assert_eq!(preferences.switch_control(), None);
    }
}
