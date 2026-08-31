use std::{
    cell::RefCell,
    collections::HashSet,
    ffi::{CString, c_char, c_void},
    fmt,
    panic::{AssertUnwindSafe, catch_unwind},
    ptr::NonNull,
    rc::Rc,
    sync::Arc,
};

use objc2_foundation::MainThreadMarker;
use serde::Serialize;

use crate::{MacNativeView, Size, WindowHandle};

type ActionCallback = unsafe extern "C" fn(*mut c_void, u64);
type PresentationCallback = unsafe extern "C" fn(*mut c_void, u64, bool);

unsafe extern "C" {
    fn quickgui_swift_ui_host_create(
        context: *mut c_void,
        callback: Option<ActionCallback>,
        presentation_callback: Option<PresentationCallback>,
    ) -> *mut c_void;
    fn quickgui_swift_ui_host_view(handle: *mut c_void) -> *mut c_void;
    fn quickgui_swift_ui_host_update(handle: *mut c_void, json: *const c_char) -> bool;
    fn quickgui_swift_ui_host_set_embedded_view(
        handle: *mut c_void,
        id: u64,
        view: *mut c_void,
        width: f64,
        height: f64,
    ) -> bool;
    fn quickgui_swift_ui_host_remove_embedded_view(handle: *mut c_void, id: u64);
    fn quickgui_swift_ui_host_fitting_size(handle: *mut c_void, width: *mut f64, height: *mut f64);
    fn quickgui_swift_ui_host_release(handle: *mut c_void);
}

/// Semantic role forwarded to a native SwiftUI button.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum SwiftUiButtonRole {
    #[default]
    Default,
    Cancel,
    Destructive,
}

/// Native SwiftUI button style. Glass variants fall back to their bordered counterparts before
/// macOS 26.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum SwiftUiButtonStyle {
    #[default]
    Automatic,
    Bordered,
    BorderedProminent,
    Borderless,
    Plain,
    Glass,
    GlassProminent,
}

/// Native control sizing forwarded to SwiftUI.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum SwiftUiControlSize {
    Mini,
    Small,
    #[default]
    Regular,
    Large,
    ExtraLarge,
}

/// Border shape used by styled native SwiftUI buttons.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum SwiftUiButtonBorderShape {
    #[default]
    Automatic,
    Capsule,
    RoundedRectangle,
    Circle,
}

/// Presentation of a SwiftUI `Label` used as a button label.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum SwiftUiLabelStyle {
    #[default]
    Automatic,
    IconOnly,
    TitleAndIcon,
    TitleOnly,
}

/// Ordered SwiftUI view modifier supported by QuickGUI's native button bridge.
#[derive(Clone, Debug, PartialEq)]
pub enum SwiftUiModifier {
    ButtonStyle(SwiftUiButtonStyle),
    ButtonBorderShape {
        shape: SwiftUiButtonBorderShape,
        corner_radius: Option<f32>,
    },
    ControlSize(SwiftUiControlSize),
    LabelStyle(SwiftUiLabelStyle),
    Tint(Arc<str>),
    Disabled(bool),
}

/// One declarative button inside a [`MacSwiftUiHost`].
#[derive(Clone, Debug, PartialEq)]
pub struct SwiftUiButton {
    pub id: u64,
    pub label: Option<Arc<str>>,
    pub system_image: Option<Arc<str>>,
    pub role: SwiftUiButtonRole,
    pub target: Option<Arc<str>>,
    pub test_id: Option<Arc<str>>,
    pub modifiers: Vec<SwiftUiModifier>,
    pub has_action: bool,
}

/// A retained QuickGUI renderer surface that can be mounted by a native SwiftUI descriptor.
///
/// The platform view is installed asynchronously at the normal window-creation boundary. Clones
/// retain the same surface identity and content-size state; dropping this value does not close the
/// surface's core window.
#[derive(Clone)]
pub struct MacEmbeddedView {
    window: WindowHandle,
    pub(crate) owner: WindowHandle,
    pub(crate) inner: Rc<RefCell<MacEmbeddedViewState>>,
}

#[derive(Clone, Debug)]
pub(crate) struct MacEmbeddedViewState {
    pub(crate) view: Option<MacNativeView>,
    pub(crate) content_size: Size,
    pub(crate) match_horizontal: bool,
    pub(crate) match_vertical: bool,
}

