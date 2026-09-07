package ui

import (
	"encoding/json"
	"fmt"
	"reflect"
	"strconv"
	"strings"
	"time"
	"unicode"

	"github.com/egoist/quickgui/go/native"
	"github.com/egoist/quickgui/go/protocol"
)

const (
	maxStyleDeclarationBytes = 4096
	maxTooltipTextBytes      = 1024
	maxKeymapJSONBytes       = 64 * 1024
	maxDragJSONBytes         = 64 * 1024
)

// GradientStop positions a color from 0 to 1. Omitted positions are distributed
// by the core; at most eight stops are retained.
type GradientStop struct {
	Color    string   `json:"color"`
	Position *float64 `json:"position,omitempty"`
}

type GradientCenter struct {
	X float64 `json:"x"`
	Y float64 `json:"y"`
}

// GradientDeclaration is the structured alternative to CSS gradient syntax.
type GradientDeclaration struct {
	Type          string          `json:"type"`
	Angle         *float64        `json:"angle,omitempty"`
	FromAngle     *float64        `json:"fromAngle,omitempty"`
	Shape         string          `json:"shape,omitempty"`
	Extent        string          `json:"extent,omitempty"`
	Center        *GradientCenter `json:"center,omitempty"`
	Interpolation string          `json:"interpolation,omitempty"`
	Stops         []GradientStop  `json:"stops"`
}

// TransitionDeclaration describes a paint transition. Duration accepts a number
// of milliseconds, time.Duration, or CSS time such as "120ms" or "0.2s".
type TransitionDeclaration struct {
	Properties []string
	Duration   any
	Easing     string
	MaxFps     any
}

type TransformMatrix struct {
	A  float64 `json:"a"`
	B  float64 `json:"b"`
	C  float64 `json:"c"`
	D  float64 `json:"d"`
	TX float64 `json:"tx"`
	TY float64 `json:"ty"`
}

type TextShadowDeclaration struct {
	OffsetX float64 `json:"offsetX"`
	OffsetY float64 `json:"offsetY"`
	Blur    float64 `json:"blur,omitempty"`
	Color   string  `json:"color,omitempty"`
}

// BoxShadowDeclaration is one shadow. An omitted Color uses the text color.
type BoxShadowDeclaration struct {
	OffsetX      float64
	OffsetY      float64
	BlurRadius   float64
	SpreadRadius float64
	Color        any
	Inset        bool
}

// Structured declarations may use an accessor of their exact Go type. This
// reflection is limited to resolving that accessor, once per declaration update;
// native layout, interaction, and painting remain in the core.
func resolveDeclaration(value any) any {
	if value == nil {
		return nil
	}
	if read, ok := styleAccessor(value); ok {
		return resolveDeclaration(read())
	}
	v := reflect.ValueOf(value)
	if v.Kind() == reflect.Func {
		if v.IsNil() {
			return nil
		}
		if v.Type().NumIn() != 0 || v.Type().NumOut() != 1 {
			panic("QuickGUI declaration accessors must take no arguments and return one value")
		}
		return resolveDeclaration(v.Call(nil)[0].Interface())
	}
	if v.Kind() == reflect.Pointer {
		if v.IsNil() {
			return nil
		}
		return v.Elem().Interface()
	}
	return value
}

func boundedDeclaration(text string) string {
	if len(text) > maxStyleDeclarationBytes {
		panic("QuickGUI style declarations are bounded to 4096 bytes")
	}
	return text
}

func declarationText(value any) string {
	switch value := value.(type) {
	case nil:
		return ""
	case string:
		return boundedDeclaration(strings.TrimSpace(value))
	case []string:
		return boundedDeclaration(strings.Join(value, " "))
	default:
		data, err := json.Marshal(value)
		if err != nil {
			panic(err)
		}
		return boundedDeclaration(string(data))
	}
}

func setDeclaration(node *native.Node, code uint16, value any) {
	setString(node, code, declarationText(value))
}

