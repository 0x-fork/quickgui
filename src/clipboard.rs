use std::{cell::RefCell, fmt, path::PathBuf, rc::Rc, sync::Arc};

use serde::{Serialize, de::DeserializeOwned};
use thiserror::Error;

/// Maximum entries retained by one [`ClipboardItem`].
pub const MAX_CLIPBOARD_ENTRIES: usize = 32;
/// Maximum aggregate UTF-8 text bytes retained by one clipboard item.
pub const MAX_CLIPBOARD_TEXT_BYTES: usize = 8 * 1024 * 1024;
/// Maximum aggregate metadata bytes retained by one clipboard item.
pub const MAX_CLIPBOARD_METADATA_BYTES: usize = 256 * 1024;
/// Maximum aggregate encoded image bytes retained by one clipboard item.
pub const MAX_CLIPBOARD_IMAGE_BYTES: usize = 64 * 1024 * 1024;
/// Maximum decoded RGBA bytes accepted by a non-macOS system clipboard conversion.
pub const MAX_CLIPBOARD_DECODED_IMAGE_BYTES: usize = 64 * 1024 * 1024;
/// Maximum external paths retained by one clipboard item.
pub const MAX_CLIPBOARD_PATHS: usize = 1024;
/// Maximum UTF-8 bytes retained by one external path.
pub const MAX_CLIPBOARD_PATH_BYTES: usize = 16 * 1024;
/// Maximum aggregate UTF-8 external-path bytes retained by one clipboard item.
pub const MAX_CLIPBOARD_TOTAL_PATH_BYTES: usize = 8 * 1024 * 1024;

/// A bounded clipboard construction or platform-access failure.
#[derive(Clone, Debug, Error, Eq, PartialEq)]
pub enum ClipboardError {
    #[error("the clipboard is unavailable")]
    Unavailable,
    #[error("the clipboard is already in use by this application")]
    Busy,
    #[error("clipboard item has {actual} entries; the maximum is {maximum}")]
    TooManyEntries { actual: usize, maximum: usize },
    #[error("clipboard text is {bytes} bytes; the maximum is {maximum}")]
    TextTooLarge { bytes: usize, maximum: usize },
    #[error("clipboard metadata is {bytes} bytes; the maximum is {maximum}")]
    MetadataTooLarge { bytes: usize, maximum: usize },
    #[error("encoded clipboard image data is {bytes} bytes; the maximum is {maximum}")]
    ImageTooLarge { bytes: usize, maximum: usize },
    #[error("decoded clipboard image data is {bytes} bytes; the maximum is {maximum}")]
    DecodedImageTooLarge { bytes: usize, maximum: usize },
    #[error("clipboard item has {actual} external paths; the maximum is {maximum}")]
    TooManyPaths { actual: usize, maximum: usize },
    #[error("clipboard path is {bytes} bytes; the maximum is {maximum}")]
    PathTooLong { bytes: usize, maximum: usize },
    #[error("clipboard paths use {bytes} bytes; the maximum is {maximum}")]
    PathsTooLarge { bytes: usize, maximum: usize },
    #[error("clipboard paths must be non-empty valid UTF-8")]
    InvalidPath,
    #[error("the system clipboard contains invalid UTF-8 text")]
    InvalidText,
    #[error("this clipboard image format is unsupported by the current platform backend")]
    UnsupportedImageFormat,
    #[error("clipboard metadata could not be serialized: {0}")]
    MetadataSerialization(Arc<str>),
    #[error("clipboard operation failed: {0}")]
    Platform(Arc<str>),
}

/// One supported encoded clipboard image representation.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum ClipboardImageFormat {
    Png,
    Jpeg,
    Webp,
    Gif,
    Svg,
    Bmp,
    Tiff,
    Ico,
    Pnm,
}

impl ClipboardImageFormat {
    pub const ALL: [Self; 9] = [
        Self::Png,
        Self::Jpeg,
        Self::Webp,
        Self::Gif,
        Self::Svg,
        Self::Bmp,
        Self::Tiff,
        Self::Ico,
        Self::Pnm,
    ];

