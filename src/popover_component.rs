use std::sync::Arc;

use crate::{
    AccessibilityPopup, AccessibilityRole, AnchorPlacement, Color, Element, ElementId,
    EventContext, FocusHandle, MAX_WINDOW_LOGICAL_COORDINATE, MAX_WINDOW_LOGICAL_DIMENSION, Point,
    PopupAnchor, PopupConstraintAdjustment, PopupGravity, PopupOptions, Rect, Size, View,
    WindowBackgroundAppearance, WindowCommandError, WindowHandle, WindowOptions, button, div,
};

const MAX_POPOVER_ANCHOR_GAP: f32 = 256.0;
const MAX_POPOVER_VIEWPORT_MARGIN: f32 = 512.0;
const DEFAULT_POPOVER_ANCHOR_GAP: f32 = 6.0;
const DEFAULT_POPOVER_VIEWPORT_MARGIN: f32 = 8.0;
const POPOVER_POSITIONER_ID_TAG: u64 = 0x847d_5a0f_0f9b_31e7;
const POPOVER_BACKDROP_ID_TAG: u64 = 0x663e_a690_8d7f_c442;
const POPOVER_TITLE_ID_TAG: u64 = 0x23ce_95af_9dc3_481b;
const POPOVER_DESCRIPTION_ID_TAG: u64 = 0xa935_070d_46db_78c1;
const POPOVER_CLOSE_ID_TAG: u64 = 0xd55f_271c_bbd9_e6a4;

/// Semantic content exposed by a controlled [`Popover`].
///
/// The value drives both the surface role and the trigger's AccessKit `has-popup` state. It does
/// not select a different renderer or retain component state.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum PopoverKind {
    #[default]
    Dialog,
    Menu,
    ListBox,
    Tree,
    Grid,
}

impl PopoverKind {
    const fn accessibility_role(self) -> AccessibilityRole {
        match self {
            Self::Dialog => AccessibilityRole::Dialog,
            Self::Menu => AccessibilityRole::Menu,
            Self::ListBox => AccessibilityRole::ListBox,
            Self::Tree => AccessibilityRole::Tree,
            Self::Grid => AccessibilityRole::Grid,
        }
    }

    const fn accessibility_popup(self) -> AccessibilityPopup {
        match self {
            Self::Dialog => AccessibilityPopup::Dialog,
            Self::Menu => AccessibilityPopup::Menu,
            Self::ListBox => AccessibilityPopup::ListBox,
            Self::Tree => AccessibilityPopup::Tree,
            Self::Grid => AccessibilityPopup::Grid,
        }
    }
}

/// A copyable declaration descriptor for one controlled, unstyled popover.
///
/// The application owns `open`, every visual declaration, and the listener that changes state.
/// QuickGUI owns stable part identities, anchored portal geometry, topmost Escape/outside-press
/// dismissal, click-through prevention, focus movement/restoration, and accessibility relations.
/// Mount either [`Self::positioner_part`] plus [`Self::popup_part`], or the merged
/// [`Self::surface_part`], only while [`Self::is_open`] is true.
///
/// The descriptor retains no allocation, component store, observer, task, timer, animation, or
/// idle scheduler source.
#[derive(Clone, Copy, Debug, PartialEq)]
#[must_use = "a Popover descriptor has no effect until one of its parts is mounted"]
pub struct Popover {
    trigger_id: ElementId,
    surface_id: ElementId,
    open: bool,
    kind: PopoverKind,
    placement: AnchorPlacement,
    anchor_gap: f32,
    viewport_margin: f32,
    initial_focus: Option<FocusHandle>,
    dismiss_on_escape: bool,
    dismiss_on_pointer_outside: bool,
}

impl Popover {
    pub fn new(
        trigger_id: impl Into<ElementId>,
        surface_id: impl Into<ElementId>,
        open: bool,
    ) -> Self {
        Self {
            trigger_id: trigger_id.into(),
            surface_id: surface_id.into(),
            open,
            kind: PopoverKind::Dialog,
            placement: AnchorPlacement::BottomStart,
            anchor_gap: DEFAULT_POPOVER_ANCHOR_GAP,
            viewport_margin: DEFAULT_POPOVER_VIEWPORT_MARGIN,
            initial_focus: None,
            dismiss_on_escape: true,
            dismiss_on_pointer_outside: true,
        }
    }

