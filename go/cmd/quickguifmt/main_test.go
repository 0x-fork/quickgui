package main

import (
	"bytes"
	"os"
	"path/filepath"
	"strings"
	"testing"
)

func TestCheckWriteAndDirectoryExclusions(t *testing.T) {
	root := t.TempDir()
	path := filepath.Join(root, "main.go")
	source := []byte("package main\nfunc main( ){}")
	if err := os.WriteFile(path, source, 0o600); err != nil {
		t.Fatal(err)
	}
	for _, name := range []string{".quickgui", "node_modules", "vendor"} {
		dir := filepath.Join(root, name)
		if err := os.Mkdir(dir, 0o755); err != nil {
			t.Fatal(err)
		}
		if err := os.WriteFile(filepath.Join(dir, "ignored.go"), []byte("invalid Go"), 0o600); err != nil {
			t.Fatal(err)
		}
	}
	var output, errors bytes.Buffer
	if status := run([]string{"-check", root, path}, nil, &output, &errors); status != 1 {
		t.Fatalf("check status %d: %s", status, &errors)
	}
	if output.String() != path+"\n" {
		t.Fatalf("expected one changed file, got %q", &output)
	}
	unchanged, _ := os.ReadFile(path)
	if !bytes.Equal(source, unchanged) {
		t.Fatal("check modified the file")
	}
	if status := run([]string{"-w", root}, nil, &output, &errors); status != 0 {
		t.Fatalf("write status %d: %s", status, &errors)
	}
	if status := run([]string{"-check", root}, nil, &output, &errors); status != 0 {
		t.Fatalf("formatted tree still fails: %s", &errors)
	}
	info, _ := os.Stat(path)
	if info.Mode().Perm() != 0o600 {
		t.Fatal("formatting changed permissions")
	}
}

func TestInvalidFilePreventsPartialWrites(t *testing.T) {
	root := t.TempDir()
	first := filepath.Join(root, "a.go")
	source := []byte("package main\nfunc main( ){}")
	if err := os.WriteFile(first, source, 0o600); err != nil {
		t.Fatal(err)
	}
	if err := os.WriteFile(filepath.Join(root, "z.go"), []byte("invalid Go"), 0o600); err != nil {
		t.Fatal(err)
	}
	var output, errors bytes.Buffer
	if status := run([]string{"-w", root}, nil, &output, &errors); status != 2 {
		t.Fatalf("syntax error status %d: %s", status, &errors)
	}
	got, _ := os.ReadFile(first)
	if !bytes.Equal(got, source) {
		t.Fatal("a syntax error caused a partial write")
	}
}

func TestStdinAndInvalidFlags(t *testing.T) {
	var output, errors bytes.Buffer
	if status := run(nil, strings.NewReader("package main\nfunc main( ){}"), &output, &errors); status != 0 {
		t.Fatalf("stdin status %d: %s", status, &errors)
	}
	if output.String() != "package main\n\nfunc main() {}\n" {
		t.Fatal(output.String())
	}
	for _, args := range [][]string{{"-w"}, {"-check"}, {"-width=0"}, {"-w", "-check", "."}} {
		if status := run(args, nil, &output, &errors); status != 2 {
			t.Fatalf("invalid flags %v returned %d", args, status)
		}
	}
}
