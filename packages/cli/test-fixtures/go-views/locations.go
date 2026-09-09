package cart

import (
	"runtime"

	"github.com/egoist/quickgui/go/ui"
)

func MappedLabel(value int, location func(string, int)) {
	_, file, line, _ := runtime.Caller(0) // setup location
	location(file, line)
	ui.Text(
		value,
		func() string {
			_, file, line, _ := runtime.Caller(0) // binding location
			location(file, line)
			return "mapped"
		},
	)
}

func PanicLabel(value int) {
	ui.Text(func() string {
		if value > 0 {
			panic("mapped prop panic") // panic location
		}
		return "ready"
	})
}

func GenericLabel[T ~int](value T) {
	ui.Text(int(value))
}

func VariadicLabel(value int, children ...any) {
	ui.View(ui.Text(value), children)
}
