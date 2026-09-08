package ui

import (
	"github.com/egoist/quickgui/go/native"
	gui "github.com/egoist/quickgui/go/ui"
	"quickgui.example/quick-git/internal/git"
)

func Dialogs() {
	app := UseApp()
	gui.Show(
		func() bool { return app.Dialog().Kind != DialogNone },
		func() {
			switch app.Dialog().Kind {
			case DialogNewBranch:
				newBranchDialog()
				return
			case DialogNewWorktree:
				newWorktreeDialog()
				return
			case DialogStash:
				stashDialog()
				return
			default:
				return
			}
		},
	)
}

func dialogFrame(title, description string, body *native.Node, actions *native.Node) {
	app := UseApp()
	gui.Dialog.Root(
		gui.DialogRootProps{
			Open: func() bool { return true },
			OnOpenChange: func(open bool, _ gui.DialogOpenChangeDetails) {
				if !open {
					app.CloseDialog()
				}
			},
			ExitDuration: 0,
		},
		func() {
			gui.Dialog.Portal(
				gui.PartProps{
					Style: gui.Styles(
						gui.Position("absolute"),
						gui.Top(0),
						gui.Right(0),
						gui.Bottom(0),
						gui.Left(0),
						gui.Display("flex"),
						gui.AlignItems("center"),
						gui.JustifyContent("center"),
					),
				},
				func() {
					gui.Dialog.Backdrop(gui.PartProps{
						Style: gui.Styles(
							gui.Position("absolute"),
							gui.Top(0),
							gui.Right(0),
							gui.Bottom(0),
							gui.Left(0),
							gui.BackgroundColor(app.Theme().Scrim),
						),
					})
					gui.Dialog.Popup(
						gui.DialogPopupProps{
							PartProps: gui.PartProps{
								Style: gui.Styles(
									gui.Display("flex"),
									gui.FlexDirection("column"),
									gui.Width(440),
									gui.Gap(14),
									gui.Padding(20),
									gui.BackgroundColor(app.Theme().Raised),
									gui.BorderWidth(1),
									gui.BorderColor(app.Theme().BorderStrong),
									gui.BorderRadius(10),
								),
							},
						},
						func() {
							gui.Dialog.Title(
								gui.PartProps{},
								func() {
									gui.Text(
										title,
										gui.FontSize(15),
										gui.FontWeight(700),
										gui.Color(app.Theme().Text),
									)
								},
							)
							gui.Show(
								func() bool { return description != "" },
								func() {
									gui.Dialog.Description(
										gui.PartProps{},
										func() {
											gui.Text(
												description,
												gui.FontSize(12.5),
												gui.LineHeight(18),
												gui.Color(app.Theme().TextSecondary),
											)
										},
									)
								},
							)
							gui.Dialog.Viewport(
								gui.PartProps{
									Style: gui.Styles(
										gui.Display("flex"),
										gui.FlexDirection("column"),
										gui.Gap(12),
										gui.Padding(2),
									),
								},
								func() { gui.Child(body) },
							)
							gui.View(
								func() {
									gui.Dialog.Close(
										gui.PartProps{
											Style: app.Theme().Button("secondary"),
										},
										func() {
											gui.Text("Cancel")
										},
									)
									gui.Child(actions)
								},
								gui.Display("flex"),
								gui.FlexDirection("row"),
								gui.JustifyContent("flex-end"),
								gui.Gap(8),
								gui.MarginTop(4),
							)
						},
					)
				},
			)
		},
	)
}

