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
