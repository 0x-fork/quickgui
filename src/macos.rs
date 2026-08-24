use std::{
    cell::{Cell, RefCell},
    collections::{HashMap, HashSet},
    ffi::c_void,
    ptr::{NonNull, null, null_mut},
    sync::{Arc, Mutex},
};

use block2::RcBlock;
use objc2::{
    ClassType, DeclaredClass,
    declare::ClassBuilder,
    declare_class,
    ffi::{
        OBJC_ASSOCIATION_RETAIN_NONATOMIC, objc_getAssociatedObject, objc_setAssociatedObject,
        object_setClass,
    },
    msg_send, msg_send_id,
    mutability::{InteriorMutable, MainThreadOnly},
    rc::Retained,
    runtime::{AnyClass, AnyObject, NSObjectProtocol, Sel},
    sel,
};
use objc2_app_kit::{
    NSAccessibilityLayoutChangedNotification, NSAccessibilityPostNotification,
    NSAccessibilityUnignoredChildren, NSAutoresizingMaskOptions, NSBox, NSBoxType, NSColor,
    NSEvent, NSEventMask, NSResponder, NSTitlePosition, NSView, NSWindow, NSWindowOrderingMode,
};
use objc2_foundation::{MainThreadMarker, NSArray, NSObject, NSPoint, NSRect, NSSize};
use winit::{
    raw_window_handle::{HasWindowHandle, RawWindowHandle},
    window::Window,
};

use crate::{ElementId, native_view::NativeViewPlacement};

/// Covers an ordered-on-screen AppKit window until WGPU presents its first frame.
///
/// A native child view can otherwise participate in AppKit composition immediately while the
/// matching GPU scene is still being prepared, briefly exposing the child over an empty window.
pub(crate) struct MacFirstFrameGuard {
    parent: Retained<NSView>,
    window: Retained<NSWindow>,
    shield: Retained<NSBox>,
    revealed: bool,
}

/// Temporarily removes the Winit content view from its still-hidden window.
///
/// WGPU's Metal backend deliberately skips drawable acquisition when the hosting `NSWindow` is
/// occluded. A detached layer has no hosting window, so it can receive and retain the complete
/// first drawable without ever ordering a partial window onscreen. Drop always reattaches the
/// exact retained view to the same window and leaves the window frame untouched.
pub(crate) struct MacDetachedContent {
    window: Retained<NSWindow>,
    content: Retained<NSView>,
}

impl MacFirstFrameGuard {
    pub fn new(window: &Arc<Window>, background: crate::Color) -> Result<Self, String> {
        let handle = window
            .window_handle()
            .map_err(|error| format!("could not access the AppKit window handle: {error}"))?;
        let RawWindowHandle::AppKit(handle) = handle.as_raw() else {
            return Err("the active window does not expose an AppKit view".to_owned());
        };
        let parent = unsafe { Retained::retain(handle.ns_view.as_ptr().cast::<NSView>()) }
            .ok_or_else(|| "the AppKit content view could not be retained".to_owned())?;
        let window = parent
            .window()
            .ok_or_else(|| "the AppKit content view is not attached to a window".to_owned())?;
        let mtm = MainThreadMarker::new().ok_or_else(|| {
            "the first-frame shield must be initialized on the main thread".to_owned()
        })?;
        let shield = unsafe { NSBox::initWithFrame(mtm.alloc(), parent.bounds()) };
        let [red, green, blue, _] = background.to_srgba8();
        let color = unsafe {
            NSColor::colorWithSRGBRed_green_blue_alpha(
                f64::from(red) / 255.0,
                f64::from(green) / 255.0,
                f64::from(blue) / 255.0,
                1.0,
            )
        };
        unsafe {
            shield.setBoxType(NSBoxType::NSBoxCustom);
            shield.setBorderWidth(0.0);
            shield.setTitlePosition(NSTitlePosition::NSNoTitle);
            shield.setFillColor(&color);
            shield.setTransparent(false);
            shield.setAutoresizingMask(
                NSAutoresizingMaskOptions::NSViewWidthSizable
                    | NSAutoresizingMaskOptions::NSViewHeightSizable,
            );
            parent.addSubview(&shield);
        }
        Ok(Self {
            parent,
            window,
            shield,
            revealed: false,
        })
    }