impl MacEmbeddedView {
    pub(crate) fn pending(
        window: WindowHandle,
        owner: WindowHandle,
        initial_size: Size,
        match_horizontal: bool,
        match_vertical: bool,
    ) -> Self {
        Self {
            window,
            owner,
            inner: Rc::new(RefCell::new(MacEmbeddedViewState {
                view: None,
                content_size: initial_size,
                match_horizontal,
                match_vertical,
            })),
        }
    }

    /// Stable core window handle used by the embedded QuickGUI subtree.
    pub const fn window_handle(&self) -> WindowHandle {
        self.window
    }

    /// Retained AppKit rendering view once the pending core window has been created.
    pub fn view(&self) -> Option<MacNativeView> {
        self.inner.borrow().view.clone()
    }

    /// Latest max-content measurement published by the embedded retained tree.
    pub fn content_size(&self) -> Size {
        self.inner.borrow().content_size
    }

    pub(crate) fn install(&self, view: MacNativeView) {
        self.inner.borrow_mut().view = Some(view);
    }

    pub(crate) fn update_content_size(&self, size: Size) -> bool {
        let mut state = self.inner.borrow_mut();
        let size = Size::new(size.width.max(1.0), size.height.max(1.0));
        if (state.content_size.width - size.width).abs() < 0.25
            && (state.content_size.height - size.height).abs() < 0.25
        {
            return false;
        }
        state.content_size = size;
        true
    }

    pub(crate) fn match_axes(&self) -> (bool, bool) {
        let state = self.inner.borrow();
        (state.match_horizontal, state.match_vertical)
    }
}

impl fmt::Debug for MacEmbeddedView {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("MacEmbeddedView")
            .field("window", &self.window)
            .field("owner", &self.owner)
            .field("ready", &self.inner.borrow().view.is_some())
            .field("content_size", &self.inner.borrow().content_size)
            .finish()
    }
}

/// One ordinary QuickGUI subtree embedded back into a SwiftUI host.
#[derive(Clone, Debug)]
pub struct SwiftUiQuickGuiHost {
    pub id: u64,
    pub embedded: Option<MacEmbeddedView>,
    pub match_horizontal: bool,
    pub match_vertical: bool,
    pub width: Option<f32>,
    pub height: Option<f32>,
    pub test_id: Option<Arc<str>>,
}

impl SwiftUiQuickGuiHost {
    pub fn new(id: u64, embedded: MacEmbeddedView) -> Self {
        Self {
            id,
            embedded: Some(embedded),
            match_horizontal: false,
            match_vertical: false,
            width: None,
            height: None,
            test_id: None,
        }
    }

