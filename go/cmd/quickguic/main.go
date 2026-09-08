// Command quickguic prepares source overlays for QuickGUI's component compiler.
package main

import (
	"flag"
	"fmt"
	"os"
	"runtime"

	"github.com/egoist/quickgui/go/internal/viewcompile"
)

func main() {
	project := flag.String("project", ".", "application directory")
	output := flag.String("out", ".quickgui/go-sources", "generated source directory")
	tests := flag.Bool("tests", false, "include application tests")
	tags := flag.String("tags", "", "Go build tags")
	goos := flag.String("goos", runtime.GOOS, "application target OS")
	goarch := flag.String("goarch", runtime.GOARCH, "application target architecture")
	flag.Parse()
	patterns := flag.Args()
	if len(patterns) == 0 {
		patterns = []string{"./..."}
	}
	path, err := viewcompile.Prepare(*project, *output, patterns, *tests, *tags, *goos, *goarch)
	if err != nil {
		fmt.Fprintln(os.Stderr, err)
		os.Exit(1)
	}
	fmt.Println(path)
}
