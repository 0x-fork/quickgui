package main

import (
	"context"
	"errors"
	"os"
	"strings"
	"time"

	"github.com/egoist/quickgui/go/native"
	"github.com/egoist/quickgui/go/reactive"
)

type completionFunc func(context.Context, string, []Message, func(string) error) (string, error)

// Conversation snapshots are immutable after publication. The UI and background
// history writer can retain them without copying message content on each delta.
type chatController struct {
	state                         *reactive.Signal[ConversationHistory]
	busy                          *reactive.Signal[bool]
	key                           *reactive.Signal[string]
	status                        *reactive.Signal[string]
	settingsOpen                  *reactive.Signal[bool]
	credentialBusy                *reactive.Signal[bool]
	writer                        *historyWriter
	nextConversation, nextMessage int64
	cancel                        context.CancelFunc
	generation                    uint64
	closing                       bool
	finished                      []func()
	complete                      completionFunc
	dispatch                      func(func())
}

func newChatController(history ConversationHistory, writer *historyWriter) *chatController {
	controller := &chatController{
		busy: reactive.NewSignal(false), key: reactive.NewSignal(strings.TrimSpace(os.Getenv("DEEPSEEK_API_KEY"))),
		status: reactive.NewSignal(""), settingsOpen: reactive.NewSignal(false), credentialBusy: reactive.NewSignal(false),
		writer: writer, nextConversation: 1, nextMessage: 1, complete: complete, dispatch: native.Dispatch,
	}
	for _, conversation := range history.Conversations {
		controller.nextConversation = max(controller.nextConversation, conversation.ID+1)
		for _, message := range conversation.Messages {
			controller.nextMessage = max(controller.nextMessage, message.ID+1)
		}
	}
	if len(history.Conversations) == 0 {
		conversation := createConversation(controller.nextConversation, controller.nextMessage, time.Now().UnixMilli())
		controller.nextConversation++
		controller.nextMessage++
		history = ConversationHistory{Version: 1, ActiveConversationID: conversation.ID, Conversations: []Conversation{conversation}}
	}
	history.Version = 1
	controller.state = reactive.NewSignal(history)
	controller.persist()
	return controller
}

func (controller *chatController) current() Conversation {
	history := controller.state.Read()
	for _, conversation := range history.Conversations {
		if conversation.ID == history.ActiveConversationID {
			return conversation
		}
	}
	return Conversation{}
}

func (controller *chatController) persist() {
	if controller.writer != nil {
		controller.writer.Schedule(controller.state.Peek())
	}
}

func (controller *chatController) edit(id int64, update func(*Conversation)) {
	history := controller.state.Peek()
	history.Conversations = append([]Conversation(nil), history.Conversations...)
	for i := range history.Conversations {
		conversation := &history.Conversations[i]
		if conversation.ID != id {
			continue
		}
		conversation.Messages = append([]ChatMessage(nil), conversation.Messages...)
		update(conversation)
		controller.state.Write(history)
		controller.persist()
		return
	}
}

func (controller *chatController) setDraft(value string) {
	if controller.closing {
		return
	}
	controller.edit(controller.state.Peek().ActiveConversationID, func(conversation *Conversation) { conversation.Draft = truncateText(value, maxPromptCharacters) })
}

func (controller *chatController) selectConversation(id int64) {
	if controller.busy.Peek() || controller.closing {
		return
	}
	history := controller.state.Peek()
	for _, conversation := range history.Conversations {
		if conversation.ID == id {
			history.ActiveConversationID = id
			controller.state.Write(history)
			controller.status.Write("")
			controller.persist()
			return
		}
	}
}

func (controller *chatController) newConversation() {
	if controller.busy.Peek() || controller.closing {
		return
	}
	current := controller.current()
	if len(current.Messages) == 1 && current.Draft == "" {
		return
	}
	conversation := createConversation(controller.nextConversation, controller.nextMessage, time.Now().UnixMilli())
	controller.nextConversation++
	controller.nextMessage++
	history := controller.state.Peek()
	history.ActiveConversationID = conversation.ID
	history.Conversations = append([]Conversation{conversation}, history.Conversations...)
	history.Conversations = history.Conversations[:min(len(history.Conversations), maxConversations)]
	controller.state.Write(history)
	controller.status.Write("")
	controller.persist()
}

func completionContext(messages []ChatMessage) []Message {
	var result []Message
	length := 0
	for index := len(messages) - 1; index >= 1; index-- {
		message := messages[index]
		if message.Streaming || message.Failed || message.Content == "" {
			continue
		}
		next := textLength(message.Content)
		if length+next > maxContextCharacters {
			break
		}
		length += next
		result = append(result, Message{Role: message.Role, Content: message.Content})
	}
	for left, right := 0, len(result)-1; left < right; left, right = left+1, right-1 {
		result[left], result[right] = result[right], result[left]
	}
	return result
}

