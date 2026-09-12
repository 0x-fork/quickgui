package ui

import (
	"reflect"

	"github.com/egoist/quickgui/go/native"
)

type elementBinding int

const propertyBindingOffset = 1024

var styleBindingNames = bindingNames(reflect.TypeOf(styleData{}), 0)
var propertyBindingNames = bindingNames(reflect.TypeOf(Props{}), propertyBindingOffset)

func bindingNames(record reflect.Type, offset int) map[string]elementBinding {
	fields := make(map[string]elementBinding)
	for index := 0; index < record.NumField(); index++ {
		field := record.Field(index)
		if field.IsExported() {
			fields[field.Name] = elementBinding(offset + index)
		}
	}
	return fields
}

// Reflection only enumerates a constructor or reusable style record. Generated
// modifiers name their field directly and dispatch to typed binding setters.
func populatedStyleBindings(style styleData) []elementBinding {
	var fields []elementBinding
	value := reflect.ValueOf(style)
	for index := 0; index < value.NumField(); index++ {
		if value.Type().Field(index).IsExported() && !value.Field(index).IsZero() {
			fields = append(fields, elementBinding(index))
		}
	}
	if len(style.groupHoverRules) != 0 && style.GroupHover == nil {
		fields = append(fields, styleBindingNames["GroupHover"])
	}
	if len(style.groupActiveRules) != 0 && style.GroupActive == nil {
		fields = append(fields, styleBindingNames["GroupActive"])
	}
	return fields
}

func populatedElementBindings(props Props) []elementBinding {
	fields := populatedStyleBindings(props.Style.style)
	value := reflect.ValueOf(props)
	for index := 0; index < value.NumField(); index++ {
		name := value.Type().Field(index).Name
		if name != "Style" && name != "Children" && name != "Ref" && !value.Field(index).IsZero() {
			fields = append(fields, elementBinding(propertyBindingOffset+index))
		}
	}
	return fields
}

func (element *Element) configureStyle(name string, option Option) *Element {
	return element.configureFields([]elementBinding{styleBindingNames[name]}, option)
}

func (element *Element) configureProperty(name string, option Option) *Element {
	return element.configureFields([]elementBinding{propertyBindingNames[name]}, option)
}

func (element *Element) configureStyles(names []string, options ...Option) *Element {
	fields := make([]elementBinding, len(names))
	for index, name := range names {
		fields[index] = styleBindingNames[name]
	}
	return element.configureFields(fields, options...)
}

func (element *Element) refreshFields(fields []elementBinding) {
	for _, field := range fields {
		if element.bindings == nil {
			element.bindings = make(map[elementBinding]*native.PropertyBinding)
		}
		binding := element.bindings[field]
		if binding == nil {
			binding = native.NewPropertyBinding(element.Node)
			element.bindings[field] = binding
		}
		binding.Set(func() {
			var previous []*native.Listener
			if len(element.componentListeners) != 0 {
				previous = append(previous, element.Node.Listeners...)
			}
			applyElementBinding(element.Node, element.props, field)
			element.restoreComponentListeners(previous)
		})
	}
}

func conditionalArguments(arguments []any) bool {
	for _, argument := range arguments {
		if _, ok := argument.(conditionalOption); ok {
			return true
		}
	}
	return false
}

// These shorthands write overlapping native property groups. Keep the complete
// declaration fallback so later overrides and conditional clearing stay ordered.
func overlappingStyle(style styleData) bool {
	return style.Flex != nil || style.Background != nil || style.BackgroundGradient != nil ||
		style.Transition != nil || style.GridColumn != nil || style.GridRow != nil ||
		style.Outline != nil || style.TextDecoration != ""
}

func (element *Element) reconcile() {
	if element.compound != nil {
		element.compound.reconcile = true
		return
	}
	if element.content != nil {
		element.content.declaration.reconcile = true
		if element.content.current != nil {
			element.content.current.reconcile()
		}
		return
	}
	if element.update != nil {
		return
	}
	if element.Removed {
		panic("a removed QuickGUI element cannot be configured")
	}
	for _, binding := range element.bindings {
		binding.Dispose()
	}
	element.bindings = nil
	var base Props
	if element.props != nil {
		base = *element.props
	}
	base.Children, base.Ref = nil, nil
	element.arguments = []any{base}
	element.update = element.BindProperties(func() {
		props := resolveProps(element.arguments)
		clearOmittedListeners(element.Node, props)
		applyPropValues(element.Node, props)
		element.restoreComponentListeners(nil)
	})
}

func setFluentListener(node *native.Node, kind int, handler func(*native.Event)) {
	if handler == nil {
		native.SetEventListener(node, kind, nil)
	} else {
		setListener(node, kind, handler)
	}
}
