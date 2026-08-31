use crate::{
    AccessibilityPopover, AccessibilityRole, Element, ElementId, EventContext, FocusHandle,
};

const DIALOG_ROOT_ID_TAG: u64 = 0x6405_1ba9_158d_f8fd;
const DIALOG_BACKDROP_ID_TAG: u64 = 0xfbf2_90af_5f5b_982d;
const DIALOG_POPOVER_ID_TAG: u64 = 0x21e9_d20f_8574_4dd8;
const DIALOG_TITLE_ID_TAG: u64 = 0xb50c_ee8e_b0c0_8ed7;
const DIALOG_DESCRIPTION_ID_TAG: u64 = 0x86a8_0ac0_5627_538e;
const DIALOG_CLOSE_ID_TAG: u64 = 0xe5d1_e6af_b19b_c827;

/// Native accessibility behavior for one in-window modal surface.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum DialogKind {
    /// A modal task or editing surface which may close from its backdrop by default.
    #[default]
    Dialog,
    /// A consequential confirmation surface whose backdrop does not dismiss it by default.
    AlertDialog,
}

impl DialogKind {
    const fn accessibility_role(self) -> AccessibilityRole {
        match self {
            Self::Dialog => AccessibilityRole::Dialog,
            Self::AlertDialog => AccessibilityRole::AlertDialog,
        }
    }

    const fn dismiss_on_backdrop(self) -> bool {
        matches!(self, Self::Dialog)
    }
}

/// Copyable declaration for one controlled, unstyled in-window dialog.
///
/// The application owns the `open` value, all visual declarations, and the listener that changes
/// that value. QuickGUI supplies stable part identities, a viewport overlay, nested topmost focus
/// containment, independent Escape/backdrop dismissal, focus restoration, app-drag exclusion,
/// native-view occlusion through the overlay plane, and exact dialog accessibility semantics.
/// Mount [`Self::root_part`] only while [`Self::is_open`] is true.
///
/// The descriptor retains no task, timer, observer, registry entry, or idle scheduler source.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Dialog {
    id: ElementId,
    open: bool,
    kind: DialogKind,
    initial_focus: Option<FocusHandle>,
    restore_focus: Option<FocusHandle>,
    dismiss_on_escape: bool,
    dismiss_on_backdrop: bool,
}

impl Dialog {
    /// Declare an ordinary modal dialog.
    pub fn new(id: impl Into<ElementId>, open: bool) -> Self {
        Self::with_kind(id, open, DialogKind::Dialog)
    }

    /// Declare a consequential alert dialog.
    ///
    /// Escape remains enabled, while a backdrop press is blocked without dismissing by default.
    pub fn alert(id: impl Into<ElementId>, open: bool) -> Self {
        Self::with_kind(id, open, DialogKind::AlertDialog)
    }

    pub fn with_kind(id: impl Into<ElementId>, open: bool, kind: DialogKind) -> Self {
        Self {
            id: id.into(),
            open,
            kind,
            initial_focus: None,
            restore_focus: None,
            dismiss_on_escape: true,
            dismiss_on_backdrop: kind.dismiss_on_backdrop(),
        }
    }

    pub const fn is_open(self) -> bool {
        self.open
    }

    pub const fn kind(self) -> DialogKind {
        self.kind
    }

    /// Prefer one mounted descendant when this dialog opens.
    ///
    /// Without an explicit target, the focus trap chooses its first enabled Tab stop and falls
    /// back to the popover root when no interactive descendant exists.
    pub fn initial_focus(mut self, focus: impl Into<ElementId>) -> Self {
        self.initial_focus = Some(FocusHandle::new(focus));
        self
    }

    /// Restore focus to one stable control after every dismissal path.
    pub fn restore_focus_to(mut self, focus: impl Into<ElementId>) -> Self {
        self.restore_focus = Some(FocusHandle::new(focus));
        self
    }

    pub const fn dismiss_on_escape(mut self, dismiss: bool) -> Self {
        self.dismiss_on_escape = dismiss;
        self
    }

    pub const fn dismiss_on_backdrop(mut self, dismiss: bool) -> Self {
        self.dismiss_on_backdrop = dismiss;
        self
    }

    pub fn root_id(self) -> ElementId {
        derived_dialog_id(self.id, DIALOG_ROOT_ID_TAG)
    }

    pub fn backdrop_id(self) -> ElementId {
        derived_dialog_id(self.id, DIALOG_BACKDROP_ID_TAG)
    }

    pub fn popover_id(self) -> ElementId {
        derived_dialog_id(self.id, DIALOG_POPOVER_ID_TAG)
    }

    pub fn title_id(self) -> ElementId {
        derived_dialog_id(self.id, DIALOG_TITLE_ID_TAG)
    }

    pub fn description_id(self) -> ElementId {
        derived_dialog_id(self.id, DIALOG_DESCRIPTION_ID_TAG)
    }

