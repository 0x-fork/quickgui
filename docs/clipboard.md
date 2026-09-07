# Clipboard

[Documentation index](README.md)

QuickGUI exposes the application clipboard through every `EventContext`, including application-wide
callbacks that do not belong to a window. Text-editor Copy, Cut, and Paste shortcuts use the same
service, so application code and built-in controls cannot drift into separate ownership paths.

```rust
let item = ClipboardItem::new_string_with_json_metadata(
    "selected text",
    &serde_json::json!({ "selection_count": 2 }),
)?;
cx.write_to_clipboard(item)?;

if let Some(item) = cx.read_from_clipboard()? {
    let text = item.text();
    let entries = item.entries();
}
```

An empty `ClipboardItem` explicitly clears the target pasteboard:

```rust
cx.write_to_clipboard(ClipboardItem::default())?;
```

## Typed item model

A clipboard item can retain up to 32 ordered representations:

- `ClipboardString` stores UTF-8 text and optional UTF-8 metadata. JSON helpers serialize without
  panicking and deserialize into an application type.
- `ClipboardData` stores an arbitrary bounded MIME representation. `html`, `rtf`, and `utf8`
  constructors cover the common rich-text formats without inventing a second clipboard model.
- `ClipboardBookmark` stores one native URL/title pair.
- `ClipboardImage` stores encoded bytes and their `ClipboardImageFormat`; reading an image on
  macOS does not decode or expand it into RGBA memory.
- `ExternalPaths` stores a validated native file list. `ClipboardItem::text()` falls back to a
  newline-separated path list when no non-empty string representation exists.

Items and all payloads are immutable and backed by `Arc`, so cloning an item does not duplicate its
text, image bytes, path array, or entry array. Public constructors validate before retention and
return `ClipboardError` on failure.

| Bound | Maximum |
| --- | ---: |
| Entries | 32 |
| Aggregate text | 8 MiB |
| Aggregate metadata | 256 KiB |
| Aggregate arbitrary MIME data | 64 MiB |
| One MIME type | 256 bytes |
| Aggregate encoded images | 64 MiB |
| External paths | 1,024 |
| One UTF-8 path | 16 KiB |
| Aggregate path bytes | 8 MiB |
| Non-macOS decoded RGBA conversion | 64 MiB |

## macOS pasteboards

The macOS backend talks directly to `NSPasteboard`. General and Find pasteboard objects are retained
only after their first operation. Reads prioritize native file lists, then UTF-8 text, then encoded
PNG, JPEG, WebP, GIF, SVG, BMP, TIFF, ICO, or PNM bytes. `NSData.length` is checked before any native
payload is copied into Rust-owned memory.

QuickGUI writes external paths as native file URLs and also supplies a text fallback. HTML, RTF,
arbitrary MIME data, and URL bookmarks map to registered pasteboard types. Multiple string entries
are combined, while one string's metadata is stored in private pasteboard types beside a
deterministic hash of the text. Metadata is restored only when the hash still matches, preventing
stale application metadata from being attached after another process changes the text. Distinct
image and data formats can coexist in one native declaration.

macOS's shared Find pasteboard is available through the same bounded model:

```rust
cx.write_to_find_pasteboard(ClipboardItem::new_string("needle")?)?;
let shared_search = cx.read_from_find_pasteboard()?;
```

### Go

`native.Clipboard` exposes asynchronous `Read`, `ReadText`, `Write`, `WriteText`,
`WriteImage`, `WriteFiles`, `ReadFind`, and `WriteFind` operations. A
`native.ClipboardItem` contains ordered `ClipboardEntry` values with a `Type` of
`"text"`, `"data"`, `"image"`, `"files"`, or `"bookmark"`. Arbitrary MIME data uses
`MIMEType` and `Data`; encoded images use those fields with `Type: "image"`.

```go
func CopyCustomData() {
	native.Clipboard.Write(
		native.ClipboardItem{Entries: []native.ClipboardEntry{
			{Type: "text", Text: "Selected item", Metadata: `{"selection_count":1}`},
			{Type: "data", MIMEType: "application/x-quickgui", Data: []byte{1, 2, 3}},
		}},
		func(err error) {
			if err != nil {
				log.Print(err)
				return
			}
			native.Clipboard.Read(func(item *native.ClipboardItem, err error) {
				if err != nil {
					log.Print(err)
				} else if item != nil {
					for _, entry := range item.Entries {
						log.Print(entry.Type, entry.MIMEType)
					}
				}
			})
		},
	)
}
```

`ReadText` returns the first text representation, or `nil` when none is available.
Use `Read` to inspect image, file, bookmark, or MIME representations; it does not
flatten them into text. `Write(native.ClipboardItem{}, done)` clears the general
clipboard. `ReadFind` and `WriteFind` target the separate macOS Find pasteboard;
an empty item clears it. Off macOS, Find reads return no item and writes report an
unsupported-operation error. Completion callbacks run on the Go application
goroutine; callers never synchronously wait for native main-thread execution.

## Other platforms

Windows and Linux use the lazy `arboard` backend. Text and native file lists remain typed. Clipboard
images are converted to or from bounded RGBA and PNG because those platform abstractions do not
expose every original encoding. Arbitrary MIME data, bookmarks, and hash-bound metadata are
currently preserved by the direct macOS backend. Multi-format writes on Windows/Linux choose native
paths first, then an image, then combined text; richer native projection belongs to their remaining
runtime-acceptance work.

## Scheduling and tests

Clipboard access is synchronous and intended for the serialized application thread. QuickGUI does
not observe change counts, poll either pasteboard, install a timer, or invalidate a window merely
because clipboard contents changed.

`TestAppContext` substitutes an in-memory general clipboard and a separate in-memory Find pasteboard
on macOS. `read_from_clipboard` and `write_to_clipboard` can seed external state, while callbacks
exercise the ordinary `EventContext` methods. Simulated editor Copy, Cut, and Paste shortcuts route
through that same store.

Run the interactive sample with:

```console
cargo run --release --example clipboard
```
