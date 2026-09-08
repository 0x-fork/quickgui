package ui

import (
	"bytes"
	gui "github.com/egoist/quickgui/go/ui"
	"strings"
	"testing"

	"github.com/egoist/quickgui/go/native"
	"github.com/egoist/quickgui/go/protocol"
	"github.com/egoist/quickgui/go/reactive"
	"quickgui.example/quick-git/internal/git"
	"quickgui.example/quick-git/internal/model"
)

func TestToolbarRetainsButtonsAndUpdatesTaskState(t *testing.T) {
	reactive.CreateRoot(func(dispose func()) struct{} {
		defer dispose()
		status, setStatus := reactive.CreateSignal(&git.RepositoryStatus{
			Branch: "main", Upstream: "origin/main", HasUpstreamCounts: true, Ahead: 1,
		})
		busy, setBusy := reactive.CreateSignal[*model.BusyState](nil)
		conflicts, setConflicts := reactive.CreateSignal(0)
		store := &model.Store{Status: status, Busy: busy, Conflicts: conflicts}
		theme, _ := reactive.CreateSignal(ThemeFor("dark"))
		root := gui.View(func() { ProvideApp(AppContext{Store: store, Theme: theme}, Toolbar) })
		buttons := toolbarButtons(root.Node)
		if len(buttons) != 5 || buttons["Fetch"] == nil || buttons["Pull"] == nil || buttons["Push"] == nil {
			t.Fatalf("missing toolbar actions: %v", buttons)
		}
		assertFrameworkToolbar(t, root.Node)
		cancelled := 0
		offset := len(root.Pending.Body())
		setBusy(&model.BusyState{Label: "Fetch", Cancel: func() { cancelled++ }})
		for _, label := range []string{"Fetch", "Pull", "Push", ""} {
			assertDisabledMutation(t, root.Node, offset, buttons[label], true)
		}
		assertFrameworkToolbar(t, root.Node)
		cancel := toolbarButtons(root.Node)["Cancel"]
		if cancel == nil || !strings.Contains(toolbarText(root.Node), "Fetch…") {
			t.Fatal("busy state did not show its label and cancellation action")
		}
		for _, listener := range cancel.Listeners {
			if listener.Type == protocol.EventClick {
				listener.Listener(&native.Event{})
			}
		}
		if cancelled != 1 {
			t.Fatal("Cancel did not reach the current operation")
		}
		setBusy(&model.BusyState{Label: "Refresh"})
		if !strings.Contains(toolbarText(root.Node), "Refresh…") || toolbarButtons(root.Node)["Cancel"] != nil {
			t.Fatal("changing an active task left a stale label or cancellation action")
		}
		setStatus(&git.RepositoryStatus{
			Branch: "feature", HasUpstreamCounts: true, Ahead: 3, Behind: 2,
		})
		setConflicts(2)
		if text := toolbarText(root.Node); !strings.Contains(text, "feature") || !strings.Contains(text, "feature32") || !strings.Contains(text, "2 conflicted") {
			t.Fatalf("status text did not follow its signals: %q", text)
		}
		offset = len(root.Pending.Body())
		setBusy(nil)
		for _, label := range []string{"Fetch", "Push", ""} {
			assertDisabledMutation(t, root.Node, offset, buttons[label], false)
		}
		// Pull stays disabled without an upstream, then re-enables when one appears.
		offset = len(root.Pending.Body())
		setStatus(&git.RepositoryStatus{Branch: "feature", Upstream: "origin/feature"})
		assertDisabledMutation(t, root.Node, offset, buttons["Pull"], false)
		for label, button := range buttons {
			if label != "main" && toolbarButtons(root.Node)[label] != button {
				t.Fatal("a task or status update rebuilt the toolbar buttons")
			}
		}
		return struct{}{}
	})
}

func assertDisabledMutation(t *testing.T, root *native.Node, offset int, button *native.Node, disabled bool) {
	t.Helper()
	if button == nil {
		t.Fatal("missing button")
	}
	expected := protocol.NewBatch()
	expected.SetBoolean(button.ID, protocol.Disabled, disabled)
	if !bytes.Contains(root.Pending.Body()[offset:], expected.Body()) {
		t.Fatalf("button %d did not declare disabled=%v", button.ID, disabled)
	}
}

func assertFrameworkToolbar(t *testing.T, node *native.Node) {
	t.Helper()
	switch node.Tag {
	case protocol.TagView, protocol.TagButton, protocol.TagText, protocol.TagSentinel, protocol.TagSvg:
	default:
		t.Fatalf("toolbar contains a hosted native control (tag %d)", node.Tag)
	}
	for _, child := range node.Children {
		assertFrameworkToolbar(t, child)
	}
}

func toolbarButtons(root *native.Node) map[string]*native.Node {
	buttons := map[string]*native.Node{}
	var walk func(*native.Node)
	walk = func(node *native.Node) {
		if node.Tag == protocol.TagButton {
			buttons[toolbarText(node)] = node
		}
		for _, child := range node.Children {
			walk(child)
		}
	}
	walk(root)
	return buttons
}

func toolbarText(node *native.Node) string {
	text := node.Text
	for _, child := range node.Children {
		text += toolbarText(child)
	}
	return text
}
