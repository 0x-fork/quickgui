package ui

import (
	"bytes"
	"testing"

	"github.com/egoist/quickgui/go/native"
	"github.com/egoist/quickgui/go/protocol"
	"github.com/egoist/quickgui/go/reactive"
)

func TestScalarStyleBindingsUpdateIndependentlyAndDispose(t *testing.T) {
	reactive.CreateRoot(func(dispose func()) struct{} {
		defer dispose()
		width := reactive.NewSignal(320.0)
		color := reactive.NewSignal("#112233")
		mounts := 0
		parent := View()
		node := View(Width(width.Read), BackgroundColor(color.Read), func() { mounts++; Text("kept") })
		native.InsertNode(parent, node, nil)
		child := node.Children[0]
		offset := len(parent.Pending.Body())
		width.Write(420)
		expected := protocol.NewBatch()
		expected.SetNumber(node.ID, protocol.Width, 420)
		if !bytes.Equal(parent.Pending.Body()[offset:], expected.Body()) {
			t.Fatal("width updated unrelated properties")
		}
		offset = len(parent.Pending.Body())
		color.Write("#abcdef")
		expected = protocol.NewBatch()
		expected.SetColor(node.ID, protocol.BackgroundColor, native.ParseColor("#abcdef"))
		if !bytes.Equal(parent.Pending.Body()[offset:], expected.Body()) {
			t.Fatal("color updated unrelated properties")
		}
		if mounts != 1 || child != node.Children[0] {
			t.Fatal("style update remounted children")
		}
		native.RemoveNode(parent, node)
		if len(width.Observers) != 0 || len(color.Observers) != 0 {
			t.Fatal("removed styles retained observers")
		}
		return struct{}{}
	})
}

func TestConditionalStyleAccessorsReleaseAndRestoreBase(t *testing.T) {
	reactive.CreateRoot(func(dispose func()) struct{} {
		defer dispose()
		active, setActive := CreateSignal(true)
		width := reactive.NewSignal(50)
		node := View(Width(20), When(active, Width(width.Read)))
		for range 3 {
			if len(width.Observers) != 1 {
				t.Fatal("missing or duplicate conditional style observer")
			}
			offset := len(node.Pending.Body())
			setActive(false)
			expected := protocol.NewBatch()
			expected.SetNumber(node.ID, protocol.Width, 20)
			if !bytes.Equal(node.Pending.Body()[offset:], expected.Body()) {
				t.Fatal("base width was not restored")
			}
			if len(width.Observers) != 0 {
				t.Fatal("inactive style kept its observer")
			}
			setActive(true)
		}
		return struct{}{}
	})
}

func TestStateStyleTracksThemeAndReleasesBindings(t *testing.T) {
	reactive.CreateRoot(func(dispose func()) struct{} {
		defer dispose()
		accent := reactive.NewSignal("#112233")
		parent := View()
		node := Button(Hover(BackgroundColor(accent.Read)), Focus(OutlineWidth(2), OutlineColor(accent.Read)), "Retained")
		native.InsertNode(parent, node, nil)
		child := node.Children[0]
		before := len(parent.Pending.Body())
		accent.Write("#abcdef")
		updates := parent.Pending.Body()[before:]
		if !bytes.Contains(updates, []byte(`"outline":"2px #abcdefff"`)) {
			t.Fatal("focus outline did not track the accent")
		}
		if node.Children[0] != child || len(accent.Observers) != 2 {
			t.Fatal("state style update remounted children or accumulated bindings")
		}
		native.RemoveNode(parent, node)
		if len(accent.Observers) != 0 {
			t.Fatal("removed state styles retained observers")
		}
		return struct{}{}
	})
}
