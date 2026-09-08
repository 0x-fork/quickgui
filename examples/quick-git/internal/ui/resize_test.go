package ui

import (
	"bytes"
	"testing"

	"github.com/egoist/quickgui/go/native"
	"github.com/egoist/quickgui/go/protocol"
	"github.com/egoist/quickgui/go/reactive"
	gui "github.com/egoist/quickgui/go/ui"
	"quickgui.example/quick-git/internal/model"
)

func TestNativeResizeReportsSizesWithoutRebuildingPanel(t *testing.T) {
	reactive.CreateRoot(func(dispose func()) struct{} {
		defer dispose()
		width, setWidth := gui.CreateSignal(420.0)
		theme, _ := gui.CreateSignal(ThemeFor("light"))
		root := captureComponent(func() {
			ProvideApp(AppContext{Theme: theme, Store: &model.Store{}}, func() {
				resizablePanel("Resize history", width, func(next float64) { setWidth(next) }, 260, 1000,
					func() { gui.Text("Retained selection") }, gui.Style(),
				)
			})
		})
		pane, divider := root.Children[0], root.Children[1]
		child := pane.Children[0]
		for _, node := range []*native.Node{root, pane, divider} {
			for _, listener := range node.Listeners {
				if listener.Type == protocol.EventPointer {
					t.Fatal("resizing must not depend on a Go pointer listener")
				}
			}
		}
		expected := protocol.NewBatch()
		expected.SetString(root.ID, protocol.Values, "[420,580]")
		if !bytes.Contains(root.Pending.Body(), expected.Body()) {
			t.Fatal("the native splitter did not receive the saved width and maximum")
		}
		host := native.NewNodeHost(1, 1)
		host.Nodes[root.ID] = root
		offset := len(root.Pending.Body())
		native.DispatchEvent(
			host,
			protocol.EventComponentChange,
			root.ID,
			`{"sizes":[460,540]}`,
			true,
		)
		if width() != 460 {
			t.Fatalf("native resize did not update the saved width: %v", width())
		}
		expected = protocol.NewBatch()
		expected.SetString(root.ID, protocol.Values, "[460,540]")
		if !bytes.Contains(root.Pending.Body()[offset:], expected.Body()) {
			t.Fatal("the native sizes were not acknowledged")
		}
		if root.Children[0] != pane || root.Children[1] != divider || pane.Children[0] != child {
			t.Fatal("resizing rebuilt the panel or its selection")
		}
		return struct{}{}
	})
}