    pub const fn mime_type(self) -> &'static str {
        match self {
            Self::Png => "image/png",
            Self::Jpeg => "image/jpeg",
            Self::Webp => "image/webp",
            Self::Gif => "image/gif",
            Self::Svg => "image/svg+xml",
            Self::Bmp => "image/bmp",
            Self::Tiff => "image/tiff",
            Self::Ico => "image/x-icon",
            Self::Pnm => "image/x-portable-anymap",
        }
    }

    pub const fn extension(self) -> &'static str {
        match self {
            Self::Png => "png",
            Self::Jpeg => "jpg",
            Self::Webp => "webp",
            Self::Gif => "gif",
            Self::Svg => "svg",
            Self::Bmp => "bmp",
            Self::Tiff => "tiff",
            Self::Ico => "ico",
            Self::Pnm => "pnm",
        }
    }

    pub fn from_mime_type(mime_type: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|format| format.mime_type() == mime_type)
            .or(match mime_type {
                "image/jpg" => Some(Self::Jpeg),
                "image/tif" => Some(Self::Tiff),
                "image/ico" => Some(Self::Ico),
                _ => None,
            })
    }

    #[cfg(target_os = "macos")]
    pub(crate) const fn uniform_type(self) -> &'static str {
        match self {
            Self::Png => "public.png",
            Self::Jpeg => "public.jpeg",
            Self::Webp => "org.webmproject.webp",
            Self::Gif => "com.compuserve.gif",
            Self::Svg => "public.svg-image",
            Self::Bmp => "com.microsoft.bmp",
            Self::Tiff => "public.tiff",
            Self::Ico => "com.microsoft.ico",
            Self::Pnm => "public.pbm",
        }
    }
}

/// Immutable encoded image bytes suitable for transfer without eager decoding.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct ClipboardImage {
    format: ClipboardImageFormat,
    bytes: Arc<[u8]>,
    id: u64,
}

impl ClipboardImage {
    pub fn new(
        format: ClipboardImageFormat,
        bytes: impl Into<Arc<[u8]>>,
    ) -> Result<Self, ClipboardError> {
        let bytes = bytes.into();
        if bytes.len() > MAX_CLIPBOARD_IMAGE_BYTES {
            return Err(ClipboardError::ImageTooLarge {
                bytes: bytes.len(),
                maximum: MAX_CLIPBOARD_IMAGE_BYTES,
            });
        }
        let id = stable_hash(&bytes);
        Ok(Self { format, bytes, id })
    }

    pub const fn format(&self) -> ClipboardImageFormat {
        self.format
    }

    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    pub const fn id(&self) -> u64 {
        self.id
    }
}

/// UTF-8 clipboard text with optional application metadata.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct ClipboardString {
    text: Arc<str>,
    metadata: Option<Arc<str>>,
}

impl ClipboardString {
    pub fn new(text: impl Into<Arc<str>>) -> Result<Self, ClipboardError> {
        let text = text.into();
        validate_text_len(text.len())?;
        Ok(Self {
            text,
            metadata: None,
        })
    }

    pub fn with_metadata(mut self, metadata: impl Into<Arc<str>>) -> Result<Self, ClipboardError> {
        let metadata = metadata.into();
        validate_metadata_len(metadata.len())?;
        self.metadata = Some(metadata);
        Ok(self)
    }

    pub fn with_json_metadata<T: Serialize>(self, metadata: &T) -> Result<Self, ClipboardError> {
        let metadata = serde_json::to_string(metadata)
            .map_err(|error| ClipboardError::MetadataSerialization(error.to_string().into()))?;
        self.with_metadata(metadata)
    }

    pub fn text(&self) -> &str {
        &self.text
    }

    pub fn metadata(&self) -> Option<&str> {
        self.metadata.as_deref()
    }

    pub fn metadata_json<T: DeserializeOwned>(&self) -> Result<Option<T>, serde_json::Error> {
        self.metadata
            .as_deref()
            .map(serde_json::from_str)
            .transpose()
    }

