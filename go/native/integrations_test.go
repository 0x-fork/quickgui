package native

import (
	"encoding/base64"
	"encoding/json"
	"errors"
	"reflect"
	"testing"

	"github.com/egoist/quickgui/go/host"
)

type serviceInvocation struct {
	id     uint32
	method string
	params string
}

type serviceHost struct {
	host.Fake
	invocations []serviceInvocation
	callMethods []string
}

func (h *serviceHost) Invoke(id uint32, method, params string) {
	h.invocations = append(h.invocations, serviceInvocation{id, method, params})
}

func (h *serviceHost) Call(method, _ string) string {
	h.callMethods = append(h.callMethods, method)
	switch method {
	case "cpu-sampler-create":
		return `{"ok":true,"value":42}`
	case "cpu-sampler-sample":
		return `{"ok":true,"value":{"percent":12.5,"intervalSeconds":1,"cpuSeconds":0.125,"totalCpuSeconds":4}}`
	default:
		return `{"ok":true,"value":true}`
	}
}

func installServiceHost(t *testing.T) *serviceHost {
	t.Helper()
	fake := &serviceHost{}
	restore := host.Install(fake)
	previousID, previousReady := appID, appReady
	setAppContext(1, true)
	t.Cleanup(func() {
		setAppContext(previousID, previousReady)
		restore()
	})
	return fake
}

func TestSecureStorageBinaryAndAbsentValuesUseTheNativeWireContract(t *testing.T) {
	fake := installServiceHost(t)
	secret := []byte{0, 1, 127, 128, 255}
	calls := 0
	SecureStorage.Set("test-service", "account", secret, func(stored bool, err error) {
		calls++
		if !stored || err != nil {
			t.Fatalf("store result = %v, %v", stored, err)
		}
	})
	request := fake.invocations[0]
	var params struct {
		Service string
		Account string
		Value   string
	}
	if err := json.Unmarshal([]byte(request.params), &params); err != nil {
		t.Fatal(err)
	}
	if request.method != "set-secure-storage" || params.Service != "test-service" || params.Value != base64.StdEncoding.EncodeToString(secret) || calls != 0 {
		t.Fatal("credential request was synchronous or did not preserve binary data")
	}
	App.dispatchHostEvent(hostEvent{kind: "invoke", target: request.id, flags: 1, value: "true"})
	for _, test := range []struct {
		json string
		want []byte
	}{{`"AA=="`, []byte{0}}, {`""`, []byte{}}, {`null`, nil}} {
		var received []byte
		SecureStorage.Get("test-service", "account", func(value []byte, err error) {
			if err != nil {
				t.Fatal(err)
			}
			received = value
		})
		request = fake.invocations[len(fake.invocations)-1]
		App.dispatchHostEvent(hostEvent{kind: "invoke", target: request.id, flags: 1, value: test.json})
		if !reflect.DeepEqual(received, test.want) {
			t.Fatalf("credential %s decoded as %#v", test.json, received)
		}
	}
	if calls != 1 {
		t.Fatal("credential write did not settle exactly once")
	}
}

func TestInvokeRejectsWithoutLeakingProgressOrPanickingOnNilCallbacks(t *testing.T) {
	fake := installServiceHost(t)
	calls := 0
	invokeWithProgress("stage-update", struct{}{}, func(string) { t.Fatal("unexpected progress") }, func(_ string, err error) {
		calls++
		if err == nil {
			t.Fatal("failed request lost its error")
		}
	})
	request := fake.invocations[0]
	rejectAllReplies(errors.New("application closed"))
	if calls != 1 || pendingProgress[request.id] != nil {
		t.Fatal("shutdown retained an update subscription")
	}
	Invoke("invalid-params", make(chan int), nil)
	setAppContext(0, false)
	Invoke("not-ready", nil, nil)
	if len(fake.invocations) != 1 {
		t.Fatal("invalid requests reached the host")
	}
}

func TestNativeIntegrationRepliesValidateJSONAndMissingReports(t *testing.T) {
	fake := installServiceHost(t)
	calls := 0
	Metrics.GetProcessMetrics(func(_ ProcessMetrics, err error) {
		calls++
		if err == nil {
			t.Fatal("malformed native JSON was accepted")
		}
	})
	App.dispatchHostEvent(hostEvent{kind: "invoke", target: fake.invocations[0].id, flags: 1, value: `invalid`})
	CrashReporter.GetLastCrashReport(func(report *CrashReport, err error) {
		calls++
		if report != nil || err != nil {
			t.Fatalf("missing report = %+v, %v", report, err)
		}
	})
	App.dispatchHostEvent(hostEvent{kind: "invoke", target: fake.invocations[1].id, flags: 1, value: `[]`})
	if calls != 2 {
		t.Fatal("requests did not complete")
	}
}

func TestCPUSamplerReleasesItsCoreHandleOnce(t *testing.T) {
	fake := installServiceHost(t)
	sampler, err := NewCPUSampler()
	if err != nil {
		t.Fatal(err)
	}
	sample, err := sampler.Sample()
	if err != nil || sample.Percent == nil || *sample.Percent != 12.5 {
		t.Fatalf("sample = %+v, %v", sample, err)
	}
	if err := sampler.Release(); err != nil {
		t.Fatal(err)
	}
	if err := sampler.Release(); err != nil {
		t.Fatal(err)
	}
	if _, err := sampler.Sample(); err == nil {
		t.Fatal("released sampler remained usable")
	}
	if !reflect.DeepEqual(fake.callMethods, []string{"cpu-sampler-create", "cpu-sampler-sample", "cpu-sampler-release"}) {
		t.Fatalf("sampler operations = %v", fake.callMethods)
	}
}

func TestLaunchURLsExcludeFlagsWebLinksAndPlatformPaths(t *testing.T) {
	got := URLsFromArguments([]string{"--inspect", "/tmp/file", `C:\Users\User`, "https://example.com", "HTTP://example.com", "quickgui://open/repo?path=%2Ftmp", "custom:action"})
	want := []string{"quickgui://open/repo?path=%2Ftmp", "custom:action"}
	if !reflect.DeepEqual(got, want) {
		t.Fatalf("launch URLs = %v", got)
	}
}
