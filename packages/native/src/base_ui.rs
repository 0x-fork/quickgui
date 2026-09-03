//! Base UI parity components bound to the Rust core.
//!
//! Separators, avatars, checkbox groups, preview cards, scroll areas, OTP fields, drawers, and
//! navigation menus follow the same contract as every other declared component: JavaScript
//! declares bounds, values, deadlines, and geometry ahead of time; the Rust core owns identity,
//! semantics, keyboard behavior, focus, dismissal, mount policy, and every exact deadline; and
//! anything the core decides travels back as one asynchronous `componentchange` event.
//!
//! The retained instances live on the hosted [`NativeView`] behind a per-`scope`
//! [`quickgui::StateAccessor`], so one hosted view addresses many declared instances without a
//! JavaScript registry and without ever asking JavaScript a synchronous question.

use super::*;

use std::time::{Duration, Instant};

use quickgui::{
    Avatar, AvatarLoadingStatus, AvatarState, CheckboxGroup, CheckboxGroupState, Drawer,
    DrawerModality, DrawerState, MAX_AVATAR_FALLBACK_DELAY, MAX_CHECKBOX_GROUP_VALUES,
    MAX_DRAWER_SNAP_POINTS, MAX_NAVIGATION_MENU_ITEMS, MAX_OTP_LENGTH, MAX_PREVIEW_CARD_DELAY,
    NavigationMenu, NavigationMenuActivationDirection, NavigationMenuItem,
    NavigationMenuOrientation, NavigationMenuState, OtpField, OtpFieldState, OtpValidationType,
    PreviewCard, PreviewCardState, ScrollArea, ScrollAreaOrientation, ScrollAreaState, Separator,
    SeparatorOrientation, Size, SwipeDirection,
};

/// Which slot of a per-axis pair one scrollbar orientation addresses.
fn axis_index(orientation: ScrollAreaOrientation) -> usize {
    usize::from(orientation.is_horizontal())
}

/// Declared part name of a standalone separator.
pub(super) const SEPARATOR_PART: &str = "separator";
/// Declared part name of an avatar root.
pub(super) const AVATAR_PART: &str = "avatar";
/// Declared part name of an avatar image.
pub(super) const AVATAR_IMAGE_PART: &str = "avatar-image";
/// Declared part name of an avatar fallback.
pub(super) const AVATAR_FALLBACK_PART: &str = "avatar-fallback";
/// Declared part name of a checkbox-group root.
pub(super) const CHECKBOX_GROUP_PART: &str = "checkbox-group";
/// Declared part name of one checkbox inside a checkbox group.
pub(super) const CHECKBOX_GROUP_ITEM_PART: &str = "checkbox-group-item";
/// Declared part name of one checkbox indicator inside a checkbox group.
pub(super) const CHECKBOX_GROUP_INDICATOR_PART: &str = "checkbox-group-indicator";
/// Declared part name of a checkbox group's parent checkbox.
pub(super) const CHECKBOX_GROUP_PARENT_PART: &str = "checkbox-group-parent";
/// Declared part name of a preview-card root.
pub(super) const PREVIEW_CARD_PART: &str = "preview-card";
/// Declared part name of a preview-card trigger.
pub(super) const PREVIEW_CARD_TRIGGER_PART: &str = "preview-card-trigger";
/// Declared part name of a preview-card portal boundary.
pub(super) const PREVIEW_CARD_PORTAL_PART: &str = "preview-card-portal";
/// Declared part name of a preview-card positioner.
pub(super) const PREVIEW_CARD_POSITIONER_PART: &str = "preview-card-positioner";
/// Declared part name of a preview-card popup.
pub(super) const PREVIEW_CARD_POPUP_PART: &str = "preview-card-popup";
/// Declared part name of a preview-card arrow.
pub(super) const PREVIEW_CARD_ARROW_PART: &str = "preview-card-arrow";
/// Declared part name of a preview-card backdrop.
pub(super) const PREVIEW_CARD_BACKDROP_PART: &str = "preview-card-backdrop";
/// Declared part name of a scroll-area root.
pub(super) const SCROLL_AREA_PART: &str = "scroll-area";
/// Declared part name of a scroll-area viewport.
pub(super) const SCROLL_AREA_VIEWPORT_PART: &str = "scroll-area-viewport";
/// Declared part name of a scroll-area content box.
pub(super) const SCROLL_AREA_CONTENT_PART: &str = "scroll-area-content";
/// Declared part name of one scroll-area scrollbar track.
pub(super) const SCROLL_AREA_SCROLLBAR_PART: &str = "scroll-area-scrollbar";
/// Declared part name of one scroll-area scrollbar thumb.
pub(super) const SCROLL_AREA_THUMB_PART: &str = "scroll-area-thumb";
/// Declared part name of a scroll-area corner.
pub(super) const SCROLL_AREA_CORNER_PART: &str = "scroll-area-corner";
/// Declared part name of an OTP-field root.
pub(super) const OTP_FIELD_PART: &str = "otp-field";
/// Declared part name of one OTP-field slot input.
pub(super) const OTP_FIELD_INPUT_PART: &str = "otp-field-input";
/// Declared part name of one OTP-field separator.
pub(super) const OTP_FIELD_SEPARATOR_PART: &str = "otp-field-separator";
/// Declared part name of a drawer root.
pub(super) const DRAWER_PART: &str = "drawer";
/// Declared part name of a drawer trigger.
pub(super) const DRAWER_TRIGGER_PART: &str = "drawer-trigger";
/// Declared part name of a drawer portal boundary.
pub(super) const DRAWER_PORTAL_PART: &str = "drawer-portal";
/// Declared part name of a drawer backdrop.
pub(super) const DRAWER_BACKDROP_PART: &str = "drawer-backdrop";
/// Declared part name of a drawer viewport.
pub(super) const DRAWER_VIEWPORT_PART: &str = "drawer-viewport";
/// Declared part name of a drawer sheet.
pub(super) const DRAWER_POPUP_PART: &str = "drawer-popup";
/// Declared part name of a drawer's scrollable body.
pub(super) const DRAWER_CONTENT_PART: &str = "drawer-content";
/// Declared part name of a drawer title.
pub(super) const DRAWER_TITLE_PART: &str = "drawer-title";
/// Declared part name of a drawer description.
pub(super) const DRAWER_DESCRIPTION_PART: &str = "drawer-description";
/// Declared part name of a drawer close control.
pub(super) const DRAWER_CLOSE_PART: &str = "drawer-close";
/// Declared part name of a drawer swipe area.
pub(super) const DRAWER_SWIPE_AREA_PART: &str = "drawer-swipe-area";
/// Declared part name of a navigation-menu root.
pub(super) const NAVIGATION_MENU_PART: &str = "navigation-menu";
/// Declared part name of a navigation-menu list.
pub(super) const NAVIGATION_MENU_LIST_PART: &str = "navigation-menu-list";
/// Declared part name of one navigation-menu item.
pub(super) const NAVIGATION_MENU_ITEM_PART: &str = "navigation-menu-item";
/// Declared part name of one navigation-menu trigger.
pub(super) const NAVIGATION_MENU_TRIGGER_PART: &str = "navigation-menu-trigger";
/// Declared part name of one navigation-menu trigger icon.
pub(super) const NAVIGATION_MENU_ICON_PART: &str = "navigation-menu-icon";
/// Declared part name of one navigation-menu portal boundary.
pub(super) const NAVIGATION_MENU_PORTAL_PART: &str = "navigation-menu-portal";
/// Declared part name of one navigation-menu positioner.
pub(super) const NAVIGATION_MENU_POSITIONER_PART: &str = "navigation-menu-positioner";
/// Declared part name of one navigation-menu popup.
pub(super) const NAVIGATION_MENU_POPUP_PART: &str = "navigation-menu-popup";
/// Declared part name of one navigation-menu panel viewport.
pub(super) const NAVIGATION_MENU_VIEWPORT_PART: &str = "navigation-menu-viewport";
/// Declared part name of one navigation-menu panel content box.
pub(super) const NAVIGATION_MENU_CONTENT_PART: &str = "navigation-menu-content";
/// Declared part name of one navigation-menu arrow.
pub(super) const NAVIGATION_MENU_ARROW_PART: &str = "navigation-menu-arrow";
/// Declared part name of one navigation-menu backdrop.
pub(super) const NAVIGATION_MENU_BACKDROP_PART: &str = "navigation-menu-backdrop";
/// Declared part name of one navigation link.
pub(super) const NAVIGATION_MENU_LINK_PART: &str = "navigation-menu-link";

