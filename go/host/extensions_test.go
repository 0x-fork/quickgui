package host

import (
	"fmt"
	"os"
	"path/filepath"
	"strings"
	"testing"
)

func TestExtensionRequirementsTrackIndependentVersions(t *testing.T) {
	originalNames, originalLoaded := extensionState.names, extensionState.loaded
	extensionState.names, extensionState.loaded = make(map[string]string), false
	t.Cleanup(func() { extensionState.names, extensionState.loaded = originalNames, originalLoaded })

	RequireExtension("acme-echo", "7.2.1+native.3")
	RequireExtension("acme-echo", "7.2.1+native.3")
	RequireExtension("terminal")
	assertPanics := func(f func()) {
		t.Helper()
		defer func() {
			if recover() == nil {
				t.Error("invalid requirement was accepted")
			}
		}()
		f()
	}
	assertPanics(func() { RequireExtension("acme-echo", "7.2.2") })
	assertPanics(func() { RequireExtension("acme-echo") })
	for _, name := range []string{"", "../provider", "provider/command", "host", strings.Repeat("a", 65)} {
		assertPanics(func() { RequireExtension(name, "1.0.0") })
	}
	for _, version := range []string{"", "latest", "^1.0.0", "1.0.0/../../other", "1.0.0+" + strings.Repeat("x", 64)} {
		assertPanics(func() { RequireExtension("invalid-version", version) })
	}
	assertPanics(func() { RequireExtension("many-versions", "1.0.0", "2.0.0") })
	if got := requiredExtensions(); len(got) != 2 || got[0] != (extensionRequirement{"acme-echo", "7.2.1+native.3"}) || got[1] != (extensionRequirement{"terminal", ""}) {
		t.Fatalf("unexpected requirements: %v", got)
	}
	assertPanics(func() { RequireExtension("too-late", "1.0.0") })
	RequireExtension("acme-echo", "7.2.1+native.3")

	extensionState.loaded = false
	for i := 2; i < 32; i++ {
		RequireExtension(fmt.Sprintf("provider-%d", i), "1.0.0")
	}
	assertPanics(func() { RequireExtension("overflow", "1.0.0") })
}

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
