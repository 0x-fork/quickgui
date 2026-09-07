package main

import (
	"bytes"
	"encoding/json"
	"errors"
	"fmt"
	"io"
	"os"
	"path/filepath"
	"runtime"
	"slices"
	"strings"
	"sync"
	"time"
)

const (
	maxConversations      = 100
	maxTranscriptMessages = 101
	maxContextCharacters  = 96000
	maxPromptCharacters   = 16000
	maxResponseCharacters = 256000
	maxHistoryBytes       = 64 << 20
	welcomeContent        = "# QuickGUI AI Chat\n\nAsk a question to stream a response from **DeepSeek**. Markdown is parsed and rendered by QuickGUI."
)

type ChatMessage struct {
	ID        int64  `json:"id"`
	Role      string `json:"role"`
	Content   string `json:"content"`
	Streaming bool   `json:"streaming,omitempty"`
	Failed    bool   `json:"failed,omitempty"`
}

type Conversation struct {
	ID        int64         `json:"id"`
	Title     string        `json:"title"`
	Messages  []ChatMessage `json:"messages"`
	Draft     string        `json:"draft"`
	CreatedAt int64         `json:"createdAt"`
	UpdatedAt int64         `json:"updatedAt"`
}

// Version 1 is the original TypeScript example's on-disk schema.
type ConversationHistory struct {
	Version              int            `json:"version"`
	ActiveConversationID int64          `json:"activeConversationId"`
	Conversations        []Conversation `json:"conversations"`
}

func createConversation(id, messageID, now int64) Conversation {
	return Conversation{ID: id, Title: "New chat", Messages: []ChatMessage{{ID: messageID, Role: "assistant", Content: welcomeContent}}, CreatedAt: now, UpdatedAt: now}
}

func titleFromPrompt(prompt string) string {
	normalized := strings.Join(strings.Fields(prompt), " ")
	if normalized == "" {
		return "New chat"
	}
	characters := []rune(normalized)
	if len(characters) > 42 {
		return string(characters[:42]) + "…"
	}
	return normalized
}

// Match the old UTF-16 character budgets without cutting a Unicode code point.
func truncateText(text string, limit int) string {
	units := 0
	for index, char := range text {
		width := 1
		if char > 0xffff {
			width = 2
		}
		if units+width > limit {
			return text[:index]
		}
		units += width
	}
	return text
}

func textLength(text string) int {
	units := 0
	for _, char := range text {
		units++
		if char > 0xffff {
			units++
		}
	}
	return units
}

func historyPath() string {
	if override := strings.TrimSpace(os.Getenv("QUICKGUI_AI_CHAT_DATA_DIR")); override != "" {
		return filepath.Join(override, "history.json")
	}
	home, _ := os.UserHomeDir()
	switch runtime.GOOS {
	case "darwin":
		return filepath.Join(home, "Library", "Application Support", "QuickGUI AI Chat", "history.json")
	case "windows":
		base := strings.TrimSpace(os.Getenv("APPDATA"))
		if base == "" {
			base = filepath.Join(home, "AppData", "Roaming")
		}
		return filepath.Join(base, "QuickGUI AI Chat", "history.json")
	default:
		base := strings.TrimSpace(os.Getenv("XDG_DATA_HOME"))
		if base == "" {
			base = filepath.Join(home, ".local", "share")
		}
		return filepath.Join(base, "quickgui-ai-chat", "history.json")
	}
}