    pub fn pending(id: u64) -> Self {
        Self {
            id,
            embedded: None,
            match_horizontal: false,
            match_vertical: false,
            width: None,
            height: None,
            test_id: None,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum SwiftUiPopoverAttachmentAnchor {
    #[default]
    Center,
    Top,
    Bottom,
    Leading,
    Trailing,
}

#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum SwiftUiPopoverArrowEdge {
    Top,
    #[default]
    Bottom,
    Leading,
    Trailing,
}

/// A controlled native SwiftUI popover with explicit trigger and content slots.
#[derive(Clone, Debug)]
pub struct SwiftUiPopover {
    pub id: u64,
    pub is_presented: bool,
    pub attachment_anchor: SwiftUiPopoverAttachmentAnchor,
    pub arrow_edge: SwiftUiPopoverArrowEdge,
    pub trigger: Vec<SwiftUiElement>,
    pub content: Vec<SwiftUiElement>,
    pub test_id: Option<Arc<str>>,
}

impl SwiftUiPopover {
    pub fn new(id: u64) -> Self {
        Self {
            id,
            is_presented: false,
            attachment_anchor: SwiftUiPopoverAttachmentAnchor::Center,
            arrow_edge: SwiftUiPopoverArrowEdge::Bottom,
            trigger: Vec::new(),
            content: Vec::new(),
            test_id: None,
        }
    }
}

/// A node in the native SwiftUI descriptor tree.
#[derive(Clone, Debug)]
pub enum SwiftUiElement {
    Button(SwiftUiButton),
    QuickGuiHost(SwiftUiQuickGuiHost),
    Popover(SwiftUiPopover),
}

impl From<SwiftUiButton> for SwiftUiElement {
    fn from(value: SwiftUiButton) -> Self {
        Self::Button(value)
    }
}

impl SwiftUiButton {
    pub fn new(id: u64) -> Self {
        Self {
            id,
            label: None,
            system_image: None,
            role: SwiftUiButtonRole::Default,
            target: None,
            test_id: None,
            modifiers: Vec::new(),
            has_action: false,
        }
    }

    pub fn label(mut self, label: impl Into<Arc<str>>) -> Self {
        self.label = Some(label.into());
        self
    }

    pub fn system_image(mut self, name: impl Into<Arc<str>>) -> Self {
        self.system_image = Some(name.into());
        self
    }

    pub fn role(mut self, role: SwiftUiButtonRole) -> Self {
        self.role = role;
        self
    }

    pub fn target(mut self, target: impl Into<Arc<str>>) -> Self {
        self.target = Some(target.into());
        self
    }

    pub fn test_id(mut self, test_id: impl Into<Arc<str>>) -> Self {
        self.test_id = Some(test_id.into());
        self
    }

    pub fn modifier(mut self, modifier: SwiftUiModifier) -> Self {
        self.modifiers.push(modifier);
        self
    }

    pub fn modifiers(mut self, modifiers: impl IntoIterator<Item = SwiftUiModifier>) -> Self {
        self.modifiers.extend(modifiers);
        self
    }

    /// Convenience builder equivalent to Expo's `buttonStyle` modifier.
    pub fn style(mut self, style: SwiftUiButtonStyle) -> Self {
        self.modifiers.push(SwiftUiModifier::ButtonStyle(style));
        self
    }

    /// Convenience builder equivalent to Expo's `controlSize` modifier.
    pub fn control_size(mut self, size: SwiftUiControlSize) -> Self {
        self.modifiers.push(SwiftUiModifier::ControlSize(size));
        self
    }

    /// Convenience builder equivalent to Expo's `disabled` modifier.
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.modifiers.push(SwiftUiModifier::Disabled(disabled));
        self
    }

    pub fn action(mut self, enabled: bool) -> Self {
        self.has_action = enabled;
        self
    }
}

#[derive(Serialize)]
#[serde(
    tag = "type",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
enum BridgeElement<'a> {
    #[serde(rename = "button")]
    Button {
        id: u64,
        #[serde(skip_serializing_if = "Option::is_none")]
        label: Option<&'a str>,
        #[serde(skip_serializing_if = "Option::is_none")]
        system_image: Option<&'a str>,
        role: SwiftUiButtonRole,
        #[serde(skip_serializing_if = "Option::is_none")]
        target: Option<&'a str>,
        #[serde(skip_serializing_if = "Option::is_none")]
        test_id: Option<&'a str>,
        modifiers: Vec<BridgeModifier<'a>>,
        has_action: bool,
    },
    #[serde(rename = "quickGuiHost")]
    QuickGuiHost {
        id: u64,
        match_horizontal: bool,
        match_vertical: bool,
        #[serde(skip_serializing_if = "Option::is_none")]
        width: Option<f32>,
        #[serde(skip_serializing_if = "Option::is_none")]
        height: Option<f32>,
        #[serde(skip_serializing_if = "Option::is_none")]
        test_id: Option<&'a str>,
    },
    #[serde(rename = "popover")]
    Popover {
        id: u64,
        is_presented: bool,
        attachment_anchor: SwiftUiPopoverAttachmentAnchor,
        arrow_edge: SwiftUiPopoverArrowEdge,
        trigger: Vec<BridgeElement<'a>>,
        content: Vec<BridgeElement<'a>>,
        #[serde(skip_serializing_if = "Option::is_none")]
        test_id: Option<&'a str>,
    },
}

impl<'a> From<&'a SwiftUiElement> for BridgeElement<'a> {
    fn from(element: &'a SwiftUiElement) -> Self {
        match element {
            SwiftUiElement::Button(button) => Self::Button {
                id: button.id,
                label: button.label.as_deref(),
                system_image: button.system_image.as_deref(),
                role: button.role,
                target: button.target.as_deref(),
                test_id: button.test_id.as_deref(),
                modifiers: button.modifiers.iter().map(BridgeModifier::from).collect(),
                has_action: button.has_action,
            },
            SwiftUiElement::QuickGuiHost(host) => Self::QuickGuiHost {
                id: host.id,
                match_horizontal: host.match_horizontal,
                match_vertical: host.match_vertical,
                width: host.width,
                height: host.height,
                test_id: host.test_id.as_deref(),
            },
            SwiftUiElement::Popover(popover) => Self::Popover {
                id: popover.id,
                is_presented: popover.is_presented,
                attachment_anchor: popover.attachment_anchor,
                arrow_edge: popover.arrow_edge,
                trigger: popover.trigger.iter().map(Self::from).collect(),
                content: popover.content.iter().map(Self::from).collect(),
                test_id: popover.test_id.as_deref(),
            },
        }
    }
}

