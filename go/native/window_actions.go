package native

import "strconv"

// Window actions enqueue mutations; they never wait for the native main thread.
func (w *Window) SetBounds(bounds WindowBounds) { w.Action("set-bounds", mustString(bounds)) }
func (w *Window) SetPosition(position Point)    { w.Action("move", mustString(position)) }
func (w *Window) SetSize(size Size)             { w.Action("resize", mustString(size)) }
func (w *Window) Minimize()                     { w.Action("minimize", "") }
func (w *Window) Maximize()                     { w.Action("maximize", "") }
func (w *Window) Restore()                      { w.Action("restore", "") }
func (w *Window) Show()                         { w.Action("set-visible", "true") }
func (w *Window) Hide()                         { w.Action("set-visible", "false") }
func (w *Window) RequestAttention()             { w.Action("request-attention", "") }
func (w *Window) SetFullscreen(value bool)      { w.Action("set-fullscreen", strconv.FormatBool(value)) }
func (w *Window) SetResizable(value bool)       { w.Action("set-resizable", strconv.FormatBool(value)) }
func (w *Window) SetMovable(value bool)         { w.Action("set-movable", strconv.FormatBool(value)) }
func (w *Window) SetMinimizable(value bool)     { w.Action("set-minimizable", strconv.FormatBool(value)) }
func (w *Window) SetMaximizable(value bool)     { w.Action("set-maximizable", strconv.FormatBool(value)) }
func (w *Window) SetClosable(value bool)        { w.Action("set-closable", strconv.FormatBool(value)) }
func (w *Window) SetDecorated(value bool)       { w.Action("set-decorated", strconv.FormatBool(value)) }
func (w *Window) SetShadow(value bool)          { w.Action("set-shadow", strconv.FormatBool(value)) }
func (w *Window) SetContentProtected(value bool) {
	w.Action("set-content-protected", strconv.FormatBool(value))
}
func (w *Window) SetWindowLevel(level string) { w.Action("set-window-level", level) }
func (w *Window) SetFocusable(value bool)     { w.Action("set-focusable", strconv.FormatBool(value)) }
func (w *Window) SetSkipTaskbar(value bool)   { w.Action("set-skip-taskbar", strconv.FormatBool(value)) }
func (w *Window) SetVisibleOnAllWorkspaces(value bool) {
	w.Action("set-visible-on-all-workspaces", strconv.FormatBool(value))
}
func (w *Window) SetOpacity(value float64) {
	w.Action("set-opacity", strconv.FormatFloat(value, 'g', -1, 64))
}
func (w *Window) SetMinimumSize(size *Size) {
	if size == nil {
		w.Action("set-minimum-size", "")
	} else {
		w.Action("set-minimum-size", mustString(size))
	}
}
func (w *Window) SetMaximumSize(size *Size) {
	if size == nil {
		w.Action("set-maximum-size", "")
	} else {
		w.Action("set-maximum-size", mustString(size))
	}
}
func (w *Window) SetIcon(icon ImageSource) { w.ImageAction("set-icon", &icon, "") }
func (w *Window) ClearIcon()               { w.ImageAction("clear-icon", nil, "") }
func (w *Window) SetTaskbarProgress(state string, progress float64) {
	w.Action("set-taskbar-progress", mustString(TaskbarProgress{State: state, Progress: progress}))
}
func (w *Window) SetTaskbarOverlayIcon(icon ImageSource, description string) {
	w.ImageAction("set-taskbar-overlay-icon", &icon, description)
}
func (w *Window) ClearTaskbarOverlayIcon() { w.Action("clear-taskbar-overlay-icon", "") }
func (w *Window) SetCursorVisible(value bool) {
	w.Action("set-cursor-visible", strconv.FormatBool(value))
}
func (w *Window) SetCursorGrab(mode string) { w.Action("set-cursor-grab", mode) }
func (w *Window) SetCursorHitTest(value bool) {
	w.Action("set-cursor-hit-test", strconv.FormatBool(value))
}
func (w *Window) SetCursorPosition(position Point) {
	w.Action("set-cursor-position", mustString(position))
}
func (w *Window) SetDocumentEdited(value bool) {
	w.Action("set-document-edited", strconv.FormatBool(value))
}
func (w *Window) SetAppearance(value string)           { w.Action("set-appearance", value) }
func (w *Window) SetBackgroundAppearance(value string) { w.Action("set-background-appearance", value) }
func (w *Window) SetVibrancy(value string)             { w.Action("set-vibrancy", value) }
func (w *Window) SetVisualEffectState(value string)    { w.Action("set-visual-effect-state", value) }
func (w *Window) ShowCharacterPalette()                { w.Action("show-character-palette", "") }
func (w *Window) SetAlwaysOnTop(flag bool, level string) {
	value := map[string]any{"flag": flag}
	if level != "" {
		value["level"] = level
	}
	w.Action("set-always-on-top", mustString(value))
}
func (w *Window) MoveTop() { w.Action("move-top", "") }
func (w *Window) MoveAbove(other *Window) {
	if other == nil || other.Closed {
		panic("ordering requires an open target window")
	}
	if other == w {
		panic("a window cannot be ordered above itself")
	}
	w.Action("move-above", strconv.FormatUint(uint64(other.NativeID), 10))
}
func (w *Window) SetIgnoreMouseEvents(ignore, forward bool) {
	w.Action("set-ignore-mouse-events", mustString(map[string]any{"ignore": ignore, "forward": forward}))
}
func (w *Window) SetEnabled(value bool) { w.Action("set-enabled", strconv.FormatBool(value)) }
func (w *Window) SetAspectRatio(ratio *Size) {
	if ratio == nil {
		w.Action("set-aspect-ratio", "")
	} else {
		w.Action("set-aspect-ratio", mustString(ratio))
	}
}
func (w *Window) SetWindowButtonVisibility(value bool) {
	w.Action("set-window-button-visibility", strconv.FormatBool(value))
}
func (w *Window) SetResizePolicy(policy *WindowResizePolicy) {
	if policy == nil {
		w.Action("set-resize-policy", "")
	} else {
		w.Action("set-resize-policy", mustString(policy))
	}
}
func (w *Window) SetMovePolicy(policy *WindowMovePolicy) {
	if policy == nil {
		w.Action("set-move-policy", "")
	} else {
		w.Action("set-move-policy", mustString(policy))
	}
}
func (w *Window) SetTabbingIdentifier(value string) { w.Action("set-tabbing-identifier", value) }
func (w *Window) SelectNextTab()                    { w.Action("select-next-tab", "") }
func (w *Window) SelectPreviousTab()                { w.Action("select-previous-tab", "") }
func (w *Window) SelectTab(index uint32) {
	w.Action("select-tab", strconv.FormatUint(uint64(index), 10))
}
func (w *Window) MergeAllWindows()    { w.Action("merge-all-windows", "") }
func (w *Window) MoveTabToNewWindow() { w.Action("move-tab-to-new-window", "") }
func (w *Window) ToggleTabBar()       { w.Action("toggle-tab-bar", "") }
func (w *Window) ToggleTabOverview()  { w.Action("toggle-tab-overview", "") }

func (w *Window) ImageAction(action string, image *ImageSource, description string) {
	if w.Closed {
		return
	}
	SendMutation(mustString(map[string]any{"method": "window-image-action", "window": w.NativeID, "action": action, "image": image, "description": description}))
}

func (w *Window) GetRestoreState(done func(WindowRestoreState, error)) {
	commandJSON(map[string]any{"method": "get-window-restore-state", "window": w.NativeID}, done)
}

func (w *Window) PopupMenu(items []MenuItem, position *Point, done func(error)) {
	if position == nil {
		PopupMenu(w, items, nil, nil, done)
	} else {
		PopupMenu(w, items, &position.X, &position.Y, done)
	}
}