    pub fn close_id(self) -> ElementId {
        derived_dialog_id(self.id, DIALOG_CLOSE_ID_TAG)
    }

    pub fn popover_focus(self) -> FocusHandle {
        FocusHandle::new(self.popover_id())
    }

    /// Request the declared initial focus in the same event that mounts the dialog.
    pub fn focus_initial(self, cx: &mut EventContext) {
        cx.focus(self.initial_focus.unwrap_or_else(|| self.popover_focus()));
    }

    /// Restore the declared focus after an explicit close action.
    ///
    /// Escape and backdrop dismissal already use the same retained target automatically.
    pub fn focus_restore(self, cx: &mut EventContext) {
        if let Some(focus) = self.restore_focus {
            cx.focus(focus);
        }
    }

    /// Decorate an application-owned trigger without adding appearance.
    pub fn trigger_part(self, id: impl Into<ElementId>, trigger: Element) -> Element {
        let trigger = trigger
            .id(id)
            .focusable()
            .accessibility_role(AccessibilityRole::Button)
            .accessibility_has_popover(AccessibilityPopover::Dialog)
            .accessibility_expanded(self.open)
            .app_region_no_drag()
            .user_select_none();
        if self.open {
            trigger.accessibility_controls(self.popover_id())
        } else {
            trigger
        }
    }

    /// Decorate the full-window portal and active modal focus boundary.
    pub fn root_part(self, root: Element) -> Element {
        root.id(self.root_id())
            .overlay()
            .inset_0()
            .size_full()
            .focus_trap()
            .restore_previous_focus()
            .app_region_no_drag()
            .cursor_default()
    }

    /// Decorate the caller-owned visual backdrop.
    pub fn backdrop_part(self, backdrop: Element) -> Element {
        backdrop
            .id(self.backdrop_id())
            .absolute()
            .inset_0()
            .size_full()
            .app_region_no_drag()
            .cursor_default()
    }

    /// Decorate the caller-owned modal popover.
    ///
    /// The popover is a negative-Tab-index focus fallback. Enabled descendant Tab stops are chosen
    /// first when the enclosing trap mounts. Title and description relationships project only
    /// when the corresponding parts are mounted, so incomplete compositions never emit dangling
    /// native node references.
    pub fn popover_part(self, popover: Element) -> Element {
        let mut popover = popover
            .id(self.popover_id())
            .track_focus(self.popover_focus())
            .tab_index(-1)
            .accessibility_role(self.kind.accessibility_role())
            .accessibility_modal(true)
            .accessibility_labelled_by(self.title_id())
            .accessibility_described_by(self.description_id())
            .app_region_no_drag()
            .cursor_default();
        if self.dismiss_on_escape {
            popover = popover.dismiss_on_escape();
        }
        if self.dismiss_on_backdrop {
            popover = popover.dismiss_on_pointer_outside();
        }
        if let Some(focus) = self.restore_focus {
            popover = popover.restore_focus_to(focus);
        }
        popover
    }

    /// Assign the stable visible label target used by the popover.
    pub fn title_part(self, title: Element) -> Element {
        title.id(self.title_id())
    }

    /// Assign the stable visible description target used by the popover.
    pub fn description_part(self, description: Element) -> Element {
        description.id(self.description_id())
    }

    /// Decorate a caller-owned close control with button behavior and no visual defaults.
    pub fn close_part(self, label: impl Into<std::sync::Arc<str>>, close: Element) -> Element {
        close
            .id(self.close_id())
            .clickable()
            .accessibility_role(AccessibilityRole::Button)
            .accessibility_label(label)
            .app_region_no_drag()
            .user_select_none()
    }
}

