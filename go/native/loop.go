package native

import (
	"errors"
	"sync"
)

type hostEvent struct {
	kind   string
	window uint32
	target uint32
	flags  uint32
	value  string
	extra  string
	data   []byte
}

const maxQueuedWork = 8192

var (
	loopStopped  = make(chan struct{})
	loopFailures = make(chan error, 1)
	workMu       sync.Mutex
	workQueue    []func()
	workWake     = make(chan struct{}, 1)
	eventMu      sync.Mutex
	eventQueue   []hostEvent
)

// Dispatch queues fn for the application goroutine. Event handlers already run there.
func Dispatch(fn func()) {
	select {
	case <-loopStopped:
		return
	default:
	}
	workMu.Lock()
	if len(workQueue) >= maxQueuedWork {
		workMu.Unlock()
		failQueue()
		return
	}
	workQueue = append(workQueue, fn)
	workMu.Unlock()
	select {
	case workWake <- struct{}{}:
	default:
	}
}

func enqueueHostEvent(kind string, window, target, flags uint32, value, extra string, data []byte) {
	select {
	case <-loopStopped:
		return
	default:
	}
	eventMu.Lock()
	if len(eventQueue) >= maxQueuedWork {
		eventMu.Unlock()
		failQueue()
		return
	}
	eventQueue = append(eventQueue, hostEvent{
		kind: kind, window: window, target: target, flags: flags,
		value: value, extra: extra, data: data,
	})
	eventMu.Unlock()
	select {
	case workWake <- struct{}{}:
	default:
	}
}

func drainWork() []func() {
	workMu.Lock()
	jobs := workQueue
	workQueue = nil
	workMu.Unlock()
	return jobs
}

func drainEvents() []hostEvent {
	eventMu.Lock()
	events := eventQueue
	eventQueue = nil
	eventMu.Unlock()
	return events
}

func failQueue() {
	select {
	case loopFailures <- errors.New("QuickGUI application queue exceeded 8192 entries"):
	default:
	}
	select {
	case workWake <- struct{}{}:
	default:
	}
}
