# Large-diff foundation: implementation specification

Date: 2026-09-10. Status: implementation started, not performance-accepted. See the [progress report](docs/history/LARGE_DIFF_PROGRESS.md).

## 1. Decision and scope

Render the exact cached patches that Pierre uses in DiffsHub, with correct content, responsive terminal interaction, and low memory use.
Linux v6.0…v7.0 is a required acceptance case. It is not an optional stress experiment.
The existing [product spec](SPEC.md) still owns controls, source correctness, themes, and terminal behavior.
This document replaces its provisional large-input selection and defines the next implementation batch.

This is product engineering, not a comparison benchmark against other viewers.
Pierre supplies workload and visual references, not expected timing results. Browser/network timings do not become terminal performance claims.
DiffGrid remains untouched. No maintainer contact, publishing, global installation, or terminal preference changes are required.

## 2. Exact workload

The pinned [DiffsHub route source](https://github.com/pierrecomputer/pierre/blob/0a7a733078f093577cd21ca65d2508806a22238a/apps/diffshub/app/api/diff/route.ts) contains five cached patch mappings.
All five downloaded bodies now have SHA-256 identities in [corpus.json](fixtures/diffshub/corpus.json).
The CDN bytes define the workload. A current PR response or a newly generated Git diff cannot silently replace them.

| Required input | Bytes | Physical lines | File records | Hunks |
| --- | ---: | ---: | ---: | ---: |
| Pierre commit `0800fb` | 610,480 | 18,857 | 73 | 109 |
| Ghostty PR `12291` | 4,183,400 | 93,812 | 25 | 130 |
| Node PR `59805` | 21,469,511 | 452,727 | 3,420 | 19,198 |
| Bun PR `30412` | 43,306,655 | 1,029,583 | 2,188 | 2,129 |
| Linux `v6.0...v7.0` | 710,870,899 | 20,064,075 | 76,872 | 421,819 |

These are local inventory results from the downloaded bodies, not website performance claims.
Physical lines include headers and markers. They are not displayed rows or changed-line counts.
Git reports 11,457,617 added lines and 4,333,921 deleted lines in the Linux patch.
Its largest file-patch record is 15,785,181 bytes. Its longest physical line is 357,709 bytes, excluding LF.

Bodies reside under `evidence/diffshub-corpus/`. Their combined size is 780,440,945 bytes, approximately 744 MiB.
The source manifest remains separate from these ignored data files. Fixture contents are never executed.

The independent reference now covers all supplied source records, line numbers, signs, and newline markers.
Normalized metadata expectations and complete terminal grids remain pending. Upstream source-blob correspondence is not established.
Observed commit refs are recorded as unverified metadata, not as proven origins for the cached bodies.
Current GitHub file-list responses are not completeness oracles for these large changes.
Exact supplied-patch correctness is mandatory here. Upstream-blob correspondence is tracked separately and does not block this patch-viewer batch.

Two supplements remain small and diagnostic:

- The existing visual/text-boundary corpus, with independent before/after bytes.
- Deterministic single-hunk and long-line cases that isolate replay, wrapping, and cancellation failures.

Generated cases cannot substitute for a failing DiffsHub case. A separate 1 GiB synthetic target is outside this batch.

## 3. What the reader experiences

The terminal opens with a loading state. The first complete, validated file appears before the remaining input reaches EOF.
Later files append without moving the reader. A file with incomplete hunks remains pending and never appears as complete.
One enormous file can remain pending until that file is coherent. The UI and quit controls remain responsive throughout.

The status distinguishes three milestones: available content, complete input/index, and completed visible styling.
Plain code can appear before colors. This does not count as completed syntax styling.
First correct content requires an actual source line with correct file/hunk context, not only a header or metadata card.
Known missing syntax context and deterministic long-line fallback remain explicit after styling work finishes.

During loading, `G` reaches the current available end and reports that input is still loading. It does not enable follow mode.
A later `G` reaches newly appended content. Split/unified changes and resize retain the active source and its position on screen.
On a later parse failure, earlier complete files remain readable with a persistent incomplete/error status. Normal quit then returns 1.

At any size, old/new bytes, line numbers, ordering, file metadata, and no-newline markers remain correct.
Performance work cannot hide files, shorten lines, omit hunks, or disable supported syntax globally for large inputs.
The existing 10,000-scalar styling fallback remains deterministic. It changes styling, not source availability.

## 4. Engineering targets

These are initial acceptance targets, not achieved results or numbers copied from Pierre.
Reference host: the current Apple Silicon Mac, macOS 26.3, `Mac17,6`, 48 GiB RAM.
Primary surface: an existing Ghostty window, 160×50 cells, split/dark/truecolor, no wrapping at startup.

| Measure | Target |
| --- | --- |
| First correct content | At most 500 ms from process start to a complete first content grid |
| Full input and unwrapped index ready | At most 2 s for each smaller showcase, at most 10 s for Linux |
| First visible viewport fully styled | At most 1 s from process start, with documented context/fallback states |
| Cold designated seek fully styled | At most 2 s from command injection to its defined completed style/context state |
| Retained-checkpoint or cached revisit styled | At most 100 ms from command injection |
| Navigation, layout switch, and resize | p95 at most 50 ms to the correct updated grid, no sampled response above 200 ms |
| Normal quit or cancellation | At most 250 ms to process exit and restored terminal modes |
| Smaller four showcases | Steady RSS at most 128 MiB, peak RSS at most 192 MiB |
| Linux showcase | Steady RSS at most 512 MiB, peak RSS at most 768 MiB |

The responsiveness target also applies while input or syntax work is active.
Cold syntax at a distant position is measured separately from correct plain-text navigation and from retained-checkpoint/cached revisits.
Every designated cold seek must settle to its defined style/context state. A 30-second watchdog failure is not a successful fallback.
That watchdog is a hang detector, not an acceptable interactive-speed claim.

The RSS targets include all viewer workers and any owned helper processes. The terminal process is reported separately.
Steady RSS is the maximum during the settled portion of a third fixed memory tour, not a selected low sample.
Disk spool size, source/index allocations, cache payloads, and OS memory measurements remain separate fields.
Kernel filesystem cache outside process RSS is not described as free memory or included in an allocator-only claim.
After warm-up, retained state must plateau within its explicit budgets. A tour cannot retain another document-sized structure.
Each tour visits the fixed file quantiles and structural extremes defined in [scenario.json](fixtures/diffshub/scenario.json).
It is not an exhaustive display of all 20 million lines. The separate full-content check covers every supplied source segment.

Failure leaves a target unmet. It does not justify silently changing the corpus, excluding expensive files, or weakening visual settings.
Any target revision needs a recorded explanation and user review.

## 5. Confirmed code paths to change

The initial manual audit found these paths. Line references describe the preserved original source, not the current edit positions.
The progress report records the original parse/index diagnostic and the first input-path corrections.

| Priority | Evidence | Required direction |
| --- | --- | --- |
| P1 | `main.rs:63` reads all input. `model.rs:205` indexes everything before returning. | Publish complete file units while later data loads. |
| P1 | `model.rs:15` owns bytes per line. `model.rs:66` stores two row streams, inverse maps, and partners. | Store source once and project rows from compact metadata. |
| P1 | `highlight.rs:62` visits files/hunks. `highlight.rs:82` replays each selected hunk from its beginning. | Direct lookup, resumable parser state, bounded checkpoint retention. |
| P1 | `render.rs:35`, `render.rs:440` repeatedly scan whole lines and rebuild wrap starts. | Cache geometry and seek directly to visible segments. |
| P2 | `app.rs:241` scans intervening display rows for file/hunk movement. | Navigate through file/hunk boundary indexes. |
| P2 | `highlight.rs:49` queues requests without a bound. Same-theme replies can cause offscreen repaints. | Coalesce viewport requests, bound replies, repaint only affected visible content. |

Existing strengths remain: viewport-only drawing, separate source/display text, separate old/new syntax states, and terminal work outside input parsing.
The audit covered ingestion, storage, projection, highlighting, and navigation. Web, database, and service concerns are irrelevant here.

## 6. Implementation design

### Source storage and ingestion

Use a process-owned, seekable spool with restrictive permissions. Copy and parse incoming bytes incrementally, including regular-file inputs.
The spool preserves exact patch bytes and makes stdin seekable. It avoids a second complete resident source buffer.
Detect source-file changes during capture and reject detected instability. Do not promise an atomic snapshot of a concurrently modified caller file.
Completed regions of the spool are immutable. Cleanup removes only this invocation's owned temporary data.

Reuse libgit2 validation on complete file-patch units first. Do not write a replacement diff parser before measuring this approach.
Framing must respect hunk counts, quoted paths, binary sections, and metadata-only files. A raw delimiter search alone is insufficient.
An ordinary unified patch without Git file delimiters can remain a single pending unit until EOF.
The current Linux corpus has no file-patch unit larger than approximately 15.1 MiB, which makes this staged approach worth measuring.

Release each file's parser-owned buffers after producing its compact index. Never retain libgit2 handles for the whole corpus.
Publish one immutable file index through a bounded queue. Transfer ownership rather than copying completed source/index payloads.
Backpressure pauses ingestion, not keyboard input. An exceptional large file remains accounted for in peak memory, not hidden behind a cache budget.
If per-file parsing misses the targets, profile it before authorizing a streaming-parser replacement.

### Source identities and compact projection

Use append-stable `FileId` values and source identities composed of file, side, and source-line ordinal.
Keep source identity, a stable text position, and screen-relative offset in the reading anchor for wrapped-line and resize recovery.
The current read-only viewer uses a byte offset in the immutable display-safe line. That offset is not an offset for raw-source export.
The sanitizer and tab width do not change during a session. Future changes to either require an explicit anchor conversion.
Display-row numbers are derived coordinates, not durable source identities.

Replace each owned line buffer with an offset/length/flags record into the spool. Use 64-bit byte offsets and checked arithmetic.
The initial record budget is at most 16 bytes per supplied source line, excluding separately reported file/hunk/block metadata.
Source line numbers derive from hunk starts and run counts. No allocation exists solely to hold one line's bytes.

Store context and replacement blocks once, with split/unified row-count prefixes and compact alignment decisions.
Resolve only the viewport and a bounded nearby region. Do not retain both full row streams or both inverse maps.
For bounded small blocks, preserve the existing reviewed pairing rule. Large-block fallback remains deterministic and source-preserving.
When a worker finishes, the viewport moves, or a theme changes, pairing must stay unchanged.

File/hunk boundary indexes support direct neighbor lookup or binary search. Switching layout does not rebuild or walk the whole document.
The 2026-09-11 navigation decision removes numeric prefixes from this batch.
The current reading controls cover rows, pages, scrolling, source endpoints, and adjacent files. `n` / `N` replaces the prototype file shortcuts.
Benchmark setup must use supported public controls. It must not drive additional product commands solely to reach a measurement anchor.
A file picker remains outside this batch.
First implement compact flat metadata. If allocation accounting shows that metadata exceeds its target, add sparse or disk-backed indexes.
No database, bespoke compressed file format, or mandatory memory-mapping subsystem is part of this batch.

### Highlighting and scheduling

Keep one syntax worker initially. More workers are not the default answer to repeated work or high memory use.
Map requested source IDs directly to their file/hunk. Retain advancing old/new state and checkpoints, initially every 256 supplied source lines.
Use Syntect's existing state/resume APIs. Checkpoints include source identity, side, grammar identity, and theme generation for correctness.
Theme changes initially invalidate theme-bound states. Reusing parser-only state across themes is optional after evidence supports it.

One latest-viewport slot replaces a request backlog. A separate cancellation signal must remain observable while output is backpressured.
Replies have count and byte limits. The UI consumes bounded batches before checking input again.
If their source/style identity remains valid, stale viewport replies can enter a bounded cache. They do not trigger unrelated redraws.
Check cancellation between bounded source/geometry work units. A single non-interruptible grammar call remains visible in diagnostics.

Never create syntax state at an arbitrary destination and present it as correct.
A cold seek can require predecessor parsing, but that work cannot block navigation. A retained checkpoint bounds replay on a warm revisit.
Eviction can turn a warm visit cold. Report that state rather than excluding the sample.

### Geometry and terminal output

Cache line geometry by source identity, pane width, tab width, gutter width, wrapping, and sanitizer version. Theme is not a geometry key.
Store byte/cell checkpoints and wrap boundaries for needed segments. Draw from the requested segment instead of scanning every preceding glyph.
Long-line discovery runs in cancellable chunks outside the input path. It cannot allocate a full expanded display string without a bound.
The original line remains accessible, including escaped bytes, tabs, combining sequences, and wide characters.

Keep the existing Ratatui buffer and synchronized terminal updates. Do not add animation, a custom terminal emulator, or scroll smoothing.
Coalesce paint work, not user navigation semantics. Preserve final destinations and direction reversals during rapid input.

The shared disposable budget stays 64 MiB for token spans, syntax checkpoints, and geometry payloads.
A separate source-page cache is capped at 32 MiB. Report container capacities and allocator overhead separately, then enforce RSS targets as well.
No individual cache entry can bypass its budget. Eviction changes recomputation cost, never source correctness.

### Code organization

Keep one Cargo package. Add `storage.rs`, `ingest.rs`, and `layout.rs` only where they separate ownership and responsibilities.
Keep `model.rs` for immutable source metadata, `app.rs` for reading state, and `render.rs` for terminal cells.
Keep scheduling in `highlight.rs` until geometry work demonstrably needs a shared internal scheduler.
No plugin framework, service, runtime daemon, or extra workspace package is required.

## 7. Correctness and measurement plan

### Full supplied-content reference

Before acceptance, independently decode every supplied source segment, number, newline marker, and metadata record from the exact cached bodies.
Freeze those expected digests and anchor records before measuring a changed viewer. The reference decoder cannot call or share the viewer's parser/layout code.
Use Git's independent patch parser/application checks where applicable, plus the small corpus's known before/after bytes, to challenge the reference decoder.
The full check compares the viewer's result against every supplied segment, not only against Git totals or viewport sentinels.
Missing context has no invented expected bytes. Binary files remain metadata cases. The current numstat inventory alone does not satisfy this gate.

Upstream-blob correspondence is a separate provenance check. It is not required to prove faithful display of the pinned input.
If upstream correspondence is pursued, check merge-base and cached snapshot identity instead of assuming current PR refs match.
Read isolated Git objects without checkout filters, hooks, repository scripts, or patch execution. Do not fetch whole repositories merely to unblock this batch.
Unresolved correspondence stays explicit. It does not justify replacing a body or claiming verified upstream origins.

### Four meaningful process scenarios

1. **Content and safety:** reconstruct all supplied sides across the five exact patches, plus the small unsafe/Unicode corpus.
2. **Incremental lifecycle:** feed complete file A through a real FIFO, block before B, append B, then repeat with malformed B and cancellation.
3. **Reading under load:** scroll forward/backward, jump across distant files and the largest hunk, switch layouts/themes, wrap, resize, and revisit prior positions.
4. **Memory and geometry:** repeat the fixed tour three times, revisit evicted regions, and traverse the longest line through wrapping and horizontal scrolling.

Use the actual release executable, parser, highlighter, files, PTY input, and signals. No mocked process or fake terminal backend supplies acceptance.
Compare completed terminal grids against independent expected content. Verify each input reaches its correct destination, not merely that any frame appears.
Targeted work counters record reparsed lines, geometry bytes, queue occupancy, and allocated index/cache bytes. They explain costs, not human-visible latency.

### Small repeatable measurement procedure

Freeze executable/lockfile/fixture hashes and settings. Acquire all network data before any timed run.
Run locally, serially, with no competing corpus runs. Do not change power, display, or terminal preferences automatically.
Use five fresh-process runs per fixture after one filesystem warm-up. Label these fresh-process/warm-filesystem runs, not cold-machine runs.
The checked-in scenario policy defines deterministic structural selectors, category counts, input cadence, and reducers.
Step A resolves selectors to exact source anchors and public input sequences. Freeze that trace before changed-viewer timings begin.
Use 40 checked inputs per category per run: forward, reverse, distant seeks, layout, and resize. Five runs supply 200 samples per category.
The forward/reverse categories use real wheel events that move the viewport, not merely the cursor. Keyboard navigation has separate correctness checks.
Schedule inputs at least 50 ms apart, with at most one pending measured command. Late completion delays the next command, never drops a sample.
Measure from command injection, not from its scheduled time. A separate bounded rapid-input sequence checks coalescing and final-position correctness.
The historical content-grid trace retains its original counted setup and key bindings. It is not the current navigation contract.
Before changed-viewer timings, version the affected input sequences explicitly. Do not rewrite historical inputs or results to match the new controls.
Use nearest-rank p95 separately for each fixture/category. Every category must meet both its p95 and maximum limit.
Opening, indexing, initial styling, cold styling, and quit limits apply to the maximum of all applicable repeats. Also report medians.
Cold-style trials use fresh processes and record cache state. Their final public command must expose a target whose syntax state is not cached.
Do not pool categories or fixtures. Timeouts and unsupported operations are failures or explicit missing baseline data, not discarded samples.

Start the opening clock before spawn. Stop each milestone at the corresponding complete, correct terminal grid or explicit index-ready event.
Record input injection, correct-grid receipt, styling completion, process exit, and terminal restoration separately.
Use standard OS process-memory tools with recorded units and at most 20 Hz sampling. Obtain peak RSS from an OS high-water measurement too.
Sampling alone cannot establish the true peak. Frame flush alone cannot establish that a pixel reached the display.

The automated result is explicitly PTY-grid latency. Actual display latency requires a visible Ghostty window and a separate recording pass.
Record the terminal version, font, cell geometry, refresh rate, and capture frame rate. Report frame-time uncertainty.
Do not mix recording overhead into unrecorded engine measurements. Videos prove appearance/movement and are not a substitute for content checks.
Inspect 80×24 and both themes as correctness variants, not a large Cartesian performance matrix.

Keep the output small: one result JSON and one table. When capture is available, include one short evidence clip per showcase.
No dashboard or multi-candidate benchmark framework is required. Missing GUI access blocks visual signoff, not engine implementation or PTY checks.

## 8. Build order and completion

| Step | Deliverable | Exit condition |
| --- | --- | --- |
| A | Pinned corpus, independent patch reference, frozen trace, baseline | Exact bodies stay fixed. Expected anchors and all supplied segments are defined. Baseline failures and memory costs are recorded. |
| B | Spool, per-file publication, compact source/block model | Full-content checks pass. Earlier files remain usable before EOF and after later failure. |
| C | Viewport projection, stable anchors, indexed navigation, bounded geometry | Correct destinations and source bytes survive scrolling, wrapping, layout changes, and resize. |
| D | Resumable highlighting, coalesced scheduling, explicit budgets | Final styling matches sequential reference. Repeated work and retained memory remain bounded as specified. |
| E | Release evidence and visual corrections | All five required cases meet targets, with actual Ghostty appearance/movement signoff recorded separately. |

Step A includes pinned source digests, structural anchors, and an original parse/index diagnostic.
The [content-grid protocol](MEASUREMENT.md) adds a resolved input trace and a runner for the preserved interactive baseline.
Its scope excludes style completion, normalized full-metadata coverage, long-line tours, and steady-memory acceptance. Those requirements remain pending.
Step B now includes the private spool, progressive file publication, compact source records, and compressed display ranges.
Full source checks and selected corpus terminal checks pass. A shared block index still needs implementation or an evidence-based design review.
Steps C–E remain incomplete or unverified. This document does not claim that the prototype meets these targets.
New reading features remain in the subsequent batch: search, copy, picker, Git/two-file commands, and context expansion.
This batch does not quietly expand into those features or stop merely because a GUI inspection is unavailable.

## 9. Source and integrity notes

The [DiffsHub homepage](https://diffshub.com/) identifies the public examples. The pinned route source supplies the exact CDN mappings.
Pierre's [rendering article](https://pierre.computer/writing/on-rendering-diffs) describes its browser architecture and published performance claims.
Those claims were inspected as background, not reproduced or adopted as our results or pass thresholds.
There is no clean-room independence claim for this product work. It must not be promoted as clean-room DiffGrid work.

The performance-audit skill supplied the source-review method and an independent read-only design review.
The pragmatic ASD-STE100 documentation method keeps the requirements, observed facts, and proposed targets distinct.