fn derived_dialog_id(parent: ElementId, tag: u64) -> ElementId {
    let mut hash = parent.as_u64() ^ tag;
    hash ^= hash >> 30;
    hash = hash.wrapping_mul(0xbf58_476d_1ce4_e5b9);
    hash ^= hash >> 27;
    hash = hash.wrapping_mul(0x94d0_49bb_1331_11eb);
    hash ^= hash >> 31;
    if hash == 0 || hash == parent.as_u64() || hash == u64::MAX {
        hash ^= tag.rotate_left(17);
    }
    ElementId::new(hash)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Color, IntoElement, TestAppContext, View, ViewContext, button, div, text};

    #[test]
    fn parts_add_exact_behavior_without_appearance() {
        let dialog = Dialog::new("settings", true).restore_focus_to("open-settings");
        let trigger = dialog.trigger_part("open-settings", div());
        assert_eq!(trigger.accessibility.role, AccessibilityRole::Button);
        assert_eq!(
            trigger.accessibility.has_popover,
            Some(AccessibilityPopover::Dialog)
        );
        assert_eq!(trigger.accessibility.expanded, Some(true));
        assert_eq!(
            trigger.accessibility.relations.controls(),
            Some(dialog.popover_id())
        );

        let root = dialog.root_part(div());
        assert!(root.focus_trap);
        assert!(root.restore_previous_focus);
        assert!(root.portal);
        assert!(root.blocks_pointer);
        assert_eq!(root.visual.background, None);
        assert_eq!(root.visual.border_color, None);

        let popover = dialog.popover_part(div());
        assert_eq!(popover.accessibility.role, AccessibilityRole::Dialog);
        assert!(popover.accessibility.modal);
        assert_eq!(
            popover.accessibility.relations.labelled_by(),
            Some(dialog.title_id())
        );
        assert_eq!(
            popover.accessibility.relations.described_by(),
            Some(dialog.description_id())
        );
        assert!(popover.dismiss_policy.on_escape());
        assert!(popover.dismiss_policy.on_pointer_outside());
        assert_eq!(popover.visual.background, None);
        assert_eq!(popover.visual.border_color, None);

        let alert = Dialog::alert("delete", true).popover_part(div());
        assert_eq!(alert.accessibility.role, AccessibilityRole::AlertDialog);
        assert!(alert.dismiss_policy.on_escape());
        assert!(!alert.dismiss_policy.on_pointer_outside());
    }

    #[test]
    fn derived_part_ids_are_stable_distinct_and_never_reuse_the_base() {
        let dialog = Dialog::new(0_u64, true);
        let ids = [
            dialog.root_id(),
            dialog.backdrop_id(),
            dialog.popover_id(),
            dialog.title_id(),
            dialog.description_id(),
            dialog.close_id(),
        ];
        for (index, id) in ids.iter().enumerate() {
            assert_ne!(*id, ElementId::new(0));
            assert_ne!(*id, ElementId::new(u64::MAX));
            assert!(!ids[..index].contains(id));
        }
        assert_eq!(dialog.popover_id(), Dialog::new(0_u64, false).popover_id());
    }

    #[derive(Default)]
    struct DialogView {
        open: bool,
        closed: usize,
    }

    impl View for DialogView {
        fn render(&mut self, cx: &mut ViewContext<'_, Self>) -> impl IntoElement {
            let dialog = Dialog::new("test-dialog", self.open)
                .initial_focus("dialog-first")
                .restore_focus_to("dialog-trigger");
            let open = cx.listener("dialog-trigger", move |view, cx| {
                view.open = true;
                dialog.focus_initial(cx);
                cx.invalidate();
            });
            let close = cx.listener(dialog.close_id(), move |view, cx| {
                view.open = false;
                view.closed += 1;
                dialog.focus_restore(cx);
                cx.invalidate();
            });
            let dismiss = cx.dismiss_listener(dialog.popover_id(), move |view, cx| {
                view.open = false;
                view.closed += 1;
                cx.invalidate();
            });

            let mut root = div()
                .size_full()
                .relative()
                .child(
                    dialog
                        .trigger_part("dialog-trigger", button().child("Open"))
                        .on_click(open),
                )
                .child(button().id("outside").child("Outside"));
            if self.open {
                let popover = dialog
                    .popover_part(
                        div()
                            .w(240.0)
                            .h(160.0)
                            .bg(Color::BLACK)
                            .child(dialog.title_part(text("Settings")))
                            .child(dialog.description_part(text("Change settings")))
                            .child(button().id("dialog-first").child("First"))
                            .child(button().id("dialog-second").child("Second"))
                            .child(dialog.close_part("Close settings", div()).on_click(close)),
                    )
                    .on_dismiss(dismiss);
                root = root.child(
                    dialog
                        .root_part(div().flex_row().items_center().justify_center())
                        .child(dialog.backdrop_part(div()))
                        .child(popover),
                );
            }
            root
        }
    }

    #[test]
    fn controlled_dialog_traps_tabs_restores_focus_and_sleeps() {
        let (mut cx, view) = TestAppContext::new(DialogView::default()).unwrap();
        let window = view.window_handle();
        cx.click(window, "dialog-trigger").unwrap();
        assert_eq!(cx.focused(window).unwrap(), Some("dialog-first".into()));

        cx.simulate_keystrokes(window, "tab").unwrap();
        assert_eq!(cx.focused(window).unwrap(), Some("dialog-second".into()));
        cx.simulate_keystrokes(window, "tab").unwrap();
        assert_eq!(
            cx.focused(window).unwrap(),
            Some(Dialog::new("test-dialog", true).close_id())
        );
        cx.simulate_keystrokes(window, "tab").unwrap();
        assert_eq!(cx.focused(window).unwrap(), Some("dialog-first".into()));
        assert!(cx.focus(window, "outside").is_err());

        cx.simulate_keystrokes(window, "escape").unwrap();
        assert!(!cx.read(view, |view| view.open).unwrap());
        assert_eq!(cx.read(view, |view| view.closed).unwrap(), 1);
        assert_eq!(cx.focused(window).unwrap(), Some("dialog-trigger".into()));

        let renders = cx.render_count(window).unwrap();
        cx.run_until_idle().unwrap();
        assert_eq!(cx.render_count(window).unwrap(), renders);
    }
}
