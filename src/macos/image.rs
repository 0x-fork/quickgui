use objc2::runtime::AnyClass;
use objc2_foundation::NSData;

use super::*;
use crate::{Image, ImageError};

/// `NSImageSymbolScaleLarge`.
const SYMBOL_SCALE_LARGE: isize = 3;
/// `NSFontWeightRegular`.
const SYMBOL_WEIGHT_REGULAR: f64 = 0.0;

/// Rasterize a named `NSImage` or SF Symbol at a requested point size and backing scale.
///
/// The native image is asked for its TIFF representation and decoded through the same bounded
/// decoder every other QuickGUI image uses, so no AppKit object outlives this call.
pub(crate) fn system_image(name: &str, point_size: f64, scale: f64) -> Result<Image, ImageError> {
    let unavailable = || ImageError::SystemImageUnavailable {
        name: name.to_owned(),
    };
    let mtm = MainThreadMarker::new().ok_or_else(unavailable)?;
    let _ = mtm;
    let native = NSString::from_str(name);
    let pixels = (point_size * scale).round().max(1.0);

    let image = unsafe { NSImage::imageNamed(&native) }
        .or_else(|| symbol_image(&native, pixels))
        .ok_or_else(unavailable)?;
    let encoded = unsafe { image.TIFFRepresentation() }.ok_or_else(unavailable)?;
    let decoded = Image::decode(encoded.bytes())?;
    let target = pixels as u32;
    if decoded.width().max(decoded.height()) == target {
        return Ok(decoded);
    }
    // Preserve the aspect ratio while fitting the longest axis to the requested pixel size.
    let longest = decoded.width().max(decoded.height()).max(1);
    let fit =
        |value: u32| ((u64::from(value) * u64::from(target)) / u64::from(longest)).max(1) as u32;
    decoded.resize(fit(decoded.width()), fit(decoded.height()))
}

/// Resolve an SF Symbol and apply a point-size configuration so the rasterization is large enough.
fn symbol_image(name: &NSString, pixels: f64) -> Option<Retained<NSImage>> {
    let image = unsafe { NSImage::imageWithSystemSymbolName_accessibilityDescription(name, None) }?;
    let Some(class) = AnyClass::get("NSImageSymbolConfiguration") else {
        return Some(image);
    };
    let configuration: Option<Retained<AnyObject>> = unsafe {
        msg_send_id![
            class,
            configurationWithPointSize: pixels,
            weight: SYMBOL_WEIGHT_REGULAR,
            scale: SYMBOL_SCALE_LARGE,
        ]
    };
    let Some(configuration) = configuration else {
        return Some(image);
    };
    let configured: Option<Retained<NSImage>> =
        unsafe { msg_send_id![&image, imageWithSymbolConfiguration: &*configuration] };
    configured.or(Some(image))
}

/// Build an `NSImage` honoring QuickGUI's template flag and additional scale representations.
pub(crate) fn native_image_with_metadata(
    mtm: MainThreadMarker,
    image: &Image,
) -> Result<Retained<NSImage>, PlatformError> {
    let encoded = image
        .to_png()
        .map_err(|error| PlatformError::Platform(error.to_string().into()))?;
    let native = NSImage::initWithData(mtm.alloc(), &NSData::with_bytes(&encoded))
        .ok_or_else(|| PlatformError::Platform("AppKit could not decode the image".into()))?;
    for (_, representation) in image.representations() {
        let encoded = representation
            .to_png()
            .map_err(|error| PlatformError::Platform(error.to_string().into()))?;
        let Some(variant) = NSImage::initWithData(mtm.alloc(), &NSData::with_bytes(&encoded))
        else {
            continue;
        };
        unsafe {
            let reps: Option<Retained<AnyObject>> = msg_send_id![&variant, representations];
            let Some(reps) = reps else { continue };
            let count: usize = msg_send![&reps, count];
            for index in 0..count {
                let rep: Option<Retained<AnyObject>> = msg_send_id![&reps, objectAtIndex: index];
                if let Some(rep) = rep {
                    let _: () = msg_send![&native, addRepresentation: &*rep];
                }
            }
        }
    }
    if !image.representations().is_empty() {
        // The receiver is the 1x representation, so its pixel size is also its point size.
        unsafe {
            native.setSize(NSSize::new(
                f64::from(image.width()),
                f64::from(image.height()),
            ));
        }
    }
    unsafe { native.setTemplate(image.is_template()) };
    Ok(native)
}
