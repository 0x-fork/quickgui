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
		item := ForwardedItem(product(), quantity(), func() { setQuantity(quantity() + 1) })
		parent := ui.View().Child(item)
		labels := append([]*native.Node(nil), item.Node.Children...)
		check := func(want []string) {
			t.Helper()
			for i, text := range want {
				if item.Node.Children[i] != labels[i] || labels[i].Children[0].Text != text {
					t.Fatalf("label %d: got %q, want %q, retained=%v", i, labels[i].Children[0].Text, text, item.Node.Children[i] == labels[i])
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
		native.RemoveNode(parent.Node, item.Node)
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
		label := Label(value())
		build := Label
		snapshot := build(value())
		shadowed := func(Label func() string) string { return Label() }(value)
		if shadowed != "initial" {
			t.Fatal("a callback shadowing a component changed meaning")
		}
		setValue("updated")
		if label.Node.Children[0].Children[0].Text != "updated" || label.Node.Children[1].Children[0].Text != "initial" || label.Node.Children[2].Children[0].Children[0].Text != "shadowed" {
			t.Fatal("prop scope changed")
		}
		if snapshot.Node.Children[0].Children[0].Text != "initial" {
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
		node := EventSnapshot(value())
		node.Node.Children[1].Listeners[0].Listener(&native.Event{})
		before := node.Pending.MutationCount()
		setValue("later")
		if node.Pending.MutationCount() != before {
			t.Fatal("event-time setter installed a persistent binding")
		}
		return struct{}{}
	})
}

func TestFluentChildrenRetainDirectExpressionsAndFactories(t *testing.T) {
	native.ResetTreeStateForTests()
	fluentBuilds, fluentChildBuilds = 0, 0
	reactive.CreateRoot(func(dispose func()) struct{} {
		defer dispose()
		value, setValue := ui.CreateSignal(2)
		view := FluentChildren(value())
		parent := ui.View().Child(view)
		children := append([]*native.Node(nil), view.Node.Children...)
		check := func(first, doubled string) {
			t.Helper()
			for i, child := range children {
				if view.Node.Children[i] != child {
					t.Fatal("a fluent child expression replaced its node")
				}
			}
			if children[0].Text != first || children[1].Text != " / " || children[2].Text != doubled || children[3].Children[0].Children[0].Text != first {
				t.Fatal("fluent child expressions or construction callbacks lost live props")
			}
		}
		check("2", "4")
		before := parent.Pending.MutationCount()
		ui.Batch(func() { setValue(3); setValue(4) })
		check("4", "8")
		if fluentBuilds != 1 || fluentChildBuilds != 1 || parent.Pending.MutationCount()-before != 4 {
			t.Fatal("fluent children must update only three text bindings and one width binding")
		}
		native.RemoveNode(parent.Node, view.Node)
		before = parent.Pending.MutationCount()
		setValue(5)
		if parent.Pending.MutationCount() != before {
			t.Fatal("removed fluent children retained their bindings")
		}
		return struct{}{}
	})
}