func encodeBackground(value any) (*uint32, string) {
	if value == nil || value == "" {
		return nil, ""
	}
	switch value := value.(type) {
	case string:
		if strings.Contains(value, "gradient(") {
			return nil, boundedDeclaration(strings.TrimSpace(value))
		}
	case GradientDeclaration:
		return nil, declarationText(value)
	}
	color := native.ParseColor(value)
	return &color, ""
}

func setBackground(node *native.Node, value any) {
	color, gradient := encodeBackground(value)
	if color == nil {
		native.ClearProperty(node, protocol.BackgroundColor)
	} else {
		native.SetColor(node, protocol.BackgroundColor, *color)
	}
	setString(node, protocol.BackgroundGradient, gradient)
}

// CSS lists and function tokens must keep commas and spaces inside rgb(),
// matrix(), and similar functions intact.
func splitCSS(value string, list bool) []string {
	var result []string
	start, depth := 0, 0
	for index, char := range value {
		switch char {
		case '(':
			depth++
		case ')':
			depth--
		}
		if depth < 0 {
			panic("QuickGUI CSS declaration has unbalanced parentheses")
		}
		if depth == 0 && ((list && char == ',') || (!list && unicode.IsSpace(char))) {
			if part := strings.TrimSpace(value[start:index]); part != "" {
				result = append(result, part)
			}
			start = index + len(string(char))
		}
	}
	if depth != 0 {
		panic("QuickGUI CSS declaration has unbalanced parentheses")
	}
	if part := strings.TrimSpace(value[start:]); part != "" {
		result = append(result, part)
	}
	return result
}

type encodedBoxShadow struct {
	OffsetX      float64 `json:"offsetX"`
	OffsetY      float64 `json:"offsetY"`
	BlurRadius   float64 `json:"blurRadius"`
	SpreadRadius float64 `json:"spreadRadius"`
	Color        *uint32 `json:"color"`
	Inset        bool    `json:"inset"`
}

func encodeBoxShadows(value any) []encodedBoxShadow {
	declarations := []BoxShadowDeclaration{}
	switch value := value.(type) {
	case nil:
	case BoxShadowDeclaration:
		declarations = append(declarations, value)
	case []BoxShadowDeclaration:
		declarations = value
	case string:
		text := strings.TrimSpace(value)
		if text == "" || strings.EqualFold(text, "none") {
			break
		}
		for _, part := range splitCSS(boundedDeclaration(text), true) {
			shadow := BoxShadowDeclaration{}
			var lengths []float64
			for _, token := range splitCSS(part, false) {
				if token == "inset" {
					if shadow.Inset {
						panic("QuickGUI box shadow repeats inset")
					}
					shadow.Inset = true
				} else if number, ok := normalizeLength(token).(float64); ok {
					lengths = append(lengths, number)
				} else {
					if shadow.Color != nil {
						panic("QuickGUI box shadow accepts one color per shadow")
					}
					shadow.Color = token
				}
			}
			if len(lengths) < 2 || len(lengths) > 4 {
				panic("QuickGUI box shadow needs two to four lengths")
			}
			shadow.OffsetX, shadow.OffsetY = lengths[0], lengths[1]
			if len(lengths) > 2 {
				shadow.BlurRadius = lengths[2]
			}
			if len(lengths) > 3 {
				shadow.SpreadRadius = lengths[3]
			}
			declarations = append(declarations, shadow)
		}
	default:
		panic(fmt.Sprintf("QuickGUI box shadow must be CSS or BoxShadowDeclaration, got %T", value))
	}
	if len(declarations) > 8 {
		panic("QuickGUI supports at most eight box shadows")
	}
	encoded := make([]encodedBoxShadow, 0, len(declarations))
	for _, shadow := range declarations {
		for _, number := range []float64{shadow.OffsetX, shadow.OffsetY, shadow.BlurRadius, shadow.SpreadRadius} {
			if !isFinite(number) {
				panic("QuickGUI box shadow lengths must be finite")
			}
		}
		var color *uint32
		if shadow.Color != nil && shadow.Color != "currentColor" && shadow.Color != "currentcolor" {
			packed := native.ParseColor(shadow.Color)
			color = &packed
		}
		encoded = append(encoded, encodedBoxShadow{shadow.OffsetX, shadow.OffsetY, shadow.BlurRadius, shadow.SpreadRadius, color, shadow.Inset})
	}
	return encoded
}

