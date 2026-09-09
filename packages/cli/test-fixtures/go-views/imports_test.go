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
		label := native.CollectChildren(func() { parts.Label(parts.Props{Name: "Mug", Quantity: quantity()}) })[0]
		first, second := label.Children[0], label.Children[1]
		before := label.Pending.MutationCount()
		setQuantity(2)
		if label.Children[0] != first || label.Children[1] != second || first.Text != "Mug" || second.Text != "2" || label.Pending.MutationCount()-before != 1 {
			t.Fatal("imported struct props did not preserve their bindings")
		}
		return struct{}{}
	})
}
