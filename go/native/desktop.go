package native

type desktopAPI struct{}

var Desktop desktopAPI

func (desktopAPI) GetSupport(done func(DesktopIntegrationSupport, error)) {
	commandJSON(map[string]any{"method": "get-desktop-integration-support"}, done)
}

func (desktopAPI) SetDockBadge(value string) {
	SendMutation(mustString(map[string]any{"method": "set-dock-badge", "value": value}))
}

func (desktopAPI) SetDockIcon(icon *ImageSource) {
	SendMutation(mustString(map[string]any{"method": "set-dock-icon", "icon": icon}))
}

var dockMenuIDs []uint32

func (desktopAPI) SetDockMenu(definition *MenuDefinition) {
	ids := []uint32{}
	var menu *string
	if definition != nil {
		encoded := mustString([]nativeMenuDefinition{{Label: definition.Label, Enabled: enabledOrTrue(definition.Enabled), Items: encodeMenuItems(definition.Items, &ids)}})
		menu = &encoded
	}
	SendMutation(mustString(map[string]any{"method": "set-dock-menu", "menu": menu}))
	releaseMenuIDs(dockMenuIDs)
	dockMenuIDs = ids
}

func (desktopAPI) AddRecentDocument(path string) {
	SendMutation(mustString(map[string]any{"method": "add-recent-document", "path": path}))
}

func (desktopAPI) ClearRecentDocuments() {
	SendMutation(`{"method":"clear-recent-documents"}`)
}

func (desktopAPI) ShowAboutPanel(options AboutPanelOptions) {
	SendMutation(mustString(map[string]any{"method": "show-about-panel", "options": options}))
}

func (desktopAPI) GetFileIcon(path, size string, done func(NativeImage, error)) {
	if size == "" {
		size = "normal"
	}
	commandJSON(map[string]any{"method": "file-icon", "path": path, "size": size}, done)
}

func (desktopAPI) SetUserTasks(tasks []UserTask, done func(error)) {
	if tasks == nil {
		tasks = []UserTask{}
	}
	commandVoid(map[string]any{"method": "set-user-tasks", "tasks": tasks}, done)
}

func (desktopAPI) GetWindowRegistry(done func(WindowRegistry, error)) {
	commandJSON(map[string]any{"method": "get-window-registry"}, done)
}

type screenAPI struct{}

var Screen screenAPI
var screenListeners subscriptions[[]Display]

func (screenAPI) GetAllDisplays(done func([]Display, error)) {
	commandJSON(map[string]any{"method": "get-displays"}, done)
}

func (screenAPI) GetPrimaryDisplay(done func(*Display, error)) {
	Screen.GetAllDisplays(func(displays []Display, err error) {
		if done == nil {
			return
		}
		for _, display := range displays {
			if display.Primary {
				done(&display, err)
				return
			}
		}
		if len(displays) != 0 {
			done(&displays[0], err)
		} else {
			done(nil, err)
		}
	})
}

func (screenAPI) GetCursorScreenPoint(done func(Point, error)) {
	commandJSON(map[string]any{"method": "get-cursor-screen-position"}, done)
}

func (screenAPI) OnChange(listener func([]Display)) func() {
	return screenListeners.add(listener)
}

type preferencesAPI struct{}

var SystemPreferences preferencesAPI
var preferencesListeners subscriptions[SystemPreferencesSnapshot]

func (preferencesAPI) GetCurrent(done func(SystemPreferencesSnapshot, error)) {
	commandJSON(map[string]any{"method": "get-system-preferences"}, done)
}

func (preferencesAPI) OnChange(listener func(SystemPreferencesSnapshot)) func() {
	return preferencesListeners.add(listener)
}

type appearanceAPI struct{}

var Appearance appearanceAPI

func (appearanceAPI) GetColorScheme(done func(string, error)) {
	SystemPreferences.GetCurrent(func(preferences SystemPreferencesSnapshot, err error) {
		if done != nil {
			done(preferences.ColorScheme, err)
		}
	})
}

type keyboardAPI struct{}

var Keyboard keyboardAPI

func (keyboardAPI) GetLayout(done func(KeyboardLayout, error)) {
	commandJSON(map[string]any{"method": "get-keyboard-layout"}, done)
}

type shellAPI struct{}

var Shell shellAPI

func (shellAPI) OpenExternal(url string, done func(error))      { OpenExternal(url, done) }
func (shellAPI) OpenPath(path string, done func(error))         { OpenPath(path, done) }
func (shellAPI) ShowItemInFolder(path string, done func(error)) { ShowItemInFolder(path, done) }
func (shellAPI) TrashItem(path string, done func(error))        { TrashItem(path, done) }
func (shellAPI) Beep()                                          { SendMutation(`{"method":"app-mutation","action":"beep"}`) }

type spellCheckerAPI struct{}

var SpellChecker spellCheckerAPI

func (spellCheckerAPI) LearnWord(word string) {
	SendMutation(mustString(map[string]any{"method": "app-mutation", "action": "learn-word", "value": word}))
}

func (spellCheckerAPI) IgnoreWord(word string) {
	SendMutation(mustString(map[string]any{"method": "app-mutation", "action": "ignore-word", "value": word}))
}
