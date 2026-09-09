# Retained Markdown

QuickGUI core includes a native `Markdown` document. It parses CommonMark with tables,
strikethrough, and task-list extensions, then composes ordinary QuickGUI text and layout elements.
It does not use a webview or HTML renderer.

Keep one `Markdown` value for each logical document. Updating the source is controlled state:

```rust
use quickgui::{Color, Markdown, MarkdownStyle, View, ViewContext};

struct Article {
    body: String,
    markdown: Markdown,
    streaming: bool,
}

impl View for Article {
    fn render(&mut self, _cx: &mut ViewContext<'_, Self>) -> impl quickgui::IntoElement {
        self.markdown.set_streaming(self.streaming);
        self.markdown.set_style(
            MarkdownStyle::default()
                .link_color(Color::rgb8(96, 165, 250))
                .code_background(Color::rgb8(17, 24, 39)),
        );
        self.markdown.set_text(&self.body);
        self.markdown.element("article-markdown")
    }
}
```

`set_text` reports whether the source changed, whether the change was append-only, its byte reparse
boundary, and whether the input hit the source limit. Append-only updates retain settled parsed
blocks and flattened `StyledText`; only the unstable tail is reparsed and reshaped. While
`streaming` is true, incomplete emphasis, strike, code, and link markers are closed in a temporary
display tail without mutating the canonical source. Turning streaming off renders the exact final
parse.

## Supported presentation

The core renderer supports paragraphs, six heading levels, emphasis, strong text, strike,
inline/fenced code, links, block quotes, ordered/unordered/task lists, tables, rules, and image
fallback labels. `MarkdownStyle` controls semantic metrics and optional text, muted, link, code,
background, and border colors; surrounding layout and paint remain application-owned.

Links are currently styled and selectable but do not own an open callback. Images currently render
their alt text and URL instead of fetching remote content. Those behaviors stay explicit so the
core never performs network access or opens URLs without application policy.

One document accepts at most 4 MiB of UTF-8 source, 32,768 top-level parsed blocks, 64 levels of
container nesting, 64 table columns, and the existing bounded rich-text highlight count. Input is
truncated only at a valid UTF-8 boundary. Settled documents add no timer, task, redraw loop, or idle
work.

The Go mutation protocol has its own 1 MiB UTF-8 bound for any single string property;
the 4 MiB source bound applies to the direct Rust API.

## Go component

`ui.Markdown` retains the core document and parser across streamed appends. Supply
the source with `ui.Value` and the streaming state with `ui.Streaming`:

```go
func Answer() *ui.Element {
	answer, _ := ui.CreateSignal("## Response\n\nWaiting for input.")
	streaming, _ := ui.CreateSignal(false)
	return ui.Markdown().Value(answer()).Streaming(streaming).
		TextColor("#e4e4e7").
		FontSize(15).
		LineHeight(23).
		MarkdownLinkColor("#60a5fa").
		MarkdownCodeBackground("#090b10").
		MarkdownBorderColor("#343843")
}
```

Markdown children are not treated as source. Numeric/string children belong to
`ui.Text`; `ui.Markdown` consumes the complete document string in `Value`.
The complete streaming example is [AI Chat](../examples/ai-chat).

Return to the [documentation index](README.md).
