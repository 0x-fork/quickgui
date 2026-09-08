package main

import (
	"log"
	"os"
	"path/filepath"
	"strings"
	"time"

	"github.com/egoist/quickgui/go/native"
	"github.com/egoist/quickgui/go/ui"
)

func main() {
	path := historyPath()
	config, _ := os.UserConfigDir()
	history, loadErr := loadHistory(path, filepath.Join(config, "quickgui-ai-chat-go", "history.json"))
	var writer *historyWriter
	if loadErr == nil {
		writer = &historyWriter{path: path}
	}
	controller := newChatController(history, nil)
	if loadErr != nil {
		// Keep an unreadable or malformed history intact instead of overwriting it.
		controller.status.Write("History could not be loaded; saving is paused: " + loadErr.Error())
	}
	if writer != nil {
		controller.writer = writer
		writer.onError = func(err error) {
			native.Dispatch(func() { controller.status.Write("Unable to save history: " + err.Error()) })
		}
	}
	if err := native.Run(func() {
		controller.persist()
		open := func() {
			native.NewWindow(native.WindowOptions{
				Title:                "QuickGUI AI Chat",
				Width:                1080,
				Height:               720,
				MinimumWidth:         760,
				MinimumHeight:        480,
				TitleBarStyle:        "hiddenInset",
				TrafficLightPosition: &native.Point{X: 16, Y: 15},
				Background:           "#0b1020",
				Component:            controller.view,
			})
		}
		native.App.OnReopen(func(event native.ReopenEvent) {
			if !event.HasVisibleWindows {
				open()
			}
		})
		native.App.OnBeforeQuit(func(native.QuitPhaseEvent) {
			controller.shutdown(func(err error) {
				if err != nil {
					controller.status.Write("Unable to save history: " + err.Error())
					return
				}
				native.App.Quit(true, nil)
			})
		})
		controller.loadCredential()
		open()
	}); err != nil {
		log.Fatal(err)
	}
}

func buttonStyle() ui.StyleBuilder {
	return ui.Style().
		Display("flex").
		AlignItems("center").
		JustifyContent("center").
		Height(34).
		FlexShrink(0).
		PaddingLeft(12).
		PaddingRight(12).
		BorderRadius(7).
		BackgroundColor("#253855").
		TextColor("#e2e8f0").
		UserSelect("none").
		AppRegion("no-drag").
		Cursor("default").
		FontSize(13).
		Hover(func(s ui.StyleBuilder) ui.StyleBuilder { return s.BackgroundColor("#304869") }).
		DisabledStyle(func(s ui.StyleBuilder) ui.StyleBuilder { return s.Opacity(0.45) })
}

func inputStyle() ui.StyleBuilder {
	return ui.Style().
		Height(38).
		Width("100%").
		PaddingLeft(12).
		PaddingRight(12).
		BackgroundColor("#0b1020").
		TextColor("#e2e8f0").
		BorderRadius(7).
		BorderWidth(1).
		BorderColor("#334155").
		FontSize(14).
		AppRegion("no-drag")
}

func (controller *chatController) view() {
	ui.View(
		func() {
			controller.sidebar()
			ui.View(
				func() {
					controller.toolbar()
					ui.VirtualList(
						func() {
							ui.KeyedFor(
								func() []ChatMessage { return controller.current().Messages },
								func(message ChatMessage) any { return message.ID },
								func(message func() ChatMessage, _ func() int) {
									messageCard(message)
								},
								nil,
							)
						},
						ui.Style().Flex(1),
						ui.Style().MinHeight(0),
						ui.Style().Width("100%"),
						ui.Style().Padding(20),
						ui.Style().Gap(14),
						ui.Style().AlignItems("center"),
						ui.EstimatedItemHeight(240),
						ui.Overscan(1),
						ui.ListAlignment("top"),
						ui.FollowMode("tail"),
					)
					controller.composer()
				},
			).Display("flex").FlexDirection("column").Flex(1).MinWidth(0).Height("100%")
		},
	).Display("flex").Width("100%").Height("100%").TextColor("#e2e8f0").FontSize(14)
}

