package ui

import (
	"fmt"
	"reflect"
	"strings"
	"testing"

	"github.com/egoist/quickgui/go/native"
	"github.com/egoist/quickgui/go/reactive"
)

func TestReturningChildFactoriesAreUnsupported(t *testing.T) {
	type namedFactory func() *Element
	for name, factory := range map[string]any{
		"element": func() *Element { return Text("unused") },
		"node":    func() *native.Node { return native.CreateText("unused") },
		"named":   namedFactory(func() *Element { return Text("unused") }),
	} {
		t.Run(name, func(t *testing.T) {
			defer func() {
				failure := recover()
				if failure == nil || !strings.Contains(fmt.Sprint(failure), "unsupported QuickGUI child") {
					t.Fatalf("returning factory was accepted: %v", failure)
				}
			}()
			View(factory)
		})
	}
}

func TestDeclaredChildrenRetainContextAndCleanupWithoutWrapperNodes(t *testing.T) {
	native.ResetTreeStateForTests()
	reactive.CreateRoot(func(dispose func()) struct{} {
		defer dispose()
		value, setValue := CreateSignal("one")
		context := reactive.CreateContext("outside")
		builds, cleaned := 0, 0
		var child *Element
		root := reactive.Provide(context, "inside", func() *Element {
			return View(func() {
				builds++
				OnCleanup(func() { cleaned++ })
				child = Text(context.Use(), value)
			})
		})
		if len(root.Children) != 1 || root.Children[0] != child.Node {
			t.Fatal("declaring a child added a wrapper node")
		}
		setValue("two")
		if builds != 1 || !reflect.DeepEqual(blockText(root.Node), []string{"inside", "two"}) {
			t.Fatal("declared child lost context or remounted")
		}
		parent := View(root)
		native.RemoveNode(parent.Node, root.Node)
		if cleaned != 1 {
			t.Fatal("child factory cleanup did not follow its parent")
		}
		return struct{}{}
	})
}

func TestDeclaredConditionalAndKeyedComponentsKeepIdentity(t *testing.T) {
	native.ResetTreeStateForTests()
	reactive.CreateRoot(func(dispose func()) struct{} {
		defer dispose()
		type item struct {
			ID   int
			Name string
		}
		items, setItems := CreateSignal([]item{{1, "one"}, {2, "two"}})
		visible, setVisible := CreateSignal(true)
		mounted, cleaned := 0, 0
		refs := map[int]*native.Node{}
		root := View(Show(visible, func() {
			View(KeyedFor(items, func(value item) any { return value.ID }, func(read func() item, _ func() int) {
				mounted++
				OnCleanup(func() { cleaned++ })
				node := Text(func() string { return read().Name })
				refs[read().ID] = node.Node
			}, nil))
		}))
		first := refs[1]
		setItems([]item{{2, "second"}, {1, "first"}})
		if mounted != 2 || refs[1] != first || !reflect.DeepEqual(blockText(root.Node), []string{"second", "first"}) {
			t.Fatal("keyed rows lost identity")
		}
		setVisible(false)
		if cleaned != 2 {
			t.Fatal("lazy rows leaked their owners")
		}
		setVisible(true)
		if mounted != 4 {
			t.Fatal("the branch did not remount")
		}
		return struct{}{}
	})
}

func TestProviderChildrenParticipateInDeclarationBlocks(t *testing.T) {
	native.ResetTreeStateForTests()
	reactive.CreateRoot(func(dispose func()) struct{} {
		defer dispose()
		var child *Element
		roots := native.CollectChildren(func() {
			Toast.Provider(ToastProviderProps{}, func() {
				child = Text("provider child")
			})
		})
		if len(roots) != 1 || len(roots[0].Group) != 1 || roots[0].Group[0] != child.Node {
			t.Fatal("a provider child disappeared from its enclosing declaration block")
		}
		return struct{}{}
	})
}
