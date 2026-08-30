use std::{
    any::{Any, TypeId},
    cell::{Cell, RefCell},
    collections::{HashMap, HashSet},
    ffi::{CStr, CString, c_void},
    os::unix::ffi::{OsStrExt, OsStringExt},
    path::{Path, PathBuf},
    ptr::{NonNull, null, null_mut},
    rc::{Rc, Weak},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
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
    runtime::{AnyClass, AnyObject, Bool, NSObjectProtocol, ProtocolObject, Sel},
    sel,
};
use objc2_app_kit::{
    NSAccessibilityLayoutChangedNotification, NSAccessibilityPostNotification,
    NSAccessibilityUnignoredChildren, NSAlert, NSAlertFirstButtonReturn, NSAlertStyle,
    NSApplication, NSAutoresizingMaskOptions, NSBox, NSBoxType, NSColor, NSDragOperation,
    NSDraggingContext, NSDraggingFormation, NSDraggingInfo, NSDraggingItem, NSDraggingSession,
    NSDraggingSource, NSEvent, NSEventMask, NSEventType, NSFilenamesPboardType,
    NSFloatingWindowLevel, NSImage, NSModalResponse, NSModalResponseCancel, NSModalResponseOK,
    NSNormalWindowLevel, NSOpenPanel, NSPanel, NSPasteboard, NSPasteboardType,
    NSPasteboardTypeString, NSPasteboardTypeURL, NSPasteboardWriting, NSPopUpMenuWindowLevel,
    NSResponder, NSSavePanel, NSScreen, NSTitlePosition, NSView, NSViewLayerContentsRedrawPolicy,
    NSWindow, NSWindowAnimationBehavior, NSWindowButton, NSWindowCollectionBehavior,
    NSWindowOrderingMode, NSWindowStyleMask, NSWindowTabGroup, NSWindowTabbingMode, NSWorkspace,
};
use objc2_foundation::{
    MainThreadMarker, NSArray, NSCopying, NSFileManager, NSObject, NSPoint, NSRange, NSRect,
    NSSize, NSString, NSStringEncodingConversionOptions, NSURL, NSUTF8StringEncoding, NSUUID,
};
use winit::{
    event_loop::EventLoopProxy,
    platform::macos::WindowExtMacOS,
    raw_window_handle::{HasWindowHandle, RawWindowHandle},
    window::Window,
};

use crate::{
    ElementId, ExternalDragOperation, ExternalDragPayload, ExternalDragText, ExternalDragUrl,
    MAX_EXTERNAL_DRAG_TEXT_BYTES, MAX_EXTERNAL_DRAG_URL_BYTES, MAX_GRABBING_POPOVERS,
    MAX_SYSTEM_WINDOW_TABS, Point, PopoverOptions, Rect, Size, WindowHandle, WindowKind,
    WindowTabState,
    native_view::NativeViewPlacement,
    platform::{
        FileDialogFilter, MAX_PLATFORM_PATH_BYTES, MAX_SELECTED_PATHS,
        MAX_SELECTED_PATHS_TOTAL_BYTES, PathPromptOptions, PlatformDialogId, PlatformError,
        PlatformResponder, PromptButton, PromptLevel, SavePathOptions,
    },
    runtime::RuntimeEvent,
    ui_tree::ExternalDropSnapshot,
};

mod file_dialog;

pub(crate) use file_dialog::{present_native_open_panel, present_native_save_panel};

const NATIVE_DROP_TEXT: u8 = 1 << 0;
const NATIVE_DROP_URL: u8 = 1 << 1;
const NATIVE_DROP_TYPED: u8 = 1 << 2;
const QUICKGUI_TYPED_DRAG_TYPE: &str = "dev.quickgui.typed-drag-token";
const MAX_COMPOSITE_PASTEBOARD_TYPES: usize = 32;
pub(crate) const MAX_NATIVE_TYPED_DRAG_SESSIONS: usize = 256;

static NATIVE_DROP_SUBCLASSES: Mutex<Vec<(&'static AnyClass, &'static AnyClass)>> =
    Mutex::new(Vec::new());
static NATIVE_DROP_ASSOCIATED_OBJECT_KEY: u8 = 0;

fn native_drop_associated_object_key() -> *const c_void {
    (&NATIVE_DROP_ASSOCIATED_OBJECT_KEY as *const u8).cast()
}

#[derive(Clone)]
pub(crate) struct MacTypedDragPayload {
    value: Arc<dyn Any>,
    value_type: TypeId,
    source_window: WindowHandle,
    source: ElementId,
}

impl MacTypedDragPayload {
    pub(crate) fn new(
        value: Arc<dyn Any>,
        value_type: TypeId,
        source_window: WindowHandle,
        source: ElementId,
    ) -> Self {
        let actual_type = value.as_ref().type_id();
        debug_assert_eq!(actual_type, value_type);
        Self {
            value,
            value_type: actual_type,
            source_window,
            source,
        }
    }

    pub(crate) fn source_window(&self) -> WindowHandle {
        self.source_window
    }

    pub(crate) fn source(&self) -> ElementId {
        self.source
    }
}

struct MacTypedDragRegistryInner {
    payloads: HashMap<Arc<str>, MacTypedDragPayload>,
}

/// Main-thread registry that keeps arbitrary Rust drag values out of the native pasteboard.
#[derive(Clone)]
pub(crate) struct MacTypedDragRegistry {
    inner: Rc<RefCell<MacTypedDragRegistryInner>>,
    pasteboard_type: Retained<NSString>,
}

impl MacTypedDragRegistry {
    pub(crate) fn new() -> Self {
        Self {
            inner: Rc::new(RefCell::new(MacTypedDragRegistryInner {
                payloads: HashMap::with_capacity(8),
            })),
            pasteboard_type: NSString::from_str(QUICKGUI_TYPED_DRAG_TYPE),
        }
    }

    fn register(
        &self,
        payload: MacTypedDragPayload,
    ) -> Result<(Arc<str>, MacTypedDragRegistration), String> {
        let mut inner = self.inner.borrow_mut();
        if inner.payloads.len() >= MAX_NATIVE_TYPED_DRAG_SESSIONS {
            return Err(format!(
                "the application already owns {MAX_NATIVE_TYPED_DRAG_SESSIONS} native typed drag sessions"
            ));
        }
        let token = (0..8)
            .find_map(|_| {
                let token: Arc<str> = Arc::from(NSUUID::UUID().UUIDString().to_string());
                (!inner.payloads.contains_key(token.as_ref())).then_some(token)
            })
            .ok_or_else(|| "could not allocate a unique native typed drag token".to_owned())?;
        inner.payloads.insert(token.clone(), payload);
        drop(inner);
        Ok((
            token.clone(),
            MacTypedDragRegistration {
                registry: Rc::downgrade(&self.inner),
                token,
            },
        ))
    }

    fn resolve(&self, token: &str) -> Option<MacTypedDragPayload> {
        self.inner.borrow().payloads.get(token).cloned()
    }
}

struct MacTypedDragRegistration {
    registry: Weak<RefCell<MacTypedDragRegistryInner>>,
    token: Arc<str>,
}

impl Drop for MacTypedDragRegistration {
    fn drop(&mut self) {
        if let Some(registry) = self.registry.upgrade() {
            registry.borrow_mut().payloads.remove(self.token.as_ref());
        }
    }
}

/// A bounded native payload accepted by the same exact-type listener path as internal drags.
#[derive(Clone)]
pub(crate) enum MacNativeDropPayload {
    Typed(MacTypedDragPayload),
    Text(Arc<ExternalDragText>),
    Url(Arc<ExternalDragUrl>),
}

impl MacNativeDropPayload {
    pub(crate) fn value_type(&self) -> TypeId {
        match self {
            Self::Typed(value) => value.value_type,
            Self::Text(_) => TypeId::of::<ExternalDragText>(),
            Self::Url(_) => TypeId::of::<ExternalDragUrl>(),
        }
    }

    pub(crate) fn value(&self) -> &dyn Any {
        match self {
            Self::Typed(value) => value.value.as_ref(),
            Self::Text(value) => value.as_ref(),
            Self::Url(value) => value.as_ref(),
        }
    }
}

fn native_drop_payload_ref(payload: &MacNativeDropPayload) -> (TypeId, &dyn Any) {
    (payload.value_type(), payload.value())
}

#[derive(Clone)]
pub(crate) struct MacNativeDropOffer {
    payloads: Arc<[MacNativeDropPayload]>,
}

impl MacNativeDropOffer {
    fn new(payloads: Vec<MacNativeDropPayload>) -> Option<Self> {
        (!payloads.is_empty()).then(|| Self {
            payloads: Arc::from(payloads),
        })
    }

    pub(crate) fn iter(&self) -> impl ExactSizeIterator<Item = (TypeId, &dyn Any)> + Clone {
        self.payloads.iter().map(
            native_drop_payload_ref
                as for<'a> fn(&'a MacNativeDropPayload) -> (TypeId, &'a dyn Any),
        )
    }

    pub(crate) fn payload(&self, index: usize) -> Option<&MacNativeDropPayload> {
        self.payloads.get(index)
    }
}

pub(crate) enum MacNativeDropPending {
    Hover {
        offer: MacNativeDropOffer,
        point: Point,
    },
    Exit,
    Drop {
        offer: MacNativeDropOffer,
        point: Point,
    },
}

enum NativeDropMode {
    None,
    File,
    Offer(MacNativeDropOffer),
}

struct NativeDropIvars {
    previous_class: &'static AnyClass,
    content_view: Retained<NSView>,
    window: WindowHandle,
    proxy: EventLoopProxy<RuntimeEvent>,
    typed_registry: MacTypedDragRegistry,
    capabilities: Cell<u8>,
    snapshot: RefCell<ExternalDropSnapshot>,
    active: RefCell<NativeDropMode>,
    last_point: Cell<Option<Point>>,
    last_target: Cell<Option<(ElementId, usize)>>,
    pending: RefCell<Option<MacNativeDropPending>>,
    event_queued: Cell<bool>,
}

declare_class!(
    struct QuickGuiNativeDropState;

    unsafe impl ClassType for QuickGuiNativeDropState {
        type Super = NSObject;
        type Mutability = InteriorMutable;
        const NAME: &'static str = "QuickGuiNativeDropState";
    }

    impl DeclaredClass for QuickGuiNativeDropState {
        type Ivars = NativeDropIvars;
    }
);

impl QuickGuiNativeDropState {
    fn new(
        previous_class: &'static AnyClass,
        content_view: Retained<NSView>,
        window: WindowHandle,
        proxy: EventLoopProxy<RuntimeEvent>,
        typed_registry: MacTypedDragRegistry,
    ) -> Retained<Self> {
        let this = Self::alloc().set_ivars(NativeDropIvars {
            previous_class,
            content_view,
            window,
            proxy,
            typed_registry,
            capabilities: Cell::new(0),
            snapshot: RefCell::new(ExternalDropSnapshot::new()),
            active: RefCell::new(NativeDropMode::None),
            last_point: Cell::new(None),
            last_target: Cell::new(None),
            pending: RefCell::new(None),
            event_queued: Cell::new(false),
        });
        unsafe { msg_send_id![super(this), init] }
    }

    fn point(&self, sender: &ProtocolObject<dyn NSDraggingInfo>) -> Option<Point> {
        let window_point = unsafe { sender.draggingLocation() };
        let point = self
            .ivars()
            .content_view
            .convertPoint_fromView(window_point, None);
        (point.x.is_finite() && point.y.is_finite())
            .then(|| Point::new(point.x as f32, point.y as f32))
    }

