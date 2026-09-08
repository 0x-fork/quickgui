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

func buttonStyle() ui.Style {
	return ui.Styles(
		ui.Display("flex"),
		ui.AlignItems("center"),
		ui.JustifyContent("center"),
		ui.Height(34),
		ui.FlexShrink(0),
		ui.PaddingLeft(12),
		ui.PaddingRight(12),
		ui.BorderRadius(7),
		ui.BackgroundColor("#253855"),
		ui.TextColor("#e2e8f0"),
		ui.UserSelect("none"),
		ui.AppRegion("no-drag"),
		ui.Cursor("default"),
		ui.FontSize(13),
		ui.Hover(ui.BackgroundColor("#304869")),
		ui.DisabledStyle(ui.Opacity(0.45)),
	)
}

func inputStyle() ui.Style {
	return ui.Styles(
		ui.Height(38),
		ui.Width("100%"),
		ui.PaddingLeft(12),
		ui.PaddingRight(12),
		ui.BackgroundColor("#0b1020"),
		ui.TextColor("#e2e8f0"),
		ui.BorderRadius(7),
		ui.BorderWidth(1),
		ui.BorderColor("#334155"),
		ui.FontSize(14),
		ui.AppRegion("no-drag"),
	)
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
						ui.Flex(1),
						ui.MinHeight(0),
						ui.Width("100%"),
						ui.Padding(20),
						ui.Gap(14),
						ui.AlignItems("center"),
						ui.EstimatedItemHeight(240),
						ui.Overscan(1),
						ui.ListAlignment("top"),
						ui.FollowMode("tail"),
					)
					controller.composer()
				},
				ui.Display("flex"),
				ui.FlexDirection("column"),
				ui.Flex(1),
				ui.MinWidth(0),
				ui.Height("100%"),
			)
		},
		ui.Display("flex"),
		ui.Width("100%"),
		ui.Height("100%"),
		ui.TextColor("#e2e8f0"),
		ui.FontSize(14),
	)
}

func (controller *chatController) sidebar() {
	ui.View(
		func() {
			ui.Text("Conversations", ui.FontSize(18), ui.FontWeight(700))
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
										ui.Width("100%"),
										ui.FontWeight(600),
										ui.WhiteSpace("nowrap"),
										ui.Overflow("hidden"),
										ui.TextOverflow("ellipsis"),
									)
									ui.Text(
										func() string {
											return time.UnixMilli(conversation().UpdatedAt).Format("Jan 2, 15:04")
										},
										ui.FontSize(11),
										ui.TextColor("#94a3b8"),
									)
								},
								ui.Display("flex"),
								ui.FlexDirection("column"),
								ui.Gap(5),
								ui.Padding(12),
								ui.Width("100%"),
								ui.FlexShrink(0),
								ui.BorderRadius(8),
								ui.AppRegion("no-drag"),
								ui.Cursor("default"),
								ui.UserSelect("none"),
								ui.Hover(ui.BackgroundColor("#1c2b42")),
								ui.When(
									func() bool {
										return controller.state.Read().ActiveConversationID == conversation().ID
									},
									ui.BackgroundColor("#263854"),
								),
								ui.Disabled(controller.busy.Read),
								ui.OnClick(func() { controller.selectConversation(conversation().ID) }),
							)
						},
						nil,
					)
				},
				ui.Display("flex"),
				ui.FlexDirection("column"),
				ui.Flex(1),
				ui.MinHeight(0),
				ui.OverflowY("scroll"),
				ui.Gap(6),
			)
			ui.Text(
				"Conversations and drafts are saved locally.",
				ui.FontSize(11),
				ui.TextColor("#94a3b8"),
				ui.LineHeight(16),
			)
		},
		ui.Display("flex"),
		ui.FlexDirection("column"),
		ui.Width(248),
		ui.Height("100%"),
		ui.FlexShrink(0),
		ui.Padding(16),
		ui.PaddingTop(52),
		ui.Gap(14),
		ui.BackgroundColor("#121b2b"),
	)
}

