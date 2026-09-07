// Package echo wraps an independently versioned native library. It imports no
// optional QuickGUI backend and needs no provider-specific changes in the core.
package echo

import (
	"encoding/json"

	"github.com/egoist/quickgui/go/host"
	"github.com/egoist/quickgui/go/native"
)

func init() { host.RequireExtension("acme-echo", "1.0.0") }

func Send(message string, done func(string, error)) {
	native.InvokeExtension(
		"acme-echo",
		"echo",
		message,
		func(raw string, err error) {
			var reply string
			if err == nil {
				err = json.Unmarshal([]byte(raw), &reply)
			}
			done(reply, err)
		},
	)
}