/// Whether one declared part belongs to this module.
///
/// A part outside this set falls through to the other binding modules untouched.
pub(super) fn owns_base_ui_part(part: &str) -> bool {
    matches!(
        part,
        SEPARATOR_PART
            | AVATAR_PART
            | AVATAR_IMAGE_PART
            | AVATAR_FALLBACK_PART
            | CHECKBOX_GROUP_PART
            | CHECKBOX_GROUP_ITEM_PART
            | CHECKBOX_GROUP_INDICATOR_PART
            | CHECKBOX_GROUP_PARENT_PART
            | PREVIEW_CARD_PART
            | PREVIEW_CARD_TRIGGER_PART
            | PREVIEW_CARD_PORTAL_PART
            | PREVIEW_CARD_POSITIONER_PART
            | PREVIEW_CARD_POPUP_PART
            | PREVIEW_CARD_ARROW_PART
            | PREVIEW_CARD_BACKDROP_PART
            | SCROLL_AREA_PART
            | SCROLL_AREA_VIEWPORT_PART
            | SCROLL_AREA_CONTENT_PART
            | SCROLL_AREA_SCROLLBAR_PART
            | SCROLL_AREA_THUMB_PART
            | SCROLL_AREA_CORNER_PART
            | OTP_FIELD_PART
            | OTP_FIELD_INPUT_PART
            | OTP_FIELD_SEPARATOR_PART
            | DRAWER_PART
            | DRAWER_TRIGGER_PART
            | DRAWER_PORTAL_PART
            | DRAWER_BACKDROP_PART
            | DRAWER_VIEWPORT_PART
            | DRAWER_POPUP_PART
            | DRAWER_CONTENT_PART
            | DRAWER_TITLE_PART
            | DRAWER_DESCRIPTION_PART
            | DRAWER_CLOSE_PART
            | DRAWER_SWIPE_AREA_PART
            | NAVIGATION_MENU_PART
            | NAVIGATION_MENU_LIST_PART
            | NAVIGATION_MENU_ITEM_PART
            | NAVIGATION_MENU_TRIGGER_PART
            | NAVIGATION_MENU_ICON_PART
            | NAVIGATION_MENU_PORTAL_PART
            | NAVIGATION_MENU_POSITIONER_PART
            | NAVIGATION_MENU_POPUP_PART
            | NAVIGATION_MENU_VIEWPORT_PART
            | NAVIGATION_MENU_CONTENT_PART
            | NAVIGATION_MENU_ARROW_PART
            | NAVIGATION_MENU_BACKDROP_PART
            | NAVIGATION_MENU_LINK_PART
    )
}

/// Whether the core owns one part's click activation.
///
/// The core registers exactly one listener per identity, so a declared `onClick` on a part whose
/// activation belongs to the component would be a second registration for the same element. The
/// binding keeps the core's, which is the one that actually moves the retained value.
pub(super) fn base_ui_owns_click(part: &str) -> bool {
    matches!(
        part,
        CHECKBOX_GROUP_ITEM_PART | CHECKBOX_GROUP_PARENT_PART | NAVIGATION_MENU_TRIGGER_PART
    )
}

/// Whether the core owns one part's text input.
pub(super) fn base_ui_owns_input(part: &str) -> bool {
    part == OTP_FIELD_INPUT_PART
}

/// Whether the core owns one part's captured pointer gesture.
pub(super) fn base_ui_owns_pointer(part: &str) -> bool {
    matches!(
        part,
        SCROLL_AREA_SCROLLBAR_PART | SCROLL_AREA_THUMB_PART | DRAWER_SWIPE_AREA_PART
    )
}

/// Whether the core owns one part's dismissal.
pub(super) fn base_ui_owns_dismiss(part: &str) -> bool {
    matches!(
        part,
        PREVIEW_CARD_POPUP_PART | DRAWER_POPUP_PART | NAVIGATION_MENU_POPUP_PART
    )
}

/// Whether the core owns one part's wheel gesture.
pub(super) fn base_ui_owns_scroll_wheel(part: &str) -> bool {
    part == SCROLL_AREA_VIEWPORT_PART
}

// ---------------------------------------------------------------------------
// Declared property readers
// ---------------------------------------------------------------------------

/// Read one bounded declared millisecond deadline.
///
/// A missing, non-finite, or negative declaration keeps `default`; a longer one is clamped by the
/// core itself, so a mistyped configuration can never park a window's wake-up arbitrarily.
fn declared_delay(node: &NativeNode, key: u16, default: Duration, limit: Duration) -> Duration {
    match node.number(key) {
        Some(value) if value.is_finite() && value >= 0.0 => {
            Duration::from_secs_f32(value / 1000.0).min(limit)
        }
        _ => default,
    }
}

/// Read one declared `[width, height]` extent pair.
fn declared_extent(node: &NativeNode, key: u16) -> Size {
    let values = declared_numbers(node, key);
    let read = |index: usize| {
        values
            .get(index)
            .copied()
            .filter(|value| value.is_finite() && *value >= 0.0)
            .unwrap_or(0.0) as f32
    };
    Size::new(read(0), read(1))
}

fn declared_orientation(node: &NativeNode) -> ScrollAreaOrientation {
    match node.string(property::ORIENTATION) {
        Some("horizontal") => ScrollAreaOrientation::Horizontal,
        _ => ScrollAreaOrientation::Vertical,
    }
}

fn declared_separator_orientation(node: &NativeNode) -> SeparatorOrientation {
    match node.string(property::ORIENTATION) {
        Some("vertical") => SeparatorOrientation::Vertical,
        _ => SeparatorOrientation::Horizontal,
    }
}

fn declared_validation(node: &NativeNode) -> OtpValidationType {
    match node.string(property::VARIANT) {
        Some("alpha") => OtpValidationType::Alpha,
        Some("alphanumeric") => OtpValidationType::Alphanumeric,
        Some("none") => OtpValidationType::None,
        _ => OtpValidationType::Numeric,
    }
}

fn declared_swipe_direction(node: &NativeNode) -> SwipeDirection {
    match node.string(property::SWIPE_DIRECTION) {
        Some("up") => SwipeDirection::Up,
        Some("left") => SwipeDirection::Left,
        Some("right") => SwipeDirection::Right,
        _ => SwipeDirection::Down,
    }
}

fn declared_modality(node: &NativeNode) -> DrawerModality {
    match node.string(property::VARIANT) {
        Some("trap-focus") => DrawerModality::TrapFocus,
        Some("non-modal") => DrawerModality::NonModal,
        _ => DrawerModality::Modal,
    }
}

fn declared_menu_orientation(node: &NativeNode) -> NavigationMenuOrientation {
    match node.string(property::ORIENTATION) {
        Some("vertical") => NavigationMenuOrientation::Vertical,
        _ => NavigationMenuOrientation::Horizontal,
    }
}

fn activation_direction_name(direction: NavigationMenuActivationDirection) -> Option<&'static str> {
    match direction {
        NavigationMenuActivationDirection::None => None,
        NavigationMenuActivationDirection::Left => Some("left"),
        NavigationMenuActivationDirection::Right => Some("right"),
        NavigationMenuActivationDirection::Up => Some("up"),
        NavigationMenuActivationDirection::Down => Some("down"),
    }
}

fn loading_status_name(status: AvatarLoadingStatus) -> &'static str {
    match status {
        AvatarLoadingStatus::Idle => "idle",
        AvatarLoadingStatus::Loading => "loading",
        AvatarLoadingStatus::Loaded => "loaded",
        AvatarLoadingStatus::Error => "error",
    }
}

// ---------------------------------------------------------------------------
// Retained instances
// ---------------------------------------------------------------------------

/// One retained avatar instance.
pub(super) struct NativeAvatarState {
    pub(super) state: AvatarState,
    pub(super) label: Arc<str>,
    delay: Duration,
    source: Arc<str>,
    /// The resolved outcome applied on the frame after the declared source changed.
    pending: Option<AvatarLoadingStatus>,
    owner: u32,
    listens: bool,
    reported: Option<AvatarLoadingStatus>,
}

impl Default for NativeAvatarState {
    fn default() -> Self {
        Self {
            state: AvatarState::new(),
            label: Arc::from(""),
            delay: Duration::ZERO,
            source: Arc::from(""),
            pending: None,
            owner: 0,
            listens: false,
            reported: None,
        }
    }
}

/// One retained checkbox-group instance.
#[derive(Default)]
pub(super) struct NativeCheckboxGroupState {
    pub(super) state: CheckboxGroupState,
    pub(super) names: Vec<String>,
    owner: u32,
    listens: bool,
    declared: Vec<String>,
    reported: Vec<String>,
}

/// One retained preview-card instance.
pub(super) struct NativePreviewCardState {
    pub(super) state: PreviewCardState,
    pub(super) placement: AnchorPlacement,
    pub(super) anchor_gap: Option<f32>,
    pub(super) viewport_margin: Option<f32>,
    owner: u32,
    listens: bool,
    declared_open: bool,
    reported: bool,
}

impl Default for NativePreviewCardState {
    fn default() -> Self {
        Self {
            state: PreviewCardState::new(),
            placement: AnchorPlacement::default(),
            anchor_gap: None,
            viewport_margin: None,
            owner: 0,
            listens: false,
            declared_open: false,
            reported: false,
        }
    }
}

/// One retained scroll-area instance.
pub(super) struct NativeScrollAreaState {
    pub(super) state: ScrollAreaState,
    pub(super) keep_mounted: bool,
    /// The last laid-out track length per axis, learned from a scrollbar's own pointer events.
    ///
    /// A track normally spans the viewport along its axis, so the declared viewport extent is the
    /// exact default until a press reports the real geometry the core laid out.
    pub(super) track_lengths: [f32; 2],
    owner: u32,
    listens: bool,
    reported: Option<serde_json::Value>,
}

