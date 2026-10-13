# Retained document metadata

Status: complete and kept as a narrow storage cleanup, 2026-09-14.
Correctness, storage accounting, and capture-first checks passed. The ordinary
memory comparison and fixed allocation follow-up completed. RSS improvement is
not established; the higher ordinary Bun peak remains unexplained.

## Storage change

The parser keeps its mutable `FileDiff` and `Hunk` types. Before publication,
`storage::read` consumes these values into immutable metadata. File labels and
hunk headers use `Box<str>`. Each file owns a boxed hunk slice.

The conversion preserves every field through exhaustive destructuring. It occurs
after projection construction and source-range rebasing. Retained consumers
continue to borrow strings and slices directly. Source storage, pairing,
projections, cache limits, and syntax coverage remain unchanged.

The implementation changes three application files. The separate metadata module
contains the retained types and conversions. `storage.rs` publishes those types
and accounts for their extents. `highlight.rs` explicitly borrows the path as
`&str`. No rendering observer or background shrink operation is present.

## Counted storage

The diagnostic borrows the completed retained document. It counts labels, hunk
headers, hunk descriptors, and unused capacity. Per-file and aggregate counts
reconcile with `FileIndex::metadata_bytes`.

| Pinned patch | Retained extent reduction | Approximate MiB |
| --- | ---: | ---: |
| Pierre package cleanup | 14,410 bytes | 0.014 |
| Ghostty library change | 6,099 bytes | 0.006 |
| Node dependency update | 924,622 bytes | 0.882 |
| Bun rewrite | 400,289 bytes | 0.382 |
| Linux version comparison | 21,214,949 bytes | 20.232 |

For the largest patch, unused buffers account for 14,150,541 bytes.
Smaller descriptors account for another 7,064,408 bytes. The combined reduction
includes file descriptors and allocated metadata, without counting either twice.
The remaining accounted document storage is 378,647,669 bytes.

All five patches retain the same file, source, row, hunk, and text-length counts.
Source indexes and projections retain the same extents. The largest observed file
contains 924 hunks. Publication work scales with the current file, not the entire
document.

These figures exclude allocator rounding, allocation headers, Arc headers,
stacks, spool/cache containers, and library allocations. They do not establish
total heap size, RSS reduction, or a process-memory limit. String shrinking and
slice conversion can relocate allocations during publication.

The [before counts](evidence/document-metadata-v1/before-counts/identity.json)
and [after counts](evidence/document-metadata-v1/after-counts/identity.json)
include the diagnostic, input, and source hashes. Adjacent archives preserve the
exact diagnostic sources and executables.

## Correctness

The [correctness report](evidence/document-metadata-v1/checks/report.json)
records these completed checks:

- Formatting and application Clippy passed. Existing vendored warnings remain.
- The release suite passed 10 library tests, 34 process contracts, and one worker
  test. Two existing opt-in process tests remained excluded.
- Independent source expectations matched all 20,847,451 supplied records across
  all five patches.
- Complete projection comparisons matched serialized metadata, source bytes,
  both layouts, inverse lookups, partners, and boundary destinations.
- The existing pattern comparison passed 2,888 accepted cases. Both parsers
  rejected the same 716 other cases.
- A five-file boundary fixture passed through published storage. It covers
  quoted and Unicode names, empty files, a rename, mode changes, a nonempty hunk
  suffix, and accepted source starts above two billion.

An additional [real Git control](evidence/document-metadata-v1/git-rename-control/rejections.json)
found an existing input-support bug. Both executables reject a valid metadata-only
rename whose names contain spaces. The accepted boundary fixture uses a rename
without spaces. Separate records retain space, quote, and Unicode coverage.
The parser issue remains separate from the metadata conversion and requires a
correctness follow-up.

The preserved candidate ran contracts that honor the executable override.
Legacy direct executable calls used the Cargo-built companion from identical
application source. The correctness report identifies both executables.

## Runtime qualification

The initial sequence reused the existing protocols:

1. A capture correctness gate.
2. Five alternating responsiveness pairs, followed by exact disk replay.
3. One fresh process per executable and patch, with three memory tours.
4. The existing audit of view order, settled holds, RSS samples, and cleanup.

The memory tours include indexing, source navigation, resizing, cache revisits,
and exit. The capture protocol separates receipt from expensive replay checks.
Memory-tour observations do not replace the capture-first latency checks.

