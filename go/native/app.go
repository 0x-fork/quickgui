package native

import (
	"encoding/base64"
	"encoding/json"
	"errors"
	"fmt"
	"os"
	"path/filepath"
	"runtime"
	"sync/atomic"

	"github.com/egoist/quickgui/go/host"
	"github.com/egoist/quickgui/go/protocol"
	"github.com/egoist/quickgui/go/reactive"
)

// App is the process-wide native application.
var App = &Application{}

// Application holds native app identity, windows, and lifecycle listeners.
type Application struct {
	NativeID         uint32
	Windows          map[uint32]*Window
	ready            bool
	exited           bool
	onReady          subscriptions[struct{}]
	onReopen         subscriptions[ReopenEvent]
	onQuit           subscriptions[QuitEvent]
	onBeforeQuit     subscriptions[QuitPhaseEvent]
	onWillQuit       subscriptions[QuitPhaseEvent]
	onOpenURLs       subscriptions[OpenURLsEvent]
	onActivate       subscriptions[struct{}]
	onDeactivate     subscriptions[struct{}]
	onSystemWake     subscriptions[struct{}]
	onKeyboardLayout subscriptions[KeyboardLayout]
	onSecondInstance subscriptions[SecondInstanceEvent]
	quitIntercepting bool
}

// ReopenEvent is a macOS dock-click or equivalent reopen.
type ReopenEvent struct {
	HasVisibleWindows bool
}

// AppOptions configure identity and quit policy before the first readiness turn.
type AppOptions struct {
	Name       string
	Version    string
	Identifier string
	QuitMode   string
	Fonts      []string
	Paths      AppPathOverrides
}

type nativeAppOptions struct {
	ResourceDir  string   `json:"resourceDir,omitempty"`
	Name         string   `json:"name,omitempty"`
	Version      string   `json:"version,omitempty"`
	Identifier   string   `json:"identifier,omitempty"`
	QuitMode     string   `json:"quitMode,omitempty"`
	Fonts        []string `json:"fonts,omitempty"`
	ConfigDir    string   `json:"configDir,omitempty"`
	DataDir      string   `json:"dataDir,omitempty"`
	LocalDataDir string   `json:"localDataDir,omitempty"`
	CacheDir     string   `json:"cacheDir,omitempty"`
	LogDir       string   `json:"logDir,omitempty"`
	RuntimeDir   string   `json:"runtimeDir,omitempty"`
	TempDir      string   `json:"tempDir,omitempty"`
}

func (a *Application) IsReady() bool { return a.ready }

func (a *Application) OnReady(listener func()) func() {
	if listener == nil {
		return func() {}
	}
	if a.ready {
		listener()
		return func() {}
	}
	return a.onReady.add(func(struct{}) { listener() })
}

func (a *Application) OnReopen(listener func(ReopenEvent)) func() {
	return a.onReopen.add(listener)
}

func (a *Application) registerWindow(window *Window) {
	if a.Windows == nil {
		a.Windows = map[uint32]*Window{}
	}
	a.Windows[window.NativeID] = window
}

func (a *Application) closeWindow(window *Window) {
	if window.Closed {
		return
	}
	host.Current.CloseWindow(a.NativeID, window.NativeID)
}

func (a *Application) didCloseWindow(window *Window) {
	delete(a.Windows, window.NativeID)
	window.didClose()
}

