package native

import (
	"encoding/json"
	"fmt"
	"strconv"
)

type AppPathOverrides struct {
	ResourceDir  string
	ConfigDir    string
	DataDir      string
	LocalDataDir string
	CacheDir     string
	LogDir       string
	RuntimeDir   string
	TempDir      string
}

type AppInfo struct {
	Name       string `json:"name"`
	Version    string `json:"version"`
	Identifier string `json:"identifier"`
}

type SystemInfo struct {
	OperatingSystem    string   `json:"operatingSystem"`
	Family             string   `json:"family"`
	Name               string   `json:"name"`
	Version            string   `json:"version,omitempty"`
	Edition            string   `json:"edition,omitempty"`
	Codename           string   `json:"codename,omitempty"`
	Architecture       string   `json:"architecture"`
	Bitness            string   `json:"bitness"`
	Hostname           string   `json:"hostname,omitempty"`
	Locale             string   `json:"locale,omitempty"`
	PreferredLanguages []string `json:"preferredLanguages"`
	LanguagesTruncated bool     `json:"languagesTruncated"`
}

type RelaunchOptions struct {
	Executable       string   `json:"executable,omitempty"`
	Arguments        []string `json:"arguments,omitempty"`
	ClearArguments   bool     `json:"clearArguments,omitempty"`
	WorkingDirectory string   `json:"workingDirectory,omitempty"`
}

type QuitEvent struct{ ExitCode int }
type QuitPhaseEvent struct{ Reason string }
type OpenURLsEvent struct{ URLs []string }
type SecondInstanceEvent struct {
	Argv []string `json:"argv"`
	Cwd  string   `json:"cwd"`
}

func (options *nativeAppOptions) applyPaths(paths AppPathOverrides) {
	if paths.ResourceDir != "" {
		options.ResourceDir = paths.ResourceDir
	}
	if paths.ConfigDir != "" {
		options.ConfigDir = paths.ConfigDir
	}
	if paths.DataDir != "" {
		options.DataDir = paths.DataDir
	}
	if paths.LocalDataDir != "" {
		options.LocalDataDir = paths.LocalDataDir
	}
	if paths.CacheDir != "" {
		options.CacheDir = paths.CacheDir
	}
	if paths.LogDir != "" {
		options.LogDir = paths.LogDir
	}
	if paths.RuntimeDir != "" {
		options.RuntimeDir = paths.RuntimeDir
	}
	if paths.TempDir != "" {
		options.TempDir = paths.TempDir
	}
}

func (a *Application) GetInfo(done func(*AppInfo, error)) {
	commandJSON(map[string]any{"method": "get-app-info"}, done)
}
func (a *Application) GetSystemInfo(done func(SystemInfo, error)) {
	commandJSON(map[string]any{"method": "get-system-info"}, done)
}
func (a *Application) IsPackaged() (bool, error) {
	return callJSON[bool]("is-application-packaged", nil)
}
func (a *Application) OnQuit(listener func(QuitEvent)) func() { return a.onQuit.add(listener) }

// OnBeforeQuit holds ordinary native quit requests until Quit(true, ...) or
// Exit(...) completes the decision. Removing the last listener stops interception.
func (a *Application) OnBeforeQuit(listener func(QuitPhaseEvent)) func() {
	a.onBeforeQuit.changed = a.syncQuitInterception
	return a.onBeforeQuit.add(listener)
}
func (a *Application) OnWillQuit(listener func(QuitPhaseEvent)) func() {
	return a.onWillQuit.add(listener)
}
func (a *Application) OnOpenURLs(listener func(OpenURLsEvent)) func() {
	return a.onOpenURLs.add(listener)
}
func (a *Application) OnActivate(listener func()) func() {
	return subscribeVoid(&a.onActivate, listener)
}
func (a *Application) OnDeactivate(listener func()) func() {
	return subscribeVoid(&a.onDeactivate, listener)
}
func (a *Application) OnSystemWake(listener func()) func() {
	return subscribeVoid(&a.onSystemWake, listener)
}
func (a *Application) OnKeyboardLayoutChange(listener func(KeyboardLayout)) func() {
	return a.onKeyboardLayout.add(listener)
}
func (a *Application) OnNotificationResponse(listener func(NotificationResponseEvent)) func() {
	return Notifications.OnResponse(listener)
}
func (a *Application) OnSecondInstance(listener func(SecondInstanceEvent)) func() {
	return a.onSecondInstance.add(listener)
}

func subscribeVoid(listeners *subscriptions[struct{}], listener func()) func() {
	if listener == nil {
		return func() {}
	}
	return listeners.add(func(struct{}) { listener() })
}

func (a *Application) syncQuitInterception() {
	intercepting := len(a.onBeforeQuit.order) != 0
	if !a.ready || a.exited || intercepting == a.quitIntercepting {
		return
	}
	a.quitIntercepting = intercepting
	SendMutation(mustString(map[string]any{"method": "set-quit-interception", "intercepting": intercepting}))
}

