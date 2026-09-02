# Text, editing, and forms

[Documentation index](README.md)

Ordinary text wraps at word boundaries by default and contributes its wrapped height to flex layout;
use `.no_wrap()` or the GPUI-compatible `.whitespace_nowrap()` where a single line is intentional.
Horizontal alignment is inherited like color and font properties: `.text_left()`, `.text_center()`, and `.text_right()` match GPUI, while
`.text_justify()` expands eligible spaces on every wrapped paragraph line except its last. Alignment
uses the element's assigned width and is shared by painting, caret placement, hit testing, selection,
and the bounded text-layout cache.

Web-like overflow is also inherited and composes with the same Flexbox constraints:

```rust
text("A title that must stay on one line")
    .w_full()
    .min_w(0.0)
    .truncate()

text("/a/very/long/path/to/important-file.rs")
    .w_full()
    .min_w(0.0)
    .whitespace_nowrap()
    .text_ellipsis_middle()
    .overflow_hidden()

text(long_summary)
    .w_full()
    .min_w(0.0)
    .line_clamp(2)
    .text_ellipsis()
```

`.truncate()` combines no wrapping, hidden overflow, and an end ellipsis. The separate
`.text_ellipsis()`, `.text_ellipsis_start()`, and `.text_ellipsis_middle()` helpers preserve the
start, end, or both ends; `TextOverflow` accepts a custom replacement string. `.line_clamp(n)`
limits layout to at least one visible line and clips by itself, while adding an ellipsis replaces
the omitted range on the final line. Use `.whitespace_normal()` to restore inherited word wrapping.

The standard `…` helpers use Cosmic Text's native Unicode-aware ellipsizing directly on the original
shaped buffer. A width change therefore reruns only line layout while preserving glyph shaping. A
custom replacement string uses a grapheme-safe visible projection on a cache miss and maps styled
ranges and pointer/selection offsets back to the source. Stable frames perform neither projection
nor shaping work, and a superseded uniquely owned custom-affix width is released immediately.
AccessKit always exposes the complete value. Controlled inputs intentionally ignore inherited display truncation because their
viewport and caret scrolling must retain a one-to-one editable buffer. Run
`cargo run --release --example text_overflow` for all five modes.

Complete fonts and their independently inherited parts use a GPUI-shaped API:

```rust
use quickgui::{
    FontFallbacks, FontFamily, FontFeatureTag, FontFeatures, font,
};

let editor_font = font(FontFamily::Monospace)
    .features(FontFeatures::disable_ligatures())
    .fallbacks(FontFallbacks::from_fonts([
        "Noto Sans CJK SC",
        "Noto Sans Arabic",
        "Apple Color Emoji",
    ]));

div()
    .font(editor_font.clone())
    .child("Inherited editor typography")
    .child(
        text("012345")
            .font_features(
                FontFeatures::new().enable(FontFeatureTag::TABULAR_NUMBERS),
            ),
    )
```

`font(...)` carries family, features, fallbacks, weight, and style as one hashable value;
`.font_family(...)`, `.font_features(...)`, and `.font_fallbacks(...)` override only that inherited
part. An empty fallback stack clears an inherited custom stack. The primary family is tried first,
then declared fallbacks in order, then Cosmic Text's platform/script fallbacks. OpenType features
and fallback stacks are part of plain and rich shaping-cache identity; canonical feature ordering
allows equivalent declarations to share a layout. `FontFeatureTag` accepts exactly four ASCII
letters or digits, one style retains at most 32 feature entries and eight fallback families, and
names are capped at 1 KiB. The default advanced shaper applies both features and fallback. The
explicit `.text_shaping_basic()` fast path remains for controlled glyph-complete text and does not
provide OpenType shaping or general fallback.

Immutable rich text uses GPUI-shaped byte-range highlights while retaining one shaped buffer:

```rust
use quickgui::{Color, HighlightStyle, styled_text};

let source = "GPU cached · deprecated";
let rich = styled_text(source).with_highlights([
    (0..10, HighlightStyle::default()
        .font_semibold()
        .color(Color::rgb8(94, 234, 212))
        .background(Color::rgba8(13, 148, 136, 48))),
    (13..23, HighlightStyle::default()
        .color(Color::rgb8(248, 113, 113))
        .strikethrough()),
]);

div().w_full().text_lg().text_center().child(rich)
```

Ordinary containers can apply the same font and decoration properties to every descendant:

