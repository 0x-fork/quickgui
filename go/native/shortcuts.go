package native

import "github.com/egoist/quickgui/go/reactive"

type globalShortcutAPI struct{}

var GlobalShortcut globalShortcutAPI
var nextShortcut uint32 = 1
var shortcutRegistrations = map[uint32]*ShortcutRegistration{}

// ShortcutRegistration owns one operating-system hotkey. All operations and
// callbacks run on the application goroutine, including Unregister completion.
type ShortcutRegistration struct {
	ID            uint32
	Accelerator   string
	registered    bool
	callback      menuCallback
	unregistering bool
	waiters       []func(error)
}

func (globalShortcutAPI) Register(accelerator string, listener func(), done func(*ShortcutRegistration, error)) {
	if nextShortcut >= 0xffff_fff0 {
		panic("QuickGUI shortcut id space exhausted")
	}
	registration := &ShortcutRegistration{
		ID: nextShortcut, Accelerator: accelerator,
		callback: menuCallback{click: listener, owner: reactive.GetOwner(), window: contextWindow()},
	}
	nextShortcut++
	shortcutRegistrations[registration.ID] = registration
	commandVoid(map[string]any{
		"method": "global-shortcut", "action": "register",
		"registration": registration.ID, "accelerator": accelerator,
	}, func(err error) {
		if err != nil {
			delete(shortcutRegistrations, registration.ID)
			registration.callback = menuCallback{}
			if done != nil {
				done(nil, err)
			}
			return
		}
		registration.registered = true
		if done != nil {
			done(registration, nil)
		}
	})
}

func (globalShortcutAPI) IsRegistered(accelerator string) bool {
	for _, registration := range shortcutRegistrations {
		if registration.registered && registration.Accelerator == accelerator {
			return true
		}
	}
	return false
}

func (registration *ShortcutRegistration) Unregister(done func(error)) {
	if registration == nil || !registration.registered {
		if done != nil {
			done(nil)
		}
		return
	}
	registration.waiters = append(registration.waiters, done)
	if registration.unregistering {
		return
	}
	registration.unregistering = true
	commandVoid(map[string]any{"method": "global-shortcut", "action": "unregister", "registration": registration.ID}, func(err error) {
		registration.unregistering = false
		if err == nil {
			registration.release()
		}
		waiters := registration.waiters
		registration.waiters = nil
		for _, waiter := range waiters {
			if waiter != nil {
				waiter(err)
			}
		}
	})
}

func (globalShortcutAPI) UnregisterAll(done func(error)) {
	// Later registrations are separate commands and must survive this completion.
	previous := make([]*ShortcutRegistration, 0, len(shortcutRegistrations))
	for _, registration := range shortcutRegistrations {
		previous = append(previous, registration)
	}
	commandVoid(map[string]any{"method": "global-shortcut", "action": "unregister-all"}, func(err error) {
		if err == nil {
			for _, registration := range previous {
				registration.release()
			}
		}
		if done != nil {
			done(err)
		}
	})
}

func (registration *ShortcutRegistration) release() {
	delete(shortcutRegistrations, registration.ID)
	registration.registered = false
	registration.callback = menuCallback{}
}

func dispatchShortcut(id uint32) {
	if registration := shortcutRegistrations[id]; registration != nil && registration.registered && !registration.unregistering {
		registration.callback.run()
	}
}
