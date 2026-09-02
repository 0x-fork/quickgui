use std::sync::Arc;

use crate::{
    AccessibilityOrientation, AccessibilityRole, Element, ElementId, FocusHandle, KeyBinding,
    StateAccessor, ViewContext, div,
};

/// Maximum items retained in one toolbar declaration.
///
/// Roving focus scans the caller's ordered item list rather than a registry, so the bound keeps
/// one keypress a bounded walk even when application data drives the toolbar.
pub const MAX_TOOLBAR_ITEMS: usize = 256;

const TOOLBAR_HORIZONTAL_KEY_CONTEXT: &str = "ToolbarHorizontal";
const TOOLBAR_VERTICAL_KEY_CONTEXT: &str = "ToolbarVertical";
const TOOLBAR_ITEM_ID_TAG: u64 = 0x9f14_c6b7_2a05_d8e3;

/// Move toolbar focus to the next enabled item.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ToolbarNext;
/// Move toolbar focus to the previous enabled item.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ToolbarPrevious;
/// Move toolbar focus to the first enabled item.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ToolbarFirst;
/// Move toolbar focus to the final enabled item.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ToolbarLast;

/// Contextual bindings used by [`ToolbarEntry::key_part`].
///
/// A horizontal toolbar answers Left and Right; a vertical toolbar answers Up and Down. The other
/// axis is left to the surrounding application, which matches the tab-list contract.
pub fn toolbar_key_bindings() -> [KeyBinding; 8] {
    [
        KeyBinding::new("right", ToolbarNext, Some(TOOLBAR_HORIZONTAL_KEY_CONTEXT)),
        KeyBinding::new(
            "left",
            ToolbarPrevious,
            Some(TOOLBAR_HORIZONTAL_KEY_CONTEXT),
        ),
        KeyBinding::new("home", ToolbarFirst, Some(TOOLBAR_HORIZONTAL_KEY_CONTEXT)),
        KeyBinding::new("end", ToolbarLast, Some(TOOLBAR_HORIZONTAL_KEY_CONTEXT)),
        KeyBinding::new("down", ToolbarNext, Some(TOOLBAR_VERTICAL_KEY_CONTEXT)),
        KeyBinding::new("up", ToolbarPrevious, Some(TOOLBAR_VERTICAL_KEY_CONTEXT)),
        KeyBinding::new("home", ToolbarFirst, Some(TOOLBAR_VERTICAL_KEY_CONTEXT)),
        KeyBinding::new("end", ToolbarLast, Some(TOOLBAR_VERTICAL_KEY_CONTEXT)),
    ]
}

/// Layout and keyboard axis for one toolbar.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum ToolbarOrientation {
    #[default]
    Horizontal,
    Vertical,
}

impl ToolbarOrientation {
    const fn accessibility(self) -> AccessibilityOrientation {
        match self {
            Self::Horizontal => AccessibilityOrientation::Horizontal,
            Self::Vertical => AccessibilityOrientation::Vertical,
        }
    }

    const fn key_context(self) -> &'static str {
        match self {
            Self::Horizontal => TOOLBAR_HORIZONTAL_KEY_CONTEXT,
            Self::Vertical => TOOLBAR_VERTICAL_KEY_CONTEXT,
        }
    }
}

/// Allocation-free controlled roving-focus value for one toolbar.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ToolbarState {
    active: Option<ElementId>,
}

impl ToolbarState {
    pub fn new(active: impl Into<ElementId>) -> Self {
        Self {
            active: Some(active.into()),
        }
    }

    pub const fn empty() -> Self {
        Self { active: None }
    }

    pub const fn active(&self) -> Option<ElementId> {
        self.active
    }

    /// Replace the roving tab stop, returning whether it changed.
    pub fn set_active(&mut self, active: Option<ElementId>) -> bool {
        if self.active == active {
            false
        } else {
            self.active = active;
            true
        }
    }

    pub fn focus(&mut self, value: impl Into<ElementId>) -> bool {
        self.set_active(Some(value.into()))
    }

