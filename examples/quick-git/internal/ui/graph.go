package ui

import (
	"fmt"
	"math"
	"strings"

	gui "github.com/egoist/quickgui/go/ui"
	"quickgui.example/quick-git/internal/git"
)

const (
	historyRowHeight = 26
	graphLaneWidth   = 14
	graphInset       = 6
)

type graphLayer struct {
	Color  int
	Source string
}

func graphLaneX(lane int) float64 {
	return graphInset + float64(lane)*graphLaneWidth + graphLaneWidth/2
}

// Each layer is an SVG mask tinted with one lane color. Paths reach the row's
// exact edges so the virtualized rows form a continuous graph.
func paintGraphRow(row git.GraphRow, width, height float64) []graphLayer {
	middle := height / 2
	radius := math.Min(graphLaneWidth/2, math.Max(1, middle-1))
	segments := map[int][]string{}
	var colors []int
	add := func(color int, path string) {
		if _, ok := segments[color]; !ok {
			colors = append(colors, color)
		}
		segments[color] = append(segments[color], path)
	}
	x := graphLaneX(row.Lane)
	for _, lane := range row.Passing {
		add(lane.Color, fmt.Sprintf("M%g 0V%g", graphLaneX(lane.Lane), height))
	}
	if row.Incoming {
		add(row.Color, fmt.Sprintf("M%g 0V%g", x, middle))
	}
	for _, join := range row.Joins {
		from := graphLaneX(join.FromLane)
		if from == x {
			add(join.Color, fmt.Sprintf("M%g 0V%g", x, middle))
			continue
		}
		direction, sweep := 1.0, 0
		if from > x {
			direction, sweep = -1, 1
		}
		add(join.Color, fmt.Sprintf("M%g 0V%gA%g %g 0 0 %d %g %gH%g", from, middle-radius, radius, radius, sweep, from+direction*radius, middle, x))
	}
	for _, edge := range row.Edges {
		to := graphLaneX(edge.ToLane)
		if to == x {
			add(edge.Color, fmt.Sprintf("M%g %gV%g", x, middle, height))
			continue
		}
		direction, sweep := -1.0, 0
		if to > x {
			direction, sweep = 1, 1
		}
		add(edge.Color, fmt.Sprintf("M%g %gH%gA%g %g 0 0 %d %g %gV%g", x, middle, to-direction*radius, radius, radius, sweep, to, middle+radius, height))
	}
	if _, ok := segments[row.Color]; !ok {
		add(row.Color, "")
	}
	layers := make([]graphLayer, 0, len(colors))
	for _, color := range colors {
		var body strings.Builder
		if path := strings.Join(segments[color], ""); path != "" {
			fmt.Fprintf(&body, `<path d="%s" fill="none" stroke="#000" stroke-width="2"/>`, path)
		}
		if color == row.Color {
			fmt.Fprintf(&body, `<circle cx="%g" cy="%g" r="4" fill="#000"/>`, x, middle)
		}
		layers = append(layers, graphLayer{Color: color, Source: fmt.Sprintf(`<svg xmlns="http://www.w3.org/2000/svg" width="%g" height="%g" viewBox="0 0 %g %g">%s</svg>`, width, height, width, height, body.String())})
	}
	return layers
}

func historyGraph(row func() *git.GraphRow, width func() float64) {
	app := UseApp()
	gui.View(
		gui.Position("relative"),
		gui.Width(width),
		gui.Height(historyRowHeight),
		gui.FlexShrink(0),
		gui.Overflow("hidden"),
		func() {
			gui.KeyedFor(
				func() []graphLayer {
					if current := row(); current != nil {
						return paintGraphRow(*current, width(), historyRowHeight)
					}
					return nil
				},
				func(layer graphLayer) any { return layer.Color },
				func(layer func() graphLayer, _ func() int) {
					gui.SVG(
						gui.Position("absolute"),
						gui.Left(0),
						gui.Top(0),
						gui.Width(width),
						gui.Height(historyRowHeight),
						gui.Color(func() string {
							palette := app.Theme().Graph
							return palette[layer().Color%len(palette)]
						}),
						gui.Value(func() string { return layer().Source }),
					)
				},
				nil,
			)
		},
	)
}