#[derive(Serialize)]
#[serde(tag = "$type")]
enum BridgeModifier<'a> {
    #[serde(rename = "buttonStyle")]
    ButtonStyle { style: SwiftUiButtonStyle },
    #[serde(rename = "buttonBorderShape")]
    ButtonBorderShape {
        shape: SwiftUiButtonBorderShape,
        #[serde(rename = "cornerRadius", skip_serializing_if = "Option::is_none")]
        corner_radius: Option<f32>,
    },
    #[serde(rename = "controlSize")]
    ControlSize { size: SwiftUiControlSize },
    #[serde(rename = "labelStyle")]
    LabelStyle { style: SwiftUiLabelStyle },
    #[serde(rename = "tint")]
    Tint { color: &'a str },
    #[serde(rename = "disabled")]
    Disabled { disabled: bool },
}

impl<'a> From<&'a SwiftUiModifier> for BridgeModifier<'a> {
    fn from(modifier: &'a SwiftUiModifier) -> Self {
        match modifier {
            SwiftUiModifier::ButtonStyle(style) => Self::ButtonStyle { style: *style },
            SwiftUiModifier::ButtonBorderShape {
                shape,
                corner_radius,
            } => Self::ButtonBorderShape {
                shape: *shape,
                corner_radius: *corner_radius,
            },
            SwiftUiModifier::ControlSize(size) => Self::ControlSize { size: *size },
            SwiftUiModifier::LabelStyle(style) => Self::LabelStyle { style: *style },
            SwiftUiModifier::Tint(color) => Self::Tint { color },
            SwiftUiModifier::Disabled(disabled) => Self::Disabled {
                disabled: *disabled,
            },
        }
    }
}

struct ActionContext {
    callback: Box<dyn Fn(u64)>,
    presentation_callback: Box<dyn Fn(u64, bool)>,
}

unsafe extern "C" fn dispatch_action(context: *mut c_void, id: u64) {
    let Some(context) = NonNull::new(context).map(|context| context.cast::<ActionContext>()) else {
        return;
    };
    // An application callback must never unwind across Swift's C ABI boundary.
    let _ = catch_unwind(AssertUnwindSafe(|| unsafe {
        (context.as_ref().callback)(id);
    }));
}

unsafe extern "C" fn dispatch_presentation(context: *mut c_void, id: u64, presented: bool) {
    let Some(context) = NonNull::new(context).map(|context| context.cast::<ActionContext>()) else {
        return;
    };
    let _ = catch_unwind(AssertUnwindSafe(|| unsafe {
        (context.as_ref().presentation_callback)(id, presented);
    }));
}

/// A retained `NSHostingView` whose SwiftUI contents are synchronized from Rust descriptors.
///
/// Construction, updates, sizing, and destruction must happen on AppKit's main thread. QuickGUI's
/// runtime satisfies that contract when this host is used from a retained view.
pub struct MacSwiftUiHost {
    handle: NonNull<c_void>,
    view: MacNativeView,
    action_context: Box<ActionContext>,
    last_payload: String,
    embedded_ids: HashSet<u64>,
}

impl MacSwiftUiHost {
    pub fn new(action: impl Fn(u64) + 'static) -> Result<Self, String> {
        Self::new_with_events(action, |_id, _presented| {})
    }

    pub fn new_with_events(
        action: impl Fn(u64) + 'static,
        presentation: impl Fn(u64, bool) + 'static,
    ) -> Result<Self, String> {
        require_main_thread()?;
        let mut action_context = Box::new(ActionContext {
            callback: Box::new(action),
            presentation_callback: Box::new(presentation),
        });
        let context = (&mut *action_context as *mut ActionContext).cast::<c_void>();
        let handle = NonNull::new(unsafe {
            quickgui_swift_ui_host_create(
                context,
                Some(dispatch_action),
                Some(dispatch_presentation),
            )
        })
        .ok_or_else(|| "SwiftUI did not create an NSHostingView".to_owned())?;
        let view_pointer = NonNull::new(unsafe { quickgui_swift_ui_host_view(handle.as_ptr()) })
            .ok_or_else(|| {
                unsafe { quickgui_swift_ui_host_release(handle.as_ptr()) };
                "SwiftUI created a host without an NSView".to_owned()
            })?;
        let view = unsafe { &*view_pointer.as_ptr().cast::<objc2_app_kit::NSView>() };
        Ok(Self {
            handle,
            view: MacNativeView::new(view),
            action_context,
            last_payload: String::new(),
            embedded_ids: HashSet::new(),
        })
    }