func (a *Application) dispatchHostEvent(ev hostEvent) {
	var extra struct {
		Error  string   `json:"error"`
		Paths  []string `json:"paths"`
		Width  uint32   `json:"width"`
		Height uint32   `json:"height"`
	}
	hasValue := ev.flags&1 != 0
	hasExtra := ev.flags&2 != 0
	if hasExtra && ev.extra != "" {
		_ = json.Unmarshal([]byte(ev.extra), &extra)
	}
	value := ev.value
	if !hasValue {
		value = ""
	}
	switch ev.kind {
	case "app-ready":
		if extra.Error != "" {
			panic(extra.Error)
		}
		a.ready = true
		setAppContext(a.NativeID, true)
		a.syncQuitInterception()
		a.onReady.emit(struct{}{})
		a.onReady.clear()
		return
	case "file-icon":
		var err error
		if extra.Error != "" {
			err = fmt.Errorf("%s", extra.Error)
		} else if ev.flags&4 == 0 || extra.Width == 0 || extra.Height == 0 || uint64(len(ev.data)) != uint64(extra.Width)*uint64(extra.Height)*4 {
			err = fmt.Errorf("native file icon response has invalid dimensions or pixel data")
		}
		encoded := ""
		if err == nil {
			encoded = mustString(NativeImage{Data: ev.data, Width: extra.Width, Height: extra.Height})
		}
		settleHostReply(ev.kind, ev.target, encoded, err)
		return
	case "command", "invoke", "shell", "popup-menu", "app-service",
		"notification-permission", "global-shortcut-operation", "tray-operation", "user-tasks":
		var err error
		if extra.Error != "" {
			err = fmt.Errorf("%s", extra.Error)
		}
		settleHostReply(ev.kind, ev.target, value, err)
		return
	case "alert-dialog", "open-dialog", "save-dialog":
		var err error
		if extra.Error != "" {
			err = fmt.Errorf("%s", extra.Error)
		}
		settleDialog(ev.target, value, extra.Paths, err)
		return
	case "menu-action":
		dispatchMenuAction(ev.target)
		return
	case "global-shortcut":
		dispatchShortcut(ev.target)
		return
	case "tray-event":
		dispatchTrayEvent(ev.target, value)
		return
	case "file-watch":
		dispatchFileWatch(ev.target, value)
		return
	case "extension-event":
		if listener := extensionListeners[ev.target]; listener != nil {
			listener(value)
		}
		return
	case "update-progress":
		if progress := pendingProgress[ev.target]; progress != nil {
			progress(value)
		}
		return
	case "power-event":
		var event PowerEvent
		if json.Unmarshal([]byte(value), &event) == nil {
			powerListeners.emit(event)
		}
		return
	case "notification-response":
		var event NotificationResponseEvent
		if json.Unmarshal([]byte(value), &event) == nil {
			notificationListeners.emit(event)
		}
		return
	case "screen-change":
		if len(screenListeners.order) != 0 {
			Screen.GetAllDisplays(func(displays []Display, err error) {
				if err == nil {
					screenListeners.emit(displays)
				}
			})
		}
		return
	case "system-preferences-change":
		if len(preferencesListeners.order) != 0 {
			SystemPreferences.GetCurrent(func(preferences SystemPreferencesSnapshot, err error) {
				if err == nil {
					preferencesListeners.emit(preferences)
				}
			})
		}
		return
	case "exit":
		a.didExit(int(ev.target))
		return
	case "host-error":
		msg := extra.Error
		if msg == "" {
			msg = "the native host failed"
		}
		panic(msg)
	case "reopen":
		event := ReopenEvent{HasVisibleWindows: value == "true"}
		a.onReopen.emit(event)
		return
	}
	if a.dispatchAppLifecycle(ev.kind, value) {
		return
	}
	owner := a.Windows[ev.window]
	if owner == nil {
		return
	}
	if ev.kind == "close" {
		a.didCloseWindow(owner)
		return
	}
	if ev.kind == "close-requested" {
		owner.didRequestClose()
		return
	}
	if len(ev.kind) >= 7 && ev.kind[:7] == "window-" {
		owner.didObserveLifecycle(ev.kind, value)
		return
	}
	eventType := protocol.EventTypeFromKind(ev.kind)
	if eventType == 0 {
		return
	}
	withCurrentWindow(owner, func() {
		DispatchEvent(owner.NodeHost, eventType, ev.target, value, hasValue)
	})
}

// Keep Go's initial goroutine on the process main thread before user main starts.
func init() { runtime.LockOSThread() }

var buildMetadata string // Injected by the TypeScript CLI.
var hasRun atomic.Bool

