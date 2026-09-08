package main

import (
	"strconv"
	"strings"

	"github.com/egoist/quickgui/go/native"
	"github.com/egoist/quickgui/go/ui"
)

func pickerAppearance() *ui.PickerAppearance {
	return &ui.PickerAppearance{
		Width:               ptr(280.0),
		RowHeight:           ptr(30.0),
		MaxVisibleRows:      ptr(7.0),
		FontSize:            ptr(12.0),
		Radius:              ptr(10.0),
		Background:          p().Popup,
		Color:               p().Ink,
		HighlightBackground: p().Accent,
		HighlightColor:      p().OnAccent,
		SelectedBackground:  p().Selection,
		MutedColor:          p().Muted,
	}
}
func pickerSource(label string, items []ui.OptionDeclaration) ui.PickerSourceProps {
	s := inputStyle()
	s.Width = 280
	return ui.PickerSourceProps{
		Items:      func() []ui.OptionDeclaration { return items },
		Appearance: pickerAppearance(),
		PartProps: ui.PartProps{
			AriaLabel: label,
			Style:     s,
		},
	}
}
func AutocompleteDemo() {
	query, setQuery := ui.CreateSignal("")
	committed, setCommitted := ui.CreateSignal("—")
	open, setOpen := ui.CreateSignal(false)
	items := []ui.OptionDeclaration{{Value: "window", Label: "Window"}, {Value: "widget", Label: "Widget"}, {Value: "tabs", Label: "Tabs"}, {Value: "table", Label: "Table"}, {Value: "toolbar", Label: "Toolbar"}, {Value: "tooltip", Label: "Tooltip"}}
	panel("Autocomplete", "Free-form text with suggestions. The core filters, ranks, and paints the popup rows.", func() {
		source := pickerSource("Search components", items)
		source.FilterMode = "fuzzy"
		source.OnOpenChange = change(setOpen)
		source.OnCommit = func(d ui.CommitDetails, _ *native.Event) { setCommitted(choose(d.Value != "", d.Value, d.InputValue)) }
		ui.Autocomplete.Root(ui.AutocompleteRootProps{PickerInputProps: ui.PickerInputProps{
			PickerSourceProps:  source,
			Placeholder:        "Type win, tab, or tool",
			OnInputValueChange: change(setQuery),
		}})
		note(func() string {
			return "query " + strconv.Quote(query()) + " · popup " + strconv.FormatBool(open()) + " · committed " + committed()
		})
	})
}
func ComboboxDemo() {
	fruit, setFruit := ui.CreateSignal(ptr("apple"))
	tags, setTags := ui.CreateSignal([]string{"go"})
	fruits := []ui.OptionDeclaration{{Value: "apple", Label: "Apple", Group: "Common"}, {Value: "banana", Label: "Banana", Group: "Common"}, {Value: "lychee", Label: "Lychee", Group: "Tropical"}, {Value: "mango", Label: "Mango", Group: "Tropical"}}
	languages := []ui.OptionDeclaration{{Value: "go", Label: "Go"}, {Value: "zig", Label: "Zig"}, {Value: "swift", Label: "Swift"}, {Value: "typescript", Label: "TypeScript"}}
	panel("Combobox", "A constrained picker and a multiple combobox. Removable chips reflect the selected values reported by the core.", func() {
		source := pickerSource("Fruit", fruits)
		source.FilterMode = "contains"
		ui.Combobox.Root(ui.ComboboxRootProps{
			Value:         fruit,
			OnValueChange: change(setFruit),
			AutoHighlight: ptr(true),
			PickerInputProps: ui.PickerInputProps{
				PickerSourceProps: source,
				Placeholder:       "Pick a fruit",
			},
		})
		source = pickerSource("Tags", languages)
		source.FilterMode = "startsWith"
		ui.Combobox.Root(
			ui.ComboboxRootProps{
				Multiple:       true,
				Values:         tags,
				OnValuesChange: change(setTags),
				AutoHighlight:  ptr(true),
				PickerInputProps: ui.PickerInputProps{
					PickerSourceProps: source,
					Placeholder:       "Add tags",
				},
			},
			func() {
				chips := ui.UseComboboxChips()
				state := ui.UseComboboxState()
				ui.Combobox.Chips(
					rowPart(),
					func() {
						ui.For(
							chips,
							func(chip ui.ComboboxChip, index func() int) {
								ui.Combobox.Chip(
									ui.ComboboxChipProps{
										Index: ptr(index()),
										PartProps: ui.PartProps{Style: ui.Styles(
											ui.Display("flex"),
											ui.AlignItems("center"),
											ui.Gap(4),
											ui.PaddingLeft(8),
											ui.PaddingRight(6),
											ui.Height(22),
											ui.BorderRadius(11),
											ui.BackgroundColor(color(func(p palette) string { return p.Selection })),
										)},
									},
									func() {
										label(chip.Label)
										ui.Combobox.ChipRemove(
											ui.ComboboxChipProps{
												Index:     ptr(index()),
												PartProps: ui.PartProps{AriaLabel: "Remove " + chip.Label},
											},
											"×",
										)
									},
								)
							},
							func(chip ui.ComboboxChip) any { return chip.Value },
							nil,
						)
					},
				)
				note(func() string {
					return "open " + strconv.FormatBool(state().PopupOpen) + " · results " + strconv.Itoa(state().ResultCount)
				})
			},
		)
		note(func() string { return "fruit " + textValue(fruit()) + " · tags [" + strings.Join(tags(), ", ") + "]" })
	})
}
func SelectDemo() {
	theme, setTheme := ui.CreateSignal(ptr("system"))
	sizes, setSizes := ui.CreateSignal([]string{"m"})
	themes := []ui.OptionDeclaration{{Value: "light", Label: "Light"}, {Value: "dark", Label: "Dark"}, {Value: "system", Label: "Match system"}}
	options := []ui.OptionDeclaration{{Value: "s", Label: "Small"}, {Value: "m", Label: "Medium"}, {Value: "l", Label: "Large"}, {Value: "xl", Label: "Extra large"}}
	panel("Select", "Single and multiple selection. The popup is rendered from the declared appearance, with ordered selected values.", func() {
		source := pickerSource("Theme", themes)
		source.Style = controlStyle()
		ui.Select.Root(
			ui.SelectRootProps{
				Value:             theme,
				OnValueChange:     change(setTheme),
				PickerSourceProps: source,
			},
			func() {
				ui.Select.Value(
					ui.PartProps{},
					func() {
						text := ui.UseSelectValueText()
						label(func() string { return textValue(text()) })
					},
				)
				ui.Select.Icon(ui.PartProps{}, "▾")
			},
		)
		source = pickerSource("Sizes", options)
		source.Style = controlStyle()
		ui.Select.Root(
			ui.SelectRootProps{
				Multiple:             true,
				Values:               sizes,
				OnValuesChange:       change(setSizes),
				AlignItemWithTrigger: ptr(true),
				PickerSourceProps:    source,
			},
			func() {
				ui.Select.Value(
					ui.PartProps{},
					func() {
						label(func() string { return choose(len(sizes()) == 0, "Pick sizes", strings.Join(sizes(), ", ")) })
					},
				)
				ui.Select.Icon(ui.PartProps{}, "▾")
				ui.Select.Positioner(
					ui.PickerPositionerProps{
						Side:       "bottom",
						Align:      "start",
						SideOffset: 6,
					},
					func() {
						ui.Select.ScrollUpArrow(ui.PartProps{})
						ui.Select.ScrollDownArrow(ui.PartProps{})
					},
				)
				state := ui.UseSelectState()
				note(func() string {
					return "popup " + strconv.FormatBool(state().PopupOpen) + " · side " + state().PopupSide + " · filled " + strconv.FormatBool(state().Filled) + " · touched " + strconv.FormatBool(state().Touched)
				})
			},
		)
		note(func() string {
			return "theme " + textValue(theme()) + " · sizes [" + strings.Join(sizes(), ", ") + "]"
		})
	})
}
