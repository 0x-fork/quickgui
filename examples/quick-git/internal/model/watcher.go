package model

import (
	"log"
	"path/filepath"
	"strings"
	"time"

	"github.com/egoist/quickgui/go/native"
)

type ChangeKind string

const (
	ChangeWorktree ChangeKind = "worktree"
	ChangeRefs     ChangeKind = "refs"
	ChangeIndex    ChangeKind = "index"
)

type WatcherOptions struct {
	Root      string
	GitDir    string
	CommonDir string
	OnChange  func(kinds map[ChangeKind]struct{})
	Debounce  time.Duration
}

func ClassifyGitPath(path string) ChangeKind {
	normalized := filepath.ToSlash(path)
	if strings.HasPrefix(normalized, "objects/") || strings.HasPrefix(normalized, "lfs/") || normalized == "objects" {
		return ""
	}
	if normalized == "index" || normalized == "index.lock" {
		return ChangeIndex
	}
	if normalized == "HEAD" || normalized == "ORIG_HEAD" || normalized == "FETCH_HEAD" ||
		normalized == "MERGE_HEAD" || normalized == "packed-refs" ||
		strings.HasPrefix(normalized, "refs/") || strings.HasPrefix(normalized, "logs/") ||
		strings.HasPrefix(normalized, "worktrees/") {
		return ChangeRefs
	}
	if strings.HasSuffix(normalized, ".lock") || normalized == "COMMIT_EDITMSG" {
		return ""
	}
	return ChangeRefs
}

func ClassifyWorktreePath(path, gitDirRelative string) ChangeKind {
	normalized := filepath.ToSlash(path)
	gitRelative := filepath.ToSlash(gitDirRelative)
	if normalized == ".git" || normalized == gitRelative {
		return ChangeRefs
	}
	if strings.HasPrefix(normalized, ".git/") {
		return ClassifyGitPath(normalized[5:])
	}
	if gitRelative != "" && strings.HasPrefix(normalized, gitRelative+"/") {
		return ClassifyGitPath(normalized[len(gitRelative)+1:])
	}
	return ChangeWorktree
}

// WatchRepository uses the Rust core's operating-system watcher. Debouncing
// starts only after a notification, so an unchanged repository has no polling loop.
func WatchRepository(options WatcherOptions) func() {
	debounce := options.Debounce
	if debounce <= 0 {
		debounce = 150 * time.Millisecond
	}
	pending := map[ChangeKind]struct{}{}
	var timer *time.Timer
	closed := false
	flush := func() {
		if closed || len(pending) == 0 {
			return
		}
		kinds := pending
		pending = map[ChangeKind]struct{}{}
		timer = nil
		if options.OnChange != nil {
			options.OnChange(kinds)
		}
	}
	note := func(kind ChangeKind) {
		if kind == "" {
			return
		}
		pending[kind] = struct{}{}
		if timer != nil {
			timer.Stop()
		}
		timer = time.AfterFunc(debounce, func() { native.Dispatch(flush) })
	}
	paths := []string{options.Root}
	for _, path := range []string{options.GitDir, options.CommonDir} {
		if path == "" {
			continue
		}
		exists := false
		for _, previous := range paths {
			if previous == path {
				exists = true
			}
		}
		if !exists {
			paths = append(paths, path)
		}
	}
	stop := native.WatchFiles(
		paths,
		func(event native.FileWatchEvent) {
			if closed {
				return
			}
			if event.Error != "" {
				log.Printf("Quick Git watcher: %s", event.Error)
			}
			if event.Rescan || event.Error != "" {
				note(ChangeWorktree)
				note(ChangeRefs)
				note(ChangeIndex)
			}
			for _, path := range event.Paths {
				gitPath := false
				for _, directory := range []string{options.GitDir, options.CommonDir} {
					if relative, ok := relativeChild(directory, path); ok {
						note(ClassifyGitPath(relative))
						gitPath = true
						break
					}
				}
				if gitPath {
					continue
				}
				if relative, ok := relativeChild(options.Root, path); ok {
					ignored := false
					for _, segment := range strings.Split(filepath.ToSlash(relative), "/") {
						if segment == "node_modules" || segment == ".quickgui" || segment == "target" {
							ignored = true
							break
						}
					}
					if !ignored {
						note(ClassifyWorktreePath(relative, ".git"))
					}
				}
			}
		},
		func(err error) {
			if err != nil {
				log.Printf("Quick Git watcher: %v", err)
			}
		},
	)
	return func() {
		if closed {
			return
		}
		closed = true
		if timer != nil {
			timer.Stop()
		}
		stop()
	}
}

func relativeChild(directory, path string) (string, bool) {
	if directory == "" {
		return "", false
	}
	relative, err := filepath.Rel(directory, path)
	return relative, err == nil && relative != ".." && !strings.HasPrefix(relative, ".."+string(filepath.Separator))
}