    pub const fn kind(mut self, kind: PopoverKind) -> Self {
        self.kind = kind;
        self
    }

    pub const fn placement(mut self, placement: AnchorPlacement) -> Self {
        self.placement = placement;
        self
    }

    /// Set the structural distance between the trigger and positioner.
    pub fn anchor_gap(mut self, gap: f32) -> Self {
        self.anchor_gap =
            finite_clamped(gap, 0.0, MAX_POPOVER_ANCHOR_GAP, DEFAULT_POPOVER_ANCHOR_GAP);
        self
    }

    /// Set the structural collision margin inside the current window viewport.
    pub fn viewport_margin(mut self, margin: f32) -> Self {
        self.viewport_margin = finite_clamped(
            margin,
            0.0,
            MAX_POPOVER_VIEWPORT_MARGIN,
            DEFAULT_POPOVER_VIEWPORT_MARGIN,
        );
        self
    }

    /// Prefer a mounted popup descendant instead of the popup root when opening.
    pub fn initial_focus(mut self, focus: impl Into<ElementId>) -> Self {
        self.initial_focus = Some(FocusHandle::new(focus));
        self
    }

    pub const fn dismiss_on_escape(mut self, dismiss: bool) -> Self {
        self.dismiss_on_escape = dismiss;
        self
    }

    pub const fn dismiss_on_pointer_outside(mut self, dismiss: bool) -> Self {
        self.dismiss_on_pointer_outside = dismiss;
        self
    }

    pub const fn is_open(self) -> bool {
        self.open
    }

    pub const fn trigger_id(self) -> ElementId {
        self.trigger_id
    }

    /// Stable popup identity retained under the existing `surface` name.
    pub const fn surface_id(self) -> ElementId {
        self.surface_id
    }

    pub const fn popup_id(self) -> ElementId {
        self.surface_id
    }

    pub fn positioner_id(self) -> ElementId {
        derived_popover_id(self.surface_id, self.trigger_id, POPOVER_POSITIONER_ID_TAG)
    }

    pub fn backdrop_id(self) -> ElementId {
        derived_popover_id(self.surface_id, self.trigger_id, POPOVER_BACKDROP_ID_TAG)
    }

    pub fn title_id(self) -> ElementId {
        derived_popover_id(self.surface_id, self.trigger_id, POPOVER_TITLE_ID_TAG)
    }

    pub fn description_id(self) -> ElementId {
        derived_popover_id(self.surface_id, self.trigger_id, POPOVER_DESCRIPTION_ID_TAG)
    }

    pub fn close_id(self) -> ElementId {
        derived_popover_id(self.surface_id, self.trigger_id, POPOVER_CLOSE_ID_TAG)
    }

    pub fn trigger_focus(self) -> FocusHandle {
        FocusHandle::new(self.trigger_id)
    }

    pub fn surface_focus(self) -> FocusHandle {
        FocusHandle::new(self.surface_id)
    }

    pub fn popup_focus(self) -> FocusHandle {
        self.surface_focus()
    }

    /// Move focus to the declared initial target, or the popup root, in the same controlled update.
    pub fn focus_surface(self, cx: &mut EventContext) {
        cx.focus(self.initial_focus.unwrap_or_else(|| self.popup_focus()));
    }

    /// Return focus to the paired trigger after an explicit action closes the popup.
    ///
    /// Escape and outside-press dismissal already restore this handle automatically.
    pub fn focus_trigger(self, cx: &mut EventContext) {
        cx.focus(self.trigger_focus());
    }

    /// Decorate an application-owned trigger without adding appearance.
    pub fn trigger_part(self, trigger: Element) -> Element {
        let trigger = trigger
            .id(self.trigger_id)
            .focusable()
            .accessibility_role(AccessibilityRole::Button)
            .user_select_none()
            .app_region_no_drag()
            .cursor_default()
            .accessibility_expanded(self.open)
            .accessibility_has_popup(self.kind.accessibility_popup());
        if self.open {
            trigger.accessibility_controls(self.surface_id)
        } else {
            trigger
        }
    }

    /// Create an unstyled semantic trigger root.
    pub fn trigger(self) -> Element {
        self.trigger_part(button())
    }