    pub fn detach_content_for_first_present(&self) -> Result<MacDetachedContent, String> {
        let content = self
            .window
            .contentView()
            .ok_or_else(|| "the AppKit window has no content view to pre-present".to_owned())?;
        if !std::ptr::eq(content.as_ref(), self.parent.as_ref()) {
            return Err("the Winit view is no longer the AppKit window's content view".to_owned());
        }
        self.window.setContentView(None);
        if self.parent.window().is_some() {
            self.window.setContentView(Some(&content));
            return Err("AppKit did not detach the hidden Winit content view".to_owned());
        }
        Ok(MacDetachedContent {
            window: self.window.clone(),
            content,
        })
    }

    /// Keep the shield above native children mounted while preparing the same frame.
    pub fn cover(&self) {
        unsafe {
            self.parent.addSubview_positioned_relativeTo(
                &self.shield,
                NSWindowOrderingMode::NSWindowAbove,
                None,
            );
        }
    }

    /// Reveal only after the renderer has completed a full frame for presentation.
    pub fn reveal(mut self) {
        self.restore();
    }

    fn restore(&mut self) {
        if self.revealed {
            return;
        }
        unsafe {
            self.shield.removeFromSuperview();
        }
        self.revealed = true;
    }
}

impl Drop for MacFirstFrameGuard {
    fn drop(&mut self) {
        self.restore();
    }
}

impl Drop for MacDetachedContent {
    fn drop(&mut self) {
        self.window.setContentView(Some(&self.content));
    }
}

static ACCESSIBILITY_SUBCLASSES: Mutex<Vec<(&'static AnyClass, &'static AnyClass)>> =
    Mutex::new(Vec::new());
static ACCESSIBILITY_ASSOCIATED_OBJECT_KEY: u8 = 0;

fn accessibility_associated_object_key() -> *const c_void {
    (&ACCESSIBILITY_ASSOCIATED_OBJECT_KEY as *const u8).cast()
}

struct AccessibilityIvars {
    children: RefCell<Vec<Retained<NSView>>>,
    previous_class: &'static AnyClass,
}

declare_class!(
    struct QuickGuiAccessibilityState;

    unsafe impl ClassType for QuickGuiAccessibilityState {
        type Super = NSObject;
        type Mutability = InteriorMutable;
        const NAME: &'static str = "QuickGuiAccessibilityState";
    }

    impl DeclaredClass for QuickGuiAccessibilityState {
        type Ivars = AccessibilityIvars;
    }
);

impl QuickGuiAccessibilityState {
    fn new(previous_class: &'static AnyClass) -> Retained<Self> {
        let this = Self::alloc().set_ivars(AccessibilityIvars {
            children: RefCell::new(Vec::with_capacity(4)),
            previous_class,
        });
        unsafe { msg_send_id![super(this), init] }
    }
}

fn accessibility_state(view: &NSView) -> &QuickGuiAccessibilityState {
    let state = unsafe {
        objc_getAssociatedObject(
            view as *const NSView as *const _,
            accessibility_associated_object_key(),
        )
    };
    // The subclass is installed only after its associated state and is restored before that state
    // is cleared, so every invocation of one of these methods has a live state object.
    unsafe { (state as *const QuickGuiAccessibilityState).as_ref() }
        .expect("QuickGUI accessibility subclass is missing its associated state")
}

fn view_as_object(view: Retained<NSView>) -> Retained<NSObject> {
    let responder: Retained<NSResponder> = Retained::into_super(view);
    Retained::into_super(responder)
}

unsafe extern "C" fn accessibility_children(this: &NSView, _cmd: Sel) -> *mut NSArray<NSObject> {
    let state = accessibility_state(this);
    let previous_class = state.ivars().previous_class;
    let inherited: *mut NSArray<NSObject> =
        unsafe { msg_send![super(this, previous_class), accessibilityChildren] };
    let native_children = state.ivars().children.borrow();
    let inherited_len = unsafe { inherited.as_ref() }.map_or(0, NSArray::len);
    let mut merged = Vec::with_capacity(inherited_len + native_children.len());
    if let Some(inherited) = unsafe { inherited.as_ref() } {
        for index in 0..inherited.len() {
            if let Some(child) = inherited.get_retained(index) {
                merged.push(child);
            }
        }
    }
    if !native_children.is_empty() {
        let native_array = NSArray::from_vec(
            native_children
                .iter()
                .cloned()
                .map(view_as_object)
                .collect(),
        );
        let untyped_native_array =
            unsafe { &*(native_array.as_ref() as *const NSArray<NSObject>).cast::<NSArray>() };
        let unignored = unsafe { NSAccessibilityUnignoredChildren(untyped_native_array) }
            .cast::<NSArray<NSObject>>();
        let unignored = unsafe { unignored.as_ref() };
        for index in 0..unignored.len() {
            if let Some(child) = unignored.get_retained(index) {
                merged.push(child);
            }
        }
    }
    Retained::autorelease_return(NSArray::from_vec(merged))
}

