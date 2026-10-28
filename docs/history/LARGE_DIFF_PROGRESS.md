# Large-diff implementation: disk-backed batch

Date: 2026-09-10. Status: disk-backed source checks and corpus terminal checks pass. Performance acceptance remains open.

## Completed work

The original executable and source snapshot remain in `evidence/large-diff-baseline/`.
The original parse/index diagnostic ran once per fixture, serially, without a GUI terminal.
The safety stop watched process-group RSS at no more than 20 samples per second.
Its 6 GiB threshold was a sampled stop condition, not an OS-enforced memory limit.

An independent Python decoder now defines the supplied-source expectations for all five patches.
It does not import the Rust viewer, libgit2, or the viewer's layout code.
It records file/hunk boundaries, source line numbers, exact bytes, signs, and missing-newline markers.
The reference also identifies file quantiles, the largest file/hunk, and the longest line.
The pinned expectations are in [reference.json](../../fixtures/diffshub/reference.json).

The input worker copies exact patch bytes into a private, anonymous temporary file.
It parses one complete file-patch unit at a time through libgit2.
Each source record uses a 16-byte offset, length, and flags entry instead of a retained byte allocation.
The parser checks each offset against the supplied bytes before publication. Missing-newline markers shorten the indexed payload, not the stored patch.

Completed file indexes reach the UI through a two-slot queue. Earlier content remains readable while later files load.
Files append without moving the reading position. A later parse failure leaves earlier files readable with a persistent incomplete status.
Normal quit after an input failure returns 1. Ordinary completion returns 0, and cancellation restores the terminal.

Repeated display rows and source-to-row locations now use compressed ranges.
The compact index preserves the existing pairing, ordering, gutters, and missing-newline rows.
Source line numbers derive from contiguous source runs. Partner lookup derives from the split projection instead of a separate per-line array.

The source-page cache has a 32 MiB payload limit. The style cache currently has a separate 32 MiB estimated payload limit.
One latest-viewport slot replaces the unbounded request queue. The reply queue holds at most 16 replies and remains cancellable under backpressure.
These limits are not total RSS limits. Complete queue-byte, container, transient, and allocator accounting remains open.

The `inspect --canonical` diagnostic exports source records through the same disk-backed file index as the viewer.
It completes each file before it publishes that file's records. It releases each exported file index before the next callback.
The diagnostic refuses terminal stdout. A nonzero exit invalidates the entire export, including an already emitted prefix.
The `inspect --summary` diagnostic retains the complete compact index and reports its source and projection payload sizes.
The full JSON diagnostic without either flag still uses the resident source model. It is not the interactive storage path.

## Baseline failures and corrections

| Original input | Parse/index result | OS peak RSS |
| --- | --- | ---: |
| Pierre | Completed | 15.3 MiB |
| Ghostty | Completed | 43.1 MiB |
| Node | Rejected an empty-file deletion | 76.6 MiB |
| Bun | Rejected an empty-file addition | 48.0 MiB |
| Linux | Rejected an empty-file deletion | 2,588.9 MiB |

These are original-executable diagnostics, not rendering measurements or acceptance results.
The diagnostic did not establish controlled cache conditions or a five-run distribution.
Elapsed values include the wrapper and polling overhead. They cannot represent first-content or scroll latency.
The original data remains in [parse-before/results.json](../../evidence/large-diff-baseline/parse-before/results.json).

The empty-file correction recognizes Git's explicit empty-blob metadata.
It supplies equivalent file headers to libgit2 without inventing source lines or changing fixture bytes.
Real Git-generated checks cover additions, deletions, modes, spaces, quoted paths, and Unicode paths.
The terminal displays those files as metadata cards.

A FIFO regression exposed an incorrect stability check on stream metadata.
Only regular files now receive the fixed-input modification check. FIFO writes remain normal stream behavior.
Zero-based nonempty source ranges also receive an explicit error.

## Evidence

| Required patch | Supplied source records compared | Result |
| --- | ---: | --- |
| Pierre | 18,420 | Exact digest match |
| Ghostty | 93,579 | Exact digest match |
| Node | 419,306 | Exact digest match |
| Bun | 1,016,971 | Exact digest match |
| Linux | 19,299,175 | Exact digest match |

