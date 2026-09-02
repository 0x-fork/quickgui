use std::{
    sync::Arc,
    time::{Duration, Instant},
};

use crate::{AccessibilityLive, AccessibilityRole, Element, ElementId, Key, ViewContext, div};

/// Maximum toasts retained by one queue.
///
/// Pushing past the bound drops the oldest toast, so a burst of application events can never grow
/// the retained tree without limit.
pub const MAX_TOASTS: usize = 8;

/// Maximum UTF-8 bytes retained by one toast title, description, or action label.
pub const MAX_TOAST_TEXT_BYTES: usize = 512;

/// Longest auto-dismiss duration retained by one toast.
pub const MAX_TOAST_DURATION: Duration = Duration::from_secs(60);

const TOAST_ROOT_ID_TAG: u64 = 0x51c8_ae37_60b2_49df;
const TOAST_TITLE_ID_TAG: u64 = 0xd94b_3f16_c827_a05e;
const TOAST_DESCRIPTION_ID_TAG: u64 = 0x0e73_82ba_195c_6df4;
const TOAST_ACTION_ID_TAG: u64 = 0xa620_5d9e_71f3_8c17;
const TOAST_CLOSE_ID_TAG: u64 = 0x3fb1_c74d_2e96_50a8;

/// Urgency of one toast, which selects its live-region politeness and role.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum ToastKind {
    #[default]
    Info,
    Success,
    Warning,
    Error,
}

impl ToastKind {
    /// Whether this kind interrupts the current screen-reader utterance.
    pub const fn is_assertive(self) -> bool {
        matches!(self, Self::Warning | Self::Error)
    }

    const fn live(self) -> AccessibilityLive {
        if self.is_assertive() {
            AccessibilityLive::Assertive
        } else {
            AccessibilityLive::Polite
        }
    }

    const fn role(self) -> AccessibilityRole {
        if self.is_assertive() {
            AccessibilityRole::Alert
        } else {
            AccessibilityRole::Status
        }
    }
}

/// Stable identity of one queued toast.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ToastId(u64);

impl ToastId {
    pub const fn value(self) -> u64 {
        self.0
    }
}

/// One caller-declared toast.
///
/// Text is shared and bounded. The application owns every visual declaration; QuickGUI retains
/// only the strings it announces and the exact duration after which the toast disappears.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Toast {
    title: Arc<str>,
    description: Option<Arc<str>>,
    action: Option<Arc<str>>,
    kind: ToastKind,
    duration: Option<Duration>,
}

impl Toast {
    /// Create a toast with an auto-dismiss duration of five seconds.
    pub fn new(title: impl Into<Arc<str>>) -> Self {
        Self {
            title: bounded_text(title.into()),
            description: None,
            action: None,
            kind: ToastKind::Info,
            duration: Some(Duration::from_secs(5)),
        }
    }

    #[must_use]
    pub fn description(mut self, description: impl Into<Arc<str>>) -> Self {
        let description = bounded_text(description.into());
        self.description = (!description.is_empty()).then_some(description);
        self
    }

    /// Label one caller-owned action control, such as *Undo*.
    #[must_use]
    pub fn action(mut self, action: impl Into<Arc<str>>) -> Self {
        let action = bounded_text(action.into());
        self.action = (!action.is_empty()).then_some(action);
        self
    }

    #[must_use]
    pub const fn kind(mut self, kind: ToastKind) -> Self {
        self.kind = kind;
        self
    }

    /// Replace the auto-dismiss duration, clamped to [`MAX_TOAST_DURATION`].
    #[must_use]
    pub fn duration(mut self, duration: Duration) -> Self {
        self.duration = Some(duration.min(MAX_TOAST_DURATION));
        self
    }

    /// Keep this toast until it is dismissed explicitly.
    #[must_use]
    pub const fn persistent(mut self) -> Self {
        self.duration = None;
        self
    }

    pub fn title(&self) -> &Arc<str> {
        &self.title
    }

    pub fn description_text(&self) -> Option<&Arc<str>> {
        self.description.as_ref()
    }

    pub fn action_label(&self) -> Option<&Arc<str>> {
        self.action.as_ref()
    }

    pub const fn toast_kind(&self) -> ToastKind {
        self.kind
    }

