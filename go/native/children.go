package native

import (
	"fmt"
	"reflect"
)

// Component is a function returning *Node or a frontend value implementing
// NodeProvider, such as *ui.Element. Construction callbacks must return a node.
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
		typ.NumOut() == 1 && typ.Out(0).Implements(nodeProviderType)
}

// componentBuild tracks temporary allocations only so unused nodes can be retired.
// Only the explicitly returned node is mounted.
type componentBuild struct {
	nodes []*Node
}

var currentComponentBuild *componentBuild

func collectCreatedNode(node *Node) {
	if currentComponentBuild != nil {
		currentComponentBuild.nodes = append(currentComponentBuild.nodes, node)
	}
}

// CollectChildren runs build once. A node-returning component mounts exactly its
// returned node. There is no implicit declaration or node collection mode.
func CollectChildren(build Component) []*Node {
	if build == nil {
		return nil
	}
	if !IsComponent(build) {
		panic(fmt.Sprintf("QuickGUI construction callbacks must return a native node; non-returning declarations are not supported (%T)", build))
	}
	previous := currentComponentBuild
	scope := &componentBuild{}
	currentComponentBuild = scope
	defer func() { currentComponentBuild = previous }()
	var result *Node
	switch fn := build.(type) {
	case func() *Node:
		if fn != nil {
			result = fn()
		}
	default:
		if !IsComponent(build) {
			panic(fmt.Sprintf("QuickGUI component must return a native node, got %T", build))
		}
		factory := reflect.ValueOf(build)
		// Reflection is limited to invoking a frontend factory at mount time.
		// Normal child calls and retained property updates use direct Go calls.
		if !factory.IsNil() {
			values := factory.Call(nil)
			if values[0].Interface() != nil {
				result = values[0].Interface().(NodeProvider).NativeNode()
			}
		}
	}
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