The historical joined-emoji case remains source-only because its terminal decoder
cannot preserve that content. It is not a passed terminal capture. These checks
do not establish physical Ghostty paint or trackpad behavior.

The runtime sequence uses a shared Mac. The comparison task received a request
to avoid competing workloads. No dedicated-host isolation is claimed. Existing
targets do not change the workload or justify a storage decision by themselves.

The 40 terminal captures passed exact replay and cleanup checks. Ten joined-emoji
cases remained source-only. Paired median text differences ranged from
−0.958 to +0.629 ms across the 28 case/action combinations. Styled differences
ranged from −3.841 to +1.765 ms. Directions varied. These observations establish
neither a general speedup nor timing equivalence.

The first memory reference process stopped at `horizontal-middle`. The old trace
expected offset 150 for a 235-column line in a 150-column content area.
The horizontal boundary fix limits the offset to 85. Anchor byte 150 remains
visible. This failure occurred before the metadata candidate ran. The failed
cohort remains separate and supplies no accepted memory comparison.

The [trace migration](evidence/document-metadata-v1/migrate_scroll_trace.py)
derives limits from original source text and historical visible source IDs.
It preserves actions, dimensions, categories, holds, source identities, and
anchors. Only horizontal state and clipped source cells can change.
Independent source and sequential-color checks still apply.

All five current-reference migrations passed. The metadata candidate also matched
the first reference trace exactly. The older, unbounded executable failed the bounded
expectation. The first control invocation omitted `--legacy-control`, so it was
an old-executable/new-expectation rejection, not a legacy compatibility pass.
An offline check compared full cell attributes for overlapping source glyphs.
It checked 708 cells across six changed views and rejected deliberate bold and
background mutations. All unaffected cells, source identities, and anchors stayed
unchanged. Independent source and color checks covered newly visible text.
No historical trace or original qualification result is overwritten.

The [fresh memory cohort](evidence/document-metadata-v1/memory-bounded/declaration.json)
uses these shared traces for both preserved executables. It retains all original
actions and holds. Ten serial processes cover 4,324 views and 2,856 settled holds.
The original failed process remains separate.

All ten ordinary processes completed. The final audit checked 4,324 views,
2,856 settled holds, and 50,038 raw RSS queries. It also checked the process
order, recomputed summaries, source identities, terminal cleanup, and absent
viewer process groups.

All RSS figures in this table use MiB. The three hold maxima correspond to
tours one, two, and three. They are not end-of-process measurements.

| Patch | Reference process peak | Candidate process peak | Reference hold maxima | Candidate hold maxima |
| --- | ---: | ---: | --- | --- |
| Pierre package cleanup | 70.53 | 54.31 | 65.64 / 70.45 / 40.63 | 54.31 / 50.05 / 50.05 |
| Ghostty library change | 58.09 | 57.63 | 57.53 / 57.38 / 58.00 | 57.63 / 52.06 / 52.06 |
| Node dependency update | 128.39 | 117.36 | 127.72 / 128.39 / 112.00 | 117.36 / 109.50 / 110.08 |
| Bun rewrite | 120.05 | 161.03 | 98.52 / 119.97 / 117.52 | 139.83 / 161.02 / 101.03 |
| Linux version comparison | 669.45 | 670.30 | 663.34 / 640.72 / 424.58 | 670.30 / 591.11 / 445.33 |

The largest-patch process peaks differ by +0.844 MiB. Its third-tour candidate
hold maximum is 20.75 MiB higher. The largest candidate held-view sample across
all three tours is 670.30 MiB, not the third-tour value of 445.33 MiB.
These observations do not establish an RSS reduction from smaller metadata.

Bun exposes an adverse ordinary peak:
120.047 MiB for the reference and 161.031 MiB for the candidate.
Several candidate holds during tour two retain approximately 161 MiB.
The lower third-tour candidate RSS does not cancel that earlier result.
Raw queries, hold extrema, and OS maxima agree. Both processes also show large
RSS decreases at different points. The cause remains unresolved.

The [ordinary audit](evidence/document-metadata-v1/memory-bounded-audit.json)
binds the raw records and recomputed summaries. The Mac also retained compressed
pages and used swap during this comparison. That host observation does not
identify the cause of any process-specific difference.

## Allocation follow-up and decision

