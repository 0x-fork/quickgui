use std::path::PathBuf;

use objc2::rc::Retained;
use objc2_app_kit::{
    NSFilenamesPboardType, NSPasteboard, NSPasteboardNameFind, NSPasteboardType,
    NSPasteboardTypeString,
};
use objc2_foundation::{NSArray, NSData, NSString};

use crate::{
    ClipboardEntry, ClipboardError, ClipboardImage, ClipboardImageFormat, ClipboardItem,
    ClipboardString, ExternalPaths, MAX_CLIPBOARD_IMAGE_BYTES, MAX_CLIPBOARD_METADATA_BYTES,
    MAX_CLIPBOARD_PATH_BYTES, MAX_CLIPBOARD_PATHS, MAX_CLIPBOARD_TEXT_BYTES,
    MAX_CLIPBOARD_TOTAL_PATH_BYTES,
};

const TEXT_TYPE_NAME: &str = "public.utf8-plain-text";
const FILENAMES_TYPE_NAME: &str = "NSFilenamesPboardType";
const TEXT_HASH_TYPE_NAME: &str = "dev.quickgui.clipboard-text-hash";
const METADATA_TYPE_NAME: &str = "dev.quickgui.clipboard-metadata";

/// Lazily retained AppKit pasteboard and QuickGUI's metadata type identifiers.
pub(crate) struct MacPasteboard {
    inner: Retained<NSPasteboard>,
    text_hash_type: Retained<NSString>,
    metadata_type: Retained<NSString>,
}

impl MacPasteboard {
    pub(crate) fn general() -> Self {
        // SAFETY: AppKit returns the process-wide retained general pasteboard wrapper. QuickGUI
        // calls this only from its serialized application event thread.
        let inner = unsafe { NSPasteboard::generalPasteboard() };
        Self::new(inner)
    }

    pub(crate) fn find() -> Self {
        // SAFETY: `NSPasteboardNameFind` is an AppKit-owned immutable string constant.
        let inner = unsafe { NSPasteboard::pasteboardWithName(NSPasteboardNameFind) };
        Self::new(inner)
    }

    #[cfg(test)]
    fn unique() -> Self {
        // SAFETY: AppKit creates an isolated pasteboard with retained ownership.
        let inner = unsafe { NSPasteboard::pasteboardWithUniqueName() };
        Self::new(inner)
    }

    fn new(inner: Retained<NSPasteboard>) -> Self {
        Self {
            inner,
            text_hash_type: NSString::from_str(TEXT_HASH_TYPE_NAME),
            metadata_type: NSString::from_str(METADATA_TYPE_NAME),
        }
    }

    pub(crate) fn read(&self) -> Result<Option<ClipboardItem>, ClipboardError> {
        if let Some(paths) = self.read_paths()? {
            let mut entries = vec![ClipboardEntry::ExternalPaths(paths)];
            // A native file copy often carries a convenience string. The file representation is
            // still useful if that optional companion text is malformed or over the text limit.
            if let Ok(Some(text)) = self.read_string() {
                entries.push(ClipboardEntry::String(text));
            }
            return ClipboardItem::new(entries).map(Some);
        }

        if let Some(text) = self.read_string()? {
            return ClipboardItem::new([text]).map(Some);
        }

        for format in ClipboardImageFormat::ALL {
            let data_type = NSString::from_str(format.uniform_type());
            if let Some(bytes) = self.read_data(&data_type, MAX_CLIPBOARD_IMAGE_BYTES, |bytes| {
                ClipboardError::ImageTooLarge {
                    bytes,
                    maximum: MAX_CLIPBOARD_IMAGE_BYTES,
                }
            })? {
                let image = ClipboardImage::new(format, bytes)?;
                return ClipboardItem::new_image(image).map(Some);
            }
        }
        Ok(None)
    }

