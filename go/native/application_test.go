package native

import (
	"encoding/json"
	"reflect"
	"testing"

	"github.com/egoist/quickgui/go/reactive"
)

func TestQuitInterceptionTracksListenerDisposalAndIsIdempotent(t *testing.T) {
	fake := installCommandHost(t)
	application := &Application{NativeID: 1, ready: true}
	var stop func()
	reactive.CreateRoot(func(dispose func()) struct{} {
		stop = application.OnBeforeQuit(func(event QuitPhaseEvent) {
			if event.Reason != "operating-system" {
				t.Fatalf("quit reason = %q", event.Reason)
			}
			application.Quit(true, nil)
		})
		if fake.payload != `{"intercepting":true,"method":"set-quit-interception"}` {
			t.Fatalf("interception = %s", fake.payload)
		}
		application.dispatchHostEvent(hostEvent{kind: "before-quit", flags: 1, value: "operating-system"})
		if fake.payload != `{"method":"exit"}` {
			t.Fatalf("held quit did not continue: %s", fake.payload)
		}
		replyCommand(fake, "command", "true", "")
		dispose()
		if fake.payload != `{"intercepting":false,"method":"set-quit-interception"}` {
			t.Fatalf("disposal left quit intercepted: %s", fake.payload)
		}
		fake.payload = "unchanged"
		stop()
		if fake.payload != "unchanged" {
			t.Fatal("unsubscribe ran twice")
		}
		return struct{}{}
	})
}

func TestAppLifecycleDeliversDeepLinksKeyboardAndExitStatus(t *testing.T) {
	fake := installCommandHost(t)
	application := &Application{NativeID: 1, ready: true}
	var urls []string
	var args []string
	var exits []int
	application.OnOpenURLs(func(event OpenURLsEvent) { urls = append(urls, event.URLs...) })
	application.OnSecondInstance(func(event SecondInstanceEvent) { args = event.Argv })
	application.OnQuit(func(event QuitEvent) { exits = append(exits, event.ExitCode) })
	application.dispatchHostEvent(hostEvent{kind: "open-urls", flags: 1, value: `["quickgui://first"]`})
	application.dispatchHostEvent(hostEvent{kind: "second-instance", flags: 1, value: `{"argv":["--flag","quickgui://second"],"cwd":"/tmp"}`})
	if !reflect.DeepEqual(urls, []string{"quickgui://first", "quickgui://second"}) || len(args) != 2 {
		t.Fatal("application links or second instance arguments were lost")
	}
	layout := ""
	application.OnKeyboardLayoutChange(func(event KeyboardLayout) { layout = event.ID })
	application.dispatchHostEvent(hostEvent{kind: "keyboard-layout-change"})
	if layout != "" || fake.payload != `{"method":"get-keyboard-layout"}` {
		t.Fatal("keyboard layout change did not query native state asynchronously")
	}
	replyCommand(fake, "command", `{"id":"com.apple.keylayout.US","name":"U.S."}`, "")
	if layout != "com.apple.keylayout.US" {
		t.Fatal("keyboard layout was not delivered")
	}
	application.dispatchHostEvent(hostEvent{kind: "exit", target: 23})
	application.dispatchHostEvent(hostEvent{kind: "exit", target: 0})
	if application.ready || !application.exited || !reflect.DeepEqual(exits, []int{23}) {
		t.Fatal("exit status was not emitted exactly once")
	}
}

func TestAppPathsKeepExplicitResourceDirectoryAndAllOverrides(t *testing.T) {
	options := nativeAppOptions{ResourceDir: "/bundle/resources", DataDir: "/default"}
	options.applyPaths(AppPathOverrides{ResourceDir: "/custom/resources", DataDir: "/custom/data", LogDir: "/logs", RuntimeDir: "/run"})
	var encoded map[string]any
	if err := json.Unmarshal([]byte(mustString(options)), &encoded); err != nil {
		t.Fatal(err)
	}
	if encoded["resourceDir"] != "/custom/resources" || encoded["dataDir"] != "/custom/data" || encoded["logDir"] != "/logs" || encoded["runtimeDir"] != "/run" {
		t.Fatalf("paths = %+v", encoded)
	}
}
