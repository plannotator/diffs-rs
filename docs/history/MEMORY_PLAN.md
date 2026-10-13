# Memory and projection implementation plan

Date: 2026-09-11. Status: storage stages implemented and checkpoint measurements complete. See [implementation results](MEMORY_IMPLEMENTATION.md). A first-color regression, syntax-allocation fixes, and full performance acceptance remain open.

Reduce memory while preserving the exact source, settled terminal cells, and reading destinations of the current viewer. Correctness is a requirement for accepting each change. Performance goals apply after that requirement passes.

This plan defines the next work in the existing Rust package. [LARGE_DIFFS.md](../../LARGE_DIFFS.md) owns performance targets; [FIVE_PATCH_ACCEPTANCE.md](../../FIVE_PATCH_ACCEPTANCE.md) owns the current measured workload. This plan adds representation invariants and migration checks. It does not implement the unbuilt features proposed in [SPEC.md](../../SPEC.md).

## 1. Starting evidence and decisions

The preserved starting executable has SHA-256 `d78884cb328909322caad55b7ed7b12063eb24b25ded60ecf32e9422665477f9`, verified during planning. The [five-patch results](FIVE_PATCH_RESULTS.md) remain the recorded starting measurements.

| Patch | Source index, bytes | Other reported index payload, bytes | Combined, MiB | Third-tour RSS, MiB |
| --- | ---: | ---: | ---: | ---: |
| Pierre | 294,720 | 79,696 | 0.36 | 303.3 |
| Ghostty | 1,497,264 | 1,594,288 | 2.95 | 123.7 |
| Node | 6,708,896 | 18,386,928 | 23.93 | 300.6 |
| Bun | 16,271,536 | 1,128,016 | 16.59 | 377.0 |
| Linux | 308,786,800 | 430,208,368 | 704.76 | 1,503.2 |

Index figures come from the existing `inspect --summary` command used in the memory audit. RSS comes from separate interactive measurements. They must not be subtracted to infer an exact allocation breakdown.

The existing `projection_bytes` counter includes `SourceRun` storage as well as both layouts and their inverse maps (`src/storage.rs:255`). The layouts and maps are already run-compressed (`src/layout.rs`). Their individual contributions are not yet recorded. Describing the whole 410 MiB aggregate as duplicate display rows would be inaccurate.

Linux's measured index payload already exceeds its 512 MiB steady RSS target. Reducing it is necessary. Pierre's 0.36 MiB of indexes cannot explain its 303.3 MiB RSS; reducing indexes alone cannot meet all five memory targets. Navigation allocations, grammar state, caches, and allocator retention need their own attribution. A leak has neither been established nor ruled out.

Decisions for the first implementation batch:

- Preserve libgit2 validation, the current pairing rule, palettes, sanitizer, syntax dependencies, controls, and Ratatui output.
- Introduce compact source spans independently of projection changes.
- Replace the two retained layouts and inverse maps with one block representation after exhaustive equivalence checks exist.
- Keep allocation attribution as a required work item for the smaller patches.
- Use concrete types in the current modules. The initial work needs no new dependency, arena library, custom allocator, memory mapping, or service.
- Treat worldwide performance leadership as the long-term objective. This batch can establish correctness and improvements against our preserved build; it cannot establish a ranking against other products.

## 2. Observable contract

These invariants apply while loading, after navigation, during pending syntax work, after eviction, and after resize. A failure rejects the change even when latency or RSS improves.

| ID | Required behavior | Evidence |
| --- | --- | --- |
| C1 | Every supplied source record retains its original bytes, kind, old/new line numbers, hunk/file identity, and final-newline flag. Context is one source record associated with both sides. | Complete independent canonical digests for all five patches; small known old/new sources. |
| C2 | File order, paths, change type, modes, counts, hunk labels, omitted-context gaps, and metadata-only/binary entries remain accurate. Empty files remain visible. | Explicit metadata expectations plus full metadata regression records. |
| C3 | In each layout, every source occurrence is present exactly once on its applicable side and in side order. Padding has no source ID, number, sign, or syntax. | Exhaustive projection comparison and independent small replacement examples. |
| C4 | Pairing is immutable after file publication. Unified replacements show old records then new records. No-newline markers follow the correct code row and label the correct side. | Row and partner comparisons; explicit paired/unpaired marker fixtures. |
| C5 | Sanitized glyphs, tab expansion, Unicode boundaries, wrapping, gutter roles, colors, and changed-word spans match the current reviewed behavior. | Existing explicit cell contracts and five-patch interior checks. |
| C6 | Resize, layout, wrap, and line-number changes preserve source identity, active side, display-text byte anchor, and permitted screen position. The below-minimum window state preserves the anchor. | Real PTY input/resize tests, including pending highlights and cache eviction. |
| C7 | Each input retains its current semantics. Source endpoints select source, file jumps include metadata files, and boundary commands do not wrap or replay after later input arrives. | Existing process navigation and loading tests. |
| C8 | Completed files remain immutable and readable while more input arrives. A later error produces an incomplete status and failure exit, never a successful truncated diff. | FIFO, malformed-input, stable-spool, signal, and cleanup tests. |
| C9 | A style reply belongs to the same source and theme generation. Old/new parser states remain separate. Painting a reply changes no source, pairing, geometry, or anchor. | Sequential syntax comparison, explicit syntax cases, burst and resize tests. |

