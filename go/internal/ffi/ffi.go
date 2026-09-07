// Package ffi is the only unsafe boundary. Native calls copy all input spans;
// callbacks copy all output spans before returning to Rust.
package ffi

import (
	"encoding/json"
	"fmt"
	"runtime"
	"sync"
	"unsafe"

	"github.com/ebitengine/purego"
)

type Event struct {
	Flags          uint32
	Extra          string
	Kind           string
	Window, Target uint32
	Value          string
	Paths          []string
	Data           []byte
	Width, Height  uint32
	Error          string
}

type Library struct {
	handle              uintptr // Keep loaded for process lifetime: AppKit and service threads retain code pointers.
	ProtocolVersion     func() uint32
	RegisterExtension   func(uintptr, []byte, uintptr) int32
	RunHost             func() int32
	SetEventCallback    func(uintptr, uintptr)
	ClearEventCallback  func(uintptr, uintptr)
	CreateApp           func([]byte, uintptr) uint32
	PrepareApp          func(uint32, uint32) int32
	AllocateWindow      func() uint32
	CreateWindow        func(uint32, uint32, []byte, uintptr, []byte, uintptr) int32
	CreateSystemPopover func(uint32, uint32, uint32, uint32, []byte, uintptr, []byte, uintptr) int32
	CreateEmbeddedView  func(uint32, uint32, uint32, uint8, uint8, []byte, uintptr, []byte, uintptr) int32
	ApplyBatch          func(uint32, uint32, []byte, uintptr) int32
	CloseWindow         func(uint32, uint32) int32
	FocusNode           func(uint32, uint32, uint32) int32
	ShowDialog          func(uint32, uint32, uint32, uint32, []byte, uintptr) int32
	Command             func(uint32, uint32, []byte, uintptr) int32
	Invoke              func(uint32, []byte, uintptr, []byte, uintptr) int32
	Call                func([]byte, uintptr, []byte, uintptr, uintptr, uintptr) int32
	DestroyApp          func(uint32) int32
	Abort               func(string)
}

func Load(path string, protocol uint32) (lib *Library, err error) {
	handle, err := openLibrary(path)
	if err != nil {
		return nil, fmt.Errorf("load QuickGUI library %s: %w", path, err)
	}
	lib = &Library{handle: handle}
	defer func() {
		if p := recover(); p != nil {
			lib = nil
			err = fmt.Errorf("incompatible QuickGUI shared library: %v", p)
		}
	}()
	for _, entry := range []struct {
		name string
		fn   any
	}{
		{"protocol_version", &lib.ProtocolVersion}, {"run_host", &lib.RunHost},
		{"register_extension", &lib.RegisterExtension},
		{"set_event_callback", &lib.SetEventCallback}, {"clear_event_callback", &lib.ClearEventCallback},
		{"create_app", &lib.CreateApp}, {"prepare_app", &lib.PrepareApp},
		{"allocate_window", &lib.AllocateWindow}, {"create_window", &lib.CreateWindow},
		{"create_system_popover", &lib.CreateSystemPopover}, {"create_embedded_view", &lib.CreateEmbeddedView},
		{"apply_batch", &lib.ApplyBatch}, {"close_window", &lib.CloseWindow}, {"focus_node", &lib.FocusNode},
		{"show_dialog", &lib.ShowDialog}, {"command", &lib.Command}, {"invoke", &lib.Invoke},
		{"call", &lib.Call}, {"destroy_app", &lib.DestroyApp}, {"abort", &lib.Abort},
	} {
		purego.RegisterLibFunc(entry.fn, handle, "quickgui_"+entry.name)
	}
	if got := lib.ProtocolVersion(); got != protocol {
		return nil, fmt.Errorf("QuickGUI protocol mismatch: Go=%d Rust=%d; rebuild the shared library", protocol, got)
	}
	return lib, nil
}

// Foreign span addresses stay integers in the callback frame: an empty native
// slice can have address 1, which Go's stack scanner rejects as a Go pointer.
// Convert only non-empty spans while the native caller still owns their memory.
//
//go:nocheckptr
func copySpan(pointer uintptr, length uintptr) []byte {
	if length == 0 {
		return nil
	}
	if pointer < 4096 || length > 64*1024*1024 {
		panic("quickgui: invalid native span")
	}
	return append([]byte(nil), unsafe.Slice((*byte)(unsafe.Pointer(pointer)), int(length))...)
}

// Allocate two trampolines for the entire process, not one per element or request.
var callbackOnce sync.Once
var eventCallback, replyCallback uintptr
var callbackLock sync.RWMutex
var eventReceiver func(Event)
var replies sync.Map

func callbacks() {
	callbackOnce.Do(func() {
		eventCallback = purego.NewCallback(func(_ purego.CDecl, kind, kindLen uintptr, window, target, flags uint32, value, valueLen, extra, extraLen, data, dataLen uintptr, _ uintptr) uintptr {
			e := Event{Kind: string(copySpan(kind, kindLen)), Window: window, Target: target, Flags: flags}
			if flags&1 != 0 {
				e.Value = string(copySpan(value, valueLen))
			}
			if flags&2 != 0 {
				e.Extra = string(copySpan(extra, extraLen))
				if err := json.Unmarshal([]byte(e.Extra), &e); err != nil {
					e.Error = err.Error()
					e.Kind = "host-error"
				}
			}
			if flags&4 != 0 {
				e.Data = copySpan(data, dataLen)
			}
			callbackLock.RLock()
			receive := eventReceiver
			callbackLock.RUnlock()
			if receive != nil {
				receive(e)
			}
			return 0
		})
		replyCallback = purego.NewCallback(func(_ purego.CDecl, data, length, context uintptr) uintptr {
			if channel, ok := replies.Load(context); ok {
				channel.(chan []byte) <- copySpan(data, length)
			}
			return 0
		})
	})
}

func (lib *Library) Listen(receive func(Event)) func() {
	callbacks()
	callbackLock.Lock()
	eventReceiver = receive
	callbackLock.Unlock()
	lib.SetEventCallback(eventCallback, 0)
	return func() {
		lib.ClearEventCallback(eventCallback, 0)
		callbackLock.Lock()
		eventReceiver = nil
		callbackLock.Unlock()
	}
}

// CallJSON only invokes CPU-only Rust services; it never waits for the platform
// main thread. Router state is thread-local, so callers use the locked UI goroutine.
func (lib *Library) CallJSON(id uintptr, method string, params []byte) ([]byte, error) {
	callbacks()
	channel := make(chan []byte, 1)
	replies.Store(id, channel)
	defer replies.Delete(id)
	m := []byte(method)
	status := lib.Call(m, uintptr(len(m)), params, uintptr(len(params)), replyCallback, id)
	runtime.KeepAlive(m)
	runtime.KeepAlive(params)
	if status != 0 {
		return nil, fmt.Errorf("native service %s failed", method)
	}
	select {
	case b := <-channel:
		var result struct {
			OK    bool            `json:"ok"`
			Value json.RawMessage `json:"value"`
			Error string          `json:"error"`
		}
		if err := json.Unmarshal(b, &result); err != nil {
			return nil, err
		}
		if !result.OK {
			return nil, fmt.Errorf("%s", result.Error)
		}
		return result.Value, nil
	default:
		return nil, fmt.Errorf("native service %s did not reply", method)
	}
}
