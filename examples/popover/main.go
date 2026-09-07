package main

import (
	"fmt"

	"github.com/egoist/quickgui/go/native"
	"github.com/egoist/quickgui/go/ui"
)

func main() { run("Popovers", 760, 560, Popovers) }

func Popovers() {
	var system *native.Window
	var anchor *native.Node
	button("System popover", func() {
		if system != nil && !system.Closed {
			system.Close()
			return
		}
		system = native.NewWindow(native.WindowOptions{
			Anchor:     anchor,
			Title:      "System popover",
			Width:      340,
			Height:     250,
			Background: "#1e293b",
			Component: func() {
				window := native.CurrentWindow()
				content("Native system popover", window.Close)
			},
		})
		system.OnClose(func(*native.Window) { system = nil })
	}, ui.Ref(func(ref *native.Node) {
		anchor = ref
	}),
	)
	open, setOpen := ui.CreateSignal(false)
	ui.View(
		ui.Display("flex"),
		ui.FlexDirection("column"),
		ui.Gap(24),
		func() {
			ui.Text(ui.FontSize(28), "Two native surfaces")
			ui.Text("Both keep their own controls and fine-grained state.")
			ui.Child(anchor)
			ui.Popover.Root(
				ui.PopoverRootProps{
					Open:         open,
					OnOpenChange: func(value bool, _ ui.PopoverOpenChangeDetails) { setOpen(value) },
				},
				func() {
					ui.Popover.Trigger(
						ui.PopoverTriggerProps{PartProps: ui.PartProps{Style: buttonStyle}},
						func() { ui.Text("In-window popover") },
					)
					ui.Popover.Positioner(
						ui.PopoverPositionerProps{PartProps: ui.PartProps{}},
						func() {
							ui.Popover.Popup(
								ui.PopoverPopupProps{PartProps: ui.PartProps{Style: ui.Style{
									Width:  340,
									Height: 250,
								}}},
								func() {
									content("Retained popover", func() { setOpen(false) })
								},
							)
						},
					)
				},
			)
		},
	)
}

func content(title string, close func()) {
	count, setCount := ui.CreateSignal(0)
	card(ui.Text(
		ui.FontSize(22),
		title,
	), ui.Text(func() string { return fmt.Sprintf("Count: %d", count()) }), func() {
		button("Increment", func() { setCount(count() + 1) })
	}, func() {
		button("Close", close)
	})
}
