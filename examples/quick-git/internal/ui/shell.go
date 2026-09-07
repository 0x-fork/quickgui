package ui

import (
	"fmt"
	"os"
	"path/filepath"
	"strings"
	"time"

	"github.com/egoist/quickgui/go/native"
	"github.com/egoist/quickgui/go/reactive"
	gui "github.com/egoist/quickgui/go/ui"
	"quickgui.example/quick-git/internal/git"
	"quickgui.example/quick-git/internal/model"
)

func App(store *model.Store, appearance reactive.Accessor[string], openRepository, openPath func(string)) func() {
	return func() {
		window := native.CurrentWindow()
		theme := gui.CreateMemo(func() Theme { return ThemeFor(appearance()) })
		dialog, setDialog := gui.CreateSignal(DialogRequest{})
		ProvideApp(AppContext{
			Store: store, Window: window, Theme: theme,
			Dialog: dialog, OpenDialog: setDialog, CloseDialog: func() { setDialog(DialogRequest{}) },
			OpenRepository:     func() { openRepository("") },
			OpenRepositoryPath: func(path string) { openPath(path) },
		}, func() {
			gui.Toast.Provider(
				gui.ToastProviderProps{
					Timeout:        4500,
					Limit:          3,
					Pitch:          6,
					SwipeDirection: "right",
				},
				func() {
					toasts := gui.UseToastManager()
					store.SetNotifier(func(notice model.Notice) {
						request := gui.ToastRequest{
							Title:       notice.Title,
							Description: notice.Description,
							Type:        gui.ToastType(notice.Type),
						}
						if notice.Timeout > 0 {
							duration := float64(notice.Timeout)
							request.Duration = &duration
						}
						toasts.Add(request)
					})
					shell()
				},
			)
		})
	}
}

func shell() {
	app := UseApp()
	store := app.Store
	gui.View(
		gui.Position("relative"),
		gui.Display("flex"),
		gui.FlexDirection("row"),
		gui.Width("100%"),
		gui.Height("100%"),
		gui.MinWidth(0),
		gui.MinHeight(0),
		gui.BackgroundColor("transparent"),
		gui.Color(app.Theme().Text),
		gui.FontSize(UIFontSize),
		func() {
			gui.Show(
				func() bool { return store.Repository() != nil },
				func() {

					gui.View(
						gui.Display("flex"),
						gui.FlexDirection("column"),
						gui.Height("100%"),
						gui.MinWidth(0),
						gui.MinHeight(0),
						gui.Width(store.SidebarWidth),
						gui.FlexShrink(0),
						gui.BackgroundColor(app.Theme().SidebarWash),
						func() {
							Sidebar()
						},
					)
					resizeDivider("Resize sidebar", store.SidebarWidth, store.SetSidebarWidth)
					gui.View(
						gui.Display("flex"),
						gui.Flex(1),
						gui.MinWidth(0),
						gui.MinHeight(0),
						gui.FlexDirection("column"),
						gui.BackgroundColor(app.Theme().Content),
						func() {
							Toolbar()
							mainView()
						},
					)

				},
				func() { Welcome() },
			)
			Dialogs()
			notices()
		},
	)
}

func mainView() {
	store := UseApp().Store
	gui.Dynamic(func() gui.Component {
		switch store.View() {
		case model.ViewChanges:
			return ChangesView
		case model.ViewHistory:
			return HistoryView
		case model.ViewBranches:
			return BranchesView
		case model.ViewWorktrees:
			return WorktreesView
		default:
			return StashesView
		}
	})
}

