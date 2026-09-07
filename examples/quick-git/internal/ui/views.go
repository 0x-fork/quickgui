package ui

import (
	"fmt"
	"path/filepath"
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
		gui.Display("flex"),
		gui.Flex(1),
		gui.MinWidth(0),
		gui.MinHeight(0),
		gui.FlexDirection("row"),
		func() {
			gui.View(
				gui.Display("flex"),
				gui.Width(store.ChangesSplit),
				gui.FlexShrink(0),
				gui.MinHeight(0),
				gui.FlexDirection("column"),
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
			)
			resizeDivider("Resize file list", store.ChangesSplit, store.SetChangesSplit)
			DiffPane()
		},
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
		gui.Display("flex"),
		gui.FlexDirection("column"),
		gui.MinHeight(0),
		gui.Flex(1),
		gui.FlexBasis(0),
		func() {
			gui.View(
				gui.Display("flex"),
				gui.FlexDirection("row"),
				gui.AlignItems("center"),
				gui.Height(32),
				gui.FlexShrink(0),
				gui.PaddingLeft(12),
				gui.PaddingRight(8),
				gui.Gap(6),
				func() {
					gui.Text(
						gui.FontSize(11),
						gui.FontWeight(600),
						gui.Color(app.Theme().TextTertiary),
						label,
					)
					gui.Text(
						gui.FontSize(11),
						gui.Color(app.Theme().TextTertiary),
						func() string {
							return fmt.Sprintf("%d", len(store.ListItems(list)))
						},
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
								gui.Disabled(store.Busy() != nil),
								gui.OnClick(func() {
									if list == model.ListUnstaged {
										store.StageAll()
									} else {
										store.UnstageAll()
									}
								}),
								gui.WithStyle(app.Theme().Button("secondary")),
								text,
							)
						},
					)
				},
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
						gui.PaddingLeft(12),
						gui.PaddingBottom(10),
						gui.FontSize(12),
						gui.Color(app.Theme().TextTertiary),
						empty,
					)
				},
			)
		},
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
				Style: gui.Style{Flex: 1, MinHeight: 0, OverflowY: "scroll"},
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
								Style: gui.Style{
									Hover:    &gui.Style{BackgroundColor: app.Theme().Hover},
									Selected: &gui.Style{BackgroundColor: app.Theme().Selection},
								},
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
										gui.Width(16),
										gui.FontWeight(700),
										gui.Color(StatusColor(app.Theme(), string(row().Item.Code))),
										func() string { return string(row().Item.Code) },
									)
								},
							)
							gui.Table.Cell(
								gui.TableCellProps{
									Column: "name",
									PartProps: gui.PartProps{
										Style:         gui.Style{PaddingRight: 8, MinWidth: 0},
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
				Style: gui.Style{
					Display:        "flex",
					AlignItems:     "center",
					JustifyContent: "center",
					Width:          22,
					Height:         22,
					BorderRadius:   4,
					Cursor:         "default",
				},
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
			text += fmt.Sprintf("+%d", *entry.Added)
		}
		if entry.Removed != nil {
			if text != "" {
				text += " "
			}
			text += fmt.Sprintf("-%d", *entry.Removed)
		}
		return text
	}
	gui.View(
		gui.Display("flex"),
		gui.Flex(1),
		gui.MinWidth(0),
		gui.FlexDirection("row"),
		gui.AlignItems("center"),
		gui.Gap(6),
		gui.Height("100%"),
		func() {
			gui.Text(
				gui.Flex(1),
				gui.MinWidth(0),
				gui.FontSize(13),
				gui.LineClamp(1),
				func() string { return item().Path },
			)
			gui.Show(
				func() bool { return counts() != "" },
				func() {
					gui.Text(
						gui.FontSize(11),
						gui.FontFamily("monospace"),
						gui.Color(app.Theme().TextTertiary),
						counts,
					)
				},
			)
		},
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
	message := fmt.Sprintf("Discard changes to %d files?", len(items))
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
		gui.Display("flex"),
		gui.FlexDirection("column"),
		gui.FlexShrink(0),
		gui.Gap(8),
		gui.Padding(12),
		gui.BorderTopWidth(1),
		gui.BorderColor(app.Theme().Border),
		func() {
			gui.Input(
				gui.Placeholder("Commit summary"),
				gui.Value(func() string { return store.Subject() }),
				gui.OnInput(func(event *native.Event) { store.SetSubject(event.Value) }),
				gui.OnSubmit(func(*native.Event) { store.Commit() }),
				gui.WithStyle(app.Theme().InputStyle()),
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
				gui.Display("flex"),
				gui.FlexDirection("row"),
				gui.AlignItems("center"),
				gui.Gap(8),
				func() {
					CheckRow("Amend", store.Amend, store.SetAmend)
					gui.View(gui.Flex(1))
					gui.Show(
						func() bool { return len(store.Agents()) > 0 },
						func() {
							gui.Button(
								gui.Disabled(store.Generating() != nil),
								gui.OnClick(func() { store.GenerateMessage("") }),
								gui.WithStyle(app.Theme().Button("secondary")),
								func() string {
									if store.Generating() != nil {
										return "Generating…"
									}
									return "Generate"
								},
							)
						},
					)
					gui.Button(
						gui.Disabled(!store.CanCommit()),
						gui.OnClick(func() { store.Commit() }),
						gui.WithStyle(app.Theme().Button("primary")),
						"Commit",
					)
				},
			)
		},
	)
}

