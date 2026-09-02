# Changelog

All notable user-facing changes to QuickGUI are recorded here.

## Unreleased

### Framework

- Added element background gradients: `bg_linear_gradient`, `bg_radial_gradient`,
  `bg_radial_gradient_at`, `bg_conic_gradient`, and the general `bg_gradient`, backed by a new
  `Gradient`/`ColorStops`/`GradientKind` API with `MAX_GRADIENT_STOPS` (8) stops, CSS angles or
  `GradientDirection`, `RadialGradientShape`/`RadialGradientExtent`/`GradientCenter`, and
  linear-sRGB, sRGB, or Oklab interpolation. Gradients are evaluated analytically in the existing
  instanced shape draw, honor rounded corners, borders, clipping, subtree opacity, and damage, and
  are swappable in hover, active, focus, validation, and drag states through
  `ElementStateStyle::bg_gradient`. `MAX_GRADIENTS_PER_FRAME` (4,096) bounds the per-frame upload.
- Extended `Background` so retained paths and canvas fills accept the same multi-stop linear,
  radial, and conic gradients instead of only two-stop linear gradients.
- Added per-corner radii: `rounded_tl`, `rounded_tr`, `rounded_br`, `rounded_bl`, `rounded_t`,
  `rounded_b`, `rounded_l`, `rounded_r`, `corner_radii(Corners)`, and `rounded_full()`. Radii are
  capped by `MAX_CORNER_RADIUS` and reduced by the CSS uniform-scale rule; element box shadows
  follow the same per-corner geometry.
- Added `border_solid()`, `border_dashed()`, and `border_dotted()`. Dash geometry is analytic arc
  length along the rounded outline, with the period scaled so a whole number of repeats closes the
  outline.
- Added `outline(width, color)`, `outline_offset(px)`, `outline_style`, `outline_dashed`,
  `outline_dotted`, and `outline_none`, plus `ElementStateStyle::outline`/`outline_offset` for
  focus rings. Outlines are painted outside the border box and never affect layout, bounded by
  `MAX_OUTLINE_WIDTH` and `MAX_OUTLINE_OFFSET`.
- Added raster element backgrounds: `bg_image(image, BackgroundSize, BackgroundRepeat,
  BackgroundPosition)` with `bg_image_cover`, `bg_image_contain`, `bg_image_tiled`, and
  `bg_image_none`. Tiles reuse the existing bounded image primitive and GPU texture cache, are
  masked by the element's rounded corners, and are capped by `MAX_BACKGROUND_IMAGE_TILES` (256).
- Added bounded CSS-shaped color filters: `Filter`, `Filters`, `ColorMatrix`,
  `MAX_FILTERS_PER_ELEMENT`, `Element::filters`/`filter`, and the `brightness`, `contrast`,
  `saturate`, `invert`, `sepia`, and `hue_rotate` shorthands. `grayscale(bool)` now routes through
  the same chain, and `ImagePrimitive::grayscale` is expressed as a `ColorMatrix`. Filters apply to
  an element's own image and background-image pixels; subtree filters, blur, and drop-shadow are
  not implemented because they require an offscreen group texture.
- Added the `effects` example.

## 0.1.1 - 2026-08-31

### JavaScript tooling

- Changed Solid window mounting to `new Window({ renderer: createRenderer(() => <App />) })`.
  Applications now import native lifecycle APIs from `@quickgui/native` directly; the Solid
  package no longer re-exports them. The Electron-style singleton `app` exposes `whenReady()`, the
  CLI owns its application loop, and creating a window before readiness now throws explicitly.
  Readiness comes from the new windowless Rust-core `Application`/`AppRunner` lifecycle, while
  `Window.getCurrentWindow()` projects the core-routed render or event window.
- Added `@quickgui/cli` project initialization, target-aware production builds, and a stable native
  development host. On macOS, development runs a signed `.app`, loads project TS/TSX on demand,
  restarts without repackaging on source edits, and keeps the prior app alive when a candidate
  cannot start.
- Production macOS builds now retain the signed `.app`, create a versioned DMG with `create-dmg`,
  and can submit, staple, and validate the DMG with an Apple Notary Keychain profile.
- Published the initial `@quickgui/native`, `@quickgui/solid`, and `@quickgui/cli` packages for
  macOS arm64 and x64.
- Added controlled `Input`/`TextArea` and retained core `Markdown` bindings to
  `@quickgui/native`/`@quickgui/solid`, plus a packaged Solid AI chat example using Vercel AI SDK
  streaming and the DeepSeek provider.
- Added Base-UI-shaped `Root`/`Trigger`/`Popup` parts for Solid `Popover` and `SystemPopover`.
  Triggers now register their retained native anchor internally, while only the system popup subtree
  crosses into its parent-owned native child window renderer.
- Fixed native controlled inputs resetting each keystroke before Solid could commit the queued
  value update.
- Fixed the Solid compiler plugin to preserve and then correctly erase TypeScript syntax after JSX
  lowering, allowing typed `.tsx` applications to build for production.

### Framework

- Added a bounded retained native `Markdown` document with CommonMark tables, task lists, streamed
  tail mending, append-only settled-prefix parsing, and cached `StyledText` flattening.

## 0.1.0 - 2026-08-27

Initial macOS-first framework release.

### Framework

- Damage-driven WGPU rendering with clean-window sleep, bounded renderer caches, batched shapes,
  retained text, images, SVGs, paths, custom shaders, shadows, and declarative motion.
- GPUI-shaped retained views with Flexbox, CSS Grid, container queries, inherited typography,
  Tailwind-style helpers, typed actions, entities, globals, async tasks, and multi-window ownership.
- Uniform and measured variable-height virtualization with native-style captured overlay scrollbars.

### macOS

- Native window roles, hidden-inset title bars, traffic-light positioning, display-aware placement,
  menus, dialogs, clipboard, notifications, document state, drag and drop, and platform services.
- Embedded `NSView` children composed between retained WGPU base and overlay planes without a black
  first frame or cross-display startup movement.
- Overflow-capable nonactivating child panels for popovers, menus, select, autocomplete, combobox,
  and context menus.

### Interaction and components

- Keyboard, mouse, IME, editable and selectable text, forms, accessibility, gestures, native
  cursors, tooltips, typed drag and drop, and deterministic input simulation.
- Unstyled selection controls, tabs, fields, disclosures, dialogs, popover menus, pickers, virtual
  tables, and virtual trees with application-owned presentation.

### Validation and resource ownership

- A 561-test library suite, downstream test-support coverage, all-example and benchmark compilation,
  warning-free Clippy/rustdoc gates, and Rust 1.89 downstream package verification.
- Live 100,000-row scrolling and AppKit composition gates enforce frame-time, CPU, memory, cache,
  lifecycle, resize, focus, native-view, popover, and idle-frame budgets.

### Platform scope

- macOS is the accepted 0.1 platform. Windows and Linux compile through Winit/WGPU but do not yet
  carry equivalent native runtime, visual, accessibility, or resource acceptance evidence.
