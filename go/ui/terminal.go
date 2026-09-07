package ui

import (
	"encoding/json"
	"fmt"

	"github.com/egoist/quickgui/go/native"
	"github.com/egoist/quickgui/go/protocol"
	"github.com/egoist/quickgui/go/reactive"
)

// TerminalPalette contains the sixteen ANSI colors in normal, then bright order.
type TerminalPalette = [16]string

// TerminalStatusDetails is a snapshot reported by the native PTY. Agent fields
// describe a detected foreground process, not the program requested at launch.
type TerminalStatusDetails struct {
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

func TerminalStatusFromEvent(event *native.Event) *TerminalStatusDetails {
	if event == nil || event.Type != protocol.EventTerminal {
		return nil
	}
	return decodeEventJSON[TerminalStatusDetails](event)
}

// TerminalProps configure the Rust core's retained PTY terminal.
type TerminalProps struct {
	Props
	Program          string
	Args             []string
	WorkingDirectory string
	Scrollback       int
	Environment      map[string]string
	Palette          any // TerminalPalette or an accessor returning one.
	CursorColor      any
	PaddingColor     string
	FontThicken      bool
	OnStatus         func(*native.Event)
}

func Terminal(props TerminalProps, children ...any) *native.Node {
	props.Children = withChildren(props.Children, children)

	node := native.CreateElement(protocol.TagTerminal)
	applyProps(node, props.Props)
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
			var palette TerminalPalette
			switch value := props.Palette.(type) {
			case TerminalPalette:
				palette = value
			case func() TerminalPalette:
				palette = value()
			case reactive.Accessor[TerminalPalette]:
				palette = value()
			default:
				panic(fmt.Sprintf("QuickGUI expected a TerminalPalette or accessor, got %T", value))
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
		setString(node, protocol.TerminalPaddingColor, props.PaddingColor)
	}
	if props.FontThicken {
		native.SetBoolean(node, protocol.TerminalFontThicken, true)
	}
	if props.OnStatus != nil {
		setListener(node, protocol.EventTerminal, props.OnStatus)
	}
	return node
}