func DiffPane() {
	app := UseApp()
	store := app.Store
	gui.View(
		gui.Display("flex"),
		gui.Flex(1),
		gui.MinWidth(0),
		gui.MinHeight(0),
		gui.FlexDirection("column"),
		gui.BackgroundColor(app.Theme().Content),
		func() {
			gui.Show(
				func() bool { return store.Diff().Target != nil },
				func() {

					gui.View(
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
						func() {
							gui.Text(
								gui.Flex(1),
								gui.MinWidth(0),
								gui.FontSize(13),
								gui.FontWeight(600),
								gui.LineClamp(1),
								func() string {
									if target := store.Diff().Target; target != nil {
										return target.Path
									}
									return ""
								},
							)
							gui.Show(
								func() bool { return store.DiffStats().Added > 0 },
								func() {
									gui.Text(
										gui.FontSize(11),
										gui.FontWeight(700),
										gui.Color(app.Theme().Success),
										gui.FontFamily("monospace"),
										fmt.Sprintf("+%d", store.DiffStats().Added),
									)
								},
							)
							gui.Show(
								func() bool { return store.DiffStats().Removed > 0 },
								func() {
									gui.Text(
										gui.FontSize(11),
										gui.FontWeight(700),
										gui.Color(app.Theme().Danger),
										gui.FontFamily("monospace"),
										fmt.Sprintf("-%d", store.DiffStats().Removed),
									)
								},
							)
							gui.Show(
								func() bool { return store.Diff().Loading },
								func() {
									gui.Text(
										gui.FontSize(11),
										gui.Color(app.Theme().TextTertiary),
										"Loading…",
									)
								},
							)
							diffActions()
						},
					)
					gui.Show(
						func() bool { return store.Diff().Error != "" },
						func() {
							gui.Text(
								gui.Padding(12),
								gui.Color(app.Theme().Danger),
								gui.FontSize(12),
								store.Diff().Error,
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
				gui.Display("flex"),
				gui.FlexDirection("row"),
				gui.Gap(6),
				func() {
					gui.Show(
						func() bool { return lineCount > 0 },
						func() {
							if mode == "staged" {
								gui.Button(
									gui.OnClick(func() { store.ApplySelectedLines("unstage") }),
									gui.WithStyle(app.Theme().Button("primary")),
									fmt.Sprintf("Unstage %d Lines", lineCount),
								)
								return

							}

							gui.Button(
								gui.OnClick(func() { store.ApplySelectedLines("discard") }),
								gui.WithStyle(app.Theme().Button("danger")),
								"Discard Lines…",
							)
							gui.Button(
								gui.OnClick(func() { store.ApplySelectedLines("stage") }),
								gui.WithStyle(app.Theme().Button("primary")),
								fmt.Sprintf("Stage %d Lines", lineCount),
							)

						},
						func() {
							if mode == "staged" {
								gui.Button(
									gui.OnClick(func() {
										if item := store.ActiveItem(); item != nil {
											store.UnstageItems([]git.ChangeItem{*item})
										}
									}),
									gui.WithStyle(app.Theme().Button("secondary")),
									"Unstage File",
								)
								return

							}

							gui.Button(
								gui.OnClick(func() {
									if item := store.ActiveItem(); item != nil {
										confirmDiscard(app, []git.ChangeItem{*item})
									}
								}),
								gui.WithStyle(app.Theme().Button("danger")),
								"Discard…",
							)
							gui.Button(
								gui.OnClick(func() {
									if item := store.ActiveItem(); item != nil {
										store.StageItems([]git.ChangeItem{*item})
									}
								}),
								gui.WithStyle(app.Theme().Button("primary")),
								"Stage File",
							)

						},
					)
				},
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
				Style: gui.Style{
					Flex:       1,
					MinHeight:  0,
					OverflowY:  "scroll",
					FontFamily: "monospace",
					FontSize:   MonoFontSize,
				},
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
	numberStyle := gui.Style{
		FontFamily:   "monospace",
		FontSize:     MonoFontSize - 1,
		Color:        theme.DiffLineNumber,
		TextAlign:    "right",
		PaddingRight: 6,
		UserSelect:   "none",
	}
	gui.Table.Row(
		gui.TableRowProps{
			Index: func() float64 { return float64(index()) },
			PartProps: gui.PartProps{
				Group: true,
				Style: gui.Style{
					BackgroundColor: background,
					Selected:        &gui.Style{BackgroundColor: selected},
				},
			},
		},
		func() {
			gui.Table.Cell(
				gui.TableCellProps{
					Column: "old",
					PartProps: gui.PartProps{
						Style: gui.Style{BackgroundColor: gutter},
					},
				},
				func() {
					gui.Show(
						func() bool {
							return row().Kind == "line" && row().OldLineNumber != nil
						},
						func() {
							gui.Text(
								gui.WithStyle(numberStyle),
								func() string { return fmt.Sprintf("%d", *row().OldLineNumber) },
							)
						},
					)
				},
			)
			gui.Table.Cell(
				gui.TableCellProps{
					Column: "new",
					PartProps: gui.PartProps{
						Style: gui.Style{BackgroundColor: gutter},
					},
				},
				func() {
					gui.Show(
						func() bool {
							return row().Kind == "line" && row().NewLineNumber != nil
						},
						func() {
							gui.Text(
								gui.WithStyle(numberStyle),
								func() string { return fmt.Sprintf("%d", *row().NewLineNumber) },
							)
						},
					)
				},
			)
			gui.Table.Cell(
				gui.TableCellProps{
					Column: "text",
					PartProps: gui.PartProps{
						Style: gui.Style{PaddingLeft: 8, PaddingRight: 8, MinWidth: 0, Gap: 8},
					},
				},
				func() {
					gui.View(
						gui.Display("flex"),
						gui.Flex(1),
						gui.MinWidth(0),
						gui.FlexDirection("row"),
						gui.AlignItems("center"),
						gui.Gap(8),
						func() {
							gui.View(
								gui.Width(12),
								gui.FlexShrink(0),
								func() {
									if mark != "" {
										icon(mark, 12, func() string { return markColor })
									}
								},
							)
							gui.Text(
								gui.Flex(1),
								gui.MinWidth(0),
								gui.FontFamily("monospace"),
								gui.FontSize(MonoFontSize),
								gui.Color(textColor),
								gui.WhiteSpace("nowrap"),
								func() string { return row().Text },
							)
							gui.Show(
								func() bool {
									return row().Kind == "hunk" && store.Diff().Target != nil && store.Diff().Target.Kind != "commit"
								},
								func() {
									gui.Button(
										gui.OnClick(func() {
											current := row()
											if store.Diff().Target.Kind == "staged" {
												store.UnstageHunk(current.FileIndex, current.HunkIndex)
												return
											}
											store.StageHunk(current.FileIndex, current.HunkIndex)
										}),
										gui.WithStyle(app.Theme().Button("secondary")),
										gui.Height(16),
										gui.FontFamily("system-ui"),
										gui.FontSize(11),
										gui.LineHeight(14),
										gui.PaddingLeft(6),
										gui.PaddingRight(6),
										gui.BorderRadius(4),
										func() string {
											if store.Diff().Target != nil && store.Diff().Target.Kind == "staged" {
												return "Unstage hunk"
											}
											return "Stage hunk"
										},
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
				Style: gui.Style{Flex: 1, MinHeight: 0, OverflowY: "scroll"},
			},
			Columns: func() []gui.TableColumnDeclaration {
				return []gui.TableColumnDeclaration{
					{ID: "graph", Track: fmt.Sprintf("%gpx", graphWidth())},
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
				Style: gui.Style{
					Hover:    &gui.Style{BackgroundColor: app.Theme().Hover},
					Selected: &gui.Style{BackgroundColor: app.Theme().Selection},
				},
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
						Style: gui.Style{MinWidth: 0, PaddingRight: 8, Overflow: "hidden"},
					},
				},
				func() {
					gui.View(
						gui.Display("flex"),
						gui.Flex(1),
						gui.MinWidth(0),
						gui.FlexDirection("row"),
						gui.AlignItems("center"),
						gui.Gap(6),
						func() {
							commitRefs(func() []git.CommitRef { return row().Item.Refs })
							gui.Text(
								gui.Flex(1),
								gui.FontSize(12.5),
								gui.LineClamp(1),
								gui.MinWidth(0),
								func() string { return row().Item.Subject },
							)
						},
					)
				},
			)
			gui.Table.Cell(
				gui.TableCellProps{
					Column: "author",
					PartProps: gui.PartProps{Style: gui.Style{
						MinWidth:     0,
						PaddingRight: 8,
						Overflow:     "hidden",
					}},
				},
				func() {
					gui.Text(
						gui.FontSize(11),
						gui.Color(app.Theme().TextTertiary),
						gui.LineClamp(1),
						func() string { return row().Item.AuthorName },
					)
				},
			)
			gui.Table.Cell(
				gui.TableCellProps{
					Column:    "date",
					PartProps: gui.PartProps{Style: gui.Style{PaddingRight: 10}},
				},
				func() {
					gui.Text(
						gui.FontSize(11),
						gui.Color(app.Theme().TextTertiary),
						gui.TextAlign("right"),
						func() string {
							return git.RelativeTime(row().Item.AuthorTime, time.Now())
						},
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
		gui.Display("flex"),
		gui.Flex(1),
		gui.MinWidth(0),
		gui.MinHeight(0),
		gui.FlexDirection("row"),
		func() {
			gui.View(
				gui.Display("flex"),
				gui.Width(store.HistorySplit),
				gui.FlexShrink(0),
				gui.MinHeight(0),
				gui.FlexDirection("column"),
				func() {
					gui.View(
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
						func() {
							gui.Text(
								gui.FontSize(11),
								gui.FontWeight(700),
								gui.TextTransform("uppercase"),
								gui.Color(app.Theme().TextTertiary),
								func() string {
									if store.History().AllBranches {
										return "All branches"
									}
									if status := store.Status(); status != nil && status.Branch != "" {
										return status.Branch
									}
									return "History"
								},
							)
							gui.Text(
								gui.FontSize(11),
								gui.Color(app.Theme().TextTertiary),
								func() string {
									suffix := ""
									if !store.History().Exhausted {
										suffix = "+"
									}
									return fmt.Sprintf("%d%s commits", len(store.History().Commits), suffix)
								},
							)
							gui.View(gui.Flex(1))
							CheckRow("All branches", func() bool { return store.History().AllBranches }, store.SetHistoryAllBranches)
						},
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
			)
			resizeDivider("Resize history", store.HistorySplit, store.SetHistorySplit)
			gui.View(
				gui.Display("flex"),
				gui.Flex(1),
				gui.MinWidth(0),
				gui.MinHeight(0),
				gui.FlexDirection("column"),
				func() {
					commitDetail()
					DiffPane()
				},
			)
		},
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
				Style: gui.Style{
					Height:     func() float64 { return float64(min(8, len(store.CommitDetail().Files)) * 24) },
					FlexShrink: 0,
					MinHeight:  0,
					OverflowY:  "scroll",
				},
			},
			Columns: func() []gui.TableColumnDeclaration {
				return []gui.TableColumnDeclaration{
					{ID: "status", Track: "24px", Align: "center"},
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
								Style: gui.Style{
									Hover:    &gui.Style{BackgroundColor: app.Theme().Hover},
									Selected: &gui.Style{BackgroundColor: app.Theme().Selection},
								},
							},
						},
						func() {
							gui.Table.Cell(
								gui.TableCellProps{
									Column:    "status",
									PartProps: gui.PartProps{},
								},
								func() {
									gui.Text(
										gui.Width(16),
										gui.FontWeight(700),
										gui.Color(StatusColor(app.Theme(), row().Item.Status)),
										func() string { return row().Item.Status },
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
										gui.Flex(1),
										gui.MinWidth(0),
										gui.LineClamp(1),
										func() string { return row().Item.Path },
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
		gui.Display("flex"),
		gui.FlexDirection("column"),
		gui.FlexShrink(0),
		gui.MaxHeight("45%"),
		gui.MinHeight(0),
		gui.OverflowY("auto"),
		gui.BorderBottomWidth(1),
		gui.BorderColor(app.Theme().Border),
		func() {
			gui.Show(
				func() bool { return store.SelectedCommit() != nil },
				func() {

					gui.View(
						gui.Padding(12),
						gui.Display("flex"),
						gui.FlexDirection("column"),
						gui.Gap(4),
						func() {
							gui.Text(
								gui.FontSize(14),
								gui.FontWeight(700),
								func() string {
									if commit := store.SelectedCommit(); commit != nil {
										return commit.Subject
									}
									return ""
								},
							)
							gui.Show(
								func() bool {
									commit := store.SelectedCommit()
									return commit != nil && commit.Body != ""
								},
								func() {
									gui.Text(
										gui.FontSize(12.5),
										gui.LineHeight(18),
										gui.UserSelect("text"),
										gui.Color(app.Theme().TextSecondary),
										func() string {
											if commit := store.SelectedCommit(); commit != nil {
												return commit.Body
											}
											return ""
										},
									)
								},
							)
							gui.Text(
								gui.FontSize(12),
								gui.Color(app.Theme().TextSecondary),
								func() string {
									if commit := store.SelectedCommit(); commit != nil {
										return commit.AuthorName + " · " + git.AbsoluteTime(commit.AuthorTime) + " · " + commit.ShortSha
									}
									return ""
								},
							)
						},
					)
					gui.Show(
						func() bool { return store.CommitDetail().Loading },
						func() {
							gui.Text(
								gui.PaddingLeft(12),
								gui.FontSize(12),
								"Loading files…",
							)
						},
					)
					commitFileTable()

				},
				func() {
					emptyState("Select a commit", "Its files appear here.")
				},
			)
		},
	)
}

func BranchesView() {
	app := UseApp()
	store := app.Store
	gui.View(
		gui.Display("flex"),
		gui.Flex(1),
		gui.MinHeight(0),
		gui.FlexDirection("column"),
		gui.OverflowY("auto"),
		gui.Padding(12),
		gui.Gap(4),
		func() {
			gui.View(
				gui.Display("flex"),
				gui.FlexDirection("row"),
				gui.AlignItems("center"),
				gui.Height(32),
				gui.Gap(8),
				func() {
					gui.Text(
						gui.FontSize(11),
						gui.FontWeight(700),
						gui.TextTransform("uppercase"),
						gui.Color(app.Theme().TextTertiary),
						gui.Flex(1),
						"Local branches",
					)
					gui.Button(
						gui.OnClick(func() { app.OpenDialog(DialogRequest{Kind: DialogNewBranch}) }),
						gui.WithStyle(app.Theme().Button("primary")),
						"New Branch",
					)
				},
			)
			gui.For(
				func() []git.BranchRef { return store.Refs().Local },
				func(branch git.BranchRef, _ func() int) {
					gui.Button(
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
						gui.WithStyle(rowStyle(app.Theme(), branch.Current)),
						func() {
							gui.Text(
								gui.Flex(1),
								gui.FontWeight(ternary(branch.Current, 700, 500)),
								gui.LineClamp(1),
								branch.Name,
							)
							gui.Show(
								func() bool { return branch.Current },
								func() {
									gui.Text(
										gui.FontSize(11),
										gui.Color(app.Theme().Accent),
										"current",
									)
								},
							)
							gui.Text(
								gui.FontSize(11),
								gui.Color(app.Theme().TextTertiary),
								branch.ShortSha,
							)
						},
					)
				},
				func(branch git.BranchRef) any { return branch.FullName },
				nil,
			)
			gui.Show(
				func() bool { return len(store.Refs().Remote) > 0 },
				func() {
					gui.Text(
						gui.FontSize(11),
						gui.FontWeight(700),
						gui.TextTransform("uppercase"),
						gui.Color(app.Theme().TextTertiary),
						gui.MarginTop(16),
						"Remote branches",
					)
				},
			)
			gui.For(
				func() []git.BranchRef { return store.Refs().Remote },
				func(branch git.BranchRef, _ func() int) {
					gui.View(
						gui.WithStyle(rowStyle(app.Theme(), false)),
						func() {
							gui.Text(
								gui.Flex(1),
								gui.LineClamp(1),
								gui.Color(app.Theme().TextSecondary),
								branch.Name,
							)
							gui.Text(
								gui.FontSize(11),
								gui.Color(app.Theme().TextTertiary),
								branch.ShortSha,
							)
						},
					)
				},
				func(branch git.BranchRef) any { return branch.FullName },
				nil,
			)
		},
	)
}

func WorktreesView() {
	app := UseApp()
	store := app.Store
	gui.View(
		gui.Display("flex"),
		gui.Flex(1),
		gui.MinHeight(0),
		gui.FlexDirection("column"),
		gui.OverflowY("auto"),
		gui.Padding(12),
		gui.Gap(4),
		func() {
			gui.View(
				gui.Display("flex"),
				gui.FlexDirection("row"),
				gui.AlignItems("center"),
				gui.Height(32),
				func() {
					gui.Text(
						gui.FontSize(11),
						gui.FontWeight(700),
						gui.TextTransform("uppercase"),
						gui.Color(app.Theme().TextTertiary),
						gui.Flex(1),
						"Worktrees",
					)
					gui.Button(
						gui.OnClick(func() { app.OpenDialog(DialogRequest{Kind: DialogNewWorktree}) }),
						gui.WithStyle(app.Theme().Button("primary")),
						"New Worktree",
					)
				},
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
						gui.WithStyle(rowStyle(app.Theme(), active)),
						func() {
							gui.Text(gui.Flex(1), gui.LineClamp(1), label)
							gui.Text(
								gui.FontSize(11),
								gui.Color(app.Theme().TextTertiary),
								filepath.Base(worktree.Path),
							)
						},
					)
				},
				func(worktree git.Worktree) any { return worktree.Path },
				nil,
			)
		},
	)
}

func StashesView() {
	app := UseApp()
	store := app.Store
	gui.View(
		gui.Display("flex"),
		gui.Flex(1),
		gui.MinHeight(0),
		gui.FlexDirection("column"),
		gui.OverflowY("auto"),
		gui.Padding(12),
		gui.Gap(4),
		func() {
			gui.View(
				gui.Display("flex"),
				gui.FlexDirection("row"),
				gui.AlignItems("center"),
				gui.Height(32),
				func() {
					gui.Text(
						gui.FontSize(11),
						gui.FontWeight(700),
						gui.TextTransform("uppercase"),
						gui.Color(app.Theme().TextTertiary),
						gui.Flex(1),
						"Stashes",
					)
					gui.Button(
						gui.Disabled(store.ChangeCount() == 0),
						gui.OnClick(func() { app.OpenDialog(DialogRequest{Kind: DialogStash}) }),
						gui.WithStyle(app.Theme().Button("primary")),
						"Stash Changes",
					)
				},
			)
			gui.Show(
				func() bool { return len(store.Stashes()) > 0 },
				func() {
					gui.For(
						store.Stashes,
						func(stash git.StashEntry, _ func() int) {
							gui.Button(
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
								gui.WithStyle(rowStyle(app.Theme(), false)),
								func() {
									gui.View(
										gui.Display("flex"),
										gui.Flex(1),
										gui.MinWidth(0),
										gui.FlexDirection("column"),
										func() {
											gui.Text(gui.LineClamp(1), stash.Summary)
											gui.Text(
												gui.FontSize(11),
												gui.Color(app.Theme().TextTertiary),
												stash.Ref+" · "+git.RelativeTime(stash.Time, time.Now()),
											)
										},
									)
									gui.Button(
										gui.OnClick(func() { store.StashApply(stash.Ref) }),
										gui.WithStyle(app.Theme().Button("secondary")),
										"Apply",
									)
									gui.Button(
										gui.OnClick(func() { store.StashPop(stash.Ref) }),
										gui.WithStyle(app.Theme().Button("secondary")),
										"Pop",
									)
								},
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
	)
}

func emptyState(title, description string) {
	app := UseApp()
	gui.View(
		gui.Display("flex"),
		gui.Flex(1),
		gui.MinHeight(0),
		gui.FlexDirection("column"),
		gui.AlignItems("center"),
		gui.JustifyContent("center"),
		gui.Gap(8),
		gui.Padding(32),
		func() {
			gui.Text(
				gui.FontSize(15),
				gui.FontWeight(500),
				gui.Color(app.Theme().TextTertiary),
				gui.TextAlign("center"),
				title,
			)
			gui.Text(
				gui.FontSize(12),
				gui.LineHeight(17),
				gui.Color(app.Theme().TextTertiary),
				gui.TextAlign("center"),
				gui.MaxWidth(360),
				description,
			)
		},
	)
}

func ternary[T any](cond bool, a, b T) T {
	if cond {
		return a
	}
	return b
}