func (controller *chatController) sidebar() {
	ui.View(
		func() {
			ui.Text("Conversations").FontSize(18).FontWeight(700)
			ui.Button(
				"New chat",
				buttonStyle(),
				ui.Disabled(controller.busy.Read),
				ui.OnClick(controller.newConversation),
			)
			ui.View(
				func() {
					ui.KeyedFor(
						func() []Conversation { return controller.state.Read().Conversations },
						func(conversation Conversation) any { return conversation.ID },
						func(conversation func() Conversation, _ func() int) {
							ui.Button(
								func() {
									ui.Text(
										func() string { return conversation().Title },
									).Width("100%").FontWeight(600).WhiteSpace("nowrap").Overflow("hidden").TextOverflow("ellipsis")
									ui.Text(
										func() string {
											return time.UnixMilli(conversation().UpdatedAt).Format("Jan 2, 15:04")
										},
									).FontSize(11).TextColor("#94a3b8")
								},
								ui.Style().Display("flex"),
								ui.Style().FlexDirection("column"),
								ui.Style().Gap(5),
								ui.Style().Padding(12),
								ui.Style().Width("100%"),
								ui.Style().FlexShrink(0),
								ui.Style().BorderRadius(8),
								ui.Style().AppRegion("no-drag"),
								ui.Style().Cursor("default"),
								ui.Style().UserSelect("none"),
								ui.Style().Hover(func(s ui.StyleBuilder) ui.StyleBuilder {
									return s.BackgroundColor("#1c2b42")
								}),
								ui.When(
									func() bool {
										return controller.state.Read().ActiveConversationID == conversation().ID
									},
									ui.Style().BackgroundColor("#263854"),
								),
								ui.Disabled(controller.busy.Read),
								ui.OnClick(func() { controller.selectConversation(conversation().ID) }),
							)
						},
						nil,
					)
				},
			).Display("flex").FlexDirection("column").Flex(1).MinHeight(0).OverflowY("scroll").Gap(6)
			ui.Text(
				"Conversations and drafts are saved locally.",
			).FontSize(11).TextColor("#94a3b8").LineHeight(16)
		},
	).Display("flex").FlexDirection("column").Width(248).Height("100%").FlexShrink(0).Padding(16).PaddingTop(52).Gap(14).BackgroundColor("#121b2b")
}

func (controller *chatController) toolbar() {
	ui.View(
		func() {
			ui.View(
				func() {
					ui.Text(
						func() string { return controller.current().Title },
					).FontSize(18).FontWeight(700).WhiteSpace("nowrap").TextOverflow("ellipsis").Overflow("hidden")
					ui.Text(
						"DeepSeek · native streaming Markdown",
					).FontSize(12).TextColor("#94a3b8")
				},
			).Display("flex").FlexDirection("column").Flex(1).MinWidth(0).Gap(4)
			ui.SystemPopover.Root(
				ui.PopoverRootProps{
					Open: controller.settingsOpen.Read,
					OnOpenChange: func(open bool, _ ui.PopoverOpenChangeDetails) {
						if !controller.credentialBusy.Peek() {
							controller.settingsOpen.Write(open)
						}
					},
				},
				func() {
					ui.SystemPopover.Trigger(
						ui.PopoverTriggerProps{PartProps: ui.PartProps{Style: buttonStyle()}},
						func() string {
							if controller.key.Read() == "" {
								return "Set API key"
							}
							return "Provider settings"
						},
					)
					ui.SystemPopover.Content(
						ui.PopoverContentProps{
							Width:          460,
							Height:         310,
							Placement:      "bottom-end",
							Gap:            ptr(8.0),
							ViewportMargin: ptr(12.0),
						},
						controller.providerSettings,
					)
				},
			)
		},
	).Display("flex").AlignItems("center").Height(72).FlexShrink(0).Padding(20).Gap(16).AppRegion("drag").BorderBottomWidth(1).BorderColor("#243247")
}

func messageCard(message func() ChatMessage) {
	ui.View(
		func() {
			ui.Text(
				func() string {
					if message().Role == "user" {
						return "You"
					}
					return "DeepSeek"
				},
			).FontSize(12).FontWeight(700).TextColor("#93c5fd")
			ui.Markdown(
				ui.Style().Width("100%"),
				ui.Style().FontSize(14),
				ui.Style().LineHeight(22),
				ui.Style().TextColor("#e2e8f0"),
				ui.Style().MarkdownLinkColor("#93c5fd"),
				ui.Style().MarkdownCodeTextColor("#c4b5fd"),
				ui.Style().MarkdownCodeBackground("#0b1020"),
				ui.Style().MarkdownBorderColor("#475569"),
				ui.Style().MarkdownMutedColor("#94a3b8"),
				ui.Streaming(func() bool { return message().Streaming }),
				ui.Value(func() string {
					if message().Streaming && message().Content == "" {
						return "_Thinking…_"
					}
					return message().Content
				}),
			)
			ui.Show(
				func() bool { return message().Failed },
				func() {
					ui.Text(
						"Response interrupted. You can send another message to continue.",
					).FontSize(12).TextColor("#fca5a5")
				},
			)
		},
	).Display("flex").FlexDirection("column").Width("100%").MaxWidth(760).Gap(10).Padding(16).BorderRadius(10).BackgroundColor("#151f31")
}

