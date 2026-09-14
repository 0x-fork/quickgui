package cart

import (
	"testing"

	"github.com/egoist/quickgui/go/native"
	"github.com/egoist/quickgui/go/protocol"
	"github.com/egoist/quickgui/go/reactive"
	"github.com/egoist/quickgui/go/ui"
)

func TestDirectChildrenAndConditionsTrackWithoutCallbackWrappers(t *testing.T) {
	native.ResetTreeStateForTests()
	reactive.CreateRoot(func(dispose func()) struct{} {
		defer dispose()
		value, setValue := ui.CreateSignal(1)
		visible, setVisible := ui.CreateSignal(true)
		cleaned := 0
		roots := native.CollectChildren(func() *ui.Element {
			return ValueCard(value(), visible(), func() { cleaned++ })
		})
		if len(roots) != 1 {
			t.Fatal("expected one returned root")
		}
		root := roots[0]
		label, branch := root.Children[0], root.Children[1]
		before := root.Pending.MutationCount()
		setValue(2)
		if root.Children[0] != label || root.Children[1] != branch || label.Children[0].Text != "2" || branch.Children[0].Text != "2" || root.Pending.MutationCount()-before != 3 {
			t.Fatal("direct props must update two labels and one width without rebuilding children")
		}
		setVisible(false)
		if cleaned != 1 || !branch.Removed {
			t.Fatal("direct bool condition did not dispose its branch")
		}
		setVisible(true)
		if root.Children[1] == branch {
			t.Fatal("a removed branch must be constructed again")
		}
		return struct{}{}
	})
}

func TestNativeNodeReturnDefinesTheComponentBoundary(t *testing.T) {
	native.ResetTreeStateForTests()
	reactive.CreateRoot(func(dispose func()) struct{} {
		defer dispose()
		value, setValue := ui.CreateSignal(1)
		node := RawNodeLabel(value())
		initial := ordinaryValue(value())
		snapshot := ui.View().Child(ui.Text(initial))
		setValue(2)
		if node.Children[0].Text != "2" || snapshot.Node.Children[0].Children[0].Text != "1" {
			t.Fatal("only a declared node return should create a component prop boundary")
		}
		return struct{}{}
	})
}

func TestCompoundInstanceSettingsRetainDirectReactiveExpressions(t *testing.T) {
	native.ResetTreeStateForTests()
	reactive.CreateRoot(func(dispose func()) struct{} {
		defer dispose()
		open, setOpen := ui.CreateSignal(false)
		builds := 0
		var trigger *native.Node
		var changes []bool
		roots := native.CollectChildren(func() *ui.Element {
			return CompoundPopover(open(), func(value bool) { changes = append(changes, value) }, func() { builds++ }, func(node *native.Node) { trigger = node })
		})
		first := trigger
		setOpen(true)
		for _, listener := range trigger.Listeners {
			if listener.Type == protocol.EventClick {
				listener.Listener(&native.Event{Type: protocol.EventClick, Target: trigger})
			}
		}
		if builds != 1 || trigger != first || len(changes) != 1 || changes[0] || len(roots) != 1 || trigger.Children[0].Text != "some text" {
			t.Fatalf("controlled instance did not retain its props and direct text: builds=%d changes=%v", builds, changes)
		}
		return struct{}{}
	})
}

func TestAnInstanceConstructedInAnEventKeepsItsControlledBinding(t *testing.T) {
	native.ResetTreeStateForTests()
	reactive.CreateRoot(func(dispose func()) struct{} {
		defer dispose()
		open, setOpen := ui.CreateSignal(false)
		var popup *ui.Element
		var changed []bool
		parent := ui.View()
		button := CreatePopoverOnClick(open(), func(value bool) { changed = append(changed, value) }, func(node *ui.Element) { popup = node; parent.Child(node) })
		for _, listener := range button.Node.Listeners {
			if listener.Type == protocol.EventClick {
				listener.Listener(&native.Event{Type: protocol.EventClick, Target: button.Node})
			}
		}
		setOpen(true)
		trigger := popup.NativeNode().Group[0]
		for _, listener := range trigger.Listeners {
			if listener.Type == protocol.EventClick {
				listener.Listener(&native.Event{Type: protocol.EventClick, Target: trigger})
			}
		}
		if len(changed) != 1 || changed[0] {
			t.Fatalf("a freshly constructed instance captured a snapshot in an event: %v", changed)
		}
		return struct{}{}
	})
}
