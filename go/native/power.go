package native

type powerMonitorAPI struct{}

var PowerMonitor powerMonitorAPI
var powerListeners subscriptions[PowerEvent]

func (powerMonitorAPI) GetState() (PowerState, error) {
	return callJSON[PowerState]("get-power-state", nil)
}

func (powerMonitorAPI) GetSystemIdleTime() (float64, error) {
	return callJSON[float64]("get-system-idle-time", nil)
}

func (powerMonitorAPI) GetSystemIdleState(thresholdSeconds float64) (string, error) {
	return callJSON[string]("get-system-idle-state", map[string]float64{"thresholdSeconds": thresholdSeconds})
}

func (powerMonitorAPI) GetSessionState() (string, error) {
	return callJSON[string]("get-session-state", nil)
}

func (powerMonitorAPI) OnEvent(listener func(PowerEvent)) func() {
	return powerListeners.add(listener)
}

type PermissionKind string
type PermissionStatus string

const (
	PermissionCamera          PermissionKind   = "camera"
	PermissionMicrophone      PermissionKind   = "microphone"
	PermissionScreenRecording PermissionKind   = "screen-recording"
	PermissionAccessibility   PermissionKind   = "accessibility"
	PermissionNotDetermined   PermissionStatus = "not-determined"
	PermissionGranted         PermissionStatus = "granted"
	PermissionDenied          PermissionStatus = "denied"
	PermissionRestricted      PermissionStatus = "restricted"
	PermissionUnknown         PermissionStatus = "unknown"
)

type permissionsAPI struct{}

var Permissions permissionsAPI

func (permissionsAPI) Status(kind PermissionKind) (PermissionStatus, error) {
	return callJSON[PermissionStatus]("get-permission-status", map[string]PermissionKind{"kind": kind})
}

func (permissionsAPI) Request(kind PermissionKind, done func(PermissionStatus, error)) {
	invokeJSON("request-permission", map[string]PermissionKind{"kind": kind}, done)
}

// PowerAssertion prevents suspension or display sleep until explicitly released.
// Use it on the application goroutine, which owns the native handle registry.
type PowerAssertion struct {
	id     uint32
	Kind   string
	Reason string
}

func NewPowerAssertion(kind, reason string) (*PowerAssertion, error) {
	id, err := callJSON[uint32]("power-assertion-acquire", map[string]string{"kind": kind, "reason": reason})
	if err != nil {
		return nil, err
	}
	return &PowerAssertion{id: id, Kind: kind, Reason: reason}, nil
}

func (assertion *PowerAssertion) Active() (bool, error) {
	if assertion == nil || assertion.id == 0 {
		return false, nil
	}
	info, err := callJSON[struct{ Active bool }]("power-assertion-info", map[string]uint32{"id": assertion.id})
	return info.Active, err
}

func (assertion *PowerAssertion) Release() (bool, error) {
	if assertion == nil || assertion.id == 0 {
		return false, nil
	}
	released, err := callJSON[bool]("power-assertion-release", map[string]uint32{"id": assertion.id})
	if err == nil {
		assertion.id = 0
	}
	return released, err
}
