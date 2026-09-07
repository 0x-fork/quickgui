package main

import (
	"fmt"
	"log"
	"time"

	"github.com/egoist/quickgui/go/native"
	"github.com/egoist/quickgui/go/reactive"
	"github.com/egoist/quickgui/go/ui"
)

type demoID string

type demo struct {
	ID          demoID
	Label       string
	Description string
}

var demos = []demo{
	{ID: "button", Label: "Button", Description: "A native SwiftUI button with an SF Symbol, Liquid Glass styling, and an asynchronous press event delivered to QuickGUI."},
	{ID: "slider", Label: "Slider", Description: "A controlled native slider with a bounded range and discrete steps. Its value is owned by the QuickGUI signal below."},
	{ID: "toggle", Label: "Toggle", Description: "A controlled SwiftUI toggle that reports its native on/off state through the hosted event queue."},
	{ID: "progress-view", Label: "Progress View", Description: "A determinate SwiftUI progress view with a semantic label and a formatted current-value label."},
	{ID: "stepper", Label: "Stepper", Description: "A bounded native stepper. SwiftUI performs the interaction and QuickGUI receives the updated numeric value."},
	{ID: "segmented-control", Label: "Segmented Control", Description: "A native segmented picker using the neutral tabs role—the Liquid Glass treatment used for Xcode-style navigation."},
	{ID: "picker", Label: "Picker", Description: "A native menu picker backed by typed options and a controlled string selection."},
	{ID: "date-picker", Label: "Date Picker", Description: "A native field-style date and time picker whose value crosses the bridge as a Unix timestamp in milliseconds."},
	{ID: "color-picker", Label: "Color Picker", Description: "A native color well with opacity support. SwiftUI selections are returned as RGBA hex strings."},
	{ID: "gauge", Label: "Gauge", Description: "A native accessory-capacity gauge with minimum, maximum, and current-value labels."},
	{ID: "text-field", Label: "Text Field", Description: "A controlled native text field that reports edits and Return-key submissions independently."},
	{ID: "secure-field", Label: "Secure Field", Description: "The secure variant uses SwiftUI's native concealed editor while keeping the same QuickGUI value contract."},
	{ID: "popover", Label: "Popover", Description: "A SwiftUI button presents a native popover, which reverse-hosts an ordinary interactive QuickGUI subtree."},
}

func main() {
	if err := native.Run(func() {
		openMainWindow()
		native.App.OnReopen(func(event native.ReopenEvent) {
			if !event.HasVisibleWindows {
				openMainWindow()
			}
		})
	}); err != nil {
		log.Fatal(err)
	}
}

func openMainWindow() {
	native.NewWindow(native.WindowOptions{
		Title:                "QuickGUI SwiftUI Components",
		Width:                920,
		Height:               680,
		MinimumWidth:         720,
		MinimumHeight:        500,
		Background:           "transparent",
		Vibrancy:             "sidebar",
		VisualEffectState:    "followWindow",
		TitleBarStyle:        "hiddenInset",
		TrafficLightPosition: &native.Point{X: 16, Y: 18},
		Component:            Gallery,
	})
}

func Gallery() {
	state := newGalleryState()
	selected := func() *string {
		current := string(state.page())
		return &current
	}
	ui.View(
		ui.Display("flex"),
		ui.FlexDirection("row"),
		ui.Width("100%"),
		ui.Height("100%"),
		ui.BackgroundColor("transparent"),
		func() {
			ui.Tabs.Root(
				ui.TabsRootProps{
					Value:         selected,
					OnValueChange: func(next string, _ *native.Event) { state.setPage(demoID(next)) },
					Orientation:   "vertical",
					Activation:    "manual",
					PartProps: ui.PartProps{
						Style: ui.Style{
							Display:       "flex",
							FlexDirection: "row",
							Width:         "100%",
							Height:        "100%",
						},
					},
				},
				func() {
					sidebar(state)
					pane(state, func() {
						renderDemo(state)
					})
				},
			)
		},
	)
}

type galleryState struct {
	page             reactive.Accessor[demoID]
	setPage          reactive.Setter[demoID]
	presses          reactive.Accessor[int]
	setPresses       reactive.Setter[int]
	open             reactive.Accessor[bool]
	setOpen          reactive.Setter[bool]
	name             reactive.Accessor[string]
	setName          reactive.Setter[string]
	password         reactive.Accessor[string]
	setPassword      reactive.Setter[string]
	submitted        reactive.Accessor[string]
	setSubmitted     reactive.Setter[string]
	volume           reactive.Accessor[float64]
	setVolume        reactive.Setter[float64]
	notifications    reactive.Accessor[bool]
	setNotifications reactive.Setter[bool]
	copies           reactive.Accessor[float64]
	setCopies        reactive.Setter[float64]
	layout           reactive.Accessor[string]
	setLayout        reactive.Setter[string]
	interval         reactive.Accessor[string]
	setInterval      reactive.Setter[string]
	scheduledAt      reactive.Accessor[float64]
	setScheduledAt   reactive.Setter[float64]
	accent           reactive.Accessor[string]
	setAccent        reactive.Setter[string]
}

