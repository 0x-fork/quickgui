use crate::{AccessibilityRole, Element, ElementId};

use crate::element::{AccessibilityOrientation, TabListBehavior};

const TABS_LIST_ID_TAG: u64 = 0x80bc_1828_48ed_58bf;
const TAB_ID_TAG: u64 = 0xf4a8_90a4_2ce9_3357;
const TAB_PANEL_ID_TAG: u64 = 0x37d8_909d_30bb_71c1;
const TAB_INDICATOR_ID_TAG: u64 = 0x11c4_1646_4527_d985;

/// Layout and keyboard axis for one unstyled tab set.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum TabsOrientation {
    #[default]
    Horizontal,
    Vertical,
}

/// Allocation-free controlled active value for one tab set.
///
/// QuickGUI deliberately does not retain or discover application values outside the mounted tree.
/// The application owns this state, mutates it only from direct interaction, and rebuilds the
/// descriptor from [`Tabs::from_state`].
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct TabsState {
    active: Option<ElementId>,
}

impl TabsState {
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

    /// Replace the controlled value, returning whether it changed.
    pub fn set_active(&mut self, active: Option<ElementId>) -> bool {
        if self.active == active {
            false
        } else {
            self.active = active;
            true
        }
    }

    pub fn select(&mut self, value: impl Into<ElementId>) -> bool {
        self.set_active(Some(value.into()))
    }

    pub fn clear(&mut self) -> bool {
        self.set_active(None)
    }
}

/// A controlled, unstyled in-window tab-set descriptor.
///
/// The application owns the active value, all content, layout, typography, colors, indicator
/// geometry, and motion. QuickGUI supplies stable part identities, one roving Tab stop, bounded
/// arrow/Home/End navigation, optional activation on arrow focus, disabled-item skipping, exact
/// tab/list/panel accessibility semantics, and optional hidden panel retention.
///
/// Manual activation and looping focus match Base UI's defaults. This descriptor retains no item
/// registry, allocation, task, timer, observer, animation, or idle scheduler source.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[must_use = "a Tabs descriptor has no effect until its parts are mounted"]
pub struct Tabs {
    root_id: ElementId,
    active: Option<ElementId>,
    orientation: TabsOrientation,
    activate_on_focus: bool,
    loop_focus: bool,
    keep_mounted: bool,
}

impl Tabs {
    pub fn new(root_id: impl Into<ElementId>, active: impl Into<ElementId>) -> Self {
        Self {
            root_id: root_id.into(),
            active: Some(active.into()),
            orientation: TabsOrientation::Horizontal,
            activate_on_focus: false,
            loop_focus: true,
            keep_mounted: false,
        }
    }

    pub fn without_selection(root_id: impl Into<ElementId>) -> Self {
        Self {
            root_id: root_id.into(),
            active: None,
            orientation: TabsOrientation::Horizontal,
            activate_on_focus: false,
            loop_focus: true,
            keep_mounted: false,
        }
    }

    pub fn from_state(root_id: impl Into<ElementId>, state: &TabsState) -> Self {
        let mut tabs = Self::without_selection(root_id);
        tabs.active = state.active();
        tabs
    }

    pub const fn orientation(mut self, orientation: TabsOrientation) -> Self {
        self.orientation = orientation;
        self
    }

    pub const fn vertical(self) -> Self {
        self.orientation(TabsOrientation::Vertical)
    }

    /// Activate an enabled tab as arrow/Home/End navigation moves focus to it.
    ///
    /// The default is manual activation: arrows move focus and Enter or Space invokes the tab's
    /// ordinary click listener.
    pub const fn activate_on_focus(mut self, activate_on_focus: bool) -> Self {
        self.activate_on_focus = activate_on_focus;
        self
    }

    pub const fn loop_focus(mut self, loop_focus: bool) -> Self {
        self.loop_focus = loop_focus;
        self
    }

    /// Retain inactive panels as `display: none` instead of omitting them from the mounted tree.
    pub const fn keep_mounted(mut self, keep_mounted: bool) -> Self {
        self.keep_mounted = keep_mounted;
        self
    }

    pub const fn root_id(self) -> ElementId {
        self.root_id
    }

    pub fn list_id(self) -> ElementId {
        derived_tabs_id(self.root_id, self.root_id, TABS_LIST_ID_TAG)
    }

    pub const fn active_value(self) -> Option<ElementId> {
        self.active
    }

