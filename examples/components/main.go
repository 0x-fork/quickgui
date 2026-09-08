package main

import (
	"log"
	"strconv"

	"github.com/egoist/quickgui/go/native"
	"github.com/egoist/quickgui/go/ui"
)

type demo struct {
	ID, Label, Source string
	Component         func()
}

var demos = []demo{
	{"accordion", "Accordion", "", AccordionDemo},
	{"alert-dialog", "Alert Dialog", "", AlertDialogDemo},
	{"autocomplete", "Autocomplete", "", AutocompleteDemo},
	{"avatar", "Avatar", "", AvatarDemo},
	{"button", "Button", "", ButtonDemo},
	{"calendar", "Calendar", "QuickGUI", CalendarDemo},
	{"checkbox", "Checkbox", "", CheckboxDemo},
	{"checkbox-group", "Checkbox Group", "", CheckboxGroupDemo},
	{"collapsible", "Collapsible", "", CollapsibleDemo},
	{"combobox", "Combobox", "", ComboboxDemo},
	{"context-menu", "Context Menu", "", ContextMenuDemo},
	{"system-context-menu", "System Context Menu", "System", SystemContextMenuDemo},
	{"date-field", "Date Field", "QuickGUI", DateFieldDemo},
	{"dialog", "Dialog", "", DialogDemo},
	{"field", "Field", "", FieldDemo},
	{"fieldset", "Fieldset", "", FieldsetDemo},
	{"form", "Form", "", FormDemo},
	{"input", "Input", "", InputDemo},
	{"menu", "Menu", "", MenuDemo},
	{"menubar", "Menubar", "", MenubarDemo},
	{"meter", "Meter", "", MeterDemo},
	{"navigation-menu", "Navigation Menu", "", NavigationMenuDemo},
	{"number-field", "Number Field", "", NumberFieldDemo},
	{"otp-field", "OTP Field", "", OtpFieldDemo},
	{"popover", "Popover", "", PopoverDemo},
	{"preview-card", "Preview Card", "", PreviewCardDemo},
	{"progress", "Progress", "", ProgressDemo},
	{"radio", "Radio", "", RadioDemo},
	{"scroll-area", "Scroll Area", "", ScrollAreaDemo},
	{"select", "Select", "", SelectDemo},
	{"separator", "Separator", "", SeparatorDemo},
	{"slider", "Slider", "", SliderDemo},
	{"splitter", "Splitter", "QuickGUI", SplitterDemo},
	{"switch", "Switch", "", SwitchDemo},
	{"tabs", "Tabs", "", TabsDemo},
	{"time-field", "Time Field", "QuickGUI", TimeFieldDemo},
	{"toast", "Toast", "", ToastDemo},
	{"toggle", "Toggle", "", ToggleDemo},
	{"toggle-group", "Toggle Group", "", ToggleGroupDemo},
	{"toolbar", "Toolbar", "", ToolbarDemo},
	{"tooltip", "Tooltip", "", TooltipDemo},
	{"table", "Table", "QuickGUI", TableDemo},
	{"tree", "Tree", "QuickGUI", TreeDemo},
}

func main() {
	if err := native.Run(func() {
		open := func() {
			native.NewWindow(native.WindowOptions{
				Title:                "QuickGUI Components",
				Width:                1080,
				Height:               780,
				MinimumWidth:         880,
				MinimumHeight:        620,
				TitleBarStyle:        "hiddenInset",
				TrafficLightPosition: &native.Point{X: 16, Y: 18},
				Background:           lightPalette.Window,
				Component: func() {
					window := native.CurrentWindow()
					appearance, setAppearance := ui.CreateSignal("light")
					size, setSize := ui.CreateSignal(native.Point{X: 1080, Y: 780})
					update := func() {
						window.GetState(func(state native.WindowState, err error) {
							if err == nil {
								setAppearance(state.Appearance)
								setSize(native.Point{
									X: state.ViewportWidth,
									Y: state.ViewportHeight,
								})
							}
						})
					}
					ui.OnCleanup(window.On(native.WindowReadyToShow, func(native.WindowEvent) { update() }))
					ui.OnCleanup(window.On(native.WindowResize, func(native.WindowEvent) { update() }))
					ui.OnCleanup(window.On(native.WindowAppearance, func(event native.WindowEvent) { setAppearance(event.Appearance) }))
					galleryContext.Provide(galleryTheme{Appearance: appearance, Size: size}, Gallery)
				},
			})
		}
		native.App.OnReopen(func(event native.ReopenEvent) {
			if !event.HasVisibleWindows {
				open()
			}
		})
		open()
	}); err != nil {
		log.Fatal(err)
	}
}

