package native

import (
	"encoding/json"

	"github.com/egoist/quickgui/go/host"
	"github.com/egoist/quickgui/go/protocol"
	"github.com/egoist/quickgui/go/reactive"
)

// Point is a logical-pixel coordinate.
type Point struct {
	X float64 `json:"x"`
	Y float64 `json:"y"`
}

// WindowOptions configure a native window and its root component.
type WindowOptions struct {
	Component               Component
	Title                   string
	Width                   float64
	Height                  float64
	Position                *Point
	InitialState            string
	DisplayID               string
	MinimumSize             *Size
	DisableMinimumSize      bool
	MinimumWidth            float64
	MinimumHeight           float64
	MaximumSize             *Size
	MaximumWidth            float64
	MaximumHeight           float64
	RepresentedFile         string
	DocumentEdited          *bool
	TabbingIdentifier       string
	Background              any
	BackgroundAppearance    string
	Vibrancy                string
	VisualEffectState       string
	PerformanceProfile      string
	Appearance              string
	TitleBarStyle           string
	Kind                    string
	Focus                   *bool
	Focusable               *bool
	Visible                 *bool
	Movable                 *bool
	Resizable               *bool
	Minimizable             *bool
	Maximizable             *bool
	Closable                *bool
	Decorated               *bool
	Shadow                  *bool
	ContentProtected        *bool
	WindowLevel             string
	SkipTaskbar             *bool
	VisibleOnAllWorkspaces  *bool
	Opacity                 *float64
	Icon                    *ImageSource
	TaskbarProgress         *TaskbarProgress
	TaskbarOverlay          *TaskbarOverlay
	CursorVisible           *bool
	CursorGrab              string
	CursorHitTest           *bool
	CursorPosition          *Point
	Menu                    []MenuDefinition
	RestoreState            *WindowRestoreState
	LineScrollPixels        *float64
	KeySequenceTimeoutMS    *float64
	ReduceMotion            *bool
	TrafficLightPosition    *Point
	Transparent             *bool
	Blur                    *bool
	Anchor                  *Node
	Placement               string
	Gap                     *float64
	Offset                  *Point
	ViewportMargin          *float64
	DismissOnEscape         *bool
	DismissOnPointerOutside *bool
	Grab                    *bool
	AcceptsKeyFocus         *bool
}

