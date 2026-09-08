package viewcompile

import (
	"bytes"
	"crypto/sha256"
	"encoding/json"
	"fmt"
	"go/ast"
	"go/importer"
	"go/parser"
	"go/token"
	"go/types"
	"io"
	"os"
	"os/exec"
	"path/filepath"
	"sort"
	"strings"
)

type packageData struct {
	Dir, ImportPath, Name, Export                                          string
	GoFiles, TestGoFiles, XTestGoFiles, Imports, TestImports, XTestImports []string
	Standard                                                               bool
	Module                                                                 *struct {
		Main    bool
		Replace *struct{ Version string }
	}
}
type sourcePackage struct {
	data    packageData
	files   []*ast.File
	inputs  map[*ast.File][]byte
	info    *types.Info
	pkg     *types.Package
	loading bool
}
type sourceImporter struct {
	load func(string) (*types.Package, error)
}

func (i sourceImporter) Import(path string) (*types.Package, error) { return i.load(path) }

func list(directory string, environment []string, flags []string, patterns []string) ([]packageData, error) {
	args := append([]string{"list", "-deps", "-json"}, flags...)
	cmd := exec.Command("go", append(args, patterns...)...)
	cmd.Dir, cmd.Env = directory, environment
	var stderr bytes.Buffer
	cmd.Stderr = &stderr
	output, err := cmd.Output()
	if err != nil {
		return nil, fmt.Errorf("go list: %w\n%s", err, stderr.String())
	}
	decoder := json.NewDecoder(bytes.NewReader(output))
	var packages []packageData
	for {
		var pkg packageData
		err := decoder.Decode(&pkg)
		if err == io.EOF {
			break
		}
		if err != nil {
			return nil, err
		}
		packages = append(packages, pkg)
	}
	return packages, nil
}

