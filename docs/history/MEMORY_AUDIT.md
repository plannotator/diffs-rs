# Native Diff memory audit

Date: 2026-09-11  
Scope: `/Users/ramos/oss/native-diff` Rust package, including the interactive reader, its input/highlight workers, and the `inspect` diagnostic path.  
Method: manual ownership tracing using the performance-audit workflow, followed by read-only existing commands. No product code was changed for this audit.

Planning clarification, 2026-09-11: [MEMORY_PLAN.md](MEMORY_PLAN.md) owns the implementation order and correctness gates. Rechecking the source clarified the aggregate index accounting and the distinction between Linux's index cost and the smaller patches' unexplained RSS.

## Executive summary

The interactive queues are bounded. A leak has neither been established nor ruled out. Retained representation size is a confirmed cost on Linux:

- The interactive reader keeps a `SourceRef` for every supplied source record and keeps both split and unified projection indexes for the complete document.
- On the 19,299,175-record Linux fixture, `inspect --summary` reports 308,786,800 bytes of source indexes and 430,208,368 bytes under `projection_bytes`: 738,995,168 bytes (about 705 MiB) before allocator overhead, file metadata, parser state, or disposable caches. The second counter includes source runs as well as both compressed layouts and inverse maps; individual contributions are not yet measured.
- The same summary process reached 1,182,040,064 bytes maximum RSS. The completed interactive three-tour run reached 1,503.2 MiB settled/peak RSS for that fixture. The difference is not explained by the cache payload limits alone and needs allocation attribution before tuning cache sizes.

The next work is specified in [MEMORY_PLAN.md](MEMORY_PLAN.md):

1. Attribute index components and navigation allocations. Pierre retains only about 0.36 MiB of reported index payload, so its 303.3 MiB RSS needs a separate explanation.
2. Compact source spans using file-relative offsets, checked wide handling, and separately encoded newline flags.
3. Store shared blocks and derive both layouts and inverse lookups from them. The current retained maps are already run-compressed; full per-line maps exist in temporary parsing and plain JSON diagnostics.
4. Reduce per-file staging, then address the navigation allocation owner identified by profiling. Plain JSON diagnostic conversion remains a separate compatibility task.

The current design is already materially better than a naïve whole-patch model: exact bytes live in a private spool, file indexes are published incrementally, and source/index objects are compact relative to owned line strings. The audit does not recommend replacing the parser or adding a database before these simpler ownership changes are measured.

## Evidence collected

The existing five-patch batch is the baseline, not a new benchmark:

| Fixture | Third-tour settled RSS | Run peak RSS | Result |
| --- | ---: | ---: | --- |
| Pierre | 303.3 MiB | 303.4 MiB | over the 128/192 MiB envelope |
| Ghostty | 123.7 MiB | 123.7 MiB | within the envelope |
| Node | 300.6 MiB | 300.6 MiB | over the envelope |
| Bun | 377.0 MiB | 377.0 MiB | over the envelope |
| Linux v6→v7 | 1,503.2 MiB | 1,503.2 MiB | over the 512/768 MiB envelope |

The Linux `inspect --summary` command was also run under `/usr/bin/time -l`:

```text
source_lines       19,299,175
spool_bytes       710,870,899
source_index_bytes 308,786,800
projection_bytes  430,208,368
maximum RSS      1,182,040,064 bytes
```

The index totals are reported by the program itself (`src/main.rs:132-140`); the RSS value is an OS observation. Neither number is a proof of a leak. Together they show that the retained document representation and the process high-water mark both require attribution.

## Ownership map

```text
input Read
  └─ BufReader<Spooling<R>>
       ├─ private temporary spool (exact bytes, on disk)
       └─ ingest unit Vec (one complete file patch, retained capacity)
            └─ libgit2 Diff/Patch + model::Document (temporary per unit)
                 └─ storage::FileIndex (retained for the session)
                      ├─ SourceRef[] (one record per source line)
                      ├─ SourceRun[] and FileDiff metadata
                      ├─ Rows: split + unified
                      ├─ Locations: split + unified
                      └─ Arc<Spool>

visible rows
  ├─ geometry cache: sanitized text + checkpoints + wrap starts
  ├─ highlight worker: transient bytes/text/parser ops + bounded style replies
  ├─ style cache: token ranges/colors
  └─ Ratatui terminal buffer: viewport-sized cells
```

## Findings

### M1 — Separate compressed layouts and source runs have a large aggregate cost

**Severity:** P1  
**Locations:** `src/storage.rs:173-185`, `src/storage.rs:398-433`, `src/layout.rs:9-13`, `src/layout.rs:105-129`  
**Confidence:** high

