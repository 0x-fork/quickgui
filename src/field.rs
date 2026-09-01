use std::sync::Arc;

use crate::{AccessibilityRole, Element, ElementId, MAX_VALIDATION_MESSAGE_BYTES};

const FIELD_ROOT_ID_TAG: u64 = 0x6669_656c_645f_726f;
const FIELD_LABEL_ID_TAG: u64 = 0x6669_656c_645f_6c61;
const FIELD_DESCRIPTION_ID_TAG: u64 = 0x6669_656c_645f_6465;
const FIELD_ERROR_ID_TAG: u64 = 0x6669_656c_645f_6572;
const FIELDSET_LEGEND_ID_TAG: u64 = 0x6669_656c_6473_6c67;
const FIELDSET_DESCRIPTION_ID_TAG: u64 = 0x6669_656c_6473_6465;

/// Caller-owned state projected consistently across every unstyled field part.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct FieldState {
    pub disabled: bool,
    pub invalid: bool,
    pub required: bool,
    pub touched: bool,
    pub dirty: bool,
    pub filled: bool,
}

impl FieldState {
    pub const fn is_valid(self) -> bool {
        !self.invalid
    }
}

/// Controlled, unstyled labeling and validation composition for one form control.
///
/// The application owns the value, validation rule, every rendered part, layout, typography,
/// colors, borders, focus treatment, and motion. QuickGUI supplies stable part identities,
/// click-to-activate label behavior, exact accessibility relationships, required/invalid/disabled
/// control state, and one bounded native validation message.
///
/// This descriptor retains no task, timer, observer, item registry, or idle scheduler source.
#[derive(Clone, Debug, Eq, PartialEq)]
#[must_use = "a Field descriptor has no effect until its parts are mounted"]
pub struct Field {
    control_id: ElementId,
    state: FieldState,
    validation_message: Option<Arc<str>>,
    validation_message_truncated: bool,
}

impl Field {
    pub fn new(control_id: impl Into<ElementId>) -> Self {
        Self {
            control_id: control_id.into(),
            state: FieldState::default(),
            validation_message: None,
            validation_message_truncated: false,
        }
    }

    pub const fn control_id(&self) -> ElementId {
        self.control_id
    }

    pub fn root_id(&self) -> ElementId {
        derived_field_id(self.control_id, FIELD_ROOT_ID_TAG)
    }

    pub fn label_id(&self) -> ElementId {
        derived_field_id(self.control_id, FIELD_LABEL_ID_TAG)
    }

    pub fn description_id(&self) -> ElementId {
        derived_field_id(self.control_id, FIELD_DESCRIPTION_ID_TAG)
    }

    pub fn error_id(&self) -> ElementId {
        derived_field_id(self.control_id, FIELD_ERROR_ID_TAG)
    }

    pub const fn state(&self) -> FieldState {
        self.state
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.state.disabled = disabled;
        self
    }

    pub fn invalid(mut self, invalid: bool) -> Self {
        self.state.invalid = invalid;
        self
    }

    pub fn required(mut self, required: bool) -> Self {
        self.state.required = required;
        self
    }

    pub fn touched(mut self, touched: bool) -> Self {
        self.state.touched = touched;
        self
    }

    pub fn dirty(mut self, dirty: bool) -> Self {
        self.state.dirty = dirty;
        self
    }

    pub fn filled(mut self, filled: bool) -> Self {
        self.state.filled = filled;
        self
    }

    /// Retain the message used by form reports and native accessibility.
    ///
    /// The visible error part remains application-owned and can use different presentation copy.
    pub fn validation_message(mut self, message: impl Into<Arc<str>>) -> Self {
        let (message, truncated) = bounded_validation_message(message.into());
        self.validation_message = message;
        self.validation_message_truncated = truncated;
        self
    }

    pub fn validation_message_text(&self) -> Option<&str> {
        self.validation_message.as_deref()
    }