func parseHistory(data []byte) (ConversationHistory, error) {
	var encoded struct {
		Version              int               `json:"version"`
		ActiveConversationID int64             `json:"activeConversationId"`
		Conversations        []json.RawMessage `json:"conversations"`
	}
	if err := json.Unmarshal(data, &encoded); err != nil {
		return ConversationHistory{}, err
	}
	if encoded.Conversations == nil {
		return ConversationHistory{}, errors.New("chat history must contain a conversations array")
	}
	if encoded.Version != 1 {
		return ConversationHistory{}, fmt.Errorf("unsupported chat history version %d", encoded.Version)
	}
	history := ConversationHistory{Version: 1}
	seen := map[int64]bool{}
	for _, raw := range encoded.Conversations[:min(len(encoded.Conversations), maxConversations)] {
		var conversation Conversation
		var fields struct {
			Messages []json.RawMessage `json:"messages"`
		}
		// Decode messages individually, so an invalid entry cannot discard its siblings.
		var header map[string]json.RawMessage
		if json.Unmarshal(raw, &header) != nil || json.Unmarshal(raw, &fields) != nil {
			continue
		}
		if bytes.Equal(header["createdAt"], []byte("null")) || len(header["createdAt"]) == 0 || bytes.Equal(header["updatedAt"], []byte("null")) || len(header["updatedAt"]) == 0 {
			continue
		}
		delete(header, "messages")
		headerJSON, _ := json.Marshal(header)
		if json.Unmarshal(headerJSON, &conversation) != nil || conversation.ID <= 0 || conversation.ID > 1<<53-1 || seen[conversation.ID] || conversation.CreatedAt < 0 || conversation.UpdatedAt < 0 {
			continue
		}
		messageIDs := map[int64]bool{}
		for _, rawMessage := range fields.Messages[max(0, len(fields.Messages)-maxTranscriptMessages):] {
			var content struct {
				Content *string `json:"content"`
			}
			if json.Unmarshal(rawMessage, &content) != nil || content.Content == nil {
				continue
			}
			var message ChatMessage
			if json.Unmarshal(rawMessage, &message) != nil || message.ID <= 0 || message.ID > 1<<53-1 || messageIDs[message.ID] || message.Role != "user" && message.Role != "assistant" {
				continue
			}
			message.Content = truncateText(message.Content, maxResponseCharacters)
			if message.Streaming {
				message.Streaming = false
				message.Failed = true
				if message.Content == "" {
					message.Content = "_Generation was interrupted when the app closed._"
				}
			}
			conversation.Messages = append(conversation.Messages, message)
			messageIDs[message.ID] = true
		}
		if len(conversation.Messages) == 0 {
			continue
		}
		conversation.Title = titleFromPrompt(conversation.Title)
		conversation.Draft = truncateText(conversation.Draft, maxPromptCharacters)
		conversation.UpdatedAt = max(conversation.CreatedAt, conversation.UpdatedAt)
		history.Conversations = append(history.Conversations, conversation)
		seen[conversation.ID] = true
	}
	slices.SortStableFunc(history.Conversations, func(a, b Conversation) int {
		if a.UpdatedAt > b.UpdatedAt {
			return -1
		}
		if a.UpdatedAt < b.UpdatedAt {
			return 1
		}
		return 0
	})
	if seen[encoded.ActiveConversationID] {
		history.ActiveConversationID = encoded.ActiveConversationID
	} else if len(history.Conversations) > 0 {
		history.ActiveConversationID = history.Conversations[0].ID
	}
	return history, nil
}

func readHistoryFile(path string) ([]byte, error) {
	file, err := os.Open(path)
	if err != nil {
		return nil, err
	}
	defer file.Close()
	data, err := io.ReadAll(io.LimitReader(file, maxHistoryBytes+1))
	if len(data) > maxHistoryBytes {
		return nil, fmt.Errorf("chat history exceeds %d MiB", maxHistoryBytes>>20)
	}
	return data, err
}

func loadHistory(path, reducedPath string) (ConversationHistory, error) {
	data, err := readHistoryFile(path)
	if err == nil {
		return parseHistory(data)
	}
	if !errors.Is(err, os.ErrNotExist) {
		return ConversationHistory{}, err
	}
	if reducedPath == "" {
		return ConversationHistory{}, nil
	}
	data, err = readHistoryFile(reducedPath)
	if errors.Is(err, os.ErrNotExist) {
		return ConversationHistory{}, nil
	}
	if err != nil {
		return ConversationHistory{}, err
	}
	return importReducedHistory(data)
}

