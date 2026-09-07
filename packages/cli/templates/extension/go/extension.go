// Package {{GO_PACKAGE}} provides reusable QuickGUI components in pure Go.
package {{GO_PACKAGE}}

import "github.com/egoist/quickgui/go/ui"

// Notice displays a reactive message. Caller styles merge with the defaults.
func Notice(message func() string, style ui.Style) {
	ui.Text(
		message,
		ui.Style{
			Padding:         16,
			BorderRadius:    8,
			BackgroundColor: "#eff6ff",
			Color:           "#1e40af",
		},
		style,
	)
}