func (controller *chatController) composer() {
	ui.View(
		func() {
			ui.Show(
				func() bool { return controller.status.Read() != "" },
				func() {
					ui.Text(
						controller.status.Read,
					).FontSize(12).TextColor("#94a3b8")
				},
			)
			ui.View(
				func() {
					ui.Input(
						inputStyle(),
						ui.Style().Flex(1),
						ui.Style().MinWidth(0),
						ui.Value(func() string { return controller.current().Draft }),
						ui.Placeholder("Message DeepSeek…"),
						ui.Disabled(controller.busy.Read),
						ui.OnInput(func(event *native.Event) { controller.setDraft(event.Value) }),
						ui.OnSubmit(func(event *native.Event) {
							controller.setDraft(event.Value)
							controller.send()
						}),
					)
					ui.Button(
						func() string {
							if controller.busy.Read() {
								return "Stop"
							}
							return "Send"
						},
						buttonStyle(),
						ui.Style().BackgroundColor("#2563eb"),
						ui.Style().Hover(func(s ui.StyleBuilder) ui.StyleBuilder {
							return s.BackgroundColor("#3b82f6")
						}),
						ui.Disabled(func() bool {
							return !controller.busy.Read() && strings.TrimSpace(controller.current().Draft) == ""
						}),
						ui.OnClick(func() {
							if controller.busy.Peek() {
								controller.stop()
							} else {
								controller.send()
							}
						}),
					)
				},
			).Display("flex").Width("100%").Gap(10).AlignItems("center")
			ui.Text(
				func() string {
					if controller.key.Read() == "" {
						return "Drafts are stored locally · sending opens Provider settings"
					}
					return "Enter to send · responses stream into individual message cards"
				},
			).FontSize(11).TextColor("#94a3b8")
		},
	).Display("flex").FlexDirection("column").FlexShrink(0).Padding(20).Gap(10).BorderTopWidth(1).BorderColor("#243247")
}

func (controller *chatController) providerSettings() {
	value, setValue := ui.CreateSignal(controller.key.Peek())
	reveal, setReveal := ui.CreateSignal(false)
	errorText, setError := ui.CreateSignal("")
	window := native.CurrentWindow()
	close := func() { controller.settingsOpen.Write(false) }
	ui.View(
		func() {
			ui.Text("DeepSeek API key").FontSize(19).FontWeight(700)
			ui.Text(
				"Stored in your operating system’s credential store. Your key is never written to conversation history.",
			).FontSize(12).LineHeight(18).TextColor("#94a3b8")
			ui.View(
				func() {
					ui.Input(
						inputStyle(),
						ui.Style().Flex(1),
						ui.Style().MinWidth(0),
						ui.Value(value),
						ui.Password(func() bool { return !reveal() }),
						ui.Disabled(controller.credentialBusy.Read),
						ui.OnInput(func(event *native.Event) { setValue(event.Value) }),
					)
					ui.Button(
						func() string {
							if reveal() {
								return "Hide"
							}
							return "Reveal"
						},
						buttonStyle(),
						ui.OnClick(func() { setReveal(!reveal()) }),
					)
				},
			).Display("flex").Gap(8).AlignItems("center")
			ui.Text(
				errorText,
			).FontSize(12).TextColor("#fca5a5").MinHeight(18)
			ui.View().Flex(1)
			ui.View(
				func() {
					ui.Button(
						"Remove key",
						buttonStyle(),
						ui.Style().TextColor("#fca5a5"),
						ui.Disabled(controller.credentialBusy.Read),
						ui.OnClick(func() {
							controller.credentialBusy.Write(true)
							native.SecureStorage.Delete(
								credentialService(),
								credentialAccount,
								func(_ bool, err error) {
									controller.credentialBusy.Write(false)
									if err != nil {
										if !window.Closed {
											setError(err.Error())
										}
										return
									}
									controller.key.Write("")
									controller.status.Write("API key removed.")
									close()
								},
							)
						}),
					)
					ui.View().Flex(1)
					ui.Button(
						"Cancel",
						buttonStyle(),
						ui.Disabled(controller.credentialBusy.Read),
						ui.OnClick(close),
					)
					ui.Button(
						"Save",
						buttonStyle(),
						ui.Style().BackgroundColor("#2563eb"),
						ui.Disabled(controller.credentialBusy.Read),
						ui.OnClick(func() {
							secret := strings.TrimSpace(value())
							if secret == "" || len(secret) > 2048 || strings.ContainsAny(secret, "\r\n") {
								setError("Enter a single-line API key of up to 2,048 characters.")
								return
							}
							controller.credentialBusy.Write(true)
							setError("")
							native.SecureStorage.SetText(
								credentialService(),
								credentialAccount,
								secret,
								func(saved bool, err error) {
									controller.credentialBusy.Write(false)
									if err != nil || !saved {
										if !window.Closed {
											if err != nil {
												setError(err.Error())
											} else {
												setError("The credential store did not save the API key.")
											}
										}
										return
									}
									controller.key.Write(secret)
									controller.status.Write("API key saved.")
									close()
								},
							)
						}),
					)
				},
			).Display("flex").Gap(8).AlignItems("center")
		},
	).Display("flex").FlexDirection("column").Width("100%").Height("100%").Padding(20).Gap(12).BackgroundColor("#182338").TextColor("#e2e8f0")
}

func ptr[T any](value T) *T { return &value }