func Gallery() {
	selected, setSelected := ui.CreateSignal("accordion")
	current := func() demo {
		for _, entry := range demos {
			if entry.ID == selected() {
				return entry
			}
		}
		return demos[0]
	}
	ui.Tabs.Root(
		ui.TabsRootProps{
			Value:         func() *string { return ptr(selected()) },
			OnValueChange: func(value string, _ *native.Event) { setSelected(value) },
			Orientation:   "vertical",
			Activation:    "manual",
			PartProps: ui.PartProps{Style: func() ui.Style {
				return ui.Styles(
					ui.Display("flex"),
					ui.Width("100%"),
					ui.Height("100%"),
					ui.BackgroundColor(p().Window),
					ui.TextColor(p().Ink),
				)
			}},
		},
		func() {
			ui.View(
				func() {
					ui.View(
						func() {
							ui.Text("Components", ui.FontSize(13), ui.FontWeight(700))
						},
						ui.Display("flex"),
						ui.Height(52),
						ui.FlexShrink(0),
						ui.AlignItems("center"),
						ui.PaddingLeft(82),
						ui.AppRegion("drag"),
					)
					ui.View(
						func() {
							ui.Tabs.List(
								ui.PartProps{Style: ui.Styles(
									ui.Display("flex"),
									ui.FlexDirection("column"),
									ui.Gap(1),
									ui.PaddingLeft(8),
									ui.PaddingRight(8),
									ui.PaddingBottom(12),
								)},
								func() {
									for index, entry := range demos {
										ui.Tabs.Tab(
											ui.TabsTabProps{
												Value: entry.ID,
												Index: ptr(index),
												PartProps: ui.PartProps{Style: func() ui.Style {
													background, ink, weight := "transparent", p().Ink, 400
													if selected() == entry.ID {
														background, ink, weight = p().Accent, p().OnAccent, 600
													}
													return ui.Styles(
														ui.Display("flex"),
														ui.AlignItems("center"),
														ui.Height(28),
														ui.FlexShrink(0),
														ui.PaddingLeft(10),
														ui.PaddingRight(10),
														ui.BorderRadius(7),
														ui.Cursor("default"),
														ui.UserSelect("none"),
														ui.BackgroundColor(background),
														ui.TextColor(ink),
														ui.FontSize(12),
														ui.FontWeight(weight),
														ui.Hover(ui.BackgroundColor(choose(selected() == entry.ID, p().Accent, p().ControlHover))),
														ui.Focus(ui.Outline("2px solid "+p().Accent)),
														ui.OutlineOffset(-2),
													)
												}},
											},
											entry.Label,
										)
									}
								},
							)
						},
						ui.Display("flex"),
						ui.FlexDirection("column"),
						ui.Flex(1),
						ui.MinHeight(0),
						ui.OverflowY("scroll"),
					)
				},
				ui.Display("flex"),
				ui.FlexDirection("column"),
				ui.Width(214),
				ui.FlexShrink(0),
				ui.Height("100%"),
				ui.BackgroundColor(color(func(p palette) string { return p.Sidebar })),
				ui.BorderRightWidth(1),
				ui.BorderColor(color(func(p palette) string { return p.Border })),
			)
			ui.View(
				func() {
					ui.View(
						func() {
							row(func() {
								ui.Text(
									func() string { return current().Label },
									ui.FontSize(15),
									ui.FontWeight(700),
								)
								ui.Text(
									func() string {
										switch current().Source {
										case "QuickGUI":
											return "QuickGUI component"
										case "System":
											return "Native system menu"
										default:
											return "Base UI part set"
										}
									},
									ui.FontSize(11),
									ui.TextColor(color(func(p palette) string { return p.Faint })),
								)
							})
							ui.Text(
								func() string {
									theme := galleryContext.Use()
									size := theme.Size()
									return strconv.Itoa(len(demos)) + " components · " + theme.Appearance() + " appearance · " + strconv.FormatFloat(size.X, 'f', 0, 64) + "×" + strconv.FormatFloat(size.Y, 'f', 0, 64)
								},
								ui.FontSize(11),
								ui.TextColor(color(func(p palette) string { return p.Faint })),
							)
						},
						ui.Display("flex"),
						ui.AlignItems("center"),
						ui.JustifyContent("space-between"),
						ui.Height(52),
						ui.FlexShrink(0),
						ui.PaddingLeft(20),
						ui.PaddingRight(20),
						ui.BorderBottomWidth(1),
						ui.BorderColor(color(func(p palette) string { return p.Border })),
						ui.AppRegion("drag"),
					)
					ui.View(
						func() {
							for _, entry := range demos {
								ui.Tabs.Panel(
									ui.TabsPanelProps{
										Value: entry.ID,
										PartProps: ui.PartProps{Style: ui.Styles(
											ui.Display("flex"),
											ui.FlexDirection("column"),
											ui.Gap(16),
											ui.MaxWidth(720),
											ui.FlexShrink(0),
										)},
									},
									entry.Component,
								)
							}
						},
						ui.Display("flex"),
						ui.FlexDirection("column"),
						ui.Flex(1),
						ui.MinHeight(0),
						ui.Padding(20),
						ui.Gap(16),
						ui.OverflowY("scroll"),
					)
				},
				ui.Display("flex"),
				ui.FlexDirection("column"),
				ui.Flex(1),
				ui.MinWidth(0),
				ui.Height("100%"),
			)
		},
	)
}