func importReducedHistory(data []byte) (ConversationHistory, error) {
	var reduced struct {
		Active        string `json:"active"`
		Conversations []struct {
			ID       string
			Title    string
			Draft    string
			Messages []Message
		} `json:"conversations"`
	}
	if err := json.Unmarshal(data, &reduced); err != nil {
		return ConversationHistory{}, err
	}
	history := ConversationHistory{Version: 1}
	nextMessage := int64(1)
	for index, old := range reduced.Conversations[:min(len(reduced.Conversations), maxConversations)] {
		conversation := createConversation(int64(index+1), nextMessage, time.Now().UnixMilli()-int64(index))
		nextMessage++
		conversation.Title, conversation.Draft = titleFromPrompt(old.Title), truncateText(old.Draft, maxPromptCharacters)
		for _, message := range old.Messages[max(0, len(old.Messages)-(maxTranscriptMessages-1)):] {
			if message.Role != "user" && message.Role != "assistant" {
				continue
			}
			conversation.Messages = append(conversation.Messages, ChatMessage{ID: nextMessage, Role: message.Role, Content: truncateText(message.Content, maxResponseCharacters)})
			nextMessage++
		}
		history.Conversations = append(history.Conversations, conversation)
		if old.ID == reduced.Active {
			history.ActiveConversationID = conversation.ID
		}
	}
	if history.ActiveConversationID == 0 && len(history.Conversations) > 0 {
		history.ActiveConversationID = history.Conversations[0].ID
	}
	return history, nil
}

func saveHistory(path string, data []byte) error {
	if err := os.MkdirAll(filepath.Dir(path), 0700); err != nil {
		return err
	}
	file, err := os.CreateTemp(filepath.Dir(path), ".history-*.tmp")
	if err != nil {
		return err
	}
	name := file.Name()
	defer os.Remove(name)
	if _, err = file.Write(data); err == nil {
		err = file.Sync()
	}
	closeErr := file.Close()
	if err != nil {
		return err
	}
	if closeErr != nil {
		return closeErr
	}
	return os.Rename(name, path)
}

// Schedule retains an immutable snapshot. Serialization and disk writes are both
// debounced off the UI goroutine; streaming does not encode the entire history
// for every delta. Flush persists the latest scheduled snapshot before quitting.
type historyWriter struct {
	path           string
	mu             sync.Mutex
	writeMu        sync.Mutex
	version, saved uint64
	history        ConversationHistory
	timer          *time.Timer
	onError        func(error)
}

func (writer *historyWriter) Schedule(history ConversationHistory) {
	writer.mu.Lock()
	writer.version++
	version := writer.version
	writer.history = history
	if writer.timer != nil {
		writer.timer.Stop()
	}
	writer.timer = time.AfterFunc(350*time.Millisecond, func() {
		if err := writer.write(version, history); err != nil && writer.onError != nil {
			writer.onError(err)
		}
	})
	writer.mu.Unlock()
}

func (writer *historyWriter) Flush() error {
	for {
		writer.mu.Lock()
		if writer.timer != nil {
			writer.timer.Stop()
			writer.timer = nil
		}
		version, history, saved := writer.version, writer.history, writer.saved
		writer.mu.Unlock()
		if version <= saved {
			return nil
		}
		if err := writer.write(version, history); err != nil {
			return err
		}
	}
}

func (writer *historyWriter) write(version uint64, history ConversationHistory) error {
	writer.writeMu.Lock()
	defer writer.writeMu.Unlock()
	writer.mu.Lock()
	stale := version <= writer.saved || version < writer.version
	writer.mu.Unlock()
	if stale {
		return nil
	}
	data, err := json.MarshalIndent(history, "", "  ")
	if err != nil {
		return err
	}
	if err := saveHistory(writer.path, data); err != nil {
		return err
	}
	writer.mu.Lock()
	writer.saved = version
	writer.mu.Unlock()
	return nil
}
