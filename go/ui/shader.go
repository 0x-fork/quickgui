package ui

import (
	"fmt"
	"math"

	"github.com/egoist/quickgui/go/native"
	"github.com/egoist/quickgui/go/protocol"
	"github.com/egoist/quickgui/go/reactive"
)

func bindShaderParameters(node *native.Node, value any) {
	var read func() []float64
	switch value := value.(type) {
	case []float64:
		read = func() []float64 { return value }
	case func() []float64:
		read = value
	case reactive.Accessor[[]float64]:
		read = value
	default:
		panic(fmt.Sprintf("QuickGUI shader parameters must be []float64 or an accessor, got %T", value))
	}
	node.Bind(func() {
		values := read()
		if len(values) > 16 {
			panic("QuickGUI exposes sixteen shader parameter floats")
		}
		for _, value := range values {
			if math.IsNaN(value) || math.IsInf(value, 0) {
				panic("QuickGUI shader parameters must be finite numbers")
			}
		}
		if values == nil {
			values = []float64{}
		}
		setJson(node, protocol.ShaderParameters, 65536, values)
	})
}