    pub fn into_text(self) -> Arc<str> {
        self.text
    }

    #[cfg(target_os = "macos")]
    pub(crate) fn text_hash(&self) -> u64 {
        stable_hash(self.text.as_bytes())
    }
}

/// A bounded immutable collection of external file-system paths.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct ExternalPaths(Arc<[PathBuf]>);

impl ExternalPaths {
    pub fn new<I, P>(paths: I) -> Result<Self, ClipboardError>
    where
        I: IntoIterator<Item = P>,
        P: Into<PathBuf>,
    {
        let mut retained = Vec::new();
        let mut total = 0usize;
        for path in paths {
            if retained.len() == MAX_CLIPBOARD_PATHS {
                return Err(ClipboardError::TooManyPaths {
                    actual: MAX_CLIPBOARD_PATHS + 1,
                    maximum: MAX_CLIPBOARD_PATHS,
                });
            }
            let path = path.into();
            let Some(value) = path.to_str().filter(|value| !value.is_empty()) else {
                return Err(ClipboardError::InvalidPath);
            };
            let bytes = value.len();
            if bytes > MAX_CLIPBOARD_PATH_BYTES {
                return Err(ClipboardError::PathTooLong {
                    bytes,
                    maximum: MAX_CLIPBOARD_PATH_BYTES,
                });
            }
            total = total
                .checked_add(bytes)
                .ok_or(ClipboardError::PathsTooLarge {
                    bytes: usize::MAX,
                    maximum: MAX_CLIPBOARD_TOTAL_PATH_BYTES,
                })?;
            if total > MAX_CLIPBOARD_TOTAL_PATH_BYTES {
                return Err(ClipboardError::PathsTooLarge {
                    bytes: total,
                    maximum: MAX_CLIPBOARD_TOTAL_PATH_BYTES,
                });
            }
            retained.push(path);
        }
        if retained.is_empty() {
            return Err(ClipboardError::InvalidPath);
        }
        Ok(Self(retained.into()))
    }

    pub fn paths(&self) -> &[PathBuf] {
        &self.0
    }
}

/// One typed representation inside a clipboard item.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum ClipboardEntry {
    String(ClipboardString),
    Image(ClipboardImage),
    ExternalPaths(ExternalPaths),
}

impl From<ClipboardString> for ClipboardEntry {
    fn from(value: ClipboardString) -> Self {
        Self::String(value)
    }
}

impl From<ClipboardImage> for ClipboardEntry {
    fn from(value: ClipboardImage) -> Self {
        Self::Image(value)
    }
}

impl From<ExternalPaths> for ClipboardEntry {
    fn from(value: ExternalPaths) -> Self {
        Self::ExternalPaths(value)
    }
}

/// A bounded immutable set of clipboard representations.
///
/// An empty item means clear the target pasteboard. Clones share all text, image, path, and entry
/// storage, so passing an item through application state does not duplicate its payload.
#[derive(Clone, Debug, Default, Eq, Hash, PartialEq)]
pub struct ClipboardItem {
    entries: Arc<[ClipboardEntry]>,
}

