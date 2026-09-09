use std::{
    collections::HashSet,
    env,
    path::{Path, PathBuf},
    sync::Arc,
};

use crate::{Result, invalid, platform};

pub const MAX_APP_NAME_BYTES: usize = 256;
pub const MAX_APP_VERSION_BYTES: usize = 128;
pub const MAX_APP_IDENTIFIER_BYTES: usize = 1_024;
pub const MAX_SYSTEM_TEXT_BYTES: usize = 1_024;
pub const MAX_SYSTEM_LOCALE_BYTES: usize = 256;
pub const MAX_SYSTEM_LOCALES_TOTAL_BYTES: usize = 16 * 1_024;
pub const MAX_PREFERRED_LANGUAGES: usize = 64;

/// Immutable package identity supplied by the application at startup.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct AppInfo {
    name: Arc<str>,
    version: Arc<str>,
    identifier: Arc<str>,
}

impl AppInfo {
    pub fn new(
        name: impl Into<Arc<str>>,
        version: impl Into<Arc<str>>,
        identifier: impl Into<Arc<str>>,
    ) -> Result<Self> {
        let name = name.into();
        let version = version.into();
        let identifier = identifier.into();
        validate_single_line(&name, "application name", MAX_APP_NAME_BYTES)?;
        validate_single_line(&version, "application version", MAX_APP_VERSION_BYTES)?;
        validate_app_identifier(&identifier)?;
        Ok(Self {
            name,
            version,
            identifier,
        })
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn version(&self) -> &str {
        &self.version
    }

    /// Stable reverse-domain or similarly portable identifier used to scope application paths.
    pub fn identifier(&self) -> &str {
        &self.identifier
    }

    /// Resolve the current process's standard application paths using this identity.
    pub fn paths(&self) -> Result<AppPaths> {
        AppPaths::resolve(self.identifier())
    }
}

/// Immutable snapshot of standard process, application, and user directories.
///
/// Application-owned directories are scoped by the package identifier. User directories and the
/// operating-system temporary directory are not created or otherwise mutated during resolution.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AppPaths(Arc<AppPathSnapshot>);

#[derive(Clone, Debug, Eq, PartialEq)]
struct AppPathSnapshot {
    executable: PathBuf,
    executable_dir: PathBuf,
    resource_dir: PathBuf,
    home_dir: Option<PathBuf>,
    config_dir: Option<PathBuf>,
    data_dir: Option<PathBuf>,
    local_data_dir: Option<PathBuf>,
    cache_dir: Option<PathBuf>,
    log_dir: Option<PathBuf>,
    runtime_dir: Option<PathBuf>,
    temp_dir: PathBuf,
    audio_dir: Option<PathBuf>,
    desktop_dir: Option<PathBuf>,
    document_dir: Option<PathBuf>,
    download_dir: Option<PathBuf>,
    picture_dir: Option<PathBuf>,
    video_dir: Option<PathBuf>,
}

impl AppPaths {
    /// Resolve paths once for the current process without creating any directories.
    pub fn resolve(identifier: &str) -> Result<Self> {
        validate_app_identifier(identifier)?;
        let executable = env::current_exe().map_err(platform)?;
        Self::from_executable(identifier, executable)
    }

    fn from_executable(identifier: &str, executable: PathBuf) -> Result<Self> {
        let executable_dir = executable
            .parent()
            .filter(|path| !path.as_os_str().is_empty())
            .ok_or_else(|| invalid("the current executable does not have a parent directory"))?
            .to_path_buf();
        let resource_dir = resource_dir_for_executable(&executable);
        let home_dir = dirs::home_dir();
        let config_dir = scoped(dirs::config_dir(), identifier);
        let data_dir = scoped(dirs::data_dir(), identifier);
        let local_data_dir = scoped(dirs::data_local_dir(), identifier);
        let cache_dir = scoped(dirs::cache_dir(), identifier);
        let runtime_dir = scoped(dirs::runtime_dir(), identifier);
        let log_dir = log_dir(identifier, home_dir.as_deref(), local_data_dir.as_deref());
        Ok(Self(Arc::new(AppPathSnapshot {
            executable,
            executable_dir,
            resource_dir,
            home_dir,
            config_dir,
            data_dir,
            local_data_dir,
            cache_dir,
            log_dir,
            runtime_dir,
            temp_dir: env::temp_dir(),
            audio_dir: dirs::audio_dir(),
            desktop_dir: dirs::desktop_dir(),
            document_dir: dirs::document_dir(),
            download_dir: dirs::download_dir(),
            picture_dir: dirs::picture_dir(),
            video_dir: dirs::video_dir(),
        })))
    }