    pub const fn duration_value(&self) -> Option<Duration> {
        self.duration
    }
}

/// One queued toast with its exact remaining lifetime.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ToastEntry {
    id: ToastId,
    toast: Toast,
    deadline: Option<Instant>,
    remaining: Option<Duration>,
}

impl ToastEntry {
    pub const fn id(&self) -> ToastId {
        self.id
    }

    pub const fn toast(&self) -> &Toast {
        &self.toast
    }

    /// The exact instant this toast disappears, or `None` while paused or persistent.
    pub const fn deadline(&self) -> Option<Instant> {
        self.deadline
    }

    /// Whether this toast's countdown is currently paused by hover or focus.
    pub const fn is_paused(&self) -> bool {
        self.remaining.is_some()
    }
}

/// A bounded in-window toast queue with exact one-shot auto-dismissal.
///
/// The manager owns no timer. It reports the single next deadline through [`Self::next_deadline`]
/// so the application sleeps exactly once with [`crate::AsyncViewContext::sleep_until`] and then
/// calls [`Self::expire`]. An empty or fully paused queue reports no deadline at all, so a settled
/// window keeps zero idle sources.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ToastManager {
    entries: Vec<ToastEntry>,
    next_id: u64,
}

impl ToastManager {
    pub const fn new() -> Self {
        Self {
            entries: Vec::new(),
            next_id: 1,
        }
    }

    pub fn entries(&self) -> &[ToastEntry] {
        &self.entries
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn entry(&self, id: ToastId) -> Option<&ToastEntry> {
        self.entries.iter().find(|entry| entry.id == id)
    }

    /// Queue one toast, returning its stable identity.
    ///
    /// Reaching [`MAX_TOASTS`] drops the oldest toast rather than growing the queue.
    pub fn push(&mut self, toast: Toast, now: Instant) -> ToastId {
        if self.entries.len() == MAX_TOASTS {
            self.entries.remove(0);
        }
        let id = ToastId(self.next_id);
        self.next_id = self.next_id.wrapping_add(1).max(1);
        let deadline = toast.duration.map(|duration| now + duration);
        self.entries.push(ToastEntry {
            id,
            toast,
            deadline,
            remaining: None,
        });
        id
    }

    /// Remove one toast, returning whether it was queued.
    pub fn dismiss(&mut self, id: ToastId) -> bool {
        let before = self.entries.len();
        self.entries.retain(|entry| entry.id != id);
        self.entries.len() != before
    }

    /// Remove every toast, returning whether the queue changed.
    pub fn clear(&mut self) -> bool {
        if self.entries.is_empty() {
            return false;
        }
        self.entries.clear();
        true
    }

    /// Pause one toast's countdown on hover or focus, returning whether it changed.
    pub fn pause(&mut self, id: ToastId, now: Instant) -> bool {
        let Some(entry) = self.entries.iter_mut().find(|entry| entry.id == id) else {
            return false;
        };
        if entry.remaining.is_some() {
            return false;
        }
        let Some(deadline) = entry.deadline.take() else {
            return false;
        };
        entry.remaining = Some(deadline.saturating_duration_since(now));
        true
    }

    /// Resume one paused toast's countdown, returning whether it changed.
    pub fn resume(&mut self, id: ToastId, now: Instant) -> bool {
        let Some(entry) = self.entries.iter_mut().find(|entry| entry.id == id) else {
            return false;
        };
        let Some(remaining) = entry.remaining.take() else {
            return false;
        };
        entry.deadline = Some(now + remaining);
        true
    }

    /// The single next auto-dismiss instant across the queue.
    pub fn next_deadline(&self) -> Option<Instant> {
        self.entries.iter().filter_map(|entry| entry.deadline).min()
    }

    /// Remove every toast whose exact deadline has passed, returning whether the queue changed.
    pub fn expire(&mut self, now: Instant) -> bool {
        let before = self.entries.len();
        self.entries
            .retain(|entry| entry.deadline.is_none_or(|deadline| deadline > now));
        self.entries.len() != before
    }
}

/// A controlled, unstyled toast-viewport descriptor.
///
/// The application owns the viewport's placement, stacking, spacing, colors, icons, and motion.
/// QuickGUI supplies stable identities, live-region announcements chosen by toast kind, exact
/// title and description relationships, and focused Escape dismissal.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[must_use = "a ToastViewport descriptor has no effect until its parts are mounted"]
pub struct ToastViewport {
    root_id: ElementId,
}

impl ToastViewport {
    pub fn new(root_id: impl Into<ElementId>) -> Self {
        Self {
            root_id: root_id.into(),
        }
    }