func newGalleryState() *galleryState {
	s := &galleryState{}
	s.page, s.setPage = ui.CreateSignal(demoID("button"))
	s.presses, s.setPresses = ui.CreateSignal(0)
	s.open, s.setOpen = ui.CreateSignal(false)
	s.name, s.setName = ui.CreateSignal("Ada")
	s.password, s.setPassword = ui.CreateSignal("")
	s.submitted, s.setSubmitted = ui.CreateSignal("none")
	s.volume, s.setVolume = ui.CreateSignal(0.4)
	s.notifications, s.setNotifications = ui.CreateSignal(true)
	s.copies, s.setCopies = ui.CreateSignal(2.0)
	s.layout, s.setLayout = ui.CreateSignal("grid")
	s.interval, s.setInterval = ui.CreateSignal("week")
	s.scheduledAt, s.setScheduledAt = ui.CreateSignal(float64(time.Now().UnixMilli()))
	s.accent, s.setAccent = ui.CreateSignal("#3366ffff")
	return s
}

func (s *galleryState) current() demo {
	for _, item := range demos {
		if item.ID == s.page() {
			return item
		}
	}
	return demos[0]
}

func sidebar(state *galleryState) {
	ui.View(
		ui.Display("flex"),
		ui.FlexDirection("column"),
		ui.Width(220),
		ui.Height("100%"),
		ui.FlexShrink(0),
		ui.BackgroundColor("transparent"),
		ui.BorderWidth(1),
		ui.BorderColor("#c9cbd0"),
		func() {
			ui.View(
				ui.Display("flex"),
				ui.FlexDirection("row"),
				ui.AlignItems("center"),
				ui.Height(54),
				ui.FlexShrink(0),
				ui.PaddingLeft(82),
				ui.AppRegion("drag"),
				func() {
					ui.Text(ui.Color("#252a33"), ui.FontSize(13), ui.FontWeight(700), "SwiftUI")
				},
			)
			ui.Text(
				ui.FlexShrink(0),
				ui.PaddingLeft(18),
				ui.PaddingBottom(7),
				ui.Color("#747b87"),
				ui.FontSize(10),
				ui.FontWeight(700),
				ui.LetterSpacing(0.7),
				"COMPONENTS",
			)
			ui.View(
				ui.Display("flex"),
				ui.FlexDirection("column"),
				ui.Flex(1),
				ui.MinHeight(0),
				ui.OverflowY("scroll"),
				func() {
					ui.Tabs.List(
						ui.PartProps{
							Style: ui.Style{
								Display:       "flex",
								FlexDirection: "column",
								Gap:           2,
								PaddingLeft:   9,
								PaddingRight:  9,
								PaddingBottom: 12,
							},
						},
						func() {
							ui.For(
								func() []demo { return demos },
								func(item demo, index func() int) {
									id := item.ID
									idx := index()
									ui.Tabs.Tab(
										ui.TabsTabProps{
											Value: string(id),
											Index: &idx,
											PartProps: ui.PartProps{
												Style: func() ui.Style {
													background, color, weight := "transparent", "#303641", any(400)
													if state.page() == id {
														background, color, weight = "#2878d4", "#ffffff", 600
													}
													return ui.Style{
														Display:         "flex",
														FlexDirection:   "row",
														AlignItems:      "center",
														Height:          29,
														FlexShrink:      0,
														PaddingLeft:     10,
														PaddingRight:    10,
														BorderRadius:    7,
														Cursor:          "default",
														UserSelect:      "none",
														BackgroundColor: background,
														Color:           color,
														FontWeight:      weight,
														Hover:           &ui.Style{BackgroundColor: "#ffffff66"},
													}
												},
											},
										},
										func() {
											ui.Text(ui.FontSize(12), item.Label)
										},
									)
								},
								func(item demo) any { return item.ID },
								nil,
							)
						},
					)
				},
			)
			ui.View(
				ui.FlexShrink(0),
				ui.Padding(13),
				ui.BorderWidth(1),
				ui.BorderColor("#c9cbd0"),
				func() {
					ui.Text(
						ui.Color("#747b87"),
						ui.FontSize(11),
						fmt.Sprintf("%d native components", len(demos)),
					)
				},
			)
		},
	)
}