func setBoxShadow(node *native.Node, value any) {
	if value == nil {
		native.ClearProperty(node, protocol.BoxShadow)
		return
	}
	setJson(node, protocol.BoxShadow, protocol.MaxStateStyleJSONBytes, encodeBoxShadows(value))
}

func durationMilliseconds(value any) (float64, bool) {
	if duration, ok := value.(time.Duration); ok {
		return float64(duration) / float64(time.Millisecond), true
	}
	if text, ok := value.(string); ok {
		text = strings.TrimSpace(text)
		multiplier := 1.0
		if strings.HasSuffix(text, "ms") {
			text = strings.TrimSuffix(text, "ms")
		} else if strings.HasSuffix(text, "s") {
			text = strings.TrimSuffix(text, "s")
			multiplier = 1000
		}
		number, err := strconv.ParseFloat(text, 64)
		return number * multiplier, err == nil && isFinite(number*multiplier)
	}
	number := toFloat(value)
	return number, isFinite(number)
}

func setTransition(node *native.Node, value any) {
	for _, code := range []uint16{protocol.Transition, protocol.TransitionProperties, protocol.TransitionDuration, protocol.TransitionEasing, protocol.TransitionMaxFps} {
		native.ClearProperty(node, code)
	}
	if value == nil {
		return
	}
	var transition TransitionDeclaration
	switch value := value.(type) {
	case TransitionDeclaration:
		transition = value
	case string:
		if strings.TrimSpace(value) == "none" || strings.TrimSpace(value) == "" {
			return
		}
		for _, part := range splitCSS(boundedDeclaration(value), true) {
			for _, token := range splitCSS(part, false) {
				if duration, ok := durationMilliseconds(token); ok {
					transition.Duration = duration
				} else if isTransitionEasing(token) {
					transition.Easing = token
				} else {
					transition.Properties = append(transition.Properties, token)
				}
			}
		}
	default:
		transition.Duration = value
	}
	if transition.Duration != nil {
		setMilliseconds(node, protocol.TransitionDuration, transition.Duration)
	}
	if len(transition.Properties) != 0 {
		setTransitionProperties(node, transition.Properties)
	}
	if transition.Easing != "" {
		setTransitionEasing(node, transition.Easing)
	}
	if transition.MaxFps != nil {
		setNumber(node, protocol.TransitionMaxFps, transition.MaxFps)
	}
}

func setTransitionProperties(node *native.Node, value any) {
	if value == nil {
		native.ClearProperty(node, protocol.TransitionProperties)
		return
	}
	var names []string
	switch value := value.(type) {
	case string:
		names = strings.Split(value, ",")
	case []string:
		names = append([]string{}, value...)
	default:
		panic("QuickGUI transition properties must be a string or []string")
	}
	for i, name := range names {
		name = strings.TrimSpace(name)
		switch name {
		case "background":
			name = "background-color"
		case "all", "background-color", "border-color", "border-width", "border-radius", "color", "box-shadow", "opacity", "transform":
		default:
			panic("QuickGUI cannot transition " + name)
		}
		names[i] = name
	}
	setString(node, protocol.TransitionProperties, strings.Join(names, ","))
}

func isTransitionEasing(name string) bool {
	switch name {
	case "linear", "ease", "ease-in", "ease-out", "ease-in-out":
		return true
	}
	return false
}

