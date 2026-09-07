package ui

import (
	"bytes"
	"testing"

	"github.com/egoist/quickgui/go/native"
	"github.com/egoist/quickgui/go/protocol"
	"github.com/egoist/quickgui/go/reactive"
)

func TestWhenRestoresBaseStyleWithoutRebuildingChildren(t *testing.T) {
	reactive.CreateRoot(func(dispose func()) struct{} {
		defer dispose()
		selected, setSelected := CreateSignal(false)
		mounts := 0
		node := View(
			BackgroundColor("#ccc"),
			When(selected, BackgroundColor("#2563eb"), Color("white")),
			func() { mounts++; Text("child") },
		)
		child := node.Children[0]
		setSelected(true)
		offset := len(node.Pending.Body())
		setSelected(false)
		expected := protocol.NewBatch()
		expected.SetColor(node.ID, protocol.BackgroundColor, native.ParseColor("#ccc"))
		expected.ClearProperty(node.ID, protocol.Color)
		if !bytes.Equal(node.Pending.Body()[offset:], expected.Body()) {
			t.Fatalf("expected only the base background and cleared color, got %v", node.Pending.Body()[offset:])
		}
		if mounts != 1 || len(node.Children) != 1 || node.Children[0] != child {
			t.Fatal("a conditional option rebuilt its children")
		}
		return struct{}{}
	})
}

func TestStyleRecordsComposeAndConditionalStylesRestoreBindings(t *testing.T) {
	reactive.CreateRoot(func(dispose func()) struct{} {
		defer dispose()
		selected, setSelected := CreateSignal(false)
		baseColor := reactive.NewSignal("#334455")
		selectedColor := reactive.NewSignal("#ddeeff")
		mounts := 0
		node := View(
			func() { mounts++; Text("retained") },
			Style{BackgroundColor: baseColor.Read, Color: "white", Padding: 12},
			Style{Padding: 0},
			When(selected, Style{BackgroundColor: selectedColor.Read, Opacity: .5}),
		)
		child := node.Children[0]
		expected := protocol.NewBatch()
		expected.SetNumber(node.ID, protocol.Padding, 0)
		if !bytes.Contains(node.Pending.Body(), expected.Body()) {
			t.Fatal("later explicit zero did not override earlier padding")
		}
		if len(baseColor.Observers) != 1 || len(selectedColor.Observers) != 0 {
			t.Fatal("inactive conditional style subscribed to its accessor")
		}
		setSelected(true)
		if len(baseColor.Observers) != 0 || len(selectedColor.Observers) != 1 {
			t.Fatal("conditional style did not replace the base binding")
		}
		baseColor.Write("#112233")
		offset := len(node.Pending.Body())
		setSelected(false)
		expected = protocol.NewBatch()
		expected.SetColor(node.ID, protocol.BackgroundColor, native.ParseColor("#112233"))
		expected.ClearProperty(node.ID, protocol.Opacity)
		if !bytes.Equal(node.Pending.Body()[offset:], expected.Body()) {
			t.Fatalf("conditional style did not restore the latest base color: %v", node.Pending.Body()[offset:])
		}
		if len(baseColor.Observers) != 1 || len(selectedColor.Observers) != 0 || mounts != 1 || node.Children[0] != child {
			t.Fatal("style composition remounted children or retained an inactive binding")
		}
		return struct{}{}
	})
}

func TestStyleRecordsMergeNestedStatesWithoutMutatingSharedStyles(t *testing.T) {
	base := Style{
		BackgroundColor: "#111111",
		Hover:           &Style{Color: "white", BackgroundColor: "#222222", Opacity: .8},
		Focus:           &Style{OutlineWidth: 2, OutlineColor: "blue"},
	}
	props := resolveProps([]any{
		base,
		Style{Hover: &Style{BackgroundColor: "#333333", Opacity: 0}},
		Style{Focus: &Style{OutlineColor: "red"}},
	})
	if props.Style.BackgroundColor != base.BackgroundColor || props.Style.Hover.Color != "white" ||
		props.Style.Hover.BackgroundColor != "#333333" || props.Style.Hover.Opacity != 0 ||
		props.Style.Focus.OutlineWidth != 2 || props.Style.Focus.OutlineColor != "red" {
		t.Fatal("style records replaced a nested state instead of merging its fields")
	}
	if base.Hover.BackgroundColor != "#222222" || base.Hover.Opacity != .8 || base.Focus.OutlineColor != "blue" {
		t.Fatal("style merging changed the shared base style")
	}
}

func TestStyleAliasesMergeIntoTheSameProperty(t *testing.T) {
	props := resolveProps([]any{
		Style{PaddingStart: 12, OverflowWrap: "normal", TransitionEasing: "linear"},
		Style{PaddingInlineStart: 0, WordWrap: "anywhere", TransitionTimingFunction: "ease-out"},
	})
	if props.Style.PaddingStart != 0 || props.Style.OverflowWrap != "anywhere" || props.Style.TransitionEasing != "ease-out" {
		t.Fatal("an alias failed to override the previous declaration")
	}
}

func TestWhenDisposesConditionalBindingsAndHandlers(t *testing.T) {
	reactive.CreateRoot(func(dispose func()) struct{} {
		defer dispose()
		enabled, setEnabled := CreateSignal(true)
		value := reactive.NewSignal("initial")
		clicks := 0
		node := Input(When(enabled, Value(value.Read), OnClick(func() { clicks++ })))
		if len(value.Observers) != 1 || len(node.Listeners) != 1 {
			t.Fatal("conditional value or click handler was not installed")
		}
		node.Listeners[0].Listener(&native.Event{})
		setEnabled(false)
		if len(value.Observers) != 0 || len(node.Listeners) != 0 || clicks != 1 {
			t.Fatal("inactive conditional props retained a binding or handler")
		}
		before := node.Pending.MutationCount()
		value.Write("inactive")
		if node.Pending.MutationCount() != before {
			t.Fatal("an inactive conditional prop still enqueued mutations")
		}
		for i := 0; i < 10; i++ {
			setEnabled(true)
			if len(value.Observers) != 1 {
				t.Fatal("conditional bindings accumulated across toggles")
			}
			setEnabled(false)
		}
		return struct{}{}
	})
}

func TestNestedWhenTracksOnlyTheActiveBranch(t *testing.T) {
	reactive.CreateRoot(func(dispose func()) struct{} {
		defer dispose()
		outer, setOuter := CreateSignal(false)
		inner := reactive.NewSignal(true)
		parent := View()
		node := View(When(outer, When(inner.Read, Color("white"))))
		native.InsertNode(parent, node, nil)
		if len(inner.Observers) != 0 {
			t.Fatal("inactive branch was evaluated")
		}
		setOuter(true)
		if len(inner.Observers) != 1 {
			t.Fatal("active branch was not tracked")
		}
		native.RemoveNode(parent, node)
		if len(inner.Observers) != 0 {
			t.Fatal("removed node retained its conditional option")
		}
		return struct{}{}
	})
}

func TestEventAwareClickOptionPreservesEventControls(t *testing.T) {
	node := Button(OnClickEvent(func(event *native.Event) { event.PreventDefault() }), "Click")
	event := &native.Event{}
	node.Listeners[0].Listener(event)
	if !event.DefaultPrevented {
		t.Fatal("event-aware callback lost the native event")
	}
}
