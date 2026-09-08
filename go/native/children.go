package native

import (
	"fmt"
	"reflect"
)

// Component is a function returning *Node or a frontend value implementing
// NodeProvider, such as *ui.Element. A func() declaration block also works.
// Components execute once; retained bindings update their nodes independently.
type Component = any

// NodeProvider lets frontend builders expose their retained native node without
// making the native package depend on a particular frontend.
type NodeProvider interface{ NativeNode() *Node }

func (node *Node) NativeNode() *Node { return node }

var nodeProviderType = reflect.TypeFor[NodeProvider]()

// IsComponent reports whether value is a supported component factory.
func IsComponent(value any) bool {
	if value == nil {
		return false
	}
	typ := reflect.TypeOf(value)
	return typ.Kind() == reflect.Func && typ.NumIn() == 0 &&
		(typ.NumOut() == 0 || (typ.NumOut() == 1 && typ.Out(0).Implements(nodeProviderType)))
}

// childBuild exists only while a declarative children block runs on the UI goroutine.
// Nested blocks collect independently, so their descendants cannot become siblings.
type childBuild struct {
	nodes []*Node
}

var currentChildBuild *childBuild

func collectCreatedNode(node *Node) {
	if currentChildBuild != nil {
		currentChildBuild.nodes = append(currentChildBuild.nodes, node)
	}
}

// DeclareReturnedNode publishes an already-built component root to an enclosing
// declaration block, just as creating a node there would. Direct composition
// does not need an active block and receives the same node unchanged.
func DeclareReturnedNode(node *Node) *Node {
	if node != nil {
		collectCreatedNode(node)
	}
	return node
}

// DeclareChild includes an existing detached node in the active children block.
func DeclareChild(node *Node) {
	if currentChildBuild == nil {
		panic("declare a child inside a QuickGUI children block")
	}
	if node != nil {
		currentChildBuild.nodes = append(currentChildBuild.nodes, node)
	}
}

// CollectChildren runs build once. A node-returning component mounts exactly its
// returned node. Declaration blocks collect detached roots in declaration order.
func CollectChildren(build Component) []*Node {
	if build == nil {
		return nil
	}
	previous := currentChildBuild
	scope := &childBuild{}
	currentChildBuild = scope
	defer func() { currentChildBuild = previous }()
	var result *Node
	returned := false
	switch fn := build.(type) {
	case func():
		if fn != nil {
			fn()
		}
	case func() *Node:
		returned = true
		if fn != nil {
			result = fn()
		}
	default:
		if !IsComponent(build) {
			panic(fmt.Sprintf("QuickGUI component must return a native node, got %T", build))
		}
		factory := reflect.ValueOf(build)
		returned = factory.Type().NumOut() == 1
		// Reflection is limited to invoking a frontend factory at mount time.
		// Normal child calls and retained property updates use direct Go calls.
		if !factory.IsNil() {
			values := factory.Call(nil)
			if returned && values[0].Interface() != nil {
				result = values[0].Interface().(NodeProvider).NativeNode()
			}
		}
	}
	if returned {
		keep := make(map[*Node]bool)
		var retain func(*Node)
		retain = func(node *Node) {
			if node == nil || keep[node] {
				return
			}
			keep[node] = true
			for _, child := range node.Children {
				retain(child)
			}
			for _, child := range node.Group {
				retain(child)
			}
		}
		retain(result)
		if result != nil {
			for parent := result.Parent; parent != nil; parent = parent.Parent {
				keep[parent] = true
			}
		}
		for _, node := range scope.nodes {
			if !keep[node] && node.Parent == nil && node.Host == nil {
				retire(node)
			}
		}
		if result == nil {
			return nil
		}
		if result.Removed {
			panic("a removed node cannot be returned from a component")
		}
		return []*Node{result}
	}

	grouped := make(map[*Node]bool)
	for _, node := range scope.nodes {
		for _, child := range node.Group {
			grouped[child] = true
		}
	}
	roots := make([]*Node, 0, len(scope.nodes))
	seen := make(map[*Node]bool)
	for _, node := range scope.nodes {
		if node.Parent == nil && node.Host == nil && !node.Removed && !grouped[node] && !seen[node] {
			roots = append(roots, node)
			seen[node] = true
		}
	}
	return roots
}
