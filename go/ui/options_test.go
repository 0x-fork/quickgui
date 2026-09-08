package ui

import (
	"bytes"
	"reflect"
	"testing"

	"github.com/egoist/quickgui/go/native"
	"github.com/egoist/quickgui/go/protocol"
	"github.com/egoist/quickgui/go/reactive"
)

func TestStyleOptionsMergeInteractionStatesWithoutChangingSharedStyles(t *testing.T) {
	for name, state := range map[string]func(...styleDeclaration) styleOption{
		"Hover": styleHover, "Active": styleActive, "Focus": styleFocus, "Disabled": styleDisabledStyle,
		"Selected": styleSelectedStyle, "Invalid": styleInvalidStyle, "Dragging": styleDragging,
		"DragOver": styleDragOver, "FocusWithin": styleFocusWithin,
	} {
		t.Run(name, func(t *testing.T) {
			shared := composeStyles(state(styleTextColor("white"), styleBackgroundColor("#222222"), styleOpacity(.8)))
			props := resolveProps([]any{
				shared,
				state(styleBackgroundColor("#333333")),
				state(styleOpacity(0)),
			})
			read := func(style styleData) *styleData {
				return reflect.ValueOf(style).FieldByName(name).Interface().(*styleData)
			}
			merged := read(props.Style.style)
			if merged.TextColor != "white" || merged.BackgroundColor != "#333333" || merged.Opacity != 0 {
				t.Fatalf("repeated state options lost an earlier property: %+v", merged)
			}
			original := read(shared)
			if original.BackgroundColor != "#222222" || original.Opacity != .8 {
				t.Fatal("state options mutated a shared style")
			}
		})
	}
}

func TestStyleOptionAliasesRespectDeclarationOrder(t *testing.T) {
	for _, pair := range []struct {
		name             string
		canonical, alias styleOption
		first, last      any
	}{
		{"OverflowWrap", styleOverflowWrap("normal"), styleWordWrap("anywhere"), "normal", "anywhere"},
		{"TransitionEasing", styleTransitionEasing("linear"), styleTransitionTimingFunction("ease-out"), "linear", "ease-out"},
		{"PaddingStart", stylePaddingStart(12), stylePaddingInlineStart(0), 12, 0},
		{"PaddingEnd", stylePaddingEnd(12), stylePaddingInlineEnd(0), 12, 0},
		{"MarginStart", styleMarginStart(12), styleMarginInlineStart(0), 12, 0},
		{"MarginEnd", styleMarginEnd(12), styleMarginInlineEnd(0), 12, 0},
		{"BorderStartWidth", styleBorderStartWidth(1), styleBorderInlineStartWidth(0), 1, 0},
		{"BorderEndWidth", styleBorderEndWidth(1), styleBorderInlineEndWidth(0), 1, 0},
	} {
		t.Run(pair.name, func(t *testing.T) {
			for _, order := range []struct {
				options []any
				want    any
			}{
				{[]any{pair.canonical, pair.alias}, pair.last},
				{[]any{pair.alias, pair.canonical}, pair.first},
			} {
				style := normalizeStyleAliases(resolveProps(order.options).Style.style)
				if got := reflect.ValueOf(style).FieldByName(pair.name).Interface(); got != order.want {
					t.Fatalf("declaration order: got %v, want %v", got, order.want)
				}
			}
		})
	}
}

func TestComposedStylesKeepPartBindingsIndependent(t *testing.T) {
	reactive.CreateRoot(func(dispose func()) struct{} {
		defer dispose()
		width := reactive.NewSignal(20)
		color := reactive.NewSignal("#112233")
		shared := composeStyles(styleWidth(width.Read), styleBackgroundColor(color.Read))
		if len(width.Observers) != 0 || len(color.Observers) != 0 {
			t.Fatal("composing styles eagerly evaluated accessors")
		}
		parent := View()
		node := View()
		applyPart(node.Node, PartProps{Style: StyleBuilder{style: shared}})
		native.InsertNode(parent.Node, node.Node, nil)
		offset := len(parent.Pending.Body())
		width.Write(30)
		expected := protocol.NewBatch()
		expected.SetNumber(node.ID, protocol.Width, 30)
		if !bytes.Equal(parent.Pending.Body()[offset:], expected.Body()) {
			t.Fatal("a composed part style updated unrelated properties")
		}
		native.RemoveNode(parent.Node, node.Node)
		if len(width.Observers) != 0 || len(color.Observers) != 0 {
			t.Fatal("a disposed composed style retained its bindings")
		}
		if resolveStyle(Style().PaddingLeft(12)).PaddingLeft != 12 {
			t.Fatal("compound parts did not accept an individual style option")
		}
		if composeStyles(styleObjectFit("contain")).ObjectFit != "contain" {
			t.Fatal("ObjectFit could not be composed with other image styles")
		}
		return struct{}{}
	})
}

func TestWhenRestoresBaseStyleWithoutRebuildingChildren(t *testing.T) {
	reactive.CreateRoot(func(dispose func()) struct{} {
		defer dispose()
		selected, setSelected := CreateSignal(false)
		mounts := 0
		node := View(
			styleBackgroundColor("#ccc"),
			When(selected, styleBackgroundColor("#2563eb"), styleTextColor("white")),
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
			styleData{BackgroundColor: baseColor.Read, TextColor: "white", Padding: 12},
			styleData{Padding: 0},
			When(selected, styleData{BackgroundColor: selectedColor.Read, Opacity: .5}),
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
	base := styleData{
		BackgroundColor: "#111111",
		Hover:           &styleData{TextColor: "white", BackgroundColor: "#222222", Opacity: .8},
		Focus:           &styleData{OutlineWidth: 2, OutlineColor: "blue"},
	}
	props := resolveProps([]any{
		base,
		styleData{Hover: &styleData{BackgroundColor: "#333333", Opacity: 0}},
		styleData{Focus: &styleData{OutlineColor: "red"}},
	})
	if props.Style.style.BackgroundColor != base.BackgroundColor || props.Style.style.Hover.TextColor != "white" ||
		props.Style.style.Hover.BackgroundColor != "#333333" || props.Style.style.Hover.Opacity != 0 ||
		props.Style.style.Focus.OutlineWidth != 2 || props.Style.style.Focus.OutlineColor != "red" {
		t.Fatal("style records replaced a nested state instead of merging its fields")
	}
	if base.Hover.BackgroundColor != "#222222" || base.Hover.Opacity != .8 || base.Focus.OutlineColor != "blue" {
		t.Fatal("style merging changed the shared base style")
	}
}

func TestStyleAliasesMergeIntoTheSameProperty(t *testing.T) {
	props := resolveProps([]any{
		styleData{PaddingStart: 12, OverflowWrap: "normal", TransitionEasing: "linear"},
		styleData{PaddingInlineStart: 0, WordWrap: "anywhere", TransitionTimingFunction: "ease-out"},
	})
	if props.Style.style.PaddingStart != 0 || props.Style.style.OverflowWrap != "anywhere" || props.Style.style.TransitionEasing != "ease-out" {
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
		node := View(When(outer, When(inner.Read, styleTextColor("white"))))
		native.InsertNode(parent.Node, node.Node, nil)
		if len(inner.Observers) != 0 {
			t.Fatal("inactive branch was evaluated")
		}
		setOuter(true)
		if len(inner.Observers) != 1 {
			t.Fatal("active branch was not tracked")
		}
		native.RemoveNode(parent.Node, node.Node)
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
