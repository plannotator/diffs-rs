# Shared reading-position fixture

This fixture supplies the same input to Pierre and the native viewer.
It contains ordinary edits, a three-to-one replacement, unequal long lines, literal tabs, Japanese text, combining accents, and emoji.

- [Old source](old/review.ts) and [new source](new/review.ts) are the independent source expectations.
- [Patch input](review.patch) is the Git-generated comparison with all context included.
- `OLD_ANCHOR`, `NEW_ANCHOR`, and `SHARED_ANCHOR` identify positions for inspection. They are not viewer commands.

The process suite regenerates the patch with Git and compares it byte-for-byte.
It also reconstructs both sources through the real executable and compares their exact bytes.
The geometry reference uses the retained source text, not the viewer's rendering functions.

## Open in Ghostty

Run this command in an existing Ghostty window:

```sh
env -u NO_COLOR /Users/ramos/oss/native-diff/target/release/native-diff patch /Users/ramos/oss/native-diff/fixtures/position/review.patch
```

1. Start with a wide window of approximately 160 columns.
2. Press `w` to enable wrapping.
3. Click `NEW_ANCHOR` in the right pane.
4. Narrow the window below 120 columns, then widen it again.
5. Press `u` twice. Press `w` twice.
6. Shrink below 40 columns or 10 rows, then restore the window.
7. Repeat with `OLD_ANCHOR` and both sides of `SHARED_ANCHOR`.
8. Press `q` to quit.

Expected: the same source, side, and text position remain active. With enough preceding content and space, the original screen row returns.
Wrapping can divide an anchor label across rows. The clicked character, not the whole label, defines the retained text position.
Keyboard movement chooses a new position. A later resize must not restore an older selection.

With the anchored text visible, disabling wrapping retains the horizontal offset. Otherwise, the offset changes to reveal that text.

## Use the same input in Pierre

Use the pinned Pierre checkout at `0a7a733078f093577cd21ca65d2508806a22238a`.
Its `PatchDiff` component accepts the complete `review.patch` text through its `patch` property.
Use the same split/unified mode, theme, available context, four-cell tab width, and wrapping state.
Do not regenerate the comparison from different source versions or use another diff for the screenshot.

This batch prepares the common input. It does not claim a completed Pierre visual comparison.

## Pending-highlight resize check

The process test `rapid_resizes_keep_the_text_anchor_while_highlighting_is_pending` generates a larger TypeScript patch.
It appends this fixture's new long line after 20,000 source lines and observes pending highlighting before the resize sequence.
It checks the final text position and terminal restoration. Its watchdogs do not establish latency percentiles.

To retain that patch and diagnostic cell previews, run:

```sh
NATIVE_DIFF_CAPTURE_DIR=/tmp/pe-verify/native-diff/2026-09-11-1703/cells cargo test --release --locked --offline --test process rapid_resizes_keep_the_text_anchor_while_highlighting_is_pending -- --exact
```

Run that command from `/Users/ramos/oss/native-diff`.
The retained patch is `/tmp/pe-verify/native-diff/2026-09-11-1703/cells/highlight-resize.patch`.
Open it with the same viewer command. Press `G`, `w`, `j`, `j`, then `t` before rapid width changes.
Press `t` to request new highlighting during the check. Press `q` while highlighting is pending to inspect cancellation.

Ghostty automation was denied during this batch. Actual terminal pixels, trackpad behavior, and display timing remain unverified.
Diagnostic ANSI/SVG files represent PTY cells, not Ghostty screenshots or recordings.
