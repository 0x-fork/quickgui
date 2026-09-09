package cart

import (
	"testing"

	"github.com/egoist/quickgui/go/native"
	"github.com/egoist/quickgui/go/reactive"
	"github.com/egoist/quickgui/go/ui"
)

func TestPlainPropsRetainNodesAndDisposeBindings(t *testing.T) {
	native.ResetTreeStateForTests()
	builds = 0
	reactive.CreateRoot(func(dispose func()) struct{} {
		defer dispose()
		quantity, setQuantity := ui.CreateSignal(1)
		product, setProduct := ui.CreateSignal(Product{ID: "mug", Name: "Mug", Price: 12})
		item := native.CollectChildren(func() { ForwardedItem(product(), quantity(), func() { setQuantity(quantity() + 1) }) })[0]
		parent := ui.View(item)
		labels := append([]*native.Node(nil), item.Children...)
		check := func(want []string) {
			t.Helper()
			for i, text := range want {
				if item.Children[i] != labels[i] || labels[i].Children[0].Text != text {
					t.Fatalf("label %d: got %q, want %q, retained=%v", i, labels[i].Children[0].Text, text, item.Children[i] == labels[i])
				}
			}
		}
		check([]string{"Mug", "1", "12", "1"})
		before := parent.Pending.MutationCount()
		setQuantity(3)
		check([]string{"Mug", "3", "36", "1"})
		if builds != 1 || parent.Pending.MutationCount()-before != 3 {
			t.Fatalf("builds=%d, mutations=%d; want one build and two text/one width mutations", builds, parent.Pending.MutationCount()-before)
		}
		before = parent.Pending.MutationCount()
		setProduct(Product{ID: "cup", Name: "Cup", Price: 10})
		check([]string{"Cup", "3", "30", "1"})
		if parent.Pending.MutationCount()-before != 2 {
			t.Fatal("unrelated props mutated")
		}
		native.RemoveNode(parent.Node, item)
		before = parent.Pending.MutationCount()
		setQuantity(4)
		if parent.Pending.MutationCount() != before {
			t.Fatal("removed component is still subscribed")
		}
		return struct{}{}
	})
}

func TestPropShadowingAndFirstClassSnapshot(t *testing.T) {
	native.ResetTreeStateForTests()
	reactive.CreateRoot(func(dispose func()) struct{} {
		defer dispose()
		value, setValue := ui.CreateSignal("initial")
		label := native.CollectChildren(func() { Label(value()) })[0]
		build := Label
		snapshot := native.CollectChildren(func() { build(value()) })[0]
		shadowed := func(Label func() string) string { return Label() }(value)
		if shadowed != "initial" {
			t.Fatal("a callback shadowing a component changed meaning")
		}
		setValue("updated")
		if label.Children[0].Children[0].Text != "updated" || label.Children[1].Children[0].Text != "initial" || label.Children[2].Children[0].Children[0].Text != "shadowed" {
			t.Fatal("prop scope changed")
		}
		if snapshot.Children[0].Children[0].Text != "initial" {
			t.Fatal("ordinary function values must retain ordinary evaluation")
		}
		return struct{}{}
	})
}

func TestEventTimeMutationDoesNotBecomeAPersistentBinding(t *testing.T) {
	native.ResetTreeStateForTests()
	reactive.CreateRoot(func(dispose func()) struct{} {
		defer dispose()
		value, setValue := ui.CreateSignal("clicked")
		node := native.CollectChildren(func() { EventSnapshot(value()) })[0]
		node.Children[1].Listeners[0].Listener(&native.Event{})
		before := node.Pending.MutationCount()
		setValue("later")
		if node.Pending.MutationCount() != before {
			t.Fatal("event-time setter installed a persistent binding")
		}
		return struct{}{}
	})
}
