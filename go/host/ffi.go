package host

import (
	"encoding/json"
	"fmt"
	"sync/atomic"

	"github.com/egoist/quickgui/go/internal/ffi"
	"github.com/egoist/quickgui/go/protocol"
)

type loaded struct {
	library  *ffi.Library
	unlisten func()
	nextCall atomic.Uint64
}

func bindLibrary(path string) (API, error) {
	library, err := ffi.Load(path, protocol.Version)
	if err != nil {
		return nil, err
	}
	return &loaded{library: library}, nil
}

func check(status int32) {
	if status != 0 {
		panic("QuickGUI native command queue rejected the operation")
	}
}

func (l *loaded) ProtocolVersion() uint32 { return l.library.ProtocolVersion() }

func (l *loaded) SetEventCallback(handler EventHandler) {
	l.ClearEventCallback()
	l.unlisten = l.library.Listen(func(e ffi.Event) {
		handler(e.Kind, e.Window, e.Target, e.Flags, e.Value, e.Extra, e.Data)
	})
}

func (l *loaded) ClearEventCallback() {
	if l.unlisten != nil {
		l.unlisten()
		l.unlisten = nil
	}
}

func (l *loaded) CreateApp(options string) uint32 {
	bytes := []byte(options)
	return l.library.CreateApp(bytes, uintptr(len(bytes)))
}

func (l *loaded) PrepareApp(app, request uint32) { check(l.library.PrepareApp(app, request)) }

func (l *loaded) AllocateWindow() uint32 { return l.library.AllocateWindow() }

func (l *loaded) CreateWindow(app, window uint32, options string, batch []byte) {
	bytes := []byte(options)
	check(l.library.CreateWindow(app, window, bytes, uintptr(len(bytes)), batch, uintptr(len(batch))))
}

func (l *loaded) CreateSystemPopover(app, window, parent, anchor uint32, options string, batch []byte) {
	bytes := []byte(options)
	check(l.library.CreateSystemPopover(app, window, parent, anchor, bytes, uintptr(len(bytes)), batch, uintptr(len(batch))))
}

func (l *loaded) CreateEmbeddedView(app, window, parent uint32, horizontal, vertical bool, options string, batch []byte) {
	bytes := []byte(options)
	var h, v uint8
	if horizontal {
		h = 1
	}
	if vertical {
		v = 1
	}
	check(l.library.CreateEmbeddedView(app, window, parent, h, v, bytes, uintptr(len(bytes)), batch, uintptr(len(batch))))
}

func (l *loaded) ApplyBatch(app, window uint32, batch []byte) {
	check(l.library.ApplyBatch(app, window, batch, uintptr(len(batch))))
}

func (l *loaded) CloseWindow(app, window uint32) { check(l.library.CloseWindow(app, window)) }

func (l *loaded) FocusNode(app, window, node uint32) { check(l.library.FocusNode(app, window, node)) }

func (l *loaded) ShowDialog(app, window, request, kind uint32, options string) {
	bytes := []byte(options)
	check(l.library.ShowDialog(app, window, request, kind, bytes, uintptr(len(bytes))))
}

func (l *loaded) Command(app, request uint32, command string) {
	bytes := []byte(command)
	check(l.library.Command(app, request, bytes, uintptr(len(bytes))))
}

func (l *loaded) Invoke(request uint32, method, params string) {
	m, p := []byte(method), []byte(params)
	check(l.library.Invoke(request, m, uintptr(len(m)), p, uintptr(len(p))))
}

func (l *loaded) Call(method, params string) string {
	value, err := l.library.CallJSON(uintptr(l.nextCall.Add(1)), method, []byte(params))
	if err != nil {
		encoded, _ := json.Marshal(map[string]any{"ok": false, "error": err.Error()})
		return string(encoded)
	}
	return fmt.Sprintf(`{"ok":true,"value":%s}`, value)
}

func (l *loaded) DestroyApp(app uint32) { check(l.library.DestroyApp(app)) }

func (l *loaded) RunHost() int { return int(l.library.RunHost()) }

func (l *loaded) Abort(message string) { l.library.Abort(message) }
