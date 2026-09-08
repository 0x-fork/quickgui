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
