package ui

import (
	"bytes"
	"fmt"
	"testing"

	"github.com/egoist/quickgui/go/native"
	"github.com/egoist/quickgui/go/protocol"
	"github.com/egoist/quickgui/go/reactive"
	gui "github.com/egoist/quickgui/go/ui"
	"quickgui.example/quick-git/internal/model"
)

func TestDividerDragsFromWindowCoordinatesAndRetainsPanel(t *testing.T) {
	reactive.CreateRoot(func(dispose func()) struct{} {
		defer dispose()
		width, setWidth := gui.CreateSignal(420.0)
		theme, _ := gui.CreateSignal(ThemeFor("light"))
		var divider *native.Node
		root := captureComponent(func() {
			ProvideApp(AppContext{Theme: theme, Store: &model.Store{}}, func() {
				gui.View(
					gui.Style{Width: width},
					func() {
						gui.Text("Retained selection")
						divider = captureComponent(func() { resizeDivider("Resize history", width, func(next float64) { setWidth(next) }) })
						gui.Child(divider)
					},
				)
			})
		})
		child := root.Children[0]
		host := native.NewNodeHost(1, 1)
		host.Nodes[divider.ID] = divider
		send := func(phase, button string, x, localX float64) {
			payload := fmt.Sprintf(`{"phase":%q,"button":%q,"position":{"x":%g,"y":100},"origin":{"x":500,"y":100},"localPosition":{"x":%g,"y":50},"localOrigin":{"x":0,"y":50}}`, phase, button, x, localX)
			native.DispatchEvent(host, protocol.EventPointer, divider.ID, payload, true)
		}
		send("move", "left", 540, 40)
		if width() != 420 {
			t.Fatal("a move without a press resized the panel")
		}
		send("down", "left", 500, 0)
		offset := len(root.Pending.Body())
		send("move", "left", 540, 0)
		if width() != 460 {
			t.Fatalf("drag did not update width: %v", width())
		}
		expected := protocol.NewBatch()
		expected.SetNumber(root.ID, protocol.Width, 460)
		if !bytes.Contains(root.Pending.Body()[offset:], expected.Body()) {
			t.Fatal("drag updated state without updating native layout")
		}
		send("move", "left", 590, 0)
		send("up", "left", 600, 10)
		if width() != 520 {
			t.Fatal("moving the divider changed the drag's origin")
		}
		send("move", "left", 650, 60)
		send("down", "right", 500, 0)
		send("move", "right", 650, 60)
		if width() != 520 {
			t.Fatal("released or non-primary pointer resized the panel")
		}
		send("down", "left", 500, 0)
		send("cancel", "left", 500, 0)
		send("move", "left", 800, 50)
		if width() != 520 || root.Children[0] != child {
			t.Fatal("cancellation resized or rebuilt the panel")
		}
		return struct{}{}
	})
}
