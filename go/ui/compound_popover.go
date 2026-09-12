package ui

import (
	"github.com/egoist/quickgui/go/native"
	"github.com/egoist/quickgui/go/reactive"
)

// Content mounts and unmounts with its popover. Declarations apply to each real
// content root, including native child-window roots, rather than its group anchor.
type compoundContent struct {
	declaration *compoundElement
	current     *Element
	children    []any
}

func (component *PopoverComponent) Content(options ...PopoverContentProps) *Element {
	return popoverContentElement(&component.instance, popoverSurfaceInWindow, componentProps("Popover.Content", options))
}

func (component *SystemPopoverComponent) Content(options ...PopoverContentProps) *Element {
	return popoverContentElement(&component.instance, popoverSurfaceSystem, componentProps("SystemPopover.Content", options))
}

func popoverContentElement(instance *componentInstance, surface popoverSurface, props PopoverContentProps) *Element {
	content := &compoundContent{}
	var element *Element
	ref := props.Ref
	element = instance.partElement(func(children Component) *native.Node {
		props.Children = func() *native.Node {
			return renderComponent(withPartChildren(children, content.children))
		}
		props.Ref = func(node *native.Node) {
			target := &Element{Node: node, childOwner: element.childOwner}
			content.current = target
			target.applyCompoundDeclaration(content.declaration)
			reactive.RunWithOwner(node.BindingOwner(), func() struct{} {
				reactive.OnCleanup(func() {
					if content.current == target {
						content.current = nil
					}
				})
				return struct{}{}
			})
			if ref != nil {
				ref(node)
			}
		}
		if surface == popoverSurfaceSystem {
			return (systemPopoverAPI{}).Content(props)
		}
		return (popoverAPI{}).Content(props)
	}, props.Children)
	content.declaration = element.compound
	element.content = content
	return element
}
