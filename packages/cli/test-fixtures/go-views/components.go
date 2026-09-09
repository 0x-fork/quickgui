package cart

import (
	"github.com/egoist/quickgui/go/native"
	"github.com/egoist/quickgui/go/ui"
)

func ValueCard(value int, visible bool, cleanup func()) *ui.Element {
	return ui.View(
		ui.Text(value),
		ui.Show(
			visible,
			func() *ui.Element {
				ui.OnCleanup(cleanup)
				return ui.Text(value)
			},
		),
	).Width(value*10).When(visible, ui.Style().Opacity(0.5))
}

func RawNodeLabel(value int) *native.Node {
	return ui.Text(value).Node
}

func ordinaryValue(value int) int { return value }
