package ui

import (
	"os"
	"path/filepath"
	"strconv"
	"strings"
	"time"

	"github.com/egoist/quickgui/go/native"
	"github.com/egoist/quickgui/go/reactive"
	gui "github.com/egoist/quickgui/go/ui"
	"quickgui.example/quick-git/internal/git"
	"quickgui.example/quick-git/internal/model"
)

func App(store *model.Store, appearance reactive.Accessor[string], openRepository, openPath func(string), onMount func(AppContext)) func() {
	return func() {
		window := native.CurrentWindow()
		theme := gui.CreateMemo(func() Theme { return ThemeFor(appearance()) })
		dialog, setDialog := gui.CreateSignal(DialogRequest{})
		context := AppContext{
			Store: store, Window: window, Theme: theme,
			Dialog: dialog, OpenDialog: setDialog, CloseDialog: func() { setDialog(DialogRequest{}) },
			OpenRepository:     func() { openRepository("") },
			OpenRepositoryPath: func(path string) { openPath(path) },
		}
		if onMount != nil {
			onMount(context)
		}
		ProvideApp(context, func() {
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
		func() {
			gui.Show(
				func() bool { return store.Repository() != nil },
				func() {

					resizablePanel("Resize sidebar", store.SidebarWidth, store.SetSidebarWidth, 180, 420,
						func() {
							Sidebar()
						},
						gui.Style{
							Display:         "flex",
							FlexDirection:   "column",
							Height:          "100%",
							MinWidth:        0,
							MinHeight:       0,
							FlexShrink:      0,
							BackgroundColor: app.Theme().SidebarWash,
						},
					)
					gui.View(
						func() {
							Toolbar()
							mainView()
						},
						gui.Style{
							Display:         "flex",
							Flex:            1,
							MinWidth:        0,
							MinHeight:       0,
							FlexDirection:   "column",
							BackgroundColor: app.Theme().Content,
						},
					)

				},
				func() { Welcome() },
			)
			Dialogs()
			notices()
		},
		gui.Style{
			Position:        "relative",
			Display:         "flex",
			FlexDirection:   "row",
			Width:           "100%",
			Height:          "100%",
			MinWidth:        0,
			MinHeight:       0,
			BackgroundColor: "transparent",
			Color:           app.Theme().Text,
			FontSize:        UIFontSize,
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
											gui.View(gui.Style{
												Width:           3,
												AlignSelf:       "stretch",
												BorderRadius:    2,
												BackgroundColor: color,
											})
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
																title,
																gui.Style{
																	FontSize:   12.5,
																	FontWeight: 700,
																	Color:      app.Theme().Text,
																	LineClamp:  2,
																},
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
																		description,
																		gui.Style{
																			FontSize:   12,
																			LineHeight: 16,
																			Color:      app.Theme().TextSecondary,
																			LineClamp:  4,
																		},
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
	return strings.TrimRight(strings.TrimRight(strconv.FormatFloat(value, 'f', 2, 64), "0"), ".")
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
		func() {
			gui.View(gui.Style{Height: TitlebarHeight, FlexShrink: 0, AppRegion: "drag"})
			gui.View(
				func() {
					gui.View(
						func() {
							icon(branchIcon, 32, func() string { return app.Theme().TextOnAccent })
						},
						gui.Style{
							Display:         "flex",
							Width:           64,
							Height:          64,
							AlignItems:      "center",
							JustifyContent:  "center",
							BorderRadius:    18,
							BackgroundColor: app.Theme().Accent,
							Color:           app.Theme().TextOnAccent,
						},
					)
					gui.Text(
						"Quick Git",
						gui.Style{FontSize: 22, FontWeight: 800, Color: app.Theme().Text},
					)
					gui.Text(
						"Open a repository to review changes, history, and worktrees.",
						gui.Style{
							FontSize:   13,
							Color:      app.Theme().TextSecondary,
							TextAlign:  "center",
							LineHeight: 19,
						},
					)
					gui.Button(
						"Open Repository…",
						gui.OnClick(func() { app.OpenRepository() }),
						app.Theme().Button("primary"),
					)
					gui.Show(
						func() bool { return len(store.RecentRepositories()) > 0 },
						func() {
							gui.View(
								func() {
									gui.Text(
										"Recent",
										gui.Style{
											FontSize:      11,
											FontWeight:    700,
											LetterSpacing: 0.4,
											TextTransform: "uppercase",
											Color:         app.Theme().TextTertiary,
											PaddingLeft:   10,
											MarginBottom:  4,
										},
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
												func() {
													gui.View(
														func() {
															gui.Text(
																gui.Style{
																	FontSize:   13,
																	FontWeight: 600,
																	Color:      app.Theme().Text,
																	LineClamp:  1,
																},
																filepath.Base(path),
															)
															gui.Text(
																gui.Style{
																	FontSize:  11,
																	Color:     app.Theme().TextTertiary,
																	LineClamp: 1,
																},
																shorten(filepath.Dir(path)),
															)
														},
														gui.Style{
															Display:       "flex",
															Flex:          1,
															MinWidth:      0,
															FlexDirection: "column",
														},
													)
													gui.Show(
														func() bool {
															return store.Opening() == path
														},
														func() {
															gui.Text(
																"Opening…",
																gui.Style{
																	FontSize: 11,
																	Color:    app.Theme().TextTertiary,
																},
															)
														},
													)
												},
												gui.Disabled(store.Opening() != ""),
												gui.OnClick(func() { app.OpenRepositoryPath(path) }),
												gui.Style{
													Display:         "flex",
													FlexDirection:   "row",
													AlignItems:      "center",
													Gap:             10,
													Height:          40,
													PaddingLeft:     10,
													PaddingRight:    10,
													BorderRadius:    8,
													BackgroundColor: "transparent",
													Cursor:          "default",
													Hover:           &gui.Style{BackgroundColor: app.Theme().Hover},
													Disabled:        &gui.Style{Opacity: 0.6},
												},
											)
										},
										nil,
										nil,
									)
								},
								gui.Style{
									Display:       "flex",
									FlexDirection: "column",
									Width:         420,
									MaxWidth:      "100%",
									Gap:           2,
									MarginTop:     8,
								},
							)
						},
					)
				},
				gui.Style{
					Display:        "flex",
					Flex:           1,
					MinHeight:      0,
					FlexDirection:  "column",
					AlignItems:     "center",
					JustifyContent: "center",
					Gap:            20,
					Padding:        40,
				},
			)
		},
		gui.Style{
			Display:         "flex",
			Flex:            1,
			MinWidth:        0,
			MinHeight:       0,
			FlexDirection:   "column",
			BackgroundColor: app.Theme().Content,
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

	gui.View(gui.Style{
		Display:      "flex",
		Height:       TitlebarHeight,
		FlexShrink:   0,
		AlignItems:   "center",
		PaddingLeft:  84,
		PaddingRight: 10,
		AppRegion:    "drag",
	})
	gui.View(
		func() {
			gui.Button(
				func() {
					gui.View(
						func() {
							icon(branchIcon, 18, func() string { return app.Theme().TextOnAccent })
						},
						gui.Style{
							Display:         "flex",
							Width:           28,
							Height:          28,
							FlexShrink:      0,
							AlignItems:      "center",
							JustifyContent:  "center",
							BorderRadius:    7,
							BackgroundColor: app.Theme().Accent,
							Color:           app.Theme().TextOnAccent,
						},
					)
					gui.View(
						func() {
							gui.Text(
								func() string { return store.RepositoryName() },
								gui.Style{
									FontSize:   13,
									FontWeight: 700,
									Color:      app.Theme().Text,
									LineClamp:  1,
								},
							)
							gui.Text(
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
								gui.Style{
									FontSize:  11,
									Color:     app.Theme().TextTertiary,
									LineClamp: 1,
								},
							)
						},
						gui.Style{
							Display:       "flex",
							Flex:          1,
							MinWidth:      0,
							FlexDirection: "column",
							Gap:           1,
						},
					)
					icon(chevronDownIcon, 14, func() string { return app.Theme().TextTertiary })
				},
				gui.AriaLabel("Repository actions"),
				gui.OnClick(func() { repositoryMenu(app) }),
				gui.Style{
					Display:         "flex",
					FlexDirection:   "row",
					AlignItems:      "center",
					Gap:             9,
					Height:          44,
					FlexShrink:      0,
					PaddingLeft:     8,
					PaddingRight:    8,
					MarginBottom:    6,
					BorderRadius:    8,
					BackgroundColor: "transparent",
					Cursor:          "default",
					Hover:           &gui.Style{BackgroundColor: app.Theme().Hover},
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
							return strconv.Itoa(store.ChangeCount())
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
		gui.Style{
			Display:       "flex",
			Flex:          1,
			MinHeight:     0,
			FlexDirection: "column",
			Gap:           2,
			PaddingTop:    4,
			PaddingLeft:   10,
			PaddingRight:  10,
			PaddingBottom: 12,
			OverflowY:     "auto",
		},
	)

}

func navRow(label string, selected func() bool, trailing any, onClick func()) {
	app := UseApp()
	gui.Button(
		func() {
			gui.Text(label, gui.Style{Flex: 1, MinWidth: 0, FontSize: 13, LineClamp: 1})
			if text, ok := trailing.(func() string); ok {
				gui.Show(
					func() bool { return text() != "" },
					func() {
						gui.Text(text, gui.Style{FontSize: 11, Color: app.Theme().TextTertiary})
					},
				)
			} else {
				gui.Child(trailing)
			}
		},
		gui.AriaLabel(label),
		gui.When(selected, gui.Selected(true)),
		gui.OnClick(func() { onClick() }),
		rowStyle(app.Theme(), false),
		gui.When(
			selected,
			gui.Style{
				BackgroundColor: func() string { return app.Theme().Selection },
				Hover:           &gui.Style{BackgroundColor: app.Theme().Selection},
			},
		),
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
		func() {
			gui.Button(
				func() {
					gui.Text(
						label,
						gui.Style{
							Flex:       1,
							MinWidth:   0,
							FontSize:   11,
							FontWeight: 600,
							Color:      color,
						},
					)
					gui.Show(
						func() bool { return count() > 0 },
						func() {
							gui.Text(
								count,
								gui.Style{
									FontSize:   11,
									FontWeight: 600,
									Color:      app.Theme().TextTertiary,
								},
							)
						},
					)
				},
				gui.OnClick(func() { onClick() }),
				gui.Style{
					Display:         "flex",
					Flex:            1,
					MinWidth:        0,
					FlexDirection:   "row",
					AlignItems:      "center",
					Gap:             6,
					Height:          22,
					PaddingLeft:     9,
					PaddingRight:    6,
					BorderRadius:    6,
					BackgroundColor: "transparent",
					Cursor:          "default",
					Hover:           &gui.Style{BackgroundColor: app.Theme().Hover},
				},
			)
			gui.Button(
				func() { toolbarIcon(plusIcon) },
				gui.AriaLabel("Add "+strings.ToLower(label)),
				gui.OnClick(func() { action() }),
				app.Theme().IconButton(),
			)
		},
		gui.Group(true),
		gui.Style{
			Display:       "flex",
			FlexDirection: "row",
			AlignItems:    "center",
			Gap:           4,
			MarginTop:     14,
			PaddingRight:  2,
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
