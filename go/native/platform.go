package native

import (
	"encoding/json"
	"fmt"
	"os"
	"path/filepath"
	"runtime"
	"strconv"

	"github.com/egoist/quickgui/go/host"
	"github.com/egoist/quickgui/go/reactive"
)

// AppPaths are the host's well-known directories.
type AppPaths struct {
	Executable    string `json:"executable"`
	ExecutableDir string `json:"executableDir"`
	ResourceDir   string `json:"resourceDir"`
	HomeDir       string `json:"homeDir,omitempty"`
	ConfigDir     string `json:"configDir,omitempty"`
	DataDir       string `json:"dataDir,omitempty"`
	LocalDataDir  string `json:"localDataDir,omitempty"`
	CacheDir      string `json:"cacheDir,omitempty"`
	TempDir       string `json:"tempDir"`
}

// GetPaths asks the host for well-known directories. done runs on the application goroutine.
func (a *Application) GetPaths(done func(*AppPaths, error)) {
	SendCommand(`{"method":"get-app-paths"}`, func(raw string, err error) {
		if err != nil {
			done(nil, err)
			return
		}
		if isNullJSON(raw) {
			done(nil, nil)
			return
		}
		var paths AppPaths
		if err := json.Unmarshal([]byte(raw), &paths); err != nil {
			done(nil, err)
			return
		}
		done(&paths, nil)
	})
}

// GetWindowState asks the host for one window snapshot. done runs on the application goroutine.
func (w *Window) GetState(done func(WindowState, error)) {
	payload, _ := json.Marshal(map[string]any{"method": "get-window-state", "window": w.NativeID})
	SendCommand(string(payload), func(raw string, err error) {
		if err != nil {
			done(WindowState{}, err)
			return
		}
		var state WindowState
		if err := json.Unmarshal([]byte(raw), &state); err != nil {
			done(WindowState{}, err)
			return
		}
		done(state, nil)
	})
}

// WindowState is a host snapshot of one window.
type WindowState struct {
	X              float64 `json:"x"`
	Y              float64 `json:"y"`
	Width          float64 `json:"width"`
	Height         float64 `json:"height"`
	ViewportWidth  float64 `json:"viewportWidth"`
	ViewportHeight float64 `json:"viewportHeight"`
	Minimized      bool    `json:"minimized"`
	Maximized      bool    `json:"maximized"`
	Fullscreen     bool    `json:"fullscreen"`
	Appearance     string  `json:"appearance"`
	Focused        bool    `json:"focused"`
	Visible        bool    `json:"visible"`
}

// AlertDialogOptions configure a native alert sheet.
type AlertDialogOptions struct {
	Message string
	Detail  string
	Level   string
	Buttons []AlertDialogButton
	Window  *Window
}

// AlertDialogButton is one alert button.
type AlertDialogButton struct {
	Label string `json:"label"`
	Role  string `json:"role,omitempty"`
}

// ShowAlertDialog presents a native alert. done receives the zero-based button index.
func ShowAlertDialog(options AlertDialogOptions, done func(int, error)) {
	buttons := options.Buttons
	if len(buttons) == 0 {
		buttons = []AlertDialogButton{{Label: "OK", Role: "default"}}
	}
	native := map[string]any{"message": options.Message, "buttons": buttons}
	if options.Level != "" {
		native["level"] = options.Level
	}
	if options.Detail != "" {
		native["detail"] = options.Detail
	}
	encoded, _ := json.Marshal(native)
	showDialog(options.Window, 0, string(encoded), func(value string, _ []string, err error) {
		if err != nil {
			done(0, err)
			return
		}
		index, _ := strconv.Atoi(value)
		done(index, nil)
	})
}

// FileDialogFilter is one named set of extensions for a file panel.
type FileDialogFilter struct {
	Name       string   `json:"name"`
	Extensions []string `json:"extensions"`
}

