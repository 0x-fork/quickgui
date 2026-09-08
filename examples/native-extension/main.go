package main

import (
	"log"
	"quickgui.example/native-extension/echo"

	"github.com/egoist/quickgui/go/native"
	"github.com/egoist/quickgui/go/ui"
)

func main() {
	if err := native.Run(func() {
		native.NewWindow(native.WindowOptions{
			Title:     "Native extension",
			Width:     480,
			Height:    260,
			Component: App,
		})
	}); err != nil {
		log.Fatal(err)
	}
}

func App() {
	message, setMessage := ui.CreateSignal("Hello from an independent native library")
	ui.View(
		func() {
			ui.Text(message)
			ui.Button(
				"Call extension",
				ui.OnClick(func() {
					echo.Send("The stock core loaded @acme/quickgui-echo 1.0.0", func(reply string, err error) {
						if err != nil {
							setMessage(err.Error())
							return
						}
						setMessage(reply)
					})
				}),
				ui.Padding(12),
				ui.BackgroundColor("#2563eb"),
				ui.TextColor("white"),
				ui.BorderRadius(8),
			)
		},
		ui.Display("flex"),
		ui.FlexDirection("column"),
		ui.Padding(24),
		ui.Gap(16),
	)
}
