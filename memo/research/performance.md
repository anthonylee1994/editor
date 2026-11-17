# CPU / GPU usage snapshot (2026-11-17)

One-off measurement to decide whether GPU-side renderer work (upstream
#66809/#66811) is worth it. Short version: **the editor's cost is CPU-side
frame building; the GPU share is small.** Idle costs nothing.

## Setup

- MacBook Pro `Mac17,6`, Apple M5 Max, macOS 27.2.1. Release build of the
  working tree after the #66896 and #66227 ports.
- `edctl launch --fresh`, one window, full screen size on the external 5K
  display (2560×1410 logical, 100 Hz) unless noted.
- Per-frame numbers: `edctl perf-reset` / `perf` (the app's own counters).
- Power: `sudo powermetrics --samplers cpu_power,gpu_power,tasks
  --show-process-gpu -i 1000 -n 10`, averaged over the 10 samples.
- Input was scripted with `edctl`, one event per call.

## Results

| Scenario | Frames drawn / presented | Draw per frame (build + paint + present) | `ed` CPU | System GPU power / active |
|---|---|---|---|---|
| Idle, focused editor | 4 / 4 in 10 s (cursor blink) | 1.9 ms | 0.4% of a core | 53 mW / 8% |
| Idle, editor not focused | 0 / 0 in 10 s | – | 0.1% | (background only) |
| Scroll `big.rs` (3.9k lines) | 32 / 32 in 14.5 s | 1.84 ms (1.44 + 0.26 + 0.13) | 2.4%, main thread 1.5% | 59 mW / 8% |
| Scroll with select-all | 18 / 17 in 9.4 s | 1.81 ms (1.40 + 0.28 + 0.20) | 2.5%, main thread 1.5% | 54 mW / 7% |
| Typing and deleting | 835 / 568 in 24 s (~35 fps) | 1.67 ms (1.29 + 0.26 + 0.12) | 10.1%, main thread 7.9% | 293 mW / 16% |
| LSP spinner in status bar | 23 / 59 in 10 s | spinner tick: 0.08 ms present, no draw | main thread 0.6% | 344 mW / 16% (rust-analyzer running) |

Instructions per drawn frame were 10–40 M across scenarios. A denser scroll
run without power sampling (79 frames in 5.4 s) cost the same per frame:
1.76 ms, 14.5 M instructions.

## Reading

- **Idle is free.** No frames are drawn or presented when nothing changes;
  a focused editor draws only the cursor blink.
- **A frame costs about 1.7–1.9 ms of main-thread CPU**, and ~77% of that is
  build (render, layout, prepaint). Paint is ~15%; handing the scene to Metal
  (`present`) is 6–10%. Extrapolated to a continuous 100 fps that is roughly
  18% of one core; this is an extrapolation, not a measurement.
- **GPU power is small.** The only scenario that moved system GPU power
  clearly above the background was typing, at about +240 mW for ~24
  presents/s. Scrolling at the scripted rate was indistinguishable from idle.
- **A visible selection** (paths, which force the extra render pass that
  #66809 removes) raised `present` from ~0.10–0.13 ms to ~0.20 ms per frame on
  the CPU side. Any GPU-side difference was below the noise at this frame rate.
- **Spinners** now cost one ~0.08 ms present ten times a second and no view
  re-render. The 23 draws in that row came from the status text changing.

So GPU-side work would save at most a few hundred mW while the window is
actively redrawing, and nothing when idle. CPU-side frame building is the
larger lever if rendering cost ever matters.

## Caveats

- Single run, 10 s power samples (covering only the first 10 s of the longer
  scroll and typing runs), with other apps (including a second display
  and the desktop session driving the test) running. System CPU power was
  1.0–1.7 W in every scenario including idle, almost all of it other
  processes, so it is not reported per scenario. The idle GPU baseline itself
  moved between 53 and 240 mW across runs.
- `powermetrics` reported 0 GPU ms/s for `ed` (and WindowServer) in every
  sample, so GPU cost could only be read system-wide.
- Scripted input gives 15–35 fps, not display-rate scrolling. Sustained
  trackpad scrolling at 100–120 Hz was not measured; the per-frame costs are
  the reliable numbers.
- Automation was enabled (`ED_CONTROL`), which records painted elements and
  handles each `edctl` request in-process; process CPU includes that.
- The spinner row was measured in this repository with rust-analyzer running
  `cargo clippy`, so its system power is dominated by that, not the editor.
- No before/after comparison for the spinner port was taken.

## Where a frame's CPU goes (sampling profile)

`sample <pid> 10 1` (1 kHz, main thread) on the same release build while
`edctl` typed and deleted characters in a 3.9k-line Rust file, with the
project panel open. Stacks separate real display-link frames from work
queued by the automation server, so the split below is for real frames only.

Main thread over 10 s: 90.3% idle, 6.0% in display-link frames, 3.2% in
queued tasks. Of the queued tasks, about 90% was the automation server
(including the extra draw it forces after each event), so the "main-thread
work outside drawing" seen in the `perf` numbers above is automation
overhead, not editor work.

Inside display-link frames (392 samples):

| Share | Where |
|---|---|
| 79% | `Window::draw` (everything below except Metal and drawable) |
| 25% | taffy layout of the window's element tree |
| 16% | the editor element (layout, prepaint and paint of the text area) |
| 10% | status bar |
| 9% | project panel |
| 7% | tab bar |
| 7% | text shaping |
| 4% | `Workspace::render` |
| 9% | Metal encode (`MetalRenderer::draw`) |
| 5% | waiting for the next drawable |
| 4% | freeing the frame's element arena |

Rows overlap where one area calls another (shaping sits inside the editor
and panels), so they don't sum to 100%.

Reading:

- **The text area is a minority of a typing frame (~16%).** Laying out and
  re-rendering the surrounding chrome (layout, status bar, project panel, tab
  bar, workspace) is around half.
- Every frame of a keystroke's animation (cursor trail, smooth scroll)
  re-renders the path from the workspace root to the editor, and with it the
  uncached siblings on that path: the status bar and the pane's tab bar.
  Only dock panels, panes and the mode view are wrapped in `AnyView::cached`.
- The project panel is cached in its dock but still showed up in 9% of frame
  samples, so something invalidates it while typing. Not investigated.
- The most promising lever is therefore making animation and keystroke
  frames skip chrome that didn't change, not speeding up the editor element
  or the GPU side.

Caveats: about 390 frame samples, one workload. The scroll profile had too
few frame samples (25) to break down. Symbols were demangled approximately.
