package viewcompile

import (
	"errors"
	"fmt"
	"go/ast"
	"go/token"
	"go/types"
	"strings"
)

func calledObject(expr ast.Expr, info *types.Info) types.Object {
	switch value := expr.(type) {
	case *ast.Ident:
		return info.Uses[value]
	case *ast.SelectorExpr:
		return info.Uses[value.Sel]
	case *ast.IndexExpr:
		return calledObject(value.X, info)
	case *ast.IndexListExpr:
		return calledObject(value.X, info)
	case *ast.ParenExpr:
		return calledObject(value.X, info)
	}
	return nil
}

func componentKey(obj types.Object) string {
	fn, ok := obj.(*types.Func)
	if !ok || fn.Pkg() == nil || fn.Type().(*types.Signature).Recv() != nil {
		return ""
	}
	return fn.Pkg().Path() + "." + fn.Name()
}

// Validate catches unsupported factories in dynamically typed child positions.
// Typed Component fields, branches, and list renderers are checked by Go itself.
// Ordinary helpers may return node handles, but are not reactive components.
func Validate(fs *token.FileSet, file *ast.File, info *types.Info) error {
	var failures []error
	var child func(ast.Expr)
	child = func(expr ast.Expr) {
		if len(failures) >= 20 {
			return
		}
		if typ := info.TypeOf(expr); typ != nil {
			if sig, ok := typ.Underlying().(*types.Signature); ok && sig.Results().Len() == 1 && element(sig.Results().At(0).Type()) {
				failures = append(failures, fmt.Errorf("%s: UI construction callbacks must use func() and declare children implicitly; node-returning component factories are not supported", fs.Position(expr.Pos())))
				return
			}
		}
		if list, ok := expr.(*ast.CompositeLit); ok {
			if _, slice := info.TypeOf(list).Underlying().(*types.Slice); slice {
				for _, item := range list.Elts {
					if pair, ok := item.(*ast.KeyValueExpr); ok {
						item = pair.Value
					}
					child(item)
				}
			}
		}
	}
	ast.Inspect(file, func(node ast.Node) bool {
		switch value := node.(type) {
		case *ast.CallExpr:
			obj := calledObject(value.Fun, info)
			if obj == nil || obj.Pkg() == nil || obj.Pkg().Path() != uiPath {
				break
			}
			if !element(info.TypeOf(value)) && obj.Name() != "Child" {
				break
			}
			if selector, ok := value.Fun.(*ast.SelectorExpr); ok {
				if selection := info.Selections[selector]; selection != nil && element(selection.Recv()) {
					break
				}
			}
			for _, arg := range value.Args {
				child(arg)
			}
		case *ast.KeyValueExpr:
			key, ok := value.Key.(*ast.Ident)
			if !ok || (key.Name != "Component" && key.Name != "Children") {
				break
			}
			obj := info.Uses[key]
			if obj != nil && obj.Pkg() != nil && strings.HasPrefix(obj.Pkg().Path(), "github.com/egoist/quickgui/go/") {
				child(value.Value)
			}
		}
		return len(failures) < 20
	})
	return errors.Join(failures...)
}
