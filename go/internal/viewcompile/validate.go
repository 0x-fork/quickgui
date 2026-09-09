package viewcompile

import (
	"errors"
	"fmt"
	"go/ast"
	"go/token"
	"go/types"
	"strings"
)

// Validate checks the authored program before rewriting it. Node construction
// has no implicit declaration side effects; each result needs an explicit use.
func Validate(fs *token.FileSet, file *ast.File, info *types.Info) error {
	var failures []error
	errAt := func(node ast.Node, message string) {
		if len(failures) < 20 {
			failures = append(failures, fmt.Errorf("%s: %s", fs.Position(node.Pos()), message))
		}
	}
	var constructed func(ast.Expr) bool
	constructed = func(expr ast.Expr) bool {
		switch value := expr.(type) {
		case *ast.ParenExpr:
			return constructed(value.X)
		case *ast.CallExpr:
			if !element(info.TypeOf(value)) {
				return false
			}
			if selector, ok := value.Fun.(*ast.SelectorExpr); ok {
				if selection := info.Selections[selector]; selection != nil && element(selection.Recv()) {
					// Setters on an existing node are imperative mutations. A chain
					// starting with a constructor still has an unused new node.
					return constructed(selector.X)
				}
			}
			return true
		}
		return false
	}
	voidCallback := func(expr ast.Expr) bool {
		typ := info.TypeOf(expr)
		if typ == nil {
			return false
		}
		signature, ok := typ.Underlying().(*types.Signature)
		return ok && signature.Results().Len() == 0
	}
	unused := func(expr ast.Expr) {
		if constructed(expr) {
			errAt(expr, "UI construction results must be returned, assigned to a node, or passed as children; implicit UI declarations are not supported.")
		}
	}
	ast.Inspect(file, func(node ast.Node) bool {
		if len(failures) >= 20 {
			return false
		}
		switch value := node.(type) {
		case *ast.ExprStmt:
			unused(value.X)
		case *ast.GoStmt:
			unused(value.Call)
		case *ast.DeferStmt:
			unused(value.Call)
		case *ast.AssignStmt:
			for i, lhs := range value.Lhs {
				if id, ok := lhs.(*ast.Ident); ok && id.Name == "_" && i < len(value.Rhs) {
					unused(value.Rhs[i])
				}
			}
		case *ast.ValueSpec:
			for i, id := range value.Names {
				if id.Name == "_" && i < len(value.Values) {
					unused(value.Values[i])
				}
			}
		case *ast.CallExpr:
			obj := calledObject(value.Fun, info)
			if obj == nil || obj.Pkg() == nil || obj.Pkg().Path() != uiPath || !element(info.TypeOf(value)) {
				break
			}
			if selector, ok := value.Fun.(*ast.SelectorExpr); ok && info.Selections[selector] != nil && element(info.Selections[selector].Recv()) {
				break
			}
			for _, arg := range value.Args {
				if voidCallback(arg) {
					errAt(arg, "UI construction callbacks must return *ui.Element or *native.Node; func() declaration blocks are not supported.")
				}
			}
		case *ast.KeyValueExpr:
			key, ok := value.Key.(*ast.Ident)
			if !ok || (key.Name != "Component" && key.Name != "Children") {
				break
			}
			obj := info.Uses[key]
			if obj != nil && obj.Pkg() != nil && strings.HasPrefix(obj.Pkg().Path(), "github.com/egoist/quickgui/go/") && voidCallback(value.Value) {
				errAt(value.Value, "UI components and children callbacks must return a native node; implicit UI declarations are not supported.")
			}
		}
		return true
	})
	return errors.Join(failures...)
}