impl Default for NativeScrollAreaState {
    fn default() -> Self {
        Self {
            state: ScrollAreaState::new(),
            keep_mounted: false,
            track_lengths: [0.0, 0.0],
            owner: 0,
            listens: false,
            reported: None,
        }
    }
}

impl NativeScrollAreaState {
    pub(super) fn track_length(&self, orientation: ScrollAreaOrientation) -> f32 {
        self.track_lengths[axis_index(orientation)]
    }

    fn snapshot(&self) -> serde_json::Value {
        let style = self.state.style_state();
        let offset = self.state.offset();
        serde_json::json!({
            "offset": { "x": offset.x, "y": offset.y },
            "scrolling": style.scrolling,
            "hovering": style.hovering,
            "hasOverflowX": style.has_overflow_x,
            "hasOverflowY": style.has_overflow_y,
            "overflowXStart": style.overflow_x_start,
            "overflowXEnd": style.overflow_x_end,
            "overflowYStart": style.overflow_y_start,
            "overflowYEnd": style.overflow_y_end,
        })
    }
}

/// One retained OTP-field instance.
pub(super) struct NativeOtpFieldState {
    pub(super) state: OtpFieldState,
    pub(super) auto_submit: Option<ElementId>,
    owner: u32,
    listens: bool,
    declared_length: usize,
    declared_value: String,
    reported: Option<String>,
    reported_complete: bool,
}

impl Default for NativeOtpFieldState {
    fn default() -> Self {
        Self {
            state: OtpFieldState::new(6),
            auto_submit: None,
            owner: 0,
            listens: false,
            declared_length: 6,
            declared_value: String::new(),
            reported: None,
            reported_complete: false,
        }
    }
}

/// One retained drawer instance.
pub(super) struct NativeDrawerState {
    pub(super) state: DrawerState,
    pub(super) modality: DrawerModality,
    owner: u32,
    listens: bool,
    declared: Arc<str>,
    declared_open: bool,
    reported: Option<serde_json::Value>,
}

impl Default for NativeDrawerState {
    fn default() -> Self {
        Self {
            state: DrawerState::new(SwipeDirection::Down),
            modality: DrawerModality::Modal,
            owner: 0,
            listens: false,
            declared: Arc::from(""),
            declared_open: false,
            reported: None,
        }
    }
}

/// One retained navigation-menu instance.
pub(super) struct NativeNavigationMenuState {
    pub(super) state: NavigationMenuState,
    pub(super) items: Vec<NavigationMenuItem>,
    pub(super) names: Vec<String>,
    pub(super) placement: AnchorPlacement,
    pub(super) loop_focus: bool,
    owner: u32,
    listens: bool,
    declared: Option<String>,
    reported: Option<serde_json::Value>,
}

impl Default for NativeNavigationMenuState {
    fn default() -> Self {
        Self {
            state: NavigationMenuState::new(),
            items: Vec::new(),
            names: Vec::new(),
            placement: AnchorPlacement::BottomStart,
            loop_focus: true,
            owner: 0,
            listens: false,
            declared: None,
            reported: None,
        }
    }
}

/// Every Base UI parity instance one hosted window retains.
#[derive(Default)]
pub(super) struct NativeBaseUiStates {
    pub(super) avatars: HashMap<u64, NativeAvatarState>,
    pub(super) checkbox_groups: HashMap<u64, NativeCheckboxGroupState>,
    pub(super) preview_cards: HashMap<u64, NativePreviewCardState>,
    pub(super) scroll_areas: HashMap<u64, NativeScrollAreaState>,
    pub(super) otp_fields: HashMap<u64, NativeOtpFieldState>,
    pub(super) drawers: HashMap<u64, NativeDrawerState>,
    pub(super) navigation_menus: HashMap<u64, NativeNavigationMenuState>,
}

/// Every Base UI component key seen in one declaration pass, so stale instances are dropped.
#[derive(Default)]
struct LiveBaseUiKeys {
    avatars: HashSet<u64>,
    checkbox_groups: HashSet<u64>,
    preview_cards: HashSet<u64>,
    scroll_areas: HashSet<u64>,
    otp_fields: HashSet<u64>,
    drawers: HashSet<u64>,
    navigation_menus: HashSet<u64>,
}

// ---------------------------------------------------------------------------
// Declaration synchronisation
// ---------------------------------------------------------------------------

impl NativeComponentStates {
    /// Reseed every declared Base UI instance and report what the core decided.
    pub(super) fn sync_base_ui(&mut self, tree: &NativeTree, window: u32, events: &EventQueue) {
        // Base UI declares several of these values on a part rather than on the root — an
        // avatar's source on its image, its fallback delay on the fallback, a preview card's
        // delays on its trigger, a scroll area's `keepMounted` on its scrollbar. The retained
        // state is per instance, so those declarations are gathered by scope first.
        let declarations = gather_base_ui_declarations(tree);

        let mut live = LiveBaseUiKeys::default();
        for (id, node) in &tree.nodes {
            let Some(part) = node.string(property::PART) else {
                continue;
            };
            let key = component_key(*id, node);
            match part {
                AVATAR_PART if live.avatars.len() < MAX_COMPONENT_INSTANCES => {
                    live.avatars.insert(key);
                    self.sync_avatar(key, *id, node, &declarations);
                }
                CHECKBOX_GROUP_PART if live.checkbox_groups.len() < MAX_COMPONENT_INSTANCES => {
                    live.checkbox_groups.insert(key);
                    self.sync_checkbox_group(key, *id, node);
                }
                PREVIEW_CARD_PART if live.preview_cards.len() < MAX_COMPONENT_INSTANCES => {
                    live.preview_cards.insert(key);
                    self.sync_preview_card(key, *id, node, &declarations);
                }
                SCROLL_AREA_PART if live.scroll_areas.len() < MAX_COMPONENT_INSTANCES => {
                    live.scroll_areas.insert(key);
                    self.sync_scroll_area(key, *id, node, &declarations);
                }
                OTP_FIELD_PART if live.otp_fields.len() < MAX_COMPONENT_INSTANCES => {
                    live.otp_fields.insert(key);
                    self.sync_otp_field(key, *id, node);
                }
                DRAWER_PART if live.drawers.len() < MAX_COMPONENT_INSTANCES => {
                    live.drawers.insert(key);
                    self.sync_drawer(key, *id, node);
                }
                NAVIGATION_MENU_PART if live.navigation_menus.len() < MAX_COMPONENT_INSTANCES => {
                    live.navigation_menus.insert(key);
                    self.sync_navigation_menu(key, *id, node, tree);
                }
                _ => {}
            }
        }
        self.base_ui
            .avatars
            .retain(|key, _| live.avatars.contains(key));
        self.base_ui
            .checkbox_groups
            .retain(|key, _| live.checkbox_groups.contains(key));
        self.base_ui
            .preview_cards
            .retain(|key, _| live.preview_cards.contains(key));
        self.base_ui
            .scroll_areas
            .retain(|key, _| live.scroll_areas.contains(key));
        self.base_ui
            .otp_fields
            .retain(|key, _| live.otp_fields.contains(key));
        self.base_ui
            .drawers
            .retain(|key, _| live.drawers.contains(key));
        self.base_ui
            .navigation_menus
            .retain(|key, _| live.navigation_menus.contains(key));
        self.report_base_ui(window, events);
    }

    fn sync_avatar(
        &mut self,
        key: u64,
        id: u32,
        node: &NativeNode,
        declarations: &BaseUiDeclarations,
    ) {
        let source = declarations.avatar_sources.get(&key);
        // Base UI declares the hold-back on `Avatar.Fallback`; a delay on the root is accepted
        // too so a composition without a fallback part still reaches the core's deadline.
        let delay = declarations.avatar_delays.get(&key).map_or_else(
            || {
                declared_delay(
                    node,
                    property::DELAY,
                    Duration::ZERO,
                    MAX_AVATAR_FALLBACK_DELAY,
                )
            },
            |delay| Duration::from_secs_f32(*delay / 1000.0).min(MAX_AVATAR_FALLBACK_DELAY),
        );
        let label: Arc<str> = Arc::from(node.string(property::ACCESSIBILITY_LABEL).unwrap_or(""));
        let listens = declares_change(node);
        let now = Instant::now();
        let retained = self.base_ui.avatars.entry(key).or_default();
        retained.owner = id;
        retained.listens = listens;
        retained.label = label;
        if retained.delay != delay {
            retained.delay = delay;
            retained.state = retained.state.delay(delay);
        }
        let source = source.cloned().unwrap_or_else(|| Arc::from(""));
        if retained.source.as_ref() != source.as_ref() {
            retained.source = Arc::clone(&source);
            let resolved = if source.is_empty() {
                AvatarLoadingStatus::Idle
            } else if native_image_source(source.as_ref()).is_ok() {
                AvatarLoadingStatus::Loaded
            } else {
                AvatarLoadingStatus::Error
            };
            // With a declared fallback delay the status passes through `Loading` for exactly one
            // frame, which is what keeps a fast decode from flashing initials on screen. Without
            // one there is nothing to hold back, so the outcome applies at once.
            if delay.is_zero() || resolved == AvatarLoadingStatus::Idle {
                retained.pending = None;
                retained.state.set_loading_status(resolved, now);
            } else {
                retained.pending = Some(resolved);
                retained
                    .state
                    .set_loading_status(AvatarLoadingStatus::Loading, now);
            }
        } else if let Some(pending) = retained.pending.take() {
            retained.state.set_loading_status(pending, now);
        }
        retained.state.poll(now);
    }

