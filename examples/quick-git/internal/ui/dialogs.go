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

func dialogFrame(title, description string, body *gui.Element, actions *gui.Element) {
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
					Style: gui.Style().
						Position("absolute").
						Top(0).
						Right(0).
						Bottom(0).
						Left(0).
						Display("flex").
						AlignItems("center").
						JustifyContent("center"),
				},
				func() {
					gui.Dialog.Backdrop(gui.PartProps{
						Style: gui.Style().
							Position("absolute").
							Top(0).
							Right(0).
							Bottom(0).
							Left(0).
							BackgroundColor(app.Theme().Scrim),
					})
					gui.Dialog.Popup(
						gui.DialogPopupProps{
							PartProps: gui.PartProps{
								Style: gui.Style().
									Display("flex").
									FlexDirection("column").
									Width(440).
									Gap(14).
									Padding(20).
									BackgroundColor(app.Theme().Raised).
									BorderWidth(1).
									BorderColor(app.Theme().BorderStrong).
									BorderRadius(10),
							},
						},
						func() {
							gui.Dialog.Title(
								gui.PartProps{},
								func() {
									gui.Text(
										title,
									).FontSize(15).FontWeight(700).TextColor(app.Theme().Text)
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
											).FontSize(12.5).LineHeight(18).TextColor(app.Theme().TextSecondary)
										},
									)
								},
							)
							gui.Dialog.Viewport(
								gui.PartProps{
									Style: gui.Style().
										Display("flex").
										FlexDirection("column").
										Gap(12).
										Padding(2),
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
							).Display("flex").FlexDirection("row").JustifyContent("flex-end").Gap(8).MarginTop(4)
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
				Style: gui.Style().
					Display("flex").
					FlexDirection("row").
					AlignItems("center").
					Gap(8).
					Height(24).
					Cursor("default").
					UserSelect("none").
					BorderRadius(4).
					FocusStyle(func(s gui.StyleBuilder) gui.StyleBuilder {
						return s.Outline("2px solid " + app.Theme().FocusRing)
					}).
					DisabledStyle(func(s gui.StyleBuilder) gui.StyleBuilder { return s.Opacity(0.5) }),
			},
			Checked:         func() gui.CheckedState { return checked() },
			OnCheckedChange: func(next bool, _ *native.Event) { onChange(next) },
		},
		func() {
			gui.Checkbox.Indicator(
				gui.PartProps{
					Style: func() gui.StyleBuilder { return checkboxBox(app.Theme(), checked()) },
				},
				func() {
					gui.Show(
						checked,
						func() { checkboxMark(true) },
					)
				},
			)
			gui.Text(label).FontSize(12.5).TextColor(app.Theme().Text)
		},
	)
}

func checkboxBox(theme Theme, checked bool) gui.StyleBuilder {
	border := theme.InputBorder
	background := theme.Input
	if checked {
		border = theme.Accent
		background = theme.Accent
	}
	return gui.Style().
		Display("flex").
		Width(15).
		Height(15).
		FlexShrink(0).
		AlignItems("center").
		JustifyContent("center").
		BorderRadius(3.5).
		BorderWidth(1).
		BorderColor(border).
		BackgroundColor(background)
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
			).FontSize(12).FontWeight(600).TextColor(app.Theme().TextSecondary)
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
					gui.Text(
						text,
					).FontSize(11.5).TextColor(app.Theme().Danger)
				},
			)
			gui.Text(
				"Based on",
			).FontSize(12).FontWeight(600).TextColor(app.Theme().TextSecondary)
			gui.Select.Root(
				gui.SelectRootProps{
					PickerSourceProps: gui.PickerSourceProps{
						PartProps: gui.PartProps{
							AriaLabel: "Base branch",
							Style: gui.Style().
								Merge(app.Theme().InputStyle()).
								FlexDirection("row").
								AlignItems("center").
								JustifyContent("space-between").
								Gap(8),
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
					).FontSize(13).TextColor(app.Theme().Text)
				},
			)
			CheckRow("Switch to the new branch", checkout, setCheckout)
		},
	).Display("flex").FlexDirection("column").Gap(10), gui.Button(
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
			).FontSize(12).FontWeight(600).TextColor(app.Theme().TextSecondary)
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
					gui.Text(
						gui.Style().FontSize(11.5),
						gui.Style().TextColor(app.Theme().Danger),
						problem(),
					)
				},
			)
			gui.Text(
				"Path",
			).FontSize(12).FontWeight(600).TextColor(app.Theme().TextSecondary)
			gui.Input(
				gui.Value(func() string { return path() }),
				gui.OnInput(func(event *native.Event) { setPath(event.Value) }),
				app.Theme().InputStyle(),
			)
		},
	).Display("flex").FlexDirection("column").Gap(10), gui.Button(
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
	).Display("flex").FlexDirection("column").Gap(10), gui.Button(
		"Stash",
		gui.OnClick(func() { submit() }),
		app.Theme().Button("primary"),
	))
}