A pending frame may show correctly classified plain text before colors arrive. Settled frames must match the complete expected cells and styles. Faster completion can change the number of pending frames; it cannot expose wrong rows or briefly apply another source's colors.

The existing long-line styling fallback remains exactly as implemented for this batch. We do not lower its threshold, classify expensive files as unsupported, truncate lines, normalize away whitespace, or start syntax parsing at an arbitrary destination. Missing source context remains missing.

The current implementation is a regression reference, not an independent correctness authority. If it conflicts with an independently established source or cell expectation, isolate and document the defect before changing the expectation. Do not bundle a new pairing rule or a visual correction into a memory refactor.

## 3. Representation and ownership

### Stable external interface

Keep `Document::row`, `row_for_source`, `info`, `bytes`, `boundary`, and `FileIndex::partner` behavior stable. `Row` remains a small value returned to the renderer, created on demand. Global source IDs remain append-stable ordinals for the life of the invocation.

The first source/projection migrations should require no changes to `render.rs`, `text.rs`, `theme.rs`, or the reading-anchor algorithm in `app.rs`. This gives the tests a narrow implementation boundary.

```text
Session
  spool: immutable published byte ranges, bounded page cache
  files: append-only Arc<FileIndex> values
    file metadata and global bases
    compact source spans
    one source-run table
    shared context/replacement/structural blocks
    small block lookup tables and marker positions

row lookup -> Row value -> existing geometry and terminal rendering
source lookup -> source run -> block + side ordinal -> row coordinate
```

Completed files and the spool have one allocation-accounting owner even when referenced by both the UI and highlight worker. No new reference cycle is permitted. Width-specific geometry and styling remain disposable; source records and pairing decisions do not depend on cache residency.

### Compact source spans

Today each source span occupies 16 bytes: absolute `u64` offset, `u32` length, and `u32` flags. For a file whose source offsets fit relative to one `u64` byte base, use an eight-byte record containing `u32 relative_offset` and `u32 length`. Keep final-newline flags in a separate bitset, one bit per source record.

Choose the compact or existing wide representation once per file. Validate all narrowing conversions and base-plus-offset-plus-length arithmetic before publication. A file that does not fit uses the wide representation with the same observable behavior. Keep the current supported line-length domain; do not steal a length bit to encode a flag. Global patch offsets can exceed 4 GiB because the file byte base remains `u64`.

The bitset flag must refer to the canonical source ordinal. A missing-final-newline marker changes the supplied source length in exactly the same way as today; neither its physical patch newline nor the marker text becomes source data. Ordinary unified input that contains multiple files in one ingestion unit must use verified offsets, not an assumed delimiter position.

For N compact records, span plus flag payload is `8*N + ceil(N/8)` bytes, plus a small per-file base/descriptor and per-file bitset rounding. Linux's 19,299,175 records imply approximately 149.5 MiB, versus 294.5 MiB today. The approximately 145 MiB difference is a layout calculation, not a promised RSS reduction. Stage S0 must confirm compact eligibility and exact per-file overhead.

Use ordinary aligned Rust fields, boxed slices, and checked conversions. Avoid packed structs or unsafe decoding. Verify record size and overflow boundaries directly, including `u32::MAX`, without constructing a 4 GiB fixture.

### One shared projection

Use one ordered block table per file. Blocks have split and unified row-start prefixes so either layout supports binary search. A block describes one of:

- Structural content: header, hunk label, omitted-context gap, or metadata row.
- Context: a contiguous source range, one logical code row per record in both layouts.
- Replacement: the old-side and new-side source sequences, their lengths, and the existing alignment decision.