    pub fn executable(&self) -> &Path {
        &self.0.executable
    }

    pub fn executable_dir(&self) -> &Path {
        &self.0.executable_dir
    }

    pub fn resource_dir(&self) -> &Path {
        &self.0.resource_dir
    }

    pub fn home_dir(&self) -> Option<&Path> {
        self.0.home_dir.as_deref()
    }

    pub fn config_dir(&self) -> Option<&Path> {
        self.0.config_dir.as_deref()
    }

    pub fn data_dir(&self) -> Option<&Path> {
        self.0.data_dir.as_deref()
    }

    pub fn local_data_dir(&self) -> Option<&Path> {
        self.0.local_data_dir.as_deref()
    }

    pub fn cache_dir(&self) -> Option<&Path> {
        self.0.cache_dir.as_deref()
    }

    pub fn log_dir(&self) -> Option<&Path> {
        self.0.log_dir.as_deref()
    }

    pub fn runtime_dir(&self) -> Option<&Path> {
        self.0.runtime_dir.as_deref()
    }

    pub fn temp_dir(&self) -> &Path {
        &self.0.temp_dir
    }

    pub fn audio_dir(&self) -> Option<&Path> {
        self.0.audio_dir.as_deref()
    }

    pub fn desktop_dir(&self) -> Option<&Path> {
        self.0.desktop_dir.as_deref()
    }

    pub fn document_dir(&self) -> Option<&Path> {
        self.0.document_dir.as_deref()
    }

    pub fn download_dir(&self) -> Option<&Path> {
        self.0.download_dir.as_deref()
    }

    pub fn picture_dir(&self) -> Option<&Path> {
        self.0.picture_dir.as_deref()
    }

    pub fn video_dir(&self) -> Option<&Path> {
        self.0.video_dir.as_deref()
    }

    pub fn with_resource_dir(mut self, path: impl Into<PathBuf>) -> Self {
        Arc::make_mut(&mut self.0).resource_dir = path.into();
        self
    }

    pub fn with_config_dir(mut self, path: Option<impl Into<PathBuf>>) -> Self {
        Arc::make_mut(&mut self.0).config_dir = path.map(Into::into);
        self
    }

    pub fn with_data_dir(mut self, path: Option<impl Into<PathBuf>>) -> Self {
        Arc::make_mut(&mut self.0).data_dir = path.map(Into::into);
        self
    }

    pub fn with_local_data_dir(mut self, path: Option<impl Into<PathBuf>>) -> Self {
        Arc::make_mut(&mut self.0).local_data_dir = path.map(Into::into);
        self
    }

    pub fn with_cache_dir(mut self, path: Option<impl Into<PathBuf>>) -> Self {
        Arc::make_mut(&mut self.0).cache_dir = path.map(Into::into);
        self
    }

    pub fn with_log_dir(mut self, path: Option<impl Into<PathBuf>>) -> Self {
        Arc::make_mut(&mut self.0).log_dir = path.map(Into::into);
        self
    }

    pub fn with_runtime_dir(mut self, path: Option<impl Into<PathBuf>>) -> Self {
        Arc::make_mut(&mut self.0).runtime_dir = path.map(Into::into);
        self
    }

