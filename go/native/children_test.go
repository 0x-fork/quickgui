package native

import (
	"testing"

	"github.com/egoist/quickgui/go/protocol"
	"github.com/egoist/quickgui/go/reactive"
)

func TestWindowMountsDeclaredRootsAndDisposesBindings(t *testing.T) {
	ResetTreeStateForTests()
	signal := reactive.NewSignal("one")
	window := &Window{NodeHost: NewNodeHost(1, 2)}
	window.Root = CreateRootNode(window.NodeHost, protocol.RootNodeID)
	builds, cleanups := 0, 0
	var first, second *Node
	dispose := window.mountComponent(func() {
		builds++
		reactive.OnCleanup(func() { cleanups++ })
		first = CreateText("")
		first.Bind(func() { ReplaceText(first, signal.Read()) })
		second = CreateText("")
		second.Bind(func() { ReplaceText(second, signal.Read()) })
	})
	if len(window.Root.Children) != 2 || window.Root.Children[0] != first || window.Root.Children[1] != second || len(signal.Observers) != 2 {
		t.Fatal("the window must mount all declared roots in order")
	}
	signal.Write("two")
	if first.Text != "two" || second.Text != "two" || builds != 1 {
		t.Fatal("a property update rebuilt the root component")
	}
	dispose()
	if cleanups != 1 || len(signal.Observers) != 0 {
		t.Fatal("component ownership leaked")
	}
}

func TestEmptyDeclarationsAndConstructionScopeRestoration(t *testing.T) {
	ResetTreeStateForTests()
	var empty Component
	if len(CollectChildren(empty)) != 0 || len(CollectChildren(func() {})) != 0 {
		t.Fatal("empty components must have no roots")
	}
	func() { defer func() { _ = recover() }(); CollectChildren(func() { panic("interrupted") }) }()
	var expected *Node
	nodes := CollectChildren(func() { expected = CreateText("restored") })
	if len(nodes) != 1 || nodes[0] != expected {
		t.Fatal("the construction scope was not restored")
	}
}
