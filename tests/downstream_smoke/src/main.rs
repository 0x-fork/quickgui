use quickgui::{
    AccessibilityRole, Accordion, AccordionState, AnchorPlacement, Application,
    AutocompleteListState, AutocompleteOptionState, AutocompletePopoverLayout, AutocompleteState,
    CONTEXT_MENU_SUBMENU_AIM_DELAY, CONTEXT_MENU_SUBMENU_HOVER_DELAY, Checkbox, Collapsible,
    ComboboxListState, ComboboxOptionState, ComboboxPopoverLayout, ComboboxState,
    ContextMenuLayout, ContextMenuState, Dialog, Field, Fieldset, FontFallbacks, FontFamily,
    FontFeatureTag, FontFeatures, IntoElement, PickerItem, PickerLayout, PickerState, Popover,
    PopoverKind, PopoverMenu, PopoverMenuItem, Radio, RadioGroup, SelectState, Switch, TableColumn,
    TableLayout, TableState, ToggleState, TreeLayout, TreeNode, TreeState, View, ViewContext,
    WindowOptions, checkbox, combobox_key_bindings, div, font, radio, radio_group, switch, text,
    text_input,
};

struct PackagedApp {
    autocomplete: AutocompleteState<&'static str>,
    combobox: ComboboxState<&'static str>,
    picker: PickerState<&'static str>,
    table: TableState,
    tree: TreeState<&'static str>,
}

impl PackagedApp {
    fn new() -> Self {
        Self {
            autocomplete: AutocompleteState::new([
                PickerItem::new("Apple", "apple").id("apple"),
                PickerItem::new("Apricot", "apricot").id("apricot"),
            ])
            .expect("packaged AutocompleteState API should accept a valid bounded source")
            .with_layout(AutocompletePopoverLayout::new(240.0, 36.0)),
            combobox: ComboboxState::new([
                PickerItem::new("System", "system").id("system"),
                PickerItem::new("Dark", "dark").id("dark"),
            ])
            .expect("packaged ComboboxState API should accept a valid bounded source")
            .with_layout(ComboboxPopoverLayout::new(240.0, 36.0))
            .with_selected_id("system"),
            picker: PickerState::new([
                PickerItem::new("Open", "open").id("open"),
                PickerItem::new("Save", "save").id("save"),
            ])
            .expect("packaged PickerState API should accept a valid bounded source")
            .with_layout(PickerLayout::new(36.0).max_visible_rows(4)),
            table: TableState::new(2).with_layout(TableLayout::new(30.0, 28.0)),
            tree: TreeState::new([TreeNode::new("src", "src", "directory")
                .child(TreeNode::new("lib", "lib.rs", "file"))])
            .expect("packaged TreeState API should accept a valid bounded source")
            .with_layout(TreeLayout::new(28.0)),
        }
    }

    fn autocomplete(view: &mut Self) -> &mut AutocompleteState<&'static str> {
        &mut view.autocomplete
    }

    fn combobox(view: &mut Self) -> &mut ComboboxState<&'static str> {
        &mut view.combobox
    }

    fn picker(view: &mut Self) -> &mut PickerState<&'static str> {
        &mut view.picker
    }

    fn table(view: &mut Self) -> &mut TableState {
        &mut view.table
    }

    fn tree(view: &mut Self) -> &mut TreeState<&'static str> {
        &mut view.tree
    }
}

impl View for PackagedApp {
    fn render(&mut self, cx: &mut ViewContext<'_, Self>) -> impl IntoElement {
        let input_value = self.autocomplete.value().clone();
        let autocomplete = self.autocomplete.element(
            cx,
            "packaged-autocomplete",
            "Packaged autocomplete",
            Self::autocomplete,
            text_input(input_value),
            |_state: AutocompleteListState| div(),
            |item, _state: AutocompleteOptionState| div().child(item.label().clone()),
            |_view, _value, _cx| {},
            |_view, _value, _cx| {},
        );
        let combobox_value = self.combobox.input_value().clone();
        let combobox = self.combobox.element(
            cx,
            "packaged-combobox",
            "Packaged constrained combobox",
            Self::combobox,
            text_input(combobox_value),
            |_state: ComboboxListState| div(),
            |item, _state: ComboboxOptionState| div().child(item.label().clone()),
            |_view, _query, _cx| {},
            |_view, _value, _cx| {},
        );
        let picker_query = self.picker.query().clone();
        let picker = self.picker.element(
            cx,
            "packaged-picker",
            "Packaged picker",
            Self::picker,
            text_input(picker_query),
            div().child("No results"),
            |matched| div().child(matched.item().label().clone()),
            |_view, _value, _cx| {},
        );
        let columns = [
            TableColumn::new("name", "Name").row_header(true),
            TableColumn::new("kind", "Kind"),
        ];
        let table = self.table.element(
            cx,
            "packaged-table",
            &columns,
            Self::table,
            |header| div().child(header.column.label().clone()),
            |cell| div().child(format!("{}:{}", cell.position.row, cell.position.column)),
            |_view, _position, _cx| {},
        );
        let tree = self.tree.element(
            cx,
            "packaged-tree",
            Self::tree,
            |row, disclosure| {
                div()
                    .child(disclosure.unwrap_or_else(div))
                    .child(row.label().clone())
            },
            |_view, _id, _cx| {},
        );
        div()
            .size_full()
            .flex_row()
            .items_center()
            .justify_evenly()
            .gap_x_2()
            .mx_auto()
            .text_center()
            .font(
                font(FontFamily::Monospace)
                    .features(FontFeatures::new().disable(FontFeatureTag::CONTEXTUAL_ALTERNATES))
                    .fallbacks(FontFallbacks::from_fonts(["Apple Color Emoji"])),
            )
            .opacity(0.9)
            .hover(|style| style.opacity(1.0))
            .child(
                text("QuickGUI packaged consumer with a deliberately long title")
                    .italic()
                    .underline()
                    .text_decoration_2()
                    .text_decoration_wavy()
                    .w(240.0)
                    .flex_1()
                    .min_w(0.0)
                    .truncate(),
            )
            .child(div().h(24.0).aspect_square().flex_none())
            .child(autocomplete)
            .child(combobox)
            .child(picker)
            .child(table)
            .child(tree)
            .child(div().hidden())
            .child(div().invisible().visible())
    }
}

