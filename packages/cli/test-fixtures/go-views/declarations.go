package cart

import (
	"github.com/egoist/quickgui/go/ui"
)

func ValueCard(value int, visible bool, cleanup func()) {
	ui.View(
		ui.Text(value),
		ui.Show(
			visible,
			func() {
				ui.OnCleanup(cleanup)
				ui.Text(value)
			},
		),
	).Width(value*10).When(visible, ui.Style().Opacity(0.5))
}

func DeclaredNodeLabel(value int) {
	ui.Text(value)
}

func ordinaryValue(value int) int { return value }
