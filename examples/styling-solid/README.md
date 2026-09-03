# QuickGUI Solid styling

Every declared style this example uses is translated into one of the Rust core's own bounded style
values and then owned by the core: JavaScript never measures, interpolates, or paints anything.

- **Text alignment** — `textAlign` including the direction-relative `start` and `end`, which stay
  logical across the boundary so an RTL subtree resolves them in the core.
- **Extended text styling** — `letterSpacing`, `wordSpacing`, `textTransform`, `textShadow`,
  `textDecoration` with its color, style, and thickness, plus `wordBreak`, `overflowWrap`, and
  `hyphens`.
- **Right to left** — `direction: "rtl"` with the logical `paddingStart` and `borderStartWidth`
  edges that follow it.
- **Gradients** — `linear-gradient()`, `radial-gradient()`, and `conic-gradient()` CSS strings, and
  the declared object form with an explicit interpolation space.
- **Corners, borders, and outlines** — a four-value `borderRadius`, per-corner radii, dashed and
  dotted borders, and outline rings that are painted outside the border box without affecting
  layout.
- **Filters and backdrop** — colour filters, a subtree blur, a drop shadow that follows the real
  painted alpha, and a `backdropFilter` material over a gradient.
- **Transforms and blending** — paint-only `transform` with `transformOrigin`, a hover lift that
  swaps the gradient, ring, and transform together, and three `mixBlendMode` values.
- **Sticky headers** — `position: "sticky"` with a `top` inset inside a scroll container.
- **Scroll snap** — `overflowX: "scroll"` with `scrollSnapType`, `scrollSnapAlign`, and
  `scrollSnapStop`.

```console
cd examples/styling-solid
bun run dev
```
