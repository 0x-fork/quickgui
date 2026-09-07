package native

import (
	"encoding/json"
	"testing"

	"github.com/egoist/quickgui/go/host"
)

func TestIndependentExtensionRequestsReturnJSON(t *testing.T) {
	fake := &watchHost{}
	defer host.Install(fake)()
	setAppContext(1, true)
	defer setAppContext(0, false)
	const reply = `{"value":null,"nested":{"count":0}}`
	got := ""
	InvokeExtension("acme-echo", "send.message", map[string]any{"message": "hello"}, func(value string, err error) {
		if err != nil {
			t.Fatal(err)
		}
		got = value
	})
	if len(fake.calls) != 1 || fake.calls[0].method != "extension/acme-echo/send.message" {
		t.Fatalf("unexpected routing: %v", fake.calls)
	}
	App.dispatchHostEvent(hostEvent{kind: "invoke", target: fake.calls[0].request, flags: 1, value: reply})
	if got != reply || len(pendingReplies) != 0 {
		t.Fatal("JSON reply lost data or retained the callback")
	}
	for _, call := range [][2]string{{"../echo", "send"}, {"echo", "other/send"}, {"echo", ""}} {
		failed := false
		InvokeExtension(call[0], call[1], nil, func(_ string, err error) { failed = err != nil })
		if !failed || len(fake.calls) != 1 {
			t.Fatal("invalid name or method crossed the native boundary")
		}
	}
}

func TestSessionRequestPreservesItsResultAndEventStream(t *testing.T) {
	fake := &watchHost{}
	defer host.Install(fake)()
	setAppContext(1, true)
	defer setAppContext(0, false)
	events := 0
	session := OpenExtension("acme-echo", nil, func(string) { events++ }, nil)
	request := fake.calls[0].request
	App.dispatchHostEvent(hostEvent{kind: "invoke", target: request})
	got := ""
	session.Request("lookup", "key", func(value string, err error) {
		if err != nil {
			t.Fatal(err)
		}
		got = value
	})
	var envelope struct {
		Session uint32 `json:"session"`
		Value   string `json:"value"`
	}
	if err := json.Unmarshal([]byte(fake.calls[1].params), &envelope); err != nil || envelope.Session != request || envelope.Value != "key" {
		t.Fatalf("invalid session envelope: %v %v", envelope, err)
	}
	App.dispatchHostEvent(hostEvent{kind: "invoke", target: fake.calls[1].request, flags: 1, value: `{"answer":42}`})
	App.dispatchHostEvent(hostEvent{kind: "extension-event", target: request})
	if got != `{"answer":42}` || events != 1 {
		t.Fatal("request lost its result or removed the event stream")
	}
	for _, method := range []string{"start", "stop"} {
		failed := false
		session.Request(method, nil, func(_ string, err error) { failed = err != nil })
		if !failed || len(fake.calls) != 2 {
			t.Fatal("session request bypassed lifecycle ownership")
		}
	}
	session.Close()
	App.dispatchHostEvent(hostEvent{kind: "invoke", target: fake.calls[2].request})
	session.Request("lookup", nil, func(_ string, err error) {
		if err == nil {
			t.Fatal("request to closed session succeeded")
		}
	})
	if len(fake.calls) != 3 || len(extensionListeners) != 0 || len(pendingReplies) != 0 {
		t.Fatal("closed session dispatched work or leaked callbacks")
	}
}

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
