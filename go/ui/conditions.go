package ui

import (
	"fmt"

	"github.com/egoist/quickgui/go/reactive"
)

func booleanRead(value any) func() bool {
	switch read := value.(type) {
	case bool:
		return func() bool { return read }
	case func() bool:
		return read
	case reactive.Accessor[bool]:
		return read
	default:
		panic(fmt.Sprintf("QuickGUI condition must be a bool or bool accessor, got %T", value))
	}
}
