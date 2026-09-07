package ui

import (
	"github.com/egoist/quickgui/go/native"
)

//go:generate go run ../internal/cmd/optionsgen

// Option configures a primitive. Style records and When are both options.
type Option interface {
	apply(*Props)
}

// A Style can be passed directly to a primitive or When. Styles merge in order;
// omitted fields preserve earlier declarations, including explicit zero values.
func (style Style) apply(props *Props) { mergeStyle(&props.Style, style) }

// StyleDeclaration is a reusable style record or a compatibility style option.
// Group styles accept the same records as ordinary primitives.
type StyleDeclaration interface {
	applyStyle(*Style)
}

func (style Style) applyStyle(target *Style) { mergeStyle(target, style) }

// StyleOption can also be nested inside Hover, Active, or Focus.
type StyleOption func(*Style)

func (option StyleOption) apply(props *Props)      { option(&props.Style) }
func (option StyleOption) applyStyle(style *Style) { option(style) }

type propertyOption func(*Props)

func (option propertyOption) apply(props *Props) { option(props) }

// WithStyle reuses a style record among ordinary options.
func WithStyle(style Style) StyleOption {
	return func(target *Style) { mergeStyle(target, style) }
}

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
func When(condition func() bool, options ...Option) Option {
	return conditionalOption{condition: condition, options: options}
}

func (option conditionalOption) apply(props *Props) {
	if option.condition() {
		for _, child := range option.options {
			child.apply(props)
		}
	}
}

func applyArguments(node *native.Node, arguments []any) {
	dynamic := false
	for _, argument := range arguments {
		if _, ok := argument.(conditionalOption); ok {
			dynamic = true
			break
		}
	}
	if !dynamic {
		applyProps(node, resolveProps(arguments))
		return
	}
	var props Props
	node.BindProperties(func() {
		props = resolveProps(arguments)
		// Remove handlers that were declared only by an inactive condition.
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
		applyPropValues(node, props)
	})
	insertChildren(node, props.Children)
	if props.Ref != nil {
		props.Ref(node)
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
