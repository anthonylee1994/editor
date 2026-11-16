# File finder

- History (recently opened files, the current file, and selected matches)
  only lists paths that are still files. They are checked when the finder
  opens and again on every worktree update while it is open, so a file deleted
  meanwhile drops out without reopening the finder. Paths outside the worktree
  are checked on disk.
- Path ranking (`fuzzy`) adds a small bonus for matches near the end of
  the path, so a path that repeats the query (`x/x.py` for `x`) ranks above one
  matching only once (`y/x.py`).
- All fuzzy matching (pickers, completions, outline, file finder) goes through
  one `fuzzy` crate built on `nucleo-matcher`; there is no second in-house
  matcher. Consequences: whitespace splits the query into terms matched
  independently in any order; smart case only downranks a case mismatch (0.9
  per character), never drops it; the length penalty is a mild tie-break; and
  accents are ignored (`cafe` matches `café`).
- Pickers are a plain results list: there is no preview pane (the
  `picker_preview` crate is gone), no Actions… footer menu, and no drag
  resizing. A picker has a fixed width and a max height it shrinks below to
  fit its content.
- The file finder only opens files (`enter`, `OpenWithoutDismiss`, and
  multi-select); it has no open-in-split actions. The text finder keeps its
  split keys (`cmd-h/j/k/l`).
