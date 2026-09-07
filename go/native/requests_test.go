package native

import (
	"encoding/json"
	"testing"

	"github.com/egoist/quickgui/go/host"
	"github.com/egoist/quickgui/go/reactive"
)

type commandHost struct {
	host.Fake
	request uint32
	payload string
}

func (h *commandHost) Command(_ uint32, request uint32, payload string) {
	h.request, h.payload = request, payload
}

func TestDeferredCommandWaitsForFinalResult(t *testing.T) {
	for _, method := range []string{"window-popup-menu", "shell-action", "app-service", "file-icon", "notification-permission", "global-shortcut", "set-tray-icon", "set-user-tasks"} {
		t.Run(method, func(t *testing.T) {
			fake := &commandHost{}
			defer host.Install(fake)()
			setAppContext(1, true)
			defer setAppContext(0, false)
			calls := 0
			SendCommand(`{"method":"`+method+`"}`, func(value string, err error) {
				calls++
				if value != "result" || err != nil {
					t.Fatalf("result = %q, %v", value, err)
				}
			})
			var payload struct{ Request uint32 }
			if err := json.Unmarshal([]byte(fake.payload), &payload); err != nil || payload.Request != fake.request {
				t.Fatalf("missing result request in %s: %v", fake.payload, err)
			}
			App.dispatchHostEvent(hostEvent{kind: "command", target: fake.request})
			if calls != 0 {
				t.Fatal("acceptance completed a deferred request")
			}
			App.dispatchHostEvent(hostEvent{kind: deferredReplyKind(method), target: fake.request, flags: 1, value: "result"})
			App.dispatchHostEvent(hostEvent{kind: "command", target: fake.request})
			if calls != 1 || replyKinds[fake.request] != "" {
				t.Fatal("result must complete exactly once and release its tracking entry")
			}
		})
	}
}

func TestDeferredCommandRejectionCompletesImmediately(t *testing.T) {
	fake := &commandHost{}
	defer host.Install(fake)()
	setAppContext(1, true)
	defer setAppContext(0, false)
	calls := 0
	SendCommand(`{"method":"window-popup-menu"}`, func(_ string, err error) {
		calls++
		if err == nil || err.Error() != "window closed" {
			t.Fatalf("error = %v", err)
		}
	})
	App.dispatchHostEvent(hostEvent{kind: "command", target: fake.request, flags: 2, extra: `{"error":"window closed"}`})
	if calls != 1 || len(pendingReplies) != 0 {
		t.Fatal("rejected request was retained")
	}
}

func TestPopupMenuKeepsActionsUntilDismissed(t *testing.T) {
	fake := &commandHost{}
	defer host.Install(fake)()
	setAppContext(1, true)
	defer setAppContext(0, false)
	window := &Window{NodeHost: NewNodeHost(1, 2)}
	clicked := false
	action := nextMenuAction
	PopupMenu(window, []MenuItem{{Label: "Open", Click: func() { clicked = true }}}, nil, nil, nil)
	App.dispatchHostEvent(hostEvent{kind: "command", target: fake.request})
	App.dispatchHostEvent(hostEvent{kind: "menu-action", target: action})
	if !clicked {
		t.Fatal("acceptance released the menu click handler")
	}
	App.dispatchHostEvent(hostEvent{kind: "popup-menu", target: fake.request})
	if _, retained := menuCallbacks[action]; retained {
		t.Fatal("dismissed menu retained its click handler")
	}
}

func TestPopupMenuRestoresWindowAndReactiveContext(t *testing.T) {
	fake := &commandHost{}
	defer host.Install(fake)()
	setAppContext(1, true)
	defer setAppContext(0, false)
	window := &Window{NodeHost: NewNodeHost(1, 2)}
	context := reactive.CreateContext("outside")
	calls := 0
	action := nextMenuAction
	reactive.CreateRoot(func(dispose func()) struct{} {
		defer dispose()
		context.Provide("repository", func() {
			PopupMenu(window, []MenuItem{{Label: "Open Repository", Click: func() {
				if CurrentWindow() != window || context.Use() != "repository" {
					t.Fatal("popup callback lost its originating window or component context")
				}
				calls++
			}}}, nil, nil, nil)
		})
		// Menu actions arrive later as app-level events, outside any window callback.
		App.dispatchHostEvent(hostEvent{kind: "menu-action", target: action})
		if calls != 1 || currentWindow != nil {
			t.Fatal("popup callback failed or leaked its window context")
		}
		window.Closed = true
		App.dispatchHostEvent(hostEvent{kind: "menu-action", target: action})
		if calls != 1 {
			t.Fatal("a closed window received a late menu action")
		}
		App.dispatchHostEvent(hostEvent{kind: "popup-menu", target: fake.request})
		return struct{}{}
	})
}