    fn sync_checkbox_group(&mut self, key: u64, id: u32, node: &NativeNode) {
        let all = declared_value_names(node, property::ITEMS, MAX_CHECKBOX_GROUP_VALUES);
        let checked = declared_value_names(node, property::VALUES, MAX_CHECKBOX_GROUP_VALUES);
        let disabled = node.boolean(property::DISABLED).unwrap_or(false);
        let listens = declares_change(node);
        let build = |all: &[String], checked: &[String]| {
            CheckboxGroupState::new(all.iter().map(|value| ElementId::named(value.as_str())))
                .checked(checked.iter().map(|value| ElementId::named(value.as_str())))
                .disabled(disabled)
        };
        let mut declared = all.clone();
        declared.push('\u{1f}'.to_string());
        declared.extend(checked.iter().cloned());
        let retained = self.base_ui.checkbox_groups.entry(key).or_default();
        retained.owner = id;
        retained.listens = listens;
        if retained.declared != declared || retained.names != all {
            retained.declared = declared;
            retained.names = all.clone();
            retained.state = build(&all, &checked);
            retained.reported = retained
                .state
                .values()
                .iter()
                .filter_map(|value| name_of(&retained.names, *value))
                .collect();
        } else {
            // The declaration is unchanged, so the core keeps the checked subset it decided.
            let current: Vec<String> = retained
                .state
                .values()
                .iter()
                .filter_map(|value| name_of(&retained.names, *value))
                .collect();
            retained.state = build(&all, &current);
        }
    }

    fn sync_preview_card(
        &mut self,
        key: u64,
        id: u32,
        node: &NativeNode,
        declarations: &BaseUiDeclarations,
    ) {
        let declared_open = node.boolean(property::OPEN).unwrap_or(false);
        // Base UI declares both deadlines on `PreviewCard.Trigger`; a declaration on the root is
        // accepted too, so either composition reaches the same retained state.
        let trigger = declarations.preview_card_delays.get(&key);
        let delay = trigger.and_then(|delays| delays.0).map_or_else(
            || {
                declared_delay(
                    node,
                    property::DELAY,
                    quickgui::DEFAULT_PREVIEW_CARD_DELAY,
                    MAX_PREVIEW_CARD_DELAY,
                )
            },
            |delay| Duration::from_secs_f32(delay / 1000.0).min(MAX_PREVIEW_CARD_DELAY),
        );
        let close_delay = trigger.and_then(|delays| delays.1).map_or_else(
            || {
                declared_delay(
                    node,
                    property::CLOSE_DELAY,
                    quickgui::DEFAULT_PREVIEW_CARD_CLOSE_DELAY,
                    MAX_PREVIEW_CARD_DELAY,
                )
            },
            |delay| Duration::from_secs_f32(delay / 1000.0).min(MAX_PREVIEW_CARD_DELAY),
        );
        let listens = declares_change(node);
        let retained = self.base_ui.preview_cards.entry(key).or_default();
        retained.owner = id;
        retained.listens = listens;
        retained.placement = node
            .string(property::ANCHOR_PLACEMENT)
            .and_then(parse_anchor_placement)
            .unwrap_or(AnchorPlacement::BottomStart);
        retained.anchor_gap = node
            .number(property::ANCHOR_GAP)
            .filter(|gap| gap.is_finite());
        retained.viewport_margin = node
            .number(property::VIEWPORT_MARGIN)
            .filter(|margin| margin.is_finite());
        if retained.declared_open != declared_open {
            retained.declared_open = declared_open;
            retained.state.set_open(declared_open);
            retained.reported = declared_open;
        }
        retained.state = retained.state.delay(delay).close_delay(close_delay);
        retained.state.poll(Instant::now());
    }

    fn sync_scroll_area(
        &mut self,
        key: u64,
        id: u32,
        node: &NativeNode,
        declarations: &BaseUiDeclarations,
    ) {
        let viewport = declared_extent(node, property::VIEWPORT_SIZE);
        let content = declared_extent(node, property::CONTENT_SIZE);
        let threshold = node.number(property::OVERFLOW_EDGE_THRESHOLD);
        // `keepMounted` is Base UI's `ScrollArea.Scrollbar` prop, but the core keeps it per scroll
        // area, so one kept scrollbar keeps the whole area's scrollbars and corner mounted.
        let keep_mounted = node.boolean(property::KEEP_MOUNTED).unwrap_or(false)
            || declarations.kept_scrollbars.contains(&key);
        let listens = declares_change(node);
        let retained = self.base_ui.scroll_areas.entry(key).or_default();
        retained.owner = id;
        retained.listens = listens;
        retained.keep_mounted = keep_mounted;
        if let Some(threshold) = threshold {
            retained.state = retained.state.overflow_edge_threshold(threshold);
        }
        if retained.state.set_geometry(viewport, content) {
            // A track normally spans the viewport along its axis; a press replaces this with the
            // scrollbar's own laid-out extent.
            retained.track_lengths = [viewport.width, viewport.height];
        }
    }

    fn sync_otp_field(&mut self, key: u64, id: u32, node: &NativeNode) {
        let length = node
            .number(property::LENGTH)
            .filter(|length| length.is_finite())
            .map_or(6, |length| (length as usize).clamp(1, MAX_OTP_LENGTH));
        let value = node.string(property::VALUE).unwrap_or("").to_owned();
        let validation = declared_validation(node);
        let mask = node.boolean(property::MASK).unwrap_or(false);
        let disabled = node.boolean(property::DISABLED).unwrap_or(false);
        let read_only = node.boolean(property::READ_ONLY).unwrap_or(false);
        let required = node.boolean(property::REQUIRED).unwrap_or(false);
        let listens = declares_change(node);
        let auto_submit = node
            .string(property::AUTO_SUBMIT)
            .filter(|form| !form.is_empty() && form.len() <= MAX_COMPONENT_VALUE_BYTES)
            .map(ElementId::named);
        let retained = self.base_ui.otp_fields.entry(key).or_default();
        retained.owner = id;
        retained.listens = listens;
        retained.auto_submit = auto_submit;
        if retained.declared_length != length {
            retained.declared_length = length;
            retained.declared_value = value.clone();
            retained.state = OtpFieldState::new(length).value(&value);
            retained.reported = Some(retained.state.text());
        } else if retained.declared_value != value {
            retained.declared_value = value.clone();
            retained.state.set_value(&value);
            retained.reported = Some(retained.state.text());
        }
        retained.state = retained
            .state
            .validation_type(validation)
            .mask(mask)
            .disabled(disabled)
            .read_only(read_only)
            .required(required);
    }

    fn sync_drawer(&mut self, key: u64, id: u32, node: &NativeNode) {
        let direction = declared_swipe_direction(node);
        let modality = declared_modality(node);
        let disable_pointer_dismissal = node
            .boolean(property::DISABLE_POINTER_DISMISSAL)
            .unwrap_or(false);
        let declared_open = node.boolean(property::OPEN).unwrap_or(false);
        let listens = declares_change(node);
        // `DrawerState::snap_points` panics on overflow or on an empty list, so the binding
        // bounds and filters the declaration before it ever reaches the core.
        let mut snap_points: Vec<f32> = declared_numbers(node, property::VALUES)
            .into_iter()
            .filter(|point| point.is_finite() && *point > 0.0)
            .map(|point| point as f32)
            .take(MAX_DRAWER_SNAP_POINTS)
            .collect();
        if snap_points.is_empty() {
            snap_points.push(1.0);
        }
        let default_snap = node
            .number(property::ITEM_INDEX)
            .filter(|index| *index >= 0.0 && index.is_finite())
            .map(|index| (index as usize).min(snap_points.len().saturating_sub(1)));
        let declared = declaration_fingerprint(
            node,
            &[
                property::SWIPE_DIRECTION,
                property::VALUES,
                property::ITEM_INDEX,
                property::DISABLE_POINTER_DISMISSAL,
            ],
        );
        let retained = self.base_ui.drawers.entry(key).or_default();
        retained.owner = id;
        retained.listens = listens;
        retained.modality = modality;
        if retained.declared != declared {
            retained.declared = declared;
            let mut state = DrawerState::new(direction)
                .snap_points(&snap_points)
                .disable_pointer_dismissal(disable_pointer_dismissal);
            if let Some(index) = default_snap {
                state = state.default_snap_point(index);
                state.set_snap_point(index);
            }
            if retained.declared_open {
                state.open();
                if let Some(index) = default_snap {
                    state.set_snap_point(index);
                }
            }
            retained.state = state;
        }
        if retained.declared_open != declared_open {
            retained.declared_open = declared_open;
            retained.state.set_open(declared_open);
        }
    }