// OpenDialogOptions configure a native open panel.
type OpenDialogOptions struct {
	Title       string
	DefaultPath string
	ButtonLabel string
	Filters     []FileDialogFilter
	Properties  []string
	Window      *Window
}

// OpenDialogResult is the Electron-shaped open-panel answer.
type OpenDialogResult struct {
	Canceled  bool
	FilePaths []string
}

type SaveDialogOptions struct {
	Title       string
	DefaultPath string
	ButtonLabel string
	Filters     []FileDialogFilter
	Window      *Window
}

type SaveDialogResult struct {
	Canceled bool
	FilePath string
}

// ShowSaveDialog asks for a destination without writing a file.
func ShowSaveDialog(options SaveDialogOptions, done func(SaveDialogResult, error)) {
	native := encodeOpenDialogOptions(OpenDialogOptions{
		Title: options.Title, DefaultPath: options.DefaultPath,
		ButtonLabel: options.ButtonLabel, Filters: options.Filters,
	})
	if native.Directory == "" {
		native.Directory = "."
	}
	encoded, err := json.Marshal(native)
	if err != nil {
		done(SaveDialogResult{}, err)
		return
	}
	showDialog(options.Window, 2, string(encoded), func(value string, paths []string, err error) {
		if len(paths) > 0 {
			value = paths[0]
		}
		done(SaveDialogResult{Canceled: value == "", FilePath: value}, err)
	})
}

type nativeOpenDialogOptions struct {
	Files            bool               `json:"files"`
	Directories      bool               `json:"directories"`
	Multiple         bool               `json:"multiple"`
	Title            string             `json:"title,omitempty"`
	Prompt           string             `json:"prompt,omitempty"`
	Directory        string             `json:"directory,omitempty"`
	SuggestedName    string             `json:"suggestedName,omitempty"`
	Filters          []FileDialogFilter `json:"filters"`
	ShowsHiddenFiles bool               `json:"showsHiddenFiles"`
}

func encodeOpenDialogOptions(options OpenDialogOptions) nativeOpenDialogOptions {
	files := contains(options.Properties, "openFile")
	directories := contains(options.Properties, "openDirectory")
	if !files && !directories {
		directories = true
	}
	filters := options.Filters
	if filters == nil {
		filters = []FileDialogFilter{}
	}
	native := nativeOpenDialogOptions{
		Files:            files,
		Directories:      directories,
		Multiple:         contains(options.Properties, "multiSelections"),
		Title:            options.Title,
		Prompt:           options.ButtonLabel,
		Filters:          filters,
		ShowsHiddenFiles: contains(options.Properties, "showHiddenFiles"),
	}
	if options.DefaultPath != "" {
		directory, suggested := splitDefaultPath(options.DefaultPath)
		native.Directory = directory
		native.SuggestedName = suggested
	}
	return native
}

func splitDefaultPath(defaultPath string) (directory, suggestedName string) {
	if pathEndsWithSeparator(defaultPath) {
		return defaultPath, ""
	}
	info, err := os.Stat(defaultPath)
	if err == nil && info.IsDir() {
		return defaultPath, ""
	}
	return filepath.Dir(defaultPath), filepath.Base(defaultPath)
}

func pathEndsWithSeparator(path string) bool {
	if path == "" {
		return false
	}
	last := path[len(path)-1]
	return last == '/' || last == filepath.Separator
}

// ShowOpenDialog presents a native open panel.
func ShowOpenDialog(options OpenDialogOptions, done func(OpenDialogResult, error)) {
	encoded, err := json.Marshal(encodeOpenDialogOptions(options))
	if err != nil {
		done(OpenDialogResult{}, err)
		return
	}
	showDialog(options.Window, 1, string(encoded), func(_ string, paths []string, err error) {
		if err != nil {
			done(OpenDialogResult{}, err)
			return
		}
		if len(paths) == 0 {
			done(OpenDialogResult{Canceled: true}, nil)
			return
		}
		done(OpenDialogResult{FilePaths: paths}, nil)
	})
}

