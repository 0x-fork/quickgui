package main

import (
	"fmt"
	"log"

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
				return ui.Style{
					Display:         "flex",
					Width:           "100%",
					Height:          "100%",
					BackgroundColor: p().Window,
					Color:           p().Ink,
				}
			}},
		},
		func() {
			ui.View(
				ui.Display("flex"),
				ui.FlexDirection("column"),
				ui.Width(214),
				ui.FlexShrink(0),
				ui.Height("100%"),
				ui.BackgroundColor(color(func(p palette) string { return p.Sidebar })),
				ui.BorderRightWidth(1),
				ui.BorderColor(color(func(p palette) string { return p.Border })),
				func() {
					ui.View(
						ui.Display("flex"),
						ui.Height(52),
						ui.FlexShrink(0),
						ui.AlignItems("center"),
						ui.PaddingLeft(82),
						ui.AppRegion("drag"),
						func() {
							ui.Text(
								ui.FontSize(13),
								ui.FontWeight(700),
								"Components",
							)
						},
					)
					ui.View(
						ui.Display("flex"),
						ui.FlexDirection("column"),
						ui.Flex(1),
						ui.MinHeight(0),
						ui.OverflowY("scroll"),
						func() {
							ui.Tabs.List(
								ui.PartProps{Style: ui.Style{
									Display:       "flex",
									FlexDirection: "column",
									Gap:           1,
									PaddingLeft:   8,
									PaddingRight:  8,
									PaddingBottom: 12,
								}},
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
													return ui.Style{
														Display:         "flex",
														AlignItems:      "center",
														Height:          28,
														FlexShrink:      0,
														PaddingLeft:     10,
														PaddingRight:    10,
														BorderRadius:    7,
														Cursor:          "default",
														UserSelect:      "none",
														BackgroundColor: background,
														Color:           ink,
														FontSize:        12,
														FontWeight:      weight,
														Hover:           &ui.Style{BackgroundColor: choose(selected() == entry.ID, p().Accent, p().ControlHover)},
														Focus:           &ui.Style{Outline: "2px solid " + p().Accent},
														OutlineOffset:   -2,
													}
												}},
											},
											entry.Label,
										)
									}
								},
							)
						},
					)
				},
			)
			ui.View(
				ui.Display("flex"),
				ui.FlexDirection("column"),
				ui.Flex(1),
				ui.MinWidth(0),
				ui.Height("100%"),
				func() {
					ui.View(
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
						func() {
							row(func() {
								ui.Text(
									ui.FontSize(15),
									ui.FontWeight(700),
									func() string { return current().Label },
								)
								ui.Text(
									ui.FontSize(11),
									ui.Color(color(func(p palette) string { return p.Faint })),
									func() string {
										return choose(current().Source == "QuickGUI", "QuickGUI component", "Base UI part set")
									},
								)
							})
							ui.Text(
								ui.FontSize(11),
								ui.Color(color(func(p palette) string { return p.Faint })),
								func() string {
									theme := galleryContext.Use()
									size := theme.Size()
									return fmt.Sprintf("%d components · %s appearance · %.0f×%.0f", len(demos), theme.Appearance(), size.X, size.Y)
								},
							)
						},
					)
					ui.View(
						ui.Display("flex"),
						ui.FlexDirection("column"),
						ui.Flex(1),
						ui.MinHeight(0),
						ui.Padding(20),
						ui.Gap(16),
						ui.OverflowY("scroll"),
						func() {
							for _, entry := range demos {
								ui.Tabs.Panel(
									ui.TabsPanelProps{
										Value: entry.ID,
										PartProps: ui.PartProps{Style: ui.Style{
											Display:       "flex",
											FlexDirection: "column",
											Gap:           16,
											MaxWidth:      720,
											FlexShrink:    0,
										}},
									},
									entry.Component,
								)
							}
						},
					)
				},
			)
		},
	)
}
