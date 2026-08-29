use std::{
    collections::HashMap,
    ffi::OsString,
    fs::{self, File},
    io::{Read, Write},
    path::{Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};

#[cfg(any(target_os = "windows", target_os = "linux"))]
use std::ffi::OsStr;
#[cfg(target_os = "windows")]
use std::process::{Command, Stdio};

use base64::Engine;
use minisign_verify::{PublicKey, Signature};
use semver::Version;
use serde::Deserialize;

use crate::{Result, SystemIntegrationError, invalid};

pub const MAX_UPDATE_MANIFEST_BYTES: usize = 1024 * 1024;
pub const MAX_UPDATE_SIGNATURE_BYTES: usize = 64 * 1024;
pub const DEFAULT_MAX_UPDATE_BYTES: u64 = 2 * 1024 * 1024 * 1024;
pub const DEFAULT_MAX_EXPANDED_UPDATE_BYTES: u64 = 8 * 1024 * 1024 * 1024;
pub const MAX_UPDATE_ARCHIVE_ENTRIES: usize = 131_072;
pub const MAX_UPDATE_INSTALLER_ARGUMENTS: usize = 128;
pub const MAX_UPDATE_INSTALLER_ARGUMENT_BYTES: usize = 128 * 1024;

/// Thread-safe cooperative cancellation for updater network, verification, and staging work.
///
/// Cancellation is observed between bounded I/O chunks and before any irreversible installer
/// launch or atomic replacement. Once an atomic replacement has started, it finishes or rolls
/// back instead of leaving a partially installed application.
#[derive(Clone, Debug, Default)]
pub struct UpdateCancellation(Arc<AtomicBool>);

impl UpdateCancellation {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn cancel(&self) {
        self.0.store(true, Ordering::Release);
    }

    pub fn is_cancelled(&self) -> bool {
        self.0.load(Ordering::Acquire)
    }

    fn check(&self) -> Result<()> {
        if self.is_cancelled() {
            Err(SystemIntegrationError::Cancelled)
        } else {
            Ok(())
        }
    }
}

/// One newer, target-specific artifact selected from signed updater metadata.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AvailableUpdate {
    pub version: String,
    pub current_version: String,
    pub target: String,
    pub url: String,
    pub signature: String,
    pub notes: Option<String>,
    pub published_at: Option<String>,
}

/// Bounded milestones emitted by download, verification, and installation operations.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum UpdateProgress {
    DownloadStarted {
        total_bytes: Option<u64>,
    },
    Downloaded {
        chunk_bytes: usize,
        downloaded_bytes: u64,
        total_bytes: Option<u64>,
    },
    DownloadFinished {
        downloaded_bytes: u64,
    },
    VerificationStarted,
    VerificationFinished,
    Staged {
        path: PathBuf,
    },
    InstallStarted {
        artifact: PathBuf,
    },
    InstallFinished {
        installation: InstalledUpdate,
    },
}

/// Native Windows installer presentation policy.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub enum WindowsUpdateInstallMode {
    BasicUi,
    Quiet,
    #[default]
    Passive,
}

impl WindowsUpdateInstallMode {
    pub const fn msi_arguments(self) -> &'static [&'static str] {
        match self {
            Self::BasicUi => &["/qb+"],
            Self::Quiet => &["/quiet"],
            Self::Passive => &["/passive"],
        }
    }

    pub const fn executable_arguments(self) -> &'static [&'static str] {
        match self {
            Self::BasicUi => &[],
            Self::Quiet => &["/S", "/R"],
            Self::Passive => &["/P", "/R"],
        }
    }
}

/// Installation overrides for a verified staged artifact.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UpdateInstallOptions {
    target_executable: Option<PathBuf>,
    retain_backup: bool,
    windows_mode: WindowsUpdateInstallMode,
    installer_arguments: Vec<OsString>,
}

impl Default for UpdateInstallOptions {
    fn default() -> Self {
        Self {
            target_executable: None,
            retain_backup: false,
            windows_mode: WindowsUpdateInstallMode::Passive,
            installer_arguments: Vec::new(),
        }
    }
}

impl UpdateInstallOptions {
    pub fn new() -> Self {
        Self::default()
    }

    /// Override the running executable used to locate the application bundle or replacement file.
    pub fn target_executable(mut self, executable: impl Into<PathBuf>) -> Self {
        self.target_executable = Some(executable.into());
        self
    }

    /// Keep the previous bundle or executable beside the installed version for caller cleanup.
    pub fn retain_backup(mut self, retain: bool) -> Self {
        self.retain_backup = retain;
        self
    }

    pub fn windows_mode(mut self, mode: WindowsUpdateInstallMode) -> Self {
        self.windows_mode = mode;
        self
    }