func CheckRow(label string, checked func() bool, onChange func(bool)) {
	app := UseApp()
	gui.Checkbox.Root(
		gui.CheckboxProps{
			PartProps: gui.PartProps{
				Style: gui.Styles(
					gui.Display("flex"),
					gui.FlexDirection("row"),
					gui.AlignItems("center"),
					gui.Gap(8),
					gui.Height(24),
					gui.Cursor("default"),
					gui.UserSelect("none"),
					gui.BorderRadius(4),
					gui.Focus(gui.Outline("2px solid "+app.Theme().FocusRing)),
					gui.DisabledStyle(gui.Opacity(0.5)),
				),
			},
			Checked:         func() gui.CheckedState { return checked() },
			OnCheckedChange: func(next bool, _ *native.Event) { onChange(next) },
		},
		func() {
			gui.Checkbox.Indicator(
				gui.PartProps{
					Style: func() gui.Style { return checkboxBox(app.Theme(), checked()) },
				},
				func() {
					gui.Show(
						checked,
						func() { checkboxMark(true) },
					)
				},
			)
			gui.Text(label, gui.FontSize(12.5), gui.Color(app.Theme().Text))
		},
	)
}

func checkboxBox(theme Theme, checked bool) gui.Style {
	border := theme.InputBorder
	background := theme.Input
	if checked {
		border = theme.Accent
		background = theme.Accent
	}
	return gui.Styles(
		gui.Display("flex"),
		gui.Width(15),
		gui.Height(15),
		gui.FlexShrink(0),
		gui.AlignItems("center"),
		gui.JustifyContent("center"),
		gui.BorderRadius(3.5),
		gui.BorderWidth(1),
		gui.BorderColor(border),
		gui.BackgroundColor(background),
	)
}

func checkboxMark(checked bool) {
	if !checked {
		return
	}
	app := UseApp()
	icon(checkIcon, 12, func() string { return app.Theme().TextOnAccent })
}

func newBranchDialog() {
	app := UseApp()
	store := app.Store
	name, setName := gui.CreateSignal("")
	checkout, setCheckout := gui.CreateSignal(true)
	initial := app.Dialog().From
	if initial == "" {
		if status := store.Status(); status != nil && status.Branch != "" {
			initial = status.Branch
		} else {
			initial = "HEAD"
		}
	}
	base, setBase := gui.CreateSignal(initial)
	problem := func() string { return git.BranchNameProblem(name()) }
	exists := func() bool {
		for _, branch := range store.Refs().Local {
			if branch.Name == name() {
				return true
			}
		}
		return false
	}
	valid := func() bool { return name() != "" && problem() == "" && !exists() }
	submit := func() {
		if !valid() {
			return
		}
		app.CloseDialog()
		store.CreateBranch(name(), base(), checkout())
	}
	options := func() []gui.OptionDeclaration {
		seen := map[string]struct{}{"HEAD": {}, base(): {}}
		items := []gui.OptionDeclaration{{Value: "HEAD", Label: "HEAD"}}
		add := func(value string) {
			if _, ok := seen[value]; ok || value == "" {
				return
			}
			seen[value] = struct{}{}
			items = append(items, gui.OptionDeclaration{Value: value, Label: value})
		}
		for _, branch := range store.Refs().Local {
			add(branch.Name)
		}
		for _, branch := range store.Refs().Remote {
			add(branch.Name)
		}
		return items
	}
	dialogFrame("New Branch", "", gui.View(
		func() {
			gui.Text(
				"Name",
				gui.FontSize(12),
				gui.FontWeight(600),
				gui.Color(app.Theme().TextSecondary),
			)
			gui.Input(
				gui.Placeholder("feature/great-idea"),
				gui.Value(func() string { return name() }),
				gui.OnInput(func(event *native.Event) { setName(event.Value) }),
				gui.OnSubmit(func(*native.Event) { submit() }),
				app.Theme().InputStyle(),
			)
			gui.Show(
				func() bool { return problem() != "" || exists() },
				func() {
					text := problem()
					if text == "" {
						text = "A branch with this name already exists."
					}
					gui.Text(text, gui.FontSize(11.5), gui.Color(app.Theme().Danger))
				},
			)
			gui.Text(
				"Based on",
				gui.FontSize(12),
				gui.FontWeight(600),
				gui.Color(app.Theme().TextSecondary),
			)
			gui.Select.Root(
				gui.SelectRootProps{
					PickerSourceProps: gui.PickerSourceProps{
						PartProps: gui.PartProps{
							AriaLabel: "Base branch",
							Style: gui.Styles(
								app.Theme().InputStyle(),
								gui.FlexDirection("row"),
								gui.AlignItems("center"),
								gui.JustifyContent("space-between"),
								gui.Gap(8),
							),
						},
						Items: options,
					},
					Value: func() *string {
						value := base()
						return &value
					},
					OnValueChange: func(value *string, _ *native.Event) {
						if value != nil {
							setBase(*value)
						}
					},
				},
				func() {
					gui.Text(
						func() string { return base() },
						gui.FontSize(13),
						gui.Color(app.Theme().Text),
					)
				},
			)
			CheckRow("Switch to the new branch", checkout, setCheckout)
		},
		gui.Display("flex"),
		gui.FlexDirection("column"),
		gui.Gap(10),
	), gui.Button(
		func() string {
			if checkout() {
				return "Create and Switch"
			}
			return "Create"
		},
		gui.Disabled(!valid()),
		gui.OnClick(func() { submit() }),
		app.Theme().Button("primary"),
	))
}