    pub fn with_temp_dir(mut self, path: impl Into<PathBuf>) -> Self {
        Arc::make_mut(&mut self.0).temp_dir = path.into();
        self
    }
}

fn scoped(base: Option<PathBuf>, identifier: &str) -> Option<PathBuf> {
    base.map(|base| base.join(identifier))
}

#[cfg(target_os = "macos")]
fn log_dir(identifier: &str, home: Option<&Path>, _local_data: Option<&Path>) -> Option<PathBuf> {
    home.map(|home| home.join("Library").join("Logs").join(identifier))
}

#[cfg(not(target_os = "macos"))]
fn log_dir(identifier: &str, _home: Option<&Path>, local_data: Option<&Path>) -> Option<PathBuf> {
    local_data.map(|path| {
        debug_assert_eq!(path.file_name(), Some(identifier.as_ref()));
        path.join("logs")
    })
}

fn resource_dir_for_executable(executable: &Path) -> PathBuf {
    let executable_dir = executable.parent().unwrap_or_else(|| Path::new("."));
    #[cfg(target_os = "macos")]
    if executable_dir
        .file_name()
        .is_some_and(|name| name == "MacOS")
        && let Some(contents) = executable_dir.parent()
        && contents.file_name().is_some_and(|name| name == "Contents")
    {
        return contents.join("Resources");
    }
    executable_dir.to_path_buf()
}

fn validate_single_line(value: &str, name: &str, maximum: usize) -> Result<()> {
    if value.is_empty()
        || value.contains(['\0', '\n', '\r'])
        || value.len() > maximum
        || value.trim() != value
    {
        return Err(invalid(format!(
            "{name} must be nonempty, single-line, NUL-free, unpadded, and at most {maximum} UTF-8 bytes"
        )));
    }
    Ok(())
}

fn validate_app_identifier(identifier: &str) -> Result<()> {
    validate_single_line(
        identifier,
        "application identifier",
        MAX_APP_IDENTIFIER_BYTES,
    )?;
    if matches!(identifier, "." | "..")
        || !identifier
            .as_bytes()
            .first()
            .is_some_and(u8::is_ascii_alphanumeric)
        || !identifier
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-'))
    {
        return Err(invalid(
            "application identifier must start with an ASCII letter or digit and contain only ASCII letters, digits, '.', '_', or '-'",
        ));
    }
    Ok(())
}

/// Compile-time operating-system family used by the running process.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum OperatingSystemFamily {
    Unix,
    Windows,
    Wasm,
    Other,
}

impl OperatingSystemFamily {
    pub const fn current() -> Self {
        #[cfg(target_arch = "wasm32")]
        return Self::Wasm;
        #[cfg(all(not(target_arch = "wasm32"), windows))]
        return Self::Windows;
        #[cfg(all(not(target_arch = "wasm32"), unix))]
        return Self::Unix;
        #[cfg(not(any(target_arch = "wasm32", windows, unix)))]
        return Self::Other;
    }

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Unix => "unix",
            Self::Windows => "windows",
            Self::Wasm => "wasm",
            Self::Other => "other",
        }
    }
}

/// Compile-time operating system used by the running process.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum OperatingSystem {
    Macos,
    Windows,
    Linux,
    FreeBsd,
    DragonFly,
    NetBsd,
    OpenBsd,
    Android,
    Ios,
    Wasm,
    Other,
}

impl OperatingSystem {
    pub const fn current() -> Self {
        #[cfg(target_os = "macos")]
        return Self::Macos;
        #[cfg(target_os = "windows")]
        return Self::Windows;
        #[cfg(target_os = "linux")]
        return Self::Linux;
        #[cfg(target_os = "freebsd")]
        return Self::FreeBsd;
        #[cfg(target_os = "dragonfly")]
        return Self::DragonFly;
        #[cfg(target_os = "netbsd")]
        return Self::NetBsd;
        #[cfg(target_os = "openbsd")]
        return Self::OpenBsd;
        #[cfg(target_os = "android")]
        return Self::Android;
        #[cfg(target_os = "ios")]
        return Self::Ios;
        #[cfg(all(target_arch = "wasm32", not(target_os = "emscripten")))]
        return Self::Wasm;
        #[cfg(not(any(
            target_os = "macos",
            target_os = "windows",
            target_os = "linux",
            target_os = "freebsd",
            target_os = "dragonfly",
            target_os = "netbsd",
            target_os = "openbsd",
            target_os = "android",
            target_os = "ios",
            all(target_arch = "wasm32", not(target_os = "emscripten"))
        )))]
        return Self::Other;
    }

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Macos => "macos",
            Self::Windows => "windows",
            Self::Linux => "linux",
            Self::FreeBsd => "freebsd",
            Self::DragonFly => "dragonfly",
            Self::NetBsd => "netbsd",
            Self::OpenBsd => "openbsd",
            Self::Android => "android",
            Self::Ios => "ios",
            Self::Wasm => "wasm",
            Self::Other => "other",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum SystemBitness {
    X32,
    X64,
    Unknown,
}

