use std::{
    collections::HashMap,
    fs::{self, File},
    io::{Read, Write},
    path::{Path, PathBuf},
    sync::Arc,
};

use base64::Engine;
use minisign_verify::{PublicKey, Signature};
use semver::Version;
use serde::Deserialize;

use crate::{Result, SystemIntegrationError, invalid};

pub const MAX_UPDATE_MANIFEST_BYTES: usize = 1024 * 1024;
pub const MAX_UPDATE_SIGNATURE_BYTES: usize = 64 * 1024;
pub const DEFAULT_MAX_UPDATE_BYTES: u64 = 2 * 1024 * 1024 * 1024;

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
        validate_https_url(endpoint, "update endpoint")?;
        let endpoint = expand_endpoint(endpoint, &self.target, &self.current_version_text);
        let agent = https_agent();
        let mut response = agent
            .get(&endpoint)
            .call()
            .map_err(|error| network(error.to_string()))?;
        let mut bytes = Vec::with_capacity(16 * 1024);
        response
            .body_mut()
            .as_reader()
            .take(MAX_UPDATE_MANIFEST_BYTES as u64 + 1)
            .read_to_end(&mut bytes)
            .map_err(|error| network(error.to_string()))?;
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

    /// Download, stream-verify, fsync, and atomically persist an update artifact without installing it.
    ///
    /// The signature must use Minisign's modern prehashed mode, which permits bounded-memory
    /// verification. Installation remains packaging-specific: macOS must replace a signed app
    /// bundle, Windows normally launches a signed installer, and Linux packaging varies.
    pub fn download_and_stage(
        &self,
        update: &AvailableUpdate,
        destination_directory: impl AsRef<Path>,
    ) -> Result<PathBuf> {
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
        let mut reader = response.body_mut().as_reader();
        let mut buffer = [0_u8; 64 * 1024];
        let mut total = 0_u64;
        loop {
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
        }
        verifier.finalize().map_err(|error| {
            SystemIntegrationError::InvalidSignature(Arc::from(error.to_string()))
        })?;
        temporary.as_file().sync_all().map_err(io_error)?;
        let destination = directory.join(artifact_file_name(&update.url, &update.version));
        temporary
            .persist_noclobber(&destination)
            .map_err(|error| io_error(error.error))?;
        Ok(destination)
    }

    /// Verify an existing local artifact using the configured public key and bounded streaming.
    pub fn verify_file(&self, path: impl AsRef<Path>, signature: &str) -> Result<()> {
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
            let read = file.read(&mut buffer).map_err(io_error)?;
            if read == 0 {
                break;
            }
            verifier.update(&buffer[..read]);
        }
        verifier
            .finalize()
            .map_err(|error| SystemIntegrationError::InvalidSignature(Arc::from(error.to_string())))
    }
}

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
}
