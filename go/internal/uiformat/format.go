// Package uiformat adds predictable line breaks to QuickGUI declarations before
// letting gofmt handle indentation, spacing, and alignment.
package uiformat

import (
	"bytes"
	"fmt"
	"go/ast"
	"go/format"
	"go/parser"
	"go/scanner"
	"go/token"
	"sort"
	"strconv"
	"strings"
)

const DefaultWidth = 100

// Source formats a complete Go file. Generated files are left to their generator.
// Width is a wrapping threshold for declarations, not a limit on string literals.
func Source(source []byte, width int) ([]byte, error) {
	if width <= 0 {
		return nil, fmt.Errorf("format width must be positive")
	}
	file, err := parser.ParseFile(token.NewFileSet(), "", source, parser.ParseComments|parser.SkipObjectResolution)
	if err != nil {
		return nil, err
	}
	if ast.IsGenerated(file) {
		return source, nil
	}
	current, err := format.Source(source)
	if err != nil {
		return nil, err
	}
	for pass := 0; pass < 16; pass++ {
		next, err := wrap(current, width)
		if err != nil {
			return nil, err
		}
		if bytes.Equal(current, next) {
			return next, nil
		}
		current = next
	}
	return nil, fmt.Errorf("UI formatting did not converge")
}

func wrap(source []byte, width int) ([]byte, error) {
	fs := token.NewFileSet()
	file, err := parser.ParseFile(fs, "", source, parser.ParseComments|parser.SkipObjectResolution)
	if err != nil {
		return nil, err
	}
	aliases := map[string]bool{}
	for _, spec := range file.Imports {
		path, _ := strconv.Unquote(spec.Path.Value)
		if path != "github.com/egoist/quickgui/go/ui" && path != "github.com/egoist/quickgui/go/native" {
			continue
		}
		name := path[strings.LastIndexByte(path, '/')+1:]
		if spec.Name != nil {
			name = spec.Name.Name
		}
		if name != "." && name != "_" {
			aliases[name] = true
		}
	}
	qualified := func(expr ast.Expr) bool {
		for {
			switch value := expr.(type) {
			case *ast.SelectorExpr:
				expr = value.X
			case *ast.IndexExpr:
				expr = value.X
			case *ast.IndexListExpr:
				expr = value.X
			default:
				id, ok := expr.(*ast.Ident)
				return ok && aliases[id.Name]
			}
		}
	}
	// Insert only whitespace and trailing commas. Keeping existing tokens in place
	// preserves comments, raw strings, and the meaning of callbacks and variadics.
	edits := map[int]string{}
	offset := func(pos token.Pos) int { return fs.Position(pos).Offset }
	breakBefore := func(previous, next token.Pos) {
		if fs.Position(previous).Line == fs.Position(next).Line {
			at := offset(next)
			if !strings.Contains(edits[at], "\n") {
				edits[at] += "\n"
			}
		}
	}
	longLine := func(start, end token.Pos) bool {
		from, to := offset(start), offset(end)
		from = bytes.LastIndexByte(source[:from], '\n') + 1
		for _, line := range bytes.Split(source[from:to], []byte{'\n'}) {
			if displayWidth(line) > width {
				return true
			}
		}
		return false
	}
	longCallback := func(fn *ast.FuncLit) bool {
		if fs.Position(fn.Body.Lbrace).Line != fs.Position(fn.Body.Rbrace).Line {
			return false
		}
		from, to := offset(fn.Pos()), offset(fn.End())
		line := source[bytes.LastIndexByte(source[:from], '\n')+1 : from]
		indent := line[:len(line)-len(bytes.TrimLeft(line, "\t "))]
		// Measure the callback on its own argument line, rather than counting the
		// preceding options that this pass will move onto separate lines.
		return displayWidth(indent)+4+displayWidth(source[from:to]) > width
	}
	split := func(open, close token.Pos, items []ast.Expr, lastEnd token.Pos) {
		breakBefore(open, items[0].Pos())
		for i := 1; i < len(items); i++ {
			breakBefore(items[i-1].End(), items[i].Pos())
		}
		if !hasComma(source[offset(lastEnd):offset(close)]) {
			at := offset(lastEnd)
			edits[at] = "," + edits[at]
		}
		breakBefore(lastEnd, close)
	}
	ast.Inspect(file, func(node ast.Node) bool {
		switch value := node.(type) {
		case *ast.CallExpr:
			if !qualified(value.Fun) || len(value.Args) == 0 {
				return true
			}
			callback := false
			for _, arg := range value.Args {
				if fn, ok := arg.(*ast.FuncLit); ok {
					callback = true
					if longCallback(fn) && len(fn.Body.List) > 0 {
						breakBefore(fn.Body.Lbrace, fn.Body.List[0].Pos())
						for i := 1; i < len(fn.Body.List); i++ {
							breakBefore(fn.Body.List[i-1].End(), fn.Body.List[i].Pos())
						}
						breakBefore(fn.Body.List[len(fn.Body.List)-1].End(), fn.Body.Rbrace)
					}
				}
			}
			multiline := fs.Position(value.Lparen).Line != fs.Position(value.Rparen).Line
			if len(value.Args) > 1 && (callback || multiline || longLine(value.Pos(), value.End())) {
				lastEnd := value.Args[len(value.Args)-1].End()
				if value.Ellipsis.IsValid() {
					lastEnd = value.Ellipsis + 3
				}
				split(value.Lparen, value.Rparen, value.Args, lastEnd)
			}
		case *ast.CompositeLit:
			if value.Type == nil || !qualified(value.Type) || len(value.Elts) < 2 {
				return true
			}
			multiline := fs.Position(value.Lbrace).Line != fs.Position(value.Rbrace).Line
			if multiline || longLine(value.Pos(), value.End()) {
				split(value.Lbrace, value.Rbrace, value.Elts, value.Elts[len(value.Elts)-1].End())
			}
		}
		return true
	})
	if len(edits) == 0 {
		return source, nil
	}
	positions := make([]int, 0, len(edits))
	for at := range edits {
		positions = append(positions, at)
	}
	sort.Ints(positions)
	var output bytes.Buffer
	previous := 0
	for _, at := range positions {
		output.Write(source[previous:at])
		output.WriteString(edits[at])
		previous = at
	}
	output.Write(source[previous:])
	return format.Source(output.Bytes())
}

func displayWidth(line []byte) int {
	columns := 0
	for _, char := range string(line) {
		if char == '\t' {
			columns += 4 - columns%4
		} else {
			columns++
		}
	}
	return columns
}

func hasComma(source []byte) bool {
	fs := token.NewFileSet()
	file := fs.AddFile("", -1, len(source))
	var scan scanner.Scanner
	scan.Init(file, source, nil, scanner.ScanComments)
	for {
		_, kind, _ := scan.Scan()
		if kind == token.COMMA {
			return true
		}
		if kind == token.EOF {
			return false
		}
	}
}