func showDialog(window *Window, kind uint32, options string, complete func(value string, paths []string, err error)) {
	assertAppReady()
	request := allocateRequest()
	pendingDialogs[request] = dialogReply{complete: complete}
	windowID := uint32(0)
	if window != nil && !window.Closed {
		windowID = window.NativeID
	}
	host.Current.ShowDialog(appID, windowID, request, kind, options)
}

func contains(values []string, want string) bool {
	for _, value := range values {
		if value == want {
			return true
		}
	}
	return false
}

// MenuItem is one native menu entry.
type MenuItem struct {
	Type        string
	Label       string
	Enabled     *bool
	Checked     bool
	Role        string
	Accelerator string
	Items       []MenuItem
	Click       func()
}

// MenuDefinition is one top-level application menu.
type MenuDefinition struct {
	Label   string
	Enabled *bool
	Items   []MenuItem
}

type nativeMenuItem struct {
	Type        string            `json:"type"`
	ID          uint32            `json:"id,omitempty"`
	Label       string            `json:"label,omitempty"`
	Enabled     bool              `json:"enabled"`
	Checked     bool              `json:"checked,omitempty"`
	Role        string            `json:"role,omitempty"`
	Accelerator string            `json:"accelerator,omitempty"`
	Items       *[]nativeMenuItem `json:"items,omitempty"`
}

type nativeMenuDefinition struct {
	Label   string           `json:"label"`
	Enabled bool             `json:"enabled"`
	Items   []nativeMenuItem `json:"items"`
}

type menuCallback struct {
	click  func()
	window *Window
	owner  *reactive.Owner
}

var (
	menuCallbacks          = map[uint32]menuCallback{}
	nextMenuAction  uint32 = 1
	applicationMenu []uint32
)

func enabledOrTrue(value *bool) bool {
	if value == nil {
		return true
	}
	return *value
}

func encodeMenuItems(items []MenuItem, ids *[]uint32) []nativeMenuItem {
	native := make([]nativeMenuItem, 0, len(items))
	for _, item := range items {
		kind := item.Type
		if kind == "" {
			if len(item.Items) > 0 {
				kind = "submenu"
			} else if item.Role != "" && item.Click == nil {
				kind = "role"
			} else {
				kind = "action"
			}
		}
		if kind == "separator" {
			native = append(native, nativeMenuItem{Type: "separator"})
			continue
		}
		if kind == "submenu" {
			// The host requires `items` on every submenu, including an empty Open Recent list.
			children := encodeMenuItems(item.Items, ids)
			native = append(native, nativeMenuItem{
				Type: "submenu", Label: item.Label, Enabled: enabledOrTrue(item.Enabled),
				Items: &children,
			})
			continue
		}
		entry := nativeMenuItem{
			Type: kind, Label: item.Label, Enabled: enabledOrTrue(item.Enabled),
			Checked: item.Checked, Role: item.Role, Accelerator: item.Accelerator,
		}
		if kind == "role" {
			entry.Type = "role"
		} else {
			entry.Type = "action"
			id := nextMenuAction
			nextMenuAction++
			if nextMenuAction >= 0xffff_fff0 {
				nextMenuAction = 1
			}
			click := item.Click
			if click == nil {
				click = func() {}
			}
			menuCallbacks[id] = menuCallback{click: click}
			*ids = append(*ids, id)
			entry.ID = id
		}
		native = append(native, entry)
	}
	return native
}

func releaseMenuIDs(ids []uint32) {
	for _, id := range ids {
		delete(menuCallbacks, id)
	}
}

func dispatchMenuAction(id uint32) {
	if callback, ok := menuCallbacks[id]; ok {
		if callback.window != nil && callback.window.Closed || callback.owner != nil && callback.owner.Disposed {
			return
		}
		withCurrentWindow(callback.window, func() {
			reactive.RunWithOwner(callback.owner, func() struct{} {
				reactive.Batch(callback.click)
				return struct{}{}
			})
		})
	}
}

