# First implementation report

Date: 2026-09-10. Status: working patch viewer. **Milestone 1 remains in progress.**

This historical report describes the first visual slice, before the disk-backed implementation.
The [large-diff progress report](LARGE_DIFF_PROGRESS.md) owns the current status and supersedes the loading/storage limits recorded here.

## What exists

The release executable opens patch files and piped patches in the current terminal.
It displays a continuous file stream, with split/unified layouts, dual line numbers, and file/hunk headers.
Dark/light themes use attributed Pierre assets, composed for opaque terminal cells.
Changed rows and changed words have separate background layers. Syntax colors come from native grammars.

Controls include line/page movement, wheel scrolling, start/end jumps, file/hunk jumps, horizontal scrolling, wrapping, and theme changes.
Layout changes retain the active source line where that line exists on either side.
Small-window recovery preserves the last usable layout and reading position.

Source bytes remain separate from display text. Escape sequences, invalid UTF-8, and directional controls receive visible literal representations.
Missing patch context receives a gap label. Combined merge diffs receive an unsupported-input error.
Malformed hunk counts, overlapping ranges, and out-of-order ranges receive errors.

## Development evidence

Eight integration tests run real processes. No mocks replace the parser, highlighter, executable, terminal I/O, or signal handling.

| Test | Evidence it supplies |
| --- | --- |
| Source reconstruction | Git generates patches from independent before/after bytes. The executable reconstructs both sides and their line numbers. |
| Invalid/non-terminal input | Malformed patches, combined diffs, overlapping hunks, and redirected interactive stdout fail without control-sequence output. |
| Interactive reading | Terminal cells change after layout, theme, wheel, wrap, resize, and start/end input. Inline emphasis has a distinct background. |
| Unsafe source | Input-controlled OSC 52 remains literal. A final bare carriage return remains visible. Ctrl-C restores the terminal. |
| Blocked producer | A real FIFO remains without a producer. The loading screen accepts quit without producer EOF. |
| Large input | An approximately 16 MiB patch contains 40,000 deletions and 40,000 additions. Totals, first/end locations, layout retention, and quit match expectations. |
| Piped input/signals | A real pipeline supplies stdin while keyboard input remains usable. SIGTERM and SIGHUP return 143 and 129. |
| Long-line fallback | A skipped 10,001-character line invalidates later syntax state. Wrapped empty panes contain no leftover gutters or source text. |

Every interactive test checks alternate-screen exit, cursor visibility, mouse-mode reset, and the original terminal-mode fields.
The terminal parser accepts complete synchronized frame updates, not a partially emitted header.
The test deadlines detect hangs. They are not human-perception performance measurements.

Source reconstruction covers uneven replacements, empty input, tabs, CRLF, missing final newlines, invalid bytes, combining marks, CJK, and emoji.
That result establishes source-model fidelity for those cases. It does not establish displayed column widths for every Unicode sequence.
The large-input test checks totals and navigation sentinels, not every large-input byte.

## Appearance evidence and its limit

`evidence/first-slice/` contains dark split, dark unified, and light unified PTY captures.
Each capture includes ANSI cells and an SVG preview. Local PNG conversions make the previews convenient to inspect.
The SVG uses synthetic 10×20 pixel cells with Menlo text. It does not reproduce Ghostty font shaping or compositing.
The diagnostic previews were inspected for readable code, split geometry, row backgrounds, and inline emphasis. They do not establish parity with Pierre.

The computer-use tool denied access to Ghostty in this session. No alternate screen-control method was used.
No viewer windows, tabs, screenshots, or screen recordings were created.
Actual Ghostty appearance and movement remain blocked for automated inspection. A manual run uses the command in the README.

## Failures found and corrected

