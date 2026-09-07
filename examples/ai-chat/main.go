package main

import (
	"context"
	"encoding/base64"
	"encoding/json"
	"fmt"
	"log"
	"os"
	"path/filepath"
	"strings"
	"sync"
	"time"

	"github.com/egoist/quickgui/go/native"
	"github.com/egoist/quickgui/go/protocol"
	"github.com/egoist/quickgui/go/reactive"
	"github.com/egoist/quickgui/go/ui"
)

type Conversation struct {
	ID       string    `json:"id"`
	Title    string    `json:"title"`
	Messages []Message `json:"messages"`
	Draft    string    `json:"draft"`
}

type chatState struct {
	Conversations []Conversation `json:"conversations"`
	Active        string         `json:"active"`
}

var persistence sync.Mutex

func main() {
	if err := native.Run(func() {
		native.NewWindow(native.WindowOptions{
			Title:         "QuickGUI AI Chat",
			Width:         1120,
			Height:        780,
			MinimumWidth:  800,
			MinimumHeight: 560,
			TitleBarStyle: "hiddenInset",
			Background:    "#0b1020",
			Component:     chat,
		})
	}); err != nil {
		log.Fatal(err)
	}
}

func chat() {
	window := native.CurrentWindow()
	var generation, revision, savedRevision uint64
	state := reactive.NewSignal(chatState{})
	draft := reactive.NewSignal("")
	key := reactive.NewSignal(strings.TrimSpace(os.Getenv("DEEPSEEK_API_KEY")))
	busy := reactive.NewSignal(false)
	status := reactive.NewSignal("")
	var cancel context.CancelFunc
	config, _ := os.UserConfigDir()
	file := filepath.Join(config, "quickgui-ai-chat-go", "history.json")

	current := func() Conversation {
		value := state.Read()
		for _, c := range value.Conversations {
			if c.ID == value.Active {
				return c
			}
		}
		return Conversation{}
	}
	change := func(fn func(*chatState)) {
		value := state.Peek()
		value.Conversations = append([]Conversation(nil), value.Conversations...)
		fn(&value)
		state.Write(value)
	}
	persist := func() {
		revision++
		version := revision
		body, _ := json.Marshal(state.Peek())
		ui.Async(
			func(context.Context) (bool, error) {
				persistence.Lock()
				defer persistence.Unlock()
				if version <= savedRevision {
					return false, nil
				}
				if err := os.MkdirAll(filepath.Dir(file), 0700); err != nil {
					return false, err
				}
				if err := os.WriteFile(file+".tmp", body, 0600); err != nil {
					return false, err
				}
				if err := os.Rename(file+".tmp", file); err != nil {
					return false, err
				}
				savedRevision = version
				return true, nil
			},
			func(_ bool, err error) {
				if err != nil {
					status.Write(err.Error())
				}
			},
		)
	}
	newChat := func() {
		if busy.Peek() {
			return
		}
		id := fmt.Sprint(time.Now().UnixNano())
		change(func(value *chatState) {
			value.Active = id
			value.Conversations = append([]Conversation{{ID: id, Title: "New chat"}}, value.Conversations...)
			if len(value.Conversations) > 100 {
				value.Conversations = value.Conversations[:100]
			}
		})
		draft.Write("")
		persist()
	}
	ui.Async(
		func(context.Context) (chatState, error) {
			var saved chatState
			body, err := os.ReadFile(file)
			if err == nil && len(body) <= 32*1024*1024 {
				err = json.Unmarshal(body, &saved)
			}
			return saved, err
		},
		func(saved chatState, err error) {
			if state.Peek().Active != "" {
				return
			}
			if err == nil && len(saved.Conversations) > 0 {
				if len(saved.Conversations) > 100 {
					saved.Conversations = saved.Conversations[:100]
				}
				state.Write(saved)
				draft.Write(current().Draft)
			} else {
				newChat()
			}
		},
	)

	credentials := map[string]any{"service": "dev.quickgui.ai-chat-example", "account": "DeepSeek API key"}
	if key.Peek() == "" {
		native.Invoke(
			"get-secure-storage",
			credentials,
			func(raw string, err error) {
				if window.Closed {
					return
				}
				var encoded string
				if err == nil && key.Peek() == "" && json.Unmarshal([]byte(raw), &encoded) == nil {
					if value, err := base64.StdEncoding.DecodeString(encoded); err == nil {
						key.Write(string(value))
					}
				}
			},
		)
	}
	updateResponse := func(id, response string) {
		change(func(value *chatState) {
			for i, c := range value.Conversations {
				if c.ID != id || len(c.Messages) == 0 {
					continue
				}
				c.Messages = append([]Message(nil), c.Messages...)
				c.Messages[len(c.Messages)-1].Content = response
				value.Conversations[i] = c
			}
		})
	}
	send := func() {
		prompt, secret := strings.TrimSpace(draft.Peek()), strings.TrimSpace(key.Peek())
		if busy.Peek() || prompt == "" {
			return
		}
		if secret == "" {
			status.Write("Enter a DeepSeek API key in Provider settings.")
			return
		}
		c := current()
		if c.ID == "" {
			return
		}
		messages := append(append([]Message(nil), c.Messages...), Message{Role: "user", Content: prompt})
		if len(messages) > 100 {
			messages = messages[len(messages)-100:]
		}
		length := 0
		for i := len(messages) - 1; i >= 0; i-- {
			length += len(messages[i].Content)
			if length > 96000 {
				messages = messages[i+1:]
				break
			}
		}
		if len(messages) == 0 {
			status.Write("Message exceeds the 96,000-byte context limit.")
			return
		}
		change(func(value *chatState) {
			for i := range value.Conversations {
				if value.Conversations[i].ID == c.ID {
					value.Conversations[i].Messages = append(append([]Message(nil), messages...), Message{Role: "assistant"})
					value.Conversations[i].Title = string([]rune(prompt)[:min(40, len([]rune(prompt)))])
					value.Conversations[i].Draft = ""
				}
			}
		})
		draft.Write("")
		busy.Write(true)
		status.Write("Streaming…")
		generation++
		requestGeneration := generation
		cancel = ui.Async(
			func(ctx context.Context) (string, error) {
				return complete(ctx, secret, messages, func(text string) error {
					native.Dispatch(func() {
						if !window.Closed && generation == requestGeneration {
							updateResponse(c.ID, text)
						}
					})
					return nil
				})
			},
			func(text string, err error) {
				if generation != requestGeneration {
					return
				}
				updateResponse(c.ID, text)
				busy.Write(false)
				status.Write("")
				if err != nil {
					status.Write(err.Error())
				}
				persist()
			},
		)
	}
	ui.CreateEffect(func() {
		text := draft.Read()
		change(func(value *chatState) {
			for i := range value.Conversations {
				if value.Conversations[i].ID == value.Active {
					value.Conversations[i].Draft = text
				}
			}
		})
	})

	var provider *native.Node
	action("Provider settings", func() {
		native.NewWindow(native.WindowOptions{
			Anchor:     provider,
			Width:      360,
			Height:     260,
			Background: "#182338",
			Component: func() {
				popover := native.CurrentWindow()
				reveal, setReveal := ui.CreateSignal(false)
				ui.View(
					ui.Display("flex"),
					ui.FlexDirection("column"),
					ui.Width("100%"),
					ui.Height("100%"),
					ui.Gap(12),
					ui.Padding(20),
					ui.Color("#e2e8f0"),
					func() {
						ui.Text(ui.FontSize(20), "DeepSeek API key")
						ui.Input(
							ui.Value(key.Read),
							ui.Password(func() bool { return !reveal() }),
							ui.Padding(12),
							ui.OnInput(func(e *native.Event) { key.Write(e.Value) }),
						)
						action("Reveal / hide", func() { setReveal(!reveal()) })
						action("Save to Keychain", func() {
							secret := strings.TrimSpace(key.Peek())
							if secret == "" || strings.ContainsAny(secret, "\r\n") {
								return
							}
							params := map[string]any{"service": credentials["service"], "account": credentials["account"], "value": base64.StdEncoding.EncodeToString([]byte(secret))}
							native.Invoke(
								"set-secure-storage",
								params,
								func(_ string, err error) {
									if window.Closed {
										return
									}
									if err != nil {
										status.Write(err.Error())
									} else {
										status.Write("API key saved to Keychain.")
										popover.Close()
									}
								},
							)
						})
					},
				)
			},
		})
	}, ui.Ref(func(ref *native.Node) {
		provider = ref
	}),
	)
	sidebar := ui.View(
		ui.Display("flex"),
		ui.FlexDirection("column"),
		ui.Gap(14),
		ui.Padding(20),
		ui.PaddingTop(52),
		ui.Width(260),
		ui.Height("100%"),
		ui.BackgroundColor("#121b2b"),
		func() {
			ui.Text(ui.FontSize(20), "Conversations")
			action("New chat", newChat)
			ui.Child(provider)
			ui.KeyedFor(
				func() []Conversation { return state.Read().Conversations },
				func(c Conversation) any { return c.ID },
				func(c func() Conversation, _ func() int) {
					node := ui.Button(
						ui.Padding(12),
						ui.BorderRadius(8),
						ui.OnClick(func() {
							if busy.Peek() {
								return
							}
							change(func(value *chatState) { value.Active = c().ID })
							draft.Write(c().Draft)
							persist()
						}),
						func() string { return c().Title },
					)
					node.Bind(func() {
						color := uint32(0)
						if state.Read().Active == c().ID {
							color = native.ParseColor("#263854")
						}
						native.SetColor(node, protocol.BackgroundColor, color)
					})
					ui.Child(node)
				},
				nil,
			)
		},
	)
	transcript := ui.Markdown(ui.Value(func() string {
		var text strings.Builder
		for _, message := range current().Messages {
			text.WriteString("### " + message.Role + "\n\n" + message.Content + "\n\n")
		}
		return text.String()
	}))
	ui.View(
		ui.Display("flex"),
		ui.Width("100%"),
		ui.Height("100%"),
		ui.Color("#e2e8f0"),
		func() {
			ui.Child(sidebar)
			ui.View(
				ui.Display("flex"),
				ui.FlexDirection("column"),
				ui.Flex(1),
				ui.Gap(14),
				ui.Padding(28),
				ui.PaddingTop(52),
				func() {
					ui.View(ui.Flex(1), ui.MinHeight(0), ui.OverflowY("scroll"), transcript)
					ui.Text(ui.Color("#94a3b8"), status.Read)
					ui.Input(
						ui.Value(draft.Read),
						ui.Placeholder("Message DeepSeek…"),
						ui.Padding(14),
						ui.OnInput(func(e *native.Event) { draft.Write(e.Value) }),
						ui.OnSubmit(func(*native.Event) { send() }),
					)
					ui.View(
						ui.Display("flex"),
						ui.Gap(10),
						func() {
							action("Send", send)
							action("Stop", func() {
								generation++
								if cancel != nil {
									cancel()
									cancel = nil
								}
								busy.Write(false)
								status.Write("Stopped")
								persist()
							})
						},
					)
				},
			)
		},
	)
}

func action(label string, click func(), options ...any) {
	args := []any{ui.Display("flex"),
		ui.Padding(10),
		ui.BorderRadius(8),
		ui.BackgroundColor("#253855"),
		ui.UserSelect("none"),
		ui.AppRegion("no-drag"),
		ui.OnClick(func() { click() }),
		label}
	args = append(args, options...)

	ui.Button(args...)

}