func newWorktreeDialog() {
	app := UseApp()
	store := app.Store
	initialBranch := app.Dialog().Branch
	branch, setBranch := gui.CreateSignal(initialBranch)
	createNew, setCreateNew := gui.CreateSignal(initialBranch == "")
	path, setPath := gui.CreateSignal(store.SuggestWorktreePath(initialBranch))
	problem := func() string {
		if !createNew() {
			return ""
		}
		return git.BranchNameProblem(branch())
	}
	submit := func() {
		if createNew() && problem() != "" {
			return
		}
		app.CloseDialog()
		if createNew() {
			store.AddWorktree(path(), branch(), "", "")
			return
		}
		store.AddWorktree(path(), "", branch(), "")
	}
	dialogFrame("New Worktree", "Adds a linked working tree beside this repository.", gui.View(
		func() {
			CheckRow("Create a new branch", createNew, setCreateNew)
			gui.Text(
				"Branch",
				gui.FontSize(12),
				gui.FontWeight(600),
				gui.Color(app.Theme().TextSecondary),
			)
			gui.Input(
				gui.Value(func() string { return branch() }),
				gui.OnInput(func(event *native.Event) {
					setBranch(event.Value)
					setPath(store.SuggestWorktreePath(event.Value))
				}),
				app.Theme().InputStyle(),
			)
			gui.Show(
				func() bool { return problem() != "" },
				func() {
					gui.Text(gui.FontSize(11.5), gui.Color(app.Theme().Danger), problem())
				},
			)
			gui.Text(
				"Path",
				gui.FontSize(12),
				gui.FontWeight(600),
				gui.Color(app.Theme().TextSecondary),
			)
			gui.Input(
				gui.Value(func() string { return path() }),
				gui.OnInput(func(event *native.Event) { setPath(event.Value) }),
				app.Theme().InputStyle(),
			)
		},
		gui.Display("flex"),
		gui.FlexDirection("column"),
		gui.Gap(10),
	), gui.Button(
		"Add Worktree",
		gui.OnClick(func() { submit() }),
		app.Theme().Button("primary"),
	))
}

func stashDialog() {
	app := UseApp()
	store := app.Store
	message, setMessage := gui.CreateSignal("")
	include, setInclude := gui.CreateSignal(true)
	submit := func() {
		app.CloseDialog()
		store.StashPush(message(), include())
	}
	dialogFrame("Stash Changes", "Saves local changes and returns the working tree to HEAD.", gui.View(
		func() {
			gui.Input(
				gui.Placeholder("Optional message"),
				gui.Value(func() string { return message() }),
				gui.OnInput(func(event *native.Event) { setMessage(event.Value) }),
				gui.OnSubmit(func(*native.Event) { submit() }),
				app.Theme().InputStyle(),
			)
			CheckRow("Include untracked files", include, setInclude)
		},
		gui.Display("flex"),
		gui.FlexDirection("column"),
		gui.Gap(10),
	), gui.Button(
		"Stash",
		gui.OnClick(func() { submit() }),
		app.Theme().Button("primary"),
	))
}
