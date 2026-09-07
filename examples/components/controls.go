package main

import (
	"fmt"
	"slices"
	"strconv"
	"strings"

	"github.com/egoist/quickgui/go/native"
	"github.com/egoist/quickgui/go/ui"
)

const gaugeWidth = 420

func gaugeReadout() {
	state := ui.UseGaugeState()
	note(func() string {
		s := state()
		return "status " + s.Status + " · display " + textValue(s.DisplayValue) + " · completion " + textValue(s.Completion)
	})
}
func gaugeTrack() ui.PartProps {
	return ui.PartProps{Style: func() ui.Style {
		return ui.Style{
			Display:         "flex",
			Width:           gaugeWidth,
			Height:          8,
			BorderRadius:    4,
			BackgroundColor: p().Track,
			Overflow:        "hidden",
		}
	}}
}
func sliderParts(values func() []float64) {
	ui.Slider.Control(
		ui.PartProps{Style: ui.Style{
			Position:   "relative",
			Display:    "flex",
			AlignItems: "center",
			Width:      gaugeWidth,
			Height:     24,
		}},
		func() {
			ui.Slider.Track(
				ui.PartProps{Style: func() ui.Style {
					return ui.Style{
						Display:         "flex",
						Width:           gaugeWidth,
						Height:          4,
						BorderRadius:    2,
						BackgroundColor: p().Track,
					}
				}},
				func() {
					style := func() ui.Style {
						v := values()
						left, width := 0.0, v[0]
						if len(v) > 1 {
							left, width = v[0], v[1]-v[0]
						}
						return ui.Style{
							Height:          4,
							BorderRadius:    2,
							BackgroundColor: p().Accent,
							MarginLeft:      left * gaugeWidth / 100,
							Width:           strconv.FormatFloat(width, 'g', -1, 64) + "%",
						}
					}
					if len(values()) > 1 {
						ui.Slider.Range(ui.PartProps{Style: style})
					} else {
						ui.Slider.Indicator(ui.PartProps{Style: style})
					}
				},
			)
			for i := range values() {
				ui.Slider.Thumb(ui.SliderThumbProps{
					Index: ptr(i),
					PartProps: ui.PartProps{Style: func() ui.Style {
						return ui.Style{
							Position:        "absolute",
							Left:            values()[i]*gaugeWidth/100 - 10,
							Top:             2,
							Width:           20,
							Height:          20,
							BorderRadius:    10,
							BackgroundColor: p().Panel,
							BorderWidth:     1,
							BorderColor:     p().Border,
							BoxShadow:       "0 1px 2px #0000003d",
							Focus:           &ui.Style{Outline: "2px solid " + p().Accent},
							OutlineOffset:   2,
						}
					}},
				})
			}
		},
	)
}
func MeterDemo() {
	level, setLevel := ui.CreateSignal([]float64{62})
	panel("Meter", "A known range, with low/high/optimum thresholds and the core's formatted gauge state.", func() {
		ui.Meter.Root(
			ui.MeterProps{
				Value:   func() *float64 { return ptr(level()[0]) },
				Min:     0,
				Max:     100,
				Low:     25,
				High:    80,
				Optimum: 50,
				GaugeFormatProps: ui.GaugeFormatProps{
					Format:    "percent",
					PartProps: columnPart(),
				},
			},
			func() {
				row(func() {
					ui.Meter.Label(ui.PartProps{}, "Disk used")
					ui.Meter.Value(
						ui.PartProps{},
						func() {
							label(func() string { return strconv.FormatFloat(level()[0], 'f', 0, 64) + "%" })
						},
					)
				})
				ui.Meter.Track(
					gaugeTrack(),
					func() {
						ui.Meter.Indicator(ui.PartProps{Style: func() ui.Style {
							value := level()[0]
							return ui.Style{
								Height:          8,
								BorderRadius:    4,
								Width:           strconv.FormatFloat(value, 'g', -1, 64) + "%",
								BackgroundColor: choose(value < 25, p().Danger, choose(value > 80, "#c88a00", p().Accent)),
							}
						}})
					},
				)
				gaugeReadout()
			},
		)
		ui.Slider.Root(
			ui.SliderRootProps{
				Value:         level,
				Min:           0,
				Max:           100,
				Step:          1,
				OnValueChange: change(setLevel),
			},
			func() { sliderParts(level) },
		)
	})
}
func ProgressDemo() {
	done, setDone := ui.CreateSignal(3)
	indeterminate, setIndeterminate := ui.CreateSignal(false)
	panel("Progress", "Determinate and indeterminate task progress. No timer keeps the gallery awake.", func() {
		ui.Progress.Root(
			ui.ProgressProps{
				Value: func() *float64 {
					if indeterminate() {
						return nil
					}
					return ptr(float64(done()))
				},
				Max:           12,
				Indeterminate: indeterminate,
				ValueText:     func() *string { return ptr(strconv.Itoa(done()) + " of 12 files") },
				GaugeFormatProps: ui.GaugeFormatProps{
					Format:    "fraction",
					PartProps: columnPart(),
				},
			},
			func() {
				row(func() {
					ui.Progress.Label(ui.PartProps{}, "Uploading")
					ui.Progress.Value(
						ui.PartProps{},
						func() {
							label(func() string { return choose(indeterminate(), "…", strconv.Itoa(done())+" / 12") })
						},
					)
				})
				ui.Progress.Track(
					gaugeTrack(),
					func() {
						ui.Progress.Indicator(ui.PartProps{Style: func() ui.Style {
							return ui.Style{
								Height:          8,
								BorderRadius:    4,
								BackgroundColor: p().Accent,
								Width:           strconv.FormatFloat(choose(indeterminate(), 35.0, float64(done())/12*100), 'g', -1, 64) + "%",
							}
						}})
					},
				)
				gaugeReadout()
			},
		)
		row(func() {
			button("−1", func() { setDone(max(0, done()-1)) })
			button("+1", func() { setDone(min(12, done()+1)) })
			button("Complete", func() { setDone(12) })
			button(func() string { return choose(indeterminate(), "Determinate", "Indeterminate") }, func() { setIndeterminate(!indeterminate()) })
		})
	})
}
func RadioDemo() {
	theme, setTheme := ui.CreateSignal("system")
	radio := func(value string, selected func() bool, caption string) {
		ui.Radio.Root(
			ui.RadioProps{Value: value, PartProps: ui.PartProps{Style: checkboxStyle()}},
			func() {
				ui.Radio.Indicator(ui.PartProps{Style: func() ui.Style {
					return ui.Style{
						Width:           15,
						Height:          15,
						BorderRadius:    8,
						BorderWidth:     choose(selected(), 4, 1),
						BorderColor:     choose(selected(), p().Accent, p().Border),
						BackgroundColor: p().Control,
					}
				}})
				label(caption)
			},
		)
	}
	panel("Radio", "Arrow keys select within a group; read-only groups retain their single Tab stop.", func() {
		ui.RadioGroup.Root(
			ui.RadioGroupProps{
				Value:         func() *string { return ptr(theme()) },
				OnValueChange: change(setTheme),
				Required:      true,
				PartProps:     columnPart(),
			},
			func() {
				for _, value := range []string{"light", "dark", "system"} {
					radio(value, func() bool { return theme() == value }, value)
				}
			},
		)
		ui.RadioGroup.Root(
			ui.RadioGroupProps{
				DefaultValue: "b",
				ReadOnly:     true,
				PartProps:    rowPart(),
			},
			func() {
				for _, value := range []string{"a", "b", "c"} {
					radio(value, func() bool { return value == "b" }, "read-only "+value)
				}
			},
		)
		note(func() string { return "theme " + theme() + " · read-only group b" })
	})
}
func SeparatorDemo() {
	panel("Separator", "Orientation, with application-defined thickness and colour. An adjustable divider is Splitter.", func() {
		label("Above the rule")
		ui.Separator.Root(ui.SeparatorProps{
			Orientation: "horizontal",
			PartProps: ui.PartProps{Style: ui.Style{
				Height:          1,
				BackgroundColor: color(func(p palette) string { return p.Border }),
			}},
		})
		label("Below the rule")
		row(func() {
			for i, caption := range []string{"Cut", "Copy", "Paste"} {
				if i > 0 {
					ui.Separator.Root(ui.SeparatorProps{
						Orientation: "vertical",
						PartProps: ui.PartProps{Style: ui.Style{
							Width:           1,
							Height:          18,
							BackgroundColor: color(func(p palette) string { return p.Border }),
						}},
					})
				}
				label(caption)
			}
		})
	})
}
func SliderDemo() {
	volume, setVolume := ui.CreateSignal([]float64{40})
	committed, setCommitted := ui.CreateSignal("—")
	interval, setInterval := ui.CreateSignal([]float64{20, 70})
	panel("Slider", "Clamping, step snapping, thumb ordering, and captured drag arithmetic belong to the core.", func() {
		ui.Slider.Root(
			ui.SliderRootProps{
				Value:            volume,
				Min:              0,
				Max:              100,
				Step:             5,
				LargeStep:        25,
				Format:           "percent",
				OnValueChange:    change(setVolume),
				OnValueCommitted: func(v []float64, _ *native.Event) { setCommitted(fmt.Sprint(v)) },
				PartProps:        columnPart(),
			},
			func() {
				ui.Slider.Label(ui.PartProps{}, "Volume")
				ui.Slider.Value(
					ui.PartProps{},
					func() {
						state := ui.UseSliderState()
						label(func() string {
							return textValue(state().DisplayValue) + " · dragging " + strconv.FormatBool(state().Dragging)
						})
					},
				)
				sliderParts(volume)
			},
		)
		note(func() string { return fmt.Sprintf("volume %v · committed %s", volume(), committed()) })
		ui.Slider.Root(
			ui.SliderRootProps{
				Value:                 interval,
				Min:                   0,
				Max:                   100,
				Step:                  1,
				MinStepsBetweenValues: 5,
				OnValueChange:         change(setInterval),
				PartProps:             columnPart(),
			},
			func() {
				ui.Slider.Label(
					ui.PartProps{},
					"Range · five steps between thumbs",
				)
				sliderParts(interval)
			},
		)
		note(func() string { return fmt.Sprintf("range %v", interval()) })
	})
}
func SplitterDemo() {
	sizes, setSizes := ui.CreateSignal([]float64{200, 160, 160})
	panel("Splitter", "Drag or use arrow keys. The core conserves the total, enforces minima, and can collapse the last pane.", func() {
		ui.Splitter.Root(
			ui.SplitterRootProps{
				Value:         sizes,
				Step:          8,
				OnSizesChange: change(setSizes),
				Panes:         []ui.SplitterPaneDeclaration{{Min: ptr(80.0)}, {Min: ptr(80.0)}, {Min: ptr(60.0), Collapsible: true}},
				PartProps: ui.PartProps{Style: ui.Style{
					Display:         "flex",
					Height:          140,
					BorderRadius:    10,
					BorderWidth:     1,
					BorderColor:     color(func(p palette) string { return p.Border }),
					Overflow:        "hidden",
					BackgroundColor: color(func(p palette) string { return p.PanelAlt }),
				}},
			},
			func() {
				for i := range 3 {
					if i > 0 {
						ui.Splitter.Handle(ui.SplitterPaneProps{
							Index: ptr(i - 1),
							PartProps: ui.PartProps{Style: ui.Style{
								Width:           6,
								BackgroundColor: color(func(p palette) string { return p.Border }),
								Cursor:          "col-resize",
							}},
						})
					}
					ui.Splitter.Pane(
						ui.SplitterPaneProps{
							Index: ptr(i),
							PartProps: ui.PartProps{Style: ui.Style{
								Display:        "flex",
								AlignItems:     "center",
								JustifyContent: "center",
							}},
						},
						func() {
							muted(func() string {
								return "pane " + strconv.Itoa(i) + " · " + strconv.FormatFloat(sizes()[i], 'f', 0, 64) + "px"
							})
						},
					)
				}
			},
		)
		note(func() string { return fmt.Sprintf("sizes %v", sizes()) })
	})
}
func switchControl(checked func() bool, set func(bool), readOnly bool, caption string) {
	row(func() {
		ui.Switch.Root(
			ui.SwitchProps{
				Checked:         checked,
				OnCheckedChange: change(set),
				ReadOnly:        readOnly,
				PartProps: ui.PartProps{
					AriaLabel: caption,
					Style: func() ui.Style {
						return ui.Style{
							Width:           44,
							Height:          24,
							BorderRadius:    12,
							Padding:         2,
							Display:         "flex",
							AlignItems:      "center",
							BackgroundColor: choose(checked(), p().Accent, p().Track),
						}
					},
				},
			},
			func() {
				ui.Switch.Thumb(ui.PartProps{Style: func() ui.Style {
					return ui.Style{
						Width:           20,
						Height:          20,
						BorderRadius:    10,
						BackgroundColor: "white",
						MarginLeft:      choose(checked(), 20, 0),
					}
				}})
			},
		)
		label(caption)
	})
}
func SwitchDemo() {
	wifi, setWifi := ui.CreateSignal(true)
	beta, setBeta := ui.CreateSignal(false)
	managed, setManaged := ui.CreateSignal(true)
	panel("Switch", "On/off state with a switch role, including a focusable read-only example.", func() {
		switchControl(wifi, setWifi, false, "Wi-Fi")
		switchControl(beta, setBeta, false, "Beta updates")
		switchControl(managed, setManaged, true, "Managed by policy · read-only")
		note(func() string {
			return "wifi " + strconv.FormatBool(wifi()) + " · beta " + strconv.FormatBool(beta()) + " · managed " + strconv.FormatBool(managed())
		})
	})
}
func TabsDemo() {
	tab, setTab := ui.CreateSignal("overview")
	panel("Tabs", "Automatic activation, keyboard navigation, lazy panels, and measured indicator geometry.", func() {
		ui.Tabs.Root(
			ui.TabsRootProps{
				Value:         func() *string { return ptr(tab()) },
				OnValueChange: change(setTab),
				Activation:    "automatic",
				Orientation:   "horizontal",
				PartProps:     columnPart(),
			},
			func() {
				ui.Tabs.List(
					rowPart(),
					func() {
						for i, value := range []string{"overview", "usage", "limits"} {
							ui.Tabs.Tab(
								ui.TabsTabProps{
									Value: value,
									Index: ptr(i),
									PartProps: ui.PartProps{Style: func() ui.Style {
										s := controlStyle()
										s.BackgroundColor = choose(tab() == value, p().Selection, "transparent")
										return s
									}},
								},
								value,
							)
						}
						ui.Tabs.Indicator(ui.TabsIndicatorProps{
							Placement: "bottom",
							PartProps: ui.PartProps{Style: ui.Style{
								Height:          2,
								BackgroundColor: color(func(p palette) string { return p.Accent }),
							}},
						})
					},
				)
				state := ui.UseTabsState()
				note(func() string {
					return fmt.Sprintf("activation %s · indicator %+v", state().ActivationDirection, state().Indicator)
				})
				ui.Tabs.Panel(
					ui.TabsPanelProps{Value: "overview"},
					"An inactive panel contributes no layout or paint.",
				)
				ui.Tabs.Panel(
					ui.TabsPanelProps{Value: "usage"},
					"Arrow keys move focus and select automatically.",
				)
				ui.Tabs.Panel(
					ui.TabsPanelProps{Value: "limits"},
					"KeepMounted retains inactive panels with display:none.",
				)
			},
		)
		note(func() string { return "active " + tab() })
	})
}
func togglePart(pressed func() bool) ui.PartProps {
	return ui.PartProps{Style: func() ui.Style {
		s := controlStyle()
		s.BackgroundColor = choose(pressed(), p().Selection, p().Control)
		s.BorderColor = choose(pressed(), p().Accent, p().Border)
		return s
	}}
}
func ToggleDemo() {
	bold, setBold := ui.CreateSignal(false)
	pinned, setPinned := ui.CreateSignal(true)
	panel("Toggle", "Buttons that stay pressed, with native toggle-button semantics.", func() {
		row(func() {
			ui.Toggle.Root(
				ui.ToggleProps{
					Pressed:         bold,
					OnPressedChange: change(setBold),
					PartProps:       togglePart(bold),
				},
				"B",
			)
			ui.Toggle.Root(
				ui.ToggleProps{
					Pressed:         pinned,
					OnPressedChange: change(setPinned),
					PartProps:       togglePart(pinned),
				},
				func() {
					ui.Toggle.Indicator(ui.PartProps{Style: ui.Style{
						Width:           6,
						Height:          6,
						BorderRadius:    3,
						BackgroundColor: color(func(p palette) string { return p.Accent }),
					}})
					label("Pinned")
				},
			)
		})
		note(func() string {
			return "bold " + strconv.FormatBool(bold()) + " · pinned " + strconv.FormatBool(pinned())
		})
	})
}
func ToggleGroupDemo() {
	align, setAlign := ui.CreateSignal([]string{"left"})
	formats, setFormats := ui.CreateSignal([]string{"bold"})
	panel("Toggle group", "Single and multiple selection, roving focus, and a disabled item.", func() {
		for _, group := range []struct {
			values   func() []string
			set      func([]string)
			items    []string
			multiple bool
		}{{align, setAlign, []string{"left", "center", "right"}, false}, {formats, setFormats, []string{"bold", "italic", "underline"}, true}} {
			items := []ui.ComponentItem{}
			for _, value := range group.items {
				items = append(items, ui.ComponentItem{Value: value, Disabled: value == "underline"})
			}
			ui.ToggleGroup.Root(
				ui.ToggleGroupProps{
					Value:         group.values,
					OnValueChange: change(group.set),
					Multiple:      group.multiple,
					Items:         items,
					PartProps:     rowPart(),
				},
				func() {
					for _, value := range group.items {
						part := togglePart(func() bool { return slices.Contains(group.values(), value) })
						part.Disabled = value == "underline"
						ui.ToggleGroup.Item(
							ui.ToggleGroupItemProps{Value: value, PartProps: part},
							value,
						)
					}
				},
			)
		}
		note(func() string {
			return "align [" + strings.Join(align(), ", ") + "] · formats [" + strings.Join(formats(), ", ") + "]"
		})
	})
}
func ToolbarDemo() {
	tool, setTool := ui.CreateSignal("select")
	query, setQuery := ui.CreateSignal("")
	items := []ui.ComponentItem{{Value: "select"}, {Value: "pen"}, {Value: "erase", Disabled: true}, {Value: "share"}, {Value: "docs"}, {Value: "find"}}
	panel("Toolbar", "One roving Tab stop across items, a button, a link, and an input. Arrow keys skip the disabled item.", func() {
		ui.Toolbar.Root(
			ui.ToolbarRootProps{
				Active:         func() *string { return ptr(tool()) },
				OnActiveChange: change(setTool),
				Items:          items,
				Orientation:    "horizontal",
				PartProps:      rowPart(),
			},
			func() {
				ui.Toolbar.Group(
					rowPart(),
					func() {
						for _, value := range []string{"select", "pen", "erase"} {
							part := togglePart(func() bool { return tool() == value })
							part.Disabled = value == "erase"
							ui.Toolbar.Item(
								ui.ToolbarItemProps{Value: value, PartProps: part},
								value,
							)
						}
					},
				)
				ui.Toolbar.Separator(ui.PartProps{Style: ui.Style{
					Width:           1,
					Height:          20,
					BackgroundColor: color(func(p palette) string { return p.Border }),
				}})
				ui.Toolbar.Button(
					ui.ToolbarItemProps{Value: "share", PartProps: control()},
					"Share",
				)
				ui.Toolbar.Link(ui.ToolbarItemProps{Value: "docs", PartProps: control()}, "Docs")
				style := inputStyle()
				style.Width = 140
				ui.Toolbar.Input(ui.ToolbarInputProps{
					PartValue: "find",
					InputPartProps: ui.InputPartProps{
						Value:       query,
						Placeholder: "Find",
						OnInput:     func(e *native.Event) { setQuery(e.Value) },
						PartProps:   ui.PartProps{Style: style},
					},
				})
			},
		)
		note(func() string { return "active " + tool() + " · find " + query() })
	})
}
