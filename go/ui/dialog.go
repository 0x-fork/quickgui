package ui

import (
	"github.com/egoist/quickgui/go/native"
	"github.com/egoist/quickgui/go/protocol"
	"github.com/egoist/quickgui/go/reactive"
)

// DialogOpenChangeReason is why a dialog opened or closed.
type DialogOpenChangeReason string

const (
	DialogTriggerPress DialogOpenChangeReason = "trigger-press"
	DialogClosePress   DialogOpenChangeReason = "close-press"
	DialogDismiss      DialogOpenChangeReason = "dismiss"
)

type DialogOpenChangeDetails struct {
	Reason DialogOpenChangeReason
	Event  *native.Event
}

type DialogRootProps struct {
	Children             Component
	Open                 func() bool
	DefaultOpen          bool
	OnOpenChange         func(bool, DialogOpenChangeDetails)
	OnOpenChangeComplete func(bool, *native.Event)
	DismissOnEscape      *bool
	DismissOnBackdrop    *bool
	EnterDuration        any
	ExitDuration         any
}

type DialogPopupProps struct {
	PartProps
	OnDismiss func(*native.Event)
}

type dialogVariant string

const (
	dialogVariantDialog      dialogVariant = "dialog"
	dialogVariantAlertDialog dialogVariant = "alertdialog"
)

type dialogState struct {
	scope   string
	variant dialogVariant
	props   DialogRootProps
	open    *reactive.Signal[bool]
}

func newDialogState(variant dialogVariant, props DialogRootProps) *dialogState {
	prefix := "qg-dialog"
	if variant == dialogVariantAlertDialog {
		prefix = "qg-alert-dialog"
	}
	return &dialogState{
		scope:   createComponentScope(prefix),
		variant: variant,
		props:   props,
		open:    reactive.NewSignal(props.DefaultOpen),
	}
}

func (state *dialogState) isOpen() bool {
	if state.props.Open != nil {
		return state.props.Open()
	}
	return state.open.Read()
}

func (state *dialogState) dismissOnEscape() bool {
	if state.props.DismissOnEscape != nil {
		return *state.props.DismissOnEscape
	}
	return true
}

func (state *dialogState) dismissOnBackdrop() bool {
	if state.props.DismissOnBackdrop != nil {
		return *state.props.DismissOnBackdrop
	}
	return state.variant != dialogVariantAlertDialog
}

func (state *dialogState) change(next bool, reason DialogOpenChangeReason, event *native.Event) {
	if state.props.Open == nil {
		state.open.Write(next)
	}
	if state.props.OnOpenChange != nil {
		state.props.OnOpenChange(next, DialogOpenChangeDetails{Reason: reason, Event: event})
	}
}

func (state *dialogState) complete(next bool, event *native.Event) {
	if state.props.OnOpenChangeComplete != nil {
		state.props.OnOpenChangeComplete(next, event)
	}
}

func (state *dialogState) applyTo(node *native.Node, part string) {
	setPart(node, part, state.scope, "")
	setString(node, protocol.Variant, string(state.variant))
	node.Bind(func() {
		setExplicitBool(node, protocol.Open, state.isOpen())
	})
}

var dialogContext = createPartContext[dialogState]()

func requireDialog(part string) *dialogState {
	return requireContext(dialogContext, part, "Dialog.Root")
}

func createDialogRoot(variant dialogVariant, props DialogRootProps) *native.Node {
	state := newDialogState(variant, props)
	return reactive.Provide(dialogContext, state, func() *native.Node {
		if props.Children == nil {
			return Fragment(nil)
		}
		return renderComponent(props.Children)
	})
}

func createDialogPart(part, name string, props PartProps) *native.Node {
	state := requireDialog(name)
	node := createViewPart(props)
	state.applyTo(node, part)
	return finishPart(node, props)
}

func createDialogTrigger(props PartProps) *native.Node {
	state := requireDialog("Dialog.Trigger")
	node := createButtonPart(props)
	state.applyTo(node, protocol.PartDialogTrigger)
	setListener(node, protocol.EventClick, forwardClick(props.OnClick, func(event *native.Event) {
		state.change(true, DialogTriggerPress, event)
	}))
	return finishPart(node, props)
}

func createDialogPortal(props PartProps) *native.Node {
	state := requireDialog("Dialog.Portal")
	node := createViewPart(props)
	state.applyTo(node, protocol.PartDialog)
	setMilliseconds(node, protocol.EnterDuration, state.props.EnterDuration)
	setMilliseconds(node, protocol.ExitDuration, state.props.ExitDuration)
	setListener(node, protocol.EventComponentChange, func(event *native.Event) {
		details := ComponentChangeFromEvent(event)
		if details == nil || details.OpenChangeComplete == nil {
			return
		}
		state.complete(*details.OpenChangeComplete, event)
	})
	return finishPart(node, props)
}

