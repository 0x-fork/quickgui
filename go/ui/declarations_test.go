package ui

import (
	"bytes"
	"encoding/json"
	"testing"

	"github.com/egoist/quickgui/go/native"
	"github.com/egoist/quickgui/go/protocol"
	"github.com/egoist/quickgui/go/reactive"
)

func TestShadowsAndTransitionsUseTheHostWireFormat(t *testing.T) {
	node := View(Style{
		BoxShadow:  "0 2px 8px rgba(0, 0, 0, 0.2), inset 0 0 0 1px currentColor",
		Transition: "background-color 120ms ease, opacity 0.2s ease-out, transform 0.2s ease-out",
	})
	if bytes.Contains(node.Pending.Body(), []byte("rgba(")) {
		t.Fatal("CSS shadow was sent as text even though the host requires JSON")
	}
	if !bytes.Contains(node.Pending.Body(), []byte(`"blurRadius":8`)) || !bytes.Contains(node.Pending.Body(), []byte(`"inset":true`)) {
		t.Fatal("shadow lost its blur or inset")
	}
	expected := protocol.NewBatch()
	expected.SetNumber(node.ID, protocol.TransitionDuration, 200)
	expected.SetString(node.ID, protocol.TransitionProperties, "background-color,opacity,transform")
	expected.SetString(node.ID, protocol.TransitionEasing, "ease-out")
	if !bytes.Contains(node.Pending.Body(), expected.Body()) {
		t.Fatal("transition was not translated into duration, property, and easing fields")
	}
}

func TestGradientBindingClearsThePreviousPaintKind(t *testing.T) {
	reactive.CreateRoot(func(dispose func()) struct{} {
		defer dispose()
		background := reactive.NewSignal[any](GradientDeclaration{Type: "linear", Stops: []GradientStop{{Color: "red"}, {Color: "blue"}}})
		node := View(Style{Background: background.Read}, "kept")
		child := node.Children[0]
		if !bytes.Contains(node.Pending.Body(), []byte(`{"type":"linear","stops":[{"color":"red"},{"color":"blue"}]}`)) {
			t.Fatal("structured gradient did not reach the host")
		}
		offset := len(node.Pending.Body())
		background.Write("#123456")
		expected := protocol.NewBatch()
		expected.SetColor(node.ID, protocol.BackgroundColor, native.ParseColor("#123456"))
		expected.ClearProperty(node.ID, protocol.BackgroundGradient)
		if !bytes.Equal(node.Pending.Body()[offset:], expected.Body()) || node.Children[0] != child {
			t.Fatal("solid background did not replace just the gradient paint")
		}
		return struct{}{}
	})
}

func TestStateStylesKeepGradientsShadowsAndNamedActiveGroups(t *testing.T) {
	style := Style{
		Background: "linear-gradient(red, blue)",
		BoxShadow:  "0 1px 2px black", Opacity: .5,
	}
	node := View(
		Style{Invalid: &style, Dragging: &style, DragOver: &style, FocusWithin: &style},
		GroupActiveNamed("card", Style{TextColor: "white"}),
		GroupActiveNamed("toolbar", Style{Opacity: .2}),
	)
	if !bytes.Contains(node.Pending.Body(), []byte(`"group":"card"`)) || !bytes.Contains(node.Pending.Body(), []byte(`"group":"toolbar"`)) {
		t.Fatal("named pressed-group rules were not serialized")
	}
	encoded, ok := encodeStateStyle("invalid", &style)
	data, err := json.Marshal(encoded)
	if !ok || err != nil || encoded.Background != "linear-gradient(red, blue)" || !bytes.Contains(data, []byte(`"boxShadow":[`)) {
		t.Fatalf("state paint declaration = %s: %v", data, err)
	}
	for _, code := range []uint16{protocol.InvalidStyle, protocol.DraggingStyle, protocol.DragOverStyle, protocol.FocusWithinStyle} {
		expected := protocol.NewBatch()
		expected.SetString(node.ID, code, string(data))
		if !bytes.Contains(node.Pending.Body(), expected.Body()) {
			t.Fatalf("state property %d was not sent", code)
		}
	}
}

func TestOptionalReactiveStateFieldsClearWithoutLosingMergedSiblings(t *testing.T) {
	reactive.CreateRoot(func(dispose func()) struct{} {
		defer dispose()
		color := reactive.NewSignal[any]("#ff0000")
		width := reactive.NewSignal[any](2)
		node := View(Style{Hover: &Style{TextColor: color.Read, BorderWidth: width.Read}}, Style{Hover: &Style{Opacity: .7}})
		offset := len(node.Pending.Body())
		reactive.Batch(func() { color.Write(nil); width.Write(nil) })
		expected := protocol.NewBatch()
		expected.SetString(node.ID, protocol.HoverStyle, `{"opacity":0.7}`)
		if !bytes.Equal(node.Pending.Body()[offset:], expected.Body()) {
			t.Fatal("optional state values did not clear while preserving the later merged declaration")
		}
		return struct{}{}
	})
}
