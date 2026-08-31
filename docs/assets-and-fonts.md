# Application assets and custom fonts

[Documentation index](README.md)

QuickGUI gives the application one immutable asset source shared by every view, event callback,
window, and image-decode worker. `BundledAssets` is the default in-memory implementation for
resources compiled with `include_bytes!`:

```rust
use quickgui::{Application, Assets, BundledAssets, WindowOptions};

let bundle = BundledAssets::new()
    .with("icons/logo.svg", include_bytes!("../assets/logo.svg"))?
    .with("images/hero.webp", include_bytes!("../assets/hero.webp"))?
    .with("fonts/Inter-Regular.ttf", include_bytes!("../assets/Inter-Regular.ttf"))?;
let assets = Assets::new(bundle);

let view = MyView::new(assets.svg("icons/logo.svg")?);
Application::new()
    .assets(assets)
    .font("fonts/Inter-Regular.ttf")
    .run(move |cx| {
        cx.open_window(WindowOptions::default(), view);
    })?;
```

`Application::with_assets(bundle)` is a shorter form when application code does not need the
`Assets` handle before startup. Static byte slices remain borrowed; owned bytes use one shared allocation.
Asset paths are normalized relative UTF-8 paths, so absolute paths, backslashes, empty segments,
`.` and `..` are rejected.

## Access from views and callbacks

`ViewContext::assets()` and `EventContext::assets()` return the same cheap handle. The
`asset_source()` name is provided as a GPUI-shaped alias:

```rust
let save = cx.listener("save", |this, cx| {
    let Ok(template) = cx.assets().load_required("copy/template.txt") else {
        return;
    };
    this.replace_template(template.as_ref());
    cx.invalidate();
});
```

`load` distinguishes a missing asset with `Ok(None)`, while `load_required` returns
`AssetError::NotFound`. `list(prefix)` returns normalized paths in deterministic order.

Sources are synchronous because compiled asset maps complete immediately. An application-defined
`AssetSource` must not perform filesystem or network work from `View::render` or an input callback;
clone `Assets` into `spawn_background` for a source that may block. The asset layer installs no
watcher, timer, polling loop, or redraw source.

## Images and SVG

Use `assets.image(path)` for raster images displayed with `img(...)`. The returned
`ImageResource` is keyed by the asset-source identity and normalized path. Reconstructing the same
handle during a view declaration therefore reuses the per-window resource state, while decoding
still runs on QuickGUI's existing bounded sleeping image-worker pool:

```rust
let avatar = cx.assets().image("images/avatar.webp")?;

img(avatar)
    .size(48.0, 48.0)
    .rounded_full()
```

`assets.svg(path)` parses synchronously and should be called once during application or view
construction. Retain the resulting `Svg`; do not parse it in every render.

## Custom fonts

Register OpenType fonts before `run` with either bytes or an asset path:

```rust
use quickgui::{Application, FontFallbacks, FontFeatureTag, FontFeatures, WindowOptions, font, text};

Application::new()
    .font(include_bytes!("../assets/Inter-Regular.ttf"))
    .font("fonts/Inter-Bold.ttf")
    .run(|cx| {
        cx.open_window(WindowOptions::default(), MyView);
    })?;

let heading = text("Fast, quiet UI")
    .font(
        font("Inter")
            .bold()
            .features(FontFeatures::new().enable(FontFeatureTag::TABULAR_NUMBERS))
            .fallbacks(FontFallbacks::from_fonts(["Apple Color Emoji"])),
    );
```

QuickGUI resolves and validates every declared font before creating a native window or renderer.
All application windows then share one main-thread Cosmic Text font database. Each window keeps its
own bounded text-layout cache and Glyphon atlas, so closing one window still releases its rendering
working set. Sharing font metadata adds no lock, background task, or idle work because Winit
serializes renderer access on the application thread.

Font family, OpenType feature, and ordered fallback declarations apply to ordinary, controlled,
and `StyledText` runs. Fallback names resolve from the same application-wide database, so a
registered application font can be used as either a primary or fallback family. SVG `<text>` uses resvg's separate
system-font database; application font registration is not injected into SVG documents.

## Resource bounds

The public constants expose the same limits enforced at the boundary:

- an asset path is at most 4 KiB and one loaded asset is at most 64 MiB;
- `BundledAssets` retains at most 4,096 entries and 256 MiB in aggregate;
- a listing returns at most 4,096 paths and 16 MiB of aggregate path data;
- an application registers at most 64 font files and 64 MiB of custom font data;
- one font file is at most 16 MiB and 32 faces, with 256 custom faces per application.
- one text style retains at most eight ordered fallback names, 32 OpenType features, and 1 KiB per
  family name.

Font collection face counts are inspected before font parsing. Invalid fonts and any limit breach
fail startup explicitly rather than creating a partial font database or allowing unbounded cache
growth.

Run the self-contained asset example with:

```console
cargo run --release --example assets
```
