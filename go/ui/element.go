package ui

import (
	"github.com/egoist/quickgui/go/native"
)

// Element is a retained native node with fluent property, event, and style
// declarations. Pass elements directly as children. Node exposes the underlying
// handle for low-level native APIs; fluent methods always return this element.
type Element struct {
	*native.Node
	props     *Props
	bindings  map[elementBinding]*native.PropertyBinding
	arguments []any
	update    func()
}

// NativeNode exposes this builder's retained node to windows and lazy components.
func (element *Element) NativeNode() *native.Node {
	if element == nil {
		return nil
	}
	return element.Node
}

func newElement(tag uint8, arguments []any) *Element {
	element := &Element{Node: native.CreateElement(tag)}
	configured := false
	for _, argument := range arguments {
		switch argument.(type) {
		case Option, Props:
			configured = true
		}
	}
	if !configured {
		insertChildren(element.Node, arguments)
		return element
	}
	var props Props
	if conditionalArguments(arguments) {
		element.arguments = append([]any(nil), arguments...)
		element.update = element.BindProperties(func() {
			props = resolveProps(element.arguments)
			clearOmittedListeners(element.Node, props)
			applyPropValues(element.Node, props)
		})
	} else {
		props = resolveProps(arguments)
		props.Style.style = normalizeStyleAliases(props.Style.style)
		element.props = &props
		if overlappingStyle(props.Style.style) {
			element.reconcile()
		} else {
			element.refreshFields(populatedElementBindings(props))
		}
	}
	insertChildren(element.Node, props.Children)
	if props.Ref != nil {
		props.Ref(element.Node)
	}
	if element.props != nil {
		element.props.Children = nil
		element.props.Ref = nil
		if len(element.bindings) == 0 && element.update == nil {
			element.props = nil
		}
	}
	return element
}

func (element *Element) configureFields(fields []elementBinding, options ...Option) *Element {
	if element.Removed {
		panic("a removed QuickGUI element cannot be configured")
	}
	if element.update != nil {
		for _, option := range options {
			element.arguments = append(element.arguments, option)
		}
		element.update()
		return element
	}
	if element.props == nil {
		element.props = &Props{}
	}
	for _, option := range options {
		option.apply(element.props)
	}
	if overlappingStyle(element.props.Style.style) {
		element.reconcile()
	} else {
		element.refreshFields(fields)
	}
	return element
}

// Style merges a reusable style, matching MoonBit's style modifier. Scalar
// accessors retain independent native bindings.
func (element *Element) Style(style StyleBuilder) *Element {
	return element.configureFields(populatedStyleBindings(style.style), style)
}

// Flex enables flex layout without arguments. With one argument it sets the
// numeric flex shorthand (grow, shrink 1, zero basis).
func (element *Element) Flex(value ...any) *Element {
	switch len(value) {
	case 0:
		return element.Display("flex")
	case 1:
		return element.configureStyles([]string{"FlexGrow", "FlexShrink", "FlexBasis"}, styleFlexGrow(value[0]), styleFlexShrink(1), styleFlexBasis(0))
	default:
		panic("QuickGUI Flex accepts zero or one value")
	}
}

// FlexWrap enables wrapping without arguments, or accepts an explicit mode.
func (element *Element) FlexWrap(value ...string) *Element {
	if len(value) == 0 {
		return element.configureStyle("FlexWrap", styleFlexWrap("wrap"))
	}
	if len(value) != 1 {
		panic("QuickGUI FlexWrap accepts zero or one value")
	}
	return element.configureStyle("FlexWrap", styleFlexWrap(value[0]))
}

func (element *Element) Bg(value any) *Element { return element.BackgroundColor(value) }

func (element *Element) GroupHover(build func(StyleBuilder) StyleBuilder) *Element {
	return element.configureStyle("GroupHover", styleGroupHover(build(Style())))
}

func (element *Element) GroupHoverNamed(name string, build func(StyleBuilder) StyleBuilder) *Element {
	return element.configureStyle("GroupHover", styleGroupHoverNamed(name, build(Style())))
}

func (element *Element) GroupActive(build func(StyleBuilder) StyleBuilder) *Element {
	return element.configureStyle("GroupActive", styleGroupActive(build(Style())))
}

func (element *Element) GroupActiveNamed(name string, build func(StyleBuilder) StyleBuilder) *Element {
	return element.configureStyle("GroupActive", styleGroupActiveNamed(name, build(Style())))
}

// When applies options while condition is true. Turning it off restores earlier
// declarations or clears omitted properties, without rebuilding children.
func (element *Element) When(condition any, options ...Option) *Element {
	element.reconcile()
	return element.configureFields(nil, When(condition, options...))
}

// OnClick handles a click without an unused event parameter.
func (element *Element) OnClick(handler func()) *Element {
	return element.configureProperty("OnClick", OnClick(handler))
}

// OnClickEvent receives the native event, including propagation controls.
func (element *Element) OnClickEvent(handler func(*native.Event)) *Element {
	return element.configureProperty("OnClick", OnClickEvent(handler))
}

// Ref exposes the native handle once at this point in the declaration chain.
func (element *Element) Ref(handler func(*native.Node)) *Element {
	if handler != nil {
		handler(element.Node)
	}
	return element
}

// OnInput receives the current text, matching MoonBit's on_input callback.
func (element *Element) OnInput(handler func(string)) *Element {
	return element.OnInputEvent(valueHandler(handler))
}

// OnInputEvent receives the full native input event.
func (element *Element) OnInputEvent(handler func(*native.Event)) *Element {
	return element.configureProperty("OnInput", OnInput(handler))
}

func (element *Element) OnSubmit(handler func(string)) *Element {
	return element.OnSubmitEvent(valueHandler(handler))
}

func (element *Element) OnSubmitEvent(handler func(*native.Event)) *Element {
	return element.configureProperty("OnSubmit", OnSubmit(handler))
}

func valueHandler(handler func(string)) func(*native.Event) {
	if handler == nil {
		return nil
	}
	return func(event *native.Event) { handler(event.Value) }
}

func (element *Element) OnChange(handler func(string)) *Element {
	return element.OnInput(handler)
}

func (element *Element) OnPointerEnter(handler func(*native.Event)) *Element {
	return element.OnMouseEnter(handler)
}

func (element *Element) OnPointerLeave(handler func(*native.Event)) *Element {
	return element.OnMouseLeave(handler)
}

func (element *Element) OnMenuSelect(handler func(*native.Event)) *Element {
	return element.OnSelect(handler)
}

func (element *Element) OnActivate(handler func(*native.Event)) *Element {
	return element.OnCommit(handler)
}
