package main

import (
	"encoding/json"

	"github.com/egoist/quickgui/go/native"
	"github.com/egoist/quickgui/go/ui"
)

func main() { run("System APIs", 900, 680, SystemAPIs) }

func SystemAPIs() {
	window := native.CurrentWindow()
	result, setResult := ui.CreateSignal("Choose a native service.")
	var buttons []any
	for _, method := range []string{"get-app-info", "get-app-paths", "get-system-info", "get-displays", "get-system-preferences", "get-keyboard-layout", "read-clipboard"} {
		buttons = append(buttons, func() {
			button(method, func() {
				payload, _ := json.Marshal(map[string]any{"method": method})
				native.SendCommand(
					string(payload),
					func(raw string, err error) {
						if window.Closed {
							return
						}
						if err != nil {
							setResult(err.Error())
							return
						}
						var value any
						if json.Unmarshal([]byte(raw), &value) == nil {
							pretty, _ := json.MarshalIndent(value, "", "  ")
							raw = string(pretty)
						}
						setResult(raw)
					},
				)
			})
		})
	}
	ui.View(
		ui.Display("flex"),
		ui.FlexDirection("column"),
		ui.Flex(1),
		ui.Gap(20),
		func() {
			ui.View(ui.Display("flex"), ui.FlexWrap("wrap"), ui.Gap(8), buttons)
			ui.View(
				ui.Flex(1),
				ui.OverflowY("scroll"),
				ui.Padding(20),
				ui.BackgroundColor("#151e30"),
				func() {
					ui.Text(
						ui.FontFamily("monospace"),
						ui.FontSize(12),
						ui.UserSelect("text"),
						result,
					)
				},
			)
		},
	)
}
