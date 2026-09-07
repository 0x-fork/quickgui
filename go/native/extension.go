package native

import (
	"encoding/json"
	"fmt"

	"github.com/egoist/quickgui/go/host"
)

var extensionListeners = map[uint32]func(string){}

// ExtensionSession is an application-owned native service. Its callbacks run on
// the UI goroutine. Close it on that goroutine when the service is no longer used.
type ExtensionSession struct {
	name   string
	id     uint32
	closed bool
	ready  bool
}

// OpenExtension starts an imported service extension asynchronously. The provider
// retains its event stream until Close; command replies never remove the stream.
func OpenExtension(name string, options any, changed func(string), ready func(error)) *ExtensionSession {
	s := &ExtensionSession{name: name}
	fail := func(err error) {
		wasClosed := s.closed
		s.closed = true
		if !wasClosed && ready != nil {
			ready(err)
		}
	}
	if !appReady {
		fail(fmt.Errorf("call native.Run before starting an extension"))
		return s
	}
	encoded, err := json.Marshal(options)
	if err != nil {
		fail(err)
		return s
	}
	s.id = allocateRequest()
	extensionListeners[s.id] = changed
	pendingReplies[s.id] = func(_ string, err error) {
		if err != nil {
			delete(extensionListeners, s.id)
			fail(err)
			return
		}
		s.ready = true
		if s.closed {
			s.stop()
			return
		}
		if ready != nil {
			ready(nil)
		}
	}
	host.Current.Invoke(s.id, "extension/"+name+"/start", string(encoded))
	return s
}

func (s *ExtensionSession) Command(method string, value any, done func(error)) {
	if done == nil {
		done = func(error) {}
	}
	if s == nil || s.closed || !s.ready {
		done(fmt.Errorf("extension is not ready or has been closed"))
		return
	}
	Invoke("extension/"+s.name+"/"+method, map[string]any{"session": s.id, "value": value}, func(_ string, err error) { done(err) })
}

func (s *ExtensionSession) stop() {
	if appReady {
		Invoke("extension/"+s.name+"/stop", map[string]any{"session": s.id}, nil)
	}
}

func (s *ExtensionSession) Close() {
	if s == nil || s.closed {
		return
	}
	s.closed = true
	delete(extensionListeners, s.id)
	if s.ready {
		s.stop()
	}
}
