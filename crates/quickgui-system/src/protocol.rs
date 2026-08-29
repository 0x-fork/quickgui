use std::path::PathBuf;

#[cfg(not(any(target_os = "windows", target_os = "linux")))]
use crate::SystemIntegrationError;
use crate::{Result, invalid, platform, validate_text};

pub const MAX_PROTOCOL_ARGUMENTS: usize = 64;
pub const MAX_PROTOCOL_ARGUMENT_BYTES: usize = 8 * 1024;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProtocolRegistrationOptions {
    pub app_name: String,
    /// Stable reverse-DNS-style identifier used for the Linux desktop entry filename.
    pub app_id: String,
    pub executable: PathBuf,
    pub arguments: Vec<String>,
}

impl ProtocolRegistrationOptions {
    pub fn new(app_name: impl Into<String>, app_id: impl Into<String>) -> Result<Self> {
        Ok(Self {
            app_name: app_name.into(),
            app_id: app_id.into(),
            executable: std::env::current_exe().map_err(platform)?,
            arguments: Vec::new(),
        })
    }
}

/// A dynamic URL-scheme registration for Windows or Linux.
///
/// macOS schemes must be declared in the signed application bundle's `CFBundleURLTypes`; runtime
/// registration is deliberately reported as unsupported.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProtocolRegistration {
    scheme: String,
    options: ProtocolRegistrationOptions,
}

impl ProtocolRegistration {
    pub fn new(scheme: impl Into<String>, options: ProtocolRegistrationOptions) -> Result<Self> {
        let scheme = scheme.into().to_ascii_lowercase();
        validate_scheme(&scheme)?;
        validate_options(&options)?;
        Ok(Self { scheme, options })
    }

    pub const fn supports_dynamic_registration() -> bool {
        cfg!(any(target_os = "windows", target_os = "linux"))
    }

    pub fn scheme(&self) -> &str {
        &self.scheme
    }

    pub fn register(&self) -> Result<()> {
        #[cfg(target_os = "windows")]
        return windows::register(self);
        #[cfg(target_os = "linux")]
        return linux::register(self);
        #[cfg(not(any(target_os = "windows", target_os = "linux")))]
        Err(SystemIntegrationError::Unsupported)
    }

    pub fn unregister(&self) -> Result<bool> {
        #[cfg(target_os = "windows")]
        return windows::unregister(self);
        #[cfg(target_os = "linux")]
        return linux::unregister(self);
        #[cfg(not(any(target_os = "windows", target_os = "linux")))]
        Err(SystemIntegrationError::Unsupported)
    }

    pub fn is_registered(&self) -> Result<bool> {
        #[cfg(target_os = "windows")]
        return windows::is_registered(self);
        #[cfg(target_os = "linux")]
        return linux::is_registered(self);
        #[cfg(not(any(target_os = "windows", target_os = "linux")))]
        Err(SystemIntegrationError::Unsupported)
    }
}

fn validate_scheme(scheme: &str) -> Result<()> {
    if scheme.len() > 64
        || !scheme
            .bytes()
            .next()
            .is_some_and(|byte| byte.is_ascii_alphabetic())
        || !scheme
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'+' | b'-' | b'.'))
    {
        return Err(invalid(
            "a URL scheme must follow RFC 3986 and be at most 64 ASCII bytes",
        ));
    }
    Ok(())
}