Begin with machine-width block counters and compact source spans. Narrow block fields only when the measured component breakdown justifies it and checked wide handling is defined. Removing duplicate structure comes before tuning every field width.

A replacement keeps the exact current grouping of a maximal non-context run inside a hunk. Compute the current `alignment_offset` once during construction. Its bounded-search thresholds and deterministic fallback remain unchanged. Scrolling, resizing, theme changes, and worker scheduling never recompute alignment.

For D deletions and A additions, let `old_pad` and `new_pad` be the current alignment padding. For logical split row j:

```text
left  = old[j - old_pad] if that ordinal exists, otherwise None
right = new[j - new_pad] if that ordinal exists, otherwise None
split code-row count = max(D + old_pad, A + new_pad)

unified code rows = old[0..D], then new[0..A]
```

For example, three deletions paired with one addition at `new_pad = 2` resolve to `(old0, None)`, `(old1, None)`, `(old2, new0)` in split view. Unified view resolves to `old0, old1, old2, new0`. Neither view stores a second copy of those source records.

Do not assume all accepted input emits one contiguous deletion span followed by one addition span. The present model collects records by side. Preserve that behavior with references to contiguous source runs when a block has several runs on a side. The common case uses one range; any auxiliary side-run references scale with runs, not records. Canonical traversal retains original supplied-record order independently of unified display order.

Extend or replace the current source-run table so a run belongs to exactly one hunk/block and records its first source ordinal, count, kind, applicable starting line numbers, and first side ordinal within that block. Runs cannot cross block/hunk boundaries. This single table supports both source metadata and inverse navigation.

Source-to-row lookup becomes: find the source run, identify its block and side ordinal, apply the split/unified formula, then account for marker rows. Row-to-source lookup finds a block by its selected prefix and resolves its side run. Structural blocks have no source identity. Header/hunk lookup tables contain only boundaries.

No-newline markers require explicit treatment. For each layout, a block can retain sorted, unique logical code-row positions that need a marker; a paired row with both flags still produces one split marker row. For logical row j, its displayed row is `j + count(markers < j)`. Marker i after logical row m is displayed at `m + i + 1`. Reverse lookup binary-searches marker display positions, then subtracts preceding marker rows before resolving a code row. Inverse source lookup always returns the code row. Side flags come from the source records. A context record marked on both sides follows the same rule.

Resolve rows with O(log(files) + log(blocks in file) + log(runs in block) + log(markers in block)) worst-case lookup and constant local arithmetic. Sequential viewport iteration may reuse a block cursor. Layout/width changes must not scan the whole document or allocate proportional to document length. Keep wrapped-segment lookup in geometry, where it already belongs.

The retained projection should scale with files, hunks, source runs, and marker exceptions. Alternating single-record changes can still make the number of runs proportional to source records. Report this case and measure its actual representation size; do not claim constant memory or universal compression.

### Builder lifetime

First construct the new representation from one validated `model::Document` file unit, preserving the parser and pairing function. The old row streams are allowed only in the separate equivalence checker during this migration. The normal viewer uses one projection implementation.

After projection equivalence passes, remove temporary full row/location vectors from ingestion. Produce blocks and source runs directly from validated records, and release parsing storage after publishing each file. Next consider borrowed input slices or offset/length metadata for temporary lines while preserving libgit2's offset checks. Reuse a bounded file scratch allocation; no session-long arena may retain every parsed unit.

Plain `inspect` JSON is a separate compatibility task. Preserve its current shape, ordering, byte arrays, and failure behavior. Its streaming conversion must not weaken the independent reference path used to validate compact storage. It does not block the interactive memory improvements.

## 4. Allocation attribution and later work

Before changing ownership, extend existing diagnostics with disjoint byte counts for source spans/flags, source runs, split/unified row runs, inverse maps, file/hunk metadata, and container capacities. After migration, report shared blocks and side-run/marker indexes under new explicit names. Keep the old `projection_bytes` aggregate documented for comparisons. Count shared allocations once; distinguish logical length, allocation capacity, and opaque library allocations.

Use an optimized build with debug information in a separate target directory for allocation profiling. Inspect one Linux indexing run and the fixed Pierre memory tour first. Use an available native allocation/VM profiler, checking tool availability before specifying automation. Capture live heap versus freed-but-retained allocator pages, grammar/regex storage, temporary strings, container capacities, and any file mappings. If allocation stacks are unavailable, keep that attribution unresolved and proceed with measured layout costs.