func notices() {
	app := UseApp()
	toasts := gui.UseToastManager()
	gui.Toast.Portal(
		gui.PartProps{
			Style: gui.Style{
				Position: "absolute",
				Right:    16,
				Bottom:   16,
				Width:    340,
				Display:  "flex",
			},
		},
		func() {
			gui.Toast.Viewport(
				gui.ToastViewportProps{
					PartProps: gui.PartProps{
						Style: gui.Style{
							Display:       "flex",
							FlexDirection: "column",
							Gap:           8,
							Width:         "100%",
						},
					},
				},
				func() {
					gui.For(
						func() []gui.ToastStackEntry { return toasts.Stack() },
						func(entry gui.ToastStackEntry, _ func() int) {
							toast := func() *gui.ToastDeclaration {
								for i := range toasts.Toasts() {
									if toasts.Toasts()[i].ID == entry.ID {
										item := toasts.Toasts()[i]
										return &item
									}
								}
								return nil
							}
							color := app.Theme().Accent
							switch entry.Type {
							case "error":
								color = app.Theme().Danger
							case "success":
								color = app.Theme().Success
							case "warning":
								color = app.Theme().Warning
							}
							opacity := 1.0
							if entry.Limited {
								opacity = 0.6
							}
							gui.Toast.Positioner(
								gui.ToastPartProps{
									ToastID:   entry.ID,
									PartProps: gui.PartProps{},
								},
								func() {
									gui.Toast.Root(
										gui.ToastPartProps{
											ToastID: entry.ID,
											PartProps: gui.PartProps{
												Style: gui.Style{
													Display:         "flex",
													FlexDirection:   "row",
													AlignItems:      "flex-start",
													Gap:             10,
													PaddingLeft:     12,
													PaddingRight:    8,
													PaddingTop:      10,
													PaddingBottom:   10,
													BackgroundColor: app.Theme().Raised,
													BorderWidth:     1,
													BorderColor:     app.Theme().BorderStrong,
													BorderRadius:    8,
													Opacity:         opacity,
													Transform:       "translateX(" + formatSwipe(entry.SwipeMovement) + "px)",
												},
											},
										},
										func() {
											gui.View(
												gui.Width(3),
												gui.AlignSelf("stretch"),
												gui.BorderRadius(2),
												gui.BackgroundColor(color),
											)
											gui.Toast.Content(
												gui.ToastPartProps{
													ToastID: entry.ID,
													PartProps: gui.PartProps{
														Style: gui.Style{
															Display:       "flex",
															Flex:          1,
															MinWidth:      0,
															FlexDirection: "column",
															Gap:           2,
														},
													},
												},
												func() {
													gui.Toast.Title(
														gui.ToastPartProps{
															ToastID:   entry.ID,
															PartProps: gui.PartProps{},
														},
														func() {
															title := ""
															if current := toast(); current != nil {
																title = current.Title
															}
															gui.Text(
																gui.FontSize(12.5),
																gui.FontWeight(700),
																gui.Color(app.Theme().Text),
																gui.LineClamp(2),
																title,
															)
														},
													)
													gui.Show(
														func() bool {
															current := toast()
															return current != nil && current.Description != ""
														},
														func() {
															gui.Toast.Description(
																gui.ToastPartProps{
																	ToastID:   entry.ID,
																	PartProps: gui.PartProps{},
																},
																func() {
																	description := ""
																	if current := toast(); current != nil {
																		description = current.Description
																	}
																	gui.Text(
																		gui.FontSize(12),
																		gui.LineHeight(16),
																		gui.Color(app.Theme().TextSecondary),
																		gui.LineClamp(4),
																		description,
																	)
																},
															)
														},
													)
												},
											)
											gui.Toast.Close(
												gui.ToastPartProps{
													ToastID: entry.ID,
													PartProps: gui.PartProps{
														AriaLabel: "Dismiss notification",
														Style:     app.Theme().IconButton(),
													},
												},
												func() { toolbarIcon(closeIcon) },
											)
										},
									)
								},
							)
						},
						func(entry gui.ToastStackEntry) any { return entry.ID },
						nil,
					)
				},
			)
		},
	)
}

func formatSwipe(value float64) string {
	return strings.TrimRight(strings.TrimRight(fmt.Sprintf("%.2f", value), "0"), ".")
}

