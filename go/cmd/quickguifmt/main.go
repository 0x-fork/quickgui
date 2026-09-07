// Command quickguifmt formats Go source with QuickGUI declaration wrapping.
package main

import (
	"bytes"
	"flag"
	"fmt"
	"io"
	"io/fs"
	"os"
	"path/filepath"
	"sort"
	"strings"

	"github.com/egoist/quickgui/go/internal/uiformat"
)

func main() {
	os.Exit(run(os.Args[1:], os.Stdin, os.Stdout, os.Stderr))
}

func run(args []string, stdin io.Reader, stdout, stderr io.Writer) int {
	flags := flag.NewFlagSet("quickguifmt", flag.ContinueOnError)
	flags.SetOutput(stderr)
	write := flags.Bool("w", false, "write formatted source back to files")
	check := flags.Bool("check", false, "list files needing formatting and exit with status 1")
	width := flags.Int("width", uiformat.DefaultWidth, "UI declaration wrapping threshold")
	if err := flags.Parse(args); err != nil {
		if err == flag.ErrHelp {
			return 0
		}
		return 2
	}
	fail := func(err error) int { fmt.Fprintln(stderr, err); return 2 }
	if *width <= 0 || (*write && *check) {
		return fail(fmt.Errorf("use a positive width and only one of -w or -check"))
	}
	if flags.NArg() == 0 {
		if *write || *check {
			return fail(fmt.Errorf("-w and -check require a file or directory"))
		}
		source, err := io.ReadAll(stdin)
		if err != nil {
			return fail(err)
		}
		result, err := uiformat.Source(source, *width)
		if err != nil {
			return fail(err)
		}
		if _, err := stdout.Write(result); err != nil {
			return fail(err)
		}
		return 0
	}
	paths, err := sourceFiles(flags.Args())
	if err != nil {
		return fail(err)
	}
	// Parse every file before writing any, so a syntax error cannot leave a partial
	// formatting pass. File permissions survive the write.
	type change struct {
		path string
		data []byte
	}
	var changes []change
	for _, path := range paths {
		source, err := os.ReadFile(path)
		if err != nil {
			return fail(err)
		}
		result, err := uiformat.Source(source, *width)
		if err != nil {
			return fail(fmt.Errorf("%s: %w", path, err))
		}
		if !*write && !*check {
			if _, err := stdout.Write(result); err != nil {
				return fail(err)
			}
		} else if !bytes.Equal(source, result) {
			changes = append(changes, change{path, result})
		}
	}
	for _, change := range changes {
		if *write {
			if err := os.WriteFile(change.path, change.data, 0); err != nil {
				return fail(err)
			}
		}
		fmt.Fprintln(stdout, change.path)
	}
	if *check && len(changes) > 0 {
		return 1
	}
	return 0
}

func sourceFiles(roots []string) ([]string, error) {
	seen := map[string]bool{}
	var files []string
	for _, root := range roots {
		err := filepath.WalkDir(root, func(path string, entry fs.DirEntry, err error) error {
			if err != nil {
				return err
			}
			if entry.IsDir() {
				name := entry.Name()
				if path != root && (strings.HasPrefix(name, ".") || name == "node_modules" || name == "vendor") {
					return filepath.SkipDir
				}
				return nil
			}
			if entry.Type().IsRegular() && strings.HasSuffix(path, ".go") {
				absolute, err := filepath.Abs(path)
				if err != nil {
					return err
				}
				if !seen[absolute] {
					seen[absolute] = true
					files = append(files, path)
				}
			}
			return nil
		})
		if err != nil {
			return nil, err
		}
	}
	sort.Strings(files)
	return files, nil
}