    fn sync_navigation_menu(&mut self, key: u64, id: u32, node: &NativeNode, tree: &NativeTree) {
        // Base UI derives the ordered model from the mounted `NavigationMenu.Item` children, so
        // the binding walks the declared subtree in child order when no `items` source is given.
        let declared_items = match bounded_json(node, property::ITEMS) {
            Some(_) => declared_items(node, MAX_NAVIGATION_MENU_ITEMS),
            None => collect_navigation_menu_items(tree, id, key),
        };
        let names: Vec<String> = declared_items
            .iter()
            .map(|item| item.value.clone())
            .collect();
        let items: Vec<NavigationMenuItem> = declared_items
            .iter()
            .map(|item| {
                NavigationMenuItem::new(ElementId::named(item.value.as_str()))
                    .disabled(item.disabled)
            })
            .collect();
        let orientation = declared_menu_orientation(node);
        let delay = declared_delay(
            node,
            property::DELAY,
            quickgui::DEFAULT_NAVIGATION_MENU_DELAY,
            quickgui::MAX_NAVIGATION_MENU_DELAY,
        );
        let close_delay = declared_delay(
            node,
            property::CLOSE_DELAY,
            quickgui::DEFAULT_NAVIGATION_MENU_CLOSE_DELAY,
            quickgui::MAX_NAVIGATION_MENU_DELAY,
        );
        let listens = declares_change(node);
        let declared = node
            .string(property::ACTIVE_VALUE)
            .filter(|value| !value.is_empty() && value.len() <= MAX_COMPONENT_VALUE_BYTES)
            .map(str::to_owned);
        let retained = self.base_ui.navigation_menus.entry(key).or_default();
        retained.owner = id;
        retained.listens = listens;
        retained.names = names;
        retained.items = items;
        retained.loop_focus = node.boolean(property::LOOP_FOCUS).unwrap_or(true);
        retained.placement = node
            .string(property::ANCHOR_PLACEMENT)
            .and_then(parse_anchor_placement)
            .unwrap_or(AnchorPlacement::BottomStart);
        if retained.declared != declared {
            retained.declared = declared.clone();
            let opened = declared.as_deref().map(ElementId::named).and_then(|value| {
                let index = retained
                    .items
                    .iter()
                    .position(|item| item.value() == value)?;
                Some((value, index))
            });
            match opened {
                Some((value, index)) => {
                    retained.state.open(value, index);
                }
                None => {
                    retained.state.close();
                }
            }
        }
        retained.state = retained
            .state
            .orientation(orientation)
            .delay(delay)
            .close_delay(close_delay);
        retained.state.poll(Instant::now());
    }