```rust
div()
    .italic()
    .underline()
    .text_decoration_color(Color::rgb8(56, 189, 248))
    .text_decoration_2()
    .text_decoration_wavy()
    .child("Inherited emphasis")
    .child(text("plain exception").not_italic().text_decoration_none())
```

`italic`/`not_italic`, `underline`/`double_underline`, `line_through`, underline color, and
`text_decoration_none` are joined by GPUI's `text_decoration_solid`/`text_decoration_wavy` and
`text_decoration_0`/`1`/`2`/`4`/`8` thickness helpers. They are part of the retained shaping key and
generate decoration geometry only for visible lines. Each solid span is one quad; each complete
wavy span is one analytic instance in the same GPU shape batch, rather than a CPU-tessellated path.
A width-only resize still reflows the existing shaped buffer, and stable frames add no timer,
allocation, or idle repaint.

Ranges are sorted, non-overlapping UTF-8 byte offsets checked at the API boundary. Foreground,
font family/features/fallbacks/weight/italic, background, solid or wavy single/double underline, thickness, and
strikethrough can vary per range. Wrapping, bidirectional shaping, measurement, glyph painting,
backgrounds, and decorations all reuse one bounded Cosmic Text entry; background rectangles and
decoration spans are each capped at 16,384 entries per text element after visible-line filtering.
Clones share the string and immutable run table, each element accepts at most 4,096 non-empty
highlights, and a
background-color-only change does not invalidate glyph shaping. See
`cargo run --release --example styled_text`.

Plain and styled immutable text is selectable by default outside controls. Drag selection crosses
sibling text leaves in document order, double-click selects Unicode words, triple-click selects a
logical line, Shift extends the existing range, Cmd/Ctrl-C copies, and Cmd/Ctrl-A selects all
mounted immutable text. Buttons, pointer listeners, and drag sources inherit web-like
`user-select: none`; use `.user_select_text()` (or `.selectable()`) to opt a control subtree back in,
and `.user_select_none()` to suppress selection explicitly. Selection geometry reuses the retained
Cosmic Text buffer and does not enter its layout key. Cross-leaf clipboard text is joined by visual
line and capped at 8 MiB without splitting UTF-8.

Controlled text fields and text areas use the same listener pattern:

```rust
let edit_name = cx.input_listener("name", |this, value, cx| {
    this.name = value.into();
    cx.invalidate();
});
let submit_name = cx.submit_listener("name", |this, value, cx| {
    this.save_name(value);
    cx.invalidate();
});
let edit_notes = cx.input_listener("notes", |this, value, cx| {
    this.notes = value.into();
    cx.invalidate();
});

text_input(self.name.clone())
    .on_input(edit_name)
    .on_submit(submit_name)
    .max_length(32)
    .input_filter(|value| !value.contains('\t'))
    .invalid(self.name.trim().is_empty())
    .validation_message("Name is required")
    .placeholder("Type a name…")
    .accessibility_label("Name")
    .w_full()

text_area(self.notes.clone())
    .on_input(edit_notes)
    .placeholder("Write multiline notes…")
    .accessibility_label("Notes")
    .size(480.0, 180.0)
```

Input constraints run against the proposed complete value before retained text or undo history is
changed. Maximum length counts Unicode grapheme clusters and truncates paste and IME commits only
at grapheme boundaries. A rejected edit leaves the value, selection, composition backup, history,
and `on_input` listener untouched. Invalidity remains controlled application state: it adds a
paint-only invalid style and native accessibility metadata, and suppresses `.on_submit(...)`
without moving focus. Single-line Return submits the committed value once; Return in a text area
continues to insert a newline.

For browser-style forms, `Field` decorates application-owned root, label, control, description, and
error parts. `Fieldset` gives related fields an exact legend/description relationship and passes
its disabled state into fields created with `.field(...)`. Neither component adds paint, layout,
validation polling, or hidden input state:

