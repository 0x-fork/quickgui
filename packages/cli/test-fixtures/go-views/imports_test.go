package cart

import (
	"testing"

	parts "example.test/quickgui-views/widgets"
	"github.com/egoist/quickgui/go/native"
	"github.com/egoist/quickgui/go/reactive"
	"github.com/egoist/quickgui/go/ui"
)

func TestImportedComponentAndPlainStructProps(t *testing.T) {
	native.ResetTreeStateForTests()
	reactive.CreateRoot(func(dispose func()) struct{} {
		defer dispose()
		quantity, setQuantity := ui.CreateSignal(1)
		label := parts.Label(parts.Props{Name: "Mug", Quantity: quantity()})
		first, second := label.Node.Children[0], label.Node.Children[1]
		before := label.Pending.MutationCount()
		setQuantity(2)
		if label.Node.Children[0] != first || label.Node.Children[1] != second || first.Text != "Mug" || second.Text != "2" || label.Pending.MutationCount()-before != 1 {
			t.Fatal("imported struct props did not preserve their bindings")
		}
		return struct{}{}
	})
}
