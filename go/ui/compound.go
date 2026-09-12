package ui

import (
	"fmt"

	"github.com/egoist/quickgui/go/native"
	"github.com/egoist/quickgui/go/reactive"
)

// Compound parts are declarations until inserted. This lets ordinary Go argument
// evaluation construct parts before their root has installed its context. Once
// mounted, they use the same retained nodes and bindings as primitive elements.
type compoundElement struct {
	build        func([]any) *native.Node
	children     []any
	declarations []compoundDeclaration
	reconcile    bool
	refs         []func(*native.Node)
	mounting     bool
}

type compoundDeclaration struct {
	fields  []elementBinding
	options []Option
}

// Keep the latest declaration for an independent property group. Content can
// reopen repeatedly, so ordinary resize/property setters must not retain history.
func (declaration *compoundElement) record(fields []elementBinding, options []Option) {
	compact := len(fields) != 0
	for _, field := range fields {
		if field == styleBindingNames["GroupHover"] || field == styleBindingNames["GroupActive"] {
			compact = false // Named group rules deliberately accumulate.
		}
	}
	if compact {
		kept := declaration.declarations[:0]
		for _, previous := range declaration.declarations {
			equal := len(previous.fields) == len(fields)
			for index := 0; equal && index < len(fields); index++ {
				equal = previous.fields[index] == fields[index]
			}
			if !equal {
				kept = append(kept, previous)
			}
		}
		clear(declaration.declarations[len(kept):])
		declaration.declarations = kept
	}
	declaration.declarations = append(declaration.declarations, compoundDeclaration{fields: fields, options: options})
}

type compoundContext struct {
	instance *componentInstance
	parent   *compoundContext
	outer    *reactive.Owner
}

var compoundContextKey = reactive.CreateContext[*compoundContext](nil)

type componentInstance struct {
	name  string
	owner *reactive.Owner
	root  *Element
}

func newCompoundElement(build func([]any) *native.Node) *Element {
	return &Element{compound: &compoundElement{build: build}}
}

func (element *Element) mountCompound() {
	declaration := element.compound
	if declaration.mounting {
		panic("a compound element cannot contain itself")
	}
	declaration.mounting = true
	node := declaration.build(declaration.children)
	if node == nil {
		panic("a compound part must return a native node")
	}
	element.Node = node
	element.compound = nil
	if element.content == nil {
		element.applyCompoundDeclaration(declaration)
	}
}

func (element *Element) applyCompoundDeclaration(declaration *compoundElement) {
	node := element.Node
	for _, listener := range node.Listeners {
		if element.componentListeners == nil {
			element.componentListeners = make(map[int]native.EventListener)
		}
		element.componentListeners[listener.Type] = listener.Listener
	}
	if declaration.reconcile {
		element.reconcile()
	}
	for _, update := range declaration.declarations {
		element.configureFields(update.fields, update.options...)
	}
	for _, ref := range declaration.refs {
		if ref != nil {
			ref(node)
		}
	}
}

func (instance *componentInstance) configure() {
	if instance.owner != nil {
		panic(instance.name + " settings must be declared before its root mounts; use accessors for live values")
	}
}

func (instance *componentInstance) rootElement(transparent bool, build func(Component) *native.Node, previous Component) *Element {
	if instance.root != nil {
		return instance.root
	}
	instance.root = newCompoundElement(func(children []any) *native.Node {
		context := &compoundContext{instance: instance, parent: compoundContextKey.Use(), outer: reactive.GetOwner()}
		scope := reactive.NewOwner(reactive.GetOwner())
		return reactive.RunWithOwner(scope, func() *native.Node {
			node := build(func() *native.Node {
				return reactive.Provide(compoundContextKey, context, func() *native.Node {
					instance.owner = reactive.GetOwner()
					content := withPartChildren(previous, children)
					if transparent {
						return View().Child(content).Node
					}
					if content == nil {
						return nil
					}
					return renderComponent(content)
				})
			})
			if instance.owner == nil {
				instance.owner = scope
			}
			instance.root.childOwner = instance.owner
			disposeWithNode(node, scope)
			return node
		})
	})
	return instance.root
}