Diagnostic instrumentation belongs in a separate opt-in build/output path and must not print into the terminal grid. Regular accepted timings use the uninstrumented release build. A Rust-only allocator wrapper can miss libgit2 and other C allocations; it cannot be treated as total process accounting. RSS and the OS high-water observation remain required.

Subsequent changes are conditional on that evidence:

| Candidate | Bounded implementation | Correctness condition |
| --- | --- | --- |
| Sanitized-text sharing | One source-text owner shared across width-specific wrap data; account for the text once while any entry retains it. | Identical escaped text and byte anchors at every width; no whole-file text cache. |
| Source/text scratch reuse | Reuse worker byte/text buffers; avoid the second full string just to append a syntax newline. Release exceptional capacity by an explicit policy. | Newline, invalid UTF-8, control escaping, and old/new syntax outputs remain identical. |
| Long-line geometry | A separately specified chunked path only if the profiler identifies it as necessary. Preserve UTF-8 decoding, escape, grapheme, tab-column, and wrap state across chunk boundaries. | Complete first/interior/end text remains accessible; arbitrary chunk cuts cannot define grapheme boundaries or reset columns. |
| Cache accounting/policy | Bound entry counts and charged capacities; preflight admission and include queued/active objects in diagnostics. | Eviction changes recomputation only. It cannot change pairing, source visibility, or final styles. |
| Syntax state/checkpoints | Retain the current two-side continuation until profiling proves a need and a byte-bounded replacement is specified. | No guessed starting state and no extra checkpoint collection without measured heap cost. |
| Allocator experiment | Compare available allocators only if attributed free-page retention remains material after representation fixes. | Same executable behavior/workload; report CPU, live heap, and RSS together. |

The initial migration changes none of the existing source, geometry, or style cache limits. Reducing those limits can turn cached navigation into cold replay, so it needs separate latency evidence. A global cache framework is unnecessary for the first stages.

## 5. Verification design

### Preserve and identify the reference

At S0, archive the current source, Cargo files, assets, tests, scripts, release binary, and syntax-reference binary in a new evidence directory before any build can overwrite the baseline binary. Record SHA-256 identities. The directory is not currently a Git repository, so a Git revision is not a sufficient rollback plan.

Preserve the existing five-patch result bundle, which currently resides under `/tmp/native-diff-five-patch.eYqc1K`, into durable evidence after validating its identities. Include its command plans, expected grids, reference inputs, and runtime manifests. Keep large pinned patch bodies by path/hash rather than copying the corpus into every stage. Record missing raw evidence explicitly; regenerate a labeled baseline if necessary.

Historical identities verified during planning:

| Artifact | SHA-256 |
| --- | --- |
| `fixtures/diffshub/corpus.json` | `bbc6ddad51aa9f84c89f14686d9c6c3493da3abdb0179d1aae75450cd8ea839f` |
| `fixtures/diffshub/reference.json` | `4073b8e04df310eb8afd78807f8c5f6c49b83cad9d2c7d9ea33a7cf1f5fd7c7e` |
| `fixtures/diffshub/measurement-v1/freeze.json` | `b1ed4121a5456ebda6ec59bd01c767a8161636b7bc0ff55a968b7f58d90da909` |
| `fixtures/diffshub/measurement-v1/trace.json.gz` | `ce8ad25165e18c96c6a204b28f16fbcab1ce8d8e3b3bc4cd70f86ad1390e5a95` |
| `evidence/content-baseline-v1/results.json` | `14ad8472edf96d4f4e1321d6fa472082988b141b11f68954a62f279fb98baa22` |

### Three complementary checks

1. **Independent source and cell truth.** Keep `tools/check_corpus.py`, the independent Python decoder, explicit small before/after sources, and current PTY cell contracts. Canonical source digests cover all supplied records; they do not cover normalized metadata or all pairings. The sequential syntax example verifies scheduling equivalence using the same grammar dependency, not independent grammar truth.
2. **Exhaustive representation equivalence.** Add one diagnostic executable/example that reads real patches and evaluates both the preserved per-file model and the new index. Compare every row in both layouts, every source-to-row destination, every partner, ordered hunk/file boundaries, metadata, and source-line metadata. Compare incrementally, fail at the first mismatch with file/hunk/source coordinates, and record counts plus streamed digests. This is a regression check across every row of all five patches; it is separate from performance measurement and from the independent source oracle. Its peak storage is one file unit plus the two per-file representations, not two full Linux documents.
3. **Real terminal behavior.** Run the actual candidate through the existing PTY tests and independently checked five-patch states. Require exact settled body cells, styles, gutters, markers, and destinations. Keep corruption controls that prove the checker rejects wrong source, side, number, blank rows, colors, and hunk headers. During burst input compare coherent frames and final destinations, not an identical count of asynchronous frames.