    /// Decorate the caller-owned portal/positioner without adding popup appearance.
    ///
    /// QuickGUI's retained overlay node is itself the portal, so this part combines the Base
    /// UI-style Portal and Positioner boundary without introducing a full-window wrapper that
    /// would block unrelated pointer input.
    pub fn positioner_part(self, positioner: Element) -> Element {
        positioner
            .id(self.positioner_id())
            .anchor_to(self.trigger_id, self.placement)
            .anchor_gap(self.anchor_gap)
            .viewport_margin(self.viewport_margin)
            .app_region_no_drag()
            .cursor_default()
    }

    /// Decorate an application-owned popup without adding layout or appearance.
    ///
    /// The popup emits [`crate::Event::Dismiss`] under [`Self::surface_id`] for every enabled
    /// dismissal path. Mounted title/description parts are related without copying their text.
    pub fn popup_part(self, popup: Element) -> Element {
        let mut popup = popup
            .id(self.surface_id)
            .restore_focus_to(self.trigger_focus())
            .track_focus(self.popup_focus())
            .accessibility_role(self.kind.accessibility_role())
            .accessibility_labelled_by(self.title_id())
            .accessibility_described_by(self.description_id())
            .block_pointer()
            .app_region_no_drag()
            .cursor_default();
        if self.dismiss_on_escape {
            popup = popup.dismiss_on_escape();
        }
        if self.dismiss_on_pointer_outside {
            popup = popup.dismiss_on_pointer_outside();
        }
        popup
    }

    /// Decorate one caller-owned element as both positioner and popup.
    ///
    /// This compact form has the same unstyled contract as composing [`Self::positioner_part`]
    /// around [`Self::popup_part`]. Use separate parts when the application needs to animate or
    /// size the positioner independently from popup presentation.
    pub fn surface_part(self, surface: Element) -> Element {
        self.popup_part(surface)
            .anchor_to(self.trigger_id, self.placement)
            .anchor_gap(self.anchor_gap)
            .viewport_margin(self.viewport_margin)
    }

    /// Create an unstyled merged positioner/popup root.
    pub fn surface(self) -> Element {
        self.surface_part(div())
    }

    /// Decorate an optional caller-painted viewport backdrop.
    pub fn backdrop_part(self, backdrop: Element) -> Element {
        backdrop
            .id(self.backdrop_id())
            .overlay()
            .inset_0()
            .size_full()
            .app_region_no_drag()
            .cursor_default()
            .accessibility_hidden(true)
    }

    /// Assign the stable mounted label target used by the popup.
    pub fn title_part(self, title: Element) -> Element {
        title.id(self.title_id())
    }

    /// Assign the stable mounted description target used by the popup.
    pub fn description_part(self, description: Element) -> Element {
        description.id(self.description_id())
    }

    /// Decorate a caller-owned close control with button behavior and no visual defaults.
    pub fn close_part(self, label: impl Into<Arc<str>>, close: Element) -> Element {
        close
            .id(self.close_id())
            .clickable()
            .cursor_default()
            .accessibility_role(AccessibilityRole::Button)
            .accessibility_label(label)
            .app_region_no_drag()
            .user_select_none()
    }
}

fn derived_popover_id(parent: ElementId, avoid: ElementId, tag: u64) -> ElementId {
    let mut hash = parent.as_u64() ^ tag;
    hash ^= hash >> 30;
    hash = hash.wrapping_mul(0xbf58_476d_1ce4_e5b9);
    hash ^= hash >> 27;
    hash = hash.wrapping_mul(0x94d0_49bb_1331_11eb);
    hash ^= hash >> 31;
    for _ in 0..5 {
        if hash != 0 && hash != parent.as_u64() && hash != avoid.as_u64() && hash != u64::MAX {
            return ElementId::new(hash);
        }
        hash = hash.wrapping_add(tag | 1);
    }
    unreachable!("five distinct candidates cannot all match four reserved popover IDs")
}

