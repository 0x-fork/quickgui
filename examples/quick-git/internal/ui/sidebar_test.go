package ui

import (
	"bytes"
	"github.com/egoist/quickgui/go/native"
	"github.com/egoist/quickgui/go/protocol"
	"github.com/egoist/quickgui/go/reactive"
	gui "github.com/egoist/quickgui/go/ui"
	"quickgui.example/quick-git/internal/model"
	"testing"
)

func TestSidebarSelectionMovesWithoutRemountingRows(t *testing.T) {
	reactive.CreateRoot(func(dispose func()) struct{} {
		defer dispose()
		selected, setSelected := gui.CreateSignal("Changes")
		theme, _ := gui.CreateSignal(ThemeFor("light"))
		var changes, history *native.Node
		root := captureComponent(func() *native.Node {
			return ProvideApp(AppContext{Theme: theme, Store: &model.Store{}}, func() *gui.Element {
				changes = navRow("Changes", func() bool { return selected() == "Changes" }, func() string { return "3" }, func() { setSelected("Changes") }).Node
				history = navRow("History", func() bool { return selected() == "History" }, func() string { return "" }, func() { setSelected("History") }).Node
				return gui.View().Children(changes, history)
			})
		})
		first := root.Children[0]
		offset := len(root.Pending.Body())
		for _, listener := range history.Listeners {
			if listener.Type == protocol.EventClick {
				listener.Listener(&native.Event{})
			}
		}
		expected := protocol.NewBatch()
		expected.SetColor(history.ID, protocol.BackgroundColor, native.ParseColor(theme().Selection))
		if !bytes.Contains(root.Pending.Body()[offset:], expected.Body()) {
			t.Fatal("history background did not follow selection")
		}
		expected = protocol.NewBatch()
		expected.SetColor(changes.ID, protocol.BackgroundColor, native.ParseColor("transparent"))
		if !bytes.Contains(root.Pending.Body()[offset:], expected.Body()) {
			t.Fatal("changes stayed highlighted")
		}
		if selected() != "History" || first != root.Children[0] {
			t.Fatal("selection rebuilt the sidebar")
		}
		return struct{}{}
	})
}
