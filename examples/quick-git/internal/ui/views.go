package ui

import (
	"path/filepath"
	"strconv"
	"time"

	"github.com/egoist/quickgui/go/native"
	gui "github.com/egoist/quickgui/go/ui"
	"quickgui.example/quick-git/internal/git"
	"quickgui.example/quick-git/internal/model"
)

type visibleItem[T any] struct {
	Index int
	Item  T
}

func visibleWindow[T any](items []T, window gui.VisibleRange) []visibleItem[T] {
	start, end := window.Start, window.End
	if start < 0 {
		start = 0
	}
	if end > len(items) {
		end = len(items)
	}
	if start > end {
		start = end
	}
	rows := make([]visibleItem[T], 0, end-start)
	for index := start; index < end; index++ {
		rows = append(rows, visibleItem[T]{Index: index, Item: items[index]})
	}
	return rows
}

func ChangesView() {
	app := UseApp()
	store := app.Store
	gui.View(
		func() {
			resizablePanel("Resize file list", store.ChangesSplit, store.SetChangesSplit, 220, 800,
				func() {
					gui.Show(
						func() bool { return store.ChangeCount() > 0 },
						func() {
							fileList(model.ListUnstaged)
							fileList(model.ListStaged)
						},
						func() {
							emptyState("No local changes", "The working tree matches the last commit.")
						},
					)
					commitComposer()
				},
				gui.Styles(
					gui.Display("flex"),
					gui.FlexShrink(0),
					gui.MinHeight(0),
					gui.FlexDirection("column"),
				),
			)
			DiffPane()
		},
		gui.Display("flex"),
		gui.Flex(1),
		gui.MinWidth(0),
		gui.MinHeight(0),
		gui.FlexDirection("row"),
	)
}

func fileList(list model.ListID) {
	app := UseApp()
	store := app.Store
	label := "Unstaged"
	if list == model.ListStaged {
		label = "Staged"
	}
	gui.View(
		func() {
			gui.View(
				func() {
					gui.Text(
						label,
						gui.FontSize(11),
						gui.FontWeight(600),
						gui.Color(app.Theme().TextTertiary),
					)
					gui.Text(
						func() int { return len(store.ListItems(list)) },
						gui.FontSize(11),
						gui.Color(app.Theme().TextTertiary),
					)
					gui.View(gui.Flex(1))
					gui.Show(
						func() bool { return len(store.ListItems(list)) > 0 },
						func() {
							text := "Stage All"
							if list == model.ListStaged {
								text = "Unstage All"
							}
							gui.Button(
								text,
								gui.Disabled(store.Busy() != nil),
								gui.OnClick(func() {
									if list == model.ListUnstaged {
										store.StageAll()
									} else {
										store.UnstageAll()
									}
								}),
								app.Theme().Button("secondary"),
							)
						},
					)
				},
				gui.Display("flex"),
				gui.FlexDirection("row"),
				gui.AlignItems("center"),
				gui.Height(32),
				gui.FlexShrink(0),
				gui.PaddingLeft(12),
				gui.PaddingRight(8),
				gui.Gap(6),
			)
			gui.Show(
				func() bool { return len(store.ListItems(list)) > 0 },
				func() {
					changeTable(list)
				},
				func() {
					empty := "No unstaged changes"
					if list == model.ListStaged {
						empty = "Nothing staged yet"
					}
					gui.Text(
						empty,
						gui.PaddingLeft(12),
						gui.PaddingBottom(10),
						gui.FontSize(12),
						gui.Color(app.Theme().TextTertiary),
					)
				},
			)
		},
		gui.Display("flex"),
		gui.FlexDirection("column"),
		gui.MinHeight(0),
		gui.Flex(1),
		gui.FlexBasis(0),
	)
}

func changeTable(list model.ListID) {
	app := UseApp()
	store := app.Store
	visibleRange, setVisibleRange := gui.CreateSignal(gui.VisibleRange{Start: 0, End: 0})
	visible := func() []visibleItem[git.ChangeItem] {
		return visibleWindow(store.ListItems(list), visibleRange())
	}
	selection := func() []gui.TableRowRange {
		ranges := store.Selection().Unstaged
		if list == model.ListStaged {
			ranges = store.Selection().Staged
		}
		return ranges
	}
	gui.Table.Root(
		gui.TableRootProps{
			PartProps: gui.PartProps{
				Style: gui.Styles(gui.Flex(1), gui.MinHeight(0), gui.OverflowY("scroll")),
			},
			Columns: func() []gui.TableColumnDeclaration {
				return []gui.TableColumnDeclaration{
					{ID: "toggle", Track: "40px", Align: "center"},
					{ID: "status", Track: "24px", Align: "center"},
					{ID: "name", Track: "1fr", RowHeader: true},
				}
			},
			RowCount:      func() float64 { return float64(len(store.ListItems(list))) },
			RowHeight:     28,
			HeaderHeight:  0,
			SelectionMode: "multiple",
			Selection:     selection,
			OnVisibleRangeChange: func(next gui.VisibleRange, _ *native.Event) {
				setVisibleRange(next)
			},
			OnSelectionChange: func(ranges []gui.TableRowRange, _ *native.Event) {
				store.SetSelection(list, ranges)
			},
			OnActivate: func(_ gui.TableCell, _ *native.Event) {
				store.ToggleStaging(list)
			},
		},
		func() {
			gui.KeyedFor(
				visible,
				func(row visibleItem[git.ChangeItem]) any { return row.Index },
				func(row func() visibleItem[git.ChangeItem], _ func() int) {
					gui.Table.Row(
						gui.TableRowProps{
							Index: func() float64 { return float64(row().Index) },
							PartProps: gui.PartProps{
								Style: gui.Styles(
									gui.Hover(gui.BackgroundColor(app.Theme().Hover)),
									gui.SelectedStyle(gui.BackgroundColor(app.Theme().Selection)),
								),
							},
						},
						func() {
							gui.Table.Cell(
								gui.TableCellProps{
									Column:    "toggle",
									PartProps: gui.PartProps{},
								},
								func() {
									stageToggle(list, func() git.ChangeItem { return row().Item })
								},
							)
							gui.Table.Cell(
								gui.TableCellProps{
									Column:    "status",
									PartProps: gui.PartProps{},
								},
								func() {
									gui.Text(
										func() string { return string(row().Item.Code) },
										gui.Width(16),
										gui.FontWeight(700),
										gui.Color(StatusColor(app.Theme(), string(row().Item.Code))),
									)
								},
							)
							gui.Table.Cell(
								gui.TableCellProps{
									Column: "name",
									PartProps: gui.PartProps{
										Style: gui.Styles(
											gui.PaddingRight(8),
											gui.MinWidth(0),
										),
										OnContextMenu: func(*native.Event) { changeMenu(app, list, row().Item) },
										OnDoubleClick: func(*native.Event) { store.ToggleStaging(list) },
									},
								},
								func() {
									changeName(list, func() git.ChangeItem { return row().Item })
								},
							)
						},
					)
				},
				nil,
			)
		},
	)
}