impl ClipboardItem {
    pub fn new<I, E>(entries: I) -> Result<Self, ClipboardError>
    where
        I: IntoIterator<Item = E>,
        E: Into<ClipboardEntry>,
    {
        let mut retained = Vec::new();
        let mut text_bytes = 0usize;
        let mut metadata_bytes = 0usize;
        let mut image_bytes = 0usize;
        let mut path_count = 0usize;
        let mut path_bytes = 0usize;

        for entry in entries {
            if retained.len() == MAX_CLIPBOARD_ENTRIES {
                return Err(ClipboardError::TooManyEntries {
                    actual: MAX_CLIPBOARD_ENTRIES + 1,
                    maximum: MAX_CLIPBOARD_ENTRIES,
                });
            }
            let entry = entry.into();
            match &entry {
                ClipboardEntry::String(value) => {
                    text_bytes = checked_total(
                        text_bytes,
                        value.text.len(),
                        MAX_CLIPBOARD_TEXT_BYTES,
                        |bytes| ClipboardError::TextTooLarge {
                            bytes,
                            maximum: MAX_CLIPBOARD_TEXT_BYTES,
                        },
                    )?;
                    if let Some(metadata) = &value.metadata {
                        metadata_bytes = checked_total(
                            metadata_bytes,
                            metadata.len(),
                            MAX_CLIPBOARD_METADATA_BYTES,
                            |bytes| ClipboardError::MetadataTooLarge {
                                bytes,
                                maximum: MAX_CLIPBOARD_METADATA_BYTES,
                            },
                        )?;
                    }
                }
                ClipboardEntry::Image(value) => {
                    image_bytes = checked_total(
                        image_bytes,
                        value.bytes.len(),
                        MAX_CLIPBOARD_IMAGE_BYTES,
                        |bytes| ClipboardError::ImageTooLarge {
                            bytes,
                            maximum: MAX_CLIPBOARD_IMAGE_BYTES,
                        },
                    )?;
                }
                ClipboardEntry::ExternalPaths(value) => {
                    path_count =
                        checked_total(path_count, value.0.len(), MAX_CLIPBOARD_PATHS, |actual| {
                            ClipboardError::TooManyPaths {
                                actual,
                                maximum: MAX_CLIPBOARD_PATHS,
                            }
                        })?;
                    let bytes = value
                        .0
                        .iter()
                        .map(|path| path.to_str().map_or(0, str::len))
                        .sum();
                    path_bytes = checked_total(
                        path_bytes,
                        bytes,
                        MAX_CLIPBOARD_TOTAL_PATH_BYTES,
                        |bytes| ClipboardError::PathsTooLarge {
                            bytes,
                            maximum: MAX_CLIPBOARD_TOTAL_PATH_BYTES,
                        },
                    )?;
                }
            }
            retained.push(entry);
        }
        Ok(Self {
            entries: retained.into(),
        })
    }

    pub fn new_string(text: impl Into<Arc<str>>) -> Result<Self, ClipboardError> {
        Self::new([ClipboardString::new(text)?])
    }

    pub fn new_string_with_metadata(
        text: impl Into<Arc<str>>,
        metadata: impl Into<Arc<str>>,
    ) -> Result<Self, ClipboardError> {
        Self::new([ClipboardString::new(text)?.with_metadata(metadata)?])
    }

    pub fn new_string_with_json_metadata<T: Serialize>(
        text: impl Into<Arc<str>>,
        metadata: &T,
    ) -> Result<Self, ClipboardError> {
        Self::new([ClipboardString::new(text)?.with_json_metadata(metadata)?])
    }

    pub fn new_image(image: ClipboardImage) -> Result<Self, ClipboardError> {
        Self::new([image])
    }

    pub fn new_paths(paths: ExternalPaths) -> Result<Self, ClipboardError> {
        Self::new([paths])
    }

