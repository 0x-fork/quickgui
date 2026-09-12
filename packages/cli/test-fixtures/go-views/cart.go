package cart

import "github.com/egoist/quickgui/go/ui"

type Product struct {
	ID    string
	Name  string
	Price int
}

var builds int

func LineItem(product Product, quantity int, add func()) *ui.Element {
	builds++
	initial := quantity
	return ui.View().Children(
		ui.Text(product.Name),
		ui.Text(quantity),
		ui.Text(product.Price*quantity),
		ui.Text(initial),
		ui.Button("Add").OnClick(add),
	).Width(quantity * 20)
}

func ForwardedItem(product Product, quantity int, add func()) *ui.Element {
	return LineItem(product, quantity, add)
}

func Label(value string) *ui.Element {
	initial := value
	return ui.View().Children(
		ui.Text(value),
		ui.Text(initial),
		ui.View().Child(func() *ui.Element {
			value := "shadowed"
			return ui.Text(value)
		}),
	)
}

func EventSnapshot(value string) *ui.Element {
	input := ui.Input().Value("initial")
	return ui.View().Children(input, ui.Button("Apply").OnClick(func() { input.Value(value) }))
}

var fluentBuilds, fluentChildBuilds int

func FluentChildren(value int) *ui.Element {
	fluentBuilds++
	return ui.View().Width(value).
		Child(value).
		Children(" / ", value*2).
		Child(ui.View().Child(func() *ui.Element {
			fluentChildBuilds++
			return ui.Text(value)
		}))
}