func stageToggle(list model.ListID, item func() git.ChangeItem) {
	app := UseApp()
	store := app.Store
	checked := list == model.ListStaged
	focus := false
	gui.Checkbox.Root(
		gui.CheckboxProps{
			PartProps: gui.PartProps{
				AriaLabel: func() string {
					if checked {
						return "Unstage " + item().Path
					}
					return "Stage " + item().Path
				},
				FocusOnPointer: &focus,
				Style: gui.Styles(
					gui.Display("flex"),
					gui.AlignItems("center"),
					gui.JustifyContent("center"),
					gui.Width(22),
					gui.Height(22),
					gui.BorderRadius(4),
					gui.Cursor("default"),
				),
			},
			Checked: func() gui.CheckedState { return checked },
			OnCheckedChange: func(next bool, _ *native.Event) {
				current := item()
				if next {
					store.StageItems([]git.ChangeItem{current})
					return
				}
				store.UnstageItems([]git.ChangeItem{current})
			},
		},
		func() {
			gui.Checkbox.Indicator(
				gui.PartProps{
					Style: checkboxBox(app.Theme(), checked),
				},
				func() {
					gui.Show(
						func() bool { return checked },
						func() { checkboxMark(true) },
					)
				},
			)
		},
	)
}

func changeName(list model.ListID, item func() git.ChangeItem) {
	app := UseApp()
	store := app.Store
	counts := func() string {
		stats := store.Numstat().Unstaged
		if list == model.ListStaged {
			stats = store.Numstat().Staged
		}
		entry, ok := stats[item().Path]
		if !ok {
			return ""
		}
		text := ""
		if entry.Added != nil {
			text += "+" + strconv.Itoa(*entry.Added)
		}
		if entry.Removed != nil {
			if text != "" {
				text += " "
			}
			text += "-" + strconv.Itoa(*entry.Removed)
		}
		return text
	}
	gui.View(
		func() {
			gui.Text(
				func() string { return item().Path },
				gui.Flex(1),
				gui.MinWidth(0),
				gui.FontSize(13),
				gui.LineClamp(1),
			)
			gui.Show(
				func() bool { return counts() != "" },
				func() {
					gui.Text(
						counts,
						gui.FontSize(11),
						gui.FontFamily("monospace"),
						gui.Color(app.Theme().TextTertiary),
					)
				},
			)
		},
		gui.Display("flex"),
		gui.Flex(1),
		gui.MinWidth(0),
		gui.FlexDirection("row"),
		gui.AlignItems("center"),
		gui.Gap(6),
		gui.Height("100%"),
	)
}

func changeMenu(app AppContext, list model.ListID, item git.ChangeItem) {
	store := app.Store
	store.SelectChange(list, indexOf(store.ListItems(list), item.ID))
	targets := store.SelectedItems(list)
	if len(targets) == 0 {
		targets = []git.ChangeItem{item}
	}
	absolute := filepath.Join(store.Repository().Root(), item.Path)
	items := []native.MenuItem{}
	if list == model.ListUnstaged {
		items = append(items, native.MenuItem{
			Label: "Stage File",
			Click: func() { store.StageItems(targets) },
		})
		items = append(items, native.MenuItem{
			Label: "Discard Changes…",
			Click: func() { confirmDiscard(app, targets) },
		})
	} else {
		items = append(items, native.MenuItem{
			Label: "Unstage File",
			Click: func() { store.UnstageItems(targets) },
		})
	}
	items = append(items,
		native.MenuItem{Type: "separator"},
		native.MenuItem{
			Label: "Reveal in Finder",
			Click: func() {
				native.ShowItemInFolder(
					absolute,
					func(error) {},
				)
			},
		},
		native.MenuItem{Label: "Copy Path", Click: func() { native.WriteClipboardText(item.Path) }},
	)
	native.PopupMenu(
		app.Window,
		items,
		nil,
		nil,
		func(error) {},
	)
}

func indexOf(items []git.ChangeItem, id string) int {
	for i, item := range items {
		if item.ID == id {
			return i
		}
	}
	return 0
}

func confirmDiscard(app AppContext, items []git.ChangeItem) {
	message := "Discard changes to " + strconv.Itoa(len(items)) + " files?"
	if len(items) == 1 {
		message = "Discard changes to " + filepath.Base(items[0].Path) + "?"
	}
	native.ShowAlertDialog(
		native.AlertDialogOptions{
			Message: message,
			Detail:  "The changes cannot be recovered.",
			Level:   "warning",
			Buttons: []native.AlertDialogButton{{Label: "Cancel"}, {Label: "Discard", Role: "destructive"}},
			Window:  app.Window,
		},
		func(index int, err error) {
			if err == nil && index == 1 {
				app.Store.DiscardItems(items)
			}
		},
	)
}