fn main() {
    let popover = Popover::new("packaged-trigger", "packaged-popover", true)
        .kind(PopoverKind::Dialog)
        .placement(AnchorPlacement::BottomEnd)
        .anchor_gap(8.0)
        .viewport_margin(12.0);
    let popover = popover.initial_focus(popover.close_id());
    let _popover_trigger = popover.trigger().child("Open packaged popover");
    let _popover_positioner =
        popover.positioner().child(popover.popup().children([
            popover.title_with(text("Packaged popover")),
            popover.description_with(text("Packaged unstyled parts")),
            popover.close_with("Close packaged popover", div()),
        ]));
    let _popover_backdrop = popover.backdrop();
    let _merged_popover_surface = popover.surface();
    let checkbox_parts = Checkbox::new(ToggleState::Mixed);
    let _checkbox_root = checkbox_parts.root_with(
        div()
            .child(checkbox_parts.indicator_with(div()))
            .child("Packaged checkbox"),
    );
    let radio_parts = Radio::new(true);
    let _radio_group_root = RadioGroup::new().root_with(
        div().child(
            radio_parts
                .root().child(radio_parts.indicator_with(div()))
                .child("Packaged radio"),
        ),
    );
    let switch_parts = Switch::new(false);
    let _switch_root = switch_parts
        .root().child(switch_parts.thumb_with(div()))
        .child("Packaged switch");
    let _selection_shorthands = (checkbox(false), radio(false), radio_group(), switch(false));
    let fieldset = Fieldset::new("packaged-fieldset");
    let field = fieldset
        .field("packaged-field")
        .required(true)
        .invalid(true)
        .validation_message("Packaged field is required");
    let _field_root = field.root().children([
        field.label_with(text("Packaged field")),
        field.control_with(text_input("")),
        field.description_with(text("Packaged field description")),
        field.error_with(text("Packaged field is required")),
    ]);
    let _fieldset_root = fieldset.root().children([
        fieldset.legend_with(text("Packaged fieldset")),
        fieldset.description_with(text("Packaged related controls")),
    ]);
    let _standalone_field = Field::new("standalone-field").state();
    let collapsible = Collapsible::new("packaged-collapsible", true).keep_mounted(true);
    let _collapsible_root = collapsible
        .root().child(collapsible.trigger().child("Packaged disclosure"))
        .children(collapsible.panel_with(text("Packaged collapsible panel")));
    let mut accordion_state = AccordionState::new().with_multiple(true);
    accordion_state
        .replace_open(["packaged-one", "packaged-two"])
        .expect("packaged bounded accordion state");
    let accordion = Accordion::new("packaged-accordion").keep_mounted(true);
    let accordion_item = accordion.item_from_state("packaged-one", 0, &accordion_state);
    let _accordion_root = accordion.root_with(
        div().child(
            accordion_item.root_with(
                div()
                    .child(
                        accordion_item.header_with(
                            div().child(
                                accordion_item
                                    .trigger()
                                    .child("Packaged accordion item"),
                            ),
                        ),
                    )
                    .children(accordion_item.panel_with(text("Packaged accordion panel"))),
            ),
        ),
    );
    let _select = SelectState::new([
        PickerItem::new("System", "system").id("system"),
        PickerItem::new("Dark", "dark").id("dark"),
    ])
    .expect("packaged SelectState API should accept a valid bounded source");
    let _context_menu = ContextMenuState::new();
    let _context_menu_layout = ContextMenuLayout::new(224.0, 36.0).vertical_padding(4.0);
    let _context_menu_timing = (
        CONTEXT_MENU_SUBMENU_HOVER_DELAY,
        CONTEXT_MENU_SUBMENU_AIM_DELAY,
    );
    let popover_menu = PopoverMenu::new([
        PopoverMenuItem::group_label("File"),
        PopoverMenuItem::action("open", "Open", ()),
        PopoverMenuItem::separator(),
    ])
    .expect("packaged PopoverMenu API should accept labeled groups and separators");
    let _group = popover_menu
        .labeled_group_with("packaged-menu", 0, div())
        .expect("the packaged group label should name its group part");
    let _separator = popover_menu
        .item_with("packaged-menu", 2, div())
        .expect("the packaged separator part should project semantics");
    let _labelled = div()
        .accessibility_role(AccessibilityRole::Group)
        .accessibility_labelled_by("packaged-label")
        .accessibility_described_by("packaged-description");
    let dialog = Dialog::alert("packaged-dialog", true).restore_focus_to("packaged-trigger");
    let _dialog_root = dialog.root().children([
        dialog.backdrop(),
        dialog.popup().children([
            dialog.title_with(text("Packaged alert")),
            dialog.description_with(text("Packaged description")),
        ]),
    ]);
    let _application = Application::new().bind_keys(combobox_key_bindings());
    let _window = WindowOptions::default().inspector(false);
    let _view = PackagedApp::new();
}
