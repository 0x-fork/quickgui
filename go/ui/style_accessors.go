package ui

import (
	"github.com/egoist/quickgui/go/native"
	"github.com/egoist/quickgui/go/reactive"
)

// Bind scalar styles independently so a resize or theme update does not rebuild
// children or subscribe unrelated properties to the same signal.
func bindStyleAccessor(node *native.Node, code uint16, value any, write func(*native.Node, uint16, any)) bool {
	read, ok := styleAccessor(value)
	if !ok {
		return false
	}
	node.Bind(func() { write(node, code, read()) })
	return true
}

func styleAccessor(value any) (func() any, bool) {
	var read func() any
	switch accessor := value.(type) {
	case func() any:
		read = accessor
	case reactive.Accessor[any]:
		read = accessor
	case func() string:
		read = func() any { return accessor() }
	case reactive.Accessor[string]:
		read = func() any { return accessor() }
	case func() float64:
		read = func() any { return accessor() }
	case reactive.Accessor[float64]:
		read = func() any { return accessor() }
	case func() float32:
		read = func() any { return accessor() }
	case reactive.Accessor[float32]:
		read = func() any { return accessor() }
	case func() int:
		read = func() any { return accessor() }
	case reactive.Accessor[int]:
		read = func() any { return accessor() }
	case func() int64:
		read = func() any { return accessor() }
	case reactive.Accessor[int64]:
		read = func() any { return accessor() }
	case func() *float64:
		read = func() any { return accessor() }
	case func() *int:
		read = func() any { return accessor() }
	default:
		return nil, false
	}
	return read, true
}

func styleValue(value any) any {
	if read, ok := styleAccessor(value); ok {
		return read()
	}
	return value
}
