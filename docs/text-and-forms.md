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


## Extended text styling

Every helper below inherits through the subtree like the other typography helpers and is resolved
once per layout build, so retained shaping keys stay canonical: two runs share a shaped buffer only
when their spacing, direction, case mapping, and break behavior all agree.

### Text shadow

```rust
text("Soft drop shadow").text_shadow(2.0, 2.0, 6.0, Color::rgba8(0, 0, 0, 200))
```

The offset and color are exact: the run is painted a second time at the offset, in the shadow
color, beneath the real text. The blur radius is **approximated** — a non-zero blur paints four
extra copies spread over the radius at 45% alpha instead of a true Gaussian blur. A shadow adds at
most `MAX_TEXT_SHADOW_SAMPLES` (5) text primitives to the display list, reuses the run's shaping
key, and never splits the shaping cache. Offsets are clamped to `TextShadow::MAX_OFFSET` (256) and
the blur to `TextShadow::MAX_BLUR` (64). In a styled-text run, spans that declare their own
highlight color keep that color in the shadow copy, so shadows suit uniformly colored text.
`text_shadow_none()` clears an inherited shadow.

### Letter and word spacing

`letter_spacing(px)` adds advance after every glyph cluster; `word_spacing(px)` adds advance after
every `U+0020` word separator (other whitespace is untouched, matching the CSS default separator
set for Latin text). Both take logical pixels, may be negative, and are clamped to
`MAX_TEXT_SPACING` (256).

### Case mapping

`text_transform(TextTransform::Uppercase | Lowercase | Capitalize)`, with the `uppercase()`,
`lowercase()`, and `capitalize()` shorthands, rewrites the shaping input only.

- Selection, copy, accessibility, and every index an application observes keep pointing at the
  **original** string. Characters whose case mapping changes their UTF-8 length (`ß` → `SS`) map to
  the nearest real boundary of the source string instead of a byte inside it.
- Editable `text_input` content is **never** transformed: an input's shaped buffer must stay
  byte-identical to its controlled value so caret indices, IME state, and clipboard round-trips
  agree. A `text_transform` inherited by an input is dropped during the layout build.
- A case mapping is not composed with a custom truncation affix (`TextOverflow::Truncate` with a
  string other than `…`): such a run keeps its full shaped layout and relies on clipping. The
  default `…` ellipsis is unaffected, because Cosmic Text applies it inside the shaped buffer.
- `text_transform_none()` clears an inherited transform.

### Overline

`overline()` and `overline_color(color)` draw a line on the font's ascent, alongside the existing
`underline()` and `strikethrough()` decorations. All three participate in the same decoration
geometry pass and the same shaping key.

### Breaking and wrapping

`word_break(WordBreak::Normal | BreakAll | KeepAll)` and
`overflow_wrap(OverflowWrap::Normal | Anywhere | BreakWord)` map onto Cosmic Text's wrap modes:

| declaration | resulting mode |
| --- | --- |
| `whitespace_nowrap()` / `TextWrap::None` | no wrapping (wins over everything below) |
| `word_break(BreakAll)` | break between any two characters |
| `word_break(KeepAll)` | ordinary word wrapping |
| `overflow_wrap(Anywhere)` | break between any two characters |
| `overflow_wrap(BreakWord)` | word wrapping, falling back to characters for an unfittable word |
| neither | the element's `TextWrap` |

`word_break` wins over `overflow_wrap`, as in CSS. `KeepAll` is approximated: QuickGUI never breaks
inside a CJK run in that mode, but it also does not add the extra CJK break opportunities `Normal`
allows.

### Soft hyphens

`hyphens(Hyphens::Manual)` keeps author-placed soft hyphens (`U+00AD`) in the shaping input, where
they act as break opportunities. `Hyphens::None`, the default, removes them before shaping, so a
soft hyphen never renders and never introduces a break; the removal is mapped, so selection and
copy still address the original string including its soft hyphens.

Known limitation: whether a visible hyphen is drawn at a break in `Manual` mode comes from the
font's own `U+00AD` glyph. QuickGUI does not substitute a hyphen at the break, and does not
suppress a font-provided one mid-line. Automatic hyphenation (`hyphens: auto`) is not implemented.

### Direction and alignment

`text_start()` and `text_end()` align to the inline start and end of the resolved layout
direction; `TextAlign::Start` is the default, and it resolves to `Left` in LTR and `Right` in RTL.
`text_direction(TextDirection::Ltr | Rtl | Auto)` forces the base paragraph direction used when
shaping bidirectional content without mirroring layout. `Element::rtl()` already sets it for its
subtree. See "Layout direction" in `docs/view-api.md`.
