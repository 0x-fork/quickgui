package ui

import (
	"fmt"
	"strconv"

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
// arguments such as View(func() { ... }) and Text("Hello").
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
	case *Element:
		if child != nil && child.Node != nil {
			return []*native.Node{child.Node}
		}
	case *native.Node:
		if child != nil {
			return []*native.Node{child}
		}
	case string:
		return []*native.Node{native.CreateText(child)}
	case int:
		return []*native.Node{native.CreateText(strconv.Itoa(child))}
	case int8:
		return []*native.Node{native.CreateText(signedText[int8](child))}
	case int16:
		return []*native.Node{native.CreateText(signedText[int16](child))}
	case int32:
		return []*native.Node{native.CreateText(signedText[int32](child))}
	case int64:
		return []*native.Node{native.CreateText(signedText[int64](child))}
	case uint:
		return []*native.Node{native.CreateText(unsignedText[uint](child))}
	case uint8:
		return []*native.Node{native.CreateText(unsignedText[uint8](child))}
	case uint16:
		return []*native.Node{native.CreateText(unsignedText[uint16](child))}
	case uint32:
		return []*native.Node{native.CreateText(unsignedText[uint32](child))}
	case uint64:
		return []*native.Node{native.CreateText(unsignedText[uint64](child))}
	case uintptr:
		return []*native.Node{native.CreateText(unsignedText[uintptr](child))}
	case float32:
		return []*native.Node{native.CreateText(float32Text(child))}
	case float64:
		return []*native.Node{native.CreateText(float64Text(child))}
	case func():
		if child != nil {
			return native.CollectChildren(child)
		}
	case func() string:
		return []*native.Node{DynamicText(child)}
	case reactive.Accessor[string]:
		return []*native.Node{DynamicText(child)}
	case func() int:
		return []*native.Node{scalarText(child, strconv.Itoa)}
	case reactive.Accessor[int]:
		return []*native.Node{scalarText(child, strconv.Itoa)}
	case func() int8:
		return []*native.Node{scalarText(child, signedText[int8])}
	case reactive.Accessor[int8]:
		return []*native.Node{scalarText(child, signedText[int8])}
	case func() int16:
		return []*native.Node{scalarText(child, signedText[int16])}
	case reactive.Accessor[int16]:
		return []*native.Node{scalarText(child, signedText[int16])}
	case func() int32:
		return []*native.Node{scalarText(child, signedText[int32])}
	case reactive.Accessor[int32]:
		return []*native.Node{scalarText(child, signedText[int32])}
	case func() int64:
		return []*native.Node{scalarText(child, signedText[int64])}
	case reactive.Accessor[int64]:
		return []*native.Node{scalarText(child, signedText[int64])}
	case func() uint:
		return []*native.Node{scalarText(child, unsignedText[uint])}
	case reactive.Accessor[uint]:
		return []*native.Node{scalarText(child, unsignedText[uint])}
	case func() uint8:
		return []*native.Node{scalarText(child, unsignedText[uint8])}
	case reactive.Accessor[uint8]:
		return []*native.Node{scalarText(child, unsignedText[uint8])}
	case func() uint16:
		return []*native.Node{scalarText(child, unsignedText[uint16])}
	case reactive.Accessor[uint16]:
		return []*native.Node{scalarText(child, unsignedText[uint16])}
	case func() uint32:
		return []*native.Node{scalarText(child, unsignedText[uint32])}
	case reactive.Accessor[uint32]:
		return []*native.Node{scalarText(child, unsignedText[uint32])}
	case func() uint64:
		return []*native.Node{scalarText(child, unsignedText[uint64])}
	case reactive.Accessor[uint64]:
		return []*native.Node{scalarText(child, unsignedText[uint64])}
	case func() uintptr:
		return []*native.Node{scalarText(child, unsignedText[uintptr])}
	case reactive.Accessor[uintptr]:
		return []*native.Node{scalarText(child, unsignedText[uintptr])}
	case func() float32:
		return []*native.Node{scalarText(child, float32Text)}
	case reactive.Accessor[float32]:
		return []*native.Node{scalarText(child, float32Text)}
	case func() float64:
		return []*native.Node{scalarText(child, float64Text)}
	case reactive.Accessor[float64]:
		return []*native.Node{scalarText(child, float64Text)}
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
	case []*Element:
		var nodes []*native.Node
		for _, element := range child {
			if element != nil && element.Node != nil {
				nodes = append(nodes, element.Node)
			}
		}
		return nodes
	default:
		panic(fmt.Sprintf("unsupported QuickGUI child %T", children))
	}
	return nil
}

// The formatter is selected once when mounting. Updates use the scalar's typed
// strconv conversion and retain the same text node.
func scalarText[T any](read func() T, format func(T) string) *native.Node {
	return DynamicText(func() string { return format(read()) })
}

func signedText[T ~int8 | ~int16 | ~int32 | ~int64](value T) string {
	return strconv.FormatInt(int64(value), 10)
}

func unsignedText[T ~uint | ~uint8 | ~uint16 | ~uint32 | ~uint64 | ~uintptr](value T) string {
	return strconv.FormatUint(uint64(value), 10)
}

func float32Text(value float32) string { return strconv.FormatFloat(float64(value), 'g', -1, 32) }
func float64Text(value float64) string { return strconv.FormatFloat(value, 'g', -1, 64) }
