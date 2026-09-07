package ffi

import (
	"os"
	"os/exec"
	"runtime"
	"testing"
	"unsafe"

	"github.com/ebitengine/purego"
)

func TestCallbackEmptyNativeSpansSurviveStackGrowth(t *testing.T) {
	// A bad pointer aborts the runtime, so exercise the real trampoline in a child.
	if os.Getenv("QUICKGUI_CALLBACK_STACK_TEST") != "1" {
		command := exec.Command(os.Args[0], "-test.run=^TestCallbackEmptyNativeSpansSurviveStackGrowth$")
		command.Env = append(os.Environ(), "QUICKGUI_CALLBACK_STACK_TEST=1", "GODEBUG=invalidptr=1")
		if output, err := command.CombinedOutput(); err != nil {
			t.Fatalf("callback crashed: %v\n%s", err, output)
		}
		return
	}
	callbacks()
	calls := 0
	eventReceiver = func(event Event) {
		growCallbackStack(80)
		if event.Kind != "windowState" || event.Window != 7 || event.Target != 2 || event.Width != 1080 || event.Height != 780 || event.Value != "" || len(event.Data) != 0 {
			t.Fatalf("corrupt callback payload: %+v", event)
		}
		calls++
	}
	defer func() { eventReceiver = nil }()
	var emit func(uintptr, uintptr, uint32, uint32, uint32, uintptr, uintptr, uintptr, uintptr, uintptr, uintptr, uintptr) uintptr
	purego.RegisterFunc(&emit, eventCallback)
	kind, extra := []byte("windowState"), []byte(`{"width":1080,"height":780}`)
	var pin runtime.Pinner
	pin.Pin(&kind[0])
	pin.Pin(&extra[0])
	defer pin.Unpin()
	for range 10 {
		// Empty native slices may use the non-null, non-dereferenceable address 1.
		emit(uintptr(unsafe.Pointer(&kind[0])), uintptr(len(kind)), 7, 2, 7, 1, 0, uintptr(unsafe.Pointer(&extra[0])), uintptr(len(extra)), 1, 0, 0)
	}
	if calls != 10 {
		t.Fatalf("received %d events", calls)
	}
	var reply func(uintptr, uintptr, uintptr) uintptr
	purego.RegisterFunc(&reply, replyCallback)
	channel := make(chan []byte, 1)
	replies.Store(uintptr(101), channel)
	defer replies.Delete(uintptr(101))
	reply(1, 0, 101)
	if len(<-channel) != 0 {
		t.Fatal("empty reply was not empty")
	}
}

//go:noinline
func growCallbackStack(depth int) byte {
	var frame [2048]byte
	frame[depth] = byte(depth)
	if depth > 0 {
		frame[0] = growCallbackStack(depth - 1)
	} else {
		runtime.GC()
	}
	return frame[depth] + frame[0]
}
