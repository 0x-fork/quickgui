package native

import "fmt"

type NotificationPermissionStatus string

const (
	NotificationPermissionNotDetermined NotificationPermissionStatus = "not-determined"
	NotificationPermissionGranted       NotificationPermissionStatus = "granted"
	NotificationPermissionDenied        NotificationPermissionStatus = "denied"
	NotificationPermissionUnsupported   NotificationPermissionStatus = "unsupported"
)

type NotificationResponseEvent struct {
	Tag      string  `json:"tag"`
	ActionID *string `json:"actionId"`
	Reply    *string `json:"reply"`
}

type notificationsAPI struct{}

var Notifications notificationsAPI
var notificationListeners subscriptions[NotificationResponseEvent]

func (notificationsAPI) Show(options NotificationOptions, done func(error)) {
	commandVoid(map[string]any{"method": "show-notification", "options": options}, done)
}

func (notificationsAPI) Dismiss(tag string, done func(error)) {
	commandVoid(map[string]any{"method": "dismiss-notification", "tag": tag}, done)
}

func (notificationsAPI) GetPermissionStatus(done func(NotificationPermissionStatus, error)) {
	notificationPermission(false, done)
}

func (notificationsAPI) RequestPermission(done func(NotificationPermissionStatus, error)) {
	notificationPermission(true, done)
}

func (notificationsAPI) OnResponse(listener func(NotificationResponseEvent)) func() {
	return notificationListeners.add(listener)
}

func notificationPermission(prompt bool, done func(NotificationPermissionStatus, error)) {
	SendCommand(mustString(map[string]any{"method": "notification-permission", "prompt": prompt}), func(value string, err error) {
		status := NotificationPermissionStatus(value)
		if err == nil {
			switch status {
			case NotificationPermissionNotDetermined, NotificationPermissionGranted, NotificationPermissionDenied, NotificationPermissionUnsupported:
			default:
				err = fmt.Errorf("invalid native notification permission response %q", value)
			}
		}
		if done != nil {
			done(status, err)
		}
	})
}