func createDialogPopup(props DialogPopupProps) *native.Node {
	state := requireDialog("Dialog.Popup")
	node := createViewPart(props.PartProps)
	state.applyTo(node, protocol.PartDialogPopup)
	setExplicitBool(node, protocol.DismissOnEscape, state.dismissOnEscape())
	setExplicitBool(node, protocol.DismissOnPointerOutside, state.dismissOnBackdrop())
	setListener(node, protocol.EventDismiss, func(event *native.Event) {
		if props.OnDismiss != nil {
			props.OnDismiss(event)
		}
		if !event.DefaultPrevented {
			state.change(false, DialogDismiss, event)
		}
	})
	return finishPart(node, props.PartProps)
}

func createDialogClose(props PartProps) *native.Node {
	state := requireDialog("Dialog.Close")
	node := createButtonPart(props)
	state.applyTo(node, protocol.PartDialogClose)
	setListener(node, protocol.EventClick, forwardClick(props.OnClick, func(event *native.Event) {
		state.change(false, DialogClosePress, event)
	}))
	return finishPart(node, props)
}

// Dialog is a controlled in-window modal dialog.
var Dialog = dialogAPI{}

type dialogAPI struct{}

func (dialogAPI) Root(props DialogRootProps, children ...any) *native.Node {
	props.Children = withPartChildren(props.Children, children)

	return createDialogRoot(dialogVariantDialog, props)
}

func (dialogAPI) Trigger(props PartProps, children ...any) *native.Node {
	props.Children = withPartChildren(props.Children, children)
	return createDialogTrigger(props)
}

func (dialogAPI) Portal(props PartProps, children ...any) *native.Node {
	props.Children = withPartChildren(props.Children, children)
	return createDialogPortal(props)
}

func (dialogAPI) Viewport(props PartProps, children ...any) *native.Node {
	props.Children = withPartChildren(props.Children, children)

	return createDialogPart(protocol.PartDialogViewport, "Dialog.Viewport", props)
}

func (dialogAPI) Backdrop(props PartProps, children ...any) *native.Node {
	props.Children = withPartChildren(props.Children, children)

	return createDialogPart(protocol.PartDialogBackdrop, "Dialog.Backdrop", props)
}

func (dialogAPI) Popup(props DialogPopupProps, children ...any) *native.Node {
	props.Children = withPartChildren(props.Children, children)
	return createDialogPopup(props)
}

func (dialogAPI) Title(props PartProps, children ...any) *native.Node {
	props.Children = withPartChildren(props.Children, children)

	return createDialogPart(protocol.PartDialogTitle, "Dialog.Title", props)
}

func (dialogAPI) Description(props PartProps, children ...any) *native.Node {
	props.Children = withPartChildren(props.Children, children)

	return createDialogPart(protocol.PartDialogDescription, "Dialog.Description", props)
}

func (dialogAPI) Close(props PartProps, children ...any) *native.Node {
	props.Children = withPartChildren(props.Children, children)
	return createDialogClose(props)
}

// AlertDialog is a consequential dialog whose backdrop does not dismiss by default.
var AlertDialog = alertDialogAPI{}

type alertDialogAPI struct{}

func (alertDialogAPI) Root(props DialogRootProps, children ...any) *native.Node {
	props.Children = withPartChildren(props.Children, children)

	return createDialogRoot(dialogVariantAlertDialog, props)
}

func (alertDialogAPI) Trigger(props PartProps, children ...any) *native.Node {
	props.Children = withPartChildren(props.Children, children)
	return createDialogTrigger(props)
}

func (alertDialogAPI) Portal(props PartProps, children ...any) *native.Node {
	props.Children = withPartChildren(props.Children, children)
	return createDialogPortal(props)
}

func (alertDialogAPI) Viewport(props PartProps, children ...any) *native.Node {
	props.Children = withPartChildren(props.Children, children)
	return Dialog.Viewport(props)
}

func (alertDialogAPI) Backdrop(props PartProps, children ...any) *native.Node {
	props.Children = withPartChildren(props.Children, children)
	return Dialog.Backdrop(props)
}

func (alertDialogAPI) Popup(props DialogPopupProps, children ...any) *native.Node {
	props.Children = withPartChildren(props.Children, children)
	return createDialogPopup(props)
}

func (alertDialogAPI) Title(props PartProps, children ...any) *native.Node {
	props.Children = withPartChildren(props.Children, children)
	return Dialog.Title(props)
}

func (alertDialogAPI) Description(props PartProps, children ...any) *native.Node {
	props.Children = withPartChildren(props.Children, children)
	return Dialog.Description(props)
}

func (alertDialogAPI) Close(props PartProps, children ...any) *native.Node {
	props.Children = withPartChildren(props.Children, children)
	return createDialogClose(props)
}