The equivalence diagnostic uses the production compact encoder/decoder. It must compare actual values, not only totals or self-reported hashes. Its preserved evaluator must not call the new projection implementation. Hashing streamed records reduces evidence size; direct comparisons provide useful mismatch locations.

Normalize both evaluators to the same global file/source identities before comparing. Compare semantic records, not Rust memory layouts or internal block identifiers. Exercise source-to-row round trips for both sides of context and replacements; marker rows have no inverse source identity.

### Focused additions to the existing tests

Extend existing fixture families and the shared process harness. Add cases only for representation boundaries that current tests do not settle:

- Unequal replacements with each padding direction, empty sides, repeated/blank lines, and accepted multiple source runs on a side.
- Old-only, new-only, shared-context, and paired no-newline markers, including source endpoints immediately beside metadata.
- Record/run/block/file transitions and compact span offsets exactly at width limits. Small encoder tests with explicit large numeric offsets cover overflow; real file/process cases cover actual reading and publication.
- Switching layouts and resizing at those boundaries while highlighting is pending, then evicting and revisiting the source.

Enumerate short valid replacement patterns in the equivalence diagnostic and stream their comparisons. Keep handwritten expected pairings and markers for the human-facing cases so two implementations cannot validate the same mistaken interpretation. Do not create a new synthetic benchmark suite or a second PTY framework.

The existing test at `tests/process.rs:1349` asserts `16 * source_records`. Replace that obsolete layout assertion when S1 lands with the new accounted-memory expectation, while retaining independent source counts, spool size, and exact terminal destinations. Correctness expectations must not be loosened to accommodate storage changes.

### Evidence and trace migration

The current `corpus_measure.py` binds its trace to a particular executable and source tree. A new build cannot use the old freeze by changing a hash in place.

For each candidate stage, create a new manifest. Require its action plan, selected sources, expected settled cells/styles, fallback states, and acceptance limits to match the preserved semantic expectations. Independently validate candidate captures, compare their settled expectations with the baseline, then bind the verified candidate, checker, dependencies, and trace. Retain both manifests and their relationship. Timing fields, diagnostic counters, and provenance fields can differ; accepted semantic expectations cannot silently change.

This adds a small validation step to the existing runner. It requires neither a new benchmark service nor hidden navigation commands. Source/index diagnostics are never the source of expected terminal content.

## 6. Work packages and acceptance

All stages are owned by the native-diff task. Source/projection stages run sequentially because they share the same ownership boundary. Product changes and diagnostics use separate reviewable changesets or snapshots.

| Stage | Work and primary files | Required result before proceeding |
| --- | --- | --- |
| S0: evidence and attribution | Preserve baseline; add component counts in `storage.rs`/`layout.rs` and diagnostic output; inspect Linux and Pierre allocations; add the candidate trace-comparison step. | Reproducible reference package, component breakdown, profiler findings/limits, unchanged source and terminal checks. No production ownership change. |
| S1: compact source spans | `storage.rs`; narrow boundary tests and existing process test update. | Compact/wide paths decode identically; all source digests pass; exact span/flag savings reported; no interface or cell change. |
| S2: shared projection | `layout.rs`, `storage.rs`, narrowly `model.rs` for existing alignment access; one equivalence diagnostic. | Every row, inverse lookup, partner, and metadata record agrees across five patches; independent process/cell checks pass; actual index savings measured. |
| S3: lower ingestion peaks | `model.rs`, `storage.rs`, `ingest.rs`. Build compact blocks without temporary full row/location vectors; reduce temporary line ownership where justified. | Same parser validation and source offsets; progressive loading, malformed input, cancellation, single-file peaks, and complete source checks pass. |
| S4: reduce navigation growth | Change only the allocation owner identified in S0, starting with Pierre; likely `highlight.rs`, `geometry.rs`, or source/text scratch ownership. | Attributed live/capacity changes, unchanged exact styling/anchors, smaller settled/peak RSS, and measured recomputation cost. Repeat attribution if necessary. |
| S5: acceptance report | Existing corpus/process runners and a new result report. | All five fixtures explicitly accounted for, correctness gates passed, timing/memory targets evaluated, old and new distributions retained, remaining failures stated. |

