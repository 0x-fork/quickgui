package native

import (
	"slices"

	"github.com/egoist/quickgui/go/reactive"
)

// Ordered listeners tolerate removal during delivery and release their captured
// component context on unsubscribe. Registrations belong to the UI goroutine.
type subscriptions[T any] struct {
	next    uint64
	order   []uint64
	entries map[uint64]func(T)
	changed func()
}

func (s *subscriptions[T]) add(listener func(T)) func() {
	if listener == nil {
		return func() {}
	}
	if s.entries == nil {
		s.entries = map[uint64]func(T){}
	}
	s.next++
	id := s.next
	owner, window := reactive.GetOwner(), contextWindow()
	s.entries[id] = func(event T) {
		if owner != nil && owner.Disposed || window != nil && window.Closed {
			return
		}
		withCurrentWindow(window, func() {
			reactive.RunWithOwner(owner, func() struct{} {
				reactive.Batch(func() { listener(event) })
				return struct{}{}
			})
		})
	}
	s.order = append(s.order, id)
	if s.changed != nil {
		s.changed()
	}
	stop := func() {
		if _, present := s.entries[id]; !present {
			return
		}
		delete(s.entries, id)
		if index := slices.Index(s.order, id); index >= 0 {
			s.order = slices.Delete(s.order, index, index+1)
		}
		if s.changed != nil {
			s.changed()
		}
	}
	if owner != nil {
		reactive.OnCleanup(stop)
	}
	return stop
}

func (s *subscriptions[T]) emit(event T) {
	for _, id := range append([]uint64{}, s.order...) {
		if listener := s.entries[id]; listener != nil {
			listener(event)
		}
	}
}

func (s *subscriptions[T]) clear() {
	s.entries = nil
	s.order = nil
}