fn validate_options(options: &ProtocolRegistrationOptions) -> Result<()> {
    validate_text(&options.app_name, "the protocol application name", 256)?;
    if options.app_name.contains(['\r', '\n']) {
        return Err(invalid("the protocol application name must be single-line"));
    }
    validate_text(&options.app_id, "the protocol application id", 1_024)?;
    if options
        .app_id
        .bytes()
        .any(|byte| !(byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-')))
    {
        return Err(invalid(
            "the protocol application id may contain only ASCII letters, digits, dots, underscores, and hyphens",
        ));
    }
    if !options.executable.is_absolute() {
        return Err(invalid("the protocol handler executable must be absolute"));
    }
    if options
        .executable
        .to_str()
        .is_none_or(|path| path.contains(['\0', '\r', '\n']))
    {
        return Err(invalid(
            "the protocol handler executable path must be valid UTF-8, NUL-free, and single-line",
        ));
    }
    if options.arguments.len() > MAX_PROTOCOL_ARGUMENTS {
        return Err(invalid("protocol handlers accept at most 64 arguments"));
    }
    let mut total = 0_usize;
    for argument in &options.arguments {
        if argument.contains('\0') || argument.contains(['\r', '\n']) {
            return Err(invalid(
                "protocol handler arguments must be NUL-free and single-line",
            ));
        }
        total = total.saturating_add(argument.len());
    }
    if total > MAX_PROTOCOL_ARGUMENT_BYTES {
        return Err(invalid(
            "protocol handler arguments cannot exceed 8192 aggregate UTF-8 bytes",
        ));
    }
    Ok(())
}

#[cfg(target_os = "windows")]
mod windows {
    use winreg::{RegKey, enums::HKEY_CURRENT_USER};

    use super::*;

    fn key_path(registration: &ProtocolRegistration) -> String {
        format!("Software\\Classes\\{}", registration.scheme)
    }

    fn command(registration: &ProtocolRegistration) -> Result<String> {
        let executable = dunce::simplified(&registration.options.executable)
            .to_str()
            .ok_or_else(|| invalid("the protocol handler executable must be valid UTF-8"))?;
        let mut parts = Vec::with_capacity(registration.options.arguments.len() + 2);
        parts.push(windows_quote(executable));
        parts.extend(
            registration
                .options
                .arguments
                .iter()
                .map(|argument| windows_quote(argument)),
        );
        parts.push("\"%1\"".to_owned());
        Ok(parts.join(" "))
    }

    pub(super) fn register(registration: &ProtocolRegistration) -> Result<()> {
        let current_user = RegKey::predef(HKEY_CURRENT_USER);
        let path = key_path(registration);
        let (root, _) = current_user.create_subkey(&path).map_err(platform)?;
        root.set_value(
            "",
            &format!("URL:{} protocol", registration.options.app_name),
        )
        .map_err(platform)?;
        root.set_value("URL Protocol", &"").map_err(platform)?;
        let (icon, _) = current_user
            .create_subkey(format!("{path}\\DefaultIcon"))
            .map_err(platform)?;
        icon.set_value(
            "",
            &format!("{},0", registration.options.executable.display()),
        )
        .map_err(platform)?;
        let (open, _) = current_user
            .create_subkey(format!("{path}\\shell\\open\\command"))
            .map_err(platform)?;
        open.set_value("", &command(registration)?)
            .map_err(platform)
    }

    pub(super) fn is_registered(registration: &ProtocolRegistration) -> Result<bool> {
        let current_user = RegKey::predef(HKEY_CURRENT_USER);
        let Ok(open) =
            current_user.open_subkey(format!("{}\\shell\\open\\command", key_path(registration)))
        else {
            return Ok(false);
        };
        let value: String = open.get_value("").map_err(platform)?;
        Ok(value == command(registration)?)
    }

    pub(super) fn unregister(registration: &ProtocolRegistration) -> Result<bool> {
        if !is_registered(registration)? {
            return Ok(false);
        }
        let current_user = RegKey::predef(HKEY_CURRENT_USER);
        current_user
            .delete_subkey_all(key_path(registration))
            .map_err(platform)?;
        Ok(true)
    }

    fn windows_quote(value: &str) -> String {
        if !value.is_empty()
            && !value
                .bytes()
                .any(|byte| matches!(byte, b' ' | b'\t' | b'\n' | b'\x0b' | b'"'))
        {
            return value.to_owned();
        }
        let mut quoted = String::from("\"");
        let mut backslashes = 0;
        for character in value.chars() {
            if character == '\\' {
                backslashes += 1;
            } else {
                if character == '"' {
                    quoted.extend(std::iter::repeat_n('\\', backslashes * 2 + 1));
                } else {
                    quoted.extend(std::iter::repeat_n('\\', backslashes));
                }
                backslashes = 0;
                quoted.push(character);
            }
        }
        quoted.extend(std::iter::repeat_n('\\', backslashes * 2));
        quoted.push('"');
        quoted
    }
}

#[cfg(target_os = "linux")]
mod linux {
    use std::{fs, process::Command};

    use ini::Ini;

    use super::*;

    fn applications_dir() -> Result<PathBuf> {
        dirs::data_dir()
            .map(|path| path.join("applications"))
            .ok_or_else(|| platform("the XDG data directory is unavailable"))
    }

    fn file_name(registration: &ProtocolRegistration) -> String {
        format!("{}-url-handler.desktop", registration.options.app_id)
    }

    fn desktop_path(registration: &ProtocolRegistration) -> Result<PathBuf> {
        Ok(applications_dir()?.join(file_name(registration)))
    }

    fn mime(registration: &ProtocolRegistration) -> String {
        format!("x-scheme-handler/{}", registration.scheme)
    }

    pub(super) fn register(registration: &ProtocolRegistration) -> Result<()> {
        let directory = applications_dir()?;
        fs::create_dir_all(&directory).map_err(platform)?;
        let path = desktop_path(registration)?;
        let mut desktop = Ini::load_from_file(&path).unwrap_or_default();
        let mut section = desktop.with_section(Some("Desktop Entry"));
        section.set("Type", "Application");
        section.set("Name", registration.options.app_name.clone());
        section.set("Exec", desktop_command(registration)?);
        section.set("Terminal", "false");
        section.set("NoDisplay", "true");
        let mime = mime(registration);
        let mut mimes = section
            .get("MimeType")
            .unwrap_or_default()
            .split(';')
            .filter(|value| !value.is_empty())
            .map(str::to_owned)
            .collect::<Vec<_>>();
        if !mimes.contains(&mime) {
            mimes.push(mime.clone());
        }
        section.set("MimeType", format!("{};", mimes.join(";")));
        desktop.write_to_file(&path).map_err(platform)?;
        run_command("update-desktop-database", &[directory.as_os_str()])?;
        run_command(
            "xdg-mime",
            &[
                std::ffi::OsStr::new("default"),
                std::ffi::OsStr::new(&file_name(registration)),
                std::ffi::OsStr::new(&mime),
            ],
        )?;
        Ok(())
    }

    pub(super) fn is_registered(registration: &ProtocolRegistration) -> Result<bool> {
        let output = Command::new("xdg-mime")
            .args(["query", "default", &mime(registration)])
            .output()
            .map_err(platform)?;
        if !output.status.success() {
            return Err(platform(format!(
                "xdg-mime exited with status {}",
                output.status
            )));
        }
        Ok(String::from_utf8_lossy(&output.stdout).trim() == file_name(registration))
    }

    pub(super) fn unregister(registration: &ProtocolRegistration) -> Result<bool> {
        if !is_registered(registration)? {
            return Ok(false);
        }
        let path = desktop_path(registration)?;
        if let Ok(mut desktop) = Ini::load_from_file(&path) {
            let mut remove_file = true;
            if let Some(section) = desktop.section_mut(Some("Desktop Entry")) {
                let mime = mime(registration);
                let mimes = section
                    .get("MimeType")
                    .unwrap_or_default()
                    .split(';')
                    .filter(|value| !value.is_empty() && *value != mime)
                    .collect::<Vec<_>>();
                if !mimes.is_empty() {
                    section.insert("MimeType".to_owned(), format!("{};", mimes.join(";")));
                    remove_file = false;
                }
            }
            if remove_file {
                fs::remove_file(&path).map_err(platform)?;
            } else {
                desktop.write_to_file(&path).map_err(platform)?;
            }
        }
        remove_mimeapps_default(registration)?;
        run_command(
            "update-desktop-database",
            &[applications_dir()?.as_os_str()],
        )?;
        Ok(true)
    }

    fn remove_mimeapps_default(registration: &ProtocolRegistration) -> Result<()> {
        let Some(config) = dirs::config_dir() else {
            return Ok(());
        };
        let path = config.join("mimeapps.list");
        let Ok(mut mimeapps) = Ini::load_from_file(&path) else {
            return Ok(());
        };
        let mime = mime(registration);
        let file = file_name(registration);
        let mut changed = false;
        for name in ["Default Applications", "Added Associations"] {
            if let Some(section) = mimeapps.section_mut(Some(name)) {
                let values = section
                    .get(&mime)
                    .unwrap_or_default()
                    .split(';')
                    .filter(|value| !value.is_empty() && *value != file)
                    .collect::<Vec<_>>();
                if values.is_empty() {
                    changed |= section.remove(&mime).is_some();
                } else {
                    section.insert(mime.clone(), format!("{};", values.join(";")));
                    changed = true;
                }
            }
        }
        if changed {
            mimeapps.write_to_file(path).map_err(platform)?;
        }
        Ok(())
    }

    fn desktop_command(registration: &ProtocolRegistration) -> Result<String> {
        let executable = registration
            .options
            .executable
            .to_str()
            .ok_or_else(|| invalid("the protocol handler executable must be valid UTF-8"))?;
        let mut command = desktop_quote(executable);
        for argument in &registration.options.arguments {
            command.push(' ');
            command.push_str(&desktop_quote(argument));
        }
        command.push_str(" %u");
        Ok(command)
    }

    fn desktop_quote(value: &str) -> String {
        let escaped = value
            .replace('\\', "\\\\")
            .replace('"', "\\\"")
            .replace('`', "\\`")
            .replace('$', "\\$")
            .replace('%', "%%");
        format!("\"{escaped}\"")
    }

    fn run_command(program: &str, arguments: &[&std::ffi::OsStr]) -> Result<()> {
        let status = Command::new(program)
            .args(arguments)
            .status()
            .map_err(platform)?;
        if !status.success() {
            return Err(platform(format!("{program} exited with status {status}")));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validates_rfc_scheme_shape() {
        assert!(validate_scheme("quickgui+preview").is_ok());
        assert!(validate_scheme("1quickgui").is_err());
        assert!(validate_scheme("quick_gui").is_err());
    }
}
