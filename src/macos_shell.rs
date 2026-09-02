use std::path::Path;

use objc2::{ClassType, rc::Retained, runtime::AnyObject};
use objc2_app_kit::{
    NSAboutPanelOptionApplicationIcon, NSAboutPanelOptionApplicationName,
    NSAboutPanelOptionApplicationVersion, NSAboutPanelOptionCredits, NSAboutPanelOptionKey,
    NSAboutPanelOptionVersion, NSApplication, NSDocumentController, NSImage, NSWorkspace,
};
use objc2_foundation::{MainThreadMarker, NSAttributedString, NSDictionary, NSString};

use crate::{AboutPanelOptions, FileIconSize, Image, PlatformError};

pub(crate) fn set_dock_badge(value: Option<&str>) -> Result<(), PlatformError> {
    let mtm = main_thread()?;
    let application = NSApplication::sharedApplication(mtm);
    let dock_tile = unsafe { application.dockTile() };
    let value = value.map(NSString::from_str);
    unsafe {
        dock_tile.setBadgeLabel(value.as_deref());
        dock_tile.display();
    }
    Ok(())
}

pub(crate) fn set_dock_icon(icon: Option<&Image>) -> Result<(), PlatformError> {
    let mtm = main_thread()?;
    let icon = icon.map(|icon| native_image(mtm, icon)).transpose()?;
    unsafe {
        NSApplication::sharedApplication(mtm).setApplicationIconImage(icon.as_deref());
    }
    Ok(())
}

pub(crate) fn add_recent_document(path: &Path) -> Result<(), PlatformError> {
    let mtm = main_thread()?;
    let url = crate::macos::native_file_url(path, false)
        .map_err(|error| PlatformError::Platform(error.into()))?;
    unsafe { NSDocumentController::sharedDocumentController(mtm).noteNewRecentDocumentURL(&url) };
    Ok(())
}

pub(crate) fn clear_recent_documents() -> Result<(), PlatformError> {
    let mtm = main_thread()?;
    unsafe { NSDocumentController::sharedDocumentController(mtm).clearRecentDocuments(None) };
    Ok(())
}

pub(crate) fn show_about_panel(options: &AboutPanelOptions) -> Result<(), PlatformError> {
    let mtm = main_thread()?;
    let application = NSApplication::sharedApplication(mtm);
    let mut keys: Vec<&NSAboutPanelOptionKey> = Vec::with_capacity(5);
    let mut values: Vec<Retained<AnyObject>> = Vec::with_capacity(5);

    if let Some(value) = options.application_name.as_deref() {
        keys.push(unsafe { NSAboutPanelOptionApplicationName });
        values.push(unsafe { Retained::cast(NSString::from_str(value)) });
    }
    if let Some(value) = options.application_version.as_deref() {
        keys.push(unsafe { NSAboutPanelOptionApplicationVersion });
        values.push(unsafe { Retained::cast(NSString::from_str(value)) });
    }
    if let Some(value) = options.version.as_deref() {
        keys.push(unsafe { NSAboutPanelOptionVersion });
        values.push(unsafe { Retained::cast(NSString::from_str(value)) });
    }
    let credits = [options.copyright.as_deref(), options.credits.as_deref()]
        .into_iter()
        .flatten()
        .collect::<Vec<_>>()
        .join("\n\n");
    if !credits.is_empty() {
        keys.push(unsafe { NSAboutPanelOptionCredits });
        let credits = NSAttributedString::initWithString(
            NSAttributedString::alloc(),
            &NSString::from_str(&credits),
        );
        values.push(unsafe { Retained::cast(credits) });
    }
    if let Some(icon) = options.icon.as_ref() {
        keys.push(unsafe { NSAboutPanelOptionApplicationIcon });
        values.push(unsafe { Retained::cast(native_image(mtm, icon)?) });
    }

    unsafe {
        if keys.is_empty() {
            application.orderFrontStandardAboutPanel(None);
        } else {
            let options: Retained<NSDictionary<NSAboutPanelOptionKey, AnyObject>> =
                NSDictionary::from_vec(&keys, values);
            application.orderFrontStandardAboutPanelWithOptions(&options);
        }
        application.activate();
    }
    Ok(())
}

pub(crate) fn file_icon(path: &Path, size: FileIconSize) -> Result<Image, PlatformError> {
    let _mtm = main_thread()?;
    let url = crate::macos::native_file_url(path, path.is_dir())
        .map_err(|error| PlatformError::Platform(error.into()))?;
    let path = unsafe { url.path() }.ok_or_else(|| {
        PlatformError::Platform("Foundation could not represent the file path".into())
    })?;
    let native = unsafe { NSWorkspace::sharedWorkspace().iconForFile(&path) };
    let encoded = unsafe { native.TIFFRepresentation() }.ok_or_else(|| {
        PlatformError::Platform("AppKit could not encode the native file icon".into())
    })?;
    let decoded = Image::decode(encoded.bytes())
        .map_err(|error| PlatformError::Platform(error.to_string().into()))?;
    let pixels = size.pixels();
    if decoded.width() == pixels && decoded.height() == pixels {
        return Ok(decoded);
    }
    let source = image_codecs::RgbaImage::from_raw(
        decoded.width(),
        decoded.height(),
        decoded.rgba().to_vec(),
    )
    .ok_or_else(|| PlatformError::Platform("the native file icon was malformed".into()))?;
    let resized = image_codecs::imageops::resize(
        &source,
        pixels,
        pixels,
        image_codecs::imageops::FilterType::Lanczos3,
    );
    Image::from_rgba(pixels, pixels, resized.into_raw())
        .map_err(|error| PlatformError::Platform(error.to_string().into()))
}

/// Convert a QuickGUI image into an `NSImage`, honoring its template flag and any additional
/// backing-scale representations.
pub(crate) fn native_image(
    mtm: MainThreadMarker,
    image: &Image,
) -> Result<Retained<NSImage>, PlatformError> {
    crate::macos::native_image_with_metadata(mtm, image)
}

fn main_thread() -> Result<MainThreadMarker, PlatformError> {
    MainThreadMarker::new().ok_or_else(|| {
        PlatformError::Platform("AppKit shell services require the main thread".into())
    })
}
