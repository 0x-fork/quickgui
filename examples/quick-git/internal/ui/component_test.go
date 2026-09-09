package ui

import "github.com/egoist/quickgui/go/native"

// Expose the tree to tests without changing the declaration-style component API.
func captureComponent(component native.Component) *native.Node {
	nodes := native.CollectChildren(component)
	if len(nodes) != 1 {
		panic("expected one component root")
	}
	return nodes[0]
}