func (controller *chatController) toolbar() {
	ui.View(
		func() {
			ui.View(
				func() {
					ui.Text(
						func() string { return controller.current().Title },
						ui.FontSize(18),
						ui.FontWeight(700),
						ui.WhiteSpace("nowrap"),
						ui.TextOverflow("ellipsis"),
						ui.Overflow("hidden"),
					)
					ui.Text(
						"DeepSeek · native streaming Markdown",
						ui.FontSize(12),
						ui.TextColor("#94a3b8"),
					)
				},
				ui.Display("flex"),
				ui.FlexDirection("column"),
				ui.Flex(1),
				ui.MinWidth(0),
				ui.Gap(4),
			)
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
		ui.Display("flex"),
		ui.AlignItems("center"),
		ui.Height(72),
		ui.FlexShrink(0),
		ui.Padding(20),
		ui.Gap(16),
		ui.AppRegion("drag"),
		ui.BorderBottomWidth(1),
		ui.BorderColor("#243247"),
	)
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
				ui.FontSize(12),
				ui.FontWeight(700),
				ui.TextColor("#93c5fd"),
			)
			ui.Markdown(
				ui.Width("100%"),
				ui.FontSize(14),
				ui.LineHeight(22),
				ui.TextColor("#e2e8f0"),
				ui.MarkdownLinkColor("#93c5fd"),
				ui.MarkdownCodeTextColor("#c4b5fd"),
				ui.MarkdownCodeBackground("#0b1020"),
				ui.MarkdownBorderColor("#475569"),
				ui.MarkdownMutedColor("#94a3b8"),
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
						ui.FontSize(12),
						ui.TextColor("#fca5a5"),
					)
				},
			)
		},
		ui.Display("flex"),
		ui.FlexDirection("column"),
		ui.Width("100%"),
		ui.MaxWidth(760),
		ui.Gap(10),
		ui.Padding(16),
		ui.BorderRadius(10),
		ui.BackgroundColor("#151f31"),
	)
}

func (controller *chatController) composer() {
	ui.View(
		func() {
			ui.Show(
				func() bool { return controller.status.Read() != "" },
				func() {
					ui.Text(
						controller.status.Read,
						ui.FontSize(12),
						ui.TextColor("#94a3b8"),
					)
				},
			)
			ui.View(
				func() {
					ui.Input(
						inputStyle(),
						ui.Flex(1),
						ui.MinWidth(0),
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
						ui.BackgroundColor("#2563eb"),
						ui.Hover(ui.BackgroundColor("#3b82f6")),
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
				ui.Display("flex"),
				ui.Width("100%"),
				ui.Gap(10),
				ui.AlignItems("center"),
			)
			ui.Text(
				func() string {
					if controller.key.Read() == "" {
						return "Drafts are stored locally · sending opens Provider settings"
					}
					return "Enter to send · responses stream into individual message cards"
				},
				ui.FontSize(11),
				ui.TextColor("#94a3b8"),
			)
		},
		ui.Display("flex"),
		ui.FlexDirection("column"),
		ui.FlexShrink(0),
		ui.Padding(20),
		ui.Gap(10),
		ui.BorderTopWidth(1),
		ui.BorderColor("#243247"),
	)
}

func (controller *chatController) providerSettings() {
	value, setValue := ui.CreateSignal(controller.key.Peek())
	reveal, setReveal := ui.CreateSignal(false)
	errorText, setError := ui.CreateSignal("")
	window := native.CurrentWindow()
	close := func() { controller.settingsOpen.Write(false) }
	ui.View(
		func() {
			ui.Text("DeepSeek API key", ui.FontSize(19), ui.FontWeight(700))
			ui.Text(
				"Stored in your operating system’s credential store. Your key is never written to conversation history.",
				ui.FontSize(12),
				ui.LineHeight(18),
				ui.TextColor("#94a3b8"),
			)
			ui.View(
				func() {
					ui.Input(
						inputStyle(),
						ui.Flex(1),
						ui.MinWidth(0),
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
				ui.Display("flex"),
				ui.Gap(8),
				ui.AlignItems("center"),
			)
			ui.Text(errorText, ui.FontSize(12), ui.TextColor("#fca5a5"), ui.MinHeight(18))
			ui.View(ui.Flex(1))
			ui.View(
				func() {
					ui.Button(
						"Remove key",
						buttonStyle(),
						ui.TextColor("#fca5a5"),
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
					ui.View(ui.Flex(1))
					ui.Button(
						"Cancel",
						buttonStyle(),
						ui.Disabled(controller.credentialBusy.Read),
						ui.OnClick(close),
					)
					ui.Button(
						"Save",
						buttonStyle(),
						ui.BackgroundColor("#2563eb"),
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
				ui.Display("flex"),
				ui.Gap(8),
				ui.AlignItems("center"),
			)
		},
		ui.Display("flex"),
		ui.FlexDirection("column"),
		ui.Width("100%"),
		ui.Height("100%"),
		ui.Padding(20),
		ui.Gap(12),
		ui.BackgroundColor("#182338"),
		ui.TextColor("#e2e8f0"),
	)
}

func ptr[T any](value T) *T { return &value }