// Prepare emits Go's native overlay manifest. Only the application and local
// replacement/workspace packages are transformed; SDK and registry sources keep
// their normal compilation and cache identity.
func Prepare(directory, output string, patterns []string, tests bool, tags, goos, goarch string) (string, error) {
	directory, err := filepath.Abs(directory)
	if err != nil {
		return "", err
	}
	output, err = filepath.Abs(output)
	if err != nil {
		return "", err
	}
	environment := append(os.Environ(), "CGO_ENABLED=0", "GOOS="+goos, "GOARCH="+goarch)
	var flags []string
	if tags != "" {
		flags = append(flags, "-tags", tags)
	}
	listed, err := list(directory, environment, flags, patterns)
	if err != nil {
		return "", err
	}
	fs := token.NewFileSet()
	sources := map[string]*sourcePackage{}
	for _, data := range listed {
		if data.Standard || strings.HasPrefix(data.ImportPath, "github.com/egoist/quickgui/go/") || data.Module == nil || (!data.Module.Main && (data.Module.Replace == nil || data.Module.Replace.Version != "")) {
			continue
		}
		groups := []packageData{data}
		if tests {
			groups[0].GoFiles = append(append([]string(nil), data.GoFiles...), data.TestGoFiles...)
			groups[0].Imports = append(append([]string(nil), data.Imports...), data.TestImports...)
			if len(data.XTestGoFiles) > 0 {
				x := data
				x.ImportPath += "_test"
				x.Name += "_test"
				x.GoFiles = data.XTestGoFiles
				x.Imports = data.XTestImports
				groups = append(groups, x)
			}
		}
		for _, group := range groups {
			pkg := &sourcePackage{data: group, inputs: map[*ast.File][]byte{}}
			for _, name := range group.GoFiles {
				path := filepath.Join(group.Dir, name)
				input, err := os.ReadFile(path)
				if err != nil {
					return "", err
				}
				file, err := parser.ParseFile(fs, path, input, parser.ParseComments|parser.SkipObjectResolution)
				if err != nil {
					return "", err
				}
				pkg.files = append(pkg.files, file)
				pkg.inputs[file] = input
			}
			sources[group.ImportPath] = pkg
		}
	}
	replacements := map[string]string{}
	if len(sources) > 0 {
		dependencies := map[string]bool{}
		for _, pkg := range sources {
			for _, path := range pkg.data.Imports {
				if sources[path] == nil && path != "unsafe" {
					dependencies[path] = true
				}
			}
		}
		var paths []string
		for path := range dependencies {
			paths = append(paths, path)
		}
		sort.Strings(paths)
		exports := map[string]string{}
		if len(paths) > 0 {
			compiled, err := list(directory, environment, append(append([]string(nil), flags...), "-export"), paths)
			if err != nil {
				return "", err
			}
			for _, pkg := range compiled {
				exports[pkg.ImportPath] = pkg.Export
			}
		}
		external := importer.ForCompiler(fs, "gc", func(path string) (io.ReadCloser, error) { return os.Open(exports[path]) })
		var load func(string) (*types.Package, error)
		load = func(path string) (*types.Package, error) {
			pkg := sources[path]
			if pkg == nil {
				return external.Import(path)
			}
			if pkg.pkg != nil {
				return pkg.pkg, nil
			}
			if pkg.loading {
				return nil, fmt.Errorf("import cycle involving %s", path)
			}
			pkg.loading = true
			pkg.info = &types.Info{Types: map[ast.Expr]types.TypeAndValue{}, Defs: map[*ast.Ident]types.Object{}, Uses: map[*ast.Ident]types.Object{}, Implicits: map[ast.Node]types.Object{}, Selections: map[*ast.SelectorExpr]*types.Selection{}, Instances: map[*ast.Ident]types.Instance{}}
			config := types.Config{Importer: sourceImporter{load}, Sizes: types.SizesFor("gc", goarch)}
			checked, err := config.Check(path, fs, pkg.files, pkg.info)
			if err != nil {
				return nil, err
			}
			pkg.pkg = checked
			return checked, nil
		}
		components := map[string]Component{}
		for path := range sources {
			if _, err := load(path); err != nil {
				return "", err
			}
		}
		for path, pkg := range sources {
			for _, file := range pkg.files {
				if ast.IsGenerated(file) {
					continue
				}
				for _, decl := range file.Decls {
					fn, ok := decl.(*ast.FuncDecl)
					if !ok || !IsComponent(fn, pkg.info) {
						continue
					}
					if _, exists := components[path+"."+fn.Name.Name]; exists {
						continue
					}
					component, err := Describe(fn, pkg.info)
					if err != nil {
						return "", fmt.Errorf("%s: %w", fs.Position(fn.Pos()), err)
					}
					if pkg.pkg.Scope().Lookup(component.Generated) != nil {
						return "", fmt.Errorf("%s: reserved generated name %s", fs.Position(fn.Pos()), component.Generated)
					}
					components[path+"."+component.Name] = component
				}
			}
		}
		for _, pkg := range sources {
			for _, file := range pkg.files {
				input := pkg.inputs[file]
				generated, err := Source(fs, file, input, pkg.pkg, pkg.info, components)
				if err != nil {
					return "", err
				}
				if bytes.Equal(input, generated) {
					continue
				}
				original := fs.PositionFor(file.Pos(), false).Filename
				sum := sha256.Sum256([]byte(original))
				target := filepath.Join(output, fmt.Sprintf("%x", sum[:8]), filepath.Base(original))
				if err := writeChanged(target, generated); err != nil {
					return "", err
				}
				replacements[original] = target
			}
		}
	}
	manifest, err := json.Marshal(struct{ Replace map[string]string }{replacements})
	if err != nil {
		return "", err
	}
	path := filepath.Join(output, "overlay.json")
	var previous struct{ Replace map[string]string }
	if data, err := os.ReadFile(path); err == nil {
		_ = json.Unmarshal(data, &previous)
	}
	if err := writeChanged(path, manifest); err != nil {
		return "", err
	}
	for original, old := range previous.Replace {
		if replacements[original] == old {
			continue
		}
		relative, err := filepath.Rel(output, old)
		if err == nil && !filepath.IsAbs(relative) && relative != ".." && !strings.HasPrefix(relative, ".."+string(filepath.Separator)) && filepath.Ext(old) == ".go" {
			_ = os.Remove(old)
			_ = os.Remove(filepath.Dir(old)) // Only empty generated directories disappear.
		}
	}
	return path, nil
}

func writeChanged(path string, data []byte) error {
	info, _ := os.Lstat(path)
	if existing, err := os.ReadFile(path); err == nil && info != nil && info.Mode()&os.ModeSymlink == 0 && bytes.Equal(existing, data) {
		return nil
	}
	if err := os.MkdirAll(filepath.Dir(path), 0755); err != nil {
		return err
	}
	temporary, err := os.CreateTemp(filepath.Dir(path), ".quickgui-")
	if err != nil {
		return err
	}
	defer os.Remove(temporary.Name())
	if _, err = temporary.Write(data); err != nil {
		temporary.Close()
		return err
	}
	if err = temporary.Chmod(0644); err != nil {
		temporary.Close()
		return err
	}
	if err = temporary.Close(); err != nil {
		return err
	}
	return os.Rename(temporary.Name(), path)
}