    pub fn clear(&mut self) -> bool {
        self.set_active(None)
    }
}

/// One caller-declared toolbar item.
///
/// The application owns the item's content and behavior. QuickGUI needs only its stable value and
/// whether it participates in keyboard navigation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ToolbarItem {
    value: ElementId,
    disabled: bool,
}

impl ToolbarItem {
    pub fn new(value: impl Into<ElementId>) -> Self {
        Self {
            value: value.into(),
            disabled: false,
        }
    }

    #[must_use]
    pub const fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    pub const fn value(self) -> ElementId {
        self.value
    }

    pub const fn is_disabled(self) -> bool {
        self.disabled
    }
}

/// A controlled, unstyled toolbar descriptor.
///
/// The application owns every item, separator, icon, label, and visual state. QuickGUI supplies
/// the Toolbar role and orientation, one roving Tab stop, bounded arrow/Home/End navigation, and
/// disabled-item skipping. The descriptor borrows the caller's ordered item list and retains no
/// registry, task, timer, observer, or idle scheduler source.
#[derive(Clone, Copy, Debug)]
#[must_use = "a Toolbar descriptor has no effect until its parts are mounted"]
pub struct Toolbar<'a> {
    root_id: ElementId,
    items: &'a [ToolbarItem],
    active: Option<ElementId>,
    orientation: ToolbarOrientation,
    loop_focus: bool,
}

impl<'a> Toolbar<'a> {
    /// Create a toolbar over the caller's ordered items.
    ///
    /// # Panics
    ///
    /// Panics with more than [`MAX_TOOLBAR_ITEMS`] items or duplicate item values.
    pub fn new(
        root_id: impl Into<ElementId>,
        state: &ToolbarState,
        items: &'a [ToolbarItem],
    ) -> Self {
        assert_toolbar_items(items);
        Self {
            root_id: root_id.into(),
            items,
            active: state.active(),
            orientation: ToolbarOrientation::Horizontal,
            loop_focus: true,
        }
    }

    pub const fn orientation(mut self, orientation: ToolbarOrientation) -> Self {
        self.orientation = orientation;
        self
    }

    pub const fn vertical(self) -> Self {
        self.orientation(ToolbarOrientation::Vertical)
    }

    /// Wrap from the last item to the first. The default is `true`.
    pub const fn loop_focus(mut self, loop_focus: bool) -> Self {
        self.loop_focus = loop_focus;
        self
    }

    pub const fn root_id(self) -> ElementId {
        self.root_id
    }

    pub const fn items(self) -> &'a [ToolbarItem] {
        self.items
    }

    pub const fn axis(self) -> ToolbarOrientation {
        self.orientation
    }

    pub fn item_id(self, value: impl Into<ElementId>) -> ElementId {
        derived_toolbar_id(self.root_id, value.into())
    }

    /// The item that currently owns the toolbar's single Tab stop.
    ///
    /// This is the controlled active value while it names an enabled mounted item, and otherwise
    /// the first enabled item, matching the composite-widget convention.
    pub fn roving_value(self) -> Option<ElementId> {
        let active = self.active.filter(|active| {
            self.items
                .iter()
                .any(|item| item.value == *active && !item.disabled)
        });
        active.or_else(|| {
            self.items
                .iter()
                .find(|item| !item.disabled)
                .map(|item| item.value)
        })
    }

    /// Decorate an application-owned toolbar root without adding layout or appearance.
    pub fn root_part(self, root: Element) -> Element {
        root.id(self.root_id)
            .accessibility_role(AccessibilityRole::Toolbar)
            .accessibility_orientation(self.orientation.accessibility())
            .app_region_no_drag()
    }

    /// Describe one declared item.
    pub fn item(self, value: impl Into<ElementId>) -> Option<ToolbarEntry<'a>> {
        let value = value.into();
        let item = self
            .items
            .iter()
            .copied()
            .find(|item| item.value == value)?;
        Some(ToolbarEntry {
            toolbar: self,
            item,
            roving: self.roving_value() == Some(value),
        })
    }
}