func Welcome() {
	app := UseApp()
	store := app.Store
	home, _ := os.UserHomeDir()
	shorten := func(path string) string {
		if home != "" && strings.HasPrefix(path, home) {
			return "~" + path[len(home):]
		}
		return path
	}
	gui.View(
		gui.Display("flex"),
		gui.Flex(1),
		gui.MinWidth(0),
		gui.MinHeight(0),
		gui.FlexDirection("column"),
		gui.BackgroundColor(app.Theme().Content),
		func() {
			gui.View(gui.Height(TitlebarHeight), gui.FlexShrink(0), gui.AppRegion("drag"))
			gui.View(
				gui.Display("flex"),
				gui.Flex(1),
				gui.MinHeight(0),
				gui.FlexDirection("column"),
				gui.AlignItems("center"),
				gui.JustifyContent("center"),
				gui.Gap(20),
				gui.Padding(40),
				func() {
					gui.View(
						gui.Display("flex"),
						gui.Width(64),
						gui.Height(64),
						gui.AlignItems("center"),
						gui.JustifyContent("center"),
						gui.BorderRadius(18),
						gui.BackgroundColor(app.Theme().Accent),
						gui.Color(app.Theme().TextOnAccent),
						func() {
							icon(branchIcon, 32, func() string { return app.Theme().TextOnAccent })
						},
					)
					gui.Text(
						gui.FontSize(22),
						gui.FontWeight(800),
						gui.Color(app.Theme().Text),
						"Quick Git",
					)
					gui.Text(
						gui.FontSize(13),
						gui.Color(app.Theme().TextSecondary),
						gui.TextAlign("center"),
						gui.LineHeight(19),
						"Open a repository to review changes, history, and worktrees.",
					)
					gui.Button(
						gui.OnClick(func() { app.OpenRepository() }),
						gui.WithStyle(app.Theme().Button("primary")),
						"Open Repository…",
					)
					gui.Show(
						func() bool { return len(store.RecentRepositories()) > 0 },
						func() {
							gui.View(
								gui.Display("flex"),
								gui.FlexDirection("column"),
								gui.Width(420),
								gui.MaxWidth("100%"),
								gui.Gap(2),
								gui.MarginTop(8),
								func() {
									gui.Text(
										gui.FontSize(11),
										gui.FontWeight(700),
										gui.LetterSpacing(0.4),
										gui.TextTransform("uppercase"),
										gui.Color(app.Theme().TextTertiary),
										gui.PaddingLeft(10),
										gui.MarginBottom(4),
										"Recent",
									)
									gui.For(
										func() []string {
											recent := store.RecentRepositories()
											if len(recent) > 8 {
												return recent[:8]
											}
											return recent
										},
										func(path string, _ func() int) {
											gui.Button(
												gui.Disabled(store.Opening() != ""),
												gui.OnClick(func() { app.OpenRepositoryPath(path) }),
												gui.Display("flex"),
												gui.FlexDirection("row"),
												gui.AlignItems("center"),
												gui.Gap(10),
												gui.Height(40),
												gui.PaddingLeft(10),
												gui.PaddingRight(10),
												gui.BorderRadius(8),
												gui.BackgroundColor("transparent"),
												gui.Cursor("default"),
												gui.Hover(gui.BackgroundColor(app.Theme().Hover)),
												gui.DisabledStyle(gui.Opacity(0.6)),
												func() {
													gui.View(
														gui.Display("flex"),
														gui.Flex(1),
														gui.MinWidth(0),
														gui.FlexDirection("column"),
														func() {
															gui.Text(
																gui.FontSize(13),
																gui.FontWeight(600),
																gui.Color(app.Theme().Text),
																gui.LineClamp(1),
																filepath.Base(path),
															)
															gui.Text(
																gui.FontSize(11),
																gui.Color(app.Theme().TextTertiary),
																gui.LineClamp(1),
																shorten(filepath.Dir(path)),
															)
														},
													)
													gui.Show(
														func() bool {
															return store.Opening() == path
														},
														func() {
															gui.Text(
																gui.FontSize(11),
																gui.Color(app.Theme().TextTertiary),
																"Opening…",
															)
														},
													)
												},
											)
										},
										nil,
										nil,
									)
								},
							)
						},
					)
				},
			)
		},
	)
}