/// An unstyled parent-owned popup window that may extend beyond its parent window.
///
/// [`Popover`] uses the parent window's existing overlay plane and is therefore physically
/// bounded by that WGPU surface. `AnchoredPopover` instead opens a borderless native child window
/// with its own WGPU surface. On macOS this is an `NSPanel` ordered above its parent; placement is
/// resolved against the display work area, not the parent content rectangle.
///
/// Popup size is explicit, matching the platform popup contract. The trigger rectangle itself is
/// resolved from retained element geometry by [`EventContext::open_anchored_popup`], so callers do
/// not duplicate coordinates or install a layout observer.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AnchoredPopover {
    size: Size,
    placement: AnchorPlacement,
    gap: f32,
    offset: Point,
    constraints: PopupConstraintAdjustment,
    grab: bool,
    accepts_key_focus: bool,
}

impl AnchoredPopover {
    pub fn new(width: f32, height: f32) -> Self {
        Self {
            size: Size::new(
                finite_clamped(width, 1.0, MAX_WINDOW_LOGICAL_DIMENSION, 240.0),
                finite_clamped(height, 1.0, MAX_WINDOW_LOGICAL_DIMENSION, 180.0),
            ),
            placement: AnchorPlacement::BottomStart,
            gap: 0.0,
            offset: Point::ZERO,
            constraints: PopupConstraintAdjustment::FIT,
            grab: true,
            accepts_key_focus: true,
        }
    }

    pub const fn size(self) -> Size {
        self.size
    }

    pub const fn placement(mut self, placement: AnchorPlacement) -> Self {
        self.placement = placement;
        self
    }

    pub fn gap(mut self, gap: f32) -> Self {
        self.gap = finite_clamped(gap, 0.0, 512.0, 0.0);
        self
    }

    pub fn offset(mut self, x: f32, y: f32) -> Self {
        self.offset = Point::new(
            finite_clamped(
                x,
                -MAX_WINDOW_LOGICAL_COORDINATE,
                MAX_WINDOW_LOGICAL_COORDINATE,
                0.0,
            ),
            finite_clamped(
                y,
                -MAX_WINDOW_LOGICAL_COORDINATE,
                MAX_WINDOW_LOGICAL_COORDINATE,
                0.0,
            ),
        );
        self
    }

    pub const fn constraint_adjustment(mut self, constraints: PopupConstraintAdjustment) -> Self {
        self.constraints = constraints;
        self
    }

    /// Select menu-style focus and outside/Escape dismissal.
    ///
    /// Passive tooltips and preview surfaces use `grab(false)` and own no event monitor.
    pub const fn grab(mut self, grab: bool) -> Self {
        self.grab = grab;
        if grab {
            self.accepts_key_focus = true;
        }
        self
    }

    /// Allow or forbid native key-window ownership after pointer interaction.
    ///
    /// `false` keeps an interactive child panel permanently non-key and also disables grabbing,
    /// which lets an owner-window text input retain keyboard and IME focus.
    pub const fn accepts_key_focus(mut self, accepts_key_focus: bool) -> Self {
        self.accepts_key_focus = accepts_key_focus;
        if !accepts_key_focus {
            self.grab = false;
        }
        self
    }

    pub fn popup_options(self) -> PopupOptions {
        let (anchor, gravity) = native_popup_placement(self.placement);
        let mut offset = self.offset;
        match self.placement {
            AnchorPlacement::TopStart | AnchorPlacement::Top | AnchorPlacement::TopEnd => {
                offset.y -= self.gap;
            }
            AnchorPlacement::BottomStart | AnchorPlacement::Bottom | AnchorPlacement::BottomEnd => {
                offset.y += self.gap;
            }
            AnchorPlacement::LeftStart | AnchorPlacement::Left | AnchorPlacement::LeftEnd => {
                offset.x -= self.gap;
            }
            AnchorPlacement::RightStart | AnchorPlacement::Right | AnchorPlacement::RightEnd => {
                offset.x += self.gap;
            }
        }
        PopupOptions::new(Rect::ZERO)
            .anchor(anchor)
            .gravity(gravity)
            .constraint_adjustment(self.constraints)
            .offset(offset.x, offset.y)
            .grab(self.grab)
            .accepts_key_focus(self.accepts_key_focus)
    }

    /// Build transparent, borderless window options without choosing application presentation.
    pub fn window_options(self, title: impl Into<String>) -> WindowOptions {
        WindowOptions::new(title)
            .size(self.size.width, self.size.height)
            .background(Color::TRANSPARENT)
            .window_background(WindowBackgroundAppearance::Transparent)
            .anchored_popup(self.popup_options())
    }

