package native

import (
	"encoding/json"
	"testing"

	"github.com/egoist/quickgui/go/host"
)

type watchCall struct {
	request uint32
	method  string
	params  string
}

type watchHost struct {
	host.Fake
	calls []watchCall
}

func (h *watchHost) Invoke(request uint32, method, params string) {
	h.calls = append(h.calls, watchCall{request, method, params})
}

func TestStoppingWatcherBeforeRegistrationReleasesNativeWatch(t *testing.T) {
	fake := &watchHost{}
	defer host.Install(fake)()
	setAppContext(1, true)
	defer setAppContext(0, false)
	stop := WatchFiles([]string{"/repository"}, func(FileWatchEvent) {
		t.Fatal("stopped watcher received a change")
	}, func(error) { t.Fatal("stopped watcher received registration completion") })
	request := fake.calls[0].request
	var params struct{ ID uint32 }
	if err := json.Unmarshal([]byte(fake.calls[0].params), &params); err != nil {
		t.Fatal(err)
	}
	stop()
	stop()
	App.dispatchHostEvent(hostEvent{kind: "invoke", target: request})
	App.dispatchHostEvent(hostEvent{kind: "file-watch", target: params.ID, flags: 1, value: `{"paths":["/repository/file.go"]}`})
	if len(fake.calls) != 2 || fake.calls[1].method != "unwatch-files" {
		t.Fatal("native watch was leaked after late registration")
	}
	App.dispatchHostEvent(hostEvent{kind: "invoke", target: fake.calls[1].request})
	if len(fileWatchers) != 0 || len(pendingReplies) != 0 {
		t.Fatal("watcher retained callbacks after stopping")
	}
}
