package ui

import (
	"github.com/egoist/quickgui/go/native"
)

//go:generate go run ../internal/cmd/optionsgen
//go:generate bun ../../scripts/generate-style-helpers.ts

// Option configures a conditional fluent declaration or a legacy constructor.
// Prefer Element methods for properties, events, and styles.
type Option interface {
	apply(*Props)
}

// Internal declarations preserve omitted fields and explicit zero values.
func (style styleData) apply(props *Props) { mergeStyle(&props.Style.style, style) }

// styleDeclaration is an internal option, style record, or fluent style.
type styleDeclaration interface {
	applyStyle(*styleData)
}

func (style styleData) applyStyle(target *styleData) { mergeStyle(target, style) }

// styleOption sets a style property. It can also be nested inside composeStyles, styleHover,
// styleActive, styleFocus, or another interaction style.
type styleOption func(*styleData)

func (option styleOption) apply(props *Props)          { option(&props.Style.style) }
func (option styleOption) applyStyle(style *styleData) { option(style) }

// composeStyles composes reusable style options in declaration order. Later values
// override the same property; interaction styles merge without mutating inputs.
// Accessors remain unevaluated until the style is bound to a node.
func composeStyles(options ...styleDeclaration) styleData {
	var style styleData
	for _, option := range options {
		option.applyStyle(&style)
	}
	return style
}

type propertyOption func(*Props)

func (option propertyOption) apply(props *Props) { option(props) }

// OnClick handles a click without requiring an unused event parameter.
func OnClick(handler func()) Option {
	var callback func(*native.Event)
	if handler != nil {
		callback = func(*native.Event) { handler() }
	}
	return OnClickEvent(callback)
}

type conditionalOption struct {
	condition func() bool
	options   []Option
}

// When applies options while condition is true. Signal reads in condition are
// tracked; turning it off restores earlier options or clears omitted properties.
func When(condition any, options ...Option) Option {
	return conditionalOption{condition: booleanRead(condition), options: options}
}

func (option conditionalOption) apply(props *Props) {
	if option.condition() {
		for _, child := range option.options {
			child.apply(props)
		}
	}
}

func clearOmittedListeners(node *native.Node, props Props) {
	for _, event := range primitiveListeners(props) {
		if event.handler == nil {
			for _, listener := range node.Listeners {
				if listener.Type == event.kind {
					native.SetEventListener(node, event.kind, nil)
					break
				}
			}
		}
	}
}

// OnClickEvent receives the native event, including propagation controls.
func OnClickEvent(handler func(*native.Event)) Option {
	return propertyOption(func(props *Props) { props.OnClick = handler })
}

func resolveProps(arguments []any) Props {
	var props Props
	var children []any
	for _, argument := range arguments {
		switch value := argument.(type) {
		case Option:
			value.apply(&props)
		case Props:
			props = value
			if value.Children != nil {
				children = append(children, value.Children)
			}
		default:
			children = append(children, argument)
		}
	}
	props.Children = children
	return props
}
