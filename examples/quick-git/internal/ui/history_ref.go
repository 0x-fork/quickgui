package ui

import (
	gui "github.com/egoist/quickgui/go/ui"
	"quickgui.example/quick-git/internal/git"
)

func commitRefBadge(ref git.CommitRef) {
	theme := UseApp().Theme()
	background, color := theme.AccentWash, theme.Accent
	if ref.Current {
		background, color = theme.Accent, theme.TextOnAccent
	} else if ref.Kind == "tag" {
		background, color = theme.WarningWash, theme.Warning
	}
	gui.View(
		func() {
			gui.Text(
				ref.Name,
				gui.FontSize(10.5),
				gui.FontWeight(700),
				gui.TextColor(color),
				gui.LineClamp(1),
				gui.TextOverflow("ellipsis"),
			)
		},
		gui.Display("flex"),
		gui.Height(16),
		gui.MinWidth(0),
		gui.MaxWidth(140),
		gui.AlignItems("center"),
		gui.PaddingLeft(5),
		gui.PaddingRight(5),
		gui.BorderRadius(4),
		gui.BackgroundColor(background),
	)
}

// Reserve space for the subject even when a commit has several long ref names.
func commitRefs(read func() []git.CommitRef) {
	visible := gui.CreateMemo(func() []git.CommitRef {
		refs := make([]git.CommitRef, 0, 3)
		for _, ref := range read() {
			if ref.Kind == "head" {
				continue
			}
			refs = append(refs, ref)
			if len(refs) == 3 {
				break
			}
		}
		return refs
	})
	gui.Show(
		func() bool { return len(visible()) > 0 },
		func() {
			gui.View(
				func() {
					gui.For(
						visible,
						func(ref git.CommitRef, _ func() int) { commitRefBadge(ref) },
						func(ref git.CommitRef) any { return string(ref.Kind) + ":" + ref.Name },
						nil,
					)
				},
				gui.Display("flex"),
				gui.FlexDirection("row"),
				gui.AlignItems("center"),
				gui.MinWidth(0),
				gui.MaxWidth("48%"),
				gui.FlexShrink(1),
				gui.Overflow("hidden"),
				gui.Gap(4),
			)
		},
	)
}