func pane(state *galleryState, body func()) {
	ui.View(
		ui.Display("flex"),
		ui.FlexDirection("column"),
		ui.Flex(1),
		ui.MinWidth(0),
		ui.Height("100%"),
		ui.BackgroundColor("#f6f6f8"),
		func() {
			ui.View(
				ui.Display("flex"),
				ui.FlexDirection("row"),
				ui.AlignItems("center"),
				ui.JustifyContent("space-between"),
				ui.Height(54),
				ui.FlexShrink(0),
				ui.PaddingLeft(22),
				ui.PaddingRight(22),
				ui.BorderWidth(1),
				ui.BorderColor("#d7d8dc"),
				ui.AppRegion("drag"),
				func() {
					ui.Text(
						ui.Color("#20242c"),
						ui.FontSize(15),
						ui.FontWeight(700),
						func() string { return state.current().Label },
					)
					ui.Text(ui.Color("#858b96"), ui.FontSize(11), "Native SwiftUI · QuickGUI state")
				},
			)
			ui.View(
				ui.Display("flex"),
				ui.FlexDirection("column"),
				ui.Flex(1),
				ui.MinHeight(0),
				ui.AlignItems("center"),
				ui.OverflowY("scroll"),
				ui.Padding(30),
				func() {
					body()
				},
			)
		},
	)
}

func renderDemo(state *galleryState) {
	demoPage(state.current().Description, demoStatus(state), func() {
		demoControl(state)
	})
}

func demoPage(description string, status func() string, control ui.Component) {
	ui.View(
		ui.Display("flex"),
		ui.FlexDirection("column"),
		ui.Width("100%"),
		ui.MaxWidth(680),
		ui.Gap(18),
		func() {
			ui.Text(ui.Color("#5f6672"), ui.FontSize(14), ui.LineHeight(21), description)
			ui.View(
				ui.Display("flex"),
				ui.FlexDirection("column"),
				ui.Width("100%"),
				ui.MinHeight(250),
				ui.Padding(22),
				ui.Gap(16),
				ui.BorderWidth(1),
				ui.BorderColor("#dedfe3"),
				ui.BorderRadius(14),
				ui.BackgroundColor("#ffffff"),
				func() {
					ui.Text(
						ui.Color("#858b96"),
						ui.FontSize(11),
						ui.FontWeight(700),
						ui.LetterSpacing(0.8),
						"LIVE SWIFTUI DEMO",
					)
					ui.View(
						ui.Display("flex"),
						ui.Flex(1),
						ui.MinHeight(170),
						ui.Width("100%"),
						ui.AlignItems("center"),
						ui.JustifyContent("center"),
						control,
					)
				},
			)
			ui.View(
				ui.Display("flex"),
				ui.FlexDirection("row"),
				ui.AlignItems("center"),
				ui.JustifyContent("space-between"),
				ui.Gap(16),
				ui.Width("100%"),
				ui.MinHeight(42),
				ui.PaddingLeft(14),
				ui.PaddingRight(14),
				ui.BorderRadius(10),
				ui.BackgroundColor("#eceef2"),
				func() {
					ui.Text(
						ui.Color("#727985"),
						ui.FontSize(11),
						ui.FontWeight(700),
						"NATIVE STATE",
					)
					ui.Text(ui.Color("#252a33"), ui.FontSize(12), status)
				},
			)
		},
	)
}

func demoStatus(state *galleryState) func() string {
	return func() string {
		switch state.page() {
		case "button":
			suffix := "es"
			if state.presses() == 1 {
				suffix = ""
			}
			return fmt.Sprintf("%d press%s", state.presses(), suffix)
		case "slider", "progress-view", "gauge":
			return fmt.Sprintf("volume %d%%", int(state.volume()*100+0.5))
		case "toggle":
			if state.notifications() {
				return "notifications on"
			}
			return "notifications off"
		case "stepper":
			return fmt.Sprintf("%g copies", state.copies())
		case "segmented-control":
			return "layout " + state.layout()
		case "picker":
			return "interval " + state.interval()
		case "date-picker":
			return time.UnixMilli(int64(state.scheduledAt())).UTC().Format(time.RFC3339Nano)
		case "color-picker":
			return "accent " + state.accent()
		case "text-field":
			return fmt.Sprintf("value “%s” · last submitted %s", state.name(), state.submitted())
		case "secure-field":
			return fmt.Sprintf("%d characters · last submitted %s", len(state.password()), state.submitted())
		case "popover":
			if state.open() {
				return "popover presented"
			}
			return "popover dismissed"
		default:
			return ""
		}
	}
}