type nativeWindowOptions struct {
	Title                          string              `json:"title,omitempty"`
	Width                          *float64            `json:"width,omitempty"`
	Height                         *float64            `json:"height,omitempty"`
	X                              *float64            `json:"x,omitempty"`
	Y                              *float64            `json:"y,omitempty"`
	InitialState                   string              `json:"initialState,omitempty"`
	DisplayID                      string              `json:"displayId,omitempty"`
	MinimumSizeEnabled             *bool               `json:"minimumSizeEnabled,omitempty"`
	MinimumWidth                   *float64            `json:"minimumWidth,omitempty"`
	MinimumHeight                  *float64            `json:"minimumHeight,omitempty"`
	MaximumWidth                   *float64            `json:"maximumWidth,omitempty"`
	MaximumHeight                  *float64            `json:"maximumHeight,omitempty"`
	RepresentedFile                string              `json:"representedFile,omitempty"`
	DocumentEdited                 *bool               `json:"documentEdited,omitempty"`
	TabbingIdentifier              string              `json:"tabbingIdentifier,omitempty"`
	Background                     *uint32             `json:"background,omitempty"`
	PerformanceProfile             string              `json:"performanceProfile,omitempty"`
	Appearance                     string              `json:"appearance,omitempty"`
	Vibrancy                       string              `json:"vibrancy,omitempty"`
	VisualEffectState              string              `json:"visualEffectState,omitempty"`
	TitleBarStyle                  string              `json:"titleBarStyle,omitempty"`
	Kind                           string              `json:"kind,omitempty"`
	Focus                          *bool               `json:"focus,omitempty"`
	Focusable                      *bool               `json:"focusable,omitempty"`
	Show                           *bool               `json:"show,omitempty"`
	Movable                        *bool               `json:"movable,omitempty"`
	Resizable                      *bool               `json:"resizable,omitempty"`
	Minimizable                    *bool               `json:"minimizable,omitempty"`
	Maximizable                    *bool               `json:"maximizable,omitempty"`
	Closable                       *bool               `json:"closable,omitempty"`
	Decorated                      *bool               `json:"decorated,omitempty"`
	Shadow                         *bool               `json:"shadow,omitempty"`
	ContentProtected               *bool               `json:"contentProtected,omitempty"`
	WindowLevel                    string              `json:"windowLevel,omitempty"`
	SkipTaskbar                    *bool               `json:"skipTaskbar,omitempty"`
	VisibleOnAllWorkspaces         *bool               `json:"visibleOnAllWorkspaces,omitempty"`
	Opacity                        *float64            `json:"opacity,omitempty"`
	Icon                           *ImageSource        `json:"icon,omitempty"`
	TaskbarProgressState           string              `json:"taskbarProgressState,omitempty"`
	TaskbarProgress                *float64            `json:"taskbarProgress,omitempty"`
	TaskbarOverlayIcon             *ImageSource        `json:"taskbarOverlayIcon,omitempty"`
	TaskbarOverlayDescription      string              `json:"taskbarOverlayDescription,omitempty"`
	CursorVisible                  *bool               `json:"cursorVisible,omitempty"`
	CursorGrab                     string              `json:"cursorGrab,omitempty"`
	CursorHitTest                  *bool               `json:"cursorHitTest,omitempty"`
	CursorX                        *float64            `json:"cursorX,omitempty"`
	CursorY                        *float64            `json:"cursorY,omitempty"`
	Menu                           string              `json:"menu,omitempty"`
	RestoreState                   *WindowRestoreState `json:"restoreState,omitempty"`
	LineScrollPixels               *float64            `json:"lineScrollPixels,omitempty"`
	KeySequenceTimeoutMS           *float64            `json:"keySequenceTimeoutMs,omitempty"`
	ReduceMotion                   *bool               `json:"reduceMotion,omitempty"`
	TrafficLightX                  *float64            `json:"trafficLightX,omitempty"`
	TrafficLightY                  *float64            `json:"trafficLightY,omitempty"`
	Transparent                    *bool               `json:"transparent,omitempty"`
	Blur                           *bool               `json:"blur,omitempty"`
	PopoverPlacement               string              `json:"popoverPlacement,omitempty"`
	PopoverGap                     *float64            `json:"popoverGap,omitempty"`
	PopoverOffsetX                 *float64            `json:"popoverOffsetX,omitempty"`
	PopoverOffsetY                 *float64            `json:"popoverOffsetY,omitempty"`
	PopoverViewportMargin          *float64            `json:"popoverViewportMargin,omitempty"`
	PopoverDismissOnEscape         *bool               `json:"popoverDismissOnEscape,omitempty"`
	PopoverDismissOnPointerOutside *bool               `json:"popoverDismissOnPointerOutside,omitempty"`
	PopoverGrab                    *bool               `json:"popoverGrab,omitempty"`
	PopoverAcceptsKeyFocus         *bool               `json:"popoverAcceptsKeyFocus,omitempty"`
}

// WindowEventName is one window lifecycle notification.
type WindowEventName string

const (
	WindowClosed          WindowEventName = "closed"
	WindowCloseRequested  WindowEventName = "closeRequested"
	WindowFocus           WindowEventName = "focus"
	WindowBlur            WindowEventName = "blur"
	WindowResize          WindowEventName = "resize"
	WindowMove            WindowEventName = "move"
	WindowReadyToShow     WindowEventName = "readyToShow"
	WindowAppearance      WindowEventName = "appearanceChange"
	WindowMinimize        WindowEventName = "minimize"
	WindowRestore         WindowEventName = "restore"
	WindowMaximize        WindowEventName = "maximize"
	WindowUnmaximize      WindowEventName = "unmaximize"
	WindowEnterFullScreen WindowEventName = "enterFullScreen"
	WindowLeaveFullScreen WindowEventName = "leaveFullScreen"
	WindowOcclusionChange WindowEventName = "occlusionChange"
	WindowLevelChange     WindowEventName = "levelChange"
	WindowWillResize      WindowEventName = "willResize"
	WindowWillMove        WindowEventName = "willMove"
	WindowStateChange     WindowEventName = "stateChange"
)

type WindowEvent struct {
	Window     *Window
	Type       WindowEventName
	Appearance string
	Size       *Size
	Position   *Point
	Occluded   *bool
	Level      string
}

