package ffi

import (
	"os"
	"testing"

	"github.com/egoist/quickgui/go/protocol"
)

// This headless smoke is also run against real staged images by check-extensions.ts.
func TestExtensionLibrarySmoke(t *testing.T) {
	core := os.Getenv("QUICKGUI_TEST_CORE")
	terminal := os.Getenv("QUICKGUI_TEST_TERMINAL")
	if core == "" || terminal == "" {
		t.Skip("native extension images not supplied")
	}
	library, err := Load(core, protocol.Version)
	if err != nil {
		t.Fatal(err)
	}
	if err := library.LoadExtension("wrong-package", terminal); err == nil {
		t.Fatal("registration must reject a descriptor for a different package")
	}
	if err := library.LoadExtension("terminal", terminal); err != nil {
		t.Fatal(err)
	}
	if err := library.LoadExtension("terminal", terminal); err != nil {
		t.Fatal(err)
	}
}
