# Zed `[patch.crates-io]` forks

Question: do we still need the `[patch.crates-io]` git forks in the root
`Cargo.toml`, or can we switch to the latest crates.io release?

Snapshot: Oct 2026, async patches updated 2026-11-16. Answer: `async-process`
patch removed, `async-task` now points at upstream, keep `notify`.

## Summary

| Patch | Fork base | Latest on crates.io | Fork-only changes | Verdict |
|---|---|---|---|---|
| `async-process` | 2.5.0 | 2.5.0 | macOS `Child::adopt_raw_pid` | Removed: no callers left |
| `async-task` | 4.7.1 | 4.7.1 | Waker vtable identity fix | Patched to upstream `41fd335`; delete on 4.7.2 |
| `notify` / `notify-types` | 9.0.0-rc.4 | 9.0.0-rc.6 | 6 macOS FSEvents fixes | Optional, but keep it (correctness) |

## async-process

- Patch removed 2026-11-16; crates.io 2.5.0 is used. The fork only added
  `adopt_raw_pid`, and its one caller (`crates/util/src/command/darwin.rs`) is
  gone.

## async-task

- The fork's fix ("Fix task waker vtable identity") was merged upstream as
  smol-rs/async-task#108 (`41fd335`, 2026-11-14) but is not released yet.
- The patch now points at that upstream commit. Delete it once 4.7.2 is on
  crates.io.

## notify / notify-types

Fork commits that upstream rc.6 doesn't have:

- `1824186` fsevent: stop the run loop via a `CFRunLoopSource` to avoid a
  shutdown race
- `6afa3aa`, `7e98367` fsevents: resume rebuilt streams from the last delivered
  event id. Otherwise events that fire during `watch`/`unwatch` stream rebuilds
  are dropped for every watched path.
- `af91c6c` fsevents: stop setting `kFSEventStreamCreateFlagWatchRoot`
- `8f73206`, `89094ba` add a `FsEventStreamStart` error variant (plus a test)

Upstream rc.6 has related but different work: #1013 (intermittent hang fix),
#1005 (skip rebuilds for covered watches), #975 (coalesce nested watches),
#1021 (ancestor recovery as rescan events). It does not have the event-id
resume or the `WatchRoot` change.

Trial: removing the patch and bumping `crates/fs` to `notify = "9.0.0-rc.6"`
passes `cargo check -p fs`. The API is compatible, but we would lose the fixes
above, which stop file-change events being missed on macOS.

## Next

- `notify`: bump the `rev` once Zed's fork is rebased onto rc.6 or later. Drop
  the patch if upstream merges the event-id resume fix.
- Others: recheck when upstream cuts a new release.
