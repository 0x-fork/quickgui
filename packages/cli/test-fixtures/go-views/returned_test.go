package cart

import (
	"testing"

	"github.com/egoist/quickgui/go/native"
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
		snapshot := ui.View(func() { ordinaryDeclaration(value()) })
		setValue(2)
		if node.Children[0].Text != "2" || snapshot.Children[0].Children[0].Text != "1" {
			t.Fatal("only a declared node return should create a component prop boundary")
		}
		return struct{}{}
	})
}