- Inherited `NO_COLOR` suppressed colors in the test process. Color tests now remove it only from their child environment.
- Early screen checks could accept partly drawn frames. The viewer now brackets updates, and tests wait for the closing marker.
- Resize and wrap changes could move the reading anchor. The app now retains source identity and visible position.
- A final bare carriage return disappeared. Only a carriage return before a newline is treated as CRLF.
- Blank continuation panes could retain gutters. Background clearing now clears cell content and styling together.
- Skipped long lines could invalidate later syntax colors. Affected lines use plain styling until the next hunk.
- Piped input failed with the default Crossterm backend on this host. The built-in `use-dev-tty` backend passes the pipeline check.
- Wheel events moved only the cursor initially. They now scroll the viewport. Ignored mouse motion does not trigger redraws.
- Time-limited inline comparison could vary between runs. Inline comparison now uses deterministic input-size limits instead.
- Initial Lab composition used the wrong reference white. Composition now adapts sRGB D65 to CSS Lab D50 and back.

The self-review pass informed these corrections. Documentation uses the pragmatic ASD-STE100 method to separate facts, procedures, and incomplete work.

## Known limits

1. The input is read, parsed, and indexed in full before the first diff appears. Per-file progressive loading is not implemented.
2. Source bytes and both row indexes stay resident. This is not an out-of-core viewer.
3. The style cache uses a 64 MiB estimated payload budget. This is not a total-memory cap, and excludes allocator/map overhead.
4. Highlighting restarts at the hunk start for each requested region. Parser checkpoints are not implemented.
5. A long jump within a huge hunk can display plain source before syntax colors arrive. Missing preceding patch context can affect syntax classification.
6. Extremely long wrapped lines still require whole-line column scans. Their worst-case input latency is not established.
7. Native grammar scopes and word comparison can differ from Shiki/Pierre. Equal theme assets do not prove equal highlighting.
8. Initial theme conversion uses token foregrounds. Token font styles and precise multiline language equivalence remain open.
9. The pointer uses reported wheel events, not pixel-level trackpad motion. Actual trackpad behavior remains untested.
10. Hunk shortcuts currently target headers. The final spec calls for a more precise changed-line destination.
11. Search, selection/copy, context expansion, file picker, persistent settings, and direct Git/two-file inputs are not implemented.
12. A malformed patch fails as a whole. Earlier files do not remain available after a later parse failure.
13. Terminal checks start in a shell-like state with a visible cursor. Arbitrary preexisting mouse/cursor states are not covered.
14. Panic and SIGKILL recovery, allocation failure, and native-library faults are not established by this pass.

No Linux changeset, 100 MiB fixture, sustained traversal, memory-profile pass, or terminal compatibility pass occurred.
No measured display latency, performance ranking, scrolling-speed claim, or Pierre-equivalence claim follows from this work.

## Reproduction record

- Host: macOS 26.3, build 25D2125, Apple Silicon `Mac17,6`, 48 GiB memory.
- Compiler: Rust 1.96.0, `ac68faa20c58cbccd01ee7208bf3b6e93a7d7f96`, target `aarch64-apple-darwin`.
- Dependencies: local `Cargo.lock`, used with `--locked`. This directory has no Git commit yet.
- Test terminal: native Unix PTY, `TERM=xterm-256color`, `COLORTERM=truecolor`, no `NO_COLOR` in child processes.
- Cell checks: 160×50, 80×24, 30×8, and 100×30. The small size exercises recovery, not normal reading.
- GUI terminal version, actual font, color profile, and display timing: not recorded because GUI acceptance did not run.
- Source, lockfile, sample, theme, executable, and preview hashes: `evidence/first-slice/SHA256SUMS`.
- Final command outcomes: `evidence/first-slice/CHECKS.txt`.

The sample spans Rust, C, TSX, Python, and JSON. It is hand-authored and passes the Git structural count check.
Its full independent before/after visual reference set still needs preparation for the Pierre comparison.

## Next work, in order

1. Inspect the actual viewer in Ghostty. Compare the same sample in pinned Pierre, in both layouts and themes.
2. Correct visual and source-anchor differences before adding controls. Add focused process regressions for observed failures.
3. Add full large-source reconstruction, then profile distant navigation and memory before choosing the smallest loading/checkpoint changes.

The full [acceptance checklist](../design/ACCEPTANCE.md) remains the release gate. This report does not weaken it.
DiffGrid, its candidates, and its benchmark artifacts remain outside this implementation.
