package cart

import (
	"github.com/egoist/quickgui/go/native"
	"github.com/egoist/quickgui/go/ui"
)

func ValueCard(value int, visible bool, cleanup func()) *ui.Element {
	return ui.View().Children(
		ui.Text(value),
		ui.Show(
			visible,
			func() *ui.Element {
				ui.OnCleanup(cleanup)
				return ui.Text(value)
			},
		),
	).Width(value*10).When(visible, ui.Style().Opacity(0.5))
}

func RawNodeLabel(value int) *native.Node {
	return ui.Text(value).Node
}

func ordinaryValue(value int) int { return value }

func CompoundPopover(open bool, change func(bool), mounted func(), ref func(*native.Node)) *ui.Element {
	mounted()
	popover := ui.NewPopover().Open(open).OnOpenChange(func(value bool, _ ui.PopoverOpenChangeDetails) { change(value) })
	return popover.Root().Child(popover.Trigger().Ref(ref).Child("some text"))
}

func CreatePopoverOnClick(open bool, change func(bool), mount func(*ui.Element)) *ui.Element {
	return ui.Button().Child("Create popover").OnClick(func() {
		popover := ui.NewPopover().Open(open).OnOpenChange(func(value bool, _ ui.PopoverOpenChangeDetails) { change(value) })
		mount(popover.Root().Child(popover.Trigger().Child("Open")))
	})
}