These counts exclude patch metadata. Shared context counts once, with both source line numbers in its record.
The comparison covers every supplied source record, not only totals or selected lines.
The [disk-backed source check](../../evidence/large-diff-storage/source-check-final.json) binds the executable, fixture identities, checker, and reference.
The earlier [resident source check](../../evidence/large-diff-baseline/source-check.json) remains intact.
A second reference generation reproduced all five complete reference files byte-for-byte.

Fourteen real-process tests pass, including two opt-in checks that require the preserved executable and downloaded corpus.
The [final process log](../../evidence/large-diff-storage/process-checks-final.txt) records the run. Formatting and Clippy checks pass.
No mock process replaces the executable, parser, producer, or terminal.

The new checks cover these behaviors:

- A real FIFO supplies complete A, blocks before B, then completes B, truncates B, or remains blocked during quit.
- Earlier content remains readable after B fails. No incomplete B source appears.
- After loading completes, replacement of the caller's input file does not change the displayed source.
- Fourteen complete cell/style grids match the preserved pre-storage viewer across scrolling, layouts, themes, wrapping, line numbers, and resize.
- All five pinned DiffsHub bodies reach complete input, exact end anchors, both layouts, resize, return navigation, and clean exit in real PTYs.

The grid comparison checks compatibility with the previous executable, not independent Pierre equivalence.
The corpus terminal check covers selected destinations, not every screen or settled style state.
Neither check establishes Ghostty pixel presentation, latency percentiles, or RSS acceptance.

## Corrections from this batch

The first terminal pass exposed a status error: loading hid the pending-highlight state. The footer now distinguishes both states.
Review exposed an error-screen regression that prevented navigation through earlier files after a later failure. Those files now remain readable.
Unrelated file arrivals no longer cancel visible styling. Valid offscreen replies enter the cache without a repaint.

The corpus terminal check then exposed a polling bottleneck with many small files.
The UI now wakes on completed input instead of waiting through a fixed polling delay per publication.
Progress-only counts repaint at most ten times per second. Source changes and user commands do not use that progress throttle.
The same corpus terminal check passes after this correction.

A follow-up run exposed a zero-timeout limitation in the terminal backend. Keyboard input stopped during blocked input reads.
A dedicated terminal-input thread now supplies events through a one-slot queue. The blocked-producer navigation and cancellation checks pass again.
The [failed intermediate run](../../evidence/large-diff-storage/process-checks.txt) remains available beside the final passing run.

Self-review removed the duplicate canonical exporter and consolidated input stability checks.
The performance review informed publication, cancellation, and error recovery. The documentation method keeps evidence separate from unfulfilled targets.

## Remaining work

### Source endpoint correction, 2026-09-11

The current viewer selects the first or last supplied source record through the existing index.
It focuses the side that contains that source, with the new side preferred for shared context.
Headers, metadata-only files, and newline markers are not endpoints. Empty source indexes leave the reading position unchanged.
Wrapped endpoints start at their first segment. During loading, only completed files supply endpoints.

The regression reproduced the old failure before the correction. Fifteen real-process tests pass after the correction.
The cases include additions, deletions, replacement hunks, leading/trailing metadata, absent final newlines, wrapped lines, and empty input.
FIFO checks cover navigation before EOF, after publication of another file, and after a later input failure.
The compatibility trace now uses file movement instead of the intentionally corrected endpoint commands. Other complete cell/style grids still match.

All five patches also pass three complete frozen endpoint grids: `G`, `g`, then `G`.
This was a correctness check with no retained timing measurements.
The [process log](../../evidence/source-endpoints/process-checks.txt) and [frozen-grid log](../../evidence/source-endpoints/frozen-grid-checks.txt) record the checks.
The baseline tools, trace, original executable, and retained results remain unchanged.
The historical freeze still describes its original source revision, not this corrected source file.

### Pager navigation, 2026-09-11

The agreed navigation scope covers rows, pages, scrolling, source endpoints, and adjacent files. Numeric prefixes are no longer planned for this batch.
`n` / `N` replaces `{` / `}` for file movement. Each command selects the adjacent file header, including metadata-only files.
`N` inside a file selects the previous file, not the current header. Commands stop at available file boundaries without wrapping.
The on-screen help and current specifications describe these controls. Existing hunk, layout, and appearance controls remain unchanged.