type windowListener struct {
	id       int
	Type     WindowEventName
	Listener func(WindowEvent)
	owner    *reactive.Owner
}

var nextWindowListenerID int

var currentWindow *Window
var componentWindow = reactive.CreateContext[*Window](nil)

func contextWindow() *Window {
	if window := componentWindow.Use(); window != nil {
		return window
	}
	return currentWindow
}

func withCurrentWindow(window *Window, fn func()) {
	previous := currentWindow
	currentWindow = window
	defer func() { currentWindow = previous }()
	fn()
}

// CurrentWindow is the Window whose component or event callback is running.
func CurrentWindow() *Window {
	window := contextWindow()
	if window == nil {
		panic("Window.Current() must be called while rendering or handling a window event")
	}
	return window
}

// Window is one native window and its retained tree.
type Window struct {
	*NodeHost
	Root              *Node
	App               *Application
	listeners         []windowListener
	mountDisposers    []func()
	menuIDs           []uint32
	closeIntercepting bool
}

// NewWindow allocates a handle, renders the tree, and queues native creation.
func NewWindow(options WindowOptions) *Window {
	if !App.IsReady() {
		panic("call native.Run before creating a QuickGUI Window")
	}
	window := &Window{
		NodeHost: NewNodeHost(App.NativeID, host.Current.AllocateWindow()),
		App:      App,
	}
	window.Root = CreateRootNode(window.NodeHost, protocol.RootNodeID)
	if options.Anchor != nil {
		parent := options.Anchor.Host
		if parent == nil || parent.Closed {
			panic("a system popover requires a mounted node in an open parent Window")
		}
		parent.Flush()
	}
	defer window.cleanupFailedConstruction()
	native := window.encodeOptions(options)
	encoded := mustString(native)
	var dispose func()
	withCurrentWindow(window, func() {
		if options.Component != nil {
			dispose = window.mountComponent(options.Component)
		}
	})
	if dispose != nil {
		window.mountDisposers = append(window.mountDisposers, dispose)
	}
	initial := window.TakeBatch()
	if options.Anchor != nil {
		parent := options.Anchor.Host
		if parent == nil || parent.Closed {
			panic("a system popover requires a mounted node in an open parent Window")
		}
		host.Current.CreateSystemPopover(window.AppID, window.NativeID, parent.NativeID, options.Anchor.ID, encoded, initial)
	} else {
		host.Current.CreateWindow(window.AppID, window.NativeID, encoded, initial)
	}
	window.NativeReady = true
	App.registerWindow(window)
	window.syncCloseInterception()
	return window
}

// NewEmbeddedWindow allocates a hidden child renderer whose native view a SwiftUI host owns.
func NewEmbeddedWindow(owner *Window, options WindowOptions, matchHorizontal, matchVertical bool) *Window {
	if owner == nil || owner.Closed {
		panic("an embedded QuickGUI view requires an open owner Window")
	}
	if !App.IsReady() {
		panic("call native.Run before creating a QuickGUI Window")
	}
	owner.Flush()
	window := &Window{
		NodeHost: NewNodeHost(App.NativeID, host.Current.AllocateWindow()),
		App:      App,
	}
	window.Root = CreateRootNode(window.NodeHost, protocol.RootNodeID)
	defer window.cleanupFailedConstruction()
	encoded := mustString(window.encodeOptions(options))
	var dispose func()
	withCurrentWindow(window, func() {
		if options.Component != nil {
			dispose = window.mountComponent(options.Component)
		}
	})
	if dispose != nil {
		window.mountDisposers = append(window.mountDisposers, dispose)
	}
	initial := window.TakeBatch()
	host.Current.CreateEmbeddedView(window.AppID, window.NativeID, owner.NativeID, matchHorizontal, matchVertical, encoded, initial)
	window.NativeReady = true
	App.registerWindow(window)
	window.syncCloseInterception()
	return window
}

// Each window owns one reactive root. Components execute once; bindings update nodes.
func (w *Window) mountComponent(component Component) func() {
	owner := reactive.NewOwner(reactive.GetOwner())
	dispose := func() { reactive.DisposeOwner(owner) }
	defer func() {
		if failure := recover(); failure != nil {
			dispose()
			panic(failure)
		}
	}()
	reactive.RunWithOwner(owner, func() struct{} {
		componentWindow.Provide(w, func() {
			reactive.OnCleanup(func() {
				if w.NativeReady && !w.Closed {
					w.Close()
				}
			})
			for _, node := range CollectChildren(component) {
				InsertNode(w.Root, node, nil)
			}
		})
		return struct{}{}
	})
	return dispose
}