func Sidebar() {
	app := UseApp()
	store := app.Store
	nav := []struct {
		ID    model.ViewID
		Label string
	}{
		{model.ViewChanges, "Changes"},
		{model.ViewHistory, "History"},
	}

	gui.View(
		gui.Display("flex"),
		gui.Height(TitlebarHeight),
		gui.FlexShrink(0),
		gui.AlignItems("center"),
		gui.PaddingLeft(84),
		gui.PaddingRight(10),
		gui.AppRegion("drag"),
	)
	gui.View(
		gui.Display("flex"),
		gui.Flex(1),
		gui.MinHeight(0),
		gui.FlexDirection("column"),
		gui.Gap(2),
		gui.PaddingTop(4),
		gui.PaddingLeft(10),
		gui.PaddingRight(10),
		gui.PaddingBottom(12),
		gui.OverflowY("auto"),
		func() {
			gui.Button(
				gui.AriaLabel("Repository actions"),
				gui.OnClick(func() { repositoryMenu(app) }),
				gui.Display("flex"),
				gui.FlexDirection("row"),
				gui.AlignItems("center"),
				gui.Gap(9),
				gui.Height(44),
				gui.FlexShrink(0),
				gui.PaddingLeft(8),
				gui.PaddingRight(8),
				gui.MarginBottom(6),
				gui.BorderRadius(8),
				gui.BackgroundColor("transparent"),
				gui.Cursor("default"),
				gui.Hover(gui.BackgroundColor(app.Theme().Hover)),
				func() {
					gui.View(
						gui.Display("flex"),
						gui.Width(28),
						gui.Height(28),
						gui.FlexShrink(0),
						gui.AlignItems("center"),
						gui.JustifyContent("center"),
						gui.BorderRadius(7),
						gui.BackgroundColor(app.Theme().Accent),
						gui.Color(app.Theme().TextOnAccent),
						func() {
							icon(branchIcon, 18, func() string { return app.Theme().TextOnAccent })
						},
					)
					gui.View(
						gui.Display("flex"),
						gui.Flex(1),
						gui.MinWidth(0),
						gui.FlexDirection("column"),
						gui.Gap(1),
						func() {
							gui.Text(
								gui.FontSize(13),
								gui.FontWeight(700),
								gui.Color(app.Theme().Text),
								gui.LineClamp(1),
								func() string { return store.RepositoryName() },
							)
							gui.Text(
								gui.FontSize(11),
								gui.Color(app.Theme().TextTertiary),
								gui.LineClamp(1),
								func() string {
									if status := store.Status(); status != nil {
										if status.Branch != "" {
											return status.Branch
										}
										if status.Detached {
											return "Detached HEAD"
										}
									}
									return ""
								},
							)
						},
					)
					icon(chevronDownIcon, 14, func() string { return app.Theme().TextTertiary })
				},
			)
			gui.For(
				func() []struct {
					ID    model.ViewID
					Label string
				} {
					return nav
				},
				func(item struct {
					ID    model.ViewID
					Label string
				}, _ func() int) {
					navRow(item.Label, func() bool { return store.View() == item.ID }, func() string {
						if item.ID == model.ViewChanges && store.ChangeCount() > 0 {
							return fmt.Sprintf("%d", store.ChangeCount())
						}
						return ""
					}, func() { store.SetView(item.ID) })
				},
				func(item struct {
					ID    model.ViewID
					Label string
				}) any {
					return item.ID
				},
				nil,
			)
			sectionRow("Branches", func() int { return len(store.Refs().Local) }, func() bool { return store.View() == model.ViewBranches }, func() { store.SetView(model.ViewBranches) }, func() { app.OpenDialog(DialogRequest{Kind: DialogNewBranch}) })
			gui.For(
				func() []git.BranchRef {
					local := store.Refs().Local
					if len(local) > 8 {
						return local[:8]
					}
					return local
				},
				func(branch git.BranchRef, _ func() int) {
					navRow(branch.Name, func() bool { return false }, func() { currentIndicator(func() bool { return branch.Current }) }, func() {
						if branch.Current {
							return
						}
						if branch.WorktreePath != "" && store.Repository() != nil && branch.WorktreePath != store.Repository().Root() {
							store.SelectWorktree(branch.WorktreePath)
							return
						}
						store.SwitchBranch(branch.Name)
					})
				},
				func(branch git.BranchRef) any { return branch.FullName },
				nil,
			)
			sectionRow("Worktrees", func() int { return len(store.Worktrees()) }, func() bool { return store.View() == model.ViewWorktrees }, func() { store.SetView(model.ViewWorktrees) }, func() { app.OpenDialog(DialogRequest{Kind: DialogNewWorktree}) })
			gui.For(
				store.Worktrees,
				func(worktree git.Worktree, _ func() int) {
					label := worktree.BranchName
					if label == "" {
						if worktree.Detached && len(worktree.HeadSha) >= 7 {
							label = worktree.HeadSha[:7] + " (detached)"
						} else {
							label = filepath.Base(worktree.Path)
						}
					}
					navRow(label, func() bool { return false }, func() {
						currentIndicator(func() bool { return store.Repository() != nil && store.Repository().Root() == worktree.Path })
					}, func() { store.SelectWorktree(worktree.Path) })
				},
				func(worktree git.Worktree) any { return worktree.Path },
				nil,
			)
			sectionRow("Stashes", func() int { return len(store.Stashes()) }, func() bool { return store.View() == model.ViewStashes }, func() { store.SetView(model.ViewStashes) }, func() { app.OpenDialog(DialogRequest{Kind: DialogStash}) })
			gui.For(
				func() []git.StashEntry {
					stashes := store.Stashes()
					if len(stashes) > 5 {
						return stashes[:5]
					}
					return stashes
				},
				func(stash git.StashEntry, _ func() int) {
					navRow(stash.Summary, func() bool { return false }, func() string { return git.RelativeTime(stash.Time, time.Now()) }, func() { store.SetView(model.ViewStashes) })
				},
				func(stash git.StashEntry) any { return stash.Ref },
				nil,
			)
		},
	)

}

