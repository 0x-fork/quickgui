package widgets

import "github.com/egoist/quickgui/go/ui"

type Props struct {
	Name     string
	Quantity int
}

func Label(props Props) {
	ui.Text(props.Name, props.Quantity)
}