func commitComposer() {
	app := UseApp()
	store := app.Store
	gui.View(
		func() {
			gui.Input(
				gui.Placeholder("Commit summary"),
				gui.Value(func() string { return store.Subject() }),
				gui.OnInput(func(event *native.Event) { store.SetSubject(event.Value) }),
				gui.OnSubmit(func(*native.Event) { store.Commit() }),
				app.Theme().InputStyle(),
			)
			gui.TextArea(
				gui.Placeholder("Description"),
				gui.Value(func() string { return store.Body() }),
				gui.OnInput(func(event *native.Event) { store.SetBody(event.Value) }),
				gui.Display("flex"),
				gui.Width("100%"),
				gui.MinHeight(72),
				gui.PaddingLeft(7),
				gui.PaddingRight(7),
				gui.PaddingTop(4),
				gui.PaddingBottom(4),
				gui.BackgroundColor(app.Theme().Input),
				gui.Color(app.Theme().Text),
				gui.BorderWidth(1),
				gui.BorderColor(app.Theme().InputBorder),
				gui.BorderRadius(6),
				gui.FontSize(UIFontSize),
			)
			gui.View(
				func() {
					CheckRow("Amend", store.Amend, store.SetAmend)
					gui.View(gui.Flex(1))
					gui.Show(
						func() bool { return len(store.Agents()) > 0 },
						func() {
							gui.Button(
								func() string {
									if store.Generating() != nil {
										return "Generating…"
									}
									return "Generate"
								},
								gui.Disabled(store.Generating() != nil),
								gui.OnClick(func() { store.GenerateMessage("") }),
								app.Theme().Button("secondary"),
							)
						},
					)
					gui.Button(
						"Commit",
						gui.Disabled(!store.CanCommit()),
						gui.OnClick(func() { store.Commit() }),
						app.Theme().Button("primary"),
					)
				},
				gui.Display("flex"),
				gui.FlexDirection("row"),
				gui.AlignItems("center"),
				gui.Gap(8),
			)
		},
		gui.Display("flex"),
		gui.FlexDirection("column"),
		gui.FlexShrink(0),
		gui.Gap(8),
		gui.Padding(12),
		gui.BorderTopWidth(1),
		gui.BorderColor(app.Theme().Border),
	)
}

func DiffPane() {
	app := UseApp()
	store := app.Store
	gui.View(
		func() {
			gui.Show(
				func() bool { return store.Diff().Target != nil },
				func() {

					gui.View(
						func() {
							gui.Text(
								func() string {
									if target := store.Diff().Target; target != nil {
										return target.Path
									}
									return ""
								},
								gui.Flex(1),
								gui.MinWidth(0),
								gui.FontSize(13),
								gui.FontWeight(600),
								gui.LineClamp(1),
							)
							gui.Show(
								func() bool { return store.DiffStats().Added > 0 },
								func() {
									gui.Text(
										"+"+strconv.Itoa(store.DiffStats().Added),
										gui.FontSize(11),
										gui.FontWeight(700),
										gui.Color(app.Theme().Success),
										gui.FontFamily("monospace"),
									)
								},
							)
							gui.Show(
								func() bool { return store.DiffStats().Removed > 0 },
								func() {
									gui.Text(
										"-"+strconv.Itoa(store.DiffStats().Removed),
										gui.FontSize(11),
										gui.FontWeight(700),
										gui.Color(app.Theme().Danger),
										gui.FontFamily("monospace"),
									)
								},
							)
							gui.Show(
								func() bool {
									return store.Diff().Loading && store.Diff().Diff == nil
								},
								func() {
									gui.Text(
										"Loading…",
										gui.FontSize(11),
										gui.Color(app.Theme().TextTertiary),
									)
								},
							)
							diffActions()
						},
						gui.Display("flex"),
						gui.FlexDirection("row"),
						gui.AlignItems("center"),
						gui.Gap(8),
						gui.Height(40),
						gui.FlexShrink(0),
						gui.PaddingLeft(14),
						gui.PaddingRight(10),
						gui.BorderBottomWidth(1),
						gui.BorderColor(app.Theme().Border),
					)
					gui.Show(
						func() bool { return store.Diff().Error != "" },
						func() {
							gui.Text(
								store.Diff().Error,
								gui.Padding(12),
								gui.Color(app.Theme().Danger),
								gui.FontSize(12),
							)
						},
					)
					diffTable()

				},
				func() {
					title := "Select a file"
					if store.View() == model.ViewHistory {
						title = "Select a commit"
					}
					emptyState(title, "Choose a change to review its diff.")
				},
			)
		},
		gui.Display("flex"),
		gui.Flex(1),
		gui.MinWidth(0),
		gui.MinHeight(0),
		gui.FlexDirection("column"),
		gui.BackgroundColor(app.Theme().Content),
	)
}

func diffActions() {
	app := UseApp()
	store := app.Store
	gui.Show(
		func() bool {
			return store.Diff().Target != nil && store.Diff().Target.Kind != "commit"
		},
		func() {
			mode := store.Diff().Target.Kind
			lineCount := store.SelectedDiffLineCount()
			gui.View(
				func() {
					gui.Show(
						func() bool { return lineCount > 0 },
						func() {
							if mode == "staged" {
								gui.Button(
									"Unstage "+strconv.Itoa(lineCount)+" Lines",
									gui.OnClick(func() { store.ApplySelectedLines("unstage") }),
									app.Theme().Button("primary"),
								)
								return

							}

							gui.Button(
								"Discard Lines…",
								gui.OnClick(func() { store.ApplySelectedLines("discard") }),
								app.Theme().Button("danger"),
							)
							gui.Button(
								"Stage "+strconv.Itoa(lineCount)+" Lines",
								gui.OnClick(func() { store.ApplySelectedLines("stage") }),
								app.Theme().Button("primary"),
							)

						},
						func() {
							if mode == "staged" {
								gui.Button(
									"Unstage File",
									gui.OnClick(func() {
										if item := store.ActiveItem(); item != nil {
											store.UnstageItems([]git.ChangeItem{*item})
										}
									}),
									app.Theme().Button("secondary"),
								)
								return

							}

							gui.Button(
								"Discard…",
								gui.OnClick(func() {
									if item := store.ActiveItem(); item != nil {
										confirmDiscard(app, []git.ChangeItem{*item})
									}
								}),
								app.Theme().Button("danger"),
							)
							gui.Button(
								"Stage File",
								gui.OnClick(func() {
									if item := store.ActiveItem(); item != nil {
										store.StageItems([]git.ChangeItem{*item})
									}
								}),
								app.Theme().Button("primary"),
							)

						},
					)
				},
				gui.Display("flex"),
				gui.FlexDirection("row"),
				gui.Gap(6),
			)
		},
	)
}

