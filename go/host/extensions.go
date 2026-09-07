package host

import (
	"fmt"
	"os"
	"path/filepath"
	"regexp"
	"runtime"
	"sort"
	"strings"
	"sync"
)

var extensionNames = regexp.MustCompile(`^[a-z][a-z0-9-]{0,63}$`)
var extensionState = struct {
	sync.Mutex
	names  map[string]bool
	loaded bool
}{names: make(map[string]bool)}

// RequireExtension declares an optional backend from a package's init function.
// Registration only records metadata; no native code or UI work runs during init.
func RequireExtension(name string) {
	extensionState.Lock()
	defer extensionState.Unlock()
	if extensionState.names[name] {
		return
	}
	if !extensionNames.MatchString(name) || len(extensionState.names) >= 32 {
		panic("QuickGUI extension name is invalid or the extension limit was exceeded")
	}
	if extensionState.loaded {
		panic("QuickGUI extensions must be declared before native.Run")
	}
	extensionState.names[name] = true
}

func requiredExtensions() []string {
	extensionState.Lock()
	defer extensionState.Unlock()
	extensionState.loaded = true
	names := make([]string, 0, len(extensionState.names))
	for name := range extensionState.names {
		names = append(names, name)
	}
	sort.Strings(names)
	return names
}

func extensionLibraryName(name string) string {
	name = strings.ReplaceAll(name, "-", "_")
	switch runtime.GOOS {
	case "darwin":
		return "libquickgui_" + name + ".dylib"
	case "windows":
		return "quickgui_" + name + ".dll"
	default:
		return "libquickgui_" + name + ".so"
	}
}

func findExtensionLibrary(name, corePath string) (string, error) {
	file := extensionLibraryName(name)
	if directory := os.Getenv("QUICKGUI_EXTENSION_DIR"); directory != "" {
		path := filepath.Join(directory, file)
		if fileExists(path) {
			return filepath.Abs(path)
		}
		return "", fmt.Errorf("QuickGUI extension %s is missing from QUICKGUI_EXTENSION_DIR", name)
	}
	directory := filepath.Dir(corePath)
	if path := filepath.Join(directory, file); fileExists(path) {
		return path, nil
	}
	// Source checkouts stage independently built extensions in their own packages.
	for i := 0; i < 8; i++ {
		path := filepath.Join(directory, "packages", "native-"+name, "lib", StageTarget(), file)
		if fileExists(path) {
			return path, nil
		}
		parent := filepath.Dir(directory)
		if parent == directory {
			break
		}
		directory = parent
	}
	return "", fmt.Errorf("QuickGUI extension %s is not bundled; rebuild with quickgui dev/build (from source: bun packages/native/build.ts --extension %s)", name, name)
}
