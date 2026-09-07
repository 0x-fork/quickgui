package native

import (
	"os"
	"path/filepath"
	"testing"
)

func TestApplicationResourcesDoNotDependOnLaunchDirectory(t *testing.T) {
	root := t.TempDir()
	bundle := filepath.Join(root, "Example.app", "Contents")
	resources := filepath.Join(bundle, "Resources")
	if err := os.MkdirAll(resources, 0700); err != nil {
		t.Fatal(err)
	}
	for _, tc := range []struct{ executable, want string }{
		{filepath.Join(bundle, "MacOS", "Example"), resources},
		{filepath.Join(root, "portable", "example"), filepath.Join(root, "portable")},
		{filepath.Join(root, "portable", "example.exe"), filepath.Join(root, "portable")},
	} {
		if got := applicationResourceDirectory(tc.executable); got != tc.want {
			t.Errorf("%s: got %s want %s", tc.executable, got, tc.want)
		}
	}
}