func setTransitionEasing(node *native.Node, value any) {
	if value == nil {
		native.ClearProperty(node, protocol.TransitionEasing)
		return
	}
	name, ok := value.(string)
	if !ok || !isTransitionEasing(name) {
		panic(fmt.Sprintf("QuickGUI does not expose the %v easing curve", value))
	}
	if name == "ease" {
		name = "ease-in-out"
	}
	setString(node, protocol.TransitionEasing, name)
}

func setGridTemplate(node *native.Node, code uint16, value any) {
	switch value := value.(type) {
	case nil:
		native.ClearProperty(node, code)
	case string:
		setString(node, code, value)
	case []string:
		setString(node, code, strings.Join(value, " "))
	case []any:
		tracks := make([]string, len(value))
		for i, track := range value {
			if text, ok := track.(string); ok {
				tracks[i] = text
			} else {
				tracks[i] = fmt.Sprintf("%gpx", toFloat(track))
			}
		}
		setString(node, code, strings.Join(tracks, " "))
	default:
		setNumber(node, code, value)
	}
}

func setGridPlacement(node *native.Node, column bool, value any) {
	start, end, span := protocol.GridRowStart, protocol.GridRowEnd, protocol.GridRowSpan
	if column {
		start, end, span = protocol.GridColumnStart, protocol.GridColumnEnd, protocol.GridColumnSpan
	}
	for _, code := range []uint16{start, end, span} {
		native.ClearProperty(node, code)
	}
	if value == nil {
		return
	}
	text, ok := value.(string)
	if !ok {
		setNumber(node, start, value)
		return
	}
	parts := strings.Split(text, "/")
	if len(parts) > 2 {
		panic("QuickGUI grid placement accepts a start and end separated by /")
	}
	var startLine *float64
	for i, part := range parts {
		part = strings.TrimSpace(part)
		if part == "" || part == "auto" {
			continue
		}
		isSpan := strings.HasPrefix(part, "span ")
		part = strings.TrimSpace(strings.TrimPrefix(part, "span "))
		number, err := strconv.ParseFloat(part, 64)
		if err != nil || !isFinite(number) {
			panic("QuickGUI grid placement requires a line number or span")
		}
		if isSpan {
			if i == 1 && startLine != nil {
				setNumber(node, end, *startLine+number)
			} else {
				setNumber(node, span, number)
			}
		} else if i == 0 {
			startLine = &number
			setNumber(node, start, number)
		} else {
			setNumber(node, end, number)
		}
	}
}

func outlineText(value any) string {
	if text, ok := value.(string); ok {
		return boundedDeclaration(text)
	}
	if value == nil {
		return ""
	}
	return fmt.Sprintf("%gpx", toFloat(value))
}

func setOutline(node *native.Node, value any) {
	for _, code := range []uint16{protocol.OutlineWidth, protocol.OutlineColor, protocol.OutlineStyle} {
		native.ClearProperty(node, code)
	}
	if value == nil {
		return
	}
	for _, token := range splitCSS(outlineText(value), false) {
		switch token {
		case "solid", "dashed", "dotted", "none":
			setString(node, protocol.OutlineStyle, token)
		default:
			if number, ok := normalizeLength(token).(float64); ok {
				setNumber(node, protocol.OutlineWidth, number)
			} else {
				setColor(node, protocol.OutlineColor, token)
			}
		}
	}
}

func applyTextDecoration(node *native.Node, value string) {
	var lines []string
	for _, token := range splitCSS(value, false) {
		switch token {
		case "none", "underline", "overline", "line-through":
			lines = append(lines, token)
		case "solid", "double", "wavy":
			setString(node, protocol.TextDecorationStyle, token)
		default:
			if number, ok := normalizeLength(token).(float64); ok {
				setNumber(node, protocol.TextDecorationThickness, number)
			} else {
				setColor(node, protocol.TextDecorationColor, token)
			}
		}
	}
	setString(node, protocol.TextDecorationLine, strings.Join(lines, " "))
}
