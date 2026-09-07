package main

import (
	"encoding/json"
	"os"
	"path/filepath"
	"time"
)

type savedSpace struct {
	Path string `json:"path"`
}
type savedState struct {
	Spaces              []savedSpace `json:"spaces"`
	ActiveSpaceID       string       `json:"activeSpaceId"`
	SidebarWidth        *float64     `json:"sidebarWidth,omitempty"`
	SidebarSectionRatio *float64     `json:"sidebarSectionRatio,omitempty"`
	Appearance          string       `json:"appearance,omitempty"`
}

func normalizeSpaces(saved []savedSpace, home string) []space {
	var result []space
	seen := map[string]bool{}
	for _, s := range append(append([]savedSpace(nil), saved...), savedSpace{Path: home}) {
		if s.Path != "" && !seen[s.Path] {
			seen[s.Path] = true
			result = append(result, space{ID: s.Path, Name: filepath.Base(s.Path), Path: s.Path})
		}
	}
	return result
}
func readSavedState(file string) (savedState, error) {
	var state savedState
	body, err := os.ReadFile(file)
	if err != nil {
		return state, err
	}
	// Keep valid settings if an older version left an unrelated malformed field.
	var fields map[string]json.RawMessage
	if err = json.Unmarshal(body, &fields); err != nil {
		return state, err
	}
	_ = json.Unmarshal(fields["spaces"], &state.Spaces)
	_ = json.Unmarshal(fields["activeSpaceId"], &state.ActiveSpaceID)
	_ = json.Unmarshal(fields["sidebarWidth"], &state.SidebarWidth)
	_ = json.Unmarshal(fields["sidebarSectionRatio"], &state.SidebarSectionRatio)
	_ = json.Unmarshal(fields["appearance"], &state.Appearance)
	return state, nil
}
func writeSavedState(file string, state savedState) error {
	body, err := json.MarshalIndent(state, "", "  ")
	if err != nil {
		return err
	}
	if err = os.MkdirAll(filepath.Dir(file), 0700); err != nil {
		return err
	}
	temp, err := os.CreateTemp(filepath.Dir(file), ".herdr-state-*")
	if err != nil {
		return err
	}
	defer os.Remove(temp.Name())
	if _, err = temp.Write(append(body, '\n')); err != nil {
		temp.Close()
		return err
	}
	if err = temp.Close(); err != nil {
		return err
	}
	return os.Rename(temp.Name(), file)
}

// One writer serializes and coalesces immutable snapshots. Shutdown flushes the
// last snapshot after the native app has exited, including a quick close/reopen.
type stateWriter struct {
	latest     *savedState // Accessed only by the application goroutine.
	updates    chan savedState
	stop, done chan struct{}
}

func newStateWriter(file string) *stateWriter {
	w := &stateWriter{updates: make(chan savedState, 1), stop: make(chan struct{}), done: make(chan struct{})}
	go func() {
		defer close(w.done)
		var pending *savedState
		var timer *time.Timer
		var tick <-chan time.Time
		flush := func() {
			if pending != nil {
				_ = writeSavedState(file, *pending)
				pending = nil
			}
		}
		for {
			select {
			case state := <-w.updates:
				pending = &state
				if timer != nil {
					timer.Stop()
				}
				timer = time.NewTimer(180 * time.Millisecond)
				tick = timer.C
			case <-tick:
				flush()
				tick = nil
			case <-w.stop:
				if timer != nil {
					timer.Stop()
				}
				select {
				case state := <-w.updates:
					pending = &state
				default:
				}
				flush()
				return
			}
		}
	}()
	return w
}
func (w *stateWriter) save(state savedState) {
	w.latest = &state
	select {
	case w.updates <- state:
	default:
		select {
		case <-w.updates:
		default:
		}
		w.updates <- state
	}
}
func (w *stateWriter) close() { close(w.stop); <-w.done }