func encodeWindowOptions(options WindowOptions) nativeWindowOptions {
	native := nativeWindowOptions{Title: options.Title}
	native.InitialState = options.InitialState
	native.DisplayID = options.DisplayID
	native.RepresentedFile = options.RepresentedFile
	native.DocumentEdited = options.DocumentEdited
	native.TabbingIdentifier = options.TabbingIdentifier
	native.PerformanceProfile = options.PerformanceProfile
	native.Appearance = options.Appearance
	native.Vibrancy = options.Vibrancy
	native.VisualEffectState = options.VisualEffectState
	native.TitleBarStyle = options.TitleBarStyle
	native.Kind = options.Kind
	native.Focus = options.Focus
	native.Focusable = options.Focusable
	native.Movable = options.Movable
	native.Resizable = options.Resizable
	native.Minimizable = options.Minimizable
	native.Maximizable = options.Maximizable
	native.Closable = options.Closable
	native.Decorated = options.Decorated
	native.Shadow = options.Shadow
	native.ContentProtected = options.ContentProtected
	native.WindowLevel = options.WindowLevel
	native.SkipTaskbar = options.SkipTaskbar
	native.VisibleOnAllWorkspaces = options.VisibleOnAllWorkspaces
	native.Opacity = options.Opacity
	native.Icon = options.Icon
	native.CursorVisible = options.CursorVisible
	native.CursorGrab = options.CursorGrab
	native.CursorHitTest = options.CursorHitTest
	native.RestoreState = options.RestoreState
	native.LineScrollPixels = options.LineScrollPixels
	native.KeySequenceTimeoutMS = options.KeySequenceTimeoutMS
	native.ReduceMotion = options.ReduceMotion
	native.PopoverPlacement = options.Placement
	native.PopoverGap = options.Gap
	native.PopoverViewportMargin = options.ViewportMargin
	native.PopoverDismissOnEscape = options.DismissOnEscape
	native.PopoverDismissOnPointerOutside = options.DismissOnPointerOutside
	native.PopoverGrab = options.Grab
	native.PopoverAcceptsKeyFocus = options.AcceptsKeyFocus
	if options.Width != 0 {
		native.Width = &options.Width
	}
	if options.Height != 0 {
		native.Height = &options.Height
	}
	if options.MinimumWidth != 0 {
		native.MinimumWidth = &options.MinimumWidth
	}
	if options.MinimumHeight != 0 {
		native.MinimumHeight = &options.MinimumHeight
	}
	if options.MaximumWidth != 0 {
		native.MaximumWidth = &options.MaximumWidth
	}
	if options.MaximumHeight != 0 {
		native.MaximumHeight = &options.MaximumHeight
	}
	if options.MinimumSize != nil {
		native.MinimumWidth = &options.MinimumSize.Width
		native.MinimumHeight = &options.MinimumSize.Height
	}
	if options.MaximumSize != nil {
		native.MaximumWidth = &options.MaximumSize.Width
		native.MaximumHeight = &options.MaximumSize.Height
	}
	if options.DisableMinimumSize {
		flag := false
		native.MinimumSizeEnabled = &flag
		native.MinimumWidth = nil
		native.MinimumHeight = nil
	}
	if options.Background != nil {
		color := ParseColor(options.Background)
		native.Background = &color
	}
	native.Show = options.Visible
	if options.BackgroundAppearance != "" {
		transparent, blur := options.BackgroundAppearance == "transparent", options.BackgroundAppearance == "blurred"
		native.Transparent = &transparent
		native.Blur = &blur
	} else {
		native.Transparent = options.Transparent
		native.Blur = options.Blur
	}
	if options.TaskbarProgress != nil {
		native.TaskbarProgressState = options.TaskbarProgress.State
		native.TaskbarProgress = &options.TaskbarProgress.Progress
	}
	if options.TaskbarOverlay != nil {
		native.TaskbarOverlayIcon = &options.TaskbarOverlay.Icon
		native.TaskbarOverlayDescription = options.TaskbarOverlay.Description
	}
	if options.Position != nil {
		native.X = &options.Position.X
		native.Y = &options.Position.Y
	}
	if options.CursorPosition != nil {
		native.CursorX = &options.CursorPosition.X
		native.CursorY = &options.CursorPosition.Y
	}
	if options.TrafficLightPosition != nil {
		native.TrafficLightX = &options.TrafficLightPosition.X
		native.TrafficLightY = &options.TrafficLightPosition.Y
	}
	if options.Offset != nil {
		native.PopoverOffsetX = &options.Offset.X
		native.PopoverOffsetY = &options.Offset.Y
	}
	return native
}

