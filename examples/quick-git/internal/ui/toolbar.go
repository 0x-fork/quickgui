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
						gui.Style{MinWidth: 0, LineClamp: 1},
					)
				},
				gui.OnClick(func() { store.SetView(model.ViewBranches) }),
				app.Theme().Button("secondary"),
				gui.Style{
					FlexDirection:   "row",
					Height:          28,
					MaxWidth:        260,
					BackgroundColor: "transparent",
					BorderWidth:     0,
				},
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
						gui.Style{
							Display:    "flex",
							AlignItems: "center",
							Gap:        3,
							FontSize:   11,
							Color:      app.Theme().TextSecondary,
							AppRegion:  "no-drag",
						},
					)
				},
			)
			gui.Show(
				func() bool { return store.Conflicts() > 0 },
				func() {
					gui.Text(
						func() string { return strconv.Itoa(store.Conflicts()) + " conflicted" },
						gui.Style{
							FontSize:   11,
							FontWeight: 600,
							Color:      app.Theme().Warning,
							AppRegion:  "no-drag",
						},
					)
				},
			)
			gui.View(gui.Style{Flex: 1, AppRegion: "drag"})
			gui.Show(busy, toolbarBusy)
			toolbarButton("Fetch", fetchIcon, store.Fetch, busy, false)
			toolbarButton("Pull", pullIcon, store.Pull, pullDisabled, false)
			toolbarButton("Push", pushIcon, store.Push, pushDisabled, false)
			toolbarButton("Refresh", refreshIcon, store.Refresh, busy, true)
		},
		gui.Style{
			Display:           "flex",
			Height:            TitlebarHeight,
			FlexShrink:        0,
			AlignItems:        "center",
			Gap:               6,
			PaddingLeft:       14,
			PaddingRight:      12,
			BorderBottomWidth: 1,
			BorderColor:       app.Theme().Border,
			BackgroundColor:   app.Theme().Content,
			AppRegion:         "drag",
		},
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
							Style:     gui.Style{Width: 32, Height: 4, FlexShrink: 0},
						},
					},
					Indeterminate: func() bool { return true },
				},
				func() {
					gui.Progress.Track(
						gui.PartProps{
							Style: gui.Style{
								Width:           "100%",
								Height:          4,
								BorderRadius:    2,
								BackgroundColor: app.Theme().BorderStrong,
							},
						},
						func() {
							gui.Progress.Indicator(gui.PartProps{
								Style: gui.Style{
									Width:           12,
									Height:          4,
									BorderRadius:    2,
									BackgroundColor: app.Theme().TextSecondary,
								},
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
				gui.Style{FontSize: 12, Color: app.Theme().TextSecondary},
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
		gui.Style{
			Display:       "flex",
			FlexDirection: "row",
			AlignItems:    "center",
			Gap:           8,
			MarginRight:   8,
			AppRegion:     "no-drag",
		},
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
		gui.Style{
			FlexDirection: "row",
			Height:        28,
			BorderRadius:  7,
		},
	)
}
