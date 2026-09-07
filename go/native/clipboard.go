package native

import "encoding/json"

// ClipboardEntry is one representation: text, image, data, bookmark, or files.
// A ClipboardItem can supply several representations of the same content.
type ClipboardEntry struct {
	Type     string
	Text     string
	Metadata string
	MIMEType string
	Data     []byte
	Paths    []string
	Title    string
	URL      string
}

type ClipboardItem struct {
	Entries []ClipboardEntry `json:"entries"`
}

type clipboardWireEntry struct {
	Kind     string   `json:"kind"`
	Text     *string  `json:"text,omitempty"`
	Metadata string   `json:"metadata,omitempty"`
	Format   string   `json:"format,omitempty"`
	Data     *[]byte  `json:"data,omitempty"`
	Paths    []string `json:"paths,omitempty"`
	URL      string   `json:"url,omitempty"`
}

func (entry ClipboardEntry) MarshalJSON() ([]byte, error) {
	encoded := clipboardWireEntry{Kind: entry.Type, Metadata: entry.Metadata, Format: entry.MIMEType, Paths: entry.Paths, URL: entry.URL}
	switch entry.Type {
	case "text":
		encoded.Text = &entry.Text
	case "bookmark":
		encoded.Text = &entry.Title
	case "image", "data":
		data := entry.Data
		if data == nil {
			data = []byte{}
		}
		encoded.Data = &data
	}
	return json.Marshal(encoded)
}

func (entry *ClipboardEntry) UnmarshalJSON(data []byte) error {
	var encoded clipboardWireEntry
	if err := json.Unmarshal(data, &encoded); err != nil {
		return err
	}
	*entry = ClipboardEntry{Type: encoded.Kind, Metadata: encoded.Metadata, MIMEType: encoded.Format, Paths: encoded.Paths, URL: encoded.URL}
	if encoded.Text != nil {
		if entry.Type == "bookmark" {
			entry.Title = *encoded.Text
		} else {
			entry.Text = *encoded.Text
		}
	}
	if encoded.Data != nil {
		entry.Data = *encoded.Data
	}
	return nil
}

type clipboardAPI struct{}

var Clipboard clipboardAPI

func (clipboardAPI) Read(done func(*ClipboardItem, error)) {
	commandJSON(map[string]any{"method": "read-clipboard"}, done)
}

func (clipboardAPI) ReadText(done func(*string, error)) {
	Clipboard.Read(func(item *ClipboardItem, err error) {
		if done == nil {
			return
		}
		if item != nil {
			for _, entry := range item.Entries {
				if entry.Type == "text" {
					done(&entry.Text, err)
					return
				}
			}
		}
		done(nil, err)
	})
}

func (clipboardAPI) Write(item ClipboardItem, done func(error)) {
	if item.Entries == nil {
		item.Entries = []ClipboardEntry{}
	}
	commandVoid(map[string]any{"method": "write-clipboard", "item": item}, done)
}

func (clipboardAPI) WriteText(text string, done func(error)) {
	Clipboard.Write(ClipboardItem{Entries: []ClipboardEntry{{Type: "text", Text: text}}}, done)
}

func (clipboardAPI) WriteImage(format string, data []byte, done func(error)) {
	Clipboard.Write(ClipboardItem{Entries: []ClipboardEntry{{Type: "image", MIMEType: format, Data: data}}}, done)
}

func (clipboardAPI) WriteFiles(paths []string, done func(error)) {
	Clipboard.Write(ClipboardItem{Entries: []ClipboardEntry{{Type: "files", Paths: paths}}}, done)
}

func (clipboardAPI) ReadFind(done func(*ClipboardItem, error)) {
	commandJSON(map[string]any{"method": "read-find-clipboard"}, done)
}

func (clipboardAPI) WriteFind(item ClipboardItem, done func(error)) {
	if item.Entries == nil {
		item.Entries = []ClipboardEntry{}
	}
	commandVoid(map[string]any{"method": "write-find-clipboard", "item": item}, done)
}
