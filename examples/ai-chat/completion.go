package main

import (
	"bufio"
	"bytes"
	"context"
	"encoding/json"
	"errors"
	"fmt"
	"io"
	"net/http"
	"strings"
	"time"
)

type Message struct {
	Role    string `json:"role"`
	Content string `json:"content"`
}

// readStream consumes complete SSE records, preserving split UTF-8 sequences.
func readStream(reader io.Reader, emit func(string) error) error {
	scanner := bufio.NewScanner(reader)
	scanner.Buffer(make([]byte, 4096), 1024*1024)
	var data []string
	dataBytes := 0
	flush := func() error {
		payload := strings.Join(data, "\n")
		data = nil
		dataBytes = 0
		if payload == "" {
			return nil
		}
		if payload == "[DONE]" {
			return io.EOF
		}
		var chunk *struct {
			Choices []struct {
				Delta struct {
					Content string `json:"content"`
				} `json:"delta"`
			} `json:"choices"`
			Error *struct {
				Message string `json:"message"`
			} `json:"error"`
		}
		if err := json.Unmarshal([]byte(payload), &chunk); err != nil {
			return err
		}
		if chunk == nil {
			return errors.New("invalid null stream record")
		}
		if chunk.Error != nil {
			return errors.New(chunk.Error.Message)
		}
		if len(chunk.Choices) > 0 {
			return emit(chunk.Choices[0].Delta.Content)
		}
		return nil
	}
	for scanner.Scan() {
		line := scanner.Text()
		if line == "" {
			if err := flush(); err != nil {
				if errors.Is(err, io.EOF) {
					return nil
				}
				return err
			}
		} else if strings.HasPrefix(line, "data:") {
			value := strings.TrimPrefix(strings.TrimPrefix(line, "data:"), " ")
			dataBytes += len(value) + 1
			if dataBytes > 1024*1024 {
				return errors.New("stream record exceeds 1 MiB")
			}
			data = append(data, value)
		}
	}
	if err := scanner.Err(); err != nil {
		return err
	}
	if err := flush(); err != nil {
		if errors.Is(err, io.EOF) {
			return nil
		}
		return err
	}
	return io.ErrUnexpectedEOF
}

func complete(ctx context.Context, key string, messages []Message, update func(string) error) (string, error) {
	ctx, cancel := context.WithTimeout(ctx, 10*time.Minute)
	defer cancel()
	body, err := json.Marshal(map[string]any{
		"model": "deepseek-v4-flash", "messages": messages, "stream": true,
		"thinking": map[string]string{"type": "disabled"},
	})
	if err != nil {
		return "", err
	}
	request, err := http.NewRequestWithContext(ctx, "POST", "https://api.deepseek.com/chat/completions", bytes.NewReader(body))
	if err != nil {
		return "", err
	}
	request.Header.Set("Content-Type", "application/json")
	request.Header.Set("Authorization", "Bearer "+key)
	response, err := http.DefaultClient.Do(request)
	if err != nil {
		return "", err
	}
	defer response.Body.Close()
	if response.StatusCode != http.StatusOK {
		return "", fmt.Errorf("DeepSeek returned HTTP %d", response.StatusCode)
	}
	var text strings.Builder
	units := 0
	last := time.Now()
	err = readStream(io.LimitReader(response.Body, 32*1024*1024), func(delta string) error {
		remaining := maxResponseCharacters - units
		part := truncateText(delta, remaining)
		text.WriteString(part)
		units += textLength(part)
		if len(part) != len(delta) {
			return errors.New("response reached the 256,000-character limit")
		}
		if time.Since(last) >= 24*time.Millisecond {
			last = time.Now()
			return update(text.String())
		}
		return nil
	})
	return text.String(), err
}