    pub const fn activates_on_focus(self) -> bool {
        self.activate_on_focus
    }

    pub const fn loops_focus(self) -> bool {
        self.loop_focus
    }

    pub const fn keeps_panels_mounted(self) -> bool {
        self.keep_mounted
    }

    /// Decorate an application-owned structural root without adding role or appearance.
    pub fn root_part(self, root: Element) -> Element {
        root.id(self.root_id)
    }

    /// Decorate an application-owned tab-list root.
    pub fn list_part(self, list: Element) -> Element {
        let mut list = list
            .id(self.list_id())
            .accessibility_role(AccessibilityRole::TabList);
        let vertical = self.orientation == TabsOrientation::Vertical;
        list.accessibility.orientation = Some(if vertical {
            AccessibilityOrientation::Vertical
        } else {
            AccessibilityOrientation::Horizontal
        });
        list.tab_list_behavior = Some(TabListBehavior {
            vertical,
            activate_on_focus: self.activate_on_focus,
            loop_focus: self.loop_focus,
        });
        list
    }

    pub fn tab(self, value: impl Into<ElementId>) -> Tab {
        let value = value.into();
        Tab {
            tabs_id: self.root_id,
            value,
            state: TabState {
                active: self.active == Some(value),
                disabled: false,
                orientation: self.orientation,
            },
            keep_mounted: self.keep_mounted,
        }
    }
}

/// Caller-visible state projected across one tab's unstyled parts.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TabState {
    pub active: bool,
    pub disabled: bool,
    pub orientation: TabsOrientation,
}

/// A copyable declaration for one controlled tab and its associated panel.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[must_use = "a Tab descriptor has no effect until one of its parts is mounted"]
pub struct Tab {
    tabs_id: ElementId,
    value: ElementId,
    state: TabState,
    keep_mounted: bool,
}

impl Tab {
    pub const fn disabled(mut self, disabled: bool) -> Self {
        self.state.disabled = disabled;
        self
    }

    pub const fn keep_mounted(mut self, keep_mounted: bool) -> Self {
        self.keep_mounted = keep_mounted;
        self
    }

    pub const fn value(self) -> ElementId {
        self.value
    }

    pub const fn state(self) -> TabState {
        self.state
    }

    pub const fn is_active(self) -> bool {
        self.state.active
    }

    pub const fn is_disabled(self) -> bool {
        self.state.disabled
    }

    pub const fn panel_is_mounted(self) -> bool {
        self.state.active || self.keep_mounted
    }

    pub fn tab_id(self) -> ElementId {
        derived_tabs_id(self.tabs_id, self.value, TAB_ID_TAG)
    }

    pub fn panel_id(self) -> ElementId {
        derived_tabs_id(self.tabs_id, self.value, TAB_PANEL_ID_TAG)
    }

    pub fn indicator_id(self) -> ElementId {
        derived_tabs_id(self.tabs_id, self.value, TAB_INDICATOR_ID_TAG)
    }

    /// Decorate an application-owned tab button without adding appearance.
    pub fn tab_part(self, tab: Element) -> Element {
        let disabled = self.state.disabled || tab.accessibility.disabled;
        tab.id(self.tab_id())
            .accessibility_role(AccessibilityRole::Tab)
            .selected(self.state.active)
            .accessibility_controls(self.panel_id())
            .clickable()
            .tab_index(0)
            .cursor_default()
            .app_region_no_drag()
            .user_select_none()
            .disabled(disabled)
    }

    /// Mount a caller-owned decorative indicator only for the active tab.
    ///
    /// Indicator geometry and motion are intentionally application-owned. Put this part inside the
    /// tab or position it absolutely in the caller's list layout.
    pub fn indicator_part(self, indicator: Element) -> Option<Element> {
        self.state
            .active
            .then(|| indicator.id(self.indicator_id()).accessibility_hidden(true))
    }

    /// Decorate and mount this tab's application-owned panel.
    ///
    /// The active panel is focusable so Tab can move from the composite tab list into panel
    /// content even when that content has no focusable first child. Inactive retained panels use
    /// `display: none`, contributing no layout, paint, input, accessibility node, or runtime work.
    pub fn panel_part(self, panel: Element) -> Option<Element> {
        let panel = panel
            .id(self.panel_id())
            .accessibility_role(AccessibilityRole::TabPanel)
            .accessibility_labelled_by(self.tab_id())
            .focusable();
        if self.state.active {
            Some(panel)
        } else if self.keep_mounted {
            Some(panel.hidden())
        } else {
            None
        }
    }
}

