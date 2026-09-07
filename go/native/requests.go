package native

import (
	"encoding/json"
	"fmt"

	"github.com/egoist/quickgui/go/host"
)

const readyRequest uint32 = 1

var nextRequest uint32 = 2
var appID uint32
var appReady bool

var pendingReplies = map[uint32]func(string, error){}
var replyKinds = map[uint32]string{}
var pendingProgress = map[uint32]func(string){}

func setAppContext(id uint32, ready bool) {
	appID = id
	appReady = ready
}

func allocateRequest() uint32 {
	if nextRequest >= 0xffff_fff0 {
		panic("QuickGUI request id space exhausted")
	}
	request := nextRequest
	nextRequest++
	return request
}

type dialogReply struct {
	complete func(value string, paths []string, err error)
}

var pendingDialogs = map[uint32]dialogReply{}

func settleReply(request uint32, value string, err error) {
	done, ok := pendingReplies[request]
	if !ok {
		return
	}
	delete(pendingReplies, request)
	delete(replyKinds, request)
	delete(pendingProgress, request)
	done(value, err)
}

func settleHostReply(kind string, request uint32, value string, err error) {
	// A deferred command first acknowledges acceptance, then delivers its result.
	// In particular, popup-menu acceptance must not release its click handlers.
	if expected := replyKinds[request]; expected != "" && kind != expected && err == nil {
		return
	}
	settleReply(request, value, err)
}

func settleDialog(request uint32, value string, paths []string, err error) {
	pending, ok := pendingDialogs[request]
	if !ok {
		return
	}
	delete(pendingDialogs, request)
	pending.complete(value, paths, err)
}

func rejectAllReplies(err error) {
	for request, done := range pendingReplies {
		delete(pendingReplies, request)
		delete(replyKinds, request)
		delete(pendingProgress, request)
		done("", err)
	}
	for request, pending := range pendingDialogs {
		delete(pendingDialogs, request)
		pending.complete("", nil, err)
	}
}

func assertAppReady() {
	if appID == 0 || !appReady {
		panic("call native.Run before using the native QuickGUI application")
	}
}

// SendMutation queues one fire-and-forget system command.
func SendMutation(payload string) {
	assertAppReady()
	host.Current.Command(appID, 0, payload)
}

// SendCommand queues a system command and invokes done when the host replies.
func SendCommand(payload string, done func(json string, err error)) {
	if done == nil {
		done = func(string, error) {}
	}
	if appID == 0 || !appReady {
		done("", fmt.Errorf("call native.Run before using the native QuickGUI application"))
		return
	}
	request := allocateRequest()
	var command map[string]json.RawMessage
	if err := json.Unmarshal([]byte(payload), &command); err != nil || command == nil {
		done("", fmt.Errorf("invalid QuickGUI command JSON"))
		return
	}
	var method string
	_ = json.Unmarshal(command["method"], &method)
	if kind := deferredReplyKind(method); kind != "" {
		command["request"] = json.RawMessage(fmt.Sprint(request))
		encoded, err := json.Marshal(command)
		if err != nil {
			done("", err)
			return
		}
		payload = string(encoded)
		replyKinds[request] = kind
	}
	pendingReplies[request] = done
	host.Current.Command(appID, request, payload)
}

func deferredReplyKind(method string) string {
	switch method {
	case "app-service", "file-icon", "notification-permission":
		return method
	case "window-popup-menu":
		return "popup-menu"
	case "shell-action":
		return "shell"
	case "global-shortcut":
		return "global-shortcut-operation"
	case "set-tray-icon", "remove-tray-icon", "show-tray-menu":
		return "tray-operation"
	case "set-user-tasks":
		return "user-tasks"
	default:
		return ""
	}
}

// Invoke queues a native service request. Its result runs on the UI goroutine.
func Invoke(method string, params any, done func(json string, err error)) {
	invokeWithProgress(method, params, nil, done)
}

func invokeWithProgress(method string, params any, progress func(string), done func(string, error)) {
	if done == nil {
		done = func(string, error) {}
	}
	if appID == 0 || !appReady {
		done("", fmt.Errorf("call native.Run before using the native QuickGUI application"))
		return
	}
	encoded, err := json.Marshal(params)
	if err != nil {
		done("", err)
		return
	}
	request := allocateRequest()
	pendingReplies[request] = done
	if progress != nil {
		pendingProgress[request] = progress
	}
	host.Current.Invoke(request, method, string(encoded))
}

func isNullJSON(text string) bool {
	return text == "" || text == "null"
}

// CallService answers one CPU-only host service synchronously and returns the JSON text of its value.
// The router uses this channel; it never waits on native main-thread execution.
func CallService(method, params string) (string, error) {
	return callService(method, params)
}

func callService(method, params string) (string, error) {
	replyText := host.Current.Call(method, params)
	var reply struct {
		OK    bool            `json:"ok"`
		Value json.RawMessage `json:"value"`
		Error string          `json:"error"`
	}
	if err := json.Unmarshal([]byte(replyText), &reply); err != nil {
		return "", fmt.Errorf("malformed native service reply: %w", err)
	}
	if !reply.OK {
		return "", fmt.Errorf("%s", reply.Error)
	}
	return string(reply.Value), nil
}
