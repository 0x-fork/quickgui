package main

import (
	_ "embed"
	"encoding/json"
	"log"
	"strconv"
	"strings"

	"github.com/egoist/quickgui/go/native"
	"github.com/egoist/quickgui/go/reactive"
	"github.com/egoist/quickgui/go/ui"
)

//go:embed issues.generated.json
var issuesJSON []byte

const pageSize = 100

var (
	ink    = "#20242c"
	muted  = "#737c8c"
	line   = "#e2e5eb"
	accent = "#285bd4"
)

type issueData struct {
	ID, Title, Project, Owner, Priority, Status, Description, Notes string
}

type issue struct {
	issueData
	status    reactive.Accessor[string]
	setStatus reactive.Setter[string]
	notes     reactive.Accessor[string]
	setNotes  reactive.Setter[string]
}

func column() ui.Style {
	return ui.Styles(
		ui.Display("flex"),
		ui.FlexDirection("column"),
		ui.MinHeight(0),
		ui.MinWidth(0),
	)
}
func row() ui.Style {
	return ui.Styles(
		ui.Display("flex"),
		ui.AlignItems("center"),
		ui.MinWidth(0),
	)
}
func caption(value any) { ui.Text(value, ui.FontSize(12), ui.LineHeight(18), ui.Color(muted)) }
func control(label any, click func(), disabled func() bool) {
	ui.Button(
		label,
		ui.Display("flex"),
		ui.AlignItems("center"),
		ui.JustifyContent("center"),
		ui.Height(32),
		ui.PaddingLeft(12),
		ui.PaddingRight(12),
		ui.BorderWidth(1),
		ui.BorderColor("#dce0e7"),
		ui.BorderRadius(6),
		ui.BackgroundColor("white"),
		ui.FontSize(12),
		ui.DisabledStyle(ui.Opacity(0.4)),
		ui.OnClick(click),
		ui.Disabled(disabled),
	)
}
func property(label string, value any) {
	ui.View(
		func() {
			caption(label)
			ui.Text(value, ui.FontSize(12), ui.FontWeight(500))
		},
		row(),
		ui.JustifyContent("space-between"),
	)
}

