use std::{
    cell::Cell,
    collections::{HashMap, HashSet},
    ffi::c_void,
    ptr::{NonNull, null},
    sync::Arc,
};

use block2::RcBlock;
use objc2::{
    ClassType, DeclaredClass, declare_class, msg_send_id, mutability::MainThreadOnly, rc::Retained,
    runtime::AnyObject,
};
use objc2_app_kit::{
    NSAutoresizingMaskOptions, NSEvent, NSEventMask, NSResponder, NSView, NSWindowOrderingMode,
};
use objc2_foundation::{MainThreadMarker, NSObject, NSPoint, NSRect, NSSize};
use winit::{
    raw_window_handle::{HasWindowHandle, RawWindowHandle},
    window::Window,
};

use crate::{ElementId, native_view::NativeViewPlacement};

declare_class!(
    struct QuickGuiHostView;

    unsafe impl ClassType for QuickGuiHostView {
        #[inherits(NSResponder, NSObject)]
        type Super = NSView;
        type Mutability = MainThreadOnly;
        const NAME: &'static str = "QuickGuiHostView";
    }

    impl DeclaredClass for QuickGuiHostView {
        type Ivars = ();
    }

    unsafe impl QuickGuiHostView {
        #[method(isFlipped)]
        fn is_flipped(&self) -> bool {
            true
        }
    }
);

#[derive(Debug)]
struct OverlayIvars {
    input_active: Cell<bool>,
}

declare_class!(
    struct QuickGuiOverlayView;

    unsafe impl ClassType for QuickGuiOverlayView {
        #[inherits(NSResponder, NSObject)]
        type Super = NSView;
        type Mutability = MainThreadOnly;
        const NAME: &'static str = "QuickGuiOverlayView";
    }

    impl DeclaredClass for QuickGuiOverlayView {
        type Ivars = OverlayIvars;
    }

    unsafe impl QuickGuiOverlayView {
        #[method(isFlipped)]
        fn is_flipped(&self) -> bool {
            true
        }

        #[method(hitTest:)]
        fn hit_test(&self, _point: NSPoint) -> *const NSView {
            if !self.ivars().input_active.get() {
                return null();
            }
            unsafe {
                self.superview()
                    .map(|view| Retained::as_ptr(&view))
                    .unwrap_or(null())
            }
        }
    }
);

struct HostedView {
    clip_view: Retained<QuickGuiHostView>,
    rounded_view: Retained<QuickGuiHostView>,
    content: crate::MacNativeView,
}

/// Owns the AppKit portion of a composed QuickGUI window.
pub(crate) struct MacNativeHost {
    parent: Retained<NSView>,
    overlay: Retained<QuickGuiOverlayView>,
    event_monitor: Option<Retained<AnyObject>>,
    hosted: HashMap<ElementId, HostedView>,
    seen: HashSet<ElementId>,
}

impl MacNativeHost {
    pub fn new(window: &Arc<Window>) -> Result<Self, String> {
        let handle = window
            .window_handle()
            .map_err(|error| format!("could not access the AppKit window handle: {error}"))?;
        let RawWindowHandle::AppKit(handle) = handle.as_raw() else {
            return Err("the active window does not expose an AppKit view".to_owned());
        };
        let parent = unsafe { Retained::retain(handle.ns_view.as_ptr().cast::<NSView>()) }
            .ok_or_else(|| "the AppKit content view could not be retained".to_owned())?;
        let mtm = MainThreadMarker::new().ok_or_else(|| {
            "native views must be initialized on the AppKit main thread".to_owned()
        })?;
        let overlay_allocated = mtm.alloc().set_ivars(OverlayIvars {
            input_active: Cell::new(false),
        });
        let overlay: Retained<QuickGuiOverlayView> =
            unsafe { msg_send_id![super(overlay_allocated), initWithFrame: parent.bounds()] };
        overlay.setWantsLayer(true);
        overlay.setHidden(true);
        unsafe {
            overlay.setAutoresizingMask(
                NSAutoresizingMaskOptions::NSViewWidthSizable
                    | NSAutoresizingMaskOptions::NSViewHeightSizable,
            );
            parent.addSubview(&overlay);
        }
        let native_window = parent
            .window()
            .ok_or_else(|| "the AppKit content view is not attached to a window".to_owned())?;
        let weak_window = Arc::downgrade(window);
        let monitor_block = RcBlock::new(move |event: NonNull<NSEvent>| {
            let event_belongs_to_window = MainThreadMarker::new().is_some_and(|mtm| unsafe {
                event.as_ref().window(mtm).is_some_and(|event_window| {
                    Retained::as_ptr(&event_window) == Retained::as_ptr(&native_window)
                })
            });
            if event_belongs_to_window && let Some(window) = weak_window.upgrade() {
                // AppKit dispatches this event after the monitor returns. Queueing a redraw lets
                // the runtime observe the resulting first responder without polling while idle.
                window.request_redraw();
            }
            event.as_ptr()
        });
        let event_monitor = unsafe {
            NSEvent::addLocalMonitorForEventsMatchingMask_handler(
                NSEventMask::LeftMouseDown
                    | NSEventMask::RightMouseDown
                    | NSEventMask::OtherMouseDown,
                &monitor_block,
            )
        }
        .ok_or_else(|| "could not monitor native-view focus changes".to_owned())?;
        Ok(Self {
            parent,
            overlay,
            event_monitor: Some(event_monitor),
            hosted: HashMap::with_capacity(4),
            seen: HashSet::with_capacity(4),
        })
    }