    /// Open the overflow-capable child using the latest retained bounds of `anchor`.
    pub fn open<V: View>(
        self,
        cx: &mut EventContext,
        anchor: impl Into<ElementId>,
        title: impl Into<String>,
        view: V,
    ) -> Result<WindowHandle, WindowCommandError> {
        cx.open_anchored_popup(anchor, view, self.window_options(title))
    }
}

impl Default for AnchoredPopover {
    fn default() -> Self {
        Self::new(240.0, 180.0)
    }
}

fn native_popup_placement(placement: AnchorPlacement) -> (PopupAnchor, PopupGravity) {
    match placement {
        AnchorPlacement::TopStart => (PopupAnchor::TopLeft, PopupGravity::TopRight),
        AnchorPlacement::Top => (PopupAnchor::Top, PopupGravity::Top),
        AnchorPlacement::TopEnd => (PopupAnchor::TopRight, PopupGravity::TopLeft),
        AnchorPlacement::BottomStart => (PopupAnchor::BottomLeft, PopupGravity::BottomRight),
        AnchorPlacement::Bottom => (PopupAnchor::Bottom, PopupGravity::Bottom),
        AnchorPlacement::BottomEnd => (PopupAnchor::BottomRight, PopupGravity::BottomLeft),
        AnchorPlacement::LeftStart => (PopupAnchor::TopLeft, PopupGravity::BottomLeft),
        AnchorPlacement::Left => (PopupAnchor::Left, PopupGravity::Left),
        AnchorPlacement::LeftEnd => (PopupAnchor::BottomLeft, PopupGravity::TopLeft),
        AnchorPlacement::RightStart => (PopupAnchor::TopRight, PopupGravity::BottomRight),
        AnchorPlacement::Right => (PopupAnchor::Right, PopupGravity::Right),
        AnchorPlacement::RightEnd => (PopupAnchor::BottomRight, PopupGravity::TopRight),
    }
}

