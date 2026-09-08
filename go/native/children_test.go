package native

import (
	"testing"

	"github.com/egoist/quickgui/go/protocol"
	"github.com/egoist/quickgui/go/reactive"
)

type returnedView struct{ node *Node }

func (view *returnedView) NativeNode() *Node {
	if view == nil {
		return nil
	}
	return view.node
}

func TestWindowMountsReturnedNodeAndReleasesUnusedDeclarations(t *testing.T) {
	ResetTreeStateForTests()
	signal := reactive.NewSignal("one")
	window := &Window{NodeHost: NewNodeHost(1, 2)}
	window.Root = CreateRootNode(window.NodeHost, protocol.RootNodeID)
	builds, cleanups := 0, 0
	var returned, unused *Node
	dispose := window.mountComponent(func() *returnedView {
		builds++
		reactive.OnCleanup(func() { cleanups++ })
		unused = CreateText("")
		unused.Bind(func() { ReplaceText(unused, signal.Read()) })
		returned = CreateText("")
		returned.Bind(func() { ReplaceText(returned, signal.Read()) })
		return &returnedView{returned}
	})
	if len(window.Root.Children) != 1 || window.Root.Children[0] != returned || !unused.Removed || len(signal.Observers) != 1 {
		t.Fatal("the window must mount only the returned node and dispose unused bindings")
	}
	signal.Write("two")
	if returned.Text != "two" || builds != 1 {
		t.Fatal("a property update rebuilt the root component")
	}
	dispose()
	if cleanups != 1 || len(signal.Observers) != 0 {
		t.Fatal("returned component ownership leaked")
	}
}

func TestReturnedNodesSupportEmptyResultsAndRestoreConstructionAfterPanic(t *testing.T) {
	ResetTreeStateForTests()
	var empty func() *Node
	if len(CollectChildren(empty)) != 0 || len(CollectChildren(func() *returnedView { return nil })) != 0 {
		t.Fatal("nil component results must be empty")
	}
	func() { defer func() { _ = recover() }(); CollectChildren(func() *Node { panic("interrupted") }) }()
	var expected *Node
	nodes := CollectChildren(func() *Node { expected = CreateText("restored"); return expected })
	if len(nodes) != 1 || nodes[0] != expected {
		t.Fatal("the construction scope was not restored")
	}
}