`FileIndex` retains `split`, `unified`, `split_locations`, and `unified_locations` for every file. `storage::read` first builds both model projections, then copies each file's rows and locations into the retained `FileIndex`. The two `Rows` values are compressed runs, but they are still two complete projection representations; the two `Locations` values are two inverse maps.

For Linux, `projection_bytes` is 430,208,368 bytes. It combines source runs, both compressed row streams, and both compressed inverse maps (`src/storage.rs:255-260`). This is the largest reported aggregate, but the cost of each constituent is unmeasured. It is separate from the source-page, geometry, and style cache payloads.

**Optimization direction:** represent each hunk/replacement block once, with source spans and pairing/alignment metadata. Resolve a split or unified row only for the viewport and nearby navigation window. Keep direct file/hunk boundary indexes, but do not retain two complete row streams when a shared block can produce both views. If a full projection is still required, store compact relative offsets and share unchanged context runs.

**Correctness risk:** high. Pairing, no-newline markers, gaps, and exact row destinations must not change. This is a representation change, not permission to alter the reviewed alignment rule.

**Verification:** compare the complete source canonical digest, all existing interior cell oracles, navigation destinations, both layouts, wrap/resize anchors, and the exact five-patch workload. Record `projection_bytes` before and after and require no source/render regressions.

### M2 — Source spans and inverse maps can use more compact coordinates

**Severity:** P1  
**Locations:** `src/storage.rs:184-185`, `src/storage.rs:252-260`, `src/model.rs:248-250`, `src/layout.rs:105-129`  
**Confidence:** high

Every source record is addressable through both split and unified locations. `SourceRef` is deliberately 16 bytes (`src/storage.rs:149-154`), so the Linux source index alone is 308,786,800 bytes. The two location structures are included in the 430,208,368-byte projection total.

The current `Locations` implementation compresses runs. Its retained size scales with location runs, not necessarily every source record. The 16-byte source spans do scale with every record. File-relative eight-byte offset/length records plus a newline bitset can reduce that separate cost while preserving a checked wide representation for exceptional files.

**Optimization direction:** first measure whether a shared block-to-row index can answer `row_for_source` by binary search. If a map is still needed, use relative source/row offsets and a compact integer width with checked overflow fallback; retain one map when the other can be derived from the shared projection. Do not change integer widths solely from the current corpus without a documented maximum-input policy.

**Correctness risk:** medium/high. `row_for_source` is used for anchors, jumps, and layout changes. Preserve O(log n) or better lookup and exact source identity.

**Verification:** add memory accounting for each map separately, then run all source endpoint, file/hunk navigation, resize, split/unified, and wrapped-line process checks. Compare every destination against the current oracle.

### M3 — The legacy `model::Document` owns a whole-document line model

**Severity:** P1 for the diagnostic path; P2 for interactive peak staging  
**Locations:** `src/model.rs:14-23`, `src/model.rs:68-79`, `src/model.rs:87-103`, `src/model.rs:213-220`, `src/main.rs:142-145`  
**Confidence:** high

`model::Document` stores an owned `Vec<u8>` for every source line and full split/unified vectors and location arrays. `Document::read` appends per-file parsed data into one whole-document object. The normal interactive path uses `storage::read`, but `inspect` without `--summary` still calls `model::Document::read` (`src/main.rs:142-145`). A very large diagnostic invocation can therefore recreate the representation that the interactive path was designed to avoid.

During compact ingestion, `storage::read` still creates a temporary `Parsed` model for each file (`src/storage.rs:367-415`) before publishing its compact index. That staging is bounded by the largest file patch in the current corpus, but it creates source-line copies, rows, locations, and the compact output at the same time.

**Optimization direction:** make plain `inspect` stream from `storage::read` (or make canonical output the only whole-content diagnostic), and keep the model representation test-only or explicitly bounded. Longer term, parse directly into compact source spans/blocks so the temporary `SourceLine.bytes` vector is not needed for ordinary ingestion. Do not remove the independent parser/oracle until equivalent correctness checks exist.

**Correctness risk:** medium. JSON shape and canonical output are user-visible diagnostics; preserve them or document a versioned replacement.

**Verification:** run summary, canonical output, and full JSON mode on representative small fixtures; run the Linux summary under `/usr/bin/time -l`; ensure the interactive path never calls the whole-document `model::Document::read`. Per-file temporary parsing is a separate migration stage.

### M4 — Per-file input and parser staging can briefly multiply a large unit

**Severity:** P2  
**Locations:** `src/ingest.rs:9-16`, `src/ingest.rs:21-51`, `src/storage.rs:356-374`, `src/model.rs:105-121`, `src/model.rs:348-377`  
**Confidence:** high

