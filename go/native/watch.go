package native

import "encoding/json"

// FileWatchEvent is an operating-system filesystem notification from the Rust core.
type FileWatchEvent struct {
	Paths  []string `json:"paths"`
	Rescan bool     `json:"rescan"`
	Error  string   `json:"error"`
}

var fileWatchers = map[uint32]func(FileWatchEvent){}

// WatchFiles registers a recursive native watcher without polling. Call stop on
// the UI goroutine. Registration and errors complete asynchronously through ready.
func WatchFiles(paths []string, changed func(FileWatchEvent), ready func(error)) (stop func()) {
	assertAppReady()
	id := allocateRequest()
	registered, stopped := false, false
	fileWatchers[id] = changed
	unwatch := func() {
		if appReady {
			Invoke("unwatch-files", map[string]any{"id": id}, func(string, error) {})
		}
	}
	Invoke("watch-files", map[string]any{"id": id, "paths": paths}, func(_ string, err error) {
		registered = err == nil
		if stopped && registered {
			unwatch()
		}
		if err != nil {
			delete(fileWatchers, id)
		}
		if !stopped && ready != nil {
			ready(err)
		}
	})
	return func() {
		if stopped {
			return
		}
		stopped = true
		delete(fileWatchers, id)
		if registered {
			unwatch()
		}
	}
}

func dispatchFileWatch(id uint32, raw string) {
	if changed := fileWatchers[id]; changed != nil {
		var event FileWatchEvent
		if err := json.Unmarshal([]byte(raw), &event); err != nil {
			event.Error = err.Error()
		}
		changed(event)
	}
}