    fn read_paths(&self) -> Result<Option<ExternalPaths>, ClipboardError> {
        // SAFETY: AppKit defines this property list as an NSArray of NSString file paths.
        let Some(value) = self
            .inner
            .propertyListForType(unsafe { NSFilenamesPboardType })
        else {
            return Ok(None);
        };
        // SAFETY: The `NSFilenamesPboardType` contract guarantees an NSArray<NSString> value.
        let filenames: Retained<NSArray<NSString>> = unsafe { Retained::cast(value) };
        if filenames.is_empty() {
            return Ok(None);
        }
        if filenames.len() > MAX_CLIPBOARD_PATHS {
            return Err(ClipboardError::TooManyPaths {
                actual: filenames.len(),
                maximum: MAX_CLIPBOARD_PATHS,
            });
        }

        let mut paths = Vec::with_capacity(filenames.len());
        let mut total = 0usize;
        for filename in filenames.iter() {
            let bytes = filename.len();
            if bytes == 0 {
                return Err(ClipboardError::InvalidPath);
            }
            if bytes > MAX_CLIPBOARD_PATH_BYTES {
                return Err(ClipboardError::PathTooLong {
                    bytes,
                    maximum: MAX_CLIPBOARD_PATH_BYTES,
                });
            }
            total = total.saturating_add(bytes);
            if total > MAX_CLIPBOARD_TOTAL_PATH_BYTES {
                return Err(ClipboardError::PathsTooLarge {
                    bytes: total,
                    maximum: MAX_CLIPBOARD_TOTAL_PATH_BYTES,
                });
            }
            paths.push(PathBuf::from(filename.to_string()));
        }
        ExternalPaths::new(paths).map(Some)
    }

    fn read_string(&self) -> Result<Option<ClipboardString>, ClipboardError> {
        // Reading NSData lets us reject an oversized native value before allocating a Rust String.
        let Some(bytes) = self.read_data(
            unsafe { NSPasteboardTypeString },
            MAX_CLIPBOARD_TEXT_BYTES,
            |bytes| ClipboardError::TextTooLarge {
                bytes,
                maximum: MAX_CLIPBOARD_TEXT_BYTES,
            },
        )?
        else {
            return Ok(None);
        };
        let text = String::from_utf8(bytes).map_err(|_| ClipboardError::InvalidText)?;
        let mut value = ClipboardString::new(text)?;

        let hash = self.read_optional_metadata_data(&self.text_hash_type, size_of::<u64>());
        let metadata =
            self.read_optional_metadata_data(&self.metadata_type, MAX_CLIPBOARD_METADATA_BYTES);
        if let (Some(hash), Some(metadata)) = (hash, metadata)
            && let Ok(hash) = <[u8; 8]>::try_from(hash.as_slice())
            && u64::from_be_bytes(hash) == value.text_hash()
            && let Ok(metadata) = String::from_utf8(metadata)
        {
            value = value.with_metadata(metadata)?;
        }
        Ok(Some(value))
    }

    fn read_optional_metadata_data(
        &self,
        data_type: &NSPasteboardType,
        maximum: usize,
    ) -> Option<Vec<u8>> {
        // Corrupt metadata never makes otherwise valid clipboard text unavailable.
        self.read_data(data_type, maximum, |_| ClipboardError::MetadataTooLarge {
            bytes: maximum.saturating_add(1),
            maximum,
        })
        .ok()
        .flatten()
    }

    fn read_data(
        &self,
        data_type: &NSPasteboardType,
        maximum: usize,
        too_large: impl FnOnce(usize) -> ClipboardError,
    ) -> Result<Option<Vec<u8>>, ClipboardError> {
        // SAFETY: The data type is an immutable NSString and the returned NSData is retained.
        let Some(data) = (unsafe { self.inner.dataForType(data_type) }) else {
            return Ok(None);
        };
        if data.len() > maximum {
            return Err(too_large(data.len()));
        }
        Ok(Some(data.bytes().to_vec()))
    }

