package host

import (
	"os"
	"path/filepath"
	"strings"
	"testing"
)

func TestExtensionLibraryLookup(t *testing.T) {
	directory := t.TempDir()
	core := filepath.Join(directory, LibraryName())
	terminal := filepath.Join(directory, extensionLibraryName("terminal"))
	t.Setenv("QUICKGUI_EXTENSION_DIR", "")
	if _, err := findExtensionLibrary("terminal", core); err == nil {
		t.Fatal("an absent backend must produce a useful error")
	}
	if err := os.WriteFile(terminal, []byte("library"), 0644); err != nil {
		t.Fatal(err)
	}
	path, err := findExtensionLibrary("terminal", core)
	if err != nil || path != terminal {
		t.Fatalf("%s: %v", path, err)
	}
	t.Setenv("QUICKGUI_EXTENSION_DIR", t.TempDir())
	if _, err := findExtensionLibrary("terminal", core); err == nil {
		t.Fatal("an explicit directory must not silently fall back to a different library")
	}
	if strings.Contains(extensionLibraryName("future-widget"), "future-widget") {
		t.Fatal("library names must use underscores, matching CLI manifests")
	}
}
