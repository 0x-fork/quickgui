package viewcompile

import (
	"go/ast"
	"go/parser"
	"go/token"
	"go/types"
	"strings"
	"testing"
)

func TestGeneratedDiagnosticUsesOriginalFileLineAndColumn(t *testing.T) {
	fs := token.NewFileSet()
	sdk, err := parser.ParseFile(fs, "ui.go", "package ui\ntype Element struct{}\nfunc Text(children ...any) *Element { return nil }", 0)
	if err != nil {
		t.Fatal(err)
	}
	ui, err := new(types.Config).Check(uiPath, fs, []*ast.File{sdk}, nil)
	if err != nil {
		t.Fatal(err)
	}
	config := types.Config{Importer: sourceImporter{func(string) (*types.Package, error) { return ui, nil }}}
	const filename = "/workspace/cart.go"
	const input = "package cart\nimport gui \"github.com/egoist/quickgui/go/ui\"\n\nfunc Label(quantity int) *gui.Element {\n\treturn gui.Text(quantity)\n}\n"
	file, err := parser.ParseFile(fs, filename, input, parser.ParseComments)
	if err != nil {
		t.Fatal(err)
	}
	info := &types.Info{Types: map[ast.Expr]types.TypeAndValue{}, Defs: map[*ast.Ident]types.Object{}, Uses: map[*ast.Ident]types.Object{}, Implicits: map[ast.Node]types.Object{}}
	pkg, err := config.Check("example/cart", fs, []*ast.File{file}, info)
	if err != nil {
		t.Fatal(err)
	}
	fn := file.Decls[1].(*ast.FuncDecl)
	component, err := Describe(fn, info)
	if err != nil {
		t.Fatal(err)
	}
	output, err := Source(fs, file, []byte(input), pkg, info, map[string]Component{"example/cart.Label": component})
	if err != nil {
		t.Fatal(err)
	}
	// Simulate a compiler error in a moved binding, after the source transform.
	broken := strings.Replace(string(output), "quantity()", "missing()", 1)
	generatedSet := token.NewFileSet()
	generated, err := parser.ParseFile(generatedSet, "/cache/generated.go", broken, parser.ParseComments)
	if err != nil {
		t.Fatal(err)
	}
	_, err = config.Check("example/cart", generatedSet, []*ast.File{generated}, nil)
	if err == nil || !strings.HasPrefix(err.Error(), filename+":5:18:") {
		t.Fatalf("diagnostic must refer to the original prop expression, got %v", err)
	}
}

func TestComponentReturnTypesPreserveAliasesShadowingAndMethods(t *testing.T) {
	fs := token.NewFileSet()
	sdk, err := parser.ParseFile(fs, "ui.go", `package ui

type Element struct{}
func Text(children ...any) *Element { return nil }
func (e *Element) Width(value any) *Element { return e }
`, 0)
	if err != nil {
		t.Fatal(err)
	}
	ui, err := new(types.Config).Check(uiPath, fs, []*ast.File{sdk}, nil)
	if err != nil {
		t.Fatal(err)
	}
	const source = `package app
import gui "github.com/egoist/quickgui/go/ui"
type ElementAlias = gui.Element
func Forward(value int) *gui.Element { return (Label)(value) }
func Label(value int) *ElementAlias { return gui.Text(value) }
func Chained(value int) *gui.Element { return gui.Text(value).Width(value) }
func Ordinary(value int) int { return value }
func Setup(value int) { callback := func() *gui.Element { return gui.Text(value) }; _ = callback }
func Mutate(node *gui.Element, value int) { node.Width(value) }
func Shadow(Label func()) { Label() }
type Widget struct{}
func (Widget) Label(value int) *gui.Element { return gui.Text(value) }
`
	file, err := parser.ParseFile(fs, "app.go", source, 0)
	if err != nil {
		t.Fatal(err)
	}
	info := &types.Info{Types: map[ast.Expr]types.TypeAndValue{}, Defs: map[*ast.Ident]types.Object{}, Uses: map[*ast.Ident]types.Object{}, Implicits: map[ast.Node]types.Object{}, Selections: map[*ast.SelectorExpr]*types.Selection{}}
	config := types.Config{Importer: sourceImporter{func(string) (*types.Package, error) { return ui, nil }}}
	pkg, err := config.Check("app", fs, []*ast.File{file}, info)
	if err != nil {
		t.Fatal(err)
	}
	components := map[string]Component{}
	for changed := true; changed; {
		changed = false
		for _, decl := range file.Decls {
			fn, ok := decl.(*ast.FuncDecl)
			if !ok || !IsComponent(fn, info) {
				continue
			}
			key := componentKey(info.Defs[fn.Name])
			if _, exists := components[key]; exists {
				continue
			}
			component, err := Describe(fn, info)
			if err != nil {
				t.Fatal(err)
			}
			components[key] = component
			changed = true
		}
	}
	if len(components) != 3 || components["app.Forward"].Name == "" || components["app.Label"].Name == "" || components["app.Chained"].Name == "" {
		t.Fatalf("incorrect component boundaries: %v", components)
	}
	output, err := Source(fs, file, []byte(source), pkg, info, components)
	if err != nil {
		t.Fatal(err)
	}
	generatedSet := token.NewFileSet()
	generated, err := parser.ParseFile(generatedSet, "generated.go", output, 0)
	if err != nil {
		t.Fatal(err)
	}
	if _, err := config.Check("app", generatedSet, []*ast.File{generated}, nil); err != nil {
		t.Fatalf("ordinary helpers or a same-named method changed meaning: %v", err)
	}
}
