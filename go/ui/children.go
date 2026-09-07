package ui

import (
	"fmt"

	"github.com/egoist/quickgui/go/native"
	"github.com/egoist/quickgui/go/reactive"
)

// Component declares a retained subtree. It has the same signature as a children
// block and can be used for window roots, routes, and conditional branches.
type Component = native.Component

// Child declares existing detached nodes or text inside a children block. Calls
// to UI constructors already declare themselves and do not need this wrapper.
func Child(value any) {
	for _, node := range childNodes(value) {
		native.DeclareChild(node)
	}
}

// withChildren keeps Props.Children compatible while allowing ordinary children
// arguments: View(props, func() { ... }) and Text(props, "Hello").
func withChildren(previous any, children []any) any {
	if len(children) == 0 {
		return previous
	}
	if previous == nil {
		return children
	}
	return []any{previous, children}
}

func withPartChildren(previous Component, children []any) Component {
	if len(children) == 0 {
		return previous
	}
	return func() {
		if previous != nil {
			previous()
		}
		Child(children)
	}
}

func renderComponent(component Component) *native.Node {
	return Fragment(native.CollectChildren(component))
}

// A block inherits the current compound context, but its effects end when its
// parent node is removed. Creating this owner inside the block preserves contexts
// that a compound root installs after constructing its own node.
func insertChildBlock(parent *native.Node, build func() []*native.Node) {
	owner := reactive.NewOwner(reactive.GetOwner())
	reactive.RunWithOwner(parent.BindingOwner(), func() struct{} {
		reactive.OnCleanup(func() { reactive.DisposeOwner(owner) })
		return struct{}{}
	})
	nodes := reactive.RunWithOwner(owner, build)
	for _, node := range nodes {
		native.InsertNode(parent, node, nil)
	}
}

func childNodes(children any) []*native.Node {
	switch child := children.(type) {
	case nil, bool:
		return nil
	case *native.Node:
		if child != nil {
			return []*native.Node{child}
		}
	case string:
		return []*native.Node{native.CreateText(child)}
	case int, int32, int64, float32, float64:
		return []*native.Node{native.CreateText(fmt.Sprint(child))}
	case func():
		if child != nil {
			return native.CollectChildren(child)
		}
	case func() string:
		return []*native.Node{DynamicText(child)}
	case reactive.Accessor[string]:
		return []*native.Node{DynamicText(child)}
	case []any:
		var nodes []*native.Node
		for _, value := range child {
			nodes = append(nodes, childNodes(value)...)
		}
		return nodes
	case []*native.Node:
		var nodes []*native.Node
		for _, node := range child {
			if node != nil {
				nodes = append(nodes, node)
			}
		}
		return nodes
	default:
		panic(fmt.Sprintf("unsupported QuickGUI child %T", children))
	}
	return nil
}