/// Bounded immutable operating-system and language snapshot.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SystemInfo {
    operating_system: OperatingSystem,
    family: OperatingSystemFamily,
    name: Arc<str>,
    version: Option<Arc<str>>,
    edition: Option<Arc<str>>,
    codename: Option<Arc<str>>,
    architecture: Arc<str>,
    bitness: SystemBitness,
    hostname: Option<Arc<str>>,
    locale: Option<Arc<str>>,
    preferred_languages: Arc<[Arc<str>]>,
    languages_truncated: bool,
}

impl SystemInfo {
    /// Query the operating system once and return a stable snapshot.
    pub fn current() -> Self {
        let info = os_info::get();
        let name =
            bounded_text(&info.os_type().to_string()).unwrap_or_else(|| Arc::from(env::consts::OS));
        let version_text = info.version().to_string();
        let version = (version_text != "unknown")
            .then(|| bounded_text(&version_text))
            .flatten();
        let architecture = info
            .architecture()
            .and_then(bounded_text)
            .unwrap_or_else(|| Arc::from(env::consts::ARCH));
        let bitness = match info.bitness() {
            os_info::Bitness::X32 => SystemBitness::X32,
            os_info::Bitness::X64 => SystemBitness::X64,
            _ => SystemBitness::Unknown,
        };
        #[cfg(not(target_arch = "wasm32"))]
        let hostname = hostname::get()
            .ok()
            .and_then(|value| value.into_string().ok())
            .and_then(|value| bounded_text(&value));
        #[cfg(target_arch = "wasm32")]
        let hostname = None;
        let (preferred_languages, languages_truncated) = preferred_languages();
        let locale = preferred_languages.first().cloned();
        Self {
            operating_system: OperatingSystem::current(),
            family: OperatingSystemFamily::current(),
            name,
            version,
            edition: info.edition().and_then(bounded_text),
            codename: info.codename().and_then(bounded_text),
            architecture,
            bitness,
            hostname,
            locale,
            preferred_languages: preferred_languages.into(),
            languages_truncated,
        }
    }

    pub const fn operating_system(&self) -> OperatingSystem {
        self.operating_system
    }

    pub const fn family(&self) -> OperatingSystemFamily {
        self.family
    }

    /// Platform or distribution name reported by the operating system.
    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn version(&self) -> Option<&str> {
        self.version.as_deref()
    }

    pub fn edition(&self) -> Option<&str> {
        self.edition.as_deref()
    }

    pub fn codename(&self) -> Option<&str> {
        self.codename.as_deref()
    }

    pub fn architecture(&self) -> &str {
        &self.architecture
    }

    pub const fn bitness(&self) -> SystemBitness {
        self.bitness
    }

    pub fn hostname(&self) -> Option<&str> {
        self.hostname.as_deref()
    }

    /// Most preferred BCP-47 locale, when the platform reports one.
    pub fn locale(&self) -> Option<&str> {
        self.locale.as_deref()
    }

    /// Preferred BCP-47 languages in platform priority order.
    pub fn preferred_languages(&self) -> &[Arc<str>] {
        &self.preferred_languages
    }

    pub const fn languages_truncated(&self) -> bool {
        self.languages_truncated
    }
}

impl Default for SystemInfo {
    fn default() -> Self {
        Self::current()
    }
}