    pub const fn viewport_id(self) -> ElementId {
        self.root_id
    }

    /// Decorate the application-owned viewport without adding layout or appearance.
    ///
    /// The viewport itself is an ordinary group; each toast is its own live region, so an
    /// unchanged queue announces nothing.
    pub fn viewport_part(self, viewport: Element) -> Element {
        viewport
            .id(self.root_id)
            .accessibility_role(AccessibilityRole::Group)
            .app_region_no_drag()
    }

    /// Describe the parts of one queued toast.
    pub fn toast(self, entry: &ToastEntry) -> ToastParts {
        ToastParts {
            viewport: self,
            id: entry.id,
            kind: entry.toast.kind,
            has_description: entry.toast.description.is_some(),
        }
    }
}

/// A copyable declaration for the parts of one queued toast.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[must_use = "a ToastParts descriptor has no effect until its parts are mounted"]
pub struct ToastParts {
    viewport: ToastViewport,
    id: ToastId,
    kind: ToastKind,
    has_description: bool,
}

impl ToastParts {
    pub const fn id(self) -> ToastId {
        self.id
    }

    pub const fn kind(self) -> ToastKind {
        self.kind
    }

    pub fn root_id(self) -> ElementId {
        derived_toast_id(self.viewport.root_id, TOAST_ROOT_ID_TAG, self.id.0)
    }

    pub fn title_id(self) -> ElementId {
        derived_toast_id(self.viewport.root_id, TOAST_TITLE_ID_TAG, self.id.0)
    }

    pub fn description_id(self) -> ElementId {
        derived_toast_id(self.viewport.root_id, TOAST_DESCRIPTION_ID_TAG, self.id.0)
    }

    pub fn action_id(self) -> ElementId {
        derived_toast_id(self.viewport.root_id, TOAST_ACTION_ID_TAG, self.id.0)
    }

    pub fn close_id(self) -> ElementId {
        derived_toast_id(self.viewport.root_id, TOAST_CLOSE_ID_TAG, self.id.0)
    }

    /// Decorate the application-owned toast root without adding layout or appearance.
    ///
    /// Informational toasts project a polite Status region; warnings and errors project an
    /// assertive Alert region. The root is focusable so keyboard users can reach the toast's
    /// action and close controls, and so Escape can dismiss the toast that has focus.
    pub fn root_part(self, root: Element) -> Element {
        let root = root
            .id(self.root_id())
            .accessibility_role(self.kind.role())
            .accessibility_live(self.kind.live())
            .accessibility_labelled_by(self.title_id())
            .focusable()
            .app_region_no_drag();
        if self.has_description {
            root.accessibility_described_by(self.description_id())
        } else {
            root
        }
    }

    /// Decorate the application-owned title, which names the toast.
    pub fn title_part(self, title: Element) -> Element {
        title.id(self.title_id())
    }

    /// Decorate the application-owned description, which describes the toast.
    pub fn description_part(self, description: Element) -> Element {
        description.id(self.description_id())
    }

    /// Decorate the application-owned action control.
    pub fn action_part(self, action: Element) -> Element {
        action
            .id(self.action_id())
            .accessibility_role(AccessibilityRole::Button)
            .clickable()
            .cursor_default()
            .app_region_no_drag()
            .user_select_none()
    }

    /// Decorate the application-owned close control.
    pub fn close_part(self, close: Element) -> Element {
        close
            .id(self.close_id())
            .accessibility_role(AccessibilityRole::Button)
            .clickable()
            .cursor_default()
            .app_region_no_drag()
            .user_select_none()
    }