func diffTable() {
	app := UseApp()
	store := app.Store
	visibleRange, setVisibleRange := gui.CreateSignal(gui.VisibleRange{Start: 0, End: 0})
	visible := func() []visibleItem[git.DiffRow] {
		return visibleWindow(store.DiffRows(), visibleRange())
	}
	gui.Table.Root(
		gui.TableRootProps{
			PartProps: gui.PartProps{
				Style: gui.Styles(
					gui.Flex(1),
					gui.MinHeight(0),
					gui.OverflowY("scroll"),
					gui.FontFamily("monospace"),
					gui.FontSize(MonoFontSize),
				),
			},
			Columns: func() []gui.TableColumnDeclaration {
				return []gui.TableColumnDeclaration{
					{ID: "old", Track: "46px", Align: "end"},
					{ID: "new", Track: "46px", Align: "end"},
					{ID: "text", Track: "1fr"},
				}
			},
			RowCount:      func() float64 { return float64(store.DiffRowCount()) },
			RowHeight:     20,
			HeaderHeight:  0,
			SelectionMode: "multiple",
			Selection:     func() []gui.TableRowRange { return store.DiffSelection() },
			OnVisibleRangeChange: func(next gui.VisibleRange, _ *native.Event) {
				setVisibleRange(next)
			},
			OnSelectionChange: func(ranges []gui.TableRowRange, _ *native.Event) {
				store.SetDiffSelection(ranges)
			},
		},
		func() {
			gui.KeyedFor(
				visible,
				func(row visibleItem[git.DiffRow]) any { return row.Index },
				func(row func() visibleItem[git.DiffRow], _ func() int) {
					diffTableRow(func() git.DiffRow { return row().Item }, func() int { return row().Index })
				},
				nil,
			)
		},
	)
}

func diffTableRow(row func() git.DiffRow, index func() int) {
	app := UseApp()
	store := app.Store
	theme := app.Theme()
	current := row()
	background := "transparent"
	textColor := theme.Text
	gutter := theme.ContentAlt
	selected := theme.SelectionMuted
	mark := ""
	markColor := theme.TextTertiary
	switch current.Kind {
	case "hunk":
		background = theme.DiffHunk
		textColor = theme.DiffHunkText
		gutter = theme.DiffHunk
	case "file":
		background = theme.ContentAlt
		textColor = theme.Text
		gutter = theme.ContentAlt
	case "notice":
		textColor = theme.TextTertiary
	case "line":
		switch current.LineKind {
		case git.LineAdded:
			background = theme.DiffAdded
			textColor = theme.DiffAddedText
			gutter = theme.DiffAddedGutter
			selected = "#bfe9cb"
			mark = plusIcon
			markColor = theme.DiffAddedText
		case git.LineRemoved:
			background = theme.DiffRemoved
			textColor = theme.DiffRemovedText
			gutter = theme.DiffRemovedGutter
			selected = "#f7c7c3"
			mark = minusIcon
			markColor = theme.DiffRemovedText
		}
	}
	numberStyle := gui.Styles(
		gui.FontFamily("monospace"),
		gui.FontSize(MonoFontSize-1),
		gui.Color(theme.DiffLineNumber),
		gui.TextAlign("right"),
		gui.PaddingRight(6),
		gui.UserSelect("none"),
	)
	gui.Table.Row(
		gui.TableRowProps{
			Index: func() float64 { return float64(index()) },
			PartProps: gui.PartProps{
				Group: true,
				Style: gui.Styles(
					gui.BackgroundColor(background),
					gui.SelectedStyle(gui.BackgroundColor(selected)),
				),
			},
		},
		func() {
			gui.Table.Cell(
				gui.TableCellProps{
					Column: "old",
					PartProps: gui.PartProps{
						Style: gui.Styles(gui.BackgroundColor(gutter)),
					},
				},
				func() {
					gui.Show(
						func() bool {
							return row().Kind == "line" && row().OldLineNumber != nil
						},
						func() {
							gui.Text(
								func() int { return *row().OldLineNumber },
								numberStyle,
							)
						},
					)
				},
			)
			gui.Table.Cell(
				gui.TableCellProps{
					Column: "new",
					PartProps: gui.PartProps{
						Style: gui.Styles(gui.BackgroundColor(gutter)),
					},
				},
				func() {
					gui.Show(
						func() bool {
							return row().Kind == "line" && row().NewLineNumber != nil
						},
						func() {
							gui.Text(
								func() int { return *row().NewLineNumber },
								numberStyle,
							)
						},
					)
				},
			)
			gui.Table.Cell(
				gui.TableCellProps{
					Column: "text",
					PartProps: gui.PartProps{
						Style: gui.Styles(
							gui.PaddingLeft(8),
							gui.PaddingRight(8),
							gui.MinWidth(0),
							gui.Gap(8),
						),
					},
				},
				func() {
					gui.View(
						func() {
							gui.View(
								func() {
									if mark != "" {
										icon(mark, 12, func() string { return markColor })
									}
								},
								gui.Width(12),
								gui.FlexShrink(0),
							)
							gui.Text(
								func() string { return row().Text },
								gui.Flex(1),
								gui.MinWidth(0),
								gui.FontFamily("monospace"),
								gui.FontSize(MonoFontSize),
								gui.Color(textColor),
								gui.WhiteSpace("nowrap"),
							)
							gui.Show(
								func() bool {
									return row().Kind == "hunk" && store.Diff().Target != nil && store.Diff().Target.Kind != "commit"
								},
								func() {
									gui.Button(
										func() string {
											if store.Diff().Target != nil && store.Diff().Target.Kind == "staged" {
												return "Unstage hunk"
											}
											return "Stage hunk"
										},
										gui.OnClick(func() {
											current := row()
											if store.Diff().Target.Kind == "staged" {
												store.UnstageHunk(current.FileIndex, current.HunkIndex)
												return
											}
											store.StageHunk(current.FileIndex, current.HunkIndex)
										}),
										app.Theme().Button("secondary"),
										gui.Height(16),
										gui.FontFamily("system-ui"),
										gui.FontSize(11),
										gui.LineHeight(14),
										gui.PaddingLeft(6),
										gui.PaddingRight(6),
										gui.BorderRadius(4),
									)
								},
							)
						},
						gui.Display("flex"),
						gui.Flex(1),
						gui.MinWidth(0),
						gui.FlexDirection("row"),
						gui.AlignItems("center"),
						gui.Gap(8),
					)
				},
			)
		},
	)
}

