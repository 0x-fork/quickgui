package main

import (
	"fmt"

	"github.com/egoist/quickgui/go/native"
	"github.com/egoist/quickgui/go/ui"
)

func main() { run("Native alerts", 720, 520, Alerts) }

func Alerts() {
	window := native.CurrentWindow()
	status, setStatus := ui.CreateSignal("Choose an alert. Results arrive asynchronously.")
	pending, setPending := ui.CreateSignal(false)
	children := []any{ui.Text(status)}
	for _, level := range []string{"info", "warning", "critical"} {
		children = append(children, ui.Button(
			ui.WithStyle(buttonStyle),
			ui.Disabled(pending),
			ui.OnClick(func() {
				setPending(true)
				native.ShowAlertDialog(
					native.AlertDialogOptions{
						Window:  window,
						Level:   level,
						Message: "QuickGUI " + level + " alert",
						Detail:  "This sheet is presented by the native window system.",
						Buttons: []native.AlertDialogButton{{Label: "Continue", Role: "default"}, {Label: "Cancel", Role: "cancel"}},
					},
					func(index int, err error) {
						if window.Closed {
							return
						}
						setPending(false)
						if err != nil {
							setStatus(err.Error())
						} else {
							setStatus(fmt.Sprintf("Selected button: %d", index))
						}
					},
				)
			}),
			level,
		))
	}
	ui.View(ui.Display("flex"), ui.FlexDirection("column"), ui.Gap(16), children)
}