/// A copyable declaration for one mounted toolbar item.
#[derive(Clone, Copy, Debug)]
#[must_use = "a ToolbarEntry descriptor has no effect until its part is mounted"]
pub struct ToolbarEntry<'a> {
    toolbar: Toolbar<'a>,
    item: ToolbarItem,
    roving: bool,
}

impl<'a> ToolbarEntry<'a> {
    pub const fn value(self) -> ElementId {
        self.item.value
    }

    pub const fn is_disabled(self) -> bool {
        self.item.disabled
    }

    /// Whether this item owns the toolbar's single Tab stop.
    pub const fn is_roving_stop(self) -> bool {
        self.roving
    }

    pub fn item_id(self) -> ElementId {
        self.toolbar.item_id(self.item.value)
    }

    /// Decorate an application-owned item without adding appearance.
    ///
    /// Exactly one enabled item is in the window's normal Tab sequence; the rest are reachable
    /// with the toolbar's arrow keys.
    pub fn item_part(self, item: Element) -> Element {
        let disabled = self.item.disabled || item.accessibility.disabled;
        item.id(self.item_id())
            .focusable()
            .tab_index(if self.roving { 0 } else { -1 })
            .key_context(self.toolbar.orientation.key_context())
            .cursor_default()
            .app_region_no_drag()
            .user_select_none()
            .disabled(disabled)
    }

    /// Attach QuickGUI's typed toolbar navigation to this item.
    ///
    /// Each item answers its own arrow keys, so the focused item is always the one that moves.
    /// Install [`toolbar_key_bindings`] once on the application keymap.
    pub fn key_part<V: 'static>(
        self,
        cx: &mut ViewContext<'_, V>,
        item: Element,
        access: fn(&mut V) -> &mut ToolbarState,
    ) -> Element {
        self.key_part_with(cx, item, StateAccessor::from(access))
    }

    /// Attach the typed toolbar navigation against a per-instance state accessor.
    ///
    /// A host that renders many declared toolbars through one view passes an accessor that
    /// captures which [`ToolbarState`] this item belongs to.
    pub fn key_part_with<V: 'static>(
        self,
        cx: &mut ViewContext<'_, V>,
        item: Element,
        access_source: StateAccessor<V, ToolbarState>,
    ) -> Element {
        let id = self.item_id();
        let root = self.toolbar.root_id;
        let value = self.item.value;
        let loop_focus = self.toolbar.loop_focus;
        let items: Arc<[ToolbarItem]> = Arc::from(self.toolbar.items);

        let next_items = items.clone();
        let access = access_source.clone();
        let next = cx.action_listener(id, move |view, _: &ToolbarNext, cx| {
            move_focus(
                view,
                cx,
                &access,
                root,
                neighbor_value(&next_items, value, true, loop_focus),
            );
        });
        let previous_items = items.clone();
        let access = access_source.clone();
        let previous = cx.action_listener(id, move |view, _: &ToolbarPrevious, cx| {
            move_focus(
                view,
                cx,
                &access,
                root,
                neighbor_value(&previous_items, value, false, loop_focus),
            );
        });
        let first_items = items.clone();
        let access = access_source.clone();
        let first = cx.action_listener(id, move |view, _: &ToolbarFirst, cx| {
            move_focus(view, cx, &access, root, edge_value(&first_items, false));
        });
        let access = access_source;
        let last = cx.action_listener(id, move |view, _: &ToolbarLast, cx| {
            move_focus(view, cx, &access, root, edge_value(&items, true));
        });

        item.on_action(next)
            .on_action(previous)
            .on_action(first)
            .on_action(last)
    }
}

fn move_focus<V: 'static>(
    view: &mut V,
    cx: &mut crate::EventContext,
    access: &StateAccessor<V, ToolbarState>,
    root: ElementId,
    target: Option<ElementId>,
) {
    let Some(target) = target else {
        return;
    };
    let changed = access.get(view).focus(target);
    cx.focus(FocusHandle::new(derived_toolbar_id(root, target)));
    if changed {
        cx.invalidate();
    }
}