fn finite_clamped(value: f32, minimum: f32, maximum: f32, fallback: f32) -> f32 {
    if value.is_finite() {
        value.clamp(minimum, maximum)
    } else {
        fallback.clamp(minimum, maximum)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::element::AnchorTarget;
    use crate::{
        App, AppConfig, AppRegion, Event, IntoElement, TestAppContext, View, ViewContext,
        WindowBounds, WindowKind,
    };

    #[test]
    fn descriptor_builds_paired_controlled_elements_without_retained_component_state() {
        assert!(std::mem::size_of::<Popover>() <= 64);
        let closed = Popover::new("trigger", "surface", false).kind(PopoverKind::Menu);
        let closed_trigger = closed.trigger();
        assert_eq!(closed_trigger.explicit_id, Some("trigger".into()));
        assert_eq!(closed_trigger.accessibility.expanded, Some(false));
        assert_eq!(closed_trigger.accessibility.relations.controls(), None);
        assert_eq!(
            closed_trigger.accessibility.has_popup,
            Some(AccessibilityPopup::Menu)
        );
        assert_eq!(closed_trigger.app_region, Some(AppRegion::NoDrag));
        assert_eq!(closed_trigger.cursor_style, Some(crate::CursorStyle::Arrow));

        let open = Popover::new("trigger", "surface", true)
            .kind(PopoverKind::Menu)
            .placement(AnchorPlacement::TopEnd);
        let trigger = open.trigger();
        assert_eq!(trigger.accessibility.expanded, Some(true));
        assert_eq!(
            trigger.accessibility.relations.controls(),
            Some("surface".into())
        );

        let surface = open.surface();
        assert_eq!(surface.explicit_id, Some("surface".into()));
        assert_eq!(surface.accessibility.role, AccessibilityRole::Menu);
        assert_eq!(surface.dismiss_policy, crate::element::DismissPolicy::BOTH);
        assert!(surface.blocks_pointer);
        assert!(surface.focusable);
        assert_eq!(surface.restore_focus, Some(open.trigger_focus()));
        assert_eq!(surface.app_region, Some(AppRegion::NoDrag));
        assert!(surface.portal);
        assert_eq!(surface.visual.background, None);
        assert_eq!(surface.visual.border_color, None);
        assert_eq!(surface.visual.shadows, None);
        let anchor = surface.anchor.expect("popover anchor");
        assert_eq!(anchor.target, AnchorTarget::Element("trigger".into()));
        assert_eq!(anchor.placement, AnchorPlacement::TopEnd);
        assert_eq!(anchor.gap, DEFAULT_POPOVER_ANCHOR_GAP);
        assert_eq!(anchor.viewport_margin, DEFAULT_POPOVER_VIEWPORT_MARGIN);
    }

    #[test]
    fn split_parts_add_exact_behavior_without_appearance_tokens() {
        let popover = Popover::new("trigger", "surface", true)
            .kind(PopoverKind::ListBox)
            .placement(AnchorPlacement::BottomEnd)
            .anchor_gap(12.0)
            .viewport_margin(20.0)
            .initial_focus("first-option")
            .dismiss_on_escape(false);

        let trigger = popover.trigger_part(crate::div());
        assert_eq!(trigger.explicit_id, Some("trigger".into()));
        assert_eq!(trigger.accessibility.role, AccessibilityRole::Button);
        assert!(trigger.focusable);
        assert_eq!(trigger.visual.background, None);
        assert_eq!(trigger.visual.border_color, None);
        assert_eq!(trigger.cursor_style, Some(crate::CursorStyle::Arrow));

        let positioner = popover.positioner_part(crate::div());
        assert_eq!(positioner.explicit_id, Some(popover.positioner_id()));
        assert!(positioner.portal);
        assert!(positioner.blocks_pointer);
        assert_eq!(positioner.visual.background, None);
        assert_eq!(positioner.visual.border_color, None);
        let anchor = positioner.anchor.expect("popover positioner anchor");
        assert_eq!(anchor.target, AnchorTarget::Element("trigger".into()));
        assert_eq!(anchor.placement, AnchorPlacement::BottomEnd);
        assert_eq!(anchor.gap, 12.0);
        assert_eq!(anchor.viewport_margin, 20.0);

        let popup = popover.popup_part(crate::div());
        assert_eq!(popup.explicit_id, Some("surface".into()));
        assert_eq!(popup.accessibility.role, AccessibilityRole::ListBox);
        assert!(!popup.portal);
        assert!(!popup.dismiss_policy.on_escape());
        assert!(popup.dismiss_policy.on_pointer_outside());
        assert!(popup.blocks_pointer);
        assert_eq!(popup.restore_focus, Some(popover.trigger_focus()));
        assert_eq!(
            popup.accessibility.relations.labelled_by(),
            Some(popover.title_id())
        );
        assert_eq!(
            popup.accessibility.relations.described_by(),
            Some(popover.description_id())
        );
        assert_eq!(popup.visual.background, None);
        assert_eq!(popup.visual.border_color, None);
        assert_eq!(popup.visual.shadows, None);
        assert_eq!(popup.visual.radius, 0.0);

        let title = popover.title_part(crate::text("Visible title"));
        let description = popover.description_part(crate::text("Visible description"));
        let close = popover.close_part("Close popover", crate::div());
        let backdrop = popover.backdrop_part(crate::div());
        assert_eq!(title.explicit_id, Some(popover.title_id()));
        assert_eq!(description.explicit_id, Some(popover.description_id()));
        assert_eq!(close.explicit_id, Some(popover.close_id()));
        assert!(close.clickable);
        assert_eq!(close.accessibility.role, AccessibilityRole::Button);
        assert_eq!(close.accessibility.label.as_deref(), Some("Close popover"));
        assert_eq!(close.visual.background, None);
        assert_eq!(backdrop.explicit_id, Some(popover.backdrop_id()));
        assert!(backdrop.portal);
        assert!(backdrop.blocks_pointer);
        assert!(backdrop.accessibility.hidden);
    }

    #[test]
    fn structural_geometry_is_bounded_and_dismissal_paths_are_independent() {
        let popover = Popover::new("trigger", "surface", true)
            .anchor_gap(-4.0)
            .viewport_margin(f32::NAN)
            .dismiss_on_escape(false)
            .dismiss_on_pointer_outside(false);
        let positioner = popover.positioner_part(crate::div());
        let anchor = positioner.anchor.expect("popover positioner anchor");
        assert_eq!(anchor.gap, 0.0);
        assert_eq!(anchor.viewport_margin, DEFAULT_POPOVER_VIEWPORT_MARGIN);

        let popup = popover.popup_part(crate::div());
        assert!(popup.dismiss_policy.is_empty());
        assert!(popup.blocks_pointer);
    }

    #[test]
    fn derived_part_ids_are_stable_distinct_and_avoid_the_declared_pair() {
        let popover = Popover::new(41_u64, 42_u64, true);
        let ids = [
            popover.positioner_id(),
            popover.backdrop_id(),
            popover.title_id(),
            popover.description_id(),
            popover.close_id(),
        ];
        for (index, id) in ids.iter().enumerate() {
            assert_ne!(*id, popover.trigger_id());
            assert_ne!(*id, popover.surface_id());
            assert_ne!(*id, ElementId::new(0));
            assert_ne!(*id, ElementId::new(u64::MAX));
            assert!(!ids[..index].contains(id));
        }
        assert_eq!(
            popover.positioner_id(),
            Popover::new(41_u64, 42_u64, false).positioner_id()
        );
    }

    #[test]
    fn anchored_descriptor_maps_placement_and_builds_a_transparent_popup_host() {
        let descriptor = AnchoredPopover::new(320.0, 200.0)
            .placement(AnchorPlacement::TopEnd)
            .gap(8.0)
            .offset(3.0, 4.0)
            .grab(false);
        assert_eq!(descriptor.size(), Size::new(320.0, 200.0));

        let popup = descriptor.popup_options();
        assert_eq!(popup.anchor_rect, Rect::ZERO);
        assert_eq!(popup.anchor, PopupAnchor::TopRight);
        assert_eq!(popup.gravity, PopupGravity::TopLeft);
        assert_eq!(popup.offset, Point::new(3.0, -4.0));
        assert_eq!(popup.constraint_adjustment, PopupConstraintAdjustment::FIT);
        assert!(!popup.grab);
        assert!(popup.accepts_key_focus);

        let options = descriptor.window_options("Unstyled popup");
        assert_eq!(options.kind, WindowKind::AnchoredPopup);
        assert_eq!(options.size, Size::new(320.0, 200.0));
        assert_eq!(options.background, Color::TRANSPARENT);
        assert_eq!(
            options.window_background,
            WindowBackgroundAppearance::Transparent
        );
        assert!(!options.focus);
        assert_eq!(options.popup, Some(popup));

        let never_key = descriptor.accepts_key_focus(false).popup_options();
        assert!(!never_key.grab);
        assert!(!never_key.accepts_key_focus);
    }

    #[derive(Default)]
    struct AnchoredLauncher {
        popup: Option<WindowHandle>,
    }

    struct AnchoredSurface;

    impl View for AnchoredSurface {
        fn render(&mut self, _cx: &mut ViewContext<'_, Self>) -> impl IntoElement {
            div().size_full()
        }
    }

    impl View for AnchoredLauncher {
        fn render(&mut self, cx: &mut ViewContext<'_, Self>) -> impl IntoElement {
            let open = cx.listener("anchor", |view, cx| {
                view.popup = Some(
                    AnchoredPopover::new(180.0, 120.0)
                        .gap(6.0)
                        .open(cx, "anchor", "Anchored surface", AnchoredSurface)
                        .expect("a mounted window can open an anchored popup"),
                );
            });
            div().size_full().relative().child(
                button()
                    .id("anchor")
                    .absolute()
                    .left(280.0)
                    .top(180.0)
                    .w(40.0)
                    .h(30.0)
                    .on_click(open),
            )
        }
    }

    #[test]
    fn anchored_popup_uses_retained_trigger_bounds_and_may_cross_the_parent_edge() {
        let parent_bounds = Rect::new(100.0, 100.0, 320.0, 240.0);
        let (mut cx, launcher) = App::new(AnchoredLauncher::default())
            .config(
                AppConfig::new("Anchor parent")
                    .window_bounds(WindowBounds::Windowed(parent_bounds))
                    .without_minimum_size(),
            )
            .into_test_context()
            .unwrap();
        let parent = launcher.window_handle();

        cx.click(parent, "anchor").unwrap();
        let popup = cx
            .read(launcher, |view| view.popup)
            .unwrap()
            .expect("listener retained the popup handle");
        let popup_state = cx.window_state(popup).unwrap();
        let popup_bounds = popup_state.bounds.bounds();

        assert_eq!(popup_state.kind, WindowKind::AnchoredPopup);
        assert_eq!(popup_bounds, Rect::new(380.0, 316.0, 180.0, 120.0));
        assert!(popup_bounds.right() > parent_bounds.right());
        assert!(popup_bounds.bottom() > parent_bounds.bottom());
        assert_eq!(
            popup_bounds.intersection(cx.primary_display().unwrap().visible_bounds()),
            Some(popup_bounds)
        );

        let renders = cx.render_count(popup).unwrap();
        cx.run_until_idle().unwrap();
        assert_eq!(cx.render_count(popup).unwrap(), renders);
    }

    #[derive(Default)]
    struct PopoverView {
        open: bool,
        chosen: bool,
    }

    impl View for PopoverView {
        fn event(&mut self, event: &Event, cx: &mut EventContext) {
            if *event == Event::Dismiss("surface".into()) {
                self.open = false;
                cx.invalidate();
            }
        }

        fn render(&mut self, cx: &mut ViewContext<'_, Self>) -> impl IntoElement {
            let popover = Popover::new("trigger", "surface", self.open).initial_focus("choose");
            let toggle = cx.listener(popover.trigger_id(), move |view, cx| {
                view.open = !view.open;
                if view.open {
                    popover.focus_surface(cx);
                } else {
                    popover.focus_trigger(cx);
                }
                cx.invalidate();
            });
            let choose = cx.listener("choose", move |view, cx| {
                view.chosen = true;
                view.open = false;
                popover.focus_trigger(cx);
                cx.invalidate();
            });

            let mut root = crate::div().children([
                popover.trigger().on_click(toggle).child("Open"),
                crate::button().id("after").child("After"),
            ]);
            if popover.is_open() {
                root = root.child(
                    popover.positioner_part(
                        crate::div().child(
                            popover
                                .popup_part(crate::div())
                                .accessibility_label("Actions")
                                .child(
                                    crate::button()
                                        .id("choose")
                                        .on_click(choose)
                                        .child("Choose"),
                                ),
                        ),
                    ),
                );
            }
            root
        }
    }

    #[test]
    fn controlled_open_focus_dismiss_and_idle_paths_use_the_existing_runtime() {
        let (mut cx, view) = TestAppContext::new(PopoverView::default()).unwrap();
        let window = view.window_handle();
        assert!(!cx.contains_element(window, "surface").unwrap());

        cx.click(window, "trigger").unwrap();
        assert!(cx.read(view, |view| view.open).unwrap());
        assert!(cx.contains_element(window, "surface").unwrap());
        assert_eq!(cx.focused(window).unwrap(), Some("choose".into()));
        let trigger_bounds = cx.element_bounds(window, "trigger").unwrap();
        let positioner_bounds = cx
            .element_bounds(
                window,
                Popover::new("trigger", "surface", true).positioner_id(),
            )
            .unwrap();
        let popup_bounds = cx.element_bounds(window, "surface").unwrap();
        assert_eq!(positioner_bounds, popup_bounds);
        assert_eq!(positioner_bounds.x, DEFAULT_POPOVER_VIEWPORT_MARGIN);
        assert!(positioner_bounds.x > trigger_bounds.x);
        assert_eq!(
            positioner_bounds.y,
            trigger_bounds.bottom() + DEFAULT_POPOVER_ANCHOR_GAP
        );

        cx.simulate_keystrokes(window, "escape").unwrap();
        assert!(!cx.read(view, |view| view.open).unwrap());
        assert!(!cx.contains_element(window, "surface").unwrap());
        assert_eq!(cx.focused(window).unwrap(), Some("trigger".into()));

        cx.click(window, "trigger").unwrap();
        cx.click(window, "choose").unwrap();
        assert!(cx.read(view, |view| view.chosen).unwrap());
        assert!(!cx.read(view, |view| view.open).unwrap());
        assert_eq!(cx.focused(window).unwrap(), Some("trigger".into()));

        let renders = cx.render_count(window).unwrap();
        cx.run_until_idle().unwrap();
        assert_eq!(cx.render_count(window).unwrap(), renders);
    }
}