    fn target(&self, offer: &MacNativeDropOffer, point: Point) -> Option<(ElementId, usize)> {
        self.ivars()
            .snapshot
            .borrow()
            .offer_target_at(point, offer.iter())
    }

    /// Queue runtime work only when the native target changes; a drop carries its exact point.
    fn hover(&self, offer: &MacNativeDropOffer, point: Point, force: bool) -> bool {
        let target = self.target(offer, point);
        let first = self.ivars().last_point.replace(Some(point)).is_none();
        let changed = self.ivars().last_target.replace(target) != target;
        if force || first || changed {
            self.queue(MacNativeDropPending::Hover {
                offer: offer.clone(),
                point,
            });
        }
        target.is_some()
    }

    fn reset_hover(&self) {
        self.ivars().last_point.set(None);
        self.ivars().last_target.set(None);
    }

    fn queue(&self, pending: MacNativeDropPending) {
        let mut slot = self.ivars().pending.borrow_mut();
        // AppKit normally concludes a successful drop without another exit callback. Preserve
        // the submitted value if a destination nevertheless emits a late exit before the event
        // loop consumes this coalesced slot.
        if !matches!(slot.as_ref(), Some(MacNativeDropPending::Drop { .. }))
            || matches!(&pending, MacNativeDropPending::Drop { .. })
        {
            *slot = Some(pending);
        }
        drop(slot);
        if self.ivars().event_queued.replace(true) {
            return;
        }
        if self
            .ivars()
            .proxy
            .send_event(RuntimeEvent::NativeDropChanged(self.ivars().window))
            .is_err()
        {
            self.ivars().event_queued.set(false);
            self.ivars().pending.borrow_mut().take();
        }
    }
}

fn native_drop_state(delegate: &AnyObject) -> &QuickGuiNativeDropState {
    let state = unsafe {
        objc_getAssociatedObject(
            delegate as *const AnyObject as *const _,
            native_drop_associated_object_key(),
        )
    };
    // The dynamic subclass is installed only after this state and is restored before it clears.
    unsafe { (state as *const QuickGuiNativeDropState).as_ref() }
        .expect("QuickGUI native-drop subclass is missing its associated state")
}

fn pasteboard_has_type(pasteboard: &NSPasteboard, expected: &NSPasteboardType) -> bool {
    unsafe { pasteboard.types() }.is_some_and(|types| {
        (0..types.len()).any(|index| types.get(index).is_some_and(|value| value == expected))
    })
}

/// Convert at most `maximum + 4` UTF-8 bytes, enough to preserve one final scalar boundary.
fn bounded_pasteboard_string(value: &NSString, maximum: usize) -> Option<(String, bool)> {
    let capacity = maximum.checked_add(4)?;
    let mut bytes = vec![0_u8; capacity];
    let mut used = 0_usize;
    let mut remaining = NSRange::default();
    let converted = unsafe {
        value.getBytes_maxLength_usedLength_encoding_options_range_remainingRange(
            bytes.as_mut_ptr().cast(),
            bytes.len(),
            &mut used,
            NSUTF8StringEncoding,
            NSStringEncodingConversionOptions(0),
            NSRange::new(0, value.length()),
            &mut remaining,
        )
    };
    if !converted && used == 0 && value.length() != 0 {
        return None;
    }
    if used > bytes.len() {
        return None;
    }
    bytes.truncate(used);
    let mut value = String::from_utf8(bytes).ok()?;
    let truncated = remaining.length != 0 || value.len() > maximum;
    if value.len() > maximum {
        let mut end = maximum;
        while !value.is_char_boundary(end) {
            end -= 1;
        }
        value.truncate(end);
    }
    Some((value, truncated))
}

fn native_offer(
    state: &QuickGuiNativeDropState,
    pasteboard: &NSPasteboard,
) -> Option<MacNativeDropOffer> {
    let capabilities = state.ivars().capabilities.get();
    let mut payloads = Vec::with_capacity(3);
    let typed_type = state.ivars().typed_registry.pasteboard_type.as_ref();
    if capabilities & NATIVE_DROP_TYPED != 0
        && pasteboard_has_type(pasteboard, typed_type)
        && let Some(value) = unsafe { pasteboard.stringForType(typed_type) }
        && let Some((token, false)) = bounded_pasteboard_string(&value, 64)
        && let Some(payload) = state.ivars().typed_registry.resolve(&token)
    {
        payloads.push(MacNativeDropPayload::Typed(payload));
    }
    if capabilities & NATIVE_DROP_URL != 0
        && pasteboard_has_type(pasteboard, unsafe { NSPasteboardTypeURL })
        && let Some(value) = unsafe { pasteboard.stringForType(NSPasteboardTypeURL) }
        && let Some((value, false)) = bounded_pasteboard_string(&value, MAX_EXTERNAL_DRAG_URL_BYTES)
        && let Ok(value) = ExternalDragUrl::new(value)
    {
        payloads.push(MacNativeDropPayload::Url(Arc::new(value)));
    }
    if capabilities & NATIVE_DROP_TEXT != 0
        && pasteboard_has_type(pasteboard, unsafe { NSPasteboardTypeString })
        && let Some(value) = unsafe { pasteboard.stringForType(NSPasteboardTypeString) }
        && let Some((value, truncated)) =
            bounded_pasteboard_string(&value, MAX_EXTERNAL_DRAG_TEXT_BYTES)
    {
        payloads.push(MacNativeDropPayload::Text(Arc::new(
            ExternalDragText::from_bounded(value, truncated),
        )));
    }
    MacNativeDropOffer::new(payloads)
}

unsafe extern "C" fn native_drag_entered(
    this: &AnyObject,
    _cmd: Sel,
    sender: &ProtocolObject<dyn NSDraggingInfo>,
) -> NSDragOperation {
    let state = native_drop_state(this);
    state.reset_hover();
    let pasteboard = unsafe { sender.draggingPasteboard() };
    if pasteboard_has_type(&pasteboard, unsafe { NSFilenamesPboardType }) {
        *state.ivars().active.borrow_mut() = NativeDropMode::File;
        return unsafe {
            msg_send![super(this, state.ivars().previous_class), draggingEntered: sender]
        };
    }
    let Some(offer) = native_offer(state, &pasteboard) else {
        *state.ivars().active.borrow_mut() = NativeDropMode::None;
        return NSDragOperation::None;
    };
    let Some(point) = state.point(sender) else {
        *state.ivars().active.borrow_mut() = NativeDropMode::None;
        return NSDragOperation::None;
    };
    let accepted = state.hover(&offer, point, true);
    *state.ivars().active.borrow_mut() = NativeDropMode::Offer(offer);
    if accepted {
        NSDragOperation::Copy
    } else {
        NSDragOperation::None
    }
}

unsafe extern "C" fn native_drag_updated(
    this: &AnyObject,
    _cmd: Sel,
    sender: &ProtocolObject<dyn NSDraggingInfo>,
) -> NSDragOperation {
    let state = native_drop_state(this);
    let active = state.ivars().active.borrow();
    match &*active {
        NativeDropMode::File => NSDragOperation::Copy,
        NativeDropMode::Offer(offer) => {
            let Some(point) = state.point(sender) else {
                return NSDragOperation::None;
            };
            let accepted = state.hover(offer, point, false);
            if accepted {
                NSDragOperation::Copy
            } else {
                NSDragOperation::None
            }
        }
        NativeDropMode::None => NSDragOperation::None,
    }
}

unsafe extern "C" fn native_drag_exited(
    this: &AnyObject,
    _cmd: Sel,
    sender: Option<&ProtocolObject<dyn NSDraggingInfo>>,
) {
    let state = native_drop_state(this);
    let active = std::mem::replace(
        &mut *state.ivars().active.borrow_mut(),
        NativeDropMode::None,
    );
    state.reset_hover();
    match active {
        NativeDropMode::File => unsafe {
            let _: () = msg_send![
                super(this, state.ivars().previous_class),
                draggingExited: sender
            ];
        },
        NativeDropMode::Offer(_) => state.queue(MacNativeDropPending::Exit),
        NativeDropMode::None => {}
    }
}

unsafe extern "C" fn native_prepare_drag(
    this: &AnyObject,
    _cmd: Sel,
    sender: &ProtocolObject<dyn NSDraggingInfo>,
) -> Bool {
    let state = native_drop_state(this);
    match &*state.ivars().active.borrow() {
        NativeDropMode::File => unsafe {
            msg_send![
                super(this, state.ivars().previous_class),
                prepareForDragOperation: sender
            ]
        },
        NativeDropMode::Offer(offer) => Bool::new(
            state
                .point(sender)
                .is_some_and(|point| state.target(offer, point).is_some()),
        ),
        NativeDropMode::None => Bool::NO,
    }
}

unsafe extern "C" fn native_perform_drag(
    this: &AnyObject,
    _cmd: Sel,
    sender: &ProtocolObject<dyn NSDraggingInfo>,
) -> Bool {
    let state = native_drop_state(this);
    match &*state.ivars().active.borrow() {
        NativeDropMode::File => unsafe {
            msg_send![
                super(this, state.ivars().previous_class),
                performDragOperation: sender
            ]
        },
        NativeDropMode::Offer(offer) => {
            let Some(point) = state.point(sender) else {
                return Bool::NO;
            };
            if state.target(offer, point).is_none() {
                state.queue(MacNativeDropPending::Exit);
                return Bool::NO;
            }
            state.queue(MacNativeDropPending::Drop {
                offer: offer.clone(),
                point,
            });
            Bool::YES
        }
        NativeDropMode::None => Bool::NO,
    }
}

unsafe extern "C" fn native_conclude_drag(
    this: &AnyObject,
    _cmd: Sel,
    sender: Option<&ProtocolObject<dyn NSDraggingInfo>>,
) {
    let state = native_drop_state(this);
    let active = std::mem::replace(
        &mut *state.ivars().active.borrow_mut(),
        NativeDropMode::None,
    );
    state.reset_hover();
    if matches!(active, NativeDropMode::File) {
        unsafe {
            let _: () = msg_send![
                super(this, state.ivars().previous_class),
                concludeDragOperation: sender
            ];
        }
    }
}

unsafe extern "C" fn native_wants_periodic_drag_updates(_this: &AnyObject, _cmd: Sel) -> Bool {
    Bool::NO
}

/// Extends Winit's existing NSWindow drag destination while forwarding its Finder file path.
pub(crate) struct MacNativeDropHost {
    window: Retained<NSWindow>,
    delegate: Retained<AnyObject>,
    associated: Retained<QuickGuiNativeDropState>,
}

