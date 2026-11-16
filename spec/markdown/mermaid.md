# Markdown diagrams

- Mermaid is compiled to SVG by `crates/mermaid`, which has no dependencies.
  It started as a copy of Ondine (a Rust port of Obsidian's Pele) and is owned
  here: Ondine and Pele are references for porting fixes, not upstreams to
  merge (porting progress: `mermaid_upstream.md`). It has no Cargo
  features: every diagram type is always compiled and renders; a diagram
  that fails to compile shows its source.
- `mermaid::render(source, &Options, &Theme)` takes typed `mermaid::Options`
  (font size, padding, source and output limits, max edges), not JSON. The
  SVG is only rasterized in the markdown preview, so browser-only features are
  gone:
  - fixed document size (no `responsive` / `useMaxWidth`) and fixed internal
    ids;
  - no fitting to a host width: every diagram draws at its natural width in
    its source direction (no max width, auto direction, tightening, or
    narrow layouts);
  - links, tooltips, callbacks, sequence `links`/`link`/`details`, and
    `accTitle`/`accDescr` still parse but are not kept or drawn;
  - image nodes keep their room but draw no image, since usvg loads no URLs.
- `mermaid::render` takes a `mermaid::Theme` (opaque colors, chart series,
  font, stroke width, corner radius) and emits concrete values, so the SVG has
  no CSS variables or `color-mix()` and rasterizes with usvg as is. The preview
  fills the theme from the editor theme (colors flattened onto the editor
  background, accents as chart series).
- Layout measures text as monospace (0.6 em per glyph), so diagrams use
  `mermaid_font_family`, defaulting to the buffer font, with common system
  monospace fonts as fallbacks. Because every label is already monospace,
  `<code>`, `<kbd>`, `<samp>`, and `<tt>` only drop their tags (Pele switches
  them to a monospace font).
- Label markup (ported from Pele 1.9): bold/italic (`b strong i em cite dfn
  var`), underline (`u ins`), and `<sub>`/`<sup>` (baseline shift, smaller
  text) are styled, and HTML styling carries across line breaks; other HTML
  tags are dropped, while tag-shaped text that is not HTML (`List<T>`) stays
  as text. Wrapping breaks at spaces, between Chinese/Japanese characters,
  at zero-width spaces, and at soft hyphens (shown as `-` only at a break);
  a label of several lines uses a 1.3 line height.
- Sankey links are filled with source-to-target gradients by default; since
  each SVG is rasterized alone, the fixed gradient ids cannot clash.
- Diagrams render at their natural size times the fence scale
  (```` ```mermaid 150 ````); wider ones scroll horizontally. There is no
  per-diagram zoom (ctrl/cmd+scroll was removed). Diagrams follow the
  preview's temporary font zoom (cmd +/-/0): they scale by current preview
  font size ÷ configured UI font size, re-rendered (not stretched) on change,
  so at the configured size they are 1:1. Theme and zoom changes keep showing the
  old raster until the new one is ready.
