package native

// Component declares a retained subtree once. Reactive bindings update its nodes
// without rerunning the component. A component may declare zero or more roots.
type Component = func()

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

// DeclareChild includes an existing detached node in the active children block.
func DeclareChild(node *Node) {
	if currentChildBuild == nil {
		panic("declare a child inside a QuickGUI children block")
	}
	if node != nil {
		currentChildBuild.nodes = append(currentChildBuild.nodes, node)
	}
}

// CollectChildren runs build once and returns the detached roots it declares, in
// declaration order. The UI package uses this for func() children blocks. Nodes
// already inserted into a parent or owned by a fragment are not separate roots.
func CollectChildren(build Component) []*Node {
	if build == nil {
		return nil
	}
	previous := currentChildBuild
	scope := &childBuild{}
	currentChildBuild = scope
	defer func() { currentChildBuild = previous }()
	build()

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