impl MacNativeDropHost {
    pub(crate) fn new(
        window: &Arc<Window>,
        handle: WindowHandle,
        proxy: EventLoopProxy<RuntimeEvent>,
        typed_registry: MacTypedDragRegistry,
    ) -> Result<Self, String> {
        MainThreadMarker::new().ok_or_else(|| {
            "native drop handling must be installed on the AppKit main thread".to_owned()
        })?;
        let raw = window
            .window_handle()
            .map_err(|error| format!("could not access the AppKit window handle: {error}"))?;
        let RawWindowHandle::AppKit(raw) = raw.as_raw() else {
            return Err("the active window does not expose an AppKit view".to_owned());
        };
        let content_view = unsafe { Retained::retain(raw.ns_view.as_ptr().cast::<NSView>()) }
            .ok_or_else(|| "the AppKit content view could not be retained".to_owned())?;
        let native_window = content_view
            .window()
            .ok_or_else(|| "the AppKit content view is not attached to a window".to_owned())?;
        let delegate = unsafe { native_window.delegate() }
            .ok_or_else(|| "the AppKit window has no Winit delegate".to_owned())?;
        let delegate: Retained<AnyObject> = unsafe { Retained::cast(delegate) };
        let existing = unsafe {
            objc_getAssociatedObject(
                Retained::as_ptr(&delegate) as *const _,
                native_drop_associated_object_key(),
            )
        };
        if !existing.is_null() {
            return Err("native drop handling is already installed on this window".to_owned());
        }

        let previous_class = unsafe { &*(delegate.class() as *const AnyClass) };
        let subclass = {
            let mut subclasses = NATIVE_DROP_SUBCLASSES
                .lock()
                .map_err(|_| "native-drop class registry is poisoned".to_owned())?;
            if let Some((_, subclass)) = subclasses
                .iter()
                .find(|(candidate, _)| *candidate == previous_class)
            {
                *subclass
            } else {
                let name = format!("QuickGuiNativeDropOf{}", previous_class.name());
                let mut builder = ClassBuilder::new(&name, previous_class).ok_or_else(|| {
                    format!("could not declare Objective-C native-drop subclass {name}")
                })?;
                unsafe {
                    builder.add_method(
                        sel!(draggingEntered:),
                        native_drag_entered as unsafe extern "C" fn(_, _, _) -> _,
                    );
                    builder.add_method(
                        sel!(draggingUpdated:),
                        native_drag_updated as unsafe extern "C" fn(_, _, _) -> _,
                    );
                    builder.add_method(
                        sel!(draggingExited:),
                        native_drag_exited as unsafe extern "C" fn(_, _, _),
                    );
                    builder.add_method(
                        sel!(prepareForDragOperation:),
                        native_prepare_drag as unsafe extern "C" fn(_, _, _) -> _,
                    );
                    builder.add_method(
                        sel!(performDragOperation:),
                        native_perform_drag as unsafe extern "C" fn(_, _, _) -> _,
                    );
                    builder.add_method(
                        sel!(concludeDragOperation:),
                        native_conclude_drag as unsafe extern "C" fn(_, _, _),
                    );
                    builder.add_method(
                        sel!(wantsPeriodicDraggingUpdates),
                        native_wants_periodic_drag_updates as unsafe extern "C" fn(_, _) -> _,
                    );
                }
                let subclass = builder.register();
                subclasses.push((previous_class, subclass));
                subclass
            }
        };
        let associated = QuickGuiNativeDropState::new(
            previous_class,
            content_view,
            handle,
            proxy,
            typed_registry,
        );
        unsafe {
            objc_setAssociatedObject(
                Retained::as_ptr(&delegate) as *mut _,
                native_drop_associated_object_key(),
                Retained::as_ptr(&associated) as *mut _,
                OBJC_ASSOCIATION_RETAIN_NONATOMIC,
            );
            object_setClass(
                Retained::as_ptr(&delegate) as *mut _,
                (subclass as *const AnyClass).cast(),
            );
        }
        let host = Self {
            window: native_window,
            delegate,
            associated,
        };
        host.register_types(0);
        Ok(host)
    }

    fn register_types(&self, capabilities: u8) {
        let mut types = vec![unsafe { NSFilenamesPboardType.copy() }];
        if capabilities & NATIVE_DROP_TEXT != 0 {
            types.push(unsafe { NSPasteboardTypeString.copy() });
        }
        if capabilities & NATIVE_DROP_URL != 0 {
            types.push(unsafe { NSPasteboardTypeURL.copy() });
        }
        if capabilities & NATIVE_DROP_TYPED != 0 {
            types.push(
                self.associated
                    .ivars()
                    .typed_registry
                    .pasteboard_type
                    .clone(),
            );
        }
        // AppKit does not expose the current registration set. Replace it synchronously on the main
        // thread and always restore Winit's legacy Finder type as the first entry.
        unsafe {
            self.window.unregisterDraggedTypes();
        }
        self.window
            .registerForDraggedTypes(&NSArray::from_vec(types));
    }

    pub(crate) fn update(
        &self,
        has_text: bool,
        has_url: bool,
        has_typed: bool,
        update_snapshot: impl FnOnce(&mut ExternalDropSnapshot),
    ) {
        let capabilities = (u8::from(has_text) * NATIVE_DROP_TEXT)
            | (u8::from(has_url) * NATIVE_DROP_URL)
            | (u8::from(has_typed) * NATIVE_DROP_TYPED);
        if self.associated.ivars().capabilities.replace(capabilities) != capabilities {
            self.register_types(capabilities);
        }
        let mut snapshot = self.associated.ivars().snapshot.borrow_mut();
        if capabilities == 0 {
            snapshot.clear();
        } else {
            update_snapshot(&mut snapshot);
        }
        drop(snapshot);

        let offer = match &*self.associated.ivars().active.borrow() {
            NativeDropMode::Offer(offer) => Some(offer.clone()),
            NativeDropMode::None | NativeDropMode::File => None,
        };
        if let (Some(offer), Some(point)) = (offer, self.associated.ivars().last_point.get()) {
            self.associated.hover(&offer, point, false);
        }
    }

    pub(crate) fn take_pending(&self) -> Option<MacNativeDropPending> {
        self.associated.ivars().event_queued.set(false);
        self.associated.ivars().pending.borrow_mut().take()
    }
}

impl Drop for MacNativeDropHost {
    fn drop(&mut self) {
        self.register_types(0);
        let previous_class = self.associated.ivars().previous_class;
        unsafe {
            object_setClass(
                Retained::as_ptr(&self.delegate) as *mut _,
                (previous_class as *const AnyClass).cast(),
            );
            objc_setAssociatedObject(
                Retained::as_ptr(&self.delegate) as *mut _,
                native_drop_associated_object_key(),
                null_mut(),
                OBJC_ASSOCIATION_RETAIN_NONATOMIC,
            );
        }
    }
}

struct TypedDragWriterIvars {
    pasteboard_type: Retained<NSString>,
    token: Retained<NSString>,
    fallback: Option<Retained<AnyObject>>,
}

declare_class!(
    struct QuickGuiTypedDragWriter;

    unsafe impl ClassType for QuickGuiTypedDragWriter {
        type Super = NSObject;
        type Mutability = MainThreadOnly;
        const NAME: &'static str = "QuickGuiTypedDragWriter";
    }

    impl DeclaredClass for QuickGuiTypedDragWriter {
        type Ivars = TypedDragWriterIvars;
    }

    unsafe impl NSObjectProtocol for QuickGuiTypedDragWriter {}

    unsafe impl NSPasteboardWriting for QuickGuiTypedDragWriter {
        #[method_id(writableTypesForPasteboard:)]
        fn writable_types(&self, pasteboard: &NSPasteboard) -> Retained<NSArray<NSPasteboardType>> {
            let mut types = Vec::with_capacity(4);
            types.push(self.ivars().pasteboard_type.clone());
            if let Some(fallback) = &self.ivars().fallback {
                let fallback_types: Retained<NSArray<NSPasteboardType>> = unsafe {
                    msg_send_id![fallback.as_ref(), writableTypesForPasteboard: pasteboard]
                };
                for index in 0..fallback_types.len().min(MAX_COMPOSITE_PASTEBOARD_TYPES) {
                    if let Some(value) = fallback_types.get_retained(index)
                        && value.as_ref() != self.ivars().pasteboard_type.as_ref()
                    {
                        types.push(value);
                    }
                }
            }
            NSArray::from_vec(types)
        }

        #[method_id(pasteboardPropertyListForType:)]
        fn property_list(&self, value_type: &NSPasteboardType) -> Option<Retained<AnyObject>> {
            if value_type == self.ivars().pasteboard_type.as_ref() {
                Some(unsafe { Retained::cast(self.ivars().token.clone()) })
            } else {
                self.ivars().fallback.as_ref().and_then(|fallback| unsafe {
                    msg_send_id![fallback.as_ref(), pasteboardPropertyListForType: value_type]
                })
            }
        }
    }
);

impl QuickGuiTypedDragWriter {
    fn new(
        mtm: MainThreadMarker,
        pasteboard_type: Retained<NSString>,
        token: Retained<NSString>,
        fallback: Option<&ProtocolObject<dyn NSPasteboardWriting>>,
    ) -> Retained<Self> {
        let fallback = fallback.and_then(|fallback| unsafe {
            Retained::retain(
                (fallback as *const ProtocolObject<dyn NSPasteboardWriting>)
                    .cast::<AnyObject>()
                    .cast_mut(),
            )
        });
        let allocated = mtm.alloc().set_ivars(TypedDragWriterIvars {
            pasteboard_type,
            token,
            fallback,
        });
        unsafe { msg_send_id![super(allocated), init] }
    }
}

struct ExternalDragSourceIvars {
    proxy: EventLoopProxy<RuntimeEvent>,
    window: WindowHandle,
}

declare_class!(
    struct QuickGuiExternalDragSource;

    unsafe impl ClassType for QuickGuiExternalDragSource {
        type Super = NSObject;
        type Mutability = MainThreadOnly;
        const NAME: &'static str = "QuickGuiExternalDragSource";
    }

    impl DeclaredClass for QuickGuiExternalDragSource {
        type Ivars = ExternalDragSourceIvars;
    }

    unsafe impl NSObjectProtocol for QuickGuiExternalDragSource {}

    unsafe impl NSDraggingSource for QuickGuiExternalDragSource {
        #[method(draggingSession:sourceOperationMaskForDraggingContext:)]
        fn source_operation_mask(
            &self,
            _session: &NSDraggingSession,
            _context: NSDraggingContext,
        ) -> NSDragOperation {
            // Payloads are offered as copies. Advertising move without a source-side commit
            // callback could let a destination mutate application-owned data unexpectedly.
            NSDragOperation::Copy
        }

        #[method(draggingSession:endedAtPoint:operation:)]
        fn dragging_session_ended(
            &self,
            _session: &NSDraggingSession,
            _screen_point: NSPoint,
            operation: NSDragOperation,
        ) {
            let operation = external_drag_operation(operation);
            let _ = self
                .ivars()
                .proxy
                .send_event(RuntimeEvent::ExternalDragEnded(
                    self.ivars().window,
                    operation,
                ));
        }
    }
);

impl QuickGuiExternalDragSource {
    fn new(
        mtm: MainThreadMarker,
        window: WindowHandle,
        proxy: EventLoopProxy<RuntimeEvent>,
    ) -> Retained<Self> {
        let allocated = mtm
            .alloc()
            .set_ivars(ExternalDragSourceIvars { proxy, window });
        unsafe { msg_send_id![super(allocated), init] }
    }
}

fn external_drag_operation(operation: NSDragOperation) -> ExternalDragOperation {
    if operation.contains(NSDragOperation::Delete) {
        ExternalDragOperation::Deleted
    } else if operation.contains(NSDragOperation::Move) {
        ExternalDragOperation::Moved
    } else if operation.contains(NSDragOperation::Link) {
        ExternalDragOperation::Linked
    } else if operation.contains(NSDragOperation::Copy) {
        ExternalDragOperation::Copied
    } else if operation == NSDragOperation::None {
        ExternalDragOperation::Cancelled
    } else {
        ExternalDragOperation::Other
    }
}

