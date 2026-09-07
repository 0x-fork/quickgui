package git

import (
	"context"
	"errors"
	"fmt"
	"os"
	"path/filepath"
	"reflect"
	"strings"
	"testing"
)

func testRepository(t *testing.T) (*Repository, context.Context) {
	t.Helper()
	root := t.TempDir()
	// Keep user signing, hooks, aliases, and global exclusions out of fixtures.
	t.Setenv("GIT_CONFIG_GLOBAL", os.DevNull)
	t.Setenv("GIT_CONFIG_NOSYSTEM", "1")
	runner := NewRunner(2)
	ctx := context.Background()
	for _, args := range [][]string{
		{"init", "-q", "-b", "main"},
		{"config", "user.email", "test@example.com"},
		{"config", "user.name", "Quick Git Tests"},
		{"config", "commit.gpgsign", "false"},
		{"config", "core.hooksPath", filepath.Join(root, "no-hooks")},
	} {
		_, err := runner.Run(ctx, args, CommandOptions{Cwd: root})
		requireOK(t, err)
	}
	repo, err := Open(ctx, runner, root, nil)
	requireOK(t, err)
	return repo, ctx
}

func requireOK(t *testing.T, err error) {
	t.Helper()
	if err != nil {
		t.Fatal(err)
	}
}

func writeRepoFile(t *testing.T, repo *Repository, name, content string) {
	t.Helper()
	path := filepath.Join(repo.Root(), name)
	requireOK(t, os.MkdirAll(filepath.Dir(path), 0o755))
	requireOK(t, os.WriteFile(path, []byte(content), 0o644))
}

func repoText(t *testing.T, repo *Repository, args ...string) string {
	t.Helper()
	text, err := repo.Runner.Text(context.Background(), args, CommandOptions{Cwd: repo.Root()})
	requireOK(t, err)
	return text
}

func TestRepositoryUnbornStageCommitAndHistory(t *testing.T) {
	repo, ctx := testRepository(t)
	writeRepoFile(t, repo, "src/app.go", "first\nsecond\n")
	writeRepoFile(t, repo, "with space.md", "# Notes\n")
	nested, err := Open(ctx, repo.Runner, filepath.Join(repo.Root(), "src"), nil)
	requireOK(t, err)
	if nested.Root() != repo.Root() {
		t.Fatalf("nested root = %q", nested.Root())
	}
	status, err := repo.Status(ctx)
	requireOK(t, err)
	if status.Branch != "main" || status.HeadSha != "" || len(UnstagedChanges(status)) != 2 {
		t.Fatalf("unborn status: %+v", status)
	}
	head, err := repo.HasHead(ctx)
	requireOK(t, err)
	if head {
		t.Fatal("unborn repository has HEAD")
	}
	commits, err := repo.Log(ctx, LogOptions{})
	requireOK(t, err)
	if len(commits) != 0 {
		t.Fatal(commits)
	}
	preview, err := repo.DiffUntracked(ctx, "with space.md")
	requireOK(t, err)
	if len(preview.Files) != 1 || preview.Files[0].Kind != FileAdded || preview.Added != 1 {
		t.Fatalf("untracked preview: %+v", preview)
	}
	requireOK(t, repo.Stage(ctx, []string{"src/app.go", "with space.md"}))
	requireOK(t, repo.Unstage(ctx, []string{"with space.md"}))
	status, err = repo.Status(ctx)
	requireOK(t, err)
	if got := StagedChanges(status); len(got) != 1 || got[0].Path != "src/app.go" {
		t.Fatal(got)
	}
	requireOK(t, repo.StageAll(ctx))
	requireOK(t, repo.Commit(ctx, "Initial commit\n\nWith a body.\n", CommitOptions{}))
	commits, err = repo.Log(ctx, LogOptions{})
	requireOK(t, err)
	if len(commits) != 1 || commits[0].Subject != "Initial commit" || strings.TrimSpace(commits[0].Body) != "With a body." || len(commits[0].Parents) != 0 {
		t.Fatalf("log: %+v", commits)
	}
	files, err := repo.CommitFiles(ctx, commits[0].Sha)
	requireOK(t, err)
	if len(files) != 2 {
		t.Fatalf("commit files: %+v", files)
	}
	diff, err := repo.DiffCommit(ctx, commits[0].Sha, "")
	requireOK(t, err)
	if len(diff.Files) != 2 || diff.Added != 3 {
		t.Fatalf("multi-file diff: %+v", diff)
	}
	status, err = repo.Status(ctx)
	requireOK(t, err)
	if !IsClean(status) {
		t.Fatal(status)
	}
}

