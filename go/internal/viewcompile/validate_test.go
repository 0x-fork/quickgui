package viewcompile

import (
	"go/ast"
	"go/parser"
	"go/token"
	"go/types"
	"strings"
	"testing"
)

func TestConstructionRequiresExplicitNodeUse(t *testing.T) {
	fs := token.NewFileSet()
	sdk, err := parser.ParseFile(fs, "ui.go", `package ui
type Element struct{}
func View(children ...any) *Element { return nil }
func Text(children ...any) *Element { return nil }
func Button(children ...any) *Element { return nil }
func (e *Element) Width(value any) *Element { return e }
func (e *Element) OnClick(handler func()) *Element { return e }
`, 0)
	if err != nil {
		t.Fatal(err)
	}
	ui, err := new(types.Config).Check(uiPath, fs, []*ast.File{sdk}, nil)
	if err != nil {
		t.Fatal(err)
	}
	for _, test := range []struct {
		name, body string
		invalid    bool
	}{
		{"view", `func App() { gui.View() }`, true},
		{"text", `func App() { gui.Text("lost") }`, true},
		{"button chain", `func App() { gui.Button("lost").Width(20) }`, true},
		{"custom component", `func Child() *gui.Element { return gui.Text("child") }; func App() { Child() }`, true},
		{"blank assignment", `func App() { _ = gui.View() }`, true},
		{"void children", `func App() *gui.Element { return gui.View(func() {}) }`, true},
		{"named void children", `func App() *gui.Element { body := func() {}; return gui.View(body) }`, true},
		{"deferred construction", `func App() { defer gui.View() }`, true},
		{"returned children", `func App() *gui.Element { return gui.View(gui.Text("child")) }`, false},
		{"returning factory", `func App() *gui.Element { return gui.View(func() *gui.Element { return gui.Text("child") }) }`, false},
		{"assigned node and mutation", `func App() *gui.Element { view := gui.View(); view.Width(20); return view }`, false},
		{"event callback", `func App() *gui.Element { view := gui.View(); return gui.Button("change").OnClick(func() { view.Width(20) }) }`, false},
	} {
		t.Run(test.name, func(t *testing.T) {
			file, err := parser.ParseFile(fs, "app.go", "package app\nimport gui \""+uiPath+"\"\n"+test.body, 0)
			if err != nil {
				t.Fatal(err)
			}
			info := &types.Info{Types: map[ast.Expr]types.TypeAndValue{}, Uses: map[*ast.Ident]types.Object{}, Selections: map[*ast.SelectorExpr]*types.Selection{}}
			config := types.Config{Importer: sourceImporter{func(string) (*types.Package, error) { return ui, nil }}}
			if _, err := config.Check("app", fs, []*ast.File{file}, info); err != nil {
				t.Fatal(err)
			}
			err = Validate(fs, file, info)
			if (err != nil) != test.invalid {
				t.Fatalf("invalid=%v, got %v", test.invalid, err)
			}
			if err != nil && !strings.HasPrefix(err.Error(), "app.go:3:") {
				t.Fatalf("diagnostic lost authored location: %v", err)
			}
		})
	}
}
