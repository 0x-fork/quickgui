use std::{
    ffi::{OsStr, OsString},
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

use crate::{Result, invalid, platform};

/// Maximum arguments retained by one orderly relaunch request.
pub const MAX_RELAUNCH_ARGUMENTS: usize = 256;
/// Maximum encoded bytes retained by one relaunch argument or path.
pub const MAX_RELAUNCH_VALUE_BYTES: usize = 32 * 1024;
/// Maximum aggregate encoded argument bytes retained by one relaunch request.
pub const MAX_RELAUNCH_ARGUMENT_BYTES: usize = 256 * 1024;

/// Overrides for an orderly application relaunch.
///
/// Unspecified values preserve the current executable, arguments (excluding argument zero), and
/// working directory. Environment variables are inherited when the prepared request is spawned.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct RelaunchOptions {
    executable: Option<PathBuf>,
    arguments: Option<Vec<OsString>>,
    working_directory: Option<PathBuf>,
}

impl RelaunchOptions {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn executable(mut self, executable: impl Into<PathBuf>) -> Self {
        self.executable = Some(executable.into());
        self
    }

    pub fn arguments<I, S>(mut self, arguments: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<OsString>,
    {
        self.arguments = Some(arguments.into_iter().map(Into::into).collect());
        self
    }

    pub fn without_arguments(mut self) -> Self {
        self.arguments = Some(Vec::new());
        self
    }

    pub fn working_directory(mut self, directory: impl Into<PathBuf>) -> Self {
        self.working_directory = Some(directory.into());
        self
    }

    pub fn executable_override(&self) -> Option<&Path> {
        self.executable.as_deref()
    }

    pub fn argument_override(&self) -> Option<&[OsString]> {
        self.arguments.as_deref()
    }

    pub fn working_directory_override(&self) -> Option<&Path> {
        self.working_directory.as_deref()
    }

    /// Resolve inherited process values and validate the complete bounded request.
    pub fn prepare(self) -> Result<RelaunchRequest> {
        RelaunchRequest::prepare(self)
    }
}

/// A complete, immutable process launch retained until orderly application teardown finishes.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RelaunchRequest {
    executable: PathBuf,
    arguments: Vec<OsString>,
    working_directory: PathBuf,
}

impl RelaunchRequest {
    pub fn current() -> Result<Self> {
        RelaunchOptions::default().prepare()
    }

    pub fn prepare(options: RelaunchOptions) -> Result<Self> {
        let executable = match options.executable {
            Some(executable) => executable,
            None => std::env::current_exe().map_err(platform)?,
        };
        let arguments = options
            .arguments
            .unwrap_or_else(|| std::env::args_os().skip(1).collect());
        let working_directory = match options.working_directory {
            Some(directory) => directory,
            None => std::env::current_dir().map_err(platform)?,
        };
        validate_absolute_path(&executable, "the relaunch executable")?;
        validate_absolute_path(&working_directory, "the relaunch working directory")?;
        if arguments.len() > MAX_RELAUNCH_ARGUMENTS {
            return Err(invalid(format!(
                "a relaunch cannot retain more than {MAX_RELAUNCH_ARGUMENTS} arguments"
            )));
        }
        let mut total = 0_usize;
        for argument in &arguments {
            let encoded = encoded_len(argument.as_os_str());
            if encoded > MAX_RELAUNCH_VALUE_BYTES || contains_nul(argument.as_os_str()) {
                return Err(invalid(format!(
                    "each relaunch argument must be NUL-free and at most {MAX_RELAUNCH_VALUE_BYTES} encoded bytes"
                )));
            }
            total = total.saturating_add(encoded);
            if total > MAX_RELAUNCH_ARGUMENT_BYTES {
                return Err(invalid(format!(
                    "relaunch arguments cannot exceed {MAX_RELAUNCH_ARGUMENT_BYTES} encoded bytes in aggregate"
                )));
            }
        }
        Ok(Self {
            executable,
            arguments,
            working_directory,
        })
    }

