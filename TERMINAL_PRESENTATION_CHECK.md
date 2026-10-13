# Actual terminal presentation check

Status: prepared, not captured. Executable binding updated 2026-09-20.
The serial qualification and bounded residency investigation are complete.
Do not run a viewer, recorder, or browser capture alongside performance workloads.

## Existing reference

Use the preserved Pierre reference at
`evidence/terminal-presentation-2026-09-14/pierre/`, not a new browser implementation.
It is a byte-for-byte copy of selected original artifacts from
`/tmp/pe-verify/native-diff/2026-09-11-1800/`. The original checksum manifest passes.
The built reference and its source remain in `pierre-reference-bundle.tar.gz`;
no Ghostty media has been captured in this directory yet.
Its `RUNBOOK.md`, `provenance.json`, `browser-checks.json`, and five original PNGs
describe the source and checked settings. The reference uses `PatchDiff` 1.4.1
from commit `0a7a733078f093577cd21ca65d2508806a22238a`.

The current `fixtures/position/review.patch` and the reference's input both hash to
`42f417cac8aae1b7f12931b8f07639d4f724240736e9062dac008d625cd34800`.
Recheck them at capture time. Bind the native executable hash separately. The old
native SVG in the original temporary directory is not an actual terminal
screenshot and was not copied into the preserved reference directory.

The kept normal executable hashes to
`1b58a20778dc5ca0eedcaec0b3d7710cc19c8cc9151d58b3180e309fe6d73000`.
It contains the kept line-local search-buffer change and shared replacement boundary, not the deferred shared-prefix candidate.
The [baseline report](evidence/clean-baseline-v1/REPORT.md) records its checks and rollback copy.
The root working source now matches the accepted implementation. A normal build no longer includes the deferred experiment.
Use the installed executable and record its hash at capture time.

## One visible window

Use one existing Ghostty window. No Computer Use, accessibility automation,
terminal preference changes, or automatic window/tab creation is needed.
Record the Ghostty version, font and size, initial terminal dimensions, display
scale, and executable hash. Unknown settings stay marked unknown.

The user opens the shared patch with explicit split and dark settings, then presses
`w` once to wrap. A mouse click on the long added line establishes the reading
position. This is not text selection: no dragging, copying, or character-selection
mode is required.

Run this in one ordinary Ghostty window, with no multiplexer:

```sh
sh -c 'printf "\033]0;native-diff-review\007"; exec env -u NO_COLOR /Users/ramos/oss/native-diff/target/release/native-diff --layout split --theme dark patch /Users/ramos/oss/native-diff/fixtures/position/review.patch'
```

The short shell command sets an exact window title before opening the real viewer.
It does not open tabs, install anything, or put recorder logs in the application.
After pressing `w`, leave the viewer open and report that it is ready. Capture
starts only after the window is available; no automated interaction is implied.

Keep the required user action small:

1. Show split dark mode with wrapping, then unified dark mode with wrapping.
2. Show light mode in both layouts. Return to split dark mode.
3. Narrow and widen the window slowly, then rapidly. Toggle `u` twice and `w`
   twice without a new navigation command. Check that the same source position
   remains visible and active. Quit with `q`.

Use the existing exact-title window recorder at
`/Users/ramos/oss/diffs/diffgrid/rehearsal/live-capture/record-window` if capture
permission is available. It records only one uniquely visible Ghostty window,
without audio or cursor capture. It needs an initial window of at least 1000 by
700 points. The native viewer itself does not require that recording size.

Keep recorder logs outside the viewer. Wait for its first complete-frame `READY`
signal before the user starts the sequence. Do not repeatedly open windows or
change permissions if capture fails. Record the blocker and ask for original
window screenshots instead.

## What to inspect

Compare original stills or clearly identified frames with the matching Pierre
state. Keep the original media, settings, timestamps, and hashes together.

- Source text and numbers: no missing, repeated, reordered, or invented content.
- Unequal replacements: blank cells do not become fictitious source lines; the
  following shared context aligns correctly.
- Long lines: tabs, Japanese text, combining accents, and emoji remain legible.
  Wrapping must not lose characters or overwrite gutters and adjacent rows.
- Change styling: whole-line changes and changed words remain distinguishable.
  Compare both themes, syntax roles, gutters, and unchanged-row backgrounds.
- Interaction: no stale rows, displaced reading position, broken redraw, or dirty
  terminal after exit. Inspect capture completeness after each width change.

Different wrap breaks from different font/cell metrics are expected. Native sign
columns, continuation markers, two unified number columns, and neutral empty-side
fill are deliberate terminal choices. Missing content is not an acceptable
terminal adaptation.

The retained reference already identifies one palette difference: native unchanged
dark rows use `#1b1b1b`; Pierre uses `#0a0a0a`. Recheck and decide that difference
explicitly. Do not describe palette parity as complete while it remains unresolved.

## Limits and next check

The recorder requests 30 frames per second and uses H.264. Its images support a
qualitative presentation review, not exact RGB equality, input latency, or a
display-frame pacing claim. Static screenshots cannot establish smooth scrolling.
Terminal-cell/source tests remain the exact correctness evidence.

If the small-content check is usable, make one separate short recording with the
existing pending-highlight resize fixture. Record whether highlighting was visibly
pending during the width changes; do not infer overlap merely from pressing `t`.
The process test already checks the exact destination. This recording adds actual
display evidence, not a replacement oracle or a new benchmark framework.

Finish with a short pass/defect/not-observed list. A capture blocker leaves actual
terminal acceptance open; it does not invalidate completed process checks or count
as a visual pass.