fn rect_contains(rect: NSRect, point: NSPoint) -> bool {
    point.x >= rect.origin.x
        && point.y >= rect.origin.y
        && point.x <= rect.origin.x + rect.size.width
        && point.y <= rect.origin.y + rect.size.height
}

unsafe extern "C" fn accessibility_hit_test(
    this: &NSView,
    _cmd: Sel,
    point: NSPoint,
) -> *mut NSObject {
    let state = accessibility_state(this);
    let previous_class = state.ivars().previous_class;
    let native_children = state.ivars().children.borrow();
    for child in native_children.iter().rev() {
        let frame: NSRect = unsafe { msg_send![child.as_ref(), accessibilityFrame] };
        if rect_contains(frame, point) {
            let hit: *mut NSObject =
                unsafe { msg_send![child.as_ref(), accessibilityHitTest: point] };
            if !hit.is_null() {
                return hit;
            }
            return Retained::as_ptr(child).cast::<NSObject>().cast_mut();
        }
    }
    unsafe { msg_send![super(this, previous_class), accessibilityHitTest: point] }
}

fn view_owns_field_editor(view: &NSView, responder: *const NSResponder, depth: usize) -> bool {
    const MAX_NATIVE_VIEW_DEPTH: usize = 64;
    if depth >= MAX_NATIVE_VIEW_DEPTH {
        return false;
    }
    if view.respondsToSelector(sel!(currentEditor)) {
        let editor: *mut NSResponder = unsafe { msg_send![view, currentEditor] };
        if std::ptr::eq(editor.cast_const(), responder) {
            return true;
        }
    }
    let subviews = unsafe { view.subviews() };
    (0..subviews.len()).any(|index| {
        subviews
            .get(index)
            .is_some_and(|child| view_owns_field_editor(child, responder, depth + 1))
    })
}

fn native_focus_owner<'a>(
    children: &'a [Retained<NSView>],
    responder: &NSResponder,
) -> Option<&'a NSView> {
    let responder_pointer = responder as *const NSResponder;
    if responder.is_kind_of::<NSView>() {
        let responder_view = unsafe { &*responder_pointer.cast::<NSView>() };
        for child in children {
            if std::ptr::eq(responder_view, child.as_ref())
                || unsafe { responder_view.isDescendantOf(child) }
            {
                return Some(child);
            }
        }
    }
    children
        .iter()
        .find(|child| view_owns_field_editor(child, responder_pointer, 0))
        .map(Retained::as_ref)
}

unsafe extern "C" fn accessibility_focus(this: &NSView, _cmd: Sel) -> *mut NSObject {
    let state = accessibility_state(this);
    let previous_class = state.ivars().previous_class;
    let native_children = state.ivars().children.borrow();
    if let Some(responder) = this.window().and_then(|window| window.firstResponder())
        && let Some(owner) = native_focus_owner(&native_children, &responder)
    {
        let focused: *mut NSObject = unsafe { msg_send![owner, accessibilityFocusedUIElement] };
        if !focused.is_null() {
            return focused;
        }
        return (owner as *const NSView).cast::<NSObject>().cast_mut();
    }
    unsafe { msg_send![super(this, previous_class), accessibilityFocusedUIElement] }
}

struct HybridAccessibilityHost {
    view: Retained<NSView>,
    associated: Retained<QuickGuiAccessibilityState>,
}

