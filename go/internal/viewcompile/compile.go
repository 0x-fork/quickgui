// Package viewcompile lowers native components into fine-grained
// accessors. It operates on a build overlay and never changes application source.
package viewcompile

import (
	"fmt"
	"go/ast"
	"go/token"
	"go/types"
	"sort"
	"strconv"
	"strings"
)

const uiPath = "github.com/egoist/quickgui/go/ui"

type Component struct {
	Name      string
	Generated string
	Lazy      []bool
}

// IsComponent uses the declared native return type as the component boundary.
func IsComponent(fn *ast.FuncDecl, info *types.Info) bool {
	if fn.Recv != nil || fn.Body == nil {
		return false
	}
	sig := info.Defs[fn.Name].Type().(*types.Signature)
	return sig.Results().Len() == 1 && element(sig.Results().At(0).Type())
}

func Describe(fn *ast.FuncDecl, info *types.Info) (Component, error) {
	c := Component{Name: fn.Name.Name, Generated: "QuickguiComponent_" + fn.Name.Name}
	if !ast.IsExported(fn.Name.Name) {
		c.Generated = "quickguiComponent_" + fn.Name.Name
	}
	if fn.Recv != nil || fn.Body == nil {
		return c, fmt.Errorf("reactive components require a top-level function with a body")
	}
	sig := info.Defs[fn.Name].Type().(*types.Signature)
	if sig.Results().Len() != 1 || !element(sig.Results().At(0).Type()) {
		return c, fmt.Errorf("components must return *ui.Element or *native.Node")
	}
	for i := 0; i < sig.Params().Len(); i++ {
		param := sig.Params().At(i)
		if param.Name() == "" || param.Name() == "_" {
			return c, fmt.Errorf("component parameters must be named")
		}
		_, callback := param.Type().Underlying().(*types.Signature)
		c.Lazy = append(c.Lazy, !callback && !element(param.Type()) && !(sig.Variadic() && i == sig.Params().Len()-1))
	}
	// Parameters used as local storage keep ordinary Go value semantics. This
	// includes taking an optional value's address and modifying a setup copy.
	storage := func(expr ast.Expr) {
		ast.Inspect(expr, func(node ast.Node) bool {
			id, ok := node.(*ast.Ident)
			if !ok {
				return true
			}
			for i := 0; i < sig.Params().Len(); i++ {
				if info.Uses[id] == sig.Params().At(i) {
					c.Lazy[i] = false
				}
			}
			return true
		})
	}
	ast.Inspect(fn.Body, func(node ast.Node) bool {
		switch n := node.(type) {
		case *ast.UnaryExpr:
			if n.Op == token.AND {
				storage(n.X)
			}
		case *ast.AssignStmt:
			for _, lhs := range n.Lhs {
				storage(lhs)
			}
		case *ast.IncDecStmt:
			storage(n.X)
		}
		return true
	})
	return c, nil
}

func element(t types.Type) bool {
	ptr, ok := types.Unalias(t).(*types.Pointer)
	if !ok {
		return false
	}
	named, ok := types.Unalias(ptr.Elem()).(*types.Named)
	return ok && named.Obj().Pkg() != nil && (named.Obj().Pkg().Path() == uiPath && named.Obj().Name() == "Element" || named.Obj().Pkg().Path() == "github.com/egoist/quickgui/go/native" && named.Obj().Name() == "Node")
}

type edit struct {
	start, end int
	text       string
}