/// The exact AppKit left-mouse-down event required to promote a later internal drag.
#[derive(Clone)]
pub(crate) struct MacMouseDownEvent(Retained<NSEvent>);

/// Retains the AppKit source and session until the native destination finishes the drag.
pub(crate) struct MacExternalDragSession {
    _source: Retained<QuickGuiExternalDragSource>,
    _session: Retained<NSDraggingSession>,
    _typed_registration: MacTypedDragRegistration,
}

/// Lazily observes AppKit drag motion after Winit's cursor leaves the content view.
///
/// Winit can stop delivering `CursorMoved` before an `NSView` tracking area emits `CursorLeft`
/// during a captured primary-button gesture. The local monitor sends at most one boundary event
/// per armed gesture and returns the original event unchanged.
pub(crate) struct MacExternalDragMonitor {
    event_monitor: Option<Retained<AnyObject>>,
    enabled: Arc<AtomicBool>,
    boundary_sent: Arc<AtomicBool>,
}

impl MacExternalDragMonitor {
    pub(crate) fn new(
        window: &Arc<Window>,
        window_handle: WindowHandle,
        proxy: EventLoopProxy<RuntimeEvent>,
    ) -> Result<Self, String> {
        MainThreadMarker::new().ok_or_else(|| {
            "external drag monitoring must be installed on the AppKit main thread".to_owned()
        })?;
        let handle = window
            .window_handle()
            .map_err(|error| format!("could not access the AppKit window handle: {error}"))?;
        let RawWindowHandle::AppKit(handle) = handle.as_raw() else {
            return Err("the active window does not expose an AppKit view".to_owned());
        };
        let view = unsafe { Retained::retain(handle.ns_view.as_ptr().cast::<NSView>()) }
            .ok_or_else(|| "the AppKit content view could not be retained".to_owned())?;
        let native_window = view
            .window()
            .ok_or_else(|| "the AppKit content view is not attached to a window".to_owned())?;
        let enabled = Arc::new(AtomicBool::new(false));
        let boundary_sent = Arc::new(AtomicBool::new(false));
        let monitor_enabled = Arc::clone(&enabled);
        let monitor_boundary_sent = Arc::clone(&boundary_sent);
        let monitor_block = RcBlock::new(move |event: NonNull<NSEvent>| {
            if monitor_enabled.load(Ordering::Relaxed)
                && !monitor_boundary_sent.load(Ordering::Relaxed)
            {
                let belongs_to_window = MainThreadMarker::new().is_some_and(|mtm| unsafe {
                    event.as_ref().window(mtm).is_some_and(|event_window| {
                        Retained::as_ptr(&event_window) == Retained::as_ptr(&native_window)
                    })
                });
                if belongs_to_window {
                    let window_point = unsafe { event.as_ref().locationInWindow() };
                    let point = view.convertPoint_fromView(window_point, None);
                    let bounds = view.bounds();
                    if point_outside_ns_rect(point, bounds)
                        && !monitor_boundary_sent.swap(true, Ordering::Relaxed)
                    {
                        let _ = proxy.send_event(RuntimeEvent::ExternalDragBoundary(
                            window_handle,
                            Point::new(point.x as f32, point.y as f32),
                        ));
                    }
                }
            }
            event.as_ptr()
        });
        let event_monitor = unsafe {
            NSEvent::addLocalMonitorForEventsMatchingMask_handler(
                NSEventMask::LeftMouseDragged,
                &monitor_block,
            )
        }
        .ok_or_else(|| "could not monitor AppKit drag motion".to_owned())?;
        Ok(Self {
            event_monitor: Some(event_monitor),
            enabled,
            boundary_sent,
        })
    }

    pub(crate) fn arm(&self) {
        self.boundary_sent.store(false, Ordering::Relaxed);
        self.enabled.store(true, Ordering::Relaxed);
    }

    pub(crate) fn disarm(&self) {
        self.enabled.store(false, Ordering::Relaxed);
        self.boundary_sent.store(false, Ordering::Relaxed);
    }
}

fn point_outside_ns_rect(point: NSPoint, bounds: NSRect) -> bool {
    point.x < bounds.origin.x
        || point.y < bounds.origin.y
        || point.x > bounds.origin.x + bounds.size.width
        || point.y > bounds.origin.y + bounds.size.height
}

impl Drop for MacExternalDragMonitor {
    fn drop(&mut self) {
        if let Some(event_monitor) = self.event_monitor.take() {
            unsafe {
                NSEvent::removeMonitor(&event_monitor);
            }
        }
    }
}

struct PopoverWatch {
    handle: WindowHandle,
    window: Retained<NSWindow>,
    anchor_window: Retained<NSWindow>,
    anchor_screen_rect: NSRect,
}

#[derive(Default)]
struct PopoverMonitorState {
    watches: Vec<PopoverWatch>,
    dismiss_pending: bool,
}

impl PopoverMonitorState {
    fn request_top_dismiss(&mut self, event: &NSEvent) -> Option<(WindowHandle, bool)> {
        let watch = self.watches.last()?;
        let event_window = MainThreadMarker::new().and_then(|mtm| unsafe { event.window(mtm) });
        let inside = event_window
            .as_ref()
            .is_some_and(|window| popover_window_contains(&watch.window, window.as_ref()));
        if inside || self.dismiss_pending {
            return None;
        }
        let same_window = event_window
            .as_ref()
            .is_some_and(|window| std::ptr::eq(watch.anchor_window.as_ref(), window.as_ref()));
        let window_point = unsafe { event.locationInWindow() };
        let screen_point = event_window.as_ref().map_or(window_point, |window| unsafe {
            window.convertPointToScreen(window_point)
        });
        let consume_anchor_press = should_consume_popover_anchor_press(
            unsafe { event.r#type() },
            same_window,
            watch.anchor_screen_rect,
            screen_point,
        );
        self.dismiss_pending = true;
        Some((watch.handle, consume_anchor_press))
    }
}

fn should_consume_popover_anchor_press(
    event_type: NSEventType,
    same_window: bool,
    anchor_rect: NSRect,
    point: NSPoint,
) -> bool {
    event_type == NSEventType::LeftMouseDown
        && same_window
        && point.x.is_finite()
        && point.y.is_finite()
        && !point_outside_ns_rect(point, anchor_rect)
}

fn popover_window_contains(root: &NSWindow, candidate: &NSWindow) -> bool {
    if std::ptr::eq(root, candidate) {
        return true;
    }
    let mut parent = unsafe { candidate.parentWindow() };
    for _ in 0..MAX_GRABBING_POPOVERS {
        let Some(window) = parent else {
            return false;
        };
        if std::ptr::eq(root, window.as_ref()) {
            return true;
        }
        parent = unsafe { window.parentWindow() };
    }
    false
}

/// One lazy local AppKit event monitor services every nested grabbing popover.
///
/// The monitor is absent while no popover owns a grab, so this path adds no idle mouse-event work
/// to ordinary windows. Entries are bounded and the topmost popover alone owns dismissal semantics.
/// Cross-application dismissal is deliberately handled after `NSApplication` has resigned active;
/// closing from a global mouse monitor can race AppKit's activation handoff and return key appearance
/// to the owner window.
pub(crate) struct MacPopoverMonitor {
    proxy: EventLoopProxy<RuntimeEvent>,
    state: Rc<RefCell<PopoverMonitorState>>,
    local_monitor: Option<Retained<AnyObject>>,
}

impl MacPopoverMonitor {
    pub(crate) fn new(proxy: EventLoopProxy<RuntimeEvent>) -> Self {
        Self {
            proxy,
            state: Rc::new(RefCell::new(PopoverMonitorState::default())),
            local_monitor: None,
        }
    }

    fn install(&mut self) -> Result<(), String> {
        if self.local_monitor.is_some() {
            return Ok(());
        }
        MainThreadMarker::new().ok_or_else(|| {
            "popover event monitoring must be installed on the AppKit main thread".to_owned()
        })?;
        let mask =
            NSEventMask::LeftMouseDown | NSEventMask::RightMouseDown | NSEventMask::OtherMouseDown;

        let local_state = Rc::clone(&self.state);
        let local_proxy = self.proxy.clone();
        let local_block = RcBlock::new(move |event: NonNull<NSEvent>| {
            let dismiss = local_state
                .borrow_mut()
                .request_top_dismiss(unsafe { event.as_ref() });
            if let Some((handle, consume_anchor_press)) = dismiss {
                let _ =
                    local_proxy.send_event(RuntimeEvent::PopoverPointerDismissRequested(handle));
                if consume_anchor_press {
                    // The active anchor is a close affordance. Consume its mouse-down so the
                    // owner cannot receive a later click and reopen after dismissal state lands.
                    return std::ptr::null_mut();
                }
            }
            event.as_ptr()
        });
        self.local_monitor = Some(
            unsafe { NSEvent::addLocalMonitorForEventsMatchingMask_handler(mask, &local_block) }
                .ok_or_else(|| "could not install the AppKit popover event monitor".to_owned())?,
        );
        Ok(())
    }

    fn uninstall(&mut self) {
        if let Some(monitor) = self.local_monitor.take() {
            unsafe { NSEvent::removeMonitor(&monitor) };
        }
    }

    pub(crate) fn watch(
        &mut self,
        handle: WindowHandle,
        window: &Arc<Window>,
        anchor_window: &Arc<Window>,
        anchor_rect: Rect,
    ) -> Result<(), String> {
        let window = appkit_window(window)?;
        let anchor_view = appkit_view(anchor_window)?;
        let anchor_window = anchor_view
            .window()
            .ok_or_else(|| "the popover anchor view is not attached to a window".to_owned())?;
        let parent_content = anchor_window.contentRectForFrameRect(anchor_window.frame());
        let anchor_screen_rect = NSRect::new(
            NSPoint::new(
                parent_content.origin.x + f64::from(anchor_rect.x),
                parent_content.origin.y + parent_content.size.height
                    - f64::from(anchor_rect.bottom()),
            ),
            NSSize::new(f64::from(anchor_rect.width), f64::from(anchor_rect.height)),
        );
        {
            let mut state = self.state.borrow_mut();
            state.watches.retain(|watch| watch.handle != handle);
            if state.watches.len() == MAX_GRABBING_POPOVERS {
                return Err(format!(
                    "an application cannot retain more than {MAX_GRABBING_POPOVERS} grabbing popovers"
                ));
            }
            state.watches.push(PopoverWatch {
                handle,
                window,
                anchor_window,
                anchor_screen_rect,
            });
            state.dismiss_pending = false;
        }
        if let Err(error) = self.install() {
            self.state
                .borrow_mut()
                .watches
                .retain(|watch| watch.handle != handle);
            return Err(error);
        }
        Ok(())
    }