func navRow(label string, selected func() bool, trailing any, onClick func()) {
	app := UseApp()
	gui.Button(
		gui.AriaLabel(label),
		gui.When(selected, gui.Selected(true)),
		gui.OnClick(func() { onClick() }),
		gui.WithStyle(rowStyle(app.Theme(), false)),
		gui.When(
			selected,
			gui.BackgroundColor(func() string { return app.Theme().Selection }),
			gui.Hover(gui.BackgroundColor(app.Theme().Selection)),
		),
		func() {
			gui.Text(gui.Flex(1), gui.MinWidth(0), gui.FontSize(13), gui.LineClamp(1), label)
			if text, ok := trailing.(func() string); ok {
				gui.Show(
					func() bool { return text() != "" },
					func() {
						gui.Text(gui.FontSize(11), gui.Color(app.Theme().TextTertiary), text)
					},
				)
			} else {
				gui.Child(trailing)
			}
		},
	)
}

func sectionRow(label string, count func() int, selected func() bool, onClick, action func()) {
	app := UseApp()
	color := func() string {
		if selected() {
			return app.Theme().Accent
		}
		return app.Theme().TextTertiary
	}
	gui.View(
		gui.Group(true),
		gui.Display("flex"),
		gui.FlexDirection("row"),
		gui.AlignItems("center"),
		gui.Gap(4),
		gui.MarginTop(14),
		gui.PaddingRight(2),
		func() {
			gui.Button(
				gui.OnClick(func() { onClick() }),
				gui.Display("flex"),
				gui.Flex(1),
				gui.MinWidth(0),
				gui.FlexDirection("row"),
				gui.AlignItems("center"),
				gui.Gap(6),
				gui.Height(22),
				gui.PaddingLeft(9),
				gui.PaddingRight(6),
				gui.BorderRadius(6),
				gui.BackgroundColor("transparent"),
				gui.Cursor("default"),
				gui.Hover(gui.BackgroundColor(app.Theme().Hover)),
				func() {
					gui.Text(
						gui.Flex(1),
						gui.MinWidth(0),
						gui.FontSize(11),
						gui.FontWeight(600),
						gui.Color(color),
						label,
					)
					gui.Show(
						func() bool { return count() > 0 },
						func() {
							gui.Text(
								gui.FontSize(11),
								gui.FontWeight(600),
								gui.Color(app.Theme().TextTertiary),
								func() string { return fmt.Sprintf("%d", count()) },
							)
						},
					)
				},
			)
			gui.Button(
				gui.AriaLabel("Add "+strings.ToLower(label)),
				gui.OnClick(func() { action() }),
				gui.WithStyle(app.Theme().IconButton()),
				func() { toolbarIcon(plusIcon) },
			)
		},
	)
}

func repositoryMenu(app AppContext) {
	store := app.Store
	recent := store.RecentRepositories()
	labels := RepositoryLabels(recent)
	current := ""
	if main := store.MainRepository(); main != nil {
		current = main.Root()
	}
	items := make([]native.MenuItem, 0, len(recent)+6)
	for _, path := range recent {
		p := path
		label := labels[path]
		if label == "" {
			label = filepath.Base(path)
		}
		items = append(items, native.MenuItem{
			Label:   label,
			Checked: path == current,
			Click: func() {
				if p != current {
					app.OpenRepositoryPath(p)
				}
			},
		})
	}
	if len(recent) > 0 {
		items = append(items, native.MenuItem{Type: "separator"})
	}
	items = append(items,
		native.MenuItem{Label: "Open Repository…", Click: app.OpenRepository},
		native.MenuItem{Type: "separator"},
		native.MenuItem{
			Label: "Reveal in Finder",
			Click: func() {
				if repo := store.Repository(); repo != nil {
					native.ShowItemInFolder(
						repo.Root(),
						func(error) {},
					)
				}
			},
		},
		native.MenuItem{
			Label: "Copy Path",
			Click: func() {
				if repo := store.Repository(); repo != nil {
					native.WriteClipboardText(repo.Root())
				}
			},
		},
		native.MenuItem{Type: "separator"},
		native.MenuItem{Label: "Close Repository", Click: store.CloseRepository},
	)
	native.PopupMenu(
		app.Window,
		items,
		nil,
		nil,
		func(error) {},
	)
}