    pub const fn validation_message_is_truncated(&self) -> bool {
        self.validation_message_truncated
    }

    /// Decorate an application-owned structural root without adding role or appearance.
    pub fn root_part(&self, root: Element) -> Element {
        root.id(self.root_id())
    }

    /// Decorate a visible label and give it native label-to-control activation.
    ///
    /// Use [`Self::passive_label_part`] for button-like controls whose label should name but not
    /// activate them.
    pub fn label_part(&self, label: Element) -> Element {
        let label = self.passive_label_part(label);
        if self.state.disabled {
            label.disabled(true)
        } else {
            label.activate_target_on_click(self.control_id)
        }
    }

    /// Decorate a label relationship without forwarding pointer activation to the control.
    pub fn passive_label_part(&self, label: Element) -> Element {
        label
            .id(self.label_id())
            .accessibility_role(AccessibilityRole::Label)
            .app_region_no_drag()
    }

    /// Decorate the application-owned control with field state and mounted relationships.
    pub fn control_part(&self, control: Element) -> Element {
        let disabled = control.accessibility.disabled || self.state.disabled;
        let required = control.accessibility.required || self.state.required;
        let mut control = control
            .id(self.control_id)
            .disabled(disabled)
            .required(required)
            .invalid(self.state.invalid)
            .accessibility_labelled_by(self.label_id())
            .app_region_no_drag();
        control = if self.state.invalid {
            control.accessibility_described_by_pair(self.description_id(), self.error_id())
        } else {
            control.accessibility_described_by(self.description_id())
        };
        if let Some(message) = self.validation_message.clone() {
            control =
                control.validation_message_retained(message, self.validation_message_truncated);
        }
        control
    }

    /// Decorate visible supplementary help for the control.
    pub fn description_part(&self, description: Element) -> Element {
        description
            .id(self.description_id())
            .accessibility_role(AccessibilityRole::Label)
    }

    /// Decorate a visible error and remove it from layout while the controlled field is valid.
    pub fn error_part(&self, error: Element) -> Element {
        error
            .id(self.error_id())
            .accessibility_role(AccessibilityRole::Label)
            .when(!self.state.invalid, Element::hidden)
    }
}

/// Controlled, unstyled group semantics for related fields.
///
/// Use [`Self::field`] to propagate disabled state into each nested field without a registry or
/// inherited runtime context. Direct custom controls can use [`Self::control_part`].
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[must_use = "a Fieldset descriptor has no effect until its parts are mounted"]
pub struct Fieldset {
    id: ElementId,
    disabled: bool,
}

impl Fieldset {
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            disabled: false,
        }
    }

    pub const fn id(self) -> ElementId {
        self.id
    }

    pub fn legend_id(self) -> ElementId {
        derived_field_id(self.id, FIELDSET_LEGEND_ID_TAG)
    }

    pub fn description_id(self) -> ElementId {
        derived_field_id(self.id, FIELDSET_DESCRIPTION_ID_TAG)
    }

    pub const fn is_disabled(self) -> bool {
        self.disabled
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Create one nested field with group disabled state already applied.
    pub fn field(self, control_id: impl Into<ElementId>) -> Field {
        Field::new(control_id).disabled(self.disabled)
    }

    /// Decorate an application-owned group root with legend and description relationships.
    pub fn root_part(self, root: Element) -> Element {
        root.id(self.id)
            .accessibility_role(AccessibilityRole::Group)
            .accessibility_labelled_by(self.legend_id())
            .accessibility_described_by(self.description_id())
            .disabled(self.disabled)
    }

    pub fn legend_part(self, legend: Element) -> Element {
        legend
            .id(self.legend_id())
            .accessibility_role(AccessibilityRole::Label)
    }

    pub fn description_part(self, description: Element) -> Element {
        description
            .id(self.description_id())
            .accessibility_role(AccessibilityRole::Label)
    }

    /// Apply fieldset disabled state to an application-owned direct control.
    pub fn control_part(self, control: Element) -> Element {
        if self.disabled {
            control.disabled(true)
        } else {
            control
        }
    }
}

