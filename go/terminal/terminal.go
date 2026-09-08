package terminal

import (
	"encoding/json"
	"fmt"
	"reflect"

	"github.com/egoist/quickgui/go/host"
	"github.com/egoist/quickgui/go/native"
	"github.com/egoist/quickgui/go/protocol"
	"github.com/egoist/quickgui/go/reactive"
	"github.com/egoist/quickgui/go/ui"
)

// Palette contains the sixteen ANSI colors in normal, then bright order.
type Palette = [16]string

// StatusDetails is a snapshot reported by the native PTY. Agent fields
// describe a detected foreground process, not the program requested at launch.
type StatusDetails struct {
	Status           string `json:"status"`
	Title            string `json:"title"`
	WorkingDirectory string `json:"workingDirectory"`
	ProcessID        int    `json:"processId,omitempty"`
	ExitCode         *int   `json:"exitCode,omitempty"`
	Signal           string `json:"signal,omitempty"`
	Message          string `json:"message,omitempty"`
	Agent            string `json:"agent,omitempty"`
	AgentStatus      string `json:"agentStatus,omitempty"`
	AgentProcessID   int    `json:"agentProcessId,omitempty"`
}

func StatusFromEvent(event *native.Event) *StatusDetails {
	if event == nil || event.Type != protocol.EventTerminal {
		return nil
	}
	value, ok := event.ValueOK()
	if !ok {
		return nil
	}
	var status *StatusDetails
	if json.Unmarshal([]byte(value), &status) != nil {
		return nil
	}
	return status
}

// Props configure the retained terminal view and its optional PTY backend.
type Props struct {
	ui.Props
	Program          string
	Args             []string
	WorkingDirectory string
	Scrollback       int
	Environment      map[string]string
	Palette          any // Palette or an accessor returning one.
	CursorColor      any
	PaddingColor     string
	FontThicken      bool
	OnStatus         func(*native.Event)
}

func View(props Props, children ...any) *native.Node {
	if props.OnStatus != nil {
		props.Props.OnStatus = props.OnStatus
	}
	arguments := []any{props.Props}
	arguments = append(arguments, children...)
	node := ui.NativeElement(protocol.TagTerminal, arguments...).Node
	if props.Program != "" {
		native.SetString(node, protocol.TerminalProgram, props.Program)
	}
	if props.Args != nil {
		encoded, err := json.Marshal(props.Args)
		if err != nil {
			panic(err)
		}
		native.SetString(node, protocol.TerminalArguments, string(encoded))
	}
	if props.WorkingDirectory != "" {
		native.SetString(node, protocol.TerminalWorkingDirectory, props.WorkingDirectory)
	}
	if props.Scrollback > 0 {
		native.SetNumber(node, protocol.TerminalScrollback, float32(props.Scrollback))
	}
	if props.Environment != nil {
		setJson(node, protocol.TerminalEnvironment, 65536, props.Environment)
	}
	if props.Palette != nil {
		node.Bind(func() {
			var palette Palette
			switch value := props.Palette.(type) {
			case Palette:
				palette = value
			case func() Palette:
				palette = value()
			case reactive.Accessor[Palette]:
				palette = value()
			default:
				panic(fmt.Sprintf("QuickGUI expected a Palette or accessor, got %T", value))
			}
			var packed [16]uint32
			for i, color := range palette {
				packed[i] = native.ParseColor(color)
			}
			setJson(node, protocol.TerminalPalette, 4096, packed)
		})
	}
	if props.CursorColor != nil {
		setColor(node, protocol.TerminalCursorColor, props.CursorColor)
	}
	if props.PaddingColor != "" {
		native.SetString(node, protocol.TerminalPaddingColor, props.PaddingColor)
	}
	if props.FontThicken {
		native.SetBoolean(node, protocol.TerminalFontThicken, true)
	}
	return node
}

// Importing this package opts the application into the prebuilt terminal backend.
func init() { host.RequireExtension("terminal") }

func setJson(node *native.Node, code uint16, limit int, value any) {
	data, err := json.Marshal(value)
	if err != nil {
		panic(err)
	}
	if len(data) > limit {
		panic("QuickGUI terminal configuration exceeds its byte limit")
	}
	native.SetString(node, code, string(data))
}

func setColor(node *native.Node, code uint16, value any) {
	node.Bind(func() {
		current := value
		if current != nil {
			accessor := reflect.ValueOf(current)
			if accessor.Kind() == reflect.Func && accessor.Type().NumIn() == 0 && accessor.Type().NumOut() == 1 {
				current = accessor.Call(nil)[0].Interface()
			}
		}
		if current == nil || current == "" {
			native.ClearProperty(node, code)
			return
		}
		native.SetColor(node, code, native.ParseColor(current))
	})
}