`ingest::visit_files` accumulates one complete file-patch unit in `unit`. `storage::read` passes that unit to `model::parse_file` while the spool already contains the bytes. Parsing owns per-line `Vec<u8>` values, rows, and locations until conversion completes. `unit.clear()` retains the vector capacity for the largest unit seen. The empty-file normalization helper can clone the complete unit (`input.to_vec()`) before parsing.

The current Linux corpus has no file-patch unit larger than roughly 15.1 MiB. That bounds source bytes in each temporary unit, not its parsed expansion or allocator retention. Their RSS contributions need profiling. Empty-file normalization applies only to its three-line metadata case; it is not an ordinary large-file copy path.

**Optimization direction:** reuse the unit buffer where safe, avoid cloning a complete unit for the synthetic empty-file headers, and eventually parse source spans directly from the unit into the compact index. Keep the private spool as the source of truth; do not add a second whole-patch resident buffer.

**Verification:** instrument peak RSS around the per-file callback, test a deliberately large single-file fixture, and confirm cancellation/error behavior while a unit is being parsed.

### M5 — Long-line reads amplify memory before cache limits can apply

**Severity:** P2  
**Locations:** `src/storage.rs:111-128`, `src/geometry.rs:127-147`, `src/highlight.rs:275-337`, `src/text.rs:4-49`  
**Confidence:** high for the mechanism; workload impact is unmeasured

Each `FileIndex::bytes` call allocates a fresh `Vec<u8>` for the complete source line. Geometry then builds a sanitized `String` and stores it in `Line`; highlighting builds another sanitized string and appends a newline. A visible line can therefore have raw bytes, sanitized text, parser operations, and style vectors live at overlapping times. `SourceRef.len` allows a line up to 4 GiB, and geometry has no early long-line bound. The highlight guard is checked only after `display_text` and `format!("{text}\n")` have already allocated.

The normal viewport keeps this temporary, and cache entry limits prevent oversized geometry from being retained. Those limits do not prevent a pathological one-line allocation or repeated work after eviction.

**Optimization direction:** reduce duplicate full strings and reuse worker scratch storage while preserving the current styling fallback exactly. A later chunked geometry path requires explicit UTF-8, escape, grapheme, tab, and wrap state across chunks. It must preserve complete text and byte anchors; no new truncation or lower styling threshold is proposed.

**Correctness risk:** high for text handling. Tabs, invalid UTF-8, controls, combining marks, and wide glyphs must remain identical.

**Verification:** add a real-process long-line fixture covering valid/invalid UTF-8, tabs, combining and wide characters; measure peak RSS and responsiveness while wrapping, resizing, and highlighting it.

### M6 — Cache budgets exclude container capacity and allocator high-water behavior

**Severity:** P2  
**Locations:** `src/storage.rs:23-28`, `src/storage.rs:65-108`, `src/geometry.rs:114-146`, `src/highlight.rs:382-412`  
**Confidence:** high for accounting gap; leak status unproven

The source, geometry, and style caches cap their payload counters. Their `HashMap` and `VecDeque` bucket/storage capacities are not included. Eviction removes entries but does not shrink those containers. The style cache additionally counts `LineStyle` and vector capacities but not the map/deque capacity. Freed allocations may remain in the process allocator and therefore in RSS.

This is consistent with a cache high-water contribution, but the three-tour results do not prove a leak: Bun grows from 318.9 to 377.0 MiB and Linux from 1,478.9 to 1,503.2 MiB, while the process may also retain parser/allocator arenas.

**Optimization direction:** expose payload, container-capacity, and live-entry bytes separately. Use a bounded structure or periodic, policy-driven compaction rather than shrinking on every eviction. Treat RSS as the acceptance metric and cache counters as diagnostics, not as a memory guarantee.

**Verification:** capture Instruments Allocations/VM Tracker data during the exact three-tour workload, correlate high-water allocations with each cache, and repeat after any policy change. Require no increase in correctness failures or cold-highlight latency beyond the documented tradeoff.

### M7 — Width-keyed geometry duplicates sanitized text across resizes

**Severity:** P2  
**Locations:** `src/geometry.rs:22-60`, `src/geometry.rs:112-147`  
**Confidence:** high

The geometry key is `(source id, width)` (`src/geometry.rs:112-113`). Each `Line` owns its own boxed display text and boundary vectors. The same source line can therefore retain duplicate sanitized text for multiple wrapped widths after resize; an unwrapped entry and wrapped entries also carry separate text allocations. The 16 MiB payload and 1,024-entry caps bound this cache, but resize-heavy tours can reach the high-water limit quickly.

**Optimization direction:** separate a small shared sanitized-text cache from width-specific boundary data, or share the text allocation between geometry entries while keeping starts/checkpoints per width. Do this only after M1/M2; the current cache is bounded and correctness-sensitive.