```rust
let save = cx.form_submit_listener("profile", |this, event, cx| {
    this.save(event.value("name").unwrap_or_default());
    cx.invalidate();
});
let report = cx.form_invalid_listener("profile", |this, report, cx| {
    this.status = report.first().and_then(|issue| issue.message()).unwrap_or("Invalid").into();
    cx.invalidate();
});
let edit_name = cx.input_listener("name", |this, value, cx| {
    this.name = value.into();
    cx.invalidate();
});
let profile = Fieldset::new("profile-fields");
let name = profile
    .field("name")
    .required(true)
    .invalid(self.name.trim().is_empty())
    .dirty(!self.name.is_empty())
    .filled(!self.name.is_empty())
    .validation_message("Name is required");

form()
    .on_form_submit(save)
    .on_form_invalid(report)
    .child(
        profile.root_part(div().children([
            profile.legend_part(text("Profile")),
            profile.description_part(text("Public account details")),
            name.root_part(div().children([
                name.label_part(text("Name")),
                name.control_part(
                    text_input(self.name.clone())
                        .on_input(edit_name)
                        .placeholder("Type a name…"),
                ),
                name.description_part(text("Shown on your profile")),
                name.error_part(text("Name is required")),
            ])),
        ])),
    )
    .child(submit_button().child("Save"))
```

Clicking a normal field label focuses a text input or activates a checkbox/radio control. Use
`.passive_label_part(...)` for a button-like select or combobox label that should name the control
without forwarding activation. A mounted invalid control references both its visible description
and visible error; a valid error part is `display: none`. `FieldState` exposes controlled
disabled/invalid/required/touched/dirty/filled flags so applications can style every part without
framework tokens.

Valid submissions expose up to 256 document-ordered controlled values without copying their text.
Invalid reports retain up to 256 issues; each message is truncated safely at 4 KiB, disabled and
nested-form controls are excluded, and focus moves to the first focusable invalid control. Every
failed attempt replaces one assertive AccessKit live node, so an identical retry is announced
again. This costs one action-driven redraw and retains one announcement—there is no validation
polling or idle frame loop. Custom controls can request the same path with
`EventContext::submit_form(...)`.

`required(true)` projects native accessibility state but does not invent a validation rule. The
application remains authoritative for the controlled invalid flag and message. Field and Fieldset
descriptors retain only IDs, six booleans, and an optional bounded shared message; they allocate no
registry and add no idle work.

Text areas preserve normalized newlines, wrap and hit-test visual lines, keep the caret visible in
both axes, and support Up/Down/Page navigation, macOS word/line/document shortcuts, drag
selection, IME composition, and the same native-style auto-hiding scrollbar as other scroll
containers. The renderer reuses one retained Cosmic Text layout for those operations; scrolling
does not start an idle frame loop.

The same bounded `StyledText` value can drive a controlled attributed input without a parallel
editor widget:

```rust
let edit_code = cx.input_listener("code", |this, value, cx| {
    this.code = value.into();
    cx.invalidate();
});

let value = styled_text(self.code.clone()).with_highlights(syntax_runs(&self.code));

styled_text_area(value)
    .on_input(edit_code)
    .font_family(FontFamily::Monospace)
    .no_wrap()
    .size(640.0, 240.0)
```

Attributed fields and areas use the run table for shaping, measurement, caret placement, pointer
hit testing, selection, IME geometry, glyphs, backgrounds, and decorations. Accepted local edits
shift, split, and merge runs until the controlled view supplies its next table; undo and redo
restore the corresponding styles. Run metadata and named-family bytes count toward each input's
existing 512 KiB history budget, the live table remains capped at 4,096 runs, and highlighting
runs only after application value changes—never on idle frames.

## Solid: Field and Fieldset

`@quickgui/solid` exposes the field layer as `Field.Root`, `Field.Label`, `Field.Control`,
`Field.Description`, `Field.Error`, and `Fieldset.Root`, `Fieldset.Legend`,
`Fieldset.Description`, `Fieldset.Control`.

```tsx
<Fieldset.Root disabled={saving()}>
  <Fieldset.Legend>Account</Fieldset.Legend>
  <Field.Root invalid={!valid()} required validationMessage="Enter an address">
    <Field.Label>Email</Field.Label>
    <Field.Control value={email()} placeholder="you@example.com" onInput={update} />
    <Field.Description>We never share it.</Field.Description>
    <Field.Error>Enter an address</Field.Error>
  </Field.Root>
</Fieldset.Root>
```

Each `Field.Root` allocates one bounded scope key that every part repeats, so the Rust binding
rebuilds the same `Field` descriptor and applies the derived root, label, description, error, and
control identities the core would have used. Because `control_part` owns the control's identity,
`Field.Control` *is* the control rather than a wrapper; its `element` prop selects the native
element and defaults to `input`. `Field.Label` forwards clicks to the control unless `passive` is
declared, a nested `Field.Root` inherits `Fieldset.Root`'s disabled state, and the six controlled
booleans plus the bounded `validationMessage` pass straight through to `FieldState`. See the
[Solid renderer guide](solid.md).