// Run owns the AppKit/Winit main thread. Components and events execute on one
// dedicated Go goroutine pinned to an OS thread, including CPU-only Rust services.
// Call Run once, from main. Dispatch schedules background results onto the UI loop.
func Run(start func(), options ...AppOptions) error {
	if !hasRun.CompareAndSwap(false, true) {
		return errors.New("native.Run may only be called once")
	}
	if err := host.Load(); err != nil {
		return err
	}
	host.Current.SetEventCallback(enqueueHostEvent)
	defer host.Current.ClearEventCallback()
	exited := make(chan error, 1)
	go func() {
		runtime.LockOSThread()
		defer runtime.UnlockOSThread()
		var result error
		defer func() {
			// Always release Run's waiter, even if application cleanup panics.
			defer func() {
				if recovered := recover(); recovered != nil {
					result = errors.Join(result, fmt.Errorf("QuickGUI cleanup: %v", recovered))
				}
				exited <- result
			}()
			if recovered := recover(); recovered != nil {
				result = fmt.Errorf("QuickGUI application: %v", recovered)
				host.Current.Abort(result.Error())
			}
			App.ready = false
			setAppContext(0, false)
			rejectAllReplies(errors.New("the QuickGUI application stopped"))
			for _, window := range App.Windows {
				window.didClose()
			}
			App.Windows = nil
			App.clearListeners()
			fileWatchers = map[uint32]func(FileWatchEvent){}
			extensionListeners = map[uint32]func(string){}
			menuCallbacks = map[uint32]menuCallback{}
			screenListeners.clear()
			preferencesListeners.clear()
			powerListeners.clear()
			notificationListeners.clear()
			for _, registration := range shortcutRegistrations {
				registration.release()
			}
			for _, icon := range trayIcons {
				icon.removed = true
				icon.callbacks = nil
				icon.listeners.clear()
			}
			trayIcons = map[uint32]*TrayIcon{}
		}()
		runApplication(start, options)
	}()
	code := host.Current.RunHost()
	close(loopStopped)
	err := <-exited
	if err != nil {
		return err
	}
	if code != 0 {
		return fmt.Errorf("QuickGUI host exited with status %d", code)
	}
	return nil
}

func runApplication(start func(), options []AppOptions) {
	if version := host.Current.ProtocolVersion(); version != protocol.Version {
		panic(fmt.Sprintf("QuickGUI protocol mismatch: Go=%d Rust=%d", protocol.Version, version))
	}
	config := nativeAppOptions{}
	if buildMetadata != "" {
		encoded, err := base64.RawURLEncoding.DecodeString(buildMetadata)
		if err != nil {
			panic(err)
		}
		if err = json.Unmarshal(encoded, &config); err != nil {
			panic(err)
		}
	}
	if len(options) > 0 {
		explicit := options[0]
		if explicit.Name != "" {
			config.Name = explicit.Name
		}
		if explicit.Version != "" {
			config.Version = explicit.Version
		}
		if explicit.Identifier != "" {
			config.Identifier = explicit.Identifier
		}
		if explicit.QuitMode != "" {
			config.QuitMode = explicit.QuitMode
		}
		if explicit.Fonts != nil {
			config.Fonts = explicit.Fonts
		}
		config.applyPaths(explicit.Paths)
	}
	executable, _ := os.Executable()
	if config.ResourceDir == "" {
		config.ResourceDir = applicationResourceDirectory(executable)
	}
	encoded, err := json.Marshal(config)
	if err != nil {
		panic(err)
	}
	id := host.Current.CreateApp(string(encoded))
	if id == 0 {
		panic("the QuickGUI host refused to create the application")
	}
	App.NativeID = id
	setAppContext(id, false)
	host.Current.PrepareApp(id, readyRequest)
	for !App.ready && !App.exited {
		waitTurn()
	}
	if App.exited {
		return
	}
	start()
	reactive.Flush()
	FlushPending()
	for !App.exited {
		waitTurn()
	}
}

// macOS bundles put assets in Contents/Resources. Other distributions keep
// resources beside the executable; neither layout depends on the launch cwd.
func applicationResourceDirectory(executable string) string {
	directory := filepath.Dir(executable)
	if filepath.Base(directory) == "MacOS" {
		resources := filepath.Join(directory, "..", "Resources")
		if info, err := os.Stat(resources); err == nil && info.IsDir() {
			return resources
		}
	}
	return directory
}

func waitTurn() {
	select {
	case <-workWake:
		processTurns()
	case <-loopStopped:
		processTurns() // Deliver lifecycle events queued before the native loop returned.
		App.didExit(0)
	}
}

func processTurns() {
	for {
		select {
		case err := <-loopFailures:
			panic(err)
		default:
		}
		events, jobs := drainEvents(), drainWork()
		if len(events) == 0 && len(jobs) == 0 {
			return
		}
		reactive.Batch(func() {
			for _, event := range events {
				App.dispatchHostEvent(event)
			}
			for _, job := range jobs {
				if !App.exited && job != nil {
					job()
				}
			}
		})
		reactive.Flush()
		FlushPending()
	}
}
