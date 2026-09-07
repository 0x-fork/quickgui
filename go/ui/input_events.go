package ui

import (
	"github.com/egoist/quickgui/go/native"
	"github.com/egoist/quickgui/go/protocol"
)

type InputModifiers struct {
	Shift   bool `json:"shift"`
	Control bool `json:"control"`
	Alt     bool `json:"alt"`
	Meta    bool `json:"meta"`
}

type KeyEventDetails struct {
	InputModifiers
	Key    string `json:"key"`
	Text   string `json:"text,omitempty"`
	Repeat bool   `json:"repeat,omitempty"`
}

type MouseEventDetails struct {
	InputModifiers
	X             float64 `json:"x"`
	Y             float64 `json:"y"`
	Button        string  `json:"button,omitempty"`
	PressedButton string  `json:"pressedButton,omitempty"`
	ClickCount    int     `json:"clickCount,omitempty"`
	FirstMouse    bool    `json:"firstMouse,omitempty"`
}

type WheelEventDetails struct {
	InputModifiers
	X       float64 `json:"x"`
	Y       float64 `json:"y"`
	DeltaX  float64 `json:"deltaX"`
	DeltaY  float64 `json:"deltaY"`
	Precise bool    `json:"precise"`
	Phase   string  `json:"phase"`
}

type GestureEventDetails struct {
	InputModifiers
	X        float64  `json:"x"`
	Y        float64  `json:"y"`
	Delta    *float64 `json:"delta,omitempty"`
	Phase    string   `json:"phase,omitempty"`
	Pressure *float64 `json:"pressure,omitempty"`
	Stage    string   `json:"stage,omitempty"`
}

type DropEventDetails struct {
	InputModifiers
	X      float64  `json:"x"`
	Y      float64  `json:"y"`
	ID     string   `json:"id,omitempty"`
	Source uint32   `json:"source,omitempty"`
	Paths  []string `json:"paths,omitempty"`
	Origin string   `json:"origin,omitempty"`
}

func KeyFromEvent(event *native.Event) *KeyEventDetails {
	if event == nil || (event.Type != protocol.EventKeyDown && event.Type != protocol.EventKeyUp) {
		return nil
	}
	return decodeEventJSON[KeyEventDetails](event)
}

func MouseFromEvent(event *native.Event) *MouseEventDetails {
	if event == nil {
		return nil
	}
	switch event.Type {
	case protocol.EventMouseDown, protocol.EventMouseUp, protocol.EventMouseMove,
		protocol.EventMouseEnter, protocol.EventMouseLeave, protocol.EventDoubleClick, protocol.EventContextMenu:
		return decodeEventJSON[MouseEventDetails](event)
	}
	return nil
}

func WheelFromEvent(event *native.Event) *WheelEventDetails {
	if event == nil || event.Type != protocol.EventWheel {
		return nil
	}
	return decodeEventJSON[WheelEventDetails](event)
}

func GestureFromEvent(event *native.Event) *GestureEventDetails {
	if event == nil {
		return nil
	}
	switch event.Type {
	case protocol.EventPinch, protocol.EventRotate, protocol.EventSmartMagnify, protocol.EventPressure:
		return decodeEventJSON[GestureEventDetails](event)
	}
	return nil
}

func DropFromEvent(event *native.Event) *DropEventDetails {
	if event == nil {
		return nil
	}
	switch event.Type {
	case protocol.EventDragStart, protocol.EventDragEnd, protocol.EventDrop, protocol.EventFilesDropped:
		return decodeEventJSON[DropEventDetails](event)
	}
	return nil
}