    pub fn view(&self) -> &objc2_app_kit::NSView {
        self.view.as_ns_view()
    }

    /// Synchronize the complete ordered SwiftUI descriptor tree. Returns whether the native root
    /// view or one of its embedded QuickGUI surfaces changed.
    pub fn sync(&mut self, elements: &[SwiftUiElement]) -> Result<bool, String> {
        require_main_thread()?;
        let bridge = elements.iter().map(BridgeElement::from).collect::<Vec<_>>();
        let payload = serde_json::to_string(&bridge)
            .map_err(|error| format!("could not encode SwiftUI elements: {error}"))?;

        let mut embedded = Vec::new();
        collect_embedded_views(elements, &mut embedded);
        let next_ids = embedded.iter().map(|(id, _)| *id).collect::<HashSet<_>>();
        let removed = self
            .embedded_ids
            .difference(&next_ids)
            .copied()
            .collect::<Vec<_>>();
        for id in removed {
            unsafe { quickgui_swift_ui_host_remove_embedded_view(self.handle.as_ptr(), id) };
        }
        let mut changed = next_ids != self.embedded_ids;
        for (id, embedded) in embedded {
            let Some(view) = embedded.view() else {
                continue;
            };
            let size = embedded.content_size();
            if !unsafe {
                quickgui_swift_ui_host_set_embedded_view(
                    self.handle.as_ptr(),
                    id,
                    view.as_ns_view() as *const _ as *mut c_void,
                    f64::from(size.width),
                    f64::from(size.height),
                )
            } {
                return Err(format!("SwiftUI rejected embedded QuickGUI view {id}"));
            }
            changed = true;
        }
        self.embedded_ids = next_ids;

        if payload != self.last_payload {
            let json = CString::new(payload.as_str())
                .map_err(|_| "encoded SwiftUI elements unexpectedly contained NUL".to_owned())?;
            if !unsafe { quickgui_swift_ui_host_update(self.handle.as_ptr(), json.as_ptr()) } {
                return Err("SwiftUI rejected its Rust element description".to_owned());
            }
            self.last_payload = payload;
            changed = true;
        }
        Ok(changed)
    }

    /// Compatibility convenience for the original button-only bridge.
    pub fn sync_buttons(&mut self, buttons: &[SwiftUiButton]) -> Result<bool, String> {
        let elements = buttons
            .iter()
            .cloned()
            .map(SwiftUiElement::Button)
            .collect::<Vec<_>>();
        self.sync(&elements)
    }

    /// Current fitting size of the hosted SwiftUI subtree, including space for native control
    /// effects such as Liquid Glass expansion.
    pub fn fitting_size(&self) -> Result<Size, String> {
        require_main_thread()?;
        let mut width = 0.0;
        let mut height = 0.0;
        unsafe {
            quickgui_swift_ui_host_fitting_size(self.handle.as_ptr(), &mut width, &mut height);
        }
        if !width.is_finite() || !height.is_finite() || width <= 0.0 || height <= 0.0 {
            return Err(format!(
                "SwiftUI returned an invalid fitting size {width}x{height}"
            ));
        }
        Ok(Size::new(width as f32, height as f32))
    }
}

fn collect_embedded_views<'a>(
    elements: &'a [SwiftUiElement],
    output: &mut Vec<(u64, &'a MacEmbeddedView)>,
) {
    for element in elements {
        match element {
            SwiftUiElement::Button(_) => {}
            SwiftUiElement::QuickGuiHost(host) => {
                if let Some(embedded) = &host.embedded {
                    output.push((host.id, embedded));
                }
            }
            SwiftUiElement::Popover(popover) => {
                collect_embedded_views(&popover.trigger, output);
                collect_embedded_views(&popover.content, output);
            }
        }
    }
}

impl fmt::Debug for MacSwiftUiHost {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("MacSwiftUiHost")
            .field("handle", &self.handle)
            .field("view", &self.view)
            .finish_non_exhaustive()
    }
}

impl Drop for MacSwiftUiHost {
    fn drop(&mut self) {
        let _ = &self.action_context;
        unsafe { quickgui_swift_ui_host_release(self.handle.as_ptr()) };
    }
}

fn require_main_thread() -> Result<MainThreadMarker, String> {
    MainThreadMarker::new()
        .ok_or_else(|| "SwiftUI hosts must be accessed on the AppKit main thread".to_owned())
}
