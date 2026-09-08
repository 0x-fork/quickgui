package ui

import (
	"fmt"
	"math"
)

// A preset may explicitly reset a more specific property. Keeping that distinct
// from an omitted nil field also preserves the reset in reusable style values.
type clearStyleValue struct{}

func layoutFraction(fraction float64) string {
	if math.IsNaN(fraction) || math.IsInf(fraction, 0) || fraction < 0 {
		fraction = 0
	}
	return fmt.Sprintf("%g%%", fraction*100)
}

func layoutGridTracks(count int, minimum, maximum string) string {
	if count <= 0 {
		return "none"
	}
	count = min(count, 1024)
	return fmt.Sprintf("repeat(%d, minmax(%s, %s))", count, minimum, maximum)
}