func (w *Window) On(eventType WindowEventName, listener func(WindowEvent)) func() {
	if w.Closed || listener == nil {
		return func() {}
	}
	nextWindowListenerID++
	entry := windowListener{id: nextWindowListenerID, Type: eventType, Listener: listener, owner: reactive.GetOwner()}
	w.listeners = append(w.listeners, entry)
	w.syncCloseInterception()
	stop := func() {
		for i, existing := range w.listeners {
			if existing.id == entry.id {
				w.listeners = append(w.listeners[:i], w.listeners[i+1:]...)
				w.syncCloseInterception()
				return
			}
		}
	}
	// Closed observers must survive component disposal during native teardown.
	if entry.owner != nil && eventType != WindowClosed {
		reactive.OnCleanup(stop)
	}
	return stop
}

func (w *Window) OnClose(listener func(*Window)) func() {
	if listener == nil {
		return func() {}
	}
	if w.Closed {
		listener(w)
		return func() {}
	}
	return w.On(WindowClosed, func(event WindowEvent) { listener(event.Window) })
}

func (w *Window) emit(eventType WindowEventName) {
	w.emitEvent(WindowEvent{Window: w, Type: eventType})
}

func (w *Window) emitEvent(event WindowEvent) {
	snapshot := append([]windowListener(nil), w.listeners...)
	withCurrentWindow(w, func() {
		for _, entry := range snapshot {
			if entry.Type != event.Type {
				continue
			}
			present := false
			for _, active := range w.listeners {
				if active.id == entry.id {
					present = true
					break
				}
			}
			if !present || event.Type != WindowClosed && entry.owner != nil && entry.owner.Disposed {
				continue
			}
			reactive.RunWithOwner(entry.owner, func() struct{} { reactive.Batch(func() { entry.Listener(event) }); return struct{}{} })
		}
	})
}

func (w *Window) Close() {
	w.App.closeWindow(w)
}

func (w *Window) Focus() {
	w.Action("focus", "")
}

func (w *Window) SetTitle(title string) {
	if w.Closed {
		return
	}
	SendMutation(mustString(map[string]any{"method": "window-action", "window": w.NativeID, "action": "set-title", "value": title}))
}

func (w *Window) SetRepresentedFile(path string) {
	w.Action("set-represented-file", path)
}

func (w *Window) Action(action, value string) {
	if w.Closed {
		return
	}
	payload := map[string]any{"method": "window-action", "window": w.NativeID, "action": action}
	if value != "" {
		payload["value"] = value
	}
	encoded, _ := json.Marshal(payload)
	SendMutation(string(encoded))
}

func (w *Window) didClose() {
	if w.Closed {
		return
	}
	w.Closed = true
	for i := len(w.mountDisposers) - 1; i >= 0; i-- {
		w.mountDisposers[i]()
	}
	w.mountDisposers = nil
	releaseMenuIDs(w.menuIDs)
	w.menuIDs = nil
	w.closeIntercepting = false
	w.emit(WindowClosed)
	for _, node := range w.Nodes {
		retire(node)
		node.Host = nil
	}
	w.Nodes = nil
	w.Root = nil
	w.Batch = protocol.NewBatch()
	w.listeners = nil
}

func (w *Window) didRequestClose() {
	if w.Closed {
		return
	}
	w.emit(WindowCloseRequested)
}

