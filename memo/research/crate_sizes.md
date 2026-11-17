# Binary size by crate

Question: what takes up space in the release `ed` binary?

Snapshot: 17 Nov 2026 (commit `2c4c2e28a1`), arm64 macOS, release profile
(`lto = "thin"`, `codegen-units = 1`, `panic = "abort"`, `strip = true`).
Per-crate numbers from `cargo bloat --release -p ed --crates`, section sizes
from `size -m target/release/ed`.

`cargo bloat` builds the binary unstripped (72.8 MiB) and can leave that in
`target/release/ed`. Run `cargo build --release -p ed` afterwards to make sure
the shipped binary is back.

## Whole binary

The shipped binary is 52.4 MiB.

| Part | Size | What it is |
|---|---|---|
| `__TEXT,__text` | 38.3 MiB | machine code (broken down by crate below) |
| `__TEXT,__const` | 10.2 MiB | read-only data: tree-sitter parse tables, embedded assets (icons, themes, spell dictionary), string literals |
| `__DATA_CONST` | 2.0 MiB | vtables, relocated constants |
| `__LINKEDIT` | 1.4 MiB | dyld binding info, code signature (symbol table is stripped; unstripped it is 21.8 MiB) |
| unwind / exception tables | 0.2 MiB | nearly gone with `panic = "abort"` |
| `__cstring` | 0.2 MiB | C string literals |

The binary is memory-mapped, so pages that are never touched cost no RAM. The
size mostly matters for disk usage and cold-start page-ins, not for memory
usage while running.

History:

| Snapshot | Shipped binary | `.text` |
|---|---|---|
| early Nov 2026, unstripped, unwinding | 117.6 MiB | 59.3 MiB |
| + `panic = "abort"`, strip fixed | 64.9 MiB | 49.4 MiB |
| 17 Nov 2026, after trimming | 52.4 MiB | 38.3 MiB |

## `.text` by crate

| Crate | Size | % of `.text` | Notes |
|---|---|---|---|
| gpui | 8.7 MiB | 22.8% | UI framework, generic over every view/element type |
| std | 5.4 MiB | 14.0% | includes monomorphized std generics (`Vec`, iterators, `HashMap`) instantiated by every crate |
| editor | 2.4 MiB | 6.3% | |
| project | 1.6 MiB | 4.1% | |
| mermaid | 1.5 MiB | 4.0% | largest optional feature: mermaid diagrams in markdown preview |
| workspace | 1.4 MiB | 3.5% | |
| sum_tree | 1.2 MiB | 3.0% | generic B-tree, instantiated per item type |
| [Unknown] | 1.0 MiB | 2.7% | mostly C code: tree-sitter grammars, zstd |
| git_ui | 897 KiB | 2.3% | |
| async_task | 579 KiB | 1.5% | one monomorphized task per spawned future type |
| serde_core | 484 KiB | 1.2% | |
| scheduler | 443 KiB | 1.1% | |
| ui | 421 KiB | 1.1% | |
| multi_buffer | 416 KiB | 1.1% | |
| regex_automata | 382 KiB | 1.0% | |
| harfrust | 373 KiB | 1.0% | text shaping for SVG text, pulled in by gpui through usvg |
| language | 373 KiB | 1.0% | |
| lsp | 359 KiB | 0.9% | |
| picker | 358 KiB | 0.9% | generic over every picker delegate |
| serde_json | 357 KiB | 0.9% | |
| usvg | 333 KiB | 0.9% | SVG parsing |
| image_webp | 333 KiB | 0.9% | image viewer, WebP decoding |
| everything else | ~9.1 MiB | ~24% | ~240 crates, each under 0.7% |

Other crates that only exist for one feature: `spellbook` 216 KiB (spell
check), `zune_jpeg` 209 KiB + `image` 178 KiB (image viewer), `clap_builder`
162 KiB (CLI args in `ed` and `control`), `fancy_regex` 154 KiB (used only by
`project`), `markdown_preview` 99 KiB, `syntect` 83 KiB (fallback
highlighting), `alacritty_terminal` 69 KiB.

Gone since the first snapshot: `prost` (596 KiB, remote protocol),
`tabular_data_preview` (138 KiB), sqlite (was part of [Unknown]).

Biggest drops since the first snapshot: std −5.1 MiB, gpui −3.7 MiB, [Unknown]
−1.4 MiB, project −1.1 MiB, mermaid −1.0 MiB, picker −0.7 MiB, workspace
−0.6 MiB. Much of the std / gpui drop is `panic = "abort"` removing landing
pads, the rest is removed code and fewer generic instantiations.

## Takeaways

- There is no single big dependency to cut. About 37% of the code is gpui
  plus std generics, which is the cost of using gpui at all.
- `mermaid` (a local crate used only by `markdown_preview`) is still the
  biggest feature you could drop (1.5 MiB) if you don't use mermaid diagrams.
- SVG support costs about 1 MiB across `usvg`, `harfrust`, `resvg`,
  `tiny_skia`; it is needed for icons, so not optional.
- Generic-heavy crates (`gpui`, `sum_tree`, `picker`, `async_task`) grow with
  every new view or task type. That's expected and doesn't hurt runtime.
- Read-only data (10.2 MiB) barely moved while code shrank by a third, so it
  is now a fifth of the binary. Tree-sitter parse tables are the bulk of it.
- cargo-bloat attributes symbols by name, so numbers are approximate. With LTO,
  inlined code is counted under the caller's crate.