    pub(crate) fn write(&self, item: &ClipboardItem) -> Result<(), ClipboardError> {
        if item.is_empty() {
            // SAFETY: The retained pasteboard is valid for the duration of this call.
            unsafe { self.inner.clearContents() };
            return Ok(());
        }

        let mut strings = Vec::new();
        let mut paths = Vec::new();
        let mut images = Vec::new();
        let mut seen_image_formats = [false; ClipboardImageFormat::ALL.len()];
        for entry in item.entries() {
            match entry {
                ClipboardEntry::String(value) => strings.push(value),
                ClipboardEntry::ExternalPaths(value) => paths.extend(value.paths()),
                ClipboardEntry::Image(value) => {
                    let index = image_format_index(value.format());
                    if !seen_image_formats[index] {
                        seen_image_formats[index] = true;
                        images.push(value);
                    }
                }
            }
        }

        let mut combined_text = String::new();
        for string in &strings {
            combined_text.push_str(string.text());
        }
        if strings.is_empty() && !paths.is_empty() {
            for (index, path) in paths.iter().enumerate() {
                if index != 0 {
                    combined_text.push('\n');
                }
                combined_text.push_str(path.to_str().expect("ExternalPaths validates UTF-8"));
            }
        }
        let has_text = !strings.is_empty() || !paths.is_empty();
        let metadata = match strings.as_slice() {
            [string] => string
                .metadata()
                .map(|metadata| (string.text_hash(), metadata)),
            _ => None,
        };

        let mut types = Vec::with_capacity(
            usize::from(!paths.is_empty())
                + usize::from(has_text)
                + usize::from(metadata.is_some()) * 2
                + images.len(),
        );
        if !paths.is_empty() {
            types.push(NSString::from_str(FILENAMES_TYPE_NAME));
        }
        if has_text {
            types.push(NSString::from_str(TEXT_TYPE_NAME));
        }
        if metadata.is_some() {
            types.push(self.text_hash_type.clone());
            types.push(self.metadata_type.clone());
        }
        for image in &images {
            types.push(NSString::from_str(image.format().uniform_type()));
        }

        let types = NSArray::from_vec(types);
        // `declareTypes` atomically replaces prior representations before individual bounded
        // payloads are installed. QuickGUI never registers a lazy pasteboard owner.
        unsafe { self.inner.declareTypes_owner(&types, None) };

        if !paths.is_empty() {
            let paths = paths
                .iter()
                .map(|path| {
                    NSString::from_str(path.to_str().expect("ExternalPaths validates UTF-8"))
                })
                .collect();
            let paths = NSArray::from_vec(paths);
            // SAFETY: NSArray<NSString> is a valid property list for the legacy filename type.
            let written = unsafe {
                self.inner
                    .setPropertyList_forType(&paths, NSFilenamesPboardType)
            };
            require_written(written, "external paths")?;
        }
        if has_text {
            self.write_data(
                combined_text.as_bytes(),
                unsafe { NSPasteboardTypeString },
                "text",
            )?;
        }
        if let Some((hash, metadata)) = metadata {
            self.write_data(&hash.to_be_bytes(), &self.text_hash_type, "text hash")?;
            self.write_data(metadata.as_bytes(), &self.metadata_type, "metadata")?;
        }
        for image in images {
            let data_type = NSString::from_str(image.format().uniform_type());
            self.write_data(image.bytes(), &data_type, "image")?;
        }
        Ok(())
    }

    fn write_data(
        &self,
        bytes: &[u8],
        data_type: &NSPasteboardType,
        representation: &'static str,
    ) -> Result<(), ClipboardError> {
        let data = NSData::with_bytes(bytes);
        // SAFETY: Both retained objects remain live across the synchronous AppKit call.
        let written = unsafe { self.inner.setData_forType(Some(&data), data_type) };
        require_written(written, representation)
    }
}

fn image_format_index(format: ClipboardImageFormat) -> usize {
    ClipboardImageFormat::ALL
        .iter()
        .position(|candidate| *candidate == format)
        .expect("all clipboard image formats are indexed")
}

fn require_written(written: bool, representation: &'static str) -> Result<(), ClipboardError> {
    if written {
        Ok(())
    } else {
        Err(ClipboardError::Platform(
            format!("macOS rejected the {representation} representation").into(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use objc2::rc::autoreleasepool;

    use super::*;

    #[test]
    fn unique_pasteboard_round_trips_text_metadata_and_clear() {
        autoreleasepool(|_| {
            let pasteboard = MacPasteboard::unique();
            assert_eq!(pasteboard.read().unwrap(), None);
            let item = ClipboardItem::new_string_with_metadata("hello", "selection:1").unwrap();
            pasteboard.write(&item).unwrap();
            assert_eq!(pasteboard.read().unwrap(), Some(item));
            pasteboard.write(&ClipboardItem::default()).unwrap();
            assert_eq!(pasteboard.read().unwrap(), None);
        });
    }

    #[test]
    fn unique_pasteboard_round_trips_files_with_text_fallback() {
        autoreleasepool(|_| {
            let pasteboard = MacPasteboard::unique();
            let paths = ExternalPaths::new(["/tmp/one", "/tmp/two"]).unwrap();
            let item = ClipboardItem::new_paths(paths.clone()).unwrap();
            pasteboard.write(&item).unwrap();
            let read = pasteboard.read().unwrap().unwrap();
            assert_eq!(read.entries()[0], ClipboardEntry::ExternalPaths(paths));
            assert_eq!(read.text().as_deref(), Some("/tmp/one\n/tmp/two"));
        });
    }

    #[test]
    fn unique_pasteboard_keeps_image_encoded() {
        autoreleasepool(|_| {
            let pasteboard = MacPasteboard::unique();
            let image = ClipboardImage::new(ClipboardImageFormat::Png, vec![1, 2, 3, 4]).unwrap();
            let item = ClipboardItem::new_image(image).unwrap();
            pasteboard.write(&item).unwrap();
            assert_eq!(pasteboard.read().unwrap(), Some(item));
        });
    }
}
