package ui

import (
	"fmt"
	"reflect"
	"testing"

	"github.com/egoist/quickgui/go/native"
	"github.com/egoist/quickgui/go/protocol"
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
				Text(index)
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
		if got := blockText(root.Node); !reflect.DeepEqual(got, []string{"before", "second", "0", "first", "1", "after"}) {
			t.Fatalf("row roots did not move together: %v", got)
		}
		if mounts != 2 || cleanups != 0 || refs[1] != first {
			t.Fatal("reordering or updating a keyed row remounted its component")
		}
		setItems([]item{})
		setItems([]item{})
		if got := blockText(root.Node); !reflect.DeepEqual(got, []string{"before", "empty", "add an item", "after"}) || cleanups != 2 || fallbackMounts != 1 {
			t.Fatalf("empty list did not retain its fallback: %v, cleanups=%d, mounts=%d", got, cleanups, fallbackMounts)
		}
		setItems([]item{{1, "again"}})
		if got := blockText(root.Node); !reflect.DeepEqual(got, []string{"before", "again", "0", "after"}) || fallbackCleanups != 1 {
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
			}).Node
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
		if got := blockText(root.Node); !reflect.DeepEqual(got, []string{"helper", "one", "two", "end"}) {
			t.Fatal(got)
		}
		setVisible(false)
		if got := blockText(root.Node); !reflect.DeepEqual(got, []string{"helper", "end"}) {
			t.Fatal(got)
		}
		setVisible(true)
		if created != 2 || len(blockText(root.Node)) != 4 {
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
		native.InsertNode(parent.Node, root, nil)
		setValue(1)
		native.RemoveNode(parent.Node, root)
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
	if len(root.Children) != 2 || root.Children[0] != prebuilt.Node {
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
		if mounts != 1 || cleanups != 0 || !reflect.DeepEqual(blockText(root.Node), []string{"second"}) {
			t.Fatal("a child signal remounted the selected component")
		}
		setSelected(true)
		setValue("hidden update")
		if mounts != 1 || cleanups != 1 || !reflect.DeepEqual(blockText(root.Node), []string{"other view"}) {
			t.Fatal("changing views did not dispose the previous component")
		}
		setSelected(false)
		if mounts != 2 || !reflect.DeepEqual(blockText(root.Node), []string{"hidden update"}) {
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

func TestNumericChildrenRetainTextAndDisposeBindings(t *testing.T) {
	reactive.CreateRoot(func(dispose func()) struct{} {
		count, setCount := CreateSignal(1000)
		reads := 0
		node := Text("Count: ", func() int { reads++; return count() })
		prefix, number := node.Children[0], node.Children[1]
		before := node.Pending.MutationCount()
		Batch(func() { setCount(2000); setCount(3000) })
		if prefix.Text != "Count: " || number.Text != "3000" || node.Children[1] != number || reads != 2 {
			t.Fatal("numeric update remounted a child or changed the static prefix")
		}
		if got := node.Pending.MutationCount() - before; got != 1 {
			t.Fatalf("expected one numeric text mutation per batch, got %d", got)
		}
		setCount(3000)
		if node.Pending.MutationCount() != before+1 {
			t.Fatal("unchanged value emitted a mutation")
		}
		dispose()
		setCount(4000)
		if reads != 2 || number.Text != "3000" {
			t.Fatal("disposed numeric binding still runs")
		}
		return struct{}{}
	})
}

func checkScalarChild[T comparable](t *testing.T, initial, updated T, first, second string) {
	t.Helper()
	reactive.CreateRoot(func(dispose func()) struct{} {
		defer dispose()
		value, setValue := CreateSignal(initial)
		node := Text(initial, value, func() T { return value() })
		for _, child := range node.Children {
			if child.Text != first {
				t.Fatalf("%T: expected %q, got %q", initial, first, child.Text)
			}
		}
		setValue(updated)
		if node.Children[0].Text != first || node.Children[1].Text != second || node.Children[2].Text != second {
			t.Fatalf("%T: scalar/accessor/function children diverged: %v", initial, blockText(node.Node))
		}
		return struct{}{}
	})
}

func TestNumericChildTypes(t *testing.T) {
	checkScalarChild(t, int(-1), int(2), "-1", "2")
	checkScalarChild(t, int8(-128), int8(127), "-128", "127")
	checkScalarChild(t, int16(-32768), int16(32767), "-32768", "32767")
	checkScalarChild(t, int32(-2147483648), int32(2147483647), "-2147483648", "2147483647")
	checkScalarChild(t, int64(-9223372036854775808), int64(9223372036854775807), "-9223372036854775808", "9223372036854775807")
	checkScalarChild(t, uint(0), uint(123), "0", "123")
	checkScalarChild(t, uint8(0), uint8(255), "0", "255")
	checkScalarChild(t, uint16(0), uint16(65535), "0", "65535")
	checkScalarChild(t, uint32(0), uint32(4294967295), "0", "4294967295")
	checkScalarChild(t, uint64(0), uint64(18446744073709551615), "0", "18446744073709551615")
	checkScalarChild(t, uintptr(0), uintptr(123), "0", "123")
	checkScalarChild(t, float32(1.2), float32(-1.25), "1.2", "-1.25")
	checkScalarChild(t, float64(1.2), float64(-1.25), "1.2", "-1.25")
	if len(Text(nil, false, true).Children) != 0 {
		t.Fatal("boolean children must remain hidden")
	}
}

func BenchmarkCounterChildren(b *testing.B) {
	for _, formatted := range []bool{false, true} {
		name := "numeric"
		if formatted {
			name = "sprintf"
		}
		b.Run(name, func(b *testing.B) {
			reactive.CreateRoot(func(dispose func()) struct{} {
				defer dispose()
				count, setCount := CreateSignal(1000)
				var node *native.Node
				if formatted {
					node = Text(func() string { return fmt.Sprintf("Count: %d", count()) }).Node
				} else {
					node = Text("Count: ", count).Node
				}
				b.ReportAllocs()
				b.ResetTimer()
				for i := 0; i < b.N; i++ {
					setCount(1000 + i%1000)
					node.Pending = protocol.NewBatch()
				}
				b.StopTimer()
				return struct{}{}
			})
		})
	}
}
