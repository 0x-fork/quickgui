package ui

import (
	"bytes"
	"testing"

	"github.com/egoist/quickgui/go/native"
	"github.com/egoist/quickgui/go/protocol"
	"github.com/egoist/quickgui/go/reactive"
)

func TestHoverGroupUsesNativeGroupPropertyAndTracksNames(t *testing.T) {
	reactive.CreateRoot(func(dispose func()) struct{} {
		defer dispose()
		name := reactive.NewSignal("card")
		node := View(Group(name.Read))
		expected := protocol.NewBatch()
		expected.SetString(node.ID, protocol.HoverGroup, "card")
		if !bytes.Contains(node.Pending.Body(), expected.Body()) {
			t.Fatal("group did not use the native hover-group property")
		}
		old := protocol.NewBatch()
		old.SetString(node.ID, protocol.Group, "card")
		if bytes.Contains(node.Pending.Body(), old.Body()) {
			t.Fatal("hover group overwrote picker grouping")
		}
		offset := len(node.Pending.Body())
		name.Write("toolbar")
		expected = protocol.NewBatch()
		expected.SetString(node.ID, protocol.HoverGroup, "toolbar")
		if !bytes.Equal(node.Pending.Body()[offset:], expected.Body()) {
			t.Fatal("group name did not update independently")
		}
		offset = len(node.Pending.Body())
		name.Write("")
		expected = protocol.NewBatch()
		expected.ClearProperty(node.ID, protocol.HoverGroup)
		if !bytes.Equal(node.Pending.Body()[offset:], expected.Body()) {
			t.Fatal("empty group did not clear")
		}
		part := createViewPart(PartProps{Group: true})
		expected = protocol.NewBatch()
		expected.SetBoolean(part.ID, protocol.HoverGroup, true)
		if !bytes.Contains(part.Pending.Body(), expected.Body()) {
			t.Fatal("compound part did not expose hover group")
		}
		return struct{}{}
	})
}
func TestNamedGroupRulesAccumulateAndTrackColorsWithoutRemounting(t *testing.T) {
	reactive.CreateRoot(func(dispose func()) struct{} {
		defer dispose()
		color := reactive.NewSignal("#112233")
		enabled := reactive.NewSignal(true)
		parent := View()
		node := View(styleGroupHover(styleOpacity(.5)), When(enabled.Read, styleGroupHoverNamed("card", styleTextColor(color.Read))), "kept")
		native.InsertNode(parent.Node, node.Node, nil)
		child := node.Children[0]
		offset := len(parent.Pending.Body())
		color.Write("#abcdef")
		updates := parent.Pending.Body()[offset:]
		if !bytes.Contains(updates, []byte(`"group":"card"`)) || !bytes.Contains(updates, []byte(`[{"opacity":0.5},{"color":`)) {
			t.Fatal("ordered named rules were not encoded")
		}
		if child != node.Children[0] || len(color.Observers) != 1 {
			t.Fatal("rule update remounted children or duplicated bindings")
		}
		enabled.Write(false)
		if len(color.Observers) != 0 {
			t.Fatal("disabled group rule retained color subscription")
		}
		offset = len(parent.Pending.Body())
		enabled.Write(true)
		if !bytes.Contains(parent.Pending.Body()[offset:], []byte(`"group":"card"`)) {
			t.Fatal("conditional group rule was not restored")
		}
		native.RemoveNode(parent.Node, node.Node)
		if len(color.Observers) != 0 {
			t.Fatal("removed group rule retained subscription")
		}
		return struct{}{}
	})
}
func TestReusingGroupStylesDoesNotMutateOtherDeclarations(t *testing.T) {
	base := styleData{GroupHover: &styleData{Opacity: .25}}
	first := resolveProps([]any{base, styleGroupHoverNamed("card", styleOpacity(.5))})
	second := resolveProps([]any{base, styleGroupHoverNamed("toolbar", styleOpacity(1))})
	left, right := groupHoverRules(first.Style.style), groupHoverRules(second.Style.style)
	if len(left) != 2 || len(right) != 2 || left[1].name != "card" || right[1].name != "toolbar" || len(groupHoverRules(base)) != 1 {
		t.Fatal("shared group styles mutated another declaration")
	}
}
