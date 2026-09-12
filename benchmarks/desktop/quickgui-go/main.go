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
func caption(value any) *ui.Element {
	return ui.Text(value).FontSize(12).LineHeight(18).TextColor(muted)
}
func control(label any, click func(), disabled func() bool) *ui.Element {
	return ui.Button().Child(label).Style(row()).JustifyContent("center").Height(32).
		PaddingLeft(12).PaddingRight(12).BorderWidth(1).BorderColor("#dce0e7").
		BorderRadius(6).BackgroundColor("white").FontSize(12).
		DisabledStyle(func(s ui.StyleBuilder) ui.StyleBuilder { return s.Opacity(0.4) }).
		OnClick(click).Disabled(disabled)
}
func property(label string, value any) *ui.Element {
	return ui.View().Children(caption(label), ui.Text(value).FontSize(12).FontWeight(500)).
		Style(row()).JustifyContent("space-between")
}

func issueTracker() *ui.Element {
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

	sidebar := ui.View().Children(
		ui.Text("Orbit").FontSize(22).FontWeight(700).PaddingLeft(12).MarginBottom(6),
		caption("Product workspace").PaddingLeft(12).MarginBottom(24),
		ui.For(
			func() []string { return []string{"All issues", "Open", "Completed"} },
			func(name string, _ func() int) *ui.Element {
				return ui.Button().Child(name).Style(row()).Height(42).PaddingLeft(12).
					JustifyContent("flex-start").BorderWidth(0).BorderRadius(7).
					FontWeight(func() int {
						if filter() == name {
							return 600
						}
						return 400
					}).
					BackgroundColor(func() string {
						if filter() == name {
							return "#e4ebfb"
						}
						return "transparent"
					}).
					TextColor(func() string {
						if filter() == name {
							return accent
						}
						return ink
					}).
					OnClick(func() { ui.Batch(func() { setFilter(name); setPage(0) }) })
			},
			func(name string) any { return name },
			nil,
		),
		ui.View().Flex(1),
		caption("September cycle\n4 projects · 5 teammates").Padding(12),
	).Style(column()).Width(176).FlexShrink(0).PaddingTop(24).PaddingBottom(24).
		PaddingLeft(12).PaddingRight(12).Gap(8).BackgroundColor("#f4f5f7").BorderRightWidth(1).BorderColor(line)

	header := ui.View().Children(
		ui.View().Children(
			ui.Text("Issue inbox").FontSize(24).FontWeight(700),
			caption(func() string {
				return strconv.Itoa(len(issues)-completed()) + " open · " + strconv.Itoa(completed()) + " completed"
			}),
		).Style(column()).Gap(6),
		ui.Input().Value(query).Placeholder("Search issues, projects, people").AriaLabel("Search issues").
			OnInputEvent(func(e *native.Event) { ui.Batch(func() { setQuery(e.Value); setPage(0) }) }).
			Width(280).Height(38).PaddingLeft(12).PaddingRight(12).BackgroundColor("#f8f9fb").
			BorderWidth(1).BorderColor("#dce0e7").BorderRadius(7),
	).Style(row()).Height(94).FlexShrink(0).PaddingLeft(24).PaddingRight(24).
		JustifyContent("space-between").BorderBottomWidth(1).BorderColor(line)

	inbox := ui.View().Children(
		ui.View().Children(
			caption(func() string { return strconv.Itoa(len(matching())) + " issues" }),
			caption("Updated this week"),
		).Style(row()).Height(48).FlexShrink(0).PaddingLeft(20).PaddingRight(20).
			JustifyContent("space-between").BackgroundColor("#fafbfc").BorderBottomWidth(1).BorderColor(line),
		// A new result page starts at the top; selection and edits keep its viewport.
		ui.For(
			func() []string {
				return []string{query() + "|" + filter() + "|" + strconv.Itoa(currentPage())}
			},
			func(_ string, _ func() int) *ui.Element {
				return ui.View().Child(
					ui.For(
						visible,
						func(index int, _ func() int) *ui.Element {
							item := issues[index]
							return ui.Button().Children(ui.Text(item.Title).FontSize(14).FontWeight(500).WhiteSpace("nowrap").TextOverflow("ellipsis").Overflow("hidden"),ui.Text(func() string {
									return item.ID + "  ·  " + item.Project + "  ·  " + item.status() + "  ·  " + item.Owner
								}).
									FontSize(11).TextColor(muted).WhiteSpace("nowrap")).Style(column()).AriaLabel(item.ID).Height(68).FlexShrink(0).JustifyContent("center").AlignItems("stretch").
								Gap(8).PaddingLeft(20).PaddingRight(20).BorderWidth(0).BorderBottomWidth(1).BorderRadius(0).BorderColor("#edf0f4").
								BackgroundColor(func() string {
									if selected() == index {
										return "#edf3ff"
									}
									return "white"
								}).
								OnClick(func() { setSelected(index) })
						},
						func(index int) any { return index },
						func() *ui.Element { return caption("No matching issues") },
					),
				).Style(column()).Flex(1).OverflowY("scroll")
			},
			func(key string) any { return key },
			nil,
		),
		ui.View().Children(
			caption(func() string { return "Page " + strconv.Itoa(currentPage()+1) + " of " + strconv.Itoa(pages()) }),
			ui.View().Children(
				control("Previous", func() { setPage(currentPage() - 1) }, func() bool { return currentPage() == 0 }),
				control("Next", func() { setPage(currentPage() + 1) }, func() bool { return currentPage()+1 >= pages() }),
			).Style(row()).Gap(8),
		).Style(row()).Height(58).FlexShrink(0).JustifyContent("space-between").PaddingLeft(20).PaddingRight(20).BorderTopWidth(1).BorderColor(line),
	).Style(column()).Flex(1)

	detailsContent := ui.View().Children(
		caption(func() string { return current().ID + " / " + current().Project }),
		ui.Text(func() string { return current().Title }).FontSize(21).LineHeight(28).FontWeight(700).MarginTop(14).MarginBottom(22),
		ui.View().Children(
			property("Status", func() string { return current().status() }),
			property("Assignee", func() string { return current().Owner }),
			property("Priority", func() string { return current().Priority }),
		).Style(column()).Gap(12).MarginBottom(24),
		ui.Text(func() string { return current().Description }).FontSize(13).LineHeight(20).MarginBottom(22),
		ui.Text("Working notes").FontSize(12).FontWeight(600).MarginBottom(8),
		ui.TextArea().Value(func() string { return current().notes() }).AriaLabel("Working notes").
			OnInputEvent(func(e *native.Event) { current().setNotes(e.Value) }).Height(100).FlexShrink(0).
			Padding(10).FontSize(13).LineHeight(19).BorderWidth(1).BorderColor("#dce0e7").BorderRadius(7),
		ui.Button().Child(func() string {
			if current().status() == "Done" {
				return "Reopen issue"
			}
			return "Mark complete"
		}).
			OnClick(func() {
				item := current()
				if item.status() == "Done" {
					item.setStatus("Open")
				} else {
					item.setStatus("Done")
				}
			}).
			Style(row()).Height(36).FlexShrink(0).JustifyContent("center").BorderWidth(0).BorderRadius(7).
			MarginTop(16).BackgroundColor(accent).TextColor("white").FontWeight(500),
		ui.Text("Changes are kept for this session.").FontSize(11).TextColor(muted).MarginTop(10),
	).Style(column()).FlexShrink(0)
	details := ui.View().Child(detailsContent).Style(column()).Width(350).FlexShrink(0).OverflowY("auto").Padding(24).BorderLeftWidth(1).BorderColor(line)
	return ui.View().Children(
		sidebar,
		ui.View().Children(
			header,
			ui.View().Children(inbox, details).Style(row()).Flex(1).MinHeight(0).AlignItems("stretch"),
		).Style(column()).Flex(1),
	).Style(row()).Width("100%").Height("100%").AlignItems("stretch").TextColor(ink).FontSize(14).BackgroundColor("white")
}

func main() {
	if err := native.Run(func() {
		native.NewWindow(native.WindowOptions{
			Title:      "Issue tracker — ready",
			Width:      1100,
			Height:     720,
			Background: "#ffffff",
			Component:  func() *native.Node { return issueTracker().Node },
		})
	}); err != nil {
		log.Fatal(err)
	}
}