func (controller *chatController) send() {
	conversation := controller.current()
	prompt := strings.TrimSpace(conversation.Draft)
	if controller.busy.Peek() || controller.closing || prompt == "" {
		return
	}
	secret := controller.key.Peek()
	if secret == "" {
		controller.settingsOpen.Write(true)
		return
	}
	responseID := controller.nextMessage + 1
	controller.nextMessage += 2
	controller.edit(conversation.ID, func(current *Conversation) {
		if len(current.Messages) == 1 {
			current.Title = titleFromPrompt(prompt)
		}
		current.Messages = append(current.Messages, ChatMessage{ID: responseID - 1, Role: "user", Content: prompt}, ChatMessage{ID: responseID, Role: "assistant", Streaming: true})
		if len(current.Messages) > maxTranscriptMessages {
			current.Messages = append(current.Messages[:1:1], current.Messages[len(current.Messages)-(maxTranscriptMessages-1):]...)
		}
		current.Draft = ""
		current.UpdatedAt = time.Now().UnixMilli()
	})
	messages := completionContext(controller.current().Messages)
	controller.busy.Write(true)
	controller.status.Write("Streaming response…")
	controller.generation++
	generation := controller.generation
	ctx, cancel := context.WithCancel(context.Background())
	controller.cancel = cancel
	go func() {
		defer cancel()
		text, err := controller.complete(ctx, secret, messages, func(text string) error {
			controller.dispatch(func() {
				if controller.generation == generation {
					controller.updateResponse(conversation.ID, responseID, text, true, false)
				}
			})
			return ctx.Err()
		})
		// Always deliver the final buffer, including after Stop. ui.Async discards
		// canceled results, which would lose tokens since the last 24 ms update.
		controller.dispatch(func() {
			if controller.generation != generation {
				return
			}
			failed := err != nil && !errors.Is(err, context.Canceled)
			if text == "" {
				switch {
				case errors.Is(err, context.Canceled):
					text = "_Generation stopped._"
				case err != nil:
					text = "_The response could not be completed._"
				default:
					text = "_The provider returned an empty response._"
				}
			}
			controller.updateResponse(conversation.ID, responseID, text, false, failed)
			controller.cancel = nil
			controller.busy.Write(false)
			switch {
			case errors.Is(err, context.Canceled):
				controller.status.Write("Generation stopped.")
			case err != nil:
				controller.status.Write(err.Error())
			default:
				controller.status.Write("")
			}
			callbacks := controller.finished
			controller.finished = nil
			for _, callback := range callbacks {
				callback()
			}
		})
	}()
}

func (controller *chatController) updateResponse(conversationID, messageID int64, text string, streaming, failed bool) {
	controller.edit(conversationID, func(conversation *Conversation) {
		for i := range conversation.Messages {
			if conversation.Messages[i].ID == messageID {
				conversation.Messages[i].Content = text
				conversation.Messages[i].Streaming = streaming
				conversation.Messages[i].Failed = failed
				return
			}
		}
	})
}

func (controller *chatController) stop() {
	if controller.cancel != nil {
		controller.cancel()
	}
}

func (controller *chatController) shutdown(done func(error)) {
	if controller.closing {
		return
	}
	controller.closing = true
	flush := func() {
		go func() {
			var err error
			if controller.writer != nil {
				err = controller.writer.Flush()
			}
			controller.dispatch(func() {
				if err != nil {
					controller.closing = false
				}
				done(err)
			})
		}()
	}
	if controller.busy.Peek() {
		controller.finished = append(controller.finished, flush)
		controller.stop()
	} else {
		flush()
	}
}

func credentialService() string {
	if value := strings.TrimSpace(os.Getenv("QUICKGUI_AI_CHAT_KEYCHAIN_SERVICE")); value != "" {
		return value
	}
	return "dev.quickgui.ai-chat-example"
}

const credentialAccount = "DeepSeek API key"

func (controller *chatController) loadCredential() {
	if controller.key.Peek() != "" {
		return
	}
	controller.credentialBusy.Write(true)
	native.SecureStorage.GetText(
		credentialService(),
		credentialAccount,
		func(value *string, err error) {
			controller.credentialBusy.Write(false)
			if err != nil {
				controller.status.Write("Unable to load API key: " + err.Error())
				return
			}
			if value != nil && controller.key.Peek() == "" {
				controller.key.Write(*value)
			}
		},
	)
}
