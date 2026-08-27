use crate::{AccessibilityRole, Element, ToggleState, div};

/// Copyable declaration for one controlled, unstyled checkbox.
///
/// The application owns the checked value, listener, layout, indicator, label, colors, and motion.
/// QuickGUI supplies the checkbox role, exact on/off/mixed state, focus/click behavior, native
/// window-drag exclusion, desktop arrow cursor, and an accessibility-hidden indicator part.
///
/// The descriptor retains no allocation, task, timer, observer, or idle scheduler source.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[must_use = "a Checkbox descriptor has no effect until one of its parts is mounted"]
pub struct Checkbox {
    state: ToggleState,
}

impl Checkbox {
    pub fn new(state: impl Into<ToggleState>) -> Self {
        Self {
            state: state.into(),
        }
    }

    pub const fn state(self) -> ToggleState {
        self.state
    }

    /// Decorate an application-owned root without adding layout or appearance.
    pub fn root_part(self, root: Element) -> Element {
        selection_root(root, AccessibilityRole::CheckBox, self.state)
    }

    /// Hide an application-owned visual indicator from the accessible name.
    pub fn indicator_part(self, indicator: Element) -> Element {
        indicator.accessibility_hidden(true)
    }
}

/// Copyable declaration for one controlled, unstyled radio button.
///
/// Put related roots inside [`RadioGroup::root_part`]. QuickGUI supplies roving Tab/arrow
/// behavior from the mounted semantic tree; the application owns every visual declaration.
/// The descriptor retains no allocation, task, timer, observer, or idle scheduler source.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[must_use = "a Radio descriptor has no effect until one of its parts is mounted"]
pub struct Radio {
    selected: bool,
}

impl Radio {
    pub const fn new(selected: bool) -> Self {
        Self { selected }
    }

    pub const fn is_selected(self) -> bool {
        self.selected
    }

    /// Decorate an application-owned root without adding layout or appearance.
    pub fn root_part(self, root: Element) -> Element {
        selection_root(
            root,
            AccessibilityRole::RadioButton,
            ToggleState::from(self.selected),
        )
    }

    /// Hide an application-owned visual indicator from the accessible name.
    pub fn indicator_part(self, indicator: Element) -> Element {
        indicator.accessibility_hidden(true)
    }
}

/// Copyable declaration for one semantic radio group.
///
/// The group adds only the accessibility relationship used by Tab and arrow-key navigation. It
/// retains no item registry, allocation, task, timer, observer, or idle scheduler source.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
#[must_use = "a RadioGroup descriptor has no effect until its root part is mounted"]
pub struct RadioGroup;

impl RadioGroup {
    pub const fn new() -> Self {
        Self
    }

    /// Decorate an application-owned group root without adding layout or appearance.
    pub fn root_part(self, root: Element) -> Element {
        root.accessibility_role(AccessibilityRole::RadioGroup)
    }
}

/// Copyable declaration for one controlled, unstyled switch.
///
/// The application owns the checked value, listener, track/root presentation, thumb presentation,
/// label, and motion. QuickGUI supplies the switch role and ordinary control interaction contract.
/// The descriptor retains no allocation, task, timer, observer, or idle scheduler source.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[must_use = "a Switch descriptor has no effect until one of its parts is mounted"]
pub struct Switch {
    checked: bool,
}

impl Switch {
    pub const fn new(checked: bool) -> Self {
        Self { checked }
    }

    pub const fn is_checked(self) -> bool {
        self.checked
    }

    /// Decorate an application-owned root/track without adding layout or appearance.
    pub fn root_part(self, root: Element) -> Element {
        selection_root(
            root,
            AccessibilityRole::Switch,
            ToggleState::from(self.checked),
        )
    }

    /// Hide an application-owned visual thumb from the accessible name.
    pub fn thumb_part(self, thumb: Element) -> Element {
        thumb.accessibility_hidden(true)
    }
}

/// Create an unstyled controlled checkbox root.
///
/// This shorthand is equivalent to `Checkbox::new(state).root_part(div())`. Use [`Checkbox`]
/// directly when composing a separate indicator part.
pub fn checkbox(state: impl Into<ToggleState>) -> Element {
    Checkbox::new(state).root_part(div())
}

/// Create an unstyled controlled radio root.
///
/// This shorthand is equivalent to `Radio::new(selected).root_part(div())`.
pub fn radio(selected: bool) -> Element {
    Radio::new(selected).root_part(div())
}

