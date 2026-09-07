package main

import (
	"errors"
	"testing"

	"github.com/egoist/quickgui/go/native"
)

func testState() (*systemState, *string, *bool) {
	message, busy := "ready", false
	return &systemState{
		window:  &native.Window{NodeHost: native.NewNodeHost(1, 1)},
		status:  func(value string) { message = value },
		busy:    func() bool { return busy },
		setBusy: func(value bool) { busy = value },
	}, &message, &busy
}

func TestActionsSerializePendingOperationsAndRecoverFromFailure(t *testing.T) {
	state, message, busy := testState()
	var complete completion
	starts := 0
	action := systemAction{"Read", func(done completion) { starts++; complete = done }}
	state.run(action)
	state.run(action)
	if starts != 1 || !*busy || *message != "Read…" {
		t.Fatalf("pending operation: starts=%d busy=%v message=%q", starts, *busy, *message)
	}
	complete("", errors.New("unavailable"))
	if *busy || *message != "Read failed: unavailable" {
		t.Fatalf("failed operation: busy=%v message=%q", *busy, *message)
	}
	state.run(action)
	complete("received", nil)
	if starts != 2 || *busy || *message != "received" {
		t.Fatalf("retry: starts=%d busy=%v message=%q", starts, *busy, *message)
	}
}

func TestClosedWindowRejectsLateResultsAndFollowupOperations(t *testing.T) {
	state, message, _ := testState()
	var complete completion
	state.run(systemAction{"Read", func(done completion) { complete = done }})
	state.window.Closed = true
	complete("late result", nil)
	if *message != "Read…" {
		t.Fatalf("late result updated a closed window: %q", *message)
	}
	called := false
	after(state, complete, func(bool) { called = true })(true, nil)
	state.run(systemAction{"Start again", func(completion) { called = true }})
	if called {
		t.Fatal("a closed window started another native operation")
	}
}
