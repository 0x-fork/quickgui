package model

import (
	"context"
	"os"
	"path/filepath"
	"reflect"
	"strings"
	"testing"
	"time"

	"github.com/egoist/quickgui/go/reactive"
	"quickgui.example/quick-git/internal/git"
)

func refreshTestStore(t *testing.T) (*Store, <-chan func(), func(...string) string, func(string, string)) {
	t.Helper()
	t.Setenv("GIT_CONFIG_GLOBAL", os.DevNull)
	t.Setenv("GIT_CONFIG_NOSYSTEM", "1")
	root := t.TempDir()
	runner := git.NewRunner(2)
	run := func(args ...string) string {
		t.Helper()
		output, err := runner.Text(context.Background(), args, git.CommandOptions{Cwd: root})
		if err != nil {
			t.Fatal(err)
		}
		return strings.TrimSpace(output)
	}
	write := func(name, content string) {
		t.Helper()
		if err := os.WriteFile(filepath.Join(root, name), []byte(content), 0o644); err != nil {
			t.Fatal(err)
		}
	}
	run("init", "-q", "-b", "main")
	run("config", "user.name", "Quick Git Tests")
	run("config", "user.email", "test@example.com")
	run("config", "commit.gpgsign", "false")
	run("config", "core.hooksPath", filepath.Join(root, "no-hooks"))
	write("a.txt", "first\n")
	write("z.txt", "first\n")
	run("add", ".")
	run("commit", "-qm", "Initial")
	write("a.txt", "second\n")
	write("z.txt", "second\n")
	run("commit", "-qam", "Second")
	repo, err := git.Open(context.Background(), runner, root, nil)
	if err != nil {
		t.Fatal(err)
	}
	queue := make(chan func(), 256)
	store := CreateStore(StoreOptions{
		Runner:      runner,
		Persistence: CreatePersistence("", DefaultState(), 0),
		enqueue:     func(fn func()) { queue <- fn },
	})
	t.Cleanup(store.Dispose)
	// Install the fixture without starting native filesystem watchers.
	store.repo, store.main = repo, repo
	reactive.Batch(func() {
		store.setRepository(repo)
		store.setMain(repo)
	})
	return store, queue, run, write
}

func waitStore(t *testing.T, queue <-chan func(), description string, ready func() bool) {
	t.Helper()
	timer := time.NewTimer(5 * time.Second)
	defer timer.Stop()
	for !ready() {
		select {
		case fn := <-queue:
			// Native dispatch applies each turn's writes atomically before effects run.
			reactive.Batch(fn)
		case <-timer.C:
			t.Fatalf("timed out waiting for %s", description)
		}
	}
}

func TestHistoryRefreshRetainsCommitFileAndDiff(t *testing.T) {
	store, queue, run, write := refreshTestStore(t)
	store.SetView(ViewHistory)
	waitStore(t, queue, "initial history diff", func() bool {
		return len(store.History().Commits) == 2 && store.Diff().Diff != nil && !store.Diff().Loading
	})
	store.SelectCommitFile("z.txt")
	waitStore(t, queue, "selected file diff", func() bool {
		diff := store.Diff()
		return diff.Target != nil && diff.Target.Path == "z.txt" && diff.Diff != nil && !diff.Loading
	})
	selection := [][]int{{2, 2}}
	store.SetDiffSelection(selection)
	selectedSHA := store.SelectedCommit().Sha
	parsed := store.Diff().Diff
	detailChanges, diffChanges := 0, 0
	sawHistoryLoading := false
	reactive.CreateRoot(func(dispose func()) struct{} {
		t.Cleanup(dispose)
		reactive.CreateEffect(func() {
			store.CommitDetail()
			detailChanges++
		})
		reactive.CreateEffect(func() {
			store.Diff()
			diffChanges++
		})
		reactive.CreateEffect(func() {
			if store.History().Loading {
				sawHistoryLoading = true
			}
		})
		return struct{}{}
	})
	refresh := func() {
		t.Helper()
		detailChanges, diffChanges = 0, 0
		sawHistoryLoading = false
		generation := store.Generation()
		store.Refresh() // The same path used when the window regains focus.
		waitStore(t, queue, "history refresh", func() bool {
			return store.Generation() > generation && sawHistoryLoading && !store.History().Loading &&
				!store.CommitDetail().Loading && !store.Diff().Loading
		})
		if detailChanges != 0 || diffChanges != 0 {
			t.Fatalf("refresh replaced visible commit details %d times and diff %d times", detailChanges, diffChanges)
		}
		if store.SelectedCommit().Sha != selectedSHA || store.CommitDetail().SelectedPath != "z.txt" ||
			store.Diff().Diff != parsed || !reflect.DeepEqual(store.DiffSelection(), selection) {
			t.Fatal("refresh lost the selected commit, file, diff, or line selection")
		}
	}
	refresh()
	refresh()

	// A commit made while unfocused moves the selected row without changing its contents.
	write("a.txt", "third\n")
	run("commit", "-qam", "Third")
	latestSHA := run("rev-parse", "HEAD")
	refresh()
	if len(store.History().Commits) != 3 || store.History().Commits[0].Sha != latestSHA ||
		!reflect.DeepEqual(store.HistorySelection(), [][]int{{1, 1}}) {
		t.Fatal("refresh did not update history and follow the original commit to its new row")
	}
	store.SelectCommit(0)
	waitStore(t, queue, "newly selected commit", func() bool {
		diff := store.Diff()
		return diff.Target != nil && diff.Target.SHA == latestSHA && diff.Diff != nil && !diff.Loading
	})
	if !strings.Contains(store.Diff().Diff.Render(), "+third") {
		t.Fatal("selecting a different commit retained the previous diff")
	}
}

func TestRefreshUpdatesWorkingTreeDiff(t *testing.T) {
	store, queue, _, write := refreshTestStore(t)
	write("a.txt", "working tree\n")
	store.Refresh()
	waitStore(t, queue, "working tree diff", func() bool {
		return store.Diff().Diff != nil && !store.Diff().Loading
	})
	previous := store.Diff().Diff
	write("a.txt", "changed while unfocused\n")
	store.Refresh()
	waitStore(t, queue, "updated working tree diff", func() bool {
		diff := store.Diff()
		return diff.Diff != nil && diff.Diff != previous && !diff.Loading
	})
	if !strings.Contains(store.Diff().Diff.Render(), "+changed while unfocused") {
		t.Fatal("refresh retained an outdated working tree diff")
	}
}
