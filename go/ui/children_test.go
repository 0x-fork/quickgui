package ui

import (
	"fmt"
	"reflect"
	"testing"

	"github.com/egoist/quickgui/go/native"
	"github.com/egoist/quickgui/go/reactive"
)

func TestKeyedComponentsRetainMultipleRootsAndDisposeRemovedRows(t *testing.T) {
	reactive.CreateRoot(func(dispose func()) struct{} {
		type item struct {
			ID    int
			Label string
		}
		items, setItems := CreateSignal([]item{{1, "one"}, {2, "two"}})
		mounts, cleanups, fallbackMounts, fallbackCleanups := 0, 0, 0, 0
		refs := map[int]*native.Node{}
		root := View(func() {
			Text("before")
			KeyedFor(items, func(i item) any { return i.ID }, func(read func() item, index func() int) {
				mounts++
				id := read().ID
				OnCleanup(func() { cleanups++ })
				Text(Ref(func(node *native.Node) { refs[id] = node }), func() string { return read().Label })
				Text(func() string { return fmt.Sprint(index()) })
			}, func() {
				fallbackMounts++
				OnCleanup(func() { fallbackCleanups++ })
				Text("empty")
				Text("add an item")
			})
			Text("after")
		})
		first := refs[1]
		setItems([]item{{2, "second"}, {1, "first"}})
		if got := blockText(root); !reflect.DeepEqual(got, []string{"before", "second", "0", "first", "1", "after"}) {
			t.Fatalf("row roots did not move together: %v", got)
		}
		if mounts != 2 || cleanups != 0 || refs[1] != first {
			t.Fatal("reordering or updating a keyed row remounted its component")
		}
		setItems([]item{})
		setItems([]item{})
		if got := blockText(root); !reflect.DeepEqual(got, []string{"before", "empty", "add an item", "after"}) || cleanups != 2 || fallbackMounts != 1 {
			t.Fatalf("empty list did not retain its fallback: %v, cleanups=%d, mounts=%d", got, cleanups, fallbackMounts)
		}
		setItems([]item{{1, "again"}})
		if got := blockText(root); !reflect.DeepEqual(got, []string{"before", "again", "0", "after"}) || fallbackCleanups != 1 {
			t.Fatalf("fallback leaked into restored rows: %v, cleanups=%d", got, fallbackCleanups)
		}
		dispose()
		if mounts != 3 || cleanups != 3 {
			t.Fatalf("each row must be disposed once: %d mounts, %d cleanups", mounts, cleanups)
		}
		return struct{}{}
	})
}

func TestChildrenBlocksPreserveNestingAndFineGrainedUpdates(t *testing.T) {
	reactive.CreateRoot(func(dispose func()) struct{} {
		defer dispose()
		value, setValue := CreateSignal("first")
		mounts := 0
		var nested *native.Node
		root := View(Props{}, func() {
			mounts++
			Text(Props{}, "before")
			nested = View(Props{}, func() {
				mounts++
				Text(Props{}, value)
				Button(Props{}, "Increment")
			})
			Text(Props{}, "after")
		})
		before := root.Pending.MutationCount()
		Batch(func() { setValue("second"); setValue("third") })
		if mounts != 2 || len(root.Children) != 3 || root.Children[1] != nested || len(nested.Children) != 2 {
			t.Fatal("a block reran or its children escaped their parent")
		}
		if root.Pending.MutationCount()-before != 1 || nested.Children[0].Children[0].Text != "third" {
			t.Fatal("a signal must update only its bound text once per batch")
		}
		return struct{}{}
	})
}

func TestChildrenBlocksHandleLazyRegionsAndComponentHelpers(t *testing.T) {
	reactive.CreateRoot(func(dispose func()) struct{} {
		defer dispose()
		visible, setVisible := CreateSignal(true)
		created := 0
		helper := func() { Text(Props{}, "helper") }
		root := View(Props{}, func() {
			helper()
			Show(visible, func() {
				created++
				Text(Props{}, "one")
				Text(Props{}, "two")
			})
			Text(Props{}, "end")
		})
		if got := blockText(root); !reflect.DeepEqual(got, []string{"helper", "one", "two", "end"}) {
			t.Fatal(got)
		}
		setVisible(false)
		if got := blockText(root); !reflect.DeepEqual(got, []string{"helper", "end"}) {
			t.Fatal(got)
		}
		setVisible(true)
		if created != 2 || len(blockText(root)) != 4 {
			t.Fatal("lazy children were duplicated or failed to remount")
		}
		return struct{}{}
	})
}

func TestCompoundChildrenInheritContextAndDisposeEffects(t *testing.T) {
	reactive.CreateRoot(func(dispose func()) struct{} {
		defer dispose()
		value, setValue := CreateSignal(0)
		effects, cleanups := 0, 0
		parent := View(Props{})
		root := Tabs.Root(TabsRootProps{DefaultValue: "one"}, func() {
			Tabs.List(PartProps{}, func() { Tabs.Tab(TabsTabProps{Value: "one"}, "One") })
			Tabs.Panel(TabsPanelProps{Value: "one"}, func() {
				CreateRenderEffect(func() { _ = value(); effects++ })
				OnCleanup(func() { cleanups++ })
				Text(Props{}, "panel")
			})
		})
		native.InsertNode(parent, root, nil)
		setValue(1)
		native.RemoveNode(parent, root)
		setValue(2)
		if effects != 2 || cleanups != 1 {
			t.Fatalf("effects=%d cleanups=%d", effects, cleanups)
		}
		return struct{}{}
	})
}

func TestChildrenBlockRestoresBuilderAfterPanic(t *testing.T) {
	func() {
		defer func() { _ = recover() }()
		View(Props{}, func() { panic("interrupted build") })
	}()
	prebuilt := Text(Props{}, "existing")
	root := View(Props{}, func() {
		Child(prebuilt)
		Child(Text(Props{}, "new"))
	})
	if len(root.Children) != 2 || root.Children[0] != prebuilt {
		t.Fatal("the builder leaked or declared a child twice")
	}
}

func TestDynamicViewSelectionPreservesChildBindings(t *testing.T) {
	reactive.CreateRoot(func(dispose func()) struct{} {
		selected, setSelected := CreateSignal(false)
		value, setValue := CreateSignal("first")
		mounts, cleanups := 0, 0
		component := func() {
			mounts++
			OnCleanup(func() { cleanups++ })
			Text(value)
		}
		root := View(func() {
			Dynamic(func() Component {
				if selected() {
					return func() { Text("other view") }
				}
				return component
			})
		})
		setValue("second")
		if mounts != 1 || cleanups != 0 || !reflect.DeepEqual(blockText(root), []string{"second"}) {
			t.Fatal("a child signal remounted the selected component")
		}
		setSelected(true)
		setValue("hidden update")
		if mounts != 1 || cleanups != 1 || !reflect.DeepEqual(blockText(root), []string{"other view"}) {
			t.Fatal("changing views did not dispose the previous component")
		}
		setSelected(false)
		if mounts != 2 || !reflect.DeepEqual(blockText(root), []string{"hidden update"}) {
			t.Fatal("returning to a view did not mount it with current state")
		}
		dispose()
		if cleanups != 2 {
			t.Fatalf("expected one cleanup per mount, got %d", cleanups)
		}
		return struct{}{}
	})
}

func blockText(node *native.Node) []string {
	var values []string
	if node.Text != "" {
		values = append(values, node.Text)
	}
	for _, child := range node.Children {
		values = append(values, blockText(child)...)
	}
	return values
}
