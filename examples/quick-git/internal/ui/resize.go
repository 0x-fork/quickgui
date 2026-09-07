package ui

import (
	"github.com/egoist/quickgui/go/native"
	gui "github.com/egoist/quickgui/go/ui"
)

func resizeDivider(label string, width func() float64, setWidth func(float64)) {
	start := 0.0
	dragging := false
	gui.View(
		gui.AriaLabel(label),
		gui.HitSlopLeft(4),
		gui.HitSlopRight(4),
		gui.Width(1),
		gui.FlexShrink(0),
		gui.Cursor("col-resize"),
		gui.AppRegion("no-drag"),
		gui.BackgroundColor(UseApp().Theme().Border),
		gui.OnPointer(func(event *native.Event) {
			pointer := gui.CapturedPointerFromEvent(event)
			if pointer == nil || pointer.Button != "left" {
				return
			}
			switch pointer.Phase {
			case "down":
				start = width()
				dragging = true
			case "move", "up":
				if dragging {
					setWidth(start + pointer.Position.X - pointer.Origin.X)
				}
				if pointer.Phase == "up" {
					dragging = false
				}
			case "cancel":
				dragging = false
			}
		}),
	)
}
