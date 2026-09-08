// Package {{GO_PACKAGE}} provides reusable QuickGUI components in pure Go.
package {{GO_PACKAGE}}

import "github.com/egoist/quickgui/go/ui"

// Notice displays a reactive message. Caller styles merge with the defaults.
func Notice(message func() string, styles ...ui.StyleDeclaration) {
	ui.Text(
		message,
		ui.Padding(16),
		ui.BorderRadius(8),
		ui.BackgroundColor("#eff6ff"),
		ui.TextColor("#1e40af"),
		ui.Styles(styles...),
	)
}