    /// Append native installer arguments without invoking a shell.
    pub fn installer_arguments<I, S>(mut self, arguments: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<OsString>,
    {
        self.installer_arguments
            .extend(arguments.into_iter().map(Into::into));
        self
    }

    pub fn executable_override(&self) -> Option<&Path> {
        self.target_executable.as_deref()
    }

    pub const fn retains_backup(&self) -> bool {
        self.retain_backup
    }

    pub const fn windows_install_mode(&self) -> WindowsUpdateInstallMode {
        self.windows_mode
    }

    pub fn extra_installer_arguments(&self) -> &[OsString] {
        &self.installer_arguments
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum UpdateInstallDisposition {
    Applied,
    InstallerLaunched,
}

/// Result of applying or launching one re-verified staged update.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InstalledUpdate {
    version: String,
    disposition: UpdateInstallDisposition,
    installed_path: PathBuf,
    backup_path: Option<PathBuf>,
    installer_process_id: Option<u32>,
}

impl InstalledUpdate {
    pub fn version(&self) -> &str {
        &self.version
    }

    pub const fn disposition(&self) -> UpdateInstallDisposition {
        self.disposition
    }

    pub fn installed_path(&self) -> &Path {
        &self.installed_path
    }

    pub fn backup_path(&self) -> Option<&Path> {
        self.backup_path.as_deref()
    }

    pub const fn installer_process_id(&self) -> Option<u32> {
        self.installer_process_id
    }

    pub const fn requires_application_exit(&self) -> bool {
        true
    }

    pub const fn relaunch_recommended(&self) -> bool {
        matches!(self.disposition, UpdateInstallDisposition::Applied)
    }
}

/// Checks Tauri-compatible updater JSON and stages only successfully verified artifacts.
#[derive(Clone, Debug)]
pub struct UpdateClient {
    current_version: Version,
    current_version_text: String,
    public_key: String,
    target: String,
    maximum_download_bytes: u64,
}

impl UpdateClient {
    pub fn new(current_version: &str, public_key: impl Into<String>) -> Result<Self> {
        let version_text = current_version.trim_start_matches('v');
        let current_version = Version::parse(version_text).map_err(|error| {
            SystemIntegrationError::InvalidUpdate(Arc::from(format!(
                "invalid current semantic version: {error}"
            )))
        })?;
        let public_key = public_key.into();
        parse_public_key(&public_key)?;
        Ok(Self {
            current_version,
            current_version_text: version_text.to_owned(),
            public_key,
            target: default_update_target(),
            maximum_download_bytes: DEFAULT_MAX_UPDATE_BYTES,
        })
    }

    pub fn target(mut self, target: impl Into<String>) -> Result<Self> {
        let target = target.into();
        if target.is_empty()
            || target.len() > 256
            || target.contains('\0')
            || target.contains(['\r', '\n'])
        {
            return Err(invalid(
                "the update target must be nonempty, single-line, and at most 256 UTF-8 bytes",
            ));
        }
        self.target = target;
        Ok(self)
    }

    pub fn maximum_download_bytes(mut self, maximum: u64) -> Result<Self> {
        if maximum == 0 || maximum > DEFAULT_MAX_UPDATE_BYTES {
            return Err(invalid(
                "the update download limit must be between 1 byte and 2 GiB",
            ));
        }
        self.maximum_download_bytes = maximum;
        Ok(self)
    }

    /// Fetch bounded update metadata and return `None` when the remote version is not newer.
    pub fn check(&self, endpoint: &str) -> Result<Option<AvailableUpdate>> {
        self.check_with_cancellation(endpoint, &UpdateCancellation::new())
    }

    /// Fetch update metadata while cooperatively observing cancellation between I/O reads.
    pub fn check_with_cancellation(
        &self,
        endpoint: &str,
        cancellation: &UpdateCancellation,
    ) -> Result<Option<AvailableUpdate>> {
        cancellation.check()?;
        validate_https_url(endpoint, "update endpoint")?;
        let endpoint = expand_endpoint(endpoint, &self.target, &self.current_version_text);
        let agent = https_agent();
        let mut response = agent
            .get(&endpoint)
            .call()
            .map_err(|error| network(error.to_string()))?;
        cancellation.check()?;
        let mut bytes = Vec::with_capacity(16 * 1024);
        CancellableReader::new(
            response
                .body_mut()
                .as_reader()
                .take(MAX_UPDATE_MANIFEST_BYTES as u64 + 1),
            cancellation,
        )
        .read_to_end(&mut bytes)
        .map_err(|error| cancelled_or(cancellation, || network(error.to_string())))?;
        cancellation.check()?;
        if bytes.len() > MAX_UPDATE_MANIFEST_BYTES {
            return Err(SystemIntegrationError::InvalidUpdate(Arc::from(
                "the update manifest exceeds 1 MiB",
            )));
        }
        self.parse_manifest(&bytes)
    }

    /// Parse already-fetched metadata using the same bounded, target-aware rules as [`Self::check`].
    pub fn parse_manifest(&self, bytes: &[u8]) -> Result<Option<AvailableUpdate>> {
        if bytes.len() > MAX_UPDATE_MANIFEST_BYTES {
            return Err(SystemIntegrationError::InvalidUpdate(Arc::from(
                "the update manifest exceeds 1 MiB",
            )));
        }
        let manifest: Manifest = serde_json::from_slice(bytes).map_err(|error| {
            SystemIntegrationError::InvalidUpdate(Arc::from(format!(
                "could not parse updater JSON: {error}"
            )))
        })?;
        let version_text = manifest.version.trim_start_matches('v');
        let remote = Version::parse(version_text).map_err(|error| {
            SystemIntegrationError::InvalidUpdate(Arc::from(format!(
                "invalid remote semantic version: {error}"
            )))
        })?;
        if remote <= self.current_version {
            return Ok(None);
        }
        let platform = match (manifest.url, manifest.signature) {
            (Some(url), Some(signature)) => ManifestPlatform { url, signature },
            (None, None) => manifest
                .platforms
                .get(&self.target)
                .cloned()
                .ok_or_else(|| {
                    SystemIntegrationError::InvalidUpdate(Arc::from(format!(
                        "the update manifest has no artifact for target {}",
                        self.target
                    )))
                })?,
            _ => {
                return Err(SystemIntegrationError::InvalidUpdate(Arc::from(
                    "a dynamic update manifest requires both url and signature",
                )));
            }
        };
        validate_https_url(&platform.url, "update artifact URL")?;
        validate_signature_metadata(&platform.signature)?;
        Ok(Some(AvailableUpdate {
            version: version_text.to_owned(),
            current_version: self.current_version_text.clone(),
            target: self.target.clone(),
            url: platform.url,
            signature: platform.signature,
            notes: manifest.notes,
            published_at: manifest.pub_date,
        }))
    }

    /// Download, stream-verify, fsync, and atomically persist an update artifact.
    pub fn download_and_stage(
        &self,
        update: &AvailableUpdate,
        destination_directory: impl AsRef<Path>,
    ) -> Result<PathBuf> {
        self.download_and_stage_with_progress_and_cancellation(
            update,
            destination_directory,
            |_| {},
            &UpdateCancellation::new(),
        )
    }

    /// Download and stage while reporting bounded byte progress on the calling thread.
    pub fn download_and_stage_with_progress(
        &self,
        update: &AvailableUpdate,
        destination_directory: impl AsRef<Path>,
        on_progress: impl FnMut(UpdateProgress),
    ) -> Result<PathBuf> {
        self.download_and_stage_with_progress_and_cancellation(
            update,
            destination_directory,
            on_progress,
            &UpdateCancellation::new(),
        )
    }

    /// Download and stage with progress plus cooperative cancellation between bounded chunks.
    pub fn download_and_stage_with_progress_and_cancellation(
        &self,
        update: &AvailableUpdate,
        destination_directory: impl AsRef<Path>,
        mut on_progress: impl FnMut(UpdateProgress),
        cancellation: &UpdateCancellation,
    ) -> Result<PathBuf> {
        cancellation.check()?;
        self.validate_update(update)?;
        let public_key = parse_public_key(&self.public_key)?;
        let signature = parse_signature(&update.signature)?;
        let mut verifier = public_key.verify_stream(&signature).map_err(|error| {
            SystemIntegrationError::InvalidSignature(Arc::from(format!(
                "streaming requires a modern prehashed Minisign signature: {error}"
            )))
        })?;
        let directory = destination_directory.as_ref();
        fs::create_dir_all(directory).map_err(io_error)?;
        let mut temporary = tempfile::NamedTempFile::new_in(directory).map_err(io_error)?;
        let agent = https_agent();
        let mut response = agent
            .get(&update.url)
            .call()
            .map_err(|error| network(error.to_string()))?;
        cancellation.check()?;
        let content_length = response
            .headers()
            .get("content-length")
            .and_then(|value| value.to_str().ok())
            .and_then(|value| value.parse::<u64>().ok());
        if content_length.is_some_and(|length| length > self.maximum_download_bytes) {
            return Err(SystemIntegrationError::InvalidUpdate(Arc::from(
                "the update artifact exceeds the configured download limit",
            )));
        }
        on_progress(UpdateProgress::DownloadStarted {
            total_bytes: content_length,
        });
        let mut reader = response.body_mut().as_reader();
        let mut buffer = [0_u8; 64 * 1024];
        let mut total = 0_u64;
        loop {
            cancellation.check()?;
            let read = reader.read(&mut buffer).map_err(io_error)?;
            if read == 0 {
                break;
            }
            total = total.saturating_add(read as u64);
            if total > self.maximum_download_bytes {
                return Err(SystemIntegrationError::InvalidUpdate(Arc::from(
                    "the update artifact exceeds the configured download limit",
                )));
            }
            verifier.update(&buffer[..read]);
            temporary.write_all(&buffer[..read]).map_err(io_error)?;
            on_progress(UpdateProgress::Downloaded {
                chunk_bytes: read,
                downloaded_bytes: total,
                total_bytes: content_length,
            });
        }
        on_progress(UpdateProgress::DownloadFinished {
            downloaded_bytes: total,
        });
        on_progress(UpdateProgress::VerificationStarted);
        cancellation.check()?;
        verifier.finalize().map_err(|error| {
            SystemIntegrationError::InvalidSignature(Arc::from(error.to_string()))
        })?;
        on_progress(UpdateProgress::VerificationFinished);
        cancellation.check()?;
        temporary.as_file().sync_all().map_err(io_error)?;
        let destination = directory.join(artifact_file_name(&update.url, &update.version));
        cancellation.check()?;
        temporary
            .persist_noclobber(&destination)
            .map_err(|error| io_error(error.error))?;
        on_progress(UpdateProgress::Staged {
            path: destination.clone(),
        });
        Ok(destination)
    }

    fn validate_update(&self, update: &AvailableUpdate) -> Result<()> {
        if update.target != self.target {
            return Err(SystemIntegrationError::InvalidUpdate(Arc::from(
                "the update target does not match this client",
            )));
        }
        if update.current_version != self.current_version_text {
            return Err(SystemIntegrationError::InvalidUpdate(Arc::from(
                "the update's current version does not match this client",
            )));
        }
        let update_version =
            Version::parse(update.version.trim_start_matches('v')).map_err(|error| {
                SystemIntegrationError::InvalidUpdate(Arc::from(format!(
                    "invalid update semantic version: {error}"
                )))
            })?;
        if update_version <= self.current_version {
            return Err(SystemIntegrationError::InvalidUpdate(Arc::from(
                "the staged update must be newer than the current version",
            )));
        }
        validate_https_url(&update.url, "update artifact URL")?;
        validate_signature_metadata(&update.signature)
    }

    /// Verify an existing local artifact using the configured public key and bounded streaming.
    pub fn verify_file(&self, path: impl AsRef<Path>, signature: &str) -> Result<()> {
        self.verify_file_with_cancellation(path, signature, &UpdateCancellation::new())
    }

    pub fn verify_file_with_cancellation(
        &self,
        path: impl AsRef<Path>,
        signature: &str,
        cancellation: &UpdateCancellation,
    ) -> Result<()> {
        cancellation.check()?;
        let metadata = fs::metadata(path.as_ref()).map_err(io_error)?;
        if metadata.len() > self.maximum_download_bytes {
            return Err(SystemIntegrationError::InvalidUpdate(Arc::from(
                "the update artifact exceeds the configured download limit",
            )));
        }
        let public_key = parse_public_key(&self.public_key)?;
        let signature = parse_signature(signature)?;
        let mut verifier = public_key.verify_stream(&signature).map_err(|error| {
            SystemIntegrationError::InvalidSignature(Arc::from(format!(
                "streaming requires a modern prehashed Minisign signature: {error}"
            )))
        })?;
        let mut file = File::open(path.as_ref()).map_err(io_error)?;
        let mut buffer = [0_u8; 64 * 1024];
        loop {
            cancellation.check()?;
            let read = file.read(&mut buffer).map_err(io_error)?;
            if read == 0 {
                break;
            }
            verifier.update(&buffer[..read]);
        }
        cancellation.check()?;
        verifier
            .finalize()
            .map_err(|error| SystemIntegrationError::InvalidSignature(Arc::from(error.to_string())))
    }

    /// Re-verify and install a staged artifact using the current platform's native strategy.
    ///
    /// macOS expects a gzip-compressed tar archive containing one `.app` bundle. Linux accepts a
    /// raw executable or a gzip-compressed tar archive containing one AppImage. Windows launches
    /// an `.exe` or `.msi` installer with the configured UI policy. The caller should request an
    /// orderly application relaunch after an applied Unix update, or orderly exit after a Windows
    /// installer is launched.
    pub fn install_staged(
        &self,
        update: &AvailableUpdate,
        artifact: impl AsRef<Path>,
        options: UpdateInstallOptions,
    ) -> Result<InstalledUpdate> {
        self.install_staged_with_progress_and_cancellation(
            update,
            artifact,
            options,
            |_| {},
            &UpdateCancellation::new(),
        )
    }

    pub fn install_staged_with_progress(
        &self,
        update: &AvailableUpdate,
        artifact: impl AsRef<Path>,
        options: UpdateInstallOptions,
        on_progress: impl FnMut(UpdateProgress),
    ) -> Result<InstalledUpdate> {
        self.install_staged_with_progress_and_cancellation(
            update,
            artifact,
            options,
            on_progress,
            &UpdateCancellation::new(),
        )
    }

    /// Re-verify and install while observing cancellation before every reversible phase.
    pub fn install_staged_with_progress_and_cancellation(
        &self,
        update: &AvailableUpdate,
        artifact: impl AsRef<Path>,
        options: UpdateInstallOptions,
        mut on_progress: impl FnMut(UpdateProgress),
        cancellation: &UpdateCancellation,
    ) -> Result<InstalledUpdate> {
        cancellation.check()?;
        self.validate_update(update)?;
        validate_install_options(&options)?;
        let artifact = artifact.as_ref();
        let metadata = fs::metadata(artifact).map_err(io_error)?;
        if !metadata.is_file() || metadata.len() > self.maximum_download_bytes {
            return Err(SystemIntegrationError::InvalidUpdate(Arc::from(
                "the staged update must be a regular file within the configured download limit",
            )));
        }
        on_progress(UpdateProgress::VerificationStarted);
        self.verify_file_with_cancellation(artifact, &update.signature, cancellation)?;
        on_progress(UpdateProgress::VerificationFinished);
        cancellation.check()?;
        on_progress(UpdateProgress::InstallStarted {
            artifact: artifact.to_path_buf(),
        });
        let installation =
            install_current_application(artifact, &options, &update.version, cancellation)?;
        on_progress(UpdateProgress::InstallFinished {
            installation: installation.clone(),
        });
        Ok(installation)
    }
}

fn validate_install_options(options: &UpdateInstallOptions) -> Result<()> {
    if options.installer_arguments.len() > MAX_UPDATE_INSTALLER_ARGUMENTS {
        return Err(invalid(format!(
            "an update installer cannot receive more than {MAX_UPDATE_INSTALLER_ARGUMENTS} extra arguments"
        )));
    }
    let mut total = 0_usize;
    for argument in &options.installer_arguments {
        let bytes = argument.as_os_str().as_encoded_bytes();
        if bytes.contains(&0) {
            return Err(invalid("update installer arguments must be NUL-free"));
        }
        total = total.saturating_add(bytes.len());
        if total > MAX_UPDATE_INSTALLER_ARGUMENT_BYTES {
            return Err(invalid(format!(
                "update installer arguments cannot exceed {MAX_UPDATE_INSTALLER_ARGUMENT_BYTES} encoded bytes in aggregate"
            )));
        }
    }
    if let Some(executable) = &options.target_executable {
        validate_install_path(executable, "the update target executable")?;
    }
    Ok(())
}

fn validate_install_path(path: &Path, name: &str) -> Result<()> {
    let bytes = path.as_os_str().as_encoded_bytes();
    if !path.is_absolute() || bytes.is_empty() || bytes.contains(&0) || bytes.len() > 32 * 1024 {
        return Err(invalid(format!(
            "{name} must be absolute, NUL-free, and at most 32768 encoded bytes"
        )));
    }
    Ok(())
}

#[cfg(target_os = "macos")]
fn install_current_application(
    artifact: &Path,
    options: &UpdateInstallOptions,
    version: &str,
    cancellation: &UpdateCancellation,
) -> Result<InstalledUpdate> {
    cancellation.check()?;
    let executable = options
        .target_executable
        .clone()
        .map_or_else(std::env::current_exe, Ok)
        .map_err(io_error)?;
    let bundle = macos_bundle_for_executable(&executable)?;
    let parent = bundle
        .parent()
        .ok_or_else(|| invalid("the current macOS application bundle has no parent directory"))?;
    let extraction = tempfile::Builder::new()
        .prefix(".quickgui-update-")
        .tempdir_in(parent)
        .map_err(io_error)?;
    let replacement = extract_macos_bundle_archive(artifact, extraction.path(), cancellation)?;
    cancellation.check()?;
    let backup =
        replace_directory_atomically(&replacement, &bundle, options.retain_backup, cancellation)?;
    Ok(InstalledUpdate {
        version: version.to_owned(),
        disposition: UpdateInstallDisposition::Applied,
        installed_path: bundle,
        backup_path: backup,
        installer_process_id: None,
    })
}

#[cfg(target_os = "linux")]
fn install_current_application(
    artifact: &Path,
    options: &UpdateInstallOptions,
    version: &str,
    cancellation: &UpdateCancellation,
) -> Result<InstalledUpdate> {
    cancellation.check()?;
    let target = match &options.target_executable {
        Some(target) => target.clone(),
        None => std::env::var_os("APPIMAGE")
            .map(PathBuf::from)
            .filter(|path| path.is_absolute())
            .map_or_else(std::env::current_exe, Ok)
            .map_err(io_error)?,
    };
    validate_install_path(&target, "the Linux update target")?;
    let extraction;
    let replacement = if is_gzip_file(artifact)? {
        let parent = target
            .parent()
            .ok_or_else(|| invalid("the Linux update target has no parent directory"))?;
        extraction = tempfile::Builder::new()
            .prefix(".quickgui-update-")
            .tempdir_in(parent)
            .map_err(io_error)?;
        extract_single_app_image(artifact, extraction.path(), cancellation)?
    } else {
        artifact.to_path_buf()
    };
    cancellation.check()?;
    let backup =
        replace_file_atomically(&replacement, &target, options.retain_backup, cancellation)?;
    Ok(InstalledUpdate {
        version: version.to_owned(),
        disposition: UpdateInstallDisposition::Applied,
        installed_path: target,
        backup_path: backup,
        installer_process_id: None,
    })
}

#[cfg(target_os = "windows")]
fn install_current_application(
    artifact: &Path,
    options: &UpdateInstallOptions,
    version: &str,
    cancellation: &UpdateCancellation,
) -> Result<InstalledUpdate> {
    cancellation.check()?;
    let extension = artifact
        .extension()
        .and_then(OsStr::to_str)
        .unwrap_or_default();
    let mut command = if extension.eq_ignore_ascii_case("msi") {
        let mut command = Command::new("msiexec.exe");
        command.arg("/i").arg(artifact);
        command.args(options.windows_mode.msi_arguments());
        command
    } else if extension.eq_ignore_ascii_case("exe") {
        let mut command = Command::new(artifact);
        command.args(options.windows_mode.executable_arguments());
        command
    } else {
        return Err(SystemIntegrationError::InvalidUpdate(Arc::from(
            "Windows updates must be staged as an .exe or .msi installer",
        )));
    };
    cancellation.check()?;
    let child = command
        .args(&options.installer_arguments)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(io_error)?;
    Ok(InstalledUpdate {
        version: version.to_owned(),
        disposition: UpdateInstallDisposition::InstallerLaunched,
        installed_path: artifact.to_path_buf(),
        backup_path: None,
        installer_process_id: Some(child.id()),
    })
}

#[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
fn install_current_application(
    _artifact: &Path,
    _options: &UpdateInstallOptions,
    _version: &str,
    cancellation: &UpdateCancellation,
) -> Result<InstalledUpdate> {
    cancellation.check()?;
    Err(SystemIntegrationError::Unsupported)
}

#[cfg(target_os = "macos")]
fn macos_bundle_for_executable(executable: &Path) -> Result<PathBuf> {
    if executable
        .extension()
        .is_some_and(|extension| extension == "app")
    {
        validate_install_path(executable, "the macOS application bundle")?;
        return Ok(executable.to_path_buf());
    }
    let bundle = executable
        .ancestors()
        .find(|path| path.extension().is_some_and(|extension| extension == "app"))
        .ok_or_else(|| {
            SystemIntegrationError::InvalidUpdate(Arc::from(
                "the running executable is not inside a macOS .app bundle",
            ))
        })?;
    validate_install_path(bundle, "the macOS application bundle")?;
    Ok(bundle.to_path_buf())
}

#[cfg(target_os = "macos")]
fn extract_macos_bundle_archive(
    archive: &Path,
    directory: &Path,
    cancellation: &UpdateCancellation,
) -> Result<PathBuf> {
    extract_single_archive_entry(
        archive,
        directory,
        |root| root.extension().is_some_and(|extension| extension == "app"),
        "a single top-level .app bundle",
        cancellation,
    )
}

#[cfg(target_os = "linux")]
fn extract_single_app_image(
    archive: &Path,
    directory: &Path,
    cancellation: &UpdateCancellation,
) -> Result<PathBuf> {
    extract_single_archive_entry(
        archive,
        directory,
        |root| {
            root.extension()
                .and_then(OsStr::to_str)
                .is_some_and(|extension| extension.eq_ignore_ascii_case("appimage"))
        },
        "a single top-level AppImage",
        cancellation,
    )
}

#[cfg(any(target_os = "macos", target_os = "linux", test))]
fn extract_single_archive_entry(
    archive_path: &Path,
    directory: &Path,
    accepts_root: impl Fn(&Path) -> bool,
    expected: &str,
    cancellation: &UpdateCancellation,
) -> Result<PathBuf> {
    use std::path::Component;

    cancellation.check()?;
    let file = File::open(archive_path).map_err(io_error)?;
    let decoder = flate2::read::GzDecoder::new(CancellableReader::new(file, cancellation));
    let mut archive = tar::Archive::new(decoder);
    let mut root: Option<OsString> = None;
    let mut entries = 0_usize;
    let mut expanded = 0_u64;
    for entry in archive
        .entries()
        .map_err(|error| cancelled_or(cancellation, || io_error(error)))?
    {
        cancellation.check()?;
        let mut entry = entry.map_err(|error| cancelled_or(cancellation, || io_error(error)))?;
        entries = entries.saturating_add(1);
        if entries > MAX_UPDATE_ARCHIVE_ENTRIES {
            return Err(SystemIntegrationError::InvalidUpdate(Arc::from(
                "the update archive contains too many entries",
            )));
        }
        expanded = expanded.saturating_add(
            entry
                .header()
                .size()
                .map_err(|error| cancelled_or(cancellation, || io_error(error)))?,
        );
        if expanded > DEFAULT_MAX_EXPANDED_UPDATE_BYTES {
            return Err(SystemIntegrationError::InvalidUpdate(Arc::from(
                "the expanded update archive exceeds 8 GiB",
            )));
        }
        let path = entry
            .path()
            .map_err(|error| cancelled_or(cancellation, || io_error(error)))?;
        let mut components = path
            .components()
            .filter(|component| !matches!(component, Component::CurDir));
        let Some(Component::Normal(first)) = components.next() else {
            return Err(SystemIntegrationError::InvalidUpdate(Arc::from(
                "the update archive contains an unsafe path",
            )));
        };
        if components.any(|component| !matches!(component, Component::Normal(_))) {
            return Err(SystemIntegrationError::InvalidUpdate(Arc::from(
                "the update archive contains an unsafe path",
            )));
        }
        match &root {
            Some(root) if root != first => {
                return Err(SystemIntegrationError::InvalidUpdate(Arc::from(format!(
                    "the update archive must contain {expected}"
                ))));
            }
            None => root = Some(first.to_os_string()),
            _ => {}
        }
        if let Some(link) = entry
            .link_name()
            .map_err(|error| cancelled_or(cancellation, || io_error(error)))?
        {
            validate_archive_link(&path, &link)?;
        }
        if !entry
            .unpack_in(directory)
            .map_err(|error| cancelled_or(cancellation, || io_error(error)))?
        {
            return Err(SystemIntegrationError::InvalidUpdate(Arc::from(
                "the update archive tried to write outside its extraction directory",
            )));
        }
    }
    cancellation.check()?;
    let root = root.ok_or_else(|| {
        SystemIntegrationError::InvalidUpdate(Arc::from("the update archive is empty"))
    })?;
    let extracted = directory.join(root);
    if !accepts_root(&extracted) || !extracted.exists() {
        return Err(SystemIntegrationError::InvalidUpdate(Arc::from(format!(
            "the update archive must contain {expected}"
        ))));
    }
    Ok(extracted)
}

#[cfg(any(target_os = "macos", target_os = "linux", test))]
fn validate_archive_link(entry: &Path, link: &Path) -> Result<()> {
    use std::path::Component;

    if link.is_absolute() {
        return Err(SystemIntegrationError::InvalidUpdate(Arc::from(
            "the update archive contains an absolute link",
        )));
    }
    let combined = entry.parent().unwrap_or_else(|| Path::new("")).join(link);
    let mut depth = 0_usize;
    for component in combined.components() {
        match component {
            Component::CurDir => {}
            Component::Normal(_) => depth = depth.saturating_add(1),
            Component::ParentDir if depth > 1 => depth -= 1,
            Component::ParentDir | Component::RootDir | Component::Prefix(_) => {
                return Err(SystemIntegrationError::InvalidUpdate(Arc::from(
                    "the update archive contains a link outside its application root",
                )));
            }
        }
    }
    Ok(())
}

#[cfg(target_os = "linux")]
fn is_gzip_file(path: &Path) -> Result<bool> {
    let mut file = File::open(path).map_err(io_error)?;
    let mut magic = [0_u8; 2];
    let read = file.read(&mut magic).map_err(io_error)?;
    Ok(read == magic.len() && magic == [0x1f, 0x8b])
}

#[cfg(any(target_os = "linux", test))]
fn replace_file_atomically(
    source: &Path,
    destination: &Path,
    retain_backup: bool,
    cancellation: &UpdateCancellation,
) -> Result<Option<PathBuf>> {
    cancellation.check()?;
    validate_install_path(destination, "the update destination")?;
    let metadata = fs::metadata(destination).map_err(io_error)?;
    if !metadata.is_file() {
        return Err(SystemIntegrationError::InvalidUpdate(Arc::from(
            "the update destination is not a regular file",
        )));
    }
    let parent = destination
        .parent()
        .ok_or_else(|| invalid("the update destination has no parent directory"))?;
    let mut temporary = tempfile::NamedTempFile::new_in(parent).map_err(io_error)?;
    let input = File::open(source).map_err(io_error)?;
    std::io::copy(
        &mut CancellableReader::new(input, cancellation),
        &mut temporary,
    )
    .map_err(|error| cancelled_or(cancellation, || io_error(error)))?;
    temporary
        .as_file()
        .set_permissions(metadata.permissions())
        .map_err(io_error)?;
    temporary.as_file().sync_all().map_err(io_error)?;
    cancellation.check()?;
    let backup = vacant_backup_path(destination)?;
    fs::rename(destination, &backup).map_err(io_error)?;
    if let Err(error) = temporary.persist_noclobber(destination) {
        return Err(rollback_replacement(&backup, destination, error.error));
    }
    sync_parent_directory(parent);
    Ok(cleanup_backup(backup, false, retain_backup))
}

#[cfg(any(target_os = "macos", test))]
fn replace_directory_atomically(
    source: &Path,
    destination: &Path,
    retain_backup: bool,
    cancellation: &UpdateCancellation,
) -> Result<Option<PathBuf>> {
    cancellation.check()?;
    validate_install_path(destination, "the update destination")?;
    if !fs::metadata(destination).map_err(io_error)?.is_dir()
        || !fs::metadata(source).map_err(io_error)?.is_dir()
    {
        return Err(SystemIntegrationError::InvalidUpdate(Arc::from(
            "macOS update replacement paths must be application directories",
        )));
    }
    let parent = destination
        .parent()
        .ok_or_else(|| invalid("the update destination has no parent directory"))?;
    let backup = vacant_backup_path(destination)?;
    cancellation.check()?;
    fs::rename(destination, &backup).map_err(io_error)?;
    if let Err(error) = fs::rename(source, destination) {
        return Err(rollback_replacement(&backup, destination, error));
    }
    sync_parent_directory(parent);
    Ok(cleanup_backup(backup, true, retain_backup))
}

#[cfg(any(target_os = "macos", target_os = "linux", test))]
fn vacant_backup_path(destination: &Path) -> Result<PathBuf> {
    let parent = destination
        .parent()
        .ok_or_else(|| invalid("the update destination has no parent directory"))?;
    let name = destination
        .file_name()
        .ok_or_else(|| invalid("the update destination has no file name"))?;
    for nonce in 0..1_024_u16 {
        let mut candidate = name.to_os_string();
        candidate.push(format!(".quickgui-backup-{}-{nonce}", std::process::id()));
        let candidate = parent.join(candidate);
        if !candidate.exists() {
            return Ok(candidate);
        }
    }
    Err(SystemIntegrationError::Platform(Arc::from(
        "could not reserve an update backup path",
    )))
}

#[cfg(any(target_os = "macos", target_os = "linux", test))]
fn cleanup_backup(path: PathBuf, directory: bool, retain: bool) -> Option<PathBuf> {
    if retain {
        return Some(path);
    }
    let result = if directory {
        fs::remove_dir_all(&path)
    } else {
        fs::remove_file(&path)
    };
    result.err().map(|_| path)
}

#[cfg(any(target_os = "macos", target_os = "linux", test))]
fn rollback_replacement(
    backup: &Path,
    destination: &Path,
    replacement_error: std::io::Error,
) -> SystemIntegrationError {
    match fs::rename(backup, destination) {
        Ok(()) => {
            if let Some(parent) = destination.parent() {
                sync_parent_directory(parent);
            }
            io_error(replacement_error)
        }
        Err(rollback_error) => SystemIntegrationError::Platform(Arc::from(format!(
            "update replacement failed ({replacement_error}); restoring '{}' to '{}' also failed ({rollback_error}); the previous application may remain at the backup path",
            backup.display(),
            destination.display(),
        ))),
    }
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
fn sync_parent_directory(parent: &Path) {
    if let Ok(directory) = File::open(parent) {
        let _ = directory.sync_all();
    }
}

#[cfg(all(test, not(any(target_os = "macos", target_os = "linux"))))]
fn sync_parent_directory(_parent: &Path) {}

#[derive(Deserialize)]
struct Manifest {
    version: String,
    #[serde(default)]
    notes: Option<String>,
    #[serde(default)]
    pub_date: Option<String>,
    #[serde(default)]
    url: Option<String>,
    #[serde(default)]
    signature: Option<String>,
    #[serde(default)]
    platforms: HashMap<String, ManifestPlatform>,
}

#[derive(Clone, Deserialize)]
struct ManifestPlatform {
    url: String,
    signature: String,
}

pub fn default_update_target() -> String {
    let system = match std::env::consts::OS {
        "macos" => "darwin",
        "windows" => "windows",
        "linux" => "linux",
        other => other,
    };
    format!("{system}-{}", std::env::consts::ARCH)
}

fn expand_endpoint(endpoint: &str, target: &str, current_version: &str) -> String {
    let (system, architecture) = target.split_once('-').unwrap_or((target, target));
    endpoint
        .replace("{{target}}", system)
        .replace("{{arch}}", architecture)
        .replace("{{current_version}}", current_version)
}

fn parse_public_key(value: &str) -> Result<PublicKey> {
    let decoded = decode_wrapped_text(value);
    PublicKey::decode(&decoded)
        .or_else(|_| PublicKey::from_base64(&decoded))
        .map_err(|error| {
            SystemIntegrationError::InvalidSignature(Arc::from(format!(
                "invalid Minisign public key: {error}"
            )))
        })
}

fn parse_signature(value: &str) -> Result<Signature> {
    validate_signature_metadata(value)?;
    let decoded = decode_wrapped_text(value);
    Signature::decode(&decoded).map_err(|error| {
        SystemIntegrationError::InvalidSignature(Arc::from(format!(
            "invalid Minisign signature: {error}"
        )))
    })
}

fn validate_signature_metadata(value: &str) -> Result<()> {
    if value.is_empty() || value.len() > MAX_UPDATE_SIGNATURE_BYTES || value.contains('\0') {
        return Err(SystemIntegrationError::InvalidUpdate(Arc::from(
            "the update signature must be nonempty, NUL-free, and at most 64 KiB",
        )));
    }
    Ok(())
}

fn decode_wrapped_text(value: &str) -> String {
    if value.contains('\n') {
        return value.to_owned();
    }
    base64::engine::general_purpose::STANDARD
        .decode(value)
        .ok()
        .and_then(|bytes| String::from_utf8(bytes).ok())
        .filter(|decoded| decoded.contains('\n'))
        .unwrap_or_else(|| value.to_owned())
}

fn validate_https_url(value: &str, name: &str) -> Result<()> {
    if value.len() > 16 * 1024 || value.contains('\0') || !value.starts_with("https://") {
        return Err(SystemIntegrationError::InvalidUpdate(Arc::from(format!(
            "{name} must be a bounded HTTPS URL"
        ))));
    }
    Ok(())
}

fn https_agent() -> ureq::Agent {
    let config = ureq::Agent::config_builder().https_only(true).build();
    ureq::Agent::new_with_config(config)
}

fn artifact_file_name(url: &str, version: &str) -> String {
    let candidate = url
        .split('?')
        .next()
        .and_then(|value| value.rsplit('/').next())
        .filter(|value| {
            !value.is_empty()
                && value.len() <= 255
                && value
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-'))
        });
    candidate
        .map(str::to_owned)
        .unwrap_or_else(|| format!("quickgui-update-{version}.bin"))
}

fn network(error: impl Into<Arc<str>>) -> SystemIntegrationError {
    SystemIntegrationError::Network(error.into())
}

fn cancelled_or(
    cancellation: &UpdateCancellation,
    fallback: impl FnOnce() -> SystemIntegrationError,
) -> SystemIntegrationError {
    if cancellation.is_cancelled() {
        SystemIntegrationError::Cancelled
    } else {
        fallback()
    }
}

struct CancellableReader<'a, R> {
    inner: R,
    cancellation: &'a UpdateCancellation,
}

impl<'a, R> CancellableReader<'a, R> {
    fn new(inner: R, cancellation: &'a UpdateCancellation) -> Self {
        Self {
            inner,
            cancellation,
        }
    }
}

impl<R: Read> Read for CancellableReader<'_, R> {
    fn read(&mut self, buffer: &mut [u8]) -> std::io::Result<usize> {
        if self.cancellation.is_cancelled() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::Interrupted,
                "update operation cancelled",
            ));
        }
        self.inner.read(buffer)
    }
}