All 16 real-process tests pass, including the five pinned large patches. Formatting and Clippy checks also pass.
The navigation checks cover exact row/page destinations, adjacent files in both layouts, file boundaries, help, progressive input, and terminal restoration.
The compatibility check uses the old and new file keys for equivalent actions. Its complete cell/style grids still match.
The [process log](../../evidence/pager-navigation/process-checks.txt) records this correctness run. No new performance results were produced.

The historical measurement tools, trace, freeze manifest, original executable, and results remain unchanged.
The [previous specifications](../../evidence/pager-navigation/spec-before.tar.gz) retain the two specification files with their original frozen hashes.
The current source and specifications intentionally differ from the historical freeze. Changed-viewer timings require a versioned trace with supported inputs.

### Reading-position correction, 2026-09-11

The [shared fixture](../../fixtures/position/README.md) retains old/new TypeScript sources and their Git-generated patch.
It includes ordinary edits, unequal replacements, literal tabs, combining accents, Japanese text, and emoji.
The new real-process regression reproduced wrapped-position drift on the preserved pre-change executable.

The viewer now retains source identity, active side, a byte offset in the immutable display-safe line, and the preferred screen row.
The display offset never substitutes for original source bytes. Source parsing, storage, palettes, pairing, and keyboard bindings remain unchanged.
Mouse selection retains its clicked text position. Keyboard movement clears the previous anchor.
Wrap and line-number changes capture the position before geometry changes. Unwrapping reveals the anchor when the previous horizontal offset cannot.
The self-review also kept unified clicks from silently changing the stored side of shared context.

The focused process checks cover both source sides, shared context, odd/even widths, automatic and explicit layouts, wrapping, and line-number visibility.
They also cover small-window recovery, exact restored screen rows, and movement followed by another resize.
A separate TypeScript case observes pending highlighting before rapid resizes. It checks the final source segment and cancellation cleanup.
All 19 real-process tests pass, including both corpus-dependent checks. Formatting and Clippy checks pass.
These are correctness checks, not accepted latency or memory measurements.

Ghostty control was denied by the computer-use tool. No alternate control route, GUI recording, or Ghostty screenshot was used.
The verification report and diagnostic PTY evidence are in `/tmp/pe-verify/native-diff/2026-09-11-1703/`.
The fixture instructions include a manual Ghostty check. Visual parity, pixel presentation, and real-window resize smoothness remain open.

### Independent rendering contracts, 2026-09-11

Eight new real-process checks use explicit source rows, cell positions, emphasis spans, syntax colors, and metadata expectations.
They reuse the existing PTY harness. They do not use the viewer's pairing algorithm to generate expected rows.
The [contract notes](../../tests/process/README.md) describe the cases, commands, and limits.

The checks reproduced two defects on the saved executable.
Shared context used new-side syntax colors in both split panes when comment or string delimiters changed the syntax state.
Binary notices used the wrong parser flag and appeared as ordinary modifications without textual hunks.

The highlighter now retains old-side tokens when they differ from new-side tokens. The cache estimate includes their allocated capacity.
The model now reads the binary flag from the diff record. Source storage, pairing, controls, palettes, and dependencies remain unchanged.
All 27 process checks pass, including the existing large-corpus checks. All five complete supplied-source digests still match.
The saved pre-change executable fails the new syntax and binary-label regressions.

Evidence is in `/var/folders/xg/f67qq94s1qsbxl_nlh9hd_xc0000gn/T/pe-verify/native-diff/2026-09-11-2051/`.
No new Ghostty capture, performance distribution, or memory acceptance follows from this correctness run.

### Bounded display geometry, 2026-09-11

The application now owns a geometry cache with a 16 MiB payload budget and a 1,024-entry limit.
Entries contain immutable safe text, sparse grapheme boundaries, and wrapped-row boundaries.
The renderer and position recovery share these entries. A retained width does not require another whole-line scan for each visible row.
The cache uses FIFO eviction. Oversized entries remain usable without retention. No dependency, key binding, theme, or syntax rule changed.