    pub fn entries(&self) -> &[ClipboardEntry] {
        &self.entries
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Concatenate string entries, falling back to newline-separated external paths.
    pub fn text(&self) -> Option<String> {
        let text_capacity = self
            .entries
            .iter()
            .filter_map(|entry| match entry {
                ClipboardEntry::String(value) => Some(value.text.len()),
                ClipboardEntry::Image(_) | ClipboardEntry::ExternalPaths(_) => None,
            })
            .sum();
        let mut answer = String::with_capacity(text_capacity);
        for entry in self.entries.iter() {
            if let ClipboardEntry::String(value) = entry {
                answer.push_str(value.text());
            }
        }
        if !answer.is_empty() {
            return Some(answer);
        }

        let mut first = true;
        for entry in self.entries.iter() {
            if let ClipboardEntry::ExternalPaths(paths) = entry {
                for path in paths.paths() {
                    if !first {
                        answer.push('\n');
                    }
                    answer.push_str(path.to_str().expect("ExternalPaths validates UTF-8"));
                    first = false;
                }
            }
        }
        (!answer.is_empty()).then_some(answer)
    }

    /// Return metadata only when this item contains exactly one string representation.
    pub fn metadata(&self) -> Option<&str> {
        match self.entries.as_ref() {
            [ClipboardEntry::String(value)] => value.metadata(),
            _ => None,
        }
    }
}

fn checked_total<E>(
    current: usize,
    additional: usize,
    maximum: usize,
    error: impl FnOnce(usize) -> E,
) -> Result<usize, E> {
    let total = current.saturating_add(additional);
    if total > maximum {
        Err(error(total))
    } else {
        Ok(total)
    }
}

fn validate_text_len(bytes: usize) -> Result<(), ClipboardError> {
    if bytes > MAX_CLIPBOARD_TEXT_BYTES {
        Err(ClipboardError::TextTooLarge {
            bytes,
            maximum: MAX_CLIPBOARD_TEXT_BYTES,
        })
    } else {
        Ok(())
    }
}

fn validate_metadata_len(bytes: usize) -> Result<(), ClipboardError> {
    if bytes > MAX_CLIPBOARD_METADATA_BYTES {
        Err(ClipboardError::MetadataTooLarge {
            bytes,
            maximum: MAX_CLIPBOARD_METADATA_BYTES,
        })
    } else {
        Ok(())
    }
}

fn stable_hash(bytes: &[u8]) -> u64 {
    let mut hash = 0xcbf2_9ce4_8422_2325u64;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash
}

#[derive(Clone, Copy)]
pub(crate) enum ClipboardTarget {
    General,
    #[cfg(target_os = "macos")]
    Find,
}

#[derive(Clone)]
pub(crate) struct ClipboardService {
    backend: Rc<RefCell<ClipboardBackend>>,
}

impl fmt::Debug for ClipboardService {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("ClipboardService(..)")
    }
}

enum ClipboardBackend {
    System(SystemClipboard),
    #[cfg(any(test, feature = "test-support"))]
    Memory(MemoryClipboard),
}

impl ClipboardService {
    pub(crate) fn system() -> Self {
        Self {
            backend: Rc::new(RefCell::new(ClipboardBackend::System(
                SystemClipboard::default(),
            ))),
        }
    }

    #[cfg(any(test, feature = "test-support"))]
    pub(crate) fn memory() -> Self {
        Self {
            backend: Rc::new(RefCell::new(ClipboardBackend::Memory(
                MemoryClipboard::default(),
            ))),
        }
    }

    pub(crate) fn read(
        &self,
        target: ClipboardTarget,
    ) -> Result<Option<ClipboardItem>, ClipboardError> {
        let mut backend = self
            .backend
            .try_borrow_mut()
            .map_err(|_| ClipboardError::Busy)?;
        match &mut *backend {
            ClipboardBackend::System(system) => system.read(target),
            #[cfg(any(test, feature = "test-support"))]
            ClipboardBackend::Memory(memory) => Ok(memory.read(target)),
        }
    }

    pub(crate) fn write(
        &self,
        target: ClipboardTarget,
        item: ClipboardItem,
    ) -> Result<(), ClipboardError> {
        let mut backend = self
            .backend
            .try_borrow_mut()
            .map_err(|_| ClipboardError::Busy)?;
        match &mut *backend {
            ClipboardBackend::System(system) => system.write(target, item),
            #[cfg(any(test, feature = "test-support"))]
            ClipboardBackend::Memory(memory) => {
                memory.write(target, item);
                Ok(())
            }
        }
    }
}

#[cfg(any(test, feature = "test-support"))]
#[derive(Default)]
struct MemoryClipboard {
    general: Option<ClipboardItem>,
    #[cfg(target_os = "macos")]
    find: Option<ClipboardItem>,
}