// SetApplicationMenu replaces the application-wide native menu declaration.
func SetApplicationMenu(definitions []MenuDefinition) {
	ids := []uint32{}
	native := make([]nativeMenuDefinition, 0, len(definitions))
	for _, definition := range definitions {
		native = append(native, nativeMenuDefinition{
			Label: definition.Label, Enabled: enabledOrTrue(definition.Enabled),
			Items: encodeMenuItems(definition.Items, &ids),
		})
	}
	payload, _ := json.Marshal(map[string]any{"method": "set-application-menu", "menu": mustString(native)})
	SendMutation(string(payload))
	releaseMenuIDs(applicationMenu)
	applicationMenu = ids
}

// PopupMenu shows a native context menu. done runs when it closes.
func PopupMenu(window *Window, items []MenuItem, x, y *float64, done func(error)) {
	if window == nil || window.Closed {
		if done != nil {
			done(fmt.Errorf("a native popup menu requires an open Window"))
		}
		return
	}
	ids := []uint32{}
	menu := []nativeMenuDefinition{{Label: "Context", Enabled: true, Items: encodeMenuItems(items, &ids)}}
	for _, id := range ids {
		callback := menuCallbacks[id]
		callback.window = window
		callback.owner = reactive.GetOwner()
		menuCallbacks[id] = callback
	}
	body := map[string]any{
		"method": "window-popup-menu",
		"window": window.NativeID,
		"menu":   mustString(menu),
	}
	if x != nil && y != nil {
		body["x"] = *x
		body["y"] = *y
	}
	encoded, _ := json.Marshal(body)
	window.Flush()
	SendCommand(string(encoded), func(_ string, err error) {
		releaseMenuIDs(ids)
		if done != nil {
			done(err)
		}
	})
}

func mustString(value any) string {
	encoded, err := json.Marshal(value)
	if err != nil {
		panic(err)
	}
	return string(encoded)
}

// WriteClipboardText writes one text item to the system clipboard.
func WriteClipboardText(text string) {
	item := map[string]any{"entries": []map[string]any{{"kind": "text", "text": text}}}
	payload, _ := json.Marshal(map[string]any{"method": "write-clipboard", "item": item})
	SendCommand(string(payload), func(string, error) {})
}

func shellAction(action, value string, done func(error)) {
	payload, _ := json.Marshal(map[string]any{
		"method": "shell-action", "action": action, "value": value,
	})
	SendCommand(string(payload), func(_ string, err error) {
		if done != nil {
			done(err)
		}
	})
}

// OpenExternal opens a URL with the platform handler.
func OpenExternal(url string, done func(error)) { shellAction("open-external", url, done) }

// OpenPath opens a file or folder with the platform handler.
func OpenPath(path string, done func(error)) { shellAction("open-path", path, done) }

// ShowItemInFolder reveals a path in the file manager.
func ShowItemInFolder(path string, done func(error)) { shellAction("reveal-path", path, done) }

// TrashItem moves a path to the Trash.
func TrashItem(path string, done func(error)) { shellAction("trash-path", path, done) }

// FallbackDataDir is used when the host has not reported a data directory.
func FallbackDataDir(appName string) string {
	if home, err := os.UserHomeDir(); err == nil && home != "" {
		switch runtime.GOOS {
		case "darwin":
			return filepath.Join(home, "Library", "Application Support", appName)
		case "windows":
			if appdata := os.Getenv("APPDATA"); appdata != "" {
				return filepath.Join(appdata, appName)
			}
		default:
			if xdg := os.Getenv("XDG_DATA_HOME"); xdg != "" {
				return filepath.Join(xdg, appName)
			}
			return filepath.Join(home, ".local", "share", appName)
		}
	}
	return filepath.Join(os.TempDir(), appName)
}
