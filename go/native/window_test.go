package native

import (
	"encoding/json"
	"testing"

	"github.com/egoist/quickgui/go/host"
	"github.com/egoist/quickgui/go/reactive"
)

func TestEncodeWindowOptionsSendsEmbeddedChrome(t *testing.T) {
	decorated := false
	shadow := false
	visible := false
	encoded, err := json.Marshal(encodeWindowOptions(WindowOptions{
		Title:                "QuickGUI SwiftUI embedded view",
		Width:                320,
		Height:               200,
		Visible:              &visible,
		Decorated:            &decorated,
		Shadow:               &shadow,
		BackgroundAppearance: "transparent",
	}))
	if err != nil {
		t.Fatal(err)
	}
	var payload map[string]any
	if err := json.Unmarshal(encoded, &payload); err != nil {
		t.Fatal(err)
	}
	if payload["decorated"] != false || payload["shadow"] != false || payload["show"] != false {
		t.Fatalf("%s", encoded)
	}
	if payload["transparent"] != true {
		t.Fatalf("transparent %v", payload["transparent"])
	}
}

type windowHost struct {
	host.Fake
	created int
	closed  int
}

func (h *windowHost) AllocateWindow() uint32                       { return 42 }
func (h *windowHost) CreateWindow(_, _ uint32, _ string, _ []byte) { h.created++ }
func (h *windowHost) CloseWindow(_, _ uint32)                      { h.closed++ }

func TestWindowOwnsComponentLifetime(t *testing.T) {
	fake := &windowHost{}
	defer host.Install(fake)()
	previous := App
	App = &Application{NativeID: 1, ready: true}
	defer func() { App = previous; pendingFlush = nil }()
	value, setValue := reactive.CreateSignal("first")
	mounts, effects, cleanups := 0, 0, 0
	var text *Node
	window := NewWindow(WindowOptions{Component: func() {
		if CurrentWindow() == nil {
			t.Fatal("missing component window")
		}
		mounts++
		text = CreateText("")
		text.Bind(func() { effects++; ReplaceText(text, value()) })
		CreateText("sibling")
		reactive.OnCleanup(func() { cleanups++ })
		DeclareChild(text)
	}})
	setValue("second")
	if mounts != 1 || effects != 2 || text.Text != "second" || fake.created != 1 {
		t.Fatal("the component remounted or its binding did not update")
	}
	if len(window.Root.Children) != 2 || window.Root.Children[0] != text || window.Root.Children[1].Text != "sibling" {
		t.Fatal("window components must collect multiple roots once in declaration order")
	}
	App.didCloseWindow(window)
	App.didCloseWindow(window)
	setValue("after close")
	if effects != 2 || cleanups != 1 || window.Root != nil || len(App.Windows) != 0 || fake.closed != 0 {
		t.Fatal("window closure did not dispose the component exactly once")
	}
}