func historyTable() {
	app := UseApp()
	store := app.Store
	visibleRange, setVisibleRange := gui.CreateSignal(gui.VisibleRange{Start: 0, End: 0})
	visible := func() []visibleItem[git.Commit] {
		history := store.History()
		window := visibleRange()
		if window.End > len(history.Commits)-60 && !history.Exhausted && !history.Loading && len(history.Commits) > 0 {
			store.LoadHistory(false)
		}
		return visibleWindow(history.Commits, window)
	}
	graphWidth := gui.CreateMemo(func() float64 {
		lanes := 1
		for _, row := range visibleWindow(store.History().Graph, visibleRange()) {
			lanes = max(lanes, row.Item.LaneCount)
		}
		return 2*graphInset + float64(min(lanes, 8))*graphLaneWidth
	})
	gui.Table.Root(
		gui.TableRootProps{
			PartProps: gui.PartProps{
				Style: gui.Styles(gui.Flex(1), gui.MinHeight(0), gui.OverflowY("scroll")),
			},
			Columns: func() []gui.TableColumnDeclaration {
				return []gui.TableColumnDeclaration{
					{ID: "graph", Track: strconv.FormatFloat(graphWidth(), 'g', -1, 64) + "px"},
					{ID: "subject", Track: "1fr", RowHeader: true},
					{ID: "author", Track: "110px"},
					{ID: "date", Track: "84px", Align: "end"},
				}
			},
			RowCount:      func() float64 { return float64(len(store.History().Commits)) },
			RowHeight:     historyRowHeight,
			HeaderHeight:  0,
			SelectionMode: "single",
			Selection:     func() []gui.TableRowRange { return store.HistorySelection() },
			OnVisibleRangeChange: func(next gui.VisibleRange, _ *native.Event) {
				setVisibleRange(next)
			},
			OnSelectionChange: func(ranges []gui.TableRowRange, _ *native.Event) {
				store.SetHistorySelection(ranges)
			},
		},
		func() {
			gui.KeyedFor(
				visible,
				func(row visibleItem[git.Commit]) any { return row.Index },
				func(row func() visibleItem[git.Commit], _ func() int) {
					historyTableRow(row, graphWidth)
				},
				nil,
			)
		},
	)
}

func historyTableRow(row func() visibleItem[git.Commit], graphWidth func() float64) {
	app := UseApp()
	store := app.Store
	gui.Table.Row(
		gui.TableRowProps{
			Index: func() float64 { return float64(row().Index) },
			PartProps: gui.PartProps{
				Style: gui.Styles(
					gui.Hover(gui.BackgroundColor(app.Theme().Hover)),
					gui.SelectedStyle(gui.BackgroundColor(app.Theme().Selection)),
				),
				OnContextMenu: func(*native.Event) {
					commit := row().Item
					native.PopupMenu(
						app.Window,
						[]native.MenuItem{
							{Label: "Copy SHA", Click: func() { native.WriteClipboardText(commit.Sha) }},
							{Label: "New Branch from Here…", Click: func() { app.OpenDialog(DialogRequest{Kind: DialogNewBranch, From: commit.Sha}) }},
							{Label: "Checkout (Detached)", Click: func() { store.CheckoutCommit(commit.Sha) }},
						},
						nil,
						nil,
						func(error) {},
					)
				},
			},
		},
		func() {
			gui.Table.Cell(
				gui.TableCellProps{
					Column:    "graph",
					PartProps: gui.PartProps{},
				},
				func() {
					historyGraph(func() *git.GraphRow {
						graph := store.History().Graph
						index := row().Index
						if index < 0 || index >= len(graph) {
							return nil
						}
						return &graph[index]
					}, graphWidth)
				},
			)
			gui.Table.Cell(
				gui.TableCellProps{
					Column: "subject",
					PartProps: gui.PartProps{
						Style: gui.Styles(
							gui.MinWidth(0),
							gui.PaddingRight(8),
							gui.Overflow("hidden"),
						),
					},
				},
				func() {
					gui.View(
						func() {
							commitRefs(func() []git.CommitRef { return row().Item.Refs })
							gui.Text(
								func() string { return row().Item.Subject },
								gui.Flex(1),
								gui.FontSize(12.5),
								gui.LineClamp(1),
								gui.MinWidth(0),
							)
						},
						gui.Display("flex"),
						gui.Flex(1),
						gui.MinWidth(0),
						gui.FlexDirection("row"),
						gui.AlignItems("center"),
						gui.Gap(6),
					)
				},
			)
			gui.Table.Cell(
				gui.TableCellProps{
					Column: "author",
					PartProps: gui.PartProps{Style: gui.Styles(
						gui.MinWidth(0),
						gui.PaddingRight(8),
						gui.Overflow("hidden"),
					)},
				},
				func() {
					gui.Text(
						func() string { return row().Item.AuthorName },
						gui.FontSize(11),
						gui.Color(app.Theme().TextTertiary),
						gui.LineClamp(1),
					)
				},
			)
			gui.Table.Cell(
				gui.TableCellProps{
					Column:    "date",
					PartProps: gui.PartProps{Style: gui.Styles(gui.PaddingRight(10))},
				},
				func() {
					gui.Text(
						func() string {
							return git.RelativeTime(row().Item.AuthorTime, time.Now())
						},
						gui.FontSize(11),
						gui.Color(app.Theme().TextTertiary),
						gui.TextAlign("right"),
					)
				},
			)
		},
	)
}