func (instance *componentInstance) partElement(build func(Component) *native.Node, previous Component) *Element {
	element := newCompoundElement(nil)
	element.compound.build = func(children []any) *native.Node {
		if instance.owner == nil {
			panic(instance.name + " parts must be mounted inside their instance's Root")
		}
		if instance.owner.Disposed {
			panic(instance.name + " cannot mount parts after its root is removed")
		}
		// Keep nested item/group contexts when this part is already being built
		// below its instance. A different instance always uses its own context.
		owner := reactive.GetOwner()
		found := false
		for context := compoundContextKey.Use(); context != nil; context = context.parent {
			if context.instance == instance {
				found = true
				break
			}
			if componentFamily(context.instance.name) == componentFamily(instance.name) {
				owner = context.outer
			}
		}
		if !found {
			owner = instance.owner
		}
		scope := reactive.NewOwner(owner)
		return reactive.RunWithOwner(scope, func() *native.Node {
			node := build(func() *native.Node {
				element.childOwner = reactive.GetOwner()
				content := withPartChildren(previous, children)
				if content == nil {
					return nil
				}
				return renderComponent(content)
			})
			disposeWithNode(node, scope)
			return node
		})
	}
	return element
}

func componentFamily(name string) string {
	switch name {
	case "Popover", "SystemPopover":
		return "popover"
	case "Dialog", "AlertDialog":
		return "dialog"
	case "Select", "Combobox", "Autocomplete":
		return "picker"
	case "Menu", "ContextMenu":
		return "menu"
	case "Progress", "Meter":
		return "gauge"
	default:
		return name
	}
}

func disposeWithNode(node *native.Node, owner *reactive.Owner) {
	reactive.RunWithOwner(node.BindingOwner(), func() struct{} {
		reactive.OnCleanup(func() { reactive.DisposeOwner(owner) })
		return struct{}{}
	})
}

// User handlers run before a component's default action. Replacing or clearing a
// fluent handler keeps that action, and PreventDefault cancels it.
func (element *Element) restoreComponentListeners(previous []*native.Listener) {
	for kind, builtin := range element.componentListeners {
		var current *native.Listener
		for _, listener := range element.Node.Listeners {
			if listener.Type == kind {
				current = listener
				break
			}
		}
		unchanged := false
		for _, listener := range previous {
			if listener == current {
				unchanged = true
				break
			}
		}
		if unchanged {
			continue
		}
		if current == nil {
			native.SetEventListener(element.Node, kind, builtin)
			continue
		}
		handler := current.Listener
		setListener(element.Node, kind, func(event *native.Event) {
			handler(event)
			if !event.DefaultPrevented {
				builtin(event)
			}
		})
	}
}

func componentProps[T any](name string, props []T) T {
	if len(props) > 1 {
		panic(name + " accepts at most one props value")
	}
	if len(props) == 1 {
		return props[0]
	}
	var zero T
	return zero
}

func componentAccessor[T any](value any) func() T {
	switch value := value.(type) {
	case nil:
		return nil
	case func() T:
		return value
	case reactive.Accessor[T]:
		return value
	case T:
		return func() T { return value }
	default:
		panic(fmt.Sprintf("QuickGUI expected a value or accessor, got %T", value))
	}
}

func componentOptionalAccessor[T any](value any) func() *T {
	switch value := value.(type) {
	case nil:
		return func() *T { return nil }
	case T:
		return func() *T { return &value }
	case *T:
		return func() *T { return value }
	case func() T:
		return func() *T { next := value(); return &next }
	case reactive.Accessor[T]:
		return func() *T { next := value(); return &next }
	case func() *T:
		return value
	case reactive.Accessor[*T]:
		return value
	default:
		panic(fmt.Sprintf("QuickGUI expected an optional value or accessor, got %T", value))
	}
}

func componentCheckedAccessor(value any) func() CheckedState {
	switch value := value.(type) {
	case func() bool:
		return func() CheckedState { return value() }
	case reactive.Accessor[bool]:
		return func() CheckedState { return value() }
	case func() string:
		return func() CheckedState { return value() }
	case reactive.Accessor[string]:
		return func() CheckedState { return value() }
	default:
		return componentAccessor[CheckedState](value)
	}
}

func componentNumberAccessor(value any) func() *float64 {
	if value == nil {
		return nil
	}
	return func() *float64 { return resolveNumber(value) }
}

func componentNumberValueAccessor(value any) func() float64 {
	read := componentNumberAccessor(value)
	if read == nil {
		return nil
	}
	return func() float64 {
		if next := read(); next != nil {
			return *next
		}
		return 0
	}
}