    pub(crate) fn unwatch(&mut self, handle: WindowHandle) {
        let empty = {
            let mut state = self.state.borrow_mut();
            let previous_top = state.watches.last().map(|watch| watch.handle);
            state.watches.retain(|watch| watch.handle != handle);
            let top = state.watches.last().map(|watch| watch.handle);
            if top != previous_top {
                state.dismiss_pending = false;
            }
            state.watches.is_empty()
        };
        if empty {
            self.uninstall();
        }
    }
}

impl Drop for MacPopoverMonitor {
    fn drop(&mut self) {
        self.uninstall();
    }
}

/// Capture the native event synchronously while Winit delivers its matching button press.
pub(crate) fn capture_left_mouse_down() -> Option<MacMouseDownEvent> {
    let mtm = MainThreadMarker::new()?;
    let event = NSApplication::sharedApplication(mtm).currentEvent()?;
    (unsafe { event.r#type() } == NSEventType::LeftMouseDown).then_some(MacMouseDownEvent(event))
}

/// Promote one process-local typed value plus an optional public payload to AppKit.
pub(crate) fn start_external_drag(
    window: &Arc<Window>,
    mouse_down: &MacMouseDownEvent,
    payload: Option<&ExternalDragPayload>,
    typed_payload: MacTypedDragPayload,
    typed_registry: &MacTypedDragRegistry,
    window_handle: WindowHandle,
    proxy: EventLoopProxy<RuntimeEvent>,
) -> Result<MacExternalDragSession, String> {
    let mtm = MainThreadMarker::new()
        .ok_or_else(|| "external dragging must start on the AppKit main thread".to_owned())?;
    let (token, typed_registration) = typed_registry.register(typed_payload)?;
    let token = NSString::from_str(&token);
    let pasteboard_type = typed_registry.pasteboard_type.clone();

    let handle = window
        .window_handle()
        .map_err(|error| format!("could not access the AppKit window handle: {error}"))?;
    let RawWindowHandle::AppKit(handle) = handle.as_raw() else {
        return Err("the active window does not expose an AppKit view".to_owned());
    };
    let view = unsafe { Retained::retain(handle.ns_view.as_ptr().cast::<NSView>()) }
        .ok_or_else(|| "the AppKit content view could not be retained".to_owned())?;
    let window_point = unsafe { mouse_down.0.locationInWindow() };
    let location = view.convertPoint_fromView(window_point, None);
    let frame = NSRect::new(
        NSPoint::new(location.x - 16.0, location.y - 16.0),
        NSSize::new(32.0, 32.0),
    );
    let (items, formation) = match payload {
        Some(ExternalDragPayload::Files(paths)) => {
            if paths.is_empty() {
                return Err("the external drag contains no retained paths".to_owned());
            }
            let file_icon = external_drag_icon("doc.fill");
            let directory_icon = external_drag_icon("folder.fill");
            let mut items = Vec::with_capacity(paths.entries().len());
            for (path, is_directory) in paths.entries() {
                let Ok(path) = CString::new(path.as_os_str().as_bytes()) else {
                    tracing::warn!("external drag skipped a path containing an interior nul byte");
                    continue;
                };
                let path = NonNull::new(path.as_ptr().cast_mut())
                    .expect("a CString always exposes a non-null pointer");
                let url = unsafe {
                    NSURL::fileURLWithFileSystemRepresentation_isDirectory_relativeToURL(
                        path,
                        *is_directory,
                        None,
                    )
                };
                let writer: &ProtocolObject<dyn NSPasteboardWriting> =
                    ProtocolObject::from_ref(url.as_ref());
                let icon = if *is_directory {
                    directory_icon.as_deref()
                } else {
                    file_icon.as_deref()
                };
                items.push(typed_dragging_item(
                    mtm,
                    pasteboard_type.clone(),
                    token.clone(),
                    Some(writer),
                    frame,
                    icon,
                ));
            }
            if items.is_empty() {
                return Err("none of the retained paths could be represented by AppKit".to_owned());
            }
            (items, NSDraggingFormation::Stack)
        }
        Some(ExternalDragPayload::Text(text)) => {
            if text.is_empty() {
                return Err("the external text drag is empty".to_owned());
            }
            let text = NSString::from_str(text.as_str());
            let writer: &ProtocolObject<dyn NSPasteboardWriting> =
                ProtocolObject::from_ref(text.as_ref());
            let icon = external_drag_icon("text.alignleft");
            (
                vec![typed_dragging_item(
                    mtm,
                    pasteboard_type.clone(),
                    token.clone(),
                    Some(writer),
                    frame,
                    icon.as_deref(),
                )],
                NSDraggingFormation::None,
            )
        }
        Some(ExternalDragPayload::Url(url)) => {
            let url_string = NSString::from_str(url.as_str());
            let url = unsafe { NSURL::URLWithString(&url_string) }
                .ok_or_else(|| "AppKit rejected the external drag URL".to_owned())?;
            let writer: &ProtocolObject<dyn NSPasteboardWriting> =
                ProtocolObject::from_ref(url.as_ref());
            let icon = external_drag_icon("link");
            (
                vec![typed_dragging_item(
                    mtm,
                    pasteboard_type.clone(),
                    token.clone(),
                    Some(writer),
                    frame,
                    icon.as_deref(),
                )],
                NSDraggingFormation::None,
            )
        }
        None => {
            let icon = external_drag_icon("square.dashed");
            (
                vec![typed_dragging_item(
                    mtm,
                    pasteboard_type,
                    token,
                    None,
                    frame,
                    icon.as_deref(),
                )],
                NSDraggingFormation::None,
            )
        }
    };

    let items = NSArray::from_vec(items);
    let source = QuickGuiExternalDragSource::new(mtm, window_handle, proxy);
    let source_protocol: &ProtocolObject<dyn NSDraggingSource> =
        ProtocolObject::from_ref(source.as_ref());
    let session = unsafe {
        view.beginDraggingSessionWithItems_event_source(
            &items,
            mouse_down.0.as_ref(),
            source_protocol,
        )
    };
    unsafe {
        session.setDraggingFormation(formation);
        session.setAnimatesToStartingPositionsOnCancelOrFail(true);
    }
    Ok(MacExternalDragSession {
        _source: source,
        _session: session,
        _typed_registration: typed_registration,
    })
}

fn typed_dragging_item(
    mtm: MainThreadMarker,
    pasteboard_type: Retained<NSString>,
    token: Retained<NSString>,
    fallback: Option<&ProtocolObject<dyn NSPasteboardWriting>>,
    frame: NSRect,
    icon: Option<&NSImage>,
) -> Retained<NSDraggingItem> {
    let writer = QuickGuiTypedDragWriter::new(mtm, pasteboard_type, token, fallback);
    let writer: &ProtocolObject<dyn NSPasteboardWriting> =
        ProtocolObject::from_ref(writer.as_ref());
    external_dragging_item(writer, frame, icon)
}

fn external_dragging_item(
    writer: &ProtocolObject<dyn NSPasteboardWriting>,
    frame: NSRect,
    icon: Option<&NSImage>,
) -> Retained<NSDraggingItem> {
    let item = unsafe { NSDraggingItem::initWithPasteboardWriter(NSDraggingItem::alloc(), writer) };
    unsafe {
        item.setDraggingFrame_contents(frame, icon.map(|icon| icon.as_ref()));
    }
    item
}

fn external_drag_icon(symbol: &str) -> Option<Retained<NSImage>> {
    let symbol = NSString::from_str(symbol);
    let image =
        unsafe { NSImage::imageWithSystemSymbolName_accessibilityDescription(&symbol, None) }?;
    unsafe {
        image.setSize(NSSize::new(32.0, 32.0));
    }
    Some(image)
}

/// Configure AppKit to ask the layer-backed Winit view for fresh content throughout live resize.
///
/// The default policy can preserve and stretch an old layer snapshot. GPU windows instead need a
/// draw callback inside AppKit's resize transaction so their layout and native children stay in
/// lockstep with the window frame.
pub(crate) fn configure_gpu_window_resize(window: &Arc<Window>) -> Result<(), String> {
    MainThreadMarker::new().ok_or_else(|| {
        "GPU window resize must be configured on the AppKit main thread".to_owned()
    })?;
    let handle = window
        .window_handle()
        .map_err(|error| format!("could not access the AppKit window handle: {error}"))?;
    let RawWindowHandle::AppKit(handle) = handle.as_raw() else {
        return Err("the active window does not expose an AppKit view".to_owned());
    };
    let view = unsafe { handle.ns_view.as_ptr().cast::<NSView>().as_ref() }
        .ok_or_else(|| "the AppKit content view pointer is null".to_owned())?;
    unsafe {
        view.setLayerContentsRedrawPolicy(
            NSViewLayerContentsRedrawPolicy::NSViewLayerContentsRedrawDuringViewResize,
        );
    }
    Ok(())
}

/// Enable or disable AppKit's implicit titlebar/background window dragging.
///
/// Hidden-inset windows disable this and opt into exact declarative drag regions instead.
pub(crate) fn set_window_movable(window: &Arc<Window>, movable: bool) -> Result<(), String> {
    MainThreadMarker::new().ok_or_else(|| {
        "window movability must be configured on the AppKit main thread".to_owned()
    })?;
    let handle = window
        .window_handle()
        .map_err(|error| format!("could not access the AppKit window handle: {error}"))?;
    let RawWindowHandle::AppKit(handle) = handle.as_raw() else {
        return Err("the active window does not expose an AppKit view".to_owned());
    };
    let view = unsafe { handle.ns_view.as_ptr().cast::<NSView>().as_ref() }
        .ok_or_else(|| "the AppKit content view pointer is null".to_owned())?;
    let window = view
        .window()
        .ok_or_else(|| "the AppKit content view is not attached to a window".to_owned())?;
    if unsafe { window.isMovable() } != movable {
        window.setMovable(movable);
    }
    if unsafe { window.isMovableByWindowBackground() } {
        window.setMovableByWindowBackground(false);
    }
    Ok(())
}

fn appkit_view(window: &Arc<Window>) -> Result<Retained<NSView>, String> {
    MainThreadMarker::new().ok_or_else(|| {
        "native window state must be changed on the AppKit main thread".to_owned()
    })?;
    let handle = window
        .window_handle()
        .map_err(|error| format!("could not access the AppKit window handle: {error}"))?;
    let RawWindowHandle::AppKit(handle) = handle.as_raw() else {
        return Err("the active window does not expose an AppKit view".to_owned());
    };
    unsafe { Retained::retain(handle.ns_view.as_ptr().cast::<NSView>()) }
        .ok_or_else(|| "the AppKit content view pointer is null".to_owned())
}

fn appkit_window(window: &Arc<Window>) -> Result<Retained<NSWindow>, String> {
    appkit_view(window)?
        .window()
        .ok_or_else(|| "the AppKit content view is not attached to a window".to_owned())
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum MacWindowTabAction {
    SelectNext,
    SelectPrevious,
    Select(usize),
    MergeAll,
    MoveToNewWindow,
    ToggleBar,
    ToggleOverview,
}

/// Configure document-window state before the first hidden surface frame is presented.
pub(crate) fn configure_document_window(
    window: &Arc<Window>,
    represented_file: Option<&Path>,
    document_edited: bool,
    tabbing_identifier: Option<&str>,
) -> Result<(), String> {
    let native = appkit_window(window)?;
    if represented_file.is_some() {
        set_native_represented_file(&native, represented_file)?;
    }
    if document_edited {
        native.setDocumentEdited(true);
    }
    set_native_tabbing_identifier(&native, tabbing_identifier);
    Ok(())
}

pub(crate) fn set_window_represented_file(
    window: &Arc<Window>,
    represented_file: Option<&Path>,
) -> Result<(), String> {
    let native = appkit_window(window)?;
    set_native_represented_file(&native, represented_file)
}

fn set_native_represented_file(
    window: &NSWindow,
    represented_file: Option<&Path>,
) -> Result<(), String> {
    let url = represented_file
        .map(|path| native_file_url(path, false))
        .transpose()?;
    unsafe {
        window.setRepresentedURL(url.as_deref());
    }
    Ok(())
}

pub(crate) fn set_window_document_edited(window: &Arc<Window>, edited: bool) -> Result<(), String> {
    appkit_window(window)?.setDocumentEdited(edited);
    Ok(())
}

pub(crate) fn show_character_palette(window: &Arc<Window>) -> Result<(), String> {
    let mtm = MainThreadMarker::new().ok_or_else(|| {
        "the character palette must be presented on the AppKit main thread".to_owned()
    })?;
    let native = appkit_window(window)?;
    NSApplication::sharedApplication(mtm).orderFrontCharacterPalette(Some(native.as_ref()));
    Ok(())
}

pub(crate) fn set_window_tabbing_identifier(
    window: &Arc<Window>,
    identifier: Option<&str>,
) -> Result<(), String> {
    let native = appkit_window(window)?;
    set_native_tabbing_identifier(&native, identifier);
    Ok(())
}

fn set_native_tabbing_identifier(window: &NSWindow, identifier: Option<&str>) {
    match identifier {
        Some(identifier) => {
            window.setTabbingMode(NSWindowTabbingMode::Preferred);
            window.setTabbingIdentifier(&NSString::from_str(identifier));
        }
        None => {
            window.setTabbingMode(NSWindowTabbingMode::Disallowed);
            window.setTabbingIdentifier(&NSString::from_str(""));
        }
    }
}

pub(crate) fn perform_window_tab_action(
    window: &Arc<Window>,
    action: MacWindowTabAction,
) -> Result<(), String> {
    let native = appkit_window(window)?;
    match action {
        MacWindowTabAction::SelectNext => native.selectNextTab(None),
        MacWindowTabAction::SelectPrevious => unsafe { native.selectPreviousTab(None) },
        MacWindowTabAction::Select(index) => {
            if let Some(group) = native.tabGroup() {
                let windows = group.windows();
                if index < windows.count() {
                    let selected = unsafe { windows.objectAtIndex(index) };
                    group.setSelectedWindow(Some(&selected));
                }
            }
        }
        MacWindowTabAction::MergeAll => unsafe { native.mergeAllWindows(None) },
        MacWindowTabAction::MoveToNewWindow => unsafe { native.moveTabToNewWindow(None) },
        MacWindowTabAction::ToggleBar => unsafe { native.toggleTabBar(None) },
        MacWindowTabAction::ToggleOverview => unsafe { native.toggleTabOverview(None) },
    }
    Ok(())
}

pub(crate) fn window_tab_state(window: &Arc<Window>) -> Result<WindowTabState, String> {
    let native = appkit_window(window)?;
    let Some(group) = native.tabGroup() else {
        return Ok(WindowTabState::default());
    };
    bounded_window_tab_state(&group)
}

fn bounded_window_tab_state(group: &NSWindowTabGroup) -> Result<WindowTabState, String> {
    let windows = group.windows();
    let total = windows.count();
    if total == 0 {
        return Ok(WindowTabState::default());
    }
    let count = total.min(MAX_SYSTEM_WINDOW_TABS);
    let selected = unsafe { group.selectedWindow() };
    let selected_index = selected.as_ref().and_then(|selected| {
        (0..count).find(|index| {
            let candidate = unsafe { windows.objectAtIndex(*index) };
            Retained::as_ptr(&candidate) == Retained::as_ptr(selected)
        })
    });
    Ok(WindowTabState {
        count,
        selected_index,
        tab_bar_visible: unsafe { group.isTabBarVisible() },
        overview_visible: unsafe { group.isOverviewVisible() },
        truncated: total > count,
    })
}

fn deepest_appkit_sheet(window: &Arc<Window>) -> Result<Retained<NSWindow>, String> {
    let mut parent = appkit_window(window)?;
    while let Some(sheet) = unsafe { parent.attachedSheet() } {
        parent = sheet;
    }
    Ok(parent)
}

/// One bounded native sheet retained until AppKit invokes its completion handler.
pub(crate) enum MacPlatformDialog {
    Prompt(Retained<NSAlert>),
    Open(Retained<NSOpenPanel>),
    Save(Retained<NSSavePanel>),
}

#[derive(Clone)]
pub(crate) struct MacPlatformDialogContext {
    owner: Option<WindowHandle>,
    id: PlatformDialogId,
    open: Arc<AtomicBool>,
    proxy: EventLoopProxy<RuntimeEvent>,
}

#[derive(Clone)]
pub(crate) struct MacPlatformDialogFocus {
    window: Retained<NSWindow>,
    responder: Retained<NSResponder>,
}

impl MacPlatformDialogFocus {
    pub(crate) fn capture(window: Option<&Arc<Window>>) -> Result<Option<Self>, String> {
        let Some(window) = window else {
            return Ok(None);
        };
        let window = deepest_appkit_sheet(window)?;
        Ok(window
            .firstResponder()
            .map(|responder| Self { window, responder }))
    }

    pub(crate) fn restore(&self) -> bool {
        if !self.window.isKeyWindow() {
            self.window.makeKeyAndOrderFront(None);
        }
        self.window.makeFirstResponder(Some(&self.responder))
    }
}

impl MacPlatformDialogContext {
    pub(crate) fn new(
        owner: Option<WindowHandle>,
        id: PlatformDialogId,
        open: Arc<AtomicBool>,
        proxy: EventLoopProxy<RuntimeEvent>,
    ) -> Self {
        Self {
            owner,
            id,
            open,
            proxy,
        }
    }
}

impl MacPlatformDialog {
    /// End a still-active native sheet before its owning QuickGUI window is released.
    pub(crate) fn cancel(&self) {
        match self {
            Self::Prompt(alert) => unsafe {
                let sheet = alert.window();
                if let Some(parent) = sheet.sheetParent() {
                    parent.endSheet_returnCode(&sheet, NSModalResponseCancel);
                } else {
                    sheet.close();
                }
            },
            Self::Open(panel) => unsafe { panel.cancel(None) },
            Self::Save(panel) => unsafe { panel.cancel(None) },
        }
    }
}

pub(crate) fn native_file_url(path: &Path, is_directory: bool) -> Result<Retained<NSURL>, String> {
    let path = CString::new(path.as_os_str().as_bytes())
        .map_err(|_| "a native path cannot contain NUL".to_owned())?;
    let path = NonNull::new(path.as_ptr().cast_mut())
        .expect("CString always exposes a non-null filesystem representation");
    Ok(unsafe {
        NSURL::fileURLWithFileSystemRepresentation_isDirectory_relativeToURL(
            path,
            is_directory,
            None,
        )
    })
}

fn native_file_path(url: &NSURL) -> Result<PathBuf, PlatformError> {
    if !unsafe { url.isFileURL() } {
        return Err(PlatformError::Platform(
            "the native panel returned a non-file URL".into(),
        ));
    }
    let path = unsafe { CStr::from_ptr(url.fileSystemRepresentation().as_ptr()) }.to_bytes();
    if path.is_empty() || path.len() > MAX_PLATFORM_PATH_BYTES {
        return Err(PlatformError::SelectionTooLarge);
    }
    Ok(PathBuf::from(std::ffi::OsString::from_vec(path.to_vec())))
}

fn finish_native_dialog<T>(
    context: &MacPlatformDialogContext,
    responder: &PlatformResponder<T>,
    result: Result<T, PlatformError>,
) {
    context.open.store(false, Ordering::Release);
    let _ = context.proxy.send_event(RuntimeEvent::PlatformDialogClosed(
        context.owner,
        context.id,
    ));
    responder.complete(result);
}

pub(crate) fn present_native_prompt(
    window: Option<&Arc<Window>>,
    context: MacPlatformDialogContext,
    level: PromptLevel,
    message: &str,
    detail: Option<&str>,
    buttons: &[PromptButton],
    responder: PlatformResponder<usize>,
) -> Result<MacPlatformDialog, String> {
    let parent = window.map(deepest_appkit_sheet).transpose()?;
    let mtm = MainThreadMarker::new()
        .ok_or_else(|| "native prompts must start on the AppKit main thread".to_owned())?;
    let alert = unsafe { NSAlert::new(mtm) };
    unsafe {
        alert.setAlertStyle(match level {
            PromptLevel::Info => NSAlertStyle::Informational,
            PromptLevel::Warning => NSAlertStyle::Warning,
            PromptLevel::Critical => NSAlertStyle::Critical,
        });
        alert.setMessageText(&NSString::from_str(message));
        if let Some(detail) = detail {
            alert.setInformativeText(&NSString::from_str(detail));
        }
    }

    let initial_focus_index = buttons
        .iter()
        .enumerate()
        .rev()
        .find(|(_, button)| !button.is_cancel())
        .map(|(index, _)| index)
        .filter(|index| *index > 0);
    let mut initial_focus = None;
    for (index, button) in buttons.iter().enumerate() {
        let native = unsafe { alert.addButtonWithTitle(&NSString::from_str(button.label())) };
        if button.is_cancel() {
            unsafe { native.setKeyEquivalent(&NSString::from_str("\u{1b}")) };
        } else if Some(index) == initial_focus_index {
            initial_focus = Some(native);
        }
    }
    if let Some(button) = initial_focus {
        unsafe { alert.window() }.setInitialFirstResponder(Some(&button));
    }

    let button_count = buttons.len();
    let finish_response = move |response: NSModalResponse| {
        let result = response
            .checked_sub(NSAlertFirstButtonReturn)
            .and_then(|index| usize::try_from(index).ok())
            .filter(|index| *index < button_count)
            .ok_or_else(|| {
                PlatformError::Platform("the native prompt closed without an answer".into())
            });
        finish_native_dialog(&context, &responder, result);
    };
    if let Some(parent) = parent {
        let completion = RcBlock::new(finish_response);
        unsafe {
            alert.beginSheetModalForWindow_completionHandler(&parent, Some(&completion));
        }
    } else {
        // NSAlert has no asynchronous application-modal API. Its native modal session still
        // dispatches AppKit events, and this branch is used only when the caller omits a parent.
        let response = unsafe { alert.runModal() };
        finish_response(response);
    }
    Ok(MacPlatformDialog::Prompt(alert))
}

pub(crate) fn shell_open_url(url: &str) -> Result<(), String> {
    MainThreadMarker::new()
        .ok_or_else(|| "URL shell actions must run on the AppKit main thread".to_owned())?;
    let url = unsafe { NSURL::initWithString(NSURL::alloc(), &NSString::from_str(url)) }
        .ok_or_else(|| "the URL is not valid for NSWorkspace".to_owned())?;
    if unsafe { NSWorkspace::sharedWorkspace().openURL(&url) } {
        Ok(())
    } else {
        Err("NSWorkspace rejected the URL".to_owned())
    }
}

pub(crate) fn shell_open_path(path: &Path) -> Result<(), String> {
    MainThreadMarker::new()
        .ok_or_else(|| "path shell actions must run on the AppKit main thread".to_owned())?;
    let url = native_file_url(path, false)?;
    if unsafe { NSWorkspace::sharedWorkspace().openURL(&url) } {
        Ok(())
    } else {
        Err("NSWorkspace rejected the path".to_owned())
    }
}

pub(crate) fn shell_reveal_path(path: &Path) -> Result<(), String> {
    MainThreadMarker::new()
        .ok_or_else(|| "path shell actions must run on the AppKit main thread".to_owned())?;
    let url = native_file_url(path, false)?;
    let urls = NSArray::from_id_slice(&[url]);
    unsafe {
        NSWorkspace::sharedWorkspace().activateFileViewerSelectingURLs(&urls);
    }
    Ok(())
}

pub(crate) fn shell_trash_path(path: &Path) -> Result<(), String> {
    MainThreadMarker::new()
        .ok_or_else(|| "trash actions must run on the AppKit main thread".to_owned())?;
    let url = native_file_url(path, false)?;
    unsafe {
        NSFileManager::defaultManager()
            .trashItemAtURL_resultingItemURL_error(&url, None)
            .map_err(|error| error.localizedDescription().to_string())
    }
}

/// Route the default Cmd-W fallback through AppKit so Winit emits `CloseRequested` normally.
pub(crate) fn perform_window_close(window: &Arc<Window>) -> Result<(), String> {
    unsafe { appkit_window(window)?.performClose(None) };
    Ok(())
}

fn top_left_screen_rect(rect: NSRect, main_screen_height: f32) -> Rect {
    Rect::new(
        rect.origin.x as f32,
        main_screen_height - (rect.origin.y + rect.size.height) as f32,
        rect.size.width as f32,
        rect.size.height as f32,
    )
}

fn appkit_main_screen_height(
    mtm: MainThreadMarker,
) -> Result<(Retained<NSArray<NSScreen>>, f32), String> {
    let screens = NSScreen::screens(mtm);
    let primary = screens
        .iter()
        .find(|screen| {
            let origin = screen.frame().origin;
            origin.x == 0.0 && origin.y == 0.0
        })
        .or_else(|| screens.iter().next())
        .ok_or_else(|| "AppKit reported no screens".to_owned())?;
    Ok((screens.clone(), primary.frame().size.height as f32))
}

/// Read the hardware pointer in QuickGUI's top-left global logical desktop coordinates.
pub(crate) fn current_cursor_screen_position() -> Option<Point> {
    let mtm = MainThreadMarker::new()?;
    let (_, main_screen_height) = appkit_main_screen_height(mtm).ok()?;
    // SAFETY: AppKit's process-wide mouse location is a value-only main-thread query.
    let point = unsafe { NSEvent::mouseLocation() };
    let point = Point::new(point.x as f32, main_screen_height - point.y as f32);
    (point.x.is_finite() && point.y.is_finite()).then_some(point)
}

/// Resolve and apply parent-relative popover geometry while both native windows remain hidden.
pub(crate) fn position_system_popover(
    window: &Arc<Window>,
    parent: &Arc<Window>,
    options: &PopoverOptions,
) -> Result<Rect, String> {
    let mtm = MainThreadMarker::new()
        .ok_or_else(|| "popover placement must be resolved on the AppKit main thread".to_owned())?;
    let child = appkit_window(window)?;
    let parent = appkit_window(parent)?;
    let (screens, main_screen_height) = appkit_main_screen_height(mtm)?;

    let parent_content = parent.contentRectForFrameRect(parent.frame());
    let parent_content = top_left_screen_rect(parent_content, main_screen_height);
    let anchor_rect = Rect::new(
        parent_content.x + options.anchor_rect.x,
        parent_content.y + options.anchor_rect.y,
        options.anchor_rect.width,
        options.anchor_rect.height,
    );
    let anchor_center = Point::new(
        anchor_rect.x + anchor_rect.width * 0.5,
        anchor_rect.y + anchor_rect.height * 0.5,
    );
    let screen = screens
        .iter()
        .find(|screen| {
            top_left_screen_rect(screen.frame(), main_screen_height).contains(anchor_center)
        })
        .map(|screen| screen.retain())
        .or_else(|| parent.screen())
        .or_else(|| NSScreen::mainScreen(mtm))
        .ok_or_else(|| "AppKit could not resolve the popover's target screen".to_owned())?;
    let visible = top_left_screen_rect(screen.visibleFrame(), main_screen_height);
    if visible.is_empty() {
        return Err("AppKit reported an empty popover work area".to_owned());
    }

    let child_content = child.contentRectForFrameRect(child.frame());
    let size = Size::new(
        child_content.size.width as f32,
        child_content.size.height as f32,
    );
    let resolved = crate::popover::place_popover(anchor_rect, size, visible, options);
    let content_frame = NSRect::new(
        NSPoint::new(
            resolved.x as f64,
            (main_screen_height - resolved.bottom()) as f64,
        ),
        NSSize::new(resolved.width as f64, resolved.height as f64),
    );
    let frame = unsafe { child.frameRectForContentRect(content_frame) };
    child.setFrame_display(frame, false);
    Ok(resolved)
}

/// Order a native window without accidentally making a `focus(false)` window key.
pub(crate) fn set_window_visibility(
    window: &Arc<Window>,
    visible: bool,
    focus: bool,
) -> Result<(), String> {
    let window = appkit_window(window)?;
    if visible {
        if focus {
            window.makeKeyAndOrderFront(None);
        } else {
            window.orderFront(None);
        }
    } else {
        window.orderOut(None);
    }
    Ok(())
}

/// Change whether one Winit-owned AppKit window may become key.
pub(crate) fn set_window_focusable(window: &Arc<Window>, focusable: bool) -> Result<(), String> {
    if window.set_can_become_key_window(focusable) {
        Ok(())
    } else {
        Err("the native window did not expose key-window policy".to_owned())
    }
}

/// Apply whole-window alpha without changing the retained GPU scene.
pub(crate) fn set_window_opacity(window: &Arc<Window>, opacity: f32) -> Result<(), String> {
    if !opacity.is_finite() || !(0.0..=1.0).contains(&opacity) {
        return Err("window opacity must be finite and between zero and one".to_owned());
    }
    let window = appkit_window(window)?;
    // SAFETY: QuickGUI owns this live main-thread AppKit window and the bounded scalar carries no
    // borrowed Objective-C state.
    unsafe { window.setAlphaValue(f64::from(opacity)) };
    Ok(())
}

/// Toggle AppKit's all-spaces collection behavior while preserving role-specific flags.
pub(crate) fn set_window_visible_on_all_workspaces(
    window: &Arc<Window>,
    visible: bool,
) -> Result<(), String> {
    let window = appkit_window(window)?;
    // SAFETY: QuickGUI owns this live main-thread AppKit window for both synchronous calls.
    let mut behavior = unsafe { window.collectionBehavior() };
    if visible {
        behavior |= NSWindowCollectionBehavior::CanJoinAllSpaces;
    } else {
        behavior &= !NSWindowCollectionBehavior::CanJoinAllSpaces;
    }
    unsafe { window.setCollectionBehavior(behavior) };
    Ok(())
}

/// Apply native z-level, space, and animation semantics for one GPUI-shaped window role.
pub(crate) fn configure_window_kind(
    window: &Arc<Window>,
    kind: WindowKind,
    focus: bool,
    accepts_key_focus: bool,
) -> Result<(), String> {
    if matches!(kind, WindowKind::Popover | WindowKind::SystemPopover)
        && !window.set_panel_can_become_key_window(accepts_key_focus)
    {
        return Err("a popover panel did not expose key-window policy".to_owned());
    }
    let window = appkit_window(window)?;
    match kind {
        WindowKind::Normal | WindowKind::Dialog => window.setLevel(NSNormalWindowLevel),
        WindowKind::Floating => window.setLevel(NSFloatingWindowLevel),
        WindowKind::Popover | WindowKind::SystemPopover => unsafe {
            if !window.isKindOfClass(NSPanel::class()) {
                return Err("a popover was not allocated as an AppKit NSPanel".to_owned());
            }
            let panel: Retained<NSPanel> = Retained::cast(window.clone());
            panel.setFloatingPanel(true);
            panel.setBecomesKeyOnlyIfNeeded(!focus);
            window.setLevel(NSPopUpMenuWindowLevel);
            window.setHidesOnDeactivate(true);
            window.setAnimationBehavior(NSWindowAnimationBehavior::UtilityWindow);
            window.setCollectionBehavior(
                NSWindowCollectionBehavior::CanJoinAllSpaces
                    | NSWindowCollectionBehavior::FullScreenAuxiliary
                    | NSWindowCollectionBehavior::Transient,
            );
        },
    }
    Ok(())
}

/// Present a parent-owned role after QuickGUI has completed its hidden first surface frame.
pub(crate) fn present_window_relation(
    window: &Arc<Window>,
    parent: Option<&Arc<Window>>,
    kind: WindowKind,
) -> Result<bool, String> {
    let Some(parent) = parent else {
        return Ok(false);
    };
    let child = appkit_window(window)?;
    if kind == WindowKind::SystemPopover {
        let parent = appkit_window(parent)?;
        if let Some(previous) = unsafe { child.parentWindow() }
            && Retained::as_ptr(&previous) != Retained::as_ptr(&parent)
        {
            unsafe { previous.removeChildWindow(&child) };
        }
        if unsafe { child.parentWindow() }.is_none() {
            unsafe {
                parent.addChildWindow_ordered(&child, NSWindowOrderingMode::NSWindowAbove);
            }
        }
        return Ok(true);
    }
    if kind != WindowKind::Dialog {
        return Ok(false);
    }
    let mut parent = appkit_window(parent)?;
    while let Some(sheet) = unsafe { parent.attachedSheet() } {
        parent = sheet;
    }
    unsafe {
        parent.beginSheet_completionHandler(&child, None);
    }
    Ok(true)
}

/// End an AppKit parent-owned presentation before hiding or releasing the Winit window.
pub(crate) fn dismiss_window_relation(
    window: &Arc<Window>,
    kind: WindowKind,
) -> Result<(), String> {
    let child = appkit_window(window)?;
    if kind == WindowKind::SystemPopover {
        if let Some(parent) = unsafe { child.parentWindow() } {
            unsafe { parent.removeChildWindow(&child) };
        }
        return Ok(());
    }
    if kind != WindowKind::Dialog {
        return Ok(());
    }
    if let Some(parent) = unsafe { child.sheetParent() } {
        unsafe {
            parent.endSheet(&child);
        }
    }
    Ok(())
}

pub(crate) fn is_window_fullscreen(window: &Arc<Window>) -> Result<bool, String> {
    Ok(appkit_window(window)?
        .styleMask()
        .contains(NSWindowStyleMask::FullScreen))
}

/// Query AppKit without Winit's temporary style-mask mutation.
///
/// Winit adds `Titled | Resizable` around `isZoomed` for borderless windows. That mutation emits
/// resize and move callbacks, so asking from one of those callbacks can recurse indefinitely.
/// QuickGUI tracks programmatic borderless maximization itself and uses this direct query only for
/// native titled windows.
pub(crate) fn is_window_maximized(window: &Arc<Window>) -> Result<bool, String> {
    Ok(appkit_window(window)?.isZoomed())
}

/// Read the current pointer in the Winit content view's logical, top-left coordinate space.
///
/// Winit's file-hover events do not carry a position on macOS, and AppKit does not synthesize
/// ordinary mouse-move events during a native drag. Reading the window's current location at the
/// enter and submit boundaries keeps element-level file drops exact without polling.
pub(crate) fn current_pointer_position(window: &Arc<Window>) -> Option<Point> {
    MainThreadMarker::new()?;
    let handle = window.window_handle().ok()?;
    let RawWindowHandle::AppKit(handle) = handle.as_raw() else {
        return None;
    };
    let view = unsafe { handle.ns_view.as_ptr().cast::<NSView>().as_ref() }?;
    let window = view.window()?;
    let window_point = unsafe { window.mouseLocationOutsideOfEventStream() };
    let point = view.convertPoint_fromView(window_point, None);
    (point.x.is_finite() && point.y.is_finite()).then(|| Point::new(point.x as f32, point.y as f32))
}

/// Start AppKit's native window drag for the current mouse-down event.
///
/// AppKit's implicit movability is enabled only for the synchronous native drag and restored
/// immediately afterward, keeping hidden-inset windows restricted to declared app regions.
pub(crate) fn perform_window_drag(
    window: &Arc<Window>,
    restore_immovable: bool,
) -> Result<(), String> {
    if restore_immovable {
        set_window_movable(window, true)?;
    }
    let result = window
        .drag_window()
        .map_err(|error| format!("could not start the native window drag: {error}"));
    if restore_immovable {
        set_window_movable(window, false)?;
    }
    result
}

/// Move the standard AppKit window controls while preserving their native spacing and behavior.
///
/// QuickGUI positions the close button from the top-left in logical points, matching web-style
/// window APIs. AppKit uses a bottom-left coordinate system, so the vertical coordinate is
/// converted using the native content-layout rect rather than a hard-coded titlebar height.
pub(crate) fn position_traffic_lights(window: &Arc<Window>, position: Point) -> Result<(), String> {
    MainThreadMarker::new()
        .ok_or_else(|| "traffic lights must be positioned on the AppKit main thread".to_owned())?;
    if !position.x.is_finite() || !position.y.is_finite() || position.x < 0.0 || position.y < 0.0 {
        return Err("traffic-light coordinates must be finite and non-negative".to_owned());
    }

    let handle = window
        .window_handle()
        .map_err(|error| format!("could not access the AppKit window handle: {error}"))?;
    let RawWindowHandle::AppKit(handle) = handle.as_raw() else {
        return Err("the active window does not expose an AppKit view".to_owned());
    };
    let view = unsafe { handle.ns_view.as_ptr().cast::<NSView>().as_ref() }
        .ok_or_else(|| "the AppKit content view pointer is null".to_owned())?;
    let window = view
        .window()
        .ok_or_else(|| "the AppKit content view is not attached to a window".to_owned())?;
    if window.styleMask().contains(NSWindowStyleMask::FullScreen) {
        return Ok(());
    }

    let close = window
        .standardWindowButton(NSWindowButton::NSWindowCloseButton)
        .ok_or_else(|| "the AppKit close button is unavailable".to_owned())?;
    let minimize = window
        .standardWindowButton(NSWindowButton::NSWindowMiniaturizeButton)
        .ok_or_else(|| "the AppKit minimize button is unavailable".to_owned())?;
    let zoom = window.standardWindowButton(NSWindowButton::NSWindowZoomButton);

    let close_frame = NSView::frame(&close);
    let minimize_frame = NSView::frame(&minimize);
    let spacing = minimize_frame.origin.x - close_frame.origin.x;
    if !spacing.is_finite() || spacing <= 0.0 {
        return Err("AppKit returned invalid traffic-light spacing".to_owned());
    }
    let frame = window.frame();
    let content_layout = unsafe { window.contentLayoutRect() };
    let titlebar_height = frame.size.height - content_layout.size.height;
    if !titlebar_height.is_finite() || titlebar_height <= 0.0 {
        return Err("AppKit returned an invalid titlebar height".to_owned());
    }

    let mut x = f64::from(position.x);
    for button in [Some(close), Some(minimize), zoom].into_iter().flatten() {
        let button_frame = NSView::frame(&button);
        let origin = traffic_light_origin(position, titlebar_height, button_frame.size.height, x);
        if (button_frame.origin.x - origin.x).abs() > 0.01
            || (button_frame.origin.y - origin.y).abs() > 0.01
        {
            unsafe {
                NSView::setFrameOrigin(&button, origin);
            }
        }
        x += spacing;
    }
    Ok(())
}

fn traffic_light_origin(
    position: Point,
    titlebar_height: f64,
    button_height: f64,
    x: f64,
) -> NSPoint {
    NSPoint::new(x, titlebar_height - f64::from(position.y) - button_height)
}

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

    /// Whether Winit can safely route APIs that look up the window's current content view.
    pub fn content_attached(&self) -> bool {
        self.parent.window().is_some()
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
                hosted.clip_view.setAlphaValue(f64::from(placement.opacity));
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

#[cfg(test)]
mod tests {
    use super::*;

    fn test_typed_payload(value: Arc<dyn Any>) -> MacTypedDragPayload {
        let value_type = value.as_ref().type_id();
        MacTypedDragPayload::new(
            value,
            value_type,
            WindowHandle::next(),
            ElementId::named("typed-source"),
        )
    }

    #[test]
    fn typed_drag_registry_retains_the_exact_shared_value_for_one_session() {
        let registry = MacTypedDragRegistry::new();
        let shared = Arc::new(String::from("process-local"));
        let erased: Arc<dyn Any> = shared.clone();
        let shared_pointer = Arc::as_ptr(&erased) as *const ();
        let payload = test_typed_payload(erased);

        let (token, registration) = registry
            .register(payload.clone())
            .expect("the first typed drag fits the registry");
        assert!(token.len() <= 64);
        assert!(registry.resolve("forged-token").is_none());
        let resolved = registry
            .resolve(&token)
            .expect("the live token resolves inside this application");
        assert_eq!(resolved.value_type, TypeId::of::<String>());
        assert_eq!(Arc::as_ptr(&resolved.value) as *const (), shared_pointer);
        assert_eq!(resolved.value.downcast_ref::<String>(), Some(&*shared));
        assert_eq!(resolved.source_window(), payload.source_window());
        assert_eq!(resolved.source(), payload.source());

        drop(registration);
        assert!(registry.resolve(&token).is_none());
    }

    #[test]
    fn typed_drag_registry_has_a_hard_session_cap_and_recovers_capacity() {
        let registry = MacTypedDragRegistry::new();
        let payload = test_typed_payload(Arc::new(7_u32));
        let mut registrations = Vec::with_capacity(MAX_NATIVE_TYPED_DRAG_SESSIONS);
        for _ in 0..MAX_NATIVE_TYPED_DRAG_SESSIONS {
            let (_, registration) = registry
                .register(payload.clone())
                .expect("every slot up to the documented cap is available");
            registrations.push(registration);
        }
        assert!(registry.register(payload.clone()).is_err());
        drop(registrations.pop());
        let (_, replacement) = registry
            .register(payload)
            .expect("dropping one session immediately returns its registry slot");
        registrations.push(replacement);
        assert_eq!(
            registry.inner.borrow().payloads.len(),
            MAX_NATIVE_TYPED_DRAG_SESSIONS
        );
    }

    #[test]
    fn native_drop_offer_keeps_private_typed_data_ahead_of_public_fallbacks() {
        let typed = test_typed_payload(Arc::new(11_u32));
        let offer = MacNativeDropOffer::new(vec![
            MacNativeDropPayload::Typed(typed),
            MacNativeDropPayload::Text(Arc::new(ExternalDragText::new("fallback"))),
            MacNativeDropPayload::Url(Arc::new(
                ExternalDragUrl::new("https://quickgui.dev").unwrap(),
            )),
        ])
        .expect("the offer is not empty");
        assert_eq!(
            offer
                .iter()
                .map(|(value_type, _)| value_type)
                .collect::<Vec<_>>(),
            vec![
                TypeId::of::<u32>(),
                TypeId::of::<ExternalDragText>(),
                TypeId::of::<ExternalDragUrl>(),
            ]
        );
    }

    #[test]
    fn native_drag_operations_map_to_stable_framework_values() {
        assert_eq!(
            external_drag_operation(NSDragOperation::None),
            ExternalDragOperation::Cancelled
        );
        assert_eq!(
            external_drag_operation(NSDragOperation::Copy),
            ExternalDragOperation::Copied
        );
        assert_eq!(
            external_drag_operation(NSDragOperation::Move),
            ExternalDragOperation::Moved
        );
        assert_eq!(
            external_drag_operation(NSDragOperation::Link),
            ExternalDragOperation::Linked
        );
        assert_eq!(
            external_drag_operation(NSDragOperation::Delete),
            ExternalDragOperation::Deleted
        );
        assert_eq!(
            external_drag_operation(NSDragOperation::Generic),
            ExternalDragOperation::Other
        );
    }

    #[test]
    fn native_drag_boundary_includes_the_content_view_edges() {
        let bounds = NSRect::new(NSPoint::new(10.0, 20.0), NSSize::new(80.0, 60.0));
        assert!(!point_outside_ns_rect(NSPoint::new(10.0, 20.0), bounds));
        assert!(!point_outside_ns_rect(NSPoint::new(90.0, 80.0), bounds));
        assert!(point_outside_ns_rect(NSPoint::new(9.9, 50.0), bounds));
        assert!(point_outside_ns_rect(NSPoint::new(50.0, 80.1), bounds));
    }

    #[test]
    fn system_popover_consumes_only_a_left_press_inside_its_own_anchor() {
        let anchor = NSRect::new(NSPoint::new(20.0, 30.0), NSSize::new(80.0, 40.0));
        let inside = NSPoint::new(99.9, 69.9);

        assert!(should_consume_popover_anchor_press(
            NSEventType::LeftMouseDown,
            true,
            anchor,
            inside,
        ));
        assert!(!should_consume_popover_anchor_press(
            NSEventType::RightMouseDown,
            true,
            anchor,
            inside,
        ));
        assert!(!should_consume_popover_anchor_press(
            NSEventType::LeftMouseDown,
            false,
            anchor,
            inside,
        ));
        assert!(!should_consume_popover_anchor_press(
            NSEventType::LeftMouseDown,
            true,
            anchor,
            NSPoint::new(100.1, 70.1),
        ));
    }

    #[test]
    fn native_text_and_url_writers_create_dragging_items() {
        let frame = NSRect::new(NSPoint::new(4.0, 8.0), NSSize::new(32.0, 32.0));
        let text = NSString::from_str("QuickGUI native text");
        let text_writer: &ProtocolObject<dyn NSPasteboardWriting> =
            ProtocolObject::from_ref(text.as_ref());
        let text_item = external_dragging_item(text_writer, frame, None);
        assert_eq!(unsafe { text_item.draggingFrame() }, frame);

        let url_string = NSString::from_str("https://github.com/egoist/quickgui");
        let url = unsafe { NSURL::URLWithString(&url_string) }.expect("the test URL is valid");
        let url_writer: &ProtocolObject<dyn NSPasteboardWriting> =
            ProtocolObject::from_ref(url.as_ref());
        let url_item = external_dragging_item(url_writer, frame, None);
        assert_eq!(unsafe { url_item.draggingFrame() }, frame);
    }

    #[test]
    fn native_drop_strings_are_bounded_on_utf8_scalar_boundaries() {
        let short = NSString::from_str("QuickGUI");
        assert_eq!(
            bounded_pasteboard_string(&short, 16),
            Some(("QuickGUI".to_owned(), false))
        );

        let long = NSString::from_str("abcéz");
        assert_eq!(
            bounded_pasteboard_string(&long, 4),
            Some(("abc".to_owned(), true))
        );

        let exact = NSString::from_str("éé");
        assert_eq!(
            bounded_pasteboard_string(&exact, 4),
            Some(("éé".to_owned(), false))
        );
    }

    #[test]
    fn traffic_light_top_left_coordinates_convert_to_appkit_space() {
        assert_eq!(
            traffic_light_origin(Point::new(16.0, 6.0), 28.0, 14.0, 16.0),
            NSPoint::new(16.0, 8.0)
        );
    }
}