impl HybridAccessibilityHost {
    fn new(view: Retained<NSView>) -> Result<Self, String> {
        let existing = unsafe {
            objc_getAssociatedObject(
                Retained::as_ptr(&view) as *const _,
                accessibility_associated_object_key(),
            )
        };
        if !existing.is_null() {
            return Err("hybrid accessibility is already installed on this AppKit view".to_owned());
        }

        // Force the view-tied class reference to static. Objective-C classes live for the process,
        // while this reference is used only until this retained view is restored during teardown.
        let previous_class = unsafe { &*(view.class() as *const AnyClass) };
        let subclass = {
            let mut subclasses = ACCESSIBILITY_SUBCLASSES
                .lock()
                .map_err(|_| "hybrid accessibility class registry is poisoned".to_owned())?;
            if let Some((_, subclass)) = subclasses
                .iter()
                .find(|(candidate, _)| *candidate == previous_class)
            {
                *subclass
            } else {
                let name = format!("QuickGuiAccessibilityOf{}", previous_class.name());
                let mut builder = ClassBuilder::new(&name, previous_class).ok_or_else(|| {
                    format!("could not declare Objective-C accessibility subclass {name}")
                })?;
                unsafe {
                    builder.add_method(
                        sel!(accessibilityChildren),
                        accessibility_children as unsafe extern "C" fn(_, _) -> _,
                    );
                    builder.add_method(
                        sel!(accessibilityChildrenInNavigationOrder),
                        accessibility_children as unsafe extern "C" fn(_, _) -> _,
                    );
                    builder.add_method(
                        sel!(accessibilityFocusedUIElement),
                        accessibility_focus as unsafe extern "C" fn(_, _) -> _,
                    );
                    builder.add_method(
                        sel!(accessibilityHitTest:),
                        accessibility_hit_test as unsafe extern "C" fn(_, _, _) -> _,
                    );
                }
                let subclass = builder.register();
                subclasses.push((previous_class, subclass));
                subclass
            }
        };

        let associated = QuickGuiAccessibilityState::new(previous_class);
        unsafe {
            objc_setAssociatedObject(
                Retained::as_ptr(&view) as *mut _,
                accessibility_associated_object_key(),
                Retained::as_ptr(&associated) as *mut _,
                OBJC_ASSOCIATION_RETAIN_NONATOMIC,
            );
            // This subclass adds no ivars; all state lives in the associated object above.
            object_setClass(
                Retained::as_ptr(&view) as *mut _,
                (subclass as *const AnyClass).cast(),
            );
        }
        Ok(Self { view, associated })
    }

    fn update(&self, placements: &[NativeViewPlacement]) {
        let mut children = self.associated.ivars().children.borrow_mut();
        let changed = children.len() != placements.len()
            || children.iter().zip(placements).any(|(current, next)| {
                Retained::as_ptr(current).cast::<c_void>() != next.view.pointer().as_ptr()
            });
        if !changed {
            return;
        }
        children.clear();
        children.extend(placements.iter().map(|placement| placement.view.retained()));
        drop(children);
        unsafe {
            NSAccessibilityPostNotification(
                self.view.as_ref(),
                NSAccessibilityLayoutChangedNotification,
            );
        }
    }
}

impl Drop for HybridAccessibilityHost {
    fn drop(&mut self) {
        let previous_class = self.associated.ivars().previous_class;
        unsafe {
            // Restore AccessKit's subclass first, then release our state. AccessKit will restore
            // Winit's original class later when its adapter is dropped.
            object_setClass(
                Retained::as_ptr(&self.view) as *mut _,
                (previous_class as *const AnyClass).cast(),
            );
            objc_setAssociatedObject(
                Retained::as_ptr(&self.view) as *mut _,
                accessibility_associated_object_key(),
                null_mut(),
                OBJC_ASSOCIATION_RETAIN_NONATOMIC,
            );
        }
    }
}

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
    accessibility: Option<HybridAccessibilityHost>,
    event_monitor: Option<Retained<AnyObject>>,
    hosted: HashMap<ElementId, HostedView>,
    seen: HashSet<ElementId>,
    seen_views: HashMap<NonNull<c_void>, ElementId>,
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
        let accessibility = Some(HybridAccessibilityHost::new(parent.clone())?);
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
            accessibility,
            event_monitor: Some(event_monitor),
            hosted: HashMap::with_capacity(4),
            seen: HashSet::with_capacity(4),
            seen_views: HashMap::with_capacity(4),
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

    pub fn reconcile(&mut self, placements: &[NativeViewPlacement]) -> Result<(), String> {
        self.seen_views.clear();
        for placement in placements {
            if let Some(first_id) = self
                .seen_views
                .insert(placement.view.pointer(), placement.id)
            {
                return Err(format!(
                    "native AppKit view {:?} is mounted by both {first_id:?} and {:?}; an NSView can have only one superview",
                    placement.view.pointer(),
                    placement.id,
                ));
            }
        }

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
        if let Some(accessibility) = &self.accessibility {
            accessibility.update(placements);
        }
        Ok(())
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
        if let Some(accessibility) = &self.accessibility {
            accessibility.update(&[]);
        }
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
        drop(self.accessibility.take());
    }
}

fn ns_rect(rect: crate::Rect) -> NSRect {
    NSRect::new(
        NSPoint::new(f64::from(rect.x), f64::from(rect.y)),
        NSSize::new(f64::from(rect.width), f64::from(rect.height)),
    )
}