func (a *Application) RequestSingleInstanceLock(identifier string, done func(bool, error)) {
	commandJSON(map[string]any{"method": "request-single-instance-lock", "identifier": identifier}, done)
}
func (a *Application) ReleaseSingleInstanceLock(done func(bool, error)) {
	commandJSON(map[string]any{"method": "release-single-instance-lock"}, done)
}

// Quit requests orderly shutdown. force completes a held quit, bypassing interception.
func (a *Application) Quit(force bool, done func(bool, error)) {
	method := "request-quit"
	if force {
		method = "exit"
	}
	commandJSON(map[string]any{"method": method}, done)
}

func (a *Application) Exit(code int, done func(bool, error)) {
	if code < 0 || code > 255 {
		if done != nil {
			done(false, fmt.Errorf("an exit code must be between 0 and 255"))
		}
		return
	}
	commandJSON(map[string]any{"method": "exit-with-code", "code": code}, done)
}

func (a *Application) SetActivationPolicy(policy string, done func(error)) {
	commandVoid(map[string]any{"method": "app-service", "action": "set-activation-policy", "value": policy}, done)
}
func (a *Application) Focus(steal bool) { appMutation("activate", strconv.FormatBool(steal)) }
func (a *Application) Hide()            { appMutation("hide", "") }
func (a *Application) Show()            { appMutation("unhide", "") }
func (a *Application) SetSecureKeyboardEntryEnabled(enabled bool) {
	appMutation("set-secure-keyboard-entry", strconv.FormatBool(enabled))
}
func (a *Application) IsInApplicationsFolder(done func(bool, error)) {
	commandJSON(map[string]any{"method": "get-applications-folder-support"}, func(support struct {
		AlreadyInstalled bool `json:"alreadyInstalled"`
	}, err error) {
		if done != nil {
			done(support.AlreadyInstalled, err)
		}
	})
}
func (a *Application) MoveToApplicationsFolder(done func(bool, error)) {
	commandJSON(map[string]any{"method": "app-service", "action": "move-to-applications-folder"}, done)
}
func (a *Application) Relaunch(options RelaunchOptions, done func(bool, error)) {
	commandJSON(map[string]any{"method": "relaunch", "options": options}, done)
}
func (a *Application) DockBounce(critical bool, done func(int64, error)) {
	value := "informational"
	if critical {
		value = "critical"
	}
	commandJSON(map[string]any{"method": "app-service", "action": "request-dock-attention", "value": value}, done)
}
func (a *Application) DockCancelBounce(id int64) {
	appMutation("cancel-dock-attention", strconv.FormatInt(id, 10))
}
func (a *Application) DockHide(done func(error)) {
	commandVoid(map[string]any{"method": "app-service", "action": "set-dock-visible", "value": "false"}, done)
}
func (a *Application) DockShow(done func(error)) {
	commandVoid(map[string]any{"method": "app-service", "action": "set-dock-visible", "value": "true"}, done)
}

func appMutation(action, value string) {
	SendMutation(mustString(map[string]any{"method": "app-mutation", "action": action, "value": value}))
}

func (a *Application) dispatchAppLifecycle(kind, value string) bool {
	switch kind {
	case "open-urls":
		var urls []string
		if json.Unmarshal([]byte(value), &urls) == nil {
			a.onOpenURLs.emit(OpenURLsEvent{URLs: urls})
		}
	case "system-wake":
		a.onSystemWake.emit(struct{}{})
	case "app-activate":
		a.onActivate.emit(struct{}{})
	case "app-deactivate":
		a.onDeactivate.emit(struct{}{})
	case "before-quit":
		a.onBeforeQuit.emit(QuitPhaseEvent{Reason: value})
	case "will-quit":
		a.onWillQuit.emit(QuitPhaseEvent{Reason: value})
	case "keyboard-layout-change":
		if len(a.onKeyboardLayout.order) != 0 {
			Keyboard.GetLayout(func(layout KeyboardLayout, err error) {
				if err == nil {
					a.onKeyboardLayout.emit(layout)
				}
			})
		}
	case "second-instance":
		var event SecondInstanceEvent
		if json.Unmarshal([]byte(value), &event) == nil {
			a.onSecondInstance.emit(event)
			if urls := URLsFromArguments(event.Argv); len(urls) != 0 {
				a.onOpenURLs.emit(OpenURLsEvent{URLs: urls})
			}
		}
	default:
		return false
	}
	return true
}

func (a *Application) didExit(code int) {
	if a.exited {
		return
	}
	a.exited = true
	a.ready = false
	setAppContext(0, false)
	for _, window := range a.Windows {
		a.didCloseWindow(window)
	}
	a.onQuit.emit(QuitEvent{ExitCode: code})
}

func (a *Application) clearListeners() {
	a.onReady.clear()
	a.onReopen.clear()
	a.onQuit.clear()
	a.onBeforeQuit.clear()
	a.onWillQuit.clear()
	a.onOpenURLs.clear()
	a.onActivate.clear()
	a.onDeactivate.clear()
	a.onSystemWake.clear()
	a.onKeyboardLayout.clear()
	a.onSecondInstance.clear()
}