func host(match any, style ui.Style, child *native.Node) {
	ui.SwiftUI.Host(
		ui.SwiftUIHostProps{
			MatchContents: match,
			PartProps: ui.PartProps{
				Style: style,
			},
		},
		func() { ui.Child(child) },
	)
}

func demoControl(state *galleryState) {
	wide := ui.Style{Width: 440}
	field := ui.Style{Width: 360}
	switch state.page() {
	case "button":
		host(true, ui.Style{}, ui.SwiftUI.Button(ui.SwiftUIButtonProps{
			Label:       "Continue",
			SystemImage: "arrow.right",
			Modifiers:   []ui.SwiftUIModifier{ui.SwiftUI.ButtonStyle("glass"), ui.SwiftUI.ControlSize("large")},
			OnPress:     func(*native.Event) { state.setPresses(state.presses() + 1) },
		}))
		return

	case "slider":
		host(ui.SwiftUIMatchContents{Vertical: true}, wide, ui.SwiftUI.Slider(ui.SwiftUISliderProps{
			Label:         func() string { return fmt.Sprintf("Volume %d%%", int(state.volume()*100+0.5)) },
			Value:         state.volume,
			Min:           0,
			Max:           1,
			Step:          0.05,
			OnValueChange: func(next float64, _ *native.Event) { state.setVolume(next) },
		}))
		return

	case "toggle":
		host(true, ui.Style{}, ui.SwiftUI.Toggle(ui.SwiftUIToggleProps{
			Label:        "Notifications",
			IsOn:         state.notifications,
			OnIsOnChange: func(next bool, _ *native.Event) { state.setNotifications(next) },
		}))
		return

	case "progress-view":
		host(ui.SwiftUIMatchContents{Vertical: true}, wide, ui.SwiftUI.ProgressView(ui.SwiftUIProgressViewProps{
			Label:             "Setup progress",
			Value:             state.volume,
			Total:             1,
			CurrentValueLabel: func() string { return fmt.Sprintf("%d%%", int(state.volume()*100+0.5)) },
		}))
		return

	case "stepper":
		host(true, ui.Style{}, ui.SwiftUI.Stepper(ui.SwiftUIStepperProps{
			Label:         func() string { return fmt.Sprintf("Copies: %g", state.copies()) },
			Value:         state.copies,
			Min:           1,
			Max:           10,
			OnValueChange: func(next float64, _ *native.Event) { state.setCopies(next) },
		}))
		return

	case "segmented-control":
		host(ui.SwiftUIMatchContents{Vertical: true}, ui.Style{Width: 340}, ui.SwiftUI.SegmentedControl(ui.SwiftUISegmentedControlProps{
			Role:              "tabs",
			Selection:         state.layout,
			Options:           []ui.SwiftUIPickerOption{{Value: "list", Label: "List"}, {Value: "grid", Label: "Grid"}},
			OnSelectionChange: func(next string, _ *native.Event) { state.setLayout(next) },
		}))
		return

	case "picker":
		host(true, ui.Style{}, ui.SwiftUI.Picker(ui.SwiftUIPickerProps{
			Label:             "Report interval",
			Selection:         state.interval,
			Style:             "menu",
			Options:           []ui.SwiftUIPickerOption{{Value: "day", Label: "Daily"}, {Value: "week", Label: "Weekly"}, {Value: "month", Label: "Monthly"}},
			OnSelectionChange: func(next string, _ *native.Event) { state.setInterval(next) },
		}))
		return

	case "date-picker":
		host(true, ui.Style{}, ui.SwiftUI.DatePicker(ui.SwiftUIDatePickerProps{
			Label:               "Schedule",
			Value:               state.scheduledAt,
			DisplayedComponents: "dateAndTime",
			Style:               "field",
			OnValueChange:       func(next float64, _ *native.Event) { state.setScheduledAt(next) },
		}))
		return

	case "color-picker":
		host(true, ui.Style{}, ui.SwiftUI.ColorPicker(ui.SwiftUIColorPickerProps{
			Label:             "Accent",
			Selection:         state.accent,
			OnSelectionChange: func(next string, _ *native.Event) { state.setAccent(next) },
		}))
		return

	case "gauge":
		host(ui.SwiftUIMatchContents{Vertical: true}, wide, ui.SwiftUI.Gauge(ui.SwiftUIGaugeProps{
			Label:             "Volume",
			Value:             state.volume,
			Min:               0,
			Max:               1,
			Style:             "accessoryLinearCapacity",
			CurrentValueLabel: func() string { return fmt.Sprintf("%d%%", int(state.volume()*100+0.5)) },
			MinimumValueLabel: "0%",
			MaximumValueLabel: "100%",
		}))
		return

	case "text-field":
		ui.View(
			ui.Display("flex"),
			ui.FlexDirection("column"),
			ui.Gap(16),
			ui.AlignItems("center"),
			func() {
				host(ui.SwiftUIMatchContents{Vertical: true}, field, ui.SwiftUI.TextField(ui.SwiftUITextFieldProps{
					Value:         state.name,
					Placeholder:   "Name",
					OnValueChange: func(next string, _ *native.Event) { state.setName(next) },
					OnSubmit:      func(*native.Event) { state.setSubmitted("text field") },
				}))
				ui.Input(
					ui.Value(state.name),
					ui.Placeholder("Framework input bound to the same value"),
					ui.OnInput(func(event *native.Event) {
						if text, ok := event.ValueOK(); ok {
							state.setName(text)
						}
					}),
					ui.Width(360),
					ui.Height(28),
					ui.FlexShrink(0),
					ui.PaddingLeft(8),
					ui.PaddingRight(8),
					ui.Color("#111827"),
					ui.BackgroundColor("#ffffff"),
					ui.BorderWidth(1),
					ui.BorderColor("#d1d5db"),
					ui.BorderRadius(6),
					ui.Focus(
						ui.BorderColor("#2563eb"),
						ui.Outline("3px solid #2563eb55"),
					),
				)
			},
		)
		return

	case "secure-field":
		host(ui.SwiftUIMatchContents{Vertical: true}, field, ui.SwiftUI.SecureField(ui.SwiftUITextFieldProps{
			Value:         state.password,
			Placeholder:   "Password",
			OnValueChange: func(next string, _ *native.Event) { state.setPassword(next) },
			OnSubmit:      func(*native.Event) { state.setSubmitted("secure field") },
		}))
		return

	case "popover":
		host(true, ui.Style{}, ui.SwiftUI.Popover.Root(
			ui.SwiftUIPopoverProps{
				IsPresented:         state.open,
				OnIsPresentedChange: state.setOpen,
				AttachmentAnchor:    "bottom",
				ArrowEdge:           "top",
			},
			func() {
				ui.SwiftUI.Popover.Trigger(ui.SwiftUIPopoverTriggerProps{
					Render: func() {
						ui.SwiftUI.Button(ui.SwiftUIButtonProps{
							Label:     "Open QuickGUI popover",
							Modifiers: []ui.SwiftUIModifier{ui.SwiftUI.ButtonStyle("glass"), ui.SwiftUI.ControlSize("large")},
						})
					},
				})
				ui.SwiftUI.Popover.Content(
					ui.SwiftUIPopoverContentProps{},
					func() {
						ui.SwiftUI.QuickGUIHostView(
							ui.SwiftUIQuickGUIHostViewProps{
								Width:  300,
								Height: 200,
							},
							func() {
								ui.View(
									ui.Display("flex"),
									ui.FlexDirection("column"),
									ui.Width(300),
									ui.Height("100%"),
									ui.Padding(20),
									ui.Gap(12),
									ui.OverflowY("auto"),
									ui.BackgroundColor("transparent"),
									func() {
										ui.Text(
											ui.Color("#111827"),
											ui.FontSize(15),
											ui.FlexShrink(0),
											"QuickGUI inside SwiftUI",
										)
										ui.Input(
											ui.Value(state.name),
											ui.OnInput(func(event *native.Event) {
												if text, ok := event.ValueOK(); ok {
													state.setName(text)
												}
											}),
											ui.Width("100%"),
											ui.Height(36),
											ui.FlexShrink(0),
											ui.Padding(8),
											ui.Color("#111827"),
											ui.BackgroundColor("#ffffff"),
											ui.BorderWidth(1),
											ui.BorderColor("#d1d5db"),
											ui.BorderRadius(8),
										)
										ui.Button(
											ui.OnClick(func() { state.setOpen(false) }),
											ui.Width("100%"),
											ui.Height(34),
											ui.FlexShrink(0),
											ui.Padding(8),
											ui.Color("#ffffff"),
											ui.BackgroundColor("#2563eb"),
											ui.BorderRadius(8),
											ui.JustifyContent("center"),
											func() string { return "Save " + state.name() },
										)
									},
								)
							},
						)
					},
				)
			},
		))
		return

	default:
		ui.Text("Unknown demo")
		return
	}
}
