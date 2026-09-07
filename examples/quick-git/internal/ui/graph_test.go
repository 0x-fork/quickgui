package ui

import (
	"strings"
	"testing"

	"quickgui.example/quick-git/internal/git"
)

func TestGraphConnectsAdjacentRowsAndEndsAtRoots(t *testing.T) {
	commits := []git.Commit{{Sha: "tip", Parents: []string{"middle"}}, {Sha: "middle", Parents: []string{"root"}}, {Sha: "root"}}
	rows := git.LayoutGraph(commits)
	want := []string{`d="M13 13V26"`, `d="M13 0V13M13 13V26"`, `d="M13 0V13"`}
	for i, row := range rows {
		layers := paintGraphRow(row, 26, historyRowHeight)
		if len(layers) != 1 || !strings.Contains(layers[0].Source, want[i]) || !strings.Contains(layers[0].Source, `<circle cx="13" cy="13" r="4"`) {
			t.Fatalf("row %d breaks the continuous lane: %+v", i, layers)
		}
	}
}

func TestGraphSeparatesLaneColorsAndUsesRoundedMerges(t *testing.T) {
	row := git.GraphRow{
		Lane: 0, Color: 0, Incoming: true, LaneCount: 4,
		Passing: []git.GraphPassing{{Lane: 1, Color: 1}},
		Joins:   []git.GraphJoin{{FromLane: 3, Color: 3}},
		Edges:   []git.GraphEdge{{FromLane: 0, ToLane: 0, Color: 0}, {FromLane: 0, ToLane: 2, Color: 2}},
	}
	byColor := map[int]string{}
	for _, layer := range paintGraphRow(row, 70, historyRowHeight) {
		byColor[layer.Color] = layer.Source
	}
	for color, path := range map[int]string{0: "M13 0V13M13 13V26", 1: "M27 0V26", 2: "M13 13H34A7 7 0 0 1 41 20V26", 3: "M55 0V6A7 7 0 0 1 48 13H13"} {
		if !strings.Contains(byColor[color], path) {
			t.Errorf("lane %d has wrong merge geometry: %s", color, byColor[color])
		}
		if strings.Contains(byColor[color], "<circle") != (color == row.Color) {
			t.Errorf("lane %d paints the wrong node", color)
		}
	}
}

func TestGraphLeftElbowsAndShortRows(t *testing.T) {
	row := git.GraphRow{Lane: 2, Color: 2, Joins: []git.GraphJoin{{FromLane: 1, Color: 1}}, Edges: []git.GraphEdge{{FromLane: 2, ToLane: 0, Color: 0}}}
	layers := paintGraphRow(row, 60, 10)
	paths := map[int]string{}
	for _, layer := range layers {
		paths[layer.Color] = layer.Source
	}
	if !strings.Contains(paths[0], "M41 5H17A4 4 0 0 0 13 9V10") || !strings.Contains(paths[1], "M27 0V1A4 4 0 0 0 31 5H41") {
		t.Fatalf("left elbows escape the short row or turn the wrong way: %v", paths)
	}
}
