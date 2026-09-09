package main

import (
	"strings"
	"testing"

	"github.com/egoist/quickgui/go/native"
	"github.com/egoist/quickgui/go/protocol"
	"github.com/egoist/quickgui/go/reactive"
	"github.com/egoist/quickgui/go/ui"
)

func checkboxTestRoot(t *testing.T, component ui.Component, check func(*native.Node)) {
	t.Helper()
	native.ResetTreeStateForTests()
	reactive.CreateRoot(func(dispose func()) struct{} {
		defer dispose()
		roots := native.CollectChildren(component)
		if len(roots) != 1 {
			t.Fatalf("expected one root, got %d", len(roots))
		}
		check(roots[0])
		return struct{}{}
	})
}

func checkboxText(node *native.Node) string {
	text := node.Text
	for _, child := range node.Children {
		text += checkboxText(child)
	}
	return text
}

func checkboxFind(root *native.Node, match func(*native.Node) bool) *native.Node {
	if match(root) {
		return root
	}
	for _, child := range root.Children {
		if found := checkboxFind(child, match); found != nil {
			return found
		}
	}
	return nil
}

func checkboxWithLabel(t *testing.T, root *native.Node, label string) *native.Node {
	t.Helper()
	node := checkboxFind(root, func(node *native.Node) bool {
		return node.Tag == protocol.TagButton && strings.Contains(checkboxText(node), label)
	})
	if node == nil {
		t.Fatalf("missing checkbox %q", label)
	}
	return node
}

func checkboxClick(t *testing.T, node *native.Node) {
	t.Helper()
	for _, listener := range node.Listeners {
		if listener.Type == protocol.EventClick {
			reactive.Batch(func() {
				listener.Listener(&native.Event{
					Type:   protocol.EventClick,
					Target: node,
				})
			})
			return
		}
	}
	t.Fatal("checkbox has no click handler")
}

func checkboxMarkIs(t *testing.T, node *native.Node, expected string) {
	t.Helper()
	if len(node.Children) == 0 {
		t.Fatal("checkbox has no indicator")
	}
	indicator := node.Children[0]
	mark := checkboxFind(indicator, func(node *native.Node) bool { return node.Tag == protocol.TagSvg })
	if (mark != nil) != (expected != "") {
		t.Fatalf("SVG indicator present: %v, expected state %q", mark != nil, expected)
	}
	if text := checkboxText(indicator); text != "" {
		t.Fatalf("indicator uses a font glyph instead of an SVG: %q", text)
	}

}

func TestCheckboxDemoIndicatorsFollowTogglesAndChildState(t *testing.T) {
	checkboxTestRoot(t, CheckboxDemo, func(root *native.Node) {
		notify := checkboxWithLabel(t, root, "Email me about releases")
		indicator := notify.Children[0]
		checkboxMarkIs(t, notify, "−")
		for _, mark := range []string{"✓", "", "✓", ""} {
			checkboxClick(t, notify)
			checkboxMarkIs(t, notify, mark)
			if notify.Children[0] != indicator {
				t.Fatal("toggle remounted the indicator")
			}
		}
		readOnly := checkboxWithLabel(t, root, "Read-only:")
		for range 3 {
			checkboxClick(t, readOnly)
			checkboxMarkIs(t, readOnly, "✓")
		}
		parent := checkboxWithLabel(t, root, "Parent, derived")
		analytics := checkboxWithLabel(t, root, "Analytics")
		crashes := checkboxWithLabel(t, root, "Crash reports")
		checkboxMarkIs(t, parent, "−")
		checkboxMarkIs(t, analytics, "✓")
		checkboxMarkIs(t, crashes, "")
		for _, mark := range []string{"✓", ""} {
			checkboxClick(t, parent)
			for _, node := range []*native.Node{parent, analytics, crashes} {
				checkboxMarkIs(t, node, mark)
			}
		}
		checkboxClick(t, analytics)
		checkboxMarkIs(t, parent, "−")
		checkboxMarkIs(t, analytics, "✓")
		checkboxMarkIs(t, crashes, "")

	})
}

func TestUncontrolledCheckboxIndicatorAndCallbackStayInSync(t *testing.T) {
	var changes []bool
	checkboxTestRoot(t, func() *native.Node {
		return checkbox(ui.CheckboxProps{
			OnCheckedChange: func(next bool, _ *native.Event) { changes = append(changes, next) },
		}, "Uncontrolled")
	}, func(root *native.Node) {
		checkboxMarkIs(t, root, "")
		checkboxClick(t, root)
		checkboxMarkIs(t, root, "✓")
		checkboxClick(t, root)
		checkboxMarkIs(t, root, "")
		if len(changes) != 2 || !changes[0] || changes[1] {
			t.Fatalf("unexpected changes: %v", changes)
		}
	})
}

func TestCheckboxGroupIndicatorsFollowNativeGroupChanges(t *testing.T) {
	checkboxTestRoot(t, CheckboxGroupDemo, func(root *native.Node) {
		parent := checkboxWithLabel(t, root, "All colours")
		green := checkboxWithLabel(t, root, "green")
		red := checkboxWithLabel(t, root, "red")
		checkboxMarkIs(t, parent, "−")
		checkboxMarkIs(t, green, "✓")
		checkboxMarkIs(t, red, "")
		group := checkboxFind(root, func(node *native.Node) bool {
			for _, listener := range node.Listeners {
				if listener.Type == protocol.EventComponentChange {
					return true
				}
			}
			return false
		})
		if group == nil {
			t.Fatal("missing group event listener")
		}
		host := native.NewNodeHost(1, 1)
		host.Nodes[group.ID] = group
		for _, update := range []struct{ values, mark string }{
			{`["red","green","blue","violet"]`, "✓"},
			{`[]`, ""},
		} {
			reactive.Batch(func() {
				native.DispatchEvent(
					host,
					protocol.EventComponentChange,
					group.ID,
					`{"checkedValues":`+update.values+`}`,
					true,
				)
			})
			for _, node := range []*native.Node{parent, green, red} {
				checkboxMarkIs(t, node, update.mark)
			}
		}

	})
}