#[cfg(any(test, feature = "test-support"))]
impl MemoryClipboard {
    fn read(&self, target: ClipboardTarget) -> Option<ClipboardItem> {
        match target {
            ClipboardTarget::General => self.general.clone(),
            #[cfg(target_os = "macos")]
            ClipboardTarget::Find => self.find.clone(),
        }
    }

    fn write(&mut self, target: ClipboardTarget, item: ClipboardItem) {
        let value = (!item.is_empty()).then_some(item);
        match target {
            ClipboardTarget::General => self.general = value,
            #[cfg(target_os = "macos")]
            ClipboardTarget::Find => self.find = value,
        }
    }
}

#[cfg(target_os = "macos")]
#[derive(Default)]
struct SystemClipboard {
    general: Option<crate::macos_clipboard::MacPasteboard>,
    find: Option<crate::macos_clipboard::MacPasteboard>,
}

#[cfg(target_os = "macos")]
impl SystemClipboard {
    fn read(&mut self, target: ClipboardTarget) -> Result<Option<ClipboardItem>, ClipboardError> {
        self.pasteboard(target).read()
    }

    fn write(
        &mut self,
        target: ClipboardTarget,
        item: ClipboardItem,
    ) -> Result<(), ClipboardError> {
        self.pasteboard(target).write(&item)
    }

    fn pasteboard(
        &mut self,
        target: ClipboardTarget,
    ) -> &mut crate::macos_clipboard::MacPasteboard {
        match target {
            ClipboardTarget::General => self
                .general
                .get_or_insert_with(crate::macos_clipboard::MacPasteboard::general),
            ClipboardTarget::Find => self
                .find
                .get_or_insert_with(crate::macos_clipboard::MacPasteboard::find),
        }
    }
}

#[cfg(not(target_os = "macos"))]
#[derive(Default)]
struct SystemClipboard {
    general: Option<arboard::Clipboard>,
}

#[cfg(not(target_os = "macos"))]
impl SystemClipboard {
    fn read(&mut self, target: ClipboardTarget) -> Result<Option<ClipboardItem>, ClipboardError> {
        match target {
            ClipboardTarget::General => self.read_general(),
        }
    }

    fn write(
        &mut self,
        target: ClipboardTarget,
        item: ClipboardItem,
    ) -> Result<(), ClipboardError> {
        match target {
            ClipboardTarget::General => self.write_general(&item),
        }
    }

    fn clipboard(&mut self) -> Result<&mut arboard::Clipboard, ClipboardError> {
        if self.general.is_none() {
            self.general = Some(arboard::Clipboard::new().map_err(map_arboard_error)?);
        }
        self.general.as_mut().ok_or(ClipboardError::Unavailable)
    }

    fn read_general(&mut self) -> Result<Option<ClipboardItem>, ClipboardError> {
        let clipboard = self.clipboard()?;
        match clipboard.get().file_list() {
            Ok(paths) if !paths.is_empty() => {
                let paths = ExternalPaths::new(paths)?;
                let mut entries = vec![ClipboardEntry::ExternalPaths(paths)];
                if let Ok(text) = clipboard.get_text()
                    && let Ok(text) = ClipboardString::new(text)
                {
                    entries.push(ClipboardEntry::String(text));
                }
                return ClipboardItem::new(entries).map(Some);
            }
            Ok(_) | Err(arboard::Error::ContentNotAvailable) => {}
            Err(error) => return Err(map_arboard_error(error)),
        }
        match clipboard.get_text() {
            Ok(text) => return ClipboardItem::new_string(text).map(Some),
            Err(arboard::Error::ContentNotAvailable) => {}
            Err(error) => return Err(map_arboard_error(error)),
        }
        match clipboard.get_image() {
            Ok(image) => encode_arboard_image(image)
                .and_then(ClipboardItem::new_image)
                .map(Some),
            Err(arboard::Error::ContentNotAvailable) => Ok(None),
            Err(error) => Err(map_arboard_error(error)),
        }
    }