    /// Enqueue one asynchronous change event per Base UI instance the core moved.
    fn report_base_ui(&mut self, window: u32, events: &EventQueue) {
        for retained in self.base_ui.avatars.values_mut() {
            let status = retained.state.loading_status();
            if retained.reported == Some(status) {
                continue;
            }
            retained.reported = Some(status);
            if retained.listens {
                enqueue_component_change(
                    events,
                    window,
                    retained.owner,
                    serde_json::json!({ "loadingStatus": loading_status_name(status) }),
                );
            }
        }
        for retained in self.base_ui.checkbox_groups.values_mut() {
            let values: Vec<String> = retained
                .state
                .values()
                .iter()
                .filter_map(|value| name_of(&retained.names, *value))
                .collect();
            if values == retained.reported {
                continue;
            }
            retained.reported = values.clone();
            if retained.listens {
                enqueue_component_change(
                    events,
                    window,
                    retained.owner,
                    serde_json::json!({ "checkedValues": values }),
                );
            }
        }
        for retained in self.base_ui.preview_cards.values_mut() {
            let open = retained.state.is_open();
            if open == retained.reported {
                continue;
            }
            retained.reported = open;
            if retained.listens {
                enqueue_component_change(
                    events,
                    window,
                    retained.owner,
                    serde_json::json!({ "open": open }),
                );
            }
        }
        for retained in self.base_ui.scroll_areas.values_mut() {
            let snapshot = retained.snapshot();
            if retained.reported.as_ref() == Some(&snapshot) {
                continue;
            }
            retained.reported = Some(snapshot.clone());
            if retained.listens {
                enqueue_component_change(events, window, retained.owner, snapshot);
            }
        }
        for retained in self.base_ui.otp_fields.values_mut() {
            let value = retained.state.text();
            let complete = retained.state.is_complete();
            let moved = retained.reported.as_deref() != Some(value.as_str());
            let completed = complete && !retained.reported_complete;
            retained.reported_complete = complete;
            if !moved {
                continue;
            }
            retained.reported = Some(value.clone());
            if retained.listens {
                let mut payload = serde_json::json!({ "value": value });
                if completed {
                    payload["complete"] = serde_json::Value::String(value);
                }
                enqueue_component_change(events, window, retained.owner, payload);
            }
        }
        for retained in self.base_ui.drawers.values_mut() {
            let snapshot = serde_json::json!({
                "open": retained.state.is_open(),
                "snapPoint": retained.state.snap_point(),
                "swiping": retained.state.is_swiping(),
                "swipeOffset": retained.state.swipe_offset(),
            });
            if retained.reported.as_ref() == Some(&snapshot) {
                continue;
            }
            retained.reported = Some(snapshot.clone());
            if retained.listens {
                enqueue_component_change(events, window, retained.owner, snapshot);
            }
        }
        for retained in self.base_ui.navigation_menus.values_mut() {
            let value = retained
                .state
                .value()
                .and_then(|value| name_of(&retained.names, value));
            let focused = retained
                .state
                .focused()
                .and_then(|value| name_of(&retained.names, value));
            let snapshot = serde_json::json!({
                "value": value,
                "focused": focused,
                "activationDirection": activation_direction_name(
                    retained.state.activation_direction(),
                ),
            });
            if retained.reported.as_ref() == Some(&snapshot) {
                continue;
            }
            retained.reported = Some(snapshot.clone());
            if retained.listens {
                enqueue_component_change(events, window, retained.owner, snapshot);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Per-instance state accessors
// ---------------------------------------------------------------------------

fn checkbox_group_accessor(key: u64) -> StateAccessor<NativeView, CheckboxGroupState> {
    StateAccessor::new(move |view: &mut NativeView| {
        &mut view
            .components
            .base_ui
            .checkbox_groups
            .entry(key)
            .or_default()
            .state
    })
}

fn preview_card_accessor(key: u64) -> StateAccessor<NativeView, PreviewCardState> {
    StateAccessor::new(move |view: &mut NativeView| {
        &mut view
            .components
            .base_ui
            .preview_cards
            .entry(key)
            .or_default()
            .state
    })
}

fn otp_field_accessor(key: u64) -> StateAccessor<NativeView, OtpFieldState> {
    StateAccessor::new(move |view: &mut NativeView| {
        &mut view
            .components
            .base_ui
            .otp_fields
            .entry(key)
            .or_default()
            .state
    })
}

fn drawer_accessor(key: u64) -> StateAccessor<NativeView, DrawerState> {
    StateAccessor::new(move |view: &mut NativeView| {
        &mut view
            .components
            .base_ui
            .drawers
            .entry(key)
            .or_default()
            .state
    })
}

fn navigation_menu_accessor(key: u64) -> StateAccessor<NativeView, NavigationMenuState> {
    StateAccessor::new(move |view: &mut NativeView| {
        &mut view
            .components
            .base_ui
            .navigation_menus
            .entry(key)
            .or_default()
            .state
    })
}

// ---------------------------------------------------------------------------
// Part application
// ---------------------------------------------------------------------------

/// The core-derived identity one declared Base UI part mounts under.
pub(super) fn base_ui_part_element_id(
    part: &str,
    id: u32,
    node: &NativeNode,
    components: &NativeComponentStates,
) -> Option<ElementId> {
    let key = component_key(id, node);
    let root = ElementId::new(key);
    let scroll_area = || ScrollArea::new(root);
    let preview_card = || native_preview_card(root, false);
    let drawer = || Drawer::new(root, false);
    let otp_field = || OtpField::new(root);
    Some(match part {
        // A preview card's own root part carries no identity in the core — the trigger is the
        // anchor — so it keeps its ordinary node identity and never collides with the trigger.
        AVATAR_PART | CHECKBOX_GROUP_PART | SCROLL_AREA_PART | OTP_FIELD_PART | DRAWER_PART
        | NAVIGATION_MENU_PART => root,
        AVATAR_IMAGE_PART => Avatar::new(root, "").image_id(),
        AVATAR_FALLBACK_PART => Avatar::new(root, "").fallback_id(),
        CHECKBOX_GROUP_ITEM_PART => {
            let state = &components.base_ui.checkbox_groups.get(&key)?.state;
            CheckboxGroup::new(root, state)
                .checkbox_id(native_part_value(node, property::PART_VALUE)?)
        }
        CHECKBOX_GROUP_PARENT_PART => {
            let state = &components.base_ui.checkbox_groups.get(&key)?.state;
            CheckboxGroup::new(root, state).parent_id()
        }
        PREVIEW_CARD_TRIGGER_PART => preview_card().trigger_id(),
        PREVIEW_CARD_PORTAL_PART | PREVIEW_CARD_POSITIONER_PART => preview_card().positioner_id(),
        PREVIEW_CARD_POPUP_PART => preview_card().popup_id(),
        PREVIEW_CARD_ARROW_PART => preview_card().arrow_id(),
        PREVIEW_CARD_BACKDROP_PART => preview_card().backdrop_id(),
        SCROLL_AREA_VIEWPORT_PART => scroll_area().viewport_id(),
        SCROLL_AREA_CONTENT_PART => scroll_area().content_id(),
        SCROLL_AREA_SCROLLBAR_PART => scroll_area().scrollbar_id(declared_orientation(node)),
        SCROLL_AREA_THUMB_PART => scroll_area().thumb_id(declared_orientation(node)),
        SCROLL_AREA_CORNER_PART => scroll_area().corner_id(),
        OTP_FIELD_INPUT_PART => otp_field().input_id(declared_index(node)),
        OTP_FIELD_SEPARATOR_PART => otp_field().separator_id(declared_index(node)),
        DRAWER_TRIGGER_PART => drawer().trigger_id(),
        DRAWER_PORTAL_PART => drawer().portal_id(),
        DRAWER_BACKDROP_PART => drawer().backdrop_id(),
        DRAWER_VIEWPORT_PART => drawer().viewport_id(),
        DRAWER_POPUP_PART => drawer().popup_id(),
        DRAWER_CONTENT_PART => drawer().content_id(),
        DRAWER_TITLE_PART => drawer().title_id(),
        DRAWER_DESCRIPTION_PART => drawer().description_id(),
        DRAWER_CLOSE_PART => drawer().close_id(),
        DRAWER_SWIPE_AREA_PART => drawer().swipe_area_id(),
        NAVIGATION_MENU_LINK_PART => native_part_value(node, property::PART_VALUE)?,
        NAVIGATION_MENU_ITEM_PART
        | NAVIGATION_MENU_TRIGGER_PART
        | NAVIGATION_MENU_ICON_PART
        | NAVIGATION_MENU_PORTAL_PART
        | NAVIGATION_MENU_POSITIONER_PART
        | NAVIGATION_MENU_POPUP_PART
        | NAVIGATION_MENU_VIEWPORT_PART
        | NAVIGATION_MENU_CONTENT_PART
        | NAVIGATION_MENU_ARROW_PART
        | NAVIGATION_MENU_BACKDROP_PART => {
            let retained = components.base_ui.navigation_menus.get(&key)?;
            let value = native_part_value(node, property::PART_VALUE)?;
            let entry = NavigationMenu::new(root, &retained.state, &retained.items).entry(value)?;
            match part {
                NAVIGATION_MENU_ITEM_PART => entry.item_id(),
                NAVIGATION_MENU_TRIGGER_PART => entry.trigger_id(),
                NAVIGATION_MENU_ICON_PART => entry.icon_id(),
                NAVIGATION_MENU_PORTAL_PART | NAVIGATION_MENU_POSITIONER_PART => {
                    entry.positioner_id()
                }
                NAVIGATION_MENU_POPUP_PART => entry.popup_id(),
                NAVIGATION_MENU_VIEWPORT_PART => entry.viewport_id(),
                NAVIGATION_MENU_CONTENT_PART => entry.content_id(),
                NAVIGATION_MENU_ARROW_PART => entry.arrow_id(),
                _ => entry.backdrop_id(),
            }
        }
        _ => return None,
    })
}

/// The popup identity a preview card derives from its scope.
///
/// The core takes caller-owned trigger and popup identities; the binding derives the popup from
/// the scope so both sides resolve without a registry.
fn preview_card_popup_id(root: ElementId) -> ElementId {
    ElementId::new(root.as_u64().rotate_left(23) ^ 0x9e37_79b9_7f4a_7c15)
}

/// The preview card one declared scope resolves to.
pub(super) fn native_preview_card(root: ElementId, open: bool) -> PreviewCard {
    PreviewCard::new(root, preview_card_popup_id(root), open)
}

fn preview_card_descriptor(root: ElementId, retained: &NativePreviewCardState) -> PreviewCard {
    let mut card = PreviewCard::from_state(root, preview_card_popup_id(root), &retained.state)
        .placement(retained.placement);
    if let Some(gap) = retained.anchor_gap {
        card = card.anchor_gap(gap);
    }
    if let Some(margin) = retained.viewport_margin {
        card = card.viewport_margin(margin);
    }
    card
}

fn drawer_descriptor(root: ElementId, retained: &NativeDrawerState) -> Drawer {
    Drawer::from_state(root, &retained.state).modal(retained.modality)
}

/// Apply one declared Base UI part.
///
/// `None` means the core decided this part is not mounted at all — a closed drawer's portal, a
/// preview card that has not opened yet, an avatar image whose source has not loaded, or a
/// scrollbar for an axis that cannot scroll without `keepMounted`.
#[allow(clippy::too_many_lines)]
pub(super) fn apply_base_ui_part(
    element: Element,
    part: &str,
    id: u32,
    node: &NativeNode,
    components: &NativeComponentStates,
    cx: &mut ViewContext<'_, NativeView>,
    listeners_enabled: bool,
) -> Option<Element> {
    let key = component_key(id, node);
    let root = ElementId::new(key);
    Some(match part {
        SEPARATOR_PART => Separator::new(declared_separator_orientation(node)).root_part(element),

        // -------------------------------------------------------------------
        // Avatar
        // -------------------------------------------------------------------
        AVATAR_PART => {
            let Some(retained) = components.base_ui.avatars.get(&key) else {
                return Some(element);
            };
            Avatar::schedule(cx, &retained.state);
            if retained.pending.is_some() {
                // The resolved outcome applies on the next pass, so ask for it immediately
                // instead of waiting out the fallback deadline the `Loading` status armed.
                cx.request_repaint_at(Instant::now());
            }
            Avatar::new(root, Arc::clone(&retained.label)).root_part(element)
        }
        AVATAR_IMAGE_PART => {
            let Some(retained) = components.base_ui.avatars.get(&key) else {
                return Some(element);
            };
            if !retained.state.shows_image() {
                return None;
            }
            Avatar::new(root, Arc::clone(&retained.label)).image_part(element)
        }
        AVATAR_FALLBACK_PART => {
            let Some(retained) = components.base_ui.avatars.get(&key) else {
                return Some(element);
            };
            if !retained.state.shows_fallback() {
                return None;
            }
            Avatar::new(root, Arc::clone(&retained.label)).fallback_part(element)
        }

        // -------------------------------------------------------------------
        // Checkbox group
        // -------------------------------------------------------------------
        CHECKBOX_GROUP_PART => match components.base_ui.checkbox_groups.get(&key) {
            Some(retained) => CheckboxGroup::new(root, &retained.state).root_part(element),
            None => element,
        },
        CHECKBOX_GROUP_ITEM_PART => {
            let Some(retained) = components.base_ui.checkbox_groups.get(&key) else {
                return Some(element);
            };
            let Some(value) = native_part_value(node, property::PART_VALUE) else {
                return Some(element);
            };
            let group = CheckboxGroup::new(root, &retained.state);
            let element = group.checkbox_part(value, element);
            if !listeners_enabled {
                return Some(element);
            }
            let click =
                group.on_checkbox_click_with(cx, value, checkbox_group_accessor(key), |_, _, _| {});
            element.on_click(click)
        }
        CHECKBOX_GROUP_INDICATOR_PART => match components.base_ui.checkbox_groups.get(&key) {
            Some(retained) => CheckboxGroup::new(root, &retained.state).indicator_part(element),
            None => element,
        },
        CHECKBOX_GROUP_PARENT_PART => {
            let Some(retained) = components.base_ui.checkbox_groups.get(&key) else {
                return Some(element);
            };
            let group = CheckboxGroup::new(root, &retained.state);
            let element = group.parent_part(element);
            if !listeners_enabled {
                return Some(element);
            }
            let click = group.on_parent_click_with(cx, checkbox_group_accessor(key), |_, _, _| {});
            element.on_click(click)
        }

        // -------------------------------------------------------------------
        // Preview card
        // -------------------------------------------------------------------
        PREVIEW_CARD_PART => {
            let Some(retained) = components.base_ui.preview_cards.get(&key) else {
                return Some(element);
            };
            PreviewCard::schedule(cx, &retained.state);
            preview_card_descriptor(root, retained).root_part(element)
        }
        PREVIEW_CARD_TRIGGER_PART => {
            let Some(retained) = components.base_ui.preview_cards.get(&key) else {
                return Some(element);
            };
            let card = preview_card_descriptor(root, retained);
            let element = card.trigger_part(element);
            if !listeners_enabled {
                return Some(element);
            }
            let hover = card.on_trigger_hover_with(cx, preview_card_accessor(key), |_, _, _| {});
            element.on_hover(hover)
        }
        PREVIEW_CARD_PORTAL_PART | PREVIEW_CARD_POSITIONER_PART => {
            let Some(retained) = components.base_ui.preview_cards.get(&key) else {
                return Some(element);
            };
            if !retained.state.is_open() {
                return None;
            }
            preview_card_descriptor(root, retained).positioner_part(element)
        }
        PREVIEW_CARD_POPUP_PART => {
            let Some(retained) = components.base_ui.preview_cards.get(&key) else {
                return Some(element);
            };
            if !retained.state.is_open() {
                return None;
            }
            let card = preview_card_descriptor(root, retained);
            let element = card.popup_part(element);
            if !listeners_enabled {
                return Some(element);
            }
            let hover = card.on_popup_hover_with(cx, preview_card_accessor(key), |_, _, _| {});
            let dismiss = card.on_dismiss_with(cx, preview_card_accessor(key), |_, _, _| {});
            element.on_hover(hover).on_dismiss(dismiss)
        }
        PREVIEW_CARD_ARROW_PART => {
            let Some(retained) = components.base_ui.preview_cards.get(&key) else {
                return Some(element);
            };
            if !retained.state.is_open() {
                return None;
            }
            preview_card_descriptor(root, retained).arrow_part(element)
        }
        PREVIEW_CARD_BACKDROP_PART => {
            let Some(retained) = components.base_ui.preview_cards.get(&key) else {
                return Some(element);
            };
            if !retained.state.is_open() {
                return None;
            }
            preview_card_descriptor(root, retained).backdrop_part(element)
        }

        // -------------------------------------------------------------------
        // Scroll area
        // -------------------------------------------------------------------
        SCROLL_AREA_PART => {
            let Some(retained) = components.base_ui.scroll_areas.get(&key) else {
                return Some(element);
            };
            let element = ScrollArea::new(root)
                .keep_mounted(retained.keep_mounted)
                .root_part(element);
            if !listeners_enabled {
                return Some(element);
            }
            let hover = cx.hover_listener(root, move |view, hovered, cx| {
                let hovered = *hovered;
                if let Some(retained) = view.components.base_ui.scroll_areas.get_mut(&key)
                    && retained.state.set_hovering(hovered)
                {
                    cx.invalidate();
                }
            });
            element.on_hover(hover)
        }
        SCROLL_AREA_VIEWPORT_PART => {
            let area = ScrollArea::new(root);
            let element = area.viewport_part(element);
            if !listeners_enabled || !components.base_ui.scroll_areas.contains_key(&key) {
                return Some(element);
            }
            let wheel = cx.scroll_wheel_listener(area.viewport_id(), move |view, event, cx| {
                if let Some(retained) = view.components.base_ui.scroll_areas.get_mut(&key)
                    && retained.state.apply_scroll_wheel(event)
                {
                    cx.invalidate();
                }
            });
            element.on_scroll_wheel(wheel)
        }
        SCROLL_AREA_CONTENT_PART => ScrollArea::new(root).content_part(element),
        SCROLL_AREA_SCROLLBAR_PART => {
            let Some(retained) = components.base_ui.scroll_areas.get(&key) else {
                return Some(element);
            };
            let orientation = declared_orientation(node);
            let area = ScrollArea::new(root).keep_mounted(retained.keep_mounted);
            if !area.shows_scrollbar(&retained.state, orientation) {
                return None;
            }
            let element = area.scrollbar_part(&retained.state, orientation, element);
            if !listeners_enabled {
                return Some(element);
            }
            let press =
                cx.pointer_listener(area.scrollbar_id(orientation), move |view, event, cx| {
                    let Some(retained) = view.components.base_ui.scroll_areas.get_mut(&key) else {
                        return;
                    };
                    // The core delivers the captured track's own laid-out size with the event, so
                    // the exact track length replaces the declared viewport default.
                    let length = if orientation.is_horizontal() {
                        event.size.width
                    } else {
                        event.size.height
                    };
                    retained.track_lengths[axis_index(orientation)] = length;
                    if retained
                        .state
                        .apply_track_pointer(event, orientation, length)
                    {
                        cx.invalidate();
                    }
                });
            element.on_pointer(press)
        }
        SCROLL_AREA_THUMB_PART => {
            let Some(retained) = components.base_ui.scroll_areas.get(&key) else {
                return Some(element);
            };
            let orientation = declared_orientation(node);
            let area = ScrollArea::new(root).keep_mounted(retained.keep_mounted);
            if !area.shows_scrollbar(&retained.state, orientation) {
                return None;
            }
            let element = area.thumb_part(orientation, element);
            if !listeners_enabled {
                return Some(element);
            }
            let drag = cx.pointer_listener(area.thumb_id(orientation), move |view, event, cx| {
                let Some(retained) = view.components.base_ui.scroll_areas.get_mut(&key) else {
                    return;
                };
                let length = retained.track_length(orientation);
                if retained
                    .state
                    .apply_thumb_pointer(event, orientation, length)
                {
                    cx.invalidate();
                }
            });
            element.on_pointer(drag)
        }
        SCROLL_AREA_CORNER_PART => {
            let Some(retained) = components.base_ui.scroll_areas.get(&key) else {
                return Some(element);
            };
            let area = ScrollArea::new(root).keep_mounted(retained.keep_mounted);
            if !area.shows_corner(&retained.state) {
                return None;
            }
            area.corner_part(element)
        }

        // -------------------------------------------------------------------
        // OTP field
        // -------------------------------------------------------------------
        OTP_FIELD_PART => match components.base_ui.otp_fields.get(&key) {
            Some(retained) => OtpField::new(root).root_part(&retained.state, element),
            None => element,
        },
        OTP_FIELD_INPUT_PART => {
            let Some(retained) = components.base_ui.otp_fields.get(&key) else {
                return Some(element);
            };
            let index = declared_index(node);
            if index >= retained.state.length() {
                return None;
            }
            let mut field = OtpField::new(root);
            if let Some(form) = retained.auto_submit {
                field = field.auto_submit(form);
            }
            if !listeners_enabled {
                return Some(field.input_part(&retained.state, index, element));
            }
            field.slot_part_with(
                cx,
                &retained.state,
                index,
                element,
                otp_field_accessor(key),
                |_, _: &str, _| {},
                |_, _: &str, _| {},
            )
        }
        OTP_FIELD_SEPARATOR_PART => {
            OtpField::new(root).separator_part(declared_index(node), element)
        }

        // -------------------------------------------------------------------
        // Drawer
        // -------------------------------------------------------------------
        DRAWER_PART => match components.base_ui.drawers.get(&key) {
            Some(retained) => drawer_descriptor(root, retained).root_part(element),
            None => element,
        },
        DRAWER_TRIGGER_PART => match components.base_ui.drawers.get(&key) {
            Some(retained) => drawer_descriptor(root, retained).trigger_part(element),
            None => element,
        },
        DRAWER_PORTAL_PART
        | DRAWER_BACKDROP_PART
        | DRAWER_VIEWPORT_PART
        | DRAWER_POPUP_PART
        | DRAWER_CONTENT_PART
        | DRAWER_TITLE_PART
        | DRAWER_DESCRIPTION_PART
        | DRAWER_CLOSE_PART
        | DRAWER_SWIPE_AREA_PART => {
            let Some(retained) = components.base_ui.drawers.get(&key) else {
                return Some(element);
            };
            // The Rust guide requires the whole sheet to be mounted only while the drawer is
            // open, so a closed drawer contributes no overlay, focus trap, or accessibility node.
            if !retained.state.is_open() {
                return None;
            }
            let drawer = drawer_descriptor(root, retained);
            match part {
                DRAWER_PORTAL_PART => drawer.portal_part(element),
                DRAWER_BACKDROP_PART => drawer.backdrop_part(element),
                DRAWER_VIEWPORT_PART => drawer.viewport_part(element),
                DRAWER_CONTENT_PART => drawer.content_part(element),
                DRAWER_TITLE_PART => drawer.title_part(element),
                DRAWER_DESCRIPTION_PART => drawer.description_part(element),
                DRAWER_CLOSE_PART => drawer.close_part(
                    node.string(property::ACCESSIBILITY_LABEL)
                        .unwrap_or("Close"),
                    element,
                ),
                DRAWER_SWIPE_AREA_PART => {
                    let element = drawer.swipe_area_part(element);
                    if !listeners_enabled {
                        return Some(element);
                    }
                    let extent = drawer_extent(cx, retained.state.direction());
                    let swipe = drawer.on_swipe_with(
                        cx,
                        extent,
                        drawer_accessor(key),
                        |_, _, _| {},
                        |_, _, _| {},
                    );
                    element.on_pointer(swipe)
                }
                _ => {
                    let element = drawer.popup_part(element);
                    if !listeners_enabled {
                        return Some(element);
                    }
                    let dismiss = drawer.on_dismiss_with(cx, drawer_accessor(key), |_, _, _| {});
                    element.on_dismiss(dismiss)
                }
            }
        }

        // -------------------------------------------------------------------
        // Navigation menu
        // -------------------------------------------------------------------
        NAVIGATION_MENU_PART => {
            let Some(retained) = components.base_ui.navigation_menus.get(&key) else {
                return Some(element);
            };
            NavigationMenu::schedule(cx, &retained.state);
            navigation_menu_descriptor(root, retained).root_part(element)
        }
        NAVIGATION_MENU_LIST_PART => match components.base_ui.navigation_menus.get(&key) {
            Some(retained) => navigation_menu_descriptor(root, retained).list_part(element),
            None => element,
        },
        NAVIGATION_MENU_LINK_PART => {
            let Some(retained) = components.base_ui.navigation_menus.get(&key) else {
                return Some(element);
            };
            let Some(value) = native_part_value(node, property::PART_VALUE) else {
                return Some(element);
            };
            navigation_menu_descriptor(root, retained).link_part(
                value,
                node.boolean(property::CHECKED).unwrap_or(false),
                element,
            )
        }
        _ => {
            let Some(retained) = components.base_ui.navigation_menus.get(&key) else {
                return Some(element);
            };
            let Some(value) = native_part_value(node, property::PART_VALUE) else {
                return Some(element);
            };
            let menu = navigation_menu_descriptor(root, retained);
            let Some(entry) = menu.entry(value) else {
                return Some(element);
            };
            match part {
                NAVIGATION_MENU_ITEM_PART => entry.item_part(element),
                NAVIGATION_MENU_ICON_PART => entry.icon_part(element),
                NAVIGATION_MENU_TRIGGER_PART => {
                    let element = entry.trigger_part(element);
                    if !listeners_enabled {
                        return Some(element);
                    }
                    let access = navigation_menu_accessor(key);
                    let click = entry.on_trigger_click_with(cx, access.clone(), |_, _, _| {});
                    let hover = entry.on_trigger_hover_with(cx, access.clone(), |_, _, _| {});
                    entry
                        .key_part_with(cx, element, access)
                        .on_click(click)
                        .on_hover(hover)
                }
                NAVIGATION_MENU_PORTAL_PART | NAVIGATION_MENU_POSITIONER_PART => {
                    if !entry.is_open() {
                        return None;
                    }
                    entry.positioner_part(element)
                }
                NAVIGATION_MENU_POPUP_PART => {
                    if !entry.is_open() {
                        return None;
                    }
                    let element = entry.popup_part(element);
                    if !listeners_enabled {
                        return Some(element);
                    }
                    let access = navigation_menu_accessor(key);
                    let hover = entry.on_popup_hover_with(cx, access.clone(), |_, _, _| {});
                    let dismiss = entry.on_dismiss_with(cx, access, |_, _, _| {});
                    element.on_hover(hover).on_dismiss(dismiss)
                }
                NAVIGATION_MENU_VIEWPORT_PART => {
                    if !entry.is_open() {
                        return None;
                    }
                    entry.viewport_part(element)
                }
                NAVIGATION_MENU_CONTENT_PART => {
                    if !entry.is_open() {
                        return None;
                    }
                    entry.content_part(element)
                }
                NAVIGATION_MENU_ARROW_PART => {
                    if !entry.is_open() {
                        return None;
                    }
                    entry.arrow_part(element)
                }
                NAVIGATION_MENU_BACKDROP_PART => {
                    if !entry.is_open() {
                        return None;
                    }
                    entry.backdrop_part(element)
                }
                _ => element,
            }
        }
    })
}

fn navigation_menu_descriptor<'a>(
    root: ElementId,
    retained: &'a NativeNavigationMenuState,
) -> NavigationMenu<'a> {
    NavigationMenu::new(root, &retained.state, &retained.items)
        .placement(retained.placement)
        .loop_focus(retained.loop_focus)
}

/// The viewport extent a drawer swipe resolves against.
///
/// The portal covers the whole window viewport on every modality, so the swipe axis extent is the
/// window's own logical size — geometry the core already owns, never re-derived in JavaScript.
fn drawer_extent(cx: &mut ViewContext<'_, NativeView>, direction: SwipeDirection) -> f32 {
    let size = cx.size();
    if direction.is_vertical() {
        size.height
    } else {
        size.width
    }
}

/// Decode a bounded declared list of component value names.
fn declared_value_names(node: &NativeNode, key: u16, limit: usize) -> Vec<String> {
    let Some(source) = bounded_json(node, key) else {
        return Vec::new();
    };
    let Ok(values) = serde_json::from_str::<Vec<String>>(source) else {
        return Vec::new();
    };
    let mut seen = HashSet::new();
    values
        .into_iter()
        .filter(|value| !value.is_empty() && value.len() <= MAX_COMPONENT_VALUE_BYTES)
        .filter(|value| seen.insert(value.clone()))
        .take(limit)
        .collect()
}

/// Declarations Base UI places on a part rather than on the component root.
///
/// Each one is gathered by scope in a single bounded pass, so a part declared after its own root
/// in the tree's iteration order still reaches the retained instance.
#[derive(Default)]
struct BaseUiDeclarations {
    avatar_sources: HashMap<u64, Arc<str>>,
    avatar_delays: HashMap<u64, f32>,
    /// The open and close deadlines a preview card's trigger declared, in milliseconds.
    preview_card_delays: HashMap<u64, (Option<f32>, Option<f32>)>,
    kept_scrollbars: HashSet<u64>,
}

fn gather_base_ui_declarations(tree: &NativeTree) -> BaseUiDeclarations {
    let mut declarations = BaseUiDeclarations::default();
    for (id, node) in &tree.nodes {
        let Some(part) = node.string(property::PART) else {
            continue;
        };
        match part {
            AVATAR_IMAGE_PART if declarations.avatar_sources.len() < MAX_COMPONENT_INSTANCES => {
                let source = node.string(property::VALUE).unwrap_or_default();
                declarations
                    .avatar_sources
                    .insert(component_key(*id, node), Arc::from(source));
            }
            AVATAR_FALLBACK_PART if declarations.avatar_delays.len() < MAX_COMPONENT_INSTANCES => {
                if let Some(delay) = node
                    .number(property::DELAY)
                    .filter(|delay| delay.is_finite() && *delay >= 0.0)
                {
                    declarations
                        .avatar_delays
                        .insert(component_key(*id, node), delay);
                }
            }
            PREVIEW_CARD_TRIGGER_PART
                if declarations.preview_card_delays.len() < MAX_COMPONENT_INSTANCES =>
            {
                let finite = |key: u16| {
                    node.number(key)
                        .filter(|value| value.is_finite() && *value >= 0.0)
                };
                let delays = (finite(property::DELAY), finite(property::CLOSE_DELAY));
                if delays.0.is_some() || delays.1.is_some() {
                    declarations
                        .preview_card_delays
                        .insert(component_key(*id, node), delays);
                }
            }
            SCROLL_AREA_SCROLLBAR_PART
                if declarations.kept_scrollbars.len() < MAX_COMPONENT_INSTANCES
                    && node.boolean(property::KEEP_MOUNTED) == Some(true) =>
            {
                declarations
                    .kept_scrollbars
                    .insert(component_key(*id, node));
            }
            _ => {}
        }
    }
    declarations
}

/// Most declared nodes one navigation menu's own subtree walk visits.
const MAX_NAVIGATION_MENU_SUBTREE_NODES: usize = 4_096;

/// Collect one navigation menu's ordered items from its own declared `Item` parts.
///
/// The walk keeps declaration order, stops at the bounds the core enforces, and skips an item
/// that belongs to a nested menu with a different scope, so a malformed composition costs a
/// shorter model rather than a panic.
fn collect_navigation_menu_items(tree: &NativeTree, root: u32, key: u64) -> Vec<DeclaredItem> {
    let mut items: Vec<DeclaredItem> = Vec::new();
    let mut seen = HashSet::new();
    let mut visited = 0_usize;
    let mut stack = vec![root];
    while let Some(id) = stack.pop() {
        visited += 1;
        if visited > MAX_NAVIGATION_MENU_SUBTREE_NODES || items.len() >= MAX_NAVIGATION_MENU_ITEMS {
            break;
        }
        let Some(node) = tree.nodes.get(&id) else {
            continue;
        };
        if id != root
            && node.string(property::PART) == Some(NAVIGATION_MENU_ITEM_PART)
            && component_key(id, node) == key
            && let Some(value) = node
                .string(property::PART_VALUE)
                .filter(|value| !value.is_empty() && value.len() <= MAX_COMPONENT_VALUE_BYTES)
            && seen.insert(value.to_owned())
        {
            items.push(DeclaredItem {
                value: value.to_owned(),
                disabled: node.boolean(property::DISABLED).unwrap_or(false),
                focusable_when_disabled: node
                    .boolean(property::FOCUSABLE_WHEN_DISABLED)
                    .unwrap_or(true),
            });
        }
        // The children are pushed in reverse so the stack yields them in declaration order.
        stack.extend(node.children.iter().rev().copied());
    }
    items
}