    /// Attach focused Escape dismissal to this toast's root.
    ///
    /// Escape is handled only while focus is inside this toast, so it never competes with a
    /// dialog, popover, or the application's own Escape handling.
    pub fn key_part<V: 'static>(
        self,
        cx: &mut ViewContext<'_, V>,
        root: Element,
        access: fn(&mut V) -> &mut ToastManager,
    ) -> Element {
        let id = self.id;
        let dismiss = cx.key_down_listener(self.root_id(), move |view, event, cx| {
            if event.key != Key::Escape || event.repeat {
                return;
            }
            if access(view).dismiss(id) {
                cx.stop_propagation();
                cx.prevent_default();
                cx.invalidate();
            }
        });
        root.on_key_down(dismiss)
    }
}

fn bounded_text(text: Arc<str>) -> Arc<str> {
    if text.len() <= MAX_TOAST_TEXT_BYTES {
        return text;
    }
    let mut end = MAX_TOAST_TEXT_BYTES;
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    Arc::from(&text[..end])
}

/// Create an unstyled toast-viewport root.
///
/// This shorthand is equivalent to `ToastViewport::new(id).viewport_part(div())`.
pub fn toast_viewport(id: impl Into<ElementId>) -> Element {
    ToastViewport::new(id).viewport_part(div())
}

