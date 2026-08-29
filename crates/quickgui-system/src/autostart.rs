use std::path::PathBuf;

#[cfg(target_os = "linux")]
use auto_launch::LinuxLaunchMode;
#[cfg(target_os = "macos")]
use auto_launch::MacOSLaunchMode;
#[cfg(target_os = "windows")]
use auto_launch::WindowsEnableMode;
use auto_launch::{AutoLaunch, AutoLaunchBuilder};

#[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
use crate::SystemIntegrationError;
use crate::{Result, invalid, platform, validate_text};

pub const MAX_AUTOSTART_ARGUMENTS: usize = 64;
pub const MAX_AUTOSTART_ARGUMENT_BYTES: usize = 8 * 1024;

/// Explicit platform strategy for login launch registration.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub enum AutoStartMode {
    /// SMAppService on macOS, the current-user Run key on Windows, and XDG Autostart on Linux.
    #[default]
    Native,
    MacOsLaunchAgent,
    MacOsAppleScript,
    LinuxSystemd,
    WindowsSystem,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AutoStartOptions {
    pub app_name: String,
    pub executable: PathBuf,
    pub arguments: Vec<String>,
    pub mode: AutoStartMode,
    pub bundle_identifier: Option<String>,
}

impl AutoStartOptions {
    pub fn new(app_name: impl Into<String>) -> Result<Self> {
        let executable = std::env::current_exe().map_err(platform)?;
        Ok(Self {
            app_name: app_name.into(),
            executable,
            arguments: Vec::new(),
            mode: AutoStartMode::Native,
            bundle_identifier: None,
        })
    }
}

/// A reusable configuration for the user's native login-launch facility.
#[derive(Clone, Debug)]
pub struct AutoStart {
    native: AutoLaunch,
}

impl AutoStart {
    pub fn new(options: AutoStartOptions) -> Result<Self> {
        validate_options(&options)?;
        let executable = options
            .executable
            .to_str()
            .ok_or_else(|| invalid("the autostart executable path must be valid UTF-8"))?;
        let mut builder = AutoLaunchBuilder::new();
        builder
            .set_app_name(&options.app_name)
            .set_app_path(executable)
            .set_args(&options.arguments);

        #[cfg(target_os = "macos")]
        {
            let mode = match options.mode {
                AutoStartMode::Native => MacOSLaunchMode::SMAppService,
                AutoStartMode::MacOsLaunchAgent => MacOSLaunchMode::LaunchAgent,
                AutoStartMode::MacOsAppleScript => MacOSLaunchMode::AppleScript,
                _ => {
                    return Err(invalid(
                        "the requested autostart mode does not apply to macOS",
                    ));
                }
            };
            builder.set_macos_launch_mode(mode);
            if let Some(identifier) = &options.bundle_identifier {
                builder.set_bundle_identifiers(&[identifier]);
            }
        }
        #[cfg(target_os = "windows")]
        builder.set_windows_enable_mode(match options.mode {
            AutoStartMode::Native => WindowsEnableMode::CurrentUser,
            AutoStartMode::WindowsSystem => WindowsEnableMode::System,
            _ => {
                return Err(invalid(
                    "the requested autostart mode does not apply to Windows",
                ));
            }
        });
        #[cfg(target_os = "linux")]
        builder.set_linux_launch_mode(match options.mode {
            AutoStartMode::Native => LinuxLaunchMode::XdgAutostart,
            AutoStartMode::LinuxSystemd => LinuxLaunchMode::Systemd,
            _ => {
                return Err(invalid(
                    "the requested autostart mode does not apply to Linux",
                ));
            }
        });
        #[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
        return Err(SystemIntegrationError::Unsupported);

        builder
            .build()
            .map(|native| Self { native })
            .map_err(platform)
    }

    pub const fn is_supported() -> bool {
        cfg!(any(
            target_os = "macos",
            target_os = "windows",
            target_os = "linux"
        ))
    }

    pub fn enable(&self) -> Result<()> {
        self.native.enable().map_err(platform)
    }

    pub fn disable(&self) -> Result<()> {
        self.native.disable().map_err(platform)
    }

    pub fn is_enabled(&self) -> Result<bool> {
        self.native.is_enabled().map_err(platform)
    }
}

fn validate_options(options: &AutoStartOptions) -> Result<()> {
    validate_text(&options.app_name, "the autostart application name", 128)?;
    if !options
        .app_name
        .bytes()
        .next()
        .is_some_and(|byte| byte.is_ascii_alphanumeric())
        || !options
            .app_name
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-'))
    {
        return Err(invalid(
            "the autostart application name must be a portable identifier containing only ASCII letters, digits, dots, underscores, and hyphens",
        ));
    }
    if !options.executable.is_absolute() {
        return Err(invalid("the autostart executable path must be absolute"));
    }
    if options
        .executable
        .to_str()
        .is_none_or(|path| path.contains(['\0', '\r', '\n']))
    {
        return Err(invalid(
            "the autostart executable path must be valid UTF-8, NUL-free, and single-line",
        ));
    }
    if options.arguments.len() > MAX_AUTOSTART_ARGUMENTS {
        return Err(invalid("autostart accepts at most 64 arguments"));
    }
    let mut bytes = 0_usize;
    for argument in &options.arguments {
        if argument.contains('\0') {
            return Err(invalid("autostart arguments must be NUL-free"));
        }
        bytes = bytes.saturating_add(argument.len());
    }
    if bytes > MAX_AUTOSTART_ARGUMENT_BYTES {
        return Err(invalid(
            "autostart arguments cannot exceed 8192 aggregate UTF-8 bytes",
        ));
    }
    if let Some(identifier) = &options.bundle_identifier {
        validate_text(identifier, "the bundle identifier", 1_024)?;
        if identifier.contains(['\r', '\n']) {
            return Err(invalid("the bundle identifier must be single-line"));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_relative_executable_and_unbounded_arguments() {
        let invalid_path = AutoStartOptions {
            app_name: "Example".to_owned(),
            executable: PathBuf::from("example"),
            arguments: Vec::new(),
            mode: AutoStartMode::Native,
            bundle_identifier: None,
        };
        assert!(validate_options(&invalid_path).is_err());

        let mut too_many = invalid_path;
        too_many.executable = std::env::current_exe().unwrap();
        too_many.arguments = vec![String::new(); MAX_AUTOSTART_ARGUMENTS + 1];
        assert!(validate_options(&too_many).is_err());
    }

    #[test]
    fn rejects_unsafe_registration_names() {
        for name in ["../Example", "Example App", "Example\nHidden"] {
            let options = AutoStartOptions {
                app_name: name.to_owned(),
                executable: std::env::current_exe().unwrap(),
                arguments: Vec::new(),
                mode: AutoStartMode::Native,
                bundle_identifier: None,
            };
            assert!(validate_options(&options).is_err(), "accepted {name:?}");
        }
    }
}
