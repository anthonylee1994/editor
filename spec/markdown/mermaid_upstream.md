# Mermaid: porting from Pele

`crates/mermaid` is owned here (see `mermaid.md`). Pele
(https://github.com/obsidianmd/pele, TypeScript) is the reference we port
useful changes from, commit by commit. This file tracks where we are.

## Status

- **Synced to Pele `3a2f075` (1.13.0+).** Next port starts from the first
  Pele commit after `3a2f075`: `git log 3a2f075..main -- src`.
- Origin: our crate is a copy of Ondine `0b589ec`, which ported Pele at
  `458322a` (≈ 1.6.0). Everything from there to 1.9.0 was reviewed below.
- Port only what changes our usvg-rasterized output or speed; browser-only
  changes (mount, zoom, ids, CSS-variable plumbing, accessibility, links,
  images, fitting to `maxWidth` / auto direction) are n/a.
- Diagrams removed here (gantt, git, journey, pie, quadrant, sankey, block,
  venn, agentflow, treeview, swimlane) are n/a.

## 458322a → 1.9.0

### Layout (`layout/layered.rs`, `layout/compound.rs`)

Ported:
- `3305c31` single node skips layout; `8de68c6` swap neighbouring nodes where
  that crosses less (transpose).
- `f6393a5` order groups and their border ports from an order-only layout of
  the whole drawing; keep the groups' own order instead when routes cross more
  than 1.5 × flat crossings + 4. Then `13e2be9`, `9bf49d1`, `992c18e` (port
  re-layout only where ports are out of order and the flat order won).
- Route crossing count as drawn: `209ebb8`, `ba26119`, `2f093da`, `2395cb1`,
  `bcf8504`, `5fdde33`.
- Speed, same output: `a61d576`, `7d02d65`, `7737949`, `303aa64`, `7bd4117`,
  `f3e8693`, `8507cff`, `58440fd`.
- Rust shape: Pele's `groups` / `orderOnly` options are a separate
  `layered_with(..)` entry point (`LayeredOptions` stays `Copy`).

N/a: `4c3351b` (Rust's stable `sort_by` already gives the same order),
`4179f3c` (JS typed-array speed), `bdc70f5`, `5b4a7e5` (comments).

### Labels and text (`text/`, `svg/builder.rs`)

Ported: `1b3e033` + `5ecbc1b` CJK wrapping, `f7fc1c5` HTML emphasis across
`<br>`, `ecaa31f` HTML5 entity aliases, `72d4935` decode each entity once,
`c43e075` `<sub>`/`<sup>`, `96369e3` 1.3 line height for multi-line labels,
`c843ac0` ZWSP / soft-hyphen breaks, `0b2ab04` strip only HTML tags,
`d5222c1` `<u>`, `ae77eac` `kbd samp tt var cite dfn ins`.

Adapted: `f6fa57b` `<code>` monospace — labels are already monospace, so
`code`/`kbd`/`samp`/`tt` only drop their tags.

### Diagrams

Ported: `cba20f1` sequence tall self-message, `5c42e53` timeline center line,
sankey `eb02018` labels apart, `6bcb901` Unicode names, `a06891e` +
`a6d453e` link gradients (fixed sequential ids, no `idPrefix` gate),
`cbd50fa` shrink labels; `e440f1a` C4 detect anchoring, `2c81661` C4 column
gap, `02ddfd1` class wraps disconnected groups (removed with host-width fitting),
`23ee8d4` state honours curve, `4d1be9d` quadrant label below title.
Flowchart / YAML parse speed: `2c9de31`, `c90c048`, `cfa9c15`, `2370199`
(scalar early return only).

### Skipped

- `c8a3431`, `fdc8e75`, `18ecad7` `$$` math via host resolver: no math
  renderer; output without a resolver is unchanged. (`fdc8e75` also stopped
  wrapping non-tight sequence titles at `wrapWidth`; not ported.)
- `2d7eda9`, `e95d825` `mermaidType`: no API use.
- `dcf0e96` rounded corners moved from `rx` to style: browser compat only, no
  rect gained rounding. `0915599` root variables: browser-only.
- `4d64461`: our Rust already does it.
- `38103ad` mount, `d2defb6` zoom: browser-only.

### Known differences

- Pele tests use proportional widths; ported tests use our 0.6 em monospace
  thresholds. At 16px, sankey adjacent-column labels can still overlap when
  no free spot exists (monospace is wider).

## 1.9.0 → `3a2f075`

Ported:
- Layout: `44d7112` start ports don't push targets down a rank; `6043f6c` +
  `52a4521` border ports in line with their edge ends (`TWIN`); `bdbad5d`
  bends snap up to 24px.
- Flowchart `205bd71`: a subgraph an edge enters or leaves drops its
  `direction`.
- Sequence: `cc8b825` bare title; `c9e7b75` comments.
- Treemap: `b38b165`, `eaaa8ae`, `1dcddf2`, `bd1bc29`. No per-box corner cap
  (Pele's `min(var(--_r),…)` needs CSS).
- Names starting with `title` (`TITLE_BEFORE_WORD` / `_NAME`, railroad
  `ABNF_TITLE`): `41bb013`, `27fae12` (radar option values see the previous
  token via `Pattern::After`), `84157ee`, `1bd683f`. Bare titles: `0f9655f`,
  `0c74342`. Keep the character before `%%`: `be3eafb`, `639154e`.
- `59879ae` xychart bounds, `bc77180` radar overflow, `318655a` +
  `602957e` invalid XML characters (styles reject, text drops).

Skipped:
- Author-set fill series (contrast label colors, edge casing, full-opacity
  group fills, `ink.ts`): only for `style`/`classDef fill:`; colour parsing and
  path sampling not worth it yet.
- Sequence spare width (`44b0024`, `cc8f4ca`, `bc05979`, `9d11243`): only
  runs only under `maxWidth`, which we dropped.
- `e31312f`, `8a29830` palette: we read no `themeVariables`.
- Monospace-measure commits (`c5851d8`, `141ab36`, `6b01062`), `03fa37f`:
  every label is monospace already. `5ebbb98`: we already wrap by character.
- Error positions (`57bf1d1`, `aaf867a`, `0f8a6a2`, …): the preview shows
  the message only. URL, zoom, tooltip (`a64db96`): not in our output.

## Checking

`cargo test -p mermaid` holds the ported Pele tests (layout, labels,
entities, per-diagram end-to-end in `lib.rs`).