**Verification:** run the existing rapid resize/wrap position suite, inspect cache entries by source and width, and compare geometry payload and RSS across repeated width changes.

### M8 — Highlighting has deliberate old/new duplication plus transient string churn

**Severity:** P2/P3  
**Locations:** `src/highlight.rs:24-60`, `src/highlight.rs:287-337`, `src/highlight.rs:358-379`, `src/highlight.rs:382-412`  
**Confidence:** high for the mechanism

`LineStyle` can retain both new-side and old-side token vectors when a context line has different syntax colors. That duplication is intentional for visual correctness. The worker also clones and sorts the requested ID list (`src/highlight.rs:232-234`), allocates a display string plus newline for each parsed line, and computes changed spans with temporary diff state.

**Optimization direction:** preserve the two-side style semantics, but reuse request scratch storage and one worker text buffer. Only consider compact token encoding after allocation profiling shows style payload is material; do not trade away exact byte ranges or colors for a guessed saving.

**Verification:** compare sequential old/new style oracles, theme changes, inline emphasis, and cold/warm highlight latency while recording style payload and container bytes.

### M9 — Per-frame formatting is allocator churn, not retained-document memory

**Severity:** P3  
**Locations:** `src/render.rs:119-220`, `src/render.rs:273-303`, `src/render.rs:342-348`, `src/render.rs:461-493`  
**Confidence:** high

Every draw formats the title/status/header strings and builds gutter strings for visible cells. These allocations are short-lived and viewport-bounded. They can affect frame-time jitter and allocator fragmentation, but they are not a plausible explanation for the corpus-scale retained RSS.

**Optimization direction:** leave them alone until retained representation and allocator attribution are fixed. If frame traces show churn, render into reusable scratch strings or write directly into the Ratatui buffer.

## Existing strengths and non-findings

- `storage::Spool` keeps exact patch bytes on disk and bounds resident source pages at 32 MiB (`src/storage.rs:20-21`, `src/storage.rs:98-108`). The interactive path does not retain a second complete patch buffer.
- Highlighting has one pending viewport request and a reply channel of 16 (`src/highlight.rs:146-165`, `src/highlight.rs:221-224`). File publication uses a channel of two and terminal events use a channel of one (`src/main.rs:169-179`). These are count bounds, not byte bounds on arbitrary message payloads.
- `App.visible` is viewport-sized (`src/app.rs:52`, `src/render.rs:126-185`), and Ratatui draws the terminal-sized buffer. The renderer does not build a full-screen representation of the entire diff.
- The syntax continuation holds one file/hunk's parser state, not the whole corpus (`src/highlight.rs:119-127`, `src/highlight.rs:253-269`).
- No global collection, detached worker, or retry loop was found that grows with every navigation event. `Worker::Drop` closes its mailbox (`src/highlight.rs:168-174`).
- `alignment_offset` allocates filtered comparison vectors, but its work is explicitly bounded by 4,096 pair/extra combinations and skips lines over 4,096 bytes (`src/model.rs:395-407`). This is localized allocation churn, not a large-corpus retained-memory cause.
- The syntax set has one worker owner. Lazy grammar/regex allocations and repeatedly loaded theme data are outside the cache counters and need profiling, especially for small patches with large RSS.

## Recommended implementation sequence

Follow stages S0 through S5 in [MEMORY_PLAN.md](MEMORY_PLAN.md): preserve evidence and attribute allocations, compact spans, introduce a shared projection with exhaustive equivalence, reduce ingestion peaks, fix the measured navigation allocation owner, and run final acceptance. The plan defines exact rollback, source/cell invariants, and the conditional scope of long-line and cache changes.

## Verification gate for every memory change

Each representation change must pass the same evidence chain:

1. Complete supplied-source canonical digest for all five pinned patches.
2. Existing interior cell checks, including split/unified, signs, line numbers, inline spans, no-newline markers, invalid UTF-8, tabs, Unicode width, and metadata-only files.
3. Real-process navigation, endpoint, hunk/file jump, wrap, width-change, and pending-highlight tests.
4. Five fresh latency repetitions and the exact three-tour memory workload.
5. A report containing process RSS peak, third-tour settled RSS, source-index bytes, projection bytes, geometry/style payload, cache container capacity, and any profiler-attributed transient peak.

An optimization is not accepted if it merely lowers a cache counter while increasing process RSS, changing a destination, or altering a rendered cell.

## Remaining uncertainty

Linux's reported index payload exceeds its steady RSS target before caches are considered. Compact source and shared projection storage address that confirmed cost. Small-patch navigation growth remains unattributed. Further evidence must distinguish live allocations, library state, and allocator retention; the audit does not prove that all excess RSS has one cause or that leaks are absent.
