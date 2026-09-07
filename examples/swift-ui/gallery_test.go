package main

import (
	"testing"

	"github.com/egoist/quickgui/go/native"
	"github.com/egoist/quickgui/go/protocol"
	"github.com/egoist/quickgui/go/reactive"
)

func TestNavigationReplacesControlAndPreservesSharedState(t *testing.T) {
	native.ResetTreeStateForTests()
	reactive.CreateRoot(func(dispose func()) struct{} {
		defer dispose()
		state := newGalleryState()
		roots := native.CollectChildren(func() { renderDemo(state) })
		find := func(tag uint8) *native.Node {
			var visit func(*native.Node) *native.Node
			visit = func(node *native.Node) *native.Node {
				if node.Tag == tag {
					return node
				}
				for _, child := range append(append([]*native.Node{}, node.Children...), node.Group...) {
					if found := visit(child); found != nil {
						return found
					}
				}
				return nil
			}
			for _, root := range roots {
				if found := visit(root); found != nil {
					return found
				}
			}
			return nil
		}
		if find(protocol.TagSwiftUIButton) == nil {
			t.Fatal("initial button missing")
		}
		state.setPresses(2)
		state.setPage("slider")
		slider := find(protocol.TagSwiftUISlider)
		if slider == nil || find(protocol.TagSwiftUIButton) != nil {
			t.Fatal("navigation retained the previous native control")
		}
		state.setVolume(0.7)
		if find(protocol.TagSwiftUISlider) != slider {
			t.Fatal("value update remounted slider")
		}
		state.setPage("button")
		if state.presses() != 2 || find(protocol.TagSwiftUIButton) == nil || find(protocol.TagSwiftUISlider) != nil {
			t.Fatal("navigation lost shared state or kept old control")
		}
		for _, entry := range demos {
			// Reverse hosting needs a real window and is checked in the native app.
			if entry.ID == "popover" {
				continue
			}
			state.setPage(entry.ID)
			if len(roots[0].Group) == 0 {
				t.Fatalf("empty demo %s", entry.ID)
			}
		}
		return struct{}{}
	})
}