func changes(file DiffFile) []string {
	var result []string
	for _, hunk := range file.Hunks {
		for _, line := range hunk.Lines {
			if line.Kind != LineContext {
				result = append(result, string(line.Kind)+":"+line.Text)
			}
		}
	}
	return result
}

func findHunk(t *testing.T, file DiffFile, text string) int {
	t.Helper()
	for i, hunk := range file.Hunks {
		for _, line := range hunk.Lines {
			if line.Text == text && line.Kind != LineContext {
				return i
			}
		}
	}
	t.Fatalf("missing changed line %q in %+v", text, file)
	return -1
}

func TestRepositoryPartialStageUnstageDiscardWithOffsets(t *testing.T) {
	repo, ctx := testRepository(t)
	var lines []string
	for i := 1; i <= 30; i++ {
		lines = append(lines, fmt.Sprintf("line %d", i))
	}
	writeRepoFile(t, repo, "file.txt", strings.Join(lines, "\n")+"\n")
	requireOK(t, repo.StageAll(ctx))
	requireOK(t, repo.Commit(ctx, "Base", CommitOptions{}))
	lines[0] = "line one!"
	lines = append(append(append([]string{}, lines[:12]...), "x", "x", "x"), lines[12:]...)
	lines = append(lines, "tail added")
	writeRepoFile(t, repo, "file.txt", strings.Join(lines, "\n")+"\n")
	diff, err := repo.DiffWorkingTree(ctx, "file.txt")
	requireOK(t, err)
	file := diff.Files[0]
	if len(file.Hunks) != 3 {
		t.Fatalf("hunks: %+v", file.Hunks)
	}
	requireOK(t, repo.StagePatch(ctx, file, []HunkSelection{{HunkIndex: findHunk(t, file, "tail added")}}))
	staged, err := repo.DiffIndex(ctx, "file.txt", "")
	requireOK(t, err)
	if got := changes(staged.Files[0]); !reflect.DeepEqual(got, []string{"added:tail added"}) {
		t.Fatal(got)
	}
	diff, err = repo.DiffWorkingTree(ctx, "file.txt")
	requireOK(t, err)
	file = diff.Files[0]
	top := findHunk(t, file, "line one!")
	selection := HunkSelection{HunkIndex: top, Lines: map[int]struct{}{}}
	for i, line := range file.Hunks[top].Lines {
		if line.Text == "line one!" || line.Text == "line 1" {
			selection.Lines[i] = struct{}{}
		}
	}
	requireOK(t, repo.StagePatch(ctx, file, []HunkSelection{selection}))
	staged, err = repo.DiffIndex(ctx, "file.txt", "")
	requireOK(t, err)
	if got := changes(staged.Files[0]); !reflect.DeepEqual(got, []string{"removed:line 1", "added:line one!", "added:tail added"}) {
		t.Fatal(got)
	}
	requireOK(t, repo.UnstagePatch(ctx, staged.Files[0], []HunkSelection{{HunkIndex: findHunk(t, staged.Files[0], "tail added")}}))
	if got := repoText(t, repo, "show", ":file.txt"); strings.Contains(got, "tail added") || !strings.Contains(got, "line one!") {
		t.Fatal(got)
	}
	diff, err = repo.DiffWorkingTree(ctx, "file.txt")
	requireOK(t, err)
	requireOK(t, repo.DiscardPatch(ctx, diff.Files[0], []HunkSelection{{HunkIndex: findHunk(t, diff.Files[0], "x")}}))
	content, err := os.ReadFile(filepath.Join(repo.Root(), "file.txt"))
	requireOK(t, err)
	if strings.Contains(string(content), "\nx\n") || !strings.Contains(string(content), "tail added") || !strings.Contains(string(content), "line one!") {
		t.Fatal(string(content))
	}
}