Two new real-process checks cover sparse Unicode/tab boundaries and source revisits after both cache limits.
The complete suite passes all 29 checks, including rapid resize during highlighting and the pinned corpus endpoints.
The full supplied-source digests still match all five patches.
The [batch specification](RESPONSIVENESS.md) records the measurement boundary, implementation, and remaining work.

### Resumed syntax replay, 2026-09-11

The worker now retains one movable old/new continuation for a hunk and theme generation.
Forward reading resumes at the next unprocessed source record. A backward request or changed hunk/theme starts a new replay.
Off-screen replay applies syntax-scope operations without generating colors. Requested source ranges select their hunk directly.
There is no cloned snapshot collection, new dependency, extra thread, or new control.

Two new real-process checks cover deep old/new comments and strings, theme changes, continued fallback, and fresh state at hunk boundaries.
All 31 process checks pass. The complete supplied-source digests still match all five patches.
On the fixed 12,000-line diagnostic, median uncached forward styling changes from 1,749.3 ms to 65.8 ms.
Cold and backward styling still take approximately 1.65 seconds. Short-run peak RSS reaches 64.9 MiB, versus 65.3 MiB before.
The [syntax replay report](SYNTAX_REPLAY.md) records all measurement conditions and remaining work.
The retained parser states have opaque, variable-size heaps. This change is not a total-memory guarantee or full performance acceptance.

### Still open

This is not a completed Stage A or a performance-accepted large-diff engine.
The [2026-09-11 baseline report](BASELINE_REPORT.md) adds the frozen content-grid trace and 25 original-viewer trials.

### Five-patch interior and current-build measurement, 2026-09-11

The independent five-patch batch completed all five pinned DiffsHub inputs. Each rehearsal checked 205 unchanged terminal states plus deliberate corruption controls. Five fresh-process latency repetitions and three sustained-memory tours completed for every patch.

Interior emitted-cell checks pass for all five. Four latency series meet their configured guards; Pierre has one 251.8 ms styled distant-seek maximum above the 200 ms limit. Ghostty is within the smaller-patch memory envelope at 123.7 MiB settled RSS; Pierre, Node, Bun, and Linux exceed their applicable memory limits. The [results report](FIVE_PATCH_RESULTS.md) records exact values and evidence paths. No Ghostty pixel or paint-time claim follows from PTY output.
Pierre and Ghostty patches opened in all five trials. Node, Bun, and Linux retained their original parser failures.
The baseline contains 800 correct scroll observations and explicitly records the incorrect start seeks and unsupported structural setup.
Full normalized metadata, sequential styling expectations, long-line tours, and steady-memory acceptance remain unresolved.
Local before/after diagnostics now cover long-line width changes and a synthetic deep-hunk seek.
These diagnostics do not establish the complete latency distributions, sustained-RSS acceptance, or actual Ghostty display behavior.

Implementation continued while that measurement gate remained closed. The original executable and pre-storage executable remain available for baseline runs.
No compatibility result replaces the required independent terminal expectations.

The current projection compresses the existing split and unified representations separately.
One shared context/replacement block index remains pending. Temporary full row arrays still exist for the current file during conversion, not for the whole corpus.
Syntax checkpoints remain pending. Bounded long-line geometry and text-position recovery pass focused process checks, not full visual acceptance.
Cold and backward syntax requests still replay from the start of the requested hunk. Forward requests can reuse one continuation.
A geometry miss scans a complete line once for that width.

Next engine work addresses resumable syntax state and the complete responsiveness/memory trace, without changes to source semantics.
The [large-diff specification](../design/LARGE_DIFFS.md) still owns the targets. Actual Ghostty inspection remains separate.

## Reproduce the source check

Requirements: Rust, Git, Python 3.11 or later, `shasum`, and a Unix PTY.

```sh
cd /Users/ramos/oss/native-diff
cargo test --release --locked -- --test-threads=1
cargo test --release --locked -- --include-ignored --test-threads=1
cargo clippy --locked --all-targets -- -D warnings
cargo fmt --check
python3 tools/check_corpus.py target/release/native-diff evidence/source-check-next.json
```

The second test command also requires both ignored-data prerequisites. It includes the five corpus bodies and the pinned pre-storage executable.
The checker requires a new output filename and rejects changed fixture hashes.
It records source correctness, not performance timings.
DiffGrid, terminal preferences, global installations, and external repositories remain untouched.