fn io_error(error: impl ToString) -> SystemIntegrationError {
    SystemIntegrationError::Platform(Arc::from(error.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    const KEY: &str = "RWQf6LRCGA9i53mlYecO4IzT51TGPpvWucNSCh1CBM0QTaLn73Y7GFO3";

    #[test]
    fn selects_newer_target_artifact() {
        let client = UpdateClient::new("1.0.0", KEY)
            .unwrap()
            .target("darwin-aarch64")
            .unwrap();
        let json = serde_json::json!({
            "version": "1.1.0",
            "platforms": {
                "darwin-aarch64": {
                    "url": "https://example.com/app.tar.gz",
                    "signature": "signed-metadata-is-verified-when-staged",
                }
            }
        });
        let update = client
            .parse_manifest(json.to_string().as_bytes())
            .unwrap()
            .unwrap();
        assert_eq!(update.version, "1.1.0");
    }

    #[test]
    fn rejects_missing_signature_metadata() {
        let client = UpdateClient::new("1.0.0", KEY).unwrap();
        let json = br#"{"version":"1.1.0","url":"https://example.com/update","signature":""}"#;
        assert!(client.parse_manifest(json).is_err());
    }

    #[test]
    fn ignores_same_or_older_versions() {
        let client = UpdateClient::new("1.2.0", KEY).unwrap();
        let json = br#"{"version":"1.2.0","url":"https://example.com/update","signature":"bad"}"#;
        assert!(client.parse_manifest(json).unwrap().is_none());
    }

    #[test]
    fn sanitizes_artifact_file_names() {
        assert_eq!(
            artifact_file_name("https://example.com/App.tar.gz?token=x", "1.0.0"),
            "App.tar.gz"
        );
        assert_eq!(
            artifact_file_name("https://example.com/%2e%2e", "1.0.0"),
            "quickgui-update-1.0.0.bin"
        );
    }

    #[test]
    fn rejects_staging_metadata_from_another_check() {
        let client = UpdateClient::new("1.0.0", KEY).unwrap();
        let directory = tempfile::tempdir().unwrap();
        let mut update = AvailableUpdate {
            version: "1.1.0".to_owned(),
            current_version: "0.9.0".to_owned(),
            target: client.target.clone(),
            url: "https://example.com/update".to_owned(),
            signature: "not-reached".to_owned(),
            notes: None,
            published_at: None,
        };
        assert!(
            client
                .download_and_stage(&update, directory.path())
                .is_err()
        );

        update.current_version = "1.0.0".to_owned();
        update.version = "1.0.0".to_owned();
        assert!(
            client
                .download_and_stage(&update, directory.path())
                .is_err()
        );
    }

    #[test]
    fn https_agent_rejects_downgrade_redirects() {
        assert!(https_agent().config().https_only());
    }

    #[test]
    fn cancellation_is_cloneable_and_wins_before_updater_io_or_mutation() {
        let cancellation = UpdateCancellation::new();
        let observer = cancellation.clone();
        cancellation.cancel();
        assert!(observer.is_cancelled());

        let client = UpdateClient::new("1.0.0", KEY).unwrap();
        assert_eq!(
            client.verify_file_with_cancellation(
                "/path/that/must/not/be-opened",
                "not-parsed",
                &observer,
            ),
            Err(SystemIntegrationError::Cancelled)
        );

        let extraction = tempfile::tempdir().unwrap();
        assert_eq!(
            extract_single_archive_entry(
                Path::new("/path/that/must/not/be-opened"),
                extraction.path(),
                |_| true,
                "one root",
                &observer,
            ),
            Err(SystemIntegrationError::Cancelled)
        );
        assert!(extraction.path().read_dir().unwrap().next().is_none());
    }

    #[test]
    fn install_options_bound_native_arguments_before_spawning() {
        let arguments = (0..=MAX_UPDATE_INSTALLER_ARGUMENTS).map(|_| OsString::from("argument"));
        let options = UpdateInstallOptions::new().installer_arguments(arguments);
        assert!(validate_install_options(&options).is_err());
        assert_eq!(
            WindowsUpdateInstallMode::Passive.msi_arguments(),
            ["/passive"]
        );
        assert_eq!(
            WindowsUpdateInstallMode::Quiet.executable_arguments(),
            ["/S", "/R"]
        );
    }

    #[test]
    fn archive_extraction_requires_one_confined_application_root() {
        let directory = tempfile::tempdir().unwrap();
        let archive = directory.path().join("Demo.tar.gz");
        write_test_archive(&archive, &[("Demo.app/Contents/MacOS/Demo", b"binary")]);
        let extraction = tempfile::tempdir().unwrap();
        let bundle = extract_single_archive_entry(
            &archive,
            extraction.path(),
            |root| root.extension().is_some_and(|extension| extension == "app"),
            "a single top-level .app bundle",
            &UpdateCancellation::new(),
        )
        .unwrap();
        assert_eq!(bundle.file_name().unwrap(), "Demo.app");
        assert_eq!(
            fs::read(bundle.join("Contents/MacOS/Demo")).unwrap(),
            b"binary"
        );

        let invalid = directory.path().join("Multiple.tar.gz");
        write_test_archive(
            &invalid,
            &[
                ("First.app/Contents/first", b"first"),
                ("Second.app/Contents/second", b"second"),
            ],
        );
        assert!(
            extract_single_archive_entry(
                &invalid,
                tempfile::tempdir().unwrap().path(),
                |_| true,
                "one root",
                &UpdateCancellation::new(),
            )
            .is_err()
        );
    }

    #[test]
    fn file_replacement_is_rollback_capable_and_can_retain_its_backup() {
        let directory = tempfile::tempdir().unwrap();
        let destination = directory.path().join("application");
        let source = directory.path().join("update");
        fs::write(&destination, b"old").unwrap();
        fs::write(&source, b"new").unwrap();

        let backup =
            replace_file_atomically(&source, &destination, true, &UpdateCancellation::new())
                .unwrap()
                .unwrap();
        assert_eq!(fs::read(&destination).unwrap(), b"new");
        assert_eq!(fs::read(backup).unwrap(), b"old");
    }

    #[test]
    fn rollback_failure_preserves_the_backup_location_in_the_error() {
        let directory = tempfile::tempdir().unwrap();
        let backup = directory.path().join("missing-backup");
        let destination = directory.path().join("application");
        let error = rollback_replacement(
            &backup,
            &destination,
            std::io::Error::other("replacement failed"),
        );

        let SystemIntegrationError::Platform(message) = error else {
            panic!("rollback failure should be a platform error");
        };
        assert!(message.contains("replacement failed"));
        assert!(message.contains(backup.to_string_lossy().as_ref()));
        assert!(message.contains(destination.to_string_lossy().as_ref()));
    }

    fn write_test_archive(path: &Path, entries: &[(&str, &[u8])]) {
        let file = File::create(path).unwrap();
        let encoder = flate2::write::GzEncoder::new(file, flate2::Compression::default());
        let mut archive = tar::Builder::new(encoder);
        for (path, bytes) in entries {
            let mut header = tar::Header::new_gnu();
            header.set_mode(0o755);
            header.set_size(bytes.len() as u64);
            header.set_cksum();
            archive
                .append_data(&mut header, path, &mut &bytes[..])
                .unwrap();
        }
        archive.into_inner().unwrap().finish().unwrap();
    }
}