fn neighbor_value(
    items: &[ToolbarItem],
    from: ElementId,
    forward: bool,
    loop_focus: bool,
) -> Option<ElementId> {
    let position = items.iter().position(|item| item.value == from)?;
    let count = items.len();
    let mut index = position;
    for _ in 0..count {
        index = if forward {
            if index + 1 == count {
                if !loop_focus {
                    return None;
                }
                0
            } else {
                index + 1
            }
        } else if index == 0 {
            if !loop_focus {
                return None;
            }
            count - 1
        } else {
            index - 1
        };
        if !items[index].disabled {
            return Some(items[index].value);
        }
    }
    None
}

fn edge_value(items: &[ToolbarItem], last: bool) -> Option<ElementId> {
    let mut enabled = items.iter().filter(|item| !item.disabled);
    if last {
        enabled.next_back().map(|item| item.value)
    } else {
        enabled.next().map(|item| item.value)
    }
}

fn assert_toolbar_items(items: &[ToolbarItem]) {
    assert!(
        items.len() <= MAX_TOOLBAR_ITEMS,
        "a toolbar retains at most {MAX_TOOLBAR_ITEMS} items"
    );
    for (index, item) in items.iter().enumerate() {
        assert!(
            !items[..index]
                .iter()
                .any(|earlier| earlier.value == item.value),
            "toolbar item values must be unique"
        );
    }
}

/// Create an unstyled semantic toolbar root.
///
/// This shorthand is equivalent to `Toolbar::new(id, state, items).root_part(div())`.
pub fn toolbar(id: impl Into<ElementId>, state: &ToolbarState, items: &[ToolbarItem]) -> Element {
    Toolbar::new(id, state, items).root_part(div())
}

