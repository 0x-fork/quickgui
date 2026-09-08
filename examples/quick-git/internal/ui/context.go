package ui

import (
	"github.com/egoist/quickgui/go/native"
	"github.com/egoist/quickgui/go/reactive"
	gui "github.com/egoist/quickgui/go/ui"
	"quickgui.example/quick-git/internal/model"
)

type DialogKind string

const (
	DialogNone        DialogKind = ""
	DialogNewBranch   DialogKind = "new-branch"
	DialogNewWorktree DialogKind = "new-worktree"
	DialogStash       DialogKind = "stash"
)

type DialogRequest struct {
	Kind   DialogKind
	From   string
	Branch string
}

type AppContext struct {
	Store              *model.Store
	Window             *native.Window
	Theme              reactive.Accessor[Theme]
	Dialog             reactive.Accessor[DialogRequest]
	OpenDialog         func(DialogRequest)
	CloseDialog        func()
	OpenRepository     func()
	OpenRepositoryPath func(string)
}

var appContext = reactive.CreateContext[AppContext](AppContext{})

func ProvideApp(value AppContext, children func()) {
	appContext.Provide(value, children)
}

func UseApp() AppContext {
	value := reactive.UseContext(appContext)
	if value.Store == nil {
		panic("useApp must run inside AppProvider")
	}
	return value
}

func boolPtr(value bool) *bool { return &value }

func rowStyle(theme Theme, selected bool) gui.Style {
	background := "transparent"
	if selected {
		background = theme.Selection
	}
	return gui.Styles(
		gui.Display("flex"),
		gui.FlexDirection("row"),
		gui.Width("100%"),
		gui.MinWidth(0),
		gui.Height(28),
		gui.FlexShrink(0),
		gui.AlignItems("center"),
		gui.Gap(8),
		gui.PaddingLeft(10),
		gui.PaddingRight(8),
		gui.BorderRadius(6),
		gui.BackgroundColor(background),
		gui.TextColor(theme.Text),
		gui.Cursor("default"),
		gui.UserSelect("none"),
		gui.Hover(gui.BackgroundColor(theme.Hover)),
		gui.Active(gui.BackgroundColor(theme.Active)),
	)
}

func RepositoryLabels(paths []string) map[string]string {
	counts := map[string]int{}
	for _, path := range paths {
		counts[model.Basename(path)]++
	}
	labels := map[string]string{}
	for _, path := range paths {
		name := model.Basename(path)
		if counts[name] > 1 {
			labels[path] = model.Basename(model.Dirname(path)) + "/" + name
		} else {
			labels[path] = name
		}
	}
	return labels
}
