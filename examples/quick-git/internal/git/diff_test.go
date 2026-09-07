package git

import (
	_ "embed"
	"reflect"
	"strings"
	"testing"
)

//go:embed testdata/sample.diff
var sampleDiff string

func TestDiffKindsNumbersPathsAndTruncation(t *testing.T) {
	files := ParseDiff(sampleDiff, true)
	if len(files) != 5 {
		t.Fatal(files)
	}
	for i, want := range []struct {
		kind  DiffFileKind
		path  string
		hunks int
	}{
		{FileModified, "src/app.ts", 2}, {FileDeleted, "README.md", 1},
		{FileRenamed, "new dir/name.txt", 1}, {FileAdded, "image.png", 0},
		{FileModified, "script.sh", 0},
	} {
		file := files[i]
		if file.Kind != want.kind || file.Path != want.path || len(file.Hunks) != want.hunks || file.Truncated != (i == 4) {
			t.Fatalf("file %d: %+v", i, file)
		}
	}
	first := files[0].Hunks[0]
	if *first.Lines[1].OldLineNumber != 2 || first.Lines[1].NewLineNumber != nil || *first.Lines[3].NewLineNumber != 3 || first.Lines[3].OldLineNumber != nil {
		t.Fatal(first.Lines)
	}
	if !files[0].Hunks[1].Lines[3].NoNewline || !files[3].Binary || files[3].OldPath != "" || files[1].NewPath != "" || files[4].NewMode != "100755" {
		t.Fatal(files)
	}
	for _, pair := range [][2]string{{"with space.txt", "with space.txt"}, {"a b/c.txt", "a b/c.txt"}, {"one.txt", "two.txt"}} {
		if got := parseGitHeaderPaths("a/" + pair[0] + " b/" + pair[1]); got != pair {
			t.Fatalf("paths %v != %v", got, pair)
		}
	}
	diff := OpenDiff(sampleDiff, true)
	if diff.Added != 4 || diff.Removed != 5 || diff.RowCount != len(diff.Rows) {
		t.Fatalf("counts: %+v", diff)
	}
	selected := diff.SelectedLines([][]int{{-5, diff.RowCount + 5}, {1, diff.RowCount}})
	if len(selected) != 4 {
		t.Fatalf("changed hunk groups: %+v", selected)
	}
	for _, group := range selected {
		for line := range group.Lines {
			if files[group.FileIndex].Hunks[group.HunkIndex].Lines[line].Kind == LineContext {
				t.Fatal("selected context")
			}
		}
	}
}

func TestPatchSelectionsMergeAndKeepHeaders(t *testing.T) {
	file := ParseDiff(sampleDiff, false)[0]
	for _, lines := range []map[int]struct{}{{}, {0: {}}} {
		if got := FormatPatch(file, []HunkSelection{{HunkIndex: 0, Lines: lines}}, false); got != "" {
			t.Fatalf("selection with no changes: %q", got)
		}
	}
	whole := []HunkSelection{{HunkIndex: 0}, {HunkIndex: 1}}
	reordered := []HunkSelection{{HunkIndex: 1}, {HunkIndex: 0, Lines: map[int]struct{}{1: {}}}, {HunkIndex: 0}, {HunkIndex: 1}}
	if got, want := FormatPatch(file, reordered, false), FormatPatch(file, whole, false); got != want {
		t.Fatalf("overlapping selections duplicated changes:\n%s", got)
	}
	patch := FormatPatch(file, []HunkSelection{{HunkIndex: 1}}, false)
	if !strings.Contains(patch, "@@ -10,3 +10,3 @@\n") || !strings.Contains(patch, "\\ No newline at end of file\n") {
		t.Fatal(patch)
	}
	renamed := ParseDiff(sampleDiff, false)[2]
	patch = FormatPatch(renamed, []HunkSelection{{HunkIndex: 0}}, false)
	if !strings.HasPrefix(patch, "diff --git a/old dir/name.txt b/new dir/name.txt\nsimilarity index 90%\nrename from old dir/name.txt\nrename to new dir/name.txt\n") {
		t.Fatal(patch)
	}
	for _, text := range []string{"", "a\nb", "a\nb\n"} {
		files := ParseDiff(SyntheticDiffText("notes.md", text, false), false)
		if len(files) != 1 || files[0].Kind != FileAdded {
			t.Fatal(files)
		}
		if text != "" && !reflect.DeepEqual(changes(files[0]), []string{"added:a", "added:b"}) {
			t.Fatal(files)
		}
	}
}