    pub fn executable(&self) -> &Path {
        &self.executable
    }

    pub fn arguments(&self) -> &[OsString] {
        &self.arguments
    }

    pub fn working_directory(&self) -> &Path {
        &self.working_directory
    }

    /// Spawn the replacement process without invoking a shell.
    ///
    /// QuickGUI's application runtime calls this only after its child-first window teardown and
    /// process-owned service cleanup have completed. Direct callers are responsible for choosing
    /// an equivalent lifecycle boundary.
    pub fn spawn(&self) -> Result<RelaunchedProcess> {
        let child = Command::new(&self.executable)
            .args(&self.arguments)
            .current_dir(&self.working_directory)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .map_err(platform)?;
        Ok(RelaunchedProcess { id: child.id() })
    }
}

/// Identity of a successfully spawned replacement process.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct RelaunchedProcess {
    id: u32,
}

impl RelaunchedProcess {
    pub const fn id(self) -> u32 {
        self.id
    }
}

fn validate_absolute_path(path: &Path, name: &str) -> Result<()> {
    let value = path.as_os_str();
    if !path.is_absolute()
        || value.is_empty()
        || contains_nul(value)
        || encoded_len(value) > MAX_RELAUNCH_VALUE_BYTES
    {
        return Err(invalid(format!(
            "{name} must be absolute, NUL-free, and at most {MAX_RELAUNCH_VALUE_BYTES} encoded bytes"
        )));
    }
    Ok(())
}

#[cfg(unix)]
fn encoded_len(value: &OsStr) -> usize {
    use std::os::unix::ffi::OsStrExt;
    value.as_bytes().len()
}

#[cfg(windows)]
fn encoded_len(value: &OsStr) -> usize {
    use std::os::windows::ffi::OsStrExt;
    value.encode_wide().count().saturating_mul(2)
}

#[cfg(not(any(unix, windows)))]
fn encoded_len(value: &OsStr) -> usize {
    value.to_string_lossy().len()
}

#[cfg(unix)]
fn contains_nul(value: &OsStr) -> bool {
    use std::os::unix::ffi::OsStrExt;
    value.as_bytes().contains(&0)
}

#[cfg(windows)]
fn contains_nul(value: &OsStr) -> bool {
    use std::os::windows::ffi::OsStrExt;
    value.encode_wide().any(|unit| unit == 0)
}

#[cfg(not(any(unix, windows)))]
fn contains_nul(value: &OsStr) -> bool {
    value.to_string_lossy().contains('\0')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn explicit_relaunch_request_preserves_native_values() {
        let executable = std::env::current_exe().unwrap();
        let directory = std::env::current_dir().unwrap();
        let request = RelaunchOptions::new()
            .executable(&executable)
            .arguments([OsString::from("--restored"), OsString::from("value")])
            .working_directory(&directory)
            .prepare()
            .unwrap();
        assert_eq!(request.executable(), executable);
        assert_eq!(
            request.arguments(),
            [OsString::from("--restored"), OsString::from("value")]
        );
        assert_eq!(request.working_directory(), directory);
    }

    #[test]
    fn relaunch_request_rejects_relative_paths_and_unbounded_arguments() {
        assert!(
            RelaunchOptions::new()
                .executable("relative-app")
                .prepare()
                .is_err()
        );
        let arguments = (0..=MAX_RELAUNCH_ARGUMENTS).map(|_| OsString::from("argument"));
        assert!(
            RelaunchOptions::new()
                .executable(std::env::current_exe().unwrap())
                .arguments(arguments)
                .prepare()
                .is_err()
        );
    }

    #[test]
    fn current_relaunch_request_is_fully_resolved() {
        let request = RelaunchRequest::current().unwrap();
        assert!(request.executable().is_absolute());
        assert!(request.working_directory().is_absolute());
        assert!(request.arguments().len() <= MAX_RELAUNCH_ARGUMENTS);
    }
}