    fn write_general(&mut self, item: &ClipboardItem) -> Result<(), ClipboardError> {
        let clipboard = self.clipboard()?;
        if item.is_empty() {
            return clipboard.clear().map_err(map_arboard_error);
        }
        if let Some(paths) = item.entries().iter().find_map(|entry| match entry {
            ClipboardEntry::ExternalPaths(paths) => Some(paths),
            ClipboardEntry::String(_) | ClipboardEntry::Image(_) => None,
        }) {
            return clipboard
                .set()
                .file_list(paths.paths())
                .map_err(map_arboard_error);
        }
        if let Some(image) = item.entries().iter().find_map(|entry| match entry {
            ClipboardEntry::Image(image) => Some(image),
            ClipboardEntry::String(_) | ClipboardEntry::ExternalPaths(_) => None,
        }) {
            let image = decode_arboard_image(image)?;
            return clipboard.set_image(image).map_err(map_arboard_error);
        }
        if let Some(text) = item.text() {
            return clipboard.set_text(text).map_err(map_arboard_error);
        }
        clipboard.clear().map_err(map_arboard_error)
    }
}

#[cfg(not(target_os = "macos"))]
fn map_arboard_error(error: arboard::Error) -> ClipboardError {
    match error {
        arboard::Error::ClipboardNotSupported => ClipboardError::Unavailable,
        arboard::Error::ClipboardOccupied => ClipboardError::Busy,
        other => ClipboardError::Platform(other.to_string().into()),
    }
}

#[cfg(not(target_os = "macos"))]
fn encode_arboard_image(
    image: arboard::ImageData<'static>,
) -> Result<ClipboardImage, ClipboardError> {
    use image_codecs::{ExtendedColorType, ImageEncoder, codecs::png::PngEncoder};
    use std::io::{self, Write};

    let width = u32::try_from(image.width).map_err(|_| ClipboardError::DecodedImageTooLarge {
        bytes: usize::MAX,
        maximum: MAX_CLIPBOARD_DECODED_IMAGE_BYTES,
    })?;
    let height = u32::try_from(image.height).map_err(|_| ClipboardError::DecodedImageTooLarge {
        bytes: usize::MAX,
        maximum: MAX_CLIPBOARD_DECODED_IMAGE_BYTES,
    })?;
    let expected = image
        .width
        .checked_mul(image.height)
        .and_then(|pixels| pixels.checked_mul(4))
        .unwrap_or(usize::MAX);
    if expected > MAX_CLIPBOARD_DECODED_IMAGE_BYTES || image.bytes.len() != expected {
        return Err(ClipboardError::DecodedImageTooLarge {
            bytes: expected,
            maximum: MAX_CLIPBOARD_DECODED_IMAGE_BYTES,
        });
    }

    struct BoundedWriter {
        bytes: Vec<u8>,
    }
    impl Write for BoundedWriter {
        fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
            let total = self.bytes.len().saturating_add(bytes.len());
            if total > MAX_CLIPBOARD_IMAGE_BYTES {
                return Err(io::Error::other("encoded clipboard image limit exceeded"));
            }
            self.bytes.extend_from_slice(bytes);
            Ok(bytes.len())
        }

        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    let mut output = BoundedWriter { bytes: Vec::new() };
    PngEncoder::new(&mut output)
        .write_image(&image.bytes, width, height, ExtendedColorType::Rgba8)
        .map_err(|error| ClipboardError::Platform(error.to_string().into()))?;
    ClipboardImage::new(ClipboardImageFormat::Png, output.bytes)
}