/// Create an unstyled semantic radio-group root.
///
/// This shorthand is equivalent to `RadioGroup::new().root_part(div())`.
pub fn radio_group() -> Element {
    RadioGroup::new().root_part(div())
}

/// Create an unstyled controlled switch root.
///
/// This shorthand is equivalent to `Switch::new(checked).root_part(div())`. Use [`Switch`]
/// directly when composing a separate thumb part.
pub fn switch(checked: bool) -> Element {
    Switch::new(checked).root_part(div())
}

fn selection_root(root: Element, role: AccessibilityRole, state: ToggleState) -> Element {
    root.accessibility_role(role)
        .toggle_state(state)
        .clickable()
        .cursor_default()
        .app_region_no_drag()
        .user_select_none()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        AppRegion, Color, CursorStyle, EventContext, IntoElement, TestAppContext, UserSelect, View,
        ViewContext, text,
    };

    #[test]
    fn parts_add_exact_behavior_without_layout_or_appearance() {
        let checkbox = Checkbox::new(ToggleState::Mixed);
        assert_eq!(checkbox.state(), ToggleState::Mixed);
        let checkbox_root = checkbox.root_part(
            div()
                .w(137.0)
                .bg(Color::rgb8(4, 5, 6))
                .border(3.0, Color::rgb8(7, 8, 9))
                .child("Visible label"),
        );
        assert_eq!(
            checkbox_root.accessibility.role,
            AccessibilityRole::CheckBox
        );
        assert_eq!(
            checkbox_root.accessibility.toggled,
            Some(ToggleState::Mixed)
        );
        assert!(checkbox_root.clickable);
        assert!(checkbox_root.focusable);
        assert_eq!(checkbox_root.cursor_style, Some(CursorStyle::Arrow));
        assert!(checkbox_root.cursor_style_explicit);
        assert_eq!(checkbox_root.app_region, Some(AppRegion::NoDrag));
        assert_eq!(checkbox_root.user_select, UserSelect::None);
        assert_eq!(checkbox_root.visual.background, Some(Color::rgb8(4, 5, 6)));
        assert_eq!(
            checkbox_root.visual.border_color,
            Some(Color::rgb8(7, 8, 9))
        );
        assert_eq!(checkbox_root.visual.border_width, 3.0);
        assert_eq!(checkbox_root.children.len(), 1);
        assert!(checkbox_root.transition.is_none());

        let indicator = checkbox.indicator_part(
            div()
                .size(19.0, 17.0)
                .bg(Color::rgb8(10, 11, 12))
                .child("decorative check"),
        );
        assert!(indicator.accessibility.hidden);
        assert_eq!(indicator.visual.background, Some(Color::rgb8(10, 11, 12)));
        assert_eq!(indicator.children.len(), 1);

        let radio = Radio::new(true);
        assert!(radio.is_selected());
        let radio_root = radio.root_part(div());
        assert_eq!(
            radio_root.accessibility.role,
            AccessibilityRole::RadioButton
        );
        assert_eq!(radio_root.accessibility.toggled, Some(ToggleState::On));
        assert!(radio.indicator_part(div()).accessibility.hidden);

        let switch = Switch::new(false);
        assert!(!switch.is_checked());
        let switch_root = switch.root_part(div());
        assert_eq!(switch_root.accessibility.role, AccessibilityRole::Switch);
        assert_eq!(switch_root.accessibility.toggled, Some(ToggleState::Off));
        assert!(switch.thumb_part(div()).accessibility.hidden);

        let group = RadioGroup::new().root_part(
            div()
                .bg(Color::rgb8(13, 14, 15))
                .child("Application-owned group"),
        );
        assert_eq!(group.accessibility.role, AccessibilityRole::RadioGroup);
        assert!(!group.clickable);
        assert!(!group.focusable);
        assert_eq!(group.visual.background, Some(Color::rgb8(13, 14, 15)));
        assert_eq!(group.children.len(), 1);
    }

    #[test]
    fn shorthands_are_semantic_empty_unstyled_roots() {
        let checked = checkbox(true);
        assert_eq!(checked.accessibility.role, AccessibilityRole::CheckBox);
        assert_eq!(checked.accessibility.toggled, Some(ToggleState::On));
        assert!(checked.children.is_empty());
        assert_eq!(checked.visual.background, None);
        assert_eq!(checked.visual.border_color, None);

        let mixed = checkbox(ToggleState::Mixed);
        assert_eq!(mixed.accessibility.toggled, Some(ToggleState::Mixed));

        let radio = radio(false);
        assert_eq!(radio.accessibility.role, AccessibilityRole::RadioButton);
        assert_eq!(radio.accessibility.toggled, Some(ToggleState::Off));

        let switch = switch(true);
        assert_eq!(switch.accessibility.role, AccessibilityRole::Switch);
        assert_eq!(switch.accessibility.toggled, Some(ToggleState::On));
        assert_eq!(
            radio_group().accessibility.role,
            AccessibilityRole::RadioGroup
        );
    }

    #[derive(Default)]
    struct ControlView {
        checked: bool,
        radio: usize,
        switched: bool,
    }

    impl View for ControlView {
        fn render(&mut self, cx: &mut ViewContext<'_, Self>) -> impl IntoElement {
            let check = cx.listener("check", |view, cx: &mut EventContext| {
                view.checked = !view.checked;
                cx.invalidate();
            });
            let radio_one = cx.listener("radio-one", |view, cx| {
                view.radio = 1;
                cx.invalidate();
            });
            let radio_zero = cx.listener("radio-zero", |view, cx| {
                view.radio = 0;
                cx.invalidate();
            });
            let toggle = cx.listener("switch", |view, cx| {
                view.switched = !view.switched;
                cx.invalidate();
            });

            let checkbox_control = Checkbox::new(self.checked);
            let first_radio = Radio::new(self.radio == 0);
            let second_radio = Radio::new(self.radio == 1);
            let switch = Switch::new(self.switched);

            div().children([
                checkbox_control
                    .root_part(
                        div().child(checkbox_control.indicator_part(text("decorative mark"))),
                    )
                    .id("check")
                    .on_click(check)
                    .child("Checkbox"),
                RadioGroup::new().root_part(
                    div().children([
                        first_radio
                            .root_part(div().child(first_radio.indicator_part(div())))
                            .id("radio-zero")
                            .on_click(radio_zero)
                            .child("First radio"),
                        second_radio
                            .root_part(div().child(second_radio.indicator_part(div())))
                            .id("radio-one")
                            .on_click(radio_one)
                            .child("Second radio"),
                        radio(false)
                            .id("radio-disabled")
                            .disabled(true)
                            .child("Disabled radio"),
                    ]),
                ),
                switch
                    .root_part(div().child(switch.thumb_part(div())))
                    .id("switch")
                    .on_click(toggle)
                    .child("Switch"),
                checkbox(false)
                    .id("disabled-check")
                    .disabled(true)
                    .child("Disabled"),
            ])
        }
    }

    #[test]
    fn parts_use_existing_click_focus_radio_and_keyboard_paths_without_idle_work() {
        let (mut cx, view) = TestAppContext::new(ControlView::default()).unwrap();
        let window = view.window_handle();

        cx.click(window, "check").unwrap();
        assert!(cx.read(view, |view| view.checked).unwrap());
        cx.simulate_keystrokes(window, "space").unwrap();
        assert!(!cx.read(view, |view| view.checked).unwrap());

        cx.simulate_keystrokes(window, "tab").unwrap();
        assert_eq!(cx.focused(window).unwrap(), Some("radio-zero".into()));
        cx.simulate_keystrokes(window, "right").unwrap();
        assert_eq!(cx.focused(window).unwrap(), Some("radio-one".into()));
        assert_eq!(cx.read(view, |view| view.radio).unwrap(), 1);
        cx.simulate_keystrokes(window, "right").unwrap();
        assert_eq!(cx.focused(window).unwrap(), Some("radio-zero".into()));
        assert_eq!(cx.read(view, |view| view.radio).unwrap(), 0);
        cx.simulate_keystrokes(window, "left").unwrap();
        assert_eq!(cx.focused(window).unwrap(), Some("radio-one".into()));
        assert_eq!(cx.read(view, |view| view.radio).unwrap(), 1);

        cx.simulate_keystrokes(window, "tab").unwrap();
        assert_eq!(cx.focused(window).unwrap(), Some("switch".into()));
        cx.click(window, "switch").unwrap();
        assert!(cx.read(view, |view| view.switched).unwrap());
        assert!(cx.click(window, "disabled-check").is_err());

        let renders = cx.render_count(window).unwrap();
        cx.run_until_idle().unwrap();
        assert_eq!(cx.render_count(window).unwrap(), renders);
    }
}