fn derived_toast_id(scope: ElementId, tag: u64, toast: u64) -> ElementId {
    let mut hash = scope
        .as_u64()
        .rotate_left(5)
        .wrapping_add(toast.rotate_right(17))
        ^ tag;
    hash ^= hash >> 30;
    hash = hash.wrapping_mul(0xbf58_476d_1ce4_e5b9);
    hash ^= hash >> 27;
    hash = hash.wrapping_mul(0x94d0_49bb_1331_11eb);
    hash ^= hash >> 31;
    if hash == 0 || hash == u64::MAX || hash == scope.as_u64() {
        hash ^= tag.rotate_left(13);
    }
    ElementId::new(hash)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Color, IntoElement, TestAppContext, View, button, text};

    #[test]
    fn queue_is_bounded_and_countdowns_are_exact() {
        let start = Instant::now();
        let mut manager = ToastManager::new();
        assert!(manager.is_empty());
        assert_eq!(manager.next_deadline(), None);
        assert!(!manager.expire(start));

        let saved = manager.push(Toast::new("Saved").duration(Duration::from_secs(4)), start);
        let failed = manager.push(
            Toast::new("Upload failed")
                .description("The network connection dropped.")
                .action("Retry")
                .kind(ToastKind::Error)
                .duration(Duration::from_secs(8)),
            start,
        );
        assert_eq!(manager.len(), 2);
        assert_ne!(saved, failed);
        assert_eq!(
            manager.next_deadline(),
            Some(start + Duration::from_secs(4))
        );

        assert!(!manager.expire(start + Duration::from_secs(3)));
        assert!(manager.expire(start + Duration::from_secs(5)));
        assert_eq!(manager.len(), 1);
        assert!(manager.entry(saved).is_none());
        assert_eq!(
            manager.next_deadline(),
            Some(start + Duration::from_secs(8))
        );

        // Hover or focus pauses the countdown; nothing expires while paused.
        assert!(manager.pause(failed, start + Duration::from_secs(5)));
        assert!(!manager.pause(failed, start + Duration::from_secs(5)));
        assert_eq!(manager.next_deadline(), None);
        assert!(manager.entry(failed).expect("paused toast").is_paused());
        assert!(!manager.expire(start + Duration::from_secs(60)));
        assert!(manager.resume(failed, start + Duration::from_secs(60)));
        assert_eq!(
            manager.next_deadline(),
            Some(start + Duration::from_secs(63))
        );
        assert!(!manager.resume(failed, start));
        assert!(manager.dismiss(failed));

        let persistent = manager.push(Toast::new("Recording").persistent(), start);
        assert_eq!(
            manager.entry(persistent).expect("persistent").deadline(),
            None
        );
        assert!(!manager.pause(persistent, start));
        assert!(!manager.expire(start + Duration::from_secs(3_600)));
        assert!(manager.entry(persistent).is_some());

        assert!(manager.dismiss(persistent));
        assert!(!manager.dismiss(persistent));
        assert!(!manager.clear());
        manager.push(Toast::new("Another"), start);
        assert!(manager.clear());

        for index in 0..MAX_TOASTS + 3 {
            manager.push(Toast::new(format!("Toast {index}")), start);
        }
        assert_eq!(manager.len(), MAX_TOASTS);
        assert_eq!(
            manager.entries()[0].toast().title().as_ref(),
            format!("Toast {}", 3).as_str()
        );

        let clamped = Toast::new("Long").duration(Duration::from_secs(600));
        assert_eq!(clamped.duration_value(), Some(MAX_TOAST_DURATION));
        let long = "x".repeat(MAX_TOAST_TEXT_BYTES + 40);
        let bounded = Toast::new(long.clone()).description(long).action("");
        assert_eq!(bounded.title().len(), MAX_TOAST_TEXT_BYTES);
        assert_eq!(
            bounded.description_text().map(|text| text.len()),
            Some(MAX_TOAST_TEXT_BYTES)
        );
        assert_eq!(bounded.action_label(), None);
    }

    #[test]
    fn kinds_select_live_politeness_and_role() {
        assert!(!ToastKind::Info.is_assertive());
        assert!(!ToastKind::Success.is_assertive());
        assert!(ToastKind::Warning.is_assertive());
        assert!(ToastKind::Error.is_assertive());

        let start = Instant::now();
        let mut manager = ToastManager::new();
        manager.push(Toast::new("Saved").kind(ToastKind::Success), start);
        manager.push(
            Toast::new("Upload failed")
                .description("Retry when back online.")
                .kind(ToastKind::Error),
            start,
        );
        let viewport = ToastViewport::new("toasts");
        let polite = viewport.toast(&manager.entries()[0]);
        let assertive = viewport.toast(&manager.entries()[1]);

        let polite_root = polite.root_part(div().bg(Color::rgb8(1, 2, 3)));
        assert_eq!(polite_root.accessibility.role, AccessibilityRole::Status);
        assert_eq!(
            polite_root.accessibility.live,
            Some(AccessibilityLive::Polite)
        );
        assert_eq!(
            polite_root.accessibility.relations.labelled_by(),
            Some(polite.title_id())
        );
        assert_eq!(polite_root.accessibility.relations.described_by(), None);
        assert!(polite_root.focusable);
        assert_eq!(polite_root.visual.background, Some(Color::rgb8(1, 2, 3)));

        let assertive_root = assertive.root_part(div());
        assert_eq!(assertive_root.accessibility.role, AccessibilityRole::Alert);
        assert_eq!(
            assertive_root.accessibility.live,
            Some(AccessibilityLive::Assertive)
        );
        assert_eq!(
            assertive_root.accessibility.relations.described_by(),
            Some(assertive.description_id())
        );

        let viewport_element = viewport.viewport_part(div().gap_2());
        assert_eq!(viewport_element.explicit_id, Some("toasts".into()));
        assert_eq!(
            viewport_element.accessibility.role,
            AccessibilityRole::Group
        );
        assert!(viewport_element.accessibility.live.is_none());

        let title = polite.title_part(text("Saved"));
        assert_eq!(title.explicit_id, Some(polite.title_id()));
        let description = assertive.description_part(text("Retry when back online."));
        assert_eq!(description.explicit_id, Some(assertive.description_id()));
        let action = assertive.action_part(div().child("Retry"));
        assert_eq!(action.explicit_id, Some(assertive.action_id()));
        assert!(action.clickable);
        let close = assertive.close_part(div().child("×"));
        assert_eq!(close.explicit_id, Some(assertive.close_id()));
        assert!(close.clickable);

        let ids = [
            viewport.viewport_id(),
            polite.root_id(),
            polite.title_id(),
            polite.description_id(),
            polite.action_id(),
            polite.close_id(),
            assertive.root_id(),
            assertive.title_id(),
        ];
        for (index, id) in ids.iter().enumerate() {
            assert!(!ids[..index].contains(id));
        }
    }

    struct ToastView {
        toasts: ToastManager,
        undone: bool,
    }

    impl Default for ToastView {
        fn default() -> Self {
            Self {
                toasts: ToastManager::new(),
                undone: false,
            }
        }
    }

    impl ToastView {
        fn toasts(view: &mut Self) -> &mut ToastManager {
            &mut view.toasts
        }
    }

    impl View for ToastView {
        fn render(&mut self, cx: &mut ViewContext<'_, Self>) -> impl IntoElement {
            let now = Instant::now();
            let publish = cx.listener("publish", move |view: &mut Self, cx| {
                view.toasts.push(
                    Toast::new("Item deleted")
                        .description("The item moved to the trash.")
                        .action("Undo")
                        .kind(ToastKind::Warning),
                    now,
                );
                cx.invalidate();
            });

            let viewport = ToastViewport::new("toasts");
            let mut surface = viewport.viewport_part(div());
            for entry in self.toasts.entries() {
                let parts = viewport.toast(entry);
                let id = entry.id();
                let undo = cx.listener(parts.action_id(), move |view: &mut Self, cx| {
                    view.undone = true;
                    view.toasts.dismiss(id);
                    cx.invalidate();
                });
                let close = cx.listener(parts.close_id(), move |view: &mut Self, cx| {
                    view.toasts.dismiss(id);
                    cx.invalidate();
                });
                let mut root = div().child(parts.title_part(text(entry.toast().title().clone())));
                if let Some(description) = entry.toast().description_text() {
                    root = root.child(parts.description_part(text(description.clone())));
                }
                if let Some(label) = entry.toast().action_label() {
                    root = root
                        .child(parts.action_part(div().child(text(label.clone())).on_click(undo)));
                }
                root = root.child(parts.close_part(div().child(text("Close")).on_click(close)));
                surface = surface.child(parts.key_part(cx, parts.root_part(root), Self::toasts));
            }

            div()
                .child(button().id("publish").child("Delete").on_click(publish))
                .child(surface)
        }
    }

    #[test]
    fn live_regions_escape_and_idle_paths_are_deterministic() {
        let (mut cx, view) = TestAppContext::new(ToastView::default()).unwrap();
        let window = view.window_handle();
        let viewport = ToastViewport::new("toasts");

        cx.click(window, "publish").unwrap();
        assert_eq!(cx.read(view, |view| view.toasts.len()).unwrap(), 1);
        let first = cx.read(view, |view| view.toasts.entries()[0].id()).unwrap();
        let parts = ToastParts {
            viewport,
            id: first,
            kind: ToastKind::Warning,
            has_description: true,
        };

        let update = cx.accessibility_update(window).unwrap();
        let node = |id: ElementId| {
            update
                .nodes
                .iter()
                .find_map(|(node_id, node)| (node_id.0 == id.as_u64()).then_some(node))
                .expect("toast accessibility node")
        };
        let root = node(parts.root_id());
        assert_eq!(root.role(), accesskit::Role::Alert);
        assert_eq!(root.live(), Some(accesskit::Live::Assertive));
        assert_eq!(
            root.labelled_by(),
            &[accesskit::NodeId(parts.title_id().as_u64())]
        );
        assert_eq!(
            root.described_by(),
            &[accesskit::NodeId(parts.description_id().as_u64())]
        );

        cx.click(window, parts.action_id()).unwrap();
        assert!(cx.read(view, |view| view.undone).unwrap());
        assert!(cx.read(view, |view| view.toasts.is_empty()).unwrap());

        cx.click(window, "publish").unwrap();
        let second = cx.read(view, |view| view.toasts.entries()[0].id()).unwrap();
        let second_parts = ToastParts {
            viewport,
            id: second,
            kind: ToastKind::Warning,
            has_description: true,
        };
        cx.focus(window, second_parts.root_id()).unwrap();
        cx.simulate_keystrokes(window, "escape").unwrap();
        assert!(cx.read(view, |view| view.toasts.is_empty()).unwrap());

        cx.click(window, "publish").unwrap();
        let third = cx.read(view, |view| view.toasts.entries()[0].id()).unwrap();
        let third_parts = ToastParts {
            viewport,
            id: third,
            kind: ToastKind::Warning,
            has_description: true,
        };
        cx.click(window, third_parts.close_id()).unwrap();
        assert!(cx.read(view, |view| view.toasts.is_empty()).unwrap());

        let renders = cx.render_count(window).unwrap();
        cx.run_until_idle().unwrap();
        assert_eq!(cx.render_count(window).unwrap(), renders);
    }

    #[test]
    fn shorthand_is_a_semantic_unstyled_root() {
        let element = toast_viewport("toasts");
        assert_eq!(element.accessibility.role, AccessibilityRole::Group);
        assert!(element.children.is_empty());
        assert_eq!(element.visual.background, None);
    }
}