#[cfg(not(target_os = "macos"))]
fn decode_arboard_image(
    image: &ClipboardImage,
) -> Result<arboard::ImageData<'static>, ClipboardError> {
    use std::borrow::Cow;

    if matches!(
        image.format(),
        ClipboardImageFormat::Svg
            | ClipboardImageFormat::Bmp
            | ClipboardImageFormat::Tiff
            | ClipboardImageFormat::Ico
            | ClipboardImageFormat::Pnm
    ) {
        return Err(ClipboardError::UnsupportedImageFormat);
    }
    let decoded = crate::Image::decode(image.bytes())
        .map_err(|error| ClipboardError::Platform(error.to_string().into()))?;
    Ok(arboard::ImageData {
        width: decoded.width() as usize,
        height: decoded.height() as usize,
        bytes: Cow::Owned(decoded.rgba().to_vec()),
    })
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use serde::{Deserialize, Serialize};

    use super::*;

    #[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
    struct Metadata {
        selections: Vec<usize>,
    }

    #[test]
    fn item_preserves_typed_entries_and_json_metadata() {
        let metadata = Metadata {
            selections: vec![1, 4],
        };
        let text = ClipboardString::new("hello")
            .unwrap()
            .with_json_metadata(&metadata)
            .unwrap();
        let paths =
            ExternalPaths::new([PathBuf::from("/tmp/one"), PathBuf::from("/tmp/two")]).unwrap();
        let item = ClipboardItem::new([ClipboardEntry::String(text), paths.into()]).unwrap();

        let ClipboardEntry::String(text) = &item.entries()[0] else {
            panic!("first entry should be text");
        };
        assert_eq!(text.metadata_json::<Metadata>().unwrap(), Some(metadata));
        assert_eq!(item.text().as_deref(), Some("hello"));
        assert_eq!(item.metadata(), None);
    }

    #[test]
    fn path_text_fallback_uses_newlines() {
        let item = ClipboardItem::new_paths(
            ExternalPaths::new([PathBuf::from("/tmp/one"), PathBuf::from("/tmp/two")]).unwrap(),
        )
        .unwrap();
        assert_eq!(item.text().as_deref(), Some("/tmp/one\n/tmp/two"));
    }

    #[test]
    fn constructors_reject_payloads_before_retaining_more_entries() {
        assert!(matches!(
            ClipboardString::new("x".repeat(MAX_CLIPBOARD_TEXT_BYTES + 1)),
            Err(ClipboardError::TextTooLarge { .. })
        ));
        let entry = ClipboardString::new("x").unwrap();
        assert!(matches!(
            ClipboardItem::new(std::iter::repeat_n(entry, MAX_CLIPBOARD_ENTRIES + 1)),
            Err(ClipboardError::TooManyEntries { .. })
        ));
    }

    #[test]
    fn memory_service_clears_and_keeps_find_separate() {
        let service = ClipboardService::memory();
        let item = ClipboardItem::new_string("general").unwrap();
        service
            .write(ClipboardTarget::General, item.clone())
            .unwrap();
        assert_eq!(service.read(ClipboardTarget::General).unwrap(), Some(item));
        service
            .write(ClipboardTarget::General, ClipboardItem::default())
            .unwrap();
        assert_eq!(service.read(ClipboardTarget::General).unwrap(), None);

        #[cfg(target_os = "macos")]
        {
            let find = ClipboardItem::new_string("find").unwrap();
            service.write(ClipboardTarget::Find, find.clone()).unwrap();
            assert_eq!(service.read(ClipboardTarget::Find).unwrap(), Some(find));
            assert_eq!(service.read(ClipboardTarget::General).unwrap(), None);
        }
    }

    #[test]
    fn image_identity_is_content_stable() {
        let first = ClipboardImage::new(ClipboardImageFormat::Png, vec![1, 2, 3]).unwrap();
        let second = ClipboardImage::new(ClipboardImageFormat::Png, vec![1, 2, 3]).unwrap();
        assert_eq!(first.id(), second.id());
        assert_eq!(
            ClipboardImageFormat::from_mime_type("image/jpg"),
            Some(ClipboardImageFormat::Jpeg)
        );
    }

    #[test]
    fn external_paths_require_utf8_and_a_value() {
        assert_eq!(
            ExternalPaths::new(Vec::<PathBuf>::new()),
            Err(ClipboardError::InvalidPath)
        );
        assert_eq!(
            ExternalPaths::new([PathBuf::new()]),
            Err(ClipboardError::InvalidPath)
        );
        assert_eq!(Path::new("/tmp/valid").to_str(), Some("/tmp/valid"));
    }
}