func (w *Window) didObserveLifecycle(kind, value string) {
	if w.Closed {
		return
	}
	switch kind {
	case "window-focus":
		if value == "true" {
			w.emit(WindowFocus)
		} else {
			w.emit(WindowBlur)
		}
	case "window-resize":
		fallthrough
	case "window-will-resize":
		var size Size
		if json.Unmarshal([]byte(value), &size) != nil {
			return
		}
		typeName := WindowResize
		if kind == "window-will-resize" {
			typeName = WindowWillResize
		}
		w.emitEvent(WindowEvent{Window: w, Type: typeName, Size: &size})
	case "window-move":
		fallthrough
	case "window-will-move":
		var position Point
		if json.Unmarshal([]byte(value), &position) != nil {
			return
		}
		typeName := WindowMove
		if kind == "window-will-move" {
			typeName = WindowWillMove
		}
		w.emitEvent(WindowEvent{Window: w, Type: typeName, Position: &position})
	case "window-ready-to-show":
		w.emit(WindowReadyToShow)
	case "window-appearance":
		appearance := "light"
		if value == "dark" {
			appearance = "dark"
		}
		w.emitEvent(WindowEvent{Window: w, Type: WindowAppearance, Appearance: appearance})
	case "window-minimize":
		if value == "true" {
			w.emit(WindowMinimize)
		} else {
			w.emit(WindowRestore)
		}
	case "window-maximize":
		if value == "true" {
			w.emit(WindowMaximize)
		} else {
			w.emit(WindowUnmaximize)
		}
	case "window-fullscreen":
		if value == "true" {
			w.emit(WindowEnterFullScreen)
		} else {
			w.emit(WindowLeaveFullScreen)
		}
	case "window-occlusion":
		occluded := value == "true"
		w.emitEvent(WindowEvent{Window: w, Type: WindowOcclusionChange, Occluded: &occluded})
	case "window-level":
		w.emitEvent(WindowEvent{Window: w, Type: WindowLevelChange, Level: value})
	case "window-state-change":
		w.emit(WindowStateChange)
	}
}

func (w *Window) cleanupFailedConstruction() {
	if failure := recover(); failure != nil {
		w.didClose()
		panic(failure)
	}
}

func (w *Window) syncCloseInterception() {
	if w.Closed || !w.NativeReady {
		return
	}
	intercepting := false
	for _, entry := range w.listeners {
		if entry.Type == WindowCloseRequested {
			intercepting = true
			break
		}
	}
	if intercepting == w.closeIntercepting {
		return
	}
	w.closeIntercepting = intercepting
	if intercepting {
		w.Action("set-close-interception", "true")
	} else {
		w.Action("set-close-interception", "false")
	}
}

// OnCloseRequested holds close requests until Close or Destroy is called.
func (w *Window) OnCloseRequested(listener func(*Window)) func() {
	if listener == nil {
		return func() {}
	}
	return w.On(WindowCloseRequested, func(event WindowEvent) { listener(event.Window) })
}

func (w *Window) Destroy() {
	if w.Closed {
		return
	}
	kept := w.listeners[:0]
	for _, entry := range w.listeners {
		if entry.Type != WindowCloseRequested {
			kept = append(kept, entry)
		}
	}
	w.listeners = kept
	w.syncCloseInterception()
	w.Close()
}

func (w *Window) encodeOptions(options WindowOptions) nativeWindowOptions {
	native := encodeWindowOptions(options)
	if options.Menu != nil {
		native.Menu = w.encodeMenu(options.Menu)
	}
	return native
}

func (w *Window) encodeMenu(definitions []MenuDefinition) string {
	ids := []uint32{}
	menus := make([]nativeMenuDefinition, 0, len(definitions))
	for _, definition := range definitions {
		menus = append(menus, nativeMenuDefinition{Label: definition.Label, Enabled: enabledOrTrue(definition.Enabled), Items: encodeMenuItems(definition.Items, &ids)})
	}
	for _, id := range ids {
		callback := menuCallbacks[id]
		callback.window = w
		callback.owner = reactive.GetOwner()
		menuCallbacks[id] = callback
	}
	releaseMenuIDs(w.menuIDs)
	w.menuIDs = ids
	return mustString(menus)
}

// SetMenu replaces the window menu; nil restores the application menu.
func (w *Window) SetMenu(definitions []MenuDefinition) {
	if w.Closed {
		return
	}
	if definitions == nil {
		releaseMenuIDs(w.menuIDs)
		w.menuIDs = nil
		w.Action("set-menu", "")
		return
	}
	w.Action("set-menu", w.encodeMenu(definitions))
}
