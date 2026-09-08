package ui

import (
	"fmt"
	"strconv"

	gui "github.com/egoist/quickgui/go/ui"
	"quickgui.example/quick-git/internal/model"
)

func Toolbar() {
	app := UseApp()
	store := app.Store
	busy := func() bool { return store.Busy() != nil }
	pullDisabled := func() bool {
		status := store.Status()
		return busy() || status == nil || status.Upstream == ""
	}
	pushDisabled := func() bool {
		status := store.Status()
		return busy() || status == nil || status.Branch == ""
	}
	gui.View(
		func() {
			gui.Button(
				func() {
					toolbarIcon(branchIcon)
					gui.Text(
						func() string {
							if status := store.Status(); status != nil {
								if status.Branch != "" {
									return status.Branch
								}
								if status.Detached && len(status.HeadSha) >= 7 {
									return status.HeadSha[:7] + " (detached)"
								}
							}
							return "…"
						},
						gui.MinWidth(0),
						gui.LineClamp(1),
					)
				},
				gui.OnClick(func() { store.SetView(model.ViewBranches) }),
				app.Theme().Button("secondary"),
				gui.FlexDirection("row"),
				gui.Height(28),
				gui.MaxWidth(260),
				gui.BackgroundColor("transparent"),
				gui.BorderWidth(0),
			)
			gui.Show(
				func() bool {
					status := store.Status()
					return status != nil && status.HasUpstreamCounts && (status.Ahead > 0 || status.Behind > 0)
				},
				func() {
					gui.View(
						func() {
							toolbarIcon(pushIcon)
							gui.Text(func() string { return fmt.Sprint(store.Status().Ahead) })
							toolbarIcon(pullIcon)
							gui.Text(func() string { return fmt.Sprint(store.Status().Behind) })
						},
						gui.AriaLabel("Upstream commit counts"),
						gui.Display("flex"),
						gui.AlignItems("center"),
						gui.Gap(3),
						gui.FontSize(11),
						gui.Color(app.Theme().TextSecondary),
						gui.AppRegion("no-drag"),
					)
				},
			)
			gui.Show(
				func() bool { return store.Conflicts() > 0 },
				func() {
					gui.Text(
						func() string { return strconv.Itoa(store.Conflicts()) + " conflicted" },
						gui.FontSize(11),
						gui.FontWeight(600),
						gui.Color(app.Theme().Warning),
						gui.AppRegion("no-drag"),
					)
				},
			)
			gui.View(gui.Flex(1), gui.AppRegion("drag"))
			gui.Show(busy, toolbarBusy)
			toolbarButton("Fetch", fetchIcon, store.Fetch, busy, false)
			toolbarButton("Pull", pullIcon, store.Pull, pullDisabled, false)
			toolbarButton("Push", pushIcon, store.Push, pushDisabled, false)
			toolbarButton("Refresh", refreshIcon, store.Refresh, busy, true)
		},
		gui.Display("flex"),
		gui.Height(TitlebarHeight),
		gui.FlexShrink(0),
		gui.AlignItems("center"),
		gui.Gap(6),
		gui.PaddingLeft(14),
		gui.PaddingRight(12),
		gui.BorderBottomWidth(1),
		gui.BorderColor(app.Theme().Border),
		gui.BackgroundColor(app.Theme().Content),
		gui.AppRegion("drag"),
	)
}

func toolbarBusy() {
	app := UseApp()
	store := app.Store
	gui.View(
		func() {
			gui.Progress.Root(
				gui.ProgressProps{
					GaugeFormatProps: gui.GaugeFormatProps{
						PartProps: gui.PartProps{
							AriaLabel: "Git operation in progress",
							Style:     gui.Styles(gui.Width(32), gui.Height(4), gui.FlexShrink(0)),
						},
					},
					Indeterminate: func() bool { return true },
				},
				func() {
					gui.Progress.Track(
						gui.PartProps{
							Style: gui.Styles(
								gui.Width("100%"),
								gui.Height(4),
								gui.BorderRadius(2),
								gui.BackgroundColor(app.Theme().BorderStrong),
							),
						},
						func() {
							gui.Progress.Indicator(gui.PartProps{
								Style: gui.Styles(
									gui.Width(12),
									gui.Height(4),
									gui.BorderRadius(2),
									gui.BackgroundColor(app.Theme().TextSecondary),
								),
							})
						},
					)
				},
			)
			gui.Text(
				func() string {
					if busy := store.Busy(); busy != nil {
						return busy.Label + "…"
					}
					return ""
				},
				gui.FontSize(12),
				gui.Color(app.Theme().TextSecondary),
			)
			gui.Show(
				func() bool {
					busy := store.Busy()
					return busy != nil && busy.Cancel != nil
				},
				func() {
					gui.Button(
						"Cancel",
						gui.OnClick(store.CancelBusy),
						app.Theme().Button("secondary"),
					)
				},
			)
		},
		gui.Display("flex"),
		gui.FlexDirection("row"),
		gui.AlignItems("center"),
		gui.Gap(8),
		gui.MarginRight(8),
		gui.AppRegion("no-drag"),
	)
}

func toolbarButton(label, icon string, click func(), disabled func() bool, iconOnly bool) {
	app := UseApp()
	gui.Button(
		func() {
			toolbarIcon(icon)
			if !iconOnly {
				gui.Text(label)
			}
		},
		gui.AriaLabel(label),
		gui.OnClick(click),
		gui.Disabled(disabled),
		app.Theme().Button("secondary"),
		gui.FlexDirection("row"),
		gui.Height(28),
		gui.BorderRadius(7),
	)
}