func issueTracker() {
	var data []issueData
	if err := json.Unmarshal(issuesJSON, &data); err != nil {
		panic(err)
	}
	if len(data) != 1000 {
		panic("Expected 1,000 benchmark issues")
	}
	issues := make([]*issue, len(data))
	for i, value := range data {
		status, setStatus := ui.CreateSignal(value.Status)
		notes, setNotes := ui.CreateSignal(value.Notes)
		issues[i] = &issue{value, status, setStatus, notes, setNotes}
	}
	query, setQuery := ui.CreateSignal("")
	filter, setFilter := ui.CreateSignal("All issues")
	page, setPage := ui.CreateSignal(0)
	selected, setSelected := ui.CreateSignal(0)
	current := func() *issue { return issues[selected()] }
	completed := ui.CreateMemo(func() int {
		n := 0
		for _, item := range issues {
			if item.status() == "Done" {
				n++
			}
		}
		return n
	})
	matching := ui.CreateMemo(func() []int {
		needle, mode := strings.ToLower(strings.TrimSpace(query())), filter()
		result := make([]int, 0, len(issues))
		for i, item := range issues {
			status := item.status()
			if mode == "Open" && status == "Done" || mode == "Completed" && status != "Done" {
				continue
			}
			if strings.Contains(strings.ToLower(item.ID+" "+item.Title+" "+item.Project+" "+item.Owner), needle) {
				result = append(result, i)
			}
		}
		return result
	})
	pages := func() int { return max(1, (len(matching())+pageSize-1)/pageSize) }
	currentPage := func() int { return min(page(), pages()-1) }
	visible := ui.CreateMemo(func() []int {
		matches := matching()
		start := currentPage() * pageSize
		return matches[start:min(start+pageSize, len(matches))]
	})

	ui.View(
		func() {
			ui.View(
				func() {
					ui.Text(
						"Orbit",
						ui.FontSize(22),
						ui.FontWeight(700),
						ui.PaddingLeft(12),
						ui.MarginBottom(6),
					)
					ui.Text(
						"Product workspace",
						ui.FontSize(12),
						ui.Color(muted),
						ui.PaddingLeft(12),
						ui.MarginBottom(24),
					)
					for _, name := range []string{"All issues", "Open", "Completed"} {
						ui.Button(
							name,
							ui.Display("flex"),
							ui.Height(42),
							ui.PaddingLeft(12),
							ui.AlignItems("center"),
							ui.JustifyContent("flex-start"),
							ui.BorderWidth(0),
							ui.BorderRadius(7),
							ui.FontWeight(func() int {
								if filter() == name {
									return 600
								}
								return 400
							}),
							ui.BackgroundColor(func() string {
								if filter() == name {
									return "#e4ebfb"
								}
								return "transparent"
							}),
							ui.Color(func() string {
								if filter() == name {
									return accent
								}
								return ink
							}),
							ui.OnClick(func() { ui.Batch(func() { setFilter(name); setPage(0) }) }),
						)
					}
					ui.View(ui.Flex(1))
					ui.Text(
						"September cycle\n4 projects · 5 teammates",
						ui.FontSize(12),
						ui.LineHeight(18),
						ui.Color(muted),
						ui.Padding(12),
					)
				},
				column(),
				ui.Width(176),
				ui.FlexShrink(0),
				ui.PaddingTop(24),
				ui.PaddingBottom(24),
				ui.PaddingLeft(12),
				ui.PaddingRight(12),
				ui.Gap(8),
				ui.BackgroundColor("#f4f5f7"),
				ui.BorderRightWidth(1),
				ui.BorderColor(line),
			)
			ui.View(
				func() {
					ui.View(
						func() {
							ui.View(
								func() {
									ui.Text("Issue inbox", ui.FontSize(24), ui.FontWeight(700))
									caption(func() string {
										return strconv.Itoa(len(issues)-completed()) + " open · " + strconv.Itoa(completed()) + " completed"
									})
								},
								column(),
								ui.Gap(6),
							)
							ui.Input(
								ui.Value(query),
								ui.Placeholder("Search issues, projects, people"),
								ui.AriaLabel("Search issues"),
								ui.OnInput(func(e *native.Event) {
									ui.Batch(func() { setQuery(e.Value); setPage(0) })
								}),
								ui.Width(280),
								ui.Height(38),
								ui.PaddingLeft(12),
								ui.PaddingRight(12),
								ui.BackgroundColor("#f8f9fb"),
								ui.BorderWidth(1),
								ui.BorderColor("#dce0e7"),
								ui.BorderRadius(7),
							)
						},
						row(),
						ui.Height(94),
						ui.FlexShrink(0),
						ui.PaddingLeft(24),
						ui.PaddingRight(24),
						ui.JustifyContent("space-between"),
						ui.BorderBottomWidth(1),
						ui.BorderColor(line),
					)
					ui.View(
						func() {
							ui.View(
								func() {
									ui.View(
										func() {
											caption(func() string { return strconv.Itoa(len(matching())) + " issues" })
											caption("Updated this week")
										},
										row(),
										ui.Height(48),
										ui.FlexShrink(0),
										ui.PaddingLeft(20),
										ui.PaddingRight(20),
										ui.JustifyContent("space-between"),
										ui.BackgroundColor("#fafbfc"),
										ui.BorderBottomWidth(1),
										ui.BorderColor(line),
									)
									// A new result page starts at the top; selection and edits keep its viewport.
									ui.For(
										func() []string {
											return []string{query() + "|" + filter() + "|" + strconv.Itoa(currentPage())}
										},
										func(_ string, _ func() int) {
											ui.View(
												func() {
													ui.For(
														visible,
														func(index int, _ func() int) {
															item := issues[index]
															ui.Button(
																func() {
																	ui.Text(
																		item.Title,
																		ui.FontSize(14),
																		ui.FontWeight(500),
																		ui.WhiteSpace("nowrap"),
																		ui.TextOverflow("ellipsis"),
																		ui.Overflow("hidden"),
																	)
																	ui.Text(
																		func() string {
																			return item.ID + "  ·  " + item.Project + "  ·  " + item.status() + "  ·  " + item.Owner
																		},
																		ui.FontSize(11),
																		ui.Color(muted),
																		ui.WhiteSpace("nowrap"),
																	)
																},
																column(),
																ui.Height(68),
																ui.FlexShrink(0),
																ui.JustifyContent("center"),
																ui.AlignItems("stretch"),
																ui.Gap(8),
																ui.PaddingLeft(20),
																ui.PaddingRight(20),
																ui.BorderWidth(0),
																ui.BorderBottomWidth(1),
																ui.BorderRadius(0),
																ui.BorderColor("#edf0f4"),
																ui.BackgroundColor(func() string {
																	if selected() == index {
																		return "#edf3ff"
																	}
																	return "white"
																}),
																ui.OnClick(func() { setSelected(index) }),
															)
														},
														func(index int) any { return index },
														func() { caption("No matching issues") },
													)
												},
												column(),
												ui.Flex(1),
												ui.OverflowY("scroll"),
											)
										},
										func(key string) any { return key },
										nil,
									)
									ui.View(
										func() {
											caption(func() string { return "Page " + strconv.Itoa(currentPage()+1) + " of " + strconv.Itoa(pages()) })
											ui.View(
												func() {
													control("Previous", func() { setPage(currentPage() - 1) }, func() bool { return currentPage() == 0 })
													control("Next", func() { setPage(currentPage() + 1) }, func() bool { return currentPage()+1 >= pages() })
												},
												row(),
												ui.Gap(8),
											)
										},
										row(),
										ui.Height(58),
										ui.FlexShrink(0),
										ui.JustifyContent("space-between"),
										ui.PaddingLeft(20),
										ui.PaddingRight(20),
										ui.BorderTopWidth(1),
										ui.BorderColor(line),
									)
								},
								column(),
								ui.Flex(1),
							)
							ui.View(
								func() {
									ui.View(
										func() {
											caption(func() string { return current().ID + " / " + current().Project })
											ui.Text(
												func() string { return current().Title },
												ui.FontSize(21),
												ui.LineHeight(28),
												ui.FontWeight(700),
												ui.MarginTop(14),
												ui.MarginBottom(22),
											)
											ui.View(
												func() {
													property("Status", func() string { return current().status() })
													property("Assignee", func() string { return current().Owner })
													property("Priority", func() string { return current().Priority })
												},
												column(),
												ui.Gap(12),
												ui.MarginBottom(24),
											)
											ui.Text(
												func() string { return current().Description },
												ui.FontSize(13),
												ui.LineHeight(20),
												ui.MarginBottom(22),
											)
											ui.Text(
												"Working notes",
												ui.FontSize(12),
												ui.FontWeight(600),
												ui.MarginBottom(8),
											)
											ui.TextArea(
												ui.Value(func() string { return current().notes() }),
												ui.AriaLabel("Working notes"),
												ui.OnInput(func(e *native.Event) {
													current().setNotes(e.Value)
												}),
												ui.Height(100),
												ui.FlexShrink(0),
												ui.Padding(10),
												ui.FontSize(13),
												ui.LineHeight(19),
												ui.BorderWidth(1),
												ui.BorderColor("#dce0e7"),
												ui.BorderRadius(7),
											)
											ui.Button(
												func() string {
													if current().status() == "Done" {
														return "Reopen issue"
													}
													return "Mark complete"
												},
												ui.OnClick(func() {
													item := current()
													if item.status() == "Done" {
														item.setStatus("Open")
													} else {
														item.setStatus("Done")
													}
												}),
												row(),
												ui.Height(36),
												ui.FlexShrink(0),
												ui.JustifyContent("center"),
												ui.BorderWidth(0),
												ui.BorderRadius(7),
												ui.MarginTop(16),
												ui.BackgroundColor(accent),
												ui.Color("white"),
												ui.FontWeight(500),
											)
											ui.Text(
												"Changes are kept for this session.",
												ui.FontSize(11),
												ui.Color(muted),
												ui.MarginTop(10),
											)
										},
										column(),
										ui.FlexShrink(0),
									)
								},
								column(),
								ui.Width(350),
								ui.FlexShrink(0),
								ui.OverflowY("auto"),
								ui.Padding(24),
								ui.BorderLeftWidth(1),
								ui.BorderColor(line),
							)
						},
						row(),
						ui.Flex(1),
						ui.MinHeight(0),
						ui.AlignItems("stretch"),
					)
				},
				column(),
				ui.Flex(1),
			)
		},
		row(),
		ui.Width("100%"),
		ui.Height("100%"),
		ui.AlignItems("stretch"),
		ui.Color(ink),
		ui.FontSize(14),
		ui.BackgroundColor("white"),
	)
}

func main() {
	if err := native.Run(func() {
		native.NewWindow(native.WindowOptions{
			Title:      "Issue tracker — ready",
			Width:      1100,
			Height:     720,
			Background: "#ffffff",
			Component:  issueTracker,
		})
	}); err != nil {
		log.Fatal(err)
	}
}