fn derived_toolbar_id(scope: ElementId, value: ElementId) -> ElementId {
    let mut hash = scope
        .as_u64()
        .rotate_left(13)
        .wrapping_add(value.as_u64().rotate_right(29))
        ^ TOOLBAR_ITEM_ID_TAG;
    hash ^= hash >> 30;
    hash = hash.wrapping_mul(0xbf58_476d_1ce4_e5b9);
    hash ^= hash >> 27;
    hash = hash.wrapping_mul(0x94d0_49bb_1331_11eb);
    hash ^= hash >> 31;
    if hash == 0 || hash == u64::MAX || hash == scope.as_u64() || hash == value.as_u64() {
        hash ^= TOOLBAR_ITEM_ID_TAG.rotate_left(19);
    }
    ElementId::new(hash)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        AppRegion, Application, Color, CursorStyle, IntoElement, UserSelect, View, WindowOptions,
        button, text,
    };

    fn items() -> [ToolbarItem; 4] {
        [
            ToolbarItem::new("bold"),
            ToolbarItem::new("italic").disabled(true),
            ToolbarItem::new("underline"),
            ToolbarItem::new("link"),
        ]
    }

    #[test]
    fn roving_stop_and_parts_are_exact_and_unstyled() {
        let items = items();
        let state = ToolbarState::empty();
        let toolbar = Toolbar::new("format", &state, &items);
        assert_eq!(toolbar.roving_value(), Some("bold".into()));

        let focused = ToolbarState::new("underline");
        let toolbar = Toolbar::new("format", &focused, &items);
        assert_eq!(toolbar.roving_value(), Some("underline".into()));

        // A disabled or absent active value falls back to the first enabled item.
        let disabled_active = ToolbarState::new("italic");
        assert_eq!(
            Toolbar::new("format", &disabled_active, &items).roving_value(),
            Some("bold".into())
        );
        let missing = ToolbarState::new("missing");
        assert_eq!(
            Toolbar::new("format", &missing, &items).roving_value(),
            Some("bold".into())
        );

        let root = toolbar.root_part(div().gap_2().bg(Color::rgb8(1, 2, 3)));
        assert_eq!(root.explicit_id, Some("format".into()));
        assert_eq!(root.accessibility.role, AccessibilityRole::Toolbar);
        assert_eq!(
            root.accessibility.orientation,
            Some(AccessibilityOrientation::Horizontal)
        );
        assert_eq!(root.app_region, Some(AppRegion::NoDrag));
        assert_eq!(root.visual.background, Some(Color::rgb8(1, 2, 3)));
        assert!(!root.focusable);

        let underline = toolbar.item("underline").expect("underline entry");
        assert!(underline.is_roving_stop());
        let element = underline.item_part(button().child("Underline"));
        assert_eq!(element.explicit_id, Some(toolbar.item_id("underline")));
        assert_eq!(element.tab_index, 0);
        assert!(element.focusable);
        assert_eq!(element.cursor_style, Some(CursorStyle::Arrow));
        assert_eq!(element.user_select, UserSelect::None);

        let bold = toolbar.item("bold").expect("bold entry");
        assert!(!bold.is_roving_stop());
        assert_eq!(bold.item_part(div()).tab_index, -1);

        let italic = toolbar.item("italic").expect("italic entry");
        assert!(italic.is_disabled());
        assert!(italic.item_part(div()).accessibility.disabled);
        assert!(toolbar.item("absent").is_none());

        let vertical = Toolbar::new("sidebar", &focused, &items).vertical();
        assert_eq!(
            vertical.root_part(div()).accessibility.orientation,
            Some(AccessibilityOrientation::Vertical)
        );

        let ids = [
            toolbar.root_id(),
            toolbar.item_id("bold"),
            toolbar.item_id("italic"),
            toolbar.item_id("underline"),
        ];
        for (index, id) in ids.iter().enumerate() {
            assert!(!ids[..index].contains(id));
        }
    }

    #[test]
    fn navigation_skips_disabled_items_and_honors_looping() {
        let items = items();
        assert_eq!(
            neighbor_value(&items, "bold".into(), true, true),
            Some("underline".into())
        );
        assert_eq!(
            neighbor_value(&items, "link".into(), true, true),
            Some("bold".into())
        );
        assert_eq!(neighbor_value(&items, "link".into(), true, false), None);
        assert_eq!(
            neighbor_value(&items, "bold".into(), false, true),
            Some("link".into())
        );
        assert_eq!(neighbor_value(&items, "bold".into(), false, false), None);
        assert_eq!(neighbor_value(&items, "absent".into(), true, true), None);
        assert_eq!(edge_value(&items, false), Some("bold".into()));
        assert_eq!(edge_value(&items, true), Some("link".into()));

        let all_disabled = [ToolbarItem::new("a").disabled(true)];
        assert_eq!(edge_value(&all_disabled, false), None);
        assert_eq!(neighbor_value(&all_disabled, "a".into(), true, true), None);
    }

    #[derive(Default)]
    struct ToolbarView {
        toolbar: ToolbarState,
        clicked: Option<ElementId>,
    }

    impl ToolbarView {
        fn toolbar(view: &mut Self) -> &mut ToolbarState {
            &mut view.toolbar
        }
    }

    impl View for ToolbarView {
        fn render(&mut self, cx: &mut ViewContext<'_, Self>) -> impl IntoElement {
            let items = items();
            let toolbar = Toolbar::new("format", &self.toolbar, &items);
            let mut root = toolbar.root_part(div());
            for item in items {
                let entry = toolbar.item(item.value()).expect("declared entry");
                let value = item.value();
                let clicked = cx.listener(entry.item_id(), move |view: &mut Self, cx| {
                    view.clicked = Some(value);
                    view.toolbar.focus(value);
                    cx.invalidate();
                });
                root = root.child(entry.key_part(
                    cx,
                    entry.item_part(div().child(text("Item")).on_click(clicked)),
                    Self::toolbar,
                ));
            }
            div()
                .child(button().id("before").child("Before"))
                .child(root)
        }
    }

    #[test]
    fn keyboard_focus_and_accessibility_paths_stay_deterministic() {
        let (mut cx, view) = Application::new()
            .bind_keys(toolbar_key_bindings())
            .into_test_context(WindowOptions::default(), ToolbarView::default())
            .unwrap();
        let window = view.window_handle();
        let state = ToolbarState::empty();
        let items = items();
        let toolbar = Toolbar::new("format", &state, &items);

        cx.simulate_keystrokes(window, "tab tab").unwrap();
        assert_eq!(cx.focused(window).unwrap(), Some(toolbar.item_id("bold")));
        cx.simulate_keystrokes(window, "right").unwrap();
        assert_eq!(
            cx.focused(window).unwrap(),
            Some(toolbar.item_id("underline"))
        );
        assert_eq!(
            cx.read(view, |view| view.toolbar.active()).unwrap(),
            Some("underline".into())
        );
        cx.simulate_keystrokes(window, "right right").unwrap();
        assert_eq!(cx.focused(window).unwrap(), Some(toolbar.item_id("bold")));
        cx.simulate_keystrokes(window, "left").unwrap();
        assert_eq!(cx.focused(window).unwrap(), Some(toolbar.item_id("link")));
        cx.simulate_keystrokes(window, "home").unwrap();
        assert_eq!(cx.focused(window).unwrap(), Some(toolbar.item_id("bold")));
        cx.simulate_keystrokes(window, "end").unwrap();
        assert_eq!(cx.focused(window).unwrap(), Some(toolbar.item_id("link")));
        // The cross axis is left to the application.
        cx.simulate_keystrokes(window, "down").unwrap();
        assert_eq!(cx.focused(window).unwrap(), Some(toolbar.item_id("link")));
        assert!(cx.click(window, toolbar.item_id("italic")).is_err());

        // Only the roving item is in the normal Tab sequence.
        cx.focus(window, "before").unwrap();
        cx.simulate_keystrokes(window, "tab").unwrap();
        assert_eq!(cx.focused(window).unwrap(), Some(toolbar.item_id("link")));

        let update = cx.accessibility_update(window).unwrap();
        let node = |id: ElementId| {
            update
                .nodes
                .iter()
                .find_map(|(node_id, node)| (node_id.0 == id.as_u64()).then_some(node))
                .expect("toolbar accessibility node")
        };
        assert_eq!(node("format".into()).role(), accesskit::Role::Toolbar);
        assert_eq!(
            node("format".into()).orientation(),
            Some(accesskit::Orientation::Horizontal)
        );
        assert!(node(toolbar.item_id("italic")).is_disabled());

        let renders = cx.render_count(window).unwrap();
        cx.run_until_idle().unwrap();
        assert_eq!(cx.render_count(window).unwrap(), renders);
    }

    #[test]
    fn bindings_are_contextual_and_complete() {
        let bindings = toolbar_key_bindings();
        assert_eq!(bindings.len(), 8);
        assert!(bindings.iter().all(|binding| {
            binding.context_predicate().is_some_and(|context| {
                context
                    .depth_of(&[crate::KeyContext::parse(TOOLBAR_HORIZONTAL_KEY_CONTEXT).unwrap()])
                    .is_some()
                    || context
                        .depth_of(
                            &[crate::KeyContext::parse(TOOLBAR_VERTICAL_KEY_CONTEXT).unwrap()],
                        )
                        .is_some()
            })
        }));
    }

    #[test]
    #[should_panic(expected = "toolbar item values must be unique")]
    fn duplicate_item_values_are_rejected() {
        let items = [ToolbarItem::new("same"), ToolbarItem::new("same")];
        let _ = Toolbar::new("format", &ToolbarState::empty(), &items);
    }

    #[test]
    fn shorthand_is_a_semantic_unstyled_root() {
        let items = items();
        let element = toolbar("format", &ToolbarState::empty(), &items);
        assert_eq!(element.accessibility.role, AccessibilityRole::Toolbar);
        assert!(element.children.is_empty());
        assert_eq!(element.visual.background, None);
    }
}