fn preferred_languages() -> (Vec<Arc<str>>, bool) {
    let mut languages = Vec::with_capacity(4);
    let mut seen = HashSet::with_capacity(4);
    let mut total_bytes = 0;
    let mut truncated = false;
    for language in sys_locale::get_locales() {
        let language = language.replace('_', "-");
        let Some(language) = bounded_locale(&language) else {
            truncated = true;
            continue;
        };
        if !seen.insert(language.clone()) {
            continue;
        }
        if languages.len() == MAX_PREFERRED_LANGUAGES
            || total_bytes + language.len() > MAX_SYSTEM_LOCALES_TOTAL_BYTES
        {
            truncated = true;
            break;
        }
        total_bytes += language.len();
        languages.push(Arc::from(language));
    }
    (languages, truncated)
}

fn bounded_locale(value: &str) -> Option<String> {
    let value = value.trim();
    if value.is_empty()
        || value.len() > MAX_SYSTEM_LOCALE_BYTES
        || value.contains(['\0', '\n', '\r'])
    {
        return None;
    }
    Some(value.to_owned())
}

fn bounded_text(value: &str) -> Option<Arc<str>> {
    let value = value.trim();
    if value.is_empty() || value.len() > MAX_SYSTEM_TEXT_BYTES || value.contains(['\0', '\n', '\r'])
    {
        return None;
    }
    Some(Arc::from(value))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validates_package_identity() {
        let info = AppInfo::new("QuickGUI", "1.2.3", "dev.quickgui.example").unwrap();
        assert_eq!(info.name(), "QuickGUI");
        assert_eq!(info.version(), "1.2.3");
        assert_eq!(info.identifier(), "dev.quickgui.example");
        assert!(AppInfo::new("QuickGUI", "1", "../escape").is_err());
        assert!(AppInfo::new(" padded ", "1", "dev.quickgui.example").is_err());
        assert!(AppInfo::new("QuickGUI", "1\n2", "dev.quickgui.example").is_err());
    }

    #[test]
    fn scopes_application_directories_without_creating_them() {
        let paths = AppPaths::from_executable(
            "dev.quickgui.example",
            PathBuf::from("/opt/QuickGUI/bin/example"),
        )
        .unwrap();
        assert_eq!(paths.executable_dir(), Path::new("/opt/QuickGUI/bin"));
        assert_eq!(paths.resource_dir(), Path::new("/opt/QuickGUI/bin"));
        for path in [
            paths.config_dir(),
            paths.data_dir(),
            paths.local_data_dir(),
            paths.cache_dir(),
            paths.runtime_dir(),
        ]
        .into_iter()
        .flatten()
        {
            assert_eq!(path.file_name().unwrap(), "dev.quickgui.example");
        }
    }

    #[test]
    fn path_overrides_are_explicit_and_immutable() {
        let original = AppPaths::from_executable(
            "dev.quickgui.example",
            PathBuf::from("/opt/QuickGUI/example"),
        )
        .unwrap();
        let shared = original.clone();
        assert!(Arc::ptr_eq(&original.0, &shared.0));
        let paths = shared
            .with_config_dir(Some("/tmp/config"))
            .with_resource_dir("/tmp/resources")
            .with_temp_dir("/tmp/custom");
        assert!(!Arc::ptr_eq(&original.0, &paths.0));
        assert_eq!(original.resource_dir(), Path::new("/opt/QuickGUI"));
        assert_eq!(paths.config_dir(), Some(Path::new("/tmp/config")));
        assert_eq!(paths.resource_dir(), Path::new("/tmp/resources"));
        assert_eq!(paths.temp_dir(), Path::new("/tmp/custom"));
    }

    #[test]
    fn current_system_snapshot_is_bounded_and_consistent() {
        let info = SystemInfo::current();
        assert!(!info.name().is_empty());
        assert!(!info.architecture().is_empty());
        assert_eq!(info.operating_system(), OperatingSystem::current());
        assert_eq!(info.family(), OperatingSystemFamily::current());
        assert_eq!(
            info.locale(),
            info.preferred_languages().first().map(AsRef::as_ref)
        );
        assert!(
            info.preferred_languages()
                .iter()
                .all(|language| language.len() <= MAX_SYSTEM_LOCALE_BYTES)
        );
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn resolves_macos_bundle_resources() {
        let executable = Path::new("/Applications/Example.app/Contents/MacOS/example");
        assert_eq!(
            resource_dir_for_executable(executable),
            Path::new("/Applications/Example.app/Contents/Resources")
        );
    }
}
