package main

import (
	"fmt"
	"strconv"
	"strings"
	"time"

	"github.com/egoist/quickgui/go/native"
	"github.com/egoist/quickgui/go/ui"
)

func fieldInput(value func() string, set func(string), placeholder string) ui.FieldControlProps {
	s := inputStyle()
	s = s.Width(280)
	return ui.FieldControlProps{InputPartProps: ui.InputPartProps{
		Value:       value,
		Placeholder: placeholder,
		OnInput:     func(e *native.Event) { set(e.Value) },
		PartProps:   ui.PartProps{Style: s},
	}}
}
func FieldDemo() {
	email, setEmail := ui.CreateSignal("")
	triggers, setTriggers := ui.CreateSignal("waiting for validation state")
	invalid := func() bool { return !strings.Contains(email(), "@") }
	panel("Field", "A control, its label, description, error, and validity. Validation triggers and debounce are reported by the core.", func() {
		ui.Field.Root(
			ui.FieldRootProps{
				Required:               true,
				Invalid:                invalid,
				Filled:                 func() bool { return email() != "" },
				ValidationMode:         "onChange",
				ValidationDebounceTime: 200,
				ValidationMessage:      func() string { return "Enter an address containing @" },
				OnValidationChange: func(v ui.FieldValidationDetails, _ *native.Event) {
					setTriggers(fmt.Sprintf("triggers %+v · delays %+v", v.Triggers, v.Delay))
				},
				PartProps: columnPart(),
			},
			func() {
				ui.Field.Label(ui.FieldLabelProps{}, "Email address")
				ui.Field.Control(fieldInput(email, setEmail, "you@example.com"))
				ui.Field.Description(ui.PartProps{}, "We only use this address for receipts.")
				ui.Field.Error(
					ui.PartProps{Style: ui.Style().
						FontSize(12).
						TextColor(color(func(p palette) string { return p.Danger }))},
					"Enter an address containing @",
				)
				ui.Field.Validity(
					ui.FieldValidityProps{},
					func() {
						note(func() string {
							return "filled " + strconv.FormatBool(email() != "") + " · invalid " + strconv.FormatBool(invalid())
						})
					},
				)
			},
		)
		note(triggers)
	})
}
func FieldsetDemo() {
	saving, setSaving := ui.CreateSignal(false)
	name, setName := ui.CreateSignal("Ada")
	org, setOrg := ui.CreateSignal("Analytical Engines")
	panel("Fieldset", "A semantic group with a legend. A single flag disables every nested field.", func() {
		switchControl(saving, setSaving, false, "Disable while saving")
		props := columnPart()
		props.Disabled = saving
		ui.Fieldset.Root(
			props,
			func() {
				ui.Fieldset.Legend(ui.PartProps{}, "Profile")
				ui.Fieldset.Description(ui.PartProps{}, "Your public identity")
				for _, field := range []struct {
					label string
					value func() string
					set   func(string)
				}{{"Name", name, setName}, {"Organization", org, setOrg}} {
					ui.Field.Root(
						ui.FieldRootProps{PartProps: columnPart()},
						func() {
							ui.Field.Label(ui.FieldLabelProps{}, field.label)
							ui.Field.Control(fieldInput(field.value, field.set, field.label))
						},
					)
				}
			},
		)
		note(func() string { return "disabled " + strconv.FormatBool(saving()) + " · " + name() + " · " + org() })
	})
}
func FormDemo() {
	email, setEmail := ui.CreateSignal("")
	plan, setPlan := ui.CreateSignal("pro")
	terms, setTerms := ui.CreateSignal(false)
	attempts, setAttempts := ui.CreateSignal(0)
	errors, setErrors := ui.CreateSignal(false)
	result, setResult := ui.CreateSignal("nothing submitted yet")
	invalid := func() bool { return !strings.Contains(email(), "@") }
	submit := func() {
		setAttempts(attempts() + 1)
		if invalid() || !terms() {
			setErrors(true)
			setResult("rejected: fix the fields below")
			return
		}
		setErrors(false)
		setResult("accepted: " + email() + " on the " + plan() + " plan")
	}
	panel("Form", "Validation appears on submission. Return in the email field also submits; Reset clears all form state.", func() {
		ui.Fieldset.Root(
			columnPart(),
			func() {
				ui.Fieldset.Legend(ui.PartProps{}, "Sign up")
				ui.Field.Root(
					ui.FieldRootProps{
						Required:          true,
						Invalid:           func() bool { return errors() && invalid() },
						Touched:           func() bool { return attempts() > 0 },
						Filled:            func() bool { return email() != "" },
						ValidationMessage: func() string { return "An address containing @ is required" },
						PartProps:         columnPart(),
					},
					func() {
						ui.Field.Label(ui.FieldLabelProps{}, "Email")
						control := fieldInput(email, setEmail, "you@example.com")
						control.OnSubmit = func(*native.Event) { submit() }
						ui.Field.Control(control)
						ui.Field.Error(
							ui.PartProps{Style: ui.Style().
								FontSize(12).
								TextColor(color(func(p palette) string { return p.Danger }))},
							"An address containing @ is required",
						)
					},
				)
				label("Plan")
				ui.RadioGroup.Root(
					ui.RadioGroupProps{
						Value:         func() *string { return ptr(plan()) },
						OnValueChange: change(setPlan),
						Required:      true,
						PartProps:     rowPart(),
					},
					func() {
						for _, value := range []string{"free", "pro", "team"} {
							ui.Radio.Root(
								ui.RadioProps{
									Value:     value,
									PartProps: togglePart(func() bool { return plan() == value }),
								},
								value,
							)
						}
					},
				)
				checkbox(ui.CheckboxProps{
					Checked:         func() ui.CheckedState { return terms() },
					OnCheckedChange: change(setTerms),
				}, "I accept the terms")
				ui.Show(
					func() bool { return errors() && !terms() },
					func() {
						ui.Text(
							"The terms must be accepted",
						).FontSize(12).TextColor(color(func(p palette) string { return p.Danger }))
					},
				)
				row(func() {
					primary("Submit", submit)
					button("Reset", func() {
						ui.Batch(func() {
							setEmail("")
							setPlan("pro")
							setTerms(false)
							setErrors(false)
							setAttempts(0)
							setResult("nothing submitted yet")
						})
					})
				})
			},
		)
		note(func() string { return "attempts " + strconv.Itoa(attempts()) + " · " + result() })
	})
}
func InputDemo() {
	text, setText := ui.CreateSignal("")
	secret, setSecret := ui.CreateSignal("")
	notes, setNotes := ui.CreateSignal("Two\nlines")
	submits, setSubmits := ui.CreateSignal(0)
	panel("Input", "Controlled native editors: single-line, password, and multiline. Return in the first field reports submission.", func() {
		input(text, setText, "Type and press Return", ui.Style().Width(300), ui.OnSubmit(func(*native.Event) { setSubmits(submits() + 1) }))
		input(secret, setSecret, "Password", ui.Style().Width(300), ui.Password(true))
		input(notes, setNotes, "Notes", ui.Style().Width(420).Height(96), ui.Multiline(true))
		note(func() string {
			return "text " + strconv.Quote(text()) + " · password length " + strconv.Itoa(len(secret())) + " · notes " + strconv.Quote(notes()) + " · submits " + strconv.Itoa(submits())
		})
	})
}
func numberControls() {
	ui.NumberField.Group(
		rowPart(),
		func() {
			ui.NumberField.Decrement(control(), "−")
			s := inputStyle()
			s = s.Width(110)
			ui.NumberField.Input(ui.NumberFieldInputProps{InputPartProps: ui.InputPartProps{PartProps: ui.PartProps{Style: s}}})
			ui.NumberField.Increment(control(), "+")
		},
	)
}
func NumberFieldDemo() {
	quantity, setQuantity := ui.CreateSignal(ptr(8.0))
	valid, setValid := ui.CreateSignal(true)
	committed, setCommitted := ui.CreateSignal[*float64](nil)
	panel("Number field", "Parsing, clamping, step snapping, and scrub gestures. Alt uses the small step; Shift uses the large step; Return commits.", func() {
		ui.NumberField.Root(
			ui.NumberFieldRootProps{
				Value:            quantity,
				Min:              0,
				Max:              100,
				Step:             1,
				SmallStep:        0.1,
				LargeStep:        10,
				Precision:        1,
				AllowWheelScrub:  ptr(true),
				Required:         true,
				OnValueChange:    func(v *float64, ok bool, _ *native.Event) { setQuantity(v); setValid(ok) },
				OnValueCommitted: change(setCommitted),
				PartProps:        columnPart(),
			},
			func() {
				state := ui.UseNumberFieldState()
				ui.NumberField.ScrubArea(
					ui.PartProps{Style: ui.Style().Cursor("ew-resize").Height(24)},
					"Drag here to change quantity",
				)
				numberControls()
				ui.NumberField.ScrubAreaCursor(
					ui.PartProps{},
					func() {
						ui.Show(
							func() bool { return state().Scrubbing },
							func() { label("Scrubbing") },
						)
					},
				)
				note(func() string {
					return "scrubbing " + strconv.FormatBool(state().Scrubbing) + " · required " + strconv.FormatBool(state().Required)
				})
			},
		)
		ui.NumberField.Root(
			ui.NumberFieldRootProps{
				DefaultValue: ptr(42.0),
				ReadOnly:     true,
				PartProps:    columnPart(),
			},
			func() { label("Read-only"); numberControls() },
		)
		note(func() string {
			return "quantity " + textValue(quantity()) + " · valid " + strconv.FormatBool(valid()) + " · committed " + textValue(committed())
		})
	})
}
func OtpFieldDemo() {
	code, setCode := ui.CreateSignal("")
	completed, setCompleted := ui.CreateSignal("not yet")
	panel("OTP field", "Six slots: typing advances, paste distributes, and Backspace walks back. Completion is reported once per completed value.", func() {
		ui.OtpField.Root(
			ui.OtpFieldRootProps{
				Value:          code,
				Length:         6,
				ValidationType: "numeric",
				OnValueChange:  change(setCode),
				OnComplete:     change(setCompleted),
				PartProps:      rowPart(),
			},
			func() {
				for i := range 6 {
					if i == 3 {
						ui.OtpField.Separator(ui.OtpFieldSeparatorProps{Index: ptr(i)}, "−")
					}
					s := inputStyle()
					s = s.Width(36)
					s = s.Height(40)
					s = s.TextAlign("center")
					s = s.FontSize(18)
					ui.OtpField.Input(ui.OtpFieldInputProps{
						Index:          i,
						InputPartProps: ui.InputPartProps{PartProps: ui.PartProps{Style: s}},
					})
				}
			},
		)
		row(func() { button("Fill 123456", func() { setCode("123456") }); button("Clear", func() { setCode("") }) })
		note(func() string { return "code " + code() + " · completed " + completed() })
	})
}
func segment(width int) ui.PartProps {
	s := inputStyle()
	s = s.Width(width)
	s = s.TextAlign("center")
	s = s.PaddingLeft(4)
	s = s.PaddingRight(4)
	return ui.PartProps{Style: s}
}
func DateFieldDemo() {
	due, setDue := ui.CreateSignal(ptr("2026-09-03"))
	iso, setISO := ui.CreateSignal(ptr("2026-01-15"))
	panel("Date field", "Civil dates with no time zone. Up/Down steps a segment; typing advances to the next; min/max constrain the result.", func() {
		for _, field := range []struct {
			value     func() *string
			set       func(*string)
			format    string
			segments  []string
			separator string
		}{{due, setDue, "mdy", []string{"month", "day", "year"}, "/"}, {iso, setISO, "ymd", []string{"year", "month", "day"}, "-"}} {
			ui.DateField.Root(
				ui.DateFieldRootProps{
					Value:         field.value,
					OnValueChange: change(field.set),
					Min:           "2026-01-01",
					Max:           "2026-12-31",
					Format:        field.format,
					PartProps:     rowPart(),
				},
				func() {
					for i, name := range field.segments {
						if i > 0 {
							label(field.separator)
						}
						ui.DateField.Segment(ui.DateFieldSegmentProps{
							Segment:   name,
							PartProps: segment(choose(name == "year", 64, 40)),
						})
					}
				},
			)
		}
		note(func() string { return "US " + textValue(due()) + " · ISO " + textValue(iso()) })
	})
}
func TimeFieldDemo() {
	at, setAt := ui.CreateSignal(ptr("09:30"))
	precise, setPrecise := ui.CreateSignal(ptr("14:05:30"))
	panel("Time field", "12-hour and 24-hour civil times. A seconds segment exists only when ShowSeconds is enabled.", func() {
		ui.TimeField.Root(
			ui.TimeFieldRootProps{
				Value:         at,
				OnValueChange: change(setAt),
				Hour12:        true,
				PartProps:     rowPart(),
			},
			func() {
				ui.TimeField.Segment(ui.TimeFieldSegmentProps{
					Segment:   "hour",
					PartProps: segment(40),
				})
				label(":")
				ui.TimeField.Segment(ui.TimeFieldSegmentProps{
					Segment:   "minute",
					PartProps: segment(40),
				})
				ui.TimeField.Segment(ui.TimeFieldSegmentProps{
					Segment:   "period",
					PartProps: segment(48),
				})
			},
		)
		ui.TimeField.Root(
			ui.TimeFieldRootProps{
				Value:         precise,
				OnValueChange: change(setPrecise),
				ShowSeconds:   true,
				PartProps:     rowPart(),
			},
			func() {
				for i, name := range []string{"hour", "minute", "second"} {
					if i > 0 {
						label(":")
					}
					ui.TimeField.Segment(ui.TimeFieldSegmentProps{
						Segment:   name,
						PartProps: segment(40),
					})
				}
			},
		)
		note(func() string { return "12-hour " + textValue(at()) + " · precise " + textValue(precise()) })
	})
}
func monthGrid(month string) [][]string {
	first, err := time.Parse("2006-01", month)
	if err != nil {
		return nil
	}
	start := first.AddDate(0, 0, -int(first.Weekday()))
	weeks := make([][]string, 6)
	for week := range weeks {
		weeks[week] = make([]string, 7)
		for day := range weeks[week] {
			weeks[week][day] = start.AddDate(0, 0, week*7+day).Format("2006-01-02")
		}
	}
	return weeks
}
func CalendarDemo() {
	day, setDay := ui.CreateSignal(ptr("2026-09-03"))
	month, setMonth := ui.CreateSignal("2026-09")
	focused, setFocused := ui.CreateSignal("2026-09-03")
	panel("Calendar", "An application-declared grid with native keyboard navigation, one Tab stop, and reported month changes.", func() {
		row(func() {
			button("Previous month", func() { t, _ := time.Parse("2006-01", month()); setMonth(t.AddDate(0, -1, 0).Format("2006-01")) })
			label(month)
			button("Next month", func() { t, _ := time.Parse("2006-01", month()); setMonth(t.AddDate(0, 1, 0).Format("2006-01")) })
		})
		ui.Calendar.Root(
			ui.CalendarRootProps{
				Value:         day,
				OnValueChange: change(setDay),
				OnMonthChange: change(setMonth),
				OnFocusChange: change(setFocused),
				Min:           "2026-01-01",
				Max:           "2026-12-31",
				FirstWeekday:  0,
				PartProps:     columnPart(),
			},
			func() {
				ui.View(
					func() {
						for _, name := range []string{"Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"} {
							ui.Text(
								name,
							).Width(32).TextAlign("center").FontSize(10)
						}
					},
				).Display("flex").Gap(3)
				ui.For(
					func() [][]string { return monthGrid(month()) },
					func(week []string, index func() int) {
						ui.Calendar.Week(
							ui.CalendarWeekProps{
								Index: ptr(index()),
								PartProps: ui.PartProps{Style: ui.Style().
									Display("flex").
									Gap(3)},
							},
							func() {
								for _, date := range week {
									selected := func() bool { return day() != nil && *day() == date }
									ui.Calendar.Day(
										ui.CalendarDayProps{
											Day: date,
											PartProps: ui.PartProps{Style: func() ui.StyleBuilder {
												return ui.Style().
													Display("flex").
													AlignItems("center").
													JustifyContent("center").
													Width(32).
													Height(28).
													BorderRadius(7).
													BackgroundColor(choose(selected(), p().Accent, p().PanelAlt)).
													TextColor(choose(selected(), p().OnAccent, choose(strings.HasPrefix(date, month()), p().Ink, p().Faint))).
													FontSize(11).
													Hover(func(s ui.StyleBuilder) ui.StyleBuilder {
														return s.BackgroundColor(choose(selected(), p().Accent, p().ControlHover))
													}).
													FocusStyle(func(s ui.StyleBuilder) ui.StyleBuilder {
														return s.Outline("2px solid " + p().Accent)
													})
											}},
										},
										date[8:],
									)
								}
							},
						)
					},
					nil,
					nil,
				)
			},
		)
		note(func() string {
			return "selected " + textValue(day()) + " · showing " + month() + " · tab stop " + focused()
		})
	})
}