func HistoryView() {
	app := UseApp()
	store := app.Store
	gui.View(
		func() {
			resizablePanel("Resize history", store.HistorySplit, store.SetHistorySplit, 260, 1000,
				func() {
					gui.View(
						func() {
							gui.Text(
								func() string {
									if store.History().AllBranches {
										return "All branches"
									}
									if status := store.Status(); status != nil && status.Branch != "" {
										return status.Branch
									}
									return "History"
								},
								gui.FontSize(11),
								gui.FontWeight(700),
								gui.TextTransform("uppercase"),
								gui.Color(app.Theme().TextTertiary),
							)
							gui.Text(
								func() string {
									suffix := ""
									if !store.History().Exhausted {
										suffix = "+"
									}
									return strconv.Itoa(len(store.History().Commits)) + suffix + " commits"
								},
								gui.FontSize(11),
								gui.Color(app.Theme().TextTertiary),
							)
							gui.View(gui.Flex(1))
							CheckRow("All branches", func() bool { return store.History().AllBranches }, store.SetHistoryAllBranches)
						},
						gui.Display("flex"),
						gui.FlexDirection("row"),
						gui.AlignItems("center"),
						gui.Height(32),
						gui.FlexShrink(0),
						gui.PaddingLeft(12),
						gui.PaddingRight(8),
						gui.Gap(8),
						gui.BorderBottomWidth(1),
						gui.BorderColor(app.Theme().Border),
					)
					gui.Show(
						func() bool { return len(store.History().Commits) > 0 },
						func() {
							historyTable()
						},
						func() {
							emptyState("No commits", "This repository has no history yet.")
						},
					)
				},
				gui.Styles(
					gui.Display("flex"),
					gui.FlexShrink(0),
					gui.MinHeight(0),
					gui.FlexDirection("column"),
				),
			)
			gui.View(
				func() {
					commitDetail()
					DiffPane()
				},
				gui.Display("flex"),
				gui.Flex(1),
				gui.MinWidth(0),
				gui.MinHeight(0),
				gui.FlexDirection("column"),
			)
		},
		gui.Display("flex"),
		gui.Flex(1),
		gui.MinWidth(0),
		gui.MinHeight(0),
		gui.FlexDirection("row"),
	)
}

func commitFileTable() {
	app := UseApp()
	store := app.Store
	visibleRange, setVisibleRange := gui.CreateSignal(gui.VisibleRange{Start: 0, End: 0})
	visible := func() []visibleItem[git.CommitFile] {
		return visibleWindow(store.CommitDetail().Files, visibleRange())
	}
	selection := func() []gui.TableRowRange {
		path := store.CommitDetail().SelectedPath
		if path == "" && len(store.CommitDetail().Files) > 0 {
			path = store.CommitDetail().Files[0].Path
		}
		for index, file := range store.CommitDetail().Files {
			if file.Path == path {
				return []gui.TableRowRange{{index, index}}
			}
		}
		return nil
	}
	gui.Table.Root(
		gui.TableRootProps{
			PartProps: gui.PartProps{
				Style: gui.Styles(
					gui.Height(func() float64 {
						return float64(min(8, len(store.CommitDetail().Files)) * 24)
					}),
					gui.FlexShrink(0),
					gui.MinHeight(0),
					gui.OverflowY("scroll"),
				),
			},
			Columns: func() []gui.TableColumnDeclaration {
				return []gui.TableColumnDeclaration{
					{ID: "status", Track: "36px", Align: "start"},
					{ID: "name", Track: "1fr", RowHeader: true},
				}
			},
			RowCount:      func() float64 { return float64(len(store.CommitDetail().Files)) },
			RowHeight:     24,
			HeaderHeight:  0,
			SelectionMode: "single",
			Selection:     selection,
			OnVisibleRangeChange: func(next gui.VisibleRange, _ *native.Event) {
				setVisibleRange(next)
			},
			OnSelectionChange: func(ranges []gui.TableRowRange, _ *native.Event) {
				if len(ranges) == 0 || len(ranges[0]) == 0 {
					return
				}
				files := store.CommitDetail().Files
				index := ranges[0][0]
				if index >= 0 && index < len(files) {
					store.SelectCommitFile(files[index].Path)
				}
			},
		},
		func() {
			gui.KeyedFor(
				visible,
				func(row visibleItem[git.CommitFile]) any { return row.Item.Path },
				func(row func() visibleItem[git.CommitFile], _ func() int) {
					gui.Table.Row(
						gui.TableRowProps{
							Index: func() float64 { return float64(row().Index) },
							PartProps: gui.PartProps{
								Style: gui.Styles(
									gui.Hover(gui.BackgroundColor(app.Theme().Hover)),
									gui.SelectedStyle(gui.BackgroundColor(app.Theme().Selection)),
								),
							},
						},
						func() {
							gui.Table.Cell(
								gui.TableCellProps{
									Column:    "status",
									PartProps: gui.PartProps{Style: gui.Styles(gui.PaddingLeft(12))},
								},
								func() {
									gui.Text(
										func() string { return row().Item.Status },
										gui.Width(16),
										gui.FontWeight(700),
										gui.Color(StatusColor(app.Theme(), row().Item.Status)),
									)
								},
							)
							gui.Table.Cell(
								gui.TableCellProps{
									Column:    "name",
									PartProps: gui.PartProps{},
								},
								func() {
									gui.Text(
										func() string { return row().Item.Path },
										gui.Flex(1),
										gui.MinWidth(0),
										gui.LineClamp(1),
									)
								},
							)
						},
					)
				},
				nil,
			)
		},
	)
}

