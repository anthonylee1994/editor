# Optimize 01: handoff (2026-11-17)

Goal: lower CPU use and heat while editing, without chasing speed that isn't
needed. Current performance is acceptable; this is about efficiency.

Measurements and method are in `memo/research/performance.md`. This file is
the summary, the open options, and how to pick the work up.

## State

- Branch `neo`, nothing pushed. Commits from this session:
  - `21345149` spinner port (#66896): rotating SVGs no longer re-render views.
  - `46c92b3b` file finder absolute paths (#66227).
  - `e21f7f41`, `1ba78f7d`, `23cfb484` spec and memo updates.
- See "Done" below for what is implemented; the rest of this file is the
  original handoff.

## Done (2026-11-17)

Results are in `memo/research/performance.md` ("Optimize 01 results").

- Option 1, changed: one runnables source per language. LSP languages (Rust)
  use the server only, no syntax query and no timeout; others query visible
  rows plus a screen each side.
- Option 2a, done safely: nested cached views are reused. The forced refresh
  existed because a view inside a reused view keeps stale recorded ranges; a
  per-view frame number now guards that instead.
- Option 3: the status bar is cached with a fixed height, and each pane's
  tab bar (with the window header) is its own cached view.

Remaining: animation frames per keystroke (option 4), language server load
(option 5).

## Findings

Machine: M5 Max MacBook Pro, external 5K display at 100 Hz, release build.
Numbers come from scripted `edctl` input (5–15 events/s), single runs.

1. **Idle is free.** No frames are drawn or presented when nothing changes.
2. **GPU is not the cost.** System GPU power rose only while typing, by about
   240 mW. The Metal renderer rewrite (#66809/#66811) was decided against on
   this basis.
3. **A frame is ~1.5–1.9 ms of main-thread CPU**, about 77% of it build
   (render, layout, prepaint). In a typing frame the editor text area is only
   ~16%; window layout is ~25%, status bar ~10%, project panel ~9%, tab bar
   ~7%.
4. **Typing draws about 9–10 frames per keystroke** (cursor trail and smooth
   scroll animations), each a full re-render from the workspace root down to
   the editor.
5. **Nested cached views never hit.** A cached view that re-renders forces
   all cached views inside it to re-render (`window.refreshing = true` in
   `prepaint_view` / `paint_view`, `crates/gpui/src/view.rs`). Since
   `4968227a` the whole workspace mode is one cached view on every editor
   change's dirty path, so the pane and dock-panel caches under it are dead.
6. **Status bar and tab bar re-render by construction**: the status bar is an
   uncached child of the workspace root; the tab bar is rendered by the pane.
7. **The runnables query is the largest editor-side cost found.** With
   rust-analyzer running, typing in a 3.9k-line Rust file used 44.7% of a core
   (10% in a one-file project without it). About 84% of background-thread CPU
   was `Editor::refresh_runnables` → `SyntaxMapMatches` →
   `ts_query_cursor__advance`. It runs after every reparse (50 ms debounce)
   over the whole buffer for single-file editors; roughly 40 ms of CPU per
   edit in that file (approximate).
8. **The language server itself is the main heat source.** System CPU power
   was ~10.8 W while rust-analyzer ran `cargo clippy`, against 1.0–1.7 W
   otherwise. One sample; outside the editor's code.

## Options

Ordered by expected payoff per risk. Each should be measured before and
after (see below).

### 1. Runnables: stop re-querying the whole buffer (recommended first)

`crates/editor/src/runnables.rs`, `refresh_runnables`.

- a. Query only the visible rows plus a margin, as the multibuffer branch
  already does, and refresh when the visible range changes. Smallest change
  for the largest saving. Trade-off: runnables outside the viewport aren't
  known until scrolled to; check what else reads them (task spawning from
  the command palette, `lsp_tasks` merge).
- b. Lengthen the debounce (e.g. 50 ms → 300–500 ms). One-line change, cuts
  work while typing continuously, does nothing for a single edit, and gutter
  markers lag.
- c. Re-query only edited ranges and keep the rest. Most efficient, most code.

Not yet checked: why the one-file project showed no runnables cost, and
whether the query itself (`runnables.scm` for Rust) is unusually slow.

### 2. Make the pane and panel caches work again

- a. Remove the forced nested refresh in `view.rs` (three lines in
  `prepaint_view` and `paint_view`). Measured on the one-file project:
  instructions per frame 9.85 M → 8.60 M (−13%), frame time about −2%.
  Expected to help more with a populated project panel, split panes or a
  terminal panel; not measured. Risk: upstream GPUI has the same forced
  refresh and its reason isn't in the fork's history, so this is a core
  rendering change. No visual problems were seen in a short run, but it was
  not tested beyond typing and scrolling.
- b. Stop caching the mode view (`workspace.rs`, the
  `mode_view(...).cached(...)` call). No GPUI change. Gives up reusing the
  mode's layout when only shared chrome (status bar, notifications, modals)
  redraws, which `spec/ui-layout.md` currently states as a decision, so the
  spec would need updating.

### 3. Keep the status bar and tab bar out of editor frames

- Status bar: wrap it in `cached(...)` in `Workspace::render`. Needs a
  definite height in the cached style, since cached views aren't measured
  from content. About 10% of a typing frame.
- Tab bar: would need to be its own view inside the pane to be cacheable.
  About 7% of a typing frame; more work than the status bar.

### 4. Fewer or cheaper animation frames per keystroke

Each cursor-trail or smooth-scroll frame re-renders the path to the editor.
Either shorten the animations (`gpui::motion`), or make their frames
paint-only so they skip render and layout. The second is a larger design
change; only worth it if 1–3 leave typing cost noticeable.

### 5. Language server load (outside the editor)

How often the check-on-save runs, clippy versus plain check, and how many
jobs it gets. Confirm with a longer measurement first; this is configuration
rather than code.

### Not worth pursuing on this evidence

GPU-side rendering work, idle cost, spinners, text shaping (the line layout
cache already hits).

## How to measure

Follow `spec/automation.md`. The scratch scripts used in this session were
temporary; the recipe is:

1. Build release, then `edctl launch --fresh <project> <file>`; run
   `edctl wait-lsp` if a language server is involved. Bring the window to the
   front: an occluded window presents no frames and input calls time out.
2. Workloads, each bracketed by `edctl perf-reset` and `edctl perf`:
   - typing: 30 × `edctl type x`, then 30 × `edctl keys backspace`, twice;
   - scrolling: 40 × `edctl scroll --id editor --dy 120`, then 40 × `--dy -120`.
3. Compare **instructions per frame** and draw/build/paint means across three
   runs; they were stable to about 1%. Process CPU and frame counts vary with
   animation timing.
4. For attribution: `sample <pid> 10 1 -file out.txt` during the workload.
   Real frames are under `CA::Display::DisplayLink::dispatch_items`; automation
   overhead is under `control_server` (about 8% of main-thread samples, and
   it forces an extra draw per event). Background work is on the
   `com.apple.root.default-qos` queues. Symbols are Rust v0-mangled.
5. Use two projects: a one-file directory (no language-server activity) and a
   directory with enough Rust files for rust-analyzer to attach. The runnables
   cost only appears in the second. Don't type into files in this repository;
   use copies.
6. Power: `sudo powermetrics --samplers cpu_power,gpu_power,tasks -i 1000 -n 10`.
   System-wide only (per-process GPU time read 0), and noisy with other apps
   running.

## Caveats on the numbers

- Single runs on one machine, with other apps active.
- Scripted input is sparser than real typing or trackpad scrolling; per-frame
  costs are reliable, sustained-rate totals are not.
- The scroll profile had too few samples to break down; the frame breakdown
  is for typing only.
- Automation was enabled for every measurement.