fn derived_field_id(parent: ElementId, tag: u64) -> ElementId {
    let mut hash = parent.as_u64() ^ tag;
    hash ^= hash >> 30;
    hash = hash.wrapping_mul(0xbf58_476d_1ce4_e5b9);
    hash ^= hash >> 27;
    hash = hash.wrapping_mul(0x94d0_49bb_1331_11eb);
    hash ^= hash >> 31;
    if hash == parent.as_u64() || hash == u64::MAX {
        hash ^= tag.rotate_left(17);
    }
    ElementId::new(hash)
}

fn bounded_validation_message(message: Arc<str>) -> (Option<Arc<str>>, bool) {
    if message.is_empty() {
        return (None, false);
    }
    if message.len() <= MAX_VALIDATION_MESSAGE_BYTES {
        return (Some(message), false);
    }
    let mut end = MAX_VALIDATION_MESSAGE_BYTES;
    while !message.is_char_boundary(end) {
        end -= 1;
    }
    (Some(Arc::from(&message[..end])), true)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        Color, Insets, TestAppContext, View, ViewContext, checkbox, div, text, text_input,
    };

    #[test]
    fn parts_preserve_application_appearance_and_wire_exact_state() {
        let field = Field::new("name")
            .required(true)
            .invalid(true)
            .touched(true)
            .dirty(true)
            .filled(false)
            .validation_message("Name is required");
        assert!(!field.state().is_valid());
        let root = field.root_part(div().w(321.0).bg(Color::rgb8(1, 2, 3)));
        assert_eq!(root.explicit_id, Some(field.root_id()));
        assert_eq!(root.visual.background, Some(Color::rgb8(1, 2, 3)));

        let label = field.label_part(text("Name").text_lg());
        assert_eq!(label.explicit_id, Some(field.label_id()));
        assert_eq!(label.accessibility.role, AccessibilityRole::Label);
        assert_eq!(label.activation_target, Some(field.control_id()));
        assert!(label.clickable);
        assert!(!label.focusable);

        let control = field.control_part(text_input("").border(3.0, Color::rgb8(4, 5, 6)));
        assert_eq!(control.explicit_id, Some(field.control_id()));
        assert!(control.accessibility.required);
        assert!(control.accessibility.invalid);
        assert_eq!(
            control.accessibility.validation_message.as_deref(),
            Some("Name is required")
        );
        assert_eq!(
            control.accessibility.relations.labelled_by(),
            Some(field.label_id())
        );
        assert_eq!(
            control.accessibility.relations.described_by(),
            Some(field.description_id())
        );
        assert_eq!(
            control.accessibility.relations.described_by_secondary(),
            Some(field.error_id())
        );
        assert_eq!(control.visual.border_widths, Insets::all(3.0));

        assert!(
            !field
                .description_part(text("Public profile"))
                .is_display_none()
        );
        assert!(!field.error_part(text("Required")).is_display_none());
        assert!(
            Field::new("valid")
                .error_part(text("Not mounted"))
                .is_display_none()
        );
    }

    #[test]
    fn messages_are_utf8_bounded_and_part_ids_are_stable_and_distinct() {
        let message = "é".repeat(MAX_VALIDATION_MESSAGE_BYTES);
        let field = Field::new(0_u64).validation_message(message);
        assert!(field.validation_message_is_truncated());
        let retained = field.validation_message_text().unwrap();
        assert!(retained.len() <= MAX_VALIDATION_MESSAGE_BYTES);
        assert!(std::str::from_utf8(retained.as_bytes()).is_ok());

        let ids = [
            field.control_id(),
            field.root_id(),
            field.label_id(),
            field.description_id(),
            field.error_id(),
        ];
        for (index, id) in ids.iter().enumerate() {
            assert_ne!(id.as_u64(), u64::MAX);
            assert!(!ids[..index].contains(id));
        }
    }

    #[derive(Default)]
    struct FieldView {
        value: Arc<str>,
        checked: bool,
    }

    impl View for FieldView {
        fn render(&mut self, cx: &mut ViewContext<'_, Self>) -> impl crate::IntoElement {
            let edit = cx.input_listener("name", |this, value, cx| {
                this.value = Arc::from(value);
                cx.invalidate();
            });
            let toggle = cx.listener("agree", |this, cx| {
                this.checked = !this.checked;
                cx.invalidate();
            });
            let name = Field::new("name")
                .required(true)
                .invalid(self.value.is_empty())
                .validation_message("Name is required");
            let agree = Field::new("agree");
            let disabled = Fieldset::new("disabled-group").disabled(true);
            let blocked = disabled.field("blocked");
            div()
                .child(name.label_part(text("Name")))
                .child(name.control_part(text_input(self.value.clone()).on_input(edit)))
                .child(name.description_part(text("Visible publicly")))
                .child(name.error_part(text("Name is required")))
                .child(agree.label_part(text("Agree")))
                .child(agree.control_part(checkbox(self.checked).on_click(toggle)))
                .child(disabled.root_part(div()).children([
                    disabled.legend_part(text("Disabled group")),
                    blocked.label_part(text("Blocked")),
                    blocked.control_part(text_input("")),
                ]))
        }
    }

    #[test]
    fn labels_focus_or_activate_controls_and_disabled_groups_do_not() {
        let (mut cx, view) = TestAppContext::new(FieldView::default()).unwrap();
        let window = view.window_handle();
        let name = Field::new("name")
            .required(true)
            .invalid(true)
            .validation_message("Name is required");
        let agree = Field::new("agree");

        cx.click(window, name.label_id()).unwrap();
        assert_eq!(cx.focused(window).unwrap(), Some(name.control_id()));
        cx.click(window, agree.label_id()).unwrap();
        assert!(cx.read(view, |view| view.checked).unwrap());

        let blocked = Fieldset::new("disabled-group")
            .disabled(true)
            .field("blocked");
        assert!(cx.click(window, blocked.label_id()).is_err());

        let update = cx.accessibility_update(window).unwrap();
        let control = update
            .nodes
            .iter()
            .find_map(|(id, node)| (id.0 == name.control_id().as_u64()).then_some(node))
            .expect("field control accessibility node");
        assert!(control.is_required());
        assert_eq!(control.invalid(), Some(accesskit::Invalid::True));
        assert_eq!(
            control.labelled_by(),
            &[accesskit::NodeId(name.label_id().as_u64())]
        );
        assert_eq!(
            control.described_by(),
            &[
                accesskit::NodeId(name.description_id().as_u64()),
                accesskit::NodeId(name.error_id().as_u64()),
            ]
        );
        let renders = cx.render_count(window).unwrap();
        cx.run_until_idle().unwrap();
        assert_eq!(cx.render_count(window).unwrap(), renders);
    }

    #[test]
    fn fieldset_wires_group_semantics_without_paint() {
        let fieldset = Fieldset::new("billing").disabled(true);
        let root = fieldset.root_part(div().bg(Color::rgb8(9, 8, 7)));
        assert_eq!(root.accessibility.role, AccessibilityRole::Group);
        assert!(root.accessibility.disabled);
        assert_eq!(
            root.accessibility.relations.labelled_by(),
            Some(fieldset.legend_id())
        );
        assert_eq!(
            root.accessibility.relations.described_by(),
            Some(fieldset.description_id())
        );
        assert_eq!(root.visual.background, Some(Color::rgb8(9, 8, 7)));
        assert!(fieldset.field("company").state().disabled);
        assert!(fieldset.control_part(text_input("")).accessibility.disabled);
    }
}