func TestRepositoryPartialNewAndDeletedFileAndNoNewline(t *testing.T) {
	for _, kind := range []string{"added", "deleted", "no-newline"} {
		t.Run(kind, func(t *testing.T) {
			repo, ctx := testRepository(t)
			requireOK(t, repo.Commit(ctx, "Base", CommitOptions{AllowEmpty: true}))
			switch kind {
			case "added":
				writeRepoFile(t, repo, "new.txt", "one\ntwo\nthree\n")
				requireOK(t, repo.StageAll(ctx))
				diff, err := repo.DiffIndex(ctx, "new.txt", "")
				requireOK(t, err)
				requireOK(t, repo.UnstagePatch(ctx, diff.Files[0], []HunkSelection{{HunkIndex: 0, Lines: map[int]struct{}{1: {}}}}))
				if got := repoText(t, repo, "show", ":new.txt"); got != "one\nthree\n" {
					t.Fatal(got)
				}
			case "deleted":
				writeRepoFile(t, repo, "old.txt", "one\ntwo\nthree\n")
				requireOK(t, repo.StageAll(ctx))
				requireOK(t, repo.Commit(ctx, "Add old file", CommitOptions{}))
				requireOK(t, os.Remove(filepath.Join(repo.Root(), "old.txt")))
				diff, err := repo.DiffWorkingTree(ctx, "old.txt")
				requireOK(t, err)
				requireOK(t, repo.StagePatch(ctx, diff.Files[0], []HunkSelection{{HunkIndex: 0, Lines: map[int]struct{}{1: {}}}}))
				if got := repoText(t, repo, "show", ":old.txt"); got != "one\nthree\n" {
					t.Fatal(got)
				}
			case "no-newline":
				writeRepoFile(t, repo, "plain.txt", "before")
				requireOK(t, repo.StageAll(ctx))
				requireOK(t, repo.Commit(ctx, "Before", CommitOptions{}))
				writeRepoFile(t, repo, "plain.txt", "after")
				diff, err := repo.DiffWorkingTree(ctx, "plain.txt")
				requireOK(t, err)
				requireOK(t, repo.StagePatch(ctx, diff.Files[0], []HunkSelection{{HunkIndex: 0}}))
				if got := repoText(t, repo, "show", ":plain.txt"); got != "after" {
					t.Fatal(got)
				}
			}
		})
	}
}

func TestRepositoryBranchesStashesWorktreesAndErrors(t *testing.T) {
	repo, ctx := testRepository(t)
	writeRepoFile(t, repo, "base.txt", "base\n")
	requireOK(t, repo.StageAll(ctx))
	requireOK(t, repo.Commit(ctx, "Initial", CommitOptions{}))
	requireOK(t, repo.CreateBranch(ctx, "feature/login", "", false))
	refs, err := repo.Refs(ctx)
	requireOK(t, err)
	if len(refs.Local) != 2 {
		t.Fatal(refs)
	}
	writeRepoFile(t, repo, "stash.txt", "wip\n")
	requireOK(t, repo.StashPush(ctx, "my stash", true, false))
	stashes, err := repo.Stashes(ctx)
	requireOK(t, err)
	if len(stashes) != 1 || stashes[0].Summary != "my stash" || stashes[0].Branch != "main" {
		t.Fatal(stashes)
	}
	requireOK(t, repo.StashPop(ctx, stashes[0].Ref))
	requireOK(t, os.Remove(filepath.Join(repo.Root(), "stash.txt")))
	linkedPath := filepath.Join(t.TempDir(), "feature")
	requireOK(t, repo.AddWorktree(ctx, linkedPath, "", "feature/login", ""))
	linked, err := repo.Worktree(ctx, linkedPath)
	requireOK(t, err)
	if linked.Info.CommonDir != repo.Info.CommonDir {
		t.Fatalf("common dirs: %q != %q", linked.Info.CommonDir, repo.Info.CommonDir)
	}
	writeRepoFile(t, linked, "feature.txt", "feature\n")
	requireOK(t, linked.StageAll(ctx))
	requireOK(t, linked.Commit(ctx, "Feature work", CommitOptions{}))
	if got := repoText(t, repo, "log", "-1", "--format=%s"); got != "Initial\n" {
		t.Fatal(got)
	}
	worktrees, err := repo.Worktrees(ctx)
	requireOK(t, err)
	if len(worktrees) != 2 {
		t.Fatal(worktrees)
	}
	requireOK(t, repo.RemoveWorktree(ctx, linkedPath, true))
	requireOK(t, repo.DeleteBranch(ctx, "feature/login", true))
	writeRepoFile(t, repo, "base.txt", "changed\n")
	writeRepoFile(t, repo, "junk.txt", "junk\n")
	var trashed []string
	repo.Trash = func(path string) error { trashed = append(trashed, path); return os.Remove(path) }
	requireOK(t, repo.Discard(ctx, []string{"base.txt"}, []string{"junk.txt"}))
	if !reflect.DeepEqual(trashed, []string{filepath.Join(repo.Root(), "junk.txt")}) {
		t.Fatal(trashed)
	}
	var failure *Error
	if err := repo.SwitchBranch(ctx, "does-not-exist"); !errors.As(err, &failure) || failure.ExitCode == 0 || failure.Summary() == "" {
		t.Fatalf("failure: %v", err)
	}
	cancelled, cancel := context.WithCancel(ctx)
	cancel()
	if _, err := repo.Status(cancelled); !errors.As(err, &failure) || !failure.Aborted {
		t.Fatalf("cancel: %v", err)
	}
}
