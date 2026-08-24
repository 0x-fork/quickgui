use std::{ffi::c_void, fmt, ptr::NonNull};

use objc2::rc::Retained;
use objc2_app_kit::NSView;

use crate::{ElementId, Rect};

/// A retained AppKit view that can participate in QuickGUI layout.
///
/// QuickGUI owns one retain while the value exists. The underlying `NSView` stays responsible for
/// its own content, delegates, and platform-specific behavior.
#[derive(Clone)]
pub struct MacNativeView {
    view: Retained<NSView>,
}

impl MacNativeView {
    /// Retain an AppKit view for declarative embedding.
    ///
    /// Subclasses such as `WKWebView`, `AVPlayerView`, and `PDFView` coerce to `&NSView`.
    pub fn new(view: &NSView) -> Self {
        let pointer = view as *const NSView as *mut NSView;
        let view = unsafe { Retained::retain(pointer) }
            .expect("a borrowed NSView must have a non-null Objective-C identity");
        Self { view }
    }

    pub fn as_ns_view(&self) -> &NSView {
        &self.view
    }

    pub(crate) fn pointer(&self) -> NonNull<c_void> {
        NonNull::from(self.view.as_ref()).cast()
    }
}

impl fmt::Debug for MacNativeView {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_tuple("MacNativeView")
            .field(&self.pointer())
            .finish()
    }
}

#[derive(Clone, Debug)]
pub(crate) struct NativeViewPlacement {
    pub id: ElementId,
    pub view: MacNativeView,
    pub bounds: Rect,
    pub clip: Rect,
    pub corner_radius: f32,
    pub z_index: i16,
    pub source_order: usize,
}
