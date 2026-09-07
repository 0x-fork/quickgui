package native

type CrashParameter struct {
	Key   string `json:"key"`
	Value string `json:"value"`
}

type CrashReporterOptions struct {
	AppName        string           `json:"appName"`
	AppVersion     string           `json:"appVersion"`
	AppIdentifier  string           `json:"appIdentifier"`
	Directory      string           `json:"directory,omitempty"`
	MaxReports     uint32           `json:"maxReports,omitempty"`
	MaxReportBytes uint64           `json:"maxReportBytes,omitempty"`
	Parameters     []CrashParameter `json:"parameters,omitempty"`
	UploadEndpoint string           `json:"uploadEndpoint,omitempty"`
	Backtrace      string           `json:"backtrace,omitempty"`
	CaptureSignals *bool            `json:"captureSignals,omitempty"`
}

type CrashLocation struct {
	File   string `json:"file"`
	Line   uint32 `json:"line"`
	Column uint32 `json:"column"`
}

type CrashReport struct {
	SchemaVersion          uint32           `json:"schemaVersion"`
	ID                     string           `json:"id"`
	Kind                   string           `json:"kind"`
	Timestamp              string           `json:"timestamp"`
	AppName                string           `json:"appName"`
	AppVersion             string           `json:"appVersion"`
	AppIdentifier          string           `json:"appIdentifier"`
	OperatingSystem        string           `json:"operatingSystem"`
	OperatingSystemVersion string           `json:"operatingSystemVersion,omitempty"`
	Architecture           string           `json:"architecture"`
	ProcessID              uint32           `json:"processId"`
	Thread                 string           `json:"thread,omitempty"`
	Message                string           `json:"message"`
	Location               *CrashLocation   `json:"location,omitempty"`
	Backtrace              string           `json:"backtrace,omitempty"`
	Signal                 *int             `json:"signal,omitempty"`
	SignalName             string           `json:"signalName,omitempty"`
	FaultAddress           string           `json:"faultAddress,omitempty"`
	Parameters             []CrashParameter `json:"parameters"`
}

type CrashUploadSummary struct {
	Attempted uint32 `json:"attempted"`
	Uploaded  uint32 `json:"uploaded"`
	Failed    uint32 `json:"failed"`
}

type crashReporterAPI struct{}

// CrashReporter exposes the core's opt-in crash capture and report store.
// Reports are uploaded only when UploadPending is called.
var CrashReporter crashReporterAPI

func (crashReporterAPI) IsStarted() (bool, error) {
	return callJSON[bool]("is-crash-reporter-started", nil)
}

func (crashReporterAPI) Start(options CrashReporterOptions, done func(string, error)) {
	invokeJSON("start-crash-reporter", options, done)
}

func (crashReporterAPI) GetLastCrashReport(done func(*CrashReport, error)) {
	invokeJSON("get-last-crash-report", struct{}{}, func(reports []CrashReport, err error) {
		if done == nil {
			return
		}
		if len(reports) == 0 {
			done(nil, err)
			return
		}
		done(&reports[0], err)
	})
}

func (crashReporterAPI) GetPendingReports(done func([]CrashReport, error)) {
	invokeJSON("get-pending-crash-reports", struct{}{}, done)
}

func (crashReporterAPI) AddExtraParameter(key, value string, done func(bool, error)) {
	invokeJSON("add-crash-extra-parameter", map[string]string{"key": key, "value": value}, done)
}

func (crashReporterAPI) RemoveExtraParameter(key string, done func(bool, error)) {
	invokeJSON("remove-crash-extra-parameter", map[string]string{"key": key}, done)
}

func (crashReporterAPI) DeleteReport(id string, done func(bool, error)) {
	invokeJSON("delete-crash-report", map[string]string{"id": id}, done)
}

func (crashReporterAPI) UploadPending(endpoint string, done func(CrashUploadSummary, error)) {
	invokeJSON("upload-pending-crash-reports", struct {
		Endpoint string `json:"endpoint,omitempty"`
	}{endpoint}, done)
}
