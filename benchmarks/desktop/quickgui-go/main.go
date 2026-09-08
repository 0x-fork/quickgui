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

func column() ui.StyleBuilder {
	return ui.Style().
		Display("flex").
		FlexDirection("column").
		MinHeight(0).
		MinWidth(0)
}
func row() ui.StyleBuilder {
	return ui.Style().
		Display("flex").
		AlignItems("center").
		MinWidth(0)
}
func caption(value any) {
	ui.Text(value).FontSize(12).LineHeight(18).TextColor(muted)
}
func control(label any, click func(), disabled func() bool) {
	ui.Button(
		label,
		ui.Style().Display("flex"),
		ui.Style().AlignItems("center"),
		ui.Style().JustifyContent("center"),
		ui.Style().Height(32),
		ui.Style().PaddingLeft(12),
		ui.Style().PaddingRight(12),
		ui.Style().BorderWidth(1),
		ui.Style().BorderColor("#dce0e7"),
		ui.Style().BorderRadius(6),
		ui.Style().BackgroundColor("white"),
		ui.Style().FontSize(12),
		ui.Style().DisabledStyle(func(s ui.StyleBuilder) ui.StyleBuilder { return s.Opacity(0.4) }),
		ui.OnClick(click),
		ui.Disabled(disabled),
	)
}
func property(label string, value any) {
	ui.View(
		func() {
			caption(label)
			ui.Text(value).FontSize(12).FontWeight(500)
		},
		row(),
	).JustifyContent("space-between")
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
					).FontSize(22).FontWeight(700).PaddingLeft(12).MarginBottom(6)
					ui.Text(
						"Product workspace",
					).FontSize(12).TextColor(muted).PaddingLeft(12).MarginBottom(24)
					for _, name := range []string{"All issues", "Open", "Completed"} {
						ui.Button(
							name,
							ui.Style().Display("flex"),
							ui.Style().Height(42),
							ui.Style().PaddingLeft(12),
							ui.Style().AlignItems("center"),
							ui.Style().JustifyContent("flex-start"),
							ui.Style().BorderWidth(0),
							ui.Style().BorderRadius(7),
							ui.Style().
								FontWeight(func() int {
									if filter() == name {
										return 600
									}
									return 400
								}),
							ui.Style().
								BackgroundColor(func() string {
									if filter() == name {
										return "#e4ebfb"
									}
									return "transparent"
								}),
							ui.Style().
								TextColor(func() string {
									if filter() == name {
										return accent
									}
									return ink
								}),
							ui.OnClick(func() { ui.Batch(func() { setFilter(name); setPage(0) }) }),
						)
					}
					ui.View().Flex(1)
					ui.Text(
						"September cycle\n4 projects · 5 teammates",
					).FontSize(12).LineHeight(18).TextColor(muted).Padding(12)
				},
				column(),
			).Width(176).FlexShrink(0).PaddingTop(24).PaddingBottom(24).PaddingLeft(12).PaddingRight(12).Gap(8).BackgroundColor("#f4f5f7").BorderRightWidth(1).BorderColor(line)
			ui.View(
				func() {
					ui.View(
						func() {
							ui.View(
								func() {
									ui.Text("Issue inbox").FontSize(24).FontWeight(700)
									caption(func() string {
										return strconv.Itoa(len(issues)-completed()) + " open · " + strconv.Itoa(completed()) + " completed"
									})
								},
								column(),
							).Gap(6)
							ui.Input(
								ui.Value(query),
								ui.Placeholder("Search issues, projects, people"),
								ui.AriaLabel("Search issues"),
								ui.OnInput(func(e *native.Event) {
									ui.Batch(func() { setQuery(e.Value); setPage(0) })
								}),
							).Width(280).Height(38).PaddingLeft(12).PaddingRight(12).BackgroundColor("#f8f9fb").BorderWidth(1).BorderColor("#dce0e7").BorderRadius(7)
						},
						row(),
					).Height(94).FlexShrink(0).PaddingLeft(24).PaddingRight(24).JustifyContent("space-between").BorderBottomWidth(1).BorderColor(line)
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
									).Height(48).FlexShrink(0).PaddingLeft(20).PaddingRight(20).JustifyContent("space-between").BackgroundColor("#fafbfc").BorderBottomWidth(1).BorderColor(line)
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
																	).FontSize(14).FontWeight(500).WhiteSpace("nowrap").TextOverflow("ellipsis").Overflow("hidden")
																	ui.Text(
																		func() string {
																			return item.ID + "  ·  " + item.Project + "  ·  " + item.status() + "  ·  " + item.Owner
																		},
																	).FontSize(11).TextColor(muted).WhiteSpace("nowrap")
																},
																column(),
																ui.Style().Height(68),
																ui.Style().FlexShrink(0),
																ui.Style().JustifyContent("center"),
																ui.Style().AlignItems("stretch"),
																ui.Style().Gap(8),
																ui.Style().PaddingLeft(20),
																ui.Style().PaddingRight(20),
																ui.Style().BorderWidth(0),
																ui.Style().BorderBottomWidth(1),
																ui.Style().BorderRadius(0),
																ui.Style().BorderColor("#edf0f4"),
																ui.Style().
																	BackgroundColor(func() string {
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
											).Flex(1).OverflowY("scroll")
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
											).Gap(8)
										},
										row(),
									).Height(58).FlexShrink(0).JustifyContent("space-between").PaddingLeft(20).PaddingRight(20).BorderTopWidth(1).BorderColor(line)
								},
								column(),
							).Flex(1)
							ui.View(
								func() {
									ui.View(
										func() {
											caption(func() string { return current().ID + " / " + current().Project })
											ui.Text(
												func() string { return current().Title },
											).FontSize(21).LineHeight(28).FontWeight(700).MarginTop(14).MarginBottom(22)
											ui.View(
												func() {
													property("Status", func() string { return current().status() })
													property("Assignee", func() string { return current().Owner })
													property("Priority", func() string { return current().Priority })
												},
												column(),
											).Gap(12).MarginBottom(24)
											ui.Text(
												func() string { return current().Description },
											).FontSize(13).LineHeight(20).MarginBottom(22)
											ui.Text(
												"Working notes",
											).FontSize(12).FontWeight(600).MarginBottom(8)
											ui.TextArea(
												ui.Value(func() string { return current().notes() }),
												ui.AriaLabel("Working notes"),
												ui.OnInput(func(e *native.Event) {
													current().setNotes(e.Value)
												}),
											).Height(100).FlexShrink(0).Padding(10).FontSize(13).LineHeight(19).BorderWidth(1).BorderColor("#dce0e7").BorderRadius(7)
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
											).Height(36).FlexShrink(0).JustifyContent("center").BorderWidth(0).BorderRadius(7).MarginTop(16).BackgroundColor(accent).TextColor("white").FontWeight(500)
											ui.Text(
												"Changes are kept for this session.",
											).FontSize(11).TextColor(muted).MarginTop(10)
										},
										column(),
									).FlexShrink(0)
								},
								column(),
							).Width(350).FlexShrink(0).OverflowY("auto").Padding(24).BorderLeftWidth(1).BorderColor(line)
						},
						row(),
					).Flex(1).MinHeight(0).AlignItems("stretch")
				},
				column(),
			).Flex(1)
		},
		row(),
	).Width("100%").Height("100%").AlignItems("stretch").TextColor(ink).FontSize(14).BackgroundColor("white")
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