S1 has a predictable storage reduction and is the first behavior-preserving implementation after S0. S2/S3 address Linux's additional index and parse costs. S4 is necessary to resolve the small-patch memory failures; it is not assumed to disappear after S2. A plain-JSON `inspect` rewrite is a later, separate diagnostic compatibility task.

Keep each accepted stage recoverable by its source and executable snapshot. Do not use destructive workspace rollback. An intermediate stage can be correctness-accepted while overall performance targets remain unmet; it cannot be reported as full acceptance.

### Test and measurement cadence

For each changed stage, run its focused real-process cases, exhaustive affected representation comparison, all five canonical digests, formatting, and Clippy. Run the complete process suite including the two corpus-dependent tests at S2 and final acceptance, or sooner for a cross-module change. The current ordinary suite has 29 active tests plus two ignored corpus-dependent tests; test counts alone are not a correctness criterion.

Use one focused attribution/latency run to reject an obviously bad candidate before spending time on the full batch. Full five-repeat latency and three-tour memory acceptance runs occur at the retained representation checkpoint after S3 and at final acceptance after S4. Additional full repeats require a changed candidate, a failure investigation, or evidence of measurement contamination. Preserve failed results.

## 7. Performance gates and coordination

Keep all five exact pinned patches. Apply existing limits separately to each patch and each milestone:

| Measurement | Limit |
| --- | --- |
| First correct source viewport | 500 ms |
| First visible viewport fully styled | 1,000 ms |
| Full input/index ready | 2,000 ms for smaller patches; 10,000 ms for Linux |
| Navigation/layout/resize | p95 <= 50 ms; no measured response > 200 ms |
| Designated cold styling / retained revisit | 2,000 ms / 100 ms |
| Quit/cancellation | 250 ms |
| Smaller-patch steady / peak RSS | 128 MiB / 192 MiB |
| Linux steady / peak RSS | 512 MiB / 768 MiB |

A faster final-ready timestamp must not hide a slower first-content timestamp. The Linux 4,535.7 ms figure in the starting report is an opening-ready bound, not first source paint or first styled viewport time. Report these milestones separately.

The existing five-category run includes endpoint seeks; its public setup may color an intended destination. It cannot alone establish every designated cold-style or retained-cache target. Label and run those cases with documented cache/replay state when claiming those limits. A timeout is never a successful styling fallback.

Compare same-condition baseline and candidate distributions, including maxima, per category and patch. Investigate a consistent slowdown even when it remains under an absolute limit. Memory savings are not sufficient justification for a known interaction regression. A change is accepted only with an explained tradeoff and evidence that the correctness and responsiveness requirements still hold.

Before profiling or measuring, confirm the host reservation with other active work, then run one foreground workload on the Mac at a time. Release the reservation when the session finishes; record release in the evidence. Ordinary editing and planning need no host reservation. External benchmark results are not correctness or timing expectations for this work.

Preserve the warm-file/fresh-process conditions, public input sequences, terminal cell sizes, settings, five repetitions, and three memory tours. Report median/p95/maximum where applicable, all failures, index component bytes, OS peak RSS, and third-tour settled RSS. Live heap, allocator retention, disk spool size, and terminal-process memory remain distinct quantities.

Profiler runs are separate from accepted timings. The independent decoder runs during preparation, outside the measured viewer process. No terminal preference, font, power setting, or system cache change belongs to this implementation.

After the engine checks pass, compare the shared position fixture and one large interior viewport in actual Ghostty, in both layouts and themes, including width changes during highlighting. Existing user-provided capture methods can supply those frames. Exact PTY cells provide strong regression evidence but cannot prove Ghostty font shaping, clipping, or frame pacing. That visual signoff stays open until actual captures are inspected.

## 8. Completion

The implementation batch is complete when the shared representation and necessary allocation fixes pass independent source/cell checks, exhaustive projection equivalence, real-process interaction/lifecycle checks, and the five-patch performance and memory gates. The final report names any remaining unsupported or visually unverified cases. A failing patch remains a failing patch.

Planning deliverables are complete in this document. Implementation started with S0 evidence preservation and allocation attribution; current progress and remaining work are recorded in [the implementation checkpoint](MEMORY_IMPLEMENTATION.md).

Planning checks: source and baseline identities were read and verified; local document links resolve. A scratch enumeration checked the padding and marker equations for 8,144 small replacement/flag combinations in both layouts. This validates the proposed arithmetic in those cases only; the production encoder, exhaustive corpus comparison, and future performance results remain unimplemented.