// Source uses type-checked object identities, including shadowing and renamed
// imports, so field names and unrelated functions cannot become prop reads.
func Source(fs *token.FileSet, file *ast.File, source []byte, pkg *types.Package, info *types.Info, components map[string]Component) ([]byte, error) {
	if ast.IsGenerated(file) {
		return source, nil
	}
	if err := Validate(fs, file, info); err != nil {
		return nil, err
	}
	var edits []edit
	offset := func(pos token.Pos) int { return fs.PositionFor(pos, false).Offset }
	render := func(start, end int) string {
		selected := append([]edit(nil), edits...)
		sort.Slice(selected, func(i, j int) bool { return selected[i].start < selected[j].start })
		var out strings.Builder
		cursor := start
		for _, e := range selected {
			if e.start < start || e.end > end {
				continue
			}
			out.Write(source[cursor:e.start])
			out.WriteString(e.text)
			cursor = e.end
		}
		out.Write(source[cursor:end])
		return out.String()
	}
	replace := func(node ast.Node, value string) {
		start, end := offset(node.Pos()), offset(node.End())
		if _, expression := node.(ast.Expr); expression {
			from, to := fs.PositionFor(node.Pos(), false), fs.PositionFor(node.End(), false)
			value = fmt.Sprintf("/*line %s:%d:%d*/%s/*line %s:%d:%d*/", from.Filename, from.Line, from.Column, value, to.Filename, to.Line, to.Column)
		}
		kept := edits[:0]
		for _, e := range edits {
			if e.start < start || e.end > end {
				kept = append(kept, e)
			}
		}
		edits = append(kept, edit{start, end, value})
	}
	aliases := map[string]string{pkg.Path(): ""}
	for _, spec := range file.Imports {
		path, _ := strconv.Unquote(spec.Path.Value)
		if spec.Name != nil {
			aliases[path] = spec.Name.Name
		} else if obj, ok := info.Implicits[spec].(*types.PkgName); ok {
			aliases[path] = obj.Imported().Name()
		}
	}
	var problem error
	additionalImports := map[string]string{}
	qualifier := func(other *types.Package) string {
		if alias, ok := aliases[other.Path()]; ok && alias != "_" {
			if alias == "." {
				return ""
			}
			return alias
		}
		alias := "quickguiType_" + other.Name()
		for strings.Contains(string(source), alias) {
			alias += "_"
		}
		aliases[other.Path()] = alias
		additionalImports[other.Path()] = alias
		return alias
	}
	typeName := func(t types.Type) string { return types.TypeString(types.Default(t), qualifier) }
	text := func(node ast.Node) string { return render(offset(node.Pos()), offset(node.End())) }
	dynamic := func(expr ast.Expr, props map[types.Object]bool) bool {
		found := false
		ast.Inspect(expr, func(node ast.Node) bool {
			if found {
				return false
			}
			switch n := node.(type) {
			case *ast.FuncLit:
				return false
			case *ast.CallExpr:
				if !info.Types[n.Fun].IsType() {
					found = true
					return false
				}
			case *ast.Ident:
				found = props[info.Uses[n]]
			}
			return true
		})
		return found
	}
	var walk func(ast.Node, map[types.Object]bool, bool)
	walk = func(root ast.Node, props map[types.Object]bool, bind bool) {
		callbacks := map[*ast.FuncLit]bool{}
		localFunctions := map[types.Object]*ast.FuncLit{}
		ast.Inspect(root, func(node ast.Node) bool {
			if assignment, ok := node.(*ast.AssignStmt); ok {
				for i, rhs := range assignment.Rhs {
					if fn, ok := rhs.(*ast.FuncLit); ok && i < len(assignment.Lhs) {
						if id, ok := assignment.Lhs[i].(*ast.Ident); ok {
							localFunctions[info.Defs[id]] = fn
						}
					}
				}
			}
			return true
		})
		mark := func(expr ast.Expr) {
			if fn, ok := expr.(*ast.FuncLit); ok {
				callbacks[fn] = true
			}
			if id, ok := expr.(*ast.Ident); ok {
				if fn := localFunctions[info.Uses[id]]; fn != nil {
					callbacks[fn] = true
				}
			}
		}
		isCallback := func(name string) bool {
			return strings.HasPrefix(name, "On") || name == "Ref" || name == "CreateEffect" || name == "CreateRenderEffect" || name == "Untrack"
		}
		ast.Inspect(root, func(node ast.Node) bool {
			switch value := node.(type) {
			case *ast.CallExpr:
				obj := calledObject(value.Fun, info)
				if obj != nil && obj.Pkg() != nil && obj.Pkg().Path() == uiPath && isCallback(obj.Name()) {
					for _, arg := range value.Args {
						mark(arg)
					}
				}
			case *ast.KeyValueExpr:
				if key, ok := value.Key.(*ast.Ident); ok {
					if obj := info.Uses[key]; obj != nil && obj.Pkg() != nil && obj.Pkg().Path() == uiPath && isCallback(key.Name) {
						mark(value.Value)
					}
				}
			}
			return true
		})
		var stack []ast.Node
		ast.Inspect(root, func(node ast.Node) bool {
			if problem != nil {
				return false
			}
			if node != nil {
				stack = append(stack, node)
				return true
			}
			node = stack[len(stack)-1]
			stack = stack[:len(stack)-1]
			switch value := node.(type) {
			case *ast.Ident:
				if props[info.Uses[value]] {
					replace(value, value.Name+"()")
				}
			case *ast.AssignStmt:
				for _, lhs := range value.Lhs {
					ast.Inspect(lhs, func(n ast.Node) bool {
						if id, ok := n.(*ast.Ident); ok && props[info.Uses[id]] {
							problem = fmt.Errorf("%s: component props are read-only; copy the value into a local variable before changing it", fs.Position(id.Pos()))
						}
						return true
					})
				}
			case *ast.IncDecStmt:
				ast.Inspect(value.X, func(n ast.Node) bool {
					if id, ok := n.(*ast.Ident); ok && props[info.Uses[id]] {
						problem = fmt.Errorf("%s: component props are read-only", fs.Position(id.Pos()))
					}
					return true
				})
			case *ast.CallExpr:
				obj := calledObject(value.Fun, info)
				if component, ok := components[componentKey(obj)]; ok {
					sig, _ := info.TypeOf(value.Fun).(*types.Signature)
					base := value.Fun
					for {
						parenthesized, ok := base.(*ast.ParenExpr)
						if !ok {
							break
						}
						base = parenthesized.X
					}
					typeArgs := ""
					switch indexed := base.(type) {
					case *ast.IndexExpr:
						base = indexed.X
						typeArgs = string(source[offset(indexed.X.End()):offset(indexed.End())])
					case *ast.IndexListExpr:
						base = indexed.X
						typeArgs = string(source[offset(indexed.X.End()):offset(indexed.End())])
					}
					var identifier *ast.Ident
					switch name := base.(type) {
					case *ast.Ident:
						identifier = name
					case *ast.SelectorExpr:
						identifier = name.Sel
					}
					if instance, ok := info.Instances[identifier]; ok {
						sig, _ = instance.Type.(*types.Signature)
					}
					if sig == nil || (!sig.Variadic() && len(value.Args) != len(component.Lazy)) || (sig.Variadic() && len(value.Args) < sig.Params().Len()-1) {
						problem = fmt.Errorf("%s: unsupported component argument list", fs.Position(value.Pos()))
						break
					}
					name := component.Generated + typeArgs
					if selector, ok := base.(*ast.SelectorExpr); ok {
						name = text(selector.X) + "." + name
					}
					var arguments, setup []string
					for i := 0; i < sig.Params().Len(); i++ {
						typ := typeName(sig.Params().At(i).Type())
						part, live := "", false
						if sig.Variadic() && i == sig.Params().Len()-1 && value.Ellipsis == token.NoPos {
							var items []string
							for _, arg := range value.Args[i:] {
								items = append(items, text(arg))
							}
							part = typ + "{" + strings.Join(items, ", ") + "}"
						} else {
							part = text(value.Args[i])
							live = dynamic(value.Args[i], props)
						}
						if !component.Lazy[i] || !live {
							temp := fmt.Sprintf("quickguiArg%d_%d", offset(value.Pos()), i)
							for strings.Contains(string(source), temp) {
								temp += "_"
							}
							setup = append(setup, "var "+temp+" "+typ+" = "+part)
							part = temp
						}
						if component.Lazy[i] {
							part = "func() " + typ + " { return " + part + " }"
						}
						arguments = append(arguments, part)
					}
					invocation := name + "(" + strings.Join(arguments, ", ") + ")"
					if len(setup) != 0 {
						invocation = "func() " + typeName(sig.Results().At(0).Type()) + " { " + strings.Join(setup, "; ") + "; return " + invocation + " }()"
					}
					replace(value, invocation)
					break
				}
				if !bind || obj == nil || obj.Pkg() == nil || obj.Pkg().Path() != uiPath {
					break
				}
				sig, ok := info.TypeOf(value.Fun).(*types.Signature)
				if !ok {
					break
				}
				// Constructors and fluent modifiers already accept typed accessors.
				if sig.Results().Len() != 1 {
					break
				}
				result := sig.Results().At(0).Type()
				style := false
				if named, ok := result.(*types.Named); ok {
					style = named.Obj().Pkg().Path() == uiPath && named.Obj().Name() == "StyleBuilder"
				}
				instance := compoundInstanceType(result)
				if !element(result) && !style && !instance {
					break
				}
				inCallback := false
				for _, ancestor := range stack {
					if fn, ok := ancestor.(*ast.FuncLit); ok && callbacks[fn] {
						inCallback = true
						break
					}
				}
				if inCallback {
					if selector, ok := value.Fun.(*ast.SelectorExpr); ok && info.Selections[selector] != nil {
						fresh := false
						for receiver := selector.X; ; {
							call, ok := receiver.(*ast.CallExpr)
							if !ok {
								break
							}
							if method, ok := call.Fun.(*ast.SelectorExpr); ok && info.Selections[method] != nil {
								receiver = method.X
								continue
							}
							fresh = element(info.TypeOf(call)) || compoundInstanceType(info.TypeOf(call))
							break
						}
						if !fresh {
							break
						}
					}
				}
				for i, arg := range value.Args {
					basic, ok := types.Default(info.TypeOf(arg)).(*types.Basic)
					if !ok || basic.Info()&(types.IsString|types.IsNumeric|types.IsBoolean) == 0 || !dynamic(arg, props) {
						continue
					}
					index := i
					if sig.Variadic() && index >= sig.Params().Len()-1 {
						index = sig.Params().Len() - 1
					}
					if index < 0 {
						continue
					}
					param := sig.Params().At(index).Type()
					if sig.Variadic() && index == sig.Params().Len()-1 {
						param = param.(*types.Slice).Elem()
					}
					if _, ok := param.Underlying().(*types.Interface); !ok {
						continue
					}
					replace(arg, "func() "+typeName(basic)+" { return "+text(arg)+" }")
				}
			}
			return true
		})
	}
	for _, decl := range file.Decls {
		fn, ok := decl.(*ast.FuncDecl)
		component, compiled := Component{}, false
		if ok && fn.Recv == nil {
			component, compiled = components[pkg.Path()+"."+fn.Name.Name]
		}
		if !compiled {
			// Native view expressions bind independently of component discovery.
			walk(decl, nil, true)
			continue
		}
		sig := info.Defs[fn.Name].Type().(*types.Signature)
		props := map[types.Object]bool{}
		var params, arguments []string
		for i := 0; i < sig.Params().Len(); i++ {
			param := sig.Params().At(i)
			typ := typeName(param.Type())
			arg := param.Name()
			if component.Lazy[i] {
				props[param] = true
				typ = "func() " + typ
				arg = "func() " + typeName(param.Type()) + " { return " + arg + " }"
			}
			params = append(params, param.Name()+" "+typ)
			arguments = append(arguments, arg)
		}
		walk(fn.Body, props, true)
		body := text(fn.Body)
		header := string(source[offset(fn.Pos()):offset(fn.Body.Pos())])
		typeParameters := ""
		if fn.Type.TypeParams != nil {
			typeParameters = string(source[offset(fn.Type.TypeParams.Pos()):offset(fn.Type.TypeParams.End())])
		}
		// Reset the source location for the generated implementation. Expression
		// rewrites preserve physical newlines inside the original body.
		location := fs.PositionFor(fn.Body.Pos(), false)
		end := fs.PositionFor(fn.End(), false)
		generated := header + "{ return " + component.Generated + "(" + strings.Join(arguments, ", ") + ") }\n\nfunc " + component.Generated + typeParameters + "(" + strings.Join(params, ", ") + ") " + typeName(sig.Results().At(0).Type()) + " " + fmt.Sprintf("/*line %s:%d:%d*/", location.Filename, location.Line, location.Column) + body + fmt.Sprintf("/*line %s:%d:%d*/", end.Filename, end.Line, end.Column)
		replace(fn, generated)
	}
	if problem != nil {
		return nil, problem
	}
	if len(additionalImports) != 0 {
		var paths []string
		for path := range additionalImports {
			paths = append(paths, path)
		}
		sort.Strings(paths)
		block := "\nimport (\n"
		for _, path := range paths {
			block += additionalImports[path] + " " + strconv.Quote(path) + "\n"
		}
		block += ")\n"
		position := fs.PositionFor(file.Name.End(), false)
		block += fmt.Sprintf("/*line %s:%d:%d*/", position.Filename, position.Line, position.Column)
		edits = append(edits, edit{offset(file.Name.End()), offset(file.Name.End()), block})
	}
	if len(edits) == 0 {
		return source, nil
	}
	filename := fs.PositionFor(file.Pos(), false).Filename
	return []byte(fmt.Sprintf("//line %s:1:1\n", filename) + render(0, len(source))), nil
}

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

// Compound descriptors expose retained settings before their Root is inserted.
func compoundInstanceType(typ types.Type) bool {
	ptr, ok := types.Unalias(typ).(*types.Pointer)
	if !ok {
		return false
	}
	named, ok := types.Unalias(ptr.Elem()).(*types.Named)
	return ok && named.Obj().Pkg() != nil && named.Obj().Pkg().Path() == uiPath && strings.HasSuffix(named.Obj().Name(), "Component")
}
