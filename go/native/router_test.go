package native

import (
	"encoding/json"
	"testing"

	"github.com/egoist/quickgui/go/host"
)

func serviceOK(value any) string {
	payload, err := json.Marshal(struct {
		OK    bool `json:"ok"`
		Value any  `json:"value"`
	}{OK: true, Value: value})
	if err != nil {
		panic(err)
	}
	return string(payload)
}

func TestRouterCreateStateAndNavigate(t *testing.T) {
	state := RouterState{
		Location:      RouteLocation{Href: "/", Pathname: "/"},
		Matched:       &RouteMatch{RouteIDs: []string{"home"}},
		HistoryIndex:  0,
		HistoryLength: 1,
	}
	restore := host.Install(&host.Fake{CallFn: func(method, params string) string {
		switch method {
		case "router-create":
			var body struct {
				Routes             []RouteDefinition `json:"routes"`
				InitialDestination string            `json:"initialDestination"`
			}
			if err := json.Unmarshal([]byte(params), &body); err != nil {
				t.Fatal(err)
			}
			if body.InitialDestination != "/" || len(body.Routes) != 1 || body.Routes[0].ID != "home" {
				t.Fatalf("create %s", params)
			}
			return serviceOK(1)
		case "router-state":
			return serviceOK(state)
		case "router-push":
			state.Location = RouteLocation{Href: "/about", Pathname: "/about"}
			state.Matched = &RouteMatch{RouteIDs: []string{"about"}}
			state.HistoryIndex = 1
			state.HistoryLength = 2
			state.CanGoBack = true
			return serviceOK(state)
		case "router-is-active":
			var body struct {
				Destination string `json:"destination"`
				End         bool   `json:"end"`
			}
			_ = json.Unmarshal([]byte(params), &body)
			return serviceOK(body.Destination == state.Location.Pathname)
		case "router-resolve":
			return serviceOK(RouteLocation{Href: "/about", Pathname: "/about"})
		case "router-release":
			return serviceOK(nil)
		default:
			t.Fatalf("unexpected method %s", method)
			return serviceOK(nil)
		}
	}})
	defer restore()

	router := NewRouter([]RouteDefinition{{ID: "home", Path: "/"}}, "/")
	if router.ID != 1 {
		t.Fatal(router.ID)
	}
	got := router.State()
	if got.Location.Pathname != "/" || got.Matched == nil || got.Matched.RouteIDs[0] != "home" {
		t.Fatalf("%+v", got)
	}
	next := router.Push("/about")
	if next.Location.Pathname != "/about" || !next.CanGoBack {
		t.Fatalf("%+v", next)
	}
	if !router.IsActive("/about", true) || router.IsActive("/", true) {
		t.Fatal("active")
	}
	if router.Resolve("/about").Pathname != "/about" {
		t.Fatal("resolve")
	}
	router.Release()
	router.Release()
}

func TestRouterDistinguishesIndexRoutesFromPathlessLayouts(t *testing.T) {
	routes := []RouteDefinition{
		{ID: "shell"},
		{ID: "settings", Path: "/settings", ParentID: "shell"},
		{ID: "general", Index: true, ParentID: "settings"},
	}
	restore := host.Install(&host.Fake{CallFn: func(method, params string) string {
		if method == "router-release" {
			return serviceOK(nil)
		}
		if method != "router-create" {
			t.Fatalf("unexpected method %s", method)
		}
		var body struct {
			Routes []map[string]any `json:"routes"`
		}
		if err := json.Unmarshal([]byte(params), &body); err != nil {
			t.Fatal(err)
		}
		if len(body.Routes) != 3 {
			t.Fatalf("routes: %s", params)
		}
		if _, present := body.Routes[0]["path"]; present {
			t.Fatal("a pathless layout must omit path")
		}
		if body.Routes[1]["path"] != "/settings" || body.Routes[2]["path"] != "" {
			t.Fatalf("an index child must send an explicit empty path: %s", params)
		}
		return serviceOK(1)
	}})
	defer restore()
	router := NewRouter(routes, "/settings")
	router.Release()

	for _, route := range routes {
		data, err := json.Marshal(route)
		if err != nil {
			t.Fatal(err)
		}
		var decoded RouteDefinition
		if err := json.Unmarshal(data, &decoded); err != nil {
			t.Fatal(err)
		}
		if decoded != route {
			t.Fatalf("route changed after JSON roundtrip: %+v -> %+v", route, decoded)
		}
	}
}

func TestCallServiceUnwrapsValue(t *testing.T) {
	restore := host.Install(&host.Fake{CallFn: func(method, params string) string {
		if method != "echo" || params != `{"n":1}` {
			t.Fatalf("%s %s", method, params)
		}
		return `{"ok":true,"value":{"n":1}}`
	}})
	defer restore()
	raw, err := CallService("echo", `{"n":1}`)
	if err != nil || raw != `{"n":1}` {
		t.Fatalf("%s %v", raw, err)
	}
}

func TestCallServiceReportsHostError(t *testing.T) {
	restore := host.Install(&host.Fake{CallFn: func(string, string) string {
		return `{"ok":false,"error":"no such router"}`
	}})
	defer restore()
	_, err := CallService("router-state", `{"id":9}`)
	if err == nil || err.Error() != "no such router" {
		t.Fatal(err)
	}
}
