package native

import (
	"encoding/json"
	"fmt"
	"regexp"

	"github.com/egoist/quickgui/go/host"
)

var extensionListeners = map[uint32]func(string){}
var extensionServiceName = regexp.MustCompile(`^[a-z][a-z0-9-]{0,63}$`)
var extensionMethodName = regexp.MustCompile(`^[A-Za-z][A-Za-z0-9_.-]{0,63}$`)

// InvokeExtension calls an imported provider without creating a session. The
// successful reply is JSON, and the callback runs on the UI goroutine.
func InvokeExtension(name, method string, value any, done func(string, error)) {
	if !extensionServiceName.MatchString(name) || !extensionMethodName.MatchString(method) {
		if done != nil {
			done("", fmt.Errorf("invalid extension name or method"))
		}
		return
	}
	Invoke("extension/"+name+"/"+method, value, done)
}

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
	if !extensionServiceName.MatchString(name) {
		fail(fmt.Errorf("invalid extension name"))
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
	s.Request(method, value, func(_ string, err error) { done(err) })
}

// Request sends a session command and returns its JSON result. It does not
// replace the event subscription established by OpenExtension.
func (s *ExtensionSession) Request(method string, value any, done func(string, error)) {
	if done == nil {
		done = func(string, error) {}
	}
	if s == nil || s.closed || !s.ready {
		done("", fmt.Errorf("extension is not ready or has been closed"))
		return
	}
	if method == "start" || method == "stop" {
		done("", fmt.Errorf("use OpenExtension and Close for session lifecycle"))
		return
	}
	InvokeExtension(s.name, method, map[string]any{"session": s.id, "value": value}, done)
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
