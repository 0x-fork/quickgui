package main

import (
	"io"
	"strings"
	"testing"
)

type oneByteReader struct{ text string }

func (r *oneByteReader) Read(p []byte) (int, error) {
	if r.text == "" {
		return 0, io.EOF
	}
	p[0] = r.text[0]
	r.text = r.text[1:]
	return 1, nil
}

func TestStreamPreservesFragmentedUTF8AndRecords(t *testing.T) {
	reader := &oneByteReader{text: ": keepalive\n\ndata: {\"choices\":[{\"delta\":{\"content\":\"你好\"}}]}\r\n\r\ndata: [DONE]\n\n"}
	var text string
	err := readStream(reader, func(delta string) error { text += delta; return nil })
	if err != nil || text != "你好" {
		t.Fatal(text, err)
	}
}

func TestStreamRejectsTruncationAndProviderErrors(t *testing.T) {
	for _, source := range []string{"data: {", "data: {\"error\":{\"message\":\"overloaded\"}}\n\n"} {
		if err := readStream(strings.NewReader(source), func(string) error { return nil }); err == nil {
			t.Fatal("stream failure was reported as a completed response")
		}
	}
}