func commitDetail() {
	app := UseApp()
	store := app.Store
	gui.View(
		func() {
			gui.Show(
				func() bool { return store.SelectedCommit() != nil },
				func() {

					gui.View(
						func() {
							gui.Text(
								func() string {
									if commit := store.SelectedCommit(); commit != nil {
										return commit.Subject
									}
									return ""
								},
								gui.FontSize(14),
								gui.FontWeight(700),
							)
							gui.Show(
								func() bool {
									commit := store.SelectedCommit()
									return commit != nil && commit.Body != ""
								},
								func() {
									gui.Text(
										func() string {
											if commit := store.SelectedCommit(); commit != nil {
												return commit.Body
											}
											return ""
										},
										gui.FontSize(12.5),
										gui.LineHeight(18),
										gui.UserSelect("text"),
										gui.Color(app.Theme().TextSecondary),
									)
								},
							)
							gui.Text(
								func() string {
									if commit := store.SelectedCommit(); commit != nil {
										return commit.AuthorName + " · " + git.AbsoluteTime(commit.AuthorTime) + " · " + commit.ShortSha
									}
									return ""
								},
								gui.FontSize(12),
								gui.Color(app.Theme().TextSecondary),
							)
						},
						gui.Padding(12),
						gui.Display("flex"),
						gui.FlexDirection("column"),
						gui.Gap(4),
					)
					gui.Show(
						func() bool { return store.CommitDetail().Loading },
						func() {
							gui.Text("Loading files…", gui.PaddingLeft(12), gui.FontSize(12))
						},
					)
					commitFileTable()

				},
				func() {
					emptyState("Select a commit", "Its files appear here.")
				},
			)
		},
		gui.Display("flex"),
		gui.FlexDirection("column"),
		gui.FlexShrink(0),
		gui.MaxHeight("45%"),
		gui.MinHeight(0),
		gui.OverflowY("auto"),
		gui.BorderBottomWidth(1),
		gui.BorderColor(app.Theme().Border),
	)
}

func BranchesView() {
	app := UseApp()
	store := app.Store
	gui.View(
		func() {
			gui.View(
				func() {
					gui.Text(
						"Local branches",
						gui.FontSize(11),
						gui.FontWeight(700),
						gui.TextTransform("uppercase"),
						gui.Color(app.Theme().TextTertiary),
						gui.Flex(1),
					)
					gui.Button(
						"New Branch",
						gui.OnClick(func() { app.OpenDialog(DialogRequest{Kind: DialogNewBranch}) }),
						app.Theme().Button("primary"),
					)
				},
				gui.Display("flex"),
				gui.FlexDirection("row"),
				gui.AlignItems("center"),
				gui.Height(32),
				gui.Gap(8),
			)
			gui.For(
				func() []git.BranchRef { return store.Refs().Local },
				func(branch git.BranchRef, _ func() int) {
					gui.Button(
						func() {
							gui.Text(
								branch.Name,
								gui.Flex(1),
								gui.FontWeight(ternary(branch.Current, 700, 500)),
								gui.LineClamp(1),
							)
							gui.Show(
								func() bool { return branch.Current },
								func() {
									gui.Text(
										"current",
										gui.FontSize(11),
										gui.Color(app.Theme().Accent),
									)
								},
							)
							gui.Text(
								branch.ShortSha,
								gui.FontSize(11),
								gui.Color(app.Theme().TextTertiary),
							)
						},
						gui.OnClick(func() {}),
						gui.OnDoubleClick(func(*native.Event) {
							if !branch.Current {
								store.SwitchBranch(branch.Name)
							}
						}),
						gui.OnContextMenu(func(*native.Event) {
							native.PopupMenu(
								app.Window,
								[]native.MenuItem{
									{Label: "Switch to Branch", Click: func() { store.SwitchBranch(branch.Name) }},
									{Label: "New Branch from Here…", Click: func() { app.OpenDialog(DialogRequest{Kind: DialogNewBranch, From: branch.Name}) }},
									{Label: "Copy Branch Name", Click: func() { native.WriteClipboardText(branch.Name) }},
									{Type: "separator"},
									{Label: "Delete Branch…", Click: func() { store.DeleteBranch(branch.Name, true) }},
								},
								nil,
								nil,
								func(error) {},
							)
						}),
						rowStyle(app.Theme(), branch.Current),
					)
				},
				func(branch git.BranchRef) any { return branch.FullName },
				nil,
			)
			gui.Show(
				func() bool { return len(store.Refs().Remote) > 0 },
				func() {
					gui.Text(
						"Remote branches",
						gui.FontSize(11),
						gui.FontWeight(700),
						gui.TextTransform("uppercase"),
						gui.Color(app.Theme().TextTertiary),
						gui.MarginTop(16),
					)
				},
			)
			gui.For(
				func() []git.BranchRef { return store.Refs().Remote },
				func(branch git.BranchRef, _ func() int) {
					gui.View(
						func() {
							gui.Text(
								branch.Name,
								gui.Flex(1),
								gui.LineClamp(1),
								gui.Color(app.Theme().TextSecondary),
							)
							gui.Text(
								branch.ShortSha,
								gui.FontSize(11),
								gui.Color(app.Theme().TextTertiary),
							)
						},
						rowStyle(app.Theme(), false),
					)
				},
				func(branch git.BranchRef) any { return branch.FullName },
				nil,
			)
		},
		gui.Display("flex"),
		gui.Flex(1),
		gui.MinHeight(0),
		gui.FlexDirection("column"),
		gui.OverflowY("auto"),
		gui.Padding(12),
		gui.Gap(4),
	)
}