The [declared Bun follow-up](evidence/document-metadata-v1/BUN_PEAK_FOLLOWUP.md)
completed one reference/candidate pair with child-only allocation logging.
It used the same actions and holds through a fixed second-tour prefix.
Each process paused at three declared points for sequential native commands.
All 504 checked views, 330 holds, six snapshots, and cleanup checks passed.
No additional pair or replacement performance run occurred.

The native `heap` rows reconcile exactly with these live-allocation totals:

| Matched checkpoint | Reference bytes | Candidate bytes | Candidate minus reference |
| --- | ---: | ---: | ---: |
| Opening | 24,241,632 | 23,790,816 | −450,816 |
| Tour-two hunk end, unified | 147,040,048 | 146,004,128 | −1,035,920 |
| Tour-two file middle, width 100 | 147,409,648 | 146,342,288 | −1,067,360 |

Live heap was lower at each instrumented checkpoint. This does not attribute
the entire difference to metadata. Other allocation counts also differed.

The native `vmmap` malloc-zone table helps separate allocated and resident memory.
At the last checkpoint, it printed 69.1M resident and 81.6M swapped for the
reference, versus 100.6M resident and 49.0M swapped for the candidate.
Allocated totals were 140.6M and 139.6M, respectively. These strings retain the
tool's rounding and units. The reference had lower residency despite higher
live allocation. Its preceding checkpoint also had higher residency despite
slightly lower live allocation.

This demonstrates a residency difference in the instrumented pair. It does not
identify the cause of the ordinary candidate's approximately 41 MiB higher peak.
Instrumentation and profiler pauses can change execution and memory behavior.
The native commands are sequential, not atomic. Heap, allocation-stack, and
vmmap totals remain separate measurements.

Decision: keep the immutable metadata representation. It removes measured unused
capacity and descriptor words from read-only objects. It preserves direct
borrowed reads and adds only a per-file publication conversion. It does not add
per-cache-admission allocations, a new eviction policy, or a rendering observer.
Complete source/projection checks passed, and the focused responsiveness results
showed mixed small differences. The three live-heap observations support the
storage rationale without proving an ordinary process-memory benefit.

This is an engineering decision with residual uncertainty, not a uniformly
favorable performance result. Publication can relocate allocations. The ordinary
Bun peak and the largest patch's higher third-tour hold maximum remain open
observations. Neither is dismissed as noise. The investigation required by the
frozen plan is complete; causal attribution of those observations is not.
No further runs are authorized merely to obtain a favorable comparison.

The [corrected offline audit](evidence/document-metadata-v1/bun-peak-audit-v2.json)
binds all native outputs. Review found that the first allocation-stack reader
omitted singular `1 call` entries. The corrected reader accepts `call` and `calls`.
It recomputed auxiliary stack totals from the same saved output. Heap totals,
native observations, and viewer checks did not change. The initial reader and
report remain preserved. A create-only output guard prevented overwriting the
first report; the correction uses a new filename.

Migration records are correctness evidence, not performance comparisons. Their
trace format retains setup and cleanup durations. The migration report's phrase
“No performance or RSS results retained” overstates that distinction. Those
durations are not used as performance evidence. No RSS samples are retained there.

## Build identities

| Role | SHA-256 |
| --- | --- |
| Preserved reference | `5858477016707a185c179e72a57449fce8eaa0312b26af3ee0266512c9160919` |
| Preserved candidate | `16f71830c815741ce1cdb5037d5a69ecd886778c238042beb66b2567f72beb58` |
| Cargo-built candidate companion | `411dbde8e730605e34abb2c7a9a19c62153e04af7935db1c60d24f2ce4a81f6a` |

The reference includes the horizontal boundary fix. Both preserved builds have
fresh target directories, exact source archives, and build recipes. Earlier
experiments and their evidence remain unchanged.

The [decision](evidence/document-metadata-v1/decision.json) and
[seal](evidence/document-metadata-v1/seal.json) identify the retained artifacts.
The application source did not change during qualification. The current normal
executable contains the kept metadata change and the horizontal boundary fix.
All stage viewers and profilers have exited. Do not restart completed drivers.

The next implementation is the separate
[Git path correctness fix](GIT_PATH_CORRECTNESS_PLAN.md). Physical terminal
presentation and first-visit coloring delays also remain open.
