package main

import (
	"encoding/json"
	"os"
	"path/filepath"
	"strings"
	"testing"
)

func TestHistoryLoadsOriginalSchemaAndInterruptedMessages(t *testing.T) {
	history, err := parseHistory([]byte(`{"version":1,"activeConversationId":7,"conversations":[{"id":7,"title":"  First   chat ","draft":"unsent draft","createdAt":100,"updatedAt":200,"messages":[{"id":1,"role":"assistant","content":"welcome"},{"id":2,"role":"user","content":"Hello"},{"id":3,"role":"assistant","content":"partial reply","streaming":true},{"id":4,"role":"assistant"}]},{"id":8,"title":"Missing timestamps","messages":[{"id":5,"role":"user","content":"invalid"}]}]}`))
	if err != nil {
		t.Fatal(err)
	}
	if history.ActiveConversationID != 7 || len(history.Conversations) != 1 {
		t.Fatal(history)
	}
	conversation := history.Conversations[0]
	if conversation.Title != "First chat" || conversation.Draft != "unsent draft" || len(conversation.Messages) != 3 {
		t.Fatal(conversation)
	}
	last := conversation.Messages[2]
	if last.Streaming || !last.Failed || last.Content != "partial reply" {
		t.Fatal(last)
	}
}

func TestHistoryImportsReducedGoFileOnlyWhenOriginalMissing(t *testing.T) {
	dir := t.TempDir()
	original, reduced := filepath.Join(dir, "original.json"), filepath.Join(dir, "go.json")
	if err := os.WriteFile(reduced, []byte(`{"active":"old-id","conversations":[{"id":"old-id","title":"Previous Go chat","draft":"draft","messages":[{"role":"user","content":"hello"},{"role":"assistant","content":"reply"}]}]}`), 0600); err != nil {
		t.Fatal(err)
	}
	history, err := loadHistory(original, reduced)
	if err != nil || len(history.Conversations) != 1 || history.Conversations[0].Draft != "draft" || len(history.Conversations[0].Messages) != 3 {
		t.Fatal(history, err)
	}
	encoded, err := json.Marshal(history)
	if err != nil {
		t.Fatal(err)
	}
	if err := saveHistory(original, encoded); err != nil {
		t.Fatal(err)
	}
	if err := os.WriteFile(reduced, []byte(`invalid`), 0600); err != nil {
		t.Fatal(err)
	}
	restored, err := loadHistory(original, reduced)
	if err != nil || restored.ActiveConversationID != history.ActiveConversationID {
		t.Fatal(restored, err)
	}
	if err := os.WriteFile(original, []byte(`invalid`), 0600); err != nil {
		t.Fatal(err)
	}
	if _, err := loadHistory(original, reduced); err == nil {
		t.Fatal("corrupt original silently replaced")
	}
}

func TestTextBudgetsPreserveUnicodeAndTitle(t *testing.T) {
	if got := truncateText("A😀你好B", 4); got != "A😀你" || textLength(got) != 4 {
		t.Fatal(got)
	}
	if got := truncateText("😀", 1); got != "" {
		t.Fatal(got)
	}
	if got := titleFromPrompt(strings.Repeat("😀", 43)); got != strings.Repeat("😀", 42)+"…" {
		t.Fatal(got)
	}
}

func TestHistoryWriterFlushesLatestSnapshotAndRejectsStaleWrite(t *testing.T) {
	path := filepath.Join(t.TempDir(), "history.json")
	writer := &historyWriter{path: path}
	first := ConversationHistory{Version: 1, ActiveConversationID: 1, Conversations: []Conversation{createConversation(1, 1, 100)}}
	writer.Schedule(first)
	second := first
	second.Conversations = append([]Conversation(nil), first.Conversations...)
	second.Conversations[0].Draft = "latest draft"
	writer.Schedule(second)
	if err := writer.Flush(); err != nil {
		t.Fatal(err)
	}
	if err := writer.write(1, first); err != nil {
		t.Fatal(err)
	}
	restored, err := loadHistory(path, "")
	if err != nil || restored.Conversations[0].Draft != "latest draft" {
		t.Fatal(restored, err)
	}
	if first.Conversations[0].Draft != "" {
		t.Fatal("saved snapshot mutated")
	}
	entries, err := os.ReadDir(filepath.Dir(path))
	if err != nil || len(entries) != 1 {
		t.Fatal("temporary history files leaked", entries, err)
	}
}