func WorktreesView() {
	app := UseApp()
	store := app.Store
	gui.View(
		func() {
			gui.View(
				func() {
					gui.Text(
						"Worktrees",
						gui.FontSize(11),
						gui.FontWeight(700),
						gui.TextTransform("uppercase"),
						gui.Color(app.Theme().TextTertiary),
						gui.Flex(1),
					)
					gui.Button(
						"New Worktree",
						gui.OnClick(func() { app.OpenDialog(DialogRequest{Kind: DialogNewWorktree}) }),
						app.Theme().Button("primary"),
					)
				},
				gui.Display("flex"),
				gui.FlexDirection("row"),
				gui.AlignItems("center"),
				gui.Height(32),
			)
			gui.For(
				store.Worktrees,
				func(worktree git.Worktree, _ func() int) {
					active := store.Repository() != nil && store.Repository().Root() == worktree.Path
					label := worktree.BranchName
					if label == "" {
						label = filepath.Base(worktree.Path)
					}
					gui.Button(
						func() {
							gui.Text(label, gui.Flex(1), gui.LineClamp(1))
							gui.Text(
								gui.FontSize(11),
								gui.Color(app.Theme().TextTertiary),
								filepath.Base(worktree.Path),
							)
						},
						gui.OnClick(func() { store.SelectWorktree(worktree.Path) }),
						gui.OnContextMenu(func(*native.Event) {
							items := []native.MenuItem{
								{Label: "Reveal in Finder", Click: func() {
									native.ShowItemInFolder(
										worktree.Path,
										func(error) {},
									)
								}},
								{Label: "Copy Path", Click: func() { native.WriteClipboardText(worktree.Path) }},
							}
							if !worktree.Main {
								items = append(items, native.MenuItem{Type: "separator"}, native.MenuItem{
									Label: "Remove Worktree",
									Click: func() { store.RemoveWorktree(worktree.Path, false) },
								})
							}
							native.PopupMenu(
								app.Window,
								items,
								nil,
								nil,
								func(error) {},
							)
						}),
						rowStyle(app.Theme(), active),
					)
				},
				func(worktree git.Worktree) any { return worktree.Path },
				nil,
			)
		},
		gui.Display("flex"),
		gui.Flex(1),
		gui.MinHeight(0),
		gui.FlexDirection("column"),
		gui.OverflowY("auto"),
		gui.Padding(12),
		gui.Gap(4),
	)
}

func StashesView() {
	app := UseApp()
	store := app.Store
	gui.View(
		func() {
			gui.View(
				func() {
					gui.Text(
						"Stashes",
						gui.FontSize(11),
						gui.FontWeight(700),
						gui.TextTransform("uppercase"),
						gui.Color(app.Theme().TextTertiary),
						gui.Flex(1),
					)
					gui.Button(
						"Stash Changes",
						gui.Disabled(store.ChangeCount() == 0),
						gui.OnClick(func() { app.OpenDialog(DialogRequest{Kind: DialogStash}) }),
						app.Theme().Button("primary"),
					)
				},
				gui.Display("flex"),
				gui.FlexDirection("row"),
				gui.AlignItems("center"),
				gui.Height(32),
			)
			gui.Show(
				func() bool { return len(store.Stashes()) > 0 },
				func() {
					gui.For(
						store.Stashes,
						func(stash git.StashEntry, _ func() int) {
							gui.Button(
								func() {
									gui.View(
										func() {
											gui.Text(stash.Summary, gui.LineClamp(1))
											gui.Text(
												stash.Ref+" · "+git.RelativeTime(stash.Time, time.Now()),
												gui.FontSize(11),
												gui.Color(app.Theme().TextTertiary),
											)
										},
										gui.Display("flex"),
										gui.Flex(1),
										gui.MinWidth(0),
										gui.FlexDirection("column"),
									)
									gui.Button(
										"Apply",
										gui.OnClick(func() { store.StashApply(stash.Ref) }),
										app.Theme().Button("secondary"),
									)
									gui.Button(
										"Pop",
										gui.OnClick(func() { store.StashPop(stash.Ref) }),
										app.Theme().Button("secondary"),
									)
								},
								gui.OnContextMenu(func(*native.Event) {
									native.PopupMenu(
										app.Window,
										[]native.MenuItem{
											{Label: "Apply", Click: func() { store.StashApply(stash.Ref) }},
											{Label: "Pop", Click: func() { store.StashPop(stash.Ref) }},
											{Label: "Drop", Click: func() { store.StashDrop(stash.Ref) }},
										},
										nil,
										nil,
										func(error) {},
									)
								}),
								rowStyle(app.Theme(), false),
							)
						},
						func(stash git.StashEntry) any { return stash.Ref },
						nil,
					)
				},
				func() {
					emptyState("No stashes", "Stash local changes to save them for later.")
				},
			)
		},
		gui.Display("flex"),
		gui.Flex(1),
		gui.MinHeight(0),
		gui.FlexDirection("column"),
		gui.OverflowY("auto"),
		gui.Padding(12),
		gui.Gap(4),
	)
}

func emptyState(title, description string) {
	app := UseApp()
	gui.View(
		func() {
			gui.Text(
				title,
				gui.FontSize(15),
				gui.FontWeight(500),
				gui.Color(app.Theme().TextTertiary),
				gui.TextAlign("center"),
			)
			gui.Text(
				description,
				gui.FontSize(12),
				gui.LineHeight(17),
				gui.Color(app.Theme().TextTertiary),
				gui.TextAlign("center"),
				gui.MaxWidth(360),
			)
		},
		gui.Display("flex"),
		gui.Flex(1),
		gui.MinHeight(0),
		gui.FlexDirection("column"),
		gui.AlignItems("center"),
		gui.JustifyContent("center"),
		gui.Gap(8),
		gui.Padding(32),
	)
}

func ternary[T any](cond bool, a, b T) T {
	if cond {
		return a
	}
	return b
}