fn derived_tabs_id(scope: ElementId, value: ElementId, tag: u64) -> ElementId {
    let mut hash = scope
        .as_u64()
        .rotate_left(19)
        .wrapping_add(value.as_u64().rotate_right(7))
        ^ tag;
    hash ^= hash >> 30;
    hash = hash.wrapping_mul(0xbf58_476d_1ce4_e5b9);
    hash ^= hash >> 27;
    hash = hash.wrapping_mul(0x94d0_49bb_1331_11eb);
    hash ^= hash >> 31;
    if hash == 0 || hash == u64::MAX || hash == scope.as_u64() || hash == value.as_u64() {
        hash ^= tag.rotate_left(29);
    }
    ElementId::new(hash)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        AppRegion, Color, CursorStyle, IntoElement, TestAppContext, UserSelect, View, ViewContext,
        button, div, text,
    };

    #[test]
    fn state_and_parts_are_bounded_unstyled_and_exact() {
        let mut state = TabsState::new("overview");
        assert_eq!(state.active(), Some("overview".into()));
        assert!(!state.select("overview"));
        assert!(state.select("files"));
        assert!(state.clear());
        assert!(!state.clear());

        let tabs = Tabs::new("workspace", "overview")
            .vertical()
            .activate_on_focus(true)
            .loop_focus(false)
            .keep_mounted(true);
        assert_eq!(tabs.active_value(), Some("overview".into()));
        assert!(tabs.activates_on_focus());
        assert!(!tabs.loops_focus());
        assert!(tabs.keeps_panels_mounted());

        let root = tabs.root_part(div().w(317.0).bg(Color::rgb8(1, 2, 3)));
        assert_eq!(root.explicit_id, Some(tabs.root_id()));
        assert_eq!(root.visual.background, Some(Color::rgb8(1, 2, 3)));

        let list = tabs.list_part(div().gap_3().border(2.0, Color::rgb8(4, 5, 6)));
        assert_eq!(list.explicit_id, Some(tabs.list_id()));
        assert_eq!(list.accessibility.role, AccessibilityRole::TabList);
        assert_eq!(
            list.accessibility.orientation,
            Some(AccessibilityOrientation::Vertical)
        );
        assert_eq!(
            list.tab_list_behavior,
            Some(TabListBehavior {
                vertical: true,
                activate_on_focus: true,
                loop_focus: false,
            })
        );
        assert_eq!(list.visual.border_width, 2.0);
        assert!(list.key_listeners.is_none());

        let active = tabs.tab("overview");
        assert_eq!(
            active.state(),
            TabState {
                active: true,
                disabled: false,
                orientation: TabsOrientation::Vertical,
            }
        );
        let ids = [active.tab_id(), active.panel_id(), active.indicator_id()];
        for (index, id) in ids.iter().enumerate() {
            assert_ne!(*id, tabs.root_id());
            assert_ne!(*id, tabs.list_id());
            assert!(!ids[..index].contains(id));
        }

        let tab = active.tab_part(div().px_4().bg(Color::rgb8(7, 8, 9)));
        assert_eq!(tab.explicit_id, Some(active.tab_id()));
        assert_eq!(tab.accessibility.role, AccessibilityRole::Tab);
        assert!(tab.accessibility.selected);
        assert_eq!(
            tab.accessibility.relations.controls(),
            Some(active.panel_id())
        );
        assert!(tab.clickable);
        assert!(tab.focusable);
        assert_eq!(tab.tab_index, 0);
        assert_eq!(tab.cursor_style, Some(CursorStyle::Arrow));
        assert_eq!(tab.app_region, Some(AppRegion::NoDrag));
        assert_eq!(tab.user_select, UserSelect::None);
        assert_eq!(tab.visual.background, Some(Color::rgb8(7, 8, 9)));
        assert!(tab.transition.is_none());

        let indicator = active
            .indicator_part(div().h(3.0).bg(Color::rgb8(10, 11, 12)))
            .expect("active indicator");
        assert_eq!(indicator.explicit_id, Some(active.indicator_id()));
        assert!(indicator.accessibility.hidden);
        assert_eq!(indicator.visual.background, Some(Color::rgb8(10, 11, 12)));

        let panel = active
            .panel_part(div().bg(Color::rgb8(13, 14, 15)))
            .expect("active panel");
        assert_eq!(panel.explicit_id, Some(active.panel_id()));
        assert_eq!(panel.accessibility.role, AccessibilityRole::TabPanel);
        assert_eq!(
            panel.accessibility.relations.labelled_by(),
            Some(active.tab_id())
        );
        assert!(panel.focusable);
        assert!(!panel.is_display_none());

        let inactive = tabs.tab("files");
        assert!(!inactive.is_active());
        assert!(inactive.indicator_part(div()).is_none());
        assert!(
            inactive
                .panel_part(div())
                .expect("retained inactive panel")
                .is_display_none()
        );
        let disabled = inactive.disabled(true).tab_part(div());
        assert!(disabled.accessibility.disabled);
        assert!(!disabled.accessibility.selected);
    }

    struct TabsView {
        manual: TabsState,
        automatic: TabsState,
    }

    impl Default for TabsView {
        fn default() -> Self {
            Self {
                manual: TabsState::new("overview"),
                automatic: TabsState::new("alpha"),
            }
        }
    }

    impl View for TabsView {
        fn render(&mut self, cx: &mut ViewContext<'_, Self>) -> impl IntoElement {
            let manual = Tabs::from_state("manual-tabs", &self.manual);
            let overview = manual.tab("overview");
            let disabled = manual.tab("disabled").disabled(true);
            let files = manual.tab("files");
            let settings = manual.tab("settings");
            let select_overview = cx.listener(overview.tab_id(), |view, cx| {
                if view.manual.select("overview") {
                    cx.invalidate();
                }
            });
            let select_disabled = cx.listener(disabled.tab_id(), |_view, _cx| {});
            let select_files = cx.listener(files.tab_id(), |view, cx| {
                if view.manual.select("files") {
                    cx.invalidate();
                }
            });
            let select_settings = cx.listener(settings.tab_id(), |view, cx| {
                if view.manual.select("settings") {
                    cx.invalidate();
                }
            });

            let automatic = Tabs::from_state("automatic-tabs", &self.automatic)
                .vertical()
                .activate_on_focus(true);
            let alpha = automatic.tab("alpha");
            let beta = automatic.tab("beta").disabled(true);
            let gamma = automatic.tab("gamma");
            let delta = automatic.tab("delta");
            let select_alpha = cx.listener(alpha.tab_id(), |view, cx| {
                if view.automatic.select("alpha") {
                    cx.invalidate();
                }
            });
            let select_beta = cx.listener(beta.tab_id(), |_view, _cx| {});
            let select_gamma = cx.listener(gamma.tab_id(), |view, cx| {
                if view.automatic.select("gamma") {
                    cx.invalidate();
                }
            });
            let select_delta = cx.listener(delta.tab_id(), |view, cx| {
                if view.automatic.select("delta") {
                    cx.invalidate();
                }
            });

            div()
                .child(button().id("before").child("Before"))
                .child(
                    manual.root_part(
                        div()
                            .child(
                                manual.list_part(
                                    div()
                                        .child(
                                            overview
                                                .tab_part(div().child("Overview"))
                                                .on_click(select_overview),
                                        )
                                        .child(
                                            disabled
                                                .tab_part(div().child("Disabled"))
                                                .on_click(select_disabled),
                                        )
                                        .child(
                                            files
                                                .tab_part(div().child("Files"))
                                                .on_click(select_files),
                                        )
                                        .child(
                                            settings
                                                .tab_part(div().child("Settings"))
                                                .on_click(select_settings),
                                        ),
                                ),
                            )
                            .children(overview.panel_part(text("Overview panel")))
                            .children(disabled.panel_part(text("Disabled panel")))
                            .children(files.panel_part(text("Files panel")))
                            .children(settings.panel_part(text("Settings panel"))),
                    ),
                )
                .child(button().id("between").child("Between"))
                .child(
                    automatic.root_part(
                        div()
                            .child(
                                automatic.list_part(
                                    div()
                                        .child(
                                            alpha
                                                .tab_part(div().child("Alpha"))
                                                .on_click(select_alpha),
                                        )
                                        .child(
                                            beta.tab_part(div().child("Beta"))
                                                .on_click(select_beta),
                                        )
                                        .child(
                                            gamma
                                                .tab_part(div().child("Gamma"))
                                                .on_click(select_gamma),
                                        )
                                        .child(
                                            delta
                                                .tab_part(div().child("Delta"))
                                                .on_click(select_delta),
                                        ),
                                ),
                            )
                            .children(alpha.panel_part(text("Alpha panel")))
                            .children(beta.panel_part(text("Beta panel")))
                            .children(gamma.panel_part(text("Gamma panel")))
                            .children(delta.panel_part(text("Delta panel"))),
                    ),
                )
                .child(button().id("after").child("After"))
        }
    }

    #[test]
    fn keyboard_mounting_accessibility_and_idle_paths_are_deterministic() {
        let (mut cx, view) = TestAppContext::new(TabsView::default()).unwrap();
        let window = view.window_handle();
        let manual = Tabs::new("manual-tabs", "overview");
        let overview = manual.tab("overview");
        let disabled = manual.tab("disabled").disabled(true);
        let files = manual.tab("files");
        let settings = manual.tab("settings");
        let automatic = Tabs::new("automatic-tabs", "alpha")
            .vertical()
            .activate_on_focus(true);
        let alpha = automatic.tab("alpha");
        let beta = automatic.tab("beta").disabled(true);
        let gamma = automatic.tab("gamma");
        let delta = automatic.tab("delta");

        cx.simulate_keystrokes(window, "tab tab").unwrap();
        assert_eq!(cx.focused(window).unwrap(), Some(overview.tab_id()));
        cx.simulate_keystrokes(window, "right").unwrap();
        assert_eq!(cx.focused(window).unwrap(), Some(files.tab_id()));
        assert!(cx.contains_element(window, overview.panel_id()).unwrap());
        assert!(!cx.contains_element(window, files.panel_id()).unwrap());

        // A manually focused inactive tab still represents the whole roving tablist for normal
        // Tab traversal, so focus exits into the active panel instead of restarting the window.
        cx.simulate_keystrokes(window, "tab").unwrap();
        assert_eq!(cx.focused(window).unwrap(), Some(overview.panel_id()));
        cx.focus(window, overview.tab_id()).unwrap();
        cx.simulate_keystrokes(window, "end space").unwrap();
        assert_eq!(cx.focused(window).unwrap(), Some(settings.tab_id()));
        assert!(!cx.contains_element(window, overview.panel_id()).unwrap());
        assert!(cx.contains_element(window, settings.panel_id()).unwrap());
        cx.simulate_keystrokes(window, "home enter left").unwrap();
        assert_eq!(cx.focused(window).unwrap(), Some(settings.tab_id()));
        assert!(cx.contains_element(window, overview.panel_id()).unwrap());
        assert!(!cx.contains_element(window, settings.panel_id()).unwrap());
        assert!(cx.click(window, disabled.tab_id()).is_err());

        cx.focus(window, alpha.tab_id()).unwrap();
        cx.simulate_keystrokes(window, "down").unwrap();
        assert_eq!(cx.focused(window).unwrap(), Some(gamma.tab_id()));
        assert!(!cx.contains_element(window, alpha.panel_id()).unwrap());
        assert!(cx.contains_element(window, gamma.panel_id()).unwrap());
        cx.simulate_keystrokes(window, "right").unwrap();
        assert_eq!(cx.focused(window).unwrap(), Some(gamma.tab_id()));
        cx.simulate_keystrokes(window, "end").unwrap();
        assert_eq!(cx.focused(window).unwrap(), Some(delta.tab_id()));
        assert!(cx.contains_element(window, delta.panel_id()).unwrap());
        cx.simulate_keystrokes(window, "down").unwrap();
        assert_eq!(cx.focused(window).unwrap(), Some(alpha.tab_id()));
        assert!(cx.contains_element(window, alpha.panel_id()).unwrap());

        let update = cx.accessibility_update(window).unwrap();
        let node = |id: ElementId| {
            update
                .nodes
                .iter()
                .find_map(|(node_id, node)| (node_id.0 == id.as_u64()).then_some(node))
                .expect("tab accessibility node")
        };
        assert_eq!(node(manual.list_id()).role(), accesskit::Role::TabList);
        assert_eq!(
            node(manual.list_id()).orientation(),
            Some(accesskit::Orientation::Horizontal)
        );
        assert_eq!(node(overview.tab_id()).role(), accesskit::Role::Tab);
        assert_eq!(node(overview.tab_id()).is_selected(), Some(true));
        assert_eq!(node(files.tab_id()).is_selected(), Some(false));
        assert_eq!(
            node(overview.tab_id()).controls(),
            &[accesskit::NodeId(overview.panel_id().as_u64())]
        );
        assert!(node(files.tab_id()).controls().is_empty());
        assert!(node(disabled.tab_id()).is_disabled());
        assert!(!node(disabled.tab_id()).supports_action(accesskit::Action::Click));
        assert_eq!(node(overview.panel_id()).role(), accesskit::Role::TabPanel);
        assert_eq!(
            node(overview.panel_id()).labelled_by(),
            &[accesskit::NodeId(overview.tab_id().as_u64())]
        );
        assert_eq!(
            node(automatic.list_id()).orientation(),
            Some(accesskit::Orientation::Vertical)
        );
        assert_eq!(node(alpha.tab_id()).is_selected(), Some(true));
        assert_eq!(node(gamma.tab_id()).is_selected(), Some(false));
        assert!(node(beta.tab_id()).is_disabled());

        let renders = cx.render_count(window).unwrap();
        cx.run_until_idle().unwrap();
        assert_eq!(cx.render_count(window).unwrap(), renders);
    }

    struct NonLoopingTabsView {
        state: TabsState,
    }

    impl Default for NonLoopingTabsView {
        fn default() -> Self {
            Self {
                state: TabsState::new("first"),
            }
        }
    }

    impl View for NonLoopingTabsView {
        fn render(&mut self, cx: &mut ViewContext<'_, Self>) -> impl IntoElement {
            let tabs = Tabs::from_state("non-looping-tabs", &self.state)
                .activate_on_focus(true)
                .loop_focus(false);
            let first = tabs.tab("first");
            let unavailable = tabs.tab("unavailable").disabled(true);
            let last = tabs.tab("last");
            let select_first = cx.listener(first.tab_id(), |view, cx| {
                if view.state.select("first") {
                    cx.invalidate();
                }
            });
            let select_last = cx.listener(last.tab_id(), |view, cx| {
                if view.state.select("last") {
                    cx.invalidate();
                }
            });

            tabs.root_part(
                div()
                    .child(
                        tabs.list_part(
                            div()
                                .child(first.tab_part(div().child("First")).on_click(select_first))
                                .child(unavailable.tab_part(div().child("Unavailable")))
                                .child(last.tab_part(div().child("Last")).on_click(select_last)),
                        ),
                    )
                    .children(first.panel_part(text("First panel")))
                    .children(unavailable.panel_part(text("Unavailable panel")))
                    .children(last.panel_part(text("Last panel"))),
            )
        }
    }

    #[test]
    fn non_looping_navigation_stops_at_each_enabled_edge() {
        let (mut cx, view) = TestAppContext::new(NonLoopingTabsView::default()).unwrap();
        let window = view.window_handle();
        let tabs = Tabs::new("non-looping-tabs", "first")
            .activate_on_focus(true)
            .loop_focus(false);
        let first = tabs.tab("first");
        let unavailable = tabs.tab("unavailable").disabled(true);
        let last = tabs.tab("last");

        cx.focus(window, first.tab_id()).unwrap();
        cx.simulate_keystrokes(window, "left").unwrap();
        assert_eq!(cx.focused(window).unwrap(), Some(first.tab_id()));
        assert!(cx.contains_element(window, first.panel_id()).unwrap());

        cx.simulate_keystrokes(window, "right").unwrap();
        assert_eq!(cx.focused(window).unwrap(), Some(last.tab_id()));
        assert!(!cx.contains_element(window, first.panel_id()).unwrap());
        assert!(cx.contains_element(window, last.panel_id()).unwrap());
        assert!(cx.click(window, unavailable.tab_id()).is_err());

        cx.simulate_keystrokes(window, "right").unwrap();
        assert_eq!(cx.focused(window).unwrap(), Some(last.tab_id()));
        assert!(cx.contains_element(window, last.panel_id()).unwrap());

        cx.simulate_keystrokes(window, "home").unwrap();
        assert_eq!(cx.focused(window).unwrap(), Some(first.tab_id()));
        assert!(cx.contains_element(window, first.panel_id()).unwrap());

        let renders = cx.render_count(window).unwrap();
        cx.run_until_idle().unwrap();
        assert_eq!(cx.render_count(window).unwrap(), renders);
    }
}
