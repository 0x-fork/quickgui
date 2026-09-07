package native

import (
	"testing"

	"github.com/egoist/quickgui/go/host"
)

func TestExtensionEventsSurviveStartAndCommandRepliesUntilClose(t *testing.T) {
	fake := &watchHost{}
	defer host.Install(fake)()
	setAppContext(1, true)
	defer setAppContext(0, false)
	events, ready := 0, 0
	session := OpenExtension("updater", map[string]any{}, func(string) { events++ }, func(err error) {
		if err != nil {
			t.Fatal(err)
		}
		ready++
	})
	request := fake.calls[0].request
	App.dispatchHostEvent(hostEvent{kind: "invoke", target: request})
	App.dispatchHostEvent(hostEvent{kind: "extension-event", target: request, value: `{"kind":"state"}`})
	session.Command("check", true, nil)
	App.dispatchHostEvent(hostEvent{kind: "invoke", target: fake.calls[1].request})
	App.dispatchHostEvent(hostEvent{kind: "extension-event", target: request, value: `{"kind":"state"}`})
	if ready != 1 || events != 2 {
		t.Fatalf("ready=%d events=%d", ready, events)
	}
	session.Close()
	session.Close()
	App.dispatchHostEvent(hostEvent{kind: "extension-event", target: request})
	if events != 2 || len(fake.calls) != 3 || fake.calls[2].method != "extension/updater/stop" {
		t.Fatal("extension stream was retained or closed twice")
	}
	App.dispatchHostEvent(hostEvent{kind: "invoke", target: fake.calls[2].request})
	if len(extensionListeners) != 0 || len(pendingReplies) != 0 {
		t.Fatal("extension callbacks leaked")
	}
}

func TestClosingExtensionBeforeStartFailureReleasesCallbacks(t *testing.T) {
	fake := &watchHost{}
	defer host.Install(fake)()
	setAppContext(1, true)
	defer setAppContext(0, false)
	session := OpenExtension("updater", nil, func(string) { t.Fatal("event after close") }, func(error) { t.Fatal("ready after close") })
	session.Close()
	App.dispatchHostEvent(hostEvent{kind: "invoke", target: fake.calls[0].request, flags: 2, extra: `{"error":"start failed"}`})
	if len(fake.calls) != 1 || len(extensionListeners) != 0 || len(pendingReplies) != 0 {
		t.Fatal("failed session retained callbacks or tried to stop an unregistered session")
	}
}

func TestClosingExtensionBeforeStartReplyStopsItAfterRegistration(t *testing.T) {
	fake := &watchHost{}
	defer host.Install(fake)()
	setAppContext(1, true)
	defer setAppContext(0, false)
	session := OpenExtension("updater", nil, func(string) { t.Fatal("event after close") }, func(error) { t.Fatal("ready after close") })
	session.Close()
	App.dispatchHostEvent(hostEvent{kind: "invoke", target: fake.calls[0].request})
	if len(fake.calls) != 2 || fake.calls[1].method != "extension/updater/stop" {
		t.Fatal("native session leaked")
	}
	App.dispatchHostEvent(hostEvent{kind: "invoke", target: fake.calls[1].request})
	if len(extensionListeners) != 0 || len(pendingReplies) != 0 {
		t.Fatal("callback leaked")
	}
}
