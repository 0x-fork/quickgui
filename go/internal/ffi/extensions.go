package ffi

import (
	"fmt"

	"github.com/ebitengine/purego"
)

// LoadExtension keeps the image loaded for process lifetime: its session workers
// and the Rust core retain function pointers. Sessions release their own resources.
func (library *Library) LoadExtension(name, path string) (err error) {
	handle, err := openLibrary(path)
	if err != nil {
		return fmt.Errorf("load QuickGUI extension %s: %w", name, err)
	}
	defer func() {
		if failure := recover(); failure != nil {
			err = fmt.Errorf("incompatible QuickGUI extension %s: %v", name, failure)
		}
	}()
	var descriptor func() uintptr
	purego.RegisterLibFunc(&descriptor, handle, "quickgui_extension_v1")
	value := descriptor()
	if value == 0 {
		return fmt.Errorf("QuickGUI extension %s returned no descriptor", name)
	}
	bytes := []byte(name)
	if library.RegisterExtension(value, bytes, uintptr(len(bytes))) != 0 {
		return fmt.Errorf("QuickGUI extension %s does not match the core ABI or release version", name)
	}
	return nil
}
