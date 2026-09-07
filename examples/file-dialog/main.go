package main

import (
	"os"
	"path/filepath"
	"strings"

	"github.com/egoist/quickgui/go/native"
	"github.com/egoist/quickgui/go/ui"
)

func main() { run("Native file dialogs", 760, 580, FileDialogs) }

func FileDialogs() {
	window := native.CurrentWindow()
	status, setStatus := ui.CreateSignal("Choose files, a folder, or a save destination.")
	pending, setPending := ui.CreateSignal(false)
	complete := func(paths []string, err error) {
		if window.Closed {
			return
		}
		setPending(false)
		if err != nil {
			setStatus(err.Error())
		} else if len(paths) == 0 {
			setStatus("Canceled")
		} else {
			setStatus(strings.Join(paths, "\n"))
		}
	}
	open := func(folder bool) {
		setPending(true)
		properties := []string{"openFile", "multiSelections"}
		if folder {
			properties = []string{"openDirectory"}
		}
		native.ShowOpenDialog(
			native.OpenDialogOptions{
				Window:     window,
				Title:      "Choose a destination",
				Properties: properties,
			},
			func(result native.OpenDialogResult, err error) { complete(result.FilePaths, err) },
		)
	}
	save := func() {
		home, _ := os.UserHomeDir()
		setPending(true)
		native.ShowSaveDialog(
			native.SaveDialogOptions{
				Window:      window,
				Title:       "Save destination",
				DefaultPath: filepath.Join(home, "notes.txt"),
			},
			func(result native.SaveDialogResult, err error) {
				var paths []string
				if !result.Canceled {
					paths = []string{result.FilePath}
				}
				complete(paths, err)
			},
		)
	}
	var buttons []any
	for _, item := range []struct {
		label string
		click func()
	}{{"Open files", func() { open(false) }}, {"Open folder", func() { open(true) }}, {"Save as…", save}} {
		buttons = append(buttons, ui.Button(
			ui.WithStyle(buttonStyle),
			ui.Disabled(pending),
			ui.OnClick(func() { item.click() }),
			item.label,
		))
	}
	card(
		ui.View(ui.Display("flex"), ui.Gap(12), buttons),
		ui.Text(
			ui.Color("#94a3b8"),
			"The save panel chooses a path. This example does not write a file.",
		),
		ui.Text(ui.UserSelect("text"), status),
	)
}
