package cart

import "github.com/egoist/quickgui/go/ui"

type Product struct {
	ID    string
	Name  string
	Price int
}

var builds int

func LineItem(product Product, quantity int, add func()) {
	builds++
	initial := quantity
	ui.View(
		ui.Text(product.Name),
		ui.Text(quantity),
		ui.Text(product.Price*quantity),
		ui.Text(initial),
		ui.Button("Add").OnClick(add),
	).Width(quantity * 20)
}

func ForwardedItem(product Product, quantity int, add func()) {
	LineItem(product, quantity, add)
}

func Label(value string) {
	initial := value
	ui.View(
		ui.Text(value),
		ui.Text(initial),
		ui.View(func() {
			value := "shadowed"
			ui.Text(value)
		}),
	)
}

func EventSnapshot(value string) {
	input := ui.Input().Value("initial")
	ui.View(input, ui.Button("Apply").OnClick(func() { input.Value(value) }))
}