    pub fn overlay_pointer(&self) -> NonNull<c_void> {
        NonNull::from(self.overlay.as_ref()).cast()
    }

    pub fn set_overlay_active(&self, active: bool) {
        self.overlay.ivars().input_active.set(active);
        self.overlay.setHidden(!active);
    }

    /// Returns whether AppKit, rather than QuickGUI's Winit view, owns keyboard focus.
    pub fn native_focus_active(&self) -> bool {
        if self.hosted.is_empty() {
            return false;
        }
        self.parent.window().is_some_and(|window| {
            window.firstResponder().is_some_and(|responder| {
                Retained::as_ptr(&responder).cast::<c_void>()
                    != Retained::as_ptr(&self.parent).cast::<c_void>()
            })
        })
    }

    /// Transfers AppKit keyboard focus back to the Winit view for QuickGUI controls.
    pub fn focus_framework(&self) {
        if let Some(window) = self.parent.window() {
            let _ = window.makeFirstResponder(Some(&self.parent));
        }
    }

    pub fn reconcile(&mut self, placements: &[NativeViewPlacement]) {
        self.seen.clear();
        for placement in placements {
            self.seen.insert(placement.id);
            let replace = self
                .hosted
                .get(&placement.id)
                .is_some_and(|hosted| hosted.content.pointer() != placement.view.pointer());
            if replace && let Some(previous) = self.hosted.remove(&placement.id) {
                unsafe {
                    previous.content.as_ns_view().removeFromSuperview();
                    previous.clip_view.removeFromSuperview();
                }
            }
            if !self.hosted.contains_key(&placement.id) {
                let hosted = self.create_hosted_view(&placement.view);
                self.hosted.insert(placement.id, hosted);
            }

            let hosted = &self.hosted[&placement.id];
            let clip_frame = ns_rect(placement.clip);
            let content_frame = NSRect::new(
                NSPoint::new(
                    f64::from(placement.bounds.x - placement.clip.x),
                    f64::from(placement.bounds.y - placement.clip.y),
                ),
                NSSize::new(
                    f64::from(placement.bounds.width),
                    f64::from(placement.bounds.height),
                ),
            );
            unsafe {
                hosted.clip_view.setFrame(clip_frame);
                hosted.rounded_view.setFrame(content_frame);
                hosted
                    .content
                    .as_ns_view()
                    .setFrame(NSRect::new(NSPoint::new(0.0, 0.0), content_frame.size));
                let rounded_view: &NSView = hosted.rounded_view.as_ref();
                if let Some(layer) = rounded_view.layer() {
                    layer.setCornerRadius(f64::from(placement.corner_radius));
                }
                self.parent.addSubview_positioned_relativeTo(
                    &hosted.clip_view,
                    NSWindowOrderingMode::NSWindowBelow,
                    Some(&self.overlay),
                );
            }
        }

        let stale = self
            .hosted
            .keys()
            .filter(|id| !self.seen.contains(id))
            .copied()
            .collect::<Vec<_>>();
        for id in stale {
            if let Some(hosted) = self.hosted.remove(&id) {
                unsafe {
                    hosted.content.as_ns_view().removeFromSuperview();
                    hosted.clip_view.removeFromSuperview();
                }
            }
        }
    }

    fn create_hosted_view(&self, content: &crate::MacNativeView) -> HostedView {
        let mtm = MainThreadMarker::new().expect("QuickGUI AppKit work stays on the main thread");
        let clip_allocated = mtm.alloc().set_ivars(());
        let clip_view: Retained<QuickGuiHostView> =
            unsafe { msg_send_id![super(clip_allocated), initWithFrame: NSRect::ZERO] };
        let rounded_allocated = mtm.alloc().set_ivars(());
        let rounded_view: Retained<QuickGuiHostView> =
            unsafe { msg_send_id![super(rounded_allocated), initWithFrame: NSRect::ZERO] };
        clip_view.setWantsLayer(true);
        rounded_view.setWantsLayer(true);
        unsafe {
            let clip_ns_view: &NSView = clip_view.as_ref();
            if let Some(layer) = clip_ns_view.layer() {
                layer.setMasksToBounds(true);
            }
            let rounded_ns_view: &NSView = rounded_view.as_ref();
            if let Some(layer) = rounded_ns_view.layer() {
                layer.setMasksToBounds(true);
            }
            clip_view.addSubview(&rounded_view);
            rounded_view.addSubview(content.as_ns_view());
            self.parent.addSubview_positioned_relativeTo(
                &clip_view,
                NSWindowOrderingMode::NSWindowBelow,
                Some(&self.overlay),
            );
        }
        HostedView {
            clip_view,
            rounded_view,
            content: content.clone(),
        }
    }
}

impl Drop for MacNativeHost {
    fn drop(&mut self) {
        if let Some(event_monitor) = self.event_monitor.take() {
            unsafe {
                NSEvent::removeMonitor(&event_monitor);
            }
        }
        for hosted in self.hosted.values() {
            unsafe {
                hosted.content.as_ns_view().removeFromSuperview();
                hosted.clip_view.removeFromSuperview();
            }
        }
        unsafe {
            self.overlay.removeFromSuperview();
        }
    }
}

fn ns_rect(rect: crate::Rect) -> NSRect {
    NSRect::new(
        NSPoint::new(f64::from(rect.x), f64::from(rect.y)),
        NSSize::new(f64::from(rect.width), f64::from(rect.height)),
    )
}
