package cart

import (
	"os"
	"path/filepath"
	"runtime"
	"strings"
	"testing"

	"github.com/egoist/quickgui/go/native"
	"github.com/egoist/quickgui/go/reactive"
	"github.com/egoist/quickgui/go/ui"
)

func TestCompiledLocationsPointToAuthoredGo(t *testing.T) {
	_, here, _, _ := runtime.Caller(0)
	original := filepath.Join(filepath.Dir(here), "locations.go")
	bytes, err := os.ReadFile("locations.go")
	if err != nil {
		t.Fatal(err)
	}
	lines := map[string]int{}
	for index, line := range strings.Split(string(bytes), "\n") {
		for _, marker := range []string{"setup location", "binding location", "panic location"} {
			if strings.Contains(line, marker) {
				lines[marker] = index + 1
			}
		}
	}
	native.ResetTreeStateForTests()
	reactive.CreateRoot(func(dispose func()) struct{} {
		defer dispose()
		value, setValue := ui.CreateSignal(0)
		seen := []int{}
		mapped := MappedLabel(value(), func(file string, line int) {
			if file != original {
				t.Errorf("mapped file %q, want %q", file, original)
			}
			seen = append(seen, line)
		})
		if len(seen) != 2 || seen[0] != lines["setup location"] || seen[1] != lines["binding location"] {
			t.Fatalf("wrong mapped lines: %v, want %v", seen, lines)
		}
		panicking := PanicLabel(value())
		if mapped == nil || panicking == nil {
			t.Fatal("expected returned component nodes")
		}
		func() {
			defer func() {
				if recover() == nil {
					t.Error("expected reactive panic")
					return
				}
				pcs := make([]uintptr, 32)
				frames := runtime.CallersFrames(pcs[:runtime.Callers(0, pcs)])
				for {
					frame, more := frames.Next()
					if strings.Contains(frame.Function, "PanicLabel") && frame.File == original && frame.Line == lines["panic location"] {
						return
					}
					if !more {
						break
					}
				}
				t.Error("panic stack did not map to its original source line")
			}()
			setValue(1)
		}()
		return struct{}{}
	})
}

func TestGenericAndVariadicProps(t *testing.T) {
	native.ResetTreeStateForTests()
	reactive.CreateRoot(func(dispose func()) struct{} {
		defer dispose()
		value, setValue := ui.CreateSignal(1)
		a, b := GenericLabel(value()), GenericLabel[int](value())
		c := VariadicLabel(value(), "child")
		children := []any{"spread"}
		d := VariadicLabel(value(), children...)
		setValue(2)
		if a.Node.Children[0].Text != "2" || b.Node.Children[0].Text != "2" || c.Node.Children[0].Children[0].Text != "2" || d.Node.Children[0].Children[0].Text != "2" {
			t.Fatal("generic or variadic props stopped tracking")
		}
		return struct{}{}
	})
}
